// SPDX-License-Identifier: GPL-3.0-or-later
//! GSettings is a native adapter; the core owns portable validation and durability.
use adw::prelude::*;
use gtk::gio;
use momentum_core::Engine;
use serde_json::{json, Value};

pub const KEYS: &[(&str, &str)] = &[
    ("grouping", "group-by"),
    ("sort", "task-sort"),
    ("direction", "sort-direction"),
    ("upcomingDays", "upcoming-range"),
    ("autoArchive", "auto-archive"),
    ("rowDensity", "row-density"),
    ("colorfulLabels", "colorful-labels"),
    ("completionSoundEnabled", "completion-sound"),
    ("completionSoundChoice", "completion-sound-choice"),
];

pub fn encode(settings: &gio::Settings) -> String {
    let mut values = serde_json::Map::new();
    for &(portable, native) in KEYS {
        let value = match portable {
            "autoArchive" | "colorfulLabels" | "completionSoundEnabled" => json!(settings.boolean(native)),
            "upcomingDays" => json!(settings.string(native).parse::<u32>().unwrap_or(7)),
            "completionSoundChoice" => json!(crate::sound::selected_choice(settings).slug()),
            _ => json!(settings.string(native).as_str()),
        };
        values.insert(portable.into(), value);
    }
    json!({"version":1,"values":values}).to_string()
}

pub fn apply(json: &str, settings: &gio::Settings) -> Result<(), String> {
    let document: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if document["version"] != 1 {
        return Err("Unsupported app settings version".into());
    }
    let values = document["values"].as_object().ok_or("Invalid app settings document")?;
    let effective: Value = serde_json::from_str(&encode(settings)).map_err(|error| error.to_string())?;
    let mut changes = Vec::new();
    for &(portable, native) in KEYS {
        let Some(value) = values.get(portable) else { continue };
        if effective["values"].get(portable) == Some(value) {
            continue;
        }
        let value = match portable {
            "autoArchive" | "colorfulLabels" | "completionSoundEnabled" => {
                value.as_bool().ok_or("Invalid app setting")?.to_variant()
            }
            "upcomingDays" => value.as_u64().ok_or("Invalid Upcoming range")?.to_string().to_variant(),
            _ => value.as_str().ok_or("Invalid app setting")?.to_variant(),
        };
        if settings.value(native) != value {
            changes.push((native, value));
        }
    }
    // Delay a separate transaction object. Delayed mode is permanent on a
    // GSettings instance, so the object bound to native controls stays immediate.
    let schema = settings.settings_schema().ok_or("Missing native settings schema")?;
    let transaction = gio::Settings::new_full(&schema, settings.backend().as_ref(), settings.path().as_deref());
    transaction.delay();
    for (key, value) in changes {
        if let Err(error) = transaction.set_value(key, &value) {
            transaction.revert();
            return Err(error.to_string());
        }
    }
    transaction.apply();
    Ok(())
}

pub fn stage(engine: &Engine, settings: &gio::Settings) -> Result<(), String> {
    stage_document(engine, settings, encode(settings))
}

/// GSettings emits the changed native key. Stage that intent alone so a
/// concurrent remote choice in another key cannot be replaced by stale UI state.
pub fn changed_document(settings: &gio::Settings, key: &str) -> Result<String, String> {
    let Some(&(portable, _)) = KEYS.iter().find(|(_, native)| *native == key) else {
        return Err("Unknown native app setting".into());
    };
    let document: Value = serde_json::from_str(&encode(settings)).map_err(|error| error.to_string())?;
    let value = document["values"][portable].clone();
    Ok(json!({"version":1,"values":{portable:value}}).to_string())
}

pub fn stage_document(engine: &Engine, settings: &gio::Settings, document: String) -> Result<(), String> {
    let previous = engine.portable_settings_json().map_err(|error| error.to_string())?;
    match engine.stage_portable_settings(document) {
        Ok(_) => Ok(()),
        Err(error) => {
            apply(&previous, settings)?;
            Err(error.to_string())
        }
    }
}

