//! `functions_jip/jip_fn_misc_ref.h` (JIP LN NVSE): the reference commands that write physics
//! (Havok) state, and the ray cast's material, on the lookups of `havok_refs.rs`. 8 of the
//! file's commands: the velocity stores (`SetLinearVelocity`, `SetAngularVelocity`,
//! `SetAngularVelocityEx`, `PushObject`), the collision property, layer and no-collision stores
//! (`SetCollisionObjProperty`, `SetCollisionObjLayerType`, `ToggleNodeCollision`) and
//! `GetRayCastMaterial`.
//!
//! Game calls, each where JIP makes it and with the arguments JIP passes (conventions confirmed
//! in the exe): `hkpEntity::activate` (`00C9C1D0`, `thiscall (body)`) after every velocity store;
//! the owning wrapper's (`hkpWorldObject` +0xC, stored by `bhkWorldObject::Link` `00C85BA0`)
//! `bhkRigidBody::SetMotionType` (vtable +0xE4, `00C8DD10` in both rigid body classes,
//! `thiscall (type)`) for property 8; the world object's `UpdateCollisionFilter` (vtable +0xC4,
//! `bhkEntity::UpdateCFilter` `00C9D5D0` for rigid bodies) after a layer store; the rigid body
//! wrapper's centre of mass in the world (vtable +0xF0, `00C8D240` / `bhkRigidBodyT`'s `00CB32A0`,
//! `thiscall (out)`, answering `out`) for a push; the game's pick (`TES::Pick_ov2`, through
//! `havok_refs::ray_cast`), the land lookup (`00457720`, `thiscall (TES, point)`) and the
//! reference's vtable +0x218 (`IsCharacter`: true for characters and the player, false for
//! creatures) for the material. Each game function decides itself what a body without a world
//! does (`activate` and the filter update do nothing; `SetMotionType` only swaps the motion).
//!
//! Threads: script commands run on the main thread while the Havok step is not running (it runs
//! between `StartThreads` and `WaitForThreads` under `TES::LockHavokUpdateMT`, `00453860`), so
//! these stores and calls happen where JIP's do in the real game. JIP takes no lock; neither does
//! the port.
//!
//! Not built: `MoveToReticle` (JIP's `MoveToCell` with its position marker: the marker is on
//! main, the move commands are lane H's and not on main yet).
//!
//! Deliberate differences (where JIP would stop the game or write elsewhere): every guest read
//! and store is checked and every walk bounded (`havok_refs.rs`); a flags argument with bits above
//! the low byte does nothing (JIP adds them to the velocity's address and stores there); an axis
//! byte other than X, Y or Z does nothing (JIP's 8-bit offset reads another field); a motion type
//! outside the game's 1 to 7 is not passed on (the game's motion builder makes a fixed motion for
//! it under a type byte that does not say fixed); a null wrapper, Havok object, shape or base form
//! counts as none where JIP reads through it; `GetRayCastMaterial` returns normally where JIP's
//! code returns with `retn 0x14` on its found paths (one stack word more than its four
//! arguments; its compiled caller restores the stack from its frame pointer, assumed, not read in
//! JIP's DLL).

use super::havok_refs as h;
use super::jip_fn_misc_ref_havok::{axis_arg, mobile_body, name_arg, node_transform, rotation};
use super::Handlers;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2844, push_object),
    (0x285c, set_angular_velocity),
    (0x28d0, set_linear_velocity),
    (0x28e3, get_ray_cast_material),
    (0x2911, set_angular_velocity_ex),
    (0x2914, set_collision_obj_property),
    (0x2916, set_collision_obj_layer_type),
    (0x2928, toggle_node_collision),
];

