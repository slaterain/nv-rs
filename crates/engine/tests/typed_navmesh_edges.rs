//! The differential test of the navmesh edges family (`engine_typed::navmesh_edges`): the rebuild of a
//! navmesh's boundary-edge pairs and preferred vertices (`00699b60`) against its translation.
//!
//! The meshes are built in guest memory as `NavMesh::Load` leaves them: the vertex array (`NVVX`), the
//! triangle array (`NVTR`, bits 13 to 15 of the flags cleared), the pair map and the preferred-vertex
//! array from their own constructors, each already holding entries the rebuild has to empty. The
//! generated meshes are grids of jittered or exact quads (an exact grid puts several vertices at the same
//! distance from a circumcenter, so ties decide), with holes, with edges to another mesh on one side,
//! with corners high enough to fail the height test, one large enough that the maps' buckets hold several
//! items; three more are made by hand for what grids never reach (see `special`). The opt-in test
//! takes the player's own navmeshes.
//!
//! What stands in: the heap (`operator new` and `delete`, the bucket tables, the block reallocation)
//! and the C runtime's square root. The exe's constants and the vtables the code reads are written at
//! their addresses. Everything else runs translated.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{memory_differences_beside_stack, Case, Diff, Input};
use engine_core::typed::{leaf, Family, MAIN_THREAD, OTHER_THREADS};
use engine_typed::dispatch::DISPATCH;
use engine_typed::navmesh_edges::{self, NAVMESH_EDGES};
use engine_typed::pathing::PATHING;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn ret_f(st0: f64) -> Ret {
    Ret {
        st0,
        ..Ret::default()
    }
}

/// The vtables the rebuild and its callees call through, with the exe's slots: the pair map's
/// (`NiTMap<unsigned short, ..>`), the `BSSimpleArray<unsigned short>`'s, the temporary vertex map's,
/// the edge-handle array's and the vertex map base's.
const VTABLES: [(u32, &[u32]); 5] = [
    (
        0x0106_abc4,
        &[
            0x0069_17a0,
            0x0049_c4c0,
            0x0069_1e50,
            0x0069_1e70,
            0x0045_34f0,
            0x0084_4a70,
            0x0069_1f30,
        ],
    ),
    (
        0x0106_abe4,
        &[0x0069_d9a0, 0x0069_1870, 0x006d_b090, 0x0069_18a0],
    ),
    (
        0x0106_b184,
        &[
            0x0069_ad70,
            0x0069_b620,
            0x0069_b680,
            0x0069_b6a0,
            0x0045_34f0,
            0x0069_b760,
            0x0069_b780,
        ],
    ),
    (
        0x0106_b1e0,
        &[0x0069_b840, 0x0069_af80, 0x006d_b090, 0x0069_1d20],
    ),
    (
        0x0106_b1f4,
        &[
            0x0069_b870,
            0x0069_b620,
            0x0069_b680,
            0x0069_b6a0,
            0x0045_34f0,
            0x00ec_60fa,
            0x00ec_60fa,
        ],
    ),
];

/// The exe's doubles the code reads: 0.5, 0, 0.1 and -0.1 (floats widened), 180, the unit length's
/// minimum, and those of the pathing family's helpers.
const DOUBLES: [(u32, u64); 12] = [
    (0x0101_1588, 0x3fe0_0000_0000_0000),
    (0x0101_2060, 0),
    (0x0101_ffa0, 0x3fb9_9999_a000_0000),
    (0x0106_b170, 0xbfb9_9999_a000_0000),
    (0x0104_ed58, 0x4066_8000_0000_0000),
    (0x0101_7cf8, 0x3eb0_c6f7_a000_0000),
    (0x0103_56e8, 0x4033_0000_0000_0000),
    (0x0101_7a40, 0x4059_0000_0000_0000),
    (0x0102_90b0, 0x3fd0_0000_0000_0000),
    (0x0101_e2c0, 0x4049_0000_0000_0000),
    (0x0102_40c0, 0x4050_0000_0000_0000),
    (0x0103_5810, 0x4044_0000_0000_0000),
];

