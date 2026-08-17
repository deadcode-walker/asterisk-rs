#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use asterisk_rs_ari::AriClient;
use asterisk_rs_ari::config::{AriConfigBuilder, TransportMode};
use asterisk_rs_ari::error::AriError;
use asterisk_rs_ari::media::{MAX_MEDIA_PAYLOAD_BYTES, MediaChannel, MediaConnectionOptions};
use asterisk_rs_core::config::ReconnectPolicy;
use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::signature::{
    ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair as SignatureKeyPair,
};
use futures_util::{SinkExt, StreamExt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::Message;

struct PrivateCaFixture {
    ca_pem: String,
    acceptor: TlsAcceptor,
}

fn der(tag: u8, contents: &[u8]) -> Vec<u8> {
    let mut encoded = vec![tag];
    if contents.len() < 128 {
        encoded.push(contents.len() as u8);
    } else {
        let length = contents.len().to_be_bytes();
        let first = length
            .iter()
            .position(|byte| *byte != 0)
            .unwrap_or(length.len() - 1);
        encoded.push(0x80 | (length.len() - first) as u8);
        encoded.extend_from_slice(&length[first..]);
    }
    encoded.extend_from_slice(contents);
    encoded
}

fn sequence(parts: &[&[u8]]) -> Vec<u8> {
    der(0x30, &parts.concat())
}

fn distinguished_name(common_name: &str) -> Vec<u8> {
    let oid_common_name = der(0x06, &[0x55, 0x04, 0x03]);
    let value = der(0x0c, common_name.as_bytes());
    let attribute = sequence(&[&oid_common_name, &value]);
    let set = der(0x31, &attribute);
    sequence(&[&set])
}

fn signature_algorithm() -> Vec<u8> {
    // ecdsa-with-SHA256 (1.2.840.10045.4.3.2)
    let oid = der(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02]);
    sequence(&[&oid])
}

fn subject_public_key_info(public_key: &[u8]) -> Vec<u8> {
    // id-ecPublicKey (1.2.840.10045.2.1) with prime256v1 (1.2.840.10045.3.1.7).
    let algorithm = sequence(&[
        &der(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01]),
        &der(0x06, &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07]),
    ]);
    let mut bit_string = vec![0];
    bit_string.extend_from_slice(public_key);
    sequence(&[&algorithm, &der(0x03, &bit_string)])
}

fn ca_extension() -> Vec<u8> {
    let basic_constraints = sequence(&[&der(0x01, &[0xff])]);
    sequence(&[
        &der(0x06, &[0x55, 0x1d, 0x13]),
        &der(0x01, &[0xff]),
        &der(0x04, &basic_constraints),
    ])
}

fn leaf_basic_constraints_extension() -> Vec<u8> {
    sequence(&[
        &der(0x06, &[0x55, 0x1d, 0x13]),
        &der(0x01, &[0xff]),
        &der(0x04, &sequence(&[])),
    ])
}

fn key_usage_extension(bits: u8, unused_bits: u8) -> Vec<u8> {
    let usage = der(0x03, &[unused_bits, bits]);
    sequence(&[
        &der(0x06, &[0x55, 0x1d, 0x0f]),
        &der(0x01, &[0xff]),
        &der(0x04, &usage),
    ])
}

fn server_auth_extension() -> Vec<u8> {
    let server_auth = der(0x06, &[0x2b, 0x06, 0x01, 0x05, 0x05, 0x07, 0x03, 0x01]);
    sequence(&[
        &der(0x06, &[0x55, 0x1d, 0x25]),
        &der(0x04, &sequence(&[&server_auth])),
    ])
}

fn subject_alt_name_extension(hostname: &str) -> Vec<u8> {
    let names = sequence(&[&der(0x82, hostname.as_bytes())]);
    sequence(&[&der(0x06, &[0x55, 0x1d, 0x11]), &der(0x04, &names)])
}

fn utc_time(day_offset: i64) -> Vec<u8> {
    let seconds = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after Unix epoch")
            .as_secs(),
    )
    .expect("current timestamp fits i64")
        + day_offset * 86_400;
    let days = seconds.div_euclid(86_400);
    let seconds_in_day = seconds.rem_euclid(86_400);

    // Gregorian civil date from days since 1970-01-01.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);

    assert!((1950..=2049).contains(&year), "UTCTime year out of range");
    let hour = seconds_in_day / 3_600;
    let minute = seconds_in_day % 3_600 / 60;
    let second = seconds_in_day % 60;
    der(
        0x17,
        format!(
            "{:02}{month:02}{day:02}{hour:02}{minute:02}{second:02}Z",
            year % 100
        )
        .as_bytes(),
    )
}

