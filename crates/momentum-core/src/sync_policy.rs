// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! One foreground cadence policy for every native client. OS background scheduling
//! remains native; these durations never authorize waking a suspended app.

/// Bounded first-edit coalescing window. Callers retain the first scheduled wake.
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn sync_edit_delay_ms() -> u64 {
    750
}

/// Foreground reconciliation interval from monotonic idle time, not wall-clock time.
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn sync_poll_delay_ms(idle_ms: u64) -> u64 {
    match idle_ms {
        0..60_000 => 5_000,
        60_000..300_000 => 15_000,
        _ => 150_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_have_a_bounded_subsecond_coalescing_window() {
        assert_eq!(sync_edit_delay_ms(), 750);
    }

    #[test]
    fn active_polling_backs_off_at_inclusive_monotonic_boundaries() {
        for idle in [0, 1, 59_999] {
            assert_eq!(sync_poll_delay_ms(idle), 5_000);
        }
        for idle in [60_000, 60_001, 299_999] {
            assert_eq!(sync_poll_delay_ms(idle), 15_000);
        }
        for idle in [300_000, u64::MAX] {
            assert_eq!(sync_poll_delay_ms(idle), 150_000);
        }
    }
}

/// Stable opaque connection identity shared by native consent preferences.
#[cfg_attr(feature = "ffi", uniffi::export)]
pub fn nextcloud_connection_identity(server_url: String, user_name: String, folder: String) -> String {
    momentum_sync::optimized::collection_identity(&momentum_sync::NextcloudCfg {
        server_url,
        user_name,
        folder,
        password: String::new(),
        compress: false,
        encrypt_key: None,
    })
}
