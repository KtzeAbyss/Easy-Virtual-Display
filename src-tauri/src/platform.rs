//! OS capability checks for the main window chrome.

/// First Windows 11 build; Mica (`DWMSBT_MAINWINDOW`) is not available below it.
#[cfg(windows)]
const WINDOWS_11_FIRST_BUILD: u32 = 22000;

/// `RtlGetVersion` is used instead of `GetVersionExW`, which reports whatever the
/// executable manifest claims rather than the real OS build.
#[cfg(windows)]
pub fn supports_mica() -> bool {
    use windows::Wdk::System::SystemServices::RtlGetVersion;
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;

    let mut info = OSVERSIONINFOW {
        dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
        ..Default::default()
    };
    let status = unsafe { RtlGetVersion(&mut info) };
    status.is_ok() && info.dwMajorVersion >= 10 && info.dwBuildNumber >= WINDOWS_11_FIRST_BUILD
}

#[cfg(not(windows))]
pub fn supports_mica() -> bool {
    false
}
