//! UI Organizer's tile operators: the three alias names and the 17 new operators it registers in
//! UIOLoad, and its jump table, which runs every operation of `Tile::Value::CalculateValue` once
//! its patch at `00A0968C` is in (the game's 23 too, in UIO's own code, and the 17 new ones).
//!
//! Each operation is translated from UIO's `DoTileOperator*` (ui_organizer.h 1760-2360) with the
//! SSE instructions' own rules: `value` is the trait's float (`[eax+8]`), `operand` the action's
//! float (`[ebp-0x38]`), both as their bits where UIO tests bits (`cmp dword ptr .., 0` is true
//! for +0 alone; `test; jle` for +0, the negative numbers and the negative NaNs).

use engine_core::prelude::*;
use engine_core::sse;

/// `TileActionType` (UIO's gameapi.h): the game's 2000 (`kAction_copy`) .. `0x7E9`, then UIO's.
pub const COPY: i32 = 0x7d0;
pub const GTE: i32 = 0x7dd;
pub const NEQ: i32 = 0x7df;
pub const LTE: i32 = 0x7e1;
pub const NEG: i32 = 0x7ea;
pub const RAND: i32 = 0x7fa;
/// `NUM_OPERATORS`: the jump table's 43 entries, `0x7D0` to `0x7FA`.
pub const COUNT: u32 = 43;

/// UIOLoad's `RegTraitID` calls, in its order: three names for the game's operators, then the 17
/// new operators (`kAction_neg` `0x7EA` .. `kAction_rand` `0x7FA`).
pub const NAMES: [(&str, i32); 20] = [
    ("ge", GTE),
    ("ne", NEQ),
    ("le", LTE),
    ("neg", NEG),
    ("recipr", 0x7eb),
    ("land", 0x7ec),
    ("lor", 0x7ed),
    ("shl", 0x7ee),
    ("shr", 0x7ef),
    ("bt", 0x7f0),
    ("sqrt", 0x7f1),
    ("pow", 0x7f2),
    ("sin", 0x7f3),
    ("cos", 0x7f4),
    ("tan", 0x7f5),
    ("asin", 0x7f6),
    ("acos", 0x7f7),
    ("atan", 0x7f8),
    ("log", 0x7f9),
    ("rand", RAND),
];

/// `RegTraitID` (`009FF8A0`, `cdecl(name, id)`): the game's text table.
const REG_TRAIT_ID: u32 = 0x009f_f8a0;
/// The static `BSRandom` (`00476C00` constructs it on its first call) and
/// `BSRandom::UnsignedInt(max)` (`00AA5230`, `thiscall`), UIO's `ADDR_GetRandomInt` on `0x11C4180`.
const RANDOM_STATIC: u32 = 0x0047_6c00;
const RANDOM_UNSIGNED_INT: u32 = 0x00aa_5230;
const RANDOM_GENERATOR: u32 = 0x011c_4180;
/// The game's `strcmp` (UIO's `ADDR_strcmp`) and heap (`GAME_HEAP_ALLOC`/`GAME_HEAP_FREE`).
const STRCMP: u32 = 0x00ec_6da0;
const GAME_HEAP: u32 = 0x011f_6238;
const HEAP_ALLOCATE: u32 = 0x00aa_3e40;
const HEAP_FREE: u32 = 0x00aa_4060;
/// `TileValue` (UIO's gameapi.h): `num` at +8, `str` at +0xC.
const VALUE_NUM: u32 = 8;
const VALUE_STR: u32 = 0xc;
/// 1.0, the answer of UIO's tests and logic operators (`0x3F800000`).
const ONE: u32 = 0x3f80_0000;
/// UIO's `kFlt1d100` (0.01) and `kDblPId180` / `kDbl180dPI` (utility.h 250).
const HUNDREDTH: f32 = 0.01;
const DEGREE: f64 = 0.017_453_292_519_943_296;
const RADIAN: f64 = 57.295_779_513_082_32;

// Translated from UIO 2.30 (426d963), ui_organizer.h 1403-1426 and 1447: UIOLoad's `RegTraitID`
// calls and its `CdeclCall(0x476C00)`.
/// UIOLoad's start: the 20 names registered in the game's text table (each a copy on the game's
/// heap that is never freed, as UIO's names are its DLL's constant strings), then the static
/// `BSRandom` constructed (the `rand` operator draws from it).
pub fn on_load(e: &mut Engine) {
    for (name, id) in NAMES {
        let length = name.len() as u32;
        let at = e.call(HEAP_ALLOCATE, &args![GAME_HEAP, length + 1]).u32();
        if at == 0 {
            continue;
        }
        e.mem.write(at, name.as_bytes());
        e.mem.set_u8(at + length, 0);
        e.call(REG_TRAIT_ID, &args![at, id]);
    }
    e.call(RANDOM_STATIC, &[]);
}

/// SSE's conversion of a float already rounded to an integer: the integer, or the "integer
/// indefinite" `0x80000000` for a NaN or a number out of range.
fn to_int(rounded: f32) -> i32 {
    if (-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
        rounded as i32
    } else {
        i32::MIN
    }
}

