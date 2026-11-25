//! The interface family: the interface unit typed (the translation is engine-fallout-interface's `interface_p3.rs`, which
//! these functions replace at the same addresses). Each function does what its translation does, read for read
//! and write for write; the functions of the family call one another through `typed_call!`, the rest by address.

#[allow(unused_imports)]
use super::interface_cpp::*;
use engine_core::prelude::*;
use engine_core::typed_call;

/// `004b7210`: the interface manager object.
const MANAGER: u32 = 0x004b_7210;
/// `004b71d0`: whether the interface manager is ready (a `bool`).
const MANAGER_IS_READY: u32 = 0x004b_71d0;
/// `HUDMainMenu` functions that the static wrappers forward to: one word, a
/// byte, a byte, nothing.
const HUD_FORWARD_WORD: u32 = 0x0077_f2f0;
/// `HUDMainMenu::SetGunScopeVisible` (Xbox PDB).
const HUD_SET_GUN_SCOPE_VISIBLE: u32 = 0x0077_f3c0;
const HUD_FORWARD_FLAG: u32 = 0x0077_f420;
const HUD_FORWARD_SECOND_FLAG: u32 = 0x0077_f490;
const HUD_FORWARD_CALL: u32 = 0x0077_f270;
/// `MapMenu::SetRootMapSpace` (Xbox PDB).
const MAP_MENU_SET_ROOT_MAP_SPACE: u32 = 0x007a_1880;

/// A pointer global to an object whose byte at `+0x1fc` [`fn_00709cc0`]
/// reads (the callee `0077f270` clears the same byte, in the HUD main
/// menu).
const HUD_MAIN_MENU: u32 = 0x011d_96c0;
/// A byte global returned by [`fn_00709d10`].
const FLAG_BYTE: u32 = 0x011d_b0ce;

/// Manager fields (PC only, not in the Xbox PDB).
const MANAGER_FLOAT_A: u32 = 0x4ac;
const MANAGER_FLOAT_B: u32 = 0x4b0;
const MANAGER_BYTE: u32 = 0x4b4;
const MANAGER_WORD: u32 = 0x4b8;
/// The manager's mode word: 0, 2 and 3 are tested by the getters below.
const MANAGER_MODE: u32 = 0x4bc;

/// Two `double` factors and the upper limit (`double`) of the half angle
/// computed by [`fn_00709d50`], and the far plane (`float`).
const ANGLE_FACTOR_A: u32 = 0x0102_3128;
const ANGLE_FACTOR_B: u32 = 0x0102_90f8;
const ANGLE_LIMIT: u32 = 0x0106_ede0;
const FAR_PLANE: u32 = 0x0103_0020;

/// `00709ac0`: returns a `float` in ST0 (a scale used by the camera setup).
const CAMERA_SCALE: u32 = 0x0070_9ac0;
/// `004de070`: a renderer flag byte.
const RENDERER_FLAG: u32 = 0x004d_e070;
/// `004de0c0` / `004de0d0`: two renderer `float`s in ST0 (their quotient is
/// the aspect ratio).
const RENDERER_WIDTH: u32 = 0x004d_e0c0;
const RENDERER_HEIGHT: u32 = 0x004d_e0d0;
/// `00403e20` on the object at `01203150`: returns a pointer to a `float`.
const FLOAT_SETTING_OBJECT: u32 = 0x0120_3150;
const FLOAT_SETTING_GET: u32 = 0x0040_3e20;
/// `NiCamera::SetViewFrustum` (Xbox PDB), taking a `const NiFrustum*`.
const NI_CAMERA_SET_VIEW_FRUSTUM: u32 = 0x00a6_faf0;
/// `NiFrustum::NiFrustum(bool ortho)` (Xbox PDB).
const NI_FRUSTUM_NEW: u32 = 0x00a7_1b70;
/// `00a6fb90` (the camera's), thiscall on the camera with a `float` and a
/// zero word.
const CAMERA_SET_UNIT: u32 = 0x00a6_fb90;
/// `0057dd70`: one `float` in, a `float` in ST0 out (the frustum's tangent).
const TANGENT: u32 = 0x0057_dd70;

