//! The viewer UI thread: one thread and one message loop host every viewer window.
//!
//! Other threads never touch a window directly. They push a [`Command`] onto a channel and
//! wake the thread's message-only *control window*, which drains the channel in order.
//! Because every window operation is serialized on this thread, opening, closing and
//! grabbing need no cross-thread bookkeeping.
//!
//! All state lives in a thread-local [`Ui`]. Window procedures reach it through [`with`]
//! and keep each `RefCell` borrow short: nothing that can dispatch a message back into a
//! window procedure (`DestroyWindow`, `SetWindowTextW`, `SetForegroundWindow`, …) is called
//! while a borrow is held.

use std::cell::{OnceCell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, PostMessageW,
    RegisterClassExW, SetForegroundWindow, TranslateMessage, HMENU, HWND_MESSAGE, MSG, WM_APP,
    WM_HOTKEY, WM_TIMER, WNDCLASSEXW, WINDOW_EX_STYLE, WINDOW_STYLE,
};

use crate::errors::EasyVirtualDisplayError;
use crate::viewer::{ViewerCaptions, ViewerTarget, ViewersChanged};

use super::grab::{self, Grab};
use super::renderer::Gpu;
use super::{hwnd_raw, window};

/// Wakes the control window: commands are waiting on the channel.
pub(super) const WM_APP_COMMAND: u32 = WM_APP + 0x10;
/// Release the pointer grab (posted by the desktop-switch hook).
const WM_APP_RELEASE_GRAB: u32 = WM_APP + 0x11;

const CONTROL_CLASS: PCWSTR = w!("EasyVirtualDisplayViewerControl");

pub(super) enum Command {
    Open {
        target: ViewerTarget,
        reply: SyncSender<Result<(), EasyVirtualDisplayError>>,
    },
    Close(i32),
    /// Close every viewer whose display index is not listed.
    Retain(Vec<i32>),
    UpdateCaptions(ViewerCaptions),
    ReleaseInput(SyncSender<()>),
    CloseAll(SyncSender<()>),
}

/// State shared with the [`ViewerHost`](crate::viewer::ViewerHost) facade.
pub(super) struct Shared {
    /// Display indices with an open viewer; written only by the UI thread.
    pub open: Mutex<Vec<i32>>,
    pub on_change: ViewersChanged,
}

pub(super) struct Ui {
    control: HWND,
    commands: Receiver<Command>,
    shared: Arc<Shared>,
    viewers: RefCell<BTreeMap<i32, HWND>>,
    grab: RefCell<Grab>,
    gpu: RefCell<Option<Rc<Gpu>>>,
}

thread_local! {
    static UI: OnceCell<Ui> = const { OnceCell::new() };
}

/// Run `f` against the UI thread's state. `None` off the UI thread or before startup.
pub(super) fn with<R>(f: impl FnOnce(&Ui) -> R) -> Option<R> {
    UI.with(|cell| cell.get().map(f))
}

/// Ask the control window to release the grab. Called from WinEvent hooks, which run on
/// the UI thread but must not tear the grab down from inside the callback.
pub(super) fn post_release_grab() {
    with(|ui| unsafe {
        let _ = PostMessageW(ui.control, WM_APP_RELEASE_GRAB, WPARAM(0), LPARAM(0));
    });
}

struct WinRtApartment;

impl WinRtApartment {
    fn initialize() -> windows::core::Result<Self> {
        unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
        Ok(Self)
    }
}

impl Drop for WinRtApartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

