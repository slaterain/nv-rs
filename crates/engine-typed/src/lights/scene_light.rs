//! A shadow scene light's own functions beside its walk: its
//! constructor and light set-up, its two luminances (to a bound and at a point, the second also
//! with a facing term), its shadow flag, its geometry list's fence clear, head and tail removals
//! and geometry removal, the `NiPointer` assignment at +0x1a0 of its accumulator and the cull of
//! its geometries.
//!
//! Done in place: the `NiPointer` counts (`InterlockedIncrement` / `InterlockedDecrement` as direct
//! import calls), the lights' RTTI slot when it holds one of the exe's constant bodies
//! ([`slot_word`]), `NiPlane`'s constructor and the `memset` of the constructor, the C runtime's
//! square root of a stored float (one correctly rounded root, as `FSQRT` then the float store gives
//! under either control word), and the facing luminance's point length, unit vectors and clamps.
//! The object cull is the culling family's. What stays by address: the list item free (`00e6e830`),
//! the renderer's shadow map release (`00b6d4c0`), the lighting property's light removal and the
//! list removals, the cull pass's callback, setters and the renderer's busy test.
//!
//! **Floats.** The luminances run on the main thread (the light queue) and on others (the light
//! sort of the render passes on the loading thread; the AI's light queries): each is written once
//! over [`Real`], every step and store as the exe makes them, and its Diff tests run both control
//! words.

use super::walk::slot_word;
use super::ShadowSceneLight;
use crate::native::{float_precision, Real};
use engine_core::ptr::{Field, Ptr};
use engine_core::sync_direct::{interlocked_decrement, interlocked_increment};
use engine_core::types::{NiTListItem, NiTPointerList};
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The RTTI slot (+8) and the records a light's kind is tested against: `NiPointLight`
/// (`011f4a98`) and `NiAmbientLight` (`011f5f6c`).
const SLOT_RTTI: u32 = 8;
const NI_POINT_LIGHT_RTTI: u32 = 0x011f_4a98;
const NI_AMBIENT_LIGHT_RTTI: u32 = 0x011f_5f6c;
/// An `NiRefObject`'s deleting slot (+4, no stack words).
const SLOT_DELETE: u32 = 4;
/// `NiLight` members (PC offsets): the world translation (+0x8c), the dimmer (+0xc4), the diffuse
/// colour (+0xd4) and the radius (+0xe0); `NiAVObject::m_uFlags` (+0x30, bit 0 culled).
const WORLD_TRANSLATE: u32 = 0x8c;
const DIMMER: u32 = 0xc4;
const COLOUR: u32 = 0xd4;
const RADIUS: u32 = 0xe0;
const FLAGS: u32 = 0x30;
/// A geometry's lighting property (+0xa8, its type at +0x1c) and its +0xc0 property.
const GEOMETRY_PROPERTY: u32 = 0xa8;
const GEOMETRY_C0: u32 = 0xc0;
/// The scene node (`011f91c8`) and its lighting offset (+0x1e4).
const SCENE_NODE: u32 = 0x011f_91c8;
const LIGHTING_OFFSET: u32 = 0x1e4;

/// An `NiPointer`'s release as the exe inlines it: the count (+4) down, the object deleted through
/// its vtable at zero.
fn release(e: &mut Engine, object: u32) {
    if interlocked_decrement(e, object.wrapping_add(4)) == 0 {
        e.vcall(object, SLOT_DELETE, &args![]);
    }
}

/// An `NiPointer`'s hold: the count (+4) up.
fn hold(e: &mut Engine, object: u32) {
    interlocked_increment(e, object.wrapping_add(4));
}

/// The inlined kind-of test: the object's RTTI ([`slot_word`]), then the chain of base records
/// (each `NiRTTI`'s +4) until `rtti` or null.
fn is_kind_of(e: &mut Engine, object: u32, rtti: u32) -> bool {
    let mut record = slot_word(e, object, SLOT_RTTI);
    while record != 0 {
        if record == rtti {
            return true;
        }
        record = e.mem.u32(record.wrapping_add(4));
    }
    false
}

