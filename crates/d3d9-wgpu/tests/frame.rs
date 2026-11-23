//! The frame-level calls Doc's house and the exterior need beyond the instructions: user clip
//! planes, occlusion counts, `GetRenderTargetData` and `ReadSurface`, clears of part of a target,
//! and NVIDIA's and ATI's alpha to coverage switches, each run through the whole backend on the
//! GPU and checked against what Direct3D 9 defines.
//!
//! The shaders and calls are built here (no game file). A machine without a GPU adapter skips
//! with a message.

use d3d9_stream::{Backend, Cmd, Data, Reply, Rows};
use d3d9_wgpu::{Output, WgpuBackend};

const END: u32 = 0x0000_FFFF;
const DEVICE: u32 = 0x1000;
const BACK: u32 = 0x1100;
const RT: u32 = 0x1200;
const DS: u32 = 0x1210;
const VS: u32 = 0x1300;
const PS: u32 = 0x1310;
const DECL: u32 = 0x1400;
const VB: u32 = 0x1500;

/// `D3DFMT_A8R8G8B8`, `D3DFMT_D24S8`.
const A8R8G8B8: u32 = 21;
const D24S8: u32 = 75;
/// The depth texture format a pixel shader can sample (`MAKEFOURCC('I', 'N', 'T', 'Z')`).
const INTZ: u32 = u32::from_le_bytes(*b"INTZ");
/// Render states.
const ZENABLE: u32 = 7;
const ZWRITEENABLE: u32 = 14;
const ALPHATESTENABLE: u32 = 15;
const CULLMODE: u32 = 22;
const ZFUNC: u32 = 23;
const ALPHAREF: u32 = 24;
const ALPHAFUNC: u32 = 25;
const CLIPPING: u32 = 136;
const CLIPPLANEENABLE: u32 = 152;
const ADAPTIVETESS_Y: u32 = 181;
const STENCILENABLE: u32 = 52;
const STENCILFAIL: u32 = 53;
const STENCILZFAIL: u32 = 54;
const STENCILPASS: u32 = 55;
const STENCILFUNC: u32 = 56;
const STENCILREF: u32 = 57;
const STENCILMASK: u32 = 58;
const STENCILWRITEMASK: u32 = 59;
const SLOPESCALEDEPTHBIAS: u32 = 175;
const TWOSIDEDSTENCILMODE: u32 = 185;
const CCW_STENCILFAIL: u32 = 186;
const CCW_STENCILZFAIL: u32 = 187;
const CCW_STENCILPASS: u32 = 188;
const CCW_STENCILFUNC: u32 = 189;
const DEPTHBIAS: u32 = 195;

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

/// One GPU device at a time in this process: each test's device is made, used and dropped on
/// its turn. Microsoft's software adapter (WARP, all GitHub's runners have) stops the process
/// with an access violation inside `d3d10warp.dll` when devices of several test threads work at
/// once (7 of 30 parallel runs here; 0 of 20 with one test thread, 0 of 20 for each test alone).
/// The game makes one device, so only the tests need this.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A backend with a `size` by `size` A8R8G8B8 target (`samples` per pixel), a D24S8 depth
/// surface of the same, a pass-through vertex shader (`mov oPos, v0`: positions in clip space)
/// and a pixel shader writing `c0`.
struct Gpu {
    b: WgpuBackend,
    next: u32,
    /// Every command run, as `NV_D3D9_RECORD` writes them ([`Gpu::recording`]).
    rec: Option<Vec<u8>>,
    /// Held until the device is gone (fields drop in order: the backend first).
    _turn: std::sync::MutexGuard<'static, ()>,
}

impl Gpu {
    fn new(size: u32, samples: u32) -> Option<Gpu> {
        Gpu::with_cache(size, samples, None)
    }

    /// As [`Gpu::new`], keeping the driver's pipelines in `cache` ([`WgpuBackend::with_cache_dir`]).
    fn with_cache(size: u32, samples: u32, cache: Option<std::path::PathBuf>) -> Option<Gpu> {
        Gpu::make(size, samples, cache, &[])
    }

    /// As [`Gpu::new`], with the environment variables `env` set while the device is made (on
    /// this test's turn, so no other test's device sees them).
    fn with_env(size: u32, samples: u32, env: &[(&str, &str)]) -> Option<Gpu> {
        Gpu::make(size, samples, None, env)
    }

    /// As [`Gpu::new`], the device with its finishing and queue threads (one finishing thread at
    /// least, even on a machine with fewer than four logical processors: CI's macOS runner has
    /// three) or (`false`) without, as on such a machine.
    fn with_threads(size: u32, samples: u32, threads: bool) -> Option<Gpu> {
        Gpu::make_with(size, samples, None, &[], threads, threads)
    }

    fn make(
        size: u32,
        samples: u32,
        cache: Option<std::path::PathBuf>,
        env: &[(&str, &str)],
    ) -> Option<Gpu> {
        Gpu::make_with(size, samples, cache, env, true, false)
    }

    /// As [`Gpu::new`], keeping every command it runs ([`Gpu::rec`]) from its `CreateDevice` on.
    fn recording(size: u32, samples: u32) -> Option<Gpu> {
        Gpu::make_full(size, samples, None, &[], true, false, true)
    }

    fn make_with(
        size: u32,
        samples: u32,
        cache: Option<std::path::PathBuf>,
        env: &[(&str, &str)],
        threads: bool,
        force: bool,
    ) -> Option<Gpu> {
        Gpu::make_full(size, samples, cache, env, threads, force, false)
    }

    fn make_full(
        size: u32,
        samples: u32,
        cache: Option<std::path::PathBuf>,
        env: &[(&str, &str)],
        threads: bool,
        force: bool,
        record: bool,
    ) -> Option<Gpu> {
        // A failed test leaves the lock poisoned; the next one still takes its turn.
        let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        d3d9_wgpu::device::force_queue_threads_for_tests(force);
        for (name, value) in env {
            std::env::set_var(name, value);
        }
        d3d9_wgpu::device::queue_threads_for_tests(threads);
        let mut b = WgpuBackend::new(Output::Offscreen);
        if let Some(dir) = cache {
            b = b.with_cache_dir(dir);
        }
        let present = words(&[size, size, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
        let back = words(&[BACK]);
        let create = Cmd::CreateDevice {
            device: DEVICE,
            focus: 0,
            window: 0,
            behavior: 0x54,
            present: Data::Bytes(&present),
            back_buffers: Data::Bytes(&back),
            depth_stencil: 0,
        };
        let rec = record.then(|| {
            let mut r = d3d9_stream::wire::header().to_vec();
            d3d9_stream::wire::encode(&create, &mut r);
            r
        });
        b.execute(create);
        for (name, _) in env {
            std::env::remove_var(name);
        }
        d3d9_wgpu::device::queue_threads_for_tests(true);
        d3d9_wgpu::device::force_queue_threads_for_tests(false);
        if b.device().is_none() {
            eprintln!("no GPU adapter: the frame tests are skipped");
            return None;
        }
        let mut g = Gpu {
            b,
            next: 0,
            rec,
            _turn: turn,
        };
        for (surface, depth, format) in [(RT, false, A8R8G8B8), (DS, true, D24S8)] {
            g.run(Cmd::CreateSurface {
                device: DEVICE,
                surface,
                depth,
                width: size,
                height: size,
                format,
                multisample: if samples > 1 { samples } else { 0 },
                quality: 0,
                flag: 0,
            });
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
            g.run(Cmd::CreateShader {
                device: DEVICE,
                shader,
                pixel,
                tokens: Data::Bytes(&t),
            });
            g.run(Cmd::SetShader {
                device: DEVICE,
                pixel,
                shader,
            });
        }
        // FLOAT4 position in stream 0, then D3DDECL_END.
        let decl = [0u8, 0, 0, 0, 3, 0, 0, 0, 0xFF, 0, 0, 0, 17, 0, 0, 0];
        g.run(Cmd::CreateDeclaration {
            device: DEVICE,
            declaration: DECL,
            elements: Data::Bytes(&decl),
        });
        g.run(Cmd::SetVertexDeclaration {
            device: DEVICE,
            declaration: DECL,
        });
        g.run(Cmd::CreateBuffer {
            device: DEVICE,
            buffer: VB,
            index: false,
            length: 48 * 64,
            usage: 0,
            format: 0,
            pool: 1,
        });
        g.run(Cmd::SetRenderTarget {
            device: DEVICE,
            index: 0,
            surface: RT,
        });
        g.run(Cmd::SetDepthStencilSurface {
            device: DEVICE,
            surface: DS,
        });
        g.state(CULLMODE, 1);
        g.state(ZENABLE, 1);
        Some(g)
    }

    fn run(&mut self, cmd: Cmd<Data<'_>>) -> Reply {
        if let Some(r) = &mut self.rec {
            d3d9_stream::wire::encode(&cmd, r);
        }
        self.b.execute(cmd)
    }

    fn state(&mut self, state: u32, value: u32) {
        self.run(Cmd::SetRenderState {
            device: DEVICE,
            state,
            value,
        });
    }

    fn colour(&mut self, c: [f32; 4]) {
        let regs = floats(&c);
        self.run(Cmd::SetConstants {
            device: DEVICE,
            kind: 3,
            start: 0,
            count: 1,
            data: Data::Bytes(&regs),
        });
    }

    fn clear(&mut self, flags: u32, colour: u32, z: f32) {
        self.run(Cmd::Clear {
            device: DEVICE,
            count: 0,
            rects: None,
            flags,
            colour,
            z: z.to_bits(),
            stencil: 0,
            model: 0,
        });
    }

    fn viewport(&mut self, x: u32, y: u32, w: u32, h: u32, min_z: f32, max_z: f32) {
        self.run(Cmd::SetViewport {
            device: DEVICE,
            viewport: [x, y, w, h, min_z.to_bits(), max_z.to_bits()],
        });
    }

    /// A triangle covering the whole target at depth `z`, in its own part of the buffer. Its
    /// edges lie outside the target: Direct3D 9's pixel centres are on whole coordinates, so an
    /// edge at clip x = -1 runs through the left column's centres and leaves its multisample
    /// samples left of the centre uncovered.
    fn cover(&mut self, z: f32) {
        let tri = floats(&[-3.0, -3.0, z, 1.0, -3.0, 9.0, z, 1.0, 9.0, -3.0, z, 1.0]);
        let offset = 48 * (self.next % 64);
        self.next += 1;
        self.run(Cmd::UploadBuffer {
            buffer: VB,
            offset,
            flags: 0,
            data: Data::Bytes(&tri),
        });
        self.run(Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: VB,
            offset,
            stride: 16,
        });
        self.run(Cmd::DrawPrimitive {
            device: DEVICE,
            kind: 4,
            start: 0,
            count: 1,
        });
    }

    /// The target's pixels (resolved), B, G, R, A bytes.
    fn pixels(&mut self) -> Vec<[u8; 4]> {
        let d = self.b.device().expect("the device");
        let (_, _, px) = d.read_surface_pixels(RT).expect("the target's pixels");
        px.chunks_exact(4)
            .map(|p| [p[0], p[1], p[2], p[3]])
            .collect()
    }

    /// The red channel of each pixel, row by row.
    fn reds(&mut self) -> Vec<u8> {
        self.pixels().iter().map(|p| p[2]).collect()
    }

    fn count(&mut self, query: u32, draw: impl FnOnce(&mut Gpu)) -> u32 {
        self.run(Cmd::QueryIssue { query, flags: 2 });
        draw(self);
        self.run(Cmd::QueryIssue { query, flags: 1 });
        self.wait(query, 1)
    }

    /// `GetData(flags)` of `query` asked until it answers `S_OK` (as the game's loops ask), at
    /// most 10 seconds; its count.
    fn wait(&mut self, query: u32, flags: u32) -> u32 {
        let start = std::time::Instant::now();
        loop {
            match self.run(Cmd::QueryGetData { query, flags }) {
                Reply::Query(Some((0, n))) => return n,
                Reply::Query(Some((1, _))) if start.elapsed().as_secs() < 10 => {}
                other => panic!("query {query:#x}: {other:?} after {:?}", start.elapsed()),
            }
        }
    }

    /// A quad of exactly the viewport (clip -1 to 1) at depth `z`.
    fn quad(&mut self, z: f32) {
        let quad = floats(&[
            -1.0, -1.0, z, 1.0, -1.0, 1.0, z, 1.0, 1.0, -1.0, z, 1.0, //
            1.0, -1.0, z, 1.0, -1.0, 1.0, z, 1.0, 1.0, 1.0, z, 1.0,
        ]);
        let offset = 96 * (self.next % 32);
        self.next += 1;
        self.run(Cmd::UploadBuffer {
            buffer: VB,
            offset,
            flags: 0,
            data: Data::Bytes(&quad),
        });
        self.run(Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: VB,
            offset,
            stride: 16,
        });
        self.run(Cmd::DrawPrimitive {
            device: DEVICE,
            kind: 4,
            start: 0,
            count: 2,
        });
    }
}

/// Direct3D 9's viewport is a rectangle of whole pixels and drawing is cut at its pixels' edges:
/// geometry larger than the viewport covers every sample of its first and last rows and columns,
/// so their resolved pixels are the interior's, on a full-target viewport and on one inside the
/// target, at 1, 4 and 8 samples. (A viewport moved half a pixel for the pixel centres cut 15 of
/// the 64 samples of a 4 by 4 target at 4x on WARP.) Where the geometry's edge is the viewport's
/// own (a quad of clip -1 to 1), Direct3D 9's whole-number centres put the left and top edges
/// through the first column's and row's centres and the right and bottom edges on the last
/// ones' far sides: the last column and row are covered in full, the first column and row by
/// their samples right of and below the centre only (measured so on the real Direct3D 9 here,
/// DESIGN.md section 10).
#[test]
fn every_sample_of_the_viewports_edge_pixels_can_be_covered() {
    const SIZE: usize = 8;
    for samples in [1u32, 4, 8] {
        let Some(mut g) = Gpu::new(SIZE as u32, samples) else {
            return;
        };
        g.state(ZENABLE, 0);
        g.colour([1.0, 1.0, 1.0, 1.0]);
        for (x, y, w, h) in [(0, 0, SIZE, SIZE), (2, 1, 4, 5)] {
            let inside = |px: usize, py: usize| px >= x && px < x + w && py >= y && py < y + h;
            // Larger than the viewport: every pixel inside it in full, nothing outside.
            g.viewport(0, 0, SIZE as u32, SIZE as u32, 0.0, 1.0);
            g.clear(1, 0, 1.0);
            g.viewport(x as u32, y as u32, w as u32, h as u32, 0.0, 1.0);
            g.cover(0.5);
            let reds = g.reds();
            for py in 0..SIZE {
                for px in 0..SIZE {
                    let want = if inside(px, py) { 255 } else { 0 };
                    assert_eq!(
                        reds[py * SIZE + px],
                        want,
                        "{samples}x, viewport {:?}: pixel ({px}, {py}) in {reds:?}",
                        (x, y, w, h)
                    );
                }
            }
            // The viewport's own edges.
            g.viewport(0, 0, SIZE as u32, SIZE as u32, 0.0, 1.0);
            g.clear(1, 0, 1.0);
            g.viewport(x as u32, y as u32, w as u32, h as u32, 0.0, 1.0);
            g.quad(0.5);
            let reds = g.reds();
            let at = |px: usize, py: usize| reds[py * SIZE + px];
            for py in 0..SIZE {
                for px in 0..SIZE {
                    let r = at(px, py);
                    let what = format!(
                        "{samples}x, viewport {:?}: quad, pixel ({px}, {py}) in {reds:?}",
                        (x, y, w, h)
                    );
                    if !inside(px, py) {
                        assert_eq!(r, 0, "{what}");
                    } else if samples == 1 || (px > x && py > y) {
                        // One sample: the centre, on the left and top edges, is drawn (the
                        // top-left rule).
                        assert_eq!(r, 255, "{what}");
                    } else {
                        assert!(r > 0 && r < 255, "{what}: part of the samples");
                    }
                }
            }
        }
    }
}

/// A vertex shader has one module for every declaration of the same kinds: its position from a
/// FLOAT2 element (filled to (x, y, 0, 1), Direct3D 9's own expansion: the triangle at depth 0)
/// and from the FLOAT4 element (at depth 0.5) draw alike from one module, in two pipelines.
#[test]
fn a_vertex_shader_has_one_module_for_float_inputs_of_any_size() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.colour([1.0, 0.0, 0.0, 1.0]);
    // Behind the triangle at 0.5 and in front of the one at 0 (`LESSEQUAL` passes both
    // against 1.0; a depth of 0.25 then shows which reached the target).
    for (declaration, ty, z_drawn) in [(DECL, 3u8, 0.5f32), (DECL + 1, 1, 0.0)] {
        if declaration != DECL {
            // POSITION as FLOAT2 in stream 0, then D3DDECL_END.
            let decl = [0u8, 0, 0, 0, ty, 0, 0, 0, 0xFF, 0, 0, 0, 17, 0, 0, 0];
            g.run(Cmd::CreateDeclaration {
                device: DEVICE,
                declaration,
                elements: Data::Bytes(&decl),
            });
        }
        g.run(Cmd::SetVertexDeclaration {
            device: DEVICE,
            declaration,
        });
        // A first pass at depth 0.25: the triangle drawn after it covers it only where it is
        // nearer (`LESSEQUAL`).
        g.clear(3, 0, 0.25);
        g.cover(0.5);
        let want = if z_drawn <= 0.25 { 255 } else { 0 };
        assert_eq!(g.reds(), vec![want; 16], "drawn at depth {z_drawn}");
    }
    let d = g.b.device().expect("the device");
    assert_eq!(
        d.module_counts().0,
        1,
        "one vertex module for FLOAT4 and FLOAT2"
    );
    assert_eq!(
        d.pipelines_made_on_draws(),
        2,
        "a pipeline for each vertex format"
    );
}

/// Draws one triangle over the whole target from `vertices` (`stride` bytes each) through a new
/// declaration `decl` of `elements` (stream 0: offset, `D3DDECLTYPE`, usage, usage index).
fn draw_with(g: &mut Gpu, decl: u32, elements: &[(u16, u8, u8, u8)], vertices: &[u8], stride: u32) {
    let mut bytes = Vec::new();
    for &(offset, ty, usage, index) in elements {
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&[ty, 0, usage, index]);
    }
    bytes.extend_from_slice(&[0xFF, 0, 0, 0, 17, 0, 0, 0]);
    g.run(Cmd::CreateDeclaration {
        device: DEVICE,
        declaration: decl,
        elements: Data::Bytes(&bytes),
    });
    g.run(Cmd::SetVertexDeclaration {
        device: DEVICE,
        declaration: decl,
    });
    let offset = 256 * (g.next % 12);
    g.next += 1;
    g.run(Cmd::UploadBuffer {
        buffer: VB,
        offset,
        flags: 0,
        data: Data::Bytes(vertices),
    });
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: VB,
        offset,
        stride,
    });
    g.run(Cmd::DrawPrimitive {
        device: DEVICE,
        kind: 4,
        start: 0,
        count: 1,
    });
}

/// The corners of a triangle over the whole target (clip x and y).
const TRIANGLE: [[f32; 2]; 3] = [[-3.0, -3.0], [-3.0, 9.0], [9.0, -3.0]];

/// A vs_2_0 passing POSITION and TEXCOORD0 on (dcl_position v0; dcl_texcoord0 v1; mov oPos, v0;
/// mov oT0, v1) and a ps_2_0 drawing its TEXCOORD0 (dcl t0; mov oC0, t0), made and set.
fn texcoord_pair(g: &mut Gpu, vs: u32, ps: u32) {
    let mut v = vec![0xFFFE_0200];
    v.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    v.extend(ins(31, &[0x8000_0005, 0x900F_0001]));
    v.extend(ins(1, &[0xC00F_0000, 0x90E4_0000]));
    v.extend(ins(1, &[0xE00F_0000, 0x90E4_0001]));
    v.push(END);
    let mut p = vec![0xFFFF_0200];
    p.extend(ins(31, &[0x8000_0000, 0xB00F_0000]));
    p.extend(ins(1, &[0x800F_0800, 0xB0E4_0000]));
    p.push(END);
    for (shader, pixel, tokens) in [(vs, false, v), (ps, true, p)] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
}

/// FLOAT3 and FLOAT1 are filled to (x, y, z, 1) and (x, 0, 0, 1) as Direct3D 9 expands them, by
/// the hardware, with one vertex module for them and FLOAT4. Each is followed in memory by a
/// value a wider read would take: a FLOAT3 TEXCOORD0 of (0.2, 0.4, 0.6) followed by 0.0 draws
/// alpha 1, where a w read from memory would give 0; a FLOAT1 TEXCOORD0 of 0.4 followed by 0.6
/// draws (0.4, 0, 0, 1), where a second component read from memory would give green. (The FLOAT3
/// positions are followed by 0.0 too, but a triangle whose w is 0 everywhere still covers the
/// target on this machine's card, so the colour carries the check.)
#[test]
fn float3_and_float1_elements_are_filled_as_direct3d_9_expands_them() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    texcoord_pair(&mut g, VS + 5, PS + 14);
    // POSITION FLOAT4 (x, y, 0.5, 1) and TEXCOORD0 FLOAT4 (0.2, 0.4, 0.6, 1): 32 bytes each.
    let four: Vec<u8> = floats(
        &TRIANGLE
            .iter()
            .flat_map(|p| [p[0], p[1], 0.5, 1.0, 0.2, 0.4, 0.6, 1.0])
            .collect::<Vec<f32>>(),
    );
    g.clear(3, 0, 1.0);
    draw_with(&mut g, DECL + 10, &[(0, 3, 0, 0), (16, 3, 5, 0)], &four, 32);
    // (0.2, 0.4, 0.6, 1) as B, G, R, A bytes (whole numbers of 1/255: no rounding tie).
    assert_eq!(g.pixels(), vec![[153, 102, 51, 255]; 16], "FLOAT4");
    // POSITION FLOAT3 (x, y, 0) and TEXCOORD0 FLOAT3 (0.2, 0.4, 0.6), each with 0.0 after it.
    let three: Vec<u8> = floats(
        &TRIANGLE
            .iter()
            .flat_map(|p| [p[0], p[1], 0.0, 0.0, 0.2, 0.4, 0.6, 0.0])
            .collect::<Vec<f32>>(),
    );
    g.clear(3, 0, 1.0);
    draw_with(
        &mut g,
        DECL + 11,
        &[(0, 2, 0, 0), (16, 2, 5, 0)],
        &three,
        32,
    );
    assert_eq!(
        g.pixels(),
        vec![[153, 102, 51, 255]; 16],
        "FLOAT3 read as (0.2, 0.4, 0.6, 1)"
    );
    // POSITION FLOAT3 (x, y, 0) and 0.0 after it, TEXCOORD0 FLOAT1 0.4 and 0.6 after it.
    let one: Vec<u8> = floats(
        &TRIANGLE
            .iter()
            .flat_map(|p| [p[0], p[1], 0.0, 0.0, 0.4, 0.6])
            .collect::<Vec<f32>>(),
    );
    g.clear(3, 0, 1.0);
    draw_with(&mut g, DECL + 13, &[(0, 2, 0, 0), (16, 0, 5, 0)], &one, 24);
    assert_eq!(
        g.pixels(),
        vec![[0, 0, 102, 255]; 16],
        "FLOAT1 read as (0.4, 0, 0, 1)"
    );
    let d = g.b.device().expect("the device");
    // With the harness's vertex shader's module, made ahead as this one's.
    d.wait_for_modules();
    assert_eq!(
        d.module_counts().0,
        1 + 1,
        "one vertex module for all three"
    );
}

/// A `D3DCOLOR` element reaches the shader with its bytes in Direct3D 9's order (memory B, G, R,
/// A: the shader's x is the third byte), and a `SHORT4` element as whole numbers, not normalised:
/// a position of (-3, -3, 0, 1) and the like covers the target.
#[test]
fn d3dcolor_and_short4_elements_read_as_direct3d_9_reads_them() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    // vs_2_0: dcl_position v0; dcl_color v1; mov oPos, v0; mov oD0, v1.
    let mut v = vec![0xFFFE_0200];
    v.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    v.extend(ins(31, &[0x8000_000A, 0x900F_0001]));
    v.extend(ins(1, &[0xC00F_0000, 0x90E4_0000]));
    v.extend(ins(1, &[0xD00F_0000, 0x90E4_0001]));
    v.push(END);
    // ps_2_0: dcl v0; mov oC0, v0.
    let mut p = vec![0xFFFF_0200];
    p.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    p.extend(ins(1, &[0x800F_0800, 0x90E4_0000]));
    p.push(END);
    for (shader, pixel, tokens) in [(VS + 6, false, v), (PS + 15, true, p)] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    // POSITION SHORT4 (x, y, 0, 1), COLOR0 D3DCOLOR bytes 0x10, 0x80, 0xF0, 0xFF: 12 bytes.
    let mut vertices = Vec::new();
    for p in TRIANGLE {
        for c in [p[0] as i16, p[1] as i16, 0, 1] {
            vertices.extend_from_slice(&c.to_le_bytes());
        }
        vertices.extend_from_slice(&[0x10, 0x80, 0xF0, 0xFF]);
    }
    g.clear(3, 0, 1.0);
    draw_with(
        &mut g,
        DECL + 12,
        &[(0, 7, 0, 0), (8, 4, 10, 0)],
        &vertices,
        12,
    );
    // The target's B, G, R, A bytes are the colour's own: R 0xF0, B 0x10.
    assert_eq!(g.pixels(), vec![[0x10, 0x80, 0xF0, 0xFF]; 16]);
}

/// The coarser key still separates what a module depends on: one vertex shader drawn with a
/// float position, a signed one (`SHORT4`) and a declaration without its TEXCOORD0 (read as
/// (0, 0, 0, 1)) takes three modules, and each draw is its own declaration's.
#[test]
fn declarations_of_other_kinds_or_a_missing_element_get_their_own_modules() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    texcoord_pair(&mut g, VS + 7, PS + 16);
    // Float: POSITION FLOAT4, TEXCOORD0 FLOAT4 (0.2, 0.4, 0.6, 1).
    let float: Vec<u8> = floats(
        &TRIANGLE
            .iter()
            .flat_map(|p| [p[0], p[1], 0.5, 1.0, 0.2, 0.4, 0.6, 1.0])
            .collect::<Vec<f32>>(),
    );
    g.clear(3, 0, 1.0);
    draw_with(
        &mut g,
        DECL + 13,
        &[(0, 3, 0, 0), (16, 3, 5, 0)],
        &float,
        32,
    );
    assert_eq!(g.pixels(), vec![[153, 102, 51, 255]; 16], "float position");
    // Signed: POSITION SHORT4 (x, y, 0, 1), the same TEXCOORD0 after it.
    let mut signed = Vec::new();
    for p in TRIANGLE {
        for c in [p[0] as i16, p[1] as i16, 0, 1] {
            signed.extend_from_slice(&c.to_le_bytes());
        }
        signed.extend(floats(&[0.2, 0.4, 0.6, 1.0]));
    }
    g.clear(3, 0, 1.0);
    draw_with(
        &mut g,
        DECL + 14,
        &[(0, 7, 0, 0), (8, 3, 5, 0)],
        &signed,
        24,
    );
    assert_eq!(g.pixels(), vec![[153, 102, 51, 255]; 16], "signed position");
    // No TEXCOORD0: the shader reads (0, 0, 0, 1), opaque black.
    let bare: Vec<u8> = floats(
        &TRIANGLE
            .iter()
            .flat_map(|p| [p[0], p[1], 0.5, 1.0])
            .collect::<Vec<f32>>(),
    );
    g.clear(3, 0x8080_8080, 1.0);
    draw_with(&mut g, DECL + 15, &[(0, 3, 0, 0)], &bare, 16);
    assert_eq!(g.pixels(), vec![[0, 0, 0, 255]; 16], "TEXCOORD0 missing");
    let d = g.b.device().expect("the device");
    // With the harness's vertex shader's module; the float one was made ahead.
    d.wait_for_modules();
    assert_eq!(
        d.module_counts().0,
        1 + 3,
        "a vertex module for float, for signed and for the missing input"
    );
}

/// Each stage's samplers in a bind group of its own (the pixel shader's 1, the vertex shader's
/// 2): a vertex shader sampling vertex texture 0 (`texldl`, a green texel) feeds a pixel shader
/// that samples nothing, and the target shows the green; then a pixel shader that also samples
/// its texture 0 (a red texel) adds the two: yellow.
#[test]
fn each_stage_samples_from_its_own_group() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    stage_samplers(&mut g);
    g.run(Cmd::SetShader {
        device: DEVICE,
        pixel: false,
        shader: VS + 2,
    });
    for (ps, want) in [(PS + 5, [0, 255, 0, 255]), (PS + 6, [0, 255, 255, 255])] {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel: true,
            shader: ps,
        });
        g.clear(3, 0, 1.0);
        g.cover(0.5);
        assert_eq!(g.pixels(), vec![want; 16], "pixel shader {ps:#x}");
    }
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
}

/// With pipeline libraries, a textured shader's library made ahead is the one its draws link:
/// the vertex shader sampling a vertex texture and the pixel shader sampling its own make no
/// library on their draws (their textures filterable, as the layout made ahead assumes). A
/// draw whose pixel shader samples a depth texture (not filterable) makes exactly one pixel
/// shader library.
#[test]
fn a_textured_shaders_library_made_ahead_is_the_one_its_draws_link() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    stage_samplers(&mut g);
    let d = g.b.device().expect("the device");
    let Some(_) = d.pipeline_library_report() else {
        eprintln!(
            "{}: no pipeline libraries, so the case is not reached here",
            d.adapter_name()
        );
        return;
    };
    d.wait_for_modules();
    let made = library_counts(d).expect("the counts");
    g.run(Cmd::SetShader {
        device: DEVICE,
        pixel: false,
        shader: VS + 2,
    });
    for ps in [PS + 5, PS + 6] {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel: true,
            shader: ps,
        });
        g.clear(3, 0, 1.0);
        g.cover(0.5);
    }
    let d = g.b.device().expect("the device");
    assert_eq!(
        library_counts(d),
        Some(made),
        "the draws linked the ones made ahead"
    );
    // An INTZ depth texture (1 by 1) for the pixel shader's sampler 0: not filterable.
    const DEPTH_TEX: u32 = 0x1604;
    let levels = words(&[DEPTH_TEX + 1]);
    g.run(Cmd::CreateTexture {
        device: DEVICE,
        texture: DEPTH_TEX,
        levels: Data::Bytes(&levels),
        width: 1,
        height: 1,
        usage: 2,
        format: INTZ,
        pool: 1,
    });
    g.run(Cmd::SetTexture {
        device: DEVICE,
        sampler: 0,
        texture: DEPTH_TEX,
    });
    g.clear(3, 0, 1.0);
    g.cover(0.5);
    let d = g.b.device().expect("the device");
    assert_eq!(
        library_counts(d),
        Some((made.0, made.1 + 1)),
        "one more pixel shader library, for the other layout"
    );
    assert_eq!(d.wgpu_errors(), 0);
}

/// Textures and shaders for each stage's own sampler group: a 1 by 1 green texture for vertex
/// sampler 0 and a red one for pixel sampler 0; a vs_3_0 (`VS + 2`) that samples its texture
/// (`texldl`) into TEXCOORD0; a ps_3_0 (`PS + 5`) that draws TEXCOORD0 and one (`PS + 6`) that
/// adds its own texture's texel to it.
fn stage_samplers(g: &mut Gpu) {
    const VS_TEX: u32 = 0x1600;
    const PS_TEX: u32 = 0x1602;
    for (texture, bgra) in [(VS_TEX, [0u8, 255, 0, 255]), (PS_TEX, [0, 0, 255, 255])] {
        let level = texture + 1;
        let levels = words(&[level]);
        g.run(Cmd::CreateTexture {
            device: DEVICE,
            texture,
            levels: Data::Bytes(&levels),
            width: 1,
            height: 1,
            usage: 0,
            format: A8R8G8B8,
            pool: 1,
        });
        g.run(Cmd::UploadSurface {
            surface: level,
            row: 4,
            rows: 1,
            data: Data::Bytes(&bgra),
        });
    }
    // D3DVERTEXTEXTURESAMPLER0 and pixel sampler 0.
    for (sampler, texture) in [(257, VS_TEX), (0, PS_TEX)] {
        g.run(Cmd::SetTexture {
            device: DEVICE,
            sampler,
            texture,
        });
    }
    // vs_3_0: dcl_position v0; dcl_position o0; dcl_texcoord0 o1; dcl_2d s0; mov o0, v0;
    // texldl o1, c0, s0 (c0: the texel's centre, level 0).
    let mut vs = vec![0xFFFE_0300];
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(31, &[0x8000_0000, 0xE00F_0000]));
    vs.extend(ins(31, &[0x8000_0005, 0xE00F_0001]));
    vs.extend(ins(31, &[0x9000_0000, 0xA00F_0800]));
    vs.extend(ins(1, &[0xE00F_0000, 0x90E4_0000]));
    vs.extend(ins(95, &[0xE00F_0001, 0xA0E4_0000, 0xA0E4_0800]));
    vs.push(END);
    let regs = floats(&[0.5, 0.5, 0.0, 0.0]);
    g.run(Cmd::SetConstants {
        device: DEVICE,
        kind: 0,
        start: 0,
        count: 1,
        data: Data::Bytes(&regs),
    });
    // ps_3_0: dcl_texcoord0 v0; mov oC0, v0.
    let mut ps_plain = vec![0xFFFF_0300];
    ps_plain.extend(ins(31, &[0x8000_0005, 0x900F_0000]));
    ps_plain.extend(ins(1, &[0x800F_0800, 0x90E4_0000]));
    ps_plain.push(END);
    // ps_3_0: dcl_texcoord0 v0; dcl_2d s0; texld r0, v0, s0; add oC0, r0, v0.
    let mut ps_sampling = vec![0xFFFF_0300];
    ps_sampling.extend(ins(31, &[0x8000_0005, 0x900F_0000]));
    ps_sampling.extend(ins(31, &[0x9000_0000, 0xA00F_0800]));
    ps_sampling.extend(ins(66, &[0x800F_0000, 0x90E4_0000, 0xA0E4_0800]));
    ps_sampling.extend(ins(2, &[0x800F_0800, 0x80E4_0000, 0x90E4_0000]));
    ps_sampling.push(END);
    for (shader, pixel, tokens) in [
        (VS + 2, false, vs),
        (PS + 5, true, ps_plain),
        (PS + 6, true, ps_sampling),
    ] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
    }
}