/// The vtable of the `BSSimpleArray<Tile::Value *>` of `00709e80` (RTTI
/// `.?AV?$BSSimpleArray@PAUValue@Tile@@$07@@`).
const SIMPLE_ARRAY_VTABLE: u32 = 0x0106_edec;
/// The base initialization the constructor calls, the storage release the
/// destructor calls, and the cdecl deallocation of the deleting destructor.
const ARRAY_INIT: u32 = 0x006b_3eb0;
const ARRAY_RELEASE: u32 = 0x0084_54f0;
const FREE: u32 = 0x0040_1030;

/// The manager object.
fn manager(e: &mut Engine) -> u32 {
    e.call(MANAGER, &[]).u32()
}

// Typed from 00709b50
/// Static wrapper (cdecl): [`fn_00709b80`] on the interface manager.
pub fn fn_00709b50(e: &mut Engine, flag: u8, first: f32, second: f32, word: u32) {
    let this = manager(e);
    typed_call!(
        super::FAMILY,
        0x0070_9b80,
        fn_00709b80(e, this, flag, first, second, word)
    );
}

// Typed from 00709b80
/// Stores the four arguments in the manager's PC-only fields `+0x4b4`
/// (byte), `+0x4ac`, `+0x4b0` (floats) and `+0x4b8` (word).
pub fn fn_00709b80(e: &mut Engine, this: u32, flag: u8, first: f32, second: f32, word: u32) {
    e.mem.set_u8(this + MANAGER_BYTE, flag);
    e.mem.set_f32(this + MANAGER_FLOAT_A, first);
    e.mem.set_f32(this + MANAGER_FLOAT_B, second);
    e.mem.set_u32(this + MANAGER_WORD, word);
}

// Typed from 00709bc0
/// `Interface::IsPipboyOpening` (Xbox PDB): the manager's mode word is 2.
pub fn interface_is_pipboy_opening(e: &mut Engine) -> bool {
    let this = manager(e);
    e.mem.u32(this + MANAGER_MODE) == 2
}

// Typed from 00709be0
/// The manager's mode word is 0.
pub fn fn_00709be0(e: &mut Engine) -> bool {
    let this = manager(e);
    e.mem.u32(this + MANAGER_MODE) == 0
}

// Typed from 00709c00
/// The manager's mode word is 3.
pub fn fn_00709c00(e: &mut Engine) -> bool {
    let this = manager(e);
    e.mem.u32(this + MANAGER_MODE) == 3
}

// Typed from 00709c20
/// Static wrapper (cdecl): forwards its argument to `0077f2f0`.
pub fn fn_00709c20(e: &mut Engine, argument: u32) {
    e.call(HUD_FORWARD_WORD, &args![argument]);
}

// Typed from 00709c40
/// `Interface::SetGunScopeVisible` (Xbox PDB): forwards the flag (as a
/// zero-extended word) to `HUDMainMenu::SetGunScopeVisible`.
pub fn interface_set_gun_scope_visible(e: &mut Engine, visible: u8) {
    e.call(HUD_SET_GUN_SCOPE_VISIBLE, &args![visible]);
}

// Typed from 00709c60
/// Static wrapper (cdecl): forwards its byte argument to `0077f420`.
pub fn fn_00709c60(e: &mut Engine, flag: u8) {
    e.call(HUD_FORWARD_FLAG, &args![flag]);
}

// Typed from 00709c80
/// Static wrapper (cdecl): forwards its byte argument to `0077f490`.
pub fn fn_00709c80(e: &mut Engine, flag: u8) {
    e.call(HUD_FORWARD_SECOND_FLAG, &args![flag]);
}

// Typed from 00709ca0
/// Static wrapper: calls `0077f270`.
pub fn fn_00709ca0(e: &mut Engine) {
    e.call(HUD_FORWARD_CALL, &[]);
}

