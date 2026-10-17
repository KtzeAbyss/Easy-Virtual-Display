//! Fallback virtual display state machine.
//!
//! When the user enables `fallbackDisplay` and the machine has no active display output
//! and none of our virtual displays, add one after a short debounce. Transient failures
//! are retried up to `MAX_ATTEMPTS` in total; the final failure is reported so the UI can
//! show it. The attempt ends (and any reported error clears) as soon as a fallback is no
//! longer needed: an output appears, the driver leaves the `ok` state, or the setting is
//! turned off.
//!
//! Each such need is an episode with its own generation. A task from an earlier episode
//! checks the generation before acting, so it can neither add a display for nor report
//! into a newer one.
//!
//! Pure-Rust logic — no OS calls — so this module is cross-platform without cfg gates.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

use crate::contracts::{AppSettings, DriverStatus, HostSnapshot};
use crate::errors::{EasyVirtualDisplayError, EasyVirtualDisplayErrorCode};

const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(1000);
const RETRY_BACKOFF: Duration = Duration::from_millis(2000);
const MAX_ATTEMPTS: u32 = 3;

pub type AddDisplayCallback = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), EasyVirtualDisplayError>> + Send>>
        + Send
        + Sync,
>;

/// Receives the fallback error to show, or `None` to clear it. Called with the state lock
/// held so reports arrive in order; it must not call back into the boundary.
pub type FallbackErrorCallback = Arc<dyn Fn(Option<EasyVirtualDisplayErrorCode>) + Send + Sync>;

struct State {
    generation: u64,
    /// A task is waiting or adding for the current generation.
    busy: bool,
    /// The current episode is over: the add succeeded or failed for good.
    settled: bool,
    error: Option<EasyVirtualDisplayErrorCode>,
    disposed: bool,
}

pub struct FallbackDisplayBoundary {
    state: Mutex<State>,
    add_display: AddDisplayCallback,
    on_error: FallbackErrorCallback,
    debounce: Duration,
    retry_backoff: Duration,
    max_attempts: u32,
}

/// Only errors that may clear on their own are worth another try.
fn is_retryable(code: EasyVirtualDisplayErrorCode) -> bool {
    matches!(
        code,
        EasyVirtualDisplayErrorCode::DriverError
            | EasyVirtualDisplayErrorCode::NativeHostUnavailable
    )
}

fn needs_fallback(host: &HostSnapshot, settings: &AppSettings) -> bool {
    settings.fallback_display
        && host.status == DriverStatus::Ok
        && host.displays.is_empty()
        && host.active_outputs == 0
}

impl FallbackDisplayBoundary {
    pub fn new(add_display: AddDisplayCallback, on_error: FallbackErrorCallback) -> Arc<Self> {
        Self::with_config(
            add_display,
            on_error,
            DEFAULT_DEBOUNCE,
            RETRY_BACKOFF,
            MAX_ATTEMPTS,
        )
    }

