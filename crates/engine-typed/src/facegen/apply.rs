//! A head's morphs applied each frame (`BSFaceGenNiNode`'s morph pass, `00663050`): for each child
//! geometry with morph data, the base copy once, then every morph of each kind whose weight is in
//! `(0, 1]`, each through the morph data's table to its `ApplyMorph`. The translations reach each
//! morph through four or five calls by address (the node's animation data, its weight getter, the
//! morph data's apply, the table getter, the morph's virtual `ApplyMorph`); here they are direct
//! when the slot holds the class's own function.
//!
//! On top, the exact skip ([`skip`]): the exe reshapes every head with any weight above zero every
//! frame, also when nothing changed; here a reshape that would write exactly what the vertex array
//! already holds (the same base, morphs and weights, and the array untouched since) is left out.
//! The vertices are the same words either way.

use super::{keyframe, morph, FAMILY};
use crate::native::{float_precision, Real};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The node's `GetAnimationData` slot (+0x100) and its own function there (`006629f0`, typed in
/// the multibound family as the folded getter of the word at +0xac).
const SLOT_GET_ANIMATION_DATA: u32 = 0x100;
const OWN_GET_ANIMATION_DATA: u32 = 0x0066_29f0;
/// The animation data's weight getters: expressions (+0x94, `006499e0`), phonemes (+0xa4,
/// `00649aa0`), the custom morph (+0xac, `00649b10`).
const SLOT_EXPRESSION: u32 = 0x94;
const SLOT_PHONEME: u32 = 0xa4;
const SLOT_CUSTOM: u32 = 0xac;
const OWN_EXPRESSION: u32 = 0x0064_99e0;
const OWN_PHONEME: u32 = 0x0064_9aa0;
const OWN_CUSTOM: u32 = 0x0064_9b10;
/// A morph's `ApplyMorph` slot (+4) and the two kinds' own functions.
const SLOT_APPLY_MORPH: u32 = 0x4;
const DIFFERENTIAL_APPLY: u32 = 0x0066_0860;
const STATISTICAL_APPLY: u32 = 0x0066_0a90;

/// The node's animation data (slot +0x100).
#[inline(always)]
fn animation_data(e: &mut Engine, node: u32) -> u32 {
    if crate::slot_target(e, node, SLOT_GET_ANIMATION_DATA) == OWN_GET_ANIMATION_DATA {
        typed_call!(
            crate::multibound::FAMILY,
            0x0066_29f0,
            crate::multibound::getters::multibound_of(e, Ptr::new(node))
        )
    } else {
        e.vcall(node, SLOT_GET_ANIMATION_DATA, &[]).u32()
    }
}

/// The value below `limit` of the keyframe at `data + offset` (`005dd4e0`), else 0.0.
#[inline(always)]
fn value_below(e: &mut Engine, data: u32, index: i32, limit: i32, offset: u32) -> f32 {
    if index < limit {
        typed_call!(
            FAMILY,
            0x005d_d4e0,
            keyframe::quantity(e, Ptr::new(data.wrapping_add(offset)), index as u32)
        )
    } else {
        0.0
    }
}

// Typed from 006499e0
/// `thiscall(data, index)`: expression weight `index` below 15 (the keyframe at +0x74), else 0.
pub fn expression_value(e: &mut Engine, this: Ptr, index: i32) -> f32 {
    value_below(e, this.addr(), index, 0xf, 0x74)
}

// Typed from 00649aa0
/// `thiscall(data, index)`: phoneme weight `index` below 16 (the keyframe at +0xdc), else 0.
pub fn phoneme_value(e: &mut Engine, this: Ptr, index: i32) -> f32 {
    value_below(e, this.addr(), index, 0x10, 0xdc)
}

// Typed from 00649b10
/// `thiscall(data, index)`: the custom morph's weight for index 0 (the keyframe at +0x110), else 0.
pub fn custom_value(e: &mut Engine, this: Ptr, index: i32) -> f32 {
    value_below(e, this.addr(), index, 1, 0x110)
}

