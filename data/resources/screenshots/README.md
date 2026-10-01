# Momentum preview screenshots

All images were captured on **October 1, 2026** from the 0.4.5 sources, each app's
built-in sample tasks in preview mode, sync off, light appearance. They show no
personal tasks or accounts and were not composited or retouched. README display
widths preserve the original aspect ratios.

| Image | Capture |
|---|---|
| `macos-preview.png` | Native macOS Today window, 2000×1400 window capture of a disposable Debug copy launched with `MOMENTUM_DEMO=1 MOMENTUM_PREVIEW_APPEARANCE=light`. |
| `ios-preview.png`, `ios-upcoming.png` | iPhone 18 Pro simulator, iOS 27.2, the Today and Upcoming tabs, launched with `--demo`, standard 9:41 status bar override. |
| `today.png`, `coming-up.png`, `new-task.png` | The GTK app's own window render (`MOMENTUM_SCREENSHOT`), 1100×760, light scheme. Rendered by the native macOS build of the Linux app, which draws the same libadwaita widgets; `today.png` and `coming-up.png` are also the Flathub screenshots. |

## macOS

Build with the [macOS guide](../../../macos/README.md), then use a disposable copy
of the app bundle with its own bundle identifier so your installed app and its
preferences stay untouched. An ad-hoc signature is enough for the copy.

```sh
PREVIEW=/tmp/MomentumPreview.app
ditto build/macos-debug/Build/Products/Debug/Momentum.app "$PREVIEW"
plutil -replace CFBundleIdentifier -string com.codedbydan.Momentum.verify "$PREVIEW/Contents/Info.plist"
codesign --force --deep -s - "$PREVIEW"
open -g --env MOMENTUM_DEMO=1 --env MOMENTUM_PREVIEW_APPEARANCE=light -a "$PREVIEW"
screencapture -x -o -l "$WINDOW_ID" macos-preview.png    # window id from CGWindowListCopyWindowInfo
```

`MOMENTUM_DEMO` seeds sample tasks in the temporary `momentum-demo` directory and
disables sync; `MOMENTUM_PREVIEW_APPEARANCE` is a Debug-only appearance override.
The native GTK capture uses the same temporary directory, so do not run both at once.

## iOS

Build with the [iOS guide](../../../ios/README.md) and use a dedicated simulator.
`--demo` selects a disposable preview store and preferences; sync and live system
notifications are disabled.

```sh
xcrun simctl install "$SIM" build/ios/Build/Products/Debug-iphonesimulator/Momentum.app
xcrun simctl ui "$SIM" appearance light
xcrun simctl launch "$SIM" com.codedbydan.Momentum.ios --demo \
  -AppleLanguages '(en)' -AppleLocale en_US \
  -UIPreferredContentSizeCategoryName UICTContentSizeCategoryL
xcrun simctl status_bar "$SIM" override --time '9:41' \
  --dataNetwork wifi --wifiMode active --wifiBars 3 \
  --batteryState discharging --batteryLevel 100
xcrun simctl io "$SIM" screenshot ios-preview.png      # then tap Upcoming for ios-upcoming.png
xcrun simctl status_bar "$SIM" clear
```

## Linux

The app renders its own window and quits; no desktop capture tool is involved, so the
image has no window decorations and the same pixels on any compositor.

```sh
MOMENTUM_DEMO=1 MOMENTUM_SCREENSHOT=$PWD/data/resources/screenshots/today.png \
  MOMENTUM_SCREENSHOT_SIZE=1100x760 ADW_DEBUG_COLOR_SCHEME=prefer-light \
  flatpak run io.github.dan_hart.Momentum.Devel
```

`MOMENTUM_SCREENSHOT_UPCOMING=1` and `MOMENTUM_SCREENSHOT_DIALOG=1` produce the other
two; `MOMENTUM_SCREENSHOT_DELAY` (seconds) waits for the view to settle. On a Mac the
same variables work with the native build described in
[docs/TESTING.md](../../../docs/TESTING.md#running-the-linux-app-natively-on-macos).
The Nearby Devices dialog needs LibreSync selected and a running node, which preview
mode does not start, so it is no longer pictured.

Preview screenshots are documentation evidence, not feature-parity or accessibility
acceptance. See the [feature ledger](../../../docs/FEATURES.md) and the
[iOS audit](../../../docs/audits/2026-09-16-ios-accessibility.md) for those limits.
