//! `TESPackage`'s place functions (Xbox PDB; the unit `fallout shared/package`, `00674fd0` to
//! `00678ec0`): the world space, cell, position, radius and reference of the package's own, second
//! and search locations for an actor, the per-type data getters' neighbour `ComputeInitialTarget`,
//! the two "is the subject at the location" tests, `IsActorAtRefTarget`, `IsTargetAtLocation`,
//! `IsTargetAtSecondLocation`, the follow target's search and its callback, the two radius
//! functions and `IsTargetAnActor`.
//!
//! A location (`PackageLocation`, 0xc bytes) has its type as a signed byte at +0, its radius at +4
//! and its reference, cell, object or object type at +8; a target (`PackageTarget`, 0x10 bytes) its
//! type as a byte at +0 and its reference, object or object type at +4. A reference keeps its base
//! form at +0x20, its parent cell at +0x40, its extra data list at +0x44 and an actor its process
//! at +0x68; a form its type as a byte at +4; a cell its flag byte at +0x24 (bit 0: interior) and
//! its world space at +0xc0.
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds
//! (checked in the exe): the package's type (`0041ca90`), location (`0055b980`), target
//! (`00671d10`) and procedure type (`009611e0`), the second location's read (`00672dd0`), a
//! location's type (`00678ca0`), its reference, cell, object and object type (`0067f390`,
//! `0067f3e0`, `0067f430`, `0067f480`: the word at +8 for the location types 0, 1, 4 and 5), its
//! radius (`0067f1c0`: the word at +4 unless the type byte is 0xff or 1) and its value test
//! (`00454b70`), a target's type (`00519b00`), reference (`00680020`), object and object type
//! (`00680050`, `00680080`: the word at +4 for the target types 1 and 2) and value (`0044ddc0`),
//! a form's type (`00401170`), a reference's base form (`007af430`) and parent cell (`008d6f30`),
//! an actor's process (`008d8520`) and the process's word at +0x28 (`0045cd60`), a cell's
//! interior bit (`00425fd0`) and world space (`0054ddd0`), a reference's furniture test
//! (`00568680`: a base form of type 0x27), the form flag 0x20 test (`00440d80`), the extra data
//! list's address (`005d43c0`), the smaller of two floats (`0040ebd0`), the settings' value
//! addresses (`0043d4d0`, `00403e20`, `00403df0`: each setting is a fixed object, so its value is
//! the word at +4), the C runtime's `_finite` and the empty error report (`0040fbe0`). The exe's
//! constants are the numbers it holds: -1.0 for "no extra radius", 100.0 for the height and
//! furniture distance limit, 0.5 and 2.0, 20.0 for the margin and 2.0 for the message's time.
//! What the exe asks twice of a one-line body with nothing between is read once. The family's own
//! functions, the point maths (the pathing family's difference, flat length and absolute value),
//! the C runtime's `_ftol2`, the bound box's two corners, the interface's picked reference, the
//! current package and the extra data's package and pointer are typed calls.
//!
//! What stays a call by address: a reference's world space (`00575d70`, which asks a slot), its
//! linked reference (`00569b80`), its start location's world, cell and position and their setter
//! (`0056ad30`, `0056ad80`, `0056add0`, `009316c0`), a cell's first reference and its grid
//! position (`0054cee0`, `00544c30`, `00544c60`), the location's resolved reference and own test
//! (`0067f2a0`, `0067f220`), the model bound's size (`00571600`), the linked door's test and
//! position, the eye level, the data handler's walk of nearby references, the followers' calls,
//! `sprintf`, the loader's warning and the on-screen message (both of which a fix or an extender
//! may replace); every slot of a reference, an actor and a process is called through its slot.
//!
//! Floats: a position is moved as its three words; the height difference is the exe's x87
//! subtract stored as a float; an integer radius becomes the nearest float (`FILD`, `FSTP` under the round-to-nearest
//! control word);
//! a float becomes an integer through the C runtime's typed `_ftol2` where the exe calls it, and
//! the bound box's half side through `FISTP` under the truncating control word the exe sets; the
//! reach adjustment's sums and quotient are the exe's x87 maths.

use super::{object, package, FAMILY};
use crate::{ai_queries, crt, extra_data, interface, pathing};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::{
    fistp_chop64, x87_add, x87_div, x87_load32, x87_quiet, x87_store32, x87_sub,
};
use engine_core::{args, typed_call, Engine};

/// The package's fields: procedure type, type, data object, location, target; the dialogue
/// package object's stored reference (type 0x1c).
const PROCEDURE: u32 = 0x18;
const PACK_TYPE: u32 = 0x20;
const DATA: u32 = 0x28;
const LOCATION: u32 = 0x2c;
const TARGET: u32 = 0x30;
const DIALOGUE_STORED_REFERENCE: u32 = 0x9c;
/// A reference's base form, parent cell, extra data list, process; a form's type and flags.
const BASE_FORM: u32 = 0x20;
const PARENT_CELL: u32 = 0x40;
const EXTRA_DATA_LIST: u32 = 0x44;
const PROCESS: u32 = 0x68;
const FORM_TYPE: u32 = 4;
const FORM_FLAGS: u32 = 8;
const FORM_FLAG_20: u32 = 0x20;
/// A cell's flag byte (bit 0: interior) and world space; a process's word at +0x28.
const CELL_FLAGS: u32 = 0x24;
const CELL_WORLD_SPACE: u32 = 0xc0;
const PROCESS_WORD_28: u32 = 0x28;

/// Calls by address: a reference's world space, linked reference, start location's world, cell
/// and position and their setter; a cell's first reference and grid position; the location's
/// resolved reference and own test; the model bound's size; the linked door's test and position.
const REFERENCE_WORLD_SPACE: u32 = 0x0057_5d70;
const LINKED_REFERENCE: u32 = 0x0056_9b80;
const START_LOCATION_WORLD: u32 = 0x0056_ad30;
const START_LOCATION_CELL: u32 = 0x0056_ad80;
const START_LOCATION_COORD: u32 = 0x0056_add0;
const SET_START_LOCATION: u32 = 0x0093_16c0;
const CELL_FIRST_REFERENCE: u32 = 0x0054_cee0;
const CELL_GRID_X: u32 = 0x0054_4c30;
const CELL_GRID_Y: u32 = 0x0054_4c60;
const LOCATION_RESOLVE_REFERENCE: u32 = 0x0067_f2a0;
const LOCATION_TEST_SUBJECT: u32 = 0x0067_f220;
const MODEL_BOUND_SIZE: u32 = 0x0057_1600;
const HAS_LINKED_DOOR: u32 = 0x0056_8e50;
const LINKED_DOOR_POSITION: u32 = 0x0056_8fa0;
/// The loader's warning (a fix may replace it), `sprintf`, the actor's eye level, the data
/// handler's walk of the references near a point, the follower calls, the reference-of-form test,
/// the player's two follower tests and the on-screen message (xNVSE may replace it).
const WARNING: u32 = 0x005b_5e40;
const SPRINTF: u32 = 0x00ec_623a;
const EYE_LEVEL: u32 = 0x008b_e940;
const NEARBY_REFERENCES: u32 = 0x0046_f280;
const FOLLOW_CANDIDATE: u32 = 0x0067_84f0;
const ADD_FOLLOWER: u32 = 0x008b_c790;
const REJECTS_CANDIDATE: u32 = 0x008b_c7d0;
const REFERENCE_OF_FORM: u32 = 0x0055_d3b0;
const PLAYER_FOLLOWER_TEST: u32 = 0x0096_2720;
const PLAYER_FOLLOWER_COUNT: u32 = 0x0096_2620;
const SHOW_MESSAGE: u32 = 0x0070_52f0;
/// The messages: the two coordinate warnings, the radius and distance report.
const TEXT_NO_REFERENCE: u32 = 0x0106_8718;
const TEXT_NO_LINKED_REFERENCE: u32 = 0x0106_86d0;
const TEXT_RADIUS_DISTANCE: u32 = 0x0106_8764;

/// Globals: the default point, the player, the two marker base forms, the follow search's found
/// reference, the data handler; the settings (each a fixed object with its value at +4): the
/// default radius, the height tolerance, the follow radius, the follower limit, the message.
const DEFAULT_POINT: u32 = 0x011f_426c;
const PLAYER: u32 = 0x011d_ea3c;
const MARKER_FORM_A: u32 = 0x011c_a248;
const MARKER_FORM_B: u32 = 0x011c_a244;
const FOUND_REFERENCE: u32 = 0x011d_6580;
const DATA_HANDLER: u32 = 0x011c_3f2c;
const DEFAULT_RADIUS_SETTING: u32 = 0x011c_de98;
const HEIGHT_TOLERANCE_SETTING: u32 = 0x011d_09d0;
const FOLLOW_RADIUS_SETTING: u32 = 0x011d_65ac;
const FOLLOWER_LIMIT_SETTING: u32 = 0x011c_dad0;
const MESSAGE_SETTING: u32 = 0x011d_3d54;
const SETTING_VALUE: u32 = 4;
/// The exe's constants: "no extra radius", the distance limit, a half, two, the margin and the
/// message's time.
const NO_EXTRA_RADIUS: f64 = -1.0;
const DISTANCE_LIMIT: f64 = 100.0;
const HALF: f64 = 0.5;
const TWO: f64 = 2.0;
const MARGIN: f32 = 20.0;
const MESSAGE_TIME: f32 = 2.0;

