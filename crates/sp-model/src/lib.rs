// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Data model for the Super Productivity file and sync formats.
//!
//! Field names and shapes intentionally mirror upstream
//! (`packages/shared-schema` and the `*.model.ts` files in
//! super-productivity v18), so that a backup JSON or a sync snapshot
//! round-trips without loss. See PLAN.md sections 1 and 4.

/// Upstream `CURRENT_SCHEMA_VERSION` (packages/shared-schema/src/schema-version.ts).
pub const SCHEMA_VERSION: u32 = 4;
/// Upstream `MIN_SUPPORTED_SCHEMA_VERSION`.
pub const MIN_SUPPORTED_SCHEMA_VERSION: u32 = 1;
/// Upstream `CROSS_MODEL_VERSION` (src/app/op-log/model/model-config.ts).
pub const CROSS_MODEL_VERSION: f32 = 4.5;

/// Entity types as they appear in the `entityType` field of an operation
/// (packages/shared-schema/src/entity-types.ts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntityType {
    Task,
    Project,
    Tag,
    Note,
    GlobalConfig,
    SimpleCounter,
    WorkContext,
    TimeTracking,
    TaskRepeatCfg,
    IssueProvider,
    Planner,
    MenuTree,
    Metric,
    Board,
    Section,
    Reminder,
    PluginUserData,
    PluginMetadata,
    Migration,
    Recovery,
    All,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_type_serializes_like_upstream() {
        assert_eq!(
            serde_json::to_string(&EntityType::TaskRepeatCfg).unwrap(),
            "\"TASK_REPEAT_CFG\""
        );
    }
}
