<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Momentum iOS parity acceleration

Date: 2026-09-18. Status: approved by the user on 2026-09-18.

This execution design accelerates the already approved iOS parity architecture in
`2026-09-16-ios-parity-design.md`. It does not change product behavior, architecture,
quality requirements, or authority. Work remains local and uncommitted. Testing uses
only the installed iOS 26.5 and iOS 27 simulators. LibreSync and physical-device work
are excluded by the user's current direction; iOS sync scope is Off and Nextcloud.

## Outcome

Reach verified iOS parity with the macOS app faster by distinguishing missing behavior
from missing evidence, batching related acceptance work, and avoiding redundant builds.
Every completed capability still requires appropriate shared tests, shipping-app native
acceptance, accessibility checks, and current project records. External limitations are
recorded precisely rather than converted into passes.

## Execution model

Use evidence-first batching instead of completing each broad feature independently or
postponing all QA until the end. Maintain one critical path in the existing
`docs/IOS-PARITY-CHECKLIST.md`, `docs/FEATURES.md`, `docs/BUGS.md`, and
`docs/PROGRESS.md`; do not create a second source of truth.

For each bounded capability slice:

1. Classify each open acceptance item as an implementation gap, verification gap,
   external limitation, or approved deferral.
2. Inspect current source and shared rules before changing code.
3. Exercise existing shipping behavior first on iOS 27 using isolated simulator data.
4. When behavior fails, add the smallest meaningful regression at the owning boundary,
   implement the focused fix, and repeat the focused check.
5. Verify the final capability on both iOS 26.5 and iOS 27.
6. Update the four existing project ledgers from exact result bundles and command output.

The remaining work is grouped into five slices in this order:

1. Organization, grouping, sorting, search, and presentation.
2. Nextcloud configuration, transport lifecycle, recovery, and local-edit safety.
3. App Intents, URLs, Spotlight, notification actions, and shared-engine invocation.
4. Keyboard, assistive actions, Dynamic Type, localization, and remaining accessibility
   acceptance that Simulator can exercise.
5. Final parity audit, documentation reconciliation, and the complete validation matrix.

## Ten acceleration controls

1. Keep a strict critical path of simulator-verifiable parity gaps.
2. Remove LibreSync and physical-device checks from active execution while preserving
   their explicit disposition in project records.
3. Separate working code that needs evidence from behavior that needs implementation.
4. Batch related native behaviors into one durable shipping-app acceptance flow.
5. Use iOS 27 for focused red/green work and both runtimes for final slice acceptance.
6. Run the fast unit lane after shared logic changes; run native view suites after
   affected UI changes and at milestones rather than after unrelated documentation.
7. Reuse disposable stores, demo fixtures, helpers, and focused schemes.
8. Freeze architecture and dependencies; permit only parity, correctness,
   accessibility, performance, and required testability changes.
9. Record exact evidence immediately after a slice so stale results cannot become claims.
10. Stop only when the parity matrix has no unexplained gaps and every remaining item is
    either verified, intentionally inapplicable, or explicitly deferred by the user.

## Validation tiers

- **Per edit:** formatting, parse/catalog/project validation, and focused unit tests.
- **Per defect:** failing regression, focused green result, and affected consumer check.
- **Per slice:** shipping-app acceptance on iOS 26.5 and 27, plus relevant shared tests.
- **Milestone/final:** generated-core preparation, the full fast mobile lane, native view
  lanes on both runtimes, affected transport/system schemes, diff review, documentation
  checks, and reconciliation of every feature/bug/checklist status.

Tests must use disposable data and no account, personal credentials, physical device,
or real task store. A simulator or sandbox limitation is documented as such. It cannot
be described as successful native behavior.

## Efficiency boundaries

Do not add broad abstractions, dependencies, speculative features, duplicate test
harnesses, or tests that mirror implementation. Do not rerun an expensive unaffected
scheme after documentation-only changes. Do rerun both supported runtimes whenever a
changed production path can behave differently across iOS versions. Shared task rules
remain in Rust; Swift and SwiftUI retain only native presentation and platform services.

## Completion gate

Momentum iOS parity is complete when the stable feature rows applicable to iOS are
Verified for the approved scope, the parity checklist contains no unexplained open item,
the final shared/mobile/native validation matrix passes, and remaining external checks
carry explicit user-approved dispositions. No commit, push, device install, hosted
service mutation, or release is part of this acceleration design.
