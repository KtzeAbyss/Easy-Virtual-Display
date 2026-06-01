//! System tray icon + menu. The menu items are built once during setup and their text /
//! enabled / checked state is updated on every snapshot or language change, using Tauri 2's
//! menu types.
//!
//! Click routing:
//!   show              → show the main window
//!   add_display       → backend.add_display()
//!   remove_last       → backend.remove_display(None) (host removes the trailing active one)
//!   keep_screen_on    → toggle AppSettings.keepScreenOn via update_settings
//!   launch_on_login   → toggle AppSettings.launchOnLogin via update_settings
//!   quit              → quit::request_quit flow (with confirmation dialog if displays active)
//!
//! Tray icon left-click also shows the main window (parity with `src/main/tray.ts:64`).

use std::sync::Mutex;

use once_cell::sync::OnceCell;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::app_state::AppRuntime;
use crate::contracts::{AppLanguage, AppSnapshot, AppSettingsPatch};
use crate::quit;
use crate::settings_store;
use crate::shell_locales::{format_template, t_tray};

pub const TRAY_ITEM_SHOW: &str = "tray_show";
pub const TRAY_ITEM_ADD_DISPLAY: &str = "tray_add_display";
pub const TRAY_ITEM_REMOVE_LAST: &str = "tray_remove_last";
pub const TRAY_ITEM_KEEP_SCREEN_ON: &str = "tray_keep_screen_on";
pub const TRAY_ITEM_LAUNCH_ON_LOGIN: &str = "tray_launch_on_login";
pub const TRAY_ITEM_QUIT: &str = "tray_quit";

/// Long-lived references to the menu items so we can update text / checked / enabled
/// state without rebuilding the menu. Stored in a process-singleton because the
/// snapshot/language broadcasters need to reach them from anywhere.
struct TrayHandles {
    show: MenuItem<Wry>,
    add_display: MenuItem<Wry>,
    remove_last: MenuItem<Wry>,
    keep_screen_on: CheckMenuItem<Wry>,
    launch_on_login: CheckMenuItem<Wry>,
    quit: MenuItem<Wry>,
    tray: tauri::tray::TrayIcon<Wry>,
}

static HANDLES: OnceCell<Mutex<TrayHandles>> = OnceCell::new();

pub fn setup(app: &AppHandle<Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = app
        .try_state::<AppRuntime>()
        .ok_or("AppRuntime must be managed before tray setup")?;
    let lang = runtime.effective_language();
    let settings = runtime.settings_snapshot();

    let show = MenuItem::with_id(app, TRAY_ITEM_SHOW, t_tray(lang, "show"), true, None::<&str>)?;
    let add_display = MenuItem::with_id(
        app,
        TRAY_ITEM_ADD_DISPLAY,
        t_tray(lang, "add_display"),
        true,
        None::<&str>,
    )?;
    let remove_last = MenuItem::with_id(
        app,
        TRAY_ITEM_REMOVE_LAST,
        t_tray(lang, "remove_last_display"),
        false, // host snapshot starts empty — enable once we have active displays
        None::<&str>,
    )?;
    let keep_screen_on = CheckMenuItem::with_id(
        app,
        TRAY_ITEM_KEEP_SCREEN_ON,
        t_tray(lang, "keep_screen_on"),
        true,
        settings.keep_screen_on,
        None::<&str>,
    )?;
    let launch_on_login = CheckMenuItem::with_id(
        app,
        TRAY_ITEM_LAUNCH_ON_LOGIN,
        t_tray(lang, "launch_on_login"),
        true,
        settings.launch_on_login,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, TRAY_ITEM_QUIT, t_tray(lang, "quit"), true, None::<&str>)?;

    let sep_1 = PredefinedMenuItem::separator(app)?;
    let sep_2 = PredefinedMenuItem::separator(app)?;
    let sep_3 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &show,
            &sep_1,
            &add_display,
            &remove_last,
            &sep_2,
            &keep_screen_on,
            &launch_on_login,
            &sep_3,
            &quit,
        ],
    )?;

    let tray = TrayIconBuilder::with_id("main")
        .icon(
            app.default_window_icon()
                .ok_or("default window icon missing")?
                .clone(),
        )
        .tooltip(t_tray(lang, "tooltip_inactive"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_icon_event)
        .build(app)?;

    HANDLES
        .set(Mutex::new(TrayHandles {
            show,
            add_display,
            remove_last,
            keep_screen_on,
            launch_on_login,
            quit,
            tray,
        }))
        .map_err(|_| "tray handles already initialized")?;

    Ok(())
}

