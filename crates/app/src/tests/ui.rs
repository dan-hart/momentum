// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Window tests: every feature driven the way the UI drives it, on a headless display.
//! This file is a child module of `window` (see the `#[path]` there), so private methods
//! are in reach. Each test gets its own window over its own data directory. Store state is
//! read and written through `win.engine().with_store[_mut]`, since the window itself only
//! holds `Arc<momentum_core::Engine>` now.
use super::*;
use crate::tests::support::*;
use serde_json::json;
use sp_model::*;

fn today_n() -> i64 {
    day_number(&today_str()).unwrap()
}

#[test]
fn today_view_shows_only_today_morning_evening_groups() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let h = headings(&win);
        assert_eq!(h, ["Today", "Morning", "Evening"]);
        let rows = row_ids(&win);
        assert_eq!(rows[0], id_of(&win, "Renew library books"), "overdue first");
        let tonight = id_of(&win, "Read two chapters");
        let dentist = id_of(&win, "Dentist appointment");
        assert!(rows.iter().position(|r| *r == dentist) < rows.iter().position(|r| *r == tonight));
        assert!(!h.iter().any(|x| x.starts_with("Completed")), "nothing done yet");
        assert!(!shown(&*win.imp().empty) && shown(&*win.imp().task_box));
    });
}
#[test]
fn quick_add_parses_tags_and_estimate_and_plans_for_the_current_view() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        win.add_task("Buy milk #home 30m");
        pump();
        let id = id_of(&win, "Buy milk");
        let (due_day, estimate, tag_ids) = win.engine().with_store(|s| {
            let t = &s.state.task.entities[&id];
            (t.due_day.clone(), t.time_estimate, t.tag_ids.clone())
        });
        assert_eq!(due_day.as_deref(), Some(today_str().as_str()), "added from Today");
        assert_eq!(estimate, 1_800_000.0);
        let tag_id = win
            .engine()
            .with_store(|s| s.state.tag.iter().find(|g| g.title == "home").map(|g| g.id.clone()))
            .expect("tag created on the fly");
        assert_eq!(tag_ids, vec![tag_id]);
        assert!(row_ids(&win).contains(&id));
        win.add_task("   ");
        assert!(win
            .engine()
            .with_store(|s| s.state.task.iter().all(|t| !t.title.is_empty())));
    });
}
#[test]
fn adding_from_tonight_tags_evening_and_from_a_tag_view_applies_that_tag() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        win.go_to(View::Tonight);
        win.add_task("Wind down");
        let id = id_of(&win, "Wind down");
        assert!(row_ids(&win).contains(&id));
        let (has_evening, urgent) = win.engine().with_store(|s| {
            let evening = s.state.tag.iter().find(|g| g.title == "Evening").unwrap().id.clone();
            let has_evening = s.state.task.entities[&id].tag_ids.contains(&evening);
            let urgent = s.state.tag.iter().find(|g| g.title == "urgent").unwrap().id.clone();
            (has_evening, urgent)
        });
        assert!(has_evening);
        win.go_to(View::tag(&urgent));
        win.add_task("Fire drill");
        let id = id_of(&win, "Fire drill");
        assert!(win
            .engine()
            .with_store(|s| s.state.task.entities[&id].tag_ids.contains(&urgent)));
        assert!(row_ids(&win).contains(&id));
    });
}
#[test]
fn completing_moves_to_completed_section_and_undo_reverts() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let id = id_of(&win, "Write release notes for 0.1");
        win.set_done(&id, true);
        pump();
        assert!(win.engine().with_store(|s| s.state.task.entities[&id].is_done));
        assert!(headings(&win).contains(&"Completed (1)".to_string()));
        assert_eq!(row_ids(&win).last(), Some(&id), "completed tasks sink to the bottom");
        win.undo_last();
        pump();
        assert!(!win.engine().with_store(|s| s.state.task.entities[&id].is_done));
        assert!(!headings(&win).iter().any(|h| h.starts_with("Completed")));
    });
}
#[test]
fn all_done_panel_appears_when_every_task_is_complete() {
    on_gtk(|| {
        let (win, _dir) = empty_window();
        win.add_task("only one");
        let id = id_of(&win, "only one");
        win.set_done(&id, true);
        pump();
        let labels: Vec<String> = descendants(win.imp().task_box.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Label>().ok())
            .map(|l| l.label().to_string())
            .collect();
        assert!(labels.iter().any(|l| l.starts_with("All done")), "{labels:?}");
        assert!(win
            .lookup_action("archive-done")
            .and_downcast::<gio::SimpleAction>()
            .unwrap()
            .is_enabled());
    });
}
#[test]
fn archive_completed_moves_tasks_and_archive_view_lists_them() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let a = id_of(&win, "Write release notes for 0.1");
        let b = id_of(&win, "Read two chapters");
        win.set_done(&a, true);
        win.set_done(&b, true);
        win.archive_done();
        pump();
        let (has_a, has_b, archived_len) = win.engine().with_store(|s| {
            (
                s.state.task.entities.contains_key(&a),
                s.state.task.entities.contains_key(&b),
                s.state.rest["archiveYoung"]["task"]["ids"].as_array().unwrap().len(),
            )
        });
        assert!(!has_a && !has_b);
        assert_eq!(archived_len, 2);
        win.go_to(View::Archive);
        let rows = row_ids(&win);
        assert!(rows.contains(&a) && rows.contains(&b));
        win.undo_last();
        pump();
        assert!(
            win.engine().with_store(|s| s.state.task.entities.contains_key(&a)),
            "restore brings it back"
        );
        assert!(!win.engine().with_store(|s| s.state.task.entities[&a].is_done));
    });
}
#[test]
fn archive_view_pages_in_batches() {
    on_gtk(|| {
        let (win, _dir) = empty_window();
        let mut tasks = vec![];
        for i in 0..230 {
            let mut t = Task::new(&format!("old {i}"), INBOX_PROJECT_ID);
            t.is_done = true;
            t.done_on = Some(now_ms() - i);
            tasks.push(t);
        }
        win.dispatch(Action::MoveToArchive {
            tasks,
            sub_tasks: vec![],
        });
        win.go_to(View::Archive);
        let shown = row_ids(&win).len();
        assert!(shown < 230 && shown >= 100, "first page only: {shown}");
        let more = descendants(win.imp().task_box.upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::Button>().ok())
            .find(|b| b.label().is_some_and(|l| l.starts_with("Show More")))
            .expect("Show More button");
        more.emit_clicked();
        pump();
        assert_eq!(row_ids(&win).len(), 230);
    });
}
#[test]
fn selection_mode_bulk_done_today_and_delete_with_undo() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let a = id_of(&win, "Plan weekend hike");
        let (home, b) = win.engine().with_store(|s| {
            let home = s.state.task.entities[&a].project_id.clone();
            let b = s
                .state
                .task
                .iter()
                .find(|t| t.project_id == home && t.due_day.as_deref() > Some(today_str().as_str()))
                .map(|t| t.id.clone())
                .expect("an upcoming Home task");
            (home, b)
        });
        // Bulk actions apply to selected rows of the current view.
        win.go_to(View::project(&home));
        win.set_selecting(true);
        assert!(win.imp().select_bar.is_revealed());
        win.toggle_selected(&a);
        win.toggle_selected(&b);
        assert_eq!(win.imp().selected.borrow().len(), 2);
        assert!(win.imp().select_count.label().starts_with('2'));
        win.bulk_today();
        pump();
        assert!(win
            .engine()
            .with_store(|s| s.state.today_ids().contains(&a) && s.state.today_ids().contains(&b)));
        assert!(!win.imp().selecting.get(), "bulk actions leave selection mode");
        win.set_selecting(true);
        win.toggle_selected(&a);
        win.bulk_done();
        pump();
        assert!(win.engine().with_store(|s| s.state.task.entities[&a].is_done));
        win.set_selecting(true);
        win.toggle_selected(&b);
        win.bulk_delete();
        pump();
        assert!(!win.engine().with_store(|s| s.state.task.entities.contains_key(&b)));
        win.undo_last();
        pump();
        assert!(
            win.engine().with_store(|s| s.state.task.entities.contains_key(&b)),
            "bulk delete is undoable"
        );
    });
}
#[test]
fn day_moves_tomorrow_next_week_tonight_and_plan_today() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let id = id_of(&win, "Write release notes for 0.1");
        win.move_to_tomorrow(&[id.clone()]);
        pump();
        let due = |w: &MomentumWindow| w.engine().with_store(|s| s.state.task.entities[&id].due_day.clone());
        assert_eq!(due(&win).as_deref(), Some(day_str(today_n() + 1).as_str()));
        assert!(!row_ids(&win).contains(&id), "no longer in Today");
        win.move_to_next_week(&[id.clone()]);
        pump();
        let monday = day_number(&due(&win).unwrap()).unwrap();
        assert_eq!(weekday(monday), 1);
        assert!(monday > today_n() && monday <= today_n() + 7);
        win.undo_last();
        pump();
        assert_eq!(
            due(&win).as_deref(),
            Some(day_str(today_n() + 1).as_str()),
            "undo restores the previous day"
        );
        win.dispatch(Action::PlanForToday {
            task_ids: vec![id.clone()],
            today: today_str(),
        });
        assert!(row_ids(&win).contains(&id));
        win.toggle_tonight(&[id.clone()]);
        pump();
        let evening = win
            .engine()
            .with_store(|s| s.state.tag.iter().find(|g| g.title == "Evening").unwrap().id.clone());
        assert!(win
            .engine()
            .with_store(|s| s.state.task.entities[&id].tag_ids.contains(&evening)));
        win.go_to(View::Tonight);
        assert!(row_ids(&win).contains(&id));
        win.toggle_tonight(&[id.clone()]);
        pump();
        assert!(!win
            .engine()
            .with_store(|s| s.state.task.entities[&id].tag_ids.contains(&evening)));
    });
}
#[test]
fn drop_targets_move_between_projects_tags_and_today() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let id = id_of(&win, "Plan weekend hike");
        let (home, work, urgent) = win.engine().with_store(|s| {
            let home = s.state.project.iter().find(|p| p.title == "Home").unwrap().id.clone();
            let work = s
                .state
                .project
                .iter()
                .find(|p| p.title == "Momentum")
                .unwrap()
                .id
                .clone();
            let urgent = s.state.tag.iter().find(|g| g.title == "urgent").unwrap().id.clone();
            (home, work, urgent)
        });
        assert!(win.drop_task(&id, &View::project(&work)));
        assert!(!win.drop_task(&id, &View::project(&work)), "already there");
        win.engine().with_store(|s| {
            assert_eq!(s.state.task.entities[&id].project_id, work);
            assert!(!s.state.project.entities[&home].task_ids.contains(&id));
            assert!(s.state.project.entities[&work].task_ids.contains(&id));
        });
        assert!(win.drop_task(&id, &View::tag(&urgent)));
        assert!(win
            .engine()
            .with_store(|s| s.state.tag.entities[&urgent].task_ids.contains(&id)));
        assert!(win.drop_task(&id, &View::Today));
        assert!(win.engine().with_store(|s| s.state.today_ids().contains(&id)));
        assert!(win.drop_task(&id, &View::Tonight));
        let evening = win
            .engine()
            .with_store(|s| s.state.tag.iter().find(|g| g.title == "Evening").unwrap().id.clone());
        assert!(win
            .engine()
            .with_store(|s| s.state.task.entities[&id].tag_ids.contains(&evening)));
    });
}
#[test]
fn manual_reorder_and_nudge_change_the_stored_order() {
    on_gtk(|| {
        let (win, _dir) = empty_window();
        reset_settings();
        for t in ["one", "two", "three"] {
            win.add_task(t);
        }
        let (a, b, c) = (id_of(&win, "one"), id_of(&win, "two"), id_of(&win, "three"));
        assert_eq!(row_ids(&win), vec![a.clone(), b.clone(), c.clone()]);
        assert!(win.reorder(&c, &a));
        pump();
        assert_eq!(row_ids(&win), vec![c.clone(), a.clone(), b.clone()]);
        assert!(!win.reorder(&c, &c), "dropping on itself is a no-op");
        win.undo_last();
        pump();
        assert_eq!(
            row_ids(&win),
            vec![a.clone(), b.clone(), c.clone()],
            "a drag reorder is undoable"
        );
        settings().set_string("task-sort", "title").unwrap();
        assert!(!win.reorder(&c, &a), "manual order only");
        reset_settings();
    });
}
#[test]
fn sorting_by_title_estimate_and_direction() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let s = settings();
        reset_settings();
        s.set_string("task-sort", "title").unwrap();
        pump();
        // The plain Today section: due today, open, in neither the Morning nor the Tonight slot.
        let (evening, morning) = win.engine().with_store(|s| {
            let find = |name: &str| s.state.tag.iter().find(|g| g.title == name).unwrap().id.clone();
            (find("Evening"), find("Morning"))
        });
        let section = |w: &MomentumWindow| -> Vec<Task> {
            let ids = row_ids(w);
            w.engine().with_store(|s| {
                ids.iter()
                    .filter_map(|i| s.state.task.entities.get(i))
                    .filter(|t| {
                        t.plan_day().as_deref() == Some(today_str().as_str())
                            && !t.is_done
                            && !t.tag_ids.contains(&evening)
                            && !t.tag_ids.contains(&morning)
                    })
                    .cloned()
                    .collect()
            })
        };
        let titles: Vec<String> = section(&win).iter().map(|t| t.title.to_lowercase()).collect();
        let mut sorted = titles.clone();
        sorted.sort();
        assert!(titles.len() >= 3 && titles == sorted, "{titles:?}");
        s.set_string("sort-direction", "descending").unwrap();
        pump();
        let titles: Vec<String> = section(&win).iter().map(|t| t.title.to_lowercase()).collect();
        let mut desc = titles.clone();
        desc.sort_by(|a, b| b.cmp(a));
        assert_eq!(titles, desc);
        s.set_string("task-sort", "estimate").unwrap();
        s.set_string("sort-direction", "ascending").unwrap();
        pump();
        let ests: Vec<f64> = section(&win).iter().map(|t| t.time_estimate).collect();
        assert!(ests.windows(2).all(|w| w[0] <= w[1]), "{ests:?}");
        reset_settings();
    });
}