/// The lighting property when its type is 1 to 12 (the inlined `(type - 1) <= 0xb`), else 0.
fn lighting_property(e: &Engine, geometry: u32) -> u32 {
    let property = e.mem.u32(geometry.wrapping_add(GEOMETRY_PROPERTY));
    if property != 0 && e.mem.u32(property.wrapping_add(0x1c)).wrapping_sub(1) <= 0xb {
        property
    } else {
        0
    }
}

/// The words of an array field from its offset on.
fn set_words<const N: usize>(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    field: Field<ShadowSceneLight, [u32; N]>,
    words: [u32; N],
) {
    let at = this.addr().wrapping_add(field.off);
    for (i, word) in (0u32..).zip(words) {
        e.mem.set_u32(at.wrapping_add(4 * i), word);
    }
}

/// Three words from `at`.
fn three(e: &Engine, at: u32) -> [u32; 3] {
    [0u32, 4, 8].map(|i| e.mem.u32(at.wrapping_add(i)))
}

// Typed from 00b9dda0
/// `ShadowSceneLight::SetLight` (`thiscall(light, NiLight)`): the `NiPointer` assignment of
/// `spLight` (nothing for the same light); `bPointLight` is whether the light is an `NiPointLight`
/// (none: no). Not one: `fLODDimmer` 1.0; one with `bDynamicLight`: its position copied to
/// `bPointPosition`. Then `bAmbientLight` is whether it is an `NiAmbientLight`.
pub fn set_light(e: &mut Engine, this: Ptr<ShadowSceneLight>, light: Ptr) {
    let light = light.addr();
    let old = e.get(this, ShadowSceneLight::spLight);
    if old != light {
        if old != 0 {
            release(e, old);
        }
        e.set(this, ShadowSceneLight::spLight, light);
        if light != 0 {
            hold(e, light);
        }
    }
    let point = light != 0 && is_kind_of(e, light, NI_POINT_LIGHT_RTTI);
    e.set(this, ShadowSceneLight::bPointLight, u8::from(point));
    if !point {
        e.set(this, ShadowSceneLight::fLODDimmer, 1.0);
    } else if e.get(this, ShadowSceneLight::bDynamicLight) != 0 {
        let position = three(e, light.wrapping_add(WORLD_TRANSLATE));
        set_words(e, this, ShadowSceneLight::bPointPosition, position);
    }
    let ambient = light != 0 && is_kind_of(e, light, NI_AMBIENT_LIGHT_RTTI);
    e.set(this, ShadowSceneLight::bAmbientLight, u8::from(ambient));
}

/// The constructor's data: `NiRefObject`'s vtable (`0101dce4`, stored first) and `ShadowSceneLight`'s (`010b7eb8`), `NiRefObject::ms_uiObjects`
/// (`011f4400`), the zero point (`011f426c`, what `NiPlane`'s constructor `00a69940` and
/// `bPointPosition` copy), the `float` `fSpotlightFOV` starts from (`010249d8`), and the
/// `BSSimpleArray` vtables (`010b7ea8` for the rooms and portals, `0101a8e8` for the nodes).
const SHADOW_SCENE_LIGHT_VTABLE: u32 = 0x010b_7eb8;
const NI_REF_OBJECT_VTABLE: u32 = 0x0101_dce4;
const NI_REF_OBJECT_COUNT: u32 = 0x011f_4400;
const ZERO_POINT: u32 = 0x011f_426c;
const SPOTLIGHT_FOV: u32 = 0x0102_49d8;
const ROOMS_ARRAY_VTABLE: u32 = 0x010b_7ea8;
const NODES_ARRAY_VTABLE: u32 = 0x0101_a8e8;

/// The identity matrix `kViewProjection` starts as, by rows.
const IDENTITY: [u32; 16] = {
    let mut m = [0u32; 16];
    let mut i = 0;
    while i < 4 {
        m[5 * i] = 0x3f80_0000;
        i += 1;
    }
    m
};

