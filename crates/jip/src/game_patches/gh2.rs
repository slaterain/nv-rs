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

use super::Area;
use crate::commands::jip_fn_aux_variable::{aux_maps, mark_changed, AuxMods};
use crate::cosave::{changed, has_change_data};
use crate::world_hooks::TextBlock;
use crate::Jip;
use engine_core::extend::{Hook, Owner, SiteSlot};
use engine_core::prelude::*;
use std::collections::BTreeMap;

/// H24's seam: the destructor's call of `IsFurniture` (the reference).
pub const DESTROY_REFR: SiteSlot = SiteSlot::new(Owner::Jip, 0x0055_a488);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GH2",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: super::nothing,
    jip_sites: &[DESTROY_REFR.addr()],
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
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/hooks.h: DestroyRefrHook.
/// The destructor's furniture test (`0x55A488`: the reference). A reference with no base, or a
/// projectile's or an explosion's, answers no at once. Any other: its own name (flag
/// `AltRefName`) and then its model path (flag `RefrModelPath`) leave JIP's maps, each that was
/// there with one user of its hook less (the flags are left: the reference goes); a temporary
/// reference (mod index 0xFF) the save holds no changes of loses its aux variables, saved
/// (marking them changed when one went) and temporary, when either map has any; then the base's
/// type answers whether it is furniture, as `IsFurniture` would.
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
    if e.mem.u8(reference + MOD_INDEX) == 0xff
        && !has_change_data(e, id)
        && !(aux_maps(e, false).is_empty() && aux_maps(e, true).is_empty())
    {
        if clear_ref_aux_vars(aux_maps(e, false), id) {
            mark_changed(e, changed::AUX_VARS);
        }
        clear_ref_aux_vars(aux_maps(e, true), id);
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
    let state = &mut e.ext::<Jip>().world_hooks;
    if let Some(block) = map(state).remove(&reference) {
        state.free_texts.push(block);
        e.add_hook_user(hook, false);
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

#[cfg(test)]
#[path = "gh2_tests.rs"]
mod tests;
