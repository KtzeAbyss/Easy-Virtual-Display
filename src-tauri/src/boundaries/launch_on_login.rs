//! Auto-start on login. Delegates to `tauri-plugin-autostart` which already handles the
//! Windows registry key + macOS LaunchAgent + Linux .desktop autostart files. The trait
//! is here so a macOS-bound backend can later swap a different implementation.

use tauri::{AppHandle, Wry};
use tauri_plugin_autostart::ManagerExt;

pub trait LaunchOnLoginBoundary: Send + Sync {
    fn sync(&self, enabled: bool);
}

pub fn create(app: AppHandle<Wry>) -> std::sync::Arc<dyn LaunchOnLoginBoundary> {
    std::sync::Arc::new(PluginBoundary { app })
}

struct PluginBoundary {
    app: AppHandle<Wry>,
}

impl LaunchOnLoginBoundary for PluginBoundary {
    fn sync(&self, enabled: bool) {
        let manager = self.app.autolaunch();
        let _ = if enabled {
            manager.enable()
        } else {
            manager.disable()
        };
    }
}
