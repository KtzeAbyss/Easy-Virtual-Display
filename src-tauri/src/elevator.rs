//! UAC-style elevation boundary. Mirrors `src/main/admin.ts`.
//!
//! Windows: writes three temp files (JSON config + elevated PS1 + wrapper PS1), launches a
//! user-context `powershell.exe` that `Start-Process -Verb RunAs`-es the elevated PS1.
//! The elevated PS1 reads the config, spawns the actual host command capturing
//! stdout/stderr to temp files, and exits with the host's exit code. The outer wrapper
//! captures shell-level errors (notably UAC cancellation → exit code 1223).
//!
//! Non-Windows: returns `driver_error` "elevation not supported" — Phase 5 is Windows-first
//! per the migration spec; macOS gets its own elevator later (e.g. `osascript` / SMJobBless).

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use tokio::fs;

use crate::errors::{EasyVirtualDisplayError, EasyVirtualDisplayErrorCode};

pub const ADMIN_CANCELLED_EXIT_CODE: i32 = 1223;
pub const ELEVATION_WRAPPER_FAILURE_EXIT_CODE: i32 = 1;

#[derive(Debug, Clone)]
pub struct ElevatedCommand {
    pub file_path: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ElevatedResult {
    pub exit_code: i32,
    /// Captured stdout from the inner process.
    pub stdout: String,
    /// Stderr from the inner process, falling back to shell-level stderr when the inner
    /// process didn't run.
    pub stderr: String,
}

pub type ElevatedFuture =
    Pin<Box<dyn Future<Output = Result<ElevatedResult, EasyVirtualDisplayError>> + Send>>;

pub trait Elevator: Send + Sync {
    fn run_elevated(&self, command: ElevatedCommand) -> ElevatedFuture;
}

pub fn create() -> Arc<dyn Elevator> {
    #[cfg(target_os = "windows")]
    {
        Arc::new(windows_impl::PowerShellElevator)
    }
    #[cfg(not(target_os = "windows"))]
    {
        Arc::new(noop_impl::Noop)
    }
}

/// Reads stdout/stderr temp files written by the elevated process. Missing file → empty.
pub(crate) async fn read_optional(path: &PathBuf) -> String {
    fs::read_to_string(path).await.unwrap_or_default()
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::process::Stdio;

    use tokio::io::AsyncReadExt;
    use tokio::process::Command;

    use super::*;

    const POWERSHELL: &str = "powershell.exe";

    pub struct PowerShellElevator;

    impl super::Elevator for PowerShellElevator {
        fn run_elevated(&self, command: ElevatedCommand) -> ElevatedFuture {
            Box::pin(run_elevated_impl(command))
        }
    }

    struct TempPaths {
        config: PathBuf,
        elevated_script: PathBuf,
        wrapper_script: PathBuf,
        stdout: PathBuf,
        stderr: PathBuf,
    }

    impl TempPaths {
        fn new() -> Self {
            let dir = std::env::temp_dir();
            let prefix = "easy-virtual-display-admin";
            Self {
                config: dir.join(format!("{prefix}-{}.json", uuid::Uuid::new_v4())),
                elevated_script: dir.join(format!("{prefix}-{}.ps1", uuid::Uuid::new_v4())),
                wrapper_script: dir.join(format!("{prefix}-{}.ps1", uuid::Uuid::new_v4())),
                stdout: dir.join(format!("{prefix}-{}.stdout.log", uuid::Uuid::new_v4())),
                stderr: dir.join(format!("{prefix}-{}.stderr.log", uuid::Uuid::new_v4())),
            }
        }

        async fn cleanup(&self) {
            let _ = tokio::join!(
                fs::remove_file(&self.config),
                fs::remove_file(&self.elevated_script),
                fs::remove_file(&self.wrapper_script),
                fs::remove_file(&self.stdout),
                fs::remove_file(&self.stderr),
            );
        }
    }

    async fn run_elevated_impl(
        command: ElevatedCommand,
    ) -> Result<ElevatedResult, EasyVirtualDisplayError> {
        let paths = TempPaths::new();

        let config = serde_json::json!({
            "filePath": command.file_path,
            "args": command.args,
            "cwd": command.cwd.to_string_lossy(),
            "stdoutPath": paths.stdout.to_string_lossy(),
            "stderrPath": paths.stderr.to_string_lossy(),
            "env": command.env,
        });

        if let Err(err) = write_three_scripts(&paths, &config).await {
            paths.cleanup().await;
            return Err(err);
        }

        let result = spawn_wrapper(&paths.wrapper_script).await;
        let exit_code = result.as_ref().map(|r| r.0).unwrap_or(ELEVATION_WRAPPER_FAILURE_EXIT_CODE);
        let shell_stderr = result.as_ref().map(|r| r.1.clone()).unwrap_or_default();
        let spawn_err = result.err();

        let stdout = read_optional(&paths.stdout).await;
        let stderr_file = read_optional(&paths.stderr).await;
        paths.cleanup().await;

        if let Some(err) = spawn_err {
            return Err(EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::DriverError,
                format!("Failed to spawn elevation wrapper: {err}"),
            ));
        }

        let stderr_combined = if stderr_file.trim().is_empty() {
            shell_stderr.trim().to_string()
        } else {
            stderr_file.trim().to_string()
        };

        Ok(ElevatedResult {
            exit_code,
            stdout: stdout.trim().to_string(),
            stderr: stderr_combined,
        })
    }

