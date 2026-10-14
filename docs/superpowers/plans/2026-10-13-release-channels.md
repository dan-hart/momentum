# Automatic install channels implementation plan

> Execute inline with focused review checkpoints; use an independent reviewer for
> the plan and final release-plumbing diff.

**Goal:** Every future successful release updates Homebrew and the self-hosted signed
Flatpak repository, with no Flathub submission step.

**Architecture:** Required credential preflight, existing signed Flatpak exports,
a shared checksum-aware Git publisher, and a reusable install-channel verification
workflow. A repository-scoped SSH deploy key authenticates tap updates.

**Tools:** Python standard library, Git, GitHub Actions/CLI, Flatpak/OSTree.

- [x] Add failing regression coverage for credential checks and real temporary-Git
  publication, checksum corruption, dirty/older tap protection, idempotency and errors.
- [x] Implement build-aux/release-channels.py with those bounded responsibilities.
- [x] Update release.yml to require channels, serialize writers, use the helper,
  and remove Flathub submission/archive work.
- [x] Add reusable/manual install-channels.yml and a small CI release-tooling lane.
- [x] Run local behavioral tests, shell/workflow lint, renderer/consumer checks,
  and review the focused diff.
- [x] Create the dedicated Homebrew deploy key and Momentum Actions secret securely;
  verify narrow scope and avoid logging/storing private material in the repository.
- [ ] Push verified plumbing to main and dispatch the shared workflow at 0.4.11.
- [ ] Verify both hosted refs/signatures, SSH update/rerun results and public tap state.
- [ ] Update FEATURES, BUGS, PROGRESS, TESTING and RELEASING with exact evidence,
  push the handoff, and recheck primary preservation and remote parity.

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
