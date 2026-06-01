//! Driver lifecycle flows. Each wraps the elevated host invocation with its
//! post-elevation choreography:
//!   install / uninstall  → stop+start backend, refresh snapshot, sync settings, broadcast
//!   applyAdminConfig     → poll snapshot for the new modes/parentGpu (max 2.5 s)

use std::time::{Duration, Instant};

use tauri::{AppHandle, Wry};
use tokio::time::sleep;

use crate::app_state::AppRuntime;
use crate::contracts::{canonical_modes_key, ApplyAdminConfigInput, HostSnapshot};
use crate::elevator::{ElevatedCommand, ElevatedResult, Elevator, ADMIN_CANCELLED_EXIT_CODE};
use crate::errors::{
    is_user_cancelled_elevation, parse_serialized_error, EasyVirtualDisplayError,
    EasyVirtualDisplayErrorCode,
};
use crate::events;
use crate::host::{
    resolve_admin_command, resolve_install_driver_command, resolve_uninstall_driver_command,
    HostCommand,
};
use crate::tray;

const CONFIG_APPLY_TIMEOUT: Duration = Duration::from_millis(2500);
const CONFIG_APPLY_POLL_INTERVAL: Duration = Duration::from_millis(250);

pub async fn install_driver(
    app: &AppHandle<Wry>,
    runtime: &AppRuntime,
) -> Result<(), EasyVirtualDisplayError> {
    let (host_command, installer_path) = resolve_install_driver_command(app);

    if tokio::fs::metadata(&installer_path).await.is_err() {
        return Err(EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::DriverInstallerMissing,
            "Bundled driver installer is missing.",
        ));
    }

    run_elevated_or_fail(
        runtime.elevator.as_ref(),
        host_command,
        EasyVirtualDisplayErrorCode::DriverError,
        "Failed to install the bundled driver.",
    )
    .await?;

    post_driver_change(app, runtime, true).await?;
    Ok(())
}

pub async fn uninstall_driver(
    app: &AppHandle<Wry>,
    runtime: &AppRuntime,
) -> Result<(), EasyVirtualDisplayError> {
    let host_command = resolve_uninstall_driver_command(app);

    run_elevated_or_fail(
        runtime.elevator.as_ref(),
        host_command,
        EasyVirtualDisplayErrorCode::DriverUninstallFailed,
        "Failed to uninstall the bundled driver.",
    )
    .await?;

    post_driver_change(app, runtime, false).await?;
    Ok(())
}

pub async fn apply_admin_config(
    app: &AppHandle<Wry>,
    runtime: &AppRuntime,
    input: ApplyAdminConfigInput,
) -> Result<(), EasyVirtualDisplayError> {
    let host_command = resolve_admin_command(app, &input);

    run_elevated_or_fail(
        runtime.elevator.as_ref(),
        host_command,
        EasyVirtualDisplayErrorCode::DriverError,
        "Failed to apply administrator configuration.",
    )
    .await?;

    wait_for_config_snapshot(runtime, &input).await?;

    let host = runtime.backend.get_snapshot().await?;
    let snap = runtime.compose_app_snapshot(host.clone());
    events::emit_snapshot(app, &snap);
    tray::refresh(&snap);
    Ok(())
}

/// Restart the backend and re-broadcast after a driver mutation. Mirrors the suffix of
/// `installBundledDriver` / `uninstallBundledDriver` in app-driver.ts.
async fn post_driver_change(
    app: &AppHandle<Wry>,
    runtime: &AppRuntime,
    just_installed: bool,
) -> Result<(), EasyVirtualDisplayError> {
    runtime.backend.stop().await?;
    runtime.backend.start().await?;
    let host = runtime.backend.get_snapshot().await?;

    runtime.set_install_prompt_shown(if just_installed {
        host.status != crate::contracts::DriverStatus::NotInstalled
    } else {
        true
    });

    let settings = runtime.settings_snapshot();
    runtime.boundaries.sync_settings(&settings);

    let snap = runtime.compose_app_snapshot(host.clone());
    events::emit_snapshot(app, &snap);
    tray::refresh(&snap);
    runtime.boundaries.handle_snapshot(&host, &settings).await;
    Ok(())
}

