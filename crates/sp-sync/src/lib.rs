// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Nextcloud (WebDAV) sync in upstream's file-based v2 format (`sync-data.json`).
//! Strategy: rebase pending local ops onto the newest remote snapshot, then upload
//! snapshot + ops with an ETag compare-and-swap. Remote ops are never replayed
//! here because the remote `state` already contains their effect.
pub mod crypto;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sp_model::{now_ms, AppData, SCHEMA_VERSION};
use sp_oplog::{apply, merge_clocks, Op, VectorClock};
use sp_store::Store;
use std::io::{Read, Write};

pub const SYNC_FILE: &str = "sync-data.json";
const MAX_RECENT_OPS: usize = 2000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFile {
    pub version: u32,
    pub sync_version: u64,
    pub schema_version: u32,
    pub vector_clock: VectorClock,
    pub last_modified: u64,
    pub client_id: String,
    pub state: AppData,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_young: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_old: Option<Value>,
    #[serde(default)]
    pub recent_ops: Vec<Op>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oldest_op_sync_version: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("HTTP error: {0}")]
    Http(#[from] ureq::Error),
    #[error("remote file is encrypted; set the encryption password in Preferences")]
    Encrypted,
    #[error("could not decrypt the sync file: {0}")]
    Decrypt(String),
    #[error("remote file uses format v{0}, expected v2 (turn off \"Surgical sync\" upstream)")]
    Version(u64),
    #[error("schema version {0} is newer than this app supports ({SCHEMA_VERSION})")]
    Schema(u32),
    #[error("bad sync file: {0}")]
    Parse(String),
    #[error("remote changed during upload; retry")]
    Conflict,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// Parse `pf_[C][E]<ver>__<body>`; body is JSON, or base64 gzip when `C`.
pub fn decode(body: &str, password: Option<&str>) -> Result<SyncFile, SyncError> {
    let rest = body
        .strip_prefix("pf_")
        .ok_or_else(|| SyncError::Parse("missing pf_ prefix".into()))?;
    let (flags, json) = rest
        .split_once("__")
        .ok_or_else(|| SyncError::Parse("missing separator".into()))?;
    let decrypted;
    let json = if flags.contains('E') {
        let pw = password.filter(|p| !p.is_empty()).ok_or(SyncError::Encrypted)?;
        decrypted = crypto::decrypt(json, pw).map_err(SyncError::Decrypt)?;
        decrypted.as_str()
    } else {
        json
    };
    let text = if flags.contains('C') {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(json.trim())
            .map_err(|e| SyncError::Parse(e.to_string()))?;
        let mut s = String::new();
        flate2::read::GzDecoder::new(&bytes[..]).read_to_string(&mut s)?;
        s
    } else {
        json.to_string()
    };
    let v: Value = serde_json::from_str(&text).map_err(|e| SyncError::Parse(e.to_string()))?;
    match v.get("version").and_then(Value::as_u64) {
        Some(2) => {}
        Some(n) => return Err(SyncError::Version(n)),
        None => return Err(SyncError::Parse("no version".into())),
    }
    let f: SyncFile = serde_json::from_value(v).map_err(|e| SyncError::Parse(e.to_string()))?;
    if f.schema_version > SCHEMA_VERSION {
        return Err(SyncError::Schema(f.schema_version));
    }
    Ok(f)
}
pub fn encode(f: &SyncFile, compress: bool, password: Option<&str>) -> Result<String, SyncError> {
    let mut body = serde_json::to_string(f).unwrap();
    if compress {
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(body.as_bytes()).unwrap();
        body = base64::engine::general_purpose::STANDARD.encode(gz.finish().unwrap());
    }
    let key = password.filter(|p| !p.is_empty());
    if let Some(pw) = key {
        body = crypto::encrypt(&body, pw).map_err(SyncError::Decrypt)?;
    }
    Ok(format!(
        "pf_{}{}2__{body}",
        if compress { "C" } else { "" },
        if key.is_some() { "E" } else { "" }
    ))
}

#[derive(Debug, Clone, Default)]
pub struct NextcloudCfg {
    pub server_url: String,
    pub user_name: String,
    pub password: String,
    pub folder: String,
    pub compress: bool,
    pub encrypt_key: Option<String>,
}
impl NextcloudCfg {
    fn folder_url(&self) -> String {
        let folder = self.folder.trim_matches('/');
        format!(
            "{}/remote.php/dav/files/{}/{}",
            self.server_url.trim_end_matches('/'),
            urlencode(self.user_name.trim()),
            folder
        )
    }
    fn file_url(&self) -> String {
        format!("{}/{}", self.folder_url(), SYNC_FILE)
    }
    fn auth(&self) -> String {
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", self.user_name.trim(), self.password))
        )
    }
    pub fn is_complete(&self) -> bool {
        !self.server_url.trim().is_empty()
            && !self.user_name.trim().is_empty()
            && !self.password.is_empty()
            && !self.folder.trim().is_empty()
    }
}
fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

