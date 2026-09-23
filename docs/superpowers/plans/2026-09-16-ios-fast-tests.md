# Fast iOS tests implementation plan

Goal: fast regression feedback for mobile presentation and real SwiftUI rendering,
without ViewInspector, private reflection, or a new dependency. Scope: F-039,
supporting F-002/F-006/F-019/F-020. Local changes only.

The existing MomentumMobile package is the default lane. Extract only the Quick Add
submission state and editor validation already embedded in views; keep task rules in
Rust. Test rejection/retry, overlapping submissions, invalid drafts, and real-core
round trips. Existing notification, lifecycle and palette tests stay in this lane.

Add a separate minimal iOS unit-test host, with no production app lifecycle or store.
Compile the actual app Swift sources into the unit bundle, excluding app entry points.
Use UIHostingController for layout/render tests of task rows, Settings labels and
commit buttons. Assert observable geometry and native colors; do not introspect SwiftUI
implementation types. UIKit-backed controls make ImageRenderer alone insufficient.
This avoids a snapshot dependency and brittle whole-screen pixel baselines. Full UI
automation remains the slower lane for interaction and accessibility acceptance.

- [x] Add package state/validation regressions; observe missing implementation failures.
- [x] Extract state into MomentumMobile and wire the actual sheets to it; run tests.
- [x] Add isolated test host, native unit target and reusable hosting fixture.
- [x] Add representative layout/trait/render regressions and verify a deliberate fault fails.
- [x] Add one runner with fast/views/coverage modes, incremental builds and timings.
- [x] Measure warm runtimes; verify iOS 26 and 27 native suites.
- [x] Document coverage limits and update FEATURES/BUGS/PROGRESS/TESTING.

No universal timing threshold on arbitrary machines. Record test execution separately
from build/simulator startup. Coverage is opt-in and scoped to Swift; no claim of all
bugs absent, complete accessibility, or parity. Cached tests must never silently skip
rebuilding changed Swift sources. Rust binding preparation is explicit and documented.
