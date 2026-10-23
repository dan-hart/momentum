// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional Nextcloud Client Push. Notifications only invalidate local knowledge;
//! periodic authoritative WebDAV reconciliation remains necessary. Protocol:
//! https://github.com/nextcloud/notify_push/blob/main/DEVELOPING.md
use super::*;
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;
use tungstenite::{protocol::WebSocketConfig, Message};
use url::Url;

const LEASE: Duration = Duration::from_secs(180);
const CONNECT_BUDGET: Duration = Duration::from_secs(10);
const CAPABILITY_TTL: Duration = Duration::from_secs(300);
const MAX_MESSAGE: usize = 16 << 10;

fn server_url(server: &str) -> Option<Url> {
    let url = Url::parse(server).ok()?;
    (url.scheme() == "https"
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none())
    .then_some(url)
}
fn endpoint(server: &str, capabilities: &str) -> Option<Url> {
    let server = server_url(server)?;
    let document: Value = serde_json::from_str(capabilities).ok()?;
    if document.pointer("/ocs/meta/statuscode")?.as_u64()? != 200 {
        return None;
    }
    let endpoint = Url::parse(
        document
            .pointer("/ocs/data/capabilities/notify_push/endpoints/websocket")?
            .as_str()?,
    )
    .ok()?;
    (endpoint.scheme() == "wss"
        && endpoint.host_str() == server.host_str()
        && endpoint.port_or_known_default() == server.port_or_known_default()
        && endpoint.username().is_empty()
        && endpoint.password().is_none()
        && endpoint.fragment().is_none())
    .then_some(endpoint)
}

#[derive(Clone, PartialEq, Eq)]
struct PushConfig {
    server: String,
    user: String,
    password: String,
}
impl PushConfig {
    fn from_cfg(cfg: &NextcloudCfg) -> Option<Self> {
        server_url(&cfg.server_url)?;
        (!cfg.user_name.is_empty() && !cfg.password.is_empty()).then(|| Self {
            server: cfg.server_url.clone(),
            user: cfg.user_name.clone(),
            password: cfg.password.clone(),
        })
    }
    fn auth(&self) -> String {
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", self.user, self.password))
        )
    }
}
#[derive(Clone)]
struct Transport {
    http: ureq::Agent,
    websocket: Arc<dyn Fn() -> Option<tungstenite::Connector> + Send + Sync>,
}
impl Default for Transport {
    fn default() -> Self {
        Self {
            http: transport_pool(),
            websocket: Arc::new(|| None),
        }
    }
}
struct Control {
    stop: AtomicBool,
    hint: AtomicBool,
    last_poll: Mutex<Instant>,
    socket: Mutex<Option<TcpStream>>,
}
impl Control {
    fn active(&self) -> bool {
        !self.stop.load(Ordering::Acquire) && self.last_poll.lock().is_ok_and(|last| last.elapsed() < LEASE)
    }
    fn cancel(&self) {
        self.stop.store(true, Ordering::Release);
        self.hint.store(false, Ordering::Release);
        if let Ok(socket) = self.socket.lock() {
            if let Some(socket) = socket.as_ref() {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
    }
    fn wait(&self, duration: Duration) {
        let until = Instant::now() + duration;
        while self.active() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100).min(until.saturating_duration_since(Instant::now())));
        }
    }
}
struct Worker {
    // The owner retains only a configuration fingerprint; raw credentials live
    // in the bounded worker, never in an abandoned finished JoinHandle.
    cfg: Option<String>,
    control: Arc<Control>,
    thread: JoinHandle<()>,
}
/// A nonblocking, device-local hint source. Call only while Nextcloud is selected
/// and the native app is active. A stopped/unavailable listener never disables
/// polling. Dropping/invalidation closes the socket without waiting for network I/O.
#[derive(Default)]
pub struct PushSession {
    worker: Option<Worker>,
    transport: Transport,
}
impl PushSession {
    #[cfg(test)]
    fn with_transport(transport: Transport) -> Self {
        Self {
            worker: None,
            transport,
        }
    }
    pub fn poll_hint(&mut self, cfg: &NextcloudCfg) -> bool {
        let config = PushConfig::from_cfg(cfg);
        let fingerprint = config.as_ref().map(|cfg| {
            sha256_hex(
                &serde_json::to_vec(&(&cfg.server, &cfg.user, &cfg.password)).expect("configuration strings serialize"),
            )
        });
        if let Some(worker) = self.worker.as_mut() {
            if fingerprint.as_ref() != worker.cfg.as_ref() {
                worker.cfg = None;
                worker.control.cancel();
            }
            if worker.thread.is_finished() {
                if let Some(worker) = self.worker.take() {
                    let _ = worker.thread.join();
                }
            } else if worker.control.stop.load(Ordering::Acquire) {
                return false;
            }
        }
        let Some(config) = config else {
            return false;
        };
        if self.worker.is_none() {
            let control = Arc::new(Control {
                stop: AtomicBool::new(false),
                hint: AtomicBool::new(false),
                last_poll: Mutex::new(Instant::now()),
                socket: Mutex::new(None),
            });
            let (cfg, state, transport) = (config.clone(), control.clone(), self.transport.clone());
            if let Ok(thread) = std::thread::Builder::new()
                .name("momentum-client-push".into())
                .spawn(move || run_worker(cfg, state, transport))
            {
                self.worker = Some(Worker {
                    cfg: fingerprint,
                    control,
                    thread,
                });
            }
        }
        self.worker.as_ref().is_some_and(|worker| {
            if let Ok(mut last) = worker.control.last_poll.lock() {
                *last = Instant::now();
            }
            worker.control.hint.swap(false, Ordering::AcqRel)
        })
    }
    pub fn invalidate(&mut self) {
        if let Some(worker) = self.worker.as_mut() {
            worker.cfg = None;
            worker.control.cancel();
        }
    }
}
impl Drop for PushSession {
    fn drop(&mut self) {
        self.invalidate();
    }
}