/// The exe's floats: 128 (a triangle's reach), `FLT_MAX` and its negation, 1/3.
const FLOATS: [(u32, u32); 4] = [
    (0x0101_e704, 0x4300_0000),
    (0x0101_6970, 0x7f7f_ffff),
    (0x0101_5f5c, 0xff7f_ffff),
    (0x0102_f200, 0x3eaa_aa9f),
];

/// The exe's constants and vtables, and the stand-ins.
fn stand_ins(e: &mut Engine) {
    for page in [
        0x0101_1000,
        0x0101_2000,
        0x0101_5000,
        0x0101_6000,
        0x0101_7000,
        0x0101_e000,
        0x0101_f000,
        0x0102_4000,
        0x0102_9000,
        0x0102_f000,
        0x0103_5000,
        0x0104_e000,
        0x0106_a000,
        0x0106_b000,
        0x010a_2000,
        0x011c_1000,
        0x011d_e000,
        0x011f_6000,
    ] {
        e.map(page, 0x1000);
    }
    for (at, bits) in DOUBLES {
        e.mem.set_u64(at, bits);
    }
    for (at, bits) in FLOATS {
        e.mem.set_u32(at, bits);
    }
    e.mem.set_u32(0x010a_2720, 4);
    for (vtable, slots) in VTABLES {
        for (k, slot) in (0u32..).zip(slots.iter()) {
            e.mem.set_u32(vtable + 4 * k, *slot);
        }
    }
    // The heap: `operator new` and `delete`, the bucket tables, the block reallocation.
    e.register(0x0040_1000, |e, a| ret(e.mem.alloc(a[0].max(1))));
    e.register(0x0040_1030, |e, a| {
        if a[0] != 0 {
            e.mem.free(a[0]);
        }
        ret(0)
    });
    e.register(0x00aa_1070, |e, a| ret(e.mem.alloc(a[0].max(1))));
    e.register(0x00aa_10f0, |e, a| {
        if a[0] != 0 {
            e.mem.free(a[0]);
        }
        ret(0)
    });
    e.register(0x0042_f5d0, |e, a| {
        let block = e.mem.alloc(a[1].max(1));
        if a[0] != 0 {
            let keep = e.mem.block_size(a[0]).unwrap_or(0).min(a[1]);
            let mut bytes = vec![0u8; keep as usize];
            e.mem.read(a[0], &mut bytes);
            e.mem.write(block, &bytes);
            e.mem.free(a[0]);
        }
        ret(block)
    });
    // The C runtime's square root (its tables aren't mapped here).
    e.register(0x00ec_6054, |_, a| {
        ret_f(f64::from_bits(u64::from(a[0]) | u64::from(a[1]) << 32).sqrt())
    });
}

