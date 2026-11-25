//! `TES`'s load of one reference into the scene (`00451ef0`, `thiscall(tes, reference, cell,
//! source, flag)`, `ret 0x10`): what the model loader's finish on main ([`super::attach`]) does
//! with each reference whose files are in. The game's other callers (`0043fc40`, `00440310`,
//! `00475b20`, `00477d10`, `0055c3e0`) call it by address too, so they reach it through its
//! registration. In a profiled load window it was most of the attach's 1.62 ms a frame with what it
//! calls.
//!
//! **Native.** The order of work and every call that does work stay the exe's. Done in place: the
//! form type (`00401170`, the byte at +4), the reference's form flags it tests (`00564e60`'s
//! `0x8000` on the reference or its base form, `005651e0`'s `0x80000`, `00452370`'s `0x1000000`),
//! the base form's flag `0x40000000` (`00452440`, the word at +0x48), the source's node
//! (`005d8710`, the counted pointer at +0x34), the node's parent (`009611e0`, +0x18), the shadow
//! scene node (`00450b80`, the table at `011f91c8`), the cell's state (`00450fd0`, the byte at
//! +0x26) and its count of references seen from a distance (`00452390`, the 16-bit word at +0xaa),
//! the loaded data's flags and count (`004523e0`, `0057b200`), the extra data's address
//! (`00430830`, +0x24), the game loader's flag (`0042ce10`), the extra data list (`005d43c0`,
//! +0x44), the byte `00452480` answers and the cast's two sizes (`0045bb80`, +0x8c). Two empty
//! bodies are not called: `00483710` (`ret`, an actor's step and the slot 0x40 of four of the six
//! reference task vtables) and `004534f0` (`ret 4`, type 0x1e's step). A virtual call whose slot
//! holds a typed function is that typed call, any other through the slot. `Script::InitActionList`
//! is this family's; `SetActionFlag` (`005ac750`) can be replaced by a fix or a built-in extender:
//! by address.
//!
//! **JIP LN NVSE.** With JIP on (the default preset) its attach of four task types calls this by
//! address, so it runs here with JIP on and off. With JIP's `bNPCWeaponMods` or `uNPCPerks` patch
//! on, its handler at `0x45234E` answers the node an NPC's reference ID is set on, as in the
//! translation.
//!
//! The reference ID set on the node (`SetReferenceIDOnScenegraph`, `004b6dc0`) is
//! [`super::scenegraph`]'s typed call.
//!
//! **Floats.** One product, on the path of a type 0x25 base form in a large cell: the radius its
//! slot 0x1bc answers (stored as a `double`) times the reference's scale, made an integer toward
//! zero (`FISTP qword` under the word with rounding control 3). The family runs on both threads'
//! words, so the product is the x87's (`x87_mul`: 24 bits under the main thread's word) and the
//! store the exe's 64-bit one, whose low word is used.

use super::{answers_true, base_form, init_action_list, is_actor, kind_test, slot_3d, FAMILY};
use super::{
    CELL_ATTACH_REFERENCE_3D, CELL_LOAD_3D, FORM_FLAGS_48, FORM_FLAG_NO_ATTACH, FORM_TYPE_23,
    GAME_LOADER_FLAGS, GAME_LOADER_POINTER, PROCESS_LISTS, PROCESS_LISTS_REMOVE_ACTOR,
    REFERENCE_EXTRA_LIST, RT_DYNAMIC_CAST, SCRIPT_SET_ACTION_FLAG, SLOT_KIND_STEP,
};
use crate::{dispatch, draw, slot_target};
use engine_core::extend::jip_patches::{NPC_PERKS, NPC_WEAPON_MODS};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::{fistp_chop64, x87_mul};
use engine_core::{args, typed_call, Engine};

