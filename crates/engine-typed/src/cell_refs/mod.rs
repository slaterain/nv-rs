//! The cell references family (`NV_TYPED=cell_refs`): the pass that brings a cell's references up
//! once its 3D is loaded (`0054bcf0`), which the game runs on main for every cell a grid shift
//! loads (`TES::GridArrayLoad`'s second loop, `004533ae`; `0045410c`; `TES::CullGridCells`,
//! `00450edb`). The hitch log's "references" part of a grid load is this function; in a play
//! session's border hitch it was 3.8 ms of the 4.6 ms load.
//!
//! For every reference of the cell, in two passes over the cell's list (the markers in the first),
//! the exe makes about ten calls by address of its own besides the work it hands on (the model
//! loader's queue, the 3D's attach): the game loader's flag (`0042ce10`), the reference's extra
//! data list (`005d43c0`, twice), `Script::InitActionList` (`005ac190`), the save object's test
//! (`0047c850`), the form type (`00401170`) and the base form's flag test (`00452440`), and the
//! virtual calls the reference's class answers with a constant body (its slots 0x100, 0x224 and the
//! 3D at 0x1d0).
//!
//! **Native.** The order of work and every call that does work stay the exe's. The field reads are
//! read in place (the game loader's word at +0x244, the extra list at +0x44, the type byte at +4,
//! the base form's word at +0x48, the cell's list at +0xac). The save object's test (the exe's
//! folded false body `0047c850`, the draw family's) is its typed call. A virtual call whose slot
//! holds a typed function is that typed call (the target decides, so a class whose slot holds
//! anything else is called through its slot as the exe calls it): `TESObjectREFR`'s vtable
//! (`0102f55c`) holds `0047c850` at 0x100, `005725b0` at 0x224 and `0043fcd0` at 0x1d0.
//! `Script::InitActionList` and the slot 0x224 body are typed here and registered at their
//! addresses. `SetActionFlag` (`005ac750`) and the cell's state (`004512a0`) can be replaced by a
//! fix or a built-in extender: called by address.
//!
//! The model loader's queue of each reference (`ModelLoader::QueueReference`, `00444850`, most of
//! the pass's time) is typed in [`queue`] and registered at its address, so the pass, which calls
//! it by address as the game's other callers do, reaches it through the registration. The small
//! functions it calls on its task pointer and the cell are typed there too.
//!
//! The finish of each task on main, once its files are in (`QueuedReference::PostProcess`,
//! `0043fdf0`, and the attaches it reaches: `00440ba0`, `00441800`, `00441c30`), is typed in
//! [`attach`] with `TES::IsCellLoaded`, the loader's word that a reference is done and the cell's
//! two counters going down. `TES`'s load of each reference into the scene (`00451ef0`), which the
//! attach and the cell's other loads call, is typed in [`scene`]; with it the actor classes' true
//! body (`008d0360`) their slot 0x100 holds, so an actor's test is a typed call too.
//!
//! The reference's form id given to the geometry under its 3D (`SetReferenceIDOnScenegraph`,
//! `004b6dc0`, a walk of the scene graph the game runs for every actor it loads) is typed in
//! [`scenegraph`] with `NiAVObject::GetProperty` (`00a59d30`), which it asks of each geometry; both
//! are registered at their addresses, so every caller by address reaches them.
//!
//! The shadow scene node's queue of the objects whose lighting is to be redone, which `TES`'s load
//! fills for every node it puts in the scene (`ShadowSceneNode::AddObject`, `00b5eeb0`, and
//! `UpdateObjectLighting`, `00b5d9f0`), is typed in [`lighting`] with the add at the tail of a list
//! of counted pointers (`00a5a310`) it makes; all three are registered at their addresses.
//!
//! The reference's script made ready as the load brings it into the scene
//! (`TESObjectREFR::InitScript`, `00565730`) is typed in [`script`] with what it asks of the
//! reference: the form its script comes from (the actor classes' slot 0x1a4, `008ae600`), the
//! form's script (`004826d0`) and the base form's container (`0055d310`); all four are registered
//! at their addresses.
//!
//! The list of extra data every reference, cell and inventory entry carries (`BaseExtraList` and
//! `ExtraDataList`, Xbox PDB) is typed in [`extra_list`]: its constructors and destructors, its
//! operations under the extra data lock (`AddExtra`, both `RemoveExtra`s, `RemoveAll`,
//! `RemoveAllDefault`, `ItemsInList`, `GetPrevExtraData`), the type bitmap, the thread's cache of
//! the last lookup and the small extra data classes of the unit, each registered at its address.
//!
//! Stage 1: the objects stay in guest memory where the game made them. The row is
//! [`Threads::Both`]: the code is integers (the origin's 0.0 for the vector constructor is a constant
//! word) but for one product in [`scene`], which keeps the x87's exact helpers.

