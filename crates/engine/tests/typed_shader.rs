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
use engine_core::typed::{MAIN_THREAD, OTHER_THREADS};
use engine_core::{entry, Engine};
use engine_typed::draw::group_c;
use engine_typed::shader::{
    self, accumulator as acc, batch, constants, geometry, lighting, state, textures,
};
use engine_typed::{crt, culling, pathing, render};

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

/// A float as play has it on the paths whose maths is native (ADR-0014's float rule): its magnitude
/// folded into 2^-12 to 2^12, its sign and mantissa kept; zeros and infinities as they come; a NaN the
/// quiet NaN 0x7fc00000. On the main thread that maths is plain `f32`, which rounds as the exe's 24-bit
/// x87 code does while every step stays inside the float range, where the x87 keeps a wider exponent:
/// from these values no step of the set-ups leaves it, and a product of five stays within 2^-60 to
/// 2^60, where the comparison reads words as floats. Play has no signalling NaNs or NaN payloads: the
/// exe quiets a signalling NaN it copies through the x87 stack where native code copies it as it is.
fn play(bits: u32) -> u32 {
    let exponent = (bits >> 23) & 0xff;
    if exponent == 0xff && bits & 0x007f_ffff != 0 {
        return 0x7fc0_0000;
    }
    if exponent == 0xff || bits & 0x7fff_ffff == 0 || (115..139).contains(&exponent) {
        return bits;
    }
    (bits & 0x807f_ffff) | ((115 + exponent % 24) << 23)
}

/// The case's float `i` as play has it ([`play`]).
fn play_bits(case: &Case, i: usize) -> u32 {
    play(bits(case, i))
}

/// How the native maths is compared with its translation (ADR-0014's float rule).
trait Native {
    /// Code only the main thread runs (plain `f32`): under the main thread's control word, floats
    /// within `ulps`, or bit for bit (NaN payloads aside) with 0. The native maths rounds as the exe
    /// does there, but a step with a `double` operand rounds twice natively (to 53 bits, then to a
    /// float) where the x87 rounds once: one ulp where the first rounding lands exactly halfway.
    fn main_thread(self, ulps: u32) -> Diff;
    /// Code that also runs on the other game threads (`f64` rounded where the exe stores a float):
    /// under their control word, bit for bit (NaN payloads aside).
    fn other_threads(self) -> Diff;
}

impl Native for Diff {
    fn main_thread(self, ulps: u32) -> Diff {
        let d = self.control_words(&[MAIN_THREAD]);
        if ulps == 0 {
            d.allow_nan_payloads()
        } else {
            d.float_ulps(ulps)
        }
    }

