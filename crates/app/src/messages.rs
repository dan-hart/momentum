// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The wording of every structured [`Message`] the engine hands back, in the strings the
//! GTK app has always shown (they are what the translations carry).
use gettextrs::gettext;
use momentum_core::Message;

use crate::window::repeat_text;

pub fn text(m: &Message) -> String {
    match m {
        Message::SaveFailed { error } => format!("{} {error}", gettext("Could not save changes:")),
        Message::TaskCompleted => gettext("Task completed"),
        Message::TaskCompletedArchived => gettext("Task completed and archived"),
        Message::TasksCompleted { n } => format!("{n} {}", gettext("tasks completed")),
        Message::TasksCompletedArchived { n } => format!("{n} {}", gettext("tasks completed and archived")),
        Message::TaskDeleted => gettext("Task deleted"),
        Message::TasksDeleted { n } => format!("{n} {}", gettext("tasks deleted")),
        Message::TaskAdded => gettext("Task added"),
        Message::TasksAdded { n } => format!("{n} {}", gettext("tasks added")),
        Message::TaskDuplicated => gettext("Task duplicated"),
        Message::MovedToTonight => gettext("Moved to tonight"),
        Message::MovedToMorning => gettext("Moved to the morning"),
        Message::MovedToToday => gettext("Moved to today"),
        Message::TasksMovedToTonight { n } => format!("{n} {}", gettext("tasks moved to tonight")),
        Message::TasksMovedToMorning { n } => format!("{n} {}", gettext("tasks moved to the morning")),
        Message::TasksMovedToToday { n } => format!("{n} {}", gettext("tasks moved to today")),
        Message::MovedToTomorrow => format!("{} {}", gettext("Moved to"), gettext("tomorrow")),
        Message::MovedToNextWeek => format!("{} {}", gettext("Moved to"), gettext("next week")),
        Message::TasksMovedToTomorrow { n } => format!("{n} {} {}", gettext("tasks moved to"), gettext("tomorrow")),
        Message::TasksMovedToNextWeek { n } => format!("{n} {} {}", gettext("tasks moved to"), gettext("next week")),
        Message::MovedToDay { day } => format!(
            "{} {}",
            gettext("Moved to"),
            crate::window::fmt_day(&momentum_core::text::day_label(day))
        ),
        Message::TasksMovedToDay { n, day } => {
            format!(
                "{n} {} {}",
                gettext("tasks moved to"),
                crate::window::fmt_day(&momentum_core::text::day_label(day))
            )
        }
        Message::PlannedForToday => gettext("Planned for today"),
        Message::PlannedForMorning => gettext("Planned for the morning"),
        Message::PlannedForTonight => gettext("Planned for tonight"),
        Message::RemovedFromToday => gettext("Removed from today"),
        Message::TasksPlannedForToday { n } => format!("{n} {}", gettext("tasks planned for today")),
        Message::MovedToProject { name } => format!("{} {name}", gettext("Moved to")),
        Message::Tagged { name } => format!("{} #{name}", gettext("Tagged")),
        Message::TasksTagged { n } => format!("{n} {}", gettext("tasks tagged")),
        Message::Archived { n } => format!("{n} {}", gettext("completed tasks archived")),
        Message::Snoozed { minutes } => format!("{} {minutes} {}", gettext("Snoozed for"), gettext("minutes")),
        Message::Undone => gettext("Undone"),
        Message::NothingToUndo => gettext("Nothing to undo"),
        Message::ManualOrderOnly => gettext("Switch to Manual Order to rearrange tasks"),
        Message::PickAWeekday => gettext("Pick at least one weekday"),
        Message::NoLongerRepeats { title } => format!("“{title}” {}", gettext("no longer repeats")),
        Message::RepeatSaved { description } => repeat_text(description),
        Message::BackupImported => gettext("Backup imported"),
        Message::BackupExported => gettext("Backup exported"),
        Message::Synced { ops_uploaded } => format!("{} ({ops_uploaded}↑)", gettext("Synced")),
        Message::LinkedWith { name } => format!("{} {name}", gettext("Linked with")),
        Message::Linking => gettext("Linking…"),
        Message::LinkFailed => gettext("Could not link. Check the code and try again."),
        Message::LinkDeclined => gettext("The other device declined. Check the code and try again."),
        Message::IdentityChanged { name } => {
            format!("{name} {}", gettext("changed its identity. Unlink it and link again."))
        }
        Message::NearbyStartFailed { error } => format!("{} {error}", gettext("Nearby sync could not start:")),
    }
}

/// One plain sentence naming a sync failure, for the dialog heading and the banner.
pub fn sync_failure_headline(kind: momentum_core::SyncFailureKind) -> String {
    use momentum_core::SyncFailureKind as K;
    match kind {
        K::Unreachable => gettext("Momentum can’t reach your server"),
        K::Unauthorized => gettext("The server didn’t accept your login"),
        K::ServerError => gettext("The server ran into a problem"),
        K::TimedOut => gettext("Sync took too long to finish"),
        K::EncryptionPasswordMissing => gettext("The sync file is encrypted"),
        K::EncryptionPasswordWrong => gettext("The encryption password doesn’t match"),
        K::RemoteFileDamaged => gettext("The copy on the server is damaged"),
        K::Incompatible => gettext("The copy on the server needs a newer Momentum"),
        K::NothingToStartFrom => gettext("Nothing to sync yet"),
        K::Conflict => gettext("Another device is syncing at the same time"),
        K::Cancelled => gettext("Sync was interrupted"),
        K::UploadIncomplete => gettext("The upload didn’t arrive whole"),
        K::Other => gettext("Sync couldn’t finish"),
    }
}

/// What to do about a sync failure, naming the Preferences controls.
pub fn sync_failure_remedy(kind: momentum_core::SyncFailureKind) -> String {
    use momentum_core::SyncFailureKind as K;
    match kind {
        K::Unreachable => gettext("Check that this computer is online and that the server address is right, then retry. Your tasks are safe on this computer."),
        K::Unauthorized => gettext("Check the user name and app password in Preferences. An app password comes from your Nextcloud security settings."),
        K::ServerError => gettext("The server answered with an error. Retry in a few minutes; if it keeps happening, check the server itself."),
        K::TimedOut => gettext("The connection is slow or unstable. Retry on a faster connection, or turn on “Compress the sync file” in Preferences to make the transfer smaller."),
        K::EncryptionPasswordMissing => gettext("Enter the encryption password you use on your other devices in Preferences."),
        K::EncryptionPasswordWrong => gettext("Enter the same encryption password you use on your other devices. A different password on one device makes the file unreadable there."),
        K::RemoteFileDamaged => gettext("No device can read the copy on the server. On the device with your most recent tasks, choose Replace Server Copy; the damaged copy is kept on the server as sync-data.json.damaged."),
        K::Incompatible => gettext("Update Momentum on this computer. If another app wrote the file, turn off its newer sync format."),
        K::NothingToStartFrom => gettext("This computer has no tasks yet and the server has no copy. Sync from a device that has your tasks first, or import a backup here."),
        K::Conflict => gettext("Two devices uploaded at the same moment. Wait a moment and retry."),
        K::Cancelled => gettext("Retry."),
        K::UploadIncomplete => gettext("The server kept only part of the file, so the previous copy is unchanged and your edits are still waiting here. Retry on a steadier connection."),
        K::Other => gettext("Retry. If it keeps failing, the details below say what the server or this computer reported."),
    }
}
