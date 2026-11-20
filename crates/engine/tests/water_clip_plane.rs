//! The water's clip plane through the Direct3D 9 device model (play report 011): the game's helper
//! at `004ed190` sets the clip planes of the water's reflection and depth passes with the device's
//! `SetClipPlane`, a stdcall method of three words (device, index, plane). The translation passed a
//! fourth word before them (what `ECX` held at the call, the device's vtable pointer), so the
//! modelled device took the vtable for the device and the device for the index and answered
//! `D3DERR_INVALIDCALL` to every one: neither pass was ever cut at the water. Here the helper runs
//! against the real model, and the model's own answer is read.

use engine::{args, Engine, Ret};
use engine_platform::d3d9;
use std::cell::RefCell;
use std::rc::Rc;

/// The game's helper: `thiscall(render object, first index, count, planes)`.
const SET_CLIP_PLANES: u32 = 0x004e_d190;
/// The render object's device.
const RENDER_OBJECT_DEVICE: u32 = 0x288;
/// `IDirect3D9::CreateDevice` and `IDirect3DDevice9::SetClipPlane` in their vtables.
const CREATE_DEVICE: u32 = 0x40;
const SET_CLIP_PLANE: u32 = 0xdc;
/// Where the test listens: the address the device's `SetClipPlane` slot is pointed at.
const LISTENER: u32 = 0x0300_0411;
const D3D_OK: u32 = 0;
const D3DERR_INVALIDCALL: u32 = 0x8876_086c;

/// An adapter table for one windowed HAL device: a 640x480 X8R8G8B8 mode, and caps with four
/// render targets and six user clip planes (`D3DCAPS9::MaxUserClipPlanes`, +0xa4).
fn table() -> String {
    let hex = |b: &[u8]| -> String { b.iter().map(|x| format!("{x:02x}")).collect() };
    let mut mode = vec![0u8; 16];
    mode[..4].copy_from_slice(&640u32.to_le_bytes());
    mode[4..8].copy_from_slice(&480u32.to_le_bytes());
    mode[8..12].copy_from_slice(&60u32.to_le_bytes());
    mode[12..].copy_from_slice(&0x16u32.to_le_bytes());
    let mut caps = vec![0u8; 304];
    caps[..4].copy_from_slice(&1u32.to_le_bytes());
    caps[0xa4..0xa8].copy_from_slice(&6u32.to_le_bytes());
    caps[0xf0..0xf4].copy_from_slice(&4u32.to_le_bytes());
    [
        format!("GetAdapterDisplayMode\t0\t0\t{}", hex(&mode)),
        format!("GetDeviceCaps\t0 1\t0\t{}", hex(&caps)),
        "Domain\td0\t0,1".to_string(),
        "CheckDeviceType\t0 1 16 16\t0\t\td0".to_string(),
    ]
    .join("\n")
}

/// What the device's `SetClipPlane` was called with, and what the model answered.
type Calls = Rc<RefCell<Vec<(Vec<u32>, u32)>>>;

/// An engine with a modelled device, a render object holding it, and the device's `SetClipPlane`
/// slot listened to (each call goes on to the model's own method; its words and answer are kept).
fn world() -> (Engine, u32, u32, Calls) {
    let mut e = Engine::new();
    d3d9::load_adapter_table(&mut e, &table());
    let create = d3d9::proc_address(&mut e, b"Direct3DCreate9").expect("Direct3DCreate9");
    let d3d = e.call(create, &[d3d9::D3D_SDK_VERSION]).u32();
    // D3DPRESENT_PARAMETERS: 640x480, windowed, on the window 0x11c.
    let present = e.mem.alloc(0x38);
    e.mem.write(present, &[0; 0x38]);
    e.mem.set_u32(present, 640);
    e.mem.set_u32(present + 4, 480);
    e.mem.set_u32(present + 0x1c, 0x11c);
    e.mem.set_u32(present + 0x20, 1);
    let out = e.mem.alloc(4);
    let hr = e
        .vcall(d3d, CREATE_DEVICE, &[0, 1, 0x11c, 0x40, present, out])
        .u32();
    assert_eq!(hr, D3D_OK, "CreateDevice");
    let device = e.mem.u32(out);
    let vtable = e.mem.u32(device);
    let model = e.mem.u32(vtable + SET_CLIP_PLANE);
    let calls: Calls = Rc::default();
    let log = calls.clone();
    e.register_double(LISTENER, move |e, a| {
        let answer = e.call(model, a).u32();
        log.borrow_mut().push((a.to_vec(), answer));
        Ret {
            eax: answer,
            ..Ret::default()
        }
    });
    e.mem.set_u32(vtable + SET_CLIP_PLANE, LISTENER);
    let object = e.mem.alloc(0x300);
    e.mem.write(object, &[0; 0x300]);
    e.mem.set_u32(object + RENDER_OBJECT_DEVICE, device);
    (e, object, device, calls)
}

fn planes(e: &mut Engine, floats: &[f32]) -> u32 {
    let at = e.mem.alloc(4 * floats.len() as u32);
    for (i, f) in floats.iter().enumerate() {
        e.mem.set_f32(at + 4 * i as u32, *f);
    }
    at
}

/// The water's clip plane is taken by the device: three words (the device, the index, the
/// plane), and the model answers `D3D_OK`.
#[test]
fn the_waters_clip_plane_is_taken_by_the_device() {
    let (mut e, object, device, calls) = world();
    // The depth pass's plane at the Quarry Junction pond (capture c1 of the original).
    let plane = planes(
        &mut e,
        &[-0.000_022_359_68, -0.555_766_3, 50.3208, -50.062_622],
    );
    e.call(SET_CLIP_PLANES, &args![object, 0i32, 1i32, plane]);
    assert_eq!(
        *calls.borrow(),
        vec![(vec![device, 0, plane], D3D_OK)],
        "SetClipPlane(device, index, plane) answered D3D_OK"
    );
}

/// Each plane of the count goes to its own index from the first, 16 bytes apart; none for a count
/// of 0.
#[test]
fn each_plane_goes_to_its_index() {
    let (mut e, object, device, calls) = world();
    let all = planes(&mut e, &(0..12).map(|i| i as f32).collect::<Vec<_>>());
    e.call(SET_CLIP_PLANES, &args![object, 2i32, 3i32, all]);
    assert_eq!(
        *calls.borrow(),
        vec![
            (vec![device, 2, all], D3D_OK),
            (vec![device, 3, all + 16], D3D_OK),
            (vec![device, 4, all + 32], D3D_OK),
        ]
    );
    calls.borrow_mut().clear();
    e.call(SET_CLIP_PLANES, &args![object, 0i32, 0i32, all]);
    assert!(calls.borrow().is_empty());
}

/// The model's own rule, which the old call ran into: an index at or past the caps' count (the
/// device's address stood there) is `D3DERR_INVALIDCALL`.
#[test]
fn the_model_refuses_an_index_past_the_caps() {
    let (mut e, _, device, _) = world();
    let plane = planes(&mut e, &[0.0, 0.0, 1.0, -5.0]);
    assert_eq!(
        e.vcall(device, SET_CLIP_PLANE, &args![6u32, plane]).u32(),
        D3DERR_INVALIDCALL
    );
    assert_eq!(
        e.vcall(device, SET_CLIP_PLANE, &args![5u32, plane]).u32(),
        D3D_OK
    );
}
