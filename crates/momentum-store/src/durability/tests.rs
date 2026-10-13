// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::{Pending, Store};
use momentum_model::{Task, INBOX_PROJECT_ID};
use momentum_oplog::Action;

fn add(store: &mut Store, title: &str) {
    let task = Task::new(title, INBOX_PROJECT_ID);
    let action = Action::AddTask { task, bottom: true };
    momentum_oplog::apply(&mut store.state, &action);
    let op = action.to_op(&store.meta.client_id, &mut store.meta.vector_clock);
    store.pending.push(Pending { op, action });
}
fn image(store: &Store) -> [Vec<u8>; 3] {
    [
        serde_json::to_vec(&store.state).unwrap(),
        serde_json::to_vec(&store.pending).unwrap(),
        serde_json::to_vec(&store.meta).unwrap(),
    ]
}
fn on_disk(dir: &Path) -> [Vec<u8>; 3] {
    FILES.map(|name| fs::read(dir.join(name)).unwrap())
}

#[test]
fn crash_writer_child() {
    let Ok(dir) = std::env::var("MOMENTUM_DURABILITY_FIXTURE_DIR") else {
        return;
    };
    let checkpoint: usize = std::env::var("MOMENTUM_DURABILITY_FIXTURE_POINT")
        .unwrap()
        .parse()
        .unwrap();
    let mut store = Store::try_load(PathBuf::from(&dir)).unwrap();
    add(&mut store, "After");
    store.meta.last_nextcloud_ms = 99;
    let stages = [
        Step::Prepared,
        Step::Replaced(0),
        Step::Replaced(1),
        Step::Replaced(2),
        Step::Durable,
        Step::Committed,
    ];
    save_with_hook(Path::new(&dir), image(&store), |step| {
        if step == stages[checkpoint] {
            std::process::exit(77);
        }
    })
    .unwrap();
    panic!("requested interruption point was not reached");
}

#[test]
fn process_interruption_recovers_one_complete_generation_at_every_commit_boundary() {
    for checkpoint in 0..6 {
        let dir = tempfile::tempdir().unwrap();
        let mut before = Store::load(dir.path().to_path_buf());
        add(&mut before, "Before");
        before.save().unwrap();
        let before_image = image(&before);
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "durability::tests::crash_writer_child", "--test-threads=1"])
            .env("MOMENTUM_DURABILITY_FIXTURE_DIR", dir.path())
            .env("MOMENTUM_DURABILITY_FIXTURE_POINT", checkpoint.to_string())
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(77),
            "child did not exit at checkpoint {checkpoint}: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        let recovered = Store::try_load(dir.path().to_path_buf()).unwrap();
        if checkpoint < 5 {
            assert_eq!(
                on_disk(dir.path()),
                before_image,
                "mixed image after checkpoint {checkpoint}"
            );
            assert_eq!(image(&recovered), before_image);
        } else {
            assert_eq!(recovered.state.task.ids.len(), 2);
            assert_eq!(recovered.pending.len(), 2);
            assert_eq!(recovered.meta.last_nextcloud_ms, 99);
            assert_eq!(recovered.meta.vector_clock[&recovered.meta.client_id], 2);
            assert_eq!(on_disk(dir.path()), image(&recovered));
        }
        assert!(!dir.path().join(JOURNAL).exists());
        assert_eq!(
            image(&Store::try_load(dir.path().to_path_buf()).unwrap()),
            image(&recovered),
            "recovery is idempotent"
        );
    }
}

#[test]
fn interrupted_first_save_restores_file_absence() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    add(&mut store, "Not committed");
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        save_with_hook(dir.path(), image(&store), |step| {
            if step == Step::Replaced(0) {
                panic!("interruption");
            }
        })
        .unwrap();
    }));
    assert!(interrupted.is_err());
    Store::try_load(dir.path().to_path_buf()).unwrap();
    assert!(FILES.iter().all(|name| !dir.path().join(name).exists()));
}

