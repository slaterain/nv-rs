//! A shadow scene light's walk over its objects: `AddObject` decides between the frustum walk (a
//! light that casts shadows), the radius walk and a plain geometry add; the walks test each object's
//! bound against the light, recurse into a node's children and add the tri-based geometries they
//! reach to the light's list (`AddTriGeom`), which also hands the light to the geometry's lighting
//! property. The exe's calls within this group are plain calls, and so are these.

use super::ShadowSceneLight;
use engine_core::ptr::{Field, Ptr};
use engine_core::types::NiTListItem;
use engine_core::{args, Engine};

/// `NiAVObject` members (Xbox PDB names, PC offsets): `m_kWorldBound` (+0x20, its radius at +0xc),
/// `m_uFlags` (+0x30), the world translation (+0x8c); `NiNode::m_kChildren`' buffer (+0xa0) and
/// `m_usSize` (+0xa6); a geometry's lighting property (+0xa8, its type at +0x1c).
const WORLD_BOUND: u32 = 0x20;
const FLAGS: u32 = 0x30;
const WORLD_TRANSLATE: u32 = 0x8c;
const CHILDREN_BASE: u32 = 0xa0;
const CHILDREN_SIZE: u32 = 0xa6;
const GEOMETRY_PROPERTY: u32 = 0xa8;
/// Virtual slots: the RTTI (+8), `IsNode` (+0xc), `IsTriBasedGeom` (+0x1c).
const SLOT_RTTI: u32 = 8;
const SLOT_IS_NODE: u32 = 0xc;
const SLOT_IS_TRI_BASED_GEOM: u32 = 0x1c;
/// The null bound (`011f4288`), the scene node pointer (`011f91c8`, its lighting offset at
/// +0x1e4) and the `NiRTTI` record the radius test accepts outright (`011f4a68`).
const NULL_BOUND: u32 = 0x011f_4288;
const SCENE_NODE: u32 = 0x011f_91c8;
const LIGHTING_OFFSET: u32 = 0x1e4;
const RADIUS_FREE_RTTI: u32 = 0x011f_4a68;
/// `AddTriGeom`'s callees and data: the function-local static `NiPointer` (`011fffe0`, guard bit 0
/// of `011fffe4`, destructor `00fdce10` given to `_atexit` `00ec658f`), the geometry test
/// `00b4f580` (on the geometry's +0xc0, with the record at `0126f554`), the property test
/// `004a2020` (with 0xc), the property's light add `00b71560` (the light and a bound), the list's
/// add at the head `00c1b4a0` (an `NiPointer`'s address) and move `00b702e0`.
const ADD_STATIC: u32 = 0x011f_ffe0;
const ADD_GUARD: u32 = 0x011f_ffe4;
const ADD_STATIC_DESTRUCT: u32 = 0x00fd_ce10;
const ATEXIT: u32 = 0x00ec_658f;
const GEOMETRY_TEST: u32 = 0x00b4_f580;
const GEOMETRY_TEST_RECORD: u32 = 0x0126_f554;
const PROPERTY_TEST: u32 = 0x004a_2020;
const PROPERTY_ADD_LIGHT: u32 = 0x00b7_1560;
const LIST_ADD: u32 = 0x00c1_b4a0;
const LIST_MOVE: u32 = 0x00b7_02e0;

/// The bound of an object, or the null bound.
fn bound_or_null(e: &Engine, object: u32) -> u32 {
    match e.mem.u32(object.wrapping_add(WORLD_BOUND)) {
        0 => NULL_BOUND,
        bound => bound,
    }
}

/// A child's bound test of the walks: flag 0x800 passes it; otherwise a missing bound or a zero
/// radius (`FLDZ; FCOMP`: a NaN passes) rejects it.
fn child_bound_passes(e: &Engine, child: u32) -> bool {
    if (e.mem.u32(child.wrapping_add(FLAGS)) >> 0xb) & 1 != 0 {
        return true;
    }
    let bound = e.mem.u32(child.wrapping_add(WORLD_BOUND));
    bound != 0 && e.mem.f32(bound.wrapping_add(0xc)) != 0.0
}

