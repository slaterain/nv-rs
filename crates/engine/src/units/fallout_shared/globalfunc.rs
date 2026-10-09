//! `fallout shared/globalfunc.cpp` (Xbox PDB source unit), subsystem `fallout shared`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit is a grab bag of free functions from `004b1230` on (122
//! functions). Session 1 (b0015) covers the first 40, `004b1230` to
//! `004b51e0`: the number tests for strings, the angle helpers, the two
//! lookup functions for form type bytes, a segment against rectangle
//! clipper, the pick helpers, the debug shape builders (`MakeTriPoint`,
//! `MakeQuadBox`, `MakeTriangle`, `MakeRectangle` and a one segment line
//! builder), the small vector and matrix products, `NiMatrix3`'s inverse,
//! the geometry snapping function `004b3e60` with its helper `004b4910`,
//! and the `bhkMouseSpringAction` constructor chain. The next session
//! continues at `004b5220`.
//!
//! Conventions this file uses, so the next session finds them:
//!
//! - x87: the game computes in extended precision and stores `float`
//!   results. The translations compute in `f64` and round to `f32` where
//!   the code stores a `float` (docs/ENGINE_CRATE.md). An x87 compare
//!   followed by `TEST AH, mask` and a conditional jump is written as the
//!   ordinary comparison it implements, including what it does for NaN
//!   (`!(a > b)` is not `a <= b`), hence the `neg_cmp_op_on_partial_ord`
//!   allowance below.
//! - A `float` or `double` the code loads from the exe's data (a constant
//!   the decompiler folds into a literal) is read from memory at its
//!   address (the `const`s below, with the value they hold in the exe).
//! - `00416870` is `NiPoint3::NiPoint3(this, x, y, z)` (returns `this`),
//!   `006815c0` returns `this` (the empty `NiPoint3` constructor the array
//!   constructors call), `00439ef0` is `this - other` and `00439e90`
//!   `this + other` on `NiPoint3` (`(this, out, other)`, they return
//!   `out`), `00457990` the length of a vector (ST0), `004a0c10` the
//!   unitize. They live in other units and are called by address.
//! - The game sometimes pushes words its callee never reads; they stay on
//!   the stack until a later call reads them (`005495f0`, `0049ec60`,
//!   `006815c0`). The translations pass each call only its own words, and
//!   the doc of the function says where the next call picks them up.
//! - The compiler's exception-unwinding frames and security cookies are
//!   not translated.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

#[allow(unused_imports)]
use crate::prelude::*;
use crate::types::{NiAVObject, NiPoint3};

// ---------------------------------------------------------------------
// Constants of the exe's data read by the code (value in the exe).
// ---------------------------------------------------------------------

/// `1.0` (`double`).
const ONE: u32 = 0x0101_2070;
/// `-1.0` (`float`).
const NEGATIVE_ONE: u32 = 0x0101_2054;
/// `FLT_MAX` (`float`), `0x7f7fffff`.
const FLOAT_MAX: u32 = 0x0101_6970;
/// `4.712389` (`float`), three quarters of a turn.
const THREE_QUARTER_TURN: u32 = 0x0101_ff34;
/// `1.5707964` (`float`), a quarter turn.
const QUARTER_TURN: u32 = 0x0101_ff38;
/// `3.1415927410125732` (`double`), half a turn.
const HALF_TURN: u32 = 0x0101_ff40;
/// `6.2831854820251465` (`double`).
const FULL_TURN: u32 = 0x0101_ff48;
/// `6.2831855` (`float`).
const FULL_TURN_FLOAT: u32 = 0x0101_ff50;
/// `-3.1415927410125732` (`double`).
const NEGATIVE_HALF_TURN: u32 = 0x0101_ff58;
/// `1.4142135381698608` (`double`), the square root of two.
const SQUARE_ROOT_OF_TWO: u32 = 0x0101_ff98;
/// `9.99999974752427e-7` (`double`): a determinant at most this large in
/// magnitude makes `Inverse_ov2` fail.
const SINGULAR_DETERMINANT: u32 = 0x0101_7cf8;
/// `0.001` (`float`): the tolerance within which the snapping function
/// treats two distances as equal.
const SNAP_TOLERANCE: u32 = 0x0101_7d00;
/// `10.0` (`float`): how far the snapping function's debug mode moves a
/// point along its vector.
const DEBUG_OFFSET: u32 = 0x0101_7b78;
/// The geometry the snapping function last ran on (`011c6320`).
const LAST_SNAPPED_OBJECT: u32 = 0x011c_6320;

/// The CRT's one-argument `double` function that `004b1460` calls
/// (`00ec9010`); the angle formulas that use it show it is the arc
/// tangent (its body was not read).
const CRT_ARC_TANGENT: u32 = 0x00ec_9010;
/// The CRT `fmod(double, double)` (`00ec9130`).
const CRT_FMOD: u32 = 0x00ec_9130;

// ---------------------------------------------------------------------
// Callees in other units.
// ---------------------------------------------------------------------

/// `operator new(size)` and `operator delete(block)` (cdecl).
const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// `__vec_ctor(block, element size, count, constructor)`: calls
/// `constructor(element)` on each element, advancing by the size.
const VECTOR_CONSTRUCT: u32 = 0x0040_1050;
/// The Gamebryo allocators (cdecl, size): objects and arrays.
const NI_OPERATOR_NEW: u32 = 0x00aa_13e0;
const NI_ARRAY_NEW: u32 = 0x00aa_1070;
/// `NiPoint3::NiPoint3(this, x, y, z)`; returns `this`.
const POINT3_CONSTRUCT: u32 = 0x0041_6870;
/// The empty `NiPoint3` constructor (returns `this`) and the 16-byte
/// element constructor (`004a7800`) the array constructors are given.
const POINT3_EMPTY_CONSTRUCTOR: u32 = 0x0068_15c0;
const COLOR_ELEMENT_CONSTRUCTOR: u32 = 0x004a_7800;
/// `NiTriShape::NiTriShape(this, ...)` (Xbox PDB), as the callers here pass
/// it: `(vertex count, vertices, 0, colors, 0, 0, 0, triangle count,
/// triangles)`.
const NI_TRI_SHAPE_CONSTRUCT: u32 = 0x00a7_4410;
/// `NiLines::NiLines(this, count, vertices, colors, 0, 0, 0, flags)`
/// (Xbox PDB).
const NI_LINES_CONSTRUCT: u32 = 0x00a7_46e0;
/// `BSShaderNoLightingProperty::BSShaderNoLightingProperty(this)`
/// (Xbox PDB).
const NO_LIGHTING_PROPERTY_CONSTRUCT: u32 = 0x00b6_fc90;
/// `NiAVObject::AttachProperty(this, property)` (Xbox PDB).
const ATTACH_PROPERTY: u32 = 0x0043_9410;
/// `BSShaderManager::PrepareObject(object, 0, 0)` (cdecl, Xbox PDB).
const PREPARE_OBJECT: u32 = 0x00b5_7e30;
/// `NiAVObject::GetWorldBound` (Xbox PDB): a pointer to the four floats of
/// the bound (centre, then radius).
const GET_WORLD_BOUND: u32 = 0x0043_d450;
/// `float` at `this + 0xc` (ST0): the radius of a bound.
const BOUND_RADIUS: u32 = 0x0084_d030;
/// Pick helpers: `00824060(this)` and `00456610(this)` (bool results),
/// `00705fc0(this, node)` (selects what to pick against) and
/// `NiPick::PickObjects(this, origin, direction, find first)` (Xbox PDB;
/// bool).
const PICK_BLOCKED_A: u32 = 0x0082_4060;
const PICK_BLOCKED_B: u32 = 0x0045_6610;
const PICK_SET_ROOT: u32 = 0x0070_5fc0;
const PICK_OBJECTS: u32 = 0x00e9_8e20;
/// The pick state object that `004b2780` clears (`011f426c`).
const PICK_STATE: u32 = 0x011f_426c;
/// Child container accessors: the count (`0043b480(this)`) and child `i`
/// (`0043b4a0(this, i)`).
const CHILD_COUNT: u32 = 0x0043_b480;
const CHILD_AT: u32 = 0x0043_b4a0;
/// Vector helpers (`NiPoint3`): `this - other` and `this + other` as
/// `(this, out, other)`, the length (ST0), the unitize, `out = vector *
/// scale` as `(out, scale, vector)` (cdecl, returns `out`) and
/// `this += other`.
const POINT3_SUBTRACT: u32 = 0x0043_9ef0;
const POINT3_ADD: u32 = 0x0043_9e90;
const POINT3_LENGTH: u32 = 0x0045_7990;
const POINT3_UNITIZE: u32 = 0x004a_0c10;
const POINT3_SCALED: u32 = 0x004a_3760;
const POINT3_ADD_ASSIGN: u32 = 0x0063_c8a0;
/// `00408820(float)` (ST0): the absolute value of a float.
const FLOAT_ABSOLUTE_VALUE: u32 = 0x0040_8820;
/// `0040eb10(a, b, tolerance)` (cdecl): whether two floats differ by at
/// most the tolerance.
const FLOATS_NEAR: u32 = 0x0040_eb10;
/// The log function `005b5e40(format, ...)`.
const LOG_MESSAGE: u32 = 0x005b_5e40;
/// `"FORMS: Invalid FormID sent to FormIDToString().\r\n"`.
const INVALID_FORM_ID_MESSAGE: u32 = 0x0101_ff60;

// Callees of `004b3e60` and `004b4910`.
/// `(this)`: the first point array of a geometry object (12-byte
/// elements); `004a8030(this)` the second one; `00456650(this)` the point
/// count (a word); `00461130(this)` the address of the world transform
/// (`this + 0x68`).
const POINTS_FIRST: u32 = 0x0049_ec60;
const POINTS_SECOND: u32 = 0x004a_8030;
const POINT_COUNT: u32 = 0x0045_6650;
const WORLD_TRANSFORM: u32 = 0x0046_1130;
/// `004b4a10(count, source points, destination, transform)` (cdecl): a
/// thunk to the function pointer at `011aa050` that transforms `count`
/// points.
const TRANSFORM_POINTS: u32 = 0x004b_4a10;
/// The `NiTransform` constructor (`00476a80(this)`),
/// `NiTransform::InvertNonUniform` (`004b4880(this, out)`, Xbox PDB name)
/// and `NiMatrix3::operator*` (`0043f8d0(this, out, other)`, engine map).
const TRANSFORM_CONSTRUCT: u32 = 0x0047_6a80;
const TRANSFORM_INVERT_NON_UNIFORM: u32 = 0x004b_4880;
const MATRIX_PRODUCT: u32 = 0x0043_f8d0;
/// The map the snapping function builds (a hash map of 0x25 buckets keyed
/// by a point index, 12-byte values): constructor `(this, 0x25)`,
/// `Find(this, key, out value)` (bool), `SetAt(this, key, value words 0,
/// 1, 2)`, the iterator start `(this)` and next `(this, &iterator, &key,
/// &value)`, and the destructor `(this)`.
const POINT_MAP_CONSTRUCT: u32 = 0x004b_9950;
const POINT_MAP_SET_AT: u32 = 0x004b_9a20;
const POINT_MAP_FIND: u32 = 0x004b_9b20;
const POINT_MAP_FIRST: u32 = 0x004b_9ba0;
const POINT_MAP_NEXT: u32 = 0x004b_9bf0;
const POINT_MAP_DESTRUCT: u32 = 0x004b_9cd0;
/// `005495f0(this)` is `NiPointer` dereference at `+0xb8`; the call site
/// pushes the flag word `3` that the next call reads:
/// `00a67090(that object, 3)` is `NiGeometryData::MarkAsChanged` (Xbox
/// PDB).
const GEOMETRY_DATA: u32 = 0x0054_95f0;
const MARK_AS_CHANGED: u32 = 0x00a6_7090;

// Callees of the `bhk` constructors and destructors.
/// `bhkRefObject`'s constructor `(this)`, the destructors `(this)` of
/// `bhkSerializable` (`00c877b0`), `bhkAction` (`00c87970`) and
/// `bhkUnaryAction` (`00c86720`), `bhkMouseSpringAction::Init(this, arg)`
/// (`00c87260`) and `operator delete(block, size)` (`00aa1460`, cdecl).
const BHK_REF_OBJECT_CONSTRUCT: u32 = 0x00c8_7c30;
const BHK_SERIALIZABLE_DESTRUCT: u32 = 0x00c8_77b0;
const BHK_ACTION_DESTRUCT: u32 = 0x00c8_7970;
const BHK_UNARY_ACTION_DESTRUCT: u32 = 0x00c8_6720;
const BHK_MOUSE_SPRING_ACTION_INIT: u32 = 0x00c8_7260;
const SIZED_DELETE: u32 = 0x00aa_1460;
/// The vtables the constructors install and the `ms_uiObjects` counters
/// (Xbox PDB) they increment.
const BHK_SERIALIZABLE_VTABLE: u32 = 0x0102_023c;
const BHK_ACTION_VTABLE: u32 = 0x0102_016c;
const BHK_UNARY_ACTION_VTABLE: u32 = 0x0102_009c;
const BHK_MOUSE_SPRING_ACTION_VTABLE: u32 = 0x0101_ffcc;
const BHK_ACTION_OBJECT_COUNT: u32 = 0x0126_8154;
const BHK_UNARY_ACTION_OBJECT_COUNT: u32 = 0x0126_8118;
const BHK_MOUSE_SPRING_ACTION_OBJECT_COUNT: u32 = 0x0126_8124;
/// The `NiRTTI` objects the `GetRTTI` functions return. The one of
/// `bhkMouseSpringAction` (`0126812c`) belongs to `004b5210`, which the
/// next session translates.
const BHK_SERIALIZABLE_RTTI: u32 = 0x0126_8134;
const BHK_ACTION_RTTI: u32 = 0x0126_8158;
const BHK_UNARY_ACTION_RTTI: u32 = 0x0126_811c;

// Tables of the form type functions.
/// The table `004b1b60` reads when `004b1680` gives an index: name
/// pointers, 4 bytes each.
const FORM_NAME_BY_INDEX: u32 = 0x0118_a2d8;
/// The table of 12-byte entries whose first word is a name pointer,
/// indexed by form type byte (0 to 0x78).
const FORM_NAME_BY_TYPE: u32 = 0x0118_7004;

layout! {
    /// `bhkSerializable` (Xbox PDB), 0x10 bytes; `bhkAction`,
    /// `bhkUnaryAction` and `bhkMouseSpringAction` add no fields.
    pub struct BhkSerializable: 0x10 {
        /// `pInfo` (Xbox PDB): `hkSerializableCinfo*`.
        0x0C pInfo: u32,
    }
}

// ---------------------------------------------------------------------
// String number tests.
// ---------------------------------------------------------------------

fn is_digit(c: i8) -> bool {
    (0x30..=0x39).contains(&(c as i32))
}

// Translated from 004b1230 (decompiled, FalloutNV.exe 1.4.0.525)
/// `IsAnInteger` (Xbox PDB): whether the string is an optional `-` and
/// digits (a lone `-` or an empty string is not).
pub fn is_an_integer(e: &mut Engine, text: Ptr) -> bool {
    let at = |e: &Engine, i: u32| e.mem.i8(text.addr().wrapping_add(i));
    let first = at(e, 0);
    if !is_digit(first) && first != b'-' as i8 {
        return false;
    }
    if at(e, 1) == 0 && first == b'-' as i8 {
        return false;
    }
    let mut i = 1;
    loop {
        let c = at(e, i);
        if c == 0 {
            return true;
        }
        if !is_digit(c) {
            return false;
        }
        i += 1;
    }
}

// Translated from 004b12d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `IsAFloat` (Xbox PDB): whether the string is digits with at most one
/// `.` and an optional leading `-` (a lone `-` or `.`, or an empty string,
/// is not).
pub fn is_a_float(e: &mut Engine, text: Ptr) -> bool {
    let at = |e: &Engine, i: u32| e.mem.i8(text.addr().wrapping_add(i));
    let first = at(e, 0);
    if first == 0 {
        return false;
    }
    if !is_digit(first) && first != b'-' as i8 && first != b'.' as i8 {
        return false;
    }
    if at(e, 1) == 0 && (first == b'-' as i8 || first == b'.' as i8) {
        return false;
    }
    let mut seen_point = first == b'.' as i8;
    let mut i = 1;
    loop {
        let c = at(e, i);
        if c == 0 {
            return true;
        }
        if !is_digit(c) && (c != b'.' as i8 || seen_point) {
            return false;
        }
        if c == b'.' as i8 {
            seen_point = true;
        }
        i += 1;
    }
}

// ---------------------------------------------------------------------
// Angles.
// ---------------------------------------------------------------------

// Translated from 004b13c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `GetZAngleFromVector` (Xbox PDB): the angle of the vector's `x`, `y`
/// measured from the `y` axis (`atan(x / y)`, plus half a turn when
/// `y < 0`; a quarter or three quarters of a turn when `y == 0`), through
/// [`clamp_angle`] into `[0, 2 pi)`.
pub fn get_z_angle_from_vector(e: &mut Engine, vector: Ptr<NiPoint3>) -> f32 {
    let angle = get_unclamped_z_angle_from_vector(e, vector);
    clamp_angle(e, angle)
}

// Translated from 004b1460 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `float` form of the CRT function `00ec9010` (the engine map's
/// `logf` name is wrong: the angle code that calls it shows the arc
/// tangent).
pub fn fn_004b1460(e: &mut Engine, value: f32) -> f32 {
    e.call(CRT_ARC_TANGENT, &args![value as f64]).f64() as f32
}

