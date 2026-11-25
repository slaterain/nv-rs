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
/// [`CUT_PS`] as a character's face's pixel shader (its constant table names `FaceGenMap0`).
const FACE_CUT_PS: u32 = 0x1760;
/// The lit pixel shader: ambient (c1) plus the sun's light colour (c3), as the game's lit shaders add
/// them.
const LIT_PS: u32 = 0x1320;
const A8R8G8B8: u32 = 21;
const D24S8: u32 = 75;
const SIZE: u32 = 256;
const NEAR: f64 = 10.0;
/// The far plane: 10,000 units, or 100,000 in the far scene ([`FAR_SCENE`]).
fn far_plane() -> f64 {
    if FAR_SCENE.get() {
        100_000.0
    } else {
        10_000.0
    }
}
thread_local! {
    /// The far scene (distant shadows): the camera high over a wide land, looking far ahead; on
    /// for this test's thread while a [`FarScene`] lives (each test runs on a thread of its own).
    static FAR_SCENE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The far scene on for the test's thread until dropped (then as it was).
struct FarScene(bool);

impl FarScene {
    fn on() -> FarScene {
        FarScene(FAR_SCENE.replace(true))
    }
}

impl Drop for FarScene {
    fn drop(&mut self) {
        FAR_SCENE.set(self.0);
    }
}
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
    if FAR_SCENE.get() {
        // At (0, 0, 3000), looking toward +y and 10 degrees down.
        let (s, c) = 10f64.to_radians().sin_cos();
        return (
            [0.0, 0.0, 3000.0],
            [[1.0, 0.0, 0.0], [0.0, s, c], [0.0, c, -s]],
        );
    }
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
    let far = far_plane();
    let (zs, zo) = (far / (far - NEAR), -NEAR * far / (far - NEAR));
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

/// A `ps_3_0` constant table naming the sampler s0 `FaceGenMap0` (a character's face's shader).
fn face_table() -> Vec<u32> {
    let mut b = Vec::new();
    for v in [28u32, 0, 0xFFFF_0300, 1, 28, 0, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    // Its name, set (sampler), first register, count, reserved, type, default.
    b.extend_from_slice(&48u32.to_le_bytes());
    for v in [3u16, 0, 1, 0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&[0; 8]);
    b.extend_from_slice(b"FaceGenMap0\0");
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
    backend_in(samples, None)
}

/// [`backend`] keeping its pipelines and lists in `cache` (a folder), when given.
fn backend_in(samples: u32, cache: Option<std::path::PathBuf>) -> Option<WgpuBackend> {
    let mut b = WgpuBackend::new(Output::Offscreen);
    if let Some(dir) = cache {
        b = b.with_cache_dir(dir);
    }
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
    hook_words_for(to_sun())
}

/// [`hook_words`] with the sun toward `sun` (a unit vector in the world).
fn hook_words_for(sun: [f64; 3]) -> ([u32; 16], [u32; 16]) {
    hook_words_at(sun, [0.0; 3])
}

/// [`hook_words_for`] with the camera's place moved by `shift`.
fn hook_words_at(sun: [f64; 3], shift: [f64; 3]) -> ([u32; 16], [u32; 16]) {
    let (pos, axes) = camera();
    let pos: [f64; 3] = std::array::from_fn(|k| pos[k] + shift[k]);
    let view = Params {
        near: NEAR as f32,
        far: far_plane() as f32,
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
    frame_with(b, words, None)
}

/// [`frame`] with the box drawn through `box_shaders` (vertex, pixel), the rest as [`frame`] draws it.
fn frame_with(b: &mut WgpuBackend, words: bool, box_shaders: Option<(u32, u32)>) -> Vec<[u8; 4]> {
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
    if let Some((vs, ps)) = box_shaders {
        box_through(b, vs, ps);
    }
    draw(b, BOX, 10);
    if box_shaders.is_some() {
        box_through(b, VS, PS);
    }
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

/// The casters' depth-only pipelines are counted for the hitch log apart from the game's pipelines
/// (`d3d9_stream::hitch::Count::ShadowPipelines`): a still box's made on the render thread the first
/// time it is seen.
#[test]
fn the_casters_pipelines_are_counted_for_the_hitch_log() {
    use d3d9_stream::hitch::{snapshot, Count};
    let before = snapshot();
    if picture(1, true, true).is_none() {
        return;
    }
    let now = snapshot();
    let d = |c: Count| now[c as usize] - before[c as usize];
    assert!(
        d(Count::ShadowPipelines) >= 1,
        "{}",
        d(Count::ShadowPipelines)
    );
    assert!(d(Count::ShadowPipelineNs) > 0);
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

/// With the card's times on (`NV_WGPU_GPU_TIMES`), the sun shadow maps are timed as their own kind
/// ("sun shadow maps") where the driver writes timestamps between passes, so their depth passes are
/// in the line's total. Where it times passes only they are not timed (the line says so); without
/// timestamps, skipped.
#[test]
fn the_gpu_times_time_the_sun_shadow_maps() {
    use d3d9_wgpu::gpu_times::{Kind, KINDS};
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable_debug();
    std::env::set_var("NV_WGPU_GPU_TIMES", "1");
    let b = backend(1);
    std::env::remove_var("NV_WGPU_GPU_TIMES");
    sun_shadow::enable(false);
    let Some(mut b) = b else {
        return;
    };
    let d = b.device().expect("the device");
    if !d.adapter_has_timestamps() {
        eprintln!("this adapter cannot time passes: skipped");
        return;
    }
    assert!(d.gpu_times_on(), "asked for, and the adapter can");
    let inside = d.timestamp_features().1;
    // The maps are drawn at the scene's clear from the frame before's casters: from the second
    // frame on.
    for n in 1..=3 {
        frame(&mut b, true);
        present(&mut b, n);
    }
    let slot = |k: Kind| KINDS.iter().position(|(x, _)| *x == k).expect("a kind");
    let maps = slot(Kind::SunShadows);
    let start = std::time::Instant::now();
    let t = loop {
        let t = b.device().and_then(|d| d.gpu_time_totals()).expect("on");
        if t.spans[maps] >= 2 || t.zeros > 0 || start.elapsed().as_secs() >= 5 {
            break t;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let d = b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
    if t.zeros > 0 {
        // Known only of AMD's driver (raw wgpu too): the times are said to be missing.
        let name = d.adapter_name().to_string();
        assert!(
            name.contains("AMD") || name.contains("Radeon"),
            "zero timestamps on {name}, not known to give them: {t:?}"
        );
        eprintln!("{name} reads timestamps as zero: the times are not shown: {t:?}");
        return;
    }
    if !inside {
        assert_eq!(t.spans[maps], 0, "{t:?}");
        eprintln!("this driver times passes only: the sun shadow maps are not timed (said)");
        return;
    }
    assert!(
        t.spans[maps] >= 2,
        "the maps of frames 2 and 3 timed: {t:?}"
    );
    assert!(t.ns[maps] > 0, "the maps took time: {t:?}");
    assert_eq!(t.left_out, 0, "every pair in order: {t:?}");
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
    /// The ribbon and a round wire in one cut-out geometry ([`RIBBON_ROUND`]).
    ribbon_round: bool,
    /// The half-opaque cut-out strip ([`FENCE`]).
    fence: bool,
    /// One of [`CABLES`] moved by x.
    cable_moved: Option<(usize, f32)>,
    /// The plate under the cable 120 high ([`PLATE`]).
    plate: bool,
    /// The tubes drawn indexed ([`TUBES`]): which three the index buffer names (written into it
    /// when it changes).
    tubes: Option<MixedIndexed>,
    /// The wire and pipe drawn as an indexed strip ([`STRIP_VB`]).
    strip: bool,
    /// [`RIBBON_ROUND`] drawn without the alpha test (its strip opaque).
    ribbon_round_opaque: bool,
    /// [`RIBBON_ROUND`] drawn with a texture too opaque for a cable ([`STRIP_TEX`]) and the alpha test.
    ribbon_round_textured: bool,
    /// [`RIBBON_ROUND`] and [`MIXED_CUT`] (`ribbon_round`, `mixed_cut`) drawn through a face's pixel
    /// shader ([`FACE_CUT_PS`]): a character's casters.
    face_cutouts: bool,
    /// The cable of `cable` written again (the same bytes) before it is drawn.
    rewrite_cable: bool,
    /// The pipe of [`CABLES`] drawn after `cable` (another buffer of the same layout and count).
    pipe_after: bool,
    /// A buffer the game releases at the frame's start (`Cmd::Destroy`), before anything is drawn.
    destroy: Option<u32>,
    /// Other buffers (and their triangle counts) drawn far off (at x -3000, y 3000): new
    /// geometries whose shadows fall nowhere the tests look.
    also: [Option<(u32, u32)>; 2],
    /// The far scene's land ([`FAR_GROUND`]), tower ([`TOWER`]) and long wall ([`WALL`]).
    far_ground: bool,
    tower: bool,
    wall: bool,
    /// The far scene's tower moved by this along x (in its matrix) instead of drawn in place.
    tower_shift: Option<f32>,
    /// The two boxes of one vertex and index buffer ([`SEG_VB`], [`SEG_IB`]) as two index ranges:
    /// both drawn (`Some(true)`) or the first alone (`Some(false)`).
    segments: Option<bool>,
    /// [`SEG_VB`]'s first range alone, and (`Some(true)`) a copy of it moved 390 along x, across the
    /// view's right edge (the same range at another place: one mesh, two objects).
    seg_copy: Option<bool>,
    /// [`EDGE_VB`]'s two ranges (`Some(true)`) or its first alone.
    edge_segments: Option<bool>,
    /// [`FAR_SEG_VB`]'s two ranges (`Some(true)`) or its first alone.
    far_segments: Option<bool>,
    /// The far scene's small caster ([`FAR_SMALL`]) moved by this along x (in its matrix).
    small_shift: Option<f32>,
    /// The tall caster far off toward a low sun ([`SUNWARD`]).
    sunward: bool,
    /// The world word (`sun_shadow::WORLD_STATE`) before the scene's words.
    world: Option<u32>,
    /// The sun toward this (a unit vector in the world) instead of [`to_sun`].
    sun: Option<[f64; 3]>,
    /// The far scene's low-sun tower ([`FAR_TOWER_LOW`]), the wide hill round the camera
    /// ([`HILL`]) and the tower past the far map's reach ([`BEYOND`]).
    low_tower: bool,
    hill: bool,
    beyond: bool,
    /// The cut-out ([`CUT`]) made this many times larger round the world origin (drawn instead of
    /// `cutout`).
    cutout_scale: Option<f32>,
    /// ... and moved by this along x (in its matrix) after the scaling.
    cutout_shift: Option<f32>,
    /// The camera's place in the frame's words moved by this (world units; the draws' matrices
    /// stay): the maps step after it.
    camera_shift: Option<[f64; 3]>,
}

/// What the index buffer of [`TUBES`] names.
#[derive(Clone, Copy, Debug, PartialEq)]
enum MixedIndexed {
    /// The fine wire, the second wire and the pipe: cables and a pipe.
    TwoWiresAPipe,
    /// The fine wire and both pipes: a longer list of what is not cable.
    AWireTwoPipes,
}

impl MixedIndexed {
    fn indices(self) -> Vec<u16> {
        let tube = |n: u32| (n * TUBE_CORNERS..(n + 1) * TUBE_CORNERS).map(|i| i as u16);
        match self {
            MixedIndexed::TwoWiresAPipe => tube(0).chain(tube(1)).chain(tube(2)).collect(),
            MixedIndexed::AWireTwoPipes => tube(0).chain(tube(2)).chain(tube(3)).collect(),
        }
    }
}

/// A buffer `handle` of `bytes` (an index buffer of 16-bit indices when `index`).
fn bytes_buffer(b: &mut WgpuBackend, handle: u32, index: bool, bytes: &[u8]) {
    run(
        b,
        Cmd::CreateBuffer {
            device: DEVICE,
            buffer: handle,
            index,
            length: bytes.len() as u32,
            usage: 0,
            format: if index { 101 } else { 0 },
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

/// A texture 8 texels across, 1 high, with alpha `alpha(i)` at texel i.
fn alpha_texture(b: &mut WgpuBackend, texture: u32, alpha: impl Fn(u8) -> u8) {
    let levels = words(&[texture + 1]);
    run(
        b,
        Cmd::CreateTexture {
            device: DEVICE,
            texture,
            levels: Data::Bytes(&levels),
            width: 8,
            height: 1,
            usage: 0,
            format: A8R8G8B8,
            pool: 1,
        },
    );
    let texels: Vec<u8> = (0..8).flat_map(|i| [0u8, 0, 0, alpha(i)]).collect();
    run(
        b,
        Cmd::UploadSurface {
            surface: texture + 1,
            row: 32,
            rows: 1,
            data: Data::Bytes(&texels),
        },
    );
}

/// Draws `count` triangles of `buffer` (position and texture coordinate, 24 bytes a vertex) with
/// the cut-out shaders, `texture` and the alpha test.
fn draw_cut(b: &mut WgpuBackend, buffer: u32, texture: u32, count: u32) {
    draw_textured(b, buffer, texture, count, true);
}

/// [`draw_cut`] with the alpha test on (`test`) or off.
fn draw_textured(b: &mut WgpuBackend, buffer: u32, texture: u32, count: u32, test: bool) {
    draw_textured_through(b, CUT_PS, buffer, texture, count, test);
}

/// [`draw_textured`] through the pixel shader `ps`.
fn draw_textured_through(
    b: &mut WgpuBackend,
    ps: u32,
    buffer: u32,
    texture: u32,
    count: u32,
    test: bool,
) {
    set_shaders(b, CUT_VS, ps, CUT_DECL);
    run(
        b,
        Cmd::SetTexture {
            device: DEVICE,
            sampler: 0,
            texture,
        },
    );
    state(b, ALPHATESTENABLE, u32::from(test));
    state(b, ALPHAREF, 128);
    state(b, ALPHAFUNC, 7);
    run(
        b,
        Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer,
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
            count,
        },
    );
    state(b, ALPHATESTENABLE, 0);
    set_shaders(b, VS, LIT_PS, DECL);
}

/// The ribbon of [`RIBBON`]'s corners (position, w, texture coordinate): 16 quads along y.
fn ribbon_corners() -> Vec<[f32; 6]> {
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
        v.extend([a, bb, c, a, c, d]);
    }
    v
}

/// A tube's rings (seven corners each, the seam twice) as an indexed strip's vertices and indices
/// (numbered from `first`), the bands joined by degenerate triangles.
fn tube_strip((x, h, r): (f32, f32, f32), first: u16) -> (Vec<[f32; 3]>, Vec<u16>) {
    let ring = |i: usize, s: usize| {
        let a = s as f32 * std::f32::consts::TAU / 6.0;
        [x + r * a.cos(), -100.0 + 50.0 * i as f32, h + r * a.sin()]
    };
    let mut v = Vec::new();
    for i in 0..=16 {
        for s in 0..=6 {
            v.push(ring(i, s));
        }
    }
    let at = |i: usize, s: usize| first + (i * 7 + s) as u16;
    let mut idx: Vec<u16> = Vec::new();
    for i in 0..16 {
        if i > 0 {
            // Joined: the last corner again, the next band's first (an even count before each
            // band, so every band starts on an even triangle).
            let last = *idx.last().expect("a band before");
            idx.extend([last, at(i, 0)]);
        }
        for s in 0..=6 {
            idx.extend([at(i, s), at(i + 1, s)]);
        }
    }
    (v, idx)
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
/// One cut-out geometry of the ribbon of [`RIBBON`] and a round wire 0.8 wide (thinner than the
/// coverage maps' texel), 300 high at x 185 (its shadow at x -40), the wire's texture coordinate
/// where the ribbon's texture is opaque.
const RIBBON_ROUND: u32 = 0x15F0;
/// A cut-out strip 60 wide (x 1395 to 1455), 2000 high, 1000 long (y -150 to 850): a ribbon's shape,
/// but its texture half opaque (stripes 7.5 units wide), as a fence's or a grille's; its shadow
/// at x -130 to -70. High up, so drawn as a cable it would cast a faint wash (the sun's width spreads
/// it), as a cut-out its full stripes. Its buffer and texture.
const FENCE: u32 = 0x1600;
const FENCE_TEX: u32 = 0x1610;
/// A texture opaque but for one texel in eight (mean alpha 0.875: no cable).
const STRIP_TEX: u32 = 0x1620;
/// A lit plate 5 units under the cable 120 high of [`CABLES`] (x -40 to 10, y 250 to 350, z 115).
const PLATE: u32 = 0x1630;
/// Tubes for the indexed draws: ten unused vertices, then the fine wire of [`CABLES`] (its shadow
/// at x -100), a second fine wire at x 400 (its shadow at x 175), the pipe of [`MIXED`] (its shadow
/// at x -40) and a second pipe 40 wide, 100 high at x 250 (its shadow about x 175); each as a list
/// of 576 corners. Its index buffer (16-bit) names three of them ([`MixedIndexed`]).
const TUBES: u32 = 0x1640;
const TUBES_IB: u32 = 0x1650;
/// The fine wire and the pipe of [`MIXED`] as one indexed strip (rings of seven corners, the bands
/// joined by degenerate triangles), seven unused vertices first; its index buffer.
const STRIP_VB: u32 = 0x1660;
const STRIP_IB: u32 = 0x1670;
const TUBE_CORNERS: u32 = 16 * 6 * 2 * 3;
const PADDING: u32 = 10;
const STRIP_PADDING: u32 = 7;
/// The far scene ([`FAR_SCENE`]): land 80,000 wide and 100,000 deep (radius about 64,000: no caster for
/// the cascades, as the game's LOD land), a tower 1,000 a side and 4,000 high at (2000, 20000) (its
/// shadow on the land at x -1500 to 1500, past the cascades), and a wall 200 thick, 1,000 high along y
/// 5000 to 14000 at x 1500 (its shadow at x 650 to 1400, across the end of the cascades).
const FAR_GROUND: u32 = 0x1680;
const TOWER: u32 = 0x1690;
const WALL: u32 = 0x16A0;
/// More of the far scene: a tower 4,000 wide and 10,000 high at x 18000 to 22000, y 30000 (with the
/// sun 10 degrees up from -y its shadow runs along +y past y 85000, past where the far map's depth
/// ended before review of #715); a long low wall the camera stands inside the sphere of (x 2000 to
/// 2400, y -20000 to 40000, 3,000 high: its shadow at x -250 to 2000; no caster for the cascades);
/// a tower past the far map's reach (y 150000).
const FAR_TOWER_LOW: u32 = 0x16B0;
const HILL: u32 = 0x16C0;
const BEYOND: u32 = 0x16D0;
/// A caster 4,000 wide and 6,000 high at y 104000 to 106000 (past 98,304 units from the camera less
/// its radius): with the sun 10 degrees up toward +y its shadow runs back to about y 70000, inside
/// the far map.
const SUNWARD: u32 = 0x16E0;
/// Report 012's LOD block in small: two boxes (x -140 to -100 and 250 to 290, y 80 to 120, 80
/// high) in one vertex buffer, each its own range of one index buffer.
const SEG_VB: u32 = 0x1700;
const SEG_IB: u32 = 0x1710;
/// Two boxes as two ranges again, the second (x 290 to 430) out of the view across its right edge,
/// its sphere reaching in (review of #844).
const EDGE_VB: u32 = 0x1720;
const EDGE_IB: u32 = 0x1730;
/// Two towers as two ranges in the far scene, past the cascades: one at x -2,500 to -1,500, the
/// other at x 10,500 to 12,500 (its centre out of the view, its sphere reaching in).
const FAR_SEG_VB: u32 = 0x1740;
const FAR_SEG_IB: u32 = 0x1750;
/// A far caster of radius about 327 (300 by 300, 500 high, at (2000, 20000)): past the cascades'
/// reach, in the far map alone.
const FAR_SMALL: u32 = 0x16F0;

/// A box from (x0, y0, 0) to (x1, y1, h): its top and four sides.
fn cuboid(x0: f32, x1: f32, y0: f32, y1: f32, h: f32) -> Vec<[f32; 3]> {
    let quad = |a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]| vec![a, b, c, a, c, d];
    let mut v = Vec::new();
    v.extend(quad([x0, y0, h], [x1, y0, h], [x1, y1, h], [x0, y1, h]));
    v.extend(quad([x0, y0, 0.0], [x1, y0, 0.0], [x1, y0, h], [x0, y0, h]));
    v.extend(quad([x0, y1, 0.0], [x1, y1, 0.0], [x1, y1, h], [x0, y1, h]));
    v.extend(quad([x0, y0, 0.0], [x0, y1, 0.0], [x0, y1, h], [x0, y0, h]));
    v.extend(quad([x1, y0, 0.0], [x1, y1, 0.0], [x1, y1, h], [x1, y0, h]));
    v
}

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

/// The camera's `ModelViewProj` with the object scaled by `s` round the world origin first.
fn model_view_proj_scaled(s: f32) -> [[f32; 4]; 4] {
    let mut m = model_view_proj();
    for row in m.iter_mut() {
        for v in row.iter_mut().take(3) {
            *v *= s;
        }
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
    let (g0, g1, h0, h1) = (-40_000.0, 40_000.0, 0.0, 100_000.0);
    buffer(
        b,
        FAR_GROUND,
        &[
            [g0, h0, 0.0],
            [g1, h0, 0.0],
            [g1, h1, 0.0],
            [g0, h0, 0.0],
            [g1, h1, 0.0],
            [g0, h1, 0.0],
        ],
    );
    buffer(
        b,
        TOWER,
        &cuboid(1500.0, 2500.0, 19_500.0, 20_500.0, 4000.0),
    );
    buffer(b, WALL, &cuboid(1400.0, 1600.0, 5000.0, 14_000.0, 1000.0));
    let mut seg = cuboid(-140.0, -100.0, 80.0, 120.0, 80.0);
    seg.extend(cuboid(250.0, 290.0, 80.0, 120.0, 80.0));
    buffer(b, SEG_VB, &seg);
    let ib: Vec<u8> = (0..seg.len() as u16).flat_map(u16::to_le_bytes).collect();
    bytes_buffer(b, SEG_IB, true, &ib);
    let mut edge = cuboid(-140.0, -100.0, 80.0, 120.0, 80.0);
    edge.extend(cuboid(290.0, 430.0, 80.0, 120.0, 80.0));
    buffer(b, EDGE_VB, &edge);
    bytes_buffer(b, EDGE_IB, true, &ib);
    let mut far_seg = cuboid(-2500.0, -1500.0, 19_500.0, 20_500.0, 4000.0);
    far_seg.extend(cuboid(10_500.0, 12_500.0, 19_000.0, 21_000.0, 4000.0));
    buffer(b, FAR_SEG_VB, &far_seg);
    bytes_buffer(b, FAR_SEG_IB, true, &ib);
    buffer(
        b,
        FAR_SMALL,
        &cuboid(1850.0, 2150.0, 19_850.0, 20_150.0, 500.0),
    );
    buffer(
        b,
        FAR_TOWER_LOW,
        &cuboid(18_000.0, 22_000.0, 30_000.0, 30_500.0, 10_000.0),
    );
    buffer(
        b,
        HILL,
        &cuboid(2000.0, 2400.0, -20_000.0, 40_000.0, 3000.0),
    );
    buffer(
        b,
        SUNWARD,
        &cuboid(-2000.0, 2000.0, 104_000.0, 106_000.0, 6000.0),
    );
    buffer(
        b,
        BEYOND,
        &cuboid(-2000.0, 2000.0, 150_000.0, 154_000.0, 4000.0),
    );
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
    // The same with a table naming its sampler `FaceGenMap0` (a face's).
    let mut face = vec![ps[0]];
    face.extend(face_table());
    face.extend_from_slice(&ps[1..]);
    for (shader, pixel, tokens) in [
        (CUT_VS, false, vs),
        (CUT_PS, true, ps),
        (FACE_CUT_PS, true, face),
    ] {
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
    // The ribbon and a round wire in one cut-out geometry.
    let mut v = ribbon_corners();
    v.extend(
        cable_corners((185.0, 300.0, 0.4))
            .iter()
            .map(|p| [p[0], p[1], p[2], 1.0, 3.5 / 8.0, 0.5]),
    );
    let bytes = floats(&v.concat());
    bytes_buffer(b, RIBBON_ROUND, false, &bytes);
    // The half-opaque strip: 40 quads along y, u across x.
    let corner = |x: f32, y: f32| [x, y, 2000.0, 1.0, (x - 1395.0) / 60.0, (y + 150.0) / 1000.0];
    let mut v = Vec::new();
    for i in 0..40 {
        let (y0, y1) = (-150.0 + 25.0 * i as f32, -125.0 + 25.0 * i as f32);
        let (a, bb, c, d) = (
            corner(1395.0, y0),
            corner(1455.0, y0),
            corner(1455.0, y1),
            corner(1395.0, y1),
        );
        v.extend([a, bb, c, a, c, d]);
    }
    bytes_buffer(b, FENCE, false, &floats(&v.concat()));
    alpha_texture(b, FENCE_TEX, |i| if i % 2 == 0 { 255 } else { 0 });
    // Opaque but for texel 3 (where the round wire of [`RIBBON_ROUND`] reads it): too opaque for
    // a cable, its strip cast in wide opaque blocks.
    alpha_texture(b, STRIP_TEX, |i| if i == 3 { 0 } else { 255 });
    let (x0, x1, y0, y1, z) = (-40.0, 10.0, 250.0, 350.0, 115.0);
    buffer(
        b,
        PLATE,
        &[
            [x0, y0, z],
            [x1, y0, z],
            [x1, y1, z],
            [x0, y0, z],
            [x1, y1, z],
            [x0, y1, z],
        ],
    );
    // The tubes, ten unused corners first.
    let mut tubes = vec![[0.0, 0.0, -10_000.0]; PADDING as usize];
    for t in [
        CABLES[0],
        (400.0, 300.0, 0.4),
        (35.0, 100.0, 20.0),
        (250.0, 100.0, 20.0),
    ] {
        tubes.extend(cable_corners(t));
    }
    buffer(b, TUBES, &tubes);
    // (Room for either list: both are three tubes.)
    let ib: Vec<u8> = MixedIndexed::TwoWiresAPipe
        .indices()
        .iter()
        .flat_map(|i| i.to_le_bytes())
        .collect();
    bytes_buffer(b, TUBES_IB, true, &ib);
    // The wire and the pipe as one strip.
    let (mut sv, mut si) = tube_strip(CABLES[0], 0);
    let (pv, pi) = tube_strip((35.0, 100.0, 20.0), sv.len() as u16);
    let last = *si.last().expect("the wire's strip");
    si.extend([last, pi[0]]);
    si.extend(pi);
    sv.extend(pv);
    let mut padded = vec![[0.0, 0.0, -10_000.0]; STRIP_PADDING as usize];
    padded.extend(sv);
    buffer(b, STRIP_VB, &padded);
    let ib: Vec<u8> = si.iter().flat_map(|i| i.to_le_bytes()).collect();
    bytes_buffer(b, STRIP_IB, true, &ib);
}

/// The strip's index count ([`STRIP_IB`]).
fn strip_indices() -> u32 {
    let (_, w) = tube_strip(CABLES[0], 0);
    let (_, p) = tube_strip((35.0, 100.0, 20.0), 0);
    (w.len() + 2 + p.len()) as u32
}

/// Draws `count` primitives of kind `kind` from index buffer `ib` over vertex buffer `vb` (positions,
/// 16 bytes a vertex) from base vertex `base`.
fn draw_indexed(b: &mut WgpuBackend, vb: u32, ib: u32, kind: u32, base: i32, count: u32) {
    run(
        b,
        Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: vb,
            offset: 0,
            stride: 16,
        },
    );
    run(
        b,
        Cmd::SetIndices {
            device: DEVICE,
            buffer: ib,
        },
    );
    run(
        b,
        Cmd::DrawIndexedPrimitive {
            device: DEVICE,
            kind,
            base_vertex: base,
            min_index: 0,
            vertices: 4 * TUBE_CORNERS,
            start_index: 0,
            count,
        },
    );
}

/// Draws `count` triangles of index buffer `ib` from index `start` over vertex buffer `vb`
/// (positions, 16 bytes a vertex; [`SEG_VB`]'s 60 vertices).
fn draw_range(b: &mut WgpuBackend, vb: u32, ib: u32, start: u32, count: u32) {
    run(
        b,
        Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: vb,
            offset: 0,
            stride: 16,
        },
    );
    run(
        b,
        Cmd::SetIndices {
            device: DEVICE,
            buffer: ib,
        },
    );
    run(
        b,
        Cmd::DrawIndexedPrimitive {
            device: DEVICE,
            kind: 4,
            base_vertex: 0,
            min_index: 0,
            vertices: 60,
            start_index: start,
            count,
        },
    );
}

const ALPHATESTENABLE: u32 = 15;
const ALPHAREF: u32 = 24;
const ALPHAFUNC: u32 = 25;

/// Lit frames, one for each of `frames` (each with the words the platform records when `words`),
/// the ground and what the frame's [`Extra`] says; each frame's pixels.
fn lit_sequence(b: &mut WgpuBackend, frames: &[Extra], words: bool) -> Vec<Vec<[u8; 4]>> {
    lit_sequence_with(b, frames, words, &mut |_, _| {})
}

/// [`lit_sequence`], with `after(f, b)` after frame `f` is presented.
fn lit_sequence_with(
    b: &mut WgpuBackend,
    frames: &[Extra],
    words: bool,
    after: &mut dyn FnMut(usize, &mut WgpuBackend),
) -> Vec<Vec<[u8; 4]>> {
    extras(b);
    let words_here = hook_words();
    let mut out = Vec::new();
    // What the tubes' index buffer holds (made with the first list).
    let mut tubes_written = Some(MixedIndexed::TwoWiresAPipe);
    for (f, e) in frames.iter().enumerate() {
        let (view, frame) = match (e.sun, e.camera_shift) {
            (None, None) => words_here,
            (sun, shift) => hook_words_at(sun.unwrap_or_else(to_sun), shift.unwrap_or([0.0; 3])),
        };
        set_shaders(b, VS, LIT_PS, DECL);
        if let Some(object) = e.destroy {
            run(b, Cmd::Destroy { object });
        }
        if words {
            if let Some(w) = e.world {
                run(
                    b,
                    Cmd::SetTransform {
                        device: DEVICE,
                        state: sun_shadow::WORLD_STATE,
                        matrix: Data::from_words(&sun_shadow::world_words(w)),
                    },
                );
            }
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
        if e.far_ground {
            draw(b, FAR_GROUND, 2);
        }
        if e.tower {
            draw(b, TOWER, 10);
        }
        if let Some(both) = e.segments {
            draw_range(b, SEG_VB, SEG_IB, 0, 10);
            if both {
                draw_range(b, SEG_VB, SEG_IB, 30, 10);
            }
        }
        for (pair, vb, ib) in [
            (e.edge_segments, EDGE_VB, EDGE_IB),
            (e.far_segments, FAR_SEG_VB, FAR_SEG_IB),
        ] {
            if let Some(both) = pair {
                draw_range(b, vb, ib, 0, 10);
                if both {
                    draw_range(b, vb, ib, 30, 10);
                }
            }
        }
        if let Some(copy) = e.seg_copy {
            draw_range(b, SEG_VB, SEG_IB, 0, 10);
            if copy {
                set_mvp(b, model_view_proj_at(390.0, 0.0));
                draw_range(b, SEG_VB, SEG_IB, 0, 10);
                set_mvp(b, model_view_proj());
            }
        }
        if let Some(dx) = e.tower_shift {
            set_mvp(b, model_view_proj_at(dx, 0.0));
            draw(b, TOWER, 10);
            set_mvp(b, model_view_proj());
        }
        if let Some(dx) = e.small_shift {
            set_mvp(b, model_view_proj_at(dx, 0.0));
            draw(b, FAR_SMALL, 10);
            set_mvp(b, model_view_proj());
        }
        if e.sunward {
            draw(b, SUNWARD, 10);
        }
        if e.wall {
            draw(b, WALL, 10);
        }
        for (on, buffer) in [
            (e.low_tower, FAR_TOWER_LOW),
            (e.hill, HILL),
            (e.beyond, BEYOND),
        ] {
            if on {
                draw(b, buffer, 10);
            }
        }
        for (buffer, triangles) in e.also.iter().flatten() {
            set_mvp(b, model_view_proj_at(-3000.0, 3000.0));
            draw(b, *buffer, *triangles);
            set_mvp(b, model_view_proj());
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
        let cut_ps = if e.face_cutouts { FACE_CUT_PS } else { CUT_PS };
        if e.mixed_cut {
            set_shaders(b, CUT_VS, cut_ps, CUT_DECL);
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
            if e.rewrite_cable {
                let v: Vec<f32> = cable_corners(CABLES[n])
                    .iter()
                    .flat_map(|c| [c[0], c[1], c[2], 1.0])
                    .collect();
                run(
                    b,
                    Cmd::UploadBuffer {
                        buffer: CABLE + n as u32,
                        offset: 0,
                        flags: 0,
                        data: Data::Bytes(&floats(&v)),
                    },
                );
            }
            draw(b, CABLE + n as u32, 16 * 6 * 2);
        }
        if e.pipe_after {
            draw(b, CABLE + 3, 16 * 6 * 2);
        }
        if e.ribbon_round {
            let count = 32 + 16 * 6 * 2;
            draw_textured_through(b, cut_ps, RIBBON_ROUND, RIBBON_TEX, count, true);
        }
        if e.ribbon_round_opaque {
            draw_textured(b, RIBBON_ROUND, RIBBON_TEX, 32 + 16 * 6 * 2, false);
        }
        if e.ribbon_round_textured {
            draw_cut(b, RIBBON_ROUND, STRIP_TEX, 32 + 16 * 6 * 2);
        }
        if e.fence {
            draw_cut(b, FENCE, FENCE_TEX, 80);
        }
        if let Some((n, x)) = e.cable_moved {
            set_mvp(b, model_view_proj_at(x, 0.0));
            draw(b, CABLE + n as u32, 16 * 6 * 2);
            set_mvp(b, model_view_proj());
        }
        if e.plate {
            draw(b, PLATE, 2);
        }
        if let Some(which) = e.tubes {
            // The game writes new indices into the same buffer when the list changes.
            if tubes_written != Some(which) {
                let ib: Vec<u8> = which
                    .indices()
                    .iter()
                    .flat_map(|i| i.to_le_bytes())
                    .collect();
                run(
                    b,
                    Cmd::UploadBuffer {
                        buffer: TUBES_IB,
                        offset: 0,
                        flags: 0,
                        data: Data::Bytes(&ib),
                    },
                );
                tubes_written = Some(which);
            }
            // Three tubes of 192 triangles.
            draw_indexed(b, TUBES, TUBES_IB, 4, PADDING as i32, TUBE_CORNERS);
        }
        if e.strip {
            draw_indexed(
                b,
                STRIP_VB,
                STRIP_IB,
                5,
                STRIP_PADDING as i32,
                strip_indices() - 2,
            );
        }
        if e.cutout || e.cutout_scale.is_some() {
            if let Some(s) = e.cutout_scale {
                let mut m = model_view_proj_scaled(s);
                if let Some(dx) = e.cutout_shift {
                    // (The first column is already scaled by s.)
                    for row in m.iter_mut() {
                        row[3] += row[0] * dx / s;
                    }
                }
                set_mvp(b, m);
            }
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
            set_mvp(b, model_view_proj());
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
        after(f, b);
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

/// Cut-outs cast in every cascade (the maintainer: shadows from everything): the checker cut-out
/// made forty times larger in the far scene stands 4,000 to 8,000 units out, in the last cascade;
/// its opaque squares shade the land and its clear ones let the sun through, as near the camera.
#[test]
fn a_cut_out_in_the_last_cascade_casts_its_alpha_tests_shape() {
    let _far = FarScene::on();
    let frames = [Extra {
        cutout_scale: Some(40.0),
        ..far_land(false, false)
    }; 8];
    let Some((off, _)) = far_sequence(1, false, &frames) else {
        return;
    };
    let (on, _) = far_sequence(1, true, &frames).expect("the adapter");
    // A point (y, z) of the square throws its shadow to (-0.75 z, y) on the ground.
    for (y, z, opaque) in [
        (125.0, 45.0, true),
        (175.0, 15.0, true),
        (175.0, 45.0, false),
        (125.0, 15.0, false),
    ] {
        let (y, z) = (40.0 * y, 40.0 * z);
        let t = taken_at(&off[7], &on[7], -0.75 * z, y);
        eprintln!("the far cut-out at ({y}, {z}): {t}");
        if opaque {
            assert!(t > 0.8, "({y}, {z}) opaque: {t}");
        } else {
            assert!(t < 0.1, "({y}, {z}) clear: {t}");
        }
    }
}

/// Review of #817: a moving cut-out far out (the far checker stepping 20 units a frame, a bush in
/// the wind in the last cascade) still casts there while it moves: the moving cut-outs are left out
/// of a cascade only where their shadow cannot reach it, never for being past the first.
#[test]
fn a_moving_cut_out_in_the_last_cascade_casts_while_it_moves() {
    let _far = FarScene::on();
    let shift = |f: usize| 20.0 * f as f32;
    let frames: Vec<Extra> = (0..16)
        .map(|f| Extra {
            cutout_scale: Some(40.0),
            cutout_shift: Some(shift(f)),
            ..far_land(false, false)
        })
        .collect();
    let Some((off, _)) = far_sequence(1, false, &frames) else {
        return;
    };
    let (on, _) = far_sequence(1, true, &frames).expect("the adapter");
    let last = frames.len() - 1;
    let dx = f64::from(shift(last));
    for (y, z, opaque) in [
        (125.0, 45.0, true),
        (175.0, 15.0, true),
        (175.0, 45.0, false),
        (125.0, 15.0, false),
    ] {
        let (y, z) = (40.0 * y, 40.0 * z);
        let t = taken_at(&off[last], &on[last], -0.75 * z + dx, y);
        eprintln!("the moving far cut-out at ({y}, {z}): {t}");
        if opaque {
            assert!(t > 0.8, "({y}, {z}) opaque: {t}");
        } else {
            assert!(t < 0.1, "({y}, {z}) clear: {t}");
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
        // Soft, not the hard line the alpha test cuts (review of #693: the old path took about as
        // much at its darkest): the band is as strong all along the ribbon, where the alpha test's
        // one-texel line comes and goes with where the cable's texel falls on the map's (0.31 to
        // 0.46 along it, measured with the coverage path off). No exact grey is asked for.
        let along: Vec<f32> = (0..=12)
            .map(|i| most_taken(off, on, -100.0, 225.0 + 25.0 * f64::from(i)))
            .collect();
        let (most, least) = (
            along.iter().copied().fold(f32::MIN, f32::max),
            along.iter().copied().fold(f32::MAX, f32::min),
        );
        eprintln!("the ribbon along it: {along:.2?} ({samples})");
        assert!(
            most - least < 0.1 && least > 0.04,
            "the ribbon's band along it {along:?} ({samples})"
        );
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
        assert_eq!(
            at(on, p)[2],
            AMBIENT_RED,
            "the pipe's shadow ({samples}, cut-out {cut})"
        );
    }
}

/// A lit draw whose state is seen in one frame only (a load's fade, a menu: here six blend
/// factors with blending off) still has its variant made on a thread; the device takes each made
/// variant in at the next present, whether that draw comes back or not, so the six do not hold
/// every place among those made at once: the lit scene drawn after them is shadowed within a few
/// frames. Before (2026-11-17), a variant was taken in only when its own draw came again, and
/// after six such the Strip had no shadow at all for the rest of the run.
#[test]
fn a_variant_whose_draw_never_comes_back_does_not_hold_the_others() {
    const SRCBLEND: u32 = 19;
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(true);
    let b = backend(1);
    sun_shadow::enable(false);
    let Some(mut b) = b else {
        return;
    };
    let b = &mut b;
    run(
        b,
        Cmd::SetShader {
            device: DEVICE,
            pixel: true,
            shader: LIT_PS,
        },
    );
    let (view, frame) = hook_words();
    let shade = pixel([-55.0, 140.0, 0.0]);
    let sun_side = pixel([50.0, 150.0, 0.0]);
    let mut shadowed_at = None;
    for f in 1..=300u32 {
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
        clear(b, 7);
        let mvp = model_view_proj();
        let regs: Vec<f32> = mvp.iter().flatten().copied().collect();
        constants(b, 0, 0, &regs);
        constants(b, 3, 1, &[0.2, 0.2, 0.2, 1.0]);
        constants(b, 3, 3, &[0.5, 0.5, 0.5, 0.0]);
        // The second frame (the first with the maps): six one-off states first.
        if f == 2 {
            for factor in 3..=8 {
                state(b, SRCBLEND, factor);
                draw(b, GROUND, 2);
            }
            state(b, SRCBLEND, 2);
        }
        draw(b, GROUND, 2);
        draw(b, BOX, 10);
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
        clear(b, 2);
        if f >= 3 {
            let d = b.device().expect("the device");
            let (_, _, px) = d.read_surface_pixels(RT).expect("the target's pixels");
            let px: Vec<[u8; 4]> = px
                .chunks_exact(4)
                .map(|p| [p[0], p[1], p[2], p[3]])
                .collect();
            if f32::from(at(&px, shade)[2]) < 0.7 * f32::from(at(&px, sun_side)[2]) {
                shadowed_at = Some(f);
                break;
            }
        }
        present(b, f);
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
    assert!(
        shadowed_at.is_some(),
        "the lit scene never took its variant (six one-off variants held the places)"
    );
}

/// Frames of the lit scene with the world word (`sun_shadow::WORLD_STATE`) before the scene's
/// words: `worlds[i]` the world of frame i + 1, the box drawn while `boxes[i]`; the last frame's
/// pixels.
fn world_frames(b: &mut WgpuBackend, worlds: &[u32], boxes: &[bool]) -> Vec<[u8; 4]> {
    const ZWRITEENABLE: u32 = 14;
    run(
        b,
        Cmd::SetShader {
            device: DEVICE,
            pixel: true,
            shader: LIT_PS,
        },
    );
    let (view, frame) = hook_words();
    for (i, (&world, &with_box)) in worlds.iter().zip(boxes).enumerate() {
        let f = i as u32 + 1;
        for (state, m) in [
            (sun_shadow::WORLD_STATE, sun_shadow::world_words(world)),
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
        clear(b, 7);
        let mvp = model_view_proj();
        let regs: Vec<f32> = mvp.iter().flatten().copied().collect();
        constants(b, 0, 0, &regs);
        constants(b, 3, 1, &[0.2, 0.2, 0.2, 1.0]);
        constants(b, 3, 3, &[0.5, 0.5, 0.5, 0.0]);
        if with_box {
            draw(b, GROUND, 2);
            draw(b, BOX, 10);
        } else {
            // Without the box the ground is drawn without depth writes: no caster comes, so only
            // forgetting the maps themselves takes the box's shadow away.
            state(b, ZWRITEENABLE, 0);
            draw(b, GROUND, 2);
            state(b, ZWRITEENABLE, 1);
        }
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
        clear(b, 2);
        if i + 1 < worlds.len() {
            present(b, f);
        }
        // The variants of each new state (the ground without depth writes too) are waited for.
        if f >= 2 {
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

/// A new world forgets the casters kept: the box drawn in world 1 and then not drawn in world 2
/// (the world left behind: Freeside's buildings at the Strip's places, run pf-fs12g) casts no
/// shadow there from the first frame the maps are drawn again, while in the same world it is kept
/// through a few frames not drawn (its grace), as before.
#[test]
fn a_new_world_forgets_the_casters_of_the_one_left() {
    let worlds_same = [1u32, 1, 1, 1, 1, 1, 1];
    let worlds_new = [1u32, 1, 1, 1, 2, 2, 2];
    let boxes = [true, true, true, true, false, false, false];
    let picture = |worlds: &[u32]| {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        sun_shadow::enable(true);
        let b = backend(1);
        sun_shadow::enable(false);
        let mut b = b?;
        Some(world_frames(&mut b, worlds, &boxes))
    };
    let Some(same) = picture(&worlds_same) else {
        return;
    };
    let new = picture(&worlds_new).expect("the adapter");
    let shade = pixel([-55.0, 140.0, 0.0]);
    let sun_side = pixel([50.0, 150.0, 0.0]);
    let dark = |px: &[[u8; 4]]| f32::from(at(px, shade)[2]) < 0.7 * f32::from(at(px, sun_side)[2]);
    assert!(
        dark(&same),
        "the same world keeps the box's shadow through its grace"
    );
    assert!(
        !dark(&new),
        "a new world keeps no shadow of the box left behind"
    );
    assert_eq!(at(&new, shade), at(&new, pixel([-110.0, 150.0, 0.0])));
}

/// The share of the sun taken (0 lit, 1 none) at ground point (x, y), `on` against `off`, from the
/// red channel.
fn taken_at(off: &[[u8; 4]], on: &[[u8; 4]], x: f64, y: f64) -> f32 {
    let p = pixel([x, y, 0.0]);
    let lit = f32::from(at(off, p)[2]);
    (lit - f32::from(at(on, p)[2])) / (lit - f32::from(AMBIENT_RED))
}

/// The most taken along the ground across x - 12 to x + 12 at y (one-unit steps).
fn most_taken(off: &[[u8; 4]], on: &[[u8; 4]], x: f64, y: f64) -> f32 {
    (0..=24)
        .map(|i| taken_at(off, on, x - 12.0 + f64::from(i), y))
        .fold(f32::MIN, f32::max)
}

/// The third frames of `frames` (three alike) off and on.
fn third_frames(samples: u32, e: Extra) -> Option<(Pixels, Pixels)> {
    let frames = [e; 3];
    let off = lit_sequence_picture(samples, false, &frames)?;
    let on = lit_sequence_picture(samples, true, &frames).expect("the adapter");
    Some((off[2].clone(), on[2].clone()))
}

/// A round wire in one cut-out geometry with a ribbon (review of #693) keeps its own coverage: its
/// shadow is a faint band all along it (no gaps, no texel-sized dots at full strength), while the
/// ribbon beside it still casts its own.
#[test]
fn a_round_wire_beside_a_ribbon_keeps_its_soft_band() {
    for samples in [1, 4] {
        let Some((off, on)) = third_frames(
            samples,
            Extra {
                ribbon_round: true,
                ..Extra::default()
            },
        ) else {
            return;
        };
        // All along the wire's shadow (at x -40), y 150 to 450.
        let along: Vec<f32> = (0..=12)
            .map(|i| most_taken(&off, &on, -40.0, 150.0 + 25.0 * f64::from(i)))
            .collect();
        eprintln!("the round wire beside a ribbon, along it: {along:.2?} ({samples})");
        let (most, least) = (
            along.iter().copied().fold(f32::MIN, f32::max),
            along.iter().copied().fold(f32::MAX, f32::min),
        );
        // As strong all along (drawn through the ribbon's alpha it came and went: 0.24 to 0.49).
        assert!(
            least > 0.04 && most < 0.6 && most - least < 0.1,
            "the wire's band along it {along:?} ({samples})"
        );
        let ribbon = most_taken(&off, &on, -100.0, 300.0);
        assert!(
            (0.04..0.6).contains(&ribbon),
            "the ribbon takes {ribbon} ({samples})"
        );
    }
}

/// A cut-out strip shaped like a ribbon (60 wide, 1000 long) whose texture is half opaque (a
/// fence, a grille) is no cable (review of #693): it keeps its cut-out shadow in the depth maps,
/// full where its opaque stripes are, lit between them.
#[test]
fn a_half_opaque_strip_keeps_its_cut_out_shadow() {
    for samples in [1, 4] {
        let Some((off, on)) = third_frames(
            samples,
            Extra {
                fence: true,
                ..Extra::default()
            },
        ) else {
            return;
        };
        let across: Vec<f32> = (0..=90)
            .map(|i| taken_at(&off, &on, -145.0 + f64::from(i), 300.0))
            .collect();
        let most = across.iter().copied().fold(f32::MIN, f32::max);
        let least = across.iter().copied().fold(f32::MAX, f32::min);
        eprintln!("the half-opaque strip: most {most}, least {least} taken ({samples})");
        assert!(most > 0.9, "its stripes' shadow takes {most} ({samples})");
        assert!(least < 0.1, "between its stripes {least} ({samples})");
    }
}

/// A receiver a few units under a cable (the cable's own surface, the pole top it hangs from: here
/// a plate 5 units under the cable 120 high) takes none of its shadow: the plate is as without the
/// shadows.
#[test]
fn a_plate_just_under_a_cable_takes_none_of_its_shadow() {
    for samples in [1, 4] {
        let Some((off, on)) = third_frames(
            samples,
            Extra {
                cable: Some(1),
                plate: true,
                ..Extra::default()
            },
        ) else {
            return;
        };
        for i in 0..=12 {
            let x = -32.0 + 2.5 * f64::from(i);
            let p = pixel([x, 300.0, 115.0]);
            assert_eq!(at(&on, p), at(&off, p), "the plate at x {x} ({samples})");
        }
    }
}

/// A cable that moves: its band follows it, and when it is gone its band is gone the next frame
/// (the frame then exactly as without the shadows).
#[test]
fn a_moving_cables_band_follows_it_and_goes_with_it() {
    let at_x = |x| Extra {
        cable_moved: Some((1, x)),
        ..Extra::default()
    };
    let frames = [
        at_x(0.0),
        at_x(0.0),
        at_x(0.0),
        at_x(30.0),
        at_x(60.0),
        Extra::default(),
        Extra::default(),
    ];
    let Some(off) = lit_sequence_picture(1, false, &frames) else {
        return;
    };
    let on = lit_sequence_picture(1, true, &frames).expect("the adapter");
    // (A frame's maps hold the casters of the frame before.)
    let still = most_taken(&off[3], &on[3], -100.0, 300.0);
    let moving = most_taken(&off[4], &on[4], -70.0, 300.0);
    let moved_here = most_taken(&off[5], &on[5], -40.0, 300.0);
    let moved_from = most_taken(&off[5], &on[5], -100.0, 300.0);
    eprintln!(
        "the cable standing {still}, moving {moving}, moved: at its new place {moved_here}, its old {moved_from}"
    );
    assert!(
        still > 0.04 && moving > 0.04 && moved_here > 0.04,
        "{still} {moving} {moved_here}"
    );
    assert!(
        moved_from < 0.02,
        "no band left where it stood: {moved_from}"
    );
    assert!(on[6] == off[6], "the cable gone, its band gone");
}

/// A geometry of a wire and a pipe drawn as an indexed strip from a base vertex (the game's usual
/// draw) casts as the same geometry drawn as a list: the wire's faint band, the pipe's full shadow.
#[test]
fn a_wire_and_pipe_drawn_as_an_indexed_strip_cast_as_the_list() {
    for samples in [1, 4] {
        let Some((off, list)) = third_frames(
            samples,
            Extra {
                mixed: true,
                ..Extra::default()
            },
        ) else {
            return;
        };
        let (off_strip, strip) = third_frames(
            samples,
            Extra {
                strip: true,
                ..Extra::default()
            },
        )
        .expect("the adapter");
        let (wire_list, wire_strip) = (
            most_taken(&off, &list, -100.0, 300.0),
            most_taken(&off_strip, &strip, -100.0, 300.0),
        );
        eprintln!("the wire's band: list {wire_list}, strip {wire_strip} ({samples})");
        assert!(
            (0.04..0.5).contains(&wire_strip),
            "{wire_strip} ({samples})"
        );
        assert!(
            (wire_strip - wire_list).abs() < 0.05,
            "{wire_list} {wire_strip} ({samples})"
        );
        let p = pixel([-40.0, 300.0, 0.0]);
        assert_eq!(
            at(&strip, p)[2],
            AMBIENT_RED,
            "the pipe's shadow ({samples})"
        );
        // Elsewhere the pictures are those of the list (the same triangles).
        let differ_off = off.iter().zip(&off_strip).filter(|(a, b)| a != b).count();
        let differ = list.iter().zip(&strip).filter(|(a, b)| a != b).count();
        eprintln!("pixels differing, list and strip: {differ} on, {differ_off} off ({samples})");
        assert!(differ < 40, "{differ} pixels differ ({samples})");
    }
}

/// The game writes new indices into a geometry of cables and pipes between frames (review of #693:
/// the list of what is not cable was kept by the geometry and drawn with the old indices; a longer
/// new list read past its end and lost the frame): from then on it casts as a new device given the
/// new indices from the start does: the second pipe's full shadow, no band of the wire it lost.
#[test]
fn new_indices_written_into_a_mixed_geometry_cast_as_on_a_new_device() {
    let first = Extra {
        tubes: Some(MixedIndexed::TwoWiresAPipe),
        ..Extra::default()
    };
    let then = Extra {
        tubes: Some(MixedIndexed::AWireTwoPipes),
        ..Extra::default()
    };
    for samples in [1, 4] {
        let Some(off) = lit_sequence_picture(samples, false, &[first; 3]) else {
            return;
        };
        let rewritten =
            lit_sequence_picture(samples, true, &[first, first, first, then, then, then])
                .expect("the adapter");
        let fresh = lit_sequence_picture(samples, true, &[then; 6]).expect("the adapter");
        // Before: the second wire's faint band where the second pipe's shadow comes later.
        let before = most_taken(&off[2], &rewritten[2], 175.0, 300.0);
        eprintln!("at x 175 before the write {before} ({samples})");
        assert!((0.04..0.6).contains(&before), "{before} ({samples})");
        for f in [4, 5] {
            let p = pixel([175.0, 300.0, 0.0]);
            assert_eq!(
                at(&rewritten[f], p)[2],
                AMBIENT_RED,
                "frame {f} ({samples})"
            );
            let differ = rewritten[f]
                .iter()
                .zip(&fresh[f])
                .filter(|(a, b)| a != b)
                .count();
            eprintln!("frame {f}: {differ} pixels differ from a new device's ({samples})");
            assert!(
                rewritten[f] == fresh[f],
                "frame {f}: {differ} pixels ({samples})"
            );
        }
    }
}

/// [`lit_sequence`] with the sun shadows on and the device's bodies limited to `limit`
/// ([`sun_shadow::limit_bodies`]); with how many times they were all forgotten.
fn limited_sequence(samples: u32, frames: &[Extra], limit: usize) -> Option<(Vec<Pixels>, u64)> {
    struct Unlimit;
    impl Drop for Unlimit {
        fn drop(&mut self) {
            sun_shadow::limit_bodies(0);
        }
    }
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(true);
    let b = backend(samples);
    sun_shadow::enable(false);
    let mut b = b?;
    let before = sun_shadow::bodies_forgotten();
    let _unlimit = Unlimit;
    sun_shadow::limit_bodies(limit);
    let on = lit_sequence(&mut b, frames, true);
    Some((on, sun_shadow::bodies_forgotten() - before))
}

/// The device's kept bodies all forgotten at their limit (review of #693, round 2: a smaller limit
/// here, so it is reached): a wire and pipe in one geometry, kept for the maps while not drawn (out
/// of sight; here through its grace), keeps the pipe's full shadow, where before its list of what is
/// not cable was taken for a buffer the game had let go and its shadow dropped at once.
#[test]
fn a_split_pole_not_drawn_keeps_its_shadow_when_the_bodies_are_forgotten() {
    let pole = Extra {
        mixed: true,
        ..Extra::default()
    };
    // Another geometry comes while the pole is not drawn: its body is one past the limit.
    let other = Extra {
        cable: Some(2),
        ..Extra::default()
    };
    let frames = [pole, pole, pole, pole, other, other, other, other];
    for samples in [1, 4] {
        // The ground and the pole: the next new geometry's body is past the limit.
        let Some((on, forgotten)) = limited_sequence(samples, &frames, 2) else {
            return;
        };
        assert!(forgotten > 0, "the bodies were forgotten ({samples})");
        let p = pixel([-40.0, 300.0, 0.0]);
        // (The first frame has no maps yet; a frame's maps hold the casters of the frame before.)
        for (f, px) in on.iter().enumerate().skip(2) {
            assert_eq!(
                at(px, p)[2],
                AMBIENT_RED,
                "the pipe's shadow in frame {f} ({samples})"
            );
        }
    }
}

/// A round wire in one geometry with a flat strip that is no cable (review of #693, round 2):
/// drawn opaque, or with a texture too opaque for a cable (opaque but for one stripe), the strip
/// casts its full shadow while the wire keeps its faint band all along it.
#[test]
fn a_round_wire_beside_a_strip_keeps_its_soft_band() {
    for (samples, textured) in [(1, false), (4, false), (1, true), (4, true)] {
        let Some((off, on)) = third_frames(
            samples,
            Extra {
                ribbon_round_opaque: !textured,
                ribbon_round_textured: textured,
                ..Extra::default()
            },
        ) else {
            return;
        };
        let along: Vec<f32> = (0..=12)
            .map(|i| most_taken(&off, &on, -40.0, 150.0 + 25.0 * f64::from(i)))
            .collect();
        eprintln!(
            "the round wire beside a strip (textured {textured}), along it: {along:.2?} ({samples})"
        );
        let (most, least) = (
            along.iter().copied().fold(f32::MIN, f32::max),
            along.iter().copied().fold(f32::MAX, f32::min),
        );
        assert!(
            least > 0.04 && most < 0.6 && most - least < 0.1,
            "the wire's band along it {along:?} (textured {textured}, {samples})"
        );
        // The strip's shadow (x -110 to -90): full along it, in its opaque blocks when textured.
        let strip: Vec<f32> = (0..=8)
            .map(|i| most_taken(&off, &on, -100.0, 200.0 + 25.0 * f64::from(i)))
            .collect();
        eprintln!("the strip along it: {strip:.2?} (textured {textured}, {samples})");
        assert!(
            strip.iter().all(|&t| t > 0.9),
            "the strip's shadow {strip:?} (textured {textured}, {samples})"
        );
    }
}

/// A geometry the game writes again every frame (a cable here) has its pieces worked out for its
/// first two bodies only: from its third on it casts whole, as any caster (a hard line), not as a
/// cable (chosen in review of #693, round 2: its cables would have to be found again each frame).
#[test]
fn a_cable_written_every_frame_casts_whole_from_its_third_body() {
    let still = Extra {
        cable: Some(1),
        ..Extra::default()
    };
    let written = Extra {
        rewrite_cable: true,
        ..still
    };
    let Some(off) = lit_sequence_picture(1, false, &[still; 6]) else {
        return;
    };
    let kept = lit_sequence_picture(1, true, &[still; 6]).expect("the adapter");
    // One body a geometry, whatever its writes: the ground and the cable stay under a limit of 3
    // (a body for each write, as before review round 2, would pass it on the third frame).
    let (rewritten, forgotten) = limited_sequence(1, &[written; 6], 3).expect("the adapter");
    assert_eq!(forgotten, 0, "one body a geometry");
    let soft = most_taken(&off[5], &kept[5], -100.0, 300.0);
    let hard = most_taken(&off[5], &rewritten[5], -100.0, 300.0);
    eprintln!("the cable kept {soft}, written every frame {hard}");
    assert!(
        soft < hard - 0.2,
        "kept, a cable's band: {soft} against {hard}"
    );
    assert!(
        hard > 0.9,
        "written every frame, a whole caster's shadow: {hard}"
    );
}

/// Two buffers alike (the same layout, offsets, upload counts and triangle count: a cable 3 wide
/// and a pipe 40 wide) are two geometries (review of #693, round 2: keyed by each buffer's own
/// number, not taken for one another): the pipe drawn after the cable casts its full shadow.
#[test]
fn two_buffers_alike_are_two_geometries() {
    let Some((_, on)) = third_frames(
        1,
        Extra {
            cable: Some(1),
            pipe_after: true,
            ..Extra::default()
        },
    ) else {
        return;
    };
    assert_eq!(at(&on, pixel([-100.0, 300.0, 0.0]))[2], AMBIENT_RED);
}

/// Bodies of geometries the game released are dropped to make room (review of #693, round 3): with
/// room for 4 bodies, the ground and a cable the game writes every frame (live) and a box the game
/// then releases, a fourth and a fifth geometry coming find room by the released box's body going,
/// so nothing is forgotten wholesale and the cable, written three times and more, still casts
/// whole (its count of bodies kept: forgotten, it would be split again and cast soft).
#[test]
fn a_released_geometrys_body_makes_room_and_nothing_is_forgotten() {
    let cable = Extra {
        cable: Some(1),
        rewrite_cable: true,
        ..Extra::default()
    };
    let with = |also: [Option<(u32, u32)>; 2], destroy: Option<u32>| Extra {
        also,
        destroy,
        ..cable
    };
    let a = Some((MOVING_BOX, 10));
    let frames = [
        with([a, None], None),
        with([a, None], None),
        with([a, None], None),
        // The box released; a new geometry: the fourth body.
        with([Some((BOX, 10)), None], Some(MOVING_BOX)),
        // Another new one: no room but the released box's.
        with([Some((BOX, 10)), Some((AFTER, 2))], None),
        with([Some((BOX, 10)), Some((AFTER, 2))], None),
        with([Some((BOX, 10)), Some((AFTER, 2))], None),
    ];
    let Some(off) = lit_sequence_picture(1, false, &frames) else {
        return;
    };
    let (on, forgotten) = limited_sequence(1, &frames, 4).expect("the adapter");
    assert_eq!(forgotten, 0, "the released box's body made room");
    for f in 4..frames.len() {
        let hard = most_taken(&off[f], &on[f], -100.0, 300.0);
        eprintln!("frame {f}: the cable written every frame takes {hard}");
        assert!(hard > 0.9, "frame {f}: the cable casts whole ({hard})");
    }
}

/// A geometry kept at two places that the game releases while it is not drawn is let go at both
/// at once (review of #693, round 3: "only this draw holds it" never held for two): the frame
/// after is the one without the shadows, not the boxes' shadows kept through their grace.
#[test]
fn a_released_geometry_at_two_places_goes_at_both() {
    let two = Extra {
        moving_box: Some((0.0, 150.0)),
        other_box: Some((150.0, 400.0)),
        ..Extra::default()
    };
    let released = Extra {
        destroy: Some(MOVING_BOX),
        ..Extra::default()
    };
    let frames = [two, two, two, released, Extra::default(), Extra::default()];
    let Some(off) = lit_sequence_picture(1, false, &frames) else {
        return;
    };
    let on = lit_sequence_picture(1, true, &frames).expect("the adapter");
    let dark = |px: &[[u8; 4]], p: [f64; 3]| {
        f32::from(at(px, pixel(p))[2]) < 0.7 * f32::from(at(px, pixel([50.0, 150.0, 0.0]))[2])
    };
    assert!(
        dark(&on[2], [-55.0, 140.0, 0.0]) && dark(&on[2], [95.0, 390.0, 0.0]),
        "both boxes cast while drawn"
    );
    for f in [4, 5] {
        assert!(on[f] == off[f], "frame {f}: both places let go");
    }
}

/// [`lit_sequence`] in the far scene ([`FAR_SCENE`]) with the sun shadows `on` (the words with
/// them), and the times the far map was drawn; none without an adapter.
fn far_sequence(samples: u32, on: bool, frames: &[Extra]) -> Option<(Vec<Pixels>, u64)> {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let _far = FarScene::on();
    sun_shadow::enable(on);
    let b = backend(samples);
    sun_shadow::enable(false);
    let mut b = b?;
    let px = lit_sequence(&mut b, frames, on);
    let drawn = b.device().map_or(0, |d| d.sun_far_maps_drawn());
    Some((px, drawn))
}

/// The far scene's land with the tower (`tower`) and the wall (`wall`).
fn far_land(tower: bool, wall: bool) -> Extra {
    Extra {
        far_ground: true,
        tower,
        wall,
        ..Extra::default()
    }
}

/// A tall caster far past the cascades (a tower 20,000 units out, where the cascades end at
/// 10,000) shadows the land there from the far map; beside its shadow the land stays lit.
#[test]
fn a_far_caster_past_the_cascades_shadows_the_land() {
    let _far = FarScene::on();
    // (A far redraw is spread over a few frames: the map is in from the fifth.)
    let frames = [far_land(true, false); 8];
    for samples in [1, 4] {
        let Some((off, _)) = far_sequence(samples, false, &frames) else {
            return;
        };
        let (on, drawn) = far_sequence(samples, true, &frames).expect("the adapter");
        assert!(drawn > 0, "the far map was drawn");
        let shade = taken_at(&off[7], &on[7], -500.0, 20_000.0);
        let beside =
            [(-3000.0, 20_000.0), (-500.0, 17_000.0)].map(|(x, y)| taken_at(&off[7], &on[7], x, y));
        eprintln!("the tower's far shadow takes {shade}, beside it {beside:?} ({samples})");
        assert!(shade > 0.8, "in the tower's shadow: {shade} ({samples})");
        assert!(
            beside.iter().all(|&t| t < 0.1),
            "beside it: {beside:?} ({samples})"
        );
    }
}

/// A long wall across the end of the cascades (view depths about 5,000 to 14,000) casts one
/// unbroken shadow: no step where the last cascade blends into the far map, and no end of the
/// shadow where the cascades end.
#[test]
fn a_shadow_across_the_end_of_the_cascades_has_no_step() {
    let _far = FarScene::on();
    let frames = [far_land(false, true); 8];
    for samples in [1, 4] {
        let Some((off, _)) = far_sequence(samples, false, &frames) else {
            return;
        };
        let (on, _) = far_sequence(samples, true, &frames).expect("the adapter");
        // y 6000 to 13750 (the wall ends at 14000).
        let along: Vec<f32> = (0..=31)
            .map(|i| taken_at(&off[7], &on[7], 1100.0, 6000.0 + 250.0 * f64::from(i)))
            .collect();
        eprintln!("the wall's shadow along it: {along:.2?} ({samples})");
        assert!(
            along.iter().all(|&t| t > 0.8),
            "the wall's shadow along it {along:?} ({samples})"
        );
        let step = along
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(step < 0.15, "a step of {step} along {along:?} ({samples})");
        // Next to the wall's foot (50 units from it, past the cascades: review of #715, round 2,
        // the far map's slope allowance lifting shadows off feet), the shadow is there too.
        let foot: Vec<f32> = [11_000.0, 12_000.0, 13_000.0]
            .iter()
            .map(|&y| taken_at(&off[7], &on[7], 1350.0, y))
            .collect();
        eprintln!("by the wall's foot: {foot:.2?} ({samples})");
        assert!(foot.iter().all(|&t| t > 0.8), "{foot:?} ({samples})");
    }
}

/// While nothing changes (the camera, the sun and the far casters still), the far map is drawn
/// once and never again.
#[test]
fn the_far_map_is_not_drawn_again_while_nothing_changes() {
    let Some((_, first)) = far_sequence(1, true, &[far_land(true, false); 8]) else {
        return;
    };
    let (_, later) = far_sequence(1, true, &[far_land(true, false); 24]).expect("the adapter");
    eprintln!("the far map drawn {first} times in 8 frames, {later} in 24");
    assert_eq!(first, 1);
    assert_eq!(later, first);
}

/// A new world forgets the far casters as the cascades' (the tower of the world left behind casts
/// nothing in the new one), while in the same world a tower not drawn keeps its far shadow through
/// its grace.
#[test]
fn a_new_world_forgets_the_far_casters() {
    let _far = FarScene::on();
    let in_world = |w, tower| Extra {
        world: Some(w),
        ..far_land(tower, false)
    };
    let same: Vec<Extra> = (0..12).map(|f| in_world(1, f < 6)).collect();
    let new: Vec<Extra> = (0..12)
        .map(|f| in_world(if f < 6 { 1 } else { 2 }, f < 6))
        .collect();
    let Some((off, _)) = far_sequence(1, false, &same) else {
        return;
    };
    let (kept, _) = far_sequence(1, true, &same).expect("the adapter");
    let (forgot, _) = far_sequence(1, true, &new).expect("the adapter");
    let kept = taken_at(&off[11], &kept[11], -500.0, 20_000.0);
    let forgot = taken_at(&off[11], &forgot[11], -500.0, 20_000.0);
    eprintln!("the tower not drawn: same world {kept}, a new world {forgot}");
    assert!(
        kept > 0.8,
        "the same world keeps it through its grace: {kept}"
    );
    assert!(forgot < 0.1, "a new world forgets it: {forgot}");
}

/// Off, the far scene with every far caster and the words in the stream: every frame bit for bit
/// the one without the words.
#[test]
fn off_the_far_scene_changes_nothing() {
    let _far = FarScene::on();
    let frames = [far_land(true, true); 8];
    for samples in [1, 4] {
        let Some((plain, drawn)) = far_sequence(samples, false, &frames) else {
            return;
        };
        assert_eq!(drawn, 0);
        let with_words = {
            let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
            sun_shadow::enable(false);
            let mut b = backend(samples).expect("the adapter");
            lit_sequence(&mut b, &frames, true)
        };
        assert!(plain == with_words, "{samples} samples");
    }
}

/// The sun 10 degrees up toward `azimuth` (radians from +x toward +y), the lowest the light takes.
fn low_sun(azimuth: f64, elevation: f64) -> [f64; 3] {
    let (s, c) = elevation.sin_cos();
    [c * azimuth.cos(), c * azimuth.sin(), s]
}

/// At the lowest sun (10 degrees, from -y) a tall tower's shadow runs along the sun's way far past
/// the far map's old depth (review of #715: it ended in a hard straight line about 950 m out): the
/// shadow is whole all along it, to y 80000.
#[test]
fn at_a_low_sun_a_long_far_shadow_does_not_end_in_a_line() {
    let _far = FarScene::on();
    let sun = low_sun(-std::f64::consts::FRAC_PI_2, 10f64.to_radians());
    let frames = [Extra {
        low_tower: true,
        sun: Some(sun),
        ..far_land(false, false)
    }; 8];
    let Some((off, _)) = far_sequence(1, false, &frames) else {
        return;
    };
    let (on, _) = far_sequence(1, true, &frames).expect("the adapter");
    let along: Vec<f32> = [45_000.0, 60_000.0, 70_000.0, 75_000.0, 80_000.0]
        .iter()
        .map(|&y| taken_at(&off[7], &on[7], 20_000.0, y))
        .collect();
    eprintln!("the low sun's long far shadow along it: {along:.2?}");
    assert!(along.iter().all(|&t| t > 0.8), "{along:?}");
}

/// The sun turning as the game turns it at dusk (timescale 30 at 30 frames a second: 7.4e-5 radians a
/// frame, 10 degrees up): the far map follows it, but is drawn again at most once every
/// `FAR_MIN_FRAMES` frames (review of #715: every few frames before, whole in one frame).
#[test]
fn a_turning_sun_redraws_the_far_map_only_now_and_then() {
    let _far = FarScene::on();
    let frames: Vec<Extra> = (0..64)
        .map(|f| Extra {
            sun: Some(low_sun(0.0, 10f64.to_radians() + 7.4e-5 * f64::from(f))),
            ..far_land(true, false)
        })
        .collect();
    let Some((_, drawn)) = far_sequence(1, true, &frames) else {
        return;
    };
    eprintln!("the far map drawn {drawn} times in 64 frames of a turning sun");
    let most = 64 / sun_shadow::FAR_MIN_FRAMES + 1;
    assert!((3..=most).contains(&drawn), "{drawn} (at most {most})");
}

/// A far caster past the far map's reach seen for one frame (a land block at the horizon as the
/// camera turns) changes nothing: the far map is not drawn again (review of #715: it was added,
/// let go the next frame, and the map drawn again each time).
#[test]
fn a_far_caster_past_the_maps_reach_does_not_redraw_it() {
    let _far = FarScene::on();
    let mut frames = vec![far_land(true, false); 24];
    let plain = far_sequence(1, true, &frames);
    let Some((_, plain)) = plain else {
        return;
    };
    frames[10].beyond = true;
    let (_, with) = far_sequence(1, true, &frames).expect("the adapter");
    eprintln!("the far map drawn {plain} times, with a far caster past its reach seen once {with}");
    assert_eq!(with, plain);
}

/// A far caster the game stops drawing while the camera stands inside its sphere (a land block under
/// cells loaded since: it is never wholly in view) is let go after its grace, and its far shadow goes
/// (review of #715: it kept casting for good).
#[test]
fn a_far_caster_round_the_camera_no_longer_drawn_goes() {
    let _far = FarScene::on();
    let frames: Vec<Extra> = (0..40)
        .map(|f| Extra {
            hill: f < 6,
            ..far_land(false, false)
        })
        .collect();
    let Some((off, _)) = far_sequence(1, false, &frames) else {
        return;
    };
    let (on, _) = far_sequence(1, true, &frames).expect("the adapter");
    let drawn = taken_at(&off[5], &on[5], 1000.0, 20_000.0);
    let gone = taken_at(&off[39], &on[39], 1000.0, 20_000.0);
    eprintln!("the hill's far shadow while drawn {drawn}, long after {gone}");
    assert!(drawn > 0.8, "while drawn: {drawn}");
    assert!(gone < 0.1, "after its grace: {gone}");
}

/// The far map through its first draw and a later redraw (review of #715, round 2): before the far
/// map is in (its first draw is spread over a few frames) the tower's shadow is lit, as off; from
/// then on, through every redraw as the sun turns (30 degrees up from +x, 7.4e-5 radians a
/// frame), its shadow is whole and the land beside it lit in EVERY frame (no half-drawn map, no
/// map read through another's matrix).
#[test]
fn the_far_shadow_holds_in_every_frame_through_redraws() {
    let _far = FarScene::on();
    let frames: Vec<Extra> = (0..64)
        .map(|f| Extra {
            sun: Some(low_sun(0.0, 30f64.to_radians() + 7.4e-5 * f64::from(f))),
            ..far_land(true, false)
        })
        .collect();
    let Some((off, _)) = far_sequence(1, false, &frames) else {
        return;
    };
    let (on, drawn) = far_sequence(1, true, &frames).expect("the adapter");
    assert!(drawn >= 2, "a redraw after the first: {drawn}");
    let (inside, beside) = ((-2000.0, 20_000.0), (-2000.0, 17_000.0));
    let shade: Vec<f32> = (0..frames.len())
        .map(|f| taken_at(&off[f], &on[f], inside.0, inside.1))
        .collect();
    let first = shade
        .iter()
        .position(|&t| t > 0.8)
        .expect("the far shadow comes");
    eprintln!("the far shadow from frame {first}");
    assert!(first <= 8, "{first}");
    let p = pixel([inside.0, inside.1, 0.0]);
    for f in 0..frames.len() {
        let lit = taken_at(&off[f], &on[f], beside.0, beside.1);
        assert!(lit < 0.1, "frame {f}: beside the shadow {lit}");
        if f < first {
            assert_eq!(at(&on[f], p), at(&off[f], p), "frame {f}: no far map yet");
        } else {
            assert!(shade[f] > 0.8, "frame {f}: in the shadow {}", shade[f]);
        }
    }
}

/// At the lowest sun (10 degrees up from -y) turning as at dusk, the tip of the tower's long far
/// shadow (about y 43000) creeps, it does not jump: each redraw is blended in from the map before
/// over a few frames (review of #715, round 2), so from frame to frame the share of the sun taken
/// anywhere along the tip changes little.
#[test]
fn a_far_shadows_tip_creeps_without_steps_as_the_sun_turns() {
    let _far = FarScene::on();
    let frames: Vec<Extra> = (0..64)
        .map(|f| Extra {
            sun: Some(low_sun(
                -std::f64::consts::FRAC_PI_2,
                10f64.to_radians() + 7.4e-5 * f64::from(f),
            )),
            ..far_land(true, false)
        })
        .collect();
    let Some((off, _)) = far_sequence(1, false, &frames) else {
        return;
    };
    let (on, drawn) = far_sequence(1, true, &frames).expect("the adapter");
    let along = |f: usize| -> Vec<f32> {
        (0..=30)
            .map(|i| taken_at(&off[f], &on[f], 2000.0, 41_500.0 + 100.0 * f64::from(i)))
            .collect()
    };
    let mut most = 0.0f32;
    for f in 9..frames.len() {
        let (a, b) = (along(f - 1), along(f));
        for (x, y) in a.iter().zip(&b) {
            most = most.max((x - y).abs());
        }
    }
    eprintln!("the tip's largest change from a frame to the next {most} ({drawn} redraws)");
    assert!(drawn >= 3, "{drawn}");
    assert!(most < FAR_TIP_STEP, "{most}");
}

/// The most the far shadow's tip may change from one frame to the next (see the test above):
/// measured 0.016 with the blend, 0.063 without it (each redraw shown at once).
const FAR_TIP_STEP: f32 = 0.035;

/// At a low sun (10 degrees up toward +y, the camera looking toward it) a tall caster more than
/// 98,304 units out casts its long shadow back into the far map: it is taken (review of #715,
/// round 2: it was refused by a sphere round the camera), and the land at y 80000 is shaded.
#[test]
fn a_tall_caster_far_toward_a_low_sun_shades_the_far_map() {
    let _far = FarScene::on();
    let frames = [Extra {
        sunward: true,
        sun: Some(low_sun(std::f64::consts::FRAC_PI_2, 10f64.to_radians())),
        ..far_land(false, false)
    }; 8];
    let Some((off, _)) = far_sequence(1, false, &frames) else {
        return;
    };
    let (on, _) = far_sequence(1, true, &frames).expect("the adapter");
    let shade = taken_at(&off[7], &on[7], 0.0, 80_000.0);
    eprintln!("the far caster toward the low sun takes {shade} at y 80000");
    assert!(shade > 0.8, "{shade}");
}

/// A far caster whose place changes by 0.1 units for one frame (the game's matrices rounding, a
/// large object nudged) still casts long after (review of #715, round 2: paired as a move, it was
/// left out of the far map for good).
#[test]
fn a_far_caster_nudged_once_still_casts_later() {
    let _far = FarScene::on();
    let frames: Vec<Extra> = (0..50)
        .map(|f| Extra {
            tower_shift: (f == 10).then_some(0.1),
            ..far_land(f != 10, false)
        })
        .collect();
    let Some((off, _)) = far_sequence(1, false, &frames) else {
        return;
    };
    let (on, _) = far_sequence(1, true, &frames).expect("the adapter");
    let later = taken_at(&off[49], &on[49], -500.0, 20_000.0);
    eprintln!("the tower nudged once, 39 frames later: {later}");
    assert!(later > 0.8, "{later}");
}

/// Review of #843: a far caster nudged once (paired as a move) and then out of sight (the camera
/// gone 10,000 past it from frame 15, the tower no longer drawn) is kept as a still one is: at
/// frame 40 the far map still holds it (it was let go as soon as it was out of view, so the far
/// map stopped casting it from out of sight until it settled).
#[test]
fn a_far_caster_nudged_then_out_of_sight_is_kept() {
    let _far = FarScene::on();
    let frames: Vec<Extra> = (0..42)
        .map(|f| match f {
            0..=14 => Extra {
                tower_shift: (f == 10).then_some(0.1),
                ..far_land(f != 10, false)
            },
            _ => Extra {
                camera_shift: Some([0.0, 30_000.0, 0.0]),
                ..far_land(false, false)
            },
        })
        .collect();
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(true);
    let b = backend(1);
    sun_shadow::enable(false);
    let Some(mut b) = b else {
        return;
    };
    let mut seen: Vec<Vec<([f64; 3], f64, bool)>> = Vec::new();
    lit_sequence_with(&mut b, &frames, true, &mut |_, b| {
        seen.push(b.device().map_or_else(Vec::new, |d| d.sun_far_places()));
    });
    // The tower's place as frame 13 left it (a frame's casters are taken as the next is drawn).
    let near_tower =
        |p: &&([f64; 3], f64, bool)| (p.0[1] - 20_000.0).abs() < 2_000.0 && p.1 > 300.0;
    let tower: Vec<_> = seen[14].iter().filter(near_tower).collect();
    eprintln!("the tower at frame 13: {tower:?}");
    assert!(!tower.is_empty(), "{:?}", seen[14]);
    assert!(
        tower.iter().any(|p| p.2),
        "moving after its nudge: {tower:?}"
    );
    let kept: Vec<_> = seen[41].iter().filter(near_tower).collect();
    eprintln!("the tower at frame 40: {kept:?}");
    assert!(
        kept.iter().any(|k| tower
            .iter()
            .any(|t| (0..3).all(|i| (k.0[i] - t.0[i]).abs() < 1.0))),
        "the nudged tower out of sight is kept: {kept:?} (was {tower:?})"
    );
}

/// Report 012's "jittery edges", through the device: a map that takes a turned sun counts its whole
/// steps from where it stood (the centre of the map drawn before), not from the world's origin, so
/// its texel grid turns about the ground round the camera; between turns the steps count from the
/// same point. The sun turns 0.05 degrees a frame at 40 up (each map is past its limit within a
/// few frames) while the camera walks 60 units a frame, so the maps step away from their anchors
/// between turns (a camera standing still keeps every map's centre on its anchor, where the two
/// rules agree).
#[test]
fn a_map_that_takes_a_new_sun_counts_its_steps_from_where_it_stood() {
    let frames: Vec<Extra> = (0..24)
        .map(|f| Extra {
            still_box: true,
            sun: Some(low_sun(
                0.3 + 0.05f64.to_radians() * f64::from(f),
                40f64.to_radians(),
            )),
            camera_shift: Some([60.0 * f64::from(f), 45.0 * f64::from(f), 0.0]),
            ..Extra::default()
        })
        .collect();
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(true);
    let b = backend(1);
    sun_shadow::enable(false);
    let Some(mut b) = b else {
        return;
    };
    let mut before: [Option<sun_shadow::Cascade>; sun_shadow::CASCADES] =
        [None; sun_shadow::CASCADES];
    let (mut turns, mut kept, mut stepped) = (0, 0, 0);
    let mut check = |f: usize, b: &mut WgpuBackend| {
        let now = b.device().expect("the device").sun_cascades_drawn();
        for (k, (was, is)) in before.iter().zip(&now).enumerate() {
            let (Some(was), Some(is)) = (was, is) else {
                continue;
            };
            if was.basis != is.basis {
                // Took the sun: anchored at the centre of the map before.
                let c = was.centre();
                if c != was.anchor {
                    stepped += 1;
                }
                for i in 0..3 {
                    assert!(
                        (is.anchor[i] - c[i]).abs() < 1e-6,
                        "frame {f} map {k}: anchor {:?}, centre before {c:?}",
                        is.anchor
                    );
                }
                turns += 1;
            } else {
                // Kept its sun: the same anchor.
                assert_eq!(is.anchor, was.anchor, "frame {f} map {k}");
                kept += 1;
            }
        }
        before = now;
    };
    lit_sequence_with(&mut b, &frames, true, &mut check);
    eprintln!("maps that took the sun {turns} ({stepped} away from their anchor), kept it {kept}");
    assert!(
        turns >= 4 && kept >= 4 && stepped >= 2,
        "turns {turns}, kept {kept}, away from the anchor {stepped}"
    );
}

/// The far map keeps the same rule: when it is drawn for a turned sun it counts its steps from the
/// centre of the far map before (the far scene, the sun turning as in
/// `a_turning_sun_redraws_the_far_map_only_now_and_then`), the camera walking 300 units a frame so
/// the far map steps away from its anchor between turns.
#[test]
fn the_far_map_takes_a_new_sun_counting_from_where_it_stood() {
    let _far = FarScene::on();
    let frames: Vec<Extra> = (0..64)
        .map(|f| Extra {
            sun: Some(low_sun(0.0, 10f64.to_radians() + 7.4e-5 * f64::from(f))),
            camera_shift: Some([300.0 * f64::from(f), 0.0, 0.0]),
            ..far_land(true, false)
        })
        .collect();
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(true);
    let b = backend(1);
    sun_shadow::enable(false);
    let Some(mut b) = b else {
        return;
    };
    let mut before: Option<sun_shadow::Cascade> = None;
    let (mut turns, mut stepped) = (0, 0);
    let mut check = |f: usize, b: &mut WgpuBackend| {
        let now = b.device().expect("the device").sun_far_drawn();
        if let (Some(was), Some(is)) = (before, now) {
            if was.basis != is.basis {
                let c = was.centre();
                if c != was.anchor {
                    stepped += 1;
                }
                for i in 0..3 {
                    assert!(
                        (is.anchor[i] - c[i]).abs() < 1e-6,
                        "frame {f}: far anchor {:?}, centre before {c:?}",
                        is.anchor
                    );
                }
                turns += 1;
            }
        }
        before = now;
    };
    lit_sequence_with(&mut b, &frames, true, &mut check);
    eprintln!("the far map took the sun {turns} times ({stepped} away from its anchor)");
    assert!(turns >= 2 && stepped >= 1, "turns {turns}, away {stepped}");
}

/// Report 012 (an LOD block's segment the game turned off kept casting): one vertex and index buffer
/// holding two boxes as two index ranges, both drawn, then only the first. The second stands across
/// the view's right edge (its centre in view, so never wholly in view); once the game draws the
/// buffers' other range but not its own for more than its grace, it is let go and its shadow goes,
/// while the first box's stays.
#[test]
fn a_segment_the_game_stops_drawing_while_it_draws_the_rest_of_its_buffers_goes() {
    let frames: Vec<Extra> = (0..26)
        .map(|f| Extra {
            segments: Some(f < 6),
            ..Extra::default()
        })
        .collect();
    let Some(off) = lit_sequence_picture(1, false, &frames) else {
        return;
    };
    let on = lit_sequence_picture(1, true, &frames).expect("the adapter");
    // The first box (x -140 to -100, y 80 to 120, 80 high) throws 60 toward -x: its shadow at
    // x -200 to -140 beside it; the second (x 250 to 290) at x 190 to 250.
    let first = |f: usize| taken_at(&off[f], &on[f], -170.0, 100.0);
    let second = |f: usize| taken_at(&off[f], &on[f], 220.0, 100.0);
    eprintln!(
        "first box {:.2} then {:.2}; second box {:.2} then {:.2}",
        first(4),
        first(25),
        second(4),
        second(25)
    );
    assert!(first(4) > 0.8 && second(4) > 0.8, "both cast while drawn");
    // Review of #844: within its grace (last drawn in frame 5) it still casts.
    assert!(
        second(10) > 0.8,
        "kept through its grace: {:.2}",
        second(10)
    );
    assert!(first(25) > 0.8, "the range still drawn keeps its shadow");
    assert!(
        second(25) < 0.1,
        "the range no longer drawn lets its shadow go"
    );
}

/// Review of #844: a copy of the same range at another place (one mesh, two objects; the copy
/// across the view's right edge) that the game stops drawing while it draws the other copy keeps
/// its shadow under the old rules (not wholly in view: kept), since no OTHER range of the buffers
/// was drawn.
#[test]
fn a_copy_of_a_range_the_game_stops_drawing_keeps_its_shadow() {
    let frames: Vec<Extra> = (0..26)
        .map(|f| Extra {
            seg_copy: Some(f < 6),
            ..Extra::default()
        })
        .collect();
    let Some(off) = lit_sequence_picture(1, false, &frames) else {
        return;
    };
    let on = lit_sequence_picture(1, true, &frames).expect("the adapter");
    // The copy stands where the second range of the test above does (x 250 to 290).
    let copy = |f: usize| taken_at(&off[f], &on[f], 220.0, 100.0);
    let first = |f: usize| taken_at(&off[f], &on[f], -170.0, 100.0);
    eprintln!("the copy {:.2} then {:.2}", copy(4), copy(25));
    assert!(first(4) > 0.8 && copy(4) > 0.8, "both cast while drawn");
    assert!(
        first(25) > 0.8 && copy(25) > 0.8,
        "the copy keeps its shadow"
    );
}

/// Review of #844: a range the game stops drawing whose centre is out of the view but whose sphere
/// reaches into it (the loaded cell beside the player's) goes too, past its grace; its shadow falls
/// into the view.
#[test]
fn a_segment_beside_the_view_the_game_stops_drawing_goes() {
    // The second box: x 290 to 430, centre (360, 100, 40), radius about 83.
    let v = to_view([360.0, 100.0, 40.0]);
    let edge = (v[0] - HALF * v[2]) / (1.0 + HALF * HALF).sqrt();
    assert!(v[0] > HALF * v[2], "its centre out of view: {v:?}");
    assert!(edge < 83.0, "its sphere reaching in: {edge}");
    let frames: Vec<Extra> = (0..26)
        .map(|f| Extra {
            edge_segments: Some(f < 6),
            ..Extra::default()
        })
        .collect();
    let Some(off) = lit_sequence_picture(1, false, &frames) else {
        return;
    };
    let on = lit_sequence_picture(1, true, &frames).expect("the adapter");
    // Its shadow at x 230 to 430: in view at x 250.
    let second = |f: usize| taken_at(&off[f], &on[f], 250.0, 100.0);
    let first = |f: usize| taken_at(&off[f], &on[f], -170.0, 100.0);
    eprintln!(
        "beside the view: {:.2}, {:.2}, then {:.2}",
        second(4),
        second(10),
        second(25)
    );
    assert!(first(4) > 0.8 && second(4) > 0.8, "both cast while drawn");
    assert!(second(10) > 0.8, "kept through its grace");
    assert!(first(25) > 0.8, "the range still drawn keeps its shadow");
    assert!(second(25) < 0.1, "the range beside the view goes");
}

/// Review of #844: the far map lets go a range the game stops drawing while it draws the other
/// range of its buffers, as the cascades do: two towers past the cascades (20,000 out), the second
/// with its centre beyond the view's right edge and its sphere reaching in; once undrawn past its
/// grace its shadow (in view, on the land at x 9,000) goes, the first's stays.
#[test]
fn the_far_map_lets_go_a_segment_the_game_stops_drawing() {
    let _far = FarScene::on();
    let v = to_view([11_500.0, 20_000.0, 2000.0]);
    let edge = (v[0] - HALF * v[2]) / (1.0 + HALF * HALF).sqrt();
    assert!(v[0] > HALF * v[2] && edge < 2400.0, "{v:?} {edge}");
    let frames: Vec<Extra> = (0..44)
        .map(|f| Extra {
            far_segments: Some(f < 6),
            ..far_land(false, false)
        })
        .collect();
    let Some((off, _)) = far_sequence(1, false, &frames) else {
        return;
    };
    let (on, _) = far_sequence(1, true, &frames).expect("the adapter");
    let second = |f: usize| taken_at(&off[f], &on[f], 9_000.0, 20_000.0);
    let first = |f: usize| taken_at(&off[f], &on[f], -3_000.0, 20_000.0);
    eprintln!(
        "far: first {:.2} then {:.2}; second {:.2} then {:.2}",
        first(9),
        first(43),
        second(9),
        second(43)
    );
    // (The far map is in from the fifth frame; frame 9 is within the grace of frame 5.)
    assert!(first(9) > 0.8 && second(9) > 0.8, "both cast while drawn");
    assert!(first(43) > 0.8, "the range still drawn keeps its shadow");
    assert!(second(43) < 0.1, "the far map lets the undrawn range go");
}

/// Review of #817: a far caster that moves (radius about 327, past the cascades' reach, stepping
/// 200 units a frame) leaves only its latest place in the far map: each old place goes at once
/// (paired as the cascades' moves), not after its grace, so no trail of shadows; once it stands
/// still a while its one place is still again.
#[test]
fn a_far_caster_stepping_across_frames_keeps_only_its_latest_place() {
    let _far = FarScene::on();
    let shift = |f: usize| 200.0 * f.min(5) as f32;
    let frames: Vec<Extra> = (0..44)
        .map(|f| Extra {
            small_shift: Some(shift(f)),
            ..far_land(false, false)
        })
        .collect();
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(true);
    let b = backend(1);
    sun_shadow::enable(false);
    let Some(mut b) = b else {
        return;
    };
    let mut seen: Vec<Vec<([f64; 3], f64, bool)>> = Vec::new();
    lit_sequence_with(&mut b, &frames, true, &mut |_, b| {
        let places = b.device().map_or_else(Vec::new, |d| d.sun_far_places());
        eprintln!(
            "far places: {:?}",
            places
                .iter()
                .map(|p| (p.0[0].round(), p.1.round(), p.2))
                .collect::<Vec<_>>()
        );
        seen.push(
            places
                .into_iter()
                .filter(|p| p.1 > 300.0 && p.1 < 400.0)
                .collect(),
        );
    });
    // (A frame's casters are taken as the next frame is drawn: after frame f, frame f - 1's place.)
    for (f, places) in seen.iter().enumerate().skip(1) {
        assert_eq!(places.len(), 1, "frame {f}: {places:?}");
        let x = 2000.0 + f64::from(shift(f - 1));
        assert!(
            (places[0].0[0] - x).abs() < 1.0,
            "frame {f}: {places:?} (at x {x})"
        );
    }
    assert!(seen[2][0].2, "moving while it steps: {:?}", seen[2]);
    assert!(!seen[43][0].2, "still again: {:?}", seen[43]);
}

/// The box's frames with still casters' pipelines made ahead ([`sun_shadow::casters_ahead`]): the
/// first frame (its pipeline sent to a thread of its own), the second after the pipeline is there
/// (the maps drawn from the first frame's casters: none kept), the third (the maps drawn from the
/// second's, the box among them).
fn ahead_frames(samples: u32) -> Option<Vec<Vec<[u8; 4]>>> {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable_debug();
    sun_shadow::casters_ahead_for_tests(Some(true));
    let b = backend(samples);
    sun_shadow::enable(false);
    sun_shadow::casters_ahead_for_tests(None);
    let mut b = b?;
    let first = frame(&mut b, true);
    present(&mut b, 1);
    if let Some(d) = b.device() {
        d.wait_for_sun_variants();
    }
    let second = frame(&mut b, true);
    present(&mut b, 2);
    let third = frame(&mut b, true);
    Some(vec![first, second, third])
}

/// Still casters' pipelines made ahead (`NV_SUN_CASTERS_AHEAD=1`): the render thread does not stop
/// to make the box's pipeline the frame the box is first seen; the box casts from the frame its
/// pipeline is there, the same shadow as with the pipeline made at once one frame earlier, and not
/// before.
#[test]
fn a_still_casters_pipeline_made_ahead_casts_once_it_is_there() {
    use d3d9_stream::hitch::{snapshot, Count};
    let Some(at_once) = picture(1, true, true) else {
        return;
    };
    let before = snapshot();
    let frames = ahead_frames(1).expect("the adapter");
    let now = snapshot();
    let made = |c: Count| now[c as usize] - before[c as usize];
    assert!(
        made(Count::ShadowPipelinesAhead) >= 1,
        "the box's caster pipeline made on a thread of its own: {}",
        made(Count::ShadowPipelinesAhead)
    );
    // (None made on the render thread: `cables_pipelines_made_ahead_are_all_made_off_the_render_thread`
    // counts that on its turn, where no other test makes any.)
    for p in [
        [-50.0, 150.0, 0.0],
        [-60.0, 140.0, 0.0],
        [-45.0, 165.0, 0.0],
    ] {
        let shadowed = at(&at_once, pixel(p));
        assert!(
            at(&frames[1], pixel(p))[2] > shadowed[2],
            "{p:?}: no shadow before the pipeline was there to keep the box: {:?} against {shadowed:?}",
            at(&frames[1], pixel(p))
        );
        assert_eq!(
            at(&frames[2], pixel(p)),
            shadowed,
            "{p:?}: the shadow once the pipeline is there"
        );
    }
}

/// The box's first two frames with still casters' pipelines made ahead on a new backend keeping
/// its pipelines and lists in `dir`; the backend is dropped after (its lists written).
fn ahead_in(dir: &std::path::Path) -> Vec<Vec<[u8; 4]>> {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable_debug();
    sun_shadow::casters_ahead_for_tests(Some(true));
    let b = backend_in(1, Some(dir.to_path_buf()));
    sun_shadow::enable(false);
    sun_shadow::casters_ahead_for_tests(None);
    let mut b = b.expect("the adapter");
    let first = frame(&mut b, true);
    present(&mut b, 1);
    let second = frame(&mut b, true);
    drop(b);
    vec![first, second]
}

/// With still casters' pipelines made ahead, the caster kinds a run sees are kept beside the
/// pipeline list (`casters.txt` in the cache folder) and made before play at the next start, as
/// the listed pipelines are: on a new backend in the same folder the box casts from the second
/// frame, the same shadow as with its pipeline made at once; in a new folder it does not yet.
#[test]
fn a_caster_kind_seen_before_on_this_machine_is_made_at_the_start() {
    let Some(at_once) = picture(1, true, true) else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("nv-rs-sun-casters-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let new = ahead_in(&dir);
    let text = std::fs::read_to_string(dir.join("casters.txt")).expect("the casters' list");
    assert!(
        text.lines().any(|l| l.starts_with("c1 0 0 p2 ")),
        "the box's kind listed: {text}"
    );
    let seen = ahead_in(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    for p in [
        [-50.0, 150.0, 0.0],
        [-60.0, 140.0, 0.0],
        [-45.0, 165.0, 0.0],
    ] {
        let shadowed = at(&at_once, pixel(p));
        assert!(
            at(&new[1], pixel(p))[2] > shadowed[2],
            "{p:?}: a kind never seen casts once its pipeline is there, not in the second frame"
        );
        assert_eq!(
            at(&seen[1], pixel(p)),
            shadowed,
            "{p:?}: a kind seen before is made at the start and casts as made at once"
        );
    }
}

/// With still casters' pipelines made ahead, a cut-out cable geometry's kinds (its own, its ribbons'
/// coverage pipelines' and its list's, each a `c1` line) are listed too and made at the next start:
/// on a new backend in the same folder it casts from its second frame as with its pipelines made at
/// once, every caster pipeline made by the start's preparation (one for each listed kind) and none
/// sent to a thread at a draw (review of #994).
#[test]
fn a_cable_geometry_seen_before_on_this_machine_is_made_at_the_start() {
    use d3d9_stream::hitch::{snapshot, Count};
    let frames = [Extra {
        ribbon_round: true,
        mixed_cut: true,
        ..Extra::default()
    }; 2];
    let Some(at_once) = lit_sequence_picture(1, true, &frames) else {
        return;
    };
    let dir = std::env::temp_dir().join(format!("nv-rs-sun-cables-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    // A start in `dir` drawing `frames` (`pause` ms after each): their pixels, and the caster pipelines
    // made where the render thread waits and on threads of their own meanwhile.
    let start = |frames: &[Extra], pause: u64| {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        sun_shadow::enable(true);
        sun_shadow::casters_ahead_for_tests(Some(true));
        let b = backend_in(1, Some(dir.clone()));
        sun_shadow::enable(false);
        sun_shadow::casters_ahead_for_tests(None);
        let mut b = b.expect("the adapter");
        let before = snapshot();
        let px = lit_sequence_with(&mut b, frames, true, &mut |_, _| {
            std::thread::sleep(std::time::Duration::from_millis(pause))
        });
        let now = snapshot();
        drop(b);
        let d = |c: Count| now[c as usize] - before[c as usize];
        (
            px,
            d(Count::ShadowPipelines),
            d(Count::ShadowPipelinesAhead),
        )
    };
    // The first start: the geometry's pipelines sent one after another over a few frames, listed.
    let _ = start(&[frames[0]; 10], 30);
    let text = std::fs::read_to_string(dir.join("casters.txt")).expect("the casters' list");
    let listed = text.lines().filter(|l| l.starts_with("c1 ")).count() as u64;
    assert!(
        text.lines()
            .any(|l| l.starts_with("c1 ") && l.split(' ').nth(1) != Some("0")),
        "a ribbon's or a list's kind listed: {text}"
    );
    let (px, made, ahead) = start(&frames, 0);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(ahead, 0, "no caster pipeline sent to a thread at a draw");
    assert_eq!(
        made, listed,
        "each listed kind made by the start's preparation, nothing more: {text}"
    );
    let worst = px[1]
        .iter()
        .zip(&at_once[1])
        .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)))
        .max()
        .unwrap_or(0);
    assert!(
        worst <= 1,
        "the second frame as with the pipelines made at once: {worst} apart at most"
    );
}

/// Vertex shaders drawing as [`VS`] does, each another kind of caster (another pipeline): `VS_KIND
/// + k` has `k` instructions more that change nothing drawn ([`vs_kind`]).
const VS_KIND: u32 = 0x1E00;

/// [`VS`]'s tokens with `extra` instructions `mov r1, c0` before its position (nothing drawn
/// changes; the shader is another, and so is its caster's pipeline).
fn vs_kind(extra: u32) -> Vec<u32> {
    let mut vs = vec![0xFFFE_0200];
    vs.extend(constant_table());
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    for _ in 0..extra {
        vs.extend(ins(1, &[0x800F_0001, 0xA0E4_0000]));
    }
    for k in 0..4u32 {
        vs.extend(ins(
            9,
            &[0xC000_0000 | (0x1_0000 << k), 0x90E4_0000, 0xA0E4_0000 | k],
        ));
    }
    vs.push(END);
    vs
}

/// A backend with still casters' pipelines made ahead (the debug step on) and the kinds
/// `VS_KIND + 1..=kinds` made; none without an adapter.
fn ahead_backend(kinds: u32) -> Option<WgpuBackend> {
    kinds_backend(kinds, true)
}

/// [`ahead_backend`] with still casters' pipelines made ahead or not (`ahead`).
fn kinds_backend(kinds: u32, ahead: bool) -> Option<WgpuBackend> {
    sun_shadow::enable_debug();
    sun_shadow::casters_ahead_for_tests(Some(ahead));
    let b = backend(1);
    sun_shadow::enable(false);
    sun_shadow::casters_ahead_for_tests(None);
    let mut b = b?;
    for k in 1..=kinds {
        let t = words(&vs_kind(k));
        run(
            &mut b,
            Cmd::CreateShader {
                device: DEVICE,
                shader: VS_KIND + k,
                pixel: false,
                tokens: Data::Bytes(&t),
            },
        );
    }
    Some(b)
}

/// The box's frame drawn through `VS_KIND + kind`.
fn kind_frame(b: &mut WgpuBackend, kind: u32) -> Vec<[u8; 4]> {
    run(
        b,
        Cmd::SetShader {
            device: DEVICE,
            pixel: false,
            shader: VS_KIND + kind,
        },
    );
    frame(b, true)
}

/// The three points in the box's shadow ([`on_the_box_casts_its_shadow_away_from_the_sun`]).
const IN_SHADOW: [[f64; 3]; 3] = [
    [-50.0, 150.0, 0.0],
    [-60.0, 140.0, 0.0],
    [-45.0, 165.0, 0.0],
];

/// Seven kinds of still caster with their pipelines made ahead and nothing waiting for them: six
/// seen for one frame each and not again, then the seventh drawn on. Its pipeline is sent once
/// one of the six's slots is free (each is kept at a present once made, whether or not its kind
/// is drawn again), and the box casts with it.
#[test]
fn a_seventh_still_kind_casts_after_six_seen_once() {
    let Some(at_once) = picture(1, true, true) else {
        return;
    };
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut b = ahead_backend(7).expect("the adapter");
    let mut last = Vec::new();
    for f in 1..=40u32 {
        last = kind_frame(&mut b, f.min(7));
        present(&mut b, f);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    for p in IN_SHADOW {
        assert_eq!(at(&last, pixel(p)), at(&at_once, pixel(p)), "{p:?}");
    }
}

/// The box's vertex and pixel shaders set to `vs` and `ps`.
fn box_through(b: &mut WgpuBackend, vs: u32, ps: u32) {
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
}

/// The box drawn as a character's part (through `VS_KIND + kind` and a face's pixel shader, with
/// the ribbons' texture bound to its sampler); the ground and the rest as [`frame`] draws them.
fn character_frame(b: &mut WgpuBackend, kind: u32) -> Vec<[u8; 4]> {
    frame_with(b, true, Some((VS_KIND + kind, FACE_CUT_PS)))
}

/// A backend for [`character_frame`] (the switch off: characters' pipelines are on threads of their
/// own whatever it says): [`kinds_backend`] with the face's shaders and the ribbons' texture made.
fn characters_backend(kinds: u32) -> Option<WgpuBackend> {
    let mut b = kinds_backend(kinds, false)?;
    extras(&mut b);
    run(
        &mut b,
        Cmd::SetTexture {
            device: DEVICE,
            sampler: 0,
            texture: RIBBON_TEX,
        },
    );
    Some(b)
}

/// Switched off (the default) too, a character's caster pipeline back from its thread is kept at
/// the next present whether or not its kind is drawn again. Seven kinds of character: six seen for
/// one frame each and not again (a load's fade, a character glimpsed), then the seventh drawn on:
/// it casts, as the first one drawn on does. (Before, each of the six kept one of the characters'
/// six threads until its kind came back: no character kind new after them cast for the rest of
/// the run, and the game's own character shadows are replaced by these.)
#[test]
fn switched_off_a_seventh_character_kind_casts_after_six_seen_once() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let Some(mut b) = characters_backend(1) else {
        return;
    };
    let mut reference = Vec::new();
    for f in 1..=12u32 {
        reference = character_frame(&mut b, 1);
        present(&mut b, f);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let mut b = characters_backend(7).expect("the adapter");
    let first = character_frame(&mut b, 1);
    present(&mut b, 1);
    let mut last = Vec::new();
    for f in 2..=40u32 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        last = character_frame(&mut b, f.min(7));
        present(&mut b, f);
    }
    for p in IN_SHADOW {
        assert_ne!(
            at(&first, pixel(p)),
            at(&reference, pixel(p)),
            "the character casts once its pipeline is there: {p:?}"
        );
        assert_eq!(at(&last, pixel(p)), at(&reference, pixel(p)), "{p:?}");
    }
}

/// A box casting through one kind whose draw moves to another kind whose pipeline takes longer than
/// the let-go's grace ([`sun_shadow::GRACE`] frames; a slow compile, or a flood of new kinds holding
/// the threads): where it cast stays meanwhile, the shadow in every frame (review of #994).
#[test]
fn a_caster_switching_to_a_slow_kind_keeps_its_shadow_past_the_grace() {
    let Some(at_once) = picture(1, true, true) else {
        return;
    };
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut b = ahead_backend(2).expect("the adapter");
    let mut px = Vec::new();
    for f in 1..=6u32 {
        px = kind_frame(&mut b, 1);
        present(&mut b, f);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    for p in IN_SHADOW {
        assert_eq!(
            at(&px, pixel(p)),
            at(&at_once, pixel(p)),
            "kind 1 casts: {p:?}"
        );
    }
    // Kind 2's pipeline takes 0.6 s on its thread: about 25 frames here, three times the grace.
    struct Delay;
    impl Drop for Delay {
        fn drop(&mut self) {
            sun_shadow::caster_delay_for_tests(0);
        }
    }
    let _delay = Delay;
    sun_shadow::caster_delay_for_tests(600);
    let frames = 7 + 3 * sun_shadow::GRACE as u32;
    for f in 7..=frames {
        let px = kind_frame(&mut b, 2);
        present(&mut b, f);
        std::thread::sleep(std::time::Duration::from_millis(20));
        for p in IN_SHADOW {
            assert_eq!(at(&px, pixel(p)), at(&at_once, pixel(p)), "frame {f} {p:?}");
        }
    }
}

/// A box casting through one kind whose draw moves to another kind, made ahead: its shadow stays
/// in every frame while the new kind's pipeline is on its way and after.
#[test]
fn a_caster_switching_kind_keeps_its_shadow() {
    let Some(at_once) = picture(1, true, true) else {
        return;
    };
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut b = ahead_backend(2).expect("the adapter");
    let _ = kind_frame(&mut b, 1);
    present(&mut b, 1);
    if let Some(d) = b.device() {
        d.wait_for_sun_variants();
    }
    for f in 2..=30u32 {
        let px = kind_frame(&mut b, if f < 6 { 1 } else { 2 });
        present(&mut b, f);
        std::thread::sleep(std::time::Duration::from_millis(20));
        if f >= 3 {
            for p in IN_SHADOW {
                assert_eq!(at(&px, pixel(p)), at(&at_once, pixel(p)), "frame {f} {p:?}");
            }
        }
    }
}

/// With still casters' pipelines made ahead, a cut-out cable geometry's ribbon and list pipelines
/// are made on threads of their own too: the render thread makes no caster pipeline, and once
/// they are all there the shadows are the ones made at once.
#[test]
fn cables_pipelines_made_ahead_are_all_made_off_the_render_thread() {
    use d3d9_stream::hitch::{snapshot, Count};
    let frames = [Extra {
        ribbon_round: true,
        mixed: true,
        mixed_cut: true,
        ..Extra::default()
    }; 6];
    let Some(off) = lit_sequence_picture(1, true, &frames) else {
        return;
    };
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    sun_shadow::enable(true);
    sun_shadow::casters_ahead_for_tests(Some(true));
    let b = backend(1);
    sun_shadow::enable(false);
    sun_shadow::casters_ahead_for_tests(None);
    let mut b = b.expect("the adapter");
    let before = snapshot();
    let on = lit_sequence(&mut b, &frames, true);
    let now = snapshot();
    let d = |c: Count| now[c as usize] - before[c as usize];
    assert_eq!(
        d(Count::ShadowPipelines),
        0,
        "no caster pipeline made on the render thread"
    );
    assert!(
        d(Count::ShadowPipelinesAhead) >= 3,
        "the casters', the ribbons' and the list's: {}",
        d(Count::ShadowPipelinesAhead)
    );
    // The same shadows (coverage added in another order may round a last bit apart).
    let worst = on[5]
        .iter()
        .zip(&off[5])
        .flat_map(|(a, b)| a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)))
        .max()
        .unwrap_or(0);
    assert!(
        worst <= 1,
        "the shadows once all are there: {worst} apart at most"
    );
}

/// With still casters' pipelines NOT made ahead (the switch off), a character's cut-out cable
/// geometries (drawn through a face's pixel shader) are as before the switch: their own pipeline
/// made on a thread of its own, their ribbons' and list's made at once on the render thread.
#[test]
fn switched_off_a_characters_ribbon_and_list_pipelines_are_made_at_once() {
    use d3d9_stream::hitch::{snapshot, Count};
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    assert!(sun_shadow::characters(), "characters cast (the default)");
    // The caster pipelines made on threads of their own and at once, the cut-outs drawn through a
    // face's pixel shader or not.
    let made = |face: bool| {
        let frames = [Extra {
            ribbon_round: true,
            mixed_cut: true,
            face_cutouts: face,
            ..Extra::default()
        }; 4];
        sun_shadow::enable(true);
        sun_shadow::casters_ahead_for_tests(Some(false));
        let b = backend(1);
        sun_shadow::enable(false);
        sun_shadow::casters_ahead_for_tests(None);
        let mut b = b?;
        let before = snapshot();
        let _ = lit_sequence(&mut b, &frames, true);
        let now = snapshot();
        let d = |c: Count| now[c as usize] - before[c as usize];
        Some((d(Count::ShadowPipelinesAhead), d(Count::ShadowPipelines)))
    };
    let Some(still) = made(false) else {
        return;
    };
    let face = made(true).expect("the adapter");
    assert_eq!(still.0, 0, "switched off, no still caster's made ahead");
    assert!(
        still.1 >= 3,
        "the cut-outs', the ribbons' and the list's: {}",
        still.1
    );
    assert_eq!(
        face.0, 1,
        "only the face's own pipeline made on a thread of its own"
    );
    assert_eq!(
        face.1,
        still.1 - 1,
        "the face's ribbons' and list's made at once, as a still caster's"
    );
}