fn discover(cfg: &PushConfig, control: &Control, transport: &Transport) -> Option<Url> {
    if !control.active() {
        return None;
    }
    // Redirects must never forward credentials to a different origin. The socket
    // handshake below likewise performs no redirect following.
    let mut response = transport
        .http
        .get(&format!(
            "{}/ocs/v2.php/cloud/capabilities",
            cfg.server.trim_end_matches('/')
        ))
        .config()
        .max_redirects(0)
        .timeout_global(Some(CONNECT_BUDGET))
        .timeout_connect(Some(Duration::from_secs(3)))
        .build()
        .header("Authorization", cfg.auth())
        .header("Accept", "application/json")
        .header("OCS-APIREQUEST", "true")
        .call()
        .ok()?;
    if response.status() != 200 || !control.active() {
        return None;
    }
    let body = response.body_mut().with_config().limit(1 << 20).read_to_string().ok()?;
    control.active().then(|| endpoint(&cfg.server, &body)).flatten()
}
fn run_worker(cfg: PushConfig, control: Arc<Control>, transport: Transport) {
    let mut discovered: Option<(Instant, Option<Url>)> = None;
    let mut backoff = Duration::from_secs(5);
    while control.active() {
        if discovered
            .as_ref()
            .is_none_or(|(time, _)| time.elapsed() >= CAPABILITY_TTL)
        {
            discovered = Some((Instant::now(), discover(&cfg, &control, &transport)));
        }
        if let Some(url) = discovered.as_ref().and_then(|(_, url)| url.as_ref()) {
            let connected = listen(&cfg, url, &control, &transport);
            if let Ok(mut socket) = control.socket.lock() {
                *socket = None;
            }
            if connected {
                backoff = Duration::from_secs(5);
            }
            control.wait(backoff);
            backoff = (backoff * 2).min(Duration::from_secs(60));
        } else {
            control.wait(CAPABILITY_TTL);
        }
    }
    control.cancel();
}
/// Bound every underlying TLS/frame read, including fragmented messages. A
/// server trickling bytes cannot restart the handshake or native-activity budget.
struct PushStream<'a> {
    socket: TcpStream,
    control: &'a Control,
    deadline: &'a std::cell::Cell<Option<Instant>>,
}
impl PushStream<'_> {
    fn check(&self) -> std::io::Result<()> {
        if !self.control.active() {
            return Err(std::io::ErrorKind::ConnectionAborted.into());
        }
        if let Some(deadline) = self.deadline.get() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(std::io::ErrorKind::TimedOut.into());
            }
            self.socket.set_read_timeout(Some(remaining))?;
            self.socket.set_write_timeout(Some(remaining))?;
        }
        Ok(())
    }
}
impl Read for PushStream<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.check()?;
        self.socket.read(bytes)
    }
}
impl Write for PushStream<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.check()?;
        self.socket.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.check()?;
        self.socket.flush()
    }
}

