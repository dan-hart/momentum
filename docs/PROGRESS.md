# Momentum progress and handoff

Updated: 2026-09-16. Product rules live in [AGENTS.md](../AGENTS.md); exact capability
and defect status live in [FEATURES.md](FEATURES.md) and [BUGS.md](BUGS.md).

## Current workspace

Desktop development was consolidated from `.claude/worktrees/pull-latest-code-70308a`,
based on `0c9c0bf`, using integration branch `task/desktop-mvp` and target `main`.
The commit containing this handoff includes the accumulated macOS/shared-core work,
Linux parity, grouping, configurable summaries and the canonical project guidance.
Preserve any subsequent uncommitted work and inspect current Git state before continuing.

The earlier secret-scanner blocker is resolved: on 2026-09-16 the user explicitly
approved adding `FileCreateFlags::PRIVATE` to `.gitallowed` for the confirmed GIO
enum-name false positive, then committing and pushing to main. The existing scanner
hook remains enabled. Earlier blocked attempts below are historical, not a new request
for approval. The outer checkout's temporary routing guide is superseded by the full
tracked AGENTS.md when main fast-forwards to this integration.

## Platform baseline

| Platform | Current state | Verification limits / next direction |
|---|---|---|
| Linux | Established native GTK4/libadwaita MVP using shared Rust engine | Current Mac-hosted pass compiled GTK/tests; Linux runtime/remote CI not rerun. Sync parity implemented; Blueprint/native Linux verification pending. |
| macOS | Established SwiftUI/AppKit MVP using UniFFI/shared core | Local build/tests and many native flows recorded; specific deferred/unverified checks below. |
| iOS | Upcoming; no native app foundation yet | Plan native frontend/platform services around shared Rust core; scope/order to be agreed. |
| Android | Upcoming; no native app foundation yet | Plan native frontend/platform services and bindings around shared Rust core; scope/order to be agreed. |

## Current priorities

- **F-038: configurable morning summary.** Linux/macOS implementation and automated
  checks pass; Mac Settings off/default readback passed. Finish native time-edit/delivery
  acceptance and Linux Blueprint/runtime checks when those environments are available.
  iOS/Android carry explicit requirements in [NOTIFICATIONS.md](NOTIFICATIONS.md).

1. **F-037: Group By on Linux/macOS, planned for mobile.** Shared Rust grouping, native
   View Options controls, independent platform tests and status. The user approved first-tag-only
   placement and six estimate intervals, plus #tag and project headings in their saved colors. Native Mac
   grouping now defaults to Morning & Night with one exclusive grouping layer. Revised
   automated checks pass; native verification limits are recorded below. No
   data migration is involved.
2. **F-027 / F-028: verify Linux sync parity.** Implementation is complete; run
   Blueprint/resource compilation and GTK tests on Linux, then check provider switching,
   persistent status on empty/populated lists, Details/Retry, and CLI routing. Use isolated
   settings/data. Physical LibreSync testing remains separately deferred by the user.
3. Establish the next mobile milestone with the user; neither mobile app is implicitly
   scaffolded by desktop feature work. Do not choose a delivery order without a decision.
4. Preserve open acceptance gaps; resume user-deferred work only when requested.

## Approved decisions — 2026-09-16

- F-037 revision: Morning & Night is the default; Today/Morning/Evening are tag-based
  groups in that mode only. Other modes show only their chosen grouping. Completed
  remains separate. Applied interpretation of “only grouping”: overdue/upcoming dates
  appear on rows instead of extra date sections; the optional clarification has not
  received a response yet. Preserve explicitly saved grouping preferences.

- F-037: Group By uses the first tag only, six approved estimate ranges, and #tag
  and project headings colored with their saved colors. See [GROUPING.md](GROUPING.md).

- Momentum respects user freedom and prioritizes excellent native experiences on Linux,
  macOS, iOS, and Android. Existing Linux/Mac technology is the architecture baseline.
- Reuse one Rust core for task rules, storage and sync. Native frontends own presentation
  and OS integration; do not duplicate product rules or share a cross-platform UI layer.
- New work covers every existing platform by default. Record concrete mobile requirements
  until those apps exist; thereafter include them by default too.
- Offline-first, account-free core task management; optional sync that is seamless when
  enabled. No advertising, tracking or analytics without explicit approval.
- Code remains open source and free to build, compile, self-host and use. Official-build
  pricing is undecided. Eventual Apple RevenueCat subscriptions and Apple/Buy Me a Coffee
  tips are intentions, not approved integrations.
