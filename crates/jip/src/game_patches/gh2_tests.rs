//! Group GH2's game patches. H24 (`DestroyRefrHook`): each branch of JIP's routine, over a
//! stand-in reference (base `+0x20`, id `+0xC`, JIP's byte `+0x5F`), its base form (type `+4`)
//! and the save's changes map (`g_BGSSaveLoadGame` `0x011DDF38` -> its map at `+0`: hash size
//! `+4`, table `+8`; an entry's next `+0`, key `+4`).

use super::*;
use crate::commands::jip_fn_aux_variable::aux_maps;
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
    let mut s = crate::world_hooks::state(e);
    s.ref_names.insert(REF, BLOCK);
    s.model_paths.insert(REF, BLOCK_TWO);
    s.ref_names.insert(OTHER, BLOCK_TWO);
    drop(s);
    e.add_hook_user(Hook::GetRefName, true);
    e.add_hook_user(Hook::GetRefName, true);
    e.add_hook_user(Hook::GetModelPath, true);
}

fn names(e: &mut Engine) -> Vec<u32> {
    crate::world_hooks::state(e)
        .ref_names
        .keys()
        .copied()
        .collect()
}

fn paths(e: &mut Engine) -> Vec<u32> {
    crate::world_hooks::state(e)
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
    // On any thread (loader threads when cells unload): the texts go while another engine holds
    // JIP's shared slot (they are in the world hooks' lock-free slot).
    assert_eq!(
        crate::test_support::with_jips_slot_held(&mut e, |e| destroy(e, REF)),
        Some(0)
    );
    assert_eq!(names(&mut e), vec![OTHER]);
    assert!(paths(&mut e).is_empty());
    assert_eq!(e.hook_users(Hook::GetRefName), 1);
    assert_eq!(e.hook_users(Hook::GetModelPath), 0);
    assert_eq!(
        crate::world_hooks::state(&mut e).free_texts,
        vec![BLOCK, BLOCK_TWO],
        "name first"
    );
    assert_eq!(e.mem.u8(REF + 0x5f), ALT_REF_NAME | REFR_MODEL_PATH | 1);
    // Again: nothing left to take, no user taken.
    assert_eq!(destroy(&mut e, REF), Some(0));
    assert_eq!(e.hook_users(Hook::GetRefName), 1);
    assert_eq!(crate::world_hooks::state(&mut e).free_texts.len(), 2);
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
        assert_eq!(mods_of(&aux_maps(&mut e, false)).len(), 3);
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
    crate::cosave::set_changed(&mut e, 0);
    destroy(&mut e, REF);
    let left = vec![(2, vec![0x0100_0001]), (3, vec![0x0100_0001])];
    assert_eq!(mods_of(&aux_maps(&mut e, false)), left);
    assert_eq!(mods_of(&aux_maps(&mut e, true)), left);
    assert_eq!(crate::cosave::changed_flags(&mut e), changed::AUX_VARS);
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
    crate::cosave::set_changed(&mut e, 0);
    destroy(&mut e, REF);
    assert_eq!(mods_of(&aux_maps(&mut e, true)).len(), 2);
    assert_eq!(
        mods_of(&aux_maps(&mut e, false)),
        vec![(3, vec![0x0100_0001])]
    );
    assert_eq!(crate::cosave::changed_flags(&mut e), 0);
    // A mod that had no owner of the id stays though empty.
    let mut e = world(0x15, TEMP_ID);
    let mut saved = AuxMods::new(8);
    saved.emplace(4);
    saved.emplace(1).emplace_with(TEMP_ID, vars);
    *aux_maps(&mut e, false) = saved;
    destroy(&mut e, REF);
    assert_eq!(mods_of(&aux_maps(&mut e, false)), vec![(4, vec![])]);
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
    assert_eq!(mods_of(&aux_maps(&mut e, false)), vec![(1, vec![id])]);
    let mut e = world(0x15, TEMP_ID);
    *aux_maps(&mut e, false) = three_mods();
    // The changes map holds the id.
    e.mem.set_u32(BUCKETS, ENTRY);
    e.mem.set_u32(ENTRY + 4, TEMP_ID);
    crate::cosave::set_changed(&mut e, 0);
    destroy(&mut e, REF);
    assert_eq!(mods_of(&aux_maps(&mut e, false)).len(), 3);
    assert_eq!(crate::cosave::changed_flags(&mut e), 0);
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
        mods_of(&aux_maps(&mut e, true)),
        vec![(2, vec![0x0100_0001]), (3, vec![0x0100_0001])]
    );
    assert!(aux_maps(&mut e, false).is_empty());
    assert_eq!(crate::cosave::changed_flags(&mut e), 0);
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
    assert!(
        other.ext::<crate::Jip>().installed,
        "the other engine holds it"
    );
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
        mods_of(&aux_maps(&mut other, true)),
        vec![(2, vec![0x0100_0001]), (3, vec![0x0100_0001])]
    );
    engine_core::extend::release_held_for_test();
}

// ---- PR 2: G06, G73, G103 ----

/// A reference at `REF` of the base `BASE_FORM`, the base's ash pile bit as `ash`.
fn pile(ash: bool) -> Engine {
    let e = world(0x15, 0x0100_0001);
    if ash {
        e.mem.set_u8(BASE_FORM + 6, IS_ASH_PILE);
    }
    e
}

/// Not installed: none of G06's eight seams, G73's or G103's two has a handler; installed with the
/// row off: G06's and G73's in (JIP's own), G103's not; the row on: G103's in too.
#[test]
fn ash_piles_not_installed_installed_and_the_row() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    for site in ASH_PILE_TESTS {
        assert!(!e.site_owners(site.addr()).contains(&Owner::Jip));
        assert_eq!(e.site(site.addr(), &[REF]), None);
    }
    for site in [ATTACH_ASH_PILE, DELETED_ACTOR_ASH_PILE, CLEAR_ASH_PILES] {
        assert!(!e.site_owners(site.addr()).contains(&Owner::Jip));
    }
    let mut e = pile(true);
    super::super::set_row(&mut e, false);
    for site in ASH_PILE_TESTS {
        assert_eq!(e.site(site.addr(), &[REF]), Some(1), "{:#x}", site.addr());
    }
    assert!(e.site_owners(ATTACH_ASH_PILE.addr()).contains(&Owner::Jip));
    assert_eq!(e.site(DELETED_ACTOR_ASH_PILE.addr(), &[REF]), None);
    assert_eq!(e.site(CLEAR_ASH_PILES.addr(), &[REF, BASE_FORM]), None);
    // No `jip_nvse.ini` (JIP's defaults); the 3.9a fix's wrapped `GetOwnerTopic` stood in.
    e.register(0x00fd_f088, |_, _| Ret {
        eax: 0xffff_ffff,
        ..Ret::default()
    });
    e.register(0x0061_a310, |_, _| Ret::default());
    super::super::set_row(&mut e, true);
    assert_eq!(e.site(DELETED_ACTOR_ASH_PILE.addr(), &[REF]), Some(1));
    // Not a pile: G103's routine answers (the exe's type branches passed over), nothing deleted.
    e.mem.set_u8(BASE_FORM + 6, 0);
    assert_eq!(e.site(CLEAR_ASH_PILES.addr(), &[REF, BASE_FORM]), Some(0));
}

