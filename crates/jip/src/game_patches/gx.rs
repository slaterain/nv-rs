//! Group GX's game patches (step 3.9: Havok, pad rumble, mouse): its handlers, wraps and slot routines. Only group
//! GX's PR edits this file (and its own tests file beside it).
//!
//! `internal/patches_game.h` and `internal/hooks.h` (JIP LN NVSE):
//! - **G29** (a fix; `MarkCreatureNoFallHook` at `0x9313EC`, `SAFE_WRITE_BUF(0xCD3FDD, ..)`): an
//!   immobile creature (its base's flag `+0x36` bit 7) gets JIP's no-fall flag on its character
//!   controller (`+0x414` bit 30) when its Havok is set up, and the in-air state treats that flag as
//!   the exe's bit 8 (the wanted state reset, flag 0x80): it no longer falls to the ground. The rest
//!   of JIP's block runs the exe's AI start (`008e4e50`) as JIP tests it.
//! - **G32** (a fix; `SafeWrite32(0xC65B47, 0x38F)`, `SAFE_WRITE_BUF(0xC65B5B, ..)`): a fixed Havok
//!   body follows its node when its collision object asks for the move (`+0xC` bit 6), in place of
//!   the exe's "only outside a Havok world"; one with no Havok object ends at once.
//! - **G49** (JIP's own; `SafeWrite32(0x939258, 0xD54A183)`): the player's constructor clears the
//!   three bytes after `bMenuModeButtonClicked` too (O14's state byte `+0xD57` among them).
//! - **G59** (a fix; `GamePadRumbleHook` at `0xEE820A`): no pad rumble (`XInputSetState`) while
//!   `bGamePadRumble` is off.
//! - **G65** (a fix; `JumpHeightFixHook` at `0xCD4468`): the fall start height of a jump is the
//!   controller's own jump height (in Havok units) above the extracted height, and the jump goes
//!   straight to the wanted state 0xB with the state word 2, without the exe's state update.
//! - **H20** (JIP's own, `InitJIPHooks`; `GetMouseMovementHook` at `0xA239E0`): the mouse axes
//!   `ToggleMouseMovement` turns off read 0, and the wheel reads 0 while its key (wheel up `0x108`,
//!   down `0x109`) is disabled; with the command built here.
//! - **O14** (`bSneakBoundingBoxFix`, on by default; `InitControllerShapeHook` at `0xC72EA3`,
//!   `SneakBoundingBoxFixHook` at `0x770B0E`): the player and teammates get a controller shape of
//!   JIP's own (made once from the shared one's capsule, `00c70de0`), whose five vertices are
//!   lowered by 6 units while the player sneaks and put back when it stands (the state in the
//!   player's byte `+0xD57`).
//!
//! **Threads.** The Havok seams run on whatever thread sets up or steps a character; the HUD's
//! sneak test on the main thread. Their state is one run-wide lock-free slot ([`GxState`], made at
//! install before the threads), as JIP's statics are one per process; the vertex sets sit behind a
//! lock held only for one copy.

use super::Area;
use crate::commands::{heap_alloc, Handlers};
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// G29's seams: the Havok set-up's no-fall block and the in-air state's bit 8 test.
pub const NO_FALL_MARK: SiteSlot = SiteSlot::new(Owner::Jip, 0x0093_13ec);
pub const NO_FALL_TEST: SiteSlot = SiteSlot::new(Owner::Jip, 0x00cd_3fdd);
/// G32's seam: `NitohkTransform`'s fixed-body test.
pub const FIXED_BODY: SiteSlot = SiteSlot::new(Owner::Jip, 0x00c6_5b5b);
/// G49's seam: the player constructor's `bMenuModeButtonClicked` store.
pub const MENU_MODE_BYTE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0093_9258);
/// G59's wrap: the `XInputSetState` thunk.
pub const PAD_RUMBLE: WrapSlot = WrapSlot::new(Owner::Jip, 0x00ee_820a);
/// G65's seam: the jump's fall start height and state.
pub const JUMP_HEIGHT: SiteSlot = SiteSlot::new(Owner::Jip, 0x00cd_4468);
/// H20's wrap: `OSInputGlobals::GetMouseMovement`.
pub const MOUSE_MOVEMENT: WrapSlot = WrapSlot::new(Owner::Jip, 0x00a2_39e0);
/// O14's seams: the controller's shape copy and the HUD's sneak test.
pub const CONTROLLER_SHAPE: SiteSlot = SiteSlot::new(Owner::Jip, 0x00c7_2ea3);
pub const SNEAK_TEST: SiteSlot = SiteSlot::new(Owner::Jip, 0x0077_0b0e);

