//! The D3DX import thunks the typed families call (group E): each is the exe's `JMP dword ptr [slot]` to a
//! `d3dx9_38.dll` function, which the platform layer answers (`engine-platform`'s D3DX, bound to the
//! import slot by its name). Typed, a typed caller reaches the import in one call instead of two (the
//! thunk's own call is gone); the platform's function stays behind the slot, since part crates do not
//! depend on `engine-platform`. Every word goes to the import unchanged and its answer comes back
//! (`stdcall`: the import pops them).

use super::FAMILY;
use engine_core::abi::OneWord;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The import slots: `D3DXMatrixMultiplyTranspose`, `D3DXVec3Normalize`, `D3DXVec3TransformNormal`,
/// `D3DXVec3TransformCoord`, `D3DXMatrixTranspose`, `D3DXMatrixInverse`, `D3DXMatrixMultiply`,
/// `D3DXPlaneNormalize` and `D3DXPlaneTransform` (the platform layer's names for them).
const MATRIX_MULTIPLY_TRANSPOSE: u32 = 0x00fd_f410;
const VEC3_NORMALIZE: u32 = 0x00fd_f41c;
const VEC3_TRANSFORM_NORMAL: u32 = 0x00fd_f420;
const VEC3_TRANSFORM_COORD: u32 = 0x00fd_f414;
const MATRIX_TRANSPOSE: u32 = 0x00fd_f42c;
const MATRIX_INVERSE: u32 = 0x00fd_f428;
const MATRIX_MULTIPLY: u32 = 0x00fd_f424;
const PLANE_NORMALIZE: u32 = 0x00fd_f430;
const PLANE_TRANSFORM: u32 = 0x00fd_f418;

// Typed from 00ee6dfe
/// `D3DXMatrixMultiplyTranspose(out, m1, m2)`'s thunk: answers the import's answer.
pub fn matrix_multiply_transpose(e: &mut Engine, out: Ptr, m1: Ptr, m2: Ptr) -> Ptr {
    e.call(MATRIX_MULTIPLY_TRANSPOSE, &args![out, m1, m2]).ptr()
}

// Typed from 00ee6dec
/// `D3DXVec3Normalize(out, v)`'s thunk.
pub fn vec3_normalize(e: &mut Engine, out: Ptr, v: Ptr) -> Ptr {
    e.call(VEC3_NORMALIZE, &args![out, v]).ptr()
}

// Typed from 00ee6df2
/// `D3DXVec3TransformNormal(out, v, m)`'s thunk.
pub fn vec3_transform_normal(e: &mut Engine, out: Ptr, v: Ptr, m: Ptr) -> Ptr {
    e.call(VEC3_TRANSFORM_NORMAL, &args![out, v, m]).ptr()
}

// Typed from 00ee6df8
/// `D3DXVec3TransformCoord(out, v, m)`'s thunk.
pub fn vec3_transform_coord(e: &mut Engine, out: Ptr, v: Ptr, m: Ptr) -> Ptr {
    e.call(VEC3_TRANSFORM_COORD, &args![out, v, m]).ptr()
}

// Typed from 00ee6dd4
/// `D3DXMatrixTranspose(out, m)`'s thunk.
pub fn matrix_transpose(e: &mut Engine, out: Ptr, m: Ptr) -> Ptr {
    e.call(MATRIX_TRANSPOSE, &args![out, m]).ptr()
}

// Typed from 00ee6dda
/// `D3DXMatrixInverse(out, determinant, m)`'s thunk (the determinant's word may be 0).
pub fn matrix_inverse(e: &mut Engine, out: Ptr, determinant: u32, m: Ptr) -> Ptr {
    e.call(MATRIX_INVERSE, &args![out, determinant, m]).ptr()
}

// Typed from 00ee6de0
/// `D3DXMatrixMultiply(out, m1, m2)`'s thunk.
pub fn matrix_multiply(e: &mut Engine, out: Ptr, m1: Ptr, m2: Ptr) -> Ptr {
    e.call(MATRIX_MULTIPLY, &args![out, m1, m2]).ptr()
}

// Typed from 00ee6dce
/// `D3DXPlaneNormalize(out, plane)`'s thunk.
pub fn plane_normalize(e: &mut Engine, out: Ptr, plane: Ptr) -> Ptr {
    e.call(PLANE_NORMALIZE, &args![out, plane]).ptr()
}

// Typed from 00ee6dc8
/// `D3DXPlaneTransform(out, plane, m)`'s thunk.
pub fn plane_transform(e: &mut Engine, out: Ptr, plane: Ptr, m: Ptr) -> Ptr {
    e.call(PLANE_TRANSFORM, &args![out, plane, m]).ptr()
}

