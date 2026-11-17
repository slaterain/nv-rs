//! The sun shadows (`d3d9_wgpu::sun_shadow`) through the whole backend on the GPU: with them off
//! the frame is bit for bit what it is without them, whatever the stream carries; with them on, a
//! box standing on the ground casts its shadow on the ground away from the sun, the ground on the
//! sun's side, the sky and what is drawn after the depth-only clear (the first-person arms) do not
//! change.
//!
//! The scene, camera and shaders are built here (no game file): a vertex shader with a constant
//! table naming `ModelViewProj` (so its draws cast), world-space vertices in vertex buffers. A
//! machine without a GPU adapter skips with a message.

use d3d9_stream::{Backend, Cmd, Data};
use d3d9_wgpu::scene_light::{self, Params};
use d3d9_wgpu::sun_shadow::{self, Frame};
use d3d9_wgpu::{Output, WgpuBackend};

const END: u32 = 0x0000_FFFF;
const DEVICE: u32 = 0x1000;
const BACK: u32 = 0x1100;
const RT: u32 = 0x1200;
const DS: u32 = 0x1210;
const VS: u32 = 0x1300;
const PS: u32 = 0x1310;
const DECL: u32 = 0x1400;
const GROUND: u32 = 0x1500;
const BOX: u32 = 0x1510;
const ARMS: u32 = 0x1520;
/// The box of [`BOX`]'s size around its own origin (placed by its `ModelViewProj`: it moves).
const MOVING_BOX: u32 = 0x1530;
/// A lit square drawn after the depth-only clear, inside the box's shadow.
const AFTER: u32 = 0x1540;
/// The cut-out: a square standing in the plane x = 0 (edge-on to the camera), its texture's alpha
/// a 2 by 2 checker, drawn with the alpha test; its shaders, declaration and texture.
const CUT: u32 = 0x1550;
const CUT_VS: u32 = 0x1560;
const CUT_PS: u32 = 0x1570;
const CUT_DECL: u32 = 0x1580;
const CUT_TEX: u32 = 0x1590;
/// The lit pixel shader: ambient (c1) plus the sun's light colour (c3), as the game's lit shaders add
/// them.
const LIT_PS: u32 = 0x1320;
const A8R8G8B8: u32 = 21;
const D24S8: u32 = 75;
const SIZE: u32 = 256;
const NEAR: f64 = 10.0;
const FAR: f64 = 10_000.0;
/// The frustum at unit distance: left, right, bottom, top (a 53-degree square view).
const HALF: f64 = 0.5;
const GREY: u32 = 0xFF80_8080;

/// One device at a time (the switch is the process's, and some adapters fail with several).
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn ins(op: u32, params: &[u32]) -> Vec<u32> {
    let mut v = vec![op | ((params.len() as u32) << 24)];
    v.extend_from_slice(params);
    v
}

fn words(w: &[u32]) -> Vec<u8> {
    w.iter().flat_map(|w| w.to_le_bytes()).collect()
}

fn floats(f: &[f32]) -> Vec<u8> {
    f.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn run(b: &mut WgpuBackend, cmd: Cmd<Data<'_>>) {
    b.execute(cmd);
}

/// The camera: at (0, -400, 300), looking toward +y and 35 degrees down; its axes (right, up,
/// ahead) in the world.
fn camera() -> ([f64; 3], [[f64; 3]; 3]) {
    let (s, c) = 35f64.to_radians().sin_cos();
    let ahead = [0.0, c, -s];
    let up = [0.0, s, c];
    ([0.0, -400.0, 300.0], [[1.0, 0.0, 0.0], up, ahead])
}

/// Toward the sun: up and toward +x, so shadows fall toward -x.
fn to_sun() -> [f64; 3] {
    let l = (0.6f64 * 0.6 + 0.8 * 0.8).sqrt();
    [0.6 / l, 0.0, 0.8 / l]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// A world point's view-space place.
fn to_view(p: [f64; 3]) -> [f64; 3] {
    let (pos, axes) = camera();
    let d = [p[0] - pos[0], p[1] - pos[1], p[2] - pos[2]];
    [dot(axes[0], d), dot(axes[1], d), dot(axes[2], d)]
}

/// The camera's `ModelViewProj` for world-space vertices: rows of clip x, y, z, w.
fn model_view_proj() -> [[f32; 4]; 4] {
    let (pos, axes) = camera();
    let mut rows = [[0.0f64; 4]; 4];
    // View rows: x_v = right . (p - pos) and so on.
    let view: Vec<[f64; 4]> = axes
        .iter()
        .map(|a| [a[0], a[1], a[2], -dot(*a, pos)])
        .collect();
    let (sx, sy) = (1.0 / HALF, 1.0 / HALF);
    let (zs, zo) = (FAR / (FAR - NEAR), -NEAR * FAR / (FAR - NEAR));
    for k in 0..4 {
        rows[0][k] = sx * view[0][k];
        rows[1][k] = sy * view[1][k];
        rows[2][k] = zs * view[2][k] + if k == 3 { zo } else { 0.0 };
        rows[3][k] = view[2][k];
    }
    rows.map(|r| r.map(|v| v as f32))
}

/// A world point's pixel on the target (centres at .5).
fn pixel(p: [f64; 3]) -> (u32, u32) {
    let v = to_view(p);
    let (x, y) = (v[0] / v[2] / HALF, v[1] / v[2] / HALF);
    let px = (x * 0.5 + 0.5) * f64::from(SIZE);
    let py = (0.5 - y * 0.5) * f64::from(SIZE);
    (px as u32, py as u32)
}

/// A constant table (`CTAB` comment) naming `ModelViewProj` at c0..c3.
fn constant_table() -> Vec<u32> {
    constant_table_for(0xFFFE_0200)
}

/// [`constant_table`] for the shader version `version`.
fn constant_table_for(version: u32) -> Vec<u32> {
    let mut b = Vec::new();
    // Header: size, creator, version, constants, their info, flags, target.
    for v in [28u32, 0, version, 1, 28, 0, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    // The constant: its name, set (float4), first register, count, reserved, type, default.
    b.extend_from_slice(&48u32.to_le_bytes());
    for v in [2u16, 0, 4, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&[0; 8]);
    b.extend_from_slice(b"ModelViewProj\0");
    b.resize(b.len().div_ceil(4) * 4, 0);
    let mut t = vec![u32::from_le_bytes(*b"CTAB")];
    t.extend(
        b.chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])),
    );
    let mut out = vec![0xFFFE | ((t.len() as u32) << 16)];
    out.extend(t);
    out
}

fn buffer(b: &mut WgpuBackend, handle: u32, corners: &[[f32; 3]]) {
    let v: Vec<f32> = corners
        .iter()
        .flat_map(|c| [c[0], c[1], c[2], 1.0])
        .collect();
    let bytes = floats(&v);
    run(
        b,
        Cmd::CreateBuffer {
            device: DEVICE,
            buffer: handle,
            index: false,
            length: bytes.len() as u32,
            usage: 0,
            format: 0,
            pool: 0,
        },
    );
    run(
        b,
        Cmd::UploadBuffer {
            buffer: handle,
            offset: 0,
            flags: 0,
            data: Data::Bytes(&bytes),
        },
    );
}

