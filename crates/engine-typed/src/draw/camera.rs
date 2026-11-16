//! The renderer's camera set-up (`00e6c780`), typed so that the matrices and vectors it keeps in the renderer go
//! through the draw's own state ([`super::statics::RENDERER_BLOCKS`], [`super::statics::CAMERA_EYE`]): the world draw
//! sets its camera as it starts (`004e9c90`, the renderer's slot 0x18c), and main's own scene draws after the post
//! (the world pass's extra passes) set theirs while the drawing side draws. Each side keeps its own; main's writes are
//! noted for the join.

use super::capture;
use engine_core::ptr::Ptr;
use engine_core::x87::{
    x87_add, x87_div, x87_fistp64_chop, x87_load32, x87_load64, x87_m32, x87_mul, x87_quiet_bits,
    x87_store32, x87_sub, X87Ext,
};
use engine_core::{args, Engine};

/// The byte that makes the renderer skip its frame's drawing (+0x6fc).
const SKIP: u32 = 0x6fc;
/// The renderer's device (+0x288), render target group (+0x888) and render state (+0x8b8).
const DEVICE: u32 = 0x288;
const TARGET_GROUP: u32 = 0x888;
const RENDER_STATE: u32 = 0x8b8;
/// The camera's right and up as kept (+0x634, +0x640) and as the quads take them (+0x64c, +0x658).
const RIGHT: u32 = 0x634;
const UP: u32 = 0x640;
const QUAD_RIGHT: u32 = 0x64c;
const QUAD_UP: u32 = 0x658;
/// The near plane (+0x684) and the depth range (+0x688).
const NEAR: u32 = 0x684;
const RANGE: u32 = 0x688;
/// The view (+0x980), projection (+0x9c0), view-projection (+0xa00) and inverse view (+0xa40) matrices.
const VIEW: u32 = 0x980;
const PROJECTION: u32 = 0x9c0;
const VIEW_PROJECTION: u32 = 0xa00;
const INVERSE_VIEW: u32 = 0xa40;
/// The zero point (`011f426c`), the eye kept (`011f474c`, own state) and the two bytes that choose the zero point as
/// the eye (`0126efd1`, `011a9604`), with the clip plane's height (`0126efd4`).
const ZERO_POINT: u32 = 0x011f_426c;
const EYE: u32 = super::statics::CAMERA_EYE.0;
const CLIP_FLAG: u32 = 0x0126_efd1;
const EYE_FLAG: u32 = 0x011a_9604;
const CLIP_HEIGHT: u32 = 0x0126_efd4;
/// The projection's scales (`double`s at `01011590` and `0106d4c8`), a negative side (`01012054`) and 2^32
/// (`01099378`, the unsigned conversion's correction).
const SCALE: u32 = 0x0101_1590;
const SCALE_FLIPPED: u32 = 0x0106_d4c8;
const NEGATIVE_SIDE: u32 = 0x0101_2054;
const TWO_TO_32: u32 = 0x0109_9378;
/// `D3DXVec4Transform`, `D3DXVec3Normalize`, `D3DXVec3TransformCoord`, `D3DXMatrixMultiply` (their imports).
const VEC4_TRANSFORM: u32 = 0x00ee_6e34;
const VEC3_NORMALIZE: u32 = 0x00ee_6dec;
const VEC3_TRANSFORM_COORD: u32 = 0x00ee_6df8;
const MATRIX_MULTIPLY: u32 = 0x00ee_6de0;
/// `IDirect3DDevice9::SetTransform` and `SetViewport`.
const SET_TRANSFORM: u32 = 0xb0;
const SET_VIEWPORT: u32 = 0xbc;
const ONE: u32 = 0x3f80_0000;

/// An unsigned word as a float (`FILD` and the 2^32 correction for a high bit), stored.
fn unsigned_to_float(e: &Engine, value: u32) -> u32 {
    #[allow(clippy::cast_possible_wrap)]
    let signed = value as i32;
    let mut x = f64::from(signed);
    if signed < 0 {
        x = x87_add(x, x87_m32(e.global::<u32>(TWO_TO_32)));
    }
    x87_store32(x)
}

/// `FISTP qword` under the code's chop control word, the low word kept.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn chop_low_word(value: f64) -> u32 {
    x87_fistp64_chop(X87Ext::load64(value)) as u32
}

