# Apple Reminders import — F-055

The native iOS and macOS integrations are optional, off by default, and live in
**Settings → Apple Reminders**. Choose **Connect Apple Reminders**, allow the system
permission, select
one list, then choose **Import Now** or turn on **Automatically Import New Reminders**.
Connecting/selecting a list alone does not import tasks. Enabling automatic import
imports the selected list's current incomplete reminders as well as later additions.

The integration only reads Apple Reminders. It never creates, updates, completes or
deletes Apple reminders. Imported tasks are independent Momentum tasks in Inbox:
trimmed title, notes plus URL, and the reminder's due calendar day (normalized to the
Gregorian date expected by the core). Blank titles and completed reminders are skipped.
Due times, alerts, priority, attachments, hierarchy and recurrence are not mapped.
Later edits/completions/deletions in either app are not mirrored. A completed reminder
that is later reopened can be imported if it has never been imported previously.

## Native access and lifecycle

The Connect button is the sole authorization request path. Both Apple apps use
`requestFullAccessToReminders()` and `NSRemindersFullAccessUsageDescription`, with an
English/German permission explanation. EventKit requires full access for reading;
there is no read-only Reminders grant. Denied/restricted/revoked access stops fetching,
shows recovery to the platform's Settings, and preserves imported tasks and list
selection.
A deleted/unavailable list prompts reselection; an empty list reports zero imports.

One retained EventKit store observes `EKEventStoreChanged`. Each accepted change
refetches the selected list and incomplete-reminder predicate; it does not retain
invalidated EKReminder/EKCalendar objects or interpret notification payloads as IDs.
Notifications during a fetch coalesce into one additional pass. Generation checks
stop remaining imports after turning automatic import off, changing lists or leaving
the foreground. Already committed tasks remain durable.

Automatic import runs while Momentum is active and on reactivation. No Reminders
background task is registered. EventKit notifications do not grant an iOS background
wake. Background Tasks are discretionary system-scheduled work, not a real-time
Reminders delivery contract. The Settings footer states this limitation explicitly.

## Identity and persistence

The adapter prefers `calendarItemExternalIdentifier` and falls back to
`calendarItemIdentifier` when no server identifier is available. It hashes the typed
identifier with SHA-256 before saving source provenance or local skip history. When
a server identifier exists, its local identifier is also saved as a hashed alias so
a reminder that first receives a server ID is not imported again. Matching existing
live/archive tasks learn new identities through durable `UpdateTask` provenance
operations before the device-local history shortcut is applied. This reconciliation
never creates a task, changes task content or consumes undo; archive Undo retains
identities learned after its original restore snapshot. Hashes
are deduplication metadata, not anonymization; they are never logged. The core stores
`momentumImportSource` and optional `momentumImportAliases` in the Task format's existing extensible fields in the same
transaction as creation. Deduplication checks both live and archived tasks under the
store lock, including concurrent import requests and retries after a crash between
task save and local-history save. Existing sync/backups preserve task provenance.
Local history also skips previously imported sources after deletion or Undo, and
survives switching lists and disabling automatic import. Reconciliation failures
remain retryable and do not advance skip history. Fetch/save recovery retries import;
a missing-list recovery reloads only the inventory. Each terminal attempt delivers
a localized accessibility announcement, including repeated identical outcomes; an
error takes precedence over a partial-success count. Local skip history lives in
device-local settings, not a task backup; reinstall/reset can remove that deleted-task history.

Apple warns that a full database sync can replace local identifiers, server IDs can
represent duplicate copies, and Exchange reminder IDs differ across devices. Thus
fallback local-only sources after an account/database reset, or simultaneous imports
on devices with different source IDs, can be imported again. Copies sharing a server
ID are treated as one source. List IDs may also become unavailable after account
changes; the user must select the list again. This feature does not claim universal
cross-device identity or two-way reconciliation. Recurring source identifiers already
imported are skipped; recurrence behavior is not translated.

## Verification boundaries

Tests use synthetic source snapshots and UUID-isolated stores/preferences. Default
preview/demo/test models never instantiate EventKit. CLI builds do not request personal
data access. Native hosted tests inject a fake source into the production model and
Settings view. Real permission prompting, system revocation, spoken accessibility,
physical-device suspension require a later approved
isolated system acceptance pass. A simulator-only opt-in regression creates one
UUID-named local list and synthetic reminder, exercises actual EventKit fetching,
and deletes its own list. It reproduced B-095 on `4a40345` and passes with an
explicit Sendable callback and nonisolated snapshot projection. It establishes no
physical-device or background-delivery claim. See [iOS testing](../ios/TESTING.md).
The reusable EventKit value adapter and import coordinator live in the existing
MomentumKit package. iOS EngineWorker and macOS AppState invoke the same Rust import
and reconciliation APIs. The native Mac Settings tab uses a popup list picker, and
application activation/resignation controls automatic imports. Demo/service-free
test state has no native Reminders source unless a synthetic source is injected.
Native Settings lifecycle/footer and permission recovery wording remain platform
specific. See [PROGRESS.md](PROGRESS.md) for exact verification and remaining native
permission/spoken accessibility boundaries.

## Official Apple references

- [Accessing the event store](https://developer.apple.com/documentation/eventkit/accessing-the-event-store)
- [Request full access to reminders](https://developer.apple.com/documentation/eventkit/ekeventstore/requestfullaccesstoreminders(completion:))
- [Reminders privacy description](https://developer.apple.com/documentation/bundleresources/information-property-list/nsremindersfullaccessusagedescription)
- [Retrieving events and reminders](https://developer.apple.com/documentation/eventkit/retrieving-events-and-reminders)
- [Updating with notifications](https://developer.apple.com/documentation/eventkit/updating-with-notifications)
- [Local calendar item identifier](https://developer.apple.com/documentation/eventkit/ekcalendaritem/calendaritemidentifier)
- [External calendar item identifier](https://developer.apple.com/documentation/eventkit/ekcalendaritem/calendaritemexternalidentifier)
- [Choosing background strategies](https://developer.apple.com/documentation/backgroundtasks/choosing-background-strategies-for-your-app)
