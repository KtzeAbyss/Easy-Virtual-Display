//! The viewer window itself: one Win32 window per mirrored display, all of them created
//! and pumped by the UI thread (see [`super::ui`]).
//!
//! Window state lives in a `RefCell` reachable from `GWLP_USERDATA`. The window procedure
//! borrows it non-reentrantly (`try_borrow_mut`) and always drops the borrow before
//! calling back into USER32 with anything that could dispatch another message — captions
//! in particular are applied by the message handler, not by the state itself.

use std::cell::RefCell;
use std::rc::Rc;

use windows::core::{w, HRESULT, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::ValidateRect;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect,
    GetWindowLongPtrW, IsIconic, LoadCursorW, LoadIconW, PostMessageW, RegisterClassExW,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, CS_HREDRAW,
    CS_VREDRAW, GWLP_USERDATA, HMENU, IDC_ARROW, IDI_APPLICATION, MINMAXINFO, SWP_NOACTIVATE,
    SWP_NOZORDER, SW_RESTORE, SW_SHOW, WM_APP, WM_CLOSE, WM_DESTROY, WM_DPICHANGED,
    WM_ERASEBKGND, WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_NCDESTROY, WM_PAINT, WM_SIZE,
    WNDCLASSEXW, WS_EX_APPWINDOW, WS_OVERLAPPEDWINDOW,
};

use crate::errors::{EasyVirtualDisplayError, EasyVirtualDisplayErrorCode};
use crate::shell_locales::format_template;
use crate::viewer::{ViewerCaptions, ViewerTarget};

use super::capture::Capture;
use super::monitor::{self, MonitorTarget};
use super::renderer::{self, Gpu, Renderer};
use super::ui;

/// A capture frame is ready to be drained.
pub(super) const WM_APP_FRAME: u32 = WM_APP + 1;

const CLASS_NAME: PCWSTR = w!("EasyVirtualDisplayViewerWindow");
const MIN_CLIENT: (i32, i32) = (480, 270);
/// Largest share of the target work area a new viewer may cover.
const INITIAL_WORK_AREA_SHARE: f32 = 0.8;

/// Build a viewer for `target` and show it. Runs on the UI thread.
pub(super) fn create(
    target: ViewerTarget,
    gpu: Rc<Gpu>,
) -> Result<HWND, EasyVirtualDisplayError> {
    let monitor = monitor::find_by_device_name(&target.device_name).ok_or_else(|| {
        EasyVirtualDisplayError::new(
            EasyVirtualDisplayErrorCode::DisplayNotFound,
            format!(
                "Display {} is not attached to the desktop, so there is nothing to mirror.",
                target.device_name
            ),
        )
    })?;

    let content = monitor.size();
    let hwnd = create_window(content, monitor).map_err(start_failed)?;

    let built = Renderer::new(gpu, hwnd, client_size(hwnd)).and_then(|renderer| {
        let capture = Capture::start(renderer.device(), monitor.handle, hwnd)?;
        Ok((renderer, capture))
    });
    let (renderer, capture) = match built {
        Ok(parts) => parts,
        Err(error) => {
            // No state is attached yet, so this destroy is invisible to the UI thread.
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            return Err(start_failed(error));
        }
    };

    let state = ViewerState {
        target,
        monitor,
        content,
        capture,
        renderer,
    };
    let caption = state.caption(None);
    // Frames are only ever *posted*, so none can be handled before the state is attached.
    let cell = Box::into_raw(Box::new(RefCell::new(state)));
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, cell as isize);
        set_title(hwnd, &caption);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(hwnd)
}

/// Bring an existing viewer to the front (a second "open" on an already-open display).
pub(super) fn focus(hwnd: HWND) {
    unsafe {
        // SW_RESTORE also un-maximizes, so only use it to undo minimization.
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}

pub(super) fn set_captions(hwnd: HWND, captions: ViewerCaptions) {
    with_state(hwnd, |state| state.target.captions = captions);
    refresh_title(hwnd);
}

/// Recompute the caption, which reflects resolution and grab state.
pub(super) fn refresh_title(hwnd: HWND) {
    let label = ui::with(|ui| ui.grab_label_for(hwnd)).flatten();
    if let Some(caption) = with_state(hwnd, |state| state.caption(label)) {
        set_title(hwnd, &caption);
    }
}

/// Borrow the state attached to `hwnd`. `None` before it is attached, after it is torn
/// down, or if it is already borrowed further up the stack.
fn with_state<R>(hwnd: HWND, f: impl FnOnce(&mut ViewerState) -> R) -> Option<R> {
    let cell = unsafe { state_cell(hwnd) }?;
    let mut state = cell.try_borrow_mut().ok()?;
    Some(f(&mut state))
}

/// # Safety
/// `hwnd` must be a window of this module's class owned by the calling thread.
unsafe fn state_cell<'a>(hwnd: HWND) -> Option<&'a RefCell<ViewerState>> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<ViewerState>;
    ptr.as_ref()
}

