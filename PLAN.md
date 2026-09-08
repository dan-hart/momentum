# Native GNOME client for Super Productivity — implementation plan

Researched 2026-09-07 against Super Productivity v18.21.2 and GNOME 50 (GNOME 51 ships 2026-09-16).

## 1. What we are dealing with upstream

| Fact | Detail |
|---|---|
| Repo | github.com/super-productivity/super-productivity (moved from johannesjo/ in 2025) |
| Stack | Angular 21 + NgRx 21 + Electron 43 + Capacitor 8, TypeScript |
| License | MIT everywhere (main repo, plugin-api, plainspace, oplog-sync, Flathub manifest). No CLA, no license changes in history. |
| Persistence | Event-sourced op log in IndexedDB (`SUP_OPS`), snapshot + op tail, compaction every 7 days. A SQLite backend exists but is not wired in. |
| Schema | `CROSS_MODEL_VERSION` 4.5, `CURRENT_SCHEMA_VERSION` 4 (min supported 1). Entity types: TASK, PROJECT, TAG, NOTE, GLOBAL_CONFIG, SIMPLE_COUNTER, TIME_TRACKING, TASK_REPEAT_CFG, ISSUE_PROVIDER, PLANNER, MENU_TREE, METRIC, BOARD, SECTION, REMINDER, PLUGIN_*, archiveYoung/archiveOld. |
| Sync | One op-log pipeline for Dropbox, OneDrive, WebDAV, Nextcloud, LocalFile, SuperSync. Ops carry vector clocks (max 20 clients, pruned). Payload = the NgRx action payload, ~155 action types with a short-code table. |
| File sync format | v2: `sync-data.json` = `{version:2, syncVersion, schemaVersion, vectorClock, lastModified, clientId, state, archiveYoung?, archiveOld?, recentOps[≤2000]}`. v3 "surgical" split: `sync-ops.json` + `sync-state__<gen>` snapshots. Content prefixed `pf_<flags>__`, optional gzip and E2EE. |
| SuperSync | Fastify + Postgres, JWT via magic link / passkey. `POST/GET /api/sync/ops`, `/snapshot`, `/status`, `/devices`, WebSocket `/api/sync/ws`. Optional E2EE: Argon2id (64 MiB, t=3, p=1) + AES-256-GCM, only `op.payload` encrypted. |
| Conflicts | Per-entity vector-clock detection; archive-wins, delete-wins for projects, disjoint-field merge, then last-writer-wins on op timestamp with clientId tie-break. |
| Other APIs | Local REST API on 127.0.0.1:3876 (Electron only, off by default, bearer token file). URL scheme `superproductivity://create-task`. Plugin API (iframe). |
| Linux today | Flathub `com.super_productivity.SuperProductivity` (repackaged .deb, x86_64 only), Snap, AppImage, Arch `extra`, nixpkgs. Only GNOME-specific code is a dead 2018 Shell extension and an ext-idle-notify helper. |
| Maintainer signal | Issue #5341 (CLI/TUI): maintainer says a standalone client "that could sync independently would be really really cool". `oplog-sync` repo was created 2026-05 to extract the sync core. A sync-simplification roadmap is in flight (`docs/long-term-plans/`, `docs/plans/2026-07-13-*`). |
| Velocity | Weekly releases, ~280 commits/month, mostly one author. Expect the sync contract to keep moving. |

The key docs to read before writing code, all in the upstream repo:
`docs/sync-and-op-log/operation-log-architecture.md`, `vector-clocks.md`, `supersync-encryption-architecture.md`, `conflict-journal-and-review.md`, `packages/sync-core/src/operation.types.ts`, `packages/sync-providers/src/file-based-sync-data.ts`, `packages/sync-core/src/compact-operation.types.ts`, `action-type-codes.ts`, `packages/shared-schema/src/supersync-http-contract.ts`, `docs/wiki/3.06-User-Data.md`, `docs/wiki/3.01-API.md`.

## 2. The central design decision: how to interoperate

Three integration depths, in increasing cost. Build them in this order; each is shippable on its own.