/// Slots: a reference's position (a pointer to three floats), an actor's point (into a buffer),
/// world space and cell, "is an actor", its kind, its 0x290 test, the reference it heads for, the
/// reach test, the reach height, the extent (into a buffer), the candidate rejection, a form's
/// 0xe4 test; a process's package, its references for the first and second location, its point,
/// its 0x4c8 reference, its 0x52c reference and the follower refusal.
const SLOT_POSITION: u32 = 0x1f4;
const SLOT_ACTOR_POINT: u32 = 0x170;
const SLOT_ACTOR_WORLD_SPACE: u32 = 0x294;
const SLOT_ACTOR_CELL: u32 = 0x298;
const SLOT_IS_ACTOR: u32 = 0x100;
const SLOT_KIND: u32 = 0x214;
const SLOT_ACTOR_290: u32 = 0x290;
const SLOT_TARGET_REFERENCE: u32 = 0x2c8;
const SLOT_REACH: u32 = 0x2cc;
const SLOT_REACH_HEIGHT: u32 = 0x380;
const SLOT_EXTENT: u32 = 0x1dc;
const SLOT_REJECT: u32 = 0x22c;
const SLOT_FORM_E4: u32 = 0xe4;
const SLOT_PROCESS_PACKAGE: u32 = 0x27c;
const SLOT_PROCESS_FIRST: u32 = 0x514;
const SLOT_PROCESS_SECOND: u32 = 0x51c;
const SLOT_PROCESS_POINT: u32 = 0x4d4;
const SLOT_PROCESS_4C8: u32 = 0x4c8;
const SLOT_PROCESS_52C: u32 = 0x52c;
const SLOT_PROCESS_REFUSE: u32 = 0x288;
/// The form's name slot.
const SLOT_FORM_NAME: u32 = 0x130;

fn at(base: u32, offset: u32) -> u32 {
    base.wrapping_add(offset)
}

/// The package type (`0041ca90`).
fn pack_type(e: &Engine, this: u32) -> i32 {
    i32::from(e.mem.i8(at(this, PACK_TYPE)))
}

/// The package's target (`00671d10`).
fn target_of(e: &Engine, this: u32) -> u32 {
    e.mem.u32(at(this, TARGET))
}

/// The second location's read (`00672dd0`): the data object's location word for the types that
/// have one.
fn held_second_location(e: &Engine, this: u32) -> u32 {
    let data = e.mem.u32(at(this, DATA));
    if data == 0 {
        return 0;
    }
    let offset = match pack_type(e, this) {
        0xf | 0x1c => 0xc,
        1 | 2 | 3 | 8 | 9 => 4,
        0x10 => 8,
        _ => return 0,
    };
    e.mem.u32(at(data, offset))
}

/// A location's type (`00678ca0`, the signed byte).
fn location_type(e: &Engine, location: u32) -> i32 {
    i32::from(e.mem.i8(location))
}

/// A location's word at +8 when its type is `kind` (`0067f390` 0, `0067f3e0` 1, `0067f430` 4,
/// `0067f480` 5), else 0.
fn location_word(e: &Engine, location: u32, kind: i32) -> u32 {
    if location_type(e, location) == kind {
        e.mem.u32(at(location, 8))
    } else {
        0
    }
}

/// A location's radius (`0067f1c0`): its word at +4 unless the type byte is 0xff or 1.
fn location_radius(e: &Engine, location: u32) -> u32 {
    match e.mem.u8(location) {
        0xff | 1 => 0,
        _ => e.mem.u32(at(location, 4)),
    }
}

/// A target's type (`00519b00`, the byte).
fn target_type(e: &Engine, target: u32) -> u32 {
    u32::from(e.mem.u8(target))
}

/// A target's word at +4 when its type is `kind` (`00680020` 0, `00680050` 1, `00680080` 2).
fn target_word(e: &Engine, target: u32, kind: u32) -> u32 {
    if target_type(e, target) == kind {
        e.mem.u32(at(target, 4))
    } else {
        0
    }
}

/// A form's type (`00401170`).
fn form_type(e: &Engine, form: u32) -> u32 {
    u32::from(e.mem.u8(at(form, FORM_TYPE)))
}

/// A reference's base form (`007af430`).
fn base_form(e: &Engine, reference: u32) -> u32 {
    e.mem.u32(at(reference, BASE_FORM))
}

/// A reference's parent cell (`008d6f30`).
fn parent_cell(e: &Engine, reference: u32) -> u32 {
    e.mem.u32(at(reference, PARENT_CELL))
}

/// An actor's process (`008d8520`).
fn process_of(e: &Engine, actor: u32) -> u32 {
    e.mem.u32(at(actor, PROCESS))
}

/// A cell's interior bit (`00425fd0`).
fn is_interior(e: &Engine, cell: u32) -> bool {
    e.mem.u8(at(cell, CELL_FLAGS)) & 1 != 0
}

/// A cell's world space (`0054ddd0`): none for an interior cell.
fn cell_world_space(e: &Engine, cell: u32) -> u32 {
    if is_interior(e, cell) {
        0
    } else {
        e.mem.u32(at(cell, CELL_WORLD_SPACE))
    }
}

/// The cell, or 0 when it is not an interior cell (the "is at" tests' cell).
fn interior_or_none(e: &Engine, cell: u32) -> u32 {
    if cell != 0 && !is_interior(e, cell) {
        0
    } else {
        cell
    }
}

/// `TESObjectREFR::IsFurniture` (`00568680`): a base form of type 0x27.
fn is_furniture(e: &Engine, reference: u32) -> bool {
    let base = base_form(e, reference);
    base != 0 && form_type(e, base) == 0x27
}

/// Whether the reference's base form is one of the two marker forms the data section keeps.
fn is_marker(e: &Engine, reference: u32) -> bool {
    let base = base_form(e, reference);
    base == e.mem.u32(MARKER_FORM_A) || base == e.mem.u32(MARKER_FORM_B)
}

/// A setting's value word (`0043d4d0`, `00403e20`, `00403df0` on a fixed setting object).
fn setting(e: &Engine, object: u32) -> u32 {
    e.mem.u32(at(object, SETTING_VALUE))
}

fn reference_world(e: &mut Engine, reference: u32) -> u32 {
    e.call(REFERENCE_WORLD_SPACE, &args![reference]).u32()
}

fn linked_reference(e: &mut Engine, reference: u32) -> u32 {
    e.call(LINKED_REFERENCE, &args![reference]).u32()
}

fn is_actor(e: &mut Engine, reference: u32) -> bool {
    e.vcall(reference, SLOT_IS_ACTOR, &args![]).bool()
}

fn kind_of(e: &mut Engine, actor: u32) -> i32 {
    e.vcall(actor, SLOT_KIND, &args![]).i32()
}

/// `_ftol2` (the C runtime's typed one): the low word.
fn truncate(e: &mut Engine, value: f64) -> i32 {
    let whole = typed_call!(crt::FAMILY, 0x00ec_62c0, crt::math::ftol2_sse(e, value));
    i32::from_ne_bytes(crt::low_word(whole).to_ne_bytes())
}

/// The model bound's size, truncated.
fn model_bound(e: &mut Engine, reference: u32) -> i32 {
    let size = e.call(MODEL_BOUND_SIZE, &args![reference]).f32();
    truncate(e, f64::from(size))
}

/// The reference the actor's process chose for this package (`slot`: the first or the second
/// location's), or `None` when the actor has no process or its process runs another package.
fn chosen_reference(e: &mut Engine, this: u32, actor: u32, slot: u32) -> Option<u32> {
    if actor == 0 {
        return None;
    }
    let process = process_of(e, actor);
    if process == 0 || e.vcall(process, SLOT_PROCESS_PACKAGE, &args![]).u32() != this {
        return None;
    }
    let process = process_of(e, actor);
    Some(e.vcall(process, slot, &args![]).u32())
}

/// The second-location getters' fallback for the location types 4 and 5: the player running a
/// type 0x1c package with a stored reference.
fn player_dialogue_reference(e: &Engine, this: u32, actor: u32) -> Option<u32> {
    if actor != e.mem.u32(PLAYER) || pack_type(e, this) != 0x1c {
        return None;
    }
    let held = e.mem.u32(at(this, DIALOGUE_STORED_REFERENCE));
    (held != 0).then_some(held)
}

/// The three words at `address`.
fn point_at(e: &Engine, address: u32) -> [u32; 3] {
    [0, 4, 8].map(|k| e.mem.u32(at(address, k)))
}

fn store_point(e: &mut Engine, out: u32, point: [u32; 3]) {
    for (k, word) in (0u32..).zip(point) {
        e.mem.set_u32(at(out, 4 * k), word);
    }
}

