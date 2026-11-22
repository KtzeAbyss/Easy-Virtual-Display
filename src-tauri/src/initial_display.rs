//! Ensure exactly one virtual display exists after the very first successful driver boot.
//!
//! Independent of the "fallback display" setting: once the store flag is set, clearing
//! every virtual display later will not recreate one.

use tauri::{AppHandle, Wry};

use crate::app_state::AppRuntime;
use crate::contracts::{DriverStatus, HostSnapshot};
use crate::settings_store;

/// Pure decision used by tests: whether this snapshot should trigger an add.
pub(crate) fn should_add_initial_display(host: &HostSnapshot, already_ensured: bool) -> bool {
    !already_ensured && host.status == DriverStatus::Ok && host.displays.is_empty()
}

/// Mark the first-run ensure as done when the machine already has a virtual display, so a
/// later wipe does not re-add. Returns whether the persisted flag was updated.
pub(crate) fn should_mark_ensured_without_add(host: &HostSnapshot, already_ensured: bool) -> bool {
    !already_ensured && !host.displays.is_empty()
}

pub async fn maybe_ensure_initial_display(
    app: &AppHandle<Wry>,
    runtime: &AppRuntime,
    host: &HostSnapshot,
) {
    if settings_store::initial_display_ensured(app) {
        return;
    }

    if should_mark_ensured_without_add(host, false) {
        let _ = settings_store::set_initial_display_ensured(app, true);
        return;
    }

    if !should_add_initial_display(host, false) {
        return;
    }

    if !runtime.try_claim_initial_display() {
        return;
    }

    // Persist before/after either outcome so a failed add cannot loop on every snapshot.
    let _ = runtime.backend.add_display().await;
    let _ = settings_store::set_initial_display_ensured(app, true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{empty_host_snapshot, DisplaySummary, Orientation};

    fn ok_empty() -> HostSnapshot {
        let mut host = empty_host_snapshot();
        host.status = DriverStatus::Ok;
        host
    }

    fn with_display() -> HostSnapshot {
        let mut host = ok_empty();
        host.displays = vec![DisplaySummary {
            index: 0,
            identifier: 1,
            device_name: "VDD".into(),
            display_name: "Virtual".into(),
            active: true,
            current_mode: None,
            current_orientation: Orientation::Landscape,
            supported_resolutions: vec![],
            unsupported_current_mode: false,
        }];
        host
    }

    #[test]
    fn skips_when_already_ensured() {
        assert!(!should_add_initial_display(&ok_empty(), true));
        assert!(!should_mark_ensured_without_add(&with_display(), true));
    }

    #[test]
    fn skips_when_driver_not_ok() {
        let mut host = ok_empty();
        host.status = DriverStatus::NotInstalled;
        assert!(!should_add_initial_display(&host, false));
    }

    #[test]
    fn adds_when_ok_and_empty() {
        assert!(should_add_initial_display(&ok_empty(), false));
    }

    #[test]
    fn marks_without_add_when_displays_already_present() {
        assert!(!should_add_initial_display(&with_display(), false));
        assert!(should_mark_ensured_without_add(&with_display(), false));
    }
}
