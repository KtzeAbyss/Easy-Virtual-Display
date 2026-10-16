//! The 11 `#[tauri::command]` handlers wiring Seam A to the Rust backend + shell.
//!
//! Mapping (Seam A TS method → Tauri command id):
//!   getSnapshot         → get_snapshot
//!   subscribeSnapshot   → (event `snapshot-changed`)
//!   onLanguageChanged   → (event `language-changed`)
//!   installDriver       → install_driver         [Phase 5: wire elevation]
//!   uninstallDriver     → uninstall_driver       [Phase 5: wire elevation]
//!   addDisplay          → add_display
//!   removeDisplay       → remove_display
//!   removeAllDisplays   → remove_all_displays
//!   setDisplayMode      → set_display_mode
//!   applyAdminConfig    → apply_admin_config     [Phase 5: wire elevation]
//!   updateSettings      → update_settings
//!   openDisplaySettings → open_display_settings
//!   showMainWindow      → show_main_window
//!   openDisplayViewer   → open_display_viewer
//!   closeDisplayViewer  → close_display_viewer

use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::app_state::AppRuntime;
use crate::contracts::{
    AppSettings, AppSettingsPatch, AppSnapshot, ApplyAdminConfigInput, SetDisplayModeInput,
};
use crate::drivers;
use crate::errors::{EasyVirtualDisplayError, EasyVirtualDisplayErrorCode};
use crate::events::emit_language_changed;
use crate::settings_store;
use crate::shell_locales::t_viewer;
use crate::tray;
use crate::viewer::{ViewerCaptions, ViewerTarget};

#[tauri::command]
pub async fn get_snapshot(
    runtime: State<'_, AppRuntime>,
) -> Result<AppSnapshot, EasyVirtualDisplayError> {
    let host = runtime.backend.get_snapshot().await?;
    Ok(runtime.compose_app_snapshot(host))
}

#[tauri::command]
pub async fn add_display(runtime: State<'_, AppRuntime>) -> Result<(), EasyVirtualDisplayError> {
    runtime.backend.add_display().await
}

#[tauri::command]
pub async fn remove_display(
    runtime: State<'_, AppRuntime>,
    index: Option<i32>,
) -> Result<(), EasyVirtualDisplayError> {
    runtime.backend.remove_display(index).await
}

#[tauri::command]
pub async fn remove_all_displays(
    runtime: State<'_, AppRuntime>,
) -> Result<(), EasyVirtualDisplayError> {
    runtime.backend.remove_all_displays().await
}

#[tauri::command]
pub async fn set_display_mode(
    runtime: State<'_, AppRuntime>,
    input: SetDisplayModeInput,
) -> Result<(), EasyVirtualDisplayError> {
    runtime.backend.set_display_mode(input).await
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    patch: AppSettingsPatch,
) -> Result<(), EasyVirtualDisplayError> {
    let merged = merge_settings(runtime.settings_snapshot(), patch);

    if let Err(e) = settings_store::save(&app, &merged) {
        return Err(EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::DriverError,
            format!("Failed to persist settings: {e}"),
        ));
    }

    let language_change = runtime.replace_settings(merged.clone());

    // OS-side effects of the new settings (autostart, display-sleep blocker). The
    // fallback-display boundary is snapshot-driven, so we fire it below from the fresh
    // host snapshot to catch the edge case "user just enabled fallbackDisplay while no
    // displays are active".
    runtime.boundaries.sync_settings(&merged);

    if let Some(lang) = language_change {
        emit_language_changed(&app, lang);
        runtime.viewers.update_captions(viewer_captions(lang));
    }

    // Make sure the host is running (and surface it if it is not), then re-emit the
    // published state so every consumer sees the same host snapshot.
    runtime.backend.start().await?;
    let snap = runtime.republish_snapshot(&app);
    tray::refresh(&snap);
    runtime.boundaries.handle_snapshot(&snap.host, &merged).await;
    Ok(())
}

fn merge_settings(mut cur: AppSettings, patch: AppSettingsPatch) -> AppSettings {
    if let Some(v) = patch.launch_on_login {
        cur.launch_on_login = v;
    }
    if let Some(v) = patch.close_to_tray {
        cur.close_to_tray = v;
    }
    if let Some(v) = patch.start_minimized {
        cur.start_minimized = v;
    }
    if let Some(v) = patch.fallback_display {
        cur.fallback_display = v;
    }
    if let Some(v) = patch.keep_screen_on {
        cur.keep_screen_on = v;
    }
    if let Some(v) = patch.theme {
        cur.theme = v;
    }
    if let Some(v) = patch.language {
        cur.language = v;
    }
    cur
}