/// A box of `half` a side standing on the ground at (x, y).
fn box_corners(x: f32, y: f32, half: f32, height: f32) -> Vec<[f32; 3]> {
    let (x0, x1, y0, y1, z0, z1) = (x - half, x + half, y - half, y + half, 0.0, height);
    let quad = |a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]| vec![a, b, c, a, c, d];
    let mut v = Vec::new();
    v.extend(quad([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]));
    v.extend(quad([x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]));
    v.extend(quad([x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]));
    v.extend(quad([x0, y0, z0], [x0, y1, z0], [x0, y1, z1], [x0, y0, z1]));
    v.extend(quad([x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]));
    v
}

/// A backend with the scene's target and depth (`samples` per pixel), the casting vertex shader
/// and a pixel shader writing `c0`, the ground and the box in vertex buffers; none without an
/// adapter.
fn backend(samples: u32) -> Option<WgpuBackend> {
    let mut b = WgpuBackend::new(Output::Offscreen);
    let present = words(&[SIZE, SIZE, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
    let back = words(&[BACK]);
    b.execute(Cmd::CreateDevice {
        device: DEVICE,
        focus: 0,
        window: 0,
        behavior: 0x54,
        present: Data::Bytes(&present),
        back_buffers: Data::Bytes(&back),
        depth_stencil: 0,
    });
    if b.device().is_none() {
        eprintln!("no GPU adapter: the sun shadows' tests are skipped");
        return None;
    }
    for (surface, is_depth, format) in [(RT, false, A8R8G8B8), (DS, true, D24S8)] {
        run(
            &mut b,
            Cmd::CreateSurface {
                device: DEVICE,
                surface,
                depth: is_depth,
                width: SIZE,
                height: SIZE,
                format,
                multisample: if samples > 1 { samples } else { 0 },
                quality: 0,
                flag: 0,
            },
        );
    }
    // vs_2_0: dcl_position v0; dp4 oPos.x..w, v0, c0..c3.
    let mut vs = vec![0xFFFE_0200];
    vs.extend(constant_table());
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    for k in 0..4u32 {
        vs.extend(ins(
            9,
            &[0xC000_0000 | (0x1_0000 << k), 0x90E4_0000, 0xA0E4_0000 | k],
        ));
    }
    vs.push(END);
    let mut ps = vec![0xFFFF_0300];
    ps.extend(ins(1, &[0x800F_0800, 0xA0E4_0000]));
    ps.push(END);
    let lit = lit_pixel_shader();
    let t = words(&lit);
    run(
        &mut b,
        Cmd::CreateShader {
            device: DEVICE,
            shader: LIT_PS,
            pixel: true,
            tokens: Data::Bytes(&t),
        },
    );
    for (shader, pixel, tokens) in [(VS, false, vs), (PS, true, ps)] {
        let t = words(&tokens);
        run(
            &mut b,
            Cmd::CreateShader {
                device: DEVICE,
                shader,
                pixel,
                tokens: Data::Bytes(&t),
            },
        );
        run(
            &mut b,
            Cmd::SetShader {
                device: DEVICE,
                pixel,
                shader,
            },
        );
    }
    let decl = [0u8, 0, 0, 0, 3, 0, 0, 0, 0xFF, 0, 0, 0, 17, 0, 0, 0];
    run(
        &mut b,
        Cmd::CreateDeclaration {
            device: DEVICE,
            declaration: DECL,
            elements: Data::Bytes(&decl),
        },
    );
    run(
        &mut b,
        Cmd::SetVertexDeclaration {
            device: DEVICE,
            declaration: DECL,
        },
    );
    run(
        &mut b,
        Cmd::SetRenderTarget {
            device: DEVICE,
            index: 0,
            surface: RT,
        },
    );
    run(
        &mut b,
        Cmd::SetDepthStencilSurface {
            device: DEVICE,
            surface: DS,
        },
    );
    for (state, value) in [(22, 1), (7, 1), (14, 1)] {
        run(
            &mut b,
            Cmd::SetRenderState {
                device: DEVICE,
                state,
                value,
            },
        );
    }
    // The ground (z 0), the box (60 a side, 60 high at (0, 150)), and a square drawn after the
    // clear, near the camera (the arms).
    let (g0, g1, h0, h1) = (-400.0, 400.0, -150.0, 900.0);
    buffer(
        &mut b,
        GROUND,
        &[
            [g0, h0, 0.0],
            [g1, h0, 0.0],
            [g1, h1, 0.0],
            [g0, h0, 0.0],
            [g1, h1, 0.0],
            [g0, h1, 0.0],
        ],
    );
    buffer(&mut b, BOX, &box_corners(0.0, 150.0, 30.0, 60.0));
    let arms = |x: f32, y: f32| {
        // A point on the camera's view, 20 units ahead.
        let (pos, axes) = camera();
        let p: [f64; 3] = std::array::from_fn(|k| {
            pos[k] + 20.0 * axes[2][k] + f64::from(x) * axes[0][k] + f64::from(y) * axes[1][k]
        });
        p.map(|v| v as f32)
    };
    buffer(
        &mut b,
        ARMS,
        &[
            arms(-1.0, -8.0),
            arms(-1.0, -5.0),
            arms(1.0, -5.0),
            arms(-1.0, -8.0),
            arms(1.0, -5.0),
            arms(1.0, -8.0),
        ],
    );
    Some(b)
}

fn constants(b: &mut WgpuBackend, kind: u8, start: u32, regs: &[f32]) {
    let bytes = floats(regs);
    run(
        b,
        Cmd::SetConstants {
            device: DEVICE,
            kind,
            start,
            count: regs.len() as u32 / 4,
            data: Data::Bytes(&bytes),
        },
    );
}

fn clear(b: &mut WgpuBackend, flags: u32) {
    run(
        b,
        Cmd::Clear {
            device: DEVICE,
            count: 0,
            rects: None,
            flags,
            colour: GREY,
            z: 1.0f32.to_bits(),
            stencil: 0,
            model: 0,
        },
    );
}

fn draw(b: &mut WgpuBackend, buffer: u32, triangles: u32) {
    run(
        b,
        Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer,
            offset: 0,
            stride: 16,
        },
    );
    run(
        b,
        Cmd::DrawPrimitive {
            device: DEVICE,
            kind: 4,
            start: 0,
            count: triangles,
        },
    );
}

/// The new lighting's words (the frustum, the sun in view space) and the sun shadows' (the
/// camera's rotation and place, the sun).
fn hook_words() -> ([u32; 16], [u32; 16]) {
    let (pos, axes) = camera();
    let sun = to_sun();
    let view = Params {
        near: NEAR as f32,
        far: FAR as f32,
        left: -HALF as f32,
        right: HALF as f32,
        bottom: -HALF as f32,
        top: HALF as f32,
        sun: [dot(axes[0], sun), dot(axes[1], sun), dot(axes[2], sun)].map(|v| v as f32),
        shadow: scene_light::SHADOW_STRENGTH,
        strength: scene_light::STRENGTH,
    };
    // Gamebryo's rotation, row by row: columns ahead, up, right.
    let [right, up, ahead] = axes;
    let rotate: [f32; 9] = std::array::from_fn(|i| {
        let (r, c) = (i / 3, i % 3);
        [ahead, up, right][c][r] as f32
    });
    let frame = Frame {
        rotate,
        position: pos.map(|v| v as f32),
        to_sun: sun.map(|v| v as f32),
        strength: scene_light::SHADOW_STRENGTH,
    };
    (view.to_words(), frame.to_words())
}

/// The frame: the ground and the box, the hook's words when `words`, the depth-only clear, the
/// arms; the target's pixels (B, G, R, A).
fn frame(b: &mut WgpuBackend, words: bool) -> Vec<[u8; 4]> {
    if words {
        let (view, sun) = hook_words();
        for (state, m) in [
            (sun_shadow::VIEW_STATE, view),
            (sun_shadow::BEGIN_STATE, sun),
        ] {
            run(
                b,
                Cmd::SetTransform {
                    device: DEVICE,
                    state,
                    matrix: Data::from_words(&m),
                },
            );
        }
    }
    clear(b, 7);
    let mvp = model_view_proj();
    let regs: Vec<f32> = mvp.iter().flatten().copied().collect();
    constants(b, 0, 0, &regs);
    // 0.4 is exactly 102 of 255: no half for an adapter to round either way, and not the clear's 0x80.
    constants(b, 3, 0, &[0.4, 0.4, 0.4, 1.0]);
    draw(b, GROUND, 2);
    constants(b, 3, 0, &[0.7, 0.6, 0.5, 1.0]);
    draw(b, BOX, 10);
    if words {
        let (view, sun) = hook_words();
        for (state, m) in [(scene_light::STATE, view), (sun_shadow::STATE, sun)] {
            run(
                b,
                Cmd::SetTransform {
                    device: DEVICE,
                    state,
                    matrix: Data::from_words(&m),
                },
            );
        }
    }
    clear(b, 2);
    constants(b, 3, 0, &[0.25, 0.75, 0.5, 1.0]);
    draw(b, ARMS, 2);
    let d = b.device().expect("the device");
    let (_, _, px) = d.read_surface_pixels(RT).expect("the target's pixels");
    px.chunks_exact(4)
        .map(|p| [p[0], p[1], p[2], p[3]])
        .collect()
}

/// The frame on a new backend with the sun shadows `on` for its device; none without an adapter.
fn picture(samples: u32, on: bool, words: bool) -> Option<Vec<[u8; 4]>> {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    if on {
        sun_shadow::enable_debug();
    }
    let b = backend(samples);
    sun_shadow::enable(false);
    let mut b = b?;
    // The maps are drawn at the scene's clear from the frame before's casters: the second frame.
    frame(&mut b, words);
    present(&mut b, 1);
    Some(frame(&mut b, words))
}

fn at(px: &[[u8; 4]], (x, y): (u32, u32)) -> [u8; 4] {
    px[(y * SIZE + x) as usize]
}

/// The ground's grey in `frame` (0.4: exactly 0x66). It was a half grey, which GitHub's software
/// adapter rounds up to 0x80, the clear's colour, and the maintainer's card rounds down to 0x7F.
fn is_ground_grey(p: [u8; 4]) -> bool {
    p == [0x66, 0x66, 0x66, 0xFF]
}

/// Off (the default): the words in the stream change nothing; the frame is bit for bit the one
/// without them.
#[test]
fn off_the_frame_is_unchanged() {
    for samples in [1, 4] {
        let Some(plain) = picture(samples, false, false) else {
            return;
        };
        let with_words = picture(samples, false, true).expect("the adapter");
        assert!(plain == with_words, "{samples} samples");
        // The scene is there: the ground grey, the box drawn.
        assert!(is_ground_grey(at(&plain, pixel([-120.0, 150.0, 0.0]))));
        assert!(!is_ground_grey(at(&plain, pixel([0.0, 150.0, 60.0]))));
    }
}

/// On: the ground just past the box on the side away from the sun is darker; the ground on the
/// sun's side, the box's sunlit top, the sky and the arms are exactly as without it.
#[test]
fn on_the_box_casts_its_shadow_away_from_the_sun() {
    for samples in [1, 4] {
        let Some(plain) = picture(samples, false, false) else {
            return;
        };
        let lit = picture(samples, true, true).expect("the adapter");
        // In the shadow: the box's edge is at x -30, its top at 60; the sun at 0.6 across, 0.8 up
        // throws the shadow 45 units past it.
        for p in [
            [-50.0, 150.0, 0.0],
            [-60.0, 140.0, 0.0],
            [-45.0, 165.0, 0.0],
        ] {
            let (shadowed, before) = (at(&lit, pixel(p))[2], at(&plain, pixel(p))[2]);
            assert!(
                f32::from(shadowed) < 0.7 * f32::from(before),
                "{p:?}: {shadowed} of {before} ({samples})"
            );
        }
        // Lit: the ground on the sun's side and well past the shadow's end, the box's top.
        for p in [
            [50.0, 150.0, 0.0],
            [-110.0, 150.0, 0.0],
            [-50.0, 260.0, 0.0],
            [0.0, 150.0, 60.0],
        ] {
            assert_eq!(
                at(&lit, pixel(p)),
                at(&plain, pixel(p)),
                "{p:?} ({samples})"
            );
        }
        // The sky (the top rows, above the ground's far edge) and the arms.
        for y in 0..8 {
            for x in 0..SIZE {
                assert_eq!(at(&lit, (x, y)), at(&plain, (x, y)), "sky {x} {y}");
            }
        }
        let arms = pixel({
            let (pos, axes) = camera();
            std::array::from_fn(|k| pos[k] + 20.0 * axes[2][k] - 6.5 * axes[1][k])
        });
        assert_eq!(at(&lit, arms), at(&plain, arms), "arms ({samples})");
        assert_ne!(at(&plain, arms), at(&plain, pixel([50.0, 150.0, 0.0])));
    }
}

/// `ps_3_0` with a constant table naming `AmbientColor` (c1) and `PSLightColor` (c3):
/// `mov r0, c3; add r0, r0, c1; mov oC0, r0`.
fn lit_pixel_shader() -> Vec<u32> {
    let mut b = Vec::new();
    for v in [28u32, 0, 0xFFFF_0300, 2, 28, 0, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    let names = [("AmbientColor", 1u16), ("PSLightColor", 3)];
    let strings = 28 + 20 * names.len();
    let mut text = Vec::new();
    for (name, index) in names {
        b.extend_from_slice(&((strings + text.len()) as u32).to_le_bytes());
        for v in [2u16, index, 1, 0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&[0; 8]);
        text.extend_from_slice(name.as_bytes());
        text.push(0);
    }
    b.extend(text);
    b.resize(b.len().div_ceil(4) * 4, 0);
    let mut t = vec![0xFFFF_0300];
    let table: Vec<u32> = std::iter::once(u32::from_le_bytes(*b"CTAB"))
        .chain(
            b.chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])),
        )
        .collect();
    t.push(0xFFFE | ((table.len() as u32) << 16));
    t.extend(table);
    t.extend(ins(1, &[0x800F_0000, 0xA0E4_0003]));
    t.extend(ins(2, &[0x800F_0000, 0x80E4_0000, 0xA0E4_0001]));
    t.extend(ins(1, &[0x800F_0800, 0x80E4_0000]));
    t.push(END);
    t
}

