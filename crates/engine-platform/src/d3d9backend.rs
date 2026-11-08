//! The real `d3d9.dll` as a backend of the Direct3D 9 call stream (`d3d9_stream`, ADR-0008):
//! each recorded call carried out on the real device as the forwarding did it on the game's
//! thread (`super::d3d9real`), on whichever thread the stream runs it (the calling one, or the
//! render thread with `NV_RENDER_THREAD=1`).
//!
//! The backend never sees the game's memory or the model: what it needs is in the command. It
//! keeps the real objects by the modelled object's address, prints once each call where the real
//! runtime answers otherwise than the model, skips the draws of a device while a binding has no
//! real counterpart, and keeps destroyed objects until the next `Present` (the game's device is a
//! pure one, which keeps no reference of its own to what is bound: an object released at once
//! would be freed while still bound, and the call that replaces the binding would read it, p169).
//!
//! A texture level the backend keeps (a managed or system-memory level: `d3d9device`'s
//! `Pixels::Real`) answers `ReadSurface` with what it was given; when its real surface is missing
//! or cannot be locked, its bytes stay here instead, so the game reads back what it wrote, as when
//! the model kept them.

use d3d9_stream::{Backend, Cmd, Data, Native, Reply, Rows};
use std::collections::HashSet;
use std::mem::ManuallyDrop;
use windows::Win32::Foundation::{BOOL, HWND, RECT};
use windows::Win32::Graphics::Direct3D9::*;

/// A real Direct3D 9 object, the counterpart of a modelled one.
enum Object {
    Device(IDirect3DDevice9),
    Surface(IDirect3DSurface9),
    Texture(IDirect3DTexture9),
    Cube(IDirect3DCubeTexture9),
    VertexBuffer(IDirect3DVertexBuffer9),
    IndexBuffer(IDirect3DIndexBuffer9),
    VertexShader(IDirect3DVertexShader9),
    PixelShader(IDirect3DPixelShader9),
    Declaration(IDirect3DVertexDeclaration9),
    Query(IDirect3DQuery9),
}

/// The real device handed to the layer's side channel (D3DX's texture loading on the game's
/// threads, `super::d3dxreal::texture_levels`): the device is made with
/// `D3DCREATE_MULTITHREADED` (the game's flags 0x54), so d3d9.dll serialises calls from any
/// thread.
pub struct SharedDevice(pub IDirect3DDevice9);

// SAFETY: a `D3DCREATE_MULTITHREADED` device takes calls from any thread (d3d9.dll locks itself).
unsafe impl Send for SharedDevice {}

/// The real side of the game's Direct3D 9 calls.
#[derive(Default)]
pub struct RealBackend {
    d3d: Option<IDirect3D9>,
    objects: engine_core::AddrMap<u32, Object>,
    /// The device last made, not counted (its entry in `objects` holds the reference): every
    /// command looks its device up, and the game has one.
    device: Option<(u32, ManuallyDrop<IDirect3DDevice9>)>,
    /// The buffers made with `D3DUSAGE_DYNAMIC` (their locks pass `DISCARD`/`NOOVERWRITE` on).
    dynamic: engine_core::AddrSet<u32>,
    /// Bindings without a real counterpart, by (device, binding: a stream number, or
    /// [`INDICES`], [`DECLARATION`], [`VERTEX_SHADER`], [`PIXEL_SHADER`]): the real binding is
    /// null instead, and the device's draws are skipped while any is left.
    broken: engine_core::AddrSet<(u32, u32)>,
    /// Destroyed objects, released at the next `Present`.
    graveyard: Vec<Object>,
    /// Levels whose bytes the real surface could not take (missing, or its lock failed): the
    /// bytes last uploaded, for `ReadSurface`.
    kept: engine_core::AddrMap<u32, Vec<u8>>,
    /// The differences already printed: (call, model's answer, real answer).
    logged: HashSet<(&'static str, u32, u32)>,
}

// SAFETY: the real objects are free-threaded COM objects of a `D3DCREATE_MULTITHREADED` device;
// the backend itself is used by one thread at a time (the stream's).
unsafe impl Send for RealBackend {}

/// The bindings [`RealBackend::broken`] names besides the streams (0..16).
const INDICES: u32 = 0x100;
const DECLARATION: u32 = 0x101;
const VERTEX_SHADER: u32 = 0x102;
const PIXEL_SHADER: u32 = 0x103;

/// `D3DCREATE_FPU_PRESERVE`, added to the game's flags: the game's x87 precision switch is
/// modelled (`crate::direct3d_device_created`), and the thread that runs the backend keeps its
/// own floating-point state.
const FPU_PRESERVE: u32 = 2;
/// `D3DUSAGE_DYNAMIC`, `D3DLOCK_READONLY`, `D3DLOCK_DISCARD`, `D3DLOCK_NOOVERWRITE`.
const USAGE_DYNAMIC: u32 = 0x200;
const LOCK_READONLY: u32 = 0x10;
const LOCK_DISCARD: u32 = 0x2000;
const LOCK_NOOVERWRITE: u32 = 0x1000;
/// `D3DPRESENT_INTERVAL_IMMEDIATE`.
const INTERVAL_IMMEDIATE: u32 = 0x8000_0000;

/// The real device's presentation interval for the game's `interval` (its `iPresentInterval`):
/// the game's, unless `NV_D3D9_PRESENT=immediate` (a measurement setting of the host, not the
/// game's) asks for `D3DPRESENT_INTERVAL_IMMEDIATE`, so a frame rate measures the work and not
/// the display's refresh. The model keeps the game's parameters and answers.
fn presentation_interval(interval: u32) -> u32 {
    static IMMEDIATE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let immediate = *IMMEDIATE.get_or_init(|| {
        std::env::var("NV_D3D9_PRESENT").is_ok_and(|v| v.eq_ignore_ascii_case("immediate"))
    });
    choose_interval(interval, immediate)
}

fn choose_interval(interval: u32, immediate: bool) -> u32 {
    if immediate {
        INTERVAL_IMMEDIATE
    } else {
        interval
    }
}

#[inline]
fn hr(r: windows::core::Result<()>) -> u32 {
    match r {
        Ok(()) => 0,
        Err(err) => err.code().0 as u32,
    }
}

/// A rectangle payload (4 words) as a `RECT`.
fn rect(d: &Option<Data<'_>>) -> Option<RECT> {
    d.as_ref().map(|d| {
        let w = d.to_words();
        RECT {
            left: w[0] as i32,
            top: w[1] as i32,
            right: w[2] as i32,
            bottom: w[3] as i32,
        }
    })
}

fn rect_ptr(r: &Option<RECT>) -> *const RECT {
    r.as_ref().map_or(std::ptr::null(), |r| r as *const RECT)
}

/// The game's `D3DPRESENT_PARAMETERS` (14 words, the 32-bit layout) with the real device window.
fn present_parameters(words: &[u32], window: isize) -> D3DPRESENT_PARAMETERS {
    let w = |i: usize| words.get(i).copied().unwrap_or(0);
    D3DPRESENT_PARAMETERS {
        BackBufferWidth: w(0),
        BackBufferHeight: w(1),
        BackBufferFormat: D3DFORMAT(w(2)),
        BackBufferCount: w(3),
        MultiSampleType: D3DMULTISAMPLE_TYPE(w(4) as i32),
        MultiSampleQuality: w(5),
        SwapEffect: D3DSWAPEFFECT(w(6) as i32),
        hDeviceWindow: HWND(window as *mut _),
        Windowed: BOOL(w(8) as i32),
        EnableAutoDepthStencil: BOOL(w(9) as i32),
        AutoDepthStencilFormat: D3DFORMAT(w(10)),
        Flags: w(11),
        FullScreen_RefreshRateInHz: w(12),
        PresentationInterval: presentation_interval(w(13)),
    }
}

impl RealBackend {
    pub fn new() -> RealBackend {
        RealBackend::default()
    }