// Translated from 004b1480 (decompiled, FalloutNV.exe 1.4.0.525)
/// `ClampAngle` (Xbox PDB): brings an angle into `[0, 2 pi)`: adds a full
/// turn to a negative angle (then takes the remainder, `fmodf`) and takes
/// the remainder of an angle at or above a full turn.
pub fn clamp_angle(e: &mut Engine, angle: f32) -> f32 {
    let full_turn: f64 = e.global(FULL_TURN);
    let full_turn_float: f32 = e.global(FULL_TURN_FLOAT);
    if angle < 0.0 {
        let shifted = (angle as f64 + full_turn) as f32;
        fn_004b1500(e, shifted, full_turn_float)
    } else if angle as f64 >= full_turn {
        fn_004b1500(e, angle, full_turn_float)
    } else {
        angle
    }
}

// Translated from 004b1500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `fmodf(a, b)`: forwards to [`fn_004b1520`].
pub fn fn_004b1500(e: &mut Engine, a: f32, b: f32) -> f32 {
    fn_004b1520(e, a, b)
}

// Translated from 004b1520 (decompiled, FalloutNV.exe 1.4.0.525)
/// `fmodf(a, b)`: the CRT `fmod` (`00ec9130`) on the two floats widened to
/// `double`, stored as a `float`.
pub fn fn_004b1520(e: &mut Engine, a: f32, b: f32) -> f32 {
    e.call(CRT_FMOD, &args![a as f64, b as f64]).f64() as f32
}

// Translated from 004b1550 (decompiled, FalloutNV.exe 1.4.0.525)
/// `GetUnclampedZAngleFromVector` (Xbox PDB): [`get_z_angle_from_vector`]
/// without the final clamp.
pub fn get_unclamped_z_angle_from_vector(e: &mut Engine, vector: Ptr<NiPoint3>) -> f32 {
    let x = e.get(vector, NiPoint3::x);
    let y = e.get(vector, NiPoint3::y);
    if y == 0.0 {
        // `x <= 0` (or unordered) takes three quarters of a turn.
        let turn = if !(x > 0.0) {
            THREE_QUARTER_TURN
        } else {
            QUARTER_TURN
        };
        return e.global(turn);
    }
    let ratio = (x as f64 / y as f64) as f32;
    let mut angle = fn_004b1460(e, ratio);
    if y < 0.0 {
        let half_turn: f64 = e.global(HALF_TURN);
        angle = (angle as f64 + half_turn) as f32;
    }
    angle
}

// Translated from 004b15e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The shortest turn from angle `from` to angle `to`: returns its
/// magnitude (at most half a turn) and stores its direction in the float
/// at `direction`: `1.0`, `-1.0`, or `0.0` when the angles are equal.
pub fn fn_004b15e0(e: &mut Engine, from: f32, to: f32, direction: Ptr) -> f32 {
    let half_turn: f64 = e.global(HALF_TURN);
    let full_turn: f64 = e.global(FULL_TURN);
    let negative_half_turn: f64 = e.global(NEGATIVE_HALF_TURN);
    let negative_one: f32 = e.global(NEGATIVE_ONE);
    let mut difference = (to as f64 - from as f64) as f32;
    e.mem.set_f32(direction.addr(), 0.0);
    if difference != 0.0 {
        e.mem.set_f32(direction.addr(), 1.0);
        if !(difference < 0.0) {
            if difference as f64 > half_turn {
                e.mem.set_f32(direction.addr(), negative_one);
                difference = (full_turn - difference as f64) as f32;
            }
        } else if difference as f64 <= negative_half_turn {
            difference = (difference as f64 + full_turn) as f32;
        } else {
            e.mem.set_f32(direction.addr(), negative_one);
        }
    }
    difference
}

// ---------------------------------------------------------------------
// Form type lookups.
// ---------------------------------------------------------------------

/// `004b1680`'s answers for the byte values 4 to 0x75 (the results of its
/// byte table and jump table), `-1` where it has none.
const FORM_TYPE_INDEX: [i8; 0x72] = [
    36, 38, -1, 52, 56, 55, -1, 54, 53, 24, 67, -1, 70, -1, 25, 17, 16, 0, 34, 44, 1, 2, -1, 3, 4,
    5, 6, 7, 8, 72, 43, 45, 9, 10, 22, 23, 11, 21, 12, 13, 14, 15, 20, 18, 31, 57, 66, 33, 19, 58,
    59, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 49, -1, 50, 26, 27, 28, 29,
    30, 32, -1, 35, 37, -1, -1, 39, 40, 41, 42, -1, -1, 46, -1, 51, 47, 48, -1, 64, -1, -1, -1, -1,
    71, 73, 74, -1, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86,
];

// Translated from 004b1680 (decompiled, FalloutNV.exe 1.4.0.525)
/// Maps a form type byte to an index (0 to 86) into the name table at
/// `0118a2d8`, or `-1` for the bytes that table does not cover (below 4,
/// above 0x75, or the gaps). A `switch` through a byte table and a jump
/// table in the game.
pub fn fn_004b1680(_e: &mut Engine, form_type: u8) -> i32 {
    match form_type.checked_sub(4) {
        Some(offset) => FORM_TYPE_INDEX
            .get(offset as usize)
            .map_or(-1, |&index| index as i32),
        None => -1,
    }
}

/// The form type bytes for which `004b1b60` reads a single name slot, with
/// the slot's address (`01188f98` to `01189034`): the pointer stored there
/// is the name.
const FORM_NAME_SLOTS: [(u8, u32); 40] = [
    (0x03, 0x0118_8fbc),
    (0x06, 0x0118_8fb4),
    (0x07, 0x0118_8f9c),
    (0x08, 0x0118_8fac),
    (0x09, 0x0118_8fd8),
    (0x0a, 0x0118_8fdc),
    (0x0b, 0x0118_8fe0),
    (0x0c, 0x0118_8f98),
    (0x0d, 0x0118_8fb0),
    (0x0e, 0x0118_8ffc),
    (0x0f, 0x0118_8fd0),
    (0x10, 0x0118_8fc0),
    (0x11, 0x0118_8fa0),
    (0x31, 0x0118_8ff0),
    (0x35, 0x0118_8fcc),
    (0x36, 0x0118_8fc8),
    (0x37, 0x0118_8fb8),
    (0x39, 0x0118_8fa8),
    (0x3a, 0x0118_8fe4),
    (0x41, 0x0118_8fc4),
    (0x45, 0x0118_8fa4),
    (0x47, 0x0118_8fd4),
    (0x48, 0x0118_8fe8),
    (0x49, 0x0118_8fec),
    (0x53, 0x0118_8ff4),
    (0x54, 0x0118_8ff8),
    (0x66, 0x0118_9000),
    (0x68, 0x0118_9004),
    (0x6a, 0x0118_9008),
    (0x6b, 0x0118_900c),
    (0x6c, 0x0118_9010),
    (0x6d, 0x0118_9014),
    (0x6e, 0x0118_9018),
    (0x6f, 0x0118_901c),
    (0x70, 0x0118_9020),
    (0x71, 0x0118_9024),
    (0x72, 0x0118_9028),
    (0x73, 0x0118_902c),
    (0x74, 0x0118_9030),
    (0x75, 0x0118_9034),
];

// Translated from 004b1b60 (decompiled, FalloutNV.exe 1.4.0.525)
/// The name string (its address) of a form type byte; the log message
/// below names the function `FormIDToString`. Through `004b1680`'s index
/// into the table at `0118a2d8` when it has one, else from the slot
/// listed for the type in `FORM_NAME_SLOTS`, else from the 12-byte table
/// at `01187004` (a type of 0x79 or more logs an error and uses entry 0).
pub fn fn_004b1b60(e: &mut Engine, form_type: u8) -> u32 {
    let index = fn_004b1680(e, form_type);
    if index >= 0 {
        return e.global(FORM_NAME_BY_INDEX + 4 * index as u32);
    }
    if let Some(&(_, slot)) = FORM_NAME_SLOTS.iter().find(|&&(t, _)| t == form_type) {
        return e.global(slot);
    }
    let mut form_type = form_type as u32;
    if form_type >= 0x79 {
        let name: u32 = e.global(FORM_NAME_BY_TYPE + 12 * form_type);
        e.call(LOG_MESSAGE, &args![INVALID_FORM_ID_MESSAGE, name]);
        form_type = 0;
    }
    e.global(FORM_NAME_BY_TYPE + 12 * form_type)
}

// ---------------------------------------------------------------------
// Segment against rectangle.
// ---------------------------------------------------------------------

fn copy_words(e: &mut Engine, dst: u32, src: u32, words: u32) {
    for i in 0..words {
        let word = e.mem.u32(src + 4 * i);
        e.mem.set_u32(dst + 4 * i, word);
    }
}

/// Builds `NiPoint3(x, y, z)` through the game's constructor in `scratch`
/// (12 bytes) and copies the three words it returns to `dst`.
fn store_point(e: &mut Engine, scratch: u32, dst: u32, x: f32, y: f32, z: f32) {
    let built = e.call(POINT3_CONSTRUCT, &args![scratch, x, y, z]).u32();
    copy_words(e, dst, built, 3);
}

/// Records an intersection point (`z` is 0) in the first free one of the
/// two result points.
fn record_hit(e: &mut Engine, scratch: u32, slots: [u32; 2], hits: &mut usize, x: f32, y: f32) {
    store_point(e, scratch, slots[*hits], x, y, 0.0);
    *hits += 1;
}

// Translated from 004b1e80 (decompiled, FalloutNV.exe 1.4.0.525)
/// Clips the segment `from` to `to` (`NiPoint3`s; only `x` and `y` are
/// read) against the integer rectangle `rect` (`x min`, `y max`, `x max`,
/// `y min`, with `x min < x max` and `y min < y max`) and stores one
/// intersection point in `out`; returns whether there is one.
///
/// - Both ends beside the same edge, or both strictly inside: `false`,
///   `out` untouched.
/// - A segment of one point gives that point (all three words of `from`)
///   when it lies on or in the rectangle; a point strictly inside is
///   rejected by the previous rule.
/// - A vertical segment gives `(x, y max, 0)` when `to.y < from.y`, else
///   `(x, y min, 0)`; a horizontal one `(x max, y, 0)` when `to.x < from.x`,
///   else `(x min, y, 0)`, provided the line crosses the rectangle.
/// - Otherwise the line `y = m x + b` through the segment is intersected
///   with the left, top, right and bottom edges in that order, keeping the
///   first two hits. One hit is the result. With two: when `from` is
///   strictly inside the rectangle the unit vector along the segment and
///   the unit vector to the first hit are added, and the second hit is
///   chosen when the sum is at most one long, else the first; when `from`
///   is not strictly inside, the hit nearer `from` is chosen (the second
///   on a tie).
pub fn fn_004b1e80(e: &mut Engine, rect: Ptr, from: Ptr, to: Ptr, out: Ptr) -> bool {
    // The game's locals: two points, a scratch point and three vectors.
    let scratch = e.mem.alloc(0x60);
    let found = segment_against_rect(e, scratch, rect, from, to, out);
    e.mem.free(scratch);
    found
}

fn segment_against_rect(
    e: &mut Engine,
    scratch: u32,
    rect: Ptr,
    from: Ptr,
    to: Ptr,
    out: Ptr,
) -> bool {
    let x_min = e.mem.i32(rect.addr());
    let y_max = e.mem.i32(rect.addr() + 4);
    let x_max = e.mem.i32(rect.addr() + 8);
    let y_min = e.mem.i32(rect.addr() + 0xc);
    if x_min >= x_max || y_max <= y_min {
        return false;
    }
    let (x_min_f, y_max_f, x_max_f, y_min_f) =
        (x_min as f64, y_max as f64, x_max as f64, y_min as f64);
    let x1 = e.mem.f32(from.addr()) as f64;
    let y1 = e.mem.f32(from.addr() + 4) as f64;
    let x2 = e.mem.f32(to.addr()) as f64;
    let y2 = e.mem.f32(to.addr() + 4) as f64;

    // Both ends beside the same edge: no intersection.
    if (x1 < x_min_f && x2 < x_min_f)
        || (y1 < y_min_f && y2 < y_min_f)
        || (x1 > x_max_f && x2 > x_max_f)
        || (y1 > y_max_f && y2 > y_max_f)
    {
        return false;
    }
    // Both ends strictly inside: nothing to clip.
    let strictly_inside = |x: f64, y: f64| x > x_min_f && x < x_max_f && y > y_min_f && y < y_max_f;
    if strictly_inside(x1, y1) && strictly_inside(x2, y2) {
        return false;
    }

    let (x_equal, y_equal) = (x1 == x2, y1 == y2);
    if x_equal && y_equal {
        // A single point.
        if x1 <= x_max_f && x1 >= x_min_f && y1 <= y_max_f && y1 >= y_min_f {
            copy_words(e, out.addr(), from.addr(), 3);
            return true;
        }
        return false;
    }
    if x_equal {
        // Vertical.
        if x1 <= x_max_f && x1 >= x_min_f {
            let y = if y2 < y1 { y_max } else { y_min };
            store_point(e, scratch, out.addr(), x1 as f32, y as f32, 0.0);
            return true;
        }
        return false;
    }
    if y_equal {
        // Horizontal.
        if y1 <= y_max_f && y1 >= y_min_f {
            let x = if x2 < x1 { x_max } else { x_min };
            store_point(e, scratch, out.addr(), x as f32, y1 as f32, 0.0);
            return true;
        }
        return false;
    }

    // The general case: the line through the segment against the edges.
    let (first_hit, second_hit, rest) = (scratch, scratch + 0xc, scratch + 0x18);
    let slots = [first_hit, second_hit];
    let float_max: f32 = e.global(FLOAT_MAX);
    e.call(
        POINT3_CONSTRUCT,
        &args![first_hit, float_max, float_max, float_max],
    );
    e.call(
        POINT3_CONSTRUCT,
        &args![second_hit, float_max, float_max, float_max],
    );
    let slope = ((y2 - y1) / (x2 - x1)) as f32;
    let intercept = (y2 - x2 * slope as f64) as f32;
    let (m, b) = (slope as f64, intercept as f64);
    let mut hits = 0;
    // Left edge.
    let y_left = (x_min_f * m + b) as f32 as f64;
    if hits < 2 && y_left <= y_max_f && y_left >= y_min_f {
        record_hit(e, rest, slots, &mut hits, x_min as f32, y_left as f32);
    }
    // Top edge.
    let x_top = ((y_max_f - b) / m) as f32 as f64;
    if hits < 2 && x_top <= x_max_f && x_top >= x_min_f {
        record_hit(e, rest, slots, &mut hits, x_top as f32, y_max as f32);
    }
    // Right edge.
    let y_right = (x_max_f * m + b) as f32 as f64;
    if hits < 2 && y_right <= y_max_f && y_right >= y_min_f {
        record_hit(e, rest, slots, &mut hits, x_max as f32, y_right as f32);
    }
    // Bottom edge.
    let x_bottom = ((y_min_f - b) / m) as f32 as f64;
    if hits < 2 && x_bottom <= x_max_f && x_bottom >= x_min_f {
        record_hit(e, rest, slots, &mut hits, x_bottom as f32, y_min as f32);
    }
    if hits == 0 {
        return false;
    }
    if hits != 2 {
        copy_words(e, out.addr(), first_hit, 3);
        return true;
    }
    let from_not_strictly_inside = x1 <= x_min_f || x1 >= x_max_f || y1 <= y_min_f || y1 >= y_max_f;
    let chosen = if from_not_strictly_inside {
        let to_first = e.mem.alloc(12);
        let to_second = e.mem.alloc(12);
        e.call(POINT3_SUBTRACT, &args![first_hit, to_first, from]);
        let distance_first = e.call(POINT3_LENGTH, &args![to_first]).f64();
        e.call(POINT3_SUBTRACT, &args![second_hit, to_second, from]);
        let distance_second = e.call(POINT3_LENGTH, &args![to_second]).f64();
        e.mem.free(to_first);
        e.mem.free(to_second);
        if distance_second <= distance_first {
            second_hit
        } else {
            first_hit
        }
    } else {
        let vectors = e.mem.alloc(0x24);
        let (along_segment, to_first, sum) = (vectors, vectors + 0xc, vectors + 0x18);
        e.call(POINT3_SUBTRACT, &args![to, along_segment, from]);
        e.call(POINT3_SUBTRACT, &args![first_hit, to_first, from]);
        e.mem.set_f32(along_segment + 8, 0.0);
        e.mem.set_f32(to_first + 8, 0.0);
        e.call(POINT3_UNITIZE, &args![along_segment]);
        e.call(POINT3_UNITIZE, &args![to_first]);
        e.call(POINT3_ADD, &args![along_segment, sum, to_first]);
        let length = e.call(POINT3_LENGTH, &args![sum]).f64();
        e.mem.free(vectors);
        let one: f64 = e.global(ONE);
        if length <= one {
            second_hit
        } else {
            first_hit
        }
    };
    copy_words(e, out.addr(), chosen, 3);
    true
}

// ---------------------------------------------------------------------
// Pick helpers.
// ---------------------------------------------------------------------

