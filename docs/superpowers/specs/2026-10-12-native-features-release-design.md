# Momentum native fixes and settings release

Date: 2026-10-12. Baseline: remote main `69682872cbf586076719d9942a49ba4f5f05759a`,
version 0.4.9. Target patch: 0.4.10, build 410. This design covers the requested
Linux fixes, iOS sharing and project navigation, recurrence previews, overdue
callouts, copy feedback, settings transfer and optional settings sync on Linux,
macOS, iOS and Android. The user approved the four product decisions interactively:
preserve existing formats, offer an opt-in checked folder migration, use the listed
portable settings scope, and show read-only future recurrence previews.

Implementation belongs in the isolated `task/native-features-release` worktree.
The primary checkout's existing sync work must remain intact. The user's request
authorizes integration into main, a patch bump, a tagged release and a new internal
TestFlight build after the relevant verification gates pass.

## Linux task entry and preferences

The project picker currently binds its display by the presentation list position.
Libadwaita's selected-item display contains one item, so its position is zero even
when another project is selected. Bind the title and color to the actual model
object. Preserve project identity when titles match, and verify the saved task's
project as well as the visible selection.

Give Preferences an explicit preferred content size of approximately 720 by 700
and verify its behavior against small and large parent windows. Preserve adaptive
scrolling and reachability at large text sizes. If the parent constraint still
reproduces the reported defect, use the supported native presentation that permits
an adequately sized desktop window rather than accepting an ineffective size change.

Use a short Add a task placeholder. Put syntax examples and optional keyboard
guidance in a native tooltip and accessible description. Preserve visible
destination context, keyboard entry and existing quick-add parsing.

## New shared tasks default to Today

New iOS shares continue to create one Inbox task through the durable share inbox.
Capture an optional plain Gregorian `plannedDay` when Add is pressed, using the
same instant as `sharedAt`. Import that saved day rather than calculating Today
when the main app eventually drains the inbox. This preserves the intended date
across a midnight delay or timezone change.

Old queued share payloads without the optional field remain unscheduled. Preserve
the existing `share:v1:` source identity and duplicate prevention. Update compose
and import feedback so the destination is clear.

## Project navigation loads after the first selection

Reproduce the valid-project case using controlled snapshot completion and the
actual compact sidebar to detail transition. The existing missing-destination
regression does not demonstrate this report. SwiftUI cancellation during the
transition is a hypothesis that must be tested before selecting a fix.

The active route must receive its completed snapshot, finish loading, and preserve
selection through compact/wide transitions. Rapid route changes must not publish
an old result into the new destination. A removed project must fall back to Today.
Check equivalent first-selection and removal behavior on macOS, Android and Linux;
their different navigation implementations need changes only if a defect is found.

## Future recurring occurrences

Upcoming projects future occurrences inside its existing 7 or 30 day horizon.
The shared Rust core owns date matching and projection, reusing the recurrence
algorithm already exercised by notification planning. Merely viewing Upcoming
must not create tasks, advance the recurrence cursor, enqueue operations, or
schedule extra reminders.

Represent projected occurrences explicitly with their recurrence configuration
and planned day. Show their title, project, schedule and recurrence meaning through
native rows. Offer Edit repeat schedule. A preview has no ordinary task completion,
drag, reorder or delete action because it is not yet a persisted task. Materialized
instances retain all normal actions.

Suppress previews whose deterministic occurrence ID is already live or archived,
including instances that were moved to a different date. Cover daily, weekly,
monthly and yearly schedules, intervals, paused/start rules, month boundaries and
day rollover. Share this contract across every frontend and document the preview
behavior separately from ordinary task listings and automation queries.

## Overdue task callouts

Return an explicit overdue status from Rust using the existing rule: unfinished
tasks whose effective local planned day precedes today. Preserve existing grouping
and order. Show a localized Overdue label, warning symbol and original due date on
each applicable native row. Meaning must remain clear without color. Completed
and archived tasks must not acquire a live overdue warning.

## Copy feedback inside task editors

Show localized Notes copied feedback in the editor that performed the copy.
The existing root overlays are beneath modal editors and are unsuitable for this
acknowledgement. Copy complete multiline and Unicode notes, including Android's
existing large-note handling. Produce success feedback only after copying succeeds.

Use one restrained native haptic where the platform and hardware support it,
respecting iOS's haptics preference and Android system settings. macOS uses the
system haptic performer when available. Standard Linux desktop hardware has no
portable haptic facility; the visible, accessible toast provides feedback there.
Repeated copies refresh the feedback deadline without accumulating overlays.

## Portable settings transfer

Introduce a typed, versioned portable settings document owned by Rust. The initial
allowlist is grouping, task sort and direction, Upcoming range, auto-archive,
row density, colorful labels, completion sound enablement and selected sound.
Native adapters map stable values into GSettings, UserDefaults and DataStore.

Keep credentials, server addresses, folders, provider selection, pairing identity,
notification permissions and enablement, background controls, badge/search
integrations, fonts, accents, haptics, shortcuts, window state and private drafts
device-local. Never serialize a raw preference store.

