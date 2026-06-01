use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::oneshot;

use crate::contracts::HostSnapshot;
use crate::errors::{
    parse_rpc_error, EasyVirtualDisplayError, EasyVirtualDisplayErrorCode,
};

use super::stderr_ring::StderrRing;

pub type SnapshotListener = Arc<dyn Fn(&HostSnapshot) + Send + Sync>;
pub type RpcResult = Result<Value, EasyVirtualDisplayError>;
pub type PendingSender = oneshot::Sender<RpcResult>;

/// Inner state machine for the JSON-RPC backend. Designed so every state transition is a
/// pure synchronous method that takes `&mut self` and can be driven from a unit test
/// without spawning a real subprocess. The five critical behaviors from §3.2 of the
/// migration spec live here.
pub struct BackendState {
    pub snapshot: HostSnapshot,
    initial_snapshot: HostSnapshot,
    pub next_request_id: i64,
    pub pending: HashMap<i64, PendingSender>,
    pub active_session_id: u64,
    pub stderr_ring: StderrRing,
    listeners: HashMap<u64, SnapshotListener>,
    next_listener_id: u64,
}

const SNAPSHOT_NOTIFICATION_METHOD: &str = "host.snapshotChanged";
const STDERR_RING_CAPACITY: usize = 50;

impl BackendState {
    pub fn new(initial_snapshot: HostSnapshot) -> Self {
        Self {
            snapshot: initial_snapshot.clone(),
            initial_snapshot,
            next_request_id: 1,
            pending: HashMap::new(),
            active_session_id: 0,
            stderr_ring: StderrRing::new(STDERR_RING_CAPACITY),
            listeners: HashMap::new(),
            next_listener_id: 1,
        }
    }

    /// Bump the session id when (re)starting; any stale messages tagged with the old id
    /// will be ignored. Resets snapshot to the initial placeholder and clears stderr.
    pub fn begin_session(&mut self) -> u64 {
        self.active_session_id = self.active_session_id.wrapping_add(1);
        if self.active_session_id == 0 {
            self.active_session_id = 1; // skip 0 so 0 always means "not started"
        }
        self.snapshot = self.initial_snapshot.clone();
        self.stderr_ring.clear();
        self.active_session_id
    }