#[allow(clippy::too_many_arguments)]
fn certificate_der(
    serial: u8,
    issuer: &[u8],
    subject: &[u8],
    public_key: &[u8],
    extensions: &[Vec<u8>],
    signer: &EcdsaKeyPair,
    rng: &SystemRandom,
) -> Vec<u8> {
    let version = der(0xa0, &der(0x02, &[2]));
    let serial = der(0x02, &[serial]);
    let algorithm = signature_algorithm();
    let validity = sequence(&[&utc_time(-1), &utc_time(364)]);
    let public_key = subject_public_key_info(public_key);
    let extension_refs = extensions.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let extensions = der(0xa3, &sequence(&extension_refs));
    let tbs = sequence(&[
        &version,
        &serial,
        &algorithm,
        issuer,
        &validity,
        subject,
        &public_key,
        &extensions,
    ]);
    let signature = signer.sign(rng, &tbs).expect("certificate signature");
    let mut signature_bits = vec![0];
    signature_bits.extend_from_slice(signature.as_ref());
    sequence(&[&tbs, &algorithm, &der(0x03, &signature_bits)])
}

fn pem_certificate(certificate: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::new();
    for chunk in certificate.chunks(3) {
        let value = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        encoded.push(ALPHABET[((value >> 18) & 63) as usize] as char);
        encoded.push(ALPHABET[((value >> 12) & 63) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[((value >> 6) & 63) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(value & 63) as usize] as char
        } else {
            '='
        });
    }
    let body = encoded
        .as_bytes()
        .chunks(64)
        .map(|line| std::str::from_utf8(line).expect("base64 is ASCII"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("-----BEGIN CERTIFICATE-----\n{body}\n-----END CERTIFICATE-----\n")
}

impl PrivateCaFixture {
    fn new() -> Self {
        let rng = SystemRandom::new();
        let ca_pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
            .expect("CA key generation");
        let ca_key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, ca_pkcs8.as_ref())
            .expect("CA key parsing");
        let ca_name = distinguished_name("asterisk-rs test CA");
        let ca_der = certificate_der(
            1,
            &ca_name,
            &ca_name,
            ca_key.public_key().as_ref(),
            &[
                ca_extension(),
                key_usage_extension(0x06, 1), // keyCertSign and cRLSign
            ],
            &ca_key,
            &rng,
        );

        let leaf_pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, &rng)
            .expect("leaf key generation");
        let leaf_key =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, leaf_pkcs8.as_ref())
                .expect("leaf key parsing");
        let leaf_der = certificate_der(
            2,
            &ca_name,
            &distinguished_name("localhost"),
            leaf_key.public_key().as_ref(),
            &[
                leaf_basic_constraints_extension(),
                subject_alt_name_extension("localhost"),
                key_usage_extension(0x80, 7), // digitalSignature
                server_auth_extension(),
            ],
            &ca_key,
            &rng,
        );

        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("TLS protocol versions")
            .with_no_client_auth()
            .with_single_cert(
                vec![
                    CertificateDer::from(leaf_der),
                    CertificateDer::from(ca_der.clone()),
                ],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(leaf_pkcs8.as_ref().to_vec())),
            )
            .expect("TLS server certificate");
        Self {
            ca_pem: pem_certificate(&ca_der),
            acceptor: TlsAcceptor::from(Arc::new(config)),
        }
    }
}

fn secure_ari_config(
    port: u16,
    transport: TransportMode,
    ca_pem: Option<&str>,
) -> asterisk_rs_ari::config::AriConfig {
    let mut builder = AriConfigBuilder::new("test-app")
        .host("localhost")
        .port(port)
        .username("testuser")
        .password("testpass")
        .secure(true)
        .transport(transport)
        .reconnect(ReconnectPolicy::none())
        // Windows platform trust evaluation can take slightly more than two
        // seconds on a cold verifier. Keep the fixture finite without racing
        // the certificate result against the client startup deadline.
        .request_timeout(Duration::from_secs(10));
    if let Some(ca_pem) = ca_pem {
        builder = builder.private_ca_pem(ca_pem.as_bytes());
    }
    builder.build().expect("secure ARI config")
}

async fn bind_tls() -> (TcpListener, PrivateCaFixture) {
    (
        TcpListener::bind("127.0.0.1:0")
            .await
            .expect("TLS listener"),
        PrivateCaFixture::new(),
    )
}