pub mod attach;
pub mod extra_list;
pub mod lighting;
pub mod queue;
pub mod scene;
pub mod scenegraph;
pub mod script;

use crate::{dispatch, draw, slot_target};
use engine_core::ptr::Ptr;
use engine_core::typed::{leaf, Family, FamilyId, Threads};
use engine_core::{args, entry, typed_call, AbiFn, Engine};

/// This family's bit: what each `typed_call!` of it names. Id 35: the next free after the families
/// in main (0-30, 32, 34) and the navmesh lane's 33.
pub const FAMILY: FamilyId = FamilyId::new(35);

/// `NV_TYPED=cell_refs`. A native conversion proved by its differential tests: on unless `NV_TYPED`
/// says otherwise (`NV_TYPED=off` or `cell_refs:off`).
pub static CELL_REFS: Family = Family {
    name: "cell_refs",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0054_bcf0, references_after_load(u32, u8)),
        entry!(0x005a_c190, init_action_list(u32, u32)),
        entry!(0x0044_4850, queue::queue_reference(u32, u32, u32, u8)),
        entry!(0x006f_74f0, queue::task_pointer_assign(u32, u32) -> u32),
        entry!(0x0092_c870, queue::add_ref(u32)),
        entry!(0x0044_cbf0, queue::task_pointer_release(u32)),
        entry!(0x0044_4cc0, queue::count_queued(u32)),
        entry!(0x0044_4ce0, queue::count_critical(u32)),
        entry!(0x0057_2350, queue::is_3d_critical(u32) -> bool),
        entry!(0x0057_25b0, kind_3d_to_40_or_69(u32) -> bool),
        entry!(0x0043_fdf0, attach::post_process(u32)),
        entry!(0x0044_1c30, attach::player_attach(u32)),
        entry!(0x0044_1800, attach::character_attach(u32)),
        entry!(0x0044_0ba0, attach::attach(u32)),
        entry!(0x0045_11e0, attach::is_cell_loaded(u32, u32, u8) -> bool),
        entry!(0x0044_09e0, attach::reference_done(u32, u32)),
        entry!(0x0044_0a10, attach::queued_count_down(u32)),
        entry!(0x0044_0a50, attach::critical_count_down(u32)),
        entry!(0x0045_1ef0, scene::load_reference(u32, u32, u32, u32, u8)),
        entry!(0x008d_0360, answers_true(u32) -> bool),
        entry!(0x004b_6dc0, scenegraph::set_reference_id_on_scenegraph(u32, u32)),
        entry!(0x00a5_9d30, scenegraph::get_property(u32, i32) -> u32),
        entry!(0x00b5_eeb0, lighting::add_object(u32, u32)),
        entry!(0x00b5_d9f0, lighting::update_object_lighting(u32, u32, u8)),
        entry!(0x00a5_a310, lighting::list_add_tail(u32, u32)),
        entry!(0x0056_5730, script::init_script(u32)),
        entry!(0x0048_26d0, script::form_script(u32) -> u32),
        entry!(0x0055_d310, script::has_container(u32) -> u32),
        entry!(0x008a_e600, script::actor_form_148(u32) -> u32),
        entry!(0x0040_ec80, extra_list::extra_data_new(u32, u8) -> u32),
        entry!(0x0040_ecb0, extra_list::extra_data_drop(u32)),
        entry!(0x0040_3550, extra_list::set_next(u32, u32)),
        entry!(0x0040_f680, extra_list::north_rotation_new(u32) -> u32),
        entry!(0x0040_f6b0, extra_list::detach_time_new(u32) -> u32),
        entry!(0x0040_f700, extra_list::extra_compare(u32, u32) -> bool),
        entry!(0x0040_f740, extra_list::base_list_new(u32) -> u32),
        entry!(0x0040_f780, extra_list::base_list_delete(u32, u32) -> u32),
        entry!(0x0040_f7b0, extra_list::base_list_drop(u32)),
        entry!(0x0040_f7d0, extra_list::clear_last_extra(u32, i32)),
        entry!(0x0040_f860, extra_list::clear_last_extra_all(u32)),
        entry!(0x0040_f900, extra_list::drop_stale_cache()),
        entry!(0x0040_f980, extra_list::added_at_head(u32) -> bool),
        entry!(0x0040_f9e0, extra_list::cached_extra(u32, i32) -> u32),
        entry!(0x0040_fa40, extra_list::cache_extra(u32, u32)),
        entry!(0x0040_fae0, extra_list::remove_all(u32, bool)),
        entry!(0x0040_fcb0, extra_list::remove_all_default(u32, bool)),
        entry!(0x0040_fe20, extra_list::items_in_list(u32) -> i32),
        entry!(0x0040_fe80, extra_list::has_extra_type(u32, u8) -> bool),
        entry!(0x0040_fee0, extra_list::set_extra_flag(u32, u8, bool)),
        entry!(0x0040_ff60, extra_list::add_extra(u32, u32) -> u32),
        entry!(0x0041_0020, extra_list::remove_extra(u32, u32, bool)),
        entry!(0x0041_0140, extra_list::remove_extra_of_type(u32, u8)),
        entry!(0x0041_02d0, extra_list::get_prev_extra_data(u32, u8) -> u32),
        entry!(0x0041_0360, extra_list::data_list_new(u32) -> u32),
        entry!(0x0041_0380, extra_list::data_list_delete(u32, u32) -> u32),
        entry!(0x0041_03b0, extra_list::data_list_drop(u32)),
        entry!(0x0041_1b00, extra_list::lock_data_new(u32) -> u32),
        entry!(0x0041_1b40, extra_list::reflected_refs_new(u32) -> u32),
        entry!(0x0041_1bb0, extra_list::reflected_refs_delete(u32, u32) -> u32),
        entry!(0x0041_1be0, extra_list::reflector_refs_new(u32) -> u32),
        entry!(0x0041_1c50, extra_list::reflector_refs_delete(u32, u32) -> u32),
        entry!(0x0041_1c80, extra_list::water_light_refs_new(u32) -> u32),
        entry!(0x0041_1cf0, extra_list::water_light_refs_delete(u32, u32) -> u32),
        entry!(0x0041_1d20, extra_list::lit_water_refs_new(u32) -> u32),
        entry!(0x0041_1d90, extra_list::lit_water_refs_delete(u32, u32) -> u32),
        entry!(0x0041_1dc0, extra_list::four_words_new(u32) -> u32),
        entry!(0x0041_1e00, extra_list::word_16_new(u32) -> u32),
        entry!(0x0041_1e20, extra_list::weapon_mod_flags_any(u32, u8) -> bool),
        entry!(0x0041_1e40, extra_list::type_92_new(u32) -> u32),
        entry!(0x0041_1e70, extra_list::type_92_delete(u32, u32) -> u32),
        entry!(0x0041_1ea0, extra_list::type_92_drop(u32)),
    ]
}

