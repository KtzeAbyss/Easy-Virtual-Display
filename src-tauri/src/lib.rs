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
mod viewer;

pub use app_state::AppRuntime;
pub use contracts::*;
pub use errors::*;
pub use host::{
    resolve_admin_command, resolve_install_driver_command, resolve_stdio_command,
    resolve_uninstall_driver_command, HostCommand, StdioJsonRpcBackend,
};

use std::sync::Arc;

use tauri::{Manager, WindowEvent};

/// Passed by the launch-on-login entry: start with no window and no install prompt.
const BACKGROUND_ARG: &str = "--background";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance MUST be registered first (per migration spec §6 Phase 0):
        // earlier than other plugins, setup side-effects, and any window/tray/sidecar work.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if args.iter().any(|arg| arg == BACKGROUND_ARG) {
                return;
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![BACKGROUND_ARG]),
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
            commands::open_display_viewer,
            commands::close_display_viewer,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            let mut settings = settings_store::load_or_default(&app_handle);
            let launched_in_background = std::env::args().any(|arg| arg == BACKGROUND_ARG);
            let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());

            let factory_handle = app_handle.clone();
            let command_factory: host::CommandFactory =
                Arc::new(move || resolve_stdio_command(&factory_handle));

            let backend = StdioJsonRpcBackend::new(command_factory, empty_host_snapshot());
            let boundaries = boundaries::SystemBoundaries::new(app_handle.clone(), backend.clone());

            // Viewer windows live outside the snapshot pipeline (the user can close one
            // directly), so re-broadcast the composed snapshot whenever the open set
            // changes — the renderer then flips its toggles without a second channel.
            let viewer_handle = app_handle.clone();
            let viewers = viewer::create(Arc::new(move || {
                let Some(runtime) = viewer_handle.try_state::<AppRuntime>() else {
                    return;
                };
                runtime.republish_snapshot(&viewer_handle);
            }));

            // The OS autostart entry wins over the persisted setting (the user may have
            // disabled it from Task Manager); an unreadable state keeps the stored value.
            if let Some(actual) = boundaries.reconcile_launch_on_login() {
                if actual != settings.launch_on_login {
                    settings.launch_on_login = actual;
                    let _ = settings_store::save(&app_handle, &settings);
                }
            }

            // Apply keep-screen-on at boot. fallback_display is snapshot-driven (handled
            // below by the broadcaster) so we don't fire it here.
            boundaries.sync_settings(&settings);

            // Compose AppSnapshot on every applied host snapshot, then fan out to (a) the
            // renderer via `snapshot-changed`, (b) the tray (tooltip + checkboxes), and
            // (c) the fallback-display boundary (which may schedule a debounced add).
            let broadcaster_handle = app_handle.clone();
            backend.set_snapshot_broadcaster(Arc::new(move |session_id, host: &HostSnapshot| {
                let Some(runtime) = broadcaster_handle.try_state::<AppRuntime>() else {
                    return;
                };

                let Some(snapshot) =
                    runtime.publish_host_snapshot(&broadcaster_handle, session_id, host.clone())
                else {
                    return;
                };
                tray::refresh(&snapshot);

                let host_clone = snapshot.host.clone();
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

            let show_window = !launched_in_background && !settings.start_minimized;

            let elevator = elevator::create();
            let runtime = AppRuntime::new(
                backend,
                boundaries,
                elevator,
                viewers,
                settings,
                system_locale,
            );
            app.manage(runtime);

            tray::setup(&app_handle)?;

            // The window starts hidden (tauri.conf.json) so start-in-tray never flashes it.
            if show_window {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // Start the host now rather than waiting for the renderer's first getSnapshot,
            // so fallback display and the tray work with the window hidden. A failure is
            // retried, and surfaced, by the renderer's getSnapshot once the window opens.
            let start_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let Some(runtime) = start_handle.try_state::<AppRuntime>() else {
                    return;
                };
                if let Err(e) = runtime.backend.start().await {
                    eprintln!("native host failed to start: {e}");
                }
            });

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
