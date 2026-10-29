//! The platform's Direct3D 9 layer forwarded to the real `d3d9.dll` (ADR-0008), Windows only,
//! switched on with `NV_D3D9=real`.
//!
//! The game's code keeps calling the modelled objects of `super::d3d9`, `super::d3d9device` and
//! `super::d3d9resource`, and keeps getting the modelled answers (they were measured against the
//! same runtime). Each modelled call that changes what is drawn is also made on a real
//! counterpart: the modelled window gets a real Win32 window, the modelled device a real device
//! on that window, each modelled resource a real resource whose contents are copied from engine
//! memory when the game unlocks it. Where the real runtime answers differently from the model,
//! the call is printed once (`NV_D3D9=real: ...`), and the model's answer stands.
//!
//! The real window lives on a thread of its own that only creates it and pumps its messages, so
//! it never waits on the game's threads (which run one at a time on threads of the thread host,
//! `crate::thread_host`). It is shown without taking the focus: the viewer's window keeps the
//! keyboard and mouse the game reads through DirectInput.
//!
//! The real device is created with `D3DCREATE_FPU_PRESERVE` added to the game's flags: the
//! game's x87 precision switch is modelled (`crate::units::platform::direct3d_device_created`,
//! from the game's own flags), and the host thread's floating-point state is not the game's.

use crate::Engine;
use std::collections::{HashMap, HashSet};
use windows::core::PCSTR;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Direct3D9::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Whether the forwarding is asked for (`NV_D3D9=real`).
pub fn wanted() -> bool {
    std::env::var("NV_D3D9").is_ok_and(|v| v.eq_ignore_ascii_case("real"))
}

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

/// The real side (`Graphics::real`): real windows by modelled handle, the real `IDirect3D9`,
/// the real objects by the modelled object's address, the open buffer locks.
#[derive(Default)]
pub struct Real {
    windows: HashMap<u32, isize>,
    d3d: Option<IDirect3D9>,
    objects: HashMap<u32, Object>,
    /// Open locks of buffers: (offset, size, flags), innermost last.
    locks: HashMap<u32, Vec<(u32, u32, u32)>>,
    /// The buffers made with `D3DUSAGE_DYNAMIC` (their locks pass `DISCARD`/`NOOVERWRITE` on).
    dynamic: HashSet<u32>,
    /// The differences already printed: (call, model's answer, real answer).
    logged: HashSet<(&'static str, u32, u32)>,
    /// Bindings the model took whose object has no real counterpart, by (device, binding: a
    /// stream number, or [`INDICES`], [`DECLARATION`], [`VERTEX_SHADER`], [`PIXEL_SHADER`]).
    /// The real binding is set to null instead, and the real draws are skipped while any is
    /// left: a pure device does not check what it draws with.
    broken: HashSet<(u32, u32)>,
    /// Real objects whose modelled ones are destroyed, released at the next `Present` ([`forget`]).
    graveyard: Vec<Object>,
}

impl std::fmt::Debug for Real {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Real")
            .field("windows", &self.windows)
            .field("d3d", &self.d3d.is_some())
            .field("objects", &self.objects.len())
            .finish()
    }
}

fn real(e: &mut Engine) -> &mut Real {
    &mut e.kernel.graphics.real
}

/// The real window of a modelled window handle, when it has one.
pub fn real_window(e: &Engine, handle: u32) -> Option<isize> {
    e.kernel.graphics.real.windows.get(&handle).copied()
}

/// `D3DPRESENT_INTERVAL_IMMEDIATE`.
const INTERVAL_IMMEDIATE: u32 = 0x8000_0000;

/// The real device's presentation interval for the game's `interval` (its `iPresentInterval`):
/// the game's, unless `NV_D3D9_PRESENT=immediate` (a measurement setting of the host, not the
/// game's) asks for `D3DPRESENT_INTERVAL_IMMEDIATE`, so a frame rate measures the work and not
/// the display's refresh (the original game measured likewise with `iPresentInterval=0`). The
/// model keeps the game's parameters and answers.
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

fn hr(r: windows::core::Result<()>) -> u32 {
    match r {
        Ok(()) => 0,
        Err(err) => err.code().0 as u32,
    }
}

/// Prints, once per kind, a call where the real runtime answers differently from the model.
fn answer(e: &mut Engine, call: &'static str, model: u32, real_hr: u32) {
    if model != real_hr && real(e).logged.insert((call, model, real_hr)) {
        eprintln!(
            "NV_D3D9=real: {call}: the model answers {model:#010x}, the real runtime {real_hr:#010x}"
        );
    }
}

fn words(e: &Engine, at: u32, n: u32) -> Vec<u32> {
    (0..n).map(|i| e.mem.u32(at + 4 * i)).collect()
}

fn device(e: &Engine, this: u32) -> Option<IDirect3DDevice9> {
    match e.kernel.graphics.real.objects.get(&this) {
        Some(Object::Device(d)) => Some(d.clone()),
        _ => None,
    }
}

/// Forgets the real counterpart of a modelled object that is destroyed (its address can be
/// reused). It is not released yet but kept until the frame's `Present`: the model lets go of an
/// object when the program unbinds it, before the same call reaches the real device, and the
/// game's device is a pure one (`D3DCREATE_PUREDEVICE`), which keeps no reference of its own to
/// what is bound: released at once, the real object would be freed while still bound, and the
/// real call that replaces the binding would read it (an access violation in
/// `SetStreamSource`, p169).
pub fn forget(e: &mut Engine, object: u32) {
    let r = real(e);
    if let Some(o) = r.objects.remove(&object) {
        r.graveyard.push(o);
    }
    r.locks.remove(&object);
    r.dynamic.remove(&object);
}

// The window.

const CLASS_NAME: PCSTR = PCSTR(b"nv-rs-d3d9\0".as_ptr());

extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Closing the game's window ends the run (the player's way out, also when the game has
    // stopped); the window itself is not destroyed first.
    if message == WM_CLOSE {
        eprintln!("NV_D3D9=real: the game's window was closed: the run ends");
        std::process::exit(0);
    }
    match message {
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP | WM_MOUSEMOVE | WM_LBUTTONDOWN
        | WM_LBUTTONUP | WM_RBUTTONDOWN | WM_RBUTTONUP | WM_MBUTTONDOWN | WM_MBUTTONUP
        | WM_XBUTTONDOWN | WM_XBUTTONUP | WM_MOUSEWHEEL => {
            window_input(window, message, wparam, lparam)
        }
        // Losing the focus lets go of everything this window held.
        WM_ACTIVATE if wparam.0 & 0xffff == 0 => release_input(),
        WM_KILLFOCUS => release_input(),
        _ => {}
    }
    // Alt and F10 are the game's keys: they do not open the window menu (Alt+F4 still closes
    // nothing, as above).
    if message == WM_SYSKEYDOWN || message == WM_SYSKEYUP {
        return LRESULT(0);
    }
    unsafe { DefWindowProcA(window, message, wparam, lparam) }
}

// The real window as an input source (ADR-0008): while it has the focus, its keyboard and mouse
// (from the window's own messages) are held here; the host merges them into the
// game's DirectInput devices at each frame ([`take_input`]).

/// What the real window's keyboard and mouse did since the last frame.
#[derive(Debug, Clone, Copy)]
pub struct Input {
    /// The keys held, by DirectInput key code (the scan code, 0x80 added for extended keys).
    pub keys: [bool; 256],
    /// The mouse's relative motion and wheel (120 a notch) since the last frame.
    pub motion: [i32; 3],
    /// The buttons held, in DirectInput's order (left, right, middle, X1, X2).
    pub buttons: [bool; 8],
}

impl Default for Input {
    fn default() -> Self {
        Input {
            keys: [false; 256],
            motion: [0; 3],
            buttons: [false; 8],
        }
    }
}

fn input() -> &'static std::sync::Mutex<Input> {
    static INPUT: std::sync::OnceLock<std::sync::Mutex<Input>> = std::sync::OnceLock::new();
    INPUT.get_or_init(|| std::sync::Mutex::new(Input::default()))
}

fn release_input() {
    if let Ok(mut i) = input().lock() {
        i.keys = [false; 256];
        i.buttons = [false; 8];
    }
    if let Ok(mut last) = LAST_MOUSE.lock() {
        *last = None;
    }
}

/// The keys the real window holds, by DirectInput key code.
pub fn held_keys() -> [bool; 256] {
    input().lock().map(|i| i.keys).unwrap_or([false; 256])
}

/// The real window's input for this frame: the keys and buttons held, the motion since the last
/// call (taken: it starts again from 0).
pub fn take_input() -> Input {
    match input().lock() {
        Ok(mut i) => {
            let now = *i;
            i.motion = [0; 3];
            now
        }
        Err(_) => Input::default(),
    }
}

/// The DirectInput key code of a key message (`lParam`: scan code in bits 16..23, the extended
/// bit 24); `None` for the fake shifts Windows sends around some extended keys.
fn dik(lparam: isize) -> Option<u8> {
    let scan = ((lparam >> 16) & 0xff) as u8;
    let extended = lparam & (1 << 24) != 0;
    if scan == 0 {
        return None;
    }
    if extended {
        if scan == 0x2a || scan == 0x36 {
            return None;
        }
        // NumLock reports as extended 0x45 (DIK_NUMLOCK); others get 0x80 (DIK_RCONTROL,
        // DIK_RMENU, the arrows, DIK_NUMPADENTER, DIK_DIVIDE, ...).
        if scan == 0x45 {
            return Some(0x45);
        }
        return Some((scan & 0x7f) | 0x80);
    }
    // Pause reports as 0x45 without the extended bit (DIK_PAUSE 0xC5).
    if scan == 0x45 {
        return Some(0xc5);
    }
    Some(scan & 0x7f)
}

/// The last mouse position (client coordinates of the window under it), for the motion.
static LAST_MOUSE: std::sync::Mutex<Option<(i32, i32, isize)>> = std::sync::Mutex::new(None);

/// A message of the real window that is input.
fn window_input(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) {
    let Ok(mut i) = input().lock() else { return };
    let button = |i: &mut Input, b: usize, down: bool| i.buttons[b] = down;
    match message {
        WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
            if let Some(code) = dik(lparam.0) {
                i.keys[code as usize] = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
            }
        }
        WM_MOUSEMOVE => {
            let (x, y) = (
                (lparam.0 & 0xffff) as i16 as i32,
                ((lparam.0 >> 16) & 0xffff) as i16 as i32,
            );
            if let Ok(mut last) = LAST_MOUSE.lock() {
                if let Some((lx, ly, w)) = *last {
                    if w == window.0 as isize {
                        i.motion[0] += x - lx;
                        i.motion[1] += y - ly;
                    }
                }
                *last = Some((x, y, window.0 as isize));
            }
        }
        WM_LBUTTONDOWN | WM_LBUTTONUP => button(&mut i, 0, message == WM_LBUTTONDOWN),
        WM_RBUTTONDOWN | WM_RBUTTONUP => button(&mut i, 1, message == WM_RBUTTONDOWN),
        WM_MBUTTONDOWN | WM_MBUTTONUP => button(&mut i, 2, message == WM_MBUTTONDOWN),
        WM_XBUTTONDOWN | WM_XBUTTONUP => {
            let b = if (wparam.0 >> 16) & 0xffff == 2 { 4 } else { 3 };
            button(&mut i, b, message == WM_XBUTTONDOWN);
        }
        WM_MOUSEWHEEL => i.motion[2] += ((wparam.0 >> 16) & 0xffff) as i16 as i32,
        _ => {}
    }
}

