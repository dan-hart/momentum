// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Two nodes on localhost: link with a pairing code, exchange ops both ways, bootstrap a
//! third from the snapshot, and reject a wrong code.
use sp_model::Task;
use sp_oplog::Action;
use sp_p2p::{Event, FileKeyStore, LinkedDevice, P2p, SyncResult};
use std::time::{Duration, Instant};

fn node(name: &str) -> (P2p, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let keys = FileKeyStore::new(dir.path().join("keys")).unwrap();
    let p = P2p::start(dir.path().join("p2p"), &format!("dev-{name}"), name, Box::new(keys), 0).unwrap();
    assert_ne!(p.listen_addr().port(), 0, "listener is up before start returns");
    (p, dir)
}
fn wait_for(p: &P2p, pred: impl Fn(&Event) -> bool) -> Event {
    let events = p.events();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match events.recv_timeout(Duration::from_millis(200)) {
            Ok(e) if pred(&e) => return e,
            Ok(_) => {}
            Err(_) if Instant::now() > deadline => panic!("timed out waiting for event"),
            Err(_) => {}
        }
    }
}
fn task_op(client: &str, title: &str) -> (sp_oplog::Op, Action) {
    let mut vc = Default::default();
    let action = Action::AddTask {
        task: Task::new(title, "INBOX_PROJECT"),
        bottom: true,
    };
    (action.to_op(client, &mut vc), action)
}
fn addr_of(p: &P2p) -> std::net::SocketAddr {
    let port = p.listen_addr().port();
    ([127, 0, 0, 1], port).into()
}

#[test]
fn link_exchange_and_bootstrap() {
    let (a, _da) = node("alpha");
    let (b, _db) = node("beta");

    // Wrong code is refused.
    let _ = a.begin_pairing();
    b.link(addr_of(&a), "000000").unwrap();
    let e = wait_for(&b, |e| matches!(e, Event::LinkFinished { .. }));
    if let Event::LinkFinished { device, .. } = &e {
        assert!(device.is_none(), "wrong code must not link");
    }
    b.link_finished(None);
    assert!(a.devices().is_empty());

    // Right code links both sides, with fingerprints pinned.
    let code = a.begin_pairing();
    b.link(addr_of(&a), &code).unwrap();
    let e = wait_for(&b, |e| matches!(e, Event::LinkFinished { .. }));
    let Event::LinkFinished { device: Some(dev), .. } = e else {
        panic!("link failed: {e:?}")
    };
    b.link_finished(Some(&dev));
    a.end_pairing();
    assert_eq!(a.devices().len(), 1);
    assert_eq!(b.devices().len(), 1);
    assert!(!b.devices()[0].fingerprint.is_empty());

    // A op travels to B, a B op travels to A, over one exchange each way.
    let (op_a, act_a) = task_op("dev-alpha", "from alpha");
    assert!(a.publish(&op_a, &act_a));
    assert!(!a.publish(&op_a, &act_a), "duplicate publish is ignored");
    a.publish_snapshot(br#"{"task":{"ids":["x"],"entities":{}}}"#);
    let (op_b, act_b) = task_op("dev-beta", "from beta");
    b.publish(&op_b, &act_b);
    let to_a = LinkedDevice {
        address: Some(addr_of(&a)),
        ..b.devices()[0].clone()
    };
    b.sync_with(&to_a).unwrap();
    let e = wait_for(&b, |e| matches!(e, Event::SyncFinished { .. }));
    assert!(
        matches!(
            e,
            Event::SyncFinished {
                result: SyncResult::Success,
                ..
            }
        ),
        "{e:?}"
    );
    let (inbox_b, snap_b) = b.take_inbox();
    assert_eq!(inbox_b.len(), 1);
    assert_eq!(inbox_b[0].id, op_a.id);
    assert_eq!(inbox_b[0].decode().unwrap().0.c, "dev-alpha");
    assert_eq!(
        snap_b.unwrap().json().unwrap(),
        br#"{"task":{"ids":["x"],"entities":{}}}"#
    );
    // Consumers need the inbound notification to apply the durable journal to
    // their task store without waiting for a later outbound exchange.
    wait_for(&a, |e| matches!(e, Event::InboundSync { .. }));
    // The listener side (A) merged B's op during the same exchange.
    let (inbox_a, _) = a.take_inbox();
    assert_eq!(
        inbox_a.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        vec![op_b.id.as_str()]
    );
    // Nothing is delivered twice.
    b.sync_with(&to_a).unwrap();
    wait_for(&b, |e| matches!(e, Event::SyncFinished { .. }));
    assert!(b.take_inbox().0.is_empty());
    assert!(a.take_inbox().0.is_empty());
    assert!(
        !b.publish(&op_a, &act_a),
        "an op received from a peer is never republished"
    );

    // A third device links to B and bootstraps from the snapshot plus both ops.
    let (c, _dc) = node("gamma");
    let code = b.begin_pairing();
    c.link(addr_of(&b), &code).unwrap();
    let e = wait_for(&c, |e| matches!(e, Event::LinkFinished { .. }));
    let Event::LinkFinished { device: Some(dev), .. } = e else {
        panic!("link failed: {e:?}")
    };
    c.link_finished(Some(&dev));
    let to_b = LinkedDevice {
        address: Some(addr_of(&b)),
        ..c.devices()[0].clone()
    };
    c.sync_with(&to_b).unwrap();
    wait_for(&c, |e| matches!(e, Event::SyncFinished { .. }));
    let (inbox_c, snap_c) = c.take_inbox();
    let mut ids: Vec<&str> = inbox_c.iter().map(|r| r.id.as_str()).collect();
    ids.sort();
    let mut want = vec![op_a.id.as_str(), op_b.id.as_str()];
    want.sort();
    assert_eq!(ids, want);
    assert_eq!(snap_c.unwrap().origin, "dev-alpha");

    // Unlinking stops the next exchange.
    b.unlink(&c.identity().device_id);
    c.sync_with(&to_b).unwrap();
    let e = wait_for(&c, |e| matches!(e, Event::SyncFinished { .. }));
    assert!(
        matches!(
            e,
            Event::SyncFinished {
                result: SyncResult::Failed(_),
                ..
            }
        ),
        "{e:?}"
    );
}
