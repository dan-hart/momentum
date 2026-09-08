// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! System-wide shortcuts through the XDG GlobalShortcuts portal (GNOME 48+, Wayland-native).
//! The desktop asks the user to confirm the bindings the first time; they can be changed in
//! Settings → Keyboard → Application Shortcuts.
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;
use gtk::prelude::*;
use gtk::glib;

use crate::application::MomentumApplication;

pub fn register_global(app: &MomentumApplication) {
    glib::spawn_future_local(glib::clone!(#[weak] app, async move {
        let Ok(gs) = GlobalShortcuts::new().await else { return };
        let Ok(session) = gs.create_session(Default::default()).await else { return };
        let wanted = [
            NewShortcut::new("add-task", "Momentum: add a task").preferred_trigger("CTRL+ALT+t"),
            NewShortcut::new("show", "Momentum: show window").preferred_trigger("CTRL+ALT+m"),
        ];
        if gs.bind_shortcuts(&session, &wanted, None, Default::default()).await.and_then(|r| r.response()).is_err() {
            tracing::info!("GlobalShortcuts portal unavailable or declined"); return;
        }
        let Ok(mut activated) = gs.receive_activated().await else { return };
        while let Some(a) = activated.next().await {
            app.activate();
            if a.shortcut_id() == "add-task" { app.main_window().focus_add(); }
        }
        drop(session);
    }));
}