    async fn write_three_scripts(
        paths: &TempPaths,
        config: &serde_json::Value,
    ) -> Result<(), EasyVirtualDisplayError> {
        let elevated = build_elevated_script(&paths.config);
        let wrapper = build_wrapper_script(&paths.elevated_script);
        let config_bytes = serde_json::to_vec(config).expect("config json must encode");

        let r1 = fs::write(&paths.config, config_bytes).await;
        let r2 = fs::write(&paths.elevated_script, elevated).await;
        let r3 = fs::write(&paths.wrapper_script, wrapper).await;
        r1.and(r2).and(r3).map_err(|e| {
            EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::DriverError,
                format!("Failed to write elevation temp files: {e}"),
            )
        })
    }

    async fn spawn_wrapper(wrapper_script: &PathBuf) -> std::io::Result<(i32, String)> {
        let mut child = Command::new(POWERSHELL)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                wrapper_script.to_string_lossy().as_ref(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stderr_buf = String::new();
        if let Some(mut stderr) = child.stderr.take() {
            let _ = stderr.read_to_string(&mut stderr_buf).await;
        }
        // Drain stdout so the pipe doesn't fill.
        if let Some(mut stdout) = child.stdout.take() {
            let mut sink = String::new();
            let _ = stdout.read_to_string(&mut sink).await;
        }
        let status = child.wait().await?;
        let exit_code = status.code().unwrap_or(ELEVATION_WRAPPER_FAILURE_EXIT_CODE);
        Ok((exit_code, stderr_buf))
    }

    fn build_elevated_script(config_path: &PathBuf) -> String {
        let p = config_path.to_string_lossy();
        format!(
            "$ErrorActionPreference = 'Stop'\n\
             $config = Get-Content -LiteralPath '{p}' -Raw | ConvertFrom-Json\n\
             function Quote-WindowsArgument {{\n\
               param([string]$Value)\n\
               if ($null -eq $Value) {{ return '\"\"' }}\n\
               if ($Value -notmatch '[\\s\"]') {{ return $Value }}\n\
               $escaped = $Value -replace '(\\\\*)\"', '$1$1\\\"'\n\
               $escaped = $escaped -replace '(\\\\+)$', '$1$1'\n\
               return '\"' + $escaped + '\"'\n\
             }}\n\
             try {{\n\
               $psi = [System.Diagnostics.ProcessStartInfo]::new()\n\
               $psi.FileName = $config.filePath\n\
               $psi.WorkingDirectory = $config.cwd\n\
               $psi.UseShellExecute = $false\n\
               $psi.RedirectStandardOutput = $true\n\
               $psi.RedirectStandardError = $true\n\
               $psi.CreateNoWindow = $true\n\
               $psi.Arguments = [string]::Join(' ', ($config.args | ForEach-Object {{ Quote-WindowsArgument $_ }}))\n\
               foreach ($prop in $config.env.PSObject.Properties) {{\n\
                 $psi.EnvironmentVariables[$prop.Name] = [string]$prop.Value\n\
               }}\n\
               $process = [System.Diagnostics.Process]::Start($psi)\n\
               $stdoutTask = $process.StandardOutput.ReadToEndAsync()\n\
               $stderrTask = $process.StandardError.ReadToEndAsync()\n\
               $process.WaitForExit()\n\
               $stdout = $stdoutTask.GetAwaiter().GetResult()\n\
               $stderr = $stderrTask.GetAwaiter().GetResult()\n\
               [System.IO.File]::WriteAllText($config.stdoutPath, $stdout, [System.Text.Encoding]::UTF8)\n\
               [System.IO.File]::WriteAllText($config.stderrPath, $stderr, [System.Text.Encoding]::UTF8)\n\
               exit $process.ExitCode\n\
             }}\n\
             catch {{\n\
               $_.Exception.Message | Set-Content -LiteralPath $config.stderrPath -Encoding utf8\n\
               exit {ELEVATION_WRAPPER_FAILURE_EXIT_CODE}\n\
             }}\n"
        )
    }

    fn build_wrapper_script(elevated_script: &PathBuf) -> String {
        let p = elevated_script.to_string_lossy();
        format!(
            "$ErrorActionPreference = 'Stop'\n\
             try {{\n\
               $process = Start-Process -FilePath '{POWERSHELL}' -ArgumentList @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', '{p}') -Verb RunAs -WindowStyle Hidden -Wait -PassThru\n\
               exit $process.ExitCode\n\
             }}\n\
             catch {{\n\
               [Console]::Error.WriteLine($_.Exception.Message)\n\
               if ($_.Exception.Message -match 'canceled by the user|cancelled by the user|operation was canceled|operation was cancelled') {{\n\
                 exit {ADMIN_CANCELLED_EXIT_CODE}\n\
               }}\n\
               exit {ELEVATION_WRAPPER_FAILURE_EXIT_CODE}\n\
             }}\n"
        )
    }
}

#[cfg(not(target_os = "windows"))]
mod noop_impl {
    use super::*;

    pub struct Noop;

    impl super::Elevator for Noop {
        fn run_elevated(&self, _command: ElevatedCommand) -> ElevatedFuture {
            Box::pin(async {
                Err(EasyVirtualDisplayError::new(
                    EasyVirtualDisplayErrorCode::DriverError,
                    "UAC elevation is only implemented for Windows.",
                ))
            })
        }
    }
}