/// `hkpEntity::activate` (`00C9C1D0`; the port's `ENTITY_ACTIVATE` in `tesobjectrefr.rs`).
pub(crate) const ENTITY_ACTIVATE: u32 = 0x00c9_c1d0;
/// `TES` `00457720`: what lies under a point of the land (its +0x1C byte is the Havok material,
/// as the game's own `00457880` reads it through `00541570`).
pub(crate) const LAND_AT: u32 = 0x0045_7720;
/// `hkpRigidBody` fields stored (see `havok_refs.rs` and the read commands for where each is
/// confirmed).
const FRICTION: u32 = 0x90;
const RESTITUTION: u32 = 0x94;
const LINEAR_DAMPING: u32 = 0x194;
const ANGULAR_DAMPING: u32 = 0x198;
const INERTIA: u32 = 0x1a0;
pub(crate) const LIN_VELOCITY: u32 = 0x1b0;
pub(crate) const ANG_VELOCITY: u32 = 0x1c0;
/// `hkpWorldObject`: the owning wrapper (+0xC), the collision filter info's flags byte (+0x2D,
/// bit 0x40 no collision), the collidable's shape (`hkCdBody` at +0x10, its `m_shape` first).
pub(crate) const OWNER: u32 = 0x0c;
pub(crate) const FILTER_FLAGS: u32 = 0x2d;
pub(crate) const SHAPE: u32 = 0x10;
/// `hkpShape::userData` (+8, the `bhkShape`, stored by `bhkShape::Link` `00C84E50`) and the
/// `bhkShape`'s material word (+0x10).
pub(crate) const SHAPE_OWNER: u32 = 0x08;
pub(crate) const SHAPE_MATERIAL: u32 = 0x10;
/// Vtable slots: `bhkRigidBody::SetMotionType`, `bhkWorldObject::UpdateCollisionFilter`, the
/// rigid body's centre of mass in the world, `TESObjectREFR`'s `IsCharacter`.
pub(crate) const SET_MOTION_TYPE_SLOT: u32 = 0xe4;
pub(crate) const UPDATE_FILTER_SLOT: u32 = 0xc4;
pub(crate) const CENTER_OF_MASS_SLOT: u32 = 0xf0;
pub(crate) const IS_CHARACTER_SLOT: u32 = 0x218;
/// JIP's `LAYER_MAX` (internal/havok.h): the layers are 1 to 42.
const LAYER_MAX: u32 = 43;
/// `TESObjectREFR::position` (+0x30).
const POSITION: u32 = 0x30;
/// `TESNPC` and `TESCreature`'s blood impact material (`eBloodImpactMaterial`, +0x1E4 / +0x148).
const NPC_IMPACT_MATERIAL: u32 = 0x1e4;
const CREATURE_IMPACT_MATERIAL: u32 = 0x148;

// ---- stores ----

/// Four floats stored at `at`; false (nothing stored) when the 16 bytes are not writable.
fn store4(e: &Engine, at: u32, v: [f32; 4]) -> bool {
    let mut b = [0u8; 16];
    for (i, x) in v.iter().enumerate() {
        b[4 * i..4 * i + 4].copy_from_slice(&x.to_le_bytes());
    }
    e.mem.try_write(at, &b).is_ok()
}

/// The word at `at` stored; false when not writable.
fn store32(e: &Engine, at: u32, v: u32) -> bool {
    e.mem.try_write(at, &v.to_le_bytes()).is_ok()
}

/// The byte at `at` stored; false when not writable.
fn store8(e: &Engine, at: u32, v: u8) -> bool {
    e.mem.try_write(at, &[v]).is_ok()
}

/// `hkpEntity::activate(body)`: the game wakes the body's island (nothing without a world, for a
/// fixed body or an island that cannot be woken).
fn activate(e: &mut Engine, body: u32) {
    e.call(ENTITY_ACTIVATE, &args![body]);
}

