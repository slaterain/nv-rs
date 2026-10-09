//! `fallout/misc/main.cpp` (Xbox PDB source unit), part 2: its functions from `008705c0` up to
//! (not including) `ffffffff` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::main`]; anything public there may be used here.
//!
//! This session covers `008705c0` to `00872f50`: the per-frame drawing
//! sequence of the main loop (the world, menu and 1st-person passes), the
//! render-target and window helpers around it, and the `HighActorCuller` /
//! `BSFadeNodeCuller` constructors.
//!
//! Conventions of the compiled code, used throughout:
//!
//! - `this` of the draw methods is the `Main` object (the pointer in the
//!   global `0x011dea0c`). Its PC offsets used here: `+0x03` bGameActive,
//!   `+0x08` the window handle, `+0x88` the world accumulator `NiPointer`,
//!   `+0x8c` the 1st-person accumulator `NiPointer`, `+0x98` the rendered
//!   menu accumulator `NiPointer`, `+0x9c` the rendering-menu flag. (The
//!   Xbox PDB has these 0x10 higher, the window handle 8 higher.)
//! - The compiler pushes a call's stack arguments before it evaluates the
//!   object expression of a method call, so `PUSH x; CALL getter; MOV
//!   ECX,EAX; CALL method` passes `x` to `method`. Every call's arguments
//!   were read from the disassembly.
//! - Functions with an exception-handling frame have their unwinding left
//!   out; their scope guards are constructed and destroyed on the normal
//!   path only.
//! - x87: floats are computed in `f64` and rounded to `f32` where the code
//!   stores a `float`.
//! - Callees whose purpose could not be confirmed from the exe are called
//!   by address and described by what they do to their arguments.

#[allow(unused_imports)]
use super::main::*;
#[allow(unused_imports)]
use crate::prelude::*;

// ---------------------------------------------------------------------------
// Callees outside this file, by exe address.
// ---------------------------------------------------------------------------

/// `NiPointer<T>::operator T*` (`00559450`): the pointer stored at `this`.
const POINTER_GET: u32 = 0x0055_9450;
/// Game-setting accessor (`0043d4d0`): the address of the setting's 4-byte
/// value (`this + 4`; a static zero when `this` is null).
const SETTING_DWORD_PTR: u32 = 0x0043_d4d0;
/// Game-setting accessor (`00408d60`): the address of the setting's 1-byte value.
const SETTING_BYTE_PTR: u32 = 0x0040_8d60;
/// Game-setting accessor (`00403e20`): the address of the setting's float value.
const SETTING_FLOAT_PTR: u32 = 0x0040_3e20;

/// Returns the pointer in the global `0x011f4748`. Its vtable slot `0xc8`
/// returns an object whose slots `0x8c` / `0x90` answer a width and a
/// height for index 0 (a render-target group).
const GET_GLOBAL_011F4748: u32 = 0x0043_c4b0;
/// Returns the pointer in the global `0x011f91ac`.
const GET_GLOBAL_011F91AC: u32 = 0x004e_3270;
/// Returns the pointer in the global `0x011f91a8`: the object that
/// `00b6e110` (create a render target) and `00b6da10` (release one) are
/// methods of.
const GET_GLOBAL_011F91A8: u32 = 0x004a_0ea0;
/// Returns the pointer in the global `0x011d8a80`.
const GET_GLOBAL_011D8A80: u32 = 0x004b_7210;
/// Returns the pointer in the global `0x011df1a8`.
const GET_GLOBAL_011DF1A8: u32 = 0x0045_37b0;
/// Returns the pointer in the global `0x011ca438`.
const GET_GLOBAL_011CA438: u32 = 0x0054_f4c0;
/// Returns the multithreaded-rendering system (the constant address
/// `0x01200088`).
const GET_MT_SYSTEM: u32 = 0x004e_a970;
/// `MTRenderingSystem::SetThreadStage` (`00ba30f0`, Xbox PDB name): `this`,
/// thread index, stage; the body only reads the thread index.
const MT_SET_THREAD_STAGE: u32 = 0x00ba_30f0;
/// The sibling of `00ba30f0` (`00ba3130`): `this`, thread index, stage.
const MT_ADVANCE_THREAD_STAGE: u32 = 0x00ba_3130;
/// Whole-system calls on the multithreaded-rendering system
/// (`00ba2f30`, `00ba2fa0`), each with only `this`.
const MT_BEGIN_FRAME: u32 = 0x00ba_2f30;
const MT_END_FRAME: u32 = 0x00ba_2fa0;
/// The pointer at index `i` of the global array at `0x011f91c8` (`00450b80`).
const INDEXED_GLOBAL: u32 = 0x0045_0b80;
/// The object held by the `NiPointer` global `0x011deb7c` (`0045c670`).
const GET_ROOT: u32 = 0x0045_c670;
/// The object held by the `NiPointer` at `this + 0xac` (`006629f0`).
const GET_ROOT_CHILD: u32 = 0x0066_29f0;
/// `GET_ROOT` then `GET_ROOT_CHILD` (`00524c90`).
const GET_ROOT_CHILD_OF_ROOT: u32 = 0x0052_4c90;
/// The address `this + 0x8c` (`0045bb80`): a three-float member.
const MEMBER_8C: u32 = 0x0045_bb80;
/// The address `this + 0x8c` (`0045bbe0`, a second body of the same shape).
const MEMBER_8C_B: u32 = 0x0045_bbe0;

/// `Interface::IsMenuIDVisible` (`00702680`, Xbox PDB), cdecl: menu id, flag.
const IS_MENU_ID_VISIBLE: u32 = 0x0070_2680;
/// `Interface::IsInMenuMode` (`00702360`, Xbox PDB).
const IS_IN_MENU_MODE: u32 = 0x0070_2360;
/// `Interface::IsTopMenuID` (`00702450`, Xbox PDB), cdecl: menu id.
const IS_TOP_MENU_ID: u32 = 0x0070_2450;
/// `Interface::IsInPipboyMenu` (`00705a00`, Xbox PDB).
const IS_IN_PIPBOY_MENU: u32 = 0x0070_5a00;
/// `Interface::IsInRenderedMenu` (`007079b0`, Xbox PDB).
const IS_IN_RENDERED_MENU: u32 = 0x0070_79b0;
/// `Interface::GetCurrentRenderedMenu` (`00707ad0`, Xbox PDB).
const GET_CURRENT_RENDERED_MENU: u32 = 0x0070_7ad0;
/// `Interface::IsCurrentRenderedMenuTopmost` (`00707b30`, Xbox PDB).
const IS_CURRENT_RENDERED_MENU_TOPMOST: u32 = 0x0070_7b30;
/// `Interface::LastMinuteUpdate` (`007058e0`, Xbox PDB).
const LAST_MINUTE_UPDATE: u32 = 0x0070_58e0;
/// `Interface::GetPipboy` (`00705990`, Xbox PDB).
const GET_PIPBOY: u32 = 0x0070_5990;
/// `FORenderedMenu::SetScreenTexture` (`007fae70`, Xbox PDB).
const SET_SCREEN_TEXTURE: u32 = 0x007f_ae70;
/// `Tile::GetMenuByClass` (`00a09030`, Xbox PDB), cdecl: menu id.
const GET_MENU_BY_CLASS: u32 = 0x00a0_9030;
/// `Tile::Lock` / `Tile::Unlock` (`00a044f0`, `00a04500`, Xbox PDB).
const TILE_LOCK: u32 = 0x00a0_44f0;
const TILE_UNLOCK: u32 = 0x00a0_4500;

/// Constructor of a 16-byte value of four floats (`00414430`).
const COLOR_CTOR: u32 = 0x0041_4430;
/// Constructor of a 12-byte value of three words (`0043d410`).
const POINT3_CTOR: u32 = 0x0043_d410;
/// Constructor of an 8-byte value of two floats (`00452dc0`); returns `this`.
const POINT2_CTOR: u32 = 0x0045_2dc0;
/// Method (`00712e60`) of the object that takes a pointer to a four-float
/// value and stores it; the draw routines call it on the root's child.
const SET_COLOR: u32 = 0x0071_2e60;
/// Profiler scope guard constructor (`00404eb0`): byte, flag, file name, line.
const PROFILE_SCOPE_CTOR: u32 = 0x0040_4eb0;
/// Profiler scope guard destructor (`00404ee0`).
const PROFILE_SCOPE_DTOR: u32 = 0x0040_4ee0;
/// The source file name the scope guards pass (`0x010829c4`).
const SOURCE_FILE_NAME: u32 = 0x0108_29c4;
/// Float helper (`00404010`), cdecl: two floats, returns a float in ST0.
const FLOAT_HELPER_A: u32 = 0x0040_4010;
/// Float helper (`0040ebd0`), cdecl: two floats, returns a float in ST0.
const FLOAT_HELPER_B: u32 = 0x0040_ebd0;
/// Float helper (`008731a0`), cdecl: a double and a float, returns a
/// double in ST0 (also part of this unit, beyond this session's range).
const FLOAT_HELPER_C: u32 = 0x0087_31a0;

/// `BSTimer::Enable` / `BSTimer::Disable` (Xbox PDB).
const TIMER_ENABLE: u32 = 0x00aa_5040;
const TIMER_DISABLE: u32 = 0x00aa_50a0;
/// `Controls::GetInstance`, `Controls::ClearKeystrokes`, `Controls::Poll`,
/// `Controls::ClearUserActions` (Xbox PDB names).
const CONTROLS_GET_INSTANCE: u32 = 0x007f_df30;
const CONTROLS_CLEAR_KEYSTROKES: u32 = 0x00a2_37b0;
const CONTROLS_POLL: u32 = 0x00a2_3010;
const CONTROLS_CLEAR_USER_ACTIONS: u32 = 0x00a2_53d0;

// Import slots (kernel32 / user32).
/// `GetActiveWindow`.
const IMPORT_GET_ACTIVE_WINDOW: u32 = 0x00fd_f2fc;
/// `GetWindowLongA`.
const IMPORT_GET_WINDOW_LONG: u32 = 0x00fd_f2e8;
/// `AdjustWindowRectEx`.
const IMPORT_ADJUST_WINDOW_RECT_EX: u32 = 0x00fd_f2f8;
/// `SetWindowPos`.
const IMPORT_SET_WINDOW_POS: u32 = 0x00fd_f2a4;
/// `DebugBreak`.
const IMPORT_DEBUG_BREAK: u32 = 0x00fd_f0c8;

// Globals.
/// The `Main` object.
const MAIN_OBJECT: u32 = 0x011d_ea0c;
/// The pointer in `0x011dea10` (the world / cell grid owner).
const WORLD_OBJECT: u32 = 0x011d_ea10;
/// The player-character pointer (`0x011dea3c`).
const PLAYER_OBJECT: u32 = 0x011d_ea3c;
/// Integer setting; above 1, the extra worker threads run.
const SETTING_THREADS: u32 = 0x011c_3ea4;
/// The `NiPointer` holding the root object (`0x011deb7c`).
const ROOT_POINTER: u32 = 0x011d_eb7c;
/// The `NiPointer` holding the menu-background render target (`0x011ded3c`).
const MENU_TARGET_POINTER: u32 = 0x011d_ed3c;
/// The `NiPointer` holding the offscreen-interface render target (`0x011dec64`).
const OFFSCREEN_TARGET_POINTER: u32 = 0x011d_ec64;

/// Main-object offsets (see the module documentation).
const MAIN_GAME_ACTIVE: u32 = 0x03;
const MAIN_WINDOW: u32 = 0x08;
const MAIN_WORLD_ACCUM: u32 = 0x88;
const MAIN_FIRST_PERSON_ACCUM: u32 = 0x8c;
const MAIN_MENU_ACCUM: u32 = 0x98;
const MAIN_RENDERING_MENU: u32 = 0x9c;

// ---------------------------------------------------------------------------
// Small helpers.
// ---------------------------------------------------------------------------

fn pointer_get(e: &mut Engine, slot: u32) -> u32 {
    e.call(POINTER_GET, &args![slot]).u32()
}

/// The int value of a game setting.
fn setting_dword(e: &mut Engine, setting: u32) -> i32 {
    let value = e.call(SETTING_DWORD_PTR, &args![setting]).u32();
    e.mem.i32(value)
}

/// The byte value of a game setting.
fn setting_byte(e: &mut Engine, setting: u32) -> u8 {
    let value = e.call(SETTING_BYTE_PTR, &args![setting]).u32();
    e.mem.u8(value)
}

/// `MTRenderingSystem::SetThreadStage(thread, stage)` on the system object.
fn mt_set_stage(e: &mut Engine, thread: u32, stage: u32) {
    let system = e.call(GET_MT_SYSTEM, &args![]).u32();
    e.call(MT_SET_THREAD_STAGE, &args![system, thread, stage]);
}

/// The `00ba3130` call on the same system object.
fn mt_advance_stage(e: &mut Engine, thread: u32, stage: u32) {
    let system = e.call(GET_MT_SYSTEM, &args![]).u32();
    e.call(MT_ADVANCE_THREAD_STAGE, &args![system, thread, stage]);
}

/// Both calls, for thread 0 then thread 1, as the draw sequence does per stage.
fn mt_both_threads(e: &mut Engine, stage: u32) {
    mt_set_stage(e, 0, stage);
    mt_advance_stage(e, 1, stage);
}

fn is_menu_visible(e: &mut Engine, menu_id: u32) -> bool {
    e.call(IS_MENU_ID_VISIBLE, &args![menu_id, 0u32]).bool()
}

/// Runs `body` between a profiler scope guard's constructor and destructor.
fn with_profile_scope<R>(
    e: &mut Engine,
    kind: u32,
    line: u32,
    body: impl FnOnce(&mut Engine) -> R,
) -> R {
    e.with_stack(0x10, |e, guard| {
        e.call(
            PROFILE_SCOPE_CTOR,
            &args![guard, kind, 1u32, SOURCE_FILE_NAME, line],
        );
        let result = body(e);
        e.call(PROFILE_SCOPE_DTOR, &args![guard]);
        result
    })
}

/// Builds a four-float value at `at` with `00414430`.
fn color_ctor(e: &mut Engine, at: u32, a: f32, b: f32, c: f32, d: f32) {
    e.call(COLOR_CTOR, &args![at, a, b, c, d]);
}

/// Builds the four-float value `(0, width_ratio, height_ratio, 0)` and
/// hands it to `00712e60` on the root's child.
fn set_ratio_color(e: &mut Engine, width_ratio: f32, height_ratio: f32) {
    e.with_stack(0x10, |e, color| {
        let at = color.addr();
        color_ctor(e, at, 0.0, width_ratio, height_ratio, 0.0);
        let child = e.call(GET_ROOT_CHILD_OF_ROOT, &args![]).u32();
        e.call(SET_COLOR, &args![child, at]);
    });
}

/// The low dword of a 64-bit `FISTP` in truncating rounding mode.
fn truncate_low_dword(value: f32) -> u32 {
    (value as f64 as i64) as u32
}

// ---------------------------------------------------------------------------
// The translated functions.
// ---------------------------------------------------------------------------

// Translated from 008705c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the byte flag in the global `0x01189184`.
pub fn fn_008705c0(e: &mut Engine) -> u8 {
    e.global::<u8>(0x0118_9184)
}

// Translated from 008705d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// An idle-time step: `0086f640`, `Main::OnIdle_UpdateProcessLists`
/// (`0086f890`, Xbox PDB), `00552570` when the thread setting is above 1,
/// then `0086f670`. (The decompiler dropped the second call.)
pub fn fn_008705d0(e: &mut Engine, this: Ptr) {
    e.call(0x0086_f640, &args![this]);
    e.call(0x0086_f890, &args![this]);
    if setting_dword(e, SETTING_THREADS) > 1 {
        e.call(0x0055_2570, &args![]);
    }
    e.call(0x0086_f670, &args![this]);
}

// Translated from 00870610 (decompiled, FalloutNV.exe 1.4.0.525)
/// An idle-time step: delayed script actions
/// (`Script::RunDelayedScriptActionsOnReferences`, `005aa720`, Xbox PDB);
/// with more than one thread, three methods (`0087a710`, `0087a850`,
/// `0087a6f0`) of the object in the global `0x011df1a8`;
/// `Interface::HandleQueuedMenuOpen` (`007094f0`, Xbox PDB); `00870680` on
/// the object in `0x011d8a80`; `DebugText::Instance(1)` and its
/// `HandleQueuedPrints` (Xbox PDB).
pub fn fn_00870610(e: &mut Engine, _this: Ptr) {
    e.call(0x005a_a720, &args![]);
    if setting_dword(e, SETTING_THREADS) > 1 {
        for method in [0x0087_a710, 0x0087_a850, 0x0087_a6f0] {
            let object = e.call(GET_GLOBAL_011DF1A8, &args![]).u32();
            e.call(method, &args![object]);
        }
    }
    e.call(0x0070_94f0, &args![]);
    let interface = e.call(GET_GLOBAL_011D8A80, &args![]).u32();
    fn_00870680(e, Ptr::new(interface));
    let debug_text = e.call(0x00a0_d9e0, &args![1u32]).u32();
    e.call(0x00a0_e770, &args![debug_text]);
}

// Translated from 00870680 (decompiled, FalloutNV.exe 1.4.0.525)
/// If the byte at `+0x4cc` is set, closes all support menus
/// (`Interface::CloseAllSupportMenus(0)`, `00706320`, Xbox PDB) and clears
/// the byte.
pub fn fn_00870680(e: &mut Engine, this: Ptr) {
    if e.mem.u8(this.addr() + 0x4cc) != 0 {
        e.call(0x0070_6320, &args![0u32]);
        e.mem.set_u8(this.addr() + 0x4cc, 0);
    }
}

// Translated from 008706b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Picks the frame's draw routine. First copies the three words at `+0x8c`
/// of the root's child into the `+0x1f0` triple of the indexed global 0
/// (`00870790`), after reading the byte `+0x131` of that object
/// (`00870770`). Then draws the menu-background frame (`008707c0`) when
/// `menu_background` is set and menu `0x3f5` is not visible, otherwise
/// the frame `00870bd0` (byte clear) or `00870a00` (byte set).
pub fn fn_008706b0(e: &mut Engine, this: Ptr, target: u32, menu_background: u8, flag: u8) {
    let first = e.call(INDEXED_GLOBAL, &args![0u32]).u32();
    let byte_131 = fn_00870770(e, Ptr::new(first));
    let root = e.call(GET_ROOT, &args![]).u32();
    let child = e.call(GET_ROOT_CHILD, &args![root]).u32();
    let position = e.call(MEMBER_8C, &args![child]).u32();
    let words = [
        e.mem.u32(position),
        e.mem.u32(position + 4),
        e.mem.u32(position + 8),
    ];
    let second = e.call(INDEXED_GLOBAL, &args![0u32]).u32();
    fn_00870790(e, Ptr::new(second), words[0], words[1], words[2]);
    if menu_background != 0 && !is_menu_visible(e, 0x3f5) {
        fn_008707c0(e, this, target, byte_131);
    } else if byte_131 == 0 {
        fn_00870bd0(e, this, target, flag);
    } else {
        fn_00870a00(e, this, target, flag);
    }
}

// Translated from 00870770 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte at `+0x131`.
pub fn fn_00870770(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x131)
}

// Translated from 00870790 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores three words (a position) at `+0x1f0`, `+0x1f4`, `+0x1f8`.
pub fn fn_00870790(e: &mut Engine, this: Ptr, first: u32, second: u32, third: u32) {
    e.mem.set_u32(this.addr() + 0x1f0, first);
    e.mem.set_u32(this.addr() + 0x1f4, second);
    e.mem.set_u32(this.addr() + 0x1f8, third);
}

// Translated from 008707c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The draw sequence of a frame with a menu background: resets the render
/// stages `1` to `0x10`, builds the frame (`00872f50`), draws the menu pass
/// (`00874b90`), then, depending on the pipboy menu, either hands the
/// conversation target (`008d80e0`) the first-person accumulator and the
/// root child's... sky object and draws the world (`00875110`), or only
/// advances stage `0x16`; then the overlays and `00876850`.
///
/// `target` is the object the frame renders into (zero for the default
/// target) and `flag` is the byte `+0x131` of the indexed global 0.
pub fn fn_008707c0(e: &mut Engine, this: Ptr, target: u32, flag: u8) {
    let global_91ac = e.call(GET_GLOBAL_011F91AC, &args![]).u32();
    let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
    let byte_flag = e.call(0x004d_c310, &args![]).u8();
    let menu = e.call(GET_CURRENT_RENDERED_MENU, &args![]).u32();
    let rendered = if menu != 0 {
        let menu = e.call(GET_CURRENT_RENDERED_MENU, &args![]).u32();
        e.call(0x007f_a950, &args![menu]).u32()
    } else {
        0
    };
    fn_00872ad0(e, this);
    for stage in 1..0x11u32 {
        mt_both_threads(e, stage);
    }
    fn_00871290(e, this);
    let frame = fn_00872f50(e, this, renderer, target, flag, byte_flag);
    e.call(
        0x0087_4b90,
        &args![this, global_91ac, renderer, byte_flag, target],
    );
    if setting_dword(e, SETTING_THREADS) > 1 {
        e.call(0x008c_80e0, &args![0u32]);
    }
    mt_both_threads(e, 0x15);
    if !e.call(IS_IN_PIPBOY_MENU, &args![]).bool() {
        mt_both_threads(e, 0x16);
    } else {
        let sky = e.call(GET_ROOT_CHILD_OF_ROOT, &args![]).u32();
        let root = e.call(GET_ROOT, &args![]).u32();
        let conversation_target = e.call(0x008d_80e0, &args![root]).u32();
        let accumulator = pointer_get(e, this.addr() + MAIN_FIRST_PERSON_ACCUM);
        e.call(0x004a_0fd0, &args![conversation_target, accumulator]);
        e.call(0x0041_fd00, &args![conversation_target, sky]);
        let indexed = e.call(INDEXED_GLOBAL, &args![0u32]).u32();
        e.call(0x00b5_e870, &args![indexed, conversation_target]);
        e.call(0x0087_4c10, &args![this]);
        e.call(0x0087_5110, &args![this, renderer, rendered, 0u32, frame]);
    }
    e.call(0x0087_5bf0, &args![this]);
    if fn_008709f0(e) != 0 && flag == 0 {
        e.call(
            0x0087_5fd0,
            &args![this, byte_flag, frame, renderer, target],
        );
        if rendered != 0 {
            e.call(0x0087_61e0, &args![this, rendered, renderer]);
        }
    } else if target != 0 {
        e.call(0x0087_6830, &args![this, byte_flag, target]);
    }
    e.call(
        0x0087_6850,
        &args![this, frame, global_91ac, renderer, 0u32],
    );
}

// Translated from 008709f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the byte flag in the global `0x011f91a4`.
pub fn fn_008709f0(e: &mut Engine) -> u8 {
    e.global::<u8>(0x011f_91a4)
}

/// The test `00870a00` and `00870bd0` make before drawing the world: false
/// when `forced_off` or `other` is set, when `005585e0` of the object from
/// `00705910` is non-zero, or when `00705100` is set.
fn world_draw_enabled(e: &mut Engine, forced_off: bool, other: u8) -> bool {
    if forced_off || other != 0 {
        return false;
    }
    let interface = e.call(0x0070_5910, &args![]).u32();
    if e.call(0x0055_85e0, &args![interface]).u32() != 0 {
        return false;
    }
    e.call(0x0070_5100, &args![]).u8() == 0
}

// Translated from 00870a00 (decompiled, FalloutNV.exe 1.4.0.525)
/// The draw sequence of a frame (variant with the byte `+0x131` set):
/// builds the frame (`00872f50`), draws the world through `00873200`,
/// `00874c10`, `00874b50` and `00875110` when `world_draw_enabled`
/// allows it (otherwise only advances stage `0x16`), the overlays, and
/// finishes with `00876850`.
pub fn fn_00870a00(e: &mut Engine, this: Ptr, target: u32, skip_world: u8) {
    let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
    let global_91ac = e.call(GET_GLOBAL_011F91AC, &args![]).u32();
    let player = e.mem.u32(PLAYER_OBJECT);
    let player_child = e.call(0x0095_0bb0, &args![player, 1u32]).u32();
    let flag_a = e.call(0x0045_6610, &args![player_child]).u8();
    let byte_flag = e.call(0x004d_c310, &args![]).u8();
    e.call(0x00b9_8480, &args![1u32, 1u32]);
    fn_00872ad0(e, this);
    let accumulator = fn_00872b00(e, this);
    fn_008727d0(e, this);
    let accumulator_flag = e.call(0x0087_49b0, &args![this]).u8();
    fn_00871290(e, this);
    let frame = fn_00872f50(e, this, renderer, target, 0, byte_flag);
    let draw_world = world_draw_enabled(e, flag_a != 0, skip_world);
    e.call(
        0x0087_3200,
        &args![this, accumulator, draw_world as u32, 1u32, accumulator_flag],
    );
    if draw_world {
        e.call(0x0087_4c10, &args![this]);
        e.call(0x0087_4b50, &args![this, frame, byte_flag, 0u32]);
        e.call(
            0x0087_5110,
            &args![this, renderer, 0u32, accumulator, frame],
        );
    } else {
        mt_both_threads(e, 0x16);
    }
    if target != 0 {
        e.call(0x0087_6830, &args![this, byte_flag, target]);
    }
    e.call(0x0071_4c20, &args![1u32]);
    if fn_008709f0(e) != 0 {
        e.call(
            0x0087_5fd0,
            &args![this, byte_flag, frame, renderer, target],
        );
    } else if target != 0 {
        e.call(0x0087_6830, &args![this, byte_flag, target]);
    }
    e.call(
        0x0087_6850,
        &args![this, frame, global_91ac, renderer, 0u32],
    );
}

