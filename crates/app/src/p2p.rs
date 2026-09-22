// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Sync with nearby devices over the local network (LibreSync transport, see docs/P2P.md).
//! The transport itself is `momentum_core::p2p`; this file supplies the keyring-backed
//! secret store, marshals the engine's events to the GTK main loop, and draws the Nearby
//! Devices dialog for linking with a pairing code.
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use momentum_core::p2p::{NearbyDevice, P2pDelegate, P2pEvent, SecretStore};
use momentum_core::Message;
use std::rc::Rc;
use std::sync::Arc;

use crate::window::MomentumWindow;

fn nearby_devices_dialog(page: &adw::PreferencesPage) -> adw::Dialog {
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&adw::HeaderBar::new());
    tv.set_content(Some(page));
    let dialog = adw::Dialog::builder()
        .title(gettext("Nearby Devices"))
        .content_width(480)
        .content_height(640)
        .child(&tv)
        .build();
    crate::typography::register_interface_root(&dialog);
    dialog
}

#[cfg(test)]
pub fn nearby_devices_dialog_for_test() -> adw::Dialog {
    nearby_devices_dialog(&adw::PreferencesPage::new())
}

/// Engine secrets (app key, device certificate and key) live in the keyring next to the
/// Nextcloud password. Values are already base64 text; the core encodes and decodes them.
struct KeyringSecrets;
impl SecretStore for KeyringSecrets {
    fn get(&self, name: String) -> Option<String> {
        crate::keyring::get_named(&format!("p2p-{name}"))
    }
    fn set(&self, name: String, value: String) -> bool {
        crate::keyring::set_named(&format!("p2p-{name}"), &value).is_ok()
    }
    fn delete(&self, name: String) {
        crate::keyring::delete_named(&format!("p2p-{name}"));
    }
}

/// Marshals engine events (raised from a background thread) to the GTK main loop.
struct WindowDelegate(glib::SendWeakRef<MomentumWindow>);
impl P2pDelegate for WindowDelegate {
    fn on_event(&self, event: P2pEvent) {
        let w = self.0.clone();
        glib::idle_add_once(move || {
            let Some(win) = w.upgrade() else { return };
            match event {
                P2pEvent::Started { port } => tracing::info!("p2p: listening on port {port}"),
                P2pEvent::StoreChanged => win.refresh(),
                P2pEvent::StatusChanged => win.update_sync_button(),
                P2pEvent::SyncStarted if win.p2p_enabled() => {
                    win.imp().nearby_syncing.set(true);
                    win.update_sync_button();
                }
                P2pEvent::SyncCompleted { error } if win.p2p_enabled() => {
                    win.imp().nearby_syncing.set(false);
                    win.set_sync_error(error);
                    win.update_background_status();
                }
                P2pEvent::SyncStarted | P2pEvent::SyncCompleted { .. } => {}
                P2pEvent::DevicesChanged | P2pEvent::DiscoveryUpdated => win.p2p_dialog_refresh(),
                P2pEvent::Notice { message } => win.toast(&crate::messages::text(&message)),
            }
        });
    }
}

impl MomentumWindow {
    pub fn p2p_enabled(&self) -> bool {
        std::env::var_os("MOMENTUM_DEMO").is_none() && crate::prefs::sync_method(&self.imp().settings) == "libresync"
    }

