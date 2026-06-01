mod resolver;
mod state;
mod stderr_ring;

pub use resolver::{
    resolve_admin_command, resolve_install_driver_command, resolve_stdio_command,
    resolve_uninstall_driver_command, HostCommand,
};
pub use state::BackendState;

use std::process::Stdio;
use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::{sleep, timeout, Duration};

use crate::contracts::{HostSnapshot, SetDisplayModeInput};
use crate::errors::{
    matches_dotnet_runtime_missing, EasyVirtualDisplayError, EasyVirtualDisplayErrorCode,
    DOTNET_RUNTIME_MISSING_MESSAGE,
};

const SNAPSHOT_RPC_METHOD: &str = "host.getSnapshot";
const ADD_DISPLAY_RPC_METHOD: &str = "host.addDisplay";
const REMOVE_DISPLAY_RPC_METHOD: &str = "host.removeDisplay";
const REMOVE_ALL_DISPLAYS_RPC_METHOD: &str = "host.removeAllDisplays";
const SET_DISPLAY_MODE_RPC_METHOD: &str = "host.setDisplayMode";

const STOP_TIMEOUT: Duration = Duration::from_secs(3);

struct WriterRequest {
    id: i64,
    bytes: Vec<u8>,
}

struct RunningHandles {
    writer_tx: mpsc::UnboundedSender<WriterRequest>,
    kill_signal: Option<oneshot::Sender<()>>,
    stopped_rx: Option<oneshot::Receiver<()>>,
    /// Session id this set of handles belongs to. If the backend has been re-started since,
    /// callers should treat these handles as stale.
    session_id: u64,
}

pub type CommandFactory = Arc<dyn Fn() -> HostCommand + Send + Sync>;
pub type SnapshotBroadcaster = Arc<dyn Fn(&HostSnapshot) + Send + Sync>;

/// Cross-platform JSON-RPC client around a stdio sidecar. Spawns the host process,
/// pumps requests/responses, and exposes the platform-neutral surface that maps to
/// §3.2 of the migration spec. The platform-specific bit (how to spawn the host) lives in
/// the `CommandFactory` closure — there's no Windows knowledge in this file.
pub struct StdioJsonRpcBackend {
    state: Arc<Mutex<BackendState>>,
    handles: Arc<Mutex<Option<RunningHandles>>>,
    command_factory: CommandFactory,
    /// Serializes concurrent start() callers so only one spawn happens at a time. The
    /// inner check then short-circuits the rest.
    start_lock: Arc<Mutex<()>>,
    /// Optional broadcaster invoked when a fresh snapshot is applied. Used by lib.rs to
    /// emit `snapshot-changed` to the renderer. Decoupled from `state.listeners` so the
    /// Tauri AppHandle dependency stays out of the testable state machine.
    snapshot_broadcaster: Arc<std::sync::Mutex<Option<SnapshotBroadcaster>>>,
}