#[test]
fn coming_up_groups_by_day_and_honours_the_range() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        reset_settings();
        let far = {
            let mut t = Task::new("far away", INBOX_PROJECT_ID);
            t.due_day = Some(day_str(today_n() + 20));
            t
        };
        let far_id = far.id.clone();
        win.dispatch(Action::AddTask {
            task: far,
            bottom: true,
        });
        win.go_to(View::Upcoming);
        let rows = row_ids(&win);
        assert!(!rows.is_empty() && !rows.contains(&far_id), "7-day window");
        let h = headings(&win);
        assert!(
            h.iter().all(|x| ["Today", "Morning", "Evening"].contains(&x.as_str())),
            "{h:?}"
        );
        assert!(win
            .engine()
            .listing(View::Upcoming, 100)
            .sections
            .iter()
            .flat_map(|s| &s.rows)
            .all(|r| matches!(r, Row::Task { row } if row.day.is_some())));
        settings().set_string("upcoming-range", "30").unwrap();
        pump();
        assert!(row_ids(&win).contains(&far_id));
        reset_settings();
    });
}
#[test]
fn search_finds_tasks_notes_projects_and_tags_after_the_debounce() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        win.add_task_with_notes("Quiet title", Some("needle in the notes"), None);
        let noted = id_of(&win, "Quiet title");
        win.go_to(View::Search);
        win.set_search_query("orca");
        pump_ms(700); // GtkSearchEntry's own delay plus the 120 ms debounce
        assert!(row_ids(&win).contains(&id_of(&win, "Test with Orca and high contrast")));
        win.set_search_query("needle");
        pump_ms(700);
        assert!(row_ids(&win).contains(&noted), "notes are searched");
        win.set_search_query("gnome");
        pump_ms(700);
        let all: Vec<String> = win.imp().rows.borrow().clone();
        assert!(
            all.iter().any(|r| r.starts_with("tag:")),
            "tag results are listed: {all:?}"
        );
        win.set_search_query("zzz-nothing");
        pump_ms(700);
        assert!(row_ids(&win).is_empty());
    });
}
#[test]
fn context_menu_offers_the_right_moves_for_a_task() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let today_task = id_of(&win, "Write release notes for 0.1");
        let later = id_of(&win, "Plan weekend hike");
        let labels = |menu: &gio::Menu| -> Vec<String> {
            let mut out = vec![];
            fn walk(m: &gio::MenuModel, out: &mut Vec<String>) {
                for i in 0..m.n_items() {
                    if let Some(l) = m
                        .item_attribute_value(i, "label", None)
                        .and_then(|v| v.str().map(String::from))
                    {
                        out.push(l);
                    }
                    for link in ["section", "submenu"] {
                        if let Some(sub) = m.item_link(i, link) {
                            walk(&sub, out);
                        }
                    }
                }
            }
            walk(menu.upcast_ref(), &mut out);
            out
        };
        let l = labels(&win.context_menu_model(&MenuKind::Task, &today_task));
        assert!(l.iter().any(|x| x.contains("Remove from Today")), "{l:?}");
        assert!(l.iter().any(|x| x.contains("Tonight")) && l.iter().any(|x| x.contains("Tomorrow")));
        assert!(l.iter().any(|x| x.contains("Next Week")) && l.iter().any(|x| x.contains("Delete")));
        let l2 = labels(&win.context_menu_model(&MenuKind::Task, &later));
        assert!(l2.iter().any(|x| x.contains("Plan for Today")), "{l2:?}");
        let inbox = INBOX_PROJECT_ID.to_string();
        let home = win
            .engine()
            .with_store(|s| s.state.project.iter().find(|p| p.title == "Home").unwrap().id.clone());
        let p_inbox = labels(&win.context_menu_model(&MenuKind::Project, &inbox));
        let p_home = labels(&win.context_menu_model(&MenuKind::Project, &home));
        assert!(
            !p_inbox.iter().any(|x| x.contains("Delete")),
            "Inbox cannot be deleted: {p_inbox:?}"
        );
        assert!(p_home.iter().any(|x| x.contains("Delete")));
    });
}
#[test]
fn task_form_builds_a_task_with_time_reminder_estimate_and_tags() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let form = crate::task_form::TaskForm::new(&win, None, INBOX_PROJECT_ID, Some(today_str()));
        form.title.set_text("Dentist follow-up");
        form.estimate.set_text("45m");
        form.time.set_text("14:30");
        form.reminder.set_selected(4); // 15 minutes before
        form.new_tags.set_text("health, urgent");
        let draft = form.into_draft();
        assert_eq!(draft.title, "Dentist follow-up");
        assert_eq!(draft.estimate_ms, 2_700_000.0);
        assert_eq!(draft.time, Some(momentum_core::ClockTime { hour: 14, minute: 30 }));
        assert_eq!(draft.reminder_minutes_before, Some(15));
        assert_eq!(draft.new_tags, vec!["health".to_string(), "urgent".to_string()]);
        let out = win.engine().create_task(draft, View::Today);
        assert!(out.changed);
        let id = win.engine().last_added_id().unwrap();
        let (due_day, due_with_time, remind_at, tag_names) = win.engine().with_store(|s| {
            let t = &s.state.task.entities[&id];
            let names: Vec<String> = t
                .tag_ids
                .iter()
                .map(|i| s.state.tag.entities[i].title.clone())
                .collect();
            (t.due_day.clone(), t.due_with_time, t.remind_at, names)
        });
        assert!(due_day.is_none(), "a time replaces the plain day");
        assert_eq!(due_with_time, local_ms(&today_str(), 14, 30));
        assert_eq!(remind_at, due_with_time.map(|d| d - 15 * 60_000));
        assert_eq!(
            tag_names,
            vec!["health", "urgent"],
            "existing tag reused, new one created"
        );

        form.time.set_text("25:99");
        assert!(form.time.has_css_class("error"));
        assert!(form.time_value().is_none() && form.reminder_minutes_before().is_none());
        form.time.set_text("");
        assert!(!form.time.has_css_class("error"));
        let plain = form.into_draft();
        assert_eq!(plain.due_day.as_deref(), Some(today_str().as_str()));
    });
}
#[test]
fn editing_an_existing_task_prefills_the_form() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let id = id_of(&win, "Dentist appointment");
        let detail = win.engine().task_detail(id).unwrap();
        let form = crate::task_form::TaskForm::new(&win, Some(&detail), INBOX_PROJECT_ID, None);
        assert_eq!(form.title_text(), "Dentist appointment");
        assert_eq!(form.time.text(), "15:30");
        assert_eq!(form.reminder.selected(), 5, "30 minutes before");
        assert_eq!(form.estimate_ms(), 3_600_000.0);
        assert_eq!(form.due_day().as_deref(), Some(today_str().as_str()));
    });
}
#[test]
fn repeat_instances_are_spawned_once_per_day() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let today = today_str();
        let (newest, before) = win.engine().with_store(|s| {
            let cfg = s.state.task_repeat_cfg.entities["demo-weekly"].clone();
            (cfg.newest_due_day(&today), s.state.task.ids.len())
        });
        win.spawn_repeats();
        win.spawn_repeats();
        win.engine().with_store(|s| match &newest {
            Some(day) => {
                let t = &s.state.task.entities[&format!("rpt_demo-weekly_{day}")];
                assert_eq!(t.due_day.as_deref(), Some(day.as_str()));
                assert_eq!(t.repeat_cfg_id.as_deref(), Some("demo-weekly"));
                assert_eq!(s.state.task.ids.len(), before + 1, "never twice");
                assert_eq!(
                    s.state.task_repeat_cfg.entities["demo-weekly"]
                        .last_task_creation_day
                        .as_deref(),
                    Some(day.as_str())
                );
            }
            None => assert_eq!(s.state.task.ids.len(), before),
        });
    });
}
#[test]
fn repeat_catch_up_creates_the_missed_instance_dated_that_day() {
    on_gtk(|| {
        let (win, _dir) = empty_window();
        let missed = day_str(today_n() - 2);
        let mut cfg = RepeatCfg::for_task(&Task::new("Standup", INBOX_PROJECT_ID));
        cfg.id = "daily".into();
        cfg.repeat_cycle = "DAILY".into();
        cfg.start_date = Some(day_str(today_n() - 30));
        cfg.last_task_creation_day = Some(day_str(today_n() - 3));
        win.engine()
            .with_store_mut(|s| s.state.task_repeat_cfg.insert("daily", cfg));
        win.spawn_repeats();
        assert!(
            win.engine().with_store(|s| s
                .state
                .task
                .entities
                .contains_key(&format!("rpt_daily_{}", today_str()))),
            "daily: newest day is today"
        );
        let mut weekly = RepeatCfg::for_task(&Task::new("Weekly", INBOX_PROJECT_ID));
        weekly.id = "weekly".into();
        weekly.start_date = Some(day_str(today_n() - 60));
        weekly.last_task_creation_day = Some(day_str(today_n() - 9));
        let wd = weekday(today_n() - 2);
        weekly.monday = wd == 1;
        weekly.tuesday = wd == 2;
        weekly.wednesday = wd == 3;
        weekly.thursday = wd == 4;
        weekly.friday = wd == 5;
        weekly.saturday = wd == 6;
        weekly.sunday = wd == 0;
        win.engine()
            .with_store_mut(|s| s.state.task_repeat_cfg.insert("weekly", weekly));
        win.spawn_repeats();
        win.refresh(); // as the day-change timer and startup do after spawning
        let id = format!("rpt_weekly_{missed}");
        let (due_day, overdue) = win.engine().with_store(|s| {
            (
                s.state.task.entities[&id].due_day.clone(),
                s.state.overdue_ids().contains(&id),
            )
        });
        assert_eq!(due_day.as_deref(), Some(missed.as_str()));
        assert!(overdue);
        assert_eq!(headings(&win)[0], "Today");
        assert!(row_ids(&win).contains(&id));
    });
}
#[test]
fn undo_stack_reverses_several_changes_in_order() {
    on_gtk(|| {
        let (win, _dir) = empty_window();
        win.add_task("a");
        let a = id_of(&win, "a");
        win.set_done(&a, true);
        win.delete_task(&a);
        pump();
        assert!(!win.engine().with_store(|s| s.state.task.entities.contains_key(&a)));
        win.undo_last();
        pump();
        assert!(win.engine().with_store(|s| s.state.task.entities[&a].is_done));
        win.undo_last();
        pump();
        assert!(!win.engine().with_store(|s| s.state.task.entities[&a].is_done));
        win.undo_last(); // nothing left to undo: no panic, no change
        pump();
        assert!(win.engine().with_store(|s| s.state.task.entities.contains_key(&a)));
    });
}
#[test]
fn duplicate_copies_everything_but_identity() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let id = id_of(&win, "Dentist appointment");
        win.duplicate_task(&id);
        pump();
        win.engine().with_store(|s| {
            let copies: Vec<&Task> = s
                .state
                .task
                .iter()
                .filter(|t| t.title.starts_with("Dentist appointment"))
                .collect();
            assert_eq!(copies.len(), 2);
            let copy = copies.iter().find(|t| t.id != id).unwrap();
            assert_eq!(copy.time_estimate, s.state.task.entities[&id].time_estimate);
            assert_eq!(copy.tag_ids, s.state.task.entities[&id].tag_ids);
            assert!(!copy.is_done);
        });
    });
}
#[test]
fn views_cycle_through_the_sidebar_and_titles_follow() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let n = win.imp().views.borrow().iter().filter(|v| v.is_some()).count();
        assert!(n >= 5 + 3 + 3, "built-ins, projects, tags: {n}");
        let mut seen = vec![];
        for _ in 0..n {
            seen.push(win.imp().view.borrow().clone());
            gtk::prelude::WidgetExt::activate_action(&win, "win.next-view", None).unwrap();
        }
        assert_eq!(*win.imp().view.borrow(), View::Today, "wraps around");
        assert!(seen.contains(&View::Upcoming) && seen.contains(&View::Archive));
        win.go_to(View::Upcoming);
        assert_eq!(win.imp().content_page.title(), "Coming Up");
        gtk::prelude::WidgetExt::activate_action(&win, "win.prev-view", None).unwrap();
        assert_eq!(*win.imp().view.borrow(), View::Tonight);
        win.go_to(View::Today);
        assert_eq!(win.imp().content_page.title(), "Today");
    });
}
#[test]
fn empty_states_name_the_configured_modifier() {
    on_gtk(|| {
        let (win, _dir) = empty_window();
        reset_settings();
        let (_, _, desc) = win.empty_state();
        assert!(desc.contains("Ctrl+N"), "{desc}");
        assert!(shown(&*win.imp().empty) && !shown(&*win.imp().task_box));
        settings().set_string("modifier-key", "super").unwrap();
        let (_, _, desc) = win.empty_state();
        assert!(desc.contains("Super+N"), "{desc}");
        reset_settings();
    });
}
#[test]
fn modifier_setting_rewrites_accelerators_and_the_shortcuts_overlay() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        reset_settings();
        let app = app();
        assert_eq!(app.accels_for_action("win.archive-done"), vec!["<Control>e"]);
        settings().set_string("modifier-key", "super").unwrap();
        pump();
        assert_eq!(app.accels_for_action("win.archive-done"), vec!["<Super>e"]);
        assert_eq!(app.accels_for_action("win.toggle-sidebar"), vec!["F9"]);
        win.apply_modifier();
        #[allow(deprecated)]
        let accels: Vec<String> = descendants(win.help_overlay().expect("help overlay").upcast_ref())
            .into_iter()
            .filter_map(|w| w.downcast::<gtk::ShortcutsShortcut>().ok())
            .filter_map(|s| s.accelerator().map(|a| a.to_string()))
            .collect();
        assert!(accels.iter().any(|a| a == "<Super>e"), "{accels:?}");
        assert!(
            accels.iter().any(|a| a == "<Control><Alt>t"),
            "portal shortcuts stay Ctrl+Alt"
        );
        assert!(
            !accels
                .iter()
                .any(|a| a.starts_with("<Control>") && !a.contains("<Alt>")),
            "{accels:?}"
        );
        reset_settings();
        pump();
        assert_eq!(app.accels_for_action("win.archive-done"), vec!["<Control>e"]);
    });
}
#[test]
fn preferences_rows_are_bound_to_settings() {
    on_gtk(|| {
        let _win = demo_window();
        reset_settings();
        let prefs = crate::prefs::MomentumPrefs::default();
        let imp = prefs.imp();
        assert!(imp.colorful_row.is_active());
        imp.colorful_row.set_active(false);
        assert!(!settings().boolean("colorful-labels"));
        assert!(!imp.server_row.is_sensitive(), "connection rows follow the sync switch");
        imp.method_row.set_selected(1);
        assert!(crate::prefs::sync_method(&settings()) == "nextcloud" && imp.server_row.is_sensitive());
        imp.method_row.set_selected(0);
        imp.auto_archive_row.set_active(true);
        assert!(settings().boolean("auto-archive"));
        imp.auto_archive_row.set_active(false);
        let choices = crate::modifier::choices();
        let super_idx = choices.iter().position(|(k, _)| *k == "super").unwrap();
        imp.modifier_row.set_selected(super_idx as u32);
        assert_eq!(settings().string("modifier-key"), "super");
        reset_settings();
    });
}
#[test]
fn morning_summary_settings_are_opt_in_and_update_the_running_engine() {
    on_gtk(|| {
        use adw::subclass::prelude::*;
        reset_settings();
        let (win, _) = demo_window();
        let prefs = crate::prefs::MomentumPrefs::default();
        let imp = prefs.imp();
        assert!(!imp.morning_summary_row.is_active());
        assert!(!imp.morning_hour_row.is_sensitive());
        assert!(!win.engine().preferences().morning_summary_enabled);
        imp.morning_summary_row.set_active(true);
        imp.morning_hour_row.set_value(9.0);
        imp.morning_minute_row.set_value(45.0);
        pump();
        let saved = win.engine().preferences();
        assert!(saved.morning_summary_enabled);
        assert_eq!(
            saved.morning_summary_time,
            momentum_core::ClockTime { hour: 9, minute: 45 }
        );
        assert!(imp.morning_hour_row.is_sensitive());
        assert_eq!(settings().int("morning-summary-minute"), 45);
        imp.morning_summary_row.set_active(false);
        pump();
        assert!(!win.engine().preferences().morning_summary_enabled);
        assert!(!imp.morning_minute_row.is_sensitive());
        assert_eq!(
            win.engine().preferences().morning_summary_time,
            saved.morning_summary_time
        );
        settings().set_int("morning-summary-hour", 10).unwrap();
        assert_eq!(
            imp.morning_hour_row.value(),
            10.0,
            "external preference changes reach the UI"
        );
        reset_settings();
        win.close();
    });
}