/// `ToggleMouseMovement` (`functions_jip/jip_fn_input.h`), built with H20.
pub const TOGGLE_MOUSE_MOVEMENT: u32 = 0x28fa;
#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (TOGGLE_MOUSE_MOVEMENT, toggle_mouse_movement),
];

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GX",
    fix_sites: &[
        SNEAK_TEST.addr(),
        NO_FALL_MARK.addr(),
        FIXED_BODY.addr(),
        CONTROLLER_SHAPE.addr(),
        NO_FALL_TEST.addr(),
        JUMP_HEIGHT.addr(),
    ],
    fix_wraps: &[PAD_RUMBLE.addr()],
    install_fixes,
    jip_sites: &[MENU_MODE_BYTE.addr()],
    jip_wraps: &[MOUSE_MOVEMENT.addr()],
    install_jip,
};

/// JIP's statics of this group, run-wide and lock-free: `s_mouseMovementState` (H20; 7),
/// `s_pcControllerShape` (O14: the guest word holding JIP's controller shape, made at the first
/// use, 0 until then) and `s_shapeVerticesZ` (O14: the five vertices as they are, then lowered;
/// their words).
pub struct GxState {
    mouse_movement: AtomicU32,
    shape_holder: AtomicU32,
    vertices: Mutex<[[u32; 4]; 10]>,
}

impl Default for GxState {
    fn default() -> Self {
        GxState {
            mouse_movement: AtomicU32::new(7),
            shape_holder: AtomicU32::new(0),
            vertices: Mutex::new([[0; 4]; 10]),
        }
    }
}

fn state(e: &mut Engine) -> &GxState {
    e.ext_free::<GxState>()
}

fn install_fixes(e: &mut Engine) {
    e.ext_free::<GxState>();
    e.on_site(NO_FALL_MARK, no_fall_mark);
    e.on_site(NO_FALL_TEST, no_fall_test);
    e.on_site(FIXED_BODY, fixed_body);
    e.on_site(JUMP_HEIGHT, jump_height);
    e.on_site(CONTROLLER_SHAPE, controller_shape);
    e.on_site(SNEAK_TEST, sneak_test);
    e.wrap(PAD_RUMBLE, pad_rumble);
}

fn install_jip(e: &mut Engine) {
    e.ext_free::<GxState>();
    e.on_site(MENU_MODE_BYTE, menu_mode_byte);
    e.wrap(MOUSE_MOVEMENT, mouse_movement);
}

