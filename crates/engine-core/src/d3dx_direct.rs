//! Typed code's calls to the D3DX maths imports, made as Rust calls.
//!
//! The typed draw family's import thunks (`engine_typed::draw::thunks`) call nine `d3dx9_38.dll`
//! maths functions (matrix products, transposes and inverses, vector transforms and normalising,
//! plane transforms and normalising) about 3,450 times a frame on the street. By address each such
//! call packs its words, finds the import's slot in the call cache, calls the registered function
//! through the one indirect call every call by address shares, and that function unpacks the words
//! again. The functions are the platform layer's (`engine-platform`'s D3DX), which the typed crate
//! cannot name.
//!
//! So the platform layer hands the core a table of them ([`D3dxFns`], [`Layer::d3dx`]): each as
//! registered at its slot (the words form) and as itself. The functions here call the plain
//! function where the call by address would have run that very registration: the slot still holds
//! it (anything registered over the slot, a test's double or a hook, is called by address as
//! before) and no per-call diagnostic is on (the call log of the differential tests, the block
//! checks). The time slice, its hook and the engine's call stack go as for the call by address
//! ([`Engine::call_typed`]); `NV_CALL_COUNT` counts them as direct rows. So the same function runs
//! with the same arguments; only the look-up's indirect call and the packing of the words go.
//! Without the layer (an engine part crate's tests) every call goes by address.
//!
//! With real threads the D3DX maths imports run outside the kernel lock (they work only on the
//! guest memory they are given) and are registered as themselves on every thread's engine, so the
//! check holds on all of them. Nothing here is particular to an operating system.
//!
//! [`Layer::d3dx`]: crate::units::platform::Layer::d3dx

use crate::abi::AbiFn;
use crate::ptr::Ptr;
use crate::Engine;

/// `D3DXMatrixMultiplyTranspose` (import slot).
pub const MATRIX_MULTIPLY_TRANSPOSE: u32 = 0x00fd_f410;
/// `D3DXVec3TransformCoord` (import slot).
pub const VEC3_TRANSFORM_COORD: u32 = 0x00fd_f414;
/// `D3DXPlaneTransform` (import slot).
pub const PLANE_TRANSFORM: u32 = 0x00fd_f418;
/// `D3DXVec3Normalize` (import slot).
pub const VEC3_NORMALIZE: u32 = 0x00fd_f41c;
/// `D3DXVec3TransformNormal` (import slot).
pub const VEC3_TRANSFORM_NORMAL: u32 = 0x00fd_f420;
/// `D3DXMatrixMultiply` (import slot).
pub const MATRIX_MULTIPLY: u32 = 0x00fd_f424;
/// `D3DXMatrixInverse` (import slot).
pub const MATRIX_INVERSE: u32 = 0x00fd_f428;
/// `D3DXMatrixTranspose` (import slot).
pub const MATRIX_TRANSPOSE: u32 = 0x00fd_f42c;
/// `D3DXPlaneNormalize` (import slot).
pub const PLANE_NORMALIZE: u32 = 0x00fd_f430;

/// A D3DX function of two words (`out`, an input) as itself: its answer word.
pub type Fn2 = fn(&mut Engine, Ptr, Ptr) -> u32;
/// A D3DX function of three words.
pub type Fn3 = fn(&mut Engine, Ptr, Ptr, Ptr) -> u32;

/// The platform layer's D3DX maths imports, each as registered at its slot and as itself: one value
/// each, so a direct call can tell that the slot still holds this registration.
pub struct D3dxFns {
    pub matrix_multiply_transpose: (AbiFn, Fn3),
    pub vec3_transform_coord: (AbiFn, Fn3),
    pub plane_transform: (AbiFn, Fn3),
    pub vec3_normalize: (AbiFn, Fn2),
    pub vec3_transform_normal: (AbiFn, Fn3),
    pub matrix_multiply: (AbiFn, Fn3),
    pub matrix_inverse: (AbiFn, Fn3),
    pub matrix_transpose: (AbiFn, Fn2),
    pub plane_normalize: (AbiFn, Fn2),
}