/// `TESObjectCELL::Load3D` (Xbox PDB).
const CELL_LOAD_3D: u32 = 0x0054_5cf0;
/// The cell's scripts' run (`(cell, 0, 0)`), the check of a cell loaded again after a detach
/// (`0054df30`: whether the detach lasted long enough) and the step its answer goes to.
const CELL_RUN_SCRIPTS: u32 = 0x0054_c740;
const CELL_REATTACH_CHECK: u32 = 0x0054_df30;
const CELL_AFTER_SCRIPTS: u32 = 0x0055_0c60;
/// The `TES` object (a global) and its test of the cell (`(tes, cell)`).
const TES_POINTER: u32 = 0x011d_ea10;
const TES_CELL_TEST: u32 = 0x0045_3490;
/// The three-float vector constructor and the call that sets the cell 3D's translation from it.
const VECTOR3_CONSTRUCT: u32 = 0x0043_d410;
const NODE_SET_LOCAL_TRANSLATE: u32 = 0x00a5_9c60;
/// The pass the accepted cell runs instead (`0054bc10`).
const CELL_ACCEPTED_PASS: u32 = 0x0054_bc10;
/// The cell's reference lock.
const CELL_LOCK_ENTER: u32 = 0x0054_1ac0;
const CELL_LOCK_LEAVE: u32 = 0x0054_1ae0;
/// `TES`'s priority for the cell's loads (`(tes, cell, 0)`).
const TES_GET_CELL_PRIORITY: u32 = 0x0045_8be0;
/// The cell's list of references (`009604f0` answers `cell + 0xac`).
const CELL_REFERENCES: u32 = 0xac;
/// The two marker base forms (globals) whose references go in the second pass.
const ROOM_MARKER_FORM: u32 = 0x011c_a238;
const MARKER_FORM: u32 = 0x011c_a230;
/// The game loader (a global): `0042ce10` tests bit 1 of its word at +0x244.
const GAME_LOADER_POINTER: u32 = 0x011d_df38;
const GAME_LOADER_FLAGS: u32 = 0x244;
/// A reference's `ExtraDataList` (`005d43c0` answers `reference + 0x44`).
const REFERENCE_EXTRA_LIST: u32 = 0x44;
/// `Script::SetActionFlag` (Xbox PDB), `(reference, extra data list, flag)`: overridable.
const SCRIPT_SET_ACTION_FLAG: u32 = 0x005a_c750;
/// The reference classes' slots: an actor's test (0x100), the actor's step (0x240), the type test
/// (0x224) and its step (0x260), the 3D (0x1d0).
const SLOT_IS_ACTOR: u32 = 0x100;
const SLOT_ACTOR_STEP: u32 = 0x240;
const SLOT_KIND_TEST: u32 = 0x224;
const SLOT_KIND_STEP: u32 = 0x260;
const SLOT_GET_3D: u32 = 0x1d0;
/// `__RTDynamicCast` and the type descriptors of the cast (reference to the actor class).
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_REFERENCE: u32 = 0x0118_41cc;
const RTTI_ACTOR_LIKE: u32 = 0x0118_46d4;
/// `ProcessLists::RemoveActorFromTempChangeList` (Xbox PDB) on the process lists (a global's
/// address, passed as the word).
const PROCESS_LISTS_REMOVE_ACTOR: u32 = 0x0096_f400;
const PROCESS_LISTS: u32 = 0x011e_0e80;
/// `ModelLoader::QueueReference` (Xbox PDB), `(loader, reference, priority, 0)`, on the model
/// loader (a global).
const MODEL_LOADER_QUEUE_REFERENCE: u32 = 0x0044_4850;
const MODEL_LOADER_POINTER: u32 = 0x011c_3b3c;
/// The save object (a global), whose test `0054bcf0` asks of an actor.
const SAVE_GAME_POINTER: u32 = 0x011d_e45c;
/// `TESObjectCELL::AttachReference3D` (`(cell, reference, 0)`).
const CELL_ATTACH_REFERENCE_3D: u32 = 0x0054_8880;
/// The base form type whose flag `0x40000000` (the word at +0x48, `00452440` through `00452460` and
/// `00624700`) keeps its reference's 3D from being attached here.
const FORM_TYPE_23: u8 = 0x23;
const FORM_FLAGS_48: u32 = 0x48;
const FORM_FLAG_NO_ATTACH: u32 = 0x4000_0000;
/// The step for a cell loaded with the flag set, and the cell's state (`cCellState = 6`).
const CELL_AFTER_ATTACH: u32 = 0x0055_35f0;
const CELL_SET_STATE: u32 = 0x0045_12a0;
/// `Script::InitActionList`'s callees: the extra list's `ScriptLocals` (`00418830`), the script run
/// manager's instance and its `InitActionList` (`(manager, master script, reference, locals)`).
const EXTRA_LIST_SCRIPT_LOCALS: u32 = 0x0041_8830;
const SCRIPT_RUN_MANAGER_INSTANCE: u32 = 0x005e_24d0;
const SCRIPT_RUN_MANAGER_INIT_ACTION_LIST: u32 = 0x005e_2730;
/// `ScriptLocals`: the master script (+0) and the action list (+8).
const LOCALS_MASTER: u32 = 0;
const LOCALS_ACTION_LIST: u32 = 8;

