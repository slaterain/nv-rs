//! Group GL part 2: JIP's light data (`InitJIPHooks`, `internal/hooks.h` (JIP LN NVSE)), patches of
//! JIP's own, in whenever JIP answers "installed" (every preset; nothing with JIP not installed).
//!
//! - H03 (`SafeWrite32(addr, 0x110)` at twelve `PUSH 0xFC`): every point light is allocated at
//!   0x110 bytes, JIP's light data after the game's 0xFC ([`point_light_size`]): its saved
//!   translation or colour (+0x100, 16 bytes). Inside the game's object JIP uses +0x9E
//!   (`resetTraits`), +0x9F (`extraFlags`), +0xE8 (the light form) and the animation state.
//! - H05 (12 bytes at `0xA68D6B`): a cloned light keeps the word +0x9C (the switch byte, the
//!   type byte, JIP's two bytes) ([`clone_word`]).
//! - H04 (`SafeWrite8(0x465488, 0xCC)`, `SafeWrite32(0x1029018 / 0x1029014, ..)`; JIP skips it
//!   when JohnnyGuitar is loaded, which the port never has): a light form is 0xCC bytes
//!   ([`light_form_size`]) and keeps its editor id: `SetEditorID` puts the form into the game's
//!   editor id map and the map's copy of the text at +0xC8 ([`set_editor_id`]), `GetEditorID`
//!   answers it ([`editor_id`]).
//! - H06 (`WriteRelJump(0x447155, LoadNifRetnNodeHook)`): a model's 3D, once (flag `0x40000000`
//!   on the root), has each point light with a `LIGH_EDID` string extra naming a form's editor id
//!   tied to that form (+0xE8, flag `0x20000000` on the light and every node up to the root), and
//!   the extra removed ([`load_nif`]).
//! - H06's companion (`WriteRelJump(0x50EF46, CreateObjectNodeHook)`, made with H06 in
//!   `InitJIPHooks`): `TESBoundObject::Clone3D` 0050ee50 never shares a model whose root carries
//!   such lights (`0x20000000`) with a reference, it clones it, so each reference's 3D has lights
//!   of its own for H09's attach to list ([`clone_3d`]). JIP's same hook also clones the models
//!   of forms marked by `InsertObject`/`AttachModel` (H08): not built until those commands are.
//! - H07 (`SafeWrite32(0x109DD0C, DestroyNiPointLightHook)`, `NiPointLight`'s deleting
//!   destructor slot): the destructor frees 0x110 bytes, and none for a light of JIP's (flag
//!   `0x20000000`), which the list below frees ([`destroy_point_light`]).
//! - H12 (`WriteRelJump(0x50DE20, UpdateAnimatedLightHook)`, `WriteRelCall(0x8C7C4E / 0x8C7E18,
//!   UpdateAnimatedLightsHook)`): a light form's flicker (flags 0x8, 0x40), pulse (0x80, 0x100)
//!   and JIP's colour shift (0x800, 0x1000) are JIP's animation ([`update_animated_light`],
//!   [`animate`]); after the game's animated lights each frame, JIP's own point lights (the list
//!   [`add_point_lights`] fills: H09's attach, `gp::queued_reference`) are freed when let go, set
//!   from their form again when marked, put into the scene's lights once (a `ShadowSceneLight`)
//!   and animated ([`animated_lights`]).
//!
//! **In with JIP installed, in every preset (the lead's decision, review of #700).** These are
//! JIP's own built-in light handling (`InitJIPHooks`, not a `[GamePatches]` switch), so they go
//! in whenever JIP answers "installed", with the game-patches row on or off: with JIP installed,
//! flickering and pulsing lights use JIP's animation and point lights carry JIP's extra fields in
//! every preset, the original-behaviour one included. This is JIP's own behaviour, not an option.
//! Point lights and light forms are made bigger in place (H03, H04), as JIP makes them, not kept
//! in a port-side record: the light's own code (the clone's member copy, the destructor) carries
//! JIP's fields with the object.
//!
//! **What the renderer receives changes only with JIP installed.** With JIP not installed every
//! seam above is one load and the exe's path; the wrap and the three slot writes are not made.
//! With JIP installed, for the game's own content (no mesh carries `LIGH_EDID`, no form uses
//! flags 0x800/0x1000): a flickering or pulsing light (placed lights, magic lights) is animated
//! by JIP's code instead of the exe's (other dimmer, position and colour values, see
//! `measure/jip/AUDIT-3.9-GL-2.md`); everything else a light has is as before.
//!
//! Threads: the list is shared by every thread's engine (a lock-free slot made at install, a
//! mutex inside): H09's attach adds on the loading threads, the animated lights' step reads it
//! where the frame's task sequence runs (JIP: a plain vector; the port takes the list out under
//! the mutex for the step and puts back what was added meanwhile after it). The slot routines
//! and the wrap keep no state.

use super::super::slots::{self, Group};
use crate::commands::jip_fn_utility::editor_id_lookup_at;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use std::sync::atomic::Ordering;
use std::sync::Mutex;

