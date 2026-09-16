# macOS CLI and Shortcuts verification — 2026-09-16

The handoff and `docs/MVP.md` remain the overall MVP acceptance baseline. This
records the CLI/automation addition, not a declaration that the whole MVP is complete.

## Implemented

- All existing `mo` commands are portable on macOS. Shared short-syntax parsing,
  accumulated duration estimates, strict dates, timed upcoming queries, safe unique
  identifiers, machine-readable success/error output, and data-directory precedence
  are covered by regression tests.
- Connected IPC rejection, timeout, disconnect or persistence failure returns an
  error without writing around the running app. Persistence errors explicitly warn
  that a change may already be present in memory; blind retries are unsafe.
- Live CLI completion uses the same auto-archive/reopen logic as the app. Offline
  operation cannot infer the GUI's local preferences. Shared connection settings
  import on launch and after CLI notification; secrets remain in the Keychain.
- The app refreshes views, badge, search indexing and sync scheduling after CLI changes.
- Every Mac build bundles and signs `mo`. The installer creates a non-overwriting
  link in `~/.local/bin`, following subsequent updates to that app bundle.
- Five native App Intents, task/project entities, and English/German App Shortcut
  phrases support Create, Find, Set Completed, Plan for Today, and Open Task.
- Creation returns its own ID from inside the engine lock. Batch reopening is one
  undoable operation and never auto-archives. Upcoming queries can include completed
  tasks. Planning a timed task clears its prior time/reminder.

## Verification

- 21 CLI integration tests passed, including a real running engine, auto-archive of
  parents/children with undo, live disk failures, rejected requests and timeout handling.
- 60 shared core tests passed; FFI feature compilation and Rust formatting passed.
- Swift package run: 117 tests reported successful in 26 suites, including the one
  opt-in Nearby network test skipped. Automation and preference regressions passed.
- Xcode Debug build, App Intents metadata extraction, App Shortcut phrase validation,
  and deep/strict code-signature checks passed. Five actions and two entity types
  are present in the metadata. Native registration was verified with the user's
  personal Apple Development identity; no signing team is saved in project defaults.
- English/German resources: 276 app, 148 package, 2 permission and 5 App Shortcut
  entries; no missing German translations or placeholder mismatches.
- Native Shortcuts.app discovered all five actions. Create → Open created a sample
  task with parsed tag/estimate and displayed it in Momentum. Find → Plan changed a
  CLI-created tomorrow task to today. Find → Set Completed returned 2 for two sample
  tasks. The temporary shortcut was removed after testing.
- The final personally signed app also passed installed `mo add` → `mo done` over
  the live socket, with archive membership verified from the demo store.
- The bundled CLI was exercised against an isolated closed-app store for add,
  upcoming, plan, today, completion/reopening, search/filtering, tags/projects,
  config and deletion. Installer idempotence and the PATH installation were checked.
- A live smoke test exposed Xcode's dependency ordering: its app target pre-build
  script ran after the Swift package had copied the old static library. A scheme
  pre-action now prepares Rust before package compilation; the target phase retains
  failure propagation. This prevents a newer CLI being bundled beside an older core.

## Limits

External Nextcloud authentication/transfers and Linux D-Bus were not exercised on this
Mac. Keychain service/account compatibility was source-checked. Native Shortcuts runs
used a running demo app; cold-launch/Siri/Spotlight invocation remain separate checks.
The `shortcuts run` command-line invocation did not return during the test and was
cancelled; the Shortcuts.app executions above succeeded. Existing old development
copies with the former bundle ID can expose an additional old Create Task action.
Overall MVP acceptance still requires the outstanding handoff checks.