#[test]
fn colourful_labels_follow_the_preference() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        reset_settings();
        assert!(win.colorful());
        settings().set_boolean("colorful-labels", false).unwrap();
        assert!(!win.colorful());
        reset_settings();
        assert!(win.color_class("#ff0000").is_some());
        assert!(win.color_class("").is_none());
    });
}
#[test]
fn quick_add_window_creates_a_task_for_today() {
    on_gtk(|| {
        let app = app();
        let main = app.ensure_window();
        let before = main.engine().with_store(|s| s.state.task.ids.len());
        crate::quick_add::open(&app);
        pump();
        let qa = app
            .windows()
            .into_iter()
            .find(|w| w.title().is_some_and(|t| t.contains("Add")) || w.downcast_ref::<MomentumWindow>().is_none())
            .expect("quick add window");
        let entry = descendants(qa.upcast_ref())
            .into_iter()
            .find_map(|w| w.downcast::<gtk::Entry>().ok())
            .expect("entry");
        entry.set_text("From quick add #quick");
        entry.emit_activate();
        pump();
        main.engine().with_store(|s| {
            assert_eq!(s.state.task.ids.len(), before + 1);
            let t = s.state.task.iter().find(|t| t.title == "From quick add").unwrap();
            assert_eq!(t.due_day.as_deref(), Some(today_str().as_str()));
            assert!(s.state.tag.iter().any(|g| g.title == "quick"));
        });
    });
}
#[test]
fn search_provider_matches_terms_and_offers_creation() {
    on_gtk(|| {
        let app = app();
        let ids = crate::search_provider::results(&app, &["orca".into()]);
        assert!(ids.iter().any(|i| !i.starts_with("add:")), "{ids:?}");
        let (name, desc) = crate::search_provider::meta(&app, &ids[0]);
        assert!(name.contains("Orca") || desc.contains("Orca"));
        let none = crate::search_provider::results(&app, &["zzz-nothing".into()]);
        assert_eq!(none.len(), 1);
        assert!(none[0].starts_with("add:"), "offers to create the task: {none:?}");
    });
}
#[test]
fn paste_and_drop_text_become_tasks() {
    on_gtk(|| {
        let (win, _dir) = empty_window();
        win.add_from_text("first thing\nsecond thing\n\n");
        assert_eq!(win.engine().with_store(|s| s.state.task.ids.len()), 2);
        win.add_from_text("https://example.org/page/");
        let notes = win.engine().with_store(|s| {
            s.state
                .task
                .iter()
                .find(|t| t.title == "example.org/page")
                .expect("link title")
                .notes
                .clone()
        });
        assert_eq!(notes.as_deref(), Some("https://example.org/page/"));
        let long = "A rather long paragraph ".repeat(10);
        win.add_from_text(&long);
        let (len_ok, has_notes) = win.engine().with_store(|s| {
            let t = s.state.task.iter().last().unwrap();
            (t.title.chars().count() <= 120, t.notes.is_some())
        });
        assert!(len_ok && has_notes, "long text is title + notes");
    });
}
#[test]
fn reminders_fire_once_for_due_tasks() {
    on_gtk(|| {
        let (win, _dir) = empty_window();
        let mut t = Task::new("Ping", INBOX_PROJECT_ID);
        t.remind_at = Some(now_ms() - 1000);
        let id = t.id.clone();
        win.dispatch(Action::AddTask { task: t, bottom: true });
        win.check_reminders();
        assert!(win.engine().notified_contains(&id));
        win.snooze_task(&id, 60);
        let at = win
            .engine()
            .with_store(|s| s.state.task.entities[&id].remind_at.unwrap());
        assert!(at > now_ms() + 59 * 60_000);
        assert!(!win.engine().notified_contains(&id), "a snooze re-arms the reminder");
        win.check_reminders();
        assert!(
            !win.engine().notified_contains(&id),
            "and it does not fire before its time"
        );
    });
}
#[test]
fn project_and_tag_management_from_the_sidebar() {
    on_gtk(|| {
        let (win, _dir) = empty_window();
        win.add_project("Garden");
        let pid = win
            .engine()
            .with_store(|s| s.state.project.iter().find(|p| p.title == "Garden").unwrap().id.clone());
        assert!(win
            .imp()
            .views
            .borrow()
            .iter()
            .flatten()
            .any(|v| *v == View::project(&pid)));
        win.go_to(View::project(&pid));
        win.add_task("Water plants");
        let tid = id_of(&win, "Water plants");
        assert_eq!(
            win.engine()
                .with_store(|s| s.state.task.entities[&tid].project_id.clone()),
            pid
        );
        win.dispatch(Action::UpdateProject {
            id: pid.clone(),
            changes: [("title".to_string(), json!("Yard"))].into_iter().collect(),
        });
        assert_eq!(win.imp().content_page.title(), "Yard");
        win.dispatch(Action::DeleteProject {
            project_id: pid.clone(),
            note_ids: vec![],
            all_task_ids: vec![tid.clone()],
        });
        assert!(!win.engine().with_store(|s| s.state.task.entities.contains_key(&tid)));
        assert_eq!(
            *win.imp().view.borrow(),
            View::Today,
            "falls back to Today when the view is gone"
        );
    });
}

