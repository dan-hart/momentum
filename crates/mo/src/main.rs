// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! `mo`: Momentum from the terminal, on Linux and macOS.
//!
//! Reads the same data as the app. When the desktop app is running, changes are
//! handed to its local socket (or Linux D-Bus); otherwise `mo` applies
//! them to the store directly. Queries always read the store.
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sp_model::*;
use sp_oplog::Action;
use sp_store::Store;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "mo", version, about = "Momentum from the terminal", long_about = None)]
struct Cli {
    /// Machine-readable JSON output
    #[arg(long, global = true)]
    json: bool,
    /// Data directory (default: the desktop app's, else the platform data dir)
    #[arg(long, global = true, env = "MO_DATA_DIR")]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Add a task. Words starting with # become tags; "1h 30m" sets the estimate.
    Add {
        title: Vec<String>,
        #[arg(short, long)]
        project: Option<String>,
        #[arg(long, conflicts_with_all = ["tonight", "morning", "tomorrow", "due"])]
        today: bool,
        #[arg(long, conflicts_with_all = ["morning", "tomorrow", "due"])]
        tonight: bool,
        #[arg(long, conflicts_with_all = ["tomorrow", "due"])]
        morning: bool,
        #[arg(long, conflicts_with = "due")]
        tomorrow: bool,
        #[arg(long, value_name = "YYYY-MM-DD", value_parser = valid_day)]
        due: Option<String>,
        #[arg(short, long)]
        notes: Option<String>,
    },
    /// Tasks due today
    Today,
    /// Today's tasks tagged Morning
    Morning,
    /// Today's tasks tagged Evening
    Tonight,
    /// Tasks due in the coming days
    Upcoming {
        #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(i64).range(1..=36500))]
        days: i64,
    },
    /// Tasks in a project (by name) or with a tag (#name); all open tasks by default
    List { filter: Option<String> },
    /// Search titles, notes, tags and projects
    Search { query: Vec<String> },
    /// Mark a task done (id prefix or unique title fragment)
    Done { task: String },
    /// Mark a task not done
    Undone { task: String },
    /// Plan a task for today
    Plan { task: String },
    /// Delete a task
    Rm { task: String },
    /// List projects
    Projects,
    /// List tags
    Tags,
    /// Sync with Nextcloud
    Sync,
    /// Show or set Nextcloud settings; passwords go to the system keychain
    Config {
        #[arg(long)]
        server: Option<String>,
        #[arg(long)]
        user: Option<String>,
        #[arg(long)]
        folder: Option<String>,
        /// Prompt for the Nextcloud app password
        #[arg(long)]
        password: bool,
        /// Prompt for the end-to-end encryption password
        #[arg(long)]
        encryption_password: bool,
    },
}

#[derive(Default, Serialize, Deserialize)]
struct CliConfig {
    method: Option<String>,
    server: Option<String>,
    user: Option<String>,
    folder: Option<String>,
    #[serde(default)]
    compress: bool,
}

#[cfg(target_os = "linux")]
const APP_IDS: [&str; 2] = ["io.github.dan_hart.Momentum", "io.github.dan_hart.Momentum.Devel"];

fn data_dir(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(d) = explicit {
        return d;
    }
    if let Some(d) = std::env::var_os("MOMENTUM_DATA_DIR").filter(|d| !d.is_empty()) {
        return PathBuf::from(d);
    }
    default_data_dir()
}

fn default_data_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    #[cfg(target_os = "linux")]
    for id in APP_IDS {
        let p = home.join(".var/app").join(id).join("data/momentum");
        if p.join("state.json").exists() {
            return p;
        }
    }
    dirs::data_dir().unwrap_or(home).join("momentum")
}