/// The address of a field of the light.
fn at<T>(this: Ptr<ShadowSceneLight>, field: Field<ShadowSceneLight, T>) -> u32 {
    this.addr().wrapping_add(field.off)
}

/// The float at `at`.
fn f(e: &Engine, at: u32) -> f32 {
    e.mem.f32(at)
}

/// The squared length `(y * y + x * x) + z * z`, in the exe's order.
fn squared_length(x: f32, y: f32, z: f32) -> f32 {
    (y * y + x * x) + z * z
}

// Typed from 00b9bbb0
/// `ShadowSceneLight::BoundWithinLightFrustum` (`thiscall(light, bound)`): for each active plane of
/// `kShadowCameraPlanes` (bit `i` of the mask at +0x1a4), the signed distance of the bound's centre
/// (`(n.y * c.y + n.x * c.x) + c.z * n.z`, stored, less `d`, stored) answers 0 unless it is above
/// `-radius` (a NaN passes); a plane the whole bound is inside (the distance at least the radius)
/// is dropped from the mask. Answers 1 otherwise.
pub fn bound_within_frustum(e: &mut Engine, this: Ptr<ShadowSceneLight>, bound: Ptr) -> u8 {
    let mask_at = at(this, ShadowSceneLight::planes_active);
    let b = bound.addr();
    for plane in 0..6u32 {
        let bit = 1u32 << plane;
        if e.mem.u32(mask_at) & bit == 0 {
            continue;
        }
        let n = this.addr().wrapping_add(0x144 + 0x10 * plane);
        let y = f(e, n.wrapping_add(4)) * f(e, b.wrapping_add(4));
        let x = f(e, n) * f(e, b);
        let z = f(e, b.wrapping_add(8)) * f(e, n.wrapping_add(8));
        let dot = (y + x) + z;
        let distance = dot - f(e, n.wrapping_add(0xc));
        let radius = f(e, b.wrapping_add(0xc));
        if -radius >= distance {
            return 0;
        }
        if radius <= distance {
            let mask = e.mem.u32(mask_at);
            e.mem.set_u32(mask_at, mask & !bit);
        }
    }
    1
}

// Typed from 00b9dae0
/// `ShadowSceneLight::BoundWithinLightRadius` (`thiscall(light, bound, NiLight, scale)`, the light's
/// own `spLight` for a null `NiLight`): a light whose RTTI is the record at `011f4a68` is always in
/// range. Otherwise the distance from the bound's centre to the `NiLight`'s position moved by the
/// scene node's lighting offset (each difference and sum stored as the exe stores them), less the
/// bound's radius times the scale, must be below the `NiLight`'s radius (+0xe0).
pub fn bound_within_radius(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    bound: Ptr,
    ni_light: u32,
    scale: f32,
) -> u8 {
    let light = if ni_light == 0 {
        e.get(this, ShadowSceneLight::spLight)
    } else {
        ni_light
    };
    if e.vcall(light, SLOT_RTTI, &args![]).u32() == RADIUS_FREE_RTTI {
        return 1;
    }
    let node = e.global::<u32>(SCENE_NODE);
    let b = bound.addr();
    let moved: [f32; 3] = [0u32, 1, 2].map(|i| {
        let delta = f(e, light.wrapping_add(WORLD_TRANSLATE + 4 * i)) - f(e, b.wrapping_add(4 * i));
        f(e, node.wrapping_add(LIGHTING_OFFSET + 4 * i)) + delta
    });
    let squared = squared_length(moved[0], moved[1], moved[2]);
    let distance = squared.sqrt();
    let gap = distance - f(e, b.wrapping_add(0xc)) * scale;
    u8::from(f(e, light.wrapping_add(0xe0)) > gap)
}