// Typed from 00709cb0
/// Static wrapper: [`fn_00709cc0`].
pub fn fn_00709cb0(e: &mut Engine) -> bool {
    typed_call!(super::FAMILY, 0x0070_9cc0, fn_00709cc0(e))
}

// Typed from 00709cc0
/// The object at `011d96c0` exists and its byte at `+0x1fc` is set.
pub fn fn_00709cc0(e: &mut Engine) -> bool {
    let menu = e.global::<u32>(HUD_MAIN_MENU);
    menu != 0 && e.mem.u8(menu + 0x1fc) != 0
}

// Typed from 00709d00
/// Static wrapper: [`fn_00709d10`].
pub fn fn_00709d00(e: &mut Engine) -> u8 {
    typed_call!(super::FAMILY, 0x0070_9d10, fn_00709d10(e))
}

// Typed from 00709d10
/// The byte global at `011db0ce`.
pub fn fn_00709d10(e: &mut Engine) -> u8 {
    e.global::<u8>(FLAG_BYTE)
}

// Typed from 00709d20
/// `Interface::SetRootMapSpace` (Xbox PDB): forwards its argument to
/// `MapMenu::SetRootMapSpace`.
pub fn interface_set_root_map_space(e: &mut Engine, space: u32) {
    e.call(MAP_MENU_SET_ROOT_MAP_SPACE, &args![space]);
}

// Typed from 00709d40
/// Static wrapper: the result of `004b71d0` (whether the manager is ready).
pub fn fn_00709d40(e: &mut Engine) -> bool {
    e.call(MANAGER_IS_READY, &[]).bool()
}

// Typed from 00709d50
/// Sets up a camera's view frustum. Does nothing when `camera` is null or
/// the renderer flag byte (`004de070`) is zero.
///
/// With `s = 00709ac0()`, `00a6fb90(camera, s, 0)` runs first. The aspect
/// ratio is `004de0c0() / 004de0d0()`. The half angle is `scale * (the
/// float at 00403e20 on 01203150) * k1 * k2` (`k1`, `k2` the doubles at
/// `01023128` and `010290f8`), replaced by the double at `0106ede0` unless
/// it is smaller (so also when it is NaN). With `t = 0057dd70(angle)` the
/// frustum is `left = -t*s*aspect`, `right = t*s*aspect`, `top = t*s`,
/// `bottom = -t*s`, `near = 1.0`, `far` = the float at `01030020`,
/// perspective; `NiCamera::SetViewFrustum` receives it.
pub fn fn_00709d50(e: &mut Engine, camera: u32, scale: f32) {
    if camera == 0 {
        return;
    }
    if e.call(RENDERER_FLAG, &[]).u8() == 0 {
        return;
    }
    let unit = e.call(CAMERA_SCALE, &[]).f32();
    e.call(CAMERA_SET_UNIT, &args![camera, unit, 0u32]);
    let aspect =
        super::narrow(e.call(RENDERER_WIDTH, &[]).f64() / e.call(RENDERER_HEIGHT, &[]).f64());
    let factor_a = e.global::<f64>(ANGLE_FACTOR_A);
    let factor_b = e.global::<f64>(ANGLE_FACTOR_B);
    let limit = e.global::<f64>(ANGLE_LIMIT);
    let setting = e
        .call(FLOAT_SETTING_GET, &args![FLOAT_SETTING_OBJECT])
        .u32();
    let mut angle = scale as f64 * e.mem.f32(setting) as f64 * factor_a * factor_b;
    if angle < limit {
        let setting = e
            .call(FLOAT_SETTING_GET, &args![FLOAT_SETTING_OBJECT])
            .u32();
        angle = scale as f64 * e.mem.f32(setting) as f64 * factor_a * factor_b;
    } else {
        angle = limit;
    }
    let angle = super::narrow(angle);
    let (unit, aspect) = (unit as f64, aspect as f64);
    e.with_stack(0x1c, |e, frustum| {
        let f = frustum.addr();
        e.call(NI_FRUSTUM_NEW, &args![f, 0u32]);
        // NiFrustum: +0 left, +4 right, +8 top, +0xc bottom, +0x10 near,
        // +0x14 far, +0x18 ortho (byte).
        let tangent = e.call(TANGENT, &args![angle]).f32() as f64;
        e.mem.set_f32(f, super::narrow(-tangent * unit * aspect));
        let tangent = e.call(TANGENT, &args![angle]).f32() as f64;
        e.mem.set_f32(f + 4, super::narrow(tangent * unit * aspect));
        let tangent = e.call(TANGENT, &args![angle]).f32() as f64;
        e.mem.set_f32(f + 0xc, super::narrow(-tangent * unit));
        let tangent = e.call(TANGENT, &args![angle]).f32() as f64;
        e.mem.set_f32(f + 8, super::narrow(tangent * unit));
        e.mem.set_u8(f + 0x18, 0);
        e.mem.set_f32(f + 0x10, 1.0);
        let far = e.global::<f32>(FAR_PLANE);
        e.mem.set_f32(f + 0x14, far);
        e.call(NI_CAMERA_SET_VIEW_FRUSTUM, &args![camera, f]);
    });
}