/// Nextcloud settings: `cli-config.json` first, then the Flatpak app's GSettings keyfile.
fn load_config(dir: &PathBuf) -> CliConfig {
    #[allow(unused_mut)] // Linux fills missing settings from the app's keyfile.
    let mut cfg: CliConfig = std::fs::read(dir.join("cli-config.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    #[cfg(target_os = "linux")]
    if dir == &default_data_dir() {
        let home = dirs::home_dir().unwrap_or_default();
        for id in APP_IDS {
            let keyfile = home.join(".var/app").join(id).join("config/glib-2.0/settings/keyfile");
            if let Ok(text) = std::fs::read_to_string(keyfile) {
                merge_linux_settings(&mut cfg, &text);
                if cfg.server.is_some() {
                    break;
                }
            }
        }
    }
    cfg
}

/// Connection fields may come from `mo config`, but the native app owns which
/// service is allowed to run. Read that choice even when cached server fields exist.
#[cfg(any(target_os = "linux", test))]
fn merge_linux_settings(cfg: &mut CliConfig, text: &str) {
    let get = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{key}=")))
            .map(|value| value.trim().trim_matches('\'').to_string())
    };
    let method = get("sync-method").filter(|value| !value.is_empty());
    cfg.method = Some(method.unwrap_or_else(|| {
        if get("p2p-enabled").as_deref() == Some("true") {
            "libresync"
        } else if get("sync-enabled").as_deref() == Some("true") {
            "nextcloud"
        } else {
            "off"
        }
        .into()
    }));
    if cfg.server.is_none() {
        cfg.compress = get("compress").as_deref() == Some("true");
    }
    cfg.server = cfg.server.take().or_else(|| get("nextcloud-server"));
    cfg.user = cfg.user.take().or_else(|| get("nextcloud-user"));
    cfg.folder = cfg.folder.take().or_else(|| get("nextcloud-folder"));
}

fn secret(purpose: &str) -> Option<String> {
    keyring::Entry::new("momentum", purpose).ok()?.get_password().ok()
}

/// Hand a payload to a running app: the data directory's socket first (any platform,
/// see `momentum_core::ipc`), then D-Bus on Linux for the case the socket is not up yet.
/// Ok(false) means no running app answered, so the caller writes the store itself.
fn forward(dir: &Path, payload: &str) -> Result<bool, String> {
    if momentum_core::ipc::forward(dir, payload)? {
        return Ok(true);
    }
    // A separate profile must never send actions to the default app on D-Bus.
    if dir == default_data_dir() {
        forward_dbus(payload)
    } else {
        Ok(false)
    }
}

/// Hand an action to the running desktop app over D-Bus (Linux). Ok(false) = app not running.
#[cfg(target_os = "linux")]
fn forward_dbus(payload: &str) -> Result<bool, String> {
    use zbus::blocking::Connection;
    use zbus::zvariant::Value;
    let Ok(conn) = Connection::session() else {
        return Ok(false);
    };
    for id in APP_IDS {
        let path = format!("/{}", id.replace('.', "/"));
        let has_owner: bool = conn
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "NameHasOwner",
                &(id,),
            )
            .ok()
            .and_then(|m| m.body().deserialize().ok())
            .unwrap_or(false);
        if !has_owner {
            continue;
        }
        let params: Vec<Value> = vec![Value::from(payload)];
        let platform: std::collections::HashMap<String, Value> = Default::default();
        conn.call_method(
            Some(id),
            path.as_str(),
            Some("org.gtk.Actions"),
            "Activate",
            &("cli", params, platform),
        )
        .map_err(|e| e.to_string())?;
        return Ok(true);
    }
    Ok(false)
}
#[cfg(not(target_os = "linux"))]
fn forward_dbus(_payload: &str) -> Result<bool, String> {
    Ok(false)
}

fn apply(store: &mut Store, dir: &Path, action: Action) -> Result<(), String> {
    let payload = serde_json::to_string(&action).map_err(|e| e.to_string())?;
    // No running app (CI, a headless box, the app not started) means write it ourselves.
    if forward(dir, &payload)? {
        // D-Bus activation can be asynchronous; apply only to this process's snapshot.
        // Never queue or persist the action a second time.
        sp_oplog::apply(&mut store.state, &action);
        return Ok(());
    }
    // A previous action may have been forwarded before the app exited. Refresh its
    // persisted metadata so an offline continuation retains that action and its op.
    *store = Store::load(dir.to_path_buf());
    sp_oplog::apply(&mut store.state, &action);
    let op = action.to_op(&store.meta.client_id, &mut store.meta.vector_clock);
    store.pending.push(sp_store::Pending { op, action });
    store.save().map_err(|e| e.to_string())
}