/// G06: the base's bit 0x20 of `jipFormFlags6` alone answers 1; other bits, a clear byte and a
/// reference with no base answer 0.
#[test]
fn the_ash_pile_test_is_the_bases_bit() {
    let mut e = pile(true);
    assert_eq!(is_ash_pile(&mut e, &[REF]), Some(1));
    e.mem.set_u8(BASE_FORM + 6, 0xff);
    assert_eq!(is_ash_pile(&mut e, &[REF]), Some(1));
    e.mem.set_u8(BASE_FORM + 6, !IS_ASH_PILE);
    assert_eq!(is_ash_pile(&mut e, &[REF]), Some(0));
    e.mem.set_u8(BASE_FORM + 6, 0);
    assert_eq!(is_ash_pile(&mut e, &[REF]), Some(0));
    e.mem.set_u32(REF + 0x20, 0);
    assert_eq!(is_ash_pile(&mut e, &[REF]), Some(0), "no base");
}

/// JIP's `DeferredInit`: the two vanilla piles (`0x11CA27C`, `0x11CA280`) get the bit, kept by
/// their ids for the restore after a load; other bits stay; a missing form is passed over; with
/// JIP not installed nothing is written.
#[test]
fn deferred_init_marks_the_two_vanilla_piles() {
    let ash = AREA_AT + 0x900;
    let goo = AREA_AT + 0x980;
    let setup = |e: &mut Engine| {
        e.map(0x011c_a000, 0x1000);
        e.mem.set_u32(0x011c_a27c, ash);
        e.mem.set_u32(0x011c_a280, goo);
        e.mem.set_u32(ash + 0xc, 0x0001_0001);
        e.mem.set_u32(goo + 0xc, 0x0001_0002);
        e.mem.set_u8(goo + 6, 1);
    };
    let mut e = world(0x15, 0x0100_0001);
    setup(&mut e);
    deferred_init(&mut e);
    assert_eq!(e.mem.u8(ash + 6), IS_ASH_PILE);
    assert_eq!(e.mem.u8(goo + 6), IS_ASH_PILE | 1);
    let kept = crate::world_hooks::state(&mut e).form_flags.clone();
    assert_eq!(kept.get(&0x0001_0001), Some(&IS_ASH_PILE));
    assert_eq!(kept.get(&0x0001_0002), Some(&(IS_ASH_PILE | 1)));
    // the cosave listener's case
    let mut e = world(0x15, 0x0100_0001);
    setup(&mut e);
    e.mem.set_u32(0x011c_a280, 0);
    e.ext::<crate::Jip>().pos_marker.marker = 1;
    crate::cosave::listen(
        &mut e,
        &nvse::plugins::Message {
            kind: nvse::plugins::msg::DEFERRED_INIT,
            data: 0,
            len: 0,
        },
    );
    assert_eq!(e.mem.u8(ash + 6), IS_ASH_PILE, "through the listener");
    assert_eq!(e.mem.u8(goo + 6), 1, "a missing form is passed over");
    // not installed
    let mut e = engine();
    crate::install_as(&mut e, false);
    e.map(AREA_AT, 0x1000);
    setup(&mut e);
    deferred_init(&mut e);
    assert_eq!(e.mem.u8(ash + 6), 0);
    assert_eq!(e.mem.u8(goo + 6), 1);
}

