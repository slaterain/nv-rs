//! The differential test of every typed function of the shader family (`engine_typed::shader`)
//! against its translation. One world serves every test: a renderer with its state cache and device,
//! a texture stage with its cache, a shader, plain and skinned geometry with their properties, passes,
//! a batch renderer with a pass map, groups, holders and an active list, the node pool, an accumulator,
//! a constant map, a light list. Each test varies the words and floats its function reads. The shadow
//! light shader's set-ups add their own on top ([`geometry_shape`], made from a [`Shape`]): twelve
//! lights, the rendering object, the camera, the scratch rows and the exe's constants, with every
//! float a set-up reads taken from a seed the float sweep moves.
//!
//! What stands in: every virtual function the code calls on a device, shader, geometry, skin, group,
//! map or property (test functions past `.text` answering a word per slot from a table), and the
//! callees outside the family: the map's `GetAt` and `SetAt`, the node and memory allocators and
//! frees, the lock, the shadow-pass test, the shaders' own set-ups, the renderer's skinned test, the
//! D3DX helpers, `atexit` and the interlocked imports. The render state cache's `SetRenderState` and
//! the batch renderer's `AddPass` are the real (family) functions.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::{entry, Engine};
use engine_typed::shader::{
    self, accumulator as acc, batch, constants, geometry, lighting, state, textures,
};
use engine_typed::{crt, culling, dispatch, pathing, render};

/// The stand-in virtual functions, one per slot, and the table of what each answers.
const VT_FN: u32 = 0x00ff_2000;
const RETS: u32 = 0x0300_0000;
/// The stand-in of the pass map's hash (its vtable's slot 4).
const MAP_HASH: u32 = 0x00ff_2800;
/// The stand-in of the pass map's key compare (its vtable's slot 8), for the tests that name the
/// dispatch family: the map's lookup (`00853130`) is then the real one and walks the map's chains.
const MAP_EQUAL: u32 = 0x00ff_2804;
const MAX_CALLS: usize = 5_000;
const FLOAT_SWEEP: usize = 48;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

fn slot_answer<const S: u32>(e: &mut Engine, _: &[u32]) -> Ret {
    ret(e.mem.u32(RETS + S))
}

macro_rules! slots {
    ($e:ident; $($s:literal),* $(,)?) => { $( $e.register(VT_FN + $s, slot_answer::<$s>); )* };
}

fn set(e: &mut Engine, at: u32, value: u32) {
    e.mem.set_u32(at, value);
}

fn bits(case: &Case, i: usize) -> u32 {
    case.f32(i).to_bits()
}

/// A vtable of 0x200 bytes of stand-ins, with `real` slots pointing at real functions.
fn vtable(e: &mut Engine, real: &[(u32, u32)]) -> u32 {
    let vt = e.mem.alloc(0x200);
    for slot in (0..0x200).step_by(4) {
        let target = real
            .iter()
            .find(|(s, _)| *s == slot)
            .map_or(VT_FN + slot, |(_, a)| *a);
        set(e, vt + slot, target);
    }
    vt
}

/// An object of `size` bytes with vtable `vt`.
fn obj(e: &mut Engine, size: u32, vt: u32) -> u32 {
    let o = e.mem.alloc(size);
    set(e, o, vt);
    o
}

/// The callees outside the family; the plain geometry pass (`00b7dde0`) too unless the test names the
/// render family, which holds it.
fn stand_ins(e: &mut Engine, render: bool) {
    stand_ins_named(e, render, false);
}

/// [`stand_ins`] for a test that names the render family (`render`: the plain geometry pass and the
/// renderer's skinned test are its own) or the dispatch family (`dispatch`: the node pool's lock is).
fn stand_ins_named(e: &mut Engine, render: bool, dispatch: bool) {
    stand_ins_all(e, render, dispatch, true);
}