/// The reference classes' slots: the two tests that hand the load on (0x224, 0x220) and where they
/// hand it (0x308, 0x304), the reference's own 3D load (`Load3D`, 0x1c8) and its position (0x1f4).
const SLOT_KIND_TEST_LOAD: u32 = 0x308;
const SLOT_OTHER_TEST: u32 = 0x220;
const SLOT_OTHER_LOAD: u32 = 0x304;
const SLOT_LOAD_3D: u32 = 0x1c8;
const SLOT_POSITION: u32 = 0x1f4;
/// The empty body `ret` (`00483710`), found in the slot 0x40 of the plain reference's, the tree's,
/// the actor's and the creature's task vtables; the NPC's and the player's hold `FinishAttach`.
const EMPTY_BODY: u32 = 0x0048_3710;
const SLOT_SOURCE_STEP: u32 = 0x40;
/// The source's model node (a counted pointer at +0x34, `005d8710`).
const SOURCE_NODE: u32 = 0x34;
/// A node's parent (`NiAVObject::m_pkParent`, +0x18, `009611e0`) and the parent's slot 0xe8 the
/// node is handed to.
const NODE_PARENT: u32 = 0x18;
const SLOT_PARENT_TAKE: u32 = 0xe8;
/// A form's flags (+8): seen from a distance (`0x8000`), the one `005651e0` tests (`0x80000`) and
/// the master particle addon nodes' (`0x1000000`).
const FORM_FLAGS: u32 = 8;
const FLAG_DISTANT: u32 = 0x8000;
const FLAG_80000: u32 = 0x0008_0000;
const FLAG_PARTICLES: u32 = 0x0100_0000;
/// A reference's loaded data (+0x64): its count (+4, `0057b200` asks whether it is above 0) and
/// flags (+0x10).
const REFERENCE_LOADED_DATA: u32 = 0x64;
const LOADED_COUNT: u32 = 4;
const LOADED_FLAGS: u32 = 0x10;
/// The cell's load state (`cCellState`, `00450fd0`) and its count of the loaded references seen
/// from a distance (`00452390`).
const CELL_STATE: u32 = 0x26;
const CELL_DISTANT_COUNT: u32 = 0xaa;
/// The shadow scene nodes (a table of four, `00450b80`; this uses the first).
const SHADOW_SCENE_NODES: u32 = 0x011f_91c8;
/// The callees that do the work, by address: the reference's 3D set (`(reference, node)`), its
/// actor base's `SetStartsDead(0)`, the shadow scene node's add (`(node)`), its actor's add, its
/// lighting update (`(node, 0)`), the reference's step after the attach, the master particle addon
/// nodes' removal and addition (`cdecl(node)`), `TESObjectREFR::InitScript`, `00547ad0` (`(cell;
/// position, count, 1.0)`), `Actor::UpdateAlpha`, the reference's scale, and the ID set on the
/// scene graph (`cdecl(node, form id)`).
const REFERENCE_SET_3D: u32 = 0x0057_0f70;
const SET_STARTS_DEAD: u32 = 0x0056_5210;
const SHADOW_ADD_OBJECT: u32 = 0x00b5_eeb0;
const SHADOW_ADD_ACTOR: u32 = 0x00b5_cbd0;
const SHADOW_UPDATE_LIGHTING: u32 = 0x00b5_d9f0;
const REFERENCE_AFTER_ATTACH: u32 = 0x0056_f700;
const REMOVE_PARTICLE_ADDON_NODES: u32 = 0x0057_8170;
const ADD_PARTICLE_ADDON_NODES: u32 = 0x0057_8060;
const REFERENCE_INIT_SCRIPT: u32 = 0x0056_5730;
const CELL_LARGE_STEP: u32 = 0x0054_7ad0;
const ACTOR_UPDATE_ALPHA: u32 = 0x008c_4640;
const REFERENCE_GET_SCALE: u32 = 0x0056_7400;
/// The form types this tests: two whose node is only handed to its parent, the one with its own
/// step, the one of the large cell's step.
const FORM_TYPE_2C: u8 = 0x2c;
const FORM_TYPE_2D: u8 = 0x2d;
const FORM_TYPE_1E: u8 = 0x1e;
const FORM_TYPE_25: u8 = 0x25;
/// The cast of a type 0x25 base form (type descriptors), the byte `00452480` answers, the cast's
/// sizes (two floats at +0x8c, `0045bb80`) and the `double` 200.0 both must pass, and the cast's
/// slot 0x1bc (its radius).
const RTTI_FORM: u32 = 0x0118_3108;
const RTTI_LARGE: u32 = 0x0118_6060;
const LARGE_STEP_ON: u32 = 0x0119_b8c8;
const CAST_SIZES: u32 = 0x8c;
const LARGE_LIMIT: u32 = 0x0101_79e0;
const SLOT_RADIUS: u32 = 0x1bc;
/// The player (a global), whose ID is not set on the scene graph here.
const PLAYER_POINTER: u32 = 0x011d_ea3c;
/// JIP's seam in place of the NPC's node lookup (`0x45234E`).
const JIP_ON_LOAD_ACTOR: u32 = 0x0045_234e;