/// `kFormType_TESCreature`, an actor base's flags byte JIP tests (`+0x36`, bit 7), the character
/// controller's flags byte holding JIP's no-fall flag (`+0x417`, bit 6: `iFlags` bit 30).
const CREATURE: u8 = 0x2b;
const BASE_FLAGS_36: u32 = 0x36;
const NO_FALL_BYTE: u32 = 0x417;
/// `Actor`'s byte `0087eeb0` reads (`+0x1B0`), its process (`+0x68`) and the process's word
/// `008e5700` tests (`+0x434`); the AI start JIP calls (`008e4e50`, thiscall: the process, the
/// actor).
const ACTOR_BYTE_1B0: u32 = 0x1b0;
const PROCESS: u32 = 0x68;
const PROCESS_WORD_434: u32 = 0x434;
const AI_START: u32 = 0x008e_4e50;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: MarkCreatureNoFallHook.
/// G29 at `0x9313EC` (`a`: the actor, its character controller), in place of the exe's block up to
/// `0x931431`: nothing for an actor with no base form; an immobile creature's controller (when it
/// has one) gets JIP's no-fall flag; then, when the actor's byte `+0x1B0` is 0 and its process's
/// word `+0x434` is 0, the AI start `008e4e50(process, actor)` (JIP tests the byte and the word
/// directly, and not the exe's `004181e0`). Answers 0 (the exe's block skipped).
fn no_fall_mark(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (actor, controller) = (a[0], a[1]);
    let base = e.mem.u32(actor.wrapping_add(0x20));
    if base == 0 {
        return Some(0);
    }
    if e.mem.u8(base.wrapping_add(4)) == CREATURE
        && e.mem.u8(base.wrapping_add(BASE_FLAGS_36)) & 0x80 != 0
        && controller != 0
    {
        let at = controller.wrapping_add(NO_FALL_BYTE);
        let byte = e.mem.u8(at);
        e.mem.set_u8(at, byte | 0x40);
    }
    if e.mem.u8(actor.wrapping_add(ACTOR_BYTE_1B0)) == 0 {
        let process = e.mem.u32(actor.wrapping_add(PROCESS));
        if e.mem.u32(process.wrapping_add(PROCESS_WORD_434)) == 0 {
            e.call(AI_START, &[process, actor]);
        }
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0xCD3FDD, ..): `TEST EAX, 0x400001`).
/// G29 at `0xCD3FDD` (`a`: the controller, its flags): the block is taken for bit 8 or JIP's
/// no-fall bit 30 (the shifted flags' bits 0 and 22). The block itself (the wanted state 0, flag
/// 0x80) stays the translation's: JIP's `OR byte` sets the same bit as the exe's `OR dword`.
fn no_fall_test(_: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(u32::from((a[1] >> 8) & 0x0040_0001 != 0))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite32(0xC65B47, 0x38F); SAFE_WRITE_BUF(0xC65B5B, ..)).
/// G32 (`a`: the collision object, the body's Havok object): 1 (it ends) with no Havok object
/// (the jump at `0xC65B45` now goes to the end), else 1 unless the collision object's bit 6
/// (`+0xC`) asks for the move. JIP does not call `00c87a40`.
fn fixed_body(e: &mut Engine, a: &[u32]) -> Option<u32> {
    if a[1] == 0 {
        return Some(1);
    }
    Some(u32::from(e.mem.u8(a[0].wrapping_add(0xc)) & 0x40 == 0))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite32(0x939258, 0xD54A183): `AND dword ptr [ECX+0xD54], 0`).
/// G49 (`a`: the player): the word at `+0xD54` cleared (the exe's store cleared its first byte).
fn menu_mode_byte(e: &mut Engine, a: &[u32]) -> Option<u32> {
    e.mem.set_u32(a[0].wrapping_add(0xd54), 0);
    Some(0)
}

/// `bGamePadRumble:Controls`' value byte.
const GAME_PAD_RUMBLE: u32 = 0x011e_0854;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GamePadRumbleHook.
/// G59 (`stdcall (user, vibration)`): 0 (`ERROR_SUCCESS`, nothing sent) while `bGamePadRumble`
/// is off; else the thunk's import (`XInputSetState`).
fn pad_rumble(e: &mut Engine, w: &[u32]) -> Ret {
    if e.mem.u8(GAME_PAD_RUMBLE) == 0 {
        return Ret::default();
    }
    e.call_next(PAD_RUMBLE.addr(), w)
}

/// `PS_HKUnitCnvrt`'s lanes (JIP's `kPackedValues`): `0x40DFF8D6`.
const HK_UNIT: u32 = 0x40df_f8d6;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: JumpHeightFixHook.
/// G65 (`a`: the controller, the extracted height's bits), in place of `0xCD4468..0xCD4486`: the
/// state word `+0x3F0` 2, the wanted state `+0x520` 0xB, and `fFallStartHeight` (`+0x544`) the jump
/// height (`+0x540`) times `PS_HKUnitCnvrt` plus the height, in single floats (no state update).
fn jump_height(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let controller = a[0];
    e.mem.set_u32(controller.wrapping_add(0x3f0), 2);
    e.mem.set_u32(controller.wrapping_add(0x520), 0xb);
    let jump = e.mem.f32(controller.wrapping_add(0x540));
    let start = jump * f32::from_bits(HK_UNIT) + f32::from_bits(a[1]);
    e.mem.set_f32(controller.wrapping_add(0x544), start);
    Some(0)
}