/// A mesh as `NavMesh::Load` leaves it: `vertices` (x, y, z), `triangles` (three vertices, three links,
/// the flags), one edge-information entry, and the pair map and preferred-vertex array made by their
/// constructors, each holding entries from before.
fn build_mesh(
    e: &mut Engine,
    vertices: &[[f32; 3]],
    triangles: &[([u16; 3], [u16; 3], u32)],
) -> u32 {
    let mesh = e.mem.alloc(0x108);
    e.mem.write(mesh, &[0u8; 0x108]);
    let vdata = e.mem.alloc(12 * vertices.len().max(1) as u32);
    for (i, v) in vertices.iter().enumerate() {
        for (k, f) in v.iter().enumerate() {
            e.mem
                .set_u32(vdata + 12 * i as u32 + 4 * k as u32, f.to_bits());
        }
    }
    let tdata = e.mem.alloc(16 * triangles.len().max(1) as u32);
    for (i, (vs, links, flags)) in triangles.iter().enumerate() {
        let t = tdata + 16 * i as u32;
        for (k, (vertex, link)) in (0u32..).zip(vs.iter().zip(links)) {
            e.mem.set_u16(t + 2 * k, *vertex);
            e.mem.set_u16(t + 6 + 2 * k, *link);
        }
        // `00693bd0(0)`: bits 13 to 15 cleared as the load leaves them.
        e.mem.set_u32(t + 0xc, flags & 0xffff_1fff);
    }
    let idata = e.mem.alloc(12);
    for (offset, data, n) in [
        (0x28, vdata, vertices.len() as u32),
        (0x38, tdata, triangles.len() as u32),
        (0x48, idata, 1),
    ] {
        e.mem.set_u32(mesh + offset + 4, data);
        e.mem.set_u32(mesh + offset + 8, n);
        e.mem.set_u32(mesh + offset + 0xc, n);
    }
    // The pair map (`00691720`, 0x25 buckets) with three entries, and the preferred-vertex array
    // (`006917d0`) with two.
    e.call(0x0069_1720, &[mesh + 0x88, 0x25]);
    for (key, value) in [(7u32, 0x1111), (44, 0x2222), (81, 0x3333)] {
        e.call(0x0069_4c50, &[mesh + 0x88, key, value]);
    }
    e.call(0x0069_17d0, &[mesh + 0x98]);
    let word = e.mem.alloc(4);
    for v in [5u16, 9] {
        e.mem.set_u16(word, v);
        e.call(0x0069_1820, &[mesh + 0x98, word]);
    }
    e.mem.free(word);
    mesh
}

/// A generated mesh: a grid of quads, each two triangles.
#[derive(Clone, Copy)]
struct Grid {
    /// Quads across and up.
    w: u32,
    h: u32,
    /// The quads left out (column, row).
    holes: &'static [(u32, u32)],
    /// Whether the right side's edges lead to another mesh.
    portals: bool,
    /// Whether the vertices move off the grid (else exact: ties between equally far vertices).
    jitter: bool,
    /// Every this-many vertex is raised 250 units (beyond the height limit).
    tall: u32,
    seed: u32,
}

const GRIDS: [Grid; 9] = [
    Grid {
        w: 4,
        h: 3,
        holes: &[(1, 1)],
        portals: true,
        jitter: true,
        tall: 5,
        seed: 1,
    },
    Grid {
        w: 3,
        h: 3,
        holes: &[],
        portals: false,
        jitter: false,
        tall: 0,
        seed: 2,
    },
    Grid {
        w: 5,
        h: 2,
        holes: &[(1, 0), (2, 1), (4, 0)],
        portals: true,
        jitter: true,
        tall: 3,
        seed: 3,
    },
    Grid {
        w: 2,
        h: 2,
        holes: &[(0, 0), (1, 1)],
        portals: false,
        jitter: false,
        tall: 4,
        seed: 4,
    },
    Grid {
        w: 1,
        h: 1,
        holes: &[],
        portals: true,
        jitter: false,
        tall: 0,
        seed: 5,
    },
    Grid {
        w: 6,
        h: 4,
        holes: &[(2, 2), (3, 1)],
        portals: true,
        jitter: true,
        tall: 7,
        seed: 6,
    },
    Grid {
        w: 0,
        h: 0,
        holes: &[],
        portals: false,
        jitter: false,
        tall: 0,
        seed: 7,
    },
    // 111 vertices: the pair map's 37 buckets hold several pairs each (portal-corner vertices 36 and
    // 110 share bucket 36) and the vertex map's buckets several records, so every chain walk runs.
    Grid {
        w: 36,
        h: 2,
        holes: &[(5, 1), (20, 0)],
        portals: true,
        jitter: true,
        tall: 9,
        seed: 8,
    },
    // Exact again, with portals: ties between pair vertices beyond the 1x1 grid.
    Grid {
        w: 3,
        h: 2,
        holes: &[],
        portals: true,
        jitter: false,
        tall: 0,
        seed: 9,
    },
];

fn mix(a: u32, b: u32, seed: u32) -> u32 {
    let mut x =
        a.wrapping_mul(0x9e37_79b1) ^ b.wrapping_mul(0x85eb_ca6b) ^ seed.wrapping_mul(0xc2b2_ae35);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^ (x >> 12)
}