/// The reference's form flags (+8).
#[inline]
fn form_flags(e: &Engine, reference: u32) -> u32 {
    e.mem.u32(reference.wrapping_add(FORM_FLAGS))
}

/// Whether the reference is seen from a distance (`00564e60`): its flag `0x8000`, or its base
/// form's (the word at +0x20, read in place).
#[inline]
fn seen_from_a_distance(e: &Engine, reference: u32) -> bool {
    if form_flags(e, reference) & FLAG_DISTANT != 0 {
        return true;
    }
    let base = e.mem.u32(reference.wrapping_add(0x20));
    base != 0 && e.mem.u32(base.wrapping_add(FORM_FLAGS)) & FLAG_DISTANT != 0
}

/// The first shadow scene node.
#[inline]
fn shadow_scene_node(e: &Engine) -> u32 {
    e.mem.u32(SHADOW_SCENE_NODES)
}

/// The reference's slot 0x220: the false body (`0047c850`, most reference classes') and the true
/// one (`008d0360`, `Explosion`'s) as their typed calls, any other through the vtable.
#[inline]
fn other_test(e: &mut Engine, reference: u32) -> bool {
    match slot_target(e, reference, SLOT_OTHER_TEST) {
        0x0047_c850 => typed_call!(
            draw::FAMILY,
            0x0047_c850,
            draw::shapes::answers_false(e, reference)
        ),
        0x008d_0360 => typed_call!(FAMILY, 0x008d_0360, answers_true(e, reference)),
        _ => e.vcall(reference, SLOT_OTHER_TEST, &args![]).bool(),
    }
}

/// `TESObjectREFR::Get3D` (`0043fcd0`) by address, as the exe calls it here (not through the slot).
#[inline]
fn get_3d(e: &mut Engine, reference: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0043_fcd0,
        dispatch::scene::reference_3d(e, Ptr::new(reference))
    )
    .addr()
}

