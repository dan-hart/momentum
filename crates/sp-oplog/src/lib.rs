// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Operation log: the `Operation` envelope, vector clocks, the compact
//! wire codec, the `pf_` file prefix, gzip and end-to-end encryption, and
//! `apply()` for every upstream action type.
//!
//! Executable upstream contracts to port:
//! - packages/sync-core/src/operation.types.ts
//! - packages/sync-core/src/compact-operation.types.ts
//! - packages/sync-core/src/action-type-codes.ts
//! - docs/sync-and-op-log/vector-clocks.md

use std::collections::BTreeMap;

/// Upstream `MAX_VECTOR_CLOCK_SIZE`.
pub const MAX_VECTOR_CLOCK_SIZE: usize = 20;

/// A vector clock keyed by client id.
pub type VectorClock = BTreeMap<String, u64>;

/// Operation kinds (`opType` in the envelope).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OpType {
    #[serde(rename = "CRT")]
    Create,
    #[serde(rename = "UPD")]
    Update,
    #[serde(rename = "DEL")]
    Delete,
    #[serde(rename = "MOV")]
    Move,
    #[serde(rename = "BATCH")]
    Batch,
    #[serde(rename = "SYNC_IMPORT")]
    SyncImport,
    #[serde(rename = "BACKUP_IMPORT")]
    BackupImport,
    #[serde(rename = "REPAIR")]
    Repair,
}

/// The full (non-compact) operation envelope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub action_type: String,
    pub op_type: OpType,
    pub entity_type: sp_model::EntityType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_ids: Option<Vec<String>>,
    pub payload: serde_json::Value,
    pub client_id: String,
    pub vector_clock: VectorClock,
    pub timestamp: u64,
    pub schema_version: u32,
}
