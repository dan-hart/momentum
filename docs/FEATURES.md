# Momentum feature ledger

Updated: 2026-09-17. See [AGENTS.md](../AGENTS.md) for policy and status definitions.
Every row has an independent Linux, macOS, iOS, and Android status. `Implemented`
is deliberately not a claim of complete native verification. Baseline entries below
are seeded from the existing source/specifications and recorded acceptance, not a
fresh audit of every legacy feature. Their evidence is dated in [PROGRESS.md](PROGRESS.md).
Split a row into stable child IDs if its sub-capabilities progress differently.

## Existing capabilities and mobile requirements

Unless a row says otherwise, mobile Planned means: expose the same Rust behavior
through native, accessible mobile UI; verify persistence, offline operation, and
platform interaction. It does not mean a mobile app or binding already exists.

| ID | Capability / acceptance scope | Linux | macOS | iOS | Android | Evidence, differences, and remaining work |
|---|---|---|---|---|---|---|
| F-001 | Shared Rust task engine and native frontend boundary | Implemented | Implemented | Planned | Planned | `momentum-core`; GTK calls directly, Mac uses UniFFI. Mobile: establish bindings/services without duplicating rules. |
| F-002 | Durable local tasks without account/network | Implemented | Implemented | Planned | Planned | MVP §§6,8; `sp-store`. Mobile: native storage locations, lifecycle-safe persistence, offline restart checks. |
| F-003 | Today, overdue, Morning/Tonight, Coming Up | Implemented | Implemented | Planned | Planned | MVP §2.1; preserve dates, slot exclusivity, ordering and recurrence semantics. |
| F-004 | Project/tag lists and management | Implemented | Implemented | Planned | Planned | MVP §§2.1,2.4; mobile management must use native navigation/forms. |
| F-005 | Quick add with tags, estimates and autocomplete | Implemented | Implemented | Planned | Planned | MVP §2.2; mobile needs native text entry and input-method checks. |
| F-006 | Task editing, notes, subtasks and duplication | Implemented | Implemented | Planned | Planned | MVP §§2.3–2.4; native forms and accessible actions. |
| F-007 | Scheduled days/times and reminders | Implemented | Implemented | Planned | Planned | MVP §2.3b; mobile notification permission/scheduling/background behavior requires design and device checks. |
| F-008 | Recurrence editor, deterministic instances and catch-up | Implemented | Implemented | Planned | Planned | MVP §§2.5–2.5b; reuse core recurrence rules on mobile. |
| F-009 | Completion, archive, restoration and auto-archive | Implemented | Implemented | Planned | Planned | MVP §§2.6,2.9; preserve parent/subtask semantics and undo. |
| F-010 | Multi-selection and bulk actions | Implemented | Implemented | Planned | Planned | MVP §2.4b; mobile uses native selection, not desktop modifier-key imitation. |
| F-011 | Single/multiple-task project drag | Implemented | Verified | Planned | Planned | User confirmed Mac single and multi-selection moves on 2026-09-16; [Mac checklist](MACOS-MVP-CHECKLIST.md). Mobile: native drag where supported plus accessible move action. |
| F-012 | Manual reorder and batch undo | Implemented | Implemented | Planned | Planned | User confirmed Mac manual reorder/Undo; descending/hidden-neighbor cases have core tests but not separate live verification. |
| F-013 | Drag to tag/day; reject archived drag | Implemented | Implemented | Planned | Planned | Core routing/payload coverage; individual Mac native gestures still unverified. Mobile needs an accessible equivalent. |
| F-014 | External text/URL drop to create tasks | Implemented | Verified | Planned | Planned | User confirmed Mac native drop, 2026-09-16. Mobile: native drop where supported; assess share/paste entry points in mobile design. |
| F-015 | Multiline paste and ordinary text replacement | Implemented | Verified | Planned | Planned | Mac live regression recorded in checklist; shared parsing in core. Mobile: clipboard/input-method verification. |
| F-016 | Search over tasks, notes, subtasks and archive | Implemented | Implemented | Planned | Planned | MVP §2.7; mobile search uses shared queries and native presentation. |
| F-017 | Undo, actionable errors and operation feedback | Implemented | Implemented | Planned | Planned | MVP §2.6; native feedback with accessible undo/recovery. |
| F-018 | Useful empty states and compact layouts | Implemented | Implemented | Planned | Planned | Mac empty/short-window checks recorded; exact minimum width unverified. Mobile: phone/tablet, keyboard and larger-text layouts. |
| F-019 | Native appearance, system accent and reduced motion | Implemented | Implemented | Planned | Planned | Mac Today selection observed with system accent/contrasting star; accessibility settings checked. Mac task checkboxes and trailing badges now center vertically against the full row; Debug build and native title/metadata-row inspection passed 2026-09-16. Mobile follows system conventions. |
| F-020 | Accessibility semantics, focus and screen readers | Implemented | Implemented | Planned | Planned | Mac AX/keyboard/contrast checks passed; spoken VoiceOver remains unverified. Linux runtime checks not rerun on this Mac. Mobile: VoiceOver/TalkBack, scalable text and native focus. |
| F-021 | Localized UI, dates and plural forms | Implemented | Implemented | Planned | Planned | English/German catalogs and Mac layouts checked; independent language review not performed. Mobile inherits language coverage with native resources. |
| F-022 | Native font selection and separate content/UI sizes | Implemented | Verified | Planned | Planned | Linux native GTK font dialog, independent 75–250% content/interface scaling, persistence/reset, missing-font fallback and runtime AT-SPI/extents checks passed 2026-09-17. Wayland blocked directed keyboard input/screenshots and the compositor expanded the requested 360×720 window, so Linux remains Implemented. Mac system font panel, persistence/reset and large-text checks are Verified. |
| F-023 | Keyboard commands, help and modifier preferences | Implemented | Implemented | Planned | Planned | MVP §2.8; mobile: hardware-keyboard support where available and touch-accessible equivalents. |
| F-024 | Desktop quick-add, background, launch and system-search integrations | Implemented | Implemented | Planned | Planned | MVP §2.11. Mac Spotlight activation/creation deferred. Mobile needs native lifecycle/search equivalents; no promise of desktop global shortcuts. |
| F-025 | Nextcloud sync with current SP format | Implemented | Implemented | Planned | Planned | Mock WebDAV coverage; normal Mac success observed. Preserve current format pending F-033. Mobile: background limits, reconnection and offline queues. |
| F-026 | LibreSync direct device pairing and convergence | Implemented | Implemented | Planned | Planned | Local integration evidence exists; physical Linux→Mac acceptance deferred; see B-001. Mobile discovery/permission/background feasibility requires verification. |
| F-027 | Exactly one selected sync provider, preserving connections | Implemented | Implemented | Planned | Planned | Linux native runtime switching on 2026-09-17 showed only the selected provider, preserved synthetic Nextcloud fields, updated CLI configuration, and never fell back across providers. A successful network exchange and linked-device transfer remain unverified; see progress. |
| F-028 | Visible provider, progress, last success, errors and retry | Implemented | Implemented | Planned | Planned | Linux native runtime on 2026-09-17 preserved Nextcloud errors with Details/Retry and showed LibreSync's no-linked-devices result. Real progress timing and successful exchanges remain unverified. Mac status bar is implemented; see progress. |
| F-029 | Backup import/export with explicit replacement semantics | Implemented | Implemented | Planned | Planned | MVP §§2.9,6; mobile: native document picker/share interfaces and safe restore flow. |
| F-030 | `mo` desktop CLI parity and safe live-app coordination | Implemented | Implemented | Not applicable | Not applicable | [CLI evidence](MACOS-AUTOMATION.md); mobile has no desktop shell CLI distribution planned. Native mobile automation is separate scope. |
| F-031 | Native automation actions | Implemented | Implemented | Planned | Planned | [Linux automation](LINUX-AUTOMATION.md) adds exact task reveal through `mo open`, GAction and URI entry points with stable-ID selection, non-mutation/error guarantees, private-bus warm/cold/coexistence/timeout coverage, and native GAction/URI/SearchProvider observations on 2026-09-17. A user-visible registered-profile cold launch was not observed, so Linux remains Implemented. Mac Shortcuts warm-app flows are verified; cold invocation remains unverified. |
| F-032 | Configurable launcher/dock badge count | Implemented | Implemented | Planned | Planned | Linux exposes due-or-scheduled today, Today including overdue, and Off/neutral Background Apps messages; live portal calls and relaunch persistence were observed 2026-09-17, but GNOME's visual Background Apps surface was not automatable. Mac offers Off/Today/due-or-scheduled today. |

