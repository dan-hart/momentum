# Momentum iOS icon

Edit `../Momentum/Resources/MomentumIcon.icon` in Apple's Icon Composer. This is the
primary iPhone/iPad icon selected in `../project.yml`; the older AppIcon bitmap is
retained as a reference, not the selected primary icon. The fast view-test target
excludes the native icon to avoid unnecessary icon compilation.

- Default: orange background, white checkmark and softer white motion lines.
- Dark: Apple's automatic charcoal-to-black background, exact sRGB `#FF6600`
  checkmark, and matching motion lines at 60% opacity.
- Two editable SVG layers on a 1024-point canvas; no baked corner mask, border,
  shadow, or margin. Foreground materials are disabled to preserve the flat mark
  and requested color. The OS supplies the enclosure treatment and tinted/clear
  variants for the user's Home Screen appearance selection.
- Filled vector outlines are intentional. The iOS 26 renderer filled the interior
  of a stroked checkmark when a color override was applied; outlines avoid that
  difference and preserve the same silhouette on 26 and 27.

## Previews

| iOS 26 dark | iOS 27 dark | iOS 27 default |
|---|---|---|
| ![iOS 26 dark](IconPreviews/dark-ios26.png) | ![iOS 27 dark](IconPreviews/dark-ios27.png) | ![iOS 27 default](IconPreviews/default-ios27.png) |

These are unretouched exports from Apple's renderer, not replacement assets. The
visible corner transparency and rim are renderer output; the source background
remains full bleed. Apple's enclosure lighting varies between system versions.

## Export and check

From the repository root with Xcode 27 installed:

```sh
ICON_TOOL='/Applications/Xcode-27.0.0.app/Contents/Applications/Icon Composer.app/Contents/Executables/ictool'
"$ICON_TOOL" "$PWD/ios/Momentum/Resources/MomentumIcon.icon" --export-image \
  --output-file "$PWD/ios/Design/IconPreviews/dark-ios27.png" \
  --platform iOS --rendition Dark --width 512 --height 512 --scale 1 \
  --design-generation 27
```

Use design generation `26` to verify the earlier renderer. Also inspect `Default`
and `TintedDark`, plus 64-point exports for Home Screen legibility. Build the normal
Momentum scheme and confirm `CFBundleIconName` is `MomentumIcon` for iPhone/iPad.

2026-09-16, dirty `task/ios-app` based on `7ec82e9`: both dark-renderer exports
produce `[255, 102, 0, 255]` at the interior checkmark sample `(280, 320)` in a
512×512 sRGB Core Graphics decode. Both have a nonuniform neutral dark background.
Use a color-managed CGImage decode for pixel checks: NSBitmapImageRep.colorAt
returns a calibrated NSColor even for these tagged sRGB files, so converting that
result again to sRGB produces misleading readings.

Design references: [Apple app-icon HIG](https://developer.apple.com/design/human-interface-guidelines/app-icons)
and [Icon Composer](https://developer.apple.com/documentation/xcode/creating-your-app-icon-using-icon-composer).
Physical Home Screen customization was not part of this export/build validation.