// Typed from 005dd4a0
/// `thiscall(data, index)`: modifier weight `index` below 17 (signed; the array at +0xa8, read in
/// place as the translation's plain call reads it), else 0.
pub fn modifier_value(e: &mut Engine, this: Ptr, index: i32) -> f32 {
    if index < 0x11 {
        let k = this.addr().wrapping_add(0xa8);
        if (index as u32) < e.mem.u32(k.wrapping_add(0x10)) {
            let data = e.mem.u32(k.wrapping_add(0xc));
            return e.mem.f32(data.wrapping_add((index as u32).wrapping_mul(4)));
        }
    }
    0.0
}

/// Weight `index` of the slot's getter on the animation data, the getter direct when the slot
/// holds the class's own.
#[inline(always)]
fn slot_value(e: &mut Engine, data: u32, slot: u32, index: i32) -> f32 {
    let target = crate::slot_target(e, data, slot);
    let own = match slot {
        SLOT_EXPRESSION => OWN_EXPRESSION,
        SLOT_PHONEME => OWN_PHONEME,
        _ => OWN_CUSTOM,
    };
    if target != own {
        let st0 = e.vcall(data, slot, &args![index as u32]).f64();
        return crate::native::to_float(st0);
    }
    let this = Ptr::new(data);
    match slot {
        SLOT_EXPRESSION => typed_call!(FAMILY, 0x0064_99e0, expression_value(e, this, index)),
        SLOT_PHONEME => typed_call!(FAMILY, 0x0064_9aa0, phoneme_value(e, this, index)),
        _ => typed_call!(FAMILY, 0x0064_9b10, custom_value(e, this, index)),
    }
}

/// Table slot `index` (`0..count`) at `this + offset`, or null (`006620d0`, `00662110`,
/// `00662150`, `00662190`).
#[inline(always)]
fn table_slot(e: &Engine, this: u32, offset: u32, count: i32, index: i32) -> u32 {
    let table = e.mem.u32(this.wrapping_add(offset));
    if table == 0 || index <= -1 || index >= count {
        return 0;
    }
    e.mem
        .u32(table.wrapping_add((index as u32).wrapping_mul(4)))
}

// Typed from 006620d0
/// The expression morph `index` (0..15), or null.
pub fn expression_morph(e: &mut Engine, this: Ptr, index: i32) -> u32 {
    table_slot(e, this.addr(), 0x8, 15, index)
}

// Typed from 00662110
/// The modifier morph `index` (0..17), or null.
pub fn modifier_morph(e: &mut Engine, this: Ptr, index: i32) -> u32 {
    table_slot(e, this.addr(), 0xc, 17, index)
}

// Typed from 00662150
/// The phoneme morph `index` (0..16), or null.
pub fn phoneme_morph(e: &mut Engine, this: Ptr, index: i32) -> u32 {
    table_slot(e, this.addr(), 0x10, 16, index)
}

// Typed from 00662190
/// The custom morph `index` (0..1), or null.
pub fn custom_morph(e: &mut Engine, this: Ptr, index: i32) -> u32 {
    table_slot(e, this.addr(), 0x14, 1, index)
}

// Typed from 00662030
/// `thiscall(morph data, kind, index, target, base, weight)`: morph `index` of `kind` (0
/// phonemes, 1 expressions, 2 modifiers, 3 the custom one) applied to `target` (`ApplyMorph`,
/// slot +4, direct for the two morph kinds), when there is one.
pub fn apply_one(
    e: &mut Engine,
    this: Ptr,
    kind: u32,
    index: i32,
    target: u32,
    base: u32,
    weight: f32,
) {
    let t = this.addr();
    let morph = match kind {
        0 => table_slot(e, t, 0x10, 16, index),
        1 => table_slot(e, t, 0x8, 15, index),
        2 => table_slot(e, t, 0xc, 17, index),
        3 => typed_call!(FAMILY, 0x0066_2190, custom_morph(e, this, index)),
        _ => 0,
    };
    if morph == 0 {
        return;
    }
    let m = Ptr::new(morph);
    match crate::slot_target(e, morph, SLOT_APPLY_MORPH) {
        DIFFERENTIAL_APPLY => typed_call!(
            FAMILY,
            0x0066_0860,
            morph::differential_apply(e, m, target, base, weight)
        ),
        STATISTICAL_APPLY => typed_call!(
            FAMILY,
            0x0066_0a90,
            morph::statistical_apply(e, m, target, base, weight)
        ),
        _ => {
            e.vcall(morph, SLOT_APPLY_MORPH, &args![target, base, weight]);
        }
    }
}

