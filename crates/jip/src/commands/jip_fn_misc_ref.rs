//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): references' interaction state and data. 22 of
//! its 104 commands here (the core track built five more: `SetInteractionDisabled`,
//! `SetInteractionDisabledType`, `SetRefName`, `SetRefrModelPath` in `world_hooks.rs` and
//! `SetOnCriticalHitEventHandler` in `actor_hooks.rs`): the getters of the player's activation a
//! script disabled (the form's `jipFormFlags6` bit, the form types in `ui_hooks.rs`'
//! `activation_disabled_types`), and the reference commands that change neither the scene graph
//! nor physics: whether a form is an item, a reference's form type, the player's teammates,
//! enable and linked children, a door's teleport door, the purge-on-unload and persistent flags,
//! the primitive (its bounds read and set, a new one added: the 3D reloaded through the game),
//! the crosshair's water, the game's own moves (`MoveToFadeDelay`, `MoveToContainer`), the playing
//! animation groups, and the position, angles, turned offsets and editor place into script
//! variables (a root node's world transform only read).
//!
//! Not built, each waiting for what it names:
//! - JIP's node lookup and scene graph work (`GetNifBlock`/`NiNode::GetBlock`/`GetNode2`, node
//!   transforms written, `ResetCollision`, controllers, lights, cameras, lines, textures; not
//!   built): `GetObjectDimensions` and `GetCenterPos` (JIP's `GetBoundingBox` walks a node's extra
//!   data), `SetPosEx`, `SetAngleEx`, `RotateAroundPoint` (JIP's `SetPos`/`SetAngle`/`Rotate`
//!   write the root node and reset its collision), the 18 `*NifBlock*`, `GetChildBlocks`,
//!   `GetBlockTextureSet`, `GetNifBlockParentNodes`, `GetMaterialPropertyValue`,
//!   `SetMaterialPropertyValue`, `SetTextureTransformKey`, `Get/SetAnimSequenceFrequency`,
//!   `PlayAnimSequence`, `AttachLight`, `RemoveLight`, `AttachLine`, `AttachExtraCamera`,
//!   `ProjectExtraCamera`, `ModelHasBlock`, `InsertNode`, `AttachModel` (JIP's insert/attach maps
//!   and model hooks), `MoveToNode`.
//! - Havok (rigid bodies, phantoms, contacts, character controllers, ray casts):
//!   `IsMobile`, `IsGrabbable` (JIP's `NiNode::IsMovable` walks collision objects),
//!   `GetRigidBodyMass`, `PushObject`, `GetObjectVelocity`, `Get/SetAngularVelocity`,
//!   `GetAngularVelocityAlt`, `GetLinearVelocityAlt`, `SetLinearVelocity`, `SetAngularVelocityEx`,
//!   `Get/SetCollisionObjProperty`, `Get/SetCollisionObjLayerType`, `ToggleNodeCollision`,
//!   `GetCollisionNodes`, `GetHasContact`, `GetHasContactBase`, `GetHasContactType`,
//!   `GetContactRefs`, `GetHasPhantom`, `GetRayCastPos`, `GetRayCastRef`, `GetRayCastMaterial`,
//!   `MoveToReticle`, `GetCrosshairRefEx` (the game's pick `00631D60` casts a Havok ray along the
//!   camera node).
//! - JIP's position marker `s_tempPosMarker` (made in its deferred start, not built):
//!   `MoveToCell`, `MoveToEditorPosition` (both through JIP's `MoveToCell`), `PlaceAtCell`.
//! - Hooks or code patches (steps 3.5/3.9): `AddRefMapMarker`, `RemoveRefMapMarker`,
//!   `RefHasMapMarker` (JIP's map marker hook), `ToggleObjectCollision` (the 3D load hook that
//!   keeps the flag's collision off), `ToggleNoZPosReset` (patches the game's code at `00629006`
//!   and `006295E9`), `SynchronizePosition` (the main loop hook reading `s_syncPosition*`).
//! - The co-save's application (step 3.3) and JIP's own extra data class: `SetLinkedReference`
//!   (its defaults map and the load's `RestoreLinkedRefs`/`SetLinkedRefID` are not built, so a
//!   load would keep the links unapplied), `RefHasExtraData`, `GetRefExtraData`,
//!   `SetRefExtraData` (`ExtraJIP`, an extra data type with JIP's own vtable and destructor).
//! - `GetPlayerPerks` calls `jip_fn_actor`'s `GetAllPerks` (lane A's module, not built yet).

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
    // lane E
    (0x2600, set_persistent),
    (0x267a, set_primitive_bound),
    (0x26f2, add_primitive),
    (0x280e, move_to_fade_delay),
    (0x283f, is_anim_playing_ex),
    (0x284a, move_to_container),
    (0x28f8, get_pos_ex),
    (0x28f9, get_angle_ex),
    (0x2909, get_translated_pos),
    (0x292a, get_editor_position),
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

