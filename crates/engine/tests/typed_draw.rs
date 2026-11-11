//! The differential test of the draw family's geometry paths (`engine_typed::draw`'s `shapes`,
//! `packed`, `skin` and `packing`, group D) against their translations, and of the call sites the
//! shader and render families make direct to them through a vtable slot.
//!
//! Every family the paths call is on (draw, render, shader, leaves, dispatch with what it calls,
//! crt), as in play. What stands in: every virtual function the code calls on the renderer, the
//! device, the render state, a geometry, its data, a property, a shader, a declaration, a group or a
//! Direct3D buffer (test functions past `.text` answering a word per slot from a table), and the
//! callees outside the families (the buffer data's stream count and declaration, the index buffer
//! makers, the point rotation, the message function, the vertex shader's entry point, the allocators
//! and frees, the critical section imports, the arrays' and maps' set and get, the chip and record
//! makers, the entry and vertex packers). Each stand-in leaves a mark in memory, so a call moved
//! across a store shows.

use engine as _;
use engine_core::prelude::*;
use engine_core::typed::diff::{Case, Diff, Input};
use engine_core::typed::leaf;
use engine_core::{entry, Engine};
use engine_typed::draw::{self, packed, packing, shapes, skin};
use engine_typed::render::{self, stream};
use engine_typed::{crt, culling, dispatch, pathing, shader};

/// The stand-in virtual functions, one per slot, and the table of what each answers.
const VT_FN: u32 = 0x00ff_2000;
const RETS: u32 = 0x0300_0000;
/// The marks the stand-ins leave.
const MARK: u32 = RETS + 0x500;
/// Test functions with their own behaviour: a buffer's `Lock` that hands out a block, a declaration's
/// slot 0xb0 that fills its two words, the point rotation, an allocator.
const LOCK_FN: u32 = 0x00ff_3000;
const WORDS_FN: u32 = 0x00ff_3010;
const CREATE_FN: u32 = 0x00ff_3020;
const MAX_CALLS: usize = 20_000;

const ONE: u32 = 0x3f80_0000;
const TWO: u32 = 0x4000_0000;
const HALF: u32 = 0x3f00_0000;

const TWO_WAYS: &[u32] = &[0, 1];

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// A mark of the call (its word count and words) folded into memory.
fn mark(e: &mut Engine, a: &[u32]) {
    let mut n = e.mem.u32(MARK);
    n = n.wrapping_mul(31).wrapping_add(a.len() as u32);
    for w in a {
        n = n.wrapping_mul(17).wrapping_add(*w);
    }
    e.mem.set_u32(MARK, n);
}

fn slot_answer<const S: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    mark(e, a);
    ret(e.mem.u32(RETS + S))
}

/// A stand-in callee answering the table word at `RETS + A`.
fn answer_at<const A: u32>(e: &mut Engine, a: &[u32]) -> Ret {
    mark(e, a);
    ret(e.mem.u32(RETS + A))
}

macro_rules! slots {
    ($e:ident; $($s:literal),* $(,)?) => { $( $e.register(VT_FN + $s, slot_answer::<$s>); )* };
}

