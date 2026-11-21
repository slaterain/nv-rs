//! `CExpression`: a script line's expression, its conversion from infix text to postfix (the
//! compiler's), its tokenizer, and `Eval`, which every `if`, `set` and command argument of a running
//! script goes through. The object: an error word (+0), the infix text (+4), the postfix text
//! (+0x204), the operator stack (+0x404: 64 words and the index at +0x100) and the operand stack
//! (+0x508: 32 operands of 16 bytes and the index at +0x200); 0x710 bytes.
//!
//! An operand on the stack: a type byte in the low byte of word 0 (0 an integer, anything else a
//! double), word 1 unused, the value at word 2 (an integer) or words 2 and 3 (a double). The other
//! bytes of words 0 and 1 are what the evaluator's own operand held before: the exe never clears
//! them, so they are carried as they are.

use super::numbers;
use super::FAMILY;
use crate::crt;
use crate::native::float_precision;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87;
use engine_core::{args, typed_call, Engine};

/// Offsets inside `CExpression`.
const ERROR: u32 = 0;
const INFIX: u32 = 0x4;
const POSTFIX: u32 = 0x204;
const OPERATORS: u32 = 0x404;
const OPERANDS: u32 = 0x508;
/// The operator stack's index, from the stack's start; and the operand stack's.
const OPERATOR_INDEX: u32 = 0x100;
const OPERAND_INDEX: u32 = 0x200;

/// The operator table (`0118cad0`): 16 entries of 8 bytes, each a word (the operator's number), a signed
/// precedence byte (+4) and a text of up to 3 bytes (+5). Number 16 means "operand" and 15 is
/// unary minus (`~`).
const OPERATOR_PRECEDENCE: u32 = 0x0118_cad4;
const OPERATOR_TEXT: u32 = 0x0118_cad5;
const OPERAND: u32 = 0x10;
const NEGATE: u32 = 0xf;
/// The error names, one pointer per error code, and `"%s"`, the format they are reported with.
const ERROR_NAMES: u32 = 0x0118_c594;
const PERCENT_S: u32 = 0x0101_9f08;
/// `SCRIPTS: Unhandled operator '%s' in Expression::Eval()` and the script log it goes to.
const UNHANDLED_OPERATOR: u32 = 0x0103_3fd0;
const SCRIPT_LOG: u32 = 0x005b_5e40;

const STRLEN: u32 = 0x0044_a670;
/// The tokenizer, by address: lStewieAl's Tweaks' script inlines replace it (`overridable.tsv`).
const NEXT_TOKEN: u32 = 0x0059_3870;
/// The operand reader, by address (`overridable.tsv`).
const READ_OPERAND: u32 = 0x005a_c7a0;
/// The CRT's `atol` body and `atof`.
const ATOL: u32 = 0x00ec_a6bd;
const ATOF: u32 = 0x00ec_a573;

/// A 16-byte operand as it sits on the operand stack.
#[derive(Clone, Copy)]
struct Operand([u32; 4]);