/// Fixed varying locations: one vertex shader writing TEXCOORD0 (red) and TEXCOORD1 (green)
/// serves a pixel shader reading TEXCOORD0 and one reading TEXCOORD1 with one module, each pair
/// drawing its own colour; a pixel shader reading TEXCOORD2, which the vertex shader does not
/// write, reads 0 from a vertex module of its own pair. Before, each pixel shader took a vertex
/// module of its own.
#[test]
fn a_vertex_shader_has_one_module_for_the_pixel_shaders_reading_what_it_writes() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    // vs_2_0: dcl_position v0; mov oPos, v0; mov oT0, c1; mov oT1, c2.
    let mut vs = vec![0xFFFE_0200];
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(1, &[0xC00F_0000, 0x90E4_0000]));
    vs.extend(ins(1, &[0xE00F_0000, 0xA0E4_0001]));
    vs.extend(ins(1, &[0xE00F_0001, 0xA0E4_0002]));
    vs.push(END);
    let t = words(&vs);
    g.run(Cmd::CreateShader {
        device: DEVICE,
        shader: VS + 1,
        pixel: false,
        tokens: Data::Bytes(&t),
    });
    g.run(Cmd::SetShader {
        device: DEVICE,
        pixel: false,
        shader: VS + 1,
    });
    let regs = floats(&[1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
    g.run(Cmd::SetConstants {
        device: DEVICE,
        kind: 0,
        start: 1,
        count: 2,
        data: Data::Bytes(&regs),
    });
    // ps_2_0: dcl tN; mov oC0, tN; for N = 0, 1, 2.
    for (n, want) in [
        (0u32, [0, 0, 255, 255]),
        (1, [0, 255, 0, 255]),
        (2, [0, 0, 0, 0]),
    ] {
        let mut ps = vec![0xFFFF_0200];
        ps.extend(ins(31, &[0x8000_0000, 0xB00F_0000 | n]));
        ps.extend(ins(1, &[0x800F_0800, 0xB0E4_0000 | n]));
        ps.push(END);
        let t = words(&ps);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader: PS + 1 + n,
            pixel: true,
            tokens: Data::Bytes(&t),
        });
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel: true,
            shader: PS + 1 + n,
        });
        g.clear(3, 0x8080_8080, 1.0);
        g.cover(0.5);
        assert_eq!(g.pixels(), vec![want; 16], "reading TEXCOORD{n}");
    }
    let d = g.b.device().expect("the device");
    // With the modules the harness's own two shaders got when they were made (each shader's
    // modules are made ahead: [`a_shaders_modules_are_made_when_the_game_creates_it`]).
    d.wait_for_modules();
    assert_eq!(
        d.module_counts(),
        (1 + 2, 1 + 3),
        "one vertex module for TEXCOORD0 and 1, one more for TEXCOORD2"
    );
}

/// A pixel shader reading a varying without a fixed location (NORMAL0, which no package's
/// shader passes, but a mod's might) keeps the pair's own locations: a vs_3_0 writing TEXCOORD0
/// and NORMAL0 and a ps_3_0 drawing the NORMAL0 it reads, green; each stage gets a module (none
/// refused) and the draw is not skipped.
#[test]
fn a_varying_without_a_fixed_location_keeps_the_pairs_own_link() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    // vs_3_0: dcl_position v0; dcl_position o0; dcl_normal o1; dcl_texcoord0 o2;
    // mov o0, v0; mov o1, c1; mov o2, c2.
    let mut vs = vec![0xFFFE_0300];
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(31, &[0x8000_0000, 0xE00F_0000]));
    vs.extend(ins(31, &[0x8000_0003, 0xE00F_0001]));
    vs.extend(ins(31, &[0x8000_0005, 0xE00F_0002]));
    vs.extend(ins(1, &[0xE00F_0000, 0x90E4_0000]));
    vs.extend(ins(1, &[0xE00F_0001, 0xA0E4_0001]));
    vs.extend(ins(1, &[0xE00F_0002, 0xA0E4_0002]));
    vs.push(END);
    // ps_3_0: dcl_normal v1; mov oC0, v1.
    let mut ps = vec![0xFFFF_0300];
    ps.extend(ins(31, &[0x8000_0003, 0x900F_0001]));
    ps.extend(ins(1, &[0x800F_0800, 0x90E4_0001]));
    ps.push(END);
    for (shader, pixel, tokens) in [(VS + 3, false, vs), (PS + 7, true, ps)] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    // NORMAL0 green, TEXCOORD0 red.
    let regs = floats(&[0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0]);
    g.run(Cmd::SetConstants {
        device: DEVICE,
        kind: 0,
        start: 1,
        count: 2,
        data: Data::Bytes(&regs),
    });
    g.clear(3, 0, 1.0);
    g.cover(0.5);
    assert_eq!(g.pixels(), vec![[0, 255, 0, 255]; 16], "NORMAL0 drawn");
    let d = g.b.device().expect("the device");
    // With the modules made ahead: the harness's two shaders', and this vertex shader's own
    // (TEXCOORD0 at its fixed location; the pixel shader reading NORMAL0 has none).
    d.wait_for_modules();
    assert_eq!(
        d.module_counts(),
        (1 + 1 + 1, 1 + 1),
        "a module for each stage of the pair, none refused"
    );
    assert_eq!(d.wgpu_errors(), 0);
}

/// A varying the vertex shader does not write reads as on the real Direct3D 9 (measured on
/// NVIDIA's driver, engine-platform's `what_an_unwritten_varying_reads_on_the_real_direct3d_9`):
/// COLOR0 white after vs_1_1, vs_2_0 and vs_3_0; COLOR1 (0, 0, 0, 1) after vs_1_1 and vs_2_0, 0
/// after vs_3_0; TEXCOORD0 0. A colour written only in x, y and z (`mov oD0.xyz, c0`) keeps the
/// rest of that value: alpha 1, and 0 for COLOR1 after vs_3_0. The pair's own link (a pixel
/// shader also reading NORMAL0, which has no fixed location) writes it the same way.
#[test]
fn an_unwritten_varying_reads_as_on_the_real_direct3d_9() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    // vs_3_0: dcl_position v0; dcl_position o0; mov o0, v0 (the harness's VS is the vs_2_0 one).
    let mut vs3 = vec![0xFFFE_0300];
    vs3.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs3.extend(ins(31, &[0x8000_0000, 0xE00F_0000]));
    vs3.extend(ins(1, &[0xE00F_0000, 0x90E4_0000]));
    vs3.push(END);
    // vs_1_1: dcl_position v0; mov oPos, v0 (shader model 1 tokens carry no length).
    let vs1 = vec![
        0xFFFE_0101,
        0x0000_001F,
        0x8000_0000,
        0x900F_0000,
        0x0000_0001,
        0xC00F_0000,
        0x90E4_0000,
        END,
    ];
    // Each also writing x, y and z of a colour output from c0 (mov oDn.xyz, c0).
    let mut vs1d0 = vs1[..vs1.len() - 1].to_vec();
    vs1d0.extend([0x0000_0001, 0xD007_0000, 0xA0E4_0000, END]);
    let mut vs2d1 = vec![0xFFFE_0200];
    vs2d1.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs2d1.extend(ins(1, &[0xC00F_0000, 0x90E4_0000]));
    vs2d1.extend(ins(1, &[0xD007_0001, 0xA0E4_0000]));
    vs2d1.push(END);
    // vs_3_0: dcl_position v0; dcl_position o0; dcl_colorN o1.xyz; mov o0, v0; mov o1.xyz, c0.
    let vs3c = |usage: u32| {
        let mut v = vec![0xFFFE_0300];
        v.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
        v.extend(ins(31, &[0x8000_0000, 0xE00F_0000]));
        v.extend(ins(31, &[usage, 0xE007_0001]));
        v.extend(ins(1, &[0xE00F_0000, 0x90E4_0000]));
        v.extend(ins(1, &[0xE007_0001, 0xA0E4_0000]));
        v.push(END);
        v
    };
    // vs_3_0 writing NORMAL0 only: dcl_position v0; dcl_position o0; dcl_normal o1; mov o0, v0;
    // mov o1, c0.
    let mut vs3n = vec![0xFFFE_0300];
    vs3n.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs3n.extend(ins(31, &[0x8000_0000, 0xE00F_0000]));
    vs3n.extend(ins(31, &[0x8000_0003, 0xE00F_0001]));
    vs3n.extend(ins(1, &[0xE00F_0000, 0x90E4_0000]));
    vs3n.extend(ins(1, &[0xE00F_0001, 0xA0E4_0000]));
    vs3n.push(END);
    for (shader, tokens) in [
        (VS + 4, vs3),
        (VS + 5, vs1),
        (VS + 6, vs1d0),
        (VS + 7, vs2d1),
        (VS + 8, vs3c(0x8000_000A)),
        (VS + 9, vs3c(0x8001_000A)),
        (VS + 10, vs3n),
    ] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel: false,
            tokens: Data::Bytes(&t),
        });
    }
    // c0 of the vertex shaders: (0.2, 0.4, 0.6, 0.8), each a whole number of 1/255.
    let regs = floats(&[0.2, 0.4, 0.6, 0.8]);
    g.run(Cmd::SetConstants {
        device: DEVICE,
        kind: 0,
        start: 0,
        count: 1,
        data: Data::Bytes(&regs),
    });
    // ps_2_0: dcl REG; mov oC0, REG. ps_3_0: dcl_USAGE vN; mov oC0, vN.
    let ps2 = |dcl: u32, src: u32| {
        let mut p = vec![0xFFFF_0200];
        p.extend(ins(31, &[0x8000_0000, dcl]));
        p.extend(ins(1, &[0x800F_0800, src]));
        p.push(END);
        p
    };
    let ps3 = |usage: u32, n: u32| {
        let mut p = vec![0xFFFF_0300];
        p.extend(ins(31, &[usage, 0x900F_0000 | n]));
        p.extend(ins(1, &[0x800F_0800, 0x90E4_0000 | n]));
        p.push(END);
        p
    };
    // ps_3_0 reading NORMAL0 and COLOR0 and drawing COLOR0: dcl_normal v1; dcl_color0 v0;
    // mov oC0, v0.
    let mut psn = vec![0xFFFF_0300];
    psn.extend(ins(31, &[0x8000_0003, 0x900F_0001]));
    psn.extend(ins(31, &[0x8000_000A, 0x900F_0000]));
    psn.extend(ins(1, &[0x800F_0800, 0x90E4_0000]));
    psn.push(END);
    let white = [255, 255, 255, 255];
    let black = [0, 0, 0, 255];
    let zero = [0, 0, 0, 0];
    // x, y and z from c0 (B 0.6, G 0.4, R 0.2), the alpha as said.
    let rgb_1 = [153, 102, 51, 255];
    let rgb_0 = [153, 102, 51, 0];
    let cases = [
        (
            "vs_1_1 COLOR0",
            VS + 5,
            ps2(0x900F_0000, 0x90E4_0000),
            white,
        ),
        (
            "vs_1_1 COLOR1",
            VS + 5,
            ps2(0x900F_0001, 0x90E4_0001),
            black,
        ),
        ("vs_2_0 COLOR0", VS, ps2(0x900F_0000, 0x90E4_0000), white),
        ("vs_2_0 COLOR1", VS, ps2(0x900F_0001, 0x90E4_0001), black),
        ("vs_2_0 TEXCOORD0", VS, ps2(0xB00F_0000, 0xB0E4_0000), zero),
        ("vs_3_0 COLOR0", VS + 4, ps3(0x8000_000A, 0), white),
        ("vs_3_0 COLOR1", VS + 4, ps3(0x8001_000A, 1), zero),
        ("vs_3_0 TEXCOORD0", VS + 4, ps3(0x8000_0005, 2), zero),
        (
            "vs_1_1 COLOR0.xyz",
            VS + 6,
            ps2(0x900F_0000, 0x90E4_0000),
            rgb_1,
        ),
        (
            "vs_2_0 COLOR1.xyz",
            VS + 7,
            ps2(0x900F_0001, 0x90E4_0001),
            rgb_1,
        ),
        ("vs_3_0 COLOR0.xyz", VS + 8, ps3(0x8000_000A, 0), rgb_1),
        ("vs_3_0 COLOR1.xyz", VS + 9, ps3(0x8001_000A, 1), rgb_0),
        ("vs_3_0 NORMAL0, COLOR0 read", VS + 10, psn, white),
    ];
    for (k, (name, vs, ps, want)) in cases.into_iter().enumerate() {
        let t = words(&ps);
        let shader = PS + 8 + k as u32;
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel: true,
            tokens: Data::Bytes(&t),
        });
        for (pixel, shader) in [(false, vs), (true, shader)] {
            g.run(Cmd::SetShader {
                device: DEVICE,
                pixel,
                shader,
            });
        }
        g.clear(3, 0x8080_8080, 1.0);
        g.cover(0.5);
        assert_eq!(g.pixels(), vec![want; 16], "{name}");
    }
}

/// The list names each plane state a pipeline's draws had: with the clip distances one pipeline
/// serves a draw with no plane on and one with a plane on, but an adapter without them
/// (Direct3D 12) needs a pipeline for each, so a list written here holds both lines (clip 0 and
/// a plane's code), as one written there does. A pipeline made ahead from a list that names only
/// its clip-0 line (the shipped list's, from before both were listed) still adds the plane
/// state's line when a draw has a plane on. A pipeline drawn again in a state it was listed with
/// offers the list nothing. (The list file holds the shipped entries too: a run that draws with
/// no plane only gives what the other runs add to.)
#[test]
fn the_list_names_each_plane_state_a_pipeline_drew_with() {
    let folder = |name: &str| {
        let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "pipeline-list-planes-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    };
    // A run with its list in `dir`: with `ahead` a `Present` first, so the listed pipelines are
    // made before the draws; then a draw in each plane state of `planes`, in order. The list the
    // device left, the pipelines made on draws, the entries the draws offered the list, whether
    // the adapter has clip distances, and its name.
    type Run = (Vec<String>, u64, u64, bool, String);
    let run = |dir: &std::path::Path, ahead: bool, planes: &[bool]| -> Option<Run> {
        let (made, offers, distances, adapter) = {
            let mut g = Gpu::with_cache(4, 1, Some(dir.to_path_buf()))?;
            if ahead {
                g.run(Cmd::Present {
                    device: DEVICE,
                    source: None,
                    dest: None,
                    window: 0,
                    dirty: false,
                    model: 0,
                    frame: 1,
                });
            }
            let plane = [1.0f32, 0.0, 0.0, -0.25].map(f32::to_bits);
            g.run(Cmd::SetClipPlane {
                device: DEVICE,
                index: 0,
                plane,
            });
            g.colour([1.0, 1.0, 1.0, 1.0]);
            g.clear(3, 0, 1.0);
            for &on in planes {
                g.state(CLIPPLANEENABLE, u32::from(on));
                g.cover(0.5);
            }
            let d = g.b.device().expect("the device");
            (
                d.pipelines_made_on_draws(),
                d.list_offers(),
                d.has_clip_distances(),
                d.adapter_name().to_string(),
            )
            // The device goes: its list is written.
        };
        let list = std::fs::read_to_string(dir.join("pipelines.txt")).expect("the list written");
        let list = list.lines().map(str::to_string).collect();
        Some((list, made, offers, distances, adapter))
    };
    let none_dir = folder("none");
    let Some((none, made, offers, distances, adapter)) = run(&none_dir, false, &[false]) else {
        return;
    };
    assert_eq!(
        (made, offers),
        (1, 1),
        "one pipeline, offered to the list once"
    );
    // What a run adds to the list of the run without a plane: one line, with a plane's code,
    // whose clip-0 twin that run listed (the same pipeline with the clip distances, else the
    // one without a plane).
    let plane_line = |list: &[String], what: &str| {
        let added: Vec<&String> = list.iter().filter(|l| !none.contains(l)).collect();
        assert_eq!(
            added.len(),
            1,
            "{what}: one more line, a plane on: {added:?}"
        );
        let mut fields: Vec<&str> = added[0].split_ascii_whitespace().collect();
        assert_ne!(fields.last(), Some(&"0"), "{what}: {}", added[0]);
        *fields.last_mut().unwrap() = "0";
        let zero = fields.join(" ");
        assert!(
            none.contains(&zero),
            "{what}: the line with no plane on: {zero}"
        );
    };
    // No plane, a plane, and each again: each state offered to the list once.
    let both_dir = folder("both");
    let (both, made, offers, ..) = run(&both_dir, false, &[false, true, false, true]).unwrap();
    plane_line(&both, "both");
    assert_eq!(offers, 2, "each plane state offered once");
    if distances {
        assert_eq!(made, 1, "{adapter}: one pipeline for both plane states");
    } else {
        assert_eq!(made, 2, "{adapter}: a pipeline for each plane state");
        eprintln!(
            "{adapter}: no clip distances, so a plane on is a pipeline of its own, listed as any \
             new one: one pipeline listed in two states is not reached here"
        );
    }
    // The first run's list again, drawing only with a plane on: its clip-0 line makes the
    // pipeline ahead, and the draw still adds the plane state's line. With the clip distances
    // that pipeline serves the draw; without, the plane state is a pipeline of its own.
    let (ahead, made, offers, ..) = run(&none_dir, true, &[true]).unwrap();
    plane_line(&ahead, "made ahead");
    assert_eq!(offers, 1, "the plane state offered");
    let want = if distances { 0 } else { 1 };
    assert_eq!(made, want, "{adapter}: pipelines made on the draw");
    let _ = std::fs::remove_dir_all(&none_dir);
    let _ = std::fs::remove_dir_all(&both_dir);
}

/// Each shader's modules are made on the module threads when the game creates it: once they
/// are, the first draw of the pair converts and makes no module (its pipeline only), and draws
/// as before.
#[test]
fn a_shaders_modules_are_made_when_the_game_creates_it() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    let d = g.b.device().expect("the device");
    d.wait_for_modules();
    assert_eq!(
        d.module_counts(),
        (1, 1),
        "the harness's two shaders' modules"
    );
    assert_eq!(d.modules_made_here(), 0);
    g.colour([1.0, 0.5, 0.25, 1.0]);
    g.clear(3, 0, 1.0);
    g.cover(0.5);
    assert_eq!(g.reds(), vec![255; 16]);
    let d = g.b.device().expect("the device");
    assert_eq!(d.modules_made_here(), 0, "no module made on the draw");
    assert_eq!(d.module_counts(), (1, 1));
    assert_eq!(d.pipelines_made_on_draws(), 1);
}

/// A draw that needs modules no module thread has started (the threads held) takes both jobs
/// back and makes them itself, and draws; let go, the threads make neither again, and a second
/// draw makes none. (After the game's start, its first draw, which waits for what is queued.)
#[test]
fn a_draw_takes_back_the_modules_no_thread_has_started() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.colour([1.0, 1.0, 1.0, 1.0]);
    g.clear(3, 0, 1.0);
    g.cover(0.5);
    let d = g.b.device().expect("the device");
    d.wait_for_modules();
    let before = d.modules_made_here();
    d.hold_modules(true);
    texcoord_pair(&mut g, VS + 11, PS + 19);
    // POSITION FLOAT4 and TEXCOORD0 FLOAT4 (0.2, 0.4, 0.6, 1): the module made ahead's inputs.
    let vertices: Vec<u8> = floats(
        &TRIANGLE
            .iter()
            .flat_map(|p| [p[0], p[1], 0.5, 1.0, 0.2, 0.4, 0.6, 1.0])
            .collect::<Vec<f32>>(),
    );
    g.clear(3, 0, 1.0);
    draw_with(
        &mut g,
        DECL + 10,
        &[(0, 3, 0, 0), (16, 3, 5, 0)],
        &vertices,
        32,
    );
    assert_eq!(g.pixels(), vec![[153, 102, 51, 255]; 16]);
    let d = g.b.device().expect("the device");
    assert_eq!(d.modules_made_here() - before, 2, "both jobs taken back");
    d.hold_modules(false);
    d.wait_for_modules();
    g.clear(3, 0, 1.0);
    draw_with(
        &mut g,
        DECL + 11,
        &[(0, 3, 0, 0), (16, 3, 5, 0)],
        &vertices,
        32,
    );
    assert_eq!(g.pixels(), vec![[153, 102, 51, 255]; 16]);
    let d = g.b.device().expect("the device");
    assert_eq!(d.modules_made_here() - before, 2, "none made again");
    assert_eq!(d.module_counts(), (1 + 1, 1 + 1));
}

/// The game's start, its first draw: with pipeline libraries it waits for every shader's
/// module and library the module threads have queued, so it takes none back and makes no
/// shader library itself. A shader the game creates after its start is counted.
#[test]
fn the_start_waits_for_every_shaders_library_and_counts_later_shaders() {
    // Each module job a second late, so jobs are in flight across the start's draw; a start that
    // waited 20 s fails instead of hanging. Undone when the test ends, however it ends.
    struct Timing;
    impl Drop for Timing {
        fn drop(&mut self) {
            d3d9_wgpu::device::module_timing_for_tests(0, 0);
        }
    }
    let _timing = Timing;
    d3d9_wgpu::device::module_timing_for_tests(1000, 20_000);
    let Some(mut g) = Gpu::new(4, 1) else { return };
    // The harness's two shaders' jobs are in flight (each module thread has one); two more
    // created with the threads held stay queued, for the start to take back.
    let d = g.b.device().expect("the device");
    d.hold_modules(true);
    texcoord_pair(&mut g, VS + 12, PS + 20);
    for (pixel, shader) in [(false, VS), (true, PS)] {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    let d = g.b.device().expect("the device");
    assert_eq!(d.shaders_created_after_start(), 0);
    assert_eq!(d.modules_pending(), 4);
    let libraries = d.pipeline_library_report().is_some();
    g.colour([1.0, 1.0, 1.0, 1.0]);
    g.clear(3, 0, 1.0);
    g.cover(0.5);
    assert_eq!(g.reds(), vec![255; 16]);
    let d = g.b.device().expect("the device");
    let before = library_counts(d);
    if libraries {
        assert_eq!(d.start_waited_for(), 4, "the start waited for all four");
        assert_eq!(d.modules_pending(), 0, "nothing left after the start");
        assert_eq!(
            d.modules_made_here(),
            0,
            "the start's draw made none itself"
        );
        // The four shaders' libraries and the stand-ins' (one pixel shader, two vertex
        // shaders: the harness's pixel shader reads nothing, the other TEXCOORD0).
        assert_eq!(before, Some((4, 3)));
        #[cfg(windows)]
        assert!(
            d.module_thread_priorities().iter().all(|&p| p == -1),
            "the module threads below normal again: {:?}",
            d.module_thread_priorities()
        );
    } else {
        eprintln!(
            "{}: no pipeline libraries, so the start does not wait",
            d.adapter_name()
        );
        assert_eq!(d.start_waited_for(), 0);
    }
    d.hold_modules(false);
    // Two new shaders after the start (vs_2_0: dcl_position v0; dcl_color v1; mov oPos, v0;
    // mov oD0, v1. ps_2_0: dcl v0; mov oC0, v0), and one made again (the same bytecode as one
    // made before: nothing new to make, not counted).
    let mut v = vec![0xFFFE_0200];
    v.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    v.extend(ins(31, &[0x8000_000A, 0x900F_0001]));
    v.extend(ins(1, &[0xC00F_0000, 0x90E4_0000]));
    v.extend(ins(1, &[0xD00F_0000, 0x90E4_0001]));
    v.push(END);
    let mut p = vec![0xFFFF_0200];
    p.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    p.extend(ins(1, &[0x800F_0800, 0x90E4_0000]));
    p.push(END);
    for (shader, pixel, tokens) in [(VS + 13, false, v), (PS + 21, true, p)] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
    }
    texcoord_pair(&mut g, VS + 14, PS + 22);
    let d = g.b.device().expect("the device");
    assert_eq!(d.shaders_created_after_start(), 2);
    // Their modules only: no library is made in the background during play.
    d.wait_for_modules();
    assert_eq!(library_counts(d), before, "no library for a later shader");
}

/// wgpu-hal's counts of the vertex and pixel shader libraries made so far; `None` without
/// libraries.
fn library_counts(d: &d3d9_wgpu::device::Device) -> Option<(u64, u64)> {
    let report = d.pipeline_library_report()?;
    // The number before `what` in wgpu-hal's line.
    let count = |what: &str| -> u64 {
        let end = report
            .find(what)
            .unwrap_or_else(|| panic!("no {what:?}: {report}"));
        report[..end]
            .rsplit(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("no count before {what:?}: {report}"))
    };
    Some((count(" vertex shader ("), count(" pixel shader (")))
}

/// A shader model 1 or 2 vertex shader always writes its two colours, as Direct3D 9's values
/// where it does not (COLOR0 white, COLOR1 (0, 0, 0, 1)), so its own module, made when the game
/// creates it, serves a pixel shader that reads a colour it does not write: the draws make no
/// module and draw those values.
#[test]
fn a_shader_model_1_or_2_vertex_shaders_own_module_serves_its_colours_readers() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    // vs_1_1: dcl_position v0; mov oPos, v0 (shader model 1 tokens carry no length).
    let vs1 = vec![
        0xFFFE_0101,
        0x0000_001F,
        0x8000_0000,
        0x900F_0000,
        0x0000_0001,
        0xC00F_0000,
        0x90E4_0000,
        END,
    ];
    // ps_2_0: dcl vN; mov oC0, vN (COLOR0, COLOR1).
    let ps = |n: u32| {
        let mut p = vec![0xFFFF_0200];
        p.extend(ins(31, &[0x8000_0000, 0x900F_0000 | n]));
        p.extend(ins(1, &[0x800F_0800, 0x90E4_0000 | n]));
        p.push(END);
        p
    };
    for (shader, pixel, tokens) in [
        (VS + 9, false, vs1),
        (PS + 17, true, ps(0)),
        (PS + 18, true, ps(1)),
    ] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
    }
    g.b.device().expect("the device").wait_for_modules();
    // The harness's vs_2_0 (its position only) and this vs_1_1, each with both pixel shaders.
    for vs in [VS, VS + 9] {
        for (ps, want) in [(PS + 17, [255, 255, 255, 255]), (PS + 18, [0, 0, 0, 255])] {
            for (pixel, shader) in [(false, vs), (true, ps)] {
                g.run(Cmd::SetShader {
                    device: DEVICE,
                    pixel,
                    shader,
                });
            }
            g.clear(3, 0x8080_8080, 1.0);
            g.cover(0.5);
            assert_eq!(g.pixels(), vec![want; 16], "{vs:#x} with {ps:#x}");
        }
    }
    let d = g.b.device().expect("the device");
    assert_eq!(d.modules_made_here(), 0, "every module made ahead");
    assert_eq!(
        d.module_counts(),
        (2, 1 + 2),
        "a vertex module for each vertex shader, its own"
    );
}

/// With pipeline libraries (wgpu-hal's, on Vulkan where the driver has them), each shader's
/// library is made with its module when the game creates it: the first draw of the pair then
/// makes no shader library (it links the two), and draws as before.
#[test]
fn a_shaders_pipeline_library_is_made_when_the_game_creates_it() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.colour([1.0, 0.5, 0.25, 1.0]);
    g.clear(3, 0, 1.0);
    let d = g.b.device().expect("the device");
    d.wait_for_modules();
    let Some(report) = d.pipeline_library_report() else {
        eprintln!(
            "{}: no pipeline libraries, so the case is not reached here",
            d.adapter_name()
        );
        return;
    };
    // The number before `what` in wgpu-hal's line.
    let count = |report: &str, what: &str| -> u64 {
        let end = report
            .find(what)
            .unwrap_or_else(|| panic!("no {what:?}: {report}"));
        report[..end]
            .rsplit(|c: char| !c.is_ascii_digit())
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| panic!("no count before {what:?}: {report}"))
    };
    let shaders = |r: &str| (count(r, " vertex shader ("), count(r, " pixel shader ("));
    let made = shaders(&report);
    // The harness's two shaders' and the stand-ins' they were made with.
    assert_eq!(made, (2, 2), "{report}");
    g.cover(0.5);
    assert_eq!(g.reds(), vec![255; 16]);
    let d = g.b.device().expect("the device");
    let after = d.pipeline_library_report().expect("the report");
    assert_eq!(
        shaders(&after),
        made,
        "a shader library made on the draw: {after}"
    );
    assert_eq!(d.pipelines_made_on_draws(), 1);
}

#[test]
fn user_clip_planes_cut_where_the_distance_is_negative() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.colour([1.0, 1.0, 1.0, 1.0]);
    // Kept where x - 0.25 w >= 0: of the pixel centres (clip x -1, -0.5, 0, 0.5) only the last
    // column.
    let plane = [1.0f32, 0.0, 0.0, -0.25].map(f32::to_bits);
    g.run(Cmd::SetClipPlane {
        device: DEVICE,
        index: 2,
        plane,
    });
    // Off: everything drawn.
    g.clear(3, 0, 1.0);
    g.cover(0.5);
    assert_eq!(g.reds(), vec![255; 16], "no plane on");
    // On (plane 2's bit).
    g.clear(3, 0, 1.0);
    g.state(CLIPPLANEENABLE, 1 << 2);
    g.cover(0.5);
    let row = [0, 0, 0, 255];
    assert_eq!(g.reds(), row.repeat(4), "plane 2 on");
    // `D3DRS_CLIPPING` off turns the planes off.
    g.clear(3, 0, 1.0);
    g.state(CLIPPING, 0);
    g.cover(0.5);
    assert_eq!(g.reds(), vec![255; 16], "clipping off");
    // With the hardware's clip distances (Vulkan, Metal) every draw takes them, planes on or
    // off: the three draws above share one pipeline. Without (Direct3D 12) a plane on is the
    // pixel shader's discard, another pipeline.
    let d = g.b.device().expect("the device");
    let want = if d.has_clip_distances() { 1 } else { 2 };
    assert_eq!(d.pipelines_made_on_draws(), want, "{}", d.adapter_name());
}

#[test]
#[cfg_attr(
    target_os = "macos",
    ignore = "Metal: multisampled occlusion counts and queries differ on the CI runner's device (Linux and macOS track, the window and drawing step)"
)]
fn occlusion_queries_count_the_samples_their_draws_pass() {
    for samples in [1u32, 4, 8] {
        let Some(mut g) = Gpu::new(4, samples) else {
            return;
        };
        for q in [0x2000, 0x2010, 0x2020, 0x2030, 0x2040] {
            g.run(Cmd::CreateQuery {
                device: DEVICE,
                query: q,
                kind: 9,
            });
        }
        g.colour([1.0, 0.0, 0.0, 1.0]);
        g.clear(3, 0, 1.0);
        let all = 16 * samples;
        assert_eq!(
            g.count(0x2000, |g| g.cover(0.25)),
            all,
            "{samples}: in front"
        );
        assert_eq!(g.count(0x2010, |g| g.cover(0.75)), 0, "{samples}: behind");
        // Two draws in one query: each sample passes twice.
        let n = g.count(0x2020, |g| {
            g.state(ZENABLE, 0);
            g.cover(0.5);
            g.cover(0.5);
            g.state(ZENABLE, 1);
        });
        assert_eq!(n, 2 * all, "{samples}: two draws");
        // Colour writes off still count; a later full clear does not.
        let n = g.count(0x2030, |g| {
            g.state(168, 0);
            g.cover(0.1);
            g.state(168, 0xF);
            g.clear(3, 0, 1.0);
        });
        assert_eq!(n, all, "{samples}: colour writes off");
        // A quad of the viewport's own edges: Direct3D 9's whole-number pixel centres leave the
        // samples left of the first column's centres and above the first row's out (the
        // standard patterns put half of them on each side; 1 of 4 and 2 of 8 are both left and
        // above). Measured so on the real Direct3D 9 here: 225 of 256 and 450 of 512 on an 8 by
        // 8 target.
        let lost = match samples {
            1 => 0,
            4 => 2 * 4 + 2 * 4 - 1,
            _ => 4 * 4 + 4 * 4 - 2,
        };
        assert_eq!(
            g.count(0x2040, |g| g.quad(0.25)),
            all - lost,
            "{samples}: the viewport's own edges"
        );
        // Asked again: the same count.
        match g.run(Cmd::QueryGetData {
            query: 0x2000,
            flags: 0,
        }) {
            Reply::Query(Some((0, n))) => assert_eq!(n, all),
            other => panic!("{other:?}"),
        }
        // An event query is the model's to answer.
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: 0x2100,
            kind: 8,
        });
        g.run(Cmd::QueryIssue {
            query: 0x2100,
            flags: 1,
        });
        assert!(matches!(
            g.run(Cmd::QueryGetData {
                query: 0x2100,
                flags: 1
            }),
            Reply::Query(None)
        ));
    }
}

