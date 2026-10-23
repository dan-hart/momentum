// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Immutable operation batches with a single strong-ETag CAS publication point.
//! Callers persist the one-way collection pin before entering, then commit this
//! in-memory result under their normal source-generation and durable-store guard.
use super::*;
use momentum_store::Pending;
use std::collections::BTreeSet;
#[path = "optimized_crypto.rs"]
mod secure;
const MANIFEST: &str = "manifest.json";
const ENVELOPE_FILE: &str = "key-envelope.json";
const CHECKPOINT_INTERVAL: usize = 32;
fn bad(message: &str) -> SyncError {
    SyncError::Parse(format!("optimized sync: {message}"))
}
fn digest(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}
/// Stable non-secret identity used by the device-local one-way upgrade pin.
pub fn collection_identity(cfg: &NextcloudCfg) -> String {
    let server = cfg.server_url.trim_end_matches('/');
    let canonical = server
        .parse::<ureq::http::Uri>()
        .ok()
        .and_then(|uri| {
            let scheme = uri.scheme_str()?.to_ascii_lowercase();
            let host = uri.host()?.to_ascii_lowercase();
            let port = uri
                .port_u16()
                .filter(|port| !((*port == 443 && scheme == "https") || (*port == 80 && scheme == "http")));
            Some(format!(
                "{scheme}://{host}{}{}",
                port.map(|p| format!(":{p}")).unwrap_or_default(),
                uri.path().trim_end_matches('/')
            ))
        })
        .unwrap_or_else(|| server.to_owned());
    // folder_url/auth encode the trimmed username on the wire; preserve its case.
    digest(
        &serde_json::to_vec(&(
            canonical,
            cfg.user_name.trim(),
            cfg.folder.trim_matches('/'),
            "momentum-v3",
        ))
        .unwrap(),
    )
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Object {
    id: String,
    hash: String,
    kind: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    schema: u32,
    collection: String,
    checkpoint: Object,
    batches: Vec<Object>,
}
#[derive(Serialize, Deserialize)]
struct Checkpoint {
    version: u32,
    state: AppData,
    clock: VectorClock,
    applied: BTreeSet<String>,
    covered: Vec<String>,
    archives: Vec<Object>,
}
#[derive(Serialize, Deserialize)]
struct Batch {
    version: u32,
    ops: Vec<Pending>,
}
#[derive(Serialize, Deserialize)]
struct Archive {
    version: u32,
    name: String,
    value: Value,
}
fn version(version: u32) -> Result<(), SyncError> {
    if version != 3 {
        Err(SyncError::Version(version as u64))
    } else {
        Ok(())
    }
}
fn strong(tag: &str) -> bool {
    tag.starts_with('"') && tag.ends_with('"') && tag.len() > 2
}
fn read(cfg: &NextcloudCfg, name: &str, guard: &ExchangeGuard<'_>) -> Result<Option<(String, String)>, SyncError> {
    guard.retry_read(|| {
        let result = guard
            .agent()?
            .get(&format!("{}/{}", cfg.folder_url(), name))
            .header("Authorization", cfg.auth())
            .call();
        guard.check()?;
        match result {
            Ok(mut response) => {
                let tag = etag(&response).unwrap_or_default();
                if !strong(&tag) {
                    return Err(SyncError::UploadUnverified);
                }
                let body = response.body_mut().with_config().limit(512 << 20).read_to_string()?;
                guard.check()?;
                Ok(Some((body, tag)))
            }
            Err(ureq::Error::StatusCode(404)) => Ok(None),
            Err(error) => Err(error.into()),
        }
    })
}
fn put(
    cfg: &NextcloudCfg,
    name: &str,
    body: &str,
    expected: Option<&str>,
    guard: &ExchangeGuard<'_>,
) -> Result<String, SyncError> {
    let tag = upload_named(cfg, name, body, expected, guard)?;
    let Some((actual, actual_tag)) = read(cfg, name, guard)? else {
        return Err(SyncError::UploadUnverified);
    };
    if actual != body || actual_tag != tag {
        return Err(SyncError::UploadUnverified);
    }
    Ok(tag)
}
fn ensure_collection(cfg: &NextcloudCfg, guard: &ExchangeGuard<'_>) -> Result<(), SyncError> {
    let result = guard.agent()?.run(
        ureq::http::Request::builder()
            .method("MKCOL")
            .uri(cfg.folder_url())
            .header("Authorization", cfg.auth())
            .body(())
            .unwrap(),
    );
    guard.check()?;
    match result {
        Ok(_) | Err(ureq::Error::StatusCode(405)) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
fn object_valid(object: &Object, kind: &str) -> Result<(), SyncError> {
    if object.kind != kind
        || object.id.len() != 64
        || object.hash.len() != 64
        || !object
            .id
            .bytes()
            .chain(object.hash.bytes())
            .all(|c| c.is_ascii_hexdigit())
    {
        return Err(bad("invalid immutable object reference"));
    }
    Ok(())
}
fn save_object<T: Serialize>(
    cfg: &NextcloudCfg,
    key: &secure::Key,
    kind: &str,
    value: &T,
    guard: &ExchangeGuard<'_>,
) -> Result<Object, SyncError> {
    for _ in 0..3 {
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|_| bad("random source unavailable"))?;
        let id = digest(&random);
        let body = key.seal(
            kind,
            &id,
            &serde_json::to_vec(value).map_err(|_| bad("object encoding failed"))?,
        )?;
        let reference = Object {
            id,
            hash: digest(body.as_bytes()),
            kind: kind.into(),
        };
        match put(cfg, &reference.id, &body, None, guard) {
            Ok(_) => {
                key.remember(&reference, &body);
                return Ok(reference);
            }
            Err(SyncError::Conflict) => continue,
            Err(error) => return Err(error),
        }
    }
    Err(SyncError::Conflict)
}
fn load_object<T: for<'de> Deserialize<'de>>(
    cfg: &NextcloudCfg,
    key: &secure::Key,
    reference: &Object,
    kind: &str,
    guard: &ExchangeGuard<'_>,
) -> Result<T, SyncError> {
    object_valid(reference, kind)?;
    let body = if let Some(body) = key.cached(reference) {
        body
    } else {
        let Some((body, _)) = read(cfg, &reference.id, guard)? else {
            return Err(bad("manifest references a missing immutable object"));
        };
        body
    };
    if digest(body.as_bytes()) != reference.hash {
        return Err(bad("immutable object checksum mismatch"));
    }
    let plain = key.unseal(kind, &reference.id, &body)?;
    key.remember(reference, &body);
    serde_json::from_slice(&plain).map_err(|_| bad("invalid immutable object payload"))
}
fn checkpoint(
    cfg: &NextcloudCfg,
    key: &secure::Key,
    state: &AppData,
    clock: &VectorClock,
    applied: &BTreeSet<String>,
    covered: Vec<String>,
    guard: &ExchangeGuard<'_>,
) -> Result<Object, SyncError> {
    let mut state = state.clone();
    let mut archives = Vec::new();
    for name in ["archiveYoung", "archiveOld"] {
        if let Some(value) = state.rest.remove(name) {
            archives.push(save_object(
                cfg,
                key,
                "archive",
                &Archive {
                    version: 3,
                    name: name.into(),
                    value,
                },
                guard,
            )?);
        }
    }
    save_object(
        cfg,
        key,
        "checkpoint",
        &Checkpoint {
            version: 3,
            state,
            clock: clock.clone(),
            applied: applied.clone(),
            covered,
            archives,
        },
        guard,
    )
}
fn reconstruct(
    cfg: &NextcloudCfg,
    identity: &str,
    key: &secure::Key,
    manifest: &Manifest,
    guard: &ExchangeGuard<'_>,
) -> Result<(AppData, VectorClock, BTreeSet<String>), SyncError> {
    version(manifest.version)?;
    if manifest.schema > SCHEMA_VERSION {
        return Err(SyncError::Schema(manifest.schema));
    }
    if manifest.collection != identity {
        return Err(bad("manifest collection mismatch"));
    }
    let cp: Checkpoint = load_object(cfg, key, &manifest.checkpoint, "checkpoint", guard)?;
    version(cp.version)?;
    if cp.covered.len() > manifest.batches.len() || !cp.covered.iter().zip(&manifest.batches).all(|(a, b)| a == &b.id) {
        return Err(bad("checkpoint batch coverage mismatch"));
    }
    let mut unique = BTreeSet::new();
    for batch in &manifest.batches {
        object_valid(batch, "batch")?;
        if !unique.insert(&batch.id) {
            return Err(bad("duplicate manifest batch"));
        }
    }
    let mut state = cp.state;
    let mut clock = cp.clock;
    let mut applied = cp.applied;
    let mut names = BTreeSet::new();
    for reference in cp.archives {
        let archive: Archive = load_object(cfg, key, &reference, "archive", guard)?;
        version(archive.version)?;
        if !["archiveYoung", "archiveOld"].contains(&archive.name.as_str()) || !names.insert(archive.name.clone()) {
            return Err(bad("invalid archive object"));
        }
        state.rest.insert(archive.name, archive.value);
    }
    for reference in &manifest.batches[cp.covered.len()..] {
        let batch: Batch = load_object(cfg, key, reference, "batch", guard)?;
        version(batch.version)?;
        for pending in batch.ops {
            if applied.insert(pending.op.id.clone()) {
                apply(&mut state, &pending.action);
                clock = merge_clocks(&clock, &pending.op.v);
            }
        }
    }
    Ok((state, clock, applied))
}
/// Exchange without touching either legacy cursor or disk. No failed exchange
/// acknowledges pending operations; the owner must durably commit successful results.
pub fn exchange_guarded(
    cfg: &NextcloudCfg,
    store: &mut Store,
    timeout: Duration,
    is_current: impl Fn() -> bool,
) -> Result<Report, SyncError> {
    Session::default().exchange_guarded(cfg, store, timeout, is_current)
}
fn exchange_inner(
    session: &mut Session,
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
    let mut optimized = cfg.clone();
    optimized.folder = format!("{}/momentum-v3", cfg.folder.trim_matches('/'));
    let cfg3 = &optimized;
    let identity = collection_identity(cfg); // identity is bound to legacy root, not nested cfg.
    let initialized = marker_required(cfg, store.dir(), ".initialized")?;
    let key = session.obtain_key(cfg, cfg3, &identity, initialized, &guard)?;
    for _ in 0..3 {
        guard.check()?;
        let remote = read(cfg3, MANIFEST, &guard)?;
        let mut cursor = remote
            .as_ref()
            .map(|(body, tag)| (tag.clone(), digest(body.as_bytes())));
        if store.pending.is_empty()
            && cursor
                .as_ref()
                .is_some_and(|cursor| store.meta.nextcloud_optimized_cursors.get(&identity) == Some(cursor))
        {
            // A task restore intentionally retains transport receipts. An unchanged
            // server must not replace that local state or invalidate native undo.
            guard.check()?;
            return Ok(Report::default());
        }
        let (mut manifest, tag, mut state, mut clock, mut applied, downloaded) = if let Some((body, tag)) = remote {
            let manifest: Manifest = serde_json::from_slice(&key.unseal("manifest", MANIFEST, &body)?)
                .map_err(|_| bad("invalid manifest"))?;
            if manifest.collection != identity {
                return Err(bad("manifest collection mismatch"));
            }
            let (state, clock, applied) = reconstruct(cfg3, &identity, &key, &manifest, &guard)?;
            (manifest, Some(tag), state, clock, applied, true)
        } else {
            if initialized {
                return Err(bad(
                    "initialized optimized collection has no manifest; refusing stale legacy bootstrap",
                ));
            }
            // Only the winner of the first manifest CAS bootstraps. Losing initializers
            // retry from the new optimized manifest, never from this stale legacy copy.
            let (state, clock, applied) = match download_validated(cfg, &guard, None)? {
                Download::File(file, ..) => {
                    let mut state = file.state;
                    for (name, value) in [("archiveYoung", file.archive_young), ("archiveOld", file.archive_old)] {
                        if let Some(value) = value {
                            state.rest.insert(name.into(), value);
                        }
                    }
                    (
                        state,
                        file.vector_clock,
                        file.recent_ops.into_iter().map(|o| o.id).collect(),
                    )
                }
                Download::Missing => {
                    if store.state.rest.get("globalConfig").is_none() {
                        return Err(SyncError::FreshState);
                    }
                    (
                        store.state.clone(),
                        store.meta.vector_clock.clone(),
                        store.pending.iter().map(|p| p.op.id.clone()).collect(),
                    )
                }
                Download::Unchanged => unreachable!(),
            };
            let checkpoint = checkpoint(cfg3, &key, &state, &clock, &applied, vec![], &guard)?;
            (
                Manifest {
                    version: 3,
                    schema: SCHEMA_VERSION,
                    collection: identity.clone(),
                    checkpoint,
                    batches: vec![],
                },
                None,
                state,
                clock,
                applied,
                false,
            )
        };
        let fresh: Vec<Pending> = store
            .pending
            .iter()
            .filter(|p| !applied.contains(&p.op.id))
            .cloned()
            .collect();
        for p in &fresh {
            if applied.insert(p.op.id.clone()) {
                apply(&mut state, &p.action);
                clock = merge_clocks(&clock, &p.op.v);
            }
        }
        let must_publish = tag.is_none() || !fresh.is_empty();
        if !fresh.is_empty() {
            let reference = save_object(cfg3, &key, "batch", &Batch { version: 3, ops: fresh }, &guard)?;
            manifest.batches.push(reference);
        }
        if must_publish && manifest.batches.len() > 0 && manifest.batches.len() % CHECKPOINT_INTERVAL == 0 {
            manifest.checkpoint = checkpoint(
                cfg3,
                &key,
                &state,
                &clock,
                &applied,
                manifest.batches.iter().map(|b| b.id.clone()).collect(),
                &guard,
            )?;
        }
        if must_publish {
            let body = key.seal(
                "manifest",
                MANIFEST,
                &serde_json::to_vec(&manifest).map_err(|_| bad("manifest encoding failed"))?,
            )?;
            match put(cfg3, MANIFEST, &body, tag.as_deref(), &guard) {
                Ok(tag) => cursor = Some((tag, digest(body.as_bytes()))),
                Err(SyncError::Conflict) => continue,
                Err(error) => return Err(error),
            }
        }
        guard.check()?;
        mark_initialized(cfg, store.dir())?;
        guard.check()?;
        let ops_uploaded = store.pending.len();
        store.state = state;
        store.meta.vector_clock = merge_clocks(&store.meta.vector_clock, &clock);
        if let Some(cursor) = cursor {
            store.meta.nextcloud_optimized_cursors.insert(identity.clone(), cursor);
        }
        store.pending.clear();
        return Ok(Report {
            downloaded,
            uploaded: must_publish,
            ops_uploaded: if must_publish { ops_uploaded } else { 0 },
        });
    }
    Err(SyncError::Conflict)
}
/// Holds at most one vault key, for a fixed 60-second lifetime. It contains no
/// password strings, and invalidation/drop wipes the key. Credential, collection
/// or envelope changes cannot reuse it. Engines should invalidate on lifecycle
/// cancellation, configuration change, provider switch and store replacement.
#[derive(Default)]
pub struct Session {
    key: Option<CachedKey>,
}
struct CachedKey {
    fingerprint: String,
    envelope: String,
    expires: Instant,
    key: std::sync::Arc<std::sync::Mutex<Option<secure::Key>>>,
    stop: std::sync::Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
}
impl CachedKey {
    fn new(fingerprint: String, envelope: String, key: secure::Key, lifetime: Duration) -> Option<Self> {
        let key = std::sync::Arc::new(std::sync::Mutex::new(Some(key)));
        let weak = std::sync::Arc::downgrade(&key);
        let stop = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
        let stop_thread = stop.clone();
        // The sleeper carries only a weak reference, never a secret or a strong owner.
        // Thus invalidation/drop wipes immediately, and idle sessions also expire.
        std::thread::Builder::new()
            .name("momentum-vault-expiry".into())
            .spawn(move || {
                let (lock, wake) = &*stop_thread;
                let Ok(stopped) = lock.lock() else {
                    return;
                };
                let Ok((stopped, _)) = wake.wait_timeout_while(stopped, lifetime, |stopped| !*stopped) else {
                    return;
                };
                if *stopped {
                    return;
                }
                if let Some(key) = weak.upgrade() {
                    if let Ok(mut key) = key.lock() {
                        *key = None;
                    }
                }
            })
            .ok()?;
        Some(Self {
            fingerprint,
            envelope,
            expires: Instant::now() + lifetime,
            key,
            stop,
        })
    }
    fn get(&self, fingerprint: &str, envelope: &str) -> Option<secure::Key> {
        if self.fingerprint != fingerprint || self.envelope != envelope || Instant::now() >= self.expires {
            return None;
        }
        self.key.lock().ok()?.as_ref().cloned()
    }
}
impl Drop for CachedKey {
    fn drop(&mut self) {
        let (lock, wake) = &*self.stop;
        if let Ok(mut stopped) = lock.lock() {
            *stopped = true;
            wake.notify_one();
        }
    }
}
impl Session {
    pub fn invalidate(&mut self) {
        self.key = None;
    }
    pub fn exchange_guarded(
        &mut self,
        cfg: &NextcloudCfg,
        store: &mut Store,
        timeout: Duration,
        is_current: impl Fn() -> bool,
    ) -> Result<Report, SyncError> {
        let result = exchange_inner(self, cfg, store, timeout, is_current);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn obtain_key(
        &mut self,
        cfg: &NextcloudCfg,
        cfg3: &NextcloudCfg,
        identity: &str,
        initialized: bool,
        guard: &ExchangeGuard<'_>,
    ) -> Result<secure::Key, SyncError> {
        let fingerprint = digest(&serde_json::to_vec(&(identity, &cfg.password, &cfg.encrypt_key)).unwrap());
        match read(cfg3, ENVELOPE_FILE, guard)? {
            Some((body, _)) => {
                let envelope = digest(body.as_bytes());
                if let Some(key) = self.key.as_ref().and_then(|cached| cached.get(&fingerprint, &envelope)) {
                    return Ok(key);
                }
                self.invalidate();
                let key = secure::open(&body, identity, cfg.encrypt_key.as_deref())?;
                guard.check()?;
                self.key = CachedKey::new(fingerprint, envelope, key.clone(), Duration::from_secs(60));
                Ok(key)
            }
            None => {
                self.invalidate();
                if initialized || read(cfg3, MANIFEST, guard)?.is_some() {
                    return Err(bad("manifest exists without its key envelope"));
                }
                ensure_collection(cfg, guard)?;
                ensure_collection(cfg3, guard)?;
                let (body, key) = secure::create(identity, cfg.encrypt_key.as_deref())?;
                guard.check()?;
                match put(cfg3, ENVELOPE_FILE, &body, None, guard) {
                    Ok(_) => {
                        self.key = CachedKey::new(
                            fingerprint,
                            digest(body.as_bytes()),
                            key.clone(),
                            Duration::from_secs(60),
                        );
                        Ok(key)
                    }
                    Err(SyncError::Conflict) => {
                        let Some((body, _)) = read(cfg3, ENVELOPE_FILE, guard)? else {
                            return Err(SyncError::UploadUnverified);
                        };
                        let key = secure::open(&body, identity, cfg.encrypt_key.as_deref())?;
                        guard.check()?;
                        self.key = CachedKey::new(
                            fingerprint,
                            digest(body.as_bytes()),
                            key.clone(),
                            Duration::from_secs(60),
                        );
                        Ok(key)
                    }
                    Err(error) => Err(error),
                }
            }
        }
    }
}
/// Reconstruct and copy only the committed optimized dataset. Objects are newly
/// encrypted for the destination identity; existing destination collections are
/// never overwritten. Source files remain byte-for-byte untouched.
pub fn copy_collection_guarded(
    cfg: &NextcloudCfg,
    target_folder: &str,
    timeout: Duration,
    is_current: impl Fn() -> bool,
) -> Result<FolderMigrationReport, SyncError> {
    let guard = ExchangeGuard {
        current: &is_current,
        deadline: Instant::now() + timeout,
    };
    guard.check()?;
    let mut target = cfg.clone();
    target.folder = target_folder.trim_matches('/').into();
    if target.folder.is_empty() || collection_identity(cfg) == collection_identity(&target) {
        return Err(bad("destination must be a different folder"));
    }
    let mut source3 = cfg.clone();
    source3.folder = format!("{}/momentum-v3", cfg.folder.trim_matches('/'));
    let mut target3 = target.clone();
    target3.folder = format!("{}/momentum-v3", target.folder);
    if read(&target3, ENVELOPE_FILE, &guard)?.is_some() || read(&target3, MANIFEST, &guard)?.is_some() {
        return Err(SyncError::Conflict);
    }
    if read(&target, SETTINGS_FILE, &guard)?.is_some() || read(&target, SYNC_FILE, &guard)?.is_some() {
        return Err(SyncError::Conflict);
    }
    let source_settings = read(cfg, SETTINGS_FILE, &guard)?;
    let Some((envelope, _)) = read(&source3, ENVELOPE_FILE, &guard)? else {
        return Err(bad("source key envelope missing"));
    };
    let source_key = secure::open(&envelope, &collection_identity(cfg), cfg.encrypt_key.as_deref())?;
    let Some((body, tag)) = read(&source3, MANIFEST, &guard)? else {
        return Err(bad("source manifest missing"));
    };
    let manifest: Manifest = serde_json::from_slice(&source_key.unseal("manifest", MANIFEST, &body)?)
        .map_err(|_| bad("invalid source manifest"))?;
    let (state, clock, applied) = reconstruct(&source3, &collection_identity(cfg), &source_key, &manifest, &guard)?;
    ensure_collection(&target, &guard)?;
    ensure_collection(&target3, &guard)?;
    let identity = collection_identity(&target);
    let (envelope, key) = secure::create(&identity, cfg.encrypt_key.as_deref())?;
    put(&target3, ENVELOPE_FILE, &envelope, None, &guard)?;
    let cp = checkpoint(&target3, &key, &state, &clock, &applied, vec![], &guard)?;
    let cp_payload: Checkpoint = load_object(&target3, &key, &cp, "checkpoint", &guard)?;
    let mut target_files = vec![
        format!("momentum-v3/{ENVELOPE_FILE}"),
        format!("momentum-v3/{}", cp.id),
        format!("momentum-v3/{MANIFEST}"),
    ];
    target_files.extend(cp_payload.archives.iter().map(|o| format!("momentum-v3/{}", o.id)));
    if let Some((body, _)) = &source_settings {
        put(&target, SETTINGS_FILE, body, None, &guard)?;
        target_files.push(SETTINGS_FILE.into());
    }
    let manifest = Manifest {
        version: 3,
        schema: SCHEMA_VERSION,
        collection: identity,
        checkpoint: cp,
        batches: vec![],
    };
    let encoded = key.seal(
        "manifest",
        MANIFEST,
        &serde_json::to_vec(&manifest).map_err(|_| bad("invalid destination manifest"))?,
    )?;
    if read(&source3, MANIFEST, &guard)? != Some((body, tag)) || read(cfg, SETTINGS_FILE, &guard)? != source_settings {
        return Err(SyncError::Conflict);
    }
    put(&target3, MANIFEST, &encoded, None, &guard)?;
    guard.check()?;
    Ok(FolderMigrationReport {
        copied_files: target_files.len() as u32,
        target_files,
    })
}
#[cfg(test)]
#[path = "optimized_tests.rs"]
mod tests;
/// The upgrade decision is deliberately outside task metadata/backups/restores.
/// An unreadable or malformed marker is an error, never permission to downgrade.
pub fn require_optimized(cfg: &NextcloudCfg, store_dir: &std::path::Path) -> Result<bool, SyncError> {
    marker_required(cfg, store_dir, "")
}
fn marker_required(cfg: &NextcloudCfg, store_dir: &std::path::Path, suffix: &str) -> Result<bool, SyncError> {
    let directory = store_dir.join(".nextcloud-protocol");
    match std::fs::symlink_metadata(&directory) {
        Ok(meta) if !meta.is_dir() => return Err(bad("protocol marker directory is invalid")),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    }
    let identity = collection_identity(cfg);
    let path = directory.join(format!("{identity}{suffix}"));
    match std::fs::symlink_metadata(&path) {
        Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => return Err(bad("protocol marker is invalid")),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    }
    let mut body = String::new();
    std::fs::File::open(path)?.take(256).read_to_string(&mut body)?;
    if body != format!("momentum-nextcloud-v3\n{identity}\n") {
        return Err(bad("protocol marker is damaged; legacy sync remains disabled"));
    }
    Ok(true)
}
/// Persist before any optimized exchange, including attempts that later fail.
/// There is intentionally no unpin/downgrade API without a verified migration.
pub fn pin_optimized(cfg: &NextcloudCfg, store_dir: &std::path::Path) -> Result<(), SyncError> {
    pin_marker(cfg, store_dir, "")
}
/// Persist a witness of a verified committed collection, independently of task
/// backup metadata. A missing manifest thereafter must never bootstrap legacy data.
pub fn mark_initialized(cfg: &NextcloudCfg, store_dir: &std::path::Path) -> Result<(), SyncError> {
    pin_optimized(cfg, store_dir)?;
    pin_marker(cfg, store_dir, ".initialized")
}
fn pin_marker(cfg: &NextcloudCfg, store_dir: &std::path::Path, suffix: &str) -> Result<(), SyncError> {
    let directory = store_dir.join(".nextcloud-protocol");
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let identity = collection_identity(cfg);
    let path = directory.join(format!("{identity}{suffix}"));
    if !marker_required(cfg, store_dir, suffix)? {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(mut file) => {
                file.write_all(format!("momentum-nextcloud-v3\n{identity}\n").as_bytes())?;
                file.sync_all()?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if !marker_required(cfg, store_dir, suffix)? {
                    return Err(bad("protocol marker disappeared"));
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
    // Also sync an existing marker: another writer may have just finished writing
    // it but not reached its own durability boundary yet.
    std::fs::File::open(&path)?.sync_all()?;
    #[cfg(unix)]
    {
        std::fs::File::open(&directory)?.sync_all()?;
        std::fs::File::open(store_dir)?.sync_all()?;
    }
    Ok(())
}
