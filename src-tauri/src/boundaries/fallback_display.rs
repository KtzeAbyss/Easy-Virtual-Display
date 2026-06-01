//! Fallback virtual display state machine. Mirrors `src/main/fallback-display.ts`.
//!
//! When the user enables `fallbackDisplay` and the host snapshot has no active displays,
//! schedule a debounced `add_display()` (1s default). If the call fails, retry up to
//! `MAX_RETRIES` times with `RETRY_BACKOFF`. Reset on any of: active display appears,
//! `fallbackDisplay` toggles off, or `dispose()` is called.
//!
//! Pure-Rust logic — no OS calls — so this module is cross-platform without cfg gates.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::{AbortHandle, JoinHandle};
use tokio::time::{sleep, Duration};

use crate::contracts::{AppSettings, HostSnapshot};
use crate::errors::EasyVirtualDisplayError;

const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(1000);
const RETRY_BACKOFF: Duration = Duration::from_millis(2000);
const MAX_RETRIES: u32 = 3;

pub type AddDisplayCallback = Arc<
    dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), EasyVirtualDisplayError>> + Send>>
        + Send
        + Sync,
>;

struct State {
    pending: Option<AbortHandle>,
    last_attempt_revision: Option<i64>,
    attempt_count: u32,
    in_flight: bool,
    disposed: bool,
}

impl State {
    fn new() -> Self {
        Self {
            pending: None,
            last_attempt_revision: None,
            attempt_count: 0,
            in_flight: false,
            disposed: false,
        }
    }

    fn abort_pending(&mut self) {
        if let Some(h) = self.pending.take() {
            h.abort();
        }
    }

    fn reset(&mut self) {
        self.abort_pending();
        self.last_attempt_revision = None;
        self.attempt_count = 0;
    }
}

pub struct FallbackDisplayBoundary {
    state: Mutex<State>,
    add_display: AddDisplayCallback,
    debounce: Duration,
    retry_backoff: Duration,
    max_retries: u32,
}

impl FallbackDisplayBoundary {
    pub fn new(add_display: AddDisplayCallback) -> Arc<Self> {
        Self::with_config(add_display, DEFAULT_DEBOUNCE, RETRY_BACKOFF, MAX_RETRIES)
    }

