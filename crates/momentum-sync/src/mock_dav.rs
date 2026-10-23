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
    pub conflict_puts: Arc<AtomicUsize>,
    pub missing_collection: Arc<AtomicBool>,
    pub collections: Arc<AtomicUsize>,
    /// When non-zero, the next PUT stores only this many bytes and still answers 2xx,
    /// like a proxy that streams a cut-off body through to the server.
    pub keep_only_next_put: Arc<AtomicUsize>,
    /// Headers of the most recent PUT, lower-cased, for asserting the upload guards.
    pub last_put_headers: Arc<Mutex<HashMap<String, String>>>,
    pub copies: Arc<AtomicUsize>,
    pub not_modified: Arc<AtomicUsize>,
    pub downloaded_bytes: Arc<AtomicUsize>,
    pub fail_gets: Arc<AtomicUsize>,
    pub fail_propfinds: Arc<AtomicUsize>,
    pub hide_after_put: Arc<AtomicBool>,
    pub omit_etag: Arc<AtomicBool>,
    pub omit_put_etag: Arc<AtomicBool>,
    pub omit_verified_etag: Arc<AtomicBool>,
    pub different_verified_etag: Arc<AtomicBool>,
    pub encode_inventory_paths: Arc<AtomicBool>,
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
        let conflict_puts = Arc::new(AtomicUsize::new(0));
        let conflicts = conflict_puts.clone();
        let missing_collection = Arc::new(AtomicBool::new(false));
        let collections = Arc::new(AtomicUsize::new(0));
        let keep_only_next_put = Arc::new(AtomicUsize::new(0));
        let last_put_headers: Arc<Mutex<HashMap<String, String>>> = Default::default();
        let copies = Arc::new(AtomicUsize::new(0));
        let not_modified = Arc::new(AtomicUsize::new(0));
        let downloaded_bytes = Arc::new(AtomicUsize::new(0));
        let fail_gets = Arc::new(AtomicUsize::new(0));
        let fail_propfinds = Arc::new(AtomicUsize::new(0));
        let hide_after_put = Arc::new(AtomicBool::new(false));
        let omit_etag = Arc::new(AtomicBool::new(false));
        let omit_put_etag = Arc::new(AtomicBool::new(false));
        let omit_verified_etag = Arc::new(AtomicBool::new(false));
        let different_verified_etag = Arc::new(AtomicBool::new(false));
        let different_verified = different_verified_etag.clone();
        let encode_inventory_paths = Arc::new(AtomicBool::new(false));
        let encoded = encode_inventory_paths.clone();
        let (omit_put, omit_verified) = (omit_put_etag.clone(), omit_verified_etag.clone());
        let (nm, bytes, fg, fp, hide, omit) = (
            not_modified.clone(),
            downloaded_bytes.clone(),
            fail_gets.clone(),
            fail_propfinds.clone(),
            hide_after_put.clone(),
            omit_etag.clone(),
        );
        let (missing, created) = (missing_collection.clone(), collections.clone());
        let (keep_only, put_headers, copied) = (keep_only_next_put.clone(), last_put_headers.clone(), copies.clone());
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
                                if fg
                                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                                    .is_ok()
                                {
                                    (503, vec![], vec![])
                                } else {
                                    match files.get(&path) {
                                        Some(b) if headers.get("if-none-match") == Some(&etag(b)) => {
                                            nm.fetch_add(1, Ordering::SeqCst);
                                            (304, vec![], vec![])
                                        }
                                        Some(b) => {
                                            bytes.fetch_add(b.len(), Ordering::SeqCst);
                                            let extra = if omit.load(Ordering::SeqCst) {
                                                vec![]
                                            } else {
                                                vec![format!("OC-ETag: {}", etag(b))]
                                            };
                                            (200, extra, b.clone())
                                        }
                                        None => (404, vec![], vec![]),
                                    }
                                }
                            }
                            "HEAD" => match files.get(&path) {
                                Some(b) => (
                                    200,
                                    vec![format!("OC-ETag: {}", etag(b)), format!("X-Size: {}", b.len())],
                                    vec![],
                                ),
                                None => (404, vec![], vec![]),
                            },
                            "COPY" => {
                                copied.fetch_add(1, Ordering::SeqCst);
                                let destination = headers
                                    .get("destination")
                                    .and_then(|d| d.find("/remote.php").map(|i| d[i..].to_string()));
                                match (files.get(&path).cloned(), destination) {
                                    (Some(b), Some(dest)) => {
                                        files.insert(dest, b);
                                        (201, vec![], vec![])
                                    }
                                    (None, _) => (404, vec![], vec![]),
                                    (_, None) => (400, vec![], vec![]),
                                }
                            }
                            "MKCOL" => {
                                created.fetch_add(1, Ordering::SeqCst);
                                missing.store(false, Ordering::SeqCst);
                                (201, vec![], vec![])
                            }
                            "PROPFIND" => {
                                pf.fetch_add(1, Ordering::SeqCst);
                                let folder = "/remote.php/dav/files/u/legacy";
                                if fp
                                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                                    .is_ok()
                                {
                                    (503, vec![], vec![])
                                } else if headers.get("depth").map(String::as_str) == Some("1") {
                                    let prefix = format!("{}/", path.trim_end_matches('/'));
                                    let members: Vec<_> =
                                        files.keys().filter(|name| name.starts_with(&prefix)).collect();
                                    if members.is_empty() && path != folder {
                                        (404, vec![], vec![])
                                    } else {
                                        let href = |value: &str| {
                                            if encoded.load(Ordering::SeqCst) {
                                                value
                                                    .replace("/legacy", "/%6Cegacy")
                                                    .replace("sync-data", "sync%2Ddata")
                                            } else {
                                                value.to_string()
                                            }
                                        };
                                        let root = href(&path);
                                        let mut xml = format!("<d:multistatus xmlns:d=\"DAV:\"><d:response><d:href>{root}/</d:href></d:response>");
                                        for name in members {
                                            let name = href(name);
                                            xml.push_str(&format!("<d:response><d:href>{name}</d:href></d:response>"));
                                        }
                                        xml.push_str("</d:multistatus>");
                                        (207, vec![], xml.into_bytes())
                                    }
                                } else if path == folder && missing.load(Ordering::SeqCst) {
                                    (404, vec![], vec![])
                                } else if path == folder || path == "/remote.php/dav/files/u/" {
                                    (207, vec![], vec![])
                                } else if let Some(b) = files.get(&path) {
                                    let xml = format!(
                                        "<?xml version=\"1.0\"?><d:multistatus xmlns:d=\"DAV:\"><d:response><d:href>{path}</d:href><d:propstat><d:prop><d:getcontentlength>{}</d:getcontentlength><d:getetag>{}</d:getetag></d:prop></d:propstat></d:response></d:multistatus>",
                                        b.len(),
                                        if omit_verified.load(Ordering::SeqCst) { String::new() }
                                        else if different_verified.load(Ordering::SeqCst) { "&quot;another-file&quot;".into() }
                                        else { etag(b).replace('"', "&quot;") }
                                    );
                                    (207, vec![], xml.into_bytes())
                                } else {
                                    (404, vec![], vec![])
                                }
                            }
                            "PUT" => {
                                p.fetch_add(1, Ordering::SeqCst);
                                *put_headers.lock().unwrap() = headers.clone();
                                let keep = keep_only.swap(0, Ordering::SeqCst);
                                if keep > 0 {
                                    body.truncate(keep);
                                }
                                let cur = files.get(&path).cloned();
                                let if_match = headers.get("if-match").cloned();
                                let none_match = headers.get("if-none-match").cloned();
                                if missing.load(Ordering::SeqCst) {
                                    (409, vec![], vec![])
                                } else if fail.swap(false, Ordering::SeqCst)
                                    || conflicts
                                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                                        .is_ok()
                                {
                                    (412, vec![], vec![])
                                } else if if_match.is_some() && cur.as_deref().map(etag) != if_match {
                                    (412, vec![], vec![])
                                } else if none_match.as_deref() == Some("*") && cur.is_some() {
                                    (412, vec![], vec![])
                                } else {
                                    let created = cur.is_none();
                                    let tag = etag(&body);
                                    files.insert(path.clone(), body);
                                    if hide.swap(false, Ordering::SeqCst) {
                                        files.remove(&path);
                                    }
                                    let extra = if omit_put.load(Ordering::SeqCst) {
                                        vec![]
                                    } else {
                                        vec![format!("OC-ETag: {tag}")]
                                    };
                                    (if created { 201 } else { 204 }, extra, vec![])
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
                let mut resp = format!("HTTP/1.1 {status} {reason}\r\nConnection: close\r\n");
                let mut declared = None;
                for h in extra {
                    if let Some(size) = h.strip_prefix("X-Size: ") {
                        declared = Some(size.to_string());
                        continue;
                    }
                    resp.push_str(&h);
                    resp.push_str("\r\n");
                }
                // A HEAD answer declares the stored size; every other answer its own body.
                resp.push_str(&format!(
                    "Content-Length: {}\r\n",
                    declared.unwrap_or_else(|| payload.len().to_string())
                ));
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
            conflict_puts,
            missing_collection,
            collections,
            keep_only_next_put,
            last_put_headers,
            copies,
            not_modified,
            downloaded_bytes,
            fail_gets,
            fail_propfinds,
            hide_after_put,
            omit_etag,
            omit_put_etag,
            omit_verified_etag,
            different_verified_etag,
            encode_inventory_paths,
        }
    }
    pub fn file_count(&self) -> usize {
        self.files.lock().unwrap().len()
    }
    pub fn raw(&self, path: &str) -> Option<Vec<u8>> {
        self.files.lock().unwrap().get(path).cloned()
    }
    pub fn remove(&self, path: &str) {
        self.files.lock().unwrap().remove(path);
    }
    /// Overwrite a stored file directly, e.g. to simulate a copy damaged on the server.
    pub fn set_raw(&self, path: &str, body: Vec<u8>) {
        self.files.lock().unwrap().insert(path.to_string(), body);
    }
}