// ---- lane E: persistence, primitives, moves, animations, positions and angles ----

/// `TESObjectREFR::SetRefPersists` (`00565480`, thiscall (reference, bool)): the exe's own
/// switch of the persistent flag (0x400), moving the reference into or out of its worldspace's
/// persistent references.
const SET_REF_PERSISTS: u32 = 0x0056_5480;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetPersistent_Execute.
/// The game's own switch, only when the argument (zero or not) differs from the flag.
fn set_persistent(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let set = read.int(0, 0) != 0;
    let persistent = e.mem.u32(a.this_obj + FORM_FLAGS) & PERSISTENT != 0;
    if set != persistent {
        e.call(SET_REF_PERSISTS, &args![a.this_obj, u32::from(set)]);
    }
    handled(true)
}

/// `BGSPrimitive` (the exe's constructors `004A4AC0` base, `004A6370` box, `004A7150` sphere,
/// `004A5280` plane): its type at +4 (1 box, 2 sphere, 3 plane), its half-sizes at +0x18. The
/// game allocates a box in 0x4C bytes and the other two in 0x34 (`004A5040`, `004A5100`,
/// `004A4F80`); their vtables `0101E8C4`, `0101EA64`, `0101E75C`.
const PRIMITIVE_TYPE: u32 = 0x04;
const VTBL_PRIMITIVE_BOX: u32 = 0x0101_e8c4;
const VTBL_PRIMITIVE_SPHERE: u32 = 0x0101_ea64;
const VTBL_PRIMITIVE_PLANE: u32 = 0x0101_e75c;
/// `ExtraPrimitive`'s vtable (its constructor `0041FAF0` sets it with type 0x6B).
const VTBL_EXTRA_PRIMITIVE: u32 = 0x0101_51b4;
/// `kFormType_TESObjectACTI`, and the base form id AddPrimitive also takes (0x21).
const ACTIVATOR: u8 = 0x15;
const PRIMITIVE_BASE_ID: u32 = 0x21;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetPrimitiveBound_Execute.
/// -1, or the old size on the axis (twice the half-size, in doubles); the new half-size (the
/// float times 0.5) goes to that axis, or to all three of a sphere; then the 3D is reloaded
/// (`Update3D`).
///
/// JIP indexes the bounds by `axis - 'X'` unchecked; the port leaves -1 and changes nothing for a
/// byte other than X, Y or Z (one the compiler does not write).
fn set_primitive_bound(e: &mut Engine, w: &[u32]) -> Ret {
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
    if p == 0 {
        return handled(true);
    }
    let val = read.float(1, 0.0) * 0.5;
    let at = p + PRIMITIVE_BOUNDS + 4 * u32::from(axis);
    a.set_result(e, f64::from(e.mem.f32(at)) * 2.0);
    if e.mem.u32(p + PRIMITIVE_TYPE) == 2 {
        for i in 0..3 {
            e.mem.set_f32(p + PRIMITIVE_BOUNDS + 4 * i, val);
        }
    } else {
        e.mem.set_f32(at, val);
    }
    crate::overrides::ref_update_3d(e, a.this_obj);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_AddPrimitive_Execute.
/// For a type 1 to 3 on a reference whose base is form 0x21 or an activator (not a plane) and
/// that has no primitive: a new `ExtraPrimitive` (added with no primitive), then a zeroed
/// primitive of the type's size with its vtable, type and half-sizes (a sphere's three are the
/// first; a plane's middle one is 1); the 3D reloaded; result 1.
fn add_primitive(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let kind = read.int(0, 0);
    if !(1..=3).contains(&kind) {
        return handled(true);
    }
    // JIP reads the base form unchecked; the port does nothing for a reference with none.
    let base = e.mem.u32(a.this_obj + BASE);
    if base == 0
        || !(super::ref_id(e, base) == PRIMITIVE_BASE_ID
            || (super::type_of(e, base) == ACTIVATOR && kind != 3))
    {
        return handled(true);
    }
    let list = a.this_obj + super::extra::REFR_EXTRA_LIST;
    if super::extra::has_type(e, list, EXTRA_PRIMITIVE) {
        return handled(true);
    }
    let x = super::extra::create(e, VTBL_EXTRA_PRIMITIVE, EXTRA_PRIMITIVE);
    e.mem.set_u32(x + EXTRA_WORD, 0);
    super::extra::add_extra(e, list, x);
    let size = if kind == 1 { 0x4c } else { 0x34 };
    let p = super::heap_alloc(e, size);
    for i in (0..size).step_by(4) {
        e.mem.set_u32(p + i, 0);
    }
    let vtbl = match kind {
        1 => VTBL_PRIMITIVE_BOX,
        2 => VTBL_PRIMITIVE_SPHERE,
        _ => VTBL_PRIMITIVE_PLANE,
    };
    e.mem.set_u32(p, vtbl);
    e.mem.set_u32(p + PRIMITIVE_TYPE, kind);
    let half = [
        read.float(1, 0.0) * 0.5,
        read.float(2, 0.0) * 0.5,
        read.float(3, 0.0) * 0.5,
    ];
    let bounds = match kind {
        2 => [half[0], half[0], half[0]],
        1 => half,
        _ => [half[0], 1.0, half[2]],
    };
    for (i, v) in bounds.into_iter().enumerate() {
        e.mem.set_f32(p + PRIMITIVE_BOUNDS + 4 * i as u32, v);
    }
    e.mem.set_u32(x + EXTRA_WORD, p);
    crate::overrides::ref_update_3d(e, a.this_obj);
    a.set_result(e, 1.0);
    handled(true)
}

/// `Actor::baseProcess` (+0x68), a process's level (+0x28: 0 is high) and a high process's
/// fade delay (`HighProcess::FadeOutAndMove` `008FE9E0` reads it at +0x3EC).
const PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
const FADE_DELAY: u32 = 0x3ec;
/// `HighProcess::FadeOutAndMove` (thiscall (process, actor, destination, 0, 0, 0)) and
/// `Script::MoveToFunctionBase` (`005CCB20`, JIP's `ADDR_MoveToMarker`: cdecl (reference,
/// destination, three float offsets)).
const FADE_OUT_AND_MOVE: u32 = 0x008f_e9e0;
const MOVE_TO_MARKER: u32 = 0x005c_cb20;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_MoveToFadeDelay_Execute.
/// An actor with a high process fades out and moves after the delay (the game's own fade); any
/// other reference moves at once (`MoveTo` with no offsets).
fn move_to_fade_delay(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let dest = read.form(0);
    if super::is_actor(e, a.this_obj) {
        let process = e.mem.u32(a.this_obj + PROCESS);
        if process != 0 && e.mem.u32(process + PROCESS_LEVEL) == 0 {
            e.mem.set_f32(process + FADE_DELAY, read.float(1, 0.0));
            e.call(
                FADE_OUT_AND_MOVE,
                &args![process, a.this_obj, dest, 0u32, 0u32, 0u32],
            );
            return handled(true);
        }
    }
    e.call(MOVE_TO_MARKER, &args![a.this_obj, dest, 0u32, 0u32, 0u32]);
    handled(true)
}

/// JIP's `kAnimGroupClassify` (`internal/jip_core.cpp`): each animation group's category,
/// sub-type and flags (its fourth byte is 0 in every entry), for the 245 groups.
#[rustfmt::skip]
const ANIM_GROUP_CLASSIFY: [[u8; 3]; 245] = [
    [1, 1, 0],[1, 1, 0],[1, 1, 0],[2, 1, 0],[2, 2, 0],[2, 3, 0],[2, 4, 0],[2, 1, 1],[2, 2, 1],[2, 3, 1],
    [2, 4, 1],[2, 1, 2],[2, 2, 2],[2, 3, 2],[2, 4, 2],[2, 3, 4],[2, 4, 4],[3, 0, 0],[3, 0, 1],[3, 0, 2],
    [3, 0, 4],[3, 0, 5],[3, 0, 6],[1, 2, 0],[1, 2, 0],[1, 2, 0],[3, 1, 0],[3, 1, 1],[3, 1, 2],[3, 1, 4],
    [3, 1, 5],[3, 1, 6],[3, 2, 0],[3, 2, 1],[3, 2, 2],[3, 2, 4],[3, 2, 5],[3, 2, 6],[3, 3, 0],[3, 3, 1],
    [3, 3, 2],[3, 3, 4],[3, 3, 5],[3, 3, 6],[3, 4, 0],[3, 4, 1],[3, 4, 2],[3, 4, 4],[3, 4, 5],[3, 4, 6],
    [3, 5, 0],[3, 5, 1],[3, 5, 2],[3, 5, 4],[3, 5, 5],[3, 5, 6],[3, 6, 0],[3, 6, 1],[3, 6, 2],[3, 6, 4],
    [3, 6, 5],[3, 6, 6],[3, 7, 0],[3, 7, 1],[3, 7, 2],[3, 7, 4],[3, 7, 5],[3, 7, 6],[3, 8, 0],[3, 8, 1],
    [3, 8, 2],[3, 8, 4],[3, 8, 5],[3, 8, 6],[3, 10, 0],[3, 10, 1],[3, 10, 2],[3, 10, 4],[3, 10, 5],[3, 10, 6],
    [3, 11, 0],[3, 11, 1],[3, 11, 2],[3, 11, 4],[3, 11, 5],[3, 11, 6],[3, 12, 0],[3, 12, 1],[3, 12, 2],[3, 12, 4],
    [3, 12, 5],[3, 12, 6],[3, 23, 0],[3, 23, 0],[3, 23, 0],[3, 23, 0],[3, 23, 0],[3, 23, 0],[3, 23, 0],[3, 23, 0],
    [3, 23, 0],[3, 23, 0],[3, 21, 0],[3, 21, 1],[3, 21, 2],[3, 21, 4],[3, 21, 5],[3, 21, 6],[3, 22, 0],[3, 22, 1],
    [3, 22, 2],[3, 22, 4],[3, 22, 5],[3, 22, 6],[3, 13, 0],[3, 13, 1],[3, 13, 2],[3, 13, 4],[3, 13, 5],[3, 13, 6],
    [3, 14, 0],[3, 14, 1],[3, 14, 2],[3, 14, 4],[3, 14, 5],[3, 14, 6],[3, 15, 0],[3, 15, 1],[3, 15, 2],[3, 15, 4],
    [3, 15, 5],[3, 15, 6],[3, 16, 0],[3, 16, 1],[3, 16, 2],[3, 16, 4],[3, 16, 5],[3, 16, 6],[3, 17, 0],[3, 17, 1],
    [3, 17, 2],[3, 17, 4],[3, 17, 5],[3, 17, 6],[3, 9, 0],[3, 9, 1],[3, 9, 2],[3, 9, 4],[3, 9, 5],[3, 9, 6],
    [3, 18, 0],[3, 18, 1],[3, 18, 2],[3, 18, 4],[3, 18, 5],[3, 18, 6],[3, 19, 0],[3, 19, 1],[3, 19, 2],[3, 19, 4],
    [3, 19, 5],[3, 19, 6],[3, 20, 0],[3, 20, 1],[3, 20, 2],[3, 20, 4],[3, 20, 5],[3, 20, 6],[1, 0, 0],[1, 0, 0],
    [1, 3, 0],[1, 3, 0],[1, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],
    [4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],
    [4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],[4, 0, 0],
    [5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],
    [5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],[5, 0, 0],
    [5, 0, 0],[5, 0, 0],[5, 0, 0],[1, 0, 0],[1, 0, 0],[1, 0, 0],[1, 0, 0],[2, 5, 0],[2, 5, 0],[2, 5, 0],
    [1, 4, 0],[1, 4, 0],[1, 4, 0],[1, 4, 0],[1, 4, 0],[1, 4, 0],[2, 5, 0],[2, 5, 0],[2, 5, 0],[2, 5, 0],
    [1, 0, 0],[2, 5, 0],[2, 5, 0],[2, 5, 0],[2, 5, 0],
];

/// The player's form id (`IsPlayer`: `refID == 0x14`), its first-person animation data (+0x690:
/// the exe's `PlayerCharacter::GetAnimation` `00950A10` answers it), a high process's animation
/// data (+0x1C0: `MiddleHighProcess::GetAnimation` `00602150`), a reference's `GetAnimData`
/// (vtable +0x1E4), and `AnimData::animGroupIDs` (+0x4C, eight 16-bit ids).
const PLAYER_ID: u32 = 0x14;
const ANIM_DATA_1ST_PERSON: u32 = 0x690;
const PROCESS_ANIM_DATA: u32 = 0x1c0;
const VT_GET_ANIM_DATA: u32 = 0x1e4;
const ANIM_GROUP_IDS: u32 = 0x4c;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_IsAnimPlayingEx_Execute.
/// 1 when one of the eight playing groups (low byte below 245) has the category and, below
/// category 4, the sub-type (0: any) and one of the flags (0: any). With a point of view for the
/// player (1 first person, else its high process's), only while it has a high process.
fn is_anim_playing_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let category = read.int(0, 0);
    let sub_type = read.int(1, 0);
    let flags = read.int(2, 0);
    let pov = read.int(3, 0);
    let refr = a.this_obj;
    let anim_data = if pov != 0 && super::ref_id(e, refr) == PLAYER_ID {
        let process = e.mem.u32(refr + PROCESS);
        if process == 0 || e.mem.u32(process + PROCESS_LEVEL) != 0 {
            return handled(true);
        }
        if pov == 1 {
            e.mem.u32(refr + ANIM_DATA_1ST_PERSON)
        } else {
            e.mem.u32(process + PROCESS_ANIM_DATA)
        }
    } else {
        let f = e.mem.u32(e.mem.u32(refr) + VT_GET_ANIM_DATA);
        e.call(f, &args![refr]).u32()
    };
    if anim_data == 0 {
        return handled(true);
    }
    for i in 0..8 {
        let id = usize::from(e.mem.u16(anim_data + ANIM_GROUP_IDS + 2 * i) & 0xff);
        let Some(&[cat, sub, fl]) = ANIM_GROUP_CLASSIFY.get(id) else {
            continue;
        };
        if u32::from(cat) == category
            && (category >= 4
                || ((sub_type == 0 || u32::from(sub) == sub_type)
                    && (flags == 0 || u32::from(fl) & flags != 0)))
        {
            a.set_result(e, 1.0);
            break;
        }
    }
    handled(true)
}