// Typed from 00b9f480
/// `ShadowSceneLight::AddTriGeom` (`thiscall(light, geometry)`): sets up its static `NiPointer` on
/// the first call. Only for a geometry with a +0xc0 object that passes `00b4f580` and a lighting
/// property of type 1 to 12 (-1 for none): when the geometry is in `lGeomList` from the fence (or
/// the head) on, its item moves before the fence (`00b702e0`, when there is one) and the property
/// gets the light (`00b71560` with the geometry's bound or the null bound); otherwise, when the
/// property's `004a2020` (0xc) fails or its type is 4, the property gets the light and the geometry
/// is added at the list's head through the static (`00c1b4a0`).
pub fn add_tri_geom(e: &mut Engine, this: Ptr<ShadowSceneLight>, geometry: u32) {
    let guard: u32 = e.global(ADD_GUARD);
    if guard & 1 == 0 {
        e.set_global(ADD_GUARD, guard | 1);
        e.set_global(ADD_STATIC, 0u32);
        e.call(ATEXIT, &args![ADD_STATIC_DESTRUCT]);
    }
    let object = e.mem.u32(geometry.wrapping_add(0xc0));
    if object == 0
        || e.call(GEOMETRY_TEST, &args![object, GEOMETRY_TEST_RECORD])
            .u32()
            == 0
    {
        return;
    }
    if geometry.wrapping_add(0x9c) == 0 {
        return;
    }
    let property = e.mem.u32(geometry.wrapping_add(GEOMETRY_PROPERTY));
    let kind = if property != 0 {
        e.mem.u32(property.wrapping_add(0x1c))
    } else {
        u32::MAX
    };
    if kind.wrapping_sub(1) > 0xb {
        return;
    }
    let list = at(this, ShadowSceneLight::lGeomList);
    let fence = e.get(this, ShadowSceneLight::kGeomListFence);
    let mut item = if fence != 0 { fence } else { e.mem.u32(list) };
    let mut found = 0;
    while item != 0 {
        let at = Ptr::<NiTListItem>::new(item);
        let next = e.get(at, NiTListItem::m_pkNext);
        if e.get(at, NiTListItem::m_element) == geometry {
            found = item;
            break;
        }
        item = next;
    }
    if found != 0 {
        if fence != 0 {
            e.call(LIST_MOVE, &args![list, found, fence]);
        }
        let bound = bound_or_null(e, geometry);
        e.call(PROPERTY_ADD_LIGHT, &args![property, this, bound]);
        return;
    }
    if e.call(PROPERTY_TEST, &args![property, 0xcu32]).u8() != 0 && kind != 4 {
        return;
    }
    e.set_global(ADD_STATIC, geometry);
    let bound = bound_or_null(e, geometry);
    e.call(PROPERTY_ADD_LIGHT, &args![property, this, bound]);
    e.call(LIST_ADD, &args![list, ADD_STATIC]);
    e.set_global(ADD_STATIC, 0u32);
}

/// The children of a node the walks recurse into: not null, a usable bound (or flag 0x800), not
/// culled (flag 1). The count is read again before each child, as the exe does.
fn each_child(e: &mut Engine, node: u32, mut visit: impl FnMut(&mut Engine, u32)) {
    let mut index = 0u32;
    while index < u32::from(e.mem.u16(node.wrapping_add(CHILDREN_SIZE))) {
        let children = e.mem.u32(node.wrapping_add(CHILDREN_BASE));
        let child = e.mem.u32(children.wrapping_add(4 * index));
        if child != 0
            && child_bound_passes(e, child)
            && e.mem.u32(child.wrapping_add(FLAGS)) & 1 == 0
        {
            visit(e, child);
        }
        index += 1;
    }
}

// Typed from 00b9f5b0
/// `ShadowSceneLight::ParseObjectAddFrustum` (`thiscall(light, object)`): not for the light's
/// shadow scene or an object with flag 0x800000. With planes active, an object without flag 0x800
/// must be within the frustum ([`bound_within_frustum`]). A node's children are parsed the same
/// way; a tri-based geometry is added ([`add_tri_geom`]). The plane mask is restored at the end.
pub fn parse_frustum(e: &mut Engine, this: Ptr<ShadowSceneLight>, object: u32) {
    let mask_at = at(this, ShadowSceneLight::planes_active);
    let saved = e.mem.u32(mask_at);
    parse_frustum_body(e, this, object, saved);
    e.mem.set_u32(mask_at, saved);
}

