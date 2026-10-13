<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Android parity acceptance

Updated: 2026-10-11. Android is integrated with main `7c583a6` (0.4.9) on
`task/android-native` for the user-authorized local main merge and release-process
work. Earlier gate11h evidence below describes its recorded 0.4.5 artifacts;
post-integration results are recorded separately in PROGRESS. This is an acceptance
inventory, not a claim of complete Android parity. Kotlin/Compose, shared Rust,
F-Droid compatibility and eventual APK/AAB distribution remain approved. The current
request authorizes committing/merging locally and preparing future release automation;
it does not authorize a push, tag, actual release or store submission.

The [approved design](superpowers/specs/2026-10-10-android-parity-design.md),
[feature ledger](FEATURES.md), and [iOS checklist](IOS-PARITY-CHECKLIST.md) define the
scope. Android uses generated UniFFI bindings to the shared Rust engine. Kotlin owns
native presentation and OS services; task rules, transactions, import parsing,
recurrence, notification source validation and sync remain in Rust. `Implemented`
means code exists with acceptance outstanding; `In progress` includes incomplete
capabilities. A missing capability remains Planned or In progress. Native Android
equivalents may differ from Apple screens and services, with the difference recorded.

The native compatibility blocker B-122 is addressed in source by rebuilding
the matching AndroidX Graphics Path 1.1.0 JNI from pinned unchanged Apache inputs
with both NDK r27 page-size flags. Local standalone ABI alignment and reproducibility
checks pass. Gate11d release and bundle-generated APK packaging pass strict checks for all eight bundled
libraries, and native iterator/Canvas tests pass on API26 and API37 with 16 KiB pages.
The final source checkpoint, minified bundle-product runtime and independent
Linux/F-Droid build have separate evidence and limits below. See [source/build details](../android/vendor/GRAPHICS-PATH.md).

## Foundation, task views and editing