/// The grid's vertices and triangles.
#[allow(clippy::type_complexity)]
fn grid(g: Grid) -> (Vec<[f32; 3]>, Vec<([u16; 3], [u16; 3], u32)>) {
    let mut vertices = Vec::new();
    if g.w > 0 {
        for j in 0..=g.h {
            for i in 0..=g.w {
                let (dx, dy) = if g.jitter {
                    let m = mix(i, j, g.seed);
                    (
                        ((m % 33) as f32 - 16.0) * 1.75,
                        (((m >> 8) % 29) as f32 - 14.0) * 1.25,
                    )
                } else {
                    (0.0, 0.0)
                };
                let raised = g.tall != 0 && (i + 2 * j + g.seed) % g.tall == 0;
                let z = ((i * 7 + j * 13 + g.seed) % 11) as f32 * 4.5
                    + if raised { 250.0 } else { 0.0 }
                    - 1000.0;
                vertices.push([
                    1024.0 + i as f32 * 128.0 + dx,
                    -2048.0 + j as f32 * 128.0 + dy,
                    z,
                ]);
            }
        }
    }
    let quad = |i: i64, j: i64| -> Option<u16> {
        if i < 0 || j < 0 || i >= i64::from(g.w) || j >= i64::from(g.h) {
            return None;
        }
        let (i, j) = (i as u32, j as u32);
        if g.holes.contains(&(i, j)) {
            return None;
        }
        // The index of its first triangle: quads in row order, holes skipped.
        let before = j * g.w + i;
        let skipped = g
            .holes
            .iter()
            .filter(|(hi, hj)| hj * g.w + hi < before)
            .count() as u32;
        Some((2 * (before - skipped)) as u16)
    };
    let v = |i: u32, j: u32| (j * (g.w + 1) + i) as u16;
    let mut triangles = Vec::new();
    for j in 0..g.h {
        for i in 0..g.w {
            let Some(a) = quad(i64::from(i), i64::from(j)) else {
                continue;
            };
            let (ii, jj) = (i64::from(i), i64::from(j));
            let below = quad(ii, jj - 1).map_or(0xffff, |q| q + 1);
            let (right, portal) = match quad(ii + 1, jj) {
                Some(q) => (q + 1, false),
                None if g.portals && i + 1 == g.w => (0, true),
                None => (0xffff, false),
            };
            let above = quad(ii, jj + 1).unwrap_or(0xffff);
            let left = quad(ii - 1, jj).unwrap_or(0xffff);
            let other = if mix(i, j, g.seed) % 3 == 0 {
                0x0800
            } else {
                0x0040
            };
            triangles.push((
                [v(i, j), v(i + 1, j), v(i + 1, j + 1)],
                [below, right, a + 1],
                other | if portal { 0x2 } else { 0 } | 0xe000,
            ));
            triangles.push((
                [v(i, j), v(i + 1, j + 1), v(i, j + 1)],
                [a, above, left],
                other,
            ));
        }
    }
    (vertices, triangles)
}

/// One mesh of the test: vertices and triangles (three vertices, three links, the flags).
type TestMesh = (Vec<[f32; 3]>, Vec<([u16; 3], [u16; 3], u32)>);

/// An isolated triangle whose edge 0 leads to another mesh (link 0, flag bit 0): its first two
/// vertices each get one ordinary and one portal edge, so both become pairs whatever their corner.
fn portal_triangle(m: &mut TestMesh, corners: [[f32; 3]; 3]) {
    let first = m.0.len() as u16;
    m.0.extend(corners);
    m.1.push(([first, first + 1, first + 2], [0, 0xffff, 0xffff], 0x1));
}

/// An isolated triangle with no portal: a corner becomes a pair only by the corner test.
fn plain_triangle(m: &mut TestMesh, corners: [[f32; 3]; 3]) {
    let first = m.0.len() as u16;
    m.0.extend(corners);
    m.1.push(([first, first + 1, first + 2], [0xffff; 3], 0));
}

