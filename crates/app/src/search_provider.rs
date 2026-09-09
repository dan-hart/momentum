// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Desktop search integration: GNOME Shell's SearchProvider2 and KDE's KRunner D-Bus runner.
//! Both are served from the app process; the desktop D-Bus-activates the app when needed and
//! the window stays hidden until a result is activated.
use adw::prelude::*;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gettextrs::gettext;
use gtk::{gio, glib};

use crate::application::MomentumApplication;
use crate::window::View;

const GNOME_XML: &str = r#"<node>
  <interface name="org.gnome.Shell.SearchProvider2">
    <method name="GetInitialResultSet"><arg type="as" name="terms" direction="in"/><arg type="as" name="results" direction="out"/></method>
    <method name="GetSubsearchResultSet"><arg type="as" name="previous_results" direction="in"/><arg type="as" name="terms" direction="in"/><arg type="as" name="results" direction="out"/></method>
    <method name="GetResultMetas"><arg type="as" name="identifiers" direction="in"/><arg type="aa{sv}" name="metas" direction="out"/></method>
    <method name="ActivateResult"><arg type="s" name="identifier" direction="in"/><arg type="as" name="terms" direction="in"/><arg type="u" name="timestamp" direction="in"/></method>
    <method name="LaunchSearch"><arg type="as" name="terms" direction="in"/><arg type="u" name="timestamp" direction="in"/></method>
  </interface>
</node>"#;

const KRUNNER_XML: &str = r#"<node>
  <interface name="org.kde.krunner1">
    <method name="Match"><arg type="s" name="query" direction="in"/><arg type="a(sssida{sv})" name="matches" direction="out"/></method>
    <method name="Actions"><arg type="a(sss)" name="matches" direction="out"/></method>
    <method name="Run"><arg type="s" name="matchId" direction="in"/><arg type="s" name="actionId" direction="in"/></method>
  </interface>
</node>"#;

const MAX_RESULTS: usize = 8;

/// Result ids: a task id, or `add:<title>` for the "create" result.
fn results(app: &MomentumApplication, terms: &[String]) -> Vec<String> {
    let query = terms.join(" ");
    let q: Vec<String> = query.to_lowercase().split_whitespace().map(str::to_string).collect();
    if q.is_empty() {
        return vec![];
    }
    let win = app.ensure_window();
    let store = win.imp().store.borrow();
    let mut ids: Vec<String> = store
        .state
        .task
        .iter()
        .filter(|t| !t.is_done)
        .filter(|t| {
            let hay = t.title.to_lowercase();
            q.iter().all(|w| hay.contains(w.as_str()))
        })
        .take(MAX_RESULTS)
        .map(|t| t.id.clone())
        .collect();
    ids.push(format!("add:{query}"));
    ids
}

fn meta(app: &MomentumApplication, id: &str) -> (String, String) {
    if let Some(title) = id.strip_prefix("add:") {
        return (format!("{} “{title}”", gettext("Create task")), gettext("Momentum"));
    }
    let win = app.ensure_window();
    let store = win.imp().store.borrow();
    match store.state.task.entities.get(id) {
        Some(t) => {
            let mut bits = vec![];
            if let Some(p) = store.state.project.entities.get(&t.project_id) {
                bits.push(p.title.clone());
            }
            if let Some(d) = &t.due_day {
                bits.push(crate::window::fmt_day(d));
            }
            (t.title.clone(), bits.join(" · "))
        }
        None => (id.to_string(), String::new()),
    }
}

fn activate(app: &MomentumApplication, id: &str) {
    let win = app.ensure_window();
    if let Some(title) = id.strip_prefix("add:") {
        win.add_task(title);
        win.go_to(View::Today);
    } else {
        win.go_to(View::Search);
        win.open_task(id);
    }
    win.present();
}

