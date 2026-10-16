//! Mirror a virtual display into a local window ("show the virtual screen like a VM
//! window", issue #39).
//!
//! Every viewer is a plain Win32 window with a Windows.Graphics.Capture session bound to
//! that display's `HMONITOR` and a D3D11 flip-model swap chain. All viewers share one UI
//! thread, one message loop and one D3D device. Interaction uses a VMware-style *grab*:
//! clicking the window confines the system cursor to the mirrored display, so mouse and
//! keyboard reach the apps on it natively after the initiating button-down is forwarded;
//! a global hotkey — or, as a last resort, `Ctrl+Alt+Del` — releases the grab.
//!
//! The Tauri layer only ever sees [`ViewerHost`] — the Windows implementation lives in
//! [`win`], and other platforms get a stub that reports `viewer_unsupported`.

use std::sync::Arc;

use crate::errors::{EasyVirtualDisplayError, EasyVirtualDisplayErrorCode};

#[cfg(target_os = "windows")]
mod win;

/// Everything a viewer window needs to know about the display it mirrors. Captions are
/// resolved (and localized) by the caller so this module stays i18n-agnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerTarget {
    /// VDD slot index — the same id `remove_display` / `set_display_mode` take.
    pub index: i32,
    /// GDI adapter name (`\\.\DISPLAY3`); resolves to the `HMONITOR` we capture.
    pub device_name: String,
    pub captions: ViewerCaptions,
}

/// Localized text shared by every viewer. `title_template` may contain an `{{index}}`
/// placeholder; the viewer resolves it for its own display index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerCaptions {
    pub title_template: String,
    /// Caption suffix while idle, e.g. `Click to control`.
    pub hint_idle: String,
    /// Caption suffix while grabbed; may contain a `{{hotkey}}` placeholder.
    pub hint_grabbed: String,
}

/// Invoked (from an arbitrary thread) whenever the set of open viewers changes; read the
/// new set with [`ViewerHost::open_indices`].
pub type ViewersChanged = Arc<dyn Fn() + Send + Sync>;

pub trait ViewerHost: Send + Sync {
    /// Open a viewer for `target`, or focus the existing one.
    fn open(&self, target: ViewerTarget) -> Result<(), EasyVirtualDisplayError>;
    fn close(&self, index: i32);
    fn update_captions(&self, captions: ViewerCaptions);
    /// Release any pointer grab without closing the viewer windows.
    fn release_input(&self);
    fn close_all(&self);
    fn open_indices(&self) -> Vec<i32>;
    /// Close every viewer whose display index is no longer in `alive` — used to reap
    /// windows when the underlying virtual display goes away.
    fn retain(&self, alive: &[i32]);
}

pub fn create(on_change: ViewersChanged) -> Arc<dyn ViewerHost> {
    #[cfg(target_os = "windows")]
    {
        win::create(on_change)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = on_change;
        Arc::new(UnsupportedViewerHost)
    }
}

#[cfg(not(target_os = "windows"))]
struct UnsupportedViewerHost;

#[cfg(not(target_os = "windows"))]
impl ViewerHost for UnsupportedViewerHost {
    fn open(&self, _target: ViewerTarget) -> Result<(), EasyVirtualDisplayError> {
        Err(unsupported())
    }
    fn close(&self, _index: i32) {}
    fn update_captions(&self, _captions: ViewerCaptions) {}
    fn release_input(&self) {}
    fn close_all(&self) {}
    fn open_indices(&self) -> Vec<i32> {
        Vec::new()
    }
    fn retain(&self, _alive: &[i32]) {}
}

#[cfg_attr(target_os = "windows", allow(dead_code))]
fn unsupported() -> EasyVirtualDisplayError {
    EasyVirtualDisplayError::new(
        EasyVirtualDisplayErrorCode::ViewerUnsupported,
        "Mirroring a virtual display into a window is only supported on Windows.",
    )
}
