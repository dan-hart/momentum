// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Window tests: every feature driven the way the UI drives it, on a headless display.
//! This file is a child module of `window` (see the `#[path]` there), so private methods
//! are in reach. Each test gets its own window over its own data directory.
use super::*;
use crate::tests::support::*;

fn today_n() -> i64 {
    day_number(&today_str()).unwrap()
}

#[test]
fn today_view_shows_overdue_today_tonight_and_completed_sections() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let h = headings(&win);
        assert_eq!(h[0], "Overdue (1)");
        assert!(
            h.contains(&"Today".to_string()) && h.contains(&"Tonight".to_string()),
            "{h:?}"
        );
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
        let store = win.imp().store.borrow();
        let t = &store.state.task.entities[&id];
        assert_eq!(t.due_day.as_deref(), Some(today_str().as_str()), "added from Today");
        assert_eq!(t.time_estimate, 1_800_000.0);
        let tag = store
            .state
            .tag
            .iter()
            .find(|g| g.title == "home")
            .expect("tag created on the fly");
        assert_eq!(t.tag_ids, vec![tag.id.clone()]);
        drop(store);
        assert!(row_ids(&win).contains(&id));
        win.add_task("   ");
        assert!(win.imp().store.borrow().state.task.iter().all(|t| !t.title.is_empty()));
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
        let store = win.imp().store.borrow();
        let evening = store
            .state
            .tag
            .iter()
            .find(|g| g.title == "Evening")
            .unwrap()
            .id
            .clone();
        assert!(store.state.task.entities[&id].tag_ids.contains(&evening));
        let urgent = store.state.tag.iter().find(|g| g.title == "urgent").unwrap().id.clone();
        drop(store);
        win.go_to(View::Tag(urgent.clone()));
        win.add_task("Fire drill");
        let id = id_of(&win, "Fire drill");
        assert!(win.imp().store.borrow().state.task.entities[&id]
            .tag_ids
            .contains(&urgent));
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
        assert!(win.imp().store.borrow().state.task.entities[&id].is_done);
        assert!(headings(&win).contains(&"Completed (1)".to_string()));
        assert_eq!(row_ids(&win).last(), Some(&id), "completed tasks sink to the bottom");
        win.undo_last();
        pump();
        assert!(!win.imp().store.borrow().state.task.entities[&id].is_done);
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
        {
            let store = win.imp().store.borrow();
            assert!(!store.state.task.entities.contains_key(&a) && !store.state.task.entities.contains_key(&b));
            assert_eq!(
                store.state.rest["archiveYoung"]["task"]["ids"]
                    .as_array()
                    .unwrap()
                    .len(),
                2
            );
        }
        win.go_to(View::Archive);
        let rows = row_ids(&win);
        assert!(rows.contains(&a) && rows.contains(&b));
        win.undo_last();
        pump();
        assert!(
            win.imp().store.borrow().state.task.entities.contains_key(&a),
            "restore brings it back"
        );
        assert!(!win.imp().store.borrow().state.task.entities[&a].is_done);
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
        let (home, b) = {
            let store = win.imp().store.borrow();
            let home = store.state.task.entities[&a].project_id.clone();
            let b = store
                .state
                .task
                .iter()
                .find(|t| t.project_id == home && t.due_day.as_deref() > Some(today_str().as_str()))
                .map(|t| t.id.clone())
                .expect("an upcoming Home task");
            (home, b)
        };
        // Bulk actions apply to selected rows of the current view.
        win.go_to(View::Project(home));
        win.set_selecting(true);
        assert!(win.imp().select_bar.is_revealed());
        win.toggle_selected(&a);
        win.toggle_selected(&b);
        assert_eq!(win.imp().selected.borrow().len(), 2);
        assert!(win.imp().select_count.label().starts_with('2'));
        win.bulk_today();
        pump();
        {
            let store = win.imp().store.borrow();
            assert!(store.state.today_ids().contains(&a) && store.state.today_ids().contains(&b));
        }
        assert!(!win.imp().selecting.get(), "bulk actions leave selection mode");
        win.set_selecting(true);
        win.toggle_selected(&a);
        win.bulk_done();
        pump();
        assert!(win.imp().store.borrow().state.task.entities[&a].is_done);
        win.set_selecting(true);
        win.toggle_selected(&b);
        win.bulk_delete();
        pump();
        assert!(!win.imp().store.borrow().state.task.entities.contains_key(&b));
        win.undo_last();
        pump();
        assert!(
            win.imp().store.borrow().state.task.entities.contains_key(&b),
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
        let due = |w: &MomentumWindow| w.imp().store.borrow().state.task.entities[&id].due_day.clone();
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
            .imp()
            .store
            .borrow()
            .state
            .tag
            .iter()
            .find(|g| g.title == "Evening")
            .unwrap()
            .id
            .clone();
        assert!(win.imp().store.borrow().state.task.entities[&id]
            .tag_ids
            .contains(&evening));
        win.go_to(View::Tonight);
        assert!(row_ids(&win).contains(&id));
        win.toggle_tonight(&[id.clone()]);
        pump();
        assert!(!win.imp().store.borrow().state.task.entities[&id]
            .tag_ids
            .contains(&evening));
    });
}
#[test]
fn drop_targets_move_between_projects_tags_and_today() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let id = id_of(&win, "Plan weekend hike");
        let store = win.imp().store.borrow();
        let home = store
            .state
            .project
            .iter()
            .find(|p| p.title == "Home")
            .unwrap()
            .id
            .clone();
        let work = store
            .state
            .project
            .iter()
            .find(|p| p.title == "Momentum")
            .unwrap()
            .id
            .clone();
        let urgent = store.state.tag.iter().find(|g| g.title == "urgent").unwrap().id.clone();
        drop(store);
        assert!(win.drop_task(&id, &View::Project(work.clone())));
        assert!(!win.drop_task(&id, &View::Project(work.clone())), "already there");
        {
            let store = win.imp().store.borrow();
            assert_eq!(store.state.task.entities[&id].project_id, work);
            assert!(!store.state.project.entities[&home].task_ids.contains(&id));
            assert!(store.state.project.entities[&work].task_ids.contains(&id));
        }
        assert!(win.drop_task(&id, &View::Tag(urgent.clone())));
        assert!(win.imp().store.borrow().state.tag.entities[&urgent]
            .task_ids
            .contains(&id));
        assert!(win.drop_task(&id, &View::Today));
        assert!(win.imp().store.borrow().state.today_ids().contains(&id));
        assert!(win.drop_task(&id, &View::Tonight));
        let evening = win
            .imp()
            .store
            .borrow()
            .state
            .tag
            .iter()
            .find(|g| g.title == "Evening")
            .unwrap()
            .id
            .clone();
        assert!(win.imp().store.borrow().state.task.entities[&id]
            .tag_ids
            .contains(&evening));
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
        let titles = |w: &MomentumWindow| -> Vec<String> {
            let store = w.imp().store.borrow();
            row_ids(w)
                .iter()
                .filter_map(|i| store.state.task.entities.get(i))
                .filter(|t| t.due_day.as_deref() == Some(today_str().as_str()) && !t.is_done)
                .map(|t| t.title.to_lowercase())
                .collect()
        };
        // Within the Today section (due today, not Evening) titles are ascending.
        let evening = win
            .imp()
            .store
            .borrow()
            .state
            .tag
            .iter()
            .find(|g| g.title == "Evening")
            .unwrap()
            .id
            .clone();
        let today_titles = |w: &MomentumWindow| -> Vec<String> {
            let store = w.imp().store.borrow();
            row_ids(w)
                .iter()
                .filter_map(|i| store.state.task.entities.get(i))
                .filter(|t| {
                    t.plan_day().as_deref() == Some(today_str().as_str()) && !t.is_done && !t.tag_ids.contains(&evening)
                })
                .map(|t| t.title.to_lowercase())
                .collect()
        };
        let t = today_titles(&win);
        let mut sorted = t.clone();
        sorted.sort();
        assert!(t.len() >= 3 && t == sorted, "{t:?}");
        s.set_string("sort-direction", "descending").unwrap();
        pump();
        let t2 = today_titles(&win);
        let mut desc = t2.clone();
        desc.sort_by(|a, b| b.cmp(a));
        assert_eq!(t2, desc);
        let _ = titles(&win);
        s.set_string("task-sort", "estimate").unwrap();
        s.set_string("sort-direction", "ascending").unwrap();
        pump();
        let store = win.imp().store.borrow();
        let ests: Vec<f64> = row_ids(&win)
            .iter()
            .filter_map(|i| store.state.task.entities.get(i))
            .filter(|t| t.plan_day().as_deref() == Some(today_str().as_str()) && !t.tag_ids.contains(&evening))
            .map(|t| t.time_estimate)
            .collect();
        assert!(ests.windows(2).all(|w| w[0] <= w[1]), "{ests:?}");
        drop(store);
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
        assert!(h.iter().any(|x| x == "Tomorrow"), "{h:?}");
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
        let store = win.imp().store.borrow();
        let inbox = INBOX_PROJECT_ID.to_string();
        let home = store
            .state
            .project
            .iter()
            .find(|p| p.title == "Home")
            .unwrap()
            .id
            .clone();
        drop(store);
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
        let form = {
            let store = win.imp().store.borrow();
            crate::task_form::TaskForm::new(&win, &store, None, INBOX_PROJECT_ID, Some(today_str()))
        };
        form.title.set_text("Dentist follow-up");
        form.estimate.set_text("45m");
        form.time.set_text("14:30");
        form.reminder.set_selected(4); // 15 minutes before
        form.new_tags.set_text("health, urgent");
        let t = form.into_task(&mut win.imp().store.borrow_mut());
        assert_eq!(t.title, "Dentist follow-up");
        assert_eq!(t.time_estimate, 2_700_000.0);
        assert!(t.due_day.is_none(), "a time replaces the plain day");
        assert_eq!(t.due_with_time, local_ms(&today_str(), 14, 30));
        assert_eq!(t.remind_at, t.due_with_time.map(|d| d - 15 * 60_000));
        let store = win.imp().store.borrow();
        let names: Vec<String> = t
            .tag_ids
            .iter()
            .map(|i| store.state.tag.entities[i].title.clone())
            .collect();
        assert_eq!(names, vec!["health", "urgent"], "existing tag reused, new one created");
        drop(store);
        form.time.set_text("25:99");
        assert!(form.time.has_css_class("error"));
        assert!(form.due_with_time().is_none() && form.remind_at().is_none());
        form.time.set_text("");
        assert!(!form.time.has_css_class("error"));
        let plain = form.into_task(&mut win.imp().store.borrow_mut());
        assert_eq!(plain.due_day.as_deref(), Some(today_str().as_str()));
    });
}
#[test]
fn editing_an_existing_task_prefills_the_form() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let id = id_of(&win, "Dentist appointment");
        let form = {
            let store = win.imp().store.borrow();
            let t = store.state.task.entities[&id].clone();
            crate::task_form::TaskForm::new(&win, &store, Some(&t), INBOX_PROJECT_ID, None)
        };
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
        let cfg = win.imp().store.borrow().state.task_repeat_cfg.entities["demo-weekly"].clone();
        let newest = cfg.newest_due_day(&today);
        let before = win.imp().store.borrow().state.task.ids.len();
        win.spawn_repeats();
        win.spawn_repeats();
        let store = win.imp().store.borrow();
        match newest {
            Some(day) => {
                let t = &store.state.task.entities[&format!("rpt_demo-weekly_{day}")];
                assert_eq!(t.due_day.as_deref(), Some(day.as_str()));
                assert_eq!(t.repeat_cfg_id.as_deref(), Some("demo-weekly"));
                assert_eq!(store.state.task.ids.len(), before + 1, "never twice");
                assert_eq!(
                    store.state.task_repeat_cfg.entities["demo-weekly"]
                        .last_task_creation_day
                        .as_deref(),
                    Some(day.as_str())
                );
            }
            None => assert_eq!(store.state.task.ids.len(), before),
        }
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
        win.imp().store.borrow_mut().state.task_repeat_cfg.insert("daily", cfg);
        win.spawn_repeats();
        let store = win.imp().store.borrow();
        assert!(
            store
                .state
                .task
                .entities
                .contains_key(&format!("rpt_daily_{}", today_str())),
            "daily: newest day is today"
        );
        drop(store);
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
        win.imp()
            .store
            .borrow_mut()
            .state
            .task_repeat_cfg
            .insert("weekly", weekly);
        win.spawn_repeats();
        win.refresh(); // as the day-change timer and startup do after spawning
        let id = format!("rpt_weekly_{missed}");
        let store = win.imp().store.borrow();
        assert_eq!(store.state.task.entities[&id].due_day.as_deref(), Some(missed.as_str()));
        assert!(store.state.overdue_ids().contains(&id));
        drop(store);
        assert_eq!(headings(&win)[0], "Overdue (1)");
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
        assert!(!win.imp().store.borrow().state.task.entities.contains_key(&a));
        win.undo_last();
        pump();
        assert!(win.imp().store.borrow().state.task.entities[&a].is_done);
        win.undo_last();
        pump();
        assert!(!win.imp().store.borrow().state.task.entities[&a].is_done);
        win.undo_last(); // nothing left to undo: no panic, no change
        pump();
        assert!(win.imp().store.borrow().state.task.entities.contains_key(&a));
    });
}
#[test]
fn duplicate_copies_everything_but_identity() {
    on_gtk(|| {
        let (win, _dir) = demo_window();
        let id = id_of(&win, "Dentist appointment");
        win.duplicate_task(&id);
        pump();
        let store = win.imp().store.borrow();
        let copies: Vec<&Task> = store
            .state
            .task
            .iter()
            .filter(|t| t.title.starts_with("Dentist appointment"))
            .collect();
        assert_eq!(copies.len(), 2);
        let copy = copies.iter().find(|t| t.id != id).unwrap();
        assert_eq!(copy.time_estimate, store.state.task.entities[&id].time_estimate);
        assert_eq!(copy.tag_ids, store.state.task.entities[&id].tag_ids);
        assert!(!copy.is_done);
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
        let (_, _, desc) = win.empty_state(&win.imp().store.borrow());
        assert!(desc.contains("Ctrl+N"), "{desc}");
        assert!(shown(&*win.imp().empty) && !shown(&*win.imp().task_box));
        settings().set_string("modifier-key", "super").unwrap();
        let (_, _, desc) = win.empty_state(&win.imp().store.borrow());
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
        imp.enabled_row.set_active(true);
        assert!(settings().boolean("sync-enabled") && imp.server_row.is_sensitive());
        imp.enabled_row.set_active(false);
        let choices = crate::modifier::choices();
        let super_idx = choices.iter().position(|(k, _)| *k == "super").unwrap();
        imp.modifier_row.set_selected(super_idx as u32);
        assert_eq!(settings().string("modifier-key"), "super");
        reset_settings();
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
        let before = main.imp().store.borrow().state.task.ids.len();
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
        let store = main.imp().store.borrow();
        assert_eq!(store.state.task.ids.len(), before + 1);
        let t = store.state.task.iter().find(|t| t.title == "From quick add").unwrap();
        assert_eq!(t.due_day.as_deref(), Some(today_str().as_str()));
        assert!(store.state.tag.iter().any(|g| g.title == "quick"));
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
        assert_eq!(win.imp().store.borrow().state.task.ids.len(), 2);
        win.add_from_text("https://example.org/page/");
        let store = win.imp().store.borrow();
        let link = store
            .state
            .task
            .iter()
            .find(|t| t.title == "example.org/page")
            .expect("link title");
        assert_eq!(link.notes.as_deref(), Some("https://example.org/page/"));
        drop(store);
        let long = "A rather long paragraph ".repeat(10);
        win.add_from_text(&long);
        let store = win.imp().store.borrow();
        let t = store.state.task.iter().last().unwrap();
        assert!(
            t.title.chars().count() <= 120 && t.notes.is_some(),
            "long text is title + notes"
        );
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
        assert!(win.imp().notified.borrow().contains(&id));
        win.snooze_task(&id, 60);
        let at = win.imp().store.borrow().state.task.entities[&id].remind_at.unwrap();
        assert!(at > now_ms() + 59 * 60_000);
        assert!(
            !win.imp().notified.borrow().contains(&id),
            "a snooze re-arms the reminder"
        );
        win.check_reminders();
        assert!(
            !win.imp().notified.borrow().contains(&id),
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
            .imp()
            .store
            .borrow()
            .state
            .project
            .iter()
            .find(|p| p.title == "Garden")
            .unwrap()
            .id
            .clone();
        assert!(win
            .imp()
            .views
            .borrow()
            .iter()
            .flatten()
            .any(|v| *v == View::Project(pid.clone())));
        win.go_to(View::Project(pid.clone()));
        win.add_task("Water plants");
        let tid = id_of(&win, "Water plants");
        assert_eq!(win.imp().store.borrow().state.task.entities[&tid].project_id, pid);
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
        assert!(!win.imp().store.borrow().state.task.entities.contains_key(&tid));
        assert_eq!(
            *win.imp().view.borrow(),
            View::Today,
            "falls back to Today when the view is gone"
        );
    });
}