    pub fn next_request_id(&mut self) -> i64 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        if self.next_request_id <= 0 {
            self.next_request_id = 1;
        }
        id
    }

    pub fn add_pending(&mut self, id: i64, tx: PendingSender) {
        self.pending.insert(id, tx);
    }

    pub fn remove_pending(&mut self, id: i64) -> Option<PendingSender> {
        self.pending.remove(&id)
    }

    pub fn subscribe(&mut self, listener: SnapshotListener) -> u64 {
        let id = self.next_listener_id;
        self.next_listener_id = self.next_listener_id.wrapping_add(1);
        self.listeners.insert(id, listener);
        id
    }

    pub fn unsubscribe(&mut self, id: u64) {
        self.listeners.remove(&id);
    }

    pub fn listener_handles(&self) -> Vec<SnapshotListener> {
        self.listeners.values().cloned().collect()
    }

    /// Process a single stdout line from the host. If a fresh snapshot was applied, the
    /// new snapshot is returned so the caller can notify listeners (outside the lock).
    pub fn process_stdout_line(&mut self, session_id: u64, line: &str) -> Option<HostSnapshot> {
        if session_id != self.active_session_id {
            return None;
        }
        if line.trim().is_empty() {
            return None;
        }

        let payload: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                // Mirrors `sidecar-listener.ts`: invalid JSON ⇒ reject every pending
                // request with native_host_unavailable + the offending payload.
                let err = EasyVirtualDisplayError::new(
                    EasyVirtualDisplayErrorCode::NativeHostUnavailable,
                    "The native host emitted invalid JSON.",
                )
                .with_detail("payload", Value::String(line.to_string()));
                self.reject_all_pending(err);
                return None;
            }
        };

        // Notifications carry no `id`. Currently only host.snapshotChanged is defined.
        if payload
            .get("method")
            .and_then(|v| v.as_str())
            .map(|s| s == SNAPSHOT_NOTIFICATION_METHOD)
            .unwrap_or(false)
        {
            if let Some(params) = payload.get("params") {
                if let Ok(snap) = serde_json::from_value::<HostSnapshot>(params.clone()) {
                    return self.apply_snapshot(snap);
                }
            }
            return None;
        }

        // Otherwise it's a response. Match by id.
        let id = match payload.get("id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return None,
        };

        let Some(tx) = self.pending.remove(&id) else {
            return None;
        };

        if let Some(err_payload) = payload.get("error") {
            let _ = tx.send(Err(parse_rpc_error(err_payload)));
        } else if let Some(result) = payload.get("result") {
            let _ = tx.send(Ok(result.clone()));
        } else {
            let _ = tx.send(Ok(Value::Null));
        }
        None
    }

    pub fn process_stderr_line(&mut self, session_id: u64, line: String) {
        if session_id != self.active_session_id {
            return;
        }
        self.stderr_ring.push(line);
    }

    /// Apply a snapshot through the revision gate. Only strictly newer snapshots win.
    pub fn apply_snapshot(&mut self, next: HostSnapshot) -> Option<HostSnapshot> {
        if next.revision <= self.snapshot.revision {
            return None;
        }
        self.snapshot = next.clone();
        Some(next)
    }

    /// Reject every pending request with the given error. The drain takes ownership of the
    /// oneshot senders so dropped receivers don't keep them alive.
    pub fn reject_all_pending(&mut self, err: EasyVirtualDisplayError) {
        let pending: Vec<_> = self.pending.drain().collect();
        for (_id, tx) in pending {
            let _ = tx.send(Err(err.clone()));
        }
    }

    /// Convenience constructor for the "host process exited" rejection path.
    pub fn make_exit_error(&self, code: Option<i32>, signal: Option<String>) -> EasyVirtualDisplayError {
        let mut details = crate::contracts::ErrorDetails::new();
        if let Some(code) = code {
            details.insert("code".into(), Value::Number(code.into()));
        }
        if let Some(signal) = signal {
            details.insert("signal".into(), Value::String(signal));
        }
        let stderr = self.stderr_ring.joined();
        if !stderr.is_empty() {
            details.insert("stderr".into(), Value::String(stderr));
        }
        EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::NativeHostUnavailable,
            "The native host exited unexpectedly.",
        )
        .with_details(details)
    }

    pub fn make_stopped_error() -> EasyVirtualDisplayError {
        EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::NativeHostUnavailable,
            "The native host was stopped.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{empty_host_snapshot, DriverStatus, HostSnapshot, ParentGpu};

    fn make_snapshot(revision: i64, driver_version: &str) -> HostSnapshot {
        let mut s = empty_host_snapshot();
        s.revision = revision;
        s.driver_version = driver_version.into();
        s.status = DriverStatus::Ok;
        s.parent_gpu = ParentGpu::Auto;
        s
    }

    fn snapshot_notification_line(snap: &HostSnapshot) -> String {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "host.snapshotChanged",
            "params": snap,
        });
        msg.to_string()
    }

    fn ok_response(id: i64, result: serde_json::Value) -> String {
        serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string()
    }

    fn err_response(id: i64, code: &str, message: &str) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32000,
                "message": message,
                "data": { "code": code, "message": message }
            }
        })
        .to_string()
    }

    #[test]
    fn revision_gate_drops_equal_and_older_snapshots() {
        let mut state = BackendState::new(empty_host_snapshot());
        let session = state.begin_session();

        let s5 = make_snapshot(5, "v5");
        let applied = state.process_stdout_line(session, &snapshot_notification_line(&s5));
        assert!(applied.is_some());
        assert_eq!(state.snapshot.revision, 5);

        let s_equal = make_snapshot(5, "vEqual");
        assert!(state
            .process_stdout_line(session, &snapshot_notification_line(&s_equal))
            .is_none());
        assert_eq!(state.snapshot.driver_version, "v5");

        let s_older = make_snapshot(3, "vOlder");
        assert!(state
            .process_stdout_line(session, &snapshot_notification_line(&s_older))
            .is_none());
        assert_eq!(state.snapshot.revision, 5);

        let s7 = make_snapshot(7, "v7");
        assert!(state
            .process_stdout_line(session, &snapshot_notification_line(&s7))
            .is_some());
        assert_eq!(state.snapshot.driver_version, "v7");
    }

    #[test]
    fn session_id_drops_stale_messages_and_pending() {
        let mut state = BackendState::new(empty_host_snapshot());
        let s1 = state.begin_session();
        let s2 = state.begin_session();
        assert_ne!(s1, s2);

        // A snapshot from the old session is dropped.
        let snap = make_snapshot(10, "from-old-session");
        assert!(state
            .process_stdout_line(s1, &snapshot_notification_line(&snap))
            .is_none());
        assert_eq!(state.snapshot.revision, 0); // unchanged

        // A response from the old session is also dropped — note add_pending/remove_pending
        // is keyed by id, but process_stdout_line short-circuits on session mismatch.
        let (tx, mut rx) = oneshot::channel::<RpcResult>();
        state.add_pending(42, tx);
        let _ = state.process_stdout_line(
            s1,
            &ok_response(42, serde_json::Value::Null),
        );
        assert!(rx.try_recv().is_err()); // still pending
        assert!(state.pending.contains_key(&42));
    }

    #[test]
    fn matching_response_resolves_pending() {
        let mut state = BackendState::new(empty_host_snapshot());
        let session = state.begin_session();
        let (tx, mut rx) = oneshot::channel::<RpcResult>();
        let id = state.next_request_id();
        state.add_pending(id, tx);

        state.process_stdout_line(session, &ok_response(id, serde_json::json!({"ok": true})));

        let resolved = rx.try_recv().expect("must resolve").unwrap();
        assert_eq!(resolved, serde_json::json!({"ok": true}));
    }

    #[test]
    fn rpc_error_maps_to_easy_virtual_display_error() {
        let mut state = BackendState::new(empty_host_snapshot());
        let session = state.begin_session();
        let (tx, mut rx) = oneshot::channel::<RpcResult>();
        let id = state.next_request_id();
        state.add_pending(id, tx);

        state.process_stdout_line(
            session,
            &err_response(id, "driver_not_installed", "Driver missing"),
        );

        let err = rx.try_recv().expect("must resolve").unwrap_err();
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::DriverNotInstalled);
        assert_eq!(err.message, "Driver missing");
    }

    #[test]
    fn invalid_json_rejects_all_pending() {
        let mut state = BackendState::new(empty_host_snapshot());
        let session = state.begin_session();
        let (tx1, rx1) = oneshot::channel::<RpcResult>();
        let (tx2, rx2) = oneshot::channel::<RpcResult>();
        let mut rx1 = rx1;
        let mut rx2 = rx2;
        state.add_pending(1, tx1);
        state.add_pending(2, tx2);

        state.process_stdout_line(session, "this is not json {{{{");

        for rx in [&mut rx1, &mut rx2] {
            let err = rx.try_recv().expect("must resolve").unwrap_err();
            assert_eq!(err.code, EasyVirtualDisplayErrorCode::NativeHostUnavailable);
        }
        assert!(state.pending.is_empty());
    }

    #[test]
    fn reject_all_pending_clears_map() {
        let mut state = BackendState::new(empty_host_snapshot());
        let _ = state.begin_session();
        let (tx, mut rx) = oneshot::channel::<RpcResult>();
        state.add_pending(1, tx);

        state.reject_all_pending(EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::NativeHostUnavailable,
            "down",
        ));
        assert!(state.pending.is_empty());
        let err = rx.try_recv().unwrap().unwrap_err();
        assert_eq!(err.message, "down");
    }

    #[test]
    fn stderr_lines_recorded_only_for_active_session() {
        let mut state = BackendState::new(empty_host_snapshot());
        let s1 = state.begin_session();
        state.process_stderr_line(s1, "first".into());
        state.process_stderr_line(s1, "second".into());
        assert_eq!(state.stderr_ring.snapshot(), vec!["first", "second"]);

        let _s2 = state.begin_session(); // session bumped, stderr cleared
        assert!(state.stderr_ring.is_empty());
        state.process_stderr_line(s1, "from-old-session".into());
        assert!(state.stderr_ring.is_empty());
    }

    #[test]
    fn exit_error_carries_stderr_and_exit_code() {
        let mut state = BackendState::new(empty_host_snapshot());
        let s1 = state.begin_session();
        state.process_stderr_line(s1, "boom".into());
        state.process_stderr_line(s1, "stack".into());

        let err = state.make_exit_error(Some(2), None);
        assert_eq!(err.code, EasyVirtualDisplayErrorCode::NativeHostUnavailable);
        let details = err.details.as_ref().expect("details");
        assert_eq!(details["code"], serde_json::json!(2));
        assert_eq!(details["stderr"], serde_json::json!("boom\nstack"));
    }

    #[test]
    fn subscribe_unsubscribe_lifecycle() {
        let mut state = BackendState::new(empty_host_snapshot());
        let session = state.begin_session();

        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter2 = counter.clone();
        let id = state.subscribe(Arc::new(move |_snap| {
            counter2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }));

        let snap = make_snapshot(1, "v1");
        let applied = state.process_stdout_line(session, &snapshot_notification_line(&snap));
        assert!(applied.is_some());

        // Listener handles are surfaced separately so the caller can emit outside the lock.
        for listener in state.listener_handles() {
            listener(&applied.clone().unwrap());
        }
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

        state.unsubscribe(id);
        assert!(state.listener_handles().is_empty());
    }
}