/// H03: the twelve `PUSH 0xFC` JIP writes 0x110 into (words: none; answered: the size).
pub const POINT_LIGHT_SIZES: [SiteSlot; 12] = [
    SiteSlot::new(Owner::Jip, 0x0050_da73),
    SiteSlot::new(Owner::Jip, 0x0079_2dc0),
    SiteSlot::new(Owner::Jip, 0x007c_6f86),
    SiteSlot::new(Owner::Jip, 0x007f_c4da),
    SiteSlot::new(Owner::Jip, 0x007f_d49f),
    SiteSlot::new(Owner::Jip, 0x0080_ea39),
    SiteSlot::new(Owner::Jip, 0x0081_c095),
    SiteSlot::new(Owner::Jip, 0x009c_6cb6),
    SiteSlot::new(Owner::Jip, 0x009c_6d3d),
    SiteSlot::new(Owner::Jip, 0x00a7_d6e2),
    SiteSlot::new(Owner::Jip, 0x00a7_d7c3),
    SiteSlot::new(Owner::Jip, 0x00b5_cd2a),
];
/// H06: `ModelLoader::LoadFile`'s 3D object (words: the object; the answer is not read).
pub const LOAD_NIF: SiteSlot = SiteSlot::new(Owner::Jip, 0x0044_7155);
/// H04: `CreateFormOfType`'s light form size (words: none; answered: the size).
pub const LIGHT_FORM_SIZE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0046_5488);
/// H12: the two task sequences' animated lights call (words: the `TES` word; answered: done).
pub const ANIMATED_LIGHTS: [SiteSlot; 2] = [
    SiteSlot::new(Owner::Jip, 0x008c_7c4e),
    SiteSlot::new(Owner::Jip, 0x008c_7e18),
];
/// H05: `NiDynamicEffect`'s member copy of the byte +0x9C (words: none; answered: the word).
pub const CLONE_WORD: SiteSlot = SiteSlot::new(Owner::Jip, 0x00a6_8d6b);
/// H06's companion: `Clone3D`'s choice to share a model (words: the model, the form, the reference;
/// answered: 1 to clone).
pub const CLONE_3D: SiteSlot = SiteSlot::new(Owner::Jip, 0x0050_ef46);
/// H12: a light form's animation of its point light (`thiscall` on the form: the light data
/// (its first word the point light) and a word the callers pass as 0; `RET 8`).
pub const UPDATE_ANIMATED_LIGHT: WrapSlot = WrapSlot::new(Owner::Jip, 0x0050_de20);

/// The part's seams, in address order.
pub const SITES: [u32; 18] = [
    LOAD_NIF.addr(),
    LIGHT_FORM_SIZE.addr(),
    POINT_LIGHT_SIZES[0].addr(),
    CLONE_3D.addr(),
    POINT_LIGHT_SIZES[1].addr(),
    POINT_LIGHT_SIZES[2].addr(),
    POINT_LIGHT_SIZES[3].addr(),
    POINT_LIGHT_SIZES[4].addr(),
    POINT_LIGHT_SIZES[5].addr(),
    POINT_LIGHT_SIZES[6].addr(),
    ANIMATED_LIGHTS[0].addr(),
    ANIMATED_LIGHTS[1].addr(),
    POINT_LIGHT_SIZES[7].addr(),
    POINT_LIGHT_SIZES[8].addr(),
    CLONE_WORD.addr(),
    POINT_LIGHT_SIZES[9].addr(),
    POINT_LIGHT_SIZES[10].addr(),
    POINT_LIGHT_SIZES[11].addr(),
];

/// `TESObjectLIGH`'s vtable (`kVtbl_TESObjectLIGH` 0x1028EE4) slots `GetEditorID` (+0x130) and
/// `SetEditorID` (+0x134), which held the shared stubs `00401280` (`""`) and `00401290` (true);
/// `NiPointLight`'s vtable slot 0 (its deleting destructor, `00a75c80`, shared with
/// `NiDirectionalLight`'s vtable).
pub const GET_EDITOR_ID_SLOT: u32 = 0x0102_9014;
pub const SET_EDITOR_ID_SLOT: u32 = 0x0102_9018;
pub const DESTROY_SLOT: u32 = 0x0109_dd0c;
/// The routines written there.
pub const GET_EDITOR_ID_ROUTINE: u32 = slots::routine(Group::GL, 0);
pub const SET_EDITOR_ID_ROUTINE: u32 = slots::routine(Group::GL, 1);
pub const DESTROY_ROUTINE: u32 = slots::routine(Group::GL, 2);

/// A point light's size with JIP (`NiPointLight`, 0x110 in JIP's `netimmerse.h`), and a light
/// form's (`TESObjectLIGH` + its editor id word).
pub const POINT_LIGHT_SIZE: u32 = 0x110;
pub const LIGHT_FORM_BYTES: u32 = 0xcc;