/// One write of a morph pass, in the order the pass makes it: the base copy, or one morph applied
/// (its kind, index and weight, and the morph's identity: the object, its vtable and its three
/// words, which the result depends on).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Op {
    Prepare,
    Apply {
        kind: u32,
        index: u32,
        value: u32,
        morph: [u32; 5],
    },
}

/// Where a pass's writes go: made at once (`defer` false, as the exe makes them) and noted, or
/// only noted, to be made at the end unless they would give what the array already holds
/// ([`skip`]).
struct Sink {
    defer: bool,
    ops: Vec<Op>,
}

impl Sink {
    fn now() -> Sink {
        Sink {
            defer: false,
            ops: Vec::new(),
        }
    }
}

/// The base copy of the context's geometry (`00653340`) through the sink.
#[inline(always)]
fn prepare(e: &mut Engine, context: u32, sink: &mut Sink) {
    sink.ops.push(Op::Prepare);
    if !sink.defer {
        let geometry = e.mem.u32(context);
        typed_call!(
            FAMILY,
            0x0065_3340,
            morph::copy_base(e, geometry, context + 4)
        );
    }
}

/// The morph `00662030` reaches for `kind` and `index` (its table read in place), with the words it
/// reads, or zeros.
fn morph_identity(e: &Engine, data: u32, kind: u32, index: u32) -> [u32; 5] {
    let (offset, count) = match kind {
        0 => (0x10, 16),
        1 => (0x8, 15),
        2 => (0xc, 17),
        3 => (0x14, 1),
        _ => return [0; 5],
    };
    let morph = table_slot(e, data, offset, count, index as i32);
    if morph == 0 {
        return [0; 5];
    }
    [
        morph,
        e.mem.u32(morph),
        e.mem.u32(morph.wrapping_add(4)),
        e.mem.u32(morph.wrapping_add(8)),
        e.mem.u32(morph.wrapping_add(0xc)),
    ]
}

/// One morph applied (`00662030` on the context's morph data, geometry and base).
#[inline(always)]
fn apply_now(e: &mut Engine, context: u32, kind: u32, index: u32, value: f32) {
    let data = e.mem.u32(context + 0x10);
    let geometry = e.mem.u32(context);
    let base = e.mem.u32(context + 0x14);
    typed_call!(
        FAMILY,
        0x0066_2030,
        apply_one(e, Ptr::new(data), kind, index as i32, geometry, base, value)
    );
}

/// One morph of `kind` and `index` with `value` when `0 < value <= 1` (a NaN never): the context's
/// geometry gets its base copy once (`00653340`, the flag at +0x1c), then `00662030` on the
/// context's morph data.
#[inline(always)]
fn apply_morph(e: &mut Engine, context: u32, kind: u32, index: u32, value: f32, sink: &mut Sink) {
    if !(value > 0.0 && value <= 1.0) {
        return;
    }
    if e.mem.u8(context + 0x1c) == 0 {
        prepare(e, context, sink);
        e.mem.set_u8(context + 0x1c, 1);
    }
    let data = e.mem.u32(context + 0x10);
    let morph = morph_identity(e, data, kind, index);
    sink.ops.push(Op::Apply {
        kind,
        index,
        value: value.to_bits(),
        morph,
    });
    if !sink.defer {
        apply_now(e, context, kind, index, value);
    }
}

/// The shape of `00662bc0`, `00662e90` and `00662f70`: for each index below `count`, the
/// animation data's weight from `slot` applied as a morph of `kind` (the data asked again each
/// step, as there).
#[allow(clippy::too_many_arguments)]
fn apply_slot_morphs(
    e: &mut Engine,
    node: u32,
    context: u32,
    count: i32,
    slot: u32,
    kind: u32,
    sink: &mut Sink,
) {
    for i in 0..count {
        let data = animation_data(e, node);
        let value = slot_value(e, data, slot, i);
        apply_morph(e, context, kind, i as u32, value, sink);
    }
}