// ---- velocities ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/havok.cpp: hkMatrix3x4::MultiplyVectorInv
// (asm).
/// The vector's x, y, z (its fourth lane cleared) times the rotation's columns, summed lane by
/// lane: `(c0*x + c1*y) + c2*z`, the fourth lane `(c0[3]*0 + c1[3]*0) + c2[3]*0`.
pub(crate) fn multiply_inv(cols: [[f32; 4]; 3], v: [f32; 4]) -> [f32; 4] {
    let lanes = |s: f32| [s, s, s, 0.0];
    let (x, y, z) = (lanes(v[0]), lanes(v[1]), lanes(v[2]));
    std::array::from_fn(|i| (cols[0][i] * x[i] + cols[1][i] * y[i]) + cols[2][i] * z[i])
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/havok.cpp: hkpRigidBody::SetVelocity
// (asm).
/// Flag 1 clear: the vector turned by the body's rotation ([`multiply_inv`]); flag 0x10 picks
/// the angular velocity (+0x1C0) over the linear (+0x1B0); flag 2 adds the stored vector (all
/// four lanes). The 16 bytes stored, then `hkpEntity::activate`. Only the low byte of `flags`
/// may be set (the caller checks).
fn set_velocity(e: &mut Engine, body: u32, vel: [f32; 4], flags: u32) {
    let v = if flags & 1 == 0 {
        match rotation(e, body) {
            Some(cols) => multiply_inv(cols, vel),
            None => return,
        }
    } else {
        vel
    };
    let at = body.wrapping_add(LIN_VELOCITY + (flags & 0x10));
    let v = if flags & 2 != 0 {
        let Some(old) = h::vec4(e, at) else {
            return;
        };
        std::array::from_fn(|i| v[i] + old[i])
    } else {
        v
    };
    if store4(e, at, v) {
        activate(e, body);
    }
}

/// The name, the three floats and the flags of `SetLinearVelocity` / `SetAngularVelocityEx`, and
/// the named block's mobile rigid body; `None` when the read failed, there is no such body, or
/// the flags have bits above the low byte (a deliberate difference, see the module).
fn velocity_args(e: &mut Engine, a: &CommandArgs, command: &str) -> Option<(u32, [f32; 4], u32)> {
    let read = Read::of(e, a);
    if !read.ok || a.this_obj == 0 {
        return None;
    }
    let name = name_arg(e, &read, 0, command);
    let vel = [
        read.float(1, 0.0),
        read.float(2, 0.0),
        read.float(3, 0.0),
        0.0,
    ];
    let flags = read.int(4, 0);
    let body = mobile_body(e, a.this_obj, &name);
    (body != 0 && flags <= 0xff).then_some((body, vel, flags))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetLinearVelocity_Execute.
/// The named block's mobile rigid body's linear velocity set (flag 1: given in the body's own
/// axes; flag 2: added), then woken; 1 when set.
fn set_linear_velocity(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some((body, vel, flags)) = velocity_args(e, &a, "SetLinearVelocity") {
        set_velocity(e, body, vel, flags ^ 1);
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetAngularVelocityEx_Execute.
/// The named block's mobile rigid body's angular velocity set (in the body's own axes unless
/// flag 1; flag 2: added), then woken; 1 when set.
fn set_angular_velocity_ex(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if let Some((body, vel, flags)) = velocity_args(e, &a, "SetAngularVelocityEx") {
        set_velocity(e, body, vel, flags | 0x10);
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetAngularVelocity_Execute (asm).
/// The named block's mobile rigid body's angular velocity set to its rotation's column of the
/// axis times the value (`(v, v, v, 0) * column`, the fourth lane `0 * c3`), then woken.
fn set_angular_velocity(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = name_arg(e, &read, 0, "SetAngularVelocity");
    let axis = axis_arg(&read, 1);
    let v = read.float(2, 0.0);
    let body = mobile_body(e, a.this_obj, &name);
    if body == 0 {
        return handled(true);
    }
    let Some(i) = axis.checked_sub(b'X').filter(|&i| i < 3) else {
        return handled(true);
    };
    let column = body.wrapping_add(h::ROTATION + 0x10 * u32::from(i));
    let Some(c) = h::vec4(e, column) else {
        return handled(true);
    };
    let s = [v, v, v, 0.0];
    let out = std::array::from_fn(|k| s[k] * c[k]);
    if store4(e, body.wrapping_add(ANG_VELOCITY), out) {
        activate(e, body);
    }
    handled(true)
}

// ---- pushes ----

/// `Normalize_V4` (JIP LN NVSE, internal/utility.cpp, asm): the four lanes over their length:
/// the squares summed `(x*x + y*y) + (z*z + w*w)`; below `FLT_EPSILON` (or NaN) the zero vector;
/// else each lane times `(3 - s*r*r) * r * 0.5` with `r` the processor's `rsqrtss` estimate.
pub(crate) fn normalize_v4(v: [f32; 4]) -> [f32; 4] {
    let s = (v[0] * v[0] + v[1] * v[1]) + (v[2] * v[2] + v[3] * v[3]);
    if s.is_nan() || s < f32::EPSILON {
        return [0.0; 4];
    }
    let r = super::ln_fn_math::rsqrt_estimate(s);
    let t = (s * r) * r;
    let u = ((3.0 - t) * r) * 0.5;
    v.map(|x| x * u)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/havok.cpp: bhkWorldObject::ApplyForce
// (asm).
/// The wrapper's centre of mass in the world (vtable +0xF0 into an aligned 16-byte buffer, read
/// where the call answers) in game units (`PS_HKUnitCnvrt` in three lanes, the fourth `0 * w`),
/// less the force point (its x, y, z); that direction normalized ([`normalize_v4`]) times the
/// force's strength (`w * kUnitConv` in three lanes, the fourth 0), added to the body's linear
/// velocity (four lanes), then `hkpEntity::activate`.
fn apply_force(e: &mut Engine, world: u32, body: u32, force: [f32; 4]) {
    let pos = e.with_stack(0x20, |e, frame| {
        let out = (frame.addr() + 0xf) & !0xf;
        let at = e.vcall(world, CENTER_OF_MASS_SLOT, &args![out]).u32();
        h::vec4(e, at)
    });
    let Some(pos) = pos else {
        return;
    };
    let k = h::from_havok();
    let scaled = [k * pos[0], k * pos[1], k * pos[2], 0.0 * pos[3]];
    let point = [force[0], force[1], force[2], 0.0];
    let dir = normalize_v4(std::array::from_fn(|i| scaled[i] - point[i]));
    let s = force[3] * h::to_havok();
    let strength = [s, s, s, 0.0];
    let at = body.wrapping_add(LIN_VELOCITY);
    let Some(old) = h::vec4(e, at) else {
        return;
    };
    let v = std::array::from_fn(|i| dir[i] * strength[i] + old[i]);
    if store4(e, at, v) {
        activate(e, body);
    }
}

/// A block's collision object (+0x1C), its world object (+0x10) and that one's Havok object
/// (+8); `None` when any is null.
fn world_and_object(e: &Engine, block: u32) -> Option<(u32, u32)> {
    let collision = h::ptr(e, block.wrapping_add(h::COLLISION_OBJECT))?;
    let world = h::ptr(e, collision.wrapping_add(h::WORLD_OBJECT))?;
    let object = h::ptr(e, world.wrapping_add(h::HAVOK_OBJECT))?;
    Some((world, object))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::ApplyForce (asm).
/// The force applied to the node's rigid body when it is mobile (motion type bit 1), then to
/// each child node's, depth first.
fn push_node(e: &mut Engine, node: u32, force: [f32; 4], depth: u32, walk: &mut h::Walk) {
    if depth >= h::MAX_DEPTH || walk.spent() {
        return;
    }
    if let Some((world, body)) = world_and_object(e, node) {
        if h::collision_type(e, body) == Some(1) && h::body_is_mobile(e, body) {
            apply_force(e, world, body, force);
        }
    }
    for child in h::child_nodes(e, node, walk) {
        if walk.spent() {
            return;
        }
        push_node(e, child, force, depth + 1, walk);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_PushObject_Execute.
/// For a reference that is not an actor and has 3D: the four floats (a point's offset and the
/// strength) with the given reference's position (or its own) added to the first three, applied
/// to every mobile rigid body under its root node.
fn push_object(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let this = a.this_obj;
    if this == 0 || h::is_actor(e, this) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let f = [0, 1, 2, 3].map(|i| read.float(i, 0.0));
    let refr = read.form(4);
    let root = h::ref_ni_node(e, this);
    if root == 0 {
        return handled(true);
    }
    let refr = if refr == 0 { this } else { refr };
    let p = [0, 4, 8].map(|o| h::rdf32(e, refr.wrapping_add(POSITION + o)));
    let [Some(px), Some(py), Some(pz)] = p else {
        return handled(true);
    };
    let force = [f[0] + px, f[1] + py, f[2] + pz, f[3] + 0.0];
    push_node(e, root, force, 0, &mut h::Walk::new());
    handled(true)
}

// ---- collision properties and layers ----

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/havok.cpp:
// hkpRigidBody::SetCollisionProperty.
/// Property 0 to 7 stored as its bits (friction, restitution, linear and angular damping, the
/// three inertia floats, the inverse mass); 8, when the motion type byte differs, passed to the
/// owning wrapper's `SetMotionType` (only a type from 1 to 7, see the module).
fn set_property(e: &mut Engine, body: u32, prop: u32, value: u32) {
    let at = match prop {
        0 => FRICTION,
        1 => RESTITUTION,
        2 => LINEAR_DAMPING,
        3 => ANGULAR_DAMPING,
        4..=6 => INERTIA + 4 * (prop - 4),
        7 => h::MASS_INV,
        _ => {
            let Some(current) = h::rd8(e, body.wrapping_add(h::MOTION_TYPE)) else {
                return;
            };
            if u32::from(current) != value && (1..=7).contains(&value) {
                if let Some(owner) = h::ptr(e, body.wrapping_add(OWNER)) {
                    e.vcall(owner, SET_MOTION_TYPE_SLOT, &args![value]);
                }
            }
            return;
        }
    };
    store32(e, body.wrapping_add(at), value);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp:
// NiNode::SetCollisionPropRecurse (asm).
/// The property set on the node's rigid body (Havok object of type 1), then on each child
/// node's, depth first.
fn set_property_recurse(
    e: &mut Engine,
    node: u32,
    prop: u32,
    value: u32,
    depth: u32,
    walk: &mut h::Walk,
) {
    if depth >= h::MAX_DEPTH || walk.spent() {
        return;
    }
    if let Some((_, body)) = world_and_object(e, node) {
        if h::collision_type(e, body) == Some(1) {
            set_property(e, body, prop, value);
        }
    }
    for child in h::child_nodes(e, node, walk) {
        if walk.spent() {
            return;
        }
        set_property_recurse(e, child, prop, value, depth + 1, walk);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetCollisionObjProperty_Execute.
/// Property 0 to 8 of the named block's rigid body, or with the name `*` of every rigid body
/// under the root node: the float as given; for 7 (the mass) its inverse (0 when not above 0);
/// for 8 (the motion type) truncated to an integer.
fn set_collision_obj_property(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = name_arg(e, &read, 0, "SetCollisionObjProperty");
    let prop = read.int(1, 0);
    let f = read.float(2, 0.0);
    if prop > 8 {
        return handled(true);
    }
    let value = match prop {
        7 if f > 0.0 => (1.0 / f).to_bits(),
        7 => 0,
        8 => super::cvttss2si(f) as u32,
        _ => f.to_bits(),
    };
    if name == b"*" {
        let root = h::ref_ni_node(e, a.this_obj);
        if root != 0 {
            set_property_recurse(e, root, prop, value, 0, &mut h::Walk::new());
        }
    } else {
        let body = h::rigid_body(e, a.this_obj, &name);
        if body != 0 {
            set_property(e, body, prop, value);
        }
    }
    handled(true)
}

/// The layer stored in the node's world object's filter info (low byte, +0x2C) when it differs,
/// then that world object's `UpdateCollisionFilter`.
fn set_layer(e: &mut Engine, node: u32, layer: u32) {
    let Some((world, object)) = world_and_object(e, node) else {
        return;
    };
    let at = object.wrapping_add(h::FILTER_INFO);
    if h::rd8(e, at).is_some_and(|b| u32::from(b) != layer) && store8(e, at, layer as u8) {
        e.vcall(world, UPDATE_FILTER_SLOT, &args![]);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/netimmerse.cpp: NiNode::SetCollisionLayer
// (asm).
/// [`set_layer`] on the node, then on each child node, depth first.
fn set_layer_recurse(e: &mut Engine, node: u32, layer: u32, depth: u32, walk: &mut h::Walk) {
    if depth >= h::MAX_DEPTH || walk.spent() {
        return;
    }
    set_layer(e, node, layer);
    for child in h::child_nodes(e, node, walk) {
        if walk.spent() {
            return;
        }
        set_layer_recurse(e, child, layer, depth + 1, walk);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetCollisionObjLayerType_Execute.
/// A layer (1 to 42) set on the node named under the root (`NiNode::GetNode`: none for an empty
/// name), or with the name `*` on the root and every node under it.
fn set_collision_obj_layer_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = name_arg(e, &read, 0, "SetCollisionObjLayerType");
    let layer = read.int(1, 0);
    if layer == 0 || layer >= LAYER_MAX {
        return handled(true);
    }
    let root = h::ref_ni_node(e, a.this_obj);
    if root == 0 {
        return handled(true);
    }
    if name == b"*" {
        set_layer_recurse(e, root, layer, 0, &mut h::Walk::new());
    } else {
        let block = h::get_block(e, root, &name);
        if block != 0 && h::is_node(e, block) {
            set_layer(e, block, layer);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_ToggleNodeCollision_Execute.
/// The named node's (`GetNode`: the seen root for an empty name) world object's no-collision
/// flag (bit 0x40 of the filter flags byte, +0x2D) set when `enable` is 0 and cleared otherwise,
/// with no filter update, as JIP stores it. (The game's own toggle, `00C85D20`, stores the same
/// bit and then calls the world object's `UpdateCollisionFilter`; JIP's command does not.)
fn toggle_node_collision(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = name_arg(e, &read, 0, "ToggleNodeCollision");
    let enable = read.int(1, 0);
    let node = h::node(e, a.this_obj, &name);
    if node == 0 {
        return handled(true);
    }
    if let Some((_, object)) = world_and_object(e, node) {
        let at = object.wrapping_add(FILTER_FLAGS);
        if let Some(flags) = h::rd8(e, at) {
            store8(e, at, (flags & 0xbf) | (u8::from(enable == 0) << 6));
        }
    }
    handled(true)
}

// ---- the ray cast's material ----

/// The script's material number of a Havok material (0 to 31, `kMaterialConvert`, the game's
/// `0058E8F0`), -1 past 31.
fn convert_material(m: u32) -> i32 {
    super::jip_fn_projectile::MATERIAL_CONVERT
        .get(m as usize)
        .map_or(-1, |&c| i32::from(c))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetRayCastMaterial (asm).
/// What the cast hits: -1 for nothing. A block without a reference (the land): the hit point in
/// game units ([`h::hit_point`]) to the game's land lookup, its +0x1C byte converted, -1 when it
/// finds nothing. An actor: its base's blood impact material (a character's `TESNPC` +0x1E4,
/// else `TESCreature` +0x148), the whole word as it is. Else the hit block's Havok object's
/// shape's `bhkShape` material word, converted.
fn ray_cast_material(e: &mut Engine, pos: [f32; 4], rot: [f32; 9], layer: i32) -> i32 {
    let Some(cast) = h::ray_cast(e, pos, rot, 50000.0, layer) else {
        return -1;
    };
    if cast.object == 0 {
        return -1;
    }
    let refr = h::block_parent_ref(e, cast.object, &mut h::Walk::new());
    if refr == 0 {
        let Some(tes) = h::ptr(e, h::TES) else {
            return -1;
        };
        let point = h::hit_point(&cast);
        let found = e.with_stack(0x20, |e, frame| {
            let at = (frame.addr() + 0xf) & !0xf;
            if !store4(e, at, point) {
                return 0;
            }
            e.call(LAND_AT, &args![tes, at]).u32()
        });
        if found == 0 {
            return -1;
        }
        return h::rd8(e, found.wrapping_add(0x1c)).map_or(-1, |m| convert_material(u32::from(m)));
    }
    if h::is_actor(e, refr) {
        let character = e.vcall(refr, IS_CHARACTER_SLOT, &args![]).bool();
        let Some(base) = h::ptr(e, refr.wrapping_add(0x20)) else {
            return -1;
        };
        let at = if character {
            NPC_IMPACT_MATERIAL
        } else {
            CREATURE_IMPACT_MATERIAL
        };
        return h::rd32(e, base.wrapping_add(at)).map_or(-1, |m| m as i32);
    }
    let material = world_and_object(e, cast.object)
        .and_then(|(_, object)| h::ptr(e, object.wrapping_add(SHAPE)))
        .and_then(|shape| h::ptr(e, shape.wrapping_add(SHAPE_OWNER)))
        .and_then(|owner| h::rd32(e, owner.wrapping_add(SHAPE_MATERIAL)));
    material.map_or(-1, convert_material)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_GetRayCastMaterial_Execute.
/// The material (see [`ray_cast_material`]) of what the named node (`GetNode`; the seen root for
/// an empty name) looks at, cast 50000 units on the layer (6); -1 when nothing.
fn get_ray_cast_material(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut material = -1;
    if read.ok && a.this_obj != 0 {
        let layer = read.int(0, 6) as i32;
        let name = name_arg(e, &read, 1, "GetRayCastMaterial");
        let node = h::node(e, a.this_obj, &name);
        if let Some((pos, rot)) = (node != 0).then(|| node_transform(e, node)).flatten() {
            material = ray_cast_material(e, pos, rot, layer);
        }
    }
    a.set_result(e, f64::from(material));
    handled(true)
}
