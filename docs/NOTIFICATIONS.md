# Notification settings

## App badge — F-032

- Off, Overdue Only, Due or Scheduled Today, and Today Including Overdue use the shared
  Rust counts. Open parent tasks count once and zero clears the native count.
- iOS sets the Home Screen badge when active. Optional task-reminder and morning-summary
  delivery updates use the same selected count, captured during planning. Future pending
  requests reconcile when this count or the delivery switches change. If the app cannot
  run again before delivery, the notification uses its last scheduled snapshot.
- macOS writes the Dock badge and can refresh it with a task reminder or summary using
  the live count. Linux uses the existing Background Apps status where supported.
- Android posts an opt-in quiet count notification. Task reminders and summaries have
  independent badging controls. Native channels and the launcher determine how notification
  dots or counts appear.

## Morning summary — F-038

The morning notification is configured in Settings.

## Shared behavior

- **Morning summary** is off by default for both new and existing installations.
  The old automatic 05:00 summary is replaced, with no task-data migration.
- Enabling exposes a local hour/minute choice, initially **08:00**. Turning it off
  stops future summaries and retains the chosen time. Task reminders remain independent.
- Choose every day, weekdays, or individual weekdays. Choose any of Today, Morning,
  Evening, and Overdue. Today includes tasks in both day periods, and a task belongs to
  at most one day period. The summary skips days where the chosen sections have no open
  tasks. A preview uses the current local task list.
- Previous installations keep their existing time and opt-in state. Until customized,
  the summary runs every day with Today, Morning and Evening selected.
- The Rust engine owns opt-in, time eligibility, task counts and the persisted local
  daily claim. Native preferences supply `morning_summary_enabled` and
  `morning_summary_time`; these preferences are not written into synchronized task data.
- While running, a desktop app evaluates the summary on its reminder tick at or after
  the selected local time. Starting later checks that day's summary; there is no backlog
  for missed days and no exact-minute delivery guarantee when the app is asleep/closed.
- At most one evaluation per local day survives restart, disable/re-enable, and time
  changes. No unfinished tasks in the selected sections means no notification. Desktop evaluation records the check for that day.
  Existing Morning/Evening counts and notification actions are retained.
- Local wall-clock time follows the current time zone. A skipped DST minute is eligible
  at the next tick after the selected time; a repeated minute cannot generate a duplicate.
  Invalid core time values fail closed. Native controls constrain values to 00:00–23:59.
- Normal OS notification permissions and Focus/Do Not Disturb still apply. This feature
  does not change permission prompts or claim OS delivery acknowledgement. On macOS an
  unattached notification adapter no longer consumes reminders or the daily summary.

## Native surfaces and mobile requirements

| Platform | Interface and behavior |
|---|---|
| Linux | Preferences → Notifications: libadwaita SwitchRow and labeled hour/minute SpinRows (24-hour clock). Time controls are disabled while off. GSettings changes update the running engine. |
| macOS | Settings → General → Notifications: switch and system time picker, respecting locale. UserDefaults changes update the running engine. Time control is disabled while off. |
| iOS | Implemented in progress: Settings → Notifications separates permission, Morning Summary (off by default, staged locale-aware time sheet) and schedule status/retry. Save commits the final hour/minute together so an intermediate wheel value cannot consume the daily claim (B-058). Shared Rust planning/ledger drives one-shot OS requests on foreground, task edits and preference changes. A short discretionary app-refresh task requests local plan replenishment no more than daily while notification authorization permits it; expiration/failure is reported to iOS, the next request is retained, and this path never starts sync. Native delegate registers Done and Snooze 1 hour; body routing opens the task or Today. Real SpringBoard summary delivery with exact counts, Today routing and relaunch persistence passes on iOS 26.5/27, along with cold task-body activation and cold/resident actions. Real authorization denial and independent clean grant acceptance also pass on both runtimes, covering disabled recovery UI and authorized planning readback. Background policy/build/launch checks pass, but Simulator cannot launch the system refresh task or automate the uninterrupted denied-to-enabled Settings transition; physical delivery, locked-device and system-triggered background execution remain outstanding. |
| Android | Native Settings switch and local time picker, off by default. Uses shared Rust planning with Android alarms, localized section counts, and a separate summary channel. Changes to summary time, days or sections reconcile pending alarms. The latest badge preference applies when the alarm delivers. |

The direct mobile denied-to-enabled Settings transition, system-triggered background
execution and physical cancellation still require additional platform acceptance.
iOS denial and clean grant boundaries, restart/time persistence, real summary delivery,
planning/reconciliation, background scheduling policy, expiration and callback
deduplication have core/package coverage, and simulator delivery/actions have native
acceptance; none of these establish physical-device or locked-device behavior. Desktop tick behavior
must not be copied as if a mobile process can stay running indefinitely.

Controls and explanatory text have English/German coverage. Native controls provide
semantic labels, keyboard interaction and disabled-state exposure. Time, enablement and
notification permission are separate: turning on the summary cannot override OS policy.

## Verification

See the dated F-038 entry in [PROGRESS.md](PROGRESS.md) for the exact tests, native
readback and remaining checks. Per-platform statuses live in [FEATURES.md](FEATURES.md).