/// A window to make on the window thread: title, style, extended style, rectangle, the real
/// parent (0 for a top-level window), and where the handle goes (0 when it failed).
struct Request {
    title: Vec<u8>,
    style: u32,
    ex_style: u32,
    rect: [i32; 4],
    parent: isize,
    reply: std::sync::mpsc::Sender<isize>,
}

/// The window thread: it makes every real window (a child window's messages go to the thread
/// that made it, so parent and children share it) and pumps their messages.
fn window_thread() -> &'static std::sync::Mutex<std::sync::mpsc::Sender<Request>> {
    static THREAD: std::sync::OnceLock<std::sync::Mutex<std::sync::mpsc::Sender<Request>>> =
        std::sync::OnceLock::new();
    THREAD.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<Request>();
        let _ = std::thread::Builder::new()
            .name("nv-d3d9-window".into())
            .spawn(move || unsafe { run_window_thread(rx) });
        std::sync::Mutex::new(tx)
    })
}

unsafe fn run_window_thread(rx: std::sync::mpsc::Receiver<Request>) {
    let module = GetModuleHandleA(None).unwrap_or_default();
    let class = WNDCLASSA {
        lpfnWndProc: Some(window_procedure),
        hInstance: module.into(),
        hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    RegisterClassA(&class);
    loop {
        while let Ok(request) = rx.try_recv() {
            let mut title = request.title;
            title.push(0);
            let [left, top, right, bottom] = request.rect;
            let child = request.parent != 0;
            // A top-level window is shown afterwards without taking the focus.
            let style = if child {
                request.style
            } else {
                request.style & !WS_VISIBLE.0
            };
            let window = CreateWindowExA(
                WINDOW_EX_STYLE(request.ex_style),
                CLASS_NAME,
                PCSTR(title.as_ptr()),
                WINDOW_STYLE(style),
                left,
                top,
                right.wrapping_sub(left),
                bottom.wrapping_sub(top),
                HWND(request.parent as *mut _),
                None,
                module,
                None,
            );
            let handle = match window {
                Ok(w) => {
                    if !child && request.style & WS_VISIBLE.0 != 0 {
                        let _ = ShowWindow(w, SW_SHOWNOACTIVATE);
                    }
                    w.0 as isize
                }
                Err(_) => 0,
            };
            let _ = request.reply.send(handle);
        }
        MsgWaitForMultipleObjects(None, false, 10, QS_ALLINPUT);
        let mut msg = MSG::default();
        while PeekMessageA(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }
    }
}

/// `CreateWindowExA` made the modelled window `handle` (a child of `parent` when that is not
/// 0): a real window of the same title, styles and rectangle (a child of the parent's real
/// window).
#[allow(clippy::too_many_arguments)]
pub fn window_created(
    e: &mut Engine,
    handle: u32,
    title: &[u8],
    style: u32,
    ex_style: u32,
    rect: [i32; 4],
    parent: u32,
) {
    if !wanted() {
        return;
    }
    let real_parent = if parent != 0 {
        match real_window(e, parent) {
            Some(p) => p,
            None => {
                eprintln!("NV_D3D9=real: a child window of a window without a real one");
                return;
            }
        }
    } else {
        0
    };
    let (reply, answer) = std::sync::mpsc::channel();
    let request = Request {
        title: title.to_vec(),
        style,
        ex_style,
        rect,
        parent: real_parent,
        reply,
    };
    let sent = window_thread()
        .lock()
        .map(|tx| tx.send(request).is_ok())
        .unwrap_or(false);
    match answer.recv().ok().filter(|&w| sent && w != 0) {
        Some(window) => {
            eprintln!(
                "NV_D3D9=real: window {handle:#x} (style {style:#x}, parent {parent:#x}, {rect:?}) is real window {window:#x}"
            );
            real(e).windows.insert(handle, window);
        }
        None => eprintln!("NV_D3D9=real: the real window could not be made"),
    }
}

/// `SetWindowPos` moved or sized the modelled window `handle` (`SWP_NOSIZE`, `SWP_NOMOVE` in
/// `flags`): the real window follows.
pub fn window_moved(e: &mut Engine, handle: u32, rect: [i32; 4], flags: u32) {
    let Some(window) = real_window(e, handle) else {
        return;
    };
    let [left, top, right, bottom] = rect;
    unsafe {
        let _ = SetWindowPos(
            HWND(window as *mut _),
            None,
            left,
            top,
            right.wrapping_sub(left),
            bottom.wrapping_sub(top),
            SET_WINDOW_POS_FLAGS(flags & 3) | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

// The device.

/// `Direct3DCreate9` made a modelled `IDirect3D9`: the real one (once).
pub fn direct3d_created(e: &mut Engine) {
    if !wanted() || real(e).d3d.is_some() {
        return;
    }
    match unsafe { Direct3DCreate9(32) } {
        Some(d3d) => real(e).d3d = Some(d3d),
        None => eprintln!("NV_D3D9=real: Direct3DCreate9 answered null"),
    }
}

/// `D3DCREATE_FPU_PRESERVE`.
const FPU_PRESERVE: u32 = 2;

/// `CreateDevice` made the modelled device `device` with these present parameters (as the
/// model filled them in), back buffers and automatic depth-stencil surface (0 for none): the
/// real device on the real window, its back buffers and depth surface as the counterparts.
pub fn device_created(
    e: &mut Engine,
    device: u32,
    focus: u32,
    behavior: u32,
    present: &[u8],
    back_buffers: &[u32],
    depth_stencil: u32,
) {
    let Some(d3d) = real(e).d3d.clone() else {
        return;
    };
    let w = |o: usize| u32::from_le_bytes(present[o..o + 4].try_into().expect("a word"));
    let map = |e: &Engine, h: u32| -> Option<HWND> {
        if h == 0 {
            return Some(HWND::default());
        }
        real_window(e, h).map(|w| HWND(w as *mut _))
    };
    let (Some(device_window), Some(focus_window)) = (map(e, w(0x1c)), map(e, focus)) else {
        eprintln!("NV_D3D9=real: CreateDevice: the game's window has no real window");
        return;
    };
    let mut pp = D3DPRESENT_PARAMETERS {
        BackBufferWidth: w(0x0),
        BackBufferHeight: w(0x4),
        BackBufferFormat: D3DFORMAT(w(0x8)),
        BackBufferCount: w(0xc),
        MultiSampleType: D3DMULTISAMPLE_TYPE(w(0x10) as i32),
        MultiSampleQuality: w(0x14),
        SwapEffect: D3DSWAPEFFECT(w(0x18) as i32),
        hDeviceWindow: device_window,
        Windowed: BOOL(w(0x20) as i32),
        EnableAutoDepthStencil: BOOL(w(0x24) as i32),
        AutoDepthStencilFormat: D3DFORMAT(w(0x28)),
        Flags: w(0x2c),
        FullScreen_RefreshRateInHz: w(0x30),
        PresentationInterval: presentation_interval(w(0x34)),
    };
    let mut out = None;
    let r = unsafe {
        d3d.CreateDevice(
            0,
            D3DDEVTYPE_HAL,
            focus_window,
            behavior | FPU_PRESERVE,
            &mut pp,
            &mut out,
        )
    };
    let Some(real_device) = out.filter(|_| r.is_ok()) else {
        eprintln!(
            "NV_D3D9=real: CreateDevice answered {:#010x}: nothing is forwarded",
            hr(r)
        );
        return;
    };
    for (i, &surface) in back_buffers.iter().enumerate() {
        if let Ok(s) = unsafe { real_device.GetBackBuffer(0, i as u32, D3DBACKBUFFER_TYPE_MONO) } {
            real(e).objects.insert(surface, Object::Surface(s));
        }
    }
    if depth_stencil != 0 {
        if let Ok(s) = unsafe { real_device.GetDepthStencilSurface() } {
            real(e).objects.insert(depth_stencil, Object::Surface(s));
        }
    }
    real(e).objects.insert(device, Object::Device(real_device));
    eprintln!(
        "NV_D3D9=real: the game's device is on the real window ({}x{}, format {}, behaviour {behavior:#x})",
        w(0x0),
        w(0x4),
        w(0x8)
    );
}

pub fn begin_scene(e: &mut Engine, this: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let r = unsafe { d.BeginScene() };
    answer(e, "BeginScene", model, hr(r));
}

pub fn end_scene(e: &mut Engine, this: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let r = unsafe { d.EndScene() };
    answer(e, "EndScene", model, hr(r));
}

#[allow(clippy::too_many_arguments)]
pub fn clear(
    e: &mut Engine,
    this: u32,
    count: u32,
    rects: u32,
    flags: u32,
    colour: u32,
    z: u32,
    stencil: u32,
    model: u32,
) {
    let Some(d) = device(e, this) else { return };
    let rect_words = if rects != 0 {
        words(e, rects, 4 * count)
    } else {
        Vec::new()
    };
    let pointer = if rects != 0 {
        rect_words.as_ptr() as *const D3DRECT
    } else {
        std::ptr::null()
    };
    let r = unsafe { d.Clear(count, pointer, flags, colour, f32::from_bits(z), stencil) };
    answer(e, "Clear", model, hr(r));
}

fn rect(e: &Engine, at: u32) -> Option<RECT> {
    (at != 0).then(|| RECT {
        left: e.mem.i32(at),
        top: e.mem.i32(at + 4),
        right: e.mem.i32(at + 8),
        bottom: e.mem.i32(at + 12),
    })
}

pub fn present(
    e: &mut Engine,
    this: u32,
    source: u32,
    dest: u32,
    window: u32,
    dirty: u32,
    model: u32,
) {
    if face_check::enabled() {
        // Diagnostic only (`NV_FACE_CHECK`, not exe behaviour).
        face_check_present();
    }
    let Some(d) = device(e, this) else { return };
    if dirty != 0 {
        answer(e, "Present with a dirty region (not forwarded)", model, 1);
        return;
    }
    if let Some(shots) = shot::wanted() {
        // Diagnostic only (`NV_SHOT`, not exe behaviour): the frame the game just drew.
        shot::at_present(&d, shots);
    }
    let (source, dest) = (rect(e, source), rect(e, dest));
    let window = match window {
        0 => HWND::default(),
        h => match real_window(e, h) {
            Some(w) => HWND(w as *mut _),
            None => return,
        },
    };
    let r = unsafe {
        d.Present(
            source
                .as_ref()
                .map_or(std::ptr::null(), |r| r as *const RECT),
            dest.as_ref().map_or(std::ptr::null(), |r| r as *const RECT),
            window,
            std::ptr::null(),
        )
    };
    answer(e, "Present", model, hr(r));
    // The frame is over: what the model let go of is bound nowhere on the real device now.
    real(e).graveyard.clear();
}

// The device's state.

fn surface_of(e: &Engine, surface: u32) -> Option<IDirect3DSurface9> {
    match e.kernel.graphics.real.objects.get(&surface) {
        Some(Object::Surface(s)) => Some(s.clone()),
        _ => None,
    }
}

/// A modelled object's real counterpart is missing: printed once by call.
fn missing(e: &mut Engine, call: &'static str) {
    answer(e, call, 0, 0xffff_ffff);
}

pub fn set_render_target(e: &mut Engine, this: u32, index: u32, surface: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let real_surface = if surface != 0 {
        match surface_of(e, surface) {
            Some(s) => Some(s),
            None => return missing(e, "SetRenderTarget of a surface without a real one"),
        }
    } else {
        None
    };
    let r = unsafe { d.SetRenderTarget(index, real_surface.as_ref()) };
    answer(e, "SetRenderTarget", model, hr(r));
}

pub fn set_depth_stencil_surface(e: &mut Engine, this: u32, surface: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let real_surface = if surface != 0 {
        match surface_of(e, surface) {
            Some(s) => Some(s),
            None => return missing(e, "SetDepthStencilSurface of a surface without a real one"),
        }
    } else {
        None
    };
    let r = unsafe { d.SetDepthStencilSurface(real_surface.as_ref()) };
    answer(e, "SetDepthStencilSurface", model, hr(r));
}

pub fn set_viewport(e: &mut Engine, this: u32, viewport: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    if viewport == 0 {
        return;
    }
    let v = words(e, viewport, 6);
    let vp = D3DVIEWPORT9 {
        X: v[0],
        Y: v[1],
        Width: v[2],
        Height: v[3],
        MinZ: f32::from_bits(v[4]),
        MaxZ: f32::from_bits(v[5]),
    };
    let r = unsafe { d.SetViewport(&vp) };
    answer(e, "SetViewport", model, hr(r));
}

pub fn set_transform(e: &mut Engine, this: u32, state: u32, matrix: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    if matrix == 0 {
        return;
    }
    let m = words(e, matrix, 16);
    if face_check::enabled() {
        // Diagnostic only (`NV_FACE_CHECK`, not exe behaviour).
        face_check::state().transforms.insert(state, m.clone());
    }
    let r = unsafe { d.SetTransform(D3DTRANSFORMSTATETYPE(state as i32), m.as_ptr() as *const _) };
    answer(e, "SetTransform", model, hr(r));
}

pub fn set_scissor_rect(e: &mut Engine, this: u32, at: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let Some(r) = rect(e, at) else { return };
    let result = unsafe { d.SetScissorRect(&r) };
    answer(e, "SetScissorRect", model, hr(result));
}

pub fn set_clip_plane(e: &mut Engine, this: u32, index: u32, plane: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    if plane == 0 {
        return;
    }
    let p = words(e, plane, 4);
    let r = unsafe { d.SetClipPlane(index, p.as_ptr() as *const f32) };
    answer(e, "SetClipPlane", model, hr(r));
}

pub fn set_render_state(e: &mut Engine, this: u32, state: u32, value: u32, model: u32) {
    if face_check::enabled() {
        // Diagnostic only (`NV_FACE_CHECK`, not exe behaviour).
        face_check_render_state(state, value);
    }
    let Some(d) = device(e, this) else { return };
    let r = unsafe { d.SetRenderState(D3DRENDERSTATETYPE(state as i32), value) };
    answer(e, "SetRenderState", model, hr(r));
}

pub fn set_texture_stage_state(
    e: &mut Engine,
    this: u32,
    stage: u32,
    kind: u32,
    value: u32,
    model: u32,
) {
    let Some(d) = device(e, this) else { return };
    let r = unsafe { d.SetTextureStageState(stage, D3DTEXTURESTAGESTATETYPE(kind as i32), value) };
    answer(e, "SetTextureStageState", model, hr(r));
}

pub fn set_sampler_state(
    e: &mut Engine,
    this: u32,
    sampler: u32,
    kind: u32,
    value: u32,
    model: u32,
) {
    let Some(d) = device(e, this) else { return };
    let r = unsafe { d.SetSamplerState(sampler, D3DSAMPLERSTATETYPE(kind as i32), value) };
    answer(e, "SetSamplerState", model, hr(r));
}

pub fn set_texture(e: &mut Engine, this: u32, sampler: u32, texture: u32, model: u32) {
    if face_check::enabled() {
        // Diagnostic only (`NV_FACE_CHECK`, not exe behaviour).
        face_check_bind(sampler, texture);
    }
    let Some(d) = device(e, this) else { return };
    let r = match e.kernel.graphics.real.objects.get(&texture) {
        _ if texture == 0 => unsafe { d.SetTexture(sampler, None::<&IDirect3DBaseTexture9>) },
        Some(Object::Texture(t)) => unsafe { d.SetTexture(sampler, t) },
        Some(Object::Cube(t)) => unsafe { d.SetTexture(sampler, t) },
        _ => return missing(e, "SetTexture of a texture without a real one"),
    };
    answer(e, "SetTexture", model, hr(r));
}

// Textures and surfaces.

/// `CreateTexture` made the modelled texture `texture` with these levels (largest first): the
/// real texture with the same arguments, its levels the counterparts of the modelled levels.
#[allow(clippy::too_many_arguments)]
pub fn texture_created(
    e: &mut Engine,
    this: u32,
    texture: u32,
    levels: &[u32],
    width: u32,
    height: u32,
    usage: u32,
    format: u32,
    pool: u32,
) {
    let Some(d) = device(e, this) else { return };
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
        return answer(e, "CreateTexture", 0, hr(r));
    };
    for (i, &level) in levels.iter().enumerate() {
        if let Ok(s) = unsafe { t.GetSurfaceLevel(i as u32) } {
            real(e).objects.insert(level, Object::Surface(s));
        }
    }
    real(e).objects.insert(texture, Object::Texture(t));
    if face_check::enabled() {
        // Diagnostic only (`NV_FACE_CHECK`, not exe behaviour).
        face_check_texture(texture, levels, (width, height), format, usage, pool);
    }
}

/// `CreateCubeTexture` made the modelled cube texture `texture` with these surfaces, face by
/// face (each face's chain largest first).
#[allow(clippy::too_many_arguments)]
pub fn cube_created(
    e: &mut Engine,
    this: u32,
    texture: u32,
    surfaces: &[u32],
    edge: u32,
    usage: u32,
    format: u32,
    pool: u32,
) {
    let Some(d) = device(e, this) else { return };
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
        return answer(e, "CreateCubeTexture", 0, hr(r));
    };
    for (i, &surface) in surfaces.iter().enumerate() {
        let (face, level) = (i as u32 / levels, i as u32 % levels);
        if let Ok(s) = unsafe { t.GetCubeMapSurface(D3DCUBEMAP_FACES(face as i32), level) } {
            real(e).objects.insert(surface, Object::Surface(s));
        }
    }
    real(e).objects.insert(texture, Object::Cube(t));
}

/// `CreateRenderTarget` (`depth` false) or `CreateDepthStencilSurface` made the modelled
/// surface `surface`; `flag` is `Lockable` or `Discard`.
#[allow(clippy::too_many_arguments)]
pub fn surface_created(
    e: &mut Engine,
    this: u32,
    surface: u32,
    depth: bool,
    width: u32,
    height: u32,
    format: u32,
    multisample: u32,
    quality: u32,
    flag: u32,
) {
    let Some(d) = device(e, this) else { return };
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
        Some(s) => {
            real(e).objects.insert(surface, Object::Surface(s));
        }
        None => answer(e, "CreateRenderTarget/DepthStencilSurface", 0, hr(r)),
    }
}

/// The game unlocked the modelled surface `surface`, whose memory in engine memory is `bits`
/// (`rows` rows of `row` bytes, `pitch` apart): the bytes go to the real surface.
pub fn surface_unlocked(e: &mut Engine, surface: u32, bits: u32, pitch: u32, row: u32, rows: u32) {
    let Some(s) = surface_of(e, surface) else {
        return;
    };
    let mut locked = D3DLOCKED_RECT::default();
    let r = unsafe { s.LockRect(&mut locked, std::ptr::null(), 0) };
    if r.is_err() {
        return answer(e, "LockRect of the real surface", 0, hr(r));
    }
    for y in 0..rows {
        let to = unsafe {
            std::slice::from_raw_parts_mut(
                (locked.pBits as *mut u8).add(y as usize * locked.Pitch as usize),
                row as usize,
            )
        };
        e.mem.read(bits + y * pitch, to);
    }
    let _ = unsafe { s.UnlockRect() };
    if face_check::enabled() {
        // Diagnostic only (`NV_FACE_CHECK`, not exe behaviour).
        face_check_upload(e, surface, &s, bits, row, rows);
    }
}

// Buffers, shaders, declarations.

/// `D3DUSAGE_DYNAMIC`, `D3DLOCK_READONLY`, `D3DLOCK_DISCARD`, `D3DLOCK_NOOVERWRITE`.
const USAGE_DYNAMIC: u32 = 0x200;
const LOCK_READONLY: u32 = 0x10;
const LOCK_DISCARD: u32 = 0x2000;
const LOCK_NOOVERWRITE: u32 = 0x1000;

/// `CreateVertexBuffer` (`index` false) or `CreateIndexBuffer` made the modelled buffer
/// `buffer`; `format` is the FVF or the index format.
#[allow(clippy::too_many_arguments)]
pub fn buffer_created(
    e: &mut Engine,
    this: u32,
    buffer: u32,
    index: bool,
    length: u32,
    usage: u32,
    format: u32,
    pool: u32,
) {
    let Some(d) = device(e, this) else { return };
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
            None => return answer(e, "CreateIndexBuffer", 0, hr(r)),
        }
    } else {
        let mut out = None;
        let r = unsafe {
            d.CreateVertexBuffer(length, usage, format, D3DPOOL(pool as i32), &mut out, null)
        };
        match out.filter(|_| r.is_ok()) {
            Some(b) => Object::VertexBuffer(b),
            None => return answer(e, "CreateVertexBuffer", 0, hr(r)),
        }
    };
    let r = real(e);
    r.objects.insert(buffer, object);
    r.locks.remove(&buffer);
    if usage & USAGE_DYNAMIC != 0 {
        r.dynamic.insert(buffer);
    } else {
        r.dynamic.remove(&buffer);
    }
}