| Feature | Android status | Capability and native difference | Remaining acceptance |
|---|---|---|---|
| F-001 | Implemented | Application-owned Rust engine; repository/StateFlow/ViewModel; generated Kotlin bindings; Material 3 Compose, adaptive navigation and private storage. No desktop toolkit or shared UI. B-105 corrects generated error payload names. | 91 JVM tests, fresh debug/minified APK/AAB, generated bindings and both ABI packaging checks pass. Complete native/consumer gates and remaining lifecycle scopes are recorded below; independent Linux Android build remains open. |
| F-002 | Implemented | Account-free offline startup, durable core store, checked recovery and private editor drafts. Sync defaults Off; Android cloud backup and device transfer exclude tasks and secrets. | Native editor/private-draft reconstruction, recovery and isolated store reopen pass. Broader interrupted real-process writes, invalid-store running-app recovery and physical durability remain open. |
| F-003 | Implemented | Today/overdue, Morning/Evening, Upcoming 7/30 days; source-aware navigation and stale-destination fallback. | Controlled local-day/recurrence and source-context regressions pass. Real OS date rollover, broader ordering/grouping and restored compact/expanded navigation lifecycle remain open. |
| F-004 | Implemented | Leading list drawer; native project/tag create, rename, color, delete and task counts. Inbox protected; project deletion shows scope; tag deletion preserves tasks. | Full parent/subtask family and two-row selection flows, case-insensitive duplicate tags, exact deletion scope and persistence; accessible spoken selected values. |
| F-005 | Implemented | Inline native Quick Add uses Rust title/tag/estimate parsing and tag-completion chips; Add also opens the full editor. Unlike current iOS, Android retains this direct-entry control. | Native parsed tags/estimate/context and three Quick Add boundary/cursor/failure cases pass. Physical IME submission, broader repeated-submit/failure interaction and spoken completion-chip acceptance remain open. |
| F-006 | In progress | Full title/project/date/time/estimate/tags/notes editor; subtasks, duplication, Copy Title and Copy Notes. Draft file holds full notes while SavedState holds a small identity. Save-before-Repeat/Duplicate, retained failed-save drafts and Save as New protect edits. B-106/B-107 track source context and large notes. | Native large-source, private draft/VM reconstruction, failure/retry, stale-task recovery and supported InputConnection/Undo preservation pass (B-128/B-129). Existing private-draft bounds prevent safely editing oversized imported tasks and boundary legacy records; complete source is retained, pending an approved format design (B-133). Streaming I/O addresses the minimum-runtime memory gap (B-134). Physical third-party keyboards, configuration/process lifecycle and broader full-editor device round trips remain open. |
| F-007 | Implemented | Native date/time controls, reminder choices, AlarmManager and notification channels; explicit opt-in, notification/exact-alarm access and settings recovery. Inexact delivery is explained. Boot/time/time-zone changes reconcile durable core sources; cold Done/Snooze/body routing uses the application engine. B-108 now validates and applies actions in one Rust transaction. | Complete shared-core/generated-consumer and scoped stale/restore/locked-policy native gates pass. Real API26 AlarmManager cold-process delivery was observed; its Done action was warm. Physical exact/inexact timing, denial/regrant, reboot/time-zone, real keyguard and scheduler-originated execution remain open. |
| F-008 | Implemented | Daily/weekly/monthly/yearly recurrence, interval/start/paused drafts; same-day/last-day/nth-weekday monthly modes and Stop use Rust rules. Native start dates/ordinals use localized presentation, weekdays follow locale ordering with full spoken names, and invalid selection announces an error and requests keyboard focus. | All cycles/modes, start-date selection, empty-weekday keyboard-focus recovery, catch-up and pause/reopen/stop persistence; native English/German larger-text acceptance. |
| F-009 | Implemented | Complete/reopen, Archive Completed, auto-archive; read-only Archive/search results; exact-batch Undo restores families and paging. | Complete/reopen and both archive settings, child-only matches, 100-row paging, batch/family restore and restart. Archive must expose no edit/completion/drag action. |
| F-010 | Implemented | List-scoped touch selection, Select All/Done Selecting, bulk complete/delete, Today/Morning/Evening/Tomorrow/next-week, project and existing/new tag actions. Archive excluded. | Every bulk operation and single Undo, family moves, selection clearing, layout above native navigation/IME at large text, and accessible alternatives. |

## Transfer, search and native presentation

