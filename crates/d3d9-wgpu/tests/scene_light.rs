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
        strength: scene_light::STRENGTH,
    }
}

fn run(b: &mut WgpuBackend, cmd: Cmd<Data<'_>>) {
    b.execute(cmd);
}

/// A backend with the scene's target and depth (`samples` per pixel), a pass-through vertex
/// shader (positions in clip space) and a pixel shader writing `c0`; none without an adapter.
fn backend(samples: u32) -> Option<WgpuBackend> {
    backend_sized(SIZE, SIZE, samples, A8R8G8B8)
}

/// [`backend`] with a `width` by `height` target of `format` (and the back buffer of that size).
fn backend_sized(width: u32, height: u32, samples: u32, format: u32) -> Option<WgpuBackend> {
    let mut b = WgpuBackend::new(Output::Offscreen);
    let present = words(&[width, height, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
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
    for (surface, is_depth, format) in [(RT, false, format), (DS, true, D24S8)] {
        run(
            &mut b,
            Cmd::CreateSurface {
                device: DEVICE,
                surface,
                depth: is_depth,
                width,
                height,
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
    let v: Vec<f32> = corners
        .iter()
        .flat_map(|c| [c[0], c[1], c[2], 1.0])
        .collect();
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

/// Where a frame carries the new lighting's parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Points {
    /// Nowhere.
    None,
    /// Just before the depth-only clear (the game's, before the first-person arms).
    Clear,
    /// Where the world is drawn (the frame draw's own point, first and third person).
    World,
    /// At both, as a first-person frame has them.
    Both,
}

/// The new lighting's parameters at `strength`, into the stream.
fn light_here(b: &mut WgpuBackend, strength: f32) {
    let m = Params {
        strength,
        ..params()
    }
    .to_words();
    run(
        b,
        Cmd::SetTransform {
            device: DEVICE,
            state: scene_light::STATE,
            matrix: Data::from_words(&m),
        },
    );
}

/// The target's pixels (B, G, R, A).
fn pixels(b: &mut WgpuBackend, surface: u32) -> Vec<[u8; 4]> {
    let d = b.device().expect("the device");
    let (_, _, px) = d.read_surface_pixels(surface).expect("the target's pixels");
    px.chunks_exact(4)
        .map(|p| [p[0], p[1], p[2], p[3]])
        .collect()
}

/// The world of [`frame`]: a valley of two walls meeting along the middle column (near at the
/// sides, far in the middle) below a strip of sky.
fn world(b: &mut WgpuBackend) {
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
}

/// The frame: the [`world`], the depth-only clear (after the new lighting's parameters when
/// `light`), a small near valley drawn after it (the arms: a crease the step would darken if it ran
/// after them), and the target's pixels.
fn frame(b: &mut WgpuBackend, light: bool) -> Vec<[u8; 4]> {
    let points = if light { Points::Clear } else { Points::None };
    frame_with(b, points, true, scene_light::STRENGTH)
}

/// The frame with the parameters at `points` (at `strength`): the [`world`]; then, in first person
/// (`depth_clear`), the depth-only clear; then the small near valley (the arms in first person,
/// in third person something drawn after the world such as the HUD's: either way a crease the step
/// would darken if it ran after it); and the target's pixels.
fn frame_with(
    b: &mut WgpuBackend,
    points: Points,
    depth_clear: bool,
    strength: f32,
) -> Vec<[u8; 4]> {
    world(b);
    if matches!(points, Points::World | Points::Both) {
        light_here(b, strength);
    }
    if depth_clear {
        if matches!(points, Points::Clear | Points::Both) {
            light_here(b, strength);
        }
        clear(b, 2);
    }
    after_world(b);
    pixels(b, RT)
}

/// The small near valley drawn after the world (clip x -0.1..0.1, y -0.9..-0.6).
fn after_world(b: &mut WgpuBackend) {
    colour(b, [0.25, 0.75, 0.5, 1.0]);
    let (near, deep) = (depth(12.0), depth(25.0));
    let (lo, hi) = (-0.9, -0.6);
    triangles(
        b,
        &[
            [-0.1, lo, near],
            [-0.1, hi, near],
            [0.0, hi, deep],
            [-0.1, lo, near],
            [0.0, hi, deep],
            [0.0, lo, deep],
            [0.0, lo, deep],
            [0.0, hi, deep],
            [0.1, hi, near],
            [0.0, lo, deep],
            [0.1, hi, near],
            [0.1, lo, near],
        ],
    );
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

/// [`frame_with`] on a new backend with the new lighting `on`; none without an adapter.
fn picture_with(
    samples: u32,
    on: bool,
    points: Points,
    depth_clear: bool,
    strength: f32,
) -> Option<Vec<[u8; 4]>> {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    scene_light::enable(on);
    let b = backend(samples);
    scene_light::enable(false);
    let mut b = b?;
    Some(frame_with(&mut b, points, depth_clear, strength))
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
        // The valley drawn after the clear (clip -0.1..0.1, -0.9..-0.6): pixels 115..141, 205..243.
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

/// Third person (no depth-only clear; the parameters where the world is drawn): the crease
/// darkens; the sky and what is drawn after the world (the HUD's stage) are exactly as without it.
/// Off, the same frame with the parameters is bit for bit the one without them.
#[test]
fn third_person_the_crease_darkens_and_what_follows_does_not() {
    let s = scene_light::STRENGTH;
    for samples in [1, 4] {
        let Some(plain) = picture_with(samples, false, Points::None, false, s) else {
            return;
        };
        let off = picture_with(samples, false, Points::World, false, s).expect("the adapter");
        assert!(plain == off, "off: unchanged ({samples})");
        let lit = picture_with(samples, true, Points::World, false, s).expect("the adapter");
        for y in 0..SIZE / 8 {
            for x in 0..SIZE {
                assert_eq!(at(&lit, x, y), at(&plain, x, y), "sky {x} {y} ({samples})");
            }
        }
        for y in 210..238 {
            for x in 118..138 {
                assert_eq!(
                    at(&lit, x, y),
                    at(&plain, x, y),
                    "after {x} {y} ({samples})"
                );
            }
        }
        let row = 140;
        let crease = at(&lit, SIZE / 2, row)[2];
        let crease_plain = at(&plain, SIZE / 2, row)[2];
        assert!(
            crease + 8 < crease_plain,
            "crease {crease} of {crease_plain} ({samples})"
        );
    }
}

/// First person through both points (where the world is drawn, then before the depth-only clear):
/// the step runs once a frame, so the picture is bit for bit the one with the parameters at the
/// clear alone (not darkened twice), and the arms are untouched. Off, nothing changes.
#[test]
fn first_person_through_both_points_runs_once() {
    let s = scene_light::STRENGTH;
    for samples in [1, 4] {
        let Some(at_clear) = picture_with(samples, true, Points::Clear, true, s) else {
            return;
        };
        let both = picture_with(samples, true, Points::Both, true, s).expect("the adapter");
        assert!(both == at_clear, "once a frame ({samples})");
        let world_only = picture_with(samples, true, Points::World, true, s).expect("the adapter");
        assert!(
            world_only == at_clear,
            "the same step at either point ({samples})"
        );
        let plain = picture_with(samples, false, Points::None, true, s).expect("the adapter");
        let off = picture_with(samples, false, Points::Both, true, s).expect("the adapter");
        assert!(plain == off, "off: unchanged ({samples})");
        assert!(plain != at_clear, "on: the step ran ({samples})");
    }
}

/// The strength the run chose reaches the shader: the crease is darker at full strength than at a
/// faint one, and the default lies between.
#[test]
fn a_stronger_strength_darkens_the_crease_more() {
    let crease = |strength: f32| -> Option<u8> {
        let px = picture_with(1, true, Points::World, false, strength)?;
        Some(at(&px, SIZE / 2, 140)[2])
    };
    let Some(faint) = crease(0.1) else {
        return;
    };
    let default = crease(scene_light::STRENGTH).expect("the adapter");
    let full = crease(1.0).expect("the adapter");
    eprintln!("crease red: faint {faint}, default {default}, full {full}");
    assert!(
        full + 4 < default && default + 4 < faint,
        "{full} {default} {faint}"
    );
}

/// A target smaller than the screen (a water reflection's, a shadow map's) bound where the
/// parameters come is never shaded, and the frame's run is not used up by it: the screen-sized
/// scene bound afterwards still is.
#[test]
fn a_smaller_target_is_not_shaded() {
    const SMALL_RT: u32 = 0x1220;
    const SMALL_DS: u32 = 0x1230;
    const HALF: u32 = SIZE / 2;
    let small_picture = |on: bool| -> Option<(Vec<[u8; 4]>, Vec<[u8; 4]>)> {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        scene_light::enable(on);
        let b = backend(4);
        scene_light::enable(false);
        let mut b = b?;
        for (surface, is_depth, format) in [(SMALL_RT, false, A8R8G8B8), (SMALL_DS, true, D24S8)] {
            run(
                &mut b,
                Cmd::CreateSurface {
                    device: DEVICE,
                    surface,
                    depth: is_depth,
                    width: HALF,
                    height: HALF,
                    format,
                    multisample: 4,
                    quality: 0,
                    flag: 0,
                },
            );
        }
        let bind = |b: &mut WgpuBackend, rt: u32, ds: u32| {
            run(
                b,
                Cmd::SetRenderTarget {
                    device: DEVICE,
                    index: 0,
                    surface: rt,
                },
            );
            run(
                b,
                Cmd::SetDepthStencilSurface {
                    device: DEVICE,
                    surface: ds,
                },
            );
        };
        bind(&mut b, SMALL_RT, SMALL_DS);
        run(
            &mut b,
            Cmd::SetViewport {
                device: DEVICE,
                viewport: [0, 0, HALF, HALF, 0f32.to_bits(), 1f32.to_bits()],
            },
        );
        world(&mut b);
        light_here(&mut b, scene_light::STRENGTH);
        let small = pixels(&mut b, SMALL_RT);
        bind(&mut b, RT, DS);
        run(
            &mut b,
            Cmd::SetViewport {
                device: DEVICE,
                viewport: [0, 0, SIZE, SIZE, 0f32.to_bits(), 1f32.to_bits()],
            },
        );
        let scene = frame_with(&mut b, Points::World, false, scene_light::STRENGTH);
        Some((small, scene))
    };
    let Some((small_off, scene_off)) = small_picture(false) else {
        return;
    };
    let (small_on, scene_on) = small_picture(true).expect("the adapter");
    assert!(small_on == small_off, "the smaller target is untouched");
    let row = 140;
    assert!(
        at(&scene_on, SIZE / 2, row)[2] + 8 < at(&scene_off, SIZE / 2, row)[2],
        "the scene is still shaded"
    );
}

/// Off, the multisampled depth surface is made exactly as before (a render attachment only); on,
/// it can also be read (the step reads it).
#[test]
fn off_the_depth_surface_is_made_as_before() {
    for (on, readable) in [(false, false), (true, true)] {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        scene_light::enable(on);
        let b = backend(4);
        scene_light::enable(false);
        let Some(mut b) = b else {
            return;
        };
        let usage = b
            .device()
            .expect("the device")
            .surface_usage(DS)
            .expect("the depth");
        assert!(usage.contains(wgpu::TextureUsages::RENDER_ATTACHMENT));
        assert_eq!(
            usage.contains(wgpu::TextureUsages::TEXTURE_BINDING),
            readable,
            "on {on}: {usage:?}"
        );
        if !on {
            assert_eq!(usage, wgpu::TextureUsages::RENDER_ATTACHMENT);
        }
    }
}

/// Grass: a comb of thin upright blades standing on a floor darkens the floor around their feet
/// only a little and the ground seen between them hardly at all (thin occluders weigh little), where
/// a solid wall darkens its foot more.
#[test]
fn a_comb_of_blades_darkens_the_floor_only_a_little() {
    // A larger target than the others: the step's reach on screen is a part of the width.
    const COMB: u32 = 1024;
    // The floor: from 60 units at the bottom to 300 at clip y 0.4; the blades stand on it at clip
    // y -0.4, two pixels wide every eight, up to clip y 0; the wall is the same, solid.
    let floor = |y: f32| {
        let t = (y + 1.0) / 1.4;
        depth(60.0) + t * (depth(300.0) - depth(60.0))
    };
    let base = floor(-0.4);
    let picture_of = |on: bool, comb: bool| -> Option<Vec<[u8; 4]>> {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        scene_light::enable(on);
        let b = backend_sized(COMB, COMB, 1, A8R8G8B8);
        scene_light::enable(false);
        let mut b = b?;
        clear(&mut b, 7);
        colour(&mut b, [0.5, 0.5, 0.5, 1.0]);
        triangles(
            &mut b,
            &[
                [-1.0, -1.0, floor(-1.0)],
                [-1.0, 0.4, floor(0.4)],
                [1.0, 0.4, floor(0.4)],
                [-1.0, -1.0, floor(-1.0)],
                [1.0, 0.4, floor(0.4)],
                [1.0, -1.0, floor(-1.0)],
            ],
        );
        let px = 2.0 / COMB as f32;
        let mut quads = Vec::new();
        let mut x = -0.5;
        while x < 0.5 {
            let w = if comb { 2.0 * px } else { 8.0 * px };
            for c in [
                [x, -0.4, base],
                [x, 0.0, base],
                [x + w, 0.0, base],
                [x, -0.4, base],
                [x + w, 0.0, base],
                [x + w, -0.4, base],
            ] {
                quads.push(c);
            }
            x += 8.0 * px;
        }
        triangles(&mut b, &quads);
        let m = params().to_words();
        run(
            &mut b,
            Cmd::SetTransform {
                device: DEVICE,
                state: scene_light::STATE,
                matrix: Data::from_words(&m),
            },
        );
        clear(&mut b, 2);
        let d = b.device().expect("the device");
        let (_, _, px) = d.read_surface_pixels(RT).expect("the target's pixels");
        Some(
            px.chunks_exact(4)
                .map(|p| [p[0], p[1], p[2], p[3]])
                .collect(),
        )
    };
    let Some(comb_off) = picture_of(false, true) else {
        return;
    };
    let comb_on = picture_of(true, true).expect("the adapter");
    let wall_off = picture_of(false, false).expect("the adapter");
    let wall_on = picture_of(true, false).expect("the adapter");
    // The floor just below the blades' feet (they stand on row 716), across the comb.
    // The mean red of rows `rows`, across the comb.
    let mean = |p: &[[u8; 4]], rows: std::ops::Range<u32>| -> f32 {
        let n = rows.len() as f32;
        rows.map(|y| {
            (300..724)
                .map(|x| f32::from(p[(x + y * COMB) as usize][2]))
                .sum::<f32>()
        })
        .sum::<f32>()
            / (n * 424.0)
    };
    // Just below the feet (row 716): the floor outside the comb.
    let comb = mean(&comb_on, 720..724) / mean(&comb_off, 720..724);
    let wall = mean(&wall_on, 720..724) / mean(&wall_off, 720..724);
    // Inside the comb, halfway up the blades: blades and the floor seen between them (the grass
    // itself; at the very feet the blades and floor meet as any wall and floor do).
    let inside = mean(&comb_on, 560..640) / mean(&comb_off, 560..640);
    eprintln!("kept: at the feet blades {comb:.4}, wall {wall:.4}; among the blades {inside:.4}");
    assert!(
        inside > 0.95,
        "the blades and the floor among them lose at most 5 %: {inside}"
    );
    assert!(wall < 0.97, "a solid wall darkens its foot: {wall}");
    assert!(
        comb > 0.9,
        "a comb of blades darkens the floor by at most 10 %: {comb}"
    );
    assert!(comb > wall, "blades {comb} darken less than a wall {wall}");
}

/// What the step costs on this machine's graphics card at 2560x1440 with 8x multisampling and the
/// game's A16B16G16R16F scene target (by hand: `--ignored --nocapture`): the same frames with and
/// without the step, each batch ended by a read-back that waits for the card; the difference a
/// frame is printed. Not a check (a shared card's timing varies): a measurement.
#[test]
#[ignore]
fn cost_at_2560x1440() {
    const W: u32 = 2560;
    const H: u32 = 1440;
    const A16B16G16R16F: u32 = 113;
    const FRAMES: u32 = 300;
    let time = |on: bool| -> Option<f64> {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        scene_light::enable(on);
        let b = backend_sized(W, H, 8, A16B16G16R16F);
        scene_light::enable(false);
        let mut b = b?;
        run(
            &mut b,
            Cmd::SetViewport {
                device: DEVICE,
                viewport: [0, 0, W, H, 0f32.to_bits(), 1f32.to_bits()],
            },
        );
        let mut best = f64::MAX;
        for _ in 0..3 {
            let start = std::time::Instant::now();
            for frame in 0..FRAMES {
                // The valley and the depth-only clear, as `frame` draws them; `Present` ends it.
                clear(&mut b, 7);
                colour(&mut b, [0.5, 0.5, 0.5, 1.0]);
                let (side, middle) = (depth(120.0), depth(250.0));
                triangles(
                    &mut b,
                    &[
                        [-1.0, -1.0, side],
                        [-1.0, 0.5, side],
                        [0.0, 0.5, middle],
                        [-1.0, -1.0, side],
                        [0.0, 0.5, middle],
                        [0.0, -1.0, middle],
                        [0.0, -1.0, middle],
                        [0.0, 0.5, middle],
                        [1.0, 0.5, side],
                        [0.0, -1.0, middle],
                        [1.0, 0.5, side],
                        [1.0, -1.0, side],
                    ],
                );
                let m = params().to_words();
                run(
                    &mut b,
                    Cmd::SetTransform {
                        device: DEVICE,
                        state: scene_light::STATE,
                        matrix: Data::from_words(&m),
                    },
                );
                clear(&mut b, 2);
                run(
                    &mut b,
                    Cmd::Present {
                        device: DEVICE,
                        source: None,
                        dest: None,
                        window: 0,
                        dirty: false,
                        model: 0,
                        frame: frame + 1,
                    },
                );
            }
            let d = b.device().expect("the device");
            let _ = d.read_surface_pixels(RT);
            best = best.min(start.elapsed().as_secs_f64() / f64::from(FRAMES));
        }
        Some(best)
    };
    let Some(off) = time(false) else {
        return;
    };
    let on = time(true).expect("the adapter");
    eprintln!(
        "2560x1440, 8x: {:.3} ms a frame off, {:.3} ms on: the step costs {:.3} ms",
        off * 1e3,
        on * 1e3,
        (on - off) * 1e3
    );
}