/// The game locked the modelled buffer `buffer` (recorded for its unlock).
pub fn buffer_locked(e: &mut Engine, buffer: u32, offset: u32, size: u32, flags: u32, model: u32) {
    if model != 0 || !e.kernel.graphics.real.objects.contains_key(&buffer) {
        return;
    }
    real(e)
        .locks
        .entry(buffer)
        .or_default()
        .push((offset, size, flags));
}

/// The game unlocked the modelled buffer `buffer`, whose memory is `bits` (`length` bytes):
/// the locked range goes to the real buffer.
pub fn buffer_unlocked(e: &mut Engine, buffer: u32, bits: u32, length: u32, model: u32) {
    if model != 0 {
        return;
    }
    let Some((offset, size, flags)) = real(e).locks.get_mut(&buffer).and_then(|l| l.pop()) else {
        return;
    };
    if flags & LOCK_READONLY != 0 {
        return;
    }
    let size = if size == 0 { length - offset } else { size };
    let pass = if e.kernel.graphics.real.dynamic.contains(&buffer) {
        flags & (LOCK_DISCARD | LOCK_NOOVERWRITE)
    } else {
        0
    };
    let mut data: *mut core::ffi::c_void = std::ptr::null_mut();
    let r = match e.kernel.graphics.real.objects.get(&buffer) {
        Some(Object::VertexBuffer(b)) => unsafe { b.Lock(offset, size, &mut data, pass) },
        Some(Object::IndexBuffer(b)) => unsafe { b.Lock(offset, size, &mut data, pass) },
        _ => return,
    };
    if r.is_err() || data.is_null() {
        return answer(e, "Lock of the real buffer", 0, hr(r));
    }
    let to = unsafe { std::slice::from_raw_parts_mut(data as *mut u8, size as usize) };
    e.mem.read(bits + offset, to);
    let _ = match e.kernel.graphics.real.objects.get(&buffer) {
        Some(Object::VertexBuffer(b)) => unsafe { b.Unlock() },
        Some(Object::IndexBuffer(b)) => unsafe { b.Unlock() },
        _ => Ok(()),
    };
}