impl Operand {
    #[inline]
    fn read(e: &Engine, at: u32) -> Operand {
        Operand([
            e.mem.u32(at),
            e.mem.u32(at.wrapping_add(4)),
            e.mem.u32(at.wrapping_add(8)),
            e.mem.u32(at.wrapping_add(12)),
        ])
    }
    #[inline]
    fn write(self, e: &Engine, at: u32) {
        for (i, word) in (0u32..).zip(self.0) {
            e.mem.set_u32(at.wrapping_add(4 * i), word);
        }
    }
    #[inline]
    fn is_float(self) -> bool {
        self.0[0] & 0xff != 0
    }
    #[inline]
    fn int(self) -> i32 {
        self.0[2] as i32
    }
    #[inline]
    fn float(self) -> f64 {
        f64::from_bits(u64::from(self.0[2]) | u64::from(self.0[3]) << 32)
    }
    #[inline]
    fn set_type(&mut self, float: bool) {
        self.0[0] = self.0[0] & !0xff | u32::from(float);
    }
    #[inline]
    fn set_int(&mut self, value: i32) {
        self.0[2] = value as u32;
    }
    #[inline]
    fn set_float(&mut self, value: f64) {
        let bits = value.to_bits();
        self.0[2] = low_word(bits);
        self.0[3] = high_word(bits);
    }
    /// The value as the exe loads it (`FILD dword` of an integer, `FLD qword` of a double).
    #[inline]
    fn value(self) -> f64 {
        if self.is_float() {
            self.float()
        } else {
            f64::from(self.int())
        }
    }
    /// Whether the value is not zero (a NaN is not zero).
    #[inline]
    fn truth(self) -> bool {
        if self.is_float() {
            self.float() != 0.0
        } else {
            self.int() != 0
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
#[inline]
fn low_word(bits: u64) -> u32 {
    bits as u32
}

/// A word read as a signed integer.
#[inline]
fn signed(word: u32) -> i32 {
    word as i32
}

#[inline]
fn high_word(bits: u64) -> u32 {
    low_word(bits >> 32)
}

/// How the running thread's arithmetic steps are made, read once per `Eval` from its x87 control
/// word as the exe's helpers read it: 53 bits (the AI threads: the native `f64` step is the x87's),
/// 24 bits to nearest (main: the native step, rounded once more, unless [`needs_exact`]), or 24 bits
/// under another rounding mode (always the exact x87 step).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Precision {
    Double,
    Float,
    Exact,
}

#[inline]
fn precision() -> Precision {
    if !float_precision() {
        Precision::Double
    } else if engine_core::x87::control_word() & 0x0c00 == 0 {
        Precision::Float
    } else {
        Precision::Exact
    }
}

/// Whether a 24-bit step's `f64` result cannot be rounded again to give the x87's single rounding
/// of the exact result: at the bottom of the range ([`near_bottom`]), or exactly halfway between
/// two 24-bit values (the `f64` rounding may have made the tie). One compare each.
#[inline]
fn needs_exact(value: f64) -> bool {
    near_bottom(value) || value.to_bits() & 0x1fff_ffff == 0x1000_0000
}

/// One arithmetic step's result rounded to a float's 24 bits (to nearest, ties to even, the
/// exponent kept), as the exe's x87 arithmetic rounds on the main thread. Infinities, NaNs and
/// zeros as they are.
#[inline]
fn at_precision(value: f64) -> f64 {
    let bits = value.to_bits();
    let exponent = (bits >> 52) & 0x7ff;
    if exponent == 0 || exponent == 0x7ff {
        return value;
    }
    const DROPPED: u64 = (1 << 29) - 1;
    const HALF: u64 = 1 << 28;
    let rest = bits & DROPPED;
    let kept = bits >> 29;
    let up = rest > HALF || (rest == HALF && kept & 1 != 0);
    f64::from_bits((kept + u64::from(up)) << 29)
}

/// Whether a result lies at the bottom of the double range (below 2^-1000, zero included), where
/// the main thread's x87 arithmetic, with its wider exponent, rounds the exact value to 24 bits
/// before the double store: there the exact x87 step is taken. No script value a player meets is
/// that small; the cost is one compare.
#[inline]
fn near_bottom(value: f64) -> bool {
    value.abs() < f64::from_bits(0x0170_0000_0000_0000)
}

// ---- The stacks ---------------------------------------------------------------------------------

#[inline]
fn push_operator(e: &Engine, stack: u32, value: u32) {
    let index = e.mem.u32(stack + OPERATOR_INDEX).wrapping_add(1);
    e.mem.set_u32(stack + OPERATOR_INDEX, index);
    e.mem
        .set_u32(stack.wrapping_add(index.wrapping_mul(4)), value);
}

#[inline]
fn pop_operator(e: &Engine, stack: u32) -> u32 {
    let index = e.mem.u32(stack + OPERATOR_INDEX);
    let value = e.mem.u32(stack.wrapping_add(index.wrapping_mul(4)));
    e.mem.set_u32(stack + OPERATOR_INDEX, index.wrapping_sub(1));
    value
}

#[inline]
fn operators_empty(e: &Engine, stack: u32) -> bool {
    e.mem.u32(stack + OPERATOR_INDEX) == u32::MAX
}

#[inline]
fn operators_full(e: &Engine, stack: u32) -> bool {
    e.mem.u32(stack + OPERATOR_INDEX).wrapping_add(1) == 0x40
}

#[inline]
fn push_operand(e: &Engine, stack: u32, operand: Operand) {
    let index = e.mem.u32(stack + OPERAND_INDEX).wrapping_add(1);
    e.mem.set_u32(stack + OPERAND_INDEX, index);
    operand.write(e, stack.wrapping_add(index.wrapping_mul(0x10)));
}

#[inline]
fn pop_operand(e: &Engine, stack: u32) -> Operand {
    let index = e.mem.u32(stack + OPERAND_INDEX);
    let operand = Operand::read(e, stack.wrapping_add(index.wrapping_mul(0x10)));
    e.mem.set_u32(stack + OPERAND_INDEX, index.wrapping_sub(1));
    operand
}

#[inline]
fn operands_empty(e: &Engine, stack: u32) -> bool {
    e.mem.u32(stack + OPERAND_INDEX) == u32::MAX
}

// Typed from 00594b80
/// Pushes a word on the operator stack (no bounds check).
pub fn operator_push(e: &mut Engine, this: Ptr, value: u32) {
    push_operator(e, this.addr(), value);
}

// Typed from 00594bc0
/// Pops the operator stack's top word (no bounds check).
pub fn operator_pop(e: &mut Engine, this: Ptr) -> u32 {
    pop_operator(e, this.addr())
}

// Typed from 00594c00
/// Whether the operator stack is empty (its index -1).
pub fn operator_is_empty(e: &mut Engine, this: Ptr) -> bool {
    operators_empty(e, this.addr())
}

// Typed from 00594c20
/// Whether the operator stack is full (64 words).
pub fn operator_is_full(e: &mut Engine, this: Ptr) -> bool {
    operators_full(e, this.addr())
}

// Typed from 00594c40
/// Empties the operator stack.
pub fn operator_clear(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr() + OPERATOR_INDEX, u32::MAX);
}

// Typed from 00594c80
/// Pushes a 16-byte operand, given as its four words, on the operand stack (no bounds check).
pub fn operand_push(e: &mut Engine, this: Ptr, word0: u32, word1: u32, word2: u32, word3: u32) {
    push_operand(e, this.addr(), Operand([word0, word1, word2, word3]));
}

// Typed from 00594cd0
/// Pops the top operand into `out` (no bounds check) and answers `out`.
pub fn operand_pop(e: &mut Engine, this: Ptr, out: Ptr) -> Ptr {
    pop_operand(e, this.addr()).write(e, out.addr());
    out
}

// Typed from 00594d20
/// Whether the operand stack is empty (its index -1).
pub fn operand_is_empty(e: &mut Engine, this: Ptr) -> bool {
    operands_empty(e, this.addr())
}

// Typed from 00594d40
/// Empties the operand stack.
pub fn operand_clear(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr() + OPERAND_INDEX, u32::MAX);
}