/// G73: `AttachAshPileEx`'s form once (then the exe's choice again); mode 2 the goo pile, any
/// other the ash pile.
#[test]
fn a_deaths_ash_pile_is_the_alternative_once_else_the_exes() {
    let mut e = world(0x15, 0x0100_0001);
    e.map(0x011c_a000, 0x1000);
    e.mem.set_u32(0x011c_a27c, 0x0aaa_0000);
    e.mem.set_u32(0x011c_a280, 0x0bbb_0000);
    let pick = |e: &mut Engine, mode: u32| e.site(ATTACH_ASH_PILE.addr(), &[mode]);
    for (mode, form) in [
        (2, 0x0bbb_0000),
        (0, 0x0aaa_0000),
        (1, 0x0aaa_0000),
        (3, 0x0aaa_0000),
    ] {
        assert_eq!(pick(&mut e, mode), Some(form), "mode {mode}");
    }
    e.ext_free::<AltAshPile>()
        .0
        .store(0x0ccc_0000, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(pick(&mut e, 2), Some(0x0ccc_0000));
    assert_eq!(pick(&mut e, 2), Some(0x0bbb_0000), "taken once");
}

/// G103's references: the pile at `REF` (base `BASE_FORM`, ash pile bit), the actor it came
/// from at `OTHER` (base at `AREA_AT + 0xa00`), `ExtraAshPileRef` stand-ins, `MarkAsDeleted`
/// logged.
fn cleanup_world(source: u32) -> Engine {
    let mut e = crate::commands::test_support::engine();
    crate::commands::test_support::extra_stand_ins(&mut e);
    e.map(AREA_AT, 0x1000);
    e.mem.set_u32(REF + 0x20, BASE_FORM);
    e.mem.set_u8(BASE_FORM + 6, IS_ASH_PILE);
    e.mem.set_u32(OTHER + 0x20, AREA_AT + 0xa00);
    e.register(0x0057_2270, |_, _| Ret::default());
    let pile_list = REF + 0x44;
    crate::commands::test_support::put_extra(&mut e, pile_list, AREA_AT + 0xb00, 0x89, source);
    crate::commands::test_support::put_extra(&mut e, OTHER + 0x44, AREA_AT + 0xb40, 0x89, REF);
    e.call_log = Some(vec![]);
    e
}

fn deleted(e: &Engine) -> Vec<Vec<u32>> {
    e.call_log
        .as_ref()
        .unwrap()
        .iter()
        .filter(|c| c.0 == 0x0057_2270)
        .map(|c| c.1.clone())
        .collect()
}

fn has_ash_ref(e: &Engine, reference: u32) -> bool {
    crate::commands::extra::has_type(e, reference + 0x44, 0x89)
}

/// G103 `ClearAshPilesHook`: an ash pile from an actor loses its `ExtraAshPileRef`, the actor
/// its own, and the pile is deleted; the routine always answers (the exe's type branches are
/// passed over).
#[test]
fn a_cleaned_ash_pile_is_deleted_and_its_actor_forgets_it() {
    let mut e = cleanup_world(OTHER);
    assert_eq!(clear_ash_piles(&mut e, &[REF, BASE_FORM]), Some(0));
    assert!(!has_ash_ref(&e, REF));
    assert!(!has_ash_ref(&e, OTHER));
    assert_eq!(deleted(&e), vec![vec![REF]]);
}

/// The source's base form flagged `0x400` (quest item): the pile stays, both keep their extras.
#[test]
fn a_quest_items_ash_pile_stays() {
    let mut e = cleanup_world(OTHER);
    e.mem.set_u8(AREA_AT + 0xa00 + 9, 4);
    assert_eq!(clear_ash_piles(&mut e, &[REF, BASE_FORM]), Some(0));
    assert!(has_ash_ref(&e, REF));
    assert!(has_ash_ref(&e, OTHER));
    assert!(deleted(&e).is_empty());
    // another bit of the flags byte does not keep it
    let mut e = cleanup_world(OTHER);
    e.mem.set_u8(AREA_AT + 0xa00 + 9, !4);
    clear_ash_piles(&mut e, &[REF, BASE_FORM]);
    assert_eq!(deleted(&e), vec![vec![REF]]);
    // a source with no base: taken as the flag clear (JIP would read address 9)
    let mut e = cleanup_world(OTHER);
    e.mem.set_u32(OTHER + 0x20, 0);
    clear_ash_piles(&mut e, &[REF, BASE_FORM]);
    assert_eq!(deleted(&e), vec![vec![REF]]);
    assert!(!has_ash_ref(&e, OTHER));
}

/// An `ExtraAshPileRef` with no source: the pile loses it and is deleted; no extra at all: just
/// deleted; a base without the bit: nothing at all.
#[test]
fn a_pile_with_no_source_or_no_extra_is_deleted_and_others_are_left() {
    let mut e = cleanup_world(0);
    clear_ash_piles(&mut e, &[REF, BASE_FORM]);
    assert!(!has_ash_ref(&e, REF));
    assert!(has_ash_ref(&e, OTHER), "nobody else touched");
    assert_eq!(deleted(&e), vec![vec![REF]]);
    let mut e = cleanup_world(OTHER);
    crate::commands::extra::remove_by_type(&mut e, REF + 0x44, 0x89);
    e.call_log = Some(vec![]);
    clear_ash_piles(&mut e, &[REF, BASE_FORM]);
    assert_eq!(deleted(&e), vec![vec![REF]]);
    assert!(has_ash_ref(&e, OTHER));
    let mut e = cleanup_world(OTHER);
    e.mem.set_u8(BASE_FORM + 6, !IS_ASH_PILE);
    assert_eq!(clear_ash_piles(&mut e, &[REF, BASE_FORM]), Some(0));
    assert!(deleted(&e).is_empty());
    assert!(has_ash_ref(&e, REF) && has_ash_ref(&e, OTHER));
}

/// The flag is exact whenever the aux maps' lock is free: through writes on one engine and
/// destroys on another (each taking the lock in turn), it is true whenever either map holds a variable and
/// false only after a destroy found both empty.
#[test]
fn the_may_hold_flag_follows_writes_and_destroys_on_two_engines() {
    let mut e = world(0x15, TEMP_ID);
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    let release = engine_core::extend::release_held_for_test;
    let holds = |e: &mut Engine| {
        let maps = crate::commands::jip_fn_aux_variable::aux_pair(e);
        !(maps.saved.is_empty() && maps.temp.is_empty())
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
    assert!(maps_may_hold(&mut e) && holds(&mut e));
    release();
    // Emptied by a write: the next destroy clears the flag; a new write sets it again.
    *aux_maps(&mut other, true) = AuxMods::new(8);
    release();
    destroy(&mut e, REF);
    // The flag first: a look at the maps (`aux_pair`) sets it, as any reach does.
    assert!(!maps_may_hold(&mut e) && !holds(&mut e));
    release();
    *aux_maps(&mut other, false) = three_mods();
    release();
    assert!(maps_may_hold(&mut e));
    e.mem.set_u32(REF + 0xc, TEMP_ID);
    destroy(&mut e, REF);
    assert!(
        maps_may_hold(&mut e) && holds(&mut e),
        "mod 3 keeps its owner"
    );
    release();
}

/// The aux-variable part takes the aux maps' own lock (`AuxMaps`), not JIP's shared slot: on
/// any thread (loader threads at cell unload) a temporary reference with aux variables loses
/// them while another engine of the run holds JIP's slot.
#[test]
fn a_temporary_reference_with_aux_variables_goes_while_another_engine_holds_jips_slot() {
    let mut e = world(0x15, TEMP_ID);
    *aux_maps(&mut e, true) = three_mods();
    crate::test_support::with_jips_slot_held(&mut e, |e| destroy(e, REF));
    assert_eq!(
        mods_of(&aux_maps(&mut e, true)),
        vec![(2, vec![0x0100_0001]), (3, vec![0x0100_0001])]
    );
}

/// A thread's engine made right after install (`share_slots_for_test`) reads the aux maps the
/// main engine's writers fill, and the destructor run on it empties them for the main engine:
/// one pair of maps for the run.
#[test]
fn a_threads_engine_shares_the_aux_maps() {
    let mut e = world(0x15, TEMP_ID);
    let mut t = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut t);
    engine_core::extend::release_held_for_test();
    *aux_maps(&mut e, true) = three_mods();
    assert_eq!(
        aux_maps(&mut t, true).len(),
        3,
        "written on the main engine"
    );
    engine_core::extend::release_held_for_test();
}

/// The test engine handed between two OS threads behind a mutex, as stepped threads hand one
/// engine over (`Engine` is not `Send`: its handlers are `Rc`s).
struct Stepped(Engine);
// SAFETY: reached only through the `Mutex` below, by one thread at a time, and nothing of it is
// kept outside the lock.
unsafe impl Send for Stepped {}

/// The hand-over of one engine between two OS threads (as stepped threads run it; not contention:
/// the two sides run in turn): the variables written on the main thread are the ones the other
/// thread's destructor erases, one pair of maps for every thread.
#[test]
fn the_destructor_on_another_thread_erases_what_the_main_thread_wrote() {
    let e = world(0x15, TEMP_ID);
    engine_core::extend::release_held_for_test();
    let engine = std::sync::Mutex::new(Stepped(e));
    let lock = || {
        engine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    };
    {
        let mut g = lock();
        *aux_maps(&mut g.0, true) = three_mods();
    }
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let mut g = lock();
                assert_eq!(destroy(&mut g.0, REF), Some(0));
                engine_core::extend::release_held_for_test();
            })
            .join()
            .expect("ran");
    });
    let mut g = lock();
    assert_eq!(
        mods_of(&aux_maps(&mut g.0, true)),
        vec![(2, vec![0x0100_0001]), (3, vec![0x0100_0001])]
    );
    drop(g);
    engine_core::extend::release_held_for_test();
}

/// The saved map too, under the held slot: a temporary reference's saved variables go, and the
/// aux variables are marked changed (the lock-free half of the flags), without JIP's slot.
#[test]
fn saved_variables_go_and_are_marked_while_another_engine_holds_jips_slot() {
    let mut e = world(0x15, TEMP_ID);
    *aux_maps(&mut e, false) = three_mods();
    crate::cosave::set_changed(&mut e, 0);
    crate::test_support::with_jips_slot_held(&mut e, |e| destroy(e, REF));
    assert_eq!(
        mods_of(&aux_maps(&mut e, false)),
        vec![(2, vec![0x0100_0001]), (3, vec![0x0100_0001])]
    );
    assert_eq!(crate::cosave::changed_flags(&mut e), changed::AUX_VARS);
}

/// Contention on the "may hold" flag: one OS thread adds and clears a temporary variable under
/// the maps' lock while another runs the destructor's aux part (`erase_owner`, another id) as
/// fast as it can, both released together by a barrier. Right after each add the flag must say
/// "may hold": the flag is set under the lock, and the destructor clears it only while it finds
/// both maps empty under the same lock. A flag stored before the lock (a destructor that found
/// the maps empty clears it after the store, then the add lands) breaks this; that interleaving
/// is timing-dependent, so this test catches such a fault only probabilistically (many rounds).
#[test]
fn the_may_hold_flag_holds_under_contention() {
    const ROUNDS: u32 = 200_000;
    let mut e = world(0x15, TEMP_ID);
    let maps = e.ext_free::<AuxMaps>();
    let barrier = std::sync::Barrier::new(2);
    let missed = std::sync::atomic::AtomicU32::new(0);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            barrier.wait();
            for _ in 0..ROUNDS {
                maps.lock().temp.emplace(1).emplace(TEMP_ID);
                if !maps.may_hold() {
                    missed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                maps.lock().temp.clear();
            }
        });
        scope.spawn(|| {
            barrier.wait();
            for _ in 0..ROUNDS {
                if maps.may_hold() {
                    erase_owner(maps, 0x0100_0999);
                }
            }
        });
    });
    assert_eq!(
        missed.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "a variable present while the flag said empty"
    );
}

