//! D3D11 presentation for the viewer windows.
//!
//! [`Gpu`] is the device, immediate context and pipeline state shared by every viewer on
//! the UI thread — shaders are compiled once, not per window. Each window's [`Renderer`]
//! owns a flip-model swap chain sized to its client area, plus a fullscreen-triangle draw
//! that scales the captured frame into an aspect-preserving letterbox. The captured
//! texture is copied into a private shader-resource texture first: the frame pool hands
//! out recycled textures whose bind flags we don't control, and the copy keeps the draw
//! independent of the pool's lifetime.

use std::rc::Rc;

use windows::core::{s, Interface, Result as WinResult};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D::Fxc::{D3DCompile, D3DCOMPILE_OPTIMIZATION_LEVEL3};
use windows::Win32::Graphics::Direct3D::{ID3DBlob, D3D_DRIVER_TYPE_HARDWARE};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11PixelShader,
    ID3D11RenderTargetView, ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D,
    ID3D11VertexShader, D3D11_BIND_SHADER_RESOURCE, D3D11_COMPARISON_ALWAYS,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_SAMPLER_DESC,
    D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DEFAULT,
    D3D11_VIEWPORT,
};
use windows::Win32::Graphics::Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST;
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIDevice, IDXGIFactory2, IDXGISwapChain1, DXGI_MWA_NO_ALT_ENTER, DXGI_PRESENT,
    DXGI_SCALING_NONE, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG,
    DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};

const SHADER_SOURCE: &[u8] = br#"
Texture2D    srcTexture : register(t0);
SamplerState srcSampler : register(s0);

struct VsOut {
    float4 pos : SV_POSITION;
    float2 uv  : TEXCOORD0;
};

// Fullscreen triangle generated from SV_VertexID: no vertex or index buffer needed.
VsOut vs_main(uint id : SV_VertexID) {
    VsOut o;
    float2 uv = float2((id << 1) & 2, id & 2);
    o.uv = uv;
    o.pos = float4(uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
    return o;
}

float4 ps_main(VsOut input) : SV_TARGET {
    return float4(srcTexture.Sample(srcSampler, input.uv).rgb, 1.0);
}
"#;

/// Private copy of the latest captured frame, kept at the mirrored display's resolution.
struct FrameSurface {
    texture: ID3D11Texture2D,
    view: ID3D11ShaderResourceView,
    size: (u32, u32),
}

/// Single-threaded by construction: it lives on, and is only used from, the UI thread.
pub(super) struct Gpu {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    vertex_shader: ID3D11VertexShader,
    pixel_shader: ID3D11PixelShader,
    sampler: ID3D11SamplerState,
}

impl Gpu {
    pub fn new() -> WinResult<Self> {
        let (device, context) = create_device()?;

        let vs_blob = compile(s!("vs_main"), s!("vs_5_0"))?;
        let ps_blob = compile(s!("ps_main"), s!("ps_5_0"))?;

        let mut vertex_shader = None;
        let mut pixel_shader = None;
        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            MaxLOD: f32::MAX,
            ..Default::default()
        };
        let mut sampler = None;

        unsafe {
            device.CreateVertexShader(blob_bytes(&vs_blob), None, Some(&mut vertex_shader))?;
            device.CreatePixelShader(blob_bytes(&ps_blob), None, Some(&mut pixel_shader))?;
            device.CreateSamplerState(&sampler_desc, Some(&mut sampler))?;
        }

        Ok(Self {
            device,
            context,
            vertex_shader: vertex_shader.expect("CreateVertexShader returned success with no shader"),
            pixel_shader: pixel_shader.expect("CreatePixelShader returned success with no shader"),
            sampler: sampler.expect("CreateSamplerState returned success with no sampler"),
        })
    }

    /// A removed device (driver update, TDR) cannot be revived; the next viewer gets a
    /// fresh one while the windows still bound to this one close on their next present.
    pub fn is_lost(&self) -> bool {
        unsafe { self.device.GetDeviceRemovedReason() }.is_err()
    }
}

