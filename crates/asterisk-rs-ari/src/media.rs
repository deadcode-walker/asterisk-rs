//! WebSocket media channel driver for exchanging audio with Asterisk.
//!
//! provides a typed interface to chan_websocket for sending and receiving
//! raw audio frames, DTMF events, and media control commands.
//!
//! requires Asterisk 20.16.0+ / 22.6.0+ / 23.0.0+

use std::collections::HashMap;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, watch};

use crate::error::{AriError, Result};

/// events received from Asterisk over the media websocket
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "event")]
#[non_exhaustive]
pub enum MediaEvent {
    /// media session established with channel details and codec info
    #[serde(rename = "MEDIA_START")]
    MediaStart {
        connection_id: String,
        channel: String,
        channel_id: String,
        format: String,
        optimal_frame_size: u32,
        ptime: u32,
        #[serde(default)]
        channel_variables: HashMap<String, String>,
    },

    /// DTMF digit completed on the channel
    #[serde(rename = "DTMF_END")]
    DtmfEnd { digit: String, duration_ms: u32 },

    /// stop sending media — Asterisk buffer is full
    #[serde(rename = "MEDIA_XOFF")]
    MediaXoff,

    /// resume sending media — Asterisk buffer drained
    #[serde(rename = "MEDIA_XON")]
    MediaXon,

    /// channel status response to a GetStatus command
    #[serde(rename = "STATUS")]
    Status {
        channel: String,
        format: String,
        queue_size: u32,
        buffering_active: bool,
        media_paused: bool,
    },

    /// buffering mode completed, optional correlation_id ties to the stop request
    #[serde(rename = "MEDIA_BUFFERING_COMPLETED")]
    MediaBufferingCompleted {
        #[serde(default)]
        correlation_id: Option<String>,
    },

    /// a previously inserted mark point has been processed
    #[serde(rename = "MEDIA_MARK_PROCESSED")]
    MediaMarkProcessed,

    /// all queued media has been sent to the channel
    #[serde(rename = "QUEUE_DRAINED")]
    QueueDrained,
}

/// commands sent to Asterisk over the media websocket
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "command")]
#[non_exhaustive]
pub enum MediaCommand {
    /// answer the channel
    #[serde(rename = "ANSWER")]
    Answer,

    /// hang up the channel with an optional cause code
    #[serde(rename = "HANGUP")]
    Hangup {
        #[serde(skip_serializing_if = "Option::is_none")]
        cause: Option<u32>,
    },

    /// start buffering mode — assembles full frames across messages
    #[serde(rename = "START_MEDIA_BUFFERING")]
    StartMediaBuffering,

    /// stop buffering mode and flush remainder
    #[serde(rename = "STOP_MEDIA_BUFFERING")]
    StopMediaBuffering {
        #[serde(skip_serializing_if = "Option::is_none")]
        correlation_id: Option<String>,
    },

    /// discard all queued audio frames
    #[serde(rename = "FLUSH_MEDIA")]
    FlushMedia,

    /// pause sending media to the channel core
    #[serde(rename = "PAUSE_MEDIA")]
    PauseMedia,

    /// resume sending media to the channel core
    #[serde(rename = "CONTINUE_MEDIA")]
    ContinueMedia,

    /// insert a marker in the frame queue
    #[serde(rename = "MARK_MEDIA")]
    MarkMedia,

    /// request channel status
    #[serde(rename = "GET_STATUS")]
    GetStatus,

    /// request notification when the media queue is empty
    #[serde(rename = "REPORT_QUEUE_DRAINED")]
    ReportQueueDrained,
}

/// internal command sent to the background task
enum InternalCmd {
    Audio(Vec<u8>),
    /// pre-serialized JSON text command
    Command(String),
}

/// connection to an Asterisk WebSocket media channel
///
/// exchanges raw audio frames and control commands with Asterisk's
/// chan_websocket channel driver. splits incoming traffic: text frames
/// become [`MediaEvent`]s, binary frames become raw audio buffers.
///
/// the connection runs in a background task; dropping the channel
/// shuts it down.
pub struct MediaChannel {
    event_rx: mpsc::Receiver<MediaEvent>,
    audio_rx: mpsc::Receiver<Vec<u8>>,
    command_tx: mpsc::Sender<InternalCmd>,
    shutdown_tx: watch::Sender<bool>,
    task_handle: tokio::task::JoinHandle<()>,
}

