# Contributing

- Build with GNOME Builder (open the folder, press Run) or with the commands in the README. First put a LibreSync checkout next to the workspace: `ln -s ../LibreSync libresync-src` (see `docs/P2P.md`).
- Run `cargo fmt --all` before committing; CI checks formatting and builds the Flatpak for x86_64 and aarch64.
- Keep to the [GNOME Human Interface Guidelines](https://developer.gnome.org/hig/): standard widgets, no custom CSS, full keyboard access, tooltips on icon buttons.
- Every user-visible string goes through `gettext` (Rust) or `_("…")` (Blueprint), and every file with strings is listed in `po/POTFILES.in` (`build-aux/check-potfiles.sh` enforces it in CI). Regenerate `po/momentum.pot` with the change that adds strings; see `docs/TRANSLATING.md`.
- Every icon-only control gets a tooltip and an accessible label; run `build-aux/a11y-dump.py` against the open app before sending UI changes. See `docs/ACCESSIBILITY.md`.
- Sync-format changes must stay byte-compatible with Super Productivity; add a test in `crates/sp-sync` and run it against `build-aux/mock-webdav.py`.
- License is GPL-3.0-or-later with SPDX headers (see `REUSE.toml` for files that cannot carry one); there is no contributor license agreement.
- Releases: bump the version in `meson.build`, `Cargo.toml` and the metainfo `<release>`, update `CHANGELOG.md`, then push a `vX.Y.Z` tag. CI attaches Flatpak bundles to the GitHub Release and publishes signed builds to the Flatpak repository on the `gh-pages` branch (signing key in the `FLATPAK_GPG_*` secrets; keep an offline backup). Flathub files live in `build-aux/flathub/`.