struct ViewerState {
    target: ViewerTarget,
    monitor: MonitorTarget,
    /// Live resolution of the mirrored display, tracked from the capture frames.
    content: (u32, u32),
    // Declaration order is drop order: the capture session must stop before the D3D
    // resources it was created from go away.
    capture: Capture,
    renderer: Renderer,
}

enum FrameAction {
    None,
    /// The mirrored display changed resolution; `rect` is its new desktop rectangle.
    Resized { rect: Option<RECT> },
    Close,
}

impl ViewerState {
    fn caption(&self, grab_label: Option<&str>) -> String {
        let captions = &self.target.captions;
        let title = format_template(
            &captions.title_template,
            &[("index", &self.target.index.to_string())],
        );
        let head = format!("{} · {}×{}", title, self.content.0, self.content.1);
        let hint = match grab_label {
            Some(label) => format_template(&captions.hint_grabbed, &[("hotkey", label)]),
            None => captions.hint_idle.clone(),
        };
        format!("{head}  —  {hint}")
    }

    /// Drain every queued frame and draw the newest one.
    fn on_frame(&mut self) -> FrameAction {
        let Some(frame) = self.capture.drain_latest() else {
            return FrameAction::None;
        };

        let capture_size = frame.size;
        let size = (
            capture_size.Width.max(1) as u32,
            capture_size.Height.max(1) as u32,
        );
        if size != self.content {
            // `ContentSize` follows the capture item, not the pool's textures. If the pool
            // is left at the old size, the next frame passes the check above and the copy
            // silently no-ops — a permanently frozen window. So only commit the new size
            // once the pool really was recreated, and retry on the next frame otherwise.
            drop(frame);
            if self.capture.recreate(capture_size).is_err() {
                return FrameAction::None;
            }
            self.content = size;
            // Frames already in flight are the old size; the recreated pool delivers the
            // new one on the next tick.
            return FrameAction::Resized {
                rect: self.refresh_monitor(),
            };
        }

        match self.renderer.present(&frame.texture) {
            Ok(()) => FrameAction::None,
            Err(_) => FrameAction::Close,
        }
    }

    /// Re-resolve the mirrored display, which may have moved on the virtual desktop.
    fn refresh_monitor(&mut self) -> Option<RECT> {
        let found = monitor::find_by_device_name(&self.target.device_name)?;
        self.monitor = found;
        Some(found.rect)
    }

    fn on_resize(&mut self, width: u32, height: u32) -> FrameAction {
        if width == 0 || height == 0 {
            return FrameAction::None;
        }
        if self.renderer.resize(width, height).is_err() {
            return FrameAction::Close;
        }
        match self.renderer.redraw() {
            Ok(()) => FrameAction::None,
            Err(_) => FrameAction::Close,
        }
    }

    /// What a click at `client` (inside the client area) should grab: the display's GDI
    /// name, its current desktop rectangle, and the matching point to park the cursor on.
    fn grab_target(&mut self, client: POINT) -> Option<(String, RECT, POINT)> {
        // A display can move on the virtual desktop without changing resolution, so the
        // capture stream alone cannot tell us that the cached rectangle is stale.
        let rect = self.refresh_monitor()?;
        let seed = self.client_to_display(client);
        Some((self.target.device_name.clone(), rect, seed))
    }