/// Waits up to half a second for `done` (the other thread's step), polling each millisecond.
fn wait_briefly_for(done: &std::sync::atomic::AtomicBool) {
    let start = std::time::Instant::now();
    while !done.load(std::sync::atomic::Ordering::SeqCst)
        && start.elapsed() < std::time::Duration::from_millis(500)
    {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

/// The copy `AuxVarGetFltCond` reads of variable `v` of `TEMP_ID` (mod 1, temporary).
fn copy_of_v(maps: &AuxMaps) -> Option<f64> {
    maps.first_value((true, 1, TEMP_ID), crate::containers::str_hash_ci(b"v"))
}

/// The copy `AuxVarGetFltCond` reads changes in the same take of the maps' lock as the maps: a
/// destructor's erase on another OS thread that comes after a command's publish read the owner's
/// values and before it wrote them waits for that write, then erases the owner from the copy
/// too. With the two in separate takes the erase lands in between and the publish then writes
/// the destroyed owner's values (a later reference given the same id would answer with them).
#[test]
fn a_destroy_during_a_publish_leaves_no_values_of_the_destroyed_owner() {
    let mut e = world(0x15, TEMP_ID);
    let maps = e.ext_free::<AuxMaps>();
    maps.lock().temp.emplace(1).emplace_with(TEMP_ID, vars);
    let done = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        maps.publish_owner_with(true, 1, TEMP_ID, || {
            scope.spawn(|| {
                assert_eq!(erase_owner(maps, TEMP_ID), Some(false));
                done.store(true, std::sync::atomic::Ordering::SeqCst);
            });
            wait_briefly_for(&done);
        });
    });
    assert!(maps.lock().temp.is_empty(), "the owner left the maps");
    assert_eq!(
        copy_of_v(maps),
        None,
        "the copy keeps no value of the destroyed owner"
    );
}

/// The mirror: a command's set and publish on another OS thread that comes after the
/// destructor's erase left the maps and before it erased the copy waits for that erase, then
/// sets and publishes. With the two in separate takes the copy's erase lands after the publish
/// and drops the new value the maps hold (the condition would answer nothing).
#[test]
fn a_publish_during_a_destroy_keeps_the_new_value_in_the_copy() {
    let mut e = world(0x15, TEMP_ID);
    let maps = e.ext_free::<AuxMaps>();
    maps.lock().temp.emplace(1).emplace_with(TEMP_ID, vars);
    maps.publish_owner_with(true, 1, TEMP_ID, || {});
    assert_eq!(copy_of_v(maps), Some(1.0));
    let done = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        let erased = erase_owner_with(maps, TEMP_ID, || {
            scope.spawn(|| {
                maps.lock().temp.emplace(1).emplace_with(TEMP_ID, vars);
                maps.publish_owner_with(true, 1, TEMP_ID, || {});
                done.store(true, std::sync::atomic::Ordering::SeqCst);
            });
            wait_briefly_for(&done);
        });
        assert_eq!(erased, Some(false));
    });
    assert_eq!(
        mods_of(&maps.lock().temp),
        vec![(1, vec![TEMP_ID])],
        "set again after the erase"
    );
    assert_eq!(
        copy_of_v(maps),
        Some(1.0),
        "the copy holds what the maps hold"
    );
}

// ---- GH2 part 3: G31, G78, G81, G90, G106 (fix rows) ----------------------------------------

/// JIP installed with the fix row on (no `jip_nvse.ini`; the 3.9a fix's wrapped `GetOwnerTopic`
/// stood in), over the stand-in area.
fn row_on() -> Engine {
    let mut e = world(0x15, 0x0100_0001);
    e.register(0x00fd_f088, |_, _| Ret {
        eax: 0xffff_ffff,
        ..Ret::default()
    });
    e.register(0x0061_a310, |_, _| Ret::default());
    super::super::set_row(&mut e, true);
    e
}

const PART_3_SITES: [SiteSlot; 6] = [
    DOOR_ANSWER_STANDS,
    SET_SCALE,
    CLEAR_WEAPON_NODE[0],
    CLEAR_WEAPON_NODE[1],
    CLOUDS_FIX,
    UPDATE_WATER_TEXTURES,
];

/// The five rows' six seams: no handler while JIP is not installed or the fix row is off; the
/// row on puts all six in.
#[test]
fn part_3_seams_only_with_the_row() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    for site in PART_3_SITES {
        assert!(!e.site_owners(site.addr()).contains(&Owner::Jip));
    }
    let mut e = world(0x15, 0x0100_0001);
    super::super::set_row(&mut e, false);
    for site in PART_3_SITES {
        assert_eq!(e.site(site.addr(), &[REF, 0]), None, "{:#x}", site.addr());
    }
    let mut e = row_on();
    assert_eq!(e.site(DOOR_ANSWER_STANDS.addr(), &[REF, 0]), Some(1));
    assert_eq!(
        e.site(SET_SCALE.addr(), &[2.0f32.to_bits()]),
        Some(2.0f32.to_bits())
    );
    for site in PART_3_SITES {
        assert!(
            e.site_owners(site.addr()).contains(&Owner::Jip),
            "{:#x}",
            site.addr()
        );
    }
}

/// G31: the door's answer stands whatever it is (JIP's jump is unconditional).
#[test]
fn g31_the_door_answer_always_stands() {
    let mut e = engine();
    for door in [0, 1] {
        assert_eq!(door_answer_stands(&mut e, &[REF, door]), Some(1));
    }
}

/// G78 `SetScaleHook`: `maxss` with `0x38D1B717` then `minss` with 100: the bounds kept, the
/// values between untouched (no rounding to two decimals), a NaN the lower bound, infinities
/// clamped, zero and negatives the lower bound.
#[test]
fn g78_the_scale_is_clamped_not_rounded() {
    let mut e = engine();
    let clamp =
        |e: &mut Engine, v: f32| f32::from_bits(set_scale_clamp(e, &[v.to_bits()]).unwrap());
    let min = f32::from_bits(0x38d1_b717);
    assert!((min - 0.0001).abs() < 1e-9);
    assert_eq!(clamp(&mut e, 1.2345), 1.2345);
    assert_eq!(clamp(&mut e, 0.005), 0.005, "below the exe's 0.01");
    assert_eq!(clamp(&mut e, 50.0), 50.0, "above the exe's 10");
    assert_eq!(clamp(&mut e, 100.0), 100.0);
    assert_eq!(clamp(&mut e, 100.5), 100.0);
    assert_eq!(clamp(&mut e, f32::INFINITY), 100.0);
    assert_eq!(clamp(&mut e, min), min);
    assert_eq!(clamp(&mut e, 0.0), min);
    assert_eq!(clamp(&mut e, -0.0), min);
    assert_eq!(clamp(&mut e, -3.0), min);
    assert_eq!(clamp(&mut e, f32::NEG_INFINITY), min);
    assert_eq!(
        clamp(&mut e, f32::NAN),
        min,
        "maxss gives its second operand"
    );
}

