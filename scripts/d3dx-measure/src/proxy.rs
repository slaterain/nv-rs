//! A proxy for the Direct3D 9 device `serve proxy` makes, to capture what D3DX does on devices that
//! take other formats than this machine's: D3DX asks the device's `IDirect3D9` (`GetDirect3D`)
//! which formats it takes, and the proxy's answers for textures and cube textures come from the
//! lists `set_lists` gives (other resource types: the real answer). A texture or cube texture of a
//! format the real device lacks is made in `D3DPOOL_SCRATCH`, which takes any format, so D3DX fills
//! and filters it as on a device that takes it. Every other method goes to the real object: its
//! vtable slots are thunks made here that put the real object in place of the proxy and jump to
//! the real method.
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[link(name = "kernel32")]
extern "system" {
    fn VirtualAlloc(address: *mut c_void, size: usize, kind: u32, protect: u32) -> *mut c_void;
}

/// A proxy object: its vtable, then the real object (the thunks read it at offset 4).
#[repr(C)]
struct Proxy {
    vtable: *const usize,
    real: *mut c_void,
}

/// The formats the proxy takes: textures, cube textures.
static LISTS: Mutex<(Vec<u32>, Vec<u32>)> = Mutex::new((Vec::new(), Vec::new()));
static REAL_D3D: AtomicUsize = AtomicUsize::new(0);
static PROXY_D3D: AtomicUsize = AtomicUsize::new(0);
static DISPLAY_FORMAT: AtomicUsize = AtomicUsize::new(0);

/// The formats the proxy device takes from now on, as a texture and as a cube texture.
pub fn set_lists(texture: Vec<u32>, cube: Vec<u32>) {
    *LISTS.lock().unwrap() = (texture, cube);
}

unsafe fn vt(object: *mut c_void, slot: usize) -> usize {
    *(*(object as *mut *const usize)).add(slot)
}

/// A vtable of `slots` forwarding thunks (`mov eax,[esp+4]; mov eax,[eax+4]; mov [esp+4],eax;
/// mov eax,[eax]; jmp [eax+slot*4]`), with `overrides` in their slots.
unsafe fn vtable(slots: usize, overrides: &[(usize, usize)]) -> *const usize {
    const MEM_COMMIT_RESERVE: u32 = 0x3000;
    const PAGE_EXECUTE_READWRITE: u32 = 0x40;
    let code = VirtualAlloc(std::ptr::null_mut(), slots * 32, MEM_COMMIT_RESERVE, PAGE_EXECUTE_READWRITE) as *mut u8;
    assert!(!code.is_null(), "VirtualAlloc");
    let table: &'static mut [usize] = Box::leak(vec![0usize; slots].into_boxed_slice());
    for (slot, entry) in table.iter_mut().enumerate() {
        let d = ((slot * 4) as u32).to_le_bytes();
        let thunk: [u8; 19] = [
            0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x04, 0x89, 0x44, 0x24, 0x04, 0x8b, 0x00, 0xff, 0xa0, d[0], d[1], d[2], d[3],
        ];
        let at = code.add(slot * 32);
        std::ptr::copy_nonoverlapping(thunk.as_ptr(), at, thunk.len());
        *entry = at as usize;
    }
    for &(slot, f) in overrides {
        table[slot] = f;
    }
    table.as_ptr()
}

type AddRef = unsafe extern "system" fn(*mut c_void) -> u32;
type CheckFormat = unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32, u32, u32) -> u32;
type CreateTexture =
    unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32, u32, u32, *mut *mut c_void, *mut c_void) -> u32;
type CreateCube = unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32, u32, *mut *mut c_void, *mut c_void) -> u32;

/// `IDirect3DDevice9::GetDirect3D`: the proxy `IDirect3D9`, with a reference on the real one.
unsafe extern "system" fn get_direct3d(_this: *mut Proxy, out: *mut *mut c_void) -> u32 {
    let real = REAL_D3D.load(Ordering::Relaxed) as *mut c_void;
    (std::mem::transmute::<usize, AddRef>(vt(real, 1)))(real);
    *out = PROXY_D3D.load(Ordering::Relaxed) as *mut c_void;
    0
}