    /// Map a point in the window's client area onto the mirrored display's desktop
    /// coordinates, accounting for the letterbox.
    fn client_to_display(&self, point: POINT) -> POINT {
        // Map against what is actually on screen, which can lag `self.content` by a frame
        // right after a resolution change.
        let drawn = self.renderer.content_size().unwrap_or(self.content);
        let (fit_x, fit_y, fit_w, fit_h) = renderer::fit_rect(drawn, self.renderer.back_size());
        let nx = ((point.x as f32 - fit_x) / fit_w.max(1.0)).clamp(0.0, 1.0);
        let ny = ((point.y as f32 - fit_y) / fit_h.max(1.0)).clamp(0.0, 1.0);
        let (mw, mh) = self.monitor.size();
        POINT {
            x: self.monitor.rect.left + (nx * (mw - 1) as f32).round() as i32,
            y: self.monitor.rect.top + (ny * (mh - 1) as f32).round() as i32,
        }
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Stateless messages first: some arrive during CreateWindowExW, before state exists.
    match msg {
        WM_GETMINMAXINFO => {
            let info = &mut *(lparam.0 as *mut MINMAXINFO);
            let (w, h) = window_size_for_client(MIN_CLIENT.0, MIN_CLIENT.1);
            info.ptMinTrackSize.x = w;
            info.ptMinTrackSize.y = h;
            return LRESULT(0);
        }
        WM_DPICHANGED => {
            let suggested = &*(lparam.0 as *const RECT);
            let _ = SetWindowPos(
                hwnd,
                None,
                suggested.left,
                suggested.top,
                suggested.right - suggested.left,
                suggested.bottom - suggested.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            return LRESULT(0);
        }
        WM_ERASEBKGND => return LRESULT(1),
        _ => {}
    }

    let Some(cell) = state_cell(hwnd) else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };

    match msg {
        WM_APP_FRAME => {
            match cell.try_borrow_mut() {
                Ok(mut state) => {
                    let action = state.on_frame();
                    drop(state);
                    match action {
                        FrameAction::None => {}
                        FrameAction::Resized { rect } => {
                            if let Some(rect) = rect {
                                ui::with(|ui| ui.update_grab_rect(hwnd, rect));
                            }
                            refresh_title(hwnd);
                        }
                        FrameAction::Close => {
                            let _ = DestroyWindow(hwnd);
                        }
                    }
                }
                Err(_) => {
                    // This message is no longer queued, but the capture-side pending flag
                    // is still set. Preserve that single notification until state is free.
                    let _ = PostMessageW(hwnd, WM_APP_FRAME, WPARAM(0), LPARAM(0));
                }
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let _ = ValidateRect(hwnd, None);
            if let Ok(mut state) = cell.try_borrow_mut() {
                let _ = state.renderer.redraw();
            }
            LRESULT(0)
        }
        WM_SIZE => {
            if let Ok(mut state) = cell.try_borrow_mut() {
                let action = state.on_resize(loword(lparam) as u32, hiword(lparam) as u32);
                drop(state);
                if matches!(action, FrameAction::Close) {
                    let _ = DestroyWindow(hwnd);
                }
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let point = POINT {
                x: loword(lparam),
                y: hiword(lparam),
            };
            let target = cell
                .try_borrow_mut()
                .ok()
                .and_then(|mut state| state.grab_target(point));
            if let Some((device_name, rect, seed)) = target {
                ui::with(|ui| ui.engage(hwnd, &device_name, rect, seed));
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            ui::with(|ui| ui.release_grab_held_by(hwnd));
            LRESULT(0)
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            // Dropping the state stops the capture session while the handle its callbacks
            // post to is still valid. A state still borrowed further up the stack is
            // leaked instead of being freed under that borrow.
            if cell.try_borrow_mut().is_ok() {
                drop(Box::from_raw(std::ptr::from_ref(cell).cast_mut()));
            }
            ui::with(|ui| ui.forget(hwnd));
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn create_window(content: (u32, u32), captured: MonitorTarget) -> windows::core::Result<HWND> {
    let instance: HINSTANCE = unsafe { GetModuleHandleW(None)? }.into();
    register_class(instance)?;

    let (work, scale) = match monitor::find_work_area_excluding(captured.handle) {
        Some((handle, work)) => (work, monitor::scale_factor(handle)),
        None => (captured.work, monitor::scale_factor(captured.handle)),
    };
    let (client_w, client_h) = initial_client_size(content, work, scale);
    let (window_w, window_h) = window_size_for_client(client_w, client_h);
    let x = work.left + ((work.right - work.left - window_w) / 2).max(0);
    let y = work.top + ((work.bottom - work.top - window_h) / 2).max(0);

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_APPWINDOW,
            CLASS_NAME,
            w!("Easy Virtual Display"),
            WS_OVERLAPPEDWINDOW,
            x,
            y,
            window_w,
            window_h,
            None,
            HMENU::default(),
            instance,
            None,
        )?
    };
    Ok(hwnd)
}

fn register_class(instance: HINSTANCE) -> windows::core::Result<()> {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Once;

    static REGISTER: Once = Once::new();
    /// Win32 error from the one registration attempt, or 0 for success. Cached across
    /// calls: `call_once` runs the closure exactly once, so a local would report every
    /// later viewer as successfully registered and hide the real failure behind an
    /// unrelated `CreateWindowExW` error.
    static CLASS_ERROR: AtomicU32 = AtomicU32::new(0);

    REGISTER.call_once(|| unsafe {
        let icon = LoadIconW(instance, PCWSTR(1 as *const u16))
            .or_else(|_| LoadIconW(None, IDI_APPLICATION))
            .unwrap_or_default();
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hIcon: icon,
            hIconSm: icon,
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };
        if RegisterClassExW(&class) == 0 {
            // Clamp to a non-zero code so "failed" never reads back as "succeeded".
            CLASS_ERROR.store(GetLastError().0.max(1), Ordering::SeqCst);
        }
    });

    match CLASS_ERROR.load(Ordering::SeqCst) {
        0 => Ok(()),
        code => Err(windows::core::Error::from(HRESULT::from_win32(code))),
    }
}

/// Initial client size, in physical pixels, for mirroring `content` onto a work area with
/// the given DPI `scale`. The mirrored display opens at the size its pixels would have
/// at that scale — a 1080p display on a 200% 4K screen asks for 3840×2160, just like any
/// other DPI-aware window — then shrinks to fit the work area.
fn initial_client_size(content: (u32, u32), work: RECT, scale: f32) -> (i32, i32) {
    let max_w = ((work.right - work.left) as f32 * INITIAL_WORK_AREA_SHARE)
        .max(MIN_CLIENT.0 as f32);
    let max_h = ((work.bottom - work.top) as f32 * INITIAL_WORK_AREA_SHARE)
        .max(MIN_CLIENT.1 as f32);
    let (content_w, content_h) = (content.0.max(1) as f32, content.1.max(1) as f32);
    let scale = scale
        .max(1.0)
        .min(max_w / content_w)
        .min(max_h / content_h);
    (
        ((content_w * scale) as i32).max(MIN_CLIENT.0),
        ((content_h * scale) as i32).max(MIN_CLIENT.1),
    )
}

fn window_size_for_client(client_w: i32, client_h: i32) -> (i32, i32) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: client_w,
        bottom: client_h,
    };
    unsafe {
        let _ = AdjustWindowRectEx(&mut rect, WS_OVERLAPPEDWINDOW, false, WS_EX_APPWINDOW);
    }
    (rect.right - rect.left, rect.bottom - rect.top)
}

fn client_size(hwnd: HWND) -> (u32, u32) {
    let mut rect = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rect);
    }
    (
        (rect.right - rect.left).max(1) as u32,
        (rect.bottom - rect.top).max(1) as u32,
    )
}

