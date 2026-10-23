// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Native settings wording and routing metadata. No user-entered values are indexed.
use gettextrs::gettext;
use momentum_core::SettingsEntry;
#[allow(non_snake_case)]
pub fn N_(text: &str) -> &str {
    text
}

pub fn category(id: &str) -> String {
    gettext(match id {
        "tasks" => N_("Tasks & Planning"),
        "appearance" => N_("Appearance"),
        "sounds" => N_("Sounds & Feedback"),
        "notifications" => N_("Notifications"),
        "sync" => N_("Sync & Devices"),
        "data" => N_("Data & Backups"),
        "integrations" => N_("App & Integrations"),
        "updates" => N_("Updates"),
        _ => N_("About & Help"),
    })
}

pub fn entries() -> Vec<SettingsEntry> {
    let labels = [
        (
            "updates.check",
            N_("Check for Updates"),
            N_("update install version release"),
        ),
        (
            "updates.automatic",
            N_("Check automatically"),
            N_("automatic app updates"),
        ),
        (
            "updates.restart",
            N_("Restart Momentum"),
            N_("restart reopen installed update"),
        ),
        ("tasks.grouping", N_("Group By"), N_("morning night evening grouping")),
        ("tasks.sort", N_("Sort By"), N_("order title estimate due")),
        ("tasks.sort-direction", N_("Order"), N_("ascending descending")),
        ("tasks.upcoming", N_("Upcoming"), N_("coming up days range")),
        (
            "tasks.day-period-visibility",
            N_("Show Morning and Evening tasks in Today"),
            N_("night hide sidebar"),
        ),
        (
            "tasks.archive",
            N_("Archive completed tasks immediately"),
            N_("done completed"),
        ),
        (
            "tasks.dynamic-messages",
            N_("Dynamic empty-state messages"),
            N_("encouragement celebrate confetti"),
        ),
        (
            "appearance.colors",
            N_("Color-code projects and tags"),
            N_("labels color"),
        ),
        ("appearance.font", N_("Font"), N_("text typography")),
        (
            "appearance.content-size",
            N_("Content size"),
            N_("text size typography"),
        ),
        (
            "appearance.interface-size",
            N_("Interface size"),
            N_("text size typography"),
        ),
        ("appearance.reset", N_("Reset Typography"), N_("font default")),
        (
            "sounds.enabled",
            N_("Play a sound when a task is completed"),
            N_("audio mute"),
        ),
        ("sounds.selection", N_("Selected sound"), N_("audio preview choose")),
        ("sounds.volume", N_("Sound volume"), N_("audio loud quiet")),
        (
            "notifications.morning-summary",
            N_("Morning summary"),
            N_("reminder hour minute"),
        ),
        ("notifications.reminders", N_("Task reminders"), N_("notifications")),
        ("sync.provider", N_("Sync method"), N_("off nextcloud libresync")),
        (
            "sync.nextcloud",
            N_("Server URL"),
            N_("nextcloud password username folder"),
        ),
        ("sync.automatic", N_("Sync automatically"), N_("automatic background")),
        (
            "sync.background",
            N_("Sync in the background"),
            N_("background automatic sync"),
        ),
        ("sync.compression", N_("Compress sync file"), N_("compression")),
        (
            "sync.folder-copy",
            N_("Copy to Momentum folder…"),
            N_("nextcloud migrate"),
        ),
        (
            "sync.devices",
            N_("Manage Devices…"),
            N_("libresync linked devices network nearby"),
        ),
        ("sync.status", N_("Sync status"), N_("retry error connection")),
        (
            "sync.settings",
            N_("Sync app settings"),
            N_("review bootstrap settings transfer"),
        ),
        ("data.tasks-import", N_("Import Backup JSON…"), N_("restore tasks")),
        ("data.tasks-export", N_("Export Backup JSON…"), N_("save tasks")),
        (
            "data.settings-import",
            N_("Import app settings…"),
            N_("restore preferences"),
        ),
        (
            "data.settings-export",
            N_("Export app settings…"),
            N_("save preferences"),
        ),
        ("integrations.shortcuts", N_("Modifier key"), N_("keyboard shortcuts")),
        (
            "integrations.startup",
            N_("Run in the background"),
            N_("login start background"),
        ),
        ("integrations.count", N_("Background task count"), N_("badge status")),
        ("about.app", N_("About Momentum"), N_("version build developer")),
        ("about.source", N_("Source Code"), N_("github open source")),
        ("about.issues", N_("Report an Issue"), N_("bugs help")),
        ("about.license", N_("License"), N_("GPL free software")),
        (
            "about.acknowledgments",
            N_("Acknowledgments"),
            N_("credits third party licenses"),
        ),
    ];
    labels
        .into_iter()
        .map(|(id, title, aliases)| SettingsEntry {
            id: id.into(),
            category: id.split('.').next().unwrap_or("about").into(),
            title: gettext(title),
            description: category(id.split('.').next().unwrap_or("about")),
            aliases: vec![gettext(aliases)],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn discovery_metadata_is_unique_public_and_routes_to_supported_directory() {
        let entries = super::entries();
        let directory = momentum_core::settings_directory();
        let mut ids = std::collections::HashSet::new();
        for entry in &entries {
            assert!(ids.insert(entry.id.clone()));
            assert!(
                directory
                    .iter()
                    .any(|d| d.id == entry.id && d.platforms.contains(&momentum_core::SettingsPlatform::Linux)),
                "{}",
                entry.id
            );
            assert!(!entry.title.is_empty());
            assert!(!entry.title.contains("/Users/"));
        }
        assert!(momentum_core::search_settings("".into(), entries.clone()).is_empty());
        assert!(momentum_core::search_settings("volume".into(), entries.clone())
            .iter()
            .any(|e| e.id == "sounds.volume"));
        assert!(momentum_core::search_settings("background sync".into(), entries)
            .iter()
            .any(|e| e.id == "sync.background"));
    }
}
