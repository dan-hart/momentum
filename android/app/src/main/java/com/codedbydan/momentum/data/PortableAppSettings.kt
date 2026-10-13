// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.data

import androidx.datastore.preferences.core.*
import com.codedbydan.momentum.core.GroupBy
import com.codedbydan.momentum.core.SortDirection
import com.codedbydan.momentum.core.SortKey
import com.codedbydan.momentum.core.Preferences
import com.codedbydan.momentum.platform.CompletionSoundChoice
import org.json.JSONObject

/** Explicit portable allowlist. Never serialize the native preference store. */
internal object PortableAppSettings {
    fun encode(settings: AppSettings, preferences: Preferences): String = JSONObject().put("version", 1)
        .put("values", JSONObject().put("grouping", group(preferences.groupBy)).put("sort", sort(preferences.sort))
            .put("direction", if(preferences.direction == SortDirection.ASCENDING) "ascending" else "descending")
            .put("upcomingDays", preferences.upcomingDays.toInt()).put("autoArchive", preferences.autoArchive)
            .put("rowDensity", if(settings.compact) "compact" else "regular").put("colorfulLabels", settings.colorfulLabels)
            .put("completionSoundEnabled", settings.completionSound).put("completionSoundChoice", settings.completionSoundChoice.key)).toString()

    fun changedDocument(previous: String, changed: String): String {
        val old = values(previous); val next = values(changed); val delta = JSONObject()
        next.keys().forEach { key -> if(!old.has(key) || old.get(key) != next.get(key)) delta.put(key, next.get(key)) }
        return JSONObject().put("version", 1).put("values", delta).toString()
    }

    /** Validate the complete known subset before changing even one native value. */
    fun apply(json: String, target: MutablePreferences) {
        val values = values(json)
        val updates = mutableListOf<() -> Unit>()
        fun string(key: String, allowed: List<String>, native: String, map: (String) -> String = { it }) {
            if(values.has(key)) {
                val value = values.get(key)
                require(value is String && value in allowed) { "Invalid app setting: $key" }
                val mapped = map(value)
                val nativeKey = stringPreferencesKey(native)
                val equivalent = native == "completion-sound-choice" && CompletionSoundChoice.resolve(target[nativeKey]).key == mapped
                if(!equivalent) updates += { target[nativeKey] = mapped }
            }
        }
        fun boolean(key: String, native: String) {
            if(values.has(key)) {
                val value = values.get(key); require(value is Boolean) { "Invalid app setting: $key" }
                updates += { target[booleanPreferencesKey(native)] = value }
            }
        }
        string("grouping", listOf("morning-night", "none", "project", "tag", "estimate"), "group") { when(it) {
            "morning-night" -> GroupBy.MORNING_NIGHT; "none" -> GroupBy.NONE; "project" -> GroupBy.PROJECT; "tag" -> GroupBy.TAG; else -> GroupBy.ESTIMATE
        }.name }
        string("sort", listOf("manual", "title", "due", "estimate", "created"), "sort") { SortKey.valueOf(it.uppercase()).name }
        string("direction", listOf("ascending", "descending"), "direction") { SortDirection.valueOf(it.uppercase()).name }
        if(values.has("rowDensity")) {
            val value = values.get("rowDensity")
            require(value is String && value in listOf("regular", "compact")) { "Invalid app setting: rowDensity" }
            updates += { target[booleanPreferencesKey("compact")] = value == "compact" }
        }
        if(values.has("upcomingDays")) {
            val value = values.get("upcomingDays")
            require(value is Number && value.toDouble() in listOf(7.0, 30.0)) { "Invalid app setting: upcomingDays" }
            updates += { target[intPreferencesKey("upcoming-days")] = value.toInt() }
        }
        boolean("autoArchive", "auto-archive"); boolean("colorfulLabels", "colorful-labels")
        boolean("completionSoundEnabled", "completion-sound")
        string("completionSoundChoice", CompletionSoundChoice.entries.map { it.key }, "completion-sound-choice")
        updates.forEach { it() }
    }
    fun values(json: String): JSONObject {
        val doc = JSONObject(json); val version = doc.get("version")
        require(version is Number && version.toDouble() == 1.0) { "Unsupported app settings version" }
        return doc.getJSONObject("values")
    }
    private fun group(value: GroupBy): String = when(value) { GroupBy.MORNING_NIGHT -> "morning-night"; else -> value.name.lowercase() }
    private fun sort(value: SortKey): String = value.name.lowercase()
}
