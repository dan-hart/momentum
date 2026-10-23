# Apple build environment isolation (B-178)

## Cause

An unrelated API credential was globally exported by the interactive shell and
inherited by the build process. Xcode's script/scheme logging printed inherited
environment variables. This was a build-tool environment exposure, not an API
integration or use of the credential by Momentum. No credential value belongs in
this document, the source tree, commands or regression fixtures.

## Guarded entry points

Use the allowlisted launcher for command-line Apple builds:

```sh
python3 macos/scripts/build-env.py xcodebuild -project macos/Momentum.xcodeproj -scheme Momentum build
python3 macos/scripts/build-env.py swift test --package-path macos/Packages/MomentumKit --build-system swiftbuild
```

The macOS packaging script, public shared core builder, native iOS test runner,
README commands and macOS CI build use the same boundary. The core builder always
sanitizes before locking; an inherited lock marker cannot bypass it. Its internal
implementation is separated only to avoid recursive shell reentry and keep the
lock owned for the complete build.

Only explicitly reviewed toolchain/user-path/locale/public build settings and the
explicit nearby-test opt-in survive.
Unknown environment variables are dropped rather than relying on a provider-name
or secret-value blacklist. Signing identity/team stay explicit Xcode arguments;
private signing/notarization credentials remain with the separate operation that
needs them. The launcher never evaluates a shell, prints the environment or formats
failed command arguments into diagnostics.

For xcodebuild the launcher also adds `-hideShellScriptEnvironment`. All generated
macOS/iOS target script phases set `showEnvVars: false`. Packaging and iOS runner
text logs are restricted to owner read/write access.

## Explicit boundary

Raw, unwrapped `xcodebuild` and Xcode launched from a credential-bearing environment
can still log scheme pre-action environments before any repository script runs.
XcodeGen’s scheme execution actions do not expose the target-phase logging control.
Target-phase settings alone do not solve that case. Use the guarded CLI commands,
or start Xcode from a clean desktop environment. The launcher cannot modify a
parent process or retroactively remove a credential from an already-running shell.
Global shell authentication was left unchanged to preserve other tools.

Existing exposed credentials must still be rotated independently. Redacting private
logs does not revoke a credential or erase its earlier appearance in a chat/tool
transcript. No provider credentials are required by Momentum's builds.

## Verification

```sh
python3 -m unittest discover -s macos/scripts/tests -p test_build_environment.py -v
python3 -m unittest discover -s ios/scripts/tests -p test_runner.py -v
```

Fixtures use harmless sentinels. Tests exercise absent/future credential variables,
required toolchain paths, literal argv, exit-code preservation, generated script
policy, inherited lock-marker bypass, iOS runner process isolation and private logs.
A real canonical signed Debug build separately checks that neither the inherited
credential nor a sentinel reaches its protected log; only booleans are reported.
A throwaway Xcode project confirms the documented raw-preaction limitation without
putting a real credential into that experiment.