// Translated from 004b2780 (decompiled, FalloutNV.exe 1.4.0.525)
/// Picks the ray `origin` / `direction` against `node` within `distance`
/// through [`fn_004b2800`]; with `reset`, first clears the pick state
/// (`00705fc0(object, 0)` and `PickObjects(object, 011f426c, 011f426c,
/// 0)`). False without doing anything when `object` or `node` is null or
/// `distance` is zero.
pub fn fn_004b2780(
    e: &mut Engine,
    object: Ptr,
    origin: Ptr,
    direction: Ptr,
    distance: f32,
    node: Ptr,
    reset: bool,
) -> bool {
    if object.is_null() || node.is_null() || distance == 0.0 {
        return false;
    }
    if reset {
        e.call(PICK_SET_ROOT, &args![object, 0u32]);
        e.call(PICK_OBJECTS, &args![object, PICK_STATE, PICK_STATE, 0u32]);
    }
    fn_004b2800(e, object, origin, direction, distance, node)
}

// Translated from 004b2800 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the ray hits `node` or one of its descendants within
/// `distance`. A node is skipped when both pick checks (`00824060(object)`
/// and `00456610(node)`) answer true, or when it has no bound
/// ([`fn_004b2960`]), or when the distance from its bound's centre to the
/// ray origin is not less than `radius + distance`. A node whose virtual
/// function `+0xc` gives a child container recurses over the children; any
/// other is picked with `PickObjects(object, origin, direction, 1)` after
/// selecting it with `00705fc0`.
pub fn fn_004b2800(
    e: &mut Engine,
    object: Ptr,
    origin: Ptr,
    direction: Ptr,
    distance: f32,
    node: Ptr,
) -> bool {
    let mut hit = false;
    if object.is_null() || node.is_null() {
        return false;
    }
    if e.call(PICK_BLOCKED_A, &args![object]).bool() && e.call(PICK_BLOCKED_B, &args![node]).bool()
    {
        return false;
    }
    if !fn_004b2960(e, node) {
        return false;
    }
    // The bound is copied (centre and radius) to a local; the vector from
    // its centre to the ray origin is built next to it. `006815c0` returns
    // its argument, which `00439ef0` then takes as `this`.
    let bound = e.call(GET_WORLD_BOUND, &args![node]).u32();
    let locals = e.mem.alloc(0x1c);
    let centre_to_origin = locals + 0x10;
    copy_words(e, locals, bound, 4);
    let centre = e.call(POINT3_EMPTY_CONSTRUCTOR, &args![locals]).u32();
    e.call(POINT3_SUBTRACT, &args![centre, centre_to_origin, origin]);
    let length = e.call(POINT3_LENGTH, &args![centre_to_origin]).f64();
    let radius = e.call(BOUND_RADIUS, &args![locals]).f64();
    e.mem.free(locals);
    if radius + distance as f64 > length {
        let children = e.vcall(node.addr(), 0xc, &[]).u32();
        if children != 0 {
            let count = e.call(CHILD_COUNT, &args![children]).i32();
            for i in 0..count {
                let child = e.call(CHILD_AT, &args![children, i]).u32();
                if fn_004b2800(e, object, origin, direction, distance, Ptr::new(child)) {
                    hit = true;
                }
            }
        } else {
            e.call(PICK_SET_ROOT, &args![object, node]);
            if e.call(PICK_OBJECTS, &args![object, origin, direction, 1u32])
                .bool()
            {
                hit = true;
            }
        }
    }
    hit
}

// Translated from 004b2960 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether the node has a world bound (`NiAVObject::m_kWorldBound`, the
/// pointer at `+0x20`) with a non-zero radius.
pub fn fn_004b2960(e: &mut Engine, node: Ptr) -> bool {
    let bound = e.get(node.cast::<NiAVObject>(), NiAVObject::m_kWorldBound);
    if bound == 0 {
        return false;
    }
    e.call(BOUND_RADIUS, &args![bound]).f32() != 0.0
}

// ---------------------------------------------------------------------
// Debug shape builders.
// ---------------------------------------------------------------------

/// An array of `count` elements of `element_size` bytes from `operator
/// new`, each constructed through the vector constructor with
/// `constructor`; null when the allocation fails.
fn new_array(e: &mut Engine, element_size: u32, count: u32, constructor: u32) -> u32 {
    let block = e.call(OPERATOR_NEW, &args![element_size * count]).u32();
    if block != 0 {
        e.call(
            VECTOR_CONSTRUCT,
            &args![block, element_size, count, constructor],
        );
    }
    block
}

/// `count` copies of the four-word colour at `color` into `colors`.
fn fill_colors(e: &mut Engine, colors: u32, color: u32, count: u32) {
    for i in 0..count {
        copy_words(e, colors + 16 * i, color, 4);
    }
}

/// The triangle index array of a debug shape: from the Gamebryo array
/// allocator, filled with the `u16` indices.
fn new_triangle_indices(e: &mut Engine, indices: &[u16]) -> u32 {
    let block = e.call(NI_ARRAY_NEW, &args![2 * indices.len() as u32]).u32();
    for (i, &index) in indices.iter().enumerate() {
        e.mem.set_u16(block + 2 * i as u32, index);
    }
    block
}

/// A new `NiTriShape` (0xc4 bytes) over the arrays; null when the
/// allocation fails.
fn new_tri_shape(
    e: &mut Engine,
    vertex_count: u32,
    vertices: u32,
    colors: u32,
    triangle_count: u32,
    triangles: u32,
) -> u32 {
    let block = e.call(NI_OPERATOR_NEW, &args![0xc4u32]).u32();
    if block == 0 {
        return 0;
    }
    e.call(
        NI_TRI_SHAPE_CONSTRUCT,
        &args![
            block,
            vertex_count,
            vertices,
            0u32,
            colors,
            0u32,
            0u32,
            0u32,
            triangle_count,
            triangles
        ],
    )
    .u32()
}

/// A new `NiLines` (0xc4 bytes) over the arrays; null when the allocation
/// fails.
fn new_lines(e: &mut Engine, count: u32, vertices: u32, colors: u32, flags: u32) -> u32 {
    let block = e.call(NI_OPERATOR_NEW, &args![0xc4u32]).u32();
    if block == 0 {
        return 0;
    }
    e.call(
        NI_LINES_CONSTRUCT,
        &args![block, count, vertices, colors, 0u32, 0u32, 0u32, flags],
    )
    .u32()
}

/// With `add_property`, gives the shape a new `BSShaderNoLightingProperty`
/// and prepares it for rendering; returns the shape.
fn finish_shape(e: &mut Engine, shape: u32, add_property: bool) -> u32 {
    if add_property {
        let block = e.call(NI_OPERATOR_NEW, &args![0x80u32]).u32();
        let property = if block == 0 {
            0
        } else {
            e.call(NO_LIGHTING_PROPERTY_CONSTRUCT, &args![block]).u32()
        };
        e.call(ATTACH_PROPERTY, &args![shape, property]);
        e.call(PREPARE_OBJECT, &args![shape, 0u32, 0u32]);
    }
    shape
}

/// `MakeTriPoint`'s triangle indices (8 triangles over 6 vertices).
const TRI_POINT_INDICES: [u16; 24] = [
    0, 2, 1, 0, 3, 2, 0, 4, 3, 0, 1, 4, 1, 2, 5, 2, 3, 5, 3, 4, 5, 4, 1, 5,
];
/// `MakeQuadBox`'s indices (24 triangles over 8 vertices).
const QUAD_BOX_INDICES: [u16; 72] = [
    0, 2, 1, 0, 3, 2, 0, 7, 3, 0, 4, 7, 1, 4, 0, 1, 5, 4, 2, 5, 1, 2, 6, 5, 3, 6, 2, 3, 7, 6, 4, 7,
    6, 4, 6, 5, 0, 1, 2, 0, 2, 3, 0, 3, 7, 0, 7, 4, 1, 0, 4, 1, 4, 5, 2, 1, 5, 2, 5, 6, 3, 2, 6, 3,
    6, 7, 4, 6, 7, 4, 5, 6,
];
/// `MakeTriangle`'s indices (both faces of one triangle).
const TRIANGLE_INDICES: [u16; 6] = [0, 2, 1, 0, 1, 2];

// Translated from 004b29b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MakeTriPoint` (Xbox PDB): a `NiTriShape` of 6 vertices (`(0, 0, +-radius
/// * sqrt 2)` and the four corners `(+-radius, +-radius, 0)`) and 8
/// triangles, every vertex coloured with the `NiColorA` at `color`; with
/// `add_property` also given a no-lighting property.
pub fn make_tri_point(e: &mut Engine, radius: f32, color: Ptr, add_property: bool) -> u32 {
    let vertices = new_array(e, 12, 6, POINT3_EMPTY_CONSTRUCTOR);
    let scratch = e.mem.alloc(12);
    let square_root_of_two: f64 = e.global(SQUARE_ROOT_OF_TWO);
    let apex = (radius as f64 * square_root_of_two) as f32;
    store_point(e, scratch, vertices, 0.0, 0.0, apex);
    store_point(e, scratch, vertices + 12, -radius, radius, 0.0);
    store_point(e, scratch, vertices + 24, radius, radius, 0.0);
    store_point(e, scratch, vertices + 36, radius, -radius, 0.0);
    store_point(e, scratch, vertices + 48, -radius, -radius, 0.0);
    let apex_below = (-(radius as f64) * square_root_of_two) as f32;
    store_point(e, scratch, vertices + 60, 0.0, 0.0, apex_below);
    e.mem.free(scratch);
    let colors = new_array(e, 16, 6, COLOR_ELEMENT_CONSTRUCTOR);
    fill_colors(e, colors, color.addr(), 6);
    let triangles = new_triangle_indices(e, &TRI_POINT_INDICES);
    let shape = new_tri_shape(e, 6, vertices, colors, 8, triangles);
    finish_shape(e, shape, add_property)
}

// Translated from 004b2eb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MakeQuadBox` (Xbox PDB): a `NiTriShape` of 8 vertices (copied from the
/// eight `NiPoint3`s `corner_0` to `corner_7`) and 24 triangles, every
/// vertex coloured with the `NiColorA` at `color`.
#[allow(clippy::too_many_arguments)]
pub fn make_quad_box(
    e: &mut Engine,
    corner_0: Ptr,
    corner_1: Ptr,
    corner_2: Ptr,
    corner_3: Ptr,
    corner_4: Ptr,
    corner_5: Ptr,
    corner_6: Ptr,
    corner_7: Ptr,
    color: Ptr,
    add_property: bool,
) -> u32 {
    let vertices = new_array(e, 12, 8, POINT3_EMPTY_CONSTRUCTOR);
    let corners = [
        corner_0, corner_1, corner_2, corner_3, corner_4, corner_5, corner_6, corner_7,
    ];
    for (i, corner) in corners.iter().enumerate() {
        copy_words(e, vertices + 12 * i as u32, corner.addr(), 3);
    }
    let colors = new_array(e, 16, 8, COLOR_ELEMENT_CONSTRUCTOR);
    fill_colors(e, colors, color.addr(), 8);
    let triangles = new_triangle_indices(e, &QUAD_BOX_INDICES);
    let shape = new_tri_shape(e, 8, vertices, colors, 0x18, triangles);
    finish_shape(e, shape, add_property)
}

// Translated from 004b3570 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MakeTriangle` (Xbox PDB): a `NiTriShape` of one triangle (3 vertices
/// passed by value as 9 floats; the index array holds both faces),
/// coloured with the `NiColorA` at `color`.
#[allow(clippy::too_many_arguments)]
pub fn make_triangle(
    e: &mut Engine,
    x0: f32,
    y0: f32,
    z0: f32,
    x1: f32,
    y1: f32,
    z1: f32,
    x2: f32,
    y2: f32,
    z2: f32,
    color: Ptr,
    add_property: bool,
) -> u32 {
    let vertices = new_array(e, 12, 3, POINT3_EMPTY_CONSTRUCTOR);
    for (i, value) in [x0, y0, z0, x1, y1, z1, x2, y2, z2].iter().enumerate() {
        e.mem.set_f32(vertices + 4 * i as u32, *value);
    }
    let colors = new_array(e, 16, 3, COLOR_ELEMENT_CONSTRUCTOR);
    fill_colors(e, colors, color.addr(), 3);
    let triangles = new_triangle_indices(e, &TRIANGLE_INDICES);
    let shape = new_tri_shape(e, 3, vertices, colors, 2, triangles);
    finish_shape(e, shape, add_property)
}

// Translated from 004b3800 (decompiled, FalloutNV.exe 1.4.0.525)
/// The cross product `this x other` of two `NiPoint3`s, built into `out`
/// (`NiPoint3::operator%`); returns `out`.
pub fn fn_004b3800(e: &mut Engine, this: Ptr<NiPoint3>, out: Ptr, other: Ptr<NiPoint3>) -> Ptr {
    let (ax, ay, az) = (
        e.get(this, NiPoint3::x) as f64,
        e.get(this, NiPoint3::y) as f64,
        e.get(this, NiPoint3::z) as f64,
    );
    let (bx, by, bz) = (
        e.get(other, NiPoint3::x) as f64,
        e.get(other, NiPoint3::y) as f64,
        e.get(other, NiPoint3::z) as f64,
    );
    let z = (ax * by - ay * bx) as f32;
    let y = (az * bx - ax * bz) as f32;
    let x = (ay * bz - az * by) as f32;
    e.call(POINT3_CONSTRUCT, &args![out, x, y, z]);
    out
}

// Translated from 004b3890 (decompiled, FalloutNV.exe 1.4.0.525)
/// A `NiLines` of one segment: vertices copied from the `NiPoint3`s `start`
/// and `end`, colours from the `NiColorA`s `start_color` and `end_color`,
/// flag bytes `[1, 0]` (the segment connects the two vertices); optionally
/// given a no-lighting property.
pub fn fn_004b3890(
    e: &mut Engine,
    start: Ptr,
    start_color: Ptr,
    end: Ptr,
    end_color: Ptr,
    add_property: bool,
) -> u32 {
    let vertices = new_array(e, 12, 2, POINT3_EMPTY_CONSTRUCTOR);
    copy_words(e, vertices, start.addr(), 3);
    copy_words(e, vertices + 12, end.addr(), 3);
    let colors = new_array(e, 16, 2, COLOR_ELEMENT_CONSTRUCTOR);
    copy_words(e, colors, start_color.addr(), 4);
    copy_words(e, colors + 16, end_color.addr(), 4);
    let flags = e.call(OPERATOR_NEW, &args![2u32]).u32();
    e.mem.set_u8(flags, 1);
    e.mem.set_u8(flags + 1, 0);
    let lines = new_lines(e, 2, vertices, colors, flags);
    finish_shape(e, lines, add_property)
}

// Translated from 004b3ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Linear interpolation for `(a, b, t0, t1, t)`: `a + (b - a) * (t - t0) /
/// (t1 - t0)`.
pub fn fn_004b3ab0(_e: &mut Engine, a: f32, b: f32, t0: f32, t1: f32, t: f32) -> f32 {
    let ratio = (t as f64 - t0 as f64) / (t1 as f64 - t0 as f64);
    (ratio * (b as f64 - a as f64) + a as f64) as f32
}

// Translated from 004b3ae0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiPoint3 * NiMatrix3` (the vector as a row of the nine floats by rows):
/// built into `out`, which it returns. `(out, vector, matrix)`, cdecl.
pub fn fn_004b3ae0(e: &mut Engine, out: Ptr, vector: Ptr, matrix: Ptr) -> Ptr {
    let v = |e: &Engine, i: u32| e.mem.f32(vector.addr() + 4 * i) as f64;
    let m = |e: &Engine, i: u32| e.mem.f32(matrix.addr() + 4 * i) as f64;
    let (v0, v1, v2) = (v(e, 0), v(e, 1), v(e, 2));
    let z = (v0 * m(e, 2) + v1 * m(e, 5) + v2 * m(e, 8)) as f32;
    let y = (v0 * m(e, 1) + v1 * m(e, 4) + v2 * m(e, 7)) as f32;
    let x = (v0 * m(e, 0) + v1 * m(e, 3) + v2 * m(e, 6)) as f32;
    e.call(POINT3_CONSTRUCT, &args![out, x, y, z]);
    out
}

// Translated from 004b3b90 (decompiled, FalloutNV.exe 1.4.0.525)
/// `MakeRectangle` (Xbox PDB): a `NiLines` outlining the square
/// `(+-radius, +-radius, 0)` (4 vertices, each coloured with the
/// `NiColorA` at `color`, every flag byte 1).
pub fn make_rectangle(e: &mut Engine, radius: f32, color: Ptr, add_property: bool) -> u32 {
    let vertices = new_array(e, 12, 4, POINT3_EMPTY_CONSTRUCTOR);
    let colors = new_array(e, 16, 4, COLOR_ELEMENT_CONSTRUCTOR);
    let flags = e.call(OPERATOR_NEW, &args![4u32]).u32();
    let scratch = e.mem.alloc(12);
    store_point(e, scratch, vertices, -radius, -radius, 0.0);
    store_point(e, scratch, vertices + 12, -radius, radius, 0.0);
    store_point(e, scratch, vertices + 24, radius, radius, 0.0);
    store_point(e, scratch, vertices + 36, radius, -radius, 0.0);
    e.mem.free(scratch);
    for i in 0..4 {
        copy_words(e, colors + 16 * i, color.addr(), 4);
        e.mem.set_u8(flags + i, 1);
    }
    let lines = new_lines(e, 4, vertices, colors, flags);
    finish_shape(e, lines, add_property)
}

// ---------------------------------------------------------------------
// The snapping function, matrices and points.
// ---------------------------------------------------------------------

