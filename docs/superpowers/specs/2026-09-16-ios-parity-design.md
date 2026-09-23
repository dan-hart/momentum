<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Momentum for iOS — architecture and parity proposal

Date: 2026-09-16. Baseline: `7ec82e9`, clean main at inspection.
Status: approved 2026-09-16. The user approved the architecture with tabs changed to
Today, Upcoming, Search and Settings, and authorized implementation through parity.
This is local work; no commit, push, publication or data migration is authorized.

Scope update, 2026-09-17: the user deferred LibreSync for now and requested Nextcloud
only. The active mobile sync selection is Off/Nextcloud. LibreSync requirements in
this original proposal remain deferred, with no claim of implementation or acceptance.

Scope update, 2026-09-18: the user resumed the remaining parity work. Off, Nextcloud
and LibreSync are again the active provider set. The user excluded spoken VoiceOver
acceptance and a real Low Power Mode toggle, and retained simulator-only validation.
The feature ledger and iOS checklist contain the current implementation and evidence;
the 2026-09-17 paragraph above remains as historical scope chronology.

## Outcome

A native SwiftUI app for iPhone and iPad, minimum iOS 26.0, compiled using Xcode 27,
with runtime verification on the installed iOS 26.5 and 27.0 simulators. All existing
macOS capabilities receive a native iOS implementation or an explicit, justified OS
equivalent. Desktop-only CLI distribution, login items, menu-bar windows and global
hotkeys do not become iOS services. Task rules, storage format, recurrence and sync
remain in Rust. Future billing, v2 migration and Android foundations are excluded.

## Architecture options and recommendation

1. **Recommended: shared Apple support with separate native apps.** Add `ios/` with
   an XcodeGen app and test targets using the existing build tooling. Extend the
   generated MomentumCore package with iOS device/simulator slices. Make portable
   portions of MomentumKit available on iOS (localized wording, task form mapping,
   preferences, Keychain, transfer values, automation); retain macOS-specific code
   behind platform boundaries. Use a dedicated iOS observable presentation model
   and lifecycle coordinator over a serialized, off-main engine worker. Keep current
   package paths to avoid an unrelated macOS project migration.
2. Make the existing Mac AppState universal. This reuses more state code immediately,
   but its synchronous main-actor calls, global selection and desktop timers/socket
   lifecycle are poor fits for independent mobile tabs and suspension.
3. Give iOS its own complete Swift support package. Isolation is straightforward,
   but duplicated localization, form mapping and preferences would drift.

The original recommendation added no third-party dependencies or hosted services.
The user subsequently requested DHFlatUIColors for the mobile accent picker; its
Swift package is the one approved additional dependency, pinned to
`821b077fc94ba45422ded8f38ee5b532dbabfd3e` (GPL-3.0), with no transitive dependencies. Apple
frameworks: SwiftUI/UIKit, Observation, UserNotifications, BackgroundTasks, Security,
CoreSpotlight, AppIntents and UniformTypeIdentifiers. Rust target support must be
installed for `aarch64-apple-ios` and `aarch64-apple-ios-sim`; only the macOS target
is currently installed. The existing ignored LibreSync source checkout must also be
made available using the repository's documented local dependency arrangement.

## Navigation and visual direction

Four stable, labeled system tabs: **Today**, **Upcoming**, **Search**, **Settings**.
Each preserves its navigation and scroll state. Today includes a Lists navigation
action for Morning, Evening, projects, tags and Archive; Settings has its own tab. Search uses a native searchable list within a regular tab; the special search role reorders it after Settings on iOS 27, contrary to the approved order. Task creation uses a labeled bottom-right floating button above the tab bar
throughout task browsing, with a fast entry sheet and an expanded editor.
On iPad, retain tab navigation with adaptive list/detail columns within Today.