// Typed from 00b9fda0
/// `ShadowSceneLight::ShadowSceneLight` (`thiscall(this)`): one more live `NiRefObject`, a count of
/// 0, the class's vtable; the lists and pointers cleared (the exe's later releases of those cleared
/// pointers find nothing); the six planes of `kShadowCameraPlanes` the zero point with distance 0.0,
/// all six active; the three arrays empty; the luminance and dimmers 0.0, the flags clear,
/// `bPointPosition` the zero point, `usFrustumCull` 0; `kViewProjection` the identity;
/// `fSpotlightFOV` from `010249d8`, `fSpotlightFalloff` 1.0; the shadow dimmer's elapsed time 0.0
/// and target 1.0; the clip planes not ready and zeroed. Answers `this`.
pub fn construct(e: &mut Engine, this: Ptr<ShadowSceneLight>) -> Ptr<ShadowSceneLight> {
    type S = ShadowSceneLight;
    // The `NiRefObject` part first: its vtable and count are what the import call could see.
    e.set(this, S::vtable, NI_REF_OBJECT_VTABLE);
    e.set(this, S::m_uiRefCount, 0);
    interlocked_increment(e, NI_REF_OBJECT_COUNT);
    e.set(this, S::vtable, SHADOW_SCENE_LIGHT_VTABLE);
    set_words(e, this, S::lGeomList, [0; 3]);
    for pointer in [
        S::spSpecularAttenuationTexture,
        S::spLight,
        S::spDynamicShadowMap,
        S::spFrustumVis,
        S::spShadowScene,
        S::kGeomListFenceObject,
        S::spShadowCamera,
        S::kGeomListFence,
        S::spShadowAccumulator,
    ] {
        e.set(this, pointer, 0);
    }
    set_words(e, this, S::lRootList, [0; 3]);
    let zero = three(e, ZERO_POINT);
    let mut planes = [0u32; 24];
    for plane in planes.chunks_exact_mut(4) {
        plane[..3].copy_from_slice(&zero);
    }
    set_words(e, this, S::planes, planes);
    e.set(this, S::planes_active, 0x3f);
    for (array, vtable) in [
        (S::kMultiboundRooms, ROOMS_ARRAY_VTABLE),
        (S::kPortals, ROOMS_ARRAY_VTABLE),
        (S::kProcessedNodes, NODES_ARRAY_VTABLE),
    ] {
        set_words(e, this, array, [vtable, 0, 0, 0]);
    }
    for dimmer in [S::fLuminance, S::fLODDimmer, S::fShadowLODDimmer] {
        e.set(this, dimmer, 0.0);
    }
    for flag in [
        S::bCubeShadowMap,
        S::bCastShadow,
        S::bPointLight,
        S::bAmbientLight,
        S::bDynamicLight,
        S::bShadowSpotlight,
        S::bShowDebugTexture,
        S::bClipSpacePlanesReady,
    ] {
        e.set(this, flag, 0);
    }
    set_words(e, this, S::bPointPosition, zero);
    e.set(this, S::usFrustumCull, 0);
    set_words(e, this, S::kViewProjection, IDENTITY);
    let fov = x87_quiet_bits(e.global::<u32>(SPOTLIGHT_FOV));
    e.set(this, S::fSpotlightFOV, f32::from_bits(fov));
    e.set(this, S::fSpotlightFalloff, 1.0);
    e.set(this, S::fShadowLODDimmerElapsed, 0.0);
    e.set(this, S::fShadowLODDimmerTarget, 1.0);
    set_words(e, this, S::pfClipSpacePlanes, [0; 24]);
    this
}

/// The list item free (`00e6e830`, `thiscall` on the list: the item).
const LIST_FREE_ITEM: u32 = 0x00e6_e830;

/// The end of a list's removals: the element held in the out `NiPointer`, the item freed, one fewer.
fn finish_pop(e: &mut Engine, list: Ptr<NiTPointerList>, item: u32, out: Ptr) -> Ptr {
    let element = e.get(Ptr::<NiTListItem>::new(item), NiTListItem::m_element);
    e.mem.set_u32(out.addr(), element);
    if element != 0 {
        hold(e, element);
    }
    e.call(LIST_FREE_ITEM, &args![list, item]);
    let count = e.get(list, NiTPointerList::m_uiCount).wrapping_sub(1);
    e.set(list, NiTPointerList::m_uiCount, count);
    out
}