#[test]
fn corrupt_recovery_record_fails_closed_without_overwriting_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    add(&mut store, "Keep");
    store.save().unwrap();
    let before = on_disk(dir.path());
    private_dir(&dir.path().join(JOURNAL)).unwrap();
    fs::write(dir.path().join(JOURNAL).join("manifest.json"), b"{}").unwrap();
    assert!(Store::try_load(dir.path().to_path_buf()).is_err());
    assert!(store.save().is_err());
    assert_eq!(on_disk(dir.path()), before);
    assert!(dir.path().join(JOURNAL).exists(), "preserve recovery evidence");
}

#[test]
fn incomplete_recovery_image_is_validated_before_live_files_change() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::load(dir.path().to_path_buf());
    add(&mut store, "Keep");
    store.save().unwrap();
    let journal = prepare(dir.path(), &[]).unwrap();
    fs::remove_file(journal.join("pending.json")).unwrap();
    let before = on_disk(dir.path());
    assert!(Store::try_load(dir.path().to_path_buf()).is_err());
    assert_eq!(on_disk(dir.path()), before);
}

#[test]
fn startup_removes_only_reserved_orphan_directories() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    for prefix in ["prepare", "complete"] {
        let orphan = dir
            .path()
            .join(format!(".momentum-transaction.{prefix}-{}", momentum_model::new_id()));
        fs::create_dir(&orphan).unwrap();
        fs::write(orphan.join("state.json"), "old private data").unwrap();
    }
    let unrelated = dir.path().join(".momentum-transaction.complete-my-backup");
    fs::create_dir(&unrelated).unwrap();
    let file = dir
        .path()
        .join(format!(".momentum-transaction.complete-{}", momentum_model::new_id()));
    fs::write(&file, "keep").unwrap();
    #[cfg(unix)]
    let link = {
        let link = dir
            .path()
            .join(format!(".momentum-transaction.prepare-{}", momentum_model::new_id()));
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();
        link
    };
    Store::try_load(dir.path().to_path_buf()).unwrap();
    assert!(unrelated.exists());
    assert_eq!(fs::read(file).unwrap(), b"keep");
    #[cfg(unix)]
    assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    // The kept entries plus the directory lock that loading creates.
    let expected = if cfg!(unix) { 4 } else { 3 };
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), expected);
}

#[test]
fn a_loader_waits_for_an_in_flight_save_instead_of_rolling_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::try_load(dir.path().to_path_buf()).unwrap();
    add(&mut store, "Before");
    store.save().unwrap();
    // Hold the directory lock as a saving process would, mid-save: a journal is
    // published and the live files are half replaced.
    let held = lock_dir(dir.path()).unwrap();
    add(&mut store, "After");
    let journal = prepare(dir.path(), &[]).unwrap();
    let images = image(&store);
    write_synced(&dir.path().join("state.json.tmp"), &images[0]).unwrap();
    fs::rename(dir.path().join("state.json.tmp"), dir.path().join("state.json")).unwrap();
    let path = dir.path().to_path_buf();
    let loader = std::thread::spawn(move || Store::try_load(path).unwrap());
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(!loader.is_finished(), "load must block while another process saves");
    // The writer finishes its save and releases the lock; only then does the loader
    // run, and it sees the committed image rather than rolling the journal back.
    write_synced(&dir.path().join("pending.json.tmp"), &images[1]).unwrap();
    fs::rename(dir.path().join("pending.json.tmp"), dir.path().join("pending.json")).unwrap();
    write_synced(&dir.path().join("meta.json.tmp"), &images[2]).unwrap();
    fs::rename(dir.path().join("meta.json.tmp"), dir.path().join("meta.json")).unwrap();
    let retired = retire(dir.path(), &journal).unwrap();
    let _ = fs::remove_dir_all(retired);
    drop(held);
    let loaded = loader.join().unwrap();
    assert_eq!(loaded.pending.len(), 2);
    assert_eq!(on_disk(dir.path()), images);
}