fn as_words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .chunks(4)
        .map(|c| {
            let mut w = [0u8; 4];
            w[..c.len()].copy_from_slice(c);
            u32::from_le_bytes(w)
        })
        .collect()
}

/// `CreateVertexShader` / `CreatePixelShader` made the modelled shader `shader` from these
/// tokens.
pub fn shader_created(e: &mut Engine, this: u32, shader: u32, pixel: bool, tokens: &[u8]) {
    let Some(d) = device(e, this) else { return };
    let tokens = as_words(tokens);
    let object = if pixel {
        match unsafe { d.CreatePixelShader(tokens.as_ptr()) } {
            Ok(s) => Object::PixelShader(s),
            Err(err) => return answer(e, "CreatePixelShader", 0, err.code().0 as u32),
        }
    } else {
        match unsafe { d.CreateVertexShader(tokens.as_ptr()) } {
            Ok(s) => Object::VertexShader(s),
            Err(err) => return answer(e, "CreateVertexShader", 0, err.code().0 as u32),
        }
    };
    real(e).objects.insert(shader, object);
}

/// `CreateVertexDeclaration` made the modelled declaration `declaration` from these elements
/// (`D3DDECL_END` included).
pub fn declaration_created(e: &mut Engine, this: u32, declaration: u32, elements: &[u8]) {
    let Some(d) = device(e, this) else { return };
    let elements = as_words(elements);
    match unsafe { d.CreateVertexDeclaration(elements.as_ptr() as *const D3DVERTEXELEMENT9) } {
        Ok(v) => {
            real(e).objects.insert(declaration, Object::Declaration(v));
        }
        Err(err) => answer(e, "CreateVertexDeclaration", 0, err.code().0 as u32),
    }
}

pub fn set_vertex_declaration(e: &mut Engine, this: u32, declaration: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let r = match e.kernel.graphics.real.objects.get(&declaration) {
        _ if declaration == 0 => unsafe {
            d.SetVertexDeclaration(None::<&IDirect3DVertexDeclaration9>)
        },
        Some(Object::Declaration(v)) => unsafe { d.SetVertexDeclaration(v) },
        _ => {
            let _ = unsafe { d.SetVertexDeclaration(None::<&IDirect3DVertexDeclaration9>) };
            return broken(
                e,
                this,
                DECLARATION,
                "SetVertexDeclaration without a real one",
                model,
            );
        }
    };
    mended(e, this, DECLARATION);
    answer(e, "SetVertexDeclaration", model, hr(r));
}

pub fn set_fvf(e: &mut Engine, this: u32, fvf: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let r = unsafe { d.SetFVF(fvf) };
    answer(e, "SetFVF", model, hr(r));
}

pub fn set_shader(e: &mut Engine, this: u32, shader: u32, pixel: bool, model: u32) {
    if face_check::enabled() {
        // Diagnostic only (`NV_FACE_CHECK`, not exe behaviour).
        face_check_shader(shader, pixel);
    }
    let Some(d) = device(e, this) else { return };
    let r = match (pixel, e.kernel.graphics.real.objects.get(&shader)) {
        (false, _) if shader == 0 => unsafe { d.SetVertexShader(None::<&IDirect3DVertexShader9>) },
        (true, _) if shader == 0 => unsafe { d.SetPixelShader(None::<&IDirect3DPixelShader9>) },
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
            return broken(e, this, which, "Set*Shader without a real one", model);
        }
    };
    mended(e, this, if pixel { PIXEL_SHADER } else { VERTEX_SHADER });
    answer(e, "Set*Shader", model, hr(r));
}

/// `Set{Vertex,Pixel}ShaderConstant{F,I,B}`: kinds 0..2 vertex float, int, bool; 3..5 pixel.
pub fn set_constants(
    e: &mut Engine,
    this: u32,
    kind: u8,
    start: u32,
    data: u32,
    count: u32,
    model: u32,
) {
    let Some(d) = device(e, this) else { return };
    if data == 0 {
        return;
    }
    if face_check::enabled() && (kind == 0 || kind == 3) {
        // Diagnostic only (`NV_FACE_CHECK`, not exe behaviour).
        face_check_constants(e, kind == 3, start, data, count);
    }
    let per = if kind % 3 == 2 { 1 } else { 4 };
    let v = words(e, data, per * count);
    let p = v.as_ptr();
    let r = unsafe {
        match kind {
            0 => d.SetVertexShaderConstantF(start, p as *const f32, count),
            1 => d.SetVertexShaderConstantI(start, p as *const i32, count),
            2 => d.SetVertexShaderConstantB(start, p as *const BOOL, count),
            3 => d.SetPixelShaderConstantF(start, p as *const f32, count),
            4 => d.SetPixelShaderConstantI(start, p as *const i32, count),
            _ => d.SetPixelShaderConstantB(start, p as *const BOOL, count),
        }
    };
    answer(e, "Set*ShaderConstant*", model, hr(r));
}

pub fn set_stream_source(
    e: &mut Engine,
    this: u32,
    stream: u32,
    buffer: u32,
    offset: u32,
    stride: u32,
    model: u32,
) {
    let Some(d) = device(e, this) else { return };
    let r = match e.kernel.graphics.real.objects.get(&buffer) {
        _ if buffer == 0 => unsafe {
            d.SetStreamSource(stream, None::<&IDirect3DVertexBuffer9>, offset, stride)
        },
        Some(Object::VertexBuffer(b)) => unsafe { d.SetStreamSource(stream, b, offset, stride) },
        _ => {
            let _ = unsafe { d.SetStreamSource(stream, None::<&IDirect3DVertexBuffer9>, 0, 0) };
            return broken(
                e,
                this,
                stream,
                "SetStreamSource without a real buffer",
                model,
            );
        }
    };
    mended(e, this, stream);
    answer(e, "SetStreamSource", model, hr(r));
}

pub fn set_indices(e: &mut Engine, this: u32, buffer: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let r = match e.kernel.graphics.real.objects.get(&buffer) {
        _ if buffer == 0 => unsafe { d.SetIndices(None::<&IDirect3DIndexBuffer9>) },
        Some(Object::IndexBuffer(b)) => unsafe { d.SetIndices(b) },
        _ => {
            let _ = unsafe { d.SetIndices(None::<&IDirect3DIndexBuffer9>) };
            return broken(e, this, INDICES, "SetIndices without a real buffer", model);
        }
    };
    mended(e, this, INDICES);
    answer(e, "SetIndices", model, hr(r));
}

/// The bindings [`Real::broken`] names besides the streams (0..16).
const INDICES: u32 = 0x100;
const DECLARATION: u32 = 0x101;
const VERTEX_SHADER: u32 = 0x102;
const PIXEL_SHADER: u32 = 0x103;

/// A binding the model took has no real counterpart (printed once by call).
fn broken(e: &mut Engine, device: u32, binding: u32, call: &'static str, model: u32) {
    real(e).broken.insert((device, binding));
    answer(e, call, model, 0xffff_ffff);
}

fn mended(e: &mut Engine, device: u32, binding: u32) {
    real(e).broken.remove(&(device, binding));
}

/// Whether the real draws of `device` are skipped (a binding without a real counterpart).
fn skipped(e: &mut Engine, device: u32) -> bool {
    let any = e
        .kernel
        .graphics
        .real
        .broken
        .iter()
        .any(|&(d, _)| d == device);
    if any {
        missing(e, "a draw skipped: a binding has no real counterpart");
    }
    any
}

// Draws.

pub fn draw_primitive(e: &mut Engine, this: u32, kind: u32, start: u32, count: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    if skipped(e, this) {
        return;
    }
    let r = unsafe { d.DrawPrimitive(D3DPRIMITIVETYPE(kind as i32), start, count) };
    answer(e, "DrawPrimitive", model, hr(r));
}

#[allow(clippy::too_many_arguments)]
pub fn draw_indexed_primitive(
    e: &mut Engine,
    this: u32,
    kind: u32,
    base_vertex: u32,
    min_index: u32,
    vertices: u32,
    start_index: u32,
    count: u32,
    model: u32,
) {
    let Some(d) = device(e, this) else { return };
    let skip = skipped(e, this);
    if face_check::enabled() {
        // Diagnostic only (`NV_FACE_CHECK`, not exe behaviour).
        face_check_draw(
            kind,
            base_vertex,
            min_index,
            vertices,
            start_index,
            count,
            skip,
        );
    }
    if skip {
        return;
    }
    let r = unsafe {
        d.DrawIndexedPrimitive(
            D3DPRIMITIVETYPE(kind as i32),
            base_vertex as i32,
            min_index,
            vertices,
            start_index,
            count,
        )
    };
    answer(e, "DrawIndexedPrimitive", model, hr(r));
}

/// `DrawPrimitiveUP`: the `vertices * stride` bytes at `data` from engine memory.
#[allow(clippy::too_many_arguments)]
pub fn draw_primitive_up(
    e: &mut Engine,
    this: u32,
    kind: u32,
    count: u32,
    data: u32,
    stride: u32,
    vertices: u32,
    model: u32,
) {
    let Some(d) = device(e, this) else { return };
    if skipped(e, this) {
        return;
    }
    let bytes = as_words(&e.mem.bytes(data, vertices * stride));
    let r = unsafe {
        d.DrawPrimitiveUP(
            D3DPRIMITIVETYPE(kind as i32),
            count,
            bytes.as_ptr() as *const _,
            stride,
        )
    };
    answer(e, "DrawPrimitiveUP", model, hr(r));
}

// Copies between resources, fixed-function lighting.

fn base_texture(e: &Engine, texture: u32) -> Option<IDirect3DBaseTexture9> {
    use windows::core::Interface;
    match e.kernel.graphics.real.objects.get(&texture) {
        Some(Object::Texture(t)) => t.cast().ok(),
        Some(Object::Cube(t)) => t.cast().ok(),
        _ => None,
    }
}

pub fn update_texture(e: &mut Engine, this: u32, source: u32, destination: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let (Some(from), Some(to)) = (base_texture(e, source), base_texture(e, destination)) else {
        return missing(e, "UpdateTexture without real textures");
    };
    let r = unsafe { d.UpdateTexture(&from, &to) };
    answer(e, "UpdateTexture", model, hr(r));
}

