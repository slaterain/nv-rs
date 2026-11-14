//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): references' interaction state and data. 12 of
//! its 104 commands: the getters of the player's activation a script disabled, which the core
//! track's `SetInteractionDisabled` and `SetInteractionDisabledType` set (`world_hooks.rs`: the
//! form's `jipFormFlags6` bit, the form types in `ui_hooks.rs`' `activation_disabled_types`), and
//! the reference data commands that touch neither the scene graph nor physics: whether a form is
//! an item, a reference's form type, the player's teammates, enable and linked children, a door's
//! teleport door, the purge-on-unload flag, the primitive and its bounds, the crosshair's water.
//!
//! The rest wait for their own batches: most read or change a reference's scene graph and
//! physics (JIP's node lookup `GetNode2`, `NiAVObject` blocks, Havok bodies); the map marker
//! commands need JIP's map marker hook, the extra data ones (`RefHasExtraData` and the two
//! after it) JIP's own `ExtraJIPData` and its co-save record, the move commands JIP's position
//! marker.

use super::Handlers;
use crate::game::JipFormFlags;
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27c8, get_interaction_disabled),
    (0x27ca, get_interaction_disabled_type),
    (0x262d, get_is_item),
    (0x2635, get_enable_children),
    (0x2636, get_linked_children),
    (0x2679, get_primitive_bound),
    (0x26ce, get_teammates),
    (0x26f1, has_primitive),
    (0x2775, get_ref_type),
    (0x27f9, get_teleport_door),
    (0x2813, get_crosshair_water),
    (0x2931, toggle_purge_on_unload),
];

/// `kHookFormFlag6_NoPCActivation` (`internal/hooks.h` (JIP LN NVSE), the bit `world_hooks.rs`
/// sets).
const NO_PC_ACTIVATION: u8 = 1 << 1;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetInteractionDisabled_Execute.
/// 1 when the form (or the calling reference) is one the player may not activate.
fn get_interaction_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(0);
    if form == 0 {
        form = a.this_obj;
    }
    if form != 0
        && e.get(Ptr::<JipFormFlags>::new(form), JipFormFlags::jipFormFlags6) & NO_PC_ACTIVATION
            != 0
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetInteractionDisabledType_Execute.
/// 1 when the form type (below `kFormType_Max`, one of the filter types) is one the player may
/// not activate.
fn get_interaction_disabled_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let kind = read.int(0, 0) as usize;
    if read.ok
        && crate::ln_events::VALID_FILTER_FORM
            .get(kind)
            .copied()
            .unwrap_or(false)
        && e.ext::<Jip>()
            .ui_hooks
            .activation_disabled_types
            .get(kind)
            .copied()
            .unwrap_or(false)
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// ---- a reference's data: its kind, children, teleport door, purge flag, primitive (batch 12) ----

/// `TESForm::flags` (+8: 0x400 persistent, 0x400000 purge on unload), `modIndex` (+0xF, 0xFF for a
/// form made in the game), `TESObjectREFR::baseForm` (+0x20).
const FORM_FLAGS: u32 = 0x08;
const MOD_INDEX: u32 = 0x0f;
const BASE: u32 = 0x20;
const PERSISTENT: u32 = 0x400;
const PURGE_ON_UNLOAD: u32 = 0x40_0000;
/// The extra data types (`kXData_*`) and the word at +0xC each holds: `ExtraTeleport` (0x2B, its
/// data, whose linked door is at +0), `ExtraEnableStateChildren` (0x38, its list inline at +0xC:
/// the exe's `0041DCA0`), `ExtraLinkedRefChildren` (0x52, the same: `0041E500`), `ExtraPrimitive`
/// (0x6B, its `BGSPrimitive`).
const EXTRA_TELEPORT: u32 = 0x2b;
const EXTRA_ENABLE_CHILDREN: u32 = 0x38;
const EXTRA_LINKED_CHILDREN: u32 = 0x52;
const EXTRA_PRIMITIVE: u32 = 0x6b;
const EXTRA_WORD: u32 = 0x0c;
/// `BGSPrimitive::bounds` (+0x18, three floats).
const PRIMITIVE_BOUNDS: u32 = 0x18;
/// `kFormType_TESObjectDOOR`, `kFormType_BGSPlaceableWater` (its water +0x4C).
const DOOR: u8 = 0x1c;
const PLACEABLE_WATER: u8 = 0x23;
const WATER: u32 = 0x4c;
/// `g_thePlayer` and its teammates (`PlayerCharacter::teammates`, +0x5FC: the exe removes a
/// teammate from that list through `00905330`).
const PLAYER: u32 = 0x011d_ea3c;
const TEAMMATES: u32 = 0x5fc;
/// `g_interfaceManager` and its crosshair reference (+0xFC: `InterfaceManager`'s constructor
/// `0070A130` clears it; `game.rs`).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const CROSSHAIR_REF: u32 = 0xfc;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetIsItem_Execute.
/// 1 for an inventory type (`kInventoryType`): the form given (the reader takes only inventory
/// objects), else the calling reference's base form.
fn get_is_item(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut form = read.form(0);
    if form == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        form = e.mem.u32(a.this_obj + BASE);
    }
    // JIP reads the base form's type unchecked; the port answers 0 for a reference with none.
    let item = form != 0
        && super::forms::INVENTORY_TYPE
            .get(usize::from(super::type_of(e, form)))
            .copied()
            .unwrap_or(false);
    a.set_result(e, if item { 1.0 } else { 0.0 });
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetRefType_Execute.
/// The reference's (given, or the calling one's) form type, echoed in the console.
fn get_ref_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut refr = read.form(0);
    if refr == 0 {
        refr = a.this_obj;
    }
    if refr != 0 {
        let t = super::type_of(e, refr);
        a.set_result(e, f64::from(t));
        crate::console::print_result(e, &a);
    }
    handled(true)
}