fn set(e: &mut Engine, at: u32, value: u32) {
    e.mem.set_u32(at, value);
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

/// The stand-ins and the exe's globals the paths read.
fn stand_ins(e: &mut Engine) {
    e.map(RETS, 0x1000);
    slots!(e; 0x04, 0x08, 0x18, 0x1c, 0x28, 0x2c, 0x30, 0x34, 0x70, 0x78, 0x7c, 0x80, 0x84, 0x88,
        0x8c, 0x90, 0x94, 0xa0, 0xa8, 0xac, 0xb0, 0xb4, 0xbc, 0xd4, 0xe0, 0xf8, 0x144, 0x148, 0x158,
        0x190, 0x1a0, 0x1b4, 0x1b8, 0x1bc);
    // The buffer data's stream count (00e8ef10) and declaration (00e8f040), the index buffer makers
    // (00e8f360, 00e8fc50), the message function, the entry point, the second matrix builder, the
    // arrays' and maps' set and get, the record's lock and the chip maker, the element adder, the
    // stream's chip maker, the packers, RemoveIB.
    e.register(0x00e8_ef10, answer_at::<0x600>);
    e.register(0x00e8_f040, answer_at::<0x604>);
    e.register(0x00e8_f360, answer_at::<0x608>);
    e.register(0x00e8_fc50, answer_at::<0x60c>);
    e.register(0x00a2_9680, answer_at::<0x610>);
    e.register(0x00e8_bf90, answer_at::<0x614>);
    e.register(0x00e6_b8f0, answer_at::<0x618>);
    e.register(0x00a9_0180, answer_at::<0x61c>);
    e.register(0x00ba_3aa0, answer_at::<0x620>);
    e.register(0x00b5_7ea0, answer_at::<0x624>);
    e.register(0x0053_7d60, answer_at::<0x62c>);
    e.register(0x00e7_6660, answer_at::<0x630>);
    e.register(0x00e6_9b40, answer_at::<0x634>);
    e.register(0x00e7_a200, answer_at::<0x638>);
    e.register(0x00a8_55f0, answer_at::<0x63c>);
    e.register(0x00e8_eee0, answer_at::<0x640>);
    e.register(0x00e8_c820, answer_at::<0x644>);
    e.register(0x00e8_f200, answer_at::<0x648>);
    e.register(0x00e8_c1e0, answer_at::<0x64c>);
    e.register(0x0043_b300, answer_at::<0x650>);
    e.register(0x00a7_b850, answer_at::<0x654>);
    e.register(0x00e6_d760, answer_at::<0x658>);
    // The map's `GetAt` (00a53c30): found as the table says, the value written through its third word.
    e.register(0x00a5_3c30, |e, a| {
        mark(e, a);
        let value = e.mem.u32(RETS + 0x65c);
        if value != 0 {
            e.mem.set_u32(a[2], value);
        }
        ret(u32::from(value != 0))
    });
    // The point rotation (004b3ae0, `cdecl(out, point, rotation)`): the point's words turned into
    // the out words, which it answers.
    e.register(0x004b_3ae0, |e, a| {
        mark(e, a);
        for i in 0..3 {
            let w = e.mem.u32(a[1] + 4 * i) ^ e.mem.u32(a[2] + 4 * i);
            e.mem.set_u32(a[0] + 4 * i, w);
        }
        ret(a[0])
    });
    // The allocators: a fresh block (00aa1070, 00aa10b0, 00aa13e0; 00aa3e40 on the manager) and the
    // frees (00aa10f0, 00aa1110, 00aa4060), each marked.
    e.register(0x00aa_1070, |e, a| {
        mark(e, a);
        ret(e.mem.alloc(a[0].clamp(4, 0x4000)))
    });
    e.register(0x00aa_10b0, |e, a| {
        mark(e, a);
        ret(e.mem.alloc(a[0].clamp(4, 0x4000)))
    });
    e.register(0x00aa_13e0, |e, a| {
        mark(e, a);
        ret(e.mem.alloc(a[0].clamp(4, 0x4000)))
    });
    e.register(0x00aa_3e40, |e, a| {
        mark(e, a);
        ret(e.mem.alloc(a[1].clamp(4, 0x4000)))
    });
    for addr in [
        0x00aa_10f0u32,
        0x00aa_1110,
        0x00aa_4060,
        0x00fd_f05c,
        0x00fd_f100,
    ] {
        e.register(addr, |e, a| {
            mark(e, a);
            ret(0)
        });
    }
    // A Direct3D buffer's `Lock`: hands out the block at `RETS + 0x700` through the third word (none
    // when it is 0) and answers the table's result.
    e.register(LOCK_FN, |e, a| {
        mark(e, a);
        let block = e.mem.u32(RETS + 0x700);
        e.mem.set_u32(a[3], block);
        ret(e.mem.u32(RETS + 0x704))
    });
    // The buffer record's lock (00e987c0, `thiscall(record, size, offset out, second out, flag, key)`): the
    // offset and the second word written, the locked buffer from the table.
    e.register(0x00e9_87c0, |e, a| {
        mark(e, a);
        e.mem.set_u32(a[2], 0x60);
        e.mem.set_u32(a[3], 0x0b0f_0003);
        ret(e.mem.u32(RETS + 0x628))
    });
    // `CreateVertexDeclaration` (`(device, elements, out)`): the out word written, the answer from the
    // table.
    e.register(CREATE_FN, |e, a| {
        mark(e, a);
        e.mem.set_u32(a[2], 0x0dec_0158);
        ret(e.mem.u32(RETS + 0x158))
    });
    // A declaration's slot 0xb0: both words written, the answer from the table.
    e.register(WORDS_FN, |e, a| {
        mark(e, a);
        let first = e.mem.u32(RETS + 0x708);
        e.mem.set_u32(a[1], first);
        e.mem.set_u32(a[2], 0x2468);
        ret(u32::from(first != 0))
    });
    for (addr, len) in [
        (0x0101_0000, 0x10000),
        (0x0106_b000, 0x3000),
        (0x011b_f000, 0x1000),
        (0x011f_4000, 0xc000),
        (0x0126_f000, 0x1000),
    ] {
        e.map(addr, len);
    }
    // 255.0 (the colour scale), the bone scale bounds and the origin the rows subtract.
    e.mem.set_u64(0x0101_e568, 255.0f64.to_bits());
    // The bone scale bounds as the exe has them: 0.99 (a double) and 1.01 (a float).
    e.mem.set_u64(0x0106_d508, 0x3fef_ae14_8000_0000);
    set(e, 0x0106_b9e0, 0x3f81_47ae);
    for (i, w) in [HALF, ONE, TWO].iter().enumerate() {
        set(e, 0x011f_474c + 4 * i as u32, *w);
    }
    set(e, 0x011b_f558, 1);
    for (i, w) in [ONE, TWO, HALF, 0x4040_0000].iter().enumerate() {
        set(e, 0x011f_4288 + 4 * i as u32, *w ^ 0x0010_0000);
    }
}

trait Common {
    fn common(self) -> Diff;
}

impl Common for Diff {
    /// Every family the paths reach, on.
    fn common(self) -> Diff {
        self.family(&draw::DRAW)
            .family(&render::RENDER)
            .family(&shader::SHADER)
            .family(&leaf::LEAVES)
            .family(&dispatch::DISPATCH)
            .family(&pathing::PATHING)
            .family(&culling::CULLING)
            .family(&crt::CRT)
            .max_calls(MAX_CALLS)
            .max_cases(240)
    }
}

/// The renderer: its device (+0x288), its draw mode and immediate byte, the vertex and index
/// managers (+0x8bc, +0x8c0), the render state (+0x8b8), the group manager and group (+0x8ac, +0x8b4)
/// and the camera vectors (+0x634, +0x640). `real` slots in its vtable.
struct Renderer {
    renderer: u32,
    vertex_manager: u32,
}

fn renderer_world(e: &mut Engine, real: &[(u32, u32)]) -> Renderer {
    let vt = vtable(e, &[]);
    let device = obj(e, 0x10, vt);
    let renderer_vt = vtable(e, real);
    let renderer = obj(e, 0xa40, renderer_vt);
    set(e, renderer + 0x288, device);
    let state = obj(e, 0x10, vt);
    set(e, renderer + 0x8b8, state);
    let vertex_manager = obj(e, 0x90, vt);
    set(e, renderer + 0x8bc, vertex_manager);
    let index_manager = obj(e, 0x10, vt);
    set(e, renderer + 0x8c0, index_manager);
    let manager = obj(e, 0x10, vt);
    let group = obj(e, 0x10, vt);
    set(e, renderer + 0x8ac, manager);
    set(e, renderer + 0x8b4, group);
    for (i, w) in [ONE, HALF, TWO, 0x3eaa_aaab, ONE, 0xbf80_0000]
        .iter()
        .enumerate()
    {
        set(e, renderer + 0x634 + 4 * i as u32, *w);
    }
    set(e, 0x011f_9508, renderer);
    Renderer {
        renderer,
        vertex_manager,
    }
}

// ---- The geometries' immediate draws. ----

/// A geometry with `controllers` controllers (each with its `OnPreDisplay` slot 0xa8 and the next at
/// +0x30) and the property state at +0x9c.
fn geometry_with(e: &mut Engine, size: u32, vt: u32, controllers: u32) -> u32 {
    let g = obj(e, size, vt);
    let stand_in = vtable(e, &[]);
    let mut next = 0;
    for _ in 0..controllers {
        let c = obj(e, 0x40, stand_in);
        set(e, c + 0x30, next);
        next = c;
    }
    set(e, g + 0xc, next);
    g
}

/// The renderer for an immediate draw: draw mode `mode`, immediate byte `immediate`, its draw slot
/// `slot` the typed draw (`real`) or a stand-in; with the real one the renderer skips drawing, so
/// the draw answers at once.
fn immediate_renderer(e: &mut Engine, slot: u32, real: u32, case: &Case, first: usize) -> u32 {
    let r = if case.word(first + 2) != 0 {
        renderer_world(e, &[(slot, real)])
    } else {
        renderer_world(e, &[])
    };
    set(e, r.renderer + 0x200, case.word(first));
    e.mem.set_u8(r.renderer + 0x208, case.word(first + 1) as u8);
    e.mem.set_u8(r.renderer + 0x6fc, 1);
    r.renderer
}

const IMMEDIATE_INPUTS: &[Input] = &[
    Input::Of(&[0, 1, 2, 3]),
    Input::Of(&[0, 1, 2]),
    Input::Of(TWO_WAYS),
    Input::Of(&[0, 2]),
];

#[test]
fn tri_shape_render_immediate() {
    Diff::new(entry!(
        0x00a7_4600,
        shapes::tri_shape_render_immediate(Ptr<_>, Ptr<_>)
    ))
    .common()
    .inputs(IMMEDIATE_INPUTS)
    .setup(|e, c| {
        stand_ins(e);
        let renderer = immediate_renderer(e, 0x1b4, 0x00e7_45a0, c, 0);
        let vt = vtable(e, &[]);
        let g = geometry_with(e, 0xc4, vt, c.word(3));
        vec![g, renderer]
    })
    .run()
    .assert_same();
}

#[test]
fn particles_render_immediate() {
    Diff::new(entry!(
        0x00a9_b1f0,
        shapes::particles_render_immediate(Ptr<_>, Ptr<_>)
    ))
    .common()
    .inputs(IMMEDIATE_INPUTS)
    .setup(|e, c| {
        stand_ins(e);
        let renderer = immediate_renderer(e, 0x1bc, 0x00e7_0140, c, 0);
        let vt = vtable(e, &[]);
        let g = geometry_with(e, 0xc4, vt, c.word(3));
        vec![g, renderer]
    })
    .run()
    .assert_same();
}

/// A segmented shape of `count` segments: segment `i` enabled when bit `i` of `enabled` is set, with
/// primitives (3 + i) when bit `i` of `primitives` is; its start 10·i + 1, the merged count and state
/// left from before. The array has one segment's room before it, which a shape of no segments merges.
fn segmented(
    e: &mut Engine,
    vt: u32,
    count: u32,
    enabled: u32,
    primitives: u32,
    changed: u32,
) -> u32 {
    let shape = geometry_with(e, 0xd0, vt, 0);
    let array = e.mem.alloc(0x14 * (count + 1)) + 0x14;
    for i in 0..count {
        let s = array + 0x14 * i;
        set(e, s, 10 * i + 1);
        set(e, s + 4, if primitives >> i & 1 != 0 { 3 + i } else { 0 });
        e.mem.set_u8(s + 8, (enabled >> i & 1) as u8);
        set(e, s + 0xc, 0x55 + i);
        e.mem.set_u8(s + 0x10, ((enabled >> i) ^ 1) as u8 & 1);
    }
    set(e, array - 0x14 + 4, 7);
    set(e, shape + 0xc4, array);
    set(e, shape + 0xc8, count);
    e.mem.set_u8(shape + 0xcc, changed as u8);
    shape
}

const SEGMENT_INPUTS: &[Input] = &[
    Input::Of(&[0, 1, 2, 3, 5]),
    Input::Of(&[0, 0b10110, 0b01001, 0b11111]),
    Input::Of(&[0, 0b11111, 0b10101, 0b01110]),
    Input::Of(&[1, 0]),
];

#[test]
fn merge_enabled_segments() {
    Diff::new(entry!(0x00a8_7040, shapes::merge_enabled_segments(Ptr<_>)))
        .common()
        .inputs(SEGMENT_INPUTS)
        .setup(|e, c| {
            stand_ins(e);
            let vt = vtable(e, &[]);
            let shape = segmented(e, vt, c.word(0), c.word(1), c.word(2), c.word(3));
            vec![shape]
        })
        .run()
        .assert_same();
}

#[test]
fn segmented_render_immediate() {
    Diff::new(entry!(
        0x00a8_7130,
        shapes::segmented_render_immediate(Ptr<_>, Ptr<_>)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 2, 3]),
        Input::Of(&[0, 0b101]),
        Input::Of(&[0b111, 0b010]),
        Input::Of(&[1, 0]),
        Input::Of(&[1, 3]),
        Input::Of(TWO_WAYS),
    ])
    .setup(|e, c| {
        stand_ins(e);
        let r = if c.word(5) != 0 {
            renderer_world(e, &[(0x1b4, 0x00e7_45a0)])
        } else {
            renderer_world(e, &[])
        };
        set(e, r.renderer + 0x200, c.word(4));
        e.mem.set_u8(r.renderer + 0x208, 1);
        e.mem.set_u8(r.renderer + 0x6fc, 1);
        let vt = vtable(e, &[]);
        let shape = segmented(e, vt, c.word(0), c.word(1), c.word(2), c.word(3));
        vec![shape, r.renderer]
    })
    .run()
    .assert_same();
}