pub(super) struct Renderer {
    gpu: Rc<Gpu>,
    swap_chain: IDXGISwapChain1,
    target: Option<ID3D11RenderTargetView>,
    frame: Option<FrameSurface>,
    back_size: (u32, u32),
}

impl Renderer {
    pub fn new(gpu: Rc<Gpu>, hwnd: HWND, client: (u32, u32)) -> WinResult<Self> {
        let swap_chain = create_swap_chain(&gpu.device, hwnd, client)?;
        let mut renderer = Self {
            gpu,
            swap_chain,
            target: None,
            frame: None,
            back_size: client,
        };
        renderer.create_target()?;
        Ok(renderer)
    }

    pub fn device(&self) -> &ID3D11Device {
        &self.gpu.device
    }

    pub fn back_size(&self) -> (u32, u32) {
        self.back_size
    }

    pub fn resize(&mut self, width: u32, height: u32) -> WinResult<()> {
        if width == 0 || height == 0 || (width, height) == self.back_size {
            return Ok(());
        }
        self.target = None;
        unsafe {
            self.gpu.context.OMSetRenderTargets(None, None);
            self.swap_chain.ResizeBuffers(
                0,
                width,
                height,
                DXGI_FORMAT_UNKNOWN,
                DXGI_SWAP_CHAIN_FLAG(0),
            )?;
        }
        self.back_size = (width, height);
        self.create_target()
    }

    /// Copy `source` into our private texture and draw it.
    ///
    /// The size is read back from the source texture rather than taken from the caller's
    /// idea of the display resolution: `CopyResource` returns `void` and silently does
    /// nothing when the two resources disagree, so deriving it here makes that class of
    /// mismatch impossible.
    pub fn present(&mut self, source: &ID3D11Texture2D) -> WinResult<()> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { source.GetDesc(&mut desc) };

        self.ensure_frame((desc.Width.max(1), desc.Height.max(1)))?;
        let Some(frame) = self.frame.as_ref() else {
            return Ok(());
        };
        unsafe { self.gpu.context.CopyResource(&frame.texture, source) };
        self.draw()
    }

    /// Size of the frame currently on screen, which is what the letterbox is computed
    /// from — input mapping uses it so a click always lands where the user pointed.
    pub fn content_size(&self) -> Option<(u32, u32)> {
        self.frame.as_ref().map(|frame| frame.size)
    }

    /// Re-present the frame we already hold. Windows.Graphics.Capture only delivers a
    /// frame when the mirrored desktop changes, so a resize or repaint of a static screen
    /// would otherwise leave the window blank.
    pub fn redraw(&mut self) -> WinResult<()> {
        if self.frame.is_none() {
            return Ok(());
        }
        self.draw()
    }

    fn draw(&mut self) -> WinResult<()> {
        let Some(frame) = self.frame.as_ref() else {
            return Ok(());
        };
        let Some(target) = self.target.clone() else {
            return Ok(());
        };

        let (fit_x, fit_y, fit_w, fit_h) = fit_rect(frame.size, self.back_size);
        let viewport = D3D11_VIEWPORT {
            TopLeftX: fit_x,
            TopLeftY: fit_y,
            Width: fit_w,
            Height: fit_h,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };

        // The context is shared, so every draw binds its complete pipeline state.
        let gpu = &self.gpu;
        unsafe {
            gpu.context
                .ClearRenderTargetView(&target, &[0.0, 0.0, 0.0, 1.0]);
            gpu.context
                .OMSetRenderTargets(Some(&[Some(target.clone())]), None);
            gpu.context.RSSetViewports(Some(&[viewport]));
            gpu.context.IASetInputLayout(None);
            gpu.context
                .IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            gpu.context.VSSetShader(&gpu.vertex_shader, None);
            gpu.context.PSSetShader(&gpu.pixel_shader, None);
            gpu.context
                .PSSetShaderResources(0, Some(&[Some(frame.view.clone())]));
            gpu.context
                .PSSetSamplers(0, Some(&[Some(gpu.sampler.clone())]));
            gpu.context.Draw(3, 0);
            // Unbind so the next CopyResource isn't blocked by the SRV still being bound,
            // and so another window's ResizeBuffers never finds our target still bound.
            gpu.context.PSSetShaderResources(0, Some(&[None]));
            gpu.context.OMSetRenderTargets(None, None);

            self.swap_chain.Present(1, DXGI_PRESENT(0)).ok()?;
        }
        Ok(())
    }

    fn ensure_frame(&mut self, content: (u32, u32)) -> WinResult<()> {
        if self.frame.as_ref().is_some_and(|f| f.size == content) {
            return Ok(());
        }
        let desc = D3D11_TEXTURE2D_DESC {
            Width: content.0,
            Height: content.1,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            ..Default::default()
        };

        let mut texture = None;
        unsafe { self.gpu.device.CreateTexture2D(&desc, None, Some(&mut texture))? };
        let texture = texture.expect("CreateTexture2D returned success with no texture");

        let mut view = None;
        unsafe {
            self.gpu
                .device
                .CreateShaderResourceView(&texture, None, Some(&mut view))?
        };
        let view = view.expect("CreateShaderResourceView returned success with no view");

        self.frame = Some(FrameSurface {
            texture,
            view,
            size: content,
        });
        Ok(())
    }

    fn create_target(&mut self) -> WinResult<()> {
        let back_buffer: ID3D11Texture2D = unsafe { self.swap_chain.GetBuffer(0)? };
        let mut target = None;
        unsafe {
            self.gpu
                .device
                .CreateRenderTargetView(&back_buffer, None, Some(&mut target))?
        };
        self.target = target;
        Ok(())
    }
}