// Typed from 00594b60
/// The operator stack's constructor: its index only (the 64 words are left as they are).
pub fn operator_stack_construct(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr() + OPERATOR_INDEX, u32::MAX);
    this
}

// Typed from 00594c60
/// The operand stack's constructor: its index only.
pub fn operand_stack_construct(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr() + OPERAND_INDEX, u32::MAX);
    this
}

// Typed from 00593490
/// `CExpression`'s constructor: the two stacks' indices, then the error word. The texts and the
/// stacks' contents are left as they are.
pub fn construct(e: &mut Engine, this: Ptr) -> Ptr {
    let operators = Ptr::new(this.addr() + OPERATORS);
    let operands = Ptr::new(this.addr() + OPERANDS);
    typed_call!(FAMILY, 0x0059_4b60, operator_stack_construct(e, operators));
    typed_call!(FAMILY, 0x0059_4c60, operand_stack_construct(e, operands));
    e.mem.set_u32(this.addr() + ERROR, 0);
    this
}

/// The error's store and report ([`set_error`]'s body).
#[inline]
fn fail(e: &mut Engine, this: u32, code: u32) {
    e.mem.set_u32(this + ERROR, code);
    let name = e.mem.u32(ERROR_NAMES.wrapping_add(code.wrapping_mul(4)));
    typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e); words PERCENT_S, name);
}

// Typed from 005934d0
/// Sets the expression's error code and reports its name (`"%s"`, through the empty report
/// function).
pub fn set_error(e: &mut Engine, this: Ptr, code: u32) {
    fail(e, this.addr(), code);
}

/// Copies `size` bytes with the exe's copy wrapper.
#[inline]
fn copy(e: &mut Engine, to: u32, from: u32, size: u32) {
    typed_call!(
        crt::FAMILY,
        0x0040_1460,
        crt::mem::memcpy_wrapper(e, Ptr::new(to), Ptr::new(from), size)
    );
}

