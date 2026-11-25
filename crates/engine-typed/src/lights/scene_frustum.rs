//! A shadow scene light's frustum functions, the rest of its own: the view volume test of two
//! cameras (`00b9c020`), the cube face test (`00b9de90`), `TestFrustumCull` (`00b9e970`),
//! `MTSetupFrustumDepthMap` (`00b9dfc0`) and `MTRenderFrustumDepthMap` (`00b9f780`).
//!
//! Done in place: `NiPlane`'s constructor (`00a69940`; the volume test's plane arrays are kept as
//! values, read once out of the culler), the local culler's destructor (`00a693e0`, its vtable),
//! `NiFrustum`'s constructor (`00a71b70`, its orthographic byte), the larger and smaller of two
//! floats (`00404010`, `0040ebd0`), the C runtime's square root of a stored float (one correctly
//! rounded root) and the `NiPointer` counts. Typed calls: the culler's frustum set-up, the scene
//! graph's update and property, the shader manager's accumulator, the point maths (normalize,
//! scale, length, add), the frustum planes, the light's accumulator hold (`00b9cfd0`), the scene
//! draw, the renderer's camera and `D3DXMatrixMultiply`. What stays by address: the cube map
//! camera, the culler's constructor, `UnitCross`, the extra data, the cameras' set-ups, the
//! renderer's shadow maps, the rendering system's tasks and stages and the depth pass.
//!
//! `NV_LIGHT_CHECK`'s lines on why `TestFrustumCull` culled a light are the translation's: with
//! this family on they are not printed (`NV_TYPED=lights:off` brings them back).
//!
//! **Floats.** These run on the main thread (the shadow preparation, the scene's visibility pass)
//! and on others (the render passes): each is written once over [`Real`], every step and store as
//! the exe makes them, and its Diff tests run both control words. A product or sum with one of the
//! exe's `double` constants is taken through the double ([`with_double`]); a negation of a stored
//! float flips its sign bit, as `FCHS` does.

use super::scene_light::{release, set_held_1a0};
use super::ShadowSceneLight;
use super::FAMILY;
use crate::native::{float_precision, to_float, Real};
use engine_core::ptr::Ptr;
use engine_core::sync_direct::interlocked_increment;
use engine_core::types::NiTListItem;
use engine_core::x87::x87_quiet_bits;
use engine_core::{args, typed_call, Engine};

/// The sign bit of a float's word.
const SIGN: u32 = 0x8000_0000;
const ONE: u32 = 0x3f80_0000;

/// `NiAVObject` and `NiLight` members: the world translation (+0x8c), the flags (+0x30, bit 0
/// culled), the world bound (+0x20), the radius (+0xe0); a camera's rotation (+0x68, three rows of
/// three) and frustum (+0xdc: left, right, top, bottom, near, far, orthographic).
const WORLD_TRANSLATE: u32 = 0x8c;
const FLAGS: u32 = 0x30;
const WORLD_BOUND: u32 = 0x20;
const RADIUS: u32 = 0xe0;
const CAMERA_ROTATE: u32 = 0x68;
const CAMERA_FRUSTUM: u32 = 0xdc;
/// The null bound (`011f4288`) and the zero point (`011f426c`).
const NULL_BOUND: u32 = 0x011f_4288;
const ZERO_POINT: u32 = 0x011f_426c;

/// The three words from `at`.
fn three(e: &Engine, at: u32) -> [u32; 3] {
    [0u32, 4, 8].map(|i| e.mem.u32(at.wrapping_add(i)))
}

/// Three words to `at`.
fn set_three(e: &mut Engine, at: u32, words: [u32; 3]) {
    for (i, word) in (0u32..).zip(words) {
        e.mem.set_u32(at.wrapping_add(4 * i), word);
    }
}

/// `words` words from `from` to `to` (`MOVSD.REP`).
fn copy_words(e: &mut Engine, to: u32, from: u32, words: u32) {
    for i in 0..words {
        let value = e.mem.u32(from.wrapping_add(4 * i));
        e.mem.set_u32(to.wrapping_add(4 * i), value);
    }
}

/// A stored float loaded at precision `R`.
#[inline(always)]
fn ld<R: Real>(bits: u32) -> R {
    R::load(bits)
}

/// A step stored to a float and loaded again (`FSTP dword; FLD dword`).
#[inline(always)]
fn st<R: Real>(value: R) -> R {
    R::load(value.store())
}

/// One step with a `double` the exe keeps in memory (`FMUL qword`, `FADD qword`), stored: on
/// the AI threads the double step itself; on main the double step rounded once more to the float
/// (the exe's 24-bit step rounds the exact value once; the two differ only when the double lands
/// exactly half way between two floats).
#[inline(always)]
fn with_double(value: f32, constant: f64, op: fn(f64, f64) -> f64) -> u32 {
    to_float(op(f64::from(value), constant)).to_bits()
}

/// The bound of an object, or the null bound.
fn bound_or_null(e: &Engine, object: u32) -> u32 {
    let bound = e.mem.u32(object.wrapping_add(WORLD_BOUND));
    if bound == 0 {
        NULL_BOUND
    } else {
        bound
    }
}

// ---------------------------------------------------------------------------------------------
// 00b9c020: the view volumes of two cameras.

/// `NiCullingProcess`'s constructor (`00a69400`, `RET 4`), its vtable (`0109c28c`, what its
/// destructor `00a693e0` stores back) and the frame's layout: the culler at +0x144 of 0x2c0
/// bytes, its camera at +0xc and its planes at +0x2c (six of normal and constant).
const CULLER_CONSTRUCT: u32 = 0x00a6_9400;
const CULLER_VTABLE: u32 = 0x0109_c28c;
const VOLUME_FRAME: u32 = 0x2c0;
const VOLUME_CULLER: u32 = 0x144;
const CULLER_CAMERA: u32 = 0xc;
const CULLER_PLANES: u32 = 0x2c;

/// The frustum's edges as pairs of its five points (the camera position, then the four far
/// corners), as the exe's byte table lists them.
const FRUSTUM_EDGES: [(usize, usize); 8] = [
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 4),
    (1, 2),
    (1, 3),
    (2, 4),
    (3, 4),
];

/// Six planes: normal and constant words each.
type Planes = [[u32; 4]; 6];

/// The five points of a camera's view volume: the world translation (its words) and the four far
/// corners `t + f*r0 +/- f*u*r1 +/- f*s*r2` (`r0..r2` the rotation's columns, `f` the far
/// distance at +0xf0, `u` the frustum's +0xe4 and `s` its +0xe0), every product and sum stored.
fn camera_points<R: Real>(e: &Engine, camera: u32) -> [[u32; 3]; 5] {
    let f = |offset: u32| ld::<R>(e.mem.u32(camera.wrapping_add(offset)));
    let p = three(e, camera.wrapping_add(WORLD_TRANSLATE));
    let (up, side, far) = (f(0xe4), f(0xe0), f(0xf0));
    let up_column = [f(0x6c), f(0x78), f(0x84)].map(|r| st(r * up));
    let side_column = [f(0x70), f(0x7c), f(0x88)].map(|r| st(r * side));
    let ups = up_column.map(|v| (v * far).store());
    let sides = side_column.map(|v| (v * far).store());
    let base = [f(0x68), f(0x74), f(0x80)].map(|r| st(r * far));
    let base = [0usize, 1, 2].map(|i| (base[i] + ld::<R>(p[i])).store());
    let step = |value: u32, delta: u32, plus: bool| {
        if plus {
            (ld::<R>(value) + ld::<R>(delta)).store()
        } else {
            (ld::<R>(value) - ld::<R>(delta)).store()
        }
    };
    let corner = |up_plus: bool, side_plus: bool| {
        [0usize, 1, 2].map(|i| step(step(base[i], ups[i], up_plus), sides[i], side_plus))
    };
    [
        p,
        corner(true, true),
        corner(true, false),
        corner(false, true),
        corner(false, false),
    ]
}

