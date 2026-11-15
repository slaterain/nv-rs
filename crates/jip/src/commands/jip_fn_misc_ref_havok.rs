//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): the reference commands that read physics
//! (Havok) state or hand a ray cast to the game, on the lookups of `havok_refs.rs`. 17 of the
//! file's commands: the read-only ones (a body's mass, linear and angular velocities, collision
//! properties and layer, whether the reference is mobile, grabbable or has a phantom, what it
//! touches) and the game calls (the crosshair pick `00631D60`, the world ray casts through
//! `TES::Pick_ov2` `00458440`).
//!
//! Not built, each waiting for what it names:
//! - Writes of Havok state: `SetLinearVelocity`, `SetAngularVelocity`, `SetAngularVelocityEx`,
//!   `PushObject` (each stores the velocity, then calls `hkpEntity::activate` `00C9C1D0`, which
//!   wakes the body's simulation island in the physics world: world work, not a plain store);
//!   `SetCollisionObjProperty` (property 8 calls `bhkRigidBody::SetMotionType`, which takes the
//!   body out of the world and adds it back; the others are plain stores, but one command cannot
//!   be half built); `SetCollisionObjLayerType` (the layer store is followed by the world object's
//!   `UpdateCollisionFilter`, slot +0xC4, a world filter update); `ToggleNodeCollision` (stores the
//!   filter info's no-collision bit with no filter update; whether the game itself makes this
//!   plain store, the condition for building a write here, is not established yet).
//! - `MoveToReticle`: JIP's `TESObjectREFR::MoveToCell` uses its position marker
//!   (`s_tempPosMarker`, made in its deferred start, not built), as lane E's `MoveToCell` waits.
//! - `GetRayCastMaterial`: JIP's material table `kMaterialConvert`, the land lookup `00457720`
//!   and the actor branch (vtable +0x218, the base's +0x1E4/+0x148) are not traced yet.
//! - `GetCollisionNodes` is lane F's (`jip_fn_misc_ref_nodes.rs`).
//!
//! Deliberate differences (where JIP would stop the game or read garbage): every Havok pointer is
//! read checked and every walk bounded (see `havok_refs.rs`); an axis byte other than X, Y or Z
//! answers nothing where JIP indexes past the vector; a null form or global argument answers
//! nothing; a ray cast or pick without the player's controller, phantom, 3D or the interface's
//! view caster answers nothing where JIP reads through null.

use super::forms::ListNode;
use super::havok_refs as h;
use super::Handlers;
use engine_core::prelude::*;
use nvse::args::Arg;
use nvse::commands::Read;
use nvse::{handled, CommandArgs, EvalArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27b6, get_has_contact),
    (0x27b7, get_contact_refs),
    (0x27b8, get_has_contact_base),
    (0x27bb, get_has_phantom),
    (0x2843, get_rigid_body_mass),
    (0x285a, get_object_velocity),
    (0x285b, get_angular_velocity),
    (0x285e, get_ray_cast_pos),
    (0x2875, get_crosshair_ref_ex),
    (0x2876, is_mobile),
    (0x2877, is_grabbable),
    (0x28e2, get_ray_cast_ref),
    (0x290f, get_linear_velocity_alt),
    (0x2910, get_angular_velocity_alt),
    (0x2913, get_collision_obj_property),
    (0x2915, get_collision_obj_layer_type),
];

pub static CONDITIONS: super::Conditions =
    &[(0x27b9, get_has_contact_type, get_has_contact_type_eval)];

/// `hkpRigidBody` fields read here (see `havok_refs.rs` for where each is confirmed).
const FRICTION: u32 = 0x90;
const RESTITUTION: u32 = 0x94;
const LINEAR_DAMPING: u32 = 0x194;
const ANGULAR_DAMPING: u32 = 0x198;
const INERTIA: u32 = 0x1a0;
const LIN_VELOCITY: u32 = 0x1b0;
const ANG_VELOCITY: u32 = 0x1c0;
/// `TESForm::typeID`, a reference's base form, `kFormType_BGSListForm` and its `tList` (+0x18).
const TYPE_ID: u32 = 0x04;
const BASE_FORM: u32 = 0x20;
const LIST_FORM_TYPE: u8 = 0x55;
const LIST_FORMS: u32 = 0x18;
/// `TESGlobal::data` (+0x24).
const GLOBAL_DATA: u32 = 0x24;
/// The block name buffers' size in JIP (`char[0x40]`).
const NAME_BUFFER: usize = 0x40;