/// `GetData` never waits: `S_FALSE` while the count is not there. Without `D3DGETDATA_FLUSH`
/// nothing sends the batch that counted, so the answer stays `S_FALSE` (the real Direct3D 9 here
/// answered so 20,000 times in a row); with the flag, or after a `Present` sent the batch, the
/// asking ends with the count.
#[test]
fn occlusion_get_data_answers_not_ready_until_the_count_is_there() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.colour([1.0, 0.0, 0.0, 1.0]);
    g.clear(3, 0, 1.0);
    for q in [0x2000, 0x2010] {
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: q,
            kind: 9,
        });
    }
    let ask = |g: &mut Gpu, query: u32, flags: u32| match g.run(Cmd::QueryGetData { query, flags })
    {
        Reply::Query(Some(answer)) => answer,
        other => panic!("{other:?}"),
    };
    // Asked before its END: not ready.
    g.run(Cmd::QueryIssue {
        query: 0x2000,
        flags: 2,
    });
    g.cover(0.5);
    assert_eq!(ask(&mut g, 0x2000, 1), (1, 0), "still counting");
    g.run(Cmd::QueryIssue {
        query: 0x2000,
        flags: 1,
    });
    for _ in 0..1000 {
        assert_eq!(ask(&mut g, 0x2000, 0), (1, 0), "not sent, not flushed");
    }
    assert_eq!(g.wait(0x2000, 1), 16, "flushed");
    // A `Present` sends the batch: asked without the flag, the count comes.
    g.clear(2, 0, 1.0);
    g.run(Cmd::QueryIssue {
        query: 0x2010,
        flags: 2,
    });
    g.cover(0.5);
    g.run(Cmd::QueryIssue {
        query: 0x2010,
        flags: 1,
    });
    assert_eq!(ask(&mut g, 0x2010, 0), (1, 0), "not sent yet");
    g.run(Cmd::Present {
        device: DEVICE,
        source: None,
        dest: None,
        window: 0,
        dirty: false,
        model: 0,
        frame: 1,
    });
    assert_eq!(g.wait(0x2010, 0), 16, "sent by the Present");
}

#[test]
fn get_render_target_data_and_read_surface_give_the_pixels_back() {
    let Some(mut g) = Gpu::new(2, 1) else { return };
    const SYS: u32 = 0x3000;
    g.run(Cmd::CreateOffscreenSurface {
        device: DEVICE,
        surface: SYS,
        width: 2,
        height: 2,
        format: A8R8G8B8,
        pool: 2,
    });
    g.clear(1, 0x1122_3344, 1.0);
    let rows = match g.run(Cmd::GetRenderTargetData {
        device: DEVICE,
        source: RT,
        destination: SYS,
        pitch: 16,
    }) {
        Reply::Rows(Some(r)) => r,
        other => panic!("{other:?}"),
    };
    let pixel = [0x44u8, 0x33, 0x22, 0x11];
    assert_eq!(
        rows,
        Rows {
            row: 8,
            bytes: pixel.repeat(4)
        }
    );
    match g.run(Cmd::ReadSurface {
        surface: SYS,
        size: 16,
    }) {
        Reply::Bytes(Some(b)) => assert_eq!(b, pixel.repeat(4)),
        other => panic!("{other:?}"),
    }
    // Without rows asked: none given, the level still kept.
    assert!(matches!(
        g.run(Cmd::GetRenderTargetData {
            device: DEVICE,
            source: RT,
            destination: SYS,
            pitch: 0,
        }),
        Reply::Rows(Some(Rows { row: 0, .. }))
    ));
    // Another size: none.
    assert!(matches!(
        g.run(Cmd::ReadSurface {
            surface: SYS,
            size: 12,
        }),
        Reply::Bytes(None)
    ));
}

/// Direct3D 9 refuses `GetRenderTargetData` of a multisampled render target
/// (`D3DERR_INVALIDCALL`, measured on the real one here): the destination keeps what it had.
#[test]
fn get_render_target_data_of_a_multisampled_target_leaves_the_destination() {
    let Some(mut g) = Gpu::new(2, 4) else { return };
    const SYS: u32 = 0x3000;
    g.run(Cmd::CreateOffscreenSurface {
        device: DEVICE,
        surface: SYS,
        width: 2,
        height: 2,
        format: A8R8G8B8,
        pool: 2,
    });
    let before: Vec<u8> = (0..16).collect();
    g.run(Cmd::UploadSurface {
        surface: SYS,
        row: 8,
        rows: 2,
        data: Data::Bytes(&before),
    });
    g.clear(1, 0x1122_3344, 1.0);
    match g.run(Cmd::GetRenderTargetData {
        device: DEVICE,
        source: RT,
        destination: SYS,
        pitch: 8,
    }) {
        Reply::Rows(Some(r)) => assert_eq!(r.bytes, before, "the destination's own rows"),
        other => panic!("{other:?}"),
    }
    match g.run(Cmd::ReadSurface {
        surface: SYS,
        size: 16,
    }) {
        Reply::Bytes(Some(b)) => assert_eq!(b, before),
        other => panic!("{other:?}"),
    }
    // A destination never written holds zeros, and is read back as such: an answer, so the
    // model goes on (it stops the run on none).
    const FRESH: u32 = 0x3010;
    g.run(Cmd::CreateOffscreenSurface {
        device: DEVICE,
        surface: FRESH,
        width: 2,
        height: 2,
        format: A8R8G8B8,
        pool: 2,
    });
    match g.run(Cmd::GetRenderTargetData {
        device: DEVICE,
        source: RT,
        destination: FRESH,
        pitch: 8,
    }) {
        Reply::Rows(Some(r)) => assert_eq!((r.row, r.bytes), (8, vec![0; 16])),
        other => panic!("{other:?}"),
    }
    match g.run(Cmd::ReadSurface {
        surface: FRESH,
        size: 16,
    }) {
        Reply::Bytes(Some(b)) => assert_eq!(b, vec![0; 16]),
        other => panic!("{other:?}"),
    }
}

/// A lost device (a driver reset, the graphics card gone) reads nothing back: `GetRenderTargetData`
/// still answers, with what the destination holds (as the real Direct3D 9 backend reads its
/// destination after a failed copy), so a save's thumbnail made then does not stop the game
/// (the model stops the run on no answer). Drawing and presenting go on without a stop too.
#[test]
fn a_read_back_after_the_device_is_lost_answers_what_the_destination_holds() {
    let Some(mut g) = Gpu::new(2, 1) else { return };
    const SYS: u32 = 0x3000;
    g.run(Cmd::CreateOffscreenSurface {
        device: DEVICE,
        surface: SYS,
        width: 2,
        height: 2,
        format: A8R8G8B8,
        pool: 2,
    });
    let before: Vec<u8> = (0..16).collect();
    g.run(Cmd::UploadSurface {
        surface: SYS,
        row: 8,
        rows: 2,
        data: Data::Bytes(&before),
    });
    g.clear(1, 0x1122_3344, 1.0);
    g.b.device().expect("the device").lose();
    g.clear(1, 0x5566_7788, 1.0);
    g.colour([1.0, 0.0, 0.0, 1.0]);
    g.cover(0.5);
    g.run(Cmd::Present {
        device: DEVICE,
        source: None,
        dest: None,
        window: 0,
        dirty: false,
        model: 0,
        frame: 1,
    });
    match g.run(Cmd::GetRenderTargetData {
        device: DEVICE,
        source: RT,
        destination: SYS,
        pitch: 8,
    }) {
        Reply::Rows(Some(r)) => assert_eq!((r.row, r.bytes.clone()), (8, before.clone())),
        other => panic!("{other:?}"),
    }
    match g.run(Cmd::ReadSurface {
        surface: SYS,
        size: 16,
    }) {
        Reply::Bytes(Some(b)) => assert_eq!(b, before),
        other => panic!("{other:?}"),
    }
}

#[test]
fn managed_levels_read_back_what_the_game_gave_them() {
    let Some(mut g) = Gpu::new(2, 1) else { return };
    const TEX: u32 = 0x4000;
    const LEVELS: [u32; 2] = [0x4010, 0x4020];
    let bad_levels = words(&[0x4110]);
    let levels = words(&LEVELS);
    g.run(Cmd::CreateTexture {
        device: DEVICE,
        texture: TEX,
        levels: Data::Bytes(&levels),
        width: 2,
        height: 2,
        usage: 0,
        format: A8R8G8B8,
        pool: 1,
    });
    let top: Vec<u8> = (0..16).collect();
    g.run(Cmd::UploadSurface {
        surface: LEVELS[0],
        row: 8,
        rows: 2,
        data: Data::Bytes(&top),
    });
    match g.run(Cmd::ReadSurface {
        surface: LEVELS[0],
        size: 16,
    }) {
        Reply::Bytes(Some(b)) => assert_eq!(b, top),
        other => panic!("{other:?}"),
    }
    // An upload shorter than the level changes only the bytes it carries.
    g.run(Cmd::UploadSurface {
        surface: LEVELS[0],
        row: 8,
        rows: 1,
        data: Data::Bytes(&[0xAA; 8]),
    });
    let mut changed = top.clone();
    changed[..8].copy_from_slice(&[0xAA; 8]);
    match g.run(Cmd::ReadSurface {
        surface: LEVELS[0],
        size: 16,
    }) {
        Reply::Bytes(Some(b)) => assert_eq!(b, changed, "a short upload"),
        other => panic!("{other:?}"),
    }
    // A level never given its bytes: none.
    assert!(matches!(
        g.run(Cmd::ReadSurface {
            surface: LEVELS[1],
            size: 4,
        }),
        Reply::Bytes(None)
    ));
    // A texture the backend cannot make (a format it does not provide) still keeps its bytes.
    g.run(Cmd::CreateTexture {
        device: DEVICE,
        texture: 0x4100,
        levels: Data::Bytes(&bad_levels),
        width: 2,
        height: 2,
        usage: 0,
        format: 0x1234_5678,
        pool: 1,
    });
    g.run(Cmd::UploadSurface {
        surface: 0x4110,
        row: 8,
        rows: 2,
        data: Data::Bytes(&top),
    });
    assert!(matches!(
        g.run(Cmd::ReadSurface {
            surface: 0x4110,
            size: 16
        }),
        Reply::Bytes(Some(_))
    ));
    // Destroyed: gone.
    g.run(Cmd::Destroy { object: TEX });
    assert!(matches!(
        g.run(Cmd::ReadSurface {
            surface: LEVELS[0],
            size: 16,
        }),
        Reply::Bytes(None)
    ));
}

#[test]
fn a_clear_of_part_of_the_target_clears_that_part_at_the_depth_asked() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.colour([1.0, 1.0, 1.0, 1.0]);
    g.clear(3, 0, 1.0);
    // A draw with the depth range 0.5 to 0.5 leaves the pass's viewport so.
    g.viewport(0, 0, 4, 4, 0.5, 0.5);
    g.state(COLORWRITEENABLE, 0);
    g.cover(0.0);
    g.state(COLORWRITEENABLE, 0xF);
    // The left half's depth cleared to 1 (the viewport makes it a clear of part).
    g.viewport(0, 0, 2, 4, 0.0, 1.0);
    g.clear(2, 0, 1.0);
    // A draw at 0.9 with LESS passes only where the depth is 1.
    g.viewport(0, 0, 4, 4, 0.0, 1.0);
    g.state(ZFUNC, 2);
    g.cover(0.9);
    let row = [255, 255, 0, 0];
    assert_eq!(g.reds(), row.repeat(4));
    // A colour clear of the right half only.
    g.viewport(2, 0, 2, 4, 0.0, 1.0);
    g.clear(1, 0x0080_0000, 1.0);
    let row = [255, 255, 0x80, 0x80];
    assert_eq!(g.reds(), row.repeat(4));
}

const COLORWRITEENABLE: u32 = 168;

/// `D3DRS_POINTSIZE`, which carries ATI's alpha to coverage switch.
const POINTSIZE: u32 = 154;

#[test]
#[cfg_attr(
    target_os = "macos",
    ignore = "Metal: multisampled occlusion counts and queries differ on the CI runner's device (Linux and macOS track, the window and drawing step)"
)]
fn nvidia_alpha_to_coverage_stands_in_for_the_alpha_test_on_multisampled_targets() {
    coverage_switch_stands_in_for_the_alpha_test(ADAPTIVETESS_Y, *b"ATOC", 0, &[]);
}

/// ATI's switch (`D3DRS_POINTSIZE` 'A2M1' on, 'A2M0' off), which the game sends on the ATI path
/// (player report 005: without it, grass under multisampling kept the alpha test at its reference
/// of 10 and was drawn as solid tufts): exactly as NVIDIA's, both switches, the alpha test's part
/// and the samples' condition. A plain point size between leaves it on.
#[test]
#[cfg_attr(
    target_os = "macos",
    ignore = "Metal: multisampled occlusion counts and queries differ on the CI runner's device (Linux and macOS track, the window and drawing step)"
)]
fn atis_alpha_to_coverage_switch_does_as_nvidias() {
    let a2m0 = u32::from_le_bytes(*b"A2M0");
    coverage_switch_stands_in_for_the_alpha_test(
        POINTSIZE,
        *b"A2M1",
        a2m0,
        &[0x3f80_0000, 0x4000_0000],
    );
}

/// The checks of an alpha to coverage switch: `state` set to `on` (a FourCC), later to `off`; each
/// of `between` set on the same state while it is on leaves it on.
fn coverage_switch_stands_in_for_the_alpha_test(
    state: u32,
    on: [u8; 4],
    off: u32,
    between: &[u32],
) {
    let on = u32::from_le_bytes(on);
    let name = String::from_utf8_lossy(&on.to_le_bytes()).into_owned();
    for samples in [1u32, 4, 8] {
        let Some(mut g) = Gpu::new(4, samples) else {
            return;
        };
        g.state(ZENABLE, 0);
        g.state(ZWRITEENABLE, 0);
        g.state(ALPHATESTENABLE, 1);
        g.state(state, on);
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: 0x2000,
            kind: 9,
        });
        // The samples the whole-target triangle (16 pixels) passes with alpha `a`, comparison `func`
        // and reference `aref` (an occlusion count: exact, where a resolve rounds).
        let covered = |g: &mut Gpu, a: f32, func: u32, aref: u32| {
            g.state(ALPHAFUNC, func);
            g.state(ALPHAREF, aref);
            g.colour([1.0, 1.0, 1.0, a]);
            g.clear(1, 0, 1.0);
            g.count(0x2000, |g| g.cover(0.5))
        };
        // The alpha test off: neither the test nor the coverage, every sample drawn.
        g.state(ALPHATESTENABLE, 0);
        assert_eq!(
            covered(&mut g, 0.25, 5, 0x80),
            16 * samples,
            "{name} {samples}x: no alpha test"
        );
        g.state(ALPHATESTENABLE, 1);
        // The game's grass (`00bac160`): GREATER, reference 10, alpha 0.25. One sample: the alpha
        // test passes it whole; multisampled the coverage gives part of the samples.
        let grass = covered(&mut g, 0.25, 5, 10);
        if samples == 1 {
            assert_eq!(grass, 16, "{name}: the grass's alpha test at one sample");
        } else {
            assert!(
                grass < 16 * samples,
                "{name} {samples}x: the grass's alpha 0.25 covers {grass} of {} samples",
                16 * samples
            );
        }
        for &value in between {
            g.state(state, value);
            assert_eq!(
                covered(&mut g, 0.25, 5, 10),
                grass,
                "{name} {samples}x: after {value:#x} the switch is as it was"
            );
        }
        if samples == 1 {
            // One sample: the alpha test, with its reference.
            assert_eq!(covered(&mut g, 0.25, 5, 0x80), 0, "{name}: 0.25 > 0.5");
            assert_eq!(covered(&mut g, 0.75, 5, 0x80), 16, "{name}: 0.75 > 0.5");
            assert_eq!(covered(&mut g, 0.25, 2, 0x80), 16, "{name}: 0.25 < 0.5");
        } else {
            // Multisampled (measured so on the real Direct3D 9 here, DESIGN.md section 10): the
            // coverage comes from the alpha alone, the reference plays no part; GREATER,
            // NOTEQUAL, GREATEREQUAL and ALWAYS cover so, NEVER, LESS, EQUAL and LESSEQUAL draw
            // nothing. Which samples an alpha covers is the hardware's own (on the RTX 5070 Ti,
            // in both: 1 of 4 from alpha 8/64, 2 from 24/64, 3 from 41/64, 4 from 57/64), so
            // only its order and ends are checked here.
            let mut last = 0;
            for k in 0..=16 {
                let a = k as f32 / 16.0;
                let n = covered(&mut g, a, 8, 0x80);
                assert!(
                    n >= last && n <= 16 * samples,
                    "{name} {samples}x: alpha {a}: {n} after {last}"
                );
                last = n;
                for func in [5, 6, 7] {
                    for aref in [0, 0x80, 0xFF] {
                        assert_eq!(
                            covered(&mut g, a, func, aref),
                            n,
                            "{name} {samples}x: alpha {a}, comparison {func}, reference {aref:#x}"
                        );
                    }
                }
                for func in 1..=4 {
                    assert_eq!(
                        covered(&mut g, a, func, 0x80),
                        0,
                        "{name} {samples}x: comparison {func}"
                    );
                }
                let ends = match k {
                    0 => Some(0),
                    16 => Some(16 * samples),
                    _ => None,
                };
                if let Some(e) = ends {
                    assert_eq!(n, e, "{name} {samples}x: alpha {a}");
                }
            }
            assert!(
                covered(&mut g, 0.5, 8, 0x80) > 0 && covered(&mut g, 0.5, 8, 0x80) < 16 * samples,
                "{name} {samples}x: alpha 0.5 covers part of the samples"
            );
        }
        g.state(ALPHAFUNC, 5);
        g.state(ALPHAREF, 0x80);
        g.colour([1.0, 1.0, 1.0, 0.25]);
        // The switch off: the alpha test again.
        g.state(state, off);
        g.clear(1, 0, 1.0);
        g.cover(0.5);
        assert_eq!(g.reds(), vec![0; 16], "{name} {samples}: switch off");
        assert_eq!(
            covered(&mut g, 0.25, 5, 10),
            16 * samples,
            "{name} {samples}x: switch off, the grass's alpha test passes it whole"
        );
        if samples == 1 {
            continue;
        }
        // The switch alone, with no other state changed between the draws (nothing else to make
        // the draw's pipeline again): on, off and on again with the alpha test on throughout.
        g.state(ALPHAFUNC, 5);
        g.state(ALPHAREF, 0x80);
        g.colour([1.0, 1.0, 1.0, 0.25]);
        let alone = |g: &mut Gpu| {
            g.clear(1, 0, 1.0);
            g.count(0x2000, |g| g.cover(0.5))
        };
        assert_eq!(alone(&mut g), 0, "{name} {samples}x: off, 0.25 > 0.5");
        g.state(state, on);
        let part = alone(&mut g);
        assert!(
            part > 0 && part < 16 * samples,
            "{name} {samples}x: the switch alone turned on: {part} samples"
        );
        g.state(state, off);
        assert_eq!(
            alone(&mut g),
            0,
            "{name} {samples}x: the switch alone turned off: the alpha test again"
        );
        g.state(state, on);
        assert_eq!(alone(&mut g), part, "{name} {samples}x: on again");
        // The switch between two draws of one pass, the same triangle drawn again with nothing
        // uploaded between (a new pass or batch would make every block again anyway): the second
        // draw follows the switch, off then on.
        let switched = |g: &mut Gpu, to: u32| {
            g.clear(1, 0, 1.0);
            g.count(0x2000, |g| {
                g.cover(0.5);
                g.state(state, to);
                g.run(Cmd::DrawPrimitive {
                    device: DEVICE,
                    kind: 4,
                    start: 0,
                    count: 1,
                });
            })
        };
        assert_eq!(
            switched(&mut g, off),
            part,
            "{name} {samples}x: coverage, then the switch off within the pass: the alpha test"
        );
        assert_eq!(
            switched(&mut g, on),
            part,
            "{name} {samples}x: the alpha test, then the switch on within the pass: coverage"
        );
        // `Reset` turns it off: every state the draw needs set again, the switch not.
        reset_for_the_alpha_test(&mut g);
        assert_eq!(
            alone(&mut g),
            0,
            "{name} {samples}x: after Reset the switch is off"
        );
    }
}

/// `Reset` of the 4x4 test device, then what [`coverage_switch_stands_in_for_the_alpha_test`]'s
/// draws need again (Reset returns every state and binding to its default): the multisampled
/// target and its depth, the shaders, the declaration, no culling or depth, the alpha test
/// (GREATER, reference 0x80) and the alpha 0.25.
fn reset_for_the_alpha_test(g: &mut Gpu) {
    let present = words(&[4, 4, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
    g.run(Cmd::Reset {
        device: DEVICE,
        window: 0,
        present: Data::Bytes(&present),
        back_buffers: Data::Bytes(&words(&[BACK])),
        depth_stencil: 0,
    });
    g.run(Cmd::SetRenderTarget {
        device: DEVICE,
        index: 0,
        surface: RT,
    });
    g.run(Cmd::SetDepthStencilSurface {
        device: DEVICE,
        surface: DS,
    });
    for (pixel, shader) in [(false, VS), (true, PS)] {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    g.run(Cmd::SetVertexDeclaration {
        device: DEVICE,
        declaration: DECL,
    });
    g.state(CULLMODE, 1);
    g.state(ZENABLE, 0);
    g.state(ZWRITEENABLE, 0);
    g.state(ALPHATESTENABLE, 1);
    g.state(ALPHAFUNC, 5);
    g.state(ALPHAREF, 0x80);
    g.colour([1.0, 1.0, 1.0, 0.25]);
}

/// The alpha test compares as the reference machine's (measured on the real Direct3D 9 here,
/// DESIGN.md section 11): as if the alpha were rounded to 8 bits, the halfway points cut to 12
/// bits. With reference 0x80, GREATER passes from alpha 2065/4096 (128.56 of 255), not just above
/// 128/255; GREATEREQUAL from 2049/4096 (127.56); EQUAL between them. Measured on the real one:
/// GREATER, GREATEREQUAL, LESS and LESSEQUAL at one sample on an A8R8G8B8 target (one NVIDIA
/// card). EQUAL (case 3 here) and NOTEQUAL are derived from the same bounds, not measured; other
/// target formats get the same rule without a measurement there.
#[test]
fn the_alpha_test_compares_as_the_reference_machine() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.state(ZENABLE, 0);
    g.state(ALPHATESTENABLE, 1);
    g.state(ALPHAREF, 0x80);
    let drawn = |g: &mut Gpu, func: u32, a: f32| {
        g.state(ALPHAFUNC, func);
        g.colour([1.0, 1.0, 1.0, a]);
        g.clear(1, 0, 1.0);
        g.cover(0.5);
        g.reds()[5] == 255
    };
    let below = |x: f32| f32::from_bits((x / 4096.0).to_bits() - 1);
    for (func, from, to) in [
        (5, 2065.0, None),
        (7, 2049.0, None),
        (3, 2049.0, Some(2065.0)),
    ] {
        assert!(!drawn(&mut g, func, below(from)), "{func}: below {from}");
        assert!(drawn(&mut g, func, from / 4096.0), "{func}: at {from}");
        if let Some(to) = to {
            assert!(drawn(&mut g, func, below(to)), "{func}: below {to}");
            assert!(!drawn(&mut g, func, to / 4096.0), "{func}: at {to}");
        }
    }
    // Above 128/255 by less than the halfway point: not GREATER (a float comparison would pass).
    assert!(!drawn(&mut g, 5, 128.55 / 255.0));
    assert!(drawn(&mut g, 3, 128.55 / 255.0));
    // The ends: reference 0 is passed by every alpha at or above it, 255 by none above it.
    g.state(ALPHAREF, 0);
    assert!(drawn(&mut g, 7, 0.0));
    assert!(!drawn(&mut g, 2, 0.0));
    g.state(ALPHAREF, 0xFF);
    assert!(!drawn(&mut g, 5, 1.0));
    assert!(drawn(&mut g, 4, 1.0));
}

/// An 8-bit multisampled target resolves gamma-corrected, as the reference machine's driver does
/// (NVIDIA's antialiasing gamma correction): a quad of the viewport's own edges covers one sample
/// in four of the corner pixel and half of the first column's and row's, which the real Direct3D
/// 9 here resolves to 137 and 188 (white on black; a plain average gives 64 and 127), at 4x and
/// 8x.
#[test]
fn an_8_bit_target_resolves_gamma_corrected() {
    for samples in [4u32, 8] {
        let Some(mut g) = Gpu::new(8, samples) else {
            return;
        };
        g.state(ZENABLE, 0);
        g.colour([1.0, 1.0, 1.0, 1.0]);
        g.clear(1, 0, 1.0);
        g.quad(0.5);
        let reds = g.reds();
        for y in 0..8 {
            for x in 0..8 {
                let want = match (x, y) {
                    (0, 0) => 137,
                    (0, _) | (_, 0) => 188,
                    _ => 255,
                };
                assert_eq!(reds[y * 8 + x], want, "{samples}x: ({x}, {y}) in {reds:?}");
            }
        }
    }
}

/// A thin ("sliver") triangle on a multisampled target is shaded at a place inside itself: its
/// varyings are never extrapolated to a pixel centre it does not cover (the white dots and orbs in
/// Doc's house, measure/render-aa/orbs2/verdict). A pixel shader without `_centroid` reads a
/// texture coordinate that falls from 0.3 on the slivers' left edges to 0 on their right; each
/// sliver (0.09 of a pixel wide) covers the right-hand sample of 4 in every pixel of its column,
/// not the centre. Shaded inside a sliver, a pixel's sample is at most 0.3 (resolved with three
/// black samples: at most 41 of 255); at the centre the coordinate would be 1.3 or more, which
/// the target keeps as 1.0 (resolved: 137).
#[test]
fn a_sliver_triangle_is_shaded_inside_itself_on_a_multisampled_target() {
    let Some(mut g) = Gpu::new(16, 4) else { return };
    const VS2: u32 = 0x1301;
    const PS2: u32 = 0x1311;
    const DECL2: u32 = 0x1401;
    const VB2: u32 = 0x1501;
    // vs_2_0: dcl_position v0; dcl_texcoord v1; mov oPos, v0; mov oT0, v1.
    let mut vs = vec![0xFFFE_0200];
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(31, &[0x8000_0005, 0x900F_0001]));
    vs.extend(ins(1, &[0xC00F_0000, 0x90E4_0000]));
    vs.extend(ins(1, &[0xE00F_0000, 0x90E4_0001]));
    vs.push(END);
    // ps_2_0: dcl t0 (not _centroid); mov oC0, t0.
    let mut ps = vec![0xFFFF_0200];
    ps.extend(ins(31, &[0x8000_0000, 0xB00F_0000]));
    ps.extend(ins(1, &[0x800F_0800, 0xB0E4_0000]));
    ps.push(END);
    for (shader, pixel, tokens) in [(VS2, false, vs), (PS2, true, ps)] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    // FLOAT4 position, FLOAT4 texture coordinate 0, D3DDECL_END.
    let decl = [
        0u8, 0, 0, 0, 3, 0, 0, 0, 0, 0, 16, 0, 3, 0, 5, 0, 0xFF, 0, 0, 0, 17, 0, 0, 0,
    ];
    g.run(Cmd::CreateDeclaration {
        device: DEVICE,
        declaration: DECL2,
        elements: Data::Bytes(&decl),
    });
    g.run(Cmd::SetVertexDeclaration {
        device: DEVICE,
        declaration: DECL2,
    });
    // One sliver a column: from 0.30 to 0.36 of a pixel right of its centre (Direct3D 9's
    // centres are on whole coordinates), 0.09 wide, over the whole height; the right-hand
    // sample of the standard 4x pattern is 0.375 right of the centre.
    let (size, high) = (16.0f32, 0.3f32);
    let mut v = Vec::new();
    for c in 0..16 {
        let left = c as f32 + 0.30 + 0.01 * (c % 7) as f32;
        let x = |s: f32| s / size * 2.0 - 1.0;
        let (l, r) = (x(left), x(left + 0.09));
        let at = |px: f32, py: f32, t: f32| [px, py, 0.5, 1.0, t, t, t, 1.0];
        for p in [
            at(l, -1.0, high),
            at(l, 1.0, high),
            at(r, -1.0, 0.0),
            at(r, -1.0, 0.0),
            at(l, 1.0, high),
            at(r, 1.0, 0.0),
        ] {
            v.extend_from_slice(&p);
        }
    }
    let data = floats(&v);
    g.run(Cmd::CreateBuffer {
        device: DEVICE,
        buffer: VB2,
        index: false,
        length: data.len() as u32,
        usage: 0,
        format: 0,
        pool: 1,
    });
    g.run(Cmd::UploadBuffer {
        buffer: VB2,
        offset: 0,
        flags: 0,
        data: Data::Bytes(&data),
    });
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: VB2,
        offset: 0,
        stride: 32,
    });
    g.state(ZENABLE, 0);
    g.clear(1, 0, 1.0);
    g.run(Cmd::DrawPrimitive {
        device: DEVICE,
        kind: 4,
        start: 0,
        count: 32,
    });
    let reds = g.reds();
    let lit = reds.iter().filter(|&&r| r > 0).count();
    let brightest = reds.iter().copied().max().unwrap_or(0);
    assert!(
        lit >= 128,
        "the slivers cover a sample in most pixels: {reds:?}"
    );
    assert!(
        brightest <= 48,
        "shaded inside the slivers (at most 41), not at the centres (137): {reds:?}"
    );
}

/// `D3DFMT_A16B16G16R16F`: the game's HDR targets.
const A16B16G16R16F: u32 = 113;

/// A `Present` with no rectangles.
fn present(g: &mut Gpu, frame: u32) {
    g.run(Cmd::Present {
        device: DEVICE,
        source: None,
        dest: None,
        window: 0,
        dirty: false,
        model: 0,
        frame,
    });
}

/// A render-target texture of one level (`level` its surface).
fn target_texture(g: &mut Gpu, texture: u32, level: u32, size: u32, format: u32) {
    let levels = words(&[level]);
    g.run(Cmd::CreateTexture {
        device: DEVICE,
        texture,
        levels: Data::Bytes(&levels),
        width: size,
        height: size,
        usage: 1,
        format,
        pool: 0,
    });
}

/// The HDR watch (a diagnostic, `bright`) reads a multisampled HDR scene where it is resolved: a
/// 16 by 16 A16B16G16R16F scene at 4 samples, 0.1 everywhere, with `pixel` (a colour at a place)
/// drawn in frame 1 only, resolved into a texture by `StretchRect`; frames 1 and 5 are watched.
/// What the watch saw in frame 1, and in frame 5 (the same scene without the pixel).
fn watched_resolved_scene(
    pixel: ([f32; 4], u32),
) -> Option<(d3d9_wgpu::bright::Seen, d3d9_wgpu::bright::Seen)> {
    let mut g = Gpu::new(16, 4)?;
    const SCENE: u32 = 0x1A00;
    const RESOLVED: u32 = 0x1A10;
    g.run(Cmd::CreateSurface {
        device: DEVICE,
        surface: SCENE,
        depth: false,
        width: 16,
        height: 16,
        format: A16B16G16R16F,
        multisample: 4,
        quality: 0,
        flag: 0,
    });
    target_texture(&mut g, RESOLVED, RESOLVED + 1, 16, A16B16G16R16F);
    g.state(ZENABLE, 0);
    // Frames 1 (with the two pixels) to 5 (without): frames 1 and 5 are watched.
    for frame in 1..=5u32 {
        g.run(Cmd::SetRenderTarget {
            device: DEVICE,
            index: 0,
            surface: SCENE,
        });
        g.run(Cmd::SetDepthStencilSurface {
            device: DEVICE,
            surface: 0,
        });
        g.clear(1, 0, 1.0);
        g.colour([0.1, 0.1, 0.1, 1.0]);
        g.quad(0.5);
        if frame == 1 {
            let (colour, at) = pixel;
            g.viewport(at, at, 1, 1, 0.0, 1.0);
            g.colour(colour);
            g.quad(0.5);
        }
        g.run(Cmd::StretchRect {
            device: DEVICE,
            source: SCENE,
            source_rect: None,
            destination: RESOLVED + 1,
            destination_rect: None,
            filter: 0,
        });
        g.run(Cmd::SetRenderTarget {
            device: DEVICE,
            index: 0,
            surface: RT,
        });
        present(&mut g, frame);
    }
    let d = g.b.device().expect("the device");
    let judged = d.hdr_watch_settle();
    let frames: Vec<u64> = judged.iter().map(|(p, _)| *p).collect();
    assert_eq!(frames, vec![1, 5], "the watched presents: {judged:?}");
    let first = &judged[0].1;
    assert_eq!(first.len(), 1, "the resolved scene only: {judged:?}");
    let (target, seen) = first[0];
    assert_eq!((target.width, target.height), (16, 16));
    let (_, clean) = judged[1].1[0];
    assert_eq!((clean.not_finite, clean.dots), (0, 0), "{clean:?}");
    assert!(clean.brightest > 0.05 && clean.brightest < 0.2, "{clean:?}");
    assert_eq!(d.wgpu_errors(), 0);
    Some((seen, clean))
}

/// A lone bright dot (3000 at (5, 5)) in a resolved scene is named, and nothing is not finite.
#[test]
fn the_hdr_watch_names_a_lone_bright_dot_of_a_resolved_scene() {
    let Some((seen, _)) = watched_resolved_scene(([3000.0, 3000.0, 3000.0, 1.0], 5)) else {
        return;
    };
    assert_eq!((seen.not_finite, seen.dots), (0, 1), "{seen:?}");
    // The dot covers at least one of its pixel's samples (3000 resolved with 0.1s).
    assert!(
        seen.brightest > 700.0 && seen.brightest.is_finite(),
        "{seen:?}"
    );
}