    /// Starts or stops the engine to match the preference.
    pub fn p2p_apply_setting(&self) {
        match (self.p2p_enabled(), self.engine().p2p_running()) {
            (true, false) if !self.imp().syncing.get() => self.p2p_start(),
            (false, true) => self.engine().p2p_stop(),
            _ => {}
        }
        self.update_sync_button();
    }
    fn p2p_start(&self) {
        let engine = self.engine();
        let delegate: Arc<dyn P2pDelegate> = Arc::new(WindowDelegate(glib::SendWeakRef::from(self.downgrade())));
        let secrets: Arc<dyn SecretStore> = Arc::new(KeyringSecrets);
        let name = glib::host_name().to_string();
        match engine.p2p_start(delegate, secrets, name.clone(), None) {
            Ok(info) => {
                tracing::info!(
                    "p2p: started as {} ({name}), listening on {}",
                    info.device_id,
                    info.port
                );
                if let Some(code) = std::env::var("MOMENTUM_P2P_CODE")
                    .ok()
                    .filter(|_| *crate::config::PROFILE == "Devel")
                {
                    self.engine().p2p_begin_pairing_with(code);
                    tracing::info!("p2p: pairing window open with a fixed code (MOMENTUM_P2P_CODE)");
                }
            }
            Err(e) => {
                tracing::warn!("p2p: could not start: {e}");
                self.set_sync_error(Some(crate::messages::text(&Message::NearbyStartFailed {
                    error: e.to_string(),
                })));
            }
        }
    }
    /// Discover linked devices and exchange ops with each; results arrive as events.
    pub fn p2p_sync_now(&self) {
        if !self.p2p_enabled() || self.imp().syncing.get() || self.imp().nearby_syncing.get() {
            return;
        }
        if !self.engine().p2p_running() {
            self.p2p_apply_setting();
        }
        self.engine().p2p_sync_now();
    }
    // ---- Nearby Devices dialog ---------------------------------------------

