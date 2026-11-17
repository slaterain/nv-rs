//! Group GH2's game patches (step 3.9: references, models, cells, ash piles): its handlers, wraps and slot routines. Only group
//! GH2's PR edits this file (and its own tests file beside it).
//!
//! **H24, `DestroyRefrHook`** (`InitJIPHooks`, `internal/hooks.h` (JIP LN NVSE):
//! `WriteRelCall(0x55A488, DestroyRefrHook)`), JIP's own: the reference's destructor body
//! (`0055a430`) calls `TESObjectREFR::IsFurniture` (`0x568680`, original bytes `E8 F3 E1 00 00`)
//! there; JIP's routine answers the same test and first lets go of what JIP keeps for the
//! reference: its own name (`SetRefName`) and model path (`SetRefrModelPath`, both built in step
//! 3.5c, [`crate::world_hooks`]), each with a user of its hook, and a temporary reference's aux
//! variables (the aux variable commands, [`crate::commands::jip_fn_aux_variable`]). Without it a
//! named or re-modelled reference that is destroyed keeps its entry: the hook's user count never
//! falls back, and a later reference built at the same address would answer the old name or
//! model.
//!
//! **G06, G73, G103**: JIP's ash piles. G06 (`InitGamePatches`, "Recognize custom ashpile
//! activators", JIP's own): the eight places the exe compares a reference's base form with the
//! two ash pile forms test the base's ash pile bit instead (the two vanilla piles get it in
//! `DeferredInit`, `MarkActivatorAshPile` gives it to others). G73 (`AttachAshPileHook`, JIP's
//! own): the ash pile a death leaves can be `AttachAshPileEx`'s form. All eight G06 places are
//! asked (`0x77765A` in group GA's `SetInfoForRef` action text, `0x95F7FE` in group GP's
//! `fn_0095f6c0`). G73's seam at
//! `0x5DBCFD` (in group GS's `fn_005db870`) is asked; it changes nothing until the
//! `AttachAshPileEx` command exists. G103
//! (`ClearAshPilesHook`, a fix row): the cell's reset walk deletes ash piles itself and no longer
//! with their actor; JIP's jump there also passes over the exe's branches for form types 0x1D and
//! 0x26 (`FLOR` and placed `INGR` references, of which the game's own plugins have none).
//!
//! **G31, G78, G81, G90, G106** (`InitGamePatches`, fix rows): a door's own stealing rules
//! decide (G31); `SetScale` clamps to about 0.0001 .. 100 with no rounding (G78); removing a
//! weapon detaches the weapon node's children instead of releasing its effects (G81); leaving an
//! interior ends the sky's weather transition at the next main loop, JIP's own main-loop function
//! ([`crate::main_loop::Native`], G90); the water forms' update asks JIP's map lookup and the
//! renderer's byte `0x11F91C5` (G106). G90 runs on the main thread (the cell change) and takes
//! JIP's slot for the main-loop list, as the script callbacks do; the others read and write game
//! memory only.
//!
//! Threads: G06's test reads the base's flag byte only and G73's form is a lock-free atomic, so
//! neither takes JIP's slot (Activate, Enable/Disable and the physics island callback may run off
//! the main thread). G103's walk `0054df30` runs in the cell attach (`0054bcf0`, from the grid
//! loads on the main thread) and its handlers take no slot either; only `deferred_init` (the
//! start-up message, main thread) writes JIP's kept form flags under the slot.

use super::Area;
use crate::commands::jip_fn_aux_variable::{maps_may_hold, mark_changed, AuxMaps, AuxMods};
use crate::cosave::{changed, has_change_data};
use crate::world_hooks::TextBlock;
use engine_core::extend::{Hook, Owner, SiteSlot};
use engine_core::prelude::*;
use std::collections::BTreeMap;

/// H24's seam: the destructor's call of `IsFurniture` (the reference).
pub const DESTROY_REFR: SiteSlot = SiteSlot::new(Owner::Jip, 0x0055_a488);