const NODE: u32 = AREA_AT + 0x900;
const NODE_VTABLE: u32 = AREA_AT + 0xa00;
const CHILDREN_AT: u32 = AREA_AT + 0xc00;
const DETACH: u32 = 0x00fe_0e80;

/// A node (vtable with the detach slot `+0xE8` recorded) whose children array holds `children`
/// (set or an empty slot, and the flag byte `+0x33`) and whose count word says `count`; the
/// children at `AREA_AT + 0xd00 + i * 0x40`.
fn node_world(children: &[(bool, u8)], count: u16) -> (Engine, Vec<u32>) {
    let mut e = engine();
    e.map(AREA_AT, 0x1000);
    e.mem.set_u32(NODE, NODE_VTABLE);
    e.mem.set_u32(NODE_VTABLE + 0xe8, DETACH);
    e.register(DETACH, |_, _| Ret::default());
    e.mem.set_u32(NODE + 0xa0, CHILDREN_AT);
    e.mem.set_u16(NODE + 0xa6, count);
    let mut all = Vec::new();
    for (i, &(set, flags)) in children.iter().enumerate() {
        let child = AREA_AT + 0xd00 + i as u32 * 0x40;
        if set {
            e.mem.set_u8(child + 0x33, flags);
            e.mem.set_u32(CHILDREN_AT + i as u32 * 4, child);
        }
        all.push(child);
    }
    e.call_log = Some(vec![]);
    (e, all)
}

fn detached(e: &Engine) -> Vec<Vec<u32>> {
    e.call_log
        .as_ref()
        .unwrap()
        .iter()
        .filter(|c| c.0 == DETACH)
        .map(|c| c.1.clone())
        .collect()
}

/// G81 `ClearWeaponNodeHook`: from the last child to the first, each one set and without bit
/// 0x80 of its byte `+0x33` is detached through the node's virtual `+0xE8`; empty slots and kept
/// children are passed over; only the count word's number of slots is read.
#[test]
fn g81_the_weapon_nodes_children_are_detached_last_first() {
    let (mut e, c) = node_world(
        &[(true, 0), (false, 0), (true, 0x80), (true, 0x7f), (true, 0)],
        4,
    );
    assert_eq!(clear_weapon_node(&mut e, &[NODE]), Some(0));
    assert_eq!(detached(&e), vec![vec![NODE, c[3]], vec![NODE, c[0]]]);
    let (mut e, _) = node_world(&[], 0);
    assert_eq!(clear_weapon_node(&mut e, &[NODE]), Some(0));
    assert!(detached(&e).is_empty(), "no children");
}

/// G90 `CloudsFixHook`: the interior cell (`TES+0x34`) cleared at once; the sky (`TES+0x68`)
/// loses its transition weather (`+0x14`) at the next main loop, once, then the callback goes.
#[test]
fn g90_the_weather_transition_ends_at_the_next_main_loop() {
    let mut e = row_on();
    let (tes, sky) = (AREA_AT + 0x900, AREA_AT + 0xa00);
    e.mem.set_u32(tes + 0x34, 0x1234);
    e.mem.set_u32(tes + 0x68, sky);
    e.mem.set_u32(sky + 0x14, 0x5678);
    e.mem.set_u32(sky + 0x10, 0x9abc);
    assert_eq!(e.site(CLOUDS_FIX.addr(), &[tes]), Some(0));
    assert_eq!(e.mem.u32(tes + 0x34), 0);
    assert_eq!(e.mem.u32(sky + 0x14), 0x5678, "not yet");
    assert_eq!(
        e.ext::<crate::Jip>().main_loop.callbacks.len(),
        1,
        "one callback"
    );
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::MAIN_GAME_LOOP, 0, 0);
    assert_eq!(e.mem.u32(sky + 0x14), 0, "the transition weather cleared");
    assert_eq!(e.mem.u32(sky + 0x10), 0x9abc, "the current weather kept");
    assert!(e.ext::<crate::Jip>().main_loop.callbacks.is_empty(), "once");
    e.mem.set_u32(sky + 0x14, 0x5678);
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::MAIN_GAME_LOOP, 0, 0);
    assert_eq!(e.mem.u32(sky + 0x14), 0x5678, "not again");
}

/// JIP's own functions in the main-loop list: a call after `call_delay` frames, `call_count`
/// times, then gone; a null sky is left alone.
#[test]
fn native_main_loop_callbacks_count_their_calls() {
    let mut e = row_on();
    let sky = AREA_AT + 0xa00;
    crate::main_loop::add_native(&mut e, crate::main_loop::Native::EndWeatherTransition, 0);
    crate::main_loop::add_native(&mut e, crate::main_loop::Native::EndWeatherTransition, sky);
    {
        let list = &mut e.ext::<crate::Jip>().main_loop.callbacks;
        list[1].call_count = 2;
        list[1].call_delay = 2;
        list[1].cycle_count = 2;
    }
    let cycle = |e: &mut Engine| {
        e.mem.set_u32(sky + 0x14, 7);
        crate::main_loop::cycle(e);
        e.mem.u32(sky + 0x14) == 0
    };
    assert!(!cycle(&mut e), "frame 1 of 2");
    assert_eq!(
        e.ext::<crate::Jip>().main_loop.callbacks.len(),
        1,
        "the null sky's went after its call"
    );
    assert!(cycle(&mut e), "frame 2: called");
    assert!(!cycle(&mut e));
    assert!(cycle(&mut e), "second call");
    assert!(e.ext::<crate::Jip>().main_loop.callbacks.is_empty());
}

const WATER_MAP: u32 = AREA_AT + 0x900;
const WATER_BUCKETS: u32 = AREA_AT + 0xa00;

/// A map of 3 buckets holding `entries` (key, value), each put at the head of its chain.
fn water_world(entries: &[(u32, u32)]) -> Engine {
    let mut e = engine();
    e.map(AREA_AT, 0x1000);
    e.map(0x011f_9000, 0x1000);
    e.mem.set_u32(WATER_MAP + 4, 3);
    e.mem.set_u32(WATER_MAP + 8, WATER_BUCKETS);
    for (i, &(key, value)) in entries.iter().enumerate() {
        let entry = AREA_AT + 0xb00 + i as u32 * 0x10;
        let bucket = WATER_BUCKETS + (key % 3) * 4;
        e.mem.set_u32(entry, e.mem.u32(bucket));
        e.mem.set_u32(entry + 4, key);
        e.mem.set_u32(entry + 8, value);
        e.mem.set_u32(bucket, entry);
    }
    e
}