/// A point's signed distance from a plane, `((n.x p.x + n.y p.y) + n.z p.z) - d`, stored.
#[inline(always)]
fn distance<R: Real>(plane: [u32; 4], point: [u32; 3]) -> R {
    let [nx, ny, nz, d] = plane.map(ld::<R>);
    let [x, y, z] = point.map(ld::<R>);
    st(nx * x + ny * y + nz * z - d)
}

/// An edge from `start` to `end` crossing plane `index`: the crossing parameter `t = ((d n - P) .
/// n) / ((Q - P) . n)` and point as the exe computes them, true when `t` is in [0, 1] and the point
/// is behind no other plane.
fn edge_crosses_inside<R: Real>(
    planes: &Planes,
    index: usize,
    start: [u32; 3],
    end: [u32; 3],
) -> bool {
    let [nx, ny, nz, d] = planes[index].map(ld::<R>);
    let s = start.map(ld::<R>);
    let q = end.map(ld::<R>);
    let a = st(st(nx * d) - s[0]);
    let b = st(st(d * ny) - s[1]);
    let c = st(st(d * nz) - s[2]);
    let delta = [0usize, 1, 2].map(|i| st(q[i] - s[i]));
    let numerator = st(b * ny + a * nx + c * nz);
    let denominator = st(nx * delta[0] + ny * delta[1] + nz * delta[2]);
    let t = st(numerator / denominator);
    // `FLDZ; FCOM; TEST AH,0x41; JP` and `FLD1; FCOMP; TEST AH,1; JNZ`: on for 0 <= t <= 1 only.
    if !(R::ZERO <= t && t <= R::ONE) {
        return false;
    }
    let point = [0usize, 1, 2].map(|i| (st(delta[i] * t) + s[i]).store());
    // `FCOM dword; TEST AH,0x41; JZ`: only a distance below 0 rejects the point (NaN passes).
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let inside = |other: usize| !(distance::<R>(planes[other], point) < R::ZERO);
    (0..6).filter(|&other| other != index).all(inside)
}

/// The side of a point for the edge test: 2 behind the plane, 1 in front, 0 on it or unordered.
#[inline(always)]
fn side_of<R: Real>(distance: R) -> u8 {
    if distance < R::ZERO {
        2
    } else if distance > R::ZERO {
        1
    } else {
        0
    }
}

// Typed from 00b9c020
/// Whether two cameras' view volumes meet (`thiscall` on a light it does not read, `RET 8`: two
/// cameras). Each camera's five points, and its six planes from a local `NiCullingProcess`'s
/// `SetFrustum` of the camera's frustum. True when a point of one camera is strictly inside all
/// planes of the other; or, for an edge of one camera's volume, when both ends lie on a plane of
/// the other; or when the edge crosses that plane inside the other planes.
pub fn volumes_meet(e: &mut Engine, _this: Ptr<ShadowSceneLight>, first: Ptr, second: Ptr) -> bool {
    if float_precision() {
        volumes_meet_in::<f32>(e, first.addr(), second.addr())
    } else {
        volumes_meet_in::<f64>(e, first.addr(), second.addr())
    }
}

