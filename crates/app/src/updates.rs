// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Flatpak's own signed-origin updater. No host command execution or sandbox escape.
use crate::update_policy::{self, UpdateState};
use adw::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use std::{cell::RefCell, collections::HashMap};

const BUS: &str = "org.freedesktop.portal.Flatpak";
const ROOT: &str = "/org/freedesktop/portal/Flatpak";
const MONITOR: &str = "org.freedesktop.portal.Flatpak.UpdateMonitor";

#[derive(Default)]
struct Service {
    connection: Option<gio::DBusConnection>,
    subscription: Option<gio::SignalSubscription>,
    path: Option<String>,
    generation: u64,
    state: Option<UpdateState>,
    rows: Vec<(
        glib::WeakRef<adw::ActionRow>,
        glib::WeakRef<adw::ButtonRow>,
        glib::WeakRef<adw::ButtonRow>,
    )>,
    started: bool,
    stop_after_install: bool,
    latest: Option<String>,
    latest_rows: Vec<glib::WeakRef<adw::ActionRow>>,
}
thread_local! { static SERVICE: RefCell<Service> = RefCell::new(Service::default()); }

fn supported() -> bool {
    update_policy::supported(
        *crate::config::APP_ID,
        std::env::var("FLATPAK_ID").ok().as_deref(),
        std::env::var_os("MOMENTUM_DEMO").is_some(),
    )
}
fn publish(state: UpdateState) {
    let previous = SERVICE.with_borrow(|s| s.state.clone());
    SERVICE.with_borrow_mut(|s| {
        s.state = Some(state.clone());
        s.rows.retain(|(row, update, cancel)| {
            let (Some(row), Some(update), Some(cancel)) = (row.upgrade(), update.upgrade(), cancel.upgrade()) else { return false; };
            row.set_subtitle(&match state {
                UpdateState::Idle => gettext("Updates use your existing signed Flatpak repository."),
                UpdateState::Checking => gettext("Monitoring continues until Stop or Momentum quits. Flatpak may take 30 minutes to check. Open Software to check now."),
                UpdateState::Available => gettext("An update is available from your Flatpak repository."),
                UpdateState::RestartRequired => gettext("Update installed. Quit Momentum with Ctrl+Q and reopen it to use the new version."),
                UpdateState::Installing(percent) => gettext("Installing update… {percent}%").replace("{percent}", &percent.to_string()),
                UpdateState::Current => gettext("Your installed Flatpak is up to date."),
                UpdateState::Cancelling => gettext("Cancelling update…"),
                UpdateState::Cancelled => gettext("Update cancelled. You can check again."),
                UpdateState::Failed => gettext("Update failed or is unavailable. Try again, or use your system software app. New permissions require a system update."),
            });
            update.set_sensitive(state == UpdateState::Available);
            cancel.set_sensitive(!matches!(state, UpdateState::Idle | UpdateState::Cancelled | UpdateState::Failed | UpdateState::Cancelling));
            true
        });
    });
    let settings = gio::Settings::new(*crate::config::APP_ID);
    if update_policy::close_after_terminal(
        &state,
        previous.as_ref(),
        settings.boolean("automatic-update-checks"),
        SERVICE.with_borrow(|s| s.stop_after_install),
    ) {
        close_monitor();
    }
}
fn close_monitor() {
    let previous = SERVICE.with_borrow_mut(|s| {
        s.generation += 1;
        (s.connection.take(), s.subscription.take(), s.path.take())
    });
    if let (Some(connection), subscription, path) = previous {
        drop(subscription);
        if let Some(path) = path {
            // Closing explicitly cancels any ongoing installation.
            connection.call(
                Some(BUS),
                &path,
                MONITOR,
                "Close",
                None,
                None,
                gio::DBusCallFlags::NONE,
                5000,
                gio::Cancellable::NONE,
                |_| {},
            );
        }
    }
}
fn cancel() {
    let (connection, subscription, path, generation) = SERVICE.with_borrow_mut(|s| {
        s.generation += 1;
        (s.connection.take(), s.subscription.take(), s.path.take(), s.generation)
    });
    drop(subscription);
    let (Some(connection), Some(path)) = (connection, path) else {
        publish(UpdateState::Cancelled);
        return;
    };
    publish(UpdateState::Cancelling);
    glib::spawn_future_local(async move {
        let result = connection
            .call_future(
                Some(BUS),
                &path,
                MONITOR,
                "Close",
                None,
                None,
                gio::DBusCallFlags::NONE,
                30000,
            )
            .await;
        if SERVICE.with_borrow(|s| s.generation == generation) {
            publish(if result.is_ok() {
                UpdateState::Cancelled
            } else {
                UpdateState::Failed
            });
        }
    });
}
fn field(values: &HashMap<String, glib::Variant>, key: &str) -> Option<String> {
    values.get(key).and_then(|value| value.get())
}
pub fn check() {
    if !supported() {
        return;
    }
    // A second monitor resets the portal's delayed poll instead of checking now.
    if !SERVICE.with_borrow(|s| update_policy::should_create_monitor(s.path.is_some(), s.state.as_ref())) {
        return;
    }
    close_monitor();
    publish(UpdateState::Checking);
    let generation = SERVICE.with_borrow(|s| s.generation);
    glib::spawn_future_local(async move {
        let result = async {
            let connection = gio::bus_get_future(gio::BusType::Session).await?;
            let sender = connection
                .unique_name()
                .expect("session bus unique name")
                .trim_start_matches(':')
                .replace('.', "_");
            let token = format!("momentum{generation}");
            let path = format!("{ROOT}/update_monitor/{sender}/{token}");
            // Subscribe before creating the monitor: its first signal can precede the reply.
            let subscription = connection.subscribe_to_signal(
                Some(BUS),
                Some(MONITOR),
                None,
                Some(&path),
                None,
                gio::DBusSignalFlags::NONE,
                move |signal| {
                    let parameters = signal.parameters;
                    if SERVICE.with_borrow(|s| s.generation != generation) {
                        return;
                    }
                    let Some((values,)) = parameters.get::<(HashMap<String, glib::Variant>,)>() else {
                        publish(UpdateState::Failed);
                        return;
                    };
                    match signal.signal_name {
                        "UpdateAvailable" => {
                            let remote = field(&values, "remote-commit");
                            SERVICE.with_borrow_mut(|s| {
                                s.latest = remote.clone();
                                s.latest_rows.retain(|row| {
                                    let Some(row) = row.upgrade() else {
                                        return false;
                                    };
                                    row.set_subtitle(remote.as_deref().unwrap_or(&gettext("Not checked")));
                                    true
                                });
                            });
                            if !SERVICE.with_borrow(|s| {
                                matches!(s.state, Some(UpdateState::Installing(_) | UpdateState::Cancelling))
                            }) {
                                publish(update_policy::available(
                                    field(&values, "running-commit").as_deref(),
                                    field(&values, "local-commit").as_deref(),
                                    remote.as_deref(),
                                ));
                            }
                        }
                        "Progress" => {
                            if !SERVICE.with_borrow(|s| update_policy::accept_progress(s.state.as_ref())) {
                                return;
                            }
                            let Some(status) = values.get("status").and_then(|v| v.get()) else {
                                publish(UpdateState::Failed);
                                return;
                            };
                            let percent = values.get("progress").and_then(|v| v.get()).unwrap_or(0);
                            publish(update_policy::progress(
                                status,
                                percent,
                                field(&values, "error").as_deref(),
                            ));
                        }
                        _ => {}
                    }
                },
            );
            if SERVICE.with_borrow(|s| s.generation != generation) {
                drop(subscription);
                return Ok::<(), glib::Error>(());
            }
            SERVICE.with_borrow_mut(|s| {
                s.connection = Some(connection.clone());
                s.subscription = Some(subscription);
                s.path = Some(path.clone());
            });
            let options = HashMap::from([("handle_token", token.to_variant())]);
            let reply = connection
                .call_future(
                    Some(BUS),
                    ROOT,
                    BUS,
                    "CreateUpdateMonitor",
                    Some(&(options,).to_variant()),
                    None,
                    gio::DBusCallFlags::NONE,
                    30000,
                )
                .await?;
            let returned = reply
                .child_value(0)
                .get::<glib::variant::ObjectPath>()
                .map(|p| p.to_string());
            if returned.as_deref() != Some(&path) {
                return Err(glib::Error::new(
                    gio::IOErrorEnum::InvalidData,
                    "Unexpected update monitor path",
                ));
            }
            if SERVICE.with_borrow(|s| s.generation != generation) {
                // Cancellation may happen while CreateUpdateMonitor is still in flight.
                connection.call(
                    Some(BUS),
                    &path,
                    MONITOR,
                    "Close",
                    None,
                    None,
                    gio::DBusCallFlags::NONE,
                    5000,
                    gio::Cancellable::NONE,
                    |_| {},
                );
            }
            // A created monitor is only listening; it does not prove no updates exist.
            Ok(())
        }
        .await;
        if result.is_err() && SERVICE.with_borrow(|s| s.generation == generation) {
            close_monitor();
            publish(UpdateState::Failed);
        }
    });
}
fn install(parent: Option<gtk::Window>) {
    if !supported() {
        return;
    }
    let target = SERVICE.with_borrow(|s| {
        if s.state != Some(UpdateState::Available) {
            return None;
        }
        Some((s.connection.clone()?, s.path.clone()?, s.generation))
    });
    let Some((connection, path, generation)) = target else {
        return;
    };
    SERVICE.with_borrow_mut(|s| s.stop_after_install = false);
    publish(UpdateState::Installing(0));
    glib::spawn_future_local(async move {
        // The Flatpak portal owns consent, signatures, repository and permissions.
        #[cfg(target_os = "linux")]
        let identifier = if let Some(parent) = parent.as_ref() {
            ashpd::WindowIdentifier::from_native(parent).await
        } else {
            None
        };
        #[cfg(not(target_os = "linux"))]
        let identifier: Option<ashpd::WindowIdentifier> = {
            let _ = parent;
            None
        };
        let Some(identifier) = identifier else {
            publish(UpdateState::Failed);
            return;
        };
        let options: HashMap<String, glib::Variant> = HashMap::new();
        let result = connection
            .call_future(
                Some(BUS),
                &path,
                MONITOR,
                "Update",
                Some(&(identifier.to_string(), options).to_variant()),
                None,
                gio::DBusCallFlags::NONE,
                i32::MAX,
            )
            .await;
        if let Err(error) = result {
            if SERVICE.with_borrow(|s| s.generation == generation) {
                let state = if error.matches(gio::IOErrorEnum::Cancelled) {
                    UpdateState::Cancelled
                } else {
                    update_policy::failure(gio::DBusError::remote_error(&error).as_deref())
                };
                publish(state);
            }
        }
    });
}

