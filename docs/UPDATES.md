# Desktop updates (F-076)

Momentum updates macOS and Linux through the channel that owns the installation.
iOS and Android retain their existing distribution mechanisms. No Momentum account
or paid service is needed. Updates do not change the task, backup, or sync formats.

## Native settings

Updates appears in desktop Settings and settings/global search. Automatic checks
are optional, off by default, and device-local; they are not exported or synced.
Checks never silently install an update or restart the app. Versions, errors and
installation state stay visible. Development/source and demo builds explain their
update route and cannot replace themselves.

macOS exposes Check for Updates in the application menu as well. Exact installed
Momentum cask metadata and resolved bundle path select Homebrew. Uncertain
ownership disables direct replacement. Homebrew refreshes metadata and upgrades
only `dan-hart/tap/momentum`; it cannot upgrade unrelated installed packages or quit
the running app. Success requires both the refreshed cask receipt and installed
bundle version. Checks can be cancelled; installations run to a verified outcome.
A bounded, signed native helper waits for the old process to exit before reopening
the app, preventing overlapping task stores and CLI services. Editing/drafts and
active sync postpone restart.

Direct-download packaged macOS Release builds use pinned Sparkle 2.10.0 and its
native update UI. Missing keys or feed configuration disable replacement. Sparkle
checks Ed25519 archive signatures before extraction; automatic installation and
system profiling are disabled. Explicit user approval is required for installation.
The signed archive feed is https://dan-hart.github.io/momentum/appcast.xml.

Linux stable Flatpak keeps its existing signed repository and uses the Flatpak
UpdateMonitor portal. **Check for Updates opens the system software app** using the
standard appstream URI. A separate **Monitor for Updates** action enables ongoing
in-app monitoring until Stop or application exit; the portal host normally polls
at approximately 30-minute intervals and provides no force-refresh operation.
Repeated monitoring reuses the same monitor rather than resetting its poll.
Opting into automatic checks starts monitoring when Momentum starts. Installation
uses native parented portal consent and reports operation progress. When done,
Momentum explains how to quit (Ctrl+Q) and reopen the updated installation; it does
not start overlapping processes. A Flatpak commit is opaque, so the UI labels it
as a commit rather than inventing a release version. Permission-expanding updates
must use the system software app. No host spawning or extra sandbox permissions
are needed.

## Release configuration and trust

Before the first updater-enabled release, configure a dedicated production Sparkle
Ed25519 identity outside the repository:

- Repository variable `SPARKLE_PUBLIC_KEY`: canonical base64 of the 32-byte public key.
- Repository secret `SPARKLE_PRIVATE_KEY`: canonical base64 of the 32-byte private seed.

Use the official checksum-pinned Sparkle `generate_keys` tooling to generate/export
keys into protected storage. Keep a protected recovery copy. Never commit private
keys or place them in command arguments/logs. The dedicated production identity and matching Actions configuration were created
during the explicitly authorized 0.4.14 release. Private recovery material remains
outside the repository. Without matching configuration, the release workflow fails.

`package-app.sh` marks distributed builds `direct` and embeds the public key through
`MOMENTUM_UPDATE_PUBLIC_KEY`. Local Debug/source builds remain ineligible even when
a public key is supplied. The Homebrew cask ships the same signed app; actual
installation ownership wins over the direct marker.

The release workflow downloads Sparkle 2.10.0 signing tools with pinned SHA-256,
checks a matching key pair, signs the exact post-signing/post-notarization ZIP,
and independently verifies the signature with CryptoKit. It binds the embedded
version, build, direct channel and public key to the appcast. The XML metadata is
served over HTTPS; its archive enclosure carries the cryptographic signature.
No claim is made that the XML itself is signed.

The public release assets include `appcast.xml` and `sparkle-public-key.txt`.
Existing asset bytes are immutable: conflicting rebuilds fail before GitHub uploads,
including partial-run retries. Legacy pre-updater tags must use their original
release workflow. After the public ZIP is downloaded and independently authenticated,
publication checks version/build monotonicity and existing public-key continuity,
then updates only the two feed files on gh-pages. Existing Flatpak/site files survive.
Pages deployment and exact public feed readback are required before the install-channel
completion gate. Implicit signing-key rotation is refused; any future rotation
requires a separately reviewed continuity plan.

## Verification boundaries

Unit tests use disposable stores, fake package-manager commands, a public RFC 8032
key vector, and isolated archives. They cover ownership ambiguity, literal arguments,
bounded subprocess trees, stale receipts, restart eligibility, portal state/cancel
semantics, archive/key/version tampering, immutable retries and workflow ordering.
They never update a personal installation. Native builds and existing Linux harness
supplement these tests. Real signed upgrade/relaunch, system software routing and
portal consent require controlled installation acceptance before a release is
claimed verified; a build or pure policy test does not establish those interactions.

Production publication evidence (2026-10-23): v0.4.14 / 18d9f8c publishes the
first authenticated updater archive and HTTPS feed. Independent readback verified
the production public key, archive signature/metadata and exact hosted appcast
bytes. This does not establish native installation/relaunch interaction acceptance.
