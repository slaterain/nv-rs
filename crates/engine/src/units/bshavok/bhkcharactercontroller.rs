//! `BSHavok/bhkcharactercontroller.obj` (Xbox PDB source unit), subsystem `BSHavok`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! `bhkCharacterController` (Xbox PDB, 0x650 bytes) is the Havok character controller of
//! the player and the NPCs: a `bhkCharacterProxy` (+0x000) followed by the
//! `hkpCharacterContext` (+0x3E0), the `bhkCharacterListener` (+0x410, the
//! `hkpCharacterProxyListener` whose callbacks receive `this + 0x410`) and the
//! controller's own state. The PC offsets used here agree with the Xbox PDB.
//!
//! Floats: x87 code is computed in `f64` and rounded to `f32` at each `float` store;
//! `FLD`/`FSTP` copies go through [`x87_quiet`]; SSE code (`MOVSS`, `MULPS`, `SQRTSS`,
//! `RSQRTSS`) computes in `f32`. Comparisons follow the exe's flag tests: `TEST AH,5; JP`
//! after `FCOM a, b` is `a < b`, `TEST AH,0x41; JNZ` skips unless `a > b`, and
//! `TEST AH,0x44; JNP` skips only on `==` (NaN passes).
//!
//! The exception-unwinding frames (`FS:[0]`) and the `/GS` cookie checks (`00ec408c`)
//! of the functions marked `Translation incomplete` are not modeled.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::units::fallout_shared::tesobjectrefr::rsqrt_estimate;

/// Offset of the `bhkCharacterListener` base in the controller: the listener callbacks
/// get `controller + 0x410` as `this`.
const LISTENER: u32 = 0x410;
/// Per-thread word the Havok-allocating functions set to 0x10 (`TLS + 0x2b4`).
const SCOPE_WORD: u32 = 0x2b4;

/// `hkMemoryRouter` lookup (`00c85750`): its `+0x10` is the allocator whose slot `+4`
/// allocates `(bytes)` and slot `+8` frees `(pointer, bytes)`.
const MEMORY_ROUTER: u32 = 0x00c8_5750;
/// `hkReferencedObject` functions of `hkreferencedobject.cpp`: `00c90510` takes a
/// reference, `00c905b0` releases one (`this` only).
const ADD_REFERENCE: u32 = 0x00c9_0510;
const REMOVE_REFERENCE: u32 = 0x00c9_05b0;
/// Sized delete of the game heap (`00aa1460`, cdecl `(pointer, size)`).
const FREE_SIZED: u32 = 0x00aa_1460;
/// The game heap's allocator (`00aa13e0`, cdecl `(size)`).
const HEAP_ALLOCATE: u32 = 0x00aa_13e0;
/// `MemoryManager` deallocation (`00aa4060`, `this` = the manager at `011f6238`).
const MEMORY_DEALLOCATE: u32 = 0x00aa_4060;
const MEMORY_MANAGER: u32 = 0x011f_6238;
/// `kernel32` `InterlockedDecrement` import slot.
const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// The empty fixed-string handle the string releases compare against.
const EMPTY_FIXED_STRING: u32 = 0x0109_b220;

/// `bhkCharacterProxy::UnMarkRead` (Xbox PDB, `00c89720`) and `UnMarkWrite`
/// (`00cac110`): thunks through the object's vtable slots 0x94 and 0x98.
const UN_MARK_READ: u32 = 0x00c8_9720;
const UN_MARK_WRITE: u32 = 0x00ca_c110;
/// `hkpCharacterProxy`'s `m_shapePhantom` getter (`00cac980`, `MOV EAX,[ECX+0x30]`).
const GET_SHAPE_PHANTOM: u32 = 0x00ca_c980;
/// `hkpCharacterProxy::getPosition` (Xbox PDB, `00cac910`) and `getLinearVelocity`
/// (`00cac950`).
const HKP_GET_POSITION: u32 = 0x00ca_c910;
const HKP_GET_LINEAR_VELOCITY: u32 = 0x00ca_c950;
/// `hkpCharacterProxy::integrateWithCollectors` (Xbox PDB, `00cafbd0`).
const INTEGRATE_WITH_COLLECTORS: u32 = 0x00ca_fbd0;
/// `bhkCharacterPointCollector` constructor, `handleCollisions` and destructor (Xbox PDB).
const POINT_COLLECTOR_CONSTRUCT: u32 = 0x00cd_3410;
const POINT_COLLECTOR_HANDLE_COLLISIONS: u32 = 0x00cd_32b0;
const POINT_COLLECTOR_DESTRUCT: u32 = 0x00cd_3490;
/// `bhkCharacterProxy::GethkPosition` (Xbox PDB, `00621a40`) and `GethkLinearVelocity`
/// (`00807080`): return a pointer to a vector.
const GET_HK_POSITION: u32 = 0x0062_1a40;
const GET_HK_LINEAR_VELOCITY: u32 = 0x0080_7080;
/// `bhkCharacterProxy` destructor body (`00cabf50`).
const CHARACTER_PROXY_DESTRUCT: u32 = 0x00ca_bf50;
/// Called on the proxy with the new cinfo (`00cabfd0`, one stack argument).
const SET_PROXY_CINFO: u32 = 0x00ca_bfd0;
/// Constructs the 0xC0-byte cinfo (`00c6da50`, one stack argument; this unit, not
/// translated yet).
const CINFO_CONSTRUCT: u32 = 0x00c6_da50;
/// `hkpWorld::removePhantom` / `addPhantom` (Xbox PDB).
const WORLD_REMOVE_PHANTOM: u32 = 0x00c9_2c70;
const WORLD_ADD_PHANTOM: u32 = 0x00c9_36a0;
/// `hkpShapePhantom::setTransform` (Xbox PDB).
const PHANTOM_SET_TRANSFORM: u32 = 0x00c9_e910;
/// `hkpWorldObject::addProperty` (Xbox PDB, `(key, value low, value high)`).
const ADD_PROPERTY: u32 = 0x00cd_2be0;
/// `hkpMotion::getMass` (Xbox PDB), result in ST0.
const GET_MASS: u32 = 0x00c8_6430;
/// `bhkRigidBody`-unit test on a motion-type byte (`00c8cce0`, cdecl, pointer to it).
const MOTION_TYPE_TEST: u32 = 0x00c8_cce0;
/// `hkVector4::setTransformedPos`, `setTransformedInversePos`, `setRotatedDir`
/// (Xbox PDB): `this` = the output vector, then the transform and the input vector.
const SET_TRANSFORMED_POS: u32 = 0x00c7_f640;
const SET_TRANSFORMED_INVERSE_POS: u32 = 0x00c7_f690;
const SET_ROTATED_DIR: u32 = 0x00c7_f700;
/// `hkRotation` set from a quaternion (`00cb2d90`, `hkrotation.cpp`) and
/// `hkMatrix3::mul` (Xbox PDB, `00cd5680`).
const ROTATION_SET: u32 = 0x00cb_2d90;
const MATRIX3_MUL: u32 = 0x00cd_5680;
/// Called on the quaternion copy before the rotation is built (`005611c0`).
const QUATERNION_PREPARE: u32 = 0x0056_11c0;
/// `bhkWorldObject::SetAVObject` (Xbox PDB), `GetAVObject`, and the
/// `bhkworldobject.obj` function `00c85ce0` (one stack argument).
const SET_AV_OBJECT: u32 = 0x00c8_5c10;
const GET_AV_OBJECT: u32 = 0x00c8_5c80;
const WORLD_OBJECT_PASS: u32 = 0x00c8_5ce0;
/// Lock call that the PC build compiled to a bare `RET` (`00a29680`).
const EMPTY_LOCK: u32 = 0x00a2_9680;
/// `bhkCollisionBox::Update` (Xbox PDB), `::Init`, and the destructor.
const COLLISION_BOX_UPDATE: u32 = 0x00cb_e8d0;
const COLLISION_BOX_INIT: u32 = 0x00cb_e9f0;
const COLLISION_BOX_DESTRUCT: u32 = 0x00cb_e880;
/// `bhkCachingShapePhantom::Init` and destructor (Xbox PDB).
const CACHING_PHANTOM_INIT: u32 = 0x00cc_2530;
const CACHING_PHANTOM_DESTRUCT: u32 = 0x00cc_2350;
/// `bhkListShape::Init` and destructor (Xbox PDB).
const LIST_SHAPE_INIT: u32 = 0x00c8_b090;
const LIST_SHAPE_DESTRUCT: u32 = 0x00c8_adf0;
/// `bhkShapeCollection` destructor (Xbox PDB).
const SHAPE_COLLECTION_DESTRUCT: u32 = 0x00cd_2de0;
/// Base constructor every `bhkRefObject`-derived constructor here starts with
/// (`00c87c30`: vtable `010c5b54`, clears `+8`).
const REF_OBJECT_CONSTRUCT: u32 = 0x00c8_7c30;
/// `bhkshape.obj` function `00c84f10` (one stack argument, returns the layer number).
const SHAPE_LAYER: u32 = 0x00c8_4f10;
/// `NiObject` cast (`00653290`, `(object, rtti)`: the object when it is of that
/// class, else 0).
const IS_KIND_OF: u32 = 0x0065_3290;
/// `NiNode::ms_RTTI` (`NiNode::GetRTTI` returns it) and `bhkCapsuleShape::ms_RTTI`.
const NODE_RTTI: u32 = 0x011f_4428;
const CAPSULE_SHAPE_RTTI: u32 = 0x0126_832c;
/// `NiFixedString` creation (`00a5b690`, cdecl), `NiObjectNET::SetName` (`00a5b950`).
const FIXED_STRING_CREATE: u32 = 0x00a5_b690;
const SET_NAME: u32 = 0x00a5_b950;
/// `"bhkColDisp"`.
const DISPLAY_NAME: u32 = 0x010c_43d8;
/// `NiNode` constructor (`00a5ecb0`, one stack argument) and the node's size.
const NODE_CONSTRUCT: u32 = 0x00a5_ecb0;
const NODE_SIZE: u32 = 0xac;
/// Child getter of the `tes.cpp` unit (`0045bc00`, one stack argument).
const NODE_CHILD_AT: u32 = 0x0045_bc00;
/// Point copy of `tes.cpp` (`00458620`, cdecl `(destination, source)`, returns the
/// destination).
const COPY_POINT: u32 = 0x0045_8620;
/// Filter-info builder for a ray of this controller (`0070c440`, writes a word through
/// its one stack argument).
const RAY_FILTER_INFO: u32 = 0x0070_c440;
/// `hkpCharacterContext` state getter (`00ca9600`, `MOV EAX,[ECX+0x10]`).
const GET_STATE: u32 = 0x00ca_9600;
/// `bhkCharacterController::SetVelocityModifier`, `GetRadius` and `GetPosition_ov2`
/// (Xbox PDB; this unit, not translated yet).
const SET_VELOCITY_MODIFIER: u32 = 0x00c6_d4d0;
const GET_RADIUS: u32 = 0x00c6_e280;
const GET_POSITION: u32 = 0x00c6_e300;
/// CRT x87 helpers taking ST0: `00ec6040` (square root), `00ec9f70` and `00eca0a0`.
const X87_SQRT: u32 = 0x00ec_6040;
const X87_FIRST_TRIG: u32 = 0x00ec_9f70;
const X87_SECOND_TRIG: u32 = 0x00ec_a0a0;
/// cdecl float helpers: `0040ebd0(a, b)` returns `a` when `b > a`, else `b`;
/// `00404010(a, b)` returns `a` when `b < a`, else `b`.
const FLOAT_MIN: u32 = 0x0040_ebd0;
const FLOAT_MAX: u32 = 0x0040_4010;

/// The 16-byte vector at `01267e30` (zero in the exe).
const ZERO_VECTOR: u32 = 0x0126_7e30;
/// `float` constants: `0.0`, `1.0`, `3.0`, `0.5`, `-0.0`.
const FLOAT_ZERO: u32 = 0x0101_1d78;
const FLOAT_ONE: u32 = 0x0101_2068;
const FLOAT_THREE: u32 = 0x0101_7718;
const FLOAT_HALF: u32 = 0x0101_6248;
const FLOAT_NEGATIVE_ZERO: u32 = 0x010c_4990;
/// `double` constants: `0.5`, `-1.0`, `0.05` (as a float), `25.0`, `1000.0`, `0.9` (as a
/// float), `0.25`, `32.0`, `20.0`, `-0.25`.
const HALF: u32 = 0x0101_1588;
const MINUS_ONE: u32 = 0x0101_a6b0;
const ROT_MOD_DECAY: u32 = 0x0103_c7c8;
const PROBE_DEPTH: u32 = 0x0104_f2f0;
const RATIO_LIMIT: u32 = 0x0101_7b70;
const HEIGHT_FRACTION: u32 = 0x0106_b9e8;
const QUARTER: u32 = 0x0102_90b0;
const THIRTY_TWO: u32 = 0x0102_f070;
const LARGE_SIZE_BONUS: u32 = 0x0102_fc70;
const PUSH_SCALE: u32 = 0x0109_c9e0;
/// `float` `0x7f7fffee` the collector reset stores as its distances.
const COLLECTOR_FAR: u32 = 0x0104_da48;
/// `float` default radius when the capsule has no Havok shape (0.1).
const DEFAULT_RADIUS: u32 = 0x010c_72bc;
/// `float` impulse factors of the object callback: 2.0, 0.1 and 3.0.
const IMPULSE_DEFAULT: u32 = 0x0101_62c0;
const IMPULSE_LIGHT: u32 = 0x0101_e2bc;
/// The mass limit of the object callback (`float`, 95.0 in the exe).
const MAX_PUSH_MASS: u32 = 0x011b_0128;
/// Mass a body must exceed for `00c6def0` (`float` global).
const BLOCKING_MASS: u32 = 0x0126_7bc0;
/// `float` global scaled by 32 in `CheckInsideOfObject`.
const INSIDE_MARGIN: u32 = 0x0126_7bcc;
/// `float` velocity-modifier time of the character callback (0.01) and its rotation
/// time (0.2).
const MODIFIER_TIME: u32 = 0x0101_3ea4;
const ROT_MOD_TIME: u32 = 0x0101_df44;
/// 16-byte vector whose first lane the character callback subtracts from (0.0).
const DOT_BASE: u32 = 0x010c_7160;
/// The vector `00c6d9f0` copies three times.
const POINT_DEFAULT: u32 = 0x011f_426c;
/// Second argument of the shape's slot 0xD4 in `BuildDisplayGeometry`.
const DISPLAY_ARGUMENT: u32 = 0x011a_9b7c;
/// The layer-filter callback pointer `00c6d7e0` calls (cdecl, one argument).
const LAYER_CALLBACK: u32 = 0x0126_7bc8;
/// Globals released by the two static tear-down functions.
const STATIC_REFERENCED_OBJECT: u32 = 0x0126_7bb8;
const STATIC_NI_POINTER: u32 = 0x0126_7be0;

/// `NiRTTI` records the `GetRTTI` overrides return.
const SHAPE_COLLECTION_RTTI: u32 = 0x0126_86d4;
const CACHING_PHANTOM_RTTI: u32 = 0x0126_85a4;
const COLLISION_BOX_RTTI: u32 = 0x0126_8444;
const LIST_SHAPE_RTTI: u32 = 0x0126_81a8;

/// Vtables (class names from their RTTI type descriptors).
const VTABLE_WORLD_OBJECT: u32 = 0x0103_0394;
const VTABLE_PHANTOM: u32 = 0x0103_078c;
const VTABLE_SHAPE_PHANTOM: u32 = 0x0103_06b4;
const VTABLE_SIMPLE_SHAPE_PHANTOM: u32 = 0x0103_05dc;
const VTABLE_CACHING_SHAPE_PHANTOM: u32 = 0x010c_4694;
const VTABLE_COLLISION_BOX: u32 = 0x010c_477c;
const VTABLE_SHAPE: u32 = 0x0103_0bf4;
const VTABLE_SHAPE_COLLECTION: u32 = 0x010c_44e4;
const VTABLE_LIST_SHAPE: u32 = 0x010c_485c;
const VTABLE_CLOSEST_RAY_HIT_COLLECTOR: u32 = 0x0103_4ec4;
const VTABLE_CD_POINT_COLLECTOR: u32 = 0x0104_da3c;

/// Counter words the inlined base constructors increment (one per class).
const COUNT_WORLD_OBJECT: u32 = 0x0126_810c;
const COUNT_PHANTOM: u32 = 0x0126_8238;
const COUNT_SHAPE_PHANTOM: u32 = 0x0126_8208;
const COUNT_SIMPLE_SHAPE_PHANTOM: u32 = 0x0126_822c;
const COUNT_CACHING_SHAPE_PHANTOM: u32 = 0x0126_85a0;
const COUNT_SHAPE: u32 = 0x0126_80f8;
const COUNT_SHAPE_COLLECTION: u32 = 0x0126_86d0;
const COUNT_LIST_SHAPE: u32 = 0x0126_81a0;

/// Property key `00c6d900` looks for.
const PROPERTY_KEY: u32 = 0x1300;

layout! {
    /// `bhkCharacterController` (Xbox PDB), 0x650 bytes; PC offsets as this unit uses
    /// them (they agree with the PDB).
    pub struct BhkCharacterController: 0x650 {
        /// `phkObject` (Xbox PDB): the `hkpCharacterProxy`.
        0x008 phkObject: u32,
        /// `pInfo` (Xbox PDB).
        0x00c pInfo: u32,
        /// `iFlags` (Xbox PDB) of the `bhkCharacterListener` base.
        0x414 iFlags: u32,
        /// `fStepHeight` (Xbox PDB).
        0x470 fStepHeight: f32,
        /// `ForwardVec` (Xbox PDB).
        0x4d0 ForwardVec: [f32; 4],
        /// `StepInfo.m_deltaTime` (Xbox PDB `hkStepInfo` at +0x4E0).
        0x4e8 StepDeltaTime: f32,
        /// `OutVelocity` (Xbox PDB).
        0x4f0 OutVelocity: [f32; 4],
        /// `VelocityMod` (Xbox PDB).
        0x500 VelocityMod: [f32; 4],
        /// `fVelocityTime` (Xbox PDB).
        0x524 fVelocityTime: f32,
        /// `fRotMod` (Xbox PDB).
        0x528 fRotMod: f32,
        /// `fRotModTime` (Xbox PDB).
        0x52c fRotModTime: f32,
        /// `fFallTime` (Xbox PDB).
        0x548 fFallTime: f32,
        /// `fPitchAngle` (Xbox PDB).
        0x550 fPitchAngle: f32,
        /// `fRollAngle` (Xbox PDB).
        0x554 fRollAngle: f32,
        /// `fActorHeight` (Xbox PDB).
        0x564 fActorHeight: f32,
        /// `RotCenter` (Xbox PDB).
        0x570 RotCenter: [f32; 4],
        /// `PushDelta` (Xbox PDB).
        0x580 PushDelta: [f32; 4],
        /// `iPushCount` (Xbox PDB).
        0x590 iPushCount: u32,
        /// `spShapePhantom` (Xbox PDB): `bhkShapePhantom*`.
        0x594 spShapePhantom: u32,
        /// `spCollisionBox` (Xbox PDB): `bhkCollisionBox*`.
        0x598 spCollisionBox: u32,
        /// `eShapeType` (Xbox PDB).
        0x59c eShapeType: i32,
        /// `spShapes` (Xbox PDB): `bhkShape*` per shape type.
        0x5a4 spShapes: u32,
        /// `FrontBack` (Xbox PDB): four `hkVector4`.
        0x5b0 FrontBack: [f32; 4],
        /// `fRadius` (Xbox PDB).
        0x5f0 fRadius: f32,
        /// `fHeight` (Xbox PDB).
        0x5f4 fHeight: f32,
        /// `eSize` (Xbox PDB).
        0x600 eSize: u32,
        /// `iPriority` (Xbox PDB).
        0x604 iPriority: u32,
        /// `spBumpedBody` (Xbox PDB): `hkRefPtr<hkpRigidBody>`.
        0x630 spBumpedBody: u32,
        /// `fBumpedForce` (Xbox PDB).
        0x634 fBumpedForce: f32,
        /// `spBumpedCharProxy` (Xbox PDB): `hkRefPtr<hkpCharacterProxy>`.
        0x638 spBumpedCharProxy: u32,
    }
}

type Ctl = BhkCharacterController;

/// Four floats at `a`, bit for bit.
fn load4(e: &Engine, a: u32) -> [f32; 4] {
    [0, 4, 8, 12].map(|o| e.mem.f32(a.wrapping_add(o)))
}

fn store4(e: &mut Engine, a: u32, v: [f32; 4]) {
    for (lane, value) in v.into_iter().enumerate() {
        e.mem.set_f32(a.wrapping_add(4 * lane as u32), value);
    }
}