// Translated from 00870bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The draw sequence of a frame (variant with the byte `+0x131` clear):
/// like `00870a00`, plus the screen-split margins (`00b4f540`) when
/// `00709cb0` says so, the water pass (`008746d0`, `00875e40`) when the
/// water object of the renderer group passes `00871220`, the shadow
/// preparation of the world accumulator (`00874920`), and the targeted
/// reference's screen position (`00a6fdb0`, `00b8b1e0`).
///
/// The compiler left a branch that is never taken (`XOR ECX,ECX; JZ`) in
/// front of a call of `00875fa0`; it is dead and not translated.
pub fn fn_00870bd0(e: &mut Engine, this: Ptr, target: u32, skip_world: u8) {
    let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
    let global_91ac = e.call(GET_GLOBAL_011F91AC, &args![]).u32();
    let player = e.mem.u32(PLAYER_OBJECT);
    let player_child = e.call(0x0095_0bb0, &args![player, 1u32]).u32();
    let flag_a = e.call(0x0045_6610, &args![player_child]).u8();
    let byte_flag = e.call(0x004d_c310, &args![]).u8();
    fn_00872ad0(e, this);
    let accumulator = fn_00872b00(e, this);
    fn_008727d0(e, this);
    let accumulator_flag = e.call(0x0087_49b0, &args![this]).u8();
    fn_00871290(e, this);
    let frame = fn_00872f50(e, this, renderer, target, 0, byte_flag);
    e.call(0x0087_4ac0, &args![this, accumulator_flag]);
    e.call(0x0087_4b50, &args![this, frame, byte_flag, 5u32]);
    let draw_world = world_draw_enabled(e, flag_a != 0, skip_world);

    // The accumulator's owner flag is saved and forced on while the world
    // is set up.
    let mut saved_owner_flag = 0u8;
    if draw_world && accumulator != 0 {
        let owner = e.call(0x0043_b230, &args![accumulator]).u32();
        saved_owner_flag = e.call(0x0045_6610, &args![owner]).u8();
        let owner = e.call(0x0043_b230, &args![accumulator]).u32();
        e.call(0x0045_0f90, &args![owner, 1u32]);
    }

    let mut margins_enabled = true;
    if e.call(0x004d_e080, &args![]).bool() {
        margins_enabled = false;
    }
    let margins_wanted = e.call(0x0070_9cb0, &args![]).u8();
    if margins_wanted != 0 {
        let width = e.call(0x004d_ee10, &args![renderer]).u32();
        let height = e.call(0x004d_ee70, &args![renderer]).u32();
        let aspect_setting = e.call(SETTING_FLOAT_PTR, &args![0x011d_ea8cu32]).u32();
        let aspect = e.call(0x0070_6e70, &args![]).f32();
        let scale: f64 = e.global(0x0101_1588);
        let factor = ((aspect as f64 * e.mem.f32(aspect_setting) as f64) * scale) as f32;
        let inner_width = (width as f64 * factor as f64) as f32;
        let margin = (width as f64 - inner_width as f64) as f32;
        if margins_enabled {
            e.call(
                0x00b4_f540,
                &args![
                    truncate_low_dword(inner_width),
                    0u32,
                    truncate_low_dword(margin),
                    height
                ],
            );
        }
    }

    e.call(
        0x0087_3200,
        &args![this, accumulator, draw_world as u32, 0u32, accumulator_flag],
    );
    if draw_world && accumulator != 0 {
        let owner = e.call(0x0043_b230, &args![accumulator]).u32();
        e.call(0x0045_0f90, &args![owner, saved_owner_flag as u32]);
    }

    let water_group = e.call(0x004e_bbc0, &args![global_91ac, 4u32]).u32();
    let water_active = fn_00871220(e, Ptr::new(water_group));
    let bit_nine = e.call(0x00b8_b390, &args![global_91ac, 9u32]).bool();
    let reference_marker = bit_nine && {
        let world = e.mem.u32(WORLD_OBJECT);
        e.call(0x005f_36f0, &args![world]).u32() == 0 && setting_byte(e, 0x011d_eadc) != 0
    };

    let mut water_drawn = false;
    if water_active && draw_world {
        water_drawn = true;
        e.call(0x0087_46d0, &args![this, frame]);
        e.call(0x0087_4b50, &args![this, frame, byte_flag, 0u32]);
    }
    if e.call(0x005b_9b00, &args![]).bool() {
        let accumulator_slot = pointer_get(e, this.addr() + MAIN_WORLD_ACCUM);
        if e.call(0x00b6_3a90, &args![accumulator_slot]).u32() != 0 {
            let accumulator_slot = pointer_get(e, this.addr() + MAIN_WORLD_ACCUM);
            e.call(0x0087_4920, &args![this, accumulator_slot, frame]);
        }
    }

    if draw_world {
        e.call(0x0087_4c10, &args![this]);
        e.call(0x0087_4b50, &args![this, frame, byte_flag, 0u32]);
        if !water_drawn {
            mt_advance_stage(e, 1, 0x16);
        }
        e.call(
            0x0087_5110,
            &args![this, renderer, 0u32, accumulator, frame],
        );
    } else {
        mt_both_threads(e, 0x16);
    }

    if water_active {
        let accumulator_slot = pointer_get(e, this.addr() + MAIN_WORLD_ACCUM);
        e.call(0x0087_5e40, &args![this, accumulator_slot, frame]);
    }

    if reference_marker {
        let world = e.mem.u32(WORLD_OBJECT);
        if e.call(0x005f_36f0, &args![world]).u32() == 0 {
            screen_position_of_reference(e);
        }
    }

    if fn_008709f0(e) != 0 {
        e.call(
            0x0087_5fd0,
            &args![this, byte_flag, frame, renderer, target],
        );
        if (fn_00871260(e) || e.call(0x005b_ac20, &args![]).bool())
            && !e.call(IS_IN_PIPBOY_MENU, &args![]).bool()
        {
            e.call(0x0087_6a20, &args![this, renderer]);
        }
    } else if target != 0 {
        e.call(0x0087_6830, &args![this, byte_flag, target]);
    }

    if margins_wanted != 0 {
        if margins_enabled {
            e.call(0x00b5_40d0, &args![]);
        }
        if e.call(0x004d_e080, &args![]).bool() {
            let renderer_object = e.call(GET_GLOBAL_011F4748, &args![]).u32();
            e.vcall(renderer_object, 0xac, &args![0x011a_9bd0u32]);
            e.with_stack(0x10, |e, color| {
                let at = color.addr();
                color_ctor(e, at, 0.0, 0.0, 0.0, 0.0);
                e.mem.set_f32(at, 0.0);
                let second: f32 = e.global(0x0102_6994);
                e.mem.set_f32(at + 4, second);
                e.mem.set_f32(at + 8, 1.0);
                e.mem.set_f32(at + 12, 0.0);
                let renderer_object = e.call(GET_GLOBAL_011F4748, &args![]).u32();
                e.call(0x0071_48c0, &args![renderer_object, at, 1u32]);
                let first: f32 = e.global(0x0103_40a4);
                e.mem.set_f32(at, first);
                e.mem.set_f32(at + 4, 1.0);
                e.mem.set_f32(at + 8, 1.0);
                e.mem.set_f32(at + 12, 0.0);
                let renderer_object = e.call(GET_GLOBAL_011F4748, &args![]).u32();
                e.call(0x0071_48c0, &args![renderer_object, at, 1u32]);
            });
        }
    }
    e.call(
        0x0087_6850,
        &args![this, frame, global_91ac, renderer, accumulator],
    );
}

/// The block of `00870bd0` that projects the position of the reference in
/// the global `0x011dea20` to the screen (`00a6fdb0` on the root child's
/// child) and hands the result to `00b8b1e0` through a two-float value.
fn screen_position_of_reference(e: &mut Engine) {
    let picked = e.mem.u32(0x011d_ea20);
    if picked == 0 || e.call(0x0045_cd60, &args![picked]).u32() == 0 {
        return;
    }
    let picked = e.mem.u32(0x011d_ea20);
    let reference = e.call(0x0045_cd60, &args![picked]).u32();
    let node = e.vcall(reference, 4, &args![]).u32();
    let position = e.call(MEMBER_8C, &args![node]).u32();
    e.with_stack(0x20, |e, block| {
        // The position (3 words) at +0x0 and three output floats.
        let at = block.addr();
        for i in 0..3 {
            let word = e.mem.u32(position + 4 * i);
            e.mem.set_u32(at + 4 * i, word);
        }
        let (output_a, output_b, output_c) = (at + 0xc, at + 0x10, at + 0x14);
        e.mem.set_f32(output_a, 0.0);
        e.mem.set_f32(output_b, 0.0);
        e.mem.set_f32(output_c, 0.0);
        let camera = e.call(GET_ROOT, &args![]).u32();
        let camera_child = e.call(GET_ROOT_CHILD, &args![camera]).u32();
        let last_argument: f32 = e.global(0x0107_18c0);
        e.call(
            0x00a6_fdb0,
            &args![
                camera_child,
                at,
                output_a,
                output_b,
                output_c,
                last_argument
            ],
        );
        let value_a = e.mem.f32(output_a);
        let value_b = e.mem.f32(output_b);
        e.with_stack(8, |e, pair| {
            let pair = e.call(POINT2_CTOR, &args![pair, value_a, value_b]).u32();
            let global = e.call(GET_GLOBAL_011F91AC, &args![]).u32();
            e.call(0x00b8_b1e0, &args![global, pair]);
        });
    });
}

// Translated from 00871220 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the float at `+0x70` is greater than the double at `0x01012060`.
pub fn fn_00871220(e: &mut Engine, this: Ptr) -> bool {
    let value = e.mem.f32(this.addr() + 0x70) as f64;
    let limit: f64 = e.global(0x0101_2060);
    value > limit
}

// Translated from 00871260 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the `NiPointer` global `0x011c7810` holds an object for which
/// `00453470` returns non-zero.
pub fn fn_00871260(e: &mut Engine) -> bool {
    if pointer_get(e, 0x011c_7810) == 0 {
        return false;
    }
    let object = pointer_get(e, 0x011c_7810);
    e.call(0x0045_3470, &args![object]).u32() != 0
}

// Translated from 00871290 (decompiled, FalloutNV.exe 1.4.0.525)
/// Per-frame shadow preparation, in three parts:
///
/// - When `005bf7b0` and `00525420` are both set, remembers the first in
///   `0x011def08`, calls `005bf770(0)`, prepares the shadow scene node
///   (the indexed global 0, `00b5b1f0`) and calls `00b6ee70` with 0 on the
///   object in `0x011f91a8`. When that state ends (the flag is set and
///   `00525420` is not), calls `005bf770(1)` and `00b6ee70` with the
///   shadow-caster limit.
/// - Then, when `005bf7b0` and the byte setting `0x011c74d4` are set,
///   prepares the scene node (`00b4f570`, `00b5b060`), tells `00b6ee70` the
///   limit, and with a non-zero limit picks the shadow casters (the player
///   and the actors of the list at `0x011e0e80` that pass the checks, up to
///   the limit; each goes to `00b5cbd0`) and walks the scene node's lights
///   (`00b5afc0` / `00b5b010`), recomputing and queuing the usable ones
///   and switching the others off (`shadow_light`). With a zero limit it
///   only calls `00b5c3c0`.
/// - Otherwise advances stages `0x11` to `0x14` on both threads.
///
/// Ends with `Main::UpdateOffscreenInterface` (`00871a50`).
pub fn fn_00871290(e: &mut Engine, this: Ptr) {
    if e.call(0x005b_f7b0, &args![]).bool() && e.call(0x0052_5420, &args![]).bool() {
        let value = e.call(0x005b_f7b0, &args![]).u8();
        e.set_global(0x011d_ef08, value);
        e.call(0x005b_f770, &args![0u32]);
        let scene = e.call(INDEXED_GLOBAL, &args![0u32]).u32();
        e.call(0x00b5_b1f0, &args![scene]);
        let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
        let factory = e.call(GET_GLOBAL_011F91A8, &args![]).u32();
        e.call(0x00b6_ee70, &args![factory, renderer, 0u32]);
    } else if e.global::<u8>(0x011d_ef08) != 0 && !e.call(0x0052_5420, &args![]).bool() {
        e.set_global(0x011d_ef08, 0u8);
        e.call(0x005b_f770, &args![1u32]);
        let limit = shadow_caster_limit(e);
        let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
        let factory = e.call(GET_GLOBAL_011F91A8, &args![]).u32();
        e.call(0x00b6_ee70, &args![factory, renderer, limit]);
    }

    if e.call(0x005b_f7b0, &args![]).bool() && fn_00871a10(e) != 0 {
        e.call(0x00b4_f570, &args![]);
        let scene = e.call(INDEXED_GLOBAL, &args![0u32]).u32();
        e.call(0x00b5_b060, &args![scene]);
        let limit = shadow_caster_limit(e);
        let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
        let factory = e.call(GET_GLOBAL_011F91A8, &args![]).u32();
        e.call(0x00b6_ee70, &args![factory, renderer, limit]);
        if limit == 0 {
            e.call(0x00b5_c3c0, &args![scene]);
        } else {
            pick_shadow_casters_and_walk_lights(e, scene, limit as i32);
        }
    } else {
        for stage in 0x11..0x15u32 {
            mt_both_threads(e, stage);
        }
    }
    main_update_offscreen_interface(e, this);
}

/// The shadow-caster limit: the setting `0x011c72d8` when `005f36f0` of the
/// world object is non-zero, otherwise `0x011c72e4`.
fn shadow_caster_limit(e: &mut Engine) -> u32 {
    let world = e.mem.u32(WORLD_OBJECT);
    let setting = if e.call(0x005f_36f0, &args![world]).u32() != 0 {
        0x011c_72d8
    } else {
        0x011c_72e4
    };
    let value = e.call(SETTING_DWORD_PTR, &args![setting]).u32();
    e.mem.u32(value)
}

/// The second part of `00871290` for a non-zero `limit`: picks the shadow
/// casters (the player's, then actors of the list at `0x011e0e80`) and
/// walks the scene node's lights.
fn pick_shadow_casters_and_walk_lights(e: &mut Engine, scene: u32, limit: i32) {
    let mut first_caster = 0u32;
    let mut chosen = 0i32;
    let player = e.mem.u32(PLAYER_OBJECT);
    if e.call(0x004e_af60, &args![player]).bool() || e.call(0x0095_0090, &args![player]).bool() {
        let player = e.mem.u32(PLAYER_OBJECT);
        if !e.call(0x008c_51c0, &args![player]).bool() {
            let player = e.mem.u32(PLAYER_OBJECT);
            let child = e.call(0x0095_0bb0, &args![player, 0u32]).u32();
            first_caster = e.call(0x00b5_cbd0, &args![scene, child]).u32();
            chosen += 1;
        }
    }
    e.call(0x0040_fbf0, &args![0x011f_1180u32, 0u32]);
    let actors = fn_00871a30(e, Ptr::new(0x011e_0e80)).addr();
    let count = e.call(0x0047_2380, &args![0x011e_0e80u32]).i32();
    let mut index = 0i32;
    while index < count && chosen < limit {
        let actor = e.mem.u32(actors + 4 * index as u32);
        index += 1;
        if e.vcall(actor, 0x1d0, &args![]).u32() == 0 {
            continue;
        }
        let process = e.vcall(actor, 0x1d0, &args![]).u32();
        if e.call(0x0096_11e0, &args![process]).u32() == 0 {
            continue;
        }
        if !e.call(0x0087_f200, &args![actor]).bool() {
            continue;
        }
        if e.vcall(actor, 0x214, &args![]).u32() == 4 {
            continue;
        }
        if e.call(0x008c_51c0, &args![actor]).bool() {
            continue;
        }
        let extra_key = e.call(0x0050_ea90, &args![]).u32();
        let process = e.vcall(actor, 0x1d0, &args![]).u32();
        let node = e.call(0x00a5_bdd0, &args![process, extra_key]).u32();
        let position = if node != 0 {
            e.call(0x0050_0940, &args![node]).u32()
        } else {
            0x011f_426c
        };
        let x = e.mem.f32(position);
        let y = e.mem.f32(position + 4);
        let z = e.mem.f32(position + 8);
        let partial = e.call(FLOAT_HELPER_A, &args![x, y]).f32();
        let largest = e.call(FLOAT_HELPER_A, &args![partial, z]).f32();
        let scale: f64 = e.global(0x0102_1928);
        let threshold = largest as f64 * scale;
        let process = e.vcall(actor, 0x1d0, &args![]).u32();
        let bound = e.call(0x0043_d450, &args![process]).u32();
        let radius = e.call(0x0084_d030, &args![bound]).f32() as f64;
        if radius < threshold {
            let process = e.vcall(actor, 0x1d0, &args![]).u32();
            e.call(0x00b5_cbd0, &args![scene, process]);
            chosen += 1;
        }
    }
    e.call(0x0040_fba0, &args![0x011f_1180u32]);
    e.call(0x00b5_cde0, &args![scene, first_caster]);

    let player = e.mem.u32(PLAYER_OBJECT);
    let player_child = e.call(0x0095_0bb0, &args![player, 1u32]).u32();
    if player_child == 0 {
        return;
    }
    let root = e.call(GET_ROOT, &args![]).u32();
    let conversation_target = e.call(0x008d_80e0, &args![root]).u32();
    let sky = e.call(GET_ROOT_CHILD, &args![root]).u32();
    e.call(0x0041_fd00, &args![conversation_target, sky]);
    let sky = e.call(GET_ROOT_CHILD, &args![root]).u32();
    let member = e.call(MEMBER_8C_B, &args![sky]).u32();
    e.call(0x00a6_94a0, &args![conversation_target, member]);
    let child_flag = e.call(0x0045_6610, &args![player_child]).u8();
    e.call(0x0045_0f90, &args![player_child, 1u32]);
    let saved_object = e.call(GET_GLOBAL_011CA438, &args![]).u32();
    e.call(0x004e_20b0, &args![0u32]);
    let mut saved_flag = 0u8;
    if saved_object != 0 {
        saved_flag = e.call(0x0045_6610, &args![saved_object]).u8();
        if setting_byte(e, 0x011c_71dc) == 0 {
            e.call(0x0045_0f90, &args![saved_object, 1u32]);
        }
    }

    let mut light = e.call(0x00b5_afc0, &args![scene]).u32();
    let mut processed = 0i32;
    while light != 0 {
        shadow_light(e, light, conversation_target, limit, &mut processed);
        light = e.call(0x00b5_b010, &args![scene]).u32();
    }
    while processed + 0x11 < 0x15 {
        mt_set_stage(e, 0, (processed + 0x11) as u32);
        processed += 1;
    }
    if saved_object != 0 {
        e.call(0x0045_0f90, &args![saved_object, saved_flag as u32]);
    }
    e.call(0x004e_20b0, &args![1u32]);
    e.call(0x00b5_b880, &args![scene, conversation_target]);
    e.call(0x0045_0f90, &args![player_child, child_flag as u32]);
}

/// One light of the walk in `00871290`: lights with geometry and a source
/// are first reset (`00b9bb10`); then, if fewer than `limit` have been
/// queued and neither the geometry nor the source is flagged, the light is
/// recomputed (`00b9d150`, `00b9e970`, `00b9dfc0`, `00b9fba0`) and queued
/// (`00b5d300`, `00ba02b0`) when its count is not `0xff` and its weight
/// (`00979260`) reaches the cutoff `0x01082cb0`; any other light has its
/// geometry flagged with `00450f90(.., 1)`.
fn shadow_light(
    e: &mut Engine,
    light: u32,
    conversation_target: u32,
    limit: i32,
    processed: &mut i32,
) {
    let geometry = e.call(0x004e_6540, &args![light]).u32();
    if geometry == 0 {
        return;
    }
    let source = e.call(0x005b_f820, &args![light]).u32();
    if source == 0 {
        return;
    }
    if !e.call(0x004e_6580, &args![light]).bool() {
        let geometry = e.call(0x004e_6540, &args![light]).u32();
        if e.call(0x0045_6610, &args![geometry]).bool() {
            let source = e.call(0x005b_f820, &args![light]).u32();
            if e.call(0x0045_6610, &args![source]).bool() {
                e.call(0x00b9_bb10, &args![light, 0.0f32, 1u32]);
            } else {
                if *processed < limit {
                    let geometry = e.call(0x004e_6540, &args![light]).u32();
                    e.call(0x0045_0f90, &args![geometry, 0u32]);
                }
                e.call(0x00b9_bb10, &args![light, 0.0f32, 0u32]);
            }
        }
    }

    let mut queued = false;
    let usable = *processed < limit && {
        let geometry = e.call(0x004e_6540, &args![light]).u32();
        !e.call(0x0045_6610, &args![geometry]).bool() && {
            let source = e.call(0x005b_f820, &args![light]).u32();
            !e.call(0x0045_6610, &args![source]).bool()
        }
    };
    if usable {
        let source = e.call(0x005b_f820, &args![light]).u32();
        // The source's four-word bound is copied to a local the code never reads.
        e.call(0x0043_d450, &args![source]);
        let source = e.call(0x005b_f820, &args![light]).u32();
        let position = e.call(MEMBER_8C, &args![source]).u32();
        let words = [
            e.mem.u32(position),
            e.mem.u32(position + 4),
            e.mem.u32(position + 8),
        ];
        e.call(
            0x00b9_d150,
            &args![light, 0u32, words[0], words[1], words[2]],
        );
        e.call(0x00b9_e970, &args![light, conversation_target]);
        if e.call(0x004e_6560, &args![light]).u16() != 0xff {
            let weight = e.call(0x0097_9260, &args![light]).f32() as f64;
            let cutoff: f64 = e.global(0x0108_2cb0);
            // `FCOMP` then `TEST AH,1`: skipped when below the cutoff or unordered.
            if weight >= cutoff {
                *processed += 1;
                e.with_stack(0x10, |e, point| {
                    e.call(POINT3_CTOR, &args![point, 0.0f32, 0u32, 0u32]);
                    let geometry = e.call(0x004e_6540, &args![light]).u32();
                    e.call(0x00a5_9c60, &args![geometry, point]);
                });
                e.call(
                    0x00b9_dfc0,
                    &args![light, conversation_target, (*processed - 1) as u32],
                );
                e.call(0x00b9_fba0, &args![light]);
                let indexed = e.call(INDEXED_GLOBAL, &args![0u32]).u32();
                e.call(0x0045_4b30, &args![indexed]);
                let indexed = e.call(INDEXED_GLOBAL, &args![0u32]).u32();
                e.call(0x00b5_d300, &args![indexed, light]);
                if setting_byte(e, 0x011c_7664) != 0 {
                    let source = e.call(0x005b_f820, &args![light]).u32();
                    e.call(0x00ba_0110, &args![light, source]);
                }
                e.call(0x00ba_02b0, &args![light]);
                queued = true;
            }
        }
    }
    if !queued {
        let geometry = e.call(0x004e_6540, &args![light]).u32();
        e.call(0x0045_0f90, &args![geometry, 1u32]);
    }
}

// Translated from 00871a10 (decompiled, FalloutNV.exe 1.4.0.525)
/// The byte value of the game setting at `0x011c74d4`.
pub fn fn_00871a10(e: &mut Engine) -> u8 {
    setting_byte(e, 0x011c_74d4)
}

// Translated from 00871a30 (decompiled, FalloutNV.exe 1.4.0.525)
/// The address of the member at `+0x88`.
pub fn fn_00871a30(_e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(this.addr() + 0x88)
}

// Translated from 00871a50 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Main::UpdateOffscreenInterface` (Xbox PDB): when in menu mode, one of
/// the byte settings `0x011db2cc` / `0x011d8ba0` is set and a rendered menu
/// is up: creates the offscreen target (`00b6e110`) if it does not
/// exist, unless in the pipboy sets the pipboy's screen texture to 0, and,
/// when the rendered menu is topmost or one of the menus `0x3e9`, `0x3f8`,
/// `0x423` exists, calls slot `0x0c` of the rendered menu with the
/// target (the reference of menu `0x3e9` is flagged during the call when in
/// the pipboy and that menu is visible). Otherwise runs
/// `Interface::LastMinuteUpdate` when in menu mode.
pub fn main_update_offscreen_interface(e: &mut Engine, _this: Ptr) {
    let enabled = e.call(IS_IN_MENU_MODE, &args![]).bool()
        && (setting_byte(e, 0x011d_b2cc) != 0 || setting_byte(e, 0x011d_8ba0) != 0)
        && e.call(IS_IN_RENDERED_MENU, &args![]).bool();
    if !enabled {
        if e.call(IS_IN_MENU_MODE, &args![]).bool() {
            e.call(LAST_MINUTE_UPDATE, &args![]);
        }
        return;
    }
    with_profile_scope(e, 0xd, 0x1ade, |e| {
        let in_pipboy = e.call(IS_IN_PIPBOY_MENU, &args![]).bool();
        if e.call(IS_IN_MENU_MODE, &args![]).bool() {
            e.call(LAST_MINUTE_UPDATE, &args![]);
        }
        if pointer_get(e, OFFSCREEN_TARGET_POINTER) == 0 {
            let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
            let factory = e.call(GET_GLOBAL_011F91A8, &args![]).u32();
            let target = e
                .call(
                    0x00b6_e110,
                    &args![factory, renderer, 0x2du32, 0u32, 0u32, 0u32],
                )
                .u32();
            e.call(0x0066_b0d0, &args![OFFSCREEN_TARGET_POINTER, target]);
        }
        if !in_pipboy {
            let pipboy = e.call(GET_PIPBOY, &args![]).u32();
            e.call(SET_SCREEN_TEXTURE, &args![pipboy, 0u32]);
        }
        let wanted = e.call(IS_CURRENT_RENDERED_MENU_TOPMOST, &args![]).bool()
            || e.call(GET_MENU_BY_CLASS, &args![0x3e9u32]).u32() != 0
            || e.call(GET_MENU_BY_CLASS, &args![0x3f8u32]).u32() != 0
            || e.call(GET_MENU_BY_CLASS, &args![0x423u32]).u32() != 0;
        if e.call(IS_IN_RENDERED_MENU, &args![]).bool() && wanted {
            let marked = in_pipboy && is_menu_visible(e, 0x3e9);
            let mut reference = 0u32;
            let mut reference_flag = 0u8;
            if marked {
                let menu = e.call(GET_MENU_BY_CLASS, &args![0x3e9u32]).u32();
                reference = e.call(0x0056_c7f0, &args![menu]).u32();
                reference_flag = e.call(0x0045_6610, &args![reference]).u8();
                e.call(0x0045_0f90, &args![reference, 1u32]);
            }
            let rendered_menu = e.call(GET_CURRENT_RENDERED_MENU, &args![]).u32();
            let target = pointer_get(e, OFFSCREEN_TARGET_POINTER);
            e.vcall(rendered_menu, 0x0c, &args![target, 1u32, 0u32]);
            if marked {
                e.call(0x0045_0f90, &args![reference, reference_flag as u32]);
            }
        }
    });
}

