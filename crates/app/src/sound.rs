// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The optional completion chime: a short bundled clip played through GTK's own media
//! stream when the `completion-sound` setting is on and a change completed tasks.
//!
//! The clip is Kenney's "Interface Sounds" `confirmation_001` (CC0-1.0, see NOTICE.md).
//! Playback needs a GTK media backend (GStreamer on GNOME); when none is available the
//! stream reports an error, which is logged once per attempt and otherwise ignored, so
//! the toast and the task change never depend on audio.
use std::cell::RefCell;

use gtk::gio;
use gtk::prelude::*;
use momentum_core::Message;

const RESOURCE: &str = "/io/github/dan_hart/Momentum/sounds/task-complete.wav";

thread_local! {
    /// The stream currently playing. Holding it keeps the clip alive until it ends; a new
    /// completion replaces it, which is fine for a clip of a third of a second.
    static PLAYING: RefCell<Option<gtk::MediaFile>> = const { RefCell::new(None) };
}

/// Only completions celebrate. Reopening, undo and every other change stay silent.
pub fn celebrates(m: &Message) -> bool {
    matches!(
        m,
        Message::TaskCompleted
            | Message::TaskCompletedArchived
            | Message::TasksCompleted { .. }
            | Message::TasksCompletedArchived { .. }
    )
}

/// Whether this change should make a sound under the current settings.
pub fn should_play(settings: &gio::Settings, m: &Message) -> bool {
    celebrates(m) && settings.boolean("completion-sound")
}

/// Plays the bundled clip once. Never blocks and never fails loudly.
pub fn play_completion() {
    let file = gtk::MediaFile::for_resource(RESOURCE);
    file.connect_error_notify(|f| {
        if let Some(e) = f.error() {
            tracing::warn!("completion sound could not play: {e}");
        }
    });
    file.connect_ended_notify(|f| {
        if f.is_ended() {
            PLAYING.with(|p| {
                let mut p = p.borrow_mut();
                if p.as_ref().is_some_and(|current| current == f) {
                    *p = None;
                }
            });
        }
    });
    file.play();
    PLAYING.with(|p| *p.borrow_mut() = Some(file));
}