/// Thread body. Reports the control window handle (as `isize`, since `HWND` is not
/// `Send`) once the thread can accept commands, then pumps messages for the lifetime of
/// the process.
pub(super) fn run(
    shared: Arc<Shared>,
    commands: Receiver<Command>,
    ready: SyncSender<Result<isize, EasyVirtualDisplayError>>,
) {
    // The capture frame pools are free-threaded, so the UI thread joins the MTA.
    let _winrt = match WinRtApartment::initialize() {
        Ok(apartment) => apartment,
        Err(error) => {
            let _ = ready.send(Err(window::start_failed(error)));
            return;
        }
    };
    let control = match create_control_window() {
        Ok(control) => control,
        Err(error) => {
            let _ = ready.send(Err(window::start_failed(error)));
            return;
        }
    };

    UI.with(|cell| {
        let _ = cell.set(Ui {
            control,
            commands,
            shared,
            viewers: RefCell::new(BTreeMap::new()),
            grab: RefCell::new(Grab::default()),
            gpu: RefCell::new(None),
        });
    });
    if ready.send(Ok(hwnd_raw(control))).is_err() {
        return;
    }

    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

impl Ui {
    fn drain_commands(&self) {
        while let Ok(command) = self.commands.try_recv() {
            match command {
                Command::Open { target, reply } => {
                    let _ = reply.send(self.open(target));
                }
                Command::Close(index) => {
                    let hwnd = self.viewers.borrow().get(&index).copied();
                    if let Some(hwnd) = hwnd {
                        destroy(hwnd);
                    }
                }
                Command::Retain(alive) => {
                    let stale: Vec<HWND> = self
                        .viewers
                        .borrow()
                        .iter()
                        .filter(|(index, _)| !alive.contains(index))
                        .map(|(_, &hwnd)| hwnd)
                        .collect();
                    stale.into_iter().for_each(destroy);
                }
                Command::UpdateCaptions(captions) => {
                    for hwnd in self.windows() {
                        window::set_captions(hwnd, captions.clone());
                    }
                }
                Command::ReleaseInput(reply) => {
                    // The caller is about to show its own dialog; leave focus alone.
                    self.release_grab(false);
                    let _ = reply.send(());
                }
                Command::CloseAll(reply) => {
                    self.windows().into_iter().for_each(destroy);
                    let _ = reply.send(());
                }
            }
        }
    }

    fn open(&self, target: ViewerTarget) -> Result<(), EasyVirtualDisplayError> {
        let index = target.index;
        let existing = self.viewers.borrow().get(&index).copied();
        if let Some(hwnd) = existing {
            window::focus(hwnd);
            return Ok(());
        }

        let hwnd = window::create(target, self.gpu()?)?;
        self.viewers.borrow_mut().insert(index, hwnd);
        self.publish();
        Ok(())
    }

    /// Called from a viewer's `WM_NCDESTROY`.
    pub fn forget(&self, hwnd: HWND) {
        let removed = {
            let mut viewers = self.viewers.borrow_mut();
            let before = viewers.len();
            viewers.retain(|_, &mut open| open != hwnd);
            viewers.len() != before
        };
        if removed {
            self.publish();
        }
    }

    /// Click-to-grab on behalf of `owner`.
    pub fn engage(&self, owner: HWND, device_name: &str, rect: RECT, seed: POINT) {
        let (engaged, previous) = {
            let Ok(mut grab) = self.grab.try_borrow_mut() else {
                return;
            };
            let previous = grab.owner().filter(|&previous| previous != owner);
            if grab.owner() == Some(owner) {
                return;
            }
            (
                grab.engage(self.control, owner, device_name, rect, seed),
                previous,
            )
        };
        if !engaged {
            return;
        }
        grab::forward_initial_press(seed);
        window::refresh_title(owner);
        if let Some(previous) = previous {
            window::refresh_title(previous);
        }
    }

    /// Release the grab, if any. `refocus` hands the keyboard back to the viewer that held
    /// it, which is what a user pressing the release hotkey expects.
    pub fn release_grab(&self, refocus: bool) {
        let Some(owner) = self
            .grab
            .try_borrow_mut()
            .ok()
            .and_then(|mut grab| grab.release(self.control))
        else {
            return;
        };
        if refocus {
            unsafe {
                let _ = SetForegroundWindow(owner);
            }
        }
        window::refresh_title(owner);
    }

    /// Release the grab only if `hwnd` holds it — for a viewer that is going away.
    pub fn release_grab_held_by(&self, hwnd: HWND) {
        if let Ok(mut grab) = self.grab.try_borrow_mut() {
            if grab.owner() == Some(hwnd) {
                grab.release(self.control);
            }
        }
    }

    pub fn grab_label_for(&self, hwnd: HWND) -> Option<&'static str> {
        self.grab.try_borrow().ok()?.label_for(hwnd)
    }

    pub fn update_grab_rect(&self, owner: HWND, rect: RECT) {
        if let Ok(grab) = self.grab.try_borrow() {
            grab.update_rect(owner, rect);
        }
    }

    fn gpu(&self) -> Result<Rc<Gpu>, EasyVirtualDisplayError> {
        let mut gpu = self.gpu.borrow_mut();
        if let Some(current) = gpu.as_ref().filter(|current| !current.is_lost()) {
            return Ok(Rc::clone(current));
        }
        let fresh = Rc::new(Gpu::new().map_err(window::start_failed)?);
        *gpu = Some(Rc::clone(&fresh));
        Ok(fresh)
    }

    fn windows(&self) -> Vec<HWND> {
        self.viewers.borrow().values().copied().collect()
    }

    fn publish(&self) {
        let open: Vec<i32> = self.viewers.borrow().keys().copied().collect();
        *self.shared.open.lock().expect("viewer set poisoned") = open;
        (self.shared.on_change)();
    }
}

fn destroy(hwnd: HWND) {
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

fn create_control_window() -> windows::core::Result<HWND> {
    let instance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    let class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(control_proc),
        hInstance: instance,
        lpszClassName: CONTROL_CLASS,
        ..Default::default()
    };
    // Registered once per UI thread start. The thread runs for the life of the process,
    // so in practice that is exactly once.
    if unsafe { RegisterClassExW(&class) } == 0 {
        return Err(windows::core::Error::from_win32());
    }
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            CONTROL_CLASS,
            PCWSTR::null(),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            HMENU::default(),
            instance,
            None,
        )
    }
}

unsafe extern "system" fn control_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_APP_COMMAND => {
            with(|ui| ui.drain_commands());
            LRESULT(0)
        }
        WM_HOTKEY if wparam.0 as i32 == grab::HOTKEY_ID => {
            with(|ui| ui.release_grab(true));
            LRESULT(0)
        }
        WM_APP_RELEASE_GRAB => {
            with(|ui| ui.release_grab(false));
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == grab::TIMER_RECLIP => {
            with(|ui| {
                if let Ok(grab) = ui.grab.try_borrow() {
                    grab.reclip();
                }
            });
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