fn present(b: &mut WgpuBackend, frame: u32) {
    run(
        b,
        Cmd::Present {
            device: DEVICE,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame,
        },
    );
}

/// Two frames of the lit scene (ambient c1 0.2, the sun's colour `sun`), each with the words the
/// platform records (at the scene's clear: the camera and frame; at the depth-only clear: the
/// same for the maps) when `words`; the second frame's pixels.
fn lit_frames(b: &mut WgpuBackend, sun: f32, words: bool) -> Vec<[u8; 4]> {
    run(
        b,
        Cmd::SetShader {
            device: DEVICE,
            pixel: true,
            shader: LIT_PS,
        },
    );
    let (view, frame) = hook_words();
    for f in 1..=3 {
        if words {
            for (state, m) in [
                (sun_shadow::VIEW_STATE, view),
                (sun_shadow::BEGIN_STATE, frame),
            ] {
                run(
                    b,
                    Cmd::SetTransform {
                        device: DEVICE,
                        state,
                        matrix: Data::from_words(&m),
                    },
                );
            }
        }
        clear(b, 7);
        let mvp = model_view_proj();
        let regs: Vec<f32> = mvp.iter().flatten().copied().collect();
        constants(b, 0, 0, &regs);
        constants(b, 3, 1, &[0.2, 0.2, 0.2, 1.0]);
        constants(b, 3, 3, &[sun, sun, sun, 0.0]);
        draw(b, GROUND, 2);
        draw(b, BOX, 10);
        if words {
            for (state, m) in [(scene_light::STATE, view), (sun_shadow::STATE, frame)] {
                run(
                    b,
                    Cmd::SetTransform {
                        device: DEVICE,
                        state,
                        matrix: Data::from_words(&m),
                    },
                );
            }
        }
        clear(b, 2);
        if f < 3 {
            present(b, f);
        }
        // The variants are made on threads of their own from the second frame (the first with the
        // maps): the third frame draws with them.
        if f == 2 {
            if let Some(d) = b.device() {
                d.wait_for_sun_variants();
            }
        }
    }
    let d = b.device().expect("the device");
    let (_, _, px) = d.read_surface_pixels(RT).expect("the target's pixels");
    px.chunks_exact(4)
        .map(|p| [p[0], p[1], p[2], p[3]])
        .collect()
}