impl std::fmt::Debug for MediaChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaChannel")
            .field("connected", &!self.command_tx.is_closed())
            .finish()
    }
}

/// type alias for an outbound (client-initiated) websocket stream
type OutboundWsStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

/// type alias for an accepted (server-side) websocket stream over raw TCP
type AcceptedWsStream = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

impl MediaChannel {
    /// connect to an Asterisk media websocket endpoint
    ///
    /// url should be the full websocket URL including the connection_id path,
    /// e.g. `ws://asterisk:8088/media/32966726-4388-456b-a333-fdf5dbecc60d`
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|e| AriError::WebSocket(e.to_string()))?;

        Ok(Self::spawn_outbound(ws_stream))
    }

    /// create from an already-accepted websocket stream over raw TCP
    ///
    /// useful when running a media server that accepts incoming connections
    pub fn from_accepted(ws_stream: AcceptedWsStream) -> Self {
        let (event_tx, event_rx) = mpsc::channel(64);
        let (audio_tx, audio_rx) = mpsc::channel(256);
        let (command_tx, command_rx) = mpsc::channel(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let task_handle = tokio::spawn(media_loop(
            ws_stream,
            event_tx,
            audio_tx,
            command_rx,
            shutdown_rx,
        ));

        Self {
            event_rx,
            audio_rx,
            command_tx,
            shutdown_tx,
            task_handle,
        }
    }

    fn spawn_outbound(ws_stream: OutboundWsStream) -> Self {
        let (event_tx, event_rx) = mpsc::channel(64);
        let (audio_tx, audio_rx) = mpsc::channel(256);
        let (command_tx, command_rx) = mpsc::channel(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let task_handle = tokio::spawn(media_loop(
            ws_stream,
            event_tx,
            audio_tx,
            command_rx,
            shutdown_rx,
        ));

        Self {
            event_rx,
            audio_rx,
            command_tx,
            shutdown_tx,
            task_handle,
        }
    }

    /// receive the next control event from Asterisk
    ///
    /// returns `None` when the connection is closed
    pub async fn recv_event(&mut self) -> Option<MediaEvent> {
        self.event_rx.recv().await
    }

    /// receive the next audio frame from Asterisk
    ///
    /// returns `None` when the connection is closed
    pub async fn recv_audio(&mut self) -> Option<Vec<u8>> {
        self.audio_rx.recv().await
    }

    /// send a control command to Asterisk
    pub async fn send_command(&self, cmd: MediaCommand) -> Result<()> {
        let json = serde_json::to_string(&cmd).map_err(AriError::Json)?;
        self.command_tx
            .send(InternalCmd::Command(json))
            .await
            .map_err(|_| AriError::Disconnected)
    }

    /// send raw audio data to Asterisk
    ///
    /// data should be encoded in the format negotiated during MEDIA_START.
    /// Asterisk will re-frame if buffering mode is active. max 65500 bytes.
    pub async fn send_audio(&self, data: Vec<u8>) -> Result<()> {
        self.command_tx
            .send(InternalCmd::Audio(data))
            .await
            .map_err(|_| AriError::Disconnected)
    }

    /// answer the channel
    pub async fn answer(&self) -> Result<()> {
        self.send_command(MediaCommand::Answer).await
    }

    /// hang up the channel with an optional cause code
    pub async fn hangup(&self, cause: Option<u32>) -> Result<()> {
        self.send_command(MediaCommand::Hangup { cause }).await
    }

    /// start media buffering mode
    pub async fn start_buffering(&self) -> Result<()> {
        self.send_command(MediaCommand::StartMediaBuffering).await
    }

    /// stop media buffering mode
    pub async fn stop_buffering(&self, correlation_id: Option<String>) -> Result<()> {
        self.send_command(MediaCommand::StopMediaBuffering { correlation_id })
            .await
    }

    /// flush all queued audio frames
    pub async fn flush(&self) -> Result<()> {
        self.send_command(MediaCommand::FlushMedia).await
    }

    /// pause media delivery to the channel core
    pub async fn pause(&self) -> Result<()> {
        self.send_command(MediaCommand::PauseMedia).await
    }

    /// resume media delivery to the channel core
    pub async fn resume(&self) -> Result<()> {
        self.send_command(MediaCommand::ContinueMedia).await
    }

    /// insert a marker in the frame queue
    pub async fn mark(&self) -> Result<()> {
        self.send_command(MediaCommand::MarkMedia).await
    }

    /// request channel status
    pub async fn get_status(&self) -> Result<()> {
        self.send_command(MediaCommand::GetStatus).await
    }

    /// request notification when the media queue is empty
    pub async fn report_queue_drained(&self) -> Result<()> {
        self.send_command(MediaCommand::ReportQueueDrained).await
    }

    /// shut down the connection
    pub fn disconnect(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task_handle.abort();
    }
}

impl Drop for MediaChannel {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// background task that bridges a websocket stream into typed channels.
///
/// generic over the stream type so it works for both outbound
/// (`MaybeTlsStream<TcpStream>`) and accepted (`TcpStream`) connections.
async fn media_loop<S>(
    ws_stream: tokio_tungstenite::WebSocketStream<S>,
    event_tx: mpsc::Sender<MediaEvent>,
    audio_tx: mpsc::Sender<Vec<u8>>,
    mut command_rx: mpsc::Receiver<InternalCmd>,
    mut shutdown_rx: watch::Receiver<bool>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    use tokio_tungstenite::tungstenite::Message;

    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<MediaEvent>(&text) {
                            Ok(event) => {
                                if event_tx.send(event).await.is_err() {
                                    // receiver dropped
                                    return;
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    "failed to parse media event"
                                );
                            }
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        if audio_tx.send(data.into()).await.is_err() {
                            // receiver dropped
                            return;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::debug!("media websocket closed by peer");
                        return;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "media websocket read error");
                        return;
                    }
                    None => return,
                    // ping/pong handled by tungstenite
                    _ => {}
                }
            }
            cmd = command_rx.recv() => {
                match cmd {
                    Some(InternalCmd::Audio(data)) => {
                        if let Err(e) = write.send(Message::Binary(data.into())).await {
                            tracing::warn!(error = %e, "failed to send audio frame");
                            return;
                        }
                    }
                    Some(InternalCmd::Command(json)) => {
                        if let Err(e) = write.send(Message::Text(json.into())).await {
                            tracing::warn!(error = %e, "failed to send media command");
                            return;
                        }
                    }
                    // command channel closed — MediaChannel dropped
                    None => return,
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::debug!("media channel shutdown requested");
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_command_answer_serialization() {
        let json = serde_json::to_string(&MediaCommand::Answer).expect("serialize answer");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("parse answer json");
        assert_eq!(parsed["command"], "ANSWER");
    }

    #[test]
    fn test_media_command_hangup_with_cause() {
        let cmd = MediaCommand::Hangup { cause: Some(16) };
        let json = serde_json::to_string(&cmd).expect("serialize hangup");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("parse hangup json");
        assert_eq!(parsed["command"], "HANGUP");
        assert_eq!(parsed["cause"], 16);
    }

    #[test]
    fn test_media_command_hangup_without_cause() {
        let cmd = MediaCommand::Hangup { cause: None };
        let json = serde_json::to_string(&cmd).expect("serialize hangup no cause");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("parse hangup json");
        assert_eq!(parsed["command"], "HANGUP");
        assert!(parsed.get("cause").is_none());
    }

    #[test]
    fn test_media_command_stop_buffering_with_correlation_id() {
        let cmd = MediaCommand::StopMediaBuffering {
            correlation_id: Some("req-42".to_string()),
        };
        let json = serde_json::to_string(&cmd).expect("serialize stop buffering");
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("parse stop buffering json");
        assert_eq!(parsed["command"], "STOP_MEDIA_BUFFERING");
        assert_eq!(parsed["correlation_id"], "req-42");
    }

    #[test]
    fn test_media_event_media_start_deserialization() {
        let json = r#"{
            "event": "MEDIA_START",
            "connection_id": "abc-123",
            "channel": "WebSocket/ws-00000001",
            "channel_id": "chan-001",
            "format": "ulaw",
            "optimal_frame_size": 160,
            "ptime": 20,
            "channel_variables": {"CALLERID(num)": "1234"}
        }"#;

        let event: MediaEvent =
            serde_json::from_str(json).expect("deserialize MEDIA_START");
        match event {
            MediaEvent::MediaStart {
                connection_id,
                channel,
                channel_id,
                format,
                optimal_frame_size,
                ptime,
                channel_variables,
            } => {
                assert_eq!(connection_id, "abc-123");
                assert_eq!(channel, "WebSocket/ws-00000001");
                assert_eq!(channel_id, "chan-001");
                assert_eq!(format, "ulaw");
                assert_eq!(optimal_frame_size, 160);
                assert_eq!(ptime, 20);
                assert_eq!(
                    channel_variables.get("CALLERID(num)"),
                    Some(&"1234".to_string())
                );
            }
            other => panic!("expected MediaStart, got {other:?}"),
        }
    }

    #[test]
    fn test_media_event_dtmf_deserialization() {
        let json = r#"{"event": "DTMF_END", "digit": "5", "duration_ms": 120}"#;
        let event: MediaEvent =
            serde_json::from_str(json).expect("deserialize DTMF_END");
        match event {
            MediaEvent::DtmfEnd {
                digit,
                duration_ms,
            } => {
                assert_eq!(digit, "5");
                assert_eq!(duration_ms, 120);
            }
            other => panic!("expected DtmfEnd, got {other:?}"),
        }
    }

    #[test]
    fn test_media_event_xoff_deserialization() {
        let json = r#"{"event": "MEDIA_XOFF"}"#;
        let event: MediaEvent =
            serde_json::from_str(json).expect("deserialize MEDIA_XOFF");
        assert!(matches!(event, MediaEvent::MediaXoff));
    }

    #[test]
    fn test_media_event_status_deserialization() {
        let json = r#"{
            "event": "STATUS",
            "channel": "WebSocket/ws-00000001",
            "format": "ulaw",
            "queue_size": 5,
            "buffering_active": true,
            "media_paused": false
        }"#;
        let event: MediaEvent =
            serde_json::from_str(json).expect("deserialize STATUS");
        match event {
            MediaEvent::Status {
                channel,
                format,
                queue_size,
                buffering_active,
                media_paused,
            } => {
                assert_eq!(channel, "WebSocket/ws-00000001");
                assert_eq!(format, "ulaw");
                assert_eq!(queue_size, 5);
                assert!(buffering_active);
                assert!(!media_paused);
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }

    #[test]
    fn test_media_event_buffering_completed_with_correlation() {
        let json =
            r#"{"event": "MEDIA_BUFFERING_COMPLETED", "correlation_id": "req-42"}"#;
        let event: MediaEvent =
            serde_json::from_str(json).expect("deserialize MEDIA_BUFFERING_COMPLETED");
        match event {
            MediaEvent::MediaBufferingCompleted { correlation_id } => {
                assert_eq!(correlation_id.as_deref(), Some("req-42"));
            }
            other => panic!("expected MediaBufferingCompleted, got {other:?}"),
        }
    }

    #[test]
    fn test_media_event_queue_drained_deserialization() {
        let json = r#"{"event": "QUEUE_DRAINED"}"#;
        let event: MediaEvent =
            serde_json::from_str(json).expect("deserialize QUEUE_DRAINED");
        assert!(matches!(event, MediaEvent::QueueDrained));
    }

    #[test]
    fn test_media_start_without_channel_variables() {
        // channel_variables should default to empty map when absent
        let json = r#"{
            "event": "MEDIA_START",
            "connection_id": "abc-123",
            "channel": "WebSocket/ws-00000001",
            "channel_id": "chan-001",
            "format": "ulaw",
            "optimal_frame_size": 160,
            "ptime": 20
        }"#;
        let event: MediaEvent =
            serde_json::from_str(json).expect("deserialize MEDIA_START without vars");
        match event {
            MediaEvent::MediaStart {
                channel_variables, ..
            } => {
                assert!(channel_variables.is_empty());
            }
            other => panic!("expected MediaStart, got {other:?}"),
        }
    }
}