#[allow(clippy::too_many_arguments)]
pub fn stretch_rect(
    e: &mut Engine,
    this: u32,
    source: u32,
    source_rect: u32,
    destination: u32,
    destination_rect: u32,
    filter: u32,
    model: u32,
) {
    let Some(d) = device(e, this) else { return };
    let (Some(from), Some(to)) = (surface_of(e, source), surface_of(e, destination)) else {
        return missing(e, "StretchRect without real surfaces");
    };
    let (a, b) = (rect(e, source_rect), rect(e, destination_rect));
    let r = unsafe {
        d.StretchRect(
            &from,
            a.as_ref().map_or(std::ptr::null(), |r| r as *const RECT),
            &to,
            b.as_ref().map_or(std::ptr::null(), |r| r as *const RECT),
            D3DTEXTUREFILTERTYPE(filter as i32),
        )
    };
    answer(e, "StretchRect", model, hr(r));
}

pub fn set_material(e: &mut Engine, this: u32, material: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    if material == 0 {
        return;
    }
    let m = words(e, material, 17);
    let r = unsafe { d.SetMaterial(m.as_ptr() as *const D3DMATERIAL9) };
    answer(e, "SetMaterial", model, hr(r));
}

pub fn set_light(e: &mut Engine, this: u32, index: u32, light: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    if light == 0 {
        return;
    }
    let l = words(e, light, 26);
    let r = unsafe { d.SetLight(index, l.as_ptr() as *const D3DLIGHT9) };
    answer(e, "SetLight", model, hr(r));
}

pub fn light_enable(e: &mut Engine, this: u32, index: u32, enable: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let r = unsafe { d.LightEnable(index, BOOL(enable as i32)) };
    answer(e, "LightEnable", model, hr(r));
}

pub fn set_software_vertex_processing(e: &mut Engine, this: u32, software: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let r = unsafe { d.SetSoftwareVertexProcessing(BOOL(software as i32)) };
    answer(e, "SetSoftwareVertexProcessing", model, hr(r));
}

// Queries, offscreen surfaces, fills and read-backs.

/// `CreateQuery` made the modelled query `query` of type `kind`: the real query.
pub fn query_created(e: &mut Engine, this: u32, query: u32, kind: u32) {
    let Some(d) = device(e, this) else { return };
    match unsafe { d.CreateQuery(D3DQUERYTYPE(kind as i32)) } {
        Ok(q) => {
            real(e).objects.insert(query, Object::Query(q));
        }
        Err(err) => answer(e, "CreateQuery", 0, err.code().0 as u32),
    }
}

fn query_of(e: &Engine, query: u32) -> Option<IDirect3DQuery9> {
    match e.kernel.graphics.real.objects.get(&query) {
        Some(Object::Query(q)) => Some(q.clone()),
        _ => None,
    }
}

pub fn query_issue(e: &mut Engine, query: u32, flags: u32) {
    let Some(q) = query_of(e, query) else { return };
    let r = unsafe { q.Issue(flags) };
    answer(e, "Query::Issue", 0, hr(r));
}

/// `GetData` of the real query after the game's `END`: its answer and its 4-byte result; `None`
/// without a real query.
pub fn query_get_data(e: &mut Engine, query: u32, flags: u32) -> Option<(u32, u32)> {
    let q = query_of(e, query)?;
    let mut value = 0u32;
    // The raw slot: `S_FALSE` (not done yet) is a success code the wrapper would fold into Ok.
    let hr = unsafe {
        use windows::core::Interface;
        (Interface::vtable(&q).GetData)(
            Interface::as_raw(&q),
            &mut value as *mut u32 as *mut core::ffi::c_void,
            4,
            flags,
        )
    };
    Some((hr.0 as u32, value))
}

/// `CreateOffscreenPlainSurface` made the modelled surface `surface`.
pub fn offscreen_created(
    e: &mut Engine,
    this: u32,
    surface: u32,
    width: u32,
    height: u32,
    format: u32,
    pool: u32,
) {
    let Some(d) = device(e, this) else { return };
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
        Some(s) => {
            real(e).objects.insert(surface, Object::Surface(s));
        }
        None => answer(e, "CreateOffscreenPlainSurface", 0, hr(r)),
    }
}

pub fn color_fill(e: &mut Engine, this: u32, surface: u32, at: u32, colour: u32, model: u32) {
    let Some(d) = device(e, this) else { return };
    let Some(s) = surface_of(e, surface) else {
        return missing(e, "ColorFill of a surface without a real one");
    };
    let area = rect(e, at);
    let r = unsafe {
        d.ColorFill(
            &s,
            area.as_ref().map_or(std::ptr::null(), |r| r as *const RECT),
            colour,
        )
    };
    answer(e, "ColorFill", model, hr(r));
}

/// `GetRenderTargetData` from the real render target into the real destination, whose rows are
/// then copied into the destination's memory at `bits` (`pitch` apart); false without real
/// surfaces (the caller stops).
pub fn get_render_target_data(
    e: &mut Engine,
    this: u32,
    source: u32,
    destination: u32,
    bits: u32,
    pitch: u32,
) -> bool {
    let Some(d) = device(e, this) else {
        return false;
    };
    let (Some(from), Some(to)) = (surface_of(e, source), surface_of(e, destination)) else {
        return false;
    };
    let r = unsafe { d.GetRenderTargetData(&from, &to) };
    answer(e, "GetRenderTargetData", 0, hr(r));
    let mut desc = D3DSURFACE_DESC::default();
    let mut locked = D3DLOCKED_RECT::default();
    unsafe {
        if to.GetDesc(&mut desc).is_err()
            || to.LockRect(&mut locked, std::ptr::null(), 0x10).is_err()
        {
            return true;
        }
        let row = pitch.min(locked.Pitch as u32) as usize;
        for y in 0..desc.Height {
            let from_row = std::slice::from_raw_parts(
                (locked.pBits as *const u8).add(y as usize * locked.Pitch as usize),
                row,
            );
            e.mem.write(bits + y * pitch, from_row);
        }
        let _ = to.UnlockRect();
    }
    true
}

/// `Reset` succeeded on the modelled device: the real device is reset with the same present
/// parameters (the real window), and its new back buffers and depth surface are the
/// counterparts of the modelled ones.
pub fn device_reset(
    e: &mut Engine,
    this: u32,
    present: &[u8],
    back_buffers: &[u32],
    depth_stencil: u32,
) {
    let Some(d) = device(e, this) else { return };
    let w = |o: usize| u32::from_le_bytes(present[o..o + 4].try_into().expect("a word"));
    let window = match w(0x1c) {
        0 => HWND::default(),
        h => match real_window(e, h) {
            Some(r) => HWND(r as *mut _),
            None => return missing(e, "Reset with a window without a real one"),
        },
    };
    let mut pp = D3DPRESENT_PARAMETERS {
        BackBufferWidth: w(0x0),
        BackBufferHeight: w(0x4),
        BackBufferFormat: D3DFORMAT(w(0x8)),
        BackBufferCount: w(0xc),
        MultiSampleType: D3DMULTISAMPLE_TYPE(w(0x10) as i32),
        MultiSampleQuality: w(0x14),
        SwapEffect: D3DSWAPEFFECT(w(0x18) as i32),
        hDeviceWindow: window,
        Windowed: BOOL(w(0x20) as i32),
        EnableAutoDepthStencil: BOOL(w(0x24) as i32),
        AutoDepthStencilFormat: D3DFORMAT(w(0x28)),
        Flags: w(0x2c),
        FullScreen_RefreshRateInHz: w(0x30),
        PresentationInterval: presentation_interval(w(0x34)),
    };
    let r = unsafe { d.Reset(&mut pp) };
    answer(e, "Reset", 0, hr(r));
    for (i, &surface) in back_buffers.iter().enumerate() {
        if let Ok(s) = unsafe { d.GetBackBuffer(0, i as u32, D3DBACKBUFFER_TYPE_MONO) } {
            real(e).objects.insert(surface, Object::Surface(s));
        }
    }
    if depth_stencil != 0 {
        if let Ok(s) = unsafe { d.GetDepthStencilSurface() } {
            real(e).objects.insert(depth_stencil, Object::Surface(s));
        }
    }
}

/// `SetLOD` on a managed modelled texture: the real texture's `SetLOD` with the same level; a
/// different previous level than the model's is printed once.
pub fn texture_set_lod(e: &mut Engine, texture: u32, lod: u32, model: u32) {
    let Some(t) = base_texture(e, texture) else {
        return missing(e, "SetLOD of a texture without a real one");
    };
    let previous = unsafe { t.SetLOD(lod) };
    answer(e, "SetLOD (the previous level)", model, previous);
}

// ---------------------------------------------------------------------------------------------
// `NV_SHOT=DIR[,every_ms]`: diagnostic only, not exe behaviour. Screenshots of the game's own
// frames on the real device: at a `Present`, at most every `every_ms` (default 5000) of wall
// time, the back buffer is copied to a system-memory surface (`GetRenderTargetData`; a
// multisampled back buffer is first resolved into a plain render target with `StretchRect`)
// and written to `DIR` as a 32-bit BMP named `frame-NNNNNN-<unix ms>.bmp`. Nothing is read or
// made when `NV_SHOT` is unset; the game's state and the model are not touched.
// ---------------------------------------------------------------------------------------------
mod shot {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    /// The directory and the interval (`NV_SHOT=DIR[,every_ms]`), read once.
    pub(super) struct Shots {
        pub dir: PathBuf,
        pub every: Duration,
    }

    /// `NV_SHOT=DIR[,every_ms]`: the directory and interval (5000 ms when not given).
    pub(super) fn parse(value: &str) -> Option<Shots> {
        let value = value.trim();
        if value.is_empty() {
            return None;
        }
        let (dir, every) = match value.rsplit_once(',') {
            Some((dir, ms)) => match ms.trim().parse::<u64>() {
                Ok(ms) => (dir, ms),
                Err(_) => (value, 5000),
            },
            None => (value, 5000),
        };
        Some(Shots {
            dir: PathBuf::from(dir),
            every: Duration::from_millis(every.max(1)),
        })
    }

