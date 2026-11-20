//! The sun shadows' characters (`d3d9_wgpu::sun_shadow::Skin`) through the whole backend on the
//! GPU: a skinned caster (a vertex shader naming `SkinModelViewProj` and `Bones`, as the game's
//! SKIN and skinned SLS families do) casts where its bones put it this frame, not where its bind
//! pose is; a moving one's shadow follows it with nothing left behind; with the characters off it
//! casts nothing (as before them); with the sun shadows off the frame is bit for bit the frame
//! without the words.
//!
//! The scene and shaders are built here (no game file). The skinned shader blends two bones by
//! the vertex's two weights (`p = w.x Bones[0] v + w.y Bones[1] v`, the game's order: weight x
//! goes with the first byte of `BLENDINDICES`, y with the second). A machine without a GPU
//! adapter skips with a message.

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
const SKIN_VS: u32 = 0x1301;
const LIT_PS: u32 = 0x1320;
const DECL: u32 = 0x1400;
const SKIN_DECL: u32 = 0x1401;
const GROUND: u32 = 0x1500;
/// The far view's ground: x -8,000 to 8,000, y 0 to 20,000.
const FAR_GROUND: u32 = 0x1501;
const BAR: u32 = 0x1510;
/// A rigid box where a head would be (60 to 120 high at (0, 150)), its face pixel shader, the
/// pixel shader with the game's actor shadow (s6), and the black texture bound there.
const HEAD: u32 = 0x1520;
const FACE_PS: u32 = 0x1321;
const GAME_PS: u32 = 0x1322;
const BLACK: u32 = 0x1600;
const A8R8G8B8: u32 = 21;
const D24S8: u32 = 75;
const SIZE: u32 = 256;
const NEAR: f64 = 10.0;
const FAR: f64 = 10_000.0;
const HALF: f64 = 0.5;
const GREY: u32 = 0xFF80_8080;
/// The bar: 20 units square, 120 high at (0, 150); its lower half moved by bone 0, its upper half
/// by bone 1.
const BAR_Y: f32 = 150.0;
const BAR_HALF: f32 = 10.0;
const JOINT: f32 = 60.0;
const TOP: f32 = 120.0;

/// One device at a time (the switches are the process's).
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
/// ahead).
fn camera() -> ([f64; 3], [[f64; 3]; 3]) {
    if FAR_VIEW.get() {
        // At (0, 0, 3000), looking toward +y and 10 degrees down: the ground 4,000 to 9,000 units
        // ahead, in the last cascade.
        let (s, c) = 10f64.to_radians().sin_cos();
        return (
            [0.0, 0.0, 3000.0],
            [[1.0, 0.0, 0.0], [0.0, s, c], [0.0, c, -s]],
        );
    }
    let (s, c) = 35f64.to_radians().sin_cos();
    (
        [0.0, -400.0, 300.0],
        [[1.0, 0.0, 0.0], [0.0, s, c], [0.0, c, -s]],
    )
}

/// Toward the sun: up and toward +x, so shadows fall toward -x (45 units across for 60 up).
fn to_sun() -> [f64; 3] {
    SUN.get()
}

thread_local! {
    /// Toward the sun for this test's thread (each test runs on a thread of its own), and the
    /// culling the characters are drawn with (`D3DRS_CULLMODE`: 1 none, 2 clockwise, 3
    /// counterclockwise).
    static SUN: std::cell::Cell<[f64; 3]> = const { std::cell::Cell::new([0.6, 0.0, 0.8]) };
    static CULL: std::cell::Cell<u32> = const { std::cell::Cell::new(1) };
    /// The characters' allowance a test sets (game units; none: measured).
    static ALLOW: std::cell::Cell<Option<f32>> = const { std::cell::Cell::new(None) };
    /// The far view ([`camera`]): the camera high over the far ground ([`FAR_GROUND`]).
    static FAR_VIEW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn model_view_proj() -> [[f32; 4]; 4] {
    let (pos, axes) = camera();
    let view: Vec<[f64; 4]> = axes
        .iter()
        .map(|a| [a[0], a[1], a[2], -dot(*a, pos)])
        .collect();
    let (zs, zo) = (FAR / (FAR - NEAR), -NEAR * FAR / (FAR - NEAR));
    let mut rows = [[0.0f64; 4]; 4];
    for k in 0..4 {
        rows[0][k] = view[0][k] / HALF;
        rows[1][k] = view[1][k] / HALF;
        rows[2][k] = zs * view[2][k] + if k == 3 { zo } else { 0.0 };
        rows[3][k] = view[2][k];
    }
    rows.map(|r| r.map(|v| v as f32))
}

fn pixel(p: [f64; 3]) -> (u32, u32) {
    let (pos, axes) = camera();
    let d = [p[0] - pos[0], p[1] - pos[1], p[2] - pos[2]];
    let v = [dot(axes[0], d), dot(axes[1], d), dot(axes[2], d)];
    let (x, y) = (v[0] / v[2] / HALF, v[1] / v[2] / HALF);
    (
        ((x * 0.5 + 0.5) * f64::from(SIZE)) as u32,
        ((0.5 - y * 0.5) * f64::from(SIZE)) as u32,
    )
}

/// A constant table (`CTAB` comment) of float constants `names` (name, first register, count) for
/// shader version `version`.
fn constant_table(version: u32, names: &[(&str, u16, u16)]) -> Vec<u32> {
    let mut b = Vec::new();
    for v in [28u32, 0, version, names.len() as u32, 28, 0, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    let strings = 28 + 20 * names.len();
    let mut text = Vec::new();
    for &(name, index, count) in names {
        b.extend_from_slice(&((strings + text.len()) as u32).to_le_bytes());
        // A sampler (`...Map`) is in register set 3, a float constant in set 2.
        let set = if name.ends_with("Map") || name.ends_with("Map0") {
            3u16
        } else {
            2
        };
        for v in [set, index, count, 0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&[0; 8]);
        text.extend_from_slice(name.as_bytes());
        text.push(0);
    }
    b.extend(text);
    b.resize(b.len().div_ceil(4) * 4, 0);
    let table: Vec<u32> = std::iter::once(u32::from_le_bytes(*b"CTAB"))
        .chain(
            b.chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])),
        )
        .collect();
    let mut t = vec![0xFFFE | ((table.len() as u32) << 16)];
    t.extend(table);
    t
}