    fn p2p_dialog_refresh(&self) {
        if let Some(f) = self.imp().p2p_dialog.borrow().clone() {
            f();
        }
    }
    pub fn show_devices_dialog(&self) {
        if !self.p2p_enabled() {
            self.toast(&gettext("Select LibreSync in Preferences first."));
            return;
        }
        let engine = self.engine();
        if !engine.p2p_running() {
            return;
        }
        let imp = self.imp();
        let Some(code) = engine.p2p_begin_pairing() else {
            return;
        };
        let Some(info) = engine.p2p_info() else { return };
        let page = adw::PreferencesPage::new();

        let me = adw::PreferencesGroup::builder().title(gettext("This Device")).build();
        let me_row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&info.device_name))
            .subtitle(format!("{} {}", gettext("Listening on port"), info.port))
            .build();
        me_row.add_prefix(&gtk::Image::from_icon_name("computer-symbolic"));
        me.add(&me_row);
        let code_row = adw::ActionRow::builder()
            .title(gettext("Pairing code"))
            .subtitle(gettext("Enter it on the other device within five minutes"))
            .build();
        let code_label = gtk::Label::builder()
            .label(format!("{} {}", &code[..3], &code[3..]))
            .css_classes(["title-2", "numeric"])
            .selectable(true)
            .build();
        code_label.update_property(&[gtk::accessible::Property::Label(&format!(
            "{} {}",
            gettext("Pairing code"),
            code
        ))]);
        code_row.add_suffix(&code_label);
        me.add(&code_row);
        page.add(&me);

        let nearby = adw::PreferencesGroup::builder()
            .title(gettext("Nearby"))
            .description(gettext(
                "Devices running Momentum on this network. Link with the code shown on the other device.",
            ))
            .build();
        let nearby_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        nearby.add(&nearby_list);
        page.add(&nearby);

        let linked = adw::PreferencesGroup::builder()
            .title(gettext("Linked Devices"))
            .build();
        let linked_list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        linked.add(&linked_list);
        page.add(&linked);

        let rebuild = {
            let w = self.downgrade();
            let nearby_list = nearby_list.clone();
            let linked_list = linked_list.clone();
            Rc::new(move || {
                let Some(w) = w.upgrade() else { return };
                while let Some(c) = nearby_list.first_child() {
                    nearby_list.remove(&c);
                }
                while let Some(c) = linked_list.first_child() {
                    linked_list.remove(&c);
                }
                let engine = w.engine();
                let devices = engine.p2p_devices();
                let discovered = engine.p2p_discovered();
                if discovered.is_empty() {
                    let row = adw::ActionRow::builder()
                        .title(gettext("Searching…"))
                        .subtitle(gettext("Open this dialog on the other device too"))
                        .build();
                    row.add_prefix(&adw::Spinner::new());
                    nearby_list.append(&row);
                }
                for d in discovered {
                    let row = adw::ActionRow::builder()
                        .title(glib::markup_escape_text(&d.name))
                        .subtitle(d.address.clone().unwrap_or_default())
                        .build();
                    let link = gtk::Button::builder()
                        .label(gettext("Link…"))
                        .valign(gtk::Align::Center)
                        .css_classes(["suggested-action"])
                        .build();
                    link.connect_clicked(glib::clone!(
                        #[weak]
                        w,
                        move |_| w.p2p_ask_code(&d)
                    ));
                    row.add_suffix(&link);
                    nearby_list.append(&row);
                }
                if devices.is_empty() {
                    linked_list.append(
                        &adw::ActionRow::builder()
                            .title(gettext("None yet"))
                            .css_classes(["dim-label"])
                            .build(),
                    );
                }
                for d in devices {
                    let seen = d
                        .last_seen_ms
                        .map(|t| format!("{} {}", gettext("Last synced"), crate::window::ago_text(t)))
                        .unwrap_or_else(|| gettext("Not synced yet"));
                    let row = adw::ActionRow::builder()
                        .title(glib::markup_escape_text(&d.name))
                        .subtitle(seen)
                        .build();
                    row.add_prefix(&gtk::Image::from_icon_name("computer-symbolic"));
                    let unlink = gtk::Button::builder()
                        .label(gettext("Unlink"))
                        .valign(gtk::Align::Center)
                        .css_classes(["flat", "destructive-action"])
                        .build();
                    unlink.connect_clicked(glib::clone!(
                        #[weak]
                        w,
                        #[strong]
                        d,
                        move |_| {
                            w.engine().p2p_unlink(d.device_id.clone());
                            w.p2p_dialog_refresh();
                            w.update_sync_button();
                        }
                    ));
                    row.add_suffix(&unlink);
                    linked_list.append(&row);
                }
            })
        };
        rebuild();
        *imp.p2p_dialog.borrow_mut() = Some(rebuild.clone() as Rc<dyn Fn()>);

        let dialog = nearby_devices_dialog(&page);
        // `p2p_begin_pairing` already started discovery on a timer; it stops in `p2p_end_pairing`.
        dialog.connect_closed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            move |_| {
                w.engine().p2p_end_pairing();
                w.imp().p2p_dialog.borrow_mut().take();
                w.update_sync_button();
            }
        ));
        dialog.present(Some(self));
    }
    fn p2p_ask_code(&self, device: &NearbyDevice) {
        let Some(addr) = device.address.clone() else { return };
        let entry = gtk::Entry::builder()
            .placeholder_text("000 000")
            .input_purpose(gtk::InputPurpose::Digits)
            .activates_default(true)
            .build();
        entry.update_property(&[gtk::accessible::Property::Label(&gettext("Pairing code"))]);
        let dlg = adw::AlertDialog::builder()
            .heading(format!("{} {}", gettext("Link with"), device.name))
            .body(gettext("Type the pairing code shown in Nearby Devices on that device."))
            .extra_child(&entry)
            .default_response("link")
            .build();
        crate::typography::register_interface_root(&dlg);
        dlg.add_responses(&[("cancel", &gettext("Cancel")), ("link", &gettext("Link"))]);
        dlg.set_response_appearance("link", adw::ResponseAppearance::Suggested);
        dlg.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[strong]
                entry,
                #[strong]
                addr,
                move |_, r| {
                    if r == "link" {
                        let code = entry.text().to_string();
                        match w.engine().p2p_link(addr.clone(), code) {
                            Ok(()) => w.toast(&crate::messages::text(&Message::Linking)),
                            Err(e) => tracing::warn!("p2p: link request: {e}"),
                        }
                    }
                }
            ),
        );
        dlg.present(Some(self));
    }
}