Use native List/Form, NavigationStack, sheets, menus, date/time controls, selection,
drag/drop and swipe actions. Use the system Liquid Glass navigation supplied by the
OS, semantic colors, SF Symbols, project/tag color accents and generous touch targets.
The iOS app accent is **#FF6600**, explicitly requested by the user on 2026-09-16;
retain that base as Momentum Default. The user additionally approved an accent
picker whose custom options come exclusively from DHFlatUIColors. The latest palette refinement limits this to AsNeeded’s nine choices in spectrum order: Alizarin, Carrot, Orange, Emerald, Turquoise, Peter River, Amethyst, Pomegranate, and Green Sea. Show one list without a country/palette selector; retain saved IDs for these colors and let retired choices use Momentum Default. The latest user refinement keeps Momentum Default exactly #FF6600 in both light and
dark mode, including Increase Contrast; do not darken it. Filled default-orange actions use
white text and symbols in both appearances; Increase Contrast is the only exception
and uses calculated ink. This supersedes the previous adaptive light-mode default. Custom palette tones target 4.5:1 normally
and 7:1 in Increase Contrast. Exact default orange is not a blanket text-contrast
guarantee on light or elevated surfaces; preserve readable semantic body text and
use contrasting black/white content on solid swatches. Persist only a palette choice
identifier, with invalid values falling back to Momentum Default. No unrestricted
accent color input. The iOS app icon must be opaque and filled edge-to-edge, without
pre-drawn rounded corners or an inset border; iOS supplies its own icon mask.
The user also requested SF Symbols throughout the UI: use meaningful symbols for
navigation, settings, editor sections and actions, with brief state-driven symbol
motion that respects Reduce Motion; retain text labels and native accessibility.
Use brief completion feedback, restrained spring transitions and optional haptics;
Reduce Motion removes unnecessary movement. Dynamic Type, VoiceOver actions, hardware
keyboard shortcuts, light/dark appearance, increased contrast and German are required.
Settings is a short native index for Task Lists, Appearance, and About Momentum;
keep grouping/sorting/archive behavior together, and colors/text/feedback together.
Task capture and editing use labeled SF Symbol actions, native swipe shortcuts,
brief nonblocking state transitions, and preserve drafts after failed saves.
Native iOS font selection and independent relative content/interface sizing preserve
the Mac typography capability while still respecting Dynamic Type.

## Shared behavior and concurrency

One Engine owns the sandboxed Application Support store. Every mutation persists
through the existing engine and returns its normal outcome/undo batch. No Swift
implementation of sorting, grouping, recurrence, parsing or sync conflict rules.
Engine reads and writes run through a serialized worker away from the main actor;
views consume immutable snapshots with stable task IDs. Sync runs on a utility worker
using existing core coordination, never blocking UI interaction on network I/O.
Publish refreshes after mutations, transport events, preferences and foreground/day
changes. Coalesce refresh/index/notification work and debounce cancellable search.
Keep one engine owner for UI, URLs, intents and notification actions; cold invocations
must use the same bootstrap path, avoiding competing stores and stale snapshots.
Initial App Intents are hosted in the app process, without an intents extension or
separate shared-store writer. The macOS AppState execution model remains unchanged;
portable support extraction must not silently make desktop APIs asynchronous.

The app does not start the desktop CLI socket or pending-file polling. Foreground
LibreSync is stopped on background transition; selected-provider foreground sync and
bounded OS-scheduled refresh resume safely. Keep saved connections and pending ops
when switching providers. Background expiration cancels or safely limits work without
discarding local changes or reporting a false successful exchange. Any core transport
API gaps are fixed in the core and tested for desktop regressions.

