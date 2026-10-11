// SPDX-License-Identifier: GPL-3.0-or-later
package com.codedbydan.momentum.platform

import android.util.AtomicFile
import com.codedbydan.momentum.core.*
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File

internal data class ReminderRecord(
    val request: NotificationRequest,
    val delivered: Boolean = false,
    val actions: Set<String> = emptySet(),
)

/** Bounded private observations. Failed parsing or saves preserve existing bytes. */
internal class ReminderObservations(path: File) {
    private val file = AtomicFile(path)

    fun load(): MutableMap<String, ReminderRecord> {
        if (!file.baseFile.exists() && !File(file.baseFile.path + ".bak").exists()) return linkedMapOf()
        val bytes = file.openRead().use { input ->
            val result = ByteArrayOutputStream()
            val buffer = ByteArray(8_192)
            while (true) {
                val count = input.read(buffer)
                if (count == -1) break
                check(result.size() + count <= MAX_BYTES) { "Reminder observations exceed the storage limit; existing data was preserved" }
                result.write(buffer, 0, count)
            }
            result.toByteArray()
        }
        val json = JSONObject(bytes.toString(Charsets.UTF_8))
        check(json.getInt("version") == 1) { "Unsupported reminder observations; existing data was preserved" }
        val items = json.getJSONArray("records")
        check(items.length() <= MAX_RECORDS) { "Invalid reminder observations; existing data was preserved" }
        return linkedMapOf<String, ReminderRecord>().apply {
            for (index in 0 until items.length()) {
                val item = items.getJSONObject(index)
                val request = decodeRequest(item.getJSONObject("request"))
                val key = ReminderIdentity.key(ReminderIdentity.observation(request))
                val actions = item.getJSONArray("actions").let { array ->
                    check(array.length() <= 2) { "Invalid reminder action receipts; existing data was preserved" }
                    (0 until array.length()).map { array.getString(it) }.toSet()
                }
                check(actions.all { it == "done" || it == "snooze" }) { "Invalid reminder action receipts; existing data was preserved" }
                check(put(key, ReminderRecord(request, item.getBoolean("delivered"), actions)) == null) {
                    "Duplicate reminder observations; existing data was preserved"
                }
            }
        }
    }

    fun save(records: Map<String, ReminderRecord>) {
        // Reject capacity overflow before touching the current file. The caller
        // cancels only the newly unaccepted alarm and reports the persistent error.
        check(records.size <= MAX_RECORDS) { "Reminder observations exceed the record limit; existing data was preserved" }
        val items = JSONArray()
        records.forEach { (key, record) ->
            check(key == ReminderIdentity.key(ReminderIdentity.observation(record.request)))
            check(record.actions.size <= 2 && record.actions.all { it == "done" || it == "snooze" })
            items.put(JSONObject().put("request", encodeRequest(record.request))
                .put("delivered", record.delivered).put("actions", JSONArray(record.actions.toList())))
        }
        val bytes = JSONObject().put("version", 1).put("records", items).toString().toByteArray(Charsets.UTF_8)
        check(bytes.size <= MAX_BYTES) { "Reminder observations exceed the storage limit; existing data was preserved" }
        check(file.baseFile.parentFile!!.exists() || file.baseFile.parentFile!!.mkdirs())
        val stream = file.startWrite()
        try { stream.write(bytes); file.finishWrite(stream) }
        catch (e: Exception) { file.failWrite(stream); throw e }
    }

    private fun encodeRequest(request: NotificationRequest): JSONObject {
        val content = when (val c = request.content) {
            is NotificationContent.Reminder -> JSONObject().put("type", "reminder").put("task", c.taskId).put("title", c.title)
                .put("due", c.dueAtMs?.toString() ?: JSONObject.NULL)
            is NotificationContent.Summary -> JSONObject().put("type", "summary").put("day", c.day).put("tasks", JSONArray(c.taskIds))
                .put("total", c.counts.total.toLong()).put("morning", c.counts.morning.toLong()).put("tonight", c.counts.tonight.toLong())
        }
        return JSONObject().put("id", request.id).put("revision", request.sourceRevision)
            .put("scheduled", request.scheduledAtMs.toString()).put("fire", request.fireAtMs.toString()).put("content", content)
    }

    private fun decodeRequest(json: JSONObject): NotificationRequest {
        fun JSONObject.timestamp(key: String): ULong = getString(key).toULong().also {
            check(it <= Long.MAX_VALUE.toULong()) { "Invalid reminder timestamp; existing data was preserved" }
        }
        fun JSONObject.count(key: String): UInt = getLong(key).also {
            check(it in 0..UInt.MAX_VALUE.toLong()) { "Invalid reminder count; existing data was preserved" }
        }.toUInt()
        val c = json.getJSONObject("content")
        val content = when (c.getString("type")) {
            "reminder" -> NotificationContent.Reminder(c.getString("task"), c.getString("title"), if (c.isNull("due")) null else c.timestamp("due"))
            "summary" -> NotificationContent.Summary(c.getString("day"), MorningSummary(c.count("total"), c.count("morning"), c.count("tonight")),
                c.getJSONArray("tasks").let { a -> (0 until a.length()).map { a.getString(it) } })
            else -> error("Unsupported reminder content; existing data was preserved")
        }
        val request = NotificationRequest(json.getString("id"), json.getString("revision"), json.timestamp("scheduled"), json.timestamp("fire"), content)
        check(request.fireAtMs >= request.scheduledAtMs) { "Invalid reminder timing; existing data was preserved" }
        return request
    }

    companion object {
        const val MAX_RECORDS = 512
        const val MAX_BYTES = 8 * 1_024 * 1_024
    }
}