// Typed from 00b9e7c0
/// An `NiTPointerList` of `NiPointer`s' head removal into an `NiPointer` the caller gives
/// (`thiscall(list, out)`): the head unlinked (the new head's previous link cleared, or the tail
/// with none), its element held in `out`, the item freed, one fewer. An empty list is not checked,
/// as in the exe. Answers `out`.
pub fn pop_head(e: &mut Engine, list: Ptr<NiTPointerList>, out: Ptr) -> Ptr {
    let item = e.get(list, NiTPointerList::m_pkHead);
    let next = e.get(Ptr::<NiTListItem>::new(item), NiTListItem::m_pkNext);
    e.set(list, NiTPointerList::m_pkHead, next);
    if next == 0 {
        e.set(list, NiTPointerList::m_pkTail, 0);
    } else {
        e.set(Ptr::<NiTListItem>::new(next), NiTListItem::m_pkPrev, 0);
    }
    finish_pop(e, list, item, out)
}

// Typed from 00b9e850
/// The tail removal counterpart of [`pop_head`] (`thiscall(list, out)`): the tail unlinked (the new
/// tail's next link cleared, or the head with none). Answers `out`.
pub fn pop_tail(e: &mut Engine, list: Ptr<NiTPointerList>, out: Ptr) -> Ptr {
    let item = e.get(list, NiTPointerList::m_pkTail);
    let previous = e.get(Ptr::<NiTListItem>::new(item), NiTListItem::m_pkPrev);
    e.set(list, NiTPointerList::m_pkTail, previous);
    if previous == 0 {
        e.set(list, NiTPointerList::m_pkHead, 0);
    } else {
        e.set(Ptr::<NiTListItem>::new(previous), NiTListItem::m_pkNext, 0);
    }
    finish_pop(e, list, item, out)
}

// Typed from 00b9cfd0
/// The `NiPointer` assignment at +0x1a0 (`thiscall(this, object)`, an accumulator's): nothing for
/// the same object, else the old one released and the new one held.
pub fn set_held_1a0(e: &mut Engine, this: Ptr, object: u32) {
    let slot = this.addr().wrapping_add(0x1a0);
    let old = e.mem.u32(slot);
    if old == object {
        return;
    }
    if old != 0 {
        release(e, old);
    }
    e.mem.set_u32(slot, object);
    if object != 0 {
        hold(e, object);
    }
}

/// The float at `at`, loaded at precision `R`.
#[inline(always)]
fn ld<R: Real>(e: &Engine, at: u32) -> R {
    R::load(e.mem.u32(at))
}

/// The squared length `(y * y + x * x) + z * z` of three stored floats, stored.
#[inline(always)]
fn squared<R: Real>(d: [u32; 3]) -> u32 {
    let [x, y, z] = d.map(R::load);
    (y * y + x * x + z * z).store()
}

/// The root of a stored float, stored.
#[inline(always)]
fn root(squared: u32) -> u32 {
    f32::from_bits(squared).sqrt().to_bits()
}

/// `FLD a; FLD b; FCOM; TEST AH,5; JP`: of two stored floats, `a` when `b < a`, else `b` (NaN too).
#[inline(always)]
fn larger(a: u32, b: u32) -> u32 {
    if f32::from_bits(b) < f32::from_bits(a) {
        a
    } else {
        b
    }
}

// Typed from 00b9dbe0
/// `ShadowSceneLight::GetLuminanceToBound` (`thiscall(light, bound)`): the distance from the
/// `NiLight`'s position moved by the scene node's lighting offset to the bound's centre, less the
/// bound's radius, over the `NiLight`'s radius; stored in `fLuminance` and answered.
pub fn luminance_to_bound(e: &mut Engine, this: Ptr<ShadowSceneLight>, bound: Ptr) -> f32 {
    if float_precision() {
        luminance_to_bound_in::<f32>(e, this, bound.addr())
    } else {
        luminance_to_bound_in::<f64>(e, this, bound.addr())
    }
}

fn luminance_to_bound_in<R: Real>(e: &mut Engine, this: Ptr<ShadowSceneLight>, b: u32) -> f32 {
    let node = e.global::<u32>(SCENE_NODE);
    let offset = three(e, node.wrapping_add(LIGHTING_OFFSET));
    let light = e.get(this, ShadowSceneLight::spLight);
    let at = light.wrapping_add(WORLD_TRANSLATE);
    let position = [
        (R::load(offset[0]) + ld::<R>(e, at)).store(),
        (ld::<R>(e, at + 4) + R::load(offset[1])).store(),
        (ld::<R>(e, at + 8) + R::load(offset[2])).store(),
    ];
    let d = [0u32, 1, 2].map(|i| (R::load(position[i as usize]) - ld::<R>(e, b + 4 * i)).store());
    let distance = R::load(root(squared::<R>(d)));
    let value = ((distance - ld::<R>(e, b + 0xc)) / ld::<R>(e, light.wrapping_add(RADIUS))).store();
    let value = f32::from_bits(value);
    e.set(this, ShadowSceneLight::fLuminance, value);
    value
}