/// [`stand_ins_named`], with the renderer's partition draw (`00e6d310`) stood in or not: a test that names the
/// draw family, which holds it, has the real one ([`draw_world`]).
fn stand_ins_all(e: &mut Engine, render: bool, dispatch: bool, partition_draw: bool) {
    e.map(RETS, 0x1000);
    slots!(e; 0x04, 0x08, 0x24, 0x30, 0x34, 0x64, 0x6c, 0x78, 0x7c, 0x80, 0x84, 0x88, 0x8c, 0xa0,
        0xa4, 0xa8, 0xac, 0xb0, 0xc0, 0xcc, 0xdc, 0xe0, 0xe4, 0xf0, 0xf4, 0xf8, 0xfc, 0x100, 0x104,
        0x10c, 0x110, 0x114, 0x15c, 0x170, 0x178, 0x1ac);
    // The pass map: `GetAt(map, key, out)` finds keys below 0x200, value `key & 3`. The lookup is a
    // member of the dispatch family, so a test that names it gets the real lookup over the world's
    // map (its hash and compare stand-ins below) instead.
    if !dispatch {
        e.register(0x0085_3130, |e, a| {
            e.mem.set_u32(a[2], a[1] & 3);
            ret(u32::from(a[1] < 0x200))
        });
    }
    e.register(0x0084_4700, |e, a| {
        let n = e.mem.u32(a[0] + 0xc);
        e.mem.set_u32(a[0] + 0xc, n + 1);
        ret(0)
    });
    // The node allocator, memory manager, operator new, frees and clears.
    e.register(0x0043_a010, |e, _| ret(e.mem.alloc(0xc)));
    e.register(0x00aa_3e40, |e, a| ret(e.mem.alloc(a[1].max(4))));
    e.register(0x00aa_13e0, |e, a| ret(e.mem.alloc(a[0].max(4))));
    e.register(0x00aa_4060, |_, _| ret(0));
    e.register(0x00b6_4ec0, |_, _| ret(0));
    // The node pool's lock: taken once more.
    if !dispatch {
        e.register(0x0040_fbf0, |e, _| {
            let n = e.mem.u32(0x011c_5f84);
            e.mem.set_u32(0x011c_5f84, n + 1);
            e.mem.set_u32(0x011c_5f80, 1);
            ret(0)
        });
    }
    // The shadow-pass test: odd types.
    e.register(0x00b6_30f0, |_, a| ret(a[0] & 1));
    if !render {
        e.register(0x00b7_dde0, |_, _| ret(0));
    }
    for addr in [
        0x00b7_ded0,
        0x00b7_dfe0,
        0x00b7_e150,
        0x00b7_dd50,
        0x00b7_deb0,
        0x00b7_ac20,
        0x00bb_96d0,
        0x00b7_a870,
        0x00b6_3320,
        0x00b9_8070,
        0x00ec_658f,
    ] {
        e.register(addr, |_, _| ret(0));
    }
    if partition_draw {
        e.register(0x00e6_d310, |_, _| ret(0));
    }
    // The renderer's skinned test: the geometry has a skin.
    if !render {
        e.register(0x00e7_2c20, |e, a| {
            ret(u32::from(e.mem.u32(a[1] + 0xbc) != 0))
        });
    }
    e.register(0x00b5_5560, |e, _| ret(e.mem.u32(RETS + 0x1f0)));
    // The D3DX helpers: a mark into the out word.
    for addr in [0x00ee_6dfe, 0x00ee_6df2, 0x00ee_6dec] {
        e.register(addr, |e, a| {
            e.mem.set_u32(a[0], 0x3f80_0001);
            ret(a[0])
        });
    }
    e.register(0x00fd_f1c0, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(0x00fd_f1c4, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    // The pass map's hash: the key modulo the bucket count.
    e.register(MAP_HASH, |e, a| {
        let size = e.mem.u32(a[0] + 4);
        ret(if size == 0 { 0 } else { a[1] % size })
    });
    // Its key compare: equal keys.
    e.register(MAP_EQUAL, |_, a| ret(u32::from(a[1] == a[2])));
    // The active list's pass-type comparison.
    e.register(0x00b9_88a0, |_, a| {
        ret((a[0].wrapping_sub(a[1]) as i32).signum() as u32)
    });
    for (addr, len) in [
        (0x0101_0000, 0x10000),
        (0x0106_b000, 0x3000),
        (0x011a_9000, 0x6000),
        (0x011b_f000, 0x1000),
        (0x011c_5000, 0x1000),
        (0x011f_4000, 0xc000),
        (0x0126_f000, 0x1000),
    ] {
        e.map(addr, len);
    }
}

/// The world's objects.
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct W {
    renderer: u32,
    cache: u32,
    device: u32,
    stage_cache: u32,
    stage: u32,
    shader: u32,
    geometry: u32,
    skinned: u32,
    skin: u32,
    property: u32,
    alpha: u32,
    data: u32,
    passes: [u32; 4],
    skin_pass: u32,
    br: u32,
    holders: [u32; 13],
    list: u32,
    acc: u32,
    camera: u32,
    cmap: u32,
    light: u32,
    ssn: u32,
    transform: u32,
}

/// A pass list at `at` with the items `items` and one spare node.
fn list_at(e: &mut Engine, at: u32, items: &[u32]) {
    let mut prev = 0;
    for item in items {
        let node = e.mem.alloc(0xc);
        set(e, node + 4, prev);
        set(e, node + 8, *item);
        if prev != 0 {
            set(e, prev, node);
        } else {
            set(e, at + 4, node);
        }
        prev = node;
    }
    set(e, at + 8, prev);
    let spare = e.mem.alloc(0xc);
    set(e, at + 0xc, spare);
    set(e, at + 0x10, items.len() as u32);
}

fn world(e: &mut Engine) -> W {
    world_with(e, false)
}

/// [`world`] for a test that names the render family: its geometry pass stays real.
fn world_with(e: &mut Engine, render: bool) -> W {
    world_named(e, render, false)
}

/// [`world`] for a test that names the render or the dispatch family ([`stand_ins_named`]).
fn world_named(e: &mut Engine, render: bool, dispatch: bool) -> W {
    world_all(e, render, dispatch, true)
}

/// [`world_named`] with the partition draw stood in or not ([`stand_ins_all`]).
fn world_all(e: &mut Engine, render: bool, dispatch: bool, partition_draw: bool) -> W {
    stand_ins_all(e, render, dispatch, partition_draw);
    let vt = vtable(e, &[]);
    let cache_vt = vtable(e, &[(0x68, 0x00e8_8780)]);
    let br_vt = vtable(e, &[(0x8c, 0x00b9_9c90)]);
    let device = obj(e, 0x10, vt);
    let cache = obj(e, 0x1114, cache_vt);
    set(e, cache + 0x10f8, device);
    for i in 0..8 {
        set(e, cache + 0xd4 + 4 * i, 0x100 + i);
    }
    let renderer = obj(e, 0xa40, vt);
    set(e, renderer + 0x288, device);
    let manager = obj(e, 0x10, vt);
    let group_obj = obj(e, 0x10, vt);
    set(e, renderer + 0x8ac, manager);
    set(e, renderer + 0x8b0, group_obj);
    set(e, renderer + 0x8b8, cache);
    for (i, w) in [ONE, HALF, TWO, 0x3eaa_aaab, ONE, 0xbf80_0000]
        .iter()
        .enumerate()
    {
        set(e, renderer + 0x634 + 4 * i as u32, *w);
    }
    set(e, 0x011f_9508, renderer);
    set(e, 0x011f_4748, renderer);
    let stage_cache = e.mem.alloc(0xb8);
    let stage = e.mem.alloc(0x14);
    set(e, stage + 4, 1);
    set(e, stage + 0x10, stage_cache);
    let target = obj(e, 0x10, vt);
    set(e, 0x0126_f99c, target);
    // The slot tables.
    for s in 0..0x20u32 {
        e.mem.set_u16(0x0126_f958 + 2 * s, (s % 10) as u16);
    }
    for s in 0..0x16u32 {
        e.mem.set_u16(0x0126_f92c + 2 * s, (s % 7) as u16);
    }
    for i in 0..8u32 {
        e.mem.set_u16(0x0126_f948 + 2 * i, (0x10 + i) as u16);
        set(e, 0x0126_f0b0 + 8 * i, 0x31 + i);
        set(e, 0x0126_f0b4 + 8 * i, 0x41 + i);
        for k in 0..3 {
            set(e, 0x0126_f050 + 12 * i + 4 * k, 0x51 + 0x10 * k + i);
        }
    }
    for i in 0..5u32 {
        set(e, 0x0126_f918 + 4 * i, 0x20 + i);
    }
    set(e, 0x011b_f558, 4);
    // The setters' counter indices (by state) and the function and blend tables.
    for s in 0..0xb0u32 {
        set(e, 0x011f_fa30 + 4 * s, s % 8);
    }
    for i in 0..8u32 {
        set(e, 0x011a_d888 + 4 * i, 0x200 + i);
    }
    for i in 0..16u32 {
        set(e, 0x011a_d8a8 + 4 * i, 0x300 + i);
    }
    // A shader: kind at +0x68, two objects at +0x30 and +0x34.
    let shader = obj(e, 0x80, vt);
    let first = obj(e, 0x10, vt);
    let second = obj(e, 0x10, vt);
    set(e, shader + 0x30, first);
    set(e, shader + 0x34, second);
    set(e, RETS + 0x1f0, shader);
    // Properties: alpha (blends, bit 9), shade (both bits 10 and 11), the shader property.
    let alpha = obj(e, 0x1c, vt);
    e.mem.set_u16(alpha + 0x18, 0x0201 | (5 << 1) | (9 << 5));
    let shade = obj(e, 0x1c, vt);
    e.mem.set_u16(shade + 0x18, 0xc00);
    let property = obj(e, 0x100, vt);
    set(e, property + 0x20, 0x40080);
    set(e, property + 0x24, 0x20);
    set(e, property + 0x28, ONE);
    set(e, property + 0x2c, ONE);
    let textures = e.mem.alloc(0x10);
    set(e, textures + 4, 0x7777);
    set(e, property + 0xb0, textures);
    set(e, 0x011f_9518, 0x5518);
    set(e, 0x011f_9510, 0x5510);
    // Geometry: bound, data, the skin.
    let bound = e.mem.alloc(0x10);
    for (i, w) in [ONE, TWO, 0x4040_0000, HALF].iter().enumerate() {
        set(e, bound + 4 * i as u32, *w);
    }
    let data = e.mem.alloc(0x38);
    e.mem.set_u16(data + 0xe, 0xabcd);
    let geometry = obj(e, 0xc4, vt);
    set(e, geometry + 0x20, bound);
    set(e, geometry + 0x9c, alpha);
    set(e, geometry + 0xa8, property);
    set(e, geometry + 0xac, shade);
    set(e, geometry + 0xb8, data);
    set(e, geometry + 0xc0, shader);
    let skin = obj(e, 0x40, vt);
    let partitions = e.mem.alloc(0x10);
    set(e, partitions + 8, 3);
    let parts = e.mem.alloc(0x84);
    set(e, partitions + 0xc, parts);
    set(e, skin + 0xc, partitions);
    let flags = e.mem.alloc(0xc);
    for (i, b) in [1u8, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0].iter().enumerate() {
        e.mem.set_u8(flags + i as u32, *b);
    }
    set(e, skin + 0x38, flags);
    e.mem.set_u8(skin + 0x3c, 1);
    let skinned = obj(e, 0xc4, vt);
    for off in [0x20, 0x9c, 0xa8, 0xac, 0xb8, 0xc0] {
        let w = e.mem.u32(geometry + off);
        set(e, skinned + off, w);
    }
    set(e, skinned + 0xbc, skin);
    let pass_of = |e: &mut Engine, geometry: u32, pass_type: u16, kind: u8, flag: u8| {
        let p = e.mem.alloc(8);
        set(e, p, geometry);
        e.mem.set_u16(p + 4, pass_type);
        e.mem.set_u8(p + 6, kind);
        e.mem.set_u8(p + 7, flag);
        p
    };
    let passes = [
        pass_of(e, geometry, 0x10, 2, 1),
        pass_of(e, geometry, 0x10, 1, 1),
        pass_of(e, geometry, 0x11, 4, 0),
        pass_of(e, geometry, 0xfc, 0, 1),
    ];
    let skin_pass = pass_of(e, skinned, 0x10, 3, 1);
    // The batch renderer: four groups, a map with three keys, an active list, holders.
    let br = obj(e, 0x98, br_vt);
    let groups = e.mem.alloc(4 * 0x68);
    set(e, br + 8, groups);
    let map_vt = if dispatch {
        vtable(e, &[(4, MAP_HASH), (8, MAP_EQUAL)])
    } else {
        vtable(e, &[(4, MAP_HASH)])
    };
    set(e, br + 0xc, map_vt);
    set(e, br + 0x10, 4);
    let buckets = e.mem.alloc(0x10);
    let item2 = e.mem.alloc(0xc);
    set(e, item2 + 4, 0x15);
    set(e, item2 + 8, 2);
    let item1 = e.mem.alloc(0xc);
    set(e, item1, item2);
    set(e, item1 + 4, 0x11);
    set(e, item1 + 8, 1);
    set(e, buckets + 4, item1);
    let item3 = e.mem.alloc(0xc);
    set(e, item3 + 4, 0x13);
    set(e, buckets + 0xc, item3);
    set(e, br + 0x14, buckets);
    set(e, br + 0x18, 2);
    set(e, br + 0x20, 0x10);
    set(e, br + 0x24, 0x30);
    e.mem.set_u16(br + 0x2c, 0x10);
    let entry2 = e.mem.alloc(8);
    e.mem.set_u16(entry2, 0x11);
    set(e, br + 0x30, entry2);
    e.mem.set_u8(br + 0x38, 1);
    list_at(e, groups + 0x68, &[passes[0], passes[1]]);
    list_at(e, groups + 0x68 + 2 * 0x14, &[passes[2]]);
    set(e, groups + 0x68 + 0x64, 3);
    list_at(e, groups, &[skin_pass]);
    set(e, groups + 0x64, 1);
    let mut holders = [0u32; 13];
    for (i, h) in holders.iter_mut().enumerate() {
        *h = e.mem.alloc(0x20);
        e.mem.set_u8(*h, (i & 1) as u8);
        e.mem.set_u16(*h + 2, 2);
        set(e, *h + 4, 4);
        set(e, *h + 8, br);
        list_at(e, *h + 0xc, &[passes[i % 4]]);
        set(e, br + 0x3c + 4 * i as u32, *h);
    }
    for i in 0..8u32 {
        set(e, br + 0x70 + 4 * i, holders[i as usize]);
    }
    set(e, br + 0x90, holders[3]);
    set(e, br + 0x94, holders[4]);
    // The node pool: two free nodes.
    let free = e.mem.alloc(0xc);
    let free2 = e.mem.alloc(0xc);
    set(e, free, free2);
    set(e, 0x011c_5f58, free);
    let list = e.mem.alloc(0x14);
    list_at(e, list, &[passes[0], passes[2], passes[1]]);
    // The accumulator: a camera, a sorted pair, the geometry list inline, the batch renderers.
    let acc = obj(e, 0x1ac, vt);
    set(e, acc + 8, 1);
    let (n1, n2) = (e.mem.alloc(0xc), e.mem.alloc(0xc));
    set(e, n1, n2);
    set(e, n1 + 8, geometry);
    set(e, n2 + 4, n1);
    set(e, n2 + 8, skinned);
    set(e, acc + 0xc, n1);
    set(e, acc + 0x10, n2);
    set(e, acc + 0x14, 2);
    set(e, acc + 0x1c, 2);
    let sorted = e.mem.alloc(8);
    set(e, sorted, geometry);
    set(e, sorted + 4, geometry);
    set(e, acc + 0x24, sorted);
    e.mem.set_u16(acc + 0x1a8, 0x10);
    set(e, acc + 0x174, br);
    set(e, acc + 0x178, br);
    set(e, acc + 0x18c, 2);
    let tls = e.tls();
    set(e, tls + 0x2bc, 1);
    set(e, 0x011f_9f80, 0x00b6_4020);
    let render_passes = e.mem.alloc(0x14);
    let pass_array = e.mem.alloc(0x10);
    for (i, p) in passes.iter().enumerate() {
        set(e, pass_array + 4 * i as u32, *p);
    }
    set(e, render_passes + 4, pass_array);
    set(e, render_passes + 0x10, 4);
    set(e, RETS + 0xa0, render_passes);
    set(e, RETS + 0xa8, render_passes);
    set(e, RETS + 0x6c, 1);
    // The alpha sort's camera: a direction at +0x68, +0x74, +0x80.
    let camera = e.mem.alloc(0x90);
    set(e, camera + 0x68, 0x3f35_04f3);
    set(e, camera + 0x74, 0x3f13_cd3a);
    set(e, camera + 0x80, 0xbf5d_b3d7);
    // A constant map: two arrays of four entries, enabled bytes 0, 1, 0, 1.
    let cmap = e.mem.alloc(0x2c);
    for array in [cmap + 0xc, cmap + 0x1c] {
        let base = e.mem.alloc(0x10);
        for i in 0..4u32 {
            let entry = obj(e, 0xc, vt);
            set(e, entry + 4, 1);
            e.mem.set_u8(entry + 8, (i & 1) as u8);
            set(e, base + 4 * i, entry);
        }
        set(e, array + 4, base);
        e.mem.set_u16(array + 8, 4);
        e.mem.set_u16(array + 0xa, 4);
        e.mem.set_u16(array + 0xc, 4);
    }
    set(e, 0x0126_f914, 1);
    // A light, a node's light list of three.
    let light = e.mem.alloc(0x44);
    for (i, w) in [HALF, ONE, 0x3dcc_cccd].iter().enumerate() {
        set(e, light + 0x28 + 4 * i as u32, *w);
    }
    set(e, light + 0x40, HALF);
    let ssn = e.mem.alloc(0xc0);
    let mut next = 0;
    for i in (0..3u32).rev() {
        let node = e.mem.alloc(0xc);
        set(e, node, next);
        set(e, node + 8, 0x100 + i);
        next = node;
    }
    set(e, ssn + 0xb4, next);
    set(e, ssn + 0xbc, 3);
    // Lights and colours, the current pass's stages and vertex shader, the drawn object.
    for (i, w) in [ONE, ONE, ONE].iter().enumerate() {
        set(e, 0x011f_4998 + 4 * i as u32, *w);
    }
    for (i, w) in [HALF, 0x3e80_0000, 0x3dcc_cccd, ONE].iter().enumerate() {
        set(e, 0x011f_a0c0 + 4 * i as u32, *w);
    }
    let effect_data = e.mem.alloc(0x10);
    set(e, effect_data + 0xc, TWO);
    let effect = e.mem.alloc(0x70);
    set(e, effect + 0x6c, effect_data);
    let effects = e.mem.alloc(8);
    set(e, effects + 4, effect);
    let manager = e.mem.alloc(0x10);
    set(e, manager + 8, effects);
    set(e, 0x011f_91ac, manager);
    for (i, w) in [HALF, ONE, 0x3e80_0000, ONE].iter().enumerate() {
        set(e, 0x011a_9bd0 + 4 * i as u32, *w);
    }
    set(e, 0x0101_8f5c, 0x3e80_0000);
    e.mem.set_u64(0x0101_a6b0, (-1.0f64).to_bits());
    e.mem.set_u64(0x0106_d508, 0.25f64.to_bits());
    set(e, 0x0106_b9e0, 0x3f60_0000);
    for (i, w) in [ONE, 0xc000_0000, 0x4110_0000].iter().enumerate() {
        set(e, 0x011f_474c + 4 * i as u32, *w);
    }
    let current = e.mem.alloc(0x60);
    let vertex_shader = e.mem.alloc(0x24);
    set(e, vertex_shader + 0x20, device);
    set(e, current + 0x5c, vertex_shader);
    let stages = e.mem.alloc(0x40);
    for i in 0..16u32 {
        let s = e.mem.alloc(0xc);
        set(e, stages + 4 * i, s);
    }
    set(e, current + 0x24, stages);
    set(e, 0x0126_f74c, current);
    let vector = e.mem.alloc(0x100);
    for (i, w) in [ONE, HALF, 0xbf80_0000].iter().enumerate() {
        set(e, vector + 0xf0 + 4 * i as u32, *w);
    }
    let object = e.mem.alloc(0x100);
    set(e, object + 0xf8, vector);
    let holder = e.mem.alloc(4);
    set(e, holder, object);
    let rendering = e.mem.alloc(0x10);
    set(e, rendering + 0xc, holder);
    set(e, 0x011f_91e0, rendering);
    // A transform: a rotation of a few degrees, a translation, a scale.
    let transform = e.mem.alloc(0x34);
    for (i, w) in [
        0x3f7e_8a6e,
        0xbdcc_6c67,
        0,
        0x3dcc_6c67,
        0x3f7e_8a6e,
        0,
        0,
        0,
        ONE,
        ONE,
        TWO,
        HALF,
        TWO,
    ]
    .iter()
    .enumerate()
    {
        set(e, transform + 4 * i as u32, *w);
    }
    W {
        renderer,
        cache,
        device,
        stage_cache,
        stage,
        shader,
        geometry,
        skinned,
        skin,
        property,
        alpha,
        data,
        passes,
        skin_pass,
        br,
        holders,
        list,
        acc,
        camera,
        cmap,
        light,
        ssn,
        transform,
    }
}

trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    fn common(self) -> Diff {
        self.family(&shader::SHADER)
            .family(&leaf::LEAVES)
            // The plain geometry pass (pass types 0xfc, 0xfd) and the renderer's skinned test are the
            // render family's (their own tests).
            // The draw family's partition draw, skinned packer and strip lists, which the batch renderer's
            // skinned draw and the render family's packing call, are stood in here and run in its tests.
            .goes_by_address(&[
                0x00b7_dde0,
                0x00e7_2c20,
                0x00e6_d310,
                0x00e6_d780,
                0x00a7_5f10,
            ])
            .max_calls(MAX_CALLS)
            .max_cases(160)
    }
}

const BYTES: &[u32] = &[0, 1, 0x101];
const LOCKS: &[u32] = &[0, 1, u32::MAX];
const TWO_WAYS: &[u32] = &[0, 1];

/// The counter of index `i` (`011ff9d8`).
fn counter(e: &mut Engine, i: u32, value: u32) {
    set(e, 0x011f_f9d8 + 4 * i, value);
}

/// The render state cache's lock byte and cached value of `state`.
fn cached_state(e: &mut Engine, w: &W, state: u32, locked: u32, value: u32) {
    e.mem.set_u8(w.cache + 0x920 + state, locked as u8);
    set(e, w.cache + 0x120 + 8 * state, value);
}

#[test]
fn z_enable() {
    Diff::new(entry!(0x00b9_7de0, state::set_z_enable(u8, u32)))
        .common()
        .inputs(&[
            Input::Of(BYTES),
            Input::Of(LOCKS),
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            counter(e, 7, c.word(2));
            cached_state(e, &w, 7, c.word(3), 1);
            vec![c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn z_write_enable() {
    Diff::new(entry!(0x00b9_7e30, state::set_z_write_enable(u8, u32)))
        .common()
        .inputs(&[Input::Of(BYTES), Input::Of(LOCKS), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            counter(e, 6, c.word(2));
            vec![c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn alpha_test_enable() {
    Diff::new(entry!(0x00b9_7ed0, state::set_alpha_test_enable(u8, u32)))
        .common()
        .inputs(&[Input::Of(BYTES), Input::Of(LOCKS), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            counter(e, 7, c.word(2));
            vec![c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn alpha_blend_enable() {
    Diff::new(entry!(0x00b9_7fa0, state::set_alpha_blend_enable(u8, u32)))
        .common()
        .inputs(&[Input::Of(BYTES), Input::Of(LOCKS), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            counter(e, 3, c.word(2));
            vec![c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn color_write() {
    Diff::new(entry!(0x00b9_8380, state::set_color_write(u32, u32)))
        .common()
        .inputs(&[Input::Word, Input::Of(LOCKS), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            counter(e, 0, c.word(2));
            vec![c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn z_function() {
    Diff::new(entry!(0x00b9_7e80, state::set_z_function(u32, u32)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 7]), Input::Of(LOCKS), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            counter(e, 7, c.word(2));
            vec![c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn alpha_function() {
    Diff::new(entry!(
        0x00b9_7f20,
        state::set_alpha_function(u32, u32, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 3, 7]),
        Input::Word,
        Input::Of(LOCKS),
        Input::Of(TWO_WAYS),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let _w = world(e);
        counter(e, 1, c.word(3));
        counter(e, 0, 1 - c.word(3));
        vec![c.word(0), c.word(1), c.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn blend() {
    Diff::new(entry!(0x00b9_7ff0, state::set_blend(u32, u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 6, 15]),
            Input::Of(&[1, 7]),
            Input::Of(LOCKS),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            counter(e, 3, c.word(3));
            vec![c.word(0), c.word(1), c.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn any_state() {
    Diff::new(entry!(0x00b9_7d90, state::set_state(u32, u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[7, 0x16, 0x1b, 0xa8]),
            Input::Word,
            Input::Of(LOCKS),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            counter(e, c.word(0) % 8, c.word(3));
            vec![c.word(0), c.word(1), c.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn alpha_to_coverage() {
    Diff::new(entry!(0x00b9_8540, state::set_alpha_to_coverage(u8, u32)))
        .common()
        .inputs(&[
            Input::Of(BYTES),
            Input::Of(LOCKS),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            set(e, 0x011f_94b8, c.word(2));
            set(e, 0x011f_fa28, c.word(3));
            vec![c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn cull_mode_of_renderer() {
    Diff::new(entry!(
        0x00b9_84d0,
        state::set_cull_mode_of_renderer(u32, u32)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 3]), Input::Word, Input::Of(TWO_WAYS)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        set(e, w.cache + 0xf4, c.word(2));
        vec![c.word(0), c.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn cull_mode() {
    Diff::new(entry!(0x00e8_79b0, state::set_cull_mode(Ptr<_>, u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 3]),
            Input::Word,
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            set(e, w.cache + 0xf4, c.word(2));
            cached_state(e, &w, 0x16, c.word(3), 0x101);
            vec![w.cache, c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn render_state() {
    Diff::new(entry!(
        0x00e8_8780,
        state::set_render_state(Ptr<_>, u32, u32, u32, u8)
    ))
    .common()
    .inputs(&[
        Input::Of(&[7, 0x1b, 0xa8]),
        Input::Of(&[0, 1, 5]),
        Input::Word,
        Input::Of(BYTES),
        Input::Of(TWO_WAYS),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        cached_state(e, &w, c.word(0), c.word(4), 1);
        vec![w.cache, c.word(0), c.word(1), c.word(2), c.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn stage_state() {
    Diff::new(entry!(
        0x00e8_8930,
        state::set_stage_state(Ptr<_>, u32, u32, u32, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 3]),
        Input::Of(&[1, 8, 9, 0x13]),
        Input::Of(&[0, 1, 0x55]),
        Input::Word,
    ])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        set(e, w.cache + 0xa20 + 8 * (8 + 1), 1);
        vec![w.cache, c.word(0), c.word(1), c.word(2), c.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn sampler_state() {
    Diff::new(entry!(
        0x00e9_10a0,
        state::set_sampler_state(Ptr<_>, u32, u32, u32, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 3]),
        Input::Of(&[1, 5, 6, 0x15]),
        Input::Of(&[0, 1, 0x55]),
        Input::Word,
    ])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        set(e, w.cache + 0xe20 + 8 * (5 + 1), 1);
        vec![w.cache, c.word(0), c.word(1), c.word(2), c.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn texture() {
    Diff::new(entry!(0x00e8_8a20, state::set_texture(Ptr<_>, u32, u32)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 15]), Input::Of(&[0, 0x1234])])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            set(e, w.cache + 0x10a0 + 4, 0x1234);
            vec![w.cache, c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn vertex_shader() {
    Diff::new(entry!(
        0x00e9_0e00,
        state::set_vertex_shader(Ptr<_>, u32, u8)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 0x40, 0x41]), Input::Of(BYTES)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        set(e, w.cache + 0x10e0, 0x40);
        vec![w.cache, c.word(0), c.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn pixel_shader() {
    Diff::new(entry!(
        0x00e8_8870,
        state::set_pixel_shader(Ptr<_>, u32, u8)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 0x40, 0x41]), Input::Of(BYTES)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        set(e, w.cache + 0x10e8, 0x40);
        vec![w.cache, c.word(0), c.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn declaration() {
    Diff::new(entry!(0x00e9_0fc0, state::set_declaration(Ptr<_>, u32, u8)))
        .common()
        .inputs(&[
            Input::Of(&[0, 0x40, 0x41]),
            Input::Of(BYTES),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            e.mem.set_u8(w.cache + 0x1100, c.word(2) as u8);
            set(e, w.cache + 0x110c, 0x40);
            vec![w.cache, c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn alpha_test_of() {
    Diff::new(entry!(0x00e8_79e0, state::set_alpha_test_of(Ptr<_>, u32)))
        .common()
        .inputs(&[
            Input::F32,
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(&[0, 0x7ff8_0000]),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(FLOAT_SWEEP)
        .setup(|e, c| {
            let w = world(e);
            let object = e.mem.alloc(0x34);
            set(e, object + 0x30, bits(c, 0));
            e.mem.set_u8(w.cache + 0x10f4, c.word(1) as u8);
            e.mem.set_u8(w.cache + 0x10f5, c.word(2) as u8);
            if c.word(3) != 0 {
                e.mem.set_u64(0x0106_d508, 0x7ff8_0000_0000_0000);
            }
            vec![w.cache, object]
        })
        .run()
        .assert_same();
}

#[test]
fn range() {
    Diff::new(entry!(0x00e8_8730, state::set_range(Ptr<_>, f32, f32)))
        .common()
        .inputs(&[Input::F32, Input::F32])
        .translation_calls_plainly(&[])
        .float_sweep(FLOAT_SWEEP)
        .setup(|_e, c| {
            let w = world(_e);
            vec![w.cache, bits(c, 0), bits(c, 1)]
        })
        .run()
        .assert_same();
}

#[test]
fn cull_side() {
    Diff::new(entry!(0x00e8_8750, state::cull_side_set(Ptr<_>) -> bool))
        .common()
        .inputs(&[Input::Word])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            set(e, w.cache + 0xf4, c.word(0));
            vec![w.cache]
        })
        .run()
        .assert_same();
}

/// The stage cache's set bytes from the bits of `mask` (A 0-7, B 8-15, C 16-20, D 21-25, E 26-30).
fn stage_banks(e: &mut Engine, cache: u32, mask: u32) {
    let banks = [
        (0x2c, 8, 0),
        (0x5c, 8, 8),
        (0x80, 5, 16),
        (0xa4, 5, 21),
        (0xb0, 5, 26),
    ];
    for (at, n, shift) in banks {
        for i in 0..n {
            e.mem
                .set_u8(cache + at + i, ((mask >> (shift + i)) & 1) as u8);
            set(e, cache + 0xc + 4 * (i % 8), 0x700 + i);
        }
    }
    for (i, at) in [0x3cu32, 0x6c, 0x90].iter().enumerate() {
        for k in 0..5 {
            set(e, cache + at + 4 * k, 0x800 + 0x10 * i as u32 + k);
        }
    }
    set(e, cache + 0x64, 3);
    set(e, cache + 0x88, 2);
    set(e, cache + 0xac, 1);
}

const MASKS: &[u32] = &[0, 0x0421_0842, 0x7fff_ffff, 0x0a5a_5a5a];

#[test]
fn cached_stage_state() {
    Diff::new(entry!(
        0x00e8_9060,
        state::cached_stage_state(Ptr<_>, u32, u32, u32) -> bool
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 7, 8, 9]), Input::Of(MASKS)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        stage_banks(e, w.stage_cache, c.word(1));
        let out = e.mem.alloc(8);
        vec![w.stage_cache, c.word(0), out, out + 4]
    })
    .run()
    .assert_same();
}

#[test]
fn apply_stage_states() {
    Diff::new(entry!(
        0x00e8_9250,
        state::apply_stage_states(Ptr<_>, u32) -> u32
    ))
    .common()
    .inputs(&[Input::Of(&[0, 3]), Input::Of(MASKS)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        stage_banks(e, w.stage_cache, c.word(1));
        vec![w.stage_cache, c.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn apply_sampler_states() {
    Diff::new(entry!(
        0x00e8_92d0,
        state::apply_sampler_states(Ptr<_>, u32) -> u32
    ))
    .common()
    .inputs(&[Input::Of(&[0, 3]), Input::Of(MASKS)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        stage_banks(e, w.stage_cache, c.word(1));
        vec![w.stage_cache, c.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn apply_states() {
    Diff::new(entry!(0x00e8_9410, state::apply_states(Ptr<_>, u32) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 3, 4, 7]), Input::Of(MASKS)])
        .translation_calls_plainly(&[0x00e8_9250, 0x00e8_92d0])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            stage_banks(e, w.stage_cache, c.word(1));
            vec![w.stage_cache, c.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn address_modes() {
    Diff::new(entry!(0x00e7_de90, state::set_address_modes(Ptr<_>, u32)))
        .common()
        .inputs(&[Input::Of(&[0, 3, 7]), Input::Of(MASKS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            stage_banks(e, w.stage_cache, c.word(1));
            vec![w.stage, c.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn filter_mode() {
    Diff::new(entry!(0x00e7_def0, state::set_filter_mode(Ptr<_>, u32)))
        .common()
        .inputs(&[Input::Of(&[0, 3, 7]), Input::Of(MASKS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            stage_banks(e, w.stage_cache, c.word(1));
            vec![w.stage, c.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn has_streams() {
    Diff::new(entry!(0x00e8_eeb0, state::has_streams(Ptr<_>) -> bool))
        .common()
        .inputs(&[Input::Of(&[0, 1, 3]), Input::Of(&[0, 1, 2, 3])])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            let data = e.mem.alloc(0x2c);
            let streams = e.mem.alloc(0xc);
            for i in 0..3 {
                let s = e.mem.alloc(0xc);
                set(e, s + 8, u32::from(i != c.word(1)));
                set(
                    e,
                    streams + 4 * i,
                    if i == 2 && c.word(1) == 3 { 0 } else { s },
                );
            }
            set(e, data + 0x20, c.word(0));
            set(e, data + 0x28, streams);
            vec![data]
        })
        .run()
        .assert_same();
}

#[test]
fn add_object_to_group() {
    Diff::new(entry!(
        0x00e8_8dc0,
        state::add_object_to_group(u32, u32, Ptr<_>, Ptr<_>, u8, u32, u32) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(TWO_WAYS),
        Input::Of(BYTES),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 9]),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let group = e.mem.u32(w.renderer + 0x8b0);
        set(e, w.data + 0x34, c.word(2));
        let parts = e.mem.u32(e.mem.u32(w.skin + 0xc) + 0xc);
        set(e, parts + 0x28, c.word(3));
        let skin = if c.word(0) != 0 { w.skin } else { 0 };
        vec![0x55, group, w.data, skin, c.word(1), 0, 0]
    })
    .run()
    .assert_same();
}

#[test]
fn array_set_at() {
    Diff::new(entry!(0x00e8_6f40, state::array_set_at(Ptr<_>, u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 3, 4, 6]),
            Input::Of(&[0, 1, 2]),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            let array = w.cmap + 0xc;
            let element = e.mem.alloc(4);
            let base = e.mem.u32(array + 4);
            let value = match c.word(1) {
                0 => 0,
                1 => e.mem.u32(base),
                _ => e.mem.u32(base + 4),
            };
            set(e, element, value);
            set(e, base + 8, 0);
            let big = e.mem.alloc(0x20);
            for i in 0..4 {
                let v = e.mem.u32(base + 4 * i);
                set(e, big + 4 * i, v);
            }
            set(e, array + 4, big);
            set(e, 0x0126_f914, c.word(2));
            vec![array, c.word(0), element]
        })
        .run()
        .assert_same();
}

#[test]
fn sort_entries_by_enabled() {
    Diff::new(entry!(
        0x00e8_70c0,
        state::sort_entries_by_enabled(u32, Ptr<_>)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 0b0110, 0b1001, 0b1111]),
        Input::Of(&[1, 2, 4]),
    ])
    .translation_calls_plainly(&[0x00e8_6f40])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let array = w.cmap + 0x1c;
        let base = e.mem.u32(array + 4);
        for i in 0..4 {
            let entry = e.mem.u32(base + 4 * i);
            e.mem.set_u8(entry + 8, ((c.word(0) >> i) & 1) as u8);
        }
        e.mem.set_u16(array + 0xa, c.word(1) as u16);
        vec![w.cmap, array]
    })
    .run()
    .assert_same();
}

#[test]
fn sort_by_enabled() {
    Diff::new(entry!(0x00e8_7200, state::sort_by_enabled(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 0b0110, 0b1001])])
        .translation_calls_plainly(&[0x00e8_6f40, 0x00e8_70c0])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            let base = e.mem.u32(w.cmap + 0x10);
            for i in 0..4 {
                let entry = e.mem.u32(base + 4 * i);
                e.mem.set_u8(entry + 8, ((c.word(0) >> i) & 1) as u8);
            }
            vec![w.cmap]
        })
        .run()
        .assert_same();
}

// The batch renderer.

/// Group 1's list 0 holds passes 0 and 1, its list 2 pass 2; group 0's list 0 the skinned pass.
fn group_list(e: &Engine, w: &W, group: u32, list: u32) -> u32 {
    e.mem.u32(w.br + 8) + group * 0x68 + list * 0x14
}

#[test]
fn first_list() {
    Diff::new(entry!(
        0x00b9_8ae0,
        batch::first_list(Ptr<_>, u32, u32) -> bool
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 2, 3])])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let out = e.mem.alloc(4);
        vec![w.br, c.word(0), out]
    })
    .run()
    .assert_same();
}

#[test]
fn pop_active() {
    Diff::new(entry!(0x00b9_9730, batch::pop_active(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            if c.word(0) != 0 {
                set(e, w.br + 0x30, 0);
            }
            vec![w.br + 0x2c]
        })
        .run()
        .assert_same();
}

#[test]
fn next_active() {
    Diff::new(entry!(
        0x00b9_9900,
        batch::next_active(Ptr<_>, u32, u32) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 0x10, 0x11, 0x40]),
        Input::Of(&[0x10, 0x11, 0x20]),
    ])
    .translation_calls_plainly(&[0x00b9_8ae0, 0x00b9_9730])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        e.mem.set_u16(w.br + 0x2c, c.word(0) as u16);
        set(e, w.br + 0x20, c.word(1));
        let out = e.mem.alloc(8);
        vec![w.br, out, out + 4]
    })
    .run()
    .assert_same();
}

#[test]
fn next_group() {
    Diff::new(entry!(
        0x00b9_a090,
        batch::next_group(Ptr<_>, u32, u32, u32) -> bool
    ))
    .common()
    .inputs(&[Input::Of(&[0, 0x10, 0x11, 0x13]), Input::Of(&[0, 0x10])])
    .translation_calls_plainly(&[0x00b9_8ae0, 0x00b9_9730, 0x00b9_9900])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        e.mem.set_u16(w.br + 0x2c, c.word(1) as u16);
        let out = e.mem.alloc(0xc);
        set(e, out, c.word(0));
        vec![w.br, out, out + 4, out + 8]
    })
    .run()
    .assert_same();
}

#[test]
fn render_group() {
    Diff::new(entry!(
        0x00b9_a120,
        batch::render_group(Ptr<_>, u32, u32, u32, u32) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0x11, 0x10]),
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::Of(BYTES),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 0x10]),
    ])
    .translation_calls_plainly(&[
        0x00b9_84d0,
        0x00b9_8ae0,
        0x00b9_8e80,
        0x00b9_9110,
        0x00b9_9390,
        0x00b9_94f0,
        0x00b9_9730,
        0x00b9_9900,
    ])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let out = e.mem.alloc(0xc);
        set(e, out, c.word(0));
        set(e, out + 4, c.word(1));
        // Every list of groups 0 and 1 holds passes.
        for group in 0..2 {
            for list in 0..5 {
                let at = group_list(e, &w, group, list);
                list_at(e, at, &[w.passes[0], w.passes[1]]);
            }
        }
        e.mem.set_u8(0x011f_9421, c.word(3) as u8);
        e.mem.set_u16(w.br + 0x2c, c.word(4) as u16);
        set(e, 0x011f_fa24, 1);
        vec![w.br, out, out + 4, out + 8, c.word(2)]
    })
    .run()
    .assert_same();
}

/// The current pass type and shader the renderer has begun, from the case.
fn begun(e: &mut Engine, w: &W, which: u32) {
    let (pass_type, shader) = match which {
        0 => (0x10, w.shader),
        1 => (0x10, 0),
        _ => (0x11, w.shader),
    };
    set(e, 0x011f_fe30, pass_type);
    set(e, 0x011f_fe2c, shader);
}

#[test]
fn render_pass() {
    Diff::new(entry!(
        0x00b9_94f0,
        batch::render_pass(Ptr<_>, u32, u32, u32, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::Of(&[0, 1, 2]),
        Input::Of(BYTES),
        Input::Of(&[0, 1, 3]),
        Input::Of(&[0, 1, 0x11]),
    ])
    .translation_calls_plainly(&[0x00b9_8e80, 0x00b9_9110, 0x00b9_9390])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        begun(e, &w, c.word(1));
        e.mem.set_u8(0x011f_9421, (c.word(3) & 1) as u8);
        set(e, RETS + 0x30, c.word(3) >> 1);
        set(e, w.shader + 0x68, c.word(4));
        let pass = if c.word(0) == 4 {
            w.skin_pass
        } else {
            w.passes[c.word(0) as usize]
        };
        let pass_type = u32::from(e.mem.u16(pass + 4));
        vec![pass, pass_type, c.word(2), 1, c.word(2) ^ 1]
    })
    .run()
    .assert_same();
}

#[test]
fn render_standard() {
    Diff::new(entry!(
        0x00b9_8e80,
        batch::render_standard(Ptr<_>, u32, u32, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0x10, 0x11, 0xca, 0xd1, 0xfc, 0x102, 4, 0x23a]),
        Input::Of(BYTES),
        Input::Of(BYTES),
        Input::Of(BYTES),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let pass = w.passes[0];
        e.mem.set_u16(pass + 4, c.word(0) as u16);
        e.mem.set_u8(pass + 7, c.word(4) as u8);
        e.mem.set_u8(0x011a_d8ec, c.word(5) as u8);
        vec![pass, c.word(1), c.word(2), c.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn render_skinned() {
    Diff::new(entry!(
        0x00b9_9110,
        batch::render_skinned(Ptr<_>, u32, u32, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0x10, 0x11, 4]),
        Input::Of(BYTES),
        Input::Of(BYTES),
        Input::Of(BYTES),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 0x011f_49d8]),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let pass = w.skin_pass;
        e.mem.set_u16(pass + 4, c.word(0) as u16);
        e.mem.set_u8(0x011a_d8ec, (c.word(4) & 1) as u8);
        e.mem.set_u8(pass + 7, (c.word(4) >> 1) as u8);
        set(e, RETS + 8, c.word(5));
        set(e, RETS + 0x6c, c.word(5) & 1);
        vec![pass, c.word(1), c.word(2), c.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn begin_pass() {
    Diff::new(entry!(0x00b9_9390, batch::begin_pass(u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0x10, 0xca, 0xd1, 0xfc, 0xfd, 0x102, 0xfe]),
            Input::Of(&[0, 1, 2]),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            begun(e, &w, c.word(1));
            e.mem.set_u8(0x011a_d8ec, c.word(2) as u8);
            vec![c.word(0), w.shader]
        })
        .run()
        .assert_same();
}

#[test]
fn end_pass() {
    Diff::new(entry!(0x00b9_88e0, batch::end_pass()))
        .common()
        .inputs(&[
            Input::Of(&[0, 0x10, 0xfe, 0x10b, 0x230]),
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            set(e, 0x011f_fe30, c.word(0));
            set(e, 0x011f_fe2c, if c.word(1) != 0 { w.shader } else { 0 });
            e.mem.set_u8(0x011a_d8ec, c.word(2) as u8);
            vec![]
        })
        .run()
        .assert_same();
}

#[test]
fn compare_by_depth() {
    Diff::new(entry!(0x00b9_8b20, batch::compare_by_depth(u32, u32) -> i32))
        .common()
        .inputs(&[
            Input::F32,
            Input::F32,
            Input::Of(TWO_WAYS),
            Input::F32,
            Input::F32,
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(256)
        .setup(|e, c| {
            let w = world(e);
            // Two geometries' bounds: the first's x and radius, the second's z and radius from the case.
            let mut items = [0u32; 2];
            for (k, item) in items.iter_mut().enumerate() {
                let bound = e.mem.alloc(0x10);
                let (a, r) = if k == 0 {
                    (bits(c, 0), bits(c, 1))
                } else {
                    (bits(c, 3), bits(c, 4))
                };
                for (i, v) in [a, TWO, a ^ 0x0080_0000, r].iter().enumerate() {
                    set(e, bound + 4 * i as u32, *v);
                }
                let geometry = e.mem.alloc(0xc4);
                set(
                    e,
                    geometry + 0x20,
                    if c.word(5) != 0 && k == 1 { 0 } else { bound },
                );
                let pass = e.mem.alloc(8);
                set(e, pass, geometry);
                *item = e.mem.alloc(4);
                set(e, *item, pass);
            }
            e.mem.set_u8(0x011f_fe35, c.word(2) as u8);
            set(e, 0x011f_fe38, w.camera);
            vec![items[0], items[1]]
        })
        .run()
        .assert_same();
}

#[test]
fn sort_alphas() {
    Diff::new(entry!(0x00b9_9a10, batch::sort_alphas(Ptr<_>, u32, u8)))
        .common()
        .inputs(&[Input::Of(BYTES), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[0x00b9_9630])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            // Each alpha holder's list: four passes with bounds at other depths.
            for k in 0..2u32 {
                let holder = e.mem.u32(w.br + 0x70 + 4 * k);
                let mut items = vec![];
                for i in 0..4u32 {
                    let bound = e.mem.alloc(0x10);
                    set(e, bound, 0x3f80_0000 + (((i * 3 + k) % 4) << 20));
                    set(e, bound + 0xc, if c.word(1) != 0 { HALF } else { 0 });
                    let geometry = e.mem.alloc(0xc4);
                    set(e, geometry + 0x20, bound);
                    let pass = e.mem.alloc(8);
                    set(e, pass, geometry);
                    items.push(pass);
                }
                list_at(e, holder + 0xc, &items);
            }
            vec![w.br, w.camera, c.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn sort_list() {
    Diff::new(entry!(0x00b9_9630, batch::sort_list(Ptr<_>, u32)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 5])])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            // A list of passes keyed by their geometry word: the stand-in comparison sorts by item value.
            let list = e.mem.alloc(0x14);
            let items: Vec<u32> = (0..c.word(0))
                .map(|i| [0x30, 0x10, 0x20, 0x10, 0x40][i as usize])
                .collect();
            list_at(e, list, &items);
            let _ = w;
            vec![list, 0x00b9_88a0]
        })
        .run()
        .assert_same();
}

#[test]
fn list_append() {
    Diff::new(entry!(0x00b9_9840, batch::list_append(Ptr<_>, u32)))
        .common()
        .inputs(&[Input::Of(TWO_WAYS), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            let list = e.mem.alloc(0x14);
            list_at(
                e,
                list,
                if c.word(0) != 0 {
                    &w.passes[..2]
                } else {
                    &w.passes[..0]
                },
            );
            if c.word(1) != 0 {
                set(e, list + 0xc, 0);
            }
            let item = e.mem.alloc(4);
            set(e, item, w.passes[3]);
            vec![list, item]
        })
        .run()
        .assert_same();
}

#[test]
fn list_prepend() {
    Diff::new(entry!(0x00b9_9890, batch::list_prepend(Ptr<_>, u32)))
        .common()
        .inputs(&[Input::Of(TWO_WAYS), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            let list = e.mem.alloc(0x14);
            list_at(
                e,
                list,
                if c.word(0) != 0 {
                    &w.passes[..2]
                } else {
                    &w.passes[..0]
                },
            );
            if c.word(1) != 0 {
                set(e, list + 0xc, 0);
            }
            let item = e.mem.alloc(4);
            set(e, item, w.passes[3]);
            vec![list, item]
        })
        .run()
        .assert_same();
}

#[test]
fn add_pass() {
    Diff::new(entry!(0x00b9_abc0, batch::add_pass(Ptr<_>, Ptr<_>, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(BYTES),
        ])
        .translation_calls_plainly(&[
            0x00b9_9840,
            0x00b9_9a50,
            0x00b9_9770,
            0x00b9_9a90,
            0x00b9_9b00,
            0x00b9_8e20,
        ])
        .float_sweep(0)
        .setup(|_e, c| {
            let w = world(_e);
            vec![
                w.holders[c.word(0) as usize],
                w.passes[c.word(1) as usize],
                c.word(2),
            ]
        })
        .run()
        .assert_same();
}

#[test]
fn register_pass() {
    Diff::new(entry!(
        0x00b9_b3b0,
        batch::register_pass(Ptr<_>, Ptr<_>, u32, u32) -> u32
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 0xc]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(BYTES),
    ])
    .translation_calls_plainly(&[
        0x00b9_9840,
        0x00b9_9a50,
        0x00b9_9770,
        0x00b9_9a90,
        0x00b9_9b00,
        0x00b9_8e20,
    ])
    .float_sweep(0)
    .setup(|_e, c| {
        let w = world(_e);
        vec![w.br, w.passes[c.word(1) as usize], c.word(0), c.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn register_alpha() {
    Diff::new(entry!(
        0x00b9_b3d0,
        batch::register_alpha(Ptr<_>, Ptr<_>, u32) -> u32
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, u32::MAX]),
    ])
    .translation_calls_plainly(&[
        0x00b9_9840,
        0x00b9_9a50,
        0x00b9_9770,
        0x00b9_9a90,
        0x00b9_9b00,
        0x00b9_8e20,
    ])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        set(e, w.br + 0x34, c.word(2));
        vec![w.br, w.passes[c.word(1) as usize], c.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn clear_holder() {
    Diff::new(entry!(0x00b9_a490, batch::clear_holder(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[0x00b9_9bc0])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            let holder = w.holders[c.word(0) as usize];
            if c.word(1) != 0 {
                set(e, holder + 8, 0);
            }
            vec![holder]
        })
        .run()
        .assert_same();
}

#[test]
fn clear_group() {
    Diff::new(entry!(0x00b9_9a50, batch::clear_group(Ptr<_>, u32)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Word])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            vec![e.mem.u32(w.br + 8) + 0x68 * c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn clear_renderer() {
    Diff::new(entry!(0x00b9_9bc0, batch::clear_renderer(Ptr<_>, u32)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 4]), Input::Of(&[0, 1, 3, 5])])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            set(e, w.br + 0x10, c.word(0));
            let _ = c.word(1);
            vec![w.br, 0]
        })
        .run()
        .assert_same();
}

#[test]
fn release_spare() {
    Diff::new(entry!(0x00b6_3e90, batch::release_spare(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 1, 3]), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            let list = w.list;
            let mut spare = 0;
            for _ in 0..c.word(0) {
                let node = e.mem.alloc(0xc);
                set(e, node, spare);
                spare = node;
            }
            set(e, list + 0xc, spare);
            if c.word(1) != 0 {
                set(e, list + 8, 0);
            }
            set(e, 0x011c_5f84, 1);
            vec![list]
        })
        .run()
        .assert_same();
}

#[test]
fn add_to_group() {
    Diff::new(entry!(
        0x00b9_9c90,
        batch::add_to_group(Ptr<_>, Ptr<_>, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::Of(&[0, 2, 4, 0x10, 0x238, 0x300]),
        Input::F32,
        Input::Of(&[0, 0x0040_0000]),
        Input::Of(&[0, 1, 2]),
    ])
    .translation_calls_plainly(&[
        0x00b9_8e20,
        0x00b9_9770,
        0x00b9_9a50,
        0x00b9_9a90,
        0x00b9_9b00,
    ])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let w = world(e);
        let pass = match c.word(0) {
            3 => 0,
            4 => {
                let p = e.mem.alloc(8);
                e.mem.set_u16(p + 4, 0x10);
                p
            }
            i => w.passes[i as usize],
        };
        set(e, w.property + 0x28, bits(c, 2));
        set(e, w.property + 0x24, c.word(3));
        // The alpha property: blends without bit 0, or none.
        match c.word(4) {
            1 => e.mem.set_u16(w.alpha + 0x18, 0x200),
            2 => set(e, w.geometry + 0x9c, 0),
            _ => {}
        }
        // A group of type 0x10 with nothing counted, so the active list takes it.
        let groups = e.mem.u32(w.br + 8);
        set(e, groups + 0x64, 0);
        vec![w.br, pass, c.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn insert_active() {
    Diff::new(entry!(0x00b9_9770, batch::insert_active(Ptr<_>, u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 0x8, 0x10, 0x11, 0x12, 0x10010]),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[0x00b9_8e20])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            if c.word(1) != 0 {
                e.mem.set_u16(w.br + 0x2c, 0);
                set(e, w.br + 0x30, 0);
            }
            vec![w.br + 0x2c, c.word(0), 0x00b9_88a0]
        })
        .run()
        .assert_same();
}

#[test]
fn group_prepend() {
    Diff::new(entry!(
        0x00b9_9a90,
        batch::group_prepend(Ptr<_>, u32, Ptr<_>)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 2, 4]), Input::Of(TWO_WAYS)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let group = e.mem.u32(w.br + 8) + 0x68;
        if c.word(1) != 0 {
            set(e, group + 0xc, 0);
        }
        vec![group, c.word(0), w.passes[3]]
    })
    .run()
    .assert_same();
}

#[test]
fn group_append() {
    Diff::new(entry!(
        0x00b9_9b00,
        batch::group_append(Ptr<_>, u32, Ptr<_>)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 2, 4]), Input::Of(TWO_WAYS)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let group = e.mem.u32(w.br + 8) + 0x68;
        if c.word(1) != 0 {
            set(e, group + 0xc, 0);
        }
        vec![group, c.word(0), w.passes[3]]
    })
    .run()
    .assert_same();
}

// The accumulator and the shaders' set-ups.

#[test]
fn process_commands() {
    Diff::new(entry!(
        0x00b6_4020,
        acc::process_commands(Ptr<_>, Ptr<_>, u32, Ptr<_>, u32) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 3]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1, 4, 14]),
        Input::F32,
    ])
    .translation_calls_plainly(&[
        0x00b9_9840,
        0x00b9_9a50,
        0x00b9_9770,
        0x00b9_9a90,
        0x00b9_9b00,
        0x00b9_8e20,
    ])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let w = world(e);
        let a = w.acc;
        e.mem.set_u8(a + 0x164, (c.word(0) & 1) as u8);
        set(e, w.shader + 0x68, if c.word(0) & 2 != 0 { 0xd } else { 0 });
        e.mem.set_u8(a + 0x3a, (c.word(1) & 1) as u8);
        e.mem.set_u8(a + 0x85, (c.word(1) >> 1) as u8);
        set(
            e,
            RETS + 0xac,
            if c.word(2) & 1 != 0 { w.passes[2] } else { 0 },
        );
        e.mem.set_u8(a + 0x84, (c.word(2) >> 1) as u8);
        set(e, w.br + 0x34, c.word(3));
        e.mem.set_u8(a + 0x16c, c.word(3) as u8);
        e.mem.set_u8(w.passes[2] + 6, c.word(4) as u8);
        e.mem.set_u8(a + 0x166, 1);
        set(e, a + 0x168, 2);
        let bound = e.mem.u32(w.geometry + 0x20);
        set(e, bound + 8, bits(c, 5));
        vec![a, w.geometry, w.geometry + 0x9c, w.property, w.shader]
    })
    .run()
    .assert_same();
}

#[test]
fn register_object() {
    Diff::new(entry!(
        0x00b6_3f10,
        acc::register_object(Ptr<_>, Ptr<_>) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Of(TWO_WAYS),
    ])
    .translation_calls_plainly(&[
        0x00b9_9840,
        0x00b9_9a50,
        0x00b9_9770,
        0x00b9_9a90,
        0x00b9_9b00,
        0x00b9_8e20,
    ])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let geometry = if c.word(0) == 1 {
            w.skinned
        } else {
            w.geometry
        };
        if c.word(0) == 2 {
            set(e, w.geometry + 0xc0, 0);
        }
        set(e, RETS + 8, if c.word(2) != 0 { 0x011f_49d8 } else { 0 });
        e.mem.set_u8(w.skin + 0x3c, (c.word(1) & 1) as u8);
        set(e, w.acc + 0x19c, c.word(1));
        set(e, 0x011f_9f84, 0x00b6_4020);
        vec![w.acc, geometry]
    })
    .run()
    .assert_same();
}

#[test]
fn finish() {
    Diff::new(entry!(0x00b6_5e80, acc::finish(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(TWO_WAYS), Input::Of(&[0, 1, 2])])
        .translation_calls_plainly(&[0x00b6_4f90, 0x00b9_8e80, 0x00b9_9110, 0x00b9_9390])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            set(e, w.acc + 8, c.word(0));
            begun(e, &w, 0);
            set(e, w.acc + 0x1c, c.word(1));
            vec![w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn draw_sorted() {
    Diff::new(entry!(0x00b6_4f90, acc::draw_sorted(Ptr<_>)))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 1, 2])])
        .translation_calls_plainly(&[0x00b9_8e80, 0x00b9_9110, 0x00b9_9390])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            begun(e, &w, 0);
            set(e, w.acc + 0x1c, c.word(0));
            // The geometry list: inline (+0xc), or one at +0x18 with two or no nodes.
            if c.word(1) != 0 {
                let list = e.mem.alloc(0xc);
                if c.word(1) == 2 {
                    let at = e.mem.alloc(0x14);
                    list_at(e, at, &[w.geometry, w.skinned]);
                    for k in 0..2 {
                        let v = e.mem.u32(at + 4 + 4 * k);
                        set(e, list + 4 * k, v);
                    }
                    set(e, list + 8, 2);
                }
                set(e, w.acc + 0x18, list);
            }
            vec![w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn blend_of() {
    Diff::new(entry!(0x00be_1ff0, acc::set_blend_of(u32, u32)))
        .common()
        .inputs(&[
            Input::F32,
            Input::F32,
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 1, 2]),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(FLOAT_SWEEP)
        .setup(|e, c| {
            let w = world(e);
            set(e, w.property + 0x28, bits(c, 0));
            set(e, w.property + 0x2c, bits(c, 1));
            let block = e.mem.alloc(0x14);
            let alpha = match c.word(2) {
                0 => 0,
                1 => w.alpha,
                _ => {
                    let a = e.mem.alloc(0x1c);
                    e.mem.set_u16(a + 0x18, 0x200);
                    a
                }
            };
            set(e, block, alpha);
            match c.word(3) {
                0 => set(e, block + 0xc, w.property),
                1 => {
                    set(e, block + 0xc, w.property);
                    set(e, w.property + 0x1c, u32::MAX);
                }
                _ => set(e, block + 0xc, 0),
            }
            e.map(0, 0x1000);
            vec![0x55, block]
        })
        .run()
        .assert_same();
}

#[test]
fn make_matrix() {
    Diff::new(entry!(
        0x00b7_1a40,
        acc::make_matrix(u32, Ptr<_>, Ptr<_>, f32)
    ))
    .common()
    .inputs(&[
        Input::F32,
        Input::F32,
        Input::F32,
        Input::F32Swept(ONE),
        Input::F32Swept(HALF),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let w = world(e);
        let out = e.mem.alloc(0x40);
        let rotation = e.mem.alloc(0x24);
        for i in 0..9u32 {
            let v = if i % 4 == 0 {
                bits(c, 0)
            } else if i == 5 {
                bits(c, 4)
            } else {
                bits(c, 1)
            };
            set(e, rotation + 4 * i, v ^ (i << 3));
        }
        let point = e.mem.alloc(0xc);
        set(e, point, bits(c, 2));
        set(e, point + 4, bits(c, 3));
        set(e, point + 8, ONE);
        let _ = w;
        vec![out, rotation, point, bits(c, 3)]
    })
    .run()
    .assert_same();
}

#[test]
fn world_transform() {
    Diff::new(entry!(
        0x00e6_fbb0,
        acc::set_world_transform(Ptr<_>, Ptr<_>, u8)
    ))
    .common()
    .inputs(&[
        Input::F32,
        Input::F32,
        Input::Of(BYTES),
        Input::F32Swept(HALF),
        Input::F32Swept(ONE),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let w = world(e);
        set(e, w.transform, bits(c, 0));
        set(e, w.transform + 0x10, bits(c, 3));
        set(e, w.transform + 0x20, bits(c, 4));
        set(e, w.transform + 0x30, bits(c, 1));
        set(e, w.renderer + 0x638, bits(c, 3));
        set(e, w.renderer + 0x644, bits(c, 4));
        vec![w.renderer, w.transform, c.word(2)]
    })
    .run()
    .assert_same();
}

#[test]
fn world_view_constants() {
    Diff::new(entry!(
        0x00b7_c750,
        acc::set_world_view_constants(u32, u32, u32)
    ))
    .common()
    .inputs(&[Input::F32, Input::Word])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let this = e.mem.alloc(0x20);
        set(e, this + 0x18, w.renderer);
        set(e, w.transform + 0x30, bits(c, 0));
        vec![this, c.word(1), w.transform]
    })
    .run()
    .assert_same();
}

#[test]
fn light_direction() {
    Diff::new(entry!(0x00b7_c7b0, acc::set_light_direction(u32)))
        .common()
        .inputs(&[Input::F32, Input::F32, Input::F32Swept(HALF)])
        .translation_calls_plainly(&[])
        .float_sweep(FLOAT_SWEEP)
        .setup(|e, c| {
            let w = world(e);
            let vector = e
                .mem
                .u32(e.mem.u32(e.mem.u32(e.mem.u32(0x011f_91e0) + 0xc)) + 0xf8);
            set(e, vector + 0xf0, bits(c, 0));
            set(e, vector + 0xf4, bits(c, 1));
            set(e, vector + 0xf8, bits(c, 2));
            vec![w.transform]
        })
        .run()
        .assert_same();
}

#[test]
fn texture_by_index() {
    Diff::new(entry!(
        0x00c0_3230,
        acc::texture_by_index(Ptr<_>, u32) -> u32
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(&[0, 0x80])])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        set(e, w.property + 0x20, c.word(1));
        vec![w.property, c.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn light_color() {
    Diff::new(entry!(0x00c0_3010, acc::accumulate_light_color(Ptr<_>)))
        .common()
        .inputs(&[
            Input::F32,
            Input::F32,
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::F32Swept(ONE),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(FLOAT_SWEEP)
        .setup(|e, c| {
            let w = world(e);
            set(e, w.light + 0x28, bits(c, 0));
            set(e, w.light + 0x40, bits(c, 1));
            set(e, w.light + 0x30, bits(c, 4));
            e.mem.set_u8(0x011f_941e, c.word(2) as u8);
            if c.word(3) != 0 {
                let color = e.mem.alloc(0xc);
                for i in 0..3 {
                    set(e, color + 4 * i, ONE);
                }
                set(e, w.light + 0x34, color);
            }
            vec![w.light]
        })
        .run()
        .assert_same();
}

#[test]
fn tint() {
    Diff::new(entry!(0x00c0_2d90, acc::set_tint(u32, Ptr<_>, u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, 0x20]),
            Input::Of(&[0, 1, 0x100]),
            Input::Of(TWO_WAYS),
            Input::F32,
        ])
        .translation_calls_plainly(&[])
        .float_sweep(FLOAT_SWEEP)
        .setup(|e, c| {
            let w = world(e);
            set(e, w.property + 0x24, c.word(0));
            let source = if c.word(2) != 0 {
                let s = e.mem.alloc(0x3c);
                set(e, s + 0x38, bits(c, 3));
                s
            } else {
                set(e, 0x0101_8f5c, bits(c, 3));
                0
            };
            vec![0x55, w.geometry, c.word(1), source]
        })
        .run()
        .assert_same();
}

#[test]
fn check_create_pass_array() {
    Diff::new(entry!(
        0x00ba_95e0,
        acc::check_create_pass_array(Ptr<_>, i32)
    ))
    .common()
    .inputs(&[Input::Of(&[0, 1, 5, u32::MAX]), Input::Of(TWO_WAYS)])
    .translation_calls_plainly(&[0x00ba_94b0, 0x00ba_8b80, 0x00ba_8d30])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        set(
            e,
            w.property + 0x3c,
            if c.word(1) != 0 { 0 } else { 0x4444 },
        );
        vec![w.property, c.word(0)]
    })
    .run()
    .assert_same();
}

#[test]
fn set_alpha() {
    Diff::new(entry!(0x00ba_8ab0, acc::set_alpha(Ptr<_>, f32)))
        .common()
        .inputs(&[Input::F32, Input::F32])
        .translation_calls_plainly(&[])
        .float_sweep(FLOAT_SWEEP)
        .setup(|e, c| {
            let w = world(e);
            set(e, w.property + 0x28, bits(c, 0));
            set(e, w.property + 0x38, 0x99);
            vec![w.property, bits(c, 1)]
        })
        .run()
        .assert_same();
}

#[test]
fn light_at() {
    Diff::new(entry!(0x00b5_ac20, acc::light_at(Ptr<_>, u32) -> u32))
        .common()
        .inputs(&[Input::Of(&[0, 1, 2, 3, 7])])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|_e, c| {
            let w = world(_e);
            vec![w.ssn, c.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn pair_textures() {
    Diff::new(entry!(0x00b7_cfb0, acc::set_pair_textures(u32, u32)))
        .common()
        .inputs(&[Input::Of(&[
            0x94, 0x1f3, 0xa1, 0x115, 0xe5, 0x161, 0x1f8, 0x1fc, 0x1f9, 0x22c, 0x230, 0x10,
        ])])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            set(e, 0x011f_95e4, 0xe4);
            set(e, 0x011f_95e8, 0xe8);
            vec![0x55, c.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn restore_states() {
    Diff::new(entry!(0x00b7_c320, acc::restore_states(u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0x250, 0x251, 6, 8, 0x10]),
            Input::Of(&[0, 1, 2]),
            Input::Of(&[0, 0x40000]),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let w = world(e);
            set(e, 0x011f_91e4, c.word(0));
            let block = e.mem.alloc(0x14);
            set(
                e,
                block + 0x10,
                match c.word(1) {
                    0 => 0,
                    1 => w.alpha,
                    _ => w.skin,
                },
            );
            set(e, w.property + 0x20, c.word(2));
            set(e, block + 0xc, w.property);
            vec![0x55, block]
        })
        .run()
        .assert_same();
}

/// The alpha sort's depth less the radius in the exe's two steps: the sum stored as a float, then the
/// radius taken off. With the camera along (1, 1, 0) and the 53-bit word, a bound at (1, 2^-30, 0) of
/// radius 1 sums to 1 + 2^-30, stored as 1.0, so its depth is 0 like (1, 0, 0)'s: the two compare equal.
/// Without the store the depth would be 2^-30. Both orders; the 24-bit word rounds the sum anyway.
#[test]
fn compare_by_depth_stores_before_the_radius() {
    Diff::new(entry!(0x00b9_8b20, batch::compare_by_depth(u32, u32) -> i32))
        .common()
        .control_words(&[0x027f])
        .inputs(&[Input::Of(TWO_WAYS)])
        .setup(|e, c| {
            let w = world(e);
            set(e, w.camera + 0x68, ONE);
            set(e, w.camera + 0x74, ONE);
            set(e, w.camera + 0x80, 0);
            e.mem.set_u8(0x011f_fe35, 1);
            set(e, 0x011f_fe38, w.camera);
            let mut items = [0u32; 2];
            for (item, y) in items.iter_mut().zip([0x3080_0000, 0]) {
                let bound = e.mem.alloc(0x10);
                set(e, bound, ONE);
                set(e, bound + 4, y);
                set(e, bound + 0xc, ONE);
                let geometry = e.mem.alloc(0xc4);
                set(e, geometry + 0x20, bound);
                let pass = e.mem.alloc(8);
                set(e, pass, geometry);
                *item = e.mem.alloc(4);
                set(e, *item, pass);
            }
            if c.word(0) != 0 {
                items.swap(0, 1);
            }
            vec![items[0], items[1]]
        })
        .run()
        .assert_same();
}

/// `set_world_transform` with the render state cache's slot 0x30 holding the typed `00e879e0`: that call
/// made direct (the plain world has a stand-in there).
#[test]
fn world_transform_typed_slot() {
    Diff::new(entry!(
        0x00e6_fbb0,
        acc::set_world_transform(Ptr<_>, Ptr<_>, u8)
    ))
    .common()
    .inputs(&[Input::F32, Input::F32, Input::Of(BYTES)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let vt = vtable(e, &[(0x68, 0x00e8_8780), (0x30, 0x00e8_79e0)]);
        set(e, w.cache, vt);
        set(e, w.transform, bits(c, 0));
        set(e, w.transform + 0x30, bits(c, 1));
        vec![w.renderer, w.transform, c.word(2)]
    })
    .run()
    .assert_same();
}

/// `render_pass` with the geometry's slots 0x30 and 0x34 holding the leaf `00acbb70`, as every geometry
/// class has them: both calls made direct (the plain world has stand-ins there).
#[test]
fn render_pass_typed_geometry_slots() {
    Diff::new(entry!(
        0x00b9_94f0,
        batch::render_pass(Ptr<_>, u32, u32, u32, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2]),
        Input::Of(BYTES),
    ])
    .translation_calls_plainly(&[0x00b9_8e80, 0x00b9_9110, 0x00b9_9390])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        begun(e, &w, c.word(1));
        let pass = w.passes[c.word(0) as usize];
        let geometry = e.mem.u32(pass);
        let vt = vtable(e, &[(0x30, 0x00ac_bb70), (0x34, 0x00ac_bb70)]);
        set(e, geometry, vt);
        let pass_type = u32::from(e.mem.u16(pass + 4));
        vec![pass, pass_type, c.word(2), 1, c.word(2) ^ 1]
    })
    .run()
    .assert_same();
}

/// The culling walk's `AppendVirtual` (`00c4f1d0`) with the accumulator's slot 0x98 holding the typed
/// `RegisterObject` (`00b63f10`), or a stand-in: the call made direct, the culling family on too.
#[test]
fn append_virtual_registers_through_the_typed_slot() {
    Diff::new(entry!(
        0x00c4_f1d0,
        culling::process::append_virtual(Ptr<_>, Ptr<_>)
    ))
    .common()
    .family(&culling::CULLING)
    .inputs(&[Input::Of(TWO_WAYS), Input::Of(&[0, 1, 2])])
    .translation_calls_plainly(&[
        0x00b9_9840,
        0x00b9_9a50,
        0x00b9_9770,
        0x00b9_9a90,
        0x00b9_9b00,
        0x00b9_8e20,
    ])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world(e);
        let target = if c.word(0) != 0 {
            0x00b6_3f10
        } else {
            VT_FN + 0x30
        };
        let vt = vtable(e, &[(0x98, target)]);
        set(e, w.acc, vt);
        set(e, w.acc + 0x19c, c.word(1));
        let process = e.mem.alloc(0xc8);
        set(e, process + 0xc4, w.acc);
        vec![process, w.geometry]
    })
    .run()
    .assert_same();
}

/// Stand-ins for the untyped functions the render family's `GetRenderPasses` and geometry packing call,
/// for the two tests below: each answers 0 and leaves a mark.
fn render_stand_ins(e: &mut Engine) {
    for addr in [
        0x00ba_9660u32,
        0x00bd_f650,
        0x00ba_9740,
        0x00bd_a060,
        0x00ba_96d0,
        0x00bd_e170,
        0x00ba_9820,
        0x00bd_f6c0,
        0x00b7_0390,
        0x00a5_9d30,
        0x005b_f7b0,
        0x004b_3ab0,
        0x00e6_d760,
        0x00e6_d780,
        0x00e8_bf90,
        0x00e8_ef10,
        0x00e8_c1e0,
        0x00e8_f200,
        0x00e8_eee0,
        0x0043_b300,
        0x00a7_b850,
        0x00a7_5f10,
    ] {
        e.register(addr, |e, a| {
            let n = e.mem.u32(RETS + 0x700);
            e.mem.set_u32(
                RETS + 0x700,
                n.wrapping_mul(31).wrapping_add(a.len() as u32),
            );
            ret(0)
        });
    }
}

/// `RegisterObject` with the property's slot 0xa0 holding the render family's `GetRenderPasses`
/// (`00b67be0`): that call made direct (the plain world has a stand-in there).
#[test]
fn register_object_typed_render_passes() {
    Diff::new(entry!(
        0x00b6_3f10,
        acc::register_object(Ptr<_>, Ptr<_>) -> bool
    ))
    .common()
    .family(&render::RENDER)
    .family(&pathing::PATHING)
    .family(&crt::CRT)
    .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(TWO_WAYS)])
    .translation_calls_plainly(&[
        0x00b9_9840,
        0x00b9_9a50,
        0x00b9_9770,
        0x00b9_9a90,
        0x00b9_9b00,
        0x00b9_8e20,
        0x00b6_6760,
        0x0040_19d0,
        // The pass list's construction, as in `check_create_pass_array`.
        0x00ba_94b0,
        0x00ba_8b80,
        0x00ba_8d30,
    ])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world_with(e, true);
        render_stand_ins(e);
        let vt = vtable(e, &[(0xa0, 0x00b6_7be0)]);
        set(e, w.property, vt);
        set(e, w.acc + 0x19c, c.word(0));
        set(e, w.property + 0x20, c.word(1));
        vec![w.acc, w.geometry]
    })
    .run()
    .assert_same();
}

/// `render_standard` with the shader's slot 0x6c holding the render family's `00e812f0`: the draw made
/// direct (the plain world has a stand-in there).
#[test]
fn render_standard_typed_draw() {
    Diff::new(entry!(
        0x00b9_8e80,
        batch::render_standard(Ptr<_>, u32, u32, u32)
    ))
    .common()
    .family(&render::RENDER)
    .inputs(&[Input::Of(&[0x10, 0x11]), Input::Of(BYTES)])
    .float_sweep(0)
    .setup(|e, c| {
        let w = world_with(e, true);
        render_stand_ins(e);
        let vt = vtable(e, &[(0x6c, 0x00e8_12f0)]);
        set(e, w.shader, vt);
        let pass = w.passes[0];
        e.mem.set_u16(pass + 4, c.word(0) as u16);
        vec![pass, c.word(1), 0, 1]
    })
    .run()
    .assert_same();
}

/// A constant map's entries for `set_shader_constants`: `count` entries of 0x38 bytes, entry `i` of
/// kind `kinds[(i + rotate) % 8]`, enabled unless `(i + pattern) % 4` is 0, without a register when
/// `(i + pattern) % 3` is 0 and without a name for even `i`, of program type `(i + program) % 4`; with
/// `hole` one element null. Answers the elements.
fn constant_entries(e: &mut Engine, rotate: u32, pattern: u32, program: u32, hole: bool) -> u32 {
    const KINDS: [u32; 8] = [1, 2, 3, 4, 5, 6, 7, 0];
    const COUNT: u32 = 10;
    let elements = e.mem.alloc(4 * COUNT);
    for i in 0..COUNT {
        let entry = e.mem.alloc(0x38);
        e.mem.set_u8(entry + 8, u8::from((i + pattern) % 4 != 0));
        let kind = KINDS[((i + rotate) % 8) as usize];
        set(e, entry + 0x14, (kind << 28) | 0x0123_4560 | i);
        set(e, entry + 0x18, (i + program) % 4);
        let register = if (i + pattern) % 3 == 0 {
            u32::MAX
        } else {
            4 * i
        };
        set(e, entry + 0x1c, register);
        set(e, entry + 0x20, 1 + i % 3);
        set(
            e,
            entry + 0x24,
            if i % 2 == 0 { 0 } else { 0x0300_0800 + i },
        );
        set(e, entry + 0x30, 0x0300_0900 + 0x10 * i);
        let at = if hole && i == 3 { 0 } else { entry };
        set(e, elements + 4 * i, at);
    }
    elements
}

/// `NiD3DShaderConstantMap::SetShaderConstants`: every kind of entry in both arrays, enabled or not,
/// with or without a register or a name, each program type, a null element, and the entries' set-ups
/// and the device's uploads answering success, failure or (the device) a positive code.
#[test]
fn set_shader_constants() {
    Diff::new(entry!(
        0x00e8_26d0,
        constants::set_shader_constants(
            Ptr<_>,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
        ) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 3, 6]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2]),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1, 2, 3]),
    ])
    .setup(|e, c| {
        stand_ins(e, false);
        slots!(e; 0x90, 0x94, 0x98, 0x9c, 0x1b4);
        for slot in [0x8c, 0x90, 0x94, 0x98, 0x9c, 0xa4] {
            set(e, RETS + slot, c.word(2) & 1);
        }
        let device_answer = [0, 0x8000_0000, 5][c.word(2) as usize];
        set(e, RETS + 0x178, device_answer);
        set(e, RETS + 0x1b4, device_answer);
        let vt = vtable(e, &[]);
        let device = obj(e, 0x10, vt);
        let map = obj(e, 0x44, vt);
        let hole = c.word(3) != 0;
        let entries = constant_entries(e, c.word(0), c.word(1), c.word(4), hole);
        let defined = constant_entries(e, c.word(0) + 1, c.word(1) + 1, c.word(4) + 2, hole);
        set(e, map + 0x10, entries);
        e.mem.set_u16(map + 0x16, 10);
        set(e, map + 0x20, defined);
        e.mem.set_u16(map + 0x26, 10);
        set(e, map + 0x38, device);
        let mut words = vec![map];
        words.extend((0..11).map(|i| 0x0700_0000 + 0x100 * i));
        words
    })
    .run()
    .assert_same();
}

/// A bit pattern near `seed`, for the fields one swept float fills: `k` picks which (0 is the seed
/// itself; the others differ in the mantissa, the lowest exponent bit and, for some, the sign), so sums
/// and products of a group's fields round.
fn near(seed: u32, k: u32) -> u32 {
    seed ^ ((k.wrapping_mul(0x9e37_79b1) >> 8) & 0x00ff_ffff) ^ ((k & 4) << 29)
}

/// The floats a [`Shape`] holds, each the seed of a group of fields ([`near`]): a light's colour; the
/// camera and the camera object's position; the rows the light set-up leaves, the colour rows and the
/// HDR scale; the renderer's matrices and the geometry's transform; a light's clip planes and matrix; a
/// light's and the geometry's position; a light's radius and strength; the game time; the fade's start
/// time and duration; the eye, the plane with its distances and vertex entry 16's four floats; the
/// shader property's floats with the fade's pairs and the lone constants; the transform's scale; a
/// light's strength; the brightness threshold of the light set-up.
const F_COLOUR: usize = 0;
const F_CAMERA: usize = 1;
const F_ROWS: usize = 2;
const F_MATRIX: usize = 3;
const F_PLANES: usize = 4;
const F_POSITION: usize = 5;
const F_RADIUS: usize = 6;
const F_TIME: usize = 7;
const F_START: usize = 8;
const F_DURATION: usize = 9;
const F_EYE: usize = 10;
const F_PROPERTY: usize = 11;
const F_SCALE: usize = 12;
const F_STRENGTH: usize = 13;
const F_THRESHOLD: usize = 14;
const SHAPE_FLOATS: usize = 15;
/// Their plain values: 1, 1, 0.75, 0.5, 0.25, 3, 50, 1234.5, 1, 0.2, 1.5, 0.6, 2, 0.5, 0.
const PLAIN_FLOATS: [u32; SHAPE_FLOATS] = [
    ONE,
    ONE,
    0x3f40_0000,
    HALF,
    0x3e80_0000,
    0x4040_0000,
    0x4248_0000,
    0x449a_5000,
    ONE,
    0x3e4c_cccd,
    0x3fc0_0000,
    0x3f19_999a,
    TWO,
    HALF,
    0,
];

/// A [`Shape`]'s switches: the geometry has no position (+0x20 null, so `011f4288` is used); the
/// rendering object's byte +8 is set; both distances of pixel entry 19 are 0; the accumulator has no
/// shadow maps (+0x194 clear); the first light's clip planes are already made (+0x208); the first
/// light's byte +0xed is set; the rendering object has no light array (+0xc null); the set-ups that
/// take the rendering object get a copy with the lights while `011f91e0`'s own has no light array; the
/// lights have no data (+0xf8 null); the light set-up's bytes `011f91de` and `011f9427`; the lights'
/// byte +0xf5; the lights' ambient colour (-1, 0, 0) at strength 1, whose luminance is exactly 0 under
/// either control word.
const NO_POSITION: u32 = 1;
const RENDER_BYTE: u32 = 2;
const ZERO_RANGE: u32 = 0x10;
const NO_SHADOW_MAPS: u32 = 0x20;
const CLIP_CACHED: u32 = 0x100;
const LIGHT_ED: u32 = 0x200;
const NO_LIGHT_ARRAY: u32 = 0x400;
const SPLIT_BLOCK: u32 = 0x800;
const NO_LIGHT_DATA: u32 = 0x1000;
const SKIP_AMBIENT: u32 = 0x2000;
const NO_EXTRA_SCALE: u32 = 0x4000;
const LIGHT_AMBIENT: u32 = 0x8000;
const DARK_AMBIENT: u32 = 0x1_0000;

/// The texture switches (a [`Shape`]'s `textures`): the property's first texture is null; it has no
/// object at +0xdc, or one without a texture; its vtable +0xe0 is the real clamp mode (`00b671c0`), the
/// shader's +0x108 the real stage binding (`00be2170`) and its +0x154 the real `UpdateTogglesConstant`
/// (`00c02d90`), else stand-ins; the choice and filter bytes of the shared texture; the property's second
/// array is empty.
const TEX_NO_FIRST: u32 = 1;
const TEX_NO_OBJECT: u32 = 2;
const TEX_NO_OBJECT_TEXTURE: u32 = 4;
const TEX_REAL_SLOTS: u32 = 8;
const TEX_CHOICE: u32 = 0x10;
const TEX_FILTER: u32 = 0x20;
const TEX_NO_SECOND: u32 = 0x40;

/// The lights the world makes (the set-ups stop at 10).
const LIGHTS: u32 = 12;
/// The stand-ins of the shader property's vtable +0x110 and +0x10c: the floats at `RETS + 0x110` and
/// `RETS + 0x10c`, in ST0.
const FLOAT_SLOT: u32 = 0x00ff_2900;
const FLOAT_SLOT_10C: u32 = 0x00ff_2910;

/// What the set-up tests' world is made of ([`geometry_shape`]).
#[derive(Clone, Copy)]
struct Shape {
    /// The pass type (`011f91e4`) and the rendering object's own (+4).
    pass_type: u32,
    kind: u16,
    /// The rendering object's light count (+9) and selector byte (+0xb), the light whose pointer is
    /// null (none when past [`LIGHTS`]), which lights are point lights (bit `i` for light `i`), and
    /// what the first-active-light doubles answer: 0 none, 1 the first light, 2 the second.
    lights: u32,
    selector: u8,
    null_light: u32,
    points: u32,
    active: u32,
    /// The shader property's flags (+0x20, +0x24), its integer at +0xe4 and its word at +0x1c (a
    /// register for the shadow projection, its type for the light set-up).
    flags: u32,
    flags_24: u32,
    steps: u32,
    register: u32,
    /// The vertex and pixel entries: 0 off, 1 on, 2 every other one.
    entries: u32,
    skinned: bool,
    /// The fog property's floats (bits 0 and 1), the property's vtable +0xf4 answer, the other
    /// property's bit and a null pixel program (bit 0), a register of -1 and the real
    /// `SetShaderConstants` on the pixel map (bit 2).
    misc: u32,
    /// The shader mode (`011f91b8`), the HDR switch (`011f941e`) and the white ambient (`011f91a7`).
    mode: u32,
    hdr: bool,
    white: bool,
    /// The alpha property's flags (+0x18) and byte (+0x1a).
    alpha_flags: u16,
    alpha_byte: u8,
    /// [`NO_POSITION`] and the other switches; [`TEX_NO_FIRST`] and the other texture switches.
    bits: u32,
    textures: u32,
    floats: [u32; SHAPE_FLOATS],
}

impl Shape {
    /// One light, every entry on, nothing special.
    fn plain() -> Shape {
        Shape {
            pass_type: 0,
            kind: 0,
            lights: 1,
            selector: 0,
            null_light: u32::MAX,
            points: 0,
            active: 0,
            flags: 0,
            flags_24: 0x20,
            steps: 1,
            register: 1,
            entries: 1,
            skinned: false,
            misc: 0,
            mode: 0,
            hdr: false,
            white: false,
            alpha_flags: 0x032b,
            alpha_byte: 0x80,
            bits: 0,
            textures: 0,
            floats: PLAIN_FLOATS,
        }
    }

    /// The shape of a [`GEOMETRY_INPUTS`] case.
    fn of(c: &Case) -> Shape {
        let (misc, options) = (c.word(5), c.word(6));
        let mut floats = PLAIN_FLOATS;
        for (i, float) in floats.iter_mut().enumerate() {
            *float = bits(c, 7 + i);
        }
        let mut switches = options & (NO_POSITION | RENDER_BYTE | ZERO_RANGE | NO_SHADOW_MAPS);
        if options & RENDER_BYTE != 0 {
            switches |= CLIP_CACHED;
        }
        Shape {
            pass_type: c.word(0),
            kind: [0, 0xc5, 0x10, 0xd6][(misc & 3) as usize],
            lights: c.word(1),
            selector: (misc & 3) as u8,
            null_light: u32::MAX,
            points: if misc & 4 != 0 { u32::MAX } else { 2 },
            active: (misc >> 1) & 1,
            flags: c.word(2),
            flags_24: if misc & 1 != 0 { 0x28010 } else { 0x20 },
            steps: c.word(1),
            register: if misc & 4 != 0 { u32::MAX } else { 1 },
            entries: c.word(3),
            skinned: c.word(4) != 0,
            misc,
            mode: if misc & 2 != 0 { 0xf } else { 8 },
            hdr: misc & 1 != 0,
            white: misc & 4 != 0,
            alpha_flags: [0x032b, 0x012b, 0x012a, 0x032a][((options >> 2) & 3) as usize],
            alpha_byte: [0x80, 0, 0xff][(misc % 3) as usize],
            bits: switches,
            textures: if options & RENDER_BYTE != 0 {
                TEX_REAL_SLOTS
            } else {
                0
            },
            floats,
        }
    }
}

/// A light for the set-up tests: its matrix (+0x10), scale (+0xd0) and fourth colour word (+0xd4), the
/// bytes at +0xed, +0xf4 (a point light) and +0xf5, its shadow map (+0x10c, a rendered texture whose
/// texture is at +0x30), six clip planes (+0x144) and the made ones (+0x1a8) with their byte (+0x208),
/// and its data (+0xf8, or none: position +0x8c, strength +0xc4, ambient colour +0xc8, colour +0xd4,
/// radius +0xe0, direction +0xf0).
fn geometry_light(e: &mut Engine, i: u32, s: &Shape) -> u32 {
    let light = e.mem.alloc(0x220);
    let values = e.mem.alloc(0x100);
    for k in 0..16u32 {
        let word = near(s.floats[F_PLANES], 0x100 + 16 * i + k);
        set(e, light + 0x10 + 4 * k, word);
    }
    set(e, light + 0xd0, TWO);
    set(e, light + 0xd4, near(s.floats[F_RADIUS], 0x20 + i));
    e.mem
        .set_u8(light + 0xed, u8::from(i == 0 && s.bits & LIGHT_ED != 0));
    e.mem.set_u8(light + 0xf4, ((s.points >> i) & 1) as u8);
    e.mem
        .set_u8(light + 0xf5, u8::from(s.bits & LIGHT_AMBIENT != 0));
    let data = if s.bits & NO_LIGHT_DATA != 0 {
        0
    } else {
        values
    };
    set(e, light + 0xf8, data);
    let rendered = e.mem.alloc(0x40);
    set(e, rendered + 0x30, 0x7500 + i);
    set(e, light + 0x10c, rendered);
    for k in 0..24u32 {
        set(
            e,
            light + 0x144 + 4 * k,
            near(s.floats[F_PLANES], 1 + k + 32 * i),
        );
        set(
            e,
            light + 0x1a8 + 4 * k,
            near(s.floats[F_PLANES], 0x400 + k + 32 * i),
        );
    }
    let made = if i == 0 {
        s.bits & CLIP_CACHED != 0
    } else {
        i & 1 != 0
    };
    e.mem.set_u8(light + 0x208, u8::from(made));
    for k in 0..3u32 {
        let position = near(s.floats[F_POSITION], 1 + 3 * i + k);
        set(e, values + 0x8c + 4 * k, position);
        set(e, values + 0xf0 + 4 * k, position ^ 0x8000_0000);
        set(
            e,
            values + 0xd4 + 4 * k,
            near(s.floats[F_COLOUR], 3 * i + k),
        );
        set(
            e,
            values + 0xc8 + 4 * k,
            near(s.floats[F_COLOUR], 0x40 + 3 * i + k),
        );
    }
    set(e, values + 0xc4, near(s.floats[F_STRENGTH], i));
    set(e, values + 0xe0, near(s.floats[F_RADIUS], 0x10 * i));
    if s.bits & DARK_AMBIENT != 0 {
        set(e, values + 0xc4, ONE);
        set(e, values + 0xc8, 0xbf80_0000);
        set(e, values + 0xcc, 0);
        set(e, values + 0xd0, 0);
    }
    light
}

/// The words at `at`, `n` of them.
fn words_at(e: &Engine, at: u32, n: u32) -> Vec<u32> {
    (0..n).map(|k| e.mem.u32(at + 4 * k)).collect()
}

/// The calls the set-ups reach that stand in. The D3DX thunks move whole floats, so what a set-up
/// computes from their results is arithmetic on real numbers and every word they write is the same on
/// both sides: a vector function (out, in, ..) writes its three floats rotated by one, a plane's
/// normalize copies and its transform rotates the four, a matrix's transpose is the real one, its
/// multiply (out, a, b) writes `a` with `b`'s low bytes mixed in and its inverse (out, determinant, in)
/// writes the floats in reverse. The ambient and pass set-ups leave a mark of the call; the fog property
/// and the first active light answer from the table, the shader property's vtable +0x110 and +0x10c a
/// float each, the CRT's `_errno` a word of the table. The model matrix (`00c4c2d0`) and the per-light
/// set-up (`00b70820`) are the real ones: typed, in the dispatch and shader families.
fn geometry_stand_ins(e: &mut Engine) {
    for addr in [0x00ee_6df8u32, 0x00ee_6df2, 0x00ee_6dec] {
        e.register(addr, |e, a| {
            let v = words_at(e, a[1], 3);
            for k in 0..3u32 {
                e.mem.set_u32(a[0] + 4 * k, v[((k + 1) % 3) as usize]);
            }
            ret(a[0])
        });
    }
    e.register(0x00ee_6dce, |e, a| {
        let v = words_at(e, a[1], 4);
        for k in 0..4u32 {
            e.mem.set_u32(a[0] + 4 * k, v[k as usize]);
        }
        ret(a[0])
    });
    e.register(0x00ee_6dc8, |e, a| {
        let v = words_at(e, a[1], 4);
        for k in 0..4u32 {
            e.mem.set_u32(a[0] + 4 * k, v[((k + 1) % 4) as usize]);
        }
        ret(a[0])
    });
    e.register(0x00ee_6dd4, |e, a| {
        let m = words_at(e, a[1], 16);
        for k in 0..16u32 {
            e.mem
                .set_u32(a[0] + 4 * k, m[(4 * (k % 4) + k / 4) as usize]);
        }
        ret(a[0])
    });
    e.register(0x00ee_6de0, |e, a| {
        let (m, n) = (words_at(e, a[1], 16), words_at(e, a[2], 16));
        for k in 0..16u32 {
            let word = m[k as usize] ^ (n[((5 * k + 3) % 16) as usize] & 0xff);
            e.mem.set_u32(a[0] + 4 * k, word);
        }
        ret(a[0])
    });
    e.register(0x00ee_6dda, |e, a| {
        let m = words_at(e, a[2], 16);
        for k in 0..16u32 {
            e.mem.set_u32(a[0] + 4 * k, m[(15 - k) as usize]);
        }
        ret(a[0])
    });
    for addr in [0x00c0_3180u32, 0x00b7_ad00, 0x00b7_ce90, 0x00c0_2c30] {
        e.register(addr, |e, a| {
            let n = e.mem.u32(RETS + 0x500);
            e.mem.set_u32(
                RETS + 0x500,
                n.wrapping_mul(31).wrapping_add(a.len() as u32),
            );
            ret(0)
        });
    }
    e.register(0x00b5_5520, |e, _| ret(e.mem.u32(RETS + 0x504)));
    e.register(0x00b7_0590, |e, _| ret(e.mem.u32(RETS + 0x508)));
    e.register(0x00b7_0600, |e, _| ret(e.mem.u32(RETS + 0x508)));
    e.register(FLOAT_SLOT, |e, _| Ret {
        st0: f64::from(f32::from_bits(e.mem.u32(RETS + 0x110))),
        ..Ret::default()
    });
    e.register(FLOAT_SLOT_10C, |e, _| Ret {
        st0: f64::from(f32::from_bits(e.mem.u32(RETS + 0x10c))),
        ..Ret::default()
    });
    // The CRT's `_errno` (`00ec85e3`): a length that overflows sets it in the square root's error
    // path, which otherwise reads the thread's data through imports. A word of the table instead.
    e.register(0x00ec_85e3, |_, _| ret(RETS + 0x600));
    slots!(e; 0x154, 0x108);
}

/// The words and floats every set-up test varies: the pass type (one of each path: the full set-up,
/// 4, 6 and 8, the eye and clip plane ranges, 0x250), 0 to 3 lights, the property's flags, the vertex
/// and pixel entries on, off or alternating, a plain or skinned geometry, the switches (HDR, shader
/// mode 8 or 0xf, white ambient, the real `SetShaderConstants`), the options (no position, the
/// rendering object's byte, the alpha property's flags, zero distances, no shadow maps), a light's
/// colour and the camera's position, and the other floats of [`Shape`], swept.
const GEOMETRY_INPUTS: &[Input] = &[
    Input::Of(&[
        0, 4, 6, 8, 0x93, 0x1f6, 0x1f7, 0x231, 0x244, 0x248, 0x24b, 0x250,
    ]),
    Input::Of(&[1, 0, 2, 3]),
    Input::Of(&[0, 0x40400, 0x24000, 0x20400]),
    Input::Of(&[1, 0, 2]),
    Input::Of(TWO_WAYS),
    Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7]),
    Input::Of(&[0, 1, 2, 3, 4, 8, 0xc, 0x10, 0x20, 0x3f]),
    Input::F32,
    Input::F32,
    Input::F32Swept(PLAIN_FLOATS[F_ROWS]),
    Input::F32Swept(PLAIN_FLOATS[F_MATRIX]),
    Input::F32Swept(PLAIN_FLOATS[F_PLANES]),
    Input::F32Swept(PLAIN_FLOATS[F_POSITION]),
    Input::F32Swept(PLAIN_FLOATS[F_RADIUS]),
    Input::F32Swept(PLAIN_FLOATS[F_TIME]),
    Input::F32Swept(PLAIN_FLOATS[F_START]),
    Input::F32Swept(PLAIN_FLOATS[F_DURATION]),
    Input::F32Swept(PLAIN_FLOATS[F_EYE]),
    Input::F32Swept(PLAIN_FLOATS[F_PROPERTY]),
    Input::F32Swept(PLAIN_FLOATS[F_SCALE]),
    Input::F32Swept(PLAIN_FLOATS[F_STRENGTH]),
    Input::F32Swept(PLAIN_FLOATS[F_THRESHOLD]),
];

/// What a set-up test passes its function.
struct Geo {
    shader: u32,
    block: u32,
    /// The rendering object (`011f91e0`), and the one the set-ups are handed (the same unless
    /// [`SPLIT_BLOCK`]).
    rendering: u32,
    lit: u32,
    geometry: u32,
    property: u32,
    first_property: u32,
    pass_type: u32,
    skin: u32,
    fog: u32,
    /// A 4x4 matrix for the by-value set-ups.
    matrix: [u32; 16],
    /// The first light (also when the rendering object's pointer to it is null) and the current
    /// pass's stage 2.
    light: u32,
    stage: u32,
}

impl Geo {
    /// The geometry's transform.
    fn transform(&self) -> u32 {
        self.geometry + 0x68
    }
}

/// The world of the set-up tests for a [`GEOMETRY_INPUTS`] case.
fn geometry_world(e: &mut Engine, c: &Case) -> Geo {
    geometry_shape(e, &Shape::of(c))
}

/// The world of the set-up tests: [`world`] with everything the set-ups read made from `s`. The tests
/// name the render and dispatch families ([`Geometry::set_ups`]), so their functions are the real ones.
fn geometry_shape(e: &mut Engine, s: &Shape) -> Geo {
    let w = world_named(e, true, true);
    geometry_stand_ins(e);
    e.map(0, 0x1000);
    for (addr, len) in [
        (0x0102_0000, 0x1000),
        (0x0102_6000, 0x1000),
        (0x0104_e000, 0x1000),
        (0x0105_0000, 0x1000),
        (0x0107_8000, 0x1000),
        (0x010a_e000, 0x2000),
    ] {
        e.map(addr, len);
    }
    let seed = |i: usize, k: u32| near(s.floats[i], k);
    // The exe's constants: the doubles 1, 3600, 6.28318, 15, 5, 255, the luminance weights (0.32, 0.31,
    // 0.36 as floats), the light set-up's weights (0.34, 0.33) and offset (0.1), the floats -1 and 0.5.
    for (addr, value) in [
        (0x0101_2070u32, 1.0f64.to_bits()),
        (0x0101_2640, 3600.0f64.to_bits()),
        (0x0104_eea0, 0x4019_21fa_0000_0000),
        (0x0101_5a38, 15.0f64.to_bits()),
        (0x0102_0998, 5.0f64.to_bits()),
        (0x0101_e568, 255.0f64.to_bits()),
        (0x010a_e8b0, 0x3fd4_7ae1_4000_0000),
        (0x010a_e8b8, 0x3fd3_d70a_4000_0000),
        (0x010a_e8c0, 0x3fd7_0a3d_8000_0000),
        (0x0107_84d0, 0x3fd5_c28f_6000_0000),
        (0x0102_6998, 0x3fd5_1eb8_6000_0000),
        (0x0101_ffa0, 0x3fb9_9999_a000_0000),
    ] {
        e.mem.set_u64(addr, value);
    }
    set(e, 0x0101_2054, 0xbf80_0000);
    set(e, 0x0101_6248, HALF);
    set(e, 0x0105_0c98, 0x4110_0000);
    set(e, 0x010a_f450, 0x42c6_0000);
    set(e, 0x011f_91e4, s.pass_type);
    // The vertex and pixel entries (22 each): off, on, or every other one.
    let vt = vtable(e, &[]);
    for table in [0x011f_ebd8u32, 0x011f_ec30] {
        for i in 0..22u32 {
            let entry = obj(e, 0x38, vt);
            let on = match s.entries {
                0 => false,
                1 => true,
                _ => i % 2 == 0,
            };
            e.mem.set_u8(entry + 8, u8::from(on));
            set(e, entry + 0x20, i);
            set(e, table + 4 * i, entry);
        }
    }
    // The lights; what the light set-up leaves for each (positions `011fa170`, then directions
    // `011fa1f0`, which the positions past the eighth run into, as in the exe); the colour row the
    // lights scale and the HDR scale.
    let array = e.mem.alloc(4 * LIGHTS);
    let mut made = [0u32; 2];
    for i in 0..LIGHTS {
        let light = geometry_light(e, i, s);
        if i < 2 {
            made[i as usize] = light;
        }
        set(e, array + 4 * i, if i == s.null_light { 0 } else { light });
    }
    let [first, second] = made;
    for (rows, base) in [(0x011f_a170u32, 1u32), (0x011f_a1f0, 0x101)] {
        for k in 0..40u32 {
            set(e, rows + 4 * k, seed(F_ROWS, base + 2 * k));
        }
    }
    for k in 0..4u32 {
        set(e, 0x011f_a0d0 + 4 * k, seed(F_ROWS, 0x70 + k));
    }
    set(e, 0x011f_9184, seed(F_ROWS, 0x7f));
    // The geometry: its transform (a rotation, a translation, the scale), its position (or the one
    // used without, `011f4288`), its data's word.
    let geometry = if s.skinned { w.skinned } else { w.geometry };
    for i in 0..12u32 {
        set(e, geometry + 0x68 + 4 * i, seed(F_MATRIX, 0x40 + i));
    }
    set(e, geometry + 0x98, s.floats[F_SCALE]);
    let bound = e.mem.u32(geometry + 0x20);
    for k in 0..4u32 {
        set(e, bound + 4 * k, seed(F_POSITION, 0x40 + k));
        set(e, 0x011f_4288 + 4 * k, seed(F_POSITION, 0x50 + k));
    }
    if s.bits & NO_POSITION != 0 {
        set(e, geometry + 0x20, 0);
    }
    set(e, w.data + 0x34, 0x5a5a_0034);
    // The rendering object.
    let rendering = e.mem.alloc(0x40);
    set(e, rendering, geometry);
    e.mem.set_u16(rendering + 4, s.kind);
    e.mem
        .set_u8(rendering + 8, u8::from(s.bits & RENDER_BYTE != 0));
    e.mem.set_u8(rendering + 9, s.lights as u8);
    e.mem.set_u8(rendering + 0xb, s.selector);
    let lights = if s.bits & NO_LIGHT_ARRAY != 0 {
        0
    } else {
        array
    };
    set(e, rendering + 0xc, lights);
    set(e, 0x011f_91e0, rendering);
    let lit = if s.bits & SPLIT_BLOCK != 0 {
        let copy = e.mem.alloc(0x40);
        let bytes = e.mem.bytes(rendering, 0x40);
        e.mem.write(copy, &bytes);
        set(e, rendering + 0xc, 0);
        copy
    } else {
        rendering
    };
    // The camera object (a position, its light) and the camera; the renderer's matrices.
    let camera = e.mem.alloc(0x200);
    for k in 0..3u32 {
        set(e, camera + 0x1e4 + 4 * k, seed(F_CAMERA, k));
        set(e, 0x011f_474c + 4 * k, seed(F_CAMERA, 8 + k));
    }
    set(e, camera + 0xe0, first);
    set(e, 0x011f_91c8, camera);
    for k in 0..48u32 {
        set(e, w.renderer + 0x940 + 4 * k, seed(F_MATRIX, k));
    }
    // The accumulators: the shadow maps' word, the colour shader mode 0xf takes.
    set(e, w.acc + 0x194, u32::from(s.bits & NO_SHADOW_MAPS == 0));
    set(e, 0x011f_95ec, w.acc);
    let manager = e.mem.alloc(0x170);
    for k in 0..4u32 {
        set(e, manager + 0x154 + 4 * k, seed(F_ROWS, 0x60 + k));
    }
    set(e, 0x011f_95f0, manager);
    // The fog property, the first active light, the switches.
    let fog = e.mem.alloc(0x70);
    set(e, fog + 0x2c, if s.misc & 1 != 0 { ONE } else { 0 });
    set(e, fog + 0x30, if s.misc & 2 != 0 { 0x4500_0000 } else { 0 });
    set(e, RETS + 0x504, fog);
    set(e, RETS + 0x508, [0, first, second][s.active as usize]);
    set(e, RETS + 0xf4, s.misc & 1);
    e.mem.set_u8(0x011f_941e, u8::from(s.hdr));
    set(e, 0x011f_91b8, s.mode);
    e.mem.set_u8(0x011f_91a7, u8::from(s.white));
    // The light set-up's bytes, brightness threshold and extra scale.
    e.mem
        .set_u8(0x011f_91de, u8::from(s.bits & SKIP_AMBIENT != 0));
    e.mem
        .set_u8(0x011f_9427, u8::from(s.bits & NO_EXTRA_SCALE != 0));
    set(e, 0x011f_947c, s.floats[F_THRESHOLD]);
    set(e, 0x011f_9190, seed(F_ROWS, 0x7e));
    // The eye, the plane and its distances, vertex entry 16's floats; the times and the fade.
    for k in 0..3u32 {
        set(e, 0x011f_a2a0 + 4 * k, seed(F_EYE, k));
    }
    for k in 0..4u32 {
        set(e, 0x011f_9604 + 4 * k, seed(F_EYE, 4 + k));
        set(e, 0x011f_95f4 + 4 * k, seed(F_EYE, 12 + k));
    }
    let range = s.bits & ZERO_RANGE == 0;
    set(e, 0x011f_9614, if range { seed(F_EYE, 8) } else { 0 });
    set(e, 0x011f_9618, if range { seed(F_EYE, 9) } else { 0 });
    set(e, 0x011f_940c, s.floats[F_TIME]);
    set(e, 0x011f_94ac, s.floats[F_START]);
    set(e, 0x011a_d838, s.floats[F_DURATION]);
    e.mem.set_u8(0x011f_94aa, 1);
    for (k, addr) in [0x011f_962cu32, 0x011f_9630, 0x011f_9634, 0x011f_9638]
        .iter()
        .enumerate()
    {
        set(e, *addr, seed(F_PROPERTY, 0x24 + k as u32));
    }
    // The lone constants (`011ad808`, `010af2f0`, `010177e8`) and the toggle the set-up clears.
    set(e, 0x011a_d808, seed(F_PROPERTY, 0x21));
    set(e, 0x010a_f2f0, seed(F_PROPERTY, 0x22));
    set(e, 0x0101_77e8, seed(F_PROPERTY, 0x23));
    set(e, 0x011f_a2d4, ONE);
    // The shader property (its floats, its vtable +0x110 and +0x10c a float each, +0xe0 the real clamp
    // mode or a stand-in answering 5), the alpha property, the block.
    let real_slots = s.textures & TEX_REAL_SLOTS != 0;
    let mut slots = vec![(0x110u32, FLOAT_SLOT), (0x10c, FLOAT_SLOT_10C)];
    if real_slots {
        slots.push((0xe0, 0x00b6_71c0));
    }
    let property_vt = vtable(e, &slots);
    set(e, w.property, property_vt);
    set(e, RETS + 0x110, seed(F_PROPERTY, 0x20));
    set(e, RETS + 0x10c, seed(F_PROPERTY, 0x28));
    set(e, RETS + 0xe0, 5);
    set(e, w.property + 0x20, s.flags);
    set(e, w.property + 0x24, s.flags_24);
    set(e, w.property + 0xe4, s.steps);
    set(e, w.property + 0x1c, s.register);
    for (k, offset) in [
        0x28u32, 0x30, 0x6c, 0x80, 0x94, 0x98, 0x9c, 0xa0, 0xe0, 0xf0, 0xf4, 0xf8, 0xfc,
    ]
    .iter()
    .enumerate()
    {
        set(e, w.property + offset, seed(F_PROPERTY, k as u32));
    }
    e.mem.set_u16(w.alpha + 0x18, s.alpha_flags);
    e.mem.set_u8(w.alpha + 0x1a, s.alpha_byte);
    let other = obj(e, 0x1c, vt);
    e.mem.set_u8(other + 0x18, (s.misc & 1) as u8);
    let block = e.mem.alloc(0xa0);
    set(e, block, w.alpha);
    set(e, block + 8, 0x0100 | s.misc);
    set(e, block + 0xc, w.property);
    set(e, block + 0x10, other);
    set(e, block + 0x9c, w.alpha);
    // The shader: its renderer, the object at +0x1c, the pixel map (the real SetShaderConstants on
    // an empty map, or a stand-in) and the vertex map.
    set(e, w.shader + 0x18, w.renderer);
    let object = obj(e, 0x10, vt);
    set(e, w.shader + 0x1c, object);
    if s.misc & 4 != 0 {
        let map_vt = vtable(e, &[(0x78, 0x00e8_26d0)]);
        let map = obj(e, 0x44, map_vt);
        set(e, w.shader + 0x30, map);
    }
    if real_slots {
        // The shadow light shader's own stage binding and `UpdateTogglesConstant`.
        let shader_vt = vtable(e, &[(0x108, 0x00be_2170), (0x154, 0x00c0_2d90)]);
        set(e, w.shader, shader_vt);
    }
    let current = e.mem.u32(0x0126_f74c);
    set(e, current + 0x44, if s.misc & 1 != 0 { 0 } else { 0x4444 });
    // The current pass's stages (each with a state cache of its own; none marked for binding), the
    // property's texture arrays, its first texture and its object at +0xdc, its clamp mode, and the
    // textures the texture set-up falls back to.
    let stages = e.mem.alloc(0x40);
    let mut stage_2 = 0;
    for i in 0..16u32 {
        let stage = e.mem.alloc(0x70);
        let cache = e.mem.alloc(0xb8);
        set(e, stage + 4, i);
        set(e, stage + 0x10, cache);
        set(e, stages + 4 * i, stage);
        if i == 2 {
            stage_2 = stage;
        }
    }
    set(e, current + 0x18, 16);
    set(e, current + 0x24, stages);
    let (firsts, seconds) = (e.mem.alloc(0x20), e.mem.alloc(0x20));
    for i in 0..8u32 {
        set(e, firsts + 4 * i, 0x7100 + i);
        let second = if s.textures & TEX_NO_SECOND != 0 {
            0
        } else {
            0x7200 + i
        };
        set(e, seconds + 4 * i, second);
    }
    set(e, w.property + 0xac, firsts);
    set(e, w.property + 0xb0, seconds);
    let single = e.mem.alloc(8);
    set(
        e,
        single,
        if s.textures & TEX_NO_FIRST != 0 {
            0
        } else {
            0x7300
        },
    );
    set(e, w.property + 0xb4, single);
    let shadow = if s.textures & TEX_NO_OBJECT != 0 {
        0
    } else {
        let shadow = e.mem.alloc(0x70);
        let texture = if s.textures & TEX_NO_OBJECT_TEXTURE != 0 {
            0
        } else {
            0x7400
        };
        set(e, shadow + 0x6c, texture);
        shadow
    };
    set(e, w.property + 0xdc, shadow);
    set(e, w.property + 0xd8, u32::from(s.selector) % 8);
    for (addr, word) in [
        (0x011f_953cu32, 0x7601u32),
        (0x011f_9540, 0x7602),
        (0x011f_9428, 0x7603),
        (0x011f_951c, 0x7604),
        (0x011f_952c, 0x7605),
        (0x011f_9514, 0x7606),
        (0x011f_95d4, 0x7607),
    ] {
        set(e, addr, word);
    }
    e.mem
        .set_u8(0x011a_d80c, u8::from(s.textures & TEX_CHOICE != 0));
    e.mem
        .set_u8(0x011f_9425, u8::from(s.textures & TEX_FILTER != 0));
    Geo {
        shader: w.shader,
        block,
        rendering,
        lit,
        geometry,
        property: w.property,
        first_property: w.alpha,
        pass_type: s.pass_type,
        skin: e.mem.u32(geometry + 0xbc),
        fog: 0x0100 | s.misc,
        matrix: std::array::from_fn(|k| seed(F_MATRIX, 0x80 + k as u32)),
        light: first,
        stage: stage_2,
    }
}

/// What the translations of the dispatch and pathing callees the set-ups now reach for real call
/// plainly (a Rust call, not logged), as typed_dispatch.rs lists them: the model matrix's (`00c4c2d0`)
/// matrix row, columns and entry helpers (`00476930`, `00476980`, `004769c0`) and the square roots
/// (`004019d0`, `00408860`) under the unitize (`004a0c10`) the light set-up calls.
const SET_UP_PLAIN: &[u32] = &[
    0x0047_6930,
    0x0047_6980,
    0x0047_69c0,
    0x0040_19d0,
    0x0040_8860,
];

trait Geometry {
    /// The families a set-up's test names: the shader family and the leaves, and the dispatch, pathing
    /// and render families the set-ups call into (the model matrix and the point helpers, the
    /// accumulator, clamp mode and stage binding).
    fn set_ups(self) -> Diff;

    /// A set-up's test: [`GEOMETRY_INPUTS`] in [`geometry_world`], the words `args` makes of it.
    fn geometry(self, plainly: &[u32], args: fn(&Geo) -> Vec<u32>) -> Diff;

    /// A set-up's test of the branches [`GEOMETRY_INPUTS`] leaves out: its own `inputs` (at most
    /// `cases` ordinary cases), the [`Shape`] `shape` makes of a case, the words `args` makes of it.
    fn shaped(
        self,
        inputs: &[Input],
        cases: usize,
        shape: fn(&Case) -> Shape,
        args: fn(&Geo, &Case) -> Vec<u32>,
    ) -> Diff;
}

impl Geometry for Diff {
    fn set_ups(self) -> Diff {
        self.family(&shader::SHADER)
            .family(&leaf::LEAVES)
            .family(&dispatch::DISPATCH)
            .family(&pathing::PATHING)
            .family(&render::RENDER)
            .translation_calls_plainly(SET_UP_PLAIN)
            // The draw family's partition draw, skinned packer and strip lists, as in `common`.
            .goes_by_address(&[0x00e6_d310, 0x00e6_d780, 0x00a7_5f10])
            .max_calls(MAX_CALLS)
            .max_cases(160)
    }

    fn geometry(self, plainly: &[u32], args: fn(&Geo) -> Vec<u32>) -> Diff {
        self.set_ups()
            .translation_calls_plainly(plainly)
            .inputs(GEOMETRY_INPUTS)
            .max_cases(400)
            .float_sweep(FLOAT_SWEEP)
            .setup(move |e, c| {
                let g = geometry_world(e, c);
                args(&g)
            })
    }

    fn shaped(
        self,
        inputs: &[Input],
        cases: usize,
        shape: fn(&Case) -> Shape,
        args: fn(&Geo, &Case) -> Vec<u32>,
    ) -> Diff {
        self.set_ups()
            .inputs(inputs)
            .max_cases(cases)
            .float_sweep(FLOAT_SWEEP)
            .setup(move |e, c| {
                let g = geometry_shape(e, &shape(c));
                args(&g, c)
            })
    }
}

/// `ShadowLightShader::SetupGeometryConstants` with the constant set-ups it calls; the pixel map's
/// `SetShaderConstants` the real one or a stand-in. Fewer cases than the set-ups' own tests: each makes
/// many calls, and a test has to end well inside a minute (the cap a mutant's run gets).
#[test]
fn setup_geometry_constants() {
    Diff::new(entry!(
        0x00b7_dab0,
        geometry::setup_geometry_constants(Ptr<_>, Ptr<_>)
    ))
    .geometry(&[0x00b7_c750], |g| vec![g.shader, g.block])
    .max_cases(240)
    .run()
    .assert_same();
}

#[test]
fn vertex_entry_12() {
    Diff::new(entry!(0x00b7_9050, geometry::vertex_entry_12(Ptr<_>, u32)))
        .geometry(&[], |g| vec![g.shader, g.skin])
        .run()
        .assert_same();
}

#[test]
fn vertex_entry_15() {
    Diff::new(entry!(
        0x00b7_90d0,
        geometry::vertex_entry_15(Ptr<_>, Ptr<_>)
    ))
    .geometry(&[], |g| vec![g.shader, g.property])
    .run()
    .assert_same();
}

#[test]
fn vertex_entry_16() {
    Diff::new(entry!(0x00b7_9100, geometry::vertex_entry_16(Ptr<_>, u32)))
        .geometry(&[], |g| vec![g.shader, 0])
        .run()
        .assert_same();
}

#[test]
fn vertex_entry_17() {
    Diff::new(entry!(
        0x00b7_9180,
        geometry::vertex_entry_17(Ptr<_>, Ptr<_>)
    ))
    .geometry(&[], |g| vec![g.shader, g.geometry])
    .run()
    .assert_same();
}

#[test]
fn vertex_entry_21() {
    Diff::new(entry!(
        0x00b7_91f0,
        geometry::vertex_entry_21(Ptr<_>, u32, Ptr<_>, Ptr<_>)
    ))
    .geometry(&[], |g| vec![g.shader, g.pass_type, g.property, g.geometry])
    .run()
    .assert_same();
}

#[test]
fn pixel_entry_2() {
    Diff::new(entry!(
        0x00b7_9410,
        geometry::pixel_entry_2(Ptr<_>, u32, Ptr<_>)
    ))
    .geometry(&[], |g| vec![g.shader, g.pass_type, g.rendering])
    .run()
    .assert_same();
}

#[test]
fn toggles() {
    Diff::new(entry!(
        0x00b7_95b0,
        geometry::toggles(Ptr<_>, u32, u32, Ptr<_>, u32, u32, Ptr<_>)
    ))
    .geometry(&[], |g| {
        vec![
            g.shader,
            g.pass_type,
            g.geometry,
            g.property,
            g.fog,
            g.rendering,
            g.first_property,
        ]
    })
    .run()
    .assert_same();
}

#[test]
fn pixel_entry_16() {
    Diff::new(entry!(
        0x00b7_96f0,
        geometry::pixel_entry_16(Ptr<_>, Ptr<_>)
    ))
    .geometry(&[], |g| vec![g.shader, g.property])
    .run()
    .assert_same();
}

#[test]
fn pixel_entry_19() {
    Diff::new(entry!(0x00b7_9730, geometry::pixel_entry_19()))
        .geometry(&[], |_| vec![])
        .run()
        .assert_same();
}

#[test]
fn pixel_entry_20() {
    Diff::new(entry!(
        0x00b7_98c0,
        geometry::pixel_entry_20(Ptr<_>, Ptr<_>)
    ))
    .geometry(&[], |g| vec![g.shader, g.property])
    .run()
    .assert_same();
}

/// The 4x4 matrix the by-value set-ups are handed.
fn matrix_words(g: &Geo) -> Vec<u32> {
    g.matrix.to_vec()
}

#[test]
fn eye_pos() {
    Diff::new((0x00b7_8ef0, geometry::eye_pos_words as AbiFn))
        .geometry(&[], |g| {
            let mut words = vec![g.pass_type, g.transform()];
            words.extend(matrix_words(g));
            words
        })
        .run()
        .assert_same();
}

#[test]
fn shadow_proj() {
    Diff::new(entry!(
        0x00b7_b930,
        geometry::shadow_proj(Ptr<_>, Ptr<_>, u32, Ptr<_>)
    ))
    .geometry(&[], |g| vec![g.shader, g.block, g.skin, g.transform()])
    .run()
    .assert_same();
}

#[test]
fn obj_to_cube() {
    Diff::new(entry!(
        0x00b7_b730,
        geometry::obj_to_cube(Ptr<_>, Ptr<_>, u32, u32, u32)
    ))
    .geometry(&[], |g| {
        vec![g.shader, g.rendering, g.pass_type, g.skin, g.transform()]
    })
    .run()
    .assert_same();
}

#[test]
fn simple_shadow_clip_planes() {
    Diff::new(entry!(
        0x00b7_be00,
        geometry::simple_shadow_clip_planes(Ptr<_>, u32)
    ))
    .geometry(&[], |g| vec![g.shader, g.pass_type])
    .run()
    .assert_same();
}

#[test]
fn lights() {
    Diff::new((0x00b7_8a90, geometry::lights_words as AbiFn))
        .geometry(&[], |g| {
            let mut words = vec![g.property, g.rendering];
            words.extend(matrix_words(g));
            words.extend([g.transform(), g.pass_type, g.skin]);
            words
        })
        .run()
        .assert_same();
}

#[test]
fn ambient_emittance() {
    Diff::new(entry!(
        0x00b7_ba70,
        geometry::ambient_emittance(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, u32)
    ))
    .geometry(&[], |g| {
        vec![g.shader, g.property, g.rendering, g.geometry, g.fog]
    })
    .run()
    .assert_same();
}

// The set-ups' branches the shared inputs leave out, each with its own inputs.

/// `vertex_entry_21` by pass type: 0xfe, 0x61, the fade of 0x230 (before its start, part way, at its
/// end where it clears the running byte, with a zero or unusual duration) and the types beside them.
#[test]
fn vertex_entry_21_by_type() {
    Diff::new(entry!(
        0x00b7_91f0,
        geometry::vertex_entry_21(Ptr<_>, u32, Ptr<_>, Ptr<_>)
    ))
    .shaped(
        &[
            Input::Of(&[0x230, 0xfe, 0x61, 0x22f, 0x231, 0xff, 0x60, 0]),
            Input::Of(&[1, 0, 2]),
            Input::F32,
            Input::Of(&[
                HALF,
                0x3e4c_cccd,
                ONE,
                0,
                0x8000_0000,
                0xbf80_0000,
                0x7f80_0000,
                0x7fc0_0000,
                0x7f80_0001,
            ]),
            Input::F32Swept(TWO),
            Input::F32Swept(PLAIN_FLOATS[F_PROPERTY]),
            Input::F32Swept(PLAIN_FLOATS[F_MATRIX]),
        ],
        480,
        |c| {
            let mut s = Shape::plain();
            s.pass_type = c.word(0);
            s.entries = c.word(1);
            s.floats[F_START] = bits(c, 2);
            s.floats[F_DURATION] = c.word(3);
            s.floats[F_TIME] = bits(c, 4);
            s.floats[F_PROPERTY] = bits(c, 5);
            s.floats[F_MATRIX] = bits(c, 6);
            s
        },
        |g, _| vec![g.shader, g.pass_type, g.property, g.geometry],
    )
    .run()
    .assert_same();
}

/// `pixel_entry_2` by pass type: 0x93 and 0x1f1 to 0x1f5 (rows 2 and 3, the register count from the
/// light count), 0x1f6 (rows 0 and 0x1b) and the types beside them, with every selector.
#[test]
fn pixel_entry_2_by_type() {
    Diff::new(entry!(
        0x00b7_9410,
        geometry::pixel_entry_2(Ptr<_>, u32, Ptr<_>)
    ))
    .shaped(
        &[
            Input::Of(&[
                0x93, 0x1f1, 0x1f3, 0x1f5, 0x1f6, 0x1f0, 0x1f7, 0x92, 0x94, 0,
            ]),
            Input::Of(&[0, 1, 2, 0xff]),
            Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
        ],
        400,
        |c| {
            let mut s = Shape::plain();
            s.pass_type = c.word(0);
            s.lights = c.word(1);
            s.selector = c.word(2) as u8;
            s
        },
        |g, _| vec![g.shader, g.pass_type, g.rendering],
    )
    .run()
    .assert_same();
}

/// `obj_to_cube` by pass type (0xfe, 0x10b, 0x2d8 to 0x2ef, 0x244 to 0x24b and the types beside
/// them) and by what it is handed: a first light with its byte +0xed clear or set, no light array, no
/// lights, a null first light, and a rendering object (`011f91e0`) without the light array.
#[test]
fn obj_to_cube_by_type() {
    Diff::new(entry!(
        0x00b7_b730,
        geometry::obj_to_cube(Ptr<_>, Ptr<_>, u32, u32, u32)
    ))
    .shaped(
        &[
            Input::Of(&[
                0, 0xfe, 0x10b, 0x2d8, 0x2ef, 0x2f0, 0x2d7, 0x244, 0x24b, 0x24c, 0x243, 0xff,
                0x10a, 0x10c,
            ]),
            Input::Of(&[0, 1, 2, 3, 4, 5]),
            Input::Of(TWO_WAYS),
            Input::Of(&[1, 0]),
            Input::F32Swept(PLAIN_FLOATS[F_CAMERA]),
            Input::F32Swept(PLAIN_FLOATS[F_MATRIX]),
            Input::F32Swept(PLAIN_FLOATS[F_PLANES]),
        ],
        400,
        |c| {
            let mut s = Shape::plain();
            s.pass_type = c.word(0);
            match c.word(1) {
                0 => {}
                1 => s.bits |= LIGHT_ED,
                2 => s.bits |= NO_LIGHT_ARRAY,
                3 => {
                    s.lights = 0;
                    s.bits |= LIGHT_ED;
                }
                4 => s.null_light = 0,
                _ => s.bits |= SPLIT_BLOCK,
            }
            s.skinned = c.word(2) != 0;
            s.entries = c.word(3);
            s.floats[F_CAMERA] = bits(c, 4);
            s.floats[F_MATRIX] = bits(c, 5);
            s.floats[F_PLANES] = bits(c, 6);
            s
        },
        |g, _| vec![g.shader, g.lit, g.pass_type, g.skin, g.transform()],
    )
    .run()
    .assert_same();
}

/// `simple_shadow_clip_planes` for each type of its range and the ones beside it, with the first
/// light's planes already made (the device gets the kept ones) or not (they are made from the view
/// matrix, the camera and the light's planes, set, kept and marked).
#[test]
fn simple_shadow_clip_planes_made_or_kept() {
    Diff::new(entry!(
        0x00b7_be00,
        geometry::simple_shadow_clip_planes(Ptr<_>, u32)
    ))
    .shaped(
        &[
            Input::Of(&[0x231, 0x232, 0x233, 0x234, 0x230, 0x235]),
            Input::Of(TWO_WAYS),
            Input::F32,
            Input::F32Swept(PLAIN_FLOATS[F_MATRIX]),
            Input::F32Swept(PLAIN_FLOATS[F_PLANES]),
        ],
        400,
        |c| {
            let mut s = Shape::plain();
            s.pass_type = c.word(0);
            if c.word(1) != 0 {
                s.bits |= CLIP_CACHED;
            }
            s.floats[F_CAMERA] = bits(c, 2);
            s.floats[F_MATRIX] = bits(c, 3);
            s.floats[F_PLANES] = bits(c, 4);
            s
        },
        |g, _| vec![g.shader, g.pass_type],
    )
    .run()
    .assert_same();
}

/// `lights` past three lights and in every mode: the rendering object's types around 0x10 to 0x13
/// and 0x231 to 0x233 (the scale), up to 12 lights (10 are set up), a null light pointer, point or
/// directional lights, shader modes 0, 10 (the half-grey row) and 0xf, pass type 0x24b or not, a
/// skinned geometry, HDR with the property's bit, an active light, entries on or alternating.
#[test]
fn lights_by_count_and_mode() {
    Diff::new((0x00b7_8a90, geometry::lights_words as AbiFn))
        .shaped(
            &[
                Input::Of(&[0x231, 0x232, 0x233, 0x234, 0x230, 0x10, 0x13, 0x14, 0xf, 0]),
                Input::Of(&[3, 0, 1, 9, 10, 11, 12]),
                Input::Of(&[0xff, 0, 1, 2, 9]),
                Input::Of(&[u32::MAX, 0, 0x2aa]),
                Input::Of(&[0, 10, 0xf]),
                Input::Of(&[0x24b, 0]),
                Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
                Input::F32,
                Input::F32Swept(PLAIN_FLOATS[F_ROWS]),
                Input::F32Swept(PLAIN_FLOATS[F_CAMERA]),
                Input::F32Swept(PLAIN_FLOATS[F_MATRIX]),
                Input::F32Swept(PLAIN_FLOATS[F_PROPERTY]),
            ],
            320,
            |c| {
                let mut s = Shape::plain();
                s.kind = c.word(0) as u16;
                s.lights = c.word(1);
                s.null_light = c.word(2);
                s.points = c.word(3);
                s.mode = c.word(4);
                s.pass_type = c.word(5);
                let switches = c.word(6);
                s.skinned = switches & 1 != 0;
                if switches & 2 != 0 {
                    s.hdr = true;
                    s.flags = 0x400;
                }
                s.active = (switches >> 2) & 1;
                s.entries = if switches & 8 != 0 { 2 } else { 1 };
                s.floats[F_SCALE] = bits(c, 7);
                s.floats[F_ROWS] = bits(c, 8);
                s.floats[F_CAMERA] = bits(c, 9);
                s.floats[F_MATRIX] = bits(c, 10);
                s.floats[F_PROPERTY] = bits(c, 11);
                s
            },
            |g, _| {
                let mut words = vec![g.property, g.rendering];
                words.extend(matrix_words(g));
                words.extend([g.transform(), g.pass_type, g.skin]);
                words
            },
        )
        .run()
        .assert_same();
}

/// `ambient_emittance`'s luminance block and its endings: the rendering object's type and the pass
/// type in and beside 0x244 to 0x24b, the property's bits that gate the block, shader modes 0, 8 and
/// 10 (10 leaves before the last factor), no active light, one, or one with the geometry's position
/// missing, pixel entry 3 on or off; the lights' colours, strength, radius and positions swept.
#[test]
fn ambient_emittance_luminance() {
    Diff::new(entry!(
        0x00b7_ba70,
        geometry::ambient_emittance(Ptr<_>, Ptr<_>, Ptr<_>, Ptr<_>, u32)
    ))
    .shaped(
        &[
            Input::Of(&[0x244, 0x24b, 0x243, 0x24c]),
            Input::Of(&[0x24b, 0x244, 0x243, 0x24c]),
            Input::Of(&[0, 0x20000]),
            Input::Of(&[0x8000, 0x28000, 0x20]),
            Input::Of(&[0, 8, 10]),
            Input::Of(&[1, 0, 2]),
            Input::Of(&[1, 0]),
            Input::F32Swept(PLAIN_FLOATS[F_COLOUR]),
            Input::F32Swept(PLAIN_FLOATS[F_POSITION]),
            Input::F32Swept(PLAIN_FLOATS[F_RADIUS]),
            Input::F32Swept(PLAIN_FLOATS[F_PROPERTY]),
            Input::F32Swept(PLAIN_FLOATS[F_STRENGTH]),
        ],
        280,
        |c| {
            let mut s = Shape::plain();
            s.kind = c.word(0) as u16;
            s.pass_type = c.word(1);
            s.flags = c.word(2);
            s.flags_24 = c.word(3);
            s.mode = c.word(4);
            s.active = u32::from(c.word(5) != 0);
            if c.word(5) == 2 {
                s.bits |= NO_POSITION;
            }
            s.entries = c.word(6);
            s.floats[F_COLOUR] = bits(c, 7);
            s.floats[F_POSITION] = bits(c, 8);
            s.floats[F_RADIUS] = bits(c, 9);
            s.floats[F_PROPERTY] = bits(c, 10);
            s.floats[F_STRENGTH] = bits(c, 11);
            s
        },
        |g, _| vec![g.shader, g.property, g.rendering, g.geometry, g.fog],
    )
    .float_sweep(2 * FLOAT_SWEEP)
    .run()
    .assert_same();
}

/// `SetupGeometryConstants` for the pass types the shared inputs leave out (0xfe, 0x61, 0x230, 0x10b,
/// 0x2d8 to 0x2ef, 0x1f1 to 0x1f5, the clip planes' and the eye's ranges, 5 and 0x251), the rendering
/// object's type the pass type or another, up to 12 lights with one null, every shader mode, the first
/// light's clip planes made or not.
#[test]
fn setup_geometry_constants_other_types() {
    Diff::new(entry!(
        0x00b7_dab0,
        geometry::setup_geometry_constants(Ptr<_>, Ptr<_>)
    ))
    .shaped(
        &[
            Input::Of(&[
                0xfe, 0x61, 0x230, 0x10b, 0x2d8, 0x2ef, 0x1f1, 0x1f5, 0x231, 0x233, 0x234, 0x244,
                0x24b, 5, 8, 0x251,
            ]),
            Input::Of(&[1, 0x10, 0xc5, 0x231, 0x24b]),
            Input::Of(&[1, 0, 3, 10, 12]),
            Input::Of(&[0xff, 1, 2]),
            Input::Of(&[0, 0x40400, 0x24000, 0x20400]),
            Input::Of(&[0, 8, 10, 0xf]),
            Input::Of(&[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29, 30, 31,
            ]),
            Input::F32,
            Input::F32,
            Input::F32Swept(PLAIN_FLOATS[F_ROWS]),
            Input::F32Swept(PLAIN_FLOATS[F_MATRIX]),
            Input::F32Swept(PLAIN_FLOATS[F_PLANES]),
            Input::F32Swept(PLAIN_FLOATS[F_POSITION]),
            Input::F32Swept(PLAIN_FLOATS[F_RADIUS]),
            Input::F32Swept(PLAIN_FLOATS[F_TIME]),
            Input::F32Swept(PLAIN_FLOATS[F_START]),
            Input::F32Swept(PLAIN_FLOATS[F_DURATION]),
            Input::F32Swept(PLAIN_FLOATS[F_EYE]),
            Input::F32Swept(PLAIN_FLOATS[F_PROPERTY]),
            Input::F32Swept(PLAIN_FLOATS[F_SCALE]),
            Input::F32Swept(PLAIN_FLOATS[F_STRENGTH]),
            Input::F32Swept(PLAIN_FLOATS[F_THRESHOLD]),
        ],
        240,
        |c| {
            let mut s = Shape::plain();
            s.pass_type = c.word(0);
            s.kind = if c.word(1) == 1 {
                c.word(0) as u16
            } else {
                c.word(1) as u16
            };
            s.lights = c.word(2);
            s.null_light = c.word(3);
            s.flags = c.word(4);
            s.mode = c.word(5);
            let switches = c.word(6);
            s.skinned = switches & 1 != 0;
            s.hdr = switches & 2 != 0;
            s.active = (switches >> 2) & 1;
            if switches & 8 != 0 {
                s.bits |= CLIP_CACHED;
            }
            s.flags_24 = if switches & 16 != 0 { 0x8010 } else { 0x20 };
            s.points = if switches & 1 != 0 { 0x2aa } else { u32::MAX };
            s.misc = switches & 4;
            for (i, float) in s.floats.iter_mut().enumerate() {
                *float = bits(c, 7 + i);
            }
            s
        },
        |g, _| vec![g.shader, g.block],
    )
    .translation_calls_plainly(&[0x00b7_c750])
    .run()
    .assert_same();
}

// The shadow light shader's per-geometry textures.

/// Every type the rendering object can have, and a few past the exe's table.
const EVERY_TYPE: [u32; 0x2f0] = {
    let mut types = [0u32; 0x2f0];
    let mut i = 0;
    while i < 0x2f0 {
        types[i] = i as u32;
        i += 1;
    }
    types
};

/// A type or two of each body of the texture set-up, then types outside its table.
const BODY_TYPES: &[u32] = &[
    0x2, 0x2eb, 0x4, 0x5, 0x59, 0x1f4, 0x64, 0x235, 0x66, 0x91, 0x68, 0x7c, 0x1e4, 0x93, 0x1f6,
    0x95, 0x10b, 0x96, 0x97, 0x12b, 0xc4, 0x128, 0xd8, 0xfe, 0x14b, 0x14c, 0x155, 0x24b, 0x1f7,
    0x1ff, 0x207, 0x20f, 0x217, 0x21f, 0x227, 0x22e, 0x230, 0x231, 0x233, 0x244, 0x247, 0x248,
    0x24a, 0x2d6, 0x2d7, 0, 1, 3, 0x2ec, 0x22f, 0x234,
];

/// A shape for the texture tests: the rendering object's type and selector byte, the texture
/// switches (bit 1 of them also makes the property's vtable +0xf4 answer a texture), and with `second`
/// clear the property's bit that keeps its second array off the stages.
fn texture_shape(kind: u32, selector: u32, switches: u32, second: bool) -> Shape {
    let mut s = Shape::plain();
    s.kind = kind as u16;
    s.selector = selector as u8;
    s.textures = switches;
    s.misc = (switches >> 1) & 1;
    if !second {
        s.flags_24 |= 0x10_0000;
    }
    s
}

/// The texture set-up for every type there is, with the shadow light shader's own slots: each of the
/// exe's 29 bodies and every type outside its table.
#[test]
fn setup_geometry_textures_every_type() {
    Diff::new(entry!(
        0x00b7_d200,
        textures::setup_geometry_textures(Ptr<_>, Ptr<_>)
    ))
    .shaped(
        &[Input::Of(&EVERY_TYPE)],
        0x2f0,
        |c| texture_shape(c.word(0), 1, TEX_REAL_SLOTS, true),
        |g, _| vec![g.shader, g.block],
    )
    // No float arithmetic in the set-up or its stage set-ups (they move words): no float sweep and one
    // control word, so a run ends inside the minute a mutant's run gets.
    .float_sweep(0)
    .control_words(&[0x027f])
    .run()
    .assert_same();
}

/// The texture set-up's bodies with everything they read varied: the selector byte, a null first
/// texture, the object at +0xdc missing or without a texture, the second array kept off the stages or
/// empty, the property's answers, the shared texture's choice and filter bytes, real or stand-in slots.
#[test]
fn setup_geometry_textures() {
    Diff::new(entry!(
        0x00b7_d200,
        textures::setup_geometry_textures(Ptr<_>, Ptr<_>)
    ))
    .shaped(
        &[
            Input::Of(BODY_TYPES),
            Input::Of(&[0, 1, 3]),
            Input::Of(&[
                0,
                TEX_NO_FIRST,
                TEX_NO_OBJECT,
                TEX_NO_OBJECT_TEXTURE,
                TEX_REAL_SLOTS,
                TEX_CHOICE,
                TEX_FILTER,
                TEX_NO_SECOND,
                TEX_REAL_SLOTS | TEX_NO_FIRST | TEX_FILTER,
                TEX_REAL_SLOTS | TEX_CHOICE | TEX_NO_SECOND | TEX_NO_OBJECT,
            ]),
            Input::Of(TWO_WAYS),
        ],
        300,
        |c| texture_shape(c.word(0), c.word(1), c.word(2), c.word(3) != 0),
        |g, _| vec![g.shader, g.block],
    )
    .float_sweep(0)
    .control_words(&[0x027f])
    .run()
    .assert_same();
}

/// The inputs of a stage set-up's test: the stage, the texture index or word, the texture switches.
const STAGE_INPUTS: &[Input] = &[
    Input::Of(&[0, 1, 6, 15]),
    Input::Of(&[0, 1, 7]),
    Input::Of(&[
        0,
        TEX_REAL_SLOTS,
        TEX_NO_SECOND,
        TEX_NO_OBJECT,
        TEX_CHOICE,
        TEX_FILTER,
        TEX_CHOICE | TEX_FILTER | TEX_REAL_SLOTS,
    ]),
];

fn stage_shape(c: &Case) -> Shape {
    texture_shape(0, c.word(1), c.word(2), true)
}

#[test]
fn stage_texture() {
    Diff::new(entry!(
        0x00b7_c0a0,
        textures::stage_texture(Ptr<_>, u32, u32)
    ))
    .shaped(STAGE_INPUTS, 100, stage_shape, |g, c| {
        vec![g.property, c.word(0), c.word(1)]
    })
    .run()
    .assert_same();
}

/// With a property (its second array's texture, or a default one by its flag) and without one.
#[test]
fn stage_second_texture() {
    Diff::new(entry!(
        0x00b7_c0e0,
        textures::stage_second_texture(Ptr<_>, u32, u32)
    ))
    .shaped(
        &[
            Input::Of(&[0, 1, 6, 15]),
            Input::Of(&[0, 1, 7]),
            Input::Of(&[
                0,
                TEX_REAL_SLOTS,
                TEX_NO_SECOND,
                TEX_NO_SECOND | TEX_REAL_SLOTS,
            ]),
            Input::Of(&[0, 0x80]),
            Input::Of(TWO_WAYS),
        ],
        200,
        |c| {
            let mut s = texture_shape(0, c.word(1), c.word(2), true);
            s.flags = c.word(3);
            s
        },
        |g, c| {
            let property = if c.word(4) != 0 { 0 } else { g.property };
            vec![property, c.word(0), c.word(1)]
        },
    )
    .run()
    .assert_same();
}

#[test]
fn stage_light_texture() {
    Diff::new(entry!(0x00b7_c120, textures::stage_light_texture(u32)))
        .shaped(STAGE_INPUTS, 100, stage_shape, |_, c| vec![c.word(0)])
        .run()
        .assert_same();
}

#[test]
fn stage_shared_texture() {
    Diff::new(entry!(0x00b7_c190, textures::stage_shared_texture(u32)))
        .shaped(STAGE_INPUTS, 100, stage_shape, |_, c| vec![c.word(0)])
        .run()
        .assert_same();
}

/// The property's answer for kind 4 null (the default texture) or not.
#[test]
fn stage_fourth_texture() {
    Diff::new(entry!(
        0x00b7_c290,
        textures::stage_fourth_texture(u32, u32, u32)
    ))
    .shaped(STAGE_INPUTS, 100, stage_shape, |g, c| {
        vec![g.property, c.word(0), c.word(1)]
    })
    .run()
    .assert_same();
}

/// The property's answer for kind 5 null (the fallback texture) or not.
#[test]
fn stage_fifth_texture() {
    Diff::new(entry!(
        0x00b7_c2c0,
        textures::stage_fifth_texture(Ptr<_>, u32, u32)
    ))
    .shaped(STAGE_INPUTS, 100, stage_shape, |g, c| {
        vec![g.property, c.word(0), c.word(1)]
    })
    .run()
    .assert_same();
}

/// A stage and a property, either or both null.
#[test]
fn stage_clamp() {
    Diff::new(entry!(0x00b7_1b10, textures::stage_clamp(Ptr<_>, u32, u32)))
        .shaped(
            &[
                Input::Of(&[0, 1, 2, 3]),
                Input::Of(&[0, TEX_REAL_SLOTS]),
                Input::Of(&[0, 3, 7]),
            ],
            100,
            |c| texture_shape(0, c.word(2), c.word(1), true),
            |g, c| {
                let stage = if c.word(0) & 1 != 0 { 0 } else { g.stage };
                let property = if c.word(0) & 2 != 0 { 0 } else { g.property };
                vec![g.shader, stage, property]
            },
        )
        .run()
        .assert_same();
}

// The lighting shaders' per-light set-up.

/// `light_setup` (`00b70820`): an index below, at and past 8; no light, a directional one or a point
/// light, with or without data; the shader property's type inside or beside 8 to 12 (its vtable +0x10c
/// then scales); the HDR, skip-ambient and no-extra-scale bytes and the light's ambient byte; the pass
/// type inside or beside 0x154 to 0x177; the scale, the strength and the brightness threshold as floats
/// with every special value, the colours swept.
#[test]
fn light_setup() {
    Diff::new(entry!(0x00b7_0820, lighting::light_setup(u32, Ptr<_>, f32)))
        .shaped(
            &[
                Input::Of(&[0, 1, 7, 8, 9]),
                Input::Of(&[1, 0, 2, 3, 4]),
                Input::Of(&[8, 12, 7, 13, 1, 0x8000_0008]),
                Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
                Input::Of(&[0x154, 0x177, 0x153, 0x178, 0]),
                Input::F32,
                Input::F32,
                Input::F32,
                Input::F32Swept(PLAIN_FLOATS[F_COLOUR]),
                Input::F32Swept(PLAIN_FLOATS[F_POSITION]),
                Input::F32Swept(PLAIN_FLOATS[F_ROWS]),
                Input::F32Swept(PLAIN_FLOATS[F_PROPERTY]),
                Input::F32Swept(PLAIN_FLOATS[F_RADIUS]),
            ],
            160,
            |c| {
                let mut s = Shape::plain();
                // 0 no light, 1 directional, 2 point, 3 directional without data, 4 point without data.
                let light = c.word(1);
                s.points = u32::from(light == 2 || light == 4);
                if light >= 3 {
                    s.bits |= NO_LIGHT_DATA;
                }
                s.register = c.word(2);
                let switches = c.word(3);
                s.hdr = switches & 1 != 0;
                if switches & 2 != 0 {
                    s.bits |= SKIP_AMBIENT;
                }
                if switches & 4 != 0 {
                    s.bits |= NO_EXTRA_SCALE;
                }
                if switches & 8 != 0 {
                    s.bits |= LIGHT_AMBIENT;
                }
                s.pass_type = c.word(4);
                s.floats[F_STRENGTH] = bits(c, 6);
                s.floats[F_THRESHOLD] = bits(c, 7);
                s.floats[F_COLOUR] = bits(c, 8);
                s.floats[F_POSITION] = bits(c, 9);
                s.floats[F_ROWS] = bits(c, 10);
                s.floats[F_PROPERTY] = bits(c, 11);
                s.floats[F_RADIUS] = bits(c, 12);
                s
            },
            |g, c| {
                let light = if c.word(1) == 0 { 0 } else { g.light };
                vec![c.word(0), light, bits(c, 5)]
            },
        )
        .run()
        .assert_same();
}

/// `light_setup`'s brightness step at a luminance of exactly 0 (the ambient colour (-1, 0, 0) at
/// strength 1: -1 times 0.33, plus 0.33, is exactly 0 at any precision, the
/// weight being a float held as a double): every component becomes the threshold. Floats the sweep
/// alone almost never meets.
#[test]
fn light_setup_luminance_zero() {
    Diff::new(entry!(0x00b7_0820, lighting::light_setup(u32, Ptr<_>, f32)))
        .shaped(
            &[
                Input::Of(&[0, 5]),
                Input::Of(&[1, 2]),
                Input::Of(TWO_WAYS),
                Input::Of(&[ONE, HALF, TWO, 0, 0xbf80_0000]),
                Input::Of(&[ONE, HALF, TWO]),
            ],
            60,
            |c| {
                let mut s = Shape::plain();
                s.points = u32::from(c.word(1) == 2);
                s.bits |= DARK_AMBIENT;
                s.register = 8;
                s.hdr = c.word(2) != 0;
                s.floats[F_THRESHOLD] = c.word(3);
                s
            },
            |g, c| vec![c.word(0), g.light, c.word(4)],
        )
        .float_sweep(0)
        .run()
        .assert_same();
}

// The batch renderer with the shadow light shader's own slots.

/// The pass for the batch renderer's typed-slot tests: the set-ups' world with the shader's slots 0x78,
/// 0x7c and 0x8c holding the shadow light shader's texture set-up, constants and state restore, the
/// rendering object a pass of the batch renderer's own (its geometry the world's, plain or skinned).
fn typed_setups_pass(e: &mut Engine, c: &Case) -> u32 {
    let mut s = texture_shape(c.word(0), 1, 0, true);
    s.pass_type = c.word(0);
    s.skinned = c.word(1) != 0;
    s.lights = 2;
    let g = geometry_shape(e, &s);
    let vt = vtable(
        e,
        &[
            (0x78, 0x00b7_d200),
            (0x7c, 0x00b7_dab0),
            (0x8c, 0x00b7_c320),
        ],
    );
    set(e, g.shader, vt);
    set(e, g.geometry + 0xc0, g.shader);
    e.mem.set_u8(g.rendering + 6, 2);
    e.mem.set_u8(g.rendering + 7, c.word(2) as u8);
    g.rendering
}

/// `render_standard` with the shader's per-geometry slots holding the shadow light shader's typed
/// set-ups (`00b7d200`, `00b7dab0`) and state restore (`00b7c320`): those calls made direct.
#[test]
fn render_standard_typed_setups() {
    Diff::new(entry!(
        0x00b9_8e80,
        batch::render_standard(Ptr<_>, u32, u32, u32)
    ))
    .set_ups()
    .inputs(&[
        Input::Of(&[0x10, 0x11, 6, 0x231, 0x1f7, 0x250]),
        Input::Of(&[0]),
        Input::Of(TWO_WAYS),
        Input::Of(BYTES),
    ])
    .translation_calls_plainly(&[0x00b7_c750])
    .setup(|e, c| {
        let pass = typed_setups_pass(e, c);
        vec![pass, c.word(3), 1, 0]
    })
    .run()
    .assert_same();
}

/// `render_skinned` with the same slots: the first drawn partition's set-ups and the geometry end made
/// direct.
#[test]
fn render_skinned_typed_setups() {
    Diff::new(entry!(
        0x00b9_9110,
        batch::render_skinned(Ptr<_>, u32, u32, u32)
    ))
    .set_ups()
    .inputs(&[
        Input::Of(&[0x10, 0x11, 6, 0x231]),
        Input::Of(&[1]),
        Input::Of(TWO_WAYS),
        Input::Of(BYTES),
    ])
    .translation_calls_plainly(&[0x00b7_c750])
    .setup(|e, c| {
        let pass = typed_setups_pass(e, c);
        vec![pass, c.word(3), 1, 0]
    })
    .run()
    .assert_same();
}

// ---- The draw family (`engine_typed::draw`): the world draw's drivers and alpha pass. ----
//
// The same world, with what the draw adds: the accumulator's per-renderer arrays, its camera (the world's
// camera object), a second batch renderer, alpha holders and sorted alpha groups, and stand-ins for what
// the draw calls outside the two families (the scene renderer's camera hand-over, the render mode
// preparation and table, the fog ambient set-up, the clip planes, the render pass tree, the reflection
// copy's renderer calls, the decal pass, a holder's own list draw, a sorted group's
// destructor and the list's `RemoveAll`).

use engine_typed::draw::{self, alpha as dalpha, drivers, own, packet};

/// Callees of the draw outside the families, stood in.
fn draw_stand_ins(e: &mut Engine) {
    for addr in [
        0x00b4_f450u32,
        0x00b9_8280,
        0x00b9_8320,
        0x0066_b0d0,
        0x00b6_b790,
        0x00b9_7550,
        0x00b6_b8d0,
        0x00b6_da10,
        0x00b9_a410,
    ] {
        e.register(addr, |_, _| ret(0));
    }
    // A batch renderer's slot 0x90 (its pass types drawn).
    e.register(VT_FN + 0x90, slot_answer::<0x90>);
    // The render object's camera data slot, and its check for the "SetCameraData" extra: present.
    e.register(VT_FN + 0x18c, slot_answer::<0x18c>);
    e.register(0x004a_0e10, |e, _| ret(e.mem.u32(RETS + 0x7f0)));
    set(e, RETS + 0x7f0, 1);
    // The fog-coloured ambient's targets and its enable object.
    e.map(0x0120_0000, 0x2000);
    let enable = e.mem.alloc(0x10);
    set(e, 0x0120_10c4, enable);
    e.register(0x00b6_3420, |_, _| ret(0x0bad_7ee0));
    e.register(0x00b6_e110, |_, _| ret(0x00c0_c0a0));
    e.register(0x00b6_b260, |_, a| ret(a[0] ^ 0x5a5a));
    // `RemoveAll`: the list emptied.
    e.register(0x00b7_1300, |e, a| {
        for k in 0..3 {
            e.mem.set_u32(a[0] + 4 * k, 0);
        }
        ret(0)
    });
    // The render mode table: mode 3's own draw; mode 5's entry empty (entry 0 then runs).
    e.register(0x00ff_2900, |_, _| ret(0));
    e.register(0x00ff_2904, |_, _| ret(0));
    set(e, 0x011f_9f40, 0x00ff_2900);
    set(e, 0x011f_9f40 + 12, 0x00ff_2904);
}

/// What the draw adds to the world: a second renderer and the alpha holders of both.
#[derive(Clone, Copy)]
struct DrawW {
    w: W,
    second: u32,
    /// The two alpha holders of each renderer (group 0, group 1).
    alpha_holders: [[u32; 2]; 2],
}

/// A sorted alpha group: flags 0, its own renderer `renderer`, distance `distance`.
fn sorted_group(e: &mut Engine, renderer: u32, distance: u32) -> u32 {
    let g = e.mem.alloc(0x30);
    set(e, g + 8, renderer);
    set(e, g + 0x20, distance);
    g
}

/// An `NiTPointerList` at `at` (head, tail, count) of `items`.
fn pointer_list(e: &mut Engine, at: u32, items: &[u32]) {
    let mut prev = 0;
    set(e, at, 0);
    for item in items {
        let node = e.mem.alloc(0xc);
        set(e, node + 4, prev);
        set(e, node + 8, *item);
        if prev != 0 {
            set(e, prev, node);
        } else {
            set(e, at, node);
        }
        prev = node;
    }
    set(e, at + 4, prev);
    set(e, at + 8, items.len() as u32);
}

fn draw_world(e: &mut Engine) -> DrawW {
    // The renderer's partition draw is the draw family's: the real one runs (a drawn buffer of 1 reads
    // no strips from the zero page).
    let w = world_all(e, false, false, false);
    e.map(0, 0x1000);
    draw_stand_ins(e);
    for off in [0x17c, 0x180, 0x184, 0x188] {
        let a = e.mem.alloc(0x20);
        set(e, w.acc + off, a);
    }
    set(e, w.acc + 8, w.camera);
    // A second batch renderer with the first's vtable, groups and map, and its own active list.
    let second = e.mem.alloc(0x98);
    for k in 0..0x98 / 4 {
        let v = e.mem.u32(w.br + 4 * k);
        set(e, second + 4 * k, v);
    }
    e.mem.set_u16(second + 0x2c, 0x11);
    set(e, second + 0x30, 0);
    // Its own pass groups, filled as the first's (two renderers never share their lists).
    let groups = e.mem.alloc(4 * 0x68);
    set(e, second + 8, groups);
    list_at(e, groups + 0x68, &[w.passes[0], w.passes[1]]);
    list_at(e, groups + 0x68 + 2 * 0x14, &[w.passes[2]]);
    set(e, groups + 0x68 + 0x64, 3);
    list_at(e, groups, &[w.skin_pass]);
    set(e, groups + 0x64, 1);
    // -FLT_MAX as the float and the double the alpha draw compares with, and the 0.001 its next
    // alphas take off their limit.
    e.map(0x0102_4000, 0x1000);
    e.mem.set_u64(0x0102_41b0, f64::from(-f32::MAX).to_bits());
    set(e, 0x0101_5f5c, (-f32::MAX).to_bits());
    e.mem.set_u64(0x0101_6978, 0.001f64.to_bits());
    set(e, w.acc + 0x178, second);
    // Its own register group holders, whose renderer is itself.
    for i in 0..13u32 {
        let h = e.mem.alloc(0x20);
        e.mem.set_u8(h, (i & 1) as u8);
        e.mem.set_u16(h + 2, 2);
        set(e, h + 4, 4);
        set(e, h + 8, second);
        list_at(e, h + 0xc, &[w.passes[(i % 4) as usize]]);
        set(e, second + 0x3c + 4 * i, h);
    }
    let mut alpha_holders = [[0u32; 2]; 2];
    for (r, br) in [w.br, second].into_iter().enumerate() {
        for g in 0..2u32 {
            let h = e.mem.alloc(0x20);
            e.mem.set_u16(h + 2, 1);
            set(e, h + 4, 4);
            set(e, h + 8, br);
            list_at(e, h + 0xc, &[w.passes[(g as usize + r) % 2]]);
            set(e, br + 0x70 + 4 * g, h);
            alpha_holders[r][g as usize] = h;
            pointer_list(e, br + 0x78 + 0xc * g, &[]);
            set(e, br + 0x90 + 4 * g, h);
        }
    }
    DrawW {
        w,
        second,
        alpha_holders,
    }
}

impl DrawW {
    fn renderers(&self) -> [u32; 2] {
        [self.w.br, self.second]
    }
}

/// The calls the translations make plainly on the draw's paths: the batch renderer's (as
/// `render_group`'s test lists them), the alpha sort's, a holder's clear, and the accumulator's and
/// batch renderer's own that the draw family types.
const DRAW_PLAIN: &[u32] = &[
    0x00b9_84d0,
    0x00b9_8ae0,
    0x00b9_8e80,
    0x00b9_9110,
    0x00b9_9390,
    0x00b9_94f0,
    0x00b9_9730,
    0x00b9_9900,
    0x00b9_9630,
    0x00b9_9bc0,
    0x00b6_3770,
    0x00b6_3a50,
    0x00b6_5580,
    0x00b6_3120,
    0x00b6_50c0,
    0x00b6_4f20,
    0x00b9_9a50,
    0x00b9_9990,
    0x004e_9c50,
    0x004e_9c10,
    0x004e_9c90,
];

trait DrawCommon {
    fn draw_common_at(self, entry: u32) -> Diff;
}

impl DrawCommon for Diff {
    /// The draw family's settings for a test of `entry`: the plain calls but the entry itself.
    fn draw_common_at(self, entry: u32) -> Diff {
        let plain: Vec<u32> = DRAW_PLAIN.iter().copied().filter(|a| *a != entry).collect();
        self.common()
            .family(&draw::DRAW)
            .translation_calls_plainly(&plain)
            .float_sweep(0)
    }
}

/// The accumulator's vtable, with its own slots 0x8c, 0xa8 and 0xac when `real`.
fn acc_slots(e: &mut Engine, acc: u32, real: bool) {
    let vt = if real {
        vtable(
            e,
            &[
                (0x8c, 0x00d4_7a40),
                (0xa8, 0x00b6_6520),
                (0xac, 0x00b6_6570),
            ],
        )
    } else {
        vtable(e, &[])
    };
    set(e, acc, vt);
}

/// The accumulator held at `011f95f0`: none, this one, or another with a count of 1 or 2.
fn held(e: &mut Engine, acc: u32, which: u32) {
    let vt = vtable(e, &[]);
    let other = obj(e, 0x10, vt);
    set(e, other + 4, 1 + (which & 1));
    let value = match which {
        0 => 0,
        1 => acc,
        _ => other,
    };
    set(e, 0x011f_95f0, value);
    set(e, acc + 4, 3);
}

#[test]
fn world_draw_first() {
    Diff::new(entry!(
        0x00b6_ba20,
        drivers::world_draw_first(u32, Ptr<_>, u32)
    ))
    .draw_common_at(0x00b6_ba20)
    .inputs(&[
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1, 2, 3]),
    ])
    .setup(|e, c| {
        let d = draw_world(e);
        if c.word(0) == 0 {
            set(e, 0x011f_4748, 0);
        }
        acc_slots(e, d.w.acc, c.word(2) != 0);
        // The real slot 0xa8 draws render mode 3, whose table entry stands in.
        set(e, d.w.acc + 0x19c, 3);
        held(e, d.w.acc, c.word(3));
        let camera = if c.word(1) != 0 { d.w.camera } else { 0 };
        vec![camera, d.w.acc, 1]
    })
    .run()
    .assert_same();
}

#[test]
fn world_draw_second() {
    Diff::new(entry!(
        0x00b6_b930,
        drivers::world_draw_second(u32, Ptr<_>, u32)
    ))
    .draw_common_at(0x00b6_b930)
    .inputs(&[Input::Of(TWO_WAYS)])
    .setup(|e, c| {
        let d = draw_world(e);
        acc_slots(e, d.w.acc, c.word(0) != 0);
        set(e, d.w.acc + 0x19c, 3);
        vec![d.w.camera, d.w.acc, 1]
    })
    .run()
    .assert_same();
}

#[test]
fn scene_draw() {
    Diff::new(entry!(0x00b6_c0d0, drivers::scene_draw(u32, Ptr<_>) -> u32))
        .draw_common_at(0x00b6_c0d0)
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[3, 5]),
        ])
        .setup(|e, c| {
            let d = draw_world(e);
            // No scene renderer only with stand-in slots: the real finish clears the renderer's word +0xc.
            if c.word(0) == 0 && c.word(2) == 0 {
                set(e, 0x011f_4748, 0);
            }
            acc_slots(e, d.w.acc, c.word(2) != 0);
            set(e, d.w.acc + 0x19c, c.word(4));
            held(e, d.w.acc, c.word(3));
            let camera = if c.word(1) != 0 { d.w.camera } else { 0 };
            vec![camera, d.w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn accumulator_draw() {
    Diff::new(entry!(0x00b6_6520, drivers::accumulator_draw(Ptr<_>)))
        .draw_common_at(0x00b6_6520)
        .inputs(&[Input::Of(&[0, 3, 5])])
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, d.w.acc + 0x19c, c.word(0));
            // Mode 0 draws nothing without a camera (the first half's own test draws).
            set(e, d.w.acc + 8, 0);
            vec![d.w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn accumulator_finish() {
    Diff::new(entry!(0x00b6_6570, drivers::accumulator_finish(Ptr<_>)))
        .draw_common_at(0x00b6_6570)
        .inputs(&[Input::Of(&[0, 3]), Input::Of(TWO_WAYS)])
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, d.w.acc + 0x19c, c.word(0));
            e.mem.set_u8(d.w.acc + 0x167, c.word(1) as u8);
            vec![d.w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn set_camera_and_finish_accumulating() {
    Diff::new(entry!(0x00d4_7a40, packet::set_camera(Ptr<_>, u32)))
        .draw_common_at(0x00d4_7a40)
        .inputs(&[Input::Word])
        .setup(|e, c| {
            let d = draw_world(e);
            vec![d.w.acc, c.word(0)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x00aa_0fe0, packet::finish_accumulating(Ptr<_>)))
        .draw_common_at(0x00aa_0fe0)
        .inputs(&[Input::Of(TWO_WAYS)])
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, d.w.acc + 8, c.word(0) * d.w.camera);
            vec![d.w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn hold_accumulator() {
    Diff::new(entry!(0x00b5_4b10, own::hold_accumulator(u32)))
        .draw_common_at(0x00b5_4b10)
        .inputs(&[Input::Of(&[0, 1, 2, 3]), Input::Of(&[0, 1, 2])])
        .setup(|e, c| {
            let d = draw_world(e);
            held(e, d.w.acc, c.word(0));
            let object = match c.word(1) {
                0 => 0,
                1 => d.w.acc,
                _ => {
                    let vt = vtable(e, &[]);
                    obj(e, 0x10, vt)
                }
            };
            vec![object]
        })
        .run()
        .assert_same();
}

/// The active list of each renderer from the case: as made, or with a shadow pass type (0x1c5) or a
/// type of the second half (0x291, one the "has a pass of these types" check lists) in front.
fn active_lists(e: &mut Engine, d: &DrawW, which: u32) {
    if which == 0 {
        return;
    }
    let kind = if which == 1 { 0x1c5u16 } else { 0x291 };
    for br in d.renderers() {
        let next = e.mem.alloc(8);
        let old = e.mem.u16(br + 0x2c);
        let link = e.mem.u32(br + 0x30);
        e.mem.set_u16(next, old);
        set(e, next + 4, link);
        e.mem.set_u16(br + 0x2c, kind);
        set(e, br + 0x30, next);
    }
}

#[test]
fn draw_types() {
    Diff::new(entry!(
        0x00b6_3770,
        drivers::draw_types(Ptr<_>, u32, u32, u32, i32)
    ))
    .draw_common_at(0x00b6_3770)
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[u32::MAX, 0, 1, 3]),
        Input::Of(&[1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 0x100]),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
    ])
    .max_cases(120)
    .setup(|e, c| {
        let d = draw_world(e);
        let (low, high) = [(1, 0x2f1), (0x10, 0x10), (0x1c4, 0x290)][c.word(0) as usize];
        set(e, d.w.acc + 0x18c, c.word(2));
        active_lists(e, &d, c.word(3));
        e.mem.set_u8(d.w.acc + 0x164, c.word(4) as u8);
        e.mem.set_u8(d.w.acc + 0x165, (c.word(4) >> 8) as u8);
        e.mem.set_u8(0x011f_94a6, c.word(5) as u8);
        vec![d.w.acc, low, high, c.word(6), c.word(1)]
    })
    .run()
    .assert_same();
}

#[test]
fn draw_group() {
    Diff::new(entry!(0x00b6_39e0, drivers::draw_group(Ptr<_>, u32, u32)))
        .draw_common_at(0x00b6_39e0)
        .inputs(&[
            Input::Of(&[0, 1, 2, 9, 12]),
            Input::Of(&[0, 1]),
            Input::Of(&[1, 2]),
            Input::Of(TWO_WAYS),
        ])
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, d.w.acc + 0x18c, c.word(2));
            if c.word(3) != 0 {
                set(e, d.w.br + 0x3c + 4 * c.word(0), 0);
            }
            vec![d.w.acc, c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

#[test]
fn sort_alphas_of_renderers() {
    Diff::new(entry!(0x00b6_3a50, drivers::sort_alphas(Ptr<_>)))
        .draw_common_at(0x00b6_3a50)
        .inputs(&[Input::Of(&[0, 1, 2]), Input::Of(BYTES)])
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, d.w.acc + 0x18c, c.word(0));
            e.mem.set_u8(d.w.acc + 0x31, c.word(1) as u8);
            vec![d.w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn has_listed_pass() {
    Diff::new(entry!(
        0x00b6_3120,
        drivers::has_listed_pass(Ptr<_>, u32, u32) -> bool
    ))
    .draw_common_at(0x00b6_3120)
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1]),
    ])
    .setup(|e, c| {
        let d = draw_world(e);
        set(e, d.w.acc + 0x18c, c.word(0));
        active_lists(e, &d, c.word(1));
        let (low, high) = [(0x288, 0x2ce), (0x10, 0x11)][c.word(2) as usize];
        vec![d.w.acc, low, high]
    })
    .run()
    .assert_same();
}

#[test]
fn clear_active_lists() {
    Diff::new(entry!(0x00b6_5580, drivers::clear_active_lists(Ptr<_>)))
        .draw_common_at(0x00b6_5580)
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, d.w.acc + 0x18c, c.word(0));
            vec![d.w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn first_half() {
    Diff::new(entry!(0x00b6_5ae0, drivers::first_half(Ptr<_>)))
        .draw_common_at(0x00b6_5ae0)
        .inputs(&[
            Input::Of(&[0, 7, 8]),
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(&[1, 2]),
            Input::Of(&[0, 1, 2]),
        ])
        .max_cases(64)
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, d.w.acc + 0x19c, c.word(0));
            if c.word(1) == 0 {
                set(e, d.w.acc + 8, 0);
            }
            e.mem.set_u8(d.w.acc + 0x164, c.word(2) as u8);
            e.mem.set_u8(0x011f_9fc1, c.word(3) as u8);
            e.mem.set_u8(d.w.acc + 0x167, c.word(4) as u8);
            set(e, d.w.acc + 0x18c, c.word(5));
            active_lists(e, &d, c.word(6));
            set(e, 0x011a_9608, 0x77);
            vec![d.w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn second_half() {
    Diff::new(entry!(0x00b6_5c60, drivers::second_half(Ptr<_>)))
        .draw_common_at(0x00b6_5c60)
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(&[0, 1, 2]),
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(&[1, 2]),
        ])
        .max_cases(64)
        .setup(|e, c| {
            let d = draw_world(e);
            e.mem.set_u8(d.w.acc + 0x38, c.word(0) as u8);
            active_lists(e, &d, c.word(1));
            set(e, 0x011f_9438, c.word(2) * 0x4242);
            set(e, 0x011f_943c, c.word(3) * 0x4343);
            set(e, d.w.acc + 0x18c, c.word(4));
            vec![d.w.acc]
        })
        .run()
        .assert_same();
}

/// The alpha state from the case: each renderer's holder count for `group` (bit r of `counts`), and
/// its list of sorted groups (bits 2r and 2r+1 of `listed`: none, one at `distance`, or that and a
/// nearer one).
fn alpha_state(e: &mut Engine, d: &DrawW, group: u32, counts: u32, listed: u32, distance: u32) {
    for (r, br) in d.renderers().into_iter().enumerate() {
        let holder = d.alpha_holders[r][group as usize];
        e.mem.set_u16(holder + 2, ((counts >> r) & 1) as u16);
        if (counts >> r) & 1 == 0 {
            // No passes: the list empty too, as the holder's count says.
            set(e, holder + 0x10, 0);
            set(e, holder + 0x14, 0);
            set(e, holder + 0x1c, 0);
        }
        let items: Vec<u32> = match (listed >> (2 * r)) & 3 {
            0 => vec![],
            1 => vec![sorted_group(e, br, distance)],
            _ => {
                let far = sorted_group(e, br, distance);
                // Just inside a limit of 0.5 by less than the 0.001 RenderNextAlpha takes off.
                let near = sorted_group(e, br, 0x3eff_be77);
                vec![far, near]
            }
        };
        pointer_list(e, br + 0x78 + 0xc * group, &items);
    }
}

#[test]
fn draw_alpha_group() {
    Diff::new(entry!(0x00b6_50c0, dalpha::draw_alpha_group(Ptr<_>, u32)))
        .draw_common_at(0x00b6_50c0)
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(&[1, 2]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 1, 6, 9]),
            Input::Of(BYTES),
            // No NaN with two renderers: a sorted group at NaN is never within a limit, and with two the
            // exe loops on it for ever (PC builds one: `draw_alpha_group_nan` has NaN with one).
            Input::Of(&[ONE, HALF, TWO, 0xc2c8_0000, 0, 0x7f7f_ffff, 0xff80_0000]),
        ])
        .max_cases(160)
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, d.w.acc + 0x18c, c.word(1));
            e.mem.set_u8(d.w.acc + 0x31, c.word(4) as u8);
            alpha_state(e, &d, c.word(0), c.word(2), c.word(3), c.word(5));
            vec![d.w.acc, c.word(0)]
        })
        .run()
        .assert_same();
}

/// `draw_alpha_group` with one batch renderer, as PC builds the accumulator, and a sorted group at a NaN
/// distance (quiet, the x87 default, signalling): the alpha loop ends there; the group is not drawn,
/// then freed.
#[test]
fn draw_alpha_group_nan() {
    Diff::new(entry!(0x00b6_50c0, dalpha::draw_alpha_group(Ptr<_>, u32)))
        .draw_common_at(0x00b6_50c0)
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(&[0, 1]),
            Input::Of(&[1, 2]),
            Input::Of(BYTES),
            Input::Of(&[0x7fc0_0000, 0xffc0_0000, 0x7f80_0001]),
        ])
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, d.w.acc + 0x18c, 1);
            e.mem.set_u8(d.w.acc + 0x31, c.word(3) as u8);
            alpha_state(e, &d, c.word(0), c.word(1), c.word(2), c.word(4));
            vec![d.w.acc, c.word(0)]
        })
        .run()
        .assert_same();
}

#[test]
fn alpha_distance() {
    Diff::new(entry!(
        0x00b9_8c80,
        dalpha::alpha_distance(Ptr<_>, u32, u32, u32) -> f64
    ))
    .draw_common_at(0x00b9_8c80)
    .inputs(&[
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 0x100, 0x101]),
        Input::F32Swept(ONE),
        Input::F32Swept(TWO),
        Input::F32Swept(HALF),
    ])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let d = draw_world(e);
        let group = c.word(0);
        alpha_state(e, &d, group, c.word(1), c.word(2), bits(c, 4));
        // The first pass's bound and the camera direction from the swept floats.
        // (Every alpha pass of the world is of the one geometry.)
        let bound = e.mem.u32(d.w.geometry + 0x20);
        set(e, bound, bits(c, 5));
        set(e, bound + 0xc, bits(c, 6));
        set(e, d.w.camera + 0x74, bits(c, 6));
        vec![d.w.br, group, d.w.camera, c.word(3)]
    })
    .run()
    .assert_same();
}

#[test]
fn clear_alpha_group() {
    Diff::new(entry!(0x00b6_4f20, dalpha::clear_alpha_group(Ptr<_>, u32)))
        .draw_common_at(0x00b6_4f20)
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(&[0, 1, 2]),
            Input::Of(TWO_WAYS),
        ])
        .setup(|e, c| {
            let d = draw_world(e);
            let group = c.word(0);
            alpha_state(e, &d, group, 1, c.word(1), ONE);
            if c.word(2) != 0 && c.word(1) != 0 {
                // A null item in the first node.
                let node = e.mem.u32(d.w.br + 0x78 + 0xc * group);
                set(e, node + 8, 0);
            }
            vec![d.w.br, group]
        })
        .run()
        .assert_same();
}

#[test]
fn draw_and_clear_type() {
    Diff::new(entry!(
        0x00b9_a3c0,
        dalpha::draw_and_clear_type(Ptr<_>, u32, u32, u32, u32) -> bool
    ))
    .draw_common_at(0x00b9_a3c0)
    .inputs(&[
        Input::Of(&[0, 0x10, 0x11, 0x13, 0x1c5]),
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::Of(&[0, 0x11]),
    ])
    .setup(|e, c| {
        let d = draw_world(e);
        for group in 0..2 {
            for list in 0..5 {
                let at = group_list(e, &d.w, group, list);
                list_at(e, at, &[d.w.passes[0], d.w.passes[1]]);
            }
        }
        e.mem.set_u16(d.w.br + 0x2c, c.word(2) as u16);
        let out = e.mem.alloc(0xc);
        set(e, out, c.word(0));
        set(e, out + 4, c.word(1));
        vec![d.w.br, out, out + 4, out + 8, 0]
    })
    .run()
    .assert_same();
}

#[test]
fn draw_holder() {
    Diff::new(entry!(0x00b9_a4d0, dalpha::draw_holder(u32, u32)))
        .draw_common_at(0x00b9_a4d0)
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(&[0, 1, 0x33]),
        ])
        .setup(|e, c| {
            let d = draw_world(e);
            let holder = d.w.holders[c.word(0) as usize];
            if c.word(1) == 0 {
                set(e, holder + 8, 0);
            }
            vec![holder, c.word(2)]
        })
        .run()
        .assert_same();
}

#[test]
fn render_next_alpha() {
    Diff::new(entry!(
        0x00b9_af60,
        dalpha::render_next_alpha(Ptr<_>, u32, u32, u32, f32)
    ))
    .draw_common_at(0x00b9_af60)
    .inputs(&[
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 0x101]),
        // The limit: the exe's floats and 0.5 (a sorted group sits just inside it).
        Input::Of(&[
            0,
            HALF,
            ONE,
            TWO,
            0xbf80_0000,
            0xff80_0000,
            0x7fc0_0000,
            0x3eaa_aaab,
        ]),
        Input::F32Swept(HALF),
        Input::Of(&[0, 1, 2, 3]),
        // The first sorted group: keeps its own passes, or has no renderer (+8 = 0).
        Input::Of(TWO_WAYS),
    ])
    .max_cases(240)
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let d = draw_world(e);
        let group = c.word(0);
        alpha_state(e, &d, group, c.word(1), c.word(2), bits(c, 5));
        // The pass's property: none, plain, decal-biased, or of kind 0x15 in shader mode 9.
        // (Every alpha pass of the world is of the one geometry.)
        let geometry = d.w.geometry;
        let property = e.mem.u32(geometry + 0xa8);
        match c.word(6) {
            0 => set(e, geometry + 0xa8, 0),
            2 => set(e, property + 0x20, 0x0400_0000),
            3 => {
                set(e, 0x011f_91b8, 9);
                set(e, property + 0x1c, 0x15);
            }
            _ => {}
        }
        // A sorted group that keeps its own passes, the other its renderer's; or the first with no
        // renderer at all.
        let list = d.w.br + 0x78 + 0xc * group;
        let head = e.mem.u32(list);
        if head != 0 {
            let g = e.mem.u32(head + 8);
            if c.word(7) != 0 {
                set(e, g + 8, 0);
            } else {
                e.mem.set_u8(g, 1);
            }
        }
        vec![d.w.br, group, d.w.camera, c.word(3), c.word(4)]
    })
    .run()
    .assert_same();
}

#[test]
fn decal_pass() {
    Diff::new(entry!(0x00b6_4e30, drivers::decal_pass(Ptr<_>)))
        .draw_common_at(0x00b6_4e30)
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(&[0, 8]),
            Input::Of(TWO_WAYS),
            Input::F32,
        ])
        .setup(|e, c| {
            let d = draw_world(e);
            e.mem.set_u8(d.w.acc + 0x16c, c.word(0) as u8);
            set(e, d.w.acc + 0x19c, c.word(1));
            e.mem.set_u8(0x011a_d80f, c.word(2) as u8);
            set(e, 0x0120_0458, bits(c, 3));
            vec![d.w.acc]
        })
        .run()
        .assert_same();
}

