//! xNVSE's `Commands_MiscRef.cpp` (xNVSE), first batch: a reference's base form, doors and where
//! they lead, locks' keys, owners and the rank they ask for, and the form and actor base flag
//! words. (`GetRefCount` and `SetRefCount` of the same file are in `inventory_forms.rs`.)

use super::{console_print, dynamic_cast, is_console_mode, rtti, Read};
use crate::invref::{self, get_by_type};
use crate::table::{guest_str, Command, Param, Returns};
use crate::{handled, CommandArgs};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_Integer`, `_Float`, `_ObjectID`,
/// `_ObjectRef`, `_ActorBase`, `_AnyForm`.
const INT: u32 = 0x01;
const FLOAT: u32 = 0x02;
const OBJECT_ID: u32 = 0x03;
const OBJECT_REF: u32 = 0x04;
const ACTOR_BASE: u32 = 0x19;
const ANY_FORM: u32 = 0x3d;
/// `kParams_OneInt`, `kParams_OneObjectID`, `kParams_SetDoorTeleport`,
/// `kParamType_OneOptionalActorBase`, `kParamType_OneInt_OneOptionalActorBase`,
/// `kParamType_OneOptionalAnyForm`, `kParamType_OneInt_OneOptionalAnyForm` (xNVSE).
const ONE_INT: &[Param] = &[p("int", INT, false)];
const ONE_OBJECT_ID: &[Param] = &[p("item", OBJECT_ID, false)];
const SET_DOOR_TELEPORT: &[Param] = &[
    p("linkedDoor", OBJECT_REF, false),
    p("x", FLOAT, true),
    p("y", FLOAT, true),
    p("z", FLOAT, true),
    p("rot", FLOAT, true),
];
const OPTIONAL_ACTOR_BASE: &[Param] = &[p("actor base", ACTOR_BASE, true)];
const INT_OPTIONAL_ACTOR_BASE: &[Param] =
    &[p("flags", INT, false), p("actor base", ACTOR_BASE, true)];
const OPTIONAL_ANY_FORM: &[Param] = &[p("actor base", ANY_FORM, true)];
const INT_OPTIONAL_ANY_FORM: &[Param] = &[p("flags", INT, false), p("actor base", ANY_FORM, true)];

/// `TESForm`: `typeID` (`+4`), `flags` (`+8`, `kFormFlags_QuestItem` 0x400 the persistent bit of a
/// reference), `refID` (`+0xC`). `TESObjectREFR`: `baseForm` (`+0x20`), `rotZ` (`+0x2C`), `posX`
/// (`+0x30`), `parentCell` (`+0x40`), `extraDataList` (`+0x44`). `TESObjectCELL`: `extraDataList`
/// (`+0x28`), `worldSpace` (`+0xC0`).
const FORM_TYPE: u32 = 4;
const FORM_FLAGS: u32 = 8;
const FORM_ID: u32 = 0xc;
const PERSISTENT: u32 = 0x400;
const REFR_BASE_FORM: u32 = 0x20;
const REFR_ROT_Z: u32 = 0x2c;
const REFR_POS: u32 = 0x30;
const REFR_PARENT_CELL: u32 = 0x40;
const REFR_EXTRA: u32 = invref::REFR_EXTRA;
const CELL_EXTRA: u32 = 0x28;
const CELL_WORLD: u32 = 0xc0;
/// `kFormType_TESObjectDOOR`.
const TYPE_DOOR: u8 = 0x1c;
/// `TESActorBase::baseData.flags` (`TESActorBaseData` at `+0x30`, its flags at `+4`).
const ACTOR_BASE_FLAGS: u32 = 0x34;
/// `BGSPlaceableWater::water` (`+0x4C`).
const PLACEABLE_WATER: u32 = 0x4c;
/// Extra data types (xNVSE `GameExtraData.h`) and their words: `ExtraPersistentCell` (`+0xC` the
/// cell), `ExtraOwnership` (`+0xC` the owner), `ExtraRank` (`+0xC`), `ExtraLock` (`+0xC` its data:
/// the key at `+4`), `ExtraTeleport` (`+0xC` its data: linked door, x, y, z, xRot, yRot, zRot),
/// `ExtraLeveledCreature` (`+0xC` the base form).
const EXTRA_PERSISTENT_CELL: u32 = 0x0c;
const EXTRA_OWNERSHIP: u32 = 0x21;
const EXTRA_RANK: u32 = 0x23;
const EXTRA_LOCK: u32 = 0x2a;
const EXTRA_TELEPORT: u32 = 0x2b;
const EXTRA_LEVELED_CREATURE: u32 = 0x2e;
const EXTRA_RANDOM_TELEPORT_MARKER: u32 = 0x3b;
const EXTRA_VALUE: u32 = 0xc;
const LOCK_KEY: u32 = 4;
const TELEPORT_X: u32 = 4;
const TELEPORT_X_ROT: u32 = 0x10;
const TELEPORT_Y_ROT: u32 = 0x14;
const TELEPORT_Z_ROT: u32 = 0x18;
/// `ExtraLock`, `ExtraTeleport` and `ExtraRank` (0x10 bytes, `GameExtraData.cpp` (xNVSE)'s
/// vtables) and their data (`ExtraLock::Data` 0x14 bytes zeroed, `ExtraTeleport::Data` 0x20).
const EXTRA_SIZE: u32 = 0x10;
const EXTRA_LOCK_VTBL: u32 = 0x0101_589c;
const EXTRA_TELEPORT_VTBL: u32 = 0x0101_58a8;
const EXTRA_RANK_VTBL: u32 = 0x0101_58cc;
const LOCK_DATA_SIZE: u32 = 0x14;
const TELEPORT_DATA_SIZE: u32 = 0x20;
/// `FormHeap_Allocate`.
const FORM_HEAP_ALLOCATE: u32 = 0x0040_1000;
/// `kRadToDegree`, `kDegreeToRad` (`Commands_MiscRef.cpp` (xNVSE), `float`s).
const RAD_TO_DEGREE: f32 = 57.295_78;
const DEGREE_TO_RAD: f32 = 0.017_453_292;

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    needs_ref: bool,
    params: &'static [Param],
    execute: AbiFn,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref,
        params,
        execute,
        eval: None,
        returns: Returns::Default,
    }
}

const fn form_cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    needs_ref: bool,
    params: &'static [Param],
    execute: AbiFn,
) -> Command {
    Command {
        returns: Returns::Form,
        ..cmd(opcode, name, short, help, needs_ref, params, execute)
    }
}

pub static COMMANDS: &[Command] = &[
    form_cmd(
        0x1403,
        "GetBaseObject",
        "gbo",
        "returns the base object id of the reference",
        true,
        &[],
        get_base_object,
    ),
    cmd(
        0x146c,
        "IsPersistent",
        "",
        "returns true if the calling reference is persistent",
        true,
        &[],
        is_persistent,
    ),
    form_cmd(
        0x146d,
        "GetParentCell",
        "gpc",
        "returns the parent cell of the reference",
        true,
        &[],
        get_parent_cell,
    ),
    form_cmd(
        0x146e,
        "GetParentWorldspace",
        "gpw",
        "returns the parent worldspace of the reference",
        true,
        &[],
        get_parent_worldspace,
    ),
    form_cmd(
        0x146f,
        "GetTeleportCell",
        "",
        "returns the cell to which the calling door reference teleports",
        true,
        &[],
        get_teleport_cell,
    ),
    form_cmd(
        0x1470,
        "GetLinkedDoor",
        "",
        "returns the door to which the calling reference is linked",
        true,
        &[],
        get_linked_door,
    ),
    form_cmd(
        0x14c8,
        "GetOpenKey",
        "GetKey",
        "returns the key associated with a lockable object",
        true,
        &[],
        get_open_key,
    ),
    form_cmd(
        0x14d7,
        "GetOwner",
        "",
        "returns the owner of the calling reference",
        true,
        &[],
        get_owner,
    ),
    form_cmd(
        0x14df,
        "GetParentCellOwner",
        "",
        "returns the owner of the cell of the calling reference",
        true,
        &[],
        get_parent_cell_owner,
    ),
    cmd(
        0x14e0,
        "GetOwningFactionRequiredRank",
        "",
        "returns the required rank for ownership of the calling reference",
        true,
        &[],
        get_owning_faction_required_rank,
    ),
    cmd(
        0x14e1,
        "GetParentCellOwningFactionRequiredRank",
        "",
        "returns the required rank for ownership of the cell of the calling reference",
        true,
        &[],
        get_parent_cell_owning_faction_required_rank,
    ),
    cmd(
        0x14ec,
        "GetActorBaseFlagsLow",
        "",
        "returns a bitfield containing actor base flags",
        false,
        OPTIONAL_ACTOR_BASE,
        get_actor_base_flags_low,
    ),
    cmd(
        0x14ed,
        "SetActorBaseFlagsLow",
        "",
        "sets actor base flags",
        false,
        INT_OPTIONAL_ACTOR_BASE,
        set_actor_base_flags_low,
    ),
    cmd(
        0x14ee,
        "GetActorBaseFlagsHigh",
        "",
        "returns a bitfield containing actor base flags",
        false,
        OPTIONAL_ACTOR_BASE,
        get_actor_base_flags_high,
    ),
    cmd(
        0x14ef,
        "SetActorBaseFlagsHigh",
        "",
        "sets actor base flags",
        false,
        INT_OPTIONAL_ACTOR_BASE,
        set_actor_base_flags_high,
    ),
    cmd(
        0x14f9,
        "SetOwningFactionRequiredRank",
        "",
        "sets the required rank for ownership of the calling reference",
        true,
        ONE_INT,
        set_owning_faction_required_rank,
    ),
    form_cmd(
        0x1510,
        "GetBaseForm",
        "gbf",
        "returns the permanent base object id of the reference",
        true,
        &[],
        get_base_form,
    ),
    cmd(
        0x1512,
        "SetOpenKey",
        "SetKey",
        "sets the key used to unlock the calling object",
        true,
        ONE_OBJECT_ID,
        set_open_key,
    ),
    cmd(
        0x151b,
        "ClearOpenKey",
        "ClearKey",
        "clears the key used to unlock the calling object",
        true,
        &[],
        clear_open_key,
    ),
    cmd(
        0x15f0,
        "GetFlagsLow",
        "",
        "returns a bitfield containing object flags",
        false,
        OPTIONAL_ANY_FORM,
        get_flags_low,
    ),
    cmd(
        0x15f1,
        "SetFlagsLow",
        "",
        "sets actor flags",
        false,
        INT_OPTIONAL_ANY_FORM,
        set_flags_low,
    ),
    cmd(
        0x15f2,
        "GetFlagsHigh",
        "",
        "returns a bitfield containing actor flags",
        false,
        OPTIONAL_ANY_FORM,
        get_flags_high,
    ),
    cmd(
        0x15f3,
        "SetFlagsHigh",
        "",
        "sets actor base flags",
        false,
        INT_OPTIONAL_ANY_FORM,
        set_flags_high,
    ),
    cmd(
        0x15f9,
        "IsLoadDoor",
        "",
        "\"returns 1 if the calling reference is a load door\"",
        true,
        &[],
        is_load_door,
    ),
    cmd(
        0x15fa,
        "GetDoorTeleportX",
        "",
        "returns x-coord to which the door teleports",
        true,
        &[],
        get_door_teleport_x,
    ),
    cmd(
        0x15fb,
        "GetDoorTeleportY",
        "",
        "returns y-coord to which the door teleports",
        true,
        &[],
        get_door_teleport_y,
    ),
    cmd(
        0x15fc,
        "GetDoorTeleportZ",
        "",
        "returns z-coord to which the door teleports",
        true,
        &[],
        get_door_teleport_z,
    ),
    cmd(
        0x15fd,
        "GetDoorTeleportRot",
        "",
        "returns z rotation to which the door teleports",
        true,
        &[],
        get_door_teleport_rot,
    ),
    cmd(
        0x15fe,
        "SetDoorTeleport",
        "",
        "sets the linked door and coordinates to which the door teleports",
        false,
        SET_DOOR_TELEPORT,
        set_door_teleport,
    ),
];

fn form_id(e: &Engine, form: u32) -> u32 {
    e.mem.u32(form + FORM_ID)
}

/// Whether a pointer xNVSE reads or writes through is missing and the script reference checks row
/// (`NV_NVSE_INVREF_CHECK`, a reference handed to a script with parts missing) answers instead: a
/// reference's base form or parent cell (a script reaches both with an inventory reference: its
/// temporary reference is in no cell, and has no base form once released, after `RemoveMeIR` or a
/// loop's reference kept past its loop), or a door's teleport data, linked door or lock data (a
/// mod's broken door data; the game makes a lock without data only when an allocation fails, in
/// `TESObjectREFR::AddLock` `005690A0`, and its loader never makes a teleport without data). xNVSE reads
/// through the null pointer and the game stops; without the row the port stops there too. With it
/// the command gives its "nothing" answer and writes nothing.
pub(super) fn passed_over(e: &mut Engine, pointer: u32) -> bool {
    pointer == 0 && invref::checks(e)
}

/// `GetFullName(TESForm *)` (xNVSE `GameForms.cpp`), a guest string for the console.
fn full_name_text(e: &mut Engine, form: u32) -> u32 {
    let name = super::string::tes_full_name(e, form);
    if name != 0 {
        let (data, len) = (e.mem.u32(name + 4), e.mem.u16(name + 8));
        if data != 0 && len != 0 {
            return data;
        }
    }
    guest_str(e, b"<no name>")
}

// ---- base forms ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetBaseObject_Execute.
fn get_base_object(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if a.this_obj != 0 {
        let base = e.mem.u32(a.this_obj + REFR_BASE_FORM);
        if base != 0 {
            let id = form_id(e, base);
            a.set_form_result(e, id);
            if is_console_mode(e) {
                let name = full_name_text(e, base);
                console_print(e, "GetBaseObject >> %08x (%s)", &[id, name]);
            }
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameObjects.cpp: GetPermanentBaseForm.
/// A reference's base form; for one made in game (file 0xFF), a placed water's water or a
/// leveled creature's real base form.
fn permanent_base_form(e: &mut Engine, reference: u32) -> u32 {
    if reference == 0 {
        return 0;
    }
    let base = e.mem.u32(reference + REFR_BASE_FORM);
    if base != 0 && form_id(e, base) >> 24 == 0xff {
        let water = dynamic_cast(e, base, rtti::TES_FORM, rtti::BGS_PLACEABLE_WATER);
        if water != 0 {
            return e.mem.u32(water + PLACEABLE_WATER);
        }
        let leveled = get_by_type(e, reference + REFR_EXTRA, EXTRA_LEVELED_CREATURE);
        if leveled != 0 && e.mem.u32(leveled + EXTRA_VALUE) != 0 {
            return e.mem.u32(leveled + EXTRA_VALUE);
        }
    }
    base
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetBaseForm_Execute.
fn get_base_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    let base = permanent_base_form(e, a.this_obj);
    if base != 0 {
        let id = form_id(e, base);
        a.set_form_result(e, id);
        if is_console_mode(e) {
            let name = full_name_text(e, base);
            console_print(e, "GetBaseForm >> %08x (%s)", &[id, name]);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_IsPersistent_Execute.
fn is_persistent(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj != 0 {
        let v = e.mem.u32(a.this_obj + FORM_FLAGS) & PERSISTENT != 0;
        a.set_result(e, f64::from(u8::from(v)));
    }
    handled(true)
}

// ---- cells ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetParentCell_Execute.
fn get_parent_cell(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if a.this_obj != 0 {
        let cell = e.mem.u32(a.this_obj + REFR_PARENT_CELL);
        if cell != 0 {
            let id = form_id(e, cell);
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetParentWorldspace_Execute.
/// The worldspace of the reference's persistent cell, else of its parent cell; 0 for an interior.
fn get_parent_worldspace(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let persistent = get_by_type(e, a.this_obj + REFR_EXTRA, EXTRA_PERSISTENT_CELL);
    let cell = if persistent != 0 {
        e.mem.u32(persistent + EXTRA_VALUE)
    } else {
        0
    };
    let world = if cell != 0 {
        e.mem.u32(cell + CELL_WORLD)
    } else {
        0
    };
    if world != 0 {
        let id = form_id(e, world);
        a.set_form_result(e, id);
    } else {
        let parent = e.mem.u32(a.this_obj + REFR_PARENT_CELL);
        if parent != 0 && e.mem.u32(parent + CELL_WORLD) != 0 {
            let id = form_id(e, e.mem.u32(parent + CELL_WORLD));
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

// ---- doors ----

/// The reference's `ExtraTeleport` data, 0 without one (or without data).
fn teleport_data(e: &mut Engine, reference: u32) -> u32 {
    let tele = get_by_type(e, reference + REFR_EXTRA, EXTRA_TELEPORT);
    if tele == 0 {
        0
    } else {
        e.mem.u32(tele + EXTRA_VALUE)
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetLinkedDoor_Execute.
/// The linked door's id. An `ExtraTeleport` without data or without a linked door (no command
/// makes one, but a mod's broken door data can) is read through, as in xNVSE, unless
/// [`passed_over`]: then 0.
fn get_linked_door(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let tele = get_by_type(e, a.this_obj + REFR_EXTRA, EXTRA_TELEPORT);
    if tele != 0 {
        let data = e.mem.u32(tele + EXTRA_VALUE);
        if passed_over(e, data) {
            return handled(true);
        }
        let door = e.mem.u32(data);
        if passed_over(e, door) {
            return handled(true);
        }
        let id = form_id(e, door);
        a.set_form_result(e, id);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetTeleportCell_Execute.
/// The linked door's parent cell, 0 when that cell is not loaded.
fn get_teleport_cell(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let data = teleport_data(e, a.this_obj);
    if data != 0 {
        let door = e.mem.u32(data);
        if door != 0 {
            let cell = e.mem.u32(door + REFR_PARENT_CELL);
            if cell != 0 {
                let id = form_id(e, cell);
                a.set_form_result(e, id);
            }
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_IsLoadDoor_Execute.
fn is_load_door(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let list = a.this_obj + REFR_EXTRA;
    if get_by_type(e, list, EXTRA_TELEPORT) != 0
        || get_by_type(e, list, EXTRA_RANDOM_TELEPORT_MARKER) != 0
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: GetTeleportInfo.
/// Where a door teleports: x, y, z, or the z rotation in degrees (`float`s, the rotation times
/// `kRadToDegree` in `float`); 0 for a reference that is not a door or has no teleport data. A
/// reference without a base form is read through, as in xNVSE, unless [`passed_over`].
fn teleport_info(e: &mut Engine, w: &[u32], which: u32) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let base = e.mem.u32(a.this_obj + REFR_BASE_FORM);
    if passed_over(e, base) || e.mem.u8(base + FORM_TYPE) != TYPE_DOOR {
        return handled(true);
    }
    let data = teleport_data(e, a.this_obj);
    if data != 0 {
        let v = if which == 3 {
            e.mem.f32(data + TELEPORT_Z_ROT) * RAD_TO_DEGREE
        } else {
            e.mem.f32(data + TELEPORT_X + 4 * which)
        };
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

fn get_door_teleport_x(e: &mut Engine, w: &[u32]) -> Ret {
    teleport_info(e, w, 0)
}
fn get_door_teleport_y(e: &mut Engine, w: &[u32]) -> Ret {
    teleport_info(e, w, 1)
}
fn get_door_teleport_z(e: &mut Engine, w: &[u32]) -> Ret {
    teleport_info(e, w, 2)
}
fn get_door_teleport_rot(e: &mut Engine, w: &[u32]) -> Ret {
    teleport_info(e, w, 3)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetDoorTeleport_Execute; GameExtraData.cpp: ExtraTeleport::Create.
/// Links a door (not a random teleport marker) to a persistent reference: its `ExtraTeleport`
/// (made when missing) gets the door and where to put the one who goes through: the coordinates
/// given, else (all three left at 999) the linked reference's position, and the rotation given in
/// degrees, else the linked reference's. 1 when set. A reference without a base form is read
/// through, as in xNVSE, unless [`passed_over`].
fn set_door_teleport(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let base = e.mem.u32(a.this_obj + REFR_BASE_FORM);
    if passed_over(e, base) || e.mem.u8(base + FORM_TYPE) != TYPE_DOOR {
        return handled(true);
    }
    let list = a.this_obj + REFR_EXTRA;
    if get_by_type(e, list, EXTRA_RANDOM_TELEPORT_MARKER) != 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let door = read.form(0);
    if !read.ok || door == 0 || e.mem.u32(door + FORM_FLAGS) & PERSISTENT == 0 {
        return handled(true);
    }
    let (mut x, mut y, mut z, mut rot) = (
        read.float(1, 999.0),
        read.float(2, 999.0),
        read.float(3, 999.0),
        read.float(4, 999.0),
    );
    let mut tele = get_by_type(e, list, EXTRA_TELEPORT);
    if tele == 0 {
        tele = invref::create_extra(e, EXTRA_TELEPORT, EXTRA_SIZE, EXTRA_TELEPORT_VTBL);
        let data = e.call(FORM_HEAP_ALLOCATE, &args![TELEPORT_DATA_SIZE]).u32();
        e.mem.set_u32(data, 0);
        e.mem.set_f32(data + TELEPORT_Y_ROT, -0.0);
        e.mem.set_f32(data + TELEPORT_X_ROT, 0.0);
        for at in [TELEPORT_X, TELEPORT_X + 4, TELEPORT_X + 8, TELEPORT_Z_ROT] {
            e.mem.set_f32(data + at, 0.0);
        }
        e.mem.set_u32(tele + EXTRA_VALUE, data);
        invref::extra_add(e, list, tele);
    }
    let data = e.mem.u32(tele + EXTRA_VALUE);
    if passed_over(e, data) {
        return handled(true);
    }
    e.mem.set_u32(data, door);
    if x == 999.0 && y == 999.0 && z == 999.0 {
        x = e.mem.f32(door + REFR_POS);
        y = e.mem.f32(door + REFR_POS + 4);
        z = e.mem.f32(door + REFR_POS + 8);
    }
    if rot == 999.0 {
        rot = e.mem.f32(door + REFR_ROT_Z);
    } else {
        rot *= DEGREE_TO_RAD;
    }
    e.mem.set_f32(data + TELEPORT_X, x);
    e.mem.set_f32(data + TELEPORT_X + 4, y);
    e.mem.set_f32(data + TELEPORT_X + 8, z);
    e.mem.set_f32(data + TELEPORT_Z_ROT, rot);
    a.set_result(e, 1.0);
    handled(true)
}

// ---- locks ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetOpenKey_Execute.
fn get_open_key(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let lock = get_by_type(e, a.this_obj + REFR_EXTRA, EXTRA_LOCK);
    if lock != 0 {
        let data = e.mem.u32(lock + EXTRA_VALUE);
        if data != 0 && e.mem.u32(data + LOCK_KEY) != 0 {
            let id = form_id(e, e.mem.u32(data + LOCK_KEY));
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetOpenKey_Execute; GameExtraData.cpp: ExtraLock::Create.
/// The key of the calling object's lock (an `ExtraLock` with zeroed data made when missing); 1
/// when set. The read is not checked: a key it did not reach is uninitialised in the C++, none
/// here.
fn set_open_key(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let form = read.form(0);
    if form == 0 {
        return handled(true);
    }
    let key = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_KEY);
    if key == 0 {
        return handled(true);
    }
    let list = a.this_obj + REFR_EXTRA;
    let mut lock = get_by_type(e, list, EXTRA_LOCK);
    if lock == 0 {
        lock = invref::create_extra(e, EXTRA_LOCK, EXTRA_SIZE, EXTRA_LOCK_VTBL);
        let data = e.call(FORM_HEAP_ALLOCATE, &args![LOCK_DATA_SIZE]).u32();
        e.mem.write(data, &[0u8; LOCK_DATA_SIZE as usize]);
        e.mem.set_u32(lock + EXTRA_VALUE, data);
        invref::extra_add(e, list, lock);
    }
    if lock != 0 {
        let data = e.mem.u32(lock + EXTRA_VALUE);
        if passed_over(e, data) {
            return handled(true);
        }
        e.mem.set_u32(data + LOCK_KEY, key);
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_ClearOpenKey_Execute.
/// The calling object's `ExtraLock` removed (and freed): the lock goes with the key. 1 when it had
/// one.
fn clear_open_key(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let list = a.this_obj + REFR_EXTRA;
    let lock = get_by_type(e, list, EXTRA_LOCK);
    if lock != 0 {
        invref::extra_remove(e, list, lock, true);
        a.set_result(e, 1.0);
    }
    handled(true)
}

// ---- owners ----

/// `GetOwner(BaseExtraList &)` (xNVSE): the list's `ExtraOwnership` owner, or 0.
pub(super) fn owner_of(e: &mut Engine, list: u32) -> u32 {
    let owner = get_by_type(e, list, EXTRA_OWNERSHIP);
    if owner == 0 {
        0
    } else {
        e.mem.u32(owner + EXTRA_VALUE)
    }
}

/// `GetOwningFactionRequiredRank(BaseExtraList &)` (xNVSE): the list's `ExtraRank`, or 0.
pub(super) fn rank_of(e: &mut Engine, list: u32) -> u32 {
    let rank = get_by_type(e, list, EXTRA_RANK);
    if rank == 0 {
        0
    } else {
        e.mem.u32(rank + EXTRA_VALUE)
    }
}

/// The extra data list of the reference's parent cell; a reference without a cell is read
/// through, as in xNVSE, unless [`passed_over`] (`None`: the command answers 0).
fn parent_cell_list(e: &mut Engine, reference: u32) -> Option<u32> {
    let cell = e.mem.u32(reference + REFR_PARENT_CELL);
    if passed_over(e, cell) {
        return None;
    }
    Some(cell.wrapping_add(CELL_EXTRA))
}

fn owner_result(e: &mut Engine, a: &CommandArgs, list: u32) {
    let owner = owner_of(e, list);
    if owner != 0 {
        let id = form_id(e, owner);
        a.set_form_result(e, id);
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetOwner_Execute.
fn get_owner(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if a.this_obj != 0 {
        owner_result(e, &a, a.this_obj + REFR_EXTRA);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetParentCellOwner_Execute.
fn get_parent_cell_owner(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    if a.this_obj != 0 {
        if let Some(list) = parent_cell_list(e, a.this_obj) {
            owner_result(e, &a, list);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetOwningFactionRequiredRank_Execute.
fn get_owning_faction_required_rank(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj != 0 {
        let rank = rank_of(e, a.this_obj + REFR_EXTRA);
        a.set_result(e, f64::from(rank));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetParentCellOwningFactionRequiredRank_Execute.
fn get_parent_cell_owning_faction_required_rank(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj != 0 {
        if let Some(list) = parent_cell_list(e, a.this_obj) {
            let rank = rank_of(e, list);
            a.set_result(e, f64::from(rank));
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetOwningFactionRequiredRank_Execute, SetOwningFactionRequiredRank.
/// A new `ExtraRank` of the rank added to the reference (another one than any it has, as xNVSE
/// adds without looking); 0 adds nothing. The rank is the result. The game's list puts an
/// `ExtraRank` at its end (`BaseExtraList::AddExtra`, `0040ff60`, puts only five types in front),
/// so a reference that already has one goes on answering the first: in the real game a rank of 2,
/// then one of 3, reads back 2 (run `inv6r1`).
fn set_owning_faction_required_rank(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if read.ok {
        let rank = read.int(0, 0);
        if rank != 0 {
            let extra = invref::create_extra(e, EXTRA_RANK, EXTRA_SIZE, EXTRA_RANK_VTBL);
            if extra != 0 {
                e.mem.set_u32(extra + EXTRA_VALUE, rank);
                invref::extra_add(e, a.this_obj + REFR_EXTRA, extra);
            }
        }
        a.set_result(e, f64::from(rank));
    }
    handled(true)
}

// ---- flag words ----

/// The actor base the flag commands read: the argument, else the calling reference's base form
/// as an actor base; `None` when the read failed.
fn actor_base(e: &mut Engine, a: &CommandArgs, read: &Read, at: usize) -> Option<u32> {
    if !read.ok {
        return None;
    }
    let mut base = read.form(at);
    if base == 0 && a.this_obj != 0 {
        let form = e.mem.u32(a.this_obj + REFR_BASE_FORM);
        if form != 0 {
            base = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_ACTOR_BASE);
        }
    }
    Some(base)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetActorBaseFlagsLow_Execute, Cmd_GetActorBaseFlagsHigh_Execute.
fn get_actor_base_flags(e: &mut Engine, w: &[u32], high: bool) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if let Some(base) = actor_base(e, &a, &read, 0).filter(|&b| b != 0) {
        let flags = e.mem.u32(base + ACTOR_BASE_FLAGS);
        let v = if high { flags >> 16 } else { flags & 0xffff };
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetActorBaseFlagsLow_Execute, Cmd_SetActorBaseFlagsHigh_Execute.
fn set_actor_base_flags(e: &mut Engine, w: &[u32], high: bool) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let data = read.int(0, 0);
    if let Some(base) = actor_base(e, &a, &read, 1).filter(|&b| b != 0) {
        let flags = e.mem.u32(base + ACTOR_BASE_FLAGS);
        let v = if high {
            ((data << 16) & 0xffff_0000) | (flags & 0xffff)
        } else {
            (data & 0xffff) | (flags & 0xffff_0000)
        };
        e.mem.set_u32(base + ACTOR_BASE_FLAGS, v);
    }
    handled(true)
}

fn get_actor_base_flags_low(e: &mut Engine, w: &[u32]) -> Ret {
    get_actor_base_flags(e, w, false)
}
fn get_actor_base_flags_high(e: &mut Engine, w: &[u32]) -> Ret {
    get_actor_base_flags(e, w, true)
}
fn set_actor_base_flags_low(e: &mut Engine, w: &[u32]) -> Ret {
    set_actor_base_flags(e, w, false)
}
fn set_actor_base_flags_high(e: &mut Engine, w: &[u32]) -> Ret {
    set_actor_base_flags(e, w, true)
}

/// The form the form flag commands read: the argument, else the calling reference itself.
fn flags_form(a: &CommandArgs, read: &Read, at: usize) -> Option<u32> {
    if !read.ok {
        return None;
    }
    let form = read.form(at);
    let form = if form == 0 { a.this_obj } else { form };
    (form != 0).then_some(form)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetFlagsLow_Execute, Cmd_GetFlagsHigh_Execute.
/// A form's flag word, low or high half: the given form's, else the calling reference's own.
fn get_flags(e: &mut Engine, w: &[u32], high: bool) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if let Some(form) = flags_form(&a, &read, 0) {
        let flags = e.mem.u32(form + FORM_FLAGS);
        let v = if high { flags >> 16 } else { flags & 0xffff };
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

fn get_flags_low(e: &mut Engine, w: &[u32]) -> Ret {
    get_flags(e, w, false)
}
fn get_flags_high(e: &mut Engine, w: &[u32]) -> Ret {
    get_flags(e, w, true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetFlagsLow_Execute.
/// The low half of the given form's flag word, else the calling reference's own.
fn set_flags_low(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let data = read.int(0, 0);
    if let Some(form) = flags_form(&a, &read, 1) {
        let flags = e.mem.u32(form + FORM_FLAGS);
        e.mem
            .set_u32(form + FORM_FLAGS, (data & 0xffff) | (flags & 0xffff_0000));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetFlagsHigh_Execute.
/// The high half of the given form's flag word; without one, unlike `SetFlagsLow`, the calling
/// reference's base form's (when it is an actor base), as xNVSE writes it.
fn set_flags_high(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let data = read.int(0, 0);
    if let Some(form) = actor_base(e, &a, &read, 1).filter(|&f| f != 0) {
        let flags = e.mem.u32(form + FORM_FLAGS);
        e.mem.set_u32(
            form + FORM_FLAGS,
            ((data << 16) & 0xffff_0000) | (flags & 0xffff),
        );
    }
    handled(true)
}

#[cfg(test)]
#[path = "misc_ref_tests.rs"]
mod tests;