/// An infinite pixel (at (10, 10)) in a resolved scene is counted as not finite, not as a dot.
#[test]
#[cfg_attr(
    target_os = "macos",
    ignore = "Metal: an infinity drawn into a half-float target and resolved arrives as a large finite value (CI run 44268804913: not finite 0, brightest 16384), so the watch, which tests finiteness by bits, has no infinity to see; likely Metal's default fast math, which wgpu-hal does not turn off (Linux and macOS track, the window and drawing step)"
)]
fn the_hdr_watch_names_an_infinite_pixel_of_a_resolved_scene() {
    let Some((seen, _)) = watched_resolved_scene(([f32::INFINITY, 0.0, 0.0, 1.0], 10)) else {
        return;
    };
    assert_eq!((seen.not_finite, seen.dots), (1, 0), "{seen:?}");
    // The infinite pixel never leaks into the brightest: the brightest finite value is the
    // scene's 0.1.
    assert!(seen.brightest > 0.05 && seen.brightest < 0.2, "{seen:?}");
}

/// The HDR watch reads a small float target the game draws into and leaves (the luminance and
/// adaptation targets) and follows its value: a 1 by 1 target drawn 1.0 in frame 1 and 0.01 in
/// frame 5 (both watched) is a jump; the read leaves what the target holds as it was.
#[test]
fn the_hdr_watch_follows_a_small_targets_value_and_leaves_it_unchanged() {
    let Some(mut g) = Gpu::new(16, 1) else { return };
    const LUM: u32 = 0x1A20;
    target_texture(&mut g, LUM, LUM + 1, 1, A16B16G16R16F);
    g.state(ZENABLE, 0);
    for frame in 1..=5u32 {
        g.run(Cmd::SetRenderTarget {
            device: DEVICE,
            index: 0,
            surface: LUM + 1,
        });
        g.run(Cmd::SetDepthStencilSurface {
            device: DEVICE,
            surface: 0,
        });
        let v = if frame == 5 { 0.01 } else { 1.0 };
        g.colour([v, v, v, 1.0]);
        g.quad(0.5);
        g.run(Cmd::SetRenderTarget {
            device: DEVICE,
            index: 0,
            surface: RT,
        });
        present(&mut g, frame);
    }
    let d = g.b.device().expect("the device");
    let judged = d.hdr_watch_settle();
    let values: Vec<f32> = judged.iter().map(|(_, r)| r[0].1.brightest).collect();
    assert_eq!(values.len(), 2, "{judged:?}");
    assert!(
        (values[0] - 1.0).abs() < 0.01 && (values[1] - 0.01).abs() < 0.001,
        "{values:?}"
    );
    assert_eq!(d.hdr_watch_totals().with_jumps, 1);
    // A16B16G16R16F, 8 bytes a pixel: 0.01 as a half float is about 0x211F.
    let (_, _, px) = d.read_surface_pixels(LUM + 1).expect("the target's pixel");
    let half = u16::from_le_bytes([px[0], px[1]]);
    assert!((0x211E..=0x2120).contains(&half), "{px:?}");
    assert_eq!(d.wgpu_errors(), 0);
}

/// A render-target texture nothing has drawn into, sampled by three draws: counted three times,
/// said once (a diagnostic with the HDR watch's switch).
#[test]
fn a_render_target_sampled_before_it_is_drawn_into_is_counted_and_said_once() {
    let Some(mut g) = Gpu::new(16, 1) else { return };
    const NEVER: u32 = 0x1A30;
    const PS_TEX: u32 = 0x1A40;
    target_texture(&mut g, NEVER, NEVER + 1, 4, A8R8G8B8);
    // ps_2_0: dcl t0; dcl_2d s0; texld r0, t0, s0; mov oC0, r0.
    let mut ps = vec![0xFFFF_0200];
    ps.extend(ins(31, &[0x8000_0000, 0xB00F_0000]));
    ps.extend(ins(31, &[0x9000_0000, 0xA00F_0800]));
    ps.extend(ins(66, &[0x800F_0000, 0xB0E4_0000, 0xA0E4_0800]));
    ps.extend(ins(1, &[0x800F_0800, 0x80E4_0000]));
    ps.push(END);
    let t = words(&ps);
    g.run(Cmd::CreateShader {
        device: DEVICE,
        shader: PS_TEX,
        pixel: true,
        tokens: Data::Bytes(&t),
    });
    g.run(Cmd::SetShader {
        device: DEVICE,
        pixel: true,
        shader: PS_TEX,
    });
    g.run(Cmd::SetTexture {
        device: DEVICE,
        sampler: 0,
        texture: NEVER,
    });
    g.state(ZENABLE, 0);
    let d = g.b.device().expect("the device");
    let (samples0, said0) = d.stale_sample_counts();
    for _ in 0..3 {
        g.quad(0.5);
    }
    let d = g.b.device().expect("the device");
    let (samples, said) = d.stale_sample_counts();
    assert_eq!((samples - samples0, said - said0), (3, 1));
    assert_eq!(d.wgpu_errors(), 0);
}

/// The register blocks are written only when what they carry changed, and only up to the
/// highest constant the bound shaders read: within one pass, (1) a change of only a vertex
/// constant moves the next draw; (2) a vertex shader reading a constant beyond the ones the last
/// block carried (c10, set before any draw) gets it with no new constant call; (3) a change of
/// only a pixel constant changes the next draw's colour.
#[test]
fn constant_changes_between_draws_in_one_pass_reach_the_shaders() {
    let Some(mut g) = Gpu::new(8, 1) else { return };
    const VS4: u32 = 0x1320;
    const VS10: u32 = 0x1330;
    // vs_2_0: dcl_position v0; add oPos, v0, cN.
    for (shader, n) in [(VS4, 4u32), (VS10, 10)] {
        let mut t = vec![0xFFFE_0200];
        t.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
        t.extend(ins(2, &[0xC00F_0000, 0x90E4_0000, 0xA0E4_0000 | n]));
        t.push(END);
        let t = words(&t);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel: false,
            tokens: Data::Bytes(&t),
        });
    }
    let vconst = |g: &mut Gpu, n: u32, v: [f32; 4]| {
        let r = floats(&v);
        g.run(Cmd::SetConstants {
            device: DEVICE,
            kind: 0,
            start: n,
            count: 1,
            data: Data::Bytes(&r),
        });
    };
    let shader = |g: &mut Gpu, s: u32| {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel: false,
            shader: s,
        })
    };
    // The top left quarter of the target (clip x -1 to 0, y 0 to 1).
    let corner = |g: &mut Gpu| {
        let q = floats(&[
            -1.0, 0.0, 0.5, 1.0, -1.0, 1.0, 0.5, 1.0, 0.0, 0.0, 0.5, 1.0, //
            0.0, 0.0, 0.5, 1.0, -1.0, 1.0, 0.5, 1.0, 0.0, 1.0, 0.5, 1.0,
        ]);
        let offset = 96 * (g.next % 32);
        g.next += 1;
        g.run(Cmd::UploadBuffer {
            buffer: VB,
            offset,
            flags: 0,
            data: Data::Bytes(&q),
        });
        g.run(Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: VB,
            offset,
            stride: 16,
        });
        g.run(Cmd::DrawPrimitive {
            device: DEVICE,
            kind: 4,
            start: 0,
            count: 2,
        });
    };
    g.state(ZENABLE, 0);
    g.state(CULLMODE, 1);
    vconst(&mut g, 10, [0.0, -1.0, 0.0, 0.0]);
    vconst(&mut g, 4, [0.0, 0.0, 0.0, 0.0]);
    shader(&mut g, VS4);
    g.colour([1.0, 0.0, 0.0, 1.0]);
    g.clear(1, 0, 1.0);
    corner(&mut g); // top left, red
    vconst(&mut g, 4, [1.0, 0.0, 0.0, 0.0]);
    corner(&mut g); // top right, red: only c4 changed
    shader(&mut g, VS10);
    corner(&mut g); // bottom left, red: c10 was never sent with a draw
    shader(&mut g, VS4);
    vconst(&mut g, 4, [1.0, -1.0, 0.0, 0.0]);
    corner(&mut g); // bottom right, red
    g.colour([0.0, 1.0, 0.0, 1.0]);
    corner(&mut g); // bottom right again, green: only the pixel shader's c0 changed
    let px = g.pixels();
    let at = |x: usize, y: usize| px[y * 8 + x];
    assert_eq!(at(1, 1), [0, 0, 255, 255], "top left");
    assert_eq!(
        at(6, 1),
        [0, 0, 255, 255],
        "top right: the vertex constant change"
    );
    assert_eq!(
        at(1, 6),
        [0, 0, 255, 255],
        "bottom left: c10 on the shader change"
    );
    assert_eq!(
        at(6, 6),
        [0, 255, 0, 255],
        "bottom right: the pixel constant change"
    );
}

/// The pipeline cache file's envelope, as `cache.rs` writes it (magic, version 2, length,
/// FNV-1a 64 of the bytes): the test makes files of its own with it.
fn envelope(data: &[u8]) -> Vec<u8> {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut out = b"NVWGPUPC".to_vec();
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(data.len() as u64).to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    out.extend_from_slice(data);
    out
}

/// The driver's pipelines kept between runs. Where the backend has a pipeline cache (Vulkan), a
/// device that made a pipeline leaves the file behind and the next one reads it; a file whose
/// bytes do not match their hash, a truncated one, and a FOREIGN one (a valid envelope around
/// wgpu bytes made for another adapter) are each ignored with a new, empty cache, and the device
/// draws the same pixels; a folder that is a file gives no cache and a working device. Without a
/// pipeline cache (Direct3D 12, the software adapter) nothing is kept and the draws are the same.
#[test]
fn the_pipeline_cache_file_is_kept_and_a_bad_one_is_ignored() {
    // A scratch folder in the build's own target folder.
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("pipeline-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The pixels and what the device made of its cache file.
    let draw = |cache: Option<std::path::PathBuf>| -> Option<(Vec<u8>, Option<String>)> {
        let mut g = Gpu::with_cache(4, 1, cache)?;
        g.colour([1.0, 0.5, 0.25, 1.0]);
        g.clear(1, 0, 1.0);
        g.cover(0.5);
        let note = g.b.device().expect("the device").pipeline_cache_note();
        Some((g.reds(), note))
    };
    let Some((plain, none)) = draw(None) else {
        return;
    };
    assert_eq!(none, None, "no folder, no cache");
    let folder = dir.join("new");
    let (first, note) = draw(Some(folder.clone())).unwrap();
    assert_eq!(first, plain);
    let Some(note) = note else {
        // No pipeline cache on this backend: nothing kept, nothing to read back.
        // (The pipeline list, `pipelines.txt`, is kept on every backend.)
        assert!(std::fs::read_dir(&folder).map_or(true, |d| d
            .flatten()
            .all(|e| e.path().extension() != Some(std::ffi::OsStr::new("bin")))));
        eprintln!("this backend keeps no pipeline cache");
        return;
    };
    assert!(note.starts_with("none yet"), "{note}");
    // The file is there after the device went, and is read back.
    let files: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "bin"))
        .collect();
    assert_eq!(files.len(), 1, "{files:?}");
    let file = files[0].clone();
    let good = std::fs::read(&file).unwrap();
    let (pixels, note) = draw(Some(folder.clone())).unwrap();
    assert_eq!(pixels, plain);
    assert!(note.as_deref().unwrap().ends_with("KB read"), "{note:?}");
    // Each bad file: ignored by name, a new cache, the same pixels.
    let wgpu_bytes = &good[28..];
    let mut altered = good.clone();
    let n = altered.len();
    altered[n / 2] ^= 0x55;
    let truncated = good[..good.len() - 3].to_vec();
    // wgpu's header: magic 8, version 4, ABI 4, backend 1, then the adapter key (15).
    let mut foreign_bytes = wgpu_bytes.to_vec();
    foreign_bytes[20] ^= 0x5A;
    let foreign = envelope(&foreign_bytes);
    let mut backend_bytes = wgpu_bytes.to_vec();
    backend_bytes[16] ^= 0x03;
    let other_backend = envelope(&backend_bytes);
    for (bytes, why) in [
        (altered, "do not match their hash"),
        (truncated, "truncated"),
        (foreign, "ignored"),
        (other_backend, "ignored"),
    ] {
        std::fs::write(&file, &bytes).unwrap();
        let (pixels, note) = draw(Some(folder.clone())).unwrap();
        assert_eq!(pixels, plain, "{why}");
        let note = note.unwrap();
        assert!(
            note.contains(why) && note.contains("ignored"),
            "{why}: {note}"
        );
    }
    // A "folder" that is a file: no cache, the device works.
    std::fs::write(dir.join("a-file"), b"not a folder").unwrap();
    let (pixels, note) = draw(Some(dir.join("a-file"))).unwrap();
    assert_eq!(pixels, plain);
    assert_eq!(note, None);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The backend's own loading screen, a sepia test card: the logo in its first ring; the bar
/// filled to the fraction done (a quarter against three quarters: some row has the ink fill at
/// three quarters where it has the paper inside the bar at a quarter); and (on Windows,
/// where GDI draws the text) another status line changes pixels. A window narrower than the bar
/// still draws.
#[test]
fn the_loading_screen_shows_the_logo_the_text_and_the_bar() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    let p = |status: &str, done: f32| d3d9_wgpu::loading::Progress {
        title: "Loading".into(),
        status: status.into(),
        done: Some(done),
    };
    let (w, h) = (640u32, 360u32);
    let d = g.b.device().expect("the device");
    let quarter = d.progress_pixels(w, h, &p("Preparing shaders 212 of 1480", 0.25));
    let most = d.progress_pixels(w, h, &p("Preparing shaders 212 of 1480", 0.75));
    let other = d.progress_pixels(w, h, &p("Preparing shaders 1111 of 1480", 0.25));
    // Narrower than the bar (64 pixels at least) and the logo: no underflow, a picture.
    let narrow = d.progress_pixels(40, 30, &p("x", 0.5));
    assert_eq!(narrow.len(), 40 * 30 * 4);
    assert_eq!(quarter.len(), (w * h * 4) as usize);
    let at = |px: &[u8], x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    // The card: sepia paper (red well above blue) over most of it.
    let sepia = quarter
        .chunks_exact(4)
        .filter(|p| p[2] > 80 && p[2] > p[0].saturating_add(30))
        .count();
    assert!(sepia > (w * h / 2) as usize, "{sepia} sepia pixels");
    // The logo in the first ring (the card has no ink there): dark pixels near the centre.
    let (cx, cy, r) = (w / 2, h * 44 / 100, h / 2 * 30 / 100);
    let dark = (cy - r..cy + r)
        .flat_map(|y| (cx - r..cx + r).map(move |x| (x, y)))
        .filter(|&(x, y)| x.abs_diff(cx).pow(2) + y.abs_diff(cy).pow(2) < r * r)
        .filter(|&(x, y)| at(&quarter, x, y).iter().all(|&c| c < 90))
        .count();
    assert!(dark > 200, "{dark} dark pixels of the logo");
    // The bar, centred under the banner: in its middle column the ink fill (dark) at three
    // quarters where a quarter shows the paper inside it, in the same row.
    let x = w / 2;
    let fill = |c: [u8; 3]| c.iter().all(|&v| v < 90);
    let paper = |c: [u8; 3]| c[2] > 150;
    let row = (0..h).find(|&y| fill(at(&most, x, y)) && paper(at(&quarter, x, y)));
    assert!(
        row.is_some(),
        "no row with the bar filled at 3/4 and empty at 1/4"
    );
    #[cfg(windows)]
    {
        let changed = quarter
            .chunks_exact(4)
            .zip(other.chunks_exact(4))
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            changed > 20,
            "{changed} pixels changed with the status line"
        );
    }
    #[cfg(not(windows))]
    let _ = other;
    // NV_LOADING_PICTURE=<file.bmp>: the card at 1920 by 1080 with some of a run's lines, to
    // look at.
    if let Some(file) = std::env::var_os("NV_LOADING_PICTURE") {
        d3d9_wgpu::loading::show_lines(&[
            "NV_D3D9 wgpu: pipeline cache wgpu_pipeline_cache_vulkan.bin: 18908 KB read",
            "NV_D3D9 wgpu: pipeline list: 272 entries (272 shipped, 0 more; 0 lines not read)",
            "NV_D3D9 wgpu: the game's device on the adapter (Vulkan) (1920x1080, format 22)",
            "NV_D3D9 wgpu: Loading: Preparing shaders 263 of 1038 (25%)",
            "NV_D3D9 wgpu: Loading: Preparing shaders 508 of 1038 (49%)",
        ]);
        let (w, h) = (1920u32, 1080u32);
        let px = d.progress_pixels(w, h, &p("Preparing shaders 508 of 1038", 0.49));
        d3d9_wgpu::loading::show_lines(&[]);
        let mut bmp = b"BM".to_vec();
        for v in [
            54 + w * h * 4,
            0,
            54,
            40,
            w,
            (h as i32).wrapping_neg() as u32,
        ] {
            bmp.extend_from_slice(&v.to_le_bytes());
        }
        bmp.extend_from_slice(&[1, 0, 32, 0]);
        bmp.extend_from_slice(&[0; 24]);
        bmp.extend_from_slice(&px);
        std::fs::write(file, bmp).expect("the picture");
    }
}

/// The frame-rate counter over a game's frame: readable on a dark frame (light glyphs) and on a
/// bright one (its dark plate), only in the top-right corner, every other pixel the frame's own
/// (it is drawn over the frame already there, not instead of it). Windows only: GDI draws its
/// glyphs. `NV_FPS_PICTURE=<file.bmp>`: the counter at 1920 by 1080 over a mid-grey frame, to
/// look at.
#[cfg(windows)]
#[test]
fn the_frame_rate_counter_sits_over_the_frame_in_its_top_right_corner() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    let (w, h) = (640u32, 360u32);
    let readout = d3d9_wgpu::frame_rate::Readout {
        fps: 144,
        slowest_tenths_ms: 93,
    };
    let d = g.b.device().expect("the device");
    let dark = d.frame_rate_pixels(w, h, [0.1, 0.1, 0.1, 1.0], readout);
    let bright = d.frame_rate_pixels(w, h, [1.0, 1.0, 1.0, 1.0], readout);
    assert_eq!(dark.len(), (w * h * 4) as usize);
    let at = |px: &[u8], x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    let corner = |x: u32, y: u32| x >= w - 160 && y < 60;
    let (bg_dark, bg_bright) = (at(&dark, 0, h - 1), at(&bright, 0, h - 1));
    assert!(bg_dark.iter().all(|&c| c < 40), "{bg_dark:?}");
    assert_eq!(bg_bright, [255, 255, 255]);
    let mut light_in_dark = 0;
    let mut dark_in_bright = 0;
    for y in 0..h {
        for x in 0..w {
            let (a, b) = (at(&dark, x, y), at(&bright, x, y));
            if !corner(x, y) {
                assert_eq!(
                    (a, b),
                    (bg_dark, bg_bright),
                    "outside the corner at {x},{y}"
                );
                continue;
            }
            light_in_dark += usize::from(a.iter().all(|&c| c > 200));
            dark_in_bright += usize::from(b.iter().all(|&c| c < 140));
        }
    }
    assert!(
        light_in_dark > 40,
        "{light_in_dark} light glyph pixels on the dark frame"
    );
    assert!(
        dark_in_bright > 1000,
        "{dark_in_bright} plate pixels on the bright frame"
    );
    // Another readout changes the glyphs.
    let other = d.frame_rate_pixels(
        w,
        h,
        [0.1, 0.1, 0.1, 1.0],
        d3d9_wgpu::frame_rate::Readout {
            fps: 37,
            slowest_tenths_ms: 412,
        },
    );
    assert!(dark
        .chunks_exact(4)
        .zip(other.chunks_exact(4))
        .any(|(a, b)| a != b));
    if let Some(file) = std::env::var_os("NV_FPS_PICTURE") {
        let (w, h) = (1920u32, 1080u32);
        let px = d.frame_rate_pixels(w, h, [0.45, 0.5, 0.55, 1.0], readout);
        std::fs::write(file, d3d9_wgpu::shot::bmp(w, h, &px)).expect("the picture");
    }
}

/// The pipelines a run made are listed in the cache folder; the next run makes them at the first
/// `Present` after their shaders exist, so its draws make none (and draw the same pixels). A
/// damaged list is skipped line by line, never a failed start.
#[test]
fn listed_pipelines_are_made_before_the_draws_that_need_them() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("pipeline-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let present = |g: &mut Gpu| {
        g.run(Cmd::Present {
            device: DEVICE,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame: 1,
        });
    };
    let run = |cache: &std::path::Path| -> Option<(Vec<u8>, u64)> {
        let mut g = Gpu::with_cache(4, 1, Some(cache.to_path_buf()))?;
        present(&mut g);
        g.colour([1.0, 0.5, 0.25, 1.0]);
        g.clear(1, 0, 1.0);
        g.cover(0.5);
        let reds = g.reds();
        let made = g.b.device().expect("the device").pipelines_made_on_draws();
        Some((reds, made))
    };
    let Some((first, made_first)) = run(&dir) else {
        return;
    };
    assert!(
        made_first >= 1,
        "the first run makes its pipeline on the draw"
    );
    let list = std::fs::read_to_string(dir.join("pipelines.txt")).expect("the list written");
    assert!(list.lines().count() >= 1, "{list}");
    let (second, made_second) = run(&dir).unwrap();
    assert_eq!(
        second, first,
        "the same pixels from the pipeline made ahead"
    );
    assert_eq!(made_second, 0, "no pipeline made on a draw: {list}");
    // A damaged list: the readable lines are still made ahead.
    std::fs::write(
        dir.join("pipelines.txt"),
        format!("junk\n{list}p1 broken\n"),
    )
    .unwrap();
    let (third, made_third) = run(&dir).unwrap();
    assert_eq!(third, first);
    assert_eq!(made_third, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// With pipeline libraries, a cold start (no pipeline cache file) makes every shader's library
/// ahead; a warm one (the file the cold run wrote, read back) only the listed shaders' (the pair
/// the cold run drew), and a pair no list names makes its libraries on its first draw (from the
/// driver's cache: about a millisecond each).
#[test]
fn a_warm_start_makes_only_the_listed_shaders_libraries() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("warm-start-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // A run: the harness's pair drawn first (the start), with another pair made before it; that
    // pair drawn after (`other`). The cache note, the jobs the start waited for, and the library
    // counts after the start and at the end.
    type Run = (String, usize, (u64, u64), (u64, u64));
    let run = |other: bool| -> Option<Run> {
        let mut g = Gpu::with_cache(4, 1, Some(dir.clone()))?;
        texcoord_pair(&mut g, VS + 12, PS + 20);
        for (pixel, shader) in [(false, VS), (true, PS)] {
            g.run(Cmd::SetShader {
                device: DEVICE,
                pixel,
                shader,
            });
        }
        g.colour([1.0, 1.0, 1.0, 1.0]);
        g.clear(3, 0, 1.0);
        g.cover(0.5);
        let d = g.b.device().expect("the device");
        let note = d.pipeline_cache_note().unwrap_or_default();
        let waited = d.start_waited_for();
        let start = library_counts(d)?;
        if other {
            for (pixel, shader) in [(false, VS + 12), (true, PS + 20)] {
                g.run(Cmd::SetShader {
                    device: DEVICE,
                    pixel,
                    shader,
                });
            }
            let vertices: Vec<u8> = floats(
                &TRIANGLE
                    .iter()
                    .flat_map(|p| [p[0], p[1], 0.5, 1.0, 0.2, 0.4, 0.6, 1.0])
                    .collect::<Vec<f32>>(),
            );
            g.clear(3, 0, 1.0);
            draw_with(
                &mut g,
                DECL + 10,
                &[(0, 3, 0, 0), (16, 3, 5, 0)],
                &vertices,
                32,
            );
            assert_eq!(g.pixels(), vec![[153, 102, 51, 255]; 16]);
        }
        let end = library_counts(g.b.device().expect("the device"))?;
        Some((note, waited, start, end))
        // The device goes: the list and the pipeline cache file are written.
    };
    let Some((note, waited, start, end)) = run(false) else {
        eprintln!("no pipeline libraries here, so the case is not reached");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    };
    // Cold: every shader's library at the start: the four, the shipped list's two fixed-function
    // shaders (the intro movie's, made at every start: `ffp`) and the stand-ins (two vertex
    // shaders for the two pixel shaders' varyings, one pixel shader).
    assert!(note.starts_with("none yet"), "{note}");
    assert_eq!((waited, start, end), (6, (6, 4), (6, 4)), "cold");
    let (note, waited, start, end) = run(true).expect("the second run");
    // Warm: the listed pair's libraries and their stand-ins' only (the other pair's modules
    // are still made ahead); its draw makes the other pair's two. The shipped list's
    // fixed-function pair is listed too: two vertex libraries (its vertex shader and a stand-in)
    // and one pixel library more throughout.
    assert!(note.ends_with("KB read"), "{note}");
    assert_eq!(waited, 6, "every module waited for");
    assert_eq!(
        start,
        (2 + 2, 2 + 1),
        "warm: the listed pairs' and their stand-ins'"
    );
    assert_eq!(end, (3 + 2, 3 + 1), "the other pair's on its first draw");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A multisampled entry is made ahead at the run's own multisample count, whatever count it was
/// recorded at (the shipped list is recorded at one antialiasing setting): the list a 4x run
/// wrote leaves an 8x run no pipeline to make on its draws, and the same pixels as an 8x run
/// without a list.
#[test]
fn a_list_recorded_at_4x_serves_a_run_at_8x() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("pipeline-list-8x-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let run = |samples: u32, cache: Option<&std::path::Path>| -> Option<(Vec<u8>, u64)> {
        let mut g = Gpu::with_cache(4, samples, cache.map(std::path::Path::to_path_buf))?;
        g.run(Cmd::Present {
            device: DEVICE,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame: 1,
        });
        g.colour([1.0, 0.5, 0.25, 1.0]);
        g.clear(1, 0, 1.0);
        g.cover(0.5);
        let reds = g.reds();
        let made = g.b.device().expect("the device").pipelines_made_on_draws();
        Some((reds, made))
    };
    let Some((_, made_4x)) = run(4, Some(&dir)) else {
        return;
    };
    assert!(made_4x >= 1, "the 4x run makes its pipeline on the draw");
    let list = std::fs::read_to_string(dir.join("pipelines.txt")).expect("the list written");
    let (plain, _) = run(8, None).unwrap();
    let (listed, made_8x) = run(8, Some(&dir)).unwrap();
    assert_eq!(made_8x, 0, "the 8x run makes no pipeline on a draw: {list}");
    assert_eq!(listed, plain, "the same pixels as an 8x run without a list");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two triangles (a triangle list) covering clip x from `x0` to `x1` and every row, at depth 0.5.
fn rect(x0: f32, x1: f32) -> Vec<f32> {
    let (y0, y1) = (-3.0, 3.0);
    vec![
        x0, y0, 0.5, 1.0, x0, y1, 0.5, 1.0, x1, y0, 0.5, 1.0, //
        x1, y0, 0.5, 1.0, x0, y1, 0.5, 1.0, x1, y1, 0.5, 1.0,
    ]
}

/// On a 4 by 4 target: the left two columns red, the right two green, nothing else.
fn left_red_right_green(g: &mut Gpu, what: &str) {
    let px = g.pixels();
    for (i, p) in px.iter().enumerate() {
        let want = if i % 4 < 2 {
            [0, 0, 255, 255]
        } else {
            [0, 255, 0, 255]
        };
        assert_eq!(*p, want, "{what}: pixel {i} of {px:?}");
    }
}

/// Uploads into a buffer the open batch already draws from, with no Present or read-back between
/// them: each draw reads the bytes uploaded before it, not those uploaded after it, and an upload
/// after the last draw changes nothing drawn. The vertices: the left half uploaded at offset 0
/// and drawn red, the right half uploaded at the same offset and drawn green, then the whole
/// target uploaded there with no draw. The indices: the same with an index buffer rewritten
/// between two `DrawIndexedPrimitive`s over one vertex buffer that holds all three rectangles.
/// Copies that landed before every draw would make the target all green; copies dropped would
/// leave the right half black.
#[test]
fn each_draw_reads_the_upload_before_it_in_one_batch() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    // Clip x of the pixel centres (Direct3D 9's whole coordinates): -1, -0.5, 0, 0.5; the
    // halves meet at -0.25, between the second and third column.
    let (left, right, whole) = (rect(-3.0, -0.25), rect(-0.25, 3.0), rect(-3.0, 3.0));
    const VB2: u32 = 0x1600;
    const VB3: u32 = 0x1610;
    const IB: u32 = 0x1620;
    for (buffer, index, length, format) in [
        (VB2, false, 96, 0),
        (VB3, false, 288, 0),
        (IB, true, 12, 101),
    ] {
        g.run(Cmd::CreateBuffer {
            device: DEVICE,
            buffer,
            index,
            length,
            usage: 0,
            format,
            pool: 1,
        });
    }
    let upload = |g: &mut Gpu, buffer: u32, data: &[u8]| {
        g.run(Cmd::UploadBuffer {
            buffer,
            offset: 0,
            flags: 0,
            data: Data::Bytes(data),
        });
    };
    // Vertices.
    g.clear(3, 0, 1.0);
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: VB2,
        offset: 0,
        stride: 16,
    });
    g.colour([1.0, 0.0, 0.0, 1.0]);
    upload(&mut g, VB2, &floats(&left));
    g.run(Cmd::DrawPrimitive {
        device: DEVICE,
        kind: 4,
        start: 0,
        count: 2,
    });
    g.colour([0.0, 1.0, 0.0, 1.0]);
    upload(&mut g, VB2, &floats(&right));
    g.run(Cmd::DrawPrimitive {
        device: DEVICE,
        kind: 4,
        start: 0,
        count: 2,
    });
    upload(&mut g, VB2, &floats(&whole));
    left_red_right_green(&mut g, "vertices");
    // Indices: one vertex buffer with the three rectangles (vertices 0-5, 6-11, 12-17).
    let all: Vec<f32> = [left, right, whole].concat();
    upload(&mut g, VB3, &floats(&all));
    g.clear(3, 0, 1.0);
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: VB3,
        offset: 0,
        stride: 16,
    });
    g.run(Cmd::SetIndices {
        device: DEVICE,
        buffer: IB,
    });
    let indices =
        |first: u16| -> Vec<u8> { (first..first + 6).flat_map(u16::to_le_bytes).collect() };
    let draw = |g: &mut Gpu| {
        g.run(Cmd::DrawIndexedPrimitive {
            device: DEVICE,
            kind: 4,
            base_vertex: 0,
            min_index: 0,
            vertices: 18,
            start_index: 0,
            count: 2,
        });
    };
    g.colour([1.0, 0.0, 0.0, 1.0]);
    upload(&mut g, IB, &indices(0));
    draw(&mut g);
    g.colour([0.0, 1.0, 0.0, 1.0]);
    upload(&mut g, IB, &indices(6));
    draw(&mut g);
    upload(&mut g, IB, &indices(12));
    left_red_right_green(&mut g, "indices");
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

/// Bindings skip looking an object up when the same handle is bound again; an object destroyed
/// and made again at a bound handle is looked up again all the same. Bound to the same handles
/// with the same offset and stride: a vertex buffer, an index buffer and a pixel shader, each
/// made again with other contents. Planted bugs it catches: any of the three not looked up again.
#[test]
fn a_handle_made_again_while_bound_is_looked_up_again() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    const VB2: u32 = 0x1700;
    const IB: u32 = 0x1710;
    let make = |g: &mut Gpu, buffer: u32, index: bool, data: &[u8]| {
        g.run(Cmd::CreateBuffer {
            device: DEVICE,
            buffer,
            index,
            length: data.len() as u32,
            usage: 0,
            format: if index { 101 } else { 0 },
            pool: 1,
        });
        g.run(Cmd::UploadBuffer {
            buffer,
            offset: 0,
            flags: 0,
            data: Data::Bytes(data),
        });
    };
    let bind = |g: &mut Gpu| {
        g.run(Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: VB2,
            offset: 0,
            stride: 16,
        });
        g.run(Cmd::SetIndices {
            device: DEVICE,
            buffer: IB,
        });
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel: true,
            shader: PS,
        });
    };
    let draw = |g: &mut Gpu| {
        g.run(Cmd::DrawIndexedPrimitive {
            device: DEVICE,
            kind: 4,
            base_vertex: 0,
            min_index: 0,
            vertices: 12,
            start_index: 0,
            count: 2,
        });
    };
    let indices =
        |first: u16| -> Vec<u8> { (first..first + 6).flat_map(u16::to_le_bytes).collect() };
    // Each piece made again so that only the new one draws the right half: a stale vertex buffer
    // (or index buffer) would draw an off-target rectangle, a stale pixel shader red.
    let off = rect(5.0, 6.0);
    // c0 red, c1 green; the left half drawn with the first objects.
    let regs = floats(&[1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0]);
    g.run(Cmd::SetConstants {
        device: DEVICE,
        kind: 3,
        start: 0,
        count: 2,
        data: Data::Bytes(&regs),
    });
    g.clear(3, 0, 1.0);
    make(
        &mut g,
        VB2,
        false,
        &floats(&[rect(-3.0, -0.25), off.clone()].concat()),
    );
    make(&mut g, IB, true, &indices(0));
    bind(&mut g);
    draw(&mut g);
    // Each destroyed and made again at its handle: the right half in the second six vertices,
    // indices of those, a pixel shader writing c1.
    for h in [VB2, IB, PS] {
        g.run(Cmd::Destroy { object: h });
    }
    make(
        &mut g,
        VB2,
        false,
        &floats(&[off, rect(-0.25, 3.0)].concat()),
    );
    make(&mut g, IB, true, &indices(6));
    let mut ps = vec![0xFFFF_0300];
    ps.extend(ins(1, &[0x800F_0800, 0xA0E4_0001]));
    ps.push(END);
    let t = words(&ps);
    g.run(Cmd::CreateShader {
        device: DEVICE,
        shader: PS,
        pixel: true,
        tokens: Data::Bytes(&t),
    });
    bind(&mut g);
    draw(&mut g);
    left_red_right_green(&mut g, "made again");
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