/// `vs_2_0`, rigid: `oPos = ModelViewProj (c0..c3) v0`.
fn rigid_vs() -> Vec<u32> {
    let mut vs = vec![0xFFFE_0200];
    vs.extend(constant_table(0xFFFE_0200, &[("ModelViewProj", 0, 4)]));
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    for k in 0..4u32 {
        vs.extend(ins(
            9,
            &[0xC000_0000 | (0x1_0000 << k), 0x90E4_0000, 0xA0E4_0000 | k],
        ));
    }
    vs.push(END);
    vs
}

/// `vs_2_0`, skinned as the game's shaders are, with two bones: `r0 = Bones[0] v0` (c4..c6),
/// `r1 = Bones[1] v0` (c7..c9), `r0.xyz = r0 w.x + r1 w.y`, `r0.w = v0.w`, `oPos =
/// SkinModelViewProj (c0..c3) r0`.
fn skinned_vs() -> Vec<u32> {
    let mut vs = vec![0xFFFE_0200];
    vs.extend(constant_table(
        0xFFFE_0200,
        &[("SkinModelViewProj", 0, 4), ("Bones", 4, 6)],
    ));
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(31, &[0x8000_0001, 0x900F_0001]));
    vs.extend(ins(31, &[0x8000_0002, 0x900F_0002]));
    for (r, first) in [(0u32, 4u32), (1, 7)] {
        for k in 0..3u32 {
            vs.extend(ins(
                9,
                &[
                    0x8000_0000 | (0x1_0000 << k) | r,
                    0x90E4_0000,
                    0xA0E4_0000 | (first + k),
                ],
            ));
        }
    }
    vs.extend(ins(1, &[0x8008_0000, 0x90FF_0000]));
    vs.extend(ins(5, &[0x8007_0000, 0x80E4_0000, 0x9000_0001]));
    vs.extend(ins(
        4,
        &[0x8007_0000, 0x80E4_0001, 0x9055_0001, 0x80E4_0000],
    ));
    for k in 0..4u32 {
        vs.extend(ins(
            9,
            &[0xC000_0000 | (0x1_0000 << k), 0x80E4_0000, 0xA0E4_0000 | k],
        ));
    }
    vs.push(END);
    vs
}

/// [`lit_ps`] whose constant table also names `FaceGenMap0` (a character's face).
fn face_ps() -> Vec<u32> {
    let mut t = vec![0xFFFF_0300];
    t.extend(constant_table(
        0xFFFF_0300,
        &[
            ("AmbientColor", 1, 1),
            ("PSLightColor", 3, 1),
            ("FaceGenMap0", 2, 1),
        ],
    ));
    t.extend(ins(1, &[0x800F_0000, 0xA0E4_0003]));
    t.extend(ins(2, &[0x800F_0000, 0x80E4_0000, 0xA0E4_0001]));
    t.extend(ins(1, &[0x800F_0800, 0x80E4_0000]));
    t.push(END);
    t
}