    pub(super) fn wanted() -> Option<&'static Shots> {
        static SHOTS: OnceLock<Option<Shots>> = OnceLock::new();
        SHOTS
            .get_or_init(|| {
                let shots = std::env::var("NV_SHOT").ok().and_then(|v| parse(&v))?;
                let _ = std::fs::create_dir_all(&shots.dir);
                Some(shots)
            })
            .as_ref()
    }

    /// The presents counted and when the last shot was taken.
    fn timing() -> std::sync::MutexGuard<'static, (u64, Option<Instant>)> {
        static T: OnceLock<Mutex<(u64, Option<Instant>)>> = OnceLock::new();
        T.get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A top-down 32-bit BMP of `width` x `height` B, G, R, X pixels.
    pub(super) fn bmp(width: u32, height: u32, bgrx: &[u8]) -> Vec<u8> {
        let image = width * height * 4;
        let mut out = Vec::with_capacity(54 + image as usize);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(54 + image).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(width as i32).to_le_bytes());
        // A negative height: rows top first.
        out.extend_from_slice(&(-(height as i32)).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&image.to_le_bytes());
        out.extend_from_slice(&[0u8; 16]);
        out.extend_from_slice(bgrx);
        out
    }

    /// At a `Present`: one shot when the interval has passed since the last.
    pub(super) fn at_present(d: &IDirect3DDevice9, shots: &Shots) {
        let frame = {
            let mut t = timing();
            t.0 += 1;
            if t.1.is_some_and(|last| last.elapsed() < shots.every) {
                return;
            }
            t.1 = Some(Instant::now());
            t.0
        };
        let wall = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let path = shots.dir.join(format!("frame-{frame:06}-{wall}.bmp"));
        match capture(d) {
            Ok((width, height, pixels)) => {
                if let Err(error) = std::fs::write(&path, bmp(width, height, &pixels)) {
                    eprintln!("NV_SHOT: {} not written: {error}", path.display());
                }
            }
            Err(error) => eprintln!("NV_SHOT: frame {frame}: {error}"),
        }
    }

    /// The back buffer's pixels (rows top first, B, G, R, X) through a system-memory copy.
    fn capture(d: &IDirect3DDevice9) -> Result<(u32, u32, Vec<u8>), String> {
        let back = unsafe { d.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO) }
            .map_err(|e| format!("GetBackBuffer: {e}"))?;
        let mut desc = D3DSURFACE_DESC::default();
        unsafe { back.GetDesc(&mut desc) }.map_err(|e| format!("GetDesc: {e}"))?;
        if desc.Format != D3DFMT_X8R8G8B8 && desc.Format != D3DFMT_A8R8G8B8 {
            return Err(format!("back buffer format {} not provided", desc.Format.0));
        }
        let (width, height) = (desc.Width, desc.Height);
        // A multisampled back buffer is resolved into a plain render target first.
        let source = if desc.MultiSampleType != D3DMULTISAMPLE_NONE {
            let mut plain = None;
            unsafe {
                d.CreateRenderTarget(
                    width,
                    height,
                    desc.Format,
                    D3DMULTISAMPLE_NONE,
                    0,
                    BOOL(0),
                    &mut plain,
                    std::ptr::null_mut(),
                )
            }
            .map_err(|e| format!("CreateRenderTarget: {e}"))?;
            let plain = plain.ok_or("CreateRenderTarget: no surface")?;
            unsafe {
                d.StretchRect(
                    &back,
                    std::ptr::null(),
                    &plain,
                    std::ptr::null(),
                    D3DTEXF_NONE,
                )
            }
            .map_err(|e| format!("StretchRect: {e}"))?;
            plain
        } else {
            back
        };
        let mut copy = None;
        unsafe {
            d.CreateOffscreenPlainSurface(
                width,
                height,
                desc.Format,
                D3DPOOL_SYSTEMMEM,
                &mut copy,
                std::ptr::null_mut(),
            )
        }
        .map_err(|e| format!("CreateOffscreenPlainSurface: {e}"))?;
        let copy = copy.ok_or("CreateOffscreenPlainSurface: no surface")?;
        unsafe { d.GetRenderTargetData(&source, &copy) }
            .map_err(|e| format!("GetRenderTargetData: {e}"))?;
        let mut locked = D3DLOCKED_RECT::default();
        unsafe { copy.LockRect(&mut locked, std::ptr::null(), D3DLOCK_READONLY as u32) }
            .map_err(|e| format!("LockRect: {e}"))?;
        let row = width as usize * 4;
        let mut pixels = Vec::with_capacity(row * height as usize);
        for y in 0..height as usize {
            let line = unsafe {
                std::slice::from_raw_parts(
                    (locked.pBits as *const u8).add(y * locked.Pitch as usize),
                    row,
                )
            };
            pixels.extend_from_slice(line);
        }
        let _ = unsafe { copy.UnlockRect() };
        Ok((width, height, pixels))
    }
}

// ---------------------------------------------------------------------------------------------
// `NV_FACE_CHECK`: diagnostic only, not exe behaviour. Reads, never writes, game memory or the
// real device's state (the read-back locks the real surface read-only).
//
// Follows the player's FaceGen face texture on the real device. Nothing is printed until the
// FaceGen side has built a face image (`0064ceb0`, which stores its size in
// `platform::FACE_CHECK_IMAGES`); the first uncompressed texture of that size created after it is
// the face texture (NiDX9SourceTextureData's CreateSurf), and only it is followed, with its own
// line budget:
// - `d3d face texture` line: its size, format, levels, usage, pool.
// - `d3d face upload` lines: each level copied to the real surface (rows, the model's row bytes,
//   the real pitch, the byte means of the copied data and of the real surface read back; with
//   `NV_FACE_CHECK_DIR` levels 0 and 3 are written there as TGA files read back from the real
//   surface).
// - `d3d face bind` lines: it bound to a sampler; then `d3d face draw` for every indexed draw while
//   it is bound and for the next 16 draws after it leaves a sampler (the hair and eyes follow the
//   face): every sampler's texture (size, format, levels, and after `|` the file each was made
//   from, `platform::FACE_CHECK_NAMES`), the pixel and vertex shaders and the draw's counts;
//   each distinct draw at most twice.
//   Each such draw line also gives the primitive type, base vertex, min index and start index, and
//   whether the real draw was skipped (a binding without a real object); it is followed by
//   `d3d face states` (z, alpha test and blend, cull, stencil, colour write states as forwarded),
//   `d3d face pixel constants` (c0..c31) and `d3d face vertex constants` (c0..c95), every
//   register set so far, as floats.
//   Meshes once drawn with the face texture are followed in their other passes too (`face mesh,
//   another pass`: the head's first, depth-writing pass binds no face texture). For the first 12
//   frames after the face texture exists, `d3d face frame N` counts the indexed draws, those with
//   ZWRITEENABLE on and those skipped; once, `d3d face projection transform` prints D3DTS_PROJECTION.
//   Each reported draw also gets `d3d face states set since the previous draw` (the
//   SetRenderState calls between the previous indexed draw and this one: a state missing there was
//   left from an earlier draw), and other meshes drawn with a pixel shader a face mesh used are
//   reported too (`another mesh, same pixel shader as a face pass`: an NPC head to compare with).
// ---------------------------------------------------------------------------------------------
mod face_check {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Mutex, OnceLock};

    pub(super) fn enabled() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var_os("NV_FACE_CHECK").is_some())
    }

    static LINES: AtomicU32 = AtomicU32::new(0);
    const LIMIT: u32 = 3000;

    pub(super) fn line(text: std::fmt::Arguments) {
        if LINES.fetch_add(1, Ordering::Relaxed) < LIMIT {
            println!("face check: {text}");
        }
    }

    #[derive(Default)]
    pub(super) struct State {
        /// Every texture created: (width, height, format, levels).
        pub textures: HashMap<u32, (u32, u32, u32, u32)>,
        /// The followed face textures and their level surfaces: surface -> (texture, level).
        pub faces: Vec<u32>,
        pub levels: HashMap<u32, (u32, u32)>,
        /// The modelled texture each sampler holds (as forwarded).
        pub samplers: HashMap<u32, u32>,
        pub pixel_shader: u32,
        pub vertex_shader: u32,
        /// Draws still to report after the face texture left the samplers.
        pub after: u32,
        pub dumps: u32,
        /// How often each draw (counts, shaders, samplers) was reported: each at most twice.
        pub seen: HashMap<String, u32>,
        /// The render states and the float shader constants as forwarded (register -> 4 words).
        pub render_states: HashMap<u32, u32>,
        pub pixel_constants: HashMap<u32, [u32; 4]>,
        pub vertex_constants: HashMap<u32, [u32; 4]>,
        /// The transforms set (`D3DTRANSFORMSTATETYPE` -> 16 words).
        pub transforms: HashMap<u32, Vec<u32>>,
        /// The render states set since the last indexed draw (in order), and the pixel shaders the
        /// face meshes were drawn with (other meshes drawn with them are reported for comparison).
        pub states_since_draw: Vec<(u32, u32)>,
        pub face_shaders: std::collections::HashSet<u32>,
        /// The meshes drawn with the face texture, by (vertices, primitives): their other passes are
        /// reported too.
        pub face_meshes: std::collections::HashSet<(u32, u32)>,
        /// This frame's indexed draws: all, with ZWRITEENABLE on, skipped; frames summed so far.
        pub frame_draws: [u32; 3],
        pub frames: u32,
        pub projection_shown: bool,
    }

    pub(super) fn state() -> std::sync::MutexGuard<'static, State> {
        static STATE: OnceLock<Mutex<State>> = OnceLock::new();
        STATE
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether a `D3DFORMAT` is a four-character DXT code.
    pub(super) fn compressed(format: u32) -> bool {
        (u32::from_le_bytes(*b"DXT1")..=u32::from_le_bytes(*b"DXT5")).contains(&format)
            && format.to_le_bytes()[..3] == *b"DXT"
    }

    /// Whether a texture of this size and format is the face texture of the face image `image`
    /// (`width << 32 | height`, 0 for none yet).
    pub(super) fn is_face(image: u64, width: u32, height: u32, format: u32) -> bool {
        image != 0 && !compressed(format) && (u64::from(width) << 32 | u64::from(height)) == image
    }

    /// The mean of each of the four bytes of 32-bit pixels.
    pub(super) fn means(bytes: &[u8]) -> [u64; 4] {
        let mut sum = [0u64; 4];
        let mut n = 0u64;
        for pixel in bytes.chunks_exact(4) {
            for c in 0..4 {
                sum[c] += u64::from(pixel[c]);
            }
            n += 1;
        }
        sum.map(|s| s / n.max(1))
    }

    /// A 32-bit B, G, R, A image as an uncompressed TGA (rows top first).
    pub(super) fn tga(width: u32, height: u32, bgra: &[u8]) -> Vec<u8> {
        let mut file = vec![0u8; 18];
        file[2] = 2;
        file[12..14].copy_from_slice(&(width as u16).to_le_bytes());
        file[14..16].copy_from_slice(&(height as u16).to_le_bytes());
        file[16] = 32;
        file[17] = 0x28;
        file.extend_from_slice(bgra);
        file
    }
}

/// `NV_FACE_CHECK`: a texture was created (diagnostic only); the face texture is followed.
fn face_check_texture(
    texture: u32,
    levels: &[u32],
    size: (u32, u32),
    format: u32,
    usage: u32,
    pool: u32,
) {
    let mut s = face_check::state();
    s.textures
        .insert(texture, (size.0, size.1, format, levels.len() as u32));
    // One texture per face image: the first one of its size made after it.
    let matched = {
        let mut pending = super::FACE_CHECK_IMAGES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let found = pending
            .iter()
            .position(|&image| face_check::is_face(image, size.0, size.1, format));
        found.map(|at| pending.remove(at))
    };
    if matched.is_none() {
        return;
    }
    s.faces.push(texture);
    for (i, &level) in levels.iter().enumerate() {
        s.levels.insert(level, (texture, i as u32));
    }
    drop(s);
    face_check::line(format_args!(
        "d3d face texture {texture:#x} {}x{} format {format} levels {} usage {usage:#x} pool {pool}",
        size.0,
        size.1,
        levels.len()
    ));
}

