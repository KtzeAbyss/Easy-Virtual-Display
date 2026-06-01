//! Prevent display sleep while a virtual display is in use.
//!
//! Windows: `SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED)`
//! sets the state on a thread basis — the state persists *only* while the calling thread is
//! alive. So we own a dedicated long-lived OS thread that processes enable/disable
//! messages from an mpsc channel; the boundary just sends.
//!
//! Non-Windows: a no-op stub. Phase 4 is Windows-first per the migration spec; macOS will
//! plug in `IOPMAssertion` later in its own cfg-gated module.

use std::sync::Arc;

pub trait KeepScreenOnBoundary: Send + Sync {
    fn sync(&self, enabled: bool);
}

pub fn create() -> Arc<dyn KeepScreenOnBoundary> {
    #[cfg(target_os = "windows")]
    {
        Arc::new(windows_impl::WindowsKeepScreenOn::new())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Arc::new(noop_impl::Noop)
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::sync::mpsc::{self, Sender};
    use windows::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
    };

    /// The boundary holds only the sender — the receiver runs on a dedicated thread
    /// (see [`spawn_worker`]) so SetThreadExecutionState's per-thread state stays valid
    /// for the entire app lifetime.
    pub struct WindowsKeepScreenOn {
        tx: Sender<bool>,
    }

    impl WindowsKeepScreenOn {
        pub fn new() -> Self {
            let tx = spawn_worker();
            Self { tx }
        }
    }

    impl super::KeepScreenOnBoundary for WindowsKeepScreenOn {
        fn sync(&self, enabled: bool) {
            let _ = self.tx.send(enabled);
        }
    }

    fn spawn_worker() -> Sender<bool> {
        let (tx, rx) = mpsc::channel::<bool>();
        std::thread::Builder::new()
            .name("keep-screen-on".into())
            .spawn(move || {
                let mut active = false;
                while let Ok(enabled) = rx.recv() {
                    if enabled == active {
                        continue;
                    }
                    unsafe {
                        if enabled {
                            SetThreadExecutionState(
                                ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED,
                            );
                        } else {
                            SetThreadExecutionState(ES_CONTINUOUS);
                        }
                    }
                    active = enabled;
                }
            })
            .expect("keep-screen-on worker thread must spawn");
        tx
    }
}

#[cfg(not(target_os = "windows"))]
mod noop_impl {
    pub struct Noop;
    impl super::KeepScreenOnBoundary for Noop {
        fn sync(&self, _enabled: bool) {}
    }
}