/// G06's seams (`InitGamePatches`, "Recognize custom ashpile activators"): the exe's two compares
/// of a reference's base form with the ash pile forms, each starting with a `call 0x7AF430`
/// (`TESObjectREFR::GetBaseForm`). Words: the reference (JIP's `ecx`); the answer's low byte is 1
/// for an ash pile. Six are in this group's translations; `0x77765A` (group GA,
/// `set_info_inventory_action`, the action text part of `hud_main_menu_set_info_for_ref`) and
/// `0x95F7FE` (group GP, `fn_0095f6c0`) are those groups' seams.
pub const ASH_PILE_TESTS: [SiteSlot; 8] = [
    SiteSlot::new(Owner::Jip, 0x0056_a30a),
    SiteSlot::new(Owner::Jip, 0x0057_3194),
    SiteSlot::new(Owner::Jip, 0x0057_3fe8),
    SiteSlot::new(Owner::Jip, 0x0057_445b),
    SiteSlot::new(Owner::Jip, 0x0057_6af0),
    SiteSlot::new(Owner::Jip, 0x0062_4958),
    SiteSlot::new(Owner::Jip, 0x0077_765a),
    SiteSlot::new(Owner::Jip, 0x0095_f7fe),
];
/// G73's seam (group GS's, in `fn_005db870`): the ash pile form a death leaves. Words: the
/// command's mode (`[ebp-0x18]`); the answer is the form.
pub const ATTACH_ASH_PILE: SiteSlot = SiteSlot::new(Owner::Jip, 0x005d_bcfd);
/// G103's seams in the cell's reset walk `fn_0054df30`: the deleted actor's ash pile (words: the
/// actor; any answer skips it) and the other references' type branches (words: the reference,
/// its base form; any answer means JIP's routine ran and the walk goes on at `0x54E5E6`).
pub const DELETED_ACTOR_ASH_PILE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0054_e421);
pub const CLEAR_ASH_PILES: SiteSlot = SiteSlot::new(Owner::Jip, 0x0054_e4e1);
/// G31's seam in `fn_00579690` (the "is it stealing" test), at the end of its door branch:
/// words the reference and the door's answer; any answer returns the door's answer.
pub const DOOR_ANSWER_STANDS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0057_9942);
/// G78's seam in `fn_00567490` (`SetScale`): words the scale's bits; the answer is the scale.
pub const SET_SCALE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0056_74d5);
/// G81's seams in `RemoveWeapon` (`00571b50`): the held weapon node and the "Weapon" child, each
/// in place of its `call 0x572160`. Words: the node.
pub const CLEAR_WEAPON_NODE: [SiteSlot; 2] = [
    SiteSlot::new(Owner::Jip, 0x0057_1f65),
    SiteSlot::new(Owner::Jip, 0x0057_1fdf),
];
/// G90's seam in `fn_00454450` (leaving an interior), in place of `008d7dc0(this, 0)`: words the
/// `TES`.
pub const CLOUDS_FIX: SiteSlot = SiteSlot::new(Owner::Jip, 0x0045_4576);
/// G106's seam in `fn_004e3d60` (the water forms' update), in place of the update map's find:
/// words the map and the water form; the answer's low byte is "already updated".
pub const UPDATE_WATER_TEXTURES: SiteSlot = SiteSlot::new(Owner::Jip, 0x004e_400e);

/// `kHookFormFlag6_IsAshPile` (the base form's `jipFormFlags6`).
pub const IS_ASH_PILE: u8 = 1 << 5;
/// `g_ashPileACTI` and `g_gooPileACTI`: the exe's words at `0x11CA27C` and `0x11CA280`.
const ASH_PILE_FORM: u32 = 0x011c_a27c;
const GOO_PILE_FORM: u32 = 0x011c_a280;
/// `kXData_ExtraAshPileRef` (its source at `+0xC`); `TESObjectREFR::MarkAsDeleted` (`00572270`).
const EXTRA_ASH_PILE_REF: u32 = 0x89;
const MARK_AS_DELETED: u32 = 0x0057_2270;
/// A form's flags (`+8`): byte `+9` bit 2 is `0x400` (`kFormFlag_QuestItem`).
const FORM_FLAGS_BYTE_9: u32 = 9;