/// A texture's handle made again without being destroyed binds the new texture; once a buffer
/// is made at it, or once it is destroyed, setting it as a texture binds none (as setting no
/// texture does): the sets look textures up in a map of their own, which follows every object
/// made at or removed from a handle. The pixel shader adds its texture to the vertex texture's
/// green. Planted bugs it catches: a texture made again not put in the textures' map; a buffer
/// made at a texture's handle, or a destroy, leaving the texture there.
#[test]
fn a_handle_made_again_as_another_texture_or_a_buffer_is_set_as_that() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    const PS_TEX: u32 = 0x1602;
    stage_samplers(&mut g);
    for (pixel, shader) in [(false, VS + 2), (true, PS + 6)] {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    let set = |g: &mut Gpu, texture: u32| {
        g.run(Cmd::SetTexture {
            device: DEVICE,
            sampler: 0,
            texture,
        });
        g.clear(3, 0, 1.0);
        g.cover(0.5);
        g.pixels()
    };
    let make = |g: &mut Gpu, bgra: [u8; 4]| {
        let levels = words(&[PS_TEX + 1]);
        g.run(Cmd::CreateTexture {
            device: DEVICE,
            texture: PS_TEX,
            levels: Data::Bytes(&levels),
            width: 1,
            height: 1,
            usage: 0,
            format: A8R8G8B8,
            pool: 1,
        });
        g.run(Cmd::UploadSurface {
            surface: PS_TEX + 1,
            row: 4,
            rows: 1,
            data: Data::Bytes(&bgra),
        });
    };
    let (yellow, cyan) = (vec![[0, 255, 255, 255]; 16], vec![[255, 255, 0, 255]; 16]);
    // Red: green + red.
    assert_eq!(set(&mut g, PS_TEX), yellow, "the first");
    // Made again, blue, no destroy between: green + blue.
    make(&mut g, [255, 0, 0, 255]);
    assert_eq!(set(&mut g, PS_TEX), cyan, "made again");
    let none = set(&mut g, 0);
    assert_ne!(none, cyan, "no texture is not blue");
    // A buffer at the handle: no texture there any more.
    g.run(Cmd::CreateBuffer {
        device: DEVICE,
        buffer: PS_TEX,
        index: false,
        length: 64,
        usage: 0,
        format: 0,
        pool: 1,
    });
    assert_eq!(set(&mut g, PS_TEX), none, "a buffer at the handle");
    // The buffer destroyed, a texture made there again, then destroyed: none.
    g.run(Cmd::Destroy { object: PS_TEX });
    make(&mut g, [255, 0, 0, 255]);
    assert_eq!(set(&mut g, PS_TEX), cyan, "made after the buffer");
    set(&mut g, 0);
    g.run(Cmd::Destroy { object: PS_TEX });
    assert_eq!(set(&mut g, PS_TEX), none, "destroyed");
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

/// A buffer's handle draws from the buffer made there, and draws nothing once a texture or a
/// surface is made at it or it is destroyed (as no buffer bound: the draw is skipped): the sets
/// look buffers up in a map of their own, which follows every object made at or removed from a
/// handle. Planted bugs it catches: a buffer not put in the buffers' map; a texture or a surface
/// made at a buffer's handle, or a destroy, leaving the buffer there.
#[test]
fn a_buffers_handle_made_again_as_another_object_or_destroyed_draws_nothing() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    const VB3: u32 = 0x1720;
    g.colour([1.0, 0.0, 0.0, 1.0]);
    let make = |g: &mut Gpu| {
        let tri = floats(&[
            -3.0, -3.0, 0.5, 1.0, -3.0, 9.0, 0.5, 1.0, 9.0, -3.0, 0.5, 1.0,
        ]);
        g.run(Cmd::CreateBuffer {
            device: DEVICE,
            buffer: VB3,
            index: false,
            length: 48,
            usage: 0,
            format: 0,
            pool: 1,
        });
        g.run(Cmd::UploadBuffer {
            buffer: VB3,
            offset: 0,
            flags: 0,
            data: Data::Bytes(&tri),
        });
    };
    // Unbound first, so that the set looks its handle up whatever was made at it since.
    let draw = |g: &mut Gpu, buffer: u32| {
        g.clear(3, 0, 1.0);
        for buffer in [0, buffer] {
            g.run(Cmd::SetStreamSource {
                device: DEVICE,
                stream: 0,
                buffer,
                offset: 0,
                stride: 16,
            });
        }
        g.run(Cmd::DrawPrimitive {
            device: DEVICE,
            kind: 4,
            start: 0,
            count: 1,
        });
        g.pixels()
    };
    let red = vec![[0, 0, 255, 255]; 16];
    let none = draw(&mut g, 0);
    assert_ne!(none, red, "nothing drawn without a buffer");
    make(&mut g);
    assert_eq!(draw(&mut g, VB3), red, "the buffer");
    // A texture made at the handle, no destroy between.
    let levels = words(&[VB3 + 1]);
    g.run(Cmd::CreateTexture {
        device: DEVICE,
        texture: VB3,
        levels: Data::Bytes(&levels),
        width: 1,
        height: 1,
        usage: 0,
        format: A8R8G8B8,
        pool: 1,
    });
    assert_eq!(draw(&mut g, VB3), none, "a texture at the handle");
    // A buffer again (over the texture), then a surface over it.
    make(&mut g);
    assert_eq!(draw(&mut g, VB3), red, "a buffer over the texture");
    g.run(Cmd::CreateSurface {
        device: DEVICE,
        surface: VB3,
        depth: false,
        width: 1,
        height: 1,
        format: A8R8G8B8,
        multisample: 0,
        quality: 0,
        flag: 0,
    });
    assert_eq!(draw(&mut g, VB3), none, "a surface at the handle");
    // A buffer again, destroyed.
    make(&mut g);
    assert_eq!(draw(&mut g, VB3), red, "a buffer over the surface");
    g.run(Cmd::Destroy { object: VB3 });
    assert_eq!(draw(&mut g, VB3), none, "destroyed");
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

/// With the answers board (the render thread's way to the game's threads), the backend posts each
/// occlusion count once the GPU has it, through its idle work alone: the same count a reader that
/// waits gets, for the game's last `END` of the query only; an event query gets none.
#[test]
fn occlusion_counts_are_posted_for_reading_without_waiting() {
    let Some(mut g) = Gpu::new(4, 4) else { return };
    let answers = std::sync::Arc::new(d3d9_wgpu::QueryAnswers::default());
    g.b.answer_to(std::sync::Arc::clone(&answers));
    g.colour([1.0, 0.0, 0.0, 1.0]);
    g.clear(3, 0, 1.0);
    for (q, kind) in [(0x2000, 9), (0x2010, 8)] {
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: q,
            kind,
        });
    }
    let posted = |g: &mut Gpu, ends: u64| {
        let start = std::time::Instant::now();
        loop {
            let more = g.b.idle();
            let a = answers.get(0x2000).expect("made");
            if let (true, Some(n)) = (a.ends == ends, a.count) {
                return n;
            }
            assert!(more, "nothing awaited, nothing posted: {a:?}");
            assert!(start.elapsed().as_secs() < 10, "not posted: {a:?}");
            std::thread::sleep(d3d9_stream::IDLE_POLL);
        }
    };
    // The whole 4 by 4 target in front of the cleared depth: 16 pixels, 4 samples each.
    g.run(Cmd::QueryIssue {
        query: 0x2000,
        flags: 2,
    });
    g.cover(0.5);
    g.run(Cmd::QueryIssue {
        query: 0x2000,
        flags: 1,
    });
    assert_eq!(posted(&mut g, 1), 64);
    assert!(!g.b.idle(), "nothing awaited once posted");
    // A second END: its own count; the reader that waits agrees.
    g.clear(2, 0, 1.0);
    let waited = g.count(0x2000, |g| g.cover(0.25));
    assert_eq!(posted(&mut g, 2), waited);
    // An event query is the model's: no answer posted for it.
    g.run(Cmd::QueryIssue {
        query: 0x2010,
        flags: 1,
    });
    let event = answers.get(0x2010).expect("made");
    assert!(!event.occlusion && event.count.is_none(), "{event:?}");
}

/// One pass of far more draws than an encoder takes before it goes to a finishing thread
/// (`SPLIT_DRAWS`, 256), each covering the target: every draw counted exactly once across the
/// encoders (an occlusion count spanning all of them), and the last draw's colour on top (their
/// order kept), with each draw's vertices uploaded into the same buffer as the batch goes on.
#[test]
fn draws_split_across_finished_encoders_keep_their_order_and_count() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.run(Cmd::CreateQuery {
        device: DEVICE,
        query: 0x2000,
        kind: 9,
    });
    g.state(ZFUNC, 8);
    g.clear(3, 0, 1.0);
    let n = 600u32;
    let colour = |i: u32| {
        [
            (i % 7) as f32 / 6.0,
            (i % 5) as f32 / 4.0,
            (i % 3) as f32 / 2.0,
            1.0,
        ]
    };
    let count = g.count(0x2000, |g| {
        for i in 0..n {
            g.colour(colour(i));
            g.cover(0.5);
        }
    });
    assert_eq!(count, n * 16, "every draw counted once");
    let last = colour(n - 1).map(|c| (c * 255.0).round() as u8);
    for p in g.pixels() {
        assert_eq!(p, [last[2], last[1], last[0], 255], "the last draw on top");
    }
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

/// The cuts between finished encoders keep what a multisampled target and its depth hold: with a
/// real depth test (LESSEQUAL), 300 draws at depth 0.25 in red, then 600 at depth 0.75 in green
/// that cross two cuts (every 256 draws). Only the red draws pass, so the count over all of them
/// is the red draws' samples and every resolved pixel is red; a cut that cleared or dropped the
/// colour or the depth (of any sample) would let green through or leave a pixel black.
#[test]
#[cfg_attr(
    target_os = "macos",
    ignore = "Metal: multisampled occlusion counts and queries differ on the CI runner's device (Linux and macOS track, the window and drawing step)"
)]
fn cuts_between_finished_encoders_keep_multisampled_colour_and_depth() {
    for samples in [1u32, 4, 8] {
        let Some(mut g) = Gpu::new(4, samples) else {
            return;
        };
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: 0x2000,
            kind: 9,
        });
        g.state(ZFUNC, 4);
        g.clear(3, 0, 1.0);
        let count = g.count(0x2000, |g| {
            g.colour([1.0, 0.0, 0.0, 1.0]);
            for _ in 0..300 {
                g.cover(0.25);
            }
            g.colour([0.0, 1.0, 0.0, 1.0]);
            for _ in 0..600 {
                g.cover(0.75);
            }
        });
        assert_eq!(
            count,
            300 * 16 * samples,
            "{samples} samples: only the red draws pass"
        );
        for p in g.pixels() {
            assert_eq!(p, [0, 0, 255, 255], "{samples} samples: red everywhere");
        }
        assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
    }
}

/// The card's own times (`NV_WGPU_GPU_TIMES`) off, the default: the device asks the driver for
/// neither timestamp feature, and no timing is made, so no query set, no timestamp and no command
/// of its own.
#[test]
fn the_gpu_times_off_make_nothing() {
    let Some(mut g) = Gpu::new(4, 4) else {
        return;
    };
    g.run(Cmd::CreateQuery {
        device: DEVICE,
        query: 0x2000,
        kind: 9,
    });
    g.clear(3, 0, 1.0);
    let _ = g.count(0x2000, |g| {
        g.cover(0.5);
        g.cover(0.5);
    });
    let _ = g.pixels();
    let d = g.b.device().expect("the device");
    assert_eq!(
        d.timestamp_features(),
        (false, false),
        "neither timestamp feature asked for"
    );
    assert!(!d.gpu_times_on());
    assert_eq!(d.gpu_time_totals(), None, "no timing, no query set");
}

/// The card's own times on: each kind of work the backend gives the card is timed in its own
/// pairs: the game's draws (world), the occlusion count's pass, a multisampled target's resolve,
/// and (where the driver writes timestamps between passes) the copies: an upload among the draws
/// and the read-back. Off where the adapter has no timestamps (said, skipped); where it has them
/// the times must be on, every pair read back in order, and the count, the submits and the
/// batches' encoders are the same as with the times off (no pass, batch or submit added).
#[test]
fn the_gpu_times_on_time_each_kind_of_work() {
    use d3d9_wgpu::gpu_times::{Kind, KINDS};
    let run = |g: &mut Gpu| {
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: 0x2000,
            kind: 9,
        });
        g.clear(3, 0, 1.0);
        let count = g.count(0x2000, |g| {
            g.cover(0.5);
            g.cover(0.5);
        });
        let _ = g.pixels();
        let s = g.b.device().expect("the device").submits();
        (count, s.submits, s.encoders)
    };
    let off = {
        let Some(mut g) = Gpu::new(4, 4) else {
            return;
        };
        run(&mut g)
    };
    let Some(mut g) = Gpu::with_env(4, 4, &[("NV_WGPU_GPU_TIMES", "1")]) else {
        return;
    };
    let d = g.b.device().expect("the device");
    if !d.adapter_has_timestamps() {
        assert!(!d.gpu_times_on());
        eprintln!("this adapter cannot time passes: skipped");
        return;
    }
    assert!(d.gpu_times_on(), "asked for, and the adapter can");
    let on = run(&mut g);
    assert_eq!(
        on, off,
        "the same count, submits and batches as with the times off"
    );
    let slot = |k: Kind| KINDS.iter().position(|(x, _)| *x == k).expect("a kind");
    let start = std::time::Instant::now();
    let t = loop {
        let t = g.b.device().and_then(|d| d.gpu_time_totals()).expect("on");
        if t.spans[slot(Kind::Resolves)] > 0 || t.zeros > 0 || start.elapsed().as_secs() >= 5 {
            break t;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert!(t.sets_made >= 1, "{t:?}");
    if t.zeros > 0 {
        // A driver that reads timestamps as zero: the times are said to be missing, not shown (the
        // line's own unit test checks that). Known only of AMD's driver (raw wgpu too); on any
        // other it is a fault of ours.
        let d = g.b.device().expect("the device");
        let name = d.adapter_name().to_string();
        assert!(
            name.contains("AMD") || name.contains("Radeon"),
            "zero timestamps on {name}, not known to give them: {t:?}"
        );
        eprintln!("{name} reads timestamps as zero: the times are not shown: {t:?}");
        assert_eq!(d.wgpu_errors(), 0);
        return;
    }
    for kind in [Kind::World, Kind::Count, Kind::Resolves] {
        assert!(t.spans[slot(kind)] >= 1, "{kind:?} timed: {t:?}");
    }
    assert!(t.ns[slot(Kind::World)] > 0, "the draws took time: {t:?}");
    assert_eq!(t.left_out, 0, "every pair in order: {t:?}");
    assert_eq!(t.untimed, 0, "{t:?}");
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

/// A `DrawPrimitiveUP` draw that lands where the register ring is full draws its own vertices: the
/// batch is sent for the ring's room before the draw's bytes are placed, not after (which handed
/// them to the batch before and left the draw reading another draw's bytes at their place in the
/// next: stray triangles). The ring is given room for one draw's blocks, so every UP draw after a
/// batch's first lands at a full ring: red over the whole target, then green on the left, then blue
/// on the right in more bytes than the green draw's place had after the red's.
#[test]
fn an_up_draw_at_a_full_register_ring_draws_its_own_vertices() {
    struct Room;
    impl Drop for Room {
        fn drop(&mut self) {
            d3d9_wgpu::device::ring_room_for_tests(0);
        }
    }
    let Some(mut g) = Gpu::new(4, 1) else {
        return;
    };
    let _room = Room;
    d3d9_wgpu::device::ring_room_for_tests(2 * d3d9_wgpu::device::register_block_for_tests() + 1);
    let up = |g: &mut Gpu, corners: &[[f32; 2]]| {
        let v: Vec<f32> = corners
            .iter()
            .flat_map(|&[x, y]| [x, y, 0.5, 1.0])
            .collect();
        g.run(Cmd::DrawPrimitiveUp {
            device: DEVICE,
            kind: 4,
            count: corners.len() as u32 / 3,
            stride: 16,
            vertices: Data::Bytes(&floats(&v)),
        });
    };
    // One triangle over the whole height left of x = 0.1 (its point far left), and its mirror right
    // of it.
    let left = [[0.1, -9.0], [0.1, 9.0], [-20.0, 0.0]];
    let right = [[0.1, -9.0], [0.1, 9.0], [20.0, 0.0]];
    g.clear(3, 0, 1.0);
    let sends = g.b.device().expect("the device").ring_sends();
    g.colour([1.0, 0.0, 0.0, 1.0]);
    up(&mut g, &[[-3.0, -3.0], [-3.0, 9.0], [9.0, -3.0]]);
    // The green draw's bytes go where the red's end (48 bytes in); placed before the send they went
    // with the red's batch, and the draw read the next batch's bytes there.
    g.colour([0.0, 1.0, 0.0, 1.0]);
    up(&mut g, &left);
    // The blue draw's two triangles (the right one twice) fill the next batch's first 96 bytes: at
    // the green draw's 48 a misplaced green draw finds the right triangle, and the left stays red.
    g.colour([0.0, 0.0, 1.0, 1.0]);
    up(&mut g, &[right, right].concat());
    let px = g.pixels();
    let d = g.b.device().expect("the device");
    assert!(
        d.ring_sends() >= sends + 2,
        "the green and the blue draw landed at a full ring"
    );
    for row in 0..4 {
        assert_eq!(px[row * 4], [0, 255, 0, 255], "row {row}: the left green");
        assert_eq!(
            px[row * 4 + 3],
            [255, 0, 0, 255],
            "row {row}: the right blue"
        );
    }
    assert_eq!(d.wgpu_errors(), 0);
}

/// A batch of more passes than one submit can carry goes in several, in order: AMD's Windows
/// driver refuses a submit of about 4,000 of its command buffers ("out of host memory"; Radeon
/// 890M), and wgpu then takes the device for lost: nothing more is drawn and no query answers.
/// 900 draws in one batch, nine occlusion queries of 100 each, each draw uploading its vertices
/// into a ring of 64 places (a copy, a pass and the count's pass each: 4,507 command buffers in
/// one submit before), with and without the finishing and queue threads. Each draw is nearer than
/// every one before it under a LESS depth test, so it passes only if every draw before it ran
/// before it, and with its own vertices (a place written again after 64 draws): every query
/// answers with all of its draws' samples only if no work moved across a submit. The last
/// draw's colour is left, nothing fails, and no submit takes more than
/// [`d3d9_wgpu::device::SUBMIT_USES`].
#[test]
fn a_batch_of_more_passes_than_a_submit_takes_goes_in_several() {
    const QUERIES: u32 = 9;
    const EACH: u32 = 100;
    for threads in [true, false] {
        let Some(mut g) = Gpu::with_threads(4, 1, threads) else {
            return;
        };
        if threads {
            assert!(
                g.b.device().is_some_and(|d| d.has_queue_thread()),
                "the queue thread on, whatever the machine"
            );
        }
        // Threaded, the queue thread held 20 ms a batch, so batches pile up unless each early send
        // waits for the ones before it.
        struct Delay;
        impl Drop for Delay {
            fn drop(&mut self) {
                d3d9_wgpu::device::submit_delay_for_tests(0);
            }
        }
        let _delay = threads.then(|| {
            d3d9_wgpu::device::submit_delay_for_tests(20);
            Delay
        });
        for q in 0..QUERIES {
            g.run(Cmd::CreateQuery {
                device: DEVICE,
                query: 0x2000 + q,
                kind: 9,
            });
        }
        // LESS.
        g.state(ZFUNC, 2);
        g.clear(3, 0, 1.0);
        for q in 0..QUERIES {
            g.run(Cmd::QueryIssue {
                query: 0x2000 + q,
                flags: 2,
            });
            for j in 0..EACH {
                let i = q * EACH + j;
                let last = i == QUERIES * EACH - 1;
                g.colour(if last {
                    [0.0, 1.0, 0.0, 1.0]
                } else {
                    [1.0, 0.0, 0.0, 1.0]
                });
                // Exact in the 24-bit depth: 1 - (i + 1) / 1024.
                g.cover(1.0 - (i + 1) as f32 / 1024.0);
            }
            g.run(Cmd::QueryIssue {
                query: 0x2000 + q,
                flags: 1,
            });
        }
        for q in 0..QUERIES {
            assert_eq!(
                g.wait(0x2000 + q, 1),
                EACH * 16,
                "threads {threads}: query {q}: each draw nearer than the ones before passed"
            );
        }
        for p in g.pixels() {
            assert_eq!(
                p,
                [0, 255, 0, 255],
                "threads {threads}: the last draw's green"
            );
        }
        let d = g.b.device().expect("the device");
        assert_eq!(d.wgpu_errors(), 0, "threads {threads}");
        let s = d.submits();
        let limit = d3d9_wgpu::device::SUBMIT_USES;
        assert!(
            s.most_uses <= limit,
            "threads {threads}: a submit took {} uses, more than {limit}",
            s.most_uses
        );
        // Three uses a draw (the copy, the pass, the count's pass).
        assert!(
            s.submits > u64::from(3 * 900 / limit),
            "threads {threads}: {s:?}"
        );
        check_submits(&s, &format!("threads {threads}"));
        // The batch was sent as it reached the limit (one command's few uses past it), so its
        // command buffers never waited unsubmitted all together.
        assert!(
            s.most_batch <= limit + 8,
            "threads {threads}: a batch took {} uses before it was sent",
            s.most_batch
        );
        assert!(
            s.bound_sends >= u64::from(3 * 900 / limit),
            "threads {threads}: the batch sent early as it reached the limit: {s:?}"
        );
        // At most about two batches' uses handed over and not yet submitted (Metal loses the
        // device past 4,096 command buffers made and not submitted).
        assert!(
            s.most_pending <= 2 * u64::from(limit + 8),
            "threads {threads}: {} uses were waiting to be submitted at once",
            s.most_pending
        );
    }
}

/// What every submit carried was what its part planned, and no command buffer was lost: counted
/// from the buffers each submit really got ([`d3d9_wgpu::device::SubmitStats`]).
fn check_submits(s: &d3d9_wgpu::device::SubmitStats, what: &str) {
    assert!(
        s.most_buffers <= s.most_planned,
        "{what}: a submit got {} command buffers, its part planned at most {}",
        s.most_buffers,
        s.most_planned
    );
    assert_eq!(
        s.buffers, s.encoders,
        "{what}: every encoder handed over was submitted, once"
    );
    assert_eq!(
        s.uses, s.batch_uses,
        "{what}: every use of the batches was counted into a submit"
    );
}

/// The crowded frame's way: one batch of many draws in few passes, its encoders cut every 256
/// draws (each with few uses) and ALL of them sent in ONE submit, in order. 640 draws in one
/// occlusion query, their triangles uploaded before the batch (no upload between draws: only the
/// colour changes), each nearer than every draw before it under a LESS depth test: the query
/// counts every draw only if each encoder's draws ran after the encoder's before. The last draw's
/// colour is left, the batch went in exactly one submit, and that submit got two or more command
/// buffers, all that the batch's encoders made.
#[test]
fn a_crowded_batch_s_encoders_go_in_one_submit_in_order() {
    const DRAWS: u32 = 640;
    const LADDER: u32 = 0x3100;
    let Some(mut g) = Gpu::with_threads(4, 1, true) else {
        return;
    };
    let d = g.b.device().expect("the device");
    assert!(
        d.has_queue_thread(),
        "the queue thread (and finishing threads) on"
    );
    let tris: Vec<f32> = (0..DRAWS)
        .flat_map(|i| {
            let z = 1.0 - (i + 1) as f32 / 1024.0;
            [-3.0, -3.0, z, 1.0, -3.0, 9.0, z, 1.0, 9.0, -3.0, z, 1.0]
        })
        .collect();
    let bytes = floats(&tris);
    g.run(Cmd::CreateBuffer {
        device: DEVICE,
        buffer: LADDER,
        index: false,
        length: bytes.len() as u32,
        usage: 0,
        format: 0,
        pool: 1,
    });
    g.run(Cmd::UploadBuffer {
        buffer: LADDER,
        offset: 0,
        flags: 0,
        data: Data::Bytes(&bytes),
    });
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: LADDER,
        offset: 0,
        stride: 16,
    });
    g.run(Cmd::CreateQuery {
        device: DEVICE,
        query: 0x2000,
        kind: 9,
    });
    // LESS.
    g.state(ZFUNC, 2);
    // The set-up sent first (a read-back sends the batch), so the batch below is the draws'.
    let _ = g.pixels();
    let before = g.b.device().expect("the device").submits();
    g.clear(3, 0, 1.0);
    let count = g.count(0x2000, |g| {
        for i in 0..DRAWS {
            g.colour(if i == DRAWS - 1 {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            });
            g.run(Cmd::DrawPrimitive {
                device: DEVICE,
                kind: 4,
                start: 3 * i,
                count: 1,
            });
        }
    });
    let after = g.b.device().expect("the device").submits();
    assert_eq!(
        count,
        DRAWS * 16,
        "every draw nearer than the ones before passed"
    );
    assert_eq!(
        after.submits,
        before.submits + 1,
        "the batch went in one submit: {before:?} -> {after:?}"
    );
    // An ordinary batch never reaches the limit, so it never pays the early send's wait.
    assert_eq!(
        after.bound_sends, before.bound_sends,
        "no early send for a batch of this size: {after:?}"
    );
    assert!(
        after.most_buffers >= 2,
        "the submit got the batch's encoders, two or more: {after:?}"
    );
    assert_eq!(
        after.encoders - before.encoders,
        after.buffers - before.buffers,
        "every encoder of the batch was submitted"
    );
    check_submits(&after, "the crowded batch");
    for p in g.pixels() {
        assert_eq!(p, [0, 255, 0, 255], "the last draw's green");
    }
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

/// The early send comes exactly at the limit: a batch of draws that each upload their vertices (a
/// copy and a pass each, two uses) just under [`d3d9_wgpu::device::SUBMIT_USES`] goes in one
/// submit with no early send; one just over it makes exactly one early send.
#[test]
fn a_batch_just_under_the_limit_is_sent_whole_and_one_just_over_once_early() {
    let Some(mut g) = Gpu::new(4, 1) else {
        return;
    };
    let limit = d3d9_wgpu::device::SUBMIT_USES;
    for (draws, early) in [(limit / 2 - 16, 0), (limit / 2 + 4, 1)] {
        // What was recorded before sent first, so the batch below is the draws' alone.
        let _ = g.pixels();
        let before = g.b.device().expect("the device").submits();
        g.clear(3, 0, 1.0);
        for _ in 0..draws {
            g.cover(0.5);
        }
        let _ = g.pixels();
        let after = g.b.device().expect("the device").submits();
        let uses = after.batch_uses - before.batch_uses;
        assert!(
            if early == 0 {
                uses < u64::from(limit)
            } else {
                uses > u64::from(limit)
            },
            "{draws} draws: {uses} uses"
        );
        assert_eq!(
            after.bound_sends - before.bound_sends,
            early,
            "{draws} draws ({uses} uses): early sends"
        );
        // Under: one submit. Over: the early batch's (one, or two when its last command's pass went
        // in an encoder of its own past the limit) and the rest's.
        let submits = after.submits - before.submits;
        assert!(
            if early == 0 {
                submits == 1
            } else {
                (2..=3).contains(&submits)
            },
            "{draws} draws ({uses} uses): {submits} submits"
        );
        check_submits(&after, &format!("{draws} draws"));
    }
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

/// The card's times on (`NV_WGPU_GPU_TIMES`) send and cut the batches exactly as off: their
/// timestamps and resolve are no uses of the encoders, so no batch goes early
/// ([`d3d9_wgpu::device::SUBMIT_USES`]) and no encoder is cut or added. With the finishing and
/// queue threads, four batches: 300 draws from one buffer (one pass, its encoder cut at 256
/// draws: two passes in two encoders); 256 draws and then an upload into the target, which sends
/// the batch with its last encoder already at a finishing thread (no encoder open: its timestamps
/// are resolved with the next batch's, none added); then draws that each upload their vertices (a copy and a
/// pass each, the copy timed between passes where the driver can) just under the limit (no early
/// send) and just over it (one). Each batch's submits, early sends, encoders and uses are the same
/// with the times on as off. On, where the driver gives times: every pair read back in order, no
/// batch untimed, each draw's pass timed as world (a pass that began with the clear too, not as
/// clears), and the copies where the driver writes timestamps between passes.
#[test]
fn the_gpu_times_on_send_and_cut_the_batches_as_off() {
    use d3d9_wgpu::gpu_times::{Kind, KINDS};
    const DRAWS: u32 = 300;
    const LADDER: u32 = 0x3200;
    let limit = d3d9_wgpu::device::SUBMIT_USES;
    let uploads = [limit / 2 - 16, limit / 2 + 4];
    // Each batch's submits, early sends, encoders and uses.
    let run = |g: &mut Gpu| -> Vec<[u64; 4]> {
        // One pass and draw each, as the counts below take them: the held draws' merging
        // (`NV_WGPU_MERGE_DRAWS`, which a run of the whole suite may set) off.
        if let Some(d) = g.b.device() {
            d.set_merge_draws(false, false, false);
        }
        let tris: Vec<f32> = (0..DRAWS)
            .flat_map(|_| {
                [
                    -3.0, -3.0, 0.5, 1.0, -3.0, 9.0, 0.5, 1.0, 9.0, -3.0, 0.5, 1.0,
                ]
            })
            .collect();
        let bytes = floats(&tris);
        g.run(Cmd::CreateBuffer {
            device: DEVICE,
            buffer: LADDER,
            index: false,
            length: bytes.len() as u32,
            usage: 0,
            format: 0,
            pool: 1,
        });
        g.run(Cmd::UploadBuffer {
            buffer: LADDER,
            offset: 0,
            flags: 0,
            data: Data::Bytes(&bytes),
        });
        g.run(Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: LADDER,
            offset: 0,
            stride: 16,
        });
        let mut out = Vec::new();
        let mut batch = |g: &mut Gpu, draw_all: &dyn Fn(&mut Gpu)| {
            // What was recorded before sent first, so the batch is the draws' alone.
            let _ = g.pixels();
            let before = g.b.device().expect("the device").submits();
            g.clear(3, 0, 1.0);
            draw_all(g);
            let _ = g.pixels();
            let after = g.b.device().expect("the device").submits();
            check_submits(&after, "a batch");
            out.push([
                after.submits - before.submits,
                after.bound_sends - before.bound_sends,
                after.encoders - before.encoders,
                after.batch_uses - before.batch_uses,
            ]);
        };
        batch(g, &|g: &mut Gpu| {
            for i in 0..DRAWS {
                draw(g, 3 * i, 1);
            }
        });
        batch(g, &|g: &mut Gpu| {
            for i in 0..256 {
                draw(g, 3 * i, 1);
            }
            g.run(Cmd::UploadSurface {
                surface: RT,
                row: 16,
                rows: 4,
                data: Data::Bytes(&[0x40u8; 64][..]),
            });
        });
        for n in uploads {
            batch(g, &|g: &mut Gpu| {
                for _ in 0..n {
                    g.cover(0.5);
                }
            });
        }
        out
    };
    let times = [("NV_WGPU_GPU_TIMES", "1")];
    let off = {
        let Some(mut g) = Gpu::make_with(4, 1, None, &[], true, true) else {
            return;
        };
        run(&mut g)
    };
    assert!(off[0][2] >= 2, "the 300 draws' encoder was cut: {off:?}");
    assert_eq!(
        (off[2][1], off[3][1]),
        (0, 1),
        "no early send just under the limit, one just over: {off:?}"
    );
    let Some(mut g) = Gpu::make_with(4, 1, None, &times, true, true) else {
        return;
    };
    let d = g.b.device().expect("the device");
    if !d.adapter_has_timestamps() {
        assert!(!d.gpu_times_on());
        eprintln!("this adapter cannot time passes: skipped");
        return;
    }
    assert!(d.gpu_times_on(), "asked for, and the adapter can");
    let inside = d.timestamp_features().1;
    let on = run(&mut g);
    assert_eq!(
        on, off,
        "each batch's submits, early sends, encoders and uses as with the times off"
    );
    let slot = |k: Kind| KINDS.iter().position(|(x, _)| *x == k).expect("a kind");
    // Each draw of the upload batches in a pass of its own, the 300 draws' two passes and the 256
    // draws' one.
    let passes = u64::from(uploads[0] + uploads[1]) + 3;
    let start = std::time::Instant::now();
    let t = loop {
        let t = g.b.device().and_then(|d| d.gpu_time_totals()).expect("on");
        if t.spans[slot(Kind::World)] >= passes || t.zeros > 0 || start.elapsed().as_secs() >= 5 {
            break t;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
    if t.zeros > 0 {
        // Known only of AMD's driver (raw wgpu too), as in the test above.
        let name = d.adapter_name().to_string();
        assert!(
            name.contains("AMD") || name.contains("Radeon"),
            "zero timestamps on {name}, not known to give them: {t:?}"
        );
        eprintln!("{name} reads timestamps as zero: the times are not shown: {t:?}");
        return;
    }
    assert_eq!(t.left_out, 0, "every pair in order: {t:?}");
    assert_eq!(t.untimed, 0, "{t:?}");
    assert_eq!(
        t.spans[slot(Kind::World)],
        passes,
        "each draw's pass as world: {t:?}"
    );
    // Each batch's clear is the load of the pass its first draw goes in (counted as world above),
    // so no pass here only clears.
    assert_eq!(t.spans[slot(Kind::Clears)], 0, "{t:?}");
    if inside {
        assert!(
            t.spans[slot(Kind::Copies)] >= u64::from(uploads[0] + uploads[1]),
            "each upload among the draws timed: {t:?}"
        );
    }
}

/// The vertex buffer of [`dynamic_state_scene`]: every shape in it once, uploaded before the
/// first draw, so that no upload falls between the draws (an upload can end the render pass, and
/// with it what the command buffer is known to have set). Each shape's first vertex.
struct Shapes {
    data: Vec<f32>,
}

impl Shapes {
    /// Clip-space vertices (x, y, z; w 1); the first vertex's number.
    fn add(&mut self, v: &[[f32; 3]]) -> u32 {
        let first = (self.data.len() / 4) as u32;
        self.data
            .extend(v.iter().flat_map(|p| [p[0], p[1], p[2], 1.0]));
        first
    }

    /// Clip x from `x0` to `x1` over the viewport's height, at depth `z0` on the left and `z1` on
    /// the right: two triangles wound clockwise on screen (Direct3D 9's front faces), or
    /// counter-clockwise.
    fn band(&mut self, x0: f32, x1: f32, z0: f32, z1: f32, clockwise: bool) -> u32 {
        let (a, b, c) = ([x0, -1.0, z0], [x0, 1.0, z0], [x1, -1.0, z1]);
        let (d, e, f) = ([x1, -1.0, z1], [x0, 1.0, z0], [x1, 1.0, z1]);
        if clockwise {
            self.add(&[a, b, c, d, e, f])
        } else {
            self.add(&[a, c, b, d, f, e])
        }
    }
}

/// `count` triangles from vertex `first` of the scene's buffer.
fn draw(g: &mut Gpu, first: u32, count: u32) {
    g.run(Cmd::DrawPrimitive {
        device: DEVICE,
        kind: 4,
        start: first,
        count,
    });
}

/// The left and top pixel of cell `i` of a 32 by 32 target cut into 8 by 8 cells, row by row.
fn cell(i: u32) -> (u32, u32) {
    ((i % 4) * 8, (i / 4) * 8)
}

/// What [`dynamic_state_scene`] leaves at a pixel: x, y, the B G R A bytes, and why.
type Want = (u32, u32, [u8; 4], &'static str);

/// Each state a linked pipeline leaves dynamic (vendor/wgpu-hal, pipeline_library.rs: cull mode,
/// front face, the depth test, write and compare, one- and two-sided stencil, constant and
/// slope-scaled depth bias), changed between draws in one command buffer, each in its own cell of a
/// 32 by 32 target; what Direct3D 9 draws there, left and right of the cell's middle. Where a draw's
/// result depends on a state the draw before it set differently, nothing but render states and
/// the viewport changes between them (no upload, no new colour), so that the change is made within
/// one render pass, where a bind sets only the states that changed.
fn dynamic_state_scene(g: &mut Gpu) -> Vec<Want> {
    const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
    const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
    const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
    let (red, green, blue, none) = (
        [0, 0, 255, 255],
        [0, 255, 0, 255],
        [255, 0, 0, 255],
        [0, 0, 0, 0],
    );
    let mut s = Shapes { data: Vec::new() };
    let cw_tri = [[-3.0, -3.0, 0.5], [-3.0, 9.0, 0.5], [9.0, -3.0, 0.5]];
    let cw = s.add(&cw_tri);
    let ccw = s.add(&[cw_tri[0], cw_tri[2], cw_tri[1]]);
    let full = s.band(-1.0, 1.0, 0.5, 0.5, true);
    let left = s.band(-1.0, 0.0, 0.5, 0.5, true);
    let left_ccw = s.band(-1.0, 0.0, 0.5, 0.5, false);
    let right = s.band(0.0, 1.0, 0.5, 0.5, true);
    let far = s.band(-1.0, 1.0, 0.75, 0.75, true);
    let sloped = s.band(-1.0, 1.0, 0.25, 0.75, true);
    let bytes = floats(&s.data);
    assert!(bytes.len() <= 48 * 64, "the scene's shapes fit the buffer");
    g.run(Cmd::UploadBuffer {
        buffer: VB,
        offset: 0,
        flags: 0,
        data: Data::Bytes(&bytes),
    });
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: VB,
        offset: 0,
        stride: 16,
    });
    let at = |g: &mut Gpu, i: u32| {
        let (x, y) = cell(i);
        g.viewport(x, y, 8, 8, 0.0, 1.0);
    };
    let mut want = Vec::new();
    let mut expect = |i: u32, left: [u8; 4], right: [u8; 4], what: &'static str| {
        let (x, y) = cell(i);
        want.push((x + 2, y + 4, left, what));
        want.push((x + 6, y + 4, right, what));
    };
    g.viewport(0, 0, 32, 32, 0.0, 1.0);
    g.clear(7, 0, 1.0);

    // Cull mode and front face: CULLMODE 2 culls triangles clockwise on screen, 3
    // counter-clockwise ones. One colour: only the cull mode and the viewport change.
    g.state(ZENABLE, 0);
    g.colour(GREEN);
    for (i, cull, tri) in [(0, 2, cw), (1, 2, ccw), (2, 3, cw), (3, 3, ccw)] {
        g.state(CULLMODE, cull);
        at(g, i);
        draw(g, tri, 1);
    }
    expect(0, none, none, "CULLMODE 2 culls a clockwise triangle");
    expect(1, green, green, "CULLMODE 2 keeps a counter-clockwise one");
    expect(2, green, green, "CULLMODE 3 keeps a clockwise triangle");
    expect(3, none, none, "CULLMODE 3 culls a counter-clockwise one");
    g.state(CULLMODE, 1);

    // Depth writes switched on alone (the same ZFUNC, the same colour): the third draw
    // (GREATER, further away) passes only where the second wrote its depth.
    g.state(ZENABLE, 1);
    g.state(ZFUNC, 4);
    at(g, 4);
    g.colour(BLUE);
    g.state(ZWRITEENABLE, 0);
    draw(g, full, 2);
    g.state(ZWRITEENABLE, 1);
    draw(g, left, 2);
    g.state(ZFUNC, 5);
    g.state(ZWRITEENABLE, 0);
    g.colour(GREEN);
    draw(g, far, 2);
    expect(4, green, blue, "ZWRITEENABLE 1 after 0 writes depth");
    g.state(ZFUNC, 4);
    g.state(ZWRITEENABLE, 1);
    g.state(ZENABLE, 0);

    // One-sided stencil: REPLACE with 1 on the left, then EQUAL 1 over the cell (one colour).
    at(g, 5);
    g.state(STENCILENABLE, 1);
    g.state(STENCILMASK, 0xFF);
    g.state(STENCILWRITEMASK, 0xFF);
    g.state(STENCILFAIL, 1);
    g.state(STENCILZFAIL, 1);
    g.state(STENCILFUNC, 8);
    g.state(STENCILPASS, 3);
    g.state(STENCILREF, 1);
    g.colour(RED);
    draw(g, left, 2);
    g.state(STENCILFUNC, 3);
    g.state(STENCILPASS, 1);
    g.state(STENCILREF, 1);
    draw(g, right, 2);
    g.colour(GREEN);
    draw(g, full, 2);
    expect(
        5,
        green,
        none,
        "STENCILFUNC EQUAL passes where REPLACE wrote, nowhere else",
    );

    // Two-sided stencil: counter-clockwise faces REPLACE with 2 (the CCW_ states), clockwise ones
    // KEEP 0; then EQUAL 2, one-sided, over the cell.
    at(g, 6);
    g.state(TWOSIDEDSTENCILMODE, 1);
    g.state(STENCILFUNC, 8);
    g.state(STENCILPASS, 1);
    g.state(CCW_STENCILFUNC, 8);
    g.state(CCW_STENCILFAIL, 1);
    g.state(CCW_STENCILZFAIL, 1);
    g.state(CCW_STENCILPASS, 3);
    g.state(STENCILREF, 2);
    g.colour(RED);
    draw(g, left_ccw, 2);
    draw(g, right, 2);
    g.state(TWOSIDEDSTENCILMODE, 0);
    g.state(STENCILFUNC, 3);
    g.colour(GREEN);
    draw(g, full, 2);
    expect(
        6,
        green,
        red,
        "two-sided stencil: the counter-clockwise faces' REPLACE",
    );
    g.state(STENCILENABLE, 0);

    // Depth bias: a coplanar draw biased nearer passes LESS against the first.
    g.state(ZENABLE, 1);
    at(g, 7);
    g.colour(RED);
    draw(g, full, 2);
    g.colour(GREEN);
    g.state(ZFUNC, 2);
    g.state(DEPTHBIAS, (-0.001f32).to_bits());
    draw(g, full, 2);
    g.state(DEPTHBIAS, 0);
    expect(7, green, green, "DEPTHBIAS brings a coplanar draw in front");

    // Slope-scaled depth bias, on a band whose depth runs from 0.25 to 0.75 across the cell.
    g.state(ZFUNC, 4);
    at(g, 8);
    g.colour(RED);
    draw(g, sloped, 2);
    g.colour(GREEN);
    g.state(ZFUNC, 2);
    g.state(SLOPESCALEDEPTHBIAS, (-2.0f32).to_bits());
    draw(g, sloped, 2);
    g.state(SLOPESCALEDEPTHBIAS, 0);
    g.state(ZFUNC, 4);
    expect(
        8,
        green,
        green,
        "SLOPESCALEDEPTHBIAS brings a coplanar sloped draw in front",
    );
    want
}

/// The scene's pixels as [`dynamic_state_scene`] says Direct3D 9 draws them, and no wgpu error.
fn check_scene(g: &mut Gpu, want: &[Want]) {
    let px = g.pixels();
    for &(x, y, p, what) in want {
        assert_eq!(px[(y * 32 + x) as usize], p, "{what} (pixel {x}, {y})");
    }
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

/// Whether this machine's device links pipelines from libraries; if not, says the test is skipped.
fn has_libraries(g: &mut Gpu) -> bool {
    let has = g.b.device().is_some_and(|d| d.uses_pipeline_libraries());
    if !has {
        eprintln!("no pipeline libraries on this device (Vulkan with VK_EXT_graphics_pipeline_library): skipped");
    }
    has
}

/// The states a linked pipeline leaves dynamic reach its draws as a whole pipeline's baked ones
/// do, in one command buffer where each draw changes one of them: cull mode and front face (the
/// world inside out if the front face were wrong), a change of the depth write alone, stencil one-
/// and two-sided, constant and slope-scaled depth bias. Every pipeline is linked.
#[test]
fn linked_pipelines_draw_cull_depth_stencil_and_bias_as_direct3d_9_does() {
    let Some(mut g) = Gpu::new(32, 1) else { return };
    if !has_libraries(&mut g) {
        return;
    }
    let want = dynamic_state_scene(&mut g);
    check_scene(&mut g, &want);
    let report = g.b.device().and_then(|d| d.pipeline_library_report());
    let report = report.expect("a report where pipelines are linked");
    assert!(report.contains("made whole: 0"), "{report}");
}

/// `NV_WGPU_LIBRARIES=off` on a device that links from libraries: no libraries (no report), and
/// the same scene drawn by whole pipelines as Direct3D 9 draws it.
#[test]
fn the_libraries_switch_off_makes_every_pipeline_whole() {
    {
        let Some(mut g) = Gpu::new(32, 1) else { return };
        if !has_libraries(&mut g) {
            return;
        }
    }
    let Some(mut g) = Gpu::with_env(32, 1, &[("NV_WGPU_LIBRARIES", "off")]) else {
        return;
    };
    let d = g.b.device().expect("the device");
    assert!(!d.uses_pipeline_libraries(), "switched off");
    assert_eq!(d.pipeline_library_report(), None);
    let want = dynamic_state_scene(&mut g);
    check_scene(&mut g, &want);
}

/// A library the driver refuses leaves its pipelines to the whole path on a device that links
/// (the test hook `NV_WGPU_LIBRARIES_TEST_REFUSE` refuses every pixel shader library): every
/// pipeline made whole, with the independent-sets layout the linked ones are bound with, and the
/// scene's pixels exactly the linked run's.
#[test]
fn a_refused_library_leaves_its_pipelines_whole_with_the_same_pixels() {
    let linked = {
        let Some(mut g) = Gpu::new(32, 1) else { return };
        if !has_libraries(&mut g) {
            return;
        }
        dynamic_state_scene(&mut g);
        g.pixels()
    };
    let refuse = [("NV_WGPU_LIBRARIES_TEST_REFUSE", "pixel-shader")];
    let Some(mut g) = Gpu::with_env(32, 1, &refuse) else {
        return;
    };
    let want = dynamic_state_scene(&mut g);
    check_scene(&mut g, &want);
    let report = g.b.device().and_then(|d| d.pipeline_library_report());
    let report = report.expect("a report: the device still has libraries");
    assert!(
        report.contains("pipelines linked from libraries that existed: 0 ")
            && report.contains("after making a library: 0 ")
            && !report.contains("made whole: 0"),
        "every pipeline whole: {report}"
    );
    assert_eq!(
        g.pixels(),
        linked,
        "the whole path draws the linked path's pixels"
    );
}

/// A shader's header (the registers but the float constants, written only when it changes) is
/// written again between two draws of one batch when only a header value changed, with no
/// read-back between the draws: the pixel shader's alpha reference, the vertex shader's clip
/// planes, and a draw after only the other stage's header changed (the unchanged one kept).
#[test]
fn a_header_change_between_two_draws_of_one_batch_reaches_the_second() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    // The pixel shader's header: alpha 0.5 is GREATER than 0x40, not than 0xC0. The halves have
    // the same viewport size, so the vertex shader's header is the same for both.
    g.colour([1.0, 1.0, 1.0, 0.5]);
    g.clear(3, 0, 1.0);
    g.state(ALPHATESTENABLE, 1);
    g.state(ALPHAFUNC, 5);
    g.state(ALPHAREF, 0x40);
    g.viewport(0, 0, 2, 4, 0.0, 1.0);
    g.cover(0.5);
    g.state(ALPHAREF, 0xC0);
    g.viewport(2, 0, 2, 4, 0.0, 1.0);
    g.cover(0.5);
    assert_eq!(
        g.reds(),
        [255, 255, 0, 0].repeat(4),
        "the alpha reference changed between the draws"
    );
    // The vertex shader's header: plane 0 on for the second draw only (kept where x - 0.25 w is
    // at least 0: the last column), red under green.
    g.state(ALPHATESTENABLE, 0);
    g.viewport(0, 0, 4, 4, 0.0, 1.0);
    g.run(Cmd::SetClipPlane {
        device: DEVICE,
        index: 0,
        plane: [1.0f32, 0.0, 0.0, -0.25].map(f32::to_bits),
    });
    g.clear(3, 0, 1.0);
    g.colour([1.0, 0.0, 0.0, 1.0]);
    g.cover(0.5);
    g.state(CLIPPLANEENABLE, 1);
    g.colour([0.0, 1.0, 0.0, 1.0]);
    g.cover(0.25);
    assert_eq!(
        g.reds(),
        [255, 255, 255, 0].repeat(4),
        "the clip planes changed between the draws"
    );
    // Only the vertex shader's header changes (the planes) between two draws the pixel
    // shader's alpha test refuses: its header stays, and nothing is drawn.
    g.state(CLIPPLANEENABLE, 0);
    g.clear(3, 0, 1.0);
    g.state(ALPHATESTENABLE, 1);
    g.state(ALPHAREF, 0xC0);
    g.colour([1.0, 1.0, 1.0, 0.5]);
    g.cover(0.5);
    g.state(CLIPPLANEENABLE, 1);
    g.cover(0.25);
    assert_eq!(
        g.reds(),
        vec![0; 16],
        "the pixel shader's header kept while the planes changed"
    );
}

/// The queue thread held this many milliseconds on each batch while it lives (tests that hand a
/// batch over and make queue operations behind it), none again after.
struct Slow;

impl Slow {
    fn hold(ms: u32) -> Slow {
        d3d9_wgpu::device::submit_delay_for_tests(ms);
        Slow
    }
}

impl Drop for Slow {
    fn drop(&mut self) {
        d3d9_wgpu::device::submit_delay_for_tests(0);
    }
}

/// `GetData` with `D3DGETDATA_FLUSH` once around one draw: the batch handed over (to the queue
/// thread, held there; or submitted at once without one), not waited for.
fn hand_over(g: &mut Gpu, query: u32, draw: impl FnOnce(&mut Gpu)) {
    g.run(Cmd::QueryIssue { query, flags: 2 });
    draw(g);
    g.run(Cmd::QueryIssue { query, flags: 1 });
    g.run(Cmd::QueryGetData { query, flags: 1 });
}

/// A batch handed to the queue thread and still held there (the thread made slow): an upload made
/// meanwhile into a buffer the held batch draws from lands after the batch's draws, as when the
/// batch was submitted at once. The left rectangle is uploaded and drawn red; one `GetData` with
/// `D3DGETDATA_FLUSH` hands the batch over without waiting; the right rectangle is uploaded into the
/// same buffer and drawn green. An upload that reached the queue before the held batch's submit
/// would make the red draw read the right rectangle too, leaving the left half black. With the
/// queue thread and without it (fewer than four logical processors).
#[test]
fn an_upload_while_the_queue_thread_holds_a_batch_lands_after_it() {
    for threads in [true, false] {
        let Some(mut g) = Gpu::with_threads(4, 1, threads) else {
            return;
        };
        let _slow = Slow::hold(300);
        const VB2: u32 = 0x1800;
        const QUERY: u32 = 0x2000;
        g.run(Cmd::CreateBuffer {
            device: DEVICE,
            buffer: VB2,
            index: false,
            length: 96,
            usage: 0,
            format: 0,
            pool: 1,
        });
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: QUERY,
            kind: 9,
        });
        let upload = |g: &mut Gpu, data: &[u8]| {
            g.run(Cmd::UploadBuffer {
                buffer: VB2,
                offset: 0,
                flags: 0,
                data: Data::Bytes(data),
            });
        };
        let draw = |g: &mut Gpu| {
            g.run(Cmd::DrawPrimitive {
                device: DEVICE,
                kind: 4,
                start: 0,
                count: 2,
            });
        };
        g.clear(3, 0, 1.0);
        g.run(Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: VB2,
            offset: 0,
            stride: 16,
        });
        upload(&mut g, &floats(&rect(-3.0, -0.25)));
        g.colour([1.0, 0.0, 0.0, 1.0]);
        hand_over(&mut g, QUERY, draw);
        upload(&mut g, &floats(&rect(-0.25, 3.0)));
        g.colour([0.0, 1.0, 0.0, 1.0]);
        draw(&mut g);
        left_red_right_green(&mut g, &format!("threads {threads}"));
        assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
    }
}