/// `HUDMainMenu` (`*0x011D96C0`) and its crosshair reference (+0x1B8: the exe's
/// `HUDMainMenu::SetInfoForRef` `00775A00` compares with it); `SetInfoForRef` itself (cdecl
/// (reference, 0, 0)), `kXData_ExtraOwnership` (0x21), the player's add (`00953FF0`, thiscall
/// (player, item, 1, 0)), `TESObjectREFR::MarkAsPickedUp` (`00572230`), a container's add
/// (`00574B30`, thiscall (container, item, 1, 0, 0)), `ExtraDataList`'s flags test
/// (`0041B3A0`, thiscall (list, mask) -> bool) and a form's `Destroy` (vtable +0x10).
const HUD_MAIN_MENU: u32 = 0x011d_96c0;
const HUD_CROSSHAIR_REF: u32 = 0x1b8;
const SET_INFO_FOR_REF: u32 = 0x0077_5a00;
const EXTRA_OWNERSHIP: u32 = 0x21;
const PLAYER_ADD_ITEM: u32 = 0x0095_3ff0;
const MARK_AS_PICKED_UP: u32 = 0x0057_2230;
const CONTAINER_ADD_ITEM: u32 = 0x0057_4b30;
const EXTRA_FLAGS_TEST: u32 = 0x0041_b3a0;
const VT_DESTROY: u32 = 0x10;
/// `TESForm::mods` (+0x10, a list with its first node inline: empty when its item is 0).
const FORM_MODS: u32 = 0x10;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_MoveToContainer_Execute.
/// An item reference into a reference whose base has a container: the HUD's crosshair text
/// cleared when it is under the crosshair, its ownership dropped when asked; into the player
/// through the player's add, else picked up, added to the container, and destroyed unless its
/// flag 1 is set, it belongs to a file, or its extra data answers the flags test 2.
fn move_to_container(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let target = read.form(0);
    let clear_owner = read.int(1, 0);
    let refr = a.this_obj;
    // JIP reads both base forms unchecked; the port does nothing when either is missing.
    let base = e.mem.u32(refr + BASE);
    let target_base = if target == 0 {
        0
    } else {
        e.mem.u32(target + BASE)
    };
    if base == 0
        || target_base == 0
        || !super::forms::INVENTORY_TYPE
            .get(usize::from(super::type_of(e, base)))
            .copied()
            .unwrap_or(false)
        || super::ln_fn_inventory::container_of(e, target_base) == 0
    {
        return handled(true);
    }
    let hud = e.mem.u32(HUD_MAIN_MENU);
    if hud != 0 && refr == e.mem.u32(hud + HUD_CROSSHAIR_REF) {
        e.call(SET_INFO_FOR_REF, &args![0u32, 0u32, 0u32]);
    }
    let list = refr + super::extra::REFR_EXTRA_LIST;
    if clear_owner != 0 {
        super::extra::remove_by_type(e, list, EXTRA_OWNERSHIP);
    }
    if super::ref_id(e, target) == PLAYER_ID {
        e.call(PLAYER_ADD_ITEM, &args![target, refr, 1u32, 0u32]);
    } else {
        e.call(MARK_AS_PICKED_UP, &args![refr]);
        e.call(CONTAINER_ADD_ITEM, &args![target, refr, 1u32, 0u32, 0u32]);
        if e.mem.u32(refr + FORM_FLAGS) & 1 == 0
            && e.mem.u32(refr + FORM_MODS) == 0
            && e.call(EXTRA_FLAGS_TEST, &args![list, 2u32]).u32() & 0xff == 0
        {
            let destroy = e.mem.u32(e.mem.u32(refr) + VT_DESTROY);
            e.call(destroy, &args![refr, 1u32]);
        }
    }
    handled(true)
}