- v2 aims to leave the SP foundation; format, migration and interoperability are undecided.
- Agents proceed with routine work; ask about major architecture, data migrations,
  dependencies/services, monetization and ambiguous product behavior.
- Verified is the completion bar, with independent per-platform status and evidence.
- Local changes only; commits, pushes and PRs require explicit requests.

## Open acceptance and decisions

| Item | State | Next action |
|---|---|---|
| Physical LibreSync Linux↔Mac, B-001/F-026 | User deferred testing; implementation is not a proven real-device fix | Await request to resume; preserve earlier evidence and diagnostics |
| Spotlight create/open, F-024/F-031 | User deferred testing after automation could not launch Spotlight | Await request to resume native discovery and warm/cold invocation |
| Spoken VoiceOver, F-020 | Unverified: VoiceOver did not remain enabled; AX/keyboard/contrast checks did pass | Test with a working VoiceOver session; do not claim spoken output from AX evidence |
| DND edge cases, F-012/F-013 | Shared tests cover tag/day, descending order, archive exclusion; separate native gestures unverified | Complete targeted native cases when environment permits |
| Linux runtime validation | Needs a Linux desktop or supported Broadway/Flatpak environment | Run Linux checks for affected changes; compilation alone is insufficient |
| Mac exact minimum width / independent German language review | Not separately verified | Target these when changing affected layouts/copy |
| Universal/release builds and remote CI | Local arm64 development evidence only; remote workflow not executed in the prior pass | Validate separately when release work is authorized |
| v2 SP departure, F-033 | Goal approved; implementation choices open | Propose migration/compatibility design before changing formats |
| Billing/tips, F-034–F-036 | Future intention; pricing and entitlements open | Product decision and current provider/store review before implementation |

## Dated handoffs

### 2026-09-16 — Morning & Night default and exclusive grouping (F-037, B-003)

- **Request:** Morning & Night becomes the default; Today/Morning/Evening derive from
  tags only in that mode. Project/Tag/Time Estimate ignore those divisions. This
  supersedes the earlier nested-grouping design and None default below.
- **Behavior:** one flat set of open tasks is sorted, then grouped once. Default
  sections are Today (no time-of-day tag), Morning, Evening, in that order; matching
  is case-insensitive and Morning wins when both tags exist. None is a flat list.
  Completed stays one separate section. Overdue tasks remain visible and Coming Up
  retains its date window, with due dates shown on rows instead of date sections.
  Search retains result-type separation. Explicit saved grouping choices are preserved.
- **Implementation:** shared Rust rules and smaller base-list construction; native
  GTK/SwiftUI menu/default mappings; regenerated UniFFI; English/German strings;
  tests updated to the new contract rather than the superseded section structure.
  Mobile requirements and MVP.md now reference the same exclusive-grouping contract.
- **B-003:** dual-tag drop/toggle previously treated raw tag presence as the active
  day period. Resolve the actual period and remove opposing matching tag IDs; regression
  covers mixed case, both tags, parent/subtask placement, moving groups and undo.
- **Evidence (dirty worktree based on 0c9c0bf):** default portable Rust and all-features/
  all-targets suites each pass 164 tests (72 shared-core); Swift passes 123 tests in
  29 suites, with the existing opt-in nearby-network test skipped. arm64 Debug Xcode
  build succeeds. GTK application/tests compile. Strict GSettings schema compile and
  isolated memory-backend readback confirm `morning-night` as the default.
- **Localization/quality:** compiler extraction reports complete German catalogs
  (286 app / 159 package strings); built resource checks include Morning & Night and
  Evening. Seven localization-tool tests, formatting, whitespace and changed-doc links
  pass. No new dependencies, data migration, commit or push.
- **Native evidence:** rebuilt isolated Mac preview shows only colored Home/Momentum
  project sections, combining Morning/Evening tasks and retaining overdue row dates.
  Further menu automation repeatedly reported “user changed the app”; preview left
  available without fighting concurrent interaction. Default-mode native selection,
  revised grouped gestures, spoken VoiceOver and native Linux acceptance remain open;
  core and Swift tests do cover default switching, None, family placement and undo.
- **Next:** finish native menu/keyboard/grouped-move acceptance when the UI is available;
  run the Linux Blueprint/GTK suite in a supported environment. Prior real-device
  LibreSync and Spotlight testing remains deferred by the user.


### 2026-09-16 — Group By implemented (F-037)