#[cfg(any(test, feature = "test-support"))]
thread_local! {
    /// The calls this thread made directly (tests).
    static DIRECT_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// One direct call made (tests count them).
#[inline(always)]
fn note_direct() {
    #[cfg(any(test, feature = "test-support"))]
    DIRECT_CALLS.with(|c| c.set(c.get() + 1));
}

/// The calls this thread made directly so far (tests).
#[cfg(any(test, feature = "test-support"))]
pub fn direct_calls() -> u64 {
    DIRECT_CALLS.with(std::cell::Cell::get)
}

/// A two-word import at `slot`: directly through `pair` when the slot holds its registration, else
/// by address.
#[inline(always)]
fn call2(e: &mut Engine, slot: u32, pair: Option<(AbiFn, Fn2)>, a: Ptr, b: Ptr) -> Ptr {
    let words = crate::args![a, b];
    if let Some((expect, f)) = pair {
        return e
            .call_typed(slot, expect, &words, |e| {
                note_direct();
                f(e, a, b)
            })
            .ptr();
    }
    e.call(slot, &words).ptr()
}

/// [`call2`] for a three-word import.
#[inline(always)]
fn call3(e: &mut Engine, slot: u32, pair: Option<(AbiFn, Fn3)>, a: Ptr, b: Ptr, c: Ptr) -> Ptr {
    let words = crate::args![a, b, c];
    if let Some((expect, f)) = pair {
        return e
            .call_typed(slot, expect, &words, |e| {
                note_direct();
                f(e, a, b, c)
            })
            .ptr();
    }
    e.call(slot, &words).ptr()
}

/// The layer's table, `None` without the layer.
#[inline(always)]
fn table() -> Option<&'static D3dxFns> {
    crate::units::platform::layer_d3dx()
}

/// `D3DXMatrixMultiplyTranspose(out, m1, m2)`: its answer (`out`).
#[inline]
pub fn matrix_multiply_transpose(e: &mut Engine, out: Ptr, m1: Ptr, m2: Ptr) -> Ptr {
    let pair = table().map(|t| t.matrix_multiply_transpose);
    call3(e, MATRIX_MULTIPLY_TRANSPOSE, pair, out, m1, m2)
}

/// `D3DXVec3TransformCoord(out, v, m)`.
#[inline]
pub fn vec3_transform_coord(e: &mut Engine, out: Ptr, v: Ptr, m: Ptr) -> Ptr {
    let pair = table().map(|t| t.vec3_transform_coord);
    call3(e, VEC3_TRANSFORM_COORD, pair, out, v, m)
}

/// `D3DXPlaneTransform(out, plane, m)`.
#[inline]
pub fn plane_transform(e: &mut Engine, out: Ptr, plane: Ptr, m: Ptr) -> Ptr {
    let pair = table().map(|t| t.plane_transform);
    call3(e, PLANE_TRANSFORM, pair, out, plane, m)
}

/// `D3DXVec3Normalize(out, v)`.
#[inline]
pub fn vec3_normalize(e: &mut Engine, out: Ptr, v: Ptr) -> Ptr {
    let pair = table().map(|t| t.vec3_normalize);
    call2(e, VEC3_NORMALIZE, pair, out, v)
}

/// `D3DXVec3TransformNormal(out, v, m)`.
#[inline]
pub fn vec3_transform_normal(e: &mut Engine, out: Ptr, v: Ptr, m: Ptr) -> Ptr {
    let pair = table().map(|t| t.vec3_transform_normal);
    call3(e, VEC3_TRANSFORM_NORMAL, pair, out, v, m)
}

/// `D3DXMatrixMultiply(out, m1, m2)`.
#[inline]
pub fn matrix_multiply(e: &mut Engine, out: Ptr, m1: Ptr, m2: Ptr) -> Ptr {
    let pair = table().map(|t| t.matrix_multiply);
    call3(e, MATRIX_MULTIPLY, pair, out, m1, m2)
}

/// `D3DXMatrixInverse(out, determinant, m)` (the determinant's word may be 0).
#[inline]
pub fn matrix_inverse(e: &mut Engine, out: Ptr, determinant: Ptr, m: Ptr) -> Ptr {
    let pair = table().map(|t| t.matrix_inverse);
    call3(e, MATRIX_INVERSE, pair, out, determinant, m)
}

/// `D3DXMatrixTranspose(out, m)`.
#[inline]
pub fn matrix_transpose(e: &mut Engine, out: Ptr, m: Ptr) -> Ptr {
    let pair = table().map(|t| t.matrix_transpose);
    call2(e, MATRIX_TRANSPOSE, pair, out, m)
}