// ---- positions and angles into script variables (`ResultVars`) ----

/// `TESObjectREFR::rotation` (+0x24) and `position` (+0x30), three floats each; the reference's
/// loaded data (+0x64) and its root node (+0x14 there; JIP's `GetRefNiNode`); a node's world
/// rotation (+0x68, nine floats by rows) and translation (+0x8C): `m_kWorld` (Xbox PDB).
const ROTATION: u32 = 0x24;
const POSITION: u32 = 0x30;
const LOADED_DATA: u32 = 0x64;
const ROOT_NODE: u32 = 0x14;
const WORLD_ROTATE: u32 = 0x68;
const WORLD_TRANSLATE: u32 = 0x8c;
/// `Flt180dPI` (JIP's `57.29578018F`, `GET_PS(9)`'s lanes).
const FLT_180_D_PI: f32 = 57.295_78;

/// `ResultVars::Set`: each value as a double into its script variable (`ScriptLocal::data`,
/// +8), x, y, z in that order.
fn set_vars(e: &mut Engine, vars: [u32; 3], values: [f32; 3]) {
    for (var, v) in vars.into_iter().zip(values) {
        e.mem.set_f64(var + 8, f64::from(v));
    }
}

fn floats3(e: &Engine, at: u32) -> [f32; 3] {
    [e.mem.f32(at), e.mem.f32(at + 4), e.mem.f32(at + 8)]
}