// ---- The data getters. ----

/// A data object whose words read back distinct values.
fn data_words(e: &mut Engine, seed: u32) -> u32 {
    let d = e.mem.alloc(0x80);
    for i in 0..0x20u32 {
        set(
            e,
            d + 4 * i,
            seed.wrapping_mul(0x0101_0101).wrapping_add(i * 0x1111),
        );
    }
    d
}

#[test]
fn active_vertex_count() {
    Diff::new(entry!(
        0x00a6_69e0,
        shapes::active_vertex_count(u32) -> u16
    ))
    .common()
    .inputs(&[Input::Word])
    .setup(|e, c| vec![data_words(e, c.word(0))])
    .run()
    .assert_same();
}

#[test]
fn particles_active_vertex_count() {
    Diff::new(entry!(
        0x00a9_23a0,
        shapes::particles_active_vertex_count(u32) -> u16
    ))
    .common()
    .inputs(&[Input::Word])
    .setup(|e, c| vec![data_words(e, c.word(0))])
    .run()
    .assert_same();
}

#[test]
fn active_triangle_count() {
    Diff::new(entry!(
        0x00a7_5f20,
        shapes::active_triangle_count(u32) -> u16
    ))
    .common()
    .inputs(&[Input::Word])
    .setup(|e, c| vec![data_words(e, c.word(0))])
    .run()
    .assert_same();
}

#[test]
fn strips_dynamic_active_triangle_count() {
    Diff::new(entry!(
        0x00a9_23b0,
        shapes::strips_dynamic_active_triangle_count(u32) -> u16
    ))
    .common()
    .inputs(&[Input::Word])
    .setup(|e, c| vec![data_words(e, c.word(0))])
    .run()
    .assert_same();
}

#[test]
fn strip_lists() {
    Diff::new(entry!(0x00a7_5f10, shapes::strip_lists(u32) -> u32))
        .common()
        .inputs(&[Input::Word])
        .setup(|e, c| vec![data_words(e, c.word(0))])
        .run()
        .assert_same();
}

#[test]
fn strips_data_rtti() {
    Diff::new(entry!(0x00a7_5f30, shapes::strips_data_rtti() -> u32))
        .common()
        .inputs(&[Input::Word])
        .setup(|_, c| vec![c.word(0)])
        .run()
        .assert_same();
}

#[test]
fn strips_dynamic_data_rtti() {
    Diff::new(entry!(0x00a9_2470, shapes::strips_dynamic_data_rtti() -> u32))
        .common()
        .inputs(&[Input::Word])
        .setup(|_, c| vec![c.word(0)])
        .run()
        .assert_same();
}

#[test]
fn answers_false() {
    Diff::new(entry!(0x0047_c850, shapes::answers_false(u32) -> bool))
        .common()
        .inputs(&[Input::Word])
        .setup(|_, c| vec![c.word(0)])
        .run()
        .assert_same();
}

// ---- The renderer's draws. ----

/// The shape draw's switches: the geometry has a skin; strips with a lengths array; the renderer data
/// has an index buffer; the data has no vertices; the renderer skips drawing; the first partition has
/// strips; the shader's slot 0x90 and the data's slot 0x90 are the typed ones; the geometry's slot
/// 0x28 answers a draw object (the segmented shape's `00e68810`, else a stand-in answering one); the
/// draw object takes the data whole; the geometry's slot 0x2c answers a partial skin object.
const SHAPE_SKIN: u32 = 1;
const SHAPE_LENGTHS: u32 = 2;
const SHAPE_INDEXED: u32 = 4;
const SHAPE_NO_VERTICES: u32 = 8;
const SHAPE_SKIPPED: u32 = 0x10;
const SHAPE_PARTITION_STRIPS: u32 = 0x20;
const SHAPE_REAL_SLOTS: u32 = 0x40;
const SHAPE_RECORDS: u32 = 0x80;
const SHAPE_WHOLE: u32 = 0x100;
const SHAPE_PARTIAL_SKIN: u32 = 0x200;

/// Renderer data (or a partition's data): `strips` strips of the lengths 5, 6, 7.. (or a list of 9
/// primitives without `lengths`), each field moved by `seed`.
fn strip_data(e: &mut Engine, strips: u32, lengths: bool, indexed: bool, seed: u32) -> u32 {
    let data = e.mem.alloc(0x60);
    set(e, data + 0x18, 0x100 + seed);
    set(e, data + 0x34, if indexed { 0x7e00_2000 } else { 0 });
    set(e, data + 0x38, 0x30 + seed);
    set(e, data + 0x3c, 4 + seed);
    set(e, data + 0x40, 9 + seed);
    set(e, data + 0x48, strips);
    if lengths {
        let array = e.mem.alloc(0x10);
        for i in 0..8u32 {
            e.mem.set_u16(array + 2 * i, (5 + i + seed) as u16);
        }
        set(e, data + 0x4c, array);
    }
    data
}

/// The geometry for the shape draw (see the switches), its renderer and partitions: (renderer,
/// geometry).
fn shape_world(
    e: &mut Engine,
    switches: u32,
    strips: u32,
    partitions: u32,
    records: u32,
) -> (u32, u32) {
    stand_ins(e);
    let r = renderer_world(e, &[]);
    e.mem
        .set_u8(r.renderer + 0x6fc, u8::from(switches & SHAPE_SKIPPED != 0));
    let lengths = switches & SHAPE_LENGTHS != 0;
    let skinned = switches & SHAPE_SKIN != 0;
    let real = switches & SHAPE_REAL_SLOTS != 0;
    let data_vt = if real {
        vtable(e, &[(0x90, 0x00a6_69e0)])
    } else {
        vtable(e, &[])
    };
    let data = obj(e, 0x60, data_vt);
    let vertices = if switches & SHAPE_NO_VERTICES != 0 {
        0x1_0000
    } else {
        0x1_0020
    };
    set(e, RETS + 0x90, vertices);
    e.mem.set_u16(data + 8, (vertices & 0xffff) as u16);
    e.mem.set_u16(data + 0xe, 0xabcd);
    let render_data = strip_data(e, strips, lengths, switches & SHAPE_INDEXED != 0, 0);
    set(e, data + 0x34, if skinned { 0 } else { render_data });
    let shader_vt = if real {
        vtable(e, &[(0x90, 0x00e8_0580)])
    } else {
        vtable(e, &[])
    };
    let shader = obj(e, 0x40, shader_vt);
    // The geometry's slots 0x28 and 0x2c: the typed getters (the segmented shape's own object at
    // 0x28, nothing at 0x2c) or stand-ins answering from the table.
    let records_object = switches & SHAPE_RECORDS != 0;
    let geometry_vt = match (real, records_object) {
        (true, true) => vtable(e, &[(0x28, 0x00e6_8810), (0x2c, 0x00ac_bb70)]),
        (true, false) => vtable(e, &[(0x28, 0x00ac_bb70), (0x2c, 0x00ac_bb70)]),
        _ => vtable(e, &[]),
    };
    let geometry = obj(e, 0xd0, geometry_vt);
    let bound = e.mem.alloc(0x10);
    set(e, geometry + 0x20, bound);
    set(e, geometry + 0xb8, data);
    set(e, geometry + 0xc0, shader);
    // The draw object: the geometry itself when the slot is the typed one, another object otherwise.
    let draw_object = if real { geometry } else { e.mem.alloc(0xd0) };
    let array = e.mem.alloc(0x14 * 4);
    for i in 0..4u32 {
        let rec = array + 0x14 * i;
        set(e, rec, 2 * i + 1);
        set(e, rec + 0xc, 3 + i);
        e.mem.set_u8(rec + 0x10, u8::from(i != 1));
    }
    set(e, draw_object + 0xc4, array);
    set(e, draw_object + 0xc8, records);
    e.mem
        .set_u8(draw_object + 0xcd, u8::from(switches & SHAPE_WHOLE != 0));
    let model = e.mem.alloc(0x50);
    e.mem.set_u16(model + 0x40, 0x23);
    if !real {
        set(e, draw_object + 0xb8, model);
    } else {
        // The shape's own model data is `data`: its triangle count.
        e.mem.set_u16(data + 0x40, 0x23);
    }
    set(
        e,
        RETS + 0x28,
        if records_object && !real {
            draw_object
        } else {
            0
        },
    );
    let partial = e.mem.alloc(0xd0);
    set(e, partial + 0xc4, 0x31);
    set(
        e,
        RETS + 0x2c,
        if switches & SHAPE_PARTIAL_SKIN != 0 {
            partial
        } else {
            0
        },
    );
    if skinned {
        let parts = e.mem.alloc(0x2c * 4);
        for p in 0..4u32 {
            let at = parts + 0x2c * p;
            e.mem.set_u16(at + 0x1c, (0x40 + p) as u16);
            let part = strip_data(e, strips + (p & 1), lengths, true, 1 + p);
            set(e, at + 0x28, part);
        }
        e.mem.set_u16(
            parts + 0x22,
            u16::from(switches & SHAPE_PARTITION_STRIPS != 0),
        );
        let instance = e.mem.alloc(0x10);
        set(e, instance + 8, partitions);
        set(e, instance + 0xc, parts);
        let skin = e.mem.alloc(0x10);
        set(e, skin + 0xc, instance);
        set(e, geometry + 0xbc, skin);
    }
    (r.renderer, geometry)
}