/// Called once for the main application. Automatic means checking only, never installing.
pub fn startup() {
    if !supported() || SERVICE.with_borrow_mut(|s| std::mem::replace(&mut s.started, true)) {
        return;
    }
    let settings = gio::Settings::new(*crate::config::APP_ID);
    if settings.boolean("automatic-update-checks") {
        check();
    }
    settings.connect_changed(Some("automatic-update-checks"), |settings, _| {
        if settings.boolean("automatic-update-checks") {
            SERVICE.with_borrow_mut(|s| s.stop_after_install = false);
            check();
        } else if SERVICE.with_borrow(|s| matches!(s.state, Some(UpdateState::Installing(_) | UpdateState::Cancelling)))
        {
            SERVICE.with_borrow_mut(|s| s.stop_after_install = true);
        } else if !SERVICE
            .with_borrow(|s| matches!(s.state, Some(UpdateState::Installing(_) | UpdateState::Cancelling)))
        {
            close_monitor();
            publish(UpdateState::Idle);
        }
    });
    // Keep the settings object alive so toggles stop/start the application monitor.
    SERVICE_SETTINGS.with_borrow_mut(|slot| *slot = Some(settings));
}
thread_local! { static SERVICE_SETTINGS: RefCell<Option<gio::Settings>> = const { RefCell::new(None) }; }

