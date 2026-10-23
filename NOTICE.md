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

- `crates/momentum-model/src/lib.rs` (entity types, schema constants)
- `crates/momentum-oplog/src/lib.rs` (operation envelope, action payloads, reducer semantics)
- `crates/momentum-oplog/src/action_codes.rs` (generated action-code table)
- `crates/momentum-sync/src/lib.rs` and `crypto.rs` (file format, encryption scheme)

Momentum is not affiliated with or endorsed by the Super Productivity project.

## Completion sounds

The ten optional task-completion sounds in `data/resources/sounds/`,
`macos/Packages/MomentumKit/Sources/MomentumKit/Resources/` and
`android/app/src/main/res/raw/` are short arrangements of acoustic recordings from
[Versilian Community Sample Library](https://versilian-studios.com/vcsl/) by
Versilian Studios, released under Creative Commons Zero (`LICENSES/CC0-1.0.txt`).
The source recordings are pinned to
[VCSL revision c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e](https://github.com/sgossner/VCSL/tree/c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e).

The arrangements use onset trimming, optional pitch shifting, amplitude envelopes,
layering, mono conversion and gain normalization. Their approved PCM16 mono WAV
masters are bundled unchanged on each platform. Per-recording URLs, source hashes,
arrangement events and approved master hashes are in
[`data/resources/sounds/provenance.json`](data/resources/sounds/provenance.json).
Run `python3 build-aux/check-completion-sounds.py` to verify all ten masters and
platform copies. Credit is not required by CC0; it is given here with thanks.

## LibreSync

Sync with nearby devices (`crates/momentum-p2p`) links against
[LibreSync](https://github.com/dan-hart/LibreSync), Copyright (c) 2026 Dan Hart,
released under the GNU Affero General Public License v3.0 (`LICENSES/AGPL-3.0-only.txt`).
Its source is not vendored here; builds check it out next to the workspace as
`libresync-src` (see `docs/P2P.md`). Momentum itself remains GPL-3.0-or-later; the
combination is distributed under the terms of both licenses as GPLv3 section 13 allows.

### Quiet original sounds

The ten additional quiet choices introduced in the native experience overhaul are
original procedural resonator compositions by Momentum contributors, licensed
GPL-3.0-or-later. They contain no external recordings. Their reproducible generator
is `build-aux/generate-quiet-sounds.py`; levels, note arrangements and master hashes
are recorded in `data/resources/sounds/quiet-provenance.json`. The previous CC0
catalog remains bundled so saved choices continue to work.

## Sparkle (macOS direct-download updates)

The macOS app embeds [Sparkle 2.10.0](https://github.com/sparkle-project/Sparkle),
an MIT-licensed native update framework. Copyright notices and its bundled external
licenses are reproduced in `LICENSES/Sparkle.txt` and the Apple resource
`Sparkle-LICENSE.txt`. Sparkle is not linked into iOS or Android. Updates retain
Momentum's GPL-3.0-or-later licensing and do not require a paid service.