/// JIP's `s_altAshPile`: the form `AttachAshPileEx` sets for the next death's ash pile, taken
/// once. Lock-free ([`Engine::ext_free`]), made at install before the threads start.
#[derive(Default)]
pub struct AltAshPile(pub std::sync::atomic::AtomicU32);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GH2",
    fix_sites: &[
        DELETED_ACTOR_ASH_PILE.addr(),
        CLEAR_ASH_PILES.addr(),
        DOOR_ANSWER_STANDS.addr(),
        SET_SCALE.addr(),
        CLEAR_WEAPON_NODE[0].addr(),
        CLEAR_WEAPON_NODE[1].addr(),
        CLOUDS_FIX.addr(),
        UPDATE_WATER_TEXTURES.addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[
        DESTROY_REFR.addr(),
        ASH_PILE_TESTS[0].addr(),
        ASH_PILE_TESTS[1].addr(),
        ASH_PILE_TESTS[2].addr(),
        ASH_PILE_TESTS[3].addr(),
        ASH_PILE_TESTS[4].addr(),
        ASH_PILE_TESTS[5].addr(),
        ASH_PILE_TESTS[6].addr(),
        ASH_PILE_TESTS[7].addr(),
        ATTACH_ASH_PILE.addr(),
    ],
    jip_wraps: &[],
    install_jip,
};

/// `kFormType_TESFurniture`, `kFormType_BGSProjectile`, `kFormType_BGSExplosion`.
const FURNITURE: u8 = 0x27;
const PROJECTILE: u8 = 0x33;
const EXPLOSION: u8 = 0x51;
/// `kHookRefFlag5F_AltRefName`, `kHookRefFlag5F_RefrModelPath` (the reference's byte `+0x5F`).
const ALT_REF_NAME: u8 = 1 << 2;
const REFR_MODEL_PATH: u8 = 1 << 3;
/// A reference's base form (`+0x20`), its id (`+0xC`, the mod index its high byte `+0xF`), JIP's
/// flag byte (`+0x5F`); a form's type (`+4`).
const BASE: u32 = 0x20;
const REF_ID: u32 = 0xc;
const MOD_INDEX: u32 = 0xf;
const JIP_FLAGS: u32 = 0x5f;
const FORM_TYPE: u32 = 4;