#[test]
fn morning_view_toggle_and_drop_mirror_tonight() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let stretch = id_of(&win, "Stretch and plan the day");
        let notes = id_of(&win, "Write release notes for 0.1");
        let read = id_of(&win, "Read two chapters");
        win.go_to(View::Morning);
        assert_eq!(row_ids(&win), vec![stretch.clone()]);
        assert_eq!(win.imp().content_page.title(), "Morning");
        // A task added here is planned for today and tagged Morning.
        win.add_task("Coffee first");
        let coffee = id_of(&win, "Coffee first");
        let (morning, evening) = win.engine().with_store(|s| {
            let find = |name: &str| s.state.tag.iter().find(|g| g.title == name).unwrap().id.clone();
            (find("Morning"), find("Evening"))
        });
        assert!(win
            .engine()
            .with_store(|s| s.state.task.entities[&coffee].tag_ids.contains(&morning)));
        assert!(row_ids(&win).contains(&coffee));
        // Toggling moves a plain task in, and an evening task swaps slots (never both tags).
        win.toggle_morning(&[notes.clone(), read.clone()]);
        pump();
        win.engine().with_store(|s| {
            for id in [&notes, &read] {
                let t = &s.state.task.entities[id];
                assert!(
                    t.tag_ids.contains(&morning) && !t.tag_ids.contains(&evening),
                    "{}",
                    t.title
                );
            }
        });
        win.undo_last();
        pump();
        assert!(
            win.engine()
                .with_store(|s| s.state.task.entities[&read].tag_ids.contains(&evening)),
            "undo restores the evening tag"
        );
        // Toggling a morning task back sends it to the plain day.
        win.toggle_morning(&[stretch.clone()]);
        pump();
        assert!(!win
            .engine()
            .with_store(|s| s.state.task.entities[&stretch].tag_ids.contains(&morning)));
        assert!(!row_ids(&win).contains(&stretch));
        // Drop targets: Morning plans and tags; dropping a morning task on Tonight swaps.
        let hike = id_of(&win, "Plan weekend hike");
        assert!(win.drop_task(&hike, &View::Morning));
        assert!(!win.drop_task(&hike, &View::Morning), "already there");
        assert!(win.drop_task(&hike, &View::Tonight));
        let (has_evening, has_morning, due_day) = win.engine().with_store(|s| {
            let t = &s.state.task.entities[&hike];
            (
                t.tag_ids.contains(&evening),
                t.tag_ids.contains(&morning),
                t.due_day.clone(),
            )
        });
        assert!(has_evening && !has_morning);
        assert_eq!(due_day.as_deref(), Some(today_str().as_str()));
        // Context menu offers the move for each slot; accelerator is bound.
        let menu = win.context_menu_model(&MenuKind::Task, &coffee);
        let mut labels = vec![];
        for i in 0..menu.n_items() {
            if let Some(sec) = menu.item_link(i, "section") {
                for j in 0..sec.n_items() {
                    if let Some(l) = sec.item_attribute_value(j, "label", None) {
                        labels.push(l.str().unwrap_or_default().to_string());
                    }
                }
            }
        }
        assert!(
            labels.iter().any(|l| l == "Move to Today") && labels.iter().any(|l| l == "Move to Tonight"),
            "{labels:?}"
        );
        assert_eq!(app().accels_for_action("win.toggle-morning"), vec!["<Shift><Control>m"]);
        // Emptying the slot removes its sidebar entry and falls back to Today.
        win.go_to(View::Morning);
        for id in row_ids(&win) {
            win.toggle_morning(&[id]);
        }
        pump();
        assert_eq!(*win.imp().view.borrow(), View::Today);
        assert!(!win.imp().views.borrow().iter().flatten().any(|v| *v == View::Morning));
        let (_, title, _) = win.empty_state();
        assert!(!title.is_empty());
    });
}