/// `n` words from `src` to `dst` (`MOVAPS`/`MOV` copies).
fn copy_words(e: &mut Engine, dst: u32, src: u32, n: u32) {
    for i in 0..n {
        let w = e.mem.u32(src.wrapping_add(4 * i));
        e.mem.set_u32(dst.wrapping_add(4 * i), w);
    }
}

/// `x2*x2 + (x1*x1 + x0*x0)` in single precision, the `MULPS`/`SHUFPS`/`ADDSS` sum.
fn sum_of_squares3(v: [f32; 4]) -> f32 {
    v[2] * v[2] + (v[1] * v[1] + v[0] * v[0])
}

/// The `RSQRTSS` estimate refined once, with the `CMPEQSS`/`ANDNPS` zero guard:
/// `(half * r) * (three - (sum * r) * r)`, `+0.0` when `sum == zero`.
fn inverse_length(sum: f32, zero: f32, half: f32, three: f32) -> f32 {
    let estimate = rsqrt_estimate(sum);
    let step = three - (sum * estimate) * estimate;
    let value = (half * estimate) * step;
    if sum == zero {
        0.0
    } else {
        value
    }
}

/// Sets the allocation-scope word to 0x10 and returns its address and old value.
fn enter_scope(e: &mut Engine) -> (u32, u32) {
    let word = e.tls() + SCOPE_WORD;
    let saved = e.mem.u32(word);
    e.mem.set_u32(word, 0x10);
    (word, saved)
}

/// Frees `bytes` at `block` through the current Havok allocator (slot 8).
fn havok_free(e: &mut Engine, block: u32, bytes: u32) {
    let router = e.call(MEMORY_ROUTER, &args![]).u32();
    let allocator = e.mem.u32(router + 0x10);
    e.vcall(allocator, 8, &args![block, bytes]);
}

/// `NiPointer` release: `InterlockedDecrement(&object->m_uiRefCount)`, then
/// `DeleteThis` (slot 4) when it reached 0.
fn release_ni_object(e: &mut Engine, object: u32) {
    let count = e
        .call(INTERLOCKED_DECREMENT, &args![object.wrapping_add(4)])
        .i32();
    if count == 0 {
        e.vcall(object, 4, &args![]);
    }
}

/// Releases a fixed-string handle unless it is the empty one.
fn release_fixed_string(e: &mut Engine, handle: u32) {
    if handle != e.global::<u32>(EMPTY_FIXED_STRING) {
        e.call(INTERLOCKED_DECREMENT, &args![handle.wrapping_sub(8)]);
    }
}

fn increment_count(e: &mut Engine, counter: u32) {
    let value: u32 = e.global(counter);
    e.set_global(counter, value.wrapping_add(1));
}

/// The listener `this` of a callback back to its controller.
fn controller_of(listener: Ptr) -> Ptr<Ctl> {
    Ptr::new(listener.addr().wrapping_sub(LISTENER))
}

// Translated from 00c6cbc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets `fRotMod` to `rot_mod` and `fRotModTime` to `time` when `|rot_mod|` is larger than
/// the current `|fRotMod|` (ordered compare: a NaN never replaces it).
pub fn fn_00c6cbc0(e: &mut Engine, this: Ptr<Ctl>, rot_mod: f32, time: f32) {
    let rot_mod = x87_quiet(rot_mod);
    let current = x87_quiet(e.get(this, Ctl::fRotMod)).abs();
    if current < rot_mod.abs() {
        e.set(this, Ctl::fRotMod, rot_mod);
        e.set(this, Ctl::fRotModTime, x87_quiet(time));
    }
}

// Translated from 00c6cc10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCharacterController::ApplyRotationModifier` (Xbox PDB): when `fRotModTime > 0`,
/// returns `value + fRotMod * min(fRotModTime, time)` (the time when `fRotModTime >
/// time`), lowers `fRotModTime` by `time` and `fRotMod` by 0.05; otherwise clears
/// `fRotMod` and returns `value`.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // the x87 compare treats NaN as "not above 0"
pub fn bhk_character_controller_apply_rotation_modifier(
    e: &mut Engine,
    this: Ptr<Ctl>,
    value: f32,
    time: f32,
) -> f32 {
    let value = x87_quiet(value);
    let remaining = e.get(this, Ctl::fRotModTime);
    if !(0.0 < remaining) {
        e.set(this, Ctl::fRotMod, 0.0);
        return value;
    }
    let remaining = x87_quiet(remaining);
    let time = x87_quiet(time);
    let used = if remaining > time { time } else { remaining };
    let rot_mod = e.get(this, Ctl::fRotMod);
    let result = (value as f64 + rot_mod as f64 * used as f64) as f32;
    let left = (e.get(this, Ctl::fRotModTime) as f64 - time as f64) as f32;
    e.set(this, Ctl::fRotModTime, left);
    let decay: f64 = e.global(ROT_MOD_DECAY);
    let rot_mod = (e.get(this, Ctl::fRotMod) as f64 - decay) as f32;
    e.set(this, Ctl::fRotMod, rot_mod);
    result
}

// Translated from 00c6cca0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkRefPtr` release: removes a reference from the object `*this` points to (when
/// there is one) and clears the pointer.
pub fn fn_00c6cca0(e: &mut Engine, this: Ptr) {
    let object = e.mem.u32(this.addr());
    if object != 0 {
        e.call(REMOVE_REFERENCE, &args![object]);
    }
    e.mem.set_u32(this.addr(), 0);
}

// Translated from 00c6ccc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// cdecl `(out, a, b)`: `out = a` with its first lane `a.x - b.x` (`SUBSS`).
pub fn fn_00c6ccc0(e: &mut Engine, out: Ptr, a: Ptr, b: Ptr) {
    let mut v = load4(e, a.addr());
    v[0] -= e.mem.f32(b.addr());
    store4(e, out.addr(), v);
}

// Translated from 00c6cce0 (decompiled, FalloutNV.exe 1.4.0.525)
/// cdecl `(out, a, b)`: `out = a` with its first lane `a.x + b.x` (`ADDSS`).
pub fn fn_00c6cce0(e: &mut Engine, out: Ptr, a: Ptr, b: Ptr) {
    let mut v = load4(e, a.addr());
    v[0] += e.mem.f32(b.addr());
    store4(e, out.addr(), v);
}

// Translated from 00c6cd00 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the 0x30 bytes at `source` to `this` (three `MOVAPS`).
pub fn fn_00c6cd00(e: &mut Engine, this: Ptr, source: Ptr) {
    copy_words(e, this.addr(), source.addr(), 12);
}

// Translated from 00c6cd30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the 0x30 bytes at `source` to `this`; returns `this`.
pub fn fn_00c6cd30(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    copy_words(e, this.addr(), source.addr(), 12);
    this
}

// Translated from 00c6cd60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the 0x40 bytes at `source` to `this`; returns `this`.
pub fn fn_00c6cd60(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    copy_words(e, this.addr(), source.addr(), 16);
    this
}

// Translated from 00c6cd90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkShapeCollection::GetRTTI` (Xbox PDB).
pub fn bhk_shape_collection_get_rtti(_e: &mut Engine, _this: Ptr) -> u32 {
    SHAPE_COLLECTION_RTTI
}

// Translated from 00c6cda0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkShapeCollection::_scalar_deleting_destructor_` (Xbox PDB): destroys, and frees the
/// 0x14 bytes when bit 0 of `flags` is set.
pub fn bhk_shape_collection_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u8,
) -> Ptr {
    e.call(SHAPE_COLLECTION_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(FREE_SIZED, &args![this, 0x14u32]);
    }
    this
}

// Translated from 00c6cdc0 (decompiled, FalloutNV.exe 1.4.0.525)
/// cdecl: frees a 0x80-byte block (`bhkCharacterProxy`'s cinfo size) through the Havok
/// allocator when `block` is not null.
pub fn fn_00c6cdc0(e: &mut Engine, block: u32) {
    if block != 0 {
        havok_free(e, block, 0x80);
    }
}

// Translated from 00c6cee0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCharacterProxy::QCinfoSize` (Xbox PDB): 0x80.
pub fn bhk_character_proxy_q_cinfo_size(_e: &mut Engine, _this: Ptr) -> u32 {
    0x80
}

// Translated from 00c6cf30 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6cf30 - the FS:[0] exception-unwinding frame and the /GS cookie check are not modeled.
/// `bhkCharacterProxy::Integrate` (Xbox PDB): between two `UnMarkWrite` calls, when the
/// proxy and its Havok proxy exist, builds a `bhkCharacterPointCollector` on the stack
/// (constructed with `this + 0x10`), runs `integrateWithCollectors(step_info, world + 0x10,
/// collector, this + 0x10)` with the world from vtable slot 0x94, handles the
/// collisions of `this + 0x10` and destroys the stack collector.
pub fn bhk_character_proxy_integrate(e: &mut Engine, this: Ptr, step_info: u32) {
    e.call(UN_MARK_WRITE, &args![this]);
    if this.addr() != 0 {
        let proxy = e.mem.u32(this.addr() + 8);
        if proxy != 0 {
            let owner = this.addr() + 0x10;
            e.with_stack(0x3dc, |e, collector| {
                e.call(POINT_COLLECTOR_CONSTRUCT, &args![collector, owner]);
                let world = e.vcall(this.addr(), 0x94, &args![]).u32();
                e.call(
                    INTEGRATE_WITH_COLLECTORS,
                    &args![proxy, step_info, world.wrapping_add(0x10), collector, owner],
                );
                e.call(POINT_COLLECTOR_HANDLE_COLLISIONS, &args![owner]);
                e.call(POINT_COLLECTOR_DESTRUCT, &args![collector]);
            });
        }
    }
    e.call(UN_MARK_WRITE, &args![this]);
}

// Translated from 00c6d010 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCharacterProxy::_scalar_deleting_destructor_` (Xbox PDB): destroys (`00cabf50`) and
/// frees the 0x3E0 bytes when bit 0 of `flags` is set.
pub fn bhk_character_proxy_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u8) -> Ptr {
    e.call(CHARACTER_PROXY_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(FREE_SIZED, &args![this, 0x3e0u32]);
    }
    this
}

// Translated from 00c6d040 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6d040 - the FS:[0] exception-unwinding frame is not modeled.
/// `bhkCachingShapePhantom::bhkCachingShapePhantom` (Xbox PDB): the inlined
/// `bhkWorldObject`, `bhkPhantom` and `bhkShapePhantom` constructors (vtables, cleared
/// `+0xC`/`+0x10`/`+0x14`, one count each), then the class vtable, `Init(info)` and its
/// own count. Returns `this`.
pub fn bhk_caching_shape_phantom_construct(e: &mut Engine, this: Ptr, info: u32) -> Ptr {
    let me = this.addr();
    construct_shape_phantom_bases(e, me);
    e.mem.set_u32(me, VTABLE_CACHING_SHAPE_PHANTOM);
    e.call(CACHING_PHANTOM_INIT, &args![this, info]);
    increment_count(e, COUNT_CACHING_SHAPE_PHANTOM);
    e.mem.set_u8(me + 0x14, 0);
    this
}

// Translated from 00c6d0e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCachingShapePhantom::GetRTTI` (Xbox PDB).
pub fn bhk_caching_shape_phantom_get_rtti(_e: &mut Engine, _this: Ptr) -> u32 {
    CACHING_PHANTOM_RTTI
}

// Translated from 00c6d0f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Returns 0x60 (a size query; the slot is not named in the map).
pub fn fn_00c6d0f0(_e: &mut Engine, _this: Ptr) -> u32 {
    0x60
}

// Translated from 00c6d100 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCachingShapePhantom::_scalar_deleting_destructor_` (Xbox PDB): destroys and frees
/// the 0x18 bytes when bit 0 of `flags` is set.
pub fn bhk_caching_shape_phantom_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u8,
) -> Ptr {
    e.call(CACHING_PHANTOM_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(FREE_SIZED, &args![this, 0x18u32]);
    }
    this
}

// Translated from 00c6d120 (decompiled, FalloutNV.exe 1.4.0.525)
/// True when the `hkpCharacterContext` (+0x3E0) state is 4, 5 or 6.
pub fn fn_00c6d120(e: &mut Engine, this: Ptr<Ctl>) -> bool {
    let state = e.call(GET_STATE, &args![this.addr() + 0x3e0]).u32();
    matches!(state, 4..=6)
}

// Translated from 00c6d150 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears `PushDelta` and `iPushCount` when `iPushCount` is not 0.
pub fn fn_00c6d150(e: &mut Engine, this: Ptr<Ctl>) {
    if e.get(this, Ctl::iPushCount) != 0 {
        copy_zero(e, this.addr() + Ctl::PushDelta.off, 4);
        e.set(this, Ctl::iPushCount, 0);
    }
}

fn copy_zero(e: &mut Engine, dst: u32, words: u32) {
    for i in 0..words {
        e.mem.set_u32(dst + 4 * i, 0);
    }
}

// Translated from 00c6d170 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpClosestCdPointCollector::reset` (Xbox PDB): clears `+0x30` and sets both distances
/// (`+0x2C`, `+4`) to the exe's far value.
pub fn hkp_closest_cd_point_collector_reset(e: &mut Engine, this: Ptr) {
    let far = x87_quiet_bits(e.global::<u32>(COLLECTOR_FAR));
    e.mem.set_u32(this.addr() + 0x30, 0);
    e.mem.set_u32(this.addr() + 0x2c, far);
    e.mem.set_u32(this.addr() + 4, far);
}

/// The inlined `bhkWorldObject`/`bhkPhantom`/`bhkShapePhantom` constructor chain.
fn construct_shape_phantom_bases(e: &mut Engine, me: u32) {
    e.call(REF_OBJECT_CONSTRUCT, &args![me]);
    e.mem.set_u32(me, VTABLE_WORLD_OBJECT);
    e.mem.set_u32(me + 0xc, 0);
    e.mem.set_u32(me + 0x10, 0);
    increment_count(e, COUNT_WORLD_OBJECT);
    e.mem.set_u32(me, VTABLE_PHANTOM);
    increment_count(e, COUNT_PHANTOM);
    e.mem.set_u32(me, VTABLE_SHAPE_PHANTOM);
    increment_count(e, COUNT_SHAPE_PHANTOM);
    e.mem.set_u8(me + 0x14, 0);
}

// Translated from 00c6d190 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkSimpleShapePhantom` constructor (RTTI of vtable `010305dc`): the base chain, then
/// its vtable and count, `+0x14` cleared. Returns `this`.
pub fn fn_00c6d190(e: &mut Engine, this: Ptr) -> Ptr {
    construct_shape_phantom_bases(e, this.addr());
    e.mem.set_u32(this.addr(), VTABLE_SIMPLE_SHAPE_PHANTOM);
    increment_count(e, COUNT_SIMPLE_SHAPE_PHANTOM);
    e.mem.set_u8(this.addr() + 0x14, 0);
    this
}

// Translated from 00c6d1e0 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6d1e0 - the FS:[0] exception-unwinding frame is not modeled.
/// `bhkCollisionBox` constructor (RTTI of vtable `010c477c`): the
/// `bhkSimpleShapePhantom` constructor, the class vtable, then `bhkCollisionBox::Init`
/// with both arguments. Returns `this`.
pub fn fn_00c6d1e0(e: &mut Engine, this: Ptr, first: u32, second: u32) -> Ptr {
    fn_00c6d190(e, this);
    e.mem.set_u32(this.addr(), VTABLE_COLLISION_BOX);
    e.call(COLLISION_BOX_INIT, &args![this, first, second]);
    this
}

// Translated from 00c6d250 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCollisionBox::GetRTTI` (Xbox PDB).
pub fn bhk_collision_box_get_rtti(_e: &mut Engine, _this: Ptr) -> u32 {
    COLLISION_BOX_RTTI
}

// Translated from 00c6d260 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCollisionBox::_scalar_deleting_destructor_` (Xbox PDB): destroys, and when bit 0 of
/// `flags` is set and `this` is not null frees the aligned block (`this` minus the
/// offset byte at `this - 1`) through the `MemoryManager`.
pub fn bhk_collision_box_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u8) -> Ptr {
    e.call(COLLISION_BOX_DESTRUCT, &args![this]);
    if flags & 1 != 0 && this.addr() != 0 {
        let offset = e.mem.u8(this.addr().wrapping_sub(1));
        let block = this.addr().wrapping_sub(u32::from(offset));
        e.call(MEMORY_DEALLOCATE, &args![MEMORY_MANAGER, block]);
    }
    this
}

// Translated from 00c6d290 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6d290 - the FS:[0] exception-unwinding frame is not modeled.
/// `bhkListShape` constructor (RTTI of vtable `010c485c`): the inlined `bhkShape` and
/// `bhkShapeCollection` constructors (vtables, cleared `+0xC`/`+0x10`, counts), the class
/// vtable, `bhkListShape::Init(info)` and its count. Returns `this`.
pub fn fn_00c6d290(e: &mut Engine, this: Ptr, info: u32) -> Ptr {
    let me = this.addr();
    e.call(REF_OBJECT_CONSTRUCT, &args![this]);
    e.mem.set_u32(me, VTABLE_SHAPE);
    e.mem.set_u32(me + 0xc, 0);
    e.mem.set_u32(me + 0x10, 0);
    increment_count(e, COUNT_SHAPE);
    e.mem.set_u32(me, VTABLE_SHAPE_COLLECTION);
    increment_count(e, COUNT_SHAPE_COLLECTION);
    e.mem.set_u32(me, VTABLE_LIST_SHAPE);
    e.call(LIST_SHAPE_INIT, &args![this, info]);
    increment_count(e, COUNT_LIST_SHAPE);
    this
}

// Translated from 00c6d310 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkListShape::GetRTTI` (Xbox PDB).
pub fn bhk_list_shape_get_rtti(_e: &mut Engine, _this: Ptr) -> u32 {
    LIST_SHAPE_RTTI
}

// Translated from 00c6d320 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkListShape::_scalar_deleting_destructor_` (Xbox PDB): destroys and frees the 0x14
/// bytes when bit 0 of `flags` is set.
pub fn bhk_list_shape_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u8) -> Ptr {
    e.call(LIST_SHAPE_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(FREE_SIZED, &args![this, 0x14u32]);
    }
    this
}

// Translated from 00c6d340 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6d340 - the FS:[0] exception-unwinding frame is not modeled.
/// Static tear-down: with the allocation-scope word at 0x10, removes a reference from
/// the object at `01267bb8` when there is one (the pointer is left as it is).
pub fn fn_00c6d340(e: &mut Engine) {
    let (word, saved) = enter_scope(e);
    let object: u32 = e.global(STATIC_REFERENCED_OBJECT);
    if object != 0 {
        e.call(REMOVE_REFERENCE, &args![object]);
    }
    e.mem.set_u32(word, saved);
}

// Translated from 00c6d3c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Static tear-down of the `NiPointer` at `01267be0`: releases the object and clears it.
pub fn fn_00c6d3c0(e: &mut Engine) {
    let object: u32 = e.global(STATIC_NI_POINTER);
    if object != 0 {
        release_ni_object(e, object);
        e.set_global(STATIC_NI_POINTER, 0u32);
    }
}

// Translated from 00c6d3f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCharacterController::SetShape` (Xbox PDB): gives the Havok proxy's shape phantom
/// the `hkpShape` of `shape` (its `+8`, or none). Does nothing (returns false) without a
/// proxy or phantom, or when the phantom already has that shape. Otherwise: the
/// `UnMarkRead`/`getPosition`/`UnMarkRead` sequence (result unused), the phantom leaves
/// its world (when it has one), gets the shape (vtable slot 0xC) and is added back to the
/// same world; returns true. The second stack word is never read.
pub fn bhk_character_controller_set_shape(
    e: &mut Engine,
    this: Ptr<Ctl>,
    shape: u32,
    _unused_2: u32,
) -> bool {
    let proxy = e.get(this, Ctl::phkObject);
    if proxy == 0 {
        return false;
    }
    let phantom = e.call(GET_SHAPE_PHANTOM, &args![proxy]).u32();
    if phantom == 0 {
        return false;
    }
    let hk_shape = if shape != 0 { e.mem.u32(shape + 8) } else { 0 };
    if e.mem.u32(phantom + 0x10) == hk_shape {
        return false;
    }
    let proxy = e.get(this, Ctl::phkObject);
    let world = e.mem.u32(phantom + 8);
    if proxy != 0 {
        e.call(UN_MARK_READ, &args![this]);
        e.call(HKP_GET_POSITION, &args![proxy]);
        e.call(UN_MARK_READ, &args![this]);
    }
    if world != 0 {
        e.call(WORLD_REMOVE_PHANTOM, &args![world, phantom]);
    }
    let hk_shape = if shape != 0 { e.mem.u32(shape + 8) } else { 0 };
    e.vcall(phantom, 0xc, &args![hk_shape]);
    if world != 0 {
        e.call(WORLD_ADD_PHANTOM, &args![world, phantom]);
    }
    true
}