// Translated from 004b3e60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Matches the points of geometry `second` to those of geometry `first`
/// and copies `first`'s vectors over `second`'s.
///
/// Both objects' first point arrays are taken to world space
/// ([`fn_004b4910`]). For each point of `second` the nearest point of
/// `first` closer than `threshold` (any distance when `threshold < 0`) is
/// found, and a map from the `first` point's index to (the `second` index,
/// the `second` count, the distance) keeps the nearest `second` point per
/// `first` point: a farther one never replaces it, one within `0.001`
/// (`0040eb10`) replaces it but keeps the earlier `second` index as the
/// entry's second word. Each pair then sets the `second` vector (second
/// array, `004a8030`) to `first`'s vector rotated by the relative rotation
/// (the inverse of `second`'s world rotation times `first`'s,
/// [`fn_004b4500`]) and unitized; when the entry's second word is a valid
/// `second` index it is copied there too. With `debug`, the points
/// (`0049ec60` arrays) are moved by 10 times their vectors as well
/// (`first`'s only when it is not the geometry of the previous run), and
/// the geometry data of `first` is marked changed too. Returns whether
/// any pair was recorded; `second`'s data (`MarkAsChanged(3)`) is marked
/// when so.
///
/// Which array is positions and which normals is not confirmed from the
/// exe, so they are called `points` (`0049ec60`) and `vectors`
/// (`004a8030`). The inverse `NiTransform` the game builds with `004b4880`
/// is not used afterwards but the call is kept; a running minimum distance
/// nobody reads is left out. The map's destructor (`004b9cd0`) runs on
/// every exit.
pub fn fn_004b3e60(
    e: &mut Engine,
    first: Ptr,
    second: Ptr,
    threshold: f32,
    _unused_3: u32,
    debug: bool,
) -> bool {
    let map = e.mem.alloc(0x10);
    e.call(POINT_MAP_CONSTRUCT, &args![map, 0x25u32]);
    // The game's locals: a rotation matrix, a transform and a point
    // (constructors that only return `this`, or leave the memory as is),
    // then the scratch for the products and vectors below.
    let scratch = e.mem.alloc(0x100);
    e.call(POINT3_EMPTY_CONSTRUCTOR, &args![scratch + 0x7c]);
    e.call(TRANSFORM_CONSTRUCT, &args![scratch]);
    e.call(POINT3_EMPTY_CONSTRUCTOR, &args![scratch + 0xf0]);
    let changed = snap_geometry(e, map, scratch, first, second, threshold, debug);
    e.mem.free(scratch);
    e.call(POINT_MAP_DESTRUCT, &args![map]);
    e.mem.free(map);
    changed
}

fn snap_geometry(
    e: &mut Engine,
    map: u32,
    scratch: u32,
    first: Ptr,
    second: Ptr,
    threshold: f32,
    debug: bool,
) -> bool {
    if first.is_null() || second.is_null() || first == second {
        return false;
    }
    let first_points = e.call(POINTS_FIRST, &args![first]).u32();
    let first_vectors = e.call(POINTS_SECOND, &args![first]).u32();
    let first_count = e.call(POINT_COUNT, &args![first]).u16() as u32;
    if first_points == 0 || first_vectors == 0 || first_count == 0 {
        return false;
    }
    let second_points = e.call(POINTS_FIRST, &args![second]).u32();
    let second_vectors = e.call(POINTS_SECOND, &args![second]).u32();
    let second_count = e.call(POINT_COUNT, &args![second]).u16() as u32;
    if second_points == 0 || second_vectors == 0 || second_count == 0 {
        return false;
    }
    let first_world = fn_004b4910(e, first);
    if first_world == 0 {
        return false;
    }
    let second_world = fn_004b4910(e, second);
    if second_world == 0 {
        e.call(OPERATOR_DELETE, &args![first_world]);
        return false;
    }

    // Locals in the scratch block: the transform (0x34 bytes at +0), two
    // matrices and the rotation, then the vectors.
    let (transform, inverse, product, matrix) =
        (scratch, scratch + 0x34, scratch + 0x58, scratch + 0x7c);
    let (difference, found, iterator, key, value) = (
        scratch + 0xa0,
        scratch + 0xac,
        scratch + 0xb8,
        scratch + 0xbc,
        scratch + 0xc0,
    );
    let (rotated, shifted_first, shifted_second) = (scratch + 0xcc, scratch + 0xd8, scratch + 0xe4);

    // The relative rotation: the inverse of `second`'s world rotation
    // times `first`'s.
    let second_transform = e.call(WORLD_TRANSFORM, &args![second]).u32();
    e.call(
        TRANSFORM_INVERT_NON_UNIFORM,
        &args![second_transform, transform],
    );
    let first_transform = e.call(WORLD_TRANSFORM, &args![first]).u32();
    let second_transform = e.call(WORLD_TRANSFORM, &args![second]).u32();
    ni_matrix3_inverse(e, Ptr::new(second_transform), Ptr::new(inverse));
    let rotation = e
        .call(MATRIX_PRODUCT, &args![inverse, product, first_transform])
        .u32();
    copy_words(e, matrix, rotation, 9);

    let float_max: f32 = e.global(FLOAT_MAX);
    let tolerance: f32 = e.global(SNAP_TOLERANCE);
    for second_index in 0..second_count {
        let mut nearest = first_count;
        let mut nearest_distance = if threshold < 0.0 {
            float_max
        } else {
            threshold
        };
        for first_index in 0..first_count {
            e.call(
                POINT3_SUBTRACT,
                &args![
                    second_world + 12 * second_index,
                    difference,
                    first_world + 12 * first_index
                ],
            );
            let distance = e.call(POINT3_LENGTH, &args![difference]).f64() as f32;
            if distance < nearest_distance {
                nearest = first_index;
                nearest_distance = distance;
            }
        }
        if nearest >= first_count {
            continue;
        }
        // The entry for this pair: (second index, second count, distance).
        let distance_bits = nearest_distance.to_bits();
        let mut partner = second_count;
        let record = if e.call(POINT_MAP_FIND, &args![map, nearest, found]).bool() {
            let kept_distance = e.mem.f32(found + 8);
            if e.call(
                FLOATS_NEAR,
                &args![nearest_distance, kept_distance, tolerance],
            )
            .bool()
            {
                partner = e.mem.u32(found);
                true
            } else {
                nearest_distance < kept_distance
            }
        } else {
            true
        };
        if record {
            e.call(
                POINT_MAP_SET_AT,
                &args![map, nearest, second_index, partner, distance_bits],
            );
        }
    }

    let mut changed = false;
    let mut position = e.call(POINT_MAP_FIRST, &args![map]).u32();
    e.mem.set_u32(iterator, position);
    while position != 0 {
        e.call(POINT_MAP_NEXT, &args![map, iterator, key, value]);
        position = e.mem.u32(iterator);
        let first_index = e.mem.u32(key);
        let second_index = e.mem.u32(value);
        let partner = e.mem.u32(value + 4);
        let moved = fn_004b4500(
            e,
            Ptr::new(matrix),
            Ptr::new(rotated),
            Ptr::new(first_vectors + 12 * first_index),
        );
        copy_words(e, second_vectors + 12 * second_index, moved.addr(), 3);
        e.call(POINT3_UNITIZE, &args![second_vectors + 12 * second_index]);
        if partner < second_count {
            copy_words(
                e,
                second_vectors + 12 * partner,
                second_vectors + 12 * second_index,
                3,
            );
        }
        if debug {
            let offset: f32 = e.global(DEBUG_OFFSET);
            if e.global::<u32>(LAST_SNAPPED_OBJECT) != first.addr() {
                let scaled = e
                    .call(
                        POINT3_SCALED,
                        &args![shifted_first, offset, first_vectors + 12 * first_index],
                    )
                    .u32();
                e.call(
                    POINT3_ADD_ASSIGN,
                    &args![first_points + 12 * first_index, scaled],
                );
            }
            let scaled = e
                .call(
                    POINT3_SCALED,
                    &args![shifted_second, offset, second_vectors + 12 * second_index],
                )
                .u32();
            e.call(
                POINT3_ADD_ASSIGN,
                &args![second_points + 12 * second_index, scaled],
            );
            if partner < second_count {
                copy_words(
                    e,
                    second_points + 12 * partner,
                    second_points + 12 * second_index,
                    3,
                );
            }
        }
        changed = true;
    }
    if changed {
        if debug {
            let data = e.call(GEOMETRY_DATA, &args![first]).u32();
            e.call(MARK_AS_CHANGED, &args![data, 3u32]);
        }
        let data = e.call(GEOMETRY_DATA, &args![second]).u32();
        e.call(MARK_AS_CHANGED, &args![data, 3u32]);
    }
    e.call(OPERATOR_DELETE, &args![first_world]);
    e.call(OPERATOR_DELETE, &args![second_world]);
    e.set_global(LAST_SNAPPED_OBJECT, first.addr());
    changed
}

// Translated from 004b4500 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiMatrix3 * NiPoint3` (`this` is the matrix, nine floats by rows): the
/// product is built into `out`, which it returns.
pub fn fn_004b4500(e: &mut Engine, this: Ptr, out: Ptr, vector: Ptr) -> Ptr {
    let m = |e: &Engine, i: u32| e.mem.f32(this.addr() + 4 * i) as f64;
    let v = |e: &Engine, i: u32| e.mem.f32(vector.addr() + 4 * i) as f64;
    let (v0, v1, v2) = (v(e, 0), v(e, 1), v(e, 2));
    let z = (m(e, 6) * v0 + m(e, 7) * v1 + m(e, 8) * v2) as f32;
    let y = (m(e, 3) * v0 + m(e, 4) * v1 + m(e, 5) * v2) as f32;
    let x = (m(e, 0) * v0 + m(e, 1) * v1 + m(e, 2) * v2) as f32;
    e.call(POINT3_CONSTRUCT, &args![out, x, y, z]);
    out
}

// Translated from 004b45b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiMatrix3::Inverse` (Xbox PDB): the inverse of the matrix `this` stored
/// into `out`, which it returns; all zeros when [`ni_matrix3_inverse_ov2`]
/// finds `this` singular.
pub fn ni_matrix3_inverse(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    let result = e.mem.alloc(0x24);
    e.call(POINT3_EMPTY_CONSTRUCTOR, &args![result]);
    if !ni_matrix3_inverse_ov2(e, this, Ptr::new(result)) {
        fn_004b4600(e, Ptr::new(result));
    }
    copy_words(e, out.addr(), result, 9);
    e.mem.free(result);
    out
}

// Translated from 004b4600 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets the matrix (nine floats by rows) to all zeros, column by column
/// through [`fn_004b4660`].
pub fn fn_004b4600(e: &mut Engine, this: Ptr) {
    fn_004b4660(e, this, 0, 0.0, 0.0, 0.0);
    fn_004b4660(e, this, 1, 0.0, 0.0, 0.0);
    fn_004b4660(e, this, 2, 0.0, 0.0, 0.0);
}

// Translated from 004b4660 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets column `column` of the matrix (nine floats by rows) to `(x, y, z)`.
pub fn fn_004b4660(e: &mut Engine, this: Ptr, column: u32, x: f32, y: f32, z: f32) {
    let base = this.addr() + 4 * column;
    e.mem.set_f32(base, x);
    e.mem.set_f32(base + 0xc, y);
    e.mem.set_f32(base + 0x18, z);
}

// Translated from 004b46a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiMatrix3::Inverse_ov2` (Xbox PDB): writes the adjugate of the matrix
/// `this` (nine floats by rows) into `out`, then divides it by the
/// determinant and returns true; when the determinant's absolute value
/// (`00408820`) is at most `1e-6` returns false with the unscaled adjugate
/// left in `out`.
pub fn ni_matrix3_inverse_ov2(e: &mut Engine, this: Ptr, out: Ptr) -> bool {
    let mut m = [0.0f64; 9];
    for (i, value) in m.iter_mut().enumerate() {
        *value = e.mem.f32(this.addr() + 4 * i as u32) as f64;
    }
    let adjugate = [
        m[4] * m[8] - m[5] * m[7],
        m[2] * m[7] - m[1] * m[8],
        m[1] * m[5] - m[2] * m[4],
        m[5] * m[6] - m[3] * m[8],
        m[0] * m[8] - m[2] * m[6],
        m[2] * m[3] - m[0] * m[5],
        m[3] * m[7] - m[4] * m[6],
        m[1] * m[6] - m[0] * m[7],
        m[0] * m[4] - m[1] * m[3],
    ];
    for (i, value) in adjugate.iter().enumerate() {
        e.mem.set_f32(out.addr() + 4 * i as u32, *value as f32);
    }
    let o = |e: &Engine, i: u32| e.mem.f32(out.addr() + 4 * i) as f64;
    let determinant = (m[0] * o(e, 0) + m[1] * o(e, 3) + m[2] * o(e, 6)) as f32;
    let magnitude = e.call(FLOAT_ABSOLUTE_VALUE, &args![determinant]).f64();
    let limit: f64 = e.global(SINGULAR_DETERMINANT);
    if magnitude <= limit {
        return false;
    }
    let inverse = (1.0 / determinant as f64) as f32;
    for i in 0..9 {
        let scaled = (o(e, i) * inverse as f64) as f32;
        e.mem.set_f32(out.addr() + 4 * i, scaled);
    }
    true
}

// Translated from 004b4910 (decompiled, FalloutNV.exe 1.4.0.525)
/// The first point array of a geometry object in world space: a new array
/// (from `operator new`; the caller frees it) of `00456650(object)`
/// 12-byte points, filled by `004b4a10(count, the object's points
/// (0049ec60), the array, the object's world transform (00461130))`. Null
/// when `object` is null or has no points.
///
/// The game pushes two words (the world transform and the array) before
/// `0049ec60(object)`, which never reads them; they stay on the stack and
/// are the last two arguments of `004b4a10`.
pub fn fn_004b4910(e: &mut Engine, object: Ptr) -> u32 {
    if object.is_null() || e.call(POINT_COUNT, &args![object]).u16() == 0 {
        return 0;
    }
    let count = e.call(POINT_COUNT, &args![object]).u16() as u32;
    let points = new_array(e, 12, count, POINT3_EMPTY_CONSTRUCTOR);
    let transform = e.call(WORLD_TRANSFORM, &args![object]).u32();
    let source = e.call(POINTS_FIRST, &args![object]).u32();
    let count = e.call(POINT_COUNT, &args![object]).u16() as u32;
    e.call(TRANSFORM_POINTS, &args![count, source, points, transform]);
    points
}

// ---------------------------------------------------------------------
// The bhk constructor chain.
// ---------------------------------------------------------------------

// Translated from 004b5040 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkMouseSpringAction::bhkMouseSpringAction` (Xbox PDB): the
/// `bhkUnaryAction` constructor, then its own vtable, `Init(arg)` and a
/// count of the objects made (`ms_uiObjects`). Returns `this`.
pub fn bhk_mouse_spring_action_bhk_mouse_spring_action(
    e: &mut Engine,
    this: Ptr<BhkSerializable>,
    arg: u32,
) -> Ptr<BhkSerializable> {
    fn_004b50c0(e, this);
    e.mem.set_u32(this.addr(), BHK_MOUSE_SPRING_ACTION_VTABLE);
    e.call(BHK_MOUSE_SPRING_ACTION_INIT, &args![this, arg]);
    let count: u32 = e.global(BHK_MOUSE_SPRING_ACTION_OBJECT_COUNT);
    e.set_global(BHK_MOUSE_SPRING_ACTION_OBJECT_COUNT, count.wrapping_add(1));
    this
}

// Translated from 004b50c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `bhkUnaryAction` constructor (its vtable's destructor is `004b51e0`,
/// `bhkUnaryAction::_scalar_deleting_destructor_`): the `bhkAction`
/// constructor, the `bhkUnaryAction` vtable and its object count. Returns
/// `this`.
pub fn fn_004b50c0(e: &mut Engine, this: Ptr<BhkSerializable>) -> Ptr<BhkSerializable> {
    fn_004b50f0(e, this);
    e.mem.set_u32(this.addr(), BHK_UNARY_ACTION_VTABLE);
    let count: u32 = e.global(BHK_UNARY_ACTION_OBJECT_COUNT);
    e.set_global(BHK_UNARY_ACTION_OBJECT_COUNT, count.wrapping_add(1));
    this
}

// Translated from 004b50f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `bhkAction` constructor (its vtable's destructor is `004b51a0`,
/// `bhkAction::_scalar_deleting_destructor_`): the `bhkSerializable`
/// constructor, the `bhkAction` vtable and its object count. Returns
/// `this`.
pub fn fn_004b50f0(e: &mut Engine, this: Ptr<BhkSerializable>) -> Ptr<BhkSerializable> {
    fn_004b5120(e, this);
    e.mem.set_u32(this.addr(), BHK_ACTION_VTABLE);
    let count: u32 = e.global(BHK_ACTION_OBJECT_COUNT);
    e.set_global(BHK_ACTION_OBJECT_COUNT, count.wrapping_add(1));
    this
}

// Translated from 004b5120 (decompiled, FalloutNV.exe 1.4.0.525)
/// The `bhkSerializable` constructor (its vtable's destructor is
/// `004b5160`): the `bhkRefObject` constructor, the `bhkSerializable`
/// vtable and a null `pInfo`. Returns `this`.
pub fn fn_004b5120(e: &mut Engine, this: Ptr<BhkSerializable>) -> Ptr<BhkSerializable> {
    e.call(BHK_REF_OBJECT_CONSTRUCT, &args![this]);
    e.mem.set_u32(this.addr(), BHK_SERIALIZABLE_VTABLE);
    e.set(this, BhkSerializable::pInfo, 0);
    this
}

