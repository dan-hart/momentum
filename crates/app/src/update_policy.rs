// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Pure portal policy: no I/O, user data, or toolkit requirements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Checking,
    Available,
    RestartRequired,
    Installing(u32),
    Current,
    Cancelling,
    Cancelled,
    Failed,
}

pub fn supported(app_id: &str, flatpak_id: Option<&str>, demo: bool) -> bool {
    !demo && app_id == "io.github.dan_hart.Momentum" && flatpak_id == Some(app_id)
}

pub fn available(running: Option<&str>, local: Option<&str>, remote: Option<&str>) -> UpdateState {
    if let (Some(local), Some(remote)) = (local, remote) {
        if local != remote {
            return UpdateState::Available;
        }
        if running.is_some_and(|running| running != local) {
            return UpdateState::RestartRequired;
        }
        if running == Some(local) {
            return UpdateState::Current;
        }
    }
    // Missing portal metadata cannot prove that an application is current.
    UpdateState::Checking
}

/// Creating another monitor resets Flatpak's delayed poll. Reuse any live monitor.
pub fn should_create_monitor(live: bool, state: Option<&UpdateState>) -> bool {
    !live
        && !matches!(
            state,
            Some(UpdateState::Checking | UpdateState::Installing(_) | UpdateState::Cancelling)
        )
}

/// Ignore late/unsolicited progress after an operation has finished.
pub fn accept_progress(state: Option<&UpdateState>) -> bool {
    matches!(state, Some(UpdateState::Installing(_)))
}

pub fn close_after_terminal(
    next: &UpdateState,
    previous: Option<&UpdateState>,
    automatic: bool,
    stop_after_install: bool,
) -> bool {
    let terminal = matches!(
        next,
        UpdateState::RestartRequired | UpdateState::Current | UpdateState::Cancelled | UpdateState::Failed
    );
    let completing_install = matches!(previous, Some(UpdateState::Installing(_) | UpdateState::Cancelling));
    terminal && (*next == UpdateState::Failed || (completing_install && (!automatic || stop_after_install)))
}

pub fn failure(error: Option<&str>) -> UpdateState {
    if matches!(
        error,
        Some("org.freedesktop.portal.Error.Cancelled" | "org.freedesktop.DBus.Error.Cancelled")
    ) {
        UpdateState::Cancelled
    } else {
        UpdateState::Failed
    }
}

pub fn progress(status: u32, percent: u32, error: Option<&str>) -> UpdateState {
    match status {
        0 => UpdateState::Installing(percent.min(100)),
        1 => UpdateState::Current,
        2 => UpdateState::RestartRequired,
        3 => failure(error),
        _ => UpdateState::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_shipped_flatpak_can_update() {
        let id = "io.github.dan_hart.Momentum";
        assert!(supported(id, Some(id), false));
        assert!(!supported(id, None, false));
        assert!(!supported(id, Some(id), true));
        assert!(!supported("io.github.dan_hart.Momentum.Devel", Some(id), false));
    }
    #[test]
    fn monitor_creation_is_not_update_success() {
        assert_eq!(available(None, None, None), UpdateState::Checking);
        assert_eq!(available(None, Some("a"), Some("a")), UpdateState::Checking);
        assert_eq!(available(Some("a"), Some("a"), Some("b")), UpdateState::Available);
        assert_eq!(available(Some("a"), Some("b"), Some("b")), UpdateState::RestartRequired);
        assert_eq!(available(Some("a"), Some("a"), Some("a")), UpdateState::Current);
    }
    #[test]
    fn cancellation_requires_a_recognized_error_name() {
        assert_eq!(
            failure(Some("org.freedesktop.portal.Error.Cancelled")),
            UpdateState::Cancelled
        );
        assert_eq!(failure(Some("UnrelatedNotCancelled")), UpdateState::Failed);
        assert_eq!(failure(None), UpdateState::Failed);
    }
    #[test]
    fn repeated_checks_reuse_existing_monitor() {
        assert!(!should_create_monitor(true, Some(&UpdateState::Checking)));
        assert!(!should_create_monitor(false, Some(&UpdateState::Checking)));
        assert!(!should_create_monitor(true, Some(&UpdateState::Available)));
        assert!(!should_create_monitor(false, Some(&UpdateState::Installing(10))));
        assert!(should_create_monitor(false, Some(&UpdateState::Cancelled)));
        assert!(should_create_monitor(false, Some(&UpdateState::Failed)));
    }
    #[test]
    fn terminal_states_ignore_late_progress() {
        for state in [
            UpdateState::RestartRequired,
            UpdateState::Current,
            UpdateState::Cancelled,
            UpdateState::Failed,
            UpdateState::Checking,
            UpdateState::Cancelling,
        ] {
            assert!(!accept_progress(Some(&state)));
        }
        assert!(accept_progress(Some(&UpdateState::Installing(0))));
    }
    #[test]
    fn disabling_automatic_checks_stops_only_after_install_finishes() {
        assert!(!close_after_terminal(
            &UpdateState::Installing(70),
            Some(&UpdateState::Installing(50)),
            false,
            true
        ));
        assert!(close_after_terminal(
            &UpdateState::RestartRequired,
            Some(&UpdateState::Installing(100)),
            false,
            true
        ));
        assert!(!close_after_terminal(
            &UpdateState::Current,
            Some(&UpdateState::Checking),
            false,
            false
        ));
        assert!(!close_after_terminal(
            &UpdateState::RestartRequired,
            Some(&UpdateState::Installing(100)),
            true,
            false
        ));
        assert!(close_after_terminal(
            &UpdateState::Failed,
            Some(&UpdateState::Checking),
            true,
            false
        ));
    }
    #[test]
    fn terminal_progress_is_not_inferred_from_percentage() {
        assert_eq!(progress(0, 100, None), UpdateState::Installing(100));
        assert_eq!(progress(2, 0, None), UpdateState::RestartRequired);
        assert_eq!(progress(1, 0, None), UpdateState::Current);
        assert_eq!(
            progress(3, 0, Some("org.freedesktop.portal.Error.Cancelled")),
            UpdateState::Cancelled
        );
        assert_eq!(
            progress(3, 0, Some("org.freedesktop.DBus.Error.NotSupported")),
            UpdateState::Failed
        );
        assert_eq!(progress(99, 100, None), UpdateState::Failed);
    }
}
