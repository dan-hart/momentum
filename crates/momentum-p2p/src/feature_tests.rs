// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The parts of the transport that need no network: journal, record adapter, pairing
//! handler and snapshots. The two-node exchange lives in `tests/two_devices.rs`.
use super::*;
use libresync::{DeviceHandler, Identity, LogicalAdapter, RecordState, RecordView, State};
use momentum_model::{Task, INBOX_PROJECT_ID};
use momentum_oplog::Action;

fn op(client: &str, title: &str) -> (Op, Action) {
    let action = Action::AddTask {
        task: Task::new(title, INBOX_PROJECT_ID),
        bottom: true,
    };
    let mut vc = Default::default();
    (action.to_op(client, &mut vc), action)
}
fn adapter(dir: &std::path::Path) -> (OpAdapter, Arc<Mutex<Journal>>) {
    let journal = Arc::new(Mutex::new(Journal::default()));
    (
        OpAdapter {
            journal: journal.clone(),
            path: dir.join("journal.json"),
        },
        journal,
    )
}

#[test]
fn snapshot_round_trips_and_is_compressed() {
    let big = vec![b'x'; 50_000];
    let s = Snapshot::new("dev", &big);
    assert!(s.gz_b64.len() < 2_000, "gzip pays off on repetitive JSON");
    assert_eq!(s.json().unwrap(), big);
    assert_eq!(s.origin, "dev");
    let bad = Snapshot {
        gz_b64: "!!!".into(),
        ..s
    };
    assert!(bad.json().is_none());
}
#[test]
fn op_record_decodes_back_to_the_typed_action() {
    let (o, a) = op("c", "t");
    let r = OpRecord {
        id: o.id.clone(),
        t: o.t,
        origin: "c".into(),
        op: serde_json::to_value(&o).unwrap(),
        action: serde_json::to_value(&a).unwrap(),
    };
    let (op2, action2) = r.decode().unwrap();
    assert_eq!(op2.id, o.id);
    assert_eq!(action2.parts().0, a.parts().0);
    assert!(OpRecord {
        action: json_null(),
        ..r
    }
    .decode()
    .is_none());
}
fn json_null() -> Value {
    Value::Null
}

#[test]
fn failed_inbound_save_can_retry_without_losing_the_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let (ad, journal) = adapter(dir.path());
    let (o, a) = op("peer", "retry me");
    let mut state = State::new("peer");
    RecordState::new(&mut state, APP_ID)
        .set(op_record(
            &OpRecord {
                id: o.id.clone(),
                t: o.t,
                origin: o.c.clone(),
                op: serde_json::to_value(&o).unwrap(),
                action: serde_json::to_value(&a).unwrap(),
            },
            false,
        ))
        .unwrap();
    std::fs::create_dir(ad.path.with_extension("tmp")).unwrap();
    assert!(ad.apply_records(&RecordView::new(&state, APP_ID)).is_err());
    assert!(journal.lock().unwrap().inbox.is_empty());
    assert!(!journal.lock().unwrap().seen.contains(&o.id));
    std::fs::remove_dir(ad.path.with_extension("tmp")).unwrap();
    ad.apply_records(&RecordView::new(&state, APP_ID)).unwrap();
    assert_eq!(Journal::load(&ad.path).unwrap().inbox.len(), 1);
    assert!(Journal::load(&ad.path).unwrap().seen.contains(&o.id));
}

#[test]
fn failed_snapshot_publication_stays_dirty_for_retry() {
    let dir = tempfile::tempdir().unwrap();
    let (ad, journal) = adapter(dir.path());
    journal.lock().unwrap().snapshot_out = Some(Snapshot::new("me", b"{}"));
    journal.lock().unwrap().snapshot_dirty = true;
    std::fs::create_dir(ad.path.with_extension("tmp")).unwrap();
    let mut state = State::new("me");
    assert!(ad.load_records(&mut RecordState::new(&mut state, APP_ID)).is_err());
    assert!(journal.lock().unwrap().snapshot_dirty);
    std::fs::remove_dir(ad.path.with_extension("tmp")).unwrap();
    ad.load_records(&mut RecordState::new(&mut state, APP_ID)).unwrap();
    assert!(!Journal::load(&ad.path).unwrap().snapshot_dirty);
}