async fn run_elevated_or_fail(
    elevator: &dyn Elevator,
    host_command: HostCommand,
    fallback_code: EasyVirtualDisplayErrorCode,
    fallback_message: &str,
) -> Result<ElevatedResult, EasyVirtualDisplayError> {
    let result = elevator
        .run_elevated(ElevatedCommand {
            file_path: host_command.program,
            args: host_command.args,
            cwd: host_command.cwd,
            env: host_command.env,
        })
        .await?;

    if result.exit_code == 0 {
        return Ok(result);
    }

    if result.exit_code == ADMIN_CANCELLED_EXIT_CODE
        || is_user_cancelled_elevation(&result.stderr)
        || is_user_cancelled_elevation(&result.stdout)
    {
        return Err(EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::AdminCancelled,
            "Administrator approval was cancelled.",
        ));
    }

    let payload = if result.stderr.is_empty() {
        result.stdout.clone()
    } else {
        result.stderr.clone()
    };
    Err(parse_serialized_error(&payload, fallback_code, fallback_message))
}

async fn wait_for_config_snapshot(
    runtime: &AppRuntime,
    input: &ApplyAdminConfigInput,
) -> Result<(), EasyVirtualDisplayError> {
    let expected = canonical_modes_key(&input.custom_modes);
    let deadline = Instant::now() + CONFIG_APPLY_TIMEOUT;

    while Instant::now() < deadline {
        let snapshot: HostSnapshot = runtime.backend.refresh_snapshot().await?;
        if snapshot.parent_gpu == input.parent_gpu
            && canonical_modes_key(&snapshot.custom_modes) == expected
        {
            return Ok(());
        }
        sleep(CONFIG_APPLY_POLL_INTERVAL).await;
    }

    Err(EasyVirtualDisplayError::new(
        EasyVirtualDisplayErrorCode::ConfigApplyTimeout,
        "Configuration was sent but the change was not confirmed within the timeout window.",
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::elevator::ElevatedFuture;
    use crate::errors::EasyVirtualDisplayErrorCode;

    fn dummy_command() -> HostCommand {
        HostCommand {
            program: "test".into(),
            args: vec![],
            cwd: std::env::temp_dir(),
            env: Default::default(),
        }
    }

    struct StubElevator {
        results: Mutex<Vec<ElevatedResult>>,
        calls: AtomicU32,
    }

    impl StubElevator {
        fn new(results: Vec<ElevatedResult>) -> Arc<Self> {
            Arc::new(Self {
                results: Mutex::new(results),
                calls: AtomicU32::new(0),
            })
        }
    }

    impl Elevator for StubElevator {
        fn run_elevated(&self, _: ElevatedCommand) -> ElevatedFuture {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let next = self
                .results
                .lock()
                .unwrap()
                .pop()
                .unwrap_or(ElevatedResult {
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                });
            Box::pin(async move { Ok(next) })
        }
    }

    #[tokio::test]
    async fn run_elevated_returns_ok_on_zero_exit() {
        let stub = StubElevator::new(vec![ElevatedResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
        }]);
        let result = run_elevated_or_fail(
            stub.as_ref(),
            dummy_command(),
            EasyVirtualDisplayErrorCode::DriverError,
            "boom",
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_elevated_maps_uac_exit_code_to_admin_cancelled() {
        let stub = StubElevator::new(vec![ElevatedResult {
            exit_code: ADMIN_CANCELLED_EXIT_CODE,
            stdout: String::new(),
            stderr: String::new(),
        }]);
        let err = run_elevated_or_fail(
            stub.as_ref(),
            dummy_command(),
            EasyVirtualDisplayErrorCode::DriverError,
            "boom",
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::AdminCancelled);
    }

    #[tokio::test]
    async fn run_elevated_maps_user_cancelled_message_to_admin_cancelled() {
        let stub = StubElevator::new(vec![ElevatedResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "The operation was canceled by the user.".into(),
        }]);
        let err = run_elevated_or_fail(
            stub.as_ref(),
            dummy_command(),
            EasyVirtualDisplayErrorCode::DriverError,
            "boom",
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::AdminCancelled);
    }

    #[tokio::test]
    async fn run_elevated_lifts_serialized_error_from_stderr() {
        let stub = StubElevator::new(vec![ElevatedResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: r#"{"code":"driver_installer_missing","message":"no installer"}"#.into(),
        }]);
        let err = run_elevated_or_fail(
            stub.as_ref(),
            dummy_command(),
            EasyVirtualDisplayErrorCode::DriverError,
            "fallback",
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::DriverInstallerMissing);
        assert_eq!(err.message, "no installer");
    }

    #[tokio::test]
    async fn run_elevated_falls_back_when_stderr_is_unstructured() {
        let stub = StubElevator::new(vec![ElevatedResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "some opaque shell error".into(),
        }]);
        let err = run_elevated_or_fail(
            stub.as_ref(),
            dummy_command(),
            EasyVirtualDisplayErrorCode::DriverUninstallFailed,
            "fallback message",
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::DriverUninstallFailed);
        assert_eq!(err.message, "some opaque shell error");
    }
}