// Translated from 00871c90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Main::SetActive` (Xbox PDB): with the byte setting `0x011deed8` set,
/// `active` becomes 1. When the controls object exists, clears keystrokes,
/// polls and clears the user actions; enables (active) or disables the
/// timer at `0x011f6394`; stores `active` at `+0x03`.
pub fn main_set_active(e: &mut Engine, this: Ptr, active: u8) {
    let mut active = active;
    if setting_byte(e, 0x011d_eed8) != 0 {
        active = 1;
    }
    if e.call(CONTROLS_GET_INSTANCE, &args![]).u32() != 0 {
        for method in [
            CONTROLS_CLEAR_KEYSTROKES,
            CONTROLS_POLL,
            CONTROLS_CLEAR_USER_ACTIONS,
        ] {
            let controls = e.call(CONTROLS_GET_INSTANCE, &args![]).u32();
            e.call(method, &args![controls]);
        }
    }
    if active != 0 {
        e.call(TIMER_ENABLE, &args![0x011f_6394u32]);
    } else {
        e.call(TIMER_DISABLE, &args![0x011f_6394u32]);
    }
    e.mem.set_u8(this.addr() + MAIN_GAME_ACTIVE, active);
}

// Translated from 00871d10 (decompiled, FalloutNV.exe 1.4.0.525)
/// False unless `GetActiveWindow()` is the window at `+0x08`, the game is
/// in menu mode and `00711e00` of the controls object is non-zero. Then,
/// unless menu `0x41e` is on top, calls `00703080` with the two ints
/// divided by the display's width (`0x0118947c`, for `x`) and height
/// (`0x01189480`, for `y`) as floats; with menu `0x41e` on top calls
/// `00706230(x, y)`. Returns true.
pub fn fn_00871d10(e: &mut Engine, this: Ptr, x: i32, y: i32) -> bool {
    let window = e.call(IMPORT_GET_ACTIVE_WINDOW, &args![]).u32();
    if window != e.mem.u32(this.addr() + MAIN_WINDOW) {
        return false;
    }
    if !e.call(IS_IN_MENU_MODE, &args![]).bool() {
        return false;
    }
    let controls = e.call(CONTROLS_GET_INSTANCE, &args![]).u32();
    if e.call(0x0071_1e00, &args![controls]).u32() == 0 {
        return false;
    }
    if !e.call(IS_TOP_MENU_ID, &args![0x41eu32]).bool() {
        let height = e.call(0x004d_c200, &args![]).i32();
        let fraction_y = (y as f64 / height as f64) as f32;
        let width = e.call(0x004d_c1f0, &args![]).i32();
        let fraction_x = (x as f64 / width as f64) as f32;
        e.call(0x0070_3080, &args![fraction_x, fraction_y]);
    } else {
        e.call(0x0070_6230, &args![x, y]);
    }
    true
}

// Translated from 00871dc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `Main::RenderMenuBackground` (Xbox PDB): needs the player object
/// (`0x011dea3c`) to pass `008d6f30` and to return a non-zero from its
/// vtable slot `0x1d0`. Marks the `Main` object as rendering the menu,
/// re-creates the menu-background target (`00b6e110`), and when its width
/// or height differs from the renderer group's sets the root child's value
/// to the (clamped) size ratios; draws the frame through `008706b0`; while
/// menu `0x40c` is visible and `004a4040` holds, accumulates the rendered
/// menu's scene (`00b6bee0`, `00b6c0d0`); and restores the state.
pub fn main_render_menu_background(e: &mut Engine, this: Ptr) {
    with_profile_scope(e, 0xd, 0x1c19, |e| {
        let player = e.mem.u32(PLAYER_OBJECT);
        if e.call(0x008d_6f30, &args![player]).u32() == 0
            || e.vcall(player, 0x1d0, &args![]).u32() == 0
        {
            return;
        }
        let saved_flag = e.global::<u8>(0x011a_d884);
        e.set_global(0x011a_d884, 1u8);
        e.mem.set_u8(this.addr() + MAIN_RENDERING_MENU, 1);
        let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
        e.call(0x004a_0370, &args![renderer]);
        e.call(TILE_LOCK, &args![]);
        let system = e.call(GET_MT_SYSTEM, &args![]).u32();
        e.call(MT_BEGIN_FRAME, &args![system]);

        if pointer_get(e, MENU_TARGET_POINTER) != 0 {
            let target = pointer_get(e, MENU_TARGET_POINTER);
            let factory = e.call(GET_GLOBAL_011F91A8, &args![]).u32();
            e.call(0x00b6_da10, &args![factory, target]);
        }
        let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
        let factory = e.call(GET_GLOBAL_011F91A8, &args![]).u32();
        let new_target = e
            .call(
                0x00b6_e110,
                &args![factory, renderer, 6u32, 0u32, 0u32, 0u32],
            )
            .u32();
        e.call(0x0066_b0d0, &args![MENU_TARGET_POINTER, new_target]);

        let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
        let group = e.vcall(renderer, 0xc8, &args![]).u32();
        let target = pointer_get(e, MENU_TARGET_POINTER);
        let target_width = fn_00872370(e, Ptr::new(target), 0);
        let group_width = e.vcall(group, 0x8c, &args![0u32]).u32();
        let mut changed = group_width != target_width;
        if !changed {
            let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
            let group = e.vcall(renderer, 0xc8, &args![]).u32();
            let target = pointer_get(e, MENU_TARGET_POINTER);
            let target_height = fn_008723d0(e, Ptr::new(target), 0);
            let group_height = e.vcall(group, 0x90, &args![0u32]).u32();
            changed = group_height != target_height;
        }
        if changed {
            let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
            let group = e.vcall(renderer, 0xc8, &args![]).u32();
            let target = pointer_get(e, MENU_TARGET_POINTER);
            let target_width = fn_00872370(e, Ptr::new(target), 0);
            let group_width = e.vcall(group, 0x8c, &args![0u32]).u32();
            let ratio = (group_width as f64 / target_width as f64) as f32;
            let width_ratio = e.call(FLOAT_HELPER_B, &args![ratio, 1.0f32]).f32();
            let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
            let group = e.vcall(renderer, 0xc8, &args![]).u32();
            let target = pointer_get(e, MENU_TARGET_POINTER);
            let target_height = fn_008723d0(e, Ptr::new(target), 0);
            let group_height = e.vcall(group, 0x90, &args![0u32]).u32();
            let ratio = (group_height as f64 / target_height as f64) as f32;
            let height_ratio = e.call(FLOAT_HELPER_B, &args![ratio, 1.0f32]).f32();
            set_ratio_color(e, width_ratio, height_ratio);
        } else if e.global::<u8>(0x011f_9426) != 0 {
            let child = e.call(GET_ROOT_CHILD_OF_ROOT, &args![]).u32();
            e.call(SET_COLOR, &args![child, 0x011a_d840u32]);
        }

        let interface = e.call(GET_GLOBAL_011D8A80, &args![]).u32();
        let image_space = e.call(0x0071_8ab0, &args![interface]).u32();
        let modifier = e.call(0x005d_2860, &args![]).u32();
        e.call(0x0052_9c90, &args![modifier]);
        if image_space != 0 {
            e.call(0x0052_99a0, &args![image_space, 1.0f32, 0u32]);
            e.call(0x0086_fd90, &args![this, 0u32]);
        }
        let mut in_pipboy = e.call(IS_IN_PIPBOY_MENU, &args![]).u8();
        if is_menu_visible(e, 0x3f5) {
            in_pipboy = 0;
        }
        let target = pointer_get(e, MENU_TARGET_POINTER);
        fn_008706b0(e, this, target, 0, in_pipboy);

        if is_menu_visible(e, 0x40c) && e.call(0x004a_4040, &args![]).bool() {
            e.with_stack(0x20, |e, guard| {
                let at = guard.addr();
                e.call(0x004a_0eb0, &args![at, 0u32]);
                let menu = e.call(GET_CURRENT_RENDERED_MENU, &args![]).u32();
                let scene = e.call(0x008b_6200, &args![menu]).u32();
                e.call(0x0041_fd00, &args![at, scene]);
                let accumulator = pointer_get(e, this.addr() + MAIN_MENU_ACCUM);
                e.call(0x004a_0fd0, &args![at, accumulator]);
                let indexed = e.call(INDEXED_GLOBAL, &args![1u32]).u32();
                let accumulator = pointer_get(e, this.addr() + MAIN_MENU_ACCUM);
                e.call(0x004a_1020, &args![accumulator, indexed]);
                let menu = e.call(GET_CURRENT_RENDERED_MENU, &args![]).u32();
                let cell = e.call(0x0055_85e0, &args![menu]).u32();
                e.call(0x00b6_bee0, &args![scene, cell, at]);
                let accumulator = pointer_get(e, this.addr() + MAIN_MENU_ACCUM);
                e.call(0x00b6_c0d0, &args![scene, accumulator, 0u32]);
                e.call(0x004a_0fd0, &args![at, 0u32]);
                e.call(0x004a_0f60, &args![at]);
            });
        }
        if image_space != 0 {
            e.call(0x0052_9c90, &args![image_space]);
            e.call(0x0086_fd90, &args![this, 0u32]);
        }
        mt_both_threads(e, 0x17);
        let system = e.call(GET_MT_SYSTEM, &args![]).u32();
        e.call(MT_END_FRAME, &args![system]);
        e.set_global(0x011d_ea29, 1u8);
        e.with_stack(0x10, |e, color| {
            let at = color.addr();
            color_ctor(e, at, 0.0, 1.0, 1.0, 0.0);
            let child = e.call(GET_ROOT_CHILD_OF_ROOT, &args![]).u32();
            e.call(SET_COLOR, &args![child, at]);
        });
        e.call(TILE_UNLOCK, &args![]);
        let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
        e.call(0x004a_03c0, &args![renderer]);
        e.mem.set_u8(this.addr() + MAIN_RENDERING_MENU, 0);
        e.set_global(0x011a_d884, saved_flag);
    });
}

// Translated from 00872370 (decompiled, FalloutNV.exe 1.4.0.525)
/// Slot `0x94` of the object held by the `NiPointer` at
/// `this + 0x30 + 4 * index` (the width the callers compare with the
/// display's), 0 when the pointer is empty.
pub fn fn_00872370(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let object = pointer_get(e, this.addr() + 0x30 + 4 * index);
    if object != 0 {
        e.vcall(object, 0x94, &args![]).u32()
    } else {
        0
    }
}

// Translated from 008723d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Slot `0x98` of the object held by the `NiPointer` at
/// `this + 0x30 + 4 * index` (the height the callers compare with the
/// display's), 0 when the pointer is empty.
pub fn fn_008723d0(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let object = pointer_get(e, this.addr() + 0x30 + 4 * index);
    if object != 0 {
        e.vcall(object, 0x98, &args![]).u32()
    } else {
        0
    }
}

// Translated from 00872430 (decompiled, FalloutNV.exe 1.4.0.525)
/// Reads the text file named by two strings (`directory` then `name`, joined
/// into a 260-byte buffer with `00406d30` / `00406d50`, opened with the
/// mode string at `0x01082cb8`) line by line, up to 512 bytes a line. A
/// line that does not start with `#` and is longer than one character loses
/// a trailing newline and is looked up (`00462f40`) in the object in the
/// global `0x011c3f2c`; a hit is passed to `00471d00` with 1. Returns 1 if
/// any line hit. The stack-cookie check is left out.
pub fn fn_00872430(e: &mut Engine, _this: Ptr, directory: u32, name: u32) -> u8 {
    let mut found = 0u8;
    e.with_stack(0x104, |e, path| {
        let path = path.addr();
        e.call(0x0040_6d30, &args![path, 0x104u32, directory]);
        e.call(0x0040_6d50, &args![path, 0x104u32, name]);
        let file = e.call(0x00ec_9a47, &args![path, 0x0108_2cb8u32]).u32();
        if file == 0 {
            return;
        }
        e.with_stack(0x200, |e, line| {
            let line = line.addr();
            while e.call(0x00ec_c104, &args![line, 0x200u32, file]).u32() != 0 {
                let length = e.call(0x0044_a670, &args![line]).u32();
                if e.mem.i8(line) as i32 != 0x23 && length > 1 {
                    if e.mem.i8(line + length - 1) as i32 == 0xa {
                        e.mem.set_u8(line + length - 1, 0);
                    }
                    let list = e.mem.u32(0x011c_3f2c);
                    let entry = e.call(0x0046_2f40, &args![list, line]).u32();
                    if entry != 0 {
                        e.call(0x0047_1d00, &args![entry, 1u32]);
                        found = 1;
                    }
                }
            }
        });
        e.call(0x00ec_9907, &args![file]);
    });
    found
}

// Translated from 00872570 (decompiled, FalloutNV.exe 1.4.0.525)
/// Frame housekeeping, cdecl; always returns 1. With `release` set: waits
/// until `0086bdf0` is true (sleeping `0040fca0(5)` between checks), stops
/// the object in `0x0126fac4` (`00ec29b0`, `00ec1800`) when `00ec17c0` says
/// so and `00ec25e0` when `00872780` does, runs `00b631d0` on the object
/// from `00b4f5c0`, clears the word at `+4` of the second out value of
/// `005477f0` for every cell slot of the world's grid, calls
/// `Main::KillMenuBGTexture` (`00877430`, Xbox PDB), and, when `00446e10`,
/// resizes the game window to the display size (`GetWindowLongA`,
/// `AdjustWindowRectEx`, `SetWindowPos`). Without it: `00bad4a0` when the
/// byte `0x011f941e` is set, and the water system calls `004e6620(1, 0)`
/// and `004e65d0` on the world's `0070ec90` object when `005d2a40`.
pub fn fn_00872570(e: &mut Engine, release: u8) -> u8 {
    if release != 0 {
        while !e.call(0x0086_bdf0, &args![]).bool() {
            e.call(0x0040_fca0, &args![5u32]);
        }
        let movie_player = e.mem.u32(0x0126_fac4);
        if e.call(0x00ec_17c0, &args![movie_player]).bool() {
            e.call(0x00ec_29b0, &args![movie_player]);
            e.call(0x00ec_1800, &args![movie_player]);
        }
        let movie_player = e.mem.u32(0x0126_fac4);
        if fn_00872780(e, Ptr::new(movie_player)) != 0 {
            e.call(0x00ec_25e0, &args![movie_player]);
        }
        let accumulator = e.call(0x00b4_f5c0, &args![]).u32();
        if accumulator != 0 {
            e.call(0x00b6_31d0, &args![accumulator]);
        }
        let world = e.mem.u32(WORLD_OBJECT);
        let grid = e.call(0x0044_ddc0, &args![world]).u32();
        let size = e.call(0x0084_e3a0, &args![grid]).i32();
        for x in 0..size {
            for y in 0..size {
                let world = e.mem.u32(WORLD_OBJECT);
                let slot = e.call(0x0045_7050, &args![world, x, y]).u32();
                let cell = e.mem.u32(slot);
                if cell != 0 {
                    let (first, second) = e.with_stack(8, |e, out| {
                        let at = out.addr();
                        e.call(0x0054_77f0, &args![cell, at, at + 4]);
                        (e.mem.u32(at), e.mem.u32(at + 4))
                    });
                    if first != 0 && second != 0 && e.mem.u32(second + 4) != 0 {
                        e.mem.set_u32(second + 4, 0);
                    }
                }
            }
        }
        let main_object = e.mem.u32(MAIN_OBJECT);
        e.call(0x0087_7430, &args![main_object]);
        if e.call(0x0044_6e10, &args![]).bool() {
            resize_window_to_display(e);
        }
    } else {
        if e.global::<u8>(0x011f_941e) != 0 {
            e.call(0x00ba_d4a0, &args![]);
        }
        let world = e.mem.u32(WORLD_OBJECT);
        if e.call(0x0070_ec90, &args![world]).u32() != 0 && e.call(0x005d_2a40, &args![]).bool() {
            let world = e.mem.u32(WORLD_OBJECT);
            let water = e.call(0x0070_ec90, &args![world]).u32();
            e.call(0x004e_6620, &args![water, 1u32, 0u32]);
            let world = e.mem.u32(WORLD_OBJECT);
            let water = e.call(0x0070_ec90, &args![world]).u32();
            e.call(0x004e_65d0, &args![water]);
        }
    }
    1
}

/// The window resize of `00872570`: a rectangle `(0, 0, width, height)`
/// (`0x0118947c`, `0x01189480`) is grown for the window style and the
/// window set to it.
fn resize_window_to_display(e: &mut Engine) {
    e.with_stack(0x10, |e, rect| {
        let rect = rect.addr();
        e.mem.set_i32(rect, 0);
        e.mem.set_i32(rect + 4, 0);
        let width = e.call(0x004d_c1f0, &args![]).u32();
        e.mem.set_u32(rect + 8, width);
        let height = e.call(0x004d_c200, &args![]).u32();
        e.mem.set_u32(rect + 12, height);
        let main_object = e.mem.u32(MAIN_OBJECT);
        let window = e.mem.u32(main_object + MAIN_WINDOW);
        // GWL_STYLE is -16.
        let style = e
            .call(IMPORT_GET_WINDOW_LONG, &args![window, -0x10i32])
            .u32();
        e.call(
            IMPORT_ADJUST_WINDOW_RECT_EX,
            &args![rect, style, 0u32, 0u32],
        );
        let left = e.mem.i32(rect);
        let top = e.mem.i32(rect + 4);
        let right = e.mem.i32(rect + 8);
        let bottom = e.mem.i32(rect + 12);
        let main_object = e.mem.u32(MAIN_OBJECT);
        let window = e.mem.u32(main_object + MAIN_WINDOW);
        // As compiled: X = left, Y = bottom, cx = right - left, cy = top - bottom.
        e.call(
            IMPORT_SET_WINDOW_POS,
            &args![
                window,
                0u32,
                left,
                bottom,
                right.wrapping_sub(left),
                top.wrapping_sub(bottom),
                0u32
            ],
        );
    });
}

// Translated from 00872780 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the object at `+0x0c` exists and `008727b0` holds for it.
pub fn fn_00872780(e: &mut Engine, this: Ptr) -> u8 {
    let inner = e.mem.u32(this.addr() + 0x0c);
    if inner != 0 {
        fn_008727b0(e, Ptr::new(inner))
    } else {
        0
    }
}

// Translated from 008727b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the word at `+0x20` is non-zero.
pub fn fn_008727b0(e: &mut Engine, this: Ptr) -> u8 {
    (e.mem.u32(this.addr() + 0x20) != 0) as u8
}

// Translated from 008727d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// With the byte setting `0x011c7adc` set: sets the word `0x011a9608` to 1
/// for the duration, and either (when `005468f0(0xb)` is false and the
/// world object has no `005f36f0` result or `004518e0` holds for it) has
/// `004e1bc0` of the world's `0070ec90` object draw with the indexed global
/// 0 flagged by `005f9af0`, or resets the state (`0x011ff104`,
/// `0x011ff8c4`, the first word of the `0070ec90` object, the `Main`
/// object's `004e2190` / `004e2120`, `00872930(0)`). Always ends by
/// advancing the stages after the first word of that object up to `0x10` on
/// both threads.
pub fn fn_008727d0(e: &mut Engine, _this: Ptr) {
    if setting_byte(e, 0x011c_7adc) != 0 {
        let saved = e.global::<u32>(0x011a_9608);
        e.set_global(0x011a_9608, 1u32);
        let blocked = e.call(0x0054_68f0, &args![0xbu32]).bool();
        let first_path = !blocked && {
            let world = e.mem.u32(WORLD_OBJECT);
            if e.call(0x005f_36f0, &args![world]).u32() == 0 {
                true
            } else {
                let world = e.mem.u32(WORLD_OBJECT);
                let object = e.call(0x005f_36f0, &args![world]).u32();
                e.call(0x0045_18e0, &args![object]).bool()
            }
        };
        if first_path {
            let indexed = e.call(INDEXED_GLOBAL, &args![0u32]).u32();
            let flag = e.call(0x005f_9ad0, &args![indexed]).u8();
            e.call(0x005f_9af0, &args![indexed, 1u32]);
            let child = e.call(GET_ROOT_CHILD_OF_ROOT, &args![]).u32();
            let world = e.mem.u32(WORLD_OBJECT);
            let cell = e.call(0x0070_ec90, &args![world]).u32();
            e.call(0x004e_1bc0, &args![cell, child, indexed]);
            e.call(0x005f_9af0, &args![indexed, flag as u32]);
        } else {
            e.set_global(0x011f_f104, 0u8);
            let world = e.mem.u32(WORLD_OBJECT);
            let cell = e.call(0x0070_ec90, &args![world]).u32();
            e.mem.set_u32(cell, 0);
            let main_object = e.mem.u32(MAIN_OBJECT);
            let water = e.call(0x004e_2190, &args![main_object]).u32();
            e.call(0x004e_2120, &args![water, 0u32]);
            e.set_global(0x011f_f8c4, 0u8);
            let world = e.mem.u32(WORLD_OBJECT);
            e.call(0x0070_ec90, &args![world]);
            fn_00872930(e, 0);
        }
        e.set_global(0x011a_9608, saved);
    }
    let world = e.mem.u32(WORLD_OBJECT);
    let cell = e.call(0x0070_ec90, &args![world]).u32();
    let first = e.mem.i32(cell);
    let mut stage = first.wrapping_add(1);
    while stage < 0x11 {
        mt_both_threads(e, stage as u32);
        stage += 1;
    }
}

// Translated from 00872930 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte in the global `0x011c7a59` (cdecl).
pub fn fn_00872930(e: &mut Engine, value: u8) {
    e.set_global(0x011c_7a59, value);
}

// Translated from 00872940 (decompiled, FalloutNV.exe 1.4.0.525)
/// The draw sequence of a frame that only needs the interface: builds the
/// frame (`00872f50`), draws the menu pass (`00874b90`), calls the update
/// routine of each visible menu (`0x41e` to `00709ae0`, `0x3f6` to
/// `00709af0`, `0x438` to `00709b00`, `0x439` to `00709b10`, `0x43a` to
/// `00709b20`, `0x43b` to `00709b30`, `0x432` to `007948d0`), finishes with
/// `00876850`, and calls `007c9ca0` when menu `0x424` is visible.
pub fn fn_00872940(e: &mut Engine, this: Ptr, target: u32) {
    let renderer = e.call(GET_GLOBAL_011F4748, &args![]).u32();
    let global_91ac = e.call(GET_GLOBAL_011F91AC, &args![]).u32();
    let byte_flag = e.call(0x004d_c310, &args![]).u8();
    fn_00872ad0(e, this);
    let frame = fn_00872f50(e, this, renderer, target, 0, byte_flag);
    e.call(
        0x0087_4b90,
        &args![this, global_91ac, renderer, byte_flag, target],
    );
    for (menu, update) in [
        (0x41eu32, 0x0070_9ae0u32),
        (0x3f6, 0x0070_9af0),
        (0x438, 0x0070_9b00),
        (0x439, 0x0070_9b10),
        (0x43a, 0x0070_9b20),
        (0x43b, 0x0070_9b30),
        (0x432, 0x0079_48d0),
    ] {
        if is_menu_visible(e, menu) {
            e.call(update, &args![]);
        }
    }
    if fn_008709f0(e) != 0 {
        e.call(
            0x0087_5fd0,
            &args![this, byte_flag, frame, renderer, target],
        );
    } else if target != 0 {
        e.call(0x0087_6830, &args![this, byte_flag, target]);
    }
    e.call(
        0x0087_6850,
        &args![this, frame, global_91ac, renderer, 0u32],
    );
    if is_menu_visible(e, 0x424) {
        e.call(0x007c_9ca0, &args![]);
    }
}

// Translated from 00872ad0 (decompiled, FalloutNV.exe 1.4.0.525)
/// When `00683a60` returns something: calls `00a81a80` if `00714a00` says
/// so, then `00714960(3)`.
pub fn fn_00872ad0(e: &mut Engine, _this: Ptr) {
    if e.call(0x0068_3a60, &args![]).u32() != 0 {
        if e.call(0x0071_4a00, &args![]).bool() {
            e.call(0x00a8_1a80, &args![]);
        }
        e.call(0x0071_4960, &args![3u32]);
    }
}

// Translated from 00872b00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Per-frame camera and value setup; returns the object that the callers
/// pass to `00873200` as the world accumulator. In order: sets the root
/// child's value to the constant at `0x011ad840`; unless `0044ddc0` of
/// `0x011f2250` is 4, calls `00c52020` on the root with the float of
/// `00710ab0` of the player object (and `0`, `0`, `1`); calls `00b54000`
/// with that float; when the byte `+0x9c` of `this` is set, sets the root
/// child's value to `(0, 1, 1, 0)`; with the byte setting `0x011ded80` set,
/// constructs the static `HighActorCuller` at `0x011def10` on first use
/// (`00872e00`, `atexit` of `0x00fd9380`) and runs `008d6f50` on it with
/// the root pointer's child; resets the values of the `NiPointer` globals
/// `0x011deb34` / `0x011deda4`; moves `0x011f9fc0` to `0x011f9fc1`;
/// stores two settings through `00872dd0` / `00872df0`; clears
/// `0x011f94a5`; and returns the result of `0045cd60` on the world's
/// `008d8520` object when its `00476c90` is 2 or 3, else 0.
pub fn fn_00872b00(e: &mut Engine, this: Ptr) -> u32 {
    let root = e.call(GET_ROOT, &args![]).u32();
    let root_child = e.call(GET_ROOT_CHILD, &args![root]).u32();
    e.call(SET_COLOR, &args![root_child, 0x011a_d840u32]);
    if e.call(0x0044_ddc0, &args![0x011f_2250u32]).u32() != 4 {
        let player = e.mem.u32(PLAYER_OBJECT);
        let field_of_view = e.call(0x0071_0ab0, &args![player]).f32();
        let root = e.call(GET_ROOT, &args![]).u32();
        e.call(0x00c5_2020, &args![root, field_of_view, 0u32, 0u32, 1u32]);
    }
    let player = e.mem.u32(PLAYER_OBJECT);
    let field_of_view = e.call(0x0071_0ab0, &args![player]).f32();
    e.call(0x00b5_4000, &args![field_of_view]);
    if e.mem.u8(this.addr() + MAIN_RENDERING_MENU) != 0 {
        e.with_stack(0x10, |e, color| {
            let at = color.addr();
            color_ctor(e, at, 0.0, 1.0, 1.0, 0.0);
            let root = e.call(GET_ROOT, &args![]).u32();
            let root_child = e.call(GET_ROOT_CHILD, &args![root]).u32();
            e.call(SET_COLOR, &args![root_child, at]);
        });
    }
    let root_pointer = pointer_get(e, ROOT_POINTER);
    let accumulator = e.call(GET_ROOT_CHILD, &args![root_pointer]).u32();
    // The compiled function has a local that stays 0 here, so the
    // `HighActorCuller` below always gets `accumulator`.
    if setting_byte(e, 0x011d_ed80) != 0 {
        let guard = e.global::<u32>(0x011d_efb4);
        if guard & 1 == 0 {
            e.set_global(0x011d_efb4, guard | 1);
            fn_00872e00(e, Ptr::new(0x011d_ef10));
            e.call(0x00ec_658f, &args![0x00fd_9380u32]);
        }
        e.call(0x008d_6f50, &args![0x011d_ef10u32, accumulator]);
    }
    for slot in [0x011d_eb34u32, 0x011d_eda4] {
        let root_pointer = pointer_get(e, ROOT_POINTER);
        let node = e.call(0x0055_8310, &args![root_pointer]).u32();
        let value = e.call(0x0043_c490, &args![node]).u32();
        let target = pointer_get(e, slot);
        e.call(0x0044_0460, &args![target, value]);
        e.with_stack(0x10, |e, point| {
            e.call(POINT3_CTOR, &args![point, 0.0f32, 0u32, 0u32]);
            let target = pointer_get(e, slot);
            e.call(0x00a5_9c60, &args![target, point]);
        });
    }
    if e.global::<u8>(0x011f_9fc0) != 0 {
        e.set_global(0x011f_9fc0, 0u8);
        e.set_global(0x011f_9fc1, 1u8);
    }
    let first = setting_byte(e, 0x011d_ecd8);
    fn_00872dd0(e, first, 1);
    let second = setting_byte(e, 0x011d_eb60);
    fn_00872df0(e, second);
    e.set_global(0x011f_94a5, 0u8);

    let mut result = 0u32;
    let world = e.mem.u32(WORLD_OBJECT);
    let highest = e.call(0x008d_8520, &args![world]).u32();
    if highest != 0 && e.call(0x0045_cd60, &args![highest]).u32() != 0 {
        let world = e.mem.u32(WORLD_OBJECT);
        let object = e.call(0x008d_8520, &args![world]).u32();
        let mode = e.call(0x0047_6c90, &args![object]).u32();
        let mode_matches = if mode == 3 {
            true
        } else {
            let world = e.mem.u32(WORLD_OBJECT);
            let object = e.call(0x008d_8520, &args![world]).u32();
            e.call(0x0047_6c90, &args![object]).u32() == 2
        };
        if mode_matches {
            let world = e.mem.u32(WORLD_OBJECT);
            let object = e.call(0x008d_8520, &args![world]).u32();
            result = e.call(0x0045_cd60, &args![object]).u32();
        }
    }
    result
}

