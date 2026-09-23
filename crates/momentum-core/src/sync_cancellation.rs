// SPDX-License-Identifier: GPL-3.0-or-later
//! Single-use cancellation survives the gap between scheduling work and FFI admission.
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

const READY: u8 = 0;
const RUNNING: u8 = 1;
const CANCELLED: u8 = 2;
const COMMITTING: u8 = 3;
const FINISHED: u8 = 4;

#[derive(Default)]
#[cfg_attr(feature = "ffi", derive(uniffi::Object))]
pub struct SyncCancellation {
    phase: AtomicU8,
}

#[cfg_attr(feature = "ffi", uniffi::export)]
impl SyncCancellation {
    #[cfg_attr(feature = "ffi", uniffi::constructor)]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Accept cancellation before admission or the final commit boundary. False
    /// means already cancelled, finished, or committing. I/O still has to drain.
    /// This operation is lock-free and never waits for networking or persistence.
    pub fn cancel(&self) -> bool {
        self.phase
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |phase| {
                matches!(phase, READY | RUNNING).then_some(CANCELLED)
            })
            .is_ok()
    }

    pub fn is_cancelled(&self) -> bool {
        self.phase.load(Ordering::SeqCst) == CANCELLED
    }
}

impl SyncCancellation {
    pub(crate) fn begin(&self) -> bool {
        self.phase
            .compare_exchange(READY, RUNNING, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Called while holding the store owner lock. Whichever transition wins sets
    /// whether this exchange can publish; a late cancellation cannot undo a commit.
    pub(crate) fn begin_commit(&self) -> bool {
        self.phase
            .compare_exchange(RUNNING, COMMITTING, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub(crate) fn finish(&self) {
        let _ = self.phase.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |phase| {
            matches!(phase, RUNNING | COMMITTING).then_some(FINISHED)
        });
    }
}

pub(crate) struct SyncOperation<'a>(pub &'a SyncCancellation);
impl Drop for SyncOperation<'_> {
    fn drop(&mut self) {
        self.0.finish();
    }
}