/// G106 `UpdateWaterTexturesHook`: the renderer's byte `0x11F91C5` when set; else the map's
/// value for the form (a chain followed), 0 when it has none or the map has no buckets.
#[test]
fn g106_the_byte_then_the_maps_value() {
    let mut e = water_world(&[(0x10, 1), (0x13, 0x200), (0x11, 5)]);
    let ask = |e: &mut Engine, key: u32| update_water_textures(e, &[WATER_MAP, key]).unwrap();
    assert_eq!(ask(&mut e, 0x10), 1, "second in its chain");
    assert_eq!(ask(&mut e, 0x13), 0x200, "the whole value");
    assert_eq!(ask(&mut e, 0x11), 5);
    assert_eq!(ask(&mut e, 0x16), 0, "same bucket, not there");
    assert_eq!(ask(&mut e, 0x12), 0, "empty bucket");
    e.mem.set_u8(0x011f_91c5, 9);
    assert_eq!(ask(&mut e, 0x12), 9, "the byte first");
    assert_eq!(ask(&mut e, 0x13), 9);
    e.mem.set_u8(0x011f_91c5, 0);
    e.mem.set_u32(WATER_MAP + 4, 0);
    assert_eq!(ask(&mut e, 0x13), 0, "no buckets");
}

// ---- GH2 part 4: G93 (fix row) and O17 (bNPCWeaponMods) --------------------------------------

const P4: u32 = 0x0520_0000;
const WEAPON: u32 = P4;
const FIRST_PERSON: u32 = P4 + 0x800;
const BIPED: u32 = P4 + 0xa00;
const ACTOR: u32 = P4 + 0xe00;
const ACTOR_BASE: u32 = P4 + 0x1100;
const PROCESS: u32 = P4 + 0x1300;
const INFO: u32 = P4 + 0x1400;
const INFO_LIST_NODE: u32 = P4 + 0x1440;
const INFO_EXTRA_LIST: u32 = P4 + 0x1480;
const INFO_FLAGS_EXTRA: u32 = P4 + 0x14c0;
const RENDER: u32 = P4 + 0x1500;
const PLAYER_REF: u32 = P4 + 0x1600;
const STUB_CLEAR_SLOT: u32 = 0x004a_aff0;
const STUB_SET_WEAPON: u32 = 0x004a_b750;
const STUB_LEVEL: u32 = 0x0047_ded0;
const STUB_RNG: u32 = 0x00aa_5230;
const STUB_UPDATE: u32 = 0x0088_db20;

thread_local! {
    static DRAWS: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The commands' engine with the extra stand-ins, a weapon (type 0x28, weapon type 2) whose
/// modded models' paths are set for the flags in `paths` (the model's length word `+8`), an NPC
/// (id 0x1234, base level 5) at the high process level whose weapon entry carries the mod flags
/// `held` (0: no flags extra), a biped of that NPC, the game's draws stubbed with `draws` (taken
/// in order) and the calls recorded.
fn weapon_world(paths: &[u32], held: u32, draws: &[u32]) -> Engine {
    let mut e = crate::commands::test_support::engine();
    crate::commands::test_support::extra_stand_ins(&mut e);
    e.map(P4, 0x2000);
    e.mem.set_u8(WEAPON + 4, 0x28);
    e.mem.set_u8(WEAPON + 0xf4, 2);
    for &flags in paths {
        e.mem.set_u16(WEAPON + MODEL_BY_MOD[flags as usize] + 8, 5);
    }
    e.mem.set_u32(ACTOR + 0xc, 0x1234);
    e.mem.set_u32(ACTOR + 0x20, ACTOR_BASE);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(PLAYER_REF + 0xc, 0x14);
    e.mem.set_u32(PROCESS + 0x114, INFO);
    e.mem.set_u32(INFO + 8, WEAPON);
    e.mem.set_u32(INFO, INFO_LIST_NODE);
    e.mem.set_u32(INFO_LIST_NODE, INFO_EXTRA_LIST);
    if held != 0 {
        crate::commands::test_support::put_extra(
            &mut e,
            INFO_EXTRA_LIST,
            INFO_FLAGS_EXTRA,
            0x8d,
            held,
        );
    }
    e.mem.set_u32(BIPED + 0x2b0, ACTOR);
    for stub in [STUB_CLEAR_SLOT, STUB_SET_WEAPON, STUB_UPDATE] {
        e.register(stub, |_, _| Ret::default());
    }
    e.register(STUB_LEVEL, |_, _| Ret {
        eax: 5,
        ..Ret::default()
    });
    DRAWS.with(|d| *d.borrow_mut() = draws.to_vec());
    e.register(STUB_RNG, |_, _| Ret {
        eax: DRAWS.with(|d| d.borrow_mut().remove(0)),
        ..Ret::default()
    });
    e.call_log = Some(vec![]);
    e
}

fn logged_calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
    e.call_log
        .as_ref()
        .unwrap()
        .iter()
        .filter(|c| c.0 == addr)
        .map(|c| c.1.clone())
        .collect()
}

/// G93 `GetWeaponModel`: the modded third-person model when its path is set (else the plain
/// one, flags taken as 0); the first-person object's model for the player, and for a type 8
/// weapon with a mod; a missing first-person object falls back to the third-person model; only the
/// low three bits of the flags count; an actor of 0 is not the player.
#[test]
fn g93_the_weapon_model_by_mod_flags() {
    let mut e = weapon_world(&[1, 5], 0, &[]);
    assert_eq!(weapon_model(&mut e, WEAPON, 0, ACTOR), WEAPON + 0x3c);
    assert_eq!(weapon_model(&mut e, WEAPON, 1, ACTOR), WEAPON + 0x270);
    assert_eq!(weapon_model(&mut e, WEAPON, 5, ACTOR), WEAPON + 0x2f0);
    assert_eq!(
        weapon_model(&mut e, WEAPON, 9, ACTOR),
        WEAPON + 0x270,
        "low bits"
    );
    assert_eq!(
        weapon_model(&mut e, WEAPON, 2, ACTOR),
        WEAPON + 0x3c,
        "no path: plain"
    );
    // The player: the first-person object of the (fallen back) flags.
    assert_eq!(
        weapon_model(&mut e, WEAPON, 2, PLAYER_REF),
        WEAPON + 0x3c,
        "none yet"
    );
    e.mem.set_u32(WEAPON + 0x250, FIRST_PERSON);
    assert_eq!(
        weapon_model(&mut e, WEAPON, 2, PLAYER_REF),
        FIRST_PERSON + 0x30
    );
    assert_eq!(
        weapon_model(&mut e, WEAPON, 0, 0),
        WEAPON + 0x3c,
        "no actor"
    );
    e.mem.set_u32(WEAPON + 0x254, FIRST_PERSON + 0x100);
    assert_eq!(
        weapon_model(&mut e, WEAPON, 1, PLAYER_REF),
        FIRST_PERSON + 0x130
    );
    assert_eq!(weapon_model(&mut e, WEAPON, 1, ACTOR), WEAPON + 0x270);
    // Type 8 with a mod: first person for anyone; without a mod: third person.
    e.mem.set_u8(WEAPON + 0xf4, 8);
    assert_eq!(weapon_model(&mut e, WEAPON, 1, ACTOR), FIRST_PERSON + 0x130);
    assert_eq!(weapon_model(&mut e, WEAPON, 0, ACTOR), WEAPON + 0x3c);
}

