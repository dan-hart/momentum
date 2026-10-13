## What does this change?

## How was it tested?

- [ ] `cargo fmt -p momentum -p mo -p momentum-core -p momentum-ffi -p uniffi-bindgen -p momentum-model -p momentum-oplog -p momentum-store -p momentum-sync -p momentum-p2p -- --check` passes
- [ ] The Flatpak builds and its meson tests pass
- [ ] Sync-format changes preserve existing formats and pass the in-process WebDAV and compatibility regressions
- [ ] New strings use `gettext` / `_()` and files are listed in `po/POTFILES.in`