fn listen(cfg: &PushConfig, endpoint: &Url, control: &Control, transport: &Transport) -> bool {
    let until = Instant::now() + CONNECT_BUDGET;
    let Some(host) = endpoint.host_str() else {
        return false;
    };
    let Some(port) = endpoint.port_or_known_default() else {
        return false;
    };
    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return false;
    };
    let mut stream = None;
    for address in addresses.take(4) {
        if !control.active() || Instant::now() >= until {
            return false;
        }
        if let Ok(socket) = TcpStream::connect_timeout(
            &address,
            Duration::from_secs(2).min(until.saturating_duration_since(Instant::now())),
        ) {
            stream = Some(socket);
            break;
        }
    }
    let Some(stream) = stream else {
        return false;
    };
    let remaining = until.saturating_duration_since(Instant::now());
    if remaining.is_zero() || !control.active() {
        return false;
    }
    let _ = stream.set_read_timeout(Some(remaining));
    let _ = stream.set_write_timeout(Some(remaining));
    if let Ok(mut stored) = control.socket.lock() {
        *stored = stream.try_clone().ok();
    }
    let deadline = std::cell::Cell::new(Some(until));
    let stream = PushStream {
        socket: stream,
        control,
        deadline: &deadline,
    };
    let config = WebSocketConfig::default()
        .read_buffer_size(4096)
        .write_buffer_size(0)
        .max_write_buffer_size(MAX_MESSAGE * 2)
        .max_message_size(Some(MAX_MESSAGE))
        .max_frame_size(Some(MAX_MESSAGE));
    let Ok((mut socket, _)) =
        tungstenite::client_tls_with_config(endpoint.as_str(), stream, Some(config), (transport.websocket)())
    else {
        return false;
    };
    if !control.active() {
        return false;
    }
    if socket.send(Message::Text(cfg.user.clone().into())).is_err()
        || socket.send(Message::Text(cfg.password.clone().into())).is_err()
    {
        return false;
    }
    // A clone controls timeouts on the same underlying socket and can interrupt
    // TLS/WebSocket reads immediately when provider or native lifecycle changes.
    if let Ok(stored) = control.socket.lock() {
        if let Some(stream) = stored.as_ref() {
            if stream.set_nonblocking(true).is_err() {
                return false;
            }
        }
    }
    let authentication_deadline = Instant::now() + CONNECT_BUDGET;
    deadline.set(Some(authentication_deadline));
    let mut authenticated = false;
    while control.active() {
        if !authenticated && Instant::now() >= authentication_deadline {
            break;
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                if text == "authenticated" {
                    authenticated = true;
                    deadline.set(None);
                    if let Ok(stored) = control.socket.lock() {
                        if let Some(stream) = stored.as_ref() {
                            let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
                        }
                    }
                } else if text.starts_with("err:") {
                    break;
                } else if authenticated && text == "notify_file" {
                    control.hint.store(true, Ordering::Release);
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(_)) => {
                if socket.flush().is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
    }
    authenticated
}

#[cfg(test)]
#[path = "push_tests.rs"]
mod tests;