async fn accept_one_websocket(listener: TcpListener, acceptor: TlsAcceptor) {
    let (tcp, _) = listener.accept().await.expect("TCP connection");
    let tls = acceptor.accept(tcp).await.expect("trusted TLS handshake");
    let mut websocket = tokio_tungstenite::accept_async(tls)
        .await
        .expect("WebSocket handshake");
    while websocket.next().await.is_some() {}
}

async fn reject_untrusted_tls(listener: TcpListener, acceptor: TlsAcceptor) {
    let (tcp, _) = listener.accept().await.expect("TCP connection");
    assert!(
        acceptor.accept(tcp).await.is_err(),
        "untrusted CA must abort TLS"
    );
}

async fn stop_rejected_tls_fixture(task: tokio::task::JoinHandle<()>) {
    // Some platform TLS stacks do not send a close alert when the client-side
    // verifier rejects a certificate. The client rejection is the behavior
    // under test; do not make fixture teardown depend on a peer close alert.
    task.abort();
    match task.await {
        Ok(()) => {}
        Err(error) if error.is_cancelled() => {}
        Err(error) => panic!("TLS fixture task failed: {error}"),
    }
}

async fn private_ca_secures_http_events_and_https_requests_case() {
    let (listener, fixture) = bind_tls().await;
    let port = listener.local_addr().expect("listener address").port();

    let untrusted = tokio::spawn(reject_untrusted_tls(listener, fixture.acceptor.clone()));
    let error = AriClient::connect(secure_ari_config(port, TransportMode::Http, None))
        .await
        .expect_err("private CA must not be trusted implicitly");
    assert!(matches!(error, AriError::WebSocket(_)));
    stop_rejected_tls_fixture(untrusted).await;

    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("rebind TLS listener");
    let acceptor = fixture.acceptor.clone();
    let server = tokio::spawn(async move {
        let (event_tcp, _) = listener.accept().await.expect("event connection");
        let event_tls = acceptor
            .accept(event_tcp)
            .await
            .expect("event TLS handshake");
        let mut events = tokio_tungstenite::accept_async(event_tls)
            .await
            .expect("event WebSocket handshake");
        let event_task = tokio::spawn(async move { while events.next().await.is_some() {} });

        let (http_tcp, _) = listener.accept().await.expect("HTTPS connection");
        let mut http_tls = acceptor
            .accept(http_tcp)
            .await
            .expect("HTTPS TLS handshake");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            let bytes = http_tls.read(&mut chunk).await.expect("HTTPS request");
            assert!(bytes > 0, "HTTPS request ended before headers");
            request.extend_from_slice(&chunk[..bytes]);
        }
        assert!(request.starts_with(b"GET /ari/channels HTTP/1.1\r\n"));
        http_tls
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]")
            .await
            .expect("HTTPS response");
        http_tls.shutdown().await.expect("HTTPS shutdown");
        event_task.abort();
        match event_task.await {
            Ok(()) => {}
            Err(error) if error.is_cancelled() => {}
            Err(error) => panic!("event fixture task failed: {error}"),
        }
    });

    let client = AriClient::connect(secure_ari_config(
        port,
        TransportMode::Http,
        Some(&fixture.ca_pem),
    ))
    .await
    .expect("private CA should secure the event WebSocket");
    let channels: Vec<serde_json::Value> = client
        .get("channels")
        .await
        .expect("same private CA should secure HTTPS requests");
    assert!(channels.is_empty());
    client.disconnect_and_wait().await;
    server.await.expect("trusted HTTP server task");
}

async fn private_ca_secures_unified_websocket_transport_case() {
    let (listener, fixture) = bind_tls().await;
    let port = listener.local_addr().expect("listener address").port();
    let untrusted = tokio::spawn(reject_untrusted_tls(listener, fixture.acceptor.clone()));
    AriClient::connect(secure_ari_config(port, TransportMode::WebSocket, None))
        .await
        .expect_err("private CA must not be trusted implicitly");
    stop_rejected_tls_fixture(untrusted).await;

    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("rebind TLS listener");
    let server = tokio::spawn(accept_one_websocket(listener, fixture.acceptor.clone()));
    let client = AriClient::connect(secure_ari_config(
        port,
        TransportMode::WebSocket,
        Some(&fixture.ca_pem),
    ))
    .await
    .expect("private CA should secure unified WSS");
    client.disconnect_and_wait().await;
    server.await.expect("unified WSS server task");
}