/// `NV_FACE_CHECK`: a level of the face texture went to the real surface.
fn face_check_upload(
    e: &Engine,
    surface: u32,
    real_surface: &IDirect3DSurface9,
    bits: u32,
    row: u32,
    rows: u32,
) {
    let Some(&(texture, level)) = face_check::state().levels.get(&surface) else {
        return;
    };
    let (width, height, format, _) = face_check::state()
        .textures
        .get(&texture)
        .copied()
        .unwrap_or_default();
    let model = e.mem.bytes(bits, row.saturating_mul(rows));
    let mut desc = D3DSURFACE_DESC::default();
    let _ = unsafe { real_surface.GetDesc(&mut desc) };
    let mut locked = D3DLOCKED_RECT::default();
    let read =
        unsafe { real_surface.LockRect(&mut locked, std::ptr::null(), D3DLOCK_READONLY as u32) };
    let mut back = Vec::new();
    let mut pitch = 0;
    if read.is_ok() {
        pitch = locked.Pitch;
        for y in 0..rows {
            let from = unsafe {
                std::slice::from_raw_parts(
                    (locked.pBits as *const u8).add(y as usize * locked.Pitch as usize),
                    row as usize,
                )
            };
            back.extend_from_slice(from);
        }
        let _ = unsafe { real_surface.UnlockRect() };
    }
    face_check::line(format_args!(
        "d3d face upload texture {texture:#x} level {level} ({}x{} of {width}x{height}, format {format}): {rows} rows of {row} bytes, real pitch {pitch} format {}, model bytes mean {:?}, real read back {} mean {:?}, same {}",
        desc.Width,
        desc.Height,
        desc.Format.0,
        face_check::means(&model),
        if read.is_ok() { "ok" } else { "failed" },
        face_check::means(&back),
        back == model,
    ));
    let bytes_per_pixel = row.checked_div(desc.Width).unwrap_or(0);
    if (level == 0 || level == 3) && bytes_per_pixel == 4 && back.len() == model.len() {
        if let Some(dir) = std::env::var_os("NV_FACE_CHECK_DIR") {
            let n = {
                let mut s = face_check::state();
                s.dumps += 1;
                s.dumps
            };
            if n <= 40 {
                let path = std::path::Path::new(&dir).join(format!(
                    "d3d-face-{n}-{texture:x}-level{level}-{}x{}.tga",
                    desc.Width, desc.Height
                ));
                let _ = std::fs::write(&path, face_check::tga(desc.Width, desc.Height, &back));
                face_check::line(format_args!(
                    "d3d face upload written to {}",
                    path.display()
                ));
            }
        }
    }
}

/// `NV_FACE_CHECK`: a sampler took a texture.
fn face_check_bind(sampler: u32, texture: u32) {
    let mut s = face_check::state();
    let old = s.samplers.insert(sampler, texture).unwrap_or(0);
    if s.faces.is_empty() {
        return;
    }
    let is_face = s.faces.contains(&texture);
    if !is_face && s.faces.contains(&old) {
        // The face texture leaves this sampler: the next draws (the hair, the eyes) are reported.
        s.after = 16;
    }
    drop(s);
    if is_face {
        face_check::line(format_args!(
            "d3d face bind texture {texture:#x} to sampler {sampler}"
        ));
    }
}

/// `NV_FACE_CHECK`: a shader was set.
fn face_check_shader(shader: u32, pixel: bool) {
    let mut s = face_check::state();
    if pixel {
        s.pixel_shader = shader;
    } else {
        s.vertex_shader = shader;
    }
}

/// `NV_FACE_CHECK`: a frame ends: its indexed draw counts (the first 12 frames once the face texture
/// exists) and, once, the projection transform as set.
fn face_check_present() {
    let mut s = face_check::state();
    if s.faces.is_empty() {
        return;
    }
    let counts = std::mem::take(&mut s.frame_draws);
    s.frames += 1;
    let frame = s.frames;
    let projection = if s.projection_shown {
        None
    } else {
        s.projection_shown = true;
        Some(s.transforms.get(&3).cloned())
    };
    drop(s);
    if frame <= 12 {
        face_check::line(format_args!(
            "d3d face frame {frame}: {} indexed draws, {} with ZWRITEENABLE on, {} skipped",
            counts[0], counts[1], counts[2]
        ));
    }
    if let Some(projection) = projection {
        let text = match projection {
            Some(m) => m
                .iter()
                .map(|w| format!("{:.4}", f32::from_bits(*w)))
                .collect::<Vec<_>>()
                .join(","),
            None => "never set".into(),
        };
        face_check::line(format_args!("d3d face projection transform (rows): {text}"));
    }
}

/// `NV_FACE_CHECK`: an indexed draw with the face texture bound, or one of the few after it.
/// `NV_FACE_CHECK`: a render state was set.
fn face_check_render_state(state: u32, value: u32) {
    let mut s = face_check::state();
    s.render_states.insert(state, value);
    s.states_since_draw.push((state, value));
}

/// `NV_FACE_CHECK`: float shader constants were set (`pixel` for the pixel shader's).
fn face_check_constants(e: &Engine, pixel: bool, start: u32, data: u32, count: u32) {
    let mut s = face_check::state();
    for i in 0..count.min(256) {
        let at = data.wrapping_add(16 * i);
        let value = [
            e.mem.u32(at),
            e.mem.u32(at + 4),
            e.mem.u32(at + 8),
            e.mem.u32(at + 12),
        ];
        let map = if pixel {
            &mut s.pixel_constants
        } else {
            &mut s.vertex_constants
        };
        map.insert(start + i, value);
    }
}

/// The render states the face check prints, with their `D3DRENDERSTATETYPE` numbers.
const FACE_CHECK_STATES: [(&str, u32); 14] = [
    ("ZENABLE", 7),
    ("FILLMODE", 8),
    ("ZWRITEENABLE", 14),
    ("ALPHATESTENABLE", 15),
    ("SRCBLEND", 19),
    ("DESTBLEND", 20),
    ("CULLMODE", 22),
    ("ZFUNC", 23),
    ("ALPHAREF", 24),
    ("ALPHAFUNC", 25),
    ("ALPHABLENDENABLE", 27),
    ("STENCILENABLE", 52),
    ("COLORWRITEENABLE", 168),
    ("BLENDOP", 171),
];

/// The determinant of the upper-left 3x3 of three rows of four float words (a negative one
/// mirrors, flipping the triangles' winding).
fn face_check_det3(rows: [&[u32]; 3]) -> f32 {
    let m = |r: usize, c: usize| f32::from_bits(rows[r][c]);
    m(0, 0) * (m(1, 1) * m(2, 2) - m(1, 2) * m(2, 1))
        - m(0, 1) * (m(1, 0) * m(2, 2) - m(1, 2) * m(2, 0))
        + m(0, 2) * (m(1, 0) * m(2, 1) - m(1, 1) * m(2, 0))
}

/// The determinants the face check prints: view (2), projection (3), world (256) when set, and
/// of vertex constants c0..c2 (rows) when set.
fn face_check_determinants(s: &face_check::State) -> String {
    let mut parts = Vec::new();
    for (name, state) in [("view", 2u32), ("projection", 3), ("world", 256)] {
        if let Some(m) = s.transforms.get(&state) {
            parts.push(format!(
                "{name} {:.4}",
                face_check_det3([&m[0..4], &m[4..8], &m[8..12]])
            ));
        }
    }
    if let (Some(a), Some(b), Some(c)) = (
        s.vertex_constants.get(&0),
        s.vertex_constants.get(&1),
        s.vertex_constants.get(&2),
    ) {
        parts.push(format!("vertex c0..c2 {:.4}", face_check_det3([a, b, c])));
    }
    parts.join(", ")
}