#[test]
fn a_batch_is_persisted_once_and_a_failed_save_keeps_every_op_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.json");
    let mut journal = Journal::default();
    let ops = [op("me", "one"), op("me", "two")];
    std::fs::create_dir(path.with_extension("tmp")).unwrap();
    assert!(journal.publish(&path, ops.iter().map(|(o, a)| (o, a))).is_err());
    assert!(journal.own.is_empty() && journal.seen.is_empty());
    std::fs::remove_dir(path.with_extension("tmp")).unwrap();
    assert_eq!(journal.publish(&path, ops.iter().map(|(o, a)| (o, a))).unwrap(), 2);
    assert_eq!(Journal::load(&path).unwrap().own, journal.own);
    // No new work needs a write: even an unavailable scratch path is harmless.
    std::fs::create_dir(path.with_extension("tmp")).unwrap();
    assert_eq!(journal.publish(&path, ops.iter().map(|(o, a)| (o, a))).unwrap(), 0);
}

#[test]
fn unchanged_load_never_rewrites_the_journal() {
    let dir = tempfile::tempdir().unwrap();
    let (ad, journal) = adapter(dir.path());
    let (o, a) = op("me", "stable");
    journal.lock().unwrap().publish(&ad.path, [(&o, &a)]).unwrap();
    let mut state = State::new("me");
    ad.load_records(&mut RecordState::new(&mut state, APP_ID)).unwrap();
    std::fs::create_dir(ad.path.with_extension("tmp")).unwrap();
    ad.load_records(&mut RecordState::new(&mut state, APP_ID)).unwrap();
    assert_eq!(RecordView::new(&state, APP_ID).snapshot().unwrap().len(), 1);
}