// Typed from 00709e80
/// Constructor of a `BSSimpleArray<Tile::Value *>`: sets the vtable, then
/// runs the base initialization `006b3eb0(this, a, b)`. Returns `this`.
pub fn fn_00709e80(e: &mut Engine, this: u32, size: u32, count: u32) -> u32 {
    e.mem.set_u32(this, SIMPLE_ARRAY_VTABLE);
    e.call(ARRAY_INIT, &args![this, size, count]);
    this
}

// Typed from 00709eb0
/// Destructor body of the same array: sets the vtable, then
/// `008454f0(this, 1)` (releases the storage).
pub fn fn_00709eb0(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, SIMPLE_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 00709ed0
/// `BSSimpleArray<Tile::Value_P_8>::_scalar_deleting_destructor_` (Xbox
/// PDB): runs [`fn_00709eb0`], then frees the object when bit 0 of `flags`
/// is set. Returns `this`.
pub fn bs_simple_array_tile_value_p_8_scalar_deleting_destructor(
    e: &mut Engine,
    this: u32,
    flags: u32,
) -> u32 {
    typed_call!(super::FAMILY, 0x0070_9eb0, fn_00709eb0(e, this));
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

/// This file's typed functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00709b50, fn_00709b50(u8, f32, f32, u32)),
        entry!(0x00709b80, fn_00709b80(u32, u8, f32, f32, u32)),
        entry!(0x00709bc0, interface_is_pipboy_opening() -> bool),
        entry!(0x00709be0, fn_00709be0() -> bool),
        entry!(0x00709c00, fn_00709c00() -> bool),
        entry!(0x00709c20, fn_00709c20(u32)),
        entry!(0x00709c40, interface_set_gun_scope_visible(u8)),
        entry!(0x00709c60, fn_00709c60(u8)),
        entry!(0x00709c80, fn_00709c80(u8)),
        entry!(0x00709ca0, fn_00709ca0()),
        entry!(0x00709cb0, fn_00709cb0() -> bool),
        entry!(0x00709cc0, fn_00709cc0() -> bool),
        entry!(0x00709d00, fn_00709d00() -> u8),
        entry!(0x00709d10, fn_00709d10() -> u8),
        entry!(0x00709d20, interface_set_root_map_space(u32)),
        entry!(0x00709d40, fn_00709d40() -> bool),
        entry!(0x00709d50, fn_00709d50(u32, f32)),
        entry!(0x00709e80, fn_00709e80(u32, u32, u32) -> u32),
        entry!(0x00709eb0, fn_00709eb0(u32)),
        entry!(
            0x00709ed0,
            bs_simple_array_tile_value_p_8_scalar_deleting_destructor(u32, u32) -> u32
        ),
    ]
}
