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
        // A failed test leaves the lock poisoned; the next one still takes its turn.
        let turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let mut b = WgpuBackend::new(Output::Offscreen);
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