/// An `NiLight`'s dimmer times each colour component, each stored: `[r, g, b]`.
fn scaled_colour<R: Real>(e: &Engine, light: u32) -> [u32; 3] {
    // `FLD; FSTP; FLD`: the dimmer stored to a float first.
    let dimmer = R::load(R::load(e.mem.u32(light.wrapping_add(DIMMER))).store());
    [0u32, 4, 8].map(|i| (dimmer * ld::<R>(e, light.wrapping_add(COLOUR + i))).store())
}

/// The point light's falloff `1 - t * t`, `t` the clamped ratio of a distance to the radius, both
/// stored floats; the low clamp compares with the double 0.0 (`01012060`), NaN kept.
fn falloff<R: Real>(ratio: u32) -> u32 {
    let ratio = if R::load(ratio) < R::ZERO {
        R::ZERO.store()
    } else {
        ratio
    };
    let t = if R::ONE < R::load(ratio) {
        R::ONE
    } else {
        R::load(ratio)
    };
    let t = R::load(t.store());
    (R::ONE - t * t).store()
}

// Typed from 00b9bc50
/// The light's luminance at a point (`thiscall(light, x, y, z, NiLight)`): 0.0 for that `NiLight`
/// and for a light with `bCastShadow` 1. Otherwise `fLuminance` 1.0, for a point light the falloff
/// of the distance to the point over the radius; then times the largest of the `NiLight`'s dimmed
/// colour components, stored in `fLuminance` and answered.
pub fn luminance_at(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    x: u32,
    y: u32,
    z: u32,
    excluded: u32,
) -> f32 {
    let light = e.get(this, ShadowSceneLight::spLight);
    if light == excluded || e.get(this, ShadowSceneLight::bCastShadow) == 1 {
        return 0.0;
    }
    if float_precision() {
        luminance_at_in::<f32>(e, this, light, [x, y, z])
    } else {
        luminance_at_in::<f64>(e, this, light, [x, y, z])
    }
}

fn luminance_at_in<R: Real>(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    light: u32,
    point: [u32; 3],
) -> f32 {
    let [r, g, b] = scaled_colour::<R>(e, light);
    let mut luminance = R::ONE.store();
    if e.get(this, ShadowSceneLight::bPointLight) != 0 {
        let at = light.wrapping_add(WORLD_TRANSLATE);
        let d = [0u32, 1, 2].map(|i| (R::load(point[i as usize]) - ld::<R>(e, at + 4 * i)).store());
        let distance = R::load(root(squared::<R>(d)));
        let ratio = (distance / ld::<R>(e, light.wrapping_add(RADIUS))).store();
        luminance = falloff::<R>(ratio);
    }
    let peak = larger(r, larger(g, b));
    let result = f32::from_bits((R::load(peak) * R::load(luminance)).store());
    e.set(this, ShadowSceneLight::fLuminance, result);
    result
}

/// `NiPoint3::Unitize` (`004a0c10`) of a stored point in place: a length above the minimum
/// (`01017cf8`, a double) divides each axis by `1 / length` stored to a float, any other zeroes it.
fn unit<R: Real>(e: &Engine, p: [u32; 3]) -> [u32; 3] {
    let [x, y, z] = p.map(R::load);
    // `NiPoint3::Length` (`00457990`): `(x * x + y * y) + z * z` stored, its root stored.
    let length = f32::from_bits(root((x * x + y * y + z * z).store()));
    #[allow(clippy::neg_cmp_op_on_partial_ord)] // a NaN length zeroes the point, as the exe's test
    if !(f64::from(length) > e.global::<f64>(MINIMUM_LENGTH)) {
        return [0; 3];
    }
    let inverse = 1.0 / length;
    p.map(|v| (f32::from_bits(v) * inverse).to_bits())
}

/// `Unitize`'s minimum length (`01017cf8`, a double) and the double 1.0 (`01012070`) the facing
/// term is cut at.
const MINIMUM_LENGTH: u32 = 0x0101_7cf8;

