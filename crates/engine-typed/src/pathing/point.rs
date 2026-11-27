//! The point helpers the pathing code calls on every node: differences, sums, scaling, lengths and
//! the square root and absolute value wrappers under them.
//!
//! Native float maths (ADR-0014's float rule, [`super`]'s floats): a helper whose every result is one
//! operation on two floats, stored, computes in plain `f32` on every thread (the same float either
//! way); the squared lengths, sums of products, run in [`Real`] at the thread's precision.

use super::layout::Point2;
use super::{float, get, FAMILY};
use crate::culling::layout::NiPoint3;
use crate::native::{float_precision, Real};
use engine_core::ptr::{Field, Ptr};
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// `fabs` of a float (`00408860`), the result in `ST0`.
const FLOAT_ABS: u32 = 0x0040_8860;

/// The three floats of a new point, `(x, y, z)`, as `NiPoint3::NiPoint3` (`00416870`) takes them.
#[inline(always)]
fn new_point(e: &mut Engine, out: Ptr<NiPoint3>, x: f32, y: f32, z: f32) {
    typed_call!(leaf::FAMILY, 0x0041_6870, leaf::point3_new(e, out, x, y, z));
}

/// Each axis of two points combined by `op`, in plain `f32` (one operation on two floats).
#[inline(always)]
fn each(e: &Engine, a: Ptr<NiPoint3>, b: Ptr<NiPoint3>, op: impl Fn(f32, f32) -> f32) -> [f32; 3] {
    [NiPoint3::x, NiPoint3::y, NiPoint3::z].map(|f| op(e.get(a, f), e.get(b, f)))
}

// Typed from 00439ef0
/// `NiPoint3::operator-` (`thiscall(this, out, other)`): each axis `this - other` (the exe z first:
/// reads only), stored to a float and passed on, into a new point at `out`.
pub fn subtract(
    e: &mut Engine,
    this: Ptr<NiPoint3>,
    out: Ptr<NiPoint3>,
    other: Ptr<NiPoint3>,
) -> Ptr<NiPoint3> {
    let [x, y, z] = each(e, this, other, |a, b| a - b);
    new_point(e, out, x, y, z);
    out
}

// Typed from 00439e90
/// `NiPoint3::operator+` (`thiscall(this, out, other)`): as [`subtract`] with sums.
pub fn add(
    e: &mut Engine,
    this: Ptr<NiPoint3>,
    out: Ptr<NiPoint3>,
    other: Ptr<NiPoint3>,
) -> Ptr<NiPoint3> {
    let [x, y, z] = each(e, this, other, |a, b| a + b);
    new_point(e, out, x, y, z);
    out
}

// Typed from 004a3760
/// `scalar * point` (`cdecl(out, scalar, point)`): each axis `scalar * point`.
pub fn scaled(
    e: &mut Engine,
    out: Ptr<NiPoint3>,
    scalar: f32,
    point: Ptr<NiPoint3>,
) -> Ptr<NiPoint3> {
    let [x, y, z] = [NiPoint3::x, NiPoint3::y, NiPoint3::z].map(|f| scalar * e.get(point, f));
    new_point(e, out, x, y, z);
    out
}

// Typed from 00439180
/// `NiPoint3::operator*=` (`thiscall(this, scalar)`): each axis in place, x first.
pub fn scale_by(e: &mut Engine, this: Ptr<NiPoint3>, scalar: f32) -> Ptr<NiPoint3> {
    for f in [NiPoint3::x, NiPoint3::y, NiPoint3::z] {
        let v = e.get(this, f) * scalar;
        e.set(this, f, v);
    }
    this
}

// Typed from 0063c8a0
/// `NiPoint3::operator+=` (`thiscall(this, other)`): each axis in place, x first.
pub fn add_to(e: &mut Engine, this: Ptr<NiPoint3>, other: Ptr<NiPoint3>) -> Ptr<NiPoint3> {
    for f in [NiPoint3::x, NiPoint3::y, NiPoint3::z] {
        let v = e.get(this, f) + e.get(other, f);
        e.set(this, f, v);
    }
    this
}

/// `x*x + y*y` in `R`.
#[inline(always)]
fn squares_xy<R: Real, S>(e: &Engine, p: Ptr<S>, x: Field<S, f32>, y: Field<S, f32>) -> R {
    let (x, y): (R, R) = (get(e, p, x), get(e, p, y));
    x * x + y * y
}

/// `(x*x + y*y) + z*z` in `R`.
#[inline(always)]
fn squares_xyz<R: Real>(e: &Engine, p: Ptr<NiPoint3>) -> R {
    let z: R = get(e, p, NiPoint3::z);
    squares_xy::<R, _>(e, p, NiPoint3::x, NiPoint3::y) + z * z
}