impl StdioJsonRpcBackend {
    pub fn new(command_factory: CommandFactory, initial_snapshot: HostSnapshot) -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(BackendState::new(initial_snapshot))),
            handles: Arc::new(Mutex::new(None)),
            command_factory,
            start_lock: Arc::new(Mutex::new(())),
            snapshot_broadcaster: Arc::new(std::sync::Mutex::new(None)),
        })
    }

    pub fn set_snapshot_broadcaster(&self, broadcaster: SnapshotBroadcaster) {
        if let Ok(mut guard) = self.snapshot_broadcaster.lock() {
            *guard = Some(broadcaster);
        }
    }

    /// Idempotent start. Concurrent callers all await the same in-progress spawn.
    pub async fn start(self: &Arc<Self>) -> Result<(), EasyVirtualDisplayError> {
        // Serialize starts: if another caller is mid-spawn, wait until they finish, then
        // check whether the child is now running (likely yes ⇒ short-circuit).
        let _start_guard = self.start_lock.lock().await;

        {
            let handles = self.handles.lock().await;
            if handles.is_some() {
                return Ok(());
            }
        }

        let cmd = (self.command_factory)();
        let mut builder = Command::new(&cmd.program);
        builder
            .args(&cmd.args)
            .current_dir(&cmd.cwd)
            .envs(cmd.env.iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        // Host.exe is a .NET *console* subsystem binary; without CREATE_NO_WINDOW Windows
        // allocates a visible cmd window for it whenever the GUI Tauri parent has no console
        // to inherit. The packaged build is exactly that case — dev mode runs under
        // powershell so the issue is invisible there.
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            builder.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = builder.spawn().map_err(|e| {
            EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::NativeHostUnavailable,
                format!("Failed to spawn native host '{}': {e}", cmd.program),
            )
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| native_host_unavailable("child stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| native_host_unavailable("child stdout unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| native_host_unavailable("child stderr unavailable"))?;

        let session_id = {
            let mut state = self.state.lock().await;
            state.begin_session()
        };

        let (writer_tx, writer_rx) = mpsc::unbounded_channel();
        let (kill_tx, kill_rx) = oneshot::channel::<()>();
        let (stopped_tx, stopped_rx) = oneshot::channel::<()>();

        {
            let mut handles = self.handles.lock().await;
            *handles = Some(RunningHandles {
                writer_tx: writer_tx.clone(),
                kill_signal: Some(kill_tx),
                stopped_rx: Some(stopped_rx),
                session_id,
            });
        }

        tokio::spawn(writer_task(
            stdin,
            writer_rx,
            self.state.clone(),
            session_id,
        ));
        tokio::spawn(reader_task(
            stdout,
            self.state.clone(),
            self.snapshot_broadcaster.clone(),
            session_id,
        ));
        tokio::spawn(stderr_task(stderr, self.state.clone(), session_id));
        tokio::spawn(exit_task(
            child,
            kill_rx,
            stopped_tx,
            self.state.clone(),
            self.handles.clone(),
            session_id,
        ));

        // Block until the initial snapshot arrives. If it fails AND the stderr ring matches
        // a .NET-missing pattern, surface that more actionable error.
        match self
            .invoke_rpc::<HostSnapshot>(SNAPSHOT_RPC_METHOD, None)
            .await
        {
            Ok(initial) => {
                let mut state = self.state.lock().await;
                state.apply_snapshot(initial);
                Ok(())
            }
            Err(err) => {
                let stderr_text = {
                    let state = self.state.lock().await;
                    state.stderr_ring.joined()
                };

                // Kill the child so it doesn't linger after a failed start.
                let _ = self.stop().await;

                if matches_dotnet_runtime_missing(&stderr_text)
                    || matches_dotnet_runtime_missing(&err.message)
                {
                    return Err(EasyVirtualDisplayError::new(
                        EasyVirtualDisplayErrorCode::DotnetRuntimeMissing,
                        DOTNET_RUNTIME_MISSING_MESSAGE,
                    )
                    .with_detail("stderr", Value::String(stderr_text)));
                }
                Err(err)
            }
        }
    }

    pub async fn stop(self: &Arc<Self>) -> Result<(), EasyVirtualDisplayError> {
        let (kill_signal, stopped_rx) = {
            let mut handles = self.handles.lock().await;
            match handles.as_mut() {
                Some(h) => (h.kill_signal.take(), h.stopped_rx.take()),
                None => return Ok(()),
            }
        };

        // Reject pending immediately so callers don't hang while we wait for the OS to
        // reap the child. The exit task will redundantly call reject_all_pending too —
        // both are idempotent since pending is drained.
        {
            let mut state = self.state.lock().await;
            state.reject_all_pending(BackendState::make_stopped_error());
        }

        if let Some(tx) = kill_signal {
            let _ = tx.send(());
        }
        if let Some(rx) = stopped_rx {
            let _ = timeout(STOP_TIMEOUT, rx).await;
        }
        Ok(())
    }

    /// Cached snapshot (no RPC). Ensures the backend has started.
    pub async fn get_snapshot(self: &Arc<Self>) -> Result<HostSnapshot, EasyVirtualDisplayError> {
        self.start().await?;
        let state = self.state.lock().await;
        Ok(state.snapshot.clone())
    }

    /// Forces an RPC to fetch the freshest snapshot, applies it through the revision gate.
    pub async fn refresh_snapshot(
        self: &Arc<Self>,
    ) -> Result<HostSnapshot, EasyVirtualDisplayError> {
        self.start().await?;
        let latest = self
            .invoke_rpc::<HostSnapshot>(SNAPSHOT_RPC_METHOD, None)
            .await?;
        let mut state = self.state.lock().await;
        if let Some(applied) = state.apply_snapshot(latest.clone()) {
            // The reader task didn't see this (we polled), so broadcast manually.
            let listeners = state.listener_handles();
            drop(state);
            if let Ok(broadcaster) = self.snapshot_broadcaster.lock() {
                if let Some(b) = broadcaster.as_ref() {
                    b(&applied);
                }
            }
            for listener in listeners {
                listener(&applied);
            }
            return Ok(applied);
        }
        Ok(latest)
    }

    pub async fn add_display(self: &Arc<Self>) -> Result<(), EasyVirtualDisplayError> {
        self.start().await?;
        self.invoke_rpc::<Value>(ADD_DISPLAY_RPC_METHOD, None).await?;
        Ok(())
    }

    pub async fn remove_display(
        self: &Arc<Self>,
        index: Option<i32>,
    ) -> Result<(), EasyVirtualDisplayError> {
        self.start().await?;
        let params = match index {
            Some(i) => Some(serde_json::json!({ "index": i })),
            None => Some(Value::Object(serde_json::Map::new())),
        };
        self.invoke_rpc::<Value>(REMOVE_DISPLAY_RPC_METHOD, params)
            .await?;
        Ok(())
    }

    pub async fn remove_all_displays(self: &Arc<Self>) -> Result<(), EasyVirtualDisplayError> {
        self.start().await?;
        self.invoke_rpc::<Value>(REMOVE_ALL_DISPLAYS_RPC_METHOD, None)
            .await?;
        Ok(())
    }

    pub async fn set_display_mode(
        self: &Arc<Self>,
        input: SetDisplayModeInput,
    ) -> Result<(), EasyVirtualDisplayError> {
        self.start().await?;
        let params = serde_json::to_value(input).map_err(|e| {
            EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::DriverError,
                format!("Failed to serialize SetDisplayModeInput: {e}"),
            )
        })?;
        self.invoke_rpc::<Value>(SET_DISPLAY_MODE_RPC_METHOD, Some(params))
            .await?;
        Ok(())
    }

    pub async fn subscribe<F>(self: &Arc<Self>, listener: F) -> u64
    where
        F: Fn(&HostSnapshot) + Send + Sync + 'static,
    {
        let mut state = self.state.lock().await;
        state.subscribe(Arc::new(listener))
    }

    pub async fn unsubscribe(self: &Arc<Self>, id: u64) {
        let mut state = self.state.lock().await;
        state.unsubscribe(id);
    }

    async fn invoke_rpc<T: serde::de::DeserializeOwned>(
        self: &Arc<Self>,
        method: &str,
        params: Option<Value>,
    ) -> Result<T, EasyVirtualDisplayError> {
        let (id, request_bytes, resp_rx) = {
            let mut state = self.state.lock().await;
            let id = state.next_request_id();
            let (tx, rx) = oneshot::channel();
            state.add_pending(id, tx);
            let mut body = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
            });
            if let Some(p) = params {
                body.as_object_mut().unwrap().insert("params".into(), p);
            }
            let mut bytes = serde_json::to_vec(&body).expect("json encode");
            bytes.push(b'\n');
            (id, bytes, rx)
        };

        // Send via writer channel. If the writer is gone (process never started or has
        // exited), reject immediately.
        let writer_tx = {
            let handles = self.handles.lock().await;
            handles.as_ref().map(|h| h.writer_tx.clone())
        };

        let Some(writer) = writer_tx else {
            let mut state = self.state.lock().await;
            state.remove_pending(id);
            return Err(EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::NativeHostUnavailable,
                "The native host is not running.",
            ));
        };

        if writer
            .send(WriterRequest {
                id,
                bytes: request_bytes,
            })
            .is_err()
        {
            let mut state = self.state.lock().await;
            state.remove_pending(id);
            return Err(EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::NativeHostUnavailable,
                "Failed to enqueue request to the native host.",
            ));
        }

        let raw = match resp_rx.await {
            Ok(Ok(value)) => value,
            Ok(Err(err)) => return Err(err),
            Err(_) => {
                return Err(EasyVirtualDisplayError::new(
                    EasyVirtualDisplayErrorCode::NativeHostUnavailable,
                    "The native host dropped the response channel.",
                ))
            }
        };

        serde_json::from_value::<T>(raw).map_err(|e| {
            EasyVirtualDisplayError::new(
                EasyVirtualDisplayErrorCode::DriverError,
                format!("RPC result decode failed for {method}: {e}"),
            )
        })
    }
}