/// The lit scene on a new backend, the sun shadows in the lighting when `on`.
fn lit_picture(samples: u32, on: bool, sun: f32) -> Option<Vec<[u8; 4]>> {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(on);
    let b = backend(samples);
    sun_shadow::enable(false);
    let mut b = b?;
    Some(lit_frames(&mut b, sun, on))
}

/// What a lit frame of [`lit_sequence`] draws besides the ground.
#[derive(Clone, Copy, Default)]
struct Extra {
    /// The box standing still at (0, 150) ([`BOX`]).
    still_box: bool,
    /// The moving box ([`MOVING_BOX`]) standing at (x, y).
    moving_box: Option<(f32, f32)>,
    /// A second copy of the moving box (the same buffer) standing at (x, y).
    other_box: Option<(f32, f32)>,
    /// The checker cut-out ([`CUT`]).
    cutout: bool,
    /// The lit square after the depth-only clear ([`AFTER`]).
    after_clear: bool,
    /// One of [`CABLES`] (its buffer `CABLE + n`).
    cable: Option<usize>,
    /// The ribbon cable ([`RIBBON`]).
    ribbon: bool,
    /// A wire and a pipe in one geometry ([`MIXED`]).
    mixed: bool,
    /// ... drawn as a cut-out (with the alpha test, its texture opaque where it is read).
    mixed_cut: bool,
}

/// Round cables along y (from y -100 to 700, six sides, rings 50 apart): (x, height, radius). Each
/// one's shadow falls on the ground at x -100 (the sun throws it 0.75 x its height toward -x).
const CABLES: [(f32, f32, f32); 4] = [
    // A fine wire 300 high: its shadow is wider than it (the sun's own width) and faint.
    (125.0, 300.0, 0.4),
    // A cable 3 wide, 120 high and 2000 high.
    (-10.0, 120.0, 1.5),
    (1400.0, 2000.0, 1.5),
    // A pipe 40 wide: no cable, a full shadow.
    (-25.0, 100.0, 20.0),
];
const CABLE: u32 = 0x15A0;
/// A ribbon cable as the game draws its telegraph lines: a flat strip 20 wide (x 115 to 135, 300
/// high, y -100 to 700) drawn with the alpha test, its texture 8 texels across with one opaque
/// (the cable, 2.5 units of the 20); its shadow falls at x -100 (the strip's middle). Its
/// buffer (position and texture coordinate) and texture.
const RIBBON: u32 = 0x15B0;
const RIBBON_TEX: u32 = 0x15C0;
/// One geometry of the fine wire of [`CABLES`] (its shadow at x -100) and a pipe 40 wide, 100 high at
/// x 35 (its shadow at x -40), as a telegraph line's poles and wires are one.
const MIXED: u32 = 0x15D0;
const MIXED_CUT: u32 = 0x15E0;

/// [`CABLES`]' corners as triangles.
fn cable_corners((x, h, r): (f32, f32, f32)) -> Vec<[f32; 3]> {
    let ring = |i: usize, s: usize| {
        let a = s as f32 * std::f32::consts::TAU / 6.0;
        [x + r * a.cos(), -100.0 + 50.0 * i as f32, h + r * a.sin()]
    };
    let mut v = Vec::new();
    for i in 0..16 {
        for s in 0..6 {
            let (a0, a1, b0, b1) = (
                ring(i, s),
                ring(i, s + 1),
                ring(i + 1, s),
                ring(i + 1, s + 1),
            );
            v.extend([a0, b0, a1, a1, b0, b1]);
        }
    }
    v
}

/// The camera's `ModelViewProj` with the object moved by (x, y) in the world first.
fn model_view_proj_at(x: f32, y: f32) -> [[f32; 4]; 4] {
    let mut m = model_view_proj();
    for row in m.iter_mut() {
        row[3] += row[0] * x + row[1] * y;
    }
    m
}

fn set_mvp(b: &mut WgpuBackend, m: [[f32; 4]; 4]) {
    let regs: Vec<f32> = m.iter().flatten().copied().collect();
    constants(b, 0, 0, &regs);
}

fn set_shaders(b: &mut WgpuBackend, vs: u32, ps: u32, decl: u32) {
    for (pixel, shader) in [(false, vs), (true, ps)] {
        run(
            b,
            Cmd::SetShader {
                device: DEVICE,
                pixel,
                shader,
            },
        );
    }
    run(
        b,
        Cmd::SetVertexDeclaration {
            device: DEVICE,
            declaration: decl,
        },
    );
}

fn state(b: &mut WgpuBackend, state: u32, value: u32) {
    run(
        b,
        Cmd::SetRenderState {
            device: DEVICE,
            state,
            value,
        },
    );
}