// Translated from 00872dd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores two bytes in the globals `0x011f9fc3` and `0x011f9fc4` (cdecl).
pub fn fn_00872dd0(e: &mut Engine, first: u8, second: u8) {
    e.set_global(0x011f_9fc3, first);
    e.set_global(0x011f_9fc4, second);
}

// Translated from 00872df0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores a byte in the global `0x011f9fc5` (cdecl).
pub fn fn_00872df0(e: &mut Engine, value: u8) {
    e.set_global(0x011f_9fc5, value);
}

// Translated from 00872e00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `HighActorCuller` (RTTI `.?AVHighActorCuller@@`, vtable
/// `0x01082cc0`): the base constructor `005d8b10`, the vtable, the
/// embedded `BSFadeNodeCuller` at `+0x08` (`00872ea0`) and three members
/// at `+0x98`, `+0x9c`, `+0xa0` (`00633c90` with 0). Returns `this`.
pub fn fn_00872e00(e: &mut Engine, this: Ptr) -> Ptr {
    e.call(0x005d_8b10, &args![this]);
    e.mem.set_u32(this.addr(), 0x0108_2cc0);
    fn_00872ea0(e, Ptr::new(this.addr() + 0x08), 0);
    for offset in [0x98, 0x9c, 0xa0] {
        e.call(0x0063_3c90, &args![this.addr() + offset, 0u32]);
    }
    this
}

// Translated from 00872ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of `BSFadeNodeCuller` (RTTI `.?AVBSFadeNodeCuller@@`, vtable
/// `0x01082ccc`): the base constructor `00a69400` with `size`, then the
/// vtable. Returns `this`.
pub fn fn_00872ea0(e: &mut Engine, this: Ptr, size: u32) -> Ptr {
    e.call(0x00a6_9400, &args![this, size]);
    e.mem.set_u32(this.addr(), 0x0108_2ccc);
    this
}

// Translated from 00872ed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiCullingProcess::AppendVirtual` (Xbox PDB): the body is a single
/// `DebugBreak()`; the one stack argument is not read.
pub fn ni_culling_process_append_virtual(e: &mut Engine, _this: Ptr, _unused_0: u32) {
    e.call(IMPORT_DEBUG_BREAK, &args![]);
}

// Translated from 00872ef0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSFadeNodeCuller::GetRTTI` (Xbox PDB): the address `0x01201f7c`.
pub fn bs_fade_node_culler_get_rtti(_e: &mut Engine, _this: Ptr) -> u32 {
    0x0120_1f7c
}

// Translated from 00872f00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSFadeNodeCuller::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor body (`00872f30`) and, with bit 0 of `flags` set, frees the
/// object (`00401030`, cdecl). Returns `this`.
pub fn bs_fade_node_culler_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    fn_00872f30(e, this);
    if flags & 1 != 0 {
        e.call(0x0040_1030, &args![this]);
    }
    this
}

// Translated from 00872f30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor body of `BSFadeNodeCuller` (inferred from its caller): the
/// base destructor `00a693e0`.
pub fn fn_00872f30(e: &mut Engine, this: Ptr) {
    e.call(0x00a6_93e0, &args![this]);
}

