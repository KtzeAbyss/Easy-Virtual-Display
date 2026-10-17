//! OS-touching side effects for `syncSystemSettings` — each behind a trait or pure-Rust
//! module so a future macOS port can drop in its own implementations.

mod fallback_display;
mod keep_screen_on;
mod launch_on_login;

use std::sync::Arc;

use tauri::{AppHandle, Wry};

pub use fallback_display::{AddDisplayCallback, FallbackDisplayBoundary};
pub use keep_screen_on::KeepScreenOnBoundary;
pub use launch_on_login::LaunchOnLoginBoundary;

use crate::contracts::{AppSettings, HostSnapshot};
use crate::host::StdioJsonRpcBackend;

/// Aggregates the three boundaries — the orchestrator mirrors `syncSystemSettings`
/// from `src/main/app-settings.ts` but split per signal:
///   `sync_settings(settings)` runs at boot and on settings change (keep_screen_on)
///   `launch_on_login` is written only when its own toggle flips (see `apply_settings_patch`)
///   `handle_snapshot(host, settings)` runs on every snapshot (fallback_display)
pub struct SystemBoundaries {
    pub launch_on_login: Arc<dyn LaunchOnLoginBoundary>,
    pub keep_screen_on: Arc<dyn KeepScreenOnBoundary>,
    pub fallback_display: Arc<FallbackDisplayBoundary>,
}

impl SystemBoundaries {
    pub fn new(app: AppHandle<Wry>, backend: Arc<StdioJsonRpcBackend>) -> Self {
        let backend_for_fallback = backend.clone();
        let add_display: AddDisplayCallback = Arc::new(move || {
            let b = backend_for_fallback.clone();
            Box::pin(async move { b.add_display().await })
        });

        Self {
            launch_on_login: launch_on_login::create(app),
            keep_screen_on: keep_screen_on::create(),
            fallback_display: FallbackDisplayBoundary::new(add_display),
        }
    }

    pub fn sync_settings(&self, settings: &AppSettings) {
        self.keep_screen_on.sync(settings.keep_screen_on);
    }

    /// Actual launch-on-login state at boot, or `None` when the OS can't tell us.
    pub fn reconcile_launch_on_login(&self) -> Option<bool> {
        launch_on_login::reconcile_at_boot(self.launch_on_login.as_ref())
    }

    pub async fn handle_snapshot(&self, host: &HostSnapshot, settings: &AppSettings) {
        self.fallback_display.handle_snapshot(host, settings).await;
    }

    pub async fn dispose(&self) {
        self.fallback_display.dispose().await;
    }

    /// Build a no-op `SystemBoundaries` for unit tests that don't have a real Tauri
    /// `AppHandle`. The fallback_display callback never actually adds a display.
    #[cfg(test)]
    pub fn noop_for_tests() -> Self {
        struct NoopLaunch;
        impl LaunchOnLoginBoundary for NoopLaunch {
            fn read(&self) -> Option<bool> {
                None
            }
            fn apply(&self, _enabled: bool) -> Result<(), String> {
                Ok(())
            }
        }
        struct NoopKeepScreen;
        impl KeepScreenOnBoundary for NoopKeepScreen {
            fn sync(&self, _enabled: bool) {}
        }
        let add: AddDisplayCallback =
            Arc::new(|| Box::pin(async move { Ok(()) }));
        Self {
            launch_on_login: Arc::new(NoopLaunch),
            keep_screen_on: Arc::new(NoopKeepScreen),
            fallback_display: FallbackDisplayBoundary::new(add),
        }
    }
}