**A. Backup-file interop (read/write `AppDataComplete` JSON).**
Import the `backups/YYYY-MM-DD_HHmmss.json` files the Electron app writes, and export the same shape. Zero protocol risk, gives a working offline app and a migration path on day one. No live sync.

**B. Controller mode (talk to a running Electron app over the local REST API).**
Useful only as a companion or for a GNOME Shell/quick-add widget. Not a native app in its own right. Optional; skip unless you want a quick-entry popover early.

**C. Independent sync peer (the actual goal).**
Reimplement the op-log client: Operation envelope, vector clocks, compact codec, `pf_` prefix + gzip + E2EE, file-based v2/v3 CAS semantics, SuperSync HTTP + WebSocket, and the reducer semantics for every action type the app must *apply*. This is what makes it a real second client next to desktop, Android and iOS.

Two ways to do C:

| Approach | Pros | Cons |
|---|---|---|
| **C1. Rust port** of `shared-schema`, `sync-core`, `sync-providers`, and the feature reducers | Native, fast, type-checked, no JS runtime in a GNOME app, crates reusable by a CLI or other GPL projects | ~155 action types to re-implement; upstream churn means continuous conformance work |
| C2. Embed upstream TS reducers in a JS engine (rquickjs/deno_core) behind a Rust facade | Always semantically identical to upstream | The reducers live inside `src/app/features/*/store`, not in a package, so extraction is still real work; JS engine in the sandbox; harder to review and unusual for a Circle app |

Recommendation: **C1**, with a conformance harness that runs upstream's own TypeScript in CI to produce golden op logs and expected states, and diffs them against the Rust implementation. Coordinate with the maintainer early, since `oplog-sync` is being extracted right now and a Rust or language-neutral spec would benefit both sides. Scope control: the native client must *apply* every action type it can receive, but only needs to *emit* the subset its UI supports. Unknown or newer action types (schemaVersion > 4) should stop sync with a clear "update the app" message rather than corrupt state, which is what upstream does for min/max schema version.

## 3. Technology choices

| Area | Choice | Why |
|---|---|---|
| Language | Rust | 42 % of GNOME Circle apps; gtk4 0.11.4, libadwaita 0.9.2 (wraps libadwaita 1.9), official gtk-rust-template, cargo vendoring for Flathub is solved |
| UI | GTK 4.22 + libadwaita 1.9, composite templates written in **Blueprint** (bundled in the GNOME SDK since 49) | Standard stack; Blueprint is what new GNOME apps use |
| Framework | Plain gtk-rs with `glib::Object` subclassing; Relm4 0.11 is optional | Most Circle apps use plain gtk-rs; fewer layers to fight when matching HIG precisely |
| Build | Meson wrapping cargo, from gtk-rust-template | Gives gresources, gettext, GSettings, desktop/metainfo validation tests, Flatpak manifest, CI nightlies |
| Storage | SQLite via `rusqlite` (bundled) | Mirrors upstream's `SUP_OPS` design: `ops(seq, op_id, json)`, `state_cache`, `vector_clock`, `client_id`, `archive_young`, `archive_old`, `meta`. Upstream even has a SQLite adapter design doc to mirror. |
| Serialization | `serde` + `serde_json`, with `#[serde(rename)]` tables generated from upstream `action-type-codes.ts` | Exact key compatibility with compact ops |
| HTTP/WebDAV | `reqwest` with `If-Match` strong-ETag CAS; `tokio-tungstenite` for SuperSync WS | Matches upstream concurrency semantics |
| Crypto | `argon2` + `aes-gcm` crates | Same parameters as upstream (Argon2id 64 MiB/3/1, AES-256-GCM, `[salt16][iv12][ct+tag]`) |
| Secrets | `oo7` | Secret Service or portal-backed file store, works sandboxed |
| Portals | `ashpd` | Background/autostart for reminders and running timers, Notifications |
| Idle detection | `ext-idle-notify-v1` via `wayland-client`, fallback D-Bus `org.gnome.Mutter.IdleMonitor` | Same two mechanisms upstream already uses |
| Notes editor | GtkSourceView 5 with Markdown language | SP notes are Markdown |
| Runtime | `org.gnome.Platform//50` now, move to `//51` when it lands (Sept 2026). 50 is EOL April 2027. |