/// G93 `SetWeaponSlotHook`: a weapon not already loaded clears slot 5, becomes the slot's form
/// and gets [`weapon_model`]'s model for the biped's actor; a form that is not a weapon, none, or
/// the same weapon already loaded changes nothing; with the fixes bit clear the call goes on to
/// the exe's (nothing of JIP's done).
#[test]
fn g93_the_weapon_slot_takes_the_modded_model() {
    let mut e = weapon_world(&[1], 0, &[]);
    e.set_jip_patches(engine_core::extend::jip_patches::FIXES);
    set_weapon_slot(&mut e, &[BIPED, WEAPON, 0x101]);
    assert_eq!(
        logged_calls(&e, STUB_CLEAR_SLOT),
        vec![vec![BIPED, 5, 1, 0]]
    );
    assert_eq!(e.mem.u32(BIPED + 0x7c), WEAPON);
    assert_eq!(
        e.mem.u32(BIPED + 0x80),
        WEAPON + 0x270,
        "the low byte's flags"
    );
    // Loaded already (the clone set): nothing.
    e.mem.set_u32(BIPED + 0x84, 1);
    e.mem.set_u32(BIPED + 0x80, 0);
    set_weapon_slot(&mut e, &[BIPED, WEAPON, 1]);
    assert_eq!(e.mem.u32(BIPED + 0x80), 0);
    assert_eq!(logged_calls(&e, STUB_CLEAR_SLOT).len(), 1);
    // The same form with no clone: loaded again.
    e.mem.set_u32(BIPED + 0x84, 0);
    set_weapon_slot(&mut e, &[BIPED, WEAPON, 0]);
    assert_eq!(e.mem.u32(BIPED + 0x80), WEAPON + 0x3c);
    // Not a weapon, or none: nothing.
    e.mem.set_u8(WEAPON + 4, 0x18);
    set_weapon_slot(&mut e, &[BIPED, WEAPON, 0]);
    set_weapon_slot(&mut e, &[BIPED, 0, 0]);
    assert_eq!(logged_calls(&e, STUB_CLEAR_SLOT).len(), 2);
}

/// G93's flags: an NPC at the high or middle-high level gives its weapon entry's mod flags; a
/// low process, no process, no entry or no flags give 0; `InitWornObject`'s answer is those
/// (0 for no actor).
#[test]
fn g93_an_npcs_held_weapon_flags() {
    let mut e = weapon_world(&[], 6, &[]);
    assert_eq!(init_worn_weapon(&mut e, &[ACTOR, BIPED, WEAPON]), Some(6));
    e.mem.set_u8(PROCESS + 0x28, 1);
    assert_eq!(init_worn_weapon(&mut e, &[ACTOR, BIPED, WEAPON]), Some(6));
    e.mem.set_u8(PROCESS + 0x28, 2);
    assert_eq!(
        init_worn_weapon(&mut e, &[ACTOR, BIPED, WEAPON]),
        Some(0),
        "low level"
    );
    e.mem.set_u8(PROCESS + 0x28, 0);
    e.mem.set_u32(PROCESS + 0x114, 0);
    assert_eq!(
        init_worn_weapon(&mut e, &[ACTOR, BIPED, WEAPON]),
        Some(0),
        "no entry"
    );
    e.mem.set_u32(PROCESS + 0x114, INFO);
    e.mem.set_u32(ACTOR + 0x68, 0);
    assert_eq!(
        init_worn_weapon(&mut e, &[ACTOR, BIPED, WEAPON]),
        Some(0),
        "no process"
    );
    assert_eq!(
        init_worn_weapon(&mut e, &[0, BIPED, WEAPON]),
        Some(0),
        "no actor"
    );
    let mut e = weapon_world(&[], 0, &[]);
    assert_eq!(
        init_worn_weapon(&mut e, &[ACTOR, BIPED, WEAPON]),
        Some(0),
        "no flags"
    );
}

/// G93 `PickWeaponModelHook`: the mod slots given (the player's) stand; else an NPC's held
/// weapon's flags (not the player's); with a biped `SetWeapon` gets them and nothing is left to
/// attach; without, the model part for those flags (none for no item).
#[test]
fn g93_the_equipped_model_is_picked_with_the_flags() {
    let mut e = weapon_world(&[1, 2], 2, &[]);
    assert_eq!(
        pick_weapon_model(&mut e, &[ACTOR, ACTOR, WEAPON, BIPED, 0]),
        Some(0)
    );
    assert_eq!(
        logged_calls(&e, STUB_SET_WEAPON),
        vec![vec![BIPED, WEAPON, 2]]
    );
    assert_eq!(
        pick_weapon_model(&mut e, &[ACTOR, ACTOR, WEAPON, BIPED, 0x101]),
        Some(0)
    );
    assert_eq!(
        logged_calls(&e, STUB_SET_WEAPON)[1],
        vec![BIPED, WEAPON, 1],
        "slots stand"
    );
    assert_eq!(
        pick_weapon_model(&mut e, &[ACTOR, ACTOR, WEAPON, 0, 0]),
        Some(WEAPON + 0x290)
    );
    assert_eq!(
        pick_weapon_model(&mut e, &[ACTOR, 0, WEAPON, 0, 0]),
        Some(WEAPON + 0x3c),
        "not an actor: no flags"
    );
    assert_eq!(
        pick_weapon_model(&mut e, &[PLAYER_REF, PLAYER_REF, WEAPON, 0, 0]),
        Some(WEAPON + 0x3c),
        "the player: no held flags"
    );
    assert_eq!(
        pick_weapon_model(&mut e, &[ACTOR, ACTOR, 0, 0, 0]),
        Some(0),
        "no item"
    );
    assert_eq!(logged_calls(&e, STUB_SET_WEAPON).len(), 2);
}

/// O17's world: the switch on with JIP's published numbers (2, 10, 60), the NPC loaded (a render
/// state with a node), its weapon (two mod slots) entry with no flags yet.
fn mods_world(draws: &[u32]) -> Engine {
    let mut e = weapon_world(&[], 0, draws);
    e.set_jip_patches(engine_core::extend::jip_patches::NPC_WEAPON_MODS);
    e.ext::<crate::Jip>()
        .game_patches
        .settings
        .weapon_mod_chance = [2, 10, 60];
    e.mem.set_u32(ACTOR + 0x64, RENDER);
    e.mem.set_u32(RENDER + 0x14, 0x7777);
    e.mem.set_u32(WEAPON + 0x350, 0x11);
    e.mem.set_u32(WEAPON + 0x358, 0x13);
    e
}

fn dealt_flags(e: &mut Engine) -> Option<u32> {
    let x = crate::commands::extra::get_by_type(e, INFO_EXTRA_LIST, 0x8d);
    (x != 0).then(|| e.mem.u32(x + 0xc))
}

/// O17 `DoOnLoadActorHook`: the node answered; the NPC marked dealt; a draw under the chance
/// (level 5 x 2 = 10) gives mods (here both slots: shuffle 0 = 1, 2, 4: slot 1 halves to 5, slot
/// 2 absent, slot 4 kept with the draw 4 under 5), the flags extra added and the weapon updated;
/// a second load deals nothing.
#[test]
fn o17_an_npc_is_dealt_weapon_mods_once() {
    let mut e = mods_world(&[4, 0]);
    assert_eq!(do_on_load_actor(&mut e, &[ACTOR]), Some(0x7777));
    assert_eq!(e.mem.u8(ACTOR + 0x143) & 0x10, 0x10);
    assert_eq!(dealt_flags(&mut e), Some(5));
    assert_eq!(
        logged_calls(&e, STUB_UPDATE),
        vec![vec![ACTOR, WEAPON, 1, INFO_EXTRA_LIST, 0]]
    );
    assert_eq!(logged_calls(&e, STUB_LEVEL), vec![vec![ACTOR_BASE + 0x30]]);
    DRAWS.with(|d| *d.borrow_mut() = vec![]);
    assert_eq!(
        do_on_load_actor(&mut e, &[ACTOR]),
        Some(0x7777),
        "dealt: no draw"
    );
}