/// `D3DXPlaneNormalize(out, plane)`.
#[inline]
pub fn plane_normalize(e: &mut Engine, out: Ptr, plane: Ptr) -> Ptr {
    let pair = table().map(|t| t.plane_normalize);
    call2(e, PLANE_NORMALIZE, pair, out, plane)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::Ret;

    /// The stand-in import as registered (the words form): counts a call by address at `0x0300_0000`,
    /// answers its first word.
    fn registered(e: &mut Engine, a: &[u32]) -> Ret {
        let n = e.mem.u32(0x0300_0000);
        e.mem.set_u32(0x0300_0000, n + 1);
        Ret {
            eax: a[0],
            ..Ret::default()
        }
    }

    /// The same import as itself: counts a direct call at `0x0300_0004`, writes its second word to
    /// its first, answers its first word.
    fn itself(e: &mut Engine, out: Ptr, v: Ptr) -> u32 {
        let n = e.mem.u32(0x0300_0004);
        e.mem.set_u32(0x0300_0004, n + 1);
        e.mem.set_u32(out.addr(), v.addr());
        out.addr()
    }

    /// A double registered over the slot later (a test's or a hook): counts at `0x0300_0008`.
    fn double(e: &mut Engine, a: &[u32]) -> Ret {
        let n = e.mem.u32(0x0300_0008);
        e.mem.set_u32(0x0300_0008, n + 1);
        Ret {
            eax: a[1],
            ..Ret::default()
        }
    }

    const SLOT: u32 = 0x00fd_f41c;

    fn engine() -> (Engine, (AbiFn, Fn2), u32) {
        let mut e = Engine::new();
        e.mem.map(0x0300_0000, 0x1000);
        e.mem.map(0x00fd_f000, 0x1000);
        let pair: (AbiFn, Fn2) = (registered, itself);
        e.register(SLOT, pair.0);
        let out = e.mem.alloc(4);
        (e, pair, out)
    }

    /// The first call fills the call cache by address; the next go directly to the function itself
    /// with the same words; a double registered over the slot is called by address from then on.
    #[test]
    fn direct_while_the_slot_holds_the_registration_else_by_address() {
        let (mut e, pair, out) = engine();
        let before = direct_calls();
        let first = call2(&mut e, SLOT, Some(pair), Ptr::new(out), Ptr::new(0x55));
        assert_eq!(
            (first.addr(), e.mem.u32(0x0300_0000)),
            (out, 1),
            "by address"
        );
        let second = call2(&mut e, SLOT, Some(pair), Ptr::new(out), Ptr::new(0x66));
        assert_eq!(second.addr(), out);
        assert_eq!(
            e.mem.u32(out),
            0x66,
            "the function itself ran with the words"
        );
        assert_eq!((e.mem.u32(0x0300_0000), e.mem.u32(0x0300_0004)), (1, 1));
        assert_eq!(direct_calls() - before, 1);
        e.register(SLOT, double);
        let third = call2(&mut e, SLOT, Some(pair), Ptr::new(out), Ptr::new(0x77));
        assert_eq!(third.addr(), 0x77, "the double's answer");
        assert_eq!(
            (
                e.mem.u32(0x0300_0000),
                e.mem.u32(0x0300_0004),
                e.mem.u32(0x0300_0008)
            ),
            (1, 1, 1)
        );
        assert_eq!(direct_calls() - before, 1, "nothing more went direct");
    }

    /// With the call log on (the differential tests) and without a table, by address.
    #[test]
    fn by_address_with_the_call_log_on_or_without_the_layer() {
        let (mut e, pair, out) = engine();
        call2(&mut e, SLOT, Some(pair), Ptr::new(out), Ptr::new(0x55));
        e.call_log = Some(Vec::new());
        call2(&mut e, SLOT, Some(pair), Ptr::new(out), Ptr::new(0x66));
        let logged = e.call_log.take().expect("still on");
        assert_eq!(logged, vec![(SLOT, vec![out, 0x66])]);
        call2(&mut e, SLOT, None, Ptr::new(out), Ptr::new(0x77));
        assert_eq!((e.mem.u32(0x0300_0000), e.mem.u32(0x0300_0004)), (3, 0));
    }
}