/// A task by id prefix or unique title fragment. Open tasks only, unless `done_ok`
/// (undone and rm must be able to reach completed tasks).
fn find_task<'a>(store: &'a Store, needle: &str, done_ok: bool) -> Result<&'a Task, String> {
    if needle.trim().is_empty() {
        return Err("a task id or title fragment is required".into());
    }
    if let Some(task) = store.state.task.entities.get(needle) {
        return Ok(task);
    }
    // Imported IDs are case-sensitive opaque strings, not necessarily UUIDs.
    let by_id: Vec<&Task> = store.state.task.iter().filter(|t| t.id.starts_with(needle)).collect();
    if !by_id.is_empty() {
        return unique_task(by_id, needle);
    }
    let n = needle.to_lowercase();
    unique_task(
        store
            .state
            .task
            .iter()
            .filter(|t| (done_ok || !t.is_done) && t.title.to_lowercase().contains(&n))
            .collect(),
        needle,
    )
}

fn unique_task<'a>(tasks: Vec<&'a Task>, needle: &str) -> Result<&'a Task, String> {
    match tasks.len() {
        1 => Ok(tasks[0]),
        0 => Err(format!("no task matches “{needle}”")),
        _ => Err(format!(
            "{} tasks match “{needle}”; use a longer id prefix:\n{}",
            tasks.len(),
            tasks
                .iter()
                .map(|t| format!("  {}  {}", t.id, t.title))
                .collect::<Vec<_>>()
                .join("\n")
        )),
    }
}

fn short_id(store: &Store, id: &str) -> String {
    let mut length = 8.min(id.chars().count());
    loop {
        let prefix: String = id.chars().take(length).collect();
        if prefix == id || !store.state.task.iter().any(|t| t.id != id && t.id.starts_with(&prefix)) {
            return prefix;
        }
        length += 1;
    }
}

fn valid_day(day: &str) -> Result<String, String> {
    let valid = day.len() == 10
        && day.bytes().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
        && parse_day(day).is_some_and(|(year, month, date)| {
            (1..=9999).contains(&year)
                && (1..=12).contains(&month)
                && (1..=31).contains(&date)
                && day_str(days_from_civil(year, month, date)) == day
        });
    if valid {
        Ok(day.into())
    } else {
        Err("expected a valid date in YYYY-MM-DD format".into())
    }
}

fn print_change(store: &Store, task: &Task, status: &str, label: &str, json: bool) {
    if json {
        let current = store.state.task.entities.get(&task.id).unwrap_or(task);
        println!("{}", serde_json::json!({"status": status, "task": current}));
    } else {
        println!("{label}: {}", task.title);
    }
}

fn fmt_ms(ms: f64) -> String {
    let m = (ms / 60000.0).round() as u64;
    if m >= 60 {
        format!("{}h{:02}", m / 60, m % 60)
    } else {
        format!("{m}m")
    }
}

fn print_tasks(store: &Store, tasks: &[&Task], json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(tasks).unwrap());
        return;
    }
    if tasks.is_empty() {
        println!("Nothing here.");
        return;
    }
    let today = today_str();
    for t in tasks {
        let mut bits = vec![];
        if let Some(p) = store.state.project.entities.get(&t.project_id) {
            bits.push(p.title.clone());
        }
        if t.time_estimate > 0.0 {
            bits.push(format!("~{}", fmt_ms(t.time_estimate)));
        }
        if let Some(d) = t.plan_day() {
            let mut when = if d == today { "today".to_string() } else { d };
            if let Some(ms) = t.due_with_time {
                let (h, m) = time_of_ms(ms);
                when = format!("{when} {h:02}:{m:02}");
            }
            bits.push(when);
        }
        for g in t.tag_ids.iter().filter_map(|i| store.state.tag.entities.get(i)) {
            bits.push(format!("#{}", g.title));
        }
        let indent = if t.parent_id.is_some() { "    " } else { "" };
        println!(
            "[{}] {}  {}{}{}",
            if t.is_done { "x" } else { " " },
            short_id(store, &t.id),
            indent,
            t.title,
            if bits.is_empty() {
                String::new()
            } else {
                format!("  · {}", bits.join(" · "))
            }
        );
    }
}