// Typed from 00662bc0
/// `thiscall(node, context)`: the 15 expression morphs (kind 1).
pub fn apply_expressions(e: &mut Engine, this: Ptr, context: u32) {
    let mut sink = Sink::now();
    apply_slot_morphs(e, this.addr(), context, 0xf, SLOT_EXPRESSION, 1, &mut sink);
}

// Typed from 00662e90
/// `thiscall(node, context)`: the 16 phoneme morphs (kind 0).
pub fn apply_phonemes(e: &mut Engine, this: Ptr, context: u32) {
    let mut sink = Sink::now();
    apply_slot_morphs(e, this.addr(), context, 0x10, SLOT_PHONEME, 0, &mut sink);
}

// Typed from 00662f70
/// `thiscall(node, context)`: the custom morph (kind 3).
pub fn apply_custom(e: &mut Engine, this: Ptr, context: u32) {
    let mut sink = Sink::now();
    apply_slot_morphs(e, this.addr(), context, 1, SLOT_CUSTOM, 3, &mut sink);
}

/// Modifier weight `index` of the node's animation data (`005dd4a0`).
#[inline(always)]
fn modifier(e: &mut Engine, node: u32, index: i32) -> f32 {
    let data = animation_data(e, node);
    typed_call!(
        FAMILY,
        0x005d_d4a0,
        modifier_value(e, Ptr::new(data), index)
    )
}

// Typed from 00653d70
/// `cdecl(a, b, below, above)`: `b - a` (the exe's `a * -1.0 + b`, exact as one subtraction)
/// split: below zero (or a NaN) negated into `below` with 0 in `above`, else 0 in `below` and the
/// difference in `above`.
pub fn split_pair(e: &mut Engine, a: f32, b: f32, below: u32, above: u32) {
    let difference = b + -a;
    if difference >= 0.0 {
        e.mem.set_f32(below, 0.0);
        e.mem.set_f32(above, difference);
    } else {
        e.mem.set_f32(below, -difference);
        e.mem.set_f32(above, 0.0);
    }
}

/// [`apply_modifiers`] through a sink.
fn modifiers_into(e: &mut Engine, node: u32, context: u32, sink: &mut Sink) {
    e.with_stack(0x10, |e, pair| {
        let (first, second) = (pair.addr(), pair.addr() + 4);
        for i in 0..0x11i32 {
            let value = match i {
                8..=0xb => {
                    let (low, high) = if i == 8 || i == 0xb {
                        (8, 0xb)
                    } else {
                        (9, 0xa)
                    };
                    let upper = modifier(e, node, high);
                    let lower = modifier(e, node, low);
                    typed_call!(
                        FAMILY,
                        0x0065_3d70,
                        split_pair(e, lower, upper, first, second)
                    );
                    let chosen = if i == low { first } else { second };
                    e.mem.f32(chosen)
                }
                0xe..=0x10 => continue,
                _ => modifier(e, node, i),
            };
            apply_morph(e, context, 2, i as u32, value, sink);
        }
    });
}

// Typed from 00662ca0
/// `thiscall(node, context)`: the modifier morphs (kind 2) for indices 0 to 13 with the animation
/// data's weights; 8 and 11, and 9 and 10, are pairs split by `00653d70` (the lower index's
/// weight first); 14 to 16 are skipped.
pub fn apply_modifiers(e: &mut Engine, this: Ptr, context: u32) {
    let mut sink = Sink::now();
    modifiers_into(e, this.addr(), context, &mut sink);
}