#[test]
fn fog_ambient() {
    Diff::new(entry!(0x00bb_b840, own::fog_ambient()))
        .draw_common_at(0x00bb_b840)
        .inputs(&[Input::Word])
        .setup(|e, c| {
            let _ = draw_world(e);
            let enable = e.mem.alloc(0x10);
            set(e, 0x0120_10c4, enable);
            set(e, 0x011a_9bd0, c.word(0));
            vec![]
        })
        .run()
        .assert_same();
}

/// A camera whose world rotation and translate are swept floats.
fn swept_camera(e: &mut Engine, c: &Case, first: usize) -> u32 {
    let camera = e.mem.alloc(0x120);
    for k in 0..12u32 {
        let w = if k % 4 == 0 {
            bits(c, first)
        } else {
            bits(c, first + 1) ^ k
        };
        set(e, camera + 0x68 + 4 * k, w);
    }
    camera
}

#[test]
fn hand_camera() {
    Diff::new(entry!(0x004e_9bb0, packet::hand_camera(u32, u32)))
        .draw_common_at(0x004e_9bb0)
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::F32Swept(ONE),
            Input::F32Swept(HALF),
        ])
        .float_sweep(16)
        .setup(|e, c| {
            let d = draw_world(e);
            set(e, RETS + 0x7f0, c.word(0));
            let camera = swept_camera(e, c, 1);
            vec![d.w.renderer, camera]
        })
        .run()
        .assert_same();
}

