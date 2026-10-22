# Calm day-period grouping

Status: approved by the user on 2026-10-22, superseding F-071's separator presentation.

Related proposal: [Settings discovery and About overhaul](2026-10-22-settings-discovery-about-design.md)
provides the canonical home and search destinations for these preferences on all
four platforms. The user approved the combined proposals on 2026-10-22.

## Requested scope

Refine Morning & Night grouping on Linux, macOS, iOS and Android. Keep Today
membership, local visibility preference, ordering, undo and sidebar access unchanged.
Other grouping modes retain their layout. Add a hidden-task reminder in Today when
the existing visibility preference is off, plus optional encouraging empty/completed
states for Today, Morning and Evening destinations across all grouping modes. Do
not introduce platform UI tests.

## Recommended presentation

- Remove explicit day-period separator lines and hide ordinary open-task row/section
  separators within Today when Morning & Night grouping is active.
- Present Morning with a native sun icon and Evening with a native moon icon beside
  their localized headings. Keep Today as the plain central heading. Icons are
  decorative; the heading text and accessibility heading semantics convey meaning.
- Use approximately 24 points of space between populated sections, scaled to each
  platform's native density. No gap for an absent group and no empty placeholder.
- Inset Morning and Evening task families slightly (approximately 12 points on
  desktop, adapted for compact mobile widths). Preserve existing relative subtask
  indentation, full selection hit areas, focus, drag targets and action controls.
- Use system typography, appearance and accent policy. Avoid outlines, tinted cards
  and extra ornament. Preserve readable text in increased contrast and larger text.
- Preserve the existing Completed section and alternative grouping presentation.

Spacing, inset, icons and separator suppression are native presentation choices;
shared Rust task rules and the reviewed B-165/B-166 fixes remain authoritative.
Retire the superseded separator-only API if it has no remaining consumers, using
the existing generated-bindings tooling rather than keeping a dead public helper.

## Minimal hidden-task footer

When **Show Morning and Evening tasks in Today** is unchecked, Today shows a small,
centered footer for its hidden unfinished tasks. This depends on the visibility
setting, not Group By: the setting hides tasks in every grouping mode, so the
reminder must remain useful in every grouping mode too. No reminder appears in
other lists or when the setting is on.

Use one compact row per populated period, with localized singular/plural wording:

> 2 tasks in Morning · Go to Morning
>
> 3 tasks in Evening · Go to Tonight

Use the platform's existing localized destination name for navigation wording.
The action opens the existing Morning or Evening/Tonight destination in the current
window or mobile navigation. It does not open another window, change the visibility
setting or modify tasks. If that destination disappears before the click, use the
existing safe navigation fallback.

Place the footer beneath Today content, centered, with secondary caption text and
small native text/link actions. It must not overlay tasks or compete with task entry.
Allow wrapping and maintain native accessible action hit areas at large text sizes.
No card, border, banner, notification or new setting.

Count unfinished top-level task families using the same effective local planned-day
and slot rules as the corresponding list. Do not count completed or archived tasks,
overdue tasks that remain visible in Today, future tasks or subtasks as extra tasks.
Morning wins if both period tags exist, as it does today. Omit a period with count
zero; omit the entire footer when both counts are zero. Refresh after edits,
completion, undo, sync, setting changes and local-day rollover.

Do not show a whole-day completion celebration or claim nothing is planned for the
day while hidden unfinished tasks remain. If the visible Today list is clear, use
neutral copy scoped to that list plus the reminder. This also applies when Today
contains only completed visible tasks or only hidden tasks.

Rust owns the structured hidden counts and completion/empty-state decision.
Native clients own localized wording, layout and existing navigation actions.
Obtain the summary alongside the listing so concurrent changes cannot produce a
completed-day message paired with a stale hidden-task count.

## Encouraging empty and completion states

Use an airy native empty-state layout with a friendly contextual symbol, a concise
heading, one encouraging sentence and at most one secondary celebration action.
Morning uses a sun, Evening a moon, and a completed whole day a checkmark/sparkle
symbol. Keep the task-entry action and hidden-task footer available where relevant.

