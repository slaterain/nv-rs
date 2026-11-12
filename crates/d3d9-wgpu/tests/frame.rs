//! The frame-level calls Doc's house and the exterior need beyond the instructions: user clip
//! planes, occlusion counts, `GetRenderTargetData` and `ReadSurface`, clears of part of a target,
//! and NVIDIA's alpha to coverage switch, each run through the whole backend on the GPU and
//! checked against what Direct3D 9 defines.
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

    fn make(
        size: u32,
        samples: u32,
        cache: Option<std::path::PathBuf>,
        env: &[(&str, &str)],
    ) -> Option<Gpu> {
        // A failed test leaves the lock poisoned; the next one still takes its turn.
        let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        for (name, value) in env {
            std::env::set_var(name, value);
        }
        let mut b = WgpuBackend::new(Output::Offscreen);
        if let Some(dir) = cache {
            b = b.with_cache_dir(dir);
        }
        let present = words(&[size, size, A8R8G8B8, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
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
        for (name, _) in env {
            std::env::remove_var(name);
        }
        if b.device().is_none() {
            eprintln!("no GPU adapter: the frame tests are skipped");
            return None;
        }
        let mut g = Gpu {
            b,
            next: 0,
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
    assert_eq!(d.module_counts().0, 1, "one vertex module for all three");
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
    assert_eq!(
        d.module_counts().0,
        3,
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
    assert_eq!(
        d.module_counts(),
        (2, 3),
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
    assert_eq!(
        d.module_counts(),
        (1, 1),
        "a module for each stage, none refused"
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

#[test]
fn nvidia_alpha_to_coverage_stands_in_for_the_alpha_test_on_multisampled_targets() {
    let atoc = u32::from_le_bytes(*b"ATOC");
    for samples in [1u32, 4, 8] {
        let Some(mut g) = Gpu::new(4, samples) else {
            return;
        };
        g.state(ZENABLE, 0);
        g.state(ZWRITEENABLE, 0);
        g.state(ALPHATESTENABLE, 1);
        g.state(ADAPTIVETESS_Y, atoc);
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
        if samples == 1 {
            // One sample: the alpha test, with its reference.
            assert_eq!(covered(&mut g, 0.25, 5, 0x80), 0, "0.25 > 0.5");
            assert_eq!(covered(&mut g, 0.75, 5, 0x80), 16, "0.75 > 0.5");
            assert_eq!(covered(&mut g, 0.25, 2, 0x80), 16, "0.25 < 0.5");
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
                    "{samples}x: alpha {a}: {n} after {last}"
                );
                last = n;
                for func in [5, 6, 7] {
                    for aref in [0, 0x80, 0xFF] {
                        assert_eq!(
                            covered(&mut g, a, func, aref),
                            n,
                            "{samples}x: alpha {a}, comparison {func}, reference {aref:#x}"
                        );
                    }
                }
                for func in 1..=4 {
                    assert_eq!(
                        covered(&mut g, a, func, 0x80),
                        0,
                        "{samples}x: comparison {func}"
                    );
                }
                let ends = match k {
                    0 => Some(0),
                    16 => Some(16 * samples),
                    _ => None,
                };
                if let Some(e) = ends {
                    assert_eq!(n, e, "{samples}x: alpha {a}");
                }
            }
            assert!(
                covered(&mut g, 0.5, 8, 0x80) > 0 && covered(&mut g, 0.5, 8, 0x80) < 16 * samples,
                "{samples}x: alpha 0.5 covers part of the samples"
            );
        }
        g.state(ALPHAFUNC, 5);
        g.state(ALPHAREF, 0x80);
        g.colour([1.0, 1.0, 1.0, 0.25]);
        // The switch off: the alpha test again.
        g.state(ADAPTIVETESS_Y, 0);
        g.clear(1, 0, 1.0);
        g.cover(0.5);
        assert_eq!(g.reds(), vec![0; 16], "{samples}: switch off");
    }
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

/// The backend's own loading screen: black at the corners and the logo lit in the middle; the
/// bar filled to the fraction done (a quarter against three quarters: some row has the fill's
/// colour at three quarters where it has the empty bar's grey at a quarter); and (on Windows,
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
    for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        assert_eq!(at(&quarter, x, y), [0, 0, 0], "corner {x},{y}");
    }
    let lit = (h / 4..h / 2)
        .flat_map(|y| (w / 3..2 * w / 3).map(move |x| (x, y)))
        .filter(|&(x, y)| at(&quarter, x, y).iter().any(|&c| c > 64))
        .count();
    assert!(lit > 500, "{lit} pixels of the logo lit");
    // The bar: from x = w/3, w/3 wide; at x = w/3 + w/6 (half of it) the fill (B G R about 89, 191,
    // 230) at three quarters, the empty bar's grey (about 64) at a quarter, in the same row.
    let x = w / 3 + w / 6;
    let fill = |c: [u8; 3]| c[2] > 180 && c[1] > 140 && c[0] < 140;
    let grey = |c: [u8; 3]| c.iter().all(|&v| (40..=90).contains(&v));
    let row = (0..h).find(|&y| fill(at(&most, x, y)) && grey(at(&quarter, x, y)));
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

/// A triangle list from clip-space vertices (x, y, z; w 1), in its own part of the buffer (at
/// most two triangles).
fn tris(g: &mut Gpu, v: &[[f32; 3]]) {
    let data: Vec<f32> = v.iter().flat_map(|p| [p[0], p[1], p[2], 1.0]).collect();
    let bytes = floats(&data);
    let offset = 96 * (g.next % 32);
    g.next += 1;
    g.run(Cmd::UploadBuffer {
        buffer: VB,
        offset,
        flags: 0,
        data: Data::Bytes(&bytes),
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
        count: (v.len() / 3) as u32,
    });
}

/// Clip x from `x0` to `x1` over the viewport's height, at depth `z0` on the left and `z1` on the
/// right: two triangles wound clockwise on screen (Direct3D 9's front faces), or counter-clockwise.
fn band(g: &mut Gpu, x0: f32, x1: f32, z0: f32, z1: f32, clockwise: bool) {
    let (a, b, c) = ([x0, -1.0, z0], [x0, 1.0, z0], [x1, -1.0, z1]);
    let (d, e, f) = ([x1, -1.0, z1], [x0, 1.0, z0], [x1, 1.0, z1]);
    if clockwise {
        tris(g, &[a, b, c, d, e, f]);
    } else {
        tris(g, &[a, c, b, d, f, e]);
    }
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
/// 32 by 32 target; what Direct3D 9 draws there, left and right of the cell's middle.
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
    // counter-clockwise ones; the pipelines differ only in the cull mode.
    g.state(ZENABLE, 0);
    g.colour(GREEN);
    let cw = [[-3.0, -3.0, 0.5], [-3.0, 9.0, 0.5], [9.0, -3.0, 0.5]];
    let ccw = [cw[0], cw[2], cw[1]];
    for (i, cull, tri) in [(0, 2, cw), (1, 2, ccw), (2, 3, cw), (3, 3, ccw)] {
        g.state(CULLMODE, cull);
        at(g, i);
        tris(g, &tri);
    }
    expect(0, none, none, "CULLMODE 2 culls a clockwise triangle");
    expect(1, green, green, "CULLMODE 2 keeps a counter-clockwise one");
    expect(2, green, green, "CULLMODE 3 keeps a clockwise triangle");
    expect(3, none, none, "CULLMODE 3 culls a counter-clockwise one");
    g.state(CULLMODE, 1);

    // Depth writes switched on alone (the same ZFUNC): the third draw (GREATER, further away)
    // passes only where the second wrote its depth.
    g.state(ZENABLE, 1);
    g.state(ZFUNC, 4);
    at(g, 4);
    g.state(ZWRITEENABLE, 0);
    g.colour(BLUE);
    band(g, -1.0, 1.0, 0.5, 0.5, true);
    g.state(ZWRITEENABLE, 1);
    g.colour(RED);
    band(g, -1.0, 0.0, 0.5, 0.5, true);
    g.state(ZFUNC, 5);
    g.state(ZWRITEENABLE, 0);
    g.colour(GREEN);
    band(g, -1.0, 1.0, 0.75, 0.75, true);
    expect(4, green, blue, "ZWRITEENABLE 1 after 0 writes depth");
    g.state(ZFUNC, 4);
    g.state(ZWRITEENABLE, 1);
    g.state(ZENABLE, 0);

    // One-sided stencil: REPLACE with 1 on the left, then EQUAL 1 over the cell.
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
    band(g, -1.0, 0.0, 0.5, 0.5, true);
    g.state(STENCILFUNC, 3);
    g.state(STENCILPASS, 1);
    g.colour(GREEN);
    band(g, -1.0, 1.0, 0.5, 0.5, true);
    expect(
        5,
        green,
        none,
        "STENCILFUNC EQUAL passes where REPLACE wrote",
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
    band(g, -1.0, 0.0, 0.5, 0.5, false);
    band(g, 0.0, 1.0, 0.5, 0.5, true);
    g.state(TWOSIDEDSTENCILMODE, 0);
    g.state(STENCILFUNC, 3);
    g.colour(GREEN);
    band(g, -1.0, 1.0, 0.5, 0.5, true);
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
    band(g, -1.0, 1.0, 0.5, 0.5, true);
    g.state(ZFUNC, 2);
    g.state(DEPTHBIAS, (-0.001f32).to_bits());
    g.colour(GREEN);
    band(g, -1.0, 1.0, 0.5, 0.5, true);
    g.state(DEPTHBIAS, 0);
    expect(7, green, green, "DEPTHBIAS brings a coplanar draw in front");

    // Slope-scaled depth bias, on a band whose depth runs from 0.25 to 0.75 across the cell.
    g.state(ZFUNC, 4);
    at(g, 8);
    g.colour(RED);
    band(g, -1.0, 1.0, 0.25, 0.75, true);
    g.state(ZFUNC, 2);
    g.state(SLOPESCALEDEPTHBIAS, (-2.0f32).to_bits());
    g.colour(GREEN);
    band(g, -1.0, 1.0, 0.25, 0.75, true);
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
