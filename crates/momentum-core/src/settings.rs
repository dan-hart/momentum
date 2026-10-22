// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Offline settings discovery. Native resources supply localized metadata; entered
//! values and secrets never belong in this directory.
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SettingsEntry {
    pub id: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub aliases: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Enum))]
pub enum SettingsPlatform {
    Linux,
    Macos,
    Ios,
    Android,
}
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "ffi", derive(uniffi::Record))]
pub struct SettingsDestination {
    pub id: String,
    pub category: String,
    pub platforms: Vec<SettingsPlatform>,
}

/// Stable destinations represent existing capabilities. Platform layers revalidate
/// runtime availability and retain routing/focus ownership.
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn settings_directory() -> Vec<SettingsDestination> {
    use SettingsPlatform::*;
    let all = &[Linux, Macos, Ios, Android][..];
    let desktop = &[Linux, Macos][..];
    let apple = &[Macos, Ios][..];
    let mut out = Vec::new();
    for (category, names, platforms) in [
        (
            "tasks",
            "grouping sort sort-direction upcoming day-period-visibility archive dynamic-messages",
            all,
        ),
        ("appearance", "theme colors", all),
        ("appearance", "accent", &[Macos, Ios, Android][..]),
        ("appearance", "compact font content-size interface-size reset", all),
        ("sounds", "enabled selection volume", all),
        ("sounds", "haptics", &[Ios, Android][..]),
        ("notifications", "permission reminders morning-summary", all),
        ("notifications", "exact-alarm", &[Android][..]),
        ("notifications", "badge", &[Ios][..]),
        (
            "sync",
            "provider nextcloud libresync automatic background compression devices status settings",
            all,
        ),
        ("sync", "folder-copy", all),
        (
            "data",
            "tasks-import tasks-export settings-import settings-export summary",
            all,
        ),
        ("integrations", "shortcuts", all),
        ("integrations", "startup", desktop),
        ("integrations", "count", &[Linux, Macos, Android][..]),
        ("integrations", "dock menu-bar", &[Macos][..]),
        ("integrations", "system-search", &[Macos, Ios, Android][..]),
        ("integrations", "reminders-import", apple),
        ("about", "app source issues license acknowledgments", all),
    ] {
        for name in names.split_whitespace() {
            out.push(SettingsDestination {
                id: format!("{category}.{name}"),
                category: category.into(),
                platforms: platforms.to_vec(),
            });
        }
    }
    out
}
fn normalized(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .replace('ß', "ss")
}
/// Rank exact titles, title prefixes, title tokens, then aliases/descriptions.
/// Every query token must match. Stable lexical tie breaking and destination-ID
/// deduplication make ordering independent of platform enumeration order.
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn search_settings(query: String, entries: Vec<SettingsEntry>) -> Vec<SettingsEntry> {
    let query = normalized(&query);
    if query.is_empty() {
        return vec![];
    }
    let tokens: Vec<_> = query.split_whitespace().collect();
    let mut matches: Vec<_> = entries
        .into_iter()
        .filter_map(|entry| {
            let title = normalized(&entry.title);
            let metadata = normalized(&format!(
                "{} {} {}",
                entry.title,
                entry.aliases.join(" "),
                entry.description
            ));
            if !tokens.iter().all(|token| metadata.contains(token)) {
                return None;
            }
            let rank = if title == query {
                0
            } else if title.starts_with(&query) {
                1
            } else if tokens.iter().all(|token| title.contains(token)) {
                2
            } else {
                3
            };
            Some((rank, normalized(&entry.category), title, entry))
        })
        .collect();
    matches.sort_by(|a, b| {
        (&a.0, &a.1, &a.2, &a.3.id, &a.3.description, &a.3.aliases).cmp(&(
            &b.0,
            &b.1,
            &b.2,
            &b.3.id,
            &b.3.description,
            &b.3.aliases,
        ))
    });
    let mut seen = HashSet::new();
    matches
        .into_iter()
        .filter_map(|(_, _, _, entry)| seen.insert(entry.id.clone()).then_some(entry))
        .collect()
}