/// The tokenizer, by address.
#[inline]
fn token_at(
    e: &mut Engine,
    this: u32,
    cursor: u32,
    out: u32,
    kind: u32,
    parse: u8,
    spaces: u32,
) -> u32 {
    e.call(
        NEXT_TOKEN,
        &args![this, cursor, out, kind, u32::from(parse != 0), spaces],
    )
    .u32()
}

// Typed from 00593500
/// Converts an infix expression of `*length` bytes into the expression's postfix buffer, the
/// shunting-yard way: the text is wrapped in parentheses in the infix buffer, read token by token,
/// operands are copied after a space, operators wait on the operator stack by precedence (a `-`
/// that follows no operand becomes unary minus `~`) and are written after a space with their
/// table text. Answers the postfix buffer with `*length` set to its length, or null after error 2
/// (the operator stack full) or 5 (unbalanced). A closing parenthesis pops until it pops number 0
/// without testing for an empty stack, as the exe does. The token buffer is 0x44 bytes, or with
/// xNVSE's fix its 0x500-byte buffer.
pub fn to_postfix(e: &mut Engine, this: Ptr, infix: Ptr, length: Ptr) -> Ptr {
    let base = this.addr();
    e.mem.set_u32(base + ERROR, 0);
    let operators = base + OPERATORS;
    e.mem.set_u32(operators + OPERATOR_INDEX, u32::MAX);
    e.mem.set_u32(base + OPERANDS + OPERAND_INDEX, u32::MAX);
    let mut write = base + INFIX;
    e.mem.set_u8(write, b'(');
    write += 1;
    let size = e.mem.u32(length.addr());
    copy(e, write, infix.addr(), size);
    write = write.wrapping_add(e.mem.u32(length.addr()));
    e.mem.set_u8(write, b')');
    e.mem.set_u8(write.wrapping_add(1), 0);
    let alt = e.expression_alt_buffer();
    e.with_stack(0x44 + 12, |e, work| {
        let token = if alt != 0 { alt } else { work.addr() };
        let cursor = work.addr() + 0x44;
        let kind = work.addr() + 0x48;
        e.mem.set_u32(cursor, base + INFIX);
        let mut out = base + POSTFIX;
        e.mem.set_u8(out, 0);
        let mut follows_operand = false;
        let append_operator = |e: &mut Engine, out: &mut u32, operator: u32| {
            e.mem.set_u8(*out, b' ');
            *out = out.wrapping_add(1);
            let text = OPERATOR_TEXT.wrapping_add(operator.wrapping_mul(8));
            let size = e.call(STRLEN, &args![text]).u32();
            copy(e, *out, text, size);
            *out = out.wrapping_add(size);
        };
        loop {
            e.mem.set_u32(kind, OPERAND);
            let count = token_at(e, base, cursor, token, kind, 0, 0);
            if count != 0 {
                let current = e.mem.i32(kind);
                if current < OPERAND as i32 {
                    match current {
                        0 => {
                            if operators_full(e, operators) {
                                fail(e, base, 2);
                                return Ptr::NULL;
                            }
                            push_operator(e, operators, 0);
                        }
                        1 => {
                            if operators_empty(e, operators) {
                                fail(e, base, 5);
                                return Ptr::NULL;
                            }
                            loop {
                                let operator = pop_operator(e, operators);
                                if operator == 0 {
                                    break;
                                }
                                append_operator(e, &mut out, operator);
                            }
                        }
                        _ => {
                            if current == 10 && !follows_operand {
                                e.mem.set_u32(kind, NEGATE);
                            }
                            let current = e.mem.u32(kind);
                            while !operators_empty(e, operators) {
                                let operator = pop_operator(e, operators);
                                let top = e
                                    .mem
                                    .i8(OPERATOR_PRECEDENCE.wrapping_add(operator.wrapping_mul(8)));
                                let next = e
                                    .mem
                                    .i8(OPERATOR_PRECEDENCE.wrapping_add(current.wrapping_mul(8)));
                                if top < next {
                                    push_operator(e, operators, operator);
                                    break;
                                }
                                append_operator(e, &mut out, operator);
                            }
                            if operators_full(e, operators) {
                                fail(e, base, 2);
                                return Ptr::NULL;
                            }
                            push_operator(e, operators, current);
                        }
                    }
                } else {
                    e.mem.set_u8(out, b' ');
                    out = out.wrapping_add(1);
                    copy(e, out, token, count);
                    out = out.wrapping_add(count);
                }
            }
            let current = e.mem.i32(kind);
            if current == OPERAND as i32 {
                follows_operand = true;
            } else if current < OPERAND as i32 && current > 1 {
                follows_operand = false;
            }
            if count == 0 {
                break;
            }
        }
        e.mem.set_u8(out, 0);
        if !operators_empty(e, operators) {
            fail(e, base, 5);
            return Ptr::NULL;
        }
        e.mem
            .set_u32(length.addr(), out.wrapping_sub(base + POSTFIX));
        Ptr::new(base + POSTFIX)
    })
}