// Translated from 00c6d4a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `bhkCollisionBox::Update` on `spCollisionBox` when there is one (tail call).
pub fn fn_00c6d4a0(e: &mut Engine, this: Ptr<Ctl>) {
    let collision_box = e.get(this, Ctl::spCollisionBox);
    if collision_box != 0 {
        e.call(COLLISION_BOX_UPDATE, &args![collision_box]);
    }
}

// Translated from 00c6d5c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCharacterController::ApplyVelocityMod` (Xbox PDB): when `fVelocityTime > 0` and
/// neither flag 0x800 nor 0x1000 is set, `OutVelocity += fVelocityTime * VelocityMod`
/// (single precision, all four lanes); when `fVelocityTime` is not above 0 (or NaN),
/// `VelocityMod` becomes the zero vector at `01267e30`.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // the x87 compare treats NaN as "not above 0"
pub fn bhk_character_controller_apply_velocity_mod(e: &mut Engine, this: Ptr<Ctl>) {
    let time = e.get(this, Ctl::fVelocityTime);
    if !(0.0 < time) {
        copy_words(e, this.addr() + Ctl::VelocityMod.off, ZERO_VECTOR, 4);
        return;
    }
    if e.get(this, Ctl::iFlags) & 0x1800 != 0 {
        return;
    }
    let modifier = load4(e, this.addr() + Ctl::VelocityMod.off);
    let velocity = load4(e, this.addr() + Ctl::OutVelocity.off);
    let out: [f32; 4] = std::array::from_fn(|lane| time * modifier[lane] + velocity[lane]);
    store4(e, this.addr() + Ctl::OutVelocity.off, out);
}

// Translated from 00c6d620 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6d620 - the /GS cookie check is not modeled.
/// cdecl `(source, transform, out)`: `out = transform * (source[+0x20], source[+0x24],
/// source[+0x10] + source[+0x28], source[+0x2C])` through `hkVector4::setTransformedPos`.
pub fn fn_00c6d620(e: &mut Engine, source: Ptr, transform: u32, out: u32) {
    let s = source.addr();
    e.with_stack(0x10, |e, point| {
        copy_words(e, point.addr(), s + 0x20, 4);
        let z = (e.mem.f32(s + 0x10) as f64 + e.mem.f32(s + 0x28) as f64) as f32;
        e.mem.set_f32(point.addr() + 8, z);
        e.call(SET_TRANSFORMED_POS, &args![out, transform, point]);
    });
}

// Translated from 00c6d670 (decompiled, FalloutNV.exe 1.4.0.525)
/// Copies the 0x34 bytes at `source` to `this` (the floats at `+0x20..+0x2C` through x87
/// loads and stores); returns `this`.
pub fn fn_00c6d670(e: &mut Engine, this: Ptr, source: Ptr) -> Ptr {
    let (t, s) = (this.addr(), source.addr());
    copy_words(e, t, s, 8);
    for o in [0x20, 0x24, 0x28, 0x2c] {
        let w = x87_quiet_bits(e.mem.u32(s + o));
        e.mem.set_u32(t + o, w);
    }
    let w = e.mem.u32(s + 0x30);
    e.mem.set_u32(t + 0x30, w);
    this
}

// Translated from 00c6d770 (decompiled, FalloutNV.exe 1.4.0.525)
/// Passes `value` to `00c85ce0` on the shape phantom and on the collision box, each when
/// present.
pub fn fn_00c6d770(e: &mut Engine, this: Ptr<Ctl>, value: u32) {
    let phantom = e.get(this, Ctl::spShapePhantom);
    if phantom != 0 {
        e.call(WORLD_OBJECT_PASS, &args![phantom, value]);
    }
    let collision_box = e.get(this, Ctl::spCollisionBox);
    if collision_box != 0 {
        e.call(WORLD_OBJECT_PASS, &args![collision_box, value]);
    }
}

// Translated from 00c6d7a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCharacterController::SetNiObject` (Xbox PDB): `SetAVObject(object, 0)` on the
/// shape phantom and on the collision box, each when present.
pub fn bhk_character_controller_set_ni_object(e: &mut Engine, this: Ptr<Ctl>, object: u32) {
    let phantom = e.get(this, Ctl::spShapePhantom);
    if phantom != 0 {
        e.call(SET_AV_OBJECT, &args![phantom, object, 0u32]);
    }
    let collision_box = e.get(this, Ctl::spCollisionBox);
    if collision_box != 0 {
        e.call(SET_AV_OBJECT, &args![collision_box, object, 0u32]);
    }
}

// Translated from 00c6d7e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// cdecl layer filter. With `check` 0 returns 1 when `fallback` is not 0, else 0. With
/// `check` set and a `shape` whose `+8` is not null: the layer is `00c84f10(layer)` on
/// that Havok object, or its `+0x10` low five bits when `layer` is -1; the result is 1
/// when the callback at `01267bc8` is null or returns false for it, else 0. Without the
/// shape it returns `check` unchanged (its byte).
pub fn fn_00c6d7e0(e: &mut Engine, shape: u32, check: u8, fallback: u8, layer: i32) -> u8 {
    if check == 0 {
        return u8::from(fallback != 0);
    }
    if shape == 0 {
        return check;
    }
    let hk = e.mem.u32(shape + 8);
    if hk == 0 {
        return check;
    }
    let number = if layer == -1 {
        e.mem.u32(hk + 0x10) & 0x1f
    } else {
        e.call(SHAPE_LAYER, &args![hk, layer]).u32()
    };
    let callback: u32 = e.global(LAYER_CALLBACK);
    let blocked = if callback != 0 {
        e.call(callback, &args![number]).u8()
    } else {
        0
    };
    u8::from(blocked == 0)
}

// Translated from 00c6d840 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorldObject::getProperty` (Xbox PDB): writes the 8-byte value of the first property
/// with `key` (array `m_properties` at `+0x78`, size `+0x7C`, 16-byte entries: key, pad,
/// value) to `out`, or zeros. Returns `out`. The third stack word is never read.
pub fn hkp_world_object_get_property(
    e: &mut Engine,
    this: Ptr,
    out: Ptr,
    key: u32,
    _unused_3: u32,
) -> Ptr {
    let count = e.mem.i32(this.addr() + 0x7c);
    if count > 0 {
        let data = e.mem.u32(this.addr() + 0x78);
        let mut index = 0;
        while index < count {
            let entry = data.wrapping_add((index as u32) << 4);
            if e.mem.u32(entry) == key {
                let low = e.mem.u32(entry + 8);
                let high = e.mem.u32(entry + 0xc);
                e.mem.set_u32(out.addr() + 4, high);
                e.mem.set_u32(out.addr(), low);
                return out;
            }
            index += 1;
        }
    }
    e.mem.set_u32(out.addr(), 0);
    e.mem.set_u32(out.addr() + 4, 0);
    out
}

/// Whether the Havok world object `hk` has a property with `key`.
fn has_property(e: &Engine, hk: u32, key: u32) -> bool {
    let count = e.mem.i32(hk + 0x7c);
    let data = e.mem.u32(hk + 0x78);
    let mut index = 0;
    while index < count {
        if e.mem.u32(data.wrapping_add((index as u32) << 4)) == key {
            return true;
        }
        index += 1;
    }
    false
}

// Translated from 00c6d8a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the world object's Havok object (`+8`) has a property with `key` (false
/// without one); the empty lock call brackets the search.
pub fn fn_00c6d8a0(e: &mut Engine, this: Ptr, key: u32) -> bool {
    let hk = e.mem.u32(this.addr() + 8);
    if hk == 0 {
        return false;
    }
    e.call(EMPTY_LOCK, &args![this]);
    let found = has_property(e, hk, key);
    e.call(EMPTY_LOCK, &args![this]);
    found
}

// Translated from 00c6d900 (decompiled, FalloutNV.exe 1.4.0.525)
/// stdcall `(object)`: when the `hkpWorldObject` has property 0x1300, the word at
/// `+0xC0` of the object its value points to; else 0.
pub fn fn_00c6d900(e: &mut Engine, object: u32) -> u32 {
    if object == 0 || !has_property(e, object, PROPERTY_KEY) {
        return 0;
    }
    e.with_stack(8, |e, out| {
        hkp_world_object_get_property(e, Ptr::new(object), out, PROPERTY_KEY, 0);
        let target = e.mem.u32(out.addr());
        e.mem.u32(target.wrapping_add(0xc0))
    })
}

// Translated from 00c6d960 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCharacterController::GetShape` (Xbox PDB): `spShapes[eShapeType]` when the type is
/// below 2 (signed), else null. The `NiPointer` release the compiler left behind runs on
/// a null pointer only and is left out.
pub fn bhk_character_controller_get_shape(e: &mut Engine, this: Ptr<Ctl>) -> u32 {
    let kind = e.get(this, Ctl::eShapeType);
    if kind < 2 {
        let slot = (this.addr() + Ctl::spShapes.off).wrapping_add((kind as u32).wrapping_mul(4));
        e.mem.u32(slot)
    } else {
        0
    }
}

// Translated from 00c6d9c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpClosestCdPointCollector::_scalar_deleting_destructor_` (Xbox PDB): sets the
/// `hkpCdPointCollector` vtable and, when bit 0 of `flags` is set, frees the 0x80 bytes
/// through the Havok allocator.
pub fn hkp_closest_cd_point_collector_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u8,
) -> Ptr {
    e.mem.set_u32(this.addr(), VTABLE_CD_POINT_COLLECTOR);
    if flags & 1 != 0 {
        havok_free(e, this.addr(), 0x80);
    }
    this
}

// Translated from 00c6d9f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clears the first word and fills the next nine with three copies of the point at
/// `011f426c`; returns `this`.
pub fn fn_00c6d9f0(e: &mut Engine, this: Ptr) -> Ptr {
    let me = this.addr();
    e.mem.set_u32(me, 0);
    for copy in 0..3 {
        copy_words(e, me + 4 + 12 * copy, POINT_DEFAULT, 3);
    }
    this
}

// Translated from 00c6de10 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6de10 - the FS:[0] exception-unwinding frame is not modeled.
/// Member tear-down: releases the `hkRefPtr` at `+0xA4` twice over (the second sees the
/// cleared pointer), then the `NiPointer` at `+0xA0` (not cleared).
pub fn fn_00c6de10(e: &mut Engine, this: Ptr) {
    let me = this.addr();
    for _ in 0..2 {
        let object = e.mem.u32(me + 0xa4);
        if object != 0 {
            e.call(REMOVE_REFERENCE, &args![object]);
        }
        e.mem.set_u32(me + 0xa4, 0);
    }
    let object = e.mem.u32(me + 0xa0);
    if object != 0 {
        release_ni_object(e, object);
    }
}

// Translated from 00c6dec0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCharacterController::FindBSReference` (Xbox PDB), cdecl `(key, object)`: the low
/// word of the `hkpWorldObject`'s property `key`, 0 without an object.
pub fn bhk_character_controller_find_bs_reference(e: &mut Engine, key: u32, object: u32) -> u32 {
    if object == 0 {
        return 0;
    }
    e.with_stack(8, |e, out| {
        hkp_world_object_get_property(e, Ptr::new(object), out, key, 0);
        e.mem.u32(out.addr())
    })
}

// Translated from 00c6def0 (decompiled, FalloutNV.exe 1.4.0.525)
/// cdecl `(body)`: true when the body's motion-type byte (`+0xE8`) passes `00c8cce0` and its
/// mass (`hkpMotion::getMass` on `+0xE0`) is above the `float` at `01267bc0`, or when the
/// low seven bits of `+0x2C` are 6; false for a null body.
pub fn fn_00c6def0(e: &mut Engine, body: u32) -> bool {
    if body == 0 {
        return false;
    }
    let motion_type = u32::from(e.mem.u8(body + 0xe8));
    let passes = e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), motion_type);
        e.call(MOTION_TYPE_TEST, &args![slot]).bool()
    });
    if passes {
        let mass = x87_quiet(e.call(GET_MASS, &args![body + 0xe0]).f32());
        let limit: f32 = e.global(BLOCKING_MASS);
        if limit < mass {
            return true;
        }
    }
    e.mem.u32(body + 0x2c) & 0x7f == 6
}

// Translated from 00c6df60 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6df60 - the FS:[0] exception-unwinding frame is not modeled.
/// Returns the cinfo at `+0xC`, creating it when there is none: `*created` says whether
/// it was missing; a new one is 0xC0 bytes from the Havok allocator, constructed by
/// `00c6da50(1)` (null when the allocation failed), stored, and handed to `00cabfd0` when
/// the Havok object (`+8`) exists. The allocation-scope word is 0x10 meanwhile.
pub fn fn_00c6df60(e: &mut Engine, this: Ptr, created: Ptr) -> u32 {
    let me = this.addr();
    let (word, saved) = enter_scope(e);
    let missing = e.mem.u32(me + 0xc) == 0;
    e.mem.set_u8(created.addr(), u8::from(missing));
    if missing {
        let router = e.call(MEMORY_ROUTER, &args![]).u32();
        let allocator = e.mem.u32(router + 0x10);
        let block = e.vcall(allocator, 4, &args![0xc0u32]).u32();
        let info = if block != 0 {
            e.call(CINFO_CONSTRUCT, &args![block, 1u32]).u32()
        } else {
            0
        };
        e.mem.set_u32(me + 0xc, info);
        if e.mem.u32(me + 8) != 0 {
            e.call(SET_PROXY_CINFO, &args![this, info]);
        }
    }
    let info = e.mem.u32(me + 0xc);
    e.mem.set_u32(word, saved);
    info
}

// Translated from 00c6e050 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds property `key` = `value` (sign-extended to 64 bits) to the shape phantom's Havok
/// object when the phantom exists, does not have that property yet, and has its Havok
/// object; the empty lock call brackets it.
pub fn fn_00c6e050(e: &mut Engine, this: Ptr<Ctl>, key: u32, value: i32) {
    let phantom = e.get(this, Ctl::spShapePhantom);
    if phantom == 0 || fn_00c6d8a0(e, Ptr::new(phantom), key) {
        return;
    }
    let hk = e.mem.u32(phantom + 8);
    if hk != 0 {
        e.call(EMPTY_LOCK, &args![phantom]);
        let high = (value >> 31) as u32;
        e.call(ADD_PROPERTY, &args![hk, key, value as u32, high]);
        e.call(EMPTY_LOCK, &args![phantom]);
    }
}

// Translated from 00c6e0b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkCharacterController::FindDisplayGeometry` (Xbox PDB): with flag 0x8000 set, looks
/// for the child named `"bhkColDisp"` of the first child (an `NiNode`) of the phantom's
/// scene object (`GetAVObject(0)`, then its slot 0xC); returns it, or clears flag 0x8000
/// and returns null when anything along the way is missing. Without the flag returns
/// null.
pub fn bhk_character_controller_find_display_geometry(e: &mut Engine, this: Ptr<Ctl>) -> u32 {
    if e.get(this, Ctl::iFlags) & 0x8000 == 0 {
        return 0;
    }
    let found = find_display_child(e, this);
    if found == 0 {
        let flags = e.get(this, Ctl::iFlags);
        e.set(this, Ctl::iFlags, flags & 0xffff_7fff);
    }
    found
}

fn find_display_child(e: &mut Engine, this: Ptr<Ctl>) -> u32 {
    let phantom = e.get(this, Ctl::spShapePhantom);
    if phantom == 0 {
        return 0;
    }
    let object = e.call(GET_AV_OBJECT, &args![phantom, 0u32]).u32();
    if object == 0 {
        return 0;
    }
    let root = e.vcall(object, 0xc, &args![]).u32();
    if root == 0 || e.mem.u16(root + 0xa6) == 0 {
        return 0;
    }
    let first = e.mem.u32(e.mem.u32(root + 0xa0));
    if first == 0 {
        return 0;
    }
    let node = e.call(IS_KIND_OF, &args![first, NODE_RTTI]).u32();
    if node == 0 {
        return 0;
    }
    let name = e.call(FIXED_STRING_CREATE, &args![DISPLAY_NAME]).u32();
    let count = u32::from(e.mem.u16(node + 0xa6));
    let mut found = 0;
    let mut index = 0;
    while index < count && found == 0 {
        let child = e.mem.u32(e.mem.u32(node + 0xa0).wrapping_add(index * 4));
        if child != 0 && e.mem.u32(child + 8) == name {
            found = child;
        }
        index += 1;
    }
    release_fixed_string(e, name);
    found
}

// Translated from 00c6e180 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6e180 - the /GS cookie check is not modeled.
/// `bhkCharacterController::SetRadius` (Xbox PDB): when `spShapes[0]` is a
/// `bhkCapsuleShape`, moves its Havok capsule's two vertices (`+0x20`, `+0x30`) apart along
/// z by the change of radius (`old - radius` is added to the first's z and subtracted
/// from the second's), copies the old radius into both `w` lanes and stores `radius`
/// (`+0x10`). The old radius is 0.1 (`010c72bc`) and the vertices the zero vector when the
/// capsule has no Havok shape.
pub fn bhk_character_controller_set_radius(e: &mut Engine, this: Ptr<Ctl>, radius: f32) {
    let shape = e.get(this, Ctl::spShapes);
    if shape == 0 {
        return;
    }
    let capsule = e.call(IS_KIND_OF, &args![shape, CAPSULE_SHAPE_RTTI]).u32();
    if capsule == 0 {
        return;
    }
    let hk = e.mem.u32(capsule + 8);
    let old = if hk != 0 {
        x87_quiet(e.mem.f32(hk + 0x10))
    } else {
        x87_quiet(e.global::<f32>(DEFAULT_RADIUS))
    };
    let radius = x87_quiet(radius);
    let change = (old as f64 - radius as f64) as f32;
    let first_source = if hk != 0 { hk + 0x20 } else { ZERO_VECTOR };
    let mut first = load4(e, first_source);
    let second_source = if hk != 0 { hk + 0x30 } else { ZERO_VECTOR };
    let mut second = load4(e, second_source);
    first[2] = (x87_quiet(first[2]) as f64 + change as f64) as f32;
    second[2] = (x87_quiet(second[2]) as f64 - change as f64) as f32;
    if hk != 0 {
        let w = x87_quiet(e.mem.f32(hk + 0x10));
        store4(e, hk + 0x20, first);
        e.mem.set_f32(hk + 0x2c, w);
    }
    let hk = e.mem.u32(capsule + 8);
    if hk != 0 {
        let w = x87_quiet(e.mem.f32(hk + 0x10));
        store4(e, hk + 0x30, second);
        e.mem.set_f32(hk + 0x3c, w);
    }
    let hk = e.mem.u32(capsule + 8);
    if hk != 0 {
        e.mem.set_f32(hk + 0x10, radius);
    }
}

// Translated from 00c6e2e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds the step's delta time (`+0x4E8`) to `fFallTime`.
pub fn fn_00c6e2e0(e: &mut Engine, this: Ptr<Ctl>) {
    let sum = (e.get(this, Ctl::StepDeltaTime) as f64 + e.get(this, Ctl::fFallTime) as f64) as f32;
    e.set(this, Ctl::fFallTime, sum);
}

// Translated from 00c6e420 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6e420 - the /GS cookie check is not modeled.
/// `bhkCharacterController::ApplyPitchAndRoll` (Xbox PDB): builds the phantom's new
/// transform from its current one (`+0xB0`, 0x40 bytes) with the rotation set from a
/// prepared copy of `rotation` (`005611c0`, `00cb2d90`); when `fPitchAngle` is not 0 (NaN
/// counts) and flag bit 25 is set, multiplies it (`hkMatrix3::mul`) by the rows
/// `(1,0,0) (0,c,-s) (0,s,c)`, and when `fRollAngle` is not 0 and bit 26 is set by
/// `(c,0,s) (0,1,0) (-s,0,c)`, with `c`/`s` the results of `00ec9f70`/`00eca0a0` on the
/// angle; then sets it on the Havok phantom (`setTransform`) when there is one.
pub fn bhk_character_controller_apply_pitch_and_roll(
    e: &mut Engine,
    this: Ptr<Ctl>,
    rotation: Ptr,
) {
    e.with_stack(0x90, |e, frame| {
        let at = |o: u32| frame.addr() + o;
        copy_words(e, at(0x10), rotation.addr(), 4);
        e.call(QUATERNION_PREPARE, &args![at(0x10)]);
        let phantom = e.get(this, Ctl::spShapePhantom);
        let hk = if phantom != 0 {
            e.mem.u32(phantom + 8)
        } else {
            0
        };
        copy_words(e, at(0x50), hk.wrapping_add(0xb0), 16);
        e.call(ROTATION_SET, &args![at(0x50), at(0x10)]);
        let zero: f32 = e.global(FLOAT_ZERO);
        let one: f32 = e.global(FLOAT_ONE);
        let negative_zero: f32 = e.global(FLOAT_NEGATIVE_ZERO);

        let pitch = e.get(this, Ctl::fPitchAngle);
        if pitch != 0.0 && (e.get(this, Ctl::iFlags) >> 0x19) & 1 != 0 {
            let angle = x87_quiet(pitch) as f64;
            let c = x87_quiet(x87_quiet(e.call(X87_FIRST_TRIG, &args![angle]).f32()));
            let s = x87_quiet(x87_quiet(e.call(X87_SECOND_TRIG, &args![angle]).f32()));
            store4(e, at(0x20), [one, zero, zero, zero]);
            store4(e, at(0x30), [zero, c, negative_zero - s, zero]);
            store4(e, at(0x40), [zero, s, c, zero]);
            e.call(MATRIX3_MUL, &args![at(0x50), at(0x20)]);
        }
        let roll = e.get(this, Ctl::fRollAngle);
        if roll != 0.0 && (e.get(this, Ctl::iFlags) >> 0x1a) & 1 != 0 {
            let angle = x87_quiet(roll) as f64;
            let c = x87_quiet(x87_quiet(e.call(X87_FIRST_TRIG, &args![angle]).f32()));
            let s = x87_quiet(x87_quiet(e.call(X87_SECOND_TRIG, &args![angle]).f32()));
            store4(e, at(0x20), [c, zero, s, zero]);
            store4(e, at(0x30), [zero, one, zero, zero]);
            store4(e, at(0x40), [negative_zero - s, zero, c, zero]);
            e.call(MATRIX3_MUL, &args![at(0x50), at(0x20)]);
        }
        if phantom != 0 {
            let hk = e.mem.u32(phantom + 8);
            if hk != 0 {
                e.call(EMPTY_LOCK, &args![phantom]);
                e.call(PHANTOM_SET_TRANSFORM, &args![hk, at(0x50)]);
                e.call(EMPTY_LOCK, &args![phantom]);
            }
        }
    });
}