fn install_jip(e: &mut Engine) {
    e.on_site(DESTROY_REFR, destroy_refr);
    e.ext_free::<AltAshPile>();
    for site in ASH_PILE_TESTS {
        e.on_site(site, is_ash_pile);
    }
    e.on_site(ATTACH_ASH_PILE, attach_ash_pile);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DestroyRefrHook.
/// The destructor's furniture test (`0x55A488`: the reference). A reference with no base, or a
/// projectile's or an explosion's, answers no at once. Any other: its own name (flag
/// `AltRefName`) and then its model path (flag `RefrModelPath`) leave JIP's maps, each that was
/// there with one user of its hook less (the flags are left: the reference goes); a temporary
/// reference (mod index 0xFF) the save holds no changes of loses its aux variables, saved
/// (marking them changed when one went) and temporary, when either map has any; then the base's
/// type answers whether it is furniture, as `IsFurniture` would.
///
/// Threads: the destructor body `0055a430` is reached from the references' deleting destructor
/// (`0055a3d0`, any thread that deletes a reference), `~MobileObject` (`0092ed43`),
/// `RemoveStolenItems` (`0072ec3f`) and `CheckConditionFilters` (`005ea1b2`, `005ea1fe`: a
/// temporary reference on the stack, on whichever thread evaluates the conditions: the AI
/// threads too, with real threads on). So it runs off the main thread. What the routine reads
/// without a lock: the reference's and base's bytes, the save's changes map (game memory) and
/// the aux maps' lock-free "may hold" flag ([`maps_may_hold`]): with nothing named and both
/// aux maps empty it takes no lock at all. Otherwise it takes the aux maps' own lock once
/// (`jip_fn_aux_variable::AuxMaps`, a leaf held for the test and both erasures, never across
/// a game call), never JIP's shared slot; the names' and model paths' maps are the world hooks'
/// lock-free slot.
fn destroy_refr(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let reference = a[0];
    let base = e.mem.u32(reference + BASE);
    if base == 0 {
        return Some(0);
    }
    let kind = e.mem.u8(base + FORM_TYPE);
    if kind == PROJECTILE || kind == EXPLOSION {
        return Some(0);
    }
    let flags = e.mem.u8(reference + JIP_FLAGS);
    if flags & (ALT_REF_NAME | REFR_MODEL_PATH) != 0 {
        if flags & ALT_REF_NAME != 0 {
            drop_text(e, reference, Hook::GetRefName, |s| &mut s.ref_names);
        }
        if e.mem.u8(reference + JIP_FLAGS) & REFR_MODEL_PATH != 0 {
            drop_text(e, reference, Hook::GetModelPath, |s| &mut s.model_paths);
        }
    }
    let id = e.mem.u32(reference + REF_ID);
    if e.mem.u8(reference + MOD_INDEX) == 0xff && !has_change_data(e, id) && maps_may_hold(e) {
        // One take of the maps' lock for the test, both erasures and the copy AuxVarGetFltCond
        // reads losing the owner (`AuxMaps`), not JIP's slot.
        let erased = erase_owner(e.ext_free::<AuxMaps>(), id);
        if erased == Some(true) {
            mark_changed(e, changed::AUX_VARS);
        }
    }
    let kind = e.mem.u8(e.mem.u32(reference + BASE) + FORM_TYPE);
    Some(u32::from(kind == FURNITURE))
}

/// `UnorderedMap<TESObjectREFR*, char*>::GetErase`, then the text freed and
/// `HookInfo::ModUsers(false)`: the reference's text leaves the map, its block waits for the
/// next text (as `SetRefName` with an empty text leaves it), and the hook loses a user; nothing
/// when the map has no text for it.
fn drop_text(
    e: &mut Engine,
    reference: u32,
    hook: Hook,
    map: fn(&mut crate::world_hooks::State) -> &mut BTreeMap<u32, TextBlock>,
) {
    let found = {
        let mut state = crate::world_hooks::state(e);
        let block = map(&mut state).remove(&reference);
        if let Some(block) = block {
            state.free_texts.push(block);
        }
        block.is_some()
    };
    if found {
        e.add_hook_user(hook, false);
    }
}

/// The destructor's aux part on the maps, under one take of their lock: with both empty the
/// "may hold" flag is cleared and `None` answered; else the id's variables leave the saved and the
/// temporary map and the copy `AuxVarGetFltCond` reads (in the same take, so a command's publish
/// on another thread cannot land between them), and whether a saved one went is answered.
fn erase_owner(maps: &AuxMaps, id: u32) -> Option<bool> {
    erase_owner_with(maps, id, || {})
}

/// [`erase_owner`], with `between` run after the maps' erasures and before the copy's (tests:
/// another thread's set and publish there must wait for this take).
fn erase_owner_with(maps: &AuxMaps, id: u32, between: impl FnOnce()) -> Option<bool> {
    let mut pair = maps.lock();
    if pair.saved.is_empty() && pair.temp.is_empty() {
        pair.maps_found_empty();
        None
    } else {
        let saved = clear_ref_aux_vars(&mut pair.saved, id);
        clear_ref_aux_vars(&mut pair.temp, id);
        between();
        maps.forget_owner_held(&pair, id);
        Some(saved)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: ClearRefAuxVars.
/// The reference id's variables out of every mod's owners; a mod whose owners became empty by it
/// goes too (a mod that had none of the id stays, empty or not). Whether any went.
fn clear_ref_aux_vars(mods: &mut AuxMods, id: u32) -> bool {
    if mods.is_empty() {
        return false;
    }
    let mut erased = false;
    mods.retain(|_, owners| {
        if owners.erase(&id) {
            erased = true;
            !owners.is_empty()
        } else {
            true
        }
    });
    erased
}

fn install_fixes(e: &mut Engine) {
    e.on_site(DELETED_ACTOR_ASH_PILE, keep_deleted_actors_ash_pile);
    e.on_site(CLEAR_ASH_PILES, clear_ash_piles);
    e.on_site(DOOR_ANSWER_STANDS, door_answer_stands);
    e.on_site(SET_SCALE, set_scale_clamp);
    for site in CLEAR_WEAPON_NODE {
        e.on_site(site, clear_weapon_node);
    }
    e.on_site(CLOUDS_FIX, clouds_fix);
    e.on_site(UPDATE_WATER_TEXTURES, update_water_textures);
}

/// G31 (`WriteRelJump(0x579942, 0x579AB7)`): using a door is stealing only when the door's own
/// rules say so (its lock and the destination cell's owner); the general "an owned object the
/// player does not own" test after them is passed over, so an unlocked door into a public cell
/// is no crime.
fn door_answer_stands(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

/// `kMinScale` (`0x38D1B717`, about 0.0001) and `SS_100`.
const MIN_SCALE_BITS: u32 = 0x38d1_b717;
const MAX_SCALE: f32 = 100.0;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetScaleHook.
/// The scale `SetScale` stores: `maxss` with about 0.0001 then `minss` with 100 (SSE: a NaN
/// gives the second operand, so the lower bound), in place of the exe's rounding to two decimals
/// and its 0.01 .. 10 clamps.
fn set_scale_clamp(_: &mut Engine, a: &[u32]) -> Option<u32> {
    let value = f32::from_bits(a[0]);
    let min = f32::from_bits(MIN_SCALE_BITS);
    let value = if value > min { value } else { min };
    let value = if value < MAX_SCALE { value } else { MAX_SCALE };
    Some(value.to_bits())
}

/// A node's children (`NiNode::m_children`): the array (`+0xA0`) and its used count (`+0xA6`).
const CHILDREN: u32 = 0xa0;
const CHILD_COUNT: u32 = 0xa6;
/// The child's flag byte JIP tests (`+0x33`, bit `0x80`: the top bit of the object's flags).
const KEEP_FLAG_BYTE: u32 = 0x33;
/// `NiNode`'s virtual that detaches a child.
const DETACH_CHILD: u32 = 0xe8;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ClearWeaponNodeHook.
/// In place of releasing the weapon node's attached effects: each child of the node, from the
/// last to the first, that is set and lacks the keep flag is detached (the node's virtual
/// `+0xE8`). The array and its count are read once, as JIP's registers hold them.
fn clear_weapon_node(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let node = a[0];
    let children = e.mem.u32(node + CHILDREN);
    let count = e.mem.u16(node + CHILD_COUNT);
    for i in (0..u32::from(count)).rev() {
        let child = e.mem.u32(children + i * 4);
        if child == 0 || e.mem.u8(child + KEEP_FLAG_BYTE) & 0x80 != 0 {
            continue;
        }
        e.vcall(node, DETACH_CHILD, &args![child]);
    }
    Some(0)
}

/// `TES::currentInterior` and `TES::sky`.
const TES_CURRENT_INTERIOR: u32 = 0x34;
const TES_SKY: u32 = 0x68;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: CloudsFixHook.
/// Leaving an interior: the interior cell cleared (as the exe's call does), and the sky's weather
/// transition ended at the next main loop (`EndWeatherTransition` on the sky, one call), so the
/// clouds do not show the old weather fading out.
fn clouds_fix(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let tes = a[0];
    e.mem.set_u32(tes + TES_CURRENT_INTERIOR, 0);
    let sky = e.mem.u32(tes + TES_SKY);
    crate::main_loop::add_native(e, crate::main_loop::Native::EndWeatherTransition, sky);
    Some(0)
}

/// The renderer's byte JIP tests first (`0x11F91C5`; the exe only clears it, `00b54280`).
const WATER_TEXTURES_HELD: u32 = 0x011f_91c5;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: UpdateWaterTexturesHook
// and internal/Ni_types.h: NiTPtrMap::Lookup.
/// In place of the update map's find: the renderer's byte when set; else the value the map keeps
/// for the water form (its buckets `+8`, their count `+4`; an entry's next `+0`, key `+4`, value
/// `+8`), 0 when it has none. A map with no buckets answers 0 (JIP would divide by zero).
fn update_water_textures(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (map, key) = (a[0], a[1]);
    let held = e.mem.u8(WATER_TEXTURES_HELD);
    if held != 0 {
        return Some(u32::from(held));
    }
    let buckets = e.mem.u32(map + 4);
    if buckets == 0 {
        return Some(0);
    }
    let mut entry = e.mem.u32(e.mem.u32(map + 8) + (key % buckets) * 4);
    while entry != 0 {
        if e.mem.u32(entry + 4) == key {
            return Some(e.mem.u32(entry + 8));
        }
        entry = e.mem.u32(entry);
    }
    Some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches,
// "Recognize custom ashpile activators" (the 14 bytes at each site).
/// The ash pile test (`mov eax, [ecx+0x20]; test byte [eax+6], 0x20`): 1 when the reference's base
/// form carries JIP's ash pile bit (the two vanilla piles get it at start-up, `MarkActivatorAshPile`
/// gives it to others), else 0. A reference with no base answers 0, as the exe's compares do (JIP
/// would read address 6).
fn is_ash_pile(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let base = e.mem.u32(a[0] + BASE);
    if base == 0 {
        return Some(0);
    }
    Some(u32::from(
        crate::world_hooks::flags6(e, base) & IS_ASH_PILE != 0,
    ))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit.
/// `g_ashPileACTI->SetJIPFlag(kHookFormFlag6_IsAshPile, true)` and the same for
/// `g_gooPileACTI`: the two vanilla piles are ash piles to G06's test. Only while JIP answers
/// "installed" (the word's bit), and not for a missing form.
pub fn deferred_init(e: &mut Engine) {
    if e.jip_patches() & engine_core::extend::jip_patches::INSTALLED == 0 {
        return;
    }
    for at in [ASH_PILE_FORM, GOO_PILE_FORM] {
        // Always mapped in the game; a test engine without the exe's data has neither.
        if !e.mem.is_mapped(at) {
            continue;
        }
        let form = e.mem.u32(at);
        if form != 0 {
            crate::world_hooks::set_jip_flag(e, form, IS_ASH_PILE, true);
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: AttachAshPileHook.
/// The form a death's ash pile is made of: `AttachAshPileEx`'s form once (then cleared), else
/// the goo pile for mode 2 and the ash pile for any other, as the exe chooses.
fn attach_ash_pile(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let alt = e
        .ext_free::<AltAshPile>()
        .0
        .swap(0, std::sync::atomic::Ordering::Relaxed);
    if alt != 0 {
        return Some(alt);
    }
    let at = if a[0] == 2 {
        GOO_PILE_FORM
    } else {
        ASH_PILE_FORM
    };
    Some(e.mem.u32(at))
}

/// `SafeWrite16(0x54E421, 0x1EEB)`: a deleted actor's ash pile is not deleted with it here
/// ([`clear_ash_piles`] deletes it when the cell's walk reaches the pile).
fn keep_deleted_actors_ash_pile(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ClearAshPilesHook.
/// Every reference that is neither an actor nor a container: an ash pile (JIP's bit on its base)
/// with no `ExtraAshPileRef` is deleted; with one whose source's base form carries flag `0x400`
/// it stays; else the source loses its `ExtraAshPileRef` (when there is a source), the pile
/// loses its own, and the pile is deleted. Anything else: nothing. Then the walk goes on at
/// `0x54E5E6`, past the exe's branches for form types `0x1D` and `0x26`.
fn clear_ash_piles(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (reference, base) = (a[0], a[1]);
    if crate::world_hooks::flags6(e, base) & IS_ASH_PILE == 0 {
        return Some(0);
    }
    let extra = crate::commands::extra::refr_extra(e, reference, EXTRA_ASH_PILE_REF);
    if extra != 0 {
        let source = e.mem.u32(extra + 0xc);
        if source != 0 {
            let source_base = e.mem.u32(source + BASE);
            // A source with no base: JIP would read address 9; taken as the flag clear.
            if source_base != 0 && e.mem.u8(source_base + FORM_FLAGS_BYTE_9) & 4 != 0 {
                return Some(0);
            }
            crate::commands::extra::remove_by_type(
                e,
                source + crate::commands::extra::REFR_EXTRA_LIST,
                EXTRA_ASH_PILE_REF,
            );
        }
        crate::commands::extra::remove_by_type(
            e,
            reference + crate::commands::extra::REFR_EXTRA_LIST,
            EXTRA_ASH_PILE_REF,
        );
    }
    e.call(MARK_AS_DELETED, &args![reference]);
    Some(0)
}
#[cfg(test)]
#[path = "gh2_tests.rs"]
mod tests;
