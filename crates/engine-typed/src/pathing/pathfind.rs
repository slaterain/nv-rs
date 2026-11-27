//! The pathfind solvers' random-angle helpers (`006d1600`, `006d1620`).

use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

use crate::native::to_float;

use super::FAMILY;

/// Callees and data.
const RANDOM_OBJECT: u32 = 0x0047_6c00;
const RANDOM_UNSIGNED: u32 = 0x0047_6be0;
const RANDOM_ANGLE_SCALE: u32 = 0x0101_e5a0;
const PI_AS_FLOAT: u32 = 0x0101_ff40;

// Typed from 006d1600
/// A random angle: [`random_angle_on`] on the global random generator.
pub fn random_angle(e: &mut Engine) -> f32 {
    let random = e.call(RANDOM_OBJECT, &args![]).u32();
    typed_call!(FAMILY, 0x006d_1620, random_angle_on(e, Ptr::new(random)))
}

// Typed from 006d1620
/// A random angle in `[-pi, pi)`: a random unsigned number times the scale at `0101e5a0`, minus
/// `pi` (`0101ff40`), stored as a `float`.
pub fn random_angle_on(e: &mut Engine, this: Ptr) -> f32 {
    let random = e.call(RANDOM_UNSIGNED, &args![this]).u32();
    let scale = e.global::<f64>(RANDOM_ANGLE_SCALE);
    let pi = e.global::<f64>(PI_AS_FLOAT);
    to_float(f64::from(random) * scale - pi)
}
