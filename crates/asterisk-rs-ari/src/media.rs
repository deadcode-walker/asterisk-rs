//! WebSocket media channel driver for exchanging audio with Asterisk.
//!
//! provides a typed interface to chan_websocket's JSON control protocol for
//! sending and receiving raw audio frames, DTMF events, and media commands.
//! Create the channel with
//! [`crate::resources::channel::ExternalMediaParams::websocket_json`] so
//! Asterisk selects `transport_data=f(json)` instead of its plaintext default.
//!
//! requires Asterisk 20.16.0+ / 22.6.0+ / 23.0.0+

use std::collections::HashMap;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, watch};

use crate::error::{AriError, Result};
use crate::websocket::OwnedTask;

const MEDIA_CLOSE_TIMEOUT: Duration = Duration::from_millis(500);
const MEDIA_WRITE_TIMEOUT: Duration = Duration::from_secs(1);
/// Maximum raw media payload accepted from or sent to Asterisk.
pub const MAX_MEDIA_PAYLOAD_BYTES: usize = 65_500;

/// Security and trust options for an outbound media WebSocket.
#[derive(Clone, Default)]
pub struct MediaConnectionOptions {
    allow_insecure_remote: bool,
    tls_trust: crate::config::TlsTrust,
}

impl MediaConnectionOptions {
    /// Create secure-by-default media connection options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Explicitly permit a cleartext `ws://` connection to a non-loopback host.
    pub fn allow_insecure_remote(mut self, allow: bool) -> Self {
        self.allow_insecure_remote = allow;
        self
    }

    /// Add one or more PEM-encoded private CA certificates for `wss://`.
    ///
    /// The bundle is parsed immediately and augments the platform trust store.
    pub fn private_ca_pem(mut self, pem: impl AsRef<[u8]>) -> Result<Self> {
        self.tls_trust = crate::config::parse_private_ca_pem(pem.as_ref())?;
        Ok(self)
    }
}

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
    DtmfEnd { channel_id: String, digit: String },

    /// stop sending media — Asterisk buffer is full
    #[serde(rename = "MEDIA_XOFF")]
    MediaXoff { channel_id: String },

    /// resume sending media — Asterisk buffer drained
    #[serde(rename = "MEDIA_XON")]
    MediaXon { channel_id: String },

    /// channel status response to a GetStatus command
    #[serde(rename = "STATUS")]
    Status {
        channel_id: String,
        queue_length: u32,
        xon_level: u32,
        xoff_level: u32,
        queue_full: bool,
        bulk_media: bool,
        media_paused: bool,
    },

    /// buffering mode completed, optional correlation_id ties to the stop request
    #[serde(rename = "MEDIA_BUFFERING_COMPLETED")]
    MediaBufferingCompleted {
        channel_id: String,
        correlation_id: String,
    },

    /// a previously inserted mark point has been processed
    #[serde(rename = "MEDIA_MARK_PROCESSED")]
    MediaMarkProcessed {
        channel_id: String,
        correlation_id: String,
    },

    /// all queued media has been sent to the channel
    #[serde(rename = "QUEUE_DRAINED")]
    QueueDrained { channel_id: String },

    /// Asterisk rejected a media control command
    #[serde(rename = "ERROR")]
    Error {
        channel_id: String,
        error_text: String,
    },
}

/// media flow direction selected by `SET_MEDIA_DIRECTION`
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum MediaDirection {
    Both,
    In,
    Out,
}

/// Latest flow-control instruction received from Asterisk.
///
/// This state is retained independently of the bounded event queue, so a slow
/// event consumer can always observe the most recent `MEDIA_XON`/`MEDIA_XOFF`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MediaFlowControl {
    /// No flow-control event has been received yet.
    Unknown,
    /// Asterisk sent `MEDIA_XON`; outbound media may flow.
    Flowing,
    /// Asterisk sent `MEDIA_XOFF`; the application should pause outbound media.
    Paused,
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
    Hangup,

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
    MarkMedia {
        #[serde(skip_serializing_if = "Option::is_none")]
        correlation_id: Option<String>,
    },

    /// request channel status
    #[serde(rename = "GET_STATUS")]
    GetStatus,

    /// request notification when the media queue is empty
    #[serde(rename = "REPORT_QUEUE_DRAINED")]
    ReportQueueDrained,

    /// select bidirectional, inbound, or outbound media flow
    #[serde(rename = "SET_MEDIA_DIRECTION")]
    SetMediaDirection { direction: MediaDirection },
}

