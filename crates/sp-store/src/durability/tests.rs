// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::{Pending, Store};
use sp_model::{Task, INBOX_PROJECT_ID};
use sp_oplog::Action;

fn add(store: &mut Store, title: &str) {
    let task = Task::new(title, INBOX_PROJECT_ID);
    let action = Action::AddTask { task, bottom: true };
    sp_oplog::apply(&mut store.state, &action);
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
    let journal = prepare(dir.path()).unwrap();
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
            .join(format!(".momentum-transaction.{prefix}-{}", sp_model::new_id()));
        fs::create_dir(&orphan).unwrap();
        fs::write(orphan.join("state.json"), "old private data").unwrap();
    }
    let unrelated = dir.path().join(".momentum-transaction.complete-my-backup");
    fs::create_dir(&unrelated).unwrap();
    let file = dir
        .path()
        .join(format!(".momentum-transaction.complete-{}", sp_model::new_id()));
    fs::write(&file, "keep").unwrap();
    #[cfg(unix)]
    let link = {
        let link = dir
            .path()
            .join(format!(".momentum-transaction.prepare-{}", sp_model::new_id()));
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();
        link
    };
    Store::try_load(dir.path().to_path_buf()).unwrap();
    assert!(unrelated.exists());
    assert_eq!(fs::read(file).unwrap(), b"keep");
    #[cfg(unix)]
    assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
    let expected = if cfg!(unix) { 3 } else { 2 };
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), expected);
}