// Translated from 00c6e680 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6e680 - the FS:[0] exception-unwinding frame and the /GS cookie check are not modeled.
/// Ground probe under `FrontBack[index]`: `*out` = that point lowered by 25, then a ray
/// from the point to `*out` (filter info from `0070c440`) is cast through vtable slot 0xC8
/// of the object at `+0x350` of the phantom's world, with an `hkpClosestRayHitCollector`
/// on the stack; on a hit `out.z += (1 - hitFraction) * 25`. The stack block keeps the
/// exe's layout (input at `+0x14`, collector at `+0xC4`).
pub fn fn_00c6e680(e: &mut Engine, this: Ptr<Ctl>, index: u32, out: Ptr) {
    let me = this.addr();
    let out = out.addr();
    e.with_stack(0x130, |e, frame| {
        let at = |o: u32| frame.addr() + o;
        let one = 0x3f80_0000u32;
        e.mem.set_u32(at(0x54), one);
        e.mem.set_u32(at(0xe4), one);
        e.mem.set_u32(at(0xc8), one);
        e.mem.set_u8(at(0x34), 0);
        e.mem.set_u32(at(0x38), 0);
        e.mem.set_u32(at(0x58), u32::MAX);
        e.mem.set_u32(at(0x84), 0);
        e.mem.set_u32(at(0x64), u32::MAX);
        e.mem.set_u32(at(0x94), 0);
        e.mem.set_u32(at(0xb4), 0);
        e.mem.set_u8(at(0xc0), 0);
        e.mem.set_u32(at(0xc4), VTABLE_CLOSEST_RAY_HIT_COLLECTOR);
        e.mem.set_u32(at(0x124), 0);
        e.mem.set_u32(at(0xe8), u32::MAX);
        e.mem.set_u32(at(0x114), 0);
        e.mem.set_u32(at(0xf4), u32::MAX);
        let point = me.wrapping_add(index.wrapping_add(0x5b).wrapping_shl(4));
        copy_words(e, out, point, 4);
        copy_words(e, at(0x14), point, 4);
        let depth: f64 = e.global(PROBE_DEPTH);
        let lowered = (e.mem.f32(out + 8) as f64 - depth) as f32;
        e.mem.set_f32(out + 8, lowered);
        copy_words(e, at(0xa4), ZERO_VECTOR, 4);
        e.mem.set_u32(at(0xb8), at(0xc4));
        e.mem.set_u32(at(0xbc), 0);
        copy_words(e, at(0x24), out, 4);
        e.call(RAY_FILTER_INFO, &args![this, at(0x10)]);
        let filter = e.mem.u32(at(0x10));
        e.mem.set_u32(at(0x38), filter);
        let proxy = e.get(this, Ctl::phkObject);
        let phantom = if proxy != 0 {
            e.call(GET_SHAPE_PHANTOM, &args![proxy]).u32()
        } else {
            0
        };
        let world = e.mem.u32(phantom.wrapping_add(8));
        let caster = if world != 0 {
            e.mem.u32(world + 0x350)
        } else {
            0
        };
        if e.vcall(caster, 0xc8, &args![at(0x14)]).bool() {
            let fraction = e.mem.f32(at(0xe4));
            let rest = (1.0 - fraction as f64) as f32;
            let z = (rest as f64 * depth + e.mem.f32(out + 8) as f64) as f32;
            e.mem.set_f32(out + 8, z);
        }
    });
}

// Translated from 00c6e820 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6e820 - the /GS cookie check is not modeled.
/// `bhkCharacterController::InitFrontBack` (Xbox PDB): with flag bit 0 set, transforms
/// four probe points by the phantom's transform (`+0xB0`) into `FrontBack[0..4]` and
/// raises each z by `fStepHeight`. With `d = fHeight * 0.5 - fRadius`, `m = d * -1` and
/// `h = fRadius` for a shape of type 9, else `-fRadius * 0.5`, the points are `(0,d,h)`,
/// `(0,m,h)`, `(m,0,h)` and `(d,0,h)` (the zero lanes are the exe's `0.0`).
pub fn bhk_character_controller_init_front_back(e: &mut Engine, this: Ptr<Ctl>) {
    if e.get(this, Ctl::iFlags) & 1 == 0 {
        return;
    }
    let me = this.addr();
    let proxy = e.get(this, Ctl::phkObject);
    let phantom = if proxy != 0 {
        e.call(GET_SHAPE_PHANTOM, &args![proxy]).u32()
    } else {
        0
    };
    let radius = x87_quiet(e.get(this, Ctl::fRadius));
    let shape_type = e.mem.u32(e.mem.u32(phantom.wrapping_add(0x10)) + 0xc);
    let half: f64 = e.global(HALF);
    let depth = (e.get(this, Ctl::fHeight) as f64 * half - radius as f64) as f32;
    let minus_one: f64 = e.global(MINUS_ONE);
    let back = (depth as f64 * minus_one) as f32;
    let side = if shape_type == 9 {
        radius
    } else {
        (-(e.get(this, Ctl::fRadius) as f64) * half) as f32
    };
    let zero: f32 = e.global(FLOAT_ZERO);
    let transform = phantom.wrapping_add(0xb0);
    e.with_stack(0x10, |e, point| {
        let p = point.addr();
        store4(e, p, [zero, depth, side, zero]);
        for i in 0..2 {
            let out = me + Ctl::FrontBack.off + 0x10 * i;
            e.call(SET_TRANSFORMED_POS, &args![out, transform, p]);
            let z = (e.get(this, Ctl::fStepHeight) as f64 + e.mem.f32(out + 8) as f64) as f32;
            e.mem.set_f32(out + 8, z);
            e.mem.set_f32(p + 4, x87_quiet(back));
        }
        store4(e, p, [back, zero, side, zero]);
        for i in 2..4 {
            let out = me + Ctl::FrontBack.off + 0x10 * i;
            e.call(SET_TRANSFORMED_POS, &args![out, transform, p]);
            let z = (e.mem.f32(out + 8) as f64 + e.get(this, Ctl::fStepHeight) as f64) as f32;
            e.mem.set_f32(out + 8, z);
            e.mem.set_f32(p, x87_quiet(depth));
        }
    });
}

// Translated from 00c6ead0 (decompiled, FalloutNV.exe 1.4.0.525)
/// cdecl `(shape)`: follows a type-10 shape to its `+0x34`, then returns it when its type
/// (`+0xC`) is 0x17, 0x1E or 0x1F, else null.
pub fn fn_00c6ead0(e: &mut Engine, shape: u32) -> u32 {
    if shape == 0 {
        return 0;
    }
    let mut shape = shape;
    let mut kind = e.mem.u32(shape + 0xc);
    if kind == 10 {
        shape = e.mem.u32(shape + 0x34);
        kind = e.mem.u32(shape + 0xc);
    }
    if matches!(kind, 0x17 | 0x1e | 0x1f) {
        shape
    } else {
        0
    }
}

// Translated from 00c6f280 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6f280 - the /GS cookie check is not modeled.
/// `bhkCharacterController::objectInteractionCallback` (Xbox PDB), `this` = the listener
/// (`controller + 0x410`). With the body (`event + 0x30`) of mass `m` and the impulse's
/// length `L`: for `0 < m < 95` (`011b0128`), the factor is 0 for motion type 6, 3.0 for
/// type 10 (0.1 when `m > 47.5`), 0.1 for 0xE and 2.0 otherwise, scaled by
/// `1000 / (L / m)` when `L / m > 1000`; when flag bit 0 is clear, bit 3 set and the event
/// point is more than 0.9 `fHeight` above the controller, the impulse becomes
/// `L * normalize(OutVelocity)`; the impulse is then scaled by the factor and the
/// reported force is `L * factor`. Otherwise the impulse is zeroed and the force is `L`.
/// A force above `fBumpedForce` replaces it and makes the body the bumped body.
pub fn bhk_character_controller_object_interaction_callback(
    e: &mut Engine,
    this: Ptr,
    _unused_1: u32,
    event: Ptr,
    impulse: Ptr,
) {
    let controller = controller_of(this);
    let impulse = impulse.addr();
    let body = e.mem.u32(event.addr() + 0x30);
    let mass = e.call(GET_MASS, &args![body + 0xe0]).f32();
    let length = sum_of_squares3(load4(e, impulse)).sqrt();
    let mut force = x87_quiet(length);
    let max_mass: f32 = e.global(MAX_PUSH_MASS);
    if mass > 0.0 && max_mass > mass {
        let motion = e.mem.u32(body + 0x2c) & 0x7f;
        let light = x87_quiet(e.global::<f32>(IMPULSE_LIGHT));
        let mut factor = match motion {
            6 => 0.0,
            10 => {
                let half: f64 = e.global(HALF);
                if max_mass as f64 * half < mass as f64 {
                    light
                } else {
                    x87_quiet(e.global::<f32>(FLOAT_THREE))
                }
            }
            0xe => light,
            _ => x87_quiet(e.global::<f32>(IMPULSE_DEFAULT)),
        };
        let ratio = (length as f64 / mass as f64) as f32;
        let limit: f64 = e.global(RATIO_LIMIT);
        if limit < ratio as f64 {
            factor = (limit / ratio as f64 * factor as f64) as f32;
        }
        let flags = e.get(controller, Ctl::iFlags);
        if flags & 1 == 0 && (flags >> 3) & 1 != 0 {
            e.with_stack(0x10, |e, position| {
                e.call(GET_POSITION, &args![controller, position]);
                let point = load4(e, event.addr());
                let here = load4(e, position.addr());
                let rise = point[2] - here[2];
                let fraction: f64 = e.global(HEIGHT_FRACTION);
                if e.get(controller, Ctl::fHeight) as f64 * fraction < rise as f64 {
                    let velocity = load4(e, controller.addr() + Ctl::OutVelocity.off);
                    let zero: f32 = e.global(FLOAT_ZERO);
                    let three: f32 = e.global(FLOAT_THREE);
                    let half: f32 = e.global(FLOAT_HALF);
                    let inverse = inverse_length(sum_of_squares3(velocity), zero, half, three);
                    store4(e, impulse, velocity.map(|v| inverse * v * force));
                }
            });
        }
        force = (length as f64 * factor as f64) as f32;
        let scaled = load4(e, impulse).map(|v| factor * v);
        store4(e, impulse, scaled);
    } else {
        store4(e, impulse, [0.0; 4]);
    }
    let best = e.get(controller, Ctl::fBumpedForce);
    if best < force {
        e.set(controller, Ctl::fBumpedForce, force);
        let body = e.mem.u32(event.addr() + 0x30);
        if body != 0 {
            e.call(ADD_REFERENCE, &args![body]);
        }
        let old = e.get(controller, Ctl::spBumpedBody);
        if old != 0 {
            e.call(REMOVE_REFERENCE, &args![old]);
        }
        e.set(controller, Ctl::spBumpedBody, body);
    }
}

// Translated from 00c6f500 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6f500 - the /GS cookie check is not modeled.
/// `bhkCharacterController::characterInteractionCallback` (Xbox PDB), `this` = the
/// listener. Keeps `proxy` as `spBumpedCharProxy`; when the other controller (`proxy +
/// 0xC0`) exists, has a priority not above ours and no flag 0x4000: pushes it along the
/// event's horizontal normal by `-0.25` times our horizontal speed (plus 20 when `eSize`
/// is 2) through `PushDelta`/`iPushCount` and `SetVelocityModifier(.., 0.01)`; and when
/// its flag bit 0 is set, turns it (`00c6cbc0` with `2 * c` and 0.2, `c` negated when the
/// contact lies to the left) by `c = clamp(t * q, 0, 1)`, where `q` is the contact's
/// sideways distance along its forward vector over half its height minus 0.05 and `t`
/// minus the cosine between our horizontal velocity and the normal.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // the x87 compare treats NaN as "not above 0"
pub fn bhk_character_controller_character_interaction_callback(
    e: &mut Engine,
    this: Ptr,
    _unused_1: u32,
    proxy: Ptr,
    event: Ptr,
) {
    let controller = controller_of(this);
    let other = e.mem.u32(proxy.addr() + 0xc0);
    e.call(ADD_REFERENCE, &args![proxy]);
    let old = e.get(controller, Ctl::spBumpedCharProxy);
    if old != 0 {
        e.call(REMOVE_REFERENCE, &args![old]);
    }
    e.set(controller, Ctl::spBumpedCharProxy, proxy.addr());
    if other == 0 {
        return;
    }
    let other = Ptr::<Ctl>::new(other);
    let size = e.get(controller, Ctl::eSize);
    if e.get(other, Ctl::iPriority) > e.get(controller, Ctl::iPriority) {
        return;
    }
    if e.get(other, Ctl::iFlags) & 0x4000 != 0 {
        return;
    }
    e.call(HKP_GET_LINEAR_VELOCITY, &args![proxy]);
    let zero: f32 = e.global(FLOAT_ZERO);
    let three: f32 = e.global(FLOAT_THREE);
    let half: f32 = e.global(FLOAT_HALF);
    let event = event.addr();

    e.with_stack(0x60, |e, frame| {
        let at = |o: u32| frame.addr() + o;
        // The event's normal, horizontal, normalized when it is not zero.
        let mut normal = load4(e, event + 0x10);
        normal[2] = 0.0;
        let sum = sum_of_squares3(normal);
        if 0.0 < sum {
            let inverse = inverse_length(sum, zero, half, three);
            normal = normal.map(|v| inverse * v);
        }
        // Our horizontal speed.
        let velocity_at = e.call(GET_HK_LINEAR_VELOCITY, &args![controller]).u32();
        let mut velocity = load4(e, velocity_at);
        velocity[2] = 0.0;
        let speed = sum_of_squares3(velocity).sqrt();
        let mut scale = x87_quiet(speed);
        if size == 2 {
            let bonus: f64 = e.global(LARGE_SIZE_BONUS);
            scale = (speed as f64 + bonus) as f32;
        }
        let push_scale: f64 = e.global(PUSH_SCALE);
        let factor = (scale as f64 * push_scale) as f32;
        store4(e, at(0x20), normal.map(|v| factor * v));
        e.call(COPY_POINT, &args![at(0x40), at(0x20)]);
        let modifier_time = x87_quiet_bits(e.global::<u32>(MODIFIER_TIME));
        let push = load4(e, other.addr() + Ctl::PushDelta.off);
        let delta = load4(e, at(0x20));
        let sum: [f32; 4] = std::array::from_fn(|lane| push[lane] + delta[lane]);
        store4(e, other.addr() + Ctl::PushDelta.off, sum);
        let count = e.get(other, Ctl::iPushCount);
        e.set(other, Ctl::iPushCount, count.wrapping_add(1));
        // `DL` still holds the 1 set before the call (the callee keeps `EDX`).
        e.call(
            SET_VELOCITY_MODIFIER,
            &args![other, at(0x40), modifier_time],
        );
        if e.get(other, Ctl::iFlags) & 1 == 0 {
            return;
        }

        // The contact point relative to the other character, horizontal.
        let point = load4(e, event);
        let position_at = e.call(GET_HK_POSITION, &args![other]).u32();
        let position = load4(e, position_at);
        let mut offset: [f32; 4] = std::array::from_fn(|lane| point[lane] - position[lane]);
        offset[2] = 0.0;
        // Its projection on the other's horizontal forward vector.
        let mut forward = load4(e, other.addr() + Ctl::ForwardVec.off);
        forward[2] = 0.0;
        let inverse = inverse_length(sum_of_squares3(forward), zero, half, three);
        let unit = forward.map(|v| inverse * v);
        let products = [0, 1, 2, 3].map(|lane| unit[lane] * offset[lane]);
        let along = x87_quiet(products[2] + (products[1] + products[0]));
        let projection = unit.map(|v| along * v);
        let distance = sum_of_squares3(projection).sqrt();
        let half_double: f64 = e.global(HALF);
        let decay: f64 = e.global(ROT_MOD_DECAY);
        let reach =
            (distance as f64 / (e.get(other, Ctl::fHeight) as f64 * half_double) - decay) as f32;
        // The normal again, horizontal and normalized.
        let mut normal = load4(e, event + 0x10);
        normal[2] = 0.0;
        let inverse = inverse_length(sum_of_squares3(normal), zero, half, three);
        let normal = normal.map(|v| inverse * v);
        // Minus the cosine between our horizontal velocity and the normal.
        let velocity_at = e.call(GET_HK_LINEAR_VELOCITY, &args![controller]).u32();
        let mut velocity = load4(e, velocity_at);
        velocity[2] = 0.0;
        let mut facing = 0.0f32;
        let sum = sum_of_squares3(velocity);
        if sum != 0.0 {
            let root = e.call(X87_SQRT, &args![x87_quiet(sum) as f64]).f32();
            let inverse = (1.0 / root as f64) as f32;
            let products = [0, 1, 2, 3].map(|lane| inverse * velocity[lane] * normal[lane]);
            let base: f32 = e.global(DOT_BASE);
            facing = x87_quiet(base - (products[2] + (products[1] + products[0])));
        }
        let product = x87_quiet((facing as f64 * reach as f64) as f32);
        let one = 0x3f80_0000u32;
        let upper = e
            .call(FLOAT_MIN, &args![one, x87_quiet_bits(product.to_bits())])
            .f32();
        let clamped = e
            .call(FLOAT_MAX, &args![0u32, x87_quiet_bits(upper.to_bits())])
            .f32();
        if !(clamped > 0.0) {
            return;
        }
        let side = projection[1] * offset[0] - projection[0] * offset[1];
        let mut turn = clamped;
        if 0.0 < side {
            let minus_one: f64 = e.global(MINUS_ONE);
            turn = x87_quiet((clamped as f64 * minus_one) as f32);
        }
        let time = x87_quiet(e.global::<f32>(ROT_MOD_TIME));
        let doubled = x87_quiet((turn as f64 + turn as f64) as f32);
        fn_00c6cbc0(e, other, doubled, time);
    });
}

/// The collidable layers `CheckInsideOfObject` skips: `(layer - 4)` indexes this table
/// from `00c6fd00` (0 = skip); layers below 4 or above 30 are tested.
const INSIDE_LAYER_TESTED: [u8; 27] = [
    0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 1, 0, 0,
];

/// Three ray casts of `shape` (vtable slot 0x20) from the input at `frame + 0x20`, each
/// first lifting both ends (z at `+0x28` and `+0x38`) by `step`; true at the first hit
/// (hit fraction below 1).
fn cast_three_rays(e: &mut Engine, frame: u32, shape: u32, step: f32, result: u32) -> bool {
    for _ in 0..3 {
        e.mem.set_u32(frame + 0x80, 0x3f80_0000);
        let from = (e.mem.f32(frame + 0x28) as f64 + step as f64) as f32;
        e.mem.set_u32(frame + 0x84, u32::MAX);
        e.mem.set_u32(frame + 0x90, u32::MAX);
        e.mem.set_f32(frame + 0x28, from);
        let to = (step as f64 + e.mem.f32(frame + 0x38) as f64) as f32;
        e.mem.set_f32(frame + 0x38, to);
        e.mem.set_u32(frame + 0xb0, 0);
        e.vcall(shape, 0x20, &args![result, frame + 0x20, frame + 0x70]);
        if 1.0 > e.mem.f32(frame + 0x80) {
            return true;
        }
    }
    false
}

