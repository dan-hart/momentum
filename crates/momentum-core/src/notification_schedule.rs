// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Device-local OS scheduling acknowledgements, separate from desktop delivery claims.
use crate::notifications::*;
use crate::{CoreError, Engine, Preferences};
use serde::{Deserialize, Serialize};
use sp_model::{day_of_ms, time_of_ms};
use sp_store::Store;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;

const FILE: &str = "notification-schedule.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Accepted {
    identity: NotificationObservation,
    fire_at_ms: u64,
    /// At-most-once suppression, not proof that the OS displayed anything.
    consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ledger {
    version: u32,
    accepted: BTreeMap<String, Accepted>,
}
impl Default for Ledger {
    fn default() -> Self {
        Self {
            version: 1,
            accepted: BTreeMap::new(),
        }
    }
}
impl Ledger {
    fn load(dir: &Path) -> Result<Self, CoreError> {
        let data = match std::fs::read(dir.join(FILE)) {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        let ledger: Self = serde_json::from_slice(&data).map_err(|_| invalid_ledger())?;
        if ledger.version != 1 || ledger.accepted.iter().any(|(id, a)| id != &a.identity.id) {
            return Err(invalid_ledger());
        }
        Ok(ledger)
    }
    fn save(&self, dir: &Path) -> Result<(), CoreError> {
        let bytes = serde_json::to_vec(self).map_err(|_| invalid_ledger())?;
        let temporary = dir.join(format!("{FILE}.tmp"));
        // Never truncate an existing temporary file or follow a symlink. A leftover
        // file is an actionable failure rather than silent loss of a prior claim.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let result = (|| -> std::io::Result<()> {
            file.write_all(&bytes)?;
            file.sync_all()?;
            std::fs::rename(&temporary, dir.join(FILE))?;
            std::fs::File::open(dir)?.sync_all()
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result.map_err(Into::into)
    }
    fn consume_elapsed(&mut self, now_ms: u64) {
        for a in self.accepted.values_mut() {
            a.consumed |= a.fire_at_ms <= now_ms;
        }
    }
}
fn invalid_ledger() -> CoreError {
    CoreError::Invalid {
        message: "Notification schedule is invalid or unsupported; existing claims were preserved".into(),
    }
}

fn desired(store: &Store, prefs: &Preferences, now_ms: u64, ledger: &Ledger) -> NotificationPlan {
    let mut result = plan(
        store,
        &NotificationPlanningOptions {
            now_ms,
            source_revision: String::new(),
            consumed_ids: ledger
                .accepted
                .iter()
                .filter(|(_, a)| a.consumed)
                .map(|(id, _)| id.clone())
                .collect(),
            morning_summary_enabled: prefs.morning_summary_enabled,
            morning_summary_time: prefs.morning_summary_time,
        },
    );
    for request in &mut result.requests {
        // Canonical structured comparison survives restarts and has no hash collisions.
        // It is private metadata (may contain task titles): never log or display it.
        // Exclude moving catch-up fire_at_ms; await latency is not a source edit.
        let summary_clock = matches!(request.content, NotificationContent::Summary { .. })
            .then_some((prefs.morning_summary_time.hour, prefs.morning_summary_time.minute));
        request.source_revision = serde_json::json!({
            "version": 1,
            "id": request.id,
            "scheduled_at_ms": request.scheduled_at_ms,
            "local_day": day_of_ms(request.scheduled_at_ms),
            "local_time": time_of_ms(request.scheduled_at_ms),
            "summary_clock": summary_clock,
            "content": request.content,
        })
        .to_string();
    }
    result
}
fn decorate(mut plan: NotificationPlan, ledger: &Ledger) -> NotificationPlan {
    for accepted in ledger.accepted.values().filter(|a| !a.consumed) {
        if plan.requests.iter().any(|r| r.observation() == accepted.identity) {
            plan.accepted.push(accepted.identity.clone());
        } else {
            plan.cancellations.push(accepted.identity.clone());
        }
    }
    plan
}

#[cfg_attr(feature = "ffi", uniffi::export)]
impl Engine {
    /// Read-only desired snapshot. Call reconcile at startup/foreground and after edits
    /// before scheduling; it durably consumes elapsed identities against clock rollback.
    pub fn notification_plan(&self, now_ms: u64) -> Result<NotificationPlan, CoreError> {
        let g = self.lock();
        let mut ledger = Ledger::load(g.store.dir())?;
        ledger.consume_elapsed(now_ms);
        Ok(decorate(desired(&g.store, &g.prefs, now_ms, &ledger), &ledger))
    }