/// The exact skip: a head's vertex array is reshaped only when the reshape would write something
/// other than what the array holds. Per array (by its address, per run) the pass keeps what fed its
/// last reshape: the identities (geometry, its data, the morph data, the base and its offsets, the
/// array and its stride, the vertex count, the thread's precision), the writes in order (each
/// morph's kind, index, weight and identity), a copy of the base offsets and of the positions it
/// left. A pass whose identities, base offsets and array positions all still match notes its
/// writes instead of making them; when the notes equal the kept ones the reshape would give the
/// same positions from the same inputs, so it is left out (with its `MarkAsChanged`: the data did
/// not change). Anything else (another writer, a reused address, a changed weight or base) makes
/// the writes as the exe does and keeps them anew. Not kept, so never skipped: a morph whose
/// `ApplyMorph` is not one of the two own functions (a class whose effects are not known) and a
/// statistical index past the vertex count (that vertex is not reset by the base copy).
pub(crate) mod skip {
    use super::Op;
    use engine_core::Engine;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// What fed one reshape and what it left.
    pub(super) struct Kept {
        key: [u32; 9],
        ops: Vec<Op>,
        base: Vec<u32>,
        positions: Vec<u32>,
    }

    /// The kept reshapes, by run (the kernel table's address), then by array address.
    static KEPT: Mutex<Vec<(usize, HashMap<u32, Kept>)>> = Mutex::new(Vec::new());
    /// At most this many arrays a run (more are forgotten all at once) and runs (the oldest go).
    const MAX_ARRAYS: usize = 1024;
    const MAX_RUNS: usize = 8;

    thread_local! {
        /// A statistical morph wrote past the vertex count during this reshape.
        static PAST_COUNT: Cell<bool> = const { Cell::new(false) };
    }

    pub(crate) fn note_past_count() {
        PAST_COUNT.with(|c| c.set(true));
    }

    pub(super) fn clear_past_count() {
        PAST_COUNT.with(|c| c.set(false));
    }

    fn past_count() -> bool {
        PAST_COUNT.with(Cell::get)
    }

    /// The positions of `count` vertices at `array` (three words a vertex, one a stride).
    pub(super) fn positions(e: &Engine, array: u32, stride: u32, count: u32) -> Vec<u32> {
        let mut out = Vec::with_capacity(3 * count as usize);
        let mut at = array;
        for _ in 0..count {
            out.push(e.mem.u32(at));
            out.push(e.mem.u32(at.wrapping_add(4)));
            out.push(e.mem.u32(at.wrapping_add(8)));
            at = at.wrapping_add(stride);
        }
        out
    }

    /// `3 * count` words from `offsets`.
    pub(super) fn words(e: &Engine, offsets: u32, count: u32) -> Vec<u32> {
        (0..3 * count)
            .map(|k| e.mem.u32(offsets.wrapping_add(4 * k)))
            .collect()
    }

    /// Whether the array's kept reshape has these identities and still matches the base offsets and
    /// the array's positions: then the pass notes its writes instead of making them.
    pub(super) fn candidate(e: &Engine, key: &[u32; 9]) -> bool {
        let run = e.kernel.id();
        let kept = KEPT.lock().unwrap_or_else(|p| p.into_inner());
        let Some((_, arrays)) = kept.iter().find(|(r, _)| *r == run) else {
            return false;
        };
        let Some(k) = arrays.get(&key[4]) else {
            return false;
        };
        let (array, stride, count, offsets) = (key[4], key[5], key[6], key[8]);
        k.key == *key
            && k.base == words(e, offsets, count)
            && k.positions == positions(e, array, stride, count)
    }

    /// Whether `ops` are the kept reshape's writes (asked only of a candidate).
    pub(super) fn same_writes(e: &Engine, key: &[u32; 9], ops: &[Op]) -> bool {
        let run = e.kernel.id();
        let kept = KEPT.lock().unwrap_or_else(|p| p.into_inner());
        kept.iter()
            .find(|(r, _)| *r == run)
            .and_then(|(_, arrays)| arrays.get(&key[4]))
            .is_some_and(|k| k.key == *key && k.ops == ops)
    }