pub fn add_page(prefs: &adw::PreferencesDialog) {
    let page = adw::PreferencesPage::builder()
        .name("updates")
        .title(gettext("Updates"))
        .icon_name("software-update-available-symbolic")
        .build();
    let group = adw::PreferencesGroup::builder().title(gettext("App Updates")).build();
    let version = adw::ActionRow::builder()
        .title(gettext("Installed version"))
        .subtitle(*crate::config::VERSION)
        .build();
    group.add(&version);
    let latest = adw::ActionRow::builder()
        .title(gettext("Latest available Flatpak commit"))
        .subtitle(gettext("Not checked"))
        .build();
    group.add(&latest);
    SERVICE.with_borrow_mut(|s| {
        if let Some(commit) = &s.latest {
            latest.set_subtitle(commit);
        }
        s.latest_rows.push(latest.downgrade());
    });
    let status = adw::ActionRow::builder().title(gettext("Update status")).build();
    group.add(&status);
    let check_row = adw::ButtonRow::builder().title(gettext("Check for Updates")).build();
    check_row.set_widget_name("updates.check");
    let monitor = adw::ButtonRow::builder().title(gettext("Monitor for Updates")).build();
    let automatic = adw::SwitchRow::builder()
        .title(gettext("Check automatically"))
        .subtitle(gettext(
            "Monitor while Momentum runs. Flatpak controls check timing. Installation always needs your approval.",
        ))
        .build();
    automatic.set_widget_name("updates.automatic");
    let install_row = adw::ButtonRow::builder().title(gettext("Install Update…")).build();
    let notes = adw::ButtonRow::builder().title(gettext("Release notes")).build();
    notes.connect_activated(|row| {
        gtk::UriLauncher::new("https://github.com/dan-hart/momentum/releases").launch(
            row.root().and_downcast::<gtk::Window>().as_ref(),
            gio::Cancellable::NONE,
            |_| {},
        );
    });
    group.add(&notes);
    notes.set_sensitive(std::env::var_os("MOMENTUM_DEMO").is_none());
    let cancel = adw::ButtonRow::builder()
        .title(gettext("Stop checking or cancel update"))
        .build();
    let restart = adw::ButtonRow::builder().title(gettext("Restart Momentum")).build();
    restart.connect_activated(|row| {
        let dialog = adw::AlertDialog::builder()
            .heading(gettext("Restart Momentum"))
            .body(gettext("After an update, quit Momentum with Ctrl+Q, then reopen it. Closing a window can leave Momentum running in the background. Your saved tasks remain on this device; syncing is not required."))
            .build();
        dialog.add_response("close", &gettext("Close"));
        dialog.set_close_response("close");
        dialog.present(Some(row));
    });
    restart.set_widget_name("updates.restart");
    group.add(&restart);
    for row in [&check_row, &monitor, &install_row, &cancel] {
        group.add(row);
    }
    group.add(&automatic);
    page.add(&group);
    prefs.add(&page);
    if supported() {
        gio::Settings::new(*crate::config::APP_ID)
            .bind("automatic-update-checks", &automatic, "active")
            .build();
        check_row.connect_activated(|row| {
            gtk::UriLauncher::new("appstream://io.github.dan_hart.Momentum").launch(
                row.root().and_downcast::<gtk::Window>().as_ref(),
                gio::Cancellable::NONE,
                |_| {},
            );
        });
        monitor.connect_activated(|_| {
            check();
        });
        install_row.connect_activated(|row| install(row.root().and_downcast::<gtk::Window>()));
        cancel.connect_activated(|_| crate::updates::cancel());
        SERVICE.with_borrow_mut(|s| {
            s.rows
                .push((status.downgrade(), install_row.downgrade(), cancel.downgrade()))
        });
        publish(SERVICE.with_borrow(|s| s.state.clone().unwrap_or(UpdateState::Idle)));
    } else {
        for row in [&check_row, &monitor, &install_row, &cancel] {
            row.set_sensitive(false);
        }
        automatic.set_sensitive(false);
        status.set_subtitle(&if std::env::var_os("MOMENTUM_DEMO").is_some() {
            gettext("Updates are disabled in demo mode.")
        } else {
            gettext("This is a development or source installation. Update it using the tool that installed it.")
        });
    }
}
