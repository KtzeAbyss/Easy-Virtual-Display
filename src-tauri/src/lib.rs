mod app_state;
mod boundaries;
mod commands;
mod contracts;
mod drivers;
mod elevator;
mod errors;
mod events;
mod host;
mod install_prompt;
mod quit;
mod settings_store;
mod shell_locales;
mod tray;

pub use app_state::AppRuntime;
pub use contracts::*;
pub use errors::*;
pub use host::{
    resolve_admin_command, resolve_install_driver_command, resolve_stdio_command,
    resolve_uninstall_driver_command, HostCommand, StdioJsonRpcBackend,
};

use std::sync::Arc;

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance MUST be registered first (per migration spec §6 Phase 0):
        // earlier than other plugins, setup side-effects, and any window/tray/sidecar work.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::add_display,
            commands::remove_display,
            commands::remove_all_displays,
            commands::set_display_mode,
            commands::update_settings,
            commands::install_driver,
            commands::uninstall_driver,
            commands::apply_admin_config,
            commands::open_display_settings,
            commands::show_main_window,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            let settings = settings_store::load_or_default(&app_handle);
            let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());

            let factory_handle = app_handle.clone();
            let command_factory: host::CommandFactory =
                Arc::new(move || resolve_stdio_command(&factory_handle));

            let backend = StdioJsonRpcBackend::new(command_factory, empty_host_snapshot());
            let boundaries = boundaries::SystemBoundaries::new(app_handle.clone(), backend.clone());

            // Apply persisted launch-on-login + keep-screen-on at boot. fallback_display
            // is snapshot-driven (handled below by the broadcaster) so we don't fire it here.
            boundaries.sync_settings(&settings);

            // Compose AppSnapshot on every applied host snapshot, then fan out to (a) the
            // renderer via `snapshot-changed`, (b) the tray (tooltip + checkboxes), and
            // (c) the fallback-display boundary (which may schedule a debounced add).
            let broadcaster_handle = app_handle.clone();
            backend.set_snapshot_broadcaster(Arc::new(move |host: &HostSnapshot| {
                let Some(runtime) = broadcaster_handle.try_state::<AppRuntime>() else {
                    return;
                };
                let snapshot = runtime.compose_app_snapshot(host.clone());
                events::emit_snapshot(&broadcaster_handle, &snapshot);
                tray::refresh(&snapshot);

                let host_clone = host.clone();
                let settings = snapshot.settings.clone();
                let handle = broadcaster_handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(runtime) = handle.try_state::<AppRuntime>() {
                        runtime
                            .boundaries
                            .handle_snapshot(&host_clone, &settings)
                            .await;
                        install_prompt::maybe_prompt_to_install_driver(
                            &handle,
                            runtime.inner(),
                            &host_clone,
                        )
                        .await;
                    }
                });
            }));

            let elevator = elevator::create();
            let runtime = AppRuntime::new(backend, boundaries, elevator, settings, system_locale);
            app.manage(runtime);

            tray::setup(&app_handle)?;

            // Intercept the X button: close-to-tray hides the window; otherwise run the
            // quit flow ourselves (so we can confirm with the user and tear the host down
            // cleanly). Mirrors `src/main/window.ts:56` + `src/main/app-quit.ts`.
            if let Some(window) = app.get_webview_window("main") {
                let close_handle = app_handle.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        let Some(runtime) = close_handle.try_state::<AppRuntime>() else {
                            return;
                        };
                        api.prevent_close();
                        if runtime.settings_snapshot().close_to_tray {
                            if let Some(win) = close_handle.get_webview_window("main") {
                                let _ = win.hide();
                            }
                        } else {
                            quit::request_quit(close_handle.clone());
                        }
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