/// `draw_shape` (`00e745a0`): skipped or without vertices; a plain geometry drawn indexed (strips with
/// or without lengths, with or without partial records, whole or not) or not; a skinned one partition
/// by partition, with or without a partial skin object; the slots typed or stand-ins.
#[test]
fn draw_shape() {
    Diff::new(entry!(0x00e7_45a0, packed::draw_shape(Ptr<_>, Ptr<_>)))
        .common()
        .translation_calls_plainly(&[0x00e7_2c20])
        .inputs(&[
            Input::Of(&[
                0,
                SHAPE_INDEXED,
                SHAPE_INDEXED | SHAPE_LENGTHS,
                SHAPE_INDEXED | SHAPE_RECORDS,
                SHAPE_INDEXED | SHAPE_RECORDS | SHAPE_WHOLE,
                SHAPE_INDEXED | SHAPE_LENGTHS | SHAPE_RECORDS | SHAPE_REAL_SLOTS,
                SHAPE_INDEXED | SHAPE_RECORDS | SHAPE_WHOLE | SHAPE_REAL_SLOTS,
                SHAPE_INDEXED | SHAPE_REAL_SLOTS,
                SHAPE_SKIN,
                SHAPE_SKIN | SHAPE_LENGTHS,
                SHAPE_SKIN | SHAPE_PARTIAL_SKIN,
                SHAPE_SKIN | SHAPE_LENGTHS | SHAPE_PARTITION_STRIPS,
                SHAPE_SKIN | SHAPE_REAL_SLOTS,
                SHAPE_INDEXED | SHAPE_NO_VERTICES,
                SHAPE_INDEXED | SHAPE_SKIPPED,
            ]),
            Input::Of(&[1, 0, 3]),
            Input::Of(&[1, 0, 2, 4]),
            Input::Of(&[0, 1, 4]),
        ])
        .max_cases(400)
        .setup(|e, c| {
            let (renderer, geometry) = shape_world(e, c.word(0), c.word(1), c.word(2), c.word(3));
            vec![renderer, geometry]
        })
        .run()
        .assert_same();
}

/// `draw_partition` (`00e6d310`): no strips, strips with or without lengths, the object's strip word.
#[test]
fn draw_partition() {
    Diff::new(entry!(
        0x00e6_d310,
        packed::draw_partition(Ptr<_>, u32, u32, u32)
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 3]),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1, 0x100]),
        Input::Word,
    ])
    .setup(|e, c| {
        stand_ins(e);
        let r = renderer_world(e, &[]);
        let part = strip_data(e, c.word(0), c.word(1) != 0, true, 2);
        let object = e.mem.alloc(0x30);
        e.mem.set_u16(object + 0x22, c.word(2) as u16);
        e.mem.set_u16(object + 0x1c, 0x44);
        vec![r.renderer, part, object, c.word(3)]
    })
    .run()
    .assert_same();
}

/// The points draw's switches: the property's alpha blends with a limit not above 0; with colours,
/// angles, bytes; an index array; the index buffer made (none kept); the data's vertex count slot the
/// typed particles' one; the renderer skips; the lock fails; the declarations exist.
const POINTS_BLEND_OFF: u32 = 1;
const POINTS_COLORS: u32 = 2;
const POINTS_ANGLES: u32 = 4;
const POINTS_BYTES: u32 = 8;
const POINTS_INDICES: u32 = 0x10;
const POINTS_NEW_INDEX_BUFFER: u32 = 0x20;
const POINTS_REAL_SLOT: u32 = 0x40;
const POINTS_SKIPPED: u32 = 0x80;
const POINTS_LOCK_FAILS: u32 = 0x200;
const POINTS_DECLARED: u32 = 0x400;

/// A float array of `n` entries from `seed`.
fn floats(e: &mut Engine, n: u32, seed: f32) -> u32 {
    let a = e.mem.alloc(4 * n.max(1));
    for i in 0..n {
        set(e, a + 4 * i, (seed + 0.25 * i as f32).to_bits());
    }
    a
}

/// The points draw's world: (renderer, geometry).
fn points_world(e: &mut Engine, switches: u32, count: u32, limit: u32, size: u32) -> (u32, u32) {
    stand_ins(e);
    let r = renderer_world(e, &[]);
    e.mem
        .set_u8(r.renderer + 0x6fc, u8::from(switches & POINTS_SKIPPED != 0));
    if switches & POINTS_DECLARED != 0 {
        set(e, r.renderer + 0x280, 0x0de0_0280);
        set(e, r.renderer + 0x284, 0x0de0_0284);
    }
    // The property state the renderer points at: the alpha property (blending when bit 0 of +0x18
    // is set) and the third word's float +0x3c.
    let vt = vtable(e, &[]);
    let alpha = obj(e, 0x20, vt);
    e.mem.set_u16(
        alpha + 0x18,
        if switches & POINTS_BLEND_OFF != 0 {
            0
        } else {
            1
        },
    );
    let third = obj(e, 0x40, vt);
    set(e, third + 0x3c, limit);
    let state = e.mem.alloc(0xc);
    set(e, state, alpha);
    set(e, state + 8, third);
    set(e, r.renderer + 0xc, state);
    let data_vt = if switches & POINTS_REAL_SLOT != 0 {
        vtable(e, &[(0x90, 0x00a9_23a0)])
    } else {
        vtable(e, &[])
    };
    let data = obj(e, 0x70, data_vt);
    set(e, RETS + 0x90, count);
    e.mem.set_u16(data + 0x50, count as u16);
    e.mem.set_u16(data + 8, (count + 2) as u16);
    let n = count + 2;
    let positions = floats(e, 3 * n, 1.5);
    let sizes = floats(e, n, 0.75);
    let scales = floats(e, n, 2.0);
    let colors = if switches & POINTS_COLORS != 0 {
        floats(e, 4 * n, 0.125)
    } else {
        0
    };
    let angles = if switches & POINTS_ANGLES != 0 {
        floats(e, n, 0.3)
    } else {
        0
    };
    set(e, data + 0x20, positions);
    set(e, data + 0x4c, sizes);
    set(e, data + 0x54, scales);
    set(e, data + 0x28, colors);
    set(e, data + 0x5c, angles);
    if switches & POINTS_BYTES != 0 {
        let bytes = e.mem.alloc(n.max(4));
        for i in 0..n {
            e.mem.set_u8(bytes + i, (7 * i + 3) as u8);
        }
        set(e, data + 0x64, bytes);
    }
    if switches & POINTS_INDICES != 0 {
        let indices = e.mem.alloc(2 * n.max(2));
        for i in 0..n {
            e.mem.set_u16(indices + 2 * i, (n - 1 - i) as u16);
        }
        set(e, data + 0x44, indices);
    }
    // The buffer data: one stream (or none) with its stride and chip; the chip's vertex buffer locks
    // into a block of its own.
    let buffer = e.mem.alloc(0x60);
    set(e, data + 0x34, buffer);
    let group_vt = vtable(e, &[]);
    let group = obj(e, 0x20, group_vt);
    set(e, buffer + 8, group);
    set(e, buffer + 0x20, 1);
    let strides = e.mem.alloc(8);
    set(e, buffer + 0x24, strides);
    let chips = e.mem.alloc(8);
    set(e, buffer + 0x28, chips);
    let lock_vt = vtable(e, &[(0x2c, LOCK_FN)]);
    let vertex_buffer = obj(e, 0x10, lock_vt);
    let chip = e.mem.alloc(0x18);
    set(e, chip + 8, vertex_buffer);
    set(e, chip + 0xc, 0x40);
    set(e, chip + 0x10, 0x2000);
    set(e, chips, chip);
    // CreateChip (the group's slot 0x18 stand-in) answers the same chip.
    set(e, RETS + 0x18, chip);
    set(e, buffer + 0x10, 0x1010);
    let vertex_block = e.mem.alloc(0x28 * 4 * n);
    set(e, RETS + 0x700, vertex_block);
    set(
        e,
        RETS + 0x704,
        if switches & POINTS_LOCK_FAILS != 0 {
            0x8000_0001
        } else {
            0
        },
    );
    // The index buffer: kept in the buffer data, or made by 00e8fc50 (none kept); `00e8f360` answers
    // one for the plain run.
    let index_buffer = obj(e, 0x10, lock_vt);
    if switches & POINTS_NEW_INDEX_BUFFER == 0 {
        set(e, buffer + 0x34, index_buffer);
    }
    set(e, RETS + 0x60c, index_buffer);
    set(e, RETS + 0x608, index_buffer);
    // The property: one pass (slot 0x90 answers 1, 0x94 none after it).
    let property = obj(e, 0x20, vt);
    set(e, RETS + 0x94, 0);
    // The geometry: bound, world transform (rotation, translation, scale), property state, data,
    // property.
    let geometry = obj(e, 0xc4, vt);
    let bound = e.mem.alloc(0x10);
    for (i, w) in [ONE, TWO, HALF, size].iter().enumerate() {
        set(e, bound + 4 * i as u32, *w);
    }
    set(e, geometry + 0x20, bound);
    for (i, w) in [
        ONE,
        0,
        0,
        0,
        HALF,
        0,
        0,
        0,
        TWO,
        0x4040_0000,
        ONE,
        0xbf80_0000,
        size,
    ]
    .iter()
    .enumerate()
    {
        set(e, geometry + 0x68 + 4 * i as u32, *w);
    }
    set(e, geometry + 0xb8, data);
    set(e, geometry + 0xc0, property);
    (r.renderer, geometry)
}