pub fn summary(json: &str) -> Result<String, String> {
    use gettextrs::gettext;
    let document: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let values = document["values"].as_object().ok_or("Invalid app settings document")?;
    let mut lines = Vec::new();
    for &(key, _) in KEYS {
        let Some(value) = values.get(key) else {
            continue;
        };
        let name = match key {
            "grouping" => gettext("Group By"),
            "sort" => gettext("Sort By"),
            "direction" => gettext("Sort direction"),
            "upcomingDays" => gettext("Upcoming range"),
            "autoArchive" => gettext("Archive completed tasks immediately"),
            "rowDensity" => gettext("Row density"),
            "colorfulLabels" => gettext("Colorful labels"),
            "completionSoundEnabled" => gettext("Completion sounds"),
            _ => gettext("Completion sound"),
        };
        let display = if let Some(enabled) = value.as_bool() {
            if enabled {
                gettext("On")
            } else {
                gettext("Off")
            }
        } else if let Some(days) = value.as_u64() {
            format!("{days} {}", gettext("days"))
        } else {
            let slug = value.as_str().ok_or("Invalid app setting")?;
            if key == "completionSoundChoice" {
                crate::sound::CompletionSoundChoice::ALL
                    .iter()
                    .find(|choice| choice.slug() == slug)
                    .map(|choice| choice.label())
                    .unwrap_or_else(|| gettext("Unknown sound"))
            } else {
                match slug {
                    "morning-night" => gettext("Morning & Night"),
                    "none" => gettext("None"),
                    "project" => gettext("Project"),
                    "tag" => gettext("Tag"),
                    "estimate" => gettext("Estimate"),
                    "manual" => gettext("Manual"),
                    "title" => gettext("Title"),
                    "due" => gettext("Due date"),
                    "created" => gettext("Created"),
                    "ascending" => gettext("Ascending"),
                    "descending" => gettext("Descending"),
                    "regular" => gettext("Regular"),
                    "compact" => gettext("Compact"),
                    _ => gettext("Unknown"),
                }
            }
        };
        lines.push(format!("{name}: {display}"));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settings() -> gio::Settings {
        let schema = gio::SettingsSchemaSource::default()
            .unwrap()
            .lookup(*crate::config::APP_ID, true)
            .unwrap();
        gio::Settings::new_full(&schema, Some(&gio::memory_settings_backend_new()), None)
    }
    #[test]
    fn portable_scope_excludes_connections_permissions_and_device_controls() {
        let settings = settings();
        settings
            .set_string("nextcloud-server", "private-server-sentinel")
            .unwrap();
        settings.set_boolean("morning-summary-enabled", true).unwrap();
        settings.set_string("row-density", "compact").unwrap();
        let encoded = encode(&settings);
        let document: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(document["values"].as_object().unwrap().len(), 9);
        assert_eq!(document["values"]["rowDensity"], "compact");
        assert!(!encoded.contains("sentinel") && !encoded.contains("summary"));
    }
    #[test]
    fn partial_restore_preserves_missing_values_and_privacy_preferences() {
        let settings = settings();
        settings.set_string("row-density", "compact").unwrap();
        settings
            .set_string("nextcloud-server", "private-server-sentinel")
            .unwrap();
        apply(
            r#"{"version":1,"values":{"grouping":"project","unknown":"ignored"}}"#,
            &settings,
        )
        .unwrap();
        assert_eq!(settings.string("group-by"), "project");
        assert_eq!(settings.string("row-density"), "compact");
        assert_eq!(settings.string("nextcloud-server"), "private-server-sentinel");
    }
    #[test]
    fn durable_failure_rolls_back_native_changes() {
        let settings = settings();
        let directory = tempfile::tempdir().unwrap();
        let engine = Engine::open(directory.path().to_string_lossy().into_owned());
        stage(&engine, &settings).unwrap();
        settings.set_string("task-sort", "title").unwrap();
        stage(&engine, &settings).unwrap();
        std::fs::create_dir(directory.path().join("portable-settings.json.tmp")).unwrap();
        settings.set_string("task-sort", "created").unwrap();
        assert!(stage(&engine, &settings).is_err());
        assert_eq!(settings.string("task-sort"), "title");
    }
    #[test]
    fn restoring_settings_does_not_delay_later_native_device_preferences() {
        let settings = settings();
        let other = gio::Settings::new_full(
            &settings.settings_schema().unwrap(),
            settings.backend().as_ref(),
            settings.path().as_deref(),
        );
        apply(r#"{"version":1,"values":{"grouping":"project"}}"#, &settings).unwrap();
        settings.set_boolean("morning-summary-enabled", true).unwrap();
        assert!(
            other.boolean("morning-summary-enabled"),
            "device controls must still persist immediately"
        );
    }
}