    /// Reconcile complete OS snapshots for this app's requests. Observations must retain
    /// the original revision metadata. Stale delivered revisions never consume a new
    /// source. Missing future requests can be rescheduled; elapsed accepted identities
    /// remain consumed even if the user dismissed the notification.
    pub fn reconcile_notifications(
        &self,
        now_ms: u64,
        pending: Vec<NotificationObservation>,
        delivered: Vec<NotificationObservation>,
    ) -> Result<NotificationPlan, CoreError> {
        let g = self.lock();
        let before = Ledger::load(g.store.dir())?;
        let mut ledger = before.clone();
        ledger.consume_elapsed(now_ms);
        let pending: BTreeSet<_> = pending.into_iter().collect();
        let delivered: BTreeSet<_> = delivered.into_iter().collect();
        let desired = desired(&g.store, &g.prefs, now_ms, &ledger);
        let mut cancellations = BTreeSet::new();
        ledger.accepted.retain(|_, a| {
            if a.consumed {
                return true;
            }
            let current = desired.requests.iter().any(|r| r.observation() == a.identity);
            if !current {
                cancellations.insert(a.identity.clone());
            }
            current && (pending.contains(&a.identity) || delivered.contains(&a.identity))
        });
        // Recover the crash window after OS success but before acceptance persisted.
        for observed in pending.union(&delivered) {
            if ledger
                .accepted
                .get(&observed.id)
                .is_some_and(|a| a.consumed && a.identity == *observed)
            {
                continue;
            }
            if let Some(request) = desired.requests.iter().find(|r| r.observation() == *observed) {
                ledger.accepted.insert(
                    observed.id.clone(),
                    Accepted {
                        identity: observed.clone(),
                        fire_at_ms: request.fire_at_ms,
                        consumed: delivered.contains(observed) || request.fire_at_ms <= now_ms,
                    },
                );
            } else {
                cancellations.insert(observed.clone());
            }
        }
        if ledger != before {
            ledger.save(g.store.dir())?;
        }
        let mut result = decorate(self::desired(&g.store, &g.prefs, now_ms, &ledger), &ledger);
        cancellations.extend(result.cancellations);
        result.cancellations = cancellations.into_iter().collect();
        Ok(result)
    }

    /// Call only AFTER the OS scheduler succeeds. Revalidate against current source
    /// after the await. On rejection cancel the returned exact revision. On any error,
    /// cancel the supplied request's exact revision: no acceptance was promised.
    pub fn accept_notification(
        &self,
        request: NotificationRequest,
        now_ms: u64,
    ) -> Result<NotificationAcceptance, CoreError> {
        let g = self.lock();
        let before = Ledger::load(g.store.dir())?;
        let mut ledger = before.clone();
        ledger.consume_elapsed(now_ms);
        let identity = request.observation();
        let mut validation = ledger.clone();
        // A duplicate acknowledgement may be idempotent, but its current source must
        // still match. Temporarily lift only its own suppression for that comparison.
        if validation
            .accepted
            .get(&request.id)
            .is_some_and(|a| a.identity == identity)
        {
            validation.accepted.remove(&request.id);
        }
        let valid = desired(&g.store, &g.prefs, now_ms, &validation)
            .requests
            .iter()
            .any(|current| {
                current.observation() == identity
                    && current.content == request.content
                    && current.scheduled_at_ms == request.scheduled_at_ms
                    && request.fire_at_ms >= request.scheduled_at_ms
                    && (request.fire_at_ms == request.scheduled_at_ms || request.fire_at_ms <= now_ms)
            });
        let accepted = valid;
        if valid && !ledger.accepted.get(&request.id).is_some_and(|a| a.identity == identity) {
            ledger.accepted.insert(
                request.id,
                Accepted {
                    identity: identity.clone(),
                    fire_at_ms: request.fire_at_ms,
                    consumed: request.fire_at_ms <= now_ms,
                },
            );
        }
        if ledger != before {
            ledger.save(g.store.dir())?;
        }
        Ok(NotificationAcceptance {
            accepted,
            cancellation: (!accepted).then_some(identity),
        })
    }
}
