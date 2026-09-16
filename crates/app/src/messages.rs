// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The wording of every structured [`Message`] the engine hands back, in the strings the
//! GTK app has always shown (they are what the translations carry).
use gettextrs::gettext;
use momentum_core::Message;

use crate::window::repeat_text;

pub fn text(m: &Message) -> String {
    match m {
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
