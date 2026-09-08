# Flathub submission files

- `io.github.dan_hart.Momentum.json`: offline manifest. Replace `REPLACE_WITH_TAG_COMMIT`
  with the commit hash of the release tag before submitting.
- `cargo-sources.json`: vendored crate sources for that tag's `Cargo.lock`. Regenerate
  after any dependency change:

  ```sh
  python3 -m venv .venv && .venv/bin/pip install aiohttp toml tomlkit
  curl -fsSLO https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
  .venv/bin/python flatpak-cargo-generator.py Cargo.lock -o build-aux/flathub/cargo-sources.json
  ```

Lint before submitting:

```sh
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest build-aux/flathub/io.github.dan_hart.Momentum.json
flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream data/io.github.dan_hart.Momentum.metainfo.xml.in.in
```