| Feature | Android status | Capability and native difference | Remaining acceptance |
|---|---|---|---|
| F-011 | Implemented | Native single/multi-task project drag; private in-app payload and visible destinations; Move action is the accessible alternative. | Two-task touch project move/reorder and exact batch Undo pass on API26/API37. Physical pointer, broader family/invalid-payload and grouped-destination gestures remain open. |
| F-012 | Implemented | Native reorder routes through Rust; selected relative order and exact batch preserved. Move Up/Down is available without dragging. | Real two-task reorder/one-step Undo and shared core ordering/boundary regressions pass. Physical single/pointer/assistive activation, hidden-neighbor and broader grouped/descending/manual gesture acceptance remain open. |
| F-013 | Implemented | Native tag/Today/Morning/Evening destinations; archived source/destination drag excluded. | Tag/day/slot gestures and Undo; stale destinations and archive rejection; accessible actions independently exercised. |
| F-014 | Implemented | External plain text/HTTP URL drop uses the shared import parser and captured destination. Bounded payloads are decoded without fetching links or arbitrary content-provider data. | Actual cross-app text, multiline and URL gestures, source project/tag/day context, invalid/oversized payload behavior, and no unintended outbound private task data. |
| F-015 | Implemented | View Options opens a private full-text import draft; user-initiated native paste/replacement feeds the shared multiline/URL parser into the captured destination. Draft restoration, busy double-submit and failed-import retention are wired. Share-to-task keeps complete source text in notes when its title preview is shortened. | Native explicit owned-clipboard paste, selected-source admission/private reconstruction and supported InputConnection composition pass. API26/27 reject large URI clipboard copies with an explicit full-notes Sharesheet alternative. Actual cross-app drop and third-party keyboard acceptance remain open. |
| F-016 | Implemented | Debounced cancellable Rust search covers every-word title/notes/project/tag, live/archived subtasks and caps; results show source metadata and archive stays read-only. | Multi-word/notes/child matches, rapid stale queries, truncation notices, clear prompt, project/tag activation, cross-tab refresh and long result rendering. |
| F-017 | Implemented | Root Snackbar with exact-batch Undo plus persistent Undo access; persistent failed-save feedback offers retry/details/dismiss. Material semantics announce error surfaces. | Repeated failures, retry committing once, no expired/wrong-batch Undo, screen-reader focus/announcements and dismiss timing in a running app. |
| F-018 | Implemented | Native empty/loading/error states, edge-to-edge/IME insets, bottom navigation or rail and leading contexts. | Small phone, tablet, landscape, keyboard, long German labels and largest supported text; selection, Add, retry and Save must remain reachable. |
| F-019 | Implemented | System/light/dark appearance, separate optional Android wallpaper color and a persisted native ten-choice accent palette matching the iOS seeds. Opaque Material color roles adapt tone and contrasting ink. Saved colorful-label opt-out controls project/tag metadata, headings, result labels and native list icons; API 36+ high-contrast text disables custom label ink without changing the saved choice. Names and selection marks/traits retain meaning without color. | Native palette/accent persistence, long German metadata/selection and colorful-label opt-out tests pass. Live OS high-contrast listener, reduced animation/system changes and broader physical visual checks remain open. Android does not reproduce Liquid Glass or SF Symbols. |
| F-020 | In progress | Native control semantics, headings, labeled actions and Material touch targets; selection/drag have touch alternatives. | TalkBack spoken output/focus, keyboard/Switch Access, independent completion/open targets, parent context for subtasks, error focus, contrast and large-text geometry. Source semantics alone are not acceptance. |
| F-021 | Implemented | English/German quantity plurals for selection, reminder intervals, project deletion, pending sync, summary and indexed titles; observable per-app locale for date/weekday/feedback/clock/index/sync wording. Section counts, subtask parent context and Android 12/24-hour preference are retained. | Native per-app EN/DE resource/quantity recomposition and actual 12/24-hour picker tests pass on their supported scopes. Longer-label/recurrence/error wording and spoken physical acceptance remain open. |
| F-022 | Implemented | Native System/Sans/Serif/Monospace built-in families; independently persisted 75–200% content/interface scales, native labeled controls and previews, and reset to System/100%/100%. Scales adjust sp styles before Android conversion; LocalDensity and OS non-linear font scaling are preserved. Task titles/metadata/notes/import source use content styles; labels/actions use interface styles. Root/editor/import toolbars grow from the actual converted title line height; one-line visual ellipsis retains complete title semantics and localized icon actions keep 48dp minimums. | Five JVM typography and native scale/reset/toolbar/48dp tests pass. German compact phone/tablet combined app/system scaling has recorded native geometry evidence; physical landscape/keyboard/TalkBack/font-fallback and broader above-200% visual acceptance remain open. |
| F-023 | Implemented | One native registry supplies global capture/project/search/tab/settings/sync/help, selection/open/complete/reopen/duplicate/copy/delete/archive/Undo, Today/slot/date/project/repeat/reorder commands, editor Save/Escape, native system help and in-app help. Ctrl/Alt/Meta is persisted. Native text editing, archive mutation exclusions, lock/busy/loading/restore/modal guards and duplicate event consumption remain explicit. | Nine resolver JVM and seven real key-event regressions pass; native text Undo has six supported InputConnection/key cases under B-129. Physical keyboard, discovery/help and tablet/accessibility acceptance remain open. |
| F-024 | Implemented | Launcher Add/Search shortcuts, ACTION_SEARCH, current-identity routing and opt-in framework AppSearch on API 31+. Only unfinished titles and opaque open URLs enter the OS index after explicit consent; stale records, opt-out and restore clear publication. Existing drafts survive result activation. Desktop window/background commands are intentionally desktop-specific. | Current framework/routing/index-clear and active-draft preservation tests pass; API26 indexing is unavailable by design. Actual launcher result display/tap, cold delivery and real permission/visibility transitions remain separate. |

