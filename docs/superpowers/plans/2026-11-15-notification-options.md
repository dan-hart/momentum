# Notification options implementation plan

**Goal** Add overdue badging and configurable summaries on all four native apps.

**Architecture** Rust owns date membership, selected sections, preview and queued notification revisions. Native settings own device-local persistence and each OS adapter owns presentation.

- [x] Extend core badge and summary rules with focused tests for old defaults, custom days and selected sections.
- [x] Connect Apple settings and delivery. Reconcile iOS pending notifications when their badge values change.
- [x] Connect Android settings and delivery to the shared core and native channels.
- [x] Connect GTK settings, summary and Background Apps status.
- [x] Regenerate bindings, verify the shared and native paths, and update the ledgers.