/// A sharp corner of `angle` radians at `at`, wound as the grids are, so its two edges pass the
/// corner test when the sine is below 0.1.
fn sharp_corner(m: &mut TestMesh, at: [f32; 2], angle: f32) {
    let [x, y] = at;
    plain_triangle(
        m,
        [
            [x, y, 0.0],
            [x + 300.0, y, 0.0],
            [x + 300.0 * angle.cos(), y + 300.0 * angle.sin(), 0.0],
        ],
    );
}

/// The meshes that are not grids, each reaching what the grids don't:
/// 0. two triangles linked across an edge they both run the same way: their boundary edges at the
///    shared vertices don't chain, so pass 2 leaves them out;
/// 1. a right triangle whose circumcenter has exactly 180 between it and a portal vertex placed
///    under it and a second 180 above it (a NaN height would take the C runtime's error path in
///    `fabs`, which needs the kernel), a triangle with a NaN corner (a NaN circumcenter), corners
///    whose sines sit near the corner test's 0.1, a zero-area triangle and one with two corners at
///    the same point (a zero edge);
/// 2. a tiny triangle far from every pair, beyond its reach: no preferred vertex.
fn special(k: u32) -> TestMesh {
    let mut m: TestMesh = (Vec::new(), Vec::new());
    match k {
        0 => {
            m.0 = vec![
                [0.0, 0.0, 0.0],
                [100.0, 0.0, 0.0],
                [50.0, 80.0, 0.0],
                [50.0, -80.0, 0.0],
            ];
            m.1 = vec![
                ([0, 1, 2], [1, 0xffff, 0xffff], 0),
                ([0, 1, 3], [0, 0xffff, 0xffff], 0),
            ];
        }
        1 => {
            plain_triangle(
                &mut m,
                [[0.0, 0.0, 20.0], [100.0, 0.0, 200.0], [0.0, 100.0, 20.0]],
            );
            portal_triangle(
                &mut m,
                [[50.0, 50.0, 20.0], [50.0, 50.0, 380.0], [400.0, 90.0, 20.0]],
            );
            portal_triangle(
                &mut m,
                [
                    [f32::NAN, 600.0, 0.0],
                    [700.0, 600.0, 0.0],
                    [650.0, 700.0, 0.0],
                ],
            );
            for (k, sine) in [0.05f32, 0.09, 0.099, 0.101, 0.11].into_iter().enumerate() {
                sharp_corner(&mut m, [-2000.0 - 400.0 * k as f32, 0.0], sine.asin());
                // The same sine below the axis, past a right angle: the other branch and limit.
                sharp_corner(
                    &mut m,
                    [-2000.0 - 400.0 * k as f32, 800.0],
                    sine.asin() - std::f32::consts::PI,
                );
            }
            plain_triangle(
                &mut m,
                [[0.0, 3000.0, 0.0], [10.0, 3000.0, 0.0], [20.0, 3000.0, 0.0]],
            );
            plain_triangle(
                &mut m,
                [
                    [500.0, 3000.0, 0.0],
                    [500.0, 3000.0, 0.0],
                    [600.0, 3100.0, 0.0],
                ],
            );
        }
        _ => {
            portal_triangle(
                &mut m,
                [[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 100.0, 0.0]],
            );
            plain_triangle(
                &mut m,
                [
                    [100_000.0, 0.0, 0.0],
                    [100_001.0, 0.0, 0.0],
                    [100_000.5, 0.8, 0.0],
                ],
            );
        }
    }
    m
}

/// The meshes the generated tests run: the grids, then the special ones.
const MESH_COUNT: u32 = GRIDS.len() as u32 + 3;

fn mesh(k: u32) -> TestMesh {
    match GRIDS.get(k as usize) {
        Some(g) => grid(*g),
        None => special(k - GRIDS.len() as u32),
    }
}