Distinguish three situations in the shared core:

- Nothing planned: a welcoming, neutral empty state. Do not claim work was completed
  or offer a completion celebration solely because the list is empty.
- This period/list is complete: celebrate only that scope. A completed Morning or
  Evening list does not imply the whole day is finished. Today with hidden tasks
  remaining uses neutral list-scoped copy plus the live footer, without whole-day
  messages or celebration.
- The whole day is complete: no unfinished Today/overdue or hidden period families
  remain, with genuine completion evidence. Support completions that were immediately
  auto-archived as well as visible completed rows. Do not build the full search/archive
  index for every ordinary listing just to obtain completion evidence.

Provide the [100-message source catalog](2026-10-22-empty-state-message-catalog.md),
localized into English/German platform resources. Rust owns stable message IDs and
eligibility/context, not localized prose. Pools: 20 Morning-list messages, 20
whole-day messages suitable at any time, 20 afternoon whole-day messages, 20
evening/night whole-day messages, and 20 Evening-list messages.

Use the local clock: morning before 12:00, afternoon 12:00–17:59, evening/night from
18:00. Date rollover reevaluates completion evidence for the new day. Pick a random
eligible message when entering a newly cleared/completed state or returning to the
destination; keep it stable while visible, without timer-driven copy cycling or
screen-reader chatter. Avoid an immediate repeat when multiple candidates exist.
Reevaluate eligibility at phase/date changes and whenever tasks change; stale copy
must not imply completion after a task is added or reopened. Never require an account,
network access, analytics or an external message-generation service.

Add **Dynamic empty-state messages** under native task-list/appearance settings,
enabled by default and saved locally per device. Off restores standard static,
accurately scoped empty/completed copy and removes the optional celebration action.
Do not silently add this setting to the existing portable-transfer/sync allowlist.

### Optional celebration

Recommend a small **Celebrate** text/button action on eligible completed-state
screens. One activation runs one brief, bounded native confetti burst; no automatic
confetti on launch, list navigation, sync or repeated redraws, and no looping.
No added sound or haptic behavior. Keep it independent of the existing completion
sound preference and retain that feature's current behavior.

Respect system reduced-motion/disabled-animation preferences. Use a static celebratory
symbol/state instead of moving particles when motion is reduced. The effect is
decorative, never blocks input, never announces individual particles, stops when the
view disappears, and is not essential to understanding completion. Use native drawing
and controls rather than a new animation dependency or shared web surface.

Official guidance/resources to consult during implementation:
[Apple motion](https://developer.apple.com/design/human-interface-guidelines/motion),
[Android Compose animation](https://developer.android.com/develop/ui/compose/animation/quick-guide),
[GNOME accessibility](https://developer.gnome.org/hig/guidelines/accessibility.html),
[GTK animation preference](https://docs.gtk.org/gtk4/property.Settings.gtk-enable-animations.html).

## Alternatives

The recommended whitespace/icon/inset treatment matches the requested calm native
layout. Subtle background cards would create stronger enclosure but add visual
weight. Spacing alone would be the smallest change but give less recognition to
Morning and Evening.

## Validation and delivery

Keep meaningful core/state/FFI unit coverage and platform builds; adapt obsolete
separator tests to any remaining semantic policy rather than test styling constants.
No new rendered UI suites. Inspect the macOS result manually, rebuild and relaunch
the local app for user testing. Update FEATURES, BUGS assessment, PROGRESS and the
grouping/MVP contract alongside implementation. Use the existing feature branch.

Unit coverage for the footer includes preference/view gating, one/both/zero periods,
singular/plural structured counts, completed/archived/future/overdue exclusion,
families without double counting, dual tags, refresh/rollover and preventing false
whole-day completion. Cover native state/navigation routing without rendered UI tests.

Completion-state units cover real completion versus unplanned/hidden work,
auto-archive, reopened tasks and undo, local noon/evening/midnight boundaries,
100 unique message IDs with eligible nonempty pools, controlled random selection,
no immediate repeats, stable visible messages, settings persistence/default/static
fallback, localization coverage, and celebration gating/cleanup/reduced-motion policy.
