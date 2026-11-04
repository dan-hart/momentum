# Sync settings redesign — F-077 / B-186

User direction: completely simplify Sync settings, improve diagnosis and Nextcloud clarity, and proceed independently with recommended choices. Local changes only. All four native apps are in scope.

## Design

Use a status-first native settings page. The primary questions are: where do tasks sync, is it working, and what should I do next? Provider choice remains explicit: Off, Nextcloud, LibreSync. Off explains local storage and preserves saved connections. Nextcloud means a personal server; LibreSync means nearby linked devices without a server.

Show current status, last successful exchange, pending changes and Sync Now near the top. Persistent errors and their remedies appear alongside status, with technical details disclosed on demand. A connection test is explicitly read-only and tests saved details; do not imply it saves edits or syncs tasks. Unsaved edits must be visible and cannot be mistaken for the tested connection.

Nextcloud basic setup shows server URL, username and app password, with a concrete HTTPS example and instructions to create an app password in Nextcloud Personal settings → Security. Explain the server base URL (including an installation subfolder), and the shared sync folder. Connection fields retain native platform persistence semantics. Never expose a password in status summaries.

Group automatic/background behavior together. Move folder, encryption, compression, irreversible optimized-format upgrade and verified folder-copy controls into Advanced Nextcloud options. Keep existing confirmation, cancellation, identity and serialization guards. Search destinations must expand hidden controls, without selecting a provider. Active advanced operations and errors remain visible. Optional app-settings synchronization comes after task synchronization controls; keep its consent, bootstrap review and retry visible.

## Alternatives considered

A wizard would help first use but complicate repairing existing connections. A single flattened form keeps everything discoverable but causes the current overload. Recommended: a compact overview with progressive disclosure, retaining native Forms, GTK preferences and Compose controls.

## Boundaries and acceptance

This is native presentation work over existing sync state and operations. No new service, dependency, format, migration, scheduling policy or account requirement. No user data is used for QA. Preserve English/German resources, keyboard and accessibility semantics, preview isolation and provider exclusion.

Verify each platform independently with available build/tests and isolated UI checks. Regression coverage should exercise disclosure/search, saved-versus-edited connection behavior and important status/error visibility. Unavailable Linux runtime, device or hosted-server checks remain explicit; do not claim Verified from source inspection or a shared-core pass.

Sources: Apple Settings HIG (https://developer.apple.com/design/human-interface-guidelines/settings), Nextcloud WebDAV manual (https://docs.nextcloud.com/server/stable/user_manual/en/files/access_webdav.html), existing F-025/F-026/F-043/F-062/F-066/F-075 contracts.