/// A reference's base form (`007af430`, the word at +0x20).
#[inline]
fn base_form(e: &mut Engine, reference: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x007a_f430, leaf::word_at_20(e, reference))
}

/// The reference a list node holds (`006815c0` answers the node; its first word is the item).
#[inline]
fn node_item(e: &mut Engine, node: u32) -> u32 {
    let at = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, node));
    e.mem.u32(at)
}

/// The node after `node` (`00726070`, the word at +4).
#[inline]
fn node_next(e: &mut Engine, node: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, node))
}

/// The reference's slot 0x100 (whether it is an actor): `TESObjectREFR`'s false body and the actor
/// classes' true body ([`answers_true`]) as their typed calls, any other through the vtable.
#[inline]
fn is_actor(e: &mut Engine, reference: u32) -> bool {
    match slot_target(e, reference, SLOT_IS_ACTOR) {
        0x0047_c850 => typed_call!(
            draw::FAMILY,
            0x0047_c850,
            draw::shapes::answers_false(e, reference)
        ),
        0x008d_0360 => typed_call!(FAMILY, 0x008d_0360, answers_true(e, reference)),
        _ => e.vcall(reference, SLOT_IS_ACTOR, &args![]).bool(),
    }
}

/// The reference's slot 0x224: [`kind_3d_to_40_or_69`] as its typed call when the slot holds it,
/// any other through the vtable.
#[inline]
fn kind_test(e: &mut Engine, reference: u32) -> bool {
    if slot_target(e, reference, SLOT_KIND_TEST) == 0x0057_25b0 {
        typed_call!(FAMILY, 0x0057_25b0, kind_3d_to_40_or_69(e, reference))
    } else {
        e.vcall(reference, SLOT_KIND_TEST, &args![]).bool()
    }
}

