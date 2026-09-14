// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! A minimal command-line device for testing the app end to end:
//!   peer DIR link ADDR CODE     link with the device at ADDR using its pairing code
//!   peer DIR push TITLE         queue an addTask op
//!   peer DIR sync               exchange with every linked device (last known address)
//!   peer DIR inbox              print and drain received ops and the newest snapshot
//!   peer DIR discover           list devices found by mDNS
//!   peer DIR serve SECS         listen with a pairing code printed on stdout
use sp_model::Task;
use sp_oplog::Action;
use sp_p2p::{Event, FileKeyStore, P2p};
use std::time::{Duration, Instant};

fn wait(p: &P2p, pred: impl Fn(&Event) -> bool) -> Option<Event> {
    let events = p.events();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(e) = events.recv_timeout(Duration::from_millis(200)) {
            if pred(&e) {
                return Some(e);
            }
        }
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (dir, cmd) = (
        std::path::PathBuf::from(args.get(1).expect("DIR")),
        args.get(2).map(String::as_str).unwrap_or("inbox"),
    );
    let keys = FileKeyStore::new(dir.join("keys")).expect("keys");
    let id = format!("peer-{}", dir.file_name().unwrap().to_string_lossy());
    let p = P2p::start(dir.join("p2p"), &id, "peer", Box::new(keys), 0).expect("start");
    match cmd {
        "link" => {
            let addr = args[3].parse().expect("ADDR");
            p.link(addr, &args[4]).expect("link");
            match wait(&p, |e| matches!(e, Event::LinkFinished { .. })) {
                Some(Event::LinkFinished { device: Some(d), .. }) => {
                    p.link_finished(Some(&d));
                    println!("linked with {} ({})", d.identity.user_id, d.identity.device_id);
                }
                other => {
                    p.link_finished(None);
                    println!("link failed: {other:?}");
                    std::process::exit(1);
                }
            }
        }
        "push" => {
            let action = Action::AddTask {
                task: Task::new(&args[3], "INBOX_PROJECT"),
                bottom: true,
            };
            let mut vc = Default::default();
            let op = action.to_op(&id, &mut vc);
            println!("queued {}", op.id);
            p.publish(&op, &action);
        }
        "sync" => {
            for d in p.devices() {
                p.sync_with(&d).expect("sync");
                match wait(&p, |e| matches!(e, Event::SyncFinished { .. })) {
                    Some(Event::SyncFinished { result, .. }) => println!("{}: {result:?}", d.name),
                    _ => println!("{}: timeout", d.name),
                }
            }
            p.save_state().ok();
            wait(&p, |e| matches!(e, Event::TaskFinished { .. }));
        }
        "discover" => {
            p.discover().expect("discover");
            match wait(&p, |e| matches!(e, Event::DiscoveryFinished { .. })) {
                Some(Event::DiscoveryFinished { devices }) => {
                    for d in devices {
                        println!(
                            "found {} ({}) at {:?} linked={}",
                            d.identity.user_id, d.identity.device_id, d.address, d.linked
                        );
                    }
                }
                _ => println!("discovery timed out"),
            }
        }
        "serve" => {
            let secs: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(60);
            println!("code {} at {}", p.begin_pairing(), p.listen_addr());
            let events = p.events();
            let deadline = Instant::now() + Duration::from_secs(secs);
            while Instant::now() < deadline {
                if let Ok(e) = events.recv_timeout(Duration::from_millis(500)) {
                    println!("{e:?}");
                    if matches!(e, Event::InboundSync { .. }) {
                        p.save_state().ok();
                    }
                }
            }
        }
        _ => {}
    }
    let (ops, snap) = p.take_inbox();
    for r in &ops {
        println!("op {} from {} at {}: {}", r.id, r.origin, r.t, r.action);
    }
    if let Some(s) = snap {
        println!(
            "snapshot from {} ({} bytes)",
            s.origin,
            s.json().map(|j| j.len()).unwrap_or(0)
        );
    }
    p.shutdown();
}
