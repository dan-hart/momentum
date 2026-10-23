# Channel-aware desktop updates

Approved by Dan on 2026-10-23. Scope: native Linux/macOS only, local implementation; no release or remote configuration changes authorized.

Settings gains Updates, manual checks, installed/latest state, optional device-local automatic checks (off by default), progress, actionable failure and explicit install/restart consent. Updates are discoverable through settings and task search. No task or portable settings format changes.

macOS Homebrew installs are positively identified by their installed cask target and use fixed, asynchronous Homebrew commands scoped to Momentum. Direct packaged Release apps use pinned Sparkle 2.10.0 with Ed25519 authenticated updates; missing keys/feed configuration disable replacement. Development/demo builds never update. Linux stable Flatpak uses the native UpdateMonitor portal for its existing signed origin; no host escape or new sandbox permissions. Source/devel installs explain their update route. Automatic checks never silently install or restart. Mobile retains its store-managed paths.

Release tooling signs the exact final zip, emits a versioned appcast, and publishes the validated appcast only after its public release asset exists. Public key continuity, version monotonicity, archive size and signatures are gates. Keys are supplied by private release configuration; implementation does not create remote secrets or publish a feed. Existing Flatpak site contents and Homebrew ownership survive.

Behavioral unit tests cover install channel discrimination, version ordering, fake process failures/cancellation, malformed portal state and feed validation. Native builds and isolated interaction checks supplement units. No tests use personal stores or execute a real update. Adversarial review focuses on ownership, injection, signature trust, stale state, cancellation, lifecycle/restart and release ordering. Unavailable native checks remain explicit.

## Review refinements

Exact ownership must be proven; ambiguous or failed Homebrew probes disable direct replacement. Subprocesses use owned process groups with bounded lifetime/output; installation cannot be cancelled as though it never happened. Homebrew success requires reading the installed app metadata afterward. Relaunch is postponed while native edits or sync are active, with explicit restart consent.

Flatpak UpdateMonitor polls on the host schedule (normally 30 minutes), not on demand. The native Check for Updates action opens the system software app; an explicitly labelled monitor action offers in-app update installation when the portal reports availability. Monitoring describes its continuing lifetime and Stop action. Install/reopen status must not invent a version from an opaque OSTree commit.

Future release signing requires configured private seed/public key. An appcast authenticates its exact archive through Ed25519; metadata alone is not a signed document. Independent verification binds embedded app version/build/key/channel to the archive. Existing public assets are immutable on reruns. Public key continuity forbids implicit rotation; missing/mismatched configuration fails before release. Legacy tags must use their original release workflow. Final hosted feed readback follows public asset verification and Pages publication.
