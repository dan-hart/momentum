// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Local persistence: JSON snapshot + pending (unsynced) ops + sync metadata.
use serde::{Deserialize, Serialize};
use sp_model::AppData;
use sp_oplog::{apply, Action, Op, VectorClock};
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
    pub fn load(dir: PathBuf) -> Self {
        std::fs::create_dir_all(&dir).ok();
        let state = read(&dir.join("state.json")).unwrap_or_else(AppData::fresh);
        let pending = read(&dir.join("pending.json")).unwrap_or_default();
        let mut meta: Meta = read(&dir.join("meta.json")).unwrap_or_default();
        if meta.client_id.is_empty() {
            meta.client_id = format!("momentum_{}", &sp_model::new_id()[24..]);
        } // random tail of a v7 uuid
        Self {
            dir,
            state,
            pending,
            meta,
        }
    }
    pub fn save(&self) -> std::io::Result<()> {
        for (name, v) in [
            ("state.json", serde_json::to_vec(&self.state)?),
            ("pending.json", serde_json::to_vec(&self.pending)?),
            ("meta.json", serde_json::to_vec(&self.meta)?),
        ] {
            let tmp = self.dir.join(format!("{name}.tmp"));
            std::fs::write(&tmp, v)?;
            std::fs::rename(tmp, self.dir.join(name))?;
        }
        Ok(())
    }
    /// Apply locally, record as pending for sync, persist.
    pub fn dispatch(&mut self, action: Action) {
        apply(&mut self.state, &action);
        let op = action.to_op(&self.meta.client_id, &mut self.meta.vector_clock);
        self.pending.push(Pending { op, action });
        if let Err(e) = self.save() {
            eprintln!("save failed: {e}");
        }
    }
    /// Replace the snapshot (backup import): pending ops are dropped, they no longer apply.
    pub fn replace_state(&mut self, state: AppData) {
        self.state = state;
        self.pending.clear();
        self.save().ok();
    }
}