    pub fn with_config(
        add_display: AddDisplayCallback,
        debounce: Duration,
        retry_backoff: Duration,
        max_retries: u32,
    ) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State::new()),
            add_display,
            debounce,
            retry_backoff,
            max_retries,
        })
    }

    pub async fn handle_snapshot(self: &Arc<Self>, host: &HostSnapshot, settings: &AppSettings) {
        let has_active = host.displays.iter().any(|d| d.active);
        let mut state = self.state.lock().await;

        if !settings.fallback_display || has_active {
            state.reset();
            return;
        }

        if state.disposed || state.in_flight || state.pending.is_some() {
            return;
        }

        if state.last_attempt_revision == Some(host.revision)
            && state.attempt_count >= self.max_retries
        {
            return;
        }

        if state.last_attempt_revision != Some(host.revision) {
            state.attempt_count = 0;
        }
        state.last_attempt_revision = Some(host.revision);

        self.schedule(&mut state, self.debounce);
    }

    /// Schedule a one-shot task that performs the add_display call after `delay`.
    /// Caller already holds the state lock.
    fn schedule(self: &Arc<Self>, state: &mut State, delay: Duration) {
        if state.disposed {
            return;
        }
        let this = Arc::clone(self);
        let handle: JoinHandle<()> = tokio::spawn(async move {
            sleep(delay).await;
            this.fire().await;
        });
        state.pending = Some(handle.abort_handle());
    }

    async fn fire(self: Arc<Self>) {
        {
            let mut state = self.state.lock().await;
            if state.disposed {
                return;
            }
            state.pending = None;
            state.in_flight = true;
        }

        let result = (self.add_display)().await;

        let mut state = self.state.lock().await;
        state.in_flight = false;
        if state.disposed {
            return;
        }
        match result {
            Ok(()) => {
                // Don't reset attempt_count here — the upcoming snapshot will either show
                // an active display (triggering reset() in handle_snapshot) or stay empty
                // and we'll know more attempts are needed.
            }
            Err(_) => {
                state.attempt_count += 1;
                if state.attempt_count < self.max_retries {
                    self.schedule(&mut state, self.retry_backoff);
                }
            }
        }
    }

    pub async fn dispose(self: &Arc<Self>) {
        let mut state = self.state.lock().await;
        state.disposed = true;
        state.in_flight = false;
        state.reset();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use crate::contracts::{default_app_settings, empty_host_snapshot, AppSettings, DisplaySummary};

    use super::*;

    fn make_settings(fallback: bool) -> AppSettings {
        let mut s = default_app_settings();
        s.fallback_display = fallback;
        s
    }

    fn make_snapshot(revision: i64, with_active: bool) -> HostSnapshot {
        let mut snap = empty_host_snapshot();
        snap.revision = revision;
        if with_active {
            snap.displays = vec![DisplaySummary {
                index: 0,
                identifier: 1,
                device_name: "VDD".into(),
                display_name: "Virtual".into(),
                active: true,
                current_mode: None,
                current_orientation: crate::contracts::Orientation::Landscape,
                supported_resolutions: vec![],
                unsupported_current_mode: false,
            }];
        }
        snap
    }

    fn counting_callback(count: Arc<AtomicU32>) -> AddDisplayCallback {
        Arc::new(move || {
            let c = count.clone();
            Box::pin(async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        })
    }

    fn failing_callback(count: Arc<AtomicU32>) -> AddDisplayCallback {
        Arc::new(move || {
            let c = count.clone();
            Box::pin(async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err(EasyVirtualDisplayError::new(
                    crate::errors::EasyVirtualDisplayErrorCode::DriverError,
                    "fake failure",
                ))
            })
        })
    }

    fn fast_config(cb: AddDisplayCallback) -> Arc<FallbackDisplayBoundary> {
        FallbackDisplayBoundary::with_config(
            cb,
            Duration::from_millis(10),
            Duration::from_millis(10),
            3,
        )
    }

    #[tokio::test]
    async fn does_not_schedule_when_fallback_disabled() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = fast_config(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&make_snapshot(1, false), &make_settings(false))
            .await;
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn does_not_schedule_when_active_displays_exist() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = fast_config(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&make_snapshot(1, true), &make_settings(true))
            .await;
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn schedules_add_when_enabled_and_no_active_displays() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = fast_config(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&make_snapshot(1, false), &make_settings(true))
            .await;
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn does_not_double_schedule_same_revision() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = fast_config(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&make_snapshot(7, false), &make_settings(true))
            .await;
        boundary
            .handle_snapshot(&make_snapshot(7, false), &make_settings(true))
            .await;
        boundary
            .handle_snapshot(&make_snapshot(7, false), &make_settings(true))
            .await;
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_on_failure_up_to_max_then_stops() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = fast_config(failing_callback(count.clone()));
        boundary
            .handle_snapshot(&make_snapshot(1, false), &make_settings(true))
            .await;
        // Initial attempt + (MAX_RETRIES - 1) retries = MAX_RETRIES total. With debounce
        // 10ms and backoff 10ms, all finish well within 200ms.
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn toggling_fallback_off_cancels_pending_attempt() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = fast_config(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&make_snapshot(1, false), &make_settings(true))
            .await;
        // Cancel before debounce fires.
        boundary
            .handle_snapshot(&make_snapshot(1, false), &make_settings(false))
            .await;
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn active_display_appearing_cancels_pending_attempt() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = fast_config(counting_callback(count.clone()));
        boundary
            .handle_snapshot(&make_snapshot(1, false), &make_settings(true))
            .await;
        boundary
            .handle_snapshot(&make_snapshot(2, true), &make_settings(true))
            .await;
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn new_revision_resets_attempt_count() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = fast_config(failing_callback(count.clone()));
        boundary
            .handle_snapshot(&make_snapshot(1, false), &make_settings(true))
            .await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(count.load(Ordering::SeqCst), 3);

        boundary
            .handle_snapshot(&make_snapshot(2, false), &make_settings(true))
            .await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        // New revision → attempt_count reset → another 3 attempts.
        assert_eq!(count.load(Ordering::SeqCst), 6);
    }

    #[tokio::test]
    async fn dispose_prevents_future_attempts() {
        let count = Arc::new(AtomicU32::new(0));
        let boundary = fast_config(counting_callback(count.clone()));
        boundary.dispose().await;
        boundary
            .handle_snapshot(&make_snapshot(1, false), &make_settings(true))
            .await;
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
}
