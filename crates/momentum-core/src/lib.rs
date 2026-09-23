// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Momentum's application core: everything the app does that is not a widget.
//!
//! The GTK app on Linux and the Swift app on macOS both drive one [`Engine`]. It owns the
//! store (`sp-store`), answers what each view shows ([`Listing`], [`Sidebar`]), performs
//! every change with its undo batch, spawns repeating tasks, finds due reminders, runs the
//! Nextcloud cycle off the main thread and hosts the nearby-device transport. Text the
//! user reads is structured ([`Message`], [`DayLabel`], [`RepeatDescription`]) so each
//! toolkit localises and formats it its own way.
//!
//! With the `ffi` feature the same types carry UniFFI attributes; `crates/momentum-ffi`
//! builds the library the Swift package links against.

pub mod demo;
mod engine;
pub mod ipc;
mod listing;
mod notification_schedule;
pub mod notifications;
#[cfg(feature = "p2p")]
pub mod p2p;
mod sync_cancellation;
pub mod text;
pub mod types;

pub use engine::{Engine, QUICK_MATCHES};
pub use notifications::{
    NotificationAcceptance, NotificationContent, NotificationObservation, NotificationPlan, NotificationRequest,
    ProjectedOccurrence,
};
pub use sync_cancellation::SyncCancellation;
pub use text::QuickAdd;
pub use types::*;

// Re-exported so the apps need only this crate for the model types they still touch.
pub use sp_model;
pub use sp_oplog;
pub use sp_store;
pub use sp_sync;

/// Parsing helpers exposed one by one for the bindings (free functions, not methods).
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn parse_estimate(text: String) -> Option<f64> {
    text::parse_estimate(&text)
}
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn format_estimate(ms: f64) -> String {
    text::format_estimate(ms)
}
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn parse_time(text: String) -> Option<ClockTime> {
    text::parse_time(&text)
}
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn parse_quick_add(text: String) -> QuickAdd {
    text::parse_quick_add(&text)
}
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn hash_word_at(text: String, cursor: u32) -> Option<HashWord> {
    text::hash_word_at(&text, cursor)
}
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn complete_hash_word(text: String, cursor: u32, name: String) -> Option<CompletedText> {
    text::complete_hash_word(&text, cursor, &name)
}
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn day_label(day: String) -> DayLabel {
    text::day_label(&day)
}
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn today() -> String {
    sp_model::today_str()
}
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn now_ms() -> u64 {
    sp_model::now_ms()
}
/// `YYYY-MM-DD` shifted by `days`.
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn day_offset(day: String, days: i64) -> String {
    match sp_model::day_number(&day) {
        Some(n) => sp_model::day_str(n + days),
        None => day,
    }
}

#[cfg(feature = "ffi")]
uniffi::setup_scaffolding!("momentum");

#[cfg(test)]
mod notification_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod transport_tests;

#[cfg(test)]
mod mutation_tests;