/// A reference's position (its slot's three floats).
fn position(e: &mut Engine, reference: u32) -> [u32; 3] {
    let address = e.vcall(reference, SLOT_POSITION, &args![]).u32();
    point_at(e, address)
}

/// An actor's point (its slot writes it into a buffer).
fn actor_point(e: &mut Engine, actor: u32) -> [u32; 3] {
    e.with_stack(12, |e, buffer| {
        let address = e.vcall(actor, SLOT_ACTOR_POINT, &args![buffer]).u32();
        point_at(e, address)
    })
}

/// The point of a cell's first reference, or for a cell without one that is not an interior the
/// corner of its grid square; `None` for an interior cell without references.
fn cell_point(e: &mut Engine, cell: u32, location: u32) -> Option<[u32; 3]> {
    let first = e.call(CELL_FIRST_REFERENCE, &args![cell]).u32();
    if first != 0 {
        return Some(position(e, first));
    }
    if is_interior(e, cell) {
        return None;
    }
    let grid = location_word(e, location, 1);
    let x = e.call(CELL_GRID_X, &args![grid]).i32();
    let y = e.call(CELL_GRID_Y, &args![grid]).i32();
    Some([
        ((x << 12) as f32).to_bits(),
        ((y << 12) as f32).to_bits(),
        0,
    ])
}

/// Logs a coordinate warning with the actor's name, then answers the actor's point.
fn warn_and_actor_point(e: &mut Engine, actor: u32, text: u32) -> [u32; 3] {
    let name = e.vcall(actor, SLOT_FORM_NAME, &args![]).u32();
    e.call(WARNING, &args![text, name]);
    actor_point(e, actor)
}

// Typed from 00674fd0
/// `TESPackage::GetSecondLocationWorld` (Xbox PDB): the world space of the second location for
/// `actor`: by location type 0 the reference's, 1 the cell's, 3 the actor's own (its slot), 4 and
/// 5 that of the reference the actor's process chose (or the actor), else the player's dialogue
/// reference's, 6 the linked reference's; no second location or type 2 the actor's start location's.
pub fn second_location_world(e: &mut Engine, this: u32, actor: u32) -> u32 {
    let location = held_second_location(e, this);
    if location == 0 || location_type(e, location) == 2 {
        if actor != 0 {
            return e.call(START_LOCATION_WORLD, &args![actor]).u32();
        }
        return 0;
    }
    match location_type(e, location) {
        0 => match location_word(e, location, 0) {
            0 => 0,
            reference => reference_world(e, reference),
        },
        1 => match location_word(e, location, 1) {
            0 => 0,
            cell => cell_world_space(e, cell),
        },
        3 if actor != 0 => e.vcall(actor, SLOT_ACTOR_WORLD_SPACE, &args![]).u32(),
        4 | 5 => {
            if let Some(chosen) = chosen_reference(e, this, actor, SLOT_PROCESS_SECOND) {
                let source = if chosen == 0 { actor } else { chosen };
                return reference_world(e, source);
            }
            match player_dialogue_reference(e, this, actor) {
                Some(held) => reference_world(e, held),
                None => 0,
            }
        }
        6 if actor != 0 && linked_reference(e, actor) != 0 => {
            let linked = linked_reference(e, actor);
            reference_world(e, linked)
        }
        _ => 0,
    }
}

// Typed from 006751a0
/// `TESPackage::GetSecondLocationCell` (Xbox PDB): as [`second_location_world`], for the cell: the
/// reference's parent cell, the cell itself, the actor's (its slot), the chosen reference's or the
/// actor's (the first location's process slot), the linked reference's; else the start location's
/// interior cell.
pub fn second_location_cell(e: &mut Engine, this: u32, actor: u32) -> u32 {
    let location = held_second_location(e, this);
    if location == 0 || location_type(e, location) == 2 {
        if actor != 0 {
            return e.call(START_LOCATION_CELL, &args![actor]).u32();
        }
        return 0;
    }
    match location_type(e, location) {
        0 => match location_word(e, location, 0) {
            0 => 0,
            reference => parent_cell(e, reference),
        },
        1 => location_word(e, location, 1),
        3 if actor != 0 => e.vcall(actor, SLOT_ACTOR_CELL, &args![]).u32(),
        4 | 5 => {
            if let Some(chosen) = chosen_reference(e, this, actor, SLOT_PROCESS_FIRST) {
                let source = if chosen == 0 { actor } else { chosen };
                return parent_cell(e, source);
            }
            match player_dialogue_reference(e, this, actor) {
                Some(held) => parent_cell(e, held),
                None => 0,
            }
        }
        6 if actor != 0 && linked_reference(e, actor) != 0 => {
            let linked = linked_reference(e, actor);
            parent_cell(e, linked)
        }
        _ => 0,
    }
}

// Typed from 00675360
/// `TESPackage::GetSecondLocationCoord` (Xbox PDB): the second location's position for `actor`
/// written to `out` (from the default point): the reference's, the cell's first reference's or its
/// grid corner, the actor's point, the chosen reference's or the actor's, the player's dialogue
/// reference's, the linked reference's; else the start location's. Returns `out`.
pub fn second_location_coord(e: &mut Engine, this: u32, out: u32, actor: u32) -> u32 {
    let location = held_second_location(e, this);
    let mut point = point_at(e, DEFAULT_POINT);
    if location == 0 || location_type(e, location) == 2 {
        if actor != 0 {
            let address = e.call(START_LOCATION_COORD, &args![actor]).u32();
            point = point_at(e, address);
        }
        store_point(e, out, point);
        return out;
    }
    match location_type(e, location) {
        0 => {
            let reference = location_word(e, location, 0);
            if reference != 0 {
                point = position(e, reference);
            }
        }
        1 => {
            let cell = location_word(e, location, 1);
            if cell != 0 {
                if let Some(found) = cell_point(e, cell, location) {
                    point = found;
                }
            }
        }
        3 if actor != 0 => point = actor_point(e, actor),
        4 | 5 => {
            if let Some(chosen) = chosen_reference(e, this, actor, SLOT_PROCESS_SECOND) {
                let source = if chosen == 0 { actor } else { chosen };
                point = position(e, source);
            } else if let Some(held) = player_dialogue_reference(e, this, actor) {
                point = position(e, held);
            }
        }
        6 if actor != 0 && linked_reference(e, actor) != 0 => {
            let linked = linked_reference(e, actor);
            point = position(e, linked);
        }
        _ => {}
    }
    store_point(e, out, point);
    out
}

// Typed from 00675670
/// The second location's radius (`0067f1c0` in place), 0 without one.
pub fn second_location_radius(e: &mut Engine, this: u32) -> u32 {
    match held_second_location(e, this) {
        0 => 0,
        location => location_radius(e, location),
    }
}

// Typed from 006756a0
/// The reference the second location stands for, for `actor`: the actor without a second location
/// or for the types 2 and 3, the location's reference for 0, the chosen reference (or the actor)
/// for 4 and 5, the linked reference for 6; 0 otherwise.
pub fn second_location_reference(e: &mut Engine, this: u32, actor: u32) -> u32 {
    let location = held_second_location(e, this);
    if location == 0 || location_type(e, location) == 2 {
        return actor;
    }
    match location_type(e, location) {
        0 => location_word(e, location, 0),
        3 => actor,
        4 | 5 => match chosen_reference(e, this, actor, SLOT_PROCESS_SECOND) {
            Some(0) => actor,
            Some(chosen) => chosen,
            None => 0,
        },
        6 if actor != 0 && linked_reference(e, actor) != 0 => linked_reference(e, actor),
        _ => 0,
    }
}

/// Whether the search location is the package's own location.
fn searches_own_location(e: &mut Engine, this: u32) -> bool {
    let search = typed_call!(FAMILY, 0x0067_2f20, package::search_location(e, this));
    search == e.mem.u32(at(this, LOCATION))
}

// Typed from 006757e0
/// `TESPackage::GetSearchLocationCell` (Xbox PDB): the own location's cell when the search location
/// is the own location, else the second location's.
pub fn search_location_cell(e: &mut Engine, this: u32, actor: u32) -> u32 {
    if searches_own_location(e, this) {
        typed_call!(FAMILY, 0x0067_5c20, location_cell(e, this, actor))
    } else {
        typed_call!(FAMILY, 0x0067_51a0, second_location_cell(e, this, actor))
    }
}

// Typed from 00675830
/// `TESPackage::GetSearchLocationCoord` (Xbox PDB): the own or second location's position, as
/// [`search_location_cell`] picks, written to `out` through a buffer of its own. Returns `out`.
pub fn search_location_coord(e: &mut Engine, this: u32, out: u32, actor: u32) -> u32 {
    let own = searches_own_location(e, this);
    let point = e.with_stack(12, |e, buffer| {
        let buffer = buffer.addr();
        let made = if own {
            typed_call!(FAMILY, 0x0067_5de0, location_coord(e, this, buffer, actor))
        } else {
            typed_call!(
                FAMILY,
                0x0067_5360,
                second_location_coord(e, this, buffer, actor)
            )
        };
        point_at(e, made)
    });
    store_point(e, out, point);
    out
}

