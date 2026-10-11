# Android structured automation proposal — F-031

Date: 2026-10-10. Source: dirty `task/android-native` worktree. This is a proposal
for approval, not an implemented public service or a claim of assistant discovery.

## Current authorized slice

The existing exported Activity accepts task URLs and Android share/search Intents.
Native launcher shortcuts offer Add Task, Search and Today. The localized help
documents title, notes, ISO due day, tags, query and stable task IDs. Add/Create
opens a private draft and preserves an active editor through the durable capture
queue. Open resolves the current task; Find opens in-app search; Complete retains
the desktop-compatible first exact case-insensitive unfinished title match.
Reopen now uses the shared idempotent batch rule and retains Undo; Plan uses Rust.
Task reads/mutations require an unlocked device. These entry points do not return
structured task entities or changed counts to the caller.

iOS already offers Create, Find, Set Completed/Reopen, Plan Today and Open through
authenticated App Intents. Sources:
`macos/Momentum/Services/CreateTaskIntent.swift`, `TaskIntents.swift`,
`TaskEntities.swift`, and the shared automation runtime. Android should obtain
equivalent capability with native Android APIs and the same Rust task rules.

## Recommended first public contract

Approve a framework-only Android App Functions adapter, initially available while
Momentum is visible and unlocked on API 37+. Add one explicit setting, **Allow
native automation**, off by default. Its confirmation must explain that trusted
Android automation agents can read unfinished task titles and selected task
metadata and invoke the approved task actions. Turning it off unregisters every
function and denies admitted callbacks before reading, changing or returning data.
No proprietary assistant SDK, GMS dependency, account, network service or task
storage migration is needed.

The proposed first version returns IDs, titles, completed state and planned day.
It excludes notes, private sync fields, credentials, device IDs, server addresses,
raw files and file paths. A separate optional **Allow automation to read notes**
setting is the alternative requiring an explicit exposure decision; keeping notes
readout unavailable is the recommended initial policy. Creation can accept notes
provided by the caller without reading existing notes back.

| Function | Typed input | Result and rule |
|---|---|---|
| Create Task | title, optional notes, planning enum, optional project ID and ISO day | Exact created ID and title from `createTaskWithId`; preserve Rust parsing, date/project rules and save errors. |
| Find Tasks | supported list enum, title substring, include-completed boolean, bounded result limit | Current non-archived task entities, stable IDs and explicit `hasMore`; no hidden full-content return. |
| Set Completed | stable ID array and boolean | Complete/reopen through existing shared batch rules; preserve archive policy and Undo. |
| Plan Today | stable ID array | Existing Rust planning rule and Undo. |
| Open Task | one stable ID | Resolve against current core state, show/queue the editor without replacing an active draft; stale IDs return a typed error. |

Initial transport bounds proposed for review: at most 100 IDs per request; each ID
1–128 ASCII letters/digits/underscore/hyphen; title/query at most 4,096 characters;
caller-supplied notes at most 64 KiB of UTF-8; at most 100 returned entities
and 256 KiB encoded request/response. Existing tasks with longer notes remain
valid and task-link draft entry keeps its existing larger bound. Oversized input/response receives an explicit typed
error; task content is never silently truncated. These are transport bounds, not
new task business rules or a restriction on existing saved task contents.

The current core exposes exact creation identity, queries, batch completion,
reopen and planning. Do not infer an exact changed count with separate before/after
FFI calls: concurrent sync can interleave. If the existing outcome cannot express
the required count, propose a small atomic Rust result API before implementing
that result. Mutations return success only after durable commit. A cancelled
request never reports a fabricated failure or success after an uncertain commit.
Creation retries cannot promise exactly-once behavior without a durable request
identity policy; version one documents that uncertainty and does not automatically
repeat creation. A durable deduplication receipt would need a separate design.

## Security and lifecycle