/// The moving box, the square after the clear and the cut-out with its shaders and checker
/// texture, made once on a backend.
fn extras(b: &mut WgpuBackend) {
    buffer(b, MOVING_BOX, &box_corners(0.0, 0.0, 30.0, 60.0));
    for (n, c) in CABLES.iter().enumerate() {
        buffer(b, CABLE + n as u32, &cable_corners(*c));
    }
    let mut mixed = cable_corners(CABLES[0]);
    mixed.extend(cable_corners((35.0, 100.0, 20.0)));
    buffer(b, MIXED, &mixed);
    // The same with a texture coordinate where the ribbon's texture is opaque.
    let v: Vec<f32> = mixed
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 1.0, 3.5 / 8.0, 0.5])
        .collect();
    let bytes = floats(&v);
    run(
        b,
        Cmd::CreateBuffer {
            device: DEVICE,
            buffer: MIXED_CUT,
            index: false,
            length: bytes.len() as u32,
            usage: 0,
            format: 0,
            pool: 0,
        },
    );
    run(
        b,
        Cmd::UploadBuffer {
            buffer: MIXED_CUT,
            offset: 0,
            flags: 0,
            data: Data::Bytes(&bytes),
        },
    );
    // The ribbon: 16 quads along y, u across x, v along y.
    let corner = |x: f32, y: f32| [x, y, 300.0, 1.0, (x - 115.0) / 20.0, (y + 100.0) / 800.0];
    let mut v = Vec::new();
    for i in 0..16 {
        let (y0, y1) = (-100.0 + 50.0 * i as f32, -50.0 + 50.0 * i as f32);
        let (a, bb, c, d) = (
            corner(115.0, y0),
            corner(135.0, y0),
            corner(135.0, y1),
            corner(115.0, y1),
        );
        v.extend([a, bb, c, a, c, d].iter().flatten().copied());
    }
    let bytes = floats(&v);
    run(
        b,
        Cmd::CreateBuffer {
            device: DEVICE,
            buffer: RIBBON,
            index: false,
            length: bytes.len() as u32,
            usage: 0,
            format: 0,
            pool: 0,
        },
    );
    run(
        b,
        Cmd::UploadBuffer {
            buffer: RIBBON,
            offset: 0,
            flags: 0,
            data: Data::Bytes(&bytes),
        },
    );
    let levels = words(&[RIBBON_TEX + 1]);
    run(
        b,
        Cmd::CreateTexture {
            device: DEVICE,
            texture: RIBBON_TEX,
            levels: Data::Bytes(&levels),
            width: 8,
            height: 1,
            usage: 0,
            format: A8R8G8B8,
            pool: 1,
        },
    );
    let texels: Vec<u8> = (0..8)
        .flat_map(|i| [0u8, 0, 0, if i == 3 { 255 } else { 0 }])
        .collect();
    run(
        b,
        Cmd::UploadSurface {
            surface: RIBBON_TEX + 1,
            row: 32,
            rows: 1,
            data: Data::Bytes(&texels),
        },
    );
    // In the plane y = 150, x -52 to -32, z 2 to 26 (about five pixels a side): every point's way
    // to the sun goes through the still box (x -30 to 30, 60 high).
    let (x0, x1, z0, z1) = (-52.0, -32.0, 2.0, 26.0);
    buffer(
        b,
        AFTER,
        &[
            [x0, 150.0, z0],
            [x1, 150.0, z0],
            [x1, 150.0, z1],
            [x0, 150.0, z0],
            [x1, 150.0, z1],
            [x0, 150.0, z1],
        ],
    );
    // The cut-out: x 0, y 100 to 200 (u 0 to 1), z 60 to 0 (v 0 to 1); position and texture
    // coordinate (24 bytes a vertex).
    let corner = |y: f32, z: f32| [0.0, y, z, 1.0, (y - 100.0) / 100.0, (60.0 - z) / 60.0];
    let (a, bb, c, d) = (
        corner(100.0, 60.0),
        corner(200.0, 60.0),
        corner(200.0, 0.0),
        corner(100.0, 0.0),
    );
    let v: Vec<f32> = [a, bb, c, a, c, d].iter().flatten().copied().collect();
    let bytes = floats(&v);
    run(
        b,
        Cmd::CreateBuffer {
            device: DEVICE,
            buffer: CUT,
            index: false,
            length: bytes.len() as u32,
            usage: 0,
            format: 0,
            pool: 0,
        },
    );
    run(
        b,
        Cmd::UploadBuffer {
            buffer: CUT,
            offset: 0,
            flags: 0,
            data: Data::Bytes(&bytes),
        },
    );
    // The checker: opaque at the top left and bottom right texels, clear at the other two.
    let levels = words(&[CUT_TEX + 1]);
    run(
        b,
        Cmd::CreateTexture {
            device: DEVICE,
            texture: CUT_TEX,
            levels: Data::Bytes(&levels),
            width: 2,
            height: 2,
            usage: 0,
            format: A8R8G8B8,
            pool: 1,
        },
    );
    let texels = [
        0u8, 0, 0, 255, 0, 0, 0, 0, //
        0, 0, 0, 0, 0, 0, 0, 255,
    ];
    run(
        b,
        Cmd::UploadSurface {
            surface: CUT_TEX + 1,
            row: 8,
            rows: 2,
            data: Data::Bytes(&texels),
        },
    );
    // vs_3_0: dcl_position v0; dcl_texcoord0 v1; dcl_position o0; dcl_texcoord0 o1;
    // dp4 o0.x..w, v0, c0..c3; mov o1, v1.
    let mut vs = vec![0xFFFE_0300];
    vs.extend(constant_table_for(0xFFFE_0300));
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(31, &[0x8000_0005, 0x900F_0001]));
    vs.extend(ins(31, &[0x8000_0000, 0xE00F_0000]));
    vs.extend(ins(31, &[0x8000_0005, 0xE00F_0001]));
    for k in 0..4u32 {
        vs.extend(ins(
            9,
            &[0xE000_0000 | (0x1_0000 << k), 0x90E4_0000, 0xA0E4_0000 | k],
        ));
    }
    vs.extend(ins(1, &[0xE00F_0001, 0x90E4_0001]));
    vs.push(END);
    // ps_3_0: dcl_texcoord0 v0; dcl_2d s0; texld r0, v0, s0; mov oC0, r0.
    let mut ps = vec![0xFFFF_0300];
    ps.extend(ins(31, &[0x8000_0005, 0x900F_0000]));
    ps.extend(ins(31, &[0x9000_0000, 0xA00F_0800]));
    ps.extend(ins(66, &[0x800F_0000, 0x90E4_0000, 0xA0E4_0800]));
    ps.extend(ins(1, &[0x800F_0800, 0x80E4_0000]));
    ps.push(END);
    for (shader, pixel, tokens) in [(CUT_VS, false, vs), (CUT_PS, true, ps)] {
        let t = words(&tokens);
        run(
            b,
            Cmd::CreateShader {
                device: DEVICE,
                shader,
                pixel,
                tokens: Data::Bytes(&t),
            },
        );
    }
    let decl = [
        0u8, 0, 0, 0, 3, 0, 0, 0, //
        0, 0, 16, 0, 1, 0, 5, 0, //
        0xFF, 0, 0, 0, 17, 0, 0, 0,
    ];
    run(
        b,
        Cmd::CreateDeclaration {
            device: DEVICE,
            declaration: CUT_DECL,
            elements: Data::Bytes(&decl),
        },
    );
}

const ALPHATESTENABLE: u32 = 15;
const ALPHAREF: u32 = 24;
const ALPHAFUNC: u32 = 25;

