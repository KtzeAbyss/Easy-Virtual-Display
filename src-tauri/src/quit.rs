//! Graceful quit flow. Mirrors `src/main/app-quit.ts`.
//!
//! If any virtual displays are active, ask the user to confirm via the system dialog
//! before removing them and stopping the host. Idempotent: a second invocation while the
//! first is in flight is ignored.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::app_state::AppRuntime;
use crate::contracts::EffectiveLanguage;
use crate::shell_locales::{format_template, t_tray};

static QUIT_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

pub fn request_quit(app: AppHandle<Wry>) {
    if QUIT_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    tauri::async_runtime::spawn(async move {
        run_quit_flow(app).await;
        QUIT_IN_FLIGHT.store(false, Ordering::SeqCst);
    });
}

async fn run_quit_flow(app: AppHandle<Wry>) {
    let Some(runtime) = app.try_state::<AppRuntime>() else {
        app.exit(0);
        return;
    };

    // Use the cached snapshot — the renderer already drives the source of truth via
    // events; we don't want to block quit on a refresh round-trip.
    let snapshot = match runtime.backend.get_snapshot().await {
        Ok(s) => s,
        Err(_) => {
            app.exit(0);
            return;
        }
    };

    let active_count = snapshot.displays.iter().filter(|d| d.active).count();
    if active_count > 0 {
        let lang = runtime.effective_language();
        let confirmed = confirm_quit(&app, lang, active_count).await;
        if !confirmed {
            return;
        }
        let _ = runtime.backend.remove_all_displays().await;
    }

    let _ = runtime.backend.stop().await;
    app.exit(0);
}

async fn confirm_quit(app: &AppHandle<Wry>, lang: EffectiveLanguage, count: usize) -> bool {
    let detail_key = if count == 1 {
        "quit_detail"
    } else {
        "quit_detail_plural"
    };
    let detail = format_template(t_tray(lang, detail_key), &[("count", &count.to_string())]);
    // Tauri's dialog has no separate detail field; stack message + detail in the body.
    let body = format!("{}\n\n{}", t_tray(lang, "quit_message"), detail);

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .message(body)
        .title(t_tray(lang, "quit_title"))
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            t_tray(lang, "quit_button").to_string(),
            t_tray(lang, "cancel").to_string(),
        ))
        .show(move |confirmed| {
            let _ = tx.send(confirmed);
        });

    rx.await.unwrap_or(false)
}
