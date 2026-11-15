//! Group GH2's game patches. H24 (`DestroyRefrHook`): each branch of JIP's routine, over a
//! stand-in reference (base `+0x20`, id `+0xC`, JIP's byte `+0x5F`), its base form (type `+4`)
//! and the save's changes map (`g_BGSSaveLoadGame` `0x011DDF38` -> its map at `+0`: hash size
//! `+4`, table `+8`; an entry's next `+0`, key `+4`).

use super::*;
use crate::containers::{Name, UnorderedMap};
use crate::cosave::Value;
use crate::test_support::engine;
use engine_core::extend::jip_patches;

const AREA_AT: u32 = 0x0500_0000;
const REF: u32 = AREA_AT + 0x100;
const OTHER: u32 = AREA_AT + 0x200;
const BASE_FORM: u32 = AREA_AT + 0x300;
const SAVE_LOAD: u32 = AREA_AT + 0x400;
const CHANGES: u32 = AREA_AT + 0x500;
const BUCKETS: u32 = AREA_AT + 0x600;
const ENTRY: u32 = AREA_AT + 0x700;
const TEMP_ID: u32 = 0xff00_0801;
const BLOCK: TextBlock = TextBlock {
    at: AREA_AT + 0x800,
    capacity: 0x80,
};
const BLOCK_TWO: TextBlock = TextBlock {
    at: AREA_AT + 0x880,
    capacity: 0x80,
};

/// JIP installed, a reference of base type `kind` with id `id`, an empty changes map (one
/// bucket).
fn world(kind: u8, id: u32) -> Engine {
    let mut e = engine();
    e.map(AREA_AT, 0x1000);
    e.map(0x011d_d000, 0x1000);
    e.mem.set_u32(0x011d_df38, SAVE_LOAD);
    e.mem.set_u32(SAVE_LOAD, CHANGES);
    e.mem.set_u32(CHANGES + 4, 1);
    e.mem.set_u32(CHANGES + 8, BUCKETS);
    crate::install_as(&mut e, true);
    e.mem.set_u32(REF + 0x20, BASE_FORM);
    e.mem.set_u32(REF + 0xc, id);
    e.mem.set_u8(BASE_FORM + 4, kind);
    e
}

fn destroy(e: &mut Engine, reference: u32) -> Option<u32> {
    e.site(0x0055_a488, &[reference])
}

/// The reference named and re-modelled as `SetRefName` and `SetRefrModelPath` leave it: both
/// flags, both texts, one user of each hook; another reference named too.
fn named(e: &mut Engine) {
    e.mem.set_u8(REF + 0x5f, ALT_REF_NAME | REFR_MODEL_PATH | 1);
    let s = &mut e.ext::<Jip>().world_hooks;
    s.ref_names.insert(REF, BLOCK);
    s.model_paths.insert(REF, BLOCK_TWO);
    s.ref_names.insert(OTHER, BLOCK_TWO);
    e.add_hook_user(Hook::GetRefName, true);
    e.add_hook_user(Hook::GetRefName, true);
    e.add_hook_user(Hook::GetModelPath, true);
}

fn names(e: &mut Engine) -> Vec<u32> {
    e.ext::<Jip>()
        .world_hooks
        .ref_names
        .keys()
        .copied()
        .collect()
}

fn paths(e: &mut Engine) -> Vec<u32> {
    e.ext::<Jip>()
        .world_hooks
        .model_paths
        .keys()
        .copied()
        .collect()
}

/// A variable `v` = 1 for `owner`.
fn vars() -> UnorderedMap<Name, Vec<Value>> {
    let mut v: UnorderedMap<Name, Vec<Value>> = UnorderedMap::new(4);
    v.emplace(Name(b"v".to_vec())).push(Value::Number(1.0));
    v
}

/// Mod 1: `TEMP_ID` alone; mod 2: `TEMP_ID` and another; mod 3: another.
fn three_mods() -> AuxMods {
    let mut mods = AuxMods::new(8);
    mods.emplace(1).emplace_with(TEMP_ID, vars);
    let two = mods.emplace(2);
    two.emplace_with(TEMP_ID, vars);
    two.emplace_with(0x0100_0001, vars);
    mods.emplace(3).emplace_with(0x0100_0001, vars);
    mods
}

