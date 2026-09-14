<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Translating Momentum

Momentum uses GNU gettext, the same pipeline as every GNOME app. Strings live in
`po/momentum.pot`; each language has a `po/<code>.po` file; `po/LINGUAS` lists the
languages that are built into the app.

## Translate on Weblate

The easiest way is the hosted Weblate instance, no build environment needed:

<https://hosted.weblate.org/projects/momentum/>

Sign in, pick your language (or request a new one), translate, done. Weblate commits to
this repository on your behalf and your name goes into the file header. Please keep the
GNOME glossary conventions of your language (the same words GNOME Settings and Files use),
and leave `%s`, `{}` and the `_` mnemonic underscore where they are.

## Translate with a .po editor

1. Add your language code to `po/LINGUAS` (one per line, alphabetical).
2. Copy the template: `msginit -l de_DE.UTF-8 -i po/momentum.pot -o po/de.po`
   (replace `de` with your code).
3. Edit with [Gtranslator](https://flathub.org/apps/org.gnome.Gtranslator),
   [Poedit](https://poedit.net/) or [Lokalize](https://apps.kde.org/lokalize/).
4. Check it compiles: `msgfmt --check --statistics -o /dev/null po/de.po`.
5. Open a pull request.

The app picks translations up at build time; run the Devel build with
`LANGUAGE=de flatpak run io.github.dan_hart.Momentum.Devel` to see yours.

## Maintainers: keeping the template fresh

Every file with translatable strings must be listed in `po/POTFILES.in`;
`build-aux/check-potfiles.sh` fails CI when one is missing. Regenerate the template and
merge it into the existing translations (inside the GNOME SDK, which has xgettext and
blueprint-compiler):

```sh
flatpak run --devel --share=network --filesystem=home --command=bash org.gnome.Sdk//50 -c '
  export PATH=/usr/lib/sdk/rust-stable/bin:$PATH &&
  meson setup target-sdk/pot-build &&
  ninja -C target-sdk/pot-build momentum-pot momentum-update-po'
```

Commit `po/momentum.pot` and the updated `.po` files with the change that added the
strings, so Weblate sees new strings as soon as they land on `main`. xgettext warns that
`.blp` is an unknown extension and falls back to C parsing; that works for Blueprint's
`_("…")` calls and can be ignored.

Wording rules for source strings, so translators get something translatable:

- Full sentences or full labels, never fragments glued together at runtime (use `format!`
  with a single translated string instead).
- Sentence case for labels and menu items, Title Case only for window and dialog titles,
  no trailing punctuation on buttons.
- Add a translator comment (`// Translators: …` in Rust, `/* Translators: … */` in
  Blueprint) whenever a string is ambiguous out of context.
- American spelling in source strings.

## Hosting on Weblate

Weblate hosts libre projects for free. The project owner requests it once at
<https://hosted.weblate.org/hosting/> with these settings:

| Field | Value |
|---|---|
| Repository | `https://github.com/dan-hart/momentum` |
| Branch | `main` |
| File mask | `po/*.po` |
| Template (monolingual base) | leave empty; the pot is `po/momentum.pot` |
| New translation base | `po/momentum.pot` |
| File format | gettext PO file |
| Push | via the Weblate GitHub app, or pull requests |
| Add-ons | "Update PO files to match POT", "Add languages to LINGUAS" |

Until that is approved, pull requests with `.po` files are just as welcome.
