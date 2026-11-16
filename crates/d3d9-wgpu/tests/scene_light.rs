//! The new lighting (`d3d9_wgpu::scene_light`) through the whole backend on the GPU: with it off
//! the frame is bit for bit what it is without it, whatever the stream carries; with it on, a
//! crease darkens, the sky and what is drawn after the depth-only clear (the first-person arms)
//! do not change.
//!
//! The scene and shaders are built here (no game file). A machine without a GPU adapter skips
//! with a message.

use d3d9_stream::{Backend, Cmd, Data};
use d3d9_wgpu::scene_light::{self, Params};
use d3d9_wgpu::{Output, WgpuBackend};

const END: u32 = 0x0000_FFFF;
const DEVICE: u32 = 0x1000;
const BACK: u32 = 0x1100;
const RT: u32 = 0x1200;
const DS: u32 = 0x1210;
const VS: u32 = 0x1300;
const PS: u32 = 0x1310;
const DECL: u32 = 0x1400;
const A8R8G8B8: u32 = 21;
const D24S8: u32 = 75;
const SIZE: u32 = 256;
const NEAR: f32 = 10.0;
const FAR: f32 = 1000.0;
/// The grey everything starts as (and the sky stays).
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

/// Direct3D's depth of a point `z` units ahead.
fn depth(z: f32) -> f32 {
    FAR / (FAR - NEAR) * (1.0 - NEAR / z)
}

/// A 75-degree horizontal view of a square target.
fn params() -> Params {
    let r = 37.5f32.to_radians().tan();
    Params {
        near: NEAR,
        far: FAR,
        left: -r,
        right: r,
        bottom: -r,
        top: r,
        // The sun up and behind the camera.
        sun: [0.0, 0.8, -0.6],
        shadow: 0.5,
    }
}

fn run(b: &mut WgpuBackend, cmd: Cmd<Data<'_>>) {
    b.execute(cmd);
}

/// A backend with the scene's target and depth (`samples` per pixel), a pass-through vertex
/// shader (positions in clip space) and a pixel shader writing `c0`; none without an adapter.
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
        eprintln!("no GPU adapter: the new lighting's tests are skipped");
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
    let mut vs = vec![0xFFFE_0200];
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(1, &[0xC00F_0000, 0x90E4_0000]));
    vs.push(END);
    let mut ps = vec![0xFFFF_0300];
    ps.extend(ins(1, &[0x800F_0800, 0xA0E4_0000]));
    ps.push(END);
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
    Some(b)
}

fn colour(b: &mut WgpuBackend, c: [f32; 4]) {
    let regs = floats(&c);
    run(
        b,
        Cmd::SetConstants {
            device: DEVICE,
            kind: 3,
            start: 0,
            count: 1,
            data: Data::Bytes(&regs),
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

/// Triangles (clip-space x, y and depth per corner).
fn triangles(b: &mut WgpuBackend, corners: &[[f32; 3]]) {
    let v: Vec<f32> = corners.iter().flat_map(|c| [c[0], c[1], c[2], 1.0]).collect();
    let bytes = floats(&v);
    run(
        b,
        Cmd::DrawPrimitiveUp {
            device: DEVICE,
            kind: 4,
            count: corners.len() as u32 / 3,
            stride: 16,
            vertices: Data::Bytes(&bytes),
        },
    );
}

/// The frame: a valley of two walls meeting along the middle column (near at the sides, far in the
/// middle) below a strip of sky, the depth-only clear (after the new lighting's parameters when
/// `light`), a small near square drawn after it (the arms), and the target's pixels (B, G, R, A).
fn frame(b: &mut WgpuBackend, light: bool) -> Vec<[u8; 4]> {
    clear(b, 7);
    colour(b, [0.5, 0.5, 0.5, 1.0]);
    let (side, middle) = (depth(120.0), depth(250.0));
    let (lo, hi) = (-1.0, 0.5);
    triangles(
        b,
        &[
            [-1.0, lo, side],
            [-1.0, hi, side],
            [0.0, hi, middle],
            [-1.0, lo, side],
            [0.0, hi, middle],
            [0.0, lo, middle],
            [0.0, lo, middle],
            [0.0, hi, middle],
            [1.0, hi, side],
            [0.0, lo, middle],
            [1.0, hi, side],
            [1.0, lo, side],
        ],
    );
    if light {
        let m = params().to_words();
        run(
            b,
            Cmd::SetTransform {
                device: DEVICE,
                state: scene_light::STATE,
                matrix: Data::from_words(&m),
            },
        );
    }
    clear(b, 2);
    colour(b, [0.25, 0.75, 0.5, 1.0]);
    let near = depth(12.0);
    triangles(
        b,
        &[
            [-0.1, -0.9, near],
            [-0.1, -0.6, near],
            [0.1, -0.6, near],
            [-0.1, -0.9, near],
            [0.1, -0.6, near],
            [0.1, -0.9, near],
        ],
    );
    let d = b.device().expect("the device");
    let (_, _, px) = d.read_surface_pixels(RT).expect("the target's pixels");
    px.chunks_exact(4)
        .map(|p| [p[0], p[1], p[2], p[3]])
        .collect()
}

/// The frame on a new backend with the new lighting `on` for its device; none without an adapter.
fn picture(samples: u32, on: bool, light: bool) -> Option<Vec<[u8; 4]>> {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    scene_light::enable(on);
    let b = backend(samples);
    scene_light::enable(false);
    let mut b = b?;
    Some(frame(&mut b, light))
}

fn at(px: &[[u8; 4]], x: u32, y: u32) -> [u8; 4] {
    px[(y * SIZE + x) as usize]
}

/// Off (the default): the parameters in the stream change nothing; the frame is bit for bit the
/// one without them.
#[test]
fn off_the_frame_is_unchanged() {
    for samples in [1, 4] {
        let Some(plain) = picture(samples, false, false) else {
            return;
        };
        let with_words = picture(samples, false, true).expect("the adapter");
        assert!(plain == with_words, "{samples} samples");
        // The scene is there: the sky grey, the walls drawn.
        assert_eq!(at(&plain, 5, 5), [0x80, 0x80, 0x80, 0xFF]);
    }
}

/// On: the crease darkens, more than the open wall; the sky and the square drawn after the
/// depth-only clear are exactly as without it.
#[test]
fn on_the_crease_darkens_and_nothing_else() {
    for samples in [1, 4] {
        let Some(plain) = picture(samples, false, false) else {
            return;
        };
        let lit = picture(samples, true, true).expect("the adapter");
        // The sky: the top eighth (the walls end at clip y 0.5, a quarter down).
        for y in 0..SIZE / 8 {
            for x in 0..SIZE {
                assert_eq!(at(&lit, x, y), at(&plain, x, y), "sky {x} {y} ({samples})");
            }
        }
        // The square drawn after the clear (clip -0.1..0.1, -0.9..-0.6): pixels 115..141, 205..243.
        for y in 210..238 {
            for x in 118..138 {
                assert_eq!(at(&lit, x, y), at(&plain, x, y), "arms {x} {y} ({samples})");
            }
        }
        // The crease (the middle column) is darker; the open wall halfway to the side is darkened
        // less, if at all.
        let row = 140;
        let crease = at(&lit, SIZE / 2, row)[2];
        let crease_plain = at(&plain, SIZE / 2, row)[2];
        let open = at(&lit, SIZE / 4, row)[2];
        assert!(
            crease + 8 < crease_plain,
            "crease {crease} of {crease_plain} ({samples})"
        );
        assert!(open > crease + 4, "open {open} crease {crease} ({samples})");
    }
}
