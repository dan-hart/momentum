// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Optional completion sounds: ten short bundled clips played through GTK's own media
//! stream when the `completion-sound` setting is on and a change completed tasks.
//!
//! The clips are arranged from VCSL recordings (CC0-1.0, see NOTICE.md).
//! Playback needs a GTK media backend (GStreamer on GNOME); when none is available the
//! stream reports an error, which is logged once per attempt and otherwise ignored, so
//! the toast and the task change never depend on audio.
use std::cell::RefCell;

use gettextrs::gettext;
use gtk::gio;
use gtk::prelude::*;
use momentum_core::Message;

/// Device-local stable identifiers, ordered exactly as the approved numbered catalog.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CompletionSoundChoice {
    WoodPop,
    DoubleCheck,
    RoundedLift,
    PocketPiano,
    StringFlick,
    #[default]
    BrightFinish,
    HarpSnap,
    ClickAndGlow,
    TinyTaDa,
    WarmSpark,
}

impl CompletionSoundChoice {
    pub const ALL: [Self; 10] = [
        Self::WoodPop,
        Self::DoubleCheck,
        Self::RoundedLift,
        Self::PocketPiano,
        Self::StringFlick,
        Self::BrightFinish,
        Self::HarpSnap,
        Self::ClickAndGlow,
        Self::TinyTaDa,
        Self::WarmSpark,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Self::WoodPop => "wood-pop",
            Self::DoubleCheck => "double-check",
            Self::RoundedLift => "rounded-lift",
            Self::PocketPiano => "pocket-piano",
            Self::StringFlick => "string-flick",
            Self::BrightFinish => "bright-finish",
            Self::HarpSnap => "harp-snap",
            Self::ClickAndGlow => "click-and-glow",
            Self::TinyTaDa => "tiny-ta-da",
            Self::WarmSpark => "warm-spark",
        }
    }

    pub fn from_setting(value: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|choice| choice.slug() == value)
            .unwrap_or_default()
    }

    pub fn index(self) -> u32 {
        Self::ALL
            .iter()
            .position(|choice| *choice == self)
            .expect("bounded sound catalog") as u32
    }

    pub fn label(self) -> String {
        match self {
            Self::WoodPop => gettext("1 Wood Pop"),
            Self::DoubleCheck => gettext("2 Double Check"),
            Self::RoundedLift => gettext("3 Rounded Lift"),
            Self::PocketPiano => gettext("4 Pocket Piano"),
            Self::StringFlick => gettext("5 String Flick"),
            Self::BrightFinish => gettext("6 Bright Finish"),
            Self::HarpSnap => gettext("7 Harp Snap"),
            Self::ClickAndGlow => gettext("8 Click and Glow"),
            Self::TinyTaDa => gettext("9 Tiny Ta-Da"),
            Self::WarmSpark => gettext("10 Warm Spark"),
        }
    }

    pub fn resource(self) -> String {
        format!("/io/github/dan_hart/Momentum/sounds/{}.wav", self.slug())
    }
}

/// Unknown future preferences display/play the default without modifying the raw value.
pub fn selected_choice(settings: &gio::Settings) -> CompletionSoundChoice {
    CompletionSoundChoice::from_setting(&settings.string("completion-sound-choice"))
}

#[cfg(not(test))]
thread_local! {
    /// The stream currently playing. Holding it keeps the clip alive until it ends; a new
    /// completion replaces it, which is fine for these clips of less than a second.
    static PLAYING: RefCell<Option<gtk::MediaFile>> = const { RefCell::new(None) };
}

#[cfg(test)]
thread_local! {
    /// GTK tests record requests instead of sending audio to the user's output device.
    static PLAY_REQUESTS: RefCell<Vec<CompletionSoundChoice>> = const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn take_play_requests() -> Vec<CompletionSoundChoice> {
    PLAY_REQUESTS.with(|requests| std::mem::take(&mut *requests.borrow_mut()))
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

/// Plays the selected bundled clip once. Never blocks and never fails loudly.
#[cfg(test)]
pub fn play_completion(choice: CompletionSoundChoice) {
    PLAY_REQUESTS.with(|requests| requests.borrow_mut().push(choice));
}

#[cfg(not(test))]
pub fn play_completion(choice: CompletionSoundChoice) {
    let file = gtk::MediaFile::for_resource(&choice.resource());
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
