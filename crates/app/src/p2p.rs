// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Sync with nearby devices over the local network (LibreSync transport, see docs/P2P.md):
//! engine lifecycle, event drain on the main loop, applying received ops, and the
//! Nearby Devices dialog for linking with a pairing code.
use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib;
use sp_model::{now_ms, AppData};
use sp_p2p::{DeviceInfo, Event, LinkedDevice, P2p, SyncResult};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::window::MomentumWindow;

/// Engine secrets (app key, device certificate and key) live in the keyring next to the
/// Nextcloud password, base64 encoded.
struct KeyringStore;
impl sp_p2p::KeyStore for KeyringStore {
    fn get(&self, name: &str) -> sp_p2p::Result<Option<Vec<u8>>> {
        use base64::Engine as _;
        Ok(crate::keyring::get_named(&format!("p2p-{name}"))
            .and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok()))
    }
    fn set(&self, name: &str, value: &[u8]) -> sp_p2p::Result<()> {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD.encode(value);
        crate::keyring::set_named(&format!("p2p-{name}"), &b64).map_err(|e| sp_p2p::Error::Protocol(e.to_string()))
    }
    fn delete(&self, name: &str) -> sp_p2p::Result<()> {
        crate::keyring::delete_named(&format!("p2p-{name}"));
        Ok(())
    }
}

fn hash_bytes(b: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    b.hash(&mut h);
    h.finish()
}