/// Lit frames, one for each of `frames` (each with the words the platform records when `words`),
/// the ground and what the frame's [`Extra`] says; each frame's pixels.
fn lit_sequence(b: &mut WgpuBackend, frames: &[Extra], words: bool) -> Vec<Vec<[u8; 4]>> {
    extras(b);
    let (view, frame) = hook_words();
    let mut out = Vec::new();
    for (f, e) in frames.iter().enumerate() {
        set_shaders(b, VS, LIT_PS, DECL);
        if words {
            for (state, m) in [
                (sun_shadow::VIEW_STATE, view),
                (sun_shadow::BEGIN_STATE, frame),
            ] {
                run(
                    b,
                    Cmd::SetTransform {
                        device: DEVICE,
                        state,
                        matrix: Data::from_words(&m),
                    },
                );
            }
        }
        clear(b, 7);
        set_mvp(b, model_view_proj());
        constants(b, 3, 1, &[0.2, 0.2, 0.2, 1.0]);
        constants(b, 3, 3, &[0.5, 0.5, 0.5, 0.0]);
        draw(b, GROUND, 2);
        if e.still_box {
            draw(b, BOX, 10);
        }
        if let Some((x, y)) = e.moving_box {
            set_mvp(b, model_view_proj_at(x, y));
            draw(b, MOVING_BOX, 10);
            set_mvp(b, model_view_proj());
        }
        if let Some((x, y)) = e.other_box {
            set_mvp(b, model_view_proj_at(x, y));
            draw(b, MOVING_BOX, 10);
            set_mvp(b, model_view_proj());
        }
        if e.ribbon {
            set_shaders(b, CUT_VS, CUT_PS, CUT_DECL);
            run(
                b,
                Cmd::SetTexture {
                    device: DEVICE,
                    sampler: 0,
                    texture: RIBBON_TEX,
                },
            );
            state(b, ALPHATESTENABLE, 1);
            state(b, ALPHAREF, 128);
            state(b, ALPHAFUNC, 7);
            run(
                b,
                Cmd::SetStreamSource {
                    device: DEVICE,
                    stream: 0,
                    buffer: RIBBON,
                    offset: 0,
                    stride: 24,
                },
            );
            run(
                b,
                Cmd::DrawPrimitive {
                    device: DEVICE,
                    kind: 4,
                    start: 0,
                    count: 32,
                },
            );
            state(b, ALPHATESTENABLE, 0);
            set_shaders(b, VS, LIT_PS, DECL);
        }
        if e.mixed {
            draw(b, MIXED, 2 * 16 * 6 * 2);
        }
        if e.mixed_cut {
            set_shaders(b, CUT_VS, CUT_PS, CUT_DECL);
            run(
                b,
                Cmd::SetTexture {
                    device: DEVICE,
                    sampler: 0,
                    texture: RIBBON_TEX,
                },
            );
            state(b, ALPHATESTENABLE, 1);
            state(b, ALPHAREF, 128);
            state(b, ALPHAFUNC, 7);
            run(
                b,
                Cmd::SetStreamSource {
                    device: DEVICE,
                    stream: 0,
                    buffer: MIXED_CUT,
                    offset: 0,
                    stride: 24,
                },
            );
            run(
                b,
                Cmd::DrawPrimitive {
                    device: DEVICE,
                    kind: 4,
                    start: 0,
                    count: 2 * 16 * 6 * 2,
                },
            );
            state(b, ALPHATESTENABLE, 0);
            set_shaders(b, VS, LIT_PS, DECL);
        }
        if let Some(n) = e.cable {
            draw(b, CABLE + n as u32, 16 * 6 * 2);
        }
        if e.cutout {
            set_shaders(b, CUT_VS, CUT_PS, CUT_DECL);
            run(
                b,
                Cmd::SetTexture {
                    device: DEVICE,
                    sampler: 0,
                    texture: CUT_TEX,
                },
            );
            state(b, ALPHATESTENABLE, 1);
            state(b, ALPHAREF, 128);
            state(b, ALPHAFUNC, 7);
            run(
                b,
                Cmd::SetStreamSource {
                    device: DEVICE,
                    stream: 0,
                    buffer: CUT,
                    offset: 0,
                    stride: 24,
                },
            );
            run(
                b,
                Cmd::DrawPrimitive {
                    device: DEVICE,
                    kind: 4,
                    start: 0,
                    count: 2,
                },
            );
            state(b, ALPHATESTENABLE, 0);
            set_shaders(b, VS, LIT_PS, DECL);
        }
        if words {
            for (state, m) in [(scene_light::STATE, view), (sun_shadow::STATE, frame)] {
                run(
                    b,
                    Cmd::SetTransform {
                        device: DEVICE,
                        state,
                        matrix: Data::from_words(&m),
                    },
                );
            }
        }
        clear(b, 2);
        if e.after_clear {
            draw(b, AFTER, 2);
        }
        let d = b.device().expect("the device");
        let (_, _, px) = d.read_surface_pixels(RT).expect("the target's pixels");
        out.push(
            px.chunks_exact(4)
                .map(|p| [p[0], p[1], p[2], p[3]])
                .collect(),
        );
        present(b, f as u32 + 1);
        // The variants are made on threads of their own from the second frame (the first with
        // the maps): the frames after draw with them.
        if let Some(d) = b.device() {
            d.wait_for_sun_variants();
        }
    }
    out
}

/// [`lit_sequence`] on a new backend with the sun shadows `on` (and the words with them).
fn lit_sequence_picture(samples: u32, on: bool, frames: &[Extra]) -> Option<Vec<Vec<[u8; 4]>>> {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(on);
    let b = backend(samples);
    sun_shadow::enable(false);
    let mut b = b?;
    Some(lit_sequence(&mut b, frames, on))
}

/// A moving caster's shadow goes with it: a box moved for two frames is drawn over the still
/// maps; when it is gone (the camera still, nothing else changing) its shadow is gone the next
/// frame, and the frame is exactly the one without the shadows (no ghost of where it last was,
/// nor of where it first stood).
#[test]
fn a_moving_casters_shadow_goes_when_it_does() {
    let moved = |x| Extra {
        moving_box: Some((x, 150.0)),
        ..Extra::default()
    };
    let frames = [
        moved(0.0),
        moved(10.0),
        moved(20.0),
        Extra::default(),
        Extra::default(),
        Extra::default(),
    ];
    for samples in [1, 4] {
        let Some(off) = lit_sequence_picture(samples, false, &frames) else {
            return;
        };
        let on = lit_sequence_picture(samples, true, &frames).expect("the adapter");
        // The fourth frame's maps hold the box where the third drew it (20, 150): its shadow is
        // on the ground there though the box is not drawn.
        let p = pixel([-35.0, 150.0, 0.0]);
        let (shadowed, lit) = (at(&on[3], p)[2], at(&off[3], p)[2]);
        assert!(
            f32::from(shadowed) < 0.7 * f32::from(lit),
            "the moving box's shadow: {shadowed} of {lit} ({samples})"
        );
        // Then it is gone.
        for f in [4, 5] {
            assert!(on[f] == off[f], "frame {f}: a shadow is left ({samples})");
        }
    }
}

