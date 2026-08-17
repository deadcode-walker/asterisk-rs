use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use asterisk_rs_ari::AriError;
use asterisk_rs_ari::server::AriServerBuilder;

#[tokio::test]
async fn external_bind_requires_explicit_opt_in() {
    let error = AriServerBuilder::new()
        .bind("0.0.0.0:0".parse().expect("valid address"))
        .build()
        .await
        .expect_err("external bind must be explicit");
    assert!(matches!(error, AriError::InvalidConfig(_)));
}

#[tokio::test]
async fn admission_hook_rejects_before_websocket_handshake() {
    let admissions = Arc::new(AtomicUsize::new(0));
    let handlers = Arc::new(AtomicUsize::new(0));
    let admission_count = Arc::clone(&admissions);
    let (server, shutdown) = AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid address"))
        .admission_hook(move |_peer: SocketAddr| {
            admission_count.fetch_add(1, Ordering::SeqCst);
            false
        })
        .build()
        .await
        .expect("server builds");
    let address = server.local_addr().expect("local address");
    let handler_count = Arc::clone(&handlers);
    let task = tokio::spawn(server.run(move |_| {
        handler_count.fetch_add(1, Ordering::SeqCst);
        async {}
    }));

    let _stream = tokio::net::TcpStream::connect(address)
        .await
        .expect("TCP connection reaches admission hook");
    tokio::time::timeout(Duration::from_secs(1), async {
        while admissions.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("admission hook observed connection");
    shutdown.shutdown();
    task.await
        .expect("server task joins")
        .expect("clean shutdown");

    assert_eq!(admissions.load(Ordering::SeqCst), 1);
    assert_eq!(handlers.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn handler_panic_is_an_observable_server_error() {
    let (server, _shutdown) = AriServerBuilder::new()
        .bind("127.0.0.1:0".parse().expect("valid address"))
        .build()
        .await
        .expect("server builds");
    let address = server.local_addr().expect("local address");
    let task = tokio::spawn(server.run(|_| async { panic!("intentional handler panic") }));
    let url = format!("ws://{address}");
    let (_socket, _) = tokio_tungstenite::connect_async(url)
        .await
        .expect("websocket connects");

    let error = tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .expect("server reports panic promptly")
        .expect("server task joins")
        .expect_err("handler panic must fail server");
    assert!(
        matches!(error, AriError::SessionTaskFailed { details } if details.contains("panicked"))
    );
}
