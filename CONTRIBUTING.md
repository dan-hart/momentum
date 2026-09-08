# Contributing

- Build with GNOME Builder (open the folder, press Run) or with the commands in the README.
- Run `cargo fmt --all` before committing; CI checks formatting and builds the Flatpak for x86_64 and aarch64.
- Keep to the [GNOME Human Interface Guidelines](https://developer.gnome.org/hig/): standard widgets, no custom CSS, full keyboard access, tooltips on icon buttons.
- Every user-visible string goes through `gettext` (Rust) or `_("…")` (Blueprint), and every file with strings is listed in `po/POTFILES.in`.
- Sync-format changes must stay byte-compatible with Super Productivity; add a test in `crates/sp-sync` and run it against `build-aux/mock-webdav.py`.
- License is GPL-3.0-or-later with SPDX headers (see `REUSE.toml` for files that cannot carry one); there is no contributor license agreement.
- Releases: bump the version in `meson.build`, `Cargo.toml` and the metainfo `<release>`, update `CHANGELOG.md`, then push a `vX.Y.Z` tag. CI attaches Flatpak bundles to the GitHub Release. Flathub files live in `build-aux/flathub/`.