/// A moving box whose buffer a second copy shares: the copy (out of the view, drawn in the first
/// frame) is not drawn in the frame the box moves, so that frame has two old places and one new.
/// The move is still found: the box's first place casts no shadow the next frame (only its new
/// place does).
#[test]
fn a_move_is_found_while_another_copy_leaves_the_view() {
    let frames = [
        Extra {
            moving_box: Some((0.0, 150.0)),
            other_box: Some((-700.0, 150.0)),
            ..Extra::default()
        },
        Extra {
            moving_box: Some((10.0, 150.0)),
            ..Extra::default()
        },
        Extra {
            moving_box: Some((10.0, 150.0)),
            ..Extra::default()
        },
    ];
    for samples in [1, 4] {
        let Some(off) = lit_sequence_picture(samples, false, &frames) else {
            return;
        };
        let on = lit_sequence_picture(samples, true, &frames).expect("the adapter");
        // The box at (10, 150) throws its shadow to x -65; at (0, 150) it threw it to x -75.
        let gone = pixel([-70.0, 150.0, 0.0]);
        assert_eq!(
            at(&on[2], gone),
            at(&off[2], gone),
            "the first place's shadow is left ({samples})"
        );
        let p = pixel([-45.0, 150.0, 0.0]);
        assert!(
            f32::from(at(&on[2], p)[2]) < 0.7 * f32::from(at(&off[2], p)[2]),
            "the new place's shadow ({samples})"
        );
    }
}

/// A moved object that comes to rest keeps its shadow when it settles back into the still maps
/// ([`sun_shadow::SETTLE`] frames later) and after; its first place casts nothing all along.
#[test]
fn a_moved_object_at_rest_keeps_its_shadow_when_it_settles() {
    let mut frames = vec![Extra {
        moving_box: Some((0.0, 150.0)),
        ..Extra::default()
    }];
    frames.extend(
        std::iter::repeat(Extra {
            moving_box: Some((10.0, 150.0)),
            ..Extra::default()
        })
        .take(sun_shadow::SETTLE as usize + 2),
    );
    for samples in [1, 4] {
        let Some(off) = lit_sequence_picture(samples, false, &frames) else {
            return;
        };
        let on = lit_sequence_picture(samples, true, &frames).expect("the adapter");
        // From the frame whose maps have the box at (10, 150) to the last: its shadow (to x -65)
        // is there, the first place's (to x -75) is not.
        let (shade, gone) = (pixel([-45.0, 150.0, 0.0]), pixel([-70.0, 150.0, 0.0]));
        for f in 2..frames.len() {
            assert!(
                f32::from(at(&on[f], shade)[2]) < 0.7 * f32::from(at(&off[f], shade)[2]),
                "frame {f}: the box's shadow ({samples})"
            );
            assert_eq!(
                at(&on[f], gone),
                at(&off[f], gone),
                "frame {f}: the first place's shadow ({samples})"
            );
        }
    }
}

/// A lit draw after the depth-only clear (the first-person arms and weapon, drawn with their own
/// camera) is drawn as without the shadows, even standing inside a shadow; the scene before the
/// clear is shadowed.
#[test]
fn a_lit_draw_after_the_depth_only_clear_is_as_without_the_shadows() {
    let frames = [Extra {
        still_box: true,
        after_clear: true,
        ..Extra::default()
    }; 3];
    for samples in [1, 4] {
        let Some(off) = lit_sequence_picture(samples, false, &frames) else {
            return;
        };
        let on = lit_sequence_picture(samples, true, &frames).expect("the adapter");
        let (on, off) = (&on[2], &off[2]);
        for (x, z) in [(-42.0, 14.0), (-44.0, 12.0), (-40.0, 17.0)] {
            let p = pixel([x, 150.0, z]);
            assert_eq!(at(on, p), at(off, p), "({x}, {z}) ({samples})");
            // The square is drawn: not the clear's colour (the sky row keeps it).
            assert_ne!(at(off, p), at(off, (p.0, 2)));
        }
        // The ground in the box's shadow is shadowed in the same frame.
        let p = pixel([-55.0, 140.0, 0.0]);
        assert!(
            f32::from(at(on, p)[2]) < 0.7 * f32::from(at(off, p)[2]),
            "the ground's shadow ({samples})"
        );
    }
}

/// An alpha-tested caster casts what its alpha test keeps: the checker cut-out's shadow is a
/// checker on the ground (the opaque texels' parts dark, the clear ones' lit exactly as without
/// the shadows).
#[test]
fn a_cut_out_casts_its_alpha_tests_shape() {
    let frames = [Extra {
        cutout: true,
        ..Extra::default()
    }; 3];
    for samples in [1, 4] {
        let Some(off) = lit_sequence_picture(samples, false, &frames) else {
            return;
        };
        let on = lit_sequence_picture(samples, true, &frames).expect("the adapter");
        let (on, off) = (&on[2], &off[2]);
        // A point (y, z) of the square throws its shadow to (-0.75 z, y) on the ground.
        for (y, z, opaque) in [
            (125.0, 45.0, true),
            (175.0, 15.0, true),
            (175.0, 45.0, false),
            (125.0, 15.0, false),
        ] {
            let p = pixel([-0.75 * z, y, 0.0]);
            if opaque {
                assert!(
                    f32::from(at(on, p)[2]) < 0.7 * f32::from(at(off, p)[2]),
                    "({y}, {z}) opaque ({samples})"
                );
            } else {
                assert_eq!(at(on, p), at(off, p), "({y}, {z}) clear ({samples})");
            }
        }
    }
}

/// Off, with the lit shader and the hook's words present (as a build with the switch off still
/// gets them from nowhere, but a device must not act on them): the frame is the plain one, exactly.
#[test]
fn off_the_lit_frame_is_unchanged_with_the_words() {
    for samples in [1, 4] {
        let Some(plain) = lit_picture(samples, false, 0.5) else {
            return;
        };
        let with_words = {
            let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
            sun_shadow::enable(false);
            let mut b = backend(samples).expect("the adapter");
            lit_frames(&mut b, 0.5, true)
        };
        assert!(plain == with_words, "{samples} samples");
    }
}

/// In the lighting: inside the box's shadow the ground takes exactly what it takes with the sun's
/// colour black (the ambient alone, nothing laid over it: the acceptance test); in the sun it is
/// exactly as without the shadows; the sky is untouched.
#[test]
fn in_the_lighting_a_shadow_takes_away_the_sun_and_nothing_else() {
    for samples in [1, 4] {
        let Some(plain) = lit_picture(samples, false, 0.5) else {
            return;
        };
        let sun_black = lit_picture(samples, false, 0.0).expect("the adapter");
        let shadowed = lit_picture(samples, true, 0.5).expect("the adapter");
        for p in [
            [-50.0, 150.0, 0.0],
            [-55.0, 140.0, 0.0],
            [-45.0, 160.0, 0.0],
        ] {
            assert_eq!(
                at(&shadowed, pixel(p)),
                at(&sun_black, pixel(p)),
                "{p:?} in the shadow ({samples})"
            );
            assert_ne!(at(&plain, pixel(p)), at(&sun_black, pixel(p)));
        }
        for p in [
            [50.0, 150.0, 0.0],
            [-110.0, 150.0, 0.0],
            [-50.0, 260.0, 0.0],
            [0.0, 150.0, 60.0],
        ] {
            assert_eq!(
                at(&shadowed, pixel(p)),
                at(&plain, pixel(p)),
                "{p:?} in the sun ({samples})"
            );
        }
        for x in 0..SIZE {
            assert_eq!(at(&shadowed, (x, 2)), at(&plain, (x, 2)), "sky {x}");
        }
    }
}