#[test]
fn camera_getters() {
    Diff::new(entry!(0x004a_0d10, packet::camera_frustum(u32) -> u32))
        .draw_common_at(0x004a_0d10)
        .inputs(&[Input::Word])
        .setup(|_, c| vec![c.word(0)])
        .run()
        .assert_same();
    Diff::new(entry!(0x0045_bbe0, packet::camera_port(u32) -> u32))
        .draw_common_at(0x0045_bbe0)
        .inputs(&[Input::Word])
        .setup(|_, c| vec![c.word(0)])
        .run()
        .assert_same();
    Diff::new(entry!(0x0045_bb80, packet::world_translate(u32) -> u32))
        .draw_common_at(0x0045_bb80)
        .inputs(&[Input::Word])
        .setup(|_, c| vec![c.word(0)])
        .run()
        .assert_same();
}

#[test]
fn rotation_columns() {
    Diff::new(entry!(0x0045_bba0, packet::rotation_column_0(u32, u32) -> u32))
        .draw_common_at(0x0045_bba0)
        .inputs(&[Input::F32Swept(ONE), Input::F32Swept(HALF)])
        .float_sweep(16)
        .setup(|e, c| {
            let camera = swept_camera(e, c, 0);
            let out = e.mem.alloc(0xc);
            vec![camera, out]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x004e_9c10, packet::rotation_column_1(u32, u32) -> u32))
        .draw_common_at(0x004e_9c10)
        .inputs(&[Input::F32Swept(ONE), Input::F32Swept(HALF)])
        .float_sweep(16)
        .setup(|e, c| {
            let camera = swept_camera(e, c, 0);
            let out = e.mem.alloc(0xc);
            vec![camera, out]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x004e_9c50, packet::rotation_column_2(u32, u32) -> u32))
        .draw_common_at(0x004e_9c50)
        .inputs(&[Input::F32Swept(ONE), Input::F32Swept(HALF)])
        .float_sweep(16)
        .setup(|e, c| {
            let camera = swept_camera(e, c, 0);
            let out = e.mem.alloc(0xc);
            vec![camera, out]
        })
        .run()
        .assert_same();
}