#[test]
fn morning_and_tonight_entries_appear_only_when_today_uses_them() {
    on_gtk(|| {
        let has = |w: &MomentumWindow, v: View| w.imp().views.borrow().iter().flatten().any(|x| *x == v);
        let (win, _dir) = empty_window();
        assert!(
            !has(&win, View::Morning) && !has(&win, View::Tonight),
            "empty store: neither entry"
        );
        win.add_task("Plain");
        let id = id_of(&win, "Plain");
        assert!(!has(&win, View::Morning));
        win.toggle_morning(&[id.clone()]);
        pump();
        assert!(has(&win, View::Morning) && !has(&win, View::Tonight));
        win.go_to(View::Morning);
        assert_eq!(row_ids(&win), vec![id.clone()]);
        // Moving the only morning task away removes the entry and drops back to Today.
        win.toggle_tonight(&[id.clone()]);
        pump();
        assert!(!has(&win, View::Morning) && has(&win, View::Tonight));
        assert_eq!(*win.imp().view.borrow(), View::Today);
        assert!(row_ids(&win).contains(&id));
        // A task tagged for the slot but planned for another day does not count.
        win.move_to_tomorrow(&[id.clone()]);
        pump();
        assert!(!has(&win, View::Tonight));
        let (demo, _d2) = demo_window();
        assert!(
            has(&demo, View::Morning) && has(&demo, View::Tonight),
            "demo data has both"
        );
    });
}