/// Re-render menu labels, checkbox state, enabled state and tooltip from the snapshot.
/// Called from the snapshot broadcaster (host snapshot arrived) and from update_settings
/// (settings or language changed).
pub fn refresh(snapshot: &AppSnapshot) {
    let Some(cell) = HANDLES.get() else {
        return;
    };
    let Ok(guard) = cell.lock() else {
        return;
    };

    let lang = snapshot.effective_language;
    let active_count = snapshot.host.displays.iter().filter(|d| d.active).count();

    let _ = guard.show.set_text(t_tray(lang, "show"));
    let _ = guard.add_display.set_text(t_tray(lang, "add_display"));
    let _ = guard
        .remove_last
        .set_text(t_tray(lang, "remove_last_display"));
    let _ = guard.remove_last.set_enabled(active_count > 0);
    let _ = guard
        .keep_screen_on
        .set_text(t_tray(lang, "keep_screen_on"));
    let _ = guard
        .keep_screen_on
        .set_checked(snapshot.settings.keep_screen_on);
    let _ = guard
        .launch_on_login
        .set_text(t_tray(lang, "launch_on_login"));
    let _ = guard
        .launch_on_login
        .set_checked(snapshot.settings.launch_on_login);
    let _ = guard.quit.set_text(t_tray(lang, "quit"));

    let tooltip = if active_count > 0 {
        format_template(t_tray(lang, "tooltip_active"), &[("count", &active_count.to_string())])
    } else {
        t_tray(lang, "tooltip_inactive").to_string()
    };
    let _ = guard.tray.set_tooltip(Some(tooltip));

    // Discourage callers that have a stale AppLanguage::System leftover from drifting.
    debug_assert!(matches!(
        snapshot.settings.language,
        AppLanguage::System | AppLanguage::En | AppLanguage::ZhCn
    ));
}

fn handle_menu_event(app: &AppHandle<Wry>, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref().to_string();
    match id.as_str() {
        TRAY_ITEM_SHOW => show_main_window(app),
        TRAY_ITEM_ADD_DISPLAY => spawn_add_display(app.clone()),
        TRAY_ITEM_REMOVE_LAST => spawn_remove_last(app.clone()),
        TRAY_ITEM_KEEP_SCREEN_ON => spawn_toggle_keep_screen_on(app.clone()),
        TRAY_ITEM_LAUNCH_ON_LOGIN => spawn_toggle_launch_on_login(app.clone()),
        TRAY_ITEM_QUIT => quit::request_quit(app.clone()),
        _ => {}
    }
}

fn handle_tray_icon_event(tray: &tauri::tray::TrayIcon<Wry>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        show_main_window(tray.app_handle());
    }
}

fn show_main_window(app: &AppHandle<Wry>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn spawn_add_display(app: AppHandle<Wry>) {
    tauri::async_runtime::spawn(async move {
        let Some(runtime) = app.try_state::<AppRuntime>() else {
            return;
        };
        let _ = runtime.backend.add_display().await;
    });
}

fn spawn_remove_last(app: AppHandle<Wry>) {
    tauri::async_runtime::spawn(async move {
        let Some(runtime) = app.try_state::<AppRuntime>() else {
            return;
        };
        let _ = runtime.backend.remove_display(None).await;
    });
}

fn spawn_toggle_keep_screen_on(app: AppHandle<Wry>) {
    tauri::async_runtime::spawn(async move {
        toggle_setting(&app, |patch, current| {
            patch.keep_screen_on = Some(!current.keep_screen_on);
        })
        .await;
    });
}

fn spawn_toggle_launch_on_login(app: AppHandle<Wry>) {
    tauri::async_runtime::spawn(async move {
        toggle_setting(&app, |patch, current| {
            patch.launch_on_login = Some(!current.launch_on_login);
        })
        .await;
    });
}

/// Common path for the two boolean tray toggles. Persists + refreshes tray + emits the
/// `snapshot-changed` event so the renderer's switches stay in sync.
async fn toggle_setting<F>(app: &AppHandle<Wry>, build_patch: F)
where
    F: FnOnce(&mut AppSettingsPatch, &crate::contracts::AppSettings),
{
    let Some(runtime) = app.try_state::<AppRuntime>() else {
        return;
    };
    let current = runtime.settings_snapshot();
    let mut patch = AppSettingsPatch::default();
    build_patch(&mut patch, &current);

    let mut merged = current;
    if let Some(v) = patch.keep_screen_on {
        merged.keep_screen_on = v;
    }
    if let Some(v) = patch.launch_on_login {
        merged.launch_on_login = v;
    }

    if settings_store::save(app, &merged).is_err() {
        return;
    }
    let _ = runtime.replace_settings(merged.clone());
    runtime.boundaries.sync_settings(&merged);

    if let Ok(host) = runtime.backend.get_snapshot().await {
        let snap = runtime.compose_app_snapshot(host.clone());
        crate::events::emit_snapshot(app, &snap);
        refresh(&snap);
        runtime.boundaries.handle_snapshot(&host, &merged).await;
    }
}