/// The reference's 3D (slot 0x1d0): `TESObjectREFR::Get3D` as its typed call when the slot holds
/// it, any other through the vtable.
#[inline]
fn slot_3d(e: &mut Engine, reference: u32) -> u32 {
    if slot_target(e, reference, SLOT_GET_3D) == 0x0043_fcd0 {
        typed_call!(
            dispatch::FAMILY,
            0x0043_fcd0,
            dispatch::scene::reference_3d(e, Ptr::new(reference))
        )
        .addr()
    } else {
        e.vcall(reference, SLOT_GET_3D, &args![]).u32()
    }
}

// Typed from 0054bcf0
/// The cell's references brought up after its 3D is loaded (`thiscall(cell, flag)`, `ret 4`):
/// `TESObjectCELL::Load3D`, the cell's scripts (`0054c740(cell, 0, 0)`), `0054df30`'s answer handed
/// to `00550c60`. When `TES` accepts the cell (`00453490`) its 3D (if any) is moved to the origin
/// and `0054bc10` runs. Otherwise, under the cell's lock and at `TES`'s priority for the cell, two
/// walks over the cell's list, the markers (base form `011ca238` or `011ca230`) brought up in the
/// first and the other references in the second: unless the game loader's flag is set, the
/// reference's action list is set up (`Script::InitActionList`, `SetActionFlag(0x1000)`); a
/// reference with form flag `0x800` or `0x20` stops there; an actor is cast to its class and, when
/// the cast holds, has its slot 0x240 run and leaves the process lists' temp-change list, any other
/// reference whose slot 0x224 says so has its slot 0x260 run; the model loader queues the
/// reference; one with a 3D is attached (`TESObjectCELL::AttachReference3D`) unless its base form
/// has type `0x23` and its flag `0x40000000`. Last `005535f0` when `flag` is set, and the cell's
/// state becomes 6.
///
/// The exe counts the pass when the walk reaches the end of the list, before it looks at the
/// reference it just read: the list's last reference is judged by the next pass's rule in each
/// walk, so when it is a marker it comes up once, at the end of the second walk, and when it is not
/// it comes up in both walks. Kept as the exe does it.
pub fn references_after_load(e: &mut Engine, this: u32, flag: u8) {
    let node_3d = e.call(CELL_LOAD_3D, &args![this]).u32();
    e.call(CELL_RUN_SCRIPTS, &args![this, 0u32, 0u32]);
    let scripts = e.call(CELL_REATTACH_CHECK, &args![this]).u8();
    e.call(CELL_AFTER_SCRIPTS, &args![this, u32::from(scripts)]);
    let tes = e.global::<u32>(TES_POINTER);
    if e.call(TES_CELL_TEST, &args![tes, this]).bool() {
        if node_3d != 0 {
            e.with_stack(12, |e, vector| {
                e.call(VECTOR3_CONSTRUCT, &args![vector, 0.0f32, 0u32, 0u32]);
                e.call(NODE_SET_LOCAL_TRANSLATE, &args![node_3d, vector]);
            });
        }
        e.call(CELL_ACCEPTED_PASS, &args![this]);
    } else {
        e.call(CELL_LOCK_ENTER, &args![this]);
        let tes = e.global::<u32>(TES_POINTER);
        let priority = e.call(TES_GET_CELL_PRIORITY, &args![tes, this, 0u32]).u32();
        let list = this.wrapping_add(CELL_REFERENCES);
        let mut cursor = list;
        let mut pass = 0u32;
        while pass < 2 && cursor != 0 {
            let item = node_item(e, cursor);
            cursor = node_next(e, cursor);
            if cursor == 0 {
                pass += 1;
                if pass < 2 {
                    cursor = list;
                }
            }
            if item == 0 {
                continue;
            }
            let mut marker = false;
            if base_form(e, item) != 0 {
                marker = base_form(e, item) == e.global::<u32>(ROOM_MARKER_FORM)
                    || base_form(e, item) == e.global::<u32>(MARKER_FORM);
            }
            if (pass == 0 && !marker) || (pass == 1 && marker) {
                continue;
            }
            bring_up(e, this, item, priority);
        }
        e.call(CELL_LOCK_LEAVE, &args![this]);
    }
    if flag != 0 {
        e.call(CELL_AFTER_ATTACH, &args![this]);
    }
    e.call(CELL_SET_STATE, &args![this, 6u32]);
}

