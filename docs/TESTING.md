<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Testing

Momentum's tests are arranged so that most of them run anywhere Rust runs, and the rest
run on a headless display. Nothing in the suite touches your real data, settings, session
bus or network: every test gets its own directory, settings live in a memory backend,
the session bus address points nowhere, and the Nextcloud tests talk to an in-process
WebDAV stand-in.

## Run it

```sh
build-aux/test.sh                 # everything, inside the GNOME SDK sandbox (recommended)
build-aux/test.sh -p sp-oplog     # arguments go to cargo test
cargo test --workspace --exclude momentum   # core crates only, on any OS, no GTK needed
```

`build-aux/test.sh` enters `org.gnome.Sdk//50` and runs `build-aux/run-tests.sh`, which
compiles the Blueprint UI, the resource bundle and the GSettings schema into
`target/test-assets`, starts `gtk4-broadwayd` when no display is available, and runs
`cargo test --workspace`. On a desktop with a display the app tests use it; set
`MOMENTUM_TEST_BROADWAY=1` to force the headless path. The Flatpak build runs the same
script as its Meson test, so CI executes the UI tests on every push. The whole suite
finishes in a few seconds once compiled.

## Layers

| Layer | Where | Needs | Covers |
|---|---|---|---|
| Model | `crates/sp-model/src/feature_tests.rs` | nothing | dates, repeat rules and catch-up, Today and Overdue membership, unknown-field round trips |
| Op log | `crates/sp-oplog/src/feature_tests.rs` | nothing | every `Action`: upstream payload, op envelope, vector clocks, `apply` on state |
| Store | `crates/sp-store/src/feature_tests.rs` | tempdir | persistence, pending queue, peer ops, snapshot adoption |
| Sync | `crates/sp-sync/src/feature_tests.rs` + `mock_dav.rs` | tempdir, loopback | first upload, download-only, two-client convergence, conflict retry, relayed-op dedup, archives, encryption errors, op cap |
| P2P | `crates/sp-p2p/src/feature_tests.rs`, `tests/two_devices.rs` | loopback | journal, record adapter, tombstones, snapshots, pairing handler; then two real nodes linking and exchanging |
| CLI | `crates/mo/tests/cli.rs` | the built binary | every `mo` command as a black box, JSON output, the queued ops |
| App logic | `crates/app/src/tests/logic.rs` | GTK thread | formatting, parsing, colours, repeat text, modifier key, demo data |
| App UI | `crates/app/src/tests/ui.rs` | display (Broadway) | windows over demo data: views and sections, quick add, completion and undo, archive paging, selection and bulk actions, day moves, drop targets, reorder, sorting, Coming Up, search, context menus, the task form, repeats and catch-up, preferences, modifier accelerators and the shortcuts overlay, quick-add window, search provider, paste/drop, reminders, projects |

The UI tests are a child module of `window` (declared with `#[path]`), so they can call
the same private methods the widgets call. `crates/app/src/tests/support.rs` runs every
app test on one GTK thread (`on_gtk`), gives each test a window over a fresh copy of the
demo store (`demo_window`) or an empty one (`empty_window`), and offers `pump`,
`pump_ms`, `headings`, `row_ids`, `id_of` and `shown` for assertions.

## Writing a test

- Put behaviour in the crate that owns it: a reducer rule belongs in `sp-oplog`, a
  membership rule in `sp-model`, and only what needs widgets in `ui.rs`.
- UI tests: create the window, act through the same methods the UI uses (`add_task`,
  `set_done`, `go_to`, actions via `activate_action`), call `pump()` after anything that
  queues work, then assert on the store, `row_ids` and `headings`. Windows are never
  presented, so check visibility with `shown(widget)`.
- Reset any setting you change (`reset_settings()`), even on the failure path if the
  test's later assertions depend on it.
- Keep tests independent of the wall clock beyond "today": derive days from `today_str()`
  and `day_str(day_number(..) + n)`.
- Anything slow (network timeouts, sleeps) is a smell; the sync mock answers instantly
  and the p2p test uses loopback with a 15 s ceiling it never approaches.

## Porting

The first six layers are plain Rust and are the contract a port must keep: run them on
macOS and iOS (`cargo test --workspace --exclude momentum`) before and after touching the
core. The UI layer documents the behaviours a native rewrite should reproduce; its test
names double as the checklist in `docs/MVP.md` section 8.