// ---- The typed families' calls: through the typed thunk directly while the draw family is on. ----

/// A word as the thunks' pointer parameter.
#[inline(always)]
fn word(value: impl OneWord) -> Ptr {
    Ptr::new(value.one_word())
}

/// `D3DXMatrixMultiplyTranspose(out, m1, m2)` through its thunk (`00ee6dfe`).
#[inline(always)]
pub(crate) fn call_matrix_multiply_transpose(
    e: &mut Engine,
    out: impl OneWord,
    m1: impl OneWord,
    m2: impl OneWord,
) {
    let (out, m1, m2) = (word(out), word(m1), word(m2));
    let _: Ptr = typed_call!(
        FAMILY,
        0x00ee_6dfe,
        matrix_multiply_transpose(e, out, m1, m2)
    );
}

/// `D3DXVec3Normalize(out, v)` through its thunk (`00ee6dec`).
#[inline(always)]
pub(crate) fn call_vec3_normalize(e: &mut Engine, out: impl OneWord, v: impl OneWord) {
    let (out, v) = (word(out), word(v));
    let _: Ptr = typed_call!(FAMILY, 0x00ee_6dec, vec3_normalize(e, out, v));
}

/// `D3DXVec3TransformNormal(out, v, m)` through its thunk (`00ee6df2`).
#[inline(always)]
pub(crate) fn call_vec3_transform_normal(
    e: &mut Engine,
    out: impl OneWord,
    v: impl OneWord,
    m: impl OneWord,
) {
    let (out, v, m) = (word(out), word(v), word(m));
    let _: Ptr = typed_call!(FAMILY, 0x00ee_6df2, vec3_transform_normal(e, out, v, m));
}

/// `D3DXVec3TransformCoord(out, v, m)` through its thunk (`00ee6df8`).
#[inline(always)]
pub(crate) fn call_vec3_transform_coord(
    e: &mut Engine,
    out: impl OneWord,
    v: impl OneWord,
    m: impl OneWord,
) {
    let (out, v, m) = (word(out), word(v), word(m));
    let _: Ptr = typed_call!(FAMILY, 0x00ee_6df8, vec3_transform_coord(e, out, v, m));
}

/// `D3DXMatrixTranspose(out, m)` through its thunk (`00ee6dd4`).
#[inline(always)]
pub(crate) fn call_matrix_transpose(e: &mut Engine, out: impl OneWord, m: impl OneWord) {
    let (out, m) = (word(out), word(m));
    let _: Ptr = typed_call!(FAMILY, 0x00ee_6dd4, matrix_transpose(e, out, m));
}

/// `D3DXMatrixInverse(out, determinant, m)` through its thunk (`00ee6dda`).
#[inline(always)]
pub(crate) fn call_matrix_inverse(
    e: &mut Engine,
    out: impl OneWord,
    determinant: impl OneWord,
    m: impl OneWord,
) {
    let (out, determinant, m) = (word(out), determinant.one_word(), word(m));
    let _: Ptr = typed_call!(FAMILY, 0x00ee_6dda, matrix_inverse(e, out, determinant, m));
}

/// `D3DXMatrixMultiply(out, m1, m2)` through its thunk (`00ee6de0`).
#[inline(always)]
pub(crate) fn call_matrix_multiply(
    e: &mut Engine,
    out: impl OneWord,
    m1: impl OneWord,
    m2: impl OneWord,
) {
    let (out, m1, m2) = (word(out), word(m1), word(m2));
    let _: Ptr = typed_call!(FAMILY, 0x00ee_6de0, matrix_multiply(e, out, m1, m2));
}

/// `D3DXPlaneNormalize(out, plane)` through its thunk (`00ee6dce`).
#[inline(always)]
pub(crate) fn call_plane_normalize(e: &mut Engine, out: impl OneWord, plane: impl OneWord) {
    let (out, plane) = (word(out), word(plane));
    let _: Ptr = typed_call!(FAMILY, 0x00ee_6dce, plane_normalize(e, out, plane));
}

/// `D3DXPlaneTransform(out, plane, m)` through its thunk (`00ee6dc8`).
#[inline(always)]
pub(crate) fn call_plane_transform(
    e: &mut Engine,
    out: impl OneWord,
    plane: impl OneWord,
    m: impl OneWord,
) {
    let (out, plane, m) = (word(out), word(plane), word(m));
    let _: Ptr = typed_call!(FAMILY, 0x00ee_6dc8, plane_transform(e, out, plane, m));
}