## 4. Crate layout

```
superproductivity-gnome/
  Cargo.toml (workspace)
  meson.build, build-aux/, data/ (metainfo, desktop, gschema, icons), po/
  crates/
    sp-model/     # AppDataComplete + every entity struct, schema v4, serde, migrations shim
    sp-oplog/     # Operation, VectorClock, compact codec, pf_ prefix, gzip, E2EE, apply() for all action types
    sp-store/     # SQLite op-log persistence, snapshot/compaction, client id
    sp-sync/      # providers: LocalFile, WebDAV/Nextcloud, SuperSync (HTTP+WS); conflict resolution
    sp-conformance/ # test harness: runs upstream TS fixtures, compares JSON
    app/          # GTK/libadwaita application
```

`sp-model`, `sp-oplog`, `sp-store`, `sp-sync` have no GTK dependency so they can be reused by a CLI or other GPL tools.

## 5. UI mapping to GNOME HIG

| Super Productivity feature | GNOME implementation |
|---|---|
| Left nav (Today, Planner, Projects, Tags, Boards, Worklog) | `AdwNavigationSplitView` with `AdwSidebar` (1.9): sections, search filter, becomes boxed list on narrow widths |
| Task list with drag reorder, subtasks, sections | `GtkListView` (not boxed list) over a `GListModel` of task objects; `GtkTreeListModel` for subtasks; DnD via `GtkDragSource`/`GtkDropTarget`; sections as headers |
| Add-task bar with `#tag`, `+project` shortcuts | `GtkEntry` at top of content view, Ctrl+N focuses it, popover completion |
| Task detail panel | `AdwOverlaySplitView` end pane on wide; `AdwDialog` (bottom sheet on narrow) |
| Running timer / current task | Pill in the header bar with elapsed time, `AdwBreakpoint` hides label on narrow |
| Focus mode | Dedicated `AdwDialog` full-window with big timer, break reminders |
| Pomodoro, break reminders, idle detection | Timers on the GLib main loop; `GNotification` with actions; idle via ext-idle-notify |
| Reminders when window closed | Background portal (`ashpd` `Background::request().auto_start(true)`) and `SetStatus` so it shows in Background Apps; there is no tray in GNOME, do not port the indicator |
| Notes | GtkSourceView Markdown editor in an `AdwDialog` or side pane |
| Worklog, metrics, simple counters | `AdwPreferencesPage`-style summaries; CSV export via file chooser portal |
| Settings | `AdwPreferencesDialog` |
| Sync setup | Preferences page: provider combo, `AdwPasswordEntryRow`, stored in oo7 |
| Keyboard | HIG defaults (Ctrl+N, Ctrl+F, Ctrl+, , Ctrl+?, Ctrl+Q) plus SP's own (e.g. Ctrl+Enter done). `AdwShortcutsDialog` (1.8). Keyboard-first is a hard requirement given SP's audience. |
| Undo after delete/done | `AdwToast` with Undo |
| Theming | Single `style.css` with `prefers-color-scheme` media queries (1.9 deprecated split stylesheets); use SP project theme colour only as an accent |

Out of scope for v1: issue providers (Jira, GitLab, GitHub…), boards/kanban, plugins, iCal import, PlainSpace. Keep their entities in the model so sync does not lose them.

## 6. Phases

