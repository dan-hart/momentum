// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The channel `mo` uses to hand actions to a running app where there is no session bus
//! (macOS): a Unix socket in the data directory. One JSON line per connection: an
//! `sp_oplog::Action`, or the strings `"sync"` / `"config"`. The app answers `ok` or `error: …`.
//!
//! On Linux the GTK app keeps `org.gtk.Actions.Activate("cli")` over D-Bus; the socket is
//! offered there too, so `mo` works the same on both when it finds the socket first.
use crate::engine::Engine;
use crate::types::CoreError;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const SOCKET_NAME: &str = "momentum.sock";

/// What the app does when `mo` sent something: rebuild views, or run a sync.
#[cfg_attr(feature = "ffi", uniffi::export(with_foreign))]
pub trait CliDelegate: Send + Sync {
    fn store_changed(&self);
    fn sync_requested(&self);
}

pub struct Server {
    stop: Arc<AtomicBool>,
    path: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn socket_path(dir: &Path) -> PathBuf {
    dir.join(SOCKET_NAME)
}

#[cfg(unix)]
fn handle(engine: &Engine, delegate: &dyn CliDelegate, line: &str) -> Result<(), String> {
    let line = line.trim();
    if line == "\"sync\"" || line == "sync" {
        delegate.sync_requested();
        return Ok(());
    }
    if line == "\"config\"" {
        delegate.store_changed();
        return Ok(());
    }
    engine.dispatch_json(line.to_string()).map_err(|e| e.to_string())?;
    delegate.store_changed();
    Ok(())
}

#[cfg_attr(feature = "ffi", uniffi::export)]
impl Engine {
    /// Listens for `mo` on the data directory's socket until the engine is dropped or
    /// `stop_cli_server` is called.
    pub fn serve_cli(self: Arc<Self>, delegate: Arc<dyn CliDelegate>) -> Result<(), CoreError> {
        #[cfg(not(unix))]
        {
            let _ = delegate;
            return Err(CoreError::Io {
                message: "no Unix sockets on this platform".into(),
            });
        }
        #[cfg(unix)]
        {
            use std::os::unix::net::UnixListener;
            let mut slot = self.ipc.lock().unwrap_or_else(|e| e.into_inner());
            if slot.is_some() {
                return Ok(());
            }
            let path = socket_path(&PathBuf::from(self.data_dir()));
            let _ = std::fs::remove_file(&path);
            let listener = UnixListener::bind(&path)?;
            listener.set_nonblocking(true)?;
            let stop = Arc::new(AtomicBool::new(false));
            let weak = Arc::downgrade(&self);
            let flag = stop.clone();
            std::thread::Builder::new()
                .name("momentum-cli".into())
                .spawn(move || {
                    while !flag.load(Ordering::SeqCst) {
                        match listener.accept() {
                            Ok((stream, _)) => {
                                let Some(engine) = weak.upgrade() else { break };
                                // BSD and macOS hand back an accepted socket that inherited
                                // the listener's O_NONBLOCK, so reading it would fail with
                                // WouldBlock whenever the client's line has not landed yet.
                                // Read this connection blocking, with a timeout so that one
                                // silent client cannot stall the accept loop.
                                let _ = stream.set_nonblocking(false);
                                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                                let mut reader = BufReader::new(&stream);
                                let mut line = String::new();
                                let reply = match reader.read_line(&mut line) {
                                    Ok(_) => match handle(&engine, delegate.as_ref(), &line) {
                                        Ok(()) => "ok\n".to_string(),
                                        Err(e) => format!("error: {e}\n"),
                                    },
                                    Err(e) => format!("error: {e}\n"),
                                };
                                let _ = (&stream).write_all(reply.as_bytes());
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                            Err(e) => {
                                log::warn!("cli socket: {e}");
                                break;
                            }
                        }
                    }
                })
                .ok();
            *slot = Some(Server { stop, path });
            Ok(())
        }
    }
    pub fn stop_cli_server(&self) {
        self.ipc.lock().unwrap_or_else(|e| e.into_inner()).take();
    }
}

/// Client side, for `mo`: hands one payload to a running app. `Ok(false)` when no app
/// listens (missing or stale socket). A connected app's timeout or rejection is an error;
/// the caller must not write behind that app's back.
#[cfg(unix)]
pub fn forward(dir: &Path, payload: &str) -> Result<bool, String> {
    use std::os::unix::net::UnixStream;
    let path = socket_path(dir);
    use std::os::unix::fs::FileTypeExt;
    match std::fs::metadata(&path) {
        Ok(meta) if meta.file_type().is_socket() => {}
        Ok(_) => return Err(format!("{} is not a Unix socket", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.to_string()),
    }
    let mut stream = match UnixStream::connect(&path) {
        Ok(s) => s,
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
            ) =>
        {
            return Ok(false); // stale socket file: the app is gone
        }
        Err(e) => return Err(e.to_string()),
    };
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .write_all(format!("{}\n", payload.trim()).as_bytes())
        .map_err(|e| e.to_string())?;
    let mut reply = String::new();
    BufReader::new(&stream)
        .read_line(&mut reply)
        .map_err(|e| e.to_string())?;
    if reply.trim().is_empty() {
        Err("running app disconnected without acknowledging the request".into())
    } else if reply.trim() == "ok" {
        Ok(true)
    } else {
        Err(reply.trim().trim_start_matches("error: ").to_string())
    }
}
#[cfg(not(unix))]
pub fn forward(_dir: &Path, _payload: &str) -> Result<bool, String> {
    Ok(false)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn inaccessible_socket_is_an_error_not_an_offline_fallback() {
        let dir = tempfile::tempdir().unwrap();
        // A non-socket path produces a connect error other than stale/refused.
        std::fs::create_dir(socket_path(dir.path())).unwrap();
        assert!(forward(dir.path(), "\"sync\"").is_err());
    }

    #[test]
    fn disconnected_peer_reports_a_clear_error() {
        use std::os::unix::net::UnixListener;
        let dir = tempfile::tempdir().unwrap();
        let listener = UnixListener::bind(socket_path(dir.path())).unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line).unwrap();
        });
        let result = forward(dir.path(), "\"sync\"");
        server.join().unwrap();
        assert!(result.is_err_and(|message| !message.is_empty()));
    }
}