// Typed from 00451ef0
/// `TES`'s load of `reference` into the scene (`this` is the `TES`, read only by the parts it hands
/// on): nothing for a null reference; a reference whose slot 0x224 or 0x220 says so is handed to
/// its slot 0x308 or 0x304 and nothing else is done. Otherwise the reference seen from a distance
/// with flag `0x80000` and a 3D gives it up first ([`give_up_distant_3d`]); a reference with form
/// flag `0x20` or `0x800` stops there; with `flag` 0 an actor leaves the process lists' temp-change
/// list. The node is the source's (set as the reference's 3D when it has none) or the reference's
/// own `Load3D` (slot 0x1c8); the source's slot 0x40 runs. A node of a type 0x2c or 0x2d base form
/// goes to its parent's slot 0xe8; any other is put in the scene ([`into_scene`]). Always: the
/// script is set up (`TESObjectREFR::InitScript`) and, unless the game loader's flag says
/// otherwise, the action list (`Script::InitActionList`, `SetActionFlag(0x1000)`); a type 0x25 base
/// form gets its large cell step ([`large_cell_step`]); an actor gets `Actor::UpdateAlpha` and,
/// unless it is the player, its form ID set on its node in the scene graph.
pub fn load_reference(
    e: &mut Engine,
    _this: u32,
    reference: u32,
    cell: u32,
    source: u32,
    flag: u8,
) {
    if reference == 0 {
        return;
    }
    if kind_test(e, reference) {
        e.vcall(reference, SLOT_KIND_TEST_LOAD, &args![]);
        return;
    }
    if other_test(e, reference) {
        e.vcall(reference, SLOT_OTHER_LOAD, &args![]);
        return;
    }
    give_up_distant_3d(e, reference);
    let base = base_form(e, reference);
    let kind = e.mem.u8(base.wrapping_add(4));
    // `00451e40`: the base form and its type asked again (the type not used), then the two flags.
    base_form(e, reference);
    if typed_call!(leaf::FAMILY, 0x0044_0d80, leaf::form_flag_20(e, reference))
        || typed_call!(leaf::FAMILY, 0x0044_0da0, leaf::form_flag_800(e, reference))
    {
        return;
    }
    if flag == 0 && is_actor(e, reference) {
        // `00451ea0`: the actor's step and the process lists' temp-change list.
        e.vcall(reference, SLOT_KIND_STEP, &args![]);
        e.call(PROCESS_LISTS_REMOVE_ACTOR, &args![PROCESS_LISTS, reference]);
    }
    let source_node = if source == 0 {
        0
    } else {
        e.mem.u32(source.wrapping_add(SOURCE_NODE))
    };
    let node = if source_node != 0 {
        let node = e.mem.u32(source.wrapping_add(SOURCE_NODE));
        if get_3d(e, reference) == 0 {
            e.call(REFERENCE_SET_3D, &args![reference, node]);
        }
        node
    } else {
        e.vcall(reference, SLOT_LOAD_3D, &args![0u32]).u32()
    };
    if source != 0 && slot_target(e, source, SLOT_SOURCE_STEP) != EMPTY_BODY {
        e.vcall(source, SLOT_SOURCE_STEP, &args![]);
    }
    if node != 0 {
        if kind == FORM_TYPE_2C || kind == FORM_TYPE_2D {
            if e.mem.u32(node.wrapping_add(NODE_PARENT)) != 0 {
                let parent = e.mem.u32(node.wrapping_add(NODE_PARENT));
                e.vcall(parent, SLOT_PARENT_TAKE, &args![node]);
            }
        } else {
            into_scene(e, reference, cell, node, kind);
        }
    }
    e.call(REFERENCE_INIT_SCRIPT, &args![reference]);
    let loader = e.global::<u32>(GAME_LOADER_POINTER);
    if e.mem.u32(loader.wrapping_add(GAME_LOADER_FLAGS)) & 2 == 0 {
        let list = reference.wrapping_add(REFERENCE_EXTRA_LIST);
        typed_call!(FAMILY, 0x005a_c190, init_action_list(e, reference, list));
        e.call(SCRIPT_SET_ACTION_FLAG, &args![reference, list, 0x1000u32]);
    }
    if kind == FORM_TYPE_25 {
        large_cell_step(e, reference, cell);
    }
    if is_actor(e, reference) {
        e.call(ACTOR_UPDATE_ALPHA, &args![reference]);
        if reference != e.global::<u32>(PLAYER_POINTER) {
            let form_id = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, reference));
            let jip_node = if e.jip_patches() & (NPC_WEAPON_MODS | NPC_PERKS) != 0 {
                e.site(JIP_ON_LOAD_ACTOR, &[reference])
            } else {
                None
            };
            let node = match jip_node {
                Some(node) => node,
                None => get_3d(e, reference),
            };
            typed_call!(
                FAMILY,
                0x004b_6dc0,
                super::scenegraph::set_reference_id_on_scenegraph(e, node, form_id)
            );
        }
    }
}

/// `00451da0`: a reference seen from a distance, with a 3D and its flag `0x80000`, gives the 3D up:
/// the node's parent (if any) is handed the node (its slot 0xe8), the reference's 3D is set to none
/// and its actor base's `SetStartsDead(0)` runs. The 3D is asked of the slot 0x1d0 each time the
/// exe asks it.
fn give_up_distant_3d(e: &mut Engine, reference: u32) {
    if !seen_from_a_distance(e, reference) || slot_3d(e, reference) == 0 {
        return;
    }
    if form_flags(e, reference) & FLAG_80000 == 0 {
        return;
    }
    let node = slot_3d(e, reference);
    let parent = e.mem.u32(node.wrapping_add(NODE_PARENT));
    if parent != 0 {
        let node = slot_3d(e, reference);
        e.vcall(parent, SLOT_PARENT_TAKE, &args![node]);
    }
    e.call(REFERENCE_SET_3D, &args![reference, 0u32]);
    e.call(SET_STARTS_DEAD, &args![reference, 0u32]);
}