// Typed from 00b9bdf0
/// The light's luminance at a point facing a direction (`thiscall(light, a, b, NiLight)`, two
/// points of three words): 0.0 for that `NiLight`, a light with `bCastShadow` 1 or a culled
/// `NiLight`. Otherwise `fLuminance` 1.0, for a point light the falloff of the distance from the
/// light to `a` over the radius (clamped to 0.0 and 1.0); then times the facing term, the dot
/// product of `-(b - a)` and `a - light` made unit vectors, cut to 1.0 above and 0.0 below (NaN
/// kept), and times the largest dimmed colour component; stored in `fLuminance` and answered.
#[allow(clippy::too_many_arguments)] // the exe's seven words
pub fn facing_luminance(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    ax: u32,
    ay: u32,
    az: u32,
    bx: u32,
    by: u32,
    bz: u32,
    excluded: u32,
) -> f32 {
    let light = e.get(this, ShadowSceneLight::spLight);
    if light == excluded
        || e.get(this, ShadowSceneLight::bCastShadow) == 1
        || e.mem.u8(light.wrapping_add(FLAGS)) & 1 != 0
    {
        return 0.0;
    }
    let (a, b) = ([ax, ay, az], [bx, by, bz]);
    if float_precision() {
        facing_luminance_in::<f32>(e, this, light, a, b)
    } else {
        facing_luminance_in::<f64>(e, this, light, a, b)
    }
}

fn facing_luminance_in<R: Real>(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    light: u32,
    a: [u32; 3],
    b: [u32; 3],
) -> f32 {
    let [r, g, bl] = scaled_colour::<R>(e, light);
    let at = light.wrapping_add(WORLD_TRANSLATE);
    let to_point = [0u32, 1, 2].map(|i| (R::load(a[i as usize]) - ld::<R>(e, at + 4 * i)).store());
    let mut luminance = R::ONE.store();
    if e.get(this, ShadowSceneLight::bPointLight) != 0 {
        let [x, y, z] = to_point.map(R::load);
        let length = R::load(root((x * x + y * y + z * z).store()));
        let ratio = (length / ld::<R>(e, light.wrapping_add(RADIUS))).store();
        // `00404010` (with 0.0) and `0040ebd0` (with 1.0), each answer stored: the falloff's clamps.
        luminance = falloff::<R>(ratio);
    }
    let along = [0, 1, 2].map(|i| (R::load(b[i]) - R::load(a[i])).store());
    let along = unit::<R>(e, along);
    let to_point = unit::<R>(e, to_point);
    let [nx, ny, nz] = along.map(|v| (-f32::from_bits(v)).to_bits()).map(R::load);
    let [tx, ty, tz] = to_point.map(R::load);
    let dot = R::load((ny * ty + nx * tx + nz * tz).store());
    // `FCOM 1.0; TEST AH,0x41; JNZ`: cut to 1.0 only when greater (NaN kept); then below 0.0 to 0.0.
    let facing = if dot > R::ONE { R::ONE } else { dot };
    let facing = R::load(facing.store());
    let facing = if facing < R::ZERO { R::ZERO } else { facing };
    let peak = larger(r, larger(g, bl));
    let result = (R::load(luminance) * R::load(facing.store()) * R::load(peak)).store();
    let result = f32::from_bits(result);
    e.set(this, ShadowSceneLight::fLuminance, result);
    result
}

/// The TLS word the shadow flag holds at 0x20 for its duration (+0x2b4); the renderer object
/// (`011f91a8`) and its shadow map release (`00b6d4c0`, `thiscall`: the map); the render level
/// (`011f91c0`, signed, at least 3) and bit 0x10 of the render flags (`011f91d8`) that keep a map.
const TLS_WORD: u32 = 0x2b4;
const RENDERER: u32 = 0x011f_91a8;
const RENDERER_RELEASE_MAP: u32 = 0x00b6_d4c0;
const RENDER_LEVEL: u32 = 0x011f_91c0;
const RENDER_FLAGS: u32 = 0x011f_91d8;

