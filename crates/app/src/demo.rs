// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! An in-process screenshot of the window over the sample data, which both apps
//! get from `momentum_core::demo`. Used to produce the metainfo screenshots
//! reproducibly:
//! `MOMENTUM_DEMO=1 MOMENTUM_SCREENSHOT=/path.png momentum`.
use gtk::prelude::*;
use gtk::{gdk, glib};

/// Render the window to a PNG after `delay` seconds (time to lay out, or to discover
/// devices), then quit.
pub fn screenshot(win: &gtk::Window, path: std::path::PathBuf, delay: u32) {
    let win = win.clone();
    glib::timeout_add_seconds_local_once(delay, move || {
        let paintable = gtk::WidgetPaintable::new(Some(&win));
        let snapshot = gtk::Snapshot::new();
        let (w, h) = (win.width() as f64, win.height() as f64);
        paintable.snapshot(&snapshot, w, h);
        if let (Some(node), Some(renderer)) = (snapshot.to_node(), win.native().and_then(|n| n.renderer())) {
            let tex = renderer.render_texture(&node, Some(&gtk::graphene::Rect::new(0.0, 0.0, w as f32, h as f32)));
            match tex.save_to_png(&path) {
                Ok(()) => tracing::info!("screenshot saved to {}", path.display()),
                Err(e) => tracing::error!("screenshot failed: {e}"),
            }
        }
        let _ = gdk::Display::default();
        win.application().map(|a| a.quit());
    });
}