// Translated from 004b5150 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkSerializable::GetRTTI` (Xbox PDB): the address of its `NiRTTI`.
pub fn bhk_serializable_get_rtti(_e: &mut Engine, _this: Ptr) -> u32 {
    BHK_SERIALIZABLE_RTTI
}

// Translated from 004b5160 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkSerializable::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`00c877b0`) and, when bit 0 of `flags` is set, frees the
/// 0x10-byte object. Returns `this`.
pub fn bhk_serializable_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(BHK_SERIALIZABLE_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(SIZED_DELETE, &args![this, 0x10u32]);
    }
    this
}

// Translated from 004b5190 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkAction::GetRTTI` (Xbox PDB): the address of its `NiRTTI`.
pub fn bhk_action_get_rtti(_e: &mut Engine, _this: Ptr) -> u32 {
    BHK_ACTION_RTTI
}

// Translated from 004b51a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkAction::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`00c87970`) and, when bit 0 of `flags` is set, frees the
/// 0x10-byte object. Returns `this`.
pub fn bhk_action_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(BHK_ACTION_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(SIZED_DELETE, &args![this, 0x10u32]);
    }
    this
}

// Translated from 004b51d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkUnaryAction::GetRTTI` (Xbox PDB): the address of its `NiRTTI`.
pub fn bhk_unary_action_get_rtti(_e: &mut Engine, _this: Ptr) -> u32 {
    BHK_UNARY_ACTION_RTTI
}

