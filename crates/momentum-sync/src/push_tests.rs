// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
fn capability(url: &str) -> String {
    serde_json::json!({"ocs":{"meta":{"status":"ok","statuscode":200},"data":{"capabilities":{"notify_push":{"endpoints":{"websocket":url}}}}}}).to_string()
}
#[test]
fn discovers_only_authenticated_same_origin_secure_push_endpoints() {
    assert!(endpoint(
        "https://cloud.example/nextcloud",
        &capability("wss://cloud.example/push/ws")
    )
    .is_some());
    for url in [
        "ws://cloud.example/push/ws",
        "wss://other.example/push/ws",
        "wss://cloud.example:444/push/ws",
        "wss://user:password@cloud.example/push/ws",
        "wss://cloud.example/push/ws#fragment",
    ] {
        assert!(endpoint("https://cloud.example", &capability(url)).is_none(), "{url}");
    }
    assert!(endpoint("http://cloud.example", &capability("wss://cloud.example/push/ws")).is_none());
    assert!(endpoint("https://cloud.example", r#"{"ocs":{"data":{"capabilities":{}}}}"#).is_none());
    assert!(endpoint(
        "https://cloud.example",
        &capability("wss://cloud.example/push/ws").replace("\"statuscode\":200", "\"statuscode\":401")
    )
    .is_none());
}

struct Fixture {
    cfg: NextcloudCfg,
    transport: Transport,
    authentications: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    capability_requests: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    capability_reply: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}
fn tls_fixture(messages: Vec<String>) -> Fixture {
    use std::{
        net::TcpListener,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
    };
    let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
    let certificate = certified.cert.der().clone();
    let private = rustls::pki_types::PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let server = Arc::new(
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certificate.clone()], private.into())
            .unwrap(),
    );
    let mut roots = rustls::RootCertStore::empty();
    roots.add(certificate.clone()).unwrap();
    let client = Arc::new(
        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    );
    let agent = ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .root_certs(vec![ureq::tls::Certificate::from_der(certificate.as_ref()).to_owned()].into())
                .build(),
        )
        .build()
        .into();
    let transport = Transport {
        http: agent,
        websocket: Arc::new(move || Some(tungstenite::Connector::Rustls(client.clone()))),
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let websocket = format!("wss://127.0.0.1:{}/push/ws", address.port());
    let server_capability = capability(&websocket);
    let authentications = Arc::new(AtomicUsize::new(0));
    let auth = authentications.clone();
    let capability_requests = Arc::new(AtomicUsize::new(0));
    let requests = capability_requests.clone();
    let capability_reply = Arc::new(std::sync::Mutex::new(None::<String>));
    let reply = capability_reply.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    std::thread::spawn(move || {
        while !stopping.load(Ordering::SeqCst) {
            let Ok((socket, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            let (server, capabilities, messages, auth) = (
                server.clone(),
                server_capability.clone(),
                messages.clone(),
                auth.clone(),
            );
            let (requests, reply) = (requests.clone(), reply.clone());
            std::thread::spawn(move || {
                socket.set_nonblocking(false).unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                socket.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
                let connection = rustls::ServerConnection::new(server).unwrap();
                let mut stream = rustls::StreamOwned::new(connection, socket);
                let mut head = Vec::new();
                let mut byte = [0];
                while !head.ends_with(b"\r\n\r\n") {
                    if stream.read(&mut byte).unwrap_or(0) == 0 || head.len() > 16_384 {
                        return;
                    }
                    head.push(byte[0]);
                }
                let head = String::from_utf8(head).unwrap();
                if head.starts_with("GET /ocs/") {
                    requests.fetch_add(1, Ordering::SeqCst);
                    if let Some(response) = reply.lock().unwrap().as_ref() {
                        let _ = stream.write_all(response.as_bytes());
                        return;
                    }
                    assert!(head
                        .to_lowercase()
                        .contains("authorization: basic dTpw".to_lowercase().as_str()));
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{capabilities}",
                        capabilities.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    return;
                }
                let key = head
                    .lines()
                    .find_map(|line| {
                        let (key, value) = line.split_once(':')?;
                        key.eq_ignore_ascii_case("sec-websocket-key").then(|| value.trim())
                    })
                    .unwrap();
                let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
                if stream.write_all(format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n").as_bytes()).is_err() { return; }
                let mut socket =
                    tungstenite::WebSocket::from_raw_socket(stream, tungstenite::protocol::Role::Server, None);
                let user = socket.read().unwrap();
                let password = socket.read().unwrap();
                if user.to_text().unwrap() != "u" || password.to_text().unwrap() != "p" {
                    let _ = socket.send(tungstenite::Message::Text("err: invalid credentials".into()));
                    return;
                }
                auth.fetch_add(1, Ordering::SeqCst);
                for message in messages {
                    if message == "_slow_fragment" {
                        // A valid unmasked text frame that never finishes within
                        // the test, but delivers data often enough to evade a
                        // per-read socket timeout.
                        let _ = socket.get_mut().write_all(&[0x81, 100, b'x']);
                        for _ in 0..40 {
                            std::thread::sleep(Duration::from_millis(40));
                            if socket.get_mut().write_all(b"x").is_err() {
                                return;
                            }
                        }
                        return;
                    }
                    if socket.send(tungstenite::Message::Text(message.into())).is_err() {
                        return;
                    }
                }
                while socket.read().is_ok() {}
            });
        }
    });
    Fixture {
        cfg: NextcloudCfg {
            server_url: format!("https://127.0.0.1:{}", address.port()),
            user_name: "u".into(),
            password: "p".into(),
            folder: "legacy".into(),
            ..Default::default()
        },
        transport,
        authentications,
        capability_requests,
        capability_reply,
        stop,
    }
}
#[test]
fn tls_websocket_authenticates_and_delivers_only_file_invalidation_hints() {
    let fixture = tls_fixture(vec![
        "notify_file".into(),
        "authenticated".into(),
        "notify_activity".into(),
        "notify_file".into(),
    ]);
    let mut session = PushSession::with_transport(fixture.transport.clone());
    let until = Instant::now() + Duration::from_secs(3);
    let mut received = false;
    while Instant::now() < until {
        if session.poll_hint(&fixture.cfg) {
            received = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        received,
        "authentications={}",
        fixture.authentications.load(std::sync::atomic::Ordering::SeqCst)
    );
    assert_eq!(fixture.authentications.load(std::sync::atomic::Ordering::SeqCst), 1);
    session.invalidate();
    assert!(!session.poll_hint(&fixture.cfg));
}
#[test]
fn unauthenticated_notifications_and_wrong_tls_roots_never_become_hints() {
    for untrusted in 0..3 {
        let fixture = tls_fixture(vec!["notify_file".into(), "err: invalid credentials".into()]);
        let transport = match untrusted {
            0 => fixture.transport.clone(),
            1 => Transport::default(),
            _ => Transport {
                http: fixture.transport.http.clone(),
                websocket: Arc::new(|| None),
            },
        };
        let mut session = PushSession::with_transport(transport);
        let until = Instant::now() + Duration::from_millis(300);
        while Instant::now() < until {
            assert!(!session.poll_hint(&fixture.cfg));
            std::thread::sleep(Duration::from_millis(10));
        }
        if untrusted != 0 {
            assert_eq!(fixture.authentications.load(std::sync::atomic::Ordering::SeqCst), 0);
        }
        session.invalidate();
    }
}
#[test]
fn insecure_provider_never_starts_a_push_worker() {
    let mut session = PushSession::default();
    let cfg = NextcloudCfg {
        server_url: "http://localhost".into(),
        user_name: "u".into(),
        password: "p".into(),
        ..Default::default()
    };
    assert!(!session.poll_hint(&cfg));
    assert!(session.worker.is_none());
}

#[test]
fn discovery_refuses_redirects_and_missing_capabilities_without_retry_storms() {
    for redirect in [true, false] {
        let fixture = tls_fixture(vec![]);
        let body = r#"{"ocs":{"meta":{"statuscode":200},"data":{"capabilities":{}}}}"#;
        *fixture.capability_reply.lock().unwrap() = Some(if redirect {
            "HTTP/1.1 302 Found\r\nLocation: https://127.0.0.1:1/steal\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
        } else {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
        });
        let mut session = PushSession::with_transport(fixture.transport.clone());
        let until = Instant::now() + Duration::from_millis(200);
        while Instant::now() < until {
            assert!(!session.poll_hint(&fixture.cfg));
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(fixture.capability_requests.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.authentications.load(Ordering::SeqCst), 0);
    }
}
#[test]
fn oversized_frames_disconnect_without_hint_and_failed_authentication_backs_off() {
    for messages in [
        vec!["authenticated".into(), "x".repeat(MAX_MESSAGE + 1)],
        vec!["err: invalid credentials".into()],
    ] {
        let fixture = tls_fixture(messages);
        let mut session = PushSession::with_transport(fixture.transport.clone());
        let until = Instant::now() + Duration::from_millis(250);
        while Instant::now() < until {
            assert!(!session.poll_hint(&fixture.cfg));
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(fixture.authentications.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.capability_requests.load(Ordering::SeqCst), 1);
    }
}
#[test]
fn invalidation_closes_worker_and_configuration_changes_cannot_reuse_old_hints() {
    let fixture = tls_fixture(vec!["authenticated".into()]);
    let mut session = PushSession::with_transport(fixture.transport.clone());
    session.poll_hint(&fixture.cfg);
    let old = session.worker.as_ref().unwrap().control.clone();
    old.hint.store(true, Ordering::SeqCst);
    let changed = NextcloudCfg {
        password: "changed".into(),
        ..fixture.cfg.clone()
    };
    assert!(!session.poll_hint(&changed));
    assert!(old.stop.load(Ordering::SeqCst));
    assert!(!old.hint.load(Ordering::SeqCst));
    session.invalidate();
    let until = Instant::now() + Duration::from_secs(2);
    while session
        .worker
        .as_ref()
        .is_some_and(|worker| !worker.thread.is_finished())
        && Instant::now() < until
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(session.worker.as_ref().is_none_or(|worker| worker.thread.is_finished()));
}
#[test]
fn activity_lease_stops_an_abandoned_native_listener() {
    let fixture = tls_fixture(vec!["authenticated".into()]);
    let mut session = PushSession::with_transport(fixture.transport.clone());
    session.poll_hint(&fixture.cfg);
    let worker = session.worker.as_ref().unwrap();
    *worker.control.last_poll.lock().unwrap() = Instant::now() - LEASE - Duration::from_secs(1);
    let until = Instant::now() + Duration::from_secs(1);
    while !worker.thread.is_finished() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(worker.thread.is_finished());
    assert!(worker.control.stop.load(Ordering::SeqCst));
}

#[test]
fn an_incomplete_trickling_frame_cannot_extend_the_native_activity_lease() {
    let fixture = tls_fixture(vec!["authenticated".into(), "_slow_fragment".into()]);
    let mut session = PushSession::with_transport(fixture.transport.clone());
    session.poll_hint(&fixture.cfg);
    let until = Instant::now() + Duration::from_secs(2);
    while fixture.authentications.load(Ordering::SeqCst) == 0 && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(fixture.authentications.load(Ordering::SeqCst), 1);
    std::thread::sleep(Duration::from_millis(80));
    let worker = session.worker.as_ref().unwrap();
    *worker.control.last_poll.lock().unwrap() = Instant::now() - LEASE - Duration::from_secs(1);
    let until = Instant::now() + Duration::from_millis(400);
    while !worker.thread.is_finished() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(worker.thread.is_finished());
}

#[test]
fn invalidation_drops_owner_configuration_but_retains_the_worker_handle() {
    let fixture = tls_fixture(vec!["authenticated".into()]);
    let mut session = PushSession::with_transport(fixture.transport.clone());
    session.poll_hint(&fixture.cfg);
    assert!(session.worker.as_ref().unwrap().cfg.is_some());
    session.invalidate();
    assert!(session.worker.is_some());
    assert!(session.worker.as_ref().unwrap().cfg.is_none());
    assert!(session.worker.as_ref().unwrap().control.stop.load(Ordering::SeqCst));
}

#[test]
fn dependency_logging_cannot_emit_credential_frames_even_with_verbose_runtime_filters() {
    // Tungstenite formats authentication frames at trace and peer close text at
    // debug. A compile-time ceiling protects every frontend and logger backend.
    assert!(log::STATIC_MAX_LEVEL <= log::LevelFilter::Info);
}
