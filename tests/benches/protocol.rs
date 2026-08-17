use std::hint::black_box;
use std::time::{Duration, Instant};

use asterisk_rs_ami::codec::AmiCodec;
use asterisk_rs_ami::event::AmiEvent;
use asterisk_rs_ami::tracker::CallTracker;
use asterisk_rs_ari::media::MediaChannel;
use asterisk_rs_core::event::EventBus;
use bytes::BytesMut;
use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio_util::codec::Decoder;

const ITERATIONS: usize = 10_000;

fn report(name: &str, operations: usize, elapsed: Duration) {
    let nanos_per_operation = elapsed.as_nanos() / operations as u128;
    println!("{name}: {operations} operations in {elapsed:?} ({nanos_per_operation} ns/op)");
}

fn codec_decode() {
    let wire = b"Asterisk Call Manager/6.0.0\r\nEvent: Newchannel\r\nChannel: PJSIP/100-0001\r\nUniqueid: bench-1\r\nLinkedid: bench-1\r\n\r\n";
    let started = Instant::now();
    for _ in 0..ITERATIONS {
        let mut buffer = BytesMut::from(&wire[..]);
        let message = AmiCodec::new()
            .decode(&mut buffer)
            .expect("benchmark fixture should decode")
            .expect("benchmark fixture should be complete");
        black_box(message);
    }
    report("ami codec decode", ITERATIONS, started.elapsed());
}

async fn tracker_admission() {
    let bus = EventBus::<AmiEvent>::new(ITERATIONS * 2);
    let (tracker, mut completed) = CallTracker::new(bus.subscribe());
    let started = Instant::now();
    for index in 0..ITERATIONS {
        let unique_id = index.to_string();
        bus.publish(AmiEvent::NewChannel {
            channel: format!("PJSIP/{unique_id}"),
            channel_state: "0".into(),
            channel_state_desc: "Down".into(),
            caller_id_num: "100".into(),
            caller_id_name: "Benchmark".into(),
            unique_id: unique_id.clone(),
            linked_id: unique_id.clone(),
        });
        bus.publish(AmiEvent::Hangup {
            channel: format!("PJSIP/{unique_id}"),
            unique_id,
            cause: 16,
            cause_txt: "Normal Clearing".into(),
        });
        black_box(
            completed
                .recv()
                .await
                .expect("tracker should complete admitted call"),
        );
    }
    report("ami tracker lifecycle", ITERATIONS, started.elapsed());
    tracker.shutdown();
}

async fn media_admission() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("benchmark listener should bind");
    let address = listener.local_addr().expect("benchmark listener address");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("benchmark media accept");
        let mut websocket = tokio_tungstenite::accept_async(stream)
            .await
            .expect("benchmark websocket handshake");
        let mut received = 0;
        while received < ITERATIONS {
            if websocket.next().await.is_none() {
                break;
            }
            received += 1;
        }
        received
    });
    let media = MediaChannel::connect(&format!("ws://{address}/media/benchmark"))
        .await
        .expect("benchmark media client should connect");
    let payload = vec![0_u8; 320];
    let started = Instant::now();
    for _ in 0..ITERATIONS {
        media
            .send_audio(black_box(payload.clone()))
            .await
            .expect("media queue should admit benchmark frame");
    }
    let received = server.await.expect("benchmark media server task");
    assert_eq!(
        received, ITERATIONS,
        "all admitted frames should reach peer"
    );
    report("ari media admission", ITERATIONS, started.elapsed());
    media.disconnect_and_wait().await;
}

fn main() {
    codec_decode();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("benchmark runtime should build");
    runtime.block_on(async {
        tracker_admission().await;
        media_admission().await;
    });
}