fn mods_of(m: &AuxMods) -> Vec<(u32, Vec<u32>)> {
    let mut out: Vec<(u32, Vec<u32>)> = m
        .iter()
        .map(|(&k, o)| {
            let mut ids: Vec<u32> = o.iter().map(|(&id, _)| id).collect();
            ids.sort_unstable();
            (k, ids)
        })
        .collect();
    out.sort_unstable();
    out
}

/// Not installed: nothing registered, the seam answers nothing (the translation calls
/// `IsFurniture`); installed with the row off (vanilla): the handler in, as JIP's own.
#[test]
fn not_installed_nothing_is_put_in() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    assert!(!e.site_owners(0x0055_a488).contains(&Owner::Jip));
    assert_eq!(e.site(0x0055_a488, &[REF]), None);
    let mut e = world(FURNITURE, 0x0100_0001);
    super::super::set_row(&mut e, false);
    assert_eq!(e.jip_patches(), jip_patches::INSTALLED);
    assert!(e.site_owners(0x0055_a488).contains(&Owner::Jip));
    assert_eq!(destroy(&mut e, REF), Some(1));
}

/// The answer is `IsFurniture`'s: furniture 1, anything else 0, a null base 0.
#[test]
fn the_answer_is_the_furniture_test() {
    for (kind, answer) in [(FURNITURE, 1), (0x15, 0), (0x28, 0), (0, 0), (0xff, 0)] {
        let mut e = world(kind, 0x0100_0001);
        assert_eq!(destroy(&mut e, REF), Some(answer), "type {kind:#x}");
    }
    let mut e = world(FURNITURE, 0x0100_0001);
    e.mem.set_u32(REF + 0x20, 0);
    assert_eq!(destroy(&mut e, REF), Some(0), "no base");
}

/// A named and re-modelled reference: both texts go with a user of each hook, their blocks wait
/// for the next text; the flags stay; another reference keeps its name.
#[test]
fn the_name_and_the_model_path_go() {
    let mut e = world(0x15, 0x0100_0001);
    named(&mut e);
    assert_eq!(destroy(&mut e, REF), Some(0));
    assert_eq!(names(&mut e), vec![OTHER]);
    assert!(paths(&mut e).is_empty());
    assert_eq!(e.hook_users(Hook::GetRefName), 1);
    assert_eq!(e.hook_users(Hook::GetModelPath), 0);
    assert_eq!(
        e.ext::<Jip>().world_hooks.free_texts,
        vec![BLOCK, BLOCK_TWO],
        "name first"
    );
    assert_eq!(e.mem.u8(REF + 0x5f), ALT_REF_NAME | REFR_MODEL_PATH | 1);
    // Again: nothing left to take, no user taken.
    assert_eq!(destroy(&mut e, REF), Some(0));
    assert_eq!(e.hook_users(Hook::GetRefName), 1);
    assert_eq!(e.ext::<Jip>().world_hooks.free_texts.len(), 2);
}

/// Each flag decides alone: only the name's flag drops only the name; only the model's flag only
/// the path; neither flag (bits 0, 1, 4 set) keeps both.
#[test]
fn each_flag_decides_its_text() {
    for (flags, left_names, left_paths) in [
        (ALT_REF_NAME, vec![OTHER], vec![REF]),
        (REFR_MODEL_PATH, vec![REF, OTHER], vec![]),
        (0x13, vec![REF, OTHER], vec![REF]),
    ] {
        let mut e = world(0x15, 0x0100_0001);
        named(&mut e);
        e.mem.set_u8(REF + 0x5f, flags);
        destroy(&mut e, REF);
        assert_eq!(names(&mut e), left_names, "flags {flags:#x}");
        assert_eq!(paths(&mut e), left_paths, "flags {flags:#x}");
    }
}

