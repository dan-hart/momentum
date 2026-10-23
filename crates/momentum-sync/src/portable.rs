// SPDX-License-Identifier: GPL-3.0-or-later
//! An independent optional payload, using the configured transport's encryption.
use super::*;
pub const SETTINGS_FILE: &str = "momentum-settings.json";
#[derive(Debug)]
pub struct PortableExchange {
    pub remote: Option<String>,
    pub published: Option<String>,
    pub uploaded: bool,
}
pub(super) fn read_named(
    cfg: &NextcloudCfg,
    name: &str,
    guard: &ExchangeGuard<'_>,
) -> Result<Option<(String, String)>, SyncError> {
    match read_named_validated(cfg, name, guard, None)? {
        NamedDownload::File(body, tag) => Ok(Some((body, tag))),
        NamedDownload::Missing => Ok(None),
        NamedDownload::Unchanged => Err(SyncError::UploadUnverified),
    }
}
enum NamedDownload {
    File(String, String),
    Missing,
    Unchanged,
}
fn read_named_validated(
    cfg: &NextcloudCfg,
    name: &str,
    guard: &ExchangeGuard<'_>,
    known: Option<&str>,
) -> Result<NamedDownload, SyncError> {
    guard.retry_read(|| {
        let mut request = guard
            .agent()?
            .get(&format!("{}/{}", cfg.folder_url(), name))
            .header("Authorization", cfg.auth());
        if let Some(tag) = known {
            request = request.header("If-None-Match", tag);
        }
        let response = request.call();
        guard.check()?;
        match response {
            Ok(mut r) => {
                if r.status() == 304 {
                    return if known.is_some() {
                        Ok(NamedDownload::Unchanged)
                    } else {
                        Err(SyncError::UploadUnverified)
                    };
                }
                let tag = etag(&r).filter(|t| strong_etag(t)).ok_or(SyncError::UploadUnverified)?;
                let limit = if name == SETTINGS_FILE { 16 << 20 } else { 512 << 20 };
                let text = r.body_mut().with_config().limit(limit).read_to_string()?;
                guard.check()?;
                Ok(NamedDownload::File(text, tag))
            }
            Err(ureq::Error::StatusCode(404)) => Ok(NamedDownload::Missing),
            Err(e) => Err(e.into()),
        }
    })
}
fn encode_settings(text: &str, cfg: &NextcloudCfg) -> Result<String, SyncError> {
    let mut body = text.to_string();
    if cfg.compress {
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(body.as_bytes())?;
        body = base64::engine::general_purpose::STANDARD.encode(gz.finish()?);
    }
    if let Some(key) = cfg.encrypt_key.as_deref().filter(|k| !k.is_empty()) {
        body = crypto::encrypt(&body, key).map_err(SyncError::Decrypt)?;
    }
    Ok(format!(
        "ms_{}{}1__{body}",
        if cfg.compress { "C" } else { "" },
        if cfg.encrypt_key.as_ref().is_some_and(|k| !k.is_empty()) {
            "E"
        } else {
            ""
        }
    ))
}
fn decode_settings(body: &str, cfg: &NextcloudCfg) -> Result<String, SyncError> {
    let (flags, mut body) = body
        .strip_prefix("ms_")
        .and_then(|s| s.split_once("__"))
        .map(|(f, b)| (f, b.to_string()))
        .ok_or_else(|| SyncError::Parse("missing portable settings prefix".into()))?;
    if !["1", "C1", "E1", "CE1"].contains(&flags) {
        return Err(SyncError::Parse("unsupported portable settings channel version".into()));
    }
    if flags.contains('E') {
        let key = cfg
            .encrypt_key
            .as_deref()
            .filter(|k| !k.is_empty())
            .ok_or(SyncError::Encrypted)?;
        body = crypto::decrypt(&body, key).map_err(|e| match e {
            crypto::DecryptError::WrongPassword => SyncError::WrongEncryptionPassword,
            crypto::DecryptError::Damaged(d) => SyncError::Decrypt(d),
        })?;
    }
    if flags.contains('C') {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&body)
            .map_err(|e| SyncError::Parse(e.to_string()))?;
        let mut plain = String::new();
        flate2::read::GzDecoder::new(bytes.as_slice())
            .take(16 << 20)
            .read_to_string(&mut plain)
            .map_err(|e| SyncError::Parse(e.to_string()))?;
        body = plain;
    }
    Ok(body)
}
/// The caller validates and causally merges the settings payload. Returning None
/// means read-only bootstrap review; no upload is acknowledged in that case.
pub fn exchange_portable_guarded(
    cfg: &NextcloudCfg,
    timeout: Duration,
    is_current: impl Fn() -> bool,
    merge: impl FnMut(Option<&str>) -> Result<Option<String>, SyncError>,
) -> Result<PortableExchange, SyncError> {
    PortableSession::default().exchange_guarded(cfg, timeout, is_current, merge)
}
fn exchange_portable_inner(
    cfg: &NextcloudCfg,
    timeout: Duration,
    is_current: impl Fn() -> bool,
    mut merge: impl FnMut(Option<&str>) -> Result<Option<String>, SyncError>,
    cached: &mut Option<CachedSettings>,
) -> Result<PortableExchange, SyncError> {
    let guard = ExchangeGuard {
        current: &is_current,
        deadline: Instant::now() + timeout,
    };
    for attempt in 0..3 {
        let known = cached.as_ref().filter(|v| &v.cfg == cfg && attempt == 0);
        let raw = read_named_validated(cfg, SETTINGS_FILE, &guard, known.map(|v| v.tag.as_str()))?;
        let (remote, tag) = match raw {
            NamedDownload::Unchanged => {
                let cached = known.ok_or(SyncError::UploadUnverified)?;
                (Some(cached.decoded.clone()), Some(cached.tag.clone()))
            }
            NamedDownload::File(body, tag) => (Some(decode_settings(&body, cfg)?), Some(tag)),
            NamedDownload::Missing => (None, None),
        };
        *cached = remote.as_ref().zip(tag.as_ref()).map(|(decoded, tag)| CachedSettings {
            cfg: cfg.clone(),
            decoded: decoded.clone(),
            tag: tag.clone(),
        });
        let published = merge(remote.as_deref())?;
        guard.check()?;
        let Some(text) = published.as_ref() else {
            return Ok(PortableExchange {
                remote,
                published: None,
                uploaded: false,
            });
        };
        if remote.as_ref() == Some(text) {
            return Ok(PortableExchange {
                remote,
                published,
                uploaded: false,
            });
        }
        let encoded = encode_settings(text, cfg)?;
        match upload_named(cfg, SETTINGS_FILE, &encoded, tag.as_deref(), &guard) {
            Ok(tag) => {
                let Some((readback, verified_tag)) = read_named(cfg, SETTINGS_FILE, &guard)? else {
                    return Err(SyncError::UploadUnverified);
                };
                if readback != encoded || tag != verified_tag {
                    return Err(SyncError::UploadUnverified);
                };
                guard.check()?;
                *cached = Some(CachedSettings {
                    cfg: cfg.clone(),
                    decoded: text.clone(),
                    tag,
                });
                return Ok(PortableExchange {
                    remote,
                    published,
                    uploaded: true,
                });
            }
            Err(SyncError::Conflict) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(SyncError::Conflict)
}

/// Per-engine validation for the independent settings channel.
#[derive(Default)]
pub struct PortableSession {
    validated: Option<CachedSettings>,
}
struct CachedSettings {
    cfg: NextcloudCfg,
    decoded: String,
    tag: String,
}
impl PortableSession {
    pub fn exchange_guarded(
        &mut self,
        cfg: &NextcloudCfg,
        timeout: Duration,
        is_current: impl Fn() -> bool,
        merge: impl FnMut(Option<&str>) -> Result<Option<String>, SyncError>,
    ) -> Result<PortableExchange, SyncError> {
        if self.validated.as_ref().is_some_and(|v| &v.cfg != cfg) {
            self.validated = None;
        }
        let result = exchange_portable_inner(cfg, timeout, is_current, merge, &mut self.validated);
        if result.is_err() {
            self.validated = None;
        }
        result
    }
}