/// One reference of [`references_after_load`]'s passes, from its action list on.
fn bring_up(e: &mut Engine, cell: u32, item: u32, priority: u32) {
    let loader = e.global::<u32>(GAME_LOADER_POINTER);
    if e.mem.u32(loader.wrapping_add(GAME_LOADER_FLAGS)) & 2 == 0 {
        let list = item.wrapping_add(REFERENCE_EXTRA_LIST);
        typed_call!(FAMILY, 0x005a_c190, init_action_list(e, item, list));
        e.call(SCRIPT_SET_ACTION_FLAG, &args![item, list, 0x1000u32]);
    }
    if typed_call!(leaf::FAMILY, 0x0044_0da0, leaf::form_flag_800(e, item))
        || typed_call!(leaf::FAMILY, 0x0044_0d80, leaf::form_flag_20(e, item))
    {
        return;
    }
    // The save object's test, `!test || test` as the exe asks it (`0047c850` answers false).
    let as_actor = is_actor(e, item) && {
        let save = e.global::<u32>(SAVE_GAME_POINTER);
        !typed_call!(
            draw::FAMILY,
            0x0047_c850,
            draw::shapes::answers_false(e, save)
        ) || typed_call!(
            draw::FAMILY,
            0x0047_c850,
            draw::shapes::answers_false(e, save)
        )
    };
    if as_actor {
        let actor = e
            .call(
                RT_DYNAMIC_CAST,
                &args![item, 0u32, RTTI_REFERENCE, RTTI_ACTOR_LIKE, 0u32],
            )
            .u32();
        if actor != 0 {
            e.vcall(actor, SLOT_ACTOR_STEP, &args![]);
            e.call(PROCESS_LISTS_REMOVE_ACTOR, &args![PROCESS_LISTS, actor]);
        }
    } else if kind_test(e, item) {
        e.vcall(item, SLOT_KIND_STEP, &args![]);
    }
    let model_loader = e.global::<u32>(MODEL_LOADER_POINTER);
    e.call(
        MODEL_LOADER_QUEUE_REFERENCE,
        &args![model_loader, item, priority, 0u32],
    );
    if slot_3d(e, item) != 0 {
        let mut attach = true;
        let base = base_form(e, item);
        if e.mem.u8(base.wrapping_add(4)) == FORM_TYPE_23 {
            let base = base_form(e, item);
            if e.mem.u32(base.wrapping_add(FORM_FLAGS_48)) & FORM_FLAG_NO_ATTACH != 0 {
                attach = false;
            }
        }
        if attach {
            e.call(CELL_ATTACH_REFERENCE_3D, &args![cell, item, 0u32]);
        }
    }
}