// Typed from 00b9dcb0
/// The light's `bCastShadow` set (`thiscall(light, flag)`), with the TLS word +0x2b4 at 0x20 for its
/// duration: a light that stops casting shadows, or any light unless the render level is at least
/// 3 and bit 0x10 of the render flags is set, gives its dynamic shadow map back to the renderer and
/// releases it.
pub fn set_cast_shadow(e: &mut Engine, this: Ptr<ShadowSceneLight>, cast: u8) {
    let tls = e.tls();
    let saved = e.mem.u32(tls.wrapping_add(TLS_WORD));
    e.mem.set_u32(tls.wrapping_add(TLS_WORD), 0x20);
    let stops = e.get(this, ShadowSceneLight::bCastShadow) != 0 && cast == 0;
    let keeps =
        !stops && e.global::<i32>(RENDER_LEVEL) >= 3 && e.global::<u8>(RENDER_FLAGS) & 0x10 != 0;
    let map = if keeps {
        0
    } else {
        e.get(this, ShadowSceneLight::spDynamicShadowMap)
    };
    if map != 0 {
        let renderer = e.global::<u32>(RENDERER);
        e.call(RENDERER_RELEASE_MAP, &args![renderer, map]);
        // Read again: the release is the renderer's.
        let map = e.get(this, ShadowSceneLight::spDynamicShadowMap);
        if map != 0 {
            release(e, map);
            e.set(this, ShadowSceneLight::spDynamicShadowMap, 0);
        }
    }
    e.set(this, ShadowSceneLight::bCastShadow, cast);
    e.mem.set_u32(tls.wrapping_add(TLS_WORD), saved);
}

// Typed from 00b9e770
/// The geometry list's fence cleared (`thiscall(light)`): from the fence item to the list's end,
/// each geometry's lighting property (type 1 to 12) gets 0 at +0x38; then the fence is 0.
pub fn clear_fence(e: &mut Engine, this: Ptr<ShadowSceneLight>) {
    let mut item = e.get(this, ShadowSceneLight::kGeomListFence);
    while item != 0 {
        let at = Ptr::<NiTListItem>::new(item);
        let geometry = e.get(at, NiTListItem::m_element);
        item = e.get(at, NiTListItem::m_pkNext);
        if geometry != 0 {
            let property = lighting_property(e, geometry);
            if property != 0 {
                e.mem.set_u32(property.wrapping_add(0x38), 0);
            }
        }
    }
    e.set(this, ShadowSceneLight::kGeomListFence, 0);
}

/// `RemoveTriGeom`'s callees: the lighting property's light removal (`00b717a0`, `thiscall`: the
/// light) and `lGeomList`'s removals by position (`00b9efa0`) and by value (`00b9fcc0`), each
/// `thiscall(list, out NiPointer, item or value)`.
const PROPERTY_REMOVE_LIGHT: u32 = 0x00b7_17a0;
const LIST_REMOVE_POS: u32 = 0x00b9_efa0;
const LIST_REMOVE: u32 = 0x00b9_fcc0;

// Typed from 00ba01a0
/// `ShadowSceneLight::RemoveTriGeom` (`thiscall(light, geometry, item)`): the geometry's lighting
/// property (type 1 to 12) drops the light. With an item, `lGeomList` loses it by position, the
/// removed `NiPointer` landing in the geometry's own argument word (released after); without, the
/// geometry, held in the item's argument word, is removed by value into the word the exe's
/// `PUSH ECX` reserved (which starts as the light), and both are released.
pub fn remove_tri_geom(e: &mut Engine, this: Ptr<ShadowSceneLight>, geometry: u32, item: u32) {
    let property = lighting_property(e, geometry);
    if property != 0 {
        e.call(PROPERTY_REMOVE_LIGHT, &args![property, this]);
    }
    let list = this.addr().wrapping_add(ShadowSceneLight::lGeomList.off);
    // The exe's words: the geometry's argument (+0), the item's (+4), the reserved word (+8).
    e.with_stack(0xc, |e, frame| {
        let (geometry_word, item_word, out) = (frame.addr(), frame.addr() + 4, frame.addr() + 8);
        e.mem.set_u32(geometry_word, geometry);
        e.mem.set_u32(item_word, item);
        e.mem.set_u32(out, this.addr());
        if item != 0 {
            e.call(LIST_REMOVE_POS, &args![list, geometry_word, item_word]);
            let removed = e.mem.u32(geometry_word);
            if removed != 0 {
                release(e, removed);
            }
            return;
        }
        e.mem.set_u32(item_word, geometry);
        hold(e, geometry);
        e.call(LIST_REMOVE, &args![list, out, item_word]);
        let removed = e.mem.u32(out);
        if removed != 0 {
            release(e, removed);
        }
        let held = e.mem.u32(item_word);
        if held != 0 {
            release(e, held);
        }
    });
}

