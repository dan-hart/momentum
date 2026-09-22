// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Pure state and wording for the GNOME Background Apps status line.

use gettextrs::{gettext, ngettext};
use momentum_core::TaskCountMode;

/// The Linux preference choices, kept separate from their localized labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundCountMode {
    DueToday,
    TodayIncludingOverdue,
    Off,
}

impl BackgroundCountMode {
    pub const VALUES: [&'static str; 3] = ["due-today", "today-including-overdue", "off"];

    pub fn from_setting(value: &str) -> Self {
        match value {
            "today-including-overdue" => Self::TodayIncludingOverdue,
            "off" => Self::Off,
            _ => Self::DueToday,
        }
    }

    pub fn task_count_mode(self) -> TaskCountMode {
        match self {
            Self::DueToday => TaskCountMode::DueToday,
            Self::TodayIncludingOverdue => TaskCountMode::TodayIncludingOverdue,
            Self::Off => TaskCountMode::None,
        }
    }
}

/// A request identity prevents a late completion from acknowledging a newer value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundStatusRequest {
    pub id: u64,
    pub message: String,
}

/// Serializes portal requests and coalesces refresh bursts to the newest value.
#[derive(Debug, Default)]
pub struct BackgroundStatusController {
    desired: Option<String>,
    inflight: Option<BackgroundStatusRequest>,
    last_acknowledged: Option<String>,
    next_id: u64,
}

impl BackgroundStatusController {
    pub fn desired(&self) -> Option<&str> {
        self.desired.as_deref()
    }

    pub fn inflight(&self) -> Option<&BackgroundStatusRequest> {
        self.inflight.as_ref()
    }

    pub fn last_acknowledged(&self) -> Option<&str> {
        self.last_acknowledged.as_deref()
    }

    pub fn refresh(&mut self, desired: String, send: &mut impl FnMut(BackgroundStatusRequest)) {
        self.desired = Some(desired);
        self.dispatch_if_needed(send);
    }

    pub fn succeeded(&mut self, id: u64, send: &mut impl FnMut(BackgroundStatusRequest)) {
        let Some(request) = self.inflight.as_ref().filter(|request| request.id == id).cloned() else {
            return;
        };
        self.inflight = None;
        self.last_acknowledged = Some(request.message);
        self.dispatch_if_needed(send);
    }

    pub fn failed(&mut self, id: u64) {
        if self.inflight.as_ref().is_some_and(|request| request.id == id) {
            self.inflight = None;
        }
    }

    fn dispatch_if_needed(&mut self, send: &mut impl FnMut(BackgroundStatusRequest)) {
        if self.inflight.is_some() {
            return;
        }
        let Some(message) = self.desired.as_ref() else {
            return;
        };
        if self.last_acknowledged.as_ref() == Some(message) {
            return;
        }
        let request = BackgroundStatusRequest {
            id: self.next_id,
            message: message.clone(),
        };
        self.next_id = self.next_id.wrapping_add(1);
        self.inflight = Some(request.clone());
        send(request);
    }
}

pub fn format_status(mode: BackgroundCountMode, count: u32) -> String {
    if mode == BackgroundCountMode::Off {
        return gettext("Momentum is running");
    }
    if count == 0 {
        return gettext("All done for today");
    }
    let message = match mode {
        BackgroundCountMode::DueToday => ngettext("1 task due today", "%d tasks due today", count),
        BackgroundCountMode::TodayIncludingOverdue => ngettext("1 open task in Today", "%d open tasks in Today", count),
        BackgroundCountMode::Off => unreachable!("handled above"),
    };
    message.replace("%d", &count.to_string())
}

#[cfg(test)]
pub fn format_status_with(
    mode: BackgroundCountMode,
    count: u32,
    mut singular: impl FnMut(&str) -> String,
    mut plural: impl FnMut(&str, &str, u32) -> String,
) -> String {
    if mode == BackgroundCountMode::Off {
        return singular("Momentum is running");
    }
    if count == 0 {
        return singular("All done for today");
    }
    let message = match mode {
        BackgroundCountMode::DueToday => plural("1 task due today", "%d tasks due today", count),
        BackgroundCountMode::TodayIncludingOverdue => plural("1 open task in Today", "%d open tasks in Today", count),
        BackgroundCountMode::Off => unreachable!("handled above"),
    };
    message.replace("%d", &count.to_string())
}

/// The portal accepts at most 96 Unicode scalar values.
pub fn truncate_status(message: &str) -> String {
    if message.chars().count() <= 96 {
        return message.to_string();
    }
    let mut truncated: String = message.chars().take(95).collect();
    truncated.push('…');
    truncated
}