#[test]
fn pass_camera_data() {
    Diff::new(entry!(
        0x004e_9c90,
        packet::pass_camera_data(u32, u32, u32, u32, u32, u32, u32)
    ))
    .draw_common_at(0x004e_9c90)
    .inputs(&[Input::Of(TWO_WAYS), Input::Word])
    .setup(|e, c| {
        let d = draw_world(e);
        set(e, RETS + 0x7f0, c.word(0));
        let w = c.word(1);
        vec![d.w.renderer, w, w ^ 1, w ^ 2, w ^ 3, w ^ 4, w ^ 5]
    })
    .run()
    .assert_same();
}

// ---- The batch renderer's direct calls into the draw family's geometry paths (group D). ----

/// `render_standard` with the geometry's slot 0xe0 (its immediate draw with the renderer) holding the
/// triangle shape's (`00a74600`), the segmented shape's (`00a87130`), the particles' (`00a9b1f0`) typed
/// draw or the triangle strips' (`00a71d70`, the dispatch family's): the call made direct; the renderer
/// is not drawing immediately, so each answers after `NiGeometry::RenderImmediate`.
#[test]
fn render_standard_typed_immediate() {
    Diff::new(entry!(
        0x00b9_8e80,
        batch::render_standard(Ptr<_>, u32, u32, u32)
    ))
    .draw_common_at(0x00b9_8e80)
    .goes_by_address(&[0x00a8_0610, 0x00a7_1d70])
    .inputs(&[Input::Of(&[0, 1, 2, 3]), Input::Of(TWO_WAYS)])
    .setup(|e, c| {
        let d = draw_world(e);
        let w = d.w;
        let target = [0x00a7_4600, 0x00a8_7130, 0x00a9_b1f0, 0x00a7_1d70][c.word(0) as usize];
        // A geometry of the segmented shape's size, the world's own fields copied, its segments
        // unchanged.
        let vt = vtable(e, &[(0xe0, target)]);
        let geometry = obj(e, 0xd0, vt);
        for off in (4..0xc4).step_by(4) {
            let word = e.mem.u32(w.geometry + off);
            set(e, geometry + off, word);
        }
        // Two changed segments, so the segmented shape's merge shows.
        let segments = e.mem.alloc(0x28);
        set(e, segments + 4, 3);
        e.mem.set_u8(segments + 8, 1);
        set(e, segments + 0x18, 2);
        e.mem.set_u8(segments + 0x1c, 1);
        set(e, geometry + 0xc4, segments);
        set(e, geometry + 0xc8, 2);
        e.mem.set_u8(geometry + 0xcc, 1);
        let pass = w.passes[0];
        set(e, pass, geometry);
        vec![pass, c.word(1), 1, 0]
    })
    .run()
    .assert_same();
}