## Sync, backups and OS integrations

| Feature | Android status | Capability and native difference | Remaining acceptance |
|---|---|---|---|
| F-025 | Implemented | Optional HTTPS Nextcloud, retained Keystore credentials, bounded foreground exchanges and discretionary WorkManager background sync. SDK 37 LAN permission also covers private-network servers. | Disposable transport fixture after latest changes, missing folder/damaged copy/repair, cancellation/provider switch/restore, last-success truthfulness; hosted TLS and actual scheduler wakes remain separate. |
| F-026 | Implemented | Selected-provider-only foreground LibreSync, six-digit pairing, manual address, device unlink and durable identities; native multicast lock and LAN permission. API26/API37 gate8b opt-in two-emulator production adapter/Keystore run verifies actual pairing, both sides' offline edit/addition/completion/deletion convergence and durable task/identity/credential reopen over an explicit NAT bridge. | Actual native pairing/encrypted convergence/reopen pass through the controlled two-emulator NAT bridge. Native multicast discovery, symmetric NAT routing, physical devices and real permission denial/regrant/unlink/retry remain open; no permanent foreground service. |
| F-027 | Implemented | Off/Nextcloud/LibreSync selection preserves connections and tasks; active native exchanges cancel and drain before replacement/restore. No account or service requirement. | Rapid provider switching, offline edits, foreground/background overlap, actual native transport cancellation and retained credentials. |
| F-028 | Implemented | Compact localized footer/settings status; selected provider, progress, last success, persistent typed failure, Retry and confirmed Replace Server Copy. | Empty/populated screen status, failed exchange never reporting success, actionable recovery, progress/attempt timing, spoken delivery and background-to-foreground readback. |
| F-029 | Implemented | Storage Access Framework creates/opens backup documents; explicit replacement confirmation, private temporary input, transport/reminder pause and drain before Rust restore. Cloud backup/transfer disabled. | Actual minified release uses Downloads CreateDocument/OpenDocument, explicit replacement and cold relaunch with exact Unicode notes retained. Native invalid/cancel/latest-request/restore-barrier tests pass. Cloud document providers and broader failed-transport/read/replacement acceptance remain open. |
| F-030 | Not applicable | `mo` is a desktop CLI and live desktop-process coordination contract. Android exposes native entry points through F-024/F-031/F-040. | Shared Rust changes still require CLI consumer checks. |
| F-031 | In progress | Launcher shortcuts and URL add/open/complete entry points are present. Equivalent native Find Tasks, Complete/Reopen Task and Plan for Today automation with structured results is incomplete. | Native action discovery, parameters/results, ambiguous-title handling, authentication/locked policy, warm/cold activity delivery and editor-conflict routing. No proprietary assistant dependency. |
| F-032 | Implemented | Off (default), due-or-scheduled today, and Today including overdue use shared Rust counts. Independent opt-in posts a quiet, private count notification with generic public content and no task titles/notes. Supported launchers may show a dot or touch-and-hold count; a guaranteed independent numeric icon badge is not the Android contract. Native edits, exchanges, foreground/day checks and bounded local work refresh the projection; dismissal persists within a process until count/mode/day/locale changes. See [Android badge guidance](https://developer.android.com/develop/ui/views/notifications/badges). | Three actual quiet/private count NotificationManager cases pass on API26/API37, including Off/zero/channel/restore and owner/drain policy. Actual launcher display, real permission regrant and scheduler-originated updates remain open. |
| F-038 | Implemented | Opt-in local morning summary and native time picker, default Off; shared bounded source plan, private/public notification content, summary opens Today. | Time-edit atomicity/persistence, locale and 12/24-hour display, future scheduling, real delivery, cold route, reboot/time-zone change and permission recovery. |
| F-040 | Implemented | `momentum:` add/createTask, completeTask/openTask aliases; Rust title/tag/estimate parsing, notes, ISO due day and tags create a draft, preserving an existing editor through queued capture. Native share and PROCESS_TEXT retain the source text. | Case/percent encoding, invalid input and caps, explicit due-day behavior, ambiguous-title completion, pending-editor conflict, PROCESS_TEXT and warm/cold invocation. Do not infer relative dates from the ISO due parameter. |
| F-041 | Implemented | Power Saver pauses discretionary automatic exchanges and nearby discovery. Manual Nextcloud sync and local task editing remain available; LibreSync requires the permitted foreground transport. WorkManager execution remains OS-discretionary. | Actual power-saver enter/leave, lifecycle/restart, selected-provider admission, background cadence and CPU/idle observation. Explicit reminder delivery follows Android alarm policy. |
| F-043 | In progress | Read-only Nextcloud Test Connection and localized recovery are implemented. LibreSync explicitly offers Sync Now. A read-only LibreSync probe is Blocked on a shared protocol/API contract: current iOS Test Connection invokes an outbound sync cycle and is not a read-only probe (B-109). | Verify bounded/cancellable Nextcloud probe without task/upload/status mutation; propose pinned LibreSync protocol work before adding a truthful read-only probe. Preserve the distinction between checking a connection and exchanging task data. |
| F-049 | Implemented | Saved automatic/background sync preferences; bounded periodic Nextcloud worker runs only for selected/configured/permitted policy. LibreSync stays foreground-only under Android lifecycle limits. | Disabled/default readback, automatic-Off exclusion, worker stop/retry/time budget, process recreation and OS-scheduled run. Mobile background execution is not desktop windowless execution. |
| F-050 | Implemented | Material pull-to-refresh updates the Rust snapshot and requests selected-provider sync; toolbar Refresh remains accessible. | Actual gesture on short/empty/long lists, provider Off, error/retry, overlapping refresh and cancellation; accessibility alternative and truthful indicator. |
| F-054 | Implemented | Android reuses shared upload-integrity checks, typed damaged-copy handling and exchange budget; native cancellation token drains work on policy changes. | Fresh shared and native cut-off upload/stalled request/failure/repair tests, bounded worker behavior, actual server fixture and existing consumers after core edits. |


## New main capabilities and release integration

| Feature | Android status | Capability and native difference | Remaining acceptance |
|---|---|---|---|
| F-056 | Planned | Main added an off-by-default completion sound on Linux/Apple. Android has no completion-sound setting/player yet. | Implement a native resource/player adapter and localized setting reusing core completion messages; verify Off, complete/reopen/Undo and lifecycle. |
| F-057 | In progress | Android accepts text/plain share and PROCESS_TEXT into a retained, user-confirmed capture draft. Main's newer Rust sharedTaskDraft and Inbox-once share-inbox contract is available through regenerated bindings but not wired to Android. | Preserve existing draft/context behavior. Decide/adapt the explicit shared-link/title normalization and Inbox import contract, then test cross-app cold/warm sharing and once-only persistence. |
| F-058 | Implemented | Main's faster sync and failure-retaining Rust changes are integrated; Android uses the same generated core with Keystore, selected-provider lifecycle, typed status and retry. | Post-integration full API26/API37 native lanes pass. Controlled core/native results do not establish physical/server/background Android performance. |
| F-059 | Not applicable | Apple TestFlight distributes Apple apps; Android uses APK/AAB and independent store/repository channels under F-060. | No Android TestFlight capability is appropriate. |
| F-060 | Implemented | Exact-tag Android APK/AAB/source release job, optional explicit distribution signing, unsigned labels, metadata/signature/notices/alignment gates and all-asset checksums. No Play/F-Droid submission. | Local clean-source packaging, checksums, archived offline Cargo resolution and actual packaged minified API26/API37 runtime pass. First hosted workflow, approved signing identity/upgrade continuity, physical release QA and independent F-Droid builder/admission remain. |

## List options and explicit platform differences

| Feature | Android status | Capability and native difference | Remaining acceptance |
|---|---|---|---|
| F-037 | Implemented | Shared exclusive Morning & Evening/project/first-tag/estimate/none grouping, native View Options, manual/title/due/estimate/created sort and direction persistence. | Every grouping/sort, saved colors, completed separation, Today-only slots, hidden neighbors and restore. Duplicate historical ledger rows share this canonical acceptance entry. |
| F-039 | Not applicable | The row specifically describes the Apple SwiftUI testing toolchain. Android has JVM/shared-native and Compose instrumentation lanes under F-001 and [TESTING.md](TESTING.md); no SwiftUI tooling is added. | Android native tests and manual acceptance remain required independently. |
| F-042 | Implemented | Static Material system-surface initial task placeholders with one localized polite loading status, no fake controls and no animation. Existing root navigation and Add remain in place as the first snapshot arrives. | Fresh controlled loading→ready/Nav/Add-bounds test is pending; native rapid startup, compact/tablet/German/large-text geometry and accessibility inspection remain open. Static presentation is independent of motion/Power Saver settings. |
| F-044 | Implemented | Android calls lazy ordinary Rust snapshots; global search/archive indexing is demand-driven. Compose lists are lazy. | Native complete 1,000-task projection, exact identity/title/section/Inbox-family/core counts, scroll/edit/reopen pass. Separate opt-in complete projection observations are 23.997 ms (API26) and 17.170 ms (API37). Test-driven Compose scroll/open timings do not prove physical scrolling smoothness or shipping cold launch; larger stores/sync/search/battery/device performance remain open. |
| F-045 | Implemented | Upcoming groups by day and disables Group By; persistent 7/30-day range and due-day metadata. | Both ranges, date rollover, completed policy, no Morning/Evening grouping and saved preference readback. |
| F-046 | Not applicable | AppKit user-customizable toolbar is a macOS integration. Android uses adaptive native top/bottom/rail actions. | Verify Android actions remain accessible at supported widths under F-018/F-020. |
| F-047 | Implemented | Selection-mode native drag transports the whole ordered selection, equivalent to iOS multi-transfer. | Real two-task touch project transfer/reorder, private in-app payload and exact Undo pass on API26/API37. Physical pointer/assistive and broader destination/family gestures remain open. |
| F-048 | Implemented | Native scrollable task action sheet and Move To project picker; compact selection bar keeps primary actions reachable and uses an overflow menu for planning/tag/delete. Planning includes the two localized weekdays after tomorrow, matching current desktop/iOS scope. | Real single/bulk menu actions, destination organization, long German labels/large text, archive exclusion and keyboard/accessibility invocation; broader grouping/presentation remains a native visual gate. |
| F-051 | Implemented | Single-task and selection-overflow planning offer Today/Morning/Evening/Tomorrow, the two following dates named by localized weekday, and next week. This matches the existing F-051 contract; it does not require a new seven-weekday picker. Rust owns day moves/family scope and Undo. | Native named-weekday group move and exact Undo pass on API26/API37; shared Rust owns date/family rules. Real OS local-date rollover, broader single/bulk menus and spoken localized feedback remain open. |
| F-052 | Implemented | Saved Regular/Compact choices in native View Options and Settings. Compact uses reduced native row padding/spacing, two title lines, one metadata line and a named notes indicator; regular retains Material ListItem with a notes preview. Full title/metadata semantics survive visual ellipsis, and controls retain 48dp minimums without replacing system text scaling. | Actual native compact row height/metadata/48dp targets, preferences and long German color/selection semantics pass. Broader largest text, archive/tablet/pointer and spoken accessibility remain open. |
| F-053 | Implemented | Android adapts navigation to bottom bar/rail with a leading list drawer rather than reproducing an iPhone split-view implementation. | Real portrait/landscape and tablet resize, retained destination/query/draft, logical focus and reachable list access. |
| F-055 | Not applicable | EventKit/Apple Reminders is Apple-specific. Android preserves compatible synchronized task/provenance data, without adding a new calendar/reminders service. | Shared consumer compatibility after core changes; no Android EventKit acceptance claim. |
| F-033 | Planned | v2 independence remains a future product direction. Existing storage/sync compatibility is preserved. | Approved format/migration design before implementation. |
| F-034 | Planned | Distribution pricing/subscriptions remain undecided. Self-built offline use has no paywall. | Product decision before any entitlement, billing dependency or commercial requirement. |
| F-035 | Not applicable | The recorded RevenueCat intention is specifically Apple-platform work, outside this Android implementation. | No Android billing SDK added. |
| F-036 | Planned | Tipping/provider integration remains future work, outside Android parity delivery. | Explicit product/store decision before implementation. |

## Recorded checks and remaining release gates

Evidence is from dirty `task/android-native`, base `200f686c`, on 2026-10-11 local
and UTC time. Production/test source is frozen at gate11h. B-130–B-132 correct
native fixtures; B-133/B-134 protect existing-format admission and near-limit
private recovery. Earlier failed checkpoints remain in the bug ledger/logs.

- **Build and packaging:** 91 JVM tests pass with zero failures/errors/skips; fresh
  debug, minified release APK and AAB build. Release lint has zero errors/nine
  warnings. Bundle validation passes with `PAGE_ALIGNMENT_16K`. Release and
  bundle-generated universal APKs pass strict LOAD/RELRO/ZIP checks for all eight
  Rust/JNA/Graphics Path/DataStore libraries across arm64/x86_64. Runtime uses a
  disposable local debug key; official signing is not configured.
- **Frozen artifact identities:** debug APK
  `b05d8d2fa4e14d077f667f81894c55e726637f6b4a3fec613d4490625e0a12bc`;
  test APK `69ca5866358123ba22bd293f714fb271a0f6e696f8481b872f18930dbf49ca4d`;
  unsigned release APK
  `781a31a8e739de1bb073141f7963b26835dd2e34638f35480e95791be29ed888`;
  AAB `80c80f0c9678d221375f44e5b17d5288ef54ecd17e4f2a95c313656eb8ea6456`;
  bundle-produced universal APK
  `90cee7ad51787171daea6a4a9eb8e3b6951e10ed3256648d6e05720de3ac1949`.
- **Complete native suites:** API26 runs 127 cases: 121 pass, six expected skips,
  zero failures (169.621 seconds). API37 runs 137: 134 pass, three expected skips,
  zero failures (182.939 seconds). API26 skips three modern large-URI clipboard
  cases and one text-import recipient case for the reproduced Android 8/8.1 grant
  bug (B-121); both skip opt-in timing and the separately orchestrated two-emulator
  harness. API37 additionally skips the API26-only clipboard preflight. Functional
  1,000-task and ordinary separate-UID explicit sharing still run in both suites.
- **Native source integrity:** all 19 focused admission/source/IME/Undo/sharing
  cases pass on API26 (102.437 seconds) and API37 (43.054 seconds), including the
  48 MiB near-limit reader regressions. Twelve earlier source/IME cases also pass
  on API36. Actual InputConnection/Ctrl+Z fixtures retain complete private source,
  caret/identity and Rust Save; they do not prove every installed keyboard.
- **Large-note performance:** identical shipping MainActivity/debug-AVD emoji input
  improves from 30.363/26.725 seconds cold/warm to 3.811/2.101 seconds in gate11h.
  Notes is reachable with one swipe, uses 4,096 units and retains all 552,960 source
  units. All six baseline/multiline/emoji cold/warm probes restore queue, draft,
  core and preferences exactly. Complete private-file comparison finds only the
  derived disabled-search receipt's `updated-at` changed. Host load 16.4–22.5
  affects these upper bounds; physical/minified performance is separate. Gate11e's
  opt-in 1,000-task projection samples are 23.997 ms/API26 and 17.170 ms/API37;
  complete IDs/titles/family/core counts and persisted edit/reopen pass currently.
- **Other scoped native evidence:** real touch drag/reorder/exact Undo, native
  12/24-hour picker, recovery, stale/restore notification actions, count privacy,
  imported names, real key events and Unicode transfer pass. Controlled two-emulator
  LibreSync pairing/encrypted offline convergence/reopen passed at gate8b. Actual
  minified API37 Downloads SAF export/replacement/cold persistence passed at gate7b;
  those adapters/Rust inputs are unchanged. Physical multicast, arbitrary document
  providers, spoken accessibility and scheduler-originated behavior are separate.
- **Shared consumers:** after the last shared-core change, both Rust workspace
  lanes pass 299 tests/one ignored; fresh generated Mac/iOS/CLI consumers build.
  MomentumKit passes 164 tests, MomentumMobile 158; unsigned Mac/iOS simulator
  Xcode builds pass. Later Android changes do not change those inputs. Linux GTK
  runtime and physical Apple deployment are not established by those checks.
- **Notices/scanners/source:** all four app artifacts retain identical 2,170,926-byte
  notices, including six pinned Rust standard-library files. Refreshed F-Droid
  binary scans of both release APKs have zero warnings/errors for known nonfree
  DEX classes/extra signing blocks. Prepared-source/metadata results and exact final
  snapshot are in [VALIDATION.md](../android/fdroid/VALIDATION.md). Fresh isolated
  offline vendored host/bindgen/both-ABI Rust builds pass for unchanged native inputs;
  this does not establish a Linux full-Gradle/offline-Maven builder or reproducibility.

### Current minified runtime

The current universal APK cold-launched on API37 with process absent beforehand,
a new running process, version 0.4.5/code 4005 and native 16,384-byte page readback.
Android reports 293 ms TotalTime/297 ms WaitTime. Actual tasks/editor/settings PNGs
and XML are in `android/rust-target/api37-release-gate11h/`. Tasks, preferences,
captures and drafts remain byte-identical; only the derived disabled-search receipt
updated its timestamp. The same minified APK also launches on API26 with a new process, native 4,096-byte
page readback and unchanged private core/settings/capture/draft files. Android 8
ThisTime/WaitTime and any accumulated stale TotalTime are preserved separately in
`android/rust-target/api26-release-gate11h-runtime.json`; see
[PROGRESS.md](PROGRESS.md). These runtime screens use synthetic data, not real accounts.

### Remaining acceptance

Android remains **In progress** overall. Per-feature rows retain required missing
interaction coverage rather than interpreting a successful suite as complete parity.
Physical TalkBack/Switch Access, third-party/hardware keyboards and pointers, real
keyguard/cold actions, actual launcher search/count surfaces, broad configuration
lifecycle, hosted TLS, scheduler-originated work, physical multicast/NAT and larger
store/battery performance remain open. Native labels and injected policy/input tests
are separate from those system interactions. Existing user-deferred desktop/Apple
checks are preserved.

Framework AppFunctions automation (F-031) awaits approval of the concrete
[optional proposal](superpowers/specs/2026-10-10-android-automation-proposal.md).
A read-only LibreSync connection probe (F-043/B-109) needs an upstream/core protocol
contract; Sync Now remains explicit. No missing capability is marked Not applicable.

Distribution requires an approved clean source revision, independent Linux/F-Droid
builder/provisioning, offline Gradle/Maven and APK dependency scan, second-host
reproducibility and signing/update continuity. Local source scanning is not store
admission. The [source recipe](../android/fdroid/README.md) remains disabled until
those gates pass. Android CI has no upload/publishing step; its first remote run is
separate. See [TESTING.md](TESTING.md), [RELEASING.md](RELEASING.md) and the
[distribution audit](../android/DISTRIBUTION_AUDIT.md).
