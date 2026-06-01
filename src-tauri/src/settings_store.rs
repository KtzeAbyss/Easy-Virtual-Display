//! Thin wrapper around `tauri-plugin-store` that holds a single `AppSettings` JSON object.
//!
//! Phase 2 only persists settings (load on startup, save on patch). Phase 4 layers
//! `syncSystemSettings` (`launchOnLogin` / `keepScreenOn` / `fallbackDisplay`) on top of
//! this — those side effects are deliberately *not* in this module so the renderer can be
//! wired before the OS boundaries are.

use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

use crate::contracts::{default_app_settings, AppSettings};

pub const SETTINGS_STORE_PATH: &str = "settings.json";
pub const SETTINGS_KEY: &str = "settings";

/// Read the persisted settings, or return [`default_app_settings`] when the store has not
/// been written yet (first-launch path).
pub fn load_or_default<R: Runtime>(app: &AppHandle<R>) -> AppSettings {
    let Ok(store) = app.store(SETTINGS_STORE_PATH) else {
        return default_app_settings();
    };
    let Some(raw) = store.get(SETTINGS_KEY) else {
        return default_app_settings();
    };
    serde_json::from_value::<AppSettings>(raw).unwrap_or_else(|_| default_app_settings())
}

pub fn save<R: Runtime>(app: &AppHandle<R>, settings: &AppSettings) -> Result<(), String> {
    let store = app
        .store(SETTINGS_STORE_PATH)
        .map_err(|e| format!("failed to open store: {e}"))?;
    let value =
        serde_json::to_value(settings).map_err(|e| format!("failed to serialize settings: {e}"))?;
    store.set(SETTINGS_KEY, value);
    store
        .save()
        .map_err(|e| format!("failed to persist store: {e}"))?;
    Ok(())
}
