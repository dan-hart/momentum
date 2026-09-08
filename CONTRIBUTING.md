# Contributing

- Build with GNOME Builder (open the folder, press Run) or with the commands in the README.
- Run `cargo fmt --all` before committing; CI checks formatting and builds the Flatpak for x86_64 and aarch64.
- Keep to the [GNOME Human Interface Guidelines](https://developer.gnome.org/hig/): standard widgets, no custom CSS, full keyboard access, tooltips on icon buttons.
- Every user-visible string goes through `gettext` (Rust) or `_("…")` (Blueprint), and every file with strings is listed in `po/POTFILES.in`.
- Sync-format changes must stay byte-compatible with Super Productivity; add a test in `crates/sp-sync` and run it against `build-aux/mock-webdav.py`.
- Licence is GPL-3.0-or-later with SPDX headers; no contributor licence agreement.