async fn private_ca_secures_media_websocket_case() {
    let (listener, fixture) = bind_tls().await;
    let port = listener.local_addr().expect("listener address").port();
    let url = format!("wss://localhost:{port}/media/test");
    let untrusted = tokio::spawn(reject_untrusted_tls(listener, fixture.acceptor.clone()));
    MediaChannel::connect(&url)
        .await
        .expect_err("private CA must not be trusted implicitly");
    stop_rejected_tls_fixture(untrusted).await;

    let listener = TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("rebind TLS listener");
    let server = tokio::spawn(accept_one_websocket(listener, fixture.acceptor.clone()));
    let options = MediaConnectionOptions::new()
        .private_ca_pem(fixture.ca_pem.as_bytes())
        .expect("private CA PEM");
    let media = MediaChannel::connect_with_options(&url, options)
        .await
        .expect("private CA should secure media WSS");
    media.disconnect_and_wait().await;
    server.await.expect("media WSS server task");
}

async fn bounded_private_ca_case(future: impl std::future::Future<Output = ()>) {
    tokio::time::timeout(Duration::from_secs(30), future)
        .await
        .expect("private-CA fixture must terminate within its platform-independent deadline");
}

#[tokio::test]
async fn private_ca_secures_http_events_and_https_requests() {
    bounded_private_ca_case(Box::pin(
        private_ca_secures_http_events_and_https_requests_case(),
    ))
    .await;
}

#[tokio::test]
async fn private_ca_secures_unified_websocket_transport() {
    bounded_private_ca_case(Box::pin(
        private_ca_secures_unified_websocket_transport_case(),
    ))
    .await;
}

#[tokio::test]
async fn private_ca_secures_media_websocket() {
    bounded_private_ca_case(Box::pin(private_ca_secures_media_websocket_case())).await;
}

#[tokio::test]
async fn remote_cleartext_media_requires_explicit_opt_in() {
    let error = MediaChannel::connect("ws://0.0.0.0:1/media/test")
        .await
        .expect_err("remote cleartext media must be rejected before I/O");
    assert!(
        matches!(error, AriError::InvalidConfig(message) if message.contains("allow_insecure_remote"))
    );

    let options = MediaConnectionOptions::new().allow_insecure_remote(true);
    let error = MediaChannel::connect_with_options("ws://0.0.0.0:1/media/test", options)
        .await
        .expect_err("opted-in connection should proceed to the expected network failure");
    assert!(!matches!(error, AriError::InvalidConfig(_)));
}

#[tokio::test]
async fn media_accepts_65500_byte_inbound_payload() {
    let (mut media, server) = media_with_payload(MAX_MEDIA_PAYLOAD_BYTES).await;
    let audio = tokio::time::timeout(std::time::Duration::from_secs(1), media.recv_audio())
        .await
        .expect("receive should be bounded")
        .expect("maximum-sized media should be delivered");
    assert_eq!(audio.len(), MAX_MEDIA_PAYLOAD_BYTES);
    server.await.expect("server should finish");
}

#[tokio::test]
async fn media_rejects_65501_byte_inbound_payload() {
    let (mut media, server) = media_with_payload(MAX_MEDIA_PAYLOAD_BYTES + 1).await;
    let audio = tokio::time::timeout(std::time::Duration::from_secs(1), media.recv_audio())
        .await
        .expect("media actor should close promptly after oversized input");
    assert!(
        audio.is_none(),
        "oversized media must not be cloned into the audio queue"
    );
    server.await.expect("server should finish");
}

#[tokio::test]
async fn media_rejects_accepted_stream_without_protocol_limits() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let client = tokio::spawn(async move {
        tokio_tungstenite::connect_async(format!("ws://{address}/media/test"))
            .await
            .unwrap()
    });
    let (stream, _) = listener.accept().await.unwrap();
    let accepted = tokio_tungstenite::accept_async(stream).await.unwrap();

    let error = MediaChannel::from_accepted(accepted)
        .expect_err("default Tungstenite limits exceed the media protocol cap");
    assert!(matches!(error, AriError::InvalidConfig(message) if message.contains("65500")));
    client.abort();
}

async fn media_with_payload(bytes: usize) -> (MediaChannel, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut websocket = tokio_tungstenite::accept_async(stream).await.unwrap();
        let _ = websocket
            .send(Message::Binary(vec![0x55; bytes].into()))
            .await;
        let _ = websocket.close(None).await;
    });
    let media = MediaChannel::connect(&format!("ws://{address}/media/test"))
        .await
        .unwrap();
    (media, server)
}