// Typed from 005ac190
/// `Script::InitActionList` (Xbox PDB), `cdecl(reference, extra data list)`: for a list whose
/// `ScriptLocals` (`00418830`, asked twice as the exe asks) have no action list yet, the script run
/// manager builds it (`InitActionList(manager, master script, reference, locals)`).
pub fn init_action_list(e: &mut Engine, reference: u32, list: u32) {
    if list == 0 {
        return;
    }
    if e.call(EXTRA_LIST_SCRIPT_LOCALS, &args![list]).u32() == 0 {
        return;
    }
    let locals = e.call(EXTRA_LIST_SCRIPT_LOCALS, &args![list]).u32();
    if e.mem.u32(locals.wrapping_add(LOCALS_ACTION_LIST)) != 0 {
        return;
    }
    let master = e.mem.u32(locals.wrapping_add(LOCALS_MASTER));
    let manager = e.call(SCRIPT_RUN_MANAGER_INSTANCE, &args![]).u32();
    e.call(
        SCRIPT_RUN_MANAGER_INIT_ACTION_LIST,
        &args![manager, master, reference, locals],
    );
}

// Typed from 005725b0
/// A reference class's slot 0x224 (`TESObjectREFR`'s, `thiscall(this)`): whether the reference's
/// own form type (the byte at +4, `00401170`) is in `0x3d..=0x40` or is `0x69`.
pub fn kind_3d_to_40_or_69(e: &mut Engine, this: u32) -> bool {
    let kind = e.mem.u8(this.wrapping_add(4));
    (0x3d..=0x40).contains(&kind) || kind == 0x69
}

// Typed from 008d0360
/// A class's constant true body (`thiscall(this)`, `mov al, 1`): the slot 0x100 of the actor
/// classes (`Actor`, `Character`, `Creature`, `PlayerCharacter`), among others.
pub fn answers_true(_e: &mut Engine, _this: u32) -> bool {
    true
}