/// `IDirect3D9::CheckDeviceFormat`: textures and cube textures from the lists.
unsafe extern "system" fn check_device_format(
    this: *mut Proxy,
    adapter: u32,
    device_type: u32,
    adapter_format: u32,
    usage: u32,
    rtype: u32,
    format: u32,
) -> u32 {
    let lists = LISTS.lock().unwrap();
    let list = match rtype {
        3 => &lists.0,
        5 => &lists.1,
        _ => {
            let real = (*this).real;
            return (std::mem::transmute::<usize, CheckFormat>(vt(real, 10)))(
                real,
                adapter,
                device_type,
                adapter_format,
                usage,
                rtype,
                format,
            );
        }
    };
    if list.contains(&format) {
        0
    } else {
        0x8876_086a
    }
}

/// Whether the real device takes `format` for `rtype` with `usage`.
unsafe fn real_takes(usage: u32, rtype: u32, format: u32) -> bool {
    let real = REAL_D3D.load(Ordering::Relaxed) as *mut c_void;
    let display = DISPLAY_FORMAT.load(Ordering::Relaxed) as u32;
    (std::mem::transmute::<usize, CheckFormat>(vt(real, 10)))(real, 0, 1, display, usage, rtype, format) == 0
}

/// `IDirect3DDevice9::CreateTexture`: in `D3DPOOL_SCRATCH` when the real device lacks the format.
unsafe extern "system" fn create_texture(
    this: *mut Proxy,
    w: u32,
    h: u32,
    levels: u32,
    usage: u32,
    format: u32,
    pool: u32,
    out: *mut *mut c_void,
    shared: *mut c_void,
) -> u32 {
    let real = (*this).real;
    let pool = if real_takes(usage, 3, format) { pool } else { 3 };
    (std::mem::transmute::<usize, CreateTexture>(vt(real, 23)))(real, w, h, levels, usage, format, pool, out, shared)
}

/// `IDirect3DDevice9::CreateCubeTexture`: the same.
unsafe extern "system" fn create_cube_texture(
    this: *mut Proxy,
    edge: u32,
    levels: u32,
    usage: u32,
    format: u32,
    pool: u32,
    out: *mut *mut c_void,
    shared: *mut c_void,
) -> u32 {
    let real = (*this).real;
    let pool = if real_takes(usage, 5, format) { pool } else { 3 };
    (std::mem::transmute::<usize, CreateCube>(vt(real, 25)))(real, edge, levels, usage, format, pool, out, shared)
}

/// The proxy for the real `device` (its `IDirect3D9` and the display mode's format read now).
pub unsafe fn proxy(device: *mut c_void) -> *mut c_void {
    type GetDirect3D = unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> u32;
    type GetDisplayMode = unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> u32;
    let mut d3d: *mut c_void = std::ptr::null_mut();
    assert_eq!((std::mem::transmute::<usize, GetDirect3D>(vt(device, 6)))(device, &mut d3d), 0);
    let mut mode = [0u32; 4];
    assert_eq!((std::mem::transmute::<usize, GetDisplayMode>(vt(device, 8)))(device, 0, mode.as_mut_ptr()), 0);
    REAL_D3D.store(d3d as usize, Ordering::Relaxed);
    DISPLAY_FORMAT.store(mode[3] as usize, Ordering::Relaxed);
    // IDirect3D9 has 17 methods, IDirect3DDevice9 119.
    let d3d_vtable = vtable(17, &[(10, check_device_format as *const () as usize)]);
    let proxy_d3d = Box::leak(Box::new(Proxy { vtable: d3d_vtable, real: d3d }));
    PROXY_D3D.store(proxy_d3d as *mut Proxy as usize, Ordering::Relaxed);
    let device_vtable = vtable(
        119,
        &[
            (6, get_direct3d as *const () as usize),
            (23, create_texture as *const () as usize),
            (25, create_cube_texture as *const () as usize),
        ],
    );
    Box::leak(Box::new(Proxy { vtable: device_vtable, real: device })) as *mut Proxy as *mut c_void
}