Use a core-owned transport generation and exclusive replacement barrier. Only one
transport may exchange/commit at a time. Local edits remain allowed during network
I/O and are merged by stable operation IDs, not a pending-array offset. Import cancels
and invalidates an exchange before replacing state; the final sync commit rejects
stale generations and cannot write old snapshots to the live store. Provider switching
stops the prior provider before starting the next and ignores its late callbacks.
Expose bounded request deadlines and cooperative cancellation in Rust; Swift task
cancellation alone does not cancel a blocking FFI call. Background expiration signals
that boundary, prevents a late commit, and leaves pending operations durable. Any
staging files stay outside the live store until the guarded commit. Tests must cover
edits during exchange, restore during exchange, rapid provider switching, expiration,
late callbacks, denied transport startup and interrupted persistence.

## Notifications and background behavior

Task reminders use OS-scheduled local notifications, with core-derived identifiers,
times and Done/Snooze actions; reconcile on edits, completion, restore, sync and
foreground activation. Permission is requested in context, with actionable denied
state. Notification scheduling must not consume a core delivery claim prematurely.
Add a tested core planning/acknowledgement boundary if the current due-now API cannot
support this. Do not reproduce reminder eligibility in Swift.

The Rust planner projects scheduled occurrences for the next seven local days without
creating tasks early. It uses the existing recurrence rules and deterministic IDs;
notification actions resolve/materialize the matching occurrence through the core
before completing or snoozing it. A changed/stopped rule invalidates its old projected
occurrences. Reserve up to 60 pending requests as a conservative app budget: task
reminders take priority in chronological order, and any remaining slots hold up to
four upcoming daily summaries. An overflow or exhausted horizon is visible in
notification settings with an explanation that opening Momentum refreshes the plan.
Replenish on foreground, edits, notification actions and opportunistic background
refresh. Never claim an indefinite schedule when iOS has not granted execution.

Persist accepted scheduling identities separately from delivery claims; only record
an accepted request after the OS scheduler succeeds. Reconcile the durable schedule
ledger with pending/delivered OS requests at startup, preserving at-most-once policy
for elapsed occurrences even when a delivered notification was dismissed. Scheduling
is not evidence of delivery. Permission denial preserves task reminders for future
reconciliation and displays recovery; future requests are replanned after permission
changes. Tests cover horizon/capacity boundaries, recurrence edits, skipped/repeated
DST times, idempotent actions and the unchanged desktop due-now path.

Morning summary remains opt-in at a local time, default 08:00. The core owns counts
and daily duplicate prevention. Mobile cannot calculate fresh task counts at a
guaranteed time while suspended. Prepare a bounded core-derived notification plan
with explicit snapshot semantics and reconcile at foreground/background refresh;
foreground catch-up and scheduled delivery share durable identity to avoid duplicates.
Settings explain that background summaries reflect the latest on-device plan and
delivery is controlled by iOS. Tests cover cancellation, denial, DST/time-zone changes,
restart, empty days and disabling/re-enabling. No perpetual background timer or socket.

Nextcloud uses the existing protocol and queued operations. LibreSync retains pairing,
discovery, linked-device management, manual sync and foreground automatic exchange.
Declare local-network/Bonjour usage, use Keychain for secrets, and show persistent
provider/progress/last-success/errors/retry. Suspended iOS devices cannot be promised
the desktop's always-listening behavior; the UI explains opening the app to exchange.
Existing physical desktop LibreSync acceptance deferrals are preserved. iOS network
acceptance uses isolated stores and test peers, never personal tasks or credentials.

## Parity acceptance map

