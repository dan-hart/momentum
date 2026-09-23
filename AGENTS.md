# Working on Momentum

## Purpose

Momentum is a task app that respects your freedom and delivers an excellent native
experience on Linux, macOS, iOS, and Android. Native quality, accessibility, privacy,
and reliable offline use are product requirements, not finishing touches.

The code will always be open source and free to build, compile, self-host, and use
yourself. Preserve the existing GPL-3.0-or-later licensing and notices. Pricing for
official distributions is undecided. Do not introduce a paywall or commercial
requirement into source builds without an explicit product decision consistent with
this commitment.

## Read first

1. Inspect the checkout, branch/worktree, Git status, and applicable instructions.
   Preserve existing changes; do not assume the outer checkout contains current work.
2. Read [FEATURES.md](docs/FEATURES.md), [BUGS.md](docs/BUGS.md), and
   [PROGRESS.md](docs/PROGRESS.md), then the specifications relevant to the task.
3. Use [MVP.md](docs/MVP.md) for existing task behavior,
   [TESTING.md](docs/TESTING.md) for verification, [RELEASING.md](docs/RELEASING.md)
   for how a tag becomes a release on every platform, and
   [macos/README.md](macos/README.md) for the Apple desktop build.
   [MACOS-MVP-CHECKLIST.md](docs/MACOS-MVP-CHECKLIST.md) and
   [MACOS-AUTOMATION.md](docs/MACOS-AUTOMATION.md) contain historical acceptance evidence.

The user's current instructions take precedence. This guide records agreed product
direction and workflow; older specs describe the existing implementation. Preserve
existing behavior unless the task changes it. Where old docs conflict with approved
direction, record the gap instead of silently treating the old behavior as the target.
Ask when the intended product behavior remains ambiguous.

## One shared core, native apps

| Component | Responsibility and current baseline |
|---|---|
| `crates/momentum-core` | Toolkit-independent Rust `Engine`: task rules, queries, mutations, undo, recurrence, reminders, storage access, and sync orchestration |
| `crates/sp-model`, `sp-oplog`, `sp-store`, `sp-sync`, `sp-p2p` | Existing data, persistence, and sync foundation; preserve compatibility pending the v2 decision |
| `crates/app` | Linux: Rust, GTK4/libadwaita, native GNOME behavior; calls the shared engine directly |
| `crates/momentum-ffi` | UniFFI boundary for native clients; bindings are generated from the core |
| `macos/Momentum` | macOS: SwiftUI with AppKit and system services where appropriate |
| `macos/Packages/MomentumKit` | Mac presentation state, localized wording, preferences, and platform adapters |
| iOS and Android | Upcoming native clients of the same Rust core; platform foundations are not implemented yet |
| `crates/mo` | Linux/macOS CLI; preserve task semantics and safe coordination with the running app |

- Implement reusable task behavior once in Rust. Do not duplicate business rules in
  Swift, Kotlin, or GTK callbacks to make one platform work.
- Keep UI layout, navigation, focus, gestures, accessibility, localized wording,
  permissions, and OS integrations in native platform layers. The core returns
  structured values rather than localized UI sentences.
- Use each platform's best practices and native controls. Follow GNOME guidance on
  Linux, Apple's HIG on Apple platforms, and Android's native design/accessibility
  guidance. Consult current official documentation when making platform decisions.
- Mobile architecture must reuse the Rust core without inheriting desktop-only
  dependencies. The current MomentumKit package imports AppKit; it is not already
  an iOS-compatible package. Agree on mobile scaffolding/dependencies before adding them.
- Do not replace native interfaces with a WebView, Electron, Flutter, React Native,
  or a shared cross-platform UI toolkit to obtain parity. Share behavior, not widgets.
- Regenerate FFI bindings through the build tooling; do not hand-edit generated code.
  Keep core message/enum handling exhaustive across existing frontends.

## Platform scope and parity

Linux and macOS are the established MVP baselines. iOS and Android are next; their
order and delivery schedule are not yet decided. An established baseline does not
mean every feature or device integration has been verified.

By default, implement a requested feature or applicable bug fix on **every existing
platform**, unless the user explicitly narrows the task. Until mobile apps exist,
record explicit iOS/Android requirements instead of creating their foundations as an
incidental part of desktop work. Once a platform is established, include it by default.

Equivalent capability does not require identical screens, gestures, or services.
Choose native equivalents and document intentional differences. A desktop-only OS
integration may be Not applicable on mobile, but must have a reason. A missing feature
is Planned or Blocked, never Not applicable merely to hide a gap.

## Offline, privacy, and sync

- Core task management must work without an account or internet connection. Local
  changes must persist reliably and remain usable when a sync service is unavailable.
- Sync is optional and off by default. When enabled, make setup, automatic syncing,
  recovery, and status understandable and seamless within OS background limits.
- The target is one extensible provider selection: Off, Nextcloud, or LibreSync,
  with room for future services. Only the selected provider should run. Both desktop
  implementations follow this policy; keep their separate verification status visible.
- Preserve direct device-to-device syncing through LibreSync as a product capability.
  Do not make a proprietary Momentum account or hosted service necessary for local use.
- Show useful progress, last successful sync, and actionable persistent errors with
  retry. Never report success for a failed exchange or conceal unsynced changes.
- Preserve offline edits, deletion semantics, and conflict handling. Switching providers
  must preserve tasks and saved connections. Ask before changing formats or migrating data.
- Store secrets in platform credential storage. Do not place credentials, personal task
  contents, device identifiers, or private server addresses in committed docs or logs.
- No advertising, tracking, or analytics without explicit user approval of the exception.
  Do not add SDKs that quietly introduce those behaviors.
- Use isolated test stores, settings, and test credentials. Demo/preview mode must never
  sync disposable data with real accounts or devices. Do not modify user data for tests.