/// `render_skinned` with the shader's slot 0x64 holding the skinned shader classes' bone set-up
/// (`00b7a2c0`), which finds nothing to upload for the shader (no switch, +0x34 not its own bones):
/// the call made direct for each drawn partition.
#[test]
fn render_skinned_typed_bone_setup() {
    Diff::new(entry!(
        0x00b9_9110,
        batch::render_skinned(Ptr<_>, u32, u32, u32)
    ))
    .draw_common_at(0x00b9_9110)
    .inputs(&[Input::Of(TWO_WAYS), Input::Of(TWO_WAYS)])
    .setup(|e, c| {
        let d = draw_world(e);
        let w = d.w;
        let skinned = e.mem.u32(w.skin_pass);
        let vt = vtable(e, &[(0x64, 0x00b7_a2c0)]);
        let shader = obj(e, 0x90, vt);
        for off in (4..0x80).step_by(4) {
            let word = e.mem.u32(w.shader + off);
            set(e, shader + off, word);
        }
        set(e, shader + 0x34, 0x0b0e_0034);
        set(e, shader + 0x80, 0x0b0e_0080);
        set(e, 0x011f_ec00, 0);
        set(e, skinned + 0xc0, shader);
        vec![w.skin_pass, c.word(0), c.word(1), 0]
    })
    .run()
    .assert_same();
}
