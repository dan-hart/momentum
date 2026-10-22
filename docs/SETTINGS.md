# Settings and daily task views

Momentum uses native Settings on Linux, macOS, iOS and Android. Available controls
vary with the platform and its permissions. The implementation and verification
scope is tracked under F-071–F-074 in [FEATURES](FEATURES.md) and [PROGRESS](PROGRESS.md).

## Find a setting

Browse a category or search inside Settings. You can also use task search: matching
settings appear in their own section, with a category breadcrumb. Choose a result
to open its control or subsection. Searching or opening a result does not change its value.
Task search retains your query when you return. Settings discovery works offline.

| Category | What you will find |
| --- | --- |
| Tasks & Planning | Grouping, sorting, Upcoming range, completed tasks, Morning/Evening visibility and empty-state messages |
| Appearance | Colors, density, fonts and text sizes supported on your device |
| Sounds & Feedback | Completion sounds, auditioning, volume and supported haptic controls |
| Notifications | Reminders, Morning summary and relevant permission/status controls |
| Sync & Devices | Selected provider, connection, devices, automatic sync, status and app-settings sync |
| Data & Backups | Import, export, restore and app-settings transfer |
| App & Integrations | Available keyboard, background, badge, system search and Reminders integrations |
| About & Help | Version/build, project information, source, issue reporting and offline legal notices |

Linux uses libadwaita's native preferences search and About dialog. macOS uses a
Settings sidebar, iPhone uses category navigation, iPad adapts to a split layout,
and Android uses native category/detail pages. Search results for an inactive sync
provider explain what is needed without switching providers or discarding connections.

## Morning, day and evening

In Today, **Group By → Morning & Night** uses sun/moon headings, extra space and
subtle indentation instead of separator lines. Empty groups are omitted. Other
Group By layouts retain their normal presentation.

In **Tasks & Planning**, turn off **Show Morning and Evening tasks in Today** to
keep today's slotted task families in their separate sidebar destinations. Overdue
work stays in Today. A quiet reminder at the bottom counts hidden unfinished
Morning/Evening families and offers a link to the appropriate list. Subtasks are
not counted again, and completed/future work is not included in that reminder.

## Completed-list messages

Today, Morning and Evening can show encouraging copy when the relevant work is
finished. The 100-message catalog includes Morning, afternoon and evening variants.
A message stays stable while you read it. A period can be complete while other
parts of the day still have work; hidden tasks never count as a completed whole day.

Turn off **Dynamic empty-state messages** in Tasks & Planning to use static copy.
When enabled, a small **Celebrate** action offers a brief visual celebration.
It never plays automatically and respects reduced-motion settings. This preference
is local to the device.

## Choose a completion sound

Open **Sounds & Feedback** and use the Preview control beside any candidate before
selecting it. Preview works with automatic completion sounds off and does not save
the candidate or enable automatic playback. Selecting a sound does not autoplay it.

Ten short quiet choices are available, from Soft Knock to Quiet Cheer. Previously
saved choices remain available. An unavailable sound from another app version is
identified without silently replacing the saved value.

The app volume control applies to both preview and completion playback. It starts
at 35%, remains local to your device, and works within system volume/mute rules.
Automatic completion sounds remain off by default. New playback replaces the
previous clip; leaving the picker cancels its preview.