// Typed from 006758a0
/// `TESPackage::GetSearchLocationRadius` (Xbox PDB): the own location's radius for `actor` or the
/// second location's, as [`search_location_cell`] picks.
pub fn search_location_radius(e: &mut Engine, this: u32, actor: u32) -> i32 {
    if searches_own_location(e, this) {
        typed_call!(FAMILY, 0x0067_6280, location_radius_for(e, this, actor))
    } else {
        second_radius(e, this)
    }
}

// Typed from 006759e0
/// `TESPackage::ComputeInitialTarget` (Xbox PDB): the target's reference for a target of type 0,
/// the actor's linked reference for type 3, else 0 (and 0 without a target).
pub fn compute_initial_target(e: &mut Engine, this: u32, actor: u32) -> u32 {
    let target = target_of(e, this);
    if target == 0 {
        return 0;
    }
    match target_type(e, target) {
        0 => target_word(e, target, 0),
        3 => linked_reference(e, actor),
        _ => 0,
    }
}

// Typed from 00675a50
/// `TESPackage::GetLocationWorld` (Xbox PDB): the own location's world space for `actor`: the
/// reference's (the actor's without one), the cell's, the actor's own (its slot), the chosen
/// reference's or the actor's, the linked reference's (else the actor's); no location or type 2 the
/// start location's, set from the current location first when the actor has none.
pub fn location_world(e: &mut Engine, this: u32, actor: u32) -> u32 {
    let location = e.mem.u32(at(this, LOCATION));
    if location == 0 || location_type(e, location) == 2 {
        if actor == 0 {
            return 0;
        }
        let first = e.call(START_LOCATION_WORLD, &args![actor]).u32();
        if first == 0 {
            e.call(SET_START_LOCATION, &args![actor]);
            return reference_world(e, actor);
        }
        return first;
    }
    match location_type(e, location) {
        0 => match location_word(e, location, 0) {
            0 if actor != 0 => reference_world(e, actor),
            0 => 0,
            reference => reference_world(e, reference),
        },
        1 => match location_word(e, location, 1) {
            0 => 0,
            cell => cell_world_space(e, cell),
        },
        3 if actor != 0 => e.vcall(actor, SLOT_ACTOR_WORLD_SPACE, &args![]).u32(),
        4 | 5 => match chosen_reference(e, this, actor, SLOT_PROCESS_FIRST) {
            Some(0) => reference_world(e, actor),
            Some(chosen) => reference_world(e, chosen),
            None => 0,
        },
        6 => {
            if actor == 0 {
                return 0;
            }
            if linked_reference(e, actor) == 0 {
                return reference_world(e, actor);
            }
            let linked = linked_reference(e, actor);
            reference_world(e, linked)
        }
        _ => 0,
    }
}

// Typed from 00675c20
/// `TESPackage::GetLocationCell` (Xbox PDB): as [`location_world`], for the cell.
pub fn location_cell(e: &mut Engine, this: u32, actor: u32) -> u32 {
    let location = e.mem.u32(at(this, LOCATION));
    if location == 0 || location_type(e, location) == 2 {
        if actor == 0 {
            return 0;
        }
        let first = e.call(START_LOCATION_CELL, &args![actor]).u32();
        if first == 0 {
            e.call(SET_START_LOCATION, &args![actor]);
            return parent_cell(e, actor);
        }
        return first;
    }
    match location_type(e, location) {
        0 => match location_word(e, location, 0) {
            0 if actor != 0 => parent_cell(e, actor),
            0 => 0,
            reference => parent_cell(e, reference),
        },
        1 => location_word(e, location, 1),
        3 if actor != 0 => e.vcall(actor, SLOT_ACTOR_CELL, &args![]).u32(),
        4 | 5 => match chosen_reference(e, this, actor, SLOT_PROCESS_FIRST) {
            Some(0) => parent_cell(e, actor),
            Some(chosen) => parent_cell(e, chosen),
            None => 0,
        },
        6 => {
            if actor == 0 {
                return 0;
            }
            match linked_reference(e, actor) {
                0 => parent_cell(e, actor),
                _ => {
                    let linked = linked_reference(e, actor);
                    parent_cell(e, linked)
                }
            }
        }
        _ => 0,
    }
}

// Typed from 00675de0
/// `TESPackage::GetLocationCoord` (Xbox PDB): the own location's position for `actor` written to
/// `out` (from the default point): the reference's (the actor's point after a warning without
/// one), the cell's first reference's or its grid corner, the actor's point, the chosen
/// reference's or the actor's, the linked reference's (the actor's point after a warning without
/// one); no location or type 2 the start location's (set from the current location first when the
/// actor's extra data has none). Returns `out`.
pub fn location_coord(e: &mut Engine, this: u32, out: u32, actor: u32) -> u32 {
    let location = e.mem.u32(at(this, LOCATION));
    let mut point = point_at(e, DEFAULT_POINT);
    if location == 0 || location_type(e, location) == 2 {
        if actor != 0 {
            let list = at(actor, EXTRA_DATA_LIST);
            let start = typed_call!(
                extra_data::FAMILY,
                0x0041_8b70,
                extra_data::access::package_start_location(e, list)
            );
            if start == 0 {
                e.call(SET_START_LOCATION, &args![actor]);
            }
            let address = e.call(START_LOCATION_COORD, &args![actor]).u32();
            point = point_at(e, address);
        }
        store_point(e, out, point);
        return out;
    }
    match location_type(e, location) {
        0 => match location_word(e, location, 0) {
            0 if actor != 0 => point = warn_and_actor_point(e, actor, TEXT_NO_REFERENCE),
            0 => {}
            reference => point = position(e, reference),
        },
        1 => {
            let cell = location_word(e, location, 1);
            if cell != 0 {
                if let Some(found) = cell_point(e, cell, location) {
                    point = found;
                }
            }
        }
        3 if actor != 0 => point = actor_point(e, actor),
        4 | 5 => {
            if let Some(chosen) = chosen_reference(e, this, actor, SLOT_PROCESS_FIRST) {
                let source = if chosen == 0 { actor } else { chosen };
                point = position(e, source);
            }
        }
        6 if actor != 0 => {
            if linked_reference(e, actor) == 0 {
                point = warn_and_actor_point(e, actor, TEXT_NO_LINKED_REFERENCE);
            } else {
                let linked = linked_reference(e, actor);
                point = position(e, linked);
            }
        }
        _ => {}
    }
    store_point(e, out, point);
    out
}

// Typed from 00676140
/// `TESPackage::GetLocationReference` (Xbox PDB): as [`second_location_reference`], for the own
/// location (the chosen reference from the first location's process slot).
pub fn location_reference(e: &mut Engine, this: u32, actor: u32) -> u32 {
    let location = e.mem.u32(at(this, LOCATION));
    if location == 0 || location_type(e, location) == 2 {
        return actor;
    }
    match location_type(e, location) {
        0 => location_word(e, location, 0),
        3 => actor,
        4 | 5 => match chosen_reference(e, this, actor, SLOT_PROCESS_FIRST) {
            Some(0) => actor,
            Some(chosen) => chosen,
            None => 0,
        },
        6 if actor != 0 && linked_reference(e, actor) != 0 => linked_reference(e, actor),
        _ => 0,
    }
}

// Typed from 00676280
/// The own location's radius for `actor`: 0 without a location; for a location that resolves to a
/// reference whose base form has the form type 0x15 and whose own radius is zero, half the
/// smallest side of the base form's bound box, truncated; otherwise the location's own radius.
pub fn location_radius_for(e: &mut Engine, this: u32, actor: u32) -> i32 {
    let location = e.mem.u32(at(this, LOCATION));
    if location == 0 {
        return 0;
    }
    let reference = e
        .call(LOCATION_RESOLVE_REFERENCE, &args![location, actor])
        .u32();
    if reference != 0 {
        let base = base_form(e, reference);
        if form_type(e, base) == 0x15 && location_radius(e, location) == 0 {
            return e.with_stack(12, |e, high| {
                e.with_stack(12, |e, low| {
                    let (high, low) = (high.addr(), low.addr());
                    typed_call!(
                        FAMILY,
                        0x0050_eb90,
                        object::bound_max_as_floats(e, base, high)
                    );
                    let low = typed_call!(
                        FAMILY,
                        0x0050_ead0,
                        object::bound_min_as_floats(e, base, low)
                    );
                    // `004578c0` in place: each side's x87 difference stored as a float.
                    let mut side = [0f32; 3];
                    for (k, slot) in (0u32..).zip(side.iter_mut()) {
                        let a = x87_load32(e.mem.u32(at(high, 4 * k)));
                        let b = x87_load32(e.mem.u32(at(low, 4 * k)));
                        let bits = x87_store32(x87_sub(a, b));
                        e.mem.set_u32(at(high, 4 * k), bits);
                        *slot = f32::from_bits(bits);
                    }
                    // `0040ebd0` twice in place, then the half stored as a float and chopped.
                    let smaller = x87_quiet(if side[1] > side[0] { side[0] } else { side[1] });
                    let smallest = x87_quiet(if side[2] > smaller { smaller } else { side[2] });
                    let half = x87_store32(engine_core::x87::x87_mul(f64::from(smallest), HALF));
                    // The low word of the 64-bit `FISTP`.
                    let whole = fistp_chop64(x87_load32(half)).to_le_bytes();
                    i32::from_le_bytes([whole[0], whole[1], whole[2], whole[3]])
                })
            });
        }
    }
    i32::from_ne_bytes(location_radius(e, location).to_ne_bytes())
}