// Typed from 00593870
/// Copies the next token at `*cursor` into `out` (NUL-terminated), moves the cursor past it and
/// answers its length; `*kind` is the operator's number, or 16 for an operand. Leading bytes below
/// `!` (signed, so also bytes 0x80 and above) are skipped, counted in `*spaces` when `count_spaces`
/// is set. Compiled operands are copied whole: `X` and a command opcode (0x1000 up to the command
/// table's end) with its parameter bytes (the 16-bit length + 5), `s l f G Z` and an index (3
/// bytes), `n` (5), `z` (9), a quoted string; `r` and an index start an operand that goes on. Other
/// bytes collect into an operand until a space, a tab or an operator; an operator alone at the
/// start is the token. A byte that starts an operator's text without completing any sets error 5
/// and is kept as operand text.
pub fn next_token(
    e: &mut Engine,
    this: Ptr,
    cursor: Ptr,
    out: Ptr,
    kind: Ptr,
    count_spaces: bool,
    spaces: Ptr,
) -> u32 {
    let mut source = e.mem.u32(cursor.addr());
    let mut write = out.addr();
    let mut first = true;
    e.mem.set_u32(kind.addr(), OPERAND);
    loop {
        let byte = e.mem.i8(source);
        if byte == 0 || byte >= 0x21 {
            break;
        }
        source = source.wrapping_add(1);
        if count_spaces && !spaces.is_null() {
            let count = e.mem.u32(spaces.addr());
            e.mem.set_u32(spaces.addr(), count.wrapping_add(1));
        }
    }
    let take = |e: &Engine, write: &mut u32, source: &mut u32, count: u32| {
        for _ in 0..count {
            e.mem.set_u8(*write, e.mem.u8(*source));
            *write = write.wrapping_add(1);
            *source = source.wrapping_add(1);
        }
    };
    loop {
        let byte = e.mem.u8(source);
        if byte == 0 {
            break;
        }
        if byte == b'X' {
            let opcode = i32::from(e.mem.i16(source.wrapping_add(1)));
            // The highest opcode is the immediate `0059390A`, moved with the table by a built-in
            // script extender (ADR-0016).
            if (0x1000..e.script_tables().max_opcode as i32).contains(&opcode) {
                let size = i32::from(e.mem.i16(source.wrapping_add(3)).wrapping_add(5)) as u32;
                copy(e, write, source, size);
                write = write.wrapping_add(size);
                source = source.wrapping_add(size);
                break;
            }
        }
        match byte {
            b's' | b'l' | b'f' | b'G' | b'Z' | b'r' => {
                take(e, &mut write, &mut source, 3);
                if byte != b'r' {
                    break;
                }
                first = false;
                continue;
            }
            b'n' => {
                take(e, &mut write, &mut source, 5);
                break;
            }
            b'z' => {
                take(e, &mut write, &mut source, 9);
                break;
            }
            b'"' => {
                take(e, &mut write, &mut source, 1);
                loop {
                    let next = e.mem.u8(source);
                    if next == 0 || next == b'"' {
                        break;
                    }
                    take(e, &mut write, &mut source, 1);
                }
                if e.mem.u8(source) == b'"' {
                    take(e, &mut write, &mut source, 1);
                }
                break;
            }
            b' ' | b'\t' => break,
            _ => {}
        }
        let mut matched = false;
        let mut partial = false;
        let mut number = 0u32;
        while number < 0x10 {
            matched = true;
            let mut text = OPERATOR_TEXT + number * 8;
            let mut at = source;
            loop {
                let expected = e.mem.u8(text);
                if expected == 0 {
                    break;
                }
                if expected != e.mem.u8(at) {
                    matched = false;
                    break;
                }
                partial = true;
                at = at.wrapping_add(1);
                text += 1;
            }
            if matched {
                break;
            }
            number += 1;
        }
        e.mem.set_u32(kind.addr(), number);
        if matched {
            if first {
                let mut text = OPERATOR_TEXT.wrapping_add(number.wrapping_mul(8));
                loop {
                    let character = e.mem.u8(text);
                    if character == 0 {
                        break;
                    }
                    e.mem.set_u8(write, character);
                    write = write.wrapping_add(1);
                    text = text.wrapping_add(1);
                    source = source.wrapping_add(1);
                }
            } else {
                e.mem.set_u32(kind.addr(), OPERAND);
            }
            break;
        }
        if partial {
            fail(e, this.addr(), 5);
        }
        e.mem.set_u8(write, byte);
        write = write.wrapping_add(1);
        first = false;
        source = source.wrapping_add(1);
    }
    e.mem.set_u8(write, 0);
    e.mem.set_u32(cursor.addr(), source);
    write.wrapping_sub(out.addr())
}