/// `draw_points` (`00e70140`): skipped, blended below the limit or not; with or without colours,
/// angles, bytes, an index array, a kept index buffer, a lock that works, the declarations;
/// the data's slot typed or a stand-in; the count and the scale over the special floats.
#[test]
fn draw_points() {
    Diff::new(entry!(0x00e7_0140, packed::draw_points(Ptr<_>, Ptr<_>)))
        .common()
        .inputs(&[
            Input::Of(&[
                0,
                POINTS_COLORS,
                POINTS_ANGLES,
                POINTS_COLORS | POINTS_ANGLES | POINTS_BYTES,
                POINTS_BYTES | POINTS_INDICES,
                POINTS_INDICES | POINTS_NEW_INDEX_BUFFER,
                POINTS_COLORS | POINTS_REAL_SLOT | POINTS_DECLARED,
                POINTS_ANGLES | POINTS_BYTES | POINTS_DECLARED,
                POINTS_BLEND_OFF,
                POINTS_SKIPPED,
                POINTS_LOCK_FAILS,
                POINTS_INDICES | POINTS_LOCK_FAILS,
            ]),
            Input::Of(&[2, 0, 1]),
            Input::Of(&[ONE, 0, 0xbf80_0000, 0x7fc0_0000]),
            Input::F32,
        ])
        .max_cases(500)
        .float_sweep(24)
        .setup(|e, c| {
            let (renderer, geometry) =
                points_world(e, c.word(0), c.word(1), c.word(2), c.f32(3).to_bits());
            vec![renderer, geometry]
        })
        .run()
        .assert_same();
}

/// The quad builders' inputs: the count, with colours, with angles (2: the first a NaN), the camera
/// vector floats.
fn quads_setup(e: &mut Engine, c: &Case, bytes: bool) -> Vec<u32> {
    stand_ins(e);
    let r = renderer_world(e, &[]);
    for (i, at) in [0x64cu32, 0x650, 0x654, 0x658, 0x65c, 0x660]
        .iter()
        .enumerate()
    {
        let value = if i == 2 {
            c.f32(3).to_bits()
        } else {
            [ONE, HALF, 0, TWO, 0xbf80_0000, 0][i]
        };
        set(e, r.renderer + at, value);
    }
    let n = c.word(0);
    let vertices = e.mem.alloc(0x28 * 4 * n.max(1));
    let positions = floats(e, 3 * n, 1.25);
    // The first point's x is the case's float (a signalling NaN keeps its form only as a memory operand).
    set(e, positions, c.f32(3).to_bits());
    let colors = if c.word(1) != 0 {
        floats(e, 4 * n, 0.5)
    } else {
        0
    };
    let sizes = floats(e, n, 0.5);
    let scales = floats(e, n, c.f32(4));
    let angles = if c.word(2) != 0 { floats(e, n, 0.7) } else { 0 };
    if c.word(2) == 2 {
        // The first angle the x87 default NaN: the first quad's offsets are then a quiet NaN whose
        // payload is below a quieted signalling NaN's, so which operand form the fourth corner's x
        // takes (memory or loaded) decides which NaN it keeps.
        set(e, angles, 0xffc0_0000);
    }
    let mut words = vec![
        r.renderer, vertices, positions, colors, sizes, scales, angles,
    ];
    if bytes {
        let b = e.mem.alloc(n.max(4));
        for i in 0..n {
            e.mem.set_u8(b + i, (0x41 + 5 * i) as u8);
        }
        words.push(b);
    }
    words.push(n);
    words
}

const QUAD_INPUTS: &[Input] = &[
    Input::Of(&[1, 0, 3]),
    Input::Of(TWO_WAYS),
    Input::Of(&[0, 1, 2]),
    Input::F32,
    Input::F32Swept(ONE),
];

#[test]
fn build_quads() {
    Diff::new(entry!(
        0x00e6_9ea0,
        packed::build_quads(Ptr<_>, u32, u32, u32, u32, u32, u32, u16)
    ))
    .common()
    .inputs(QUAD_INPUTS)
    .float_sweep(48)
    .setup(|e, c| quads_setup(e, c, false))
    .run()
    .assert_same();
}

#[test]
fn build_quads_with_byte() {
    Diff::new(entry!(
        0x00e6_ab40,
        packed::build_quads_with_byte(Ptr<_>, u32, u32, u32, u32, u32, u32, u32, u16)
    ))
    .common()
    .inputs(QUAD_INPUTS)
    .float_sweep(48)
    .setup(|e, c| quads_setup(e, c, true))
    .run()
    .assert_same();
}

/// The declarations: made already, made now, or failed (the device's answer).
fn declaration_setup(e: &mut Engine, c: &Case, slot: u32) -> Vec<u32> {
    stand_ins(e);
    let r = renderer_world(e, &[]);
    let device_vt = vtable(e, &[(0x158, CREATE_FN)]);
    let device = obj(e, 0x10, device_vt);
    set(e, r.renderer + 0x288, device);
    if c.word(0) != 0 {
        set(e, r.renderer + slot, 0x0dec_1a2e);
    }
    set(e, RETS + 0x158, c.word(1));
    vec![r.renderer]
}

#[test]
fn declaration() {
    Diff::new(entry!(0x00e6_96a0, packed::declaration(Ptr<_>) -> u32))
        .common()
        .inputs(&[Input::Of(TWO_WAYS), Input::Of(&[0, 1, 0x8000_0000])])
        .setup(|e, c| declaration_setup(e, c, 0x280))
        .run()
        .assert_same();
}

#[test]
fn declaration_with_byte() {
    Diff::new(entry!(
        0x00e6_9790,
        packed::declaration_with_byte(Ptr<_>) -> u32
    ))
    .common()
    .inputs(&[Input::Of(TWO_WAYS), Input::Of(&[0, 1, 0x8000_0000])])
    .setup(|e, c| declaration_setup(e, c, 0x284))
    .run()
    .assert_same();
}

// ---- The skinned draw's bones. ----

/// A transform (rotation, translation, scale) at `at` from `seed`, with `scale`.
fn transform_at(e: &mut Engine, at: u32, seed: f32, scale: u32) {
    let rotation = [
        1.0,
        seed * 0.1,
        0.0,
        -seed * 0.1,
        1.0,
        0.25,
        0.0,
        -0.25,
        1.0f32,
    ];
    for (i, v) in rotation.iter().enumerate() {
        set(e, at + 4 * i as u32, v.to_bits());
    }
    for (i, v) in [seed, 2.0 * seed, -seed].iter().enumerate() {
        set(e, at + 0x24 + 4 * i as u32, v.to_bits());
    }
    set(e, at + 0x30, scale);
}