fn volumes_meet_in<R: Real>(e: &mut Engine, first: u32, second: u32) -> bool {
    let points = [camera_points::<R>(e, first), camera_points::<R>(e, second)];
    let planes = e.with_stack(VOLUME_FRAME, |e, frame| {
        let culler = frame.addr().wrapping_add(VOLUME_CULLER);
        e.call(CULLER_CONSTRUCT, &args![culler, 0u32]);
        let planes = [first, second].map(|camera| {
            e.mem.set_u32(culler.wrapping_add(CULLER_CAMERA), camera);
            typed_call!(
                crate::culling::FAMILY,
                0x00a6_94a0,
                crate::culling::process::set_frustum(
                    e,
                    Ptr::new(culler),
                    Ptr::new(camera.wrapping_add(CAMERA_FRUSTUM))
                )
            );
            let at = culler.wrapping_add(CULLER_PLANES);
            let plane = |i: u32| [0u32, 4, 8, 0xc].map(|o| e.mem.u32(at + 0x10 * i + o));
            [0u32, 1, 2, 3, 4, 5].map(plane)
        });
        e.mem.set_u32(culler, CULLER_VTABLE);
        planes
    });
    // Camera 0's points against camera 1's planes, then the other way round.
    for camera in 0..2 {
        let against = &planes[1 - camera];
        if points[camera].iter().any(|&point| {
            against
                .iter()
                .all(|&plane| distance::<R>(plane, point) > R::ZERO)
        }) {
            return true;
        }
    }
    for camera in 0..2 {
        let against = &planes[1 - camera];
        let points = &points[camera];
        for (from, to) in FRUSTUM_EDGES {
            let (start, end) = (points[from], points[to]);
            for index in 0..6 {
                let side_start = side_of(distance::<R>(against[index], start));
                let side_end = side_of(distance::<R>(against[index], end));
                if side_start == side_end {
                    if side_start == 0 {
                        return true;
                    }
                } else if edge_crosses_inside::<R>(against, index, start, end) {
                    return true;
                }
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------------------------
// 00b9de90: a cube face of a light against a culler's camera.

/// `BSCubeMapCamera`'s constructor (`00bd8290`, `RET 4`), `SetupTransformation` (`00bd7200`, `RET
/// 4`: the face) and destructor (`00bd74a0`); the thread's word +0x2b4 (0x20 meanwhile).
const CUBE_CAMERA_CONSTRUCT: u32 = 0x00bd_8290;
const CUBE_CAMERA_SETUP: u32 = 0x00bd_7200;
const CUBE_CAMERA_DESTRUCT: u32 = 0x00bd_74a0;
const TLS_WORD_2B4: u32 = 0x2b4;

/// `NiAVObject::Update` (`00a59c60`) with the exe's zeroed update data (a `float` 0.0 and five
/// zero bytes) at `data`.
fn update_with_zeros(e: &mut Engine, object: u32, data: u32) {
    e.mem.set_u32(data, 0);
    for byte in 4..9u32 {
        e.mem.set_u8(data + byte, 0);
    }
    typed_call!(
        crate::scene::FAMILY,
        0x00a5_9c60,
        crate::scene::walk::update(e, Ptr::new(object), data)
    );
}

// Typed from 00b9de90
/// `ShadowSceneLight::CubeFaceFrustumFrustumIntersection` (`thiscall`, `RET 0x18`: a position,
/// an unread word, an object whose camera is at +0xc, a 16-bit face). With the thread's word
/// +0x2b4 at 0x20 meanwhile: a local `BSCubeMapCamera` at the position, set up for the face and
/// updated, against the object's camera ([`volumes_meet`]); then destroyed.
#[allow(clippy::too_many_arguments)]
pub fn cube_face_meets(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    x: u32,
    y: u32,
    z: u32,
    _unused: u32,
    source: Ptr,
    face: u16,
) -> bool {
    let tls = e.tls();
    let source_camera = e.mem.u32(source.addr().wrapping_add(CULLER_CAMERA));
    let saved = e.mem.u32(tls + TLS_WORD_2B4);
    e.mem.set_u32(tls + TLS_WORD_2B4, 0x20);
    // The frame: the update data (+0) and the local camera (+0x10, 0x140 bytes).
    let answer = e.with_stack(0x150, |e, frame| {
        let data = frame.addr();
        let camera = data + 0x10;
        e.call(CUBE_CAMERA_CONSTRUCT, &args![camera, 0u32]);
        set_three(e, camera + 0x58, [x, y, z]);
        e.call(CUBE_CAMERA_SETUP, &args![camera, u32::from(face)]);
        update_with_zeros(e, camera, data);
        let answer = typed_call!(
            FAMILY,
            0x00b9_c020,
            volumes_meet(e, this, Ptr::new(source_camera), Ptr::new(camera))
        );
        e.call(CUBE_CAMERA_DESTRUCT, &args![camera]);
        answer
    });
    e.mem.set_u32(tls + TLS_WORD_2B4, saved);
    answer
}

// ---------------------------------------------------------------------------------------------
// 00b9e970: TestFrustumCull.

/// The render level (`011f91c0`, signed), the render flags (`011f91d8`, bit 0x10 the cube maps),
/// the dimmer's fade distances (`011f9444`, `011f9448`), the shadow dimmer's (`011f944c`,
/// `011f9450`), the elapsed step (`011f9410`) and the fade time (`011ad834`); the renderer
/// (`011f91a8`) and its shadow map release (`00b6d4c0`, `RET 4`).
const RENDER_LEVEL: u32 = 0x011f_91c0;
const RENDER_FLAGS: u32 = 0x011f_91d8;
const DIMMER_NEAR: u32 = 0x011f_9444;
const DIMMER_FAR: u32 = 0x011f_9448;
const SHADOW_NEAR: u32 = 0x011f_944c;
const SHADOW_FAR: u32 = 0x011f_9450;
const ELAPSED_STEP: u32 = 0x011f_9410;
const FADE_TIME: u32 = 0x011a_d834;
const RENDERER: u32 = 0x011f_91a8;
const RENDERER_RELEASE_MAP: u32 = 0x00b6_d4c0;
/// The culler's planes (+0x2c, six of four words) and their mask (+0x8c).
const PLANE_MASK: u32 = 0x8c;
/// A geometry's property (+0xa8), its type (+0x1c, 1 to 12) and its "lights changed" byte (+0x74).
const GEOMETRY_PROPERTY: u32 = 0xa8;
const CULLED: u16 = 0xff;

/// Whether the high-level cube shadow maps are on: `[011f91c0] >= 3` and bit 0x10 of
/// `[011f91d8]`.
fn cube_maps_on(e: &Engine) -> bool {
    e.global::<i32>(RENDER_LEVEL) >= 3 && e.global::<u8>(RENDER_FLAGS) & 0x10 != 0
}

/// The light's shadow map handed back to the renderer and released.
fn release_shadow_map(e: &mut Engine, this: Ptr<ShadowSceneLight>) -> bool {
    let map = e.get(this, ShadowSceneLight::spDynamicShadowMap);
    if map == 0 {
        return false;
    }
    let renderer = e.global::<u32>(RENDERER);
    e.call(RENDERER_RELEASE_MAP, &args![renderer, map]);
    let map = e.get(this, ShadowSceneLight::spDynamicShadowMap);
    if map != 0 {
        release(e, map);
        e.set(this, ShadowSceneLight::spDynamicShadowMap, 0);
    }
    true
}

/// A fade from 1 below `near` to 0 at `far` (1 also with `far` 0): `None` beyond `far`.
#[inline(always)]
fn fade<R: Real>(d: R, near: R, far: R) -> Option<u32> {
    if d < near || far == R::ZERO {
        Some(ONE)
    } else if far < d {
        None
    } else {
        Some((R::ONE - (d - near) / (far - near)).store())
    }
}

// Typed from 00b9e970
/// `ShadowSceneLight::TestFrustumCull` (`thiscall`, `RET 4`: the culler). The cull word: for a
/// shadow caster with a shadow camera, 0 when its volume meets the culler's camera's, else 0xff;
/// otherwise 0xff when the `NiLight`'s sphere is outside an active plane, and, when it crosses
/// one, the light casts shadows with the cube maps on and uses one, a bit per cube face that misses.
/// Unless culled, the distance from the culler's camera less the radius (scaled by the shader
/// accumulator's object's +0x110 when there is one) fades the dimmer (culled beyond its far
/// distance) and, for a shadow caster with a dimmer above 0, the shadow dimmer; the shadow dimmer
/// is then scaled by the shadow scene's property 2 (+0x3c) and the fade-in (the elapsed time, kept,
/// over the fade time, at most 1; reversed for a zero target). A change of the culled state, or a
/// dimmer reaching or leaving 0, marks every geometry's lighting property and, with the old shadow
/// dimmer 0, releases the shadow map. The word and both dimmers are stored.
pub fn test_frustum_cull(e: &mut Engine, this: Ptr<ShadowSceneLight>, culler: u32) {
    if float_precision() {
        test_frustum_cull_in::<f32>(e, this, culler);
    } else {
        test_frustum_cull_in::<f64>(e, this, culler);
    }
}

fn test_frustum_cull_in<R: Real>(e: &mut Engine, this: Ptr<ShadowSceneLight>, culler: u32) {
    let light = e.get(this, ShadowSceneLight::spLight);
    let position = three(e, light.wrapping_add(WORLD_TRANSLATE));
    let radius_bits = x87_quiet_bits(e.mem.u32(light.wrapping_add(RADIUS)));
    let radius = ld::<R>(radius_bits);
    let cast = e.get(this, ShadowSceneLight::bCastShadow) != 0;
    let shadow_camera = e.get(this, ShadowSceneLight::spShadowCamera);
    let mut result: u16 = 0;
    if cast && shadow_camera != 0 {
        let culler_camera = e.mem.u32(culler.wrapping_add(CULLER_CAMERA));
        let meets = typed_call!(
            FAMILY,
            0x00b9_c020,
            volumes_meet(e, this, Ptr::new(culler_camera), Ptr::new(shadow_camera))
        );
        result = if meets { 0 } else { CULLED };
    } else {
        let mask = e.mem.u32(culler.wrapping_add(PLANE_MASK));
        let [x, y, z] = position.map(ld::<R>);
        let mut inside = true;
        for plane in 0..6u32 {
            if mask & (1 << plane) == 0 {
                continue;
            }
            let at = culler.wrapping_add(CULLER_PLANES + 0x10 * plane);
            let [nx, ny, nz, d] = [0u32, 4, 8, 0xc].map(|o| ld::<R>(e.mem.u32(at + o)));
            let dot = ny * y + x * nx + nz * z;
            // `FCHS; FCOMPP; TEST AH,1; JZ`: outside unless `-radius < distance`.
            if -radius >= st(st(dot) - d) {
                result = CULLED;
                break;
            }
            if st(dot - d) < R::ZERO {
                inside = false;
            }
        }
        if result != CULLED
            && !inside
            && cast
            && cube_maps_on(e)
            && e.get(this, ShadowSceneLight::bCubeShadowMap) != 0
        {
            let [x, y, z] = position;
            for face in 0..6u16 {
                let meets = typed_call!(
                    FAMILY,
                    0x00b9_de90,
                    cube_face_meets(e, this, x, y, z, radius_bits, Ptr::new(culler), face)
                );
                if !meets {
                    result |= 1 << face;
                }
            }
        }
    }

    let old_dimmer = e.get(this, ShadowSceneLight::fLODDimmer).to_bits();
    let old_shadow = e.get(this, ShadowSceneLight::fShadowLODDimmer).to_bits();
    let mut dimmer = x87_quiet_bits(old_dimmer);
    let mut shadow = x87_quiet_bits(old_shadow);
    if result != CULLED {
        let culler_camera = e.mem.u32(culler.wrapping_add(CULLER_CAMERA));
        let camera = three(e, culler_camera.wrapping_add(WORLD_TRANSLATE));
        let [dx, dy, dz] = [0usize, 1, 2].map(|i| st(ld::<R>(position[i]) - ld::<R>(camera[i])));
        let squared = (dy * dy + dx * dx + dz * dz).store();
        let root = ld::<R>(f32::from_bits(squared).sqrt().to_bits());
        let mut distance = (root - radius).store();
        let accumulator = typed_call!(
            crate::render::FAMILY,
            0x00b4_f5c0,
            crate::render::passes::shader_accumulator(e)
        );
        if e.mem.u32(accumulator.wrapping_add(8)) != 0 {
            let accumulator = typed_call!(
                crate::render::FAMILY,
                0x00b4_f5c0,
                crate::render::passes::shader_accumulator(e)
            );
            let object = e.mem.u32(accumulator.wrapping_add(8));
            distance = (ld::<R>(e.mem.u32(object.wrapping_add(0x110))) * ld::<R>(distance)).store();
        }
        let d = ld::<R>(distance);
        let near = ld::<R>(e.global::<u32>(DIMMER_NEAR));
        let far = ld::<R>(e.global::<u32>(DIMMER_FAR));
        let fade_shadow = match fade(d, near, far) {
            Some(value) => {
                dimmer = value;
                R::ZERO < ld::<R>(value)
            }
            None => {
                result = CULLED;
                dimmer = 0;
                shadow = 0;
                false
            }
        };
        if fade_shadow && e.get(this, ShadowSceneLight::bCastShadow) != 0 {
            let near = ld::<R>(e.global::<u32>(SHADOW_NEAR));
            let far = ld::<R>(e.global::<u32>(SHADOW_FAR));
            shadow = fade(d, near, far).unwrap_or(0);
        }
        let scene = e.get(this, ShadowSceneLight::spShadowScene);
        if scene != 0 {
            let property = typed_call!(
                crate::cell_refs::FAMILY,
                0x00a5_9d30,
                crate::cell_refs::scenegraph::get_property(e, scene, 2)
            );
            if property != 0 {
                let factor = ld::<R>(e.mem.u32(property.wrapping_add(0x3c)));
                shadow = (factor * ld::<R>(shadow)).store();
            }
        }
        let elapsed = e
            .get(this, ShadowSceneLight::fShadowLODDimmerElapsed)
            .to_bits();
        let elapsed = (ld::<R>(elapsed) + ld::<R>(e.global::<u32>(ELAPSED_STEP))).store();
        e.set(
            this,
            ShadowSceneLight::fShadowLODDimmerElapsed,
            f32::from_bits(elapsed),
        );
        let mut fraction = (ld::<R>(elapsed) / ld::<R>(e.global::<u32>(FADE_TIME))).store();
        if ld::<R>(fraction) > R::ONE {
            fraction = ONE;
        }
        let target = e.get(this, ShadowSceneLight::fShadowLODDimmerTarget);
        shadow = if target == 0.0 {
            ((R::ONE - ld::<R>(fraction)) * ld::<R>(shadow)).store()
        } else {
            (ld::<R>(fraction) * ld::<R>(shadow)).store()
        };
    }

    // A dimmer that was 0 and is now above 0, or the other way round.
    let crossed = |old: u32, new: u32| {
        let (old, new) = (f32::from_bits(old), f32::from_bits(new));
        (old == 0.0 && new > 0.0) || (old > 0.0 && new == 0.0)
    };
    let frustum = e.get(this, ShadowSceneLight::usFrustumCull);
    if (frustum == CULLED) != (result == CULLED)
        || crossed(old_dimmer, dimmer)
        || crossed(old_shadow, shadow)
    {
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
            let property = e.mem.u32(geometry.wrapping_add(GEOMETRY_PROPERTY));
            if property != 0 && e.mem.u32(property.wrapping_add(0x1c)).wrapping_sub(1) <= 0xb {
                e.mem.set_u8(property.wrapping_add(0x74), 1);
            }
        }
        if f32::from_bits(old_shadow) == 0.0 {
            release_shadow_map(e, this);
        }
    }
    e.set(this, ShadowSceneLight::usFrustumCull, result);
    e.set(
        this,
        ShadowSceneLight::fLODDimmer,
        f32::from_bits(x87_quiet_bits(dimmer)),
    );
    e.set(
        this,
        ShadowSceneLight::fShadowLODDimmer,
        f32::from_bits(x87_quiet_bits(shadow)),
    );
}

// ---------------------------------------------------------------------------------------------
// 00b9dfc0: MTSetupFrustumDepthMap.

/// The renderer's free shadow map (`00b6d3f0`), the light's map setter (`00b5aee0`, `RET 4`),
/// the rendering system (`01200088`) and its `SetThreadStage` (`00ba30f0`, `RET 8`) and
/// `AddAccumTask` (`00ba3390`, `RET 0x24`); the dimmer limit (`double` 0.05 at `01082cb0`).
const RENDERER_GET_MAP: u32 = 0x00b6_d3f0;
const SET_SHADOW_MAP: u32 = 0x00b5_aee0;
const RENDERING_SYSTEM: u32 = 0x0120_0088;
const SET_THREAD_STAGE: u32 = 0x00ba_30f0;
const ADD_ACCUM_TASK: u32 = 0x00ba_3390;
const SHADOW_DIMMER_LIMIT: u32 = 0x0108_2cb0;
/// The sun direction (`011a9484`) and the fallback up vector (`011a946c`), three floats each; the
/// small-cross bounds (`double`s 0.01 at `01031148`, -0.01 at `010b7ea0`); `NiPoint3::UnitCross`
/// (`0053d1a0`, `RET 8`: out, other).
const SUN_DIRECTION: u32 = 0x011a_9484;
const FALLBACK_UP: u32 = 0x011a_946c;
const SMALL_POSITIVE: u32 = 0x0103_1148;
const SMALL_NEGATIVE: u32 = 0x010b_7ea0;
const UNIT_CROSS: u32 = 0x0053_d1a0;
/// `NiObjectNET::GetExtraData` (`00a5bdd0`, `RET 4`) of the name at `012031e4`; the extents'
/// scale (`double` 1.5 at `01016ff0`), cap (`float` 256.0 at `01032adc`), the field of view's
/// scale (`double` 150.0 at `0106c948`), the sun offset (`double` 10.0 at `01020758`), the far
/// plane's extra (`double` 400.0 at `0106b178`) and the near share (`double` 0.2 at `01018a90`).
const GET_EXTRA_DATA: u32 = 0x00a5_bdd0;
const EXTRA_DATA_NAME: u32 = 0x0120_31e4;
const EXTENT_SCALE: u32 = 0x0101_6ff0;
const EXTENT_CAP: u32 = 0x0103_2adc;
const FOV_SCALE: u32 = 0x0106_c948;
const SUN_OFFSET_SCALE: u32 = 0x0102_0758;
const FAR_EXTRA: u32 = 0x0106_b178;
const NEAR_SHARE: u32 = 0x0101_8a90;
/// The script test `005bf7b0`, the accumulator source `00b540f0`, `NiPointer` assignment
/// (`0066b0d0`, `RET 4`), the allocator (`00aa13e0`) and `NiCamera::NiCamera` (`00a712f0`, 0x114
/// bytes), `LookAtWorldPoint` (`00a701b0`, `RET 8`), `SetViewFrustum` (`00a6faf0`, `RET 4`), the
/// view port setter (`00712e60`, `RET 4`) and the near/far fit (`00a70080`, `RET 4`); the depth
/// pass flag (`011f91b0`).
const SCRIPT_TEST: u32 = 0x005b_f7b0;
const ACCUMULATOR_SOURCE: u32 = 0x00b5_40f0;
const NI_POINTER_ASSIGN: u32 = 0x0066_b0d0;
const NEW_OBJECT: u32 = 0x00aa_13e0;
const NI_CAMERA_CONSTRUCT: u32 = 0x00a7_12f0;
const LOOK_AT_WORLD_POINT: u32 = 0x00a7_01b0;
const SET_VIEW_FRUSTUM: u32 = 0x00a6_faf0;
const SET_VIEW_PORT: u32 = 0x0071_2e60;
const FIT_NEAR_FAR: u32 = 0x00a7_0080;
const DEPTH_PASS_FLAG: u32 = 0x011f_91b0;

/// The light's shadow map and accumulator released (the map handed back to the renderer first).
fn release_map_and_accumulator(e: &mut Engine, this: Ptr<ShadowSceneLight>) {
    if release_shadow_map(e, this) {
        let accumulator = e.get(this, ShadowSceneLight::spShadowAccumulator);
        if accumulator != 0 {
            release(e, accumulator);
            e.set(this, ShadowSceneLight::spShadowAccumulator, 0);
        }
    }
}

// Typed from 00b9dfc0
/// `ShadowSceneLight::MTSetupFrustumDepthMap` (`thiscall`, `RET 8`: an unread word, the light's
/// index). A light with the cube maps on (`bCastShadow`) or a shadow spotlight gets a shadow map
/// from the renderer when it has none (its `NiLight` culled when there is none to get). Unless it
/// is that kind of light, not culled, with a shadow dimmer above 0.05 and a map, its map and
/// accumulator are released; otherwise its orthographic frustum and shadow camera are set up
/// ([`setup_frustum`]). The thread stage of the index is set last.
pub fn setup_frustum_depth_map(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    _unused: u32,
    index: u32,
) {
    let wanted = (e.get(this, ShadowSceneLight::bCastShadow) != 0 && cube_maps_on(e))
        || e.get(this, ShadowSceneLight::bShadowSpotlight) != 0;
    if e.get(this, ShadowSceneLight::spDynamicShadowMap) == 0 {
        let renderer = e.global::<u32>(RENDERER);
        let map = e.call(RENDERER_GET_MAP, &args![renderer]).u32();
        if map == 0 {
            let light = e.get(this, ShadowSceneLight::spLight);
            let flags = e.mem.u32(light.wrapping_add(FLAGS));
            e.mem.set_u32(light.wrapping_add(FLAGS), flags | 1);
        }
        e.call(SET_SHADOW_MAP, &args![this, map]);
    }
    let dimmer = f64::from(e.get(this, ShadowSceneLight::fShadowLODDimmer));
    let limit: f64 = e.global(SHADOW_DIMMER_LIMIT);
    if wanted
        && e.get(this, ShadowSceneLight::usFrustumCull) != CULLED
        && dimmer > limit
        && e.get(this, ShadowSceneLight::spDynamicShadowMap) != 0
    {
        if float_precision() {
            setup_frustum::<f32>(e, this, index);
        } else {
            setup_frustum::<f64>(e, this, index);
        }
    } else {
        release_map_and_accumulator(e, this);
    }
    e.call(
        SET_THREAD_STAGE,
        &args![RENDERING_SYSTEM, 0u32, index.wrapping_add(0x11)],
    );
}

/// `NiAVObject::GetExtraData` of the extents' name on the shadow scene.
fn extents_data(e: &mut Engine, scene: u32) -> u32 {
    let name = e.global::<u32>(EXTRA_DATA_NAME);
    e.call(GET_EXTRA_DATA, &args![scene, name]).u32()
}

/// `NiPoint3::operator*` (`0045bb20`): the point at `point` times `scale` into `out`; answers `out`.
fn times(e: &mut Engine, point: u32, out: u32, scale: u32) -> u32 {
    typed_call!(
        crate::dispatch::FAMILY,
        0x0045_bb20,
        crate::dispatch::math::times(e, Ptr::new(point), Ptr::new(out), f32::from_bits(scale))
    )
    .addr()
}

/// `MTSetupFrustumDepthMap`'s main path (its frame's locals in one block, as the translation keeps
/// them).
fn setup_frustum<R: Real>(e: &mut Engine, this: Ptr<ShadowSceneLight>, index: u32) {
    e.with_stack(0xd0, |e, frame| {
        // Locals: D (+0x00), R (+0x0c), U (+0x18), a cross product (+0x24), up (+0x30), P (+0x3c),
        // Q (+0x48), the extents (+0x54), the NiFrustum (+0x60, 0x1c bytes), the view port (+0x7c),
        // the light's transform (+0x8c, 0x34 bytes), a scaled point (+0xc0).
        let f = frame.addr();
        let (d, r, u, cross, up) = (f, f + 0x0c, f + 0x18, f + 0x24, f + 0x30);
        let (p, q, extents, frustum) = (f + 0x3c, f + 0x48, f + 0x54, f + 0x60);
        let (port, transform, scaled) = (f + 0x7c, f + 0x8c, f + 0xc0);
        let light = e.get(this, ShadowSceneLight::spLight);
        copy_words(e, transform, light.wrapping_add(0x68), 0xd);
        let origin = three(e, light.wrapping_add(WORLD_TRANSLATE));
        let scene = e.get(this, ShadowSceneLight::spShadowScene);
        let target = three(e, scene.wrapping_add(WORLD_TRANSLATE));
        set_three(
            e,
            d,
            [0usize, 1, 2].map(|i| (ld::<R>(target[i]) - ld::<R>(origin[i])).store()),
        );
        typed_call!(
            crate::dispatch::FAMILY,
            0x004a_0c10,
            crate::dispatch::math::normalize(e, Ptr::new(d))
        );
        let sun = three(e, SUN_DIRECTION);
        set_three(e, up, sun.map(|w| x87_quiet_bits(w) ^ SIGN));
        e.call(UNIT_CROSS, &args![d, r, up]);
        let (small_positive, small_negative): (f64, f64) =
            (e.global(SMALL_POSITIVE), e.global(SMALL_NEGATIVE));
        let small = three(e, r).iter().all(|&w| {
            let v = f64::from(f32::from_bits(w));
            small_negative < v && v < small_positive
        });
        if small {
            copy_words(e, up, FALLBACK_UP, 3);
            let result = e.call(UNIT_CROSS, &args![d, cross, up]).u32();
            copy_words(e, r, result, 3);
        }
        let result = e.call(UNIT_CROSS, &args![d, cross, r]).u32();
        copy_words(e, u, result, 3);
        // `NiFrustum::NiFrustum(false)` (`00a71b70`), then orthographic.
        e.mem.set_u8(frustum + 0x18, 1);
        let extra = extents_data(e, scene);
        let scene = e.get(this, ShadowSceneLight::spShadowScene);
        set_three(e, d, three(e, scene.wrapping_add(WORLD_TRANSLATE)));
        let source = if extra != 0 {
            let scale = x87_quiet_bits(e.mem.u32(scene.wrapping_add(0x98)));
            times(e, extra.wrapping_add(0x18), cross, scale)
        } else {
            ZERO_POINT
        };
        copy_words(e, extents, source, 3);
        let radius = x87_quiet_bits(e.mem.u32(bound_or_null(e, scene).wrapping_add(0xc)));
        let factor: f64 = e.global(EXTENT_SCALE);
        // The larger of each scaled extent (z, y, x) and the radius (`00404010`), then the smaller of
        // the cap and that (`0040ebd0`), each through a float.
        let mut larger = radius;
        for i in [2u32, 1, 0] {
            let extent = f32::from_bits(e.mem.u32(extents + 4 * i));
            let scaled = with_double(extent, factor, |a, b| a * b);
            if f32::from_bits(larger) < f32::from_bits(scaled) {
                larger = scaled;
            }
        }
        let larger = x87_quiet_bits(larger);
        let cap = x87_quiet_bits(e.global::<u32>(EXTENT_CAP));
        let width = if f32::from_bits(larger) > f32::from_bits(cap) {
            cap
        } else {
            larger
        };
        let toward = three(e, d);
        let light = e.get(this, ShadowSceneLight::spLight);
        let origin_now = three(e, light.wrapping_add(WORLD_TRANSLATE));
        set_three(
            e,
            q,
            [0usize, 1, 2].map(|i| (ld::<R>(origin_now[i]) - ld::<R>(toward[i])).store()),
        );
        let length = typed_call!(
            crate::pathing::FAMILY,
            0x0045_7990,
            crate::pathing::point::length(e, Ptr::new(q))
        );
        let fov_scale: f64 = e.global(FOV_SCALE);
        let ratio = ld::<R>(width) / ld::<R>(length.to_bits());
        e.set(
            this,
            ShadowSceneLight::fSpotlightFOV,
            to_float(ratio.wide() * fov_scale),
        );
        // Left, right, top, bottom (`FCHS` of the width), near 1.0 and far the light's radius.
        let negative = width ^ SIGN;
        for (offset, word) in [(0u32, negative), (0xc, negative), (4, width), (8, width)] {
            e.mem.set_u32(frustum + offset, word);
        }
        e.mem.set_u32(frustum + 0x10, ONE);
        e.mem.set_u32(
            frustum + 0x14,
            x87_quiet_bits(e.mem.u32(light.wrapping_add(RADIUS))),
        );
        for (offset, word) in [(0u32, 0), (0xc, 0), (4, ONE), (8, ONE)] {
            e.mem.set_u32(port + offset, word);
        }
        if e.get(this, ShadowSceneLight::bCastShadow) == 0
            || e.call(SCRIPT_TEST, &args![]).u8() == 0
        {
            return;
        }
        let locals = Locals {
            p,
            q,
            u,
            frustum,
            port,
            scaled,
        };
        shadow_camera::<R>(e, this, index, origin, &locals);
    });
}

/// The frame's locals the shadow camera's set-up passes on.
struct Locals {
    p: u32,
    q: u32,
    u: u32,
    frustum: u32,
    port: u32,
    scaled: u32,
}

/// The update of `object` with the zeroed update data in a frame of its own (as the exe's
/// `NiUpdateData` local).
fn update_object(e: &mut Engine, object: u32) {
    e.with_stack(0xc, |e, data| update_with_zeros(e, object, data.addr()));
}

/// `MTSetupFrustumDepthMap`'s shadow camera: the accumulator from its source (its +0x19c 7), a
/// new `NiCamera` when there is none, placed at the light, updated and pointed at the shadow
/// scene (moved by ten times the sun direction when the extents' x plus y are below their z) with
/// `U` up; the frustum and view port set, the camera updated and fitted to the scene's bound; its
/// planes set from its frustum with the far plane 400 further (and the near plane at a fifth of
/// that without the depth pass), the frustum set again; the accumulator holds the light and the
/// accumulation task is added.
fn shadow_camera<R: Real>(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    index: u32,
    origin: [u32; 3],
    l: &Locals,
) {
    let base = this.addr();
    let accumulator_slot = base.wrapping_add(ShadowSceneLight::spShadowAccumulator.off);
    let camera_slot = base.wrapping_add(ShadowSceneLight::spShadowCamera.off);
    let accumulator = e.call(ACCUMULATOR_SOURCE, &args![]).u32();
    e.call(NI_POINTER_ASSIGN, &args![accumulator_slot, accumulator]);
    let accumulator = e.get(this, ShadowSceneLight::spShadowAccumulator);
    e.mem.set_u32(accumulator.wrapping_add(0x19c), 7);
    if e.get(this, ShadowSceneLight::spShadowCamera) == 0 {
        let memory = e.call(NEW_OBJECT, &args![0x114u32]).u32();
        let camera = if memory != 0 {
            e.call(NI_CAMERA_CONSTRUCT, &args![memory]).u32()
        } else {
            0
        };
        e.call(NI_POINTER_ASSIGN, &args![camera_slot, camera]);
    }
    let camera = e.get(this, ShadowSceneLight::spShadowCamera);
    set_three(e, camera.wrapping_add(0x58), origin);
    update_object(e, camera);
    let scene = e.get(this, ShadowSceneLight::spShadowScene);
    set_three(e, l.p, three(e, scene.wrapping_add(WORLD_TRANSLATE)));
    let extra = extents_data(e, scene);
    let scene = e.get(this, ShadowSceneLight::spShadowScene);
    let scale = x87_quiet_bits(e.mem.u32(scene.wrapping_add(0x98)));
    let moved = times(e, extra.wrapping_add(0xc), l.scaled, scale);
    typed_call!(
        crate::pathing::FAMILY,
        0x0063_c8a0,
        crate::pathing::point::add_to(e, Ptr::new(l.p), Ptr::new(moved))
    );
    let scene = e.get(this, ShadowSceneLight::spShadowScene);
    let scale = x87_quiet_bits(e.mem.u32(scene.wrapping_add(0x98)));
    times(e, extra.wrapping_add(0x18), l.q, scale);
    let [qx, qy, qz] = three(e, l.q).map(ld::<R>);
    if qy + qx < qz {
        let ten = f64::from_bits(e.global::<u64>(SUN_OFFSET_SCALE));
        let sun = three(e, SUN_DIRECTION);
        let offset = sun.map(|w| with_double(f32::from_bits(w), ten, |a, b| a * b));
        set_three(e, l.q, offset);
        let p = three(e, l.p);
        set_three(
            e,
            l.p,
            [0usize, 1, 2].map(|i| (ld::<R>(offset[i]) + ld::<R>(p[i])).store()),
        );
    }
    let camera = e.get(this, ShadowSceneLight::spShadowCamera);
    e.call(LOOK_AT_WORLD_POINT, &args![camera, l.p, l.u]);
    e.call(SET_VIEW_FRUSTUM, &args![camera, l.frustum]);
    e.call(SET_VIEW_PORT, &args![camera, l.port]);
    update_object(e, camera);
    let saved_near = x87_quiet_bits(e.mem.u32(l.frustum + 0x10));
    let scene = e.get(this, ShadowSceneLight::spShadowScene);
    let bound = bound_or_null(e, scene);
    let saved_far = x87_quiet_bits(e.mem.u32(l.frustum + 0x14));
    let camera = e.get(this, ShadowSceneLight::spShadowCamera);
    e.call(FIT_NEAR_FAR, &args![camera, bound]);
    let depth_pass = e.global::<u32>(DEPTH_PASS_FLAG);
    let camera = e.get(this, ShadowSceneLight::spShadowCamera);
    copy_words(e, l.frustum, camera.wrapping_add(CAMERA_FRUSTUM), 7);
    let extra_far: f64 = e.global(FAR_EXTRA);
    let far = with_double(
        f32::from_bits(e.mem.u32(l.frustum + 0x14)),
        extra_far,
        |a, b| a + b,
    );
    e.mem.set_u32(l.frustum + 0x14, far);
    if depth_pass == 0 {
        let share: f64 = e.global(NEAR_SHARE);
        let near = with_double(f32::from_bits(far), share, |a, b| a * b);
        e.mem.set_u32(l.frustum + 0x10, near);
    }
    typed_call!(
        crate::culling::FAMILY,
        0x00a7_4e10,
        crate::culling::planes::frustum_planes_set(
            e,
            Ptr::new(base.wrapping_add(ShadowSceneLight::planes.off)),
            Ptr::new(l.frustum),
            Ptr::new(camera.wrapping_add(CAMERA_ROTATE))
        )
    );
    e.mem.set_u32(l.frustum + 0x10, saved_near);
    e.mem.set_u32(l.frustum + 0x14, saved_far);
    let camera = e.get(this, ShadowSceneLight::spShadowCamera);
    e.call(SET_VIEW_FRUSTUM, &args![camera, l.frustum]);
    let accumulator = e.get(this, ShadowSceneLight::spShadowAccumulator);
    typed_call!(
        FAMILY,
        0x00b9_cfd0,
        set_held_1a0(e, Ptr::new(accumulator), base)
    );
    let accumulator = e.get(this, ShadowSceneLight::spShadowAccumulator);
    let scene = e.get(this, ShadowSceneLight::spShadowScene);
    let camera = e.get(this, ShadowSceneLight::spShadowCamera);
    e.call(
        ADD_ACCUM_TASK,
        &args![
            RENDERING_SYSTEM,
            camera,
            0u32,
            scene,
            0u32,
            0u32,
            accumulator,
            0u32,
            index.wrapping_add(0x11),
            0u32
        ],
    );
}

// ---------------------------------------------------------------------------------------------
// 00b9f780: MTRenderFrustumDepthMap.

/// The rendering system's target select (`00ba3130`, `RET 8`); the renderer (`011f4748`) and its
/// slots +0xb4 (its clear colour into a local) and +0xac (set the clear colour); the colour the
/// local starts as (`011a9bd0`, four words); `BSRenderedTexture::StopOffscreen` (`00b6b260`), the
/// clear `00b6b8d0` (`cdecl`: 7, the target); the state object (`011f9508`, its view and
/// projection at +0x980 and +0x9c0); the renderer's step after the camera (`00b6b790`: `00b6b6c0`,
/// then its state +0x200 from 2 back to 0 when its slot +0x180 answers true); the depth pass's locals (`00ba37a0` with the
/// map, `00ba3a00`; `004eba20`, `004eba90`), the depth value (`010162c0` to `012003c4`), the pass
/// `00ba3cb0` (`RET 0xc`, on `[[011f91ac] + 8] + 0x44`) and the render `00b97550` (`RET 0x18`, on
/// `[011f91ac]`); the debug texture's `004bc320` (`RET 4`), `00b55560` (`cdecl`, 9) and
/// `00bd22c0` (`RET 4`).
const SELECT_TARGET: u32 = 0x00ba_3130;
const RENDERER_SINGLETON: u32 = 0x011f_4748;
const SLOT_GET_CLEAR_COLOUR: u32 = 0xb4;
const SLOT_SET_CLEAR_COLOUR: u32 = 0xac;
const CLEAR_COLOUR: u32 = 0x011a_9bd0;
const STOP_OFFSCREEN: u32 = 0x00b6_b260;
const CLEAR_TARGET: u32 = 0x00b6_b8d0;
const STATE_OBJECT: u32 = 0x011f_9508;
const RENDERER_STEP: u32 = 0x00b6_b790;
const PASS_LOCAL: u32 = 0x00ba_37a0;
const PASS_LOCAL_DESTRUCT: u32 = 0x00ba_3a00;
const PASS_LIST: u32 = 0x004e_ba20;
const PASS_LIST_DESTRUCT: u32 = 0x004e_ba90;
const DEPTH_VALUE: u32 = 0x0101_62c0;
const DEPTH_VALUE_SET: u32 = 0x0120_03c4;
const RENDER_MANAGER: u32 = 0x011f_91ac;
const PASS_ADD: u32 = 0x00ba_3cb0;
const RENDER_PASS: u32 = 0x00b9_7550;
const DEBUG_TEXTURE: u32 = 0x004b_c320;
const DEBUG_SHOWER: u32 = 0x00b5_5560;
const DEBUG_SHOW: u32 = 0x00bd_22c0;

/// The light's translation in the camera's frame, negated: `-((t.y r.y + t.x r.x) + t.z r.z)`
/// for a rotation row `r` (stored, then `FCHS`).
#[inline(always)]
fn camera_frame<R: Real>(t: [R; 3], row: [u32; 3]) -> u32 {
    let [x, y, z] = row.map(ld::<R>);
    (t[1] * y + t[0] * x + t[2] * z).store() ^ SIGN
}

// Typed from 00b9f780
/// `ShadowSceneLight::MTRenderFrustumDepthMap` (`thiscall`, `RET 8`: an object whose `NiPointer`
/// +0xc4 holds the accumulator meanwhile, the light's index). Selects target 0x11 + index, clears
/// the shadow map's target in white (the renderer's colour kept and put back), draws the shadow
/// camera's accumulator (released after, with its +0x1a0 and the object's +0xc4), hands the camera
/// to the state object, copies its view to `kShadowView` with the light's translation in the
/// camera's frame in the last row, its projection to `kShadowProj`, and `kViewProjection =
/// kShadowView * kShadowProj`; then the depth pass into a map from the renderer unless
/// `[011f91b0]`, the map shown with `bShowDebugTexture`, and `bClipSpacePlanesReady` cleared.
pub fn render_frustum_depth_map(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    holder: u32,
    index: u32,
) {
    if float_precision() {
        render_frustum_depth_map_in::<f32>(e, this, holder, index);
    } else {
        render_frustum_depth_map_in::<f64>(e, this, holder, index);
    }
}

fn render_frustum_depth_map_in<R: Real>(
    e: &mut Engine,
    this: Ptr<ShadowSceneLight>,
    holder: u32,
    index: u32,
) {
    let base = this.addr();
    e.call(
        SELECT_TARGET,
        &args![RENDERING_SYSTEM, 1u32, index.wrapping_add(0x11)],
    );
    e.with_stack(0x24, |e, colours| {
        let saved = colours.addr();
        let white = saved + 0x10;
        copy_words(e, saved, CLEAR_COLOUR, 4);
        let renderer = e.global::<u32>(RENDERER_SINGLETON);
        e.vcall(renderer, SLOT_GET_CLEAR_COLOUR, &args![saved]);
        for word in 0..4u32 {
            e.mem.set_u32(white + 4 * word, ONE);
        }
        let renderer = e.global::<u32>(RENDERER_SINGLETON);
        e.vcall(renderer, SLOT_SET_CLEAR_COLOUR, &args![white]);
        let map = e.get(this, ShadowSceneLight::spDynamicShadowMap);
        let target = e.call(STOP_OFFSCREEN, &args![map]).u32();
        e.call(CLEAR_TARGET, &args![7u32, target]);
        let renderer = e.global::<u32>(RENDERER_SINGLETON);
        e.vcall(renderer, SLOT_SET_CLEAR_COLOUR, &args![saved]);
    });
    // The holder's `NiPointer` +0xc4 takes the accumulator.
    let accumulator = e.get(this, ShadowSceneLight::spShadowAccumulator);
    let slot = holder.wrapping_add(0xc4);
    let old = e.mem.u32(slot);
    if old != accumulator {
        if old != 0 {
            release(e, old);
        }
        e.mem.set_u32(slot, accumulator);
        if accumulator != 0 {
            interlocked_increment(e, accumulator.wrapping_add(4));
        }
    }
    let camera = e.get(this, ShadowSceneLight::spShadowCamera);
    let accumulator = e.get(this, ShadowSceneLight::spShadowAccumulator);
    typed_call!(
        crate::draw::FAMILY,
        0x00b6_c0d0,
        crate::draw::drivers::scene_draw(e, camera, Ptr::new(accumulator))
    );
    let accumulator = e.get(this, ShadowSceneLight::spShadowAccumulator);
    let held = e.mem.u32(accumulator.wrapping_add(0x1a0));
    if held != 0 {
        release(e, held);
        e.mem.set_u32(accumulator.wrapping_add(0x1a0), 0);
    }
    let held = e.mem.u32(slot);
    if held != 0 {
        release(e, held);
        e.mem.set_u32(slot, 0);
    }

    let camera = e.get(this, ShadowSceneLight::spShadowCamera);
    let state = e.global::<u32>(STATE_OBJECT);
    // The camera's rotation rows, read before the hand-over as the exe does.
    let row = |offsets: [u32; 3]| offsets.map(|o| e.mem.u32(camera.wrapping_add(o)));
    let (c, b, a) = (
        row([0x70, 0x7c, 0x88]),
        row([0x6c, 0x78, 0x84]),
        row([0x68, 0x74, 0x80]),
    );
    typed_call!(
        crate::draw::FAMILY,
        0x004e_9bb0,
        crate::draw::packet::hand_camera(e, state, camera)
    );
    let light = e.get(this, ShadowSceneLight::spLight);
    e.with_stack(0xb4, |e, frame| {
        // The frame: the view (+0), the projection (+0x40), the light's transform (+0x80); only
        // the projection is passed on, as `D3DXMatrixMultiply`'s out.
        let projection = frame.addr() + 0x40;
        let t = three(e, light.wrapping_add(WORLD_TRANSLATE)).map(ld::<R>);
        let shadow_view = base.wrapping_add(ShadowSceneLight::kShadowView.off);
        let shadow_projection = base.wrapping_add(ShadowSceneLight::kShadowProj.off);
        copy_words(e, shadow_view, state.wrapping_add(0x980), 0x10);
        set_three(
            e,
            shadow_view + 0x30,
            [camera_frame(t, c), camera_frame(t, b), camera_frame(t, a)],
        );
        copy_words(e, projection, state.wrapping_add(0x9c0), 0x10);
        copy_words(e, shadow_projection, projection, 0x10);
        let _: Ptr = typed_call!(
            crate::draw::FAMILY,
            0x00ee_6de0,
            crate::draw::thunks::matrix_multiply(
                e,
                Ptr::new(projection),
                Ptr::new(shadow_view),
                Ptr::new(shadow_projection)
            )
        );
        copy_words(
            e,
            base.wrapping_add(ShadowSceneLight::kViewProjection.off),
            projection,
            0x10,
        );
    });
    e.call(RENDERER_STEP, &args![]);
    if e.global::<u32>(DEPTH_PASS_FLAG) == 0 {
        depth_pass(e, this);
    }
    if e.get(this, ShadowSceneLight::bShowDebugTexture) != 0 {
        let map = e.get(this, ShadowSceneLight::spDynamicShadowMap);
        let texture = e.call(DEBUG_TEXTURE, &args![map, 0u32]).u32();
        let shower = e.call(DEBUG_SHOWER, &args![9u32]).u32();
        e.call(DEBUG_SHOW, &args![shower, texture]);
    }
    e.set(this, ShadowSceneLight::bClipSpacePlanesReady, 0);
}

/// The depth pass of `MTRenderFrustumDepthMap`: a map from the renderer, the pass locals (a
/// 16-byte one on the map, a list), the depth value, the pass added and rendered into the light's
/// shadow map, the map handed back, the locals destroyed.
fn depth_pass(e: &mut Engine, this: Ptr<ShadowSceneLight>) {
    let renderer = e.global::<u32>(RENDERER);
    let map = e.call(RENDERER_GET_MAP, &args![renderer]).u32();
    e.with_stack(0x34, |e, frame| {
        let pass = frame.addr();
        let list = pass + 0x10;
        for byte in 0..3u32 {
            e.mem.set_u8(pass + byte, 0);
        }
        for word in 1..4u32 {
            e.mem.set_u32(pass + 4 * word, 0);
        }
        e.call(PASS_LOCAL, &args![pass, map]);
        e.call(PASS_LIST, &args![list]);
        let value = x87_quiet_bits(e.global::<u32>(DEPTH_VALUE));
        e.set_global(DEPTH_VALUE_SET, value);
        let manager = e.global::<u32>(RENDER_MANAGER);
        let inner = e.mem.u32(manager.wrapping_add(8));
        let target = e.mem.u32(inner.wrapping_add(0x44));
        e.call(PASS_ADD, &args![target, 2u32, pass, 0u32]);
        let shadow_map = e.get(this, ShadowSceneLight::spDynamicShadowMap);
        let state = e.global::<u32>(STATE_OBJECT);
        let manager = e.global::<u32>(RENDER_MANAGER);
        e.call(
            RENDER_PASS,
            &args![manager, 0x11u32, state, shadow_map, shadow_map, list, 1u32],
        );
        let renderer = e.global::<u32>(RENDERER);
        e.call(RENDERER_RELEASE_MAP, &args![renderer, map]);
        e.call(PASS_LIST_DESTRUCT, &args![list]);
        e.call(PASS_LOCAL_DESTRUCT, &args![pass]);
    });
}
