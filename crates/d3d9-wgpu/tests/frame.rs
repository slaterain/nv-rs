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

/// A backend with a `size` by `size` A8R8G8B8 target (`samples` per pixel), a D24S8 depth
/// surface of the same, a pass-through vertex shader (`mov oPos, v0`: positions in clip space)
/// and a pixel shader writing `c0`.
struct Gpu {
    b: WgpuBackend,
    next: u32,
}

impl Gpu {
    fn new(size: u32, samples: u32) -> Option<Gpu> {
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
        let mut g = Gpu { b, next: 0 };
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
        let tri = floats(&[
            -3.0, -3.0, z, 1.0, -3.0, 9.0, z, 1.0, 9.0, -3.0, z, 1.0,
        ]);
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
        match self.run(Cmd::QueryGetData { query, flags: 1 }) {
            Reply::Query(Some((0, n))) => n,
            other => panic!("query {query:#x}: {other:?}"),
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
    for samples in [1u32, 4] {
        let Some(mut g) = Gpu::new(4, samples) else {
            return;
        };
        for q in [0x2000, 0x2010, 0x2020, 0x2030] {
            g.run(Cmd::CreateQuery {
                device: DEVICE,
                query: q,
                kind: 9,
            });
        }
        g.colour([1.0, 0.0, 0.0, 1.0]);
        g.clear(3, 0, 1.0);
        let all = 16 * samples;
        assert_eq!(g.count(0x2000, |g| g.cover(0.25)), all, "{samples}: in front");
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
        g.run(Cmd::ReadSurface { surface: 0x4110, size: 16 }),
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
    for samples in [1u32, 4] {
        let Some(mut g) = Gpu::new(4, samples) else {
            return;
        };
        g.state(ZENABLE, 0);
        g.state(ZWRITEENABLE, 0);
        g.state(ALPHATESTENABLE, 1);
        g.state(ALPHAFUNC, 5); // GREATER
        g.state(ALPHAREF, 0x80);
        g.state(ADAPTIVETESS_Y, atoc);
        // Alpha 0.25: the alpha test (0.25 > 0.5) fails; coverage gives about a quarter.
        g.colour([1.0, 1.0, 1.0, 0.25]);
        g.clear(1, 0, 1.0);
        g.cover(0.5);
        let reds = g.reds();
        if samples == 1 {
            assert_eq!(reds, vec![0; 16], "one sample: the alpha test");
        } else {
            // The hardware's coverage pattern for an alpha is its own: some samples, not all.
            let sum: u32 = reds.iter().map(|&r| u32::from(r)).sum();
            assert!(
                sum > 0 && sum < 16 * 255,
                "four samples: partial coverage, {reds:?}"
            );
        }
        // The switch off: the alpha test again.
        g.state(ADAPTIVETESS_Y, 0);
        g.clear(1, 0, 1.0);
        g.cover(0.5);
        assert_eq!(g.reds(), vec![0; 16], "{samples}: switch off");
    }
}
