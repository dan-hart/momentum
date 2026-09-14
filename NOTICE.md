# Notices

The build system, Flatpak scaffolding and CI configuration derive from
[gtk-rust-template](https://gitlab.gnome.org/World/Rust/gtk-rust-template)
by Bilal Elmoussaoui, MIT License.

Momentum is licensed under the GNU General Public License v3.0 or later
(see `LICENSE`).

## Super Productivity

Momentum implements the data model and sync protocol of
[Super Productivity](https://github.com/super-productivity/super-productivity),
Copyright (c) 2018 Johannes Millan, released under the MIT License
(`LICENSES/MIT.txt`).

Files that are translations of upstream TypeScript into Rust carry both the
GPL-3.0-or-later SPDX header and an upstream attribution comment naming the
source file. At the time of writing these are:

- `crates/sp-model/src/lib.rs` (entity types, schema constants)
- `crates/sp-oplog/src/lib.rs` (operation envelope, action payloads, reducer semantics)
- `crates/sp-oplog/src/action_codes.rs` (generated action-code table)
- `crates/sp-sync/src/lib.rs` and `crypto.rs` (file format, encryption scheme)

Momentum is not affiliated with or endorsed by the Super Productivity project.

## LibreSync

Sync with nearby devices (`crates/sp-p2p`) links against
[LibreSync](https://github.com/dan-hart/LibreSync), Copyright (c) 2026 Dan Hart,
released under the GNU Affero General Public License v3.0 (`LICENSES/AGPL-3.0-only.txt`).
Its source is not vendored here; builds check it out next to the workspace as
`libresync-src` (see `docs/P2P.md`). Momentum itself remains GPL-3.0-or-later; the
combination is distributed under the terms of both licenses as GPLv3 section 13 allows.
