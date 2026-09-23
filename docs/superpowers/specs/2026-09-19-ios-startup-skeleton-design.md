# iOS Startup Skeleton Design

**Date:** 2026-09-19  
**Feature:** F-042, related to F-019/F-020/F-021/F-039  
**Scope:** Momentum iOS/iPadOS initial Today load  
**Status:** Approved by the user, who authorized the recommended design and implementation without an intermediate mockup

## Goal

Replace the initial task spinner with a polished skeleton experience that resembles the Today screen and four-tab layout. It must feel native on iOS 26 and 27, remain fast and battery-conscious, preserve the stable floating-button geometry, and respect accessibility settings.

## Considered approaches

1. **Redact the real app chrome and render a Today-shaped content skeleton — selected.** The actual `TabView`, navigation title, toolbar, safe areas, and floating control remain in their production geometry. The initial task body supplies placeholder rows. This produces the smallest transition and cannot drift from the native tab bar.
2. **Cover the app with a separate full-screen replica.** This provides total visual control but duplicates native tab-bar and navigation geometry, increasing drift across iPhone, iPad, Dynamic Type, and future OS releases.
3. **Use a static launch-screen image.** This can appear before SwiftUI starts but cannot shimmer, adapt to appearance or text size, or wait for the first real snapshot. It would also duplicate the production UI.

## Presentation

While the first Today snapshot loads:

- The real four-tab `TabView`, Today navigation chrome, and bottom safe-area geometry remain mounted.
- SwiftUI's native redaction covers the real tab labels and Today chrome.
- The task area shows an inset-grouped Today skeleton with a section heading and five rows. Each row contains a completion-circle placeholder plus primary and secondary text bars.
- The existing floating Add Task position remains reserved, preventing a loading-to-content jump.
- One soft highlight band moves across the placeholder content. It uses opacity and translation only, runs at no more than 30 frames per second, and is scoped to the short-lived skeleton.
- The skeleton uses semantic system fill colors, so light, dark, and Increase Contrast appearances stay coherent without fixed gray values.
- The loaded screen replaces the skeleton with a 0.2-second opacity transition. No spring, bounce, blur, or layout animation is used.

The skeleton is a loading state rather than a second app screen. It never displays user task data and does not persist after the first valid Today snapshot.

## Accessibility and energy

- The placeholder rows are hidden from accessibility.
- During loading, assistive technology receives one localized `Loading Momentum` status element instead of reading fake tasks or disabled tabs.
- Interaction is blocked until the initial snapshot is ready, preventing taps on redacted controls. The redacted TabView subtree is also accessibility-hidden; an unredacted sibling exposes the sole loading status.
- Reduce Motion disables both shimmer and the content crossfade.
- Low Power Mode disables shimmer while retaining the static skeleton.
- An inactive scene pauses shimmer. A `TimelineView(.animation(minimumInterval:paused:))` schedule uses a minimum interval of 1/30 second and pauses for Reduce Motion, Low Power Mode, an inactive scene, or completed startup.
- Dynamic Type scales placeholder line heights and row spacing; compact and accessibility sizes remain scroll-safe.

## Architecture

- Add a focused `TodaySkeletonView` and reusable shimmer modifier in the iOS design layer. Only the root Today `TaskScreen` receives startup-skeleton mode; Upcoming, Search, projects, tags and Archive retain their ordinary loading presentation.
- Add a small, pure `StartupSkeletonMotionPolicy` so Reduce Motion and Low Power Mode behavior is directly unit-testable.
- The root Today `TaskScreen` replaces its initial `ProgressView` with the skeleton and reports once, only after assigning a valid `.today` snapshot. Later refreshes and first visits to other destinations never show `today-loading-skeleton`.
- `TaskWorkspace` forwards the first Today-ready event.
- `RootView` owns the one-shot startup presentation state, redacts the real tab hierarchy, blocks interaction, hides that subtree from accessibility, and exposes the single loading accessibility element.
- Existing later refresh behavior remains unchanged; skeletons do not reappear for ordinary invalidations.

## Cold entry and failure precedence

- A cold Spotlight route, reminder editor, or another explicit system route supersedes the startup skeleton immediately and permanently for that scene. The route is neither delayed nor replayed, and later navigation to Today uses its ordinary loading state if needed.
- A summary notification may continue to select Today; it uses the skeleton only while the first Today snapshot is genuinely loading.
- URL mutations continue through the existing serialized engine path. They do not create a second loading owner or consume the pending route.
- `StoreUnavailableView` takes immediate precedence over the skeleton. Retry constructs the normal startup path again and reaches the skeleton only after the store becomes available.
- Hosted integration tests cover cold Spotlight, a cold reminder presentation, store failure/retry and exactly-once route preservation.

## Verification

Test first with the existing fast hosted SwiftUI harness:

- Initial Today state contains the skeleton and retains the floating Add Task frame.
- The skeleton disappears after the first snapshot and does not return during later refreshes.
- The actual four-tab contract remains mounted while redacted. During loading, the accessibility snapshot contains exactly `Loading Momentum` and excludes tabs, toolbar actions, Add Task and placeholder controls; the production hierarchy returns after readiness.
- Reduce Motion, Low Power Mode and inactive-scene policy produce a static skeleton; the full motion policy verifies the 30 fps upper bound and 0.2-second transition.
- Later tab switches and sidebar destinations never show the Today skeleton.
- Light/dark and standard/increased contrast renders contain nonblank adaptive placeholder content.
- AX5 and compact-width renders fit without horizontal clipping.
- The loading hierarchy exposes one meaningful accessibility status and no fake task controls.

Run public `UIHostingController` hosted tests, the complete iOS unit lanes on iOS 26.5 and 27, the production simulator build, localization checks, and `git diff --check`. Capture simulator screenshots with disposable demo data only and record each runtime separately. Do not add XCUITest/XCUI, ViewInspector or a physical-device completion claim.