    /// Prints, once per kind, a call where the real runtime answers otherwise than the model.
    #[inline]
    fn answer(&mut self, call: &'static str, model: u32, real: u32) {
        if model != real {
            self.differs(call, model, real);
        }
    }

    #[cold]
    #[inline(never)]
    fn differs(&mut self, call: &'static str, model: u32, real: u32) {
        if self.logged.insert((call, model, real)) {
            eprintln!(
                "NV_D3D9=real: {call}: the model answers {model:#010x}, the real runtime {real:#010x}"
            );
        }
    }

    /// A modelled object's real counterpart is missing: printed once by call.
    fn missing(&mut self, call: &'static str) {
        self.answer(call, 0, 0xffff_ffff);
    }

    /// The real device of the modelled device `at`, borrowed (the table holds its reference for
    /// the whole command: only `Destroy` takes it out, into the graveyard).
    #[inline]
    fn device(&self, at: u32) -> Option<ManuallyDrop<IDirect3DDevice9>> {
        let found = match &self.device {
            Some((d, device)) if *d == at => Some(&**device),
            _ => match self.objects.get(&at) {
                Some(Object::Device(d)) => Some(d),
                _ => None,
            },
        };
        // SAFETY: an interface is one non-null COM pointer; the copy is never released.
        found.map(|d| ManuallyDrop::new(unsafe { std::mem::transmute_copy(d) }))
    }

    fn insert(&mut self, at: u32, object: Object) {
        if let Object::Device(d) = &object {
            // SAFETY: as in `device`; the copy goes when the entry does (here and in `destroy`).
            self.device = Some((at, ManuallyDrop::new(unsafe { std::mem::transmute_copy(d) })));
        } else if self.device.as_ref().is_some_and(|(d, _)| *d == at) {
            self.device = None;
        }
        // A counterpart replaced in place (the back buffers after a `Reset`) is released now, as
        // the forwarding always did; only destroyed objects wait for the `Present`.
        self.objects.insert(at, object);
    }

    fn destroy(&mut self, at: u32) {
        if self.device.as_ref().is_some_and(|(d, _)| *d == at) {
            self.device = None;
        }
        if let Some(o) = self.objects.remove(&at) {
            self.graveyard.push(o);
        }
        self.dynamic.remove(&at);
        self.kept.remove(&at);
    }

