# macOS CLI parity and Shortcuts implementation plan

**Goal:** Make every documented `mo` command work consistently on macOS, package the
CLI with the Mac app, and expose useful composable native Shortcuts actions.

**Architecture:** Keep CLI operations in portable Rust and use the existing per-store
Unix socket to coordinate with a running engine. Shortcuts use App Intents and task
entities, backed by the running AppState/shared engine rather than a second writer.

**Constraints:** Preserve the dirty Claude worktree, the shared Linux behavior and
handoff/MVP requirements. No commits or pushes. All verification uses temporary data.

## CLI parity

- [x] Review every documented command and current CLI tests against macOS behavior.
- [x] Add failing regressions for parsing, queries, JSON, forwarding failures and
  platform path/config issues found in the audit.
- [x] Fix portable behavior without writing behind a live app on IPC failure.
- [x] Test closed-app and running-engine paths; preserve Linux compilation.
- [x] Bundle a signed `mo` executable and document installation and data-directory rules.

Owned files: crates/mo, relevant shared IPC tests/code; packaging in macos scripts
and project.yml. Do not introduce unrelated CLI features.

## Shortcuts

- [x] Define stable task entities and queries with useful output properties.
- [x] Expose Create Task, Find Tasks, Set Task Completed, Plan Tasks for Today and
  Open Task. Create accepts title, notes and planning destination; output is a task.
- [x] Use testable MomentumKit adapters for validation and all mutations through
  AppState, preserving refresh, undo and sync behavior.
- [x] Register an AppShortcutsProvider and localized summaries/phrases.
- [x] Verify compiled intent metadata, model tests, native discovery and execution
  where the local Shortcuts application permits it. Report any actual UI blocker.

Owned files: macos/Momentum/Services/CreateTaskIntent.swift, additional App Intents
files as needed, MomentumKit automation adapter/tests, app registration and catalogs.

## Final verification

- [x] Review each workstream against the requirements and review code correctness.
- [x] Run Rust CLI/core tests, Swift tests, Xcode build, localization and signature checks.
- [x] Exercise the bundled CLI against isolated data and a running demo app.
- [x] Document exact commands/actions, supported behavior and any unverified external
  sync or native discovery conditions. Leave a working preview, with saved preferences.

Evidence and platform limitations: `docs/MACOS-AUTOMATION.md`. Overall MVP signoff remains governed by the handoff.