fn main() {
    let cli = Cli::parse();
    let dir = data_dir(cli.data_dir.clone());
    let mut store = Store::load(dir.clone());
    let json = cli.json;
    let result: Result<(), String> = (|| {
        match cli.cmd {
            Cmd::Add {
                title,
                project,
                today,
                tonight,
                morning,
                tomorrow,
                due,
                notes,
            } => {
                let parsed = momentum_core::text::parse_quick_add(&title.join(" "));
                let mut tags = parsed.tags;
                if parsed.title.is_empty() {
                    return Err("a title is required".into());
                }
                let project_id = match &project {
                    Some(name) => store
                        .state
                        .project
                        .iter()
                        .find(|p| p.title.eq_ignore_ascii_case(name))
                        .map(|p| p.id.clone())
                        .ok_or_else(|| format!("no project named “{name}”"))?,
                    None => store
                        .state
                        .project
                        .ids
                        .first()
                        .cloned()
                        .unwrap_or(INBOX_PROJECT_ID.into()),
                };
                if tonight {
                    tags.push("Evening".into());
                }
                if morning {
                    tags.push("Morning".into());
                }
                let mut task = Task::new(&parsed.title, &project_id);
                task.time_estimate = parsed.estimate_ms;
                task.notes = notes;
                task.due_day = if today || tonight || morning {
                    Some(today_str())
                } else if tomorrow {
                    Some(day_str(day_number(&today_str()).unwrap_or(0) + 1))
                } else {
                    due
                };
                for name in tags {
                    let existing = store
                        .state
                        .tag
                        .iter()
                        .find(|g| g.title.eq_ignore_ascii_case(&name))
                        .map(|g| g.id.clone());
                    let id = match existing {
                        Some(id) => id,
                        None => {
                            let tag = Tag::new(&name);
                            let id = tag.id.clone();
                            apply(&mut store, &dir, Action::AddTag { tag })?;
                            id
                        }
                    };
                    if !task.tag_ids.contains(&id) {
                        task.tag_ids.push(id);
                    }
                }
                let shown = task.clone();
                apply(&mut store, &dir, Action::AddTask { task, bottom: true })?;
                print_tasks(&store, &[&shown], json);
            }
            Cmd::Today => {
                let ids = store.state.today_ids();
                let tasks: Vec<&Task> = ids.iter().filter_map(|i| store.state.task.entities.get(i)).collect();
                print_tasks(&store, &tasks, json);
            }
            Cmd::Tonight | Cmd::Morning => {
                let want = if matches!(cli.cmd, Cmd::Morning) {
                    "morning"
                } else {
                    "evening"
                };
                let evening = store
                    .state
                    .tag
                    .iter()
                    .find(|t| t.title.eq_ignore_ascii_case(want))
                    .map(|t| t.id.clone());
                let ids = store.state.today_ids();
                let tasks: Vec<&Task> = ids
                    .iter()
                    .filter_map(|i| store.state.task.entities.get(i))
                    .filter(|t| evening.as_ref().is_some_and(|e| t.tag_ids.contains(e)))
                    .collect();
                print_tasks(&store, &tasks, json);
            }
            Cmd::Upcoming { days } => {
                let today_n = day_number(&today_str()).unwrap_or(0);
                let mut tasks: Vec<&Task> = store
                    .state
                    .task
                    .iter()
                    .filter(|t| !t.is_done && t.parent_id.is_none())
                    .filter(|t| {
                        t.plan_day()
                            .as_deref()
                            .and_then(day_number)
                            .is_some_and(|d| d > today_n && d <= today_n + days)
                    })
                    .collect();
                tasks.sort_by_key(|t| (t.plan_day(), t.due_with_time));
                print_tasks(&store, &tasks, json);
            }
            Cmd::List { filter } => {
                let tasks: Vec<&Task> = match filter.as_deref() {
                    Some(f) if f.starts_with('#') => {
                        let g = store
                            .state
                            .tag
                            .iter()
                            .find(|g| g.title.eq_ignore_ascii_case(&f[1..]))
                            .ok_or_else(|| format!("no tag {f}"))?;
                        g.task_ids
                            .iter()
                            .filter_map(|i| store.state.task.entities.get(i))
                            .collect()
                    }
                    Some(name) => {
                        let p = store
                            .state
                            .project
                            .iter()
                            .find(|p| p.title.eq_ignore_ascii_case(name))
                            .ok_or_else(|| format!("no project named “{name}”"))?;
                        p.task_ids
                            .iter()
                            .filter_map(|i| store.state.task.entities.get(i))
                            .collect()
                    }
                    None => store
                        .state
                        .task
                        .iter()
                        .filter(|t| !t.is_done && t.parent_id.is_none())
                        .collect(),
                };
                print_tasks(&store, &tasks, json);
            }
            Cmd::Search { query } => {
                let q: Vec<String> = query.iter().map(|w| w.to_lowercase()).collect();
                let tasks: Vec<&Task> = store
                    .state
                    .task
                    .iter()
                    .filter(|t| {
                        let mut hay = t.title.to_lowercase();
                        if let Some(n) = &t.notes {
                            hay.push('\n');
                            hay.push_str(&n.to_lowercase());
                        }
                        for g in t.tag_ids.iter().filter_map(|i| store.state.tag.entities.get(i)) {
                            hay.push('\n');
                            hay.push_str(&g.title.to_lowercase());
                        }
                        if let Some(p) = store.state.project.entities.get(&t.project_id) {
                            hay.push('\n');
                            hay.push_str(&p.title.to_lowercase());
                        }
                        q.iter().all(|w| hay.contains(w.as_str()))
                    })
                    .collect();
                print_tasks(&store, &tasks, json);
            }
            Cmd::Done { task } => {
                let t = find_task(&store, &task, false)?.clone();
                apply(
                    &mut store,
                    &dir,
                    Action::UpdateTask {
                        id: t.id.clone(),
                        changes: [("isDone".to_string(), serde_json::json!(true))].into_iter().collect(),
                    },
                )?;
                print_change(&store, &t, "done", "Done", json);
            }
            Cmd::Undone { task } => {
                let t = find_task(&store, &task, true)?.clone();
                apply(
                    &mut store,
                    &dir,
                    Action::UpdateTask {
                        id: t.id.clone(),
                        changes: [("isDone".to_string(), serde_json::json!(false))].into_iter().collect(),
                    },
                )?;
                print_change(&store, &t, "undone", "Not done", json);
            }
            Cmd::Plan { task } => {
                let t = find_task(&store, &task, false)?.clone();
                apply(
                    &mut store,
                    &dir,
                    Action::PlanForToday {
                        task_ids: vec![t.id.clone()],
                        today: today_str(),
                    },
                )?;
                print_change(&store, &t, "planned", "Planned for today", json);
            }
            Cmd::Rm { task } => {
                let t = find_task(&store, &task, true)?.clone();
                let subs: Vec<Task> = t
                    .sub_task_ids
                    .iter()
                    .filter_map(|i| store.state.task.entities.get(i).cloned())
                    .collect();
                apply(
                    &mut store,
                    &dir,
                    Action::DeleteTask {
                        task: t.clone(),
                        sub_tasks: subs,
                    },
                )?;
                print_change(&store, &t, "deleted", "Deleted", json);
            }
            Cmd::Projects => {
                if json {
                    let v: Vec<&sp_model::Project> = store.state.project.iter().filter(|p| !p.is_archived).collect();
                    println!("{}", serde_json::to_string_pretty(&v).unwrap());
                    return Ok(());
                }
                for p in store.state.project.iter().filter(|p| !p.is_archived) {
                    let open = p
                        .task_ids
                        .iter()
                        .filter_map(|i| store.state.task.entities.get(i))
                        .filter(|t| !t.is_done)
                        .count();
                    println!("{:<28} {open} open", p.title);
                }
            }
            Cmd::Tags => {
                if json {
                    let v: Vec<&sp_model::Tag> = store.state.tag.iter().filter(|g| g.id != TODAY_TAG_ID).collect();
                    println!("{}", serde_json::to_string_pretty(&v).unwrap());
                    return Ok(());
                }
                for g in store.state.tag.iter().filter(|g| g.id != TODAY_TAG_ID) {
                    println!("#{:<27} {} tasks", g.title, g.task_ids.len());
                }
            }
            Cmd::Sync => {
                let c = load_config(&dir);
                match c.method.as_deref() {
                    Some("off") => return Err("Sync is turned off. Select a sync method in Momentum Settings.".into()),
                    None | Some("nextcloud") | Some("libresync") => {}
                    Some(_) => return Err("Unknown sync method. Select a sync method in Momentum Settings.".into()),
                }
                if forward(&dir, "\"sync\"")? {
                    if json {
                        println!("{}", serde_json::json!({"status": "requested"}));
                    } else {
                        println!("Sync requested from the running app.");
                    }
                    return Ok(());
                }
                if c.method.as_deref() == Some("libresync") {
                    return Err("Open Momentum to sync with LibreSync.".into());
                }
                let cfg = sp_sync::NextcloudCfg {
                    server_url: c.server.clone().ok_or(
                        "no server configured; run `mo config --server URL --user NAME --folder DIR --password`",
                    )?,
                    user_name: c.user.clone().unwrap_or_default(),
                    password: secret("nextcloud").ok_or("no app password stored; run `mo config --password`")?,
                    folder: c.folder.clone().unwrap_or_else(|| "super-productivity".into()),
                    compress: c.compress,
                    encrypt_key: secret("encryption"),
                };
                let r = sp_sync::sync(&cfg, &mut store).map_err(|e| e.to_string())?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({"status": "synced", "downloaded": r.downloaded, "ops_uploaded": r.ops_uploaded})
                    );
                } else {
                    println!(
                        "Synced (downloaded: {}, uploaded: {} op{}).",
                        r.downloaded,
                        r.ops_uploaded,
                        if r.ops_uploaded == 1 { "" } else { "s" }
                    );
                }
            }
            Cmd::Config {
                server,
                user,
                folder,
                password,
                encryption_password,
            } => {
                let settings_changed = server.is_some() || user.is_some() || folder.is_some();
                let mut c = load_config(&dir);
                if let Some(s) = server {
                    c.server = Some(s);
                }
                if let Some(u) = user {
                    c.user = Some(u);
                }
                if let Some(f) = folder {
                    c.folder = Some(f);
                }
                if settings_changed {
                    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                    let temporary = dir.join("cli-config.json.tmp");
                    std::fs::write(&temporary, serde_json::to_vec_pretty(&c).unwrap()).map_err(|e| e.to_string())?;
                    std::fs::rename(temporary, dir.join("cli-config.json")).map_err(|e| e.to_string())?;
                }
                for (on, purpose, prompt) in [
                    (password, "nextcloud", "Nextcloud app password: "),
                    (encryption_password, "encryption", "Encryption password: "),
                ] {
                    if on {
                        let p = rpassword::prompt_password(prompt).map_err(|e| e.to_string())?;
                        keyring::Entry::new("momentum", purpose)
                            .and_then(|e| e.set_password(&p))
                            .map_err(|e| e.to_string())?;
                    }
                }
                if settings_changed || password || encryption_password {
                    momentum_core::ipc::forward(&dir, "\"config\"")?;
                }
                if json {
                    let mut value = serde_json::to_value(&c).unwrap();
                    value["data_dir"] = serde_json::json!(dir);
                    println!("{}", value);
                } else {
                    println!(
                        "server:  {}\nuser:    {}\nfolder:  {}\ndata:    {}",
                        c.server.as_deref().unwrap_or("-"),
                        c.user.as_deref().unwrap_or("-"),
                        c.folder.as_deref().unwrap_or("super-productivity"),
                        dir.display()
                    );
                }
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        if json {
            eprintln!("{}", serde_json::json!({"error": e}));
        } else {
            eprintln!("mo: {e}");
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod sync_config_tests {
    use super::*;
    #[test]
    fn linux_provider_preference_overrides_cached_connection_settings() {
        let mut config = CliConfig {
            server: Some("https://saved.example".into()),
            ..Default::default()
        };
        merge_linux_settings(
            &mut config,
            "sync-method='off'\nnextcloud-server='https://other.example'\n",
        );
        assert_eq!(config.method.as_deref(), Some("off"));
        assert_eq!(config.server.as_deref(), Some("https://saved.example"));
        merge_linux_settings(&mut config, "sync-method='libresync'\nsync-enabled=true\n");
        assert_eq!(config.method.as_deref(), Some("libresync"));
    }
    #[test]
    fn legacy_linux_dual_sync_prefers_libresync_and_unknown_method_fails_closed() {
        let mut config = CliConfig::default();
        merge_linux_settings(&mut config, "sync-enabled=true\np2p-enabled=true\n");
        assert_eq!(config.method.as_deref(), Some("libresync"));
        merge_linux_settings(&mut config, "sync-method='future-provider'\n");
        assert_eq!(
            config.method.as_deref(),
            Some("future-provider"),
            "the caller rejects unknown providers rather than falling back to Nextcloud"
        );
    }
}
