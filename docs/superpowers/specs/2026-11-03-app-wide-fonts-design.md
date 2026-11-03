# App-wide font choices (F-022)

Approved by Dan on 2026-11-03: add System, Atkinson Hyperlegible Mono and
OpenDyslexic to the native typography settings on Linux, macOS, iOS and Android.
Bundle fonts and licenses for offline use. Keep existing installed-font pickers,
saved custom font selections, independent content/interface sizes, OS accessibility
text scaling and existing reset semantics. Selection alone must not change sizes.
Font preferences remain local, outside portable task/settings sync.

Use platform adapters: Pango/GTK application fonts on Linux, CoreText registration
from the shared MomentumKit resource bundle on Apple platforms, and packaged Compose
font families on Android. No new service or UI toolkit; task rules and data formats
are unaffected. iOS follows AsNeeded's bundled registration and Dynamic Type custom
font approach, with the specifically requested Mono family rather than AsNeeded's
proportional Atkinson font. Cover explicit native text styles too; keep SF Symbols
and OS-owned chrome in their native font. Unknown/unavailable fonts fall back safely.

Verify resource integrity/packaging, persistence, independent sizes, fallback and
reset with isolated tests. Build all available native clients, inspect representative
isolated native views where possible, and record Linux/device/accessibility limits
without marking unperformed acceptance Verified. Update FEATURES, BUGS and PROGRESS.
Commit and push only to task/app-wide-fonts; no release or merge is authorized.