/// The same for a texture: a draw sampling it, the batch handed over and held, new texels
/// uploaded into it, a second draw sampling it. The vertex shader samples a 1 by 1 texture
/// (`stage_samplers`): green when the left half is drawn, red uploaded while the batch is held,
/// then the right half drawn. The left half stays green (the old texel), the right is red (the
/// new); an upload that reached the queue before the held batch's submit would turn both red.
#[test]
fn a_texture_upload_while_the_queue_thread_holds_a_batch_lands_after_it() {
    for threads in [true, false] {
        let Some(mut g) = Gpu::with_threads(4, 1, threads) else {
            return;
        };
        stage_samplers(&mut g);
        for (pixel, shader) in [(false, VS + 2), (true, PS + 5)] {
            g.run(Cmd::SetShader {
                device: DEVICE,
                pixel,
                shader,
            });
        }
        const VB2: u32 = 0x1800;
        const QUERY: u32 = 0x2000;
        // The vertex texture's level (`stage_samplers`).
        const VS_TEX_LEVEL: u32 = 0x1601;
        g.run(Cmd::CreateBuffer {
            device: DEVICE,
            buffer: VB2,
            index: false,
            length: 192,
            usage: 0,
            format: 0,
            pool: 1,
        });
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: QUERY,
            kind: 9,
        });
        let both: Vec<f32> = [rect(-3.0, -0.25), rect(-0.25, 3.0)].concat();
        g.run(Cmd::UploadBuffer {
            buffer: VB2,
            offset: 0,
            flags: 0,
            data: Data::Bytes(&floats(&both)),
        });
        g.run(Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: VB2,
            offset: 0,
            stride: 16,
        });
        let draw = |g: &mut Gpu, start: u32| {
            g.run(Cmd::DrawPrimitive {
                device: DEVICE,
                kind: 4,
                start,
                count: 2,
            });
        };
        g.clear(3, 0, 1.0);
        let _slow = Slow::hold(300);
        hand_over(&mut g, QUERY, |g| draw(g, 0));
        g.run(Cmd::UploadSurface {
            surface: VS_TEX_LEVEL,
            row: 4,
            rows: 1,
            data: Data::Bytes(&[0, 0, 255, 255]),
        });
        draw(&mut g, 6);
        let px = g.pixels();
        for (i, p) in px.iter().enumerate() {
            let want = if i % 4 < 2 {
                [0, 255, 0, 255]
            } else {
                [0, 0, 255, 255]
            };
            assert_eq!(*p, want, "threads {threads}: pixel {i} of {px:?}");
        }
        assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
    }
}

/// An occlusion query whose batch is held by the queue thread: `GetData` answers not ready while
/// it is held (it never waits), then the count of the held batch's draw (all 16 samples of the
/// 4 by 4 target, then the left half's 8). With the queue thread and without it.
#[test]
fn an_occlusion_count_of_a_held_batch_is_that_batchs() {
    for threads in [true, false] {
        let Some(mut g) = Gpu::with_threads(4, 1, threads) else {
            return;
        };
        let _slow = Slow::hold(300);
        const QUERY: u32 = 0x2000;
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: QUERY,
            kind: 9,
        });
        g.state(ZFUNC, 8);
        g.clear(3, 0, 1.0);
        assert_eq!(
            g.count(QUERY, |g| g.cover(0.5)),
            16,
            "threads {threads}: the whole target"
        );
        g.clear(3, 0, 1.0);
        let left = g.count(QUERY, |g| {
            g.quad(0.5);
            g.viewport(0, 0, 2, 4, 0.0, 1.0);
            g.quad(0.5);
            g.viewport(0, 0, 4, 4, 0.0, 1.0);
        });
        assert_eq!(
            left,
            16 + 8,
            "threads {threads}: the quad, then its left half"
        );
        assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
    }
}

/// What a panic carried: its message.
fn panic_message(p: Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

/// A failure on the queue thread comes back to the render thread with its message: a panic in a
/// batch's job, and a finishing thread's failure (600 draws, so encoders go to the finishing
/// threads; one upload among them, so the batch is sent whole at the `Present`). Each is planted, the batch handed over by a `Present` (which does not wait for it);
/// then the next wait (`wgpu_errors`) raises it, and once raised the device goes on. Planted again,
/// the next flush (`GetData` with `D3DGETDATA_FLUSH`) raises it after the queue thread has run the
/// batch. Planted again and never raised (no flush or wait after it), the device is dropped
/// without blocking or panicking. A queue thread that has ended: waits, flushes and reads still
/// finish (its jobs run on the render thread).
#[test]
fn a_failure_on_the_queue_thread_is_raised_on_the_render_thread() {
    use d3d9_wgpu::device::{
        plant_queue_failure_for_tests as plant, PLANT_FINISHING, PLANT_PANIC, PLANT_THREAD_ENDS,
    };
    use std::panic::{catch_unwind, AssertUnwindSafe};
    struct Clear;
    impl Drop for Clear {
        fn drop(&mut self) {
            plant(0);
        }
    }
    let _clear = Clear;
    const QUERY: u32 = 0x2000;
    let present = |g: &mut Gpu| {
        g.run(Cmd::Present {
            device: DEVICE,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame: 1,
        });
    };
    for (kind, says) in [
        (PLANT_PANIC, "a planted queue thread panic (tests)"),
        (PLANT_FINISHING, "a planted finishing failure (tests)"),
    ] {
        let Some(mut g) = Gpu::new(4, 1) else { return };
        if g.b.device().is_some_and(|d| !d.has_queue_thread()) {
            eprintln!("no queue thread on this machine: skipped");
            return;
        }
        // Each draw its own, as the batch sizes below take them (`NV_WGPU_MERGE_DRAWS` off).
        if let Some(d) = g.b.device() {
            d.set_merge_draws(false, false, false);
        }
        g.run(Cmd::CreateQuery {
            device: DEVICE,
            query: QUERY,
            kind: 9,
        });
        // One upload, then the same triangle drawn again: the batch stays far under the most one
        // submit takes, so it is handed over by the `Present` and not sent early on the way
        // (`Device::bound_batch`, whose wait would raise the failure in the draws).
        let draws = |g: &mut Gpu| {
            g.cover(0.5);
            for _ in 1..600 {
                g.run(Cmd::DrawPrimitive {
                    device: DEVICE,
                    kind: 4,
                    start: 0,
                    count: 1,
                });
            }
        };
        // At the next wait.
        plant(kind);
        draws(&mut g);
        present(&mut g);
        let raised = catch_unwind(AssertUnwindSafe(|| g.b.device().map(|d| d.wgpu_errors())));
        let why = panic_message(raised.expect_err("raised at the wait"));
        assert!(why.contains(says), "{kind}: {why}");
        // Raised once; the device goes on.
        draws(&mut g);
        present(&mut g);
        assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0), "{kind}");
        // At the next flush, the queue thread done with the failed batch first.
        plant(kind);
        draws(&mut g);
        present(&mut g);
        std::thread::sleep(std::time::Duration::from_millis(500));
        g.run(Cmd::QueryIssue {
            query: QUERY,
            flags: 2,
        });
        g.cover(0.5);
        g.run(Cmd::QueryIssue {
            query: QUERY,
            flags: 1,
        });
        let raised = catch_unwind(AssertUnwindSafe(|| {
            g.run(Cmd::QueryGetData {
                query: QUERY,
                flags: 1,
            })
        }));
        let why = panic_message(raised.expect_err("raised at the flush"));
        assert!(why.contains(says), "{kind}: {why}");
        // Never raised: the device dropped all the same.
        plant(kind);
        draws(&mut g);
        present(&mut g);
        drop(g);
    }
    // The queue thread ended (after the wait's own job): later waits, flushes, presents and
    // reads finish, their jobs run on the render thread.
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.clear(3, 0, 1.0);
    g.cover(0.75);
    plant(PLANT_THREAD_ENDS);
    present(&mut g);
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
    std::thread::sleep(std::time::Duration::from_millis(200));
    g.colour([1.0, 0.0, 0.0, 1.0]);
    g.cover(0.5);
    present(&mut g);
    g.colour([0.0, 1.0, 0.0, 1.0]);
    g.cover(0.25);
    present(&mut g);
    for p in g.pixels() {
        assert_eq!(p, [0, 255, 0, 255], "drawn after the queue thread ended");
    }
    assert_eq!(g.b.device().map(|d| d.wgpu_errors()), Some(0));
}

// The fixed-function draws: the intro movie's tiles (exe 00ec0280, 00ec0460, 00ebff30).

/// `D3DFMT_X8R8G8B8`, `D3DUSAGE_DYNAMIC`, `D3DFVF_XYZRHW | D3DFVF_TEX1`.
const X8R8G8B8: u32 = 22;
const DYNAMIC: u32 = 0x200;
const XYZRHW_TEX1: u32 = 0x104;

impl Gpu {
    fn stage_state(&mut self, stage: u32, kind: u32, value: u32) {
        self.run(Cmd::SetTextureStageState {
            device: DEVICE,
            stage,
            kind,
            value,
        });
    }

    fn sampler_state(&mut self, sampler: u32, kind: u32, value: u32) {
        self.run(Cmd::SetSamplerState {
            device: DEVICE,
            sampler,
            kind,
            value,
        });
    }

    /// No vertex or pixel shader bound: the fixed-function pipeline draws.
    fn no_shaders(&mut self) {
        for pixel in [false, true] {
            self.run(Cmd::SetShader {
                device: DEVICE,
                pixel,
                shader: 0,
            });
        }
    }

    /// A `width` by `height` X8R8G8B8 texture as the movie makes its tiles (dynamic, default
    /// pool) at handle `texture`, holding `bgrx` (rows packed), bound to sampler 0.
    fn movie_texture(&mut self, texture: u32, width: u32, height: u32, bgrx: &[u8]) {
        let levels = words(&[texture + 1]);
        self.run(Cmd::CreateTexture {
            device: DEVICE,
            texture,
            levels: Data::Bytes(&levels),
            width,
            height,
            usage: DYNAMIC,
            format: X8R8G8B8,
            pool: 0,
        });
        self.run(Cmd::UploadSurface {
            surface: texture + 1,
            row: 4 * width,
            rows: height,
            data: Data::Bytes(bgrx),
        });
        self.run(Cmd::SetTexture {
            device: DEVICE,
            sampler: 0,
            texture,
        });
    }

    /// The movie's draw state (00ec0460, then 00ebff30's render states): alpha blending off;
    /// stage 0 alpha SELECTARG1, colour argument 1 TEXTURE, alpha argument 1 TEXTURE, colour
    /// MODULATE (argument 2 CURRENT: the diffuse, white without one); linear min, mag and mip
    /// filters, clamped U and V; depth test and writes off, every colour channel written.
    fn movie_state(&mut self) {
        self.state(27, 0);
        for (state, value) in [(4, 2), (2, 2), (5, 2), (1, 4)] {
            self.stage_state(0, state, value);
        }
        for (state, value) in [(6, 2), (5, 2), (7, 2), (1, 3), (2, 3)] {
            self.sampler_state(0, state, value);
        }
        self.state(ZENABLE, 0);
        self.state(ZWRITEENABLE, 0);
        self.state(168, 0xF);
    }

    /// One movie tile (00ec0280): a strip of four pre-transformed vertices from (x, y) to
    /// (x + w, y + h), texture coordinates 0 to 1, 24 bytes each, drawn with SetFVF(XYZRHW |
    /// TEX1) and DrawPrimitiveUP.
    fn movie_quad(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let mut v = Vec::new();
        for (px, py, u, t) in [
            (x, y, 0.0, 0.0),
            (x, y + h, 0.0, 1.0),
            (x + w, y, 1.0, 0.0),
            (x + w, y + h, 1.0, 1.0),
        ] {
            v.extend(floats(&[px, py, 0.0, 1.0, u, t]));
        }
        self.run(Cmd::SetFvf {
            device: DEVICE,
            fvf: XYZRHW_TEX1,
        });
        self.run(Cmd::DrawPrimitiveUp {
            device: DEVICE,
            kind: 5,
            count: 2,
            stride: 24,
            vertices: Data::Bytes(&v),
        });
    }
}

/// A texture whose texel (x, y) has red 16 x, green 16 y, blue 0x80 and an alpha byte of 0 (an
/// X8 format's: read as opaque).
fn ramp(width: u32, height: u32) -> Vec<u8> {
    let mut b = Vec::new();
    for y in 0..height {
        for x in 0..width {
            b.extend_from_slice(&[0x80, (16 * y) as u8, (16 * x) as u8, 0]);
        }
    }
    b
}

/// What Direct3D 9 shows of [`ramp`] drawn as a quad of exactly its size from whole pixel
/// coordinates with linear filtering: pixel n's centre is at screen n, which is texture
/// coordinate n / size, half-way between texels n - 1 and n (Microsoft's "Directly Mapping Texels
/// to Pixels"): 16 n - 8, and 0 for the first (clamped addressing repeats texel 0).
fn ramp_seen(n: u32) -> u8 {
    (16 * n).saturating_sub(8) as u8
}

fn near(a: u8, b: u8) -> bool {
    a.abs_diff(b) <= 1
}

/// The intro movie's tile draw, call for call as the game makes it (no shaders, SetFVF(XYZRHW
/// | TEX1), DrawPrimitiveUP), shows the uploaded texels where Direct3D 9 puts them: every pixel
/// the half-texel blend of its two texels, blue 0x80, opaque. (The backend skipped every draw
/// without shaders: the movie played to a black screen on Vulkan.)
#[test]
fn the_movies_fixed_function_tile_shows_its_texels_where_direct3d_9_does() {
    let Some(mut g) = Gpu::new(16, 1) else { return };
    g.movie_texture(0x1700, 16, 16, &ramp(16, 16));
    g.no_shaders();
    g.movie_state();
    g.clear(1, 0xFF00_0000, 1.0);
    g.movie_quad(0.0, 0.0, 16.0, 16.0);
    let px = g.pixels();
    for y in 0..16u32 {
        for x in 0..16u32 {
            let p = px[(16 * y + x) as usize];
            assert!(
                near(p[2], ramp_seen(x))
                    && near(p[1], ramp_seen(y))
                    && near(p[0], 0x80)
                    && p[3] == 255,
                "pixel ({x}, {y}): {p:?}, want red {} green {} blue 128 alpha 255",
                ramp_seen(x),
                ramp_seen(y)
            );
        }
    }
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
}