// Translated from 00872f50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Selects the frame's render target. When `008709f0` is set and
/// `skip_new_target` is 0: creates a target for `renderer` (`00b6e110`, kind
/// 4), records it in `0x011f9438`, and if the target's width or height
/// differs from the renderer's, sets the root child's value to the
/// (clamped) ratios (`set_ratio_color`); then makes the target's group
/// current (`00b6b8d0(7, group)`). Otherwise makes `screen`'s group
/// current (`00b6b260(screen)`, or slot `0xc8` of `renderer` when
/// `use_renderer_group`), or with no `screen` the default (`00b6b890(7,
/// 0)`). Returns the created target (0 when none).
pub fn fn_00872f50(
    e: &mut Engine,
    _this: Ptr,
    renderer: u32,
    screen: u32,
    skip_new_target: u8,
    use_renderer_group: u8,
) -> u32 {
    let mut created = 0u32;
    if fn_008709f0(e) != 0 && skip_new_target == 0 {
        with_profile_scope(e, 0x27, 0x1e6d, |e| {
            let factory = e.call(GET_GLOBAL_011F91A8, &args![]).u32();
            created = e
                .call(
                    0x00b6_e110,
                    &args![factory, renderer, 4u32, 0u32, 0u32, 0u32],
                )
                .u32();
            e.call(0x0087_31d0, &args![created]);
            e.set_global(0x011f_9438, created);
            let width = e.call(0x004d_ee10, &args![renderer]).u32();
            let target_width = fn_00872370(e, Ptr::new(created), 0);
            let mut changed = width != target_width;
            if !changed {
                let height = e.call(0x004d_ee70, &args![renderer]).u32();
                let target_height = fn_008723d0(e, Ptr::new(created), 0);
                changed = height != target_height;
            }
            if changed {
                let scale: f64 = e.global(0x0108_2d20);
                let width = e.call(0x004d_ee10, &args![renderer]).u32();
                let target_width = fn_00872370(e, Ptr::new(created), 0);
                let ratio = (width as f64 * scale) / target_width as f64;
                let width_ratio = e.call(FLOAT_HELPER_C, &args![ratio, 1.0f32]).f64() as f32;
                let height = e.call(0x004d_ee70, &args![renderer]).u32();
                let target_height = fn_008723d0(e, Ptr::new(created), 0);
                let ratio = (height as f64 * scale) / target_height as f64;
                let height_ratio = e.call(FLOAT_HELPER_C, &args![ratio, 1.0f32]).f64() as f32;
                set_ratio_color(e, width_ratio, height_ratio);
            }
            let group = e.call(0x00b6_b260, &args![created]).u32();
            e.call(0x00b6_b8d0, &args![7u32, group]);
        });
    } else {
        with_profile_scope(e, 0x26, 0x1e84, |e| {
            if screen != 0 {
                if use_renderer_group != 0 {
                    let group = e.vcall(renderer, 0xc8, &args![]).u32();
                    e.call(0x00b6_b8d0, &args![7u32, group]);
                } else {
                    let group = e.call(0x00b6_b260, &args![screen]).u32();
                    e.call(0x00b6_b8d0, &args![7u32, group]);
                }
            } else {
                e.call(0x00b6_b890, &args![7u32, 0u32]);
            }
        });
    }
    created
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x008705c0, fn_008705c0() -> u8),
        entry!(0x008705d0, fn_008705d0(Ptr)),
        entry!(0x00870610, fn_00870610(Ptr)),
        entry!(0x00870680, fn_00870680(Ptr)),
        entry!(0x008706b0, fn_008706b0(Ptr, u32, u8, u8)),
        entry!(0x00870770, fn_00870770(Ptr) -> u8),
        entry!(0x00870790, fn_00870790(Ptr, u32, u32, u32)),
        entry!(0x008707c0, fn_008707c0(Ptr, u32, u8)),
        entry!(0x008709f0, fn_008709f0() -> u8),
        entry!(0x00870a00, fn_00870a00(Ptr, u32, u8)),
        entry!(0x00870bd0, fn_00870bd0(Ptr, u32, u8)),
        entry!(0x00871220, fn_00871220(Ptr) -> bool),
        entry!(0x00871260, fn_00871260() -> bool),
        entry!(0x00871290, fn_00871290(Ptr)),
        entry!(0x00871a10, fn_00871a10() -> u8),
        entry!(0x00871a30, fn_00871a30(Ptr) -> Ptr),
        entry!(0x00871a50, main_update_offscreen_interface(Ptr)),
        entry!(0x00871c90, main_set_active(Ptr, u8)),
        entry!(0x00871d10, fn_00871d10(Ptr, i32, i32) -> bool),
        entry!(0x00871dc0, main_render_menu_background(Ptr)),
        entry!(0x00872370, fn_00872370(Ptr, u32) -> u32),
        entry!(0x008723d0, fn_008723d0(Ptr, u32) -> u32),
        entry!(0x00872430, fn_00872430(Ptr, u32, u32) -> u8),
        entry!(0x00872570, fn_00872570(u8) -> u8),
        entry!(0x00872780, fn_00872780(Ptr) -> u8),
        entry!(0x008727b0, fn_008727b0(Ptr) -> u8),
        entry!(0x008727d0, fn_008727d0(Ptr)),
        entry!(0x00872930, fn_00872930(u8)),
        entry!(0x00872940, fn_00872940(Ptr, u32)),
        entry!(0x00872ad0, fn_00872ad0(Ptr)),
        entry!(0x00872b00, fn_00872b00(Ptr) -> u32),
        entry!(0x00872dd0, fn_00872dd0(u8, u8)),
        entry!(0x00872df0, fn_00872df0(u8)),
        entry!(0x00872e00, fn_00872e00(Ptr) -> Ptr),
        entry!(0x00872ea0, fn_00872ea0(Ptr, u32) -> Ptr),
        entry!(0x00872ed0, ni_culling_process_append_virtual(Ptr, u32)),
        entry!(0x00872ef0, bs_fade_node_culler_get_rtti(Ptr) -> u32),
        entry!(
            0x00872f00,
            bs_fade_node_culler_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x00872f30, fn_00872f30(Ptr)),
        entry!(0x00872f50, fn_00872f50(Ptr, u32, u32, u8, u8) -> u32),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    type Log = Vec<(u32, Vec<u32>)>;
    type Returns = Rc<RefCell<HashMap<u32, Vec<Ret>>>>;

    /// Every address this part calls (functions and, harmlessly, globals):
    /// each gets a test double so that nothing another unit translated runs
    /// for real.
    const CALLEES: &[u32] = &[
        0x0040_1030,
        0x0040_3e20,
        0x0040_4010,
        0x0040_4eb0,
        0x0040_4ee0,
        0x0040_6d30,
        0x0040_6d50,
        0x0040_8d60,
        0x0040_ebd0,
        0x0040_fba0,
        0x0040_fbf0,
        0x0040_fca0,
        0x0041_4430,
        0x0041_fd00,
        0x0043_b230,
        0x0043_c490,
        0x0043_c4b0,
        0x0043_d410,
        0x0043_d450,
        0x0043_d4d0,
        0x0044_0460,
        0x0044_6e10,
        0x0044_a670,
        0x0044_ddc0,
        0x0045_0b80,
        0x0045_0f90,
        0x0045_18e0,
        0x0045_2dc0,
        0x0045_3470,
        0x0045_37b0,
        0x0045_4b30,
        0x0045_6610,
        0x0045_7050,
        0x0045_bb80,
        0x0045_bbe0,
        0x0045_c670,
        0x0045_cd60,
        0x0046_2f40,
        0x0047_1d00,
        0x0047_2380,
        0x0047_6c90,
        0x004a_0370,
        0x004a_03c0,
        0x004a_0ea0,
        0x004a_0eb0,
        0x004a_0f60,
        0x004a_0fd0,
        0x004a_1020,
        0x004a_4040,
        0x004b_7210,
        0x004d_c1f0,
        0x004d_c200,
        0x004d_c310,
        0x004d_e080,
        0x004d_ee10,
        0x004d_ee70,
        0x004e_1bc0,
        0x004e_20b0,
        0x004e_2120,
        0x004e_2190,
        0x004e_3270,
        0x004e_6540,
        0x004e_6560,
        0x004e_6580,
        0x004e_65d0,
        0x004e_6620,
        0x004e_a970,
        0x004e_af60,
        0x004e_bbc0,
        0x0050_0940,
        0x0050_ea90,
        0x0052_4c90,
        0x0052_5420,
        0x0052_99a0,
        0x0052_9c90,
        0x0054_68f0,
        0x0054_77f0,
        0x0054_f4c0,
        0x0055_2570,
        0x0055_8310,
        0x0055_85e0,
        0x0055_9450,
        0x0056_c7f0,
        0x005a_a720,
        0x005b_9b00,
        0x005b_ac20,
        0x005b_f770,
        0x005b_f7b0,
        0x005b_f820,
        0x005d_2860,
        0x005d_2a40,
        0x005d_8b10,
        0x005f_36f0,
        0x005f_9ad0,
        0x005f_9af0,
        0x0063_3c90,
        0x0066_29f0,
        0x0066_b0d0,
        0x0068_3a60,
        0x0070_2360,
        0x0070_2450,
        0x0070_2680,
        0x0070_3080,
        0x0070_5100,
        0x0070_58e0,
        0x0070_5910,
        0x0070_5990,
        0x0070_5a00,
        0x0070_6230,
        0x0070_6320,
        0x0070_6e70,
        0x0070_79b0,
        0x0070_7ad0,
        0x0070_7b30,
        0x0070_94f0,
        0x0070_9ae0,
        0x0070_9af0,
        0x0070_9b00,
        0x0070_9b10,
        0x0070_9b20,
        0x0070_9b30,
        0x0070_9cb0,
        0x0070_ec90,
        0x0071_0ab0,
        0x0071_1e00,
        0x0071_2e60,
        0x0071_48c0,
        0x0071_4960,
        0x0071_4a00,
        0x0071_4c20,
        0x0071_8ab0,
        0x0079_48d0,
        0x007c_9ca0,
        0x007f_a950,
        0x007f_ae70,
        0x007f_df30,
        0x0084_d030,
        0x0084_e3a0,
        0x0086_bdf0,
        0x0086_f640,
        0x0086_f670,
        0x0086_f890,
        0x0086_fd90,
        0x0087_31a0,
        0x0087_31d0,
        0x0087_3200,
        0x0087_46d0,
        0x0087_4920,
        0x0087_49b0,
        0x0087_4ac0,
        0x0087_4b50,
        0x0087_4b90,
        0x0087_4c10,
        0x0087_5110,
        0x0087_5bf0,
        0x0087_5e40,
        0x0087_5fd0,
        0x0087_61e0,
        0x0087_6830,
        0x0087_6850,
        0x0087_6a20,
        0x0087_7430,
        0x0087_a6f0,
        0x0087_a710,
        0x0087_a850,
        0x0087_f200,
        0x008b_6200,
        0x008c_51c0,
        0x008c_80e0,
        0x008d_6f30,
        0x008d_6f50,
        0x008d_80e0,
        0x008d_8520,
        0x0095_0090,
        0x0095_0bb0,
        0x0096_11e0,
        0x0097_9260,
        0x00a0_44f0,
        0x00a0_4500,
        0x00a0_9030,
        0x00a0_d9e0,
        0x00a0_e770,
        0x00a2_3010,
        0x00a2_37b0,
        0x00a2_53d0,
        0x00a5_9c60,
        0x00a5_bdd0,
        0x00a6_93e0,
        0x00a6_9400,
        0x00a6_94a0,
        0x00a6_fdb0,
        0x00a8_1a80,
        0x00aa_5040,
        0x00aa_50a0,
        0x00b4_f540,
        0x00b4_f570,
        0x00b4_f5c0,
        0x00b5_4000,
        0x00b5_40d0,
        0x00b5_afc0,
        0x00b5_b010,
        0x00b5_b060,
        0x00b5_b1f0,
        0x00b5_b880,
        0x00b5_c3c0,
        0x00b5_cbd0,
        0x00b5_cde0,
        0x00b5_d300,
        0x00b5_e870,
        0x00b6_31d0,
        0x00b6_3a90,
        0x00b6_b260,
        0x00b6_b890,
        0x00b6_b8d0,
        0x00b6_bee0,
        0x00b6_c0d0,
        0x00b6_da10,
        0x00b6_e110,
        0x00b6_ee70,
        0x00b8_b1e0,
        0x00b8_b390,
        0x00b9_8480,
        0x00b9_bb10,
        0x00b9_d150,
        0x00b9_dfc0,
        0x00b9_e970,
        0x00b9_fba0,
        0x00ba_0110,
        0x00ba_02b0,
        0x00ba_2f30,
        0x00ba_2fa0,
        0x00ba_30f0,
        0x00ba_3130,
        0x00ba_d4a0,
        0x00c5_2020,
        0x00ec_17c0,
        0x00ec_1800,
        0x00ec_25e0,
        0x00ec_29b0,
        0x00ec_658f,
        0x00ec_9907,
        0x00ec_9a47,
        0x00ec_c104,
        0x00fd_9380,
        0x00fd_f0c8,
        0x00fd_f2a4,
        0x00fd_f2e8,
        0x00fd_f2f8,
        0x00fd_f2fc,
        0x0101_1588,
        0x0101_2060,
        0x0102_1928,
        0x0102_6994,
        0x0103_40a4,
        0x0107_18c0,
        0x0108_29c4,
        0x0108_2cb0,
        0x0108_2cb8,
        0x0108_2cc0,
        0x0108_2ccc,
        0x0108_2d20,
        0x0118_9184,
        0x011a_9608,
        0x011a_9bd0,
        0x011a_d840,
        0x011a_d884,
        0x011c_3ea4,
        0x011c_3f2c,
        0x011c_71dc,
        0x011c_72d8,
        0x011c_72e4,
        0x011c_74d4,
        0x011c_7664,
        0x011c_7810,
        0x011c_7a59,
        0x011c_7adc,
        0x011d_8ba0,
        0x011d_b2cc,
        0x011d_ea0c,
        0x011d_ea10,
        0x011d_ea20,
        0x011d_ea29,
        0x011d_ea3c,
        0x011d_ea8c,
        0x011d_eadc,
        0x011d_eb34,
        0x011d_eb60,
        0x011d_eb7c,
        0x011d_ec64,
        0x011d_ecd8,
        0x011d_ed3c,
        0x011d_ed80,
        0x011d_eda4,
        0x011d_eed8,
        0x011d_ef08,
        0x011d_ef10,
        0x011d_efb4,
        0x011e_0e80,
        0x011f_1180,
        0x011f_2250,
        0x011f_426c,
        0x011f_6394,
        0x011f_91a4,
        0x011f_941e,
        0x011f_9426,
        0x011f_9438,
        0x011f_94a5,
        0x011f_9fc0,
        0x011f_9fc1,
        0x011f_9fc3,
        0x011f_9fc4,
        0x011f_9fc5,
        0x011f_f104,
        0x011f_f8c4,
        0x0120_1f7c,
        0x0126_fac4,
    ];

    /// The engine with a double for every callee, the exe's data pages
    /// mapped (zeroed), and the settings accessors pointing at a zero value.
    struct Rig {
        e: Engine,
        returns: Returns,
        zero: u32,
    }

    impl Rig {
        fn new() -> Rig {
            let mut e = Engine::new();
            for page in (0x0100_0000u32..0x0130_0000).step_by(0x1000) {
                e.map(page, 0x1000);
            }
            let returns: Returns = Rc::new(RefCell::new(HashMap::new()));
            for &addr in CALLEES {
                let table = returns.clone();
                e.register_double(addr, move |_, _| {
                    let mut table = table.borrow_mut();
                    match table.get_mut(&addr) {
                        Some(queue) if queue.len() > 1 => queue.remove(0),
                        Some(queue) if queue.len() == 1 => queue[0],
                        _ => Ret::default(),
                    }
                });
            }
            let zero = e.mem.alloc(0x100);
            let mut rig = Rig { e, returns, zero };
            for setting in [SETTING_DWORD_PTR, SETTING_BYTE_PTR, SETTING_FLOAT_PTR] {
                rig.set(setting, &[zero]);
            }
            // The object `0070ec90` returns is read by `008727d0`.
            let cell = rig.e.mem.alloc(0x40);
            rig.set(0x0070_ec90, &[cell]);
            rig
        }

        /// The values the double at `addr` returns in `eax`, in order; the
        /// last one repeats.
        fn set(&mut self, addr: u32, values: &[u32]) {
            let queue = values
                .iter()
                .map(|&eax| Ret {
                    eax,
                    ..Ret::default()
                })
                .collect();
            self.returns.borrow_mut().insert(addr, queue);
        }

        /// A double that returns `value` in ST0.
        fn set_float(&mut self, addr: u32, value: f64) {
            let ret = Ret {
                st0: value,
                ..Ret::default()
            };
            self.returns.borrow_mut().insert(addr, vec![ret]);
        }

        fn object(&mut self, size: u32) -> u32 {
            self.e.mem.alloc(size)
        }

        /// An object at a fixed address (one mapped page) whose vtable slots
        /// (byte offset, result) are doubles returning the result in `eax`.
        fn object_at(&mut self, object: u32, slots: &[(u32, u32)]) {
            self.e.map(object, 0x1000);
            let vtable = object + 0x100;
            self.e.mem.set_u32(object, vtable);
            for &(offset, result) in slots {
                let target = 0x00f2_0000 + object + offset;
                self.e.register_double(target, move |_, _| Ret {
                    eax: result,
                    ..Ret::default()
                });
                self.e.mem.set_u32(vtable + offset, target);
            }
        }

        /// A settings accessor that answers by the setting it is asked
        /// about (`values`: setting address, value); any other setting reads 0.
        fn settings(&mut self, accessor: u32, values: &[(u32, u32)]) {
            let zero = self.zero;
            let mut blocks: HashMap<u32, u32> = HashMap::new();
            for &(setting, value) in values {
                let block = self.e.mem.alloc(8);
                self.e.mem.set_u32(block, value);
                blocks.insert(setting, block);
            }
            self.e.register_double(accessor, move |_, a| Ret {
                eax: blocks.get(&a[0]).copied().unwrap_or(zero),
                ..Ret::default()
            });
        }

        /// `NiPointer` reads that answer by the slot asked about (`slots`:
        /// slot address, pointer); any other slot is empty.
        fn pointers(&mut self, slots: &[(u32, u32)]) {
            let table: HashMap<u32, u32> = slots.iter().copied().collect();
            self.e.register_double(POINTER_GET, move |_, a| Ret {
                eax: table.get(&a[0]).copied().unwrap_or(0),
                ..Ret::default()
            });
        }

        /// An object whose vtable slots (byte offset, result) are doubles
        /// returning the given result in `eax`.
        fn object_with_slots(&mut self, slots: &[(u32, u32)]) -> u32 {
            let object = self.e.mem.alloc(0x40);
            let vtable = self.e.mem.alloc(0x400);
            self.e.mem.set_u32(object, vtable);
            for &(offset, result) in slots {
                let target = 0x00f0_0000 + (vtable & 0xffff) * 0x400 + offset;
                self.e.register_double(target, move |_, _| Ret {
                    eax: result,
                    ..Ret::default()
                });
                self.e.mem.set_u32(vtable + offset, target);
            }
            object
        }

        /// Makes the settings accessor `accessor` hand out, call by call,
        /// the addresses of blocks holding `values`.
        fn setting(&mut self, accessor: u32, values: &[u32]) {
            let blocks: Vec<u32> = values
                .iter()
                .map(|&value| {
                    let block = self.e.mem.alloc(8);
                    self.e.mem.set_u32(block, value);
                    block
                })
                .collect();
            self.set(accessor, &blocks);
        }

        /// Runs `addr` and returns its result and the calls it made.
        fn run(&mut self, addr: u32, args: &[u32]) -> (Ret, Log) {
            self.e.call_log = Some(vec![]);
            let ret = self.e.call(addr, args);
            let mut log = self.e.call_log.take().unwrap();
            log.remove(0);
            (ret, log)
        }
    }

    fn addrs(log: &Log) -> Vec<u32> {
        log.iter().map(|(addr, _)| *addr).collect()
    }

    /// The addresses of `log` that are in `wanted`, in call order.
    fn only(log: &Log, wanted: &[u32]) -> Vec<u32> {
        addrs(log)
            .into_iter()
            .filter(|addr| wanted.contains(addr))
            .collect()
    }

    /// The argument lists of the calls to `addr`.
    fn calls_to(log: &Log, addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    #[test]
    fn flag_getters_read_their_globals() {
        let mut r = Rig::new();
        r.e.set_global(0x0118_9184, 7u8);
        r.e.set_global(0x011f_91a4, 1u8);
        assert_eq!(r.e.call(0x008705c0, &[]).u8(), 7);
        assert_eq!(r.e.call(0x008709f0, &[]).u8(), 1);
    }

    #[test]
    fn idle_step_runs_the_cell_update_only_with_several_threads() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.setting(SETTING_DWORD_PTR, &[2]);
        let (_, log) = r.run(0x008705d0, &args![this]);
        assert_eq!(
            addrs(&log),
            vec![
                0x0086_f640,
                0x0086_f890,
                SETTING_DWORD_PTR,
                0x0055_2570,
                0x0086_f670
            ]
        );
        assert_eq!(calls_to(&log, 0x0086_f640), vec![vec![this]]);
        r.setting(SETTING_DWORD_PTR, &[1]);
        let (_, log) = r.run(0x008705d0, &args![this]);
        assert_eq!(
            addrs(&log),
            vec![0x0086_f640, 0x0086_f890, SETTING_DWORD_PTR, 0x0086_f670]
        );
    }

    #[test]
    fn delayed_actions_step_closes_support_menus_and_prints() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        let interface = r.object(0x600);
        r.e.mem.set_u8(interface + 0x4cc, 1);
        r.set(GET_GLOBAL_011D8A80, &[interface]);
        r.set(GET_GLOBAL_011DF1A8, &[0x1234]);
        r.set(0x00a0_d9e0, &[0x77]);
        r.setting(SETTING_DWORD_PTR, &[2]);
        let (_, log) = r.run(0x00870610, &args![this]);
        assert_eq!(
            addrs(&log),
            vec![
                0x005a_a720,
                SETTING_DWORD_PTR,
                GET_GLOBAL_011DF1A8,
                0x0087_a710,
                GET_GLOBAL_011DF1A8,
                0x0087_a850,
                GET_GLOBAL_011DF1A8,
                0x0087_a6f0,
                0x0070_94f0,
                GET_GLOBAL_011D8A80,
                0x0070_6320,
                0x00a0_d9e0,
                0x00a0_e770,
            ]
        );
        assert_eq!(calls_to(&log, 0x0087_a850), vec![vec![0x1234]]);
        assert_eq!(calls_to(&log, 0x0070_6320), vec![vec![0]]);
        assert_eq!(calls_to(&log, 0x00a0_d9e0), vec![vec![1]]);
        assert_eq!(calls_to(&log, 0x00a0_e770), vec![vec![0x77]]);
        assert_eq!(r.e.mem.u8(interface + 0x4cc), 0);

        // One thread: the three object methods are skipped (and the flag
        // is already cleared, so no menu is closed).
        r.setting(SETTING_DWORD_PTR, &[1]);
        let (_, log) = r.run(0x00870610, &args![this]);
        assert!(!addrs(&log).contains(&0x0087_a710));
        assert!(!addrs(&log).contains(&0x0070_6320));
    }

    #[test]
    fn support_menus_close_only_when_flagged() {
        let mut r = Rig::new();
        let this = r.object(0x600);
        let (_, log) = r.run(0x00870680, &args![this]);
        assert!(log.is_empty());
        r.e.mem.set_u8(this + 0x4cc, 1);
        let (_, log) = r.run(0x00870680, &args![this]);
        assert_eq!(log, vec![(0x0070_6320, vec![0])]);
        assert_eq!(r.e.mem.u8(this + 0x4cc), 0);
    }

    #[test]
    fn byte_and_position_accessors() {
        let mut r = Rig::new();
        let object = r.object(0x200);
        r.e.mem.set_u8(object + 0x131, 9);
        assert_eq!(r.e.call(0x00870770, &args![object]).u8(), 9);
        r.e.call(0x00870790, &args![object, 1u32, 2u32, 3u32]);
        assert_eq!(r.e.mem.u32(object + 0x1f0), 1);
        assert_eq!(r.e.mem.u32(object + 0x1f4), 2);
        assert_eq!(r.e.mem.u32(object + 0x1f8), 3);
    }

    /// A rig for the draw sequences: a `Main` object, distinct fake objects
    /// from the getters, and the indexed global 0 with the byte `+0x131`.
    fn frame_rig(byte_131: u8) -> (Rig, u32, u32) {
        let mut r = Rig::new();
        let this = r.object(0x200);
        let indexed = r.object(0x200);
        r.e.mem.set_u8(indexed + 0x131, byte_131);
        r.set(INDEXED_GLOBAL, &[indexed]);
        r.set(GET_GLOBAL_011F4748, &[0x1000]);
        // The renderer-like object: slot 0xc8 returns a group, 0xac a value.
        r.object_at(0x1000, &[(0xc8, 0x6c00), (0xac, 0)]);
        r.set(GET_GLOBAL_011F91AC, &[0x2000]);
        r.set(GET_ROOT, &[0x3000]);
        r.set(GET_ROOT_CHILD, &[0x4000]);
        r.set(GET_ROOT_CHILD_OF_ROOT, &[0x4000]);
        let water_group = r.object(0x100);
        r.set(0x004e_bbc0, &[water_group]);
        let position = r.object(16);
        for (i, word) in [10u32, 20, 30].iter().enumerate() {
            r.e.mem.set_u32(position + 4 * i as u32, *word);
        }
        r.set(MEMBER_8C, &[position]);
        (r, this, indexed)
    }

    #[test]
    fn frame_selector_stores_the_camera_position_and_picks_a_draw() {
        // Byte clear: the plain world frame (the unique `00874ac0` call).
        let (mut r, this, indexed) = frame_rig(0);
        let (_, log) = r.run(0x008706b0, &args![this, 0x5000u32, 0u32, 0u32]);
        assert_eq!(r.e.mem.u32(indexed + 0x1f0), 10);
        assert_eq!(r.e.mem.u32(indexed + 0x1f4), 20);
        assert_eq!(r.e.mem.u32(indexed + 0x1f8), 30);
        assert!(addrs(&log).contains(&0x0087_4ac0));
        assert!(!addrs(&log).contains(&0x00b9_8480));

        // Byte set: the variant with the `00b98480` call.
        let (mut r, this, _) = frame_rig(1);
        let (_, log) = r.run(0x008706b0, &args![this, 0x5000u32, 0u32, 0u32]);
        assert!(addrs(&log).contains(&0x00b9_8480));
        assert!(!addrs(&log).contains(&0x0087_4ac0));

        // Menu background requested and menu 0x3f5 not visible.
        let (mut r, this, _) = frame_rig(1);
        let (_, log) = r.run(0x008706b0, &args![this, 0x5000u32, 1u32, 0u32]);
        assert_eq!(calls_to(&log, IS_MENU_ID_VISIBLE)[0], vec![0x3f5, 0]);
        assert!(addrs(&log).contains(&0x0087_5bf0));
        assert!(!addrs(&log).contains(&0x00b9_8480));

        // Menu background requested but 0x3f5 visible: the byte decides.
        let (mut r, this, _) = frame_rig(1);
        r.set(IS_MENU_ID_VISIBLE, &[1]);
        let (_, log) = r.run(0x008706b0, &args![this, 0x5000u32, 1u32, 0u32]);
        assert!(addrs(&log).contains(&0x00b9_8480));
    }

    /// The stage argument of every call to a `SetThreadStage`-style address.
    fn stages(log: &Log, addr: u32) -> Vec<u32> {
        calls_to(log, addr).iter().map(|a| a[2]).collect()
    }

    #[test]
    fn menu_background_frame_resets_the_stages_and_draws_the_menu_pass() {
        let (mut r, this, _) = frame_rig(0);
        r.set(GET_MT_SYSTEM, &[0x0120_0088]);
        r.set(0x004d_c310, &[1]);
        let (_, log) = r.run(0x008707c0, &args![this, 0x5000u32, 0u32]);
        let expected: Vec<u32> = (1..=0x16).collect();
        assert_eq!(stages(&log, MT_SET_THREAD_STAGE), expected);
        assert_eq!(stages(&log, MT_ADVANCE_THREAD_STAGE), expected);
        let first = calls_to(&log, MT_SET_THREAD_STAGE)[0].clone();
        assert_eq!(first, vec![0x0120_0088, 0, 1]);
        let first = calls_to(&log, MT_ADVANCE_THREAD_STAGE)[0].clone();
        assert_eq!(first, vec![0x0120_0088, 1, 1]);
        // Not the first-person frame (its `8709f0` flag is clear): the
        // target is cleared with `00876830`.
        assert_eq!(
            calls_to(&log, 0x0087_4b90),
            vec![vec![this, 0x2000, 0x1000, 1, 0x5000]]
        );
        assert_eq!(calls_to(&log, 0x0087_6830), vec![vec![this, 1, 0x5000]]);
        assert_eq!(
            calls_to(&log, 0x0087_6850),
            vec![vec![this, 0, 0x2000, 0x1000, 0]]
        );
        assert!(!addrs(&log).contains(&0x0087_5110));
        // The byte flag is set and a target is given: the renderer's own
        // group (vtable slot 0xc8) is made current.
        assert_eq!(calls_to(&log, 0x00b6_b8d0), vec![vec![7, 0x6c00]]);
    }

    #[test]
    fn menu_background_frame_in_the_pipboy_draws_the_conversation_target() {
        let (mut r, this, indexed) = frame_rig(0);
        r.set(IS_IN_PIPBOY_MENU, &[1]);
        r.set(GET_CURRENT_RENDERED_MENU, &[0x6000]);
        r.set(0x007f_a950, &[0x7000]);
        r.set(0x008d_80e0, &[0x8000]);
        r.pointers(&[(this + MAIN_FIRST_PERSON_ACCUM, 0x9000)]);
        let (_, log) = r.run(0x008707c0, &args![this, 0u32, 0u32]);
        assert_eq!(calls_to(&log, 0x004a_0fd0), vec![vec![0x8000, 0x9000]]);
        assert_eq!(calls_to(&log, 0x0041_fd00), vec![vec![0x8000, 0x4000]]);
        assert_eq!(calls_to(&log, 0x00b5_e870), vec![vec![indexed, 0x8000]]);
        assert_eq!(
            calls_to(&log, 0x0087_5110),
            vec![vec![this, 0x1000, 0x7000, 0, 0]]
        );
        // Stage 0x16 is not advanced in the pipboy.
        assert!(!stages(&log, MT_SET_THREAD_STAGE).contains(&0x16));
    }

    #[test]
    fn menu_background_frame_with_the_overlay_flag_draws_the_rendered_menu() {
        let (mut r, this, _) = frame_rig(0);
        r.e.set_global(0x011f_91a4, 1u8);
        r.set(GET_CURRENT_RENDERED_MENU, &[0x6000]);
        r.set(0x007f_a950, &[0x7000]);
        r.set(0x00b6_e110, &[0xf000]);
        r.set(0x004d_c310, &[1]);
        r.setting(SETTING_DWORD_PTR, &[2]);
        let (_, log) = r.run(0x008707c0, &args![this, 0u32, 0u32]);
        // The multithread setting is above 1.
        assert_eq!(calls_to(&log, 0x008c_80e0), vec![vec![0]]);
        assert_eq!(
            calls_to(&log, 0x0087_5fd0),
            vec![vec![this, 1, 0xf000, 0x1000, 0]]
        );
        assert_eq!(
            calls_to(&log, 0x0087_61e0),
            vec![vec![this, 0x7000, 0x1000]]
        );
        assert_eq!(
            calls_to(&log, 0x0087_6850),
            vec![vec![this, 0xf000, 0x2000, 0x1000, 0]]
        );

        // With the byte `+0x131` set (the flag argument), the target is
        // not created and `00876830` runs for a non-zero target instead.
        let (mut r, this, _) = frame_rig(0);
        r.e.set_global(0x011f_91a4, 1u8);
        let (_, log) = r.run(0x008707c0, &args![this, 0x5000u32, 1u32]);
        assert!(!addrs(&log).contains(&0x0087_5fd0));
        assert_eq!(calls_to(&log, 0x0087_6830), vec![vec![this, 0, 0x5000]]);
    }

    #[test]
    fn world_frame_with_depth_pass_draws_the_world_when_allowed() {
        let (mut r, this, _) = frame_rig(1);
        r.set(0x0087_49b0, &[5]);
        r.set(0x004d_c310, &[1]);
        let (_, log) = r.run(0x00870a00, &args![this, 0x5000u32, 0u32]);
        assert_eq!(calls_to(&log, 0x0087_3200), vec![vec![this, 0, 1, 1, 5]]);
        assert_eq!(
            only(&log, &[0x0087_4c10, 0x0087_4b50, 0x0087_5110, 0x0087_6830]),
            vec![
                0x0087_4c10,
                0x0087_4b50,
                0x0087_5110,
                0x0087_6830,
                0x0087_6830
            ]
        );
        assert_eq!(calls_to(&log, 0x0087_4b50), vec![vec![this, 0, 1, 0]]);
        assert_eq!(
            calls_to(&log, 0x0087_5110),
            vec![vec![this, 0x1000, 0, 0, 0]]
        );
        // Once before and once after the overlay test (flag clear).
        assert_eq!(
            calls_to(&log, 0x0087_6830),
            vec![vec![this, 1, 0x5000], vec![this, 1, 0x5000]]
        );
        assert_eq!(calls_to(&log, 0x0071_4c20), vec![vec![1]]);
        assert_eq!(calls_to(&log, 0x00b9_8480), vec![vec![1, 1]]);
        assert_eq!(
            calls_to(&log, 0x0087_6850),
            vec![vec![this, 0, 0x2000, 0x1000, 0]]
        );
    }

    #[test]
    fn world_frame_with_depth_pass_skips_the_world_for_each_reason() {
        // The skip argument.
        let (mut r, this, _) = frame_rig(1);
        let (_, log) = r.run(0x00870a00, &args![this, 0u32, 1u32]);
        assert_eq!(calls_to(&log, 0x0087_3200), vec![vec![this, 0, 0, 1, 0]]);
        assert!(!addrs(&log).contains(&0x0087_5110));
        let set: Vec<u32> = (1..=0x14).chain([0x16]).collect();
        assert_eq!(stages(&log, MT_SET_THREAD_STAGE), set);
        // The flag of the player's child: the other tests are not asked.
        let (mut r, this, _) = frame_rig(1);
        r.set(0x0045_6610, &[1]);
        let (_, log) = r.run(0x00870a00, &args![this, 0u32, 0u32]);
        assert!(!addrs(&log).contains(&0x0055_85e0));
        assert!(!addrs(&log).contains(&0x0087_5110));
        // `005585e0` of the interface object.
        let (mut r, this, _) = frame_rig(1);
        r.set(0x0070_5910, &[0x77]);
        r.set(0x0055_85e0, &[1]);
        let (_, log) = r.run(0x00870a00, &args![this, 0u32, 0u32]);
        assert_eq!(calls_to(&log, 0x0055_85e0), vec![vec![0x77]]);
        assert!(!addrs(&log).contains(&0x0070_5100));
        assert!(!addrs(&log).contains(&0x0087_5110));
        // `00705100`.
        let (mut r, this, _) = frame_rig(1);
        r.set(0x0070_5100, &[1]);
        let (_, log) = r.run(0x00870a00, &args![this, 0u32, 0u32]);
        assert!(!addrs(&log).contains(&0x0087_5110));
    }

    #[test]
    fn world_frame_with_depth_pass_uses_the_overlay_draw_when_flagged() {
        let (mut r, this, _) = frame_rig(1);
        r.e.set_global(0x011f_91a4, 1u8);
        r.set(0x004d_c310, &[1]);
        let (_, log) = r.run(0x00870a00, &args![this, 0x5000u32, 0u32]);
        // The first `8709f0` also makes `00872f50` create a target.
        assert_eq!(calls_to(&log, 0x0087_5fd0).len(), 1);
        assert_eq!(calls_to(&log, 0x0087_5fd0)[0][..3], [this, 1, 0]);
        assert_eq!(calls_to(&log, 0x0087_6830).len(), 1);
    }

    #[test]
    fn plain_world_frame_orders_its_passes() {
        let (mut r, this, _) = frame_rig(0);
        r.set(0x0087_49b0, &[5]);
        r.set(0x004d_c310, &[1]);
        let (_, log) = r.run(0x00870bd0, &args![this, 0x5000u32, 0u32]);
        assert_eq!(calls_to(&log, 0x0087_4ac0), vec![vec![this, 5]]);
        assert_eq!(
            calls_to(&log, 0x0087_4b50),
            vec![vec![this, 0, 1, 5], vec![this, 0, 1, 0]]
        );
        assert_eq!(calls_to(&log, 0x0087_3200), vec![vec![this, 0, 1, 0, 5]]);
        assert_eq!(
            calls_to(&log, 0x0087_5110),
            vec![vec![this, 0x1000, 0, 0, 0]]
        );
        // No water pass: the second thread's stage 0x16 is advanced.
        let advanced: Vec<u32> = (1..=0x14).chain([0x16]).collect();
        assert_eq!(stages(&log, MT_ADVANCE_THREAD_STAGE), advanced);
        assert_eq!(
            calls_to(&log, 0x0087_6850),
            vec![vec![this, 0, 0x2000, 0x1000, 0]]
        );
        assert_eq!(calls_to(&log, 0x0087_6830), vec![vec![this, 1, 0x5000]]);
    }

    #[test]
    fn plain_world_frame_saves_and_restores_the_accumulator_flag() {
        let (mut r, this, _) = frame_rig(0);
        r.set(0x0043_b230, &[0x7777]);
        // `00872b00` returns the object `0045cd60` gives for the world's
        // `008d8520`, when `00476c90` is 2 or 3.
        r.set(0x008d_8520, &[0x6000]);
        r.set(0x0045_cd60, &[0x6100]);
        r.set(0x0047_6c90, &[3]);
        let (_, log) = r.run(0x00870bd0, &args![this, 0u32, 0u32]);
        assert_eq!(calls_to(&log, 0x0087_3200)[0][1], 0x6100);
        // Forced on, then restored to the flag that was read (0).
        assert_eq!(
            calls_to(&log, 0x0045_0f90),
            vec![vec![0x7777, 1], vec![0x7777, 0]]
        );
    }

    #[test]
    fn plain_world_frame_letterboxes_with_the_scaled_width() {
        let (mut r, this, _) = frame_rig(0);
        r.set(0x0070_9cb0, &[1]);
        r.set(0x004d_ee10, &[1000]);
        r.set(0x004d_ee70, &[500]);
        r.set_float(0x0070_6e70, 1.0);
        // The aspect setting (a float) and the scale constant.
        let aspect_block = r.object(8);
        r.e.mem.set_f32(aspect_block, 1.0);
        r.set(SETTING_FLOAT_PTR, &[aspect_block]);
        r.e.set_global(0x0101_1588, 0.5f64);
        let (_, log) = r.run(0x00870bd0, &args![this, 0u32, 0u32]);
        assert_eq!(calls_to(&log, 0x00b4_f540), vec![vec![500, 0, 500, 500]]);
        assert_eq!(calls_to(&log, 0x00b5_40d0).len(), 1);
    }

    #[test]
    fn plain_world_frame_letterbox_is_skipped_when_disabled() {
        let (mut r, this, _) = frame_rig(0);
        r.set(0x0070_9cb0, &[1]);
        r.set(0x004d_e080, &[1]);
        r.set(0x004d_ee10, &[1000]);
        let renderer = r.object_with_slots(&[(0xac, 0)]);
        r.set(GET_GLOBAL_011F4748, &[renderer]);
        let (_, log) = r.run(0x00870bd0, &args![this, 0u32, 0u32]);
        assert!(!addrs(&log).contains(&0x00b4_f540));
        assert!(!addrs(&log).contains(&0x00b5_40d0));
    }

    #[test]
    fn plain_world_frame_resets_the_clear_colors_after_the_letterbox() {
        let (mut r, this, _) = frame_rig(0);
        r.set(0x0070_9cb0, &[1]);
        r.set(0x004d_e080, &[1]);
        r.e.set_global(0x0102_6994, 0.25f32);
        r.e.set_global(0x0103_40a4, 0.75f32);
        let renderer = r.object_with_slots(&[(0xac, 0)]);
        r.set(GET_GLOBAL_011F4748, &[renderer]);
        let colors: Rc<RefCell<Vec<[f32; 4]>>> = Rc::new(RefCell::new(vec![]));
        {
            let colors = colors.clone();
            r.e.register_double(0x0071_48c0, move |e, a| {
                let at = a[1];
                colors.borrow_mut().push([
                    e.mem.f32(at),
                    e.mem.f32(at + 4),
                    e.mem.f32(at + 8),
                    e.mem.f32(at + 12),
                ]);
                Ret::default()
            });
        }
        let (_, log) = r.run(0x00870bd0, &args![this, 0u32, 0u32]);
        assert_eq!(
            *colors.borrow(),
            vec![[0.0, 0.25, 1.0, 0.0], [0.75, 1.0, 1.0, 0.0]]
        );
        // The color is built with zeros first; both are set on the renderer.
        assert_eq!(calls_to(&log, 0x0041_4430)[0][1..], [0, 0, 0, 0]);
        assert_eq!(calls_to(&log, 0x0071_48c0).len(), 2);
        assert_eq!(calls_to(&log, 0x0071_48c0)[0][0], renderer);
        assert_eq!(calls_to(&log, 0x0071_48c0)[0][2], 1);
    }

    #[test]
    fn plain_world_frame_draws_the_water_pass_when_the_level_is_high_enough() {
        let (mut r, this, _) = frame_rig(0);
        let water = r.object(0x100);
        r.e.mem.set_f32(water + 0x70, 2.0);
        r.e.set_global(0x0101_2060, 1.0f64);
        r.set(0x004e_bbc0, &[water]);
        r.pointers(&[(this + MAIN_WORLD_ACCUM, 0x9000)]);
        r.set(0x005b_9b00, &[1]);
        r.set(0x00b6_3a90, &[3]);
        let (_, log) = r.run(0x00870bd0, &args![this, 0u32, 0u32]);
        assert_eq!(
            only(&log, &[0x0087_46d0, 0x0087_4920, 0x0087_4c10, 0x0087_5e40]),
            vec![0x0087_46d0, 0x0087_4920, 0x0087_4c10, 0x0087_5e40]
        );
        assert_eq!(calls_to(&log, 0x0087_46d0), vec![vec![this, 0]]);
        assert_eq!(calls_to(&log, 0x0087_4920), vec![vec![this, 0x9000, 0]]);
        assert_eq!(calls_to(&log, 0x0087_5e40), vec![vec![this, 0x9000, 0]]);
        // The water pass already advanced the second thread: no advance.
        assert!(!stages(&log, MT_ADVANCE_THREAD_STAGE).contains(&0x16));
    }

    #[test]
    fn plain_world_frame_projects_the_picked_reference() {
        let (mut r, this, _) = frame_rig(0);
        r.set(0x00b8_b390, &[1]);
        r.settings(SETTING_BYTE_PTR, &[(0x011d_eadc, 1)]);
        r.e.set_global(0x011d_ea20, 0x5555u32);
        let node = r.object(0x40);
        let reference = r.object_with_slots(&[(4, node)]);
        r.set(0x0045_cd60, &[reference]);
        let pair: Rc<RefCell<Vec<f32>>> = Rc::new(RefCell::new(vec![]));
        r.e.register_double(0x00a6_fdb0, |e, a| {
            // The outputs are the third and fourth arguments.
            e.mem.set_f32(a[2], 2.5);
            e.mem.set_f32(a[3], 3.5);
            Ret::default()
        });
        {
            let pair = pair.clone();
            r.e.register_double(POINT2_CTOR, move |_, a| {
                pair.borrow_mut()
                    .extend([f32::from_bits(a[1]), f32::from_bits(a[2])]);
                Ret {
                    eax: a[0],
                    ..Ret::default()
                }
            });
        }
        let (_, log) = r.run(0x00870bd0, &args![this, 0u32, 0u32]);
        assert_eq!(*pair.borrow(), vec![2.5, 3.5]);
        let projection = calls_to(&log, 0x00a6_fdb0);
        assert_eq!(projection.len(), 1);
        assert_eq!(projection[0][0], 0x4000);
        let handed = calls_to(&log, 0x00b8_b1e0);
        assert_eq!(handed.len(), 1);
        assert_eq!(handed[0][0], 0x2000);
    }

    #[test]
    fn plain_world_frame_draws_the_overlays_when_flagged() {
        let (mut r, this, _) = frame_rig(0);
        r.e.set_global(0x011f_91a4, 1u8);
        r.set(0x0045_3470, &[1]);
        r.pointers(&[(0x011c_7810, 0x5000)]);
        let (_, log) = r.run(0x00870bd0, &args![this, 0x6000u32, 0u32]);
        assert_eq!(calls_to(&log, 0x0087_5fd0).len(), 1);
        assert_eq!(calls_to(&log, 0x0087_6a20), vec![vec![this, 0x1000]]);
        assert!(!addrs(&log).contains(&0x0087_6830));
        // In the pipboy the extra pass is skipped.
        let (mut r, this, _) = frame_rig(0);
        r.e.set_global(0x011f_91a4, 1u8);
        r.set(0x0045_3470, &[1]);
        r.pointers(&[(0x011c_7810, 0x5000)]);
        r.set(IS_IN_PIPBOY_MENU, &[1]);
        let (_, log) = r.run(0x00870bd0, &args![this, 0x6000u32, 0u32]);
        assert!(!addrs(&log).contains(&0x0087_6a20));
    }

    #[test]
    fn water_level_compares_against_the_double() {
        let mut r = Rig::new();
        let object = r.object(0x100);
        r.e.set_global(0x0101_2060, 1.0f64);
        r.e.mem.set_f32(object + 0x70, 2.0);
        assert!(r.e.call(0x0087_1220, &args![object]).bool());
        r.e.mem.set_f32(object + 0x70, 1.0);
        assert!(!r.e.call(0x0087_1220, &args![object]).bool());
        r.e.mem.set_f32(object + 0x70, f32::NAN);
        assert!(!r.e.call(0x0087_1220, &args![object]).bool());
    }

    #[test]
    fn pointer_test_needs_an_object_with_a_non_zero_answer() {
        let mut r = Rig::new();
        assert!(!r.e.call(0x0087_1260, &args![]).bool());
        r.pointers(&[(0x011c_7810, 0x5000)]);
        assert!(!r.e.call(0x0087_1260, &args![]).bool());
        r.set(0x0045_3470, &[2]);
        let (ret, log) = r.run(0x0087_1260, &args![]);
        assert!(ret.bool());
        assert_eq!(calls_to(&log, 0x0045_3470), vec![vec![0x5000]]);
    }

    /// A float's bit pattern as an argument word.
    fn word(value: f32) -> u32 {
        value.to_bits()
    }

    /// The address a vtable slot of `object` points at.
    fn slot_target(r: &Rig, object: u32, offset: u32) -> u32 {
        let vtable = r.e.mem.u32(object);
        r.e.mem.u32(vtable + offset)
    }

    #[test]
    fn shadow_preparation_starts_the_special_state() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.set(0x005b_f7b0, &[1]);
        r.set(0x0052_5420, &[1]);
        r.set(GET_GLOBAL_011F4748, &[0x1111]);
        r.set(GET_GLOBAL_011F91A8, &[0x2222]);
        r.set(INDEXED_GLOBAL, &[0x3333]);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(r.e.global::<u8>(0x011d_ef08), 1);
        assert_eq!(calls_to(&log, 0x005b_f770), vec![vec![0]]);
        assert_eq!(calls_to(&log, 0x00b5_b1f0), vec![vec![0x3333]]);
        assert_eq!(calls_to(&log, 0x00b6_ee70), vec![vec![0x2222, 0x1111, 0]]);
        // The shadow setting byte is clear: the stages 0x11 to 0x14 advance.
        assert_eq!(
            stages(&log, MT_SET_THREAD_STAGE),
            vec![0x11, 0x12, 0x13, 0x14]
        );
        assert_eq!(
            stages(&log, MT_ADVANCE_THREAD_STAGE),
            vec![0x11, 0x12, 0x13, 0x14]
        );
        // It ends with the offscreen-interface update.
        assert_eq!(*addrs(&log).last().unwrap(), IS_IN_MENU_MODE);
    }

    #[test]
    fn shadow_preparation_ends_the_special_state_with_the_caster_limit() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.e.set_global(0x011d_ef08, 1u8);
        r.set(GET_GLOBAL_011F4748, &[0x1111]);
        r.set(GET_GLOBAL_011F91A8, &[0x2222]);
        r.settings(SETTING_DWORD_PTR, &[(0x011c_72e4, 7), (0x011c_72d8, 9)]);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(r.e.global::<u8>(0x011d_ef08), 0);
        assert_eq!(calls_to(&log, 0x005b_f770), vec![vec![1]]);
        assert_eq!(calls_to(&log, 0x00b6_ee70), vec![vec![0x2222, 0x1111, 7]]);

        // With `005f36f0` non-zero the other setting is the limit.
        r.e.set_global(0x011d_ef08, 1u8);
        r.set(0x005f_36f0, &[1]);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(calls_to(&log, 0x00b6_ee70), vec![vec![0x2222, 0x1111, 9]]);

        // The state is not ended while `00525420` still holds.
        r.e.set_global(0x011d_ef08, 1u8);
        r.set(0x0052_5420, &[1]);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert!(!addrs(&log).contains(&0x005b_f770));
        assert_eq!(r.e.global::<u8>(0x011d_ef08), 1);
    }

    #[test]
    fn shadow_preparation_with_no_casters_only_resets_the_scene() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.set(0x005b_f7b0, &[1]);
        r.set(INDEXED_GLOBAL, &[0x3333]);
        r.set(GET_GLOBAL_011F4748, &[0x1111]);
        r.set(GET_GLOBAL_011F91A8, &[0x2222]);
        r.settings(SETTING_BYTE_PTR, &[(0x011c_74d4, 1)]);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(
            only(
                &log,
                &[
                    0x00b4_f570,
                    0x00b5_b060,
                    0x00b6_ee70,
                    0x00b5_c3c0,
                    0x00b5_cbd0
                ]
            ),
            vec![0x00b4_f570, 0x00b5_b060, 0x00b6_ee70, 0x00b5_c3c0]
        );
        assert_eq!(calls_to(&log, 0x00b5_b060), vec![vec![0x3333]]);
        assert_eq!(calls_to(&log, 0x00b5_c3c0), vec![vec![0x3333]]);
        // The stage loop of the other branch did not run.
        assert!(stages(&log, MT_SET_THREAD_STAGE).is_empty());
    }

    /// The rig for the caster pick: scene `0x3333`, the player object, one
    /// actor in the list at `0x011e0f08`, limit `limit`.
    fn caster_rig(limit: u32) -> (Rig, u32, u32) {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.set(0x005b_f7b0, &[1]);
        r.set(INDEXED_GLOBAL, &[0x3333]);
        r.set(GET_GLOBAL_011F4748, &[0x1111]);
        r.set(GET_GLOBAL_011F91A8, &[0x2222]);
        r.settings(SETTING_BYTE_PTR, &[(0x011c_74d4, 1)]);
        r.settings(SETTING_DWORD_PTR, &[(0x011c_72e4, limit)]);
        r.e.set_global(PLAYER_OBJECT, 0x5100u32);
        r.set(0x004e_af60, &[1]);
        r.set(0x0095_0bb0, &[0x6000, 0]);
        r.set(0x00b5_cbd0, &[0x6100]);
        let actor = r.object_with_slots(&[(0x1d0, 0x7000), (0x214, 1)]);
        r.e.mem.set_u32(0x011e_0f08, actor);
        r.set(0x0047_2380, &[1]);
        r.set(0x0096_11e0, &[1]);
        r.set(0x0087_f200, &[1]);
        r.set(0x0050_ea90, &[0x7001]);
        r.set(0x0043_d450, &[0x7002]);
        r.set_float(0x0084_d030, -1.0);
        (r, this, actor)
    }

    #[test]
    fn casters_are_the_player_then_the_actors_that_pass_the_checks() {
        let (mut r, this, _) = caster_rig(2);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(
            calls_to(&log, 0x00b5_cbd0),
            vec![vec![0x3333, 0x6000], vec![0x3333, 0x7000]]
        );
        assert_eq!(calls_to(&log, 0x0040_fbf0), vec![vec![0x011f_1180, 0]]);
        assert_eq!(calls_to(&log, 0x0040_fba0), vec![vec![0x011f_1180]]);
        // The extra-data key and process were asked for the radius test.
        assert_eq!(calls_to(&log, 0x00a5_bdd0), vec![vec![0x7000, 0x7001]]);
        assert_eq!(calls_to(&log, 0x0043_d450), vec![vec![0x7000]]);
        // The player's first caster is handed back to the scene.
        assert_eq!(calls_to(&log, 0x00b5_cde0), vec![vec![0x3333, 0x6100]]);
        // The player's child is gone the second time: the walk is skipped.
        assert!(!addrs(&log).contains(&0x00b5_afc0));
    }

    #[test]
    fn casters_stop_at_the_limit() {
        let (mut r, this, _) = caster_rig(1);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(calls_to(&log, 0x00b5_cbd0), vec![vec![0x3333, 0x6000]]);
        assert_eq!(calls_to(&log, 0x0096_11e0).len(), 0);
    }

    #[test]
    fn actors_that_fail_a_check_are_not_casters() {
        // Not loaded, no 3D data, dead (mode 4), refractive, too far.
        for (case, addr, value) in [
            (0, 0x0096_11e0u32, 0u32),
            (1, 0x0087_f200, 0),
            (3, 0x008c_51c0, 1),
        ] {
            let (mut r, this, _) = caster_rig(2);
            r.set(addr, &[value]);
            if case == 3 {
                // The player check also asks `008c51c0`; only the actor's
                // answer is changed.
                r.set(addr, &[0, 1]);
            }
            let (_, log) = r.run(0x0087_1290, &args![this]);
            assert_eq!(calls_to(&log, 0x00b5_cbd0).len(), 1, "case {case}");
        }
        let (mut r, this, actor) = caster_rig(2);
        let mode_slot = slot_target(&r, actor, 0x214);
        r.e.register_double(mode_slot, |_, _| Ret {
            eax: 4,
            ..Ret::default()
        });
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(calls_to(&log, 0x00b5_cbd0).len(), 1);
        // Too far: the radius is not below the scaled position.
        let (mut r, this, _) = caster_rig(2);
        r.set_float(0x0084_d030, 5.0);
        r.set_float(FLOAT_HELPER_A, 1.0);
        r.e.set_global(0x0102_1928, 3.0f64);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(calls_to(&log, 0x00b5_cbd0).len(), 1);
        // Near enough: 2.0 < 1.0 * 3.0.
        let (mut r, this, _) = caster_rig(2);
        r.set_float(0x0084_d030, 2.0);
        r.set_float(FLOAT_HELPER_A, 1.0);
        r.e.set_global(0x0102_1928, 3.0f64);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(calls_to(&log, 0x00b5_cbd0).len(), 2);
    }

    #[test]
    fn the_light_walk_ends_with_the_remaining_stages_and_the_scene_update() {
        let (mut r, this, _) = caster_rig(1);
        r.set(0x0095_0bb0, &[0x6000]);
        r.set(GET_ROOT, &[0x3000]);
        r.set(GET_ROOT_CHILD, &[0x4000]);
        r.set(0x008d_80e0, &[0x8000]);
        r.set(MEMBER_8C_B, &[0x9000]);
        r.set(0x00b5_afc0, &[0x7100]);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(calls_to(&log, 0x0041_fd00), vec![vec![0x8000, 0x4000]]);
        assert_eq!(calls_to(&log, 0x00a6_94a0), vec![vec![0x8000, 0x9000]]);
        // The child is flagged, then restored to the flag read (0).
        assert_eq!(
            calls_to(&log, 0x0045_0f90),
            vec![vec![0x6000, 1], vec![0x6000, 0]]
        );
        assert_eq!(calls_to(&log, 0x004e_20b0), vec![vec![0], vec![1]]);
        // One light, without geometry: the walk moves on; no light was
        // queued, so the four remaining stages run on thread 0.
        assert_eq!(calls_to(&log, 0x004e_6540), vec![vec![0x7100]]);
        assert_eq!(calls_to(&log, 0x00b5_b010), vec![vec![0x3333]]);
        assert_eq!(
            stages(&log, MT_SET_THREAD_STAGE),
            vec![0x11, 0x12, 0x13, 0x14]
        );
        assert!(stages(&log, MT_ADVANCE_THREAD_STAGE).is_empty());
        assert_eq!(calls_to(&log, 0x00b5_b880), vec![vec![0x3333, 0x8000]]);
    }

    #[test]
    fn the_saved_object_is_flagged_during_the_walk_unless_the_setting_says_not() {
        let (mut r, this, _) = caster_rig(1);
        r.set(0x0095_0bb0, &[0x6000]);
        r.set(GET_GLOBAL_011CA438, &[0x6500]);
        r.set(0x0045_6610, &[0]);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        let flags = calls_to(&log, 0x0045_0f90);
        assert_eq!(
            flags,
            vec![
                vec![0x6000, 1],
                vec![0x6500, 1],
                vec![0x6500, 0],
                vec![0x6000, 0]
            ]
        );
        // With the byte setting `0x011c71dc` the object is left alone first.
        let (mut r, this, _) = caster_rig(1);
        r.set(0x0095_0bb0, &[0x6000]);
        r.set(GET_GLOBAL_011CA438, &[0x6500]);
        r.settings(SETTING_BYTE_PTR, &[(0x011c_74d4, 1), (0x011c_71dc, 1)]);
        let (_, log) = r.run(0x0087_1290, &args![this]);
        assert_eq!(
            calls_to(&log, 0x0045_0f90),
            vec![vec![0x6000, 1], vec![0x6500, 0], vec![0x6000, 0]]
        );
    }

    /// Calls `shadow_light` for a light `0x7100` with the objects set up.
    fn light_rig() -> (Rig, u32) {
        let mut r = Rig::new();
        let indexed = r.object(0x40);
        r.set(INDEXED_GLOBAL, &[indexed]);
        r.set(0x004e_6540, &[0x5001]);
        r.set(0x005b_f820, &[0x5002]);
        r.set(0x004e_6580, &[1]);
        let position = r.object(16);
        for (i, word) in [1u32, 2, 3].iter().enumerate() {
            r.e.mem.set_u32(position + 4 * i as u32, *word);
        }
        r.set(MEMBER_8C, &[position]);
        r.set(0x004e_6560, &[0x10]);
        r.set_float(0x0097_9260, 1.0);
        r.e.set_global(0x0108_2cb0, 0.05f64);
        (r, indexed)
    }

    fn walk_light(r: &mut Rig, limit: i32, processed: i32) -> (i32, Log) {
        r.e.call_log = Some(vec![]);
        let mut processed = processed;
        shadow_light(&mut r.e, 0x7100, 0x8000, limit, &mut processed);
        (processed, r.e.call_log.take().unwrap())
    }

    #[test]
    fn a_light_without_geometry_or_source_is_left_alone() {
        let (mut r, _) = light_rig();
        r.set(0x004e_6540, &[0]);
        let (processed, log) = walk_light(&mut r, 4, 0);
        assert_eq!(processed, 0);
        assert_eq!(addrs(&log), vec![0x004e_6540]);
        let (mut r, _) = light_rig();
        r.set(0x005b_f820, &[0]);
        let (_, log) = walk_light(&mut r, 4, 0);
        assert_eq!(addrs(&log), vec![0x004e_6540, 0x005b_f820]);
    }

    #[test]
    fn a_usable_light_is_recomputed_and_queued() {
        let (mut r, indexed) = light_rig();
        r.settings(SETTING_BYTE_PTR, &[(0x011c_7664, 1)]);
        let (processed, log) = walk_light(&mut r, 4, 1);
        assert_eq!(processed, 2);
        assert_eq!(calls_to(&log, 0x00b9_d150), vec![vec![0x7100, 0, 1, 2, 3]]);
        assert_eq!(calls_to(&log, 0x00b9_e970), vec![vec![0x7100, 0x8000]]);
        // The ordinal is the count before this light.
        assert_eq!(calls_to(&log, 0x00b9_dfc0), vec![vec![0x7100, 0x8000, 1]]);
        assert_eq!(
            calls_to(&log, 0x00a5_9c60),
            vec![vec![0x5001, calls_to(&log, 0x0043_d410)[0][0]]]
        );
        assert_eq!(calls_to(&log, 0x0043_d410)[0][1..], [0, 0, 0]);
        assert_eq!(calls_to(&log, 0x00b5_d300), vec![vec![indexed, 0x7100]]);
        assert_eq!(calls_to(&log, 0x00ba_0110), vec![vec![0x7100, 0x5002]]);
        assert_eq!(calls_to(&log, 0x00ba_02b0), vec![vec![0x7100]]);
        assert!(calls_to(&log, 0x0045_0f90).is_empty());
        // Without the setting the light is not added to the source's list.
        let (mut r, _) = light_rig();
        let (_, log) = walk_light(&mut r, 4, 0);
        assert!(calls_to(&log, 0x00ba_0110).is_empty());
    }

    #[test]
    fn a_light_that_is_too_dim_or_full_is_switched_off() {
        // Weight below the cutoff.
        let (mut r, _) = light_rig();
        r.set_float(0x0097_9260, 0.01);
        let (processed, log) = walk_light(&mut r, 4, 0);
        assert_eq!(processed, 0);
        assert_eq!(calls_to(&log, 0x0045_0f90), vec![vec![0x5001, 1]]);
        assert!(calls_to(&log, 0x00b9_dfc0).is_empty());
        // Count 0xff.
        let (mut r, _) = light_rig();
        r.set(0x004e_6560, &[0xff]);
        let (processed, log) = walk_light(&mut r, 4, 0);
        assert_eq!(processed, 0);
        assert_eq!(calls_to(&log, 0x0045_0f90), vec![vec![0x5001, 1]]);
        // The limit is reached: nothing is recomputed.
        let (mut r, _) = light_rig();
        let (processed, log) = walk_light(&mut r, 2, 2);
        assert_eq!(processed, 2);
        assert!(calls_to(&log, 0x00b9_d150).is_empty());
        assert_eq!(calls_to(&log, 0x0045_0f90), vec![vec![0x5001, 1]]);
        // Geometry flagged by `00456610`.
        let (mut r, _) = light_rig();
        r.set(0x0045_6610, &[1]);
        let (_, log) = walk_light(&mut r, 4, 0);
        assert!(calls_to(&log, 0x00b9_d150).is_empty());
    }

    #[test]
    fn a_light_with_a_dirty_state_is_reset_before_the_decision() {
        // `004e6580` is false and the geometry is flagged: the source flag
        // decides between the two resets.
        let (mut r, _) = light_rig();
        r.set(0x004e_6580, &[0]);
        r.set(0x0045_6610, &[1, 1, 1, 1]);
        let (_, log) = walk_light(&mut r, 4, 0);
        assert_eq!(calls_to(&log, 0x00b9_bb10), vec![vec![0x7100, 0, 1]]);
        // The source is not flagged: below the limit the geometry is
        // cleared first.
        let (mut r, _) = light_rig();
        r.set(0x004e_6580, &[0]);
        r.set(0x0045_6610, &[1, 0]);
        let (_, log) = walk_light(&mut r, 4, 0);
        assert_eq!(
            only(&log, &[0x0045_0f90, 0x00b9_bb10]),
            vec![0x0045_0f90, 0x00b9_bb10]
        );
        assert_eq!(calls_to(&log, 0x00b9_bb10), vec![vec![0x7100, 0, 0]]);
        assert_eq!(calls_to(&log, 0x0045_0f90)[0], vec![0x5001, 0]);
        // At the limit the geometry is left alone by the reset.
        let (mut r, _) = light_rig();
        r.set(0x004e_6580, &[0]);
        r.set(0x0045_6610, &[1, 0, 1]);
        let (_, log) = walk_light(&mut r, 2, 2);
        assert_eq!(calls_to(&log, 0x00b9_bb10), vec![vec![0x7100, 0, 0]]);
        assert_eq!(calls_to(&log, 0x0045_0f90), vec![vec![0x5001, 1]]);
    }

    #[test]
    fn setting_byte_getter_and_member_address() {
        let mut r = Rig::new();
        r.settings(SETTING_BYTE_PTR, &[(0x011c_74d4, 3)]);
        assert_eq!(r.e.call(0x0087_1a10, &[]).u8(), 3);
        assert_eq!(r.e.call(0x0087_1a30, &args![0x1000u32]).u32(), 0x1088);
    }

    #[test]
    fn offscreen_update_does_nothing_outside_a_rendered_menu() {
        let mut r = Rig::new();
        let (_, log) = r.run(0x0087_1a50, &args![0u32]);
        assert_eq!(addrs(&log), vec![IS_IN_MENU_MODE, IS_IN_MENU_MODE]);
        // In menu mode the last-minute update runs once.
        r.set(IS_IN_MENU_MODE, &[1]);
        let (_, log) = r.run(0x0087_1a50, &args![0u32]);
        assert_eq!(only(&log, &[LAST_MINUTE_UPDATE]), vec![LAST_MINUTE_UPDATE]);
        // A setting is on but no rendered menu is showing.
        r.settings(SETTING_BYTE_PTR, &[(0x011d_b2cc, 1)]);
        let (_, log) = r.run(0x0087_1a50, &args![0u32]);
        assert_eq!(
            addrs(&log),
            vec![
                IS_IN_MENU_MODE,
                SETTING_BYTE_PTR,
                IS_IN_RENDERED_MENU,
                IS_IN_MENU_MODE,
                LAST_MINUTE_UPDATE
            ]
        );
        // The second setting alone is enough.
        r.settings(SETTING_BYTE_PTR, &[(0x011d_8ba0, 1)]);
        let (_, log) = r.run(0x0087_1a50, &args![0u32]);
        assert!(addrs(&log).contains(&IS_IN_RENDERED_MENU));
    }

    #[test]
    fn offscreen_update_creates_the_target_and_refreshes_the_rendered_menu() {
        let mut r = Rig::new();
        r.set(IS_IN_MENU_MODE, &[1]);
        r.set(IS_IN_RENDERED_MENU, &[1]);
        r.settings(SETTING_BYTE_PTR, &[(0x011d_b2cc, 1)]);
        r.set(GET_GLOBAL_011F4748, &[0x1111]);
        r.set(GET_GLOBAL_011F91A8, &[0x2222]);
        r.set(0x00b6_e110, &[0x5555]);
        r.set(GET_PIPBOY, &[0x6666]);
        r.set(IS_CURRENT_RENDERED_MENU_TOPMOST, &[1]);
        let menu = r.object_with_slots(&[(0x0c, 0)]);
        r.set(GET_CURRENT_RENDERED_MENU, &[menu]);
        let refresh = slot_target(&r, menu, 0x0c);
        let (_, log) = r.run(0x0087_1a50, &args![0u32]);
        let guard = calls_to(&log, PROFILE_SCOPE_CTOR);
        assert_eq!(guard.len(), 1);
        assert_eq!(guard[0][1..], [0xd, 1, SOURCE_FILE_NAME, 0x1ade]);
        assert_eq!(
            calls_to(&log, 0x00b6_e110),
            vec![vec![0x2222, 0x1111, 0x2d, 0, 0, 0]]
        );
        assert_eq!(
            calls_to(&log, 0x0066_b0d0),
            vec![vec![OFFSCREEN_TARGET_POINTER, 0x5555]]
        );
        assert_eq!(calls_to(&log, SET_SCREEN_TEXTURE), vec![vec![0x6666, 0]]);
        // The pointer read after the creation still comes back empty here.
        assert_eq!(calls_to(&log, refresh), vec![vec![menu, 0, 1, 0]]);
        assert_eq!(calls_to(&log, PROFILE_SCOPE_DTOR).len(), 1);
        assert_eq!(*addrs(&log).last().unwrap(), PROFILE_SCOPE_DTOR);
    }

    #[test]
    fn offscreen_update_flags_the_pipboy_reference_around_the_refresh() {
        let mut r = Rig::new();
        r.set(IS_IN_MENU_MODE, &[1]);
        r.set(IS_IN_RENDERED_MENU, &[1]);
        r.set(IS_IN_PIPBOY_MENU, &[1]);
        r.set(IS_MENU_ID_VISIBLE, &[1]);
        r.settings(SETTING_BYTE_PTR, &[(0x011d_b2cc, 1)]);
        r.pointers(&[(OFFSCREEN_TARGET_POINTER, 0x5555)]);
        r.set(GET_MENU_BY_CLASS, &[0x7000]);
        r.set(0x0056_c7f0, &[0x7100]);
        r.set(0x0045_6610, &[0]);
        let menu = r.object_with_slots(&[(0x0c, 0)]);
        r.set(GET_CURRENT_RENDERED_MENU, &[menu]);
        let refresh = slot_target(&r, menu, 0x0c);
        let (_, log) = r.run(0x0087_1a50, &args![0u32]);
        // No target creation, and no pipboy screen texture in the pipboy.
        assert!(!addrs(&log).contains(&0x00b6_e110));
        assert!(!addrs(&log).contains(&SET_SCREEN_TEXTURE));
        assert_eq!(calls_to(&log, 0x0056_c7f0), vec![vec![0x7000]]);
        assert_eq!(
            only(&log, &[0x0045_0f90, refresh]),
            vec![0x0045_0f90, refresh, 0x0045_0f90]
        );
        assert_eq!(
            calls_to(&log, 0x0045_0f90),
            vec![vec![0x7100, 1], vec![0x7100, 0]]
        );
        assert_eq!(calls_to(&log, refresh), vec![vec![menu, 0x5555, 1, 0]]);
    }

    #[test]
    fn offscreen_update_needs_a_topmost_menu_or_one_of_the_three_menus() {
        for (menu_class, expected) in [(0x3e9u32, true), (0x3f8, true), (0x423, true), (0, false)] {
            let mut r = Rig::new();
            r.set(IS_IN_MENU_MODE, &[1]);
            r.set(IS_IN_RENDERED_MENU, &[1]);
            r.settings(SETTING_BYTE_PTR, &[(0x011d_b2cc, 1)]);
            r.pointers(&[(OFFSCREEN_TARGET_POINTER, 0x5555)]);
            let menu = r.object_with_slots(&[(0x0c, 0)]);
            r.set(GET_CURRENT_RENDERED_MENU, &[menu]);
            let refresh = slot_target(&r, menu, 0x0c);
            let class = menu_class;
            r.e.register_double(GET_MENU_BY_CLASS, move |_, a| Ret {
                eax: (a[0] == class) as u32,
                ..Ret::default()
            });
            let (_, log) = r.run(0x0087_1a50, &args![0u32]);
            assert_eq!(
                calls_to(&log, refresh).len(),
                expected as usize,
                "{menu_class:x}"
            );
        }
    }

    #[test]
    fn set_active_toggles_the_timer_and_clears_the_controls() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.set(CONTROLS_GET_INSTANCE, &[0x7777]);
        let (_, log) = r.run(0x0087_1c90, &args![this, 1u32]);
        assert_eq!(
            addrs(&log),
            vec![
                SETTING_BYTE_PTR,
                CONTROLS_GET_INSTANCE,
                CONTROLS_GET_INSTANCE,
                CONTROLS_CLEAR_KEYSTROKES,
                CONTROLS_GET_INSTANCE,
                CONTROLS_POLL,
                CONTROLS_GET_INSTANCE,
                CONTROLS_CLEAR_USER_ACTIONS,
                TIMER_ENABLE
            ]
        );
        assert_eq!(calls_to(&log, CONTROLS_POLL), vec![vec![0x7777]]);
        assert_eq!(calls_to(&log, TIMER_ENABLE), vec![vec![0x011f_6394]]);
        assert_eq!(r.e.mem.u8(this + 3), 1);
        // Inactive, and no controls object.
        r.set(CONTROLS_GET_INSTANCE, &[0]);
        let (_, log) = r.run(0x0087_1c90, &args![this, 0u32]);
        assert_eq!(
            addrs(&log),
            vec![SETTING_BYTE_PTR, CONTROLS_GET_INSTANCE, TIMER_DISABLE]
        );
        assert_eq!(r.e.mem.u8(this + 3), 0);
    }

    #[test]
    fn the_always_active_setting_forces_the_flag() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.settings(SETTING_BYTE_PTR, &[(0x011d_eed8, 1)]);
        let (_, log) = r.run(0x0087_1c90, &args![this, 0u32]);
        assert_eq!(addrs(&log).last(), Some(&TIMER_ENABLE));
        assert_eq!(r.e.mem.u8(this + 3), 1);
    }

    #[test]
    fn pointer_forwarding_needs_the_active_window_and_menu_mode() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.e.mem.set_u32(this + 8, 0x1234);
        r.set(IMPORT_GET_ACTIVE_WINDOW, &[0x9999]);
        let (ret, log) = r.run(0x0087_1d10, &args![this, 10i32, 20i32]);
        assert!(!ret.bool());
        assert_eq!(addrs(&log), vec![IMPORT_GET_ACTIVE_WINDOW]);
        r.set(IMPORT_GET_ACTIVE_WINDOW, &[0x1234]);
        let (ret, log) = r.run(0x0087_1d10, &args![this, 10i32, 20i32]);
        assert!(!ret.bool());
        assert_eq!(addrs(&log), vec![IMPORT_GET_ACTIVE_WINDOW, IS_IN_MENU_MODE]);
        r.set(IS_IN_MENU_MODE, &[1]);
        let (ret, log) = r.run(0x0087_1d10, &args![this, 10i32, 20i32]);
        assert!(!ret.bool());
        assert_eq!(
            addrs(&log),
            vec![
                IMPORT_GET_ACTIVE_WINDOW,
                IS_IN_MENU_MODE,
                CONTROLS_GET_INSTANCE,
                0x0071_1e00
            ]
        );
    }

    #[test]
    fn pointer_forwarding_divides_by_the_display_size() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.e.mem.set_u32(this + 8, 0x1234);
        r.set(IMPORT_GET_ACTIVE_WINDOW, &[0x1234]);
        r.set(IS_IN_MENU_MODE, &[1]);
        r.set(CONTROLS_GET_INSTANCE, &[0x7777]);
        r.set(0x0071_1e00, &[1]);
        r.set(0x004d_c1f0, &[1000]);
        r.set(0x004d_c200, &[500]);
        let (ret, log) = r.run(0x0087_1d10, &args![this, 250i32, 100i32]);
        assert!(ret.bool());
        assert_eq!(
            only(&log, &[0x004d_c200, 0x004d_c1f0, 0x0070_3080, 0x0070_6230]),
            vec![0x004d_c200, 0x004d_c1f0, 0x0070_3080]
        );
        assert_eq!(
            calls_to(&log, 0x0070_3080),
            vec![vec![word(0.25), word(0.2)]]
        );
        assert_eq!(calls_to(&log, IS_TOP_MENU_ID), vec![vec![0x41e]]);
        // With menu 0x41e on top the position goes through unchanged.
        r.set(IS_TOP_MENU_ID, &[1]);
        let (ret, log) = r.run(0x0087_1d10, &args![this, -5i32, 100i32]);
        assert!(ret.bool());
        assert_eq!(calls_to(&log, 0x0070_6230), vec![vec![(-5i32) as u32, 100]]);
        assert!(!addrs(&log).contains(&0x0070_3080));
    }

    #[test]
    fn target_sizes_come_from_the_pointed_to_object() {
        let mut r = Rig::new();
        let this = r.object(0x80);
        let target = r.object_with_slots(&[(0x94, 640), (0x98, 480)]);
        r.pointers(&[(this + 0x30, target), (this + 0x38, target)]);
        assert_eq!(r.e.call(0x0087_2370, &args![this, 0u32]).u32(), 640);
        assert_eq!(r.e.call(0x0087_23d0, &args![this, 0u32]).u32(), 480);
        assert_eq!(r.e.call(0x0087_2370, &args![this, 2u32]).u32(), 640);
        assert_eq!(r.e.call(0x0087_23d0, &args![this, 2u32]).u32(), 480);
        // An empty slot gives 0.
        assert_eq!(r.e.call(0x0087_2370, &args![this, 1u32]).u32(), 0);
        assert_eq!(r.e.call(0x0087_23d0, &args![this, 1u32]).u32(), 0);
    }

    /// The rig of the menu-background render: a player with a vtable, a
    /// renderer whose group answers `800 x 300`, and a menu target whose
    /// first buffer answers `target_size`.
    fn menu_background_rig(target_size: (u32, u32)) -> (Rig, u32, u32, u32) {
        let (mut r, this, _) = frame_rig(0);
        let player = r.object_with_slots(&[(0x1d0, 1)]);
        r.e.set_global(PLAYER_OBJECT, player);
        r.set(0x008d_6f30, &[1]);
        let group = r.object_with_slots(&[(0x8c, 800), (0x90, 300)]);
        r.object_at(0x1000, &[(0xc8, group), (0xac, 0)]);
        let buffer = r.object_with_slots(&[(0x94, target_size.0), (0x98, target_size.1)]);
        let target = r.object(0x40);
        r.pointers(&[(MENU_TARGET_POINTER, target), (target + 0x30, buffer)]);
        r.set(GET_GLOBAL_011F91A8, &[0x2222]);
        r.set(0x00b6_e110, &[0x5555]);
        (r, this, target, group)
    }

    #[test]
    fn menu_background_render_needs_a_player_with_a_process() {
        let mut r = Rig::new();
        let this = r.object(0x200);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert_eq!(
            addrs(&log),
            vec![PROFILE_SCOPE_CTOR, 0x008d_6f30, PROFILE_SCOPE_DTOR]
        );
        assert_eq!(
            calls_to(&log, PROFILE_SCOPE_CTOR)[0][1..],
            [0xd, 1, SOURCE_FILE_NAME, 0x1c19]
        );
        // The player's vtable slot answers 0.
        let player = r.object_with_slots(&[(0x1d0, 0)]);
        r.e.set_global(PLAYER_OBJECT, player);
        r.set(0x008d_6f30, &[1]);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert_eq!(addrs(&log).len(), 4);
        assert_eq!(r.e.mem.u8(this + 0x9c), 0);
    }

    #[test]
    fn menu_background_render_recreates_the_target_and_keeps_a_matching_size() {
        let (mut r, this, target, _) = menu_background_rig((800, 300));
        r.e.set_global(0x011a_d884, 5u8);
        r.set(GET_MT_SYSTEM, &[0x0120_0088]);
        r.set(GET_CURRENT_RENDERED_MENU, &[0x7a00]);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert_eq!(calls_to(&log, TILE_LOCK).len(), 1);
        assert_eq!(calls_to(&log, MT_BEGIN_FRAME), vec![vec![0x0120_0088]]);
        assert_eq!(calls_to(&log, 0x00b6_da10), vec![vec![0x2222, target]]);
        assert_eq!(
            calls_to(&log, 0x00b6_e110),
            vec![vec![0x2222, 0x1000, 6, 0, 0, 0]]
        );
        assert_eq!(
            calls_to(&log, 0x0066_b0d0),
            vec![vec![MENU_TARGET_POINTER, 0x5555]]
        );
        assert_eq!(
            only(
                &log,
                &[
                    TILE_LOCK,
                    MT_BEGIN_FRAME,
                    0x00b6_da10,
                    0x00b6_e110,
                    0x0066_b0d0
                ]
            ),
            vec![
                TILE_LOCK,
                MT_BEGIN_FRAME,
                0x00b6_da10,
                0x00b6_e110,
                0x0066_b0d0
            ]
        );
        // The size matches and the flag byte is clear: no ratio color.
        let colors = calls_to(&log, SET_COLOR);
        assert_eq!(
            colors.iter().filter(|c| c[1] == 0x011a_d840).count(),
            1,
            "only the frame's own"
        );
        assert_eq!(calls_to(&log, FLOAT_HELPER_B).len(), 0);
        // The state is restored and the end of the frame is marked.
        assert_eq!(r.e.mem.u8(this + 0x9c), 0);
        assert_eq!(r.e.global::<u8>(0x011a_d884), 5);
        assert_eq!(r.e.global::<u8>(0x011d_ea29), 1);
        assert!(stages(&log, MT_SET_THREAD_STAGE).contains(&0x17));
        assert_eq!(calls_to(&log, MT_END_FRAME), vec![vec![0x0120_0088]]);
        assert_eq!(calls_to(&log, TILE_UNLOCK).len(), 1);
        assert_eq!(*addrs(&log).iter().rev().nth(1).unwrap(), 0x004a_03c0);
        assert_eq!(calls_to(&log, 0x004a_03c0), vec![vec![0x1000]]);
    }

    #[test]
    fn menu_background_render_keeps_the_old_target_released_only_when_there_is_one() {
        let (mut r, this, _, _) = menu_background_rig((800, 300));
        r.pointers(&[]);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert!(calls_to(&log, 0x00b6_da10).is_empty());
    }

    #[test]
    fn menu_background_render_sets_the_size_ratios_when_the_target_differs() {
        // Width differs (800 against 400) and height differs (300 against 600).
        let (mut r, this, _, _) = menu_background_rig((400, 600));
        let calls = Rc::new(RefCell::new(0));
        {
            let calls = calls.clone();
            r.e.register_double(FLOAT_HELPER_B, move |_, _| {
                let mut calls = calls.borrow_mut();
                *calls += 1;
                Ret {
                    st0: if *calls == 1 { 1.5 } else { 0.75 },
                    ..Ret::default()
                }
            });
        }
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert_eq!(
            calls_to(&log, FLOAT_HELPER_B),
            vec![vec![word(2.0), word(1.0)], vec![word(0.5), word(1.0)]]
        );
        let first_color = calls_to(&log, COLOR_CTOR)[0].clone();
        assert_eq!(first_color[1..], [0, word(1.5), word(0.75), 0]);
        let set = calls_to(&log, SET_COLOR)[0].clone();
        assert_eq!(set, vec![0x4000, first_color[0]]);
    }

    #[test]
    fn menu_background_render_height_alone_also_counts() {
        let (mut r, this, _, _) = menu_background_rig((800, 100));
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert_eq!(calls_to(&log, FLOAT_HELPER_B).len(), 2);
    }

    #[test]
    fn menu_background_render_resets_the_clear_value_from_the_flag_byte() {
        let (mut r, this, _, _) = menu_background_rig((800, 300));
        r.e.set_global(0x011f_9426, 1u8);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        let own = calls_to(&log, SET_COLOR)
            .iter()
            .filter(|c| c[1] == 0x011a_d840)
            .count();
        assert_eq!(own, 2);
    }

    #[test]
    fn menu_background_render_fades_the_image_space_around_the_frame() {
        let (mut r, this, target, _) = menu_background_rig((800, 300));
        r.set(GET_GLOBAL_011D8A80, &[0x4440]);
        r.set(0x0071_8ab0, &[0x6660]);
        r.set(0x005d_2860, &[0x7770]);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert_eq!(calls_to(&log, 0x0071_8ab0), vec![vec![0x4440]]);
        assert_eq!(
            calls_to(&log, 0x0052_9c90),
            vec![vec![0x7770], vec![0x6660]]
        );
        assert_eq!(
            calls_to(&log, 0x0052_99a0),
            vec![vec![0x6660, word(1.0), 0]]
        );
        assert_eq!(
            calls_to(&log, 0x0086_fd90),
            vec![vec![this, 0], vec![this, 0]]
        );
        // The frame goes to the menu target: `00870bd0` is the plain frame.
        assert!(addrs(&log).contains(&0x0087_4ac0));
        let _ = target;
    }

    #[test]
    fn menu_background_render_skips_the_world_in_the_pipboy_unless_menu_3f5_shows() {
        // In the pipboy: the frame is asked to skip the world.
        let (mut r, this, _, _) = menu_background_rig((800, 300));
        r.set(IS_IN_PIPBOY_MENU, &[1]);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert!(addrs(&log).contains(&0x0087_4ac0));
        assert!(!addrs(&log).contains(&0x0087_5110));
        // With menu 0x3f5 visible the flag is dropped.
        let (mut r, this, _, _) = menu_background_rig((800, 300));
        r.set(IS_IN_PIPBOY_MENU, &[1]);
        r.set(IS_MENU_ID_VISIBLE, &[1, 0]);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert!(addrs(&log).contains(&0x0087_5110));
    }

    #[test]
    fn menu_background_render_accumulates_the_rendered_menu_scene() {
        let (mut r, this, _, _) = menu_background_rig((800, 300));
        r.set(IS_MENU_ID_VISIBLE, &[0, 1]);
        r.set(0x004a_4040, &[1]);
        r.set(GET_CURRENT_RENDERED_MENU, &[0x7a00]);
        r.set(0x008b_6200, &[0x7a10]);
        r.e.register_double(0x0055_85e0, |_, a| Ret {
            eax: if a[0] == 0x7a00 { 0x7b00 } else { 0 },
            ..Ret::default()
        });
        r.pointers(&[(MENU_TARGET_POINTER, 0), (this + MAIN_MENU_ACCUM, 0x9a00)]);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        let object = calls_to(&log, 0x004a_0eb0)[0].clone();
        let at = object[0];
        assert_eq!(object[1], 0);
        assert_eq!(calls_to(&log, 0x0041_fd00), vec![vec![at, 0x7a10]]);
        assert_eq!(
            calls_to(&log, 0x004a_0fd0),
            vec![vec![at, 0x9a00], vec![at, 0]]
        );
        assert_eq!(calls_to(&log, 0x004a_1020).len(), 1);
        assert_eq!(calls_to(&log, 0x004a_1020)[0][0], 0x9a00);
        assert_eq!(calls_to(&log, 0x00b6_bee0), vec![vec![0x7a10, 0x7b00, at]]);
        assert_eq!(calls_to(&log, 0x00b6_c0d0), vec![vec![0x7a10, 0x9a00, 0]]);
        assert_eq!(calls_to(&log, 0x004a_0f60), vec![vec![at]]);
        // Not when `004a4040` refuses.
        r.set(0x004a_4040, &[0]);
        let (_, log) = r.run(0x0087_1dc0, &args![this]);
        assert!(!addrs(&log).contains(&0x004a_0eb0));
    }

    /// A rig for the list-file reader: the lines the file yields, the names
    /// the data handler knows.
    fn list_file_rig(lines: &[&str], known: &[&str]) -> (Rig, Rc<RefCell<Vec<String>>>) {
        let mut r = Rig::new();
        let lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
        let next = Rc::new(RefCell::new(0usize));
        r.e.register_double(0x00ec_9a47, |_, _| Ret {
            eax: 0x77,
            ..Ret::default()
        });
        r.e.register_double(0x00ec_c104, move |e, a| {
            let mut next = next.borrow_mut();
            if *next >= lines.len() {
                return Ret::default();
            }
            e.mem.set_cstr(a[0], lines[*next].as_bytes());
            *next += 1;
            Ret {
                eax: a[0],
                ..Ret::default()
            }
        });
        r.e.register_double(0x0044_a670, |e, a| Ret {
            eax: e.mem.cstr(a[0]).len() as u32,
            ..Ret::default()
        });
        let asked: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
        {
            let asked = asked.clone();
            let known: Vec<String> = known.iter().map(|k| k.to_string()).collect();
            r.e.register_double(0x0046_2f40, move |e, a| {
                let name = String::from_utf8(e.mem.cstr(a[1])).unwrap();
                asked.borrow_mut().push(name.clone());
                let index = known.iter().position(|k| *k == name);
                Ret {
                    eax: index.map_or(0, |i| 0x9000 + i as u32),
                    ..Ret::default()
                }
            });
        }
        (r, asked)
    }

    #[test]
    fn list_file_lines_activate_the_known_entries() {
        let (mut r, asked) = list_file_rig(
            &["# comment\n", "a.esp\n", "\n", "b.esp", "missing.esp\n"],
            &["a.esp", "b.esp"],
        );
        r.e.set_global(0x011c_3f2c, 0x4242u32);
        let this = r.object(0x10);
        let (ret, log) = r.run(0x0087_2430, &args![this, 0x1111u32, 0x2222u32]);
        assert_eq!(ret.u8(), 1);
        assert_eq!(*asked.borrow(), vec!["a.esp", "b.esp", "missing.esp"]);
        assert_eq!(
            calls_to(&log, 0x0047_1d00),
            vec![vec![0x9000, 1], vec![0x9001, 1]]
        );
        let first = calls_to(&log, 0x0040_6d30)[0].clone();
        assert_eq!(first[1..], [0x104, 0x1111]);
        let second = calls_to(&log, 0x0040_6d50)[0].clone();
        assert_eq!(second, vec![first[0], 0x104, 0x2222]);
        assert_eq!(
            calls_to(&log, 0x00ec_9a47),
            vec![vec![first[0], 0x0108_2cb8]]
        );
        assert_eq!(calls_to(&log, 0x00ec_9907), vec![vec![0x77]]);
        assert_eq!(calls_to(&log, 0x0046_2f40)[0][0], 0x4242);
        // Every line is read into the same 512-byte buffer from the file.
        assert_eq!(calls_to(&log, 0x00ec_c104)[0][1..], [0x200, 0x77]);
    }

    #[test]
    fn list_file_without_hits_or_without_a_file_returns_zero() {
        let (mut r, _) = list_file_rig(&["x.esp\n"], &[]);
        let this = r.object(0x10);
        let (ret, log) = r.run(0x0087_2430, &args![this, 1u32, 2u32]);
        assert_eq!(ret.u8(), 0);
        assert!(calls_to(&log, 0x0047_1d00).is_empty());
        let (mut r, _) = list_file_rig(&[], &[]);
        r.e.register_double(0x00ec_9a47, |_, _| Ret::default());
        let (ret, log) = r.run(0x0087_2430, &args![this, 1u32, 2u32]);
        assert_eq!(ret.u8(), 0);
        assert!(!addrs(&log).contains(&0x00ec_c104));
        assert!(!addrs(&log).contains(&0x00ec_9907));
    }

    #[test]
    fn list_file_skips_comments_and_one_character_lines() {
        let (mut r, asked) = list_file_rig(&["#a.esp\n", "x", "\n"], &["a.esp", "x"]);
        let this = r.object(0x10);
        let (ret, _) = r.run(0x0087_2430, &args![this, 1u32, 2u32]);
        assert_eq!(ret.u8(), 0);
        assert!(asked.borrow().is_empty());
    }

    #[test]
    fn housekeeping_without_release_clears_adaptation_and_wakes_the_water() {
        let mut r = Rig::new();
        r.e.set_global(0x011f_941e, 1u8);
        r.set(0x0070_ec90, &[0x6600]);
        r.set(0x005d_2a40, &[1]);
        let (ret, log) = r.run(0x0087_2570, &args![0u32]);
        assert_eq!(ret.u8(), 1);
        assert_eq!(
            addrs(&log),
            vec![
                0x00ba_d4a0,
                0x0070_ec90,
                0x005d_2a40,
                0x0070_ec90,
                0x004e_6620,
                0x0070_ec90,
                0x004e_65d0
            ]
        );
        assert_eq!(calls_to(&log, 0x004e_6620), vec![vec![0x6600, 1, 0]]);
        assert_eq!(calls_to(&log, 0x004e_65d0), vec![vec![0x6600]]);
        // No adaptation flag and nothing from `005d2a40`.
        r.e.set_global(0x011f_941e, 0u8);
        r.set(0x005d_2a40, &[0]);
        let (ret, log) = r.run(0x0087_2570, &args![0u32]);
        assert_eq!(ret.u8(), 1);
        assert_eq!(addrs(&log), vec![0x0070_ec90, 0x005d_2a40]);
        // No object at all.
        r.set(0x0070_ec90, &[0]);
        let (_, log) = r.run(0x0087_2570, &args![0u32]);
        assert_eq!(addrs(&log), vec![0x0070_ec90]);
    }

    /// The rig of the release housekeeping: a movie player at `0x0126fac4`,
    /// the `Main` object, a 2 x 2 cell grid.
    fn release_rig() -> (Rig, u32, u32) {
        let mut r = Rig::new();
        let player = r.object(0x40);
        let inner = r.object(0x40);
        r.e.mem.set_u32(player + 0xc, inner);
        r.e.set_global(0x0126_fac4, player);
        let main_object = r.object(0x40);
        r.e.mem.set_u32(main_object + 8, 0x1234);
        r.e.set_global(MAIN_OBJECT, main_object);
        r.set(0x0086_bdf0, &[1]);
        r.set(0x0044_ddc0, &[0x6000]);
        (r, player, inner)
    }

    #[test]
    fn release_housekeeping_waits_for_the_loading_thread_and_stops_the_movie() {
        let (mut r, player, inner) = release_rig();
        r.set(0x0086_bdf0, &[0, 0, 1]);
        r.set(0x00ec_17c0, &[1]);
        r.e.mem.set_u32(inner + 0x20, 1);
        r.set(0x00b4_f5c0, &[0x6600]);
        let (ret, log) = r.run(0x0087_2570, &args![1u32]);
        assert_eq!(ret.u8(), 1);
        assert_eq!(calls_to(&log, 0x0040_fca0), vec![vec![5], vec![5]]);
        assert_eq!(calls_to(&log, 0x0086_bdf0).len(), 3);
        assert_eq!(
            only(&log, &[0x00ec_17c0, 0x00ec_29b0, 0x00ec_1800, 0x00ec_25e0]),
            vec![0x00ec_17c0, 0x00ec_29b0, 0x00ec_1800, 0x00ec_25e0]
        );
        for addr in [0x00ec_17c0, 0x00ec_29b0, 0x00ec_1800, 0x00ec_25e0] {
            assert_eq!(calls_to(&log, addr), vec![vec![player]]);
        }
        assert_eq!(calls_to(&log, 0x00b6_31d0), vec![vec![0x6600]]);
        assert_eq!(calls_to(&log, 0x0087_7430).len(), 1);
    }

    #[test]
    fn release_housekeeping_leaves_the_movie_alone_when_idle() {
        let (mut r, _, _) = release_rig();
        let (_, log) = r.run(0x0087_2570, &args![1u32]);
        assert!(!addrs(&log).contains(&0x00ec_29b0));
        assert!(!addrs(&log).contains(&0x00ec_25e0));
        // No accumulator either.
        assert!(!addrs(&log).contains(&0x00b6_31d0));
    }

    #[test]
    fn release_housekeeping_clears_the_cached_reference_of_each_cell() {
        let (mut r, _, _) = release_rig();
        r.e.set_global(WORLD_OBJECT, 0x6100u32);
        // A 2 x 2 grid of slots holding cells 0x7000, none, 0x7002, 0x7003.
        let slots = r.object(0x20);
        for (i, cell) in [0x7000u32, 0, 0x7002, 0x7003].iter().enumerate() {
            r.e.mem.set_u32(slots + 4 * i as u32, *cell);
        }
        r.set(0x0084_e3a0, &[2]);
        r.e.register_double(0x0045_7050, move |_, a| Ret {
            eax: slots + 4 * (a[1] * 2 + a[2]),
            ..Ret::default()
        });
        // The cells' second out values: 0x8000 (with data), 0x8010 (without).
        let with_data = r.object(0x20);
        r.e.mem.set_u32(with_data + 4, 0x1234);
        let without_data = r.object(0x20);
        let untouched = r.object(0x20);
        r.e.mem.set_u32(untouched + 4, 0x55);
        r.e.register_double(0x0054_77f0, move |e, a| {
            let (first, second) = match a[0] {
                0x7000 => (1, with_data),
                0x7002 => (0, untouched),
                _ => (1, without_data),
            };
            e.mem.set_u32(a[1], first);
            e.mem.set_u32(a[2], second);
            Ret::default()
        });
        let (_, log) = r.run(0x0087_2570, &args![1u32]);
        let asked: Vec<u32> = calls_to(&log, 0x0054_77f0).iter().map(|c| c[0]).collect();
        assert_eq!(asked, vec![0x7000, 0x7002, 0x7003]);
        assert_eq!(
            calls_to(&log, 0x0045_7050),
            vec![
                vec![0x6100, 0, 0],
                vec![0x6100, 0, 1],
                vec![0x6100, 1, 0],
                vec![0x6100, 1, 1]
            ]
        );
        assert_eq!(r.e.mem.u32(with_data + 4), 0);
        assert_eq!(r.e.mem.u32(untouched + 4), 0x55);
    }

    #[test]
    fn release_housekeeping_resizes_the_window_to_the_display() {
        let (mut r, _, _) = release_rig();
        r.set(0x0044_6e10, &[1]);
        r.set(0x004d_c1f0, &[1280]);
        r.set(0x004d_c200, &[720]);
        r.set(IMPORT_GET_WINDOW_LONG, &[0x16cf_0000]);
        let seen: Rc<RefCell<Vec<[i32; 4]>>> = Rc::new(RefCell::new(vec![]));
        {
            let seen = seen.clone();
            r.e.register_double(IMPORT_ADJUST_WINDOW_RECT_EX, move |e, a| {
                seen.borrow_mut().push([
                    e.mem.i32(a[0]),
                    e.mem.i32(a[0] + 4),
                    e.mem.i32(a[0] + 8),
                    e.mem.i32(a[0] + 12),
                ]);
                // The rectangle grows by the frame.
                e.mem.set_i32(a[0], -8);
                e.mem.set_i32(a[0] + 4, -31);
                e.mem.set_i32(a[0] + 8, 1288);
                e.mem.set_i32(a[0] + 12, 727);
                Ret::default()
            });
        }
        let (_, log) = r.run(0x0087_2570, &args![1u32]);
        assert_eq!(*seen.borrow(), vec![[0, 0, 1280, 720]]);
        assert_eq!(
            calls_to(&log, IMPORT_GET_WINDOW_LONG),
            vec![vec![0x1234, 0xffff_fff0]]
        );
        let adjust = calls_to(&log, IMPORT_ADJUST_WINDOW_RECT_EX);
        assert_eq!(adjust[0][1..], [0x16cf_0000, 0, 0]);
        // As compiled: x = left, y = bottom, cx = right - left, cy = top - bottom.
        assert_eq!(
            calls_to(&log, IMPORT_SET_WINDOW_POS),
            vec![vec![
                0x1234,
                0,
                (-8i32) as u32,
                727,
                1296,
                (-31i32 - 727) as u32,
                0
            ]]
        );
        // Without the window request nothing is resized.
        r.set(0x0044_6e10, &[0]);
        let (_, log) = r.run(0x0087_2570, &args![1u32]);
        assert!(!addrs(&log).contains(&IMPORT_SET_WINDOW_POS));
    }

    #[test]
    fn current_movie_and_inner_flag_accessors() {
        let mut r = Rig::new();
        let outer = r.object(0x40);
        assert_eq!(r.e.call(0x0087_2780, &args![outer]).u8(), 0);
        let inner = r.object(0x40);
        r.e.mem.set_u32(outer + 0xc, inner);
        assert_eq!(r.e.call(0x0087_2780, &args![outer]).u8(), 0);
        r.e.mem.set_u32(inner + 0x20, 0x1234);
        assert_eq!(r.e.call(0x0087_2780, &args![outer]).u8(), 1);
        assert_eq!(r.e.call(0x0087_27b0, &args![inner]).u8(), 1);
        r.e.mem.set_u32(inner + 0x20, 0);
        assert_eq!(r.e.call(0x0087_27b0, &args![inner]).u8(), 0);
    }

    #[test]
    fn stage_reset_only_advances_when_the_water_setting_is_off() {
        let mut r = Rig::new();
        let cell = r.object(0x40);
        r.e.mem.set_i32(cell, 3);
        r.set(0x0070_ec90, &[cell]);
        let this = r.object(0x10);
        let (_, log) = r.run(0x0087_27d0, &args![this]);
        let expected: Vec<u32> = (4..0x11).collect();
        assert_eq!(stages(&log, MT_SET_THREAD_STAGE), expected);
        assert_eq!(stages(&log, MT_ADVANCE_THREAD_STAGE), expected);
        assert_eq!(
            addrs(&log)[..3],
            [SETTING_BYTE_PTR, 0x0070_ec90, GET_MT_SYSTEM]
        );
    }

    #[test]
    fn stage_reset_draws_with_the_flagged_object_when_the_cell_is_usable() {
        let mut r = Rig::new();
        let cell = r.object(0x40);
        let this = r.object(0x10);
        r.set(0x0070_ec90, &[cell]);
        r.settings(SETTING_BYTE_PTR, &[(0x011c_7adc, 1)]);
        r.e.set_global(0x011a_9608, 0x55u32);
        let indexed = r.object(0x10);
        r.set(INDEXED_GLOBAL, &[indexed]);
        r.set(0x005f_9ad0, &[0]);
        r.set(GET_ROOT_CHILD_OF_ROOT, &[0x4000]);
        let during: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(vec![]));
        {
            let during = during.clone();
            r.e.register_double(0x004e_1bc0, move |e, _| {
                during.borrow_mut().push(e.global::<u32>(0x011a_9608));
                Ret::default()
            });
        }
        let (_, log) = r.run(0x0087_27d0, &args![this]);
        assert_eq!(*during.borrow(), vec![1]);
        assert_eq!(r.e.global::<u32>(0x011a_9608), 0x55);
        assert_eq!(calls_to(&log, 0x0054_68f0), vec![vec![0xb]]);
        assert_eq!(calls_to(&log, 0x005f_9ad0), vec![vec![indexed]]);
        assert_eq!(
            calls_to(&log, 0x005f_9af0),
            vec![vec![indexed, 1], vec![indexed, 0]]
        );
        assert_eq!(
            calls_to(&log, 0x004e_1bc0),
            vec![vec![cell, 0x4000, indexed]]
        );
        // The first path does not reset the water state.
        assert!(!addrs(&log).contains(&0x004e_2120));
    }

    #[test]
    fn stage_reset_takes_the_first_path_for_a_world_with_the_check_true() {
        let mut r = Rig::new();
        let cell = r.object(0x40);
        let this = r.object(0x10);
        r.set(0x0070_ec90, &[cell]);
        r.settings(SETTING_BYTE_PTR, &[(0x011c_7adc, 1)]);
        r.set(0x005f_36f0, &[0x6000]);
        r.set(0x0045_18e0, &[1]);
        let indexed = r.object(0x10);
        r.set(INDEXED_GLOBAL, &[indexed]);
        let (_, log) = r.run(0x0087_27d0, &args![this]);
        assert_eq!(calls_to(&log, 0x0045_18e0), vec![vec![0x6000]]);
        assert_eq!(calls_to(&log, 0x004e_1bc0).len(), 1);
        // The check false: the other path.
        r.set(0x0045_18e0, &[0]);
        let (_, log) = r.run(0x0087_27d0, &args![this]);
        assert!(calls_to(&log, 0x004e_1bc0).is_empty());
        assert_eq!(calls_to(&log, 0x004e_2120).len(), 1);
    }

    #[test]
    fn stage_reset_clears_the_water_state_when_the_check_blocks() {
        let mut r = Rig::new();
        let cell = r.object(0x40);
        r.e.mem.set_i32(cell, 15);
        let this = r.object(0x10);
        r.set(0x0070_ec90, &[cell]);
        r.settings(SETTING_BYTE_PTR, &[(0x011c_7adc, 1)]);
        r.set(0x0054_68f0, &[1]);
        let main_object = r.object(0x40);
        r.e.set_global(MAIN_OBJECT, main_object);
        r.set(0x004e_2190, &[0x6600]);
        r.e.set_global(0x011f_f104, 1u8);
        r.e.set_global(0x011f_f8c4, 1u8);
        r.e.set_global(0x011c_7a59, 5u8);
        let (_, log) = r.run(0x0087_27d0, &args![this]);
        assert_eq!(r.e.global::<u8>(0x011f_f104), 0);
        assert_eq!(r.e.global::<u8>(0x011f_f8c4), 0);
        assert_eq!(r.e.global::<u8>(0x011c_7a59), 0);
        // The first word of the `0070ec90` object is cleared, so the final
        // stage loop starts at 1.
        assert_eq!(r.e.mem.u32(cell), 0);
        assert_eq!(calls_to(&log, 0x004e_2190), vec![vec![main_object]]);
        assert_eq!(calls_to(&log, 0x004e_2120), vec![vec![0x6600, 0]]);
        assert_eq!(
            stages(&log, MT_SET_THREAD_STAGE),
            (1..0x11).collect::<Vec<u32>>()
        );
        // `005f36f0` was not asked: the stage check already blocked.
        assert!(!addrs(&log).contains(&0x005f_36f0));
        assert_eq!(r.e.global::<u32>(0x011a_9608), 0);
    }

    #[test]
    fn small_stores_write_their_globals() {
        let mut r = Rig::new();
        r.e.call(0x0087_2930, &args![7u32]);
        assert_eq!(r.e.global::<u8>(0x011c_7a59), 7);
        r.e.call(0x0087_2dd0, &args![3u32, 4u32]);
        assert_eq!(r.e.global::<u8>(0x011f_9fc3), 3);
        assert_eq!(r.e.global::<u8>(0x011f_9fc4), 4);
        r.e.call(0x0087_2df0, &args![9u32]);
        assert_eq!(r.e.global::<u8>(0x011f_9fc5), 9);
    }

    #[test]
    fn interface_frame_calls_the_update_of_each_visible_menu() {
        let mut r = Rig::new();
        let this = r.object(0x200);
        r.set(GET_GLOBAL_011F4748, &[0x1000]);
        r.set(GET_GLOBAL_011F91AC, &[0x2000]);
        r.set(0x004d_c310, &[1]);
        r.e.register_double(IS_MENU_ID_VISIBLE, |_, a| Ret {
            eax: [0x41e, 0x432, 0x424].contains(&a[0]) as u32,
            ..Ret::default()
        });
        let (_, log) = r.run(0x0087_2940, &args![this, 0u32]);
        assert_eq!(
            only(
                &log,
                &[
                    0x0070_9ae0,
                    0x0070_9af0,
                    0x0070_9b00,
                    0x0070_9b10,
                    0x0070_9b20,
                    0x0070_9b30,
                    0x0079_48d0,
                    0x007c_9ca0
                ]
            ),
            vec![0x0070_9ae0, 0x0079_48d0, 0x007c_9ca0]
        );
        // The menu ids are asked in order, then 0x424 at the end.
        let asked: Vec<u32> = calls_to(&log, IS_MENU_ID_VISIBLE)
            .iter()
            .map(|c| c[0])
            .collect();
        assert_eq!(
            asked,
            vec![0x41e, 0x3f6, 0x438, 0x439, 0x43a, 0x43b, 0x432, 0x424]
        );
        assert_eq!(
            calls_to(&log, 0x0087_4b90),
            vec![vec![this, 0x2000, 0x1000, 1, 0]]
        );
        // The frame is built for the default target (no screen given).
        assert_eq!(calls_to(&log, 0x00b6_b890), vec![vec![7, 0]]);
        assert_eq!(
            calls_to(&log, 0x0087_6850),
            vec![vec![this, 0, 0x2000, 0x1000, 0]]
        );
        assert_eq!(*addrs(&log).last().unwrap(), 0x007c_9ca0);
    }

    #[test]
    fn interface_frame_without_menus_only_finishes() {
        let mut r = Rig::new();
        let this = r.object(0x200);
        r.set(GET_GLOBAL_011F4748, &[0x1000]);
        r.set(GET_GLOBAL_011F91AC, &[0x2000]);
        let (_, log) = r.run(0x0087_2940, &args![this, 0x5000u32]);
        assert!(!addrs(&log).contains(&0x007c_9ca0));
        assert!(!addrs(&log).contains(&0x0070_9ae0));
        // A target is cleared with `00876830` (byte flag 0, no overlay flag).
        assert_eq!(calls_to(&log, 0x0087_6830), vec![vec![this, 0, 0x5000]]);
        // With the overlay flag the overlay draw replaces it.
        r.e.set_global(0x011f_91a4, 1u8);
        let (_, log) = r.run(0x0087_2940, &args![this, 0x5000u32]);
        assert!(!addrs(&log).contains(&0x0087_6830));
        assert_eq!(calls_to(&log, 0x0087_5fd0).len(), 1);
    }

    #[test]
    fn interface_prelude_runs_only_when_the_object_exists() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        let (_, log) = r.run(0x0087_2ad0, &args![this]);
        assert_eq!(addrs(&log), vec![0x0068_3a60]);
        r.set(0x0068_3a60, &[1]);
        let (_, log) = r.run(0x0087_2ad0, &args![this]);
        assert_eq!(addrs(&log), vec![0x0068_3a60, 0x0071_4a00, 0x0071_4960]);
        assert_eq!(calls_to(&log, 0x0071_4960), vec![vec![3]]);
        r.set(0x0071_4a00, &[1]);
        let (_, log) = r.run(0x0087_2ad0, &args![this]);
        assert_eq!(
            addrs(&log),
            vec![0x0068_3a60, 0x0071_4a00, 0x00a8_1a80, 0x0071_4960]
        );
    }

    /// The rig of the per-frame setup: a root, its child and a `Main`.
    fn setup_rig() -> (Rig, u32) {
        let mut r = Rig::new();
        let this = r.object(0x200);
        r.set(GET_ROOT, &[0x3000]);
        r.set(GET_ROOT_CHILD, &[0x4000]);
        r.set(0x0044_ddc0, &[4]);
        (r, this)
    }

    #[test]
    fn per_frame_setup_sets_the_color_and_the_field_of_view() {
        let (mut r, this) = setup_rig();
        r.set_float(0x0071_0ab0, 1.25);
        r.e.set_global(PLAYER_OBJECT, 0x5100u32);
        let (ret, log) = r.run(0x0087_2b00, &args![this]);
        assert_eq!(ret.u32(), 0);
        assert_eq!(calls_to(&log, SET_COLOR)[0], vec![0x4000, 0x011a_d840]);
        assert_eq!(calls_to(&log, 0x0044_ddc0)[0], vec![0x011f_2250]);
        // The mode is 4: no field of view on the root, but `00b54000` runs.
        assert!(calls_to(&log, 0x00c5_2020).is_empty());
        assert_eq!(calls_to(&log, 0x0071_0ab0), vec![vec![0x5100]]);
        assert_eq!(calls_to(&log, 0x00b5_4000), vec![vec![word(1.25)]]);
        // Any other mode sets it on the root first.
        r.set(0x0044_ddc0, &[3]);
        let (_, log) = r.run(0x0087_2b00, &args![this]);
        assert_eq!(
            calls_to(&log, 0x00c5_2020),
            vec![vec![0x3000, word(1.25), 0, 0, 1]]
        );
        assert_eq!(calls_to(&log, 0x0071_0ab0).len(), 2);
    }

    #[test]
    fn per_frame_setup_marks_the_rendering_menu_with_a_color() {
        let (mut r, this) = setup_rig();
        let (_, log) = r.run(0x0087_2b00, &args![this]);
        assert!(calls_to(&log, COLOR_CTOR).is_empty());
        r.e.mem.set_u8(this + 0x9c, 1);
        let (_, log) = r.run(0x0087_2b00, &args![this]);
        let ctor = calls_to(&log, COLOR_CTOR);
        assert_eq!(ctor.len(), 1);
        assert_eq!(ctor[0][1..], [0, word(1.0), word(1.0), 0]);
        assert_eq!(calls_to(&log, SET_COLOR)[1], vec![0x4000, ctor[0][0]]);
    }

    #[test]
    fn per_frame_setup_builds_the_actor_culler_once() {
        let (mut r, this) = setup_rig();
        r.settings(SETTING_BYTE_PTR, &[(0x011d_ed80, 1)]);
        r.pointers(&[(ROOT_POINTER, 0x3100)]);
        let (_, log) = r.run(0x0087_2b00, &args![this]);
        // Construction, then the `atexit` of its destructor, then the call.
        assert_eq!(
            only(&log, &[0x005d_8b10, 0x0063_3c90, 0x00ec_658f, 0x008d_6f50]),
            vec![
                0x005d_8b10,
                0x0063_3c90,
                0x0063_3c90,
                0x0063_3c90,
                0x00ec_658f,
                0x008d_6f50
            ]
        );
        assert_eq!(calls_to(&log, 0x005d_8b10), vec![vec![0x011d_ef10]]);
        assert_eq!(calls_to(&log, 0x00ec_658f), vec![vec![0x00fd_9380]]);
        assert_eq!(calls_to(&log, 0x008d_6f50), vec![vec![0x011d_ef10, 0x4000]]);
        assert_eq!(r.e.mem.u32(0x011d_ef10), 0x0108_2cc0);
        assert_eq!(r.e.mem.u32(0x011d_ef18), 0x0108_2ccc);
        assert_eq!(r.e.global::<u32>(0x011d_efb4) & 1, 1);
        // The second frame only calls the method.
        let (_, log) = r.run(0x0087_2b00, &args![this]);
        assert!(!addrs(&log).contains(&0x005d_8b10));
        assert!(!addrs(&log).contains(&0x00ec_658f));
        assert_eq!(calls_to(&log, 0x008d_6f50).len(), 1);
    }

    #[test]
    fn per_frame_setup_resets_the_two_value_holders() {
        let (mut r, this) = setup_rig();
        r.pointers(&[
            (ROOT_POINTER, 0x3100),
            (0x011d_eb34, 0xaaa0),
            (0x011d_eda4, 0xbbb0),
        ]);
        r.set(0x0055_8310, &[0x3200]);
        r.set(0x0043_c490, &[0x3300]);
        let (_, log) = r.run(0x0087_2b00, &args![this]);
        assert_eq!(
            calls_to(&log, 0x0055_8310),
            vec![vec![0x3100], vec![0x3100]]
        );
        assert_eq!(
            calls_to(&log, 0x0043_c490),
            vec![vec![0x3200], vec![0x3200]]
        );
        assert_eq!(
            calls_to(&log, 0x0044_0460),
            vec![vec![0xaaa0, 0x3300], vec![0xbbb0, 0x3300]]
        );
        let points = calls_to(&log, POINT3_CTOR);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0][1..], [0, 0, 0]);
        assert_eq!(
            calls_to(&log, 0x00a5_9c60),
            vec![vec![0xaaa0, points[0][0]], vec![0xbbb0, points[1][0]]]
        );
    }

    #[test]
    fn per_frame_setup_moves_the_flag_and_stores_the_settings() {
        let (mut r, this) = setup_rig();
        r.e.set_global(0x011f_9fc0, 1u8);
        r.e.set_global(0x011f_94a5, 1u8);
        r.settings(SETTING_BYTE_PTR, &[(0x011d_ecd8, 3), (0x011d_eb60, 4)]);
        r.run(0x0087_2b00, &args![this]);
        assert_eq!(r.e.global::<u8>(0x011f_9fc0), 0);
        assert_eq!(r.e.global::<u8>(0x011f_9fc1), 1);
        assert_eq!(r.e.global::<u8>(0x011f_9fc3), 3);
        assert_eq!(r.e.global::<u8>(0x011f_9fc4), 1);
        assert_eq!(r.e.global::<u8>(0x011f_9fc5), 4);
        assert_eq!(r.e.global::<u8>(0x011f_94a5), 0);
        // With the flag clear, `0x011f9fc1` is left alone.
        r.e.set_global(0x011f_9fc1, 7u8);
        r.run(0x0087_2b00, &args![this]);
        assert_eq!(r.e.global::<u8>(0x011f_9fc1), 7);
    }

    #[test]
    fn per_frame_setup_returns_the_object_of_a_rider_or_follower_mode() {
        let (mut r, this) = setup_rig();
        r.set(0x008d_8520, &[0x6000]);
        r.set(0x0045_cd60, &[0x6100]);
        r.set(0x0047_6c90, &[3]);
        let (ret, log) = r.run(0x0087_2b00, &args![this]);
        assert_eq!(ret.u32(), 0x6100);
        assert_eq!(calls_to(&log, 0x0047_6c90).len(), 1);
        // Mode 2 (asked a second time).
        r.set(0x0047_6c90, &[2]);
        let (ret, log) = r.run(0x0087_2b00, &args![this]);
        assert_eq!(ret.u32(), 0x6100);
        assert_eq!(calls_to(&log, 0x0047_6c90).len(), 2);
        // Any other mode, no object, or no `0045cd60` answer.
        r.set(0x0047_6c90, &[1]);
        assert_eq!(r.run(0x0087_2b00, &args![this]).0.u32(), 0);
        r.set(0x0047_6c90, &[3]);
        r.set(0x0045_cd60, &[0]);
        assert_eq!(r.run(0x0087_2b00, &args![this]).0.u32(), 0);
        r.set(0x0045_cd60, &[0x6100]);
        r.set(0x008d_8520, &[0]);
        assert_eq!(r.run(0x0087_2b00, &args![this]).0.u32(), 0);
    }

    #[test]
    fn actor_culler_constructors_set_their_vtables() {
        let mut r = Rig::new();
        let culler = r.object(0xb0);
        let ret = r.e.call(0x0087_2e00, &args![culler]);
        assert_eq!(ret.u32(), culler);
        assert_eq!(r.e.mem.u32(culler), 0x0108_2cc0);
        assert_eq!(r.e.mem.u32(culler + 8), 0x0108_2ccc);
        let (_, log) = r.run(0x0087_2e00, &args![culler]);
        assert_eq!(
            addrs(&log),
            vec![
                0x005d_8b10,
                0x00a6_9400,
                0x0063_3c90,
                0x0063_3c90,
                0x0063_3c90
            ]
        );
        assert_eq!(calls_to(&log, 0x00a6_9400), vec![vec![culler + 8, 0]]);
        assert_eq!(
            calls_to(&log, 0x0063_3c90),
            vec![
                vec![culler + 0x98, 0],
                vec![culler + 0x9c, 0],
                vec![culler + 0xa0, 0]
            ]
        );
    }

    #[test]
    fn fade_node_culler_constructor_and_destructors() {
        let mut r = Rig::new();
        let culler = r.object(0x40);
        let ret = r.e.call(0x0087_2ea0, &args![culler, 5u32]);
        assert_eq!(ret.u32(), culler);
        assert_eq!(r.e.mem.u32(culler), 0x0108_2ccc);
        let (_, log) = r.run(0x0087_2ea0, &args![culler, 5u32]);
        assert_eq!(log, vec![(0x00a6_9400, vec![culler, 5])]);
        // The destructor body, then the free for an odd flag.
        let (ret, log) = r.run(0x0087_2f00, &args![culler, 1u32]);
        assert_eq!(ret.u32(), culler);
        assert_eq!(
            log,
            vec![(0x00a6_93e0, vec![culler]), (0x0040_1030, vec![culler])]
        );
        let (ret, log) = r.run(0x0087_2f00, &args![culler, 0u32]);
        assert_eq!(ret.u32(), culler);
        assert_eq!(log, vec![(0x00a6_93e0, vec![culler])]);
        let (_, log) = r.run(0x0087_2f00, &args![culler, 2u32]);
        assert_eq!(log, vec![(0x00a6_93e0, vec![culler])]);
        let (_, log) = r.run(0x0087_2f30, &args![culler]);
        assert_eq!(log, vec![(0x00a6_93e0, vec![culler])]);
    }

    #[test]
    fn culling_process_stubs() {
        let mut r = Rig::new();
        let (_, log) = r.run(0x0087_2ed0, &args![0x1000u32, 0x2000u32]);
        assert_eq!(log, vec![(IMPORT_DEBUG_BREAK, vec![])]);
        assert_eq!(r.e.call(0x0087_2ef0, &args![0x1000u32]).u32(), 0x0120_1f7c);
    }

    /// The rig of the target selection: a target whose first buffer
    /// answers `buffer_size`, a renderer answering `display`.
    fn target_rig(display: (u32, u32), buffer_size: (u32, u32)) -> (Rig, u32) {
        let mut r = Rig::new();
        r.e.set_global(0x011f_91a4, 1u8);
        r.set(GET_GLOBAL_011F91A8, &[0x2222]);
        let buffer = r.object_with_slots(&[(0x94, buffer_size.0), (0x98, buffer_size.1)]);
        let target = r.object(0x40);
        r.pointers(&[(target + 0x30, buffer)]);
        r.set(0x00b6_e110, &[target]);
        r.set(0x004d_ee10, &[display.0]);
        r.set(0x004d_ee70, &[display.1]);
        r.set(0x00b6_b260, &[0x6600]);
        r.set(GET_ROOT_CHILD_OF_ROOT, &[0x4000]);
        (r, target)
    }

    #[test]
    fn target_selection_creates_the_target_and_makes_its_group_current() {
        let (mut r, target) = target_rig((1280, 720), (1280, 720));
        let this = r.object(0x10);
        let (ret, log) = r.run(0x0087_2f50, &args![this, 0x1000u32, 0x5000u32, 0u32, 0u32]);
        assert_eq!(ret.u32(), target);
        let guard = calls_to(&log, PROFILE_SCOPE_CTOR)[0].clone();
        assert_eq!(guard[1..], [0x27, 1, SOURCE_FILE_NAME, 0x1e6d]);
        assert_eq!(
            calls_to(&log, 0x00b6_e110),
            vec![vec![0x2222, 0x1000, 4, 0, 0, 0]]
        );
        assert_eq!(calls_to(&log, 0x0087_31d0), vec![vec![target]]);
        assert_eq!(r.e.global::<u32>(0x011f_9438), target);
        assert_eq!(calls_to(&log, 0x00b6_b260), vec![vec![target]]);
        assert_eq!(calls_to(&log, 0x00b6_b8d0), vec![vec![7, 0x6600]]);
        // The sizes match: no color.
        assert!(calls_to(&log, COLOR_CTOR).is_empty());
        assert_eq!(*addrs(&log).last().unwrap(), PROFILE_SCOPE_DTOR);
    }

    #[test]
    fn target_selection_sets_the_ratios_when_the_width_or_height_differs() {
        for display in [(1920, 720), (1280, 1080)] {
            let (mut r, _) = target_rig(display, (1280, 720));
            r.e.set_global(0x0108_2d20, 1.0f64);
            let calls = Rc::new(RefCell::new(0));
            {
                let calls = calls.clone();
                r.e.register_double(FLOAT_HELPER_C, move |_, _| {
                    let mut calls = calls.borrow_mut();
                    *calls += 1;
                    Ret {
                        st0: if *calls == 1 { 1.25 } else { 0.75 },
                        ..Ret::default()
                    }
                });
            }
            let this = r.object(0x10);
            let (_, log) = r.run(0x0087_2f50, &args![this, 0x1000u32, 0u32, 0u32, 0u32]);
            let ratios = calls_to(&log, FLOAT_HELPER_C);
            assert_eq!(ratios.len(), 2);
            let first = f64::from_bits(ratios[0][0] as u64 | (ratios[0][1] as u64) << 32);
            let second = f64::from_bits(ratios[1][0] as u64 | (ratios[1][1] as u64) << 32);
            assert_eq!(first, display.0 as f64 / 1280.0);
            assert_eq!(second, display.1 as f64 / 720.0);
            assert_eq!(ratios[0][2], word(1.0));
            let color = calls_to(&log, COLOR_CTOR)[0].clone();
            assert_eq!(color[1..], [0, word(1.25), word(0.75), 0]);
            assert_eq!(calls_to(&log, SET_COLOR), vec![vec![0x4000, color[0]]]);
        }
    }

    #[test]
    fn target_selection_scales_the_ratio_by_the_constant() {
        let (mut r, _) = target_rig((1000, 500), (1000, 500));
        // A different size than the display makes the ratios appear.
        r.set(0x004d_ee10, &[2000]);
        r.e.set_global(0x0108_2d20, 0.5f64);
        let this = r.object(0x10);
        let (_, log) = r.run(0x0087_2f50, &args![this, 0x1000u32, 0u32, 0u32, 0u32]);
        let ratios = calls_to(&log, FLOAT_HELPER_C);
        let first = f64::from_bits(ratios[0][0] as u64 | (ratios[0][1] as u64) << 32);
        assert_eq!(first, 2000.0 * 0.5 / 1000.0);
    }

    #[test]
    fn target_selection_without_a_new_target_picks_a_group() {
        let mut r = Rig::new();
        let this = r.object(0x10);
        r.set(0x00b6_b260, &[0x6600]);
        r.object_at(0x1000, &[(0xc8, 0x6c00)]);
        // The new-target flag stops the creation even with `008709f0`.
        r.e.set_global(0x011f_91a4, 1u8);
        let (ret, log) = r.run(0x0087_2f50, &args![this, 0x1000u32, 0x5000u32, 1u32, 0u32]);
        assert_eq!(ret.u32(), 0);
        let guard = calls_to(&log, PROFILE_SCOPE_CTOR)[0].clone();
        assert_eq!(guard[1..], [0x26, 1, SOURCE_FILE_NAME, 0x1e84]);
        assert_eq!(calls_to(&log, 0x00b6_b260), vec![vec![0x5000]]);
        assert_eq!(calls_to(&log, 0x00b6_b8d0), vec![vec![7, 0x6600]]);
        assert!(!addrs(&log).contains(&0x00b6_e110));
        // The renderer's own group.
        let (_, log) = r.run(0x0087_2f50, &args![this, 0x1000u32, 0x5000u32, 1u32, 1u32]);
        assert_eq!(calls_to(&log, 0x00b6_b8d0), vec![vec![7, 0x6c00]]);
        assert!(!addrs(&log).contains(&0x00b6_b260));
        // No screen: the default group.
        let (_, log) = r.run(0x0087_2f50, &args![this, 0x1000u32, 0u32, 1u32, 1u32]);
        assert_eq!(calls_to(&log, 0x00b6_b890), vec![vec![7, 0]]);
        assert!(!addrs(&log).contains(&0x00b6_b8d0));
        // `008709f0` clear: also the second branch.
        r.e.set_global(0x011f_91a4, 0u8);
        let (_, log) = r.run(0x0087_2f50, &args![this, 0x1000u32, 0u32, 0u32, 0u32]);
        assert_eq!(calls_to(&log, 0x00b6_b890), vec![vec![7, 0]]);
        assert!(!addrs(&log).contains(&0x00b6_e110));
    }

    // END TESTS
}