/// The skin for the bone fill: `bones` bones (each node's world transform and scale, a bind transform
/// each 0x4c bytes), the root, the data's transform; its entry point, register count and buffer as
/// the case says: (renderer, skin, world).
fn bone_world(
    e: &mut Engine,
    bones: u32,
    current: u32,
    registers: u32,
    capacity: u32,
    scale: u32,
) -> (u32, u32, u32) {
    stand_ins(e);
    let r = renderer_world(e, &[]);
    set(e, RETS + 0x614, 0x0e17_0001);
    let skin = e.mem.alloc(0x40);
    let data = e.mem.alloc(0x50);
    set(e, skin + 8, data);
    transform_at(e, data + 0xc, 0.5, ONE);
    let binds = e.mem.alloc(0x4c * bones.max(1));
    for b in 0..bones {
        transform_at(e, binds + 0x4c * b, 0.25 + b as f32, ONE);
    }
    set(e, data + 0x40, binds);
    set(e, data + 0x44, bones);
    let root = e.mem.alloc(0xa0);
    transform_at(e, root + 0x68, 1.5, scale);
    set(e, skin + 0x10, root);
    let list = e.mem.alloc(4 * bones.max(1));
    for b in 0..bones {
        let node = e.mem.alloc(0xa0);
        transform_at(
            e,
            node + 0x68,
            0.75 * (b + 1) as f32,
            if b == 1 { scale } else { ONE },
        );
        set(e, list + 4 * b, node);
    }
    set(e, skin + 0x14, list);
    set(
        e,
        skin + 0x18,
        if current != 0 {
            0x0e17_0001
        } else {
            0x0e17_0002
        },
    );
    set(e, skin + 0x20, registers);
    set(e, skin + 0x24, capacity);
    let size = capacity.max(0x40) + 0x100;
    let buffer = e.mem.alloc(size);
    // Words left from an earlier fill, so a word this fill does not write shows.
    for i in 0..size / 4 {
        set(e, buffer + 4 * i, 0x5ca1_ab1e ^ i);
    }
    set(e, skin + 0x28, buffer);
    let world = e.mem.alloc(0x34);
    transform_at(e, world, 2.0, TWO);
    (r.renderer, skin, world)
}

/// `bone_matrices` (`00e6fe30`): already filled for this entry point and register count, or filled
/// for 3 or 4 registers, either layout, with the world or without; the buffer made anew or kept; no,
/// one or two bones; the scales over the special floats.
#[test]
fn bone_matrices() {
    Diff::new(entry!(
        0x00e6_fe30,
        skin::bone_matrices(Ptr<_>, Ptr<_>, Ptr<_>, u8, u32, u8)
    ))
    .common()
    // What the transform inverse's and product's translations call plainly (the dispatch family's tests list
    // the same).
    .translation_calls_plainly(&[
        0x0040_19d0,
        0x0040_8860,
        0x0047_6930,
        0x0047_6980,
        0x0047_69c0,
    ])
    .inputs(&[
        Input::Of(&[2, 0, 1]),
        Input::Of(&[0, 1]),
        Input::Of(&[3, 4]),
        Input::Of(&[0, 1]),
        Input::Of(&[0, 1]),
        Input::Of(&[0x400, 0]),
        Input::F32,
        Input::Of(TWO_WAYS),
    ])
    .max_cases(400)
    .float_sweep(32)
    .setup(|e, c| {
        let registers = c.word(2);
        // The register count the skin holds: the one asked for, or another (so a filled skin is filled
        // again).
        let stored = registers ^ (c.word(7) * 7);
        let (renderer, skin, world) = bone_world(
            e,
            c.word(0),
            c.word(1),
            stored,
            c.word(5),
            c.f32(6).to_bits(),
        );
        vec![renderer, skin, world, c.word(3), registers, c.word(4)]
    })
    .run()
    .assert_same();
}

/// `bone_setup` (`00b7a2c0`): the switch present and on or not, the shader's own bone constants or
/// not, a partition or none, +0x38 set or not; the bone fill finds the skin filled already.
#[test]
fn bone_setup() {
    Diff::new(entry!(
        0x00b7_a2c0,
        skin::bone_setup(Ptr<_>, u32, u32, u32, u32, u32, u32, u32, u32) -> u32
    ))
    .common()
    .translation_calls_plainly(&[
        0x0040_19d0,
        0x0040_8860,
        0x0047_6930,
        0x0047_6980,
        0x0047_69c0,
    ])
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
    ])
    .setup(|e, c| {
        let (_, skin, world) = bone_world(e, 1, 1, 3, 0x400, ONE);
        let vt = vtable(e, &[]);
        let switch = match c.word(0) {
            0 => 0,
            n => {
                let s = obj(e, 0x10, vt);
                e.mem.set_u8(s + 8, (n - 1) as u8);
                s
            }
        };
        set(e, 0x011f_ec00, switch);
        let own = obj(e, 0x10, vt);
        let shader = obj(e, 0x90, vt);
        set(e, shader + 0x80, own);
        set(e, shader + 0x34, if c.word(1) != 0 { own } else { 0x5151 });
        set(e, shader + 0x38, c.word(3));
        set(e, RETS + 0x70, 0x0b0e_3333);
        let pass = e.mem.alloc(0x60);
        let constants = obj(e, 0x10, vt);
        set(e, pass + 0x5c, constants);
        set(e, 0x0126_f74c, pass);
        // No partition only for a shader without its own bones: with them the exe reads the null
        // partition's count.
        let partition = if c.word(2) != 0 || c.word(1) != 0 {
            let p = e.mem.alloc(0x30);
            e.mem.set_u16(p + 0x20, 3);
            set(e, p + 4, 0x0b0e_4444);
            p
        } else {
            0
        };
        vec![
            shader, 0x1111, skin, partition, 0x2222, 0x3333, 0x4444, world, 0x5555,
        ]
    })
    .run()
    .assert_same();
}

// ---- The packing. ----

/// `remake_chip` (`00e8bfa0`): the group's `CreateChip` the typed one or a stand-in answering a chip
/// or none; the stream within the count or past it.
#[test]
fn remake_chip() {
    Diff::new(entry!(
        0x00e8_bfa0,
        packing::remake_chip(Ptr<_>, Ptr<_>, u32) -> bool
    ))
    .common()
    .translation_calls_plainly(&[0x00e9_5230])
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 1, 3]),
        Input::Of(&[0, 1]),
    ])
    .setup(|e, c| {
        stand_ins(e);
        let real = c.word(0) == 2;
        let (data, _) = chip_world(e, real);
        set(e, RETS + 0x18, if c.word(0) == 1 { 0 } else { 0x0c41_0001 });
        // The typed `CreateChip` makes a chip for a stream in the count (a stride of 0 faults).
        let chip = e.mem.alloc(0x18);
        set(e, RETS + 0x62c, chip);
        set(
            e,
            data + 0x20,
            if real { c.word(1).max(1) } else { c.word(1) },
        );
        let stream = if real { 0 } else { c.word(2) };
        vec![0x0bad_0001, data, stream]
    })
    .run()
    .assert_same();
}

/// A buffer data for the chips with its geometry group (the dynamic group, its `CreateChip` the typed
/// one when `real`): (buffer data, group). The group's arrays and maps, the records and chips the
/// tables answer.
fn chip_world(e: &mut Engine, real: bool) -> (u32, u32) {
    let group_vt = if real {
        vtable(e, &[(0x18, 0x00e9_5550)])
    } else {
        vtable(e, &[])
    };
    let group = obj(e, 0x60, group_vt);
    let vt = vtable(e, &[]);
    let device = obj(e, 0x10, vt);
    set(e, group + 8, device);
    let data = e.mem.alloc(0x60);
    set(e, data + 8, group);
    set(e, data + 0xc, 0x1c00_0000);
    e.mem.set_u8(data + 0x14, 0x31);
    set(e, data + 0x1c, 12);
    let strides = e.mem.alloc(0x10);
    for i in 0..4u32 {
        set(e, strides + 4 * i, 0x18 + 4 * i);
    }
    set(e, data + 0x24, strides);
    let chips = e.mem.alloc(0x10);
    set(e, data + 0x28, chips);
    set(e, data + 0x20, 1);
    // The group's two arrays (+0x2c records, +0x3c chips; +0x30 and +0x40 their bases, +0x34 the
    // size) and the cursor (+0x4c).
    let records = e.mem.alloc(0x10);
    let chip_array = e.mem.alloc(0x10);
    set(e, group + 0x30, records);
    set(e, group + 0x40, chip_array);
    e.mem.set_u16(group + 0x34, 2);
    (data, group)
}