- **Workspace:** new local edits on the staged, uncommitted desktop baseline at
  `0c9c0bf` in `pull-latest-code-70308a`. No commit, push, scanner exception, dependency,
  data migration or mobile scaffold was added.
- **Shared core:** None/Project/Tag/Time Estimate; first valid stored tag only;
  approved six contiguous estimate intervals; alphabetical named groups; preserved
  date/completion sections, family placement, search caps and archive paging. Manual
  reordering and keyboard nudges stay inside a group and undo restores full order.
  Group metadata includes saved project/tag colors. See [GROUPING.md](GROUPING.md).
- **Linux:** native settings-backed radio submenu, persistent `group-by` preference,
  localized headings and colored project/#tag names with safe markup escaping,
  wrapping and neutral high-contrast/color-preference fallback. GTK tests compile;
  schema compilation and supported-choice readback pass. Native Blueprint/GTK runtime
  checks still require a Linux desktop/SDK (unavailable on this host).
- **macOS:** native Picker in View Options and View menu, local persistence, localized
  list sections and saved project/#tag colors. Existing color accessibility policy and
  typography apply; date context stays neutral and overdue context stays red.
- **Automated evidence:** five new core grouping regressions cover exact boundaries,
  first-tag uniqueness, subtasks, per-group sorting, metadata colors, grouping without
  writes, None restoration, reorder/nudge/undo, search truncation and archive paging.
  Portable Rust suites pass in default and all-features/all-targets configurations
  (159 tests each). Final Swift suite reports 123 tests in 29 suites passing, with the
  existing opt-in nearby-network test skipped. Final arm64 development Xcode build
  succeeds. GTK application and test binary compile; runtime tests were not run.
- **Native Mac evidence:** isolated `MOMENTUM_DEMO=1` preview with sync disabled;
  native menu selection/readback for Project, Tag and Time Estimate; screenshots
  confirm colored project and #tag headings and separate overdue context. Grouped
  rows retain the demo tasks and expected ranges. The feature has not separately
  repeated real drag gestures, spoken VoiceOver, or every font/contrast/window size;
  core reorder regressions and existing accessibility policy tests pass.
- **Localization/checks:** compiler-extracted catalogs are complete (285 app and 158
  package German entries); compiled German grouping strings and existing interpolation/
  plural/intent resources pass. Seven localization-tool tests, Rust formatting and
  whitespace checks pass. German wording has not had independent language review.
- **Mobile:** explicit iOS/Android native control, shared-core behavior, persistence,
  accessibility, reordering and test requirements recorded in GROUPING.md. Both Planned.
- **Next:** native Linux acceptance (including menu/schema resources, grouped row
  ordering, selection and color fallbacks); targeted grouped native drag/accessibility
  checks where available. Real-device LibreSync and Spotlight remain user-deferred.


### 2026-09-16 — Integration into main authorized

- User explicitly requested committing and pushing to `main`; this authorizes this
  integration, without changing the local-only default for future work.
- Remote main was fetched and matched the development base; integration can fast-forward.
- The full native Mac/shared-core work, Linux sync parity, AGENTS.md and platform ledgers
  were staged. Generated binaries, personal data/recovery files and signing identities
  remain excluded. Integration and replacement of the routing guide did not occur.
- Commit stopped at a scanner false positive; the proposed allowlist exception was
  rejected by automatic approval review and is awaiting explicit user approval. No
  exception was written and no commit/push occurred.
- Formatting, whitespace and seven localization-tool tests passed. Fresh regression
  checks passed: 24 CLI tests; Swift reported 120 tests in 28 suites with its opt-in
  network test skipped. Existing Linux native UI and deferred physical LibreSync/
  Spotlight limits remain unchanged; pushing is not verification.

### 2026-09-16 — Project guidance and parity ledger established

- Created the full AGENTS.md and three ledgers from the user's explicit decisions;
  added an outer-checkout routing guide to preserve the current development location.
- Seeded platform status conservatively from current source and recorded acceptance.
  No blanket “Verified” migration from the older “MVP implemented” description.
- Found a current source-backed gap: Linux GSettings/preferences bind independent
  `sync-enabled` and `p2p-enabled` switches, while macOS selects one provider. The user
  requested Linux parity; work is active as F-027/F-028.
- Documentation links, ledger IDs/status cells and whitespace were checked. No commit/push.

### 2026-09-16 — Linux sync parity implemented (F-027 / F-028)

- Replaced the independent Nextcloud/nearby switches with one native libadwaita ComboRow.
  Only the selected provider's settings are shown. Legacy dual-enabled preferences resolve
  to LibreSync as on macOS; no tasks, secrets or pairings are migrated/deleted.