    /// Keeps the reshape just made (or forgets the array when it may not be skipped).
    pub(super) fn keep(e: &Engine, key: [u32; 9], ops: Vec<Op>) {
        let run = e.kernel.id();
        let own = ops.iter().all(|op| match op {
            Op::Prepare => true,
            Op::Apply { morph, .. } => {
                morph[0] == 0 || {
                    let apply = e.mem.u32(morph[1].wrapping_add(4));
                    apply == super::DIFFERENTIAL_APPLY || apply == super::STATISTICAL_APPLY
                }
            }
        });
        let keepable = own && !past_count();
        let (array, stride, count, offsets) = (key[4], key[5], key[6], key[8]);
        let fresh = keepable.then(|| Kept {
            key,
            ops,
            base: words(e, offsets, count),
            positions: positions(e, array, stride, count),
        });
        let mut kept = KEPT.lock().unwrap_or_else(|p| p.into_inner());
        let at = match kept.iter().position(|(r, _)| *r == run) {
            Some(at) => at,
            None => {
                if kept.len() >= MAX_RUNS {
                    kept.remove(0);
                }
                kept.push((run, HashMap::new()));
                kept.len() - 1
            }
        };
        let arrays = &mut kept[at].1;
        match fresh {
            Some(k) => {
                if arrays.len() >= MAX_ARRAYS && !arrays.contains_key(&array) {
                    arrays.clear();
                }
                arrays.insert(array, k);
            }
            None => {
                arrays.remove(&array);
            }
        }
    }
}

/// Makes the noted writes, in order (a deferred pass whose writes differ from the kept ones).
fn make(e: &mut Engine, context: u32, ops: &[Op]) {
    for op in ops {
        match *op {
            Op::Prepare => {
                let geometry = e.mem.u32(context);
                typed_call!(
                    FAMILY,
                    0x0065_3340,
                    morph::copy_base(e, geometry, context + 4)
                );
            }
            Op::Apply {
                kind, index, value, ..
            } => apply_now(e, context, kind, index, f32::from_bits(value)),
        }
    }
}

/// `distance <= limit + ten` at the thread's precision (the sum is not stored).
#[inline(always)]
fn within<R: Real>(distance: f32, limit: f32, ten: f32) -> bool {
    let r = |v: f32| R::load(v.to_bits());
    r(distance) <= r(limit) + r(ten)
}

/// `distance > limit - ten` at the thread's precision.
#[inline(always)]
fn beyond<R: Real>(distance: f32, limit: f32, ten: f32) -> bool {
    let r = |v: f32| R::load(v.to_bits());
    r(distance) > r(limit) - r(ten)
}

/// The by-address callees of the morph pass.
mod at {
    pub const CHILD_COUNT: u32 = 0x0043_b480;
    pub const CHILD_AT: u32 = 0x0043_b4a0;
    pub const GEOMETRY_DATA: u32 = 0x0054_95f0;
    pub const MORPH_DATA: u32 = 0x0065_3230;
    pub const MORPH_BASE: u32 = 0x0065_3430;
    pub const DATA_LOCK: u32 = 0x00a6_7360;
    pub const DATA_BEGIN: u32 = 0x00a6_7410;
    pub const DATA_UNLOCK: u32 = 0x00a6_73d0;
    pub const FACE_VERTICES: u32 = 0x0045_6650;
    pub const SETTING_REAL: u32 = 0x0045_0410;
    pub const FAR_SETTING: u32 = 0x011d_5b28;
    pub const NEAR_SETTING: u32 = 0x011d_5b34;
    pub const CAMERA_HOLDER: u32 = 0x011d_59ec;
    pub const DEFAULT_POSITION: u32 = 0x011f_426c;
    pub const POSITION_OF: u32 = 0x0045_bb80;
    pub const DIFFERENCE: u32 = 0x0043_9ef0;
    pub const LENGTH: u32 = 0x0045_7990;
    pub const QUARTERED: u32 = 0x0052_5420;
    pub const IN_MENU_MODE: u32 = 0x0070_2360;
    pub const PARENT: u32 = 0x0096_11e0;
    pub const TEN: u32 = 0x0101_7b78;
    pub const QUARTER: u32 = 0x0102_90b0;
}

/// The node's fields the pass reads (`BSFaceGenNiNode`, PC offsets).
const FORCE_BASE_MORPH: u32 = 0xd4;
const IN_DIALOGUE: u32 = 0xe2;
const DISABLE_MORPH_UPDATES: u32 = 0xe3;
const FORCE_MORPH_UPDATES: u32 = 0xe4;