/// `create_chip` (`00e95550`): one stream (key from the data or from the stride; the record found,
/// found null or absent; the chip found or made) or several (the arrays grown or not, the cursor
/// wrapping, the record and the chip in the arrays or made).
#[test]
fn create_chip() {
    Diff::new(entry!(
        0x00e9_5550,
        packing::create_chip(Ptr<_>, u32, u32) -> u32
    ))
    .common()
    .translation_calls_plainly(&[0x00e9_5230])
    .inputs(&[
        Input::Of(&[1, 2, 3]),
        Input::Of(&[0, 1, 5]),
        Input::Of(&[0, 1, 2]),
        Input::Of(TWO_WAYS),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 3]),
    ])
    .max_cases(300)
    .setup(|e, c| {
        stand_ins(e);
        let (data, group) = chip_world(e, false);
        let streams = c.word(0);
        set(e, data + 0x20, streams);
        if c.word(3) != 0 {
            set(e, data + 0xc, 0);
        }
        // The map answers a record (or none), the chip map a chip (or none).
        let record = e.mem.alloc(0x18);
        let chip = e.mem.alloc(0x18);
        set(
            e,
            RETS + 0x65c,
            match c.word(2) {
                0 => 0,
                1 => record,
                _ => chip,
            },
        );
        set(e, RETS + 0x628, 0x0b0f_0000);
        set(e, RETS + 0x62c, chip);
        // The arrays: the slot at the cursor filled or not, the cursor at `c.word(5)`.
        if c.word(4) != 0 {
            let records = e.mem.u32(group + 0x30);
            let chips = e.mem.u32(group + 0x40);
            set(e, records, record);
            set(e, chips, chip);
        }
        set(e, group + 0x4c, c.word(5));
        // The stride of 0 a stream past the count gives faults; the tests keep it in the count.
        vec![group, data, c.word(1).min(streams.saturating_sub(1))]
    })
    .run()
    .assert_same();
}

/// The vertex manager: its staging block and capacity, a vertex buffer that locks into a block.
fn staging_world(e: &mut Engine, capacity: u32, lock_result: u32) -> (u32, u32) {
    stand_ins(e);
    let r = renderer_world(e, &[]);
    let manager = r.vertex_manager;
    let block = e.mem.alloc(capacity.max(4));
    set(e, manager + 0x40, block);
    set(e, manager + 0x44, capacity);
    let locked = e.mem.alloc(0x100);
    for i in 0..0x40u32 {
        set(e, locked + 4 * i, 0x1000_0000 + i);
    }
    set(e, RETS + 0x700, locked);
    set(e, RETS + 0x704, lock_result);
    let lock_vt = vtable(e, &[(0x2c, LOCK_FN)]);
    let buffer = obj(e, 0x10, lock_vt);
    (manager, buffer)
}

/// `lock_vertices` (`00e8bff0`): the lock failing or not, the staging block grown or kept, copied or
/// not by the flags.
#[test]
fn lock_vertices() {
    Diff::new(entry!(
        0x00e8_bff0,
        packing::lock_vertices(Ptr<_>, u32, u32, u32, u32) -> u32
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 0x8000_0005, 1]),
        Input::Of(&[0x80, 0x10]),
        Input::Of(&[0x40, 0x80, 0]),
        Input::Of(&[0, 0x1000, 0x2000, 0x800]),
    ])
    .setup(|e, c| {
        let (manager, buffer) = staging_world(e, c.word(1), c.word(0));
        vec![manager, buffer, 0x20, c.word(2), c.word(3)]
    })
    .run()
    .assert_same();
}

/// `unlock_vertices` (`00e8c0d0`): the staged bytes copied back, the unlock's answer.
#[test]
fn unlock_vertices() {
    Diff::new(entry!(
        0x00e8_c0d0,
        packing::unlock_vertices(Ptr<_>, u32) -> bool
    ))
    .common()
    .inputs(&[Input::Of(&[0, 0x8000_0001, 7]), Input::Of(&[0x40, 0, 0x10])])
    .setup(|e, c| {
        let (manager, buffer) = staging_world(e, 0x80, 0);
        let block = e.mem.u32(manager + 0x40);
        for i in 0..0x20u32 {
            set(e, block + 4 * i, 0x2000_0000 + i);
        }
        let target = e.mem.alloc(0x80);
        set(e, manager + 0x48, target);
        set(e, manager + 0x4c, c.word(1));
        set(e, RETS + 0x30, c.word(0));
        vec![manager, buffer]
    })
    .run()
    .assert_same();
}

/// A shader declaration: `streams` streams of `entries` entries (usages from `usages`), the modified
/// byte, the built one, its element array and capacity, its device.
fn declaration_world(
    e: &mut Engine,
    streams: u32,
    entries: u32,
    usages: &[u32],
    modified: u32,
    built: u32,
    capacity: u32,
) -> u32 {
    stand_ins(e);
    let vt = vtable(e, &[(0xb0, WORDS_FN)]);
    let declaration = obj(e, 0x40, vt);
    let device_vt = vtable(e, &[]);
    let device = obj(e, 0x10, device_vt);
    set(e, declaration + 0x10, device);
    let r = renderer_world(e, &[]);
    set(e, declaration + 8, r.renderer);
    set(e, declaration + 0xc, r.vertex_manager);
    set(e, declaration + 0x14, capacity);
    set(e, declaration + 0x1c, entries);
    set(e, declaration + 0x20, streams);
    let array = e.mem.alloc(0x10 * streams.max(1));
    set(e, declaration + 0x24, array);
    for s in 0..streams {
        let at = array + 0x10 * s;
        let list = e.mem.alloc(0x1c * entries.max(1));
        set(e, at + 8, list);
        set(e, at + 0xc, 0x24);
        for k in 0..entries {
            let entry = list + 0x1c * k;
            let usage = usages[((s * entries + k) as usize) % usages.len()];
            set(e, entry + 4, 4 * k);
            set(e, entry + 8, usage);
            set(e, entry + 0xc, if usage == 0x11 { 0x11 } else { 2 });
        }
    }
    e.mem.set_u8(declaration + 0x28, modified as u8);
    // A built declaration: a Direct3D object (its `Release` a stand-in).
    let built = if built != 0 {
        obj(e, 0x10, device_vt)
    } else {
        0
    };
    set(e, declaration + 0x30, built);
    if capacity != 0 {
        let elements = e.mem.alloc(8 * capacity);
        set(e, declaration + 0x2c, elements);
    }
    set(e, RETS + 0x630, 0xc);
    declaration
}

/// `build_declaration` (`00e76a80`): not modified (the built one answered), or rebuilt: a built one
/// released, entries in use or none, the element array kept, too small or missing, the device's
/// answer.
#[test]
fn build_declaration() {
    Diff::new(entry!(
        0x00e7_6a80,
        packing::build_declaration(Ptr<_>) -> u32
    ))
    .common()
    .inputs(&[
        Input::Of(&[1, 0]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 2, 3]),
        Input::Of(&[0, 0x0bd0_0001]),
        Input::Of(&[0, 2, 8]),
        Input::Of(&[0, 0x8000_0000]),
    ])
    .max_cases(300)
    .setup(|e, c| {
        let declaration = declaration_world(
            e,
            c.word(1),
            c.word(2),
            &[0, 0xffff_ffff, 3, 0x11, 5],
            c.word(0),
            c.word(3),
            c.word(4),
        );
        set(e, RETS + 0x158, c.word(5));
        vec![declaration]
    })
    .run()
    .assert_same();
}

