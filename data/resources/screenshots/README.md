# Momentum preview screenshots

The root README's Apple screenshots were captured on **September 16, 2026** from
local development builds based on `7ec82e9` (dirty `task/ios-app`). Both use the
shared engine's built-in sample tasks. They do not show personal tasks or accounts.

| Image | Capture |
|---|---|
| `macos-preview.png` | Native macOS Today window, light appearance, built from this worktree. The footer confirms “Preview mode · Sync is off.” |
| `ios-preview.png` | iPhone simulator, iOS 27, Today with the floating Add task button, light appearance, ordinary Dynamic Type, exact default #FF6600 accent. |

The iOS status bar uses the simulator's standard 9:41 / full-battery override.
Images are native window/device captures; task content and UI were not composited
or retouched. README display widths preserve the original aspect ratios.
The existing Linux screenshots predate this update and remain historical previews.

## Capture another macOS preview

Build with the [macOS guide](../../../macos/README.md), then use a disposable copy
of the app bundle with a separate bundle identifier/preferences domain if you
already run Momentum normally. A local ad-hoc signature is sufficient for that
preview copy. Do not replace your installed app or use your personal store.

```sh
# Point at the isolated preview app bundle.
MACOS_PREVIEW_APP=/path/to/MomentumPreview.app
open -n -a "$MACOS_PREVIEW_APP" --env MOMENTUM_DEMO=1 \
  --env MOMENTUM_PREVIEW_APPEARANCE=light
```

`MOMENTUM_DEMO` creates sample tasks in the temporary `momentum-demo` directory
and disables sync. `MOMENTUM_PREVIEW_APPEARANCE` is a Debug-only appearance override.
Launch without extra app arguments for the initial window: during this capture,
argument-bearing launches did not present it. Check the Preview mode footer before
capturing the window. Use macOS window capture, excluding other apps and the desktop.

```sh
screencapture -x -o -l "$MACOS_PREVIEW_WINDOW" macos-preview.png
```

## Capture another iOS preview

Build with the [iOS guide](../../../ios/README.md) and use a dedicated simulator.
`--demo` selects a disposable preview store and preferences domain; sync and live
system notifications are disabled. Never seed a personal device for screenshots.

```sh
# Set these to your dedicated simulator ID and built simulator app path.
xcrun simctl install "$IOS_PREVIEW_DEVICE" "$IOS_PREVIEW_APP"
xcrun simctl ui "$IOS_PREVIEW_DEVICE" appearance light
xcrun simctl launch "$IOS_PREVIEW_DEVICE" com.codedbydan.Momentum.ios --demo \
  -AppleLanguages '(en)' -AppleLocale en_US \
  -UIPreferredContentSizeCategoryName UICTContentSizeCategoryL
xcrun simctl status_bar "$IOS_PREVIEW_DEVICE" override --time '9:41' \
  --dataNetwork wifi --wifiMode active --wifiBars 3 \
  --batteryState discharging --batteryLevel 100
# Wait for sample tasks to appear before capture.
xcrun simctl io "$IOS_PREVIEW_DEVICE" screenshot ios-preview.png
xcrun simctl status_bar "$IOS_PREVIEW_DEVICE" clear
```

Preview screenshots are documentation evidence, not feature-parity or
accessibility acceptance. See the [feature ledger](../../../docs/FEATURES.md) and
[iOS audit](../../../docs/audits/2026-09-16-ios-accessibility.md) for those limits.
