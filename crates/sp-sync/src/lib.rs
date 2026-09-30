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
use std::time::{Duration, Instant};

/// The whole-exchange budget for a foreground sync. A large sync file over a slow or
/// relayed connection needs minutes, not seconds; stalls are bounded separately by
/// the per-phase timeouts in `ExchangeGuard::agent`.
pub const DEFAULT_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(300);
/// A budget a system background wake can honour before the OS ends the process.
pub const BACKGROUND_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(25);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(45);

pub const SYNC_FILE: &str = "sync-data.json";
/// Where `replace_remote` parks an unreadable server copy before overwriting it.
pub const DAMAGED_FILE: &str = "sync-data.json.damaged";
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
    #[error("the sync file on the server could not be read: {0}")]
    Decrypt(String),
    #[error("wrong encryption password for the sync file on the server")]
    WrongEncryptionPassword,
    #[error("remote file uses format v{0}, expected v2 (turn off \"Surgical sync\" upstream)")]
    Version(u64),
    #[error("schema version {0} is newer than this app supports ({SCHEMA_VERSION})")]
    Schema(u32),
    #[error("bad sync file: {0}")]
    Parse(String),
    #[error("no sync file on the server yet and this device has no full data set; import a backup or sync another device first")]
    FreshState,
    #[error("remote changed during upload; retry")]
    Conflict,
    #[error("sync was cancelled because its source changed")]
    Cancelled,
    #[error("sync exceeded its deadline; retry when the connection is available")]
    Deadline,
    #[error("the server kept {kept} of {sent} bytes of the upload; the previous copy is unchanged")]
    UploadIncomplete { sent: u64, kept: u64 },
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// What went wrong, for interfaces that explain a failure in their own words.
/// Every variant maps to one plain-language headline and one remedy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncFailureKind {
    /// The server could not be reached: no network, DNS, connection refused, TLS.
    Unreachable,
    /// The server answered 401 or 403.
    Unauthorized,
    /// The server answered with another error status.
    ServerError,
    /// The whole-exchange budget ran out.
    TimedOut,
    /// The file is encrypted and no password is set locally.
    EncryptionPasswordMissing,
    /// The file could not be decrypted with the local password.
    EncryptionPasswordWrong,
    /// The server copy is truncated, corrupt or not a sync file.
    RemoteFileDamaged,
    /// The server copy uses a format or schema this app does not support.
    Incompatible,
    /// No server copy exists and this device has no full data set to publish.
    NothingToStartFrom,
    /// Another device changed the file during every upload attempt.
    Conflict,
    /// The exchange was cancelled or superseded locally.
    Cancelled,
    /// The server accepted fewer bytes than were sent; the previous copy remains.
    UploadIncomplete,
    /// Anything else: a local I/O failure while persisting, for example.
    Other,
}

