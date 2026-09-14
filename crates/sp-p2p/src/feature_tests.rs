// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! The parts of the transport that need no network: journal, record adapter, pairing
//! handler and snapshots. The two-node exchange lives in `tests/two_devices.rs`.
use super::*;
use libresync::{DeviceHandler, Identity, LogicalAdapter, RecordState, RecordView, State};
use sp_model::{Task, INBOX_PROJECT_ID};
use sp_oplog::Action;

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