/// A list's non-empty items as array elements; the array (its id the result) only when there is
/// one.
fn array_of_list(e: &mut Engine, a: &CommandArgs, head: u32) {
    let items: Vec<super::arrays::Element> =
        super::list_items(e, Ptr::<super::forms::ListNode>::new(head))
            .into_iter()
            .filter(|&f| f != 0)
            .map(super::arrays::Element::Form)
            .collect();
    if !items.is_empty() {
        let id = super::arrays::create_array(e, a.script, &items);
        a.set_result(e, f64::from(id));
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetTeammates_Execute.
fn get_teammates(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let player = e.mem.u32(PLAYER);
    if player != 0 {
        array_of_list(e, &a, player + TEAMMATES);
    }
    handled(true)
}

/// GetEnableChildren and GetLinkedChildren: the extra's children list as an array.
fn children(e: &mut Engine, w: &[u32], kind: u32) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let x = super::extra::refr_extra(e, a.this_obj, kind);
    if x != 0 {
        array_of_list(e, &a, x + EXTRA_WORD);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetEnableChildren_Execute.
fn get_enable_children(e: &mut Engine, w: &[u32]) -> Ret {
    children(e, w, EXTRA_ENABLE_CHILDREN)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetLinkedChildren_Execute.
fn get_linked_children(e: &mut Engine, w: &[u32]) -> Ret {
    children(e, w, EXTRA_LINKED_CHILDREN)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetTeleportDoor_Execute.
/// A door reference's linked door (its `ExtraTeleport` data's first word).
fn get_teleport_door(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let base = e.mem.u32(a.this_obj + BASE);
    if base == 0 || super::type_of(e, base) != DOOR {
        return handled(true);
    }
    let x = super::extra::refr_extra(e, a.this_obj, EXTRA_TELEPORT);
    if x == 0 {
        return handled(true);
    }
    let data = e.mem.u32(x + EXTRA_WORD);
    if data == 0 {
        return handled(true);
    }
    let door = e.mem.u32(data);
    if door != 0 {
        super::set_form_result(e, &a, door);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_TogglePurgeOnUnload_Execute.
/// The reference's purge-on-unload flag (bit 22) as the result; with an argument whose value is
/// not the flag's (0 or 1 compared with the whole value: 2 differs from either), a reference made
/// in the game (mod index 0xFF) that is not persistent has the flag flipped and is marked
/// changed (`MarkModified(1)`).
fn toggle_purge_on_unload(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let flags = e.mem.u32(a.this_obj + FORM_FLAGS);
    let current = u32::from(flags & PURGE_ON_UNLOAD != 0);
    a.set_result(e, f64::from(current));
    if super::jip_fn_utility::num_args(e, &a) == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if read.ok
        && current != read.int(0, 0)
        && e.mem.u8(a.this_obj + MOD_INDEX) == 0xff
        && flags & PERSISTENT == 0
    {
        e.mem
            .set_u32(a.this_obj + FORM_FLAGS, flags ^ PURGE_ON_UNLOAD);
        super::extra::mark_modified(e, a.this_obj, 1);
    }
    handled(true)
}

/// The reference's `BGSPrimitive`, 0 for none.
fn primitive(e: &mut Engine, refr: u32) -> u32 {
    let x = super::extra::refr_extra(e, refr, EXTRA_PRIMITIVE);
    if x == 0 {
        0
    } else {
        e.mem.u32(x + EXTRA_WORD)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_HasPrimitive_Execute.
fn has_primitive(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj != 0 && primitive(e, a.this_obj) != 0 {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetPrimitiveBound_Execute.
/// -1, or twice the primitive's half-size on the axis (a float times 2.0, in doubles).
///
/// JIP indexes the bounds by `axis - 'X'` unchecked; the compiler only writes X, Y or Z, and the
/// port leaves -1 for any other byte rather than read past the three floats.
fn get_primitive_bound(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let axis = match read.args.first() {
        Some(nvse::args::Arg::Byte(b)) => b.wrapping_sub(b'X'),
        _ => return handled(true),
    };
    if axis > 2 {
        return handled(true);
    }
    let p = primitive(e, a.this_obj);
    if p != 0 {
        let half = e.mem.f32(p + PRIMITIVE_BOUNDS + 4 * u32::from(axis));
        a.set_result(e, f64::from(half) * 2.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetCrosshairWater_Execute.
/// The water of the placeable water under the crosshair; echoed in the console (the form, or 0).
fn get_crosshair_water(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let manager = e.mem.u32(INTERFACE_MANAGER);
    let refr = if manager == 0 {
        0
    } else {
        e.mem.u32(manager + CROSSHAIR_REF)
    };
    let mut water = 0;
    if refr != 0 {
        let base = e.mem.u32(refr + BASE);
        if base != 0 && super::type_of(e, base) == PLACEABLE_WATER {
            water = e.mem.u32(base + WATER);
        }
    }
    if water != 0 {
        super::set_form_result(e, &a, water);
    }
    crate::console::print_form(e, &a, water);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_tests.rs"]
mod tests;