/// Integer division as `CDQ` and `IDIV`: the exe raises a divide error (the game stops) for a zero
/// divisor and for `i32::MIN / -1`.
fn idiv(dividend: i32, divisor: i32) -> (i32, i32) {
    match (dividend.checked_div(divisor), dividend.checked_rem(divisor)) {
        (Some(quotient), Some(remainder)) => (quotient, remainder),
        _ => panic!(
            "integer divide error (#DE) in CExpression::Eval 00593db0: {dividend} / {divisor}"
        ),
    }
}

/// A double truncated to an integer by the CRT's `_ftol2_sse` (its low word).
#[inline]
fn to_int(e: &mut Engine, value: f64) -> i32 {
    let long = typed_call!(crt::FAMILY, 0x00ec_62c0, crt::math::ftol2_sse(e, value));
    signed(low_word(long))
}

/// What a binary operator gives: the result's type byte (a double or not) and its value, or
/// `None` after error 4.
enum Value {
    Int(i32),
    Float(f64),
}

/// One binary operator of [`eval`]. Integer results replace only the operand's low value word, as
/// the exe stores them.
#[allow(clippy::too_many_arguments)]
fn binary(
    e: &mut Engine,
    this: u32,
    operator: u32,
    left: Operand,
    mut right: Operand,
    parse: u8,
    token: u32,
    precision: Precision,
) -> Option<Value> {
    let both_int = !left.is_float() && !right.is_float();
    let truth = |v: bool| Some(Value::Int(i32::from(v)));
    match operator {
        2 => truth(left.truth() && right.truth()),
        3 => truth(left.truth() || right.truth()),
        4..=9 => {
            let value = if both_int {
                let (l, r) = (left.int(), right.int());
                match operator {
                    4 => l <= r,
                    5 => l < r,
                    6 => l >= r,
                    7 => l > r,
                    8 => l == r,
                    _ => l != r,
                }
            } else {
                let (l, r) = (left.value(), right.value());
                match operator {
                    4 => l <= r,
                    5 => l < r,
                    6 => l >= r,
                    7 => l > r,
                    8 => l == r,
                    _ => l != r,
                }
            };
            truth(value)
        }
        10..=12 => {
            if both_int {
                let (l, r) = (left.int(), right.int());
                return Some(Value::Int(match operator {
                    10 => l.wrapping_sub(r),
                    11 => l.wrapping_add(r),
                    _ => l.wrapping_mul(r),
                }));
            }
            let (l, r) = (left.value(), right.value());
            let v = match operator {
                10 => l - r,
                11 => l + r,
                _ => l * r,
            };
            Some(Value::Float(match precision {
                Precision::Double => v,
                Precision::Float if !needs_exact(v) => at_precision(v),
                _ => match operator {
                    10 => x87::x87_sub(l, r),
                    11 => x87::x87_add(l, r),
                    _ => x87::x87_mul(l, r),
                },
            }))
        }
        13 | 14 => {
            if !right.truth() {
                if parse == 0 {
                    fail(e, this, 4);
                    return None;
                }
                // `FLD1` and `FSTP qword` over the right operand's value.
                right.set_float(1.0);
            }
            let divide = operator == 13;
            if both_int {
                let (quotient, remainder) = idiv(left.int(), right.int());
                return Some(Value::Int(if divide { quotient } else { remainder }));
            }
            let value = match (left.is_float(), right.is_float(), divide) {
                (_, _, true) => {
                    let (l, r) = (left.value(), right.value());
                    let v = l / r;
                    match precision {
                        Precision::Double => v,
                        Precision::Float if !needs_exact(v) => at_precision(v),
                        _ => x87::x87_div(l, r),
                    }
                }
                (false, true, false) => {
                    let divisor = to_int(e, right.float());
                    f64::from(idiv(left.int(), divisor).1)
                }
                (true, false, false) => {
                    let dividend = to_int(e, left.float());
                    f64::from(idiv(dividend, right.int()).1)
                }
                _ => {
                    let dividend = to_int(e, left.float());
                    let divisor = to_int(e, right.float());
                    f64::from(idiv(dividend, divisor).1)
                }
            };
            Some(Value::Float(value))
        }
        _ => {
            e.call(SCRIPT_LOG, &args![UNHANDLED_OPERATOR, token]);
            Some(Value::Int(0))
        }
    }
}