/// The comparison's common part: the families play has on, the callees the native code does in its
/// body, the translation's plain calls the typed helpers make by address, the two locals.
trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&NAVMESH_EDGES)
            .family(&PATHING)
            .family(&leaf::LEAVES)
            .family(&DISPATCH)
            // The pair map's walk and each pair's point, difference and squared length (pass 3), and
            // the point constructor and differences the native code writes in place.
            .ignore_calls(&[
                0x0069_4d30,
                0x004b_9ba0,
                0x0068_f0a0,
                0x0043_9ef0,
                0x0059_5c80,
                0x0041_6870,
                // The native square root takes `FSQRT` where the translation calls the runtime's.
                0x00ec_6054,
                // An array's 12-byte entry (pass 2's edges, every vertex point), read in place.
                0x006a_1440,
            ])
            // The pathing family's root, absolute value and edge vertices call these by address; their
            // translations plainly.
            .translation_calls_plainly(&[0x0040_19d0, 0x0040_8860, 0x0068_efd0])
            // Pass 3's two locals: the planar difference and the chosen vertex.
            .local_at(0x0058_9850, 0, 12)
            .local_at(0x0069_1820, 1, 2)
            .control_words(&[OTHER_THREADS, MAIN_THREAD])
            .max_calls(200_000)
    }
}

#[test]
fn rebuild_edge_pairs() {
    const MESHES: [u32; MESH_COUNT as usize] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    Diff::new(entry!(
        0x0069_9b60,
        navmesh_edges::rebuild_edge_pairs(Ptr<_>)
    ))
    .common()
    .inputs(&[Input::Of(&MESHES)])
    .setup(|e, case: &Case| {
        stand_ins(e);
        let (vertices, triangles) = mesh(case.word(0));
        vec![build_mesh(e, &vertices, &triangles)]
    })
    .run()
    .assert_same();
}

/// What the rebuild left in one mesh, for the opt-in test's report: the pairs and the preferred
/// vertices that are not `0xffff`.
fn outcome(e: &Engine, mesh: u32) -> (u32, u32) {
    let pairs = e.mem.u32(mesh + 0x88 + 0xc);
    let data = e.mem.u32(mesh + 0x98 + 4);
    let n = e.mem.u32(mesh + 0x98 + 8);
    let chosen = (0..n).filter(|i| e.mem.u16(data + 2 * i) != 0xffff).count() as u32;
    (pairs, chosen)
}

/// The families play has on beside this one.
const CALLEES: [&Family; 3] = [&PATHING, &leaf::LEAVES, &DISPATCH];
/// Those and this one.
const ALL: [&Family; 4] = [&NAVMESH_EDGES, &PATHING, &leaf::LEAVES, &DISPATCH];

/// One mesh run whole on two engines from the same state, with families `a` on one and `b` on the
/// other, under both control words: the heap and every byte beside the stack for locals must match
/// after the call. The pairs and preferred vertices found, or what differed.
fn both_ways(
    vertices: &[[f32; 3]],
    triangles: &[([u16; 3], [u16; 3], u32)],
    a: &[&Family],
    b: &[&Family],
) -> Result<(u32, u32), String> {
    let mut result = (0, 0);
    for word in [OTHER_THREADS, MAIN_THREAD] {
        let mut sides = Vec::new();
        let mut after = Vec::new();
        for families in [a, b] {
            let mut e = Engine::new_untyped_for_test();
            stand_ins(&mut e);
            let mesh = build_mesh(&mut e, vertices, triangles);
            e.with_stack(4, |_, _| ());
            if !families.is_empty() {
                e.typed_apply_for_test(families, "");
            }
            // `FLDCW` as the game does it: the thread's word too, which `fabs` saves and restores
            // through `_ctrlfp` (with the helpers' word alone it "restores" the default).
            engine_core::units::crt::set_x87_control_word(&mut e, word);
            e.call(0x0069_9b60, &[mesh]);
            after.push(engine_core::x87::control_word());
            engine_core::units::crt::set_x87_control_word(
                &mut e,
                engine_core::x87::DEFAULT_CONTROL_WORD,
            );
            sides.push((e, mesh));
        }
        if after.iter().any(|&w| w != word) {
            return Err(format!(
                "the control word {word:04x} did not hold: {after:04x?}"
            ));
        }
        let ((a, mesh), (b, _)) = (&sides[0], &sides[1]);
        if (a.mem.heap_top(), a.mem.heap_available()) != (b.mem.heap_top(), b.mem.heap_available())
        {
            return Err(format!("the heap, control word {word:04x}"));
        }
        let differences = memory_differences_beside_stack(a, b, 8);
        if !differences.is_empty() {
            let words: Vec<(u32, u32)> = differences
                .iter()
                .map(|&at| (a.mem.u32(at & !3), b.mem.u32(at & !3)))
                .collect();
            let blocks: Vec<u32> = [0x2c, 0x3c, 0x90, 0x9c]
                .iter()
                .map(|&k| a.mem.u32(mesh + k))
                .collect();
            return Err(format!(
                "memory differs at {differences:08x?} ({words:08x?}; mesh at {mesh:08x}, vertices, \
                 triangles, pair buckets, preferred at {blocks:08x?}), control word {word:04x}"
            ));
        }
        result = outcome(a, *mesh);
    }
    Ok(result)
}