/// The second location's radius done in place (`00675670`: the second location's read and its
/// radius, both one-line bodies).
fn second_radius(e: &Engine, this: u32) -> i32 {
    match held_second_location(e, this) {
        0 => 0,
        location => i32::from_ne_bytes(location_radius(e, location).to_ne_bytes()),
    }
}

/// The default radius setting.
fn default_radius(e: &Engine) -> i32 {
    i32::from_ne_bytes(setting(e, DEFAULT_RADIUS_SETTING).to_ne_bytes())
}

/// The radius the movement tests use for the reference a location stands for, and whether it is
/// exact: 20 and exact for a marker; for an actor 90 and exact for kind 9, else its model bound
/// plus 20; else its model bound plus 20.0 (0.0 for the base form types 0x15, 0x1b, 0x20 and
/// 0x21), never negative.
fn radius_for_reference(e: &mut Engine, reference: u32) -> (i32, bool) {
    if is_marker(e, reference) {
        return (0x14, true);
    }
    if is_actor(e, reference) {
        if kind_of(e, reference) == 9 {
            return (0x5a, true);
        }
        return (model_bound(e, reference).wrapping_add(0x14), false);
    }
    let base = base_form(e, reference);
    let margin = if matches!(form_type(e, base), 0x15 | 0x1b | 0x20 | 0x21) {
        0.0
    } else {
        MARGIN
    };
    let bound = model_bound(e, reference);
    let margin = truncate(e, f64::from(margin));
    (bound.wrapping_add(margin).max(0), false)
}

/// Whether the actor's process holds a point (its slot answers one).
fn process_has_point(e: &mut Engine, actor: u32) -> bool {
    let process = process_of(e, actor);
    process != 0 && e.vcall(process, SLOT_PROCESS_POINT, &args![]).u32() != 0
}

/// The actor's process's point copied over `point`.
fn copy_process_point(e: &mut Engine, actor: u32, point: u32) {
    let process = process_of(e, actor);
    let address = e.vcall(process, SLOT_PROCESS_POINT, &args![]).u32();
    let copy = point_at(e, address);
    store_point(e, point, copy);
}

/// `this - other` into `out` (the pathing family's difference).
fn difference(e: &mut Engine, this: u32, out: u32, other: u32) -> u32 {
    typed_call!(
        pathing::FAMILY,
        0x0043_9ef0,
        pathing::point::subtract(e, Ptr::new(this), Ptr::new(out), Ptr::new(other))
    )
    .addr()
}

/// A point's flat length (the pathing family's).
fn flat_length(e: &mut Engine, point: u32) -> f32 {
    typed_call!(
        pathing::FAMILY,
        0x0058_9850,
        pathing::point::length_2d(e, Ptr::new(point))
    )
}

/// The flat distance between `point` and the position of `subject`.
fn flat_distance_from(e: &mut Engine, point: u32, subject: u32) -> f32 {
    let subject_at = e.vcall(subject, SLOT_POSITION, &args![]).u32();
    e.with_stack(12, |e, buffer| {
        let buffer = buffer.addr();
        difference(e, point, buffer, subject_at);
        flat_length(e, buffer)
    })
}

/// The subject's reach test: whether it is within `radius` of `point`.
fn reaches(e: &mut Engine, subject: u32, point: u32, radius: i32, exact: bool, last: u32) -> bool {
    e.vcall(
        subject,
        SLOT_REACH,
        &args![point, radius as f32, u32::from(!exact), last],
    )
    .bool()
}

/// The place test the two "is at the location" functions start with.
enum Place {
    /// A type 1 location in the same cell: yes at once.
    Immediate,
    /// The same place; the subject's interior cell (0 when none).
    Same(u32),
    Different,
}

fn same_place(e: &mut Engine, location: u32, subject: u32, world: u32, cell: u32) -> Place {
    let cell = interior_or_none(e, cell);
    let subject_world = reference_world(e, subject);
    let subject_cell = interior_or_none(e, parent_cell(e, subject));
    let same = if subject_cell != 0 || cell != 0 {
        if subject_cell == cell && location != 0 && location_type(e, location) == 1 {
            return Place::Immediate;
        }
        subject_cell == cell
    } else {
        subject_world == world
    };
    if same {
        Place::Same(subject_cell)
    } else {
        Place::Different
    }
}

/// The location's own test (`0067f220`): `Some(answer)` when it decides.
fn location_decides(e: &mut Engine, location: u32, actor: u32, subject: u32) -> Option<bool> {
    let (decided, answer) = e.with_stack(4, |e, answer| {
        let decided = e
            .call(
                LOCATION_TEST_SUBJECT,
                &args![location, actor, subject, answer],
            )
            .bool();
        (decided, e.mem.u8(answer.addr()))
    });
    decided.then_some(answer != 0)
}

// Typed from 00676390
/// Whether `subject` stands at the package's second location for `actor`. No second location is
/// yes; a type 3 location is yes when the actor's 0x290 test says no; the subject must share the
/// location's world space or cell (a type 1 location is yes on a cell match); the location's own
/// test may decide; then the radius (the second location's, 0 with `ignore_extra`, else
/// `extra_radius` unless it is -1.0; a positive location radius drops `furniture_check`), from the
/// reference the location stands for when still 0, else the default radius setting, and the
/// subject's reach test decides. A type 5 package also needs the height difference to the second
/// location within 100; with `furniture_check` and a furniture reference whose subject's process
/// holds a point, the answer is whether the subject is an actor of kind 4 or 9 whose process's
/// 0x4c8 slot gives that reference.
pub fn subject_at_second_location(
    e: &mut Engine,
    this: u32,
    subject: u32,
    actor: u32,
    ignore_extra: u8,
    extra_radius: f32,
    furniture_check: u8,
) -> bool {
    e.with_stack(12, |e, point| {
        second_location_test(
            e,
            [this, subject, actor, point.addr()],
            ignore_extra,
            extra_radius,
            furniture_check,
        )
    })
}

fn second_location_test(
    e: &mut Engine,
    [this, subject, actor, point]: [u32; 4],
    ignore_extra: u8,
    extra_radius: f32,
    furniture_check: u8,
) -> bool {
    if subject == 0 {
        return false;
    }
    let mut furniture_check = furniture_check != 0;
    let subject_actor = if is_actor(e, subject) { subject } else { 0 };
    let location = held_second_location(e, this);
    if location == 0 {
        return true;
    }
    if location_type(e, location) == 3 && !e.vcall(actor, SLOT_ACTOR_290, &args![]).bool() {
        return true;
    }
    let world = typed_call!(FAMILY, 0x0067_4fd0, second_location_world(e, this, actor));
    let cell = typed_call!(FAMILY, 0x0067_51a0, second_location_cell(e, this, actor));
    let subject_cell = match same_place(e, location, subject, world, cell) {
        Place::Immediate => return true,
        Place::Same(cell) => cell,
        Place::Different => return false,
    };
    if let Some(answer) = location_decides(e, location, actor, subject) {
        return answer;
    }
    let mut radius = second_radius(e, this);
    if radius > 0 {
        furniture_check = false;
    }
    if ignore_extra != 0 {
        radius = 0;
    } else if f64::from(extra_radius) != NO_EXTRA_RADIUS {
        radius = truncate(e, f64::from(extra_radius));
    }
    let mut exact = false;
    typed_call!(
        FAMILY,
        0x0067_5360,
        second_location_coord(e, this, point, actor)
    );
    let reference = typed_call!(
        FAMILY,
        0x0067_56a0,
        second_location_reference(e, this, actor)
    );
    if radius == 0 {
        if reference != 0 {
            if is_furniture(e, reference) && process_has_point(e, subject) {
                let holder = process_of(e, actor);
                if e.mem.u32(at(holder, PROCESS_WORD_28)) == 0 {
                    copy_process_point(e, actor, point);
                }
                radius = 0x14;
                exact = true;
            } else {
                (radius, exact) = radius_for_reference(e, reference);
            }
        } else if location_type(e, location) == 3 {
            exact = true;
            radius = 10;
        }
    }
    if radius == 0 {
        radius = default_radius(e);
    }
    if pack_type(e, this) == 5 && subject_cell != 0 {
        let subject_at = e.vcall(subject, SLOT_POSITION, &args![]).u32();
        let second_height = e.with_stack(12, |e, buffer| {
            let made = typed_call!(
                FAMILY,
                0x0067_5360,
                second_location_coord(e, this, buffer.addr(), actor)
            );
            e.mem.u32(at(made, 8))
        });
        let height = x87_load32(e.mem.u32(at(subject_at, 8)));
        let height = f32::from_bits(x87_store32(x87_sub(height, x87_load32(second_height))));
        let length = typed_call!(pathing::FAMILY, 0x0040_8840, pathing::point::abs(e, height));
        if f64::from(length) > DISTANCE_LIMIT {
            return false;
        }
    }
    if furniture_check
        && reference != 0
        && is_furniture(e, reference)
        && process_has_point(e, subject)
    {
        // The exe works the flat distance out and does not use it.
        flat_distance_from(e, point, subject);
        if subject_actor == 0 {
            return false;
        }
        if kind_of(e, subject_actor) != 4 && kind_of(e, subject_actor) != 9 {
            return false;
        }
        let process = process_of(e, subject_actor);
        return e.vcall(process, SLOT_PROCESS_4C8, &args![]).u32() == reference;
    }
    reaches(e, subject, point, radius, exact, 1)
}