/// JIP's `GetRefNiNode`: the reference's root node, 0 without 3D.
fn ref_ni_node(e: &Engine, refr: u32) -> u32 {
    let loaded = e.mem.u32(refr + LOADED_DATA);
    if loaded == 0 {
        0
    } else {
        e.mem.u32(loaded + ROOT_NODE)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: GetSinCos_V3 (asm).
/// The sines (one three-lane `Cos_V3` of pi/2 less each angle, in floats) and the cosines (one
/// three-lane `Cos_V3` of the angles): the lanes of each call share its whole-turn reduction.
fn sin_cos(angles: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    use super::ln_fn_math::cos_v3;
    let pid2 = std::f32::consts::FRAC_PI_2;
    (cos_v3(angles.map(|x| pid2 - x)), cos_v3(angles))
}

fn neg(v: f32) -> f32 {
    f32::from_bits(v.to_bits() ^ 0x8000_0000)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiMatrix33::FromEulerPRY
// (asm).
/// The rotation matrix (by rows) of pitch, roll and yaw, each product in the order the asm's
/// lanes multiply, the minus signs as its sign flips.
fn from_euler_pry(pry: [f32; 3]) -> [f32; 9] {
    let ([s0, s1, s2], [c0, c1, c2]) = sin_cos(pry);
    [
        c1 * c2,
        c1 * s2,
        neg(s1),
        (s0 * s1) * c2 + neg(c0 * s2),
        (s0 * s1) * s2 + c0 * c2,
        c1 * s0,
        (c0 * s1) * c2 + s0 * s2,
        (c0 * s1) * s2 + neg(s0 * c2),
        c1 * c0,
    ]
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiMatrix33::MultiplyVector
// (asm).
/// Each row times the vector, summed as `haddps` twice sums: (a0 + a1) + (a2 + a3), where a3 is
/// the float after the row (the next row's first; after the last row `after`, the word past the
/// matrix) times the vector's cleared fourth lane: 0, or NaN for an infinite or NaN word.
fn multiply_vector(m: &[f32; 9], after: f32, v: [f32; 3]) -> [f32; 3] {
    let row = |i: usize| {
        let next = if i == 2 { after } else { m[3 * i + 3] };
        (m[3 * i] * v[0] + m[3 * i + 1] * v[1]) + (m[3 * i + 2] * v[2] + next * 0.0)
    };
    [row(0), row(1), row(2)]
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/Ni_types.cpp: NiMatrix33::ToEulerPRYInv
// (asm).
/// `[ATan2(m7, m8), ASin(-m6), ATan2(m3, m0)]` (JIP's routines, `ln_fn_math`).
fn to_euler_pry_inv(m: &[f32; 9]) -> [f32; 3] {
    use super::ln_fn_math::{asin, atan2};
    [atan2(m[7], m[8]), asin(neg(m[6])), atan2(m[3], m[0])]
}

fn matrix_at(e: &Engine, at: u32) -> [f32; 9] {
    std::array::from_fn(|i| e.mem.f32(at + 4 * i as u32))
}

/// The three script variable arguments from `first`, or `None` when the reader did not give
/// three.
fn var_args(read: &Read, first: usize) -> Option<[u32; 3]> {
    let vars = [
        read.local(first),
        read.local(first + 1),
        read.local(first + 2),
    ];
    vars.iter().all(|&v| v != 0).then_some(vars)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetPosEx_Execute.
/// The reference's position into the three variables.
fn get_pos_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    if let Some(vars) = var_args(&read, 0) {
        let pos = floats3(e, a.this_obj + POSITION);
        set_vars(e, vars, pos);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetAngleEx_Execute.
/// The reference's angles in degrees (each float times `Flt180dPI`) into the three variables;
/// with the local flag, the angles of its root node's world rotation (`ToEulerPRYInv`), or for the
/// player or a reference without 3D those of the matrix made from its angles.
fn get_angle_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let Some(vars) = var_args(&read, 0) else {
        return handled(true);
    };
    let refr = a.this_obj;
    let rotation = floats3(e, refr + ROTATION);
    let angles = if read.int(3, 0) == 0 {
        rotation
    } else {
        let node = ref_ni_node(e, refr);
        let m = if node != 0 && super::ref_id(e, refr) != PLAYER_ID {
            matrix_at(e, node + WORLD_ROTATE)
        } else {
            from_euler_pry(rotation)
        };
        to_euler_pry_inv(&m)
    };
    set_vars(e, vars, angles.map(|v| v * FLT_180_D_PI));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetTranslatedPos_Execute, and nvse/GameObjects.cpp: TESObjectREFR::GetTranslatedPos (asm).
/// The offsets turned by the root node's world rotation plus its world translation, or without
/// 3D turned by the matrix of the reference's angles plus its position, into the three
/// variables; result 1.
///
/// Without 3D, JIP's matrix is on its stack and the last row's product reads the word past it
/// (the return address, a finite positive value): its product is +0, as the port takes it.
fn get_translated_pos(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let Some(vars) = var_args(&read, 3) else {
        return handled(true);
    };
    let refr = a.this_obj;
    let mods = [read.float(0, 0.0), read.float(1, 0.0), read.float(2, 0.0)];
    let node = ref_ni_node(e, refr);
    let (turned, add) = if node != 0 {
        let m = matrix_at(e, node + WORLD_ROTATE);
        let after = e.mem.f32(node + WORLD_TRANSLATE);
        (
            multiply_vector(&m, after, mods),
            floats3(e, node + WORLD_TRANSLATE),
        )
    } else {
        let m = from_euler_pry(floats3(e, refr + ROTATION));
        (multiply_vector(&m, 0.0, mods), floats3(e, refr + POSITION))
    };
    let pos = [turned[0] + add[0], turned[1] + add[1], turned[2] + add[2]];
    set_vars(e, vars, pos);
    a.set_result(e, 1.0);
    handled(true)
}

/// `Actor`'s editor place (the exe's `0087F920`, the actor's `GetStartingPosition`): position
/// +0x160, z angle +0x16C, world or cell +0x170. `ExtraStartingPosition` (type 0x0F: its
/// constructor `00430780` copies the position to +0xC and the angles to +0x18) and
/// `ExtraStartingWorldOrCell` (type 0x49, the form at +0xC: constructor `004308C0`).
const ACTOR_START_POS: u32 = 0x160;
const ACTOR_START_Z_ROT: u32 = 0x16c;
const ACTOR_START_CELL: u32 = 0x170;
const EXTRA_STARTING_POSITION: u32 = 0x0f;
const EXTRA_STARTING_WORLD_OR_CELL: u32 = 0x49;
const STARTING_ROTATION: u32 = 0x18;
/// `kFormType_TESObjectCELL`, `kFormType_TESWorldSpace` and a worldspace's persistent cell
/// (+0x34).
const CELL: u8 = 0x39;
const WORLD_SPACE: u8 = 0x41;
const WORLD_CELL: u32 = 0x34;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetEditorPosition_Execute.
/// The reference's editor position into the first three variables and, when all six are given,
/// its editor angles in degrees into the last three; the result is its editor cell (a
/// worldspace's persistent cell). An actor's place is its own (angles 0, 0, z); another
/// reference's needs its starting world or cell and starting position extras.
fn get_editor_position(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let num_args = super::jip_fn_utility::num_args(e, &a);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let Some(pos_vars) = var_args(&read, 0) else {
        return handled(true);
    };
    let refr = a.this_obj;
    let mut place: Option<(u32, [f32; 3], [f32; 3])> = None;
    if super::is_actor(e, refr) {
        let cell = e.mem.u32(refr + ACTOR_START_CELL);
        let pos = floats3(e, refr + ACTOR_START_POS);
        let rot = [0.0, 0.0, e.mem.f32(refr + ACTOR_START_Z_ROT)];
        place = Some((cell, pos, rot));
    } else {
        let xcell = super::extra::refr_extra(e, refr, EXTRA_STARTING_WORLD_OR_CELL);
        if xcell != 0 {
            let cell = e.mem.u32(xcell + EXTRA_WORD);
            let xpos = super::extra::refr_extra(e, refr, EXTRA_STARTING_POSITION);
            if xpos != 0 {
                let pos = floats3(e, xpos + EXTRA_WORD);
                let rot = floats3(e, xpos + STARTING_ROTATION);
                place = Some((cell, pos, rot));
            }
        }
    }
    let Some((mut cell, pos, rot)) = place else {
        return handled(true);
    };
    if cell == 0 {
        return handled(true);
    }
    match super::type_of(e, cell) {
        CELL => {}
        WORLD_SPACE => {
            cell = e.mem.u32(cell + WORLD_CELL);
            if cell == 0 {
                return handled(true);
            }
        }
        _ => return handled(true),
    }
    set_vars(e, pos_vars, pos);
    if num_args == 6 {
        if let Some(rot_vars) = var_args(&read, 3) {
            set_vars(e, rot_vars, rot.map(|v| v * FLT_180_D_PI));
        }
    }
    super::set_form_result(e, &a, cell);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_tests.rs"]
mod tests;
