<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Copyright (C) 2026 Dan Hart -->
# Linux automation

Momentum uses ordinary Linux interfaces rather than a separate automation service:
the `mo` command, GApplication actions, registered URL schemes, the GNOME Shell search
provider, KRunner, and launcher actions. They all use the same local task store and Rust
task rules as the native app.

## Capability and test audit

The audit found one gap—revealing one exact current task—and added `mo open`,
`app.open-task`, and `momentum://open-task`. Every other mapped capability already had
behavioral coverage, so no duplicate tests were added for it.

| Capability | Linux surface | Behavioral regression evidence |
|---|---|---|
| Create | `mo add`, `momentum --add`, `momentum://add`, launcher New Task | `add_parses_tags_estimate_and_scheduling_flags`; `quick_add_window_creates_a_task_for_today`; `paste_and_drop_text_become_tasks` |
| Find/list | `mo today`, `morning`, `tonight`, `upcoming`, `list`, and `search --json` | `projects_tags_search_and_filters`; `upcoming_includes_scheduled_tasks_using_their_local_day` |
| Complete | `mo done` and notification Done | `done_undone_plan_and_rm_by_prefix_or_title`; `live_done_honors_auto_archive_and_undo_for_parent_and_children`; `reminders_fire_once_for_due_tasks` |
| Reopen | `mo undone` | `done_undone_plan_and_rm_by_prefix_or_title`; `live_done_honors_auto_archive_and_undo_for_parent_and_children` |
| Plan for today | `mo plan` | `done_undone_plan_and_rm_by_prefix_or_title`; `day_moves_tomorrow_next_week_tonight_and_plan_today` |
| Sync | `mo sync` | `live_engine_receives_one_tag_and_json_sync_acknowledgment`; `sync_respects_selected_provider_without_falling_back_to_nextcloud`; `live_rejection_and_timeout_never_fall_back_to_direct_write` |
| Search | GNOME Shell, KRunner, Search view, and launcher Search | `search_provider_matches_terms_and_offers_creation`; `search_finds_tasks_notes_projects_and_tags_after_the_debounce` |
| Quick Add | global shortcut, `momentum --quick-add`, and launcher New Task | `quick_add_window_creates_a_task_for_today`; `quick_add_parses_tags_and_estimate_and_plans_for_the_current_view` |
| Open/reveal | `mo open TASK`, `app.open-task('s')`, `momentum://open-task?id=…` | `accepted_open_has_exact_json_and_does_not_mutate_the_store`; `open_refuses_custom_data_dirs_without_writing`; `private_bus_serializes_warm_then_cold_gapplication_open`; `coexisting_profiles_receive_only_their_own_store_request`; `private_bus_nonreply_times_out_without_writing`; `exact_task_reveal_uses_its_project_focuses_the_stable_id_and_opens_the_editor`; `exact_task_reveal_falls_back_to_title_search_but_focuses_the_stable_id`; `open_task_action_and_uri_are_parameterized_and_invalid_requests_are_non_mutating` |

## Command examples

```sh
mo add "Call the bank #admin 15m" --today
mo today --json
mo search bank --json
mo done bank
mo undone bank
mo plan bank
mo sync --json
mo open bank
mo --json open 01a0835a

gio open 'momentum://add?title=Call%20the%20bank'
gio open 'momentum://open-task?id=01a0835a'
gdbus call --session \
  --dest io.github.dan_hart.Momentum \
  --object-path /io/github/dan_hart/Momentum \
  --method org.gtk.Actions.Activate open-task "[<'01a0835a'>]" '{}'
```

`TASK` follows the existing CLI lookup rules: an exact stable ID wins, otherwise a
unique case-sensitive ID prefix or unique case-insensitive title fragment is accepted.
Ambiguous, missing, archived, deleted, and stale tasks fail without opening or changing
anything. Exact opening prefers the task's Project view. If an imported task's project
is unavailable, Momentum searches its title and still selects the row by stable ID
before presenting the editor.

## JSON contract

Pass global `--json` before or after the subcommand. Successful exact opening is one
compact JSON object on standard output:

```json
{"status":"requested","task":{"id":"01a0835a","title":"Call the bank"}}
```

`task` is the complete standard task JSON, not the abbreviated example above.
`requested` means that the desktop accepted the action; it does not claim that a person
has already viewed the editor. Failures exit nonzero, write nothing to standard output,
and place the same human-readable error inside a JSON envelope on standard error:

```json
{"error":"no task matches “missing”"}
```

Other commands retain their established contracts: task listings are JSON arrays;
mutations return a status plus the standard task object; sync returns `requested` for a
running app or `synced` with transfer counts for a completed direct Nextcloud exchange.
No JSON response includes credentials.

## Offline and running-app behavior

- Listing, finding, and searching read the durable store and work offline.
- With no app running, task mutations use the established local-store path and enqueue
  sync operations. Direct Nextcloud sync works offline from the app process when its
  configured server is reachable; LibreSync requires the app process.
- `mo open` is deliberately not a store operation. With no owner it asks D-Bus to
  activate the registered GApplication, then sends `org.gtk.Actions.Activate` only
  after activation succeeds. It never appends an oplog action or writes the store.
- The resolved store selects exactly one GApplication: the stable Flatpak store targets
  `io.github.dan_hart.Momentum`, and the Devel Flatpak store targets
  `io.github.dan_hart.Momentum.Devel`. `MO_DATA_DIR`, `--data-dir`, and native/custom
  stores remain valid for other `mo` commands, but `mo open` refuses them because no
  safe one-to-one application profile mapping exists.
- With Momentum running, mutations use the local socket first and Linux D-Bus during
  startup. Once a live endpoint accepts a connection, rejection, timeout,
  disconnection, or persistence failure is an error; `mo` never writes around it.
- An open request is successful only after the GApplication action call acknowledges
  it. Every production D-Bus method call has a five-second deadline. Rejection, timeout,
  disappearance, or malformed acknowledgement is an error and never falls back to a
  write.

Tests use temporary stores and a private `dbus-daemon`. They never address the normal
session bus or personal task data.

## Linux and macOS differences

The shared `mo` query and mutation commands remain available on both desktops. Exact
`mo open` is Linux-only and uses GApplication/GAction; on macOS, use the Momentum
**Open Task** Shortcut. Linux also exposes URI, GNOME Shell, KRunner, and desktop-file
surfaces, while macOS exposes native Shortcuts and Spotlight actions. These interfaces
are equivalent capabilities, not identical platform APIs; mobile automation remains a
separate planned design.