| Phase | Deliverable | Rough size |
|---|---|---|
| 0. Spike | gtk-rust-template scaffold, app ID, Flatpak builds, load a backup JSON and render Today list | 2–3 weeks |
| 1. Data core | `sp-model` (all entities, schema v4), `sp-oplog` envelope + vector clocks + compact codec + prefix/gzip/E2EE, `sp-store` SQLite, conformance harness green against upstream fixtures | 4–6 weeks |
| 2. Local MVP | Today, Projects, Tags, add/edit/done/reorder, subtasks, time tracking, notes, reminders, notifications, focus mode, backup import/export, preferences, shortcuts dialog | 6–8 weeks |
| 3. Sync | LocalFile first (single writer, simplest), then WebDAV/Nextcloud (v2 then v3), then SuperSync HTTP + WS + E2EE; `apply()` for the full action table; conflict handling; multi-device tests against the Electron app | 6–10 weeks |
| 4. Ship | a11y pass (Orca, keyboard, high contrast), i18n hooked to GNOME l10n, screenshots, metainfo/OARS/branding, aarch64 build, Flathub submission | 3–4 weeks, then ongoing |

Phase 2 is shippable on Flathub as an offline app with backup import. Do not wait for Phase 3 to get users and feedback.

## 7. Risks

- **Upstream churn.** Weekly releases and an active sync-simplification plan. Mitigate with the conformance harness pinned to an upstream tag, a `schemaVersion` gate, and by joining the `oplog-sync` extraction effort so the contract is documented once.
- **Reducer fidelity.** ~155 action types. Prioritise by frequency in real op logs (task CRUD, time tracking, done/undone, move, tag/project assignment), and treat the rest as "apply from snapshot state" until covered.
- **Flathub and GNOME Circle AI policies (since 2026-05-29).** Flathub rejects new submissions containing AI-generated or AI-assisted code, manifests or PR text; Circle adopted the same policy and has paused new submissions until its backlog clears. Decide early whether you will use AI assistance, and document provenance accordingly. Circle also forbids "GNOME" in the app name.
- **Name and app ID.** MIT grants no trademark rights. Pick a distinct name and ID (`io.github.<you>.<Name>` unless you own a domain), and ask the maintainer whether they mind a name that references Super Productivity. Do not reuse `com.super_productivity.*`.
- **Runtime EOL.** GNOME 50 runtime is EOL April 2027; plan to be on 51 before Flathub submission.

## 8. License

**Decision: GPL-3.0-or-later for the whole project**, including the protocol crates.

Why this works:

- Upstream is MIT with no CLA, so its code, docs, schema and formats can be ported into a GPL project. Keep the MIT notice and the line "Copyright (c) 2018 Johannes Millan" in every file that is a translation of upstream code, and list upstream in a `LICENSES/MIT.txt` alongside `LICENSES/GPL-3.0-or-later.txt` (REUSE layout).
- GNOME Circle requires an OSI licence and no CLA; GPL-3.0-or-later is the most common Circle licence (Planify, Solanum, Endeavour). Flathub accepts it.
- Every planned dependency (gtk4-rs, libadwaita-rs, rusqlite, reqwest, tokio-tungstenite, argon2, aes-gcm, oo7, ashpd, gettext-rs) is MIT, Apache-2.0 or LGPL, all GPL-compatible.

Trade-off to be aware of: code flows one way. Upstream cannot take your GPL crates into its MIT `oplog-sync` repo. If you later want to contribute a port back, you can relicense your own contributions to MIT at that point, since you hold the copyright and there is no CLA on either side.

Practical steps:

1. Put `SPDX-License-Identifier: GPL-3.0-or-later` headers in every source file; `LICENSE` at the root is the GPL-3.0 text.
2. `project_license` in the metainfo is `GPL-3.0-or-later`; `license` in each `Cargo.toml` is `GPL-3.0-or-later`.
3. Add a `NOTICE` or README section crediting Super Productivity and stating which files derive from it.

## 9. First week checklist

1. Clone upstream at tag `v18.21.2`; read the docs listed in section 1.
2. Open a GitHub Discussion upstream describing the plan, referencing #5341 and the `oplog-sync` repo, and asking about naming.
3. `git clone https://gitlab.gnome.org/World/Rust/gtk-rust-template`, run its init script with your app ID, confirm `flatpak-builder` succeeds on `org.gnome.Sdk//50`.
4. Write `sp-model` from `packages/shared-schema` and the `*.model.ts` files; round-trip a real backup JSON with zero diff.
5. Extract `action-type-codes.ts` into a generated Rust table with a test that fails if upstream's file changes.