// Translated from 00c6fa20 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6fa20 - the /GS cookie check is not modeled.
/// `bhkCharacterController::CheckInsideOfObject` (Xbox PDB): from the controller's
/// position to a point `GetRadius() + 32 * (01267bcc)` along the phantom's rotated y axis,
/// casts rays against every overlapping collidable of the Havok phantom (array `+0x160`,
/// count `+0x164`) whose layer is tested and whose shape type is 6, 8 or 10: in the
/// collidable's local space, three casts outward and, without a hit, three inward, each
/// lifted by a quarter of `fActorHeight` more. True when an inward cast hits.
pub fn bhk_character_controller_check_inside_of_object(e: &mut Engine, this: Ptr<Ctl>) -> bool {
    let phantom = e.get(this, Ctl::spShapePhantom);
    let hk = if phantom != 0 {
        e.mem.u32(phantom + 8)
    } else {
        0
    };
    let quarter: f64 = e.global(QUARTER);
    let step = (e.get(this, Ctl::fActorHeight) as f64 * quarter) as f32;
    let radius = e.call(GET_RADIUS, &args![this]).f64();
    let thirty_two: f64 = e.global(THIRTY_TWO);
    let lift = (radius + e.global::<f32>(INSIDE_MARGIN) as f64 * thirty_two) as f32;
    let zero: f32 = e.global(FLOAT_ZERO);
    e.with_stack(0xd0, |e, frame| {
        let f = frame.addr();
        store4(e, f + 0x50, [zero, lift, zero, zero]);
        let rotation_owner = if phantom != 0 {
            e.mem.u32(phantom + 8)
        } else {
            0
        };
        e.call(
            SET_ROTATED_DIR,
            &args![f + 0x50, rotation_owner.wrapping_add(0xb0), f + 0x50],
        );
        e.call(GET_POSITION, &args![this, f + 0x60]);
        let position = load4(e, f + 0x60);
        let direction = load4(e, f + 0x50);
        let end: [f32; 4] = std::array::from_fn(|lane| position[lane] + direction[lane]);
        store4(e, f + 0xc0, end);
        let mut index: i32 = 0;
        if e.mem.i32(hk + 0x164) <= 0 {
            return false;
        }
        loop {
            let entries = e.mem.u32(hk + 0x160);
            let collidable = e
                .mem
                .u32(entries.wrapping_add((index as u32).wrapping_mul(8)) + 4);
            if collidable != 0 {
                let layer = (e.mem.u32(collidable + 0x1c) & 0x7f).wrapping_sub(4);
                let tested = layer > 0x1a || INSIDE_LAYER_TESTED[layer as usize] != 0;
                let shape = e.mem.u32(collidable);
                if tested && matches!(e.mem.u32(shape + 0xc), 6 | 8 | 10) {
                    e.mem.set_u32(f + 0x80, 0x3f80_0000);
                    e.mem.set_u32(f + 0x84, u32::MAX);
                    e.mem.set_u32(f + 0x90, u32::MAX);
                    e.mem.set_u32(f + 0x40, 0);
                    e.mem.set_u32(f + 0x44, 0);
                    e.mem.set_u32(f + 0xb0, 0);
                    let transform = e.mem.u32(collidable + 8);
                    e.call(
                        SET_TRANSFORMED_INVERSE_POS,
                        &args![f + 0x20, transform, f + 0x60],
                    );
                    let transform = e.mem.u32(collidable + 8);
                    e.call(
                        SET_TRANSFORMED_INVERSE_POS,
                        &args![f + 0x30, transform, f + 0xc0],
                    );
                    let outward = cast_three_rays(e, f, shape, step, f + 0x1b);
                    let transform = e.mem.u32(collidable + 8);
                    e.call(
                        SET_TRANSFORMED_INVERSE_POS,
                        &args![f + 0x20, transform, f + 0xc0],
                    );
                    let transform = e.mem.u32(collidable + 8);
                    e.call(
                        SET_TRANSFORMED_INVERSE_POS,
                        &args![f + 0x30, transform, f + 0x60],
                    );
                    if !outward && cast_three_rays(e, f, shape, step, f + 0x1a) {
                        return true;
                    }
                }
            }
            index += 1;
            if index >= e.mem.i32(hk + 0x164) {
                return false;
            }
        }
    })
}

// Translated from 00c6fd80 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c6fd80 - the FS:[0] exception-unwinding frame is not modeled.
/// `bhkCharacterController::BuildDisplayGeometry` (Xbox PDB): unless the display node
/// exists already, attaches a new `NiNode` named `"bhkColDisp"` to `node` (or, when
/// `node` is null, to the phantom's scene object, or that object's first child when it is
/// an `NiNode` and it has children), sets the node's `+0x58..+0x60` to `RotCenter` over
/// the parent's scale (`+0x98`) and `+0x64` to `|1 / scale|` when the scale is not 0,
/// hands it to the current shape's slot 0xD4 and sets flag 0x8000. Nothing without a
/// current shape. The allocation-scope word is 0x10 meanwhile.
pub fn bhk_character_controller_build_display_geometry(e: &mut Engine, this: Ptr<Ctl>, node: u32) {
    let (word, saved) = enter_scope(e);
    if bhk_character_controller_find_display_geometry(e, this) == 0 {
        build_display_node(e, this, node);
    }
    e.mem.set_u32(word, saved);
}

