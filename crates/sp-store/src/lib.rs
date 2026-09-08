// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Local persistence: a SQLite op log mirroring upstream's `SUP_OPS`
//! IndexedDB layout (ops, stateCache, vectorClock, clientId, archiveYoung,
//! archiveOld, meta), plus snapshot and compaction.
//!
//! See docs/sync-and-op-log/operation-log-architecture.md upstream.