/// `ps_3_0` lit as the game's SLS shaders take their actor shadow: `dcl_2d s6; texld r1, c0, s6;
/// mul r0, c3, r1; add r0, r0, c1; mov oC0, r0` (the sun's colour times the actor shadow map).
fn game_ps() -> Vec<u32> {
    let mut t = vec![0xFFFF_0300];
    t.extend(constant_table(
        0xFFFF_0300,
        &[
            ("AmbientColor", 1, 1),
            ("PSLightColor", 3, 1),
            ("ShadowMap", 6, 1),
        ],
    ));
    t.extend(ins(31, &[0x9000_0000, 0xA00F_0806]));
    t.extend(ins(66, &[0x800F_0001, 0xA0E4_0000, 0xA0E4_0806]));
    t.extend(ins(5, &[0x800F_0000, 0xA0E4_0003, 0x80E4_0001]));
    t.extend(ins(2, &[0x800F_0000, 0x80E4_0000, 0xA0E4_0001]));
    t.extend(ins(1, &[0x800F_0800, 0x80E4_0000]));
    t.push(END);
    t
}

/// `ps_3_0` naming `AmbientColor` (c1) and `PSLightColor` (c3): `oC0 = c3 + c1`.
fn lit_ps() -> Vec<u32> {
    let mut t = vec![0xFFFF_0300];
    t.extend(constant_table(
        0xFFFF_0300,
        &[("AmbientColor", 1, 1), ("PSLightColor", 3, 1)],
    ));
    t.extend(ins(1, &[0x800F_0000, 0xA0E4_0003]));
    t.extend(ins(2, &[0x800F_0000, 0x80E4_0000, 0xA0E4_0001]));
    t.extend(ins(1, &[0x800F_0800, 0x80E4_0000]));
    t.push(END);
    t
}

fn bytes_buffer(b: &mut WgpuBackend, handle: u32, bytes: &[u8]) {
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
            data: Data::Bytes(bytes),
        },
    );
}

/// The bar's triangles (a box from `z0` to `z1`) as skinned vertices: position, the two weights
/// (all on `bone`), the bone bytes (0, 1) in `D3DCOLOR` order; 28 bytes a vertex.
/// The bar's box from `z0` to `z1` high, closed: each face counterclockwise seen from outside (in
/// the game's right-handed world), so one culling mode draws its outside alone.
fn box_part(z0: f32, z1: f32) -> Vec<[f32; 3]> {
    let (x0, x1, y0, y1) = (-BAR_HALF, BAR_HALF, BAR_Y - BAR_HALF, BAR_Y + BAR_HALF);
    let quad = |a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]| vec![a, b, c, a, c, d];
    let mut v = Vec::new();
    v.extend(quad([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]));
    v.extend(quad([x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [x1, y0, z0]));
    v.extend(quad([x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]));
    v.extend(quad([x1, y1, z0], [x0, y1, z0], [x0, y1, z1], [x1, y1, z1]));
    v.extend(quad([x0, y1, z0], [x0, y0, z0], [x0, y0, z1], [x0, y1, z1]));
    v.extend(quad([x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]));
    v
}

fn bar_part(z0: f32, z1: f32, bone: usize, out: &mut Vec<u8>) {
    let v = box_part(z0, z1);
    let w = if bone == 0 { [1.0, 0.0] } else { [0.0, 1.0] };
    for p in v {
        out.extend(floats(&[p[0], p[1], p[2], 1.0, w[0], w[1]]));
        out.extend([0u8, 1, 0, 0]);
    }
}

