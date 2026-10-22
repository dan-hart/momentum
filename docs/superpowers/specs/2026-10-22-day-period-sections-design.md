# Morning, Today and Evening sections

Status: approved by the user on 2026-10-22; implemented locally; platform verification scope is recorded in PROGRESS.

## Scope

Add the same capability to Linux, macOS, iOS and Android using the shared Rust
engine and native controls. Keep changes local; no commit, push or release.

## Presentation

In Today with Morning & Night grouping, retain the existing Morning → Today →
Evening order. Show only populated groups. Add a native horizontal separator at
each boundary between populated day-period groups: above Today when Morning is
present, and above Evening when an earlier group is present. If only Morning and
Evening exist, show one separator between them. A lone group has no separator.
Do not insert an empty Today group or leading/trailing lines.

Keep visible localized headings, existing task ordering, parent/subtask families,
selection and the separate Completed section. Decorative separators are hidden
from screen readers. Other grouping modes and other lists keep their presentation.

## Preference

Add a native setting near Group By: **Show Morning and Evening tasks in Today**,
enabled by default. Turning it off removes today's Morning/Evening task families
from Today regardless of Group By, including their completed rows. Their existing
Morning/Evening sidebar or mobile list-picker destinations remain available.
Search, project/tag lists and stored tasks are unaffected. Overdue tasks remain
visible in Today even when they have a Morning/Evening tag, since the slot
destinations only include tasks planned for today.

The setting persists locally per device. Do not add it to settings transfer or
optional settings sync in this change. Today selection and empty/all-done states
must reflect the visible listing; task-wide summaries and existing navigation
counts retain their established task-membership semantics.

## Implementation boundaries

Rust owns the Today filtering decision and section-boundary policy, exposed through
the existing generated UniFFI tooling. Native clients own divider rendering,
localized English/German wording, settings controls and local persistence. No
task-format changes, migration, dependencies or services are needed.

## Verification and records

Core regressions cover all populated-group combinations, grouping-independent
visibility, completed tasks, parent/subtask families, overdue retention, default
behavior and unaffected alternative destinations. Native preference tests cover
persistence and refreshed listings; native presentation checks cover conditional
dividers, readable headings and scalable text.

Run applicable Rust workspace/CLI/FFI checks, Apple package and app builds, Android
checks and GTK compilation/runtime checks where available. Record platform-specific
limits explicitly. Update F-037 plus a new feature ID, BUGS assessment, PROGRESS,
GROUPING and MVP alongside implementation. No platform is marked Verified from
shared-core tests alone.

## Alternatives considered

- Recommended: conditional native dividers and one device-local visibility toggle.
  This follows the requested layout with minimal new configuration.
- Separate Morning and Evening toggles offer finer control but add settings beyond
  the requested combined choice.
- Card backgrounds or spacing alone provide separation but do not supply the
  requested horizontal lines.