/// The movie's letterbox: a viewport inside the target (x 4, y 2, 8 by 8, as `Movie::Update`
/// sets one) and a tile at the same pixels. Pre-transformed positions are the target's pixels
/// whatever the viewport, so the tile lands at (4, 2) as on Direct3D 9; nothing outside it is
/// drawn.
#[test]
fn a_pre_transformed_tile_lands_on_its_own_pixels_inside_a_smaller_viewport() {
    let Some(mut g) = Gpu::new(16, 1) else { return };
    g.movie_texture(0x1710, 8, 8, &ramp(8, 8));
    g.no_shaders();
    g.movie_state();
    g.clear(1, 0xFF00_0000, 1.0);
    g.viewport(4, 2, 8, 8, 0.0, 1.0);
    g.movie_quad(4.0, 2.0, 8.0, 8.0);
    let px = g.pixels();
    for y in 0..16u32 {
        for x in 0..16u32 {
            let p = px[(16 * y + x) as usize];
            if (4..12).contains(&x) && (2..10).contains(&y) {
                let (u, v) = (x - 4, y - 2);
                assert!(
                    near(p[2], ramp_seen(u)) && near(p[1], ramp_seen(v)) && near(p[0], 0x80),
                    "pixel ({x}, {y}): {p:?}"
                );
            } else {
                assert_eq!(p, [0, 0, 0, 255], "pixel ({x}, {y}) is outside the tile");
            }
        }
    }
}

/// Two texture stages and a vertex colour, by `D3DTEXTUREOP`'s formulas: stage 0 MODULATE
/// (texture, diffuse) for the colour and SELECTARG2 (the diffuse's alpha); stage 1 ADD (current,
/// texture factor) for the colour and SELECTARG1 (current) for the alpha; each stage's result
/// clamped to 1. Texel (R 255, G 128, B 0), diffuse 0x80FF8040, factor 0x00102030: red 255 + 16
/// clamped, green 128 * 128 / 255 + 32, blue 0 + 48, alpha 128.
#[test]
fn texture_stages_combine_as_d3dtextureop_says() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.movie_texture(0x1720, 1, 1, &[0, 128, 255, 0]);
    g.no_shaders();
    g.state(27, 0);
    g.state(ZENABLE, 0);
    const TFACTOR: u32 = 60;
    g.state(TFACTOR, 0x0010_2030);
    // D3DTSS_COLOROP 1, COLORARG1 2, COLORARG2 3, ALPHAOP 4, ALPHAARG1 5, ALPHAARG2 6; D3DTA
    // DIFFUSE 0, CURRENT 1, TEXTURE 2, TFACTOR 3.
    for (stage, state, value) in [
        (0, 1, 4),
        (0, 2, 2),
        (0, 3, 0),
        (0, 4, 3),
        (0, 6, 0),
        (1, 1, 7),
        (1, 2, 1),
        (1, 3, 3),
        (1, 4, 2),
        (1, 5, 1),
    ] {
        g.stage_state(stage, state, value);
    }
    // XYZRHW | DIFFUSE | TEX1, 28 bytes: a quad over the 4 by 4 target.
    let mut v = Vec::new();
    for (x, y) in [(0.0f32, 0.0f32), (0.0, 4.0), (4.0, 0.0), (4.0, 4.0)] {
        v.extend(floats(&[x, y, 0.5, 1.0]));
        v.extend(0x80FF_8040u32.to_le_bytes());
        v.extend(floats(&[0.5, 0.5]));
    }
    g.clear(1, 0, 1.0);
    g.run(Cmd::SetFvf {
        device: DEVICE,
        fvf: 0x144,
    });
    g.run(Cmd::DrawPrimitiveUp {
        device: DEVICE,
        kind: 5,
        count: 2,
        stride: 28,
        vertices: Data::Bytes(&v),
    });
    for p in g.pixels() {
        assert!(
            p[2] == 255 && near(p[1], 96) && near(p[0], 48) && near(p[3], 128),
            "{p:?}: want B 48, G 96, R 255, A 128"
        );
    }
}

/// The movie's fixed-function shaders and pipeline are made before play like the game's: the
/// first run makes its pipeline on the tile's draw and lists it with the shaders' bytecode (`f1`
/// lines: the game creates no such shader); the next run makes the shaders at its start and the
/// pipeline at its first `Present`, so the tile's draw makes none, with the same pixels.
#[test]
fn the_movies_pipeline_is_made_before_its_first_frame_from_the_second_start() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("ffp-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let run = |cache: &std::path::Path| -> Option<(Vec<u8>, u64)> {
        let mut g = Gpu::with_cache(16, 1, Some(cache.to_path_buf()))?;
        g.run(Cmd::Present {
            device: DEVICE,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame: 1,
        });
        g.movie_texture(0x1740, 16, 16, &ramp(16, 16));
        g.no_shaders();
        g.movie_state();
        g.clear(1, 0xFF00_0000, 1.0);
        g.movie_quad(0.0, 0.0, 16.0, 16.0);
        let reds = g.reds();
        let made = g.b.device().expect("the device").pipelines_made_on_draws();
        Some((reds, made))
    };
    let Some((first, made_first)) = run(&dir) else {
        return;
    };
    assert_eq!(
        made_first, 1,
        "the first run makes the tile's pipeline on its draw"
    );
    let list = std::fs::read_to_string(dir.join("pipelines.txt")).expect("the list written");
    assert!(
        list.lines().filter(|l| l.starts_with("f1 ")).count() >= 2,
        "the fixed-function vertex and pixel shaders listed"
    );
    let (second, made_second) = run(&dir).unwrap();
    assert_eq!(second, first, "the same pixels");
    assert_eq!(made_second, 0, "no pipeline made on the tile's draw");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `Reset` returns the texture stage states to Direct3D 9's defaults (stage 0: MODULATE the
/// texture by the diffuse, SELECTARG1 its alpha): a stage 0 set to SELECTARG2 (the diffuse, white
/// here) before the Reset draws the tile's texels after it.
#[test]
fn reset_returns_the_texture_stages_to_their_defaults() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.movie_texture(0x1750, 1, 1, &[0x40, 0x80, 0xC0, 0]);
    g.no_shaders();
    g.movie_state();
    g.stage_state(0, 1, 3);
    g.clear(1, 0, 1.0);
    g.movie_quad(0.0, 0.0, 4.0, 4.0);
    assert!(
        g.pixels().iter().all(|p| *p == [255, 255, 255, 255]),
        "SELECTARG2: the diffuse, white: {:?}",
        g.pixels()
    );
    let present = words(&[4, 4, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
    g.run(Cmd::Reset {
        device: DEVICE,
        window: 0,
        present: Data::Bytes(&present),
        back_buffers: Data::Bytes(&words(&[BACK])),
        depth_stencil: 0,
    });
    // Everything the draw needs again, but the stage states.
    g.run(Cmd::SetRenderTarget {
        device: DEVICE,
        index: 0,
        surface: RT,
    });
    g.run(Cmd::SetTexture {
        device: DEVICE,
        sampler: 0,
        texture: 0x1750,
    });
    g.state(ZENABLE, 0);
    g.state(CULLMODE, 1);
    g.clear(1, 0, 1.0);
    g.movie_quad(0.0, 0.0, 4.0, 4.0);
    assert!(
        g.pixels().iter().all(|p| *p == [0x40, 0x80, 0xC0, 255]),
        "the default stages: the texel: {:?}",
        g.pixels()
    );
}

/// A shader draw after the movie's tile reads the constants the game set before the tile: the
/// game's renderer sends only constants that changed, so the first shader draw after the movie
/// can read registers set long before it. The fixed-function shaders' own constants must not
/// stand in for them (the stage's block is written again on leaving fixed function).
#[test]
fn a_shader_draw_after_the_tile_reads_the_constants_set_before_it() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.colour([0.0, 1.0, 0.0, 1.0]);
    g.clear(3, 0, 1.0);
    g.cover(0.5);
    assert!(g.pixels().iter().all(|p| *p == [0, 255, 0, 255]), "green");
    g.movie_texture(0x1760, 1, 1, &[0, 0, 255, 0]);
    g.no_shaders();
    g.movie_state();
    // No read-back between the tile and the shader draw: one batch, so the draw's register
    // blocks follow the tile's in the same ring (a read-back starts a new one, which writes every
    // block again and would hide a stale one).
    g.movie_quad(0.0, 0.0, 4.0, 4.0);
    for (pixel, shader) in [(false, VS), (true, PS)] {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    g.run(Cmd::SetVertexDeclaration {
        device: DEVICE,
        declaration: DECL,
    });
    // No constant set again: c0 is still the green set before the tile.
    g.cover(0.5);
    assert!(
        g.pixels().iter().all(|p| *p == [0, 255, 0, 255]),
        "green from the constant set before the tile: {:?}",
        g.pixels()
    );
}

/// A vertex declaration set after an FVF replaces it, and the same declaration set again after
/// the FVF is the layout again (the game's 00ec0280 saves the declaration, sets its FVF and sets
/// the declaration back): the shader draw after the movie's tile draws with its own layout.
#[test]
fn a_declaration_set_again_after_an_fvf_is_the_layout_again() {
    let Some(mut g) = Gpu::new(4, 1) else { return };
    g.movie_texture(0x1730, 1, 1, &[0, 0, 255, 0]);
    g.no_shaders();
    g.movie_state();
    g.clear(1, 0, 1.0);
    g.movie_quad(0.0, 0.0, 4.0, 4.0);
    assert!(g.pixels().iter().all(|p| p[2] == 255), "the tile drew red");
    for (pixel, shader) in [(false, VS), (true, PS)] {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    g.run(Cmd::SetVertexDeclaration {
        device: DEVICE,
        declaration: DECL,
    });
    g.colour([0.0, 1.0, 0.0, 1.0]);
    g.cover(0.5);
    assert!(
        g.pixels().iter().all(|p| *p == [0, 255, 0, 255]),
        "the shader draw covered the target in green: {:?}",
        g.pixels()
    );
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
}

/// A depth stand-in (a colour target smaller than the bound depth surface) is kept while a draw
/// uses it and let go once idle for more than 300 presents: on a 256 x 256 depth surface, a 64 x 64
/// target cleared once and a 128 x 128 one cleared every frame give two stand-ins, and after 302
/// presents only the 128 x 128 one is left, for the same depth surface, and it is the stand-in made
/// at the first frame (not one let go and made again).
#[test]
fn idle_depth_stand_ins_go_and_used_ones_stay() {
    let Some(mut g) = Gpu::new(256, 1) else {
        return;
    };
    const SMALL: u32 = 0x3100;
    const MEDIUM: u32 = 0x3110;
    for (surface, size) in [(SMALL, 64), (MEDIUM, 128)] {
        g.run(Cmd::CreateSurface {
            device: DEVICE,
            surface,
            depth: false,
            width: size,
            height: size,
            format: A8R8G8B8,
            multisample: 0,
            quality: 0,
            flag: 0,
        });
    }
    let target = |g: &mut Gpu, surface: u32| {
        g.run(Cmd::SetRenderTarget {
            device: DEVICE,
            index: 0,
            surface,
        });
    };
    let present = |g: &mut Gpu| {
        g.run(Cmd::Present {
            device: DEVICE,
            source: None,
            dest: None,
            window: 0,
            dirty: false,
            model: 0,
            frame: 1,
        });
    };
    target(&mut g, SMALL);
    g.clear(3, 0, 1.0);
    let mut first = Vec::new();
    for frame in 0..302 {
        target(&mut g, MEDIUM);
        g.clear(3, 0, 1.0);
        if frame == 0 {
            first =
                g.b.device()
                    .expect("the device")
                    .depth_stand_ins_for_tests();
        }
        present(&mut g);
    }
    assert_eq!(
        first.iter().map(|k| (k.1, k.2)).collect::<Vec<_>>(),
        [(64, 64), (128, 128)]
    );
    let left =
        g.b.device()
            .expect("the device")
            .depth_stand_ins_for_tests();
    assert_eq!(left.len(), 1, "{left:?}");
    assert_eq!((left[0].1, left[0].2), (128, 128));
    assert_eq!(left[0].0, first[1].0, "the same depth surface");
    assert_eq!(
        left[0].3, first[1].3,
        "the stand-in drawn with every frame is the one made at the first, never let go"
    );
    assert_eq!(g.b.device().expect("the device").wgpu_errors(), 0);
}

/// Each draw's column of the target, every draw's digest and the script's commands as a
/// recording ([`records_script`]).
type Script = (Vec<[u8; 4]>, Vec<u64>, Vec<u8>);

/// One clear, then draws into columns of their own of a 16 by 16 target, with the draw records
/// off or on ([`d3d9_wgpu::device::Device::set_draw_records`]). Between two draws there is no
/// other command than the change named, so the draw after each change finds the last draw's
/// record: a texture changed (only the textures' counter moves), a sampler state changed, the same
/// texture and the same render state set again (the record is used as it is), a render state of
/// the pipeline changed, then the fixed-function pixel shader: a texture stage state changed, and
/// a 2D texture replaced by a cube one on its stage. Returns row 0 (each draw's column) and the
/// digests. The commands are kept as `NV_D3D9_RECORD` writes them.
fn records_script(records: bool) -> Option<Script> {
    records_script_with(records, false)
}

/// [`records_script`] with the registers read from storage or not
/// ([`d3d9_wgpu::device::Device::set_storage_regs`]; None where the adapter cannot).
fn records_script_with(records: bool, storage: bool) -> Option<Script> {
    const PS_TEX: u32 = 0x1602;
    const BLUE_TEX: u32 = 0x1610;
    const CUBE: u32 = 0x1620;
    const COLOROP: u32 = 1;
    let mut g = Gpu::recording(16, 1)?;
    {
        let d = g.b.device().expect("the device");
        d.set_draw_records(records);
        if d.set_storage_regs(storage) != storage {
            return None;
        }
        d.record_draw_digests(true);
    }
    stage_samplers(&mut g);
    let levels = words(&[BLUE_TEX + 1]);
    g.run(Cmd::CreateTexture {
        device: DEVICE,
        texture: BLUE_TEX,
        levels: Data::Bytes(&levels),
        width: 1,
        height: 1,
        usage: 0,
        format: A8R8G8B8,
        pool: 1,
    });
    g.run(Cmd::UploadSurface {
        surface: BLUE_TEX + 1,
        row: 4,
        rows: 1,
        data: Data::Bytes(&[255, 0, 0, 255]),
    });
    // A 1 by 1 cube texture, every face blue.
    let faces: Vec<u32> = (1..=6).map(|k| CUBE + k).collect();
    let face_words = words(&faces);
    g.run(Cmd::CreateCubeTexture {
        device: DEVICE,
        texture: CUBE,
        surfaces: Data::Bytes(&face_words),
        edge: 1,
        usage: 0,
        format: A8R8G8B8,
        pool: 1,
    });
    for &surface in &faces {
        g.run(Cmd::UploadSurface {
            surface,
            row: 4,
            rows: 1,
            data: Data::Bytes(&[255, 0, 0, 255]),
        });
    }
    let shader = |g: &mut Gpu, pixel: bool, shader: u32| {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    };
    let texture = |g: &mut Gpu, texture: u32| {
        g.run(Cmd::SetTexture {
            device: DEVICE,
            sampler: 0,
            texture,
        });
    };
    // Column `i`: clip x of the pixel centres is -1 + i / 8 (Direct3D 9's whole coordinates).
    let mut column = 0u32;
    let mut draw = |g: &mut Gpu| {
        let x = -1.0 + column as f32 / 8.0;
        let tri = floats(&rect(x - 0.0625, x + 0.0625));
        let offset = 96 * column;
        column += 1;
        g.run(Cmd::UploadBuffer {
            buffer: VB,
            offset,
            flags: 0,
            data: Data::Bytes(&tri),
        });
        g.run(Cmd::SetStreamSource {
            device: DEVICE,
            stream: 0,
            buffer: VB,
            offset,
            stride: 16,
        });
        g.run(Cmd::DrawPrimitive {
            device: DEVICE,
            kind: 4,
            start: 0,
            count: 2,
        });
    };
    shader(&mut g, false, VS + 2);
    shader(&mut g, true, PS + 6);
    g.clear(3, 0, 1.0);
    // 0: the vertex texture's green and the pixel texture's red.
    draw(&mut g);
    // 1: only the pixel texture changes: green and blue.
    texture(&mut g, BLUE_TEX);
    draw(&mut g);
    // 2: a sampler state (point magnification): the group changes, the colour does not.
    g.run(Cmd::SetSamplerState {
        device: DEVICE,
        sampler: 0,
        kind: 5,
        value: 1,
    });
    draw(&mut g);
    // 3, 4: the same texture and the same render state set again.
    texture(&mut g, BLUE_TEX);
    draw(&mut g);
    g.state(COLORWRITEENABLE, 0xF);
    draw(&mut g);
    // 5: a render state of the pipeline: red, blue and alpha written, not green.
    g.state(COLORWRITEENABLE, 0xD);
    draw(&mut g);
    // 6: all channels again, the red texture and the shader that draws the vertex texture alone.
    g.state(COLORWRITEENABLE, 0xF);
    texture(&mut g, PS_TEX);
    shader(&mut g, true, PS + 5);
    draw(&mut g);
    // 7: no pixel shader: the fixed-function stage 0 modulates its texture (red) by the diffuse
    // (white without one).
    shader(&mut g, true, 0);
    draw(&mut g);
    // 8: a texture stage state: stage 0 takes argument 2 (the diffuse: white).
    g.stage_state(0, COLOROP, 3);
    draw(&mut g);
    // 9: back to modulate.
    g.stage_state(0, COLOROP, 4);
    draw(&mut g);
    // 10: the cube texture on stage 0, nothing else between: the stage's shader samples a cube.
    texture(&mut g, CUBE);
    draw(&mut g);
    let row = g.pixels()[..16].to_vec();
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
    let digests = d.take_draw_digests();
    Some((row, digests, g.rec.take().expect("the commands kept")))
}

/// The draw records ([`d3d9_wgpu::device::Device::set_draw_records`]) give wgpu what resolving
/// every draw afresh gives it: the same digest at each draw and the same pixels, with every change
/// of [`records_script`] between two draws that would otherwise use one record, and each change
/// is drawn.
#[test]
fn draw_records_give_wgpu_what_each_draw_resolved_afresh_gives() {
    let Some((off_row, off, recording)) = records_script(false) else {
        return;
    };
    let Some((on_row, on, _)) = records_script(true) else {
        return;
    };
    assert_eq!(off.len(), 11, "every draw noted");
    assert_eq!(on, off, "the digests, draw by draw");
    assert_eq!(on_row, off_row, "the pixels, draw by draw");
    let yellow = [0, 255, 255, 255];
    let cyan = [255, 255, 0, 255];
    let blue = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    let red = [0, 0, 255, 255];
    let white = [255, 255, 255, 255];
    let mut want = vec![
        yellow, cyan, cyan, cyan, cyan, blue, green, red, white, red, blue,
    ];
    want.resize(16, [0, 0, 0, 0]);
    assert_eq!(on_row, want);
    // The same script as a recording through the replay the example runs on the game's
    // (`replay_draw_digests`): its draws give the same digests off and on, and the ones above.
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let replay = |records| {
        d3d9_wgpu::replay_draw_digests(&recording, records, 0, u64::MAX, None)
            .expect("the script's recording")
            .expect("a GPU adapter, as above")
            .into_iter()
            .map(|(d, _)| d)
            .collect::<Vec<u64>>()
    };
    assert_eq!(replay(false), off, "the recording replayed, records off");
    assert_eq!(replay(true), off, "the recording replayed, records on");
}

/// The registers read from storage ([`d3d9_wgpu::device::Device::set_storage_regs`]: each draw
/// finds its blocks through its entry in the draw table, its instance number) give wgpu the same
/// draws as the uniform blocks, and the same pixels: the [`records_script`] draws, whose later
/// draws have entries past the first (so this also shows that the backend's instance number
/// counts from the draw's first instance: run with `NV_WGPU_BACKEND=dx12` on Direct3D 12, where
/// the shader's own instance number does not), records off and on, and its recording replayed.
#[test]
fn registers_from_storage_give_wgpu_and_the_picture_what_uniform_blocks_give() {
    let Some((off_row, off, recording)) = records_script_with(true, false) else {
        return;
    };
    let Some((row, digests, _)) = records_script_with(true, true) else {
        eprintln!("this adapter cannot read the registers from storage: not checked");
        return;
    };
    assert_eq!(off.len(), 11, "every draw noted");
    assert_eq!(digests, off, "the digests, draw by draw");
    assert_eq!(row, off_row, "the pixels, draw by draw");
    let Some((row, digests, _)) = records_script_with(false, true) else {
        return;
    };
    assert_eq!(digests, off, "records off: the digests");
    assert_eq!(row, off_row, "records off: the pixels");
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let replayed = d3d9_wgpu::replay_draw_digests_with(&recording, true, true, 0, u64::MAX, None)
        .expect("the script's recording")
        .expect("a GPU adapter, as above")
        .into_iter()
        .map(|(d, _)| d)
        .collect::<Vec<u64>>();
    assert_eq!(replayed, off, "the recording replayed from storage");
}

/// A draw into the A8R8G8B8 target, then with nothing between but `SetRenderTarget` a draw into
/// a 16 by 16 A16B16G16R16F target texture, then with nothing between but a `Clear` of it a third
/// draw, the draw records off or on. The bytes of the half-float target after its first draw and
/// after the clear and its second, and the digests.
fn records_target_script(records: bool) -> Option<(Vec<u8>, Vec<u8>, Vec<u64>)> {
    const HALF: u32 = 0x1630;
    let mut g = Gpu::new(16, 1)?;
    {
        let d = g.b.device().expect("the device");
        d.set_draw_records(records);
        d.record_draw_digests(true);
    }
    target_texture(&mut g, HALF, HALF + 1, 16, A16B16G16R16F);
    g.state(ZENABLE, 0);
    g.clear(3, 0, 1.0);
    g.colour([1.0, 0.0, 0.0, 1.0]);
    g.cover(0.5);
    g.run(Cmd::SetRenderTarget {
        device: DEVICE,
        index: 0,
        surface: HALF + 1,
    });
    g.cover(0.5);
    let read = |g: &mut Gpu| {
        let d = g.b.device().expect("the device");
        d.read_surface_pixels(HALF + 1)
            .expect("the target's pixels")
            .2
    };
    let first = read(&mut g);
    g.clear(1, 0, 1.0);
    g.cover(0.5);
    let second = read(&mut g);
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
    Some((first, second, d.take_draw_digests()))
}

/// The draw records give wgpu what resolving each draw afresh gives it when only the render
/// target changes between two draws (its format is in the pipeline's key) and when only a clear
/// comes between two: the same digests and bytes off and on, and the half-float target red.
#[test]
fn draw_records_follow_a_target_change_and_a_clear_between_two_draws() {
    let Some((off_first, off_second, off)) = records_target_script(false) else {
        return;
    };
    let Some((on_first, on_second, on)) = records_target_script(true) else {
        return;
    };
    assert_eq!(off.len(), 3, "every draw noted");
    assert_eq!(on, off, "the digests, draw by draw");
    assert_eq!(on_first, off_first, "the target after its first draw");
    assert_eq!(
        on_second, off_second,
        "the target after the clear and its second draw"
    );
    // A16B16G16R16F: red 1.0 (0x3C00), green and blue 0, alpha 1.0, at every pixel.
    let red = [0x00, 0x3C, 0, 0, 0, 0, 0x00, 0x3C];
    for bytes in [&on_first, &on_second] {
        assert_eq!(bytes.len(), 16 * 16 * 8);
        assert!(bytes.chunks_exact(8).all(|p| p == red), "{:?}", &bytes[..8]);
    }
}

/// Two draws into a 1 by 1 target texture that also sample it, nothing between them but the
/// second's vertices, the draw records off or on: each adds the vertex texture's red (64) to what
/// it samples, so the second must sample what the first drew (a copy made just before it, not the
/// copy made before the first). The target's bytes and the digests.
fn records_sampled_target_script(records: bool) -> Option<(Vec<u8>, Vec<u64>)> {
    const QUARTER: u32 = 0x1640;
    const LOOP: u32 = 0x1650;
    let mut g = Gpu::new(16, 1)?;
    {
        let d = g.b.device().expect("the device");
        d.set_draw_records(records);
        d.record_draw_digests(true);
    }
    stage_samplers(&mut g);
    // The vertex texture: red 64, the rest 0 (B, G, R, A).
    let levels = words(&[QUARTER + 1]);
    g.run(Cmd::CreateTexture {
        device: DEVICE,
        texture: QUARTER,
        levels: Data::Bytes(&levels),
        width: 1,
        height: 1,
        usage: 0,
        format: A8R8G8B8,
        pool: 1,
    });
    g.run(Cmd::UploadSurface {
        surface: QUARTER + 1,
        row: 4,
        rows: 1,
        data: Data::Bytes(&[0, 0, 64, 0]),
    });
    g.run(Cmd::SetTexture {
        device: DEVICE,
        sampler: 257,
        texture: QUARTER,
    });
    target_texture(&mut g, LOOP, LOOP + 1, 1, A8R8G8B8);
    g.run(Cmd::SetRenderTarget {
        device: DEVICE,
        index: 0,
        surface: LOOP + 1,
    });
    g.run(Cmd::SetDepthStencilSurface {
        device: DEVICE,
        surface: 0,
    });
    g.state(ZENABLE, 0);
    g.clear(1, 0, 1.0);
    for (pixel, shader) in [(false, VS + 2), (true, PS + 6)] {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    g.run(Cmd::SetTexture {
        device: DEVICE,
        sampler: 0,
        texture: LOOP,
    });
    g.cover(0.5);
    g.cover(0.5);
    let d = g.b.device().expect("the device");
    let bytes = d
        .read_surface_pixels(LOOP + 1)
        .expect("the target's pixel")
        .2;
    assert_eq!(d.wgpu_errors(), 0);
    Some((bytes, d.take_draw_digests()))
}

/// The draw records keep no bind group for a draw that samples one of its own colour targets:
/// the second of two draws of one record samples a copy of what the first drew, records off
/// and on (red 64 + 64).
#[test]
fn draw_records_keep_no_groups_for_a_draw_sampling_its_own_target() {
    let Some((off_bytes, off)) = records_sampled_target_script(false) else {
        return;
    };
    let Some((on_bytes, on)) = records_sampled_target_script(true) else {
        return;
    };
    assert_eq!(off.len(), 2, "every draw noted");
    assert_eq!(on, off, "the digests, draw by draw");
    assert_eq!(
        off_bytes,
        [0, 0, 128, 0],
        "records off: the second draw saw the first's red"
    );
    assert_eq!(on_bytes, off_bytes, "records on");
}

/// Four draws of one batch through a vs_3_0 and a ps_3_0 that read every kind of register the
/// device writes, with the registers from storage (`storage`) or in uniform blocks: each draw its
/// own float constants in both stages (a place, a colour picked through `a0`, a pixel colour), its
/// own integer and boolean constants in both stages (a loop count, a branch), a constant the
/// shader defines, its own user clip plane and alpha reference. The pixels of a 4 by 4 target
/// (one column a draw) and the digests. None where the adapter cannot read them from storage.
fn every_register_script(storage: bool) -> Option<(Vec<[u8; 4]>, Vec<u64>)> {
    const ALPHAFUNC_GREATER: u32 = 5;
    let mut g = Gpu::new(4, 1)?;
    {
        let d = g.b.device().expect("the device");
        if d.set_storage_regs(storage) != storage {
            eprintln!("this adapter cannot read the registers from storage: not checked");
            return None;
        }
        d.record_draw_digests(true);
    }
    let f = |x: f32| x.to_bits();
    // vs_3_0: dcl_position v0; dcl_position o0; dcl_texcoord0 o1; def c10, 0.25, 0, 0, 0;
    // mova a0.x, c4.x; add o0, v0, c0; mov r1, c[a0.x + 5]; if b0; add r1.y, r1.y, c10.x; endif;
    // rep i0; add r1.z, r1.z, c10.x; endrep; mov o1, r1.
    let mut vs = vec![0xFFFE_0300];
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(31, &[0x8000_0000, 0xE00F_0000]));
    vs.extend(ins(31, &[0x8000_0005, 0xE00F_0001]));
    vs.extend(ins(81, &[0xA00F_000A, f(0.25), 0, 0, 0]));
    vs.extend(ins(46, &[0xB001_0000, 0xA000_0004]));
    vs.extend(ins(2, &[0xE00F_0000, 0x90E4_0000, 0xA0E4_0000]));
    vs.extend(ins(1, &[0x800F_0001, 0xA0E4_2005, 0xB000_0000]));
    vs.extend(ins(40, &[0xE0E4_0800]));
    vs.extend(ins(2, &[0x8002_0001, 0x8055_0001, 0xA000_000A]));
    vs.extend(ins(43, &[]));
    vs.extend(ins(38, &[0xF0E4_0000]));
    vs.extend(ins(2, &[0x8004_0001, 0x80AA_0001, 0xA000_000A]));
    vs.extend(ins(39, &[]));
    vs.extend(ins(1, &[0xE00F_0001, 0x80E4_0001]));
    vs.push(END);
    // ps_3_0: dcl_texcoord0 v0; def c9, 0.125, 0, 0, 0; add r0, v0, c2; if b1;
    // add r0.x, r0.x, c9.x; endif; rep i1; add r0.y, r0.y, c9.x; endrep; mov oC0, r0.
    let mut ps = vec![0xFFFF_0300];
    ps.extend(ins(31, &[0x8000_0005, 0x900F_0000]));
    ps.extend(ins(81, &[0xA00F_0009, f(0.125), 0, 0, 0]));
    ps.extend(ins(2, &[0x800F_0000, 0x90E4_0000, 0xA0E4_0002]));
    ps.extend(ins(40, &[0xE0E4_0801]));
    ps.extend(ins(2, &[0x8001_0000, 0x8000_0000, 0xA000_0009]));
    ps.extend(ins(43, &[]));
    ps.extend(ins(38, &[0xF0E4_0001]));
    ps.extend(ins(2, &[0x8002_0000, 0x8055_0000, 0xA000_0009]));
    ps.extend(ins(39, &[]));
    ps.extend(ins(1, &[0x800F_0800, 0x80E4_0000]));
    ps.push(END);
    for (shader, pixel, tokens) in [(VS + 50, false, vs), (PS + 50, true, ps)] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    let consts = |g: &mut Gpu, kind: u8, start: u32, w: &[u32]| {
        let per = if kind % 3 == 2 { 1 } else { 4 };
        let b = words(w);
        g.run(Cmd::SetConstants {
            device: DEVICE,
            kind,
            start,
            count: w.len() as u32 / per,
            data: Data::Bytes(&b),
        });
    };
    g.clear(3, 0, 1.0);
    g.state(ALPHATESTENABLE, 1);
    g.state(ALPHAFUNC, ALPHAFUNC_GREATER);
    g.state(CLIPPING, 1);
    g.state(CLIPPLANEENABLE, 1);
    // One column's strip (column 0's centres at clip x = -1), the whole height.
    let (l, r) = (-1.25f32, -0.75f32);
    let strip = floats(&[
        l, -3.0, 0.5, 1.0, l, 3.0, 0.5, 1.0, r, -3.0, 0.5, 1.0, r, -3.0, 0.5, 1.0, l, 3.0, 0.5,
        1.0, r, 3.0, 0.5, 1.0,
    ]);
    g.run(Cmd::UploadBuffer {
        buffer: VB,
        offset: 0,
        flags: 0,
        data: Data::Bytes(&strip),
    });
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: VB,
        offset: 0,
        stride: 16,
    });
    // Each draw: its column, which of c5..c7 `a0` picks, the colours, its booleans, loop counts,
    // clip plane (keeping rows down to its own) and alpha reference.
    for k in 0..4u32 {
        let x = 0.5 * k as f32;
        consts(&mut g, 0, 0, &[f(x), 0, 0, 0]);
        consts(&mut g, 0, 4, &[f((k % 3) as f32), 0, 0, 0]);
        consts(
            &mut g,
            0,
            5,
            &[
                f(0.5),
                f(0.0),
                f(0.0),
                f(0.75),
                f(0.0),
                f(0.25),
                f(0.0),
                f(0.75),
                f(0.0),
                f(0.0),
                f(0.25),
                f(0.75),
            ],
        );
        consts(&mut g, 2, 0, &[k % 2]);
        consts(&mut g, 1, 0, &[k, 0, 1, 0]);
        consts(&mut g, 3, 2, &[f(0.0625 * k as f32), 0, 0, 0]);
        consts(&mut g, 5, 1, &[(k / 2) % 2]);
        consts(&mut g, 4, 1, &[3 - k, 0, 1, 0]);
        g.run(Cmd::SetClipPlane {
            device: DEVICE,
            index: 0,
            plane: [0, f(1.0), 0, f(0.25 + 0.5 * k as f32)],
        });
        g.state(ALPHAREF, if k == 3 { 250 } else { 16 * k });
        g.run(Cmd::DrawPrimitive {
            device: DEVICE,
            kind: 4,
            start: 0,
            count: 2,
        });
    }
    let px = g.pixels();
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
    Some((px, d.take_draw_digests()))
}

/// The registers from storage give the shaders what the uniform blocks give them, register by
/// register: four draws of one batch with their own float, integer and boolean constants in both
/// stages, a relative read, a defined constant, a clip plane and an alpha reference each draw the
/// same pixels, and the digests (which name the bytes each draw's blocks cover, read through its
/// draw table entry from storage) are alike.
#[test]
fn registers_from_storage_read_every_kind_of_register_as_uniform_blocks_do() {
    let Some((uniform, off)) = every_register_script(false) else {
        return;
    };
    let Some((stored, on)) = every_register_script(true) else {
        return;
    };
    // The script tells the draws apart: each column its own pixels.
    let column = |px: &[[u8; 4]], c: usize| -> Vec<[u8; 4]> {
        px.iter().skip(c).step_by(4).copied().collect()
    };
    for a in 0..4 {
        for b in a + 1..4 {
            assert_ne!(
                column(&uniform, a),
                column(&uniform, b),
                "columns {a} and {b}: {uniform:?}"
            );
        }
    }
    assert_eq!(stored, uniform, "the pixels");
    assert_eq!(on, off, "the digests");
}

