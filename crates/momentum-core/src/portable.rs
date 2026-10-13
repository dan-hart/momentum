// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Private native preferences. Neither values, consent nor clocks belong in AppData.
use crate::{CoreError, Engine, GroupBy, Outcome, SortDirection, SortKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
const FILE: &str = "portable-settings.json";
pub(crate) type Values = BTreeMap<String, Value>;
type Clock = BTreeMap<String, u64>;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Document {
    pub version: u32,
    pub values: Values,
}
impl Document {
    pub(crate) fn parse(text: &str) -> Result<Self, CoreError> {
        let doc: Self = serde_json::from_str(text).map_err(invalid)?;
        if doc.version != 1 {
            return Err(invalid(
                "Unsupported app settings version; update Momentum before importing this file",
            ));
        }
        let mut values = Values::new();
        for (k, v) in doc.values {
            if validate(&k, &v)? {
                values.insert(k, v);
            }
        }
        Ok(Self { version: 1, values })
    }
    pub(crate) fn json(&self) -> String {
        serde_json::to_string(self).expect("validated JSON")
    }
}
fn invalid(e: impl std::fmt::Display) -> CoreError {
    CoreError::Invalid { message: e.to_string() }
}
fn validate(key: &str, v: &Value) -> Result<bool, CoreError> {
    let valid = match key {
        "grouping" => v
            .as_str()
            .is_some_and(|s| ["morning-night", "none", "project", "tag", "estimate"].contains(&s)),
        "sort" => v
            .as_str()
            .is_some_and(|s| ["manual", "title", "due", "estimate", "created"].contains(&s)),
        "direction" => v.as_str().is_some_and(|s| ["ascending", "descending"].contains(&s)),
        "upcomingDays" => matches!(v.as_u64(), Some(7 | 30)),
        "autoArchive" | "colorfulLabels" | "completionSoundEnabled" => v.is_boolean(),
        "rowDensity" => v.as_str().is_some_and(|s| ["regular", "compact"].contains(&s)),
        "completionSoundChoice" => v.as_str().is_some_and(|s| {
            [
                "wood-pop",
                "double-check",
                "rounded-lift",
                "pocket-piano",
                "string-flick",
                "bright-finish",
                "harp-snap",
                "click-and-glow",
                "tiny-ta-da",
                "warm-spark",
            ]
            .contains(&s)
        }),
        _ => return Ok(false),
    };
    if valid {
        Ok(true)
    } else {
        Err(invalid(format!(
            "Invalid app setting '{key}'; this device's settings were preserved"
        )))
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Cell {
    pub value: Value,
    pub clock: Clock,
    pub actor: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<Version>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Version {
    value: Value,
    clock: Clock,
    actor: String,
}
impl Cell {
    fn versions(&self) -> Vec<Version> {
        let mut values = self.conflicts.clone();
        values.push(Version {
            value: self.value.clone(),
            clock: self.clock.clone(),
            actor: self.actor.clone(),
        });
        values
    }
    fn context(&self) -> Clock {
        self.conflicts
            .iter()
            .fold(self.clock.clone(), |clock, c| merge_clocks(&clock, &c.clock))
    }
    fn merge(&self, other: &Self) -> Self {
        // Retain the causal frontier, including losing concurrent values. Collapsing
        // it into a synthetic winner clock makes three-device merge order matter.
        // A local edit observes the entire frontier and causally supersedes it.
        let mut versions = self.versions();
        versions.extend(other.versions());
        versions.sort_by_key(|v| (v.actor.clone(), v.clock.clone(), v.value.to_string()));
        versions.dedup();
        let frontier: Vec<_> = versions
            .iter()
            .filter(|v| !versions.iter().any(|other| dominates(&other.clock, &v.clock)))
            .cloned()
            .collect();
        let winner = frontier
            .iter()
            .max_by_key(|v| (v.actor.clone(), v.value.to_string(), v.clock.clone()))
            .expect("nonempty causal frontier")
            .clone();
        let conflicts = frontier.into_iter().filter(|v| v != &winner).collect();
        Self {
            value: winner.value,
            clock: winner.clock,
            actor: winner.actor,
            conflicts,
        }
    }
}
fn dominates(a: &Clock, b: &Clock) -> bool {
    b.iter().all(|(id, n)| a.get(id).copied().unwrap_or(0) >= *n) && a != b
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Payload {
    pub version: u32,
    pub cells: BTreeMap<String, Cell>,
}
impl Default for Payload {
    fn default() -> Self {
        Self {
            version: 1,
            cells: BTreeMap::new(),
        }
    }
}
impl Payload {
    pub(crate) fn parse(text: &str) -> Result<Self, CoreError> {
        let mut payload: Self = serde_json::from_str(text).map_err(invalid)?;
        if payload.version != 1 {
            return Err(invalid("Unsupported synced app settings version"));
        }
        let mut cells = BTreeMap::new();
        for (k, c) in payload.cells {
            if !validate(&k, &c.value)? {
                continue;
            }
            for v in c.versions() {
                if !validate(&k, &v.value)?
                    || v.actor.is_empty()
                    || v.clock.get(&v.actor).copied().unwrap_or(0) == 0
                    || v.clock.values().any(|n| *n == 0)
                {
                    return Err(invalid("Invalid settings causal metadata"));
                }
            }
            cells.insert(k, c.merge(&c));
        }
        payload.cells = cells;
        Ok(payload)
    }
    pub(crate) fn document(&self) -> Document {
        Document {
            version: 1,
            values: self.cells.iter().map(|(k, c)| (k.clone(), c.value.clone())).collect(),
        }
    }
    pub(crate) fn json(&self) -> String {
        serde_json::to_string(self).expect("validated JSON")
    }
    pub(crate) fn merge(&mut self, other: &Self) {
        for (k, incoming) in &other.cells {
            let Some(local) = self.cells.get_mut(k) else {
                self.cells.insert(k.clone(), incoming.clone());
                continue;
            };
            *local = local.merge(incoming);
        }
    }
}
fn merge_clocks(a: &Clock, b: &Clock) -> Clock {
    let mut c = a.clone();
    for (k, n) in b {
        c.entry(k.clone()).and_modify(|v| *v = (*v).max(*n)).or_insert(*n);
    }
    c
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SettingsBootstrapChoice {
    UseLocal,
    UseRemote,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SettingsReview {
    pub id: String,
    pub target: String,
    pub remote: Payload,
    pub choice: Option<SettingsBootstrapChoice>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PortableState {
    #[serde(default)]
    pub revision: u64,
    pub version: u32,
    pub payload: Payload,
    pub enabled: bool,
    pub pending: BTreeSet<String>,
    pub initialized: BTreeSet<String>,
    #[serde(default)]
    pub nearby_adopted: BTreeMap<String, Payload>,
    #[serde(default)]
    pub review: Option<SettingsReview>,
    pub last_success_ms: u64,
    pub error: Option<String>,
}
impl Default for PortableState {
    fn default() -> Self {
        Self {
            revision: 0,
            version: 1,
            payload: Payload::default(),
            enabled: false,
            pending: BTreeSet::new(),
            initialized: BTreeSet::new(),
            nearby_adopted: BTreeMap::new(),
            review: None,
            last_success_ms: 0,
            error: None,
        }
    }
}
impl PortableState {
    pub(crate) fn load(dir: &Path) -> Result<Self, CoreError> {
        let bytes = match std::fs::read(dir.join(FILE)) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        let mut state: Self = serde_json::from_slice(&bytes).map_err(invalid)?;
        if state.version != 1 {
            return Err(invalid("Unsupported local app settings version"));
        }
        state.payload = Payload::parse(&state.payload.json())?;
        for receipt in state.nearby_adopted.values_mut() {
            *receipt = Payload::parse(&receipt.json())?;
        }
        if let Some(review) = &mut state.review {
            review.remote = Payload::parse(&review.remote.json())?;
            if review.id.is_empty() || review.target.is_empty() {
                return Err(invalid("Invalid settings review"));
            }
        }
        Ok(state)
    }
    pub(crate) fn bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(invalid)
    }
    pub(crate) fn stage(&mut self, doc: Document, actor: &str) -> bool {
        let mut changed = false;
        for (k, value) in doc.values {
            if self
                .payload
                .cells
                .get(&k)
                .is_some_and(|c| c.value == value && c.conflicts.is_empty())
            {
                continue;
            }
            let mut clock = self.payload.cells.get(&k).map(Cell::context).unwrap_or_default();
            let counter = self
                .payload
                .cells
                .values()
                .flat_map(|c| c.versions())
                .filter_map(|v| v.clock.get(actor).copied())
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            clock.insert(actor.into(), counter);
            self.payload.cells.insert(
                k.clone(),
                Cell {
                    value,
                    clock,
                    actor: actor.into(),
                    conflicts: vec![],
                },
            );
            self.pending.insert(k);
            changed = true;
        }
        changed
    }
    pub(crate) fn status(&self) -> SettingsSyncStatus {
        SettingsSyncStatus {
            revision: self.revision,
            enabled: self.enabled,
            pending_keys: self.pending.len() as u32,
            bootstrap_required: self.review.is_some(),
            last_success_ms: self.last_success_ms,
            error: self.error.clone(),
            remote_settings_json: self.review.as_ref().map(|r| r.remote.document().json()),
            review_id: self.review.as_ref().map(|r| r.id.clone()),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SettingsSyncStatus {
    pub revision: u64,
    pub enabled: bool,
    pub pending_keys: u32,
    pub bootstrap_required: bool,
    pub last_success_ms: u64,
    pub error: Option<String>,
    pub remote_settings_json: Option<String>,
    pub review_id: Option<String>,
}
pub(crate) fn apply_preferences(g: &mut crate::engine::Inner) {
    let v = g.portable.payload.document().values;
    if let Some(s) = v.get("grouping").and_then(Value::as_str) {
        g.prefs.group_by = match s {
            "project" => GroupBy::Project,
            "tag" => GroupBy::Tag,
            "estimate" => GroupBy::Estimate,
            "none" => GroupBy::None,
            _ => GroupBy::MorningNight,
        };
    }
    if let Some(s) = v.get("sort").and_then(Value::as_str) {
        g.prefs.sort = match s {
            "title" => SortKey::Title,
            "due" => SortKey::Due,
            "estimate" => SortKey::Estimate,
            "created" => SortKey::Created,
            _ => SortKey::Manual,
        };
    }
    if let Some(s) = v.get("direction").and_then(Value::as_str) {
        g.prefs.direction = if s == "descending" {
            SortDirection::Descending
        } else {
            SortDirection::Ascending
        };
    }
    if let Some(n) = v.get("upcomingDays").and_then(Value::as_u64) {
        g.prefs.upcoming_days = n as u32;
    }
    if let Some(n) = v.get("autoArchive").and_then(Value::as_bool) {
        g.prefs.auto_archive = n;
    }
}
impl Engine {
    pub(crate) fn edit_portable<T>(
        &self,
        f: impl FnOnce(&mut PortableState, &str) -> Result<T, CoreError>,
    ) -> Result<T, CoreError> {
        let mut g = self.lock();
        let mut candidate = g.portable.clone();
        let result = f(&mut candidate, &g.store.meta.client_id)?;
        if candidate != g.portable {
            let values_changed = candidate.payload.document() != g.portable.payload.document();
            let wake = candidate.payload != g.portable.payload
                || candidate.enabled != g.portable.enabled
                || candidate.review != g.portable.review
                || candidate.nearby_adopted != g.portable.nearby_adopted
                || candidate.error != g.portable.error;
            if values_changed {
                candidate.revision = g.portable.revision.saturating_add(1);
            }
            g.store.save_portable(&candidate.bytes()?)?;
            g.portable = candidate;
            apply_preferences(&mut g);
            if wake {
                g.invalidate();
            }
        }
        Ok(result)
    }
}
#[cfg_attr(feature = "ffi", uniffi::export)]
impl Engine {
    /// A partial versioned document: adapters preserve native values for missing keys.
    pub fn portable_settings_json(&self) -> Result<String, CoreError> {
        Ok(self.lock().portable.payload.document().json())
    }
    /// Validate every known key, durably stage the full edit, then publish it locally.
    pub fn stage_portable_settings(&self, json: String) -> Result<Outcome, CoreError> {
        let doc = Document::parse(&json)?;
        self.edit_portable(|s, id| {
            Ok(if s.stage(doc, id) {
                Outcome::changed()
            } else {
                Outcome::none()
            })
        })
    }
    pub fn settings_sync_status(&self) -> SettingsSyncStatus {
        let mut status = self.lock().portable.status();
        if status.enabled {
            if let Some(error) = self.settings_error.lock().unwrap_or_else(|e| e.into_inner()).clone() {
                status.error = Some(error);
            }
        }
        status
    }
    pub fn set_settings_sync_enabled(&self, enabled: bool) -> Result<SettingsSyncStatus, CoreError> {
        self.edit_portable(|s, _| {
            s.enabled = enabled;
            if !enabled {
                s.review = None;
                s.error = None;
            }
            Ok(s.status())
        })?;
        if !enabled {
            *self.settings_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
        #[cfg(feature = "p2p")]
        {
            let runtime = self.p2p.lock().unwrap_or_else(|e| e.into_inner()).clone();
            if let Some(rt) = runtime {
                if let Err(error) = rt.refresh_settings_channel() {
                    self.settings_failed(error.to_string());
                }
            }
        }
        Ok(self.settings_sync_status())
    }
    pub fn set_settings_bootstrap_choice(
        &self,
        choice: SettingsBootstrapChoice,
        expected_review_id: String,
    ) -> Result<SettingsSyncStatus, CoreError> {
        self.edit_portable(|s, _| {
            let review = s.review.as_mut().filter(|r| r.id == expected_review_id);
            if !s.enabled || review.is_none() {
                return Err(invalid(
                    "Settings changed since this review; sync and review the current choices again",
                ));
            }
            review.unwrap().choice = Some(choice);
            Ok(s.status())
        })
    }
    pub fn import_settings(&self, path: String) -> Result<Outcome, CoreError> {
        self.stage_portable_settings(std::fs::read_to_string(path)?)
    }
    pub fn export_settings(&self, path: String) -> Result<Outcome, CoreError> {
        std::fs::write(path, self.portable_settings_json()?)?;
        Ok(Outcome::changed())
    }
    pub fn import_backup_with_settings(&self, path: String, restore_settings: bool) -> Result<Outcome, CoreError> {
        if !restore_settings {
            return self.import_backup(path);
        }
        let v: Value = serde_json::from_slice(&std::fs::read(path)?).map_err(invalid)?;
        let data = momentum_model::AppData::from_backup(v.clone()).map_err(invalid)?;
        let doc = v
            .get("momentumSettings")
            .map(|v| Document::parse(&v.to_string()))
            .transpose()?;
        #[cfg(feature = "p2p")]
        let mut nearby = self.p2p.lock().unwrap_or_else(|e| e.into_inner());
        #[cfg(feature = "p2p")]
        self.stop_p2p_locked(&mut nearby);
        let mut g = self.lock();
        g.sync_generation = g.sync_generation.wrapping_add(1);
        let mut store = g.store.clone();
        let mut portable = g.portable.clone();
        if let Some(doc) = doc {
            portable.stage(doc, &store.meta.client_id);
        }
        if portable.payload.document() != g.portable.payload.document() {
            portable.revision = g.portable.revision.saturating_add(1);
        }
        store.state = data;
        store.pending.clear();
        store.meta.p2p_restore_epoch = store.meta.p2p_restore_epoch.wrapping_add(1);
        store.meta.p2p_bootstrapped = true;
        store.meta.p2p_applied_ops.clear();
        store.save_with_portable(&portable.bytes()?)?;
        g.store = store;
        g.portable = portable;
        g.undo.clear();
        apply_preferences(&mut g);
        g.invalidate();
        Ok(Outcome::changed())
    }
    pub fn export_backup_with_settings(&self, path: String, include_settings: bool) -> Result<Outcome, CoreError> {
        if !include_settings {
            return self.export_backup(path);
        }
        let g = self.lock();
        let body = json!({"data":g.store.state,"timestamp":momentum_model::now_ms(),"crossModelVersion":4.5,"momentumSettings":g.portable.payload.document()});
        std::fs::write(path, serde_json::to_vec_pretty(&body).map_err(invalid)?)?;
        Ok(Outcome::changed())
    }
}
impl PortableState {
    fn review_for(&self, target: &str, remote: Payload) -> SettingsReview {
        if let Some(review) = self
            .review
            .as_ref()
            .filter(|r| r.target == target && r.remote == remote)
        {
            return review.clone();
        }
        SettingsReview {
            id: momentum_model::new_id(),
            target: target.into(),
            remote,
            choice: None,
        }
    }
    /// First contact requires a concrete choice when both devices hold values.
    pub(crate) fn prepare(
        &self,
        target: &str,
        remote: Option<&Payload>,
        actor: &str,
    ) -> Result<Option<Self>, CoreError> {
        let mut candidate = self.clone();
        // Exact causal equality is a retry of our own already published image, so
        // a failed local commit can recover without inventing a bootstrap conflict.
        let needs_choice = !self.initialized.contains(target)
            && !self.payload.cells.is_empty()
            && remote.is_some_and(|p| !p.cells.is_empty() && p != &self.payload);
        let choice = self
            .review
            .as_ref()
            .filter(|review| review.target == target && Some(&review.remote) == remote)
            .and_then(|r| r.choice);
        if needs_choice && choice.is_none() {
            return Ok(None);
        }
        if let Some(remote) = remote {
            candidate.payload.merge(remote);
            if needs_choice {
                match choice.unwrap() {
                    SettingsBootstrapChoice::UseLocal => {
                        candidate.stage(self.payload.document(), actor);
                    }
                    SettingsBootstrapChoice::UseRemote => {
                        candidate.stage(remote.document(), actor);
                    }
                }
            }
        }
        candidate.initialized.insert(target.into());
        candidate.review = None;
        candidate.error = None;
        Ok(Some(candidate))
    }
}
impl Engine {
    pub(crate) fn settings_failed(&self, message: String) {
        *self.settings_error.lock().unwrap_or_else(|e| e.into_inner()) = Some(message.clone());
        if let Err(error) = self.edit_portable(|s, _| {
            if s.enabled {
                s.error = Some(message);
            }
            Ok(())
        }) {
            log::warn!("could not persist settings sync error: {error}");
        }
    }
    pub(crate) fn commit_settings_exchange(
        &self,
        snapshot: &PortableState,
        candidate: PortableState,
    ) -> Result<(), CoreError> {
        self.edit_portable(|live, _| {
            if !live.enabled {
                return Ok(());
            }
            if snapshot.review.as_ref().is_some_and(|review| review.choice.is_some()) && live.review != snapshot.review
            {
                return Err(invalid(
                    "Settings review changed while syncing; review the current choices again",
                ));
            }
            let mut payload = candidate.payload.clone();
            payload.merge(&live.payload);
            let concurrent: BTreeSet<_> = live
                .pending
                .iter()
                .filter(|k| live.payload.cells.get(*k) != snapshot.payload.cells.get(*k))
                .cloned()
                .collect();
            live.payload = payload;
            live.pending = concurrent;
            live.initialized.extend(candidate.initialized);
            live.review = None;
            live.error = None;
            live.last_success_ms = momentum_model::now_ms();
            Ok(())
        })?;
        *self.settings_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }
    pub(crate) fn exchange_nextcloud_settings(
        &self,
        cfg: &momentum_sync::NextcloudCfg,
        timeout: std::time::Duration,
        current: &dyn Fn() -> bool,
    ) -> Result<(), CoreError> {
        let (snapshot, actor) = {
            let g = self.lock();
            (g.portable.clone(), g.store.meta.client_id.clone())
        };
        if !snapshot.enabled {
            return Ok(());
        }
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        (&cfg.server_url, &cfg.user_name, &cfg.folder).hash(&mut hash);
        let target = format!("nextcloud:{:016x}", hash.finish());
        let mut candidate = None;
        let mut remote_payload = None;
        momentum_sync::exchange_portable_guarded(
            cfg,
            timeout,
            || current() && self.lock().portable.enabled,
            |remote| {
                let parsed = remote
                    .map(Payload::parse)
                    .transpose()
                    .map_err(|e| momentum_sync::SyncError::Parse(e.to_string()))?;
                remote_payload = parsed.clone();
                candidate = snapshot
                    .prepare(&target, parsed.as_ref(), &actor)
                    .map_err(|e| momentum_sync::SyncError::Parse(e.to_string()))?;
                Ok(candidate
                    .as_ref()
                    .and_then(|s| (!s.payload.cells.is_empty()).then(|| s.payload.json())))
            },
        )
        .map_err(|e| CoreError::Transient {
            kind: e.kind().into(),
            message: e.to_string(),
        })?;
        if !current() {
            return Err(CoreError::Transient {
                kind: crate::SyncFailureKind::Cancelled,
                message: "Settings sync was cancelled".into(),
            });
        }
        if let Some(candidate) = candidate {
            self.commit_settings_exchange(&snapshot, candidate)
        } else {
            self.edit_portable(|live, _| {
                if live.enabled {
                    live.review = remote_payload.map(|remote| live.review_for(&target, remote));
                    live.error = None;
                }
                Ok(())
            })?;
            *self.settings_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
            Ok(())
        }
    }
}
#[cfg(feature = "p2p")]
#[derive(Serialize, Deserialize)]
struct NearbyEnvelope {
    version: u32,
    /// Exact durable images adopted from each peer, keyed by opaque peer target.
    accepted: BTreeMap<String, Payload>,
    payload: Payload,
}
#[cfg(feature = "p2p")]
pub(crate) struct NearbySettings {
    pub engine: std::sync::Weak<Engine>,
    pub stopped: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
#[cfg(feature = "p2p")]
fn nearby_target(peer: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    peer.hash(&mut hash);
    format!("libresync:{:016x}", hash.finish())
}
#[cfg(feature = "p2p")]
impl momentum_p2p::SettingsHandler for NearbySettings {
    fn enabled(&self) -> bool {
        !self.stopped.load(std::sync::atomic::Ordering::SeqCst)
            && self.engine.upgrade().is_some_and(|e| e.lock().portable.enabled)
    }
    fn outgoing(&self) -> momentum_p2p::Result<String> {
        let e = self
            .engine
            .upgrade()
            .ok_or_else(|| momentum_p2p::Error::Protocol("settings owner stopped".into()))?;
        let g = e.lock();
        serde_json::to_string(&NearbyEnvelope {
            version: momentum_p2p::SETTINGS_CHANNEL_VERSION,
            accepted: g.portable.nearby_adopted.clone(),
            payload: g.portable.payload.clone(),
        })
        .map_err(|e| momentum_p2p::Error::Protocol(e.to_string()))
    }
    fn receive(&self, peer: &str, json: &str) -> momentum_p2p::Result<()> {
        let e = self
            .engine
            .upgrade()
            .ok_or_else(|| momentum_p2p::Error::Protocol("settings owner stopped".into()))?;
        let envelope: NearbyEnvelope =
            serde_json::from_str(json).map_err(|e| momentum_p2p::Error::Protocol(e.to_string()))?;
        if envelope.version != momentum_p2p::SETTINGS_CHANNEL_VERSION {
            return Err(momentum_p2p::Error::Protocol(
                "unsupported settings channel version".into(),
            ));
        }
        let remote =
            Payload::parse(&envelope.payload.json()).map_err(|e| momentum_p2p::Error::Protocol(e.to_string()))?;
        for receipt in envelope.accepted.values() {
            Payload::parse(&receipt.json()).map_err(|e| momentum_p2p::Error::Protocol(e.to_string()))?;
        }
        let target = nearby_target(peer);
        e.edit_portable(|live, actor| {
            if !live.enabled || self.stopped.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(invalid("settings sync is disabled or its provider stopped"));
            }
            // Runtime identity uses the store actor. Another peer's receipt is
            // irrelevant, even if that earlier exchange was fully accepted.
            let receipt = envelope.accepted.get(&nearby_target(actor));
            if live.initialized.contains(&target) && live.nearby_adopted.contains_key(&target) && receipt.is_none() {
                return Ok(());
            }
            let acknowledged = receipt.is_some_and(|p| {
                live.payload
                    .cells
                    .iter()
                    .all(|(key, cell)| p.cells.get(key) == Some(cell))
            });
            if let Some(candidate) = live.prepare(&target, Some(&remote), actor)? {
                // Persist the receipt together with the adopted image, before
                // the next export can tell this peer its settings were accepted.
                live.nearby_adopted.insert(target.clone(), remote.clone());
                live.payload = candidate.payload;
                live.initialized = candidate.initialized;
                live.review = None;
                live.error = None;
                if let Some(receipt) = receipt {
                    live.pending
                        .retain(|key| live.payload.cells.get(key) != receipt.cells.get(key));
                    if acknowledged {
                        live.last_success_ms = momentum_model::now_ms();
                    }
                }
            } else {
                live.review = Some(live.review_for(&target, remote));
                live.error = None;
            }
            Ok(())
        })
        .map_err(|e| momentum_p2p::Error::Protocol(e.to_string()))?;
        *e.settings_error.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }
    fn failed(&self, error: String) {
        if self.stopped.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        if let Some(e) = self.engine.upgrade() {
            e.settings_failed(error);
        }
    }
}