/// `OSInputGlobals`' mouse words: x, y (`+0x1B24`, `+0x1B28`) and the wheel (`+0x1B2C`).
const MOUSE_AXES: u32 = 0x1b24;
const MOUSE_WHEEL: u32 = 0x1b2c;
/// xNVSE's macro codes of the wheel moved up and down (`kMacro_MouseButtonOffset` + 8, + 9).
const WHEEL_UP: u32 = 0x108;
const WHEEL_DOWN: u32 = 0x109;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: GetMouseMovementHook.
/// H20 (thiscall: the input globals, the type): types above 3 read 0. 3, the wheel: its word, but
/// 0 while the key of its direction is disabled (up for a positive word, down for a negative).
/// Others: 0 unless `s_mouseMovementState`'s bit `type - 1` (taken modulo 32, as `BT` does) is
/// set, else the word at `+0x1B24 + (type - 1) * 4`.
fn mouse_movement(e: &mut Engine, w: &[u32]) -> Ret {
    let (this, kind) = (w[0], w[1]);
    let value = if kind > 3 {
        0
    } else if kind == 3 {
        let wheel = e.mem.u32(this.wrapping_add(MOUSE_WHEEL));
        if wheel == 0 {
            0
        } else {
            let key = if (wheel as i32) < 0 {
                WHEEL_DOWN
            } else {
                WHEEL_UP
            };
            if crate::input::is_key_disabled(e, key) {
                0
            } else {
                wheel
            }
        }
    } else {
        let index = kind.wrapping_sub(1);
        let on = state(e).mouse_movement.load(Ordering::Relaxed);
        if on >> (index & 31) & 1 == 0 {
            0
        } else {
            e.mem.u32(
                this.wrapping_add(MOUSE_AXES)
                    .wrapping_add(index.wrapping_mul(4)),
            )
        }
    };
    Ret {
        eax: value,
        ..Ret::default()
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_input.h:
// Cmd_ToggleMouseMovement_Execute.
/// `ToggleMouseMovement [flags]` (opcode `0x28FA`): answers the state's bits 0 and 1 (the x and y
/// axes on); with a read argument the state becomes it with bit 2.
fn toggle_mouse_movement(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let before = state(e).mouse_movement.load(Ordering::Relaxed);
    a.set_result(e, f64::from((before & 3) as i32));
    if crate::commands::jip_fn_utility::num_args(e, &a) != 0 {
        let read = Read::of(e, &a);
        if read.ok {
            let toggle = read.int(0, 0);
            state(e).mouse_movement.store(toggle | 4, Ordering::Relaxed);
        }
    }
    handled(true)
}

/// The game's shared controller shape (`NiPointer` at `01267be0`), the shape's Havok object
/// (`+8`), its vertices (`+0x40`, five 16-byte vectors from `+0x20`, `0x30` apart), the shape maker
/// (`00c70de0`, cdecl: vertices `A` and `B`, radius, step height).
const SHARED_SHAPE: u32 = 0x0126_7be0;
const MAKE_SHAPE: u32 = 0x00c7_0de0;
/// The cinfo's owner (`+0x80`), its reference (`+0xCC`), a reference's form id (`+0xC`, the
/// player's 0x14) and teammate byte (`+0x18D`).
const INFO_OWNER: u32 = 0x80;
const OWNER_REFERENCE: u32 = 0xcc;
/// The sneak shape's state byte in the player (`+0xD57`), its process word JIP tests (`+0x190`,
/// `+0x94` masked with 0xC00: 0x400 sneaking).
const SNEAK_STATE: u32 = 0xd57;

/// The shape's five vertices' first word.
fn shape_vertices(e: &Engine, shape: u32) -> u32 {
    let hk = e.mem.u32(shape.wrapping_add(8));
    e.mem.u32(hk.wrapping_add(0x40)).wrapping_add(0x20)
}

/// `s_pcControllerShape`'s guest word: made (the game's heap, cleared) at the first ask.
fn shape_holder(e: &mut Engine) -> u32 {
    let held = state(e).shape_holder.load(Ordering::Acquire);
    if held != 0 {
        return held;
    }
    let word = heap_alloc(e, 4);
    e.mem.set_u32(word, 0);
    match state(e)
        .shape_holder
        .compare_exchange(0, word, Ordering::AcqRel, Ordering::Acquire)
    {
        Ok(_) => word,
        Err(other) => {
            crate::commands::heap_free(e, word);
            other
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitControllerShapeHook.
/// O14 at `0xC72EA3` (`a`: the cinfo, the frame's vertices `A` and `B`, the radius's and the step
/// height's bits): the `NiPointer` the controller's shape is copied from. The shared one
/// (`01267be0`) unless the cinfo's owner's reference is the player (form id 0x14) or a teammate;
/// for those JIP's own (`s_pcControllerShape`): made at the first such ask from the frame's
/// capsule (`00c70de0`), after the shared shape's five vertices are kept as they are and lowered
/// (each lane not below 0 less 6.0, in single floats); `NiReplaceObject` counts the new shape's
/// reference.
fn controller_shape(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let owner = e.mem.u32(a[0].wrapping_add(INFO_OWNER));
    if owner == 0 {
        return Some(SHARED_SHAPE);
    }
    let reference = e.mem.u32(owner.wrapping_add(OWNER_REFERENCE));
    if e.mem.u32(reference.wrapping_add(0xc)) != 0x14
        && e.mem.u8(reference.wrapping_add(0x18d)) == 0
    {
        return Some(SHARED_SHAPE);
    }
    let holder = shape_holder(e);
    if e.mem.u32(holder) != 0 {
        return Some(holder);
    }
    let shared = e.mem.u32(SHARED_SHAPE);
    let from = shape_vertices(e, shared);
    let mut sets = [[0u32; 4]; 10];
    for vertex in 0..5u32 {
        let kept: [u32; 4] =
            std::array::from_fn(|lane| e.mem.u32(from + vertex * 0x30 + lane as u32 * 4));
        // `COMISS` with 0.0 and `JB`: below 0 or unordered keeps the lane.
        let lowered = kept.map(|word| {
            let value = f32::from_bits(word);
            if value >= 0.0 {
                (value - 6.0).to_bits()
            } else {
                word
            }
        });
        sets[vertex as usize] = kept;
        sets[5 + vertex as usize] = lowered;
    }
    *state(e).vertices.lock().unwrap_or_else(|p| p.into_inner()) = sets;
    let shape = e.call(MAKE_SHAPE, &[a[1], a[2], a[3], a[4]]).u32();
    replace_object(e, holder, shape);
    Some(holder)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: NiReplaceObject.
/// The `NiPointer` at `holder` set to `object`: nothing when it holds it already; the held
/// object's reference count down (deleted through its virtual `+4` at 0), `object`'s up.
fn replace_object(e: &mut Engine, holder: u32, object: u32) {
    let held = e.mem.u32(holder);
    if held == object {
        return;
    }
    if held != 0 && refs_add(e, held, -1) <= 0 {
        let vtable = e.mem.u32(held);
        let delete = e.mem.u32(vtable.wrapping_add(4));
        e.call(delete, &[held]);
    }
    e.mem.set_u32(holder, object);
    if object != 0 {
        refs_add(e, object, 1);
    }
}

/// `lock inc` / `lock dec` of an `NiRefObject`'s count (`+4`): the new count.
fn refs_add(e: &mut Engine, object: u32, by: i32) -> i32 {
    let at = object.wrapping_add(4);
    match e.mem.atomic_u32(at) {
        Some(count) => (count.fetch_add(by as u32, Ordering::AcqRel) as i32).wrapping_add(by),
        None => {
            let count = (e.mem.u32(at) as i32).wrapping_add(by);
            e.mem.set_u32(at, count as u32);
            count
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SneakBoundingBoxFixHook.
/// O14 at `0x770B0E` (`a`: the player), in place of the exe's sneak call: 0 with no process word
/// (`+0x190`); else sneaking is its `+0x94` masked with 0xC00 being 0x400. When that differs from
/// the state byte (`+0xD57`) and JIP's shape exists, the byte takes it and the shape's five
/// vertices become the lowered set (sneaking) or the kept one. Answers sneaking.
fn sneak_test(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let player = a[0];
    let process = e.mem.u32(player.wrapping_add(0x190));
    if process == 0 {
        return Some(0);
    }
    let sneaking = e.mem.u32(process.wrapping_add(0x94)) & 0xc00 == 0x400;
    let byte = player.wrapping_add(SNEAK_STATE);
    if e.mem.u8(byte) == u8::from(sneaking) {
        return Some(u32::from(sneaking));
    }
    let holder = state(e).shape_holder.load(Ordering::Acquire);
    let shape = if holder != 0 { e.mem.u32(holder) } else { 0 };
    if shape == 0 {
        return Some(u32::from(sneaking));
    }
    e.mem.set_u8(byte, u8::from(sneaking));
    let to = shape_vertices(e, shape);
    let first = if sneaking { 5 } else { 0 };
    let sets = *state(e).vertices.lock().unwrap_or_else(|p| p.into_inner());
    for (vertex, set) in sets[first..first + 5].iter().enumerate() {
        for (lane, word) in set.iter().enumerate() {
            e.mem
                .set_u32(to + vertex as u32 * 0x30 + lane as u32 * 4, *word);
        }
    }
    Some(u32::from(sneaking))
}

#[cfg(test)]
#[path = "gx_tests.rs"]
mod tests;
