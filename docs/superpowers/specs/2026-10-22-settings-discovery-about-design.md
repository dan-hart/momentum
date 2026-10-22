# Settings discovery and About

Status: approved by the user on 2026-10-22. Applies to Linux, macOS, iOS and Android.

## Goal and relation to current work

Make every available setting easy to browse and find, and give About a considered
native presentation. This joins the proposed [day-period layout and empty-state
changes](2026-10-22-day-period-layout-design.md) and its
[100-message catalog](2026-10-22-empty-state-message-catalog.md).
Existing preference values, keys, defaults, settings transfer boundaries, consent,
sync providers and storage formats remain intact. No new dependency is required.
The [completion-sound refresh](2026-10-22-completion-sound-refresh-design.md) adds
ten audition candidates, independent preview controls and a device-local volume
control under Sounds & Feedback, preserving prior saved choices.

## Approaches considered

1. **Categorized native Settings with a searchable directory (recommended).**
   Clear homes for controls, native navigation and exact destinations from either
   search surface. More work than adding a filter, but fixes browsing and discovery.
2. **Keep existing screens and add text filtering.** Smaller change, but leaves
   overloaded General tabs and long pages, and produces inconsistent destinations.
3. **Replace Settings with a command palette.** Fast for experienced users, but weak
   for browsing and unnecessarily replaces familiar native controls.

## Information architecture

Use the same conceptual categories across platforms. Show only applicable sections;
do not add empty categories or unsupported controls to manufacture visual parity.
Each setting has one canonical home; existing contextual View Options can remain
shortcuts to the same underlying preference.

| Category | Settings and subsections |
| --- | --- |
| Tasks & Planning | Group By, Sort By, sort direction, Upcoming range; Morning/Evening visibility in Today; archive completed tasks; dynamic empty-state messages |
| Appearance | System/light/dark where supported, accent/dynamic color where supported, project/tag colors, compact rows; font, content size, interface size, typography reset |
| Sounds & Feedback | Completion sound enablement, sound selection and preview; mobile haptics where supported |
| Notifications | Permission/status and system settings links; task reminder availability; Morning summary and native time picker; Android exact-alarm status/actions |
| Sync & Devices | Off/Nextcloud/LibreSync selection; existing provider fields and credentials; automatic/background sync; compression; folder-copy flow; linked devices; status/retry; app-settings sync, review/bootstrap choices |
| Data & Backups | Task import/export and existing include/restore settings options; app-settings import/export; existing task/storage summaries and automation information |
| App & Integrations | Keyboard shortcuts/modifier where supported; startup/background lifetime, Dock/menu-bar/background count; system search consent/status/retry; Apple Reminders import on Apple platforms |
| About & Help | About Momentum; source code, issue reporting, license and third-party acknowledgments; developer/project information |

Before replacing each screen, reconcile every existing control and action against
this table and a stable destination ID. Preserve less prominent recovery actions,
disabled explanations and platform-specific controls. This is reorganization of
existing capability, not authorization to introduce missing platform integrations.

## Native presentation

- **macOS:** retain the system Settings scene and Command-comma. Use a searchable
  category sidebar and a grouped detail form with descriptive subsection headings.
  Resize without clipped controls; reuse an existing Settings window on navigation.
- **iOS:** native searchable Settings root and navigation stack; grouped forms and
  category subtitles. Adapt to iPad width with native split navigation where the
  existing shell supports it. Preserve Back behavior and text scaling.
- **Android:** native Material settings category list, search field and detail
  routes using existing Compose dependencies; adaptive content width and system Back.
  Avoid relying on an experimental API requiring an unapproved dependency upgrade.
- **Linux:** named AdwPreferencesPages and PreferencesGroups, native built-in
  preferences search, and existing action rows. Extend matching only as necessary
  for aliases/global search, while retaining native page navigation and focus.

Use concise descriptions only where a control needs explanation. Keep recovery
status adjacent to its control, and destructive/import flows in their existing
confirmation/review sequence. Existing failed-sync banners remain discoverable.

## Search and destinations

Settings search is available from every Settings category and searches the complete
available directory, not just the visible page. Results show a localized title and
breadcrumb (for example, “Dynamic empty-state messages · Tasks & Planning”).
Blank query restores browsing; no matches shows an actionable clear-search state.