## Active feature request

| ID | Capability / acceptance scope | Linux | macOS | iOS | Android | Evidence, differences, and remaining work |
|---|---|---|---|---|---|---|
| F-037 | Exclusive grouping by day period, project, first tag, or similar estimates | Implemented | Implemented | Planned | Planned | [Grouping contract](GROUPING.md): Linux native action/readback and headings for every mode, first-tag placement, all six estimate buckets, Completed separation, due-date visibility and persistence were observed 2026-09-17. Pointer drag, keyboard reorder/Undo, cross-group rejection and visual high-contrast/color fallback were unavailable, so Linux remains Implemented. Mac has its previously recorded implementation evidence. Mobile must use the same exclusive grouping rules. |
| F-038 | Opt-in morning summary with a configurable local time | Implemented | Implemented | Planned | Planned | [Notification contract](NOTIFICATIONS.md): Linux native Off/08:00 defaults, enabled 09:45 persistence and one accepted portal notification with no duplicate were observed 2026-09-17. Live SpinRows were absent from AT-SPI and denial/failure delivery was unavailable, so Linux remains Implemented. Mac Settings readback confirms Off and disabled 08:00; interactive time editing/system delivery remain unverified. Mobile must implement the same opt-in policy with native scheduling and permissions. |

## Future decisions and work

These are product intentions, not permission to add services, billing or migrations.

| ID | Capability / acceptance scope | Linux | macOS | iOS | Android | Decision or prerequisite |
|---|---|---|---|---|---|---|
| F-033 | v2 independence from Super Productivity | Planned | Planned | Planned | Planned | Format, migration and continued interoperability undecided; preserve current compatibility. |
| F-034 | Optional future commercial distribution/subscription model | Planned | Planned | Planned | Planned | Pricing/entitlements undecided. Source remains open and free to build, compile, self-host and use. |
| F-035 | RevenueCat subscription integration on Apple platforms | Not applicable | Planned | Planned | Not applicable | Apple-only stated intention; no SDK/entitlement approval. Other platforms' billing approaches undecided under F-034. |
| F-036 | Tipping through Apple and/or Buy Me a Coffee | Planned | Planned | Planned | Planned | Provider/platform scope and current store requirements need a later decision; no integration exists. |

## Adding or updating a feature

Assign the next unused F-ID; never renumber existing entries. Record the request and
observable acceptance criteria, core ownership, native behavior per platform, status,
test/build/live evidence, and remaining checks. Link a focused specification when a
table row cannot describe the scope precisely. A mobile placeholder must explain the
required behavior, not just say “port later.” Update [BUGS.md](BUGS.md) for defects and
[PROGRESS.md](PROGRESS.md) for the handoff. A source review establishes implementation,
not Verified status; completed acceptance may be carried forward only with dated evidence.