/// The camera's position (`006633f0`): `0045bb80` of the object at `011d59ec`, or `011f426c`.
fn camera_position(e: &mut Engine) -> u32 {
    let holder = e.mem.u32(at::CAMERA_HOLDER);
    if holder == 0 {
        at::DEFAULT_POSITION
    } else {
        e.call(at::POSITION_OF, &args![holder]).u32()
    }
}

/// A setting's float (`006633d0`, `006633e0`: `00450410` on it), stored.
fn setting(e: &mut Engine, which: u32) -> f32 {
    crate::native::to_float(e.call(at::SETTING_REAL, &args![which]).f64())
}

// Typed from 00663050
/// `thiscall(node, force)`, `RET 4`: the morph pass of a head (see the module). Per child
/// geometry with data and morph data whose data locks (`00a67360(1)`): the context
/// `{geometry, cursor, morph data, base, vertex count, prepared}`; the base copy first when
/// forced (`force` or `bForceBaseMorph`, cleared) or with `bForceMorphUpdates`; within the near
/// setting (+10; both settings quartered when `00525420`), or in dialogue, or in menu mode, the
/// phonemes (unless morph updates are disabled), and within the far setting also the
/// expressions, modifiers and the custom morph. A face whose data's +0xd4 is set and whose
/// expressions did not run still gets its modifiers when modifier 0 is above zero and the parent's
/// slot +0x10 answers; one without that, beyond the far setting less 10 and given nothing, has its
/// animation reset (slot +0xb4). Unlocked after each; `bForceMorphUpdates` cleared at the end.
pub fn morph_pass(e: &mut Engine, this: Ptr, force: u8) {
    let node = this.addr();
    e.with_stack(0x70, |e, frame| {
        let local = |offset: u32| frame.addr() + 0x70 - offset;
        let context = local(0x24);
        // The context's cursor constructed in place (`00534550` -> `00540720`: two words and a byte
        // zeroed: no call, so no seam is lost; neither address is an extender hook).
        e.mem.set_u32(context + 4, 0);
        e.mem.set_u32(context + 8, 0);
        e.mem.set_u8(context + 0xc, 0);
        let count = e.call(at::CHILD_COUNT, &args![node]).u32();
        let mut i = 0u32;
        while i < count {
            e.mem.set_u8(context + 0x1c, 0);
            let child = e.call(at::CHILD_AT, &args![node, i]).u32();
            i += 1;
            if child == 0 {
                continue;
            }
            let geometry = e.vcall(child, 0x1c, &[]).u32();
            e.mem.set_u32(context, geometry);
            if geometry == 0 || e.call(at::GEOMETRY_DATA, &args![geometry]).u32() == 0 {
                continue;
            }
            let morph_data = e.call(at::MORPH_DATA, &args![geometry]).u32();
            e.mem.set_u32(context + 0x10, morph_data);
            if morph_data == 0 {
                continue;
            }
            let base = e.call(at::MORPH_BASE, &args![geometry]).u32();
            e.mem.set_u32(context + 0x14, base);
            if base == 0 {
                continue;
            }
            let data = e.call(at::GEOMETRY_DATA, &args![geometry]).u32();
            if !e.call(at::DATA_LOCK, &args![data, 1u32]).bool() {
                continue;
            }
            let data = e.call(at::GEOMETRY_DATA, &args![geometry]).u32();
            e.call(at::DATA_BEGIN, &args![data, context + 4]);
            let vertices = u32::from(e.call(at::FACE_VERTICES, &args![geometry]).u16());
            e.mem.set_u32(context + 0x18, vertices);
            // The exact skip (see [`skip`]): what this reshape depends on.
            let key = [
                geometry,
                data,
                morph_data,
                base,
                e.mem.u32(context + 4),
                e.mem.u32(context + 8),
                vertices,
                u32::from(float_precision()),
                e.mem.u32(base.wrapping_add(0xc)),
            ];
            let mut sink = Sink {
                defer: skip::candidate(e, &key),
                ops: Vec::new(),
            };
            skip::clear_past_count();
            if force != 0 || e.mem.u8(node + FORCE_BASE_MORPH) != 0 {
                prepare(e, context, &mut sink);
                e.mem.set_u8(context + 0x1c, 1);
                e.mem.set_u8(node + FORCE_BASE_MORPH, 0);
            }
            let position = camera_position(e);
            let camera = local(0x3c);
            for k in 0..3 {
                let word = e.mem.u32(position + 4 * k);
                e.mem.set_u32(camera + 4 * k, word);
            }
            let ten = e.mem.f32(at::TEN);
            let own = e.call(at::POSITION_OF, &args![node]).u32();
            let difference = e
                .call(at::DIFFERENCE, &args![camera, local(0x5c), own])
                .u32();
            let distance = crate::native::to_float(e.call(at::LENGTH, &args![difference]).f64());
            let mut far = setting(e, at::FAR_SETTING);
            let mut near = setting(e, at::NEAR_SETTING);
            if e.call(at::QUARTERED, &[]).bool() {
                let quarter: f64 = e.global(at::QUARTER);
                (far, near) = if float_precision() {
                    let q = f32::narrow(quarter);
                    (far * q, near * q)
                } else {
                    (
                        crate::native::to_float(f64::from(far) * quarter),
                        crate::native::to_float(f64::from(near) * quarter),
                    )
                };
            }
            let single = float_precision();
            let close = |e: &mut Engine, limit: f32| -> bool {
                let inside = if single {
                    within::<f32>(distance, limit, ten)
                } else {
                    within::<f64>(distance, limit, ten)
                };
                inside || e.mem.u8(node + IN_DIALOGUE) != 0 || e.call(at::IN_MENU_MODE, &[]).bool()
            };
            let mut near_enough = false;
            let mut close_enough = false;
            if close(e, near) {
                near_enough = true;
                if close(e, far) {
                    close_enough = true;
                }
            }
            if e.mem.u8(node + FORCE_MORPH_UPDATES) != 0 && e.mem.u8(context + 0x1c) == 0 {
                prepare(e, context, &mut sink);
                e.mem.set_u8(context + 0x1c, 1);
            }
            let mut all_applied = false;
            let mut base_applied = false;
            if near_enough && e.mem.u8(node + DISABLE_MORPH_UPDATES) == 0 {
                apply_slot_morphs(e, node, context, 0x10, SLOT_PHONEME, 0, &mut sink);
                base_applied = true;
                if close_enough {
                    apply_slot_morphs(e, node, context, 0xf, SLOT_EXPRESSION, 1, &mut sink);
                    modifiers_into(e, node, context, &mut sink);
                    apply_slot_morphs(e, node, context, 1, SLOT_CUSTOM, 3, &mut sink);
                    all_applied = true;
                }
            }
            let animation = animation_data(e, node);
            if e.vcall(animation, 0xd4, &[]).bool() {
                if !all_applied && modifier(e, node, 0) > 0.0 {
                    let parent = e.call(at::PARENT, &args![node]).u32();
                    if e.vcall(parent, 0x10, &[]).u32() != 0 {
                        modifiers_into(e, node, context, &mut sink);
                    }
                }
            } else if !base_applied {
                let far = setting(e, at::FAR_SETTING);
                let outside = if single {
                    beyond::<f32>(distance, far, ten)
                } else {
                    beyond::<f64>(distance, far, ten)
                };
                if outside {
                    let animation = animation_data(e, node);
                    e.vcall(
                        animation,
                        0xb4,
                        &args![0.0f32, 1u32, 1u32, 1u32, 1u32, 0u32],
                    );
                }
            }
            if !sink.ops.is_empty() {
                if !sink.defer {
                    skip::keep(e, key, sink.ops);
                } else if !skip::same_writes(e, &key, &sink.ops) {
                    make(e, context, &sink.ops);
                    skip::keep(e, key, sink.ops);
                }
            }
            let data = e.call(at::GEOMETRY_DATA, &args![geometry]).u32();
            e.call(at::DATA_UNLOCK, &args![data]);
        }
    });
    e.mem.set_u8(node + FORCE_MORPH_UPDATES, 0);
}
