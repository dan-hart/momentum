# Group By (F-037)

Revised 2026-09-16 for Linux and macOS; required for the future native iOS and Android
clients. This is an offline presentation preference, independent of task sorting and
sync. It does not change tasks, their tags/projects/estimates, or saved manual order.

## Behavior

- View Options → Group By offers **Morning & Night** (default), **None**, **Project**,
  **Tag**, and **Time Estimate**. The selected mode persists locally across views and
  launches (GSettings/UserDefaults `group-by`), rather than syncing to other devices.
  Missing/unknown preferences use Morning & Night; explicitly saved None/Project/Tag/
  Time Estimate choices remain selected. No stored task data or tag names are migrated.
- **Exactly one grouping layer:** the selected mode determines open-task groups.
  No automatic Today/Morning/Evening, Overdue or upcoming-date sections surround
  Project, Tag, or Time Estimate groups. None produces one flat open-task list.
  Completed tasks stay together in one separate section at the bottom. Search keeps
  its result-type separation (tasks, projects, tags, archive), caps and notes.
- **Morning & Night:** groups appear in **Today → Morning → Evening** order, showing
  only populated groups. Morning and Evening are determined by case-insensitive
  `Morning` / `Evening` tag names. All remaining tasks go into Today (the unassigned
  day-period category, not an extra due-date filter). If both tags exist, Morning wins.
  Grouping changes presentation only; existing sidebar views still filter by due day.
  Moving a task to the other day period removes opposing tags; undo restores them.
- Overdue tasks remain in the Today view within their selected group, with their due
  dates visible. Coming Up keeps its 7/30-day filter and shows dates on each row rather
  than date headings. Empty states, task membership and archive paging stay unchanged.
- Project groups use the project name in its saved color, with the same accessibility
  and colorful-label fallbacks as tag groups. Unavailable projects use **No Project**.
- Tag groups use the **first existing tag in the task's stored tag order**, excluding
  the virtual TODAY tag. A task appears once, even with multiple tags. Tags aren't
  alphabetized before choosing the first. Missing tags fall through to the next valid
  tag; tasks without one use **Untagged**. A heading displays **#Urgent**, preserving
  the tag's spelling/case, in its saved color. Search result-type context retains its normal
  styling. Invalid/missing colors, disabled colorful labels and increased contrast
  use readable system text; macOS also honors Differentiate Without Color.
- Named groups are ordered case-insensitively by title, then stable ID for duplicate
  titles. No Project and Untagged come last. Tasks retain the selected sort and
  direction inside each group; reversing task order doesn't reverse group order.
- Estimates use the task's full estimate, not elapsed or remaining time. Intervals
  are contiguous at millisecond precision:

  | Label | Estimate |
  |---|---|
  | Up to 15 min | greater than zero, through 15 minutes |
  | 16–30 min | greater than 15, through 30 minutes |
  | 31–60 min | greater than 30, through 60 minutes |
  | 1–2 hours | greater than 60, through 120 minutes |
  | Over 2 hours | greater than 120 minutes |
  | No estimate | absent/zero, negative, or nonfinite |

  These groups always run shortest to longest, then No estimate. Whole-minute labels
  are shorthand: a 15-minute-and-one-second estimate belongs in 16–30 min.
- In normal lists, subtasks stay directly under their parent and inherit its group.
  Search matches each task/subtask independently, as before. Search project/tag links,
  result caps and the single truncation note per result type are preserved.
- In Manual Order, dragging and keyboard nudging reorder within the current group.
  Cross-group row drops do nothing; edit task properties or use the existing sidebar
  drop targets to change project/tag/day. One undo restores the original full order,
  including relative positions among tasks hidden by grouping. Other sort modes keep
  their existing Manual Order requirement.

## Native clients and accessibility

The shared Rust core owns grouping, interval boundaries, order, family placement and
reordering restrictions. `Section.group` carries structured metadata (including tag/project
colors) through UniFFI. Frontends localize headings; no localized sentences enter storage.

- **Linux:** GTK settings action and native Group By radio submenu within View Options;
  libadwaita list headings and escaped Pango text for colored tags/projects. Follow
  [GNOME menu guidance](https://developer.gnome.org/hig/patterns/controls/menus.html).
- **macOS:** SwiftUI Picker in both toolbar View Options and the system View menu;
  existing native list sections, keyboard access and accessible combined group headings.
  Reuse the app's color accessibility policy and user typography preferences.
- **iOS — Planned:** expose these modes through native list options, persist locally,
  consume the same Rust metadata and intervals, preserve family/selection behavior and exclusive grouping,
  and support Dynamic Type, VoiceOver, increased contrast and non-color identification.
  Add touch-friendly editing/reordering consistent with the future native app structure.
- **Android — Planned:** expose a native single-choice list-options control, persist
  locally and consume the same Rust contract. Support scalable text, TalkBack, system
  contrast, touch targets and accessible alternatives to drag reordering.

Mobile implementation must include tests for all modes, Morning & Night defaults, first-tag-only placement,
interval boundaries, parent/subtask placement, flat None mode, visible due dates, persistence, localized
headings, accessibility, and edits/sync moving tasks between groups. Neither mobile
foundation nor dependencies are added as part of this desktop feature.

## Verification

See the dated F-037 entry in [PROGRESS.md](PROGRESS.md) for checks and limitations.
A Mac-hosted GTK compile is not native Linux acceptance. Existing real-device LibreSync
and Spotlight deferrals remain in effect.