#[test]
#[ignore]
fn benchmark_offline_publish() {
    let ops: Vec<_> = (0..1_000).map(|i| op("me", &format!("offline task {i}"))).collect();
    let dir = tempfile::tempdir().unwrap();
    let mut one_at_a_time = Journal::default();
    let path = dir.path().join("individual.json");
    let started = Instant::now();
    for (o, a) in &ops {
        // Reference the original one-save-per-op path, including its cheaper,
        // unsynced atomic write. Do not inflate the baseline with staging clones.
        one_at_a_time.seen.insert(o.id.clone());
        one_at_a_time.own.push(OpRecord {
            id: o.id.clone(),
            t: o.t,
            origin: o.c.clone(),
            op: serde_json::to_value(o).unwrap(),
            action: serde_json::to_value(a).unwrap(),
        });
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec(&one_at_a_time).unwrap()).unwrap();
        std::fs::rename(tmp, &path).unwrap();
    }
    let individual = started.elapsed();
    let mut batch = Journal::default();
    let started = Instant::now();
    batch
        .publish(&dir.path().join("batch.json"), ops.iter().map(|(o, a)| (o, a)))
        .unwrap();
    let elapsed = started.elapsed();
    assert_eq!(one_at_a_time.own, batch.own);
    assert_eq!(Journal::load(&dir.path().join("batch.json")).unwrap().own, batch.own);
    eprintln!(
        "1,000 queued ops: individual={individual:?}, batch={elapsed:?}, speedup={:.1}x",
        individual.as_secs_f64() / elapsed.as_secs_f64()
    );
}
#[test]
fn adapter_publishes_own_ops_once_and_collects_foreign_ones() {
    let dir = tempfile::tempdir().unwrap();
    let (ad, journal) = adapter(dir.path());
    let (o, a) = op("me", "mine");
    journal.lock().unwrap().own.push(OpRecord {
        id: o.id.clone(),
        t: o.t,
        origin: "me".into(),
        op: serde_json::to_value(&o).unwrap(),
        action: serde_json::to_value(&a).unwrap(),
    });
    journal.lock().unwrap().seen.insert(o.id.clone());
    let mut state = State::new("me");
    let mut records = RecordState::new(&mut state, APP_ID);
    ad.load_records(&mut records).unwrap();
    assert_eq!(records.snapshot().unwrap().len(), 1);
    let clock_before = records.get(SCHEMA, OP_ENTITY, &o.id).unwrap().unwrap().clock;
    ad.load_records(&mut records).unwrap();
    assert_eq!(
        records.get(SCHEMA, OP_ENTITY, &o.id).unwrap().unwrap().clock,
        clock_before,
        "a second load does not rewrite (and re-clock) an op already in the state"
    );
    // A peer's op appears in the shared state: it lands in the inbox exactly once.
    let (po, pa) = op("peer", "theirs");
    records
        .set(op_record(
            &OpRecord {
                id: po.id.clone(),
                t: po.t,
                origin: "peer".into(),
                op: serde_json::to_value(&po).unwrap(),
                action: serde_json::to_value(&pa).unwrap(),
            },
            false,
        ))
        .unwrap();
    let view = RecordView::new(&state, APP_ID);
    ad.apply_records(&view).unwrap();
    ad.apply_records(&view).unwrap();
    let j = journal.lock().unwrap();
    assert_eq!(
        j.inbox.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        vec![po.id.as_str()]
    );
    assert!(
        !j.inbox.iter().any(|r| r.id == o.id),
        "own ops never come back through the inbox"
    );
    assert!(
        dir.path().join("journal.json").exists(),
        "the journal is persisted on every change"
    );
}
#[test]
fn old_own_ops_are_tombstoned_and_ignored_by_readers() {
    let dir = tempfile::tempdir().unwrap();
    let (ad, journal) = adapter(dir.path());
    let (o, a) = op("me", "old");
    journal.lock().unwrap().own.push(OpRecord {
        id: o.id.clone(),
        t: now_ms() - (KEEP_DAYS + 1) * 86_400_000,
        origin: "me".into(),
        op: serde_json::to_value(&o).unwrap(),
        action: serde_json::to_value(&a).unwrap(),
    });
    let mut state = State::new("me");
    ad.load_records(&mut RecordState::new(&mut state, APP_ID)).unwrap();
    let rec = RecordView::new(&state, APP_ID).snapshot().unwrap().remove(0);
    assert!(rec.tombstone && rec.fields.is_empty());
    let j = journal.lock().unwrap();
    assert!(j.own.is_empty() && j.tombstoned.contains(&o.id));
    drop(j);
    // A fresh reader sees the tombstone and takes nothing.
    let (reader, rj) = adapter(&tempfile::tempdir().unwrap().path().to_path_buf());
    reader.apply_records(&RecordView::new(&state, APP_ID)).unwrap();
    assert!(rj.lock().unwrap().inbox.is_empty());
}
#[test]
fn newest_snapshot_wins_and_own_snapshot_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let (ad, journal) = adapter(dir.path());
    let mut state = State::new("me");
    fn put(state: &mut State, t: u64, origin: &str) {
        let mut records = RecordState::new(state, APP_ID);
        let fields = std::collections::BTreeMap::from([
            ("t".to_string(), FieldValue::I64(t as i64)),
            ("origin".to_string(), FieldValue::String(origin.into())),
            (
                "gz_b64".to_string(),
                FieldValue::String(Snapshot::new(origin, b"{}").gz_b64),
            ),
        ]);
        records
            .set(SyncRecord {
                schema: SCHEMA.into(),
                entity: SNAPSHOT_ENTITY.into(),
                id: SNAPSHOT_ID.into(),
                fields,
                ..SyncRecord::default()
            })
            .unwrap();
    }
    put(&mut state, 10, "a");
    ad.apply_records(&RecordView::new(&state, APP_ID)).unwrap();
    assert_eq!(journal.lock().unwrap().snapshot_in.as_ref().map(|s| s.t), Some(10));
    put(&mut state, 5, "b");
    ad.apply_records(&RecordView::new(&state, APP_ID)).unwrap();
    assert_eq!(
        journal.lock().unwrap().snapshot_in.as_ref().map(|s| s.origin.clone()),
        Some("a".into()),
        "older snapshot ignored"
    );
    journal.lock().unwrap().snapshot_out = Some(Snapshot {
        t: 20,
        origin: "me".into(),
        gz_b64: String::new(),
    });
    put(&mut state, 20, "me");
    ad.apply_records(&RecordView::new(&state, APP_ID)).unwrap();
    assert_eq!(
        journal.lock().unwrap().snapshot_in.as_ref().map(|s| s.t),
        Some(10),
        "our own snapshot never bootstraps us"
    );
}
#[test]
fn pairing_window_gates_approval_and_links_persist() {
    let dir = tempfile::tempdir().unwrap();
    let identity = Identity::new("me", APP_ID, "host");
    let keys = FileKeyStore::new(dir.path().join("keys")).unwrap();
    let h = Handler::new(identity.clone(), Box::new(keys), dir.path());
    let peer = Identity::new("peer", APP_ID, "laptop");
    assert!(
        !h.approve_link_with_fingerprint(&peer, "fp").unwrap(),
        "closed window refuses"
    );
    assert!(h.pairing_secret().is_none());
    *h.pairing.lock().unwrap() = Some(("123456".into(), Instant::now() + PAIRING_WINDOW));
    assert_eq!(h.pairing_secret().as_deref(), Some("123456"));
    assert!(h.approve_link_with_fingerprint(&peer, "fp").unwrap());
    assert!(h.is_linked(&peer) && h.is_linked_with_fingerprint(&peer, "fp"));
    assert!(
        !h.is_linked_with_fingerprint(&peer, "other"),
        "a changed certificate is not trusted"
    );
    *h.pairing.lock().unwrap() = Some(("123456".into(), Instant::now() - std::time::Duration::from_secs(1)));
    assert!(
        !h.approve_link_with_fingerprint(&Identity::new("late", APP_ID, "x"), "fp")
            .unwrap(),
        "expired window"
    );
    // An outgoing link's typed code takes precedence over our own code.
    *h.entered.lock().unwrap() = Some("999999".into());
    assert_eq!(h.pairing_secret().as_deref(), Some("999999"));
    // Links survive a restart through devices.json.
    let again = Handler::new(
        identity,
        Box::new(FileKeyStore::new(dir.path().join("keys")).unwrap()),
        dir.path(),
    );
    assert_eq!(again.linked().len(), 1);
    assert_eq!(again.linked()[0].name, "laptop");
    // Keys are generated once and cached.
    assert_eq!(again.app_key().unwrap(), again.app_key().unwrap());
    assert_eq!(
        again.device_keys().unwrap().fingerprint(),
        again.device_keys().unwrap().fingerprint()
    );
}