#[test]
fn portable_crash_writer_child() {
    let Ok(dir) = std::env::var("MOMENTUM_PORTABLE_FIXTURE_DIR") else {
        return;
    };
    let checkpoint: usize = std::env::var("MOMENTUM_PORTABLE_FIXTURE_POINT")
        .unwrap()
        .parse()
        .unwrap();
    let mut store = Store::try_load(PathBuf::from(&dir)).unwrap();
    add(&mut store, "After");
    let steps = [
        Step::Prepared,
        Step::Replaced(0),
        Step::Replaced(1),
        Step::Replaced(2),
        Step::Replaced(3),
        Step::Durable,
        Step::Committed,
    ];
    save_portable_with_hook(Path::new(&dir), image(&store), b"after", |step| {
        if step == steps[checkpoint] {
            std::process::exit(77)
        }
    })
    .unwrap();
    panic!("missed checkpoint");
}
#[test]
fn portable_and_tasks_recover_together_after_every_process_interruption() {
    for checkpoint in 0..7 {
        let dir = tempfile::tempdir().unwrap();
        let mut before = Store::load(dir.path().to_path_buf());
        add(&mut before, "Before");
        before.save_with_portable(b"before").unwrap();
        let initial = image(&before);
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "durability::tests::portable_crash_writer_child",
                "--test-threads=1",
            ])
            .env("MOMENTUM_PORTABLE_FIXTURE_DIR", dir.path())
            .env("MOMENTUM_PORTABLE_FIXTURE_POINT", checkpoint.to_string())
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(77),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        let after = Store::try_load(dir.path().to_path_buf()).unwrap();
        if checkpoint < 6 {
            assert_eq!(image(&after), initial);
            assert_eq!(fs::read(dir.path().join(PORTABLE)).unwrap(), b"before");
        } else {
            assert_eq!(after.state.task.ids.len(), 2);
            assert_eq!(fs::read(dir.path().join(PORTABLE)).unwrap(), b"after");
        }
        assert!(!dir.path().join(JOURNAL).exists());
    }
}
#[test]
fn settings_only_crash_writer_child() {
    let Ok(dir) = std::env::var("MOMENTUM_SETTINGS_ONLY_FIXTURE_DIR") else {
        return;
    };
    let checkpoint: usize = std::env::var("MOMENTUM_SETTINGS_ONLY_FIXTURE_POINT")
        .unwrap()
        .parse()
        .unwrap();
    let stages = [Step::Prepared, Step::Replaced(0), Step::Durable, Step::Committed];
    save_portable_only_with_hook(Path::new(&dir), b"after", None, |step| {
        if step == stages[checkpoint] {
            std::process::exit(77);
        }
    })
    .unwrap();
    panic!("missed settings-only checkpoint");
}
#[cfg(unix)]
#[test]
fn settings_only_process_interruption_preserves_task_inodes_and_recovers_settings() {
    use std::os::unix::fs::MetadataExt;
    for existed in [false, true] {
        for checkpoint in 0..4 {
            let dir = tempfile::tempdir().unwrap();
            let mut store = Store::load(dir.path().to_path_buf());
            add(&mut store, "Keep");
            store.save().unwrap();
            if existed {
                store.save_portable(b"before").unwrap();
            }
            let initial = on_disk(dir.path());
            let inodes = FILES.map(|name| fs::metadata(dir.path().join(name)).unwrap().ino());
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "durability::tests::settings_only_crash_writer_child",
                    "--test-threads=1",
                ])
                .env("MOMENTUM_SETTINGS_ONLY_FIXTURE_DIR", dir.path())
                .env("MOMENTUM_SETTINGS_ONLY_FIXTURE_POINT", checkpoint.to_string())
                .output()
                .unwrap();
            assert_eq!(
                child.status.code(),
                Some(77),
                "{}",
                String::from_utf8_lossy(&child.stderr)
            );
            Store::try_load(dir.path().to_path_buf()).unwrap();
            assert_eq!(on_disk(dir.path()), initial);
            assert_eq!(
                FILES.map(|name| fs::metadata(dir.path().join(name)).unwrap().ino()),
                inodes
            );
            if checkpoint == 3 {
                assert_eq!(fs::read(dir.path().join(PORTABLE)).unwrap(), b"after");
            } else if existed {
                assert_eq!(fs::read(dir.path().join(PORTABLE)).unwrap(), b"before");
            } else {
                assert!(!dir.path().join(PORTABLE).exists());
            }
            assert!(!dir.path().join(JOURNAL).exists());
            Store::try_load(dir.path().to_path_buf()).unwrap();
            assert_eq!(on_disk(dir.path()), initial);
        }
    }
}
#[test]
fn settings_only_commit_keeps_the_existing_directory_exclusion() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::load(dir.path().to_path_buf());
    store.save().unwrap();
    store.save_portable(b"before").unwrap();
    let initial = on_disk(dir.path());
    let held = lock_dir(dir.path()).unwrap();
    let journal = prepare_selected(dir.path(), &[PORTABLE], true).unwrap();
    write_synced(&dir.path().join(format!("{PORTABLE}.tmp")), b"after").unwrap();
    fs::rename(dir.path().join(format!("{PORTABLE}.tmp")), dir.path().join(PORTABLE)).unwrap();
    let path = dir.path().to_path_buf();
    let loader = std::thread::spawn(move || {
        Store::try_load(path.clone()).unwrap();
        fs::read(path.join(PORTABLE)).unwrap()
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(
        !loader.is_finished(),
        "a loader passed an unfinished settings transaction"
    );
    let retired = retire(dir.path(), &journal).unwrap();
    fs::remove_dir_all(retired).unwrap();
    drop(held);
    assert_eq!(loader.join().unwrap(), b"after");
    assert_eq!(on_disk(dir.path()), initial);
}
#[test]
fn settings_identity_crash_writer_child() {
    let Ok(dir) = std::env::var("MOMENTUM_SETTINGS_IDENTITY_FIXTURE_DIR") else {
        return;
    };
    let checkpoint: usize = std::env::var("MOMENTUM_SETTINGS_IDENTITY_FIXTURE_POINT")
        .unwrap()
        .parse()
        .unwrap();
    let store = Store::try_load(PathBuf::from(&dir)).unwrap();
    let stages = [
        Step::Prepared,
        Step::Replaced(0),
        Step::Replaced(1),
        Step::Durable,
        Step::Committed,
    ];
    save_portable_only_with_hook(Path::new(&dir), b"after", Some(&store.meta), |step| {
        if step == stages[checkpoint] {
            std::process::exit(77);
        }
    })
    .unwrap();
    panic!("missed identity checkpoint");
}
#[test]
fn fresh_settings_identity_and_image_recover_as_one_generation() {
    for checkpoint in 0..5 {
        let dir = tempfile::tempdir().unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "durability::tests::settings_identity_crash_writer_child",
                "--test-threads=1",
            ])
            .env("MOMENTUM_SETTINGS_IDENTITY_FIXTURE_DIR", dir.path())
            .env("MOMENTUM_SETTINGS_IDENTITY_FIXTURE_POINT", checkpoint.to_string())
            .output()
            .unwrap();
        assert_eq!(
            child.status.code(),
            Some(77),
            "{}",
            String::from_utf8_lossy(&child.stderr)
        );
        let store = Store::try_load(dir.path().to_path_buf()).unwrap();
        assert!(!dir.path().join("state.json").exists() && !dir.path().join("pending.json").exists());
        if checkpoint == 4 {
            let meta: crate::Meta = serde_json::from_slice(&fs::read(dir.path().join("meta.json")).unwrap()).unwrap();
            assert_eq!(meta.client_id, store.meta.client_id);
            assert_eq!(fs::read(dir.path().join(PORTABLE)).unwrap(), b"after");
            assert_eq!(
                Store::try_load(dir.path().to_path_buf()).unwrap().meta.client_id,
                meta.client_id
            );
        } else {
            assert!(!dir.path().join("meta.json").exists());
            assert!(!dir.path().join(PORTABLE).exists());
        }
        assert!(!dir.path().join(JOURNAL).exists());
    }
}
#[test]
fn a_second_fresh_owner_cannot_publish_settings_under_a_different_identity() {
    let dir = tempfile::tempdir().unwrap();
    let first = Store::try_load(dir.path().to_path_buf()).unwrap();
    let second = Store::try_load(dir.path().to_path_buf()).unwrap();
    assert_ne!(first.meta.client_id, second.meta.client_id);
    first.save_portable(b"first").unwrap();
    let metadata = fs::read(dir.path().join("meta.json")).unwrap();
    assert!(second.save_portable(b"second").is_err());
    assert_eq!(fs::read(dir.path().join(PORTABLE)).unwrap(), b"first");
    assert_eq!(fs::read(dir.path().join("meta.json")).unwrap(), metadata);
    assert!(!dir.path().join("state.json").exists() && !dir.path().join("pending.json").exists());
}