/// The cull pass's data: the callback for each geometry (`011f91ec`, `cdecl`, when set), bit 0 of
/// the render flags (`011ad82c`), the pass words (`011f91e0` cleared, `011f91e4` the pass), the
/// renderer singleton (`011f4748`, its +8 object's busy test `00aa0ff0`), the pass setters
/// (`00b7a870` for types 1 to 5, `00bbd880` for 0x11; `cdecl(pass, 0)`) and the shader objects'
/// slot +0x24.
const GEOMETRY_CALLBACK: u32 = 0x011f_91ec;
const CULL_RENDER_FLAGS: u32 = 0x011a_d82c;
const PASS_STATE: u32 = 0x011f_91e0;
const PASS: u32 = 0x011f_91e4;
const RENDERER_SINGLETON: u32 = 0x011f_4748;
const RENDERER_BUSY: u32 = 0x00aa_0ff0;
const PASS_SETTER: u32 = 0x00b7_a870;
const PASS_SETTER_11: u32 = 0x00bb_d880;
const SLOT_SHADER_24: u32 = 0x24;

// Typed from 00b9d020
/// The light's geometries culled (`thiscall(light, culler)`, answers 1): unless the `NiLight` is
/// culled, each geometry of `lGeomList` with a +0xc0 property goes to the callback (when set);
/// with bit 0 of the render flags, the renderer's busy test is asked and the property's type
/// (+0x68) picks a pass: 1 to 5 (0x5c with the geometry's +0xbc word, else 0x58) or 0x11 (0x1ca or
/// 0x1c8), set when not busy (the pass words, the setter, the property's +0x34 and +0x30 objects'
/// slot +0x24), then the geometry is culled; any other type skips it.
pub fn cull_geometries(e: &mut Engine, this: Ptr<ShadowSceneLight>, culler: u32) -> bool {
    let light = e.get(this, ShadowSceneLight::spLight);
    if e.mem.u8(light.wrapping_add(FLAGS)) & 1 != 0 {
        return true;
    }
    let mut item = e
        .mem
        .u32(this.addr().wrapping_add(ShadowSceneLight::lGeomList.off));
    while item != 0 {
        let at = Ptr::<NiTListItem>::new(item);
        let geometry = e.get(at, NiTListItem::m_element);
        item = e.get(at, NiTListItem::m_pkNext);
        if geometry == 0 {
            continue;
        }
        let property = e.mem.u32(geometry.wrapping_add(GEOMETRY_C0));
        if property == 0 {
            continue;
        }
        let callback = e.global::<u32>(GEOMETRY_CALLBACK);
        if callback != 0 {
            e.call(callback, &args![geometry]);
        }
        if e.global::<u8>(CULL_RENDER_FLAGS) & 1 == 0 {
            continue;
        }
        let renderer = e.global::<u32>(RENDERER_SINGLETON);
        let object = e.mem.u32(renderer.wrapping_add(8));
        let busy = e.call(RENDERER_BUSY, &args![object]).u8() != 0;
        let kind = e.mem.i32(property.wrapping_add(0x68));
        let with_word = e.mem.u32(geometry.wrapping_add(0xbc)) != 0;
        let (setter, pass) = match kind {
            1..=5 => (PASS_SETTER, if with_word { 0x5c } else { 0x58 }),
            0x11 => (PASS_SETTER_11, if with_word { 0x1ca } else { 0x1c8 }),
            _ => continue,
        };
        if !busy {
            e.set_global(PASS_STATE, 0u32);
            e.set_global(PASS, pass);
            e.call(setter, &args![pass, 0u32]);
            for shader in [0x34u32, 0x30] {
                let object = e.mem.u32(property.wrapping_add(shader));
                e.vcall(object, SLOT_SHADER_24, &args![]);
            }
        }
        typed_call!(
            crate::culling::FAMILY,
            0x00a5_9e00,
            crate::culling::visible::cull(e, Ptr::new(geometry), Ptr::new(culler))
        );
    }
    true
}
