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