    pub fn with_config(
        add_display: AddDisplayCallback,
        on_error: FallbackErrorCallback,
        debounce: Duration,
        retry_backoff: Duration,
        max_attempts: u32,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                generation: 0,
                busy: false,
                settled: false,
                error: None,
                disposed: false,
            }),
            add_display,
            on_error,
            debounce,
            retry_backoff,
            max_attempts,
        })
    }

    pub async fn handle_snapshot(self: &Arc<Self>, host: &HostSnapshot, settings: &AppSettings) {
        let mut state = self.state.lock().await;
        if state.disposed {
            return;
        }

        if !needs_fallback(host, settings) {
            if state.busy || state.settled {
                state.generation += 1;
                state.busy = false;
                state.settled = false;
            }
            self.set_error(&mut state, None);
            return;
        }

        if state.busy || state.settled {
            return;
        }
        state.busy = true;
        let generation = state.generation;
        tokio::spawn(Arc::clone(self).run(generation));
    }

    async fn run(self: Arc<Self>, generation: u64) {
        let mut delay = self.debounce;
        let mut attempts = 0;
        loop {
            sleep(delay).await;
            if !self.is_current(&*self.state.lock().await, generation) {
                return;
            }

            let result = (self.add_display)().await;
            attempts += 1;

            let mut state = self.state.lock().await;
            if !self.is_current(&state, generation) {
                return;
            }
            let error = match result {
                Ok(()) => None,
                Err(e) if is_retryable(e.code) && attempts < self.max_attempts => {
                    delay = self.retry_backoff;
                    continue;
                }
                Err(e) => Some(e.code),
            };
            state.busy = false;
            state.settled = true;
            self.set_error(&mut state, error);
            return;
        }
    }

    fn is_current(&self, state: &State, generation: u64) -> bool {
        !state.disposed && state.generation == generation
    }

    fn set_error(&self, state: &mut State, error: Option<EasyVirtualDisplayErrorCode>) {
        if state.error != error {
            state.error = error;
            (self.on_error)(error);
        }
    }

    pub async fn dispose(self: &Arc<Self>) {
        let mut state = self.state.lock().await;
        state.disposed = true;
        state.generation += 1;
        state.busy = false;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex as StdMutex;

    use crate::contracts::{
        default_app_settings, empty_host_snapshot, AppSettings, DisplaySummary,
    };

    use super::*;

    type Reports = Arc<StdMutex<Vec<Option<EasyVirtualDisplayErrorCode>>>>;

    fn make_settings(fallback: bool) -> AppSettings {
        let mut s = default_app_settings();
        s.fallback_display = fallback;
        s
    }

    fn empty_machine() -> HostSnapshot {
        let mut snap = empty_host_snapshot();
        snap.status = DriverStatus::Ok;
        snap
    }

    fn with_monitor() -> HostSnapshot {
        let mut snap = empty_machine();
        snap.active_outputs = 1;
        snap
    }

    fn with_own_display(active: bool) -> HostSnapshot {
        let mut snap = empty_machine();
        snap.active_outputs = i32::from(active);
        snap.displays = vec![DisplaySummary {
            index: 0,
            identifier: 1,
            device_name: "VDD".into(),
            display_name: "Virtual".into(),
            active,
            current_mode: None,
            current_orientation: crate::contracts::Orientation::Landscape,
            supported_resolutions: vec![],
            unsupported_current_mode: false,
        }];
        snap
    }

    /// Fails with each code in turn, then succeeds once they run out.
    fn scripted_callback(
        count: Arc<AtomicU32>,
        failures: Vec<EasyVirtualDisplayErrorCode>,
    ) -> AddDisplayCallback {
        let failures = Arc::new(failures);
        Arc::new(move || {
            let n = count.fetch_add(1, Ordering::SeqCst) as usize;
            let failure = failures.get(n).copied();
            Box::pin(async move {
                match failure {
                    Some(code) => Err(EasyVirtualDisplayError::new(code, "fake failure")),
                    None => Ok(()),
                }
            })
        })
    }

    fn counting_callback(count: Arc<AtomicU32>) -> AddDisplayCallback {
        scripted_callback(count, vec![])
    }

    fn failing_callback(
        count: Arc<AtomicU32>,
        code: EasyVirtualDisplayErrorCode,
    ) -> AddDisplayCallback {
        scripted_callback(count, vec![code; 10])
    }

    fn recorder() -> (Reports, FallbackErrorCallback) {
        let reports: Reports = Arc::default();
        let sink = reports.clone();
        (reports, Arc::new(move |e| sink.lock().unwrap().push(e)))
    }

    fn fast_config(
        cb: AddDisplayCallback,
        on_error: FallbackErrorCallback,
    ) -> Arc<FallbackDisplayBoundary> {
        FallbackDisplayBoundary::with_config(
            cb,
            on_error,
            Duration::from_millis(10),
            Duration::from_millis(10),
            3,
        )
    }

    fn quiet(cb: AddDisplayCallback) -> Arc<FallbackDisplayBoundary> {
        fast_config(cb, Arc::new(|_| {}))
    }

    async fn settle() {
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    #[tokio::test]
    async fn does_not_add_when_fallback_disabled() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = quiet(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(false))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn does_not_add_while_any_output_is_active() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = quiet(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&with_monitor(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn does_not_add_when_an_own_display_exists_even_inactive() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = quiet(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&with_own_display(false), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn does_not_add_while_driver_is_not_ok() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = quiet(counting_callback(count.clone()));
        let mut snap = empty_machine();
        snap.status = DriverStatus::NotInstalled;
        boundary.handle_snapshot(&snap, &make_settings(true)).await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn adds_once_when_nothing_is_displayed() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = quiet(counting_callback(count.clone()));
        for _ in 0..3 {
            boundary
                .handle_snapshot(&empty_machine(), &make_settings(true))
                .await;
        }
        settle().await;
        // Later snapshots that still look empty (the new display not enumerated yet)
        // must not add a second one.
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn adds_again_after_the_need_ends_and_returns() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = quiet(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        settle().await;
        boundary
            .handle_snapshot(&with_own_display(true), &make_settings(true))
            .await;
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn retries_transient_failures_up_to_the_limit_then_reports() {
        let count = Arc::new(AtomicU32::new(0));
        let (reports, on_error) = recorder();
        let boundary = fast_config(
            failing_callback(count.clone(), EasyVirtualDisplayErrorCode::DriverError),
            on_error,
        );
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 3);
        assert_eq!(
            *reports.lock().unwrap(),
            vec![Some(EasyVirtualDisplayErrorCode::DriverError)]
        );

        // The same episode does not start over on later snapshots.
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn permanent_failure_is_reported_without_retry() {
        let count = Arc::new(AtomicU32::new(0));
        let (reports, on_error) = recorder();
        let boundary = fast_config(
            failing_callback(count.clone(), EasyVirtualDisplayErrorCode::LimitExceeded),
            on_error,
        );
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(
            *reports.lock().unwrap(),
            vec![Some(EasyVirtualDisplayErrorCode::LimitExceeded)]
        );
    }

    #[tokio::test]
    async fn success_after_a_retry_reports_nothing() {
        let count = Arc::new(AtomicU32::new(0));
        let (reports, on_error) = recorder();
        let boundary = fast_config(
            scripted_callback(
                count.clone(),
                vec![EasyVirtualDisplayErrorCode::DriverError],
            ),
            on_error,
        );
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 2);
        assert!(reports.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn error_clears_when_the_need_ends() {
        let count = Arc::new(AtomicU32::new(0));
        let (reports, on_error) = recorder();
        let boundary = fast_config(
            failing_callback(count.clone(), EasyVirtualDisplayErrorCode::LimitExceeded),
            on_error,
        );
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        settle().await;
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(false))
            .await;
        assert_eq!(
            *reports.lock().unwrap(),
            vec![Some(EasyVirtualDisplayErrorCode::LimitExceeded), None]
        );
    }

    #[tokio::test]
    async fn toggling_fallback_off_cancels_pending_attempt() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = quiet(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(false))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn output_appearing_cancels_pending_attempt() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = quiet(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        boundary
            .handle_snapshot(&with_monitor(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn stale_retry_does_not_report_into_a_newer_episode() {
        let count = Arc::new(AtomicU32::new(0));
        let (reports, on_error) = recorder();
        // Slow enough that the episode ends while the first attempt is in flight.
        let slow: AddDisplayCallback = {
            let count = count.clone();
            Arc::new(move || {
                count.fetch_add(1, Ordering::SeqCst);
                Box::pin(async {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Err(EasyVirtualDisplayError::new(
                        EasyVirtualDisplayErrorCode::LimitExceeded,
                        "fake failure",
                    ))
                })
            })
        };
        let boundary = fast_config(slow, on_error);
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        tokio::time::sleep(Duration::from_millis(30)).await;
        boundary
            .handle_snapshot(&with_monitor(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert!(reports.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn dispose_prevents_future_attempts() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = quiet(counting_callback(count.clone()));
        boundary.dispose().await;
        boundary
            .handle_snapshot(&empty_machine(), &make_settings(true))
            .await;
        settle().await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
}