// Typed from 006768d0
/// Whether `subject` (an actor running this package) stands at the package's own location: as
/// [`subject_at_second_location`] for the own location with the subject as its own actor, but no
/// location is no automatic yes; a clear `furniture_check` is set for an actor of kind 4 or 9; the
/// location's own test runs only for a zero location radius; `skip_radius` zeroes the radius, else
/// `extra_radius` (unless -1.0) is added to it; a radius up to 20 and a marker reference make it
/// exact; the furniture case answers whether the subject is within 100 of the point and of kind
/// 4, or of kind 9.
pub fn subject_at_location(
    e: &mut Engine,
    this: u32,
    subject: u32,
    skip_radius: u8,
    extra_radius: f32,
    furniture_check: u8,
) -> bool {
    e.with_stack(12, |e, point| {
        own_location_test(
            e,
            [this, subject, point.addr()],
            skip_radius,
            extra_radius,
            furniture_check,
        )
    })
}

fn own_location_test(
    e: &mut Engine,
    [this, subject, point]: [u32; 3],
    skip_radius: u8,
    extra_radius: f32,
    furniture_check: u8,
) -> bool {
    if subject == 0 {
        return false;
    }
    let mut furniture_check = furniture_check != 0;
    let subject_actor = if is_actor(e, subject) { subject } else { 0 };
    if !furniture_check
        && subject_actor != 0
        && (kind_of(e, subject_actor) == 4 || kind_of(e, subject_actor) == 9)
    {
        furniture_check = true;
    }
    let location = e.mem.u32(at(this, LOCATION));
    if location != 0
        && location_type(e, location) == 3
        && !e.vcall(subject, SLOT_ACTOR_290, &args![]).bool()
    {
        return true;
    }
    let world = typed_call!(FAMILY, 0x0067_5a50, location_world(e, this, subject));
    let cell = typed_call!(FAMILY, 0x0067_5c20, location_cell(e, this, subject));
    match same_place(e, location, subject, world, cell) {
        Place::Immediate => return true,
        Place::Same(_) => {}
        Place::Different => return false,
    }
    if location != 0 && location_radius(e, location) == 0 {
        if let Some(answer) = location_decides(e, location, subject, subject) {
            return answer;
        }
    }
    let mut radius = typed_call!(FAMILY, 0x0067_6280, location_radius_for(e, this, subject));
    if skip_radius != 0 {
        radius = 0;
    } else if f64::from(extra_radius) != NO_EXTRA_RADIUS {
        radius = truncate(e, f64::from(extra_radius)).wrapping_add(radius);
    }
    if radius > 0 {
        furniture_check = false;
    }
    let mut exact = false;
    typed_call!(
        FAMILY,
        0x0067_5de0,
        location_coord(e, this, point, subject_actor)
    );
    let reference = if location == 0 {
        0
    } else {
        typed_call!(FAMILY, 0x0067_6140, location_reference(e, this, subject))
    };
    if radius == 0 && location != 0 {
        if reference != 0 {
            if is_furniture(e, reference) && process_has_point(e, subject) {
                let holder = process_of(e, subject);
                if e.mem.u32(at(holder, PROCESS_WORD_28)) == 0 {
                    copy_process_point(e, subject, point);
                }
                radius = 0x14;
                exact = true;
            } else {
                (radius, exact) = radius_for_reference(e, reference);
            }
        } else if location_type(e, location) == 3 {
            exact = true;
            radius = 10;
        }
    }
    let own = typed_call!(FAMILY, 0x0067_6280, location_radius_for(e, this, subject));
    if (own as u32) < 0x15 && reference != 0 && is_marker(e, reference) {
        exact = true;
    }
    if radius == 0 {
        radius = default_radius(e);
    }
    if furniture_check
        && reference != 0
        && is_furniture(e, reference)
        && process_has_point(e, subject)
    {
        let length = flat_distance_from(e, point, subject_actor);
        if f64::from(length) < DISTANCE_LIMIT && kind_of(e, subject) == 4 {
            return true;
        }
        return kind_of(e, subject) == 9;
    }
    reaches(e, subject, point, radius, exact, 1)
}

/// The reference the actor heads for (its slot), or 0.
fn heading_for(e: &mut Engine, actor: u32) -> u32 {
    e.vcall(actor, SLOT_TARGET_REFERENCE, &args![]).u32()
}

/// Whether two places match the way the "is at" tests compare them: the cells when either has an
/// interior one, else the world spaces.
fn same_cell_or_world(first: (u32, u32), second: (u32, u32)) -> bool {
    if first.1 != 0 || second.1 != 0 {
        first.1 == second.1
    } else {
        first.0 == second.0
    }
}

// Typed from 00676e40
/// `TESPackage::IsActorAtRefTarget` (Xbox PDB): whether `actor` has reached the reference it heads
/// for, with `extra` added to the radius. They must share a world space or cell; the radius is
/// `GetPackageRadiusActorToRefTarget`'s, or 10 for a furniture reference whose actor's process
/// holds a point (the goal then that point) or a form type 0x30 reference, 90 or 200 for some
/// actors of kind 9 or 4, the model bound plus 20 for other actors, 20 for a marker; the actor's
/// reach test decides (a furniture reference also passes within 100 for kind 4 or for kind 9).
/// The radius and distance are formatted for the error report when the actor is the interface's
/// picked reference.
pub fn actor_at_ref_target(e: &mut Engine, this: u32, actor: u32, extra: i32) -> bool {
    if actor == 0 || heading_for(e, actor) == 0 {
        return false;
    }
    e.with_stack(12, |e, point| {
        e.with_stack(12, |e, gap| {
            actor_at_target(e, [this, actor, point.addr(), gap.addr()], extra)
        })
    })
}

fn actor_at_target(e: &mut Engine, [this, actor, point, gap]: [u32; 4], extra: i32) -> bool {
    let actor_as_actor = if is_actor(e, actor) { actor } else { 0 };
    let mut exact = false;
    let target = heading_for(e, actor);
    let target_at = position(e, target);
    store_point(e, point, target_at);
    if e.call(HAS_LINKED_DOOR, &args![target]).u32() != 0 {
        let address = e.call(LINKED_DOOR_POSITION, &args![target]).u32();
        let door = point_at(e, address);
        store_point(e, point, door);
    }
    let target_world = reference_world(e, target);
    let target_cell = interior_or_none(e, parent_cell(e, target));
    let actor_world = reference_world(e, actor);
    let actor_cell = interior_or_none(e, parent_cell(e, actor));
    if !same_cell_or_world((actor_world, actor_cell), (target_world, target_cell)) {
        return false;
    }
    let radius = typed_call!(
        FAMILY,
        0x0067_87e0,
        radius_actor_to_ref_target(e, this, actor, 0)
    );
    let mut radius = truncate(e, f64::from(radius));
    let mut result = false;
    let actor_at = e.vcall(actor, SLOT_POSITION, &args![]).u32();
    difference(e, point, gap, actor_at);
    if is_furniture(e, target) && process_has_point(e, actor) {
        let holder = process_of(e, actor);
        if e.mem.u32(at(holder, PROCESS_WORD_28)) == 0 {
            copy_process_point(e, actor, point);
        }
        radius = 10;
        exact = true;
    } else if form_type(e, base_form(e, target)) == 0x30 {
        radius = 10;
        exact = true;
    } else if radius <= 0x14 && is_actor(e, target) && target != e.mem.u32(PLAYER) {
        if kind_of(e, target) == 9 {
            radius = 0x5a;
            exact = true;
        } else if kind_of(e, actor) == 4 {
            radius = 200;
            exact = true;
        } else {
            radius = model_bound(e, target).wrapping_add(0x14);
        }
    } else if is_marker(e, target) {
        radius = 0x14;
        exact = true;
    }
    radius = radius.wrapping_add(extra);
    let height = f32::from_bits(e.mem.u32(at(gap, 8)));
    let flat = typed_call!(pathing::FAMILY, 0x0040_8840, pathing::point::abs(e, height));
    let tolerance = f32::from_bits(setting(e, HEIGHT_TOLERANCE_SETTING));
    if f64::from(tolerance) > f64::from(flat) {
        e.mem.set_f32(at(gap, 8), 0.0);
    }
    if reaches(e, actor, point, radius, exact, 0) {
        result = true;
    }
    if target != 0 && is_furniture(e, target) && process_has_point(e, actor) {
        let length = flat_distance_from(e, point, actor);
        let close = f64::from(length) < DISTANCE_LIMIT && kind_of(e, actor_as_actor) == 4;
        if close || kind_of(e, actor_as_actor) == 9 {
            result = true;
        }
    }
    let picked = typed_call!(
        interface::FAMILY,
        0x0070_3180,
        interface::interface_cpp::interface_get_target_refr(e)
    );
    if picked == actor_as_actor {
        let length = flat_length(e, gap);
        // `_finite` in place.
        if f64::from(length).is_finite() {
            e.with_stack(308, |e, buffer| {
                e.call(
                    SPRINTF,
                    &args![buffer, TEXT_RADIUS_DISTANCE, radius, f64::from(length)],
                );
                typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words buffer);
            });
        }
    }
    result
}