## Native quality and accessibility

- Design for each platform's conventions: system appearance/accent, typography,
  navigation, menus, keyboard behavior, touch/pointer interactions, and adaptive layouts.
- Accessibility is part of every feature: semantic labels and roles, logical focus,
  keyboard access where appropriate, screen-reader support, scalable text, contrast,
  reduced motion, and alternatives to color-only meaning.
- Provide useful empty, loading, offline, error, and recovery states. Keep task entry
  easy to find. Prefer undo for reversible task operations and clearly mark destruction.
- Motion and haptics should explain actions or provide restrained feedback; respect
  platform accessibility settings. Essential behavior must not depend on animation.
- Keep localized strings in platform resources and preserve existing English/German
  coverage. Test longer strings, larger text, and compact layouts for changed UI.
- Native interaction checks matter: core tests cannot prove drag-and-drop, focus,
  screen-reader output, or system integrations work in a running app.

## Living project records

Keep these three ledgers current in the same change as implementation:

- **`docs/FEATURES.md`:** stable `F-###` IDs, exact capability/acceptance scope, shared
  core impact, separate Linux/macOS/iOS/Android statuses, native differences, evidence,
  and remaining work. Split rows when parts have different acceptance status.
- **`docs/BUGS.md`:** stable `B-###` IDs, severity, reproduction, expected/actual behavior,
  affected-platform assessment, cause when known, fix, regression evidence, and remaining
  validation. Retain resolved entries. Distinguish suspected impact from reproduced bugs.
- **`docs/PROGRESS.md`:** current priorities and blockers, approved decisions and open
  questions, plus dated handoffs with linked IDs, platforms touched, checks/results,
  verification limits, and next concrete actions. Reference evidence instead of copying
  whole specs or treating stale test counts as current results.

Use these platform statuses consistently:

| Status | Meaning |
|---|---|
| Planned | Required or proposed, with implementation still outstanding |
| In progress | Work has started but the intended implementation is incomplete |
| Implemented | Code exists; required verification is still outstanding |
| Verified | Relevant tests, platform build, and applicable native interaction checks passed for the recorded scope/revision |
| Blocked | Progress needs a stated dependency, decision, or external change; record the last completed stage and next action |
| Not applicable | Intentionally inapplicable on this platform, with a concrete reason |

**Verified is the bar for calling work complete.** A shared-core pass does not make
four platforms Verified. A deferred check is not a pass: preserve the implementation
status and identify the user-approved deferral. Record unavailable SDK/device/CI checks
and continue useful independent work; do not repeatedly ask the user to perform them.
Evidence must identify its date, scope, revision or dirty-worktree state, and source
(automated, live inspection, or user confirmation). Reassess affected evidence after changes.

## Working style and authority

- Proceed autonomously with routine, reversible implementation and fixes inside the
  agreed scope. Keep changes focused; preserve inherited work and unrelated behavior.
- Ask before major architecture changes, data migrations, adding dependencies/services,
  monetization decisions, or choosing ambiguous product behavior. Prepare a concrete
  proposal first. Reuse approvals already given; do not ask again for the same decision.
- Keep questions concise. Report material findings and blockers, not a narration of
  every command. Finish useful independent work while waiting for necessary answers.
- **Local changes only by default.** Do not commit, push, open a PR, merge, tag, publish,
  or release without the user's explicit request. Do not reset, clean, overwrite, or
  discard unrelated changes. Use `task/` for new branches when a branch is needed.
- Search/read the relevant source before editing. Use existing architecture and tools;
  avoid speculative frameworks, broad rewrites, and unrelated cleanup.

## Verification and handoff

1. Identify the feature/bug IDs and affected platforms before implementing.
2. Add appropriate behavioral regression coverage for logic changes and bug fixes.
   Test rules in the core and native behavior at the platform boundary; do not add tests
   that only mirror implementation or meaningless checks for documentation/cosmetic edits.
3. Run the relevant checks from [TESTING.md](docs/TESTING.md). Shared-core changes must
   also check existing consumers, including `mo` and generated FFI compatibility.
   Useful entry points:
   - `cargo test --workspace --exclude momentum`
   - `cargo test --workspace --exclude momentum --all-features --all-targets`
   - `swift test --package-path macos/Packages/MomentumKit`
   - `build-aux/test.sh` on a supported Linux/Flatpak environment
   - `macos/scripts/build-core.sh --debug` and the Xcode build from the Mac README
4. For Linux changes on a Mac, compilation alone is not Linux runtime validation.
   For native UI changes, inspect relevant flows using isolated data. Keep any skipped
   real-device, network, accessibility, or CI checks explicit. Do not rerun previously
   deferred checks unless the user resumes them or their scope clearly changes.
5. Update all affected ledgers and specs, review the diff, and run appropriate formatting/
   documentation-link checks. Documentation-only changes do not require rebuilding apps.
6. Handoff concisely: what changed, platform status, actual verification, remaining gaps,
   and next steps. Never claim a build, fix, or platform is complete from inference alone.

## Future direction, not authorization to implement

- **v2:** become independent of Super Productivity. The replacement format, migration
  strategy, and ongoing SP interoperability are undecided. Preserve current compatibility
  until an explicit design is approved; do not rename/rewrite the `sp-*` foundation
  merely to make the project appear independent.
- **Revenue:** monthly subscriptions are a possibility, with RevenueCat desired on Apple
  platforms eventually. Tipping through Apple and/or Buy Me a Coffee is also a future
  goal. Pricing, entitlements, distribution, and rollout remain undecided. Investigate
  current platform rules when that work is requested; add no SDKs or billing now.
- These plans must preserve open-source freedom, free self-build/self-host use, privacy,
  native quality, and offline core task management.