| Existing IDs | iOS acceptance |
|---|---|
| F-001–002 | Generated FFI, sandbox persistence, offline launch/edit/restart, real Rust store |
| F-003–006 | All views, projects/tags and colors, quick-add grammar/autocomplete, full task editor, notes/subtasks/duplicate |
| F-007–008 | Date/time/reminders, native notification actions, full repeat editor and catch-up |
| F-009–010 | Complete/archive/restore/auto-archive, multi-selection and every bulk mutation |
| F-011–015 | Native drag/move/tag/day/reorder with accessible alternatives, archived guards, external text/URL drop, multiline paste |
| F-016–018 | Search notes/subtasks/archive, stable paging, undo/error recovery, loading/empty/all-done layouts |
| F-019–023 | Native appearance, accessible motion/color/focus, EN/DE, font and size controls, hardware keyboard help |
| F-024 | URL entry and system search; mobile lifecycle in place of login/menu-bar/global-hotkey services |
| F-025–028 | Off/Nextcloud/LibreSync exclusivity, real protocol, paired test peer, visible durable status and recovery |
| F-029 | Native document export/import, security-scoped access, explicit destructive replacement confirmation |
| F-030 | Desktop CLI is not applicable on iOS; its unchanged shared-core behavior still gets regression checks |
| F-031–032 | Native create/find/complete/plan automation, cold/warm invocation, configurable app icon badge |
| F-037–038 | All exclusive grouping modes and order/undo rules; configurable opt-in summary with mobile delivery semantics |

F-033–036 remain future product decisions, not requirements to introduce monetization
or migration. Existing B-003 core regression is inherited and exercised through iOS;
B-001 is not closed by an iOS simulator or loopback pass.

## Delivery sequence

1. Generated device/simulator FFI build, portable Apple support, isolated iOS bootstrap,
   engine-worker regression tests and a running native four-tab app.
2. Complete offline task workflows, forms, organization, archive/search, grouping,
   transfer, undo and preferences; validate each against the parity map.
3. Native notifications, lifecycle-aware sync and pairing, backup, Spotlight, intents,
   badges and hardware keyboard integration.
4. Polish, EN/DE accessibility/layout coverage, performance profiling, energy/lifecycle
   checks, both-OS native acceptance and desktop regression verification.

Keep FEATURES/BUGS/PROGRESS current throughout implementation, distinguishing code,
automated checks and observed native behavior. Completion requires Verified status for
the recorded iOS parity scope; unavailable physical-device/system checks remain visible
and prevent an unqualified parity-complete claim.

## Verification and evidence

Use fresh test stores/preferences and sync-disabled demo fixtures. Run Rust portable
workspace suites, shared Apple Swift tests, iOS boundary tests and XCUITest workflows
on iOS 26.5/27.0 iPhone plus iPad layout coverage. Build the iOS device target unsigned
and the existing macOS target. Linux runtime checks are required only for changed
Linux-specific behavior; unavailable Linux checks are recorded without claiming a pass.

Verify persistence after relaunch, cross-tab refresh, rapid edit/search cancellation,
notifications and transport lifecycle, provider switching, interrupted sync/import,
VoiceOver labels/actions, Dynamic Type, Reduce Motion, increased contrast, German
layout and compact landscape. Measure representative large-list scrolling, launch,
mutation latency and background activity using Xcode tooling; report observed results,
not universal battery or frame-rate guarantees. Physical notification delivery,
background scheduling and energy measurements require a suitable device when available.

## Source references

- Repository: AGENTS.md; docs/FEATURES.md, BUGS.md, PROGRESS.md, MVP.md, TESTING.md,
  NOTIFICATIONS.md, GROUPING.md and P2P.md; macos/README.md.
- Current source: momentum-core; momentum-ffi; MomentumKit AppState, Services,
  TaskFormModel, Preferences, Typography, Shortcuts; macos/project.yml and build-core.sh.
- [Apple tab-bar HIG](https://developer.apple.com/design/human-interface-guidelines/tab-bars)
  and [SwiftUI tab navigation](https://developer.apple.com/documentation/swiftui/enhancing-your-app-content-with-tab-navigation).
- [Apple iOS 27 overview](https://developer.apple.com/ios/whats-new/).
- [Apple background strategies](https://developer.apple.com/documentation/backgroundtasks/choosing-background-strategies-for-your-app).

Environment inspection confirmed Xcode 27.0 build 27A266a, iOS 27 SDK and available
iOS 26.5/27.0 simulators. Local xcdocs search currently fails with an embedding-service
error; official Apple web documentation and installed SDK interfaces are the fallback.