/// The translations only against every family on (play), and the other families on against every
/// family on (this one alone differs).
#[test]
fn generated_meshes_both_ways() {
    for k in 0..MESH_COUNT {
        let (vertices, triangles) = mesh(k);
        for (a, what) in [
            (&[][..], "translations only"),
            (&CALLEES[..], "the callees typed"),
        ] {
            if let Err(why) = both_ways(&vertices, &triangles, a, &ALL) {
                panic!("mesh {k} against {what}: {why}");
            }
        }
    }
}

/// A `NAVM` record's mesh: `NVVX` (12 bytes a vertex) and `NVTR` (16 bytes a triangle).
#[allow(clippy::type_complexity)]
fn record_mesh(record: &esm::Record) -> Option<(Vec<[f32; 3]>, Vec<([u16; 3], [u16; 3], u32)>)> {
    let vx = &record.get(esm::FourCC(*b"NVVX"))?.data;
    let tr = &record.get(esm::FourCC(*b"NVTR"))?.data;
    let f = |b: &[u8], at: usize| f32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    let h = |b: &[u8], at: usize| u16::from_le_bytes(b[at..at + 2].try_into().unwrap());
    let vertices = (0..vx.len() / 12)
        .map(|i| [f(vx, 12 * i), f(vx, 12 * i + 4), f(vx, 12 * i + 8)])
        .collect();
    let triangles = (0..tr.len() / 16)
        .map(|i| {
            let t = 16 * i;
            (
                [h(tr, t), h(tr, t + 2), h(tr, t + 4)],
                [h(tr, t + 6), h(tr, t + 8), h(tr, t + 10)],
                u32::from_le_bytes(tr[t + 12..t + 16].try_into().unwrap()),
            )
        })
        .collect();
    Some((vertices, triangles))
}

