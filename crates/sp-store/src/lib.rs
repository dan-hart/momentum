// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Local persistence: JSON snapshot + pending (unsynced) ops + sync metadata.
mod durability;
use serde::{Deserialize, Serialize};
use sp_model::AppData;
use sp_oplog::{apply, merge_clocks, Action, Op, VectorClock};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pending {
    pub op: Op,
    pub action: Action,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Meta {
    pub client_id: String,
    pub vector_clock: VectorClock,
    pub last_sync_version: u64,
    pub last_etag: Option<String>,
    /// A peer snapshot was adopted or judged unnecessary; later peer snapshots are ignored.
    #[serde(default)]
    pub p2p_bootstrapped: bool,
    /// Durable boundary for discarding queued transport data after an explicit restore.
    /// Default zero keeps existing stores/journals compatible without a migration.
    #[serde(default)]
    pub p2p_restore_epoch: u64,
    /// Exact receipts for the most recent inbox, independent of the upload queue.
    /// The next receive prunes IDs no longer in the transport's unacknowledged inbox.
    #[serde(default)]
    pub p2p_applied_ops: std::collections::BTreeSet<String>,
    /// Epoch ms of the last successful Nextcloud cycle (0 = never), for the status caption.
    #[serde(default)]
    pub last_nextcloud_ms: u64,
    /// Epoch ms of the last exchange with a nearby device (0 = never).
    #[serde(default)]
    pub last_nearby_ms: u64,
    /// The day the morning summary notification was last shown.
    #[serde(default)]
    pub last_summary_day: String,
}

#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
    pub state: AppData,
    pub pending: Vec<Pending>,
    pub meta: Meta,
}

fn read<T: for<'a> Deserialize<'a>>(p: &PathBuf) -> Option<T> {
    serde_json::from_slice(&std::fs::read(p).ok()?).ok()
}

impl Store {
    /// Compatibility entry point. Recovery failure must never silently become a fresh
    /// writable store. New application boundaries should surface `try_load` errors.
    pub fn load(dir: PathBuf) -> Self {
        Self::try_load(dir).expect("store recovery failed; original files have been preserved")
    }
    pub fn try_load(dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&dir)?;
        durability::recover(&dir)?;
        durability::cleanup(&dir);
        let state = read(&dir.join("state.json")).unwrap_or_else(AppData::fresh);
        let pending = read(&dir.join("pending.json")).unwrap_or_default();
        let mut meta: Meta = read(&dir.join("meta.json")).unwrap_or_default();
        if meta.client_id.is_empty() {
            meta.client_id = format!("momentum_{}", &sp_model::new_id()[24..]);
        } // random tail of a v7 uuid
        Ok(Self {
            dir,
            state,
            pending,
            meta,
        })
    }
    pub fn save(&self) -> std::io::Result<()> {
        durability::save(
            &self.dir,
            [
                serde_json::to_vec(&self.state)?,
                serde_json::to_vec(&self.pending)?,
                serde_json::to_vec(&self.meta)?,
            ],
        )
    }
    /// Apply locally, record as pending for sync, persist.
    pub fn dispatch(&mut self, action: Action) {
        self.dispatch_batch([action]);
    }
    /// Stage one local operation in memory. The owner must commit with `save` before
    /// publishing this candidate; vector-clock and pending-op rules stay centralized.
    pub fn stage(&mut self, action: Action) {
        apply(&mut self.state, &action);
        let op = action.to_op(&self.meta.client_id, &mut self.meta.vector_clock);
        self.pending.push(Pending { op, action });
    }
    /// Apply an ordered logical batch and persist its complete image once.
    pub fn dispatch_batch(&mut self, actions: impl IntoIterator<Item = Action>) {
        for action in actions {
            self.stage(action);
        }
        if let Err(e) = self.save() {
            eprintln!("save failed: {e}");
        }
    }
    pub fn dir(&self) -> &std::path::Path {
        &self.dir
    }
    /// Applies an op that arrived from a peer. Ops from other clients are also queued for
    /// the Nextcloud upload so a device without Nextcloud still reaches the server through
    /// one that has it. Returns false when the op was already applied here.
    pub fn apply_remote(&mut self, op: Op, action: Action) -> bool {
        if op.c == self.meta.client_id || self.pending.iter().any(|p| p.op.id == op.id) {
            return false;
        }
        apply(&mut self.state, &action);
        self.meta.vector_clock = merge_clocks(&self.meta.vector_clock, &op.v);
        self.pending.push(Pending { op, action });
        true
    }
    /// Takes a peer's snapshot on board. A store with no tasks and nothing pending adopts it
    /// whole; otherwise entities missing here are added without generating ops (their owner
    /// already syncs them). Returns how many entities were added.
    pub fn adopt_snapshot(&mut self, snap: AppData) -> usize {
        let added = self.stage_snapshot(snap);
        self.save().ok();
        added
    }
    /// Merge a peer snapshot into a private candidate; the owner commits the entire
    /// received batch, including operations and metadata, with one `save`.
    pub fn stage_snapshot(&mut self, snap: AppData) -> usize {
        self.meta.p2p_bootstrapped = true;
        if self.state.task.ids.is_empty() && self.pending.is_empty() {
            self.state = snap;
            return usize::MAX;
        }
        let mut n = 0;
        let projects: Vec<_> = snap
            .project
            .iter()
            .filter(|p| !self.state.project.entities.contains_key(&p.id))
            .cloned()
            .collect();
        for project in projects {
            apply(&mut self.state, &Action::AddProject { project });
            n += 1;
        }
        let tags: Vec<_> = snap
            .tag
            .iter()
            .filter(|t| !self.state.tag.entities.contains_key(&t.id))
            .cloned()
            .collect();
        for tag in tags {
            apply(&mut self.state, &Action::AddTag { tag });
            n += 1;
        }
        for c in snap.task_repeat_cfg.iter() {
            if !self.state.task_repeat_cfg.entities.contains_key(&c.id) {
                self.state.task_repeat_cfg.insert(&c.id.clone(), c.clone());
                n += 1;
            }
        }
        let missing: Vec<_> = snap
            .task
            .iter()
            .filter(|t| !self.state.task.entities.contains_key(&t.id))
            .cloned()
            .collect();
        for t in missing.iter().filter(|t| t.parent_id.is_none()) {
            let mut parent = t.clone();
            parent.sub_task_ids.clear(); // re-linked below by AddSubTask, exactly once
            apply(
                &mut self.state,
                &Action::AddTask {
                    task: parent,
                    bottom: true,
                },
            );
            n += 1;
        }
        for t in missing.iter().filter(|t| t.parent_id.is_some()) {
            apply(
                &mut self.state,
                &Action::AddSubTask {
                    task: t.clone(),
                    parent_id: t.parent_id.clone().unwrap_or_default(),
                },
            );
            n += 1;
        }
        n
    }
    /// Replace the snapshot (backup import): pending ops are dropped, they no longer apply.
    pub fn replace_state(&mut self, state: AppData) -> std::io::Result<()> {
        let mut replacement = self.clone();
        replacement.state = state;
        replacement.pending.clear();
        replacement.meta.p2p_restore_epoch = replacement.meta.p2p_restore_epoch.wrapping_add(1);
        // A restored backup is authoritative locally; a later bootstrap snapshot
        // must not union back entities that the user deliberately replaced.
        replacement.meta.p2p_bootstrapped = true;
        replacement.meta.p2p_applied_ops.clear();
        replacement.save()?;
        *self = replacement;
        Ok(())
    }
}

#[cfg(test)]
mod feature_tests;