/// A string argument as JIP's 0x40-byte name buffer holds it.
fn name_arg(e: &Engine, read: &Read, i: usize, command: &str) -> Vec<u8> {
    super::buffer_arg(e, read, i, NAME_BUFFER, command)
}

/// The axis byte argument `i` (`kParamType_Axis`), 0 when not given.
fn axis_arg(read: &Read, i: usize) -> u8 {
    match read.args.get(i) {
        Some(Arg::Byte(b)) => *b,
        _ => 0,
    }
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

/// `ResultVars::Set`: each value as a double into its script variable (`ScriptLocal::data`, +8).
fn set_vars(e: &mut Engine, vars: [u32; 3], values: [f32; 3]) {
    for (var, v) in vars.into_iter().zip(values) {
        e.mem.set_f64(var.wrapping_add(8), f64::from(v));
    }
}

/// The rigid body of the named block when it is mobile (`GetRigidBody(name)` and `IsMobile`), 0
/// otherwise.
fn mobile_body(e: &Engine, refr: u32, name: &[u8]) -> u32 {
    let body = h::rigid_body(e, refr, name);
    if body != 0 && h::body_is_mobile(e, body) {
        body
    } else {
        0
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/havok.cpp: hkMatrix3x4::MultiplyVector
// (asm).
/// Each of the rotation's three 16-byte columns dotted with the vector's x, y, z (its fourth lane
/// cleared), each summed as `haddps` twice sums: `(c0*v0 + c1*v1) + (c2*v2 + c3*0)`.
fn rotate(cols: [[f32; 4]; 3], v: [f32; 4]) -> [f32; 3] {
    cols.map(|c| (c[0] * v[0] + c[1] * v[1]) + (c[2] * v[2] + c[3] * 0.0))
}

/// The body's rotation columns (+0xF0, +0x100, +0x110).
fn rotation(e: &Engine, body: u32) -> Option<[[f32; 4]; 3]> {
    Some([
        h::vec4(e, body.wrapping_add(h::ROTATION))?,
        h::vec4(e, body.wrapping_add(h::ROTATION + 0x10))?,
        h::vec4(e, body.wrapping_add(h::ROTATION + 0x20))?,
    ])
}

// ---- contacts ----

/// JIP's `TempFormList` filled by `PopulateFormSet`: a form list's items (its first node's data
/// even when empty, as JIP's do-while inserts it), or the form itself; none for a null form.
fn form_set(e: &Engine, form: u32) -> Vec<u32> {
    if form == 0 {
        return Vec::new();
    }
    if h::rd8(e, form.wrapping_add(TYPE_ID)) == Some(LIST_FORM_TYPE) {
        super::list_items(e, Ptr::<ListNode>::new(form.wrapping_add(LIST_FORMS)))
    } else {
        vec![form]
    }
}

/// The base form of a reference (+0x20), 0 when unreadable.
fn base_of(e: &Engine, refr: u32) -> u32 {
    h::rd32(e, refr.wrapping_add(BASE_FORM)).unwrap_or(0)
}

/// Whether any touched object's reference (the body under an actor's feet first) passes `test`.
/// One budget ([`h::Walk`]) covers the whole command: the contact lists and every parent lookup.
fn any_contact(e: &Engine, refr: u32, test: impl Fn(u32) -> bool) -> bool {
    let mut walk = h::Walk::new();
    let Some(c) = h::contacts(e, refr, &mut walk) else {
        return false;
    };
    if c.under_feet != 0 {
        let r = h::object_parent_ref(e, c.under_feet, &mut walk);
        if r != 0 && test(r) {
            return true;
        }
    }
    c.bodies.into_iter().any(|body| {
        let r = h::object_parent_ref(e, body, &mut walk);
        r != 0 && test(r)
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetHasContact_Execute.
/// 1 when the reference touches the form (or one in the form list): the body under an actor's
/// feet, an actor's controller's touched objects, else the reference's phantom's and bodies'.
fn get_has_contact(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let set = form_set(e, read.form(0));
    if any_contact(e, a.this_obj, |r| set.contains(&r)) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetHasContactBase_Execute.
/// [`get_has_contact`] matching the touched references' base forms.
fn get_has_contact_base(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let set = form_set(e, read.form(0));
    if any_contact(e, a.this_obj, |r| set.contains(&base_of(e, r))) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h: GetHasContactType.
/// Whether a touched reference's base form has the type (`typeID`, compared as a whole word).
fn has_contact_type(e: &Engine, refr: u32, type_id: u32) -> bool {
    any_contact(e, refr, |r| {
        let base = base_of(e, r);
        base != 0 && h::rd8(e, base.wrapping_add(TYPE_ID)).map(u32::from) == Some(type_id)
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetHasContactType_Execute/_Eval.
/// 1 when a touched reference's base form is of the type.
fn get_has_contact_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && a.this_obj != 0 && has_contact_type(e, a.this_obj, read.int(0, 0)) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

fn get_has_contact_type_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    if a.this_obj != 0 && has_contact_type(e, a.this_obj, a.arg1) {
        e.mem.set_f64(a.result, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetContactRefs_Execute.
/// An array of the references touched: the one under an actor's feet appended first, the others
/// inserted uniquely (JIP's `InsertUnique`, a sorted insert by form address); no array when
/// none.
fn get_contact_refs(e: &mut Engine, w: &[u32]) -> Ret {
    use super::arrays::{Element, TempElements};
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let mut walk = h::Walk::new();
    let Some(c) = h::contacts(e, a.this_obj, &mut walk) else {
        return handled(true);
    };
    let mut elements = TempElements::new();
    if c.under_feet != 0 {
        let r = h::object_parent_ref(e, c.under_feet, &mut walk);
        if r != 0 {
            elements.append(Element::Form(r));
        }
    }
    for body in c.bodies {
        let r = h::object_parent_ref(e, body, &mut walk);
        if r != 0 {
            elements.insert_unique(Element::Form(r));
        }
    }
    if !elements.is_empty() {
        let id = super::arrays::create_array(e, a.script, &elements.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetHasPhantom_Execute.
/// 1 for a reference with 3D (loaded data and root node) that is an actor, has a
/// `bhkSimpleShapePhantom` trigger phantom, or has a phantom on a node.
fn get_has_phantom(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let refr = a.this_obj;
    if refr == 0 {
        return handled(true);
    }
    let Some(loaded) = h::ptr(e, refr.wrapping_add(h::LOADED_DATA)) else {
        return handled(true);
    };
    let Some(root) = h::ptr(e, loaded.wrapping_add(h::ROOT_NODE)) else {
        return handled(true);
    };
    let phantom = h::ptr(e, loaded.wrapping_add(h::PHANTOM))
        .is_some_and(|p| h::rd32(e, p) == Some(h::BHK_SIMPLE_SHAPE_PHANTOM_VTBL));
    if h::is_actor(e, refr) || phantom || h::has_phantom(e, root) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// ---- bodies ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetRigidBodyMass_Execute.
/// For a reference that is not an actor, the masses of the rigid bodies under its root node
/// summed (`NiNode::GetBodyMass(0)`).
fn get_rigid_body_mass(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let refr = a.this_obj;
    if refr == 0 || h::is_actor(e, refr) {
        return handled(true);
    }
    let root = h::ref_ni_node(e, refr);
    if root != 0 {
        let mass = h::body_mass(e, root);
        a.set_result(e, f64::from(mass));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetObjectVelocity_Execute.
/// The root's mobile rigid body's linear velocity on the axis, or without one its length
/// (`Length_V4` of all four lanes: `(x*x + y*y) + (z*z + w*w)`, then the estimate's square
/// root).
fn get_object_velocity(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let axis = axis_arg(&read, 0);
    let body = mobile_body(e, a.this_obj, b"");
    let Some(v) = (body != 0)
        .then(|| h::vec4(e, body.wrapping_add(LIN_VELOCITY)))
        .flatten()
    else {
        return handled(true);
    };
    if axis == 0 {
        let s = (v[0] * v[0] + v[1] * v[1]) + (v[2] * v[2] + v[3] * v[3]);
        a.set_result(e, f64::from(super::ln_fn_math::newton_sqrt(s)));
    } else if let Some(i) = axis.checked_sub(b'X').filter(|&i| i < 3) {
        a.set_result(e, f64::from(v[usize::from(i)]));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetAngularVelocity_Execute (asm).
/// The named block's mobile rigid body's angular velocity (x, y, z, the fourth lane cleared):
/// dotted with the rotation's column of the axis (`(a0 + a1) + (a2 + 0 * c3)`), or without an
/// axis its length (the exact `sqrtss` of the squares summed so).
fn get_angular_velocity(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = name_arg(e, &read, 0, "GetAngularVelocity");
    let axis = axis_arg(&read, 1);
    let body = mobile_body(e, a.this_obj, &name);
    if body == 0 {
        return handled(true);
    }
    let Some(v) = h::vec4(e, body.wrapping_add(ANG_VELOCITY)) else {
        return handled(true);
    };
    let v = [v[0], v[1], v[2], 0.0];
    let index = axis.wrapping_sub(b'X') as i8;
    let value = if index < 0 {
        ((v[0] * v[0] + v[1] * v[1]) + (v[2] * v[2] + v[3] * v[3])).sqrt()
    } else if index < 3 {
        let Some(c) = h::vec4(e, body.wrapping_add(h::ROTATION + 0x10 * index as u32)) else {
            return handled(true);
        };
        (v[0] * c[0] + v[1] * c[1]) + (v[2] * c[2] + v[3] * c[3])
    } else {
        return handled(true);
    };
    a.set_result(e, f64::from(value));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetLinearVelocityAlt_Execute.
/// The named block's mobile rigid body's linear velocity into the three variables, with the
/// local flag turned by its rotation ([`rotate`]); 1 when written.
fn get_linear_velocity_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = name_arg(e, &read, 0, "GetLinearVelocityAlt");
    let Some(vars) = var_args(&read, 1) else {
        return handled(true);
    };
    let body = mobile_body(e, a.this_obj, &name);
    if body == 0 {
        return handled(true);
    }
    let Some(v) = h::vec4(e, body.wrapping_add(LIN_VELOCITY)) else {
        return handled(true);
    };
    let out = if read.int(4, 0) == 0 {
        [v[0], v[1], v[2]]
    } else {
        let Some(cols) = rotation(e, body) else {
            return handled(true);
        };
        rotate(cols, v)
    };
    set_vars(e, vars, out);
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetAngularVelocityAlt_Execute.
/// The named block's mobile rigid body's angular velocity into the three variables, turned by
/// its rotation unless the global flag is set; 1 when written.
fn get_angular_velocity_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = name_arg(e, &read, 0, "GetAngularVelocityAlt");
    let Some(vars) = var_args(&read, 1) else {
        return handled(true);
    };
    let body = mobile_body(e, a.this_obj, &name);
    if body == 0 {
        return handled(true);
    }
    let Some(v) = h::vec4(e, body.wrapping_add(ANG_VELOCITY)) else {
        return handled(true);
    };
    let out = if read.int(4, 0) == 0 {
        let Some(cols) = rotation(e, body) else {
            return handled(true);
        };
        rotate(cols, v)
    } else {
        [v[0], v[1], v[2]]
    };
    set_vars(e, vars, out);
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetCollisionObjProperty_Execute.
/// A property (0 to 8) of the named block's rigid body: friction, restitution, linear and
/// angular damping, the three inertia floats, the mass (1 over the inverse mass in doubles, only
/// when that is above 0), the motion type.
fn get_collision_obj_property(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = name_arg(e, &read, 0, "GetCollisionObjProperty");
    let prop = read.int(1, 0);
    if prop > 8 {
        return handled(true);
    }
    let body = h::rigid_body(e, a.this_obj, &name);
    if body == 0 {
        return handled(true);
    }
    let float = |at: u32| h::rdf32(e, body.wrapping_add(at)).map(f64::from);
    let value = match prop {
        0 => float(FRICTION),
        1 => float(RESTITUTION),
        2 => float(LINEAR_DAMPING),
        3 => float(ANGULAR_DAMPING),
        4..=6 => float(INERTIA + 4 * (prop - 4)),
        7 => match h::rdf32(e, body.wrapping_add(h::MASS_INV)) {
            Some(inv) if inv > 0.0 => Some(1.0 / f64::from(inv)),
            _ => None,
        },
        _ => h::rd8(e, body.wrapping_add(h::MOTION_TYPE)).map(f64::from),
    };
    if let Some(v) = value {
        a.set_result(e, v);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetCollisionObjLayerType_Execute.
/// The collision layer (the low byte of the filter info, +0x2C) of the named node's world object
/// (`GetNode2`: the loaded root for an empty name).
fn get_collision_obj_layer_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = name_arg(e, &read, 0, "GetCollisionObjLayerType");
    let node = h::node2(e, a.this_obj, &name);
    if node == 0 {
        return handled(true);
    }
    if let Some(layer) =
        h::havok_object(e, node).and_then(|o| h::rd8(e, o.wrapping_add(h::FILTER_INFO)))
    {
        a.set_result(e, f64::from(layer));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::IsMobile.
/// An actor or a projectile, or a reference whose root node is movable.
fn ref_is_mobile(e: &Engine, refr: u32) -> bool {
    if h::is_actor(e, refr) || h::is_projectile(e, refr) {
        return true;
    }
    let root = h::ref_ni_node(e, refr);
    root != 0 && h::is_movable(e, root)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_IsMobile_Execute.
fn is_mobile(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj != 0 && ref_is_mobile(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

/// `kINIS_bAllowHavokGrabTheLiving_GamePlay`'s value (`Setting` at `011E0B1C`, value +4), as
/// JIP's `INIS_BOOL` reads it (a whole word); the game's own grab test reads the same setting
/// (`CreateMouseSpring` `0095F930`).
const GRAB_THE_LIVING: u32 = 0x011e_0b20;
/// `Actor::lifeState` (+0x108), `Projectile::projFlags` (+0xC8), and the projectile's virtual
/// `GetProjectileType` (+0x304) and `IsProximityTriggered` (+0x318), the slots and flag word the
/// game's grab test uses.
const LIFE_STATE: u32 = 0x108;
const PROJ_FLAGS: u32 = 0xc8;
const PROJECTILE_TYPE_SLOT: u32 = 0x304;
const PROXIMITY_TRIGGERED_SLOT: u32 = 0x318;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::IsGrabbable.
/// An actor when the setting allows the living or it is dead (life state 1 or 2); a projectile
/// with flag 0x200 whose type is 3 and that is proximity triggered; else a reference whose root
/// node is movable.
fn ref_is_grabbable(e: &mut Engine, refr: u32) -> bool {
    if h::is_actor(e, refr) {
        return h::rd32(e, GRAB_THE_LIVING).is_some_and(|v| v != 0)
            || matches!(h::rd32(e, refr.wrapping_add(LIFE_STATE)), Some(1 | 2));
    }
    if h::is_projectile(e, refr) {
        return h::rd32(e, refr.wrapping_add(PROJ_FLAGS)).is_some_and(|f| f & 0x200 != 0)
            && e.vcall(refr, PROJECTILE_TYPE_SLOT, &[]).u32() == 3
            && e.vcall(refr, PROXIMITY_TRIGGERED_SLOT, &[]).bool();
    }
    let root = h::ref_ni_node(e, refr);
    root != 0 && h::is_movable(e, root)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_IsGrabbable_Execute.
fn is_grabbable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj != 0 && ref_is_grabbable(e, a.this_obj) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// ---- game calls ----

/// The node's world translation and the word after it (16 bytes) and world rotation.
pub(crate) fn node_transform(e: &Engine, node: u32) -> Option<([f32; 4], [f32; 9])> {
    let pos = h::vec4(e, node.wrapping_add(h::WORLD_TRANSLATE))?;
    let mut rot = [0f32; 9];
    for (i, r) in rot.iter_mut().enumerate() {
        *r = h::rdf32(e, node.wrapping_add(h::WORLD_ROTATE + 4 * i as u32))?;
    }
    Some((pos, rot))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetRayCastPos_Execute.
/// The point the named node (`GetNode`; the seen root for an empty name) looks at, cast up to
/// the range (50000) on the layer (6), into the three globals' values with result 1, when the
/// cast counts as found or seven arguments were given. The fourth argument is read and unused,
/// as in JIP.
fn get_ray_cast_pos(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let num_args = super::jip_fn_utility::num_args(e, &a);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let globals = [read.form(0), read.form(1), read.form(2)];
    let layer = read.int(4, 6) as i32;
    let name = name_arg(e, &read, 5, "GetRayCastPos");
    let range = read.float(6, 50000.0);
    if globals.contains(&0) {
        return handled(true);
    }
    let node = h::node(e, a.this_obj, &name);
    let Some((pos, rot)) = (node != 0).then(|| node_transform(e, node)).flatten() else {
        return handled(true);
    };
    let Some(cast) = h::ray_cast(e, pos, rot, range, layer) else {
        return handled(true);
    };
    let (coords, found) = h::hit_coords(e, &cast);
    if found || num_args >= 7 {
        for (g, v) in globals.into_iter().zip(coords) {
            e.mem.set_f32(g.wrapping_add(GLOBAL_DATA), v);
        }
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetRayCastRef_Execute.
/// The reference of the block the named node looks at (cast 50000 on the layer, 6), if any.
fn get_ray_cast_ref(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let layer = read.int(0, 6) as i32;
    let name = name_arg(e, &read, 1, "GetRayCastRef");
    let node = h::node(e, a.this_obj, &name);
    let Some((pos, rot)) = (node != 0).then(|| node_transform(e, node)).flatten() else {
        return handled(true);
    };
    if let Some(cast) = h::ray_cast(e, pos, rot, 50000.0, layer) {
        let refr = h::block_parent_ref(e, cast.object, &mut h::Walk::new());
        if refr != 0 {
            super::set_form_result(e, &a, refr);
        }
    }
    handled(true)
}

/// `InterfaceManager` (`g_interfaceManager` `011D8A80`): the crosshair reference +0xFC and the
/// view caster +0x13C (`0070BC20` stores the one and calls the pick on the other).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const CROSSHAIR_REF: u32 = 0xfc;
const VIEW_CASTER: u32 = 0x13c;
/// `ViewCaster`'s pick (`00631D60`, thiscall `(start, direction, range, outDistance, outFlag)`;
/// the port's `run_006319e0.rs`) and JIP's range for it (`0x46400000`, 12288).
const VIEW_PICK: u32 = 0x0063_1d60;
const PICK_RANGE: u32 = 0x4640_0000;
/// The player's cached first person camera position (+0xDD4), third person shoulder offset
/// (+0xD58) and actual third person flag (byte +0x64B) (`playercharacter.rs`).
const CAMERA_POS: u32 = 0xdd4;
const SHOULDER_OFFSET: u32 = 0xd58;
const ACTUALLY_3RD_PERSON: u32 = 0x64b;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h: GetCrosshairRef
// (asm).
/// The view caster's pick from the player's camera position (16 bytes, plus the shoulder offset
/// lane by lane in third person) along the player's root's world rotation column 1, 12288 units;
/// both outputs written to the start's fourth word, as JIP passes them. 0 without the player, its
/// 3D or the view caster.
fn crosshair_pick(e: &mut Engine) -> u32 {
    let Some(player) = h::ptr(e, h::PLAYER) else {
        return 0;
    };
    let Some(mut start) = h::vec4(e, player.wrapping_add(CAMERA_POS)) else {
        return 0;
    };
    if h::rd8(e, player.wrapping_add(ACTUALLY_3RD_PERSON)).is_some_and(|b| b != 0) {
        let Some(offset) = h::vec4(e, player.wrapping_add(SHOULDER_OFFSET)) else {
            return 0;
        };
        start = std::array::from_fn(|i| start[i] + offset[i]);
    }
    let root = h::ref_ni_node(e, player);
    let dir = [0x6c, 0x78, 0x84].map(|o| h::rd32(e, root.wrapping_add(o)));
    let (Some(caster), true, [Some(d0), Some(d1), Some(d2)]) = (
        h::ptr(e, INTERFACE_MANAGER).and_then(|m| h::ptr(e, m.wrapping_add(VIEW_CASTER))),
        root != 0,
        dir,
    ) else {
        return 0;
    };
    e.with_stack(0x20, |e, frame| {
        let at = frame.addr();
        for (i, v) in start.iter().enumerate() {
            e.mem.set_f32(at + 4 * i as u32, *v);
        }
        e.mem.set_u32(at + 0x10, d0);
        e.mem.set_u32(at + 0x14, d1);
        e.mem.set_u32(at + 0x18, d2);
        let out = at + 0xc;
        e.call(
            VIEW_PICK,
            &args![caster, at, at + 0x10, PICK_RANGE, out, out],
        )
        .u32()
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetCrosshairRefEx_Execute.
/// The interface's crosshair reference, or without one what the view caster's pick finds.
fn get_crosshair_ref_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let mut refr = h::ptr(e, INTERFACE_MANAGER)
        .and_then(|m| h::rd32(e, m.wrapping_add(CROSSHAIR_REF)))
        .unwrap_or(0);
    if refr == 0 {
        refr = crosshair_pick(e);
    }
    if refr != 0 {
        super::set_form_result(e, &a, refr);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_misc_ref_havok_tests.rs"]
mod tests;