#[test]
fn inbox_read_and_acknowledgement_preserve_new_arrivals_and_failed_ack() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.json");
    let record = |name: &str| {
        let (o, a) = op("peer", name);
        OpRecord {
            id: o.id.clone(),
            t: o.t,
            origin: "peer".into(),
            op: serde_json::to_value(o).unwrap(),
            action: serde_json::to_value(a).unwrap(),
        }
    };
    let first = record("first");
    let second = record("second");
    let snap = Snapshot::new("peer", b"first");
    let newer = Snapshot::new("peer", b"newer");
    let mut journal = Journal {
        inbox: vec![first.clone()],
        snapshot_in: Some(snap.clone()),
        ..Default::default()
    };
    journal.save(&path).unwrap();
    let read = journal.inbox();
    assert_eq!(journal.inbox(), read, "reading must not consume the retry batch");
    assert_eq!(Journal::load(&path).unwrap().inbox(), read);
    journal.inbox.push(second.clone());
    journal.snapshot_in = Some(newer.clone());
    journal.save(&path).unwrap();
    std::fs::create_dir(path.with_extension("tmp")).unwrap();
    assert!(journal.acknowledge(&path, &read.0, read.1.as_ref()).is_err());
    assert_eq!(journal.inbox, [first, second.clone()]);
    assert_eq!(Journal::load(&path).unwrap().inbox, journal.inbox);
    std::fs::remove_dir(path.with_extension("tmp")).unwrap();
    journal.acknowledge(&path, &read.0, read.1.as_ref()).unwrap();
    assert_eq!(journal.inbox, [second]);
    assert_eq!(journal.snapshot_in, Some(newer));
    assert_eq!(Journal::load(&path).unwrap().inbox(), journal.inbox());
}

#[test]
fn shutdown_cancels_a_stalled_sync_all_socket() {
    let dir = tempfile::tempdir().unwrap();
    let node = P2p::start(
        dir.path().join("node"),
        "cancel-fixture",
        "Fixture",
        Box::new(FileKeyStore::new(dir.path().join("keys")).unwrap()),
        0,
    )
    .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (arrived_tx, arrived) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let (_stream, _) = listener.accept().unwrap();
        arrived_tx.send(()).unwrap();
        let _ = released.recv_timeout(Duration::from_secs(10));
    });
    node.handler.remember(
        &Identity::new("stalled-peer", APP_ID, "Fixture"),
        &"a".repeat(64),
        Some(address),
    );
    node.sync_all().unwrap();
    arrived.recv_timeout(Duration::from_secs(8)).unwrap();
    let start = Instant::now();
    node.shutdown();
    let elapsed = start.elapsed();
    let _ = release.send(());
    server.join().unwrap();
    assert!(
        elapsed < Duration::from_secs(2),
        "stopped only after the I/O timeout: {elapsed:?}"
    );
}