pub fn register(
    app: &MomentumApplication,
    connection: &gio::DBusConnection,
    base_path: &str,
) -> Result<(), glib::Error> {
    let gnome = gio::DBusNodeInfo::for_xml(GNOME_XML)?;
    let krunner = gio::DBusNodeInfo::for_xml(KRUNNER_XML)?;
    let gnome_iface = gnome.lookup_interface("org.gnome.Shell.SearchProvider2").unwrap();
    let krunner_iface = krunner.lookup_interface("org.kde.krunner1").unwrap();

    connection
        .register_object(&format!("{base_path}/SearchProvider"), &gnome_iface)
        .method_call(glib::clone!(
            #[weak]
            app,
            move |_, _, _, _, method, params, invocation| {
                match method {
                    "GetInitialResultSet" => {
                        let (terms,): (Vec<String>,) = params.get().unwrap_or_default();
                        invocation.return_value(Some(&(results(&app, &terms),).to_variant()));
                    }
                    "GetSubsearchResultSet" => {
                        let (_, terms): (Vec<String>, Vec<String>) = params.get().unwrap_or_default();
                        invocation.return_value(Some(&(results(&app, &terms),).to_variant()));
                    }
                    "GetResultMetas" => {
                        let (ids,): (Vec<String>,) = params.get().unwrap_or_default();
                        let metas: Vec<glib::Variant> = ids
                            .iter()
                            .map(|id| {
                                let (name, description) = meta(&app, id);
                                let dict = glib::VariantDict::new(None);
                                dict.insert("id", id.as_str());
                                dict.insert("name", name.as_str());
                                dict.insert("description", description.as_str());
                                dict.insert(
                                    "gicon",
                                    if id.starts_with("add:") {
                                        "list-add-symbolic"
                                    } else {
                                        *crate::config::APP_ID
                                    },
                                );
                                dict.end()
                            })
                            .collect();
                        let arr = glib::Variant::array_from_iter_with_type(&glib::VariantTy::VARDICT, metas);
                        invocation.return_value(Some(&glib::Variant::tuple_from_iter([arr])));
                    }
                    "ActivateResult" => {
                        let (id, _terms, _ts): (String, Vec<String>, u32) = params.get().unwrap_or_default();
                        activate(&app, &id);
                        invocation.return_value(None);
                    }
                    "LaunchSearch" => {
                        let (terms, _ts): (Vec<String>, u32) = params.get().unwrap_or_default();
                        let win = app.ensure_window();
                        win.go_to(View::Search);
                        win.set_search_query(&terms.join(" "));
                        win.present();
                        invocation.return_value(None);
                    }
                    _ => invocation.return_dbus_error("org.freedesktop.DBus.Error.UnknownMethod", "no such method"),
                }
            }
        ))
        .build()?;

    connection
        .register_object(&format!("{base_path}/KRunner"), &krunner_iface)
        .method_call(glib::clone!(
            #[weak]
            app,
            move |_, _, _, _, method, params, invocation| {
                match method {
                    "Match" => {
                        let (query,): (String,) = params.get().unwrap_or_default();
                        let terms: Vec<String> = query.split_whitespace().map(str::to_string).collect();
                        let ids = if query.trim().len() < 2 {
                            vec![]
                        } else {
                            results(&app, &terms)
                        };
                        // (id, text, icon, type, relevance, properties); type 100 = ExactMatch, 70 = PossibleMatch
                        let matches: Vec<(String, String, String, i32, f64, glib::VariantDict)> = ids
                            .iter()
                            .enumerate()
                            .map(|(i, id)| {
                                let (name, description) = meta(&app, id);
                                let props = glib::VariantDict::new(None);
                                props.insert("subtext", description.as_str());
                                let is_add = id.starts_with("add:");
                                (
                                    id.clone(),
                                    name,
                                    (if is_add { "list-add" } else { *crate::config::APP_ID }).to_string(),
                                    if is_add { 70 } else { 100 },
                                    1.0 - i as f64 * 0.05,
                                    props,
                                )
                            })
                            .collect();
                        let items: Vec<glib::Variant> = matches
                            .into_iter()
                            .map(|(id, text, icon, kind, rel, props)| (id, text, icon, kind, rel, props).to_variant())
                            .collect();
                        let arr = glib::Variant::array_from_iter_with_type(
                            glib::VariantTy::new("(sssida{sv})").unwrap(),
                            items,
                        );
                        invocation.return_value(Some(&glib::Variant::tuple_from_iter([arr])));
                    }
                    "Actions" => {
                        let arr = glib::Variant::array_from_iter_with_type(
                            glib::VariantTy::new("(sss)").unwrap(),
                            Vec::<glib::Variant>::new(),
                        );
                        invocation.return_value(Some(&glib::Variant::tuple_from_iter([arr])));
                    }
                    "Run" => {
                        let (id, _action): (String, String) = params.get().unwrap_or_default();
                        activate(&app, &id);
                        invocation.return_value(None);
                    }
                    _ => invocation.return_dbus_error("org.freedesktop.DBus.Error.UnknownMethod", "no such method"),
                }
            }
        ))
        .build()?;
    Ok(())
}