fn native_host_unavailable(msg: &str) -> EasyVirtualDisplayError {
    EasyVirtualDisplayError::new(
        EasyVirtualDisplayErrorCode::NativeHostUnavailable,
        msg.to_string(),
    )
}

async fn writer_task(
    mut stdin: ChildStdin,
    mut rx: mpsc::UnboundedReceiver<WriterRequest>,
    state: Arc<Mutex<BackendState>>,
    session_id: u64,
) {
    while let Some(req) = rx.recv().await {
        let write_result = stdin.write_all(&req.bytes).await;
        let flush_result = match write_result {
            Ok(()) => stdin.flush().await,
            Err(e) => Err(e),
        };
        if let Err(err) = flush_result {
            // Mirror sidecar-rpc.ts: stdin write failure ⇒ this request gets rejected.
            let mut guard = state.lock().await;
            if guard.active_session_id == session_id {
                if let Some(tx) = guard.remove_pending(req.id) {
                    let _ = tx.send(Err(EasyVirtualDisplayError::new(
                        EasyVirtualDisplayErrorCode::NativeHostUnavailable,
                        format!("Failed to write to the native host process: {err}"),
                    )));
                }
            }
            // Once the pipe is broken, future writes will keep failing; let the loop
            // drain the channel and continue rejecting until exit_task takes over.
        }
    }
}

