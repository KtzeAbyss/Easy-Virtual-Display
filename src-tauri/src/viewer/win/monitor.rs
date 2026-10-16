//! Resolve the GDI adapter name the .NET host reports (`\\.\DISPLAY3`) to the `HMONITOR`
//! and desktop rectangle Windows.Graphics.Capture needs.

use std::mem::size_of;

use windows::Win32::Foundation::{BOOL, FALSE, LPARAM, POINT, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromPoint, HDC, HMONITOR, MONITORINFOEXW,
    MONITOR_DEFAULTTONULL,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// `USER_DEFAULT_SCREEN_DPI`: the DPI that corresponds to 100% scaling.
const BASE_DPI: f32 = 96.0;

#[derive(Clone, Copy)]
pub(super) struct MonitorTarget {
    pub handle: HMONITOR,
    /// Position and size on the virtual desktop, in physical pixels.
    pub rect: RECT,
    /// Usable area after excluding taskbars and docked windows.
    pub work: RECT,
}

impl MonitorTarget {
    pub fn size(&self) -> (u32, u32) {
        (
            (self.rect.right - self.rect.left).max(1) as u32,
            (self.rect.bottom - self.rect.top).max(1) as u32,
        )
    }
}

struct Search<'a> {
    wanted: &'a str,
    found: Option<MonitorTarget>,
}

/// `None` when the display is attached but not active (no monitor rectangle yet), or when
/// the name simply doesn't exist.
pub(super) fn find_by_device_name(device_name: &str) -> Option<MonitorTarget> {
    let mut search = Search {
        wanted: device_name,
        found: None,
    };
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_proc),
            LPARAM(&mut search as *mut Search as isize),
        );
    }
    search.found
}

/// Effective scale factor of a monitor (1.0 at 100%, 2.0 at 200%).
pub(super) fn scale_factor(handle: HMONITOR) -> f32 {
    let (mut dpi_x, mut dpi_y) = (0, 0);
    match unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) } {
        Ok(()) if dpi_x > 0 => dpi_x as f32 / BASE_DPI,
        _ => 1.0,
    }
}

struct WorkAreaSearch {
    excluded: HMONITOR,
    found: Option<(HMONITOR, RECT)>,
}

/// Select a monitor, and its work area, that is not the captured display. The cursor's
/// monitor is the most useful default because it is normally where the user opened the
/// viewer.
pub(super) fn find_work_area_excluding(excluded: HMONITOR) -> Option<(HMONITOR, RECT)> {
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) }.is_ok() {
        let handle = unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONULL) };
        if handle != excluded {
            if let Some(info) = unsafe { monitor_info(handle) } {
                return Some((handle, info.monitorInfo.rcWork));
            }
        }
    }

    let mut search = WorkAreaSearch {
        excluded,
        found: None,
    };
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_work_area),
            LPARAM(&mut search as *mut WorkAreaSearch as isize),
        );
    }
    search.found
}

unsafe extern "system" fn enum_proc(
    handle: HMONITOR,
    _hdc: HDC,
    _clip: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let search = &mut *(lparam.0 as *mut Search);

    let Some(info) = monitor_info(handle) else {
        return TRUE;
    };

    let len = info
        .szDevice
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(info.szDevice.len());
    let name = String::from_utf16_lossy(&info.szDevice[..len]);

    if name.eq_ignore_ascii_case(search.wanted) {
        search.found = Some(MonitorTarget {
            handle,
            rect: info.monitorInfo.rcMonitor,
            work: info.monitorInfo.rcWork,
        });
        return FALSE; // stop enumerating
    }

    TRUE
}

unsafe extern "system" fn enum_work_area(
    handle: HMONITOR,
    _hdc: HDC,
    _clip: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let search = &mut *(lparam.0 as *mut WorkAreaSearch);
    if handle == search.excluded {
        return TRUE;
    }
    if let Some(info) = monitor_info(handle) {
        search.found = Some((handle, info.monitorInfo.rcWork));
        return FALSE;
    }
    TRUE
}

unsafe fn monitor_info(handle: HMONITOR) -> Option<MONITORINFOEXW> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    GetMonitorInfoW(handle, &mut info.monitorInfo)
        .as_bool()
        .then_some(info)
}
