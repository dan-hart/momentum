// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in checked copy. No source deletion or connection-folder mutation occurs here.
use super::*;
use std::collections::BTreeSet;
#[derive(Debug)]
pub struct FolderMigrationReport {
    pub copied_files: u32,
    pub target_files: Vec<String>,
}
fn dav_path(href: &str) -> Result<String, SyncError> {
    let uri: ureq::http::Uri = href
        .parse()
        .map_err(|e| SyncError::Parse(format!("Invalid DAV path: {e}")))?;
    let bytes = uri.path().as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(SyncError::Parse("Incomplete DAV path escape".into()));
            }
            let hex = |b: u8| match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                b'A'..=b'F' => Some(b - b'A' + 10),
                _ => None,
            };
            let value = hex(bytes[i + 1])
                .zip(hex(bytes[i + 2]))
                .map(|(a, b)| a * 16 + b)
                .ok_or_else(|| SyncError::Parse("Invalid DAV path escape".into()))?;
            decoded.push(value);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| SyncError::Parse("DAV path is not UTF-8".into()))
}
fn collection(cfg: &NextcloudCfg, guard: &ExchangeGuard<'_>) -> Result<Option<BTreeSet<String>>, SyncError> {
    guard.retry_read(|| {
        let response = guard.agent()?.run(
            ureq::http::Request::builder()
                .method("PROPFIND")
                .uri(cfg.folder_url())
                .header("Authorization", cfg.auth())
                .header("Depth", "1")
                .body(())
                .unwrap(),
        );
        guard.check()?;
        let mut response = match response {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(404)) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let xml = response.body_mut().with_config().limit(1 << 20).read_to_string()?;
        guard.check()?;
        let mut names = BTreeSet::new();
        let mut rest = xml.as_str();
        let mut seen = false;
        let root = dav_path(&cfg.folder_url())?.trim_end_matches('/').to_string();
        loop {
            let Some(start) = rest.find('<') else { break };
            rest = &rest[start + 1..];
            let Some(tag_end) = rest.find('>') else {
                return Err(SyncError::Parse("Incomplete folder inventory".into()));
            };
            let tag = rest[..tag_end].split_whitespace().next().unwrap_or("");
            rest = &rest[tag_end + 1..];
            if tag.starts_with('/') || !(tag == "href" || tag.ends_with(":href")) {
                continue;
            }
            let Some(end) = rest.find("</") else {
                return Err(SyncError::Parse("Incomplete folder inventory".into()));
            };
            let href = rest[..end].replace("&amp;", "&").replace("&quot;", "\"");
            let path = dav_path(&href)?;
            seen = true;
            if path.trim_end_matches('/') != root {
                let prefix = format!("{root}/");
                let name = path
                    .strip_prefix(&prefix)
                    .ok_or_else(|| SyncError::Parse("Folder inventory contains an unrelated path".into()))?;
                if name.is_empty() {
                    return Err(SyncError::Parse("Invalid folder member".into()));
                }
                names.insert(name.to_string());
            }
            rest = &rest[end..];
        }
        if !seen {
            return Err(SyncError::Parse(
                "Server did not provide a verifiable folder inventory".into(),
            ));
        }
        Ok(Some(names))
    })
}
pub fn migrate_folder_guarded(
    cfg: &NextcloudCfg,
    target: &str,
    timeout: Duration,
    is_current: impl Fn() -> bool,
) -> Result<FolderMigrationReport, SyncError> {
    let target = target.trim_matches('/');
    if target.is_empty()
        || target == cfg.folder.trim_matches('/')
        || target.split('/').any(|s| {
            s.is_empty()
                || s == "."
                || s == ".."
                || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return Err(SyncError::Parse("Choose a different valid destination folder".into()));
    }
    let guard = ExchangeGuard {
        current: &is_current,
        deadline: Instant::now() + timeout,
    };
    let mut destination = cfg.clone();
    destination.folder = target.into();
    let source_names = collection(cfg, &guard)?.ok_or(SyncError::FreshState)?;
    if !source_names.contains(SYNC_FILE) {
        return Err(SyncError::FreshState);
    }
    let supported = [SYNC_FILE, SETTINGS_FILE, DAMAGED_FILE];
    if source_names.iter().any(|name| !supported.contains(&name.as_str())) {
        return Err(SyncError::Parse(
            "Source folder has additional files; keep the current folder or copy those files manually".into(),
        ));
    }
    let target_names = collection(&destination, &guard)?.unwrap_or_default();
    if target_names.iter().any(|name| !source_names.contains(name)) {
        return Err(SyncError::Conflict);
    }
    let mut sources = Vec::new();
    for name in &source_names {
        let source = portable::read_named(cfg, name, &guard)?.ok_or(SyncError::Conflict)?;
        if let Some((body, _)) = portable::read_named(&destination, name, &guard)? {
            if body != source.0 {
                return Err(SyncError::Conflict);
            }
        }
        sources.push((name.clone(), source));
    }
    let unchanged = || -> Result<(), SyncError> {
        if collection(cfg, &guard)?.as_ref() != Some(&source_names) {
            return Err(SyncError::Conflict);
        }
        for (name, source) in &sources {
            if portable::read_named(cfg, name, &guard)?.as_ref() != Some(source) {
                return Err(SyncError::Conflict);
            }
        }
        Ok(())
    };
    unchanged()?;
    for (name, (body, _)) in &sources {
        guard.check()?;
        if portable::read_named(&destination, name, &guard)?.is_none() {
            upload_named(&destination, name, body, None, &guard)?;
        }
        let Some((readback, _)) = portable::read_named(&destination, name, &guard)? else {
            return Err(SyncError::UploadUnverified);
        };
        if &readback != body {
            return Err(SyncError::UploadUnverified);
        }
    }
    unchanged()?;
    if collection(&destination, &guard)?.as_ref() != Some(&source_names) {
        return Err(SyncError::Conflict);
    }
    // Recheck the destination after the source pass too: another device may have
    // overwritten an earlier copied member while a later member was verified.
    for (name, (body, _)) in &sources {
        if portable::read_named(&destination, name, &guard)?
            .as_ref()
            .map(|(readback, _)| readback)
            != Some(body)
        {
            return Err(SyncError::UploadUnverified);
        }
    }
    unchanged()?;
    guard.check()?;
    Ok(FolderMigrationReport {
        copied_files: sources.len() as u32,
        target_files: source_names.into_iter().collect(),
    })
}