// Typed from 00e6c780
/// The renderer's camera set-up (vtable +0x18c), `thiscall(renderer, location, direction, up, right, frustum, port)`:
/// nothing with the skip byte set. The eye is the location, or with the bytes `0126efd1` and `011a9604` set the zero
/// point (the other one is kept at `011f474c`). The view matrix (right, up, direction as columns, the eye's negated dot
/// products as the last row) and its inverse, the view sent (`SetTransform(2)`); right and up kept twice, the near
/// plane and the depth range; the projection for an orthographic or perspective frustum (its handedness from the
/// render state's slot 0x4c), with `0126efd1` its third column the clip plane through `(0, 0, [0126efd4])`; the
/// projection sent (`SetTransform(3)`), the viewport from the render target group's size and the port rectangle; the
/// view-projection made and the near and far planes to the render state's slot 0x48. The renderer's words are the
/// draw's own state; the arguments are the camera's (an image of the packet's on the drawing side).
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn set_camera_data(
    e: &mut Engine,
    this: Ptr,
    location: Ptr,
    direction: Ptr,
    up: Ptr,
    right: Ptr,
    frustum: Ptr,
    port: Ptr,
) {
    let t = this.addr();
    if e.global::<u8>(t.wrapping_add(SKIP)) != 0 {
        return;
    }
    let (loc, dir, up, right, fr, port) = (
        location.addr(),
        direction.addr(),
        up.addr(),
        right.addr(),
        frustum.addr(),
        port.addr(),
    );
    let word = |e: &Engine, a: u32, i: u32| e.global::<u32>(a.wrapping_add(4 * i));
    let eye: [u32; 3] = if e.global::<u8>(CLIP_FLAG) != 0 && e.global::<u8>(EYE_FLAG) != 0 {
        for i in 0..3 {
            let v = word(e, ZERO_POINT, i);
            capture::set_own_at(e, EYE + 4 * i, v);
        }
        [word(e, loc, 0), word(e, loc, 1), word(e, loc, 2)]
    } else {
        for i in 0..3 {
            let v = word(e, loc, i);
            capture::set_own_at(e, EYE + 4 * i, v);
        }
        [
            word(e, ZERO_POINT, 0),
            word(e, ZERO_POINT, 1),
            word(e, ZERO_POINT, 2),
        ]
    };
    let copy = |e: &Engine, a: u32, i: u32| x87_quiet_bits(word(e, a, i));
    let set = |e: &mut Engine, offset: u32, value: u32| capture::set_word(e, t, offset, value);
    for i in 0..3 {
        let values = [copy(e, right, i), copy(e, up, i), copy(e, dir, i), 0];
        for (k, v) in (0u32..).zip(values) {
            set(e, VIEW + 0x10 * i + 4 * k, v);
        }
    }
    let ld = |e: &Engine, a: u32, i: u32| x87_load32(word(e, a, i));
    let (ex, ey, ez) = (x87_load32(eye[0]), x87_load32(eye[1]), x87_load32(eye[2]));
    let along_right = x87_store32(x87_add(
        x87_add(x87_mul(ld(e, right, 0), ex), x87_mul(ld(e, right, 1), ey)),
        x87_mul(ld(e, right, 2), ez),
    ));
    set(e, VIEW + 0x30, x87_store32(-x87_load32(along_right)));
    let along_up = x87_store32(x87_add(
        x87_add(x87_mul(ld(e, up, 0), ex), x87_mul(ld(e, up, 1), ey)),
        x87_mul(ld(e, up, 2), ez),
    ));
    set(e, VIEW + 0x34, x87_store32(-x87_load32(along_up)));
    let along_dir = x87_store32(x87_add(
        x87_add(x87_mul(ld(e, dir, 1), ey), x87_mul(ld(e, dir, 0), ex)),
        x87_mul(ez, x87_m32(word(e, dir, 2))),
    ));
    set(e, VIEW + 0x38, x87_store32(-x87_load32(along_dir)));
    set(e, VIEW + 0x3c, ONE);
    for (row, source) in (0u32..).zip([right, up, dir]) {
        for i in 0..3 {
            let v = copy(e, source, i);
            set(e, INVERSE_VIEW + 0x10 * row + 4 * i, v);
        }
        set(e, INVERSE_VIEW + 0x10 * row + 0xc, 0);
    }
    for (i, v) in (0u32..).zip(eye) {
        set(e, INVERSE_VIEW + 0x30 + 4 * i, x87_quiet_bits(v));
    }
    set(e, INVERSE_VIEW + 0x3c, ONE);
    let device = e.global::<u32>(t.wrapping_add(DEVICE));
    capture::own_image(e, t.wrapping_add(VIEW), 0x40, |e, view| {
        e.vcall(device, SET_TRANSFORM, &args![2u32, view]);
    });
    for i in 0..3 {
        let r = word(e, right, i);
        set(e, QUAD_RIGHT + 4 * i, r);
        let r = word(e, right, i);
        set(e, RIGHT + 4 * i, r);
    }
    for i in 0..3 {
        let u = word(e, up, i);
        set(e, QUAD_UP + 4 * i, u);
        let u = word(e, up, i);
        set(e, UP + 4 * i, u);
    }
    // NiFrustum: left, right, top, bottom, near, far, then the byte m_bOrtho (Xbox PDB).
    set(e, NEAR, copy(e, fr, 4));
    let object = e.global::<u32>(t.wrapping_add(RENDER_STATE));
    let range = x87_store32(x87_sub(ld(e, fr, 5), x87_m32(word(e, fr, 4))));
    set(e, RANGE, range);
    let ortho = e.global::<u8>(fr.wrapping_add(0x18)) != 0;
    let width = x87_store32(x87_sub(ld(e, fr, 1), x87_m32(word(e, fr, 0))));
    let width_sum = x87_store32(x87_add(ld(e, fr, 0), x87_m32(word(e, fr, 1))));
    let height = x87_store32(x87_sub(ld(e, fr, 2), x87_m32(word(e, fr, 3))));
    let height_sum = x87_store32(x87_add(ld(e, fr, 3), x87_m32(word(e, fr, 2))));
    let inverse_range = x87_store32(x87_div(1.0, x87_load32(range)));
    let flipped = e.vcall(object, 0x4c, &args![]).u8() != 0;
    let scale = x87_load64(e.global::<u64>(SCALE));
    let w = x87_load32(width);
    let proj = PROJECTION;
    let first = if flipped {
        x87_div(f64::from_bits(e.global::<u64>(SCALE_FLIPPED)), w)
    } else {
        x87_div(scale, w)
    };
    set(e, proj, x87_store32(first));
    set(e, proj + 0x10, 0);
    if ortho {
        set(e, proj + 0x20, 0);
    }
    let sum = if flipped {
        x87_load32(width_sum)
    } else {
        -x87_load32(width_sum)
    };
    let x_offset = x87_store32(x87_div(sum, w));
    if ortho {
        set(e, proj + 0x30, x_offset);
    } else {
        set(e, proj + 0x20, x_offset);
        set(e, proj + 0x30, 0);
    }
    set(e, proj + 4, 0);
    let h = x87_load32(height);
    set(e, proj + 0x14, x87_store32(x87_div(scale, h)));
    if ortho {
        set(e, proj + 0x24, 0);
    }
    let y_offset = x87_store32(x87_div(-x87_load32(height_sum), h));
    if ortho {
        set(e, proj + 0x34, y_offset);
    } else {
        set(e, proj + 0x24, y_offset);
        set(e, proj + 0x34, 0);
    }
    set(e, proj + 8, 0);
    set(e, proj + 0x18, 0);
    let inv = x87_load32(inverse_range);
    if ortho {
        set(e, proj + 0x28, inverse_range);
        let near = x87_mul(inv, x87_m32(word(e, fr, 4)));
        set(e, proj + 0x38, x87_store32(-near));
        set(e, proj + 0xc, 0);
        set(e, proj + 0x1c, 0);
        set(e, proj + 0x2c, 0);
        set(e, proj + 0x3c, ONE);
    } else {
        let far = x87_mul(inv, x87_m32(word(e, fr, 5)));
        set(e, proj + 0x28, x87_store32(far));
        let near_far = x87_mul(ld(e, fr, 4), x87_m32(word(e, fr, 5)));
        set(e, proj + 0x38, x87_store32(-x87_mul(inv, near_far)));
        set(e, proj + 0xc, 0);
        set(e, proj + 0x1c, 0);
        set(e, proj + 0x2c, ONE);
        set(e, proj + 0x3c, 0);
    }
    let own = |e: &Engine, offset: u32| capture::own_word(e, t, offset);
    e.with_stack(0x80, |e, frame| {
        let l = frame.addr();
        if e.global::<u8>(CLIP_FLAG) != 0 {
            // The view's z axis (0, 0, 1, 0) and the point (0, 0, [0126efd4]).
            for (offset, value) in [
                (0x58, 0),
                (0x5c, 0),
                (0x64, 0),
                (0x28, 0),
                (0x2c, 0),
                (0x60, ONE),
            ] {
                e.set_global(l + offset, value);
            }
            let height = x87_quiet_bits(e.global::<u32>(CLIP_HEIGHT));
            e.set_global(l + 0x30, height);
            capture::own_image_read(e, t.wrapping_add(VIEW), 0x40, |e, view| {
                e.call(VEC4_TRANSFORM, &args![l + 0x48, l + 0x58, view]);
                e.call(VEC3_NORMALIZE, &args![l + 0x48, l + 0x48]);
                e.call(VEC3_TRANSFORM_COORD, &args![l + 0x38, l + 0x28, view]);
            });
            let n = |e: &Engine, i: u32| x87_load32(e.global::<u32>(l + 0x48 + 4 * i));
            let p = |e: &Engine, i: u32| x87_load32(e.global::<u32>(l + 0x38 + 4 * i));
            let (nx, ny, nz) = (n(e, 0), n(e, 1), n(e, 2));
            let distance = x87_store32(x87_add(
                x87_add(x87_mul(p(e, 1), ny), x87_mul(p(e, 0), nx)),
                x87_mul(p(e, 2), nz),
            ));
            let plane_w = x87_store32(-x87_load32(distance));
            e.set_global(l + 0x64, plane_w);
            let negative = x87_quiet_bits(e.global::<u32>(NEGATIVE_SIDE));
            let sign = |v: f64| {
                if 0.0 < v {
                    ONE
                } else if v < 0.0 {
                    negative
                } else {
                    0
                }
            };
            let sign_x = sign(nx);
            e.set_global(l + 0x1c, sign_x);
            let qx = x87_store32(x87_div(x87_load32(sign_x), x87_m32(own(e, proj))));
            e.set_global(l + 0x48, qx);
            let sign_y = sign(ny);
            e.set_global(l + 0x1c, sign_y);
            let qy = x87_store32(x87_div(x87_load32(sign_y), x87_m32(own(e, proj + 0x14))));
            e.set_global(l + 0x4c, qy);
            let qw = x87_store32(x87_div(
                x87_sub(1.0, x87_load32(own(e, proj + 0x28))),
                x87_load32(own(e, proj + 0x38)),
            ));
            e.set_global(l + 0x54, qw);
            let cw = x87_load32(plane_w);
            let dot = x87_store32(x87_add(
                x87_add(
                    x87_add(x87_mul(x87_load32(qy), ny), x87_mul(x87_load32(qx), nx)),
                    nz,
                ),
                x87_mul(x87_load32(qw), cw),
            ));
            let factor = x87_load32(x87_store32(x87_div(1.0, x87_load32(dot))));
            let column = [
                x87_store32(x87_mul(nx, factor)),
                x87_store32(x87_mul(ny, factor)),
                x87_store32(x87_mul(nz, factor)),
                x87_store32(x87_mul(factor, cw)),
            ];
            for (i, v) in (0u32..).zip(column) {
                e.set_global(l + 0x38 + 4 * i, v);
            }
            for (i, v) in (0u32..).zip(column) {
                set(e, proj + 8 + 0x10 * i, v);
            }
        }
        capture::own_image(e, t.wrapping_add(PROJECTION), 0x40, |e, proj| {
            e.vcall(device, SET_TRANSFORM, &args![3u32, proj]);
        });
        let group = e.global::<u32>(t.wrapping_add(TARGET_GROUP));
        let target_width = e.vcall(group, 0x8c, &args![0u32]).u32();
        let wf = x87_load32(unsigned_to_float(e, target_width));
        let group = e.global::<u32>(t.wrapping_add(TARGET_GROUP));
        let target_height = e.vcall(group, 0x90, &args![0u32]).u32();
        let hf = x87_load32(unsigned_to_float(e, target_height));
        // NiRect<float>: left, right, top, bottom.
        let x = chop_low_word(x87_mul(wf, x87_m32(word(e, port, 0))));
        e.set_global(l + 0x68, x);
        let y = chop_low_word(x87_mul(x87_sub(1.0, ld(e, port, 2)), hf));
        e.set_global(l + 0x6c, y);
        let span = x87_sub(ld(e, port, 1), x87_m32(word(e, port, 0)));
        e.set_global(l + 0x70, chop_low_word(x87_mul(wf, span)));
        let span = x87_sub(ld(e, port, 2), x87_m32(word(e, port, 3)));
        e.set_global(l + 0x74, chop_low_word(x87_mul(hf, span)));
        e.set_global(l + 0x78, 0u32);
        e.set_global(l + 0x7c, ONE);
        let device = e.global::<u32>(t.wrapping_add(DEVICE));
        e.vcall(device, SET_VIEWPORT, &args![l + 0x68]);
    });
    // View, projection and view-projection are one run of the renderer's words: one image for the product.
    capture::own_image(e, t.wrapping_add(VIEW), 0xc0, |e, view| {
        e.call(
            MATRIX_MULTIPLY,
            &args![
                view + (VIEW_PROJECTION - VIEW),
                view,
                view + (PROJECTION - VIEW)
            ],
        );
    });
    let object = e.global::<u32>(t.wrapping_add(RENDER_STATE));
    let far = copy(e, fr, 5);
    let near = copy(e, fr, 4);
    e.vcall(object, 0x48, &args![near, far]);
}
