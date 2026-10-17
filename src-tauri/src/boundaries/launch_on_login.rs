//! Auto-start on login. Delegates to `tauri-plugin-autostart` which already handles the
//! Windows registry key + macOS LaunchAgent + Linux .desktop autostart files. The trait
//! is here so a macOS-bound backend can later swap a different implementation.
//!
//! The OS entry is the source of truth: the user can disable it from Task Manager behind
//! our back, so we only write it when the user flips our own toggle, and read it back at
//! boot instead of re-asserting the persisted setting.

use tauri::{AppHandle, Wry};
use tauri_plugin_autostart::ManagerExt;

pub trait LaunchOnLoginBoundary: Send + Sync {
    /// Actual OS state, or `None` when it can't be read.
    fn read(&self) -> Option<bool>;
    fn apply(&self, enabled: bool) -> Result<(), String>;
}

pub fn create(app: AppHandle<Wry>) -> std::sync::Arc<dyn LaunchOnLoginBoundary> {
    std::sync::Arc::new(PluginBoundary { app })
}

struct PluginBoundary {
    app: AppHandle<Wry>,
}

impl LaunchOnLoginBoundary for PluginBoundary {
    fn read(&self) -> Option<bool> {
        self.app.autolaunch().is_enabled().ok()
    }

    fn apply(&self, enabled: bool) -> Result<(), String> {
        let manager = self.app.autolaunch();
        if enabled {
            return manager.enable().map_err(|e| e.to_string());
        }
        // Deleting an entry that is already gone fails; the plugin flattens errors to
        // strings, so check the outcome rather than the error kind.
        match manager.disable() {
            Ok(()) => Ok(()),
            Err(_) if self.read() == Some(false) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Boot-time reconciliation. Returns the OS state when known. An entry that is still
/// enabled gets rewritten so its command line tracks the current executable path and
/// launch arguments; `enable()` also clears a Task Manager override, which is harmless
/// here because `read()` only reports `true` when no such override is in effect.
pub fn reconcile_at_boot(boundary: &dyn LaunchOnLoginBoundary) -> Option<bool> {
    let actual = boundary.read()?;
    if actual {
        let _ = boundary.apply(true);
    }
    Some(actual)
}