/// The node put in the scene: the cell's 3D loaded (`TESObjectCELL::Load3D`), the node added to the
/// first shadow scene node (and as an actor's for an actor), the cell's count of references seen
/// from a distance up by one for one, the reference attached (`TESObjectCELL::AttachReference3D`,
/// then `0056f700`) unless its base form is of type 0x23 with flag `0x40000000`. The master
/// particle addon nodes are removed (form flag `0x1000000` and a loaded data count above 0) or else
/// added (loaded data flag 2 in a cell of state 6 or 5). Last the lighting is updated, but for type
/// 0x1e (whose step is the empty body `004534f0`).
fn into_scene(e: &mut Engine, reference: u32, cell: u32, node: u32, kind: u8) {
    e.call(CELL_LOAD_3D, &args![cell]);
    let shadow = shadow_scene_node(e);
    e.call(SHADOW_ADD_OBJECT, &args![shadow, node]);
    if is_actor(e, reference) {
        let shadow = shadow_scene_node(e);
        e.call(SHADOW_ADD_ACTOR, &args![shadow, node]);
    }
    if seen_from_a_distance(e, reference) {
        let at = cell.wrapping_add(CELL_DISTANT_COUNT);
        let count = e.mem.u16(at);
        e.mem.set_u16(at, count.wrapping_add(1));
    }
    let mut attach = true;
    let base = base_form(e, reference);
    if e.mem.u8(base.wrapping_add(4)) == FORM_TYPE_23 {
        let base = base_form(e, reference);
        if e.mem.u32(base.wrapping_add(FORM_FLAGS_48)) & FORM_FLAG_NO_ATTACH != 0 {
            attach = false;
        }
    }
    if attach {
        e.call(CELL_ATTACH_REFERENCE_3D, &args![cell, reference, 0u32]);
        e.call(REFERENCE_AFTER_ATTACH, &args![reference]);
    }
    let loaded = e.mem.u32(reference.wrapping_add(REFERENCE_LOADED_DATA));
    if form_flags(e, reference) & FLAG_PARTICLES != 0
        && loaded != 0
        && e.mem.i32(loaded.wrapping_add(LOADED_COUNT)) > 0
    {
        e.call(REMOVE_PARTICLE_ADDON_NODES, &args![node]);
    } else if loaded != 0 && e.mem.u32(loaded.wrapping_add(LOADED_FLAGS)) & 2 != 0 {
        let state = e.mem.u8(cell.wrapping_add(CELL_STATE));
        if state == 6 || state == 5 {
            e.call(ADD_PARTICLE_ADDON_NODES, &args![node]);
        }
    }
    // The exe asks the slot 0x100 here for an actor's step, the empty body `00483710`: the question
    // stays (a class's slot is called as the exe calls it), the step goes.
    is_actor(e, reference);
    if kind == FORM_TYPE_1E {
        // `004534f0(base form, extra data)` is empty: the base form asked of the reference stays.
        base_form(e, reference);
    } else {
        let shadow = shadow_scene_node(e);
        e.call(SHADOW_UPDATE_LIGHTING, &args![shadow, node, 0u32]);
    }
}

/// A type 0x25 base form cast to its class (`__RTDynamicCast`) in a large cell (the byte `00452480`
/// set, both sizes above 200.0): `00547ad0` on the cell with the reference's position (slot 0x1f4),
/// the radius (the cast's slot 0x1bc) times the reference's scale as an integer toward zero, and
/// 1.0.
fn large_cell_step(e: &mut Engine, reference: u32, cell: u32) {
    let base = base_form(e, reference);
    let cast = e
        .call(
            RT_DYNAMIC_CAST,
            &args![base, 0u32, RTTI_FORM, RTTI_LARGE, 0u32],
        )
        .u32();
    if cast == 0 || e.mem.u8(LARGE_STEP_ON) == 0 {
        return;
    }
    let sizes = cast.wrapping_add(CAST_SIZES);
    let (first, second) = (e.mem.f32(sizes), e.mem.f32(sizes.wrapping_add(4)));
    let limit = e.mem.f64(LARGE_LIMIT);
    if !(f64::from(first) > limit && f64::from(second) > limit) {
        return;
    }
    let radius = e.vcall(cast, SLOT_RADIUS, &args![]).f64();
    let scale = e.call(REFERENCE_GET_SCALE, &args![reference]).f64();
    let [b0, b1, b2, b3, ..] = fistp_chop64(x87_mul(scale, radius)).to_le_bytes();
    let count = u32::from_le_bytes([b0, b1, b2, b3]);
    let position = e.vcall(reference, SLOT_POSITION, &args![]).u32();
    let x = e.mem.u32(position);
    let y = e.mem.u32(position.wrapping_add(4));
    let z = e.mem.u32(position.wrapping_add(8));
    e.call(CELL_LARGE_STEP, &args![cell, x, y, z, count, 1.0f32]);
}