/// `CVTSS2SI` / `CVTPS2DQ` under the game's rounding (to nearest, ties to even).
pub fn cvt_round(x: f32) -> i32 {
    // `round` takes halves away from zero; a half goes to the even neighbour instead.
    // (`x - round(x)` is exact for every float.)
    let away = x.round();
    let even = if (away - x).abs() == 0.5 {
        2.0 * (x / 2.0).round()
    } else {
        away
    };
    to_int(even)
}

/// `CVTTSS2SI`: truncated.
pub fn cvt_trunc(x: f32) -> i32 {
    to_int(x.trunc())
}

/// `RCPSS`: the processor's own estimate of `1 / x` (the instruction itself on x86_64, so the
/// same bits as UIO's on the same processor; elsewhere the exact quotient).
#[cfg(target_arch = "x86_64")]
pub fn rcpss(x: f32) -> f32 {
    use std::arch::x86_64::{_mm_cvtss_f32, _mm_rcp_ss, _mm_set_ss};
    // SAFETY: SSE is part of the x86_64 baseline.
    #[allow(unused_unsafe)]
    unsafe {
        _mm_cvtss_f32(_mm_rcp_ss(_mm_set_ss(x)))
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn rcpss(x: f32) -> f32 {
    1.0 / x
}

/// `COMISS value, operand` as flags: (ZF, CF); an unordered pair sets both (and PF).
fn comiss(value: f32, operand: f32) -> (bool, bool) {
    if value.is_nan() || operand.is_nan() {
        (true, true)
    } else {
        (value == operand, value < operand)
    }
}

/// 1.0 or 0 as UIO's `cmovcc ecx, 0x3F800000` writes them.
fn flag(condition: bool) -> u32 {
    if condition {
        ONE
    } else {
        0
    }
}

/// A double result stored as UIO's `cvtsd2ss` stores it.
fn single(x: f64) -> u32 {
    (x as f32).to_bits()
}

// Translated from UIO 2.30 (426d963), ui_organizer.h 1804-2360: DoTileOperatorADD ..
// DoTileOperatorRAND (DoTileOperatorCOPY in `copy`).
/// The trait's new float bits after the operation `kind` (every table entry but `copy`), or
/// `None` when UIO leaves the value as it is. `text` is whether the action brought a string
/// (`[ebp-0x34]`); `random(n)` is `BSRandom::UnsignedInt(n)` on the static generator.
pub fn arithmetic(
    kind: i32,
    value: u32,
    operand: u32,
    text: bool,
    random: &mut dyn FnMut(u32) -> u32,
) -> Option<u32> {
    let (v, o) = (f32::from_bits(value), f32::from_bits(operand));
    let positive = |bits: u32| (bits as i32) > 0;
    let pair = |f: fn(i32, i32) -> i32| (f(cvt_round(v), cvt_round(o)) as f32).to_bits();
    match kind {
        // add sub mul div min max
        0x7d1 => Some(sse::add(v, o).to_bits()),
        0x7d2 => Some(sse::sub(v, o).to_bits()),
        0x7d3 => Some(sse::mul(v, o).to_bits()),
        0x7d4 => (operand != 0).then(|| sse::div(v, o).to_bits()),
        0x7d5 => Some(sse::min(v, o).to_bits()),
        0x7d6 => Some(sse::max(v, o).to_bits()),
        // mod: on the rounded integers (`idiv`; UIO's 0x80000000 / -1 faults: here 0).
        0x7d7 => {
            let divisor = cvt_round(o);
            (divisor != 0).then(|| (cvt_round(v).wrapping_rem(divisor) as f32).to_bits())
        }
        // floor ceil: the sum converted under MXCSR rounding down / up.
        0x7d8 => Some((to_int(sse::add(v, o).floor()) as f32).to_bits()),
        0x7d9 => Some((to_int(sse::add(v, o).ceil()) as f32).to_bits()),
        // abs: of the sum, the sign bit cleared.
        0x7da => Some(sse::add(v, o).to_bits() & 0x7fff_ffff),
        // round: to a multiple of the operand, (value / operand + 0.01) rounded.
        0x7db => (value != 0 && positive(operand)).then(|| {
            let quotient = sse::add(sse::div(v, o), HUNDREDTH);
            sse::mul(cvt_round(quotient) as f32, o).to_bits()
        }),
        // gt gte eq neq lt lte (`comiss`, then `cmova`/`cmovnb`/`cmovz`/`cmovnz`/`cmovb`/`cmovbe`).
        0x7dc..=0x7e1 => {
            let (zf, cf) = comiss(v, o);
            Some(flag(match kind {
                0x7dc => !cf && !zf,
                0x7dd => !cf,
                0x7de => zf,
                0x7df => !zf,
                0x7e0 => cf,
                _ => cf || zf,
            }))
        }
        // and: a value of +0 stays; else 1 with a string, or whether the operand is nonzero.
        0x7e2 => (value != 0).then(|| flag(text || operand != 0)),
        // or: the value's bits (with the operand's when there is no string) nonzero.
        0x7e3 => {
            let bits = if text { value } else { value | operand };
            Some(flag(bits != 0))
        }
        // not: 0 with a string, else whether the operand is +0.
        0x7e4 => Some(flag(!text && operand == 0)),
        // onlyif / onlyifnot.
        0x7e5 => (operand == 0 && !text).then_some(0),
        0x7e6 => (operand != 0 || text).then_some(0),
        // neg: the operand with its sign flipped (+0 stays +0).
        0x7ea => Some(if operand == 0 {
            0
        } else {
            operand ^ 0x8000_0000
        }),
        // recipr: the processor's estimate of 1 / operand (+0 gives 0).
        0x7eb => Some(if operand == 0 { 0 } else { rcpss(o).to_bits() }),
        // land lor shl shr: on the rounded integers (`pslld`/`psrad` count from the whole
        // quadword: past 31, 0 / the sign).
        0x7ec => Some(pair(|a, b| a & b)),
        0x7ed => Some(pair(|a, b| a | b)),
        0x7ee => Some(pair(|a, b| if b as u32 > 31 { 0 } else { a << b })),
        0x7ef => Some(pair(|a, b| a >> (b as u32).min(31))),
        // bt: bit (operand & 31) of the value, both truncated.
        0x7f0 => {
            let bit = cvt_trunc(o) & 0x1f;
            Some(flag((cvt_trunc(v) >> bit) & 1 != 0))
        }
        // sqrt: of an operand without the sign bit.
        0x7f1 => ((operand as i32) >= 0).then(|| sse::sqrt(o).to_bits()),
        // pow sin cos tan asin acos atan log: in double (the CRT's functions).
        0x7f2 => Some(single((v as f64).powf(o as f64))),
        0x7f3 => Some(single(((o as f64) * DEGREE).sin())),
        0x7f4 => Some(single(((o as f64) * DEGREE).cos())),
        0x7f5 => Some(single(((o as f64) * DEGREE).tan())),
        0x7f6 => Some(single((o as f64).asin() * RADIAN)),
        0x7f7 => Some(single((o as f64).acos() * RADIAN)),
        0x7f8 => Some(single((o as f64).atan() * RADIAN)),
        0x7f9 => positive(operand).then(|| single((o as f64).ln())),
        // rand: 0 .. n-1 for n = the truncated operand; 0 when that is not positive.
        0x7fa => {
            let n = cvt_trunc(o);
            Some(if n > 0 {
                (random(n as u32) as i32 as f32).to_bits()
            } else {
                0
            })
        }
        // ref, begin, end (the parentheses), and anything outside the table: nothing.
        _ => None,
    }
}

// Translated from UIO 2.30 (426d963), ui_organizer.h 1760-1803: DoTileOperatorCOPY.
/// `copy`: without a string the operand's bits; with one, the value's string replaced by a copy
/// on the game's heap unless it is already the same text (then the "changed" flag is cleared);
/// a copy sets it. Answers the flag.
fn copy(e: &mut Engine, value: u32, operand: u32, text: u32, changed: bool) -> bool {
    if text == 0 {
        e.mem.set_u32(value + VALUE_NUM, operand);
        return changed;
    }
    let old = e.mem.u32(value + VALUE_STR);
    if old != 0 {
        if e.call(STRCMP, &args![text, old]).u32() == 0 {
            return false;
        }
        e.call(HEAP_FREE, &args![GAME_HEAP, old]);
    }
    let bytes = nvse::table::guest_text(e, text);
    let size = bytes.len() as u32 + 1;
    let copy = e.call(HEAP_ALLOCATE, &args![GAME_HEAP, size]).u32();
    // (UIO copies into what the heap answers without a test; the game's heap never answers 0.)
    if copy != 0 {
        e.mem.write(copy, &bytes);
        e.mem.set_u8(copy + size - 1, 0);
    }
    e.mem.set_u32(value + VALUE_STR, copy);
    true
}

// Translated from UIO 2.30 (426d963), ui_organizer.h 2444-2462: the patch at 0xA0968C (the
// action advanced, then `jmp kDoOperatorJumpTable[kind - 0x7D0]`, past the table back to the loop).
/// The seam at `00A0968C`: `[value, kind, operand bits, string, changed]`. Runs UIO's operation
/// (nothing for a kind outside its table) and answers the "string changed" flag; it always
/// answers, so the game's own switch never runs while UIO is on.
pub fn operate(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (value, kind, operand, text, changed) = (a[0], a[1] as i32, a[2], a[3], a[4] != 0);
    if kind == COPY {
        return Some(copy(e, value, operand, text, changed) as u32);
    }
    let current = e.mem.u32(value + VALUE_NUM);
    let mut random = |n: u32| {
        e.call(RANDOM_UNSIGNED_INT, &args![RANDOM_GENERATOR, n])
            .u32()
    };
    if let Some(bits) = arithmetic(kind, current, operand, text != 0, &mut random) {
        e.mem.set_u32(value + VALUE_NUM, bits);
    }
    Some(changed as u32)
}

#[cfg(test)]
#[path = "operators_tests.rs"]
mod tests;