/// Largest `content`-shaped rectangle that fits inside `client`, centred (letterbox).
/// Shared with the input mapping so a click in the window lands on the matching pixel.
pub(super) fn fit_rect(content: (u32, u32), client: (u32, u32)) -> (f32, f32, f32, f32) {
    let (cw, ch) = (content.0.max(1) as f32, content.1.max(1) as f32);
    let (ww, wh) = (client.0.max(1) as f32, client.1.max(1) as f32);
    let scale = (ww / cw).min(wh / ch);
    let w = cw * scale;
    let h = ch * scale;
    (((ww - w) * 0.5).floor(), ((wh - h) * 0.5).floor(), w, h)
}

fn create_device() -> WinResult<(ID3D11Device, ID3D11DeviceContext)> {
    let mut device = None;
    let mut context = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
    }
    Ok((
        device.expect("D3D11CreateDevice returned success with no device"),
        context.expect("D3D11CreateDevice returned success with no context"),
    ))
}

fn create_swap_chain(
    device: &ID3D11Device,
    hwnd: HWND,
    client: (u32, u32),
) -> WinResult<IDXGISwapChain1> {
    let dxgi_device: IDXGIDevice = device.cast()?;
    let adapter = unsafe { dxgi_device.GetAdapter()? };
    let factory: IDXGIFactory2 = unsafe { adapter.GetParent()? };

    let desc = DXGI_SWAP_CHAIN_DESC1 {
        Width: client.0.max(1),
        Height: client.1.max(1),
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        Scaling: DXGI_SCALING_NONE,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
        ..Default::default()
    };

    let swap_chain = unsafe { factory.CreateSwapChainForHwnd(device, hwnd, &desc, None, None)? };
    // Alt is a normal key for the mirrored desktop — don't let DXGI eat Alt+Enter.
    unsafe { let _ = factory.MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER); }
    Ok(swap_chain)
}

fn compile(entry: windows::core::PCSTR, target: windows::core::PCSTR) -> WinResult<ID3DBlob> {
    let mut code = None;
    let mut errors = None;
    unsafe {
        D3DCompile(
            SHADER_SOURCE.as_ptr() as *const _,
            SHADER_SOURCE.len(),
            s!("viewer.hlsl"),
            None,
            None,
            entry,
            target,
            D3DCOMPILE_OPTIMIZATION_LEVEL3,
            0,
            &mut code,
            Some(&mut errors),
        )?;
    }
    Ok(code.expect("D3DCompile returned success with no bytecode"))
}

fn blob_bytes(blob: &ID3DBlob) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize())
    }
}
