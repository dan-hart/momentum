<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Releasing

One version, one tag, one workflow. Every app and the command line carry the same
version, a release is a `vX.Y.Z` tag, and the Release workflow does the rest: it runs the
whole test suite against the tag, builds every binary, attaches them to a GitHub Release,
updates the Homebrew tap, publishes the signed Flatpak repository, and opens the Flathub
update.

## One version

`Cargo.toml`'s workspace version is the source of truth. Four other files carry a copy,
and `build-aux/release.py` keeps them equal:

| File | Copy |
|---|---|
| `meson.build` | `project(version:)`, which becomes the GTK app's `--version` and metainfo |
| `macos/project.yml` | `MARKETING_VERSION`, plus `CURRENT_PROJECT_VERSION` derived as `major*10000 + minor*100 + patch` so `CFBundleVersion` only ever goes up |
| `data/…metainfo.xml.in.in` | the newest `<release>` and the screenshot URLs, which point at the tag |
| `CHANGELOG.md` | the newest `## [X.Y.Z]` section |

```sh
build-aux/release.py version    # print it
build-aux/release.py check      # every copy agrees; CI runs this on every push
```

CI's *Versions aligned* job fails a pull request that changes one copy without the
others, and the Release workflow refuses a tag that does not equal `v<version>`.

## Cut a release

1. Make sure `CHANGELOG.md`'s `[Unreleased]` section says what changed. Its bullets
   become the metainfo `<release>` entry and the GitHub Release notes.
2. Bump, review, commit and tag:

   ```sh
   build-aux/release.py bump 0.4.0 --summary "One sentence for the app stores."
   git diff                         # Cargo.toml, Cargo.lock, meson.build, macos/project.yml, CHANGELOG.md, metainfo
   git commit -am "Momentum 0.4.0"
   git tag -a v0.4.0 -m "Momentum 0.4.0"
   git push origin main v0.4.0
   ```

   `--summary` is optional. `--date` overrides today's date. The command refuses a
   version that is not newer, and an empty `[Unreleased]` section unless you pass
   `--allow-empty`.
3. Watch the **Release** workflow. It can also be started by hand from the Actions tab
   with an existing tag, which reruns everything for that tag.

Before tagging, run on a real Mac what CI cannot: the opt-in nearby-sync test in
[TESTING.md](TESTING.md).

## What the workflow does

| Job | What | Runs when |
|---|---|---|
| Verify | tag equals every version copy; release notes from the changelog | always |
| CI, macOS | the complete `ci.yml` and `macos.yml` suites against the tag: rustfmt, core tests, translations, the Flatpak build with the headless GTK UI tests on both architectures, the macOS unit tests, app build and localization checks | always |
| Flatpak bundle | `momentum-vX.Y.Z-{x86_64,aarch64}.flatpak` from the release manifest, tests included | tests passed |
| macOS app and mo | `Momentum-vX.Y.Z-macos.zip` (universal, `mo` inside `Contents/MacOS`) and `mo-vX.Y.Z-macos-universal.tar.gz`, via `macos/scripts/package-app.sh` | tests passed |
| mo for Linux | `mo-vX.Y.Z-linux-{x86_64,aarch64}.tar.gz`, built on Ubuntu 22.04 so the binary loads on Homebrew's glibc and on any desktop distribution of the last few years | tests passed |
| Publish Flatpak repository | signed OSTree commits for both architectures on `gh-pages`, the repository behind `flatpak install --user https://dan-hart.github.io/momentum/momentum.flatpakref` | tests passed and the signing key is configured |
| GitHub Release | the release named *Momentum X.Y.Z* with every file above, `SHA256SUMS.txt`, and `flathub-vX.Y.Z.tar.gz` (the Flathub manifest pinned to the tag plus regenerated `cargo-sources.json`) | every build succeeded |
| Update Homebrew tap | writes `Formula/momentum-cli.rb` and `Casks/momentum.rb` in `dan-hart/homebrew-tap` from `build-aux/homebrew/*.rb.in` with the assets' digests | the tap token is configured |
| Open the Flathub update | pushes a branch and pull request to `flathub/io.github.dan_hart.Momentum` with the pinned manifest | the Flathub token is configured and that repository exists |

A release with a missing optional secret still completes: the optional job is skipped
and the release stays reproducible by rerunning the workflow once the secret is set.

## Install channels after a release

```sh
brew install --cask dan-hart/tap/momentum     # macOS app, mo on PATH
brew install dan-hart/tap/momentum-cli        # mo alone, macOS or Linux
flatpak install --user https://dan-hart.github.io/momentum/momentum.flatpakref
```

The cask and the formula both provide `mo`, so install one or the other; the cask is the
one for people who want the app.

## Secrets

| Secret | Used by | Without it |
|---|---|---|
| `FLATPAK_GPG_PRIVATE_KEY`, `FLATPAK_GPG_KEY_ID` | Publish Flatpak repository | the repository on `gh-pages` is not updated; bundles are still attached to the release |
| `HOMEBREW_TAP_TOKEN` | Update Homebrew tap | the tap is not updated. A fine-grained personal access token with *Contents: read and write* on `dan-hart/homebrew-tap` |
| `MACOS_CERTIFICATE_P12`, `MACOS_CERTIFICATE_PASSWORD` | macOS app and mo | the app and `mo` are signed ad hoc, and the cask tells users to install with `--no-quarantine`. The P12 is the *Developer ID Application* certificate with its private key, base64-encoded |
| `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD` | macOS app and mo | the app is not notarized. The password is an app-specific password for the Apple ID; the team ID is also needed for Developer ID signing |
| `FLATHUB_TOKEN` | Open the Flathub update | no pull request is opened; the manifest is still attached to the release. A token that can push branches and open pull requests on the Flathub app repository |
| `LIBRESYNC_TOKEN` | every LibreSync checkout | falls back to the workflow token, which is enough while LibreSync is public |

Keep an offline backup of the Flatpak signing key: a lost key means a new repository for
every user.

## Flathub

Momentum is not on Flathub yet. The first submission is by hand, with the files from the
release's `flathub-vX.Y.Z.tar.gz`: the manifest pins the tag's commit and the LibreSync
tag it was built with, and `cargo-sources.json` vendors every crate for an offline build.
Once `flathub/io.github.dan_hart.Momentum` exists and `FLATHUB_TOKEN` is set, every later
release opens its update pull request automatically. Lint before submitting:

```sh
flatpak run --command=flatpak-builder-lint org.flatpak.Builder manifest io.github.dan_hart.Momentum.json
flatpak run --command=flatpak-builder-lint org.flatpak.Builder appstream data/io.github.dan_hart.Momentum.metainfo.xml.in.in
```

## Building a release locally

The same script CI uses, producing the same files in `dist/`:

```sh
macos/scripts/package-app.sh dist            # ad hoc signed unless MACOS_SIGN_IDENTITY is set
REQUIRE_UNIVERSAL=1 macos/scripts/package-app.sh dist    # fail unless both architectures are in
```

For a universal binary, `rustup target add x86_64-apple-darwin` first. Flatpak bundles
come from the release manifest, as in the README's *Build from source*.
