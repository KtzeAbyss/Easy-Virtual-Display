use std::collections::HashMap;
use std::path::PathBuf;

use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::contracts::ApplyAdminConfigInput;

/// All the pieces a `Command` needs to be spawned. We construct it from the resolver
/// rather than from the call site so the platform-specific knowledge (resource layout,
/// dev-vs-packaged branching, dotnet env vars) stays in this single module.
#[derive(Debug, Clone)]
pub struct HostCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
}

/// Root of the repo as seen at compile time. Used only when the packaged resource is
/// absent (i.e., dev mode). `CARGO_MANIFEST_DIR` is `src-tauri/`, so the repo root is its
/// parent.
fn dev_project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri must have a parent (the repo root)")
        .to_path_buf()
}

fn dotnet_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    env.insert("DOTNET_CLI_TELEMETRY_OPTOUT".to_string(), "1".to_string());
    env.insert("DOTNET_NOLOGO".to_string(), "1".to_string());
    env
}

fn packaged_host_exe(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .resolve(
            "native/EasyVirtualDisplay.Host.exe",
            BaseDirectory::Resource,
        )
        .ok()
        .filter(|p| p.exists())
}

fn packaged_driver_installer(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .resolve(
            "vendor/parsec-vdd/parsec-vdd-0.45.0.0.exe",
            BaseDirectory::Resource,
        )
        .ok()
        .filter(|p| p.exists())
}

fn dev_csproj() -> PathBuf {
    dev_project_root()
        .join("native")
        .join("EasyVirtualDisplay.Host")
        .join("EasyVirtualDisplay.Host.csproj")
}

fn dev_driver_installer() -> PathBuf {
    dev_project_root()
        .join("vendor")
        .join("parsec-vdd")
        .join("parsec-vdd-0.45.0.0.exe")
}

/// Wrap a list of "args to the host CLI" into either a packaged-exe invocation or a
/// `dotnet run --project ... -- ...` invocation, keeping cwd + env consistent.
fn wrap_host_invocation(app: &AppHandle, host_args: Vec<String>) -> HostCommand {
    if let Some(exe) = packaged_host_exe(app) {
        let cwd = exe
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(dev_project_root);
        return HostCommand {
            program: exe.to_string_lossy().into_owned(),
            args: host_args,
            cwd,
            env: HashMap::new(),
        };
    }

    let mut args = vec![
        "run".to_string(),
        "--project".to_string(),
        dev_csproj().to_string_lossy().into_owned(),
        "--".to_string(),
    ];
    args.extend(host_args);

    HostCommand {
        program: "dotnet".to_string(),
        args,
        cwd: dev_project_root(),
        env: dotnet_env(),
    }
}

pub fn resolve_stdio_command(app: &AppHandle) -> HostCommand {
    wrap_host_invocation(app, vec!["--stdio".to_string()])
}

pub fn resolve_admin_command(app: &AppHandle, input: &ApplyAdminConfigInput) -> HostCommand {
    let parent_gpu_str = match input.parent_gpu {
        crate::contracts::ParentGpu::Auto => "auto",
        crate::contracts::ParentGpu::Nvidia => "nvidia",
        crate::contracts::ParentGpu::Amd => "amd",
    };

    let modes_json = serde_json::to_string(&input.custom_modes).unwrap_or_else(|_| "[]".into());

    wrap_host_invocation(
        app,
        vec![
            "apply-admin-config".to_string(),
            "--modes".to_string(),
            modes_json,
            "--parent-gpu".to_string(),
            parent_gpu_str.to_string(),
        ],
    )
}

pub fn resolve_install_driver_command(app: &AppHandle) -> (HostCommand, PathBuf) {
    let installer = packaged_driver_installer(app).unwrap_or_else(dev_driver_installer);
    let cmd = wrap_host_invocation(
        app,
        vec![
            "install-driver".to_string(),
            "--installer-path".to_string(),
            installer.to_string_lossy().into_owned(),
        ],
    );
    (cmd, installer)
}

pub fn resolve_uninstall_driver_command(app: &AppHandle) -> HostCommand {
    wrap_host_invocation(app, vec!["uninstall-driver".to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_project_root_points_to_repo_root() {
        let root = dev_project_root();
        assert!(root.join("native").exists(), "{root:?} missing native/");
        assert!(root.join("src").exists(), "{root:?} missing src/");
    }

    #[test]
    fn dev_csproj_path_exists_on_disk() {
        assert!(dev_csproj().exists());
    }
}