#[test]
fn auto_archive_sends_completed_tasks_straight_to_the_archive() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        reset_settings();
        let id = id_of(&win, "Write release notes for 0.1");
        // Off by default: completing keeps the task in a Completed section.
        win.set_done(&id, true);
        pump();
        assert!(win.engine().with_store(|s| s.state.task.entities.contains_key(&id)));
        win.undo_last();
        pump();
        settings().set_boolean("auto-archive", true).unwrap();
        win.set_done(&id, true);
        pump();
        win.engine().with_store(|s| {
            assert!(!s.state.task.entities.contains_key(&id), "gone from the live list");
            assert_eq!(s.state.rest["archiveYoung"]["task"]["entities"][&id]["isDone"], true);
        });
        assert!(!headings(&win).iter().any(|h| h.starts_with("Completed")));
        win.undo_last();
        pump();
        assert!(
            !win.engine().with_store(|s| s.state.task.entities[&id].is_done),
            "undo restores it as an open task"
        );
        // Bulk completion archives the whole selection in one undoable batch.
        let a = id_of(&win, "Read two chapters");
        let b = id_of(&win, "Prep tomorrow's lunch");
        win.set_selecting(true);
        win.toggle_selected(&a);
        win.toggle_selected(&b);
        win.bulk_done();
        pump();
        win.engine().with_store(|s| {
            assert!(!s.state.task.entities.contains_key(&a) && !s.state.task.entities.contains_key(&b));
            assert_eq!(s.state.rest["archiveYoung"]["task"]["ids"].as_array().unwrap().len(), 2);
        });
        win.undo_last();
        pump();
        assert!(win
            .engine()
            .with_store(|s| s.state.task.entities.contains_key(&a) && !s.state.task.entities[&b].is_done));
        // Reopening (done -> not done) never archives.
        settings().set_boolean("auto-archive", true).unwrap();
        win.set_done(&a, false);
        pump();
        assert!(win.engine().with_store(|s| s.state.task.entities.contains_key(&a)));
        reset_settings();
    });
}

