# Morning summary — F-038

Approved direction: 2026-09-16. Every native app must let the user configure the
morning notification in Settings instead of sending it automatically.

## Shared behavior

- **Morning summary** is off by default for both new and existing installations.
  The old automatic 05:00 summary is replaced, with no task-data migration.
- Enabling exposes a local hour/minute choice, initially **08:00**. Turning it off
  stops future summaries and retains the chosen time. Task reminders remain independent.
- The Rust engine owns opt-in, time eligibility, task counts and the persisted local
  daily claim. Native preferences supply `morning_summary_enabled` and
  `morning_summary_time`; these preferences are not written into synchronized task data.
- While running, a desktop app evaluates the summary on its reminder tick at or after
  the selected local time. Starting later checks that day's summary; there is no backlog
  for missed days and no exact-minute delivery guarantee when the app is asleep/closed.
- At most one evaluation per local day survives restart, disable/re-enable, and time
  changes. No unfinished tasks in Today means no notification; that day remains checked.
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
| iOS | Planned: native Settings switch and locale-aware time picker, off by default. Reuse Rust summary rules with native notification permissions and scheduling appropriate to suspended apps. Disable/reschedule pending summaries when the preference changes. |
| Android | Planned: native Settings switch and local time picker, off by default. Reuse Rust summary rules with native notification channels/permissions and scheduling within OS background limits. Disable/reschedule pending summaries when the preference changes. |

Mobile scheduling, permission denial, restart, DST, background delivery and cancellation
require implementation and device acceptance once each app exists. Desktop tick behavior
must not be copied as if a mobile process can stay running indefinitely.

Controls and explanatory text have English/German coverage. Native controls provide
semantic labels, keyboard interaction and disabled-state exposure. Time, enablement and
notification permission are separate: turning on the summary cannot override OS policy.

## Verification

See the dated F-038 entry in [PROGRESS.md](PROGRESS.md) for the exact tests, native
readback and remaining checks. Per-platform statuses live in [FEATURES.md](FEATURES.md).