    fn surface(&self, at: u32) -> Option<IDirect3DSurface9> {
        match self.objects.get(&at) {
            Some(Object::Surface(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn base_texture(&self, at: u32) -> Option<IDirect3DBaseTexture9> {
        use windows::core::Interface;
        match self.objects.get(&at) {
            Some(Object::Texture(t)) => t.cast().ok(),
            Some(Object::Cube(t)) => t.cast().ok(),
            _ => None,
        }
    }

    fn broken(&mut self, device: u32, binding: u32, call: &'static str) {
        self.broken.insert((device, binding));
        self.answer(call, 0, 0xffff_ffff);
    }

    #[inline]
    fn mended(&mut self, device: u32, binding: u32) {
        // Nearly always none is broken: then there is nothing to look up.
        if !self.broken.is_empty() {
            self.broken.remove(&(device, binding));
        }
    }

    /// Whether the real draws of `device` are skipped (a binding without a real counterpart).
    #[inline]
    fn skipped(&mut self, device: u32) -> bool {
        let any = !self.broken.is_empty() && self.broken.iter().any(|&(d, _)| d == device);
        if any {
            self.missing("a draw skipped: a binding has no real counterpart");
        }
        any
    }

    /// The real device's back buffers and depth surface as the counterparts of the modelled ones.
    fn implicit_surfaces(
        &mut self,
        d: &IDirect3DDevice9,
        back_buffers: &[u32],
        depth_stencil: u32,
    ) {
        for (i, &surface) in back_buffers.iter().enumerate() {
            if let Ok(s) = unsafe { d.GetBackBuffer(0, i as u32, D3DBACKBUFFER_TYPE_MONO) } {
                self.insert(surface, Object::Surface(s));
            }
        }
        if depth_stencil != 0 {
            if let Ok(s) = unsafe { d.GetDepthStencilSurface() } {
                self.insert(depth_stencil, Object::Surface(s));
            }
        }
    }

    fn create_device(
        &mut self,
        device: u32,
        focus: isize,
        window: isize,
        behavior: u32,
        present: &[u32],
        back_buffers: &[u32],
        depth_stencil: u32,
    ) -> Reply {
        let none = Reply::Device {
            created: false,
            native: None,
        };
        if self.d3d.is_none() {
            match unsafe { Direct3DCreate9(32) } {
                Some(d3d) => self.d3d = Some(d3d),
                None => {
                    eprintln!("NV_D3D9=real: Direct3DCreate9 answered null");
                    return none;
                }
            }
        }
        let d3d = self.d3d.clone().expect("made above");
        let mut pp = present_parameters(present, window);
        let mut out = None;
        let r = unsafe {
            d3d.CreateDevice(
                0,
                D3DDEVTYPE_HAL,
                HWND(focus as *mut _),
                behavior | FPU_PRESERVE,
                &mut pp,
                &mut out,
            )
        };
        let Some(real) = out.filter(|_| r.is_ok()) else {
            eprintln!(
                "NV_D3D9=real: CreateDevice answered {:#010x}: nothing is forwarded",
                hr(r)
            );
            return none;
        };
        self.implicit_surfaces(&real, back_buffers, depth_stencil);
        self.insert(device, Object::Device(real.clone()));
        eprintln!(
            "NV_D3D9=real: the game's device is on the real window ({}x{}, format {}, behaviour {behavior:#x})",
            present.first().copied().unwrap_or(0),
            present.get(1).copied().unwrap_or(0),
            present.get(2).copied().unwrap_or(0),
        );
        Reply::Device {
            created: true,
            native: Some(Native(Box::new(SharedDevice(real)))),
        }
    }

    /// The level or surface's bytes (rows packed, `size` of them): the kept ones when its real
    /// surface could not take the last upload, else the real surface's.
    fn read_surface(&mut self, surface: u32, size: u32) -> Option<Vec<u8>> {
        if let Some(bytes) = self.kept.get(&surface) {
            return (bytes.len() == size as usize).then(|| bytes.clone());
        }
        let s = self.surface(surface)?;
        let mut desc = D3DSURFACE_DESC::default();
        unsafe { s.GetDesc(&mut desc) }.ok()?;
        let (block, bytes) = super::d3d9device::format_blocks(desc.Format.0)?;
        let row = desc.Width.div_ceil(block) * bytes;
        let rows = desc.Height.div_ceil(block);
        if row * rows != size {
            return None;
        }
        let mut locked = D3DLOCKED_RECT::default();
        let r = unsafe { s.LockRect(&mut locked, std::ptr::null(), LOCK_READONLY) };
        if r.is_err() {
            self.answer("LockRect of the real surface to read", 0, hr(r));
            return None;
        }
        let mut out = Vec::with_capacity(size as usize);
        for y in 0..rows {
            out.extend_from_slice(unsafe {
                std::slice::from_raw_parts(
                    (locked.pBits as *const u8).add(y as usize * locked.Pitch as usize),
                    row as usize,
                )
            });
        }
        let _ = unsafe { s.UnlockRect() };
        Some(out)
    }

    /// The game unlocked `surface`: its `rows` rows of `row` bytes go to the real surface, or are
    /// kept here when it has none or cannot be locked.
    fn upload_surface(&mut self, surface: u32, row: u32, rows: u32, data: &Data<'_>) {
        let Some(s) = self.surface(surface) else {
            self.kept.insert(surface, data.bytes().into_owned());
            return;
        };
        let mut locked = D3DLOCKED_RECT::default();
        let r = unsafe { s.LockRect(&mut locked, std::ptr::null(), 0) };
        if r.is_err() {
            self.answer("LockRect of the real surface", 0, hr(r));
            self.kept.insert(surface, data.bytes().into_owned());
            return;
        }
        for y in 0..rows {
            let to = unsafe {
                std::slice::from_raw_parts_mut(
                    (locked.pBits as *mut u8).add(y as usize * locked.Pitch as usize),
                    row as usize,
                )
            };
            data.copy_to(y * row, to);
        }
        let _ = unsafe { s.UnlockRect() };
        if !self.kept.is_empty() {
            self.kept.remove(&surface);
        }
    }

    fn upload_buffer(&mut self, buffer: u32, offset: u32, flags: u32, data: &Data<'_>) {
        if flags & LOCK_READONLY != 0 {
            return;
        }
        let size = data.len();
        let pass = if self.dynamic.contains(&buffer) {
            flags & (LOCK_DISCARD | LOCK_NOOVERWRITE)
        } else {
            0
        };
        let mut at: *mut core::ffi::c_void = std::ptr::null_mut();
        let r = match self.objects.get(&buffer) {
            Some(Object::VertexBuffer(b)) => unsafe { b.Lock(offset, size, &mut at, pass) },
            Some(Object::IndexBuffer(b)) => unsafe { b.Lock(offset, size, &mut at, pass) },
            _ => return,
        };
        if r.is_err() || at.is_null() {
            return self.answer("Lock of the real buffer", 0, hr(r));
        }
        data.copy_to(0, unsafe {
            std::slice::from_raw_parts_mut(at as *mut u8, size as usize)
        });
        let _ = match self.objects.get(&buffer) {
            Some(Object::VertexBuffer(b)) => unsafe { b.Unlock() },
            Some(Object::IndexBuffer(b)) => unsafe { b.Unlock() },
            _ => Ok(()),
        };
    }

    fn render_target_data(
        &mut self,
        device: u32,
        source: u32,
        destination: u32,
        pitch: u32,
    ) -> Option<Rows> {
        let d = self.device(device)?;
        let (from, to) = (self.surface(source)?, self.surface(destination)?);
        let r = unsafe { d.GetRenderTargetData(&from, &to) };
        self.answer("GetRenderTargetData", 0, hr(r));
        let none = Rows {
            row: 0,
            bytes: Vec::new(),
        };
        if pitch == 0 {
            // A texture level the backend keeps: the caller reads it back when it needs it.
            return Some(none);
        }
        let mut desc = D3DSURFACE_DESC::default();
        let mut locked = D3DLOCKED_RECT::default();
        unsafe {
            if to.GetDesc(&mut desc).is_err()
                || to
                    .LockRect(&mut locked, std::ptr::null(), LOCK_READONLY)
                    .is_err()
            {
                return Some(none);
            }
            let row = pitch.min(locked.Pitch as u32);
            let mut bytes = Vec::with_capacity((row * desc.Height) as usize);
            for y in 0..desc.Height {
                bytes.extend_from_slice(std::slice::from_raw_parts(
                    (locked.pBits as *const u8).add(y as usize * locked.Pitch as usize),
                    row as usize,
                ));
            }
            let _ = to.UnlockRect();
            Some(Rows { row, bytes })
        }
    }

    fn query_get_data(&mut self, query: u32, flags: u32) -> Option<(u32, u32)> {
        let Some(Object::Query(q)) = self.objects.get(&query) else {
            return None;
        };
        let mut value = 0u32;
        // The raw slot: `S_FALSE` (not done yet) is a success code the wrapper would fold into Ok.
        let hr = unsafe {
            use windows::core::Interface;
            (Interface::vtable(q).GetData)(
                Interface::as_raw(q),
                &mut value as *mut u32 as *mut core::ffi::c_void,
                4,
                flags,
            )
        };
        Some((hr.0 as u32, value))
    }
}

/// `Set*ShaderConstant*` of `kind` (0..2 vertex F, I, B; 3..5 pixel) with the `count` registers
/// at `p` (4 words each, or one for booleans).
#[inline]
fn forward_constants(
    d: &IDirect3DDevice9,
    kind: u8,
    start: u32,
    p: *const u32,
    count: u32,
) -> windows::core::Result<()> {
    // SAFETY: `p` holds `count` registers of the kind's size.
    unsafe {
        match kind {
            0 => d.SetVertexShaderConstantF(start, p as *const f32, count),
            1 => d.SetVertexShaderConstantI(start, p as *const i32, count),
            2 => d.SetVertexShaderConstantB(start, p as *const BOOL, count),
            3 => d.SetPixelShaderConstantF(start, p as *const f32, count),
            4 => d.SetPixelShaderConstantI(start, p as *const i32, count),
            _ => d.SetPixelShaderConstantB(start, p as *const BOOL, count),
        }
    }
}

impl Backend for RealBackend {
    fn execute(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        match cmd {
            Cmd::CreateDevice {
                device,
                focus,
                window,
                behavior,
                present,
                back_buffers,
                depth_stencil,
            } => {
                return self.create_device(
                    device,
                    focus,
                    window,
                    behavior,
                    &present.to_words(),
                    &back_buffers.to_words(),
                    depth_stencil,
                )
            }
            Cmd::ReadSurface { surface, size } => {
                return Reply::Bytes(self.read_surface(surface, size))
            }
            Cmd::GetRenderTargetData {
                device,
                source,
                destination,
                pitch,
            } => return Reply::Rows(self.render_target_data(device, source, destination, pitch)),
            Cmd::QueryGetData { query, flags } => {
                return Reply::Query(self.query_get_data(query, flags))
            }
            Cmd::Destroy { object } => self.destroy(object),
            Cmd::UploadSurface {
                surface,
                row,
                rows,
                data,
            } => self.upload_surface(surface, row, rows, &data),
            Cmd::UploadBuffer {
                buffer,
                offset,
                flags,
                data,
            } => self.upload_buffer(buffer, offset, flags, &data),
            Cmd::SetLod {
                texture,
                lod,
                model,
            } => match self.base_texture(texture) {
                Some(t) => {
                    let previous = unsafe { t.SetLOD(lod) };
                    self.answer("SetLOD (the previous level)", model, previous);
                }
                None => self.missing("SetLOD of a texture without a real one"),
            },
            Cmd::QueryIssue { query, flags } => {
                if let Some(Object::Query(q)) = self.objects.get(&query) {
                    let r = unsafe { q.Issue(flags) };
                    self.answer("Query::Issue", 0, hr(r));
                }
            }
            _ => self.device_command(cmd),
        }
        Reply::None
    }
}

impl RealBackend {
    /// A command of a device (the device's methods): nothing without its real device.
    fn device_command(&mut self, cmd: Cmd<Data<'_>>) {
        let device = match cmd {
            Cmd::Reset { device, .. }
            | Cmd::Present { device, .. }
            | Cmd::BeginScene { device, .. }
            | Cmd::EndScene { device, .. }
            | Cmd::Clear { device, .. }
            | Cmd::SetRenderTarget { device, .. }
            | Cmd::SetDepthStencilSurface { device, .. }
            | Cmd::SetViewport { device, .. }
            | Cmd::SetTransform { device, .. }
            | Cmd::SetScissorRect { device, .. }
            | Cmd::SetClipPlane { device, .. }
            | Cmd::SetGammaRamp { device, .. }
            | Cmd::SetRenderState { device, .. }
            | Cmd::SetTextureStageState { device, .. }
            | Cmd::SetSamplerState { device, .. }
            | Cmd::SetTexture { device, .. }
            | Cmd::SetVertexDeclaration { device, .. }
            | Cmd::SetFvf { device, .. }
            | Cmd::SetShader { device, .. }
            | Cmd::SetConstants { device, .. }
            | Cmd::SetStreamSource { device, .. }
            | Cmd::SetIndices { device, .. }
            | Cmd::DrawPrimitive { device, .. }
            | Cmd::DrawIndexedPrimitive { device, .. }
            | Cmd::DrawPrimitiveUp { device, .. }
            | Cmd::SetMaterial { device, .. }
            | Cmd::SetLight { device, .. }
            | Cmd::LightEnable { device, .. }
            | Cmd::SetSoftwareVertexProcessing { device, .. }
            | Cmd::UpdateTexture { device, .. }
            | Cmd::StretchRect { device, .. }
            | Cmd::ColorFill { device, .. }
            | Cmd::CreateTexture { device, .. }
            | Cmd::CreateCubeTexture { device, .. }
            | Cmd::CreateSurface { device, .. }
            | Cmd::CreateOffscreenSurface { device, .. }
            | Cmd::CreateBuffer { device, .. }
            | Cmd::CreateShader { device, .. }
            | Cmd::CreateDeclaration { device, .. }
            | Cmd::CreateQuery { device, .. } => device,
            other => unreachable!("{} is not a device's command", other.name()),
        };
        let Some(d) = self.device(device) else { return };
        let ok = 0;
        match cmd {
            Cmd::Reset {
                window,
                present,
                back_buffers,
                depth_stencil,
                ..
            } => {
                let mut pp = present_parameters(&present.to_words(), window);
                let r = unsafe { d.Reset(&mut pp) };
                self.answer("Reset", ok, hr(r));
                self.implicit_surfaces(&d, &back_buffers.to_words(), depth_stencil);
            }
            Cmd::Present {
                source,
                dest,
                window,
                dirty,
                model,
                ..
            } => {
                if dirty {
                    return self.answer("Present with a dirty region (not forwarded)", model, 1);
                }
                let (source, dest) = (rect(&source), rect(&dest));
                let r = unsafe {
                    d.Present(
                        rect_ptr(&source),
                        rect_ptr(&dest),
                        HWND(window as *mut _),
                        std::ptr::null(),
                    )
                };
                self.answer("Present", model, hr(r));
                // The frame is over: what the model let go of is bound nowhere on the real
                // device now.
                self.graveyard.clear();
            }
            Cmd::BeginScene { model, .. } => {
                let r = unsafe { d.BeginScene() };
                self.answer("BeginScene", model, hr(r));
            }
            Cmd::EndScene { model, .. } => {
                let r = unsafe { d.EndScene() };
                self.answer("EndScene", model, hr(r));
            }
            Cmd::Clear {
                count,
                rects,
                flags,
                colour,
                z,
                stencil,
                model,
                ..
            } => {
                let words = rects.as_ref().map(|r| r.to_words());
                let pointer = words
                    .as_ref()
                    .map_or(std::ptr::null(), |w| w.as_ptr() as *const D3DRECT);
                let r =
                    unsafe { d.Clear(count, pointer, flags, colour, f32::from_bits(z), stencil) };
                self.answer("Clear", model, hr(r));
            }
            Cmd::SetRenderTarget { index, surface, .. } => {
                let real = match surface {
                    0 => None,
                    s => match self.surface(s) {
                        Some(s) => Some(s),
                        None => {
                            return self.missing("SetRenderTarget of a surface without a real one")
                        }
                    },
                };
                let r = unsafe { d.SetRenderTarget(index, real.as_ref()) };
                self.answer("SetRenderTarget", ok, hr(r));
            }
            Cmd::SetDepthStencilSurface { surface, .. } => {
                let real = match surface {
                    0 => None,
                    s => match self.surface(s) {
                        Some(s) => Some(s),
                        None => {
                            return self
                                .missing("SetDepthStencilSurface of a surface without a real one")
                        }
                    },
                };
                let r = unsafe { d.SetDepthStencilSurface(real.as_ref()) };
                self.answer("SetDepthStencilSurface", ok, hr(r));
            }
            Cmd::SetViewport { viewport: v, .. } => {
                let vp = D3DVIEWPORT9 {
                    X: v[0],
                    Y: v[1],
                    Width: v[2],
                    Height: v[3],
                    MinZ: f32::from_bits(v[4]),
                    MaxZ: f32::from_bits(v[5]),
                };
                let r = unsafe { d.SetViewport(&vp) };
                self.answer("SetViewport", ok, hr(r));
            }
            Cmd::SetTransform { state, matrix, .. } => {
                let m = matrix.to_words();
                let r = unsafe {
                    d.SetTransform(D3DTRANSFORMSTATETYPE(state as i32), m.as_ptr() as *const _)
                };
                self.answer("SetTransform", ok, hr(r));
            }
            Cmd::SetScissorRect { rect: r, .. } => {
                let area = RECT {
                    left: r[0],
                    top: r[1],
                    right: r[2],
                    bottom: r[3],
                };
                let result = unsafe { d.SetScissorRect(&area) };
                self.answer("SetScissorRect", ok, hr(result));
            }
            Cmd::SetClipPlane { index, plane, .. } => {
                let r = unsafe { d.SetClipPlane(index, plane.as_ptr() as *const f32) };
                self.answer("SetClipPlane", ok, hr(r));
            }
            Cmd::SetGammaRamp {
                swap_chain,
                flags,
                ramp,
                ..
            } => {
                // `void` in the SDK: nothing to compare with the model's answer.
                let bytes = ramp.bytes();
                let entry = |at: usize| {
                    bytes
                        .get(at..at + 2)
                        .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
                };
                let mut table = D3DGAMMARAMP::default();
                for i in 0..0x100 {
                    table.red[i] = entry(2 * i);
                    table.green[i] = entry(0x200 + 2 * i);
                    table.blue[i] = entry(0x400 + 2 * i);
                }
                unsafe { d.SetGammaRamp(swap_chain, flags, &table) };
            }
            Cmd::SetRenderState { state, value, .. } => {
                let r = unsafe { d.SetRenderState(D3DRENDERSTATETYPE(state as i32), value) };
                self.answer("SetRenderState", ok, hr(r));
            }
            Cmd::SetTextureStageState {
                stage, kind, value, ..
            } => {
                let r = unsafe {
                    d.SetTextureStageState(stage, D3DTEXTURESTAGESTATETYPE(kind as i32), value)
                };
                self.answer("SetTextureStageState", ok, hr(r));
            }
            Cmd::SetSamplerState {
                sampler,
                kind,
                value,
                ..
            } => {
                let r =
                    unsafe { d.SetSamplerState(sampler, D3DSAMPLERSTATETYPE(kind as i32), value) };
                self.answer("SetSamplerState", ok, hr(r));
            }
            Cmd::SetTexture {
                sampler, texture, ..
            } => {
                let r = match self.objects.get(&texture) {
                    _ if texture == 0 => unsafe {
                        d.SetTexture(sampler, None::<&IDirect3DBaseTexture9>)
                    },
                    Some(Object::Texture(t)) => unsafe { d.SetTexture(sampler, t) },
                    Some(Object::Cube(t)) => unsafe { d.SetTexture(sampler, t) },
                    _ => return self.missing("SetTexture of a texture without a real one"),
                };
                self.answer("SetTexture", ok, hr(r));
            }
            Cmd::SetVertexDeclaration { declaration, .. } => {
                let r = match self.objects.get(&declaration) {
                    _ if declaration == 0 => unsafe {
                        d.SetVertexDeclaration(None::<&IDirect3DVertexDeclaration9>)
                    },
                    Some(Object::Declaration(v)) => unsafe { d.SetVertexDeclaration(v) },
                    _ => {
                        let _ = unsafe {
                            d.SetVertexDeclaration(None::<&IDirect3DVertexDeclaration9>)
                        };
                        return self.broken(
                            device,
                            DECLARATION,
                            "SetVertexDeclaration without a real one",
                        );
                    }
                };
                self.mended(device, DECLARATION);
                self.answer("SetVertexDeclaration", ok, hr(r));
            }
            Cmd::SetFvf { fvf, .. } => {
                let r = unsafe { d.SetFVF(fvf) };
                self.answer("SetFVF", ok, hr(r));
            }
            Cmd::SetShader { pixel, shader, .. } => {
                let r = match (pixel, self.objects.get(&shader)) {
                    (false, _) if shader == 0 => unsafe {
                        d.SetVertexShader(None::<&IDirect3DVertexShader9>)
                    },
                    (true, _) if shader == 0 => unsafe {
                        d.SetPixelShader(None::<&IDirect3DPixelShader9>)
                    },
                    (false, Some(Object::VertexShader(s))) => unsafe { d.SetVertexShader(s) },
                    (true, Some(Object::PixelShader(s))) => unsafe { d.SetPixelShader(s) },
                    _ => {
                        let _ = unsafe {
                            if pixel {
                                d.SetPixelShader(None::<&IDirect3DPixelShader9>)
                            } else {
                                d.SetVertexShader(None::<&IDirect3DVertexShader9>)
                            }
                        };
                        let which = if pixel { PIXEL_SHADER } else { VERTEX_SHADER };
                        return self.broken(device, which, "Set*Shader without a real one");
                    }
                };
                self.mended(device, if pixel { PIXEL_SHADER } else { VERTEX_SHADER });
                self.answer("Set*Shader", ok, hr(r));
            }
            Cmd::SetConstants {
                kind,
                start,
                count,
                data,
                ..
            } => {
                let words = data.to_words();
                let per = if kind % 3 == 2 { 1 } else { 4 };
                assert!(
                    words.len() >= (per * count) as usize,
                    "SetConstants: {count} registers in {} words",
                    words.len()
                );
                let r = forward_constants(&d, kind, start, words.as_ptr(), count);
                self.answer("Set*ShaderConstant*", ok, hr(r));
            }
            Cmd::SetStreamSource {
                stream,
                buffer,
                offset,
                stride,
                ..
            } => {
                let r = match self.objects.get(&buffer) {
                    _ if buffer == 0 => unsafe {
                        d.SetStreamSource(stream, None::<&IDirect3DVertexBuffer9>, offset, stride)
                    },
                    Some(Object::VertexBuffer(b)) => unsafe {
                        d.SetStreamSource(stream, b, offset, stride)
                    },
                    _ => {
                        let _ = unsafe {
                            d.SetStreamSource(stream, None::<&IDirect3DVertexBuffer9>, 0, 0)
                        };
                        return self.broken(
                            device,
                            stream,
                            "SetStreamSource without a real buffer",
                        );
                    }
                };
                self.mended(device, stream);
                self.answer("SetStreamSource", ok, hr(r));
            }
            Cmd::SetIndices { buffer, .. } => {
                let r = match self.objects.get(&buffer) {
                    _ if buffer == 0 => unsafe { d.SetIndices(None::<&IDirect3DIndexBuffer9>) },
                    Some(Object::IndexBuffer(b)) => unsafe { d.SetIndices(b) },
                    _ => {
                        let _ = unsafe { d.SetIndices(None::<&IDirect3DIndexBuffer9>) };
                        return self.broken(device, INDICES, "SetIndices without a real buffer");
                    }
                };
                self.mended(device, INDICES);
                self.answer("SetIndices", ok, hr(r));
            }
            Cmd::DrawPrimitive {
                kind, start, count, ..
            } => {
                if self.skipped(device) {
                    return;
                }
                let r = unsafe { d.DrawPrimitive(D3DPRIMITIVETYPE(kind as i32), start, count) };
                self.answer("DrawPrimitive", ok, hr(r));
            }
            Cmd::DrawIndexedPrimitive {
                kind,
                base_vertex,
                min_index,
                vertices,
                start_index,
                count,
                ..
            } => {
                if self.skipped(device) {
                    return;
                }
                let r = unsafe {
                    d.DrawIndexedPrimitive(
                        D3DPRIMITIVETYPE(kind as i32),
                        base_vertex,
                        min_index,
                        vertices,
                        start_index,
                        count,
                    )
                };
                self.answer("DrawIndexedPrimitive", ok, hr(r));
            }
            Cmd::DrawPrimitiveUp {
                kind,
                count,
                stride,
                vertices,
                ..
            } => {
                if self.skipped(device) {
                    return;
                }
                let words = vertices.to_words();
                let r = unsafe {
                    d.DrawPrimitiveUP(
                        D3DPRIMITIVETYPE(kind as i32),
                        count,
                        words.as_ptr() as *const _,
                        stride,
                    )
                };
                self.answer("DrawPrimitiveUP", ok, hr(r));
            }
            Cmd::SetMaterial { material, .. } => {
                let m = material.to_words();
                assert!(m.len() >= 17, "SetMaterial: {} words", m.len());
                let r = unsafe { d.SetMaterial(m.as_ptr() as *const D3DMATERIAL9) };
                self.answer("SetMaterial", ok, hr(r));
            }
            Cmd::SetLight { index, light, .. } => {
                let l = light.to_words();
                assert!(l.len() >= 26, "SetLight: {} words", l.len());
                let r = unsafe { d.SetLight(index, l.as_ptr() as *const D3DLIGHT9) };
                self.answer("SetLight", ok, hr(r));
            }
            Cmd::LightEnable { index, enable, .. } => {
                let r = unsafe { d.LightEnable(index, BOOL(enable as i32)) };
                self.answer("LightEnable", ok, hr(r));
            }
            Cmd::SetSoftwareVertexProcessing { software, .. } => {
                let r = unsafe { d.SetSoftwareVertexProcessing(BOOL(software as i32)) };
                self.answer("SetSoftwareVertexProcessing", ok, hr(r));
            }
            Cmd::UpdateTexture {
                source,
                destination,
                ..
            } => {
                let (Some(from), Some(to)) =
                    (self.base_texture(source), self.base_texture(destination))
                else {
                    return self.missing("UpdateTexture without real textures");
                };
                let r = unsafe { d.UpdateTexture(&from, &to) };
                self.answer("UpdateTexture", ok, hr(r));
            }
            Cmd::StretchRect {
                source,
                source_rect,
                destination,
                destination_rect,
                filter,
                ..
            } => {
                let (Some(from), Some(to)) = (self.surface(source), self.surface(destination))
                else {
                    return self.missing("StretchRect without real surfaces");
                };
                let (a, b) = (rect(&source_rect), rect(&destination_rect));
                let r = unsafe {
                    d.StretchRect(
                        &from,
                        rect_ptr(&a),
                        &to,
                        rect_ptr(&b),
                        D3DTEXTUREFILTERTYPE(filter as i32),
                    )
                };
                self.answer("StretchRect", ok, hr(r));
            }
            Cmd::ColorFill {
                surface,
                rect: area,
                colour,
                ..
            } => {
                let Some(s) = self.surface(surface) else {
                    return self.missing("ColorFill of a surface without a real one");
                };
                let area = rect(&area);
                let r = unsafe { d.ColorFill(&s, rect_ptr(&area), colour) };
                self.answer("ColorFill", ok, hr(r));
            }
            Cmd::CreateTexture {
                texture,
                levels,
                width,
                height,
                usage,
                format,
                pool,
                ..
            } => {
                let levels = levels.to_words();
                let mut out = None;
                let r = unsafe {
                    d.CreateTexture(
                        width,
                        height,
                        levels.len() as u32,
                        usage,
                        D3DFORMAT(format),
                        D3DPOOL(pool as i32),
                        &mut out,
                        std::ptr::null_mut(),
                    )
                };
                let Some(t) = out.filter(|_| r.is_ok()) else {
                    return self.answer("CreateTexture", 0, hr(r));
                };
                for (i, &level) in levels.iter().enumerate() {
                    if let Ok(s) = unsafe { t.GetSurfaceLevel(i as u32) } {
                        self.insert(level, Object::Surface(s));
                    }
                }
                self.insert(texture, Object::Texture(t));
            }
            Cmd::CreateCubeTexture {
                texture,
                surfaces,
                edge,
                usage,
                format,
                pool,
                ..
            } => {
                let surfaces = surfaces.to_words();
                let levels = surfaces.len() as u32 / 6;
                let mut out = None;
                let r = unsafe {
                    d.CreateCubeTexture(
                        edge,
                        levels,
                        usage,
                        D3DFORMAT(format),
                        D3DPOOL(pool as i32),
                        &mut out,
                        std::ptr::null_mut(),
                    )
                };
                let Some(t) = out.filter(|_| r.is_ok()) else {
                    return self.answer("CreateCubeTexture", 0, hr(r));
                };
                for (i, &surface) in surfaces.iter().enumerate() {
                    let (face, level) = (i as u32 / levels, i as u32 % levels);
                    if let Ok(s) =
                        unsafe { t.GetCubeMapSurface(D3DCUBEMAP_FACES(face as i32), level) }
                    {
                        self.insert(surface, Object::Surface(s));
                    }
                }
                self.insert(texture, Object::Cube(t));
            }
            Cmd::CreateSurface {
                surface,
                depth,
                width,
                height,
                format,
                multisample,
                quality,
                flag,
                ..
            } => {
                let mut out = None;
                let (format, multisample, flag) = (
                    D3DFORMAT(format),
                    D3DMULTISAMPLE_TYPE(multisample as i32),
                    BOOL(flag as i32),
                );
                let null = std::ptr::null_mut();
                let r = unsafe {
                    if depth {
                        d.CreateDepthStencilSurface(
                            width,
                            height,
                            format,
                            multisample,
                            quality,
                            flag,
                            &mut out,
                            null,
                        )
                    } else {
                        d.CreateRenderTarget(
                            width,
                            height,
                            format,
                            multisample,
                            quality,
                            flag,
                            &mut out,
                            null,
                        )
                    }
                };
                match out.filter(|_| r.is_ok()) {
                    Some(s) => self.insert(surface, Object::Surface(s)),
                    None => self.answer("CreateRenderTarget/DepthStencilSurface", 0, hr(r)),
                }
            }
            Cmd::CreateOffscreenSurface {
                surface,
                width,
                height,
                format,
                pool,
                ..
            } => {
                let mut out = None;
                let r = unsafe {
                    d.CreateOffscreenPlainSurface(
                        width,
                        height,
                        D3DFORMAT(format),
                        D3DPOOL(pool as i32),
                        &mut out,
                        std::ptr::null_mut(),
                    )
                };
                match out.filter(|_| r.is_ok()) {
                    Some(s) => self.insert(surface, Object::Surface(s)),
                    None => self.answer("CreateOffscreenPlainSurface", 0, hr(r)),
                }
            }
            Cmd::CreateBuffer {
                buffer,
                index,
                length,
                usage,
                format,
                pool,
                ..
            } => {
                let null = std::ptr::null_mut();
                let object = if index {
                    let mut out = None;
                    let r = unsafe {
                        d.CreateIndexBuffer(
                            length,
                            usage,
                            D3DFORMAT(format),
                            D3DPOOL(pool as i32),
                            &mut out,
                            null,
                        )
                    };
                    match out.filter(|_| r.is_ok()) {
                        Some(b) => Object::IndexBuffer(b),
                        None => return self.answer("CreateIndexBuffer", 0, hr(r)),
                    }
                } else {
                    let mut out = None;
                    let r = unsafe {
                        d.CreateVertexBuffer(
                            length,
                            usage,
                            format,
                            D3DPOOL(pool as i32),
                            &mut out,
                            null,
                        )
                    };
                    match out.filter(|_| r.is_ok()) {
                        Some(b) => Object::VertexBuffer(b),
                        None => return self.answer("CreateVertexBuffer", 0, hr(r)),
                    }
                };
                self.insert(buffer, object);
                if usage & USAGE_DYNAMIC != 0 {
                    self.dynamic.insert(buffer);
                } else {
                    self.dynamic.remove(&buffer);
                }
            }
            Cmd::CreateShader {
                shader,
                pixel,
                tokens,
                ..
            } => {
                let tokens = tokens.to_words();
                let object = if pixel {
                    match unsafe { d.CreatePixelShader(tokens.as_ptr()) } {
                        Ok(s) => Object::PixelShader(s),
                        Err(err) => {
                            return self.answer("CreatePixelShader", 0, err.code().0 as u32)
                        }
                    }
                } else {
                    match unsafe { d.CreateVertexShader(tokens.as_ptr()) } {
                        Ok(s) => Object::VertexShader(s),
                        Err(err) => {
                            return self.answer("CreateVertexShader", 0, err.code().0 as u32)
                        }
                    }
                };
                self.insert(shader, object);
            }
            Cmd::CreateDeclaration {
                declaration,
                elements,
                ..
            } => {
                let elements = elements.to_words();
                match unsafe {
                    d.CreateVertexDeclaration(elements.as_ptr() as *const D3DVERTEXELEMENT9)
                } {
                    Ok(v) => self.insert(declaration, Object::Declaration(v)),
                    Err(err) => self.answer("CreateVertexDeclaration", 0, err.code().0 as u32),
                }
            }
            Cmd::CreateQuery { query, kind, .. } => {
                match unsafe { d.CreateQuery(D3DQUERYTYPE(kind as i32)) } {
                    Ok(q) => self.insert(query, Object::Query(q)),
                    Err(err) => self.answer("CreateQuery", 0, err.code().0 as u32),
                }
            }
            other => unreachable!("{} is not a device's command", other.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_presentation_interval_is_the_games_unless_immediate_is_asked() {
        assert_eq!(choose_interval(1, false), 1);
        assert_eq!(choose_interval(0, false), 0);
        assert_eq!(choose_interval(1, true), INTERVAL_IMMEDIATE);
    }

    /// Commands of a device the backend never made, and of objects it never made, are skipped
    /// (no device: nothing to call); reads of what it never had answer none.
    #[test]
    fn commands_without_real_objects_do_nothing() {
        let mut b = RealBackend::new();
        let words = [0u32; 16];
        for cmd in [
            Cmd::SetRenderState {
                device: 1,
                state: 7,
                value: 1,
            },
            Cmd::SetTransform {
                device: 1,
                state: 256,
                matrix: Data::from_words(&words),
            },
            Cmd::DrawIndexedPrimitive {
                device: 1,
                kind: 4,
                base_vertex: 0,
                min_index: 0,
                vertices: 3,
                start_index: 0,
                count: 1,
            },
            Cmd::Destroy { object: 2 },
            Cmd::QueryIssue { query: 3, flags: 1 },
        ] {
            assert!(matches!(b.execute(cmd), Reply::None));
        }
        assert!(matches!(
            b.execute(Cmd::QueryGetData { query: 3, flags: 1 }),
            Reply::Query(None)
        ));
        assert!(matches!(
            b.execute(Cmd::GetRenderTargetData {
                device: 1,
                source: 2,
                destination: 3,
                pitch: 16,
            }),
            Reply::Rows(None)
        ));
    }

    /// A level without a real surface keeps the bytes it was given, and answers them back; a
    /// destroyed one forgets them.
    #[test]
    fn a_level_without_a_real_surface_keeps_its_bytes() {
        let mut b = RealBackend::new();
        let bytes: Vec<u8> = (0..64u8).collect();
        b.execute(Cmd::UploadSurface {
            surface: 0x40,
            row: 16,
            rows: 4,
            data: Data::Bytes(&bytes),
        });
        match b.execute(Cmd::ReadSurface {
            surface: 0x40,
            size: 64,
        }) {
            Reply::Bytes(Some(read)) => assert_eq!(read, bytes),
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            b.execute(Cmd::ReadSurface {
                surface: 0x40,
                size: 32
            }),
            Reply::Bytes(None)
        ));
        b.execute(Cmd::Destroy { object: 0x40 });
        assert!(matches!(
            b.execute(Cmd::ReadSurface {
                surface: 0x40,
                size: 64
            }),
            Reply::Bytes(None)
        ));
    }
}