    fn other_threads(self) -> Diff {
        self.control_words(&[OTHER_THREADS]).allow_nan_payloads()
    }
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

/// [`stand_ins_named`], with the draw family's members the batch renderer reaches (the renderer's
/// partition draw `00e6d310` and the shadow light shader's pass set-ups) stood in or not: a test that
/// names the draw family has the real ones ([`draw_world`]), with what they call by address stood in.
fn stand_ins_all(e: &mut Engine, render: bool, dispatch: bool, draw: bool) {
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
    for addr in [0x00b7_a870, 0x00b6_3320, 0x00ec_658f] {
        e.register(addr, |_, _| ret(0));
    }
    if draw {
        for addr in [
            // The pass end's flags (the draw family's: real where a test names it).
            0x00bb_96d0,
            0x00e6_d310,
            0x00b7_ded0,
            0x00b7_dfe0,
            0x00b7_e150,
            0x00b7_dd50,
            0x00b7_deb0,
            0x00b7_ac20,
        ] {
            e.register(addr, |_, _| ret(0));
        }
    } else {
        for addr in SETUP_CALLEES {
            e.register(*addr, marked);
        }
    }
    // The renderer's skinned test: the geometry has a skin.
    if !render {
        e.register(0x00e7_2c20, |e, a| {
            ret(u32::from(e.mem.u32(a[1] + 0xbc) != 0))
        });
    }
    e.register(0x00b5_5560, |e, _| ret(e.mem.u32(RETS + 0x1f0)));
    // The D3DX imports (behind the thunks, which are the draw family's): a mark into the out word.
    for addr in [0x00fd_f410, 0x00fd_f420, 0x00fd_f41c] {
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

/// [`world_named`] with the draw family's members stood in or not ([`stand_ins_all`]).
fn world_all(e: &mut Engine, render: bool, dispatch: bool, draw: bool) -> W {
    stand_ins_all(e, render, dispatch, draw);
    // The two constant switch objects the first pass begin sets (`011febf0`, `011fec38`).
    for at in [0x011f_ebf0u32, 0x011f_ec38] {
        let switch = e.mem.alloc(0x24);
        set(e, at, switch);
    }
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
    // A pass of the size the cut reads (0x10: its light count +9 and lights +0xc, none here).
    let pass_of = |e: &mut Engine, geometry: u32, pass_type: u16, kind: u8, flag: u8| {
        let p = e.mem.alloc(0x10);
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
    /// As [`Common::common`], but the shader family not named: the shader functions Group C's
    /// set-ups reach ([`SHADER_CALLED`]) run by address, their translations on both sides. The world
    /// transform is that family's code for any thread (`f64` maths, its own tests compare it on the main
    /// thread's word within ulps), while these set-ups' own maths is the exe's there, bit for bit.
    fn shader_translated(self) -> Diff;
}

/// The shader family's functions Group C's set-ups call: the world transform, the alpha function, the
/// blend, z writes and the z test, a stage's address modes, a constant map's `SetShaderConstants`.
const SHADER_CALLED: &[u32] = &[
    0x00e6_fbb0,
    0x00b9_7f20,
    0x00b9_7ff0,
    0x00b9_7e30,
    0x00b9_7de0,
    0x00e7_de90,
    0x00e8_26d0,
];

impl Common for Diff {
    fn common(self) -> Diff {
        shared(self.family(&shader::SHADER))
    }

    fn shader_translated(self) -> Diff {
        shared(self).goes_by_address(SHADER_CALLED)
    }
}

/// What [`Common`]'s settings share: the leaves, what runs by address, the limits.
fn shared(d: Diff) -> Diff {
    d.family(&leaf::LEAVES)
        // The plain geometry pass (pass types 0xfc, 0xfd) and the renderer's skinned test are the
        // render family's (their own tests).
        // The draw family's partition draw, skinned packer and strip lists, which the batch renderer's
        // skinned draw and the render family's packing call, and its pass set-ups, land toggles and
        // fog property, which the batch renderer and the toggles set-up call, are stood in here or
        // run by address and run in its tests.
        .goes_by_address(&[
            0x00b7_dde0,
            0x00e7_2c20,
            0x00e6_d310,
            0x00e6_d780,
            0x00a7_5f10,
            0x00b7_dd50,
            0x00b7_deb0,
            0x00b7_ded0,
            0x00b7_dfe0,
            0x00b7_e150,
            0x00b7_ac20,
            0x00c0_2c30,
            0x00b5_5520,
            // The render family's stage bind, which the pass set-ups call.
            0x00e7_eb00,
            // The draw family's D3DX thunks, which the set-ups call.
            0x00ee_6dfe,
            0x00ee_6dec,
            0x00ee_6df2,
            0x00ee_6df8,
            0x00ee_6dd4,
            0x00ee_6dda,
            0x00ee_6de0,
            0x00ee_6dce,
            0x00ee_6dc8,
        ])
        // draw_f's helpers: off until its gate, so by address, as in play; run in typed_draw.
        .goes_by_address(DRAW_F_HELPERS)
        .max_calls(MAX_CALLS)
        .max_cases(160)
}

/// draw_f's helpers (`engine_typed::draw_f`): the float maximum and minimum, the class test, the list
/// node pool's allocation.
const DRAW_F_HELPERS: &[u32] = &[0x0040_4010, 0x0040_ebd0, 0x0043_b300, 0x0043_a010];

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

/// The stencil enable (`00b98070`): its counter index is state 0x34's (4 in the world).
#[test]
fn stencil_enable() {
    Diff::new(entry!(0x00b9_8070, state::set_stencil_enable(u8, u32)))
        .common()
        .inputs(&[Input::Of(BYTES), Input::Of(LOCKS), Input::Of(TWO_WAYS)])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            counter(e, 4, c.word(2));
            vec![c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

/// The clip planes (`00b98280`): each plane byte 0 or not, the lock, the counter (state 0x98's index 0).
#[test]
fn clip_planes() {
    Diff::new(entry!(
        0x00b9_8280,
        state::set_clip_planes(u8, u8, u8, u8, u8, u8, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 0x80]),
        Input::Of(&[0, 2]),
        Input::Of(&[0, 0xff]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 3]),
        Input::Of(&[0, 1]),
        Input::Of(LOCKS),
        Input::Of(TWO_WAYS),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| {
        let _w = world(e);
        counter(e, 0, c.word(7));
        (0..7).map(|k| c.word(k)).collect()
    })
    .run()
    .assert_same();
}

/// The depth bias (`00b98320`): plain floats and NaNs (a signalling one comes out quiet), the lock, the
/// counter (its index word set to 5).
#[test]
fn depth_bias() {
    Diff::new(entry!(0x00b9_8320, state::set_depth_bias(u32, u32)))
        .common()
        .inputs(&[
            Input::Of(&[0, ONE, 0x7f80_0001, 0x7fc0_0000, 0xff80_0000]),
            Input::Of(LOCKS),
            Input::Of(TWO_WAYS),
        ])
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| {
            let _w = world(e);
            set(e, 0x011f_fd3c, 5);
            counter(e, 5, c.word(2));
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

/// The range runs on the main thread and another game thread: its difference is `f64` rounded at the
/// store. Under the other threads' word bit for bit (NaN payloads aside: the stores no longer quiet a
/// signalling NaN), every input; under the main thread's within one ulp (the difference of two floats
/// far apart in size rounds to 53 bits, then to a float, where the exe's 24-bit `FSUB` rounds once).
#[test]
fn range() {
    Diff::new(entry!(0x00e8_8730, state::set_range(Ptr<_>, f32, f32)))
        .common()
        .other_threads()
        .inputs(&[Input::F32, Input::F32])
        .translation_calls_plainly(&[])
        .float_sweep(FLOAT_SWEEP)
        .setup(|_e, c| {
            let w = world(_e);
            vec![w.cache, bits(c, 0), bits(c, 1)]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x00e8_8730, state::set_range(Ptr<_>, f32, f32)))
        .common()
        .main_thread(1)
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
        // The fade alpha: opaque, or below 1 (the blocked test's second half).
        Input::Of(&[ONE, HALF]),
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
        set(e, w.property + 0x2c, c.word(5));
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

/// `set_world_transform`'s words: every rotation entry and the scale from the case (as they came, or
/// folded by `fold`), and the renderer's directions' second components (the world's first and third
/// are 1, 2 and 1/3, -1): all eighteen products of the model directions' sums nonzero. Entries 0, 4
/// and 8 are the case's floats 0, 3 and 4; the others its floats 5, 6 and 7, each used twice with its
/// low mantissa bits changed (entries 1 and 5 from float 5, 2 and 6 from 6, 3 and 7 from 7).
fn world_transform_words(e: &mut Engine, c: &Case, fold: fn(u32) -> u32) -> Vec<u32> {
    let w = world(e);
    set(e, w.transform, fold(bits(c, 0)));
    set(e, w.transform + 0x10, fold(bits(c, 3)));
    set(e, w.transform + 0x20, fold(bits(c, 4)));
    for (entry, input) in [(1u32, 5), (2, 6), (3, 7), (5, 5), (6, 6), (7, 7)] {
        set(
            e,
            w.transform + 4 * entry,
            fold(bits(c, input) ^ (entry << 3)),
        );
    }
    set(e, w.transform + 0x30, fold(bits(c, 1)));
    set(e, w.renderer + 0x638, fold(bits(c, 3)));
    set(e, w.renderer + 0x644, fold(bits(c, 4)));
    vec![w.renderer, w.transform, c.word(2)]
}

/// A float as a rotation's entry, a direction's component or a scale has it in play: its magnitude
/// folded into 2^-4 to 4 (sign and mantissa kept; zeros and infinities as they come; a NaN the quiet
/// NaN 0x7fc00000, as [`play`]).
fn unit(bits: u32) -> u32 {
    let exponent = (bits >> 23) & 0xff;
    if exponent == 0xff && bits & 0x007f_ffff != 0 {
        return 0x7fc0_0000;
    }
    if exponent == 0xff || bits & 0x7fff_ffff == 0 || (123..129).contains(&exponent) {
        return bits;
    }
    (bits & 0x807f_ffff) | ((123 + exponent % 6) << 23)
}

/// The world transform runs on the main thread and another game thread: `f64` sums rounded at the
/// stores. Every rotation entry comes from the case, so each of the six sums has its three products
/// nonzero ([`world_transform_words`]). Under the other threads' word bit for bit (NaN payloads
/// aside), every input. Under the main thread's the exe rounds each of the three products and two
/// sums of a direction to 24 bits where the native sums round once at the store: floats within 3
/// ulps, or else within 2^-14 in value. The two part by at most 11 * 2^-24 of the largest product (the
/// exe's five roundings and the native one), which is many ulps of a result where the sums cancel to
/// near zero; with the rotation, the scale and the directions as play has them ([`unit`]: below 4, so
/// every product is below 2^6) it is below 2^-14.
#[test]
fn world_transform() {
    Diff::new(entry!(
        0x00e6_fbb0,
        acc::set_world_transform(Ptr<_>, Ptr<_>, u8)
    ))
    .common()
    .other_threads()
    .inputs(&[
        Input::F32,
        Input::F32,
        Input::Of(BYTES),
        Input::F32Swept(HALF),
        Input::F32Swept(ONE),
        Input::F32Swept(0x3eaa_aaab),
        Input::F32Swept(0xbf00_0000),
        Input::F32Swept(0x3e80_0000),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| world_transform_words(e, c, |b| b))
    .run()
    .assert_same();
    let report = Diff::new(entry!(
        0x00e6_fbb0,
        acc::set_world_transform(Ptr<_>, Ptr<_>, u8)
    ))
    .common()
    .main_thread(3)
    .float_abs(2f32.powi(-14))
    .inputs(&[
        Input::F32,
        Input::F32,
        Input::Of(BYTES),
        Input::F32Swept(HALF),
        Input::F32Swept(ONE),
        Input::F32Swept(0x3eaa_aaab),
        Input::F32Swept(0xbf00_0000),
        Input::F32Swept(0x3e80_0000),
    ])
    .translation_calls_plainly(&[])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| world_transform_words(e, c, unit))
    .run();
    println!("world_transform on the main thread: {report}");
    report.assert_same();
}

/// `set_world_view_constants`' words: the transform's scale from the case, as it came or folded by
/// `fold`.
fn world_view_words(e: &mut Engine, c: &Case, fold: fn(u32) -> u32) -> Vec<u32> {
    let w = world(e);
    let this = e.mem.alloc(0x20);
    set(e, this + 0x18, w.renderer);
    set(e, w.transform + 0x30, fold(bits(c, 0)));
    vec![this, c.word(1), w.transform]
}

/// The world-view constants' world transform is `f64` (it also runs on another game thread): under the
/// other threads' word bit for bit, every input; under the main thread's with the scale as play has it
/// ([`play`]: a denormal scale makes denormal entries, which the exe rounds twice and which compare
/// exactly), within 3 ulps, where the exe rounds each product and sum of the model directions to 24
/// bits (the world's rotation and directions keep the sums from cancelling: see `world_transform`).
#[test]
fn world_view_constants() {
    Diff::new(entry!(
        0x00b7_c750,
        acc::set_world_view_constants(u32, u32, u32)
    ))
    .common()
    .other_threads()
    .inputs(&[Input::F32, Input::Word])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| world_view_words(e, c, |b| b))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x00b7_c750,
        acc::set_world_view_constants(u32, u32, u32)
    ))
    .common()
    .main_thread(3)
    .inputs(&[Input::F32, Input::Word])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| world_view_words(e, c, play))
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

/// The tint copies its floats: bit for bit under either word, but a signalling NaN is no longer quieted
/// (the exe copies through the x87 stack), so NaN payloads may differ.
#[test]
fn tint() {
    Diff::new(entry!(0x00c0_2d90, acc::set_tint(u32, Ptr<_>, u32, u32)))
        .common()
        .allow_nan_payloads()
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

/// The alpha is compared and stored: bit for bit under either word, but a signalling NaN is no longer
/// quieted by its store (the exe's goes through the x87 stack), so NaN payloads may differ.
#[test]
fn set_alpha() {
    Diff::new(entry!(0x00ba_8ab0, acc::set_alpha(Ptr<_>, f32)))
        .common()
        .allow_nan_payloads()
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

/// The alpha sort's depth as the main thread sums it (the native maths is plain `f32` there):
/// `(x dx + y dy) + z dz`, each sum rounded to 24 bits before the next. With the camera along
/// (1, 1, 1), a bound at (1, y, z) of radius 1 against one on the x axis at the exe's depth of it,
/// radius 1 too. With y = z = 2^-24 each sum is a tie that goes to the even 1.0: depth 1.0, where
/// `x + (y + z)` or the sum rounded once gives 1 + 2^-23. With y = 2^-24 and z = 3 * 2^-25 the first
/// sum goes to 1.0 and the second to 1 + 2^-23, where `(x + z) + y` gives 1 + 2^-22 (its second sum a
/// tie to even). The two compare equal in both orders, with the radius taken off and not; each other
/// order, and the sum rounded once, fails in one of the two. On the main thread's word only, where
/// the code runs.
#[test]
fn compare_by_depth_at_24_bits_rounds_each_sum() {
    Diff::new(entry!(0x00b9_8b20, batch::compare_by_depth(u32, u32) -> i32))
        .common()
        .control_words(&[MAIN_THREAD])
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
        ])
        .setup(|e, c| {
            let w = world(e);
            set(e, w.camera + 0x68, ONE);
            set(e, w.camera + 0x74, ONE);
            set(e, w.camera + 0x80, ONE);
            e.mem.set_u8(0x011f_fe35, c.word(1) as u8);
            set(e, 0x011f_fe38, w.camera);
            // 2^-24; 3 * 2^-25; and the exe's depth of (1, y, z): 1.0, or 1 + 2^-23.
            let (z, depth) = if c.word(2) == 0 {
                (0x3380_0000, ONE)
            } else {
                (0x33c0_0000, ONE + 1)
            };
            let mut items = [0u32; 2];
            for (item, centre) in items.iter_mut().zip([[ONE, 0x3380_0000, z], [depth, 0, 0]]) {
                let bound = e.mem.alloc(0x10);
                for (i, v) in (0u32..).zip(centre.iter().chain(&[ONE])) {
                    set(e, bound + 4 * i, *v);
                }
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

/// `world_transform_typed_slot`'s words: the cache's slot 0x30 the typed `00e879e0`, the transform's
/// first rotation entry and its scale from the case, as they came or folded by `fold`.
fn world_transform_slot_words(e: &mut Engine, c: &Case, fold: fn(u32) -> u32) -> Vec<u32> {
    let w = world(e);
    let vt = vtable(e, &[(0x68, 0x00e8_8780), (0x30, 0x00e8_79e0)]);
    set(e, w.cache, vt);
    set(e, w.transform, fold(bits(c, 0)));
    set(e, w.transform + 0x30, fold(bits(c, 1)));
    vec![w.renderer, w.transform, c.word(2)]
}

/// `set_world_transform` with the render state cache's slot 0x30 holding the typed `00e879e0`: that call
/// made direct (the plain world has a stand-in there). Under the other threads' word bit for bit, every
/// input; under the main thread's with the floats as play has them ([`play`]), within 3 ulps (the
/// world's rotation and directions keep the sums from cancelling).
#[test]
fn world_transform_typed_slot() {
    Diff::new(entry!(
        0x00e6_fbb0,
        acc::set_world_transform(Ptr<_>, Ptr<_>, u8)
    ))
    .common()
    .other_threads()
    .inputs(&[Input::F32, Input::F32, Input::Of(BYTES)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| world_transform_slot_words(e, c, |b| b))
    .run()
    .assert_same();
    Diff::new(entry!(
        0x00e6_fbb0,
        acc::set_world_transform(Ptr<_>, Ptr<_>, u8)
    ))
    .common()
    .main_thread(3)
    .inputs(&[Input::F32, Input::F32, Input::Of(BYTES)])
    .translation_calls_plainly(&[])
    .float_sweep(0)
    .setup(|e, c| world_transform_slot_words(e, c, play))
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
    // The pathing family's square root takes `FSQRT` where the translation calls the C runtime's `sqrt`.
    .ignore_calls(&[0x00ec_6054])
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
/// and products of a group's fields round. As play has it ([`play`]): a zero seed's fields would be
/// denormals, an infinite one's next to the float's largest, and the set-ups' maths is native.
fn near(seed: u32, k: u32) -> u32 {
    play(seed ^ ((k.wrapping_mul(0x9e37_79b1) >> 8) & 0x00ff_ffff) ^ ((k & 4) << 29))
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

/// The calls the set-ups reach that stand in. The D3DX imports (the slots the thunks jump through: the
/// thunks are the draw family's, the same on both sides) move whole floats, so what a set-up
/// computes from their results is arithmetic on real numbers and every word they write is the same on
/// both sides: a vector function (out, in, ..) writes its three floats rotated by one, a plane's
/// normalize copies and its transform rotates the four, a matrix's transpose is the real one, its
/// multiply (out, a, b) writes `a` with `b`'s low bytes mixed in and its inverse (out, determinant, in)
/// writes the floats in reverse. The ambient and pass set-ups leave a mark of the call; the fog property
/// and the first active light answer from the table, the shader property's vtable +0x110 and +0x10c a
/// float each, the CRT's `_errno` a word of the table. The model matrix (`00c4c2d0`) and the per-light
/// set-up (`00b70820`) are the real ones: typed, in the dispatch and shader families.
fn geometry_stand_ins(e: &mut Engine) {
    for addr in [0x00fd_f414u32, 0x00fd_f420, 0x00fd_f41c] {
        e.register(addr, |e, a| {
            let v = words_at(e, a[1], 3);
            for k in 0..3u32 {
                e.mem.set_u32(a[0] + 4 * k, v[((k + 1) % 3) as usize]);
            }
            ret(a[0])
        });
    }
    e.register(0x00fd_f430, |e, a| {
        let v = words_at(e, a[1], 4);
        for k in 0..4u32 {
            e.mem.set_u32(a[0] + 4 * k, v[k as usize]);
        }
        ret(a[0])
    });
    e.register(0x00fd_f418, |e, a| {
        let v = words_at(e, a[1], 4);
        for k in 0..4u32 {
            e.mem.set_u32(a[0] + 4 * k, v[((k + 1) % 4) as usize]);
        }
        ret(a[0])
    });
    e.register(0x00fd_f42c, |e, a| {
        let m = words_at(e, a[1], 16);
        for k in 0..16u32 {
            e.mem
                .set_u32(a[0] + 4 * k, m[(4 * (k % 4) + k / 4) as usize]);
        }
        ret(a[0])
    });
    e.register(0x00fd_f424, |e, a| {
        let (m, n) = (words_at(e, a[1], 16), words_at(e, a[2], 16));
        for k in 0..16u32 {
            let word = m[k as usize] ^ (n[((5 * k + 3) % 16) as usize] & 0xff);
            e.mem.set_u32(a[0] + 4 * k, word);
        }
        ret(a[0])
    });
    e.register(0x00fd_f428, |e, a| {
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

/// The world of the set-up tests: [`world`] with everything the set-ups read made from `s`, its floats
/// as play has them ([`play`]: the set-ups' maths is native, plain `f32` on the main thread). The tests
/// name the render and dispatch families ([`Geometry::set_ups`]), so their functions are the real ones.
fn geometry_shape(e: &mut Engine, s: &Shape) -> Geo {
    let s = &Shape {
        floats: s.floats.map(play),
        ..*s
    };
    let w = world_named(e, true, true);
    // The renderer's two directions zero: the world-view constants' world transform then turns them into
    // model-space directions of zero on both sides. Its sums are `f64` (it also runs on another thread),
    // which on the main thread's word differ from the exe's 24-bit steps in the last bits, more where
    // they cancel: its own tests compare them (`world_transform`, `world_view_constants`).
    for k in 0..6u32 {
        set(e, w.renderer + 0x634 + 4 * k, 0);
    }
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

/// The dispatch family's functions the set-ups call (the 4x4 inverse, the normalisation): by address in
/// the set-ups' tests, to their translations on both sides. Their maths is native, rounding a sum of
/// products once where the exe's 24-bit code rounds each step, so on the main thread's word, the only
/// one the set-ups run on, they part from the exe in the last bits (the dispatch family's own tests
/// compare them there, within their tolerance; the render family's tests run them direct).
const SET_UP_DISPATCH: &[u32] = &[0x00c4_c2d0, 0x004a_0c10];

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
            .family(&pathing::PATHING)
            // The pathing family's square root takes `FSQRT` where the translation calls the C
            // runtime's `sqrt`.
            .ignore_calls(&[0x00ec_6054])
            // The held accumulator's getter (`00b4f5d0`) is a read of `011f95f0` (own state), done in place
            // through the packet (`packet::held_accumulator`).
            .ignore_calls(&[0x00b4_f5d0])
            .family(&render::RENDER)
            .goes_by_address(SET_UP_DISPATCH)
            .translation_calls_plainly(SET_UP_PLAIN)
            // The draw family's partition draw, skinned packer and strip lists, land toggles, fog
            // property and D3DX thunks, as in `common`.
            .goes_by_address(&[
                0x00e6_d310,
                0x00e6_d780,
                0x00a7_5f10,
                0x00c0_2c30,
                0x00b5_5520,
                0x00ee_6dfe,
                0x00ee_6dec,
                0x00ee_6df2,
                0x00ee_6df8,
                0x00ee_6dd4,
                0x00ee_6dda,
                0x00ee_6de0,
                0x00ee_6dce,
                0x00ee_6dc8,
            ])
            .goes_by_address(DRAW_F_HELPERS)
            .max_calls(MAX_CALLS)
            .max_cases(160)
            // The set-ups run on the main thread only (the thread census; each one's only caller is
            // `SetupGeometryConstants`, and the light set-up's are the lights' loops): native `f32`,
            // compared under the main thread's control word, within one ulp (a step with a `double`
            // operand, a luminance weight or a constant, rounds twice natively).
            .main_thread(1)
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
                vec![c.word(0), light, play_bits(c, 5)]
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
// preparation and table, the fog ambient set-up, the render pass tree, the reflection
// copy's renderer calls, the decal pass, a holder's own list draw, a sorted group's
// destructor and the list's `RemoveAll`).

use engine_typed::draw::{self, alpha as dalpha, drivers, own, packet};

/// Callees of the draw outside the families, stood in.
fn draw_stand_ins(e: &mut Engine) {
    for addr in [
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
    // The render object's camera data slot (copying what its pointers point to, so the digest compares the
    // camera the draw hands over), and its check for the "SetCameraData" extra: present.
    e.map(CAMERA_DATA_COPY, 0x1000);
    e.register(VT_FN + 0x18c, camera_data_answer);
    e.register(0x004a_0e10, |e, _| ret(e.mem.u32(RETS + 0x7f0)));
    set(e, RETS + 0x7f0, 1);
    // The fog-coloured ambient's targets and its enable object.
    e.map(0x0120_0000, 0x3000);
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

/// Where [`camera_data_answer`] copies the camera data it is handed (a block `draw_stand_ins` maps).
const CAMERA_DATA_COPY: u32 = 0x0121_4000;

/// The render object's slot 0x18c, `thiscall(object, translate, column 0, column 1, column 2, port, frustum)`:
/// the bytes each pointer points to (a point and three columns of 0xc bytes, the port's 0x10, the frustum's
/// 0x1c) copied to [`CAMERA_DATA_COPY`], an unmapped one left out; then the slot's answer.
fn camera_data_answer(e: &mut Engine, a: &[u32]) -> Ret {
    let mut at = CAMERA_DATA_COPY;
    for (i, size) in [0xcu32, 0xc, 0xc, 0xc, 0x10, 0x1c].into_iter().enumerate() {
        let mut bytes = vec![0u8; size as usize];
        if let Some(&from) = a.get(i + 1) {
            if e.mem.try_read(from, &mut bytes).is_ok() {
                e.mem.write(at, &bytes);
            }
        }
        at += size;
    }
    slot_answer::<0x18c>(e, a)
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
        // `RenderBatches`' accumulator getter is the render family's (its own tests).
        .goes_by_address(&[0x00b4_f5c0])
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(TWO_WAYS),
            Input::Of(&[0, 1, 0x33]),
            // The renderer's slot 0x90: a stand-in, or `RenderBatches` (then the typed one, direct).
            Input::Of(TWO_WAYS),
        ])
        .setup(|e, c| {
            let d = draw_world(e);
            let holder = d.w.holders[c.word(0) as usize];
            if c.word(1) == 0 {
                set(e, holder + 8, 0);
            }
            if c.word(3) != 0 {
                let vt = e.mem.u32(d.w.br);
                set(e, vt + 0x90, 0x00b9_9e20);
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

/// The shader manager's mode (`00b4f450`, typed: on every caller's path): every mode of the tables and past them,
/// each table's entry there set or null (then its first entry, itself set or null), every write (the mode and
/// both function words).
#[test]
fn select_shader_mode() {
    Diff::new(entry!(0x00b4_f450, own::select_shader_mode(u32)))
        .draw_common_at(0x00b4_f450)
        .inputs(&[
            Input::Of(&[0, 1, 3, 8, 9, 10, 0xf, 0x10, 0x11]),
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(TWO_WAYS),
        ])
        .setup(|e, c| {
            let _ = draw_world(e);
            let (mode, nulls, first_null) = (c.word(0), c.word(1), c.word(2));
            for k in 0..0x12u32 {
                let a = if k == mode && nulls & 1 != 0 {
                    0
                } else {
                    0x4400 + k
                };
                let b = if k == mode && nulls & 2 != 0 {
                    0
                } else {
                    0x8800 + k
                };
                set(e, 0x011f_9f40 + 4 * k, a);
                set(e, 0x011f_9f80 + 4 * k, b);
            }
            if first_null != 0 {
                set(e, 0x011f_9f40, 0);
                set(e, 0x011f_9f80, 0);
            }
            set(e, 0x011f_91b8, 0x5555);
            set(e, 0x011f_9f38, 0x6666);
            set(e, 0x011f_9f3c, 0x7777);
            vec![mode]
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

// ---- The draw family's pass begin and end (group B). ----
//
// The draw world ([`draw_world`], which already gives the pass set-ups what they read when the drivers'
// tests reach them) with a fuller current shader pass (`0126f74c`, [`pass_world`]): a render state list
// of the case's states, six stages of the world's kind (stage 2 without a texture), a mask byte, an
// object at +0x44, the world's constants holder; the shader's sources for its counted pointers; the
// constant entries, their pass type words and masks; a fog property for scene node 1. What stands in:
// the setters `restore_state` calls by address, and the shader's own set-ups and D3DX helpers the pass
// set-ups call by address (each folds its words into a mark), the pass's constants target (`00e7f7c0`,
// an object whose slots 0x7c and 0x8c are the shader family's shader setters).

use engine_typed::draw::{constants as dconstants, passes, setups};

trait PassCommon {
    fn pass_common(self) -> Diff;
}

impl PassCommon for Diff {
    /// Group B's settings: the draw family named; the translations its tests reach call the shader
    /// family's setters by address (the cull mode setter too, which other draws' translations call
    /// plainly), so nothing is listed as called plainly.
    fn pass_common(self) -> Diff {
        self.common().family(&draw::DRAW).float_sweep(0)
    }
}

/// The setters `restore_state` calls by address.
const RESTORE_SETTERS: &[u32] = &[
    0x0071_4a40,
    0x0071_4a60,
    0x0071_4aa0,
    0x0071_4b00,
    0x0071_4a80,
    0x0071_4ac0,
    0x0071_4ae0,
    0x004e_cb40,
    0x0071_4b20,
    0x0071_4b50,
    0x0071_4b80,
    0x004e_b510,
    0x004e_ced0,
    0x00b9_85f0,
    0x0071_4bf0,
];

/// The setters among them that are the draw family's (group C's lock counters).
const DRAW_COUNTERS: &[u32] = &[
    0x0071_4a40,
    0x0071_4a60,
    0x0071_4aa0,
    0x0071_4ae0,
    0x004e_ced0,
];

/// A stand-in that folds its words into a mark (`RETS + 0x780`), so the calls' order and words show.
fn marked(e: &mut Engine, a: &[u32]) -> Ret {
    let mut h = e
        .mem
        .u32(RETS + 0x780)
        .wrapping_mul(31)
        .wrapping_add(a.len() as u32);
    for w in a {
        h = h.wrapping_mul(17).wrapping_add(*w);
    }
    e.mem.set_u32(RETS + 0x780, h);
    ret(0)
}

/// The device's float vertex constants (slot 0x178) stood in: the register and the registers' words
/// folded into a mark (`RETS + 0x784`), so what a set-up uploads shows. Answers 0 (success).
fn folded_constants(e: &mut Engine, a: &[u32]) -> Ret {
    let mut h = e.mem.u32(RETS + 0x784).wrapping_mul(31).wrapping_add(a[1]);
    for k in 0..a[3].min(16) * 4 {
        h = h
            .wrapping_mul(17)
            .wrapping_add(e.mem.u32(a[2].wrapping_add(4 * k)));
    }
    e.mem.set_u32(RETS + 0x784, h);
    ret(0)
}

/// What [`pass_world`] makes: the draw world and its current pass.
#[derive(Clone, Copy)]
struct PassW {
    d: DrawW,
    pass: u32,
}

/// The draw world with a fuller current pass ([`PassW`]): `states` as its render state list (state,
/// value), `mask` its mask byte; the first pass's geometry being rendered, of type `pass_type`. The
/// render family's stage bind (`00e7eb00`, which these tests don't name) stands in: each bind leaves a
/// mark, so which stages are bound shows.
fn pass_world(e: &mut Engine, states: &[(u32, u32)], mask: u8, pass_type: u32) -> PassW {
    let d = draw_world(e);
    let w = d.w;
    e.register(0x00e7_eb00, marked);
    for addr in RESTORE_SETTERS {
        // The draw family's own counters (group C) run as themselves: these tests name the family.
        if !DRAW_COUNTERS.contains(addr) {
            e.register(*addr, marked);
        }
    }
    e.register(VT_FN + 0x178, folded_constants);
    let vt = vtable(e, &[]);
    // The constants holder: it answers through its slots 0x84 and 0x7c; the device at +0x20.
    let holder = obj(e, 0x24, vt);
    set(e, holder + 0x20, w.device);
    let pass = e.mem.alloc(0x60);
    set(e, pass + 0x5c, holder);
    let list = e.mem.alloc(0x20);
    set(e, pass + 0x30, list);
    let mut next = 0u32;
    for (state, value) in states.iter().rev() {
        let node = e.mem.alloc(0xc);
        set(e, node, *state);
        set(e, node + 4, *value);
        set(e, node + 8, next);
        next = node;
    }
    set(e, list + 0x10, next);
    // Sixteen stages (the pair textures reach stage 14), the pass counting six.
    let array = e.mem.alloc(0x40);
    for i in 0..16u32 {
        let s = e.mem.alloc(0x14);
        set(e, s + 4, i);
        set(e, s + 8, if i == 2 { 0 } else { 0x7e00_0000 + i });
        set(e, s + 0x10, w.stage_cache);
        set(e, array + 4 * i, s);
    }
    set(e, pass + 0x18, 6);
    set(e, pass + 0x24, array);
    e.mem.set_u8(pass + 0x4c, mask);
    let second = obj(e, 0x10, vt);
    set(e, pass + 0x44, second);
    set(e, 0x0126_f74c, pass);
    // The pass being rendered: the world's first pass, its geometry placed by the world's transform.
    let geometry = e.mem.u32(w.passes[0]);
    for k in 0..13u32 {
        let word = e.mem.u32(w.transform + 4 * k);
        set(e, geometry + 0x68 + 4 * k, word);
    }
    let rendering = e.mem.u32(0x011f_91e0);
    set(e, rendering, geometry);
    set(e, 0x011f_91e4, pass_type);
    // The shader's renderer (+0x18), which its world-view constants go through.
    set(e, w.shader + 0x18, w.renderer);
    // The exe's 1.0 (a double, `01012070`) the set-ups and the land toggles add or take off.
    e.mem.set_u64(0x0101_2070, 1.0f64.to_bits());
    PassW { d, pass }
}

const PASS_STATES: &[(u32, u32)] = &[(7, 1), (0x13, 5), (0x16, 2), (0xbe, 0x3f80_0000), (0x89, 0)];

/// A render state put back: the states with a setter of their own (each kind), and others through
/// the render-state cache's slot 0x68 (the real cache, or a cache whose slot is another function).
#[test]
fn restore_state() {
    Diff::new(entry!(0x00b9_8610, passes::restore_state(u32, u32)))
        .pass_common()
        .inputs(&[
            Input::Of(&[
                7, 0xe, 0xf, 0x13, 0x14, 0x17, 0x18, 0x19, 0x1b, 0x34, 0x35, 0x36, 0x37, 0x38,
                0x39, 0x3a, 0x3b, 0x98, 0xa8, 0xbe, 0xc3, 0x16, 0x1a, 0x89, 0, 6, 0xc4, 0xff,
            ]),
            Input::Of(&[0, 1, 0x3f80_0000]),
            Input::Of(TWO_WAYS),
        ])
        .max_cases(200)
        .setup(|e, c| {
            let p = pass_world(e, &[], 0, 0);
            if c.word(2) != 0 {
                let cache_vt = vtable(e, &[]);
                e.register(VT_FN + 0x68, marked);
                set(e, p.d.w.cache, cache_vt);
            }
            set(
                e,
                p.d.w.cache + 0x124 + 8 * c.word(0),
                0x5eed_0000 + c.word(0),
            );
            vec![c.word(0), c.word(1)]
        })
        .run()
        .assert_same();
}

/// The pass's render states set (slot 0x100) or put back (slot 0x104): no states, one, several; the
/// cache's locks clear or set.
const PASS_STATES_INPUTS: &[Input] = &[Input::Of(&[0, 1, 5]), Input::Of(&[0, 1, 2])];

fn pass_states_setup(e: &mut Engine, c: &Case) -> Vec<u32> {
    let states = &PASS_STATES[..c.word(0) as usize];
    let p = pass_world(e, states, 0, 0);
    for k in 0..8u32 {
        e.mem.set_u8(p.d.w.cache + 0x920 + k, (c.word(1) & 1) as u8);
    }
    vec![p.d.w.shader, c.word(1)]
}

#[test]
fn set_pass_states() {
    Diff::new(entry!(0x00be_0eb0, passes::set_pass_states(u32, u32)))
        .pass_common()
        .inputs(PASS_STATES_INPUTS)
        .setup(pass_states_setup)
        .run()
        .assert_same();
}

#[test]
fn restore_pass_states() {
    Diff::new(entry!(0x00be_0ee0, passes::restore_pass_states(u32, u32)))
        .pass_common()
        .inputs(PASS_STATES_INPUTS)
        .setup(pass_states_setup)
        .run()
        .assert_same();
}

/// The alpha function (slot 0x84) and the alpha test with the cull mode (slot 0x88) of a geometry's
/// state block: its alpha property's flags and reference, its second property's bits 10 and 11, the
/// pass types 0x56 and 0x57, the flag; a null block for the test.
#[test]
fn alpha_function_and_test_of() {
    Diff::new(entry!(0x00be_20b0, passes::alpha_function_of(u32, u32)))
        .pass_common()
        .inputs(&[
            Input::Of(&[0, 0x1c00, 0x0e01, 0xffff]),
            Input::Of(&[0, 0x80, 0xff]),
        ])
        .setup(|e, c| {
            let p = pass_world(e, &[], 0, 0);
            let block = e.mem.alloc(0x20);
            let vt = vtable(e, &[]);
            let alpha = obj(e, 0x1c, vt);
            e.mem.set_u16(alpha + 0x18, c.word(0) as u16);
            e.mem.set_u8(alpha + 0x1a, c.word(1) as u8);
            set(e, block, alpha);
            vec![p.d.w.shader, block]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x00be_20e0, passes::alpha_test_of(u32, u32, bool)))
        .pass_common()
        .inputs(&[
            Input::Of(&[0, 1, 2, 3]),
            Input::Of(&[0, 0x200, 0xc00, 0xe00]),
            Input::Of(&[0x10, 0x56, 0x57, 0x58]),
            Input::Of(TWO_WAYS),
        ])
        .setup(|e, c| {
            let p = pass_world(e, &[], 0, c.word(2));
            let vt = vtable(e, &[]);
            let block = match c.word(0) {
                0 => 0,
                _ => e.mem.alloc(0x20),
            };
            if block != 0 {
                let flags = c.word(1) as u16;
                if c.word(0) & 1 != 0 {
                    let first = obj(e, 0x1c, vt);
                    e.mem.set_u16(first + 0x18, flags);
                    set(e, block, first);
                }
                if c.word(0) >= 2 {
                    let second = obj(e, 0x1c, vt);
                    e.mem.set_u16(second + 0x18, flags);
                    set(e, block + 0x10, second);
                }
            }
            vec![p.d.w.shader, block, c.word(3)]
        })
        .run()
        .assert_same();
}

/// The pass end's states (slot 0x110) by the type being rendered, and put back (slot 0x114).
#[test]
fn end_states() {
    Diff::new(entry!(0x00be_0e50, passes::end_states(u32, u32)))
        .pass_common()
        .inputs(&[
            Input::Of(&[0x10, 0x251, 0x252, 0x254, 0x256, 0x257]),
            Input::Of(&[0, 1]),
        ])
        .setup(|e, c| {
            let p = pass_world(e, &[], 0, c.word(0));
            for k in [0x1b, 0xe, 0x17] {
                e.mem.set_u8(p.d.w.cache + 0x920 + k, c.word(1) as u8);
            }
            vec![p.d.w.shader, 0]
        })
        .run()
        .assert_same();
    Diff::new(entry!(0x00be_0e90, passes::end_states_restored(u32)))
        .pass_common()
        .inputs(&[Input::Of(&[0, 1])])
        .setup(|e, c| {
            let p = pass_world(e, &[], 0, 0);
            for k in [0x1b, 0xe, 0x17] {
                e.mem.set_u8(p.d.w.cache + 0x920 + k, c.word(0) as u8);
            }
            vec![p.d.w.shader]
        })
        .run()
        .assert_same();
}

/// The masked stages bound: masks choosing none, some and all of the element bits; a stage without a
/// texture; a null stage ending the run.
#[test]
fn bind_masked_stages() {
    Diff::new(entry!(0x00be_21b0, passes::bind_masked_stages()))
        .pass_common()
        .goes_by_address(&[0x00e7_eb00])
        .inputs(&[
            Input::Of(&[0, 1, 0x0b, 0x7f, 0x80, 0xff]),
            Input::Of(TWO_WAYS),
        ])
        .setup(|e, c| {
            let p = pass_world(e, &[], c.word(0) as u8, 0);
            for i in 0..8u32 {
                // Stage 5's bit above the mask byte: only a sign-extended mask has it.
                set(e, 0x011a_e640 + 4 * i, if i == 5 { 0x100 } else { 1 << i });
            }
            if c.word(1) != 0 {
                let array = e.mem.u32(p.pass + 0x24);
                set(e, array + 4 * 4, 0);
            }
            vec![]
        })
        .run()
        .assert_same();
}

/// The pass begin's end (slot 0xf4): the target's shader setters real or stood in, with and without
/// the pass's object +0x44.
#[test]
fn begin_done() {
    Diff::new(entry!(0x00be_1f90, passes::begin_done(u32, u32)))
        .pass_common()
        .inputs(&[Input::Of(TWO_WAYS), Input::Of(TWO_WAYS)])
        .setup(|e, c| {
            let p = pass_world(e, &[], 0, 0);
            let vt = if c.word(0) != 0 {
                vtable(e, &[(0x7c, 0x00e8_8870), (0x8c, 0x00e9_0e00)])
            } else {
                vtable(e, &[])
            };
            let target = obj(e, 0x1114, vt);
            for k in (4..0x1114).step_by(4) {
                let word = e.mem.u32(p.d.w.cache + k);
                set(e, target + k, word);
            }
            set(e, RETS + 0x7f8, target);
            e.register(0x00e7_f7c0, |e, _| ret(e.mem.u32(RETS + 0x7f8)));
            if c.word(1) == 0 {
                set(e, p.pass + 0x44, 0);
            }
            set(e, RETS + 0x84, 0x0517_0084);
            set(e, RETS + 0x7c, 0x0517_007c);
            vec![p.d.w.shader, 0]
        })
        .run()
        .assert_same();
}

/// The fog property of a scene node.
#[test]
fn fog_property() {
    Diff::new(entry!(0x00b5_5520, passes::fog_property(u32) -> u32))
        .pass_common()
        .inputs(&[Input::Of(&[0, 1, 2])])
        .setup(|e, c| {
            pass_world(e, &[], 0, 0);
            for i in 0..3u32 {
                let node = e.mem.alloc(0x140);
                set(e, node + 0x134, 0x0f06_0000 + i);
                set(e, 0x011f_91c8 + 4 * i, node);
            }
            vec![c.word(0)]
        })
        .run()
        .assert_same();
}

/// The shader's own set-ups the pass set-ups call by address and the D3DX imports behind the thunks they
/// call (`D3DXVec3TransformCoord`, `D3DXMatrixTranspose`), stood in.
const SETUP_CALLEES: &[u32] = &[
    0x00b7_c3a0,
    0x00b7_c510,
    0x00b7_c580,
    0x00b7_c600,
    0x00b7_c850,
    0x00b7_cb00,
    0x00fd_f414,
    0x00fd_f42c,
];

/// A geometry's state block whose shader property (+0xc) is the world's, with a texture word at its
/// +0xac and its slot 0xf4 answering `fourth` (stage 3's texture).
fn setup_state(e: &mut Engine, p: &PassW, fourth: u32) -> u32 {
    let w = p.d.w;
    for addr in SETUP_CALLEES {
        e.register(*addr, marked);
    }
    let source = e.mem.alloc(4);
    set(e, source, 0x7e00_00ac);
    set(e, w.property + 0xac, source);
    set(e, RETS + 0xf4, fourth);
    let state = e.mem.alloc(0x20);
    set(e, state + 8, 0x0517_0008);
    set(e, state + 0xc, w.property);
    state
}

/// A begin of a type group (0xfc to 0x102, or 0xca to 0xd1): the shader's object +0x30 and the pass's
/// object +0x44 there or not.
fn begin_types_setup(e: &mut Engine, c: &Case) -> Vec<u32> {
    let p = pass_world(e, &[], 0, 0);
    setup_state(e, &p, 0);
    let vt = vtable(e, &[]);
    for at in [0x011f_ebf0u32, 0x011f_ec38] {
        let switch = obj(e, 0x24, vt);
        set(e, at, switch);
    }
    if c.word(0) == 0 {
        set(e, p.d.w.shader + 0x30, 0);
    }
    if c.word(1) == 0 {
        set(e, p.pass + 0x44, 0);
    }
    vec![p.d.w.shader]
}

#[test]
fn begin_types_fc() {
    Diff::new(entry!(0x00b7_dd50, setups::begin_types_fc(Ptr<_>)))
        .pass_common()
        .goes_by_address(&[0x00e7_eb00])
        .inputs(&[Input::Of(TWO_WAYS), Input::Of(TWO_WAYS)])
        .setup(begin_types_setup)
        .run()
        .assert_same();
}

#[test]
fn begin_types_ca() {
    Diff::new(entry!(0x00b7_deb0, setups::begin_types_ca(Ptr<_>)))
        .pass_common()
        .goes_by_address(&[0x00e7_eb00])
        .inputs(&[Input::Of(TWO_WAYS), Input::Of(TWO_WAYS)])
        .setup(begin_types_setup)
        .run()
        .assert_same();
}

/// A per-geometry set-up of the types 0xca, 0xd1 or 0x102: stage 3's texture answered or not, the
/// rendered pass's byte +9, the rendered geometry's scale over the special floats.
const SETUP_TYPE_INPUTS: &[Input] = &[
    Input::Of(&[0, 0x7e00_00f4]),
    Input::Of(&[0, 1, 0xff]),
    Input::F32,
];

fn setup_type_setup(e: &mut Engine, c: &Case) -> Vec<u32> {
    setup_type_words(e, c, |b| b)
}

/// [`setup_type_setup`] with the scale as play has it ([`play`]): the main thread's comparison.
fn setup_type_play(e: &mut Engine, c: &Case) -> Vec<u32> {
    setup_type_words(e, c, play)
}

/// A pass set-up's words, the geometry's scale from the case folded by `fold`.
fn setup_type_words(e: &mut Engine, c: &Case, fold: fn(u32) -> u32) -> Vec<u32> {
    let p = pass_world(e, &[], 0, 0);
    let state = setup_state(e, &p, c.word(0));
    let rendering = e.mem.u32(0x011f_91e0);
    e.mem.set_u8(rendering + 9, c.word(1) as u8);
    let geometry = e.mem.u32(rendering);
    set(e, geometry + 0x98, fold(bits(c, 2)));
    set(e, 0x011f_951c, 0x7e00_951c);
    vec![p.d.w.shader, state]
}

// The three pass set-ups reach the shader family's world transform (through the world-view constants)
// with the case's float as the geometry's scale: its maths is native `f64`. Under the other threads'
// word bit for bit (NaN payloads aside: the scale it hands the matrix is no longer quieted), every
// input; under the main thread's with the scale as play has it (a denormal scale makes denormal
// entries, which the exe rounds twice and which compare exactly), within one ulp (the exe rounds each
// product and sum of the model directions to 24 bits).

#[test]
fn setup_type_ca() {
    Diff::new(entry!(0x00b7_ded0, setups::setup_type_ca(Ptr<_>, Ptr<_>)))
        .pass_common()
        .other_threads()
        .goes_by_address(&[0x00e7_eb00, 0x00c4_c2d0])
        .inputs(SETUP_TYPE_INPUTS)
        .max_cases(160)
        .setup(setup_type_setup)
        .run()
        .assert_same();
    Diff::new(entry!(0x00b7_ded0, setups::setup_type_ca(Ptr<_>, Ptr<_>)))
        .pass_common()
        .main_thread(1)
        .goes_by_address(&[0x00e7_eb00, 0x00c4_c2d0])
        .inputs(SETUP_TYPE_INPUTS)
        .max_cases(160)
        .setup(setup_type_play)
        .run()
        .assert_same();
}

#[test]
fn setup_type_d1() {
    Diff::new(entry!(0x00b7_dfe0, setups::setup_type_d1(Ptr<_>, Ptr<_>)))
        .pass_common()
        .other_threads()
        .goes_by_address(&[0x00e7_eb00, 0x00c4_c2d0])
        .inputs(SETUP_TYPE_INPUTS)
        .max_cases(160)
        .setup(setup_type_setup)
        .run()
        .assert_same();
    Diff::new(entry!(0x00b7_dfe0, setups::setup_type_d1(Ptr<_>, Ptr<_>)))
        .pass_common()
        .main_thread(1)
        .goes_by_address(&[0x00e7_eb00, 0x00c4_c2d0])
        .inputs(SETUP_TYPE_INPUTS)
        .max_cases(160)
        .setup(setup_type_play)
        .run()
        .assert_same();
}

#[test]
fn setup_type_102() {
    Diff::new(entry!(0x00b7_e150, setups::setup_type_102(Ptr<_>, Ptr<_>)))
        .pass_common()
        .other_threads()
        .goes_by_address(&[0x00e7_eb00, 0x00c4_c2d0])
        .inputs(SETUP_TYPE_INPUTS)
        .max_cases(160)
        .setup(setup_type_setup)
        .run()
        .assert_same();
    Diff::new(entry!(0x00b7_e150, setups::setup_type_102(Ptr<_>, Ptr<_>)))
        .pass_common()
        .main_thread(1)
        .goes_by_address(&[0x00e7_eb00, 0x00c4_c2d0])
        .inputs(SETUP_TYPE_INPUTS)
        .max_cases(160)
        .setup(setup_type_play)
        .run()
        .assert_same();
}

/// The vertex (`011febd8`) and pixel (`011fec30`) constant entries, 22 each: every third missing (but
/// entry 8, which the pass constants read), the others on or off.
fn entries(e: &mut Engine) {
    let vt = vtable(e, &[]);
    for list in [0x011f_ebd8u32, 0x011f_ec30] {
        for k in 0..22u32 {
            let entry = if k % 3 == 2 && k != 8 {
                0
            } else {
                obj(e, 0x24, vt)
            };
            if entry != 0 {
                e.mem.set_u8(entry + 8, (k % 2) as u8);
            }
            set(e, list + 4 * k, entry);
        }
    }
}

/// The tail of the pass kinds 1 to 5: the two masks of the pass type clear or partly set, the switch
/// lists' entries on, off or missing.
#[test]
fn tail_kinds_1_to_5() {
    Diff::new(entry!(0x00b7_ac20, setups::tail_kinds_1_to_5(Ptr<_>, u32)))
        .pass_common()
        .inputs(&[
            Input::Of(&[0, 0x0000_0006, 0x00a5_a5a4, 0xffff_ffff]),
            Input::Of(&[0, 0x0000_0018, 0x5a5a_5a5a]),
            Input::Of(&[0x10, 0x2f0]),
        ])
        .setup(|e, c| {
            let p = pass_world(e, &[], 0, 0);
            entries(e);
            set(e, 0x011f_b4c0 + 4 * c.word(2), c.word(0));
            set(e, 0x011f_a8e0 + 4 * c.word(2), c.word(1));
            vec![p.d.w.shader, c.word(2)]
        })
        .run()
        .assert_same();
}

/// The pass constants: types 0x250 and 0x251 (nothing), others with their entry words; the fog
/// property there or not, with distances or none; the rendered geometry skinned or not.
#[test]
fn setup_pass_constants() {
    Diff::new(entry!(
        0x00b7_b450,
        setups::setup_pass_constants(Ptr<_>, u32)
    ))
    .pass_common()
    .inputs(&[
        Input::Of(&[0x10, 0x250, 0x251, 0x2f0]),
        Input::Of(&[0, 0x0000_0200, 0xffff_ffff]),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(TWO_WAYS),
        Input::F32,
    ])
    .max_cases(260)
    .setup(|e, c| {
        let p = pass_world(e, &[], 0, 0);
        for addr in SETUP_CALLEES {
            e.register(*addr, marked);
        }
        entries(e);
        let pass_type = c.word(0);
        set(e, 0x011f_cc80 + 4 * pass_type, c.word(1));
        set(e, 0x011f_c0a0 + 4 * pass_type, c.word(1).rotate_left(3));
        // The fog property of scene node 1: none, without distances, with them.
        e.map(0x010a_d000, 0x1000);
        set(e, 0x010a_d6e0, 0x48f4_2400);
        e.mem.set_u8(0x011f_91c4, 1);
        let node = e.mem.alloc(0x140);
        set(e, 0x011f_91c8 + 4, node);
        if c.word(2) != 0 {
            let fog = e.mem.alloc(0x64);
            for (k, word) in [ONE, HALF, 0x3e80_0000].iter().enumerate() {
                set(e, fog + 0x20 + 4 * k as u32, *word);
            }
            if c.word(2) == 2 {
                set(e, fog + 0x2c, bits(c, 4));
                set(e, fog + 0x30, TWO);
                set(e, fog + 0x60, 0x3f40_0000);
            }
            // A near distance with no far one.
            if c.word(2) == 3 {
                set(e, fog + 0x2c, ONE);
            }
            set(e, node + 0x134, fog);
        }
        let geometry = e.mem.u32(e.mem.u32(0x011f_91e0));
        let skin = if c.word(3) != 0 { p.d.w.skin } else { 0 };
        set(e, geometry + 0xbc, skin);
        set(e, p.d.w.shader + 0x18, p.d.w.renderer);
        vec![p.d.w.shader, pass_type]
    })
    .run()
    .assert_same();
}

/// Every pass type up to 0x2f0, for [`select_pass`] (each switch table's arms).
static EVERY_PASS_TYPE: [u32; 0x2f0] = {
    let mut a = [0u32; 0x2f0];
    let mut i = 0;
    while i < 0x2f0 {
        a[i] = i as u32;
        i += 1;
    }
    a
};

/// The pass choice: every pass type, the shader's counted pointers the same as their sources or not,
/// with counts that reach 0 when let go or not.
#[test]
fn select_pass() {
    Diff::new(entry!(0x00b7_af80, setups::select_pass(Ptr<_>, u32)))
        .pass_common()
        .inputs(&[Input::Of(&EVERY_PASS_TYPE), Input::Of(&[0, 1, 2, 3, 4])])
        .max_cases(0x2f0 * 5)
        .setup(|e, c| {
            // The plain world: the pass choice reads only the shader and the pass table.
            let w = world_all(e, false, false, false);
            let shader = w.shader;
            let vt = vtable(e, &[]);
            let pass_type = c.word(0);
            set(e, 0x011f_dff8 + 4 * pass_type, 0x0a55_0000 + pass_type);
            // The sources +0x6c to +0x88; with way 1 the held pointers are the sources already, with
            // ways 0 and 2 others whose counts are 1 (let go: deleted) and 2, with way 3 none held, with
            // way 4 the inline sources (+0x70, +0x7c, +0x80) null.
            for k in 0..8u32 {
                let o = obj(e, 0x10, vt);
                set(e, o + 4, 2 + k);
                set(e, shader + 0x6c + 4 * k, o);
            }
            for (at, from) in [(0x28u32, 0x70u32), (0x30, 0x7c), (0x34, 0x80)] {
                let held = match c.word(1) {
                    1 => e.mem.u32(shader + from),
                    3 => 0,
                    _ => {
                        let o = obj(e, 0x10, vt);
                        set(e, o + 4, 1 + (c.word(1) % 4) / 2);
                        o
                    }
                };
                set(e, shader + at, held);
                if c.word(1) == 4 {
                    set(e, shader + from, 0);
                }
            }
            vec![shader, pass_type]
        })
        .run()
        .assert_same();
}

/// The land toggles: the flag, the byte of the geometry's table, its count over the edge values.
#[test]
fn land_toggles() {
    Diff::new(entry!(
        0x00c0_2c30,
        setups::land_toggles(Ptr<_>, u32, u32, u32, u32)
    ))
    .pass_common()
    .inputs(&[
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1, 3]),
        Input::Of(&[0, 1, 0x7fff, 0xffff]),
    ])
    .setup(|e, c| {
        let p = pass_world(e, &[], 0, 0);
        e.register(VT_FN + 0xec, slot_answer::<0xec>);
        let vt = vtable(e, &[]);
        let geometry = obj(e, 0xc8, vt);
        let table = e.mem.alloc(4);
        set(e, table, 0xff80_0700);
        set(e, geometry + 0xc4, table);
        set(e, RETS + 0xec, 0x0001_0000 | c.word(2));
        set(e, 0x011a_9bd4, 0x3e80_0000);
        vec![p.d.w.shader, 0x0bad_0001, c.word(0), geometry, c.word(1)]
    })
    .run()
    .assert_same();
}

/// A lighting property's fade: its flags and lock bit, the near and far distances and the property's
/// distance over the special floats.
#[test]
fn property_fade() {
    Diff::new(entry!(0x00b6_6b80, setups::property_fade(Ptr<_>) -> f32))
        .pass_common()
        .inputs(&[
            Input::Of(TWO_WAYS),
            Input::Of(&[0, 0x0080_0000]),
            Input::F32,
            Input::F32Swept(TWO),
            Input::F32Swept(0x4080_0000),
        ])
        .float_sweep(48)
        .setup(|e, c| {
            let p = pass_world(e, &[], 0, 0);
            let property = p.d.w.property;
            set(e, property + 0x20, c.word(0));
            set(e, property + 0x24, c.word(1));
            set(e, property + 0x34, bits(c, 2));
            set(e, 0x011f_9454, bits(c, 3));
            set(e, 0x011f_9458, bits(c, 4));
            vec![property]
        })
        .run()
        .assert_same();
}

/// A vertex shader with a renderer whose device is the world's, and a constant map entry of kind
/// `kind` (the table's index), with its own buffer (`own` 1), without (0) or flagged as having one with
/// none (2), `count` registers.
fn constant_entry(e: &mut Engine, kind: u32, own: u32, count: u32) -> (u32, u32) {
    let p = pass_world(e, &[], 0, 0);
    e.register(VT_FN + 0x180, slot_answer::<0x180>);
    e.register(VT_FN + 0x188, slot_answer::<0x188>);
    let vt = vtable(e, &[]);
    let renderer = e.mem.alloc(0x1100);
    set(e, renderer + 0x10f8, p.d.w.device);
    let shader = obj(e, 0x40, vt);
    set(e, shader + 0x28, renderer);
    for (k, value) in [0u32, 1, 3, 8, 9].iter().enumerate() {
        set(e, 0x011f_5fc4 + 4 * k as u32, *value);
    }
    let entry = e.mem.alloc(0x40);
    let top = if own != 0 { 0x2000_0000 } else { 0x1000_0000 };
    set(e, entry + 0x14, top | kind);
    set(e, entry + 0x1c, 7);
    set(e, entry + 0x20, count);
    set(e, entry + 0x28, 0x20);
    let buffer = e.mem.alloc(0x100);
    set(e, entry + 0x30, if own == 2 { 0 } else { buffer });
    for k in 0..0x40u32 {
        set(e, buffer + 4 * k, 0xb0f0_0000 + k);
    }
    // The device's answers: floats succeed, integers fail.
    set(e, RETS + 0x178, 0);
    set(e, RETS + 0x180, 0x8000_0005);
    (shader, entry)
}

/// `SetShaderConstant`: each kind (float, boolean, integer, the array kinds), with its own buffer or
/// not, no count or one given, the entry's data or other data.
#[test]
fn set_vertex_constant() {
    Diff::new(entry!(
        0x00e8_9ba0,
        dconstants::set_constant(Ptr<_>, Ptr<_>, u32, u32) -> bool
    ))
    .pass_common()
    .goes_by_address(&[0x00ec_44d0])
    .inputs(&[
        Input::Of(&[0, 1, 2, 3, 4]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 3]),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 2]),
    ])
    .setup(|e, c| {
        let (shader, entry) = constant_entry(e, c.word(0), c.word(1), c.word(2));
        let data = if c.word(3) != 0 {
            let d = e.mem.alloc(0x40);
            set(e, d, 0xda7a_0000);
            d
        } else {
            0
        };
        vec![shader, entry, data, c.word(4)]
    })
    .run()
    .assert_same();
}

/// `SetShaderConstantArray`: kinds other than 8 and 9 refused; with its own buffer or not, no, one or
/// three indices, one or four registers each.
#[test]
fn set_vertex_constant_array() {
    Diff::new(entry!(
        0x00e8_9c70,
        dconstants::set_constant_array(Ptr<_>, Ptr<_>, u32, u32, u32, u32) -> bool
    ))
    .pass_common()
    .goes_by_address(&[0x00ec_44d0])
    .inputs(&[
        Input::Of(&[0, 2, 3, 4]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 3]),
        Input::Of(&[1, 4]),
    ])
    .setup(|e, c| {
        let (shader, entry) = constant_entry(e, c.word(0), c.word(1), 0);
        let data = e.mem.alloc(0x400);
        for k in 0..0x100u32 {
            set(e, data + 4 * k, 0xda7a_0000 + k);
        }
        let indices = e.mem.alloc(8);
        for (k, i) in [2u16, 0, 3].iter().enumerate() {
            e.mem.set_u16(indices + 2 * k as u32, *i);
        }
        vec![shader, entry, data, c.word(2), c.word(3), indices]
    })
    .run()
    .assert_same();
}

/// A shader like the world's whose slots hold the shadow light shader's typed functions: 0x80
/// blending, 0x84 the alpha function, 0x88 the alpha test, 0x110 the end's states, 0xf0 the pass
/// choice, 0xf4 the begin's end, 0xf8 the pair textures, 0xfc the pass constants, 0x100 the render
/// states, 0x104 the states put back, 0x114 the end's states put back. Its counted pointers' sources
/// are counted objects; the pass table gives a copy of `p`'s pass for every type; the pass's constants target
/// (`00e7f7c0`) is stood in by a cache-like object.
fn typed_slot_shader(e: &mut Engine, p: &PassW) -> u32 {
    let vt = vtable(
        e,
        &[
            (0x80, 0x00be_1ff0),
            (0x84, 0x00be_20b0),
            (0x88, 0x00be_20e0),
            (0x110, 0x00be_0e50),
            (0xf0, 0x00b7_af80),
            (0xf4, 0x00be_1f90),
            (0xf8, 0x00b7_cfb0),
            (0xfc, 0x00b7_b450),
            (0x100, 0x00be_0eb0),
            (0x104, 0x00be_0ee0),
            (0x114, 0x00be_0e90),
        ],
    );
    let shader = obj(e, 0x90, vt);
    for off in (4..0x90).step_by(4) {
        let word = e.mem.u32(p.d.w.shader + off);
        set(e, shader + off, word);
    }
    let plain = vtable(e, &[]);
    for k in 0..8u32 {
        let o = obj(e, 0x10, plain);
        set(e, o + 4, 2);
        set(e, shader + 0x6c + 4 * k, o);
    }
    set(e, shader + 0x18, p.d.w.renderer);
    // The pass table gives another pass than the current one (`p`'s copy with a render state list of
    // its own), so the pass choice shows in what the later slots read.
    let other = e.mem.alloc(0x60);
    for k in (0..0x60).step_by(4) {
        let word = e.mem.u32(p.pass + k);
        set(e, other + k, word);
    }
    let list = e.mem.alloc(0x20);
    let node = e.mem.alloc(0xc);
    set(e, node, 0x89);
    set(e, node + 4, 7);
    set(e, list + 0x10, node);
    set(e, other + 0x30, list);
    for t in 0..0x2f0u32 {
        set(e, 0x011f_dff8 + 4 * t, other);
    }
    let target = obj(e, 0x1114, plain);
    for k in (4..0x1114).step_by(4) {
        let word = e.mem.u32(p.d.w.cache + k);
        set(e, target + k, word);
    }
    set(e, RETS + 0x7f8, target);
    e.register(0x00e7_f7c0, |e, _| ret(e.mem.u32(RETS + 0x7f8)));
    for addr in SETUP_CALLEES {
        e.register(*addr, marked);
    }
    entries(e);
    shader
}

/// `begin_pass` with the typed slots: each pass type group (its own begin, or the slots 0xfc, 0xf8,
/// 0x100), the pass before (none, this shader's), the restore owed or not: every slot call direct.
#[test]
fn begin_pass_typed_slots() {
    Diff::new(entry!(0x00b9_9390, batch::begin_pass(u32, u32)))
        .pass_common()
        .inputs(&[
            Input::Of(&[0x10, 0xca, 0xd1, 0xfc, 0xfd, 0x102, 0x250]),
            Input::Of(&[0, 1, 2]),
            Input::Of(TWO_WAYS),
        ])
        .setup(|e, c| {
            let p = pass_world(e, PASS_STATES, 0x0b, 0);
            let shader = typed_slot_shader(e, &p);
            let (pass_type, current) = match c.word(1) {
                0 => (0, 0),
                1 => (0x11, shader),
                _ => (0x10b, shader),
            };
            set(e, 0x011f_fe30, pass_type);
            set(e, 0x011f_fe2c, current);
            e.mem.set_u8(0x011a_d8ec, c.word(2) as u8);
            vec![c.word(0), shader]
        })
        .run()
        .assert_same();
}

/// `end_pass` with the typed slots (0x104, 0x114): the restore owed or not, a type that also sets a
/// sampler back.
#[test]
fn end_pass_typed_slots() {
    Diff::new(entry!(0x00b9_88e0, batch::end_pass()))
        .pass_common()
        .inputs(&[Input::Of(&[0x10, 0xfe, 0x230]), Input::Of(TWO_WAYS)])
        .setup(|e, c| {
            let p = pass_world(e, PASS_STATES, 0, 0);
            let shader = typed_slot_shader(e, &p);
            set(e, 0x011f_fe30, c.word(0));
            set(e, 0x011f_fe2c, shader);
            e.mem.set_u8(0x011a_d8ec, c.word(1) as u8);
            vec![]
        })
        .run()
        .assert_same();
}

/// `render_standard` with the typed per-geometry slots (0x80, 0x84, 0x88, 0x110): a pass with its
/// sort flag or without, the three flags, the restore owed or not.
#[test]
fn render_standard_typed_pass_slots() {
    Diff::new(entry!(
        0x00b9_8e80,
        batch::render_standard(Ptr<_>, u32, u32, u32)
    ))
    .pass_common()
    .inputs(&[
        Input::Of(&[0, 2]),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
    ])
    .setup(|e, c| {
        let p = pass_world(e, &[], 0, 0);
        let shader = typed_slot_shader(e, &p);
        let pass = p.d.w.passes[c.word(0) as usize];
        let geometry = e.mem.u32(pass);
        set(e, geometry + 0xc0, shader);
        e.mem.set_u8(0x011a_d8ec, c.word(4) as u8);
        vec![pass, c.word(1), c.word(2), c.word(3)]
    })
    .run()
    .assert_same();
}

/// The toggles set-up reaching the land toggles (types 0x1f7 to 0x22e) and the fog property: both
/// calls direct. The shader property a land one (bit 0x4000 of its +0x20) or not: only a land one keeps
/// the land toggles' fourth float (the toggles' third overwrites it otherwise).
#[test]
fn toggles_typed_land_and_fog() {
    Diff::new(entry!(
        0x00b7_95b0,
        geometry::toggles(Ptr<_>, u32, u32, Ptr<_>, u32, u32, Ptr<_>)
    ))
    .pass_common()
    .inputs(&[Input::Of(&[0x10, 0x1f7, 0x22e]), Input::Of(TWO_WAYS)])
    .setup(|e, c| {
        let p = pass_world(e, &[], 0, 0);
        entries(e);
        if c.word(1) != 0 {
            let flags = e.mem.u32(p.d.w.property + 0x20);
            set(e, p.d.w.property + 0x20, flags | 0x4000);
        }
        // Pixel entry 4 on.
        let entry = e.mem.u32(0x011f_ec30 + 4 * 4);
        e.mem.set_u8(entry + 8, 1);
        e.register(VT_FN + 0xec, slot_answer::<0xec>);
        e.register(VT_FN + 0x154, slot_answer::<0x154>);
        let w = p.d.w;
        let table = e.mem.alloc(4);
        set(e, w.property + 0xc4, table);
        let node = e.mem.alloc(0x140);
        set(e, node + 0x134, 0x0f06_0134);
        set(e, 0x011f_91c8, node);
        vec![
            w.shader,
            c.word(0),
            w.geometry,
            w.property,
            0x0f06_0000,
            0,
            w.alpha,
        ]
    })
    .run()
    .assert_same();
}

// ---- The list draws the world draw made through translations (the drawing thread's still-live list) ----

use engine_typed::draw::lists as dlists;

#[test]
fn own_list_draw() {
    Diff::new(entry!(0x00b9_9990, dlists::own_list_draw(Ptr<_>, u32)))
        .draw_common_at(0x00b9_9990)
        .inputs(&[Input::Of(&[0, 1, 2, 3, 4]), Input::Of(&[0, 1, 0x33])])
        .setup(|e, c| {
            let d = draw_world(e);
            let w = &d.w;
            let list = e.mem.alloc(0x14);
            match c.word(0) {
                // Empty: nothing drawn, nothing ended.
                0 => {}
                1 => list_at(e, list, &[w.passes[0]]),
                2 => list_at(e, list, &[w.passes[0], w.passes[1], w.skin_pass]),
                // A null pass stops the walk; a pass without a geometry is not drawn.
                3 => list_at(e, list, &[w.passes[2], 0, w.passes[1]]),
                _ => {
                    let bare = e.mem.alloc(8);
                    e.mem.set_u16(bare + 4, 0x10);
                    list_at(e, list, &[bare, w.passes[3]]);
                }
            }
            vec![list, c.word(1)]
        })
        .run()
        .assert_same();
}

/// The world of `render_batches`' cases (its test and its view check).
fn render_batches_world(e: &mut Engine, c: &Case) -> Vec<u32> {
    let d = draw_world(e);
    let w = &d.w;
    let acc = w.acc;
    set(e, RETS + 0xf80, acc);
    e.register(0x00b4_f5c0, |e, _| ret(e.mem.u32(RETS + 0xf80)));
    e.map(0x011a_9000, 0x1000);
    e.map(0x0120_1000, 0x1000);
    // Groups 0 to 3 (the map stand-in answers the type's low two bits). Group 0 (type 0x10): every list holds
    // two passes of its own type with the sort byte set, so each list's blend byte reaches the pass draw.
    // Group 1 (type 0x11): its lists 2 and 3 hold a pass of its own type with the sort byte set. The rest:
    // every other list holds passes of other types.
    let own_11 = e.mem.alloc(8);
    set(e, own_11, w.geometry);
    e.mem.set_u16(own_11 + 4, 0x11);
    e.mem.set_u8(own_11 + 6, 4);
    e.mem.set_u8(own_11 + 7, 1);
    for group in 0..4u32 {
        for list in 0..5u32 {
            let at = group_list(e, w, group, list);
            if group == 0 {
                list_at(e, at, &[w.passes[0], w.passes[1]]);
            } else if group == 1 && (list == 2 || list == 3) {
                list_at(e, at, &[own_11, w.passes[2]]);
            } else if (group + list) % 2 == 0 {
                let items = [
                    w.passes[(list % 4) as usize],
                    w.passes[((list + 1) % 4) as usize],
                ];
                list_at(e, at, &items);
            } else {
                list_at(e, at, &[]);
            }
        }
        set(e, e.mem.u32(w.br + 8) + group * 0x68 + 0x64, 2);
    }
    // The active list: empty, one type, or three (the inline head and two nodes).
    let head = w.br + 0x2c;
    match c.word(1) {
        0 => {
            e.mem.set_u16(head, 0);
            set(e, head + 4, 0);
        }
        1 => {
            e.mem.set_u16(head, 0x10);
            set(e, head + 4, 0);
        }
        3 => {
            let next = e.mem.alloc(8);
            e.mem.set_u16(head, 0);
            set(e, head + 4, next);
            e.mem.set_u16(next, 0x10);
            set(e, next + 4, 0);
        }
        _ => {
            let second = e.mem.alloc(8);
            let third_node = e.mem.alloc(8);
            e.mem.set_u16(head, 0x11);
            set(e, head + 4, second);
            e.mem.set_u16(second, 0x10);
            set(e, second + 4, third_node);
            e.mem.set_u16(third_node, 0x13);
            set(e, third_node + 4, 0);
        }
    }
    e.mem.set_u8(0x011f_94a6, c.word(2) as u8);
    e.mem.set_u8(0x011f_9421, c.word(3) as u8);
    // The accumulator's two bytes that make a shadow pass (odd types here) only clear its group.
    e.mem.set_u8(acc + 0x164, u8::from(c.word(4) == 1));
    e.mem.set_u8(acc + 0x165, u8::from(c.word(4) == 2));
    set(e, 0x011f_fa24, 2);
    set(e, 0x011f_f9e8, 1);
    let (first, last) = match c.word(0) {
        0 => (0, 0x1234_56ff),
        1 => (0x10, 0x11),
        _ => (0x12, 0x2f1),
    };
    vec![w.br, first, last, c.word(5)]
}

#[test]
fn render_batches() {
    Diff::new(entry!(
        0x00b9_9e20,
        dlists::render_batches(Ptr<_>, u32, u32, u32)
    ))
    .draw_common_at(0x00b9_9e20)
    // The accumulator getter is the render family's: stood in here.
    .goes_by_address(&[0x00b4_f5c0])
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        // The active list: empty, one type, three, or type 0 at the head with one after it.
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 0x5a]),
    ])
    .setup(render_batches_world)
    .run()
    .assert_same();
}

// ---- The two list draws against the drawing thread's view (`draw::detach`) ----

use engine_core::typed::diff::{memory_digest, Value};
use engine_typed::draw::detach;

/// `f` run on two engines `setup` makes alike for each case: one with the lists of the renderer `setup` names
/// (its second answer) detached into the drawing thread's view around the call, as the cut does, one with no
/// view. Both must answer the same and leave the same memory: a list word the draw read or wrote in place
/// would differ (it reads 0 there while the view holds it), and the close stops a test build on a held word
/// written in place.
fn list_draw_view_same(
    name: &str,
    f: AbiFn,
    cases: &[Vec<u32>],
    setup: impl Fn(&mut Engine, &Case) -> (Vec<u32>, u32),
) {
    let mut total = 0;
    for words in cases {
        let case = Case(words.iter().map(|w| Value::Word(*w)).collect());
        let mut plain = Engine::new();
        let (a, _) = setup(&mut plain, &case);
        let answer = f(&mut plain, &a);
        let mut viewed = Engine::new();
        let (b, renderer) = setup(&mut viewed, &case);
        assert_eq!(a, b, "{name} {words:x?}: the setups differ");
        let mut held = Vec::new();
        detach::list_words(&viewed, renderer, &mut held);
        total += held.len();
        detach::open(&mut viewed, &held);
        let viewed_answer = f(&mut viewed, &b);
        detach::close(&mut viewed);
        assert_eq!(
            answer, viewed_answer,
            "{name} {words:x?}: the answers differ"
        );
        assert_eq!(
            memory_digest(&plain),
            memory_digest(&viewed),
            "{name} {words:x?}: the memory differs"
        );
    }
    assert!(total > 0, "{name}: no case held a list word");
}

/// Every combination of `values` per argument.
fn product(values: &[&[u32]]) -> Vec<Vec<u32>> {
    values.iter().fold(vec![vec![]], |cases, vs| {
        cases
            .iter()
            .flat_map(|c| {
                vs.iter().map(move |v| {
                    let mut c = c.clone();
                    c.push(*v);
                    c
                })
            })
            .collect()
    })
}

/// The own list draw on the holder that keeps its own list (`draw_world`'s holder 1), its renderer's lists
/// detached.
#[test]
fn own_list_draw_with_the_lists_detached() {
    list_draw_view_same(
        "own_list_draw",
        entry!(0x00b9_9990, dlists::own_list_draw(Ptr<_>, u32)).1,
        &product(&[&[0, 1, 0x33]]),
        |e, c| {
            let d = draw_world(e);
            let holder = d.w.holders[1];
            (vec![holder + 0xc, c.word(0)], d.w.br)
        },
    );
}

/// `RenderBatches` on the world renderer, its lists detached, over its test's cases.
#[test]
fn render_batches_with_the_lists_detached() {
    list_draw_view_same(
        "render_batches",
        entry!(0x00b9_9e20, dlists::render_batches(Ptr<_>, u32, u32, u32)).1,
        &product(&[
            &[0, 1, 2],
            &[0, 1, 2, 3],
            TWO_WAYS,
            TWO_WAYS,
            &[0, 1, 2],
            &[0x5a],
        ]),
        |e, c| {
            let words = render_batches_world(e, c);
            let renderer = words[0];
            (words, renderer)
        },
    );
}

// ---- Group C: the render states' lock counters (the draw family's `group_c`) ----

/// The lock counters' inputs: the count given back; the counter's value before (none left, one,
/// several); the state's cached lock byte and value.
const RELEASE: &[Input] = &[
    Input::Of(&[0, 1, 3]),
    Input::Of(&[0, 1, 2, 5]),
    Input::Of(TWO_WAYS),
    Input::Of(TWO_WAYS),
];

/// A lock counter's words: its counter at `at` and the state `state`'s cache from the case.
fn release_words(e: &mut Engine, c: &Case, at: u32, state: u32) -> Vec<u32> {
    let w = world(e);
    set(e, at, c.word(1));
    cached_state(e, &w, state, c.word(2), c.word(3));
    vec![c.word(0)]
}

#[test]
fn release_z_enable() {
    Diff::new(entry!(0x0071_4a40, group_c::release_z_enable(u32)))
        .common()
        .inputs(RELEASE)
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| release_words(e, c, 0x011f_f9d8, 7))
        .run()
        .assert_same();
}

#[test]
fn release_z_write() {
    Diff::new(entry!(0x0071_4a60, group_c::release_z_write(u32)))
        .common()
        .inputs(RELEASE)
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| release_words(e, c, 0x011f_f9dc, 14))
        .run()
        .assert_same();
}

#[test]
fn release_alpha_test() {
    Diff::new(entry!(0x0071_4aa0, group_c::release_alpha_test(u32)))
        .common()
        .inputs(RELEASE)
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| release_words(e, c, 0x011f_f9e8, 15))
        .run()
        .assert_same();
}

#[test]
fn release_alpha_blend() {
    Diff::new(entry!(0x0071_4ae0, group_c::release_alpha_blend(u32)))
        .common()
        .inputs(RELEASE)
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| release_words(e, c, 0x011f_f9f0, 27))
        .run()
        .assert_same();
}

#[test]
fn release_color_write() {
    Diff::new(entry!(0x004e_ced0, group_c::release_color_write(u32)))
        .common()
        .inputs(RELEASE)
        .translation_calls_plainly(&[])
        .float_sweep(0)
        .setup(|e, c| release_words(e, c, 0x011f_fa18, 0xa8))
        .run()
        .assert_same();
}

// ---- Group C: the tall grass shader (the draw family's `tall_grass`) ----

/// Where the tall grass test keeps what its stand-ins answer: the shadow pass test, the active lights.
const TALL_GRASS_SWITCHES: u32 = RETS + 0x400;

/// The tall grass set-up's inputs: the transform's scale and first entry; the property's kind (4 has
/// the light work); a scene light or none; a shadow pass or not; the point lights on or off; the
/// property's active lights (none or two).
const TALL_GRASS: &[Input] = &[
    Input::F32,
    Input::F32Swept(ONE),
    Input::Of(&[4, 1]),
    Input::Of(TWO_WAYS),
    Input::Of(TWO_WAYS),
    Input::Of(TWO_WAYS),
    Input::Of(&[0, 2]),
];

/// `D3DXMatrixInverse`'s import (`00fdf428`) stood in: the matrix's words in reverse order.
fn matrix_inverse_stand_in(e: &mut Engine) {
    e.register(0x00fd_f428, |e, a| {
        for k in 0..16u32 {
            let word = e.mem.u32(a[2] + 4 * (15 - k));
            e.mem.set_u32(a[0] + 4 * k, word);
        }
        ret(a[0])
    });
}

/// The tall grass set-up's words: the world's renderer as the shader's, a transform of play's floats
/// (the scale and the first entry the case's, a small rotation, a translation), the state's property,
/// the accumulator's light list; the callees it reaches by address stood in (the shadow test, the
/// active lights' count, the point lights' and the shadow's set-ups, the base set-up answering 0x4242).
fn tall_grass_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    e.map(0x0120_0000, 0x1000);
    matrix_inverse_stand_in(e);
    e.register(0x00b6_30f0, |e, _| ret(e.mem.u32(TALL_GRASS_SWITCHES)));
    e.register(0x00b7_07d0, |e, _| ret(e.mem.u32(TALL_GRASS_SWITCHES + 4)));
    e.register(0x00ba_a720, |_, _| ret(0));
    e.register(0x00ba_a850, |_, _| ret(0));
    e.register(0x00e8_07d0, |_, _| ret(0x4242));
    set(e, TALL_GRASS_SWITCHES, c.word(4));
    set(e, TALL_GRASS_SWITCHES + 4, c.word(6));
    e.mem.set_u8(0x011f_9ef0, c.word(5) as u8);
    set(e, 0x011f_95ec, w.acc);
    set(e, 0x011f_91e4, 0x31);
    let vt = vtable(e, &[]);
    let this = obj(e, 0x1e0, vt);
    set(e, this + 0x18, w.renderer);
    let t = e.mem.alloc(0x34);
    let words = [
        play_bits(c, 1),
        0x3dcc_6c67,
        0,
        0xbdcc_6c67,
        ONE,
        0,
        0,
        0,
        ONE,
        0x40a0_0000,
        0x40c0_0000,
        0x40e0_0000,
        play_bits(c, 0),
    ];
    for (i, v) in (0u32..).zip(words) {
        set(e, t + 4 * i, v);
    }
    let state = e.mem.alloc(0x10);
    let property = obj(e, 0x20, vt);
    set(e, property + 0x1c, c.word(2));
    set(e, state + 0xc, property);
    let middle = e.mem.alloc(0xe4);
    let lights = e.mem.alloc(0xfc);
    let light = e.mem.alloc(0x100);
    set(e, w.acc + 0x194, middle);
    set(e, middle + 0xe0, lights);
    set(e, lights + 0xf8, if c.word(3) != 0 { light } else { 0 });
    for (k, v) in (0u32..).zip([0x3dcc_cccd, 0x3e4c_cccd, 0x3e99_999a]) {
        set(e, light + 0xc8 + 4 * k, v);
    }
    for (k, v) in (0u32..).zip([0x3ecc_cccd, HALF, 0x3f19_999a]) {
        set(e, light + 0xd4 + 4 * k, v);
    }
    for (k, v) in (0u32..).zip([ONE, 0xc000_0000, 0x4040_0000]) {
        set(e, light + 0xf0 + 4 * k, v);
    }
    vec![this, 1, 2, 3, 4, state, 6, t, 8]
}

/// `TallGrassShader::SetupGeometry` (`00baac10`): main-thread code (`f32`, the exe's 24-bit products
/// with play's floats), so on the main thread's word only, bit for bit (NaN payloads aside). The
/// shader family's functions it calls run by address ([`Common::shader_translated`]).
#[test]
fn tall_grass_setup_geometry() {
    Diff::new(entry!(
        0x00ba_ac10,
        engine_typed::draw::tall_grass::setup_geometry(
            Ptr<_>,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
        ) -> u32
    ))
    .shader_translated()
    // The unitize (the dispatch family's, its own tests): by address, its translation on both sides.
    .goes_by_address(&[0x004a_0c10])
    // The shader manager's accumulator getter is a read of `011f95ec`, done in place
    // (`packet::shader_accumulator`).
    .ignore_calls(&[0x00b4_f5c0])
    .main_thread(0)
    .inputs(TALL_GRASS)
    .translation_calls_plainly(&[])
    .float_sweep(FLOAT_SWEEP)
    .setup(tall_grass_words)
    .run()
    .assert_same();
}

/// The tall grass's slot 0x78's inputs: the pass type (the shadow passes 0x284 to 0x287 and either
/// side); the property's texture.
const TALL_GRASS_TEXTURES: &[Input] = &[
    Input::Of(&[0x283, 0x284, 0x286, 0x287, 0x288]),
    Input::Of(&[0, 0x7e57_0000]),
];

/// The tall grass's slot 0x78's words: the shader (its slot 0x108 stood in), the state's property
/// (its slot 0xb4 answering the case's texture), the current pass's two stages, the rendered pass's
/// first scene light with a shadow map (`GetTexture` stood in: its answer from its words).
fn tall_grass_textures_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let _ = world(e);
    e.register(VT_FN + 0xb4, slot_answer::<0xb4>);
    e.register(VT_FN + 0x108, |_, _| ret(0));
    e.register(0x004b_c320, |_, a| ret(a[0] ^ 0x5a5a_0000 ^ a[1]));
    set(e, RETS + 0xb4, c.word(1));
    set(e, 0x011f_91e4, c.word(0));
    let vt = vtable(e, &[]);
    let this = obj(e, 0x1e0, vt);
    let property = obj(e, 0x80, vt);
    let state = e.mem.alloc(0x10);
    set(e, state + 0xc, property);
    let pass = e.mem.alloc(0x60);
    let stages = e.mem.alloc(8);
    for k in 0..2u32 {
        let stage = e.mem.alloc(0x10);
        set(e, stage + 8, 0x0101_0101 * (k + 1));
        set(e, stages + 4 * k, stage);
    }
    set(e, pass + 0x24, stages);
    set(e, 0x0126_f74c, pass);
    let light = e.mem.alloc(0x110);
    set(e, light + 0x10c, 0x0d0d_0000);
    let lights = e.mem.alloc(4);
    set(e, lights, light);
    let record = e.mem.alloc(0x10);
    set(e, record + 0xc, lights);
    set(e, 0x011f_91e0, record);
    vec![this, state]
}

/// The tall grass shader's slot 0x78 (`00bac950`): no floats; the main thread's word.
#[test]
fn tall_grass_setup_textures() {
    Diff::new(entry!(
        0x00ba_c950,
        engine_typed::draw::tall_grass::setup_textures(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(TALL_GRASS_TEXTURES)
    .setup(tall_grass_textures_words)
    .run()
    .assert_same();
}

/// The instance copy's inputs: the instance count; the active count (all, one fewer, none); an
/// instance entry or none.
const TALL_GRASS_INSTANCES: &[Input] =
    &[Input::Of(&[3, 0x101]), Input::Of(&[0, 1, 2]), Input::Flag];

/// The tall grass's instance data at `data`: its geometry, `count` instances of a pattern, `active` of
/// them active, one instance's triangles 0x2ab (so the 16-bit product wraps for large counts), the
/// wind's speed `speed`; the geometry's model data has 0x1234 triangles and its slot 0xa4 stood in.
fn grass_instances(e: &mut Engine, geometry: u32, count: u32, active: u32, speed: u32) -> u32 {
    let vt = vtable(e, &[]);
    let model = obj(e, 0x60, vt);
    e.mem.set_u16(model + 0x40, 0x1234);
    set(e, model + 0x34, 0x0bbf_0000);
    set(e, geometry + 0xb8, model);
    let shape = e.mem.alloc(0x40);
    e.mem.set_u16(shape + 0x34, 0x2ab);
    let source = e.mem.alloc(count * 0x10);
    for i in 0..count * 4 {
        set(e, source + 4 * i, 0x1000_0000 + i);
    }
    let data = e.mem.alloc(0x40);
    set(e, data, geometry);
    set(e, data + 8, shape);
    e.mem.set_u16(data + 0x10, count as u16);
    e.mem.set_u16(data + 0x12, active as u16);
    set(e, data + 0x14, source);
    set(e, data + 0x34, speed);
    data
}

/// The instance copy's words: the shader with its instance array and entry (or none), the property's
/// instance data ([`grass_instances`]).
fn tall_grass_instances_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let _ = world(e);
    let count = c.word(0);
    let active = match c.word(1) {
        0 => count,
        1 => count - 1,
        _ => 0,
    };
    let vt = vtable(e, &[]);
    let this = obj(e, 0x1e0, vt);
    let array = e.mem.alloc(count * 0x10);
    set(e, this + 0x1a4, array);
    if c.word(2) != 0 {
        let entry = e.mem.alloc(0x30);
        set(e, this + 0x1a8, entry);
    }
    let geometry = e.mem.alloc(0xc0);
    let data = grass_instances(e, geometry, count, active, ONE);
    let property = obj(e, 0x80, vt);
    set(e, property + 0x7c, data);
    vec![this, 0x0bad_0001, property]
}

/// The tall grass's instance copy (`00baaf40`): no floats; the main thread's word. The copy is the CRT
/// family's `_memcpy` (its own tests), by address here.
#[test]
fn tall_grass_copy_instances() {
    Diff::new(entry!(
        0x00ba_af40,
        engine_typed::draw::tall_grass::copy_instances(Ptr<_>, u32, Ptr<_>)
    ))
    .common()
    .goes_by_address(&[0x00ec_44d0])
    .main_thread(0)
    .inputs(TALL_GRASS_INSTANCES)
    .setup(tall_grass_instances_words)
    .run()
    .assert_same();
}

/// The tall grass's slot 0x7c's inputs: its `SetupGeometry` slot the real one or a stand-in; full
/// bright; an alpha property or none; the game time and the wind's speed (play's floats: the phase
/// below or past the cycle, NaN, infinities); the property's bit 0x800; HDR; the shader level; the
/// shadows (on, skipped, without textures, the flag off) and their fade (0.5 truncates to 0, 1.5 to 1,
/// -1.5 to -1, a NaN and 2^64 to the indefinite, whose low word is 0); the constant maps (none,
/// stand-ins, empty real ones with one program).
const TALL_GRASS_CONSTANTS: &[Input] = &[
    Input::Flag,
    Input::Flag,
    Input::Flag,
    Input::F32,
    Input::F32Swept(0x4040_0000),
    Input::Flag,
    Input::Flag,
    Input::Of(&[1, 2, 3]),
    Input::Of(&[0, 1, 2, 3]),
    Input::Of(&[HALF, 0x3fc0_0000, 0xbfc0_0000, 0x7fc0_0000, 0x5f80_0000]),
    Input::Of(&[0, 1, 2]),
];

/// The tall grass's slot 0x7c's words: the world's renderer as the shader's; the rendered geometry
/// (a transform, a bound, a skin word, its model data's buffers) with a property of kind 1 (so the real
/// `SetupGeometry` stops after its up axis) whose instance data has the case's wind speed; the exe's
/// constants (255, 3600 and the cycle, `double`s); the frame's values from the case; the constant maps
/// as the case says (a real map is `NiD3DShaderConstantMap`'s `SetShaderConstants` with no entries, which
/// keeps the program).
fn tall_grass_constants_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    matrix_inverse_stand_in(e);
    e.map(0x0120_0000, 0x1000);
    e.map(0x0104_e000, 0x1000);
    for (addr, value) in [
        (0x0101_2640u32, 3600.0f64.to_bits()),
        (0x0104_eea0, 0x4019_21fa_0000_0000),
        (0x0101_e568, 255.0f64.to_bits()),
    ] {
        e.mem.set_u64(addr, value);
    }
    e.register(VT_FN + 0x60, |_, _| ret(0));
    e.register(0x00e8_07d0, |_, _| ret(0x4242));
    // `_CIfmod` (the CRT family's, its own tests) stands in: the exe's tables it reads through its
    // descriptor are not in the tests' memory. Both sides get `fmod` of the two doubles.
    e.register(0x00ec_913a, |_, a| {
        let double = |i: usize| f64::from_bits(u64::from(a[i]) | (u64::from(a[i + 1]) << 32));
        Ret {
            st0: double(0) % double(2),
            ..Ret::default()
        }
    });
    let shader_vt = if c.word(0) != 0 {
        vtable(e, &[(0x60, 0x00ba_ac10)])
    } else {
        vtable(e, &[])
    };
    let this = obj(e, 0x1e0, shader_vt);
    // The alpha row marked, so a word the set-up leaves shows.
    for k in 0..4u32 {
        set(e, this + 0x1d0 + 4 * k, 0x7777_0000 + k);
    }
    set(e, this + 0x18, w.renderer);
    let array = e.mem.alloc(0x30);
    set(e, this + 0x1a4, array);
    let entry = e.mem.alloc(0x30);
    set(e, this + 0x1a8, entry);
    // The rendered geometry: a transform (a small rotation, a translation, scale 2), a bound, a skin
    // word.
    let geometry = e.mem.alloc(0xc0);
    let words = [
        0x3f7e_8a6e,
        0xbdcc_6c67,
        0,
        0x3dcc_6c67,
        0x3f7e_8a6e,
        0,
        0,
        0,
        ONE,
        0x40a0_0000,
        0x40c0_0000,
        0x40e0_0000,
        TWO,
    ];
    for (i, v) in (0u32..).zip(words) {
        set(e, geometry + 0x68 + 4 * i, v);
    }
    let bound = e.mem.alloc(0x10);
    set(e, geometry + 0x20, bound);
    set(e, geometry + 0xbc, 0x5c1d_0000);
    let record = e.mem.alloc(0x10);
    set(e, record, geometry);
    set(e, 0x011f_91e0, record);
    set(e, 0x011f_91e4, 0x31);
    let data = grass_instances(e, geometry, 3, 2, play_bits(c, 4));
    let vt = vtable(e, &[]);
    let property = obj(e, 0x80, vt);
    set(e, property + 0x1c, 1);
    set(
        e,
        property + 0x24,
        if c.word(5) != 0 { 0x0800 } else { 0x0400 },
    );
    set(e, property + 0x7c, data);
    let state = e.mem.alloc(0x10);
    set(e, state + 0xc, property);
    if c.word(2) != 0 {
        let alpha = obj(e, 0x20, vt);
        e.mem.set_u16(alpha + 0x18, 0x1aed);
        e.mem.set_u8(alpha + 0x1a, 0x80);
        set(e, state, alpha);
    }
    // The frame's values.
    e.mem.set_u8(0x011f_91a7, c.word(1) as u8);
    for (k, v) in (0u32..).zip([0x3e80_0000, HALF, 0x3f40_0000, ONE]) {
        set(e, 0x011f_9ee0 + 4 * k, v);
        set(e, 0x011a_9bd0 + 4 * k, v ^ 0x0000_0101);
    }
    set(e, 0x011f_940c, play_bits(c, 3));
    e.mem.set_u8(0x011f_941e, c.word(6) as u8);
    set(e, 0x011f_9188, 0x3fa0_0000);
    set(e, 0x011f_91c0, c.word(7));
    let shadows = c.word(8);
    e.mem.set_u8(0x011f_9427, u8::from(shadows == 1));
    set(e, 0x011f_95e4, u32::from(shadows != 2));
    e.mem
        .set_u8(0x011f_91d8, if shadows == 3 { 0x1f } else { 0x3f });
    set(e, 0x011f_d890, c.word(9));
    for (k, at) in (0u32..).zip([0x011f_d874u32, 0x011f_d878, 0x011a_dce0, 0x011f_d880]) {
        set(e, at, 0x3d00_0000 + k);
    }
    // The current pass's programs and the shader's constant maps.
    let pass = e.mem.alloc(0x60);
    set(e, 0x0126_f74c, pass);
    match c.word(10) {
        0 => {}
        1 => {
            set(e, pass + 0x44, 0x0b0b_0044);
            set(e, pass + 0x5c, 0x0b0b_005c);
            for at in [0x30u32, 0x34] {
                let map = obj(e, 0x40, vt);
                set(e, this + at, map);
            }
        }
        _ => {
            set(e, pass + 0x44, 0x0b0b_0044);
            let map_vt = vtable(e, &[(0x78, 0x00e8_26d0)]);
            for at in [0x30u32, 0x34] {
                let map = obj(e, 0x40, map_vt);
                set(e, this + at, map);
            }
        }
    }
    vec![this, state]
}

/// The tall grass shader's slot 0x7c (`00bac9c0`): main-thread code (`f32`; the steps with a `double`
/// constant in `f64`, rounded once, and with the exe's constants, which have 24 significant bits or
/// fewer, that is one rounding as the x87's) on the main thread's word, bit for bit (NaN payloads
/// aside), with play's floats. Its `SetupGeometry`, the instance copy and the CRT's `_memcpy` run by
/// address here (their own tests; the instance copy is a plain call in the translation), and so do
/// the shader family's ([`Common::shader_translated`]); the CRT's `_CIfmod` stands in.
#[test]
fn tall_grass_setup_constants() {
    Diff::new(entry!(
        0x00ba_c9c0,
        engine_typed::draw::tall_grass::setup_constants(Ptr<_>, Ptr<_>)
    ))
    .shader_translated()
    .goes_by_address(&[0x00ba_ac10, 0x00ba_af40, 0x00ec_913a, 0x00ec_44d0])
    .translation_calls_plainly(&[0x00ba_af40])
    .main_thread(0)
    .inputs(TALL_GRASS_CONSTANTS)
    .float_sweep(FLOAT_SWEEP)
    // Without an alpha property the alpha function's read of its flags stops (the exe tests it for
    // null only for the alpha row): below shader level 2 or without HDR.
    .expect_stops(|c| c.word(2) == 0 && (c.word(7) < 2 || c.word(6) == 0))
    .setup(tall_grass_constants_words)
    .run()
    .assert_same();
}

// ---- Group C: the no-lighting shader (the draw family's `no_lighting`) ----

/// The no-lighting shader for these tests: its passes (+0x6c to +0xe0) and declarations (+0x184 to
/// +0x1b0) marked, each declaration a counted object; its entries +0x170, +0x174 and +0x178; the world's
/// renderer as its own (+0x18).
fn no_lighting_shader(e: &mut Engine, w: &W) -> u32 {
    let vt = vtable(e, &[]);
    let shader = obj(e, 0x1b4, vt);
    set(e, shader + 0x18, w.renderer);
    for at in (0x6cu32..=0xe0).step_by(4) {
        set(e, shader + at, 0x0a55_0000 + at);
    }
    for at in (0x184u32..=0x1b0).step_by(4) {
        let declaration = obj(e, 0x10, vt);
        set(e, declaration + 4, 1);
        set(e, shader + at, declaration);
    }
    for at in [0x170u32, 0x174, 0x178] {
        let entry = e.mem.alloc(0x30);
        e.mem.set_u8(entry + 8, 0x77);
        set(e, shader + at, entry);
    }
    shader
}

/// The no-lighting pass choice's pass types: each of the exe's jump tables' cases, the default inside
/// the second table, and either side of each range.
const NO_LIGHTING_PASSES: &[u32] = &[
    0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f, 0x50,
    0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x2db, 0x2dc, 0x2dd, 0x2de, 0x2e2, 0x2e3,
    0x2e4, 0x2e8, 0x2e9, 0x2ee, 0x2ef, 0x2f0,
];

/// The no-lighting shader's pass choice (`00bc6970`): every pass type of its tables, with the counted
/// pointer +0x28 empty, holding an object counted once (released and deleted when cleared) or twice.
#[test]
fn no_lighting_select_pass() {
    Diff::new(entry!(
        0x00bc_6970,
        engine_typed::draw::no_lighting::select_pass(Ptr<_>, u32)
    ))
    .common()
    .main_thread(0)
    .inputs(&[Input::Of(NO_LIGHTING_PASSES), Input::Of(&[0, 1, 2])])
    .max_cases(200)
    .setup(|e, c| {
        let w = world(e);
        let shader = no_lighting_shader(e, &w);
        if c.word(1) != 0 {
            let vt = vtable(e, &[]);
            let held = obj(e, 0x10, vt);
            set(e, held + 4, c.word(1));
            set(e, shader + 0x28, held);
        }
        vec![shader, c.word(0)]
    })
    .run()
    .assert_same();
}

/// The no-lighting fog's inputs: the pass type (around each switch's range); the fog property (none,
/// its distances both 0.0, the case's); its far and near distances (play's floats).
const NO_LIGHTING_FOG: &[Input] = &[
    Input::Of(&[0x49, 0x4a, 0x51, 0x52, 0x55, 0x56, 0x57, 0x58]),
    Input::Of(&[0, 1, 2]),
    Input::F32,
    Input::F32Swept(0x4120_0000),
];

/// The no-lighting fog's words: the shader; the scene's fog property (shadow scene node 2's, the fog
/// index 2) with a colour and a power, its distances as the case says; the default far distance and
/// the fog colour.
fn no_lighting_fog_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    e.map(0x0120_2000, 0x1000);
    e.map(0x010a_d000, 0x1000);
    let shader = no_lighting_shader(e, &w);
    let fog = if c.word(1) != 0 {
        let fog = e.mem.alloc(0x80);
        for (k, v) in (0u32..).zip([0x3e00_0000, 0x3e80_0000, 0x3f00_0000]) {
            set(e, fog + 0x20 + 4 * k, v);
        }
        set(e, fog + 0x60, 0x4040_0000);
        if c.word(1) == 2 {
            set(e, fog + 0x30, play_bits(c, 2));
            set(e, fog + 0x2c, play_bits(c, 3));
        }
        fog
    } else {
        0
    };
    let node = e.mem.alloc(0x140);
    set(e, node + 0x134, fog);
    set(e, 0x011f_91c8 + 8, node);
    e.mem.set_u8(0x011f_91c4, 2);
    set(e, 0x010a_d6e0, 0x4680_0000);
    for k in 0..4u32 {
        set(e, 0x011a_9bd0 + 4 * k, 0x3d00_0000 + k);
    }
    vec![shader, c.word(0)]
}

/// The no-lighting shader's slot 0xfc (`00bc6de0`): main-thread code (`f32`) on the main thread's
/// word, bit for bit (NaN payloads aside), with play's floats. The fog property's getter is the draw
/// family's (its own test), by address here.
#[test]
fn no_lighting_setup_fog() {
    Diff::new(entry!(
        0x00bc_6de0,
        engine_typed::draw::no_lighting::setup_fog(Ptr<_>, u32)
    ))
    .common()
    .main_thread(0)
    .inputs(NO_LIGHTING_FOG)
    .float_sweep(FLOAT_SWEEP)
    .setup(no_lighting_fog_words)
    .run()
    .assert_same();
}

/// The no-lighting shader's sub-texture copy (`00bc6fa0`): no, one or five entries. No floats; the
/// main thread's word. The copy is the CRT family's `_memcpy` (its own tests), by address here.
#[test]
fn no_lighting_copy_subtextures() {
    Diff::new(entry!(
        0x00bc_6fa0,
        engine_typed::draw::no_lighting::copy_subtextures(Ptr<_>, Ptr<_>, u32)
    ))
    .common()
    .goes_by_address(&[0x00ec_44d0])
    .main_thread(0)
    .inputs(&[Input::Of(&[0, 1, 5])])
    .setup(no_lighting_subtexture_words)
    .run()
    .assert_same();
}

/// The words of `no_lighting_copy_subtextures`'s cases (also its own-state check's).
fn no_lighting_subtexture_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    e.map(0x0120_2000, 0x1000);
    let shader = no_lighting_shader(e, &w);
    let count = c.word(0);
    let entries = e.mem.alloc(0x50);
    for i in 0..0x14u32 {
        set(e, entries + 4 * i, 0x5b00_0000 + i);
    }
    let data = e.mem.alloc(0x70);
    set(e, data + 0x68, entries);
    e.mem.set_u8(data + 0x6c, count as u8);
    let geometry = e.mem.alloc(0xc0);
    set(e, geometry + 0xb8, data);
    vec![shader, geometry, 0x0bad_0002]
}

/// The material constants' inputs: the property's alpha; the material's colour through its pointer or
/// its own; the colour (play's floats; green and blue below 0.1 unless swept); HDR; the scale; the
/// neutral colour equal to the colour as the exe leaves it or not.
const NO_LIGHTING_MATERIAL: &[Input] = &[
    Input::F32,
    Input::Flag,
    Input::F32,
    Input::F32Swept(0x3d4c_cccd),
    Input::F32Swept(0x3ca3_d70a),
    Input::Flag,
    Input::F32Swept(0x4000_0000),
    Input::Flag,
];

/// The material constants' words: the property's alpha, the material's colour (through its pointer or
/// its own) and scale, HDR, the exe's 0.1 (`0101ffa0`), the neutral colour (the colour as the exe
/// scales it, worked out in `f32`, or another) and the replacement colour.
fn no_lighting_material_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    e.map(0x0120_2000, 0x1000);
    e.mem.set_u64(0x0101_ffa0, 0x3fb9_9999_a000_0000);
    let shader = no_lighting_shader(e, &w);
    let vt = vtable(e, &[]);
    let property = obj(e, 0x80, vt);
    set(e, property + 0x28, play_bits(c, 0));
    let material = e.mem.alloc(0x48);
    let colour = [play_bits(c, 2), play_bits(c, 3), play_bits(c, 4)];
    let at = if c.word(1) != 0 {
        let own = e.mem.alloc(0x10);
        set(e, material + 0x34, own);
        own
    } else {
        material + 0x28
    };
    for (k, v) in (0u32..).zip(colour) {
        set(e, at + 4 * k, v);
    }
    let scale = play_bits(c, 6);
    set(e, material + 0x40, scale);
    let hdr = c.word(5) != 0;
    e.mem.set_u8(0x011f_941e, u8::from(hdr));
    let values = colour.map(f32::from_bits);
    let scaled = hdr || values.iter().all(|v| f64::from(0.1f32) > f64::from(*v));
    let left = if scaled {
        values.map(|v| v * f32::from_bits(scale))
    } else {
        values
    };
    for k in 0..3u32 {
        let neutral = if c.word(7) != 0 {
            left[k as usize].to_bits()
        } else {
            0x3f00_0000 + k
        };
        set(e, 0x011f_4998 + 4 * k, neutral);
        set(e, 0x011a_9b7c + 4 * k, 0x3e00_0000 + k);
    }
    vec![shader, property, material]
}

/// The no-lighting material constants (`00bc6fe0`): main-thread code (`f32`; the comparisons with the
/// exe's `double` 0.1 in `f64`, exact) on the main thread's word, bit for bit (NaN payloads aside), with
/// play's floats.
#[test]
fn no_lighting_material_constants() {
    Diff::new(entry!(
        0x00bc_6fe0,
        engine_typed::draw::no_lighting::material_constants(Ptr<_>, Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(NO_LIGHTING_MATERIAL)
    .float_sweep(FLOAT_SWEEP)
    .setup(no_lighting_material_words)
    .run()
    .assert_same();
}

/// The light fog's inputs: the pass type (either side of 0x2d8 to 0x2ef); the light's direction and
/// colour, the fog's offset and far value (play's floats).
const NO_LIGHTING_LIGHT_FOG: &[Input] = &[
    Input::Of(&[0x2d7, 0x2d8, 0x2e5, 0x2ef, 0x2f0]),
    Input::F32,
    Input::F32Swept(0x3f00_0000),
    Input::F32Swept(0xbf40_0000),
    Input::F32,
    Input::F32Swept(0x3f20_0000),
    Input::F32Swept(0x3e80_0000),
    Input::F32Swept(0x3dcc_cccd),
    Input::F32Swept(0x4680_0000),
];

/// The no-lighting light fog (`00bc7100`): main-thread code (`f32`, the dot product in the exe's order)
/// on the main thread's word, bit for bit (NaN payloads aside), with play's floats.
#[test]
fn no_lighting_light_fog() {
    Diff::new(entry!(
        0x00bc_7100,
        engine_typed::draw::no_lighting::light_fog(Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(NO_LIGHTING_LIGHT_FOG)
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let w = world(e);
        e.map(0x0120_2000, 0x1000);
        let shader = no_lighting_shader(e, &w);
        set(e, 0x011f_91e4, c.word(0));
        for k in 0..3u32 {
            set(e, 0x011f_a2a0 + 4 * k, play_bits(c, 1 + k as usize));
            set(e, 0x011f_9604 + 4 * k, play_bits(c, 4 + k as usize));
        }
        set(e, 0x011f_9610, play_bits(c, 7));
        set(e, 0x011f_9618, play_bits(c, 8));
        vec![shader]
    })
    .run()
    .assert_same();
}

/// The texture matrix's inputs: the state's texturing property (none, no first map, a map without a
/// transform, a transform as it is, a transform marked changed).
const NO_LIGHTING_TEXTURE_MATRIX: &[Input] = &[Input::Of(&[0, 1, 2, 3, 4])];

/// The no-lighting texture matrix (`00bc8e00`): no arithmetic; the main thread's word. The transform's
/// update (`00a88cd0`) is stood in: a mark in its matrix, the changed byte cleared.
#[test]
fn no_lighting_texture_matrix() {
    Diff::new(entry!(
        0x00bc_8e00,
        engine_typed::draw::no_lighting::texture_matrix(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(NO_LIGHTING_TEXTURE_MATRIX)
    .setup(|e, c| {
        let w = world(e);
        e.map(0x0120_1000, 0x2000);
        e.register(0x00a8_8cd0, |e, a| {
            e.mem.set_u32(a[0] + 0x20, 0x3f80_0001);
            e.mem.set_u8(a[0] + 0x1c, 0);
            ret(0)
        });
        let shader = no_lighting_shader(e, &w);
        let state = e.mem.alloc(0x18);
        let kind = c.word(0);
        if kind != 0 {
            let texturing = e.mem.alloc(0x30);
            let maps = e.mem.alloc(8);
            set(e, texturing + 0x20, maps);
            if kind >= 2 {
                let map = e.mem.alloc(0x10);
                set(e, maps, map);
                if kind >= 3 {
                    let transform = e.mem.alloc(0x50);
                    for i in 0..9u32 {
                        set(e, transform + 0x20 + 4 * i, 0x3e00_0000 + 0x10_0000 * i);
                    }
                    e.mem.set_u8(transform + 0x1c, u8::from(kind == 4));
                    set(e, map + 0xc, transform);
                }
            }
            set(e, state + 0x14, texturing);
        }
        vec![shader, state]
    })
    .run()
    .assert_same();
}

/// The no-lighting shader's slot 0x78 (`00bc8d40`): no stage 0, or a stage with the property's texture
/// (none or one) and its clamp mode (the property's slot 0xcc). No floats; the main thread's word. The
/// stage's address modes are the shader family's; its bind (the render family's) stands in, a mark of
/// its words.
#[test]
fn no_lighting_setup_textures() {
    Diff::new(entry!(
        0x00bc_8d40,
        engine_typed::draw::no_lighting::setup_textures(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(&[
        Input::Flag,
        Input::Of(&[0, 0x7e57_0000]),
        Input::Of(&[0, 1, 2]),
    ])
    .setup(|e, c| {
        let w = world(e);
        e.register(0x00e7_eb00, marked);
        e.register(VT_FN + 0xb4, slot_answer::<0xb4>);
        set(e, RETS + 0xb4, c.word(1));
        set(e, RETS + 0xcc, c.word(2));
        let shader = no_lighting_shader(e, &w);
        let vt = vtable(e, &[]);
        let property = obj(e, 0x80, vt);
        let state = e.mem.alloc(0x18);
        set(e, state + 0xc, property);
        let pass = e.mem.alloc(0x60);
        let stages = e.mem.alloc(8);
        set(e, stages, if c.word(0) != 0 { w.stage } else { 0 });
        set(e, pass + 0x24, stages);
        set(e, 0x0126_f74c, pass);
        vec![shader, state]
    })
    .run()
    .assert_same();
}

/// The no-lighting geometry constants' inputs: a skin or none; the pass type (the sub-textures' range,
/// the falloff's zero types, others); the property's flags +0x20 (bit 0x40, bit 31) and +0x24 (bit 0);
/// the alpha property (none, its flags field 0, 1, 2); the state's fifth property (none, flag 0 clear
/// or set); the constant maps (none, stand-ins).
const NO_LIGHTING_CONSTANTS: &[Input] = &[
    Input::Flag,
    Input::Of(&[0x42, 0x52, 0x56, 0x57, 0x2d8]),
    Input::Of(&[0, 0x40, 0x8000_0000, 0x8000_0040]),
    Input::Flag,
    Input::Of(&[0, 1, 2, 3]),
    Input::Of(&[0, 1, 2]),
    Input::Flag,
];

/// The no-lighting geometry constants' words: the shader; the rendered geometry (a transform, a bound,
/// a skin word, its model data's buffers and sub-textures); the state's properties as the case says
/// (the material with its own colour); the skinned origin; `NiTransform::MakeIdentity` stood in (an
/// identity into its frame).
fn no_lighting_constants_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    e.map(0x0120_1000, 0x2000);
    e.register(0x00a8_6bf0, |e, a| {
        for (i, v) in (0u32..).zip([ONE, 0, 0, 0, ONE, 0, 0, 0, ONE, 0, 0, 0, ONE]) {
            e.mem.set_u32(a[0] + 4 * i, v);
        }
        ret(a[0])
    });
    let shader = no_lighting_shader(e, &w);
    let vt = vtable(e, &[]);
    let target = obj(e, 0x10, vt);
    set(e, shader + 0x1c, target);
    let geometry = e.mem.alloc(0xc0);
    for (i, v) in (0u32..).zip([
        ONE,
        0,
        0,
        0,
        ONE,
        0,
        0,
        0,
        ONE,
        0x40a0_0000,
        0x40c0_0000,
        0x40e0_0000,
        ONE,
    ]) {
        set(e, geometry + 0x68 + 4 * i, v);
    }
    let bound = e.mem.alloc(0x10);
    set(e, geometry + 0x20, bound);
    if c.word(0) != 0 {
        set(e, geometry + 0xbc, 0x5c1d_0000);
    }
    let data = e.mem.alloc(0x70);
    set(e, data + 0x34, 0x0bbf_0000);
    let entries = e.mem.alloc(0x20);
    set(e, data + 0x68, entries);
    e.mem.set_u8(data + 0x6c, 2);
    set(e, geometry + 0xb8, data);
    let record = e.mem.alloc(0x10);
    set(e, record, geometry);
    set(e, 0x011f_91e0, record);
    set(e, 0x011f_91e4, c.word(1));
    for k in 0..3u32 {
        set(e, 0x011f_474c + 4 * k, 0x4100_0000 + 0x10_0000 * k);
    }
    let property = obj(e, 0x80, vt);
    set(e, property + 0x20, c.word(2));
    set(e, property + 0x24, c.word(3));
    set(e, property + 0x28, HALF);
    for k in 0..4u32 {
        set(e, property + 0x70 + 4 * k, 0x3f10_0000 + k);
    }
    let material = e.mem.alloc(0x48);
    for (k, v) in (0u32..).zip([HALF, 0x3e80_0000, ONE]) {
        set(e, material + 0x28 + 4 * k, v);
    }
    set(e, material + 0x40, ONE);
    let state = e.mem.alloc(0x18);
    set(e, state + 8, material);
    set(e, state + 0xc, property);
    if c.word(4) != 0 {
        let alpha = obj(e, 0x20, vt);
        let field = [0u16, 1, 2][c.word(4) as usize - 1];
        e.mem.set_u16(alpha + 0x18, field << 5);
        set(e, state, alpha);
    }
    if c.word(5) != 0 {
        let other = obj(e, 0x20, vt);
        e.mem.set_u16(other + 0x18, (c.word(5) - 1) as u16);
        set(e, state + 0x10, other);
    }
    let pass = e.mem.alloc(0x60);
    set(e, 0x0126_f74c, pass);
    if c.word(6) != 0 {
        set(e, pass + 0x44, 0x0b0b_0044);
        set(e, pass + 0x5c, 0x0b0b_005c);
        for at in [0x30u32, 0x34] {
            let map = obj(e, 0x40, vt);
            set(e, shader + at, map);
        }
    }
    vec![shader, state]
}

/// `BSShaderNoLighting::SetupGeometryConstants` (`00bc9a50`): the main thread's word, bit for bit (NaN
/// payloads aside). Its helpers (the sub-textures, the texture matrix, the light fog, the material
/// constants: their own tests above) are plain calls in the translation and run by address here; so do
/// `_memcpy` and the stage bind. So do the shader family's ([`Common::shader_translated`]).
#[test]
fn no_lighting_setup_constants() {
    Diff::new(entry!(
        0x00bc_9a50,
        engine_typed::draw::no_lighting::setup_constants(Ptr<_>, Ptr<_>)
    ))
    .shader_translated()
    .goes_by_address(&[
        0x00bc_6fa0,
        0x00bc_8e00,
        0x00bc_7100,
        0x00bc_6fe0,
        0x00ec_44d0,
    ])
    .translation_calls_plainly(&[0x00bc_6fa0, 0x00bc_8e00, 0x00bc_7100, 0x00bc_6fe0])
    .main_thread(0)
    .inputs(NO_LIGHTING_CONSTANTS)
    .setup(no_lighting_constants_words)
    .run()
    .assert_same();
}

// ---- Group C: the sky shader (the draw family's `sky`) ----

/// A second texture stage like the world's (`w.stage`): stage 1, the same stage cache.
fn second_stage(e: &mut Engine, w: &W) -> u32 {
    let stage = e.mem.alloc(0x14);
    set(e, stage + 4, 2);
    let cache = e.mem.u32(w.stage + 0x10);
    set(e, stage + 0x10, cache);
    stage
}

/// The sky's slot 0x78's inputs: the property's first and second textures (none or one); the pass type
/// (the six that take the first texture twice, and others); a stage 1 or none; the property's clamp
/// mode.
const SKY_TEXTURES: &[Input] = &[
    Input::Of(&[0, 0x7e57_0070]),
    Input::Of(&[0, 0x7e57_0080]),
    Input::Of(&[0x2d4, 0x23c, 0x23a, 0x2d2, 0x23d, 0x23e, 0x2d3, 0x30]),
    Input::Flag,
    Input::Of(&[0, 2]),
];

/// The sky shader's slot 0x78 (`00b89cc0`): no floats; the main thread's word. The stages' address
/// modes are the shader family's; their bind (the render family's) stands in, a mark of its words.
#[test]
fn sky_setup_textures() {
    Diff::new(entry!(
        0x00b8_9cc0,
        engine_typed::draw::sky::setup_textures(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(SKY_TEXTURES)
    .max_cases(200)
    .setup(|e, c| {
        let w = world(e);
        e.register(0x00e7_eb00, marked);
        set(e, RETS + 0xcc, c.word(4));
        set(e, 0x011f_91e4, c.word(2));
        let vt = vtable(e, &[]);
        let shader = obj(e, 0xf0, vt);
        let property = obj(e, 0x90, vt);
        set(e, property + 0x70, c.word(0));
        set(e, property + 0x80, c.word(1));
        let state = e.mem.alloc(0x18);
        set(e, state + 0xc, property);
        let pass = e.mem.alloc(0x60);
        let stages = e.mem.alloc(8);
        set(e, stages, w.stage);
        if c.word(3) != 0 {
            let stage = second_stage(e, &w);
            set(e, stages + 4, stage);
        }
        set(e, pass + 0x24, stages);
        set(e, 0x0126_f74c, pass);
        vec![shader, state]
    })
    .run()
    .assert_same();
}

/// The sky's slot 0x7c's inputs: the held accumulator skips type 13 or not; it has a camera or not; the
/// pass type (5 and another); the property's kind (every way of the exe's switch: 0 to 7, and 8 past
/// it); HDR; the sun's x and the origin's x, the sky's height (play's floats); the constant maps (none,
/// stand-ins); the state's fifth property (none, flag 0 clear or set); the geometry's x translation and
/// the held accumulator camera's x (play's floats, so the camera move's rounding order is compared).
const SKY_CONSTANTS: &[Input] = &[
    Input::Flag,
    Input::Flag,
    Input::Of(&[5, 6]),
    Input::Of(&[0, 1, 2, 3, 4, 5, 6, 7, 8]),
    Input::Flag,
    Input::F32,
    Input::F32Swept(0x4500_0000),
    Input::F32Swept(0x4396_0000),
    Input::Flag,
    Input::Of(&[0, 1, 2]),
    Input::F32Swept(0x4580_0000),
    Input::F32Swept(0x4480_0000),
];

/// The sky's slot 0x7c's words: the world's renderer as the shader's; the rendered geometry (a
/// transform with a translation, a bound, a skin word, its model data's buffers); the held accumulator
/// (`011f95f0`) with the case's byte +0x164 and camera; the scene camera (`011f95d8`); the sun, the origin,
/// the sky's height and the exe's 7.0; the property (its kind, value, blend index, four colours); the
/// blend values, the sky colours, the HDR value; the maps and the fifth property as the case says.
fn sky_constants_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    e.mem.set_u64(0x0101_5a40, 7.0f64.to_bits());
    let vt = vtable(e, &[]);
    let shader = obj(e, 0xf0, vt);
    set(e, shader + 0x18, w.renderer);
    let target = obj(e, 0x10, vt);
    set(e, shader + 0x1c, target);
    let geometry = e.mem.alloc(0xc0);
    for (i, v) in (0u32..).zip([
        ONE,
        0,
        0,
        0,
        ONE,
        0,
        0,
        0,
        ONE,
        play_bits(c, 10),
        0xc520_0000,
        0x4400_0000,
        ONE,
    ]) {
        set(e, geometry + 0x68 + 4 * i, v);
    }
    let bound = e.mem.alloc(0x10);
    set(e, geometry + 0x20, bound);
    set(e, geometry + 0xbc, 0x5c1d_0000);
    let data = e.mem.alloc(0x40);
    set(e, data + 0x34, 0x0bbf_0000);
    set(e, geometry + 0xb8, data);
    let record = e.mem.alloc(0x10);
    set(e, record, geometry);
    set(e, 0x011f_91e0, record);
    set(e, 0x011f_91e4, c.word(2));
    let acc = e.mem.alloc(0x1ac);
    e.mem.set_u8(acc + 0x164, c.word(0) as u8);
    if c.word(1) != 0 {
        let camera = e.mem.alloc(0xa0);
        for (k, v) in (0u32..).zip([play_bits(c, 11), 0x4300_0000, 0xc2c8_0000]) {
            set(e, camera + 0x8c + 4 * k, v);
        }
        set(e, acc + 8, camera);
    }
    set(e, 0x011f_95f0, acc);
    let scene_camera = e.mem.alloc(0xa0);
    for (k, v) in (0u32..).zip([0x4570_0000, 0xc500_0000, 0x4420_0000]) {
        set(e, scene_camera + 0x8c + 4 * k, v);
    }
    set(e, 0x011f_95d8, scene_camera);
    for (k, v) in (0u32..).zip([play_bits(c, 5), 0x4600_0000, 0x4680_0000]) {
        set(e, 0x011f_a2a0 + 4 * k, v);
    }
    for (k, v) in (0u32..).zip([play_bits(c, 6), 0x4500_0000, 0x4300_0000]) {
        set(e, 0x011f_474c + 4 * k, v);
    }
    set(e, 0x011f_f8a8, play_bits(c, 7));
    let property = obj(e, 0x90, vt);
    set(e, property + 0x8c, c.word(3));
    set(e, property + 0x84, 0x3f40_0000);
    e.mem.set_u16(property + 0x88, 2);
    for k in 0..4u32 {
        set(e, property + 0x60 + 4 * k, 0x3e00_0000 + k);
    }
    for k in 0..4u32 {
        set(e, 0x011f_f8b4 + 4 * k, 0x3d00_0000 + k);
    }
    for k in 0..12u32 {
        set(e, 0x011f_f8e0 + 4 * k, 0x3c00_0000 + k);
        set(e, 0x011f_f910 + 4 * k, 0x7777_0000 + k);
    }
    e.mem.set_u8(0x011f_941e, c.word(4) as u8);
    set(e, 0x011a_d87c, 0x3fc0_0000);
    let state = e.mem.alloc(0x18);
    set(e, state + 0xc, property);
    if c.word(9) != 0 {
        let other = obj(e, 0x20, vt);
        e.mem.set_u16(other + 0x18, (c.word(9) - 1) as u16);
        set(e, state + 0x10, other);
    }
    let pass = e.mem.alloc(0x60);
    set(e, 0x0126_f74c, pass);
    if c.word(8) != 0 {
        set(e, pass + 0x44, 0x0b0b_0044);
        set(e, pass + 0x5c, 0x0b0b_005c);
        for at in [0x30u32, 0x34] {
            let map = obj(e, 0x40, vt);
            set(e, shader + at, map);
        }
    }
    vec![shader, state]
}

/// The sky shader's slot 0x7c (`00b89d80`): main-thread code (`f32`; the height over the exe's 7.0 in
/// `f64`, rounded once: 7.0 has three significant bits, so one rounding as the x87's) on the main
/// thread's word, bit for bit (NaN payloads aside), with play's floats. The shader family's run by
/// address ([`Common::shader_translated`]).
#[test]
fn sky_setup_constants() {
    Diff::new(entry!(
        0x00b8_9d80,
        engine_typed::draw::sky::setup_constants(Ptr<_>, Ptr<_>)
    ))
    .shader_translated()
    // The held accumulator's getter is a read of `011f95f0`, done in place (`packet::held_accumulator`).
    .ignore_calls(&[0x00b4_f5d0])
    .main_thread(0)
    .inputs(SKY_CONSTANTS)
    .float_sweep(FLOAT_SWEEP)
    .setup(sky_constants_words)
    .run()
    .assert_same();
}

// ---- Group C: the water shader (the draw family's `water`) ----

/// A current pass with `count` stages (each 0x10 bytes, its texture word marked), at `0126f74c`.
fn pass_with_stages(e: &mut Engine, count: u32) -> u32 {
    let pass = e.mem.alloc(0x60);
    let stages = e.mem.alloc(4 * count);
    for k in 0..count {
        let stage = e.mem.alloc(0x10);
        set(e, stage + 8, 0x0101_0101 * (k + 1));
        set(e, stages + 4 * k, stage);
    }
    set(e, pass + 0x24, stages);
    set(e, 0x0126_f74c, pass);
    pass
}

/// The water's slot 0x78's inputs: the pass type (the two that only clear, another); the reflection and
/// its switch (+0x80); the second rendered texture and its switch (+0x81); the third texture (+0x138);
/// the fourth's switch (+0x60) and texture (+0x148); the fifth texture (+0x144) and its switch (+0x63).
const WATER_TEXTURES: &[Input] = &[
    Input::Of(&[0x288, 0x289, 0x30]),
    Input::Of(&[0, 0x7e57_013c]),
    Input::Flag,
    Input::Of(&[0, 0x7e57_943c]),
    Input::Flag,
    Input::Of(&[0, 0x7e57_0138]),
    Input::Flag,
    Input::Of(&[0, 0x7e57_0148]),
    Input::Of(&[0, 0x7e57_0144]),
    Input::Flag,
];

/// The water shader's slot 0x78 (`00b863a0`): no floats; the main thread's word. `GetTexture` stands
/// in: its answer from its words.
#[test]
fn water_setup_textures() {
    Diff::new(entry!(
        0x00b8_63a0,
        engine_typed::draw::water::setup_textures(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(WATER_TEXTURES)
    .max_cases(200)
    .setup(|e, c| {
        let _ = world(e);
        e.register(VT_FN + 0x108, |_, _| ret(0));
        e.register(0x004b_c320, |_, a| ret(a[0] ^ 0x5a5a_0000 ^ a[1]));
        set(e, 0x011f_91e4, c.word(0));
        set(e, 0x011f_9510, 0x0def_0000);
        set(e, 0x011f_943c, c.word(3));
        let vt = vtable(e, &[]);
        let this = obj(e, 0x100, vt);
        let property = obj(e, 0x150, vt);
        set(e, property + 0x13c, c.word(1));
        e.mem.set_u8(property + 0x80, c.word(2) as u8);
        e.mem.set_u8(property + 0x81, c.word(4) as u8);
        set(e, property + 0x138, c.word(5));
        e.mem.set_u8(property + 0x60, c.word(6) as u8);
        set(e, property + 0x148, c.word(7));
        set(e, property + 0x144, c.word(8));
        e.mem.set_u8(property + 0x63, c.word(9) as u8);
        let state = e.mem.alloc(0x18);
        set(e, state + 0xc, property);
        pass_with_stages(e, 5);
        vec![this, state]
    })
    .run()
    .assert_same();
}

/// The water's slot 0x7c's inputs: the pass type (the two stencil-only ones, the two with the third word
/// 0, another); the property's switch +0x60; a fog property or none; the property's +0x104 (play's float:
/// below 10, past it); the sun's x and a light's red (play's floats); the light list (none, one light,
/// a null light first, then two); the light count (a small one, one past 2^31); the constant maps (none,
/// stand-ins).
const WATER_CONSTANTS: &[Input] = &[
    Input::Of(&[0x288, 0x289, 0x29a, 0x2ab, 0x30]),
    Input::Flag,
    Input::Flag,
    Input::F32,
    Input::F32Swept(0x4500_0000),
    Input::F32Swept(0x3f40_0000),
    Input::Of(&[0, 1, 2]),
    Input::Of(&[3, 0x8000_0001]),
    Input::Flag,
];

/// The water's slot 0x7c's words: the world's renderer as the shader's; the rendered geometry; the
/// water property (its colours, values, stencil, shade, the case's +0x104 and switch, a light list);
/// the sun, the origin, the water system's values; the fog property of scene node 2; the exe's 10.0, 1.0
/// and 2^32; the stencil state setters stood in (each a mark of its words).
fn water_constants_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    e.map(0x0102_0000, 0x1000);
    e.map(0x0109_9000, 0x1000);
    e.mem.set_u64(0x0102_0758, 10.0f64.to_bits());
    e.mem.set_u64(0x0101_2070, 1.0f64.to_bits());
    set(e, 0x0109_9378, 0x4f80_0000);
    for addr in [0x00b9_8180u32, 0x00b9_80c0, 0x00b9_8230] {
        e.register(addr, marked);
    }
    let vt = vtable(e, &[]);
    let this = obj(e, 0x100, vt);
    set(e, this + 0x18, w.renderer);
    let geometry = e.mem.alloc(0xc0);
    for (i, v) in (0u32..).zip([
        ONE,
        0,
        0,
        0,
        ONE,
        0,
        0,
        0,
        ONE,
        0x4580_0000,
        0xc520_0000,
        0x4400_0000,
        ONE,
    ]) {
        set(e, geometry + 0x68 + 4 * i, v);
    }
    let bound = e.mem.alloc(0x10);
    set(e, geometry + 0x20, bound);
    let data = e.mem.alloc(0x40);
    set(e, data + 0x34, 0x0bbf_0000);
    set(e, geometry + 0xb8, data);
    let record = e.mem.alloc(0x10);
    set(e, record, geometry);
    set(e, 0x011f_91e0, record);
    set(e, 0x011f_91e4, c.word(0));
    for (k, v) in (0u32..).zip([play_bits(c, 4), 0x4600_0000, 0x4680_0000]) {
        set(e, 0x011f_a2a0 + 4 * k, v);
    }
    for (k, v) in (0u32..).zip([0x4500_0000, 0x4300_0000, 0x4200_0000]) {
        set(e, 0x011f_474c + 4 * k, v);
    }
    set(e, 0x011f_f108, 0x3e40_0000);
    for k in 0..8u32 {
        set(e, 0x011f_f398 + 4 * k, 0x3c80_0000 + k);
    }
    let property = obj(e, 0x150, vt);
    for at in (0x6cu32..0x130).step_by(4) {
        set(e, property + at, 0x3d00_0000 + at);
    }
    set(e, property + 0x104, play_bits(c, 3));
    set(e, property + 0x84, 0x55);
    e.mem.set_u8(property + 0x60, c.word(1) as u8);
    e.mem.set_u8(property + 0x82, 0x33);
    set(e, property + 0x130, c.word(7));
    let lights = match c.word(6) {
        0 => 0,
        kind => {
            let mut next = 0;
            let count = if kind == 1 { 1 } else { 3 };
            for n in 0..count {
                let node = e.mem.alloc(0xc);
                let light = if kind == 2 && n == count - 1 {
                    0
                } else {
                    let light = e.mem.alloc(0xe4);
                    for (k, offset) in (0u32..).zip([0x8cu32, 0x90, 0x94, 0xe0]) {
                        set(e, light + offset, 0x4100_0000 + k + 0x10 * n);
                    }
                    set(e, light + 0xc4, 0x3fc0_0000);
                    set(e, light + 0xd4, play_bits(c, 5));
                    set(e, light + 0xd8, 0x3f00_0000);
                    set(e, light + 0xdc, 0x3e80_0000);
                    light
                };
                set(e, node + 8, light);
                set(e, node, next);
                next = node;
            }
            next
        }
    };
    set(e, property + 0x128, lights);
    let state = e.mem.alloc(0x18);
    set(e, state + 0xc, property);
    let fog = if c.word(2) != 0 {
        let fog = e.mem.alloc(0x80);
        for (k, v) in (0u32..).zip([0x3e00_0000, 0x3e80_0000, 0x3f00_0000]) {
            set(e, fog + 0x20 + 4 * k, v);
        }
        set(e, fog + 0x2c, 0x4120_0000);
        set(e, fog + 0x30, 0x4500_0000);
        set(e, fog + 0x60, 0x4040_0000);
        fog
    } else {
        0
    };
    let node = e.mem.alloc(0x140);
    set(e, node + 0x134, fog);
    set(e, 0x011f_91c8 + 8, node);
    e.mem.set_u8(0x011f_91c4, 2);
    let pass = e.mem.alloc(0x60);
    set(e, 0x0126_f74c, pass);
    if c.word(8) != 0 {
        set(e, pass + 0x44, 0x0b0b_0044);
        set(e, pass + 0x5c, 0x0b0b_005c);
        for at in [0x30u32, 0x34] {
            let map = obj(e, 0x40, vt);
            set(e, this + at, map);
        }
    }
    vec![this, state]
}

/// The water shader's slot 0x7c (`00b864d0`): main-thread code (`f32`; the property's +0x104 over the
/// exe's 10.0 in `f64`, rounded once: 10.0 has three significant bits, so one rounding as the x87's; the
/// light count, an unsigned integer, rounded once) on the main thread's word, bit for bit (NaN payloads
/// aside), with play's floats. The fog property's getter (the draw family's) and the shader family's
/// functions run by address ([`Common::shader_translated`]).
#[test]
fn water_setup_constants() {
    Diff::new(entry!(
        0x00b8_64d0,
        engine_typed::draw::water::setup_constants(Ptr<_>, Ptr<_>)
    ))
    .shader_translated()
    .main_thread(0)
    .inputs(WATER_CONSTANTS)
    .float_sweep(FLOAT_SWEEP)
    .setup(water_constants_words)
    .run()
    .assert_same();
}

// ---- Group C: the SpeedTree leaf shader (the draw family's `speedtree`) ----

/// The leaf `SetupGeometry`'s inputs: the transform's scale and first entry (play's floats); the
/// state's shader property of kind 6, of another kind, or none.
const LEAF_GEOMETRY: &[Input] = &[Input::F32, Input::F32Swept(ONE), Input::Of(&[6, 5, 0])];

/// The leaf `SetupGeometry`'s words: the world's renderer as the shader's, a transform of play's floats
/// (the scale and the first entry the case's, a small rotation, a translation), the state's property;
/// the light set-up and the camera rows stood in (each a mark of its words), `D3DXMatrixInverse`'s import
/// stood in.
fn leaf_geometry_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    matrix_inverse_stand_in(e);
    e.register(0x00bb_1960, marked);
    e.register(0x00bb_1a90, marked);
    let vt = vtable(e, &[]);
    let this = obj(e, 0x38c, vt);
    set(e, this + 0x18, w.renderer);
    let t = e.mem.alloc(0x34);
    let words = [
        play_bits(c, 1),
        0x3dcc_6c67,
        0,
        0xbdcc_6c67,
        ONE,
        0,
        0,
        0,
        ONE,
        0x40a0_0000,
        0x40c0_0000,
        0x40e0_0000,
        play_bits(c, 0),
    ];
    for (i, v) in (0u32..).zip(words) {
        set(e, t + 4 * i, v);
    }
    let state = e.mem.alloc(0x18);
    if c.word(2) != 0 {
        let property = obj(e, 0x80, vt);
        set(e, property + 0x1c, c.word(2));
        set(e, state + 0xc, property);
    }
    vec![this, 1, 2, 3, 4, state, 6, t, 8]
}

/// The leaf shader's `SetupGeometry` (`00bb25d0`): main-thread code (`f32`, the exe's 24-bit products
/// with play's floats) on the main thread's word, bit for bit (NaN payloads aside). The shader family's
/// functions run by address ([`Common::shader_translated`]).
#[test]
fn leaf_setup_geometry() {
    Diff::new(entry!(
        0x00bb_25d0,
        engine_typed::draw::speedtree::setup_geometry(
            Ptr<_>,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
        ) -> u32
    ))
    .shader_translated()
    .main_thread(0)
    .inputs(LEAF_GEOMETRY)
    .float_sweep(FLOAT_SWEEP)
    .setup(leaf_geometry_words)
    .run()
    .assert_same();
}

/// The leaf slot 0x7c's inputs: its `SetupGeometry` slot the real one or a stand-in; the pass's scene
/// lights (none, two); full bright; shader mode 9 or another; the leaf data's +0x18 (play's float); the
/// constant maps (none, stand-ins).
const LEAF_CONSTANTS: &[Input] = &[
    Input::Flag,
    Input::Of(&[0, 2]),
    Input::Flag,
    Input::Of(&[9, 8]),
    Input::F32,
    Input::Flag,
];

/// The leaf slot 0x7c's words: the world's renderer as the shader's; the rendered pass (its light count
/// and lights) and geometry; the state's leaf property, whose slots 0x100 (the leaves, 0x300 bytes of a
/// pattern) and 0xfc (the leaf data) answer through the stand-ins; the leaf scales, the exe's float for
/// mode 9; the scene lights' set-up stood in (a mark of its words) and what the real `SetupGeometry`
/// calls stood in as in [`leaf_geometry_words`].
fn leaf_constants_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    e.map(0x0120_0000, 0x1000);
    e.map(0x0102_3000, 0x1000);
    matrix_inverse_stand_in(e);
    e.register(0x00bb_1960, marked);
    e.register(0x00bb_1a90, marked);
    e.register(0x00b7_1360, marked);
    e.register(VT_FN + 0x60, |_, _| ret(0));
    let shader_vt = if c.word(0) != 0 {
        vtable(e, &[(0x60, 0x00bb_25d0)])
    } else {
        vtable(e, &[])
    };
    let this = obj(e, 0x38c, shader_vt);
    set(e, this + 0x18, w.renderer);
    let geometry = e.mem.alloc(0xc0);
    for (i, v) in (0u32..).zip([
        ONE,
        0,
        0,
        0,
        ONE,
        0,
        0,
        0,
        ONE,
        0x4580_0000,
        0xc520_0000,
        0x4400_0000,
        ONE,
    ]) {
        set(e, geometry + 0x68 + 4 * i, v);
    }
    let bound = e.mem.alloc(0x10);
    set(e, geometry + 0x20, bound);
    set(e, geometry + 0xbc, 0x5c1d_0000);
    let data = e.mem.alloc(0x40);
    set(e, data + 0x34, 0x0bbf_0000);
    set(e, geometry + 0xb8, data);
    let record = e.mem.alloc(0x10);
    set(e, record, geometry);
    let count = c.word(1);
    e.mem.set_u8(record + 9, count as u8);
    let lights = e.mem.alloc(8);
    for k in 0..2u32 {
        set(e, lights + 4 * k, 0x1170_0000 + k);
    }
    set(e, record + 0xc, lights);
    set(e, 0x011f_91e0, record);
    set(e, 0x011f_91e4, 0x31);
    e.mem.set_u8(0x011f_91a7, c.word(2) as u8);
    set(e, 0x011f_91b8, c.word(3));
    set(e, 0x0102_31a8, 0x3f40_0000);
    set(e, 0x0120_0608, 0x4000_0000);
    set(e, 0x0120_060c, 0x3f00_0000);
    let vt = vtable(e, &[]);
    let property = obj(e, 0x80, vt);
    set(e, property + 0x1c, 6);
    set(e, property + 0x6c, 0x3f20_0000);
    let leaves = e.mem.alloc(0x300);
    for k in 0..0xc0u32 {
        set(e, leaves + 4 * k, 0x1eaf_0000 + k);
    }
    set(e, RETS + 0x100, leaves);
    let leaf_data = e.mem.alloc(0x20);
    for (k, v) in (0u32..).zip([
        0x3e00_0000,
        0x3e80_0000,
        0x3f00_0000,
        play_bits(c, 4),
        0x3fc0_0000,
    ]) {
        set(e, leaf_data + 0xc + 4 * k, v);
    }
    set(e, RETS + 0xfc, leaf_data);
    let state = e.mem.alloc(0x18);
    set(e, state + 0xc, property);
    let pass = e.mem.alloc(0x60);
    set(e, 0x0126_f74c, pass);
    if c.word(5) != 0 {
        set(e, pass + 0x44, 0x0b0b_0044);
        set(e, pass + 0x5c, 0x0b0b_005c);
        for at in [0x30u32, 0x34] {
            let map = obj(e, 0x40, vt);
            set(e, this + at, map);
        }
    }
    vec![this, state]
}

/// The leaf shader's slot 0x7c (`00bb2760`): main-thread code (`f32`) on the main thread's word, bit for
/// bit (NaN payloads aside), with play's floats. Its `SetupGeometry` and `_memcpy` run by address here
/// (their own tests), so do the shader family's ([`Common::shader_translated`]).
#[test]
fn leaf_setup_constants() {
    Diff::new(entry!(
        0x00bb_2760,
        engine_typed::draw::speedtree::setup_constants(Ptr<_>, Ptr<_>)
    ))
    .shader_translated()
    .goes_by_address(&[0x00bb_25d0, 0x00ec_44d0])
    .main_thread(0)
    .inputs(LEAF_CONSTANTS)
    .float_sweep(FLOAT_SWEEP)
    .setup(leaf_constants_words)
    .run()
    .assert_same();
}

/// The leaf shader's slot 0xfc (`00bb1b50`): main-thread code (`f32`) on the main thread's word, bit for
/// bit (NaN payloads aside), with play's floats: no fog property, or one with the case's distances. The
/// fog property's getter (the draw family's) runs by address.
#[test]
fn leaf_setup_fog() {
    Diff::new(entry!(
        0x00bb_1b50,
        engine_typed::draw::speedtree::setup_fog(Ptr<_>, u32)
    ))
    .common()
    .main_thread(0)
    .inputs(&[Input::Flag, Input::F32, Input::F32Swept(0x4120_0000)])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let _ = world(e);
        let vt = vtable(e, &[]);
        let this = obj(e, 0x38c, vt);
        let fog = if c.word(0) != 0 {
            let fog = e.mem.alloc(0x80);
            for (k, v) in (0u32..).zip([0x3e00_0000, 0x3e80_0000, 0x3f00_0000]) {
                set(e, fog + 0x20 + 4 * k, v);
            }
            set(e, fog + 0x30, play_bits(c, 1));
            set(e, fog + 0x2c, play_bits(c, 2));
            set(e, fog + 0x60, 0x4040_0000);
            fog
        } else {
            0
        };
        let node = e.mem.alloc(0x140);
        set(e, node + 0x134, fog);
        set(e, 0x011f_91c8 + 8, node);
        e.mem.set_u8(0x011f_91c4, 2);
        vec![this, 0x0bad_0003]
    })
    .run()
    .assert_same();
}

// ---- Group C: what the set-ups share (the draw family's `group_c`) ----

/// The pass types' flag bits (`00b815e0`): a pass type whose words have every pattern of the bits read;
/// each entry present or not. No floats; the main thread's word.
#[test]
fn pass_flags() {
    Diff::new(entry!(0x00b8_15e0, group_c::pass_flags(u32)))
        .common()
        .main_thread(0)
        .inputs(&[Input::Of(&[1, 2, 5]), Input::Of(&[0, 0x1ff, 0x0a5])])
        .setup(|e, c| {
            let _ = world(e);
            for pass in 0..8u32 {
                set(e, 0x011f_f248 + 4 * pass, 0x5a5a_5a5a ^ (pass * 0x2f));
                set(e, 0x011f_f120 + 4 * pass, 0xa5a5_a5a5 ^ (pass * 0x63));
            }
            for k in 0..9u32 {
                let entry = if c.word(1) & (1 << k) != 0 {
                    let entry = e.mem.alloc(0x10);
                    e.mem.set_u8(entry + 8, 0x77);
                    entry
                } else {
                    0
                };
                set(e, 0x011f_f4a0 + 4 * k, entry);
            }
            vec![c.word(0)]
        })
        .run()
        .assert_same();
}

/// A material's colour constant (`00c03180`): main-thread code (`f32`) on the main thread's word, bit
/// for bit (NaN payloads aside), with play's floats: its colour through its pointer or its own; HDR on or
/// off with the image space's scale and the material's.
#[test]
fn material_colour_constant() {
    Diff::new(entry!(0x00c0_3180, group_c::material_colour_constant(u32)))
        .common()
        .main_thread(0)
        .inputs(&[
            Input::Flag,
            Input::Flag,
            Input::F32,
            Input::F32Swept(0x3fc0_0000),
            Input::F32Swept(0x4000_0000),
        ])
        .float_sweep(FLOAT_SWEEP)
        .setup(|e, c| {
            let _ = world(e);
            let material = e.mem.alloc(0x48);
            let at = if c.word(0) != 0 {
                let own = e.mem.alloc(0x10);
                set(e, material + 0x34, own);
                own
            } else {
                material + 0x28
            };
            for (k, v) in (0u32..).zip([play_bits(c, 2), 0x3f00_0000, 0x3e80_0000]) {
                set(e, at + 4 * k, v);
            }
            set(e, material + 0x40, play_bits(c, 3));
            e.mem.set_u8(0x011f_941e, c.word(1) as u8);
            let data = e.mem.alloc(0x10);
            set(e, data + 0xc, play_bits(c, 4));
            let effect = e.mem.alloc(0x70);
            set(e, effect + 0x6c, data);
            let effects = e.mem.alloc(8);
            set(e, effects + 4, effect);
            let manager = e.mem.alloc(0x10);
            set(e, manager + 8, effects);
            set(e, 0x011f_91ac, manager);
            vec![material]
        })
        .run()
        .assert_same();
}

// ---- The value packet (the drawing thread's step 2: `draw::capture`, `draw::cut`) ----

use engine_core::typed::diff::memory_digest_beside_stack;
use engine_typed::draw::{capture, cut};

/// `f` run, for each case and both scribbles, on two engines `setup` makes alike (it answers the call's words and
/// the words `fill` takes from): one plain, one with the value packet open around the call, filled by `fill` with
/// what the cut takes and scribbled (POISON). Both must answer the same and leave the same memory beside the stack
/// for locals; every read made through the packet must have found its copy, and no scribbled byte may have been
/// written. Some case must read through the packet.
fn packet_same(
    name: &str,
    f: AbiFn,
    cases: &[Vec<u32>],
    setup: impl Fn(&mut Engine, &Case) -> (Vec<u32>, Vec<u32>),
    fill: impl Fn(&mut Engine, &[u32]),
) {
    packet_same_cases(name, f, &word_cases(cases), false, setup, fill);
}

/// Cases of words only.
fn word_cases(cases: &[Vec<u32>]) -> Vec<Case> {
    cases
        .iter()
        .map(|words| Case(words.iter().map(|w| Value::Word(*w)).collect()))
        .collect()
}

/// [`packet_same`] with whole cases (floats among the words). With `plain_open` the plain engine runs with the
/// packet open and filled as well, only not scribbled: for a set-up that hands helpers a guest-stack image of a
/// copy, so both engines lay out their stacks alike (stand-ins that note the stack addresses they are handed then
/// compare); the plain engine's answers equal the packet off's by the Diff tests.
fn packet_same_cases(
    name: &str,
    f: AbiFn,
    cases: &[Case],
    plain_open: bool,
    setup: impl Fn(&mut Engine, &Case) -> (Vec<u32>, Vec<u32>),
    fill: impl Fn(&mut Engine, &[u32]),
) {
    let mut reads = 0;
    for case in cases {
        let words = &case.0;
        for scribble in [capture::SCRIBBLE, capture::SCRIBBLE_OTHER] {
            let mut plain = Engine::new();
            let (a, plain_from) = setup(&mut plain, case);
            plain.with_stack(4, |_, _| ());
            if plain_open {
                assert!(capture::begin(&plain), "{name}: a packet was left open");
                fill(&mut plain, &plain_from);
            }
            let answer = f(&mut plain, &a);
            if plain_open {
                let report = capture::end(&mut plain);
                assert_eq!(report.misses, 0, "{name} {words:x?}: {report:?}");
            }
            let mut poisoned = Engine::new();
            let (b, from) = setup(&mut poisoned, case);
            poisoned.with_stack(4, |_, _| ());
            assert_eq!(a, b, "{name} {words:x?}: the setups differ");
            assert!(capture::begin(&poisoned), "{name}: a packet was left open");
            fill(&mut poisoned, &from);
            capture::poison(&mut poisoned, scribble);
            let poisoned_answer = f(&mut poisoned, &b);
            let report = capture::end(&mut poisoned);
            assert_eq!(
                answer, poisoned_answer,
                "{name} {words:x?} {scribble:#x}: the answers differ"
            );
            assert_eq!(
                memory_digest_beside_stack(&plain),
                memory_digest_beside_stack(&poisoned),
                "{name} {words:x?} {scribble:#x}: the memory differs"
            );
            assert_eq!(
                (report.misses, report.written),
                (0, 0),
                "{name} {words:x?} {scribble:#x}: {report:?}"
            );
            reads += report.reads;
        }
    }
    assert!(reads > 0, "{name}: no case read through the packet");
}

/// A camera of the size `NiCamera` has (0x114 bytes, as the cut takes it) with the test world's camera's words,
/// in its place in the accumulator.
fn full_camera(e: &mut Engine, w: &W) -> u32 {
    let camera = e.mem.alloc(0x114);
    for k in 0..0x90 / 4 {
        let word = e.mem.u32(w.camera + 4 * k);
        set(e, camera + 4 * k, word);
    }
    for k in 0x90 / 4..0x114 / 4 {
        set(e, camera + 4 * k, 0x3e80_0000 + k);
    }
    set(e, w.acc + 8, camera);
    camera
}

/// The alpha distance reads the camera's direction and the first pass's bound through the packet.
#[test]
fn alpha_distance_reads_the_packet() {
    packet_same(
        "alpha_distance",
        entry!(
            0x00b9_8c80,
            dalpha::alpha_distance(Ptr<_>, u32, u32, u32) -> f64
        )
        .1,
        &product(&[&[0, 1], &[1, 3], &[0, 1, 2], &[0, 1, 0x101]]),
        |e, c| {
            let d = draw_world(e);
            let group = c.word(0);
            alpha_state(e, &d, group, c.word(1), c.word(2), HALF);
            let camera = full_camera(e, &d.w);
            (
                vec![d.w.br, group, camera, c.word(3)],
                vec![camera, d.w.geometry],
            )
        },
        |e, w| {
            cut::take_camera(e, w[0]);
            cut::take_geometry_values(e, w[1]);
        },
    );
}

/// The per-geometry pass set-ups read the rendered geometry's world transform (its inverse and the world-view
/// constants take an image of the copy), its scale and the rendered pass's light count through the packet.
#[test]
fn pass_setups_read_the_packet() {
    for (name, f) in [
        (
            "setup_type_ca",
            entry!(0x00b7_ded0, setups::setup_type_ca(Ptr<_>, Ptr<_>)).1,
        ),
        (
            "setup_type_d1",
            entry!(0x00b7_dfe0, setups::setup_type_d1(Ptr<_>, Ptr<_>)).1,
        ),
        (
            "setup_type_102",
            entry!(0x00b7_e150, setups::setup_type_102(Ptr<_>, Ptr<_>)).1,
        ),
    ] {
        packet_same(
            name,
            f,
            &product(&[&[0, 0x7e00_00f4], &[0, 1, 0xff], &[ONE, 0x3fc0_0000, 0]]),
            |e, c| {
                let p = pass_world(e, &[], 0, 0);
                let state = setup_state(e, &p, c.word(0));
                let rendering = e.mem.u32(0x011f_91e0);
                e.mem.set_u8(rendering + 9, c.word(1) as u8);
                let geometry = e.mem.u32(rendering);
                set(e, geometry + 0x98, c.word(2));
                set(e, 0x011f_951c, 0x7e00_951c);
                (vec![p.d.w.shader, state], vec![rendering, geometry])
            },
            |e, w| {
                capture::take(e, w[0], 9, 0xa);
                cut::take_geometry_values(e, w[1]);
            },
        );
    }
}

/// A lighting property's fade reads its second flags and its distance through the packet.
#[test]
fn property_fade_reads_the_packet() {
    packet_same(
        "property_fade",
        entry!(0x00b6_6b80, setups::property_fade(Ptr<_>) -> f32).1,
        &product(&[
            &[0, 1],
            &[0, 0x0080_0000],
            &[ONE, 0x4100_0000, 0],
            &[TWO],
            &[0x4080_0000],
        ]),
        |e, c| {
            let p = pass_world(e, &[], 0, 0);
            let property = p.d.w.property;
            set(e, property + 0x20, c.word(0));
            set(e, property + 0x24, c.word(1));
            set(e, property + 0x34, c.word(2));
            set(e, 0x011f_9454, c.word(3));
            set(e, 0x011f_9458, c.word(4));
            (vec![property], vec![p.d.w.geometry])
        },
        |e, w| cut::take_geometry_values(e, w[0]),
    );
}

/// The world draw's first half with `NV_DRAW_THREAD=poison` (this thread's mode): the cut fills the packet from
/// the accumulator's lists (each listed geometry held) and scribbles it, and the half (the camera handed to the
/// scene renderer through an image of the copy, the render mode's draw) does what it does with the switch off.
/// The join gives the references back.
#[test]
fn the_first_half_reads_the_packet_the_cut_fills() {
    let f = entry!(0x00b6_ba20, drivers::world_draw_first(u32, Ptr<_>, u32)).1;
    let mut reads = 0;
    for case in product(&[&[0, 1], &[0, 1, 2, 3]]) {
        let setup = |e: &mut Engine| {
            let d = draw_world(e);
            acc_slots(e, d.w.acc, case[0] != 0);
            set(e, d.w.acc + 0x19c, 3);
            held(e, d.w.acc, case[1]);
            // The listed geometries are the scene graph's: counted once already.
            for g in [d.w.geometry, d.w.skinned] {
                set(e, g + 4, 1);
            }
            let camera = full_camera(e, &d.w);
            e.with_stack(4, |_, _| ());
            vec![camera, d.w.acc, 1]
        };
        let mut plain = Engine::new();
        let a = setup(&mut plain);
        let answer = f(&mut plain, &a);
        let mut poisoned = Engine::new();
        let b = setup(&mut poisoned);
        capture::set_thread_mode(Some(capture::Mode::Poison));
        let poisoned_answer = f(&mut poisoned, &b);
        cut::join(&mut poisoned);
        capture::set_thread_mode(None);
        let report = cut::last_report().expect("the cut opened a packet");
        assert_eq!(answer, poisoned_answer, "{case:x?}: the answers differ");
        assert_eq!(
            memory_digest_beside_stack(&plain),
            memory_digest_beside_stack(&poisoned),
            "{case:x?}: the memory differs"
        );
        assert_eq!(
            (report.misses, report.written),
            (0, 0),
            "{case:x?}: {report:?}"
        );
        assert!(report.held > 0, "{case:x?}: no geometry held: {report:?}");
        reads += report.reads;
    }
    assert!(reads > 0, "the first half read nothing through the packet");
}

/// A pass like the world's first, of a geometry of its own (the world's geometry copied, with its own bound),
/// counted once as the scene graph's geometries are.
fn own_geometry_pass(e: &mut Engine, d: &DrawW) -> u32 {
    let geometry = e.mem.alloc(0xc4);
    for k in 0..0xc4 / 4 {
        let word = e.mem.u32(d.w.geometry + 4 * k);
        set(e, geometry + 4 * k, word);
    }
    set(e, geometry + 4, 1);
    let bound = e.mem.alloc(0x10);
    let from = e.mem.u32(d.w.geometry + 0x20);
    for k in 0..4 {
        let word = e.mem.u32(from + 4 * k);
        set(e, bound + 4 * k, word);
    }
    set(e, geometry + 0x20, bound);
    let pass = e.mem.alloc(0x10);
    for k in 0..4 {
        let word = e.mem.u32(d.w.passes[0] + 4 * k);
        set(e, pass + 4 * k, word);
    }
    set(e, pass, geometry);
    pass
}

/// The whole world draw with `NV_DRAW_THREAD=poison`, the way the world pass runs it: render mode 0 (the
/// accumulator's own slots 0xa8 and 0xac), the first half then the second, with the cut's packet open from the
/// cut to the join. The lists hold pass groups through each renderer's pass map, register group and alpha holders,
/// sorted alpha groups at +0x78 and grouped alphas at +0x90, in one renderer or two; the second half copies the
/// water reflection (+0x38 set, a pass of the types 0x288..0x2ce listed) or not. Every read through the packet
/// must find what the cut took (a miss stops a POISON run), nothing scribbled may be written, and the draw must
/// leave the memory as it does with the switch off: so the cut's walk covers every list the draw renders.
#[test]
fn the_whole_draw_reads_the_packet_the_cut_fills() {
    let first = entry!(0x00b6_ba20, drivers::world_draw_first(u32, Ptr<_>, u32)).1;
    let second = entry!(0x00b6_b930, drivers::world_draw_second(u32, Ptr<_>, u32)).1;
    let mut reads = 0;
    // Each case twice: the own state kept in guest memory (the test world's shader classes are not listed
    // clean), and taken (`statics::set_forced`): then the draw's render states, shader statics and camera live in
    // the packet, scribbled in guest memory, and go back at the join.
    let cases = product(&[TWO_WAYS, &[1, 2], &[0, 1, 2], &[0, 5, 0xf]]);
    for (case, forced) in cases
        .into_iter()
        .flat_map(|c| [(c.clone(), false), (c, true)])
    {
        let setup = |e: &mut Engine| {
            let d = draw_world(e);
            let mut own = Vec::new();
            acc_slots(e, d.w.acc, true);
            set(e, d.w.acc + 0x19c, 0);
            held(e, d.w.acc, 1);
            for g in [d.w.geometry, d.w.skinned] {
                set(e, g + 4, 1);
            }
            e.mem.set_u8(d.w.acc + 0x38, case[0] as u8);
            set(e, d.w.acc + 0x18c, case[1]);
            active_lists(e, &d, case[2]);
            // The sorted alpha groups of both alpha groups (bits 2r, 2r + 1 of the case's word, as alpha_state),
            // each keeping its own list with a pass of a geometry of its own; and the grouped alphas (+0x90) each
            // a holder whose pass's geometry is its own too. A list the cut's walk left out would then leave a
            // geometry out of the packet, and the draw's read of it would miss.
            for group in 0..2 {
                alpha_state(e, &d, group, 3, case[3], HALF);
                for (r, br) in d.renderers().into_iter().enumerate() {
                    // Only the renderers the accumulator counts are drawn (and walked).
                    let drawn = (r as u32) < case[1];
                    let mut node = e.mem.u32(br + 0x78 + 0xc * group);
                    while node != 0 {
                        let g = e.mem.u32(node + 8);
                        e.mem.set_u8(g, 1);
                        e.mem.set_u16(g + 2, 1);
                        set(e, g + 4, 4);
                        let pass = own_geometry_pass(e, &d);
                        if drawn {
                            own.push(e.mem.u32(pass));
                        }
                        list_at(e, g + 0xc, &[pass]);
                        node = e.mem.u32(node);
                    }
                    let h = e.mem.alloc(0x20);
                    e.mem.set_u16(h + 2, 1);
                    set(e, h + 4, 4);
                    set(e, h + 8, br);
                    let pass = own_geometry_pass(e, &d);
                    if drawn {
                        own.push(e.mem.u32(pass));
                    }
                    list_at(e, h + 0xc, &[pass]);
                    set(e, br + 0x90 + 4 * group, h);
                }
            }
            set(e, 0x011f_9438, 0x4242);
            set(e, 0x011f_943c, 0x4343);
            let camera = full_camera(e, &d.w);
            e.with_stack(4, |_, _| ());
            (vec![camera, d.w.acc, 1], own)
        };
        let mut plain = Engine::new();
        let (a, _) = setup(&mut plain);
        let answer = (first(&mut plain, &a), second(&mut plain, &a));
        let mut poisoned = Engine::new();
        let (b, own) = setup(&mut poisoned);
        statics::set_forced(forced);
        capture::set_thread_mode(Some(capture::Mode::Poison));
        let poisoned_first = first(&mut poisoned, &b);
        // Between the halves the packet holds every listed geometry: the sorted groups' and the grouped alphas'
        // own included (counted once by the scene graph, once by the packet).
        for g in &own {
            assert_eq!(
                poisoned.mem.u32(g + 4),
                2,
                "{case:x?}: geometry {g:#x} not held by the cut"
            );
        }
        let poisoned_answer = (poisoned_first, second(&mut poisoned, &b));
        capture::set_thread_mode(None);
        statics::set_forced(false);
        let report = cut::last_report().expect("the cut opened a packet and the join closed it");
        assert!(
            !capture::is_open(&poisoned),
            "{case:x?}: the packet stayed open"
        );
        assert_eq!(answer, poisoned_answer, "{case:x?}: the answers differ");
        assert_eq!(
            memory_digest_beside_stack(&plain),
            memory_digest_beside_stack(&poisoned),
            "{case:x?}: the memory differs"
        );
        assert_eq!(
            (report.misses, report.written),
            (0, 0),
            "{case:x?}: {report:?}"
        );
        assert_eq!(
            report.owned > 0,
            forced,
            "{case:x?} forced {forced}: {report:?}"
        );
        assert_eq!(report.kept_main, 0, "{case:x?} forced {forced}: {report:?}");
        assert!(report.held > 0, "{case:x?}: no geometry held: {report:?}");
        reads += report.reads;
    }
    assert!(reads > 0, "the draw read nothing through the packet");
}

/// The alpha draw's next pass (`00b9af60`) reads the camera's direction and the passes' bounds through the packet:
/// scribbled, either would change the order of the sorted groups or which are within the limit.
#[test]
fn render_next_alpha_reads_the_packet() {
    packet_same(
        "render_next_alpha",
        entry!(
            0x00b9_af60,
            dalpha::render_next_alpha(Ptr<_>, u32, u32, u32, f32)
        )
        .1,
        &product(&[
            TWO_WAYS,
            &[1, 3],
            &[1, 2, 0xf],
            &[0, 1, 0x101],
            &[HALF, ONE, TWO, 0x3eaa_aaab],
        ]),
        |e, c| {
            let d = draw_world(e);
            let group = c.word(0);
            alpha_state(e, &d, group, c.word(1), c.word(2), HALF);
            let camera = full_camera(e, &d.w);
            (
                vec![d.w.br, group, camera, c.word(3), c.word(4)],
                vec![camera, d.w.geometry],
            )
        },
        |e, w| {
            cut::take_camera(e, w[0]);
            cut::take_geometry_values(e, w[1]);
        },
    );
}

/// The pass constants with vertex entry 8 on (the fog constants and the fog colour's copies): the fog index and
/// the fog property's distances, power and colour come through the packet the cut's frame values fill, in both
/// branches (no distances: the far default and the fog colour; distances: far, span, power and the property's
/// colour).
#[test]
fn setup_pass_constants_reads_the_fog_through_the_packet() {
    packet_same(
        "setup_pass_constants",
        entry!(0x00b7_b450, setups::setup_pass_constants(Ptr<_>, u32)).1,
        &product(&[
            &[0x10, 0x2f0],
            &[0x0000_0200, 0xffff_ffff],
            &[1, 2, 3],
            TWO_WAYS,
        ]),
        |e, c| {
            let p = pass_world(e, &[], 0, 0);
            for addr in SETUP_CALLEES {
                e.register(*addr, marked);
            }
            entries(e);
            let pass_type = c.word(0);
            set(e, 0x011f_cc80 + 4 * pass_type, c.word(1));
            set(e, 0x011f_c0a0 + 4 * pass_type, c.word(1).rotate_left(3));
            e.map(0x010a_d000, 0x1000);
            set(e, 0x010a_d6e0, 0x48f4_2400);
            e.mem.set_u8(0x011f_91c4, 1);
            let node = e.mem.alloc(0x140);
            set(e, 0x011f_91c8 + 4, node);
            let fog = e.mem.alloc(0x64);
            for (k, word) in [ONE, HALF, 0x3e80_0000].iter().enumerate() {
                set(e, fog + 0x20 + 4 * k as u32, *word);
            }
            match c.word(2) {
                2 => {
                    set(e, fog + 0x2c, 0x3f40_0000);
                    set(e, fog + 0x30, TWO);
                    set(e, fog + 0x60, 0x3f40_0000);
                }
                3 => set(e, fog + 0x2c, ONE),
                _ => {}
            }
            set(e, node + 0x134, fog);
            let geometry = e.mem.u32(e.mem.u32(0x011f_91e0));
            let skin = if c.word(3) != 0 { p.d.w.skin } else { 0 };
            set(e, geometry + 0xbc, skin);
            set(e, p.d.w.shader + 0x18, p.d.w.renderer);
            (vec![p.d.w.shader, pass_type], vec![p.d.w.camera, p.d.w.acc])
        },
        |e, w| cut::take_frame(e, w[0], Ptr::new(w[1])),
    );
}

/// The geometry world of the light checks: two lights (the first a point light with `points` 1), the pass type,
/// the switches of the light set-up (HDR, skip-ambient, no-extra-scale, the light's ambient byte).
fn light_check_world(e: &mut Engine, points: u32, pass_type: u32, switches: u32) -> Geo {
    let mut s = Shape::plain();
    s.lights = 2;
    s.points = points;
    s.pass_type = pass_type;
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
    geometry_shape(e, &s)
}

/// The per-light set-up (`00b70820`) reads the pass light (+0xd0, +0xd4, +0xf4, +0xf5, +0xf8) and its light
/// (position, values to +0xe8, direction) through the packet the cut fills (`cut::take_pass_lights`): with
/// them scribbled in guest memory it sets what it sets without a packet.
#[test]
fn light_setup_reads_the_pass_lights_through_the_packet() {
    packet_same(
        "light_setup",
        entry!(0x00b7_0820, lighting::light_setup(u32, Ptr<_>, f32)).1,
        &product(&[&[0, 1, 7], &[0, 1], &[0, 1, 2, 9], &[0x154, 0x30]]),
        |e, c| {
            let g = light_check_world(e, c.word(1), c.word(3), c.word(2));
            (vec![c.word(0), g.light, HALF], vec![g.rendering])
        },
        |e, w| cut::take_pass_lights(e, w[0]),
    );
}

/// The geometry's lights (`00b78a90`, which runs the per-light set-up for each pass light) read the pass's
/// light count, array and lights through the packet.
#[test]
fn lights_read_the_pass_lights_through_the_packet() {
    packet_same(
        "lights",
        geometry::lights_words as AbiFn,
        &product(&[&[0, 1, 3], &[0, 0x10, 0x30], &[0, 1]]),
        |e, c| {
            let g = light_check_world(e, c.word(0), c.word(1), c.word(2));
            let mut words = vec![g.property, g.rendering];
            words.extend(matrix_words(&g));
            words.extend([g.transform(), g.pass_type, g.skin]);
            (words, vec![g.rendering])
        },
        |e, w| cut::take_pass_lights(e, w[0]),
    );
}

/// Vertex entries 15 and 21 (`00b790d0`, `00b791f0`, the fade of pass type 0x230) read the shaders' game time
/// and start time through the packet the cut's frame values fill.
#[test]
fn vertex_entries_read_the_times_through_the_packet() {
    for (name, f) in [
        (
            "vertex_entry_15",
            entry!(0x00b7_90d0, geometry::vertex_entry_15(Ptr<_>, Ptr<_>)).1,
        ),
        (
            "vertex_entry_21",
            entry!(
                0x00b7_91f0,
                geometry::vertex_entry_21(Ptr<_>, u32, Ptr<_>, Ptr<_>)
            )
            .1,
        ),
    ] {
        packet_same(
            name,
            f,
            &product(&[&[0x230, 0xfe], &[ONE, 0x4120_0000]]),
            |e, c| {
                let g = light_check_world(e, 0, c.word(0), 0);
                set(e, 0x011f_940c, c.word(1));
                set(e, 0x011f_94ac, HALF);
                let words = if name == "vertex_entry_15" {
                    vec![g.shader, g.property]
                } else {
                    vec![g.shader, g.pass_type, g.property, g.geometry]
                };
                let camera = e.mem.alloc(0x120);
                let acc = e.mem.alloc(0x40);
                (words, vec![camera, acc, g.geometry])
            },
            |e, w| {
                cut::take_frame(e, w[0], Ptr::new(w[1]));
                // The geometry's values (vertex entry 21 reads its translation through the packet: value packet B).
                cut::take_geometry_values(e, w[2]);
            },
        );
    }
}

/// The shadow lights' geometry world: two lights, the pass type, the light's byte +0xed and its made clip planes
/// as the case says.
fn shadow_check_world(e: &mut Engine, pass_type: u32, bits: u32) -> Geo {
    let mut s = Shape::plain();
    s.lights = 2;
    s.pass_type = pass_type;
    s.bits |= bits;
    geometry_shape(e, &s)
}

/// The cube projection (`00b7b730`) reads the pass's light array and first light, the light's byte +0xed and its
/// matrix (+0x10) through the packet the cut fills (`cut::take_pass_lights`).
#[test]
fn obj_to_cube_reads_the_pass_lights_through_the_packet() {
    packet_same(
        "obj_to_cube",
        entry!(
            0x00b7_b730,
            geometry::obj_to_cube(Ptr<_>, Ptr<_>, u32, u32, u32)
        )
        .1,
        &product(&[&[0, 0x30, 0x244, 0xfe], &[0, LIGHT_ED], &[0, 1]]),
        |e, c| {
            let g = shadow_check_world(e, c.word(0), c.word(1));
            (
                vec![g.shader, g.rendering, g.pass_type, c.word(2), g.transform()],
                vec![g.rendering],
            )
        },
        |e, w| cut::take_pass_lights(e, w[0]),
    );
}

/// The simple shadow's clip planes (`00b7be00`) read the pass's first light and its six planes (+0x144) through
/// the packet; the light's clip-plane cache (+0x1a8, made +0x208) is left in place (piece 4).
#[test]
fn simple_shadow_clip_planes_read_the_pass_lights_through_the_packet() {
    packet_same(
        "simple_shadow_clip_planes",
        entry!(
            0x00b7_be00,
            geometry::simple_shadow_clip_planes(Ptr<_>, u32)
        )
        .1,
        &product(&[&[0x231, 0x234, 0x230], &[0, CLIP_CACHED]]),
        |e, c| {
            let g = shadow_check_world(e, c.word(0), c.word(1));
            (vec![g.shader, g.pass_type], vec![g.rendering])
        },
        |e, w| cut::take_pass_lights(e, w[0]),
    );
}

// ---- Group C: the lighting 3.0 shader (the draw family's `lighting30`) ----

/// The lighting 3.0 shader's constant entries (`01201080` to `012010e8`, the pointers made at load):
/// each an entry whose switch (+8) is `on` (a pattern of the case), its count word +0x20 marked.
fn lighting30_entries(e: &mut Engine, on: u32) {
    e.map(0x0120_0000, 0x3000);
    for k in 0..27u32 {
        let entry = e.mem.alloc(0x30);
        e.mem.set_u8(entry + 8, u8::from(on & (1 << (k % 32)) != 0));
        set(e, entry + 0x20, 0x77);
        set(e, 0x0120_1080 + 4 * k, entry);
    }
}

/// A lighting property for these tests: its words +0x6c to +0xec marked (floats), its flags +0x20 and
/// +0x24 the given ones, its alpha +0x28 and +0x30.
fn lighting30_property(e: &mut Engine, flags_20: u32, flags_24: u32, alpha: u32) -> u32 {
    let vt = vtable(e, &[]);
    let property = obj(e, 0x100, vt);
    for at in (0x6cu32..0xf0).step_by(4) {
        set(e, property + at, 0x3d00_0000 + 0x1000 * at);
    }
    set(e, property + 0x20, flags_20);
    set(e, property + 0x24, flags_24);
    set(e, property + 0x28, alpha);
    set(e, property + 0x30, 0x3f40_0000);
    property
}

/// The lighting 3.0 shader's slot 0x78 (`00bbe650`): each pass type that returns early and others; its
/// slots 0x13c, 0x140 and 0x10c stood in. No floats; the main thread's word.
#[test]
fn lighting30_setup_textures() {
    Diff::new(entry!(
        0x00bb_e650,
        engine_typed::draw::lighting30::setup_textures(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(&[Input::Of(&[0x1e0, 0x1e1, 0xe, 0xf, 0x10, 0x1c4])])
    .setup(|e, c| {
        let _ = world(e);
        for slot in [0x13cu32, 0x140, 0x10c] {
            e.register(VT_FN + slot, marked);
        }
        set(e, 0x011f_91e4, c.word(0));
        let vt = vtable(e, &[]);
        let this = obj(e, 0xa4, vt);
        let property = lighting30_property(e, 0, 0, HALF);
        let state = e.mem.alloc(0x18);
        set(e, state + 0xc, property);
        vec![this, state]
    })
    .run()
    .assert_same();
}

/// The lighting 3.0 helpers' inputs: the entries' switches (all on, all off, a pattern); the
/// property's bit 0x10 of +0x24; the slot 0xf4's answer.
const LIGHTING30_SMALL: &[Input] = &[
    Input::Of(&[0, u32::MAX, 0x0555_5555]),
    Input::Of(&[0, 0x10]),
    Input::Of(&[0, 0x7e57_0000]),
];

/// The small lighting 3.0 helpers' words: the entries, a property, the exe's glow scale, the slot 0xf4
/// stood in (its answer the case's).
fn lighting30_small_words(e: &mut Engine, c: &Case) -> (u32, u32) {
    let _ = world(e);
    lighting30_entries(e, c.word(0));
    set(e, 0x0101_77e8, 0x3fa0_0000);
    set(e, RETS + 0xf4, c.word(2));
    let vt = vtable(e, &[]);
    let this = obj(e, 0xa4, vt);
    let property = lighting30_property(e, 0, c.word(1), HALF);
    (this, property)
}

/// The emittance (`00bbbda0`): no floats; the main thread's word.
#[test]
fn lighting30_emittance() {
    Diff::new(entry!(
        0x00bb_bda0,
        engine_typed::draw::lighting30::emittance(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(LIGHTING30_SMALL)
    .setup(|e, c| {
        let (this, property) = lighting30_small_words(e, c);
        vec![this, property]
    })
    .run()
    .assert_same();
}

/// The glow (`00bbbdf0`): no arithmetic; the main thread's word.
#[test]
fn lighting30_glow() {
    Diff::new(entry!(
        0x00bb_bdf0,
        engine_typed::draw::lighting30::glow(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(LIGHTING30_SMALL)
    .setup(|e, c| {
        let (this, property) = lighting30_small_words(e, c);
        vec![this, property]
    })
    .run()
    .assert_same();
}

/// The alpha (`00bbbe30`): no arithmetic; the main thread's word.
#[test]
fn lighting30_alpha() {
    Diff::new(entry!(
        0x00bb_be30,
        engine_typed::draw::lighting30::alpha(Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(LIGHTING30_SMALL)
    .setup(|e, c| {
        let (_, property) = lighting30_small_words(e, c);
        vec![property]
    })
    .run()
    .assert_same();
}

/// The scene camera's position for these tests (the shadow scene node 0's +0x1e4, three floats: the
/// case's x, play's).
fn scene_camera_at(e: &mut Engine, x: u32) {
    let node = e.mem.alloc(0x200);
    for (k, v) in (0u32..).zip([x, 0xc480_0000, 0x4300_0000]) {
        set(e, node + 0x1e4 + 4 * k, v);
    }
    set(e, 0x011f_91c8, node);
}

/// The lighting 3.0 position helpers' inputs: the entries' switches; the camera's x, the origin's x, the
/// sun's x (play's floats); the flag word (the world matrix's).
const LIGHTING30_POSITIONS: &[Input] = &[
    Input::Of(&[0, u32::MAX, 0x0555_5555]),
    Input::F32,
    Input::F32Swept(0x4500_0000),
    Input::F32Swept(0x4600_0000),
    Input::Flag,
];

/// The position helpers' words: the entries, the scene camera, the origin, the sun, a transform of play's
/// floats (a small rotation, a translation, scale 2), a bound.
fn lighting30_position_words(e: &mut Engine, c: &Case) -> (u32, u32, u32) {
    let _ = world(e);
    lighting30_entries(e, c.word(0));
    scene_camera_at(e, play_bits(c, 1));
    for (k, v) in (0u32..).zip([play_bits(c, 2), 0x4300_0000, 0x4200_0000]) {
        set(e, 0x011f_474c + 4 * k, v);
    }
    for (k, v) in (0u32..).zip([play_bits(c, 3), 0x4680_0000, 0x4600_0000]) {
        set(e, 0x011f_a2a0 + 4 * k, v);
    }
    for k in 0..8u32 {
        set(e, 0x011f_d874 + 4 * k, 0x3c00_0000 + k);
    }
    let vt = vtable(e, &[]);
    let this = obj(e, 0xa4, vt);
    let transform = e.mem.alloc(0x34);
    for (i, v) in (0u32..).zip([
        0x3f7e_8a6e,
        0xbdcc_6c67,
        0,
        0x3dcc_6c67,
        0x3f7e_8a6e,
        0,
        0,
        0,
        ONE,
        0x4580_0000,
        0xc520_0000,
        0x4400_0000,
        TWO,
    ]) {
        set(e, transform + 4 * i, v);
    }
    (this, transform, transform + 0x24)
}

/// The eye position (`00bbbcf0`): main-thread code (`f32`) on the main thread's word, bit for bit (NaN
/// payloads aside), with play's floats.
#[test]
fn lighting30_eye_position() {
    Diff::new(entry!(
        0x00bb_bcf0,
        engine_typed::draw::lighting30::eye_position(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(LIGHTING30_POSITIONS)
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let (this, _, position) = lighting30_position_words(e, c);
        vec![this, position]
    })
    .run()
    .assert_same();
}

/// The world matrix (`00bbba00`): main-thread code (`f32`) on the main thread's word, bit for bit (NaN
/// payloads aside), with play's floats; the flag word clear or set.
#[test]
fn lighting30_world_matrix() {
    Diff::new(entry!(
        0x00bb_ba00,
        engine_typed::draw::lighting30::world_matrix(Ptr<_>, u32, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(LIGHTING30_POSITIONS)
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let (this, transform, _) = lighting30_position_words(e, c);
        vec![this, c.word(4), transform]
    })
    .run()
    .assert_same();
}

/// The light position (`00bbb890`, the matrix by value): main-thread code (`f32`) on the main thread's
/// word, bit for bit (NaN payloads aside), with play's floats; `D3DXVec3TransformCoord`'s import stood in.
#[test]
fn lighting30_light_position() {
    Diff::new(entry!(
        0x00bb_b890,
        engine_typed::draw::lighting30::light_position(Ptr<_>, [u32; 16], Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(LIGHTING30_POSITIONS)
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let (this, transform, _) = lighting30_position_words(e, c);
        // `D3DXVec3TransformCoord` (out, in, matrix): the input's words turned and marked with the
        // matrix's first word in their mantissas (a float stays a float, so no NaN is made that
        // only one side would quiet), so the input, the matrix and where the output goes all show.
        e.register(0x00fd_f414, |e, a| {
            let mark = e.mem.u32(a[2]);
            for k in 0..3u32 {
                let word = e.mem.u32(a[1] + 4 * ((k + 1) % 3));
                e.mem.set_u32(a[0] + 4 * k, word ^ (mark >> 9));
            }
            ret(a[0])
        });
        let mut words = vec![this];
        words.extend((0..16u32).map(|k| 0x3e00_0000 + 0x0010_0000 * k));
        words.push(transform);
        words
    })
    .run()
    .assert_same();
}

/// The lighting 3.0 fog (`00bbbe80`): main-thread code (`f32`, the dot product in the exe's order) on
/// the main thread's word, bit for bit (NaN payloads aside), with play's floats; the far and near
/// distances both 0.0 or the case's.
#[test]
fn lighting30_fog() {
    Diff::new(entry!(
        0x00bb_be80,
        engine_typed::draw::lighting30::fog(Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(&[
        Input::Of(&[u32::MAX, 0]),
        Input::F32,
        Input::F32Swept(0x3f00_0000),
        Input::F32Swept(0x4120_0000),
        // The fog distances: both 0.0, both set, only the far one, only the near one.
        Input::Of(&[0, 1, 2, 3]),
        Input::F32Swept(0x4680_0000),
        Input::F32Swept(0x4500_0000),
    ])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let _ = world(e);
        lighting30_entries(e, c.word(0));
        scene_camera_at(e, play_bits(c, 1));
        for (k, v) in (0u32..).zip([play_bits(c, 2), 0x4600_0000, 0x4680_0000]) {
            set(e, 0x011f_a2a0 + 4 * k, v);
        }
        for (k, v) in (0u32..).zip([0x3f00_0000, play_bits(c, 3), 0xbf40_0000]) {
            set(e, 0x011f_9604 + 4 * k, v);
        }
        set(e, 0x011f_9610, 0x4400_0000);
        if c.word(4) == 1 || c.word(4) == 2 {
            set(e, 0x011f_9618, play_bits(c, 5));
        }
        if c.word(4) == 1 || c.word(4) == 3 {
            set(e, 0x011f_9614, play_bits(c, 6));
        }
        set(e, 0x0101_2054, 0xbf80_0000);
        let vt = vtable(e, &[]);
        let this = obj(e, 0xa4, vt);
        vec![this]
    })
    .run()
    .assert_same();
}

/// The lighting 3.0 shader's slot 0xfc (`00bbe120`): every pass type of its index tables and others;
/// the masks' patterns; the fog and eye entries on or off (with the switches); a fog property or none,
/// its distances both 0.0 or the case's. Main-thread code (`f32`) on the main thread's word, bit for bit
/// (NaN payloads aside), with play's floats. The fog property's getter (the draw family's) runs by
/// address.
#[test]
fn lighting30_setup_switches() {
    Diff::new(entry!(
        0x00bb_e120,
        engine_typed::draw::lighting30::setup_switches(Ptr<_>, u32)
    ))
    .common()
    .main_thread(0)
    .inputs(&[
        Input::Of(&[
            0x1e0, 0x1e1, 10, 15, 0x1d3, 0x1d6, 0x1da, 0x2da, 0x2db, 0x2dc, 0x2e0, 0x2e7, 0x2ed,
            0x1c4,
        ]),
        Input::Of(&[0, u32::MAX, 0x0555_5555]),
        Input::Of(&[0, 1, 2]),
        Input::F32,
        Input::F32Swept(0x4120_0000),
    ])
    .max_cases(200)
    .float_sweep(FLOAT_SWEEP)
    .setup(lighting30_switches_words)
    .run()
    .assert_same();
}

/// The words of `lighting30_setup_switches`'s cases (also its own-state check's).
fn lighting30_switches_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let _ = world(e);
    lighting30_entries(e, c.word(1));
    if c.word(1) == 0x0555_5555 {
        // A null entry pointer in each table (the switches skip it, the flag bytes still set).
        set(e, 0x0120_1088, 0);
        set(e, 0x0120_10cc, 0);
    }
    for k in 0..0x80u32 {
        set(
            e,
            0x0120_0a28 + 4 * k,
            0x5a5a_5a5a ^ k.wrapping_mul(0x01f3_c1d5),
        );
        set(
            e,
            0x0120_0868 + 4 * k,
            0xa5a5_a5a5 ^ k.wrapping_mul(0x0326_9e1b),
        );
    }
    let vt = vtable(e, &[]);
    let this = obj(e, 0xa4, vt);
    for at in [0x30u32, 0x34] {
        let map = obj(e, 0x40, vt);
        set(e, this + at, map);
    }
    e.register(VT_FN + 0x24, marked);
    let fog = if c.word(2) != 0 {
        let fog = e.mem.alloc(0x80);
        for (k, v) in (0u32..).zip([0x3e00_0000, 0x3e80_0000, 0x3f00_0000]) {
            set(e, fog + 0x20 + 4 * k, v);
        }
        set(e, fog + 0x60, 0x4040_0000);
        if c.word(2) == 2 {
            set(e, fog + 0x30, play_bits(c, 3));
            set(e, fog + 0x2c, play_bits(c, 4));
        }
        fog
    } else {
        0
    };
    let node = e.mem.alloc(0x140);
    set(e, node + 0x134, fog);
    set(e, 0x011f_91c8 + 8, node);
    e.mem.set_u8(0x011f_91c4, 2);
    e.map(0x010a_d000, 0x1000);
    set(e, 0x010a_d6e0, 0x48f4_2400);
    for k in 0..4u32 {
        set(e, 0x011a_9bd0 + 4 * k, 0x3d00_0000 + k);
    }
    for (k, v) in (0u32..).zip([0x4600_0000, 0x4680_0000, 0x4500_0000]) {
        set(e, 0x011f_a2a0 + 4 * k, v);
        set(e, 0x011f_474c + 4 * k, v ^ 0x0040_0000);
    }
    vec![this, c.word(0)]
}

/// The lighting 3.0 material's inputs: the shader mode (8, 10, another); the pass type (the light-scaled
/// range and another); the property's alpha (play's float) and its flags +0x24 (bits 0x20000, 0x8000);
/// the alpha property (none in mode 8 only, its flags 0, 9, field 2); the old ambient fog byte; the
/// light-scaled path's alpha test answer and first active light (none, one).
const LIGHTING30_MATERIAL: &[Input] = &[
    Input::Of(&[3, 8, 10]),
    Input::Of(&[0x1bd, 0x1c3, 0x1c4]),
    Input::F32,
    Input::Of(&[0x8000, 0, 0x2_0000]),
    Input::Of(&[0, 1, 2, 3, 4]),
    Input::Flag,
    Input::Of(&[0, 1]),
    Input::Of(&[1, 0]),
];

/// The material's words: the entries, the defaults (`011a9be0`), the exe's 255.0; the property (its
/// alpha the case's, its slot 0x110's factor in `ST0`), the alpha property (flags and reference); the
/// light-scaled path's callees stood in (the alpha test, the first active light, the lengths and
/// clamps answering in `ST0`, the transforms marking their words), the sun's colour, a light's data.
fn lighting30_material_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let _ = world(e);
    lighting30_entries(e, 0x0555_5555);
    e.mem.set_u64(0x0101_e568, 255.0f64.to_bits());
    for k in 0..4u32 {
        set(e, 0x011a_9be0 + 4 * k, 0x3e00_0000 + k);
    }
    set(e, 0x011f_91b8, c.word(0));
    e.mem.set_u8(0x0120_0bf5, c.word(5) as u8);
    let alpha_test = c.word(6);
    set(e, RETS + 0x3f0, alpha_test);
    e.register(0x004a_2020, |e, _| ret(e.mem.u32(RETS + 0x3f0)));
    let light = if c.word(7) != 0 {
        let data = e.mem.alloc(0x100);
        for (k, v) in (0u32..).zip([0x4120_0000, 0x4100_0000, 0x40c0_0000]) {
            set(e, data + 0xd4 + 4 * k, v);
        }
        set(e, data + 0xc4, 0x3f00_0000);
        set(e, data + 0xe0, 0x4220_0000);
        let light = e.mem.alloc(0x100);
        set(e, light + 0xf8, data);
        light
    } else {
        0
    };
    set(e, RETS + 0x3f4, light);
    e.register(0x00b7_0600, |e, _| ret(e.mem.u32(RETS + 0x3f4)));
    // The lengths and the clamps answer in `ST0`: from their words, so a wrong word shows.
    for addr in [0x00b7_19e0u32, 0x0045_7990] {
        e.register(addr, |e, a| Ret {
            st0: f64::from(f32::from_bits(e.mem.u32(a[0]))) + 1.5,
            ..Ret::default()
        });
    }
    e.register(0x0068_b050, |_, a| Ret {
        st0: f64::from(f32::from_bits(a[2]))
            .max(f64::from_bits(u64::from(a[0]) | (u64::from(a[1]) << 32))),
        ..Ret::default()
    });
    e.register(0x00b7_19c0, |_, a| {
        let first = f64::from_bits(u64::from(a[0]) | (u64::from(a[1]) << 32));
        let second = f64::from_bits(u64::from(a[2]) | (u64::from(a[3]) << 32));
        Ret {
            st0: first.min(second),
            ..Ret::default()
        }
    });
    e.register(0x0043_9ef0, marked);
    // A point times a scalar (`00633b40`: the point, the result, the scale): the result written, so the
    // light's colour length (scaled by its fade) can exceed the sun's and be the one chosen.
    e.register(0x0063_3b40, |e, a| {
        let scale = f32::from_bits(a[2]);
        for k in 0..3u32 {
            let value = f32::from_bits(e.mem.u32(a[0] + 4 * k)) * scale;
            e.mem.set_u32(a[1] + 4 * k, value.to_bits());
        }
        ret(a[1])
    });
    let node = e.mem.alloc(0x200);
    let list = e.mem.alloc(0x100);
    let sun = e.mem.alloc(0x100);
    for (k, v) in (0u32..).zip([0x3f00_0000, 0x3e80_0000, 0x3fc0_0000]) {
        set(e, sun + 0xd4 + 4 * k, v);
    }
    set(e, list + 0xf8, sun);
    set(e, node + 0xe0, list);
    set(e, 0x011f_91c8, node);
    let vt = vtable(e, &[]);
    e.register(VT_FN + 0x110, |_, _| Ret {
        st0: 0.75,
        ..Ret::default()
    });
    let this = obj(e, 0xa4, vt);
    let property = lighting30_property(e, 0, c.word(3), play_bits(c, 2));
    let state = e.mem.alloc(0x18);
    if c.word(4) != 0 || c.word(0) != 8 {
        let alpha = obj(e, 0x20, vt);
        let flags: u16 = match c.word(4) {
            1 => 1,
            2 => 1 << 9,
            4 => 1 << 5,
            _ => 2 << 5,
        };
        e.mem.set_u16(alpha + 0x18, flags);
        e.mem.set_u8(alpha + 0x1a, 0x80);
        set(e, state, alpha);
    }
    let bound = e.mem.alloc(0x10);
    vec![this, bound, property, state, c.word(1)]
}

/// The lighting 3.0 material constants (`00bbe9c0`) with the light-scaled alpha: main-thread code
/// (`f32`; a callee's `double` in `ST0` in `f64`, rounded once) on the main thread's word, bit for bit
/// (NaN payloads aside), with play's floats.
#[test]
fn lighting30_material() {
    Diff::new(entry!(
        0x00bb_e9c0,
        engine_typed::draw::lighting30::material(Ptr<_>, u32, Ptr<_>, Ptr<_>, u32)
    ))
    .common()
    .main_thread(0)
    .inputs(LIGHTING30_MATERIAL)
    .max_cases(200)
    .float_sweep(FLOAT_SWEEP)
    .setup(lighting30_material_words)
    .run()
    .assert_same();
}

/// The lighting 3.0 colour (`00bbecd0`): the material's colour through its pointer or its own; equal to
/// the neutral colour or not; the property with lights or none; HDR on or off. Main-thread code (`f32`)
/// on the main thread's word, bit for bit (NaN payloads aside), with play's floats.
#[test]
fn lighting30_colour() {
    Diff::new(entry!(
        0x00bb_ecd0,
        engine_typed::draw::lighting30::colour(Ptr<_>, Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(&[
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::Flag,
        Input::F32,
        Input::F32Swept(0x4000_0000),
    ])
    .float_sweep(FLOAT_SWEEP)
    .setup(|e, c| {
        let _ = world(e);
        lighting30_entries(e, 0x0555_5555);
        let material = e.mem.alloc(0x48);
        let colour = [play_bits(c, 4), 0x3f00_0000, 0x3e80_0000];
        let at = if c.word(0) != 0 {
            let own = e.mem.alloc(0x10);
            set(e, material + 0x34, own);
            own
        } else {
            material + 0x28
        };
        for (k, v) in (0u32..).zip(colour) {
            set(e, at + 4 * k, v);
            let neutral = if c.word(1) != 0 { v } else { v ^ 1 };
            set(e, 0x011f_4998 + 4 * k, neutral);
        }
        set(e, material + 0x40, play_bits(c, 5));
        let property = lighting30_property(e, 0, 0, HALF);
        let list = e.mem.alloc(8);
        set(e, list, if c.word(2) != 0 { 0x1170_0000 } else { 0 });
        set(e, property + 0xb4, list);
        e.mem.set_u8(0x011f_941e, c.word(3) as u8);
        let data = e.mem.alloc(0x10);
        set(e, data + 0xc, 0x3fc0_0000);
        let effect = e.mem.alloc(0x70);
        set(e, effect + 0x6c, data);
        let effects = e.mem.alloc(8);
        set(e, effects + 4, effect);
        let manager = e.mem.alloc(0x10);
        set(e, manager + 8, effects);
        set(e, 0x011f_91ac, manager);
        for k in 0..4u32 {
            set(e, 0x0120_1b30 + 4 * k, 0x3e00_0000 + 0x0080_0000 * k);
        }
        let vt = vtable(e, &[]);
        let this = obj(e, 0xa4, vt);
        vec![this, material, property]
    })
    .run()
    .assert_same();
}

/// A rendered pass for the lighting 3.0 tests: its type `pass_type`, `count` lights (each with a matrix
/// at +0x10 of a pattern and its byte +0xed set for the first when `marked`), its light-selected byte.
fn lighting30_pass(e: &mut Engine, pass_type: u32, count: u32, marked: bool) -> u32 {
    let pass = e.mem.alloc(0x10);
    e.mem.set_u16(pass + 4, pass_type as u16);
    e.mem.set_u8(pass + 9, count as u8);
    if count != 0 {
        let lights = e.mem.alloc(4 * count);
        for k in 0..count {
            let light = e.mem.alloc(0x220);
            for i in 0..16u32 {
                set(e, light + 0x10 + 4 * i, 0x3e00_0000 + 0x0010_0000 * i + k);
            }
            e.mem.set_u8(light + 0xed, u8::from(marked && k == 0));
            set(e, lights + 4 * k, light);
        }
        set(e, pass + 0xc, lights);
    }
    pass
}

/// The lighting 3.0 view matrix (`00bbe6b0`): its entry on or off; the pass's lights (none, two, the
/// first marked); the pass type (the copied ranges, the first light's, another); the flag word; the
/// projection's entry. No arithmetic but the origin's sums (`f32`); the D3DX imports and `GetD3DFromNi`
/// stood in; the main thread's word.
#[test]
fn lighting30_view_matrix() {
    Diff::new(entry!(
        0x00bb_e6b0,
        engine_typed::draw::lighting30::view_matrix(Ptr<_>, Ptr<_>, u32, u32, Ptr<_>)
    ))
    .common()
    .goes_by_address(&[0x00ec_44d0])
    .main_thread(0)
    .inputs(&[
        Input::Of(&[0, u32::MAX, 0x0000_0040]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0x1bd, 0x2d8, 0x1c4, 0x30, 0x1c3, 0x2ef, 0x1c7, 0x1c8]),
        Input::Flag,
        Input::F32,
    ])
    .float_sweep(FLOAT_SWEEP)
    .setup(lighting30_view_matrix_words)
    .run()
    .assert_same();
}

/// The words of `lighting30_view_matrix`'s cases (also its own-state check's).
fn lighting30_view_matrix_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    lighting30_entries(e, c.word(0));
    matrix_inverse_stand_in(e);
    for addr in [0x00fd_f424u32, 0x00fd_f42c] {
        e.register(addr, |e, a| {
            for k in 0..16u32 {
                let word = e.mem.u32(a[1] + 4 * ((5 * k + 3) % 16));
                e.mem.set_u32(a[0] + 4 * k, word ^ 1);
            }
            ret(a[0])
        });
    }
    e.register(0x00b7_99a0, |e, a| {
        for k in 0..16u32 {
            e.mem.set_u32(a[0] + 4 * k, 0x3f00_0000 + k);
        }
        ret(a[0])
    });
    for k in 0..3u32 {
        set(
            e,
            0x011f_474c + 4 * k,
            if k == 0 { play_bits(c, 4) } else { 0x4300_0000 },
        );
    }
    let renderer = w.renderer;
    for k in 0..16u32 {
        set(e, renderer + 0x940 + 4 * k, 0x3c00_0000 + k);
    }
    let vt = vtable(e, &[]);
    let this = obj(e, 0xa4, vt);
    set(e, this + 0x18, renderer);
    let pass = match c.word(1) {
        0 => lighting30_pass(e, 0x30, 0, false),
        1 => lighting30_pass(e, 0x30, 2, false),
        _ => lighting30_pass(e, 0x30, 2, true),
    };
    let transform = e.mem.alloc(0x34);
    vec![this, pass, c.word(2), c.word(3), transform]
}

/// The lighting 3.0 lights' inputs: the shader mode (0xf, 10, another); the pass type (below 0x1bd,
/// 0x1c3, 0x1c4, 0x1c6, another); the pass's type and light count (two, none, a shadow type); HDR with
/// the property's bit 0x400, full bright; the active lights; the light data's counts (play's floats:
/// whole numbers below 3 and others); the flag word; the transform's scale (play's float).
const LIGHTING30_LIGHTS: &[Input] = &[
    Input::Of(&[0xf, 0xa, 3]),
    Input::Of(&[0x30, 0x1c3, 0x1c4, 0x1c5, 0x1c6, 0x1d0]),
    // The pass: two lights, none, a shadow type with two, one, as many as the limit (4), more (6).
    Input::Of(&[0, 1, 2, 3, 4, 5]),
    Input::Of(&[0, 1, 2, 3]),
    Input::Flag,
    Input::Of(&[0, 0x3f80_0000, 0x4000_0000, 0x3fc0_0000]),
    Input::Flag,
    Input::F32,
    // The exe's limit on the light count: 4, or 1 (below the pass's lights).
    Input::Of(&[4, 1]),
    // The property's bit 0x400 (+0x20), apart from HDR, so all four combinations run.
    Input::Flag,
];

/// The lights' words: the entries, the setters stood in (each writing its index into the light colour
/// row and an entry, so the colour and the entries depend on the lights), the light scale, the fog
/// colour, the limit, the 0.5, the held accumulator with an ambient colour, the camera position, the
/// D3DX imports and the unitize stood in.
fn lighting30_lights_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let _ = world(e);
    lighting30_entries(e, 0x0555_5555);
    scene_camera_at(e, 0x4500_0000);
    set(e, 0x011f_91b8, c.word(0));
    for addr in [0x00b7_0de0u32, 0x00b7_1130] {
        e.register(addr, |e, a| {
            for k in 0..4u32 {
                e.mem
                    .set_u32(0x011f_a0d0 + 4 * k, 0x3e80_0000 + 0x0010_0000 * (a[0] + k));
            }
            let entry = 0x0120_1c90 + 0x20 * (a[0] & 7);
            for k in 0..4u32 {
                e.mem
                    .set_u32(entry + 4 * k, 0x3f00_0000 + 0x0008_0000 * (a[0] + k));
            }
            ret(0)
        });
    }
    let hdr = c.word(3) & 1;
    e.mem.set_u8(0x011f_941e, hdr as u8);
    e.mem.set_u8(0x011f_91a7, (c.word(3) >> 1) as u8);
    set(e, 0x011f_9184, 0x3fc0_0000);
    for k in 0..4u32 {
        set(e, 0x011a_9bd0 + 4 * k, 0x3d00_0000 + k);
        set(e, 0x0120_1b30 + 4 * k, 0x3d80_0000 + k);
    }
    set(e, 0x011a_e60c, c.word(8));
    set(e, 0x0101_6248, HALF);
    let active = c.word(4);
    set(e, RETS + 0x3f8, active * 2);
    e.register(0x00b7_0790, |e, _| ret(e.mem.u32(RETS + 0x3f8)));
    let held = e.mem.alloc(0x1ac);
    for k in 0..4u32 {
        set(e, held + 0x154 + 4 * k, 0x3e00_0000 + k);
    }
    set(e, 0x011f_95f0, held);
    for addr in [0x00fd_f420u32, 0x00fd_f41c, 0x00fd_f414] {
        e.register(addr, |e, a| {
            for k in 0..3u32 {
                let word = e.mem.u32(a[1] + 4 * ((k + 1) % 3));
                e.mem.set_u32(a[0] + 4 * k, word);
            }
            ret(a[0])
        });
    }
    e.register(0x004a_0c10, |_, _| ret(0));
    let (kind, count) = match c.word(2) {
        0 => (0x30, 2),
        1 => (0x30, 0),
        2 => (0x1c5, 2),
        3 => (0x30, 1),
        4 => (0x30, 4),
        _ => (0x30, 6),
    };
    let pass = lighting30_pass(e, kind, count, false);
    let property = lighting30_property(e, if c.word(9) != 0 { 0x400 } else { 0 }, 0, HALF);
    let material = e.mem.alloc(0x48);
    set(e, material + 0x38, 0x3f30_0000);
    let matrix = e.mem.alloc(0x40);
    let transform = e.mem.alloc(0x34);
    set(e, transform + 0x30, play_bits(c, 7));
    // The light data's counts as the setters leave them: the case's, for the passes that read them as set.
    set(e, 0x0120_1c70, c.word(5));
    set(e, 0x0120_1c74, c.word(5));
    let vt = vtable(e, &[]);
    let this = obj(e, 0xa4, vt);
    vec![
        this,
        property,
        pass,
        c.word(1),
        material,
        matrix,
        c.word(6),
        transform,
    ]
}

/// The lighting 3.0 lights (`00bbc050` with the set-up `00bbc0a4` it jumps to): main-thread code
/// (`f32`) on the main thread's word, bit for bit (NaN payloads aside), with play's floats. The CRT's
/// `_ftol2` and the dispatch family's unitize run by address (their own tests).
#[test]
fn lighting30_lights() {
    Diff::new(entry!(
        0x00bb_c050,
        engine_typed::draw::lighting30::lights(
            Ptr<_>,
            Ptr<_>,
            Ptr<_>,
            u32,
            Ptr<_>,
            u32,
            u32,
            Ptr<_>,
        )
    ))
    .common()
    .goes_by_address(&[0x00ec_62c0, 0x004a_0c10])
    // The held accumulator's getter is a read of `011f95f0`, done in place (`packet::held_accumulator`).
    .ignore_calls(&[0x00b4_f5d0])
    .main_thread(0)
    .inputs(LIGHTING30_LIGHTS)
    .max_cases(400)
    .float_sweep(FLOAT_SWEEP)
    .setup(lighting30_lights_words)
    .run()
    .assert_same();
}

/// The lighting 3.0 geometry set-up's plain callees in the translation (its helpers, typed above with
/// their own tests): the light position, the lights, the view and world matrices, the eye position, the
/// emittance, the material, the colour, the glow, the alpha, the fog.
const LIGHTING30_HELPERS: &[u32] = &[
    0x00bb_b890,
    0x00bb_c050,
    0x00bb_e6b0,
    0x00bb_ba00,
    0x00bb_bcf0,
    0x00bb_bda0,
    0x00bb_e9c0,
    0x00bb_ecd0,
    0x00bb_bdf0,
    0x00bb_be30,
    0x00bb_be80,
];

/// The geometry set-up's inputs: the pass type (the z-write range, the special set-ups', the shadow
/// clip planes' range, another); the alpha property's flag 0; the refraction and parallax entries; the
/// light refusing the computed planes, the held accumulator refusing them (its +0x164, +0x165); the
/// constant maps (none, stand-ins); the state's fifth property (none, flag 0 clear or set); the game time
/// (play's float).
const LIGHTING30_CONSTANTS: &[Input] = &[
    Input::Of(&[
        0x1d4, 0x1e0, 0xe, 0x1c4, 0x1c7, 0x30, 0x1e1, 0xf, 0x1d3, 0x1d6,
    ]),
    Input::Flag,
    Input::Of(&[u32::MAX, 0]),
    Input::Of(&[0, 1, 2, 3]),
    Input::Flag,
    Input::Of(&[0, 1, 2]),
    Input::F32,
    // The refraction count (+0xe4, an integer): small, one past a float's 24 bits, negative.
    Input::Of(&[3, 0x0100_0001, 0xffff_fffd]),
    // Skinned or not (the geometry's skin instance +0xbc: the flag the helpers take).
    Input::Flag,
];

/// The geometry set-up's words: the world's renderer as the shader's (its light matrices at +0x980 and
/// +0x9c0), the rendered pass (two lights, the first's planes +0x144 and +0x1a8 and its refusing byte
/// +0x208), its geometry (a transform, a bound, a skin word, its data's buffers), the state's properties;
/// the special set-ups, the camera position, the plane store and the D3DX imports stood in; the helpers
/// run by address (their translations both sides).
fn lighting30_constants_words(e: &mut Engine, c: &Case) -> Vec<u32> {
    let w = world(e);
    lighting30_entries(e, c.word(2));
    matrix_inverse_stand_in(e);
    for addr in [
        0x00bb_dda0u32,
        0x00bb_dcd0,
        0x00bb_de20,
        0x00b7_1b40,
        0x00b7_1ad0,
        0x00fd_f424,
        0x00fd_f42c,
        0x00fd_f430,
        0x00fd_f418,
        0x00fd_f414,
        0x00fd_f420,
        0x00fd_f41c,
        // The lights' setters, the active light count and `GetD3DFromNi` (the helpers' callees; the
        // helpers' own tests cover what they do with them).
        0x00b7_0de0,
        0x00b7_1130,
        0x00b7_0790,
        0x00b7_99a0,
    ] {
        e.register(addr, marked);
    }
    e.mem.set_u64(0x0101_2640, 3600.0f64.to_bits());
    e.map(0x0104_e000, 0x1000);
    e.mem.set_u64(0x0104_eea0, 0x4019_21fa_0000_0000);
    set(e, 0x011f_940c, play_bits(c, 6));
    scene_camera_at(e, 0x4500_0000);
    let renderer = w.renderer;
    for k in 0..32u32 {
        set(e, renderer + 0x980 + 4 * k, 0x3c00_0000 + 0x0004_0000 * k);
    }
    let held = e.mem.alloc(0x1ac);
    e.mem.set_u8(held + 0x164, u8::from(c.word(3) & 2 != 0));
    e.mem.set_u8(held + 0x165, u8::from(c.word(3) == 3));
    set(e, 0x011f_95ec, held);
    let vt = vtable(e, &[]);
    let this = obj(e, 0xa4, vt);
    set(e, this + 0x18, renderer);
    let target = obj(e, 0x10, vt);
    set(e, this + 0x1c, target);
    let pass_type = c.word(0);
    let pass = lighting30_pass(e, pass_type, 2, false);
    let first = e.mem.u32(e.mem.u32(pass + 0xc));
    for k in 0..24u32 {
        set(e, first + 0x144 + 4 * k, 0x3e00_0000 + 0x0002_0000 * k);
        set(e, first + 0x1a8 + 4 * k, 0x3f00_0000 + k);
    }
    e.mem.set_u8(first + 0x208, u8::from(c.word(3) == 1));
    let geometry = e.mem.alloc(0xc0);
    for (i, v) in (0u32..).zip([
        ONE,
        0,
        0,
        0,
        ONE,
        0,
        0,
        0,
        ONE,
        0x4580_0000,
        0xc520_0000,
        0x4400_0000,
        ONE,
    ]) {
        set(e, geometry + 0x68 + 4 * i, v);
    }
    let bound = e.mem.alloc(0x10);
    set(e, geometry + 0x20, bound);
    let data = e.mem.alloc(0x40);
    set(e, data + 0x34, 0x0bbf_0000);
    set(e, geometry + 0xb8, data);
    if c.word(8) != 0 {
        // A skin instance (only its being there is read: the helpers take it as their flag).
        set(e, geometry + 0xbc, 0x5c1d_0000);
    }
    set(e, pass, geometry);
    set(e, 0x011f_91e0, pass);
    set(e, 0x011f_91e4, pass_type);
    let property = lighting30_property(e, 0, 0, HALF);
    set(e, property + 0xe4, c.word(7));
    // No lights on the property (its light list empty), and the exe's limit on the light count.
    let list = e.mem.alloc(8);
    set(e, property + 0xb4, list);
    set(e, 0x011a_e60c, 4);
    let material = e.mem.alloc(0x48);
    let state = e.mem.alloc(0x18);
    let alpha = obj(e, 0x20, vt);
    e.mem.set_u16(alpha + 0x18, c.word(1) as u16);
    set(e, state, alpha);
    set(e, state + 8, material);
    set(e, state + 0xc, property);
    if c.word(5) != 0 {
        let other = obj(e, 0x20, vt);
        e.mem.set_u16(other + 0x18, (c.word(5) - 1) as u16);
        set(e, state + 0x10, other);
    }
    let current = e.mem.alloc(0x60);
    set(e, 0x0126_f74c, current);
    if c.word(4) != 0 {
        set(e, current + 0x44, 0x0b0b_0044);
        set(e, current + 0x5c, 0x0b0b_005c);
        for at in [0x34u32, 0x48] {
            let map = obj(e, 0x40, vt);
            set(e, current + at, map);
        }
        for at in [0x30u32, 0x34] {
            set(e, this + at, 0x0a0a_0000 + at);
        }
    }
    vec![this, state]
}

/// The lighting 3.0 shader's slot 0x7c (`00bbef90`): main-thread code (`f32`; the refraction's steps
/// with the exe's `double`s, which have 24 significant bits or fewer, in `f64` rounded once) on the main
/// thread's word, bit for bit (NaN payloads aside), with play's floats. Its helpers (their own tests
/// above) are plain calls in the translation and run by address here, and so do the shader, dispatch and
/// render families' functions it calls ([`Common::shader_translated`]).
#[test]
fn lighting30_setup_constants() {
    Diff::new(entry!(
        0x00bb_ef90,
        engine_typed::draw::lighting30::setup_constants(Ptr<_>, Ptr<_>)
    ))
    .shader_translated()
    .goes_by_address(LIGHTING30_HELPERS)
    .goes_by_address(&[0x00c4_c2d0])
    // The shader accumulator's getter is a read of `011f95ec`, done in place (`packet::shader_accumulator`).
    .ignore_calls(&[0x00b4_f5c0])
    .translation_calls_plainly(LIGHTING30_HELPERS)
    .main_thread(0)
    .inputs(LIGHTING30_CONSTANTS)
    .max_cases(320)
    .float_sweep(FLOAT_SWEEP)
    .setup(lighting30_constants_words)
    .run()
    .assert_same();
}

/// The `010bb980` shader's slot 0x78 (`00bd02e0`): each of the exe's lists by its pass types and the
/// base set-up (the shader family's) outside them. The stage set-ups are the shader family's
/// (`00b7c0a0`, `00b7c0e0`) or by address (`00b7c1f0`, stood in); the main thread's word.
#[test]
fn lighting30_stage_textures() {
    Diff::new(entry!(
        0x00bd_02e0,
        engine_typed::draw::lighting30::setup_stage_textures(Ptr<_>, Ptr<_>)
    ))
    .common()
    .main_thread(0)
    .inputs(&[Input::Of(&[
        0xc5, 0xc6, 0xdb, 0x128, 0x130, 0x142, 0x143, 0x30,
    ])])
    .setup(|e, c| {
        let w = world(e);
        e.register(0x00b7_c1f0, marked);
        e.register(VT_FN + 0x108, marked);
        set(e, 0x011f_91e4, c.word(0));
        let vt = vtable(e, &[]);
        let this = obj(e, 0xa4, vt);
        let property = lighting30_property(e, 0, 0, HALF);
        let textures = e.mem.alloc(0x20);
        for k in 0..8u32 {
            set(e, textures + 4 * k, 0x7e57_0000 + k);
        }
        set(e, property + 0xac, textures);
        set(e, property + 0xb0, textures);
        let state = e.mem.alloc(0x18);
        set(e, state + 0xc, property);
        let pass = e.mem.alloc(0x60);
        let stages = e.mem.alloc(0x20);
        for k in 0..8u32 {
            let stage = if k == 0 { w.stage } else { second_stage(e, &w) };
            set(e, stages + 4 * k, stage);
        }
        set(e, pass + 0x24, stages);
        set(e, 0x0126_f74c, pass);
        vec![this, state]
    })
    .run()
    .assert_same();
}

/// The light table entry (`00bb95e0`): entry 0 or 3, eight words. No floats; the main thread's word.
#[test]
fn set_light_entry() {
    Diff::new(entry!(
        0x00bb_95e0,
        group_c::set_light_entry(u32, u32, u32, u32, u32, u32, u32, u32, u32)
    ))
    .common()
    .main_thread(0)
    .inputs(&[Input::Of(&[0, 3])])
    .setup(|e, c| {
        let _ = world(e);
        e.map(0x0120_1000, 0x1000);
        let mut words = vec![c.word(0)];
        words.extend((0..8u32).map(|k| 0x3c00_0000 + k));
        words
    })
    .run()
    .assert_same();
}

/// The pass end's flags (`00bb96d0`): the entry at `012010bc` switched on or off before. No floats; the
/// main thread's word.
#[test]
fn pass_end_flags() {
    Diff::new(entry!(0x00bb_96d0, group_c::pass_end_flags(Ptr<_>, u32)))
        .common()
        .main_thread(0)
        .inputs(&[Input::Flag])
        // No world: its stand-ins hold this address (the tests that don't name the draw family stand it in).
        .setup(|e, c| {
            lighting30_entries(e, if c.word(0) != 0 { u32::MAX } else { 0 });
            e.mem.set_u8(0x0120_0863, 1);
            vec![0x0bad_0004, 0x0bad_0005]
        })
        .run()
        .assert_same();
}

// ---- The draw's own state (the drawing thread's step 2, piece 3: `draw::statics`) ----

use engine_typed::draw::statics;

/// The pages that hold Group C's statics block (`01200450..01202150`), mapped in every own-state world so the cut can
/// take the block.
const GROUP_C_PAGES: (u32, u32) = (0x0120_0000, 0x3000);

/// The names of `statics::GLOBAL_BLOCKS`, in its order.
const GLOBAL_BLOCK_NAMES: [&str; 10] = [
    "the rendering pass and type",
    "the shader and held accumulators",
    "the shader statics",
    "Group C's statics",
    "the current pass",
    "the group zero flag",
    "the last light's point flag",
    "the terrain fade flag",
    "the shader mode",
    "the shader mode's function words",
];

/// `f` run, for each case and both scribbles, on two engines `setup` makes alike (it answers the call's words and
/// the accumulator): one plain, one with the value packet open around the call holding the draw's own state
/// (`statics::take`: the shader statics, the render-state cache, the accumulator's camera), scribbled (POISON).
/// Both must answer the same and leave the same memory beside the stack for locals once the join has put the own
/// state back; nothing scribbled may be written past the packet, and in this serial order the join keeps no
/// byte of main's.
fn own_state_same(
    name: &str,
    f: AbiFn,
    cases: &[Vec<u32>],
    setup: impl Fn(&mut Engine, &Case) -> (Vec<u32>, u32),
    fill: impl Fn(&mut Engine, u32, &[u32]),
) {
    own_state_same_floats(name, f, cases, &[], setup, fill);
}

/// [`own_state_same`] with the case words at `floats` given as floats (for setups that read them so).
fn own_state_same_floats(
    name: &str,
    f: AbiFn,
    cases: &[Vec<u32>],
    floats: &[usize],
    setup: impl Fn(&mut Engine, &Case) -> (Vec<u32>, u32),
    fill: impl Fn(&mut Engine, u32, &[u32]),
) {
    for words in cases {
        for scribble in [capture::SCRIBBLE, capture::SCRIBBLE_OTHER] {
            let case = Case(
                (0..)
                    .zip(words)
                    .map(|(i, w)| {
                        if floats.contains(&i) {
                            Value::F32(*w)
                        } else {
                            Value::Word(*w)
                        }
                    })
                    .collect(),
            );
            let mut plain = Engine::new();
            plain.map(GROUP_C_PAGES.0, GROUP_C_PAGES.1);
            let (a, _) = setup(&mut plain, &case);
            plain.with_stack(4, |_, _| ());
            let answer = f(&mut plain, &a);
            let mut owned = Engine::new();
            owned.map(GROUP_C_PAGES.0, GROUP_C_PAGES.1);
            let (b, acc) = setup(&mut owned, &case);
            owned.with_stack(4, |_, _| ());
            assert!(capture::begin(&owned), "{name}: a packet was left open");
            statics::take(&owned, Ptr::new(acc));
            for (block, (lo, hi)) in GLOBAL_BLOCK_NAMES.iter().zip(statics::GLOBAL_BLOCKS) {
                assert!(
                    capture::owns(&owned, lo, hi - lo),
                    "{name}: the cut did not take {block} ({lo:08x}..{hi:08x})"
                );
            }
            fill(&mut owned, acc, &b);
            capture::poison(&mut owned, scribble);
            let owned_answer = f(&mut owned, &b);
            let report = capture::end(&mut owned);
            assert_eq!(
                answer, owned_answer,
                "{name} {words:x?} {scribble:#x}: the answers differ"
            );
            assert_eq!(
                memory_digest_beside_stack(&plain),
                memory_digest_beside_stack(&owned),
                "{name} {words:x?} {scribble:#x}: the memory differs"
            );
            assert_eq!(
                (report.misses, report.written, report.kept_main),
                (0, 0, 0),
                "{name} {words:x?} {scribble:#x}: {report:?}"
            );
        }
    }
}

/// The lock-counted render-state setters keep their counters and the render-state cache in the packet: with
/// the counters and cached values scribbled in guest memory, they set what they set with the switch off.
#[test]
fn render_state_setters_keep_the_draws_own_state() {
    for (name, f, counter_index, state) in [
        (
            "set_z_enable",
            entry!(0x00b9_7de0, state::set_z_enable(u8, u32)).1,
            7u32,
            7u32,
        ),
        (
            "set_z_write_enable",
            entry!(0x00b9_7e30, state::set_z_write_enable(u8, u32)).1,
            6,
            0xe,
        ),
        (
            "set_alpha_test_enable",
            entry!(0x00b9_7ed0, state::set_alpha_test_enable(u8, u32)).1,
            7,
            0xf,
        ),
        (
            "set_alpha_blend_enable",
            entry!(0x00b9_7fa0, state::set_alpha_blend_enable(u8, u32)).1,
            3,
            0x1b,
        ),
        (
            "set_stencil_enable",
            entry!(0x00b9_8070, state::set_stencil_enable(u8, u32)).1,
            4,
            0x34,
        ),
        (
            "set_depth_bias",
            entry!(0x00b9_8320, state::set_depth_bias(u32, u32)).1,
            0,
            0xc3,
        ),
    ] {
        own_state_same(
            name,
            f,
            &product(&[BYTES, LOCKS, TWO_WAYS, TWO_WAYS]),
            |e, c| {
                let w = world(e);
                // The device's SetRenderState counted and its last state and value kept, so a cache compared in
                // place (a set sent or skipped wrongly) shows.
                e.map(CONSTANT_UPLOAD_COPY, 0x2000);
                e.register(VT_FN + 0xe4, set_render_state_answer);
                counter(e, counter_index, c.word(2));
                cached_state(e, &w, state, c.word(3), 1);
                (vec![c.word(0), c.word(1)], w.acc)
            },
            |_, _, _| {},
        );
    }
}

/// The clip planes' setter (`00b98280`) keeps its counter and the cached state in the packet.
#[test]
fn clip_planes_keep_the_draws_own_state() {
    own_state_same(
        "set_clip_planes",
        entry!(
            0x00b9_8280,
            state::set_clip_planes(u8, u8, u8, u8, u8, u8, u32)
        )
        .1,
        &product(&[&[0, 1], &[0, 4], LOCKS, TWO_WAYS, TWO_WAYS]),
        |e, c| {
            let w = world(e);
            e.map(CONSTANT_UPLOAD_COPY, 0x2000);
            e.register(VT_FN + 0xe4, set_render_state_answer);
            counter(e, 0, c.word(3));
            cached_state(e, &w, 0x98, c.word(4), 1);
            (
                vec![c.word(0), 1, c.word(1), 0, 1, c.word(0), c.word(2)],
                w.acc,
            )
        },
        |_, _, _| {},
    );
}

/// Group C's lock releases (`00714a40` and its neighbours) give their locks back in the packet and set their
/// state through it: the counter the release lowers is the one the setter then reads.
#[test]
fn lock_releases_keep_the_draws_own_state() {
    for (name, f, at, state) in [
        (
            "release_z_enable",
            entry!(0x0071_4a40, group_c::release_z_enable(u32)).1,
            0x011f_f9d8u32,
            7u32,
        ),
        (
            "release_z_write",
            entry!(0x0071_4a60, group_c::release_z_write(u32)).1,
            0x011f_f9dc,
            0xe,
        ),
        (
            "release_alpha_test",
            entry!(0x0071_4aa0, group_c::release_alpha_test(u32)).1,
            0x011f_f9e8,
            0xf,
        ),
        (
            "release_alpha_blend",
            entry!(0x0071_4ae0, group_c::release_alpha_blend(u32)).1,
            0x011f_f9f0,
            0x1b,
        ),
        (
            "release_color_write",
            entry!(0x004e_ced0, group_c::release_color_write(u32)).1,
            0x011f_fa18,
            0xa8,
        ),
    ] {
        own_state_same(
            name,
            f,
            &product(&[&[0, 1, 3], &[0, 1, 2, 5], TWO_WAYS, TWO_WAYS]),
            |e, c| {
                let w = world(e);
                e.map(CONSTANT_UPLOAD_COPY, 0x2000);
                e.register(VT_FN + 0xe4, set_render_state_answer);
                set(e, at, c.word(1));
                cached_state(e, &w, state, c.word(2), c.word(3));
                (vec![c.word(0)], w.acc)
            },
            |_, _, _| {},
        );
    }
}

/// The decal pass (`00b64e30`) takes and gives back the z-write and depth-bias locks in the packet, both by the
/// typed setters and by hand (`011ff9dc`, `011ff9e4`), and reads the decal depth bias (`01200458`, Group C's
/// block) from it.
#[test]
fn decal_pass_keeps_the_draws_own_state() {
    own_state_same(
        "decal_pass",
        entry!(0x00b6_4e30, drivers::decal_pass(Ptr<_>)).1,
        &product(&[&[1], &[0, 8], TWO_WAYS, &[0, ONE, 0x7f80_0001], &[0, 1, 2]]),
        |e, c| {
            let d = draw_world(e);
            e.map(CONSTANT_UPLOAD_COPY, 0x2000);
            e.register(VT_FN + 0xe4, set_render_state_answer);
            e.mem.set_u8(d.w.acc + 0x16c, c.word(0) as u8);
            set(e, d.w.acc + 0x19c, c.word(1));
            e.mem.set_u8(0x011a_d80f, c.word(2) as u8);
            set(e, 0x0120_0458, c.word(3));
            set(e, 0x011f_f9dc, c.word(4) + 1);
            set(e, 0x011f_f9e4, c.word(4) + 1);
            (vec![d.w.acc], d.w.acc)
        },
        |_, _, _| {},
    );
}

/// The per-light set-up (`00b70820`) writes the light colour rows, a point light's position (`011fa170` on) and
/// a directional light's direction (`011fa1f0` on, the colour table's rows through `own::store_light_row`) into
/// the packet, after the cut took the pass lights.
#[test]
fn light_setup_writes_the_draws_own_constants() {
    own_state_same(
        "light_setup",
        entry!(0x00b7_0820, lighting::light_setup(u32, Ptr<_>, f32)).1,
        &product(&[&[0, 3], &[0, 1], &[0, 1, 9], &[0x154, 0x30]]),
        |e, c| {
            let g = light_check_world(e, c.word(1), c.word(3), c.word(2));
            let acc = e.mem.alloc(0x200);
            (vec![c.word(0), g.light, HALF], acc)
        },
        |e, _, _| cut::take_pass_lights(e, e.mem.u32(0x011f_91e0)),
    );
}

/// The geometry's lights (`00b78a90`) set each pass light up and read the rows back from the packet; the shader
/// mode (`011f91b8`, own state) 0xf takes the held accumulator's ambient instead.
#[test]
fn lights_write_the_draws_own_constants() {
    own_state_same(
        "lights",
        geometry::lights_words as AbiFn,
        &product(&[&[0, 1, 3], &[0, 0x10, 0x30], &[0, 1], &[0, 0xf]]),
        |e, c| {
            let g = light_check_world(e, c.word(0), c.word(1), c.word(2));
            set(e, 0x011f_91b8, c.word(3));
            let mut words = vec![g.property, g.rendering];
            words.extend(matrix_words(&g));
            words.extend([g.transform(), g.pass_type, g.skin]);
            (words, e.mem.alloc(0x200))
        },
        |e, _, _| cut::take_pass_lights(e, e.mem.u32(0x011f_91e0)),
    );
}

/// The shader manager's mode (`00b4f450`, typed: `own::select_shader_mode`) sets the mode and its two function
/// words in the packet: a mode with entries in both tables, one without (the tables' first entries), past them.
#[test]
fn select_shader_mode_keeps_the_draws_own_state() {
    own_state_same(
        "select_shader_mode",
        entry!(0x00b4_f450, own::select_shader_mode(u32)).1,
        &product(&[&[0, 3, 9, 0xf]]),
        |e, c| {
            let _ = world(e);
            for k in 0..16u32 {
                set(e, 0x011f_9f40 + 4 * k, if k == 9 { 0 } else { 0x4400 + k });
                set(e, 0x011f_9f80 + 4 * k, if k == 9 { 0 } else { 0x8800 + k });
            }
            (vec![c.word(0)], e.mem.alloc(0x200))
        },
        |_, _, _| {},
    );
}

/// `setup` with an accumulator of its own (the Group C set-ups take none): the own-state check's form.
fn with_acc(
    setup: fn(&mut Engine, &Case) -> Vec<u32>,
) -> impl Fn(&mut Engine, &Case) -> (Vec<u32>, u32) {
    move |e, c| {
        let words = setup(e, c);
        (words, e.mem.alloc(0x200))
    }
}

/// The lighting 3.0 view matrix (`00bbe6b0`) copies the view and the projection (`own::copy_constants`, both
/// branches: the view's entry on, the projection's entry on with the flag) into Group C's block in the packet,
/// and reads its entries' slots from it.
#[test]
fn lighting30_view_matrix_writes_the_draws_own_constants() {
    own_state_same_floats(
        "lighting30::view_matrix",
        entry!(
            0x00bb_e6b0,
            engine_typed::draw::lighting30::view_matrix(Ptr<_>, Ptr<_>, u32, u32, Ptr<_>)
        )
        .1,
        &product(&[
            &[0, u32::MAX, 0x0000_0040],
            &[0, 2],
            &[0x1bd, 0x2d8, 0x1c4],
            TWO_WAYS,
            &[ONE],
        ]),
        &[4],
        with_acc(lighting30_view_matrix_words),
        |e, acc, w| {
            // The pass's lights as the cut takes them (lighting 3.0 reads them through the packet: value packet B).
            cut::take_pass_lights(e, w[1]);
            cut::take_frame(e, 0, Ptr::new(acc));
        },
    );
}

/// The lighting 3.0 switches (`00bbe120`) read the pass masks and the entries' slots from Group C's block in
/// the packet and write its flag bytes there.
#[test]
fn lighting30_switches_keep_the_draws_own_state() {
    own_state_same_floats(
        "lighting30::setup_switches",
        entry!(
            0x00bb_e120,
            engine_typed::draw::lighting30::setup_switches(Ptr<_>, u32)
        )
        .1,
        &product(&[
            &[0x1e0, 10, 0x1d3, 0x2da, 0x2e7, 0x1c4],
            &[0, u32::MAX, 0x0555_5555],
            &[0, 2],
            &[ONE],
            &[0x4120_0000],
        ]),
        &[3, 4],
        with_acc(lighting30_switches_words),
        |e, acc, _| cut::take_frame(e, 0, Ptr::new(acc)),
    );
}

/// The no-lighting shader's sub-texture copy (`00bc6fa0`) copies into Group C's block in the packet.
#[test]
fn no_lighting_subtextures_go_to_the_draws_own_constants() {
    own_state_same(
        "no_lighting::copy_subtextures",
        entry!(
            0x00bc_6fa0,
            engine_typed::draw::no_lighting::copy_subtextures(Ptr<_>, Ptr<_>, u32)
        )
        .1,
        &product(&[&[0, 1, 5]]),
        with_acc(no_lighting_subtexture_words),
        |e, _, w| {
            // The sub-textures as the cut takes them for a no-lighting geometry: the copy reads them through the
            // packet (value packet B).
            let data = e.mem.u32(w[1] + 0xb8);
            capture::take(e, data, 0x68, 0x6d);
            let count = u32::from(e.mem.u8(data + 0x6c));
            capture::take(e, e.mem.u32(data + 0x68), 0, count << 4);
        },
    );
}

/// The pass constants with vertex entry 8 on write the fog constants and the fog colour's copies into the draw's
/// own shader statics (the fog read through the packet the frame's values fill).
#[test]
fn setup_pass_constants_writes_the_draws_own_constants() {
    own_state_same(
        "setup_pass_constants",
        entry!(0x00b7_b450, setups::setup_pass_constants(Ptr<_>, u32)).1,
        &product(&[
            &[0x10, 0x2f0],
            &[0x0000_0200, 0xffff_ffff],
            &[1, 2, 3],
            TWO_WAYS,
        ]),
        |e, c| {
            let p = pass_world(e, &[], 0, 0);
            for addr in SETUP_CALLEES {
                e.register(*addr, marked);
            }
            entries(e);
            let pass_type = c.word(0);
            set(e, 0x011f_cc80 + 4 * pass_type, c.word(1));
            set(e, 0x011f_c0a0 + 4 * pass_type, c.word(1).rotate_left(3));
            e.map(0x010a_d000, 0x1000);
            set(e, 0x010a_d6e0, 0x48f4_2400);
            e.mem.set_u8(0x011f_91c4, 1);
            let node = e.mem.alloc(0x140);
            set(e, 0x011f_91c8 + 4, node);
            let fog = e.mem.alloc(0x64);
            for (k, word) in [ONE, HALF, 0x3e80_0000].iter().enumerate() {
                set(e, fog + 0x20 + 4 * k as u32, *word);
            }
            match c.word(2) {
                2 => {
                    set(e, fog + 0x2c, 0x3f40_0000);
                    set(e, fog + 0x30, TWO);
                    set(e, fog + 0x60, 0x3f40_0000);
                }
                3 => set(e, fog + 0x2c, ONE),
                _ => {}
            }
            set(e, node + 0x134, fog);
            let geometry = e.mem.u32(e.mem.u32(0x011f_91e0));
            let skin = if c.word(3) != 0 { p.d.w.skin } else { 0 };
            set(e, geometry + 0xbc, skin);
            set(e, p.d.w.shader + 0x18, p.d.w.renderer);
            set(e, RETS + 0x7f8, p.d.w.camera);
            (vec![p.d.w.shader, pass_type], p.d.w.acc)
        },
        |e, acc, _| {
            let camera = e.mem.u32(RETS + 0x7f8);
            cut::take_frame(e, camera, Ptr::new(acc));
        },
    );
}

/// The device's `SetRenderState` (slot 0xe4), `(device, state, value)`: counted at [`CONSTANT_UPLOAD_COPY`] + 0x1f00, the
/// state and value kept after it; then the slot's answer.
fn set_render_state_answer(e: &mut Engine, a: &[u32]) -> Ret {
    let at = CONSTANT_UPLOAD_COPY + 0x1f00;
    let n = e.mem.u32(at);
    e.mem.set_u32(at, n + 1);
    e.mem.set_u32(at + 4, a[1]);
    e.mem.set_u32(at + 8, a[2]);
    slot_answer::<0xe4>(e, a)
}

/// Where [`constants_upload_answer`] copies what the device is handed (one 0x10-byte row a register).
const CONSTANT_UPLOAD_COPY: u32 = 0x0121_0000;

/// The device's `SetVertexShaderConstantF` / `SetPixelShaderConstantF` (slots 0x178, 0x1b4), `(device, register,
/// data, count)`: the rows the data points to copied to [`CONSTANT_UPLOAD_COPY`] at the register's row (the
/// vertex rows, then the pixel rows 0x100 on), so the digest compares what was uploaded; then the slot's answer.
fn constants_upload_answer<const S: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    let (register, data, count) = (a[1], a[2], a[3].min(4));
    let base = CONSTANT_UPLOAD_COPY + if S == 0x1b4 { 0x1000 } else { 0 };
    let mut bytes = vec![0u8; 16 * count as usize];
    if register < 0x100 && e.mem.try_read(data, &mut bytes).is_ok() {
        e.mem.write(base + 16 * register, &bytes);
    }
    slot_answer::<S>(e, a)
}

/// The constant map's uploads (`00e826d0`) hand the device the draw's own constants: with the entries' data in
/// the shader statics (scribbled in guest memory), the device gets the rows the draw holds (an image of the
/// packet's copy).
#[test]
fn set_shader_constants_uploads_the_draws_own_constants() {
    own_state_same(
        "set_shader_constants",
        entry!(
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
        )
        .1,
        &product(&[&[0, 1, 3, 6], &[0, 1, 2, 3], &[0, 1]]),
        |e, c| {
            stand_ins(e, false);
            slots!(e; 0x90, 0x94, 0x98, 0x9c);
            e.register(VT_FN + 0x178, constants_upload_answer::<0x178>);
            e.register(VT_FN + 0x1b4, constants_upload_answer::<0x1b4>);
            e.map(CONSTANT_UPLOAD_COPY, 0x2000);
            for slot in [0x8c, 0x90, 0x94, 0x98, 0x9c, 0xa4] {
                set(e, RETS + slot, 1);
            }
            let vt = vtable(e, &[]);
            let device = obj(e, 0x10, vt);
            let map = obj(e, 0x44, vt);
            let entries = constant_entries(e, c.word(0), c.word(1), c.word(2), false);
            let defined = constant_entries(e, c.word(0) + 1, c.word(1) + 1, c.word(2) + 2, false);
            // Each entry's data in the shader statics' constant rows, the rows filled.
            for (list, base) in [(entries, 0x011f_a0c0u32), (defined, 0x011f_d868)] {
                for i in 0..10u32 {
                    let entry = e.mem.u32(list + 4 * i);
                    set(e, entry + 0x30, base + 0x30 * i);
                }
            }
            for k in 0..0x100u32 {
                set(e, 0x011f_a0c0 + 4 * k, 0x3f80_0000 + 0x101 * k);
                set(e, 0x011f_d868 + 4 * k, 0x4000_0000 + 0x303 * k);
            }
            set(e, map + 0x10, entries);
            e.mem.set_u16(map + 0x16, 10);
            set(e, map + 0x20, defined);
            e.mem.set_u16(map + 0x26, 10);
            set(e, map + 0x38, device);
            let mut words = vec![map];
            words.extend((0..11).map(|i| 0x0700_0000 + 0x100 * i));
            (words, 0)
        },
        |_, _, _| {},
    );
}

/// The lighting 3.0 shader's geometry set-up (`00bbef90`) with its typed helpers reads the geometry's world
/// transform (through the packet's image of it), its bound, its pass's lights and the game time through the packet:
/// with the cut's copies scribbled in guest memory (POISON), it answers and writes as without the packet. A plain
/// pass type, a shadow pass type (the clip planes) and the 0xe set-up; every constant entry on.
#[test]
fn lighting30_setup_constants_reads_the_packet() {
    packet_same_cases(
        "lighting30_setup_constants",
        entry!(
            0x00bb_ef90,
            engine_typed::draw::lighting30::setup_constants(Ptr<_>, Ptr<_>)
        )
        .1,
        &word_cases(&product(&[&[0x30, 0x1c4, 0xe], &[3, 0x0100_0001]])),
        true,
        |e, c| {
            let case = Case(vec![
                Value::Word(c.word(0)),
                Value::Word(0),
                Value::Word(u32::MAX),
                Value::Word(0),
                Value::Word(1),
                Value::Word(0),
                Value::F32(0x4561_2340),
                Value::Word(c.word(1)),
                Value::Word(0),
            ]);
            let words = lighting30_constants_words(e, &case);
            let pass = e.mem.u32(0x011f_91e0);
            let geometry = e.mem.u32(pass);
            as_drawn(e, geometry, words[1]);
            d3dx_content_stand_ins(e);
            (words, vec![pass, geometry])
        },
        |e, w| {
            cut::take_geometry_values(e, w[1]);
            cut::take_pass_lights(e, w[0]);
            capture::take(e, 0x011f_940c, 0, 0x10);
            capture::take(e, 0x011f_94ac, 0, 4);
        },
    );
}

/// The geometry as the world draw has it: a vtable (the cut asks its immediate draw), and its shader property
/// (+0xa8) the state's (`block`'s +0xc), when the setup left them out.
fn as_drawn(e: &mut Engine, geometry: u32, block: u32) {
    if e.mem.u32(geometry) == 0 {
        let vt = vtable(e, &[]);
        set(e, geometry, vt);
    }
    if e.mem.u32(geometry + 0xa8) == 0 {
        let property = e.mem.u32(block + 0xc);
        set(e, geometry + 0xa8, property);
    }
}

/// Cases of a test's inputs from their raw words: a float input's word as a float's bits, the others as words.
fn cases_of(inputs: &[Input], picks: &[&[u32]]) -> Vec<Case> {
    picks
        .iter()
        .map(|words| {
            assert_eq!(words.len(), inputs.len());
            Case(
                inputs
                    .iter()
                    .zip(words.iter())
                    .map(|(input, w)| match input {
                        Input::F32 | Input::F32Swept(_) => Value::F32(*w),
                        _ => Value::Word(*w),
                    })
                    .collect(),
            )
        })
        .collect()
}

/// A Group C set-up (`thiscall(shader, state)`) with its test's setup: with what the cut takes for the rendered
/// pass (its geometry's values, its lights) and the time values scribbled in guest memory (POISON), it answers and
/// writes as without the packet.
fn set_up_reads_the_packet(
    name: &str,
    f: AbiFn,
    inputs: &[Input],
    picks: &[&[u32]],
    setup: fn(&mut Engine, &Case) -> Vec<u32>,
) {
    packet_same_cases(
        name,
        f,
        &cases_of(inputs, picks),
        true,
        |e, case| {
            let words = setup(e, case);
            let pass = e.mem.u32(0x011f_91e0);
            let geometry = e.mem.u32(pass);
            as_drawn(e, geometry, words[1]);
            (words, vec![pass, geometry])
        },
        take_drawn,
    );
}

/// What the cut takes for the rendered pass `w[0]` and its geometry `w[1]` (its world transform, bound and shader
/// property values: the cut's `take_geometry` without the skin, which these setups only mark, and particles) and
/// for the frame (no camera or accumulator: the empty bound, the fog, the time values).
fn take_drawn(e: &mut Engine, w: &[u32]) {
    let geometry = w[1];
    capture::take(e, geometry, 0x68, 0x9c);
    let bound = e.mem.u32(geometry + 0x20);
    capture::take(e, bound, 0, 0x10);
    let property = e.mem.u32(geometry + 0xa8);
    capture::take(e, property, 0x24, 0x38);
    cut::take_pass_lights(e, w[0]);
    cut::take_frame(e, 0, Ptr::new(0));
}

/// The sky's slot 0x7c reads the geometry's world transform through the packet's image (the renderer's world
/// transform from it) and its copy when the held accumulator skips type 13.
#[test]
fn sky_setup_constants_reads_the_packet() {
    set_up_reads_the_packet(
        "sky_setup_constants",
        entry!(
            0x00b8_9d80,
            engine_typed::draw::sky::setup_constants(Ptr<_>, Ptr<_>)
        )
        .1,
        SKY_CONSTANTS,
        &[
            &[
                1,
                0,
                5,
                3,
                1,
                ONE,
                0x4500_0000,
                0x4396_0000,
                1,
                1,
                0x4580_0000,
                0x4480_0000,
            ],
            &[
                0,
                1,
                6,
                2,
                0,
                HALF,
                0x4500_0000,
                0x4396_0000,
                0,
                2,
                0x4580_0000,
                0x4480_0000,
            ],
        ],
        sky_constants_words,
    );
}

/// The tall grass's slot 0x7c hands its `SetupGeometry` the packet's image of the transform and reads the
/// game time and the property's second flags through the packet.
#[test]
fn tall_grass_setup_constants_reads_the_packet() {
    packet_same_cases(
        "tall_grass_setup_constants",
        entry!(
            0x00ba_c9c0,
            engine_typed::draw::tall_grass::setup_constants(Ptr<_>, Ptr<_>)
        )
        .1,
        &cases_of(
            TALL_GRASS_CONSTANTS,
            &[
                &[
                    1,
                    0,
                    1,
                    0x4561_2340,
                    0x4040_0000,
                    1,
                    1,
                    2,
                    0,
                    0x3fc0_0000,
                    2,
                ],
                &[0, 1, 1, HALF, 0x4040_0000, 0, 0, 3, 3, HALF, 1],
            ],
        ),
        true,
        |e, case| {
            let words = tall_grass_constants_words(e, case);
            let pass = e.mem.u32(0x011f_91e0);
            let geometry = e.mem.u32(pass);
            as_drawn(e, geometry, words[1]);
            (words, vec![pass, geometry])
        },
        |e, w| {
            take_drawn(e, w);
            // The tall grass arrays as the cut takes them for a geometry of the tall grass shader (its property's
            // instance data, the data's counts and array, the array).
            let property = e.mem.u32(w[1] + 0xa8);
            capture::take(e, property, 0x7c, 0x80);
            let instances = e.mem.u32(property + 0x7c);
            capture::take(e, instances, 0x10, 0x18);
            let count = u32::from(e.mem.u16(instances + 0x10));
            capture::take(e, e.mem.u32(instances + 0x14), 0, count << 4);
        },
    );
}

/// The no-lighting slot 0x7c: the renderer's world transform from the packet's image, the property's alpha
/// through the packet (the material constants); a skinned geometry (an identity) and the sub-texture pass types.
#[test]
fn no_lighting_setup_constants_reads_the_packet() {
    set_up_reads_the_packet(
        "no_lighting_setup_constants",
        entry!(
            0x00bc_9a50,
            engine_typed::draw::no_lighting::setup_constants(Ptr<_>, Ptr<_>)
        )
        .1,
        NO_LIGHTING_CONSTANTS,
        &[
            &[0, 0x42, 0x40, 1, 1, 1, 1],
            &[1, 0x2d8, 0x8000_0000, 0, 2, 0, 0],
        ],
        no_lighting_constants_words,
    );
}

/// The water's slot 0x7c: the renderer's world transform from the packet's image.
#[test]
fn water_setup_constants_reads_the_packet() {
    set_up_reads_the_packet(
        "water_setup_constants",
        entry!(
            0x00b8_64d0,
            engine_typed::draw::water::setup_constants(Ptr<_>, Ptr<_>)
        )
        .1,
        WATER_CONSTANTS,
        &[
            &[0x30, 1, 1, ONE, 0x4500_0000, 0x3f40_0000, 1, 3, 1],
            &[
                0x288,
                0,
                0,
                HALF,
                0x4500_0000,
                0x3f40_0000,
                2,
                0x8000_0001,
                0,
            ],
        ],
        water_constants_words,
    );
}

/// `SetupGeometryConstants` (`00b7dab0`) with its set-ups: the transform through the packet's image (the model
/// matrix, the world-view constants and every set-up handed it), the lights and the time values through the
/// packet, the refraction pass type's geometry translation (vertex entry 21) too. The cut's geometry values take the
/// world transform, so the shadow projection, the eye position (pass type 0x244) and the object-to-cube's
/// `GetD3DFromNi` must read the image they are handed.
#[test]
fn setup_geometry_constants_reads_the_packet() {
    let mut cases = Vec::new();
    for pass_type in [0u32, 0x230, 0xfe, 6, 0x244] {
        for lights in [1u32, 2] {
            cases.push(vec![pass_type, lights]);
        }
    }
    packet_same_cases(
        "setup_geometry_constants",
        entry!(
            0x00b7_dab0,
            geometry::setup_geometry_constants(Ptr<_>, Ptr<_>)
        )
        .1,
        &word_cases(&cases),
        true,
        |e, c| {
            let mut s = Shape::plain();
            s.pass_type = c.word(0);
            s.lights = c.word(1);
            s.steps = c.word(1);
            let g = geometry_shape(e, &s);
            as_drawn(e, g.geometry, g.block);
            (vec![g.shader, g.block], vec![g.rendering, g.geometry])
        },
        |e, w| {
            cut::take_geometry_values(e, w[1]);
            cut::take_pass_lights(e, w[0]);
            // The frame's values (the empty bound, the fog, the time values).
            cut::take_frame(e, 0, Ptr::new(0));
        },
    );
}

/// The alpha sort's comparison (`00b98b20`) reads the camera's direction and each pass's bound through the
/// packet.
#[test]
fn compare_by_depth_reads_the_packet() {
    packet_same(
        "compare_by_depth",
        entry!(0x00b9_8b20, batch::compare_by_depth(u32, u32) -> i32).1,
        &product(&[&[0, 1], &[0, 1]]),
        |e, c| {
            let w = world(e);
            let camera = full_camera(e, &w);
            let vt = vtable(e, &[]);
            let mut items = [0u32; 2];
            let mut geometries = [0u32; 2];
            for (k, item) in (0u32..).zip(items.iter_mut()) {
                let bound = e.mem.alloc(0x10);
                for (i, v) in (0u32..).zip([0x4000_0000 + k, TWO, 0x4100_0000 - k, HALF]) {
                    set(e, bound + 4 * i, v);
                }
                let geometry = obj(e, 0xc4, vt);
                set(
                    e,
                    geometry + 0x20,
                    if c.word(1) != 0 && k == 1 { 0 } else { bound },
                );
                geometries[k as usize] = geometry;
                let pass = e.mem.alloc(8);
                set(e, pass, geometry);
                *item = e.mem.alloc(4);
                set(e, *item, pass);
            }
            e.mem.set_u8(0x011f_fe35, c.word(0) as u8);
            set(e, 0x011f_fe38, camera);
            (
                vec![items[0], items[1]],
                vec![camera, geometries[0], geometries[1]],
            )
        },
        |e, w| {
            cut::take_camera(e, w[0]);
            cut::take_geometry_values(e, w[1]);
            cut::take_geometry_values(e, w[2]);
            // The empty bound, as the cut takes it for the frame.
            capture::take(e, 0x011f_4288, 0, 0x10);
        },
    );
}

/// The tall grass's slot 0x78 reads the rendered pass's first light (its shadow map's owner) through the
/// packet, in the shadow pass types.
#[test]
fn tall_grass_setup_textures_reads_the_packet() {
    packet_same(
        "tall_grass_setup_textures",
        entry!(
            0x00ba_c950,
            engine_typed::draw::tall_grass::setup_textures(Ptr<_>, Ptr<_>)
        )
        .1,
        &[vec![0x284, 0x7e57_0000], vec![0x287, 0]],
        |e, c| {
            let words = tall_grass_textures_words(e, c);
            let record = e.mem.u32(0x011f_91e0);
            // One light, as a shadow pass has.
            e.mem.set_u8(record + 9, 1);
            (words, vec![record])
        },
        |e, w| cut::take_pass_lights(e, w[0]),
    );
}

/// A geometry of the drawn size for the Group C copy tests: `from`'s words (when not 0), no skin, `property` as its
/// shader property (+0xa8), and a shader whose class word is `class` (+0xc0: what the cut's walk asks to know which
/// arrays to take).
fn group_c_geometry(e: &mut Engine, from: u32, property: u32, class: u32) -> u32 {
    let geometry = e.mem.alloc(0xc4);
    if from != 0 {
        for k in 0..0xc0 / 4 {
            let word = e.mem.u32(from + 4 * k);
            set(e, geometry + 4 * k, word);
        }
    }
    let vt = vtable(e, &[]);
    set(e, geometry, vt);
    set(e, geometry + 0xbc, 0);
    set(e, geometry + 0xa8, property);
    let shader = e.mem.alloc(4);
    set(e, shader, class);
    set(e, geometry + 0xc0, shader);
    geometry
}

/// The tall grass's instance copy (`00baaf40`) reads the instance data, its counts and its array through the
/// packet and copies from the packet's image of the array: with what the cut takes for the drawn geometry (its
/// shader of the tall grass class) scribbled in guest memory (POISON), it copies and writes as without.
#[test]
fn tall_grass_copy_instances_reads_the_packet() {
    packet_same_cases(
        "tall_grass_copy_instances",
        entry!(
            0x00ba_af40,
            engine_typed::draw::tall_grass::copy_instances(Ptr<_>, u32, Ptr<_>)
        )
        .1,
        &word_cases(&[vec![3, 0, 1], vec![1, 1, 0], vec![5, 2, 1]]),
        true,
        |e, c| {
            let words = tall_grass_instances_words(e, c);
            let geometry = group_c_geometry(e, 0, words[2], 0x010b_8980);
            (words, vec![geometry])
        },
        |e, w| cut::take_geometry_values(e, w[0]),
    );
}

/// The no-lighting sub-texture copy (`00bc6fa0`) reads the count and the array through the packet and copies from
/// the packet's image of the entries: with what the cut takes for the geometry (its shader of the no-lighting
/// class) scribbled in guest memory (POISON), it copies and writes as without.
#[test]
fn no_lighting_copy_subtextures_reads_the_packet() {
    packet_same_cases(
        "no_lighting_copy_subtextures",
        entry!(
            0x00bc_6fa0,
            engine_typed::draw::no_lighting::copy_subtextures(Ptr<_>, Ptr<_>, u32)
        )
        .1,
        &word_cases(&[vec![1], vec![5]]),
        true,
        |e, c| {
            let w = world(e);
            e.map(0x0120_2000, 0x1000);
            let shader = no_lighting_shader(e, &w);
            let entries = e.mem.alloc(0x50);
            for i in 0..0x14u32 {
                set(e, entries + 4 * i, 0x5b00_0000 + i);
            }
            let data = e.mem.alloc(0x70);
            set(e, data + 0x68, entries);
            e.mem.set_u8(data + 0x6c, c.word(0) as u8);
            let geometry = group_c_geometry(e, 0, 0, 0x010b_b230);
            set(e, geometry + 0xb8, data);
            (vec![shader, geometry, 0x0bad_0002], vec![geometry])
        },
        |e, w| cut::take_geometry_values(e, w[0]),
    );
}

/// The SpeedTree leaves' slot 0x7c (`00bb2760`) with a leaf property of its class (`010b9700`): its leaf data
/// (slot 0xfc, +0x88) and leaves (slot 0x100, the data's +8) read through the packet without the calls, the 0x300
/// bytes copied from the packet's image: with what the cut takes for the rendered geometry (its shader of the leaf
/// class), the pass's lights and the frame scribbled in guest memory (POISON), it answers and writes as without.
#[test]
fn leaf_setup_constants_reads_the_leaves_through_the_packet() {
    packet_same_cases(
        "leaf_setup_constants",
        entry!(
            0x00bb_2760,
            engine_typed::draw::speedtree::setup_constants(Ptr<_>, Ptr<_>)
        )
        .1,
        &cases_of(
            LEAF_CONSTANTS,
            &[&[1, 2, 1, 9, ONE, 0], &[0, 0, 0, 8, HALF, 0]],
        ),
        true,
        |e, case| {
            let words = leaf_constants_words(e, case);
            // The leaf property of its class (a block large enough for its +0x88), its leaf data (the slot 0xfc
            // answer) holding the leaves (slot 0x100's).
            let old = e.mem.u32(words[1] + 0xc);
            let property = e.mem.alloc(0x90);
            for k in 0..0x80 / 4 {
                let word = e.mem.u32(old + 4 * k);
                set(e, property + 4 * k, word);
            }
            set(e, words[1] + 0xc, property);
            let leaf_data = e.mem.u32(RETS + 0xfc);
            let leaves = e.mem.u32(RETS + 0x100);
            set(e, leaf_data + 8, leaves);
            set(e, property + 0x88, leaf_data);
            set(e, property, 0x010b_9700);
            let record = e.mem.u32(0x011f_91e0);
            let from = e.mem.u32(record);
            let geometry = group_c_geometry(e, from, property, 0x010b_9190);
            set(e, record, geometry);
            (words, vec![record, geometry])
        },
        |e, w| {
            cut::take_geometry_values(e, w[1]);
            cut::take_pass_lights(e, w[0]);
            cut::take_frame(e, 0, Ptr::new(0));
        },
    );
}

/// The D3DX imports the lighting 3.0 helpers call, each answering from what its inputs hold (as
/// [`geometry_stand_ins`] does) rather than marking the call by its words: for the packet tests, where a helper
/// handed the packet's image and one handed guest memory get the same words but must read the same values. The
/// plane normalize (`00fdf430`) and transform (`00fdf418`), the transpose (`00fdf42c`), the multiply (`00fdf424`)
/// and the inverse (`00fdf428`).
fn d3dx_content_stand_ins(e: &mut Engine) {
    e.register(0x00fd_f430, |e, a| {
        let v = words_at(e, a[1], 4);
        for k in 0..4u32 {
            e.mem.set_u32(a[0] + 4 * k, v[k as usize]);
        }
        ret(a[0])
    });
    e.register(0x00fd_f418, |e, a| {
        let v = words_at(e, a[1], 4);
        let m = words_at(e, a[2], 16);
        for k in 0..4u32 {
            e.mem
                .set_u32(a[0] + 4 * k, v[((k + 1) % 4) as usize] ^ (m[k as usize] & 0xff));
        }
        ret(a[0])
    });
    e.register(0x00fd_f42c, |e, a| {
        let m = words_at(e, a[1], 16);
        for k in 0..16u32 {
            e.mem
                .set_u32(a[0] + 4 * k, m[(4 * (k % 4) + k / 4) as usize]);
        }
        ret(a[0])
    });
    e.register(0x00fd_f424, |e, a| {
        let (m, n) = (words_at(e, a[1], 16), words_at(e, a[2], 16));
        for k in 0..16u32 {
            let word = m[k as usize] ^ (n[((5 * k + 3) % 16) as usize] & 0xff);
            e.mem.set_u32(a[0] + 4 * k, word);
        }
        ret(a[0])
    });
    e.register(0x00fd_f428, |e, a| {
        let m = words_at(e, a[2], 16);
        for k in 0..16u32 {
            e.mem.set_u32(a[0] + 4 * k, m[(15 - k) as usize]);
        }
        ret(a[0])
    });
}
