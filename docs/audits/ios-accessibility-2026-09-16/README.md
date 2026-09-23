# Accessibility audit evidence — 2026-09-16

Synthetic UI-test fixtures only, dirty `task/ios-app` based on `7ec82e9`.
See the [audit report](../2026-09-16-ios-accessibility.md) for runtime/appearance
context, findings, failed attempts and acceptance limits.

`*-inventory.json` files are unfiltered XCTest `.all` diagnostic results.
`automatedFindingsEmpty: false` means the audit returned findings. A green
traversal is not an accessibility pass. Duplicate findings across screens,
covered/offscreen elements and stale system nodes are retained. Empty element
fields mean the referenced node could not be resolved; the issue itself is kept.

Unprefixed inventories are iOS 27 light; `dark26-` is iOS 26.5 dark with Increase
Contrast configured; `ipad27-` is iPad mini on iPadOS 27, light, AX5. `landscape27-` is the German
landscape retry; its screenshot anomaly remains unresolved. Images are original screenshots, and the subtask text file
is the original native accessibility hierarchy. No images were edited.
