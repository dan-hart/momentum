// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! An in-process WebDAV stand-in for tests: GET/PUT with ETags, `If-Match`,
//! `If-None-Match: *`, MKCOL, and a switch to fail the next PUT with 412 so the
//! conflict retry can be exercised. No Python, no ports to coordinate.
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub struct MockDav {
    pub url: String,
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    pub puts: Arc<AtomicUsize>,
    pub gets: Arc<AtomicUsize>,
    pub propfinds: Arc<AtomicUsize>,
    pub reject_authentication: Arc<AtomicBool>,
    pub fail_next_put: Arc<AtomicBool>,
    pub missing_collection: Arc<AtomicBool>,
    pub collections: Arc<AtomicUsize>,
}

fn etag(body: &[u8]) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    body.hash(&mut h);
    format!("\"{:016x}\"", h.finish())
}

impl MockDav {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let files: Arc<Mutex<HashMap<String, Vec<u8>>>> = Default::default();
        let puts = Arc::new(AtomicUsize::new(0));
        let gets = Arc::new(AtomicUsize::new(0));
        let propfinds = Arc::new(AtomicUsize::new(0));
        let reject_authentication = Arc::new(AtomicBool::new(false));
        let fail_next_put = Arc::new(AtomicBool::new(false));
        let missing_collection = Arc::new(AtomicBool::new(false));
        let collections = Arc::new(AtomicUsize::new(0));
        let (missing, created) = (missing_collection.clone(), collections.clone());
        let (f, p, g, pf, reject, fail) = (
            files.clone(),
            puts.clone(),
            gets.clone(),
            propfinds.clone(),
            reject_authentication.clone(),
            fail_next_put.clone(),
        );
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let head_end = loop {
                    let n = match stream.read(&mut chunk) {
                        Ok(0) | Err(_) => break None,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break Some(i + 4);
                    }
                };
                let Some(head_end) = head_end else { continue };
                let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
                let mut lines = head.lines();
                let req = lines.next().unwrap_or_default().to_string();
                let mut headers = HashMap::new();
                for l in lines {
                    if let Some((k, v)) = l.split_once(':') {
                        headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
                    }
                }
                let len: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
                let mut body = buf[head_end..].to_vec();
                while body.len() < len {
                    let n = match stream.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    body.extend_from_slice(&chunk[..n]);
                }
                let mut parts = req.split_whitespace();
                let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or("/").to_string());
                let (status, extra, payload): (u16, Vec<String>, Vec<u8>) = {
                    let mut files = f.lock().unwrap();
                    if reject.load(Ordering::SeqCst)
                        || headers.get("authorization").map(String::as_str) != Some("Basic dTpw")
                    {
                        (401, vec![], vec![])
                    } else {
                        match method {
                            "GET" => {
                                g.fetch_add(1, Ordering::SeqCst);
                                match files.get(&path) {
                                    Some(b) => (200, vec![format!("OC-ETag: {}", etag(b))], b.clone()),
                                    None => (404, vec![], vec![]),
                                }
                            }
                            "MKCOL" => {
                                created.fetch_add(1, Ordering::SeqCst);
                                missing.store(false, Ordering::SeqCst);
                                (201, vec![], vec![])
                            }
                            "PROPFIND" => {
                                pf.fetch_add(1, Ordering::SeqCst);
                                let folder = "/remote.php/dav/files/u/sp";
                                if path == folder && missing.load(Ordering::SeqCst) {
                                    (404, vec![], vec![])
                                } else if path == folder || path == "/remote.php/dav/files/u/" {
                                    (207, vec![], vec![])
                                } else {
                                    (404, vec![], vec![])
                                }
                            }
                            "PUT" => {
                                p.fetch_add(1, Ordering::SeqCst);
                                let cur = files.get(&path).cloned();
                                let if_match = headers.get("if-match").cloned();
                                let none_match = headers.get("if-none-match").cloned();
                                if missing.load(Ordering::SeqCst) {
                                    (409, vec![], vec![])
                                } else if fail.swap(false, Ordering::SeqCst) {
                                    (412, vec![], vec![])
                                } else if if_match.is_some() && cur.as_deref().map(etag) != if_match {
                                    (412, vec![], vec![])
                                } else if none_match.as_deref() == Some("*") && cur.is_some() {
                                    (412, vec![], vec![])
                                } else {
                                    let created = cur.is_none();
                                    let tag = etag(&body);
                                    files.insert(path.clone(), body);
                                    (if created { 201 } else { 204 }, vec![format!("OC-ETag: {tag}")], vec![])
                                }
                            }
                            _ => (405, vec![], vec![]),
                        }
                    }
                };
                let reason = match status {
                    200 => "OK",
                    201 => "Created",
                    204 => "No Content",
                    207 => "Multi-Status",
                    401 => "Unauthorized",
                    404 => "Not Found",
                    412 => "Precondition Failed",
                    _ => "Error",
                };
                let mut resp = format!(
                    "HTTP/1.1 {status} {reason}\r\nConnection: close\r\nContent-Length: {}\r\n",
                    payload.len()
                );
                for h in extra {
                    resp.push_str(&h);
                    resp.push_str("\r\n");
                }
                resp.push_str("\r\n");
                let _ = stream.write_all(resp.as_bytes());
                let _ = stream.write_all(&payload);
                let _ = stream.flush();
            }
        });
        Self {
            url,
            files,
            puts,
            gets,
            propfinds,
            reject_authentication,
            fail_next_put,
            missing_collection,
            collections,
        }
    }
    pub fn file_count(&self) -> usize {
        self.files.lock().unwrap().len()
    }
    pub fn raw(&self, path: &str) -> Option<Vec<u8>> {
        self.files.lock().unwrap().get(path).cloned()
    }
}
