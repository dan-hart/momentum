# Momentum bug ledger

Updated: 2026-10-22. Use the platform status vocabulary in [AGENTS.md](../AGENTS.md).

F-073 celebration visual refinement: the user requested clearer button styling and a
larger effect. Tracked as a feature refinement in FEATURES/PROGRESS; completion
eligibility and task data rules remain unchanged.

For each bug, also state impact: reproduced, reported, suspected, not reproduced,
unaffected, or unknown. A status alone must not imply that a bug reproduced on that OS.
Keep resolved entries. Verification gaps without a known product defect belong in
[PROGRESS.md](PROGRESS.md), not as invented bugs here.

Severity: Critical = data loss/security/core unusable; High = major workflow impaired;
Medium = meaningful defect with workaround; Low = minor presentation/polish. Assess
severity from evidence; do not infer security or data loss from an ordinary sync failure.

## B-174 — Linux sound selection reenters native model replacement

- **Severity:** High (native crash). **Impact:** reproduced in hosted Linux GTK tests
  and an isolated GTK-on-macOS native diagnostic. Other native frontends use separate
  controls and are unaffected by this GTK callback.
- **Reproduction/cause:** selecting a completion sound synchronously saves GSettings;
  its change handler replaces the ComboRow model before the native selection setter
  finishes. The native backtrace shows invalid selection state during GObject notify.
- **Fix:** defer settings-to-model refresh until the main-loop idle, keep the row weak,
  read the latest saved value and retain the feedback-write guard. The existing sound
  preferences regression covers every sound, explicit previews and unknown-value
  fallback without rewriting newer values. Native GTK regression fails before and
  passes after the correction; hosted Linux confirmation remains required.

## B-172 — Celebrate icon and label lose contrast on bright accents

- **Severity:** Medium. **Impact:** iOS reproduced in the native component and
  shared Apple source; implicit white on the exact orange `#FF6600` is **2.936:1**,
  below the 3:1 meaningful-icon threshold and 4.5:1 normal-text threshold. macOS
  shares the affected implicit-foreground implementation. Linux native white on
  several default libadwaita accents passes icon contrast but misses normal-text AA.
  Android normal colors pass for its 20 tested palette/appearance pairs, but overlapping
  focused/pressed native ripple layers can reduce them below 4.5:1. Wallpaper-driven
  dynamic colors retain native role pairing, with device acceptance outstanding.
- **Cause/fix:** choosing a native prominent style did not guarantee a safe foreground
  for the actual app/system accent. Apple now resolves the actual accent and paints
  an opaque fill with measured black/white icon and label ink, including pressed fill.
  Linux preserves native pill/focus/high-contrast/disabled treatment while calculating
  enabled normal/hover/pressed fill and ink; accent changes refresh the scoped provider.
  Android retains primary/onPrimary and native feedback, with Celebrate-only ripple
  opacity bounded using the resolved theme roles and worst overlapping state, including
  the API 26/27 pressed-alpha multiplier.
- **Evidence:** failing-before/fixed-after numerical regressions cover native palettes,
  custom luminance boundaries and interaction states. iOS four light/dark and standard/
  increased-contrast component renders pass; orange resolves to black at 7.153:1 normal
  and 5.637:1 pressed. Seven Android palette/ripple tests, debug assembly and lint pass;
  independent continuous compositing measures at least 4.5226:1 across the 20 palette
  pairs under the conservative overlapping-state bound. Exact checks and remaining
  platform acceptance are in PROGRESS.
  No blanket claim is made for arbitrary third-party themes, wallpapers or physical devices.

## B-173 — Native overhaul release gates reject Settings resources

- **Severity:** High (build/release blocked). **Impact:** reproduced in hosted Linux
  Flatpak Blueprint compilation and macOS compiled-catalog completeness checks.
  Android is unaffected; iOS shares the proper-name catalog correction.
- **Cause/fix:** Adw.ButtonRow has no subtitle property; backup rows now use native
  activatable Adw.ActionRow with their original subtitles/actions. Two new Mac Settings
  labels lacked German entries; app entries are translated and shared proper names
  explicitly marked nontranslatable.
- **Evidence:** Blueprint reports both unsupported properties before the change and
  compiles all resources after it. The compiled localization regression fails on both
  missing labels before the fix, then passes all 66 checks; seven importer tests and
  source completeness checks pass. Exact-head hosted gates remain required before release.

## B-170 — Native period completion loses its destination or shows stale claims

- **Severity:** Medium. **Impact:** Android source/unit-reproduced final-period
  autoarchive fallback; Apple source-confirmed stale completed-row presentation.
- **Cause/fix:** native navigation previously fell back when the last slot task was
  archived, before interpreting scoped completion. Preserve a completed current
  period using the atomic snapshot. Apple additionally gates legacy AllDone copy
  with core day truth; stale completed rows get neutral copy and archive access.
- **Status/evidence:** Implemented on Android and Apple with JNI/model/presentation
  regressions and final builds. GTK preserves completed current periods too. Full
  native accessibility/device interaction remains unverified; F-073/PROGRESS.

## B-171 — macOS Settings search result has an incomplete click target

- **Severity:** Low. **Impact:** reproduced in an isolated live Mac demo build:
  clicking result text opened Settings, but the wider row's center did not.
- **Cause/fix:** intrinsic-width plain Button labels inside the task List. Expand
  both Settings search result labels to full-width rectangular hit regions and use
  native borderless button behavior. Footer actions also get separate accessible
  containment/labels and full-width action regions.
- **Status/evidence:** Implemented; final Mac/iOS builds cover the changed shared
  views. Route state is unit-tested. Isolated live Mac recheck confirms row-center
  activation now opens the matching setting. Both separate footer rows independently
  activate the correct period in live Mac inspection. Broader native screen-reader output
  remains separate from this scoped interaction proof; see PROGRESS.

## B-168 — Completion audio is too loud and cannot be auditioned before saving

- **Severity:** Medium. **Impact:** user-reported loudness on the current experience;
  source confirms unattenuated native playback and preview gated on a saved choice
  and automatic playback. Physical loudness was not reproduced on each device.
- **Fix/status:** Implemented on all platforms: quiet original masters, shared local
  gain, single-clip playback, independent candidate previews and preserved legacy/
  unknown choices. Rust portable sound identifiers are now extensible strings in
  the existing format. F-074; native listening remains unverified.
- **Regression evidence:** original/new PCM byte parity, duration/levels/endpoints;
  native recording-player, SoundPool lifecycle, bounded gain and preview/state units;
  portable known/future sound round-trip regression failed before the validator fix.

## B-169 — Completion celebration crosses day/period membership boundaries

- **Severity:** Medium. **Impact:** reproduced by adversarial shared-core unit tests;
  affects all native day-status consumers before the review fix.
- **Reproduction:** finish an unscheduled/future task in another context and open an
  unplanned Today/period; or finish today's Morning work while an overdue Morning
  task remains visible only in Today.
- **Cause/fix:** completion evidence and pending period membership used broad date
  rules. Separate exact-day period membership from whole-day overdue membership for
  active and archived tasks, with timed-date precedence. Atomic listing/status is
  preserved. Four new regressions include failing-before/passing-after evidence.
- **Status:** Implemented across shared consumers. Broader native interactions remain
  outside the unit proof; verification is recorded in PROGRESS.

## B-167 — Linux task entry exposes distracting syntax examples

- **Severity:** Low. **Impact:** user-reported on Linux; source confirms examples
  in the main entry and floating quick-add tooltips/accessibility descriptions.
- **Expected:** a simple “Add a task” prompt matching the requested macOS wording.
- **Fix/status:** Implemented on Linux: both entry placeholders and tooltips use the
  existing EN/DE “Add a task” label; verbose accessibility descriptions are removed.
  Parsing and keyboard actions are unchanged. Other platforms are unaffected by
  this Linux-specific correction. Native Linux interaction remains unverified.
- **Evidence:** source/localization inspection and formatting checks, 2026-10-22
  dirty worktree on `task/native-experience-overhaul`; see PROGRESS.

## 2026-10-22 — day-period sections feature assessment

F-071 extends F-037 with conditional native separators and optional exclusion of
today's Morning/Evening families from Today on all four platforms. Source inspection
at f9637e2 confirms Morning → Today → Evening ordering already exists in Rust; the
requested separator and visibility controls are new capabilities, not a reproduced
ordering defect. That initial assessment assigned no B-ID; the subsequent adversarial
review found B-165 and B-166 below. Regression and native verification limits are
recorded in the dated [PROGRESS.md](PROGRESS.md) handoff.

## B-165 — Hidden day-period tasks intercept Today reordering

- **Severity:** Medium. **Impact:** reproduced in the shared engine, affecting every
  native consumer of Today keyboard/accessibility ordering with day-period tasks hidden.
- **Reproduction:** in manual order, place a hidden Morning task between visible A
  and B in the same group. Nudging A down reports a mutation but leaves visible A/B
  unchanged. Stale drag payloads can also reorder hidden sources or targets.
- **Cause/fix:** ordering used unfiltered Today membership while rendering used the
  new preference. Share preference-aware visible root membership between listing and
  reorder/nudge eligibility; retain unfiltered storage context for correct undo.
- **Regression/status:** red/green core unit tests cover both display directions,
  nudge directions, same-group modes, hidden drag inputs and full stored-order undo.
  Native interaction remains outside the unit-test scope. F-071; final results in PROGRESS.

## B-166 — Android settings-write failure ignores saved local Today visibility

- **Severity:** Medium. **Impact:** source-confirmed Android startup/recovery path;
  physical-device reproduction unavailable on this host. Other native clients apply
  device-local preferences outside portable-settings reconciliation.
- **Reproduction:** persist local visibility off, then reopen while native preferences
  remain readable but portable reconciliation cannot write the settings file. The
  engine continues offline with default visibility on, revealing previously hidden rows.
- **Cause/fix:** the device-local setter ran only after a fallible portable settings
  edit. Apply readable device-local preferences before portable native writes on
  startup and bootstrap retries; retain the settings error until successful recovery.
- **Regression/status:** Implemented. A nonrendering repository regression injects
  native-write failure, checks startup and repeated retry, then permits recovery.
  It is compiled but not device-executed on this host. Host Kotlin/Rust boundary
  tests run separately; final results and limits are in PROGRESS. F-071.

## 2026-10-12 — native feature and release work

Feature evidence began in the isolated task/native-features-release worktree based on 69682872. Final feature source f23df0a and metadata/test correction e67ac9d are verified for the recorded scopes and delivered in 0.4.11; see [PROGRESS.md](PROGRESS.md) for exact-source CI, native limits and release receipts.

## B-164 — Optional install-channel gates allow incomplete release delivery

