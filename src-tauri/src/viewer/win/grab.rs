//! VMware-style pointer grab.
//!
//! Grabbing confines the *real* system cursor to the mirrored display's rectangle. Mouse
//! and keyboard then reach the apps on that display through the normal input path. Only
//! the button-down that transitions from the viewer into the mirrored desktop is
//! forwarded; all subsequent input follows the normal system path.
//!
//! The cursor is a single system-wide resource, so there is exactly one [`Grab`], owned
//! by the viewer UI thread. Its release hotkey and re-clip timer belong to that thread's
//! control window, and its WinEvent hooks are delivered on the same thread.
//!
//! Ways out of a grab:
//! - the release hotkey (first combination Windows accepts from [`HOTKEYS`]);
//! - any desktop switch — `Ctrl+Alt+Del`, a UAC prompt, locking the session. Windows
//!   handles the secure attention sequence before any application sees it, so this works
//!   even while a full-screen Remote Desktop client swallows every other key.
//!
//! Windows drops the clip rectangle whenever the foreground window changes, which happens
//! on the very first click into the mirrored desktop. A `EVENT_SYSTEM_FOREGROUND` hook
//! re-applies it immediately, and a slow timer covers anything the hook misses.

use std::sync::Mutex;

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, SendInput, UnregisterHotKey, HOT_KEY_MODIFIERS, INPUT, INPUT_0, INPUT_MOUSE,
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOUSEEVENTF_LEFTDOWN, MOUSEINPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    ClipCursor, GetAncestor, GetCursorPos, KillTimer, SetCursorPos, SetForegroundWindow, SetTimer,
    WindowFromPoint, EVENT_SYSTEM_DESKTOPSWITCH, EVENT_SYSTEM_FOREGROUND, GA_ROOT,
    WINEVENT_OUTOFCONTEXT,
};

use super::{monitor, ui};

pub(super) const HOTKEY_ID: i32 = 0xE7D;
pub(super) const TIMER_RECLIP: usize = 1;
const TIMER_RECLIP_MS: u32 = 1000;

const VK_G: u32 = 0x47;
const VK_F12: u32 = 0x7B;

/// Tried in order; the first one Windows accepts wins, so a conflicting global hotkey in
/// another app degrades to a different combination instead of breaking the release path.
/// `Ctrl+Alt+Break` / `Home` / `End` are deliberately absent: Remote Desktop owns them.
const HOTKEYS: &[(u32, &str)] = &[(VK_G, "Ctrl+Alt+G"), (VK_F12, "Ctrl+Alt+F12")];

/// The rectangle the cursor is confined to, or `None` when nothing is grabbed. Read by
/// the foreground hook and cleared by [`release_global_clip`] from any thread.
static CLIP: Mutex<Option<RECT>> = Mutex::new(None);

struct Active {
    owner: HWND,
    /// GDI name of the grabbed display, so the timer can follow it if it moves.
    device_name: String,
    /// Where the cursor sat before the grab, so releasing puts it back.
    restore: POINT,
    hooks: [HWINEVENTHOOK; 2],
    label: &'static str,
}

#[derive(Default)]
pub(super) struct Grab {
    active: Option<Active>,
}

impl Grab {
    pub fn owner(&self) -> Option<HWND> {
        self.active.as_ref().map(|active| active.owner)
    }