/// `NV_DATA="...\Data" cargo test -p engine --test typed_navmesh_edges -- --ignored real_navmeshes`:
/// the player's own navmeshes, `FalloutNV.esm`'s and those of the plugins `NV_NAVMESH_PLUGINS` names
/// (comma-separated file names in `NV_DATA`), up to `NV_NAVMESH_LIMIT` a plugin (default 150, spread
/// over the file, the largest five always in), each both ways under both control words; those of up
/// to `NV_NAVMESH_DIFF_TRIANGLES` triangles (default 48) also through the call-for-call comparison.
/// Nothing of the game's is stored.
#[test]
#[ignore = "needs the player's FalloutNV.esm (set NV_DATA)"]
fn real_navmeshes() {
    use esm::{FourCC, Plugin};
    let data = std::path::PathBuf::from(std::env::var("NV_DATA").expect("NV_DATA"));
    let limit: usize = std::env::var("NV_NAVMESH_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(150);
    let diff_triangles: usize = std::env::var("NV_NAVMESH_DIFF_TRIANGLES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(48);
    let plugins = std::env::var("NV_NAVMESH_PLUGINS").unwrap_or_else(|_| "FalloutNV.esm".into());
    let mut small = Vec::new();
    let mut failures = Vec::new();
    let mut play_differs = 0u32;
    for name in plugins.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        let plugin = Plugin::open(data.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let entries: Vec<_> = plugin.records_of_type(FourCC(*b"NAVM")).collect();
        let mut meshes = Vec::new();
        for entry in &entries {
            let Ok(record) = plugin.record(entry) else {
                continue;
            };
            if let Some(mesh) = record_mesh(&record) {
                meshes.push((record.header.form_id.0, mesh));
            }
        }
        let mut chosen: Vec<usize> = (0..meshes.len())
            .step_by((meshes.len() / limit.max(1)).max(1))
            .collect();
        let mut by_size: Vec<usize> = (0..meshes.len()).collect();
        by_size.sort_by_key(|&i| std::cmp::Reverse(meshes[i].1 .1.len()));
        chosen.extend(by_size.into_iter().take(5));
        chosen.sort_unstable();
        chosen.dedup();
        // `NV_NAVMESH_ONLY=<form id in hex>`: that navmesh alone.
        if let Some(only) = std::env::var("NV_NAVMESH_ONLY")
            .ok()
            .and_then(|v| u32::from_str_radix(v.trim(), 16).ok())
        {
            chosen = (0..meshes.len()).filter(|&i| meshes[i].0 == only).collect();
        }
        let (mut pairs, mut preferred, mut triangles) = (0u64, 0u64, 0u64);
        for &i in &chosen {
            let (id, (v, t)) = &meshes[i];
            // Play (every family on) against the translations only. Where they differ, this family
            // alone against the other families on: if that matches, the difference is the other
            // families' native floats (their last bits on main), which the translated rebuild also
            // meets when it calls them; if not, it is this family's.
            let (p, c) = match both_ways(v, t, &[], &ALL) {
                Ok(found) => found,
                Err(why) => match both_ways(v, t, &CALLEES, &ALL) {
                    Ok(found) => {
                        eprintln!(
                            "{name} {id:08x}: play differs from the translations only ({why}); \
                             this family alone matches"
                        );
                        play_differs += 1;
                        found
                    }
                    Err(alone) => {
                        failures.push(format!(
                            "{name} {id:08x}: {why}; this family alone: {alone}"
                        ));
                        // And call for call, whatever its size: the first call that differs.
                        small.push((*id, v.clone(), t.clone()));
                        continue;
                    }
                },
            };
            pairs += u64::from(p);
            preferred += u64::from(c);
            triangles += t.len() as u64;
            if t.len() <= diff_triangles && small.len() < 64 {
                small.push((*id, v.clone(), t.clone()));
            }
        }
        eprintln!(
            "{name}: {} navmeshes of {} compared both ways ({triangles} triangles, {pairs} edge \
             pairs, {preferred} preferred vertices found)",
            chosen.len(),
            meshes.len()
        );
    }
    eprintln!("{play_differs} navmeshes differ in play from the translations only");
    assert!(
        !small.is_empty(),
        "no small navmesh for the call-for-call comparison"
    );
    let count = small.len();
    let small: &'static [_] = small.leak();
    let indices: &'static [u32] = (0..count as u32).collect::<Vec<_>>().leak();
    Diff::new(entry!(
        0x0069_9b60,
        navmesh_edges::rebuild_edge_pairs(Ptr<_>)
    ))
    .common()
    .inputs(&[Input::Of(indices)])
    .max_cases(count)
    .setup(move |e, case: &Case| {
        stand_ins(e);
        let (_, v, t) = &small[case.word(0) as usize];
        vec![build_mesh(e, v, t)]
    })
    .run()
    .assert_same();
    eprintln!("{count} small real navmeshes compared call for call");
    assert!(failures.is_empty(), "{failures:#?}");
}
