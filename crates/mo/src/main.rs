// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! `mo`: Momentum from the terminal, on Linux and macOS.
//!
//! Reads the same data as the app. When the desktop app is running (Linux), changes are
//! handed to it over D-Bus so nothing is written behind its back; otherwise `mo` applies
//! them to the store directly. Queries always read the store.
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sp_model::*;
use sp_oplog::Action;
use sp_store::Store;
use std::path::PathBuf;

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
        #[arg(long)]
        today: bool,
        #[arg(long)]
        tonight: bool,
        #[arg(long)]
        tomorrow: bool,
        #[arg(long, value_name = "YYYY-MM-DD")]
        due: Option<String>,
        #[arg(short, long)]
        notes: Option<String>,
    },
    /// Tasks due today
    Today,
    /// Today's tasks tagged Evening
    Tonight,
    /// Tasks due in the coming days
    Upcoming {
        #[arg(long, default_value_t = 7)]
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
    server: Option<String>,
    user: Option<String>,
    folder: Option<String>,
    #[serde(default)]
    compress: bool,
}

const APP_IDS: [&str; 2] = ["io.github.dan_hart.Momentum", "io.github.dan_hart.Momentum.Devel"];

fn data_dir(explicit: Option<PathBuf>) -> PathBuf {
    if let Some(d) = explicit {
        return d;
    }
    let home = dirs::home_dir().unwrap_or_default();
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
    let mut cfg: CliConfig = std::fs::read(dir.join("cli-config.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    if cfg.server.is_none() {
        let home = dirs::home_dir().unwrap_or_default();
        for id in APP_IDS {
            let keyfile = home.join(".var/app").join(id).join("config/glib-2.0/settings/keyfile");
            if let Ok(text) = std::fs::read_to_string(keyfile) {
                let get = |k: &str| {
                    text.lines()
                        .find_map(|l| l.strip_prefix(&format!("{k}=")))
                        .map(|v| v.trim().trim_matches('\'').to_string())
                };
                cfg.server = cfg.server.or_else(|| get("nextcloud-server"));
                cfg.user = cfg.user.or_else(|| get("nextcloud-user"));
                cfg.folder = cfg.folder.or_else(|| get("nextcloud-folder"));
                cfg.compress = get("compress").is_some_and(|v| v == "true");
                if cfg.server.is_some() {
                    break;
                }
            }
        }
    }
    cfg
}

fn secret(purpose: &str) -> Option<String> {
    keyring::Entry::new("momentum", purpose).ok()?.get_password().ok()
}

/// Hand an action to the running desktop app over D-Bus (Linux). Ok(false) = app not running.
#[cfg(target_os = "linux")]
fn forward(payload: &str) -> Result<bool, String> {
    use zbus::blocking::Connection;
    use zbus::zvariant::Value;
    let conn = Connection::session().map_err(|e| e.to_string())?;
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
fn forward(_payload: &str) -> Result<bool, String> {
    Ok(false)
}

fn apply(store: &mut Store, action: Action) -> Result<(), String> {
    let payload = serde_json::to_string(&action).map_err(|e| e.to_string())?;
    if forward(&payload)? {
        return Ok(());
    }
    store.dispatch(action);
    Ok(())
}

fn find_task<'a>(store: &'a Store, needle: &str) -> Result<&'a Task, String> {
    let n = needle.to_lowercase();
    let by_id: Vec<&Task> = store.state.task.iter().filter(|t| t.id.starts_with(&n)).collect();
    if by_id.len() == 1 {
        return Ok(by_id[0]);
    }
    let by_title: Vec<&Task> = store
        .state
        .task
        .iter()
        .filter(|t| !t.is_done && t.title.to_lowercase().contains(&n))
        .collect();
    match by_title.len() {
        1 => Ok(by_title[0]),
        0 => Err(format!("no task matches “{needle}”")),
        _ => Err(format!(
            "{} tasks match “{needle}”; use an id prefix:\n{}",
            by_title.len(),
            by_title
                .iter()
                .map(|t| format!("  {}  {}", &t.id[..8], t.title))
                .collect::<Vec<_>>()
                .join("\n")
        )),
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
        if let Some(d) = &t.due_day {
            bits.push(if *d == today { "today".into() } else { d.clone() });
        }
        for g in t.tag_ids.iter().filter_map(|i| store.state.tag.entities.get(i)) {
            bits.push(format!("#{}", g.title));
        }
        let indent = if t.parent_id.is_some() { "    " } else { "" };
        println!(
            "[{}] {}  {}{}{}",
            if t.is_done { "x" } else { " " },
            &t.id[..8],
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
                tomorrow,
                due,
                notes,
            } => {
                let (mut words, mut tags, mut est) = (vec![], vec![], 0.0);
                for w in &title {
                    if let Some(t) = w.strip_prefix('#') {
                        tags.push(t.to_string());
                    } else if let Some(ms) = parse_estimate(w) {
                        est = ms;
                    } else {
                        words.push(w.as_str());
                    }
                }
                if words.is_empty() {
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
                let mut task = Task::new(&words.join(" "), &project_id);
                task.time_estimate = est;
                task.notes = notes;
                task.due_day = if today || tonight {
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
                            apply(&mut store, Action::AddTag { tag })?;
                            id
                        }
                    };
                    if !task.tag_ids.contains(&id) {
                        task.tag_ids.push(id);
                    }
                }
                let shown = task.clone();
                apply(&mut store, Action::AddTask { task, bottom: true })?;
                print_tasks(&store, &[&shown], json);
            }
            Cmd::Today => {
                let ids = store.state.today_ids();
                let tasks: Vec<&Task> = ids.iter().filter_map(|i| store.state.task.entities.get(i)).collect();
                print_tasks(&store, &tasks, json);
            }
            Cmd::Tonight => {
                let evening = store
                    .state
                    .tag
                    .iter()
                    .find(|t| t.title.eq_ignore_ascii_case("evening"))
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
                        t.due_day
                            .as_deref()
                            .and_then(day_number)
                            .is_some_and(|d| d > today_n && d <= today_n + days)
                    })
                    .collect();
                tasks.sort_by(|a, b| a.due_day.cmp(&b.due_day));
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
                let t = find_task(&store, &task)?.clone();
                apply(
                    &mut store,
                    Action::UpdateTask {
                        id: t.id.clone(),
                        changes: [("isDone".to_string(), serde_json::json!(true))].into_iter().collect(),
                    },
                )?;
                println!("Done: {}", t.title);
            }
            Cmd::Undone { task } => {
                let t = find_task(&store, &task)?.clone();
                apply(
                    &mut store,
                    Action::UpdateTask {
                        id: t.id.clone(),
                        changes: [("isDone".to_string(), serde_json::json!(false))].into_iter().collect(),
                    },
                )?;
                println!("Not done: {}", t.title);
            }
            Cmd::Plan { task } => {
                let t = find_task(&store, &task)?.clone();
                apply(
                    &mut store,
                    Action::PlanForToday {
                        task_ids: vec![t.id.clone()],
                        today: today_str(),
                    },
                )?;
                println!("Planned for today: {}", t.title);
            }
            Cmd::Rm { task } => {
                let t = find_task(&store, &task)?.clone();
                let subs: Vec<Task> = t
                    .sub_task_ids
                    .iter()
                    .filter_map(|i| store.state.task.entities.get(i).cloned())
                    .collect();
                apply(
                    &mut store,
                    Action::DeleteTask {
                        task: t.clone(),
                        sub_tasks: subs,
                    },
                )?;
                println!("Deleted: {}", t.title);
            }
            Cmd::Projects => {
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
                for g in store.state.tag.iter().filter(|g| g.id != TODAY_TAG_ID) {
                    println!("#{:<27} {} tasks", g.title, g.task_ids.len());
                }
            }
            Cmd::Sync => {
                if forward("\"sync\"")? {
                    println!("Sync requested from the running app.");
                    return Ok(());
                }
                let c = load_config(&dir);
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
                println!(
                    "Synced (downloaded: {}, uploaded: {} op{}).",
                    r.downloaded,
                    r.ops_uploaded,
                    if r.ops_uploaded == 1 { "" } else { "s" }
                );
            }
            Cmd::Config {
                server,
                user,
                folder,
                password,
                encryption_password,
            } => {
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
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                std::fs::write(dir.join("cli-config.json"), serde_json::to_vec_pretty(&c).unwrap())
                    .map_err(|e| e.to_string())?;
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
                println!(
                    "server:  {}\nuser:    {}\nfolder:  {}\ndata:    {}",
                    c.server.as_deref().unwrap_or("-"),
                    c.user.as_deref().unwrap_or("-"),
                    c.folder.as_deref().unwrap_or("super-productivity"),
                    dir.display()
                );
            }
        }
        Ok(())
    })();
    if let Err(e) = result {
        eprintln!("mo: {e}");
        std::process::exit(1);
    }
}

fn parse_estimate(s: &str) -> Option<f64> {
    let (mut total, mut num, mut any) = (0.0, String::new(), false);
    for c in s.chars() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c)
        } else if c == 'h' || c == 'm' {
            total += num.parse::<f64>().ok()? * if c == 'h' { 3_600_000.0 } else { 60_000.0 };
            num.clear();
            any = true;
        } else {
            return None;
        }
    }
    (any && num.is_empty()).then_some(total)
}
