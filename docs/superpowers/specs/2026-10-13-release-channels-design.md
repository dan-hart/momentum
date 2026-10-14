# Required automatic install channels

The user explicitly requests future releases to update the Homebrew tap and the
existing signed GitHub Pages Flatpak repository automatically, skipping Flathub.
This changes release plumbing only; no version bump or native app rebuild is needed.

## Design

- Require HOMEBREW_TAP_SSH_KEY, FLATPAK_GPG_PRIVATE_KEY and FLATPAK_GPG_KEY_ID
  at release preflight. Report missing names, never credential values. Missing
  configuration fails before builds instead of silently skipping publication.
- Use a new dedicated write-enabled deploy key on dan-hart/homebrew-tap. Store
  its private half only as the Momentum Actions secret. Do not copy the personal
  GitHub token or existing personal SSH identity. Preserve the Flatpak signing key.
- Keep both Flatpak architecture exports serialized, retain site files and signing
  identity, and make their publication a required dependency of the release.
  Serialize release workflows to prevent competing repository writers.
- Remove the Flathub submission job, secret gate and generated submission archive
  from the release workflow. Existing historical assets/tags/PRs remain untouched.
  Flathub runtime/SDK dependencies are still needed to build the native Flatpak.
- Publish Homebrew from final public asset bytes after checksum validation. Reject
  older versions and dirty tap checkouts, preserve unrelated formulae, propagate
  commit/push failures, and treat identical definitions as a successful rerun.
  A write-authentication dry run is required even when the content is unchanged.
- A reusable dispatchable install-channel workflow runs the same Homebrew publisher
  and verifies both live signed hosted refs. Call it from Release, and use current
  0.4.11 assets to validate it without retagging or changing binaries.
- Verification covers missing credentials without disclosure, asset corruption,
  dirty/older tap protection, publication and idempotency using temporary real Git
  repositories, failed commit/push propagation, workflow dependency/permission lint,
  deployed Actions execution, secret/deploy-key scope and public channel readback.

## Boundaries

All source work uses the existing isolated task/native-features-release worktree.
The primary checkout and its 23 inherited changes remain untouched. No native
features, app settings, user data, TestFlight builds or distribution signing change.
The request authorizes the scoped GitHub credential setup and workflow publication.

## Review refinements

- Pin the existing full public signing fingerprint in the repository variable
  FLATPAK_GPG_FINGERPRINT; verify live metadata against that trusted fingerprint
  and require exact requested versions for both stable architecture refs.
- Guard both existing hosted refs against an older release before building/export;
  verify each newly exported version before publishing. Replace the swallowed Git
  commit failure with the tested publisher.
- Release uses global concurrency group momentum-release; the shared Homebrew job
  uses momentum-homebrew-publication. The groups are distinct, preventing nested
  caller/child cancellation while manual and automatic tap writes share a lock.
- Shared workflow inputs: explicit version (blank manual input resolves from source),
  tooling_ref (defaults to current workflow SHA), notarized boolean, and expected
  public fingerprint (repository-variable fallback). Production passes exact version,
  workflow SHA and notarization gate; it passes only the tap SSH secret.
- Check out tooling at workflow SHA separately from tagged build source, so a manual
  0.4.11 validation can use new plumbing while tags/artifacts stay immutable.
- Hosted verification retries at most six times, with 20-second intervals, for
  GitHub Pages propagation. A stale signed ref or unexpected key is never accepted.

The shared hosted job explicitly requests a legacy GitHub Pages build with pages:write,
because GITHUB_TOKEN pushes alone do not trigger that build. It verifies write
authentication and public readback with no private signing material. The export job
checks the imported private key's public fingerprint before any build/export.