fn set_title(hwnd: HWND, text: &str) {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR(wide.as_ptr()));
    }
}

pub(super) fn start_failed(error: windows::core::Error) -> EasyVirtualDisplayError {
    EasyVirtualDisplayError::new(
        EasyVirtualDisplayErrorCode::ViewerStartFailed,
        format!("Could not start the display viewer: {error}"),
    )
}

fn loword(lparam: LPARAM) -> i32 {
    (lparam.0 & 0xFFFF) as u16 as i16 as i32
}

fn hiword(lparam: LPARAM) -> i32 {
    ((lparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(width: i32, height: i32) -> RECT {
        RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        }
    }

    #[test]
    fn initial_size_is_one_to_one_at_100_percent() {
        assert_eq!(
            initial_client_size((1920, 1080), work(2560, 1400), 1.0),
            (1920, 1080)
        );
    }

    #[test]
    fn initial_size_follows_dpi_scale_on_a_4k_screen() {
        // 2× would be 3840×2160; 80% of a 3840×2100 work area caps it at 1680 tall.
        let (w, h) = initial_client_size((1920, 1080), work(3840, 2100), 2.0);
        assert_eq!(h, 1680);
        assert_eq!(w, (1920.0 * (1680.0 / 1080.0)) as i32);
    }

    #[test]
    fn initial_size_scales_up_to_the_dpi_factor_when_room_allows() {
        assert_eq!(
            initial_client_size((1280, 720), work(3840, 2100), 1.5),
            (1920, 1080)
        );
    }
}