/// `pack_stream` (`00e76770`): a partition or the data's count (none: the chip answered), the slot 0xb0
/// answering set or not, the renderer's byte +0x5d4, a locked address or the chip kept, remade or
/// failing, the derived vectors kept for the data's own pack or packed, the flags, the unlock's answer.
#[test]
fn pack_stream() {
    Diff::new(entry!(
        0x00e7_6770,
        packing::pack_stream(Ptr<_>, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2]),
        Input::Of(&[1, 0]),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 0x07, 0x0f, 0xfff]),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1]),
    ])
    .max_cases(400)
    .setup(|e, c| {
        let declaration = declaration_world(e, 2, 4, &[0, 3, 4, 7, 13, 14, 1], 0, 0, 0);
        set(e, RETS + 0x708, c.word(1) * 0x0bd0_0002);
        let renderer = e.mem.u32(declaration + 8);
        e.mem.set_u8(renderer + 0x5d4, c.word(2) as u8);
        set(e, RETS + 0xa8, 0x77);
        set(e, RETS + 0xa0, 4);
        let data_vt = vtable(e, &[(0x90, 0x00a6_69e0)]);
        let data = obj(e, 0x60, data_vt);
        e.mem.set_u16(data + 8, 6);
        set(e, data + 0x20, 0x0da7_0020);
        set(e, data + 0x24, 0x0da7_0024);
        let buffer = e.mem.alloc(0x60);
        set(e, buffer + 0x20, 2);
        let strides = e.mem.alloc(8);
        set(e, buffer + 0x24, strides);
        let chips = e.mem.alloc(8);
        set(e, buffer + 0x28, chips);
        set(e, data + 0x34, buffer);
        let partition = match c.word(0) {
            0 => 0,
            1 => {
                let p = e.mem.alloc(0x30);
                e.mem.set_u16(p + 0x1c, 5);
                set(e, p + 0x28, buffer);
                p
            }
            _ => {
                let p = e.mem.alloc(0x30);
                set(e, p + 0x28, buffer);
                p
            }
        };
        // The chip: kept (its buffer describes itself as vertex data, big enough), remade, or none.
        let lock_vt = vtable(e, &[(0x2c, LOCK_FN)]);
        let vertex_buffer = obj(e, 0x10, lock_vt);
        let chip = e.mem.alloc(0x18);
        set(e, chip + 8, vertex_buffer);
        set(e, chip + 0x14, 0x80);
        set(e, RETS + 0x634, chip);
        let locked = e.mem.alloc(0x200);
        set(e, RETS + 0x700, locked);
        set(e, RETS + 0x704, 0);
        set(e, RETS + 0x30, c.word(6));
        let (chip_word, locked_word) = match c.word(3) {
            0 => (0, 0),
            1 => (chip, 0),
            2 => (chip, locked),
            _ => (0, locked),
        };
        // `GetDesc` (slot 0x34) leaves the description as the stack has it: format and size from the
        // table, which the stand-in doesn't write; the chip is then remade.
        let group_vt = vtable(e, &[]);
        let group = obj(e, 0x20, group_vt);
        set(e, buffer + 8, group);
        set(e, RETS + 0x18, chip);
        let derived = if c.word(5) != 0 { 0x0de2_0000 } else { 0 };
        vec![
            declaration,
            data,
            derived,
            partition,
            c.word(4),
            chip_word,
            1,
            locked_word,
            0x88,
            0x99,
        ]
    })
    .run()
    .assert_same();
}

/// `pack_skinned` (`00e6d780`): no buffer data or data; packed and clean (true at once) or not, forced
/// or not; the packer present (its slot 0xac the typed stream pack or a stand-in) or not, its stream
/// count; the vertex packer's and the index packer's answers.
#[test]
fn pack_skinned() {
    Diff::new(entry!(
        0x00e6_d780,
        packing::pack_skinned(Ptr<_>, Ptr<_>, Ptr<_>, u32, Ptr<_>, Ptr<_>, u8) -> bool
    ))
    .common()
    .inputs(&[
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0x4000, 0x0030, 0x0fff, 0x0010]),
        Input::Of(TWO_WAYS),
        Input::Of(&[0, 1, 2, 3]),
        Input::Of(&[0, 1, 2]),
        Input::Of(&[0, 0x0e13_0000, 0x0e1b_0034]),
    ])
    .max_cases(400)
    .setup(|e, c| {
        stand_ins(e);
        let r = renderer_world(e, &[]);
        // The buffer data: packed (a stream with a buffer) or not, its streams owner.
        let b = e.mem.alloc(0x60);
        let vt = vtable(e, &[]);
        let owner = obj(e, 0x10, vt);
        set(e, b + 8, owner);
        let chips = e.mem.alloc(8);
        let chip = e.mem.alloc(0x18);
        set(e, chips, chip);
        set(e, b + 0x28, chips);
        if c.word(0) & 1 != 0 {
            set(e, b + 0x20, 1);
            set(e, chip + 8, 0x0e00_0008);
        }
        e.mem.set_u8(b + 0x14, (c.word(0) >> 1) as u8);
        set(e, b + 0x30, 0x30);
        set(e, b + 0x34, 0x0e1b_0034);
        // The geometry data's flags.
        let g = e.mem.alloc(0x40);
        e.mem.set_u16(g + 0xe, c.word(1) as u16);
        e.mem.set_u16(g + 0xc, 0x1001);
        set(e, g + 0x24, 1);
        set(e, g + 0x28, c.word(2));
        // The partition's counts.
        let s = e.mem.alloc(0x30);
        set(e, s + 0x14, 0x0e14_0014);
        set(e, s + 0x18, 0x0e18_0018);
        e.mem.set_u16(s + 0x1c, 9);
        e.mem.set_u16(s + 0x1e, 4);
        e.mem.set_u16(s + 0x22, (c.word(0) & 2) as u16);
        // The packer: none, a stand-in slot, the typed stream pack (which then finds nothing to
        // pack: the slot 0xb0 answers none), with one or two streams.
        let packer = match c.word(3) {
            0 => 0,
            n => {
                let pvt = if n == 3 {
                    vtable(e, &[(0xac, 0x00e7_6770), (0xb0, WORDS_FN)])
                } else {
                    vtable(e, &[])
                };
                let p = obj(e, 0x30, pvt);
                set(e, p + 0x20, if n == 2 { 2 } else { 1 });
                p
            }
        };
        set(e, RETS + 0x708, 0);
        set(e, RETS + 0xac, c.word(4) & 1);
        set(e, RETS + 0x644, c.word(4) >> 1);
        set(e, RETS + 0x648, c.word(5));
        let geometry = if c.word(4) == 2 && c.word(3) == 0 {
            0
        } else {
            g
        };
        vec![r.renderer, b, geometry, 0x0e55_0000, s, packer, c.word(2)]
    })
    .run()
    .assert_same();
}

// ---- The direct call sites in the render family. ----

/// `prepare_streams` with the data's declaration's slot 0x8c the folded "false" and the shader's own
/// declaration's slot 0xb4 the typed build, and with a partition the typed skinned packer.
#[test]
fn prepare_streams_typed_slots() {
    Diff::new(entry!(
        0x00e8_12f0,
        stream::prepare_streams(Ptr, u32, u32, u32, u32) -> u32
    ))
    .common()
    .inputs(&[Input::Of(TWO_WAYS), Input::Of(TWO_WAYS), Input::Of(&[0, 1])])
    .setup(|e, c| {
        let own = declaration_world(e, 1, 2, &[0, 3], c.word(1), 0x0bd0_0003, 4);
        let r = renderer_world(e, &[]);
        let vt = vtable(e, &[]);
        let shader = obj(e, 0x40, vt);
        let device = obj(e, 0x10, vt);
        set(e, shader + 0x14, device);
        set(e, shader + 0x18, r.renderer);
        let target = obj(e, 0x10, vt);
        set(e, shader + 0x1c, target);
        let own_vt = e.mem.u32(own);
        let typed_vt = e.mem.alloc(0x200);
        for slot in (0..0x200).step_by(4) {
            let word = e.mem.u32(own_vt + slot);
            set(e, typed_vt + slot, word);
        }
        set(e, typed_vt + 0xb4, 0x00e7_6a80);
        set(e, own, typed_vt);
        set(e, shader + 0x28, own);
        let geometry = e.mem.alloc(0xc4);
        let data = e.mem.alloc(0x50);
        set(e, geometry + 0xb8, data);
        let declaration_vt = vtable(e, &[(0x8c, 0x0047_c850)]);
        let shader_declaration = obj(e, 0x30, declaration_vt);
        set(
            e,
            data + 0x30,
            if c.word(0) != 0 {
                shader_declaration
            } else {
                0
            },
        );
        let buffer = e.mem.alloc(0x60);
        set(e, data + 0x34, buffer);
        let partition = if c.word(2) != 0 {
            let p = e.mem.alloc(0x30);
            set(e, p + 0x28, buffer);
            p
        } else {
            0
        };
        // The plain geometry's packer: packed and clean, so it answers at once.
        let chips = e.mem.alloc(8);
        let chip = e.mem.alloc(0x18);
        set(e, chip + 8, 0x0e00_0008);
        set(e, chips, chip);
        set(e, buffer + 0x20, 1);
        set(e, buffer + 0x28, chips);
        let strides = e.mem.alloc(8);
        set(e, buffer + 0x24, strides);
        e.mem.set_u16(data + 0xe, 0x4000);
        vec![shader, geometry, partition, buffer, 0]
    })
    .run()
    .assert_same();
}
