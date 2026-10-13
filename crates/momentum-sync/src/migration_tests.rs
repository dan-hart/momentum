// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::mock_dav::MockDav;
use std::sync::atomic::Ordering;
fn cfg(d: &MockDav) -> NextcloudCfg {
    NextcloudCfg {
        server_url: d.url.clone(),
        user_name: "u".into(),
        password: "p".into(),
        folder: "legacy".into(),
        ..Default::default()
    }
}
const SOURCE: &str = "/remote.php/dav/files/u/legacy/sync-data.json";
const TARGET: &str = "/remote.php/dav/files/u/momentum/sync-data.json";
#[test]
fn checked_folder_copy_retains_source_and_is_idempotent() {
    let d = MockDav::start();
    d.set_raw(SOURCE, b"pf_2__original".to_vec());
    let report = migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(10), || true).unwrap();
    assert_eq!(report.copied_files, 1);
    assert_eq!(d.raw(SOURCE), d.raw(TARGET));
    migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(10), || true).unwrap();
    assert_eq!(d.puts.load(Ordering::SeqCst), 1);
}
#[test]
fn conflicting_target_missing_source_cancellation_and_short_upload_never_verify() {
    let d = MockDav::start();
    assert!(migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(10), || true).is_err());
    d.set_raw(SOURCE, b"source".to_vec());
    d.set_raw(TARGET, b"different".to_vec());
    assert!(migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(10), || true).is_err());
    assert_eq!(d.raw(TARGET).unwrap(), b"different");
    let d = MockDav::start();
    d.set_raw(SOURCE, b"source".to_vec());
    d.keep_only_next_put.store(2, Ordering::SeqCst);
    assert!(migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(10), || true).is_err());
    assert_eq!(d.raw(SOURCE).unwrap(), b"source");
    let d = MockDav::start();
    d.set_raw(SOURCE, b"source".to_vec());
    assert!(
        migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(10), || d
            .puts
            .load(Ordering::SeqCst)
            == 0)
        .is_err()
    );
}
#[test]
fn a_source_change_during_copy_requires_retry_and_never_switches() {
    let d = MockDav::start();
    d.set_raw(SOURCE, b"source".to_vec());
    let changed = std::sync::atomic::AtomicBool::new(false);
    assert!(
        migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(10), || {
            if d.puts.load(Ordering::SeqCst) > 0 && !changed.swap(true, Ordering::SeqCst) {
                d.set_raw(SOURCE, b"new source".to_vec());
            }
            true
        })
        .is_err()
    );
    assert_eq!(d.raw(SOURCE).unwrap(), b"new source");
}
#[test]
fn any_unrelated_target_file_is_a_conflict_even_when_named_like_the_folder() {
    let d = MockDav::start();
    d.set_raw(SOURCE, b"source".to_vec());
    d.set_raw("/remote.php/dav/files/u/momentum/momentum", b"unrelated".to_vec());
    assert!(migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(10), || true).is_err());
    assert_eq!(d.puts.load(Ordering::SeqCst), 0);
}
#[test]
fn folder_copy_supports_nextcloud_base_path_and_the_existing_large_task_limit() {
    let d = MockDav::start();
    let mut c = cfg(&d);
    c.server_url.push_str("/nextcloud");
    let source = "/nextcloud/remote.php/dav/files/u/legacy/sync-data.json";
    let target = "/nextcloud/remote.php/dav/files/u/momentum/sync-data.json";
    d.set_raw(source, b"pf_2__original".to_vec());
    migrate_folder_guarded(&c, "momentum", Duration::from_secs(10), || true).unwrap();
    assert_eq!(d.raw(source), d.raw(target));
    let d = MockDav::start();
    let body = [b"pf_2__large-task-".as_slice(), vec![b'x'; 17 << 20].as_slice()].concat();
    d.set_raw(SOURCE, body.clone());
    migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(30), || true).unwrap();
    assert_eq!(d.raw(SOURCE), Some(body.clone()));
    assert_eq!(d.raw(TARGET), Some(body));
}
#[test]
fn encoded_dav_inventory_paths_are_normalized_and_settings_keep_the_smaller_limit() {
    let d = MockDav::start();
    let mut c = cfg(&d);
    c.server_url.push_str("/nextcloud");
    d.encode_inventory_paths.store(true, Ordering::SeqCst);
    let source = "/nextcloud/remote.php/dav/files/u/legacy/sync-data.json";
    let target = "/nextcloud/remote.php/dav/files/u/momentum/sync-data.json";
    d.set_raw(source, b"pf_2__original".to_vec());
    migrate_folder_guarded(&c, "momentum", Duration::from_secs(10), || true).unwrap();
    assert_eq!(d.raw(source), d.raw(target));
    let d = MockDav::start();
    d.set_raw(SOURCE, b"pf_2__original".to_vec());
    d.set_raw(
        "/remote.php/dav/files/u/legacy/momentum-settings.json",
        vec![b'x'; 17 << 20],
    );
    assert!(migrate_folder_guarded(&cfg(&d), "momentum", Duration::from_secs(10), || true).is_err());
    assert_eq!(d.puts.load(Ordering::SeqCst), 0);
}
