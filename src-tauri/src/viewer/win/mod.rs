//! Windows implementation of [`ViewerHost`](super::ViewerHost).
//!
//! Every viewer window lives on one UI thread ([`ui`]), started on the first `open`. This
//! type is the thread-safe front door: it forwards each call as a [`Command`] and, where
//! the caller needs an answer, waits a bounded time for the reply. The only state read
//! from other threads is the set of open display indices, which the UI thread publishes.

mod capture;
mod grab;
mod monitor;
mod renderer;
mod ui;
mod window;

use std::sync::mpsc::{channel, sync_channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::errors::{EasyVirtualDisplayError, EasyVirtualDisplayErrorCode};

use super::{ViewerCaptions, ViewerHost, ViewerTarget, ViewersChanged};
use ui::{Command, Shared, WM_APP_COMMAND};

/// Creating the window, the D3D device and the capture session is a handful of
/// milliseconds; this only guards against a wedged GPU driver.
const START_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the quit flow waits for the UI thread to tidy up before moving on.
const SETTLE_TIMEOUT: Duration = Duration::from_millis(500);

/// `HWND` wraps a raw pointer, so it is neither `Send` nor storable in shared state.
/// Handles cross threads as plain `isize`.
#[inline]
pub(super) fn hwnd_from_raw(raw: isize) -> HWND {
    HWND(raw as *mut std::ffi::c_void)
}

#[inline]
pub(super) fn hwnd_raw(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

pub(super) fn create(on_change: ViewersChanged) -> Arc<dyn ViewerHost> {
    Arc::new(WindowsViewerHost {
        shared: Arc::new(Shared {
            open: Mutex::new(Vec::new()),
            on_change,
        }),
        inner: Mutex::new(Inner::default()),
    })
}

pub(super) struct WindowsViewerHost {
    shared: Arc<Shared>,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    ui: Option<UiHandle>,
    /// Latest localized captions. Kept under the same lock as the command channel so an
    /// `open` and a language change are always delivered in a consistent order.
    captions: Option<ViewerCaptions>,
}

struct UiHandle {
    commands: Sender<Command>,
    control: isize,
}

impl UiHandle {
    fn start(shared: Arc<Shared>) -> Result<Self, EasyVirtualDisplayError> {
        let (commands, receiver) = channel();
        let (ready, started) = sync_channel(1);
        std::thread::Builder::new()
            .name("viewer-ui".into())
            .spawn(move || ui::run(shared, receiver, ready))
            .map_err(|error| start_failed(format!("Could not start the viewer thread: {error}")))?;
        let control = started
            .recv_timeout(START_TIMEOUT)
            .map_err(|_| start_failed("Timed out waiting for the viewer thread to start."))??;
        Ok(Self { commands, control })
    }

    /// `false` once the UI thread is gone.
    fn send(&self, command: Command) -> bool {
        if self.commands.send(command).is_err() {
            return false;
        }
        // A failed wake-up leaves the command queued for the next successful one.
        let _ = unsafe {
            PostMessageW(
                hwnd_from_raw(self.control),
                WM_APP_COMMAND,
                WPARAM(0),
                LPARAM(0),
            )
        };
        true
    }
}

impl WindowsViewerHost {
    /// Forward `command` if the UI thread is running. Nothing to do otherwise: without a
    /// UI thread there are no windows to close, release or relabel.
    fn send(&self, command: Command) {
        let mut inner = self.inner.lock().expect("viewer host poisoned");
        if let Some(ui) = inner.ui.as_ref() {
            if !ui.send(command) {
                inner.ui = None;
            }
        }
    }

    fn send_and_wait(&self, command: impl FnOnce(std::sync::mpsc::SyncSender<()>) -> Command) {
        let (reply, done) = sync_channel(1);
        self.send(command(reply));
        let _ = done.recv_timeout(SETTLE_TIMEOUT);
    }
}

impl ViewerHost for WindowsViewerHost {
    fn open(&self, mut target: ViewerTarget) -> Result<(), EasyVirtualDisplayError> {
        let (reply, result) = sync_channel(1);
        {
            let mut inner = self.inner.lock().expect("viewer host poisoned");
            if let Some(captions) = inner.captions.clone() {
                target.captions = captions;
            }
            if inner.ui.is_none() {
                inner.ui = Some(UiHandle::start(Arc::clone(&self.shared))?);
            }
            let ui = inner.ui.as_ref().expect("viewer UI thread just started");
            if !ui.send(Command::Open { target, reply }) {
                inner.ui = None;
                return Err(start_failed("The viewer thread has stopped."));
            }
        }
        result
            .recv_timeout(START_TIMEOUT)
            .map_err(|_| start_failed("Timed out waiting for the display viewer window to start."))?
    }

    fn close(&self, index: i32) {
        self.send(Command::Close(index));
    }

    fn update_captions(&self, captions: ViewerCaptions) {
        let mut inner = self.inner.lock().expect("viewer host poisoned");
        inner.captions = Some(captions.clone());
        if let Some(ui) = inner.ui.as_ref() {
            if !ui.send(Command::UpdateCaptions(captions)) {
                inner.ui = None;
            }
        }
    }

    fn release_input(&self) {
        // Free the cursor right away, then give the UI thread a bounded chance to restore
        // its position and tear down the hotkey, hooks and timer.
        grab::release_global_clip();
        self.send_and_wait(Command::ReleaseInput);
    }

    fn close_all(&self) {
        grab::release_global_clip();
        self.send_and_wait(Command::CloseAll);
    }

    fn open_indices(&self) -> Vec<i32> {
        self.shared.open.lock().expect("viewer set poisoned").clone()
    }

    fn retain(&self, alive: &[i32]) {
        // Always forwarded, so it is ordered after any `open` still queued on the thread.
        self.send(Command::Retain(alive.to_vec()));
    }
}

fn start_failed(message: impl Into<String>) -> EasyVirtualDisplayError {
    EasyVirtualDisplayError::new(EasyVirtualDisplayErrorCode::ViewerStartFailed, message)
}