fn build_display_node(e: &mut Engine, this: Ptr<Ctl>, node: u32) {
    let mut node = node;
    if node == 0 {
        let phantom = e.get(this, Ctl::spShapePhantom);
        if phantom == 0 {
            return;
        }
        let object = e.call(GET_AV_OBJECT, &args![phantom, 0u32]).u32();
        if object == 0 {
            return;
        }
        node = e.vcall(object, 0xc, &args![]).u32();
        if node == 0 {
            return;
        }
        if e.mem.u16(node + 0xa6) != 0 {
            let child = e.call(NODE_CHILD_AT, &args![node, 0u32]).u32();
            if child == 0 {
                return;
            }
            node = e.call(IS_KIND_OF, &args![child, NODE_RTTI]).u32();
        }
        if node == 0 {
            return;
        }
    }
    if bhk_character_controller_get_shape(e, this) == 0 {
        return;
    }
    let block = e.call(HEAP_ALLOCATE, &args![NODE_SIZE]).u32();
    let display = if block != 0 {
        e.call(NODE_CONSTRUCT, &args![block, 0u32]).u32()
    } else {
        0
    };
    e.vcall(node, 0xdc, &args![display, 1u32]);
    let name = e.call(FIXED_STRING_CREATE, &args![DISPLAY_NAME]).u32();
    let name = e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), name);
        e.call(SET_NAME, &args![display, slot]);
        e.mem.u32(slot.addr())
    });
    release_fixed_string(e, name);
    let inverse = (1.0 / e.mem.f32(node + 0x98) as f64) as f32;
    let center = e.with_stack(0x10, |e, copy| {
        let at = e
            .call(COPY_POINT, &args![copy, this.addr() + Ctl::RotCenter.off])
            .u32();
        [0, 4, 8].map(|o| e.mem.f32(at + o))
    });
    let x = (center[0] as f64 * inverse as f64) as f32;
    let y = (center[1] as f64 * inverse as f64) as f32;
    let z = (inverse as f64 * center[2] as f64) as f32;
    e.mem.set_f32(display.wrapping_add(0x58), x);
    e.mem.set_f32(display.wrapping_add(0x5c), y);
    e.mem.set_f32(display.wrapping_add(0x60), z);
    let scale = e.mem.f32(node + 0x98);
    if 0.0 != scale {
        let inverse = (1.0 / scale as f64) as f32;
        e.mem.set_f32(
            display.wrapping_add(0x64),
            x87_quiet(x87_quiet(inverse).abs()),
        );
    }
    let shape = bhk_character_controller_get_shape(e, this);
    e.vcall(shape, 0xd4, &args![display, DISPLAY_ARGUMENT]);
    let flags = e.get(this, Ctl::iFlags);
    e.set(this, Ctl::iFlags, flags | 0x8000);
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00c6_cbc0, fn_00c6cbc0(Ptr<Ctl>, f32, f32)),
        entry!(
            0x00c6_cc10,
            bhk_character_controller_apply_rotation_modifier(Ptr<Ctl>, f32, f32) -> f32
        ),
        entry!(0x00c6_cca0, fn_00c6cca0(Ptr)),
        entry!(0x00c6_ccc0, fn_00c6ccc0(Ptr, Ptr, Ptr)),
        entry!(0x00c6_cce0, fn_00c6cce0(Ptr, Ptr, Ptr)),
        entry!(0x00c6_cd00, fn_00c6cd00(Ptr, Ptr)),
        entry!(0x00c6_cd30, fn_00c6cd30(Ptr, Ptr) -> Ptr),
        entry!(0x00c6_cd60, fn_00c6cd60(Ptr, Ptr) -> Ptr),
        entry!(0x00c6_cd90, bhk_shape_collection_get_rtti(Ptr) -> u32),
        entry!(
            0x00c6_cda0,
            bhk_shape_collection_scalar_deleting_destructor(Ptr, u8) -> Ptr
        ),
        entry!(0x00c6_cdc0, fn_00c6cdc0(u32)),
        entry!(0x00c6_cee0, bhk_character_proxy_q_cinfo_size(Ptr) -> u32),
        entry!(0x00c6_cf30, bhk_character_proxy_integrate(Ptr, u32)),
        entry!(
            0x00c6_d010,
            bhk_character_proxy_scalar_deleting_destructor(Ptr, u8) -> Ptr
        ),
        entry!(0x00c6_d040, bhk_caching_shape_phantom_construct(Ptr, u32) -> Ptr),
        entry!(0x00c6_d0e0, bhk_caching_shape_phantom_get_rtti(Ptr) -> u32),
        entry!(0x00c6_d0f0, fn_00c6d0f0(Ptr) -> u32),
        entry!(
            0x00c6_d100,
            bhk_caching_shape_phantom_scalar_deleting_destructor(Ptr, u8) -> Ptr
        ),
        entry!(0x00c6_d120, fn_00c6d120(Ptr<Ctl>) -> bool),
        entry!(0x00c6_d150, fn_00c6d150(Ptr<Ctl>)),
        entry!(0x00c6_d170, hkp_closest_cd_point_collector_reset(Ptr)),
        entry!(0x00c6_d190, fn_00c6d190(Ptr) -> Ptr),
        entry!(0x00c6_d1e0, fn_00c6d1e0(Ptr, u32, u32) -> Ptr),
        entry!(0x00c6_d250, bhk_collision_box_get_rtti(Ptr) -> u32),
        entry!(
            0x00c6_d260,
            bhk_collision_box_scalar_deleting_destructor(Ptr, u8) -> Ptr
        ),
        entry!(0x00c6_d290, fn_00c6d290(Ptr, u32) -> Ptr),
        entry!(0x00c6_d310, bhk_list_shape_get_rtti(Ptr) -> u32),
        entry!(
            0x00c6_d320,
            bhk_list_shape_scalar_deleting_destructor(Ptr, u8) -> Ptr
        ),
        entry!(0x00c6_d340, fn_00c6d340()),
        entry!(0x00c6_d3c0, fn_00c6d3c0()),
        entry!(
            0x00c6_d3f0,
            bhk_character_controller_set_shape(Ptr<Ctl>, u32, u32) -> bool
        ),
        entry!(0x00c6_d4a0, fn_00c6d4a0(Ptr<Ctl>)),
        entry!(
            0x00c6_d5c0,
            bhk_character_controller_apply_velocity_mod(Ptr<Ctl>)
        ),
        entry!(0x00c6_d620, fn_00c6d620(Ptr, u32, u32)),
        entry!(0x00c6_d670, fn_00c6d670(Ptr, Ptr) -> Ptr),
        entry!(0x00c6_d770, fn_00c6d770(Ptr<Ctl>, u32)),
        entry!(
            0x00c6_d7a0,
            bhk_character_controller_set_ni_object(Ptr<Ctl>, u32)
        ),
        entry!(0x00c6_d7e0, fn_00c6d7e0(u32, u8, u8, i32) -> u8),
        entry!(
            0x00c6_d840,
            hkp_world_object_get_property(Ptr, Ptr, u32, u32) -> Ptr
        ),
        entry!(0x00c6_d8a0, fn_00c6d8a0(Ptr, u32) -> bool),
        entry!(0x00c6_d900, fn_00c6d900(u32) -> u32),
        entry!(
            0x00c6_d960,
            bhk_character_controller_get_shape(Ptr<Ctl>) -> u32
        ),
        entry!(
            0x00c6_d9c0,
            hkp_closest_cd_point_collector_scalar_deleting_destructor(Ptr, u8) -> Ptr
        ),
        entry!(0x00c6_d9f0, fn_00c6d9f0(Ptr) -> Ptr),
        entry!(0x00c6_de10, fn_00c6de10(Ptr)),
        entry!(
            0x00c6_dec0,
            bhk_character_controller_find_bs_reference(u32, u32) -> u32
        ),
        entry!(0x00c6_def0, fn_00c6def0(u32) -> bool),
        entry!(0x00c6_df60, fn_00c6df60(Ptr, Ptr) -> u32),
        entry!(0x00c6_e050, fn_00c6e050(Ptr<Ctl>, u32, i32)),
        entry!(
            0x00c6_e0b0,
            bhk_character_controller_find_display_geometry(Ptr<Ctl>) -> u32
        ),
        entry!(
            0x00c6_e180,
            bhk_character_controller_set_radius(Ptr<Ctl>, f32)
        ),
        entry!(0x00c6_e2e0, fn_00c6e2e0(Ptr<Ctl>)),
        entry!(
            0x00c6_e420,
            bhk_character_controller_apply_pitch_and_roll(Ptr<Ctl>, Ptr)
        ),
        entry!(0x00c6_e680, fn_00c6e680(Ptr<Ctl>, u32, Ptr)),
        entry!(
            0x00c6_e820,
            bhk_character_controller_init_front_back(Ptr<Ctl>)
        ),
        entry!(0x00c6_ead0, fn_00c6ead0(u32) -> u32),
        entry!(
            0x00c6_f280,
            bhk_character_controller_object_interaction_callback(Ptr, u32, Ptr, Ptr)
        ),
        entry!(
            0x00c6_f500,
            bhk_character_controller_character_interaction_callback(Ptr, u32, Ptr, Ptr)
        ),
        entry!(
            0x00c6_fa20,
            bhk_character_controller_check_inside_of_object(Ptr<Ctl>) -> bool
        ),
        entry!(
            0x00c6_fd80,
            bhk_character_controller_build_display_geometry(Ptr<Ctl>, u32)
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scratch words the doubles write their observations to (inside the mapped exe range).
    const SCRATCH: u32 = 0x012f_0000;

    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x0100_0000, 0x0030_0000);
        e.map(0x00fd_f000, 0x1000);
        e.call_log = Some(vec![]);
        e
    }

    fn ret(eax: u32) -> Ret {
        Ret {
            eax,
            ..Ret::default()
        }
    }

    fn ret_st0(st0: f64) -> Ret {
        Ret {
            st0,
            ..Ret::default()
        }
    }

    fn nothing(_: &mut Engine, _: &[u32]) -> Ret {
        Ret::default()
    }

    fn stub(e: &mut Engine, addresses: &[u32]) {
        for a in addresses {
            e.register(*a, nothing);
        }
    }

    fn returns(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| ret(value));
    }

    /// The argument words of every logged call to `address`.
    fn calls(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|c| c.0 == address)
            .map(|c| c.1.clone())
            .collect()
    }

    /// The logged call addresses, in order, among `of`.
    fn order(e: &Engine, of: &[u32]) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|c| c.0)
            .filter(|a| of.contains(a))
            .collect()
    }

    /// A heap object whose fresh vtable has the given `(byte offset, target)` slots.
    fn with_vtable(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
        let vtable = e.mem.alloc(0x200);
        for (offset, target) in slots {
            e.mem.set_u32(vtable + offset, *target);
        }
        let object = e.mem.alloc(0x700);
        e.mem.set_u32(object, vtable);
        object
    }

    fn controller(e: &mut Engine) -> Ptr<Ctl> {
        e.new_object::<Ctl>()
    }

    fn vec_at(e: &mut Engine, values: [f32; 4]) -> u32 {
        let a = e.mem.alloc(0x10);
        store4(e, a, values);
        a
    }

    /// A Havok allocator reached through `MEMORY_ROUTER`: slot 4 allocates (returns
    /// `block`), slot 8 frees (logged at `0x0900_0f08`).
    fn install_allocator(e: &mut Engine, block: u32) {
        e.register_double(0x0900_0f04, move |_, _| ret(block));
        e.register(0x0900_0f08, nothing);
        let allocator = with_vtable(e, &[(4, 0x0900_0f04), (8, 0x0900_0f08)]);
        let router = e.mem.alloc(0x20);
        e.mem.set_u32(router + 0x10, allocator);
        returns(e, MEMORY_ROUTER, router);
    }

    #[test]
    fn rot_mod_is_replaced_only_by_a_larger_magnitude() {
        let mut e = engine();
        let c = controller(&mut e);
        e.set(c, Ctl::fRotMod, 0.5);
        fn_00c6cbc0(&mut e, c, -0.75, f32::from_bits(0x7f80_0001));
        assert_eq!(e.get(c, Ctl::fRotMod), -0.75);
        assert_eq!(e.get(c, Ctl::fRotModTime).to_bits(), 0x7fc0_0001);
        fn_00c6cbc0(&mut e, c, 0.25, 9.0);
        fn_00c6cbc0(&mut e, c, f32::NAN, 9.0);
        assert_eq!(e.get(c, Ctl::fRotMod), -0.75);
        assert_eq!(e.get(c, Ctl::fRotModTime).to_bits(), 0x7fc0_0001);
    }

    #[test]
    fn rotation_modifier_uses_the_shorter_time_and_decays() {
        let mut e = engine();
        e.set_global(ROT_MOD_DECAY, 0.05f32 as f64);
        let c = controller(&mut e);
        e.set(c, Ctl::fRotMod, 2.0);
        e.set(c, Ctl::fRotModTime, 0.5);
        let r = bhk_character_controller_apply_rotation_modifier(&mut e, c, 1.0, 0.25);
        assert_eq!(r, 1.5);
        assert_eq!(e.get(c, Ctl::fRotModTime), 0.25);
        assert_eq!(e.get(c, Ctl::fRotMod), (2.0f64 - 0.05f32 as f64) as f32);
        e.set(c, Ctl::fRotMod, 2.0);
        e.set(c, Ctl::fRotModTime, 0.125);
        let r = bhk_character_controller_apply_rotation_modifier(&mut e, c, 1.0, 0.25);
        assert_eq!(r, 1.25);
        assert_eq!(e.get(c, Ctl::fRotModTime), -0.125);
        for remaining in [0.0, f32::NAN] {
            e.set(c, Ctl::fRotMod, 2.0);
            e.set(c, Ctl::fRotModTime, remaining);
            let r = bhk_character_controller_apply_rotation_modifier(&mut e, c, 3.0, 0.25);
            assert_eq!(r, 3.0);
            assert_eq!(e.get(c, Ctl::fRotMod), 0.0);
        }
    }

    #[test]
    fn ref_pointer_release_clears_the_slot() {
        let mut e = engine();
        stub(&mut e, &[REMOVE_REFERENCE]);
        let slot = e.mem.alloc(4);
        e.mem.set_u32(slot, 0x4444);
        fn_00c6cca0(&mut e, Ptr::new(slot));
        fn_00c6cca0(&mut e, Ptr::new(slot));
        assert_eq!(calls(&e, REMOVE_REFERENCE), vec![vec![0x4444]]);
        assert_eq!(e.mem.u32(slot), 0);
    }

    #[test]
    fn first_lane_difference_keeps_the_other_lanes() {
        let mut e = engine();
        let a = vec_at(&mut e, [5.0, 6.0, 7.0, 8.0]);
        let b = vec_at(&mut e, [2.0, 100.0, 100.0, 100.0]);
        let out = e.mem.alloc(0x10);
        fn_00c6ccc0(&mut e, Ptr::new(out), Ptr::new(a), Ptr::new(b));
        assert_eq!(load4(&e, out), [3.0, 6.0, 7.0, 8.0]);
    }

    #[test]
    fn first_lane_sum_keeps_the_other_lanes() {
        let mut e = engine();
        let a = vec_at(&mut e, [5.0, 6.0, 7.0, 8.0]);
        let b = vec_at(&mut e, [2.0, 100.0, 100.0, 100.0]);
        let out = e.mem.alloc(0x10);
        fn_00c6cce0(&mut e, Ptr::new(out), Ptr::new(a), Ptr::new(b));
        assert_eq!(load4(&e, out), [7.0, 6.0, 7.0, 8.0]);
    }

    fn filled(e: &mut Engine, words: u32) -> u32 {
        let a = e.mem.alloc(4 * words + 8);
        for i in 0..words {
            e.mem.set_u32(a + 4 * i, 0x7f80_0001 + i);
        }
        a
    }

    #[test]
    fn copy_of_three_vectors() {
        let mut e = engine();
        let src = filled(&mut e, 13);
        let dst = e.mem.alloc(0x40);
        fn_00c6cd00(&mut e, Ptr::new(dst), Ptr::new(src));
        assert_eq!(e.mem.bytes(dst, 0x30), e.mem.bytes(src, 0x30));
        assert_eq!(e.mem.u32(dst + 0x30), 0);
    }

    #[test]
    fn copy_of_three_vectors_returns_this() {
        let mut e = engine();
        let src = filled(&mut e, 13);
        let dst = e.mem.alloc(0x40);
        assert_eq!(
            fn_00c6cd30(&mut e, Ptr::new(dst), Ptr::new(src)).addr(),
            dst
        );
        assert_eq!(e.mem.bytes(dst, 0x30), e.mem.bytes(src, 0x30));
        assert_eq!(e.mem.u32(dst + 0x30), 0);
    }

    #[test]
    fn copy_of_four_vectors_returns_this() {
        let mut e = engine();
        let src = filled(&mut e, 17);
        let dst = e.mem.alloc(0x48);
        assert_eq!(
            fn_00c6cd60(&mut e, Ptr::new(dst), Ptr::new(src)).addr(),
            dst
        );
        assert_eq!(e.mem.bytes(dst, 0x40), e.mem.bytes(src, 0x40));
        assert_eq!(e.mem.u32(dst + 0x40), 0);
    }

    #[test]
    fn shape_collection_rtti() {
        let mut e = engine();
        assert_eq!(
            bhk_shape_collection_get_rtti(&mut e, Ptr::NULL),
            0x0126_86d4
        );
    }

    #[test]
    fn shape_collection_deleting_destructor_frees_0x14_bytes() {
        let mut e = engine();
        stub(&mut e, &[SHAPE_COLLECTION_DESTRUCT, FREE_SIZED]);
        let this = Ptr::new(0x5000);
        let r = bhk_shape_collection_scalar_deleting_destructor(&mut e, this, 0);
        assert_eq!(r.addr(), 0x5000);
        assert!(calls(&e, FREE_SIZED).is_empty());
        bhk_shape_collection_scalar_deleting_destructor(&mut e, this, 1);
        assert_eq!(calls(&e, SHAPE_COLLECTION_DESTRUCT).len(), 2);
        assert_eq!(calls(&e, FREE_SIZED), vec![vec![0x5000, 0x14]]);
    }

    #[test]
    fn cinfo_block_is_freed_through_havok() {
        let mut e = engine();
        install_allocator(&mut e, 0);
        fn_00c6cdc0(&mut e, 0);
        assert!(calls(&e, 0x0900_0f08).is_empty());
        fn_00c6cdc0(&mut e, 0x7000);
        assert_eq!(calls(&e, 0x0900_0f08)[0][1..], [0x7000, 0x80]);
    }

    #[test]
    fn proxy_cinfo_size() {
        let mut e = engine();
        assert_eq!(bhk_character_proxy_q_cinfo_size(&mut e, Ptr::NULL), 0x80);
    }

    #[test]
    fn integrate_runs_the_collectors_between_unmark_writes() {
        let mut e = engine();
        stub(
            &mut e,
            &[
                UN_MARK_WRITE,
                POINT_COLLECTOR_CONSTRUCT,
                INTEGRATE_WITH_COLLECTORS,
                POINT_COLLECTOR_HANDLE_COLLISIONS,
                POINT_COLLECTOR_DESTRUCT,
            ],
        );
        returns(&mut e, 0x0900_0094, 0x6600);
        let this = with_vtable(&mut e, &[(0x94, 0x0900_0094)]);
        e.mem.set_u32(this + 8, 0x7700);
        bhk_character_proxy_integrate(&mut e, Ptr::new(this), 0x8800);
        let all = [
            UN_MARK_WRITE,
            POINT_COLLECTOR_CONSTRUCT,
            0x0900_0094,
            INTEGRATE_WITH_COLLECTORS,
            POINT_COLLECTOR_HANDLE_COLLISIONS,
            POINT_COLLECTOR_DESTRUCT,
        ];
        let mut expected = all.to_vec();
        expected.push(UN_MARK_WRITE);
        assert_eq!(order(&e, &all), expected);
        let local = calls(&e, POINT_COLLECTOR_CONSTRUCT)[0][0];
        assert_eq!(calls(&e, POINT_COLLECTOR_CONSTRUCT)[0][1], this + 0x10);
        assert_eq!(
            calls(&e, INTEGRATE_WITH_COLLECTORS)[0],
            vec![0x7700, 0x8800, 0x6610, local, this + 0x10]
        );
        assert_eq!(
            calls(&e, POINT_COLLECTOR_HANDLE_COLLISIONS)[0],
            vec![this + 0x10]
        );
        assert_eq!(calls(&e, POINT_COLLECTOR_DESTRUCT)[0], vec![local]);

        e.call_log = Some(vec![]);
        e.mem.set_u32(this + 8, 0);
        bhk_character_proxy_integrate(&mut e, Ptr::new(this), 0x8800);
        assert_eq!(order(&e, &all), vec![UN_MARK_WRITE, UN_MARK_WRITE]);
    }

    #[test]
    fn proxy_deleting_destructor_frees_0x3e0_bytes() {
        let mut e = engine();
        stub(&mut e, &[CHARACTER_PROXY_DESTRUCT, FREE_SIZED]);
        bhk_character_proxy_scalar_deleting_destructor(&mut e, Ptr::new(0x5000), 2);
        assert!(calls(&e, FREE_SIZED).is_empty());
        bhk_character_proxy_scalar_deleting_destructor(&mut e, Ptr::new(0x5000), 3);
        assert_eq!(calls(&e, FREE_SIZED), vec![vec![0x5000, 0x3e0]]);
        assert_eq!(calls(&e, CHARACTER_PROXY_DESTRUCT).len(), 2);
    }

    fn check_vtable_is(expected: u32) -> impl FnMut(&mut Engine, &[u32]) -> Ret {
        move |e, a| {
            assert_eq!(e.mem.u32(a[0]), expected);
            Ret::default()
        }
    }

    #[test]
    fn caching_shape_phantom_constructor() {
        let mut e = engine();
        stub(&mut e, &[REF_OBJECT_CONSTRUCT]);
        e.register_double(
            CACHING_PHANTOM_INIT,
            check_vtable_is(VTABLE_CACHING_SHAPE_PHANTOM),
        );
        e.set_global(COUNT_WORLD_OBJECT, 4u32);
        let this = e.mem.alloc(0x18);
        for o in [0xc, 0x10, 0x14] {
            e.mem.set_u32(this + o, 0xffff_ffff);
        }
        let r = bhk_caching_shape_phantom_construct(&mut e, Ptr::new(this), 0x99);
        assert_eq!(r.addr(), this);
        assert_eq!(e.mem.u32(this), VTABLE_CACHING_SHAPE_PHANTOM);
        assert_eq!(e.mem.u32(this + 0xc), 0);
        assert_eq!(e.mem.u32(this + 0x10), 0);
        assert_eq!(e.mem.u8(this + 0x14), 0);
        assert_eq!(e.mem.u8(this + 0x15), 0xff);
        assert_eq!(calls(&e, CACHING_PHANTOM_INIT), vec![vec![this, 0x99]]);
        assert_eq!(e.global::<u32>(COUNT_WORLD_OBJECT), 5);
        for counter in [
            COUNT_PHANTOM,
            COUNT_SHAPE_PHANTOM,
            COUNT_CACHING_SHAPE_PHANTOM,
        ] {
            assert_eq!(e.global::<u32>(counter), 1);
        }
    }

    #[test]
    fn caching_shape_phantom_rtti() {
        let mut e = engine();
        assert_eq!(
            bhk_caching_shape_phantom_get_rtti(&mut e, Ptr::NULL),
            0x0126_85a4
        );
    }

    #[test]
    fn size_query_is_0x60() {
        let mut e = engine();
        assert_eq!(fn_00c6d0f0(&mut e, Ptr::NULL), 0x60);
    }

    #[test]
    fn caching_shape_phantom_deleting_destructor_frees_0x18_bytes() {
        let mut e = engine();
        stub(&mut e, &[CACHING_PHANTOM_DESTRUCT, FREE_SIZED]);
        bhk_caching_shape_phantom_scalar_deleting_destructor(&mut e, Ptr::new(0x5000), 1);
        assert_eq!(calls(&e, CACHING_PHANTOM_DESTRUCT), vec![vec![0x5000]]);
        assert_eq!(calls(&e, FREE_SIZED), vec![vec![0x5000, 0x18]]);
    }

    #[test]
    fn states_four_to_six_count() {
        let mut e = engine();
        let c = controller(&mut e);
        for (state, expected) in [(3, false), (4, true), (5, true), (6, true), (7, false)] {
            returns(&mut e, GET_STATE, state);
            assert_eq!(fn_00c6d120(&mut e, c), expected);
        }
        assert_eq!(calls(&e, GET_STATE)[0], vec![c.addr() + 0x3e0]);
    }

    #[test]
    fn push_delta_is_cleared_with_its_count() {
        let mut e = engine();
        let c = controller(&mut e);
        store4(&mut e, c.addr() + 0x580, [1.0, 2.0, 3.0, 4.0]);
        fn_00c6d150(&mut e, c);
        assert_eq!(load4(&e, c.addr() + 0x580), [1.0, 2.0, 3.0, 4.0]);
        e.set(c, Ctl::iPushCount, 3);
        fn_00c6d150(&mut e, c);
        assert_eq!(load4(&e, c.addr() + 0x580), [0.0; 4]);
        assert_eq!(e.get(c, Ctl::iPushCount), 0);
    }

    #[test]
    fn collector_reset_sets_the_far_distances() {
        let mut e = engine();
        e.set_global(COLLECTOR_FAR, 0x7f7f_ffeeu32);
        let this = e.mem.alloc(0x34);
        e.mem.set_u32(this + 0x30, 9);
        hkp_closest_cd_point_collector_reset(&mut e, Ptr::new(this));
        assert_eq!(e.mem.u32(this + 0x30), 0);
        assert_eq!(e.mem.u32(this + 0x2c), 0x7f7f_ffee);
        assert_eq!(e.mem.u32(this + 4), 0x7f7f_ffee);
    }

    #[test]
    fn simple_shape_phantom_constructor() {
        let mut e = engine();
        stub(&mut e, &[REF_OBJECT_CONSTRUCT]);
        let this = e.mem.alloc(0x18);
        e.mem.set_u8(this + 0x14, 7);
        assert_eq!(fn_00c6d190(&mut e, Ptr::new(this)).addr(), this);
        assert_eq!(e.mem.u32(this), VTABLE_SIMPLE_SHAPE_PHANTOM);
        assert_eq!(e.mem.u8(this + 0x14), 0);
        assert_eq!(calls(&e, REF_OBJECT_CONSTRUCT), vec![vec![this]]);
        assert_eq!(e.global::<u32>(COUNT_SIMPLE_SHAPE_PHANTOM), 1);
        assert_eq!(e.global::<u32>(COUNT_SHAPE_PHANTOM), 1);
    }

    #[test]
    fn collision_box_constructor_runs_init() {
        let mut e = engine();
        stub(&mut e, &[REF_OBJECT_CONSTRUCT]);
        e.register_double(COLLISION_BOX_INIT, check_vtable_is(VTABLE_COLLISION_BOX));
        let this = e.mem.alloc(0x20);
        assert_eq!(fn_00c6d1e0(&mut e, Ptr::new(this), 0x11, 0x22).addr(), this);
        assert_eq!(calls(&e, COLLISION_BOX_INIT), vec![vec![this, 0x11, 0x22]]);
        assert_eq!(e.global::<u32>(COUNT_SIMPLE_SHAPE_PHANTOM), 1);
    }

    #[test]
    fn collision_box_rtti() {
        let mut e = engine();
        assert_eq!(bhk_collision_box_get_rtti(&mut e, Ptr::NULL), 0x0126_8444);
    }

    #[test]
    fn collision_box_deleting_destructor_frees_the_aligned_block() {
        let mut e = engine();
        stub(&mut e, &[COLLISION_BOX_DESTRUCT, MEMORY_DEALLOCATE]);
        let block = e.mem.alloc(0x40);
        let this = block + 8;
        e.mem.set_u8(this - 1, 8);
        bhk_collision_box_scalar_deleting_destructor(&mut e, Ptr::new(this), 0);
        assert!(calls(&e, MEMORY_DEALLOCATE).is_empty());
        bhk_collision_box_scalar_deleting_destructor(&mut e, Ptr::new(this), 1);
        assert_eq!(
            calls(&e, MEMORY_DEALLOCATE),
            vec![vec![MEMORY_MANAGER, block]]
        );
        assert_eq!(calls(&e, COLLISION_BOX_DESTRUCT).len(), 2);
    }

    #[test]
    fn list_shape_constructor_runs_init() {
        let mut e = engine();
        stub(&mut e, &[REF_OBJECT_CONSTRUCT]);
        e.register_double(LIST_SHAPE_INIT, check_vtable_is(VTABLE_LIST_SHAPE));
        let this = e.mem.alloc(0x14);
        e.mem.set_u32(this + 0xc, 5);
        assert_eq!(fn_00c6d290(&mut e, Ptr::new(this), 0x33).addr(), this);
        assert_eq!(e.mem.u32(this + 0xc), 0);
        assert_eq!(calls(&e, LIST_SHAPE_INIT), vec![vec![this, 0x33]]);
        for counter in [COUNT_SHAPE, COUNT_SHAPE_COLLECTION, COUNT_LIST_SHAPE] {
            assert_eq!(e.global::<u32>(counter), 1);
        }
    }

    #[test]
    fn list_shape_rtti() {
        let mut e = engine();
        assert_eq!(bhk_list_shape_get_rtti(&mut e, Ptr::NULL), 0x0126_81a8);
    }

    #[test]
    fn list_shape_deleting_destructor_frees_0x14_bytes() {
        let mut e = engine();
        stub(&mut e, &[LIST_SHAPE_DESTRUCT, FREE_SIZED]);
        bhk_list_shape_scalar_deleting_destructor(&mut e, Ptr::new(0x5000), 1);
        assert_eq!(calls(&e, LIST_SHAPE_DESTRUCT), vec![vec![0x5000]]);
        assert_eq!(calls(&e, FREE_SIZED), vec![vec![0x5000, 0x14]]);
    }

    #[test]
    fn static_reference_release_runs_in_scope() {
        let mut e = engine();
        e.register(REMOVE_REFERENCE, |e, _| {
            let word = e.tls() + SCOPE_WORD;
            assert_eq!(e.mem.u32(word), 0x10);
            Ret::default()
        });
        let word = e.tls() + SCOPE_WORD;
        e.mem.set_u32(word, 7);
        fn_00c6d340(&mut e);
        assert!(calls(&e, REMOVE_REFERENCE).is_empty());
        e.set_global(STATIC_REFERENCED_OBJECT, 0x4321u32);
        fn_00c6d340(&mut e);
        assert_eq!(calls(&e, REMOVE_REFERENCE), vec![vec![0x4321]]);
        assert_eq!(e.mem.u32(word), 7);
        assert_eq!(e.global::<u32>(STATIC_REFERENCED_OBJECT), 0x4321);
    }

    #[test]
    fn static_ni_pointer_release() {
        let mut e = engine();
        e.register(0x0900_0004, nothing);
        let object = with_vtable(&mut e, &[(4, 0x0900_0004)]);
        returns(&mut e, INTERLOCKED_DECREMENT, 1);
        e.set_global(STATIC_NI_POINTER, object);
        fn_00c6d3c0(&mut e);
        assert_eq!(calls(&e, INTERLOCKED_DECREMENT), vec![vec![object + 4]]);
        assert!(calls(&e, 0x0900_0004).is_empty());
        assert_eq!(e.global::<u32>(STATIC_NI_POINTER), 0);
        returns(&mut e, INTERLOCKED_DECREMENT, 0);
        e.set_global(STATIC_NI_POINTER, object);
        fn_00c6d3c0(&mut e);
        assert_eq!(calls(&e, 0x0900_0004), vec![vec![object]]);
        fn_00c6d3c0(&mut e);
        assert_eq!(calls(&e, INTERLOCKED_DECREMENT).len(), 2);
    }

    #[test]
    fn set_shape_swaps_the_phantom_shape_in_its_world() {
        let mut e = engine();
        stub(
            &mut e,
            &[
                UN_MARK_READ,
                HKP_GET_POSITION,
                WORLD_REMOVE_PHANTOM,
                WORLD_ADD_PHANTOM,
            ],
        );
        e.register(0x0900_000c, nothing);
        let phantom = with_vtable(&mut e, &[(0xc, 0x0900_000c)]);
        e.mem.set_u32(phantom + 8, 0x6600);
        e.mem.set_u32(phantom + 0x10, 0x1111);
        returns(&mut e, GET_SHAPE_PHANTOM, phantom);
        let c = controller(&mut e);
        e.set(c, Ctl::phkObject, 0x5500);
        let shape = e.mem.alloc(0x10);
        e.mem.set_u32(shape + 8, 0x1111);
        assert!(!bhk_character_controller_set_shape(&mut e, c, shape, 0));
        e.mem.set_u32(shape + 8, 0x2222);
        assert!(bhk_character_controller_set_shape(&mut e, c, shape, 0));
        let all = [
            UN_MARK_READ,
            HKP_GET_POSITION,
            WORLD_REMOVE_PHANTOM,
            0x0900_000c,
            WORLD_ADD_PHANTOM,
        ];
        assert_eq!(
            order(&e, &all),
            vec![
                UN_MARK_READ,
                HKP_GET_POSITION,
                UN_MARK_READ,
                WORLD_REMOVE_PHANTOM,
                0x0900_000c,
                WORLD_ADD_PHANTOM
            ]
        );
        assert_eq!(calls(&e, UN_MARK_READ)[0], vec![c.addr()]);
        assert_eq!(calls(&e, HKP_GET_POSITION)[0], vec![0x5500]);
        assert_eq!(calls(&e, WORLD_REMOVE_PHANTOM)[0], vec![0x6600, phantom]);
        assert_eq!(calls(&e, 0x0900_000c)[0], vec![phantom, 0x2222]);
        assert_eq!(calls(&e, WORLD_ADD_PHANTOM)[0], vec![0x6600, phantom]);
        // No shape: the phantom gets none; without a proxy nothing happens.
        e.call_log = Some(vec![]);
        assert!(bhk_character_controller_set_shape(&mut e, c, 0, 0));
        assert_eq!(calls(&e, 0x0900_000c)[0], vec![phantom, 0]);
        e.set(c, Ctl::phkObject, 0);
        assert!(!bhk_character_controller_set_shape(&mut e, c, shape, 0));
    }

    #[test]
    fn collision_box_update_runs_when_there_is_one() {
        let mut e = engine();
        stub(&mut e, &[COLLISION_BOX_UPDATE]);
        let c = controller(&mut e);
        fn_00c6d4a0(&mut e, c);
        assert!(calls(&e, COLLISION_BOX_UPDATE).is_empty());
        e.set(c, Ctl::spCollisionBox, 0x4400);
        fn_00c6d4a0(&mut e, c);
        assert_eq!(calls(&e, COLLISION_BOX_UPDATE), vec![vec![0x4400]]);
    }

    #[test]
    fn velocity_mod_is_added_or_reset() {
        let mut e = engine();
        let c = controller(&mut e);
        let base = c.addr();
        e.set(c, Ctl::fVelocityTime, 0.5);
        store4(&mut e, base + 0x500, [1.0, 2.0, 3.0, 4.0]);
        store4(&mut e, base + 0x4f0, [10.0; 4]);
        bhk_character_controller_apply_velocity_mod(&mut e, c);
        assert_eq!(load4(&e, base + 0x4f0), [10.5, 11.0, 11.5, 12.0]);
        e.set(c, Ctl::iFlags, 0x1000);
        bhk_character_controller_apply_velocity_mod(&mut e, c);
        assert_eq!(load4(&e, base + 0x4f0), [10.5, 11.0, 11.5, 12.0]);
        store4(&mut e, ZERO_VECTOR, [7.0, 8.0, 9.0, 6.0]);
        for time in [0.0, f32::NAN] {
            e.set(c, Ctl::fVelocityTime, time);
            store4(&mut e, base + 0x500, [1.0; 4]);
            bhk_character_controller_apply_velocity_mod(&mut e, c);
            assert_eq!(load4(&e, base + 0x500), [7.0, 8.0, 9.0, 6.0]);
        }
    }

    /// `setTransformedPos`/`setTransformedInversePos`/`setRotatedDir` stand-in: copies the
    /// input vector to the output.
    fn copy_vector(e: &mut Engine, a: &[u32]) -> Ret {
        copy_words(e, a[0], a[2], 4);
        Ret::default()
    }

    #[test]
    fn raised_point_is_transformed() {
        let mut e = engine();
        e.register(SET_TRANSFORMED_POS, copy_vector);
        let source = e.mem.alloc(0x30);
        e.mem.set_f32(source + 0x10, 1.5);
        store4(&mut e, source + 0x20, [2.0, 3.0, 4.0, 5.0]);
        let out = e.mem.alloc(0x10);
        fn_00c6d620(&mut e, Ptr::new(source), 0x8800, out);
        assert_eq!(load4(&e, out), [2.0, 3.0, 5.5, 5.0]);
        assert_eq!(calls(&e, SET_TRANSFORMED_POS)[0][..2], [out, 0x8800]);
    }

    #[test]
    fn copy_of_0x34_bytes_quiets_the_x87_floats() {
        let mut e = engine();
        let src = filled(&mut e, 13);
        let dst = e.mem.alloc(0x34);
        assert_eq!(
            fn_00c6d670(&mut e, Ptr::new(dst), Ptr::new(src)).addr(),
            dst
        );
        assert_eq!(e.mem.bytes(dst, 0x20), e.mem.bytes(src, 0x20));
        assert_eq!(e.mem.u32(dst + 0x20), 0x7fc0_0009);
        assert_eq!(e.mem.u32(dst + 0x2c), 0x7fc0_000c);
        assert_eq!(e.mem.u32(dst + 0x30), 0x7f80_000d);
    }

    #[test]
    fn value_is_passed_to_phantom_and_box() {
        let mut e = engine();
        stub(&mut e, &[WORLD_OBJECT_PASS]);
        let c = controller(&mut e);
        e.set(c, Ctl::spCollisionBox, 0x4400);
        fn_00c6d770(&mut e, c, 9);
        e.set(c, Ctl::spShapePhantom, 0x3300);
        fn_00c6d770(&mut e, c, 8);
        assert_eq!(
            calls(&e, WORLD_OBJECT_PASS),
            vec![vec![0x4400, 9], vec![0x3300, 8], vec![0x4400, 8]]
        );
    }

    #[test]
    fn ni_object_is_set_on_phantom_and_box() {
        let mut e = engine();
        stub(&mut e, &[SET_AV_OBJECT]);
        let c = controller(&mut e);
        e.set(c, Ctl::spShapePhantom, 0x3300);
        bhk_character_controller_set_ni_object(&mut e, c, 0x77);
        e.set(c, Ctl::spCollisionBox, 0x4400);
        bhk_character_controller_set_ni_object(&mut e, c, 0x78);
        assert_eq!(
            calls(&e, SET_AV_OBJECT),
            vec![
                vec![0x3300, 0x77, 0],
                vec![0x3300, 0x78, 0],
                vec![0x4400, 0x78, 0]
            ]
        );
    }

    #[test]
    fn layer_filter_cases() {
        let mut e = engine();
        assert_eq!(fn_00c6d7e0(&mut e, 0, 0, 0, -1), 0);
        assert_eq!(fn_00c6d7e0(&mut e, 0, 0, 5, -1), 1);
        assert_eq!(fn_00c6d7e0(&mut e, 0, 3, 0, -1), 3);
        let shape = e.mem.alloc(0x10);
        assert_eq!(fn_00c6d7e0(&mut e, shape, 2, 0, -1), 2);
        let hk = e.mem.alloc(0x20);
        e.mem.set_u32(hk + 0x10, 0xe5);
        e.mem.set_u32(shape + 8, hk);
        // No callback: allowed.
        assert_eq!(fn_00c6d7e0(&mut e, shape, 1, 0, -1), 1);
        e.register(0x0900_0100, |_, a| ret(u32::from(a[0] == 5)));
        e.set_global(LAYER_CALLBACK, 0x0900_0100u32);
        assert_eq!(fn_00c6d7e0(&mut e, shape, 1, 0, -1), 0);
        assert_eq!(calls(&e, 0x0900_0100), vec![vec![5]]);
        returns(&mut e, SHAPE_LAYER, 6);
        assert_eq!(fn_00c6d7e0(&mut e, shape, 1, 0, 7), 1);
        assert_eq!(calls(&e, SHAPE_LAYER), vec![vec![hk, 7]]);
    }

    type PropertyEntry = (u32, u32, u32);

    fn property_object(e: &mut Engine, entries: &[PropertyEntry]) -> u32 {
        let object = e.mem.alloc(0x100);
        let data = e.mem.alloc(0x10 * entries.len() as u32 + 0x10);
        for (i, (key, low, high)) in entries.iter().enumerate() {
            let entry = data + 0x10 * i as u32;
            e.mem.set_u32(entry, *key);
            e.mem.set_u32(entry + 8, *low);
            e.mem.set_u32(entry + 0xc, *high);
        }
        e.mem.set_u32(object + 0x78, data);
        e.mem.set_u32(object + 0x7c, entries.len() as u32);
        object
    }

    #[test]
    fn property_lookup_returns_value_or_zero() {
        let mut e = engine();
        let object = property_object(&mut e, &[(1, 0x10, 0x11), (2, 0x20, 0x21)]);
        let out = e.mem.alloc(8);
        let r = hkp_world_object_get_property(&mut e, Ptr::new(object), Ptr::new(out), 2, 0);
        assert_eq!(r.addr(), out);
        assert_eq!((e.mem.u32(out), e.mem.u32(out + 4)), (0x20, 0x21));
        hkp_world_object_get_property(&mut e, Ptr::new(object), Ptr::new(out), 3, 0);
        assert_eq!((e.mem.u32(out), e.mem.u32(out + 4)), (0, 0));
    }

    #[test]
    fn property_presence_on_the_havok_object() {
        let mut e = engine();
        stub(&mut e, &[EMPTY_LOCK]);
        let this = e.mem.alloc(0x10);
        assert!(!fn_00c6d8a0(&mut e, Ptr::new(this), 1));
        assert!(calls(&e, EMPTY_LOCK).is_empty());
        let hk = property_object(&mut e, &[(1, 0, 0), (4, 0, 0)]);
        e.mem.set_u32(this + 8, hk);
        assert!(fn_00c6d8a0(&mut e, Ptr::new(this), 4));
        assert!(!fn_00c6d8a0(&mut e, Ptr::new(this), 5));
        assert_eq!(calls(&e, EMPTY_LOCK), vec![vec![this]; 4]);
    }

    #[test]
    fn property_0x1300_target_word() {
        let mut e = engine();
        let target = e.mem.alloc(0xd0);
        e.mem.set_u32(target + 0xc0, 0xabcd);
        let object = property_object(&mut e, &[(5, 0, 0), (0x1300, target, 0)]);
        assert_eq!(fn_00c6d900(&mut e, object), 0xabcd);
        let other = property_object(&mut e, &[(5, target, 0)]);
        assert_eq!(fn_00c6d900(&mut e, other), 0);
        assert_eq!(fn_00c6d900(&mut e, 0), 0);
    }

    #[test]
    fn shape_by_type_below_two() {
        let mut e = engine();
        let c = controller(&mut e);
        e.mem.set_u32(c.addr() + 0x5a0, 0x10);
        e.mem.set_u32(c.addr() + 0x5a4, 0x20);
        e.mem.set_u32(c.addr() + 0x5a8, 0x30);
        e.mem.set_u32(c.addr() + 0x5ac, 0x40);
        for (kind, expected) in [(-1, 0x10), (0, 0x20), (1, 0x30), (2, 0), (5, 0)] {
            e.set(c, Ctl::eShapeType, kind);
            assert_eq!(bhk_character_controller_get_shape(&mut e, c), expected);
        }
    }

    #[test]
    fn closest_collector_deleting_destructor() {
        let mut e = engine();
        install_allocator(&mut e, 0);
        let this = e.mem.alloc(0x80);
        hkp_closest_cd_point_collector_scalar_deleting_destructor(&mut e, Ptr::new(this), 0);
        assert_eq!(e.mem.u32(this), VTABLE_CD_POINT_COLLECTOR);
        assert!(calls(&e, 0x0900_0f08).is_empty());
        hkp_closest_cd_point_collector_scalar_deleting_destructor(&mut e, Ptr::new(this), 1);
        assert_eq!(calls(&e, 0x0900_0f08)[0][1..], [this, 0x80]);
    }

    #[test]
    fn three_default_points() {
        let mut e = engine();
        store4(&mut e, POINT_DEFAULT, [1.0, 2.0, 3.0, 9.0]);
        let this = e.mem.alloc(0x28);
        e.mem.set_u32(this, 5);
        assert_eq!(fn_00c6d9f0(&mut e, Ptr::new(this)).addr(), this);
        assert_eq!(e.mem.u32(this), 0);
        for copy in 0..3 {
            let at = this + 4 + 12 * copy;
            assert_eq!([0, 4, 8].map(|o| e.mem.f32(at + o)), [1.0, 2.0, 3.0]);
        }
    }

    #[test]
    fn member_tear_down() {
        let mut e = engine();
        stub(&mut e, &[REMOVE_REFERENCE]);
        e.register(0x0900_0004, nothing);
        returns(&mut e, INTERLOCKED_DECREMENT, 0);
        let pointer = with_vtable(&mut e, &[(4, 0x0900_0004)]);
        let this = e.mem.alloc(0xa8);
        e.mem.set_u32(this + 0xa4, 0x4242);
        e.mem.set_u32(this + 0xa0, pointer);
        fn_00c6de10(&mut e, Ptr::new(this));
        assert_eq!(calls(&e, REMOVE_REFERENCE), vec![vec![0x4242]]);
        assert_eq!(e.mem.u32(this + 0xa4), 0);
        assert_eq!(calls(&e, INTERLOCKED_DECREMENT), vec![vec![pointer + 4]]);
        assert_eq!(calls(&e, 0x0900_0004), vec![vec![pointer]]);
        assert_eq!(e.mem.u32(this + 0xa0), pointer);
    }

    #[test]
    fn bs_reference_is_the_property_low_word() {
        let mut e = engine();
        let object = property_object(&mut e, &[(7, 0x7777, 0x8888)]);
        assert_eq!(
            bhk_character_controller_find_bs_reference(&mut e, 7, object),
            0x7777
        );
        assert_eq!(
            bhk_character_controller_find_bs_reference(&mut e, 8, object),
            0
        );
        assert_eq!(bhk_character_controller_find_bs_reference(&mut e, 7, 0), 0);
    }

    #[test]
    fn heavy_or_type_six_bodies_block() {
        let mut e = engine();
        e.register(MOTION_TYPE_TEST, |e, a| {
            ret(u32::from(e.mem.u32(a[0]) == 2))
        });
        e.register(GET_MASS, |e, a| ret_st0(e.mem.f32(a[0] - 0xa0) as f64));
        e.set_global(BLOCKING_MASS, 10.0f32);
        let body = e.mem.alloc(0x100);
        e.mem.set_u8(body + 0xe8, 2);
        e.mem.set_f32(body + 0x40, 50.0);
        assert!(fn_00c6def0(&mut e, body));
        assert_eq!(calls(&e, GET_MASS)[0], vec![body + 0xe0]);
        e.mem.set_f32(body + 0x40, f32::NAN);
        assert!(!fn_00c6def0(&mut e, body));
        e.mem.set_u32(body + 0x2c, 0x86);
        assert!(fn_00c6def0(&mut e, body));
        e.mem.set_u8(body + 0xe8, 3);
        e.mem.set_u32(body + 0x2c, 0x7);
        assert!(!fn_00c6def0(&mut e, body));
        assert!(!fn_00c6def0(&mut e, 0));
    }

    #[test]
    fn cinfo_is_created_once() {
        let mut e = engine();
        install_allocator(&mut e, 0x6000);
        e.register(CINFO_CONSTRUCT, |e, a| {
            let word = e.tls() + SCOPE_WORD;
            assert_eq!(e.mem.u32(word), 0x10);
            ret(a[0] + 1)
        });
        stub(&mut e, &[SET_PROXY_CINFO]);
        let word = e.tls() + SCOPE_WORD;
        e.mem.set_u32(word, 3);
        let this = e.mem.alloc(0x10);
        e.mem.set_u32(this + 8, 1);
        let created = e.mem.alloc(4);
        assert_eq!(
            fn_00c6df60(&mut e, Ptr::new(this), Ptr::new(created)),
            0x6001
        );
        assert_eq!(e.mem.u8(created), 1);
        assert_eq!(calls(&e, 0x0900_0f04)[0][1..], [0xc0]);
        assert_eq!(calls(&e, CINFO_CONSTRUCT), vec![vec![0x6000, 1]]);
        assert_eq!(calls(&e, SET_PROXY_CINFO), vec![vec![this, 0x6001]]);
        assert_eq!(e.mem.u32(word), 3);
        assert_eq!(
            fn_00c6df60(&mut e, Ptr::new(this), Ptr::new(created)),
            0x6001
        );
        assert_eq!(e.mem.u8(created), 0);
        assert_eq!(calls(&e, CINFO_CONSTRUCT).len(), 1);
    }

    #[test]
    fn property_is_added_once_sign_extended() {
        let mut e = engine();
        stub(&mut e, &[EMPTY_LOCK, ADD_PROPERTY]);
        let c = controller(&mut e);
        fn_00c6e050(&mut e, c, 9, -2);
        let phantom = e.mem.alloc(0x10);
        e.set(c, Ctl::spShapePhantom, phantom);
        let hk = property_object(&mut e, &[(3, 0, 0)]);
        e.mem.set_u32(phantom + 8, hk);
        fn_00c6e050(&mut e, c, 9, -2);
        fn_00c6e050(&mut e, c, 3, 5);
        assert_eq!(
            calls(&e, ADD_PROPERTY),
            vec![vec![hk, 9, 0xffff_fffe, 0xffff_ffff]]
        );
    }

    /// A phantom whose scene object's first child (an `NiNode` per `IS_KIND_OF`) has
    /// `children`; returns the controller.
    fn display_scene(e: &mut Engine, children: &[u32]) -> Ptr<Ctl> {
        let node = e.mem.alloc(0x100);
        let list = e.mem.alloc(0x40);
        for (i, child) in children.iter().enumerate() {
            e.mem.set_u32(list + 4 * i as u32, *child);
        }
        e.mem.set_u32(node + 0xa0, list);
        e.mem.set_u16(node + 0xa6, children.len() as u16);
        e.register_double(IS_KIND_OF, move |_, _| ret(node));
        let root = e.mem.alloc(0x100);
        let first = e.mem.alloc(4);
        let root_list = e.mem.alloc(4);
        e.mem.set_u32(root_list, first);
        e.mem.set_u32(root + 0xa0, root_list);
        e.mem.set_u16(root + 0xa6, 1);
        returns(e, 0x0900_020c, root);
        let object = with_vtable(e, &[(0xc, 0x0900_020c)]);
        returns(e, GET_AV_OBJECT, object);
        let c = controller(e);
        e.set(c, Ctl::spShapePhantom, 0x3300);
        c
    }

    #[test]
    fn display_geometry_is_found_by_name() {
        let mut e = engine();
        returns(&mut e, FIXED_STRING_CREATE, 0x5508);
        stub(&mut e, &[INTERLOCKED_DECREMENT]);
        let named = e.mem.alloc(0x10);
        e.mem.set_u32(named + 8, 0x5508);
        let other = e.mem.alloc(0x10);
        let c = display_scene(&mut e, &[0, other, named]);
        assert_eq!(bhk_character_controller_find_display_geometry(&mut e, c), 0);
        assert!(calls(&e, GET_AV_OBJECT).is_empty());
        e.set(c, Ctl::iFlags, 0x8001);
        assert_eq!(
            bhk_character_controller_find_display_geometry(&mut e, c),
            named
        );
        assert_eq!(e.get(c, Ctl::iFlags), 0x8001);
        assert_eq!(calls(&e, GET_AV_OBJECT)[0], vec![0x3300, 0]);
        assert_eq!(calls(&e, IS_KIND_OF)[0][1], NODE_RTTI);
        assert_eq!(calls(&e, FIXED_STRING_CREATE)[0], vec![DISPLAY_NAME]);
        assert_eq!(calls(&e, INTERLOCKED_DECREMENT), vec![vec![0x5500]]);
        // Not found: the flag is cleared; the empty string is not released.
        e.set_global(EMPTY_FIXED_STRING, 0x5508u32);
        e.mem.set_u32(named + 8, 0);
        assert_eq!(bhk_character_controller_find_display_geometry(&mut e, c), 0);
        assert_eq!(e.get(c, Ctl::iFlags), 1);
        assert_eq!(calls(&e, INTERLOCKED_DECREMENT).len(), 1);
    }

    #[test]
    fn radius_change_moves_the_capsule_vertices() {
        let mut e = engine();
        let hk = e.mem.alloc(0x40);
        e.mem.set_f32(hk + 0x10, 0.5);
        store4(&mut e, hk + 0x20, [1.0, 2.0, 3.0, 9.0]);
        store4(&mut e, hk + 0x30, [4.0, 5.0, 6.0, 9.0]);
        let capsule = e.mem.alloc(0x10);
        e.mem.set_u32(capsule + 8, hk);
        e.register_double(IS_KIND_OF, move |_, _| ret(capsule));
        let c = controller(&mut e);
        bhk_character_controller_set_radius(&mut e, c, 0.25);
        assert!(calls(&e, IS_KIND_OF).is_empty());
        e.set(c, Ctl::spShapes, 0x7100);
        bhk_character_controller_set_radius(&mut e, c, 0.25);
        assert_eq!(calls(&e, IS_KIND_OF)[0], vec![0x7100, CAPSULE_SHAPE_RTTI]);
        assert_eq!(load4(&e, hk + 0x20), [1.0, 2.0, 3.25, 0.5]);
        assert_eq!(load4(&e, hk + 0x30), [4.0, 5.0, 5.75, 0.5]);
        assert_eq!(e.mem.f32(hk + 0x10), 0.25);
    }

    #[test]
    fn fall_time_grows_by_the_step() {
        let mut e = engine();
        let c = controller(&mut e);
        e.set(c, Ctl::StepDeltaTime, 0.25);
        e.set(c, Ctl::fFallTime, 1.0);
        fn_00c6e2e0(&mut e, c);
        assert_eq!(e.get(c, Ctl::fFallTime), 1.25);
    }

    #[test]
    fn pitch_and_roll_rotate_the_phantom_transform() {
        let mut e = engine();
        stub(&mut e, &[QUATERNION_PREPARE, ROTATION_SET, EMPTY_LOCK]);
        e.set_global(FLOAT_ONE, 1.0f32);
        e.set_global(FLOAT_NEGATIVE_ZERO, -0.0f32);
        e.register(X87_FIRST_TRIG, |_, _| ret_st0(0.5));
        e.register(X87_SECOND_TRIG, |_, _| ret_st0(0.25));
        e.register(MATRIX3_MUL, |e, a| {
            let n = e.mem.u32(SCRATCH);
            copy_words(e, SCRATCH + 4 + 0x30 * n, a[1], 12);
            e.mem.set_u32(SCRATCH, n + 1);
            Ret::default()
        });
        e.register(PHANTOM_SET_TRANSFORM, |e, a| {
            copy_words(e, SCRATCH + 0x100, a[1], 16);
            Ret::default()
        });
        let hk = e.mem.alloc(0xf0);
        for i in 0..16 {
            e.mem.set_u32(hk + 0xb0 + 4 * i, 0x100 + i);
        }
        let phantom = e.mem.alloc(0x10);
        e.mem.set_u32(phantom + 8, hk);
        let c = controller(&mut e);
        e.set(c, Ctl::spShapePhantom, phantom);
        e.set(c, Ctl::fPitchAngle, 1.0);
        e.set(c, Ctl::fRollAngle, 2.0);
        e.set(c, Ctl::iFlags, 1 << 25);
        let rotation = vec_at(&mut e, [1.0, 2.0, 3.0, 4.0]);
        bhk_character_controller_apply_pitch_and_roll(&mut e, c, Ptr::new(rotation));
        assert_eq!(e.mem.u32(SCRATCH), 1);
        assert_eq!(load4(&e, SCRATCH + 4), [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(load4(&e, SCRATCH + 0x14), [0.0, 0.5, -0.25, 0.0]);
        assert_eq!(load4(&e, SCRATCH + 0x24), [0.0, 0.25, 0.5, 0.0]);
        assert_eq!(calls(&e, X87_FIRST_TRIG)[0], args![1.0f64]);
        let prepared = calls(&e, QUATERNION_PREPARE)[0][0];
        assert_eq!(calls(&e, ROTATION_SET)[0][1], prepared);
        for i in 0..16 {
            assert_eq!(e.mem.u32(SCRATCH + 0x100 + 4 * i), 0x100 + i);
        }
        assert_eq!(calls(&e, PHANTOM_SET_TRANSFORM)[0][0], hk);
        // Roll only.
        e.set(c, Ctl::iFlags, 1 << 26);
        bhk_character_controller_apply_pitch_and_roll(&mut e, c, Ptr::new(rotation));
        assert_eq!(e.mem.u32(SCRATCH), 2);
        assert_eq!(load4(&e, SCRATCH + 0x34), [0.5, 0.0, 0.25, 0.0]);
        assert_eq!(load4(&e, SCRATCH + 0x44), [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(load4(&e, SCRATCH + 0x54), [-0.25, 0.0, 0.5, 0.0]);
        assert_eq!(calls(&e, X87_FIRST_TRIG)[1], args![2.0f64]);
    }

    #[test]
    fn ground_probe_raises_the_point_by_the_hit() {
        let mut e = engine();
        e.set_global(PROBE_DEPTH, 25.0f64);
        e.register(RAY_FILTER_INFO, |e, a| {
            e.mem.set_u32(a[1], 0x55);
            Ret::default()
        });
        e.register(0x0900_00c8, |e, a| {
            let input = a[1];
            copy_words(e, SCRATCH, input, 10);
            let collector = e.mem.u32(input + 0xa4);
            e.mem.set_u32(SCRATCH + 0x40, collector);
            e.mem.set_f32(input + 0xd0, 0.5);
            ret(e.mem.u32(SCRATCH + 0x80))
        });
        let caster = with_vtable(&mut e, &[(0xc8, 0x0900_00c8)]);
        let world = e.mem.alloc(0x360);
        e.mem.set_u32(world + 0x350, caster);
        let phantom = e.mem.alloc(0x10);
        e.mem.set_u32(phantom + 8, world);
        returns(&mut e, GET_SHAPE_PHANTOM, phantom);
        let c = controller(&mut e);
        e.set(c, Ctl::phkObject, 0x5500);
        store4(&mut e, c.addr() + 0x5c0, [1.0, 2.0, 30.0, 4.0]);
        let out = e.mem.alloc(0x10);
        fn_00c6e680(&mut e, c, 1, Ptr::new(out));
        assert_eq!(load4(&e, out), [1.0, 2.0, 5.0, 4.0]);
        assert_eq!(load4(&e, SCRATCH), [1.0, 2.0, 30.0, 4.0]);
        assert_eq!(load4(&e, SCRATCH + 0x10), [1.0, 2.0, 5.0, 4.0]);
        assert_eq!(e.mem.u32(SCRATCH + 0x20), 0);
        assert_eq!(e.mem.u32(SCRATCH + 0x24), 0x55);
        let input = calls(&e, 0x0900_00c8)[0][1];
        assert_eq!(e.mem.u32(SCRATCH + 0x40), input + 0xb0);
        assert_eq!(calls(&e, RAY_FILTER_INFO)[0][0], c.addr());
        e.mem.set_u32(SCRATCH + 0x80, 1);
        fn_00c6e680(&mut e, c, 1, Ptr::new(out));
        assert_eq!(load4(&e, out), [1.0, 2.0, 17.5, 4.0]);
    }

    #[test]
    fn front_back_points_are_placed() {
        let mut e = engine();
        e.set_global(HALF, 0.5f64);
        e.set_global(MINUS_ONE, -1.0f64);
        e.register(SET_TRANSFORMED_POS, copy_vector);
        let shape = e.mem.alloc(0x10);
        e.mem.set_u32(shape + 0xc, 9);
        let phantom = e.mem.alloc(0x20);
        e.mem.set_u32(phantom + 0x10, shape);
        returns(&mut e, GET_SHAPE_PHANTOM, phantom);
        let c = controller(&mut e);
        e.set(c, Ctl::phkObject, 0x5500);
        e.set(c, Ctl::fRadius, 0.5);
        e.set(c, Ctl::fHeight, 2.0);
        e.set(c, Ctl::fStepHeight, 0.25);
        bhk_character_controller_init_front_back(&mut e, c);
        assert!(calls(&e, SET_TRANSFORMED_POS).is_empty());
        e.set(c, Ctl::iFlags, 1);
        bhk_character_controller_init_front_back(&mut e, c);
        let points = |e: &Engine| [0, 1, 2, 3].map(|i| load4(e, c.addr() + 0x5b0 + 0x10 * i));
        assert_eq!(
            points(&e),
            [
                [0.0, 0.5, 0.75, 0.0],
                [0.0, -0.5, 0.75, 0.0],
                [-0.5, 0.0, 0.75, 0.0],
                [0.5, 0.0, 0.75, 0.0]
            ]
        );
        assert_eq!(
            calls(&e, SET_TRANSFORMED_POS)[0][..2],
            [c.addr() + 0x5b0, phantom + 0xb0]
        );
        e.mem.set_u32(shape + 0xc, 8);
        bhk_character_controller_init_front_back(&mut e, c);
        assert_eq!(points(&e)[0], [0.0, 0.5, 0.0, 0.0]);
    }

    #[test]
    fn shape_type_filter_follows_type_ten() {
        let mut e = engine();
        let inner = e.mem.alloc(0x40);
        e.mem.set_u32(inner + 0xc, 0x1e);
        let outer = e.mem.alloc(0x40);
        e.mem.set_u32(outer + 0xc, 10);
        e.mem.set_u32(outer + 0x34, inner);
        assert_eq!(fn_00c6ead0(&mut e, outer), inner);
        e.mem.set_u32(inner + 0xc, 0x20);
        assert_eq!(fn_00c6ead0(&mut e, outer), 0);
        e.mem.set_u32(outer + 0xc, 0x17);
        assert_eq!(fn_00c6ead0(&mut e, outer), outer);
        assert_eq!(fn_00c6ead0(&mut e, 0), 0);
    }

    /// The object callback's world: a body with `mass` (read by the `GET_MASS` double from
    /// the motion's `+0x20`) and `motion`, the impulse `(3,4,0,0)`. Returns the controller,
    /// the body, the event and the impulse.
    fn object_event(e: &mut Engine, mass: f32, motion: u32) -> [u32; 4] {
        e.set_global(MAX_PUSH_MASS, 95.0f32);
        e.set_global(IMPULSE_DEFAULT, 2.0f32);
        e.set_global(IMPULSE_LIGHT, 0.1f32);
        e.set_global(FLOAT_THREE, 3.0f32);
        e.set_global(FLOAT_HALF, 0.5f32);
        e.set_global(HALF, 0.5f64);
        e.set_global(RATIO_LIMIT, 1000.0f64);
        e.set_global(HEIGHT_FRACTION, 0.9f32 as f64);
        e.register(GET_MASS, |e, a| ret_st0(e.mem.f32(a[0] + 0x20) as f64));
        stub(e, &[ADD_REFERENCE, REMOVE_REFERENCE]);
        let body = e.mem.alloc(0x110);
        e.mem.set_f32(body + 0x100, mass);
        e.mem.set_u32(body + 0x2c, motion);
        let event = e.mem.alloc(0x40);
        e.mem.set_u32(event + 0x30, body);
        let impulse = vec_at(e, [3.0, 4.0, 0.0, 0.0]);
        let c = controller(e);
        [c.addr(), body, event, impulse]
    }

    fn object_callback(e: &mut Engine, c: u32, event: u32, impulse: u32) {
        let listener = Ptr::new(c + LISTENER);
        bhk_character_controller_object_interaction_callback(
            e,
            listener,
            0,
            Ptr::new(event),
            Ptr::new(impulse),
        );
    }

    #[test]
    fn object_interaction_scales_the_impulse() {
        let mut e = engine();
        let [c, body, event, impulse] = object_event(&mut e, 10.0, 0x81);
        let ctl = Ptr::<Ctl>::new(c);
        e.set(ctl, Ctl::spBumpedBody, 0x9900);
        object_callback(&mut e, c, event, impulse);
        assert_eq!(load4(&e, impulse), [6.0, 8.0, 0.0, 0.0]);
        assert_eq!(e.get(ctl, Ctl::fBumpedForce), 10.0);
        assert_eq!(e.get(ctl, Ctl::spBumpedBody), body);
        assert_eq!(calls(&e, ADD_REFERENCE), vec![vec![body]]);
        assert_eq!(calls(&e, REMOVE_REFERENCE), vec![vec![0x9900]]);
        // A weaker push leaves the bumped body alone.
        store4(&mut e, impulse, [0.3, 0.4, 0.0, 0.0]);
        object_callback(&mut e, c, event, impulse);
        assert_eq!(calls(&e, ADD_REFERENCE).len(), 1);
        // Type 10 above half the mass limit: 0.1.
        let [c, _, event, impulse] = object_event(&mut e, 50.0, 10);
        object_callback(&mut e, c, event, impulse);
        let force = e.get(Ptr::<Ctl>::new(c), Ctl::fBumpedForce);
        assert_eq!(force, (5.0f64 * 0.1f32 as f64) as f32);
        // Out of the mass range: zero impulse, the force is the length.
        let [c, _, event, impulse] = object_event(&mut e, 95.0, 1);
        object_callback(&mut e, c, event, impulse);
        assert_eq!(load4(&e, impulse), [0.0; 4]);
        assert_eq!(e.get(Ptr::<Ctl>::new(c), Ctl::fBumpedForce), 5.0);
    }

    #[test]
    fn object_interaction_from_above_follows_the_velocity() {
        let mut e = engine();
        let [c, _, event, impulse] = object_event(&mut e, 0.001, 1);
        let ctl = Ptr::<Ctl>::new(c);
        e.register(GET_POSITION, |e, a| {
            store4(e, a[1], [0.0; 4]);
            Ret::default()
        });
        store4(&mut e, event, [0.0, 0.0, 2.0, 0.0]);
        e.set(ctl, Ctl::fHeight, 1.0);
        e.set(ctl, Ctl::iFlags, 8);
        store4(&mut e, c + 0x4f0, [0.0, 2.0, 0.0, 0.0]);
        object_callback(&mut e, c, event, impulse);
        // Ratio 5000 > 1000: factor 2 * 1000 / 5000.
        let ratio = (5.0f64 / 0.001f32 as f64) as f32;
        let factor = (1000.0 / ratio as f64 * 2.0) as f32;
        let inverse = inverse_length(4.0, 0.0, 0.5, 3.0);
        let y = factor * (inverse * 2.0 * 5.0);
        assert_eq!(load4(&e, impulse), [0.0, y, 0.0, 0.0]);
        assert_eq!(calls(&e, GET_POSITION)[0][0], c);
    }

    fn character_world(e: &mut Engine) {
        e.set_global(FLOAT_THREE, 3.0f32);
        e.set_global(FLOAT_HALF, 0.5f32);
        e.set_global(HALF, 0.5f64);
        e.set_global(ROT_MOD_DECAY, 0.05f32 as f64);
        e.set_global(PUSH_SCALE, -0.25f64);
        e.set_global(LARGE_SIZE_BONUS, 20.0f64);
        e.set_global(MODIFIER_TIME, 0.01f32);
        e.set_global(ROT_MOD_TIME, 0.2f32);
        e.set_global(MINUS_ONE, -1.0f64);
        stub(
            e,
            &[ADD_REFERENCE, REMOVE_REFERENCE, HKP_GET_LINEAR_VELOCITY],
        );
        e.register(COPY_POINT, |e, a| {
            copy_words(e, a[0], a[1], 4);
            ret(a[0])
        });
        e.register(SET_VELOCITY_MODIFIER, |e, a| {
            copy_words(e, SCRATCH, a[1], 4);
            Ret::default()
        });
        e.register(X87_SQRT, |_, a| {
            ret_st0(f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32).sqrt())
        });
        e.register(FLOAT_MIN, |_, a| {
            let (low, high) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
            ret_st0(if high > low { low } else { high } as f64)
        });
        e.register(FLOAT_MAX, |_, a| {
            let (low, high) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
            ret_st0(if high < low { low } else { high } as f64)
        });
    }

    #[test]
    fn character_interaction_pushes_and_turns_the_other() {
        let mut e = engine();
        character_world(&mut e);
        let velocity = vec_at(&mut e, [0.0, -2.0, 7.0, 0.0]);
        returns(&mut e, GET_HK_LINEAR_VELOCITY, velocity);
        let position = vec_at(&mut e, [0.0, 0.0, 9.0, 0.0]);
        returns(&mut e, GET_HK_POSITION, position);
        let c = controller(&mut e);
        let other = controller(&mut e);
        e.set(c, Ctl::iPriority, 2);
        e.set(other, Ctl::iPriority, 1);
        e.set(other, Ctl::iFlags, 1);
        e.set(other, Ctl::fHeight, 4.0);
        store4(&mut e, other.addr() + 0x4d0, [1.0, 0.0, 3.0, 0.0]);
        let proxy = e.mem.alloc(0xd0);
        e.mem.set_u32(proxy + 0xc0, other.addr());
        let event = e.mem.alloc(0x20);
        store4(&mut e, event, [1.0, 0.0, 0.0, 0.0]);
        store4(&mut e, event + 0x10, [0.0, 1.0, 5.0, 0.0]);
        e.set(c, Ctl::spBumpedCharProxy, 0x9900);
        let listener = Ptr::new(c.addr() + LISTENER);
        let (proxy_ptr, event_ptr) = (Ptr::new(proxy), Ptr::new(event));
        bhk_character_controller_character_interaction_callback(
            &mut e, listener, 0, proxy_ptr, event_ptr,
        );
        assert_eq!(e.get(c, Ctl::spBumpedCharProxy), proxy);
        assert_eq!(calls(&e, REMOVE_REFERENCE), vec![vec![0x9900]]);
        assert_eq!(calls(&e, HKP_GET_LINEAR_VELOCITY), vec![vec![proxy]]);
        // Push: -0.25 * speed 2 along the normalized normal.
        let unit = inverse_length(1.0, 0.0, 0.5, 3.0);
        let push = [0.0, -0.5f32 * unit, 0.0, 0.0];
        assert_eq!(load4(&e, other.addr() + 0x580), push);
        assert_eq!(load4(&e, SCRATCH), push);
        assert_eq!(e.get(other, Ctl::iPushCount), 1);
        assert_eq!(calls(&e, SET_VELOCITY_MODIFIER)[0][2], 0.01f32.to_bits());
        assert_eq!(calls(&e, GET_HK_POSITION), vec![vec![other.addr()]]);
        // Turn: facing about 1, reach about 1 / 2 - 0.05, doubled.
        let turn = e.get(other, Ctl::fRotMod);
        assert!((turn - 0.9).abs() < 1e-4, "{turn}");
        assert_eq!(e.get(other, Ctl::fRotModTime), 0.2);
        // A higher-priority other is left alone.
        e.set(other, Ctl::iPriority, 3);
        bhk_character_controller_character_interaction_callback(
            &mut e, listener, 0, proxy_ptr, event_ptr,
        );
        assert_eq!(e.get(other, Ctl::iPushCount), 1);
    }

    #[test]
    fn inside_check_casts_outward_then_inward() {
        let mut e = engine();
        e.set_global(QUARTER, 0.25f64);
        e.set_global(THIRTY_TWO, 32.0f64);
        e.register(GET_RADIUS, |_, _| ret_st0(0.5));
        e.register(SET_ROTATED_DIR, copy_vector);
        e.register(SET_TRANSFORMED_INVERSE_POS, copy_vector);
        e.register(GET_POSITION, |e, a| {
            store4(e, a[1], [0.0; 4]);
            Ret::default()
        });
        // Each cast logs its start z and hits on the cast number held at SCRATCH + 4.
        e.register(0x0900_0020, |e, a| {
            let n = e.mem.u32(SCRATCH) + 1;
            e.mem.set_u32(SCRATCH, n);
            let from = e.mem.f32(a[2] + 8);
            e.mem.set_f32(SCRATCH + 8 * n, from);
            let hit = n == e.mem.u32(SCRATCH + 4);
            e.mem.set_f32(a[3] + 0x10, if hit { 0.5 } else { 1.0 });
            Ret::default()
        });
        let shape = with_vtable(&mut e, &[(0x20, 0x0900_0020)]);
        e.mem.set_u32(shape + 0xc, 8);
        let skipped = e.mem.alloc(0x20);
        e.mem.set_u32(skipped + 0x1c, 0x84);
        e.mem.set_u32(skipped, shape);
        let tested = e.mem.alloc(0x20);
        e.mem.set_u32(tested + 0x1c, 0x81);
        e.mem.set_u32(tested, shape);
        e.mem.set_u32(tested + 8, 0x7700);
        let entries = e.mem.alloc(0x20);
        e.mem.set_u32(entries + 4, skipped);
        e.mem.set_u32(entries + 0xc, tested);
        let hk = e.mem.alloc(0x170);
        e.mem.set_u32(hk + 0x160, entries);
        e.mem.set_u32(hk + 0x164, 2);
        let phantom = e.mem.alloc(0x10);
        e.mem.set_u32(phantom + 8, hk);
        let c = controller(&mut e);
        e.set(c, Ctl::spShapePhantom, phantom);
        e.set(c, Ctl::fActorHeight, 4.0);
        // No hit: six casts, three outward (from the position) and three inward.
        assert!(!bhk_character_controller_check_inside_of_object(&mut e, c));
        assert_eq!(e.mem.u32(SCRATCH), 6);
        let froms = [1, 2, 3, 4, 5, 6].map(|n| e.mem.f32(SCRATCH + 8 * n));
        assert_eq!(froms, [1.0, 2.0, 3.0, 1.0, 2.0, 3.0]);
        assert_eq!(calls(&e, SET_ROTATED_DIR)[0][1], hk + 0xb0);
        assert_eq!(calls(&e, SET_TRANSFORMED_INVERSE_POS)[0][1], 0x7700);
        // An outward hit skips the inward casts; an inward hit returns true.
        e.mem.set_u32(SCRATCH, 0);
        e.mem.set_u32(SCRATCH + 4, 2);
        assert!(!bhk_character_controller_check_inside_of_object(&mut e, c));
        assert_eq!(e.mem.u32(SCRATCH), 2);
        e.mem.set_u32(SCRATCH, 0);
        e.mem.set_u32(SCRATCH + 4, 5);
        assert!(bhk_character_controller_check_inside_of_object(&mut e, c));
        assert_eq!(e.mem.u32(SCRATCH), 5);
    }

    #[test]
    fn display_geometry_is_built_under_the_node() {
        let mut e = engine();
        e.set_global(EMPTY_FIXED_STRING, 0x5508u32);
        stub(&mut e, &[SET_NAME, INTERLOCKED_DECREMENT]);
        returns(&mut e, FIXED_STRING_CREATE, 0x5508);
        returns(&mut e, HEAP_ALLOCATE, 0x6000);
        let display = e.mem.alloc(0x100);
        returns(&mut e, NODE_CONSTRUCT, display);
        e.register(COPY_POINT, |e, a| {
            copy_words(e, a[0], a[1], 4);
            ret(a[0])
        });
        e.register(0x0900_00dc, nothing);
        e.register(0x0900_00d4, nothing);
        let node = with_vtable(&mut e, &[(0xdc, 0x0900_00dc)]);
        e.mem.set_f32(node + 0x98, -2.0);
        let shape = with_vtable(&mut e, &[(0xd4, 0x0900_00d4)]);
        let c = controller(&mut e);
        e.set(c, Ctl::spShapes, shape);
        store4(&mut e, c.addr() + 0x570, [2.0, 4.0, 6.0, 0.0]);
        let word = e.tls() + SCOPE_WORD;
        e.mem.set_u32(word, 5);
        bhk_character_controller_build_display_geometry(&mut e, c, node);
        assert_eq!(calls(&e, HEAP_ALLOCATE), vec![vec![NODE_SIZE]]);
        assert_eq!(calls(&e, NODE_CONSTRUCT), vec![vec![0x6000, 0]]);
        assert_eq!(calls(&e, 0x0900_00dc), vec![vec![node, display, 1]]);
        assert_eq!(calls(&e, SET_NAME)[0][0], display);
        assert!(calls(&e, INTERLOCKED_DECREMENT).is_empty());
        let placed = [0x58, 0x5c, 0x60, 0x64].map(|o| e.mem.f32(display + o));
        assert_eq!(placed, [-1.0, -2.0, -3.0, 0.5]);
        assert_eq!(
            calls(&e, 0x0900_00d4),
            vec![vec![shape, display, DISPLAY_ARGUMENT]]
        );
        assert_eq!(e.get(c, Ctl::iFlags), 0x8000);
        assert_eq!(e.mem.u32(word), 5);
        // Without a current shape nothing is built.
        e.call_log = Some(vec![]);
        e.set(c, Ctl::iFlags, 0);
        e.set(c, Ctl::eShapeType, 2);
        bhk_character_controller_build_display_geometry(&mut e, c, node);
        assert!(calls(&e, HEAP_ALLOCATE).is_empty());
        assert_eq!(e.get(c, Ctl::iFlags), 0);
    }
}