impl MomentumWindow {
    pub fn p2p_enabled(&self) -> bool {
        // Demo runs never sync, except the Nearby Devices screenshot, which needs a running
        // node (in the demo's temporary directory, with file-backed keys).
        if std::env::var_os("MOMENTUM_DEMO").is_some() {
            return std::env::var_os("MOMENTUM_SCREENSHOT_DEVICES").is_some();
        }
        self.imp().settings.boolean("p2p-enabled")
    }
    fn p2p(&self) -> Option<Rc<P2p>> {
        self.imp().p2p.borrow().clone()
    }
    /// Starts or stops the engine to match the preference.
    pub fn p2p_apply_setting(&self) {
        match (self.p2p_enabled(), self.p2p().is_some()) {
            (true, false) => self.p2p_start(),
            (false, true) => self.p2p_stop(),
            _ => {}
        }
        self.update_sync_button();
    }
    fn p2p_start(&self) {
        let imp = self.imp();
        let (dir, id) = {
            let s = imp.store.borrow();
            (s.dir().join("p2p"), s.meta.client_id.clone())
        };
        let name = glib::host_name().to_string();
        let keys: Box<dyn sp_p2p::KeyStore> = if std::env::var_os("MOMENTUM_DEMO").is_some() {
            match sp_p2p::FileKeyStore::new(dir.join("keys")) {
                Ok(k) => Box::new(k),
                Err(e) => {
                    tracing::warn!("p2p: demo key store: {e}");
                    return;
                }
            }
        } else {
            Box::new(KeyringStore)
        };
        match P2p::start(dir, &id, &name, keys, sp_p2p::DEFAULT_PORT) {
            Ok(p) => {
                let p = Rc::new(p);
                *imp.p2p.borrow_mut() = Some(p.clone());
                // Wake the main loop per event; the drain runs on the GTK thread.
                let weak = glib::SendWeakRef::from(self.downgrade());
                p.events().set_waker(move || {
                    let w = weak.clone();
                    glib::idle_add_once(move || {
                        if let Some(win) = w.upgrade() {
                            win.p2p_drain();
                        }
                    });
                });
                tracing::info!(
                    "p2p: started as {} ({name}), listening on {}",
                    p.identity().device_id,
                    p.listen_addr()
                );
                if let Some(code) = std::env::var("MOMENTUM_P2P_CODE")
                    .ok()
                    .filter(|_| *crate::config::PROFILE == "Devel")
                {
                    p.begin_pairing_with(&code);
                    tracing::info!("p2p: pairing window open with a fixed code (MOMENTUM_P2P_CODE)");
                }
                self.p2p_publish_pending();
                self.p2p_publish_snapshot();
                let periodic = glib::timeout_add_seconds_local(
                    300,
                    glib::clone!(
                        #[weak(rename_to = w)]
                        self,
                        #[upgrade_or]
                        glib::ControlFlow::Break,
                        move || {
                            w.p2p_sync_now();
                            glib::ControlFlow::Continue
                        }
                    ),
                );
                *imp.p2p_periodic.borrow_mut() = Some(periodic);
                self.p2p_schedule_sync(3);
            }
            Err(e) => {
                tracing::warn!("p2p: could not start: {e}");
                self.toast(&format!("{} {e}", gettext("Nearby sync could not start:")));
            }
        }
    }
    fn p2p_stop(&self) {
        let imp = self.imp();
        for slot in [&imp.p2p_periodic, &imp.p2p_sync_debounce, &imp.p2p_snapshot_debounce] {
            if let Some(id) = slot.borrow_mut().take() {
                id.remove();
            }
        }
        if let Some(p) = imp.p2p.borrow_mut().take() {
            p.shutdown();
        }
        tracing::info!("p2p: stopped");
    }
    fn p2p_drain(&self) {
        let Some(p) = self.p2p() else { return };
        let events = p.events();
        loop {
            let e = match events.try_recv() {
                Ok(Some(e)) => e,
                Ok(None) => break,
                Err(err) => {
                    tracing::warn!("p2p: event stream: {err}");
                    break;
                }
            };
            tracing::debug!("p2p: event {e:?}");
            match e {
                Event::ListenerStarted { address } => tracing::info!("p2p: listening on {address}"),
                Event::LinkFinished { device, error, .. } => {
                    p.link_finished(device.as_ref());
                    match (device, error) {
                        (Some(d), _) => {
                            self.toast(&format!("{} {}", gettext("Linked with"), d.identity.user_id));
                            self.p2p_schedule_sync(1);
                        }
                        (None, Some(e)) => {
                            tracing::warn!("p2p: link failed: {e}");
                            self.toast(&gettext("Could not link. Check the code and try again."));
                        }
                        _ => self.toast(&gettext("The other device declined. Check the code and try again.")),
                    }
                    self.p2p_dialog_refresh();
                }
                Event::LinkingRequested { request } => {
                    // The handler approved it while our pairing window was open.
                    if p.devices()
                        .iter()
                        .any(|d| d.device_id == request.device.identity.device_id)
                    {
                        self.toast(&format!(
                            "{} {}",
                            gettext("Linked with"),
                            request.device.identity.user_id
                        ));
                        self.p2p_dialog_refresh();
                        self.p2p_schedule_sync(1);
                    }
                }
                Event::DiscoveryFinished { devices } => {
                    *self.imp().p2p_discovered.borrow_mut() = devices;
                    self.p2p_dialog_refresh();
                }
                Event::SyncFinished { device, result, .. } => match result {
                    SyncResult::Success => {
                        tracing::debug!("p2p: synced with {}", device.identity.device_id);
                        self.p2p_apply_inbox();
                    }
                    SyncResult::Failed(msg) => {
                        tracing::debug!("p2p: sync with {} failed: {msg}", device.identity.device_id)
                    }
                },
                Event::InboundSync { device, applied } => {
                    tracing::debug!("p2p: {} pushed {applied} entries", device.identity.device_id);
                    self.p2p_apply_inbox();
                }
                Event::TaskFinished { .. } => {
                    p.save_state().ok();
                    self.update_sync_button();
                }
                Event::FingerprintChanged { device, .. } => {
                    tracing::warn!("p2p: fingerprint changed for {}", device.identity.device_id);
                    self.toast(&format!(
                        "{} {}",
                        device.identity.user_id,
                        gettext("changed its identity. Unlink it and link again.")
                    ));
                }
                Event::Error { message } => tracing::warn!("p2p: {message}"),
                _ => {}
            }
        }
    }
    /// Applies received ops (and a bootstrap snapshot on first contact) to the store.
    fn p2p_apply_inbox(&self) {
        let Some(p) = self.p2p() else { return };
        let (ops, snap) = p.take_inbox();
        if ops.is_empty() && snap.is_none() {
            return;
        }
        let imp = self.imp();
        let mut changed = false;
        let mut skip_before = 0;
        {
            let mut store = imp.store.borrow_mut();
            if let Some(s) = snap.filter(|_| !store.meta.p2p_bootstrapped) {
                if let Some(data) = s.json().and_then(|j| serde_json::from_slice::<AppData>(&j).ok()) {
                    let added = store.adopt_snapshot(data);
                    if added == usize::MAX {
                        skip_before = s.t; // whole snapshot adopted: ops it already contains are older
                        tracing::info!("p2p: adopted snapshot from {}", s.origin);
                    } else {
                        tracing::info!("p2p: merged {added} entities from {}'s snapshot", s.origin);
                    }
                    changed = true;
                }
            }
            // First contact done either way: later snapshots must not resurrect deleted items.
            store.meta.p2p_bootstrapped = true;
            let mut n = 0;
            for r in ops.into_iter().filter(|r| r.t > skip_before) {
                if let Some((op, action)) = r.decode() {
                    n += store.apply_remote(op, action) as usize;
                }
            }
            if n > 0 {
                tracing::info!("p2p: applied {n} ops from peers");
                changed = true;
            }
            store.save().ok();
        }
        imp.settings.set_int64("last-p2p-ms", now_ms() as i64).ok();
        if changed {
            self.refresh();
        } else {
            self.update_sync_button();
        }
    }
    /// Hands every own pending op to the transport (called from `refresh`).
    pub fn p2p_publish_pending(&self) {
        let Some(p) = self.p2p() else { return };
        let mut any = false;
        {
            let store = self.imp().store.borrow();
            for pd in store.pending.iter().filter(|pd| pd.op.c == store.meta.client_id) {
                if !p.has_seen(&pd.op.id) {
                    any |= p.publish(&pd.op, &pd.action);
                }
            }
        }
        if any {
            self.p2p_schedule_sync(5);
        }
        self.p2p_schedule_snapshot();
    }
    /// Republishes the bootstrap snapshot when the state changed, at most once a minute.
    fn p2p_schedule_snapshot(&self) {
        let imp = self.imp();
        if imp.p2p_snapshot_debounce.borrow().is_some() {
            return;
        }
        let id = glib::timeout_add_seconds_local_once(
            60,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move || {
                    w.imp().p2p_snapshot_debounce.borrow_mut().take();
                    w.p2p_publish_snapshot();
                }
            ),
        );
        *imp.p2p_snapshot_debounce.borrow_mut() = Some(id);
    }
    fn p2p_publish_snapshot(&self) {
        let Some(p) = self.p2p() else { return };
        let imp = self.imp();
        let Ok(json) = serde_json::to_vec(&imp.store.borrow().state) else {
            return;
        };
        let h = hash_bytes(&json);
        if imp.p2p_snapshot_hash.get() == h {
            return;
        }
        imp.p2p_snapshot_hash.set(h);
        p.publish_snapshot(&json);
        self.p2p_schedule_sync(5);
    }
    fn p2p_schedule_sync(&self, secs: u32) {
        let imp = self.imp();
        if let Some(id) = imp.p2p_sync_debounce.borrow_mut().take() {
            id.remove();
        }
        let id = glib::timeout_add_seconds_local_once(
            secs,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                move || {
                    w.imp().p2p_sync_debounce.borrow_mut().take();
                    w.p2p_sync_now();
                }
            ),
        );
        *imp.p2p_sync_debounce.borrow_mut() = Some(id);
    }
    /// Discover linked devices and exchange ops with each; results arrive as events.
    pub fn p2p_sync_now(&self) {
        if let Some(p) = self.p2p() {
            if let Err(e) = p.sync_all() {
                tracing::warn!("p2p: sync: {e}");
            }
        }
    }
    pub fn p2p_status_text(&self) -> Option<String> {
        let p = self.p2p()?;
        let n = p.devices().len();
        let last = self.imp().settings.int64("last-p2p-ms") as u64;
        Some(match (n, last) {
            (0, _) => gettext("No linked devices"),
            (_, 0) => format!("{n} {}", gettext("linked devices, not synced yet")),
            _ => format!("{n} {}, {}", gettext("devices"), crate::window::ago_text(last)),
        })
    }

    // ---- Nearby Devices dialog ---------------------------------------------

    fn p2p_dialog_refresh(&self) {
        if let Some(f) = self.imp().p2p_dialog.borrow().clone() {
            f();
        }
    }
    pub fn show_devices_dialog(&self) {
        if !self.p2p_enabled() {
            self.toast(&gettext("Turn on nearby sync in Preferences first."));
            return;
        }
        let Some(p) = self.p2p() else { return };
        let imp = self.imp();
        let code = p.begin_pairing();
        let page = adw::PreferencesPage::new();

        let me = adw::PreferencesGroup::builder().title(gettext("This Device")).build();
        let me_row = adw::ActionRow::builder()
            .title(glib::markup_escape_text(&p.identity().user_id))
            .subtitle(format!("{} {}", gettext("Listening on port"), p.listen_addr().port()))
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
            let p = p.clone();
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
                let devices = p.devices();
                let discovered: Vec<DeviceInfo> = w
                    .imp()
                    .p2p_discovered
                    .borrow()
                    .iter()
                    .filter(|d| !devices.iter().any(|l| l.device_id == d.identity.device_id))
                    .cloned()
                    .collect();
                if discovered.is_empty() {
                    let row = adw::ActionRow::builder()
                        .title(gettext("Searching…"))
                        .subtitle(gettext("Open this dialog on the other device too"))
                        .build();
                    row.add_prefix(&adw::Spinner::new());
                    nearby_list.append(&row);
                }
                for d in discovered {
                    let addr = d.address.map(|a| a.to_string()).unwrap_or_default();
                    let row = adw::ActionRow::builder()
                        .title(glib::markup_escape_text(&d.identity.user_id))
                        .subtitle(&addr)
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
                        .last_seen
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
                    let p2 = p.clone();
                    unlink.connect_clicked(glib::clone!(
                        #[weak]
                        w,
                        move |_| {
                            p2.unlink(&d.device_id);
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

        let tv = adw::ToolbarView::new();
        tv.add_top_bar(&adw::HeaderBar::new());
        tv.set_content(Some(&page));
        let dialog = adw::Dialog::builder()
            .title(gettext("Nearby Devices"))
            .content_width(480)
            .content_height(640)
            .child(&tv)
            .build();
        // Look for devices while the dialog is open.
        p.discover().ok();
        let ticker = glib::timeout_add_seconds_local(
            4,
            glib::clone!(
                #[strong]
                p,
                move || {
                    p.discover().ok();
                    glib::ControlFlow::Continue
                }
            ),
        );
        let ticker = std::cell::RefCell::new(Some(ticker));
        dialog.connect_closed(glib::clone!(
            #[weak(rename_to = w)]
            self,
            #[strong]
            p,
            move |_| {
                if let Some(t) = ticker.borrow_mut().take() {
                    t.remove();
                }
                p.end_pairing();
                w.imp().p2p_dialog.borrow_mut().take();
                w.update_sync_button();
            }
        ));
        dialog.present(Some(self));
    }
    fn p2p_ask_code(&self, device: &DeviceInfo) {
        let Some(p) = self.p2p() else { return };
        let Some(addr) = device.address else { return };
        let entry = gtk::Entry::builder()
            .placeholder_text("000 000")
            .input_purpose(gtk::InputPurpose::Digits)
            .activates_default(true)
            .build();
        entry.update_property(&[gtk::accessible::Property::Label(&gettext("Pairing code"))]);
        let dlg = adw::AlertDialog::builder()
            .heading(format!("{} {}", gettext("Link with"), device.identity.user_id))
            .body(gettext("Type the pairing code shown in Nearby Devices on that device."))
            .extra_child(&entry)
            .default_response("link")
            .build();
        dlg.add_responses(&[("cancel", &gettext("Cancel")), ("link", &gettext("Link"))]);
        dlg.set_response_appearance("link", adw::ResponseAppearance::Suggested);
        dlg.connect_response(
            None,
            glib::clone!(
                #[weak(rename_to = w)]
                self,
                #[strong]
                entry,
                move |_, r| {
                    if r == "link" {
                        let code = entry.text().to_string();
                        if let Err(e) = p.link(addr, &code) {
                            tracing::warn!("p2p: link request: {e}");
                        } else {
                            w.toast(&gettext("Linking…"));
                        }
                    }
                }
            ),
        );
        dlg.present(Some(self));
    }
}

/// Bytes of a linked device row for tests and the CLI (`mo devices` may reuse it later).
#[allow(dead_code)]
pub fn describe(d: &LinkedDevice) -> String {
    format!("{} ({})", d.name, d.device_id)
}
