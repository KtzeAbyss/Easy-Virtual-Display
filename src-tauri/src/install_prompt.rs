//! First-run nudge to install the bundled Parsec virtual display driver, mirroring
//! `src/main/app-driver.ts:maybePromptToInstallDriver`.
//!
//! Latch semantics:
//!   - host status anything other than `not_installed` → reset the latch so a *future*
//!     uninstall (e.g. user removed the driver from Windows Settings) re-prompts;
//!   - host status `not_installed` → claim the latch atomically; only the winning caller
//!     shows the dialog. Confirming routes straight into `drivers::install_driver` (same
//!     flow the renderer's "Install Driver" button uses).

use tauri::{AppHandle, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::app_state::AppRuntime;
use crate::contracts::{DriverStatus, EffectiveLanguage, HostSnapshot};
use crate::drivers;
use crate::shell_locales::t_common;

pub async fn maybe_prompt_to_install_driver(
    app: &AppHandle<Wry>,
    runtime: &AppRuntime,
    host: &HostSnapshot,
) {
    if host.status != DriverStatus::NotInstalled {
        runtime.set_install_prompt_shown(false);
        return;
    }
    if !runtime.try_claim_install_prompt() {
        return;
    }

    let lang = runtime.effective_language();
    if !show_confirm(app, lang).await {
        return;
    }

    // Errors here are surfaced to the user via the renderer's own driver-state UI on the
    // next broadcast (e.g. UAC cancelled → driver remains not_installed, status text
    // updates). Swallowing matches the TS behavior at app-driver.ts:128.
    let _ = drivers::install_driver(app, runtime).await;
}

async fn show_confirm(app: &AppHandle<Wry>, lang: EffectiveLanguage) -> bool {
    let title = t_common(lang, "install_driver_title").to_string();
    // Tauri's dialog has no separate "detail" pane; stack message + detail like quit.rs.
    let body = format!(
        "{}\n\n{}",
        t_common(lang, "install_driver_message"),
        t_common(lang, "install_driver_detail")
    );
    let confirm_label = t_common(lang, "install_driver_action").to_string();
    let cancel_label = t_common(lang, "cancel").to_string();

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .message(body)
        .title(title)
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(
            confirm_label,
            cancel_label,
        ))
        .show(move |confirmed| {
            let _ = tx.send(confirmed);
        });

    rx.await.unwrap_or(false)
}