/// The darkest red along the ground across x -112 to -88 at y 300 (where every one of
/// [`CABLES`]' shadows falls, a pixel or two wide) in a frame.
fn darkest_under_the_cables(px: &[[u8; 4]]) -> u8 {
    (0..=24)
        .map(|i| at(px, pixel([-112.0 + f64::from(i), 300.0, 0.0]))[2])
        .min()
        .expect("pixels")
}

/// A frame's pixels (B, G, R, A).
type Pixels = Vec<[u8; 4]>;

/// The third frame of a sequence drawing cable `n` of [`CABLES`] alone, off and on.
fn cable_pictures(samples: u32, n: usize) -> Option<(Pixels, Pixels)> {
    let frames = [Extra {
        cable: Some(n),
        ..Extra::default()
    }; 3];
    let off = lit_sequence_picture(samples, false, &frames)?;
    let on = lit_sequence_picture(samples, true, &frames).expect("the adapter");
    Some((off[2].clone(), on[2].clone()))
}

/// The lit ground's red (ambient 0.2 and the sun's 0.5, both exactly representable after the
/// sum: 0.7 x 255 = 178.5 is not, so the ground is read from the frame) and the ambient alone
/// (the sun taken away: 0.2 x 255 = 51).
const AMBIENT_RED: u8 = 51;

/// A thin caster (a wire 0.8 wide, 300 high) darkens the ground by about its real share of the
/// sun, a faint band; nowhere a full shadow; the ground beside the band as without the shadows.
#[test]
fn a_wire_casts_a_faint_band_not_a_full_shadow() {
    for samples in [1, 4] {
        let Some((off, on)) = cable_pictures(samples, 0) else {
            return;
        };
        let lit = darkest_under_the_cables(&off);
        let dark = darkest_under_the_cables(&on);
        // Taken: about a fifth of the sun (0.8 over a band about 4 wide), never all of it.
        let taken = f32::from(lit - dark) / f32::from(lit - AMBIENT_RED);
        eprintln!("the wire: {dark} of {lit} (ambient {AMBIENT_RED}): {taken} of the sun taken ({samples})");
        assert!(
            (0.04..0.5).contains(&taken),
            "the wire takes {taken} of the sun ({dark} of {lit}, {samples})"
        );
        for x in [-140.0, -60.0] {
            let p = pixel([x, 300.0, 0.0]);
            assert_eq!(
                at(&on, p),
                at(&off, p),
                "beside the band at x {x} ({samples})"
            );
        }
    }
}

/// A cable's shadow is fainter the higher it hangs over the ground (the sun's own width spreads
/// it): the same cable 120 and 2000 units up, both shadows on the same ground.
#[test]
fn a_cables_shadow_fades_with_its_height() {
    for samples in [1, 4] {
        let Some((off, low)) = cable_pictures(samples, 1) else {
            return;
        };
        let (_, high) = cable_pictures(samples, 2).expect("the adapter");
        let lit = darkest_under_the_cables(&off);
        let low = lit - darkest_under_the_cables(&low);
        let high = lit - darkest_under_the_cables(&high);
        eprintln!("taken by the cable 120 high {low}, 2000 high {high}, of {lit} ({samples})");
        assert!(high > 0, "the high cable casts ({samples})");
        assert!(
            f32::from(high) < 0.75 * f32::from(low),
            "high {high} against low {low} ({samples})"
        );
        // Not a full shadow even low down: the cable is narrower than the band the map shows.
        assert!(lit - low > AMBIENT_RED, "low {low} of {lit} ({samples})");
    }
}

/// A caster wider than any cable (a pipe 40 wide) casts the full shadow it always did: the sun
/// taken away exactly in its middle.
#[test]
fn a_thick_caster_still_casts_a_full_shadow() {
    for samples in [1, 4] {
        let Some((off, on)) = cable_pictures(samples, 3) else {
            return;
        };
        let p = pixel([-100.0, 300.0, 0.0]);
        assert_eq!(at(&on, p)[2], AMBIENT_RED, "({samples})");
        assert_ne!(at(&off, p)[2], AMBIENT_RED);
    }
}

/// Off, with the cables drawn and the words in the stream: every frame bit for bit the one
/// without the words.
#[test]
fn off_the_cables_change_nothing() {
    let frames: Vec<Extra> = (0..CABLES.len())
        .map(|n| Extra {
            cable: Some(n),
            still_box: true,
            ..Extra::default()
        })
        .collect();
    for samples in [1, 4] {
        let Some(plain) = lit_sequence_picture(samples, false, &frames) else {
            return;
        };
        let with_words = {
            let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
            sun_shadow::enable(false);
            let mut b = backend(samples).expect("the adapter");
            lit_sequence(&mut b, &frames, true)
        };
        assert!(plain == with_words, "{samples} samples");
    }
}

/// A ribbon cable (as Goodsprings' telegraph lines are drawn: a strip with the cable in its
/// texture's alpha, drawn with the alpha test) casts a faint band of about the cable's share of
/// the sun, not the hard one-texel line its alpha test would cut; beside it, nothing.
#[test]
fn a_ribbon_cable_casts_a_faint_band() {
    let frames = [Extra {
        ribbon: true,
        ..Extra::default()
    }; 3];
    for samples in [1, 4] {
        let Some(off) = lit_sequence_picture(samples, false, &frames) else {
            return;
        };
        let on = lit_sequence_picture(samples, true, &frames).expect("the adapter");
        let (off, on) = (&off[2], &on[2]);
        let lit = darkest_under_the_cables(off);
        let dark = darkest_under_the_cables(on);
        let taken = f32::from(lit - dark) / f32::from(lit - AMBIENT_RED);
        eprintln!("the ribbon: {dark} of {lit}: {taken} of the sun taken ({samples})");
        assert!(
            (0.04..0.6).contains(&taken),
            "the ribbon takes {taken} of the sun ({dark} of {lit}, {samples})"
        );
        for x in [-140.0, -60.0] {
            let p = pixel([x, 300.0, 0.0]);
            assert_eq!(
                at(on, p),
                at(off, p),
                "beside the band at x {x} ({samples})"
            );
        }
    }
}

/// A wire and a pipe in one geometry (as Goodsprings' telegraph line is poles and wires in one):
/// the wire is taken out and casts its faint band, the pipe still casts its full shadow; drawn plain
/// and as a cut-out (as Goodsprings' is).
#[test]
fn a_wire_in_one_geometry_with_a_pipe_is_taken_out() {
    for (samples, cut) in [(1, false), (4, false), (1, true), (4, true)] {
        let frames = [Extra {
            mixed: !cut,
            mixed_cut: cut,
            ..Extra::default()
        }; 3];
        let Some(off) = lit_sequence_picture(samples, false, &frames) else {
            return;
        };
        let on = lit_sequence_picture(samples, true, &frames).expect("the adapter");
        let (off, on) = (&off[2], &on[2]);
        let lit = darkest_under_the_cables(off);
        let taken = f32::from(lit - darkest_under_the_cables(on)) / f32::from(lit - AMBIENT_RED);
        assert!(
            (0.04..0.5).contains(&taken),
            "the wire takes {taken} of the sun ({samples}, cut-out {cut})"
        );
        let p = pixel([-40.0, 300.0, 0.0]);
        assert_eq!(at(on, p)[2], AMBIENT_RED, "the pipe's shadow ({samples}, cut-out {cut})");
    }
}