/// Projectiles, explosions and references with no base keep everything (JIP returns at once).
#[test]
fn projectiles_explosions_and_no_base_keep_their_texts() {
    for kind in [PROJECTILE, EXPLOSION] {
        let mut e = world(kind, TEMP_ID);
        named(&mut e);
        *aux_maps(&mut e, false) = three_mods();
        assert_eq!(destroy(&mut e, REF), Some(0));
        assert_eq!(names(&mut e), vec![REF, OTHER], "type {kind:#x}");
        assert_eq!(paths(&mut e), vec![REF]);
        assert_eq!(e.hook_users(Hook::GetRefName), 2);
        assert_eq!(mods_of(aux_maps(&mut e, false)).len(), 3);
    }
    let mut e = world(0x15, TEMP_ID);
    named(&mut e);
    e.mem.set_u32(REF + 0x20, 0);
    assert_eq!(destroy(&mut e, REF), Some(0));
    assert_eq!(names(&mut e), vec![REF, OTHER]);
}

/// A temporary reference with no changes in the save: its variables leave every mod, saved and
/// temporary; a mod left with no owner goes, others stay; the saved ones are marked changed.
#[test]
fn a_temporary_references_aux_variables_go() {
    let mut e = world(0x15, TEMP_ID);
    *aux_maps(&mut e, false) = three_mods();
    *aux_maps(&mut e, true) = three_mods();
    e.ext::<Jip>().saved.changed = 0;
    destroy(&mut e, REF);
    let left = vec![(2, vec![0x0100_0001]), (3, vec![0x0100_0001])];
    assert_eq!(mods_of(aux_maps(&mut e, false)), left);
    assert_eq!(mods_of(aux_maps(&mut e, true)), left);
    assert_eq!(e.ext::<Jip>().saved.changed, changed::AUX_VARS);
}

/// Only the saved map's change marks: the temporary map alone clears with no mark, and a saved
/// map without the id marks nothing.
#[test]
fn only_saved_variables_mark_the_change() {
    let mut e = world(0x15, TEMP_ID);
    *aux_maps(&mut e, true) = three_mods();
    let mut saved = AuxMods::new(8);
    saved.emplace(3).emplace_with(0x0100_0001, vars);
    *aux_maps(&mut e, false) = saved;
    e.ext::<Jip>().saved.changed = 0;
    destroy(&mut e, REF);
    assert_eq!(mods_of(aux_maps(&mut e, true)).len(), 2);
    assert_eq!(
        mods_of(aux_maps(&mut e, false)),
        vec![(3, vec![0x0100_0001])]
    );
    assert_eq!(e.ext::<Jip>().saved.changed, 0);
    // A mod that had no owner of the id stays though empty.
    let mut e = world(0x15, TEMP_ID);
    let mut saved = AuxMods::new(8);
    saved.emplace(4);
    saved.emplace(1).emplace_with(TEMP_ID, vars);
    *aux_maps(&mut e, false) = saved;
    destroy(&mut e, REF);
    assert_eq!(mods_of(aux_maps(&mut e, false)), vec![(4, vec![])]);
}

/// A reference from a plugin (mod index not 0xFF), or a temporary one the save has changes of,
/// keeps its variables.
#[test]
fn placed_or_saved_references_keep_their_variables() {
    let id = 0x0100_0801;
    let mut e = world(0x15, id);
    let mut saved = AuxMods::new(8);
    saved.emplace(1).emplace_with(id, vars);
    *aux_maps(&mut e, false) = saved;
    destroy(&mut e, REF);
    assert_eq!(mods_of(aux_maps(&mut e, false)), vec![(1, vec![id])]);
    let mut e = world(0x15, TEMP_ID);
    *aux_maps(&mut e, false) = three_mods();
    // The changes map holds the id.
    e.mem.set_u32(BUCKETS, ENTRY);
    e.mem.set_u32(ENTRY + 4, TEMP_ID);
    e.ext::<Jip>().saved.changed = 0;
    destroy(&mut e, REF);
    assert_eq!(mods_of(aux_maps(&mut e, false)).len(), 3);
    assert_eq!(e.ext::<Jip>().saved.changed, 0);
}

/// `ClearRefAuxVars` on an empty map: nothing, no.
#[test]
fn clearing_an_empty_map_answers_no() {
    let mut mods = AuxMods::new(8);
    assert!(!clear_ref_aux_vars(&mut mods, TEMP_ID));
    let mut mods = three_mods();
    assert!(clear_ref_aux_vars(&mut mods, TEMP_ID));
    assert!(!clear_ref_aux_vars(&mut mods, TEMP_ID), "already gone");
}