/// `NiPointLight`'s vtable (`kVtbl_NiPointLight`), `NiStringExtraData`'s
/// (`kVtbl_NiStringExtraData`), and the shared `return this` an object's slot +0xC holds when it
/// is a node (`IS_NODE`, `ADDR_ReturnThis`).
pub const POINT_LIGHT_VTABLE: u32 = 0x0109_dd0c;
pub const STRING_EXTRA_VTABLE: u32 = 0x0109_d39c;
const RETURN_THIS: u32 = 0x0068_15c0;
/// `NiAVObject`: its parent (+0x18), flags (+0x30; JIP's bits in the byte +0x33: 0x20
/// `kNiFlag_IsPointLight`, 0x40 `kNiFlag_DoneInitLights`), local translation (+0x58, then the
/// scale +0x64). `NiObjectNET`: its extra data array (+0x10) and count (u16 +0x14); an extra's
/// name (+8) and a string extra's text (+0xC). `NiNode`: its children (+0xA0) and their count
/// (u16 +0xA6, JIP's read). `NiRefObject`'s count (+4).
const PARENT: u32 = 0x18;
const FLAGS: u32 = 0x30;
const FLAGS_HIGH: u32 = 0x33;
const IS_POINT_LIGHT: u8 = 0x20;
const DONE_INIT_LIGHTS: u8 = 0x40;
const LOCAL_TRANSLATE: u32 = 0x58;
const EXTRA_DATA: u32 = 0x10;
const EXTRA_COUNT: u32 = 0x14;
const EXTRA_NAME: u32 = 8;
const STRING_EXTRA_TEXT: u32 = 0xc;
const CHILDREN: u32 = 0xa0;
const CHILD_COUNT: u32 = 0xa6;
const REF_COUNT: u32 = 4;
/// A point light's JIP fields (`netimmerse.h`): `resetTraits` (+0x9E), `extraFlags` (+0x9F, bit 1:
/// the translation saved at +0x100), the dimmer `fadeValue` (+0xC4; JIP's animation keeps its
/// progress at +0xC8 and the dimmer's step at +0xCC, the pulse's direction in the sign of +0xD0),
/// the diffuse colour (+0xD4) and the radius (+0xE0), `baseLight` (+0xE8), the animation's offset
/// (+0xF0, 16 bytes) and the saved translation or colour (+0x100, 16 bytes).
const RESET_TRAITS: u32 = 0x9e;
const EXTRA_FLAGS: u32 = 0x9f;
const DIMMER: u32 = 0xc4;
const DIFFUSE: u32 = 0xd4;
const BASE_LIGHT: u32 = 0xe8;
const OFFSET: u32 = 0xf0;
const SAVED: u32 = 0x100;
/// A light form's radius (int, +0xA0), colour (bytes r, g, b, +0xA4), flags (+0xA8; 0x20 "off by
/// default") and fade (+0xB4); the editor id word JIP adds (+0xC8).
const FORM_RADIUS: u32 = 0xa0;
const FORM_COLOUR: u32 = 0xa4;
const FORM_FLAGS: u32 = 0xa8;
const FORM_FADE: u32 = 0xb4;
const FORM_EDITOR_ID: u32 = 0xc8;
/// The flags of an animated light (`test dword ptr [ecx+0xA8], 0x19C8`).
const ANIMATED: u32 = 0x19c8;
/// The game's editor id map (`*011C54C8`), its node allocator (`0043a010`, `thiscall`, the
/// receiver unused) and the empty text the stub answers (`01011584`).
const EDITOR_IDS_MAP: u32 = 0x011c_54c8;
const MAP_NEW_NODE: u32 = 0x0043_a010;
const EMPTY_TEXT: u32 = 0x0101_1584;
/// `NiLight`'s destructor body (`00a72010`), the sized free (`00aa1460`, `cdecl` (block, size))
/// and allocation (`00aa13e0`, `cdecl` (size)).
const LIGHT_DESTRUCTOR: u32 = 0x00a7_2010;
const SIZED_FREE: u32 = 0x00aa_1460;
const ALLOCATE: u32 = 0x00aa_13e0;
/// `NiObjectNET::RemoveExtraDataAt` (`00a5b990`, `thiscall`, a `u16` index).
const REMOVE_EXTRA_AT: u32 = 0x00a5_b990;
/// The exe's animated lights step JIP's hook repeats first (`00455640`, `thiscall` on `TES`: the
/// word `011C56E8` cleared, then the interior cell's (`TES +0x34`) or the grid's (`TES +8`) lights).
const GAME_ANIMATED_LIGHTS: u32 = 0x0045_5640;
/// `g_shadowSceneNode` (`*011F91C8`), the scene lights' lock (`SCENE_LIGHTS_CS`, taken with the
/// exe's recursive lock `0040fbf0` (lock, label) and `0040fba0`), a `ShadowSceneLight`'s
/// constructor (`00b9fda0`, 0x250 bytes) and the scene's light add (`00b5eca0`, `thiscall`).
const SHADOW_SCENE_NODE: u32 = 0x011f_91c8;
const SCENE_LIGHTS_LOCK: u32 = 0x011f_9ea0;
const LOCK_ENTER: u32 = 0x0040_fbf0;
const LOCK_LEAVE: u32 = 0x0040_fba0;
const SCENE_LIGHT_SIZE: u32 = 0x250;
const SCENE_LIGHT_CONSTRUCT: u32 = 0x00b9_fda0;
const SCENE_ADD_LIGHT: u32 = 0x00b5_eca0;
/// A `ShadowSceneLight`'s point light word (+0xF4, a 16-bit 1), light (+0xF8), the byte +0xFC
/// (`bDynamicLight`) JIP sets to 1, and +0x240 (the scene's +0x1E0).
const SCENE_LIGHT_POINT: u32 = 0xf4;
const SCENE_LIGHT_LIGHT: u32 = 0xf8;
const SCENE_LIGHT_FC: u32 = 0xfc;
const SCENE_LIGHT_240: u32 = 0x240;
const SCENE_1E0: u32 = 0x1e0;
/// The frame's seconds (`float` at `011F63A0`), the game's generator (`GAME_RNG` `011C4180`) and
/// `GetRandomInt` (`00AA5230`, `thiscall` with the bound).
const FRAME_SECONDS: u32 = 0x011f_63a0;
const GAME_RNG: u32 = 0x011c_4180;
const GET_RANDOM_INT: u32 = 0x00aa_5230;

/// `kAnimatedLightMods` (JIP's `DoUpdateAnimatedLight`): the flicker's speed (normal, slow), the
/// pulse's (normal, slow), the colour shift's (normal, slow), the dimmer target's scale and base.
const SPEED_FLICKER: [u32; 2] = [0x40f0_0000, 0x4089_2492];
const SPEED_PULSE: [u32; 2] = [0x3e00_0000, 0x3d80_0000];
const SPEED_SHIFT: [u32; 2] = [0x4080_0000, 0x4000_0000];
const DIMMER_SCALE: u32 = 0x3b9c_09c1;
const DIMMER_BASE: u32 = 0x3f2c_cccd;
/// The rows after them: the flicker's offset step (0.1, three lanes), the colour shift's
/// (`0x3C010204`), the mask of a random (0x7F) and the flicker's centre (0x3F); lane 3 is 0.
const FLICKER_STEP: u32 = 0x3dcc_cccd;
const SHIFT_STEP: u32 = 0x3c01_0204;
/// `kColourMults` (JIP's `SetLightProperties`): 1/255 for the colour, 1.0 for the radius.
const COLOUR_MULT: u32 = 0x3b80_8081;