impl SyncError {
    pub fn kind(&self) -> SyncFailureKind {
        use SyncFailureKind as K;
        match self {
            SyncError::Http(e) => match e {
                ureq::Error::StatusCode(401 | 403) => K::Unauthorized,
                ureq::Error::StatusCode(_) => K::ServerError,
                ureq::Error::Timeout(_) => K::TimedOut,
                ureq::Error::HostNotFound
                | ureq::Error::ConnectionFailed
                | ureq::Error::Io(_)
                | ureq::Error::Tls(_)
                | ureq::Error::Rustls(_)
                | ureq::Error::BodyStalled => K::Unreachable,
                _ => K::ServerError,
            },
            SyncError::Encrypted => K::EncryptionPasswordMissing,
            SyncError::WrongEncryptionPassword => K::EncryptionPasswordWrong,
            SyncError::Decrypt(_) | SyncError::Parse(_) => K::RemoteFileDamaged,
            SyncError::Version(_) | SyncError::Schema(_) => K::Incompatible,
            SyncError::FreshState => K::NothingToStartFrom,
            SyncError::Conflict => K::Conflict,
            SyncError::Cancelled => K::Cancelled,
            SyncError::Deadline => K::TimedOut,
            SyncError::UploadIncomplete { .. } => K::UploadIncomplete,
            SyncError::Io(_) => K::Other,
        }
    }
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
        decrypted = crypto::decrypt(json, pw).map_err(|e| match e {
            crypto::DecryptError::WrongPassword => SyncError::WrongEncryptionPassword,
            crypto::DecryptError::Damaged(detail) => SyncError::Decrypt(detail),
        })?;
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
    fn account_url(&self) -> String {
        format!(
            "{}/remote.php/dav/files/{}/",
            self.server_url.trim_end_matches('/'),
            urlencode(self.user_name.trim())
        )
    }
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

/// Verify that a Nextcloud WebDAV account and configured collection are reachable
/// without reading the sync file or changing remote/local data. A missing collection
/// is valid because the first real sync creates it after authenticating the account root.
pub fn probe_guarded(cfg: &NextcloudCfg, timeout: Duration, is_current: impl Fn() -> bool) -> Result<(), SyncError> {
    let guard = ExchangeGuard {
        current: &is_current,
        deadline: Instant::now() + timeout,
    };
    let propfind = |url: &str| {
        guard.check()?;
        let result = guard.agent()?.run(
            ureq::http::Request::builder()
                .method("PROPFIND")
                .uri(url)
                .header("Authorization", cfg.auth())
                .header("Depth", "0")
                .body(())
                .unwrap(),
        );
        guard.check()?;
        result.map(|_| ()).map_err(SyncError::from)
    };
    match propfind(&cfg.folder_url()) {
        Ok(()) => Ok(()),
        Err(SyncError::Http(ureq::Error::StatusCode(404))) => propfind(&cfg.account_url()),
        Err(error) => Err(error),
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

struct ExchangeGuard<'a> {
    current: &'a dyn Fn() -> bool,
    deadline: Instant,
}
impl ExchangeGuard<'_> {
    fn check(&self) -> Result<(), SyncError> {
        if !(self.current)() {
            return Err(SyncError::Cancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(SyncError::Deadline);
        }
        Ok(())
    }
    fn agent(&self) -> Result<ureq::Agent, SyncError> {
        self.check()?;
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        Ok(ureq::Agent::config_builder()
            // WebDAV collection creation uses MKCOL, an HTTP extension method.
            .allow_non_standard_methods(true)
            // The whole exchange stays within its budget, and a dead connection still
            // fails fast: connecting and the first response byte have their own limits,
            // while a body transfer may use everything that is left.
            .timeout_global(Some(remaining))
            .timeout_connect(Some(CONNECT_TIMEOUT.min(remaining)))
            .timeout_recv_response(Some(RESPONSE_TIMEOUT.min(remaining)))
            .build()
            .into())
    }
}

#[cfg(test)]
fn download(cfg: &NextcloudCfg) -> Result<Option<(SyncFile, String)>, SyncError> {
    download_guarded(
        cfg,
        &ExchangeGuard {
            current: &|| true,
            deadline: Instant::now() + DEFAULT_EXCHANGE_TIMEOUT,
        },
    )
}
fn download_guarded(cfg: &NextcloudCfg, guard: &ExchangeGuard<'_>) -> Result<Option<(SyncFile, String)>, SyncError> {
    let result = guard
        .agent()?
        .get(&cfg.file_url())
        .header("Authorization", &cfg.auth())
        .call();
    guard.check()?;
    match result {
        Ok(mut r) => {
            let tag = etag(&r).unwrap_or_default();
            let body = r.body_mut().with_config().limit(512 << 20).read_to_string()?;
            guard.check()?;
            let decoded = decode(&body, cfg.encrypt_key.as_deref())?;
            guard.check()?;
            Ok(Some((decoded, tag)))
        }
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
fn sha1_hex(body: &[u8]) -> String {
    use sha1::Digest;
    let digest = sha1::Sha1::digest(body);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// The server's stored size and ETag for the sync file, or `None` when it is absent.
/// Read with PROPFIND: a HEAD answer may omit Content-Length behind a proxy, and a
/// missing size must never pass for a matching one.
fn remote_size(cfg: &NextcloudCfg, guard: &ExchangeGuard<'_>) -> Result<Option<(u64, String)>, SyncError> {
    let result = guard.agent()?.run(
        ureq::http::Request::builder()
            .method("PROPFIND")
            .uri(cfg.file_url())
            .header("Authorization", cfg.auth())
            .header("Depth", "0")
            .body(())
            .unwrap(),
    );
    guard.check()?;
    match result {
        Ok(mut r) => {
            let xml = r.body_mut().with_config().limit(1 << 20).read_to_string()?;
            let size = xml_text(&xml, "getcontentlength")
                .and_then(|v| v.trim().parse().ok())
                .ok_or_else(|| SyncError::Parse("PROPFIND answer without getcontentlength".into()))?;
            let tag = xml_text(&xml, "getetag")
                .map(|t| t.trim().to_string())
                .unwrap_or_default();
            Ok(Some((size, tag)))
        }
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// The text of the first `<…:name>` element, namespace prefix ignored, entities decoded.
fn xml_text<'a>(xml: &'a str, name: &str) -> Option<String> {
    let open = format!(":{name}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find("</")? + start;
    Some(xml[start..end].replace("&quot;", "\"").replace("&amp;", "&"))
}

/// Upload with three guards against a cut-off transfer being accepted as the file:
/// Nextcloud verifies `OC-Checksum` and `X-Expected-Entity-Length` itself, and the
/// stored size is read back afterwards. A partial copy is reported, never trusted.
fn upload(
    cfg: &NextcloudCfg,
    body: &str,
    expected: Option<&str>,
    guard: &ExchangeGuard<'_>,
) -> Result<String, SyncError> {
    let sent = body.len() as u64;
    let checksum = format!("SHA1:{}", sha1_hex(body.as_bytes()));
    // At most one collection creation attempt; a server that keeps returning 404
    // must not cause unbounded recursion or continue after cancellation.
    for attempt in 0..2 {
        let mut req = guard
            .agent()?
            .put(&cfg.file_url())
            .header("Authorization", &cfg.auth())
            .header("Content-Type", "application/octet-stream")
            .header("OC-Checksum", &checksum)
            .header("X-Expected-Entity-Length", sent.to_string());
        req = match expected {
            Some(tag) if tag.starts_with('"') => req.header("If-Match", tag),
            Some(_) => req,
            None => req.header("If-None-Match", "*"),
        };
        let result = req.send(body);
        guard.check()?;
        match result {
            Ok(r) => {
                let tag = etag(&r).unwrap_or_default();
                if let Some((kept, _)) = remote_size(cfg, guard)? {
                    if kept != sent {
                        return Err(SyncError::UploadIncomplete { sent, kept });
                    }
                }
                return Ok(tag);
            }
            Err(ureq::Error::StatusCode(412)) => return Err(SyncError::Conflict),
            Err(ureq::Error::StatusCode(404 | 409)) if expected.is_none() && attempt == 0 => {
                guard.agent()?.run(
                    ureq::http::Request::builder()
                        .method("MKCOL")
                        .uri(cfg.folder_url())
                        .header("Authorization", cfg.auth())
                        .body(())
                        .unwrap(),
                )?;
                guard.check()?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!("the second PUT always returns")
}

/// Publish this device's full data set as the server copy, keeping the current
/// server file as `sync-data.json.damaged` first. This is the recovery for a server
/// copy nobody can read any more; it never merges, so the caller must have chosen
/// the device whose data should win. Other devices see a new sync version, download
/// it and re-upload their own pending edits on their next exchange.
pub fn replace_remote(
    cfg: &NextcloudCfg,
    store: &mut Store,
    timeout: Duration,
    is_current: impl Fn() -> bool,
) -> Result<Report, SyncError> {
    let guard = ExchangeGuard {
        current: &is_current,
        deadline: Instant::now() + timeout,
    };
    guard.check()?;
    if store.state.rest.get("globalConfig").is_none() {
        return Err(SyncError::FreshState);
    }
    let current = remote_size(cfg, &guard)?;
    if current.is_some() {
        let result = guard.agent()?.run(
            ureq::http::Request::builder()
                .method("COPY")
                .uri(cfg.file_url())
                .header("Authorization", cfg.auth())
                .header("Destination", format!("{}/{}", cfg.folder_url(), DAMAGED_FILE))
                .header("Overwrite", "T")
                .body(())
                .unwrap(),
        );
        guard.check()?;
        result?;
    }
    let sv = store.meta.last_sync_version + 1;
    let mut file = SyncFile {
        version: 2,
        sync_version: sv,
        schema_version: SCHEMA_VERSION,
        vector_clock: store.meta.vector_clock.clone(),
        last_modified: now_ms(),
        client_id: store.meta.client_id.clone(),
        state: store.state.clone(),
        archive_young: None,
        archive_old: None,
        recent_ops: vec![],
        oldest_op_sync_version: None,
    };
    file.archive_young = file.state.rest.remove("archiveYoung");
    file.archive_old = file.state.rest.remove("archiveOld");
    let expected = current.as_ref().map(|(_, tag)| tag.as_str());
    let new_tag = upload(
        cfg,
        &encode(&file, cfg.compress, cfg.encrypt_key.as_deref())?,
        expected,
        &guard,
    )?;
    store.pending.clear();
    store.meta.last_sync_version = sv;
    store.meta.last_etag = Some(new_tag);
    Ok(Report {
        downloaded: false,
        uploaded: true,
        ops_uploaded: 0,
    })
}

#[derive(Debug, Default)]
pub struct Report {
    pub downloaded: bool,
    pub uploaded: bool,
    pub ops_uploaded: usize,
}

/// One sync cycle, including persistence for callers that exclusively own the store.
pub fn sync(cfg: &NextcloudCfg, store: &mut Store) -> Result<Report, SyncError> {
    let report = exchange(cfg, store)?;
    store.save()?;
    Ok(report)
}

/// Exchange a snapshot in memory without writing to its backing directory.
/// Concurrent store owners must merge local changes and validate their replacement
/// generation before persisting the result. Errors may leave this snapshot mutated.
pub fn exchange(cfg: &NextcloudCfg, store: &mut Store) -> Result<Report, SyncError> {
    exchange_guarded(cfg, store, DEFAULT_EXCHANGE_TIMEOUT, || true)
}

/// Cancellation is cooperative between requests and before returning a result.
/// An in-flight request is bounded by the remaining whole-exchange deadline.
/// Callers must still recheck their generation under the final persistence lock.
pub fn exchange_guarded(
    cfg: &NextcloudCfg,
    store: &mut Store,
    timeout: Duration,
    is_current: impl Fn() -> bool,
) -> Result<Report, SyncError> {
    let guard = ExchangeGuard {
        current: &is_current,
        deadline: Instant::now() + timeout,
    };
    guard.check()?;
    let mut report = Report::default();
    for _attempt in 0..3 {
        let remote = download_guarded(cfg, &guard)?;
        let (mut file, tag) = match remote {
            Some((f, tag)) => {
                if f.sync_version != store.meta.last_sync_version {
                    let mut state = f.state.clone();
                    // Archives live beside `state` in the file; keep them in `rest` locally so
                    // archiving ops can update them and they round-trip on upload.
                    for (k, v) in [("archiveYoung", &f.archive_young), ("archiveOld", &f.archive_old)] {
                        if let Some(v) = v {
                            state.rest.insert(k.into(), v.clone());
                        }
                    }
                    for p in &store.pending {
                        apply(&mut state, &p.action);
                    }
                    store.state = state;
                    store.meta.vector_clock = merge_clocks(&store.meta.vector_clock, &f.vector_clock);
                    report.downloaded = true;
                }
                (f, Some(tag))
            }
            None if store.state.rest.get("globalConfig").is_none() => return Err(SyncError::FreshState),
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
            guard.check()?;
            return Ok(report);
        }
        let sv = file.sync_version + 1;
        let known: std::collections::HashSet<&str> = file.recent_ops.iter().map(|o| o.id.as_str()).collect();
        let fresh: Vec<Op> = store
            .pending
            .iter()
            .filter(|p| !known.contains(p.op.id.as_str()))
            .map(|p| Op {
                sv: Some(sv),
                ..p.op.clone()
            })
            .collect();
        file.recent_ops.extend(fresh);
        let cut = file.recent_ops.len().saturating_sub(MAX_RECENT_OPS);
        file.recent_ops.drain(..cut);
        file.oldest_op_sync_version = file.recent_ops.first().and_then(|o| o.sv);
        file.sync_version = sv;
        file.schema_version = SCHEMA_VERSION;
        file.vector_clock = store.meta.vector_clock.clone();
        file.last_modified = now_ms();
        file.client_id = store.meta.client_id.clone();
        file.state = store.state.clone();
        file.archive_young = file.state.rest.remove("archiveYoung").or(file.archive_young);
        file.archive_old = file.state.rest.remove("archiveOld").or(file.archive_old);
        match upload(
            cfg,
            &encode(&file, cfg.compress, cfg.encrypt_key.as_deref())?,
            tag.as_deref(),
            &guard,
        ) {
            Ok(new_tag) => {
                report.uploaded = true;
                report.ops_uploaded = store.pending.len();
                store.pending.clear();
                store.meta.last_sync_version = sv;
                store.meta.last_etag = Some(new_tag);
                guard.check()?;
                return Ok(report);
            }
            Err(SyncError::Conflict) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(SyncError::Conflict)
}

#[cfg(test)]
mod feature_tests;
#[cfg(test)]
mod mock_dav;

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
        assert!(matches!(
            decode(&enc, Some("wrong")),
            Err(SyncError::WrongEncryptionPassword)
        ));
    }
}