Include app settings on export and Restore included app settings on import are
independent controls, initially unchecked. Task-only imports preserve settings.
Support settings-only transfer without replacing tasks. Validate known keys and
types before applying anything; unsupported or missing keys preserve local values.
Reject invalid known values or incompatible document versions with actionable
feedback. Preserve the existing task replacement confirmation and recovery rules.
Exercise failures without leaving a partial settings update or activating services.

## Optional settings sync

Sync app settings is a separate device-local switch, off by default. Its state is
never imported or synchronized. Settings use the selected Nextcloud or LibreSync
provider and its existing lifecycle, credentials and encryption configuration;
no additional service or dependency is introduced.

New Momentum portable values, pending settings changes and merge metadata live
outside `AppData` and ordinary task snapshots. Task export and sync must not carry
them when the corresponding settings control is off. Cover the export/import/sync
opt-in matrix, empty-device bootstrap, disable/re-enable and provider switching.

Use a separate versioned settings payload so disabled devices do not publish or
apply settings through ordinary task snapshots. Persist typed per-key changes
durably. Merge causally ordered values and use a deterministic concurrent tie
break so two transports and independently edited keys converge. Do not use a
whole-document replacement or wall-clock-only winner.

On first enablement, if both local and remote choices exist, show the concrete
choice between this device's settings and synced settings before adoption.
Otherwise seed the missing side. Disabling leaves current local settings intact.
Older peers must receive an explicit capability/version response instead of silently
dropping settings operations. Settings sync failures retain pending changes and
show recovery; they must not falsely report success or interrupt offline task use.

## Momentum ownership and compatible formats

Rename the five local foundation crates to `momentum-model`, `momentum-oplog`,
`momentum-store`, `momentum-sync` and `momentum-p2p`, with their consumers, tooling
and all tracked repository documentation, including historical records. Remove
customer branding and compatibility-only
URL aliases; keep `momentum://` entry points. Preserve required upstream attribution
and licenses in disclosure files and required source headers. Updating names and
paths must not remove an attribution obligation identified by NOTICE.

Retain existing task, backup and sync data formats for this patch. Default new
connections to the `momentum` folder on every platform. Preserve existing explicit,
custom and inherited folders before changing defaults so an upgrade does not open
an empty remote or split devices. Narrow legacy compatibility code may be necessary
for that preservation; any legacy-name literal outside legal disclosures must be
limited to that connection migration and expressly accepted as an exception.
Moving an existing remote folder requires a checked copy
and explicit switch, keeping the original recoverable.

Compatibility also preserves opaque legacy state, including an imported
`globalConfig` slice if present. These fields are not Momentum's native preferences
and are never applied as app settings, but they continue to round-trip in legacy
task backup and sync data. The new settings opt-ins govern Momentum's portable
preferences; they do not sanitize existing imported data. If removing legacy
configuration from every payload is required, select the native format migration
and specify preservation of the original separately.

This option removes names and presentation coupling while preserving inherited
formats. It does not claim a new independent storage or wire schema.

## Tests and release gates

Add behavioral regressions before each bug fix. Shared Rust tests cover recurrence,
overdue status, settings validation/convergence, backup recovery and compatibility.
Native tests cover picker display and persistence, first project navigation, share
capture/import dates, visible editor feedback, settings controls and adaptive layout.
Use synthetic data, isolated preferences and disposable stores throughout.

Regenerate UniFFI bindings through the existing tooling. Verify CLI and all four
native consumers, English/German resources, large text, compact layouts, contrast
and reduced motion where affected. Run both Rust workspace commands, Apple package
and hosted iOS lanes, Android JVM/instrumentation and release lint, and the supported
GNOME SDK/Broadway suite on both Linux architectures. Record native interaction
evidence and any real-device or external-server limits separately.

Update FEATURES, BUGS, PROGRESS, relevant specs and CHANGELOG in the implementation
change. Existing IDs include F-003/004/005/006/008/017/018/020/025/026/029/033/057/059.
Allocate new settings and preview capability IDs and new reproduced bug IDs centrally;
the current maximum IDs are F-060 and B-138.

Verify the candidate revision before integrating into main, then verify the release
revision and aligned 0.4.10 metadata before pushing the annotated tag. Observe the
Release workflow and check published assets, provenance and checksums. The existing
workflow includes desktop, CLI and Android artifacts; iOS TestFlight is a separate
signed archive/export/upload using app 6821523171 and the existing internal group.
Check actual app/extension signatures, App Group entitlements, privacy manifests,
version/build, Apple processing and group access before reporting availability.

## Baseline evidence

The unchanged isolated baseline passed 323 Rust tests with zero failures and five
existing opt-in skips on 2026-10-12. Current version copies all report 0.4.9. Apple
bindings were generated from the repository's pinned LibreSync v0.6.0 source.
The compiled-catalog macOS package lane reports 178 tests with its existing opt-in
nearby test skipped; the portable iOS package lane reports 161 tests with its
existing opt-in nearby suite skipped. These checks
used this isolated worktree and temporary test stores, with logs in
`/tmp/momentum-native-features-baseline-{rust,macos,ios}.log`.
These baseline results do not verify the requested implementation.