/// The target-at-location tests' place check: the target's world space and interior cell against
/// the own location's for the actor.
fn target_shares_location(e: &mut Engine, this: u32, actor: u32, target: u32) -> bool {
    let target_world = reference_world(e, target);
    let target_cell = interior_or_none(e, parent_cell(e, target));
    let world = typed_call!(FAMILY, 0x0067_5a50, location_world(e, this, actor));
    let cell = typed_call!(FAMILY, 0x0067_5c20, location_cell(e, this, actor));
    let cell = interior_or_none(e, cell);
    same_cell_or_world((world, cell), (target_world, target_cell))
}

/// The flat distance from the location's position (`coord`: the own or second location's getter
/// into a buffer) to the target's position, truncated.
fn distance_to_target(e: &mut Engine, target: u32, coord: impl Fn(&mut Engine, u32) -> u32) -> i32 {
    let target_at = e.vcall(target, SLOT_POSITION, &args![]).u32();
    let length = e.with_stack(12, |e, buffer| {
        e.with_stack(12, |e, gap| {
            let made = coord(e, buffer.addr());
            difference(e, made, gap.addr(), target_at);
            flat_length(e, gap.addr())
        })
    });
    truncate(e, f64::from(length))
}

// Typed from 006773a0
/// `TESPackage::IsTargetAtLocation` (Xbox PDB): whether the reference `actor` heads for stands at
/// the own location: the same world space or cell, the location's own test, then the flat distance
/// truncated within the radius (the location's, or the model bound of the reference it stands
/// for, or the default radius setting when that is not above zero) plus `extra`.
pub fn target_at_location(e: &mut Engine, this: u32, actor: u32, extra: i32) -> bool {
    if actor == 0 || heading_for(e, actor) == 0 {
        return false;
    }
    let target = heading_for(e, actor);
    if !target_shares_location(e, this, actor, target) {
        return false;
    }
    let location = e.mem.u32(at(this, LOCATION));
    if location != 0 {
        if let Some(answer) = location_decides(e, location, actor, target) {
            return answer;
        }
    }
    let mut radius = typed_call!(FAMILY, 0x0067_6280, location_radius_for(e, this, actor));
    if radius == 0 && location != 0 {
        let reference = typed_call!(FAMILY, 0x0067_6140, location_reference(e, this, actor));
        if reference != 0 {
            radius = model_bound(e, reference);
        }
    }
    if radius <= 0 {
        radius = default_radius(e);
    }
    let radius = radius.wrapping_add(extra);
    let distance = distance_to_target(e, target, |e, buffer| {
        typed_call!(FAMILY, 0x0067_5de0, location_coord(e, this, buffer, actor))
    });
    distance <= radius
}

// Typed from 00677570
/// `TESPackage::IsTargetAtSecondLocation` (Xbox PDB): as [`target_at_location`] (the own location's
/// place check), with the second location's own test and position and a float radius: a negative
/// `radius` is worked out from the second location's radius, the model bound of the reference it
/// stands for or the default radius setting. True when the flat distance, truncated, is at most it.
pub fn target_at_second_location(e: &mut Engine, this: u32, actor: u32, radius: f32) -> bool {
    let target = heading_for(e, actor);
    if actor == 0 || target == 0 {
        return false;
    }
    if !target_shares_location(e, this, actor, target) {
        return false;
    }
    let location = held_second_location(e, this);
    if location != 0 {
        if let Some(answer) = location_decides(e, location, actor, target) {
            return answer;
        }
    }
    let mut radius = radius;
    if radius < 0.0 {
        let mut worked_out = second_radius(e, this);
        if worked_out == 0 && location != 0 {
            let reference = typed_call!(
                FAMILY,
                0x0067_56a0,
                second_location_reference(e, this, actor)
            );
            if reference != 0 {
                worked_out = model_bound(e, reference);
            }
        }
        if worked_out <= 0 {
            worked_out = default_radius(e);
        }
        radius = worked_out as f32;
    }
    let distance = distance_to_target(e, target, |e, buffer| {
        typed_call!(
            FAMILY,
            0x0067_5360,
            second_location_coord(e, this, buffer, actor)
        )
    });
    f64::from(radius) >= f64::from(distance)
}

// Typed from 00678670
/// `TESPackage::GetPackageRadiusActorToLocation` (Xbox PDB): the radius within which `actor` is at
/// the location (the second location when `second` is set, else the own one), as a float: the
/// location's own radius, else from the reference it resolves to 10 (furniture), 90 (an actor of
/// kind 9), 20 (a marker) or the model bound plus 20, never negative; a type 3 location without a
/// reference 10; zero becomes the default radius setting. 0 without that location.
pub fn radius_actor_to_location(e: &mut Engine, this: u32, actor: u32, second: u8) -> f32 {
    let location = if second != 0 {
        held_second_location(e, this)
    } else {
        e.mem.u32(at(this, LOCATION))
    };
    if location == 0 {
        return 0.0;
    }
    let mut radius = i32::from_ne_bytes(location_radius(e, location).to_ne_bytes());
    if radius == 0 {
        let reference = e
            .call(LOCATION_RESOLVE_REFERENCE, &args![location, actor])
            .u32();
        if reference != 0 {
            radius = if is_furniture(e, reference) {
                10
            } else if is_actor(e, reference) {
                if kind_of(e, reference) == 9 {
                    0x5a
                } else {
                    model_bound(e, reference).wrapping_add(0x14)
                }
            } else if is_marker(e, reference) {
                0x14
            } else {
                model_bound(e, reference).wrapping_add(0x14)
            };
            radius = radius.max(0);
        } else if location_type(e, location) == 3 {
            radius = 10;
        }
    }
    if radius == 0 {
        radius = default_radius(e);
    }
    radius as f32
}

// Typed from 006787e0
/// `TESPackage::GetPackageRadiusActorToRefTarget` (Xbox PDB): the radius within which `actor` is at
/// the reference it heads for, as a float: 10 for furniture, 20 for a marker; else the target's
/// value for a type 0x1c package whose actor's package extra has a target, or for a package with a
/// target whose procedure type is not 0x1a; when still 0 and the reference is no actor, its model
/// bound, plus the actor's reach adjustment for base form types other than 0x15, 0x1b, 0x20 and
/// 0x21; zero becomes the default radius setting. (The location `second` picks is read and not
/// used: one-line bodies, so nothing is done.)
pub fn radius_actor_to_ref_target(e: &mut Engine, this: u32, actor: u32, _second: u8) -> f32 {
    let mut radius: i32 = 0;
    let actor_as_actor = if is_actor(e, actor) { actor } else { 0 };
    let target = heading_for(e, actor);
    if target != 0 {
        if is_furniture(e, target) {
            radius = 10;
        } else if is_marker(e, target) {
            radius = 0x14;
        } else {
            let list = at(actor_as_actor, EXTRA_DATA_LIST);
            let extra = typed_call!(
                extra_data::FAMILY,
                0x0041_cb10,
                extra_data::packages::package(e, list)
            );
            let extra_target = if extra != 0 { target_of(e, extra) } else { 0 };
            if pack_type(e, this) == 0x1c && extra != 0 && extra_target != 0 {
                radius = e.mem.i32(at(extra_target, 8));
            } else {
                let own = target_of(e, this);
                if own != 0 && e.mem.i32(at(this, PROCEDURE)) != 0x1a {
                    radius = e.mem.i32(at(own, 8));
                }
            }
            if radius == 0 && !is_actor(e, target) {
                radius = model_bound(e, target);
                let kind = form_type(e, base_form(e, target));
                if !matches!(kind, 0x15 | 0x1b | 0x20 | 0x21) && actor_as_actor != 0 {
                    let added = reach_adjustment(e, actor, actor_as_actor, target);
                    radius = radius.wrapping_add(added);
                }
            }
        }
    }
    if radius == 0 {
        radius = default_radius(e);
    }
    radius as f32
}

/// An extent's half height (the actor's extent slot writes a box into a buffer; its z over 2.0).
fn half_extent(e: &mut Engine, actor: u32) -> f64 {
    let z = e.with_stack(12, |e, buffer| {
        let address = e.vcall(actor, SLOT_EXTENT, &args![buffer]).u32();
        e.mem.u32(at(address, 8))
    });
    x87_div(x87_load32(z), TWO)
}