/// connection to an Asterisk WebSocket media channel
///
/// exchanges raw audio frames and JSON control commands with Asterisk's
/// chan_websocket channel driver. The Asterisk channel must be created with
/// [`crate::resources::channel::ExternalMediaParams::websocket_json`]. Splits
/// incoming traffic: text frames become [`MediaEvent`]s, binary frames become
/// raw audio buffers.
///
/// the connection runs in a background task; dropping the channel
/// shuts it down.
pub struct MediaChannel {
    event_rx: mpsc::Receiver<MediaEvent>,
    audio_rx: mpsc::Receiver<Vec<u8>>,
    control_tx: mpsc::Sender<String>,
    outbound_audio_tx: mpsc::Sender<Vec<u8>>,
    flow_control_rx: watch::Receiver<MediaFlowControl>,
    shutdown_tx: watch::Sender<bool>,
    task: OwnedTask,
}

impl std::fmt::Debug for MediaChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaChannel")
            .field("connected", &!self.control_tx.is_closed())
            .finish()
    }
}

/// type alias for an outbound (client-initiated) websocket stream
type OutboundWsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// type alias for an accepted (server-side) websocket stream over raw TCP
type AcceptedWsStream = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

impl MediaChannel {
    /// connect to an Asterisk JSON-control media websocket endpoint
    ///
    /// url should be the full websocket URL including the connection_id path,
    /// e.g. `ws://asterisk:8088/media/32966726-4388-456b-a333-fdf5dbecc60d`
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with_options(url, MediaConnectionOptions::default()).await
    }

    /// connect with explicit cleartext and private-CA policy
    pub async fn connect_with_options(url: &str, options: MediaConnectionOptions) -> Result<Self> {
        let parsed =
            url::Url::parse(url).map_err(|error| AriError::InvalidUrl(error.to_string()))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| AriError::InvalidUrl("media websocket URL has no host".to_owned()))?;
        if parsed.scheme() == "ws"
            && !options.allow_insecure_remote
            && !crate::config::is_loopback_host(host)
        {
            return Err(AriError::InvalidConfig(format!(
                "cleartext media websocket to non-loopback host '{host}' requires allow_insecure_remote(true)"
            )));
        }
        let tls_connector =
            crate::websocket::connector_for_url(url, &options.tls_trust.rustls_roots)?;
        let (ws_stream, _) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio_tungstenite::connect_async_tls_with_config(
                url,
                Some(crate::websocket::websocket_config(MAX_MEDIA_PAYLOAD_BYTES)),
                false,
                Some(tls_connector),
            ),
        )
        .await
        .map_err(|_| AriError::WebSocket("media websocket connection timed out".to_owned()))?
        .map_err(|e| AriError::WebSocket(e.to_string()))?;

        Ok(Self::spawn_outbound(ws_stream))
    }

    /// create from an already-accepted JSON-control websocket stream over raw TCP
    ///
    /// useful when running a media server that accepts incoming connections
    /// The accepting server remains responsible for configuring frame, message,
    /// and write-buffer limits before passing the established stream here.
    pub fn from_accepted(ws_stream: AcceptedWsStream) -> Result<Self> {
        let config = ws_stream.get_config();
        if config
            .max_message_size
            .is_none_or(|limit| limit > MAX_MEDIA_PAYLOAD_BYTES)
            || config
                .max_frame_size
                .is_none_or(|limit| limit > MAX_MEDIA_PAYLOAD_BYTES)
        {
            return Err(AriError::InvalidConfig(format!(
                "accepted media websocket must cap messages and frames at {MAX_MEDIA_PAYLOAD_BYTES} bytes"
            )));
        }
        let (event_tx, event_rx) = mpsc::channel(64);
        let (audio_tx, audio_rx) = mpsc::channel(256);
        let (control_tx, control_rx) = mpsc::channel(64);
        let (outbound_audio_tx, outbound_audio_rx) = mpsc::channel(256);
        let (flow_control_tx, flow_control_rx) = watch::channel(MediaFlowControl::Unknown);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let task_handle = tokio::spawn(media_loop(
            ws_stream,
            event_tx,
            audio_tx,
            control_rx,
            outbound_audio_rx,
            flow_control_tx,
            shutdown_rx,
        ));

        Ok(Self {
            event_rx,
            audio_rx,
            control_tx,
            outbound_audio_tx,
            flow_control_rx,
            shutdown_tx,
            task: OwnedTask::new(task_handle),
        })
    }

    /// accept a raw TCP connection as a bounded JSON-control media WebSocket
    ///
    /// This is preferred to [`Self::from_accepted`] because frame and message
    /// limits are installed during the WebSocket handshake.
    pub async fn accept(stream: tokio::net::TcpStream) -> Result<Self> {
        let websocket = tokio_tungstenite::accept_async_with_config(
            stream,
            Some(crate::websocket::websocket_config(MAX_MEDIA_PAYLOAD_BYTES)),
        )
        .await
        .map_err(|error| AriError::WebSocket(error.to_string()))?;
        Self::from_accepted(websocket)
    }

    fn spawn_outbound(ws_stream: OutboundWsStream) -> Self {
        let (event_tx, event_rx) = mpsc::channel(64);
        let (audio_tx, audio_rx) = mpsc::channel(256);
        let (control_tx, control_rx) = mpsc::channel(64);
        let (outbound_audio_tx, outbound_audio_rx) = mpsc::channel(256);
        let (flow_control_tx, flow_control_rx) = watch::channel(MediaFlowControl::Unknown);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let task_handle = tokio::spawn(media_loop(
            ws_stream,
            event_tx,
            audio_tx,
            control_rx,
            outbound_audio_rx,
            flow_control_tx,
            shutdown_rx,
        ));

        Self {
            event_rx,
            audio_rx,
            control_tx,
            outbound_audio_tx,
            flow_control_rx,
            shutdown_tx,
            task: OwnedTask::new(task_handle),
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

    /// enqueue a control command for transmission to Asterisk
    ///
    /// Success means the bounded, priority control queue accepted the command;
    /// it does not acknowledge socket transmission or Asterisk processing.
    pub async fn send_command(&self, cmd: MediaCommand) -> Result<()> {
        let json = serde_json::to_string(&cmd).map_err(AriError::Json)?;
        self.control_tx
            .send(json)
            .await
            .map_err(|_| AriError::Disconnected)
    }

    /// enqueue raw audio data for transmission to Asterisk
    ///
    /// data should be encoded in the format negotiated during MEDIA_START.
    /// Asterisk will re-frame if buffering mode is active. max 65500 bytes.
    /// Success means the bounded audio queue accepted the frame; it does not
    /// acknowledge socket transmission. Control commands use a separate,
    /// higher-priority queue.
    pub async fn send_audio(&self, data: Vec<u8>) -> Result<()> {
        if data.len() > MAX_MEDIA_PAYLOAD_BYTES {
            return Err(AriError::WebSocket(format!(
                "audio frame too large: {} bytes (max 65500)",
                data.len()
            )));
        }
        self.outbound_audio_tx
            .send(data)
            .await
            .map_err(|_| AriError::Disconnected)
    }

    /// return the latest flow-control state without consuming an event
    pub fn flow_control(&self) -> MediaFlowControl {
        *self.flow_control_rx.borrow()
    }

    /// wait until the retained flow-control state changes
    ///
    /// Returns `None` when the media actor has stopped.
    pub async fn flow_control_changed(&mut self) -> Option<MediaFlowControl> {
        self.flow_control_rx.changed().await.ok()?;
        Some(*self.flow_control_rx.borrow_and_update())
    }

    /// answer the channel
    pub async fn answer(&self) -> Result<()> {
        self.send_command(MediaCommand::Answer).await
    }

    /// hang up the channel
    pub async fn hangup(&self) -> Result<()> {
        self.send_command(MediaCommand::Hangup).await
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
    pub async fn mark(&self, correlation_id: Option<String>) -> Result<()> {
        self.send_command(MediaCommand::MarkMedia { correlation_id })
            .await
    }

    /// request channel status
    pub async fn get_status(&self) -> Result<()> {
        self.send_command(MediaCommand::GetStatus).await
    }

    /// request notification when the media queue is empty
    pub async fn report_queue_drained(&self) -> Result<()> {
        self.send_command(MediaCommand::ReportQueueDrained).await
    }

    /// select the media flow direction
    pub async fn set_media_direction(&self, direction: MediaDirection) -> Result<()> {
        self.send_command(MediaCommand::SetMediaDirection { direction })
            .await
    }

    /// shut down the connection
    pub fn disconnect(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task.abort();
    }

    /// shut down the connection cooperatively and wait for the actor to stop
    pub async fn disconnect_and_wait(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task.shutdown_and_wait("ARI media websocket").await;
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
    mut control_rx: mpsc::Receiver<String>,
    mut outbound_audio_rx: mpsc::Receiver<Vec<u8>>,
    flow_control_tx: watch::Sender<MediaFlowControl>,
    mut shutdown_rx: watch::Receiver<bool>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    use tokio_tungstenite::tungstenite::Message;

    let (mut write, mut read) = ws_stream.split();

    loop {
        // Shutdown and control traffic take priority over bulk audio whenever
        // more than one branch is ready.
        tokio::select! {
            biased;
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::debug!("media channel shutdown requested");
                    match tokio::time::timeout(
                        MEDIA_CLOSE_TIMEOUT,
                        write.send(Message::Close(None)),
                    )
                    .await
                    {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            tracing::debug!(error = %error, "failed to send media close frame");
                        }
                        Err(_) => tracing::debug!("timed out sending media close frame"),
                    }
                    return;
                }
            }
            cmd = control_rx.recv() => {
                match cmd {
                    Some(json) => {
                        match tokio::time::timeout(
                            MEDIA_WRITE_TIMEOUT,
                            write.send(Message::Text(json.into())),
                        ).await {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => {
                                tracing::warn!(error = %e, "failed to send media command");
                                return;
                            }
                            Err(_) => {
                                tracing::warn!("timed out sending media command");
                                return;
                            }
                        }
                    }
                    None => return,
                }
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<MediaEvent>(&text) {
                            Ok(event) => {
                                match &event {
                                    MediaEvent::MediaXoff { .. } => {
                                        flow_control_tx.send_replace(MediaFlowControl::Paused);
                                    }
                                    MediaEvent::MediaXon { .. } => {
                                        flow_control_tx.send_replace(MediaFlowControl::Flowing);
                                    }
                                    _ => {}
                                }
                                match event_tx.try_send(event) {
                                    Ok(()) => {}
                                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                        tracing::warn!("media event channel full, dropping event");
                                    }
                                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                        return;
                                    }
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
                        if data.len() > MAX_MEDIA_PAYLOAD_BYTES {
                            tracing::warn!(
                                payload_bytes = data.len(),
                                limit = MAX_MEDIA_PAYLOAD_BYTES,
                                "rejecting oversized inbound media frame"
                            );
                            return;
                        }
                        match audio_tx.try_send(data.to_vec()) {
                            Ok(()) => {}
                            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                tracing::debug!("audio channel full, dropping frame");
                            }
                            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                // receiver dropped
                                return;
                            }
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
            audio = outbound_audio_rx.recv() => {
                match audio {
                    Some(data) => {
                        match tokio::time::timeout(
                            MEDIA_WRITE_TIMEOUT,
                            write.send(Message::Binary(data.into())),
                        ).await {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => {
                                tracing::warn!(error = %e, "failed to send audio frame");
                                return;
                            }
                            Err(_) => {
                                tracing::warn!("timed out sending audio frame");
                                return;
                            }
                        }
                    }
                    None => return,
                }
            }
        }
    }
}
