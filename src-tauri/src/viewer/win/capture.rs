//! Windows.Graphics.Capture session bound to a single monitor.
//!
//! The frame pool is created free-threaded, so `FrameArrived` fires on a thread-pool
//! thread; it only posts `WM_APP_FRAME` to the viewer window and returns. All D3D work
//! then happens on the window thread, which keeps the device context single-threaded.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use windows::core::{IInspectable, Interface, Result as WinResult};
use windows::Foundation::{EventRegistrationToken, TypedEventHandler};
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::{E_NOTIMPL, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11Texture2D};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

use super::window::WM_APP_FRAME;
use super::{hwnd_from_raw, hwnd_raw};

/// Two buffers is what the WGC samples use: enough to decouple producer and consumer
/// without adding a frame of latency.
const BUFFER_COUNT: i32 = 2;
const PIXEL_FORMAT: DirectXPixelFormat = DirectXPixelFormat::B8G8R8A8UIntNormalized;

/// A frame borrowed from the pool. The underlying texture stays valid only while this
/// value is alive, so callers copy out of it before dropping.
pub(super) struct CapturedFrame {
    pub texture: ID3D11Texture2D,
    pub size: SizeInt32,
    _frame: Direct3D11CaptureFrame,
}

pub(super) struct Capture {
    item: GraphicsCaptureItem,
    pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    device: IDirect3DDevice,
    frame_token: EventRegistrationToken,
    closed_token: EventRegistrationToken,
    frame_message_pending: Arc<AtomicBool>,
    stopped: Cell<bool>,
}

impl Capture {
    pub fn start(d3d_device: &ID3D11Device, monitor: HMONITOR, hwnd: HWND) -> WinResult<Self> {
        // Windows.Graphics.Capture needs Windows 10 1803+. Checking up front turns a
        // confusing mid-setup failure into one clear message.
        if !GraphicsCaptureSession::IsSupported().unwrap_or(false) {
            return Err(windows::core::Error::new(
                E_NOTIMPL,
                "This version of Windows does not support Windows.Graphics.Capture.",
            ));
        }

        let dxgi_device: IDXGIDevice = d3d_device.cast()?;
        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)? };
        let device: IDirect3DDevice = inspectable.cast()?;

        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let item: GraphicsCaptureItem = unsafe { interop.CreateForMonitor(monitor)? };

        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &device,
            PIXEL_FORMAT,
            BUFFER_COUNT,
            item.Size()?,
        )?;

        let target = hwnd_raw(hwnd);
        let frame_message_pending = Arc::new(AtomicBool::new(false));
        let callback_pending = frame_message_pending.clone();
        let frame_token = pool.FrameArrived(&TypedEventHandler::<
            Direct3D11CaptureFramePool,
            IInspectable,
        >::new(move |_, _| {
            if !callback_pending.swap(true, Ordering::AcqRel) {
                let posted = unsafe {
                    PostMessageW(hwnd_from_raw(target), WM_APP_FRAME, WPARAM(0), LPARAM(0))
                };
                if posted.is_err() {
                    callback_pending.store(false, Ordering::Release);
                }
            }
            Ok(())
        }))?;

        let closed_token = item.Closed(
            &TypedEventHandler::<GraphicsCaptureItem, IInspectable>::new(move |_, _| {
                unsafe {
                    let _ = PostMessageW(hwnd_from_raw(target), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
                Ok(())
            }),
        )?;

        let session = pool.CreateCaptureSession(&item)?;
        // The mirrored desktop should show its own cursor; the yellow capture border is
        // noise. `SetIsBorderRequired` only exists on Windows 11 / 10 21H2+ — older builds
        // return an error we can safely ignore.
        let _ = session.SetIsCursorCaptureEnabled(true);
        let _ = session.SetIsBorderRequired(false);
        session.StartCapture()?;

        Ok(Self {
            item,
            pool,
            session,
            device,
            frame_token,
            closed_token,
            frame_message_pending,
            stopped: Cell::new(false),
        })
    }

    /// Detach the event handlers and end the session. Runs from `Drop`, which the viewer
    /// reaches in `WM_NCDESTROY` while its handle is still valid, so no callback can post
    /// to a window handle Windows may already have recycled.
    pub fn stop(&self) {
        if self.stopped.replace(true) {
            return;
        }
        let _ = self.pool.RemoveFrameArrived(self.frame_token);
        let _ = self.item.RemoveClosed(self.closed_token);
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }

    /// Drains the pool and returns its newest frame while allowing at most one more frame
    /// message to be queued. The second drain closes the race with callbacks that observed
    /// the old pending flag immediately before it was cleared.
    pub fn drain_latest(&self) -> Option<CapturedFrame> {
        let mut latest = None;
        while let Some(frame) = self.next_frame() {
            latest = Some(frame);
        }

        self.frame_message_pending.store(false, Ordering::Release);

        while let Some(frame) = self.next_frame() {
            latest = Some(frame);
        }
        latest
    }

    /// Pops one frame, or `None` when the pool is empty.
    fn next_frame(&self) -> Option<CapturedFrame> {
        let frame = self.pool.TryGetNextFrame().ok()?;
        let size = frame.ContentSize().ok()?;
        let surface = frame.Surface().ok()?;
        let access: IDirect3DDxgiInterfaceAccess = surface.cast().ok()?;
        let texture: ID3D11Texture2D = unsafe { access.GetInterface().ok()? };
        Some(CapturedFrame {
            texture,
            size,
            _frame: frame,
        })
    }

    /// Re-sizes the pool after the mirrored display changes resolution.
    pub fn recreate(&self, size: SizeInt32) -> WinResult<()> {
        self.pool
            .Recreate(&self.device, PIXEL_FORMAT, BUFFER_COUNT, size)
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        self.stop();
    }
}