Use the existing application-owned repository; perform reads/FFI work off the main
thread. Gate opt-in, `KeyguardManager.isDeviceLocked`, restore admission and scope
at entry and immediately before core mutation or response disclosure. Deny locked
calls with an actionable error; do not expose task content through a lock-screen
Activity, device-protected storage, notification or raw error details. Do not
automatically dismiss secure keyguard. Never log request bodies, titles or IDs.

Activity-scoped registrations exist only while the foreground Activity is active;
unregister on stop, opt-out and before restore, drain admitted operations and
re-register only after the explicit restore owner resumes. Use cancellation and
one completion callback per request. No permanent foreground service is proposed.
The first scope cannot perform a cold background request; existing task links
still provide cold launch into Momentum for user review.

Cold structured automation would require one exported `AppFunctionService`
protected by the framework's `BIND_APP_FUNCTION_SERVICE` permission and disabled
by default on unsupported OS versions. That is an additional service/public
security boundary, requiring explicit approval rather than an incidental fallback.
It must apply the same off-default, unlocked, current-ID and restore rules. A
caller package extra or referrer is not authorization; the framework service's
caller identity is not a stable package allowlist on Android 17.

## API availability and verified limits

[Android's App Functions overview](https://developer.android.com/reference/android/app/appfunctions/package-summary)
describes a beta/experimental feature for trusted, system-privileged agents. The
framework begins at API 36; API 37 adds the direct XML metadata, discovery and
runtime registration APIs suitable for the no-new-dependency foreground option.
Metadata must describe every function; Activity scope identifies the registered
Activity and requires the caller to select it. Foreground availability therefore
does not imply cold assistant invocation.

The installed `android-37.0/android.jar` was inspected with `javap`: it exposes
`AppFunctionManager.registerAppFunction`, function search/state, execution,
`AppFunctionMetadata` scope and `AppFunctionService`. No new build/dependency was
added. The public docs also include later version 37.2 access levels and runtime
user-control properties; they are absent from this installed 37.0 SDK. A concrete
implementation must use only the actual SDK APIs and recheck OS/runtime support.
See [metadata access levels](https://developer.android.com/reference/android/app/appfunctions/AppFunctionMetadata)
and [runtime user control](https://developer.android.com/reference/android/app/appfunctions/AppFunctionManager).

[The service documentation](https://developer.android.com/reference/android/app/appfunctions/AppFunctionService)
requires heavy work off its main-thread callback, cancellation handling and one
result/error callback. On Android 17 its calling package is empty and signing
information is unknown, so a custom per-package allowlist cannot be assumed.
[The execution permission](https://developer.android.com/reference/android/Manifest.permission#EXECUTE_APP_FUNCTIONS)
also has platform allowlisting requirements. A successful self-call does not prove
an ordinary third-party automation app or vendor assistant can discover/call it.

Actual system/OEM assistant discovery and locked invocation remain unverified.
Do not ship an unprotected broadcast, ContentProvider, result-Activity RPC or
proprietary App Actions integration to bypass missing framework availability.
Pre-37 devices retain the localized existing task links and launcher shortcuts,
with an explicit reason that structured App Functions are unavailable.

## Acceptance and approval

After approval, verify metadata and self-execution against isolated stores, denial
for unsupported/disabled/locked/restoring cases, bounded typed results, cancellation,
save failure, current deleted/completed/archived IDs, concurrent sync, and editor
conflicts. Separately verify actual authorized external-agent discovery, cold
service invocation if approved, and real OS authentication on disposable devices.
Do not label closure-injected lock tests as system lock-screen acceptance.

Requested decision: approve the off-default, title/ID-only, API37 foreground App
Functions contract above; keep note readout and cold background service deferred.
An approval should also choose whether exact mutation counts must be added to the
shared result API before exposing Set Completed/Plan, or whether version one may
return truthful changed/unchanged outcomes. No public function adapter has been
implemented as part of the current native entry-point/count slice.