- Manual/automatic/CLI sync respects the selected provider. Switching cancels a queued
  Nextcloud debounce, stops unselected LibreSync, and waits for an in-flight Nextcloud
  cycle to finish before starting LibreSync. Preview mode never starts a sync transport.
- Added a persistent content footer for Off/provider/progress/last-success/setup state,
  including empty lists. Failures retain Details and Retry. LibreSync lifecycle events
  now update native progress/error state rather than merely repainting a caption.
- Export/import non-secret connection settings for the desktop CLI; selected method is
  owned by native preferences. Linux keyfile resolution honors Off/LibreSync even with
  cached Nextcloud credentials and rejects unknown methods rather than falling back.
- Updated English/German UI strings, schema, user-facing sync docs and feature statuses.
- **Validation:** native GSettings provider-policy test passed using an isolated memory
  backend; two CLI policy tests and all 22 CLI integration tests passed. The first CLI
  attempt could not create sandboxed Unix sockets; the permitted isolated rerun passed.
  `cargo check -p momentum`, `cargo test -p momentum --no-run`, Rust formatting, strict
  GSettings schema compilation, POTFILES completeness, German PO compilation and whitespace
  checks passed. Existing dependency warning: `block v0.1.6` future incompatibility.
- **Limits:** this Mac has no Blueprint compiler, Broadway display, or active Linux
  container. The new GTK settings/empty-state/error regression compiles but has not run;
  Rust compilation does not compile Blueprint resources or verify native Linux UI.
  Linux-only keyfile loading integration is source-reviewed; its parsing policy is tested
  on this host. No real-service/physical LibreSync test, remote CI, release, commit or push.
- **Source/evidence:** dirty worktree based on `0c9c0bf`; tests live in
  `crates/app/src/prefs.rs`, `crates/app/src/tests/ui.rs`, and `crates/mo/src/main.rs`.
  Local diagnostic logs: `/tmp/momentum-linux-check-final.log`,
  `/tmp/momentum-linux-sync-policy.log`, `/tmp/momentum-linux-cli-verified.log`.
  These ephemeral logs are supporting evidence, not portable project dependencies.

### 2026-09-16 — Prior desktop acceptance evidence (carried forward)

This is historical evidence from [MACOS-MVP-CHECKLIST.md](MACOS-MVP-CHECKLIST.md),
not a claim that those commands were rerun while writing this guide. Source state:
dirty worktree based on `0c9c0bf`; affected changes must be revalidated.

- Portable Rust: 152 tests per default/all-features variant; GTK compilation and test
  compilation; formatting and gettext/catalog checks passed in the recorded sweep.
- Swift: 120 tests reported across 28 suites, including one skipped opt-in network test.
  Signed Mac Debug build and compiled German resource checks passed.
- User confirmed native single/multi-project drag, reorder/Undo and external text/URL
  drops. Native Mac keyboard, fonts, empty-state and contrast checks are documented.
- AX audits reported zero unnamed controls (22 main-window, 36 task-editor controls).
  Spoken VoiceOver is not covered by those audits.
- Normal Mac app was reopened on the user's store with Today selected, system accent
  background and contrasting star. Nextcloud success was observed. Real-device
  LibreSync and Spotlight checks were explicitly deferred, not passed.

## Handoff format

Append a dated entry with feature/bug IDs, platforms changed, observable behavior,
source revision or dirty state, actual commands/results and native/user evidence,
remaining gaps and next steps. Refresh current priorities and ledger cells in the same
change. Never include private tasks, credentials or machine-specific server addresses.

## 2026-09-16 — F-038 configurable morning summary

- **Scope/state:** dirty development worktree based on `0c9c0bf`; inherited staged
  changes preserved. Linux/macOS Implemented, iOS/Android Planned. No commit/push.
- Replaced the hard-coded automatic 05:00 rule with shared-core opt-in and local time.
  Off by default, initially 08:00, independent task reminders. Native settings retain
  the time when disabled and update the running engine. English/German copy explains
  local time, next-launch catch-up and the running-app requirement.
- Regenerated UniFFI through `macos/scripts/build-core.sh --debug`; native GTK and
  SwiftUI controls map to the same Rust preference record. Mac notification adapter
  must be attached before consuming reminders/summary eligibility.