/// What [`merge_script`] gives: the target's pixels, the draws' digests in the order the game gave
/// them and in the order the pass was given them, and the keys of held draws drawn with the draws
/// they held ([`d3d9_wgpu::device::Device::held_counts`]).
type MergeScript = (Vec<[u8; 4]>, Vec<u64>, Vec<u64>, (u64, u64));

/// A 4 by 4 target, the registers from storage, held draws merged (`merge`) and sorted (`sort`)
/// or not ([`d3d9_wgpu::device::Device::set_merge_draws`]), blended draws held after the opaque ones
/// (`across`) or not: one strip of two triangles in the
/// buffer, covering column 0 at depth 0.5, drawn by a vs_2_0 moving it right by `c0` and passing
/// `c1` on, and a ps_2_0 drawing what it was passed (`PS + 40`) or its own `c0` (`PS + 41`); the
/// depth test LESSEQUAL with depth writes. `guard`: draws of the same strip with the two pixel
/// shaders in turn; else four opaque draws of one key in columns 0 to 3, a blended one in column
/// 0, and two more of the first key in columns 1 and 2. None where the adapter cannot read the
/// registers from storage.
fn merge_script(merge: bool, sort: bool, across: bool, guard: bool) -> Option<MergeScript> {
    const ALPHABLENDENABLE: u32 = 27;
    const SRCBLEND: u32 = 19;
    const DESTBLEND: u32 = 20;
    let mut g = Gpu::new(4, 1)?;
    {
        let d = g.b.device().expect("the device");
        if !d.set_storage_regs(true) {
            eprintln!("this adapter cannot read the registers from storage: not checked");
            return None;
        }
        d.set_merge_draws(merge, sort, across);
        d.record_draw_digests(true);
    }
    // vs_2_0: dcl_position v0; add oPos, v0, c0; mov oT0, c1.
    let mut vs = vec![0xFFFE_0200];
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(2, &[0xC00F_0000, 0x90E4_0000, 0xA0E4_0000]));
    vs.extend(ins(1, &[0xE00F_0000, 0xA0E4_0001]));
    vs.push(END);
    // ps_2_0: dcl t0; mov oC0, t0. And ps_2_0: mov oC0, c0.
    let mut passed = vec![0xFFFF_0200];
    passed.extend(ins(31, &[0x8000_0000, 0xB00F_0000]));
    passed.extend(ins(1, &[0x800F_0800, 0xB0E4_0000]));
    passed.push(END);
    let mut own = vec![0xFFFF_0200];
    own.extend(ins(1, &[0x800F_0800, 0xA0E4_0000]));
    own.push(END);
    for (shader, pixel, tokens) in [
        (VS + 40, false, vs),
        (PS + 40, true, passed),
        (PS + 41, true, own),
    ] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
    }
    let set_shader = |g: &mut Gpu, pixel: bool, shader: u32| {
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    };
    let regs = |g: &mut Gpu, kind: u8, start: u32, r: &[f32]| {
        let b = floats(r);
        g.run(Cmd::SetConstants {
            device: DEVICE,
            kind,
            start,
            count: r.len() as u32 / 4,
            data: Data::Bytes(&b),
        });
    };
    set_shader(&mut g, false, VS + 40);
    set_shader(&mut g, true, PS + 40);
    g.clear(3, 0, 1.0);
    g.state(ZWRITEENABLE, 1);
    g.state(ZFUNC, 4);
    // Column 0's centres are at clip x = -1 (Direct3D 9's whole coordinates).
    let (l, r) = (-1.25f32, -0.75f32);
    let strip = floats(&[
        l, -3.0, 0.5, 1.0, l, 3.0, 0.5, 1.0, r, -3.0, 0.5, 1.0, r, -3.0, 0.5, 1.0, l, 3.0, 0.5,
        1.0, r, 3.0, 0.5, 1.0,
    ]);
    g.run(Cmd::UploadBuffer {
        buffer: VB,
        offset: 0,
        flags: 0,
        data: Data::Bytes(&strip),
    });
    // The same strip again, for the blended draw (vertices of its own).
    g.run(Cmd::UploadBuffer {
        buffer: VB,
        offset: 96,
        flags: 0,
        data: Data::Bytes(&strip),
    });
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: VB,
        offset: 0,
        stride: 16,
    });
    let draw = |g: &mut Gpu, column: u32, colour: [f32; 4]| {
        let mut r = vec![0.5 * column as f32, 0.0, 0.0, 0.0];
        r.extend_from_slice(&colour);
        regs(g, 0, 0, &r);
        g.run(Cmd::DrawPrimitive {
            device: DEVICE,
            kind: 4,
            start: 0,
            count: 2,
        });
    };
    let red = [1.0, 0.0, 0.0, 1.0];
    let green = [0.0, 1.0, 0.0, 1.0];
    let blue = [0.0, 0.0, 1.0, 1.0];
    let white = [1.0; 4];
    if guard {
        // The second pixel shader in column 0, the first in column 1, the second again in column
        // 1: drawn last there at the same depth, so column 1 is its blue.
        set_shader(&mut g, true, PS + 41);
        regs(&mut g, 3, 0, &red);
        draw(&mut g, 0, white);
        set_shader(&mut g, true, PS + 40);
        draw(&mut g, 1, green);
        set_shader(&mut g, true, PS + 41);
        regs(&mut g, 3, 0, &blue);
        draw(&mut g, 1, white);
    } else {
        for (column, colour) in [red, green, blue, white].into_iter().enumerate() {
            draw(&mut g, column as u32, colour);
        }
        let stream = |g: &mut Gpu, offset: u32| {
            g.run(Cmd::SetStreamSource {
                device: DEVICE,
                stream: 0,
                buffer: VB,
                offset,
                stride: 16,
            });
        };
        // Blended, tested against depth and writing none, as see-through surfaces are drawn.
        g.state(ALPHABLENDENABLE, 1);
        g.state(ZWRITEENABLE, 0);
        g.state(SRCBLEND, 2);
        g.state(DESTBLEND, 2);
        stream(&mut g, 96);
        draw(&mut g, 0, [0.0, 0.0, 0.5, 1.0]);
        stream(&mut g, 0);
        g.state(ALPHABLENDENABLE, 0);
        g.state(ZWRITEENABLE, 1);
        g.state(DESTBLEND, 1);
        draw(&mut g, 1, [1.0, 1.0, 0.0, 1.0]);
        draw(&mut g, 2, [1.0, 1.0, 0.0, 1.0]);
    }
    let px = g.pixels();
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
    assert_eq!(
        d.held_mismatches(),
        0,
        "each held draw drawn with its own state"
    );
    Some((
        px,
        d.take_draw_digests(),
        d.take_emitted_digests(),
        d.held_counts(),
    ))
}

/// Opaque draws of one key held and drawn as one instanced draw (`NV_WGPU_MERGE_DRAWS`), each
/// instance with its own registers (its own place and colour): the same pixels as one draw each,
/// every draw given to the pass once with its own digest, never across a blended draw; sorted
/// the same.
#[test]
fn merged_draws_draw_what_one_draw_each_draws() {
    let Some((off_px, off, off_emitted, off_held)) = merge_script(false, false, false, false)
    else {
        return;
    };
    assert_eq!(off.len(), 7, "every draw noted");
    assert_eq!(off_emitted, off, "unmerged: given in the game's order");
    assert_eq!(off_held, (0, 0), "nothing held unmerged");
    let red = [0, 0, 255, 255];
    let yellow = [0, 255, 255, 255];
    let white = [255, 255, 255, 255];
    for row in off_px.chunks_exact(4) {
        assert_eq!(&row[1..], &[yellow, yellow, white], "{off_px:?}");
        assert_eq!(&row[0][1..3], &red[1..3], "column 0: red with blue added");
    }
    for sort in [false, true] {
        let (px, drawn, mut emitted, held) =
            merge_script(true, sort, false, false).expect("the adapter, as above");
        assert_eq!(px, off_px, "the pixels, sorted {sort}");
        assert_eq!(
            drawn, off,
            "the digests as the game gave the draws, sorted {sort}"
        );
        assert_eq!(
            held,
            (2, 6),
            "four draws as one, then two after the blended one, sorted {sort}"
        );
        let mut want = off.clone();
        want.sort_unstable();
        emitted.sort_unstable();
        assert_eq!(emitted, want, "every draw given once, sorted {sort}");
    }
}

/// A held draw of the same vertices as a held draw of another key (another pass over the same
/// surface) is not drawn before it: the held draws are drawn first. Column 1 takes the last
/// draw's blue, as with one draw each.
#[test]
fn merged_draws_keep_a_second_pass_over_the_same_vertices_after_the_first() {
    let Some((off_px, off, _, _)) = merge_script(false, false, false, true) else {
        return;
    };
    let red = [0, 0, 255, 255];
    let blue = [255, 0, 0, 255];
    for row in off_px.chunks_exact(4) {
        assert_eq!(&row[..2], &[red, blue], "{off_px:?}");
    }
    let (px, drawn, _, held) =
        merge_script(true, false, false, true).expect("the adapter, as above");
    assert_eq!(px, off_px, "the pixels");
    assert_eq!(drawn, off, "the digests");
    assert_eq!(
        held,
        (3, 3),
        "each draw on its own: no key drawn before the other's draw"
    );
}

/// Blended draws held after the opaque ones (`NV_WGPU_SORT_DRAWS=pass`): the blended one is drawn
/// after the opaque ones held before it, the opaque draws given after it are drawn after it (they
/// could lie behind it), and the pixels are as with one draw each.
#[test]
fn merged_draws_around_a_held_blended_draw_draw_what_one_draw_each_draws() {
    let Some((off_px, off, _, _)) = merge_script(false, false, false, false) else {
        return;
    };
    let (px, drawn, mut emitted, held) =
        merge_script(true, true, true, false).expect("the adapter, as above");
    assert_eq!(px, off_px, "the pixels");
    assert_eq!(drawn, off, "the digests as the game gave the draws");
    let mut want = off.clone();
    want.sort_unstable();
    emitted.sort_unstable();
    assert_eq!(emitted, want, "every draw given once");
    assert_eq!(
        held,
        (3, 7),
        "the first four opaque draws as one, the blended one, the last two as one"
    );
}

/// A draw of [`held_script`]'s strip in a column, with a colour.
type StripDraw = dyn Fn(&mut Gpu, u32, [f32; 4]);
/// What [`held_script`] runs: its draws, given the strip's draw.
type HeldDraws = dyn Fn(&mut Gpu, &StripDraw);
/// What [`held_script`] gives: the pixels, the held keys drawn with their draws, the mismatches.
type HeldScript = (Vec<[u8; 4]>, (u64, u64), u64);

/// An 8 by 8 target, the registers from storage, held draws merged (`merge`, with `pass`: blended
/// draws held after the opaque ones) or not: the vs_2_0 of [`merge_script`] (moved right by `c0`,
/// passing `c1` on) and its ps_2_0 drawing what it was passed, two strips of one column each in
/// the buffer (the second one column further right), the depth test LESSEQUAL with depth writes.
/// `script` draws; then the pixels, the held draws drawn with the draws they held, and the held
/// draws drawn with another state than they asked for. None where the adapter cannot read the
/// registers from storage.
fn held_script(merge: bool, pass: bool, script: &HeldDraws) -> Option<HeldScript> {
    let mut g = Gpu::new(8, 1)?;
    {
        let d = g.b.device().expect("the device");
        if !d.set_storage_regs(true) {
            eprintln!("this adapter cannot read the registers from storage: not checked");
            return None;
        }
        d.set_merge_draws(merge, pass, pass);
        d.record_draw_digests(true);
    }
    // vs_2_0: dcl_position v0; add oPos, v0, c0; mov oT0, c1. ps_2_0: dcl t0; mov oC0, t0.
    let mut vs = vec![0xFFFE_0200];
    vs.extend(ins(31, &[0x8000_0000, 0x900F_0000]));
    vs.extend(ins(2, &[0xC00F_0000, 0x90E4_0000, 0xA0E4_0000]));
    vs.extend(ins(1, &[0xE00F_0000, 0xA0E4_0001]));
    vs.push(END);
    let mut ps = vec![0xFFFF_0200];
    ps.extend(ins(31, &[0x8000_0000, 0xB00F_0000]));
    ps.extend(ins(1, &[0x800F_0800, 0xB0E4_0000]));
    ps.push(END);
    for (shader, pixel, tokens) in [(VS + 60, false, vs), (PS + 60, true, ps)] {
        let t = words(&tokens);
        g.run(Cmd::CreateShader {
            device: DEVICE,
            shader,
            pixel,
            tokens: Data::Bytes(&t),
        });
        g.run(Cmd::SetShader {
            device: DEVICE,
            pixel,
            shader,
        });
    }
    g.clear(3, 0, 1.0);
    g.state(ZWRITEENABLE, 1);
    g.state(ZFUNC, 4);
    // Column c's centres are at clip x = -1 + c / 4 (Direct3D 9's whole coordinates).
    let mut verts = Vec::new();
    for shift in [0.0f32, 0.25] {
        let (l, r) = (-1.125 + shift, -0.875 + shift);
        verts.extend([
            l, -3.0, 0.5, 1.0, l, 3.0, 0.5, 1.0, r, -3.0, 0.5, 1.0, r, -3.0, 0.5, 1.0, l, 3.0, 0.5,
            1.0, r, 3.0, 0.5, 1.0,
        ]);
    }
    let b = floats(&verts);
    g.run(Cmd::UploadBuffer {
        buffer: VB,
        offset: 0,
        flags: 0,
        data: Data::Bytes(&b),
    });
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: VB,
        offset: 0,
        stride: 16,
    });
    let draw = |g: &mut Gpu, column: u32, colour: [f32; 4]| {
        let mut r = vec![0.25 * column as f32, 0.0, 0.0, 0.0];
        r.extend_from_slice(&colour);
        let bytes = floats(&r);
        g.run(Cmd::SetConstants {
            device: DEVICE,
            kind: 0,
            start: 0,
            count: 2,
            data: Data::Bytes(&bytes),
        });
        g.run(Cmd::DrawPrimitive {
            device: DEVICE,
            kind: 4,
            start: 0,
            count: 2,
        });
    };
    script(&mut g, &draw);
    let px = g.pixels();
    let d = g.b.device().expect("the device");
    assert_eq!(d.wgpu_errors(), 0);
    Some((px, d.held_counts(), d.held_mismatches()))
}

/// Pairs of opaque draws of one strip that differ in exactly one thing the pass takes besides
/// the registers (the viewport, the scissor rectangle, the stream offset, the first vertex) are
/// not merged into one key: merged, the pixels are those of one draw each, every held draw is
/// drawn with the state it asked for, and no two of the pairs share a key.
#[test]
fn merged_draws_keep_apart_draws_that_differ_in_one_pass_setting() {
    const SCISSORTESTENABLE: u32 = 174;
    let script = |g: &mut Gpu, draw: &StripDraw| {
        let viewport = |g: &mut Gpu, x: u32, w: u32| {
            g.run(Cmd::SetViewport {
                device: DEVICE,
                viewport: [x, 0, w, 8, 0f32.to_bits(), 1f32.to_bits()],
            });
        };
        let scissor = |g: &mut Gpu, bottom: i32| {
            g.run(Cmd::SetScissorRect {
                device: DEVICE,
                rect: [0, 0, 8, bottom],
            });
        };
        let stream = |g: &mut Gpu, offset: u32| {
            g.run(Cmd::SetStreamSource {
                device: DEVICE,
                stream: 0,
                buffer: VB,
                offset,
                stride: 16,
            });
        };
        // The viewport: the second moved a column right.
        viewport(g, 0, 8);
        draw(g, 0, [1.0, 0.0, 0.0, 1.0]);
        viewport(g, 1, 7);
        draw(g, 0, [0.0, 1.0, 0.0, 1.0]);
        viewport(g, 0, 8);
        // The scissor: the second cut to the top half.
        g.state(SCISSORTESTENABLE, 1);
        scissor(g, 8);
        draw(g, 2, [0.0, 0.0, 1.0, 1.0]);
        scissor(g, 4);
        draw(g, 2, [1.0, 1.0, 0.0, 1.0]);
        g.state(SCISSORTESTENABLE, 0);
        // The stream offset: the second strip, a column further right.
        draw(g, 4, [1.0, 0.0, 1.0, 1.0]);
        stream(g, 96);
        draw(g, 4, [0.0, 1.0, 1.0, 1.0]);
        stream(g, 0);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    let (on, held, mismatches) = held_script(true, false, &script).expect("the adapter, as above");
    assert_eq!(on, off, "the pixels");
    assert_eq!(mismatches, 0, "each held draw drawn with its own state");
    assert_eq!(held, (6, 6), "no two draws of the pairs share a key");
    // The pairs drew apart: column 1 (the moved viewport), the scissored half of column 2, and
    // column 5 (the second strip) have the second draws' colours.
    let at = |x: usize, y: usize| off[8 * y + x];
    assert_eq!(at(5, 2), [255, 255, 0, 255], "the second strip");
    assert_eq!(
        at(2, 0),
        [0, 255, 255, 255],
        "the scissored draw's top half"
    );
    assert_eq!(at(2, 6), [255, 0, 0, 255], "the first draw's bottom half");
}

/// With blended draws held after the opaque ones (`NV_WGPU_SORT_DRAWS=pass`), a blended draw
/// that ignores depth (as a sky layer or a glare) or writes it (as a fading object) is given in
/// the game's order: an opaque draw given after it, in the same place, still covers it (or fails
/// behind it), as with one draw each.
#[test]
fn held_blended_draws_never_move_past_opaque_draws_when_they_ignore_or_write_depth() {
    const ALPHABLENDENABLE: u32 = 27;
    const SRCBLEND: u32 = 19;
    const DESTBLEND: u32 = 20;
    let script = |g: &mut Gpu, draw: &StripDraw| {
        g.state(SRCBLEND, 2);
        g.state(DESTBLEND, 2);
        // Column 0: a blended draw with the depth test off, then an opaque draw over it.
        g.state(ALPHABLENDENABLE, 1);
        g.state(ZENABLE, 0);
        draw(g, 0, [0.0, 0.0, 0.5, 1.0]);
        g.state(ZENABLE, 1);
        g.state(ALPHABLENDENABLE, 0);
        draw(g, 0, [1.0, 0.0, 0.0, 1.0]);
        // Column 1: a blended draw writing depth, then an opaque draw at the same depth (LESS:
        // it fails where the blended draw wrote).
        g.state(ALPHABLENDENABLE, 1);
        draw(g, 1, [0.0, 0.0, 0.5, 1.0]);
        g.state(ALPHABLENDENABLE, 0);
        g.state(ZFUNC, 2);
        draw(g, 1, [0.0, 1.0, 0.0, 1.0]);
        g.state(ZFUNC, 4);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    let (on, _, mismatches) = held_script(true, true, &script).expect("the adapter, as above");
    assert_eq!(on, off, "the pixels");
    assert_eq!(mismatches, 0, "each held draw drawn with its own state");
    assert_eq!(
        off[0],
        [0, 0, 255, 255],
        "column 0: the opaque draw over the depth-off one"
    );
    assert!(
        (127..=128).contains(&off[1][0]) && off[1][1..3] == [0, 0],
        "column 1: the blended draw alone (the opaque one failed): {:?}",
        off[1]
    );
}

/// [`held_script`]'s strip drawn with its registers: moved right by `column` columns and back by
/// `z` in depth, passing `colour` on.
fn strip_at(g: &mut Gpu, column: u32, z: f32, colour: [f32; 4]) {
    let mut r = vec![0.25 * column as f32, 0.0, z, 0.0];
    r.extend_from_slice(&colour);
    let bytes = floats(&r);
    g.run(Cmd::SetConstants {
        device: DEVICE,
        kind: 0,
        start: 0,
        count: 2,
        data: Data::Bytes(&bytes),
    });
    g.run(Cmd::DrawPrimitive {
        device: DEVICE,
        kind: 4,
        start: 0,
        count: 2,
    });
}

/// [`held_script`]'s vertex stream 0 at `offset` (96: the second strip, a column further right).
fn strip_stream(g: &mut Gpu, offset: u32) {
    g.run(Cmd::SetStreamSource {
        device: DEVICE,
        stream: 0,
        buffer: VB,
        offset,
        stride: 16,
    });
}

/// With blended draws held after the opaque ones (`NV_WGPU_SORT_DRAWS=pass`), an opaque draw the
/// game gives after a see-through draw (blended, tested against depth, writing none) and lying
/// behind it on screen still covers it, as with one draw each: it is not drawn ahead of it.
#[test]
fn an_opaque_draw_after_a_held_see_through_draw_still_covers_it() {
    const ALPHABLENDENABLE: u32 = 27;
    const SRCBLEND: u32 = 19;
    const DESTBLEND: u32 = 20;
    let script = |g: &mut Gpu, _: &StripDraw| {
        // Column 1: the see-through draw (the second strip, its own vertices) in front ...
        g.state(SRCBLEND, 2);
        g.state(DESTBLEND, 2);
        g.state(ALPHABLENDENABLE, 1);
        g.state(ZWRITEENABLE, 0);
        strip_stream(g, 96);
        strip_at(g, 0, 0.0, [0.0, 0.0, 0.5, 1.0]);
        // ... then an opaque draw behind it (the first strip moved a column right): the
        // see-through draw wrote no depth, so it passes and covers it.
        g.state(ALPHABLENDENABLE, 0);
        g.state(ZWRITEENABLE, 1);
        g.state(DESTBLEND, 1);
        strip_stream(g, 0);
        strip_at(g, 1, 0.25, [1.0, 0.0, 0.0, 1.0]);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    assert_eq!(off[1], [0, 0, 255, 255], "column 1: the opaque draw's red");
    let (on, held, mismatches) = held_script(true, true, &script).expect("the adapter, as above");
    assert_eq!(on, off, "the pixels");
    assert_eq!(mismatches, 0, "each held draw drawn with its own state");
    assert_eq!(held, (2, 2), "both draws held");
}

/// Merged in place (`NV_WGPU_MERGE_DRAWS=1`, not sorted): of draws of two keys at exactly the
/// same depth given K1, K2, K1 (the depth test LESSEQUAL, so the last one shows), the last K1
/// does not join the first (it would be drawn before K2): the pixel is the last draw's, as with
/// one draw each, while draws of one key given one after another still join.
#[test]
fn merged_draws_in_place_keep_which_of_two_coplanar_surfaces_shows() {
    let script = |g: &mut Gpu, _: &StripDraw| {
        // K1 (the first strip) in columns 3 and 4 and then 1: one run of K1.
        strip_at(g, 3, 0.0, [1.0, 1.0, 1.0, 1.0]);
        strip_at(g, 4, 0.0, [1.0, 1.0, 1.0, 1.0]);
        strip_at(g, 1, 0.0, [1.0, 0.0, 0.0, 1.0]);
        // K2 (the second strip) over column 1 at the same depth.
        strip_stream(g, 96);
        strip_at(g, 0, 0.0, [0.0, 1.0, 0.0, 1.0]);
        // K1 again over column 1 at the same depth: drawn last, it shows.
        strip_stream(g, 0);
        strip_at(g, 1, 0.0, [0.0, 0.0, 1.0, 1.0]);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    assert_eq!(off[1], [255, 0, 0, 255], "column 1: the last draw's blue");
    let (on, held, mismatches) = held_script(true, false, &script).expect("the adapter, as above");
    assert_eq!(on, off, "the pixels");
    assert_eq!(mismatches, 0, "each held draw drawn with its own state");
    assert_eq!(
        held,
        (3, 5),
        "the first three K1 draws as one, K2, the last K1 on its own"
    );
}

/// Sorted (`NV_WGPU_SORT_DRAWS`), a draw that writes depth but no colour is never moved past a
/// draw that writes colour: moved past another surface's draw it would hide that draw's colour or
/// let it show where the game's order did not. Of the pairs colour A then depth-only D in front of
/// it (column 1) and depth-only D then colour A behind it (column 3), one would differ whichever
/// of the two keys the sort put first; the pixels are those of one draw each. The held draws
/// write one set of channels at a time: A, then both D draws as one, then A.
#[test]
fn sorted_draws_keep_depth_only_draws_in_place() {
    const COLORWRITEENABLE: u32 = 168;
    let script = |g: &mut Gpu, _: &StripDraw| {
        // Column 1: A (the first strip), then D (the second strip, no colour) in front of it.
        strip_at(g, 1, 0.0, [1.0, 0.0, 0.0, 1.0]);
        strip_stream(g, 96);
        g.state(COLORWRITEENABLE, 0);
        strip_at(g, 0, -0.25, [0.0, 1.0, 0.0, 1.0]);
        // Column 3: D in front first, then A behind it.
        strip_at(g, 2, -0.25, [0.0, 1.0, 0.0, 1.0]);
        g.state(COLORWRITEENABLE, 0xF);
        strip_stream(g, 0);
        strip_at(g, 3, 0.0, [1.0, 0.0, 0.0, 1.0]);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    assert_eq!(
        off[1],
        [0, 0, 255, 255],
        "column 1: A's red (D wrote no colour)"
    );
    assert_eq!(off[3][2], 0, "column 3: no red (A behind D's depth)");
    let (on, held, mismatches) = held_script(true, true, &script).expect("the adapter, as above");
    assert_eq!(on, off, "the pixels");
    assert_eq!(mismatches, 0, "each held draw drawn with its own state");
    assert_eq!(
        held,
        (3, 4),
        "A; both depth-only draws as one; A: no draw moved past one writing other channels"
    );
}

/// Sorted, draws that write the same channels are moved past each other even when they leave
/// some channel alone: here red, green and blue of an A8R8G8B8 target, alpha masked, as the
/// game's world draws often are. A at column 1, B (the other strip) at column 3, A again at
/// column 2: the two A draws as one, and the pixels those of one draw each.
#[test]
fn sorted_draws_with_the_same_masked_channels_are_held() {
    const COLORWRITEENABLE: u32 = 168;
    let script = |g: &mut Gpu, _: &StripDraw| {
        g.state(COLORWRITEENABLE, 7);
        strip_at(g, 1, 0.0, [1.0, 0.0, 0.0, 1.0]);
        strip_stream(g, 96);
        strip_at(g, 2, 0.0, [0.0, 1.0, 0.0, 1.0]);
        strip_stream(g, 0);
        strip_at(g, 2, -0.25, [0.0, 0.0, 1.0, 1.0]);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    let (on, held, mismatches) = held_script(true, true, &script).expect("the adapter, as above");
    assert_eq!(on, off, "the pixels");
    assert_eq!(mismatches, 0, "each held draw drawn with its own state");
    assert_eq!(held, (2, 3), "the two A draws as one across B");
}

/// Sorted, a draw over the same vertices as a held draw of another key has the held draws drawn
/// first; the draws held after it still keep to its colour channels. Two cases in one run, so
/// that whichever of the two pipelines a sort puts first, one of them would show a moved draw:
/// - column 1: colour A1 (LESSEQUAL, at column 3) then A2 (LESS) over the same strip, then
///   depth-only D in front of A2: A2's red must show;
/// - column 5: depth-only D1 (LESSEQUAL, at column 7, so its early draw lays no depth there) then
///   D2 (LESS) over the same strip, then colour A behind D2: A must not show.
#[test]
fn sorted_draws_keep_their_channels_after_a_draw_over_held_vertices() {
    const COLORWRITEENABLE: u32 = 168;
    let script = |g: &mut Gpu, _: &StripDraw| {
        g.state(ZFUNC, 4);
        strip_at(g, 3, 0.0, [1.0, 0.0, 0.0, 1.0]);
        g.state(ZFUNC, 2);
        strip_at(g, 1, 0.0, [1.0, 0.0, 0.0, 1.0]);
        g.state(COLORWRITEENABLE, 0);
        strip_stream(g, 96);
        strip_at(g, 0, -0.25, [0.0, 1.0, 0.0, 1.0]);
        g.state(ZFUNC, 4);
        strip_at(g, 6, -0.25, [0.0, 1.0, 0.0, 1.0]);
        g.state(ZFUNC, 2);
        strip_at(g, 4, -0.25, [0.0, 1.0, 0.0, 1.0]);
        g.state(COLORWRITEENABLE, 0xF);
        strip_stream(g, 0);
        strip_at(g, 5, 0.0, [1.0, 0.0, 0.0, 1.0]);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    assert_eq!(off[1], [0, 0, 255, 255], "column 1: A2's red");
    assert_eq!(off[5][2], 0, "column 5: no red (A behind D1 and D2)");
    let (on, _, mismatches) = held_script(true, true, &script).expect("the adapter, as above");
    assert_eq!(on, off, "the pixels");
    assert_eq!(mismatches, 0, "each held draw drawn with its own state");
}

/// Sorted, a draw that writes alpha and one that leaves it alone are never moved past each other
/// (an A8R8G8B8 target). Two cases in one run, so that whichever of the two pipelines a sort puts
/// first, one of them would show a moved draw:
/// - column 1: Y (red, green, blue and alpha 0.25) behind, then X (red, green and blue only) in
///   front: X's colour with Y's alpha;
/// - column 3: X in front first, then Y behind it: X's colour, the alpha left as cleared.
#[test]
fn sorted_draws_keep_alpha_writes_apart() {
    const COLORWRITEENABLE: u32 = 168;
    let x = |g: &mut Gpu, column: u32| {
        g.state(COLORWRITEENABLE, 7);
        strip_stream(g, 0);
        strip_at(g, column, -0.25, [1.0, 0.0, 0.0, 1.0]);
    };
    let y = |g: &mut Gpu, column: u32| {
        g.state(COLORWRITEENABLE, 0xF);
        strip_stream(g, 96);
        strip_at(g, column - 1, 0.0, [0.0, 1.0, 0.0, 0.25]);
    };
    let script = move |g: &mut Gpu, _: &StripDraw| {
        y(g, 1);
        x(g, 1);
        x(g, 3);
        y(g, 3);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    assert_eq!(off[1][3], 64, "column 1: Y's alpha (X wrote none)");
    assert_eq!(off[3][3], 0, "column 3: the cleared alpha (Y behind X)");
    let (on, _, mismatches) = held_script(true, true, &script).expect("the adapter, as above");
    assert_eq!(on, off, "the pixels, alpha included");
    assert_eq!(mismatches, 0, "each held draw drawn with its own state");
}

/// Merged or sorted, a draw that writes no depth is never held: each such draw lands over the ones
/// before it wherever it passes the depth test, so moved past another over the same pixels it
/// would show where the game's order covered it (#890's planted fault G1 survived without this).
/// With depth writes off: P (red, LESSEQUAL) then Q (green, LESS) at column 1, and Q then P at
/// column 3, so one column would show a moved draw whichever pipeline a sort puts first.
#[test]
fn sorted_draws_never_hold_draws_without_depth_writes() {
    let script = |g: &mut Gpu, _: &StripDraw| {
        g.state(ZWRITEENABLE, 0);
        strip_at(g, 1, 0.0, [1.0, 0.0, 0.0, 1.0]);
        g.state(ZFUNC, 2);
        strip_stream(g, 96);
        strip_at(g, 0, 0.0, [0.0, 1.0, 0.0, 1.0]);
        strip_at(g, 2, 0.0, [0.0, 1.0, 0.0, 1.0]);
        g.state(ZFUNC, 4);
        strip_stream(g, 0);
        strip_at(g, 3, 0.0, [1.0, 0.0, 0.0, 1.0]);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    assert_eq!(off[1], [0, 255, 0, 255], "column 1: Q's green, drawn last");
    assert_eq!(off[3], [0, 0, 255, 255], "column 3: P's red, drawn last");
    let (on, held, mismatches) = held_script(true, true, &script).expect("the adapter, as above");
    assert_eq!(on, off, "the pixels");
    assert_eq!(mismatches, 0, "each draw drawn with its own state");
    assert_eq!(held, (0, 0), "none held");
}

/// Merged or sorted, a draw over the same vertices as a held draw of another key (another pass
/// over the same surface) has the held draws drawn first, so the later pass still lands last
/// (#890's planted fault G2 survived without this). The keys differ only in the cull mode, which
/// changes nothing on these front faces; both depth tests pass at the same depth (LESSEQUAL). Red
/// then green at column 1, green then red at column 3, all over one strip, so one column would
/// show the earlier pass whichever key a sort puts first.
#[test]
fn held_draws_over_one_surface_keep_their_order_across_keys() {
    let script = |g: &mut Gpu, _: &StripDraw| {
        strip_at(g, 1, 0.0, [1.0, 0.0, 0.0, 1.0]);
        g.state(CULLMODE, 1);
        strip_at(g, 1, 0.0, [0.0, 1.0, 0.0, 1.0]);
        strip_at(g, 3, 0.0, [0.0, 1.0, 0.0, 1.0]);
        g.state(CULLMODE, 3);
        strip_at(g, 3, 0.0, [1.0, 0.0, 0.0, 1.0]);
    };
    let Some((off, _, _)) = held_script(false, false, &script) else {
        return;
    };
    assert_eq!(off[1], [0, 255, 0, 255], "column 1: green, drawn last");
    assert_eq!(off[3], [0, 0, 255, 255], "column 3: red, drawn last");
    for (merge, pass) in [(true, false), (true, true)] {
        let (on, _, mismatches) = held_script(merge, pass, &script).expect("the adapter, as above");
        assert_eq!(on, off, "merge {merge} pass {pass}: the pixels");
        assert_eq!(mismatches, 0, "each held draw drawn with its own state");
    }
}
