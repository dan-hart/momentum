// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Sync providers (LocalFile, WebDAV/Nextcloud, SuperSync) and conflict
//! resolution on top of `sp-oplog` and `sp-store`.
//!
//! Upstream contracts: packages/sync-providers/src/file-based-sync-data.ts,
//! packages/shared-schema/src/supersync-http-contract.ts,
//! docs/sync-and-op-log/supersync-encryption-architecture.md.