/// JIP's `s_activePtLights`: the point lights of its own the step animates, in JIP's order (it
/// appends, and frees by moving the last into the freed place).
#[derive(Default)]
pub struct ActivePointLights(pub Mutex<Vec<u32>>);

fn lights_list(e: &mut Engine) -> std::sync::MutexGuard<'_, Vec<u32>> {
    e.ext_free::<ActivePointLights>()
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The lights JIP animates (a copy, for the tests and the cosave readers).
pub fn active_point_lights(e: &mut Engine) -> Vec<u32> {
    lights_list(e).clone()
}

pub fn install(e: &mut Engine) {
    // Made before the threads start, so every thread's engine shares it.
    e.ext_free::<ActivePointLights>();
    for slot in POINT_LIGHT_SIZES {
        e.on_site(slot, point_light_size);
    }
    e.on_site(CLONE_WORD, clone_word);
    e.on_site(LIGHT_FORM_SIZE, light_form_size);
    e.on_site(LOAD_NIF, load_nif);
    e.on_site(CLONE_3D, clone_3d);
    for slot in ANIMATED_LIGHTS {
        e.on_site(slot, animated_lights);
    }
    e.wrap(UPDATE_ANIMATED_LIGHT, update_animated_light);
    slots::write_slot(e, GET_EDITOR_ID_SLOT, GET_EDITOR_ID_ROUTINE, editor_id);
    slots::write_slot(e, SET_EDITOR_ID_SLOT, SET_EDITOR_ID_ROUTINE, set_editor_id);
    slots::write_slot(e, DESTROY_SLOT, DESTROY_ROUTINE, destroy_point_light);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: InitJIPHooks,
// `SafeWrite32(addr, 0x110)`.
/// H03: 0x110.
fn point_light_size(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(POINT_LIGHT_SIZE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: InitJIPHooks,
// `SAFE_WRITE_BUF(0xA68D6B, ..)`.
/// H05: the word.
fn clone_word(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: InitJIPHooks,
// `SafeWrite8(0x465488, 0xCC)`.
/// H04: 0xCC.
fn light_form_size(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(LIGHT_FORM_BYTES)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: TESObjectLIGHGetEDIDHook.
/// H04, `GetEditorID` (`thiscall`, the form): the word +0xC8, or `""` (`01011584`) when 0.
fn editor_id(e: &mut Engine, w: &[u32]) -> Ret {
    let text = e.mem.u32(w[0].wrapping_add(FORM_EDITOR_ID));
    let text = if text == 0 { EMPTY_TEXT } else { text };
    text.into_ret()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: TESObjectLIGHSetEDIDHook.
/// H04, `SetEditorID` (`thiscall`, the form and the text; `RET 4`): a node of the editor id
/// map's pool (`0043a010`, JIP's `ECX` the form) is put at the head of the text's bucket (the
/// map's slot +4), the map's count goes up, the map's slot +0xC sets the node (node, text,
/// form), and the node's key (the map's copy of the text) goes to the form's +0xC8. Answers
/// true. As JIP's: no test for a text already there (the newest is found first). With no map
/// (never once the game has started) the port answers true and does nothing; JIP would read
/// through null.
fn set_editor_id(e: &mut Engine, w: &[u32]) -> Ret {
    let (form, text) = (w[0], w[1]);
    let map = e.mem.u32(EDITOR_IDS_MAP);
    if map == 0 {
        return 1u32.into_ret();
    }
    let node = e.call(MAP_NEW_NODE, &args![form]).u32();
    let bucket = e.vcall(map, 4, &args![text]).u32();
    let at = e
        .mem
        .u32(map.wrapping_add(8))
        .wrapping_add(bucket.wrapping_mul(4));
    let head = e.mem.u32(at);
    e.mem.set_u32(at, node);
    e.mem.set_u32(node, head);
    let count = e.mem.u32(map.wrapping_add(0xc)).wrapping_add(1);
    e.mem.set_u32(map.wrapping_add(0xc), count);
    e.vcall(map, 0xc, &args![node, text, form]);
    let key = e.mem.u32(node.wrapping_add(4));
    e.mem.set_u32(form.wrapping_add(FORM_EDITOR_ID), key);
    1u32.into_ret()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DestroyNiPointLightHook.
/// H07, `NiPointLight`'s deleting destructor (`thiscall`, the light and the flags; `RET 4`):
/// `NiLight`'s destructor body (`00a72010`), then, with flag bit 0 and the light not one of JIP's
/// (`0x20000000` clear), the light's 0x110 bytes freed (`00aa1460`). Answers the light.
fn destroy_point_light(e: &mut Engine, w: &[u32]) -> Ret {
    let (light, flags) = (w[0], w[1]);
    e.call(LIGHT_DESTRUCTOR, &args![light]);
    if e.mem.u8(light.wrapping_add(FLAGS_HIGH)) & IS_POINT_LIGHT == 0 && flags & 1 != 0 {
        e.call(SIZED_FREE, &args![light, POINT_LIGHT_SIZE]);
    }
    light.into_ret()
}

/// Whether the guest text at `text` is exactly `LIGH_EDID` (JIP compares the extra's name with
/// its `NiFixedString` "LIGH_EDID": the game's string table keeps one copy of a text, so the
/// same pointer is the same text).
fn is_ligh_edid(e: &Engine, text: u32) -> bool {
    const NAME: &[u8] = b"LIGH_EDID\0";
    text != 0
        && NAME
            .iter()
            .enumerate()
            .all(|(i, &b)| e.mem.u8(text.wrapping_add(i as u32)) == b)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: LoadNifRetnNodeHook.
/// H06 (words: the 3D object `LoadFile` answers): a root not set up yet (`0x40000000` clear) is
/// marked and its point lights set up ([`init_point_lights`]). JIP's hook also counts the model
/// up and reads the 3D object, as the exe's code before the seam does. Answers 0 (not read).
fn load_nif(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let root = w[0];
    if root == 0 {
        return Some(0);
    }
    let flags = e.mem.u8(root.wrapping_add(FLAGS_HIGH));
    if flags & DONE_INIT_LIGHTS != 0 {
        return Some(0);
    }
    e.mem
        .set_u8(root.wrapping_add(FLAGS_HIGH), flags | DONE_INIT_LIGHTS);
    init_point_lights(e, root, root);
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: CreateObjectNodeHook.
/// H06's companion at `0x50EF46` (words: the model `Clone3D` would share, the form, the
/// reference): a model whose byte +0x33 has 0x20 (H06 found lights of JIP's in it) is cloned
/// (answers 1, JIP's jump to `0x50F0FE`); else the exe's checks go on (answers 0). Without it the
/// first reference would hold the cached model itself (its lights then listed as that
/// reference's), and every later copy would carry lights already marked listed, never lit.
/// (`Clone3D` never gets here with no model; 0 answers 0.)
/// H08: a form or a reference that JIP's `InsertNode`/`AttachModel` marked (`jipFormFlags6` `+6`,
/// 0xC0) is cloned too, so its own copy gets the entries.
fn clone_3d(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let (model, form, reference) = (w[0], w[1], w[2]);
    let marked = |e: &Engine, f: u32| {
        f != 0
            && e.mem.u8(f + 6)
                & (crate::commands::jip_fn_misc_ref_insert::INSERT_NODE
                    | crate::commands::jip_fn_misc_ref_insert::ATTACH_MODEL)
                != 0
    };
    Some(u32::from(
        (model != 0 && e.mem.u8(model.wrapping_add(FLAGS_HIGH)) & IS_POINT_LIGHT != 0)
            || marked(e, form)
            || marked(e, reference),
    ))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: InitPointLights.
/// Each child of `node` in order: a point light (its vtable) has its extra data searched from
/// the last for one named `LIGH_EDID`; the first found, when it is a string extra whose text is
/// a form's editor id, ties the light to the form (+0xE8) and marks it `0x20000000`, and each
/// parent up to `root` (stopping at one already marked); the extra is removed either way. A
/// node (slot +0xC is `return this`) is searched the same way. A parent missing before `root`
/// stops the marking (JIP would read through null).
pub(crate) fn init_point_lights(e: &mut Engine, node: u32, root: u32) {
    let count = e.mem.u16(node.wrapping_add(CHILD_COUNT));
    let children = e.mem.u32(node.wrapping_add(CHILDREN));
    for i in 0..u32::from(count) {
        let child = e.mem.u32(children.wrapping_add(4 * i));
        if child == 0 {
            continue;
        }
        let vtable = e.mem.u32(child);
        if vtable == POINT_LIGHT_VTABLE {
            tag_light(e, child, root);
        } else if e.mem.u32(vtable.wrapping_add(0xc)) == RETURN_THIS {
            init_point_lights(e, child, root);
        }
    }
}

/// [`init_point_lights`]' light.
fn tag_light(e: &mut Engine, light: u32, root: u32) {
    let extras = e.mem.u32(light.wrapping_add(EXTRA_DATA));
    let count = e.mem.u16(light.wrapping_add(EXTRA_COUNT));
    for index in (0..count).rev() {
        let extra = e.mem.u32(extras.wrapping_add(4 * u32::from(index)));
        if extra == 0 || !is_ligh_edid(e, e.mem.u32(extra.wrapping_add(EXTRA_NAME))) {
            continue;
        }
        let text = e.mem.u32(extra.wrapping_add(STRING_EXTRA_TEXT));
        if e.mem.u32(extra) == STRING_EXTRA_VTABLE && text != 0 {
            let form = editor_id_lookup_at(e, text);
            if form != 0 {
                let flags = e.mem.u8(light.wrapping_add(FLAGS_HIGH));
                e.mem
                    .set_u8(light.wrapping_add(FLAGS_HIGH), flags | IS_POINT_LIGHT);
                e.mem.set_u32(light.wrapping_add(BASE_LIGHT), form);
                let mut at = light;
                loop {
                    at = e.mem.u32(at.wrapping_add(PARENT));
                    if at == 0 {
                        break;
                    }
                    let flags = e.mem.u8(at.wrapping_add(FLAGS_HIGH));
                    if flags & IS_POINT_LIGHT != 0 {
                        break;
                    }
                    e.mem
                        .set_u8(at.wrapping_add(FLAGS_HIGH), flags | IS_POINT_LIGHT);
                    if at == root {
                        break;
                    }
                }
            }
        }
        e.call(REMOVE_EXTRA_AT, &args![light, u32::from(index)]);
        return;
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: AddPointLights.
/// The point lights of JIP's under `node` (H09's attach calls it for a 3D marked `0x20000000`):
/// each child in order marked `0x20000000`; a point light not listed yet (`0x40000000` clear) is
/// marked, hidden when its form is "off by default" (flag 0x20), set from its form
/// ([`set_light_properties`]) and appended to the list; a node is searched the same way. A light
/// with no form (JIP would read through null) is left alone.
pub fn add_point_lights(e: &mut Engine, node: u32) {
    let count = e.mem.u16(node.wrapping_add(CHILD_COUNT));
    let children = e.mem.u32(node.wrapping_add(CHILDREN));
    for i in 0..u32::from(count) {
        let child = e.mem.u32(children.wrapping_add(4 * i));
        if child == 0 || e.mem.u8(child.wrapping_add(FLAGS_HIGH)) & IS_POINT_LIGHT == 0 {
            continue;
        }
        let vtable = e.mem.u32(child);
        if vtable == POINT_LIGHT_VTABLE {
            let flags = e.mem.u8(child.wrapping_add(FLAGS_HIGH));
            let form = e.mem.u32(child.wrapping_add(BASE_LIGHT));
            if flags & DONE_INIT_LIGHTS != 0 || form == 0 {
                continue;
            }
            e.mem
                .set_u8(child.wrapping_add(FLAGS_HIGH), flags | DONE_INIT_LIGHTS);
            if e.mem.u32(form.wrapping_add(FORM_FLAGS)) & 0x20 != 0 {
                let hidden = e.mem.u8(child.wrapping_add(FLAGS));
                e.mem.set_u8(child.wrapping_add(FLAGS), hidden | 1);
            }
            set_light_properties(e, child, form);
            lights_list(e).push(child);
        } else if e.mem.u32(vtable.wrapping_add(0xc)) == RETURN_THIS {
            add_point_lights(e, child);
        }
    }
}

fn read4(e: &Engine, at: u32) -> [f32; 4] {
    std::array::from_fn(|i| e.mem.f32(at.wrapping_add(4 * i as u32)))
}

fn write4(e: &mut Engine, at: u32, v: [f32; 4]) {
    for (i, lane) in v.iter().enumerate() {
        e.mem.set_u32(at.wrapping_add(4 * i as u32), lane.to_bits());
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: SetLightProperties.
/// A light set from its form: the dimmer (+0xC4) the form's fade (its bits), the ambient colour
/// (+0xC8..+0xD0) and the four words at +0xF0 0; the diffuse colour (+0xD4) the form's colour
/// bytes over 255 (single floats, `cvtdq2ps` then `mulps`) and the radius (+0xE0) the form's
/// radius as a float (its int ORed with the colour's fourth byte, as JIP's lanes do).
fn set_light_properties(e: &mut Engine, light: u32, form: u32) {
    let fade = e.mem.u32(form.wrapping_add(FORM_FADE));
    e.mem.set_u32(light.wrapping_add(DIMMER), fade);
    for at in [0xc8u32, 0xcc, 0xd0, 0xf0, 0xf4, 0xf8, 0xfc] {
        e.mem.set_u32(light.wrapping_add(at), 0);
    }
    let radius = e.mem.u32(form.wrapping_add(FORM_RADIUS));
    let colour = e.mem.u32(form.wrapping_add(FORM_COLOUR));
    let mult = f32::from_bits(COLOUR_MULT);
    // Lane 3 is multiplied by 1.0 (`kColourMults`), which leaves it as it is.
    let lanes = [
        (colour & 0xff) as i32 as f32 * mult,
        ((colour >> 8) & 0xff) as i32 as f32 * mult,
        ((colour >> 16) & 0xff) as i32 as f32 * mult,
        ((colour >> 24) | radius) as i32 as f32,
    ];
    write4(e, light.wrapping_add(DIFFUSE), lanes);
}

/// `GetRandomInt(GAME_RNG, 0xFFFFFFFF)`.
fn random(e: &mut Engine) -> u32 {
    e.call(GET_RANDOM_INT, &args![GAME_RNG, 0xffff_ffffu32])
        .u32()
}

/// The animation's progress (+0xC8) for this frame: from 0 (a light not started: `start` runs)
/// or past 1.0 a new leg (`None`); else the progress plus `step`.
fn progress(e: &mut Engine, light: u32, step: f32) -> Option<f32> {
    let bits = e.mem.u32(light.wrapping_add(DIMMER + 4));
    if bits == 0 {
        return None;
    }
    let next = f32::from_bits(bits) + step;
    // `comiss`/`jbe`: a new leg only when ordered and above 1.0.
    if next > 1.0 {
        None
    } else {
        Some(next)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DoUpdateAnimatedLight.
/// One frame of JIP's animation of `light` by `form`'s flags, in single floats as JIP's SSE code
/// (lane 3 of its vectors included: the flicker writes it into the local scale +0x64):
/// - flicker (0x8, 0x40 slow): the step is the frame's seconds times 7.5 (slow 4.2857). The
///   dimmer moves toward a target (the step's share of +0xCC each frame) and the local translation
///   toward a point up to 6.4 units from the saved one (+0x100, saved and `extraFlags` bit 1 set
///   when the light starts); a new leg (progress +0xC8 from 0, or past 1.0) picks the dimmer's
///   target `(random & 0x3F) / 210 + 0.675` times the form's fade and the point
///   `((random & 0x7F) - 63) * 0.1` per axis (randoms z, y, x in that order of calls).
/// - pulse (0x80, 0x100 slow): the step is the seconds times 0.125 (slow 0.0625); at or above the
///   form's fade, or at or below a quarter of it, the direction (the sign of +0xD0) turns; the
///   dimmer moves by the step in the direction (`xorps` with the word +0xD0).
/// - JIP's colour shift (else; 0x800, 0x1000 slow): the step is the seconds times 4.0 (slow 2.0);
///   the diffuse colour moves toward the saved colour (+0x100) times `(random & 0x7F) / 127` per
///   channel.
pub fn animate(e: &mut Engine, form: u32, light: u32) {
    let flags = e.mem.u32(form.wrapping_add(FORM_FLAGS));
    let seconds = e.mem.f32(FRAME_SECONDS);
    if flags & 0x48 != 0 {
        let step = seconds * f32::from_bits(SPEED_FLICKER[((flags >> 6) & 1) as usize]);
        let fade = e.mem.f32(form.wrapping_add(FORM_FADE));
        let dimmer = e.mem.f32(light.wrapping_add(DIMMER));
        let mut change = e.mem.f32(light.wrapping_add(DIMMER + 8));
        let mut position = read4(e, light.wrapping_add(LOCAL_TRANSLATE));
        position[3] = f32::from_bits(0);
        let mut saved = read4(e, light.wrapping_add(SAVED));
        let mut offset = read4(e, light.wrapping_add(OFFSET));
        let started = e.mem.u32(light.wrapping_add(DIMMER + 4)) != 0;
        let progress = if !started {
            let extra = e.mem.u8(light.wrapping_add(EXTRA_FLAGS));
            e.mem.set_u8(light.wrapping_add(EXTRA_FLAGS), extra | 1);
            write4(e, light.wrapping_add(SAVED), position);
            saved = position;
            None
        } else {
            progress(e, light, step)
        };
        let progress = match progress {
            Some(next) => next,
            None => {
                let pick = (random(e) & 0x3f) as i32 as f32;
                change = (pick * f32::from_bits(DIMMER_SCALE) + f32::from_bits(DIMMER_BASE)) * fade
                    - dimmer;
                e.mem
                    .set_u32(light.wrapping_add(DIMMER + 8), change.to_bits());
                let first = random(e);
                let second = random(e);
                let third = random(e);
                // Lanes x, y, z: `(random & 0x7F) - 0x3F` times 0.1; lane 3: 0 times 0.0.
                let picks = [third, second, first];
                offset = std::array::from_fn(|lane| {
                    let (centred, scale) = match picks.get(lane) {
                        Some(r) => ((r & 0x7f) as i32 - 0x3f, f32::from_bits(FLICKER_STEP)),
                        None => (0, 0.0),
                    };
                    centred as f32 * scale - position[lane] + saved[lane]
                });
                write4(e, light.wrapping_add(OFFSET), offset);
                step
            }
        };
        e.mem
            .set_u32(light.wrapping_add(DIMMER + 4), progress.to_bits());
        let dimmer = dimmer + change * step;
        e.mem.set_u32(light.wrapping_add(DIMMER), dimmer.to_bits());
        let by = [step, step, step, 0.0];
        let position: [f32; 4] =
            std::array::from_fn(|lane| position[lane] + offset[lane] * by[lane]);
        write4(e, light.wrapping_add(LOCAL_TRANSLATE), position);
    } else if flags & 0x180 != 0 {
        let step = seconds * f32::from_bits(SPEED_PULSE[((flags >> 8) & 1) as usize]);
        let fade = e.mem.f32(form.wrapping_add(FORM_FADE));
        let dimmer = e.mem.f32(light.wrapping_add(DIMMER));
        // `comiss fade, dimmer; setbe`: not above (a NaN counts); `comiss fade / 4, dimmer;
        // setnb`: ordered and not below.
        let at_top = fade.partial_cmp(&dimmer) != Some(std::cmp::Ordering::Greater);
        let at_bottom = fade * 0.25 >= dimmer;
        let sign = if at_top || at_bottom { 0x8000_0000 } else { 0 };
        let direction = e.mem.u32(light.wrapping_add(DIMMER + 0xc)) ^ sign;
        e.mem.set_u32(light.wrapping_add(DIMMER + 0xc), direction);
        let moved = f32::from_bits(step.to_bits() ^ direction);
        e.mem
            .set_u32(light.wrapping_add(DIMMER), (dimmer + moved).to_bits());
    } else {
        let step = seconds * f32::from_bits(SPEED_SHIFT[((flags >> 12) & 1) as usize]);
        let mut colour = read4(e, light.wrapping_add(DIFFUSE));
        colour[3] = f32::from_bits(0);
        let mut offset = read4(e, light.wrapping_add(OFFSET));
        let mut saved = read4(e, light.wrapping_add(SAVED));
        let started = e.mem.u32(light.wrapping_add(DIMMER + 4)) != 0;
        let progress = if !started {
            write4(e, light.wrapping_add(SAVED), colour);
            saved = colour;
            None
        } else {
            progress(e, light, step)
        };
        let progress = match progress {
            Some(next) => next,
            None => {
                let first = random(e);
                let second = random(e);
                let third = random(e);
                // Lanes r, g, b: `random & 0x7F` times `0x3C010204`; lane 3: 0 times 0.0.
                let picks = [third, second, first];
                offset = std::array::from_fn(|lane| {
                    let (pick, scale) = match picks.get(lane) {
                        Some(r) => ((r & 0x7f) as i32, f32::from_bits(SHIFT_STEP)),
                        None => (0, 0.0),
                    };
                    pick as f32 * scale * saved[lane] - colour[lane]
                });
                write4(e, light.wrapping_add(OFFSET), offset);
                step
            }
        };
        e.mem
            .set_u32(light.wrapping_add(DIMMER + 4), progress.to_bits());
        // `movlps` and `movss`: lanes r, g, b only.
        for (lane, (value, moved)) in colour.iter_mut().zip(offset).take(3).enumerate() {
            *value += moved * step;
            e.mem.set_u32(
                light.wrapping_add(DIFFUSE + 4 * lane as u32),
                value.to_bits(),
            );
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: UpdateAnimatedLightHook.
/// H12, `0050de20` (`thiscall` on the form: the light data, a word): a form with an animated
/// flag (0x19C8) animates the data's point light ([`animate`]); never the exe's code. Answers 0
/// (the callers do not read it; JIP leaves `EAX` as its code left it). No light (JIP would read
/// through null): nothing.
fn update_animated_light(e: &mut Engine, w: &[u32]) -> Ret {
    let (form, data) = (w[0], w[1]);
    if e.mem.u32(form.wrapping_add(FORM_FLAGS)) & ANIMATED == 0 {
        return Ret::default();
    }
    let light = e.mem.u32(data);
    if light != 0 {
        animate(e, form, light);
    }
    Ret::default()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: UpdateAnimatedLightsHook.
/// H12 at `0x8C7C4E` / `0x8C7E18` (words: the `TES` word): the exe's animated lights
/// (`00455640`, which JIP's hook repeats inline), then, with the list not empty and the scene
/// lights' lock held, each listed light from the last:
/// - with no parent: freed (0x110 bytes, its destructor ran already, H07) and taken out (the last
///   moved into its place) once its count is 0, else kept;
/// - hidden (flag bit 0): nothing;
/// - marked `resetTraits`: unmarked and set from its form again, and with `extraFlags` bit 1 its
///   local translation and scale (16 bytes) back from +0x100 (the bit cleared);
/// - a count of 1 (only its parent holds it): put into the scene's lights, a new
///   `ShadowSceneLight` (0x250 bytes) with the point light word 1, the light (counted up), the
///   byte +0xFC 1 and +0x240 the scene's +0x1E0 (`00b5eca0`); no animation this frame;
/// - else animated when its form has an animated flag ([`animate`]).
///
/// Answers 0 (the exe's call skipped). With no scene node or no `ShadowSceneLight` (JIP would
/// read through null) the light is not added.
fn animated_lights(e: &mut Engine, w: &[u32]) -> Option<u32> {
    e.call(GAME_ANIMATED_LIGHTS, &args![w[0]]);
    let mut lights = std::mem::take(&mut *lights_list(e));
    if lights.is_empty() {
        return Some(0);
    }
    let scene = e.mem.u32(SHADOW_SCENE_NODE);
    e.call(LOCK_ENTER, &args![SCENE_LIGHTS_LOCK, 0u32]);
    let mut i = lights.len();
    while i > 0 {
        i -= 1;
        let light = lights[i];
        if e.mem.u32(light.wrapping_add(PARENT)) == 0 {
            if e.mem.u32(light.wrapping_add(REF_COUNT)) == 0 {
                e.call(SIZED_FREE, &args![light, POINT_LIGHT_SIZE]);
                lights.swap_remove(i);
            }
            continue;
        }
        if e.mem.u8(light.wrapping_add(FLAGS)) & 1 != 0 {
            continue;
        }
        let form = e.mem.u32(light.wrapping_add(BASE_LIGHT));
        if e.mem.u8(light.wrapping_add(RESET_TRAITS)) != 0 {
            e.mem.set_u8(light.wrapping_add(RESET_TRAITS), 0);
            set_light_properties(e, light, form);
            let extra = e.mem.u8(light.wrapping_add(EXTRA_FLAGS));
            if extra & 1 != 0 {
                e.mem.set_u8(light.wrapping_add(EXTRA_FLAGS), extra & 0xfe);
                for word in 0..4 {
                    let value = e.mem.u32(light.wrapping_add(SAVED + 4 * word));
                    e.mem
                        .set_u32(light.wrapping_add(LOCAL_TRANSLATE + 4 * word), value);
                }
            }
        }
        if e.mem.u32(light.wrapping_add(REF_COUNT)) == 1 {
            add_scene_light(e, scene, light);
            continue;
        }
        if e.mem.u32(form.wrapping_add(FORM_FLAGS)) & ANIMATED != 0 {
            animate(e, form, light);
        }
    }
    e.call(LOCK_LEAVE, &args![SCENE_LIGHTS_LOCK]);
    let mut list = lights_list(e);
    let added = std::mem::take(&mut *list);
    *list = lights;
    list.extend(added);
    Some(0)
}

/// [`animated_lights`]' add of a light to the scene's lights.
fn add_scene_light(e: &mut Engine, scene: u32, light: u32) {
    if scene == 0 {
        return;
    }
    let block = e.call(ALLOCATE, &args![SCENE_LIGHT_SIZE]).u32();
    if block == 0 {
        return;
    }
    let scene_light = e.call(SCENE_LIGHT_CONSTRUCT, &args![block]).u32();
    e.mem
        .set_u16(scene_light.wrapping_add(SCENE_LIGHT_POINT), 1);
    let at = light.wrapping_add(REF_COUNT);
    match e.mem.atomic_u32(at) {
        Some(count) => {
            count.fetch_add(1, Ordering::SeqCst);
        }
        None => {
            let count = e.mem.u32(at).wrapping_add(1);
            e.mem.set_u32(at, count);
        }
    }
    e.mem
        .set_u32(scene_light.wrapping_add(SCENE_LIGHT_LIGHT), light);
    // One byte (`bDynamicLight`), as JIP and the exe write it; +0xFD..+0xFF are kept.
    e.mem.set_u8(scene_light.wrapping_add(SCENE_LIGHT_FC), 1);
    let word = e.mem.u32(scene.wrapping_add(SCENE_1E0));
    e.mem
        .set_u32(scene_light.wrapping_add(SCENE_LIGHT_240), word);
    e.call(SCENE_ADD_LIGHT, &args![scene, scene_light]);
}

#[cfg(test)]
#[path = "gl_lights_tests.rs"]
mod tests;