fn backend() -> Option<WgpuBackend> {
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
        eprintln!("no GPU adapter: the characters' tests are skipped");
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
                multisample: 0,
                quality: 0,
                flag: 0,
            },
        );
    }
    for (shader, pixel, tokens) in [
        (VS, false, rigid_vs()),
        (SKIN_VS, false, skinned_vs()),
        (LIT_PS, true, lit_ps()),
        (FACE_PS, true, face_ps()),
        (GAME_PS, true, game_ps()),
    ] {
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
    }
    let decl = [0u8, 0, 0, 0, 3, 0, 0, 0, 0xFF, 0, 0, 0, 17, 0, 0, 0];
    // Position (float4), blend weights (float2, usage 1), blend indices (D3DCOLOR, usage 2).
    let skin_decl = [
        0u8, 0, 0, 0, 3, 0, 0, 0, 0, 0, 16, 0, 1, 0, 1, 0, 0, 0, 24, 0, 4, 0, 2, 0, 0xFF, 0, 0, 0,
        17, 0, 0, 0,
    ];
    for (declaration, elements) in [(DECL, &decl[..]), (SKIN_DECL, &skin_decl[..])] {
        run(
            &mut b,
            Cmd::CreateDeclaration {
                device: DEVICE,
                declaration,
                elements: Data::Bytes(elements),
            },
        );
    }
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
    let (g0, g1, h0, h1) = (-400.0f32, 400.0, -150.0, 900.0);
    let ground: Vec<f32> = [[g0, h0], [g1, h0], [g1, h1], [g0, h0], [g1, h1], [g0, h1]]
        .iter()
        .flat_map(|p| [p[0], p[1], 0.0, 1.0])
        .collect();
    bytes_buffer(&mut b, GROUND, &floats(&ground));
    let (g0, g1, h0, h1) = (-8000.0f32, 8000.0, 0.0, 20_000.0);
    let far: Vec<f32> = [[g0, h0], [g1, h0], [g1, h1], [g0, h0], [g1, h1], [g0, h1]]
        .iter()
        .flat_map(|p| [p[0], p[1], 0.0, 1.0])
        .collect();
    bytes_buffer(&mut b, FAR_GROUND, &floats(&far));
    let mut bar = Vec::new();
    bar_part(0.0, JOINT, 0, &mut bar);
    bar_part(JOINT, TOP, 1, &mut bar);
    bytes_buffer(&mut b, BAR, &bar);
    // The head: a box from 60 to 120 high at (0, 150), positions only.
    let head: Vec<f32> = box_part(JOINT, TOP)
        .iter()
        .flat_map(|p| [p[0], p[1], p[2], 1.0])
        .collect();
    bytes_buffer(&mut b, HEAD, &floats(&head));
    // Black, opaque: the game's actor shadow everywhere it is read.
    let levels = words(&[BLACK + 1]);
    run(
        &mut b,
        Cmd::CreateTexture {
            device: DEVICE,
            texture: BLACK,
            levels: Data::Bytes(&levels),
            width: 8,
            height: 1,
            usage: 0,
            format: A8R8G8B8,
            pool: 1,
        },
    );
    let texels: Vec<u8> = (0..8).flat_map(|_| [0u8, 0, 0, 255]).collect();
    run(
        &mut b,
        Cmd::UploadSurface {
            surface: BLACK + 1,
            row: 32,
            rows: 1,
            data: Data::Bytes(&texels),
        },
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

fn set(b: &mut WgpuBackend, vs: u32, decl: u32) {
    run(
        b,
        Cmd::SetShader {
            device: DEVICE,
            pixel: false,
            shader: vs,
        },
    );
    run(
        b,
        Cmd::SetShader {
            device: DEVICE,
            pixel: true,
            shader: LIT_PS,
        },
    );
    run(
        b,
        Cmd::SetVertexDeclaration {
            device: DEVICE,
            declaration: decl,
        },
    );
}

/// Render state `state` set to `value`.
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

/// The pixel shader `ps`.
fn set_ps(b: &mut WgpuBackend, ps: u32) {
    run(
        b,
        Cmd::SetShader {
            device: DEVICE,
            pixel: true,
            shader: ps,
        },
    );
}

fn draw(b: &mut WgpuBackend, buffer: u32, stride: u32, triangles: u32) {
    run(
        b,
        Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer,
            offset: 0,
            stride,
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

fn hook(b: &mut WgpuBackend, states: [u32; 2]) {
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
    let [right, up, ahead] = axes;
    let rotate: [f32; 9] = std::array::from_fn(|i| [ahead, up, right][i % 3][i / 3] as f32);
    let frame = Frame {
        rotate,
        position: pos.map(|v| v as f32),
        to_sun: sun.map(|v| v as f32),
        strength: scene_light::SHADOW_STRENGTH,
    };
    for (state, m) in states.into_iter().zip([view.to_words(), frame.to_words()]) {
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

/// The two bones' rows (3x4 each): bone 0 moved by `s0`, bone 1 (the bar's upper half) by `s1`.
fn bones_two(s0: [f32; 3], s1: [f32; 3]) -> Vec<f32> {
    let row = |s: [f32; 3]| {
        [
            1.0, 0.0, 0.0, s[0], 0.0, 1.0, 0.0, s[1], 0.0, 0.0, 1.0, s[2],
        ]
    };
    let mut r = row(s0).to_vec();
    r.extend(row(s1));
    r
}

/// Frames of the ground and the skinned bar, bone 1 moved by each of `shifts` in turn, with the
/// words the platform records when `words`; each frame's pixels (B, G, R, A).
fn frames(b: &mut WgpuBackend, shifts: &[[f32; 3]], words: bool) -> Vec<Vec<[u8; 4]>> {
    let shots: Vec<Shot> = shifts
        .iter()
        .map(|s| Shot {
            bar: Some(([0.0; 3], *s)),
            ..Shot::default()
        })
        .collect();
    frames_of(b, &shots, words)
}

/// What a frame of [`frames_of`] draws besides the ground.
#[derive(Clone, Copy, Default)]
struct Shot {
    /// The skinned bar, its two bones moved by these.
    bar: Option<([f32; 3], [f32; 3])>,
    /// The rigid head ([`HEAD`]) drawn through a face's pixel shader.
    head: bool,
    /// The ground drawn through the pixel shader with the game's actor shadow (s6 black).
    game_shadow: bool,
    /// The bar drawn blended (fading in or out: the game blends such a character), so it casts
    /// nothing of ours.
    bar_blended: bool,
    /// The bar's bones scaled by this round the origin (before their moves).
    bar_scale: Option<f32>,
}

/// Frames as `shots` say, with the words when `words`; each frame's pixels.
fn frames_of(b: &mut WgpuBackend, shots: &[Shot], words: bool) -> Vec<Vec<[u8; 4]>> {
    let mut out = Vec::new();
    for (f, shot) in shots.iter().enumerate() {
        if words {
            hook(b, [sun_shadow::VIEW_STATE, sun_shadow::BEGIN_STATE]);
        }
        clear(b, 7);
        let mvp: Vec<f32> = model_view_proj().iter().flatten().copied().collect();
        set(b, VS, DECL);
        if shot.game_shadow {
            set_ps(b, GAME_PS);
            run(
                b,
                Cmd::SetTexture {
                    device: DEVICE,
                    sampler: 6,
                    texture: BLACK,
                },
            );
        }
        constants(b, 0, 0, &mvp);
        constants(b, 3, 1, &[0.2, 0.2, 0.2, 1.0]);
        constants(b, 3, 3, &[0.6, 0.6, 0.6, 0.0]);
        draw(b, if FAR_VIEW.get() { FAR_GROUND } else { GROUND }, 16, 2);
        if let Some((s0, s1)) = shot.bar {
            set(b, SKIN_VS, SKIN_DECL);
            constants(b, 0, 0, &mvp);
            let mut bones = bones_two(s0, s1);
            if let Some(scale) = shot.bar_scale {
                // Each bone's rotation part (the rows' first three) scaled round the origin.
                for (i, v) in bones.iter_mut().enumerate() {
                    if i % 4 != 3 {
                        *v *= scale;
                    }
                }
            }
            constants(b, 0, 4, &bones);
            state(b, 22, CULL.get());
            state(b, 27, u32::from(shot.bar_blended));
            constants(b, 3, 3, &[0.5, 0.5, 0.5, 0.0]);
            draw(b, BAR, 28, 24);
            constants(b, 3, 3, &[0.6, 0.6, 0.6, 0.0]);
            state(b, 22, 1);
            state(b, 27, 0);
        }
        if shot.head {
            set(b, VS, DECL);
            set_ps(b, FACE_PS);
            constants(b, 0, 0, &mvp);
            draw(b, HEAD, 16, 12);
        }
        if words {
            hook(b, [scene_light::STATE, sun_shadow::STATE]);
        }
        clear(b, 2);
        let d = b.device().expect("the device");
        let (_, _, px) = d.read_surface_pixels(RT).expect("the target's pixels");
        out.push(
            px.chunks_exact(4)
                .map(|p| [p[0], p[1], p[2], p[3]])
                .collect(),
        );
        run(
            b,
            Cmd::Present {
                device: DEVICE,
                source: None,
                dest: None,
                window: 0,
                dirty: false,
                model: 0,
                frame: f as u32 + 1,
            },
        );
        // The lit variants are made on threads of their own: from the next frame on, drawn.
        if let Some(d) = b.device() {
            d.wait_for_sun_variants();
        }
    }
    out
}

/// The frames on a new backend: the sun shadows `on`, with the characters `characters`.
fn picture(on: bool, characters: bool, shifts: &[[f32; 3]]) -> Option<Vec<Vec<[u8; 4]>>> {
    let shots: Vec<Shot> = shifts
        .iter()
        .map(|s| Shot {
            bar: Some(([0.0; 3], *s)),
            ..Shot::default()
        })
        .collect();
    picture_of(on, characters, &shots)
}

/// [`picture`] of the frames `shots` say.
fn picture_of(on: bool, characters: bool, shots: &[Shot]) -> Option<Vec<Vec<[u8; 4]>>> {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(on);
    let b = backend();
    sun_shadow::enable(false);
    let mut b = b?;
    if let Some(d) = b.device() {
        d.set_sun_characters(characters);
        d.set_sun_character_allowance(ALLOW.get());
    }
    Some(frames_of(&mut b, shots, on))
}

fn at(px: &[[u8; 4]], p: [f64; 3]) -> u8 {
    let (x, y) = pixel(p);
    px[(y * SIZE + x) as usize][2]
}

fn dark(on: &[[u8; 4]], off: &[[u8; 4]], p: [f64; 3]) -> bool {
    f32::from(at(on, p)) < 0.7 * f32::from(at(off, p))
}

/// Where the bar's upper half throws its shadow (60 to 120 high: 45 to 90 units toward -x) when
/// bone 1 moved it by `y` along y: a point in the middle of it.
fn upper_shadow(y: f64) -> [f64; 3] {
    [-75.0, f64::from(BAR_Y) + y, 0.0]
}

/// One shadow a character in the last cascade too: just past the tip of the far bar's shadow (see
/// [`a_character_in_the_last_cascade_casts`]) the game's actor shadow (black everywhere) gives way
/// to ours, as near the camera.
#[test]
fn in_the_last_cascade_the_games_actor_shadow_gives_way_to_the_characters() {
    FAR_VIEW.set(true);
    let shots = [Shot {
        bar: Some(([0.0, 1500.0, 0.0], [0.0, 1500.0, 0.0])),
        bar_scale: Some(30.0),
        game_shadow: true,
        ..Shot::default()
    }; 4];
    let Some(_) = picture_of(false, true, &shots) else {
        return;
    };
    let ours = picture_of(true, true, &shots).expect("the adapter");
    let theirs = picture_of(true, false, &shots).expect("the adapter");
    // The tip at x -3,000 (3,600 high, 300 to each side); the pixels just past it (a pixel is about
    // 25 units here).
    let most = (0..6)
        .map(|i| {
            let p = [-3005.0 - 5.0 * f64::from(i), 6000.0, 0.0];
            i32::from(at(&ours[3], p)) - i32::from(at(&theirs[3], p))
        })
        .max()
        .expect("points");
    assert!(most > 20, "the game's shadow left out under ours: {most}");
}

/// Characters cast in every cascade (the maintainer: shadows from everything): in the far view the
/// bar, thirty times its size and 6,000 units out (the last cascade), shades the ground along its
/// shadow, both halves; beside it the ground stays lit.
#[test]
fn a_character_in_the_last_cascade_casts() {
    FAR_VIEW.set(true);
    let shots = [Shot {
        bar: Some(([0.0, 1500.0, 0.0], [0.0, 1500.0, 0.0])),
        bar_scale: Some(30.0),
        ..Shot::default()
    }; 3];
    // (The far view stays on for the asserts: `pixel` reads the camera. Each test has its own
    // thread, so it ends with the test.)
    let Some(off) = picture_of(false, true, &shots) else {
        return;
    };
    let on = picture_of(true, true, &shots).expect("the adapter");
    let (on, off) = (&on[2], &off[2]);
    // Its foot at (0, 6000); the sun's way puts 3,600 units of height 2,700 toward -x.
    for x in [-700.0, -2000.0] {
        assert!(dark(on, off, [x, 6000.0, 0.0]), "its shadow at x {x}");
    }
    assert_eq!(
        at(on, [-700.0, 7500.0, 0.0]),
        at(off, [-700.0, 7500.0, 0.0]),
        "beside its shadow"
    );
}

/// A bar bent by its second bone casts where its bones put it: the upper half's shadow is beside
/// the moved upper half, and none is where its bind pose would cast; the lower half casts where it
/// stands.
#[test]
fn a_skinned_bar_bent_by_two_bones_casts_where_the_bent_shape_is() {
    let moved = [0.0, 200.0, 0.0];
    let shifts = [moved; 3];
    let Some(off) = picture(false, true, &shifts) else {
        return;
    };
    let on = picture(true, true, &shifts).expect("the adapter");
    let (on, off) = (&on[2], &off[2]);
    assert!(
        dark(on, off, upper_shadow(200.0)),
        "the moved half's shadow"
    );
    assert!(
        dark(on, off, [-30.0, f64::from(BAR_Y), 0.0]),
        "the lower half's shadow"
    );
    assert_eq!(
        at(on, upper_shadow(0.0)),
        at(off, upper_shadow(0.0)),
        "no shadow of the bind pose"
    );
}

/// A character walking: its shadow follows it frame by frame (one frame behind: the maps are
/// drawn at a frame's scene clear from the frame before's draws) and nothing stays where it was.
#[test]
fn a_moving_skinned_casters_shadow_follows_it_with_no_trail() {
    let shifts: Vec<[f32; 3]> = (0..6).map(|f| [0.0, 60.0 * f as f32, 0.0]).collect();
    let Some(off) = picture(false, true, &shifts) else {
        return;
    };
    let on = picture(true, true, &shifts).expect("the adapter");
    for f in 2..6 {
        let here = f64::from(60.0 * (f - 1) as f32);
        assert!(
            dark(&on[f], &off[f], upper_shadow(here)),
            "frame {f}: the shadow where the frame before drew it"
        );
        for gone in 0..f - 1 {
            let was = f64::from(60.0 * gone as f32);
            assert_eq!(
                at(&on[f], upper_shadow(was)),
                at(&off[f], upper_shadow(was)),
                "frame {f}: a shadow left at its place of frame {gone}"
            );
        }
    }
}

/// With the characters off, a skinned draw casts nothing: the frame is the frame of the sun
/// shadows without it (the bar's place unshadowed, as before characters cast).
#[test]
fn with_the_characters_off_a_skinned_draw_casts_nothing() {
    let shifts = [[0.0, 0.0, 0.0]; 3];
    let Some(off) = picture(false, false, &shifts) else {
        return;
    };
    let on = picture(true, false, &shifts).expect("the adapter");
    for p in [upper_shadow(0.0), [-30.0, f64::from(BAR_Y), 0.0]] {
        assert_eq!(at(&on[2], p), at(&off[2], p), "{p:?}");
    }
    let with = picture(true, true, &shifts).expect("the adapter");
    assert!(dark(&with[2], &off[2], upper_shadow(0.0)));
}

/// With the sun shadows off, a scene with characters and the words is bit for bit the scene
/// without the words.
#[test]
fn off_a_frame_with_characters_is_unchanged() {
    let shifts = [[0.0, 200.0, 0.0]; 3];
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(false);
    let (Some(mut a), Some(mut b)) = (backend(), backend()) else {
        return;
    };
    let plain = frames(&mut a, &shifts, false);
    let with_words = frames(&mut b, &shifts, true);
    assert!(plain == with_words);
}

/// A character running away from the sun (6 units a frame toward -x, the sun toward +x): its
/// sunlit side keeps the sun's full light, frame after frame. The characters' layer holds the pose
/// of the frame before, nearer the sun by a run's step: the layer's allowance for a character's
/// own lit draw (`SKIN_RECEIVER_BIAS`) keeps that from shading it (review of #726).
#[test]
fn a_character_running_away_from_the_sun_keeps_its_sunlit_side() {
    let x = |f: usize| -80.0 - 6.0 * f as f32;
    let shots: Vec<Shot> = (0..7)
        .map(|f| Shot {
            bar: Some(([x(f), 0.0, 0.0], [x(f), 0.0, 0.0])),
            ..Shot::default()
        })
        .collect();
    let Some(off) = picture_of(false, true, &shots) else {
        return;
    };
    let on = picture_of(true, true, &shots).expect("the adapter");
    for f in 3..7 {
        // The bar's side toward the sun (x + 10), half way up each half.
        for z in [30.0, 90.0] {
            let p = [f64::from(x(f)) + 10.0, f64::from(BAR_Y), z];
            assert_eq!(at(&on[f], p), at(&off[f], p), "frame {f}, {z} high");
        }
        // It still casts on the ground (the frame before's place).
        assert!(dark(
            &on[f],
            &off[f],
            [f64::from(x(f - 1)) - 75.0, f64::from(BAR_Y), 0.0]
        ));
    }
}

/// A face (a rigid draw whose pixel shader names `FaceGenMap0`) casts as a character's part: this
/// frame only, gone the frame after it is no longer drawn (a head's shadow never stays behind
/// without its body, review of #726).
#[test]
fn a_faces_shadow_goes_the_frame_after_it_is_not_drawn() {
    let shots: Vec<Shot> = (0..7)
        .map(|f| Shot {
            head: f < 4,
            ..Shot::default()
        })
        .collect();
    let Some(off) = picture_of(false, true, &shots) else {
        return;
    };
    let on = picture_of(true, true, &shots).expect("the adapter");
    assert!(dark(&on[3], &off[3], upper_shadow(0.0)), "the head casts");
    for f in 5..7 {
        assert_eq!(
            at(&on[f], upper_shadow(0.0)),
            at(&off[f], upper_shadow(0.0)),
            "frame {f}: the head's shadow stayed"
        );
    }
}

/// One shadow a character: where the characters cast, the game's own actor shadow (`ShadowMap`,
/// here black everywhere) is read as lit; with the characters off it darkens the sun's light as
/// it always did (as with the sun shadows off).
#[test]
fn the_games_actor_shadow_gives_way_to_the_characters_only_while_they_cast() {
    let shots: Vec<Shot> = (0..4)
        .map(|_| Shot {
            bar: Some(([0.0; 3], [0.0; 3])),
            game_shadow: true,
            ..Shot::default()
        })
        .collect();
    let Some(off) = picture_of(false, true, &shots) else {
        return;
    };
    let ours = picture_of(true, true, &shots).expect("the adapter");
    let theirs = picture_of(true, false, &shots).expect("the adapter");
    // Just past the tip of the bar's own shadow (in the sun: ours takes the sun away entirely
    // inside it), within the few texels round it where the game's shadow gives way to ours: lit,
    // where the game's black darkens it with the characters off.
    let under = [-101.0, f64::from(BAR_Y), 0.0];
    assert_eq!(
        at(&theirs[3], under),
        at(&off[3], under),
        "the game's shadow as before"
    );
    assert!(
        at(&ours[3], under) > at(&theirs[3], under) + 20,
        "the game's shadow left out under ours: {} vs {}",
        at(&ours[3], under),
        at(&theirs[3], under)
    );
    // Away from any character's shadow of ours: the game's stays (review of #726, round 2).
    let p = [60.0, 300.0, 0.0];
    assert_eq!(
        at(&ours[3], p),
        at(&off[3], p),
        "the game's shadow where ours is not"
    );
}

/// With the characters on but none cast in a frame (none drawn), the game's own actor shadow shows
/// as before: it gives way only where ours replace it.
#[test]
fn with_no_character_cast_the_games_actor_shadow_stays() {
    let shots: Vec<Shot> = (0..4)
        .map(|_| Shot {
            game_shadow: true,
            ..Shot::default()
        })
        .collect();
    let Some(off) = picture_of(false, true, &shots) else {
        return;
    };
    let on = picture_of(true, true, &shots).expect("the adapter");
    let p = [60.0, 300.0, 0.0];
    assert_eq!(at(&on[3], p), at(&off[3], p));
}

/// A closed character drawn with culling, running away from the sun with no allowance at all:
/// only the culling (the draw's own: seen from the sun it keeps the side away from the sun)
/// keeps its sunlit side lit. Counterclockwise culling draws the box's outside.
#[test]
fn the_draws_own_culling_keeps_a_closed_characters_sunlit_side() {
    CULL.set(3);
    ALLOW.set(Some(0.0));
    let x = |f: usize| -80.0 - 6.0 * f as f32;
    let shots: Vec<Shot> = (0..7)
        .map(|f| Shot {
            bar: Some(([x(f), 0.0, 0.0], [x(f), 0.0, 0.0])),
            ..Shot::default()
        })
        .collect();
    let Some(off) = picture_of(false, true, &shots) else {
        return;
    };
    let on = picture_of(true, true, &shots).expect("the adapter");
    for f in 3..7 {
        for z in [30.0, 90.0] {
            let p = [f64::from(x(f)) + 10.0, f64::from(BAR_Y), z];
            // The bar is drawn there (its outside: not the ground's grey behind it).
            assert_ne!(at(&off[f], p), 204, "frame {f}: the box's side is drawn");
            assert_eq!(at(&on[f], p), at(&off[f], p), "frame {f}, {z} high");
        }
        assert!(dark(
            &on[f],
            &off[f],
            [f64::from(x(f - 1)) - 75.0, f64::from(BAR_Y), 0.0]
        ));
    }
}

/// A two-sided character part (drawn with no culling: turning the culling round does nothing)
/// under a high sun (70 degrees), stepping 6 units a frame away from it: 17.5 units along the
/// sun's rays, past the fixed 12. The allowance measured from the frame's step keeps its sunlit
/// side lit.
#[test]
fn the_frames_step_keeps_a_two_sided_parts_sunlit_side_under_a_high_sun() {
    let e = 70f64.to_radians();
    SUN.set([e.cos(), 0.0, e.sin()]);
    let x = |f: usize| -80.0 - 6.0 * f as f32;
    let shots: Vec<Shot> = (0..7)
        .map(|f| Shot {
            bar: Some(([x(f), 0.0, 0.0], [x(f), 0.0, 0.0])),
            ..Shot::default()
        })
        .collect();
    let Some(off) = picture_of(false, true, &shots) else {
        return;
    };
    let on = picture_of(true, true, &shots).expect("the adapter");
    for f in 3..7 {
        for z in [30.0, 90.0] {
            let p = [f64::from(x(f)) + 10.0, f64::from(BAR_Y), z];
            assert_eq!(at(&on[f], p), at(&off[f], p), "frame {f}, {z} high");
        }
    }
}

/// A character that casts nothing of ours this frame (drawn blended: fading in or out) keeps the
/// game's own shadow, though another character (a face) casts: the game's shadow gives way only
/// under our characters' shadows.
#[test]
fn a_character_not_cast_keeps_the_games_shadow_while_another_casts() {
    let shots: Vec<Shot> = (0..4)
        .map(|_| Shot {
            head: true,
            bar: Some(([150.0, 300.0, 0.0], [150.0, 300.0, 0.0])),
            bar_blended: true,
            game_shadow: true,
            bar_scale: None,
        })
        .collect();
    let Some(off) = picture_of(false, true, &shots) else {
        return;
    };
    let on = picture_of(true, true, &shots).expect("the adapter");
    // Where the blended bar's shadow would fall (it stands at (150, 450)).
    let p = [150.0 - 75.0, f64::from(BAR_Y) + 300.0, 0.0];
    assert_eq!(at(&on[3], p), at(&off[3], p));
    // Round the face's own shadow (just past its tip) the game's gives way.
    let tip = [-101.0, f64::from(BAR_Y), 0.0];
    assert!(
        at(&on[3], tip) > at(&off[3], tip) + 40,
        "{} vs {}",
        at(&on[3], tip),
        at(&off[3], tip)
    );
}