#[test]
fn sync_status_remains_visible_in_empty_lists_and_errors_persist() {
    on_gtk(|| {
        reset_settings();
        settings().set_boolean("auto-sync", false).unwrap();
        let (win, _) = empty_window();
        settings().set_string("sync-method", "nextcloud").unwrap();
        pump();
        assert!(shown(&*win.imp().sync_label));
        assert!(win.imp().sync_label.text().contains("Nextcloud"));
        assert!(!win.p2p_enabled());
        let config_path = std::path::PathBuf::from(win.engine().data_dir()).join("cli-config.json");
        let config: serde_json::Value = serde_json::from_slice(&std::fs::read(&config_path).unwrap()).unwrap();
        assert_eq!(config["method"], "nextcloud");
        win.set_sync_error(Some("Connection failed".into()));
        win.refresh();
        assert!(win.imp().banner.is_revealed());
        assert!(win.imp().sync_label.text().contains("needs attention"));
        settings().set_string("sync-method", "off").unwrap();
        pump();
        assert!(!win.imp().banner.is_revealed());
        assert!(win.imp().sync_label.text().contains("off"));
        let config: serde_json::Value = serde_json::from_slice(&std::fs::read(config_path).unwrap()).unwrap();
        assert_eq!(config["method"], "off");
        reset_settings();
    });
}