fn viewer_captions(lang: crate::contracts::EffectiveLanguage) -> ViewerCaptions {
    ViewerCaptions {
        title_template: t_viewer(lang, "title").to_string(),
        hint_idle: t_viewer(lang, "hint_idle").to_string(),
        hint_grabbed: t_viewer(lang, "hint_grabbed").to_string(),
    }
}

#[tauri::command]
pub async fn install_driver(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
) -> Result<(), EasyVirtualDisplayError> {
    drivers::install_driver(&app, &runtime).await
}

#[tauri::command]
pub async fn uninstall_driver(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
) -> Result<(), EasyVirtualDisplayError> {
    drivers::uninstall_driver(&app, &runtime).await
}

#[tauri::command]
pub async fn apply_admin_config(
    app: AppHandle,
    runtime: State<'_, AppRuntime>,
    input: ApplyAdminConfigInput,
) -> Result<(), EasyVirtualDisplayError> {
    drivers::apply_admin_config(&app, &runtime, input).await
}

#[tauri::command]
pub fn open_display_settings(app: AppHandle) -> Result<(), EasyVirtualDisplayError> {
    app.opener()
        .open_url("ms-settings:display", None::<&str>)
        .map_err(|e| {
            EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::DriverError,
                format!("Failed to open system display settings: {e}"),
            )
        })
}

/// Open (or focus) the mirror window for a virtual display — issue #39.
#[tauri::command]
pub async fn open_display_viewer(
    runtime: State<'_, AppRuntime>,
    index: i32,
) -> Result<(), EasyVirtualDisplayError> {
    let host = runtime.backend.get_snapshot().await?;
    let display = host
        .displays
        .iter()
        .find(|d| d.index == index)
        .ok_or_else(|| {
            EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::DisplayNotFound,
                format!("No virtual display with index {index}."),
            )
        })?;

    if !display.active {
        return Err(EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::DisplayNotFound,
            format!("Virtual display {index} is not active, so it has nothing to mirror."),
        ));
    }

    let lang = runtime.effective_language();
    let target = ViewerTarget {
        index,
        device_name: display.device_name.clone(),
        captions: viewer_captions(lang),
    };

    // Waiting for the viewer UI thread to build the window, D3D device and capture
    // session blocks briefly — keep it off the async runtime's worker.
    let viewers = runtime.viewers.clone();
    tauri::async_runtime::spawn_blocking(move || viewers.open(target))
        .await
        .map_err(|error| {
            EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::ViewerStartFailed,
                format!("Display viewer startup was interrupted: {error}"),
            )
        })?
}

#[tauri::command]
pub fn close_display_viewer(
    runtime: State<'_, AppRuntime>,
    index: i32,
) -> Result<(), EasyVirtualDisplayError> {
    runtime.viewers.close(index);
    Ok(())
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), EasyVirtualDisplayError> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{default_app_settings, AppLanguage, AppTheme};

    #[test]
    fn merge_settings_applies_only_provided_fields() {
        let cur = default_app_settings();
        let patch = AppSettingsPatch {
            theme: Some(AppTheme::Dark),
            language: Some(AppLanguage::ZhCn),
            ..Default::default()
        };
        let merged = merge_settings(cur.clone(), patch);
        assert_eq!(merged.theme, AppTheme::Dark);
        assert_eq!(merged.language, AppLanguage::ZhCn);
        // Other fields untouched.
        assert_eq!(merged.launch_on_login, cur.launch_on_login);
        assert_eq!(merged.close_to_tray, cur.close_to_tray);
        assert_eq!(merged.keep_screen_on, cur.keep_screen_on);
    }

    #[test]
    fn merge_settings_with_empty_patch_is_identity() {
        let cur = default_app_settings();
        let merged = merge_settings(cur.clone(), AppSettingsPatch::default());
        assert_eq!(merged, cur);
    }

    #[test]
    fn viewer_captions_follow_effective_language() {
        let en = viewer_captions(crate::contracts::EffectiveLanguage::En);
        let zh = viewer_captions(crate::contracts::EffectiveLanguage::ZhCn);

        assert_ne!(en, zh);
        assert!(en.title_template.contains("{{index}}"));
        assert!(zh.title_template.contains("{{index}}"));
    }
}