fn etag(r: &ureq::http::Response<ureq::Body>) -> Option<String> {
    ["oc-etag", "etag"]
        .iter()
        .find_map(|h| r.headers().get(*h)?.to_str().ok().map(|s| s.trim().to_string()))
}

fn download(cfg: &NextcloudCfg) -> Result<Option<(SyncFile, String)>, SyncError> {
    match ureq::get(&cfg.file_url()).header("Authorization", &cfg.auth()).call() {
        Ok(mut r) => {
            let tag = etag(&r).unwrap_or_default();
            let body = r.body_mut().with_config().limit(512 << 20).read_to_string()?;
            Ok(Some((decode(&body, cfg.encrypt_key.as_deref())?, tag)))
        }
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn upload(cfg: &NextcloudCfg, body: &str, expected: Option<&str>) -> Result<String, SyncError> {
    let mut req = ureq::put(&cfg.file_url())
        .header("Authorization", &cfg.auth())
        .header("Content-Type", "application/octet-stream");
    req = match expected {
        Some(tag) if tag.starts_with('"') => req.header("If-Match", tag),
        Some(_) => req,
        None => req.header("If-None-Match", "*"),
    };
    match req.send(body) {
        Ok(r) => Ok(etag(&r).unwrap_or_default()),
        Err(ureq::Error::StatusCode(412)) => Err(SyncError::Conflict),
        Err(ureq::Error::StatusCode(404 | 409)) if expected.is_none() => {
            ureq::run(
                ureq::http::Request::builder()
                    .method("MKCOL")
                    .uri(cfg.folder_url())
                    .header("Authorization", cfg.auth())
                    .body(())
                    .unwrap(),
            )?;
            upload(cfg, body, None)
        }
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub downloaded: bool,
    pub uploaded: bool,
    pub ops_uploaded: usize,
}

/// One sync cycle. Mutates `store` (state, pending, meta) on success.
pub fn sync(cfg: &NextcloudCfg, store: &mut Store) -> Result<Report, SyncError> {
    let mut report = Report::default();
    for _attempt in 0..3 {
        let remote = download(cfg)?;
        let (mut file, tag) = match remote {
            Some((f, tag)) => {
                if f.sync_version != store.meta.last_sync_version {
                    let mut state = f.state.clone();
                    for p in &store.pending {
                        apply(&mut state, &p.action);
                    }
                    store.state = state;
                    store.meta.vector_clock = merge_clocks(&store.meta.vector_clock, &f.vector_clock);
                    report.downloaded = true;
                }
                (f, Some(tag))
            }
            None => (
                SyncFile {
                    version: 2,
                    sync_version: 0,
                    schema_version: SCHEMA_VERSION,
                    vector_clock: VectorClock::new(),
                    last_modified: 0,
                    client_id: store.meta.client_id.clone(),
                    state: store.state.clone(),
                    archive_young: None,
                    archive_old: None,
                    recent_ops: vec![],
                    oldest_op_sync_version: None,
                },
                None,
            ),
        };
        if store.pending.is_empty() && tag.is_some() {
            store.meta.last_sync_version = file.sync_version;
            store.meta.last_etag = tag;
            store.save()?;
            return Ok(report);
        }
        let sv = file.sync_version + 1;
        file.recent_ops.extend(store.pending.iter().map(|p| Op {
            sv: Some(sv),
            ..p.op.clone()
        }));
        let cut = file.recent_ops.len().saturating_sub(MAX_RECENT_OPS);
        file.recent_ops.drain(..cut);
        file.oldest_op_sync_version = file.recent_ops.first().and_then(|o| o.sv);
        file.sync_version = sv;
        file.schema_version = SCHEMA_VERSION;
        file.vector_clock = store.meta.vector_clock.clone();
        file.last_modified = now_ms();
        file.client_id = store.meta.client_id.clone();
        file.state = store.state.clone();
        match upload(
            cfg,
            &encode(&file, cfg.compress, cfg.encrypt_key.as_deref())?,
            tag.as_deref(),
        ) {
            Ok(new_tag) => {
                report.uploaded = true;
                report.ops_uploaded = store.pending.len();
                store.pending.clear();
                store.meta.last_sync_version = sv;
                store.meta.last_etag = Some(new_tag);
                store.save()?;
                return Ok(report);
            }
            Err(SyncError::Conflict) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(SyncError::Conflict)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_prefix_and_gzip() {
        let f = SyncFile {
            version: 2,
            sync_version: 3,
            schema_version: 4,
            vector_clock: VectorClock::new(),
            last_modified: 1,
            client_id: "c".into(),
            state: AppData::fresh(),
            archive_young: None,
            archive_old: None,
            recent_ops: vec![],
            oldest_op_sync_version: None,
        };
        for c in [false, true] {
            let s = encode(&f, c, None).unwrap();
            assert!(s.starts_with(if c { "pf_C2__" } else { "pf_2__" }));
            assert_eq!(decode(&s, None).unwrap().sync_version, 3);
        }
        assert!(matches!(decode("pf_E2__x", None), Err(SyncError::Encrypted)));
        let enc = encode(&f, true, Some("secret")).unwrap();
        assert!(enc.starts_with("pf_CE2__"));
        assert_eq!(decode(&enc, Some("secret")).unwrap().sync_version, 3);
        assert!(matches!(decode(&enc, Some("wrong")), Err(SyncError::Decrypt(_))));
    }
}

/// Two clients converging through a WebDAV server. Run with
/// `MOMENTUM_DAV=http://127.0.0.1:8765 cargo test -p sp-sync -- --ignored`.
#[cfg(test)]
mod dav_tests {
    use super::*;
    use sp_model::{Task, INBOX_PROJECT_ID};
    use sp_oplog::Action;
    #[test]
    #[ignore]
    fn two_clients_converge() {
        let cfg = NextcloudCfg {
            server_url: std::env::var("MOMENTUM_DAV").unwrap(),
            user_name: "u".into(),
            password: "p".into(),
            folder: "sp".into(),
            compress: true,
            encrypt_key: Some("k".into()),
        };
        let tmp = std::env::temp_dir().join(format!("momentum-dav-{}", now_ms()));
        let (mut a, mut b) = (Store::load(tmp.join("a")), Store::load(tmp.join("b")));
        a.dispatch(Action::AddTask {
            task: Task::new("from A", INBOX_PROJECT_ID),
            bottom: true,
        });
        let r = sync(&cfg, &mut a).unwrap();
        assert!(r.uploaded && r.ops_uploaded == 1);
        let r = sync(&cfg, &mut b).unwrap();
        assert!(r.downloaded && !r.uploaded);
        assert_eq!(
            b.state.task.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(),
            ["from A"]
        );
        b.dispatch(Action::AddTask {
            task: Task::new("from B", INBOX_PROJECT_ID),
            bottom: true,
        });
        a.dispatch(Action::AddTask {
            task: Task::new("from A2", INBOX_PROJECT_ID),
            bottom: true,
        });
        sync(&cfg, &mut b).unwrap();
        sync(&cfg, &mut a).unwrap(); // rebases A2 onto B's upload
        sync(&cfg, &mut b).unwrap();
        let titles = |s: &Store| {
            let mut v: Vec<String> = s.state.task.iter().map(|t| t.title.clone()).collect();
            v.sort();
            v
        };
        assert_eq!(titles(&a), ["from A", "from A2", "from B"]);
        assert_eq!(titles(&a), titles(&b));
        assert_eq!(a.meta.last_sync_version, 3);
        assert!(a.pending.is_empty() && b.pending.is_empty());
        let (f, _) = download(&cfg).unwrap().unwrap();
        assert_eq!(f.recent_ops.len(), 3);
        assert_eq!(f.recent_ops[0].a, "HA");
        assert_eq!(f.recent_ops[2].sv, Some(3));
        assert_eq!(f.vector_clock.len(), 2);
    }
}
