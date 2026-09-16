<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# macOS localization

English is the development language; German is included in both targets. App strings
live in `Momentum/Resources/Localizable.xcstrings`. Package strings live in
`Packages/MomentumKit/Sources/MomentumKit/Resources/Localizable.xcstrings` and every
package `String(localized:)` selects `bundle: .module`. The app's permission text is
in `Momentum/Resources/InfoPlist.xcstrings`.

## Update and verify

From the repository root:

```sh
xcodegen generate --spec macos/project.yml
xcodebuild -project macos/Momentum.xcodeproj -scheme Momentum \
  -configuration Debug -derivedDataPath /tmp/momentum-localization build
python3 macos/scripts/localize.py --sync /tmp/momentum-localization --import-gettext
python3 macos/scripts/localize.py --require-complete
python3 -m unittest discover -s macos/scripts/tests -p test_localize.py
swift test --package-path macos/Packages/MomentumKit
# Rebuild after editing/syncing catalogs to compile their current contents.
xcodebuild -project macos/Momentum.xcodeproj -scheme Momentum \
  -configuration Debug -derivedDataPath /tmp/momentum-localization build
swift macos/scripts/check-localization.swift \
  /tmp/momentum-localization/Build/Products/Debug/Momentum.app
```

`SWIFT_EMIT_LOC_STRINGS=YES` is configured in `project.yml`. Sync uses the compiler's
`.stringsdata`, preserving keys such as `%u`, `%llu`, and `%@`, and App Intent parameter
summaries. Do not use `xcstringstool extract` for these catalogs: its lightweight
parser produces `%arg` placeholders that do not match Foundation's runtime keys.
The validation script rejects these placeholders and translation type mismatches.

The importer uses only nonempty, non-fuzzy, context-free singular entries from
`po/de.po`; it never overwrites an existing German catalog translation. It also maps
gettext `{}` placeholders to the compiler's exact types when the placeholder counts
agree. Context-dependent or gettext plural entries are deliberately left for manual
review. German catalog plural variants are preserved. Review newly missing keys in
the catalogs and supply appropriate `one` and `other` variants for counted nouns.
Running the import twice is safe and makes no additional changes.

Current coverage is 249 translated app entries, 141 package entries, and two permission
strings, with zero missing. Three app keys containing only empty text, formatting, or
a number are explicitly nontranslatable. The initial catalogs import 62 confirmed
gettext translations; the remaining text is newly translated for macOS. These are
implementation translations and have not had an independent German-language review.

The package tests and bundle checker run without an app host or UI automation. The
checker verifies the actual compiled app and package bundles, including numeric
interpolation, singular/plural selection, and the App Intent parameter placeholder.
They explicitly select each German `.lproj`: `locale: de` controls formatting but
does not itself override a bundle's process-selected language. The app uses macOS's
normal preferred-language selection.
