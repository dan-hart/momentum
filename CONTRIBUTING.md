# Contributing

- Build with GNOME Builder (open the folder, press Run) or with the commands in the README. First put a LibreSync checkout next to the workspace: `ln -s ../LibreSync libresync-src` (see `docs/P2P.md`).
- Run `cargo fmt -p momentum -p mo -p sp-model -p sp-oplog -p sp-store -p sp-sync -p sp-p2p` before committing (not `--all`, which would also reformat the LibreSync checkout); CI checks formatting and builds the Flatpak for x86_64 and aarch64.
- Keep to the [GNOME Human Interface Guidelines](https://developer.gnome.org/hig/): standard widgets, no custom CSS, full keyboard access, tooltips on icon buttons.
- Every user-visible string goes through `gettext` (Rust) or `_("…")` (Blueprint), and every file with strings is listed in `po/POTFILES.in` (`build-aux/check-potfiles.sh` enforces it in CI). Regenerate `po/momentum.pot` with the change that adds strings; see `docs/TRANSLATING.md`.
- Every icon-only control gets a tooltip and an accessible label; run `build-aux/a11y-dump.py` against the open app before sending UI changes. See `docs/ACCESSIBILITY.md`.
- Every change comes with tests: reducer and model rules in their crates, UI behaviour in `crates/app/src/tests/ui.rs`. Run `build-aux/test.sh` before pushing; CI runs the same suite headless. See `docs/TESTING.md`.
- Sync-format changes must stay byte-compatible with Super Productivity; add a test in `crates/sp-sync/src/feature_tests.rs` against the in-process WebDAV mock (`mock_dav.rs`).
- License is GPL-3.0-or-later with SPDX headers (see `REUSE.toml` for files that cannot carry one); there is no contributor license agreement.
- Releases: `build-aux/release.py bump X.Y.Z`, commit, push a `vX.Y.Z` tag. One workflow then runs every test and publishes the GitHub Release with all binaries, the Homebrew tap, the signed Flatpak repository on `gh-pages` and the Flathub update. See `docs/RELEASING.md`.
