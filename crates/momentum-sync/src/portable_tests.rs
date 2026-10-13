// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::mock_dav::MockDav;
fn cfg(d: &MockDav) -> NextcloudCfg {
    NextcloudCfg {
        server_url: d.url.clone(),
        user_name: "u".into(),
        password: "p".into(),
        folder: "legacy".into(),
        compress: true,
        encrypt_key: Some("fixture-secret".into()),
    }
}
#[test]
fn settings_channel_is_separate_encrypted_verified_and_retryable() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    let local = r#"{"version":1,"cells":{"sort":{"value":"title","actor":"a","clock":{"a":1}}}}"#;
    let r = exchange_portable_guarded(&c, Duration::from_secs(10), || true, |_| Ok(Some(local.into()))).unwrap();
    assert!(r.uploaded);
    let raw = dav
        .raw("/remote.php/dav/files/u/legacy/momentum-settings.json")
        .unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("title"));
    assert!(dav.raw("/remote.php/dav/files/u/legacy/sync-data.json").is_none());
    let r = exchange_portable_guarded(
        &c,
        Duration::from_secs(10),
        || true,
        |remote| {
            assert_eq!(remote, Some(local));
            Ok(Some(local.into()))
        },
    )
    .unwrap();
    assert!(!r.uploaded);
    dav.keep_only_next_put.store(7, std::sync::atomic::Ordering::SeqCst);
    assert!(exchange_portable_guarded(&c, Duration::from_secs(10), || true, |_| Ok(Some("changed".into()))).is_err());
}
#[test]
fn settings_bootstrap_read_never_writes_without_choice() {
    let dav = MockDav::start();
    let c = cfg(&dav);
    exchange_portable_guarded(&c, Duration::from_secs(10), || true, |_| Ok(Some("remote".into()))).unwrap();
    let puts = dav.puts.load(std::sync::atomic::Ordering::SeqCst);
    let r = exchange_portable_guarded(
        &c,
        Duration::from_secs(10),
        || true,
        |remote| {
            assert_eq!(remote, Some("remote"));
            Ok(None)
        },
    )
    .unwrap();
    assert_eq!(r.remote.as_deref(), Some("remote"));
    assert!(!r.uploaded);
    assert_eq!(puts, dav.puts.load(std::sync::atomic::Ordering::SeqCst));
}