The existing **in-app global search** additionally shows a separate Settings results
section for nonblank queries. Preserve task/project/tag/archive result ordering,
limits and task-only actions; settings results cannot enter task multi-selection,
completion, drag, archive or task count calculations. Show up to five relevant
settings and a “See all settings results” action when there are more. That action
opens Settings search with the same query. Zero task matches must still display
matching settings. Clearing/returning from Settings preserves the original task
search query and navigation state. Do not add settings to OS search indexes.

Both surfaces resolve the same stable destination ID. Activation opens the category,
reveals the relevant subsection/row, and places accessible focus there. A brief
non-motion-dependent emphasis may help locate it; never toggle a setting or open a
permission dialog just by selecting its search result. Provider-specific matches
open their category with an explanation when inactive; they must not switch providers.
Truly unsupported platform destinations are omitted. Revalidate capability on
activation and fall back to the category if availability changes.

Define structured category/destination/capability IDs once in a toolkit-independent
Rust directory exposed through generated FFI. Native resources own translated
titles, descriptions and aliases; native code owns routing and widgets. Share pure
ranking over caller-supplied localized metadata so matching rules do not diverge:
trim/case-normalize, support multiword matches, rank exact title before title prefix,
other title tokens, then aliases/descriptions. Use deterministic category/title
tie-breaking and deduplicate destination IDs. Search localized labels and aliases
(including “night”/“evening”, “text size”/“font”, “backup”/“restore”). Do not index
entered values, passwords, personal paths, server addresses or task contents into
the settings directory. Search stays offline and read-only.

Native built-in search may have different visual ranking behavior; global search
and custom native result lists use the shared matcher. Document that intentional
native difference rather than replacing libadwaita search with a custom widget.

## About presentation

Use AsNeeded's existing AboutView as structural inspiration: a generous app identity
area, readable values, developer/project information and technical details. Use
Momentum's icon, native system typography, system accent and restrained spacing.
Avoid transferring AsNeeded-specific people, pricing promises or dependencies.

- App icon/name, a short task-focused description, actual version and build.
- Three concise values: offline use without an account; privacy and optional sync;
  open source under GPL-3.0-or-later with free self-build/self-host use.
- Dan Hart attribution, verified project/source and issue links. Reuse established
  project destinations; add personal/support/beta links only if already maintained
  for Momentum. Do not invent a support channel or a donation/purchase flow.
- License and existing packaged third-party notices, contributor/translator credits
  derived from maintained project metadata. Keep legal notices reachable offline.
- “Copy app details” copies only app/version/build/platform information explicitly
  shown to the user. Exclude tasks, logs, store paths, device/account IDs and servers.
  Omit unavailable build metadata rather than showing invented values.

Apple and Android use native scrollable detail screens with scalable text and
semantic sections. macOS standard About menu opens the same enriched content as
Settings. Linux retains AdwAboutDialog, using its native links, credits, legal and
detail facilities to provide equivalent information. No animation is necessary.

## Verification and records

Add meaningful unit/state coverage for directory completeness and unique IDs,
platform capability filtering, EN/DE resource coverage, matching/ranking/dedup,
blank/no-match queries, exact destinations, back/query restoration and task/settings
result separation. Verify navigation can reveal inactive-provider controls safely.
Check sanitized app-details output and actual bundle/package metadata mapping.
Retain existing persistence/transfer regression tests so reorganizing controls does
not reset settings or accidentally make device-local preferences portable.

No new rendered UI test suites. Run shared Rust and FFI/CLI consumer checks, Apple
hostless packages, Android JVM checks and platform builds; inspect native browsing,
search/focus, accessibility, large text and long German labels where environments
are available. Record unavailable Linux/mobile runtime checks explicitly.

Implementation updates FEATURES, BUGS and PROGRESS with stable feature IDs,
per-platform implementation/verification evidence and remaining native checks.
Update current settings/help documentation and links; distinguish proposed design
from shipped behavior. Deliver in reviewable stages: directory/routes; native
Settings reorganization/search; global search discovery; About. Coordinate the
new empty-state toggle with the linked day-period work, then perform adversarial
review and unit-coverage review before the requested branch publication.

## References inspected

- AsNeeded local `AsNeeded/Views/Screens/Settings/AboutView.swift` (read-only,
  2026-10-22): identity, values, developer/support, thanks, technical details,
  dependencies and beta sections.
- [Apple SwiftUI search](https://developer.apple.com/documentation/swiftui/search)
  and [Settings scene](https://developer.apple.com/documentation/swiftui/settings).
- [Android Compose search](https://developer.android.com/develop/ui/compose/components/search-bar).
- [AdwPreferencesDialog](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.PreferencesDialog.html).