/// O17's draws: the chance held between min and max; a draw at or over it gives nothing (but the
/// NPC is marked); the shuffle's halving and drops.
#[test]
fn o17_the_chance_and_the_shuffle() {
    // Draw 10 = chance 10: nothing.
    let mut e = mods_world(&[10]);
    do_on_load_actor(&mut e, &[ACTOR]);
    assert_eq!(dealt_flags(&mut e), None);
    assert_eq!(e.mem.u8(ACTOR + 0x143) & 0x10, 0x10);
    // Level 40 x 2 = 80 held to 60: draw 59 under; shuffle 4 (4, 1, 2): slot 4 halves to 30;
    // slot 1 present, 30 <= 59: only the first kept (4).
    let mut e = mods_world(&[59, 4]);
    e.register(STUB_LEVEL, |_, _| Ret {
        eax: 40,
        ..Ret::default()
    });
    do_on_load_actor(&mut e, &[ACTOR]);
    assert_eq!(dealt_flags(&mut e), Some(4));
    // Level 1 x 2 = 2 held up to 10; draw 9; shuffle 1 (1, 4, 2): 1 halves to 5, 4 present and
    // 5 <= 9: only the first (1).
    let mut e = mods_world(&[9, 1]);
    e.register(STUB_LEVEL, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    do_on_load_actor(&mut e, &[ACTOR]);
    assert_eq!(dealt_flags(&mut e), Some(1));
    // A negative level word (sign-extended) is a huge chance: held to 60. Draw 0, shuffle 5
    // (4, 2, 1): 4 halves 60 to 30, 2 absent, 1 present and 30 > 0: kept: 5.
    let mut e = mods_world(&[0, 5]);
    e.register(STUB_LEVEL, |_, _| Ret {
        eax: 0xffff,
        ..Ret::default()
    });
    do_on_load_actor(&mut e, &[ACTOR]);
    assert_eq!(dealt_flags(&mut e), Some(5));
    // Only the level word counts (`cwde`): 0xABCD_0005 is level 5, chance 10; draw 30: nothing
    // (the whole word would give 60, over 30).
    let mut e = mods_world(&[30]);
    e.register(STUB_LEVEL, |_, _| Ret {
        eax: 0xabcd_0005,
        ..Ret::default()
    });
    do_on_load_actor(&mut e, &[ACTOR]);
    assert_eq!(dealt_flags(&mut e), None);
    // Chance 10, draw 7, shuffle 0 (1, 2, 4): 1 halves to 5; 2 absent; 4 present with 5 <= 7:
    // dropped: 1.
    let mut e = mods_world(&[7, 0]);
    do_on_load_actor(&mut e, &[ACTOR]);
    assert_eq!(dealt_flags(&mut e), Some(1));
}

/// O17's conditions: no node (answer 0), a save loading, dead, a low process, already dealt,
/// a teammate (marked, no draw), a weapon type over 9 or flags 0xA0, flags there already, no mod
/// slots; and the switch off or a number 0: no answer.
#[test]
fn o17_who_is_dealt_mods() {
    let mut e = mods_world(&[]);
    e.mem.set_u32(RENDER + 0x14, 0);
    assert_eq!(do_on_load_actor(&mut e, &[ACTOR]), Some(0));
    assert_eq!(e.mem.u8(ACTOR + 0x143), 0);
    let mut e = mods_world(&[]);
    let save_load = e.mem.u32(0x011d_df38);
    e.mem.set_u8(save_load + 0x244, 2);
    assert_eq!(do_on_load_actor(&mut e, &[ACTOR]), Some(0x7777));
    assert_eq!(e.mem.u8(ACTOR + 0x143), 0, "a save loading");
    let mut e = mods_world(&[]);
    e.mem.set_u32(ACTOR + 0x108, 2);
    do_on_load_actor(&mut e, &[ACTOR]);
    assert_eq!(e.mem.u8(ACTOR + 0x143), 0, "dead");
    let mut e = mods_world(&[]);
    e.mem.set_u8(PROCESS + 0x28, 2);
    do_on_load_actor(&mut e, &[ACTOR]);
    assert_eq!(e.mem.u8(ACTOR + 0x143), 0, "low process");
    let mut e = mods_world(&[]);
    e.mem.set_u8(ACTOR + 0x18d, 1);
    do_on_load_actor(&mut e, &[ACTOR]);
    assert_eq!(e.mem.u8(ACTOR + 0x143), 0x10, "a teammate is marked");
    assert_eq!(dealt_flags(&mut e), None);
    for (at, value) in [(0xf4u32, 10u8), (0x100, 0x20), (0x100, 0x80)] {
        let mut e = mods_world(&[]);
        e.mem.set_u8(WEAPON + at, value);
        do_on_load_actor(&mut e, &[ACTOR]);
        assert_eq!(dealt_flags(&mut e), None, "{at:#x} {value:#x}");
        assert!(logged_calls(&e, STUB_LEVEL).is_empty());
    }
    let mut e = mods_world(&[]);
    e.mem.set_u32(WEAPON + 0x350, 0);
    e.mem.set_u32(WEAPON + 0x358, 0);
    do_on_load_actor(&mut e, &[ACTOR]);
    assert!(logged_calls(&e, STUB_LEVEL).is_empty(), "no mod slots");
    let mut e = mods_world(&[]);
    crate::commands::test_support::put_extra(&mut e, INFO_EXTRA_LIST, INFO_FLAGS_EXTRA, 0x8d, 0);
    do_on_load_actor(&mut e, &[ACTOR]);
    assert!(
        logged_calls(&e, STUB_LEVEL).is_empty(),
        "flags there already"
    );
    let mut e = mods_world(&[]);
    e.mem.set_u32(INFO_LIST_NODE, 0);
    do_on_load_actor(&mut e, &[ACTOR]);
    assert!(logged_calls(&e, STUB_LEVEL).is_empty(), "no first list");
    let mut e = mods_world(&[]);
    e.set_jip_patches(engine_core::extend::jip_patches::FIXES);
    assert_eq!(do_on_load_actor(&mut e, &[ACTOR]), None, "switch off");
    let mut e = mods_world(&[]);
    e.ext::<crate::Jip>()
        .game_patches
        .settings
        .weapon_mod_chance = [2, 0, 60];
    assert_eq!(do_on_load_actor(&mut e, &[ACTOR]), None, "a number 0");
}

/// O17 `ResetActorFlagsRespawnHook`: the dealt mark cleared (other bits kept), the actor's extra
/// list answered; the switch off: no answer.
#[test]
fn o17_a_respawn_clears_the_mark() {
    let mut e = mods_world(&[]);
    e.mem.set_u8(ACTOR + 0x143, 0x13);
    assert_eq!(
        reset_actor_flags_respawn(&mut e, &[ACTOR]),
        Some(ACTOR + 0x44)
    );
    assert_eq!(e.mem.u8(ACTOR + 0x143), 3);
    e.set_jip_patches(0);
    assert_eq!(reset_actor_flags_respawn(&mut e, &[ACTOR]), None);
}