/// [`squares_xy`] at the running thread's precision, stored to a float.
#[inline(always)]
pub(crate) fn squared_xy<S>(e: &Engine, p: Ptr<S>, x: Field<S, f32>, y: Field<S, f32>) -> f32 {
    if float_precision() {
        squares_xy::<f32, S>(e, p, x, y)
    } else {
        float(squares_xy::<f64, S>(e, p, x, y))
    }
}

/// [`squares_xyz`] at the running thread's precision, stored to a float.
#[inline(always)]
pub(crate) fn squared_xyz(e: &Engine, p: Ptr<NiPoint3>) -> f32 {
    if float_precision() {
        squares_xyz::<f32>(e, p)
    } else {
        float(squares_xyz::<f64>(e, p))
    }
}

// Typed from 00595c80
/// The squared length of a point's x and y (`thiscall(this)`), stored to a float.
pub fn length_xy_squared(e: &mut Engine, this: Ptr<NiPoint3>) -> f32 {
    squared_xy(e, this, NiPoint3::x, NiPoint3::y)
}

// Typed from 004a7290
/// The squared length of a point (`thiscall(this)`), stored to a float.
pub fn length_squared(e: &mut Engine, this: Ptr<NiPoint3>) -> f32 {
    squared_xyz(e, this)
}

// Typed from 00457990
/// `NiPoint3::Length` (`thiscall(this)`): the squared length stored to a float, its root
/// (`004579e0`).
pub fn length(e: &mut Engine, this: Ptr<NiPoint3>) -> f32 {
    let squared = squared_xyz(e, this);
    typed_call!(FAMILY, 0x0045_79e0, root_of(e, squared))
}

// Typed from 00589850
/// `NiPoint2::Length` (`thiscall(this)`): as [`length`] in two axes.
pub fn length_2d(e: &mut Engine, this: Ptr<Point2>) -> f32 {
    let squared = squared_xy(e, this, Point2::x, Point2::y);
    typed_call!(FAMILY, 0x0045_79e0, root_of(e, squared))
}

// Typed from 00408840
/// `std::abs` of a float (`cdecl(value)`): `fabs` (`00408860`). Kept a call: the C runtime's `fabs`
/// saves and restores the x87 control word through `_ctrlfp`, the engine's copy of it too, and
/// it is not among the hot callees.
pub fn abs(e: &mut Engine, value: f32) -> f32 {
    e.call(FLOAT_ABS, &args![value]).f32()
}

// Typed from 004579e0
/// A float's square root (`cdecl(value)`), through `004019b0`.
pub fn root_of(e: &mut Engine, value: f32) -> f32 {
    typed_call!(FAMILY, 0x0040_19b0, root(e, value))
}

// Typed from 004019b0
/// A float's square root (`cdecl(value)`), through `004019d0`.
pub fn root(e: &mut Engine, value: f32) -> f32 {
    typed_call!(FAMILY, 0x0040_19d0, root_rounded(e, value))
}

// Typed from 004019d0
/// A float's square root (`cdecl(value)`): the C runtime's `sqrt` (`00ec6054`) of the value as a
/// `double`, stored to a float. Its `FSQRT` at the thread's precision then the store round the root
/// once to a float on every thread: [`f32::sqrt`]. A negative value gives the indefinite NaN there
/// as here; the C runtime's domain error handling (`errno`) is left out.
pub fn root_rounded(_e: &mut Engine, value: f32) -> f32 {
    value.sqrt()
}

// Typed from 004a0bd0
/// The point negated (`thiscall(this, out)`): each axis loaded and its sign changed.
pub fn negated(e: &mut Engine, this: Ptr<NiPoint3>, out: Ptr<NiPoint3>) -> Ptr<NiPoint3> {
    let [x, y, z] = [NiPoint3::x, NiPoint3::y, NiPoint3::z].map(|f| -e.get(this, f));
    new_point(e, out, x, y, z);
    out
}

// Typed from 0069e0c0
/// A point on a circle (`thiscall(direction, out, radius)`): `(radius * x, radius * y, 0)` from a
/// direction's cosine and sine.
pub fn on_circle(
    e: &mut Engine,
    this: Ptr<Point2>,
    out: Ptr<NiPoint3>,
    radius: f32,
) -> Ptr<NiPoint3> {
    let y = radius * e.get(this, Point2::y);
    let x = radius * e.get(this, Point2::x);
    new_point(e, out, x, y, 0.0);
    out
}