/// The reach adjustment of `GetPackageRadiusActorToRefTarget`: the actor's reach height
/// truncated, plus, when its eye line (its position's z plus its eye level less the reach height,
/// stored as a float) is above the target's z by less than half its extent's height, that half
/// truncated. The exe's x87 maths, each value stored where the exe stores it.
fn reach_adjustment(e: &mut Engine, actor: u32, actor_as_actor: u32, target: u32) -> i32 {
    let reach = e.vcall(actor_as_actor, SLOT_REACH_HEIGHT, &args![]).f32();
    let mut added = truncate(e, f64::from(reach));
    let eye = f64::from(e.call(EYE_LEVEL, &args![actor_as_actor]).f32());
    let actor_at = e.vcall(actor_as_actor, SLOT_POSITION, &args![]).u32();
    let z = x87_load32(e.mem.u32(at(actor_at, 8)));
    let eye_line = f32::from_bits(x87_store32(x87_sub(
        x87_add(z, eye),
        x87_load32(reach.to_bits()),
    )));
    let target_at = e.vcall(target, SLOT_POSITION, &args![]).u32();
    let target_z = f32::from_bits(e.mem.u32(at(target_at, 8)));
    if target_z < eye_line {
        let gap = x87_sub(f64::from(eye_line), f64::from(target_z));
        if half_extent(e, actor) > gap {
            let half = half_extent(e, actor);
            added = added.wrapping_add(truncate(e, half));
        }
    }
    added
}

// Typed from 00678ec0
/// `TESPackage::IsTargetAnActor` (Xbox PDB): whether the target stands for an actor: type 0 a
/// reference that is an actor, type 1 an object of form type 0x2a or 0x2b, type 2 the object types
/// 0xe and 0xf, type 3 the actor's linked reference when it is an actor.
pub fn target_is_actor(e: &mut Engine, this: u32, actor: u32) -> bool {
    let target = target_of(e, this);
    if target == 0 {
        return false;
    }
    match target_type(e, target) {
        0 => match target_word(e, target, 0) {
            0 => false,
            held => is_actor(e, held),
        },
        1 => {
            let object = target_word(e, target, 1);
            matches!(form_type(e, object), 0x2a | 0x2b)
        }
        2 => (0xe..=0xf).contains(&i32::from_ne_bytes(target_word(e, target, 2).to_ne_bytes())),
        3 if linked_reference(e, actor) != 0 => {
            let held = linked_reference(e, actor);
            is_actor(e, held)
        }
        _ => false,
    }
}

/// The persistent reference of a reference with the form flag 0x20: the extra data list's pointer
/// extra (the list's address, `005d43c0`, is never null for a reference).
fn refine_reference(e: &mut Engine, reference: u32) -> u32 {
    if reference != 0 && e.mem.u32(at(reference, FORM_FLAGS)) & FORM_FLAG_20 != 0 {
        let list = at(reference, EXTRA_DATA_LIST);
        if list != 0 {
            return typed_call!(
                extra_data::FAMILY,
                0x0041_c8d0,
                extra_data::cell_data::reference_pointer(e, list)
            );
        }
    }
    reference
}

/// The data handler's walk of the references near the actor (its parent cell, its position twice,
/// each with the follow radius setting) with the follow candidate callback and `context`; the
/// first reference the callback took, read from its global and cleared.
fn find_close_reference(e: &mut Engine, actor: u32, context: u32) -> u32 {
    let radius_first = setting(e, FOLLOW_RADIUS_SETTING);
    let position_first = e.vcall(actor, SLOT_POSITION, &args![]).u32();
    let radius_second = setting(e, FOLLOW_RADIUS_SETTING);
    let position_second = e.vcall(actor, SLOT_POSITION, &args![]).u32();
    let cell = parent_cell(e, actor);
    let handler = e.mem.u32(DATA_HANDLER);
    e.call(
        NEARBY_REFERENCES,
        &args![
            handler,
            cell,
            position_second,
            radius_second,
            position_first,
            radius_first,
            FOLLOW_CANDIDATE,
            context
        ],
    );
    let found = e.mem.u32(FOUND_REFERENCE);
    e.mem.set_u32(FOUND_REFERENCE, 0);
    found
}

// Typed from 006780e0
/// The reference the package's target stands for, for `actor`, registering a new follower: target
/// type 0 its reference and 3 the actor's linked reference (each the persistent reference for a
/// form with the flag 0x20), 1 the player when the target object is the player's base form, else
/// the first nearby reference of that object the callback takes, 2 the first nearby reference the
/// callback takes. For an actor and a package not "created": a type 1 or 7 package adds the
/// reference as the actor's follower, the player unless its process's 0x52c reference is the player,
/// the player's follower test holds or its count is over the limit setting (then the actor's
/// process is told and the message setting shown); a type 2 package adds the actor as a follower
/// of a reference other than the player. The callback's context for type 2 (and for type 1 without
/// an object) has its first word unwritten, as in the exe.
pub fn follow_target(e: &mut Engine, this: u32, actor: u32, flag: u8) -> u32 {
    let target = target_of(e, this);
    if target == 0 {
        return 0;
    }
    let result = match target_type(e, target) {
        0 => {
            let held = target_word(e, target, 0);
            refine_reference(e, held)
        }
        3 => {
            let linked = linked_reference(e, actor);
            refine_reference(e, linked)
        }
        1 => {
            let mut object = 0;
            let held = target_word(e, target, 1);
            if held != 0 && e.vcall(held, SLOT_FORM_E4, &args![]).bool() {
                object = target_word(e, target, 1);
            }
            let player = e.mem.u32(PLAYER);
            if player != 0 && object == base_form(e, player) {
                player
            } else {
                e.with_stack(12, |e, context| {
                    let context = context.addr();
                    e.mem.set_u32(context, object);
                    e.mem.set_u32(at(context, 4), actor);
                    e.mem.set_u8(at(context, 8), flag);
                    find_close_reference(e, actor, context)
                })
            }
        }
        2 => e.with_stack(12, |e, context| {
            let context = context.addr();
            e.mem.set_u32(at(context, 4), actor);
            e.mem.set_u8(at(context, 8), flag);
            find_close_reference(e, actor, context)
        }),
        _ => 0,
    };
    // `00674d40` in place: the flag 0x800 (created).
    if result != 0 && is_actor(e, result) && e.mem.u32(at(this, 0x1c)) & 0x800 == 0 {
        let player = e.mem.u32(PLAYER);
        let kind = pack_type(e, this);
        if kind == 1 || kind == 7 {
            if result == player {
                let process = process_of(e, actor);
                let chosen = e.vcall(process, SLOT_PROCESS_52C, &args![]).u32();
                if chosen != player && !e.call(PLAYER_FOLLOWER_TEST, &args![player, actor]).bool() {
                    let count = e.call(PLAYER_FOLLOWER_COUNT, &args![player]).i32();
                    let limit =
                        i32::from_ne_bytes(setting(e, FOLLOWER_LIMIT_SETTING).to_ne_bytes());
                    if count > limit {
                        let process = process_of(e, actor);
                        e.vcall(process, SLOT_PROCESS_REFUSE, &args![actor, u32::MAX]);
                        let message = setting(e, MESSAGE_SETTING);
                        e.call(
                            SHOW_MESSAGE,
                            &args![message, 0u32, 0u32, 0u32, MESSAGE_TIME, 0u32],
                        );
                        return result;
                    }
                }
            }
            e.call(ADD_FOLLOWER, &args![result, actor]);
        } else if kind == 2 && result != player {
            e.call(ADD_FOLLOWER, &args![actor, result]);
        }
    }
    result
}

// Typed from 006784f0
/// The follow search's callback (`cdecl(reference, context)`, the context {form, actor, flag}):
/// takes `reference` (into the found reference's global) when the form answers its 0xe4 test, the
/// reference is of that form, it has no form flag 0x20 and, for an actor, it is not the searching
/// actor, its 0x22c test does not refuse it and, with the flag, it runs no type 1 package and the
/// actor's candidate test does not refuse it.
pub fn follow_candidate(e: &mut Engine, reference: u32, context: u32) -> bool {
    if context == 0 {
        return false;
    }
    let form = e.mem.u32(context);
    let actor = e.mem.u32(at(context, 4));
    let flag = e.mem.u8(at(context, 8));
    if !e.vcall(form, SLOT_FORM_E4, &args![]).bool()
        || !e.call(REFERENCE_OF_FORM, &args![reference, form]).bool()
        || e.mem.u32(at(reference, FORM_FLAGS)) & FORM_FLAG_20 != 0
    {
        return false;
    }
    if is_actor(e, reference) {
        if reference == actor || e.vcall(reference, SLOT_REJECT, &args![1u32]).bool() {
            return false;
        }
        if flag != 0 {
            let package = typed_call!(
                ai_queries::FAMILY,
                0x0093_44a0,
                ai_queries::queries::current_package(e, reference)
            );
            if package != 0 && pack_type(e, package) == 1 {
                return false;
            }
            if e.call(REJECTS_CANDIDATE, &args![actor, reference]).bool() {
                return false;
            }
        }
    }
    e.mem.set_u32(FOUND_REFERENCE, reference);
    true
}