fn parse_frustum_body(e: &mut Engine, this: Ptr<ShadowSceneLight>, object: u32, saved: u32) {
    if object == e.get(this, ShadowSceneLight::spShadowScene) {
        return;
    }
    let flags = e.mem.u32(object.wrapping_add(FLAGS));
    if (flags >> 0x17) & 1 != 0 {
        return;
    }
    if saved != 0
        && (flags >> 0xb) & 1 == 0
        && bound_within_frustum(e, this, Ptr::new(bound_or_null(e, object))) == 0
    {
        return;
    }
    let node = e.vcall(object, SLOT_IS_NODE, &args![]).u32();
    if node != 0 {
        each_child(e, node, |e, child| parse_frustum(e, this, child));
    } else if e.vcall(object, SLOT_IS_TRI_BASED_GEOM, &args![]).u32() != 0 {
        add_tri_geom(e, this, object);
    }
}

// Typed from 00b9f6a0
/// `ShadowSceneLight::ParseObjectAddRadius` (`thiscall(light, object)`): an object without flag
/// 0x800 must be within the light's radius ([`bound_within_radius`] with its `NiLight` and 1.0).
/// A node's children are parsed the same way; a tri-based geometry is added with what its second
/// `IsTriBasedGeom` answers. The plane mask is restored at the end.
pub fn parse_radius(e: &mut Engine, this: Ptr<ShadowSceneLight>, object: u32) {
    let mask_at = at(this, ShadowSceneLight::planes_active);
    let saved = e.mem.u32(mask_at);
    parse_radius_body(e, this, object);
    e.mem.set_u32(mask_at, saved);
}

fn parse_radius_body(e: &mut Engine, this: Ptr<ShadowSceneLight>, object: u32) {
    let flags = e.mem.u32(object.wrapping_add(FLAGS));
    if (flags >> 0xb) & 1 == 0 {
        let bound = bound_or_null(e, object);
        let light = e.get(this, ShadowSceneLight::spLight);
        if bound_within_radius(e, this, Ptr::new(bound), light, 1.0) == 0 {
            return;
        }
    }
    let node = e.vcall(object, SLOT_IS_NODE, &args![]).u32();
    if node != 0 {
        each_child(e, node, |e, child| parse_radius(e, this, child));
    } else if e.vcall(object, SLOT_IS_TRI_BASED_GEOM, &args![]).u32() != 0 {
        let geometry = e.vcall(object, SLOT_IS_TRI_BASED_GEOM, &args![]).u32();
        add_tri_geom(e, this, geometry);
    }
}

// Typed from 00ba0110
/// `ShadowSceneLight::AddObject` (`thiscall(light, object)`): a node that is not culled (flag 1)
/// and has a bound with a radius is parsed by frustum (a light with `bCastShadow`) or by radius,
/// except the light's own shadow scene without flag 0x800000, which goes the geometry way; that way
/// (also for anything else) adds a tri-based geometry ([`add_tri_geom`]).
pub fn add_object(e: &mut Engine, this: Ptr<ShadowSceneLight>, object: u32) {
    if object == 0 {
        return;
    }
    let node = e.vcall(object, SLOT_IS_NODE, &args![]).u32();
    let flags = e.mem.u32(object.wrapping_add(FLAGS));
    let parse = node != 0 && flags & 1 == 0 && {
        let bound = e.mem.u32(node.wrapping_add(WORLD_BOUND));
        bound != 0 && f(e, bound.wrapping_add(0xc)) != 0.0
    };
    if parse {
        let cast = e.get(this, ShadowSceneLight::bCastShadow);
        let shadow_scene = cast != 0 && object == e.get(this, ShadowSceneLight::spShadowScene);
        let hidden = (e.mem.u32(object.wrapping_add(FLAGS)) >> 0x17) & 1 != 0;
        if !(shadow_scene && !hidden) {
            if cast != 0 {
                parse_frustum(e, this, node);
            } else {
                parse_radius(e, this, node);
            }
            return;
        }
    }
    if e.vcall(object, SLOT_IS_TRI_BASED_GEOM, &args![]).u32() != 0 {
        add_tri_geom(e, this, object);
    }
}