- **Severity:** Medium. **Impact:** A successful release can silently omit the Mac/Linux Homebrew tap and hosted Linux repository when optional publishing credentials are absent. Mobile distribution is unaffected.
- **Reproduction/cause:** 0.4.11 skipped Homebrew because its token was missing. The hosted signing gate was also optional, and commit failures were swallowed. Built-in workflow-token pushes do not automatically trigger a legacy GitHub Pages build.
- **Fix:** Require publishing credentials and a pinned public signing fingerprint before builds; require both signed architecture exports and the shared checksum-aware Homebrew/hosted workflow. Use a repository-scoped deploy key, explicitly request Pages, reject older versions and propagate Git failures. A retry after a rejected push publishes its existing local commit even if the definitions are unchanged. Remove Flathub submission and archive work per the user's chosen self-hosted channel.
- **Regression/evidence:** Twenty-three isolated Python/Git/workflow regressions and actionlint/shell checks pass; commit/push rejection and retry cases fail before their fixes. Real Linux OSTree verifies both public 0.4.11 versions and signatures, rejects stale versions, a different fingerprint and downgrade attempts. Independent plan/code reviews approve the focused release plumbing.
- **Status/remaining:** Verified for the scoped Linux/macOS release plumbing at 102a978: [Actions run 38488293659](https://github.com/dan-hart/momentum/actions/runs/38488293659) passes all 23 regressions and both shared jobs using existing 0.4.11 assets. Actual SSH authentication/publication rerun, repository write access, explicit Pages build and both public trusted signatures/exact versions pass; Pages reports built. A future new tag/full native release has not been run. Existing native builds, immutable assets, TestFlight and Android delivery are unchanged. F-070.

## B-163 — Self-hosted download page and repository descriptor retain obsolete branding

- **Severity:** Low. **Impact:** The public Linux download page and software-center repository description still mention Super Productivity after the source/native branding cleanup. Other native clients do not consume this Flatpak descriptor.
- **Cause/fix:** These static files live only on gh-pages, outside the main-source scan. Commit 6928b9f changes the tagline in index.html and Comment in momentum.flatpakrepo to the approved native-planner/optional-sync wording. The two-file diff has one replaced line per file. Parsed descriptor keys, URL, public signing key, signed refs, summary and release artifacts are unchanged.
- **Evidence/status:** Verified for hosted metadata: public descriptor/page readback, exact two-line GitHub diff, reachable install/repository links and fresh isolated OSTree pulls for both architectures at 0.4.11 with Good signature. No app/data migration, binary rebuild or new version is required. Related F-067; latest PROGRESS handoff records the user-approved self-hosted channel.

## B-161 — Homebrew packages lag releases and the CLI template fails current audit

- **Severity:** Medium. **Impact:** Mac cask and Mac/Linux CLI remained at 0.4.0 while GitHub shipped 0.4.11. Mobile distribution is unaffected.
- **Cause/fix:** The release has no HOMEBREW_TAP_TOKEN, so its update_homebrew gate is false. Direct authorized publishing uses the existing account. Current stock Homebrew also reproduces two ComponentsOrder errors for the Mac download block and a redundant-version audit error. Put the universal Mac archive at the formula root, infer the version from its literal URL, and retain the native Linux overrides. Actual release URLs and digests are unchanged by the layout correction.
- **Regression/evidence:** Original stock style/audit checks fail with those diagnostics; corrected definitions pass unmodified style, strict all-OS/all-architecture formula audit and strict cask audit. The real Homebrew loader checks all six simulated package selections against public release bytes, downloads both Mac packages, and installs/tests the CLI at 0.4.11. Public tap main is ae3cb0f. The reusable consumer check is build-aux/tests/test_homebrew_packages.rb.
- **Status/remaining:** Verified for package definitions, checksums and recorded Mac CLI installation. Native Linux Homebrew installation and Mac app launch are separate checks. The former missing automation credential is addressed separately by F-070/B-164 with a dedicated tap deploy key; no personal token is copied into Actions.

## B-162 — Flathub initial submission was closed and the release has no app repository

- **Severity:** Medium. **Impact:** Linux Flathub listing is unavailable; signed GitHub Pages Flatpak refs are already 0.4.11. Other platforms do not use Flathub.
- **Cause:** Initial [PR #10133](https://github.com/flathub/flathub/pull/10133) was automatically closed on 2026-09-09 because its submission checklist was missing. No flathub/io.github.dan_hart.Momentum repository exists, and FLATHUB_TOKEN is absent. The release gate is open_flathub_pr=false.
- **Investigation/evidence:** Live repository/PR state and release gate logs are checked 2026-10-13. The existing 0.4.11 archive checksum matches SHA256SUMS; its source pins are Momentum e67ac9d and LibreSync 839c98c/v0.6.0. This verifies a reference artifact, not Flathub eligibility, admission or an offline Flathub build.
- **Status/next:** Planned with user-approved deferral: the user confirms the intended shipping channel is the existing self-hosted signed repository, which is already 0.4.11. Flathub is not a release blocker. Any future admission requires human submission under [current Flathub requirements](https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy). A human must assess and disclose app/packaging provenance, author a compliant manifest and handle the existing closed submission according to its review guidance. No Flathub files, PR body, comments or reopening were changed by this follow-up.

## B-139 — Linux task editor project picker selects the wrong item

- **Severity:** Medium. **Impact:** Linux reproduced in a mapped GTK combo, including nonfirst/duplicate-title projects. Other native pickers are unaffected by this GTK factory bug.
- **Cause/fix:** Factory rendering used the filtered list-item position instead of the original StringList object identity. Resolve the project by model object identity; display/color and saved project ID regression passes.
- **Status/remaining:** Verified on Linux for the recorded scope: mapped duplicate-title/nonfirst project regression, persisted ID/display/color, final EN/DE native probes and both hosted architectures/profiles pass. See final release evidence in PROGRESS.

## B-140 — Linux Preferences is clamped to a short parent window

- **Severity:** Medium. **Impact:** Linux reproduced with a 600 × 400 task window. Other platform settings presentations use different native containers.
- **Cause/fix:** The embedded AdwDialog inherited the small parent allocation. Prefer useful 720 × 700 sizing and independent native presentation for short desktop parents. Hosted solid decorations reduce a requested 600 × 400 parent to 590 × 390, so the initial 600px content-width cutoff still embedded Preferences at 390px. Use libadwaita's native 450px compact boundary. Regression checks cover normal/solid decorations at 600px and 480px, independent modal/transient/resizable roots, unchanged parent allocations, compact scrolling and large embedded presentation.
- **Status/remaining:** Verified on Linux for the recorded scope: ten focused normal/solid-decoration Preferences checks, final 250% native keyboard/scrolling controls, 515-test local suites and both hosted architectures/profiles pass. Full-desktop/spoken acceptance remains outside this scope.

## B-141 — Linux quick entry shows dense guidance inline

- **Severity:** Low. **Impact:** Linux user-reported presentation issue.
- **Cause/fix:** Use concise Add a task… placeholder with native tooltip and accessibility description for entry, tags, estimate and Escape. Parsing/destination context remains in the shared core.
- **Status/remaining:** Verified on Linux for the recorded scope: initialized EN/DE native quick entry, concise placeholder and tooltip/accessibility guidance pass; parser/destination regressions remain intact.

## B-142 — Reported iOS project view spinner needs a second tap

- **Severity:** Medium. **Impact:** iOS user-reported and now reproduced on 2026-10-13 using actual simulator touch through Lists → Home, on both a prior 0.4.9 build and current-source disposable app. First tap leaves Loading tasks; returning to Lists and tapping again loads the tasks. Owned iOS 26.5 shipping demo loads Search → Home on one tap; first compact sidebar and controlled delayed snapshot tests pass. Android first/rapid/deleted-project boundary tests pass. GTK/macOS list queries are synchronous.
- **Cause/fix:** Actual touch causes the new project screen to disappear and its completed snapshot to be cancelled while an old Today screen reappears; no replacement load starts. Hosted accessibility activation did not reproduce that transition. Move route identity to the enclosing NavigationStack, so the stateful navigation container owns the selected route. Stale results remain cancelled/rejected. The same opt-in real-touch regression fails before the change and passes on iOS26.5/27 after it, with first selection, reentry, project switching and Today return. Hosted current/stale load regressions pass.
- **Status/remaining:** Verified on iOS for the recorded scope: actual 0.4.10/410 first-touch/reentry/switch/Today routes and full 113 hosted views (three existing skips) pass on 26.5/27; independent review approved. GTK/macOS synchronous queries and Android first/rapid/deleted-project regressions do not share this navigation-container cause. Other-platform assessments remain scoped to tested paths.

## B-143 — iOS share-sheet capture has no Today plan

- **Severity:** Medium. **Impact:** iOS existing share import left new tasks unplanned. Other platforms do not use this extension queue.
- **Cause/fix:** Capture the local Gregorian day on Add, persist optional plannedDay, and import through the existing Rust draft rule. Legacy queued items preserve their missing day. Midnight/timezone/retry/dedup and hosted Today import tests pass.
- **Status/remaining:** Verified on iOS for the recorded scope: captured-day midnight/timezone/retry/dedup and hosted Today import tests, both supported simulator lanes and the signed 0.4.11/411 shipping extension pass. Physical share-sheet use remains a device QA limit.

## B-144 — Upcoming omits future recurring occurrences

- **Severity:** Medium. **Impact:** Shared listing only read persisted tasks, affecting all native clients.
- **Cause/fix:** Shared read-only recurrence projection supplies preview metadata, respects horizon/start/paused rules and suppresses existing/moved/archived IDs without task writes. Native rows expose only Edit repeat schedule.
- **Status/remaining:** Verified on all platforms for the recorded scope: pure projection/core regressions, final native read-only row/schedule action and large-text layouts, consumer builds and tagged artifacts pass. Physical/spoken limits remain in PROGRESS.

## B-145 — Copy Notes lacks acknowledged success feedback

- **Severity:** Low. **Impact:** User-reported gap across platforms.
- **Cause/fix:** After successful full copy, show editor-local toast and native Apple/Android haptic. GTK provides accessible visual feedback. Exact payload, modal visibility, failure/disabled silence and repeated deadlines are tested.
- **Status/remaining:** Verified on all platforms for the recorded scope: exact Unicode/multiline clipboard, editor-visible feedback, failed/disabled silence and repeated-deadline tests pass. Native Apple/Android dispatch is tested; physical vibration and spoken output are not inferred.

## B-146 — Native settings changes can replace an unseen synced key

- **Severity:** High. **Impact:** Reproduced during feature implementation on Apple and GTK; Android received the same boundary regression.
- **Cause/fix:** Stage only local native deltas, read core status before its values, reject stale asynchronous applications and update device summary fields through an atomic core API. Preserve equivalent unsupported native fallbacks. Unseen-sort/local-color tests pass; Android raw-sound preservation repair passes its native regressions and independent review.
- **Status/remaining:** Verified on all platforms for the recorded scope: unseen-key/atomic publication/raw-sound regressions, final native consumers and tagged packaging pass.

## B-147 — Fresh native stores cannot seed an empty Nextcloud folder

- **Severity:** High. **Impact:** Public Engine loopback reproduction failed for empty, local-task and reimported native-backup stores because absent opaque globalConfig triggered FreshState.
- **Cause/fix:** Ordinary sync fills absent globalConfig with an empty object only in the detached candidate, preserving existing opaque values. Successful upload persists it; failure/cancel/damage/encryption/unsupported/conflict/persistence cases preserve live data and pending work. Lower-level fresh-state and explicit replacement protections remain.
- **Status/remaining:** Verified in shared core and existing consumers for the recorded scope: fresh/empty/local/reimport loopback tests and failure/cancel/damage/encryption/conflict/persistence guards pass; independent review and final platform builds approve the unchanged task format. External hosted-server acceptance remains separate.

## B-148 — Damaged local snapshot can be treated as an empty store

- **Severity:** Critical. **Impact:** Public checked-startup harness reproduced malformed JSON and structurally incomplete {} snapshots becoming empty, allowing bootstrap to acknowledge real pending work. This was detected while verifying B-147.
- **Cause/fix:** Present unreadable/malformed state/pending/meta fail startup with privacy-safe file errors. Task collection ids/entities are structurally required; absent snapshot plus task history/pending data also fails. Preserve all original bytes. Legitimate fresh/settings-only stores and optional legacy/opaque fields remain supported.
- **Status/remaining:** Verified in shared storage and existing consumers for the recorded scope: public failure/positive harnesses preserve damaged bytes; store/core tests and all final native builds pass.

## B-149 — Cancelling folder-copy ownership can lose cancellation or service recovery

- **Severity:** Medium. **Impact:** New iOS migration tests reproduced Cancel during factory admission being ignored and connection Save being accepted during copy. Android review found cancelled caller context prevents resuming automatic foreground sync.
- **Cause/fix:** iOS latches cancellation before operation creation and blocks Save while copy owns the connection; cancellation, stale/save-failure boundaries pass. Android resumes automatic foreground ownership in a non-cancellable recovery boundary, with native regression and independent approval.
- **Status/remaining:** Verified on iOS/Android for the recorded scope: cancellation admission, Save exclusion, stale/save-failure and automatic-service recovery regressions pass. GTK/core original-connection preservation and final native consumers also pass; external-server/physical limits remain explicit.

## B-150 — Opening a GTK repeat editor invents a start date

- **Severity:** Medium. **Impact:** Mapped GTK regression reproduced saving an existing paused schedule with no start day assigning today.
- **Cause/fix:** Only the calendar selection updates the start day; add an accessible Clear start day action. Opening/saving no longer mutates the anchor.
- **Status/remaining:** Verified on Linux for the recorded scope: original RED/GREEN, localized native schedule action/open/cancel and both hosted architectures/profiles pass; saving preserves the absent anchor.

## B-151 — macOS portable settings changes leave the visible list stale

- **Severity:** Medium. **Impact:** macOS reproduced by the existing grouping test. Other platforms refresh through their own native adapters.
- **Cause:** Staging changes the core before the old equality check, so the check no longer detects the changed listing preferences.
- **Fix/status:** Rebuild the visible listing after portable preferences apply. Verified on macOS for the recorded scope; the original grouping regression and native remote-apply regression pass. Final shipping and consumer gates pass; physical/spoken/external-server limits remain in PROGRESS.

## B-152 — macOS settings-save failure loses its visible error

- **Severity:** Medium. **Impact:** macOS independently reproduced using a denied temporary settings-file write.
- **Cause:** Rollback succeeds, then forced native refresh clears the failure immediately.
- **Fix/status:** Retain the rejected partial intent and its error through no-op notifications. A local Retry saves that intent without replacing unseen keys. Bootstrap choices wait for local recovery and task-sync errors have separate actions. Verified on macOS for the recorded scope; native regressions pass and specification review approved. Final shipping and consumer gates pass; physical/spoken/external-server limits remain in PROGRESS.

## B-153 — Failed initial native settings seed exports an empty document

- **Severity:** Medium. **Impact:** macOS independently reproduced; the same unconditional baseline assignment is present on iOS, with equivalent missing full-seed retry on GTK. Android retains its bootstrap flag and is unaffected in the reviewed scope.
- **Cause:** Native adapters mark their values applied despite a failed durable full seed. The next export stages an empty delta.
- **Fix/status:** Mark a baseline only after successful full seeding. Native Retry/edit/export/consent recover all nine values, with visible local errors. Core validates and commits settings-only or opted combined imports before native adapters supersede rejected intent. Invalid imports preserve native/core values, pending Retry and durable bytes; valid partial imports retain missing choices and cannot replay old intent. Task-only restore remains usable. GTK captures the imported revision before applying callbacks so later commits remain observable. Verified on Mac/iOS/GTK for the recorded scope; Mac15/iOS18 and GTK atomicity/callback regressions pass with fresh specification and independent quality approval. Final shipping and consumer gates pass; physical/spoken/external-server limits remain in PROGRESS.

## B-154 — Failed Linux legacy-folder pin marks migration complete

- **Severity:** High; existing remote connection could be redirected. **Impact:** Linux reproduced with a denied folder-write fixture. Native Apple/Android adapters use separate preservation logic.
- **Cause/fix:** The marker was written after an ignored folder-save failure. Propagate the error, save the marker only after success, and use the safe effective legacy folder in transport, CLI export and Preferences until pinning succeeds.
- **Regression evidence:** The failure test was red with the marker incorrectly set and passes after the repair, including retry. Task 7 specification review approved the preservation contract on 2026-10-12.
- **Status/remaining:** Verified on Linux for the recorded scope: original denied-write RED/GREEN/retry, final full native suites and both hosted architectures/profiles pass.

## B-155 — Android optional settings failure blocks local device controls

- **Severity:** High; turning off local sync or notifications must stay usable. **Impact:** Isolated API26/API37 tests reproduced local controls being blocked after a denied initial portable seed. Mac/iOS/GTK keep local summary/device writes separate from portable failure in the reviewed paths.
- **Cause:** `updateSettings` and `updatePreferences` guard portable application even for transformations that only change device controls.
- **Fix/status:** Compute the nine-key portable delta before guarding or staging. Device-only controls persist and refresh local summary behavior while retaining the portable error/recovery state; portable edits/transfers remain guarded. Verified on Android for the recorded scope, with ten repository tests passing on API26/API37 and specification/independent quality approval. Full native API26/API37 suites and final shipping gates pass; physical/spoken limits remain in PROGRESS.

## B-156 — Recurring previews copy full notes for every date

- **Severity:** High; long notes cause avoidable memory growth. **Impact:** Isolated public-core fixture with one daily schedule and 30 previews retained about 128.52 MiB additional peak memory for a 4 MiB note, while returning only 2,490 snippet bytes. This is measured amplification, not a reproduced out-of-memory crash.
- **Cause/fix:** Each projected date cloned the complete template note. Bound notes once on cloned preview configurations using the existing snippet rule; stored templates and materialized tasks retain full notes and metadata.
- **Regression evidence:** The projection test failed on the original full-note copies and now verifies bounded Unicode/multiline/blank snippets, unchanged state/cursors and full-note materialization. Both Rust workspace lanes pass 396 tests with five existing ignores.
- **Status/remaining:** Verified in the shared core and native consumers for the recorded scope; independent review approved. The same bounded 4 MiB/30-preview fixture now peaks at 15.97 MiB versus 136.33 MiB before, with identical output and unchanged task/pending/cursor checks. These are individual synthetic desktop samples. All final native consumer/release gates pass.

## B-157 — macOS recurring preview compresses large text and hides its callout

- **Severity:** Medium. **Impact:** Reproduced in the current native demo at minimum 680px width, German, and 32pt content/interface sizes. The trailing schedule button steals row width and the preview callout clips inside its combined subtitle. Other platforms use distinct preview layouts and remain assessed separately.
- **Cause/fix:** Put the schedule action below the full-width row, explicitly apply native title/action typography, and render the preview callout on its own wrapping line. Keep the shared accessible subtitle and read-only action restrictions.
- **Status/remaining:** Verified on macOS for the recorded native scope: rebuilt universal Release app, German 32pt/680px full callout/action, action activation/cancel and English first-project/preview/settings regression pass. Row 346pt contains child 338pt; catalog 358/358 German entries passes. Physical/spoken/network limits remain in PROGRESS.

## B-158 — iOS recurring preview callout disappears in clipped large-text metadata

- **Severity:** Medium. **Impact:** Reproduced in the 0.4.10/410 demo on iOS27, compact phone, German and maximum accessibility text. The title/project metadata ellipsis hides the appended preview label. Android already uses a separate label; GTK has a dedicated preview presentation; B-157 records the distinct Mac layout defect.
- **Cause/fix:** Give Recurring preview its own wrapping caption and omit only its duplicate visual metadata part. Keep the complete shared accessibility value and schedule-only action.
- **Status/remaining:** Verified on iOS for the recorded scope: rebuilt 0.4.10/410 native German maximum-text screenshot shows the full wrapping callout and schedule action; both 113-test hosted view suites and first-touch routes pass again. Compiled German 442/442 entries and shared/app strings pass. Physical/spoken limits remain in PROGRESS.

## B-159 — Linux recurring previews overflow compact large-text windows

- **Severity:** Medium. **Impact:** Reproduced with initialized English/German resources at 250% in a 440 × 710 native window. The text schedule suffix expands the preview list beyond the viewport, squeezing the title and callout; German's action is offscreen. Other platforms use separate native presentations; B-157/B-158 record their repaired large-text cases.
- **Cause/fix:** The long text suffix contributes an unbounded minimum width. After replacing it with a native symbolic edit button carrying the complete localized tooltip and accessible name, the hidden selection ActionBar still forces a 602px toolbar width. Hide inactive bulk controls from layout, while retaining their existing active/reveal behavior. Preview actions still open only the repeat schedule.
- **Status/remaining:** Verified on Linux for the recorded native scope. The original mapped regression fails at 571px in the 440px viewport; the repaired GTK lane passes title/callout wrapping, action bounds/focus/open/cancel, exact store/pending/metadata preservation, and real selection entry/bulk/cancel/route-exit visibility. Final X11 and Broadway all-feature/all-target suites each pass 515 tests (115 GTK) with six existing ignores. Initialized English/German rerenders and independent specification/quality review pass; German row 419×262 fits the 440×710 root with complete text and an actual working schedule action. All four final hosted profile/architecture gates and tagged packaging pass.

## B-160 — Linux settings actions notify a disposed native window

- **Severity:** High. **Impact:** Reproduced in the candidate ARM GTK suite and on an owned X11/Xvfb runtime with MALLOC_PERTURB_=26, including one test thread. A supported native close probe retaining a menu/popover reports an invalid action group when a GSettings-backed action changes after its window closes. This is a native GTK action-group lifetime defect, separate from B-159 and the shared settings rules.
- **Cause/fix:** A retained menu observer prolongs the native action group after window disposal, while its settings-action callback still refers to the old window. Detach window-owned actions before the automatically chained parent disposal. Retain normal live-window/background-hide behavior and the existing rejected-settings retry regression.
- **Status/remaining:** Verified on Linux for the recorded native scope. Exact original `7e9517e` exits with SIGSEGV in the unchanged rejected-settings retry fixture; adding only the guarded action detach makes all 112 original GTK tests pass under default and single-threaded X11/MALLOC_PERTURB_=26. Final X11 and Broadway all-feature/all-target suites each pass 515 tests (115 GTK), six existing ignores. Actual shipping task-menu retained close/repeated-destroy and live background action/engine guards pass in initialized EN/DE native probes; these added guards preserve behavior and are not claimed as individually failing before the fix. No action-group or unpresented-dialog critical remains. The repeat-Save fixture now verifies the production handler closes its dialog, removing a redundant second close while retaining its start-date assertion. Independent specification and quality reviews approve the final source. All four final hosted profile/architecture gates and tagged packaging pass. Other platforms use separate native action lifetimes; no shared-core defect is inferred.

## 2026-10-11 — F-056 completion-sound choices assessment

This is a requested enhancement to feedback and settings, rather than a reproduced
product defect. It replaces the former single clip with ten selectable CC0 clips on
all four native platforms and adds Android’s optional sound setting. Default,
persistence, fallback and completion-only regression evidence is recorded in
[PROGRESS.md](PROGRESS.md). Existing unrelated defects retain their own status;
audible output and physical-device acceptance are separate verification gates.


## B-138 — Android CI cannot find the SDK manager

- **Severity:** Medium. **Feature:** F-060/F-056 verification. **Impact:** reproduced
  in the Android source-build run on main `4ffb52a` (2026-10-11).
- **Reproduction:** the pinned Rust/Android package step invokes bare `sdkmanager`;
  [run 38117069859](https://github.com/dan-hart/momentum/actions/runs/38117069859)
  exits 127 before Android compilation because that executable is absent from PATH.
- **Expected:** the existing runner SDK can install the pinned Android packages and
  reach build/test validation. **Fix:** invoke the manager from the runner’s existing
  `ANDROID_HOME/cmdline-tools/latest/bin` directory. No toolchain pin is changed.

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Not applicable — Android CI setup | Not applicable — Android CI setup | Not applicable — Android CI setup | Verified — SDK setup and source build passed on `74a2a8b` |

- **Evidence:** failure log inspected live on 2026-10-11. The corrected package
  setup and full unsigned source-build validation passed on `74a2a8b` in
  [run 38161413954](https://github.com/dan-hart/momentum/actions/runs/38161413954),
  including release APK/AAB, JVM tests, lint, notices and native alignment.

## 2026-10-10 — internal TestFlight preparation assessment

F-059 changes the iOS app and share-extension identifiers for the requested App Store
Connect record. Source inspection also found missing required-reason API manifests
(B-104). Signed packaging, Apple processing, and internal distribution are verified in
[PROGRESS.md](PROGRESS.md). Existing native interaction
and sync acceptance gaps retain their individual statuses.

## B-104 — iOS app and share extension omit privacy manifests

- **Severity:** Medium. **Feature:** F-059. **Impact:** missing packaging reproduced
  by source inspection; App Store Connect rejection has not been reproduced.
- **Reproduction:** inspect the 0.4.9 (`1ade8ab`) iOS targets: neither includes a
  `PrivacyInfo.xcprivacy`, although the app reads/writes its preferences and the
  linked Rust library references `stat`, `fstat`, and `lstat`.
- **Expected:** each shipping bundle declares the reasons for its required-reason
  APIs before TestFlight upload. The app uses private preferences, private/group files,
  and user-selected backup files; the share extension writes its App Group inbox.
- **Fix:** add native privacy resources to the app and extension. App preferences use
  `CA92.1`; private/group file metadata uses `C617.1`; user-selected backup metadata
  uses `3B52.1`. No tracking or developer data collection is added. Reason meanings
  were checked against [Apple's API reference](https://developer.apple.com/documentation/bundleresources/app-privacy-configuration/nsprivacyaccessedapitypes/nsprivacyaccessedapitype).

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Not applicable — no Apple bundle | Not applicable — this request concerns the iOS TestFlight targets; desktop distribution is separate | Verified — signed app/extension manifests packaged and Apple processing accepted | Not applicable — no Apple bundle |

- **Evidence:** both manifest property lists pass `plutil -lint` in the dirty
  `task/ios-testflight` worktree on 2026-10-10. The successful unsigned device Release
  archive and final signed IPA package and parse both manifests. Binary imports agree
  with the declared preference/file API scope. Apple accepted 0.4.9 (409), reports
  VALID processing and IN_BETA_TESTING, and the internal group has build access. This
  verifies manifest packaging/server acceptance; physical installation and native
  feature QA remain separate.

## B-001 — Linked LibreSync devices do not populate the Mac

- **Severity:** High. **Feature:** [F-026](FEATURES.md).
- **Reported behavior:** Linux receives data but the linked macOS app does not show it.
- **Expected:** valid changes from the selected LibreSync peer reach the shared store
  and refresh the native UI, or an actionable connection error explains the failure.
- **Reproduction:** link Linux/Mac, select LibreSync, create a task on Linux, trigger
  sync and inspect the Mac. Real-device acceptance is currently deferred by the user.

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Blocked — receiving worked per user; sending/peer connectivity not fully verified | Implemented — receiving symptom reported; changes made, physical reproduction/acceptance deferred | Not applicable — app not implemented | Not applicable — app not implemented |

- **Findings/fix:** initial bootstrap must remain pending until a valid snapshot;
  bridge progress/failures must reach native state; disposable demo data must never
  sync. These fixes and isolated regressions are recorded in the
  [Mac acceptance history](MACOS-MVP-CHECKLIST.md). A linked peer also timed out over
  the network; the root cause of the original real-device symptom is not conclusively
  proven by passing loopback tests.
- **Evidence:** prior two-node Swift/UniFFI integration checks and shared-core
  regressions passed; details and dates are in that history. This documentation pass
  did not rerun them.
- **Next:** resume only when the user lifts the real-device deferral; verify both
  directions with isolated test data and confirm UI state plus durable-store readback.

## B-002 — macOS quick-add consumes multiline paste as one input

- **Severity:** Medium. **Feature:** F-015. **Resolved scope:** native Mac quick-add.
- **Reproduction:** paste two task lines into quick-add. Previously the native field
  consumed Paste before the SwiftUI handler and retained newlines in a single input.
- **Expected:** create separate tasks for multiline input while ordinary paste replaces
  the native selection without corrupting cursor/selection behavior.

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Not applicable — this reproduced defect was in the AppKit text-field path | Verified — fixed and live-tested 2026-09-16 | Not applicable — app not implemented | Not applicable — app not implemented |

- **Fix:** observe multiline text changes in native quick-add and route them through
  shared text import; retain the editor's normal single-line behavior.
- **Evidence:** focused import regression, successful Debug build, and native
  two-task creation plus ordinary selection replacement recorded under “F5 and native
  paste” in [MACOS-MVP-CHECKLIST.md](MACOS-MVP-CHECKLIST.md).
- **Regression scope:** the common parsing contract applies to all future frontends;
  the Not applicable cells do not waive F-015 for mobile.

## New bug template

Use `B-<next ID>` with: severity; linked feature; report date; environment/revision;
reproduction steps; expected versus actual; separate status and impact for each OS;
cause (or explicitly unknown); fix; test/build/native evidence; remaining checks and
next action. Record a blocker and last completed stage rather than closing an issue
because its reproduction environment is unavailable.

## 2026-09-16 — F-037 regression assessment

Group By introduces no newly observed defect in the existing bug entries. Shared
regressions cover task preservation, parent/subtask placement, archive/search limits,
and reorder undo; native Mac checks cover menu changes and colored headings. Linux
runtime and additional grouped native drag/accessibility checks remain verification
gaps in [PROGRESS.md](PROGRESS.md), not assumed bug fixes. The existing LibreSync and
Spotlight deferrals are unchanged.

## B-003 — Dual time-of-day tags prevent a move to Evening

- **Severity:** Medium. **Feature:** F-037. Reproduced in the shared Rust core during
  the exclusive-grouping revision on 2026-09-16.
- **Reproduction:** create a task tagged both Morning and Evening. Morning takes
  precedence when displaying it. Drop it onto Tonight, or use the Evening toggle.
- **Expected:** it moves into Evening, removing opposing Morning tags; undo restores
  the original tags and grouping.
- **Actual:** drop was treated as already in Evening because the raw tag existed;
  toggle removed Evening while leaving the task in Morning.
- **Cause/fix:** use the resolved day period for drop/toggle decisions, and remove
  all matching opposing tag IDs by case-insensitive name. Listing, sidebar membership
  and grouping resolve the same names and Morning precedence.
- **Platforms:** Linux/macOS Implemented (shared-core reproduction and regression);
  iOS/Android Planned (consume the same core when native apps exist).
- **Evidence:** regression covers both-tag precedence, mixed-case names, parent/subtask
  placement, forced drop, toggle, regrouping and undo. Native gesture acceptance is not
  inferred from core tests; see the dated verification in PROGRESS.md.

## 2026-09-16 — F-038 regression assessment

The automatic morning summary is now an explicit opt-in feature (F-038), replacing
its prior specified behavior rather than treating it as a newly reproduced bug.
Regression coverage preserves task reminders, suppresses summaries while off, and
prevents duplicate daily claims after restart or preference edits. Mac adapter gating
also prevents consuming a summary before a notification adapter is attached. Existing
bug statuses and user-deferred checks are unchanged; native verification limits remain
in [PROGRESS.md](PROGRESS.md).

## 2026-09-16 — F-019 macOS task-row alignment

User-requested visual refinement: the row previously aligned its checkbox with the
first text baseline. It now centers the checkbox, archived checkmark and trailing
badges against the full title/metadata stack. This is a macOS-only presentation change;
no shared task rules or other platforms changed. Build and native two-line-row
inspection passed; no additional functional defect was observed.

## B-004 — iOS quick entry crashes on stale text-selection indices

- **Severity:** High. **Features:** F-005/F-015. **Impact:** reproduced on the iOS 27
  simulator in the new dirty iOS implementation; Linux/macOS paths are unaffected,
  Android has no corresponding native implementation.
- **Reproduction:** open Add task while SwiftUI publishes a text selection belonging
  to a different text update. The autocomplete cursor calculation used that index
  directly in the current string's Unicode-scalar view and trapped on the main thread.
- **Expected:** quick entry remains usable across focus changes, text replacement,
  emoji, selection and IME updates; native editing retains its cursor.
- **Cause/evidence:** both 2026-09-16 native runs produced EXC_BREAKPOINT in
  `_StringGuts.validateInclusiveScalarIndex` → `QuickAddSheet.cursor`. Result bundle
  `/tmp/momentum-ios-editor-red-v2.xcresult`; isolated test store, base `7ec82e9` dirty.
- **Fix:** validate index bounds and convert with `samePosition(in:)` before scalar
  distance; fall back to the end when the selection is stale. Added stable field
  identifier and a Unicode replacement native regression.
- **Status:** iOS Implemented, with scoped iOS 27 native regression verified by
  `testQuickAddReplacesUnicodeTextWithoutCrashing` in
  `/tmp/momentum-ios-editor-green.xcresult` (2026-09-16, same dirty worktree).
  iOS 26 and IME acceptance remain outstanding.

## B-005 — iOS palette labels lose contrast in native rendering

- **Severity:** Medium. **Features:** F-019/F-020. Reproduced in isolated iOS 26.5
  and 27 simulator acceptance on 2026-09-16, dirty `task/ios-app` at `7ec82e9`.
- **Reproduction:** choose a custom accent in Settings → Accent Color. The native
  button style tinted primary/secondary label hierarchies; small hex values became
  pale. The floating tab bar's soft scroll-edge treatment also faded nearby rows.
- **Expected:** readable color names, values and controls in light/dark/increased
  contrast, with selection conveyed by a checkmark and accessibility state.
- **Fix in progress:** explicit system primary text; contrast-adjusted secondary
  text and swatch outlines; native hard top/bottom scroll edges. Exact base values remain
  unchanged, while rendered tones adapt against native surfaces.
- **Evidence:** palette-wide numerical tests pass. Native screenshot/audit in
  `/tmp/momentum-ios26-accent-stable.xcresult` identifies the lower “Wet Asphalt”
  label. Subsequent hard-edge capture shows the label clearly but the audit still
  flags partially occluded content. Fully visible preview and appearance checks
  remain in progress; this is not a blanket accessibility acceptance.
- **Platforms/status:** iOS Implemented with verification outstanding. Linux/macOS
  unaffected by this SwiftUI mobile palette path; Android has no corresponding UI.

## B-006 — iOS Repeat and Duplicate ignore unsaved task edits

- **Severity:** Medium. **Features:** F-006/F-008. Reproduced in iOS 27 native tests,
  dirty `task/ios-app` at `7ec82e9`, 2026-09-16.
- **Reproduction:** change a task title, then choose Repeat or Duplicate before Save.
  Previously the action used the old persisted title; cancelling the outer editor
  discarded the draft while leaving the newly created repeat/duplicate behind.
- **Fix:** validate and save the current draft first, refresh stored form/tag values,
  then perform the action. Footers explain the save-first behavior. Invalid forms
  remain disabled and errors keep the editor open.
- **Evidence:** both action regressions failed before the fix and passed afterward
  in `/tmp/momentum-ios-editor-draft-green.xcresult`; five editor tests and one launch
  test passed. Its separate accent audit failed (B-005), so the combined run is not
  reported as green. Independent source review passed.
- **Platforms/status:** iOS Implemented; scoped iOS 27 native regression verified,
  iOS 26 still pending. Desktop paths unaffected; Android not implemented.

## B-007 — iOS task drag type is not declared in the app manifest

- **Severity:** Low. **Features:** F-011/F-014. The iOS 26.5 native run logs that
  `com.codedbydan.momentum.tasks` is exported by the shared Transferable type but
  absent from the app manifest when a draggable row appears. Actual gesture failure
  has not been reproduced; macOS already declares this type, Linux is unaffected,
  and Android has no corresponding native implementation.
- **Fix:** mirror the existing macOS exported type declaration in the iOS XcodeGen
  specification; retain the shared payload and public-data conformance.
- **Status/evidence:** iOS Implemented, dirty `task/ios-app`, 2026-09-16;
  warning in `/tmp/momentum-ios26-accent-hard-edges.log`. The corrected simulator
  app built and its exported-type declaration was read back from the built Info.plist
  (`/tmp/momentum-ios26-accent-readable-headings.log`). Native drag/drop acceptance
  remains outstanding.

## B-008 — iOS Quick Add dismisses after an unsuccessful mutation

- **Severity:** Medium. **Features:** F-002/F-005. iOS source-confirmed: the capture
  sheet unconditionally dismissed after awaiting add/import, ignoring Outcome.changed.
  Desktop consumers are not changed; their analogous behavior was not reproduced.
- **Expected:** keep entered text and show actionable failure feedback when no task
  was saved. **Fix:** native model exposes the core mutation result, Quick Add only
  dismisses on change, restores focus and retains draft text otherwise. No task rules
  were duplicated in Swift. **Status:** Implemented; scoped iOS 27 rejection/retry and successful-save native
  regressions passed in `/tmp/momentum-ios-flow-polish-v2.xcresult` on 2026-09-16,
  dirty `task/ios-app` worktree. Test uses an estimate-only input rejected by the real
  core, verifies the retained draft and retries successfully. The same regression also passed in the full 10-test iOS 26.5 dark-mode suite
  (`/tmp/momentum-ios26-flow-polish.xcresult`). Disk-failure injection remains separate.

## B-009 — iOS toolbar overrides a filled action's contrasting symbol color

- **Severity:** Medium. **Features:** F-019/F-020. Reproduced by native screenshot
  inspection on iOS 26.5 dark mode after the 10-test flow suite passed. Edit any task:
  Save's checkmark is pale on #FF6600 despite a black foreground style on the Button.
- **Expected:** readable contrasting ink on the filled Create/Save action while
  retaining the exact dark default accent. **Cause:** native toolbar promotion can
  override foreground styling on the enclosing Button.
- **Fix:** explicit palette rendering/foreground on the action image, retaining the
  accessible localized button label and native prominent style. **Status:**
  Verified for the iOS 26.5 dark editor/commit scope: final build and two native
  capture/retry + edit/delete/undo tests passed (`/tmp/momentum-sheet-ink-fix.xcresult`),
  and screenshot inspection confirms a black checkmark against the orange
  fill. iOS 27 light recheck also passed in `/tmp/momentum-sheet-ink-ios27.xcresult`;
  screenshot inspection confirms white ink on the darkened orange. Original source screenshot:
  `/tmp/momentum-ios26-polish-evidence/04A74F39-4FC5-412A-B16E-C9CFCAFACFEC.png`.
- iOS-specific toolbar component; other frontends do not use this implementation.


## B-010 — stale notification Snooze mutates completed tasks

- **Severity:** Medium. **Features:** F-007. Reproduced with a shared-core regression:
  create a task, snooze it, complete it, then invoke Snooze from the stale notification.
  Before the fix, the operation reported success and changed the reminder timestamp.
- **Expected:** completed, archived, deleted or missing tasks reject Snooze without
  changing their reminder. **Cause:** the shared operation checked existence only.
- **Fix:** the Rust engine rejects completed tasks, retaining the existing missing/
  archived guard. Applies to Linux, macOS and the new iOS notification action adapter;
  no frontend duplicates task eligibility rules. Android has no consumer yet.
- **Evidence:** failing regression observed before the guard; full all-feature/all-target
  core/CLI workspace checks passed afterward (excluding GTK app), and final focused
  regression covers actual deletion too. Logs `/tmp/momentum-notification-native-core.log`
  and `/tmp/momentum-stale-snooze-final.log`, 2026-09-16 dirty `task/ios-app`.
  Native stale-notification interaction on each OS remains unverified.

## B-011 — disabling a summary while permission is denied leaves its pending request

- **Severity:** Medium. **Features:** F-007/F-038. iOS coordinator regression:
  a future accepted summary exists, OS permission is denied, then summary is disabled.
  The unauthorized branch only reported the plan and skipped its cancellations.
- **Expected:** remove the exact obsolete request without adding notifications,
  requesting permission, or consuming a core scheduling claim. **Fix:** apply and
  verify the read-only plan's cancellations even when scheduling is unauthorized.
- **Evidence:** regression failed before the change (`/tmp/momentum-notification-denied-red.log`);
  all 52 mobile tests / nine suites passed afterward (`/tmp/momentum-notification-final-mobile.log`),
  2026-09-16 dirty `task/ios-app`. On 2026-09-18, shipping-app simulator tests on
  iOS 26.5/27 denied real authorization and verified the disabled recovery UI; separate
  fresh-grant runs verified `Allowed` plus the native planning horizon. The current
  Simulator exposes no notification service through `simctl privacy`, and its Settings
  switch rejected cross-app XCTest input, so the uninterrupted denied-to-enabled
  transition remains manual acceptance. This coordinator is iOS-specific; desktop
  reminder delivery uses separate adapters.

## B-012 — task editor presentation emits an invalid-frame warning

- **Severity:** Low; runtime diagnostic reproduced, visible impact not established.
  **Features:** F-020. Opening task editors in native iOS 26.5/27 tests emits
  `Invalid frame dimension (negative or non-finite)` during sheet presentation.
- The warning predates notification wiring: it is present in
  `/tmp/momentum-ios26-flow-polish.log` and `/tmp/momentum-sheet-ink-ios27.log`, and
  recurs in `/tmp/momentum-notification-final27.log`. Recorded editor interactions
  and inspected screenshots have not demonstrated a related functional failure.
- **Cause:** SwiftUI's `.keyboard` toolbar placement emits the warning as soon as
  any keyboard toolbar item is inserted for the presented editor. Removing the whole
  keyboard toolbar removes the warning; removing its symbol, spacer or item group does
  not. Conditional insertion moves the warning to the first field focus, confirming
  the framework accessory path rather than task data or sheet geometry.
- **Fix:** replace the `.keyboard` toolbar with a keyboard-aware bottom safe-area bar
  inside TaskEditor. It appears only while a field is focused, retains the SF Symbol
  Done action, uses native bar material, and gives the actual button label a 44-point
  minimum accessibility frame.
- **Status/evidence:** Verified for the iOS 26.5/27 simulator scope. The real
  open/focus/edit/save/reopen/copy flow passes without the warning on iOS 26.5
  (32.037s, `/tmp/momentum-b012-keyboard-layout26-final2.xcresult`) and iOS 27
  (33.339s, `/tmp/momentum-b012-keyboard-layout27-final.xcresult`). Assertions prove
  the action exists, is hittable, is at least 44 points tall and stays above the
  software keyboard. The retained iOS 26.5 screenshot was inspected. Final portable
  and native-view suites pass. macOS/Linux are unaffected; Android has no UI consumer.

## 2026-09-16 — F-019 curated accent assessment

The user requested AsNeeded’s nine-color selection instead of the full country-based
DHFlatUIColors catalog. This is an iOS product refinement, not a new reproduced defect.
Existing default/contrast safeguards remain; regression checks cover retained and retired
saved choices. macOS/Linux use their platform accents and are unaffected. Native evidence
for the reduced picker is recorded in [PROGRESS.md](PROGRESS.md).

## 2026-09-16 — exact default orange decision (F-019; B-005/B-009 scope)

The user explicitly replaced light-mode darkening with exact #FF6600 in both
appearances. Adaptive custom-color and neutral-text fixes remain; default-orange
text on light backgrounds no longer carries the prior contrast-pass claim. Filled
Create/Save symbols now derive black/white ink from the actual rendered accent
instead of assuming white in light mode. This preserves B-009’s toolbar-level
foreground fix while accommodating the brighter default. No new reproduced defect.

## 2026-09-16 — iOS accessibility audit findings

All entries below are from source inspection and isolated native simulator audits
of dirty `task/ios-app` based on `7ec82e9`. No production fix is included in this
audit change. The [audit report](audits/2026-09-16-ios-accessibility.md) records raw
results, screenshots, reproduction, candidates and remaining assistive testing.
Linux/macOS were not reassessed; these iOS findings do not change their statuses.
Android is Planned and has no implemented UI to reproduce these defects.

## B-013 — exact default orange has insufficient foreground contrast

- **Severity:** High. **Features:** F-018/F-019/F-020. **iOS:** In progress remediation.
- **Reproduce:** default accent, light appearance; inspect selected picker values,
  tab text and tint-colored actions. #FF6600 against white is 2.936:1, below
  Apple's text guidance. The default also remains exact with Increase Contrast.
- **Cause:** the latest user decision intentionally keeps the base orange exact;
  using the same token for all foreground roles creates the contrast deficit.
- **Expected/fix:** retain the approved brand/fill color while defining readable
  semantic foreground roles consistent with the latest approved button styling. No silent
  reversion to the previously rejected darker orange. See A-01.
- **2026-09-17 user steering:** All default-orange filled buttons now use white text and
  symbols, including Create/Save and every floating Add action, in both appearances
  with calculated contrasting ink as the sole exception when Increase Contrast is
  enabled. This supersedes the earlier Today-only white styling. White on #FF6600 is 2.936:1; the requested
  styling is not an accessibility contrast remediation. Custom accents retain
  calculated ink. The selected default-orange palette swatch follows the same policy.
- **Remaining:** foreground contrast findings remain open under the approved exact
  orange/white policy. Native trait and rendered-button regression tests verify styling,
  not an accessibility contrast pass.

## B-014 — largest text truncates Settings values and iPad empty-state title

- **Severity:** High. **Features:** F-019/F-020. **iOS:** Planned remediation.
- **Reproduce:** AX5, Task Lists settings in English/German; selected grouping and
  sort values truncate. iPad mini Today also truncates its empty-state heading;
  its AX5 editor Save glyph extends outside the filled action. Full-width iPad
  Settings values fit, so phone picker truncation is not reproduced there.
- **Expected:** full meaningful values/headings with wrapping/scrolling at large
  text. Native menu values remain full in the accessibility tree.
- **Cause/fix:** menu value/empty-state layout does not provide full visible text
  at AX5; adapt the layout without shrinking or capping Dynamic Type. See A-07.
- **2026-09-17 partial remediation:** fresh shipping-app regressions reproduce
  iPadOS 27 empty-title clipping (`/tmp/momentum-ax5-empty-ipad27-before.xcresult`)
  and iOS 26.5 Settings clipping (`/tmp/momentum-ax5-settings26-before.xcresult`).
  The empty title now uses an explicitly wrapping label inside ContentUnavailableView.
  Strict native clipping checks pass on iOS 26.5/27 phone and iPad mini simulators;
  dated results are in PROGRESS. The inspected iPad screenshot shows the complete
  two-line title. This check covers the English portrait heading, not every empty
  state, locale, orientation or spoken VoiceOver interaction.
- **AX5 description reachability:** separate shipping-app tests pass on iOS 26.5
  (8.429s, `/tmp/momentum-empty-scroll26.xcresult`) and 27 (9.816s,
  `/tmp/momentum-empty-scroll27.xcresult`). The whole description scrolls between
  the navigation bar and Add task, and Add opens quick entry. Both screenshots
  were inspected. This required no further layout change; heading and description
  need not fit simultaneously on the smallest viewport at AX5.
- Task Lists now retains compact menu pickers at standard sizes and uses wrapping
  selected-value rows with native inline selection pages at accessibility sizes.
  Initial screenshots show complete grouping/sort values on phone/iPad, but the
  whole-screen native audit still reports anonymous clipping on both. These findings
  remain unsuppressed. Selection/persistence now pass in English/German on iOS
  26.5/27 phone simulators, including all four preferences after a cold relaunch. Do not call the whole B-014 issue resolved
  from these scoped functional results.
- **Toolbar follow-up, 2026-09-17:** `SheetCommitButton` now clears the inherited
  content font on its symbol so native toolbar sizing applies. A rendered-pixel
  regression reproduces the old AX5 glyph crowding and verifies Save/Create ink
  stays inset within its orange fill. iOS 26.5 phone standard/light and AX5/dark
  flows pass, including saving and creating. The same cases pass on iOS 27
  phone and iPad mini 26.5/27 (eight scenarios total); both iPad AX5 screenshots
  were inspected. Dated bundles and timings are recorded in PROGRESS. Task
  typography and the approved ink policy are retained.
  This does not resolve the separate Settings audit findings. Recheck landscape
  and other window sizes independently.

## B-015 — task row exposes undersized accessibility frames

- **Severity:** Medium. **Features:** F-006/F-020. **iOS:** Implemented; native
  phone target-size/tap checks pass, broader assistive acceptance remains open.
- **Reproduce:** add a task with tag/estimate. XCTest reports actionable metadata
  at 130.3 × 14.3 points; Open title frames measure approximately 20.3 points high.
- **Expected:** adequately sized independent Open and Complete targets. Source
  gives only Complete a 44-point minimum. Effective touch slop remains unmeasured.
- **Fix (2026-09-17):** group title, metadata and note/reminder symbols into one
  Open button with a 44-point minimum height and rectangular hit shape. Complete
  stays independent; archive stays read-only. The Open accessibility value retains
  project, estimate, schedule, tag, notes and reminder information. Existing row
  context/swipe/drag actions and named reorder actions remain in place. See A-02.
- **Regression evidence:** isolated production-view baseline fails at 20.33 points
  (`/tmp/momentum-row-targets26-before.xcresult`). Post-fix native tests pass on
  iOS 26.5 and 27, including separate/nonoverlapping 44-point Open/Complete targets
  for parent/subtask/second rows, accessible metadata, unfiltered hit-region audit,
  a bottom-right Open tap, and completion/reopening. Bundles:
  `/tmp/momentum-row-targets26-metadata.xcresult` (19.914s) and
  `/tmp/momentum-row-targets27-metadata.xcresult` (22.213s). Screenshot inspected.
  An intermediate exact-double comparison failed at 43.99999999999994 points;
  geometry assertions now allow 0.001-point conversion noise.
- Three existing in-process row rendering regressions also pass (0.293s total,
  `/tmp/momentum-row-rendering26.xcresult`): Dynamic Type/custom content scaling,
  compact long-title wrapping, and completed appearance in light/dark. These
  component checks supplement the native target/tap regressions.
- **Remaining:** largest-text/native tablet/physical target checks, spoken
  VoiceOver and alternate-input acceptance are not established by this phone test.
  No claim that the original effective touch slop was measured. Existing B-012
  editor frame warning still appears when opening the editor; it is not hidden.
- **2026-09-16 scope note:** the new floating Add task control has its own
  minimum-target/layout regression checks; task row geometry is unchanged.
  Moving Add does not resolve B-015 or the other accessibility audit findings.

## B-016 — operation feedback automatically disappears after five seconds

- **Severity:** Medium. **Features:** F-017/F-020. **iOS:** Superseded by B-087;
  implemented with deterministic model and native-host coverage.
- **Reproduce:** after the 2026-09-17 correction, every operation result stayed visible
  until it was dismissed or replaced. This removed the former reading deadline but made
  routine success messages persistent across tabs.
- **Expected:** routine information is a transient, accessible toast; an undoable mutation
  keeps its exact Undo action long enough to act; a persistence failure remains until the
  person acknowledges it.
- **Fix (2026-09-19):** one root-owned presentation replaces the per-list banner. Ordinary
  information dismisses after 3.5 seconds, exact-batch Undo after 5 seconds, and
  `SaveFailed` remains focused with a Dismiss action. One cancellable sleep task restarts
  for replacement messages, while the existing low-priority announcement is emitted once.
- **Evidence:** see B-087 and the dated PROGRESS handoff. Spoken VoiceOver remains outside
  the current user-approved scope.

## B-017 — task-list subtask relationship lacks accessible context

- **Severity:** Medium. **Features:** F-006/F-020. **iOS:** Implemented; native simulator checks and spoken acceptance recorded below.
- **Reproduce:** add identically named children under two parents, return to Today,
  inspect child Open/Complete actions. Previously both controls had identical labels.
- **Expected:** the relationship communicated by indentation is also available
  nonvisually, including duplicate child titles and filtered search results.
- **Cause/fix:** `isSubtask` previously changed padding only. The mobile worker now
  resolves distinct parent titles off-main once per snapshot, reusing visible rows
  and querying hidden live/archived parents through the read-only core batch API. English/German Open/Complete/Reopen labels
  include the parent; the two independent 44-point actions remain intact. Current
  snapshots reflect parent renames. Read-only archived rows expose the same context,
  including filtered parents in either archive tier. Localized generic “subtask”
  remains a fallback only when the parent record is unavailable. The additive
  `task_reference_titles` API preserves live-only `task_title` semantics and data
  formats; bindings are regenerated by the normal tooling.
- **Regression:** the old native hierarchy contains two identical Open/Complete
  child labels (`/tmp/momentum-subtask-context26-before.xcresult`, failed expected
  contextual-label assertion). The worker tests cover duplicate titles, filtered
  parents, rename refresh and empty search; all four EngineWorker tests pass.
  English/German native open, complete/reopen and filtered Search checks plus the
  independent target/metadata regression pass on iOS 26.5 (3 tests, 61.605s;
  `/tmp/momentum-subtask-context26-after.xcresult`) and iOS 27 (3 tests, 69.958s;
  `/tmp/momentum-subtask-context27-verified.xcresult`). Exported English/German
  hierarchies confirm distinct parent labels and separate 44-point targets; German
  Search screenshot inspected. Dirty `task/ios-app`, 2026-09-17.
- **Archive follow-up:** an isolated mobile regression reproduced empty parent names
  after archival. The read-only batch lookup now passes core tests for both tiers,
  live/young precedence, invalid/missing records and unchanged task/undo state.
  All 116 mobile tests and 254 enabled all-feature workspace tests pass. Native
  English/German archive Search and live-action regression checks pass on iOS 26.5
  (3 tests, 51.699s; `/tmp/momentum-archived-context26.xcresult`) and 27 (3 tests,
  62.017s; `/tmp/momentum-archived-context27.xcresult`). Exported archive labels are
  read-only StaticText with parent names and metadata. Actual speech is not tested.
- **Remaining:** actual spoken VoiceOver/focus and alternate input. Further dip17pm testing is
  user-deferred, not passed. Other platforms are unchanged/unverified for this bug.

## B-018 — small validation error text falls short of contrast guidance

- **Severity:** Medium. **Features:** F-006/F-020. **iOS:** Verified for the recorded error-color, field-name and phone-simulator recovery scope.
- **Reproduce:** enter `invalid` in Estimate at ordinary text/light appearance.
  The visible red guidance triggers XCTest's “Contrast nearly passed” warning,
  whose detail requires a larger font for this contrast.
- **Cause/fix:** task/recurrence errors used `.red` directly. `AccentTheme.errorText`
  now resolves semantic system red against native backgrounds with 4.55:1 normal
  and 7.05:1 increased-contrast targets. Error SF Symbols complement the existing
  meaningful messages. Estimate guidance remains immediately after its field.
  The disabled recurrence value also uses readable secondary text after an
  unfiltered audit exposed its separate contrast issue. The Estimate field has an
  explicit localized name: a native regression first reproduced its previously
  empty label after text entry. Default orange is unchanged.
- **Evidence:** the original guidance failure is reproduced in
  `/tmp/momentum-validation26-before.xcresult`. The 48-combination native trait
  regression and existing default-orange/white-ink regression pass on iOS 26.5
  (0.007s; `/tmp/momentum-error-ink26.xcresult`) and 27 (0.006s;
  `/tmp/momentum-error-ink27.xcresult`). Native light/dark estimate correction,
  localized field labels and German AX5 recovery pass on both runtimes:
  `/tmp/momentum-validation26-current.xcresult` (three estimate cases) and
  `/tmp/momentum-validation27-final.xcresult`. Final recurrence rejection/recovery
  passes in `/tmp/momentum-repeat-error26.xcresult` (34.824s) and
  `/tmp/momentum-repeat-error27.xcresult` (40.644s).
- **Audit scope:** ordinary estimate audits are unfiltered. German AX5 handles only
  the verified toolbar-covered `Fällig` label with pixel/geometry guards (B-039).
  Recurrence defers only a partly-offscreen `Starts` finding and must pass another
  unfiltered audit once it is visible. Original failed findings remain attached;
  these scoped checks do not imply whole-app accessibility compliance.
- **Build:** normal production iOS simulator build passes, exit 0
  (`/tmp/momentum-validation-shipping-build.log`), dirty `task/ios-app`, 2026-09-17.
- **Remaining:** spoken error announcement/focus remains separately open under
  A-05, and B-014 retains the oversized AX5 Save symbol. Broader tablet/assistive
  acceptance is not implied by this phone-simulator scope.
  No physical acceptance is implied by the user-deferred dip17pm checks.

## 2026-09-16 — fast iOS regression infrastructure (F-039)

- No accessibility finding is closed by adding tests. B-013–B-018 remain open;
  component geometry/rendering is not whole-screen or assistive-technology acceptance.
- B-008's rejection/retry behavior now also has fast tests through the exact
  Quick Add presentation state consumed by the sheet, including a real Rust fixture.
  Deliberately accepting unchanged outcomes failed three assertions; restored source
  passes the full 67-test package suite. See the F-039 progress entry for UI evidence.
- B-009/default-orange regression protection includes rendered black symbol pixels
  inside the actual orange filled button. Forcing white ink failed in both appearances.
  Toolbar promotion remains covered by the existing native acceptance scope, not by
  the new standalone component test. The production accent policy is unchanged.
- Bootstrap-only fixture issues (scene lifecycle, absent key window and appearance
  forwarding) were corrected in the separate test host/fixture. They are not product
  defects. The runner rejects empty test selections and stale prepared core artifacts.

## 2026-09-16 — dark icon rendering assessment (F-019)

- The first native icon draft exposed an iOS 26 SVG stroke/color-override rendering
  difference: the checkmark's open interior filled in. Replaced strokes with closed
  filled outlines before acceptance; final iOS 26/27 exports retain the same mark.
- This was caught in design verification, not reported as a shipped defect. Existing
  accessibility findings remain open. No task behavior or other platform changed.

## 2026-09-16 — automation review and test-harness assessment (F-031/F-040)

- Review of the new shared entity query found that `try?` could hide a Mac startup
  error and that iOS resolved each ID through a separate lifecycle refresh. The final
  query acquires the runtime once and resolves a batch, omitting stale IDs while
  propagating startup errors. Mutation batches retain strict all-ID validation.
  A new portable regression proves ordered/deduplicated/stale resolution and strict
  mutation rejection; independent source re-review confirmed both fixes.
- Native URL test failures initially came from relaunching through
  `XCUIApplication.open` while expecting warm navigation and expecting an unscheduled
  URL task in Today. The acceptance test now uses Safari for the warm handoff and
  supplies an explicit due date. Direct URL opening from the UI-test runner is
  rejected by iOS as untrusted; no product workaround or security bypass was added.
- The App Intents test framework rejects Customer OS builds with security error 803.
  That exact condition is reported as a skip, never a passed native invocation.
  A loader failure on iOS 26 also showed that this framework must live in its own
  iOS 27-only test target; a runtime availability guard does not isolate its dylibs.
  Existing accessibility findings and broader parity gaps remain open.
- The iOS 26.5 Shortcuts provider error was isolated to simulator signing in the
  tested Xcode 27 configuration. A fresh one-action app reproduced code 1
  (`Couldn't find AppShortcutsProvider`) when unsigned/ad-hoc; `linkd` could not
  resolve its team identity. A verified Apple Development signature made the probe
  execute, and the same signing correction made Momentum's native Create Task
  acceptance pass. No product authentication workaround, account setup, provisioning
  change or security-setting change was needed. Reproduction steps are in
  [iOS testing](../ios/TESTING.md); final evidence is recorded in PROGRESS.md.

## B-019 — an obsolete Nextcloud exchange can overwrite a restored store

- **Severity:** Critical. **Features:** F-025/F-029. **Shared-core reproduction:**
  pause a real loopback GET, import a replacement, add another task, then release
  the exchange. The original implementation reports success from the obsolete
  snapshot; its cloned Store retains the live directory and may write there before
  any engine-level check. Pending-array offsets also become invalid on replacement.
- **Affected platforms:** Linux/macOS use this shared path; native reproduction is
  not claimed. iOS links the same core and now has a native backup screen; provider UI remains unfinished. Android is
  Planned. Shared fix implemented; native platform acceptance remains outstanding.
- **Fix:** memory-only exchange, replacement generation checks between requests and
  under the final store lock, stable pending-op identity merge, and a sync lease
  retained through persistence. An import during GET prevents the later PUT.
- **Evidence:** red/green `transport_tests::import_during_exchange_rejects_the_old_result_without_writing_over_replacement`;
  validates live and reopened replacement state and zero obsolete uploads. The
  actual HTTP wait is bounded by a 30-second whole-exchange deadline; cancellation
  is cooperative and cannot retract a request already accepted by a server.
- **Follow-through:** explicit cooperative Nextcloud cancellation now uses the same
  generation barrier; its active lease remains held until I/O returns. Journaled
  replacement recovery and P2P exclusion are implemented under B-022/B-027. Native
  provider/lifecycle integration and remote-peer restore acceptance remain unfinished; local iOS document-flow evidence is recorded in PROGRESS.md.

## B-020 — sync commit rolls back clocks and local metadata for concurrent edits

- **Severity:** High. **Feature:** F-025. **Shared-core reproduction:** pause GET,
  add a task locally, then finish sync. The new pending operation survived but its
  vector-clock counter regressed from 2 to 1 in the reproduced fixture.
- **Fix:** merge live vector clocks with the exchange result; retain live summary
  claims and nearby metadata. Pending operations are identified by stable IDs.
- **Evidence:** red/green `transport_tests::edits_during_exchange_remain_pending_and_keep_their_vector_clock`.
  Linux/macOS share the implementation; native/network-server acceptance is not
  inferred. iOS transport UI and Android remain unfinished.

## B-021 — unrelated JSON is accepted as an empty destructive backup

- **Severity:** Critical. **Feature:** F-029. **Shared-core reproduction:** import
  `{}` or `{ "data": {} }` over an existing task store. Serde defaults previously
  accepted either object as empty task data, so import replaced existing tasks.
- **Fix:** backup parsing requires an explicit task collection with both `ids`
  and `entities`; malformed typed values still fail deserialization. Raw/wrapped
  backups, explicitly empty collections and unknown fields remain supported.
- **Evidence:** model red/green validation plus
  `transport_tests::invalid_backup_preserves_tasks_pending_undo_and_disk`. The fixture
  has an actual undo batch before import; creation alone is not undoable.
- **Platforms:** shared fix implemented for Linux/macOS and linked iOS core. Native
  backup-state and app-model tests reject unrelated JSON without changing tasks or revision; system-picker/error presentation acceptance is tracked in PROGRESS.md. Android implementation is Planned.


## B-022 — failed or interrupted store replacement can report success or mix generations

- **Severity:** Critical. **Features:** F-001/F-029. **Reproduction:** block a later
  JSON temporary-file write during import/save. Import previously returned success
  despite persistence failure; a successful earlier rename could leave state, pending
  operations and metadata from different generations.
- **Fix in progress:** private undo journal around the existing three JSON files,
  synced preparation and commit boundaries, recovery before loading, and clone/save/
  publish replacement. Failed import preserves live tasks, pending operations and undo.
  Unknown/incomplete journals fail closed and remain available for recovery.
- **Evidence:** actual child-process exits at six write boundaries recover a coherent
  old or committed image; first-save interruption, failed writes, corrupt/incomplete
  records and orphan cleanup are covered by `momentum-store` tests. Core tests exercise failed
  import and checked startup; CLI returns its normal JSON error instead of a panic.
- **Platform boundary:** iOS implements an unavailable screen with retry; macOS a
  native retry/quit alert. Linux still uses the legacy fatal startup boundary when a
  journal cannot be recovered; friendly Linux startup handling remains outstanding.
  Android is Planned. Process-exit tests do not establish physical power-loss safety.
- **Remaining:** native recovery/performance acceptance and write-cost optimization
  (B-023). P2P replacement exclusion is implemented under B-027 and ordinary mutation
  publication/error handling under B-025. Local iOS document-flow evidence is recorded
  in PROGRESS.md; remote-provider lifecycle and physical power-loss safety remain open.

## B-023 — durability barriers increase repeated-mutation latency

- **Severity:** High. **Features:** F-001/F-029. **Affected:** all shared-store callers;
  mobile actor work stays off the main thread, while existing Mac/GTK calls may block UI.
- **Measured on 2026-09-17 dirty `task/ios-app`:** the first post-journal core run took
  41.59 seconds versus 1.09 seconds in the preceding transport run; mobile package tests
  took 4.456 seconds versus 0.107 seconds. Builds/tests overlapped, so these are workload
  observations, not isolated benchmarks or device energy measurements.
- **Cause:** every mutation synchronously journals and flushes the three-file store;
  loops can persist more than once per user action. Correctness tests pass but this is
  not accepted as performance parity.
- **Partial mitigation:** demo/preview seeding now commits one ordered batch; core
  suite time fell to 17.41 seconds. Sequential final Swift runs took 2.168 seconds
  (mobile) and 15.823 seconds (Mac, down from 119.134 seconds before batching).
  This does not resolve ordinary mutation latency.
- **Further mitigation:** main core edit boundaries now stage a private candidate and
  commit once per action. A 100-task completion/undo/paste sample measured 48.93 /
  52.60 / 56.29 ms; committed batches wake the sync observer once. B-025 covers the
  associated failure atomicity. Full core suite was 13.43 seconds before the final
  no-op case; mobile tests were 1.704 seconds and Mac tests 11.214 seconds.
- **Next:** validate large-store and physical-device latency/energy, and assess remaining
  direct-store/native-main-thread writes while retaining crash recovery. Never disable
  durability in tests merely to conceal the cost. Physical battery checks remain open.


## B-024 — native view-test host lacks app translations

- **Severity:** Medium (verification infrastructure). **Feature:** F-039.
- **Reproduction:** render a production SwiftUI view with German environment locale
  in `MomentumFastTests`; screenshot still shows English. Strings resolve from the
  test host's main bundle, which previously contained no app catalog.
- **Fix:** include the production string catalog in the isolated host; the recovery
  rendering regression asserts that the host's German localization resolves its title.
- **Scope:** iOS view tests only. This invalidates German-localization claims based
  solely on the old component harness, not actual app/UI-test screenshots. Mac/Linux
  production behavior is unaffected. Native rerun and image inspection recorded in
  PROGRESS; existing whole-app accessibility acceptance remains separate.


## B-025 — ordinary edits publish unsaved state and consume undo after write failure

- **Severity:** High. **Features:** F-001/F-002/F-005/F-012/F-031. **Reproduction:** block a
  store temporary-file write, then complete/delete a task batch, create a tagged task,
  or undo a batch. Previous normal dispatch logged the error but changed live tasks
  and pending operations, consumed undo, and returned success. Backup-specific B-022
  handling did not cover these regular mutation paths.
- **Fix in progress:** evaluate an edit on a private engine candidate, retain operation
  order/clocks, perform one journaled save, then publish and notify once under the same
  owner lock. Save failure returns a structured error outcome without replacing live
  tasks/undo or waking sync. CLI mutation failure remains a normal error and no longer
  retains an uncommitted operation in memory.
- **Regression evidence:** red/green failure injection for bulk edits, task creation,
  undo retry and organization identities; callback records prove observers see only
  a complete committed batch. Additional tests cover compound capture/paste/tag edits,
  no-op form saves and repeat-cursor retry. Swift package checks cover failed capture,
  automation errors, notification-action retry and Mac form save outcomes. Current
  native checks and verification limits are recorded in PROGRESS.
- **Scope limits:** selected main engine edit boundaries are covered; direct external
  `with_store_mut`/Store callers and legacy Rust tag helpers retain their existing
  separate error contracts. B-027 extends durable publication to P2P adoption.
  Desktop dismissal/error presentation and full
  native interaction acceptance require their own evidence. Android remains Planned.


## B-026 — rejected capture or missing tag targets can still create tags

- **Severity:** Medium. **Features:** F-004/F-005. Shared reproduction: submit only
  `#orphan 30m`, or apply a new tag by name to a missing task ID. The engine returned
  no change while creating and persisting the tag as a side effect.
- **Fix:** validate the parsed title and target membership before creating any tags.
- **Evidence:** a real writable isolated-store regression compares complete state,
  pending operations and clocks before/after both rejected actions. Its initial
  fixture incorrectly dropped its temp-directory owner and was corrected before
  obtaining a real red/green failure; that accidental green is not acceptance.
- **Scope:** shared fix implemented; native Quick Add state coverage and current
  platform builds are recorded in PROGRESS. Android remains Planned.

## B-027 — nearby receive and restore bypass durable publication boundaries

- **Severity:** High. **Features:** F-002/F-026/F-029. Shared reproduction: fail a
  task-store write while receiving peer data, or restore and restart with an old
  transport inbox. Previously live data/bootstrap/timestamps changed despite save
  failure, the transport consumed the batch before persistence, and queued data could
  reappear after restore. Concurrent starts could also create separate runtimes.
- **Fix implemented:** commit received snapshot/operations/repeats/metadata on one
  private engine candidate; acknowledge exact journal entries only after commit.
  Failed acknowledgement retains its retry batch and preserves newer arrivals.
  Start/stop/restore share provider admission; stopped callbacks check their flag
  under the store lock. Nextcloud and LibreSync cannot be admitted simultaneously.
  Stop also waits for the entire store-commit/journal-acknowledgement section, so an
  old runtime cannot overwrite a replacement runtime's journal. Delegates are called
  after releasing that barrier to allow synchronous lifecycle requests.
- Restore increments a backward-compatible, default-zero internal metadata epoch
  in the same durable store transaction. On startup, a mismatched journal discards
  pre-restore queued data and unpublished operations, retains identity/trust/seen IDs,
  and publishes the current snapshot. Restored data is no longer eligible for initial
  peer-snapshot union. Invalid journals fail startup without overwriting the evidence.
  The task/backup/remote operation formats are unchanged; this adds internal metadata.
- **Evidence:** real red/green regressions for failed peer save, active-runtime
  restore, stale inbox after process/runtime restart, and corrupt-journal startup.
  Additional tests cover provider exclusion, concurrent startup, late stopped
  application, durable retry/acknowledgement and arrivals during acknowledgement.
  A paused-acknowledgement regression first demonstrated that restore returned before
  the old runtime finished, then passed with the receive barrier; a delegate reentrancy
  test exercises synchronous stop without holding that barrier.
- **Limits:** final builds/checks are in PROGRESS. Physical cross-device convergence,
  native provider switching/recovery, journal power-loss behavior and remote changes
  first received after a restore remain separate acceptance. Restore replaces local
  data; it cannot retract changes already exchanged with another device. B-029 covers
  acknowledgement failure followed by a provider switch. Android is
  Planned; Linux/macOS/iOS share the implemented core correction.

## B-028 — stopping Sync All waits for stalled peer I/O instead of cancelling it

- **Severity:** High for mobile lifecycle responsiveness. **Feature:** F-026.
  `BackgroundEngine::run` does not pass its command cancellation token into the
  custom discovery/exchange closure. Shutdown therefore waited for peer socket
  timeouts even though the background engine requested cancellation.
- **Fix implemented:** a runtime-owned LibreSync cancellation token is supplied to
  every Sync All request and checked before discovery and between peers. Shutdown
  cancels the token before joining the transport, closing registered sockets.
- **Evidence:** an isolated loopback peer accepts but stalls the TLS handshake.
  Before the fix, shutdown took 5.002 seconds and failed the under-two-second guard;
  after the fix the regression passes. The test still includes the normal two-second
  discovery pass before measuring shutdown. Discovery itself remains bounded by that
  timeout and is not instantaneously interruptible. Native background-expiration
  and physical network testing remain outstanding.

## B-029 — an unacknowledged peer operation replays after another provider clears pending

- **Severity:** High. **Features:** F-002/F-025/F-026. **Reproduction:** receive a
  peer AddTask, let its task-store commit succeed but fail transport acknowledgement,
  complete the task locally, upload through Nextcloud, reopen the store and retry
  the peer inbox. The original AddTask overwrote the newer completion because pending
  operations had been the only duplicate check. Snapshot-covered operations could
  also replay when acknowledgement retried without the original snapshot boundary.
- **Fix implemented:** persist exact peer-operation receipts in the same transaction
  as the received state, including operations covered by a bootstrap snapshot. They
  remain independent of Nextcloud's upload queue. Each subsequent receive retains
  only receipts still in the unacknowledged transport inbox; restore clears them in
  its durable replacement transaction. This is default-empty internal metadata,
  preserving task, backup and remote operation formats without a migration.
- **Evidence:** both cases reproduced before the fix. The provider-switch regression
  uses an actual isolated HTTP exchange, failed journal write and process reopen;
  the bootstrap regression also checks durable pruning once the inbox is empty.
- **Platforms:** Linux/macOS/iOS share the implemented core path; final compilation
  and package evidence is in PROGRESS. Physical provider switching and native sync
  UI acceptance remain outstanding. Android is Planned.

## B-030 — Settings button titles inherit insufficient orange contrast

- **Severity:** Medium. **Features:** F-029/F-020. **Affected:** iOS Settings labels
  used inside native buttons. No shared-core change; desktop/Android do not use this
  SwiftUI component.
- **Reproduction:** open Settings → Backups in light appearance with the default
  #FF6600 accent. Apple's native contrast audit flags Export Backup and Import Backup.
  Component screenshots independently show orange title text.
- **Cause/fix:** hierarchical `.primary` styling inherits the button's foreground;
  explicitly applying `Color.primary` keeps readable system title text while symbols
  retain the selected accent.
- **Evidence:** the failing audit is `/tmp/momentum-backup-ui-27.xcresult`; final
  light/dark/native regression results are recorded in PROGRESS.md. Spoken screen
  reader acceptance remains separate from these automated contrast checks.

## B-031 — credential errors can be mistaken for absence or successful removal

- **Severity:** High. **Feature:** F-025. **Affected:** shared Apple Keychain wrapper;
  Linux/Android use different credential adapters and are not reproduced here.
- **Cause:** legacy `get` returns nil for every OS error, and `set(account, "")`
  returned true without checking `SecItemDelete`. A locked or denied read must not
  be treated as permission to replace a missing connection.
- **Fix:** additive checked Data read/write/delete APIs distinguish item-not-found
  from denial, lock and storage errors. The legacy empty-password setter now returns
  false on deletion failure. Existing desktop accounts and protection defaults stay
  compatible. New mobile connections store all fields/secrets in one atomic item,
  with device-only access after first unlock. Corrupt records remain errors.
- **Evidence:** injected OS failures through the real wrapper exercise denied reads,
  failed deletion, no add fallback after denied updates and failed insertion. An
  isolated actual Keychain round trip and native attribute readback are recorded
  in PROGRESS.md. No production credential or account is used.
- **Remaining:** native mobile setup/retry UX must consume the checked errors. Existing
  desktop UI and LibreSync callbacks still use legacy optional reads; their denial
  handling remains separate work, not a claimed fix from this additive API.

## B-032 — unsigned native test host cannot verify Keychain access

- **Severity:** Medium. **Features:** F-025/F-039. **Affected:** iOS test harness only.
- **Reproduction:** the real native connection test fails with Security status
  `errSecMissingEntitlement` when the runner uses `CODE_SIGNING_ALLOWED=NO`. The
  cancellation test passes, separating the signing failure from FFI behavior.
- **Fix:** native fast tests use local ad-hoc simulator signing, an explicit empty
  development team and no certificate/provisioning requirement. Xcode supplies
  simulated application entitlements. No personal identity is embedded in source.
- **Evidence:** the unsigned failure is `views-1789658659094151000.xcresult`; the
  signed two-test probe passes at `/tmp/momentum-keychain-signed-probe.xcresult`.
  Full iOS 26.5/27 inventories and timings are recorded in PROGRESS.md. This does
  not alter the separate system Shortcuts development-signature requirement.


## B-033 — foreground return waits for the periodic sync interval

- **Severity:** Medium. **Feature:** F-025. **Affected:** iOS coordinator added in this
  dirty worktree; not a reproduced desktop regression.
- **Reproduction:** background an in-flight automatic exchange, return before its
  cancellation drains, then release the cancelled operation. The next scheduled
  delay was 300 seconds rather than an immediate foreground retry.
- **Cause/fix:** the resumed foreground event was ignored while work was active.
  Remember it until the exchange drains, then schedule an immediate retry. The old
  transport is still awaited; no overlap or false success is introduced.
- **Regression:** continuation-controlled `foregroundReturnRetriesImmediatelyAfterCancelledExchangeDrains`
  failed with the periodic delay and passes after the fix. Real suspended-device
  network acceptance remains outstanding. Final evidence is in PROGRESS.md.

## B-034 — sync execution allowance mishandles boundary expiration callbacks

- **Severity:** Medium. **Feature:** F-025. **Affected:** new iOS background allowance;
  discovered in injected native tests before physical-device acceptance.
- **Reproduction:** an expiration callback delivered before the begin function returns
  fails to balance the subsequently returned token. A retained callback invoked after
  completion can also request cancellation after the original exchange ended.
- **Fix:** record expiration/completion independently of the token, balance a returned
  token even after synchronous expiration, and ignore callbacks after completion.
  End the assertion promptly on expiration; do not wait for blocking network I/O.
- **Regression:** both native tests failed against the first implementation and pass
  after the fix. Normal completion, repeated finish and invalid tokens are covered.
  These injected orderings test defensive handling, not a claim that UIKit delivered
  either ordering on a physical device. Final evidence is in PROGRESS.md.


## B-035 — isolated Sync settings dims the provider label below readable contrast

- **Severity:** Medium. **Features:** F-019/F-025. **Affected:** iOS preview/test
  Settings; production provider controls were not reproduced as affected.
- **Reproduction:** navigate to Sync in an isolated app and run the native contrast
  audit. The disabled navigation row's “Sync Provider” label nearly passes contrast.
- **Fix:** isolated mode now presents an explicitly read-only Off status using native
  LabeledContent, rather than a disabled link with dimmed text. No setup action,
  credential access or transport becomes available in isolated mode.
- **Evidence:** `/tmp/momentum-native-sync-settings27.xcresult` records the original
  audit failure. Final reruns are recorded in PROGRESS.md; no broad app compliance
  claim follows from this scoped screen audit.


## B-036 — hard scroll-edge effects create two-tone screen backgrounds

- **Severity:** Medium. **Feature:** F-019. **Affected:** iOS tabbed interface. The
  user supplied a light-mode Today screenshot showing pale bands above and below
  the grouped task canvas on iOS 27.
- **Cause:** RootView explicitly forced `.scrollEdgeEffectStyle(.hard)` at both edges.
  Apple's hard effect intentionally paints a nearly opaque, sharply bounded area
  behind stationary controls; it does not preserve a continuous grouped canvas.
- **Fix:** hide scroll-edge effects throughout the tab hierarchy using the public
  iOS 26+ modifier. Native floating buttons and tab controls retain their own materials.
  The canvas continues to follow the system appearance, without hard-coded light colors.
- **Verification:** `BackgroundTests` samples clear canvas regions behind top/bottom
  controls and between cards in the actual app, with light/dark screenshots for all
  four tabs. Final iOS 26.5/27 evidence is recorded in PROGRESS.md. Component-only
  renders are deliberately not used as proof of safe-area appearance.

## B-037 — Nextcloud cannot create a missing sync folder

- **Severity:** High. **Feature:** F-025. **Affected:** shared transport used by
  Linux, macOS, CLI and iOS. Reproduced through the actual iOS Rust/HTTP boundary;
  native desktop interaction has not been rerun.
- **Reproduction:** import a disposable complete backup, configure an absent WebDAV
  collection, then sync. GET returns 404 and the first PUT returns 409. The client
  fails before sending MKCOL: `MKCOL not valid for HTTP version HTTP/1.1`.
- **Cause:** ureq 3 rejects extension methods by default. The shared transport's
  collection-creation path never enabled WebDAV methods. The original Rust mock
  accepted PUT without requiring a collection, so existing tests missed this branch.
- **Expected:** create the collection once, retry the upload within the original
  deadline/cancellation contract, and clear pending changes only after commit.
- **Fix:** explicitly permit WebDAV extension methods in the scoped ureq agent;
  existing deadline, cancellation, TLS validation and bounded folder retry remain.
- **Verification:** the missing-collection Rust regression failed before the fix,
  then passed; actual iOS 26.5/27 loopback exchanges create the folder and converge.
  Shared Rust/CLI and macOS Swift regressions pass. B-037 is resolved for this
  transport scope; hosted-server and native desktop interaction remain unverified.
  Exact results and bundles are recorded in PROGRESS.md.

## B-038 — saved connection status triggers a native text-clipping finding

- **Severity:** Medium. **Features:** F-019/F-020/F-025. **Affected:** iOS Sync settings.
- **Reproduction:** configure/save a connection and audit the saved-status row. With
  the production scroll-edge configuration, Apple's native text-clipping audit identifies
  the `sync-saved` label: “Connection Saved” may clip at larger Dynamic Type sizes.
- **Status:** Verified for the reproduced iOS 26.5/27 simulator scope. The saved
  confirmation renders completely in English light and German dark at AX5; both
  unfiltered audits pass on both runtimes. Normal-size auditing reports one
  unavailable text-clipping node on both runtimes even though the visible footer is
  complete. The captured hierarchy shows retained Form cells above the viewport;
  on iOS 26 it additionally reports exact secondary labels already covered by B-053.
- **Investigation boundary:** an earlier anonymous contrast finding came from the QA
  host omitting the production scroll-edge configuration. Neutral accent and primary
  status ink did not fix it; matching RootView's actual edge settings removed it.
  Those experimental color changes were reverted. iOS 26.5 still reports an anonymous
  contrast finding, also reproduced on iOS 27 with an opaque navigation surface;
  its element/cause remain unresolved. No default-orange policy changed.
- Reduced native Forms isolate clipping to Label variants while plain Text and
  explicit stacks pass. The same stack still fails in the complete form, so that
  speculative production change was reverted. Reduced contrast examples identify
  a partly occluded row and a disabled Save button, but do not establish the full
  form's anonymous finding. See the dated probe bundles in PROGRESS.md.
- **Remediation (2026-09-17):** the saved confirmation now lives in
  a wrapping Options footer; Save Connection appears only for unsaved changes. The
  redundant disabled action is removed and successful saving directs accessibility
  focus to its confirmation. Plain Text in the old row did not clear the native finding.
- **Typography isolation:** the same iOS 27 audit failed with native
  semantic body/caption fonts and with the QA host's app-font override removed.
  Both runs identify the fully on-screen `Options` header, so inherited typography
  did not explain the original result. Those speculative experiments were reverted.
- **Final audit treatment:** the normal-size regression may handle at most one
  unavailable `.textClipped` finding. On iOS 26 only, it also recognizes the exact
  Sync secondary labels through the separately verified B-053 classification. Every
  labeled clipping issue, visible hit-region issue, other label and every iOS 27
  contrast issue still fails. This bounded treatment is paired with unfiltered English
  light and German dark AX5 audits, exact saved-label/frame assertions, the 5.5:1/7.05:1
  token regression and retained issue attachments.
- **Evidence:** final matrices pass three tests on iOS 26.5 (108.551s,
  `/tmp/momentum-sync-saved26-matrix-final.xcresult`) and iOS 27 (109.295s,
  `/tmp/momentum-sync-saved27-final.xcresult`). Hosted Nextcloud/TLS, spoken
  VoiceOver and physical-device acceptance remain separate and unverified.

## B-039 — scrolled content overlaps the navigation title

- **2026-09-29 note:** the opaque surface below was replaced at the user's request by
  the Liquid Glass navigation bar with the system's soft top scroll-edge effect, which
  keeps titles readable while content shows through. The pixel evidence below describes
  the earlier opaque fix and no longer applies.

- **Severity:** Medium. **Features:** F-019/F-020/F-025. **Affected:** iOS Sync settings, Today and Accent Color reproduced;
  shared task and Settings navigation also received the same canvas correction.
- **Reproduction:** save a connection and scroll to the bottom. The flat-canvas
  configuration hides the scroll-edge treatment, allowing form text behind the
  navigation title/status area. Reproduced in English/light and German AX5/dark
  screenshots on iOS 27; this was not covered by B-036's empty-screen pixel checks.
- **Cause/fix:** the pushed screen needs an opaque navigation background matching
  `systemGroupedBackground`. Applying toolbar preferences outside NavigationStack
  had no effect. SyncSettings now applies the shared navigation-canvas modifier on
  its own content. Extended to task/list and Settings destinations after reproducing
  the same problem on Today and Accent Color. Native controls and the exact accent
  policy remain intact. Automatic background visibility preserves expanded large
  titles; forcing the background visible hid those titles on iOS 26.5. Native tests
  now inspect title pixels as well as canvas pixels.
- **Evidence:** before `/tmp/momentum-sync-setup27-final-renders`, after
  `/tmp/momentum-sync-nav-inner27-renders`. Both inspected on September 17 in the
  dirty iOS worktree. Scoped light/dark regression and remaining results are recorded
  in PROGRESS.md. Light/dark pixel checks pass on iOS 26.5 and 27 for this Sync
  screen. The final automatic-visibility implementation also passes all four empty
  tab canvases/title-ink checks and eight scrolled destinations in light/dark mode
  on iOS 26.5 and 27; exact bundles are in PROGRESS.md. Remaining sheet, large-text
  navigation and full accessibility checks are outside this scoped verification.
- **Sheet follow-up (2026-09-17):** German AX5 TaskEditor reproduces the same
  overlap (`/tmp/momentum-validation26-final.xcresult`): `Fällig` at y=41 is visible
  beneath the navigation title and fails native contrast. Applied the existing
  canvas modifier to task and recurrence Form content. Inspected after screenshots
  show the label fully covered; German AX5 pixel/geometry checks pass on iOS 26.5/27
  as part of B-018's validation tests. XCTest retains the covered label and still
  reports its contrast; a narrow label/type/geometry exception is documented and
  guarded by opaque-toolbar pixel assertions. Original failing artifacts remain.
  This does not resolve B-014's oversized Save symbol.


- **Configured task-list regression (2026-09-17):** the new Nextcloud status top
  inset hid the large Today title even though the accessibility element existed.
  The native pixel regression measured zero title-ink pixels. Placing status in the
  scrolling List restores native title rendering; final cross-version evidence is
  in PROGRESS.md under F-028/B-043.

## B-040 — unreadable nearby-sync secrets can be replaced as if absent

- **Severity:** High. **Features:** F-026/F-027. **Affected:** shared Apple Keychain
  bridge used by macOS and the upcoming iOS LibreSync runtime. Linux uses a different
  secret-store adapter; its error handling has not been reproduced in this scope.
- **Reproduction:** return `errSecInteractionNotAllowed` when the core asks for a
  saved key, then permit a later write. The old optional-read bridge returned nil,
  authorizing the core's create-if-missing path to replace the key. Malformed data
  had the same unsafe absence interpretation. Reproduced with isolated Keychain stubs.
- **Fix:** latch read/write/delete failures for the lifetime of the transport's
  Keychain bridge; reject subsequent mutations. Validate base64 on read before it
  crosses the optional FFI callback. A new runtime creates a fresh bridge for retry.
  Existing item names/formats are preserved; no credential migration.
- **Evidence:** focused Swift regression first failed nine assertions, then all eight
  checked-Keychain tests passed (`/tmp/momentum-nearby-keys-red.log` and
  `/tmp/momentum-nearby-keys-green.log`, September 17 dirty iOS worktree). Broader
  The rebuilt Mac suite passes (136 tests including peer exchange), and all four
  native Keychain/sync boundary tests pass on iOS 26.5 and 27. Physical locked-device
  acceptance remains unverified.

## B-041 — nearby listener receives operations without notifying the app

- **Severity:** High. **Feature:** F-026. **Affected:** shared Rust transport used by
  Linux/macOS and the iOS adapter. Reproduced with isolated local peers on macOS;
  Linux runtime and physical iOS discovery remain separate checks.
- **Reproduction:** the mobile adapter receives a task, completes it on the second
  peer and syncs back. The first peer's transport journal receives the completion,
  but its task store stays unfinished until another outbound exchange occurs.
- **Cause:** the listener captured its event sink before `Engine.spawn()` installed
  one. Inbound events therefore never reached the application event pump.
- **Fix:** install the sink before starting the listener; retain the actual-port
  startup barrier and occupied-port fallback. Wake the app pump after runtime
  publication so queued startup events do not wait for another command.
- **Evidence:** the real two-node regression first timed out waiting for InboundSync,
  then passed in 0.14 seconds (`/tmp/momentum-nearby-inbound-green.log`). The immediate
  startup-notification regression also failed before its fix. All 15 shared core
  nearby tests pass in 0.88 seconds (`/tmp/momentum-nearby-core-green.log`), September
  17 dirty `task/ios-app`. Rebuilt mobile host and native iOS 26.5/27 two-peer
  exchanges pass; the Mac suite including peer exchange also passes. See PROGRESS
  for bundles and the later user deferral of iOS LibreSync.

## B-042 — nearby state-save completion causes an endless background save loop

- **Severity:** High (idle CPU, storage writes and UI callbacks). **Feature:** F-026.
  **Affected:** shared nearby runtime on Linux/macOS and the iOS adapter.
- **Reproduction:** start an isolated node and request a transport-state save. Every
  TaskFinished event queues another save, whose completion repeats the process.
- **Fix:** track internally submitted save tickets and consume their completion
  without scheduling another save. Failed saves report an error; inbound exchanges
  explicitly request persistence. Other commands retain their existing save behavior.
- **Evidence:** a real runtime regression failed after 32 continuing notifications;
  after the fix it persists `state.bin` and settles without further notifications
  (`/tmp/momentum-nearby-save-loop-red.log` and `-green.log`, 0.20 seconds). The 15-test
  shared nearby suite also passes. This verifies the loop, not physical-device energy
  consumption; foreground/background profiling remains required for iOS acceptance.


## B-043 — healthy Nextcloud status is absent from task lists

- **Severity:** Medium. **Feature:** F-028. **Affected:** iOS task screens; desktop
  frontends already have persistent status entries and are unchanged.
- **Reproduction:** configure Nextcloud without an error, then return to a task list.
  The old entry is guarded by `failure != nil`, hiding progress, pending changes and
  last success. The same guard affects both empty and populated list branches.
- **Fix:** a shared native navigation entry displays the selected provider, active
  progress/stopping state, last successful exchange and pending changes. Failures
  retain a persistent recovery entry. Every state opens the same Sync settings;
  selecting Off hides the entry. Absolute dates add no idle timer. The entry is a
  scrolling list row at every text size: the first top-inset version hid Today’s
  large title, caught by full-screen inspection and a failing title-pixel assertion.
  Native list disclosure replaces the component's custom chevron.
- **Evidence:** the new iOS 27 native test first fails because the healthy status
  entry is absent (`/tmp/momentum-status-link-red27.xcresult`), then passes after the
  change (`/tmp/momentum-status-link-green27.xcresult`, 22.134 seconds). Two extended
  native flows pass on iOS 26.5/27 (46.827/49.701 seconds). The initial test verifies
  the shared production component over a real loopback exchange, details navigation,
  successful status publication and Off. Full production-view Today/Upcoming/Search,
  quick-add, title pixels and AX5 scrolling checks now pass on both target runtimes
  (54.145s on iOS 27, 51.542s on iOS 26.5); final app build also passes. This verifies
  the visual/native interaction scope. Spoken VoiceOver remains part of F-020's
  outstanding accessibility acceptance. Bundles and revision scope are in PROGRESS.md.

## B-044 — task-open routing can collide with Keyboard Help

- **Severity:** Medium. **Feature:** F-023. **Affected:** iOS keyboard-help sheet only;
  no shared-core or desktop changes.
- **Reproduction:** open Keyboard Help, then route a task from an OS entry point.
  The existing presentation gate only considered capture/editor sheets and attempted
  a second sheet over help.
- **Fix:** keep the route queued while help is visible and retry it on dismissal,
  using the existing pending-notification/automation path.
- **Evidence:** the isolated native integration test fails before the gate change
  (`/tmp/momentum-keyboard-help-red26.xcresult`, September 17, dirty `task/ios-app`
  based on `7ec82e9`). The post-fix model/automation suite passes on iOS 26.5 and
  27. The actual iOS 27 help-sheet Done action now opens the queued editor and
  preserves the task (15.983s, `/tmp/momentum-keyboard-and-sync-final27.xcresult`).
  Matching iOS 26.5 native dismissal passes in 14.130s
  (`/tmp/momentum-keyboard-and-sync-final26.xcresult`), verifying the queued-sheet
  routing scope on both versions. This fix does not resolve F-023's separate
  failing navigation/Search keyboard commands.


## B-045 — iOS navigation and Search keyboard commands do not dispatch

- **Severity:** Medium. **Feature:** F-023. **Affected:** native iOS scene commands;
  no shared-core, Linux or macOS behavior changes.
- **Reproduction:** launch Today and press Command-2 or Command-F. Both the isolated
  Nextcloud QA scene and shipping app's isolated test mode remain on Today. The
  shipping regression fails before this fix in
  `/tmp/momentum-keyboard-shipping-probe27.xcresult` (September 17, dirty
  `task/ios-app`, base `7ec82e9`).
- **Cause and fix:** Settings/Command-Comma was registered inside Navigate instead
  of the standard app-settings group. Moving it to `.appSettings` restores that
  menu's dispatch. Search also used the text-editing insertion point, which did not
  dispatch from a task list; it now belongs to Navigate. Reduced scene tests
  isolated these placements while retaining the shared bindings, model, availability
  gates and action handlers. Repeated keys, explicit identities, renamed menu titles
  and replacing ForEach did not fix the original arrangement and are not retained.
- **Status:** In progress. The command-placement change restores navigation and
  Search in the warmed QA sequence; clean cross-version and shipping-entry results
  are recorded in the F-023/B-045 progress handoff. iOS 27 shipping Command-F still
  fails both immediately after launch and after Command-2 navigation. The same
  six QA scenarios and both shipping regressions pass on iOS 26.5. Registering QA
  commands eagerly, like the shipping scene, did not resolve the iOS 27 failure
  and that experiment was reverted. The retained change
  preserves every binding, including fixed Command-Comma after modifier changes.
- Temporary key-event tracing and probe tests were removed. This is not a claim
  about the framework's internal cause or a physical-keyboard acceptance pass.
- **September 17 follow-up:** the unchanged isolated QA Search test passes on a
  physical iPhone running iOS 27.0 (`/tmp/momentum-search-physical27.xcresult`,
  13.662s). It sends one Command-F after launch, types a query, and finds the seeded
  task. This is XCTest-generated input on a physical device, not a manually operated
  external keyboard or shipping-entry device acceptance. On a second iOS 27 simulator,
  the shipping navigation-then-Search test passes but cold Search still fails
  (`/tmp/momentum-search-second-device27.xcresult`). Keep both regressions enabled;
  the remaining simulator failure is not established as a production or XCTest bug.
- QA-only native Find configuration, held-modifier delivery, distinct Search labels,
  another text-editing insertion point and a priority UIKit key-command responder
  did not resolve the iOS 27 simulator failure. All experiments were removed; no
  Search-specific native adapter, diagnostic logging, warm-up or key retry was added
  to production.

- **2026-09-18 testing disposition:** the user removed the iOS XCUITest suite, so the
  synthetic key-dispatch regression no longer exists in source. This does not close
  the product behavior or turn prior failures into passes. Command binding and action
  logic remain unit-tested; actual cold hardware-key routing requires manual simulator
  evidence before this bug can be closed.

- **Latest physical evidence (2026-09-17):** the resumed seven-case device run
  reaches the tests after unlock. Six cases pass, but cold Search now also fails
  on the physical iOS 27 QA app (`/tmp/momentum-planning-keys-device27-unlocked.xcresult`,
  13.722s, overall exit 65). The screenshot remains on Today after Command-F and
  the assertion cannot find SearchField. This supersedes any interpretation that
  the earlier device pass establishes reliable cold Search. Manual external-keyboard
  behavior and the underlying dispatch cause remain unverified; no test is waived.

## B-046 — iOS task Delete keyboard acceptance does not reach its action

- **Severity:** Medium. **Feature:** F-023. **Status:** In progress.
- **Affected scope:** reproduced in iOS 26.5 simulator UI automation while adding
  task keyboard commands. The iOS 27 simulator and physical-device tests fail earlier
  at cold Select All (B-047), so neither establishes a Delete result. Physical-keyboard impact is
  unverified. No Rust, macOS or Linux behavior changed.
- **Reproduction:** launch the isolated task-list QA scene, select the seeded task
  with Command-A, and send Command-Delete. Selection works but the task remains;
  `testKeyboardTaskDeletionAndUndo` retains this as a separate failing regression.
- **Investigation:** a QA-only trace at `UIApplication.sendEvent` observed Command
  modifier events but no Delete press in `/tmp/momentum-task-delete-app-trace26.xcresult`.
  Earlier responder-only traces could not establish whether UIKit received the key.
  Named Delete, ASCII backspace and DEL attempts failed; a priority UIKit Delete
  command also failed. This evidence does not identify the upstream cause or prove
  that a physical keyboard is affected. Do not alter the shared binding or waive
  acceptance on that assumption.
- **Current implementation and next step:** task Delete uses the existing shared
  binding and Rust-backed delete operation. The named XCTest key and failure are
  retained; no key retries, input warm-up, event swizzling or Delete-specific adapter
  remains. Cross-runtime results follow in PROGRESS; verify physical key delivery
  and default-modifier deletion/undo before closing this scope. Selection, completion,
  text editing and modifier/tab isolation have independent tests so this failure
  cannot hide their results.

- **Simulator-only follow-up, 2026-09-17:** an independent System Events →
  Simulator keyboard path did not deliver the known-good Command-2 control on
  iOS 26.5. It therefore supplies no trustworthy Select All/Delete result and
  cannot replace or waive the existing XCTest failure. Disposable probes were
  cleaned; no production workaround or physical-device test was added.

- **2026-09-18 testing disposition:** the user removed the iOS XCUITest suite and its
  synthetic Delete input. Shared binding, enablement and Rust deletion/Undo behavior
  stay unit-tested. The unresolved input-delivery question remains In progress and
  needs manual external-keyboard simulator acceptance.

## B-047 — cold task Select All does not invoke the enabled iOS 27 responder

- **Severity:** Medium. **Feature:** F-023. **Status:** In progress.
- **Affected scope:** iOS 27 simulator and physical iPhone, isolated QA entry point
  using production task views. The same selection/completion/reopen/undo scenario
  passes on iOS 26.5.
  No macOS/Linux/shared-core change. Device automation reproduces the failure, but
  manually operated external-keyboard impact and the underlying cause are unverified.
- **Reproduction:** cold-launch the task-list fixture, wait for the seeded task row,
  then send Command-A as the first shortcut. The expected Actions (1) button never
  appears. Both independent selection and deletion tests fail at this prerequisite
  (`/tmp/momentum-task-keys-ready27.xcresult`).
- **Investigation:** temporary public responder lifecycle tracing confirmed successful
  first-responder acquisition in a key window. UIKit repeatedly queried Select All
  and received true while the view remained first responder; `selectAll(_:)` was
  never invoked (`/tmp/momentum-task-selection-lifecycle27.xcresult`). This rules out
  the tested hypothesis of failed initial focus acquisition; it does not establish
  why dispatch fails. A public native-menu rebuild after focus acquisition also
  failed (`/tmp/momentum-task-menu-refresh27.xcresult`). Both the experiment and
  temporary diagnostics were removed.
- **Device evidence:** `/tmp/momentum-task-keys-physical27.xcresult` reproduces the
  same cold Select All failure in both task cases (14.832s, 14.974s). Native Search
  text editing (21.982s) and modifier/tab/editor isolation (32.741s) pass on that
  device. These use production views in the disposable QA app, not the shipping
  entry point or manually operated keyboard.
- **Remaining validation:** investigate the remaining input/dispatch boundary and
  validate cold-start behavior manually. The modifier/help/tab/editor isolation scenario passes on both simulator
  versions, but its preceding interactions do not replace cold-start acceptance.
  Keep Delete independently tracked as B-046 and Search as B-045. Additional Open,
  Duplicate, Copy and Archive acceptance explicitly selects through touch; passing
  those action tests does not resolve this cold keyboard-selection failure. See
  the dated F-023 progress record for each runtime.

- **2026-09-18 testing disposition:** the user removed the iOS XCUITest source and
  target that injected Command-A. Responder/action state remains covered by unit
  tests; the previously reproduced dispatch failure remains open until a manual
  simulator keyboard check establishes current behavior.


## B-048 — selection Actions overlaps the native tab bar

- **Severity:** Medium. **Features:** F-010, F-018, F-020. **Status:** Verified for the recorded phone-simulator scope.
- **Affected scope:** reproduced visually on iOS 26.5 in the production TaskScreen
  hosted by the isolated QA app. The selected-task Actions button occupies the
  center of the tab bar and its symbol is partially obscured. macOS/Linux do not
  use this nested navigation/tab-bar layout; iOS 27 and largest-text validation
  follow below rather than being inferred from the screenshot.
- **Reproduction/evidence:** select the seeded task, open/edit through the keyboard,
  then dismiss the editor. The final screenshot in
  `/tmp/momentum-task-more-keys-final26.xcresult`, Open/edit/copy case, shows the
  overlap even though action dispatch assertions pass.
- **Cause and implementation:** Actions used a NavigationStack bottom toolbar inside
  a TabView. Both controls now share the bottom safe-area inset above the tab bar,
  using a horizontal row at standard sizes and a vertical stack at accessibility
  sizes. Its opaque system grouped canvas prevents scrolling text from showing
  behind the controls. Native geometry, actual menu completion and rendered Add ink
  are checked; existence alone cannot prove that a control is usable. The initial
  regression fails because Actions is not hittable (13.742s,
  `/tmp/momentum-selection-layout-red26.xcresult`).
- **Fixture correction:** per-argument defaults preserve an explicitly requested
  content size. A stacked-layout assertion and screenshots distinguish genuine AX5
  from the earlier incorrectly overridden standard-size dark run; see PROGRESS.

- **Verification:** September 17, dirty `task/ios-app`, base `7ec82e9`. Final
  light/standard-dark/genuine AX5-dark geometry, rendered white Add ink, touch-menu
  completion and scroll-to-result cases all pass on iOS 26.5 and 27
  (`/tmp/momentum-selection-canvas-final26.xcresult` and
  `/tmp/momentum-selection-canvas-final27.xcresult`). Final screenshots were inspected;
  the normal Momentum simulator build passes
  (`/tmp/momentum-task-actions-selection-build.log`). Physical-device/iPad/landscape
  and spoken VoiceOver acceptance remain outside this recorded fix scope.


## B-049 — duplicate selection can follow another task creation

- **Severity:** Medium. **Features:** F-006, F-023. **Status:** Verified for the
  actor identity regression and recorded phone-simulator duplicate flow.
- **Affected scope:** the iOS keyboard duplicate path awaited duplication, then
  separately awaited the global last-added ID. Another EngineWorker caller could
  create a task between those calls, causing selection to target the other task.
  The deterministic host test reproduces that actor-call ordering using the real
  core/store (`/tmp/momentum-duplicate-selection-red.log`, 0.160s failure). This is
  a selection error; it does not establish task loss or a reproduced device race.
  macOS's synchronous duplicate/ID sequence has no equivalent Swift actor suspension;
  this fix does not change Rust, Linux or macOS behavior.
- **Fix:** `EngineWorker.duplicateTaskWithID` returns the Outcome and optional copied
  identity from one uninterrupted actor method. The native list consumes that result
  directly; a failed/no-op duplication returns no identity. The existing Outcome-only
  API remains available for existing callers.
- **Fast evidence:** `TaskEditingTests` covers the returned identity surviving later
  creation, eight concurrent duplicates interleaved with other creations, unique copy
  IDs and invalid-source failure. The focused editing/organization suite passes all
  13 tests in 2.015s (5.296s including incremental build),
  `/tmp/momentum-planning-fast-focused.log`. Other create/last-added-ID call pairs
  are not claimed covered by this scoped correction.
- **Native/build evidence:** duplicate/archive/Undo passes on iOS 26.5 and 27 phone
  simulators (`/tmp/momentum-planning-keys-green26.xcresult` and
  `/tmp/momentum-planning-keys-green27.xcresult`); the normal Momentum simulator app
  builds successfully (`/tmp/momentum-planning-app-build.log`). These native cases
  verify serial user interaction; the fast actor tests prove concurrent identity
  isolation. Physical QA initialization was interrupted by device authentication,
  so this change has no new physical-device acceptance. All evidence is from the
  September 17 dirty `task/ios-app` worktree, base `7ec82e9`.

## B-050 — project destination label triggers maximum-text clipping audit

- **Severity:** Medium. **Features:** F-018, F-020, F-023. **Status:** Verified for
  the recorded maximum-text dark phone-simulator audit scope.
- **Reproduction:** in the new iOS Move to Project keyboard sheet, enable maximum
  Dynamic Type and run the native text-clipping audit. iOS 26.5 reports “Text clipped”
  for the Inbox label (`/tmp/momentum-organization-boundaries26.xcresult`, 17.804s).
  Exported element/screen attachments identify the combined folder/text label;
  multi-line project names also wrap under the symbol. This is an iOS sheet issue;
  Linux/macOS interfaces and shared rules are unchanged.
- **Correction:** render the decorative SF Symbol and vertically fitting title as
  separate baseline-aligned elements, with one project-name accessibility label on
  the button. Text wraps within its own column and the symbol is accessibility-hidden.
  The unfiltered contrast/hit-region/text-clipping audit remains enabled; no finding
  is suppressed. Final audits pass on iOS 26.5 (20.437s) and 27 (21.534s), with
  inspected screenshots and a successful normal app build. See the dated
  `/tmp/momentum-organization-final26.xcresult`,
  `/tmp/momentum-organization-final27.xcresult` and PROGRESS; no spoken VoiceOver,
  iPad or physical-device acceptance is implied.

## B-051 — keyboard New Project initially has no text focus

- **Severity:** Medium. **Features:** F-004, F-023. **Status:** Verified for
  keyboard-only creation through phone-simulator automation.
- **Reproduction:** open New Project with Command-Shift-N and type immediately,
  without tapping Name. The iOS 26.5 UI test fails because no element has keyboard
  focus (`/tmp/momentum-project-keyboard-entry26.xcresult`, 20.073s). The earlier
  creation test tapped Name and therefore did not establish a keyboard-only flow.
- **Correction:** the native project/tag editor binds its Name field to FocusState
  and focuses it when the creation sheet appears. Existing records keep their
  previous initial-focus behavior. The attempted defaultFocus-only approach did
  not fix the issue and was removed. No delays or warm-up input are retained.
- **Evidence:** direct application-level typing, Command-Return Save, creation in
  Lists and tab preservation pass on iOS 26.5 (28.308s) and 27 (29.344s), both exit 0
  (`/tmp/momentum-project-appearance-focus26.xcresult` and
  `/tmp/momentum-project-appearance-focus27.xcresult`). The normal app build passes.
  This is not manual external-keyboard or physical-device acceptance; no desktop
  focus code changes.

## B-052 — English repeat interval uses plural wording for one

- **Severity:** Low. **Features:** F-008, F-021. **Status:** Verified for the iOS
  phone-simulator editor and both compiled Apple catalogs; macOS native UI pending.
- **Reproduction:** the iOS 27 Repeat editor showed “1 weeks” in the keyboard flow
  (`/tmp/momentum-organization-final27-attachments/5BBFB9A1-F058-4E7C-9FF5-2B74918E2DD4.png`).
  Expected singular wording for one and plural wording otherwise.
- **Cause/fix:** both Apple application catalogs supplied German one/other forms
  but no English forms for `%u days`, `%u weeks`, `%u months` and `%u years`.
  Added the missing English variations, preserving the typed keys and German text.
  Recurrence rules and stored data are unchanged.
- **Regression evidence:** the new `macos/scripts/tests/test_repeat_catalogs.py`
  compiles each actual catalog and resolves resources with Foundation. Before the
  fix, both tests failed on all four English singulars (eight failures total);
  afterwards all 64 English/German checks pass at counts 0, 1, 2 and 99. The fast
  suite takes about 0.4 seconds on this host, without launching a simulator.
  Logs: `/tmp/momentum-repeat-catalog-red.log` and
  `/tmp/momentum-repeat-catalog-green.log`, dirty local worktree, 2026-09-17.
- **Affected-platform assessment:** iOS reproduced visually; macOS reproduced by
  compiled production resources and fixed in its catalog. Linux presents a static
  unit subtitle beside a separate SpinRow, so this Apple combined-string defect
  does not establish a Linux defect; Linux native assessment remains outstanding.
  Android remains planned. The separate repeat-description formatter is unchanged.
- **Native verification:** the first iOS 26.5 run shows the corrected “1 week” on
  screen but the new test incorrectly searched for that as a standalone element.
  Its actual combined accessibility label is “Every, 1 week”; the test now uses
  that label and the Stepper's named Increment button. This test-query failure
  (`/tmp/momentum-repeat-units26.xcresult`) is not a recurrence regression.
- **Native results:** corrected interval-change/save/reopen/stop test passes on
  iOS 26.5 (31.493s, `/tmp/momentum-repeat-label26.xcresult`) and iOS 27
  (32.510s, `/tmp/momentum-repeat-label27.xcresult`), both exit 0. Exported screens
  show “2 weeks”; assertions verify singular before increment and persisted plural
  after reopening. This scope does not claim German native interaction, macOS
  editor interaction, spoken VoiceOver or physical-device acceptance.

## B-053 — iOS 26 reports false contrast failures for readable Settings text

- **Severity:** Medium. **Features:** F-019/F-020. **Status:** Verified for the
  reproduced iOS 26.5 simulator scope; the app's rendered contrast is independently
  enforced and iOS 27 remains unfiltered.
- **Reproduction:** on iOS 26.5, the shipping isolated app's existing
  `LaunchTests.testSettingsCategoriesAndTaskPreferencePersist` completes preference
  persistence, opens Appearance and fails its native contrast audit at line 85.
  `/tmp/momentum-ax5-settings26-options.xcresult` (2026-09-17, dirty `task/ios-app`)
  identifies the “Colors” section header in its element screenshot.
- **Expected:** readable section headers, footers and actions meeting the platform
  contrast requirement without hiding real audit findings.
- **2026-09-17 investigation:** isolated Appearance audit reproduces light-mode
  failure while dark passes on iOS 26.5. Increasing the gray contrast, using
  hierarchical primary, explicit adaptive UIKit label color, or fixed-size layout
  did not remove the finding. Exported pixels verify solid-black text on the
  (242,242,247) canvas in the explicit-label experiment, 18.8194:1 contrast, yet
  Apple's audit still flags Colors. Original ink measured 4.5547:1. The same iOS 26
  behavior later reproduced for Notification section/footer text and the black
  Refresh Schedule label, while the equivalent iOS 27 workflows passed.
- **Fix:** strengthened the shared secondary-text token to a 5.5:1 normal-contrast
  floor across all supported native surfaces (7.05:1 with Increase Contrast) and
  kept Refresh Schedule primary text. XCTest handles only the exact verified iOS 26
  labels; every other label/runtime remains unfiltered. A native trait regression
  independently enforces the ratios, preventing the audit exception from masking a
  color-token regression.
- **Scope:** reproduced on the iOS 26.5 phone simulator in light mode; dark passes.
  Final isolated light/dark tests pass on iOS 26.5 (29.353s,
  `/tmp/momentum-appearance26-final.xcresult`) and unfiltered on iOS 27 (33.501s,
  `/tmp/momentum-appearance27-final.xcresult`). Shipping Appearance/Notification
  workflows pass on iOS 26.5 (three tests, 82.855s,
  `/tmp/momentum-secondary-native26-final.xcresult`) and iOS 27 (three tests,
  87.835s, `/tmp/momentum-secondary-native27.xcresult`). Exported iOS 26 pixels
  measure `#616164` on `#F2F2F7` at 5.5316:1 and black on white at 21:1.
  Spoken VoiceOver and physical-device acceptance remain separately deferred.


## B-054 — shared count messages use plural English for one

- **Severity:** Low. **Features:** F-017/F-021. **Status:** Verified for compiled Apple plural resources and
  the native iOS completion/summary regression; broader localization acceptance
  remains separate.
- **Reproduce:** complete the only task on Today. The iOS 26.5 screenshot shows
  “1 tasks completed” in feedback and “You completed 1 tasks. Time to switch off.”
  in the all-done summary (`/tmp/momentum-feedback26-after.xcresult`, 2026-09-17,
  dirty `task/ios-app`). Expected singular “task” for one, plural for other counts.
- Both texts come from MomentumKit Strings and its shared catalog; macOS impact
  is suspected from shared presentation resources but has not been reproduced in
  the running Mac app. Linux has separate resources; no reproduced Linux impact.
- Source inspection confirms the shared catalog has German one/other variants
  for these keys but no English plural variants. The adjacent completed-and-archived
  key has the same missing English variants; its native symptom is not yet reproduced.
- **Fix:** added English one/other variants for all 21 shared keys with the same
  missing-English-variant pattern: completion/summary, bulk task actions, relative
  times, recurrence/snooze wording, Upcoming description and linked-device count.
  This changes wording resources only; LibreSync implementation remains deferred.
  German wording, lookup keys and typed placeholders are retained.
- **Evidence:** `test_shared_plurals.py` compiles the real MomentumKit catalog and
  checks Foundation lookups for both languages at counts 0/1/2/99. Baseline:
  21 incorrect English singular outputs out of 168; after: all 168 pass (~0.4s).
  `/tmp/momentum-shared-plurals-before.log` retains the baseline. All 136 shared
  package tests pass (12.462s, `/tmp/momentum-shared-plurals-kit-tests.log`).
- Native iOS checks assert the exact completion banner and all-done description,
  then dismiss/Undo: iOS 26.5 23.150s (`/tmp/momentum-completion-plurals26.xcresult`)
  and iOS 27 26.285s (`/tmp/momentum-completion-plurals27.xcresult`). The inspected
  screenshot reads “1 task completed” and “You completed 1 task. Time to switch off.”
- B-052 covered repeat-editor intervals; this is the shared presentation catalog.
  Compiled German outputs pass, but this scope does not claim every affected
  message was exercised through a running German/macOS UI. Physical runs remain
  user-deferred; Linux resources are unchanged.

### B-054 consumer packaging evidence

Normal macOS and iOS simulator builds pass on 2026-09-17, dirty `task/ios-app`:
`/tmp/momentum-shared-plurals-macos-build.log` and
`/tmp/momentum-shared-plurals-ios-build.log`. The same 168-output verifier also
passes against the MomentumKit resources packaged inside each built application.
No source fallback or standalone-test bundle substitutes for these packaged checks.

## B-055 — repeated mobile snapshot reads rebuild all task rows

- **Severity:** Medium. **Features:** F-001/F-039. **iOS:** Verified for immutable task projection and native row/archive regression scope.
- **Cause identified in source:** Previously, TaskSnapshot.tasks flattened and filtered the entire
  listing on every access. TaskScreen reads it for selection, keyboard context and
  drag payloads during native view updates. EngineWorker already materializes the
  same ordered task array off the main actor while creating the snapshot.
- **Expected:** immutable presentation snapshots should reuse their task projection;
  filtering, ordering and task rules remain owned by Rust.
- **Fix:** TaskSnapshot retains the worker's existing ordered task array as an
  immutable value; view updates reuse it without changing Rust task semantics.
- **Verification:** opt-in Release `MomentumPerformanceTests` scheme measures
  100 reads of a real 1,000-task snapshot on the iOS simulator. The 26.5 baseline
  averages 15.939ms; the fixed path measures 2–4µs on 26.5 and 27, near the timer
  floor. Full fast suite (117 tests) and native Open/Complete/reopen/archive Search
  regressions and the production simulator build pass. Dated bundles and memory observations are in PROGRESS. Fixture
  creation/persistence and actor hops are outside the measured interval. No timing
  threshold, frame-rate or battery-life claim is made. Everyday tests skip this
  benchmark before creating its fixture. Shared Rust/macOS/Linux code is unchanged.

## B-056 — Plan for Today suggests only tasks already planned for Today

- **Severity:** Medium. **Features:** F-031. **Status:** Verified for the reproduced
  iOS simulator Shortcuts flow; macOS builds/tests pass, but native Mac Shortcuts
  interaction remains unverified.
- **Reproduction:** create an unscheduled task, run Momentum's **Plan for Today** App
  Shortcut and open its task picker. The real iOS 27 Shortcuts sheet contained no task
  row, so the action could not perform its purpose. The entity query supplied `.today`
  suggestions, which excluded the unscheduled task that needed planning.
- **Expected/fix:** entity suggestions used by Plan, Complete and Open should offer
  active tasks from all current lists. `MomentumTaskQuery.suggestedEntities()` now
  requests the shared `.all` scope. Rust still owns list membership and mutations;
  completed/archive filtering and stale-ID validation are unchanged.
- **Regression evidence:** the real system picker now offers the unscheduled task and
  planning moves it into Today. The complete five-action Shortcuts suites pass 5/5 on
  iOS 26.5 (`/tmp/momentum-appintents-all-actions26.xcresult`, 147.090s) and iOS 27
  (`/tmp/momentum-appintents-all-actions27.xcresult`, 180.360s). The final portable
  mobile suite passes 119 tests, both native view/model suites execute 46 tests with
  two intentional availability skips and no failures, MomentumKit passes 136 tests,
  and the macOS Debug app builds. No physical device or user task store was used.

## B-057 — cold notification action crashes after applying its mutation

- **Severity:** High. **Features:** F-007/F-038. **Status:** Verified for cold and
  resident notification actions on iOS 26.5/27 simulators; physical/locked checks are
  user-deferred.
- **Reproduction:** schedule a reminder, let SpringBoard deliver it, terminate Momentum,
  expand the notification and tap **Done**. The shared engine completed the task and the
  delivered notification disappeared, then the cold-launched app terminated with
  `NSInternalInconsistencyException: Call must be made on main thread` while UIKit updated
  its background state-restoration archive.
- **Cause:** the async `UNUserNotificationCenterDelegate` response method allowed the
  system completion continuation to resume away from the main actor after awaiting the
  engine. UIKit then ran state-restoration work on that executor. The task mutation,
  delivery identity and notification plan were already correct.
- **Fix:** use the completion-handler delegate method, transfer its one-shot Objective-C
  callback to `MainActor`, perform the engine route and delivery removal there, and call
  the completion on the same actor for success and rejection paths. A Debug-only system
  notification marker keeps OS cold launches on the disposable test store with sync and
  Spotlight disabled; Release builds ignore it.
- **Evidence:** the pre-fix native flow reproduced the crash in
  `/tmp/momentum-notification-action26-r4.xcresult`. The final real-system suite drives
  native scheduling and actual Notification Center routes: cold body activation, cold
  Done and resident Snooze pass 3/3 on iOS 26.5
  (`/tmp/momentum-notification-system-final26.xcresult`, 343.071s) and iOS 27
  (`/tmp/momentum-notification-system-final27.xcresult`, 340.348s). Body activation opens
  the exact task, Done completes once after termination, and Snooze clears delivery while
  leaving the task open. Package tests verify shared-owner deduplication, the exact
  one-hour replacement plan and auto-archive handling. Final `prepare`
  and portable gates pass, and both native view/model suites execute 47 tests with two
  intentional availability skips and no failures. Background
  replenishment, Focus, locked-device and physical-device behavior remain separate.

## B-058 — editing the iOS morning-summary time can consume the wrong time

- **Severity:** Medium. **Features:** F-038. **Status:** Verified on the iOS 26.5
  and 27 simulators; physical delivery remains user-deferred.
- **Reproduction:** enable Morning Summary, then use the inline wheel picker to change
  both hour and minute. Each wheel wrote through immediately. If the intermediate hour
  plus the old minute was already in the past, reconciliation accepted that intermediate
  time for the current day before the user reached the intended minute.
- **Expected:** editing is provisional until one explicit save; scheduling observes the
  final hour and minute together. **Cause:** the inline `DatePicker` bound directly to two
  `AppStorage` values, while preference observation reconciled after each separate wheel
  update. The shared at-most-once ledger correctly prevented a second same-day summary.
- **Fix:** the time row now opens a native medium sheet with a staged wheel picker and
  SF Symbol Cancel/Save actions. Save commits the final hour and minute together; Cancel
  leaves scheduling unchanged.
- **Evidence:** the pre-fix iOS 27 system run
  (`/tmp/momentum-notification-summary27-r2.xcresult`) ended with the intended preference
  time saved but the isolated ledger already claimed the earlier intermediate time. The
  final real SpringBoard summary delivers the exact counts, opens Today, and preserves
  the chosen time across relaunch on iOS 27
  (`/tmp/momentum-notification-summary27-r3.xcresult`, 100.790s) and iOS 26.5
  (`/tmp/momentum-notification-summary26.xcresult`, 81.800s). Focused Notification
  Settings suites also pass 2/2 on iOS 27
  (`/tmp/momentum-notification-settings27-atomic.xcresult`) and iOS 26.5
  (`/tmp/momentum-notification-settings26-atomic.xcresult`). The diagnostic accessibility
  traversal now includes the staged sheet and completes all 32 screens on iOS 27
  (`/tmp/momentum-summary-accessibility27-r2.xcresult`); its 117 retained raw findings
  remain an inventory, not a blanket compliance pass. No device or personal store was used.

## B-059 — iOS project and tag destinations ignore their saved colors

- **Severity:** Low. **Features:** F-004/F-037. **Status:** Verified on iOS 26.5/27
  simulators for list and grouped-heading presentation; broader color-setting acceptance
  remains tracked by the iOS parity checklist.
- **Reproduction:** create a project or tag with Custom color enabled, then inspect Lists
  or group a task list by Project/Tag. Before the fix, every destination symbol and every
  grouped heading used the same secondary label color even though the shared Rust model
  supplied each saved color. Task-row metadata already honored saved colors.
- **Expected:** saved project/tag colors remain visible wherever the destination is
  represented, while accessibility settings can replace color with a readable neutral
  treatment. Color must not carry the only meaning.
- **Fix:** Lists now applies each saved color to the project/tag SF Symbol while retaining
  primary text, and grouped section headings apply the same saved color. Colorful Labels
  off, Increase Contrast and Differentiate Without Color all use the shared secondary-text
  fallback. Native trailing swipe Edit Project/Edit Tag actions expose the existing editors
  without changing the context-menu path.
- **Evidence:** the shipping UI creates custom-color project/tag destinations, verifies
  their saved switch state before and after relaunch, keeps a parent/subtask family and
  multi-selection/tag assignment intact, then checks the exact two-task project deletion
  scope. It passes on iOS 27
  (`/tmp/momentum-organization-foundation27-r7.xcresult`, 100.758s) and iOS 26.5
  (`/tmp/momentum-organization-foundation26-r1.xcresult`, 102.163s test time). The retained
  iOS 27 screenshot was visually inspected at
  `/tmp/momentum-org-r7-export/8EA604E8-0352-4B47-8A1D-DFDDD220CFCA.png`. No physical device
  or personal store was used; Linux/macOS were unaffected by these iOS presentation files.

## B-060 — iOS tag autocomplete lacks complete native keyboard control

- **Severity:** Medium. **Features:** F-005/F-039. **Status:** Verified for the iOS 26.5/27
  simulator shipping flow and native responder-command boundary; physical keyboard testing
  is user-deferred with all physical-device work.
- **Reproduction:** enter `#a` when `alpha` and `alpine` exist. Before the fix, touch could
  accept a suggestion, but autocomplete exposed no selected row and did not handle Up/Down,
  Tab, Return or Escape. A multiline SwiftUI field also made the insertion cursor unavailable
  to reliable Unicode-aware completion replacement.
- **Expected:** the first suggestion is selected, arrows move selection, Tab/Return accept,
  Escape dismisses, touch remains equivalent and ordinary/Unicode text entry is preserved.
- **Fix:** the quick-add field now uses a native, dynamically sized `UITextView` bridge with
  explicit UTF-16/Unicode-scalar cursor conversion. Priority `UIKeyCommand` and HID paths own
  autocomplete keys only while suggestions are visible; the text delegate handles native
  Return input. Rows expose selected semantics plus a checkmark and restrained accent
  background, and touch calls the same completion path.
- **Evidence:** pre-fix `/tmp/momentum-tag-autocomplete-red27.xcresult` fails because no
  suggestion is selected. Final shipping suites pass 2/2 on iOS 27
  (`/tmp/momentum-quick-add-foundation27-r2.xcresult`, 80.567s) and iOS 26.5
  (`/tmp/momentum-quick-add-foundation26-r2.xcresult`, 78.330s), including parser persistence,
  Down Arrow, Tab, native Return input and touch. The native view regression dispatches the
  priority Return/Escape commands in
  `ios/DerivedData/TestReports/views-1789719784251416000.xcresult` and
  `views-1789719810528016000.xcresult`. XCUITest did not deliver direct Return/Escape
  `typeKey` events to the multiline view, so that simulator harness limitation is not
  represented as a physical-keyboard pass. Unicode replacement, rejected-draft retry and
  More Details parsing pass 3/3 in `/tmp/momentum-quick-add-regression27-r1.xcresult`.

## B-061 — iOS task editor project picker omits its selected accessibility value

- **Severity:** Medium. **Features:** F-006/F-020. **Status:** Verified on the iOS 26.5
  and 27 simulators for source-aware creation; spoken VoiceOver acceptance remains open.
- **Reproduction:** open a project, start Add task, choose More Details and inspect the
  Project picker through accessibility. The form selected the source project correctly,
  but the picker exposed an empty value instead of its visible project name.
- **Expected:** assistive technology and UI automation can determine the selected project
  before the task is created. **Cause:** the SwiftUI picker relied on implicit semantics;
  after assigning a stable identifier, its selected value was not preserved by the
  resulting accessibility element.
- **Fix:** the picker now supplies both a stable `task-editor-project` identifier and an
  explicit accessibility value resolved from the current project ID. Task rules and
  persistence remain in the shared Rust engine.
- **Evidence:** the pre-fix iOS 27 shipping run fails at the empty project value in
  `/tmp/momentum-task-creation-context27-red-r2.xcresult`. The complete Today, project,
  tag, Morning and Evening creation matrix passes 3/3 on iOS 27
  (`/tmp/momentum-task-creation-context27-final.xcresult`, 76.019s) and iOS 26.5
  (`/tmp/momentum-task-creation-context26-final.xcresult`, 76.733s), with no failures,
  skips or runtime warnings. No physical device or personal store was used.

## B-062 — creating a tag from task actions retains the completed selection

- **Severity:** Low. **Features:** F-004/F-010. **Status:** Verified on iOS 26.5/27
  simulators; no desktop/shared-core behavior changed.
- **Reproduction:** select a parent/subtask family, open Actions, choose New Tag, enter a
  name and Add. The shared engine creates and applies the tag, but the Actions control
  remains selected. Choosing an existing tag through the adjacent menu clears selection.
- **Expected:** every completed selection action returns the list to ordinary browsing.
  Cancel may retain selection so the user can choose a different action.
- **Cause/fix:** the new-tag alert awaited the same organization command but omitted the
  selection cleanup supplied to `TaskActionsMenu`. It now clears the selected IDs and
  exits edit mode after the awaited operation, matching existing-tag behavior.
- **Evidence:** the first expanded iOS 27 organization run reaches the new `trip` tag but
  fails because Actions remains visible at line 67
  (`/tmp/momentum-organization-new-tag27-r1.xcresult`). The complete project/tag family,
  relaunch and deletion flow then passes on iOS 27 in 118.447s
  (`/tmp/momentum-organization-new-tag27-r2.xcresult`) and iOS 26.5 in 118.510s
  (`/tmp/momentum-organization-new-tag26-final.xcresult`). Only disposable simulator data
  was used.

## B-063 — selection mode prevents the iOS task drag from beginning

- **Severity:** Medium. **Features:** F-011/F-012/F-013. **Status:** Verified for
  shipping row reorder and project/tag/day destination gestures on iOS 26.5/27
  simulators; assistive Move Up/Down invocation remains a separate acceptance gap.
- **Reproduction:** enter Select Tasks, select a row and drag it or a multi-selection.
  SwiftUI List owns the edit-mode gesture and the row's `.draggable` session never begins,
  so the existing `TaskTransfer` never reaches any drop destination.
- **Expected:** a selected row exposes a clear 44-point reorder handle that starts the
  same single/multiple payload, while ordinary non-selection rows keep native drag.
- **Cause/fix:** SwiftUI's edit-mode List gesture competes with the row drag modifier.
  Selection mode now presents a small `UIViewRepresentable` SF Symbol handle backed by
  `UIDragInteraction`. It exports the shared custom-UTI JSON plus the same plain-text
  proxy as `TaskTransfer`; its Dynamic Type symbol, secondary tint and localized label
  retain native accessibility behavior. Rust still owns ordering and Undo.
- **Platform assessment:** Linux/macOS are unaffected by this UIKit/List interaction;
  iOS is Verified for row reorder; Android remains Planned.
- **Evidence:** `/tmp/momentum-drag-drop27-red.xcresult` reproduces the absent drag
  session. The real native gesture then moves one task and a two-task selection, preserves
  their relative order and restores both with one Undo on iOS 26.5
  (`/tmp/momentum-drag-drop26-destinations-final-r2.xcresult`) and iOS 27
  (`/tmp/momentum-drag-drop27-destinations-final-r2.xcresult`).
  Focused shared-core reorder tests pass for Manual Order, grouping, descending order,
  hidden neighbors and invalid destinations. No physical device or personal data was used.

## B-064 — URL and paragraph imports discard their source list

- **Severity:** Medium. **Features:** F-014/F-015. **Status:** iOS Verified on 26.5/27
  simulators; shared fix Implemented for Linux/macOS with native desktop reruns outstanding.
- **Reproduction:** paste or drop a URL or long paragraph while Today, a project, a tag or
  a day-period list is active. Multiple short lines use that view, but URL/paragraph
  branches create the task in Inbox with no source metadata.
- **Expected:** every parsed text shape keeps the same destination; URL content becomes
  Notes and its readable host/path becomes the title.
- **Cause/fix:** `add_from_text` passed its `View` only to the short-lines branch.
  A view-aware notes helper now applies URL and paragraph tasks through `add_task` with
  the supplied destination before adding Notes or an explicit due date. The public
  explicit `add_task_with_notes` behavior remains unchanged.
- **Evidence:** the new core assertions fail before the fix because URL `due_day` is nil;
  `/tmp/momentum-paste27-r2.xcresult` independently shows the URL absent from Today while
  feedback incorrectly suggests success in that list. The focused parser regression and
  final two-runtime shipping suites above now pass, creating two Today lines and a Today
  URL task whose Notes retain `https://example.org/mobile-drop/`. Cross-app drag and marked-text IME
  acceptance remain open. No physical device, account or personal store was used.

## B-065 — selection-mode context menu captures long-distance task drags

- **Severity:** Medium. **Features:** F-011/F-013. **Status:** Verified on iOS 26.5/27
  simulators; desktop and shared-core behavior are unaffected.
- **Reproduction:** enter Select Tasks, select a row and drag its handle far enough toward
  a project, tag or day destination. The row's ordinary browsing context menu opens and
  consumes the gesture before the native task payload reaches the destination.
- **Expected:** selection-mode drags reach the visible destination strip. The selected
  task commands remain available from the Actions button.
- **Cause/fix:** the row kept a context-menu recognizer while SwiftUI List edit mode and
  the UIKit drag handle were active. Row context-menu content is now present only outside
  edit mode, where it remains useful; selection mode already exposes the same commands in
  its dedicated Actions menu.
- **Evidence:** the first iOS 27 project-destination attempts opened the context menu
  (`/tmp/momentum-drag-destinations27-green-r1.xcresult` and `-r2.xcresult`). After the
  fix, the expanded iOS 27 scheme passes all three tests
  (`/tmp/momentum-drag-drop27-destinations-final-r2.xcresult`), including a two-task project
  move plus tag and Morning moves. The matching iOS 26.5 scheme passes all three tests
  (`/tmp/momentum-drag-drop26-destinations-final-r2.xcresult`). Only disposable simulator
  data was used.

## B-066 — creating a case-insensitive duplicate tag mutates the existing tag

- **Severity:** Medium. **Features:** F-004. **Status:** Verified on iOS 26.5/27
  simulators; shared Rust identity behavior is unchanged.
- **Reproduction:** create a custom-color tag named `outdoors`, then use the standalone
  New Tag editor to save `OUTDOORS` without a custom color. The shared engine correctly
  returns the existing tag ID, but the editor treats that ID as newly created and sends
  an update that changes the existing spelling and clears its color.
- **Expected:** duplicate creation is idempotent. It must retain the existing tag's
  title and metadata and must not add another tag.
- **Cause/fix:** the editor could not distinguish a newly created ID from an ID resolved
  by Rust's case-insensitive identity rule. `EngineWorker.createTagIfAbsent` now captures
  existing IDs, asks Rust to resolve/create the tag, and reports whether the returned ID
  was new. Swift does not reproduce trimming or matching rules. The standalone editor
  applies metadata only when the ID was newly created; normal editing still renames,
  recolors and clears colors.
- **Evidence:** the focused package regression first failed to compile because the
  create-only result did not exist, then passes 1/1; all organization package tests pass
  7/7 and the full mobile lane passes 126 tests/21 suites. The expanded shipping flow
  passes on iOS 27 in 180.056s
  (`/tmp/momentum-organization27-final-r2.xcresult`) and iOS 26.5 in 187.383s
  (`/tmp/momentum-organization26-final-r2.xcresult`). It proves one case-insensitive tag,
  retained lowercase spelling and retained custom-color state using disposable stores.

## B-067 — saved Nextcloud section headers use secondary ink and trigger a stale native contrast report

- **Severity:** Medium. **Features:** F-020/F-025. **Status:** Verified on iOS 26.5/27
  simulators for the rendered contrast and shipping saved-connection flow; spoken
  VoiceOver and hosted Nextcloud remain separate open acceptance boundaries.
- **Reproduction:** save a Nextcloud connection, scroll the production form to
  `Connection Saved`, and run the native contrast audit. The `Connection` and `Options`
  section headers inherited the app's secondary-text token. After changing the visible
  header ink, XCTest could still report the old `Connection` contrast result even though
  the element screenshot contained primary black ink on the system grouped background.
- **Expected:** section headings use readable semantic primary ink in both appearances.
  A framework audit mismatch must remain narrowly bounded and must never hide a visible
  contrast failure or another element/type.
- **Fix:** both section headers now use `Color.primary`. The one accepted XCTest finding
  must be a contrast issue for the exact static-text label `Connection`, and the test
  independently rasterizes that element, finds its dominant background and requires a
  rendered contrast ratio of at least 7:1 with a material amount of foreground ink.
- **Evidence:** the original iOS 27 shipping matrix passed 7/8 cases and reproduced the
  header finding in `/tmp/momentum-nextcloud-shipping27-r1.xcresult`. The corrected exact
  audit passes in 20.762s at `/tmp/momentum-nextcloud-accessibility27-r5.xcresult`.
  The complete eight-case iOS 26.5 matrix passes at
  `/tmp/momentum-nextcloud-shipping26-r1.xcresult`; its database records 8/8 Success,
  including normal, English AX5 light and German AX5 dark audits. The broader sync
  package, view and five-case loopback transport lanes also pass on both runtimes.

## B-068 — installed sync QA host claims the production task URL schemes

- **Severity:** Medium. **Features:** F-031/F-040. **Status:** Verified on iOS 26.5/27
  simulators; the shipping app's URL registration and task behavior were unchanged.
- **Reproduction:** install the loopback `MomentumSyncTestHost`, then run the production
  Safari `momentum://add` acceptance. Before the fix, iOS could activate the QA host
  instead of `com.codedbydan.Momentum.ios`; the production test then waited on UI owned
  by the wrong process.
- **Expected:** only the shipping app registers `momentum`.
  The loopback host may export the shared public task UTType for Spotlight activities,
  but it must not own application URL schemes.
- **Cause/fix:** the fixture source excluded the production plist, but XcodeGen still
  resolved its generated target through `Momentum/Info.plist`. The host now has an
  explicit fixture-only plist with its display/scene metadata and exported task UTType,
  and no `CFBundleURLTypes`. Project generation preserves that boundary.
- **Evidence:** `/tmp/momentum-automation27-final-r1.xcresult` passes all five Shortcuts
  cases and fails only the URL case while the installed fixture plist contains both
  schemes. After the fix, the exact URL flow passes 1/1 on iOS 27 at
  `/tmp/momentum-url27-collision-fix-r1.xcresult` and iOS 26.5 at
  `/tmp/momentum-url26-collision-fix-r2.xcresult`. The rebuilt fixture plist was read
  back without `CFBundleURLTypes`; corrected Spotlight activation passes 2/2 with zero
  runtime warnings on each runtime at `/tmp/momentum-spotlight-activation27-r2.xcresult`
  and `/tmp/momentum-spotlight-activation26-r2.xcresult`.

## B-069 — iOS saves malformed or insecure Nextcloud connection records

- **Severity:** Medium. **Features:** F-025/F-027. **Status:** Verified for the
  connection/persistence/transport unit boundary on iOS 26.5/27; hosted acceptance
  remains part of F-025.
- **Reproduction:** enter a missing-scheme URL, remote HTTP URL, embedded credentials,
  query or fragment and choose Save Connection. The draft previously replaced the
  atomic Keychain record, then failed only after a transport attempt. A malformed
  record left by that path could also remain eligible for automatic sync.
- **Expected:** reject unsafe or ambiguous fields before cancellation or persistence,
  preserve the previous saved connection, identify the field in the UI and never admit
  an invalid saved record to transport.
- **Fix:** `NextcloudConnection.validated()` trims URL/user/folder presentation input,
  preserves password bytes, requires HTTPS except for localhost/IPv4/IPv6 loopback,
  and rejects user-info/query/fragment URLs. `NextcloudSyncState` validates before
  draining or saving, blocks invalid legacy records and presents a wrapping semantic
  error at accessibility sizes.
- **Evidence:** 24 focused connection/state tests pass; complete portable suites pass
  136/136. The five-case real loopback Swift/UniFFI/Rust WebDAV suite passes on iOS 27
  at `ios/DerivedData/TestReports/transport-1789754174230200000.xcresult` and iOS 26.5
  at `ios/DerivedData/TestReports/transport-1789754194672519000.xcresult`. The AX5
  error component passes phone/iPad widths. No hosted credentials or device were used.

## B-070 — asynchronous mobile form errors do not receive accessibility focus

- **Severity:** Medium. **Features:** F-006/F-017/F-020. **Status:** Implemented;
  rendered layout and focus wiring compile on iOS 26.5/27, while spoken VoiceOver
  focus order remains manual acceptance.
- **Reproduction:** make Quick Add, task save, recurrence save or project move fail.
  The visible error appeared without deterministic accessibility focus. The estimate
  field's visible correction text was also a separate element with no field hint.
- **Expected:** asynchronous failures become discoverable without color or a reading
  deadline, wrap at the largest text size, and keep inline correction guidance associated
  with the field a person is editing.
- **Fix:** one reusable `AccessibleErrorMessage` provides semantic error ink, an SF
  Symbol and an adaptive AX layout. Each asynchronous flow moves accessibility focus
  to the new error. Estimate retains keyboard focus and supplies the same correction as
  its accessibility hint. Nextcloud validation uses the same semantic presentation.
- **Evidence:** the production component renders within phone and iPad widths at AX5;
  the full unit lane passes 136 package plus 52 native tests on both supported simulator
  runtimes. Actual VoiceOver speech, rotor order and alternate input are not inferred.

## 2026-09-18 — iOS unit-only testing regression assessment

Removing the XCUITest targets is an approved testing-architecture change, not a
product bug or evidence that prior interaction findings were fixed. The replacement
suite covers portable behavior, production SwiftUI layout/rendering, accessibility
metadata, model integration, Keychain and isolated loopback transport through unit
targets. A repository-shape regression fails if an iOS UI-testing bundle,
`XCUIApplication`, or a retired UI-test source directory is added again. B-045–B-047
remain In progress because unit tests cannot establish hardware-key dispatch; other
gestures, system prompts, external-app flows and spoken assistive technology retain
their explicit manual acceptance boundaries.

## B-071 — iOS LibreSync groundwork is not reachable from the app

- **Severity:** High. **Features:** F-026/F-027/F-028. **Status:** Implemented; real
  two-peer simulator transport is Verified on iOS 26.5/27, while separate-device
  Bonjour/native interaction remains open.
- **Reproduction:** before the 2026-09-18 scope update, Settings exposed only Off and
  Nextcloud even though the mobile package contained a serial nearby lifecycle and the
  shared core could run LibreSync.
- **Expected:** people can select LibreSync, understand its state, pair and unlink
  devices, sync explicitly and recover without starting a second provider.
- **Fix:** Settings now exposes one Off/Nextcloud/LibreSync picker, status and recovery;
  the native device screen shows this device, a temporary six-digit code, discovered and
  linked peers, a one-time-code link sheet and destructive unlink confirmation. The app
  composes a Keychain-backed core runtime after opening the one shared worker. Provider,
  foreground and restore transitions serialize stop/drain/start, and local edits coalesce
  before a nearby exchange.
- **Evidence:** 140 portable tests pass, including lifecycle/event/provider/edit
  scheduling; the AX5 phone/iPad LibreSync screen renders in the native unit target.
  Real Swift/UniFFI/Rust peers pass pairing, bidirectional task changes, callbacks,
  foreground restart, stable identity/certificate reuse and unlink in both simulator
  runtimes. No physical device or personal store was used.

## B-072 — SwiftUI reconciliation can cancel Quick Add IME composition

- **Severity:** Medium. **Features:** F-005/F-015/F-020. **Status:** Verified at the
  production UIKit text boundary on iOS 26.5/27 simulators.
- **Reproduction:** while `UITextView` owns marked text, a model/suggestion update with
  a different value replaced the complete text storage and insertion point.
- **Expected:** marked text remains owned by the active input method until it commits;
  suggestion and paste updates must not discard the marked range.
- **Fix/evidence:** `QuickAddTextInput` now defers SwiftUI storage reconciliation while
  `markedTextRange` exists. The native regression creates real marked text, applies the
  production synchronizer and verifies both the composing text and range survive.
  Interactive third-party-keyboard acceptance remains manual.

## B-073 — cold iOS 27 Search, Select All and Delete depend on scene-command delivery

- **Severity:** Medium. **Feature:** F-023. **Status:** Implemented; manual external-key
  simulator acceptance remains before B-045/B-046/B-047 can be closed.
- **Reproduction:** the retired app-driving suite observed iOS 27 cold Command-F and
  Command-A failures and an iOS 26.5 Command-Delete failure even when the SwiftUI command
  was enabled; the shared binding and action logic were correct.
- **Expected:** these primary actions reach the same current model/list context on a cold
  launch while editors retain standard text behavior.
- **Fix:** public SwiftUI `onKeyPress` fallbacks route configured-modifier Search, fixed
  Command-A Select All and configured-modifier Delete through the existing action contexts.
  They return `.ignored` when the app/list action is unavailable, allowing native text
  input and system shortcuts to continue through the responder chain.
- **Evidence:** focused native unit tests prove cold Search state/focus revision, selection,
  configured Delete and wrong-modifier rejection; the complete 59-test view lane passes on
  iOS 26.5/27. Unit dispatch does not establish an external keyboard event, so the older
  bug IDs remain In progress rather than being waived.

## B-074 — iOS installed builds report a stale hardcoded marketing version

- **Severity:** Low. **Feature:** F-039. **Status:** Verified on dip17pm.
- **Reproduction:** build and install the current iOS target, then read the installed-app
  inventory. The project declares marketing version `0.4.0`, but the app reports `1.0`.
- **Expected:** the installed bundle reports the marketing and build versions defined by
  Xcode build settings so generated projects and release configuration stay authoritative.
- **Cause/fix:** `Info.plist` hardcoded both version strings. It now references
  `$(MARKETING_VERSION)` and `$(CURRENT_PROJECT_VERSION)`, and the XcodeGen source
  declares the same substitutions so project regeneration cannot restore the literals.
- **Evidence:** the signed Debug device build succeeds and `devicectl` reads back Momentum
  `0.4.0` build `1` from dip17pm after replacement installation. On 2026-09-19 a
  pre-install bundle readback caught the hardcoded value after it reappeared in the dirty
  worktree; the settings references were restored, a permanent runner regression was
  added, and a fresh signed install/readback again reports `0.4.0` build `1`. The sidebar
  follow-up exposed the generator as the remaining source on 2026-09-19; regenerating the
  project and all 12 runner regressions now preserve the substitutions.

## B-075 — the first iOS task snapshot moves the floating Add Task control

- **Severity:** Low. **Feature:** F-019. **Status:** Implemented; automated presentation
  policy passes on iOS 26.5/27 simulators, with visual simulator inspection pending.
- **Reproduction:** launch Today while the first task snapshot is still loading. The
  ProgressView-to-list transition participates in the same animated content replacement
  as later refreshes, so the bottom-right Add Task control can visibly shift.
- **Expected:** the floating control remains in a stable bottom inset and the first
  snapshot is assigned without movement. Later refreshes may animate when Reduce Motion
  is off.
- **Cause/fix:** initial and subsequent snapshots shared one animated update path, and
  the control lived inside changing content geometry. `TaskSnapshotTransition`
  makes the first assignment immediate and respects Reduce Motion; the control now stays
  in a stable safe-area inset independent of the snapshot.
- **Platform assessment:** iOS only; Linux/macOS layouts and Android's planned client are
  unaffected.
- **Evidence:** `ViewRenderingTests` covers immediate first load, later smooth refresh,
  search and reduced-motion policy. The complete unit lane passes 152 package tests and
  80 hosted SwiftUI tests with two intentional skips on both supported runtimes. Both
  Debug simulator app builds succeed. No manual visual or physical-device claim is made.

## B-076 — the iOS task-list sidebar enters from the wrong edge and cannot remain visible

- **Severity:** Medium. **Features:** F-003/F-004/F-018. **Status:** Implemented;
  adaptive policy and hosted-view boundaries pass on iOS 26.5/27, while manual
  portrait/landscape interaction remains pending.
- **Reproduction:** open Lists from Today on a phone. The old trailing pushed picker
  enters from the detail side and cannot become a persistent leading column on a capable
  landscape iPhone.
- **Expected:** compact layouts reveal a native leading sidebar and return to detail after
  selection. A regular-width container that fits the approved sidebar and accessible
  detail minima keeps both columns visible, without device-name checks.
- **Cause/fix:** Today used an independent `NavigationStack` destination rather than an
  adaptive workspace. `TaskWorkspace` now owns a `NavigationSplitView`, preserves its
  selected list across tab/size changes, prefers detail in compact mode and falls back to
  Today when a selected Morning, Evening, project or tag disappears.
- **Platform assessment:** iOS only; desktop navigation is unchanged.
- **Evidence:** package policy tests cover exact width/size-class boundaries, persistence,
  compact selection and each disappearing-context fallback. Hosted SwiftUI tests cover
  portrait, capable landscape and the combined-width boundary on both runtimes; app
  builds succeed. The leading-edge gesture and visibly persistent landscape columns have
  not yet been manually inspected.

## B-077 — Quick Add does not open the task it just created

- **Severity:** Medium. **Features:** F-005/F-006. **Status:** Implemented; actor and
  sheet-sequencing integration tests pass on iOS 26.5/27, with manual sheet-transition
  inspection pending.
- **Reproduction:** submit a valid single-line or multiline Quick Add entry. The sheet
  dismisses after creation but leaves the person on the list instead of opening the
  standard editor for the newly created task.
- **Expected:** successful single creation opens that task; multiline import opens the
  last created task. Failure or an unchanged outcome retains the current draft. A queued
  notification route waits until the editor closes.
- **Cause/fix:** the mutation returned only a changed outcome, so the UI had no durable
  identity to route after dismissal. Actor-isolated worker helpers now capture
  `lastAddedId()` before another caller can enter the worker. Quick Add returns a typed
  success, stores the identity in root-owned pending-editor state, dismisses, and lets
  the root present one standard `TaskEditor` before notification routing resumes.
- **Platform assessment:** iOS only; shared task rules are unchanged.
- **Evidence:** package tests cover concurrent single creation, multiline last identity,
  unchanged and persistence-failure outcomes. Native integration tests cover exact editor
  routing and queued notification ordering. Both complete unit lanes and app builds pass;
  no XCUI, ViewInspector or physical device was used.

## B-078 — Sync Settings starts redundant asynchronous work during back navigation

- **Severity:** Medium. **Features:** F-025/F-028. **Status:** Implemented; lifecycle
  integration tests pass on iOS 26.5/27, with manual back-transition inspection pending.
- **Reproduction:** push Sync Settings and immediately navigate back while status is
  loading. The page launches its own load/status task even though `MobileAppModel` already
  owns foreground refresh, so page disappearance can compete with the same state work.
- **Expected:** the app model remains the single lifecycle owner and Sync Settings only
  observes published state.
- **Cause/fix:** a page `.task` duplicated root-owned loading. It was removed; provider
  changes retain the existing drain/cancellation barriers and foreground refresh remains
  in `MobileAppModel`.
- **Platform assessment:** iOS only; transport and desktop provider behavior are unchanged.
- **Evidence:** `SyncSettingsIntegrationTests` mounts and dismisses the production screen
  while asserting no page-owned persistence/status load. Both native unit lanes and both
  simulator app builds pass. Hosted Nextcloud/TLS and physical navigation remain outside
  this evidence.

## B-079 — keyboard shortcut discovery is exposed on iPhone

- **Severity:** Low. **Feature:** F-023. **Status:** Implemented; capability policy and
  compiled production routing pass on iOS 26.5/27, with manual iPad presentation pending.
- **Reproduction:** inspect Settings, Help or app commands on iPhone. The app exposes
  keyboard shortcut discovery intended for the iPadOS hardware-keyboard experience.
- **Expected:** iPad shows the discovery row, help, scene commands and fallback handlers;
  iPhone omits them while ordinary software-keyboard editing remains unchanged.
- **Cause/fix:** shortcut presentation had no injected platform-capability boundary.
  `MobilePlatformCapabilities` now receives the UIKit interface idiom and gates all four
  entry points through one testable policy.
- **Platform assessment:** iOS/iPadOS presentation only; desktop shortcuts are unchanged.
- **Evidence:** package tests prove phone omission and iPad inclusion; production sources
  compile in both native unit lanes and app builds. External-key delivery and an actual
  iPad help presentation remain manual under F-023.

## B-080 — the iOS accent checkmark does not follow the live selection

- **Severity:** Low. **Features:** F-019/F-020. **Status:** Implemented; binding and
  accessibility metadata pass on iOS 26.5/27, with the full visual trait matrix pending.
- **Reproduction:** change the accent preview while the Appearance list is mounted. The
  visible color updates, but a row-local checkmark can remain on the previous choice.
- **Expected:** the resolved selected color ID drives one trailing native checkmark and
  selected accessibility trait immediately; the swatch remains a separate cue.
- **Cause/fix:** row presentation did not derive all selection state from the live
  resolved binding. The extracted accent row now uses that identity for its checkmark,
  trait, restrained transition and selection feedback.
- **Platform assessment:** iOS only; the approved palette and default orange are unchanged.
- **Evidence:** a mounted production-row regression changes the binding and checks public
  UIKit accessibility metadata. Palette/motion coverage and both unit lanes pass. Manual
  light/dark, Increase Contrast and Dynamic Type inspection remains pending.

## B-081 — completion and selection controls compete in iOS selection mode

- **Severity:** Medium. **Features:** F-010/F-020. **Status:** Implemented; hosted row
  semantics pass on iOS 26.5/27, with manual touch/assistive inspection pending.
- **Reproduction:** enter Select Tasks. Each row retains its ordinary completion circle
  beside the native list-selection indicator, presenting two nearby controls with
  different state-changing meanings.
- **Expected:** selection mode exposes only the native row-selection affordance and bulk
  Actions menu. Browsing mode retains completion and its accessibility actions.
- **Cause/fix:** `TaskRow` did not distinguish browsing from selection presentation. An
  explicit row mode now omits the completion control and action in selection while
  preserving ordinary completion behavior.
- **Platform assessment:** iOS only; shared bulk/completion rules are unchanged.
- **Evidence:** hosted SwiftUI tests assert that the selection row omits and the browsing
  row retains the production completion control. Selection feedback integration tests,
  both complete unit lanes and both app builds pass. No spoken assistive or physical
  device claim is made.

## B-082 — Morning & Night can label tomorrow tasks as Today

- **Severity:** Medium. **Features:** F-003/F-037. **Status:** shared-core behavior
  Verified by fresh Rust/CLI/Apple consumer automation; native Linux runtime remains
  unverified on this Mac.
- **Reproduction:** select Morning & Night grouping in Upcoming, a project, a tag or
  search containing an untagged task due tomorrow. The generic grouping fallback maps it
  to `TaskGroup::Today`, producing a false Today heading.
- **Expected:** Morning & Night sections exist only where Today/Morning/Evening describe
  the active day view. Other views keep one plain section and show due-day metadata on rows.
- **Cause/fix:** `task_group` used Today as the fallback outside a Today-style view, and
  `group_listing` assumed every row must receive a group. The former now returns no group
  outside day views; the latter preserves the original plain listing in that case.
- **Platform assessment:** shared core affects Linux, macOS, iOS and CLI consumers;
  Android remains Planned.
- **Evidence:** the focused Rust regression covers Upcoming, project, tag and search plus
  preserved Today/Morning/Evening, completed, family, ordering and Undo behavior. Both
  workspace Rust commands, the 23-test CLI suite, MomentumMobile's 152 tests,
  MomentumKit's 136 tests, generated Apple core build and both iOS app builds pass.
  `build-aux/test.sh` stops at `flatpak: not found`, so no Linux runtime pass is claimed.

## B-083 — compact Today presents a back affordance instead of an explicit sidebar control

- **Severity:** Medium. **Features:** F-018/F-020. **Status:** Verified on iOS 26.5/27
  simulators; corrected build installed and launched on dip17pm.
- **Reproduction:** open the compact Today workspace. The detail-first
  `NavigationSplitView` can synthesize a navigation back affordance for its hidden leading
  column, which communicates navigation history instead of the persistent list hierarchy.
- **Expected:** Today has a leading native button with the `sidebar.leading` SF Symbol and
  localized **Show lists** label. It reveals the sidebar from the leading edge. Ordinary
  pushed task destinations retain their standard back behavior.
- **Cause/fix:** the split view owned the compact column transition but left its root
  navigation item implicit. The first correction replaced the icon but only mutated
  `preferredCompactColumn`/column-visibility state. Apple documents that visibility is
  ignored after a split view collapses, and the device showed that this state change did
  not perform the navigation pop. The root `TaskScreen` now hides the synthesized Back
  item and its replacement button invokes the environment's native dismiss action. The
  split view then reports the resulting sidebar state through its binding. Nested screens
  do not receive the replacement button and retain native back navigation.
- **Evidence:** a public-API `UIHostingController` regression renders the real compact
  workspace, finds the single actionable leading control, asserts no Back accessibility
  element, retains a screenshot, invokes the control, requires the sidebar marker to
  become visible and the detail marker to disappear, and verifies the resulting binding
  state. This stronger assertion failed against the installed implementation before the
  native-dismiss fix. Focused and complete unit lanes pass on iOS 26.5 and iOS 27. Each
  complete lane runs 152 portable tests in 23 suites plus 90 native tests with two
  intentional opt-in skips. The corrected signed `0.4.0` build `1` installs and launches
  on dip17pm, and CoreDevice confirms a live Momentum process. Touch interaction is not
  claimed from process readback, and no user data was inspected.

## B-084 — configured sync providers have no connection test

- **Severity:** Medium. **Features:** F-025/F-026/F-043. **Status:** Implemented; shared,
  portable-mobile and focused iOS 27 native tests pass. Hosted and separate-device checks
  remain explicit acceptance limits.
- **Reproduction:** save a Nextcloud connection or link a LibreSync peer. Sync Settings
  previously offered only a full task exchange, so credentials or reachability could not
  be checked independently.
- **Expected:** every selected, configured provider offers one accessible **Test Connection**
  action with testing, success and actionable failure states. Off shows no action.
- **Cause/fix:** the transport boundary exposed exchange but no non-mutating health action.
  Nextcloud now runs a cancellable authenticated PROPFIND probe and accepts a missing target
  collection after the account root succeeds. LibreSync starts or joins an outbound cycle
  and awaits only that cycle ID, including the event-before-return race. Neither path changes
  tasks, pending operations or persistent last-sync state.
- **Evidence:** `momentum-sync` and `momentum-core` regressions cover read-only success, missing
  collection, auth, deadline, cancellation and no state mutation. MomentumMobile covers
  provider lifecycle, exact-cycle attribution, unrelated events, timeout and cancellation.
  The iOS 27 transport Nextcloud suite passed seven tests with one intentional hosted-TLS
  skip, and Sync Settings integration passed ten tests. See PROGRESS for result bundles.

## B-085 — task-list sync status consumes primary vertical space

- **Severity:** Medium. **Features:** F-028/F-043. **Status:** Implemented; unit and focused
  hosted-view coverage pass, with final visual simulator inspection recorded in PROGRESS.
- **Reproduction:** enable a provider and open Today. The previous status entry appeared
  near the top of the list and competed with tasks and the empty state.
- **Expected:** one compact, tappable line after list content says `Last synced just now`,
  a localized minute/absolute time, `Not synced yet`, `Syncing…`, or an attention state.
- **Cause/fix:** `TaskScreen` placed the full provider summary before task content. The
  replacement footer uses footnote styling, a 44-point target and a minute-cadence
  `TimelineView` only while visible; tapping it opens Sync Settings.
- **Evidence:** status formatting and production Sync Settings integration tests pass.
  Empty and populated layouts remain part of the complete two-runtime native matrix.

## B-086 — the floating Add Task button reserves a full-width bottom strip

- **Severity:** Medium. **Features:** F-019/F-020. **Status:** Implemented; build and hosted
  geometry regressions pass, with final visual simulator inspection recorded in PROGRESS.
- **Reproduction:** open a task list. The bottom safe-area inset keeps the capsule stable
  but paints and reserves the entire row, hiding useful canvas and blocking surrounding taps.
- **Expected:** only the bottom-trailing capsule occupies visible space; the last list row
  can still scroll clear of it and its position remains stable on first load.
- **Cause/fix:** the control lived in a full-width safe-area inset. It is now a trailing
  overlay with an invisible scroll-content bottom margin. Its white default-orange label,
  Increase Contrast behavior, accessible sizing, animation and haptic policy are unchanged.
- **Evidence:** the app builds and the existing floating-control/startup regressions cover
  stable first-load geometry, Dynamic Type and foreground contrast.

## B-087 — routine task-operation feedback remains persistent and screen-owned

- **Severity:** Medium. **Features:** F-017/F-019/F-020. **Status:** Implemented; deterministic
  timeout, exact undo and persistent-error tests pass on iOS 27. Two-runtime completion is
  recorded in PROGRESS.
- **Reproduction:** delete or update a task, then switch tabs. The old banner remained until
  dismissal and each retained task screen owned a copy.
- **Expected:** one root-owned Liquid Glass toast appears above navigation, ordinary text
  disappears after 3.5 seconds, exact Undo remains for 5 seconds, and `SaveFailed` stays
  focused with Dismiss.
- **Cause/fix:** feedback was an untyped message held by each task screen. `MobileAppModel`
  now owns a typed presentation with a unique ID, severity and optional undo batch. One
  cancellable sleep drives transient deadlines; the root overlay respects Reduce Motion,
  contrast and transparency settings and invokes `undo_batch` with the captured ID.
- **Evidence:** `TaskMutationIntegrationTests` passes nine tests on iOS 27, including
  replacement deadlines, exact-batch Undo and persistent save failure. Focused model tests
  use an injected sleep seam and do not wait on wall-clock deadlines.

## B-088 — saved project and tag rows lack filled color identity and task counts

- **Severity:** Low. **Features:** F-004/F-020. **Status:** Implemented; shared count and
  focused hosted-view regressions pass.
- **Reproduction:** open Lists with colored projects or tags. Previous symbols read as
  outline accents and rows did not show how many active task families they contained.
- **Expected:** the full `folder.fill` or `tag.fill` symbol uses the saved color and each
  row shows its localized unfinished-family count. Color is supplementary to shape/text.
- **Cause/fix:** SwiftUI iterated editing projections without canonical counts. The Rust
  sidebar projection now maps every direct member to its live root family, deduplicates
  parent plus child membership and excludes completed/archived roots. `ListPicker` consumes
  those entries and applies the existing Colorful Labels, Increase Contrast and
  Differentiate Without Color fallbacks.
- **Evidence:** shared-core tests cover child-only membership, deduplication and completion.
  The production hosted view test verifies filled SF Symbols, saved colors and accessible
  counts. Linux/macOS compile against the additive field; their current rows are unchanged.

## B-089 — ordinary task lists eagerly build the global search/archive index

- **Severity:** High. **Features:** F-001/F-003/F-004/F-039/F-044. **Status:** Verified
  for the shared-core behavior and iOS 26.5/27 first-snapshot boundary.
- **Reproduction:** open Today, Upcoming, a project or a tag with a large task store. Before
  producing the requested rows, `Engine::listing` lowercased every searchable field and
  deserialized both archive tiers even though those screens never consume the index.
- **Expected:** ordinary task lists perform only their requested projection. Search and
  Archive may build and reuse the invalidation-aware global index after a mutation.
- **Cause/fix:** the listing entry point created `SearchIndex` unconditionally and recurrence
  startup parsed every archived task just to check generated IDs. Index creation is now lazy
  for Archive (Search keeps its existing path), and recurrence checks archive entity-map keys.
  No second UI snapshot cache was added, avoiding stale duplicated task state.
- **Evidence:** a red/green core regression proves Today leaves the index absent and Archive
  creates it; another preserves archived-repeat deduplication without task deserialization.
  The Release unit/view benchmark measures a cold persisted-store reopen through the full
  first 1,000-task Today snapshot at 80.8 ms on iOS 27 and 67.0 ms on iOS 26.5 with a
  100 ms regression ceiling. See PROGRESS.

## B-090 — stale conditional iOS destinations remain on the loading spinner

- **Severity:** High. **Features:** F-003/F-004/F-018/F-039. **Status:** Verified on
  the iOS 26.5 and 27 simulator unit hosts; physical interaction was not rerun.
- **Reproduction:** expose Morning in the Today sidebar, then make that conditional
  destination disappear before its detail snapshot is accepted, such as through a sync
  commit or local-day change. Selecting Morning can remain on “Loading tasks” indefinitely.
  The same stale-destination path applies to Evening, deleted projects and deleted tags.
- **Expected:** every completed snapshot ends the loading state. Conditional destinations
  still fall back to Today or dismiss according to their navigation context.
- **Cause/fix:** `TaskScreen` returned on `viewExists == false` before assigning the completed
  snapshot. Workspace reconciliation was asked to change routes, but the disappearing detail
  still owned a nil snapshot and could retain its spinner. Snapshot assignment now happens
  first; the existing dismissal/fallback policy runs afterward.
- **Evidence:** the hosted SwiftUI regression failed before the change because no snapshot
  assignment arrived and the loading state timed out, then passed after the ordering fix.
  A second regression switches through the compact sidebar into a valid Morning list and
  verifies its task appears. The complete lane passes 156 portable tests plus 95 native
  view/integration tests with two intentional opt-in skips on both supported runtimes.

## B-094 — closing the Linux window aborted the process from a panic during finalization

- **Severity:** Medium. **Feature:** F-024. **Status:** Implemented; GTK regression test
  added (runs in CI's Flatpak jobs). Found on 2026-10-01 while running the GTK app
  natively on macOS, where closing the window does not end the process.
- **Reported behavior:** closing the window aborted with "thread caused non-unwinding
  panic" (`momentum-2026-10-01-102019.ips`): `MomentumWindow`'s `Drop` asked the tag
  popover for its parent after GTK had finalized the widget, the gtk-rs instance check
  panicked, and a panic inside `g_object_unref` cannot unwind.
- **Expected:** closing a window finalizes it quietly; on GNOME this usually coincides
  with quitting, which is why it went unnoticed.
- **Fix:** the popover is unparented in `ObjectImpl::dispose`, while widgets are alive;
  `Drop` keeps only the engine teardown. `closing_a_window_finalizes_it_without_a_panic`
  opens the popover's parent, closes the window and pumps the loop.
- **Note:** GTK tests cannot run natively on macOS: GTK there must own the process main
  thread, and the test harness runs GTK on a worker thread, which Linux permits.

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Implemented — fix and test; CI runtime | Not applicable — SwiftUI app; the native GTK run is a developer convenience | Not applicable | Not applicable |

## B-093 — the macOS app crashed when the last Today task was completed

- **Severity:** High. **Features:** F-003/F-019. **Status:** Fixed (structural); the
  exact user gesture could not be replayed here because the screen was locked, so the
  live checkbox check is recorded as pending in PROGRESS.
- **Reported behavior:** completing the last task of Today crashed the Mac app
  (2026-09-30 18:03 CDT, release 0.4.3). A crash with the same signature happened on
  2026-09-29 19:38 CDT on 0.4.1 while a sync download replaced the listing.
- **Crash:** `EXC_BREAKPOINT` in `ViewListTree.visitItem` called from
  `OutlineListCoordinator.outlineView(_:child:ofItem:)` while AppKit's
  `NSTableView.endUpdates` estimated row heights inside SwiftUI's
  `OutlineListCoordinator.diffRows`: the outline view asked for a row SwiftUI's list
  tree no longer had.
- **Cause:** the user archives completed tasks immediately and groups Today by tag.
  Completing the last task removes the last section and, with nothing left, the core
  returns an empty listing; the view then replaced the whole `List` with the
  empty-state view in the same animated update that removed the final rows, while
  the sections were identified by offset (`id: \.offset`) so a Completed section and
  the day section it replaced looked like the same item, and loose top-level rows
  (the all-done panel, Show More) sat beside sections. The socket-driven path without
  animation never crashed in a demo copy; the checkbox and Space paths animate.
- **Fix:** the `List` stays mounted and the empty state is an overlay; every section is
  identified by `SectionIdentity` (kind + group, `MomentumKit/Listing.swift`); the
  all-done panel and Show More are sections of their own. A MomentumKit test asserts
  every listing's section identities are distinct and that emptying Today never
  reuses a day section's identity for Completed.
- **Evidence (2026-10-01, throwaway demo copies with the user's two preferences, driven
  in the background through the CLI socket and the accessibility checkbox):** a
  pre-fix build survived ticking the last checkbox, ticking it while thirty rows were
  inserted through the socket, and socket-driven completion of 28 rows, so the exact
  trigger on the user's Mac (a Nextcloud exchange was running on another thread at
  crash time, and the diff ran inside a selection-update guard) was not replayed; the
  fixed build takes the same checkbox gesture to a clean empty state and refills.
  Mac package 142 passed (new identity test). iOS: `ViewRenderingTests` drives the same
  transition (immediate archive, tag groups, three tasks completed to empty, one added
  back) on the UIKit-backed list, whose empty state already lives inside the List.
  Linux: the GTK list is rebuilt row by row with the empty page toggled by visibility;
  the new GTK test completes every Today row the same way and refills; it compiles
  here and runs in CI's Flatpak jobs. Signed macOS release reinstalled.

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Implemented — not an outline view; GTK regression test added (runs in CI) | Implemented — structural fix; fixed demo copy verified by checkbox; the user's exact trigger not replayed | Implemented — UIKit-backed List; view-lane regression test drives the same transition | Not applicable — no app |

## B-092 — a cut-off Nextcloud upload was stored as the sync file and locked every device out

- **Severity:** High. **Features:** F-028/F-054. **Status:** Fixed in the core; the
  user's server was repaired on 2026-09-29 and all three copies agree (version 85).
- **Reported behavior:** the phone, on cellular through the Tailscale relay, showed a
  network-style sync failure while its connection test passed; a few minutes later the
  Mac reported "could not decrypt the sync file: Invalid input length: 4849657".
- **Reproduction (from the server's timestamps and both devices' metadata):** the Mac
  synced at 17:43:41 CDT (version 83). The phone downloaded that copy at 17:46:04, then
  uploaded the merge of its nine pending edits. At 17:46:48 the 30 s whole-exchange
  deadline closed the connection after 4,849,657 body bytes. Nextcloud, behind an nginx
  that streams request bodies through, stored the partial body and answered with a
  fresh ETag. A base64 body of length 1 mod 4 cannot be decoded, so every later
  download on any device failed at the decode step.
- **Expected:** a transfer that stops early is never accepted as the sync file; a large
  file over a slow link gets the time it needs; the failure is named in plain words.
- **Cause:** one fixed 30 s budget for download, merge and upload; no upload integrity
  check; a decode failure classified as a generic "connection" problem on iOS and shown
  raw on the desktops. The connection probe reads no file, so it kept passing.
- **Fix:** every upload sends `OC-Checksum` (SHA-1) and `X-Expected-Entity-Length`,
  which Nextcloud verifies, and reads the stored size back with PROPFIND; a mismatch is
  `SyncError::UploadIncomplete` and the pending edits stay. The foreground budget is
  five minutes with 20 s connect and 45 s first-byte limits; background wakes pass 25 s.
  Decryption tells a damaged body (`Decrypt`) from a wrong password
  (`WrongEncryptionPassword`). `SyncError::kind()` → `SyncFailureKind` rides on
  `CoreError::Actionable`/`Transient`. `replace_remote` publishes one device's full data
  after parking the damaged copy as `sync-data.json.damaged`; exposed as
  `Engine::replace_remote_sync_file`, `mo sync --replace-remote`, and a Replace Server
  Copy action in each app.
- **Recovery performed:** the corrupt file was downloaded and kept with local backups
  of both stores under the Mac's Momentum `backups/` folder; `mo sync --replace-remote`
  published the Mac's copy (version 84); the phone downloaded it, re-applied its nine
  edits and published version 85; the Mac downloaded version 85.
- **Regression evidence:** momentum-sync tests `a_cut_off_upload_is_reported_and_never_replaces_the_previous_copy`,
  `a_damaged_server_copy_is_told_apart_from_a_wrong_password_and_can_be_replaced`,
  `replace_remote_refuses_a_fresh_device_and_works_without_a_server_copy` (mock proxy keeps
  half a body; truncated encrypted body; COPY aside); core transport tests answer the
  size read-back. Platform evidence is in PROGRESS (2026-09-29 evening).

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Implemented — same core; dialog names the failure and offers Replace Server Copy (GTK test) | Implemented — same core; popover names the failure and offers Replace Server Copy; signed 0.4.3 release installed and syncing | Implemented — fixed core on the device synced version 85 live; failure section names the kind, shows Details, acknowledges Try Again, offers Replace Server Copy (view test) | Not applicable — no app |

## B-091 — a Dock click could open two macOS main windows

- **Severity:** Medium. **Feature:** F-024. **Status:** Implemented on macOS; the
  scripted reopen check is recorded in PROGRESS (2026-09-28).
- **Reported behavior:** with the app kept running in the background and its window
  closed, clicking Momentum in the Dock opened two main windows.
- **Expected:** exactly one main window exists at a time; a reopen brings it forward.
- **Cause:** the main scene was a `WindowGroup`. On a reopen with no visible window the
  app delegate opened the group's window itself while SwiftUI's own reopen handling could
  create another window of the same group, and window restoration could add more.
- **Fix:** the main scene is a single-instance `Window(id: "main")`. `openWindow`, the
  delegate's reopen path, the menu bar item, the ⌃⌥M shortcut and URL/Spotlight handoff all
  resolve to that one window. `New Window` is not offered.

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Unaffected — one GTK window by design | Implemented — reproduced by report; scripted reopen shows one window | Not applicable — single scene | Not applicable — no app |

- **Remaining:** a manual Dock click on the installed app after the next release.

## 2026-10-02 — F-055 verification assessment

No reproduced existing product defect was identified by the Apple Reminders work.
The initial sandboxed Rust run could not bind test sockets; the unsandboxed isolated
workspace suite passed. EventKit background delivery and identifier-reset boundaries
are documented feature limitations, not newly invented bugs. Native permission and
assistive-technology acceptance remain in [the progress ledger](PROGRESS.md).

## B-095 — Apple Reminders import traps in a background EventKit callback

- **Severity:** High. **Feature:** F-055. **Status:** Implemented on iOS; physical crash signature confirmed and corrected build installed; manual import acceptance remains open.
- **Reported:** importing a reminder on the installed iOS app crashes. On 2026-10-02,
  the first 2026-10-01 evening report and latest 2026-10-02 12:17 CDT report were
  retrieved from the paired phone. Both identify the old non-Sendable nested
  projection in `EventKitRemindersSource.incompleteReminders(in:)`, with
  `_dispatch_assert_queue_fail` and `_swift_task_checkIsolatedSwift` on
  `com.apple.eventkit.reminders.search` (`EXC_BREAKPOINT` / `SIGTRAP`).
- **Reproduced:** original PR head `4a40345` traps with `EXC_BREAKPOINT`/`SIGTRAP`
  when real EventKit returns one synthetic reminder in a fresh iOS 27 simulator.
  The stack enters Swift's actor-executor check from the nested projection closure
  on `com.apple.eventkit.reminders.search`.
- **Expected:** callbacks may arrive on EventKit's queue and safely return immutable
  snapshots to the main-actor import coordinator.
- **Cause/fix:** the legacy Objective-C completion inherits main-actor isolation
  through the nested map closure. Use an explicitly `@Sendable` completion and a
  `nonisolated` projection; no EventKit objects cross into the resumed caller.
- **Regression:** the same nonempty real EventKit fixture passes with the correction;
  three portable tests cover background success/failure and synchronous completion.
  The real-framework regression is simulator-only and opt-in, touches only its own
  UUID-named synthetic list, and never requests permission.

| Linux | macOS | iOS | Android |
|---|---|---|---|
| Unaffected — no adapter | Uses corrected shared adapter; callback tests pass | Implemented; simulator regression passes; physical cause confirmed, corrected app installed/launched | Not applicable — no app |

- **2026-10-02 device follow-up:** the existing correction in `f49160b`, retained
  in the dirty `task/apple-reminders-import` checkout after the shared-adapter move,
  was development-signed and over-installed as 0.4.5 (405). Launch succeeded.
  Current regression/build and post-launch evidence is recorded in PROGRESS.
- **Remaining:** explicit manual Import Now, permission/revocation and longer-use
  acceptance on the phone. Process survival and absent new reports do not prove
  a manually initiated import or its resulting task content.


## B-096 — Apple Reminders recovery reloads lists without retrying import

- **Severity:** High. **Feature:** F-055. **Source:** PR #4 review, confirmed at
  `f49160b` by an isolated production-coordinator probe on 2026-10-02.
- **Reproduction:** fail reminder fetch or task persistence, then choose Try Again.
  The old action called `loadLists()`, clearing the issue without fetching/saving again.
- **Expected/fix:** shared typed retry dispatches fetch/save issues to `importNow()`;
  a missing list reloads only the list inventory. Both native Apple Settings use it.
- **Platforms:** iOS/macOS Implemented; the original runtime defect was confirmed in
  the iOS coordinator, while macOS first gains the corrected integration in this change.
  Linux/Android Not applicable: no EventKit integration.
- **Evidence:** regression covers fetch retry, save retry and missing-list inventory
  recovery with UUID-isolated preferences and synthetic sources. See the PR #4
  review-fix handoff in [PROGRESS.md](PROGRESS.md) for final checks and limits.

## B-097 — reminder identity upgrades remain only in device-local history

- **Severity:** Medium. **Feature:** F-055. **Source:** PR #4 review, confirmed at
  `f49160b` by a synthetic production-coordinator probe on 2026-10-02.
- **Reproduction:** import a local-only reminder, then refetch it with a server ID and
  its original local alias. The history shortcut skipped the core, leaving synchronized
  task provenance unable to match that server ID on another device.
- **Expected/fix:** reconcile identities into matching live/archive tasks through
  ordinary durable task-update operations; keep task content and undo intact. A history
  hit only reconciles and never recreates a deleted/undone task. Archive restore must
  retain identities learned after the archive snapshot was captured. Metadata changes
  notify native sync without incrementing the count of newly created tasks.
- **Platforms:** iOS/macOS Implemented. Linux consumes compatible provenance updates
  through the shared reducer; its native EventKit UI is Not applicable. Android Planned
  for consuming shared task/sync data; EventKit integration is Not applicable.
- **Evidence:** core/coordinator regressions and consumer checks are recorded in the
  PR #4 review-fix handoff in [PROGRESS.md](PROGRESS.md). Identifier resets, differing
  Exchange IDs and simultaneous independent imports retain the documented limitations
  in [APPLE-REMINDERS.md](APPLE-REMINDERS.md).

## B-098 — Apple Reminders terminal results have no explicit accessibility delivery

- **Severity:** High. **Feature:** F-055. **Source:** PR #4 source review on 2026-10-02.
- **Assessment:** missing explicit result/error delivery is confirmed in the original
  iOS view. The reported vanished-focus/spoken behavior was not reproduced live.
- **Expected/fix:** deliver a terminal accessible result or recovery message after
  each attempt, including repeated identical outcomes. A partial import followed by
  failure delivers the error rather than announcing an incomplete success.
- **Platforms:** iOS/macOS Implemented; macOS first gains this integration in the same
  change. Linux/Android Not applicable: no EventKit integration.
- **Evidence:** coordinator feedback-event and native view checks are recorded in the
  PR #4 review-fix handoff in [PROGRESS.md](PROGRESS.md). Spoken VoiceOver acceptance
  remains separate and must not be inferred from accessible labels or unit tests.

## B-099 — a background sync exchange never re-arms automatic sync (iOS); desktops skip the catch-up when the window returns

- **Severity:** High. **Features:** F-049/F-025. **Affected:** iOS, reproduced in code
  and by a new package test; macOS and Linux assessed from code, not reproduced live.
- **Reproduction (iOS):** with Sync in the background on, let a background wake start
  an exchange and open Momentum before it finishes. Expected: the exchange's result
  shows and automatic sync continues. Actual: the foreground return was remembered
  but the wake's completion scheduled nothing, so no automatic exchange ran again
  until a local edit, a pull-to-refresh or another background/foreground cycle; edits
  made during the wake also missed the 10 s upload. A wake that hit its 25 s budget
  was recorded as a network failure.
- **Cause/fix:** `syncInBackground` ended without the `schedule(after:)` hand-off that
  `run()` has (B-033's fix had only reached the foreground path). It now ends through
  the same scheduling, resets the retry flag at admission, and treats a budget
  timeout like an expired wake: not recorded, the next foreground exchange reports
  its own result. macOS/Linux: with background sync on, a returning window neither
  redrew nor synced; now it always redraws and syncs when the shared core rule
  `window_return_sync_due` says the idle cycle was missed (or background sync is
  off). macOS additionally declines App Nap while hidden with automatic background
  sync configured, so the 150 s timer keeps its cadence.
- **Regression:** iOS `foregroundReturnDuringABackgroundWakeRetriesAtOnceAndReArmsTheCycle`
  and `aWakeThatRunsOutOfBudgetIsNotAFailureAndTheForegroundRetries`; core
  `window_return_tests`; macOS `theWindowsReturnRedrawsAndDecidesCatchUpFromTheSharedRule`.
  Linux wiring compiles; GTK runtime not exercised here. Remaining: a real background
  wake on a device, and the App Nap effect observed only by reasoning.

## 2026-10-06 — F-056 sound verification assessment

No product defect was reproduced in the sound setting/completion paths. Tests prove
that the default is off, the enabled switch is respected, disabling takes effect,
reopen/undo stay silent, and bulk completion produces one sound request. Native
SwiftPM's uncompiled localization resources are a build-backend limitation; the
full Xcode-backend package suite passes. Audible output and the documented iOS
non-app completion scope remain explicitly unverified/limited, not claimed as fixed.

## B-100 — unverifiable Nextcloud uploads can be acknowledged

- **Severity:** High. **Features:** F-054/F-058. **Impact:** reproduced in isolated
  Rust WebDAV regression tests; native occurrence not independently reproduced.
- **Reproduction:** PUT returns success but the verification read returns 404, or an
  existing file's GET omits its ETag. The original code could acknowledge an absent
  copy or overwrite without a precondition. Review also found that trusting a missing
  PUT validator or a different verification validator could cache an unadopted copy.
- **Expected/fix:** require strong overwrite preconditions, actual stored length and
  matching PUT/PROPFIND validators. Keep edits queued on missing/mismatched evidence;
  do not replay a PUT with uncertain effects. Recovery wording no longer promises the
  earlier server copy survived a partial write.
- **Platforms:** Linux/macOS/iOS Implemented through the shared core; Android Planned
  when its native consumer exists. Shared reproduction is not an OS-specific runtime claim.
- **Evidence:** missing size, missing download/upload/verification validators, different
  validator and partial-upload regressions pass. Native gates and limits: F-058 in
  [PROGRESS.md](PROGRESS.md); [benchmark scope](SYNC-PERFORMANCE.md).

## B-101 — failed LibreSync journal writes suppress retry

- **Severity:** High. **Feature:** F-058. **Impact:** reproduced in isolated adapter
  tests by making the journal scratch path unwritable.
- **Reproduction:** receive an operation or publish a snapshot while the journal save
  fails. The original code mutated seen/inbox/dirty flags first, so another delivery
  could be discarded or a failed snapshot treated as published. Local publication also
  swallowed journal errors and serialized the entire backlog once per operation.
- **Expected/fix:** persist a staged journal before publishing memory changes, batch
  pending operations once, skip unchanged journal writes and surface errors with retry.
  Successful writes sync both the file and its parent directory.
- **Platforms:** Linux/macOS/iOS Implemented; shared reproduction confirmed, native
  failure injection not performed. Android Planned for the eventual core consumer.
- **Evidence:** inbound redelivery after failed save, snapshot retry, batch persistence,
  no-op adapter load and reentrant engine error callback pass. Two real Rust LibreSync
  peers converge. Remaining native/physical checks are recorded under F-058.

## B-102 — a same-version server replacement is ignored

- **Severity:** High. **Feature:** F-058. **Impact:** reproduced in the Rust fixture.
- **Reproduction:** replace the remote sync file with different task data/ETag while
  retaining its numeric sync version. The original comparison ignored the replacement.
- **Expected/fix:** adopt changed remote data when either version or ETag changes;
  permit body-free polling only for a successfully validated engine session without
  pending edits and with unchanged connection/options/version/validator.
- **Platforms:** Linux/macOS/iOS Implemented through Rust; Android Planned. No real
  server incident or platform-specific reproduction is claimed.
- **Evidence:** replacement adoption and cache invalidation regressions pass. A paused
  no-change exchange also preserves concurrent edits, vector clocks, undo and disk.
  See [F-058 evidence](SYNC-PERFORMANCE.md) and native limits in PROGRESS.

## B-103 — corrupt gzip lacks damaged-copy recovery

- **Severity:** Medium. **Features:** F-028/F-058. **Impact:** reproduced by decoding
  an invalid compressed fixture, without modifying any user or hosted server data.
- **Reproduction:** a syntactically prefixed compressed sync file has invalid gzip
  bytes. The decoder returned generic I/O failure, hiding damaged-copy replacement.
- **Expected/fix:** classify compressed decoding failure as RemoteFileDamaged,
  preserving the existing explicit replacement/backup workflow and pending edits.
- **Platforms:** Linux/macOS/iOS Implemented through shared classification; Android
  Planned. Native spoken/action acceptance remains unverified.
- **Evidence:** invalid-gzip classification regression passes; replacement and encrypted
  damage regressions remain green. Native gates are recorded in PROGRESS.

## B-105 — generated Kotlin error payloads redeclare Throwable.message

- **Severity:** High. **Features:** F-001/F-017. **Source:** Android source-build
  compiler failure reproduced during native scaffolding on 2026-10-10.
- **Reproduction:** generate Kotlin from the actual `momentum-ffi` library, then
  compile error variants whose Rust payload field is named `message`. UniFFI's
  exception projection also implements `Throwable.message`; generated declarations
  collide and the Android client cannot compile.
- **Expected/fix:** keep Rust errors and wire/Apple compatibility intact. Rename only
  the Kotlin projection's Actionable/Transient/Io/Invalid/Nearby payload field to
  `detail` in [uniffi.toml](../android/uniffi.toml), then regenerate through the build.
  Generated code is not hand-edited.
- **Platforms:** Android Verified for generated Kotlin compilation; compiler defect reproduced. Existing Linux,
  macOS and iOS exception projections are unaffected by the Kotlin-only renaming.
- **Evidence:** the code-generation regression in
  [test_codegen.py](../android/scripts/tests/test_codegen.py) generates actual
  bindings and checks the exception declarations. Current generated-binding/build checks pass as recorded below. See [Android acceptance](ANDROID-PARITY-CHECKLIST.md).

- **Current verification (2026-10-10):** Android Verified for generated Kotlin compilation and error projections. Fresh locked generation, 33 host-tool regressions, 69 JVM tests and gate11d debug/minified-release APK/AAB builds pass. Existing wire/Apple projections remain compatible; generated files remain build outputs. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-106 — a new Android editor can save into the later navigation context

- **Severity:** High. **Feature:** F-006. **Source:** source-confirmed ownership defect
  during Android review on 2026-10-10; pre-fix live misfiling was not separately recorded.
- **Reproduction:** open a new task from Morning/project/tag, navigate to another
  destination while the editor remains open, then Save. Reading the active route at
  Save time applies the later destination instead of the editor's creation source.
- **Expected/fix:** capture and persist the editor's source view/context when it opens;
  pass that source into Rust task creation independently of subsequent navigation.
  Keep drafts on failure and do not duplicate Rust membership rules in Kotlin.
- **Platforms:** Android Verified for the captured-source regression. No equivalent new defect is established in
  Linux/macOS/iOS; their prior source-context evidence remains independent.
- **Evidence:** `sourceContextSurvivesNavigationBeforeSave` in
  [ComposeFlowTest.kt](../android/app/src/androidTest/java/com/codedbydan/momentum/ComposeFlowTest.kt)
  covers Morning → new draft → other project → Save, asserting the original family
  destination/slot. Current API26/API37 native suites pass this regression; broader real-OS process
  recreation and project/tag interaction remain separate acceptance.

- **Current verification (2026-10-10):** Android Verified for the controlled captured-source regression. The source-context regression passes in the current API26/API37 native suites with the unchanged gate11d production application. Broader physical lifecycle/project/tag interaction remains in the Android checklist. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-107 — large Android task notes crash native editor layout

- **Severity:** High. **Feature:** F-006/F-018. **Source:** runtime reproduced on the
  disposable API 36 emulator on 2026-10-10 with approximately 450 KB of Unicode notes.
- **Reproduction:** open/render the long notes draft in the native editor. An
  unbounded text viewport triggered a Compose layout-state/measurement exception.
- **Expected/fix:** bound the notes TextField's viewport while retaining all source
  text and ordinary scrolling/editing. Large draft content belongs in the private
  atomic draft file; SavedState carries a small draft identity. The potential Binder
  saved-state overflow was a source risk, not a separately reproduced crash.
- **Platforms:** Android Verified for bounded native layout/private-draft preservation; runtime failure reproduced. This does not
  establish a corresponding Apple/Linux editor defect.
- **Evidence:** the focused `largePrivateDraftUsesSmallSavedStateAndRestoresWithoutTruncation`
  regression in [ComposeFlowTest.kt](../android/app/src/androidTest/java/com/codedbydan/momentum/ComposeFlowTest.kt)
  passed on API 36 after the bounded viewport fix. Full Unicode preservation, fresh
  final suite, process/configuration recreation, IME and large-text device checks are
  separate acceptance gates; no broad native verification is claimed.

- **Current verification (2026-10-10):** Android Verified for native bounded-layout/private-draft preservation. The original large-private-draft regression and six bounded-source cases pass on API26/36/37. Full draft/VM recreation and one Rust Save preserve undisplayed Unicode. B-128 separately records layout performance and native IME scope; physical-device interaction remains open. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-108 — stale Android notification actions have a validation/mutation race

- **Severity:** High. **Feature:** F-007. **Source:** source-confirmed concurrency
  defect on 2026-10-10; a stale notification corrupting live tasks was not reproduced.
- **Reproduction:** Done/Snooze first validates a notification's source revision,
  then mutates in a separate engine call. Native sync/restore can change the task
  between calls even when Kotlin serializes UI operations, allowing a stale action
  to affect the changed task.
- **Expected/fix:** the additive Rust `perform_notification_action` API validates the
  source, applies the action and durably commits under the same engine owner lock.
  [ReminderScheduler.kt](../android/app/src/main/java/com/codedbydan/momentum/platform/ReminderScheduler.kt)
  uses this atomic entry point. Rejected/stale/invalid/failed actions must preserve
  task, source ledger and Undo state; one accepted action must commit once.
- **Platforms:** Android/shared core Verified for the atomic-action scope. Existing
  GTK/CLI/Apple generated consumers remain compatible. Equivalent existing-platform races have
  not been established by this Android review.
- **Evidence:** [notification_actions.rs](../crates/momentum-core/tests/notification_actions.rs)
  covers accepted/repeated/stale/invalid-source, failure and restore boundaries.
  The implementation pass reported 26 targeted Rust tests passing; complete workspace/generated-consumer/native checks now pass below. Cold
  physical notification action delivery remains a separate OS acceptance gate.

- **Current verification (2026-10-10):** Android/shared core Verified for atomic source validation and mutation. The six atomic notification-action and 21 source-ledger regressions pass in both complete Rust workspace gates (299 passed, one ignored). Fresh generated core, CLI and Apple consumer checks pass; Android native action/restore regressions pass. Actual locked OS invocation remains B-119's separate manual gate. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-109 — LibreSync connection-test wording overstates a read-only guarantee

- **Severity:** Medium. **Feature:** F-043. **Source:** source-confirmed contract and
  documentation discrepancy on 2026-10-10; no new live data-loss claim.
- **Reproduction:** iOS `NearbyLifecycle.testConnection` calls `runtime.syncNow` and
  waits for its attributed outbound cycle; `CoreNearbyRuntime.syncNow` calls Rust
  `p2pSyncNow`. This starts a task-data exchange rather than a read-only health probe.
  Previous blanket “never changes task state” documentation exceeds that source contract.
- **Expected/next action:** scope the read-only guarantee to Nextcloud's WebDAV probe.
  Android keeps LibreSync's explicit Sync Now action. A true bounded/cancellable
  LibreSync health probe needs an approved pinned protocol/core API proposal; do not
  relabel an exchange as a read-only test or duplicate protocol behavior in Kotlin.
- **Platforms:** Android read-only LibreSync probe Blocked on the missing shared
  protocol contract; Nextcloud probe Implemented. iOS source behavior confirmed,
  existing feature status preserved; runtime mutation during a probe is unverified.
  Linux/macOS connection-test behavior was not assessed in this pass.
- **Evidence:** [NearbyLifecycle.swift](../ios/Packages/MomentumMobile/Sources/MomentumMobile/NearbyLifecycle.swift)
  and [CoreNearbyRuntime.swift](../ios/Packages/MomentumMobile/Sources/MomentumMobile/CoreNearbyRuntime.swift).
  Android F-043 remains In progress with this blocked sub-capability visible in
  [Android acceptance](ANDROID-PARITY-CHECKLIST.md).

## B-110 — the older Espresso input injector fails before Android 37 UI actions

- **Severity:** High for Android acceptance tooling. **Features:** F-001/F-020.
  **Source:** runtime reproduced by the baseline API 37 instrumentation suite on
  2026-10-10; this is a test-framework compatibility failure, not evidence that the
  product's task actions fail.
- **Reproduction:** the resolved Espresso 3.5.x input strategy reflects the removed
  `android.hardware.input.InputManager.getInstance` method. Compose tests fail before
  input with `java.lang.NoSuchMethodException: android.hardware.input.InputManager.getInstance []`
  in `InputManagerEventInjectionStrategy`. Baseline log:
  `android/rust-target/api37-native-suite-baseline.log`.
- **Fix:** explicitly pin Espresso 3.7.0 and use Compose's `junit4.v2.createComposeRule`
  in native UI tests. No product input workaround or unsupported OS reflection added.
- **Platforms/status:** Android Verified for this API 36/37 framework-failure scope
  at the gate below; unrelated platform test frameworks are unaffected.
- **Evidence:** API 36 and API 37 arm64/16 KiB full gate each passed 37 native tests,
  zero failures, in 44.574/43.485 seconds. Logs:
  `android/rust-target/api36-native-gate2.log`, `android/rust-target/api37-native-gate2.log`.
  App SHA-256 `f57e78680af734f015e72c02f0666fc8181eebec5c84cbefb9ee0409e5f3ee66`;
  test SHA-256 `70ceaad54872cc5bf512f376e1f81933bb3d5c4a92f4e28c74a6bed535e411c3`.
  This dirty-source gate predates typography/system indexing/recurrence refinements;
  their verification remains independent.

## B-111 — Android editor and restore admission can race suspended work

- **Severity:** High. **Features:** F-006/F-029/F-040. **Source:** source-confirmed
  ownership/admission defects on 2026-10-10, with controlled native regressions after
  the fix. Pre-fix live draft loss or restore corruption was not separately established.
- **Reproduction:** while task detail loading suspends, another Add/share can replace
  its draft. Restore that claims its source/busy state only after a suspension permits
  cancellation, repeated restoration or another action to race the same backup input.
  Unbounded externally supplied search/identifier values can also inflate SavedState;
  a Binder crash from those values was a source risk, not a reproduced failure.
- **Fix:** explicit editor-loading/restore admission, retained ordered private capture
  queue, captured backup input and synchronous busy claim reject competing edits,
  cancellation and reentry; resume pending captures after ownership is released.
  Search is capped at 4,096 characters and invalid/oversized task identifiers never
  enter SavedState. Keep failed drafts and concrete errors.
- **Platforms/status:** Android Verified for the controlled task-open, restore and
  oversized-state regressions below. Existing Linux/macOS/iOS behavior is not assessed
  by these Android tests. Broader lifecycle and actual external-entry acceptance remain
  explicit rather than implied by this status.
- **Evidence:** three [ViewModelConcurrencyTest](../android/app/src/androidTest/java/com/codedbydan/momentum/ViewModelConcurrencyTest.kt)
  cases passed in both 37-test API 36/API 37 gates recorded under B-110, using controlled
  suspended repository reads/restore barriers and 450 KB rejected input. Eight capture
  queue tests and five text-import tests also passed. App/test hashes identify the exact
  pre-typography dirty-source artifact; future editor/queue changes need fresh coverage.
  The later capture-count change introduced the separate drain-ordering regression
  recorded as B-118; this earlier evidence does not verify that new source.

## B-112 — Android change events can release the restore service barrier

- **Severity:** High. **Features:** F-025/F-026/F-029. **Source:** source-confirmed
  admission defect on 2026-10-10; live restore corruption was not reproduced.
- **Reproduction:** an edit or settings operation admitted before restore finishes
  while `pauseForRestore` drains native I/O, then calls `changed`. The former event
  sequence check treated that newer event as permission to resume services, even
  though backup replacement had not returned.
- **Expected/fix:** only the restore owner may release the barrier. `changed` keeps
  transport, reminder and search admission paused; the ViewModel explicitly calls
  `resumeAfterRestore` after import succeeds, fails or is cancelled, including a
  failure while pausing. UI restore admission remains held through that resume.
- **Platforms/status:** Android Verified for controlled restore-barrier ownership; other frontends were not changed or
  established as affected by this Kotlin lifecycle defect.
- **Evidence/limits:** new isolated native regressions in
  [CoordinatorTransportTest.kt](../android/app/src/androidTest/java/com/codedbydan/momentum/platform/CoordinatorTransportTest.kt)
  and [ViewModelConcurrencyTest.kt](../android/app/src/androidTest/java/com/codedbydan/momentum/ViewModelConcurrencyTest.kt)
  model an earlier edit completing during the barrier and successful/failed pause
  ownership. They pass on current API26/API37 artifacts as recorded below; earlier
  pre-fix artifacts are not used as evidence for this source.

- **Current verification (2026-10-10):** Android Verified for controlled restore-barrier ownership. Current CoordinatorTransport/ViewModelConcurrency native cases pass on API26 and API37, including an earlier operation completing inside the barrier and successful/failed pause resumption. Actual document-provider replacement/relaunch is separately recorded under F-029; hosted transport remains open. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-113 — sync setup failures can prevent Android notification opt-out

- **Severity:** High, privacy. **Features:** F-007/F-038. **Source:** source-confirmed
  control-flow defect on 2026-10-10; actual notification removal is verified by the
  disposable-emulator regression below.
- **Reproduction:** after a task-title notification is displayed, revoke Android 37
  LAN access with LibreSync selected, then disable notifications. The preference
  persists, but sync setup throws before reminder reconciliation, so the displayed
  private notification is not promptly removed. Invalid native nearby credentials
  reach the same failure path without changing runtime permission mid-process.
- **Expected/fix:** foreground, preference-change and restore-resume processing
  reconcile local notifications independently of foreground/background sync setup.
  A sync error remains visible while disabling notifications cancels app-owned
  displayed notifications and scheduled alarms.
- **Platforms/status:** Android Verified for controlled native notification removal; corresponding Linux/macOS/iOS control
  flow was not assessed by this Android-specific reproduction.
- **Evidence/limits:** `syncSetupFailureCannotKeepAPostedPrivateReminderAfterOptOut`
  in [CoordinatorTransportTest.kt](../android/app/src/androidTest/java/com/codedbydan/momentum/platform/CoordinatorTransportTest.kt)
  uses an isolated store, deliberately incomplete Keystore credentials and a real
  NotificationManager observation. It is restricted to an explicitly opted-in
  disposable emulator and passes the current native gate below. No production store,
  account, index or background work is used.

- **Current verification (2026-10-10):** Android Verified for the native notification opt-out failure path. The opt-in disposable test passes on API26/API37 with an actual NotificationManager observation and incomplete fixture Keystore credentials: sync failure remains actionable and displayed private reminders are removed after opt-out. Real LAN revocation/system permission recovery remains a separate acceptance scope. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-114 — pre-restore notification actions accept identical restored tasks

- **Severity:** High. **Feature:** F-007/F-029. **Source:** reproduced by a targeted
  shared-core regression on 2026-10-10 in the dirty `task/android-native` worktree.
- **Reproduction:** schedule and accept a reminder, export/import the same backup,
  then perform the old Done/Snooze action. The restored task retains its ID, title
  and reminder time, so the old source identity previously still validated and
  changed the replacement task.
- **Expected/fix:** include the existing durable `p2p_restore_epoch` in private
  notification source revisions. Restoring changes source identity even for equal
  task fields, and old actions fail closed under the engine owner lock. No public
  API or task/store format changed; existing scheduled source identities are
  intentionally invalidated and native reconciliation must reschedule them.
- **Platforms/status:** shared-core Verified for restored-source identity rejection. Android uses the atomic action API;
  Linux/macOS/iOS use the same scheduling source identity where integrated, but
  their platform action routing was not separately reproduced as vulnerable.
  Native restore cancellation reduces exposure without repairing this shared
  identity contract. Fresh generated-consumer checks pass as recorded below.
- **Evidence/limits:** the same-ID/title/time regression in
  [notification_actions.rs](../crates/momentum-core/tests/notification_actions.rs)
  failed before the fix because the old action changed the restored task. It
  checks both Done and Snooze rejection, unchanged task/disk/Undo state, reopen
  identity persistence and acceptance of the newly generated request. Targeted
  post-fix `notification_actions` (6) and `notification_ledger` (21) tests all passed.
  Full workspace/generated/native consumer checks pass; results and their
  runtime limits are recorded in the dated progress handoff.

- **Current verification (2026-10-10):** Shared core Verified for restored-source identity rejection. Both full Rust gates pass 299 tests with one intentional ignore; fresh mo/generated FFI, MomentumKit (164), MomentumMobile (158), unsigned Mac and iOS-simulator builds pass. Android native restore/stale-action cases pass. Apple/Linux action-routing runtime and physical reminder delivery are not inferred from consumer builds. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-115 — Android typography sliders expose a 44dp semantic target

- **Severity/reproduction:** medium accessibility defect, reproduced on API 36 and
  API 37/16 KiB in `TypographySettingsTest.nativeControlsChangeOnlyTheirAreaAndResetBoth`
  during the 2026-10-10 gate3 dirty-source run. The assertion measured 44dp where the
  native control requires a minimum of 48dp. Existing pointer hit-area expansion does
  not establish the measured semantic/layout target.
- **Expected/actual:** size controls retain a minimum 48dp accessible area. The native
  Slider track/thumb retained its smaller semantic bounds inside an outer min-height
  modifier, despite the requested surrounding space.
- **Cause/fix:** the first modifier-order attempt still exposed 44dp in gate4b,
  including 44.19046dp on API 26. Exact Material3 1.4.0 source lays out the native
  range node from its track/thumb sizes; the default thumb is 44dp. The public native
  thumb slot now provides `SliderDefaults.Thumb` with a 48dp minimum and shared
  interaction source. Native range actions remain; the original threshold is unchanged.
- **Platforms/status:** Android Verified for the native 48dp range target.
  Linux/macOS/iOS are not affected by this Compose modifier-order defect.
- **Evidence/limits:** both `android/rust-target/api36-native-gate3.log` and
  `api37-native-gate3.log` record the 44dp failure. The same failed gate also uncovered
  a separate toolbar fixture error: an Application configuration context lacked the
  UI/display association needed by native window services. That fixture now retains
  its host Activity context, changes the actual app locale through LocaleManager
  and supplies real Android Density; fixtures restore the prior app locale and its
  combined-scale title/action/scroll checks remain intact.
  The Activity-context exception stopped in gate4b, while the root toolbar case then
  found ambiguous task/list actions sharing the root View Options label (B-116).
  Current native thumb and unique named-action gates pass below.

- **Current verification (2026-10-10):** Android Verified for the recorded 48dp native range target. Current native typography/toolbar tests pass with the original 48dp threshold on supported API26/API37 scopes. The native thumb slot keeps range semantics and independent scale/reset behavior. Spoken/pointer/physical-device acceptance remains open. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-117 — Android silently discards sync-status persistence failures

- **Severity:** Medium. **Features:** F-025/F-026. **Source:** source-confirmed
  persistence defect on 2026-10-10; the isolated native write-fault regression
  passes below.
- **Reproduction:** retain a prior saved sync error, make
  `android-platform-state.json.new` a directory, then finish a successful exchange
  or report a newer failure. AtomicFile cannot start its write, but the former
  `runCatching` discarded the exception. Live UI shows the current result while
  restart reads the older status without a persistence warning.
- **Expected/fix:** a separate localized status-save warning preserves the actual
  exchange result, typed recovery error and last successful sync. Synchronize the
  latest private desired status with write attempts, retain it on failure, and
  offer an explicit local-only save retry. Do not recursively persist a write
  failure or start another exchange from that retry. The warning/retry remain
  accessible when the selected provider is Off.
- **Platforms/status:** Android Verified for the native atomic-write/retry fault path. Other frontends do not use this Kotlin
  AtomicFile status adapter; their own status persistence was not assessed here.
- **Evidence/limits:** the real damaged-copy recovery test in
  [CoordinatorTransportTest.kt](../android/app/src/androidTest/java/com/codedbydan/momentum/platform/CoordinatorTransportTest.kt)
  now injects an isolated atomic-write obstruction, verifies actual exchange
  success plus retained old bytes, then saves the latest typed error and checks
  restart. It asserts retry preserves the core success timestamp and does not
  repeat server-copy replacement. Current Android build/native gates pass for this scope below.

- **Current verification (2026-10-10):** Android Verified for isolated AtomicFile status-write failure and local retry. The real exchange/recovery native fixture passes on API26/API37, retaining prior bytes on write failure, recording actual exchange success truthfully, and saving the latest typed status through local-only retry without repeating replacement. External-server acceptance remains separate. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-118 — A queued drain can reopen a dismissed share

- **Severity:** High. **Features:** F-006/F-040. **Platforms:** Android reproduced;
  Linux/macOS/iOS do not use this Android capture queue.
- **Reproduction/evidence:** two durable shared drafts and twenty repeated drain
  requests, then dismiss the first draft. On both API 36 and API 37, the gate3
  regression timed out with the first draft still visible although the queue had
  already removed its reference. The focused diagnostic recorded `title=First`,
  `loading=false`, `error=null`, and `queue=1`. Logs:
  `android/rust-target/api36-native-gate3.log`, `api37-native-gate3.log`, and
  `queue-gate3-diagnostic.log`; dirty `task/android-native` based on `200f686c`.
- **Cause:** the newly added queue-count read suspended an older drain before its
  idle check. Dismissal handed completion to a different coroutine, so that older
  drain could claim the old head before completion committed its removal.
- **Fix/status:** Verified for controlled concurrent capture drain/completion. Dismissal records its pending completion synchronously;
  every drain commits that completion under the queue mutex before claiming a head.
  It retains the completion on failure and preserves unreadable queue bytes. Idle
  drains no longer suspend solely to recount a currently presented editor.
- **Acceptance:** current concurrent drain, corrupt-completion recovery, queue
  count and complete native capture checks pass below. This is a
  reproduced regression, not an inference from the earlier passing gate2 suite.


- **Current verification (2026-10-10):** Verified for concurrent capture drain/dismissal and retained recovery. The current complete capture suite passes on API26/API37, including repeated concurrent drains, corrupt completion retention and queue count. Earlier gate3 failures remain historical evidence; actual shipping warm/cold FIFO entry and full sharing have separate recorded probes. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-119 — Android reminder actions can mutate tasks while locked

- **Severity:** High. **Features:** F-007/F-031. **Source:** source-confirmed on
  2026-10-10 in dirty `task/android-native`; real lock-screen invocation is not
  claimed as reproduced. Notification privacy hid text but the broadcast Done
  and Snooze actions did not request authentication and the receiver did not
  check keyguard before core mutation.
- **Expected/fix:** require native action authentication (effective API31+) and
  reject a locked callback on every supported API immediately before mutation.
  Preserve displayed notification and private receipt for retry after unlocking;
  show localized guidance to open Momentum and unlock. Existing Open routing
  remains protected through the Activity. Use the same Rust atomic action API.
- **Platforms/status:** Android Verified for injected callback policy/native flags; actual
  OS keyguard acceptance remains open. iOS
  App Intents already request authentication; the other platforms do not use
  this Android broadcast adapter and were not reproduced here.
- **Evidence/limits:** `ReminderOsTest` now checks actual API31+ notification
  authentication flags and injected locked-policy Done/Snooze rejection, unchanged
  core/receipt/displayed source, and successful unlocked exactly-once retry.
  A deterministic closure tests callback policy, not actual OS keyguard behavior.
  See [native action authentication](https://developer.android.com/reference/android/app/Notification.Action.Builder#setAuthenticationRequired(boolean)).

- **Current verification (2026-10-10):** Android Verified for the injected locked-callback policy and native action flags. Current ReminderOsTest passes on API26/API37: locked Done/Snooze preserve core, receipt and displayed source; unlocked retry commits once; API31+ action authentication flags are present. Injected policy does not prove actual OS keyguard behavior; physical locked-device acceptance remains open. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-116 — Android toolbar locale fixture loses native display association

- **Severity:** Low; verification infrastructure. **Features:** F-020/F-021/F-022.
  Gate3 reproduced an exception in both API 36/API 37 toolbar tests when an
  Application configuration context replaced the native Activity context.
- **Cause/fix:** Android window services require a UI context with display association.
  The fixture now retains its Activity, uses actual per-app LocaleManager changes
  and restores the prior locales. A debug-only test-host configuration declaration
  keeps composition mounted; Android's context-backed font converter remains at
  system scale 2, combined with interface/content scale 2.
- **Status/evidence:** Verified for native Activity-context/toolbar fixture behavior. Gate4b no longer reports the UI-context exception.
  Its root toolbar assertion instead found three controls all named View Options:
  the root toolbar, task menu and hidden list menu. Product task/list actions now
  name their task/list, leaving the root label unique. Logs are
  `android/rust-target/api36-native-gate3.log` and `api37-native-gate4b.log`.
  Current toolbar/navigation native acceptance passes for the recorded scope below.
  Production MainActivity retained its UI context throughout; other platforms do not
  use this Compose test fixture.

- **Current verification (2026-10-10):** Verified for the native Activity-context/toolbar fixture scope. Current API37 toolbar, locale and named-action tests pass, including German combined scaling. B-120 has separate phone/tablet native geometry evidence. No production UI-context replacement was introduced; manual focus/spoken acceptance remains open. See
  the final artifact/test identities in [PROGRESS.md](PROGRESS.md).

## B-120 — German large-text Android bottom navigation overflows the window

- **Severity:** Medium (P2). **Features:** F-020/F-022/F-053. **Affected:** Android
  reproduced; other platforms do not use the Material navigation-suite label layout.
- **Reproduction/evidence:** 2026-10-10 parent live inspection on the owned API 36
  phone emulator, German/dark appearance and system font scale 2, after gate4b.
  `android/rust-target/api36-german-dark-large.png` shows Demnächst split within its
  word and Einstellungen extending beyond the right edge. This is observed native
  behavior on a dirty artifact, not a suspected source-only problem.
- **Expected/fix:** all four native navigation targets and their labels stay inside
  the window. Native label Text now has a single visual line with ellipsis bounded
  to its available item width. Its complete localized Text semantics are retained,
  as are native selectable targets and Android font scaling; no Density override or
  reduced text scale is introduced in product code.
- **Status/limits:** Verified for the recorded native navigation geometry scope.
  On 2026-10-10, live API37 inspection of dirty gate6b APK
  `4513860022158c19f9bbec45d7cd9c53cc4c15f4c08913e6c38b3beaceb069b5`
  used German/dark, OS font scale 2 and native content/interface scales 2.
  Phone and real 800dp tablet-window labels stayed within their native items and
  window; screenshots and full semantic labels were inspected. All settings and
  window overrides were restored. Evidence: `android/rust-target/b114-live-api37/`.
  A new actual
  German system-scale-2/interface-scale-2 navigation regression checks all four tab
  targets, full label semantics, one-line layout, 48dp geometry and window edges.
  This automated phone regression passes on API26 and API37 in gate7b.
  A further test-only 800dp adaptive-window fixture selects the actual Material rail
  at German system/interface scale 2 and checks every full semantic label inside its
  native item and real host-window bounds. It allows healthy native width expansion:
  Material 1.4.0's 80dp rail token is a minimum, and item measurement honors supplied
  parent constraints. No current rail clipping is inferred solely from the previous
  bottom-bar screenshot. The tablet fixture also passes gate7b on API37; its app/test
  hashes are recorded in `android/rust-target/api37-native-gate7b-artifacts.json`.
  Landscape, spoken TalkBack output and physical-device acceptance remain separate.

## B-121 — Large Android clipboard copies exceed Binder and Android 8 URI grants fail

- **Severity:** High for the uncaught copy failure; Medium for the remaining
  Android 8 cross-app clipboard limitation. **Features:** F-006/F-015/F-023.
  **Reproduced:** 2026-10-10, dirty `task/android-native`. An isolated API37 native
  ClipboardManager probe exactly copied smaller ASCII controls but copying
  2,097,152 ASCII characters or 552,960 UTF-16 emoji characters threw
  `RuntimeException` caused by `TransactionTooLargeException`. The editor's full
  notes exceed this limit and previously passed directly into the same Binder call.
  Native draft/core data must remain complete; silent truncation is unacceptable.
- **Fix:** small explicit Copy Notes/Title remains sensitive plain text. Larger
  text uses a private, nonexported read-only FileProvider stream, with 32 files of
  at most 4,000,000 UTF-8 bytes each. I/O occurs off the main thread and failures
  surface localized feedback while preserving notes and the previous clipboard.
  Explicit Import Paste streams only this app's validated URI and rejects input
  beyond 1,000,000 characters without replacing the draft. No arbitrary URI fetch,
  public provider, storage permission, new dependency or task-format change was added.
- **Android 8 limitation:** an independent API26 cross-UID probe reproduced zero
  readable URI content and no read grant. Android 8/8.1 ClipboardService requests
  its grant as system UID, which ActivityManager refuses; Android 9 uses the source
  UID. See the official sources and probe scope in
  [DISTRIBUTION_AUDIT.md](../android/DISTRIBUTION_AUDIT.md). Large Copy Notes/Title
  now rejects before changing the prior clip or creating a payload on API26/27.
  Full-title sharing is not implemented. **Android cross-app large clipboard on
  API26/27 remains Blocked by the platform grant behavior.**
- **Separate native alternative:** the editor's user-invoked **Share full notes**
  opens the standard Sharesheet with the complete private text stream and a
  temporary read-only grant to the selected recipient. The separate sharing cache
  retains existing files through clipboard replacement and process recreation;
  capacity failure explains how to clear only app cache. No automatic share or
  all-app grant is made. Cache eviction and user clearing can end stream availability;
  chooser cancellation does not imply an expired recipient grant.
- **Verification/status:** Android Verified for scoped safe-copy/preflight and
  explicit notes-sharing regressions, not unrestricted large clipboard support.
  Gate8b API26 passes the preserved-old-clip preflight and all three notes-share
  cases; Gate9 API36 passes all three notes-share cases including native chooser
  touch. Direct separate-UID full-text/read-only sharing also passes API37.
  The actual shipping TaskEditor Sharesheet on API36 delivers all 1,105,920 UTF-8
  bytes exactly to a separate UID. After SIGKILL of the sender, the same recipient
  URI reread restarts the shipping FileProvider and preserves the complete text,
  read grant and denied write access. Evidence is in ignored
  `android/rust-target/clipboard-probe/uri/api36-shipping-share-process-death.log`.
  API28+ helper regressions and the independent API37 URI clipboard probe preserve
  complete text/read-only grants and revoke recipient access after replacement.
- **Platforms/remaining:** Linux/macOS/iOS do not use this Android Binder adapter;
  their clipboard behavior is unaffected by these Kotlin changes and was not
  retested here. Minimum-SDK/native regressions are refreshed in the next shared
  gate; arbitrary recipient apps, API27 runtime, indefinite cache retention and
  very-large editor performance are not established by the scoped proofs above.

## B-122 — AndroidX Graphics Path JNI has a partial 16 KiB RELRO page

- **Severity:** High; release compatibility. **Affected:** Android dependency;
  other platform frontends do not load this JNI. **Features:** F-020/F-022/F-053.
- **Evidence:** 2026-10-10 dirty `task/android-native` gate4b APK contains
  `androidx.graphics:graphics-path:1.0.1` with 16 KiB LOAD alignment but GNU_RELRO
  endpoints `0x6000` (arm64) and `0x7000` (x86_64). The official 1.1.0 AAR still
  ends RELRO at `0x6000` on both ABIs. These fail the strict alignment checker;
  a task-app runtime crash is not claimed as reproduced.
- **Cause/fix:** exact upstream 1.1.0 CMake supplies max-page-size only. NDK r27
  requires common-page-size as well. Fourteen unchanged Apache-licensed release
  source/build/license files are pinned locally; a dedicated source build uses
  both flags and preserves upstream exports/API. Gradle replaces only JNI entries
  of the hash-verified 1.1.0 AAR. Classes/resources/licenses remain byte-identical;
  no binary patch or weakened ELF/ZIP check is used.
- **Status/verification:** Verified for the scoped source/alignment/native checks; local source-build and packaging checks
  pass. Five focused Python regressions pass (32 total script tests at
  gate10b). Both rebuilt ABIs have LOAD `0x4000`, RELRO endpoint `0xc000`
  and only JNI_OnLoad exported. Separate output-directory builds reproduced
  byte-for-byte on this macOS host; all original non-JNI AAR bytes were compared.
  Source commit `1c44f0be720ff16cd30357ad2375239bf97a1da8` matches all five
  published Kotlin source files. See [provenance](../android/vendor/GRAPHICS-PATH.md).
- **Native/package evidence:** gate7b iterator/Canvas regressions pass on API26 and
  API37 with 16 KiB pages. Gate10b unsigned release APK and bundle-generated universal
  APK pass strict LOAD/RELRO and ZIP checks for all eight libraries; bundle configuration
  declares `PAGE_ALIGNMENT_16K`. Both APKs and the AAB retain the source provenance
  and packaged notices. Exact artifacts are recorded in PROGRESS.
- **Latest native reassessment:** API26 gate10c passes both regressions; API37's
  full gate10c conic-render comparison reports 182 differing edge pixels against
  its 64-pixel coverage limit. Controlled native inspection finds identical 66
  converted segments before/after size calculation and radii between 23.9999896
  and 24.0000195 for the expected 24-unit circle. Android's `Paint()` default enables
  antialiasing from API31. Explicit antialiasing yields 182 edge differences; explicit
  non-antialiased coverage yields nine. The fixture now explicitly selects the
  coverage mode and independently checks the actual curve geometry within 0.01;
  the original 64-pixel limit is retained. No JNI or product rendering fix is claimed.
  See [Android's constructor contract](https://developer.android.com/reference/android/graphics/Paint#Paint()).
- **Current acceptance:** gate11h passes both native iterator/Canvas cases in each
  complete API26/API37 suite, retaining the 64-pixel bound and independent geometry
  checks. Both current release APKs pass strict checks for all eight libraries; the
  minified bundle product runs on actual 16 KiB API37 pages. Exact hashes and
  minimum-runtime checks are in PROGRESS.
- **Remaining:** independent Linux/F-Droid reproducibility and broader physical
  acceptance. The local byte comparison does not establish a second-host build.


## B-123 — Warm Android background projections miss recurrence at day rollover

- **Severity:** Medium. **Features:** F-007/F-013/F-032. **Source:** source-confirmed
  on 2026-10-10 in dirty `task/android-native`. A warm process generating reminders
  or task counts from a worker/date event read the core without the day processing
  performed by the visible task snapshot. Newly due repeat instances therefore
  depended on opening/refreshing the UI; no fake-clock OS reproduction is claimed.
- **Expected/fix:** all native projections use one repository helper that checks
  live admission after queueing, invokes Rust `dayChanged` then `spawnRepeats` under
  the same local mutex, and reads the projection only while still admitted. The
  coordinator also prepares the day before combined/date refresh and emits a
  revision when recurrence creates tasks. Restore does not consume the day signal
  while paused and resumes only through its existing explicit owner barrier.
- **Platforms/status:** Android Verified for the serialized native projection
  regression scope on API26/API37, 2026-10-10 dirty gate7b. All three
  `CoordinatorLocalDayTest` cases pass on both runtimes; logs and exact app/test
  hashes are in `android/rust-target/api{26,37}-native-gate7b*`.
  Shared recurrence rules and data format are unchanged; other native clients do
  not use this Kotlin projection path and were not reproduced here.
- **Regression scope:** `CoordinatorLocalDayTest` uses an isolated missed-repeat
  backup and injected day observation, with real Rust creation, count and summary
  outputs. It checks one actual instance, deduplication, explicit restore resume,
  and admission revoked while waiting for the local queue. It neither changes a
  global clock nor claims actual OS background scheduling acceptance.

## B-124 — Count notification cancellation cancels its coroutine owner

- **Severity:** High. **Features:** F-007/F-025/F-026/F-032. **Reproduced:** dirty
  `task/android-native`, parent native gate5b on API36/API37 and gate5c on API26.
  Default Off/zero count reconciliation and restore drain threw
  `JobCancellationException`; restore could return before cancelling/draining the
  active native HTTP exchange. `TaskCountOsTest` and the existing coordinator
  restore regressions exposed the failure.
- **Cause:** inside `withContext`, unqualified `cancel()` resolved to the implicit
  `CoroutineScope.cancel` extension instead of the enclosing private notification
  helper. Read-only `javap` of gate5 debug bytecode confirms
  `CoroutineScopeKt.cancel$default` in `cancelAndDrain` and reconciliation branches.
- **Fix/status:** Android Verified for cancellation-owner and native transport
  regression scope on API26/API37, 2026-10-10 dirty gate7b. The private helper and every invocation are
  named `cancelPostedNotification`; explicit `scope.cancel()` remains only in close.
  `ReminderScheduler.cancel(NotificationRequest)` has a distinct required argument
  and was not affected. Other platforms do not use this adapter.
- **Acceptance:** retain the real notification privacy/zero/Off/channel tests and
  add explicit calling-job-active assertions after cancellation and restore drain.
  Gate7b passes the caller-job, zero/Off, real notification and native
  exchange/restore cancellation cases on API26/API37. API26's channel-block fixture
  required a fresh initially blocked channel because Android 8 ignores an app's
  attempted importance update to its existing channel; the corrected full channel
  fixture still needs the next minimum-SDK gate. Gate8b passes all three real
  framework count tests on API37. Prior gate5 is a failure, not evidence that the
  corrected sources pass. Historical logs: `android/rust-target/` files
  `api36-native-gate5b.log`, `api37-native-gate5b.log`, `api26-native-gate5c.log`.

## B-125 — A stale Android backup read replaces the latest selection

- **Severity:** Medium. **Feature:** F-029. **Source-confirmed reproduction:**
  dirty `task/android-native`, 2026-10-10. Start a slow document-provider read A,
  select backup B while A remains blocked, and allow B then A to finish. Without
  request ownership, A republishes itself and deletes B's prepared private file.
  Cancelling preparation also leaves a late A completion able to reopen replacement
  confirmation. No production document or task store was used for this assessment.
- **Fix/status:** Android Verified for the controlled private-provider regression
  scope on API26/API37, 2026-10-10 dirty gate7b. Both backup preparation cases in
  `NativeRecoveryTest` pass with real Rust replacement and cancellation. Preparation
  has a latest-request generation and cancellable job; each read owns a unique
  temporary file and removes only its unpublished file in `finally`. A stale or
  cancelled completion cannot replace the selected bytes or publish an error over
  a newer request. The localized reading/Cancel state is visible before confirmation.
  A blocking third-party stream may finish its current read before cleanup; this
  does not claim immediate cancellation of arbitrary provider I/O.
- **Regression scope:** `NativeRecoveryTest` uses private isolated backups generated
  by the real Rust engine and a controlled blocking InputStream. It checks A/B
  ordering with actual replacement contents, and Cancel with unchanged local tasks
  and no late confirmation. The minified gate7b bundle universal APK also passes
  actual API37 Downloads SAF export, explicit replacement of a later synthetic
  control task and cold persistence; full Unicode notes are retained. Evidence:
  `android/rust-target/api37-release-saf-result.json` and corresponding native XML.
  Third-party/cloud providers remain unverified. Core data and restore
  semantics are unchanged; Apple uses a separate coordinated immutable-input path.

## B-126 — An unreadable private draft loses its Android Retry identity

- **Severity:** Medium. **Feature:** F-006. **Source-confirmed reproduction:**
  dirty `task/android-native`, 2026-10-10. Reconstruct the VM with a normal private
  draft UUID whose file read fails. Startup retained the bytes but cleared the sole
  SavedState identity; the generic Retry refreshed tasks and could not recover the
  retained draft. New editor creation could then displace its recovery reference.
- **Fix/status:** Android Verified for the private-file/SavedState/UI recovery
  regression scope on API26/API37, 2026-10-10 dirty gate7b. All four private-draft
  `NativeRecoveryTest` cases and `NativeDraftRecoveryUiTest` pass on both runtimes.
  The exact app/test hashes and logs are in `android/rust-target/api{26,37}-native-gate7b*`.
  A separate
  SavedState-only list retains up to 32 UUID/context references, without task text
  or a storage format change. Retry reads the original private file off the main
  thread, validates capture ownership/order, and claims an idle editor before
  presenting it. New editors use fresh UUIDs; active editors and failed reads retain
  their bytes. When the list is full, an additional active failure keeps its original
  SavedState pointer and new editors wait rather than overwrite a reference.
- **Native recovery:** localized generic status and a minimum-48dp Retry action are
  present on the root, task editor and text-import editor. Retry is disabled while
  an editor or operation owns presentation; recovery checks the existing unlock
  policy again before publication. No private draft text appears in recovery status.
- **Regression scope:** four `NativeRecoveryTest` cases use real private draft files
  for active-editor protection, failed retry/reconstruction, two retained failures,
  and the full-list extra reference. `NativeDraftRecoveryUiTest` checks the actual
  Compose button policy and successful recovery. Tests use disposable directories;
  actual OS lock/third-party provider acceptance remains separate. Other platforms
  do not use this Android SavedState path; shared core/data formats are unchanged.

## B-127 — Imported long names bypass Quick Add bounds and bury native actions

- **Severity:** Medium. **Features:** F-004/F-005/F-006/F-017/F-048. **Affected:**
  Android native presentation. The shared core deliberately preserves imported
  names; the other frontends do not use these Compose controls and are not claimed
  as reproduced here. Source inspection and disposable native fixtures on
  2026-10-10 use dirty `task/android-native`, without changing core naming rules.
- **Reproduction:** an imported long tag can expand a Quick Add completion chip
  and insert a completed name beyond the normal entry limit. Rejected completion
  feedback is then lost on Android 8 when its input system sends an unchanged-text
  caret/composition update. Gate9 API26 native inspection reproduced the default
  hint replacing the error while the rejected input itself stayed unchanged.
  Unbounded project/tag/task names also expand list labels, action-sheet headings,
  destination chips and editor organization/subtask controls; multiline name fields
  can displace Color, Save and destructive-scope information.
- **Fix/status:** Android Verified for the scoped controlled native regressions.
  Tag completion uses the same UTF-8 entry admission as normal editing, preserves
  the text/caret on rejection and keeps feedback through selection-only updates.
  Feedback clears after a real accepted text edit. Labels retain complete data and
  semantics while native line limits, viewport bounds, single-line fields and
  scrollable dialogs keep actions reachable. Full names remain in Rust, exports,
  drafts and accessibility text; no import limit or migration was introduced.
- **Evidence:** all three `QuickAddBoundaryTest` cases pass on API26 gate10b,
  including actual DPAD caret movement, persistent rejection feedback, unchanged
  input/core tags and subsequent edit recovery. Gate9 API36 covered bounded/full
  tag semantics and editor reachability. New real-Rust export/import fixtures
  cover drawer editing/deletion scope, project movement, Search/grouping, bounded
  destination chips and task-editor project/subtask controls. All five imported-name and two editor-name fixtures pass in current
  API26/API37 complete suites. API37 also checks German/system-font-2/app-font-2
  labels: full 8,192-unit semantics and the same bounded 48dp chip height as its
  short reference, within the existing 1dp tolerance. Core trimming is respected.
- **Remaining:** broader device geometry and physical TalkBack/third-party
  IME/pointer checks. Current full suites and artifacts are recorded in PROGRESS.

## B-128 — Very large emoji notes block the Android editor's first frame

- **Severity:** High; native responsiveness. **Features:** F-006/F-018/F-040.
  **Affected:** Android; no corresponding Apple/Linux failure is claimed.
- **Reproduction/evidence:** controlled API36 gate9 shipping MainActivity with a
  disposable private draft containing 552,960 UTF-16 units / 1,105,920 UTF-8 bytes
  of emoji. The first editor frame appears by 30.363 seconds cold / 26.725 seconds
  with the same warm process, using 31.41 / 26.29 seconds of per-process CPU.
  The warm Activity launch itself times out after 10 seconds, then eventually
  presents the complete note. Warm ordinary short text takes 1.405 seconds / 1.08
  CPU seconds; 600,000-unit multiline text takes 2.181 / 2.29 seconds. Host load
  affects wall time; the per-process CPU measurements independently reproduce the
  expensive path. These are observed upper bounds, not benchmark guarantees.
- **State safety:** each run preserves the prior empty capture index, private
  draft listing and core/settings hashes byte-for-byte. No task was saved or
  exchanged. Only fixture metadata/screenshots/timings are retained, without the
  full note text. Evidence is under `android/rust-target/notes-performance-api36`.
- **Cause:** the sampled warm run spends 24.950 exclusive CPU seconds in
  `LineBreaker.nComputeLineBreaks`, about 96% of its 26.022-second main-thread CPU
  total. The Compose 1.12.1/Material 3 source already resolves this field to simple
  line breaking and no hyphenation; setting those values again does not avoid
  laying out the complete paragraph. Bounded height alone does not bound text layout.
- **Status:** Android Verified for the scoped native/performance regression.
  Both Notes and text-import source now use native
  section editing above 32,768 UTF-16 units, with bounded 4,096-unit fields and
  localized position/navigation controls. The existing full private draft remains
  authoritative; Copy, explicit sharing, Save and Rust parsing still use all text.
  Android ICU boundaries preserve ordinary graphemes and CRLF; a pathological
  oversized grapheme uses an explicit lossless code-point continuation. Section
  selection applies only to the visible text, as the UI explains. No data limit,
  storage format, core behavior or third-party dependency changed.
- **IME boundary:** active composition retains the native proposed text/selection
  unchanged within a fixed temporary budget; committed sections rewindow to 4,096
  units. Rewriting active composition clears Compose's InputConnection composition
  and reproduced an appended second composing replacement in gate11a. Whole-edit
  rejection preserves the source and caret when the budget is exceeded. EOF
  deletion, CRLF, emoji and oversized joined-grapheme progress have regressions.
- **Current evidence:** all 14 JVM source-section tests pass in gate11h's 91-test lane.
  All seven bounded-source and six actual Android InputConnection/native Undo
  cases pass on API26/API37, with the earlier six-plus-six also passing on API36. The full source persists through the private draft and one Rust
  Save. Exact artifact identities are recorded in [PROGRESS.md](PROGRESS.md).
- **Repeated performance:** the same API36 shipping Activity/debug-AVD fixture now
  appears by 3.811 seconds cold / 2.101 seconds warm, using 5.49 / 1.56 per-process
  CPU seconds. Notes is reachable with one swipe and lays out 4,096 units while
  retaining all 552,960 source units. All six short/multiline/emoji cold/warm runs
  preserve the original queue, private-draft listing and core/settings bytes.
  The fresh gate11h run has host load 16.4–22.5 with other emulator tests active: these are observed upper
  bounds demonstrating the removed layout stall, not physical-device or minified
  release timing guarantees. Raw evidence and before/after values are in
  `android/rust-target/notes-performance-api36/gate11h-summary.json` and
  `gate11h-before-after.json`. Complete private bytes were also compared; only
  the derived disabled-search receipt's `updated-at` changed, while every other
  private file remained identical.
- **Current full acceptance:** API26 has 121 passes/six expected skips; API37 has
  134 passes/three expected skips, zero failures.
- **Remaining:** physical keyboard/third-party IME acceptance is separate from supported native
  input interception; no installed-keyboard or spoken-accessibility claim is made.

## B-129 — Native text Undo can replace a different large-note section

- **Severity:** High; editor data integrity. **Features:** F-006/F-015/F-023.
  **Affected:** Android's new large-source presentation; no corresponding
  Apple/Linux defect is claimed. Reproduced on a disposable API37 fixture in the
  dirty `task/android-native` worktree on 2026-10-10 local time.
- **Reproduction:** replace one character in the first 4,096-unit section of a
  synthetic note, move to its distinct second section, focus Notes and send actual
  native Ctrl+Z key events through the Dialog window. The complete VM/private draft
  no longer matches the edited source because the remembered legacy field Undo
  history restores a previous section's text into the current source range.
- **Evidence:** gate11b's two actual Android InputConnection composition tests
  pass; its native cross-section Undo regression fails at complete-source equality.
  App SHA-256 `2d3136004870e763662f287a6747f25b80b84d63c0631f4d034a493328a33246`;
  test SHA-256 `808da1b894c5a0e337c9102ca0afe9efcf9889deeeca76503376bbb64ec812e6`.
  Log: `android/rust-target/api37-gate11b-ime-undo.log`. Initial edit and distinct
  second-section assertions pass before the Z event; this is native reproduction,
  rather than an inference from the remembered UndoManager source.
- **Fix/status:** Android Verified for the scoped controlled source/IME/Undo regressions. A public Compose field key separates native
  history when navigation, external source replacement or committed rewindowing
  changes the field's source mapping. Ordinary same-section typing/composition
  keeps its identity; focus and caret restoration use supported native APIs.
  No shared task rule, storage format, core Undo behavior or private framework API
  changes. The large-text hint explains this section-specific text Undo scope.
- **Lifecycle follow-up:** gate11c fixed cross-section Undo but exposed a disposal
  callback: the retired field's `onBlur` sent its old processor text into the new
  section, clearing focus or overwriting the latest complete draft. The field
  captures its immutable history scope and rejects retired callbacks before source
  preparation or admission. All 12 focused source/IME/Undo cases pass on API26,
  API36 and API37 in gate11d, including native focus, final Rust Save, same-section
  Undo, short final sections and composition-triggered rewindowing.
- **Current verification:** all six native InputConnection/Undo cases and seven
  bounded-source cases pass on API26/API37 in gate11h; complete private source,
  focus/caret and final Rust Save checks remain intact. Both complete native suites
  and the 91-test JVM/build lane pass. Ordinary same-section Undo restores the
  edited section and leaves every other section unchanged.
- **Remaining:** physical keyboard/IME acceptance remains separate from controlled
  Android key/input connections. Existing-format capacity gaps are B-133.

## B-130 — Android scale fixture expects a count on an uncounted sidebar destination

- **Severity:** Low; verification fixture. **Feature:** F-044. **Affected:** Android
  instrumentation only; no production count or task-loss defect was established.
- **Reproduction/evidence:** gate11d API26 full suite runs 123 cases with one
  failure at `TaskScaleTest.kt:58`: fixed Today sidebar count is zero while the
  strengthened fixture expects 1,000. Complete listing row/title/identity/section
  checks pass before this assertion. The exact installed application/test hashes
  are in the gate11d handoff. Log: `android/rust-target/api26-native-gate11d-full.log`.
- **Cause:** shared `listing::sidebar` intentionally assigns zero to fixed navigation
  destinations; project/tag entries carry canonical unfinished-family counts.
  The fixture assumed a capability the fixed-sidebar contract does not expose.
- **Fix/status:** Verified for this fixture scope on API26 and API37. Assert the Inbox project's
  exact 1,000-family sidebar count and independent Rust `task_count(DueToday)`
  count, both initially and after reopening; retain complete listing and persisted
  last-task editing assertions. No production source, rule, preference or contract
  changed. Both corrected functional/opt-in scale cases pass on API26 (18.739 s)
  and API37 (23.083 s), using application SHA-256 `695e973cba1c1132bdff110001e80f28bc9ac6a6924079695d0cbcd24a9aac63`
  and test SHA-256 `ad5fae1c3bd03d3da8ef7d4d822c0b5eb819ed08c1b3800df40dd6586a7d82fc`.
  Both gate11h complete native suites have zero failures, including the
  functional 1,000-task case; the opt-in timing samples above remain separate. Exact results and
  measurement limits are recorded in [PROGRESS.md](PROGRESS.md).

## B-131 — Native Sharesheet fixture taps a clipped recipient label

- **Severity:** Low; verification fixture. **Features:** F-006/F-020.
  **Affected:** Android instrumentation; no provider/full-text transfer failure was
  established. Direct separate-UID sharing passes in the same failing full runs.
- **Reproduction/evidence:** API37 gate11d/gate11e full suites time out waiting for
  the native recipient after injecting a touch at its accessible label. Passive
  native-component observations remain on `ChooserActivityLauncher` for 30 seconds.
  Live four/ten-second screenshots show a collapsed native sheet with the owned
  tile's second label line clipped near the navigation bar. Accessibility visibility
  of that text did not prove a reachable touch target. Evidence:
  `android/rust-target/api37-gate11e-chooser-watch.log` and
  `api37-gate11e-chooser-live-10s.png`.
- **Fix/status:** Verified for the native Sharesheet fixture on API26/API37; changed in the test only. Expand the modern native sheet even
  when the exact label exists, refetch after native idle, require the exact label's
  enabled/visible clickable tile wholly inside the resolver window, and send a
  real touch to its center. Original receipt/visibility deadlines and complete
  Unicode, separate UID, read permission and no-write assertions remain intact.
  API26 preserves its existing full-screen selection path. All three sharing cases pass in gate11h focused/full suites on both runtimes,
  including the real chooser touch. Production sharing code is unchanged. Arbitrary
  recipient/provider and physical-device acceptance remain separate.


## B-132 — Large-source fixture checks visibility before the native dialog settles

- **Severity:** Low; verification fixture. **Features:** F-006/F-040. **Affected:**
  Android instrumentation; no source/save defect was established by this failure.
- **Evidence:** API37 gate11e's full suite fails the huge-emoji fixture at its
  initial `performScrollTo().assertIsDisplayed()`, before editing or Save. The same
  six bounded-source cases pass on an isolated replay, and all source/IME cases
  pass in gate11d. Log: `android/rust-target/api37-native-gate11e-full.log`.
- **Assessment:** the fixture awaited ViewModel loading, then immediately opened
  a native Dialog, supplied the source and checked visibility. Android window/IME
  and bring-into-view work can run outside Compose's test clock; `isDisplayed`
  additionally checks the host's actual visible screen rectangle. A readiness race
  is suspected, but the exact failed geometry was not captured in that old run.
- **Fix/status:** Verified for the controlled native visibility fixture; changed in the test only. Await the placed field and its
  actual public visibility, re-scrolling if later native window work invalidates
  the first scroll. The original ten-second wall deadline, complete VM/private
  source and Rust Save equality checks remain unchanged. Failure diagnostics
  contain only bounds, placement and focus; no task/source text or private
  framework APIs. All seven bounded-source cases and both complete API26/API37 suites pass in
  gate11h. The earlier failed geometry remains unproven; no product workaround or
  widened timeout was introduced.


## B-133 — Android accepts edits that cannot fit its private draft format

- **Severity:** High; draft durability. **Features:** F-006/F-015/F-040.
  **Affected:** Android presentation/private drafts; shared task rules and storage
  have no new text limit. Other native clients were not reproduced here.
- **Reproduction:** an already durable 4,000,000-byte ASCII Notes field accepts
  one additional byte into the ViewModel. The delayed private write then rejects
  it at the existing codec limit, leaving older disk text for process recovery.
  API37 gate11g's native RED fails immediately at unchanged-source equality after
  that edit (one test, one failure, 0.193 seconds). Baseline persistence completed
  first. App `695e973cba1c1132bdff110001e80f28bc9ac6a6924079695d0cbcd24a9aac63`,
  test `e3ad282fdd5d2b32f908bd6dc71fbd85b1bf86efa5819fc8f6749081b80fcafa`;
  `android/rust-target/api37-gate11g-draft-admission-red.log`.
- **Cause:** `setEditor` published before checking deterministic private codec
  admission. The existing v2 format allows eight 4,000,000-byte UTF-8 fields,
  10,000 writeUTF tags and a 4,500,000-byte total record. A valid boundary v1 record
  can exceed v2 by its additional marker byte; startup/recovery ownership also
  needed admission before editable publication or reference removal.
- **Fix/status:** Verified for the existing-format admission/retention regressions. Exact allocation-free
  UTF-8/modified-UTF-8 byte accounting runs before changing text, context, UUID,
  recovery ownership or the scheduled write. Only the last admitted field/tag
  identities and bounded counts are cached; rejected input leaves them intact.
  Whole-edit refusal keeps the previous native text/caret and shows EN/DE feedback.
  The writer retains defensive validation. No codec version, cap, task rule,
  dependency or migration changed.
- **Coverage:** ten new JVM budget regressions and gate11h's complete 91-test JVM lane
  pass. New native cases check exact VM/private-file/identity/reconstruction
  preservation, real native input refusal followed by a durable deletion, an
  oversized existing core task left untouched, and exact old disk/inline record
  references retained through Retry/new capture. Current artifact identities and
  actual native results are recorded in [PROGRESS.md](PROGRESS.md). All three
  admission cases and the new native whole-edit/caret/deletion case pass on both
  API26/API37; full suites also pass. Gate11g's API26 memory gap is B-134.
- **Existing-format limitation:** opening an imported task beyond these private
  draft bounds now refuses explicitly and preserves its complete core contents.
  An oversized legacy draft keeps its original recovery reference/bytes. Editing
  those records needs a separately approved private-draft format design; no
  truncation or new shared-core limit is introduced to hide this gap.

## B-134 — Near-limit private draft reads duplicate large buffers on Android 8

- **Severity:** High; private-draft recovery. **Features:** F-006/F-015/F-040.
  **Affected:** Android private draft I/O; reproduced on the controlled API26
  emulator with a 48 MiB application heap. No shared-core or Apple/Linux defect
  is claimed.
- **Evidence:** gate11g's 19 focused API26 cases have 17 passes and two failures.
  The exact-boundary legacy recovery case throws `OutOfMemoryError` while allocating
  a 6,000,016-byte String in `DraftStore.read`, with about 4.81 MiB free. The native
  durable-deletion case subsequently reports an unreadable draft; the same memory
  cause is suspected, but that failure did not independently capture an allocation
  stack. API37 passes the identical 19 cases and its complete 137-case suite.
  Exact installed artifacts and the original failures are retained in
  `android/rust-target/api26-native-gate11g-final-focused.log` and its artifact
  record. This is a real minimum-runtime recovery gap, rather than a passed check.
- **Cause:** disk reads retain a growing encoded byte buffer, its String, a full
  decoded binary record and individual decoded fields. Writes additionally encode
  and decode the whole record for validation. The native fixtures also held large
  redundant expected/source/file buffers; those copies amplified the production
  reader's allocations.
- **Fix/status:** Verified for the controlled near-limit recovery regressions. Read and write the existing Base64/binary v1/v2
  format through bounded streams, validate before opening an atomic write, and
  preserve AtomicFile finalization. Remove redundant fixture snapshots while
  retaining exact source, file size/SHA-256, identity and recovery assertions.
  No format version, cap, shared task rule, dependency or migration changes.
- **Coverage:** twelve streaming regressions pass in the standalone 26-test lane,
  including independent legacy/canonical bytes, maximum v1/v2 and inline records,
  partial reads, trailing raw/decoded bytes, malformed fields/tags/markers and
  caller-owned stream lifetimes. VM errors propagate rather than becoming corruption.
- **Current native/build verification:** gate11h passes all 19 focused cases on
  API26 (102.437 seconds) and API37 (43.054 seconds), preserving exact full-source,
  file length/SHA-256, native caret, identity, deletion/reconstruction and original
  legacy reference checks. Both complete native suites and all 91 JVM tests pass.
  Source review finds no further material codec blocker; per-field byte/String
  allocations and the existing legacy SavedState String remain.
- **Remaining:** broader physical/low-memory/process-failure acceptance, distinct
  from these controlled 48 MiB API26 fixtures. A VM memory error propagates rather
  than being silently classified as a corrupt saved draft.

## B-135 — Shared link title decoding panics on malformed percent escapes with Unicode

- **Severity:** High; shared-core input crash. **Feature:** F-057.
  **Affected:** the new shared task draft/title parser used by iOS share capture.
  The defect was reproduced in Rust tests, not a physical Apple share-sheet crash.
  Android's existing capture adapter does not yet call this newer helper; generated
  bindings expose the corrected core API for its future F-057 adaptation.
- **Reproduction/cause:** a URL path such as `%🧪` or `%A漢` causes the percent decoder
  to slice a Rust String at byte positions inside a UTF-8 character. Both new
  regression tests fail with a character-boundary panic before the correction;
  `android/rust-target/integration-shared-url-red.log` records the two failures.
- **Fix/status:** Verified for shared parsing and consumer compilation. Decode
  hexadecimal escape digits from ASCII bytes; preserve invalid escapes literally
  and keep the existing lossy handling of invalid decoded UTF-8. No task/store
  format or FFI signature changes.
- **Evidence:** two behavioral tests cover malformed, incomplete, mixed Unicode,
  valid UTF-8, upper/lowercase hexadecimal and invalid decoded bytes, retaining the
  original notes/link. The integrated all-feature/all-target Rust lane passes
  323 tests with five pre-existing ignored cases. Regenerated Apple core/bindings
  and 170 MomentumKit plus 161 MomentumMobile tests pass on 2026-10-11 in the dirty
  Android integration worktree. Full app build evidence is recorded in PROGRESS.
- **Remaining:** physical Apple share-sheet acceptance remains a separate gate.

## B-136 — Explicit Android draft save leaves a delayed atomic writer active

- **Severity:** High; private draft publication/recovery ordering.
  **Features:** F-006/F-015/F-040. **Affected:** Android presentation/private drafts.
  No shared task/store format, Apple or Linux defect is claimed.
- **Reproduction:** the first integrated API26 full suite fails the exact private
  file hash assertion in `NativeDraftAdmissionTest` and a standalone private read
  in `BoundedSourceImeTest`. API37 passes. Repeating the two API26 cases on the same
  installed APK reproduces the admission failure. A read-only publication watcher
  sees a 5,311,302-byte base file reset to 4,796 bytes while a complete 5,333,448-byte
  backup remains; the pending debounced writer is still publishing after an explicit
  flush returned. Source/identity/refused-edit assertions pass; the hash failure
  alone does not show that rejected input was saved.
- **Cause:** `persistDraft` serialized one write but did not cancel/drain the
  scheduled autosave. A subsequent independent reader could race Android 8
  AtomicFile's base-to-backup rename/replacement. The
  [AtomicFile contract](https://developer.android.com/reference/android/util/AtomicFile)
  requires caller mutual exclusion.
- **Fix/status:** Verified for the controlled persistence and recovery scope. Explicit
  persistence cancels and joins the pending autosave before entering the draft lock,
  then writes the current accepted draft. The delayed job uses a private writer to
  avoid self-joining; cancellation propagates instead of becoming a UI error.
  Existing exact source/file hash/identity/restoration/IME assertions and deadlines
  remain, with additional checks that cancellation did not publish a false error.
- **Evidence:** original combined app SHA-256
  `6409e2c82a79f93fd714cc06055128677acd148509453aee3011f7f310279552`;
  `android/rust-target/integration-main-api26-full.log` records 119 passes, six
  expected skips and two failures. That artifact predates the B-135/B-136 fixes.
  Final artifact identities and results are recorded in PROGRESS.
- **Final regression evidence:** corrected app
  `96afa07123f1167c54855fafb8d449b212fb859c797b0d31c3877909a528b920`, test
  `3e784cda2db4584b3d4c3f76874538094485ad725f7a93803f934945c540a436`:
  both former API26 failures pass unchanged in the focused two-case run (12.268s)
  and the complete API26/API37 runs. API26 has 121 passes/six expected skips/zero
  failures (179.198s); API37 has 134 passes/three expected skips/zero failures
  (193.974s), with 91 JVM passes. Ordinary task/preferences/capture/draft files
  remain byte-identical; API37's derived system-search receipt timestamp changes.
- **Remaining:** broader physical low-memory/process-failure and lifecycle acceptance.

## B-137 — Android release metadata validator expects an obsolete aapt2 label

- **Severity:** Medium; future Android release job blocked. **Feature:** F-060.
  **Affected:** Android release packaging only; installed apps are unaffected.
- **Reproduction/cause:** the first clean committed-source build succeeds with
  release APK/AAB, JVM tests and lint, then packaging refuses valid API26 metadata.
  Pinned Build Tools 37's `aapt2 dump badging` prints `minSdkVersion:'26'`; the
  initial fixture/parser expected the older `sdkVersion:'26'` label. Recorded in
  `android/rust-target/integration-clean-source-package-first.log`.
- **Fix/status:** Verified for local clean-source packaging; first hosted execution remains open. Parse the
  actual current minimum-SDK line and retain support for the legacy label. Require
  exactly one minimum/target value, 26/37. Wrong versions/IDs/SDKs/debuggable
  artifacts remain rejected. The behavioral metadata fixture uses real current
  output spelling and covers the legacy spelling explicitly.
- **Evidence:** clean committed `c24b114` packaging succeeds, including fresh APK/AAB,
  JVM tests/release lint, all native alignment checks, matching source and seven
  verified artifact digests. See the 2026-10-11 PROGRESS handoff.
- **Remaining:** first hosted tagged workflow; store admission and official-key
  signing/update acceptance remain separate.