async fn reader_task(
    stdout: ChildStdout,
    state: Arc<Mutex<BackendState>>,
    broadcaster: Arc<std::sync::Mutex<Option<SnapshotBroadcaster>>>,
    session_id: u64,
) {
    let mut reader = BufReader::new(stdout);
    let mut buf = String::new();
    loop {
        buf.clear();
        match reader.read_line(&mut buf).await {
            Ok(0) => return, // EOF — exit_task will run cleanup
            Ok(_) => {
                let (applied_snapshot, listeners) = {
                    let mut guard = state.lock().await;
                    let applied = guard.process_stdout_line(session_id, buf.trim_end());
                    let listeners = if applied.is_some() {
                        guard.listener_handles()
                    } else {
                        Vec::new()
                    };
                    (applied, listeners)
                };
                if let Some(snap) = applied_snapshot {
                    if let Ok(guard) = broadcaster.lock() {
                        if let Some(b) = guard.as_ref() {
                            b(&snap);
                        }
                    }
                    for listener in listeners {
                        listener(&snap);
                    }
                }
            }
            Err(_) => return,
        }
    }
}

async fn stderr_task(
    stderr: ChildStderr,
    state: Arc<Mutex<BackendState>>,
    session_id: u64,
) {
    let mut reader = BufReader::new(stderr);
    let mut buf = String::new();
    loop {
        buf.clear();
        match reader.read_line(&mut buf).await {
            Ok(0) => return,
            Ok(_) => {
                let trimmed = buf.trim_end();
                if trimmed.is_empty() {
                    continue;
                }
                let mut guard = state.lock().await;
                guard.process_stderr_line(session_id, trimmed.to_string());
            }
            Err(_) => return,
        }
    }
}

async fn exit_task(
    mut child: Child,
    kill_rx: oneshot::Receiver<()>,
    stopped_tx: oneshot::Sender<()>,
    state: Arc<Mutex<BackendState>>,
    handles: Arc<Mutex<Option<RunningHandles>>>,
    session_id: u64,
) {
    let exit_status = tokio::select! {
        result = child.wait() => result,
        _ = kill_rx => {
            // Best-effort graceful: ask the child to die, then give it a moment.
            let _ = child.kill().await;
            // child.kill() also reaps via wait, but we wait again to be sure.
            sleep(Duration::from_millis(50)).await;
            child.wait().await
        }
    };

    let code = exit_status.ok().and_then(|s| s.code());

    let exit_err = {
        let state_guard = state.lock().await;
        state_guard.make_exit_error(code, None)
    };

    {
        let mut state_guard = state.lock().await;
        if state_guard.active_session_id == session_id {
            state_guard.reject_all_pending(exit_err);
        }
    }

    {
        let mut handles_guard = handles.lock().await;
        if let Some(h) = handles_guard.as_ref() {
            if h.session_id == session_id {
                *handles_guard = None;
            }
        }
    }

    let _ = stopped_tx.send(());
}

#[cfg(test)]
mod tests {
    //! Integration tests that drive a real .NET host subprocess. These rely on
    //! `npm run build:native` having published the host binary into
    //! `native/EasyVirtualDisplay.Host/bin/publish/` (or `dotnet` being on PATH so the
    //! resolver can fall back to `dotnet run`).

    use super::*;
    use crate::contracts::empty_host_snapshot;

    fn dev_command() -> HostCommand {
        let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let csproj = project_root.join("native/EasyVirtualDisplay.Host/EasyVirtualDisplay.Host.csproj");
        let mut env = std::collections::HashMap::new();
        env.insert("DOTNET_CLI_TELEMETRY_OPTOUT".into(), "1".into());
        env.insert("DOTNET_NOLOGO".into(), "1".into());
        HostCommand {
            program: "dotnet".into(),
            args: vec![
                "run".into(),
                "--no-build".into(),
                "--project".into(),
                csproj.to_string_lossy().into_owned(),
                "--".into(),
                "--stdio".into(),
            ],
            cwd: project_root,
            env,
        }
    }

    #[tokio::test]
    #[ignore = "requires `npm run build:native` first (slow; opt-in)"]
    async fn end_to_end_get_snapshot_against_real_dotnet_host() {
        let backend = StdioJsonRpcBackend::new(
            Arc::new(|| dev_command()),
            empty_host_snapshot(),
        );
        backend.start().await.expect("start should succeed");
        let snap = backend.get_snapshot().await.expect("get_snapshot");
        assert!(!snap.driver_version.is_empty());
        backend.stop().await.expect("stop should succeed");
    }
}