/// Only the temporary map holds variables (the saved one empty): the temporary reference's go
/// from it (emptied mods removed) and nothing is marked changed (JIP tests both maps' sizes).
#[test]
fn only_temporary_variables_go_without_a_change() {
    let mut e = world(0x15, TEMP_ID);
    *aux_maps(&mut e, true) = three_mods();
    destroy(&mut e, REF);
    assert_eq!(
        mods_of(aux_maps(&mut e, true)),
        vec![(2, vec![0x0100_0001]), (3, vec![0x0100_0001])]
    );
    assert!(aux_maps(&mut e, false).is_empty());
    assert_eq!(e.ext::<Jip>().saved.changed, 0);
}

/// With both aux maps empty and nothing named, the routine takes no lock: it answers on this
/// engine while a second engine of the run (a real thread's) holds JIP's shared slot (asking for
/// the slot here would be refused: two engines on one thread), and the lock-free flag says the
/// maps may hold nothing.
#[test]
fn a_temporary_reference_goes_while_another_engine_holds_jips_slot() {
    let mut e = world(FURNITURE, TEMP_ID);
    // A first destroy with the maps empty clears the flag the set-up may have left.
    *aux_maps(&mut e, true) = AuxMods::new(8);
    assert_eq!(destroy(&mut e, REF), Some(1));
    assert!(!maps_may_hold(&mut e));
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    engine_core::extend::release_held_for_test();
    assert!(other.ext::<Jip>().installed, "the other engine holds it");
    assert_eq!(destroy(&mut e, REF), Some(1), "no lock taken");
    engine_core::extend::release_held_for_test();
}

/// One temporary map for the run: variables written on another engine (a real thread's) are
/// cleared when the temporary reference is destroyed on this one.
#[test]
fn temporary_variables_written_on_one_engine_go_on_another() {
    let mut e = world(0x15, TEMP_ID);
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    engine_core::extend::release_held_for_test();
    *aux_maps(&mut other, true) = three_mods();
    engine_core::extend::release_held_for_test();
    destroy(&mut e, REF);
    engine_core::extend::release_held_for_test();
    assert_eq!(
        mods_of(aux_maps(&mut other, true)),
        vec![(2, vec![0x0100_0001]), (3, vec![0x0100_0001])]
    );
    engine_core::extend::release_held_for_test();
}

/// The flag is exact whenever JIP's slot is free: through writes on one engine and destroys on
/// another (each holding the slot in turn), it is true whenever either map holds a variable and
/// false only after a destroy found both empty.
#[test]
fn the_may_hold_flag_follows_writes_and_destroys_on_two_engines() {
    let mut e = world(0x15, TEMP_ID);
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    let release = engine_core::extend::release_held_for_test;
    let holds = |e: &mut Engine| {
        let jip = e.ext::<Jip>();
        !(jip.saved.aux_vars.is_empty() && jip.temp_aux_vars.is_empty())
    };
    release();
    // Empty maps: a destroy clears the flag.
    destroy(&mut e, REF);
    assert!(!maps_may_hold(&mut e));
    release();
    // A write on the other engine sets it, a destroy of another id keeps it.
    *aux_maps(&mut other, true) = three_mods();
    release();
    e.mem.set_u32(REF + 0xc, 0xff00_0999);
    destroy(&mut e, REF);
    assert!(holds(&mut e) && maps_may_hold(&mut e));
    release();
    // Emptied by a write: the next destroy clears the flag; a new write sets it again.
    *aux_maps(&mut other, true) = AuxMods::new(8);
    release();
    destroy(&mut e, REF);
    assert!(!holds(&mut e) && !maps_may_hold(&mut e));
    release();
    *aux_maps(&mut other, false) = three_mods();
    release();
    assert!(maps_may_hold(&mut e));
    e.mem.set_u32(REF + 0xc, TEMP_ID);
    destroy(&mut e, REF);
    assert!(holds(&mut e) && maps_may_hold(&mut e), "mod 3 keeps its owner");
    release();
}