/// JIP LN NVSE's `bDoublePrecision` (`SetOptionalPatch(8)`): its operator hook replaces the
/// operator switch at `0x593FBC`. The switch's bit is tested first; then JIP's handler gets the
/// operator's index (less 2), the left operand's type word and value words (low, high), the
/// right's, and the current operand's address, and writes the result there. True when the handler
/// took the operator.
#[inline]
fn jip_double_precision(
    e: &mut Engine,
    operator: u32,
    left: Operand,
    right: Operand,
    current: u32,
) -> bool {
    e.jip_patches() & engine_core::extend::jip_patches::DOUBLE_PRECISION != 0
        && e.site(
            0x0059_3fbc,
            &[
                operator.wrapping_sub(2),
                left.0[0],
                left.0[2],
                left.0[3],
                right.0[0],
                right.0[2],
                right.0[3],
                current,
            ],
        )
        .is_some()
}

// Typed from 00593db0
/// `CExpression::Eval`: evaluates `length` bytes of a compiled (postfix) expression and answers
/// its value, 0 after an error (the code is in the expression). Nothing runs when `length` is 0 or
/// the expression already has an error; over 0x200 bytes is error 1. The bytes are copied after a
/// 0x48-byte token buffer, in one stack block as in the exe (with xNVSE's fix the token buffer is
/// 0x4B4 bytes further down, so no token reaches the copy). Operands: decimal integers (`atol`),
/// decimal floats (`atof`), else a compiled operand read by `005ac7a0` (in `parse` mode at the
/// token's place in the original bytes, and without failing). Operators pop their operands (error 3
/// when missing): logic, comparisons and arithmetic on integers when both are integers, else on
/// doubles; a zero divisor is error 4, or in `parse` mode the double 1.0. One operand must remain
/// (error 5 otherwise).
///
/// Seam `005949d4`, the compiled operand's read outside parse mode (xNVSE's command call), with
/// the token's address in the expression's own bytes: an answer is the read's result.
#[allow(clippy::too_many_arguments)]
pub fn eval(
    e: &mut Engine,
    this: Ptr,
    data: Ptr,
    reference: u32,
    container: u32,
    script: u32,
    locals: u32,
    length: i32,
    parse: u8,
) -> f64 {
    let base = this.addr();
    if length == 0 || e.mem.u32(base + ERROR) != 0 {
        return 0.0;
    }
    let operands = base + OPERANDS;
    e.mem.set_u32(operands + OPERAND_INDEX, u32::MAX);
    if length > 0x200 {
        fail(e, base, 1);
        return 0.0;
    }
    let precision = precision();
    // The frame as the exe's: the token buffer (its byte 0x47 the "skip push" flag) directly
    // before the copy of the bytes; then the current operand, the copy cursor, the source position,
    // the kind and the operand offset.
    let moved = if e.expression_alt_buffer() != 0 {
        0x4b4
    } else {
        0
    };
    e.with_stack(moved + 0x48 + 0x208 + 16 + 16, |e, frame| {
        let token = frame.addr();
        let skip_push = token + moved + 0x47;
        let copied = token + moved + 0x48;
        let current = copied + 0x208;
        let cursor = current + 16;
        let position = cursor + 4;
        let kind = cursor + 8;
        let offset = cursor + 12;
        e.mem.set_u32(position, data.addr());
        e.mem.set_u32(kind, OPERAND);
        e.mem.set_u8(skip_push, 0);
        e.mem.set_u8(current, 0);
        e.mem.set_u64(current + 8, 0);
        copy(e, copied, data.addr(), length as u32);
        e.mem.set_u8(copied.wrapping_add(length as u32), 0);
        e.mem.set_u32(cursor, copied);
        let next = |e: &mut Engine| token_at(e, base, cursor, token, kind, parse, position);
        let mut size = next(e);
        while size != 0 {
            let operator = e.mem.i32(kind);
            if operator < OPERAND as i32 && operator > 1 {
                if operands_empty(e, operands) {
                    fail(e, base, 3);
                    return 0.0;
                }
                let right = pop_operand(e, operands);
                let mut result = Operand::read(e, current);
                if operator == NEGATE as i32 {
                    if right.is_float() {
                        result.set_float(-right.float());
                    } else {
                        result.set_int(right.int().wrapping_neg());
                    }
                    result.0[0] = result.0[0] & !0xff | (right.0[0] & 0xff);
                } else {
                    if operands_empty(e, operands) {
                        fail(e, base, 3);
                        return 0.0;
                    }
                    let left = pop_operand(e, operands);
                    if jip_double_precision(e, operator as u32, left, right, current) {
                        // JIP's operator wrote the current operand's type byte and value.
                        result = Operand::read(e, current);
                    } else {
                        match binary(
                            e,
                            base,
                            operator as u32,
                            left,
                            right,
                            parse,
                            token,
                            precision,
                        ) {
                            Some(Value::Int(v)) => {
                                result.set_type(false);
                                result.set_int(v);
                            }
                            Some(Value::Float(v)) => {
                                result.set_type(true);
                                result.set_float(v);
                            }
                            None => return 0.0,
                        }
                    }
                }
                result.write(e, current);
                push_operand(e, operands, result);
            } else {
                if typed_call!(
                    FAMILY,
                    0x004b_1230,
                    numbers::is_an_integer(e, Ptr::new(token))
                ) {
                    e.mem.set_u8(current, 0);
                    let value = e.call(ATOL, &args![token]).u32();
                    e.mem.set_u32(current + 8, value);
                } else if typed_call!(FAMILY, 0x004b_12d0, numbers::is_a_float(e, Ptr::new(token)))
                {
                    e.mem.set_u8(current, 1);
                    let value = e.call(ATOF, &args![token]).f64();
                    e.mem.set_f64(current + 8, value);
                } else {
                    e.mem.set_u32(offset, 0);
                    e.mem.set_u8(current, 1);
                    if parse != 0 {
                        let at = e.mem.u32(position);
                        e.call(
                            READ_OPERAND,
                            &args![
                                current + 8,
                                at,
                                offset,
                                reference,
                                container,
                                script,
                                locals,
                                u32::from(parse)
                            ],
                        );
                    } else if !{
                        // Seam 005949d4 (xNVSE's command call; words: the result's address, the
                        // offset's address, the reference, the container, the script, the locals,
                        // the token's address in the expression's own bytes): an answer is the
                        // read's result, made on the script's data instead of the token's copy.
                        let at = data.addr().wrapping_add(
                            e.mem.u32(cursor).wrapping_sub(copied).wrapping_sub(size),
                        );
                        let words = [
                            current + 8,
                            offset,
                            reference,
                            container,
                            script,
                            locals,
                            at,
                        ];
                        match e.site(0x0059_49d4, &words) {
                            Some(answer) => answer != 0,
                            None => e
                                .call(
                                    READ_OPERAND,
                                    &args![
                                        current + 8,
                                        token,
                                        offset,
                                        reference,
                                        container,
                                        script,
                                        locals,
                                        0u32
                                    ],
                                )
                                .bool(),
                        }
                    } {
                        fail(e, base, 5);
                        return 0.0;
                    }
                }
                if e.mem.u8(skip_push) == 0 {
                    push_operand(e, operands, Operand::read(e, current));
                } else {
                    e.mem.set_u8(skip_push, 0);
                }
            }
            let moved = e.mem.u32(position).wrapping_add(size);
            e.mem.set_u32(position, moved);
            size = next(e);
        }
        if operands_empty(e, operands) {
            fail(e, base, 5);
            return 0.0;
        }
        let last = pop_operand(e, operands);
        last.write(e, current);
        if !operands_empty(e, operands) {
            fail(e, base, 5);
            return 0.0;
        }
        last.value()
    })
}
