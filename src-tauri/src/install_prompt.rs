//! Auto-install the bundled Parsec virtual display driver when it is missing.
//!
//! Latch semantics:
//!   - host status anything other than `not_installed` → reset the latch so a *future*
//!     uninstall (e.g. user removed the driver from Windows Settings) can auto-install again;
//!   - host status `not_installed` → claim the latch atomically; only the winning caller
//!     starts `drivers::install_driver` (same flow as the renderer's "Install Driver"
//!     button, including the UAC prompt). Skipped while the main window is hidden
//!     (background / start-in-tray launches) so UAC does not pop uninvited; the UI still
//!     offers a manual install.

use tauri::{AppHandle, Manager, Wry};

use crate::app_state::AppRuntime;
use crate::contracts::{DriverStatus, HostSnapshot};
use crate::drivers;

pub async fn maybe_prompt_to_install_driver(
    app: &AppHandle<Wry>,
    runtime: &AppRuntime,
    host: &HostSnapshot,
) {
    if host.status != DriverStatus::NotInstalled {
        runtime.set_install_prompt_shown(false);
        return;
    }
    let window_visible = app
        .get_webview_window("main")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    if !window_visible || !runtime.try_claim_install_prompt() {
        return;
    }

    // Errors surface via the next host snapshot (e.g. UAC cancelled → still not_installed).
    let _ = drivers::install_driver(app, runtime).await;
}
