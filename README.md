# Momentum

A native GNOME task planner and time tracker that reads, writes and syncs the
same data as [Super Productivity](https://super-productivity.com).

Status: project scaffold. See [PLAN.md](../PLAN.md) for the roadmap.

## Layout

| Path | What |
|---|---|
| `crates/app` | GTK 4 + libadwaita application (binary `momentum`) |
| `crates/sp-model` | Super Productivity data model, schema v4 |
| `crates/sp-oplog` | Operation log: envelope, vector clocks, codec, crypto, apply() |
| `crates/sp-store` | SQLite persistence for the op log |
| `crates/sp-sync` | LocalFile, WebDAV/Nextcloud and SuperSync providers |
| `data/` | Desktop file, metainfo, GSettings schema, icons, Blueprint UI |
| `build-aux/` | Flatpak manifest and helper scripts |

## Building

Requires the GNOME 50 SDK, the Rust SDK extension and flatpak-builder:

```sh
flatpak install --user flathub org.gnome.Sdk//50 org.gnome.Platform//50 \
  org.freedesktop.Sdk.Extension.rust-stable//25.08 \
  org.freedesktop.Sdk.Extension.llvm22//25.08 org.flatpak.Builder
flatpak run org.flatpak.Builder --user --install --force-clean flatpak_app \
  build-aux/io.github.danhart.Momentum.Devel.json
flatpak run io.github.danhart.Momentum.Devel
```

Or open the folder in GNOME Builder and press Run.

## Renaming

The name, app ID and author are placeholders. Run
`python3 build-aux/rename.py` to change them across the tree.

## License

GPL-3.0-or-later. See `LICENSE` and `NOTICE.md`.