// Translated from 004b51e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `bhkUnaryAction::_scalar_deleting_destructor_` (Xbox PDB): runs the
/// destructor (`00c86720`) and, when bit 0 of `flags` is set, frees the
/// 0x10-byte object. Returns `this`.
pub fn bhk_unary_action_scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(BHK_UNARY_ACTION_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(SIZED_DELETE, &args![this, 0x10u32]);
    }
    this
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x004b1230, is_an_integer(Ptr) -> bool),
        entry!(0x004b12d0, is_a_float(Ptr) -> bool),
        entry!(0x004b13c0, get_z_angle_from_vector(Ptr<NiPoint3>) -> f32),
        entry!(0x004b1460, fn_004b1460(f32) -> f32),
        entry!(0x004b1480, clamp_angle(f32) -> f32),
        entry!(0x004b1500, fn_004b1500(f32, f32) -> f32),
        entry!(0x004b1520, fn_004b1520(f32, f32) -> f32),
        entry!(
            0x004b1550,
            get_unclamped_z_angle_from_vector(Ptr<NiPoint3>) -> f32
        ),
        entry!(0x004b15e0, fn_004b15e0(f32, f32, Ptr) -> f32),
        entry!(0x004b1680, fn_004b1680(u8) -> i32),
        entry!(0x004b1b60, fn_004b1b60(u8) -> u32),
        entry!(0x004b1e80, fn_004b1e80(Ptr, Ptr, Ptr, Ptr) -> bool),
        entry!(0x004b2780, fn_004b2780(Ptr, Ptr, Ptr, f32, Ptr, bool) -> bool),
        entry!(0x004b2800, fn_004b2800(Ptr, Ptr, Ptr, f32, Ptr) -> bool),
        entry!(0x004b2960, fn_004b2960(Ptr) -> bool),
        entry!(0x004b29b0, make_tri_point(f32, Ptr, bool) -> u32),
        entry!(
            0x004b2eb0,
            make_quad_box(Ptr, Ptr, Ptr, Ptr, Ptr, Ptr, Ptr, Ptr, Ptr, bool) -> u32
        ),
        entry!(
            0x004b3570,
            make_triangle(f32, f32, f32, f32, f32, f32, f32, f32, f32, Ptr, bool) -> u32
        ),
        entry!(
            0x004b3800,
            fn_004b3800(Ptr<NiPoint3>, Ptr, Ptr<NiPoint3>) -> Ptr
        ),
        entry!(0x004b3890, fn_004b3890(Ptr, Ptr, Ptr, Ptr, bool) -> u32),
        entry!(0x004b3ab0, fn_004b3ab0(f32, f32, f32, f32, f32) -> f32),
        entry!(0x004b3ae0, fn_004b3ae0(Ptr, Ptr, Ptr) -> Ptr),
        entry!(0x004b3b90, make_rectangle(f32, Ptr, bool) -> u32),
        entry!(0x004b3e60, fn_004b3e60(Ptr, Ptr, f32, u32, bool) -> bool),
        entry!(0x004b4500, fn_004b4500(Ptr, Ptr, Ptr) -> Ptr),
        entry!(0x004b45b0, ni_matrix3_inverse(Ptr, Ptr) -> Ptr),
        entry!(0x004b4600, fn_004b4600(Ptr)),
        entry!(0x004b4660, fn_004b4660(Ptr, u32, f32, f32, f32)),
        entry!(0x004b46a0, ni_matrix3_inverse_ov2(Ptr, Ptr) -> bool),
        entry!(0x004b4910, fn_004b4910(Ptr) -> u32),
        entry!(
            0x004b5040,
            bhk_mouse_spring_action_bhk_mouse_spring_action(
                Ptr<BhkSerializable>,
                u32,
            ) -> Ptr<BhkSerializable>
        ),
        entry!(
            0x004b50c0,
            fn_004b50c0(Ptr<BhkSerializable>) -> Ptr<BhkSerializable>
        ),
        entry!(
            0x004b50f0,
            fn_004b50f0(Ptr<BhkSerializable>) -> Ptr<BhkSerializable>
        ),
        entry!(
            0x004b5120,
            fn_004b5120(Ptr<BhkSerializable>) -> Ptr<BhkSerializable>
        ),
        entry!(0x004b5150, bhk_serializable_get_rtti(Ptr) -> u32),
        entry!(0x004b5160, bhk_serializable_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x004b5190, bhk_action_get_rtti(Ptr) -> u32),
        entry!(0x004b51a0, bhk_action_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x004b51d0, bhk_unary_action_get_rtti(Ptr) -> u32),
        entry!(0x004b51e0, bhk_unary_action_scalar_deleting_destructor(Ptr, u32) -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    fn p(addr: u32) -> Ptr {
        Ptr::new(addr)
    }

    fn ret_u(v: u32) -> Ret {
        Ret {
            eax: v,
            ..Ret::default()
        }
    }

    fn ret_f(v: f64) -> Ret {
        Ret {
            st0: v,
            ..Ret::default()
        }
    }

    fn arg_f(a: &[u32], i: usize) -> f32 {
        f32::from_bits(a[i])
    }

    /// An engine with the pages and constants of the exe's data this file
    /// reads, as the exe holds them.
    fn engine() -> Engine {
        let mut e = Engine::new();
        for page in [
            0x0101_2000,
            0x0101_6000,
            0x0101_7000,
            0x0101_f000,
            0x0118_7000,
            0x0118_8000,
            0x0118_9000,
            0x0118_a000,
            0x011c_6000,
            0x0126_8000,
        ] {
            e.map(page, 0x1000);
        }
        e.set_global(ONE, 1.0f64);
        e.set_global(NEGATIVE_ONE, -1.0f32);
        e.set_global(FLOAT_MAX, f32::MAX);
        e.set_global(THREE_QUARTER_TURN, 4.712389f32);
        e.set_global(QUARTER_TURN, 1.5707964f32);
        e.set_global(HALF_TURN, 3.1415927410125732f64);
        e.set_global(FULL_TURN, 6.2831854820251465f64);
        e.set_global(FULL_TURN_FLOAT, 6.2831855f32);
        e.set_global(NEGATIVE_HALF_TURN, -3.1415927410125732f64);
        e.set_global(SQUARE_ROOT_OF_TWO, 1.4142135381698608f64);
        e.set_global(SINGULAR_DETERMINANT, 9.99999974752427e-7f64);
        e.set_global(SNAP_TOLERANCE, 0.001f32);
        e.set_global(DEBUG_OFFSET, 10.0f32);
        e
    }

    fn text(e: &mut Engine, s: &str) -> Ptr {
        let block = e.mem.alloc(s.len() as u32 + 1);
        e.mem.set_cstr(block, s.as_bytes());
        p(block)
    }

    /// A block holding the given floats.
    fn floats(e: &mut Engine, values: &[f32]) -> u32 {
        let p = e.mem.alloc(4 * values.len() as u32);
        for (i, v) in values.iter().enumerate() {
            e.mem.set_f32(p + 4 * i as u32, *v);
        }
        p
    }

    fn read_floats(e: &Engine, p: u32, n: u32) -> Vec<f32> {
        (0..n).map(|i| e.mem.f32(p + 4 * i)).collect()
    }

    fn assert_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (a, b) in actual.iter().zip(expected) {
            assert!((a - b).abs() < 1e-5, "{actual:?} != {expected:?}");
        }
    }

    /// Stand-ins for the `NiPoint3` helpers other units own.
    fn point_doubles(e: &mut Engine) {
        e.register(POINT3_CONSTRUCT, |e, a| {
            for i in 0..3 {
                e.mem.set_f32(a[0] + 4 * i as u32, arg_f(a, 1 + i));
            }
            ret_u(a[0])
        });
        e.register(POINT3_EMPTY_CONSTRUCTOR, |_, a| ret_u(a[0]));
        e.register(POINT3_SUBTRACT, |e, a| {
            for i in 0..3 {
                let v = e.mem.f32(a[0] + 4 * i) - e.mem.f32(a[2] + 4 * i);
                e.mem.set_f32(a[1] + 4 * i, v);
            }
            ret_u(a[1])
        });
        e.register(POINT3_ADD, |e, a| {
            for i in 0..3 {
                let v = e.mem.f32(a[0] + 4 * i) + e.mem.f32(a[2] + 4 * i);
                e.mem.set_f32(a[1] + 4 * i, v);
            }
            ret_u(a[1])
        });
        e.register(POINT3_LENGTH, |e, a| {
            let sum: f64 = (0..3)
                .map(|i| (e.mem.f32(a[0] + 4 * i) as f64).powi(2))
                .sum();
            ret_f(sum.sqrt())
        });
        e.register(POINT3_UNITIZE, |e, a| {
            let v: Vec<f32> = (0..3).map(|i| e.mem.f32(a[0] + 4 * i)).collect();
            let length = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            for (i, x) in v.iter().enumerate() {
                e.mem.set_f32(a[0] + 4 * i as u32, x / length);
            }
            Ret::default()
        });
        e.register(POINT3_SCALED, |e, a| {
            for i in 0..3 {
                let v = e.mem.f32(a[2] + 4 * i) * arg_f(a, 1);
                e.mem.set_f32(a[0] + 4 * i, v);
            }
            ret_u(a[0])
        });
        e.register(POINT3_ADD_ASSIGN, |e, a| {
            for i in 0..3 {
                let v = e.mem.f32(a[0] + 4 * i) + e.mem.f32(a[1] + 4 * i);
                e.mem.set_f32(a[0] + 4 * i, v);
            }
            ret_u(a[0])
        });
    }

    fn math_doubles(e: &mut Engine) {
        e.register(CRT_ARC_TANGENT, |_, a| ret_f(f64::take(a, &mut 0).atan()));
        e.register(CRT_FMOD, |_, a| {
            let mut i = 0;
            let x = f64::take(a, &mut i);
            let y = f64::take(a, &mut i);
            ret_f(x % y)
        });
        e.register(FLOAT_ABSOLUTE_VALUE, |_, a| ret_f(arg_f(a, 0).abs() as f64));
    }

    fn take_log(e: &mut Engine) -> Vec<(u32, Vec<u32>)> {
        e.call_log.take().unwrap_or_default()
    }

    fn calls_to(log: &[(u32, Vec<u32>)], addr: u32) -> Vec<Vec<u32>> {
        log.iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    // ---- strings ----

    #[test]
    fn is_an_integer_accepts_digits_with_an_optional_minus() {
        let mut e = engine();
        for (s, want) in [
            ("0", true),
            ("123", true),
            ("-5", true),
            ("-", false),
            ("", false),
            ("12a", false),
            ("1.5", false),
            ("+1", false),
            ("1-", false),
        ] {
            let p = text(&mut e, s);
            assert_eq!(e.call(0x004b_1230, &args![p]).bool(), want, "{s:?}");
        }
    }

    #[test]
    fn is_a_float_accepts_one_point_and_a_leading_minus() {
        let mut e = engine();
        for (s, want) in [
            ("1.5", true),
            ("-.5", true),
            ("5.", true),
            (".5", true),
            ("-0.25", true),
            ("7", true),
            (".", false),
            ("-", false),
            ("", false),
            ("1.2.3", false),
            ("1-2", false),
            ("..", false),
            ("a", false),
        ] {
            let p = text(&mut e, s);
            assert_eq!(e.call(0x004b_12d0, &args![p]).bool(), want, "{s:?}");
        }
    }

    // ---- angles ----

    fn vector(e: &mut Engine, x: f32, y: f32) -> Ptr<NiPoint3> {
        Ptr::new(floats(e, &[x, y, 0.0]))
    }

    #[test]
    fn z_angle_uses_the_arc_tangent_and_half_a_turn_below_the_axis() {
        let mut e = engine();
        math_doubles(&mut e);
        let v = vector(&mut e, 1.0, 1.0);
        assert_close(
            &[e.call(0x004b_1550, &args![v]).f32()],
            &[std::f32::consts::FRAC_PI_4],
        );
        let v = vector(&mut e, 1.0, -1.0);
        assert_close(&[e.call(0x004b_1550, &args![v]).f32()], &[2.356194]);
        // The unclamped form keeps the negative angle.
        let v = vector(&mut e, -1.0, 1.0);
        assert_close(
            &[e.call(0x004b_1550, &args![v]).f32()],
            &[-std::f32::consts::FRAC_PI_4],
        );
    }

    #[test]
    fn z_angle_on_the_x_axis_is_a_quarter_or_three_quarters_of_a_turn() {
        let mut e = engine();
        math_doubles(&mut e);
        e.call_log = Some(vec![]);
        for (x, want) in [(2.0, 1.5707964), (-2.0, 4.712389), (0.0, 4.712389)] {
            let v = vector(&mut e, x, 0.0);
            assert_close(&[e.call(0x004b_1550, &args![v]).f32()], &[want]);
            assert_close(&[e.call(0x004b_13c0, &args![v]).f32()], &[want]);
        }
        // No arc tangent was needed.
        let log = take_log(&mut e);
        assert!(calls_to(&log, CRT_ARC_TANGENT).is_empty());
    }

    #[test]
    fn z_angle_clamps_into_a_turn() {
        let mut e = engine();
        math_doubles(&mut e);
        let v = vector(&mut e, -1.0, 1.0);
        assert_close(&[e.call(0x004b_13c0, &args![v]).f32()], &[5.497787]);
        let v = vector(&mut e, 1.0, 1.0);
        assert_close(
            &[e.call(0x004b_13c0, &args![v]).f32()],
            &[std::f32::consts::FRAC_PI_4],
        );
    }

    #[test]
    fn the_float_arc_tangent_wrapper_calls_the_crt_with_a_double() {
        let mut e = engine();
        math_doubles(&mut e);
        e.call_log = Some(vec![]);
        let r = e.call(0x004b_1460, &args![1.0f32]).f32();
        assert_close(&[r], &[std::f32::consts::FRAC_PI_4]);
        let log = take_log(&mut e);
        assert_eq!(calls_to(&log, CRT_ARC_TANGENT), vec![args![1.0f64]]);
    }

    #[test]
    fn clamp_angle_wraps_negative_and_large_angles_only() {
        let mut e = engine();
        math_doubles(&mut e);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x004b_1480, &args![1.0f32]).f32(), 1.0);
        assert_close(&[e.call(0x004b_1480, &args![-1.0f32]).f32()], &[5.2831855]);
        assert_close(&[e.call(0x004b_1480, &args![7.0f32]).f32()], &[0.7168147]);
        let log = take_log(&mut e);
        // The remainder was taken for the last two only.
        assert_eq!(calls_to(&log, CRT_FMOD).len(), 2);
        assert_eq!(
            calls_to(&log, CRT_FMOD)[1],
            args![7.0f64, 6.2831855f32 as f64]
        );
    }

    #[test]
    fn the_remainder_wrappers_forward_to_fmod() {
        let mut e = engine();
        math_doubles(&mut e);
        assert_eq!(e.call(0x004b_1500, &args![7.5f32, 2.0f32]).f32(), 1.5);
        assert_eq!(e.call(0x004b_1520, &args![-7.5f32, 2.0f32]).f32(), -1.5);
    }

    #[test]
    fn angle_difference_gives_magnitude_and_direction() {
        let mut e = engine();
        let direction = p(e.mem.alloc(4));
        let turn = |e: &mut Engine, from: f32, to: f32| {
            let r = e.call(0x004b_15e0, &args![from, to, direction]).f32();
            (r, e.mem.f32(direction.addr()))
        };
        assert_eq!(turn(&mut e, 1.0, 1.0), (0.0, 0.0));
        assert_eq!(turn(&mut e, 0.0, 1.0), (1.0, 1.0));
        // Backwards by less than half a turn: negative, direction -1.
        assert_eq!(turn(&mut e, 1.0, 0.0), (-1.0, -1.0));
        // Forwards by more than half a turn: the short way round, -1.
        let (magnitude, sign) = turn(&mut e, 0.0, 4.0);
        assert_close(&[magnitude], &[2.2831855]);
        assert_eq!(sign, -1.0);
        // Backwards by more than half a turn: wraps, direction 1.
        let (magnitude, sign) = turn(&mut e, 4.0, 0.0);
        assert_close(&[magnitude], &[2.2831855]);
        assert_eq!(sign, 1.0);
    }

    // ---- form type lookups ----

    #[test]
    fn form_type_index_follows_the_switch() {
        let mut e = engine();
        for (form_type, want) in [
            (3u8, -1),
            (4, 36),
            (5, 38),
            (6, -1),
            (7, 52),
            (0xb, 54),
            (0xc, 53),
            (0x10, 70),
            (0x75, 86),
            (0x76, -1),
            (0xff, -1),
        ] {
            assert_eq!(
                e.call(0x004b_1680, &args![form_type]).i32(),
                want,
                "{form_type:#x}"
            );
        }
    }

    #[test]
    fn form_type_name_prefers_the_index_then_the_slot_then_the_table() {
        let mut e = engine();
        // Type 4: index 36 into the name table.
        e.set_global(FORM_NAME_BY_INDEX + 4 * 36, 0xaaaa_0001u32);
        assert_eq!(e.call(0x004b_1b60, &args![4u8]).u32(), 0xaaaa_0001);
        // Type 3: no index, its own slot.
        e.set_global(0x0118_8fbcu32, 0xbbbb_0003u32);
        assert_eq!(e.call(0x004b_1b60, &args![3u8]).u32(), 0xbbbb_0003);
        // Type 0x75: index 86 wins over its slot.
        e.set_global(FORM_NAME_BY_INDEX + 4 * 86, 0xcccc_0075u32);
        e.set_global(0x0118_9034u32, 0xdddd_0075u32);
        assert_eq!(e.call(0x004b_1b60, &args![0x75u8]).u32(), 0xcccc_0075);
        // Type 1: the 12-byte table.
        e.set_global(FORM_NAME_BY_TYPE + 12, 0xeeee_0001u32);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x004b_1b60, &args![1u8]).u32(), 0xeeee_0001);
        // Type 0x78 is the last table entry and logs nothing.
        e.set_global(FORM_NAME_BY_TYPE + 12 * 0x78, 0xeeee_0078u32);
        assert_eq!(e.call(0x004b_1b60, &args![0x78u8]).u32(), 0xeeee_0078);
        assert!(calls_to(&take_log(&mut e), LOG_MESSAGE).is_empty());
    }

    #[test]
    fn form_type_name_logs_an_unknown_type_and_uses_entry_zero() {
        let mut e = engine();
        e.set_global(FORM_NAME_BY_TYPE, 0x1111_0000u32);
        e.set_global(FORM_NAME_BY_TYPE + 12 * 0x7a, 0x2222_007au32);
        e.call_log = Some(vec![]);
        e.register(LOG_MESSAGE, |_, _| Ret::default());
        assert_eq!(e.call(0x004b_1b60, &args![0x7au8]).u32(), 0x1111_0000);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, LOG_MESSAGE),
            vec![args![INVALID_FORM_ID_MESSAGE, 0x2222_007au32]]
        );
    }

    // ---- segment against rectangle ----

    /// Runs the clipper on `rect` = `(x min, y max, x max, y min)`.
    fn clip(e: &mut Engine, rect: [i32; 4], from: [f32; 3], to: [f32; 2]) -> (bool, Vec<f32>) {
        let r = e.mem.alloc(16);
        for (i, v) in rect.iter().enumerate() {
            e.mem.set_i32(r + 4 * i as u32, *v);
        }
        let from = floats(e, &from);
        let to = floats(e, &[to[0], to[1], 0.0]);
        let out = floats(e, &[7.0, 7.0, 7.0]);
        let found = e
            .call(0x004b_1e80, &args![p(r), p(from), p(to), p(out)])
            .bool();
        (found, read_floats(e, out, 3))
    }

    /// The rectangle `x 0..10`, `y 0..10`.
    const RECT: [i32; 4] = [0, 10, 10, 0];

    #[test]
    fn clip_rejects_an_empty_rectangle() {
        let mut e = engine();
        point_doubles(&mut e);
        let (found, out) = clip(&mut e, [10, 10, 0, 0], [5.0, 5.0, 0.0], [20.0, 5.0]);
        assert!(!found);
        assert_eq!(out, vec![7.0; 3]);
        let (found, _) = clip(&mut e, [0, 0, 10, 10], [5.0, 5.0, 0.0], [20.0, 5.0]);
        assert!(!found);
    }

    #[test]
    fn clip_rejects_segments_beside_an_edge_or_wholly_inside() {
        let mut e = engine();
        point_doubles(&mut e);
        for (from, to) in [
            ([-5.0, 5.0, 0.0], [-1.0, 5.0]),
            ([15.0, 5.0, 0.0], [11.0, 9.0]),
            ([5.0, -5.0, 0.0], [3.0, -1.0]),
            ([5.0, 15.0, 0.0], [2.0, 12.0]),
            // Both strictly inside.
            ([2.0, 2.0, 0.0], [8.0, 8.0]),
        ] {
            let (found, out) = clip(&mut e, RECT, from, to);
            assert!(!found, "{from:?} {to:?}");
            assert_eq!(out, vec![7.0; 3]);
        }
    }

    #[test]
    fn clip_of_a_single_point_gives_the_point_with_its_z() {
        let mut e = engine();
        point_doubles(&mut e);
        // A point strictly inside is "wholly inside": nothing to clip.
        let (found, out) = clip(&mut e, RECT, [3.0, 4.0, 9.0], [3.0, 4.0]);
        assert!(!found);
        assert_eq!(out, vec![7.0; 3]);
        // The border counts as inside.
        let (found, out) = clip(&mut e, RECT, [10.0, 10.0, 1.0], [10.0, 10.0]);
        assert!(found);
        assert_eq!(out, vec![10.0, 10.0, 1.0]);
    }

    #[test]
    fn clip_of_a_vertical_segment_picks_the_edge_by_direction() {
        let mut e = engine();
        point_doubles(&mut e);
        // Upwards (y grows): the y min edge.
        let (found, out) = clip(&mut e, RECT, [5.0, 5.0, 3.0], [5.0, 15.0]);
        assert!(found);
        assert_eq!(out, vec![5.0, 0.0, 0.0]);
        // Downwards: the y max edge.
        let (found, out) = clip(&mut e, RECT, [5.0, 5.0, 3.0], [5.0, -5.0]);
        assert!(found);
        assert_eq!(out, vec![5.0, 10.0, 0.0]);
        // A vertical line beside the rectangle is rejected before this.
        let (found, _) = clip(&mut e, RECT, [11.0, 5.0, 0.0], [11.0, 6.0]);
        assert!(!found);
    }

    #[test]
    fn clip_of_a_horizontal_segment_picks_the_edge_by_direction() {
        let mut e = engine();
        point_doubles(&mut e);
        let (found, out) = clip(&mut e, RECT, [5.0, 5.0, 0.0], [-5.0, 5.0]);
        assert!(found);
        assert_eq!(out, vec![10.0, 5.0, 0.0]);
        let (found, out) = clip(&mut e, RECT, [5.0, 5.0, 0.0], [15.0, 5.0]);
        assert!(found);
        assert_eq!(out, vec![0.0, 5.0, 0.0]);
    }

    #[test]
    fn clip_of_a_slanted_segment_from_inside_chooses_by_the_unit_vector_sum() {
        let mut e = engine();
        point_doubles(&mut e);
        // The line y = x / 2 + 1 crosses the left edge at (0, 1) and the right
        // edge at (10, 6). `from` is strictly inside; the unit vectors along
        // the segment and to the first hit cancel, so the second hit is chosen.
        let (found, out) = clip(&mut e, RECT, [2.0, 2.0, 0.0], [12.0, 7.0]);
        assert!(found);
        assert_close(&out, &[10.0, 6.0, 0.0]);
        // Heading for the first hit the unit vectors add up to more than one
        // long: the first hit.
        let (found, out) = clip(&mut e, RECT, [8.0, 5.0, 0.0], [-4.0, -1.0]);
        assert!(found);
        assert_close(&out, &[0.0, 1.0, 0.0]);
    }

    #[test]
    fn clip_of_a_slanted_segment_from_outside_chooses_the_nearer_hit() {
        let mut e = engine();
        point_doubles(&mut e);
        // y = x + 5 hits the left edge at (0, 5) and the top at (5, 10).
        let (found, out) = clip(&mut e, RECT, [-2.0, 3.0, 0.0], [4.0, 9.0]);
        assert!(found);
        assert_close(&out, &[0.0, 5.0, 0.0]);
        // y = 11 - x / 2 hits the top at (2, 10) and the right at (10, 6);
        // from (12, 5) the second is nearer.
        let (found, out) = clip(&mut e, RECT, [12.0, 5.0, 0.0], [0.0, 11.0]);
        assert!(found);
        assert_close(&out, &[10.0, 6.0, 0.0]);
        // A line that misses the rectangle.
        let (found, _) = clip(&mut e, RECT, [-5.0, 12.0, 0.0], [5.0, 20.0]);
        assert!(!found);
    }

    // ---- picking ----

    layout! {
        /// A test node: the vtable word, then the world bound pointer.
        struct TestNode: 0x40 {
            0x20 bound: u32,
        }
    }

    fn bound(e: &mut Engine, centre: [f32; 3], radius: f32) -> u32 {
        floats(e, &[centre[0], centre[1], centre[2], radius])
    }

    #[test]
    fn a_node_has_a_bound_when_its_radius_is_not_zero() {
        let mut e = engine();
        e.register(BOUND_RADIUS, |e, a| ret_f(e.mem.f32(a[0] + 0xc) as f64));
        let node: Ptr<TestNode> = e.new_object();
        assert!(!e.call(0x004b_2960, &args![node]).bool());
        let b = bound(&mut e, [0.0; 3], 0.0);
        e.set(node, TestNode::bound, b);
        assert!(!e.call(0x004b_2960, &args![node]).bool());
        let b = bound(&mut e, [0.0; 3], 2.5);
        e.set(node, TestNode::bound, b);
        assert!(e.call(0x004b_2960, &args![node]).bool());
    }

    struct PickWorld {
        e: Engine,
        object: Ptr,
        origin: Ptr,
        direction: Ptr,
        root: Ptr<TestNode>,
        leaves: Vec<Ptr<TestNode>>,
        picked: Rc<RefCell<Vec<u32>>>,
    }

    /// A root node with a container of two leaves, each with a bound of
    /// radius 1 at the origin; picking succeeds for the leaves in `hits`.
    fn pick_world(blocked: bool, hits: Vec<usize>) -> PickWorld {
        let mut e = engine();
        e.register(BOUND_RADIUS, |e, a| ret_f(e.mem.f32(a[0] + 0xc) as f64));
        point_doubles(&mut e);
        e.register(GET_WORLD_BOUND, |e, a| ret_u(e.mem.u32(a[0] + 0x20)));
        e.register(PICK_BLOCKED_A, |_, _| ret_u(1));
        if blocked {
            e.register(PICK_BLOCKED_B, |_, _| ret_u(1));
        } else {
            e.register(PICK_BLOCKED_B, |_, _| ret_u(0));
        }
        let container = e.mem.alloc(8);
        e.register(CHILD_COUNT, |_, _| ret_u(2));
        e.register_double(CHILD_AT, move |e, a| {
            assert_eq!(a[0], container);
            ret_u(e.mem.u32(container + 4 * a[1]))
        });
        // Virtual slot 0xc: the root returns the container, leaves null.
        let root_table = 0x0300_0000;
        let leaf_table = 0x0300_1000;
        e.put_vtable(root_table, &[0, 0, 0, 0x0300_2000]);
        e.put_vtable(leaf_table, &[0, 0, 0, 0x0300_2010]);
        e.register_double(0x0300_2000, move |_, _| ret_u(container));
        e.register(0x0300_2010, |_, _| ret_u(0));
        let make = |e: &mut Engine, table: u32, radius: f32| {
            let node: Ptr<TestNode> = e.new_object();
            e.mem.set_u32(node.addr(), table);
            let b = bound(e, [0.0; 3], radius);
            e.set(node, TestNode::bound, b);
            node
        };
        let root = make(&mut e, root_table, 1.0);
        let leaves = vec![make(&mut e, leaf_table, 1.0), make(&mut e, leaf_table, 1.0)];
        e.mem.set_u32(container, leaves[0].addr());
        e.mem.set_u32(container + 4, leaves[1].addr());
        let picked = Rc::new(RefCell::new(Vec::new()));
        let last_root = Rc::new(RefCell::new(0u32));
        {
            let last_root = last_root.clone();
            e.register_double(PICK_SET_ROOT, move |_, a| {
                *last_root.borrow_mut() = a[1];
                Ret::default()
            });
        }
        {
            let picked = picked.clone();
            let leaf_addrs: Vec<u32> = leaves.iter().map(|l| l.addr()).collect();
            e.register_double(PICK_OBJECTS, move |_, a| {
                let root = *last_root.borrow();
                picked.borrow_mut().push(root);
                let hit = hits.iter().any(|&i| leaf_addrs[i] == root) && a[3] == 1;
                ret_u(hit as u32)
            });
        }
        let object = p(e.mem.alloc(16));
        let origin = p(floats(&mut e, &[0.0, 0.0, 0.0]));
        let direction = p(floats(&mut e, &[1.0, 0.0, 0.0]));
        PickWorld {
            e,
            object,
            origin,
            direction,
            root,
            leaves,
            picked,
        }
    }

    #[test]
    fn picking_recurses_over_the_children_and_picks_the_leaves() {
        let mut w = pick_world(false, vec![1]);
        let r =
            w.e.call(
                0x004b_2800,
                &args![w.object, w.origin, w.direction, 5.0f32, w.root],
            )
            .bool();
        assert!(r);
        // Both leaves were picked against, in order.
        let expected: Vec<u32> = w.leaves.iter().map(|l| l.addr()).collect();
        assert_eq!(*w.picked.borrow(), expected);
    }

    #[test]
    fn picking_without_a_hit_is_false() {
        let mut w = pick_world(false, vec![]);
        let r =
            w.e.call(
                0x004b_2800,
                &args![w.object, w.origin, w.direction, 5.0f32, w.root],
            )
            .bool();
        assert!(!r);
        assert_eq!(w.picked.borrow().len(), 2);
    }

    #[test]
    fn picking_skips_nodes_too_far_from_the_ray_origin() {
        let mut w = pick_world(false, vec![0, 1]);
        // The bound's centre is 100 away from the ray origin; radius 1 and
        // distance 5 do not reach it.
        let far = p(floats(&mut w.e, &[100.0, 0.0, 0.0]));
        let r =
            w.e.call(
                0x004b_2800,
                &args![w.object, far, w.direction, 5.0f32, w.root],
            )
            .bool();
        assert!(!r);
        assert!(w.picked.borrow().is_empty());
    }

    #[test]
    fn picking_stops_at_a_node_both_checks_block() {
        let mut w = pick_world(true, vec![0, 1]);
        let r =
            w.e.call(
                0x004b_2800,
                &args![w.object, w.origin, w.direction, 5.0f32, w.root],
            )
            .bool();
        assert!(!r);
        assert!(w.picked.borrow().is_empty());
        // Null arguments are refused before any call.
        w.e.call_log = Some(vec![]);
        assert!(!w
            .e
            .call(
                0x004b_2800,
                &args![Ptr::<()>::NULL, w.origin, w.direction, 5.0f32, w.root]
            )
            .bool());
        assert_eq!(take_log(&mut w.e).len(), 1);
    }

    #[test]
    fn the_pick_wrapper_checks_its_arguments_and_can_reset_the_state() {
        let mut w = pick_world(false, vec![0]);
        w.e.call_log = Some(vec![]);
        for (object, distance, node) in [
            (Ptr::<()>::NULL, 5.0f32, w.root),
            (w.object, 0.0, w.root),
            (w.object, 5.0, Ptr::<TestNode>::NULL),
        ] {
            let r =
                w.e.call(
                    0x004b_2780,
                    &args![object, w.origin, w.direction, distance, node, true],
                )
                .bool();
            assert!(!r);
        }
        let log = take_log(&mut w.e);
        assert_eq!(log.len(), 3, "only the three top-level calls");
        // Without reset: straight to the recursive picker.
        let r =
            w.e.call(
                0x004b_2780,
                &args![w.object, w.origin, w.direction, 5.0f32, w.root, false],
            )
            .bool();
        assert!(r);
        // With reset: the state is cleared first.
        w.e.call_log = Some(vec![]);
        w.e.call(
            0x004b_2780,
            &args![w.object, w.origin, w.direction, 5.0f32, w.root, true],
        );
        let log = take_log(&mut w.e);
        assert_eq!(calls_to(&log, PICK_SET_ROOT)[0], args![w.object, 0u32]);
        assert_eq!(
            calls_to(&log, PICK_OBJECTS)[0],
            args![w.object, PICK_STATE, PICK_STATE, 0u32]
        );
    }

    // ---- debug shape builders ----

    fn shape_engine() -> Engine {
        let mut e = engine();
        point_doubles(&mut e);
        e.register(OPERATOR_NEW, |e, a| ret_u(e.mem.alloc(a[0])));
        e.register(NI_OPERATOR_NEW, |e, a| ret_u(e.mem.alloc(a[0])));
        e.register(NI_ARRAY_NEW, |e, a| ret_u(e.mem.alloc(a[0])));
        e.register(VECTOR_CONSTRUCT, |_, _| Ret::default());
        e.register(NI_TRI_SHAPE_CONSTRUCT, |_, a| ret_u(a[0]));
        e.register(NI_LINES_CONSTRUCT, |_, a| ret_u(a[0]));
        e.register(NO_LIGHTING_PROPERTY_CONSTRUCT, |_, a| ret_u(a[0]));
        e.register(ATTACH_PROPERTY, |_, _| Ret::default());
        e.register(PREPARE_OBJECT, |_, _| Ret::default());
        e
    }

    fn color(e: &mut Engine) -> Ptr {
        p(floats(e, &[1.0, 0.5, 0.25, 0.75]))
    }

    fn words(e: &Engine, p: u32, n: u32) -> Vec<u32> {
        (0..n).map(|i| e.mem.u32(p + 4 * i)).collect()
    }

    fn color_words() -> Vec<u32> {
        [1.0f32, 0.5, 0.25, 0.75]
            .iter()
            .map(|v| v.to_bits())
            .collect()
    }

    #[test]
    fn make_tri_point_builds_a_bipyramid_over_the_colour() {
        let mut e = shape_engine();
        let c = color(&mut e);
        e.call_log = Some(vec![]);
        let shape = e.call(0x004b_29b0, &args![2.0f32, c, false]).u32();
        let log = take_log(&mut e);
        let ctor = &calls_to(&log, NI_TRI_SHAPE_CONSTRUCT)[0];
        // (this, vertex count, vertices, 0, colors, 0, 0, 0, triangle count, triangles)
        assert_eq!(ctor[0], shape);
        assert_eq!((ctor[1], ctor[3], ctor[8]), (6, 0, 8));
        assert_eq!(&ctor[5..8], &[0, 0, 0]);
        let apex = (2.0f64 * 1.4142135381698608) as f32;
        assert_close(
            &read_floats(&e, ctor[2], 18),
            &[
                0.0, 0.0, apex, -2.0, 2.0, 0.0, 2.0, 2.0, 0.0, 2.0, -2.0, 0.0, -2.0, -2.0, 0.0,
                0.0, 0.0, -apex,
            ],
        );
        for i in 0..6 {
            assert_eq!(words(&e, ctor[4] + 16 * i, 4), color_words());
        }
        let indices: Vec<u16> = (0..24).map(|i| e.mem.u16(ctor[9] + 2 * i)).collect();
        assert_eq!(indices, TRI_POINT_INDICES);
        // No property without the flag; the arrays came from the allocators.
        assert!(calls_to(&log, ATTACH_PROPERTY).is_empty());
        assert!(calls_to(&log, PREPARE_OBJECT).is_empty());
        assert_eq!(
            calls_to(&log, OPERATOR_NEW),
            vec![args![72u32], args![96u32]]
        );
        assert_eq!(calls_to(&log, NI_ARRAY_NEW), vec![args![48u32]]);
        assert_eq!(calls_to(&log, NI_OPERATOR_NEW), vec![args![0xc4u32]]);
        assert_eq!(
            calls_to(&log, VECTOR_CONSTRUCT)
                .iter()
                .map(|a| (a[1], a[2], a[3]))
                .collect::<Vec<_>>(),
            vec![(12, 6, 0x0068_15c0), (16, 6, 0x004a_7800)]
        );
    }

    #[test]
    fn making_a_shape_with_a_property_attaches_and_prepares_it() {
        let mut e = shape_engine();
        let c = color(&mut e);
        e.call_log = Some(vec![]);
        let shape = e.call(0x004b_29b0, &args![1.0f32, c, true]).u32();
        let log = take_log(&mut e);
        let property = calls_to(&log, NO_LIGHTING_PROPERTY_CONSTRUCT)[0][0];
        assert_eq!(
            calls_to(&log, NI_OPERATOR_NEW),
            vec![args![0xc4u32], args![0x80u32]]
        );
        assert_eq!(
            calls_to(&log, ATTACH_PROPERTY),
            vec![args![shape, property]]
        );
        assert_eq!(
            calls_to(&log, PREPARE_OBJECT),
            vec![args![shape, 0u32, 0u32]]
        );
    }

    #[test]
    fn a_failed_shape_allocation_gives_a_null_shape() {
        let mut e = shape_engine();
        e.register(NI_OPERATOR_NEW, |_, _| ret_u(0));
        let c = color(&mut e);
        e.call_log = Some(vec![]);
        assert_eq!(e.call(0x004b_29b0, &args![1.0f32, c, false]).u32(), 0);
        assert!(calls_to(&take_log(&mut e), NI_TRI_SHAPE_CONSTRUCT).is_empty());
    }

    #[test]
    fn make_quad_box_copies_the_eight_corners() {
        let mut e = shape_engine();
        let c = color(&mut e);
        let corners: Vec<Ptr> = (0..8)
            .map(|i| {
                p(floats(
                    &mut e,
                    &[i as f32, 10.0 + i as f32, 20.0 + i as f32],
                ))
            })
            .collect();
        e.call_log = Some(vec![]);
        e.call(
            0x004b_2eb0,
            &args![
                corners[0], corners[1], corners[2], corners[3], corners[4], corners[5], corners[6],
                corners[7], c, false
            ],
        );
        let log = take_log(&mut e);
        let ctor = &calls_to(&log, NI_TRI_SHAPE_CONSTRUCT)[0];
        assert_eq!((ctor[1], ctor[8]), (8, 0x18));
        let want: Vec<f32> = (0..8)
            .flat_map(|i| [i as f32, 10.0 + i as f32, 20.0 + i as f32])
            .collect();
        assert_eq!(read_floats(&e, ctor[2], 24), want);
        for i in 0..8 {
            assert_eq!(words(&e, ctor[4] + 16 * i, 4), color_words());
        }
        let indices: Vec<u16> = (0..72).map(|i| e.mem.u16(ctor[9] + 2 * i)).collect();
        assert_eq!(indices, QUAD_BOX_INDICES);
        assert_eq!(calls_to(&log, NI_ARRAY_NEW), vec![args![144u32]]);
    }

    #[test]
    fn make_triangle_takes_its_vertices_by_value() {
        let mut e = shape_engine();
        let c = color(&mut e);
        e.call_log = Some(vec![]);
        e.call(
            0x004b_3570,
            &args![1.0f32, 2.0f32, 3.0f32, 4.0f32, 5.0f32, 6.0f32, 7.0f32, 8.0f32, 9.0f32, c, true],
        );
        let log = take_log(&mut e);
        let ctor = &calls_to(&log, NI_TRI_SHAPE_CONSTRUCT)[0];
        assert_eq!((ctor[1], ctor[8]), (3, 2));
        assert_eq!(
            read_floats(&e, ctor[2], 9),
            (1..=9).map(|i| i as f32).collect::<Vec<_>>()
        );
        for i in 0..3 {
            assert_eq!(words(&e, ctor[4] + 16 * i, 4), color_words());
        }
        let indices: Vec<u16> = (0..6).map(|i| e.mem.u16(ctor[9] + 2 * i)).collect();
        assert_eq!(indices, TRIANGLE_INDICES);
        assert_eq!(calls_to(&log, ATTACH_PROPERTY).len(), 1);
    }

    #[test]
    fn the_segment_builder_makes_a_two_vertex_line() {
        let mut e = shape_engine();
        let start = p(floats(&mut e, &[1.0, 2.0, 3.0]));
        let end = p(floats(&mut e, &[4.0, 5.0, 6.0]));
        let c0 = p(floats(&mut e, &[0.1, 0.2, 0.3, 0.4]));
        let c1 = p(floats(&mut e, &[0.5, 0.6, 0.7, 0.8]));
        e.call_log = Some(vec![]);
        e.call(0x004b_3890, &args![start, c0, end, c1, false]);
        let log = take_log(&mut e);
        // (this, count, vertices, colors, 0, 0, 0, flags)
        let ctor = &calls_to(&log, NI_LINES_CONSTRUCT)[0];
        assert_eq!(ctor[1], 2);
        assert_eq!(&ctor[4..7], &[0, 0, 0]);
        assert_eq!(
            read_floats(&e, ctor[2], 6),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
        assert_eq!(
            read_floats(&e, ctor[3], 8),
            vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8]
        );
        assert_eq!((e.mem.u8(ctor[7]), e.mem.u8(ctor[7] + 1)), (1, 0));
        assert_eq!(
            calls_to(&log, OPERATOR_NEW),
            vec![args![24u32], args![32u32], args![2u32]]
        );
    }

    #[test]
    fn make_rectangle_outlines_the_square() {
        let mut e = shape_engine();
        let c = color(&mut e);
        e.call_log = Some(vec![]);
        e.call(0x004b_3b90, &args![3.0f32, c, true]);
        let log = take_log(&mut e);
        let ctor = &calls_to(&log, NI_LINES_CONSTRUCT)[0];
        assert_eq!(ctor[1], 4);
        assert_eq!(
            read_floats(&e, ctor[2], 12),
            vec![-3.0, -3.0, 0.0, -3.0, 3.0, 0.0, 3.0, 3.0, 0.0, 3.0, -3.0, 0.0]
        );
        for i in 0..4 {
            assert_eq!(words(&e, ctor[3] + 16 * i, 4), color_words());
            assert_eq!(e.mem.u8(ctor[7] + i), 1);
        }
        // The arrays and the flags are allocated before the points are built.
        assert_eq!(
            calls_to(&log, OPERATOR_NEW),
            vec![args![48u32], args![64u32], args![4u32]]
        );
        assert_eq!(calls_to(&log, ATTACH_PROPERTY).len(), 1);
    }

    // ---- vectors and matrices ----

    #[test]
    fn the_cross_product_is_built_into_out() {
        let mut e = engine();
        point_doubles(&mut e);
        let a = floats(&mut e, &[1.0, 2.0, 3.0]);
        let b = floats(&mut e, &[4.0, 5.0, 6.0]);
        let out = e.mem.alloc(12);
        let r = e.call(0x004b_3800, &args![p(a), p(out), p(b)]);
        assert_eq!(r.u32(), out);
        // (2*6 - 3*5, 3*4 - 1*6, 1*5 - 2*4)
        assert_eq!(read_floats(&e, out, 3), vec![-3.0, 6.0, -3.0]);
    }

    #[test]
    fn interpolation_runs_a_to_b_over_t0_to_t1() {
        let mut e = engine();
        let r = |e: &mut Engine, t: f32| {
            e.call(0x004b_3ab0, &args![10.0f32, 20.0f32, 2.0f32, 4.0f32, t])
                .f32()
        };
        assert_eq!(r(&mut e, 2.0), 10.0);
        assert_eq!(r(&mut e, 3.0), 15.0);
        assert_eq!(r(&mut e, 4.0), 20.0);
        // Not clamped.
        assert_eq!(r(&mut e, 6.0), 30.0);
    }

    #[test]
    fn row_vector_times_matrix_and_matrix_times_vector_differ_by_transposition() {
        let mut e = engine();
        point_doubles(&mut e);
        let m = floats(&mut e, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
        let v = floats(&mut e, &[1.0, 2.0, 3.0]);
        let out = e.mem.alloc(12);
        let r = e.call(0x004b_3ae0, &args![p(out), p(v), p(m)]);
        assert_eq!(r.u32(), out);
        assert_eq!(read_floats(&e, out, 3), vec![30.0, 36.0, 42.0]);
        let r = e.call(0x004b_4500, &args![p(m), p(out), p(v)]);
        assert_eq!(r.u32(), out);
        assert_eq!(read_floats(&e, out, 3), vec![14.0, 32.0, 50.0]);
    }

    #[test]
    fn the_column_setter_and_the_zeroing_function() {
        let mut e = engine();
        let m = floats(&mut e, &[9.0; 9]);
        e.call(0x004b_4660, &args![p(m), 1u32, 1.0f32, 2.0f32, 3.0f32]);
        assert_eq!(
            read_floats(&e, m, 9),
            vec![9.0, 1.0, 9.0, 9.0, 2.0, 9.0, 9.0, 3.0, 9.0]
        );
        e.call(0x004b_4600, &args![p(m)]);
        assert_eq!(read_floats(&e, m, 9), vec![0.0; 9]);
    }

    #[test]
    fn the_inverse_divides_the_adjugate_by_the_determinant() {
        let mut e = engine();
        math_doubles(&mut e);
        let m = floats(&mut e, &[1.0, 2.0, 3.0, 0.0, 1.0, 4.0, 5.0, 6.0, 0.0]);
        let out = e.mem.alloc(36);
        assert!(e.call(0x004b_46a0, &args![p(m), p(out)]).bool());
        assert_eq!(
            read_floats(&e, out, 9),
            vec![-24.0, 18.0, 5.0, 20.0, -15.0, -4.0, -5.0, 4.0, 1.0]
        );
        let diagonal = floats(&mut e, &[2.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 5.0]);
        assert!(e.call(0x004b_46a0, &args![p(diagonal), p(out)]).bool());
        assert_close(
            &read_floats(&e, out, 9),
            &[0.5, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.2],
        );
    }

    #[test]
    fn a_singular_matrix_leaves_the_adjugate_and_the_wrapper_gives_zeros() {
        let mut e = engine();
        point_doubles(&mut e);
        math_doubles(&mut e);
        let singular = floats(&mut e, &[1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 7.0, 8.0, 9.0]);
        let out = e.mem.alloc(36);
        assert!(!e.call(0x004b_46a0, &args![p(singular), p(out)]).bool());
        // Unscaled: the first adjugate entry is m4 * m8 - m5 * m7 = 36 - 48.
        assert_eq!(e.mem.f32(out), -12.0);
        // The wrapper writes the zero matrix instead.
        let result = e.mem.alloc(36);
        let r = e.call(0x004b_45b0, &args![p(singular), p(result)]);
        assert_eq!(r.u32(), result);
        assert_eq!(read_floats(&e, result, 9), vec![0.0; 9]);
        // And the inverse of an invertible matrix.
        let diagonal = floats(&mut e, &[2.0, 0.0, 0.0, 0.0, 4.0, 0.0, 0.0, 0.0, 5.0]);
        e.call(0x004b_45b0, &args![p(diagonal), p(result)]);
        assert_close(
            &read_floats(&e, result, 9),
            &[0.5, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0, 0.0, 0.2],
        );
    }

    // ---- world space points ----

    /// A geometry object: `+0` point count, `+4` first array, `+8` second
    /// array, `+0x68` the world transform (its rotation first).
    fn geometry(e: &mut Engine, points: &[[f32; 3]], vectors: &[[f32; 3]]) -> Ptr {
        let object = e.mem.alloc(0x68 + 0x34);
        e.mem.set_u32(object, points.len() as u32);
        let flat = |rows: &[[f32; 3]]| rows.iter().flatten().copied().collect::<Vec<f32>>();
        let point_block = floats(e, &flat(points));
        let v = floats(e, &flat(vectors));
        e.mem.set_u32(object + 4, point_block);
        e.mem.set_u32(object + 8, v);
        for (i, w) in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
            .iter()
            .enumerate()
        {
            e.mem.set_f32(object + 0x68 + 4 * i as u32, *w);
        }
        p(object)
    }

    fn geometry_doubles(e: &mut Engine) {
        e.register(POINTS_FIRST, |e, a| ret_u(e.mem.u32(a[0] + 4)));
        e.register(POINTS_SECOND, |e, a| ret_u(e.mem.u32(a[0] + 8)));
        e.register(POINT_COUNT, |e, a| ret_u(e.mem.u32(a[0])));
        e.register(WORLD_TRANSFORM, |_, a| ret_u(a[0] + 0x68));
    }

    #[test]
    fn world_points_transform_the_first_array_into_a_new_block() {
        let mut e = engine();
        point_doubles(&mut e);
        geometry_doubles(&mut e);
        e.register(OPERATOR_NEW, |e, a| ret_u(e.mem.alloc(a[0])));
        e.register(VECTOR_CONSTRUCT, |_, _| Ret::default());
        e.register(TRANSFORM_POINTS, |_, _| Ret::default());
        let object = geometry(&mut e, &[[1.0; 3], [2.0; 3]], &[[0.0; 3], [0.0; 3]]);
        e.call_log = Some(vec![]);
        let block = e.call(0x004b_4910, &args![object]).u32();
        let log = take_log(&mut e);
        assert_ne!(block, 0);
        assert_eq!(calls_to(&log, OPERATOR_NEW), vec![args![24u32]]);
        assert_eq!(
            calls_to(&log, VECTOR_CONSTRUCT),
            vec![args![block, 12u32, 2u32, 0x0068_15c0u32]]
        );
        let source = e.mem.u32(object.addr() + 4);
        assert_eq!(
            calls_to(&log, TRANSFORM_POINTS),
            vec![args![2u32, source, block, object.addr() + 0x68]]
        );
        // Order: count twice, new, vector constructor, transform, source,
        // count, transform points.
        let order: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        assert_eq!(
            order,
            vec![
                0x004b_4910,
                POINT_COUNT,
                POINT_COUNT,
                OPERATOR_NEW,
                VECTOR_CONSTRUCT,
                WORLD_TRANSFORM,
                POINTS_FIRST,
                POINT_COUNT,
                TRANSFORM_POINTS
            ]
        );
    }

    #[test]
    fn world_points_of_nothing_are_null() {
        let mut e = engine();
        geometry_doubles(&mut e);
        assert_eq!(e.call(0x004b_4910, &args![Ptr::<()>::NULL]).u32(), 0);
        let empty = geometry(&mut e, &[], &[]);
        assert_eq!(e.call(0x004b_4910, &args![empty]).u32(), 0);
    }

    // ---- the snapping function ----

    struct SnapWorld {
        e: Engine,
        first: Ptr,
        second: Ptr,
        changed: Rc<RefCell<Vec<(u32, u32)>>>,
        freed: Rc<RefCell<Vec<u32>>>,
    }

    type Arrays<'a> = (&'a [[f32; 3]], &'a [[f32; 3]]);

    /// The doubles for everything the snapping function calls outside the
    /// file, with a real in-memory map; world points are the object points
    /// (identity transform).
    fn snap_world(first: Arrays, second: Arrays) -> SnapWorld {
        let mut e = engine();
        point_doubles(&mut e);
        math_doubles(&mut e);
        geometry_doubles(&mut e);
        e.register(OPERATOR_NEW, |e, a| ret_u(e.mem.alloc(a[0])));
        e.register(VECTOR_CONSTRUCT, |_, _| Ret::default());
        e.register(TRANSFORM_POINTS, |e, a| {
            // Identity transform: a plain copy of `count` points.
            copy_words(e, a[2], a[1], 3 * a[0]);
            Ret::default()
        });
        let freed = Rc::new(RefCell::new(Vec::new()));
        {
            let freed = freed.clone();
            e.register_double(OPERATOR_DELETE, move |e, a| {
                freed.borrow_mut().push(a[0]);
                e.mem.free(a[0]);
                Ret::default()
            });
        }
        e.register(TRANSFORM_CONSTRUCT, |_, _| Ret::default());
        e.register(TRANSFORM_INVERT_NON_UNIFORM, |_, _| Ret::default());
        e.register(MATRIX_PRODUCT, |e, a| {
            // The product of the identity inverse and the identity rotation.
            for i in 0..9 {
                e.mem
                    .set_f32(a[1] + 4 * i, if i % 4 == 0 { 1.0 } else { 0.0 });
            }
            ret_u(a[1])
        });
        e.register(FLOATS_NEAR, |_, a| {
            ret_u(((arg_f(a, 0) - arg_f(a, 1)).abs() <= arg_f(a, 2)) as u32)
        });
        let map: Rc<RefCell<BTreeMap<u32, [u32; 3]>>> = Rc::default();
        e.register(POINT_MAP_CONSTRUCT, |_, a| ret_u(a[0]));
        e.register(POINT_MAP_DESTRUCT, |_, _| Ret::default());
        {
            let map = map.clone();
            e.register_double(POINT_MAP_FIND, move |e, a| match map.borrow().get(&a[1]) {
                Some(value) => {
                    for (i, w) in value.iter().enumerate() {
                        e.mem.set_u32(a[2] + 4 * i as u32, *w);
                    }
                    ret_u(1)
                }
                None => ret_u(0),
            });
        }
        {
            let map = map.clone();
            e.register_double(POINT_MAP_SET_AT, move |_, a| {
                map.borrow_mut().insert(a[1], [a[2], a[3], a[4]]);
                Ret::default()
            });
        }
        {
            let map = map.clone();
            e.register_double(POINT_MAP_FIRST, move |_, _| {
                ret_u(!map.borrow().is_empty() as u32)
            });
        }
        {
            let map = map.clone();
            e.register_double(POINT_MAP_NEXT, move |e, a| {
                // The iterator cell holds 1 + the index of the entry to give.
                let index = e.mem.u32(a[1]) - 1;
                let map = map.borrow();
                let (key, value) = map.iter().nth(index as usize).unwrap();
                e.mem.set_u32(a[2], *key);
                for (i, w) in value.iter().enumerate() {
                    e.mem.set_u32(a[3] + 4 * i as u32, *w);
                }
                let next = if (index as usize) + 1 < map.len() {
                    index + 2
                } else {
                    0
                };
                e.mem.set_u32(a[1], next);
                Ret::default()
            });
        }
        e.register(GEOMETRY_DATA, |_, a| ret_u(a[0] + 0x1000));
        let changed = Rc::new(RefCell::new(Vec::new()));
        {
            let changed = changed.clone();
            e.register_double(MARK_AS_CHANGED, move |_, a| {
                changed.borrow_mut().push((a[0], a[1]));
                Ret::default()
            });
        }
        let first = geometry(&mut e, first.0, first.1);
        let second = geometry(&mut e, second.0, second.1);
        SnapWorld {
            e,
            first,
            second,
            changed,
            freed,
        }
    }

    fn snap(w: &mut SnapWorld, threshold: f32, debug: bool) -> bool {
        w.e.call(
            0x004b_3e60,
            &args![w.first, w.second, threshold, 0u32, debug],
        )
        .bool()
    }

    fn vectors_of(w: &SnapWorld, object: Ptr, n: u32) -> Vec<f32> {
        read_floats(&w.e, w.e.mem.u32(object.addr() + 8), 3 * n)
    }

    #[test]
    fn snapping_copies_the_nearest_first_vector_onto_each_second_point() {
        let mut w = snap_world(
            (
                &[[0.0; 3], [10.0, 0.0, 0.0]],
                &[[0.0, 0.0, 2.0], [0.0, 3.0, 0.0]],
            ),
            (
                &[[0.5, 0.0, 0.0], [9.5, 0.0, 0.0], [50.0, 50.0, 50.0]],
                &[[5.0, 5.0, 5.0], [6.0, 6.0, 6.0], [7.0, 7.0, 7.0]],
            ),
        );
        w.e.call_log = Some(vec![]);
        assert!(snap(&mut w, 2.0, false));
        let log = take_log(&mut w.e);
        // The vectors are the first object's, unitized; the third second point
        // (too far) keeps its own.
        assert_close(
            &vectors_of(&w, w.second, 3),
            &[0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 7.0, 7.0, 7.0],
        );
        // Only the second object's data is marked, with flag word 3.
        assert_eq!(*w.changed.borrow(), vec![(w.second.addr() + 0x1000, 3)]);
        // The world point blocks were released and the object remembered.
        assert_eq!(w.freed.borrow().len(), 2);
        assert_eq!(w.e.global::<u32>(LAST_SNAPPED_OBJECT), w.first.addr());
        // The map was built with 0x25 buckets and destroyed.
        assert_eq!(calls_to(&log, POINT_MAP_CONSTRUCT)[0][1], 0x25);
        assert_eq!(calls_to(&log, POINT_MAP_DESTRUCT).len(), 1);
        // Two entries: first 0 -> second 0 (distance 0.5), first 1 -> second 1.
        let sets = calls_to(&log, POINT_MAP_SET_AT);
        assert_eq!(sets.len(), 2);
        assert_eq!(&sets[0][1..4], &[0, 0, 3]);
        assert_eq!(f32::from_bits(sets[0][4]), 0.5);
        assert_eq!(&sets[1][1..4], &[1, 1, 3]);
    }

    #[test]
    fn a_negative_threshold_matches_at_any_distance() {
        let mut w = snap_world(
            (&[[0.0; 3]], &[[0.0, 0.0, 4.0]]),
            (&[[1000.0, 0.0, 0.0]], &[[9.0, 9.0, 9.0]]),
        );
        assert!(!snap(&mut w, 2.0, false));
        assert!(w.changed.borrow().is_empty());
        // Nothing recorded: the geometry data is untouched, but the last
        // object is still remembered.
        assert_eq!(w.e.global::<u32>(LAST_SNAPPED_OBJECT), w.first.addr());
        assert!(snap(&mut w, -1.0, false));
        assert_close(&vectors_of(&w, w.second, 1), &[0.0, 0.0, 1.0]);
    }

    #[test]
    fn two_second_points_within_the_tolerance_share_a_first_point() {
        let mut w = snap_world(
            (&[[0.0; 3]], &[[0.0, 5.0, 0.0]]),
            (
                &[[0.5, 0.0, 0.0], [0.5002, 0.0, 0.0], [0.7, 0.0, 0.0]],
                &[[1.0, 1.0, 1.0], [2.0, 2.0, 2.0], [3.0, 3.0, 3.0]],
            ),
        );
        w.e.call_log = Some(vec![]);
        assert!(snap(&mut w, 5.0, false));
        let log = take_log(&mut w.e);
        // Second point 0 (0.5) first; 1 (0.5002) is within 0.001 of it so it
        // replaces it keeping index 0 as the partner; 2 (0.7) is farther.
        let sets = calls_to(&log, POINT_MAP_SET_AT);
        assert_eq!(sets.len(), 2);
        assert_eq!(&sets[0][1..4], &[0, 0, 3]);
        assert_eq!(&sets[1][1..4], &[0, 1, 0]);
        // The entry names second point 1, and its partner 0 gets a copy.
        assert_close(
            &vectors_of(&w, w.second, 3),
            &[0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 3.0, 3.0, 3.0],
        );
    }

    #[test]
    fn a_nearer_second_point_replaces_the_entry_and_a_farther_one_does_not() {
        let mut w = snap_world(
            (&[[0.0; 3]], &[[0.0, 0.0, 6.0]]),
            (
                &[[2.0, 0.0, 0.0], [1.0, 0.0, 0.0], [3.0, 0.0, 0.0]],
                &[[1.0, 1.0, 1.0], [2.0, 2.0, 2.0], [3.0, 3.0, 3.0]],
            ),
        );
        w.e.call_log = Some(vec![]);
        assert!(snap(&mut w, 5.0, false));
        let log = take_log(&mut w.e);
        let sets = calls_to(&log, POINT_MAP_SET_AT);
        // 2.0 first; 1.0 is nearer (replaces, partner = the second count);
        // 3.0 is farther (no call).
        assert_eq!(sets.len(), 2);
        assert_eq!(&sets[1][1..4], &[0, 1, 3]);
        assert_close(
            &vectors_of(&w, w.second, 3),
            &[1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 3.0, 3.0, 3.0],
        );
    }

    #[test]
    fn snapping_in_debug_mode_moves_the_points_and_marks_both() {
        let mut w = snap_world(
            (&[[0.0; 3]], &[[0.0, 0.0, 1.0]]),
            (&[[0.5, 0.0, 0.0]], &[[9.0, 9.0, 9.0]]),
        );
        assert!(snap(&mut w, 2.0, true));
        // Each point moved by 10 times its vector: the second's by its new
        // vector (0, 0, 1), the first's too (the previous object differs).
        let second_points = read_floats(&w.e, w.e.mem.u32(w.second.addr() + 4), 3);
        assert_close(&second_points, &[0.5, 0.0, 10.0]);
        let first_points = read_floats(&w.e, w.e.mem.u32(w.first.addr() + 4), 3);
        assert_close(&first_points, &[0.0, 0.0, 10.0]);
        assert_eq!(
            *w.changed.borrow(),
            vec![(w.first.addr() + 0x1000, 3), (w.second.addr() + 0x1000, 3)]
        );
        // Run again on the same first object: its points stay put.
        assert!(snap(&mut w, 2.0, true));
        let first_points = read_floats(&w.e, w.e.mem.u32(w.first.addr() + 4), 3);
        assert_close(&first_points, &[0.0, 0.0, 10.0]);
    }

    #[test]
    fn snapping_refuses_missing_inputs_and_still_destroys_the_map() {
        let mut w = snap_world(
            (&[[0.0; 3]], &[[0.0, 0.0, 1.0]]),
            (&[[0.5, 0.0, 0.0]], &[[9.0, 9.0, 9.0]]),
        );
        w.e.call_log = Some(vec![]);
        for (first, second) in [
            (Ptr::<()>::NULL, w.second),
            (w.first, Ptr::<()>::NULL),
            (w.first, w.first),
        ] {
            let r =
                w.e.call(0x004b_3e60, &args![first, second, 2.0f32, 0u32, false])
                    .bool();
            assert!(!r);
        }
        let log = take_log(&mut w.e);
        assert_eq!(calls_to(&log, POINT_MAP_CONSTRUCT).len(), 3);
        assert_eq!(calls_to(&log, POINT_MAP_DESTRUCT).len(), 3);
        // No point arrays were asked for.
        assert!(calls_to(&log, POINTS_FIRST).is_empty());
        // An object without points.
        let empty = geometry(&mut w.e, &[], &[]);
        let r =
            w.e.call(0x004b_3e60, &args![empty, w.second, 2.0f32, 0u32, false])
                .bool();
        assert!(!r);
        let r =
            w.e.call(0x004b_3e60, &args![w.first, empty, 2.0f32, 0u32, false])
                .bool();
        assert!(!r);
        assert!(w.freed.borrow().is_empty());
    }

    // ---- bhk objects ----

    fn bhk_engine() -> Engine {
        let mut e = engine();
        e.register(BHK_REF_OBJECT_CONSTRUCT, |_, _| Ret::default());
        e.register(BHK_MOUSE_SPRING_ACTION_INIT, |_, _| Ret::default());
        e.register(BHK_SERIALIZABLE_DESTRUCT, |_, _| Ret::default());
        e.register(BHK_ACTION_DESTRUCT, |_, _| Ret::default());
        e.register(BHK_UNARY_ACTION_DESTRUCT, |_, _| Ret::default());
        e.register(SIZED_DELETE, |_, _| Ret::default());
        e
    }

    #[test]
    fn the_serializable_constructor_installs_its_vtable_and_clears_pinfo() {
        let mut e = bhk_engine();
        let this: Ptr<BhkSerializable> = e.new_object();
        e.set(this, BhkSerializable::pInfo, 0x1234);
        e.call_log = Some(vec![]);
        let r = e.call(0x004b_5120, &args![this]);
        assert_eq!(r.u32(), this.addr());
        assert_eq!(e.mem.u32(this.addr()), BHK_SERIALIZABLE_VTABLE);
        assert_eq!(e.get(this, BhkSerializable::pInfo), 0);
        assert_eq!(
            calls_to(&take_log(&mut e), BHK_REF_OBJECT_CONSTRUCT),
            vec![args![this]]
        );
    }

    #[test]
    fn each_constructor_in_the_chain_counts_its_object() {
        let mut e = bhk_engine();
        let this: Ptr<BhkSerializable> = e.new_object();
        e.call(0x004b_50f0, &args![this]);
        assert_eq!(e.mem.u32(this.addr()), BHK_ACTION_VTABLE);
        assert_eq!(e.global::<u32>(BHK_ACTION_OBJECT_COUNT), 1);
        e.call(0x004b_50c0, &args![this]);
        assert_eq!(e.mem.u32(this.addr()), BHK_UNARY_ACTION_VTABLE);
        assert_eq!(e.global::<u32>(BHK_UNARY_ACTION_OBJECT_COUNT), 1);
        assert_eq!(e.global::<u32>(BHK_ACTION_OBJECT_COUNT), 2);
        e.call_log = Some(vec![]);
        let r = e.call(0x004b_5040, &args![this, 0x77u32]);
        assert_eq!(r.u32(), this.addr());
        assert_eq!(e.mem.u32(this.addr()), BHK_MOUSE_SPRING_ACTION_VTABLE);
        assert_eq!(e.global::<u32>(BHK_MOUSE_SPRING_ACTION_OBJECT_COUNT), 1);
        assert_eq!(e.global::<u32>(BHK_UNARY_ACTION_OBJECT_COUNT), 2);
        assert_eq!(e.global::<u32>(BHK_ACTION_OBJECT_COUNT), 3);
        let log = take_log(&mut e);
        assert_eq!(
            calls_to(&log, BHK_MOUSE_SPRING_ACTION_INIT),
            vec![args![this, 0x77u32]]
        );
        // The base constructor ran before the Init call.
        let order: Vec<u32> = log.iter().map(|(a, _)| *a).collect();
        assert_eq!(
            order,
            vec![
                0x004b_5040,
                BHK_REF_OBJECT_CONSTRUCT,
                BHK_MOUSE_SPRING_ACTION_INIT
            ]
        );
    }

    #[test]
    fn get_rtti_returns_the_class_objects() {
        let mut e = bhk_engine();
        assert_eq!(
            e.call(0x004b_5150, &args![Ptr::<()>::NULL]).u32(),
            0x0126_8134
        );
        assert_eq!(
            e.call(0x004b_5190, &args![Ptr::<()>::NULL]).u32(),
            0x0126_8158
        );
        assert_eq!(
            e.call(0x004b_51d0, &args![Ptr::<()>::NULL]).u32(),
            0x0126_811c
        );
    }

    #[test]
    fn the_scalar_deleting_destructors_free_only_when_asked() {
        let mut e = bhk_engine();
        let this = p(0x4000_0000);
        for (address, destructor) in [
            (0x004b_5160, BHK_SERIALIZABLE_DESTRUCT),
            (0x004b_51a0, BHK_ACTION_DESTRUCT),
            (0x004b_51e0, BHK_UNARY_ACTION_DESTRUCT),
        ] {
            e.call_log = Some(vec![]);
            let r = e.call(address, &args![this, 0u32]);
            assert_eq!(r.u32(), this.addr());
            let log = take_log(&mut e);
            assert_eq!(calls_to(&log, destructor), vec![args![this]]);
            assert!(calls_to(&log, SIZED_DELETE).is_empty());
            e.call_log = Some(vec![]);
            e.call(address, &args![this, 1u32]);
            let log = take_log(&mut e);
            assert_eq!(calls_to(&log, SIZED_DELETE), vec![args![this, 0x10u32]]);
        }
    }
}