/// One line of float constants: `cN=(x,y,z,w)` for each register set so far, in order.
fn face_check_constant_list(map: &HashMap<u32, [u32; 4]>, below: u32) -> String {
    let mut registers: Vec<u32> = map.keys().copied().filter(|&r| r < below).collect();
    registers.sort();
    registers
        .iter()
        .map(|r| {
            let v = map[r].map(f32::from_bits);
            format!("c{r}=({:.3},{:.3},{:.3},{:.3})", v[0], v[1], v[2], v[3])
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `NV_FACE_CHECK`: an indexed draw with the face texture bound, or one of the few after it.
#[allow(clippy::too_many_arguments)]
fn face_check_draw(
    kind: u32,
    base_vertex: u32,
    min_index: u32,
    vertices: u32,
    start_index: u32,
    count: u32,
    skipped: bool,
) {
    let mut s = face_check::state();
    let since = std::mem::take(&mut s.states_since_draw);
    if s.faces.is_empty() {
        return;
    }
    let zwrite = s.render_states.get(&14).copied().unwrap_or(1) != 0;
    s.frame_draws[0] += 1;
    s.frame_draws[1] += u32::from(zwrite);
    s.frame_draws[2] += u32::from(skipped);
    let bound = s.samplers.values().any(|t| s.faces.contains(t));
    if bound {
        s.face_meshes.insert((vertices, count));
    }
    let face_mesh = s.face_meshes.contains(&(vertices, count));
    if face_mesh {
        let shader = s.pixel_shader;
        s.face_shaders.insert(shader);
    }
    let label = if bound {
        "with the face texture"
    } else if face_mesh {
        "face mesh, another pass"
    } else if s.after > 0 {
        s.after -= 1;
        "after the face"
    } else if s.face_shaders.contains(&s.pixel_shader) {
        "another mesh, same pixel shader as a face pass"
    } else {
        return;
    };
    let mut samplers: Vec<(u32, u32)> = s.samplers.iter().map(|(&k, &v)| (k, v)).collect();
    samplers.sort();
    let list: Vec<String> = samplers
        .iter()
        .filter(|&&(_, t)| t != 0)
        .map(|&(sampler, t)| {
            let face = if s.faces.contains(&t) { " FACE" } else { "" };
            match s.textures.get(&t) {
                Some(&(w, h, f, l)) => format!("s{sampler}={t:#x} {w}x{h} f{f} l{l}{face}"),
                None => format!("s{sampler}={t:#x} ?{face}"),
            }
        })
        .collect();
    let (pixel, vertex) = (s.pixel_shader, s.vertex_shader);
    let key = format!(
        "{vertices} {count} {start_index} {pixel:#x} {vertex:#x} {}",
        list.join(", ")
    );
    let times = {
        let n = s.seen.entry(key).or_insert(0);
        *n += 1;
        *n
    };
    if times > 2 {
        return;
    }
    let states: Vec<String> = FACE_CHECK_STATES
        .iter()
        .map(|&(name, number)| match s.render_states.get(&number) {
            Some(value) => format!("{name}={value:#x}"),
            None => format!("{name}=unset"),
        })
        .collect();
    let determinants = face_check_determinants(&s);
    let pixel_constants = face_check_constant_list(&s.pixel_constants, 32);
    let vertex_constants = face_check_constant_list(&s.vertex_constants, 96);
    drop(s);
    // The file names the textures were made from (`00e68ef0`).
    let names: Vec<String> = {
        let known = super::FACE_CHECK_NAMES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        samplers
            .iter()
            .filter(|&&(_, t)| t != 0)
            .map(|&(sampler, t)| {
                format!("s{sampler} {}", known.get(&t).map_or("?", String::as_str))
            })
            .collect()
    };
    face_check::line(format_args!(
        "d3d face draw ({label}) {vertices} vertices {count} triangles, primitive {kind} base vertex {base_vertex} min index {min_index} start index {start_index}{}, pixel shader {pixel:#x} vertex shader {vertex:#x}: {} | {}",
        if skipped { " SKIPPED on the real device" } else { "" },
        list.join(", "),
        names.join("; ")
    ));
    face_check::line(format_args!(
        "d3d face states: {} | determinants: {determinants}",
        states.join(" ")
    ));
    let since: Vec<String> = since
        .iter()
        .map(|(state, value)| format!("{state}={value:#x}"))
        .collect();
    face_check::line(format_args!(
        "d3d face states set since the previous draw: {}",
        since.join(" ")
    ));
    face_check::line(format_args!("d3d face pixel constants: {pixel_constants}"));
    face_check::line(format_args!(
        "d3d face vertex constants: {vertex_constants}"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real device keeps the game's interval unless `NV_D3D9_PRESENT=immediate`.
    #[test]
    fn the_presentation_interval_is_the_games_unless_immediate_is_asked() {
        assert_eq!(choose_interval(1, false), 1);
        assert_eq!(choose_interval(0, false), 0);
        assert_eq!(choose_interval(1, true), INTERVAL_IMMEDIATE);
    }

    #[test]
    fn shot_settings_and_bmp() {
        let s = shot::parse("C:\\shots,2500").unwrap();
        assert_eq!(s.dir, std::path::PathBuf::from("C:\\shots"));
        assert_eq!(s.every, std::time::Duration::from_millis(2500));
        let s = shot::parse("C:\\shots").unwrap();
        assert_eq!(s.every, std::time::Duration::from_millis(5000));
        let s = shot::parse("D:\\a,b").unwrap();
        assert_eq!(s.dir, std::path::PathBuf::from("D:\\a,b"));
        assert!(shot::parse(" ").is_none());
        let file = shot::bmp(2, 1, &[1, 2, 3, 0, 4, 5, 6, 0]);
        assert_eq!(&file[0..2], b"BM");
        assert_eq!(file.len(), 54 + 8);
        assert_eq!(i32::from_le_bytes(file[22..26].try_into().unwrap()), -1);
        assert_eq!(u16::from_le_bytes(file[28..30].try_into().unwrap()), 32);
        assert_eq!(&file[54..], &[1, 2, 3, 0, 4, 5, 6, 0]);
    }

    #[test]
    fn face_check_helpers() {
        assert!(face_check::compressed(u32::from_le_bytes(*b"DXT1")));
        assert!(face_check::compressed(u32::from_le_bytes(*b"DXT5")));
        assert!(!face_check::compressed(21));
        // Only an uncompressed texture of the face image's size, once an image was made.
        let image = (256u64 << 32) | 256;
        assert!(face_check::is_face(image, 256, 256, 21));
        assert!(!face_check::is_face(0, 256, 256, 21));
        assert!(!face_check::is_face(image, 512, 512, 21));
        assert!(!face_check::is_face(
            image,
            256,
            256,
            u32::from_le_bytes(*b"DXT5")
        ));
        assert_eq!(face_check::means(&[1, 2, 3, 4, 3, 4, 5, 6]), [2, 3, 4, 5]);
        let file = face_check::tga(1, 1, &[9, 8, 7, 6]);
        assert_eq!(file.len(), 22);
        assert_eq!((file[2], file[12], file[14], file[16]), (2, 1, 1, 32));
        assert_eq!(&file[18..], &[9, 8, 7, 6]);
        let mut map = HashMap::new();
        map.insert(3u32, [1f32.to_bits(), 0, 0, 0]);
        map.insert(1u32, [0, 2f32.to_bits(), 0, 0]);
        map.insert(40u32, [0; 4]);
        let one = 1f32.to_bits();
        let minus = (-1f32).to_bits();
        assert_eq!(
            face_check_det3([&[one, 0, 0], &[0, one, 0], &[0, 0, one]]),
            1.0
        );
        assert_eq!(
            face_check_det3([&[minus, 0, 0], &[0, one, 0], &[0, 0, one]]),
            -1.0
        );
        assert_eq!(
            face_check_constant_list(&map, 32),
            "c1=(0.000,2.000,0.000,0.000) c3=(1.000,0.000,0.000,0.000)"
        );
    }

    /// A key message's `lParam` with this scan code (extended or not).
    fn key(scan: isize, extended: bool) -> isize {
        (scan << 16) | if extended { 1 << 24 } else { 0 }
    }

    #[test]
    fn key_messages_give_directinput_key_codes() {
        // W, Escape, left Ctrl and Alt: the scan code.
        assert_eq!(dik(key(0x11, false)), Some(0x11));
        assert_eq!(dik(key(0x01, false)), Some(0x01));
        assert_eq!(dik(key(0x1d, false)), Some(0x1d));
        assert_eq!(dik(key(0x38, false)), Some(0x38));
        // Right Ctrl and Alt, the arrows, numpad Enter: extended, 0x80 added.
        assert_eq!(dik(key(0x1d, true)), Some(0x9d));
        assert_eq!(dik(key(0x38, true)), Some(0xb8));
        assert_eq!(dik(key(0x48, true)), Some(0xc8));
        assert_eq!(dik(key(0x50, true)), Some(0xd0));
        assert_eq!(dik(key(0x1c, true)), Some(0x9c));
        // The numpad's own keys are not extended (DIK_NUMPAD8 0x48).
        assert_eq!(dik(key(0x48, false)), Some(0x48));
        // NumLock (extended 0x45) and Pause (plain 0x45).
        assert_eq!(dik(key(0x45, true)), Some(0x45));
        assert_eq!(dik(key(0x45, false)), Some(0xc5));
        // The fake shifts around extended keys are dropped.
        assert_eq!(dik(key(0x2a, true)), None);
        assert_eq!(dik(key(0, false)), None);
    }

    /// p169's crash: on a pure device (the game's behaviour flags 0x54) the runtime keeps no
    /// reference of its own to a bound vertex buffer. The model lets go of a buffer when the
    /// program rebinds the stream, before the same call reaches the real device; the real buffer
    /// must outlive that call (it is released at the next `Present`), or the real
    /// `SetStreamSource` reads freed memory. Skipped where there is no Direct3D 9 device.
    #[test]
    fn a_buffer_the_model_lets_go_of_stays_alive_until_present() {
        let (reply, answer) = std::sync::mpsc::channel();
        let sent = window_thread()
            .lock()
            .map(|tx| {
                tx.send(Request {
                    title: b"nv-rs test".to_vec(),
                    style: 0,
                    ex_style: 0,
                    rect: [0, 0, 64, 64],
                    parent: 0,
                    reply,
                })
                .is_ok()
            })
            .unwrap_or(false);
        let Some(window) = answer.recv().ok().filter(|&w| sent && w != 0) else {
            return;
        };
        let Some(d3d) = (unsafe { Direct3DCreate9(32) }) else {
            return;
        };
        let mut pp = D3DPRESENT_PARAMETERS {
            BackBufferWidth: 64,
            BackBufferHeight: 64,
            BackBufferCount: 1,
            SwapEffect: D3DSWAPEFFECT_DISCARD,
            hDeviceWindow: HWND(window as *mut _),
            Windowed: BOOL(1),
            ..Default::default()
        };
        let mut out = None;
        // Hardware vertex processing, pure, multithreaded: the game's 0x54.
        let made = unsafe {
            d3d.CreateDevice(
                0,
                D3DDEVTYPE_HAL,
                HWND(window as *mut _),
                0x54 | FPU_PRESERVE,
                &mut pp,
                &mut out,
            )
        };
        let Some(real_device) = out.filter(|_| made.is_ok()) else {
            return;
        };
        let mut e = Engine::new();
        let device = 0x1000;
        real(&mut e)
            .objects
            .insert(device, Object::Device(real_device));
        let (first, second) = (0x2000, 0x2008);
        // D3DUSAGE_WRITEONLY, default pool, 96 bytes.
        buffer_created(&mut e, device, first, false, 96, 8, 0, 0);
        set_stream_source(&mut e, device, 0, first, 0, 24, 0);
        // The model rebinds stream 0: the first buffer goes before the real call.
        buffer_created(&mut e, device, second, false, 96, 8, 0, 0);
        forget(&mut e, first);
        assert_eq!(e.kernel.graphics.real.graveyard.len(), 1);
        set_stream_source(&mut e, device, 0, second, 0, 24, 0);
        set_stream_source(&mut e, device, 0, 0, 0, 0, 0);
        present(&mut e, device, 0, 0, 0, 0, 0);
        assert!(e.kernel.graphics.real.graveyard.is_empty());
    }

    /// A real device on a hidden window of the window thread, as an engine object at 0x1000
    /// (`behavior`: the creation flags); `None` where Direct3D 9 has no device.
    fn test_device(e: &mut Engine, behavior: u32) -> Option<u32> {
        let (reply, answer) = std::sync::mpsc::channel();
        let sent = window_thread()
            .lock()
            .map(|tx| {
                tx.send(Request {
                    title: b"nv-rs test".to_vec(),
                    style: 0,
                    ex_style: 0,
                    rect: [0, 0, 64, 64],
                    parent: 0,
                    reply,
                })
                .is_ok()
            })
            .unwrap_or(false);
        let window = answer.recv().ok().filter(|&w| sent && w != 0)?;
        let d3d = unsafe { Direct3DCreate9(32) }?;
        let mut pp = D3DPRESENT_PARAMETERS {
            BackBufferWidth: 64,
            BackBufferHeight: 64,
            BackBufferCount: 1,
            SwapEffect: D3DSWAPEFFECT_DISCARD,
            hDeviceWindow: HWND(window as *mut _),
            Windowed: BOOL(1),
            ..Default::default()
        };
        let mut out = None;
        let made = unsafe {
            d3d.CreateDevice(
                0,
                D3DDEVTYPE_HAL,
                HWND(window as *mut _),
                behavior | FPU_PRESERVE,
                &mut pp,
                &mut out,
            )
        };
        let device = out.filter(|_| made.is_ok())?;
        real(e).objects.insert(0x1000, Object::Device(device));
        Some(0x1000)
    }

    /// `SetLOD` on a managed texture reaches the real texture, and the real one answers the
    /// previous level as the model does.
    #[test]
    fn set_lod_reaches_the_real_texture() {
        let mut e = Engine::new();
        let Some(device) = test_device(&mut e, 0x54) else {
            return;
        };
        let (texture, levels) = (0x3000, [0x3008, 0x3010, 0x3018]);
        // 64x64, three levels, A8R8G8B8, managed.
        texture_created(&mut e, device, texture, &levels, 64, 64, 0, 21, 1);
        texture_set_lod(&mut e, texture, 2, 0);
        texture_set_lod(&mut e, texture, 1, 2);
        let t = base_texture(&e, texture).expect("the real texture");
        assert_eq!(unsafe { t.GetLOD() }, 1);
        assert!(
            e.kernel.graphics.real.logged.is_empty(),
            "no answer differed: {:?}",
            e.kernel.graphics.real.logged
        );
    }
}