- **Automated:** 166 Rust tests passed in each default and all-features/all-targets
  workspace configuration (excluding GTK runtime); 74 shared-core tests include default
  off, before/at selected minute, late launch, persisted once-per-day claim, next-day
  eligibility, disable/re-enable/time edits, empty tasks and independent task reminders.
  126 Swift tests passed in 29 suites; existing opt-in nearby integration remains skipped.
  New Swift tests cover preference persistence/validation and notification adapter gating.
- **Build/settings/localization:** signed arm64 macOS Debug build passed; GTK test binary
  compiled on this Mac; strict GSettings schema compilation and two memory-backend
  preference tests passed. Linux native regression for switch/time bindings and immediate
  engine updates added and compiled, not executed. German coverage: app 290/290, kit
  159/159, permission strings 2/2 and shortcuts 5/5; runtime bundle check and seven
  localization-tool tests passed.
- **Native Mac readback:** opened isolated demo Settings → General. Accessibility tree
  exposed Notifications, Morning summary Off, disabled system time picker at 08:00,
  and the explanatory copy. Subsequent screenshot targeted a changed main window rather
  than Settings, so visual layout/time-edit interaction and actual OS summary delivery
  are not marked verified. No test enabled morning notifications in the user's settings.
- **Limits/next:** run Blueprint/resource compilation and GTK UI regression on Linux;
  verify native time editing, restart persistence and system delivery using an isolated
  notification profile, including denial, larger text and German layout. Mobile app
  foundations remain absent. LibreSync and Spotlight real-device deferrals unchanged.
- Local logs: `/tmp/momentum-morning-{core,ffi,swift,build,linux,schema,rust,rust-all}.log`.

## 2026-09-16 — F-019 macOS centered task checkboxes

- User explicitly scoped this refinement to macOS. Changed `TaskRowView`'s outer
  HStack from first-text-baseline to center alignment. Checkboxes, archived checkmarks
  and trailing badges center against the entire title/metadata stack, including when
  text wraps; subtask indentation and native checkbox behavior are unchanged.
- Verified scope: signed arm64 Debug build passed (`/tmp/momentum-checkbox-build.log`);
  launched the updated isolated demo and visually inspected Today, Morning and Evening
  rows with title plus metadata. Checkboxes and reminder/repeat icons are centered.
  Larger text and wrapped-title variants were not separately exercised this pass.
- No logic change or new unit tests; no Linux/mobile changes requested. Preview is
  open with the rebuilt app. Existing staged work preserved; no commit or push.

## 2026-09-16 — Local Release installation

- User requested a release installation on this Mac. Built current dirty-worktree
  source as optimized arm64 Xcode Release, Momentum 0.4.0 (build 1), with local Apple
  Development signing. Installed at `/Applications/Momentum.app`; application data and
  preferences retained in their existing locations. Updated the existing development
  `~/.local/bin/mo` symlink to the installed app's bundled companion.
- Release initially failed loading Rust proc-macro helpers. A minimal `extern crate`
  reproduction exposed `mis-aligned LINKEDIT string pool` on macOS 27, matching
  [Rust issue 157750](https://github.com/rust-lang/rust/issues/157750). Local build override
  `CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_STRIP=none` avoids stripping compiler helpers;
  app/CLI release optimization and LTO remain enabled. Use that environment variable
  for release builds on this toolchain. No project-wide compiler setting was changed.
- Verification: Xcode Release build succeeded, installed app and bundled `mo` passed
  strict/deep code-signature validation, installed executable hashes match the built
  release, German bundle checks passed, and the installed CLI responds to `--version`.
  Launched the installed app successfully with Today selected and existing task data.
  This is a local installation, not a notarized distribution or published release.
- Logs: `/tmp/momentum-release-install.log` and `/tmp/momentum-release-core-fixed.log`.
  Previous feature verification/deferred acceptance remains as recorded. No Git commit,
  push, tag or publication occurred.

## 2026-09-16 — Desktop integration approved after release installation

- User requested commit/push and explicitly approved the enum-name scanner exception.
  The exception is limited to `FileCreateFlags::PRIVATE`; the pre-commit secret hook
  remains enabled. Consolidation includes all 127 reviewed pending files plus the
  approved `.gitallowed` update; installed/generated app bundles remain excluded.
- Remote main matched base `0c9c0bf` at preparation. Integration uses a normal
  fast-forward push from `task/desktop-mvp`; no force push or history rewrite.
- Formatting, staged whitespace checks, localization-tool tests and the full cached
  secret scan are integration gates. Recent Rust/Swift, GTK compilation, GSettings,
  localization, Debug/Release builds and native acceptance are recorded above.
  This integration does not promote any deferred or unverified platform checks.
