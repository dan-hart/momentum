<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Sync with nearby devices

Momentum can sync directly between your devices on the same network, without a server,
using [LibreSync](https://github.com/dan-hart/LibreSync) as the transport. Choose LibreSync or Nextcloud as the active provider; they do not run together.
Switching providers retains tasks, pending changes and saved connections. Changes
received through LibreSync can be uploaded later if you switch to Nextcloud.

## Using it

1. Preferences/Settings → Sync → **LibreSync** on both devices.
2. **Manage Devices…** on both. Each shows a six-digit pairing code and lists the other
   under Nearby. On one of them press **Link…** and type the code shown on the other.
3. Done. Changes travel within a few seconds of being made, and every five minutes as a
   catch-up. **Sync Now** (Ctrl+R) also exchanges with nearby devices.

The code is valid for five minutes while the dialog is open. Linking pins the other
device's certificate; if it ever changes (reinstall, new keys) Momentum tells you and you
unlink and link again. Unlink from the same dialog.

## What travels

- **Operations, not files.** Every change is one of Momentum's operations (the same ops
  the Nextcloud sync uploads). Each becomes one immutable LibreSync record keyed by the
  op id, so records never conflict and last-writer-wins on the transport loses nothing.
  The receiving device applies the typed action through `sp_oplog::apply`, exactly like a
  local change, and queues it for a future Nextcloud upload if that provider is selected later,
  unless the server already has that op id.
- **A bootstrap snapshot.** Each device publishes a gzip-compressed snapshot of its whole
  state (at most once a minute, only when it changed). A device syncing for the first time
  adopts the newest snapshot if it has no tasks of its own, otherwise merges in the
  entities it does not have; after that only ops are applied.
- **Tombstones after 30 days.** A device tombstones its own ops older than `KEEP_DAYS`,
  so the shared state stays bounded; later joiners rely on the snapshot.

Everything is end-to-end encrypted by LibreSync (XChaCha20-Poly1305 with a per-app key
exchanged at link time, TLS with pinned self-signed certificates on the wire). The app
key and device certificate live in the keyring beside the Nextcloud password; the engine
state (`state.bin`), op journal and device list live in `p2p/` under the data directory.

## Network

- TCP on port 52345 (any free port if that one is taken, for example a Devel build next to
  the release) and multicast UDP 5353 for mDNS discovery. Fedora Workstation's default
  firewall zone allows both; on Fedora Server or with ufw open them as described in
  LibreSync's `docs/PACKAGING-LINUX.md`.
- Only devices with the same app id (`io.github.dan_hart.Momentum`, shared by Devel and
  release builds) are discovered.
  This is a cross-platform protocol identifier, independent of the macOS bundle ID
  `com.codedbydan.Momentum`.
- A linked device that does not answer discovery is still tried at its last known address,
  which covers a Tailscale or other overlay network once it has been linked on the LAN.

## Building

LibreSync is AGPL-3.0-only and not on crates.io, so the crate is a path dependency on a
checkout next to the workspace at `libresync-src` (a git dependency would also work now
that the repository is public; the checkout keeps one pinned copy for cargo, meson and
flatpak-builder alike):

- locally: `ln -s ../LibreSync libresync-src` (the directory is gitignored);
- in CI: `actions/checkout` of `dan-hart/LibreSync` at the pinned tag into
  `libresync-src` (a `LIBRESYNC_TOKEN` secret is only needed if that repository were
  private; it is public);
- in the Flatpak manifests: a second `dir` source copies `../libresync-src` into the build.

`crates/sp-p2p` is the only crate that talks to LibreSync; the app sees `P2p`, `Event`,
`LinkedDevice` and `OpRecord`. `cargo test -p sp-p2p` runs two nodes on localhost: wrong
code refused, link, exchange both ways, bootstrap a third device, unlink.

For scripted testing, `MOMENTUM_P2P_CODE=123456` (Devel builds only) opens the pairing
window with that code at startup, and `cargo run -p sp-p2p --example peer` is a minimal
command-line device: `peer DIR link ADDR CODE`, `peer DIR push "title"`,
`peer DIR sync`, `peer DIR inbox`.