#[test]
fn grouping_settings_refresh_native_headings_without_losing_tasks() {
    on_gtk(|| {
        reset_settings();
        let (win, _dir) = demo_window();
        let original = row_ids(&win);
        let original_headings = headings(&win);
        let pending = win.engine().pending_count();
        for (choice, expected) in [
            ("project", GroupBy::Project),
            ("tag", GroupBy::Tag),
            ("estimate", GroupBy::Estimate),
        ] {
            // The same settings-backed action used by the native radio menu.
            win.lookup_action("group-by")
                .unwrap()
                .change_state(&choice.to_variant());
            pump();
            assert_eq!(settings().string("group-by"), choice);
            assert_eq!(win.engine().preferences().group_by, expected);
            assert_ne!(headings(&win), original_headings);
            assert!(win
                .engine()
                .listing(View::Today, 100)
                .sections
                .iter()
                .all(|s| s.kind == SectionKind::Plain));
            if choice == "tag" || choice == "project" {
                let name = if choice == "tag" { "#urgent" } else { "Momentum" };
                let tag_heading = descendants(win.imp().task_box.upcast_ref())
                    .into_iter()
                    .filter_map(|w| w.downcast::<gtk::Label>().ok())
                    .find(|l| l.has_css_class("heading") && l.text().contains(name))
                    .unwrap();
                if win.colorful() {
                    assert!(tag_heading.uses_markup(), "group name uses its saved color");
                }
                settings().set_boolean("colorful-labels", false).unwrap();
                pump();
                let neutral = descendants(win.imp().task_box.upcast_ref())
                    .into_iter()
                    .filter_map(|w| w.downcast::<gtk::Label>().ok())
                    .find(|l| l.has_css_class("heading") && l.text().contains(name))
                    .unwrap();
                assert!(!neutral.uses_markup(), "color preference applies to group headings too");
                settings().set_boolean("colorful-labels", true).unwrap();
                pump();
            }
            let mut current = row_ids(&win);
            current.sort();
            let mut all = original.clone();
            all.sort();
            assert_eq!(current, all);
        }
        assert!(headings(&win).iter().any(|s| s.contains("No estimate")));
        assert_eq!(win.engine().pending_count(), pending);
        settings().set_string("group-by", "none").unwrap();
        pump();
        assert!(headings(&win).is_empty());
        settings().set_string("group-by", "morning-night").unwrap();
        pump();
        assert_eq!(row_ids(&win), original);
        assert_eq!(headings(&win), original_headings);
        reset_settings();
    });
}
