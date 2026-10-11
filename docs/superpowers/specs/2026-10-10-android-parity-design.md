# Native Android parity

The user requested autonomous implementation of a Kotlin/Jetpack Compose Android
client through iOS parity, using current Android conventions and compatible with
F-Droid. Work is local on `task/android-native`, isolated from the dirty primary
checkout. This request approves the native Android scaffolding and necessary FLOSS
AndroidX/Kotlin/UniFFI/JNA dependencies; no service, migration or publication is added.

## Architecture

Reuse `momentum-core` through generated UniFFI Kotlin bindings. Rust remains the
authority for task membership, editing transactions, ordering, undo, recurrence,
notification plans, persistence and sync. Android owns localized presentation,
navigation, permission requests, lifecycle, secure credential storage and OS services.
One application-scoped engine is accessed off the main thread. A repository and
ViewModel expose immutable StateFlow snapshots to lifecycle-aware Compose collectors.
No AppKit/GTK dependency, second task database, WebView, telemetry or account.

Material 3 uses native controls, adaptive bottom navigation/rail, system light/dark
appearance, optional dynamic color, edge-to-edge insets, scalable text, 48dp targets
and semantic labels. The destinations are Today, Upcoming, Search and Settings.
Projects, tags, Morning, Evening and Archive are list contexts. Drafts survive
configuration changes and failed saves. Archived tasks remain read-only. Undo is
attached to the precise Rust batch. Destructive organization/restore actions need
concrete scope confirmation; ordinary task operations use undo.

## Acceptance and implementation order

1. Source-built Android Rust libraries and generated bindings; pinned stable build
   tooling and dependencies, English/German resources, adaptive launcher icon.
2. Offline startup/recovery, lists/search, complete/reopen, full editor (notes, tags,
   project, date/time, estimate), subtasks, duplication, selection/bulk actions,
   archive/undo, project/tag management, grouping/sorting and recurrence.
3. Storage Access Framework export/confirmed restore, share-to-task and deep links.
4. Optional Nextcloud/LibreSync with Android Keystore secrets, provider exclusion,
   bounded foreground/background work, persistent truthful errors and retry.
5. Opt-in native reminders and actions, permission/boot/time-change handling and
   bounded scheduling; platform equivalents for automation and system search.
6. Behavioral tests and emulator acceptance using disposable app stores, native
   layout checks, release/lint/build verification and explicit parity gap records.

F-Droid compatibility means FLOSS dependencies/toolchain, no proprietary Google
services, source-built native code, pinned inputs, metadata/recipe and documented
build commands. Admission and bit-for-bit reproducibility require separate F-Droid
build verification; do not claim either from a local APK. Preserve existing licensing,
including LibreSync's AGPL notice. Android should support API 26+ and target the latest
stable Android SDK available; do not require a preview SDK to build the release.
The user also wants eventual Google Play and alternative-store distribution. Produce
standard APK and Android App Bundle targets from the same app; keep signing outside
source and avoid coupling ordinary functionality to a store or proprietary service.

Apple Reminders import is intentionally Apple-specific; no equivalent Android service
is added. LibreSync physical-device discovery and any explicitly deferred desktop/iOS
checks stay separate. Update FEATURES, BUGS, PROGRESS and the Android checklist with
actual evidence at this dirty worktree. No commits, pushes, PRs, tags or releases.

## Native boundary requirements from review

Network I/O runs outside the local edit queue. Cancel and drain an active exchange
before provider changes or restore. Only selected foreground LibreSync runs; no
permanent background service. WorkManager performs discretionary bounded Nextcloud
work and notification-plan replenishment. AlarmManager schedules reminder times;
the UI states that delivery can be delayed if exact-alarm access is unavailable.
Notification permission is requested only after an explicit opt-in. Reboot and
clock/time-zone changes reconcile schedules against the core's durable revision
ledger; cold Done/Snooze actions use the same process-owned engine. Never claim OS
delivery from scheduling acceptance. Disable Android cloud backup so tasks, secrets
and transport identities are not exposed through a new default backup path.

Verify APK zip and ELF 16KiB alignment for Rust and JNA native libraries, then test
a 16KiB runtime separately. Pin LibreSync source provenance for F-Droid. The Android
parity checklist accounts for every iOS capability and F-ID, including remaining
native reorder/drag, capture, summary/badge, appearance, automation/search and recovery
work, rather than treating the existence of a screen as acceptance.