    /// Label of the release hotkey when `hwnd` holds the grab (for its window caption).
    pub fn label_for(&self, hwnd: HWND) -> Option<&'static str> {
        self.active
            .as_ref()
            .filter(|active| active.owner == hwnd)
            .map(|active| active.label)
    }

    /// Confine the cursor to `rect` on behalf of `owner` and park it at `seed`. A grab
    /// held by another viewer is handed over, keeping its original restore point. Returns
    /// `false` when Windows refuses every candidate hotkey — grabbing without a way out
    /// would trap the user. The caller forwards the initiating click afterwards with
    /// [`forward_initial_press`].
    pub fn engage(
        &mut self,
        control: HWND,
        owner: HWND,
        device_name: &str,
        rect: RECT,
        seed: POINT,
    ) -> bool {
        if let Some(active) = self.active.as_mut() {
            active.owner = owner;
            active.device_name = device_name.to_owned();
        } else {
            let Some(label) = register_hotkey(control) else {
                return false;
            };
            let mut restore = POINT::default();
            unsafe {
                let _ = GetCursorPos(&mut restore);
                SetTimer(control, TIMER_RECLIP, TIMER_RECLIP_MS, None);
            }
            self.active = Some(Active {
                owner,
                device_name: device_name.to_owned(),
                restore,
                hooks: [
                    hook(EVENT_SYSTEM_FOREGROUND, foreground_proc),
                    hook(EVENT_SYSTEM_DESKTOPSWITCH, desktop_switch_proc),
                ],
                label,
            });
        }

        unsafe {
            let _ = SetCursorPos(seed.x, seed.y);
        }
        set_clip(Some(rect));
        true
    }

    /// Undo the grab and put the cursor back. Returns the viewer that held it so the
    /// caller can refresh its caption and, if appropriate, give it focus again.
    pub fn release(&mut self, control: HWND) -> Option<HWND> {
        let active = self.active.take()?;
        unsafe {
            let _ = KillTimer(control, TIMER_RECLIP);
            for hook in active.hooks {
                let _ = UnhookWinEvent(hook);
            }
            let _ = UnregisterHotKey(control, HOTKEY_ID);
        }
        set_clip(None);
        unsafe {
            let _ = SetCursorPos(active.restore.x, active.restore.y);
        }
        Some(active.owner)
    }

    /// The owner's display moved or changed resolution.
    pub fn update_rect(&self, owner: HWND, rect: RECT) {
        if self.owner() == Some(owner) && clip_engaged() {
            set_clip(Some(rect));
        }
    }

    /// Timer tick: follow the grabbed display if it moved, and re-apply the clip in case
    /// Windows dropped it without a foreground change.
    pub fn reclip(&self) {
        let Some(active) = self.active.as_ref() else {
            return;
        };
        // A cleared clip means another thread released the cursor on purpose (quit flow);
        // the owning grab is about to be torn down, so do not re-confine it.
        if !clip_engaged() {
            return;
        }
        match monitor::find_by_device_name(&active.device_name) {
            Some(found) => set_clip(Some(found.rect)),
            None => apply_clip(),
        }
    }
}

/// Free the cursor immediately, from any thread. The UI thread then restores the saved
/// cursor position and tears down the hotkey, hooks and timer when it gets to it.
pub(super) fn release_global_clip() {
    set_clip(None);
}

/// The physical press was delivered to the viewer before the cursor moved. Forward only
/// that down transition; the user's real release completes the click or drag naturally.
pub(super) fn forward_initial_press(seed: POINT) {
    unsafe {
        let target = WindowFromPoint(seed);
        if !target.0.is_null() {
            let root = GetAncestor(target, GA_ROOT);
            if !root.0.is_null() {
                let _ = SetForegroundWindow(root);
            }
        }

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dwFlags: MOUSEEVENTF_LEFTDOWN,
                    ..Default::default()
                },
            },
        };
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

fn clip_engaged() -> bool {
    CLIP.lock().expect("grab clip poisoned").is_some()
}

/// Publish and apply under one lock, so the foreground hook can never re-apply a
/// rectangle that was replaced or cleared in between.
fn set_clip(rect: Option<RECT>) {
    let mut clip = CLIP.lock().expect("grab clip poisoned");
    *clip = rect;
    unsafe {
        let _ = ClipCursor(rect.as_ref().map(|rect| rect as *const RECT));
    }
}

fn apply_clip() {
    let clip = CLIP.lock().expect("grab clip poisoned");
    if let Some(rect) = clip.as_ref() {
        unsafe {
            let _ = ClipCursor(Some(rect));
        }
    }
}

fn register_hotkey(control: HWND) -> Option<&'static str> {
    let modifiers: HOT_KEY_MODIFIERS = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT;
    HOTKEYS
        .iter()
        .find(|(vk, _)| unsafe { RegisterHotKey(control, HOTKEY_ID, modifiers, *vk) }.is_ok())
        .map(|(_, label)| *label)
}

type WinEventProc = unsafe extern "system" fn(HWINEVENTHOOK, u32, HWND, i32, i32, u32, u32);

fn hook(event: u32, proc: WinEventProc) -> HWINEVENTHOOK {
    unsafe { SetWinEventHook(event, event, None, Some(proc), 0, 0, WINEVENT_OUTOFCONTEXT) }
}

unsafe extern "system" fn foreground_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _object_id: i32,
    _child_id: i32,
    _thread_id: u32,
    _time: u32,
) {
    apply_clip();
}

unsafe extern "system" fn desktop_switch_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _object_id: i32,
    _child_id: i32,
    _thread_id: u32,
    _time: u32,
) {
    // Stop the foreground hook from re-confining the cursor on the way back, then let the
    // control window finish the release outside this callback.
    release_global_clip();
    ui::post_release_grab();
}
