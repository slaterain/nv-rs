//! xNVSE's run-time expression compiler (ADR-0016 §I-14; plan step 3.8, part 1): the
//! `ExpressionParser` that compiles the old script syntax's expressions into the bytecode the
//! evaluator ([`crate::expr`]) runs and the decompiler ([`crate::decompile`]) reads back, and the
//! parse routines of the commands that use it (`Cmd_Expression_Parse`, `Cmd_Let_Parse`,
//! `Cmd_BeginLoop_Parse`, `Cmd_Loop_Parse`, `Cmd_Null_Parse`, `Cmd_Function_Parse`,
//! `Cmd_Call_Parse`), which the game's own script compiler (translated in
//! `engine-fallout-script`) calls through a command record's parse word.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a): `ScriptUtils.cpp` (the operator table and
//! its rules, `ExpressionParser`, `Cmd_Expression_Parse`, the one-line lambda macro),
//! `ScriptTokens.cpp` (`ScriptToken::Write`, `CanConvertOperand`, `Operator::GetResult`),
//! `GameScript.cpp` (`ScriptBuffer::ResolveRef`, `GetRefIdx`, `GetVariableType`,
//! `ScriptLineBuffer::Write*`), `Commands_Scripting.cpp` (the parse routines) and
//! `Hooks_Script.cpp` (`ParseNestedFunction`, the loop offsets).
//!
//! The parser works on the game's own buffers in guest memory, as xNVSE's does: the line's text
//! (`ScriptLineBuffer::paramText`, which it rewrites while it works, as xNVSE does) and its
//! output (`dataBuf`). Every length read from the line is held to the line's 0x200-byte buffers:
//! where xNVSE would read or write past them (a corrupt length, an unterminated text), the port
//! stops at the buffer's end instead. Errors are printed to the console as xNVSE prints them at
//! run time (`ShowError`, `ShowCompilerError`); their text is printed as it is, where xNVSE hands
//! it to `Console_Print` as a format (a `%` in a script's name or text is printed, not expanded).
//! One deliberate difference: an open loop's jump word is kept by its offset in the script's
//! data, not its address, so a loop over the data's growth gets the bytes xNVSE meant to write
//! ([`begin_loop_parse`]).
//!
//! Not here: the new script language (`Compiler/`, scripts starting with `name`; deferred), and
//! `ExpressionParser::ParseParentheses` (the GECK's inline expressions; editor only).

use crate::expr::{can_convert, op, op_symbol, operand_count, Ty};
use crate::table::{guest_text, opcode_by_name, record_of, return_type, Returns};
use crate::Nvse;
use engine_core::prelude::*;
use std::collections::HashMap;

// ---- the game's structures ----

/// `ScriptLineBuffer` (0x41C bytes).
pub(crate) mod ln {
    pub const NUMBER: u32 = 0;
    pub const TEXT: u32 = 4;
    pub const TEXT_LEN: u32 = 0x204;
    pub const OFFSET: u32 = 0x208;
    pub const DATA: u32 = 0x20c;
    pub const DATA_LEN: u32 = 0x40c;
    pub const OPCODE: u32 = 0x410;
    pub const CALLING_REF: u32 = 0x414;
    /// The size of the text and of the output.
    pub const SIZE: u32 = 0x200;
    /// The whole structure.
    pub const STRUCT: u32 = 0x41c;
}

/// `ScriptBuffer` (0x58 bytes).
pub(crate) mod sb {
    pub const TEXT: u32 = 0;
    pub const NAME: u32 = 0xc;
    pub const ERROR: u32 = 0x14;
    pub const PARTIAL: u32 = 0x18;
    pub const CUR_LINE: u32 = 0x1c;
    pub const DATA: u32 = 0x20;
    pub const DATA_OFFSET: u32 = 0x24;
    /// `info` (`Script::ScriptInfo`): unused variable count, references, data length, variables,
    /// type.
    pub const UNUSED_VARS: u32 = 0x28;
    pub const NUM_REFS: u32 = 0x2c;
    pub const VAR_COUNT: u32 = 0x34;
    pub const TYPE: u32 = 0x38;
    pub const VARS: u32 = 0x3c;
    pub const REFS: u32 = 0x44;
    pub const CURRENT_SCRIPT: u32 = 0x4c;
    pub const SIZE: u32 = 0x58;
}

/// `Script` (0x54 bytes at run time).
pub(crate) mod sc {
    pub const UNUSED_VARS: u32 = 0x18;
    pub const NUM_REFS: u32 = 0x1c;
    pub const DATA_LEN: u32 = 0x20;
    pub const VAR_COUNT: u32 = 0x24;
    pub const COMPILED: u32 = 0x2a;
    pub const TEXT: u32 = 0x2c;
    pub const DATA: u32 = 0x30;
    pub const REFS: u32 = 0x44;
    pub const VARS: u32 = 0x4c;
    pub const SIZE: u32 = 0x54;
}

/// `Script::eType_Quest`.
const QUEST_SCRIPT: u16 = 1;
/// `Script::VariableType`.
pub(crate) mod vt {
    pub const FLOAT: u8 = 0;
    pub const INTEGER: u8 = 1;
    pub const STRING: u8 = 2;
    pub const ARRAY: u8 = 3;
    pub const REF: u8 = 4;
    pub const INVALID: u8 = 5;
}

pub(crate) const FORM_HEAP_ALLOCATE: u32 = 0x0040_1000;
pub(crate) const FORM_HEAP_FREE: u32 = 0x0040_1030;
pub(crate) const LOOKUP_FORM: u32 = 0x0048_39c0;
/// `GetFormByID(const char *editorID)` (`00483a00`, `cdecl`).
pub(crate) const FORM_BY_EDITOR_ID: u32 = 0x0048_3a00;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_TES_OBJECT_REFR: u32 = 0x0118_41cc;
const RTTI_TES_SCRIPTABLE_FORM: u32 = 0x0118_3254;
/// `TESForm::GetIsReference`, the `+0xF0` virtual function.
const GET_IS_REFERENCE: u32 = 0xf0;
/// Form types (`TESForm +4`): `TESGlobal`, `TESObjectREFR`, `TESQuest`.
const FORM_GLOBAL: u8 = 6;
const FORM_REFR: u8 = 0x3a;
const FORM_QUEST: u8 = 0x47;
/// `kFormFlags_QuestItem`, the persistent bit of a reference.
const PERSISTENT: u32 = 0x400;
/// `ConsoleManager::GetSingleton(bool)` (`0071b160`): its `scriptContext` (`+0`) is the compiler
/// `Script::Compile` runs on.
const CONSOLE_MANAGER: u32 = 0x0071_b160;
/// `ScriptCompiler::Compile` (`005aeb90`), `ScriptBuffer`'s constructor and destructor,
/// `Script`'s constructor and destructor.
const SCRIPT_COMPILE: u32 = 0x005a_eb90;
const BUFFER_CONSTRUCTOR: u32 = 0x005a_e490;
const BUFFER_DESTRUCTOR: u32 = 0x005a_e5c0;
const SCRIPT_CONSTRUCTOR: u32 = 0x005a_a0f0;
const SCRIPT_DESTRUCTOR: u32 = 0x005a_a1a0;
/// The output offset of the open block's jump word (`011cadbc`), which a lambda's compile saves
/// and clears.
const BLOCK_JUMP: u32 = 0x011c_adbc;

// ---- token types (`Token_Type`) ----

/// `Token_Type` as a code: [`Ty`]'s values, then `kTokenType_Invalid` and `kTokenType_Empty`.
pub(crate) type Code = u8;
pub(crate) mod tt {
    use super::Code;
    pub const NUMBER: Code = 0;
    pub const BOOLEAN: Code = 1;
    pub const STRING: Code = 2;
    pub const FORM: Code = 3;
    pub const REF: Code = 4;
    pub const GLOBAL: Code = 5;
    pub const ARRAY: Code = 6;
    pub const ARRAY_ELEMENT: Code = 7;
    pub const SLICE: Code = 8;
    pub const COMMAND: Code = 9;
    pub const NUMERIC_VAR: Code = 11;
    pub const REF_VAR: Code = 12;
    pub const STRING_VAR: Code = 13;
    pub const ARRAY_VAR: Code = 14;
    pub const AMBIGUOUS: Code = 15;
    pub const FOR_EACH: Code = 17;
    pub const PAIR: Code = 21;
    pub const ASSIGNABLE_STRING: Code = 22;
    pub const LAMBDA: Code = 23;
    pub const LEFT: Code = 25;
    pub const RIGHT: Code = 26;
    pub const OPTIONAL_EMPTY: Code = 27;
    pub const INVALID: Code = 28;
    pub const EMPTY: Code = 29;
}

const TYPES: [Ty; 28] = [
    Ty::Number,
    Ty::Boolean,
    Ty::String,
    Ty::Form,
    Ty::Ref,
    Ty::Global,
    Ty::Array,
    Ty::ArrayElement,
    Ty::Slice,
    Ty::Command,
    Ty::Variable,
    Ty::NumericVar,
    Ty::RefVar,
    Ty::StringVar,
    Ty::ArrayVar,
    Ty::Ambiguous,
    Ty::Operator,
    Ty::ForEachContext,
    Ty::Byte,
    Ty::Short,
    Ty::Int,
    Ty::Pair,
    Ty::AssignableString,
    Ty::Lambda,
    Ty::LambdaScriptData,
    Ty::LeftToken,
    Ty::RightToken,
    Ty::OptionalEmpty,
];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptTokens.cpp: CanConvertOperand.
/// `CanConvertOperand` (the run-time build's conversion lists).
pub(crate) fn convert(from: Code, to: Code) -> bool {
    if from == to {
        return true;
    }
    if from >= tt::INVALID || to >= tt::INVALID {
        return false;
    }
    can_convert(TYPES[usize::from(from)], TYPES[usize::from(to)])
}

/// `TokenTypeToString`.
pub(crate) fn type_name(t: Code) -> &'static str {
    const NAMES: [&str; 28] = [
        "Number",
        "Boolean",
        "String",
        "Form",
        "Ref",
        "Global",
        "Array",
        "Array Element",
        "Slice",
        "Command",
        "Variable",
        "Numeric Variable",
        "Reference Variable",
        "String Variable",
        "Array Variable",
        "Ambiguous",
        "Operator",
        "ForEach Context",
        "Byte",
        "Short",
        "Int",
        "Pair",
        "Assignable String",
        "Lambda",
        "LambdaScriptData",
        "LeftToken",
        "RightToken",
        "OptionalEmpty",
    ];
    NAMES.get(usize::from(t)).copied().unwrap_or("Invalid")
}

// ---- operators (`s_operators`, `kOpRule_*`) ----

/// An `OperationRule` (its handler is the evaluator's): left, right, result, asymmetric.
#[derive(Clone, Copy)]
struct Rule(Code, Code, Code, bool);

use tt::{
    AMBIGUOUS as AM, ARRAY as AR, ARRAY_ELEMENT as AE, ARRAY_VAR as AV, ASSIGNABLE_STRING as AS,
    BOOLEAN as BO, COMMAND as CM, FORM as FO, FOR_EACH as FE, GLOBAL as GL, INVALID as IV,
    NUMBER as NU, NUMERIC_VAR as NV, PAIR as PA, REF_VAR as RV, RIGHT as RT, SLICE as SL,
    STRING as ST, STRING_VAR as SV,
};

const fn r(l: Code, rh: Code, res: Code) -> Rule {
    Rule(l, rh, res, false)
}
const fn ra(l: Code, rh: Code, res: Code) -> Rule {
    Rule(l, rh, res, true)
}

#[rustfmt::skip]
const COMPARISON: &[Rule] = &[r(AM, AM, BO), r(AM, NU, BO), r(AM, ST, BO), r(NU, NU, BO), r(ST, ST, BO)];
#[rustfmt::skip]
const EQUALITY: &[Rule] = &[
    r(AM, AM, BO), r(AM, NU, BO), r(AM, FO, BO), r(AM, ST, BO),
    r(NU, NU, BO), r(ST, ST, BO), r(FO, FO, BO), ra(FO, NU, BO), r(AR, AR, BO),
];
const LOGICAL: &[Rule] = &[r(AM, AM, BO), r(AM, BO, BO), r(BO, BO, RT)];
#[rustfmt::skip]
const ADDITION: &[Rule] = &[r(AM, AM, AM), r(AM, NU, NU), r(AM, ST, ST), r(NU, NU, NU), r(ST, ST, ST)];
const ARITHMETIC: &[Rule] = &[r(AM, AM, NU), r(NU, AM, NU), r(NU, NU, NU)];
#[rustfmt::skip]
const MULTIPLY: &[Rule] = &[r(AM, AM, AM), r(ST, AM, ST), r(NU, AM, AM), r(NU, NU, NU), r(ST, NU, ST)];
const INTEGER: &[Rule] = &[r(AM, AM, NU), r(NU, AM, NU), r(NU, NU, NU)];
#[rustfmt::skip]
const ASSIGNMENT: &[Rule] = &[
    ra(AM, AM, AM), ra(AM, ST, ST), ra(AM, NU, NU), ra(AM, AR, AR), ra(AM, FO, FO),
    ra(NV, AM, NU), ra(RV, AM, FO), ra(SV, AM, ST), ra(AV, AM, AR), ra(AE, AM, AM),
    ra(AS, ST, ST), ra(NV, NU, NU), ra(SV, ST, ST), ra(RV, FO, FO), ra(RV, NU, FO),
    ra(GL, NU, NU), ra(AV, AR, AR), ra(AE, NU, NU), ra(AE, ST, ST), ra(AE, FO, FO),
    ra(AE, AR, AR),
];
#[rustfmt::skip]
const PLUS_EQUALS: &[Rule] = &[
    ra(NV, AM, NU), ra(SV, AM, ST), ra(AE, AM, AM), ra(GL, AM, NU), r(AM, AM, AM),
    ra(AM, NU, NU), ra(AM, ST, ST),
    ra(NV, NU, NU), ra(AE, NU, NU), ra(SV, ST, ST), ra(AE, ST, ST), ra(GL, NU, NU),
];
/// `-=`, `*=`, `/=`, `^=` and `|=`, `&=`, `%=` share these rules.
#[rustfmt::skip]
const NUMBER_EQUALS: &[Rule] = &[
    ra(NV, AM, NU), ra(AE, AM, NU), ra(GL, AM, NU), r(AM, AM, NU), ra(AM, NU, NU),
    ra(NV, NU, NU), ra(AE, NU, NU), ra(GL, NU, NU),
];
const NEGATION: &[Rule] = &[ra(AM, IV, NU), ra(NU, IV, NU)];
const LOGICAL_NOT: &[Rule] = &[ra(AM, IV, BO), ra(BO, IV, BO)];
#[rustfmt::skip]
const LEFT_BRACKET: &[Rule] = &[
    ra(AR, AM, AE), ra(ST, AM, ST), ra(AM, ST, AE), ra(AM, NU, AM), ra(AM, AM, AM), ra(AM, SL, AM),
    ra(AR, NU, AE), ra(AR, ST, AE), ra(AE, NU, AS), ra(SV, NU, AS), ra(AE, SL, AS), ra(SV, SL, AS),
    ra(ST, NU, ST), ra(AR, SL, AR), ra(ST, SL, ST),
];
const MEMBER_ACCESS: &[Rule] = &[
    ra(AR, AM, AE),
    ra(AM, ST, AE),
    ra(AM, AM, AE),
    ra(AR, ST, AE),
];
#[rustfmt::skip]
const SLICE: &[Rule] = &[r(AM, AM, SL), r(AM, NU, SL), r(AM, ST, SL), r(ST, ST, SL), r(NU, NU, SL)];
const IN: &[Rule] = &[
    ra(AV, AM, FE),
    ra(AV, AR, FE),
    ra(SV, ST, FE),
    ra(RV, FO, FE),
];
#[rustfmt::skip]
const TO_STRING: &[Rule] = &[ra(AM, IV, ST), ra(ST, IV, ST), ra(NU, IV, ST), ra(FO, IV, ST), ra(AR, IV, ST)];
const TO_NUMBER: &[Rule] = &[ra(AM, IV, NU), ra(ST, IV, NU), ra(NU, IV, NU)];
const DEREFERENCE: &[Rule] = &[ra(AM, IV, AE), ra(AR, IV, AE)];
#[rustfmt::skip]
const BOX: &[Rule] = &[ra(AM, IV, AR), ra(NU, IV, AR), ra(ST, IV, AR), ra(FO, IV, AR), ra(AR, IV, AR)];
#[rustfmt::skip]
const MAKE_PAIR: &[Rule] = &[
    ra(ST, AM, PA), ra(NU, AM, PA), ra(AM, NU, PA), ra(AM, ST, PA), ra(AM, AR, PA), ra(AM, FO, PA),
    ra(AM, AM, PA),
    ra(ST, NU, PA), ra(ST, ST, PA), ra(ST, FO, PA), ra(ST, AR, PA), ra(NU, NU, PA), ra(NU, ST, PA),
    ra(NU, FO, PA), ra(NU, AR, PA),
];
/// The run-time build's dot rules (the editor's third rule is left out there).
const DOT: &[Rule] = &[ra(FO, CM, CM), ra(AE, CM, CM)];
const BITWISE_NOT: &[Rule] = &[ra(NU, IV, NU)];

/// An operator's rules (`numRules`, `rules`).
fn rules(o: u8) -> &'static [Rule] {
    match o {
        op::ASSIGNMENT => ASSIGNMENT,
        op::LOGICAL_OR | op::LOGICAL_AND => LOGICAL,
        op::SLICE => SLICE,
        op::EQUALS | op::NOT_EQUAL => EQUALITY,
        op::GREATER_THAN..=op::LESS_OR_EQUAL => COMPARISON,
        op::BITWISE_OR..=op::RIGHT_SHIFT | op::MODULO => INTEGER,
        op::ADD => ADDITION,
        op::SUBTRACT | op::DIVIDE | op::EXPONENT => ARITHMETIC,
        op::MULTIPLY => MULTIPLY,
        op::NEGATION => NEGATION,
        op::LOGICAL_NOT => LOGICAL_NOT,
        op::LEFT_BRACKET => LEFT_BRACKET,
        op::IN => IN,
        op::TO_STRING => TO_STRING,
        op::PLUS_EQUALS => PLUS_EQUALS,
        op::TIMES_EQUALS..=op::MINUS_EQUALS => NUMBER_EQUALS,
        op::TO_NUMBER => TO_NUMBER,
        op::DEREFERENCE => DEREFERENCE,
        op::MEMBER_ACCESS => MEMBER_ACCESS,
        op::MAKE_PAIR => MAKE_PAIR,
        op::BOX => BOX,
        op::DOT => DOT,
        op::BITWISE_OR_EQUALS..=op::MODULO_EQUALS => NUMBER_EQUALS,
        op::BITWISE_NOT => BITWISE_NOT,
        _ => &[],
    }
}

/// `kOpType_LeftParen`, `RightParen`, `RightBracket`, `LeftBrace`, `RightBrace`.
const LEFT_PAREN: u8 = 22;
const RIGHT_PAREN: u8 = 23;
const RIGHT_BRACKET: u8 = 25;
const LEFT_BRACE: u8 = 38;
const RIGHT_BRACE: u8 = 39;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptTokens.cpp: Operator::GetResult.
/// The type an operator gives its operands' types at compile time (`GetResult`), `INVALID` for
/// none: the first rule that fits, either way round unless it is asymmetric.
pub(crate) fn result_type(o: u8, lhs: Code, rhs: Code) -> Code {
    for rule in rules(o) {
        let mut result = IV;
        if convert(lhs, rule.0) && convert(rhs, rule.1) {
            result = rule.2;
        }
        if !rule.3 && convert(lhs, rule.1) && convert(rhs, rule.0) {
            result = rule.2;
        }
        if result == tt::LEFT {
            result = lhs;
        } else if result == RT {
            result = rhs;
        }
        if result != IV {
            return result;
        }
    }
    IV
}

fn is_unary(o: u8) -> bool {
    operand_count(o) == 1
}
fn is_binary(o: u8) -> bool {
    operand_count(o) == 2
}
fn is_open_bracket(o: u8) -> bool {
    matches!(o, LEFT_PAREN | LEFT_BRACE | op::LEFT_BRACKET)
}
fn is_closing_bracket(o: u8) -> bool {
    matches!(o, RIGHT_PAREN | RIGHT_BRACE | RIGHT_BRACKET)
}
fn is_right_associative(o: u8) -> bool {
    o == op::ASSIGNMENT || is_unary(o) || (op::PLUS_EQUALS..=op::MINUS_EQUALS).contains(&o)
}
/// `Operator::Precedes`: whether `top` (on the stack) is popped before `o` is pushed.
fn precedes(top: u8, o: u8) -> bool {
    let (tp, p) = (
        crate::expr::op_precedence(top),
        crate::expr::op_precedence(o),
    );
    if is_right_associative(top) {
        p < tp
    } else {
        p <= tp
    }
}
fn matched_bracket(o: u8) -> u8 {
    match o {
        op::LEFT_BRACKET => b']',
        RIGHT_BRACKET => b'[',
        LEFT_PAREN => b')',
        RIGHT_PAREN => b'(',
        LEFT_BRACE => b'}',
        RIGHT_BRACE => b'{',
        _ => 0,
    }
}
fn symbol(o: u8) -> &'static [u8] {
    op_symbol(o).as_bytes()
}

// ---- the C runtime's character classes ("C" locale; bytes from 0x80 are none of them) ----

pub(crate) fn is_space(c: u8) -> bool {
    matches!(c, 0x09..=0x0d | b' ')
}
pub(crate) fn is_punct(c: u8) -> bool {
    matches!(c, 0x21..=0x2f | 0x3a..=0x40 | 0x5b..=0x60 | 0x7b..=0x7e)
}

// ---- the compiler's state ----

/// What xNVSE's compiler keeps between calls (globals in its source), on the engine.
#[derive(Default)]
pub struct State {
    /// `s_parserDepth`.
    depth: i32,
    /// `g_lastCommandReturnType`.
    last_command_type: Option<Code>,
    /// `g_lambdaParentScriptMap`: a lambda made while compiling and its parent script.
    pub(crate) lambda_parents: Vec<(u32, u32)>,
    /// `g_currentScriptStack`.
    pub(crate) scripts: Vec<u32>,
    /// `g_variableDefinitionsMap`: a variable's type by script and name (the name as written).
    pub(crate) var_types: HashMap<(u32, Vec<u8>), u8>,
    /// `s_loopStartOffsets`: the open loops' jump words, by their offset in the script's data
    /// (xNVSE keeps their addresses; see [`begin_loop_parse`]).
    loop_starts: Vec<u32>,
    /// `g_patchedEol`: the game's end-of-line check is off while a nested command parses.
    pub(crate) eol_check_off: bool,
    /// `PatchDisable_ScriptBufferValidateRefVars`: a lambda compiles without the check of its
    /// referenced objects.
    pub(crate) skip_ref_check: bool,
    /// `g_currentScriptRestorePoundChar`: the `#version` tags made comments, per open compile.
    pub(crate) pound_restores: Vec<Vec<u32>>,
}

pub(crate) fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Nvse>().compiler
}

/// `g_scriptCommands.GetByName`: while a script compiles, through the table's name map
/// ([`crate::table::opcode_by_name_mapped`], made once); otherwise the table's own walk.
pub(crate) fn command_by_name(e: &mut Engine, name: &[u8]) -> Option<u32> {
    if state(e).scripts.is_empty() {
        return opcode_by_name(e, name);
    }
    crate::table::opcode_by_name_mapped(e, name)
}

// ---- guest helpers ----

fn ascii_eq_ci(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// `StrCompare(l, r) == 0` (the game's upper case table), `None` for a null text.
pub(crate) fn str_equal(e: &mut Engine, l: Option<&[u8]>, r: Option<&[u8]>) -> bool {
    match (l, r) {
        (None, None) => true,
        (Some(l), Some(r)) => {
            let upper = crate::vars::upper_table(e);
            crate::vars::str_compare(&upper, l, r) == std::cmp::Ordering::Equal
        }
        _ => false,
    }
}

/// A NUL-terminated text, `None` for a null pointer.
pub(crate) fn opt_text(e: &Engine, at: u32) -> Option<Vec<u8>> {
    (at != 0).then(|| guest_text(e, at))
}

/// A `String` (`m_data`, `m_dataLen`, `m_bufLen`): its data's text (`None`: null).
pub(crate) fn string_data(e: &Engine, s: u32) -> Option<Vec<u8>> {
    opt_text(e, e.mem.u32(s))
}

/// `String::CStr`: the text, or `""` for no data or a zero length.
pub(crate) fn string_cstr(e: &Engine, s: u32) -> Vec<u8> {
    if e.mem.u32(s) != 0 && e.mem.u16(s + 4) != 0 {
        guest_text(e, e.mem.u32(s))
    } else {
        Vec::new()
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTypes.cpp: String::Set.
/// `String::Set` (xNVSE's own, on the game's heap).
pub(crate) fn string_set(e: &mut Engine, s: u32, text: &[u8]) {
    let len = text.len() as u32;
    let mut data = e.mem.u32(s);
    if len > u32::from(e.mem.u16(s + 6)) {
        e.call(FORM_HEAP_FREE, &args![data]);
        data = e.call(FORM_HEAP_ALLOCATE, &args![len + 1]).u32();
        e.mem.set_u32(s, data);
        e.mem.set_u16(s + 6, if data != 0 { len as u16 } else { 0 });
    }
    if data != 0 {
        e.mem.write(data, text);
        e.mem.set_u8(data + len, 0);
        e.mem.set_u16(s + 4, len as u16);
    }
}

/// The heap copy of a text, NUL-terminated.
fn heap_text(e: &mut Engine, text: &[u8]) -> u32 {
    let at = e
        .call(FORM_HEAP_ALLOCATE, &args![text.len() as u32 + 1])
        .u32();
    e.mem.write(at, text);
    e.mem.set_u8(at + text.len() as u32, 0);
    at
}

/// The items of a `tList` (its head node at `head`), in order, nulls left out.
pub(crate) fn list_items(e: &Engine, head: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut node = head;
    // A list longer than the guest's memory could hold is a broken one.
    for _ in 0..0x10_0000 {
        if node == 0 {
            break;
        }
        let item = e.mem.u32(node);
        if item != 0 {
            out.push(item);
        }
        node = e.mem.u32(node + 4);
    }
    out
}

/// `tList::Append`: into the head node when it is empty, else a new node after the last.
pub(crate) fn list_append(e: &mut Engine, head: u32, item: u32) {
    let mut last = head;
    for _ in 0..0x10_0000 {
        let next = e.mem.u32(last + 4);
        if next == 0 {
            break;
        }
        last = next;
    }
    if e.mem.u32(last) == 0 {
        e.mem.set_u32(last, item);
        return;
    }
    let node = e.call(FORM_HEAP_ALLOCATE, &args![8u32]).u32();
    e.mem.set_u32(node, item);
    e.mem.set_u32(node + 4, 0);
    e.mem.set_u32(last + 4, node);
}

/// `Script::VarInfoList::GetVariableByName` over the list at `head`.
pub(crate) fn variable_by_name(e: &mut Engine, head: u32, name: &[u8]) -> u32 {
    for info in list_items(e, head) {
        let n = string_data(e, info + 0x18);
        if str_equal(e, n.as_deref(), Some(name)) {
            return info;
        }
    }
    0
}

fn cast(e: &mut Engine, form: u32, to: u32) -> u32 {
    if form == 0 {
        return 0;
    }
    e.call(DYNAMIC_CAST, &args![form, 0u32, RTTI_TES_FORM, to, 0u32])
        .u32()
}

fn form_type(e: &Engine, form: u32) -> u8 {
    e.mem.u8(form + 4)
}

/// `TESForm::GetIsReference` (the run-time build's virtual).
fn is_reference(e: &mut Engine, form: u32) -> bool {
    e.vcall(form, GET_IS_REFERENCE, &[]).bool()
}

/// `GetFormByID(editorID)`.
pub(crate) fn form_by_editor_id(e: &mut Engine, name: &[u8]) -> u32 {
    let size = (name.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, at| {
        e.mem.write(at.addr(), name);
        e.mem.set_u8(at.addr() + name.len() as u32, 0);
        e.call(FORM_BY_EDITOR_ID, &args![at.addr()]).u32()
    })
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp: GetScriptFromForm.
/// `GetScriptFromForm`: a reference's base form's script, or the form's own.
pub(crate) fn script_of_form(e: &mut Engine, mut form: u32) -> u32 {
    if cast(e, form, RTTI_TES_OBJECT_REFR) != 0 {
        form = crate::commands::base_form(e, form);
    }
    let scriptable = cast(e, form, RTTI_TES_SCRIPTABLE_FORM);
    if scriptable != 0 {
        e.mem.u32(scriptable + 4)
    } else {
        0
    }
}

// ---- printing ----

/// An argument of a message format.
pub(crate) enum A<'a> {
    S(&'a [u8]),
    D(i64),
}

/// `vsprintf` of the `%s` and `%d` formats xNVSE's compiler messages use, held to `cap` bytes
/// less the NUL (`vsprintf_s` into a buffer of that size).
pub(crate) fn format(f: &str, args: &[A], cap: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut it = args.iter();
    let b = f.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 1 < b.len() {
            match b[i + 1] {
                b's' => {
                    if let Some(A::S(s)) = it.next() {
                        out.extend_from_slice(s);
                    }
                    i += 2;
                    continue;
                }
                b'd' => {
                    if let Some(A::D(d)) = it.next() {
                        out.extend_from_slice((*d as i32).to_string().as_bytes());
                    }
                    i += 2;
                    continue;
                }
                b'%' => {
                    out.push(b'%');
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out.truncate(cap.saturating_sub(1));
    out
}

/// `g_ErrOut.Show` at run time (`ShowError`): the text to the console.
pub(crate) fn show_error(e: &mut Engine, text: &[u8]) {
    crate::errors::print_long(e, text);
}

/// `ShowCompilerError(lineBuf, ...)`: "Error on line N" and the message to the console.
fn compiler_error(e: &mut Engine, line: u32, message: &[u8]) {
    let mut text = format(
        "Error on line %d\n\n",
        &[A::D(i64::from(e.mem.u32(line + ln::NUMBER) as i32))],
        0x400,
    );
    let mut m = message.to_vec();
    m.truncate(0x1ff);
    text.extend_from_slice(&m);
    text.truncate(0x3ff);
    show_error(e, &text);
}

/// `ExpressionParser::ScriptLineError`, with `s_expressionErrors`' texts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Msg {
    CantParse,
    TooManyOperators,
    TooManyOperands,
    MismatchedBrackets,
    InvalidOperands,
    MismatchedQuotes,
    InvalidDotSyntax,
    CantFindVariable,
    ExpectedStringVariable,
    TooManyArgs,
    RefRequired,
    MissingParam,
    ExpectedUserFunction,
    UserFunctionParamsUndefined,
    ExpectedStringLiteral,
    UnquotedString,
}

impl Msg {
    fn text(self) -> &'static str {
        match self {
            Msg::CantParse => "Could not parse this line.",
            Msg::TooManyOperators => "Too many operators.",
            Msg::TooManyOperands => "Too many operands.",
            Msg::MismatchedBrackets => "Mismatched brackets.",
            Msg::InvalidOperands => "Invalid operands for operator %s",
            Msg::MismatchedQuotes => "Mismatched quotes.",
            Msg::InvalidDotSyntax => "Left of dot must be quest or persistent reference.",
            Msg::CantFindVariable => "Unknown variable '%s'.",
            Msg::ExpectedStringVariable => "Expected string variable after '$'.",
            Msg::TooManyArgs => "More args provided than expected by function or command.",
            Msg::RefRequired => "Commands '%s' must be called on a reference.",
            Msg::MissingParam => "Missing required parameter '%s' for parameter #'%d'.",
            Msg::ExpectedUserFunction => "Expected user function.",
            Msg::UserFunctionParamsUndefined => "Could not parse user function parameter list in function definition.\nMay be caused by undefined variable,  missing {brackets}, or attempt to use a single variable to hold more than one parameter.",
            Msg::ExpectedStringLiteral => "Expected string literal.",
            Msg::UnquotedString => "Unquoted argument '%s' will be treated as string by default. Check spelling if a form or variable was intended.",
        }
    }

    /// A warning (`bCanDisable`): printed without the line header; at run time it never
    /// stops anything.
    fn is_warning(self) -> bool {
        self == Msg::UnquotedString
    }
}

// ---- the line's output (`ScriptLineBuffer::Write*`) ----

fn data_len(e: &Engine, line: u32) -> u32 {
    e.mem.u32(line + ln::DATA_LEN)
}
fn set_data_len(e: &mut Engine, line: u32, v: u32) {
    e.mem.set_u32(line + ln::DATA_LEN, v);
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp: ScriptLineBuffer::Write.
/// `ScriptLineBuffer::Write`: refused (with xNVSE's message) when the output would reach its
/// 0x200 bytes.
pub(crate) fn write(e: &mut Engine, line: u32, bytes: &[u8]) -> bool {
    let off = data_len(e, line);
    if off.wrapping_add(bytes.len() as u32) >= ln::SIZE {
        show_error(
            e,
            b"Line data buffer overflow! To fix this make the line shorter in length.",
        );
        return false;
    }
    e.mem.write(line + ln::DATA + off, bytes);
    set_data_len(e, line, off + bytes.len() as u32);
    true
}

/// A word written back into the output at `at` (a length xNVSE stores where it reserved room):
/// kept inside the line's structure, where xNVSE's would go wherever the offset points.
fn patch_u16(e: &mut Engine, line: u32, at: u32, v: u16) {
    if ln::DATA.wrapping_add(at) <= ln::STRUCT - 2 {
        e.mem.set_u16(line + ln::DATA + at, v);
    }
}
fn patch_u8(e: &mut Engine, line: u32, at: u32, v: u8) {
    if ln::DATA.wrapping_add(at) < ln::STRUCT {
        e.mem.set_u8(line + ln::DATA + at, v);
    }
}

// ---- tokens (`ScriptToken`, compile-time kinds) ----

/// A token the parser made.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Tok {
    Number(f64),
    Str(Vec<u8>),
    /// A form of the reference list (`RefVariable`, its index).
    Ref {
        ref_var: u32,
        idx: u16,
    },
    Global {
        idx: u16,
    },
    /// A command (its record), its calling reference's index, and whether it takes its calling
    /// reference from the stack (the dot syntax).
    Command {
        record: u32,
        idx: u16,
        from_stack: bool,
    },
    /// A variable (`VariableInfo`): its token type (`INVALID` when its type is unknown), its
    /// variable type, its reference's index.
    Var {
        code: Code,
        var_type: u8,
        idx: u16,
        info: u32,
    },
    Lambda(u32),
    OptionalEmpty,
}

impl Tok {
    fn code(&self) -> Code {
        match self {
            Tok::Number(_) => tt::NUMBER,
            Tok::Str(_) => tt::STRING,
            Tok::Ref { .. } => tt::REF,
            Tok::Global { .. } => tt::GLOBAL,
            Tok::Command { .. } => tt::COMMAND,
            Tok::Var { code, .. } => *code,
            Tok::Lambda(_) => tt::LAMBDA,
            Tok::OptionalEmpty => tt::OPTIONAL_EMPTY,
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptTokens.cpp: ScriptToken
    // (VariableInfo *, UInt16, UInt32).
    /// A variable's token by its variable type.
    fn var(info: u32, idx: u16, var_type: u8) -> Tok {
        let code = match var_type {
            vt::ARRAY => tt::ARRAY_VAR,
            vt::STRING => tt::STRING_VAR,
            vt::INTEGER | vt::FLOAT => tt::NUMERIC_VAR,
            vt::REF => tt::REF_VAR,
            _ => IV,
        };
        Tok::Var {
            code,
            var_type,
            idx,
            info,
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptTokens.cpp: ScriptToken::Write.
    /// `ScriptToken::Write`: its code letter and value.
    fn write(&self, e: &mut Engine, line: u32) -> bool {
        match self {
            Tok::Number(v) => {
                if v.floor() != *v {
                    write(e, line, b"Z");
                    write(e, line, &v.to_le_bytes())
                } else {
                    // The unary minus is an operator: every literal is positive.
                    let val = crate::commands::string::dtoui3(*v);
                    if val < 0x100 {
                        write(e, line, b"B");
                        write(e, line, &[val as u8])
                    } else if val < 0x1_0000 {
                        write(e, line, b"I");
                        write(e, line, &(val as u16).to_le_bytes())
                    } else {
                        write(e, line, b"L");
                        write(e, line, &val.to_le_bytes())
                    }
                }
            }
            Tok::Str(s) => {
                write(e, line, b"S");
                // `WriteString`: a length word that does not fit answers true, as xNVSE's does.
                let len = s.len() as u32;
                if len < 0x1_0000 && write(e, line, &(len as u16).to_le_bytes()) && len != 0 {
                    return write(e, line, s);
                }
                true
            }
            Tok::Ref { idx, .. } => {
                write(e, line, b"R");
                write(e, line, &idx.to_le_bytes())
            }
            Tok::Global { idx } => {
                write(e, line, b"G");
                write(e, line, &idx.to_le_bytes())
            }
            Tok::Command {
                record,
                idx,
                from_stack,
            } => {
                write(e, line, if *from_stack { b"x" } else { b"X" });
                write(e, line, &idx.to_le_bytes());
                let opcode = e.mem.u32(record + 8) as u16;
                write(e, line, &opcode.to_le_bytes())
            }
            Tok::Var {
                code,
                var_type,
                idx,
                info,
            } => {
                if *code == IV {
                    show_error(
                        e,
                        &format(
                            "Unhandled token type %s for ScriptToken::Write",
                            &[A::S(type_name(*code).as_bytes())],
                            0x400,
                        ),
                    );
                    return false;
                }
                write(e, line, b"V");
                write(e, line, &[*var_type]);
                write(e, line, &idx.to_le_bytes());
                let var_idx = if *info == 0 { 0 } else { e.mem.u32(*info) };
                write(e, line, &(var_idx as u16).to_le_bytes())
            }
            Tok::Lambda(script) => {
                write(e, line, b"F");
                let len = e.mem.u32(script + sc::DATA_LEN);
                write(e, line, &len.to_le_bytes());
                // A length past the output cannot be written: refused as any write past it.
                if len >= ln::SIZE {
                    return write(e, line, &vec![0; ln::SIZE as usize]);
                }
                let data = e.mem.u32(script + sc::DATA);
                let bytes = e.mem.bytes(data, len);
                write(e, line, &bytes)
            }
            Tok::OptionalEmpty => {
                write(e, line, b"K");
                true
            }
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptTokens.cpp:
    // ScriptToken::GetTESForm (the compile-time kinds).
    /// `GetTESForm` of a compile-time token: a lambda's script, a reference's form, or for a
    /// reference variable (whose `value.var` is its `VariableInfo` here) the form whose id
    /// that variable's data holds.
    fn form(&self, e: &mut Engine) -> u32 {
        match self {
            Tok::Lambda(s) => *s,
            Tok::Ref { ref_var, .. } if *ref_var != 0 => e.mem.u32(ref_var + 8),
            Tok::Var {
                code: tt::REF_VAR,
                info,
                ..
            } if *info != 0 => {
                let id = e.mem.u32(info + 8);
                e.call(LOOKUP_FORM, &args![id]).u32()
            }
            _ => 0,
        }
    }
}

// ---- parameters ----

/// A parameter (`ParamInfo`): its name (`typeStr`), its type, whether it is optional.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Param {
    pub name: Vec<u8>,
    pub kind: u32,
    pub optional: bool,
}

/// A command's parameters: the guest's list (read as asked), or a list of the compiler's own.
pub(crate) enum Params {
    Guest { at: u32, count: u32 },
    Host(Vec<Param>),
}

impl Params {
    fn count(&self) -> u32 {
        match self {
            Params::Guest { count, .. } => *count,
            Params::Host(v) => v.len() as u32,
        }
    }
    fn get(&self, e: &Engine, i: u32) -> Param {
        match self {
            Params::Guest { at, .. } => {
                let p = at.wrapping_add(i.wrapping_mul(12));
                Param {
                    name: guest_text(e, e.mem.u32(p)),
                    kind: e.mem.u32(p + 4),
                    optional: e.mem.u32(p + 8) != 0,
                }
            }
            Params::Host(v) => v[i as usize].clone(),
        }
    }
}

/// `kNVSEParamType_NoTypeCheck`.
const NO_TYPE_CHECK: u32 = 0;
/// `kNVSEParamType_Number`, `_String`, `_Array`, `_Form`.
pub(crate) const NVSE_NUMBER: u32 = (1 << tt::NUMBER) | (1 << tt::AMBIGUOUS);
pub(crate) const NVSE_STRING: u32 = (1 << tt::STRING) | (1 << tt::AMBIGUOUS);
pub(crate) const NVSE_ARRAY: u32 = (1 << tt::ARRAY) | (1 << tt::AMBIGUOUS);
pub(crate) const NVSE_FORM: u32 = (1 << tt::FORM) | (1 << tt::AMBIGUOUS);

/// `kParams_DefaultUserFunctionParams`: ten unchecked optional arguments.
fn default_user_function_params() -> Vec<Param> {
    (0..10)
        .map(|_| Param {
            name: b"argument".to_vec(),
            kind: NO_TYPE_CHECK,
            optional: true,
        })
        .collect()
}

/// `kMaxUdfParams`.
const MAX_UDF_PARAMS: usize = 15;
/// `kUserFunction_Version`.
const USER_FUNCTION_VERSION: u8 = 1;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// ExpressionParser::ValidateArgType (its part for xNVSE's parameter types: at run time every
// caller passes them; the game's types are checked only by the GECK's inline expressions).
/// `ValidateArgType`: whether an argument of type `arg` fits a parameter of `kind`: any type
/// for no check, else one of the kind's types that the argument converts to.
pub(crate) fn validate_arg_type(kind: u32, arg: Code) -> bool {
    if kind == NO_TYPE_CHECK {
        return true;
    }
    (0..tt::INVALID).any(|i| kind & (1 << i) != 0 && convert(arg, i))
}

// ---- text helpers ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp: FormatString.
/// `FormatString` of a string argument: `%r` a line break, `%q` a quote, `%%` kept.
pub(crate) fn format_string(s: &mut Vec<u8>) {
    let mut pos = 0;
    while let Some(found) = s[pos.min(s.len())..].iter().position(|&c| c == b'%') {
        pos += found;
        if pos + 1 >= s.len() {
            break;
        }
        let insert = match s[pos + 1] {
            b'%' => {
                pos += 2;
                continue;
            }
            b'r' | b'R' => b'\n',
            b'q' | b'Q' => b'"',
            _ => {
                pos += 1;
                continue;
            }
        };
        s.splice(pos..pos + 2, [insert]);
        pos += 1;
    }
}

/// `std::stoul(token, &parsed, radix)` with the whole token parsed: the value, or `None` where
/// xNVSE's catch or its length check refuses it.
fn stoul_whole(t: &[u8], radix: u32) -> Option<u32> {
    let mut i = 0;
    if radix == 16
        && t.len() > 2
        && t[0] == b'0'
        && (t[1] | 0x20) == b'x'
        && (t[2] as char).is_ascii_hexdigit()
    {
        i = 2;
    }
    let start = i;
    let mut v: u64 = 0;
    while i < t.len() {
        let Some(d) = (t[i] as char).to_digit(radix) else {
            break;
        };
        v = v * u64::from(radix) + u64::from(d);
        if v > u64::from(u32::MAX) {
            // `ERANGE`: `out_of_range`, caught.
            return None;
        }
        i += 1;
    }
    if i == start {
        // No digit at all: `invalid_argument`, caught.
        return None;
    }
    (i == t.len()).then_some(v as u32)
}

// ---- the parser (`ExpressionParser`) ----

/// `ScriptLineMacro`'s kinds xNVSE 6.4.9 registers (only the one-line lambda).
const MACRO_ONE_LINE_LAMBDA: u8 = 1;

/// A saved line (`SavedScriptLine`): its text from the offset, the offset, the length.
struct Saved {
    text: Vec<u8>,
    at: u32,
    len: u32,
}

/// An `ExpressionParser` on a line and a script buffer.
pub(crate) struct Parser {
    buf: u32,
    line: u32,
    len: u32,
    args_parsed: u32,
    saved: Vec<Saved>,
    macros: u8,
    script: u32,
}

/// Runs `f` with a new parser, as xNVSE's constructor and destructor bracket a parse: the
/// parser depth counts it, and when the outermost parser ends, the lambdas made meanwhile are
/// deleted.
pub(crate) fn with_parser<R>(
    e: &mut Engine,
    buf: u32,
    line: u32,
    f: impl FnOnce(&mut Engine, &mut Parser) -> R,
) -> R {
    let stored = e.mem.u32(line + ln::TEXT_LEN);
    let len = if stored != 0 {
        stored
    } else {
        text_len(e, line, 0)
    }
    .min(ln::SIZE);
    let script = state(e)
        .scripts
        .last()
        .copied()
        .unwrap_or_else(|| e.mem.u32(buf + sb::CURRENT_SCRIPT));
    state(e).depth += 1;
    let mut p = Parser {
        buf,
        line,
        len,
        args_parsed: 0,
        saved: Vec::new(),
        macros: 0,
        script,
    };
    let result = f(e, &mut p);
    let s = state(e);
    s.depth -= 1;
    if s.depth == 0 {
        let lambdas: Vec<u32> = s.lambda_parents.drain(..).map(|(l, _)| l).collect();
        for lambda in lambdas {
            delete_script(e, lambda);
        }
    }
    result
}

/// The length of the line's text from `from` to its NUL (or the buffer's end).
fn text_len(e: &Engine, line: u32, from: u32) -> u32 {
    let mut i = from;
    while i < ln::SIZE && e.mem.u8(line + ln::TEXT + i) != 0 {
        i += 1;
    }
    i - from.min(i)
}

/// `Script::Delete`: the destructor and the heap block.
fn delete_script(e: &mut Engine, script: u32) {
    e.call(SCRIPT_DESTRUCTOR, &args![script]);
    e.call(FORM_HEAP_FREE, &args![script]);
}

impl Parser {
    // ---- the cursor ----

    fn offset(&self, e: &Engine) -> u32 {
        e.mem.u32(self.line + ln::OFFSET)
    }
    fn set_offset(&self, e: &mut Engine, v: u32) {
        e.mem.set_u32(self.line + ln::OFFSET, v);
    }
    fn advance(&self, e: &mut Engine, n: u32) {
        let o = self.offset(e);
        self.set_offset(e, o.wrapping_add(n));
    }
    /// `Peek(idx)`: the text's byte, 0 at or past the length; index `u32::MAX` is `Peek`'s
    /// default argument (-1), the cursor's byte (where the argument-end trim steps below 0).
    fn peek_at(&self, e: &Engine, idx: u32) -> u8 {
        let idx = if idx == u32::MAX { self.offset(e) } else { idx };
        if idx < self.len {
            e.mem.u8(self.line + ln::TEXT + idx)
        } else {
            0
        }
    }
    fn peek(&self, e: &Engine) -> u8 {
        self.peek_at(e, self.offset(e))
    }
    /// The text's byte at `idx` without the length (`Text()[idx]`), 0 past the buffer.
    fn text_byte(&self, e: &Engine, idx: u32) -> u8 {
        if idx < ln::SIZE {
            e.mem.u8(self.line + ln::TEXT + idx)
        } else {
            0
        }
    }
    fn set_text_byte(&self, e: &mut Engine, idx: u32, v: u8) {
        if idx < ln::SIZE {
            e.mem.set_u8(self.line + ln::TEXT + idx, v);
        }
    }
    /// The text from `idx` to its NUL (`std::string(Text() + idx)`).
    fn text_from(&self, e: &Engine, idx: u32) -> Vec<u8> {
        let n = text_len(e, self.line, idx);
        if n == 0 {
            return Vec::new();
        }
        e.mem.bytes(self.line + ln::TEXT + idx, n)
    }
    fn skip_spaces(&self, e: &mut Engine) {
        // `isspace(*CurText())`: the text's own bytes, not held to the length.
        while is_space(self.text_byte(e, self.offset(e))) {
            self.advance(e, 1);
        }
    }

    // ---- messages ----

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::Message.
    /// `Message`: an error with the line's number, a warning as it is.
    fn message(&self, e: &mut Engine, m: Msg, args: &[A]) {
        let body = format(m.text(), args, 0x400);
        if m.is_warning() {
            show_error(e, &body);
            return;
        }
        let number = i64::from(e.mem.u32(self.line + ln::NUMBER) as i32);
        let mut text = format("Error line %d\n\n", &[A::D(number)], 0x400);
        text.extend_from_slice(&body);
        text.truncate(0x3ff);
        show_error(e, &text);
    }

    /// `PrintCompileError`.
    fn compile_error(&self, e: &mut Engine, message: &[u8]) {
        compiler_error(e, self.line, message);
    }

    // ---- saved lines ----

    fn save_line(&mut self, e: &Engine) {
        let at = self.offset(e);
        self.saved.push(Saved {
            text: self.text_from(e, at),
            at,
            len: self.len,
        });
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::RestoreScriptLine.
    /// `RestoreScriptLine`: where the text from the saved offset changed (a macro, a bracket's
    /// end cut), the saved text comes back and the cursor goes to its end.
    fn restore_line(&mut self, e: &mut Engine) {
        let Some(s) = self.saved.pop() else {
            return;
        };
        let now = self.text_from(e, s.at);
        if !ascii_eq_ci(&now, &s.text) {
            let room = ln::SIZE.saturating_sub(s.at);
            if (s.text.len() as u32) < room {
                e.mem.write(self.line + ln::TEXT + s.at, &s.text);
                e.mem
                    .set_u8(self.line + ln::TEXT + s.at + s.text.len() as u32, 0);
            }
            self.len = s.len;
            e.mem.set_u32(self.line + ln::TEXT_LEN, s.len);
            self.set_offset(e, s.at + s.text.len() as u32);
        }
    }

    // ---- macros ----

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // g_expressionParserMacros (the one-line lambda), ExpressionParser::HandleMacros,
    // ScriptLineMacro::EvalMacro.
    /// `HandleMacros`: the one-line lambda `{a, b} => expression` at the cursor becomes
    /// `begin function {a, b}`, `SetFunctionValue expression`, `end` (three lines). False when
    /// it does not fit the line.
    fn handle_macros(&mut self, e: &mut Engine) -> bool {
        let at = self.offset(e);
        let text = self.text_from(e, at);
        let Some(rewritten) = one_line_lambda(&text) else {
            return true;
        };
        let left = ln::SIZE.saturating_sub(at);
        // `strcpy_s` needs room for the NUL too: a text of exactly the room left is refused as
        // well (xNVSE's would stop the game there).
        if rewritten.len() as u32 >= left {
            show_error(e, b"Line length limit reached, could not translate macro.");
            return false;
        }
        e.mem.write(self.line + ln::TEXT + at, &rewritten);
        e.mem
            .set_u8(self.line + ln::TEXT + at + rewritten.len() as u32, 0);
        let len = rewritten.len() as u32 + at;
        e.mem.set_u32(self.line + ln::TEXT_LEN, len);
        self.len = len.min(ln::SIZE);
        self.macros |= MACRO_ONE_LINE_LAMBDA;
        true
    }

    // ---- operators ----

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseOperator.
    /// `ParseOperator`: the operator at the cursor, binary (or a closing bracket) or unary (or
    /// an opening bracket); a comma is a delimiter, passed over even when not consuming.
    fn parse_operator(&self, e: &mut Engine, binary: bool, consume: bool) -> Option<u8> {
        let first = self.peek(e);
        if first == b',' {
            self.advance(e, 1);
            return None;
        }
        let mut ops: Vec<u8> = (0..op::MAX)
            .filter(|&o| {
                if binary {
                    is_binary(o) || is_closing_bracket(o)
                } else {
                    is_unary(o) || is_open_bracket(o)
                }
            })
            .filter(|&o| symbol(o)[0] == first)
            .collect();
        let second = self.peek_at(e, self.offset(e).wrapping_add(1));
        let mut found = None;
        if second != 0 && is_punct(second) {
            let mut i = 0;
            while i < ops.len() {
                let s = symbol(ops[i]);
                let s1 = s.get(1).copied().unwrap_or(0);
                if s1 == second {
                    found = Some(ops[i]);
                    break;
                } else if s1 != 0 {
                    ops.remove(i);
                } else {
                    i += 1;
                }
            }
        } else {
            found = ops.iter().copied().find(|&o| symbol(o).len() == 1);
            // "for consistency with macro": a lone `=` is the assignment.
            if found.is_none() && first == b'=' {
                found = Some(op::ASSIGNMENT);
            }
        }
        if found.is_none() && ops.len() == 1 {
            found = Some(ops[0]);
        }
        if let Some(o) = found {
            if consume {
                self.advance(e, symbol(o).len() as u32);
            }
        }
        found
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::MatchOpenBracket.
    /// `MatchOpenBracket`: the position of the bracket closing the one just passed, `Err(-1)`
    /// for none, `Err(-2)` for a quote left open inside.
    fn match_open_bracket(&self, e: &Engine, open: u8) -> Result<u32, i32> {
        let closing = matched_bracket(open);
        let opening = symbol(open)[0];
        let mut count = 1u32;
        let mut i = self.offset(e);
        while i < self.len && self.text_byte(e, i) != 0 {
            let c = self.text_byte(e, i);
            if c == b'"' {
                i += 1;
                let mut closed = false;
                while i < self.len && self.text_byte(e, i) != 0 {
                    if self.text_byte(e, i) == b'"' {
                        closed = true;
                        break;
                    }
                    i += 1;
                }
                if closed {
                    i += 1;
                    continue;
                }
                return Err(-2);
            } else if c == opening {
                count += 1;
            } else if c == closing {
                count -= 1;
            }
            if count == 0 {
                break;
            }
            i += 1;
        }
        if count != 0 {
            Err(-1)
        } else {
            Ok(i)
        }
    }

    /// The bracket's position, or its error's message.
    fn bracket_end(&self, e: &mut Engine, open: u8) -> Option<u32> {
        match self.match_open_bracket(e, open) {
            Ok(i) => Some(i),
            Err(-2) => {
                self.message(e, Msg::MismatchedQuotes, &[]);
                None
            }
            Err(_) => {
                self.message(e, Msg::MismatchedBrackets, &[]);
                None
            }
        }
    }

    // ---- tokens ----

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::GetCurToken.
    /// `GetCurToken`: a quoted string with its quotes, or letters, digits and `_` (and `.`
    /// while it is all digits); `Err` at the end of the text with nothing read
    /// (`OffsetOutOfBoundsError`).
    fn cur_token(&self, e: &mut Engine) -> Result<Vec<u8>, ()> {
        let start = self.offset(e);
        if self.peek(e) == b'"' {
            self.advance(e, 1);
            let mut ch = self.peek(e);
            while ch != 0 && ch != b'"' {
                self.advance(e, 1);
                ch = self.peek(e);
            }
            if ch == b'"' {
                self.advance(e, 1);
            }
            return Ok(self.slice(e, start, self.offset(e)));
        }
        let mut numeric = true;
        let mut ch = self.peek(e);
        while ch != 0 {
            if is_space(ch) || (is_punct(ch) && ch != b'_' && (ch != b'.' || !numeric)) {
                break;
            }
            self.advance(e, 1);
            if !ch.is_ascii_digit() {
                numeric = false;
            }
            ch = self.peek(e);
        }
        let token = self.slice(e, start, self.offset(e));
        if ch == 0 && token.is_empty() {
            return Err(());
        }
        Ok(token)
    }

    fn slice(&self, e: &Engine, from: u32, to: u32) -> Vec<u8> {
        (from..to.min(ln::SIZE))
            .map(|i| self.text_byte(e, i))
            .collect()
    }

    // ---- the script buffer ----

    fn vars_head(&self) -> u32 {
        self.buf + sb::VARS
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::LookupVariable.
    /// `LookupVariable`: a local of the script being compiled, or of the script of the form a
    /// reference names (none for a reference variable or an unscripted form).
    fn lookup_variable(&self, e: &mut Engine, name: &[u8], ref_var: u32) -> u32 {
        let mut head = self.vars_head();
        if ref_var != 0 {
            let form = e.mem.u32(ref_var + 8);
            if form == 0 {
                return 0;
            }
            let script = script_of_form(e, form);
            if script == 0 {
                return 0;
            }
            head = script + sc::VARS;
        }
        variable_by_name(e, head, name)
    }

    /// `ScriptBuffer::GetVariableType`.
    fn variable_type(&self, e: &mut Engine, info: u32, ref_var: u32) -> u8 {
        variable_type(e, self.buf, info, ref_var, self.script)
    }

    // ---- operands ----

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseOperand(Operator *).
    /// `ParseOperand`: the operand at the cursor; `None` after an error (most with a message).
    fn parse_operand(&mut self, e: &mut Engine, cur_op: Option<u8>) -> Option<Tok> {
        let first = self.peek(e);
        let mut want_string_var = false;
        if first == 0 {
            self.message(e, Msg::CantParse, &[]);
            return None;
        }
        if first == b'"' {
            self.advance(e, 1);
            let from = self.offset(e);
            // `strchr(CurText(), '"')`: not held to the length, only to the buffer.
            let mut i = from;
            let end = loop {
                let c = self.text_byte(e, i);
                if c == 0 {
                    break None;
                }
                if c == b'"' {
                    break Some(i);
                }
                i += 1;
            };
            let Some(end) = end else {
                self.message(e, Msg::MismatchedQuotes, &[]);
                return None;
            };
            let mut lit = self.slice(e, from, end);
            self.set_offset(e, end + 1);
            format_string(&mut lit);
            return Some(Tok::Str(lit));
        }
        if first == b'$' {
            want_string_var = true;
            self.advance(e, 1);
        }
        let Ok(mut token) = self.cur_token(e) else {
            self.compile_error(e, b"Failed to read script line; reached out of bounds");
            return None;
        };
        let mut ref_token = token.clone();
        if !want_string_var {
            if ascii_eq_ci(&token, b"_") {
                return Some(Tok::OptionalEmpty);
            }
            if ascii_eq_ci(&token, b"begin") {
                return self.parse_lambda(e);
            }
            let var_type = variable_type_by_name(e, &token);
            if var_type != vt::INVALID {
                self.skip_spaces(e);
                let Ok(name) = self.cur_token(e) else {
                    self.compile_error(e, b"Failed to read variable name; line out of bounds");
                    return None;
                };
                let info = self.create_variable(e, &name, var_type);
                if info == 0 {
                    return None;
                }
                return Some(Tok::var(info, 0, var_type));
            }
        }
        if cur_op.is_some_and(|o| o == op::MEMBER_ACCESS) {
            if token.is_empty() || want_string_var {
                self.message(e, Msg::ExpectedStringLiteral, &[]);
                return None;
            }
            return Some(Tok::Str(token));
        }
        if token.starts_with(b"0x") {
            if let Some(v) = stoul_whole(&token, 16) {
                return Some(Tok::Number(f64::from(v)));
            }
            self.compile_error(e, b"Invalid hexadecimal syntax");
            return None;
        }
        if token.starts_with(b"0b") {
            if let Some(v) = stoul_whole(&token[2..], 2) {
                return Some(Tok::Number(f64::from(v)));
            }
            self.compile_error(e, b"Invalid binary syntax");
            return None;
        }
        let (value, used) = crate::decompile::strtod(&token);
        if used == token.len() {
            return Some(Tok::Number(value));
        }
        // A calling object.
        let mut ref_idx = 0u16;
        let mut has_dot = false;
        if self.peek(e) == b'.' {
            self.advance(e, 1);
            has_dot = true;
            ref_token = std::mem::take(&mut token);
            let Ok(t) = self.cur_token(e) else {
                self.compile_error(e, b"Failed to read token; line out of bounds 2");
                return None;
            };
            token = t;
        }
        if !has_dot {
            let info = self.lookup_variable(e, &token, 0);
            if info != 0 {
                let t = self.variable_type(e, info, 0);
                return Some(Tok::var(info, 0, t));
            }
        }
        let from_stack = cur_op == Some(op::DOT);
        let ref_var = resolve_ref(e, self.buf, &ref_token, self.script);
        if has_dot && ref_var == 0 {
            // The dot operator's syntax instead (chained dots).
            has_dot = false;
            let back = token.len() as u32 + 1;
            let o = self.offset(e);
            self.set_offset(e, o.wrapping_sub(back));
            token = std::mem::take(&mut ref_token);
        } else if ref_var != 0 {
            ref_idx = ref_index(e, self.buf, ref_var) as u16;
        }
        if ref_var != 0 {
            let form = e.mem.u32(ref_var + 8);
            if !has_dot {
                if e.mem.u32(ref_var + 0xc) != 0 {
                    let name = string_data(e, ref_var).unwrap_or_default();
                    let info = variable_by_name(e, self.vars_head(), &name);
                    return Some(Tok::var(info, 0, vt::REF));
                }
                if form != 0 && form_type(e, form) == FORM_GLOBAL {
                    return Some(Tok::Global { idx: ref_idx });
                }
                return Some(Tok::Ref {
                    ref_var,
                    idx: ref_idx,
                });
            }
            if form != 0 && !is_reference(e, form) && form_type(e, form) != FORM_QUEST {
                self.message(e, Msg::InvalidDotSyntax, &[]);
                return None;
            }
        }
        if !want_string_var {
            if let Some(opcode) = command_by_name(e, &token) {
                let record = record_of(e, opcode);
                let quest = e.mem.u16(self.buf + sb::TYPE) == QUEST_SCRIPT;
                if quest && e.mem.u16(record + 0x10) != 0 && ref_var == 0 && !from_stack {
                    let name = guest_text(e, e.mem.u32(record));
                    self.message(e, Msg::RefRequired, &[A::S(&name)]);
                    return None;
                }
                if ref_var != 0 {
                    let form = e.mem.u32(ref_var + 8);
                    if form != 0 && !is_reference(e, form) {
                        return None;
                    }
                }
                return Some(Tok::Command {
                    record,
                    idx: ref_idx,
                    from_stack: false,
                });
            }
        }
        let info = self.lookup_variable(e, &token, ref_var);
        if info == 0 && has_dot {
            self.message(e, Msg::CantFindVariable, &[A::S(&token)]);
            return None;
        }
        if info != 0 {
            let t = self.variable_type(e, info, ref_var);
            if want_string_var && t != vt::STRING {
                self.message(e, Msg::ExpectedStringVariable, &[]);
                return None;
            }
            return Some(Tok::var(info, ref_idx, t));
        }
        if want_string_var {
            self.message(e, Msg::ExpectedStringVariable, &[]);
            return None;
        }
        if ref_var != 0 {
            self.message(e, Msg::InvalidDotSyntax, &[]);
            return None;
        }
        if cur_op != Some(op::MEMBER_ACCESS) {
            self.message(e, Msg::UnquotedString, &[A::S(&token)]);
        }
        format_string(&mut token);
        Some(Tok::Str(token))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::CreateVariable, CreateVariable.
    /// `CreateVariable`: a variable declared inside an expression (`int i := 0`), or the one
    /// already declared with that name (its type checked); 0 after an error.
    fn create_variable(&self, e: &mut Engine, name: &[u8], var_type: u8) -> u32 {
        create_variable(e, self.script, self.buf, name, var_type, self.line)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::PeekOperand.
    /// `PeekOperand`: the operand at the cursor (inside any parentheses), the cursor left where
    /// it was, and how much text it takes.
    fn peek_operand(&mut self, e: &mut Engine) -> (Option<Tok>, u32) {
        let mut read = 0;
        let start = self.offset(e);
        self.skip_spaces(e);
        self.save_line(e);
        while self.text_byte(e, self.offset(e)) == b'(' {
            self.advance(e, 1);
            let Some(end) = self.bracket_end(e, LEFT_PAREN) else {
                return (None, read);
            };
            self.set_text_byte(e, end, 0);
            if read == 0 {
                read = end.wrapping_add(1).wrapping_sub(start);
            }
        }
        if !self.handle_macros(e) {
            return (None, read);
        }
        let operand = self.parse_operand(e, None);
        if read == 0 {
            read = self.offset(e).wrapping_sub(start);
        }
        self.restore_line(e);
        self.set_offset(e, start);
        (operand, read)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseLambda.
    /// `ParseLambda`: `begin ... end` (nested blocks counted) compiled as a script of its own
    /// that shares this script's variables and references; its token is the compiled data.
    fn parse_lambda(&mut self, e: &mut Engine) -> Option<Tok> {
        let begin_at = self.offset(e).wrapping_sub(5);
        let mut nest = 1;
        while nest != 0 {
            let Ok(token) = self.cur_token(e) else {
                self.compile_error(e, b"Lambda function syntax error");
                return None;
            };
            if token.is_empty() {
                self.advance(e, 1);
                continue;
            }
            if ascii_eq_ci(&token, b"begin") {
                nest += 1;
            } else if ascii_eq_ci(&token, b"end") {
                nest -= 1;
            }
        }
        // The text from `begin` to the cursor and the byte after it (xNVSE copies that byte too
        // and leaves the copy unterminated; the port ends it with a NUL).
        let end = self.offset(e);
        let text_len = end.wrapping_sub(begin_at).wrapping_add(1);
        let text: Vec<u8> = (0..text_len.min(ln::SIZE))
            .map(|i| self.text_byte(e, begin_at.wrapping_add(i)))
            .collect();
        let lambda_text = heap_text(e, &text);
        compile_lambda(e, self, begin_at, lambda_text)
    }

    // ---- expressions ----

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseSubExpression.
    /// `ParseSubExpression`: an expression of `len` bytes from the cursor, written in postfix
    /// order; its type, `INVALID` after an error, `EMPTY` for nothing.
    fn sub_expression(&mut self, e: &mut Engine, len: u32) -> Code {
        self.save_line(e);
        let mut ops: Vec<u8> = Vec::new();
        let mut operands: Vec<Code> = Vec::new();
        let end = self.offset(e).wrapping_add(len);
        let mut last_was_operand = false;
        while self.offset(e) < end {
            let ch = self.peek(e);
            if ch == 0 {
                break;
            }
            if is_space(ch) {
                self.advance(e, 1);
                continue;
            }
            let operand_type;
            if !self.handle_macros(e) {
                return IV;
            }
            if let Some(o) = self.parse_operator(e, last_was_operand, true) {
                if is_open_bracket(o) {
                    if operand_count(o) != 0 {
                        // The array subscript.
                        while ops.last().is_some_and(|&t| precedes(t, o)) {
                            self.pop_operator(e, &mut ops, &mut operands);
                        }
                        ops.push(o);
                    }
                    let Some(close) = self.bracket_end(e, o) else {
                        return IV;
                    };
                    self.save_line(e);
                    self.set_text_byte(e, close, 0);
                    let inner = close.wrapping_sub(self.offset(e));
                    operand_type = self.sub_expression(e, inner);
                    self.restore_line(e);
                    self.set_offset(e, close.wrapping_add(1));
                    last_was_operand = true;
                } else if is_closing_bracket(o) {
                    self.message(e, Msg::MismatchedBrackets, &[]);
                    return IV;
                } else {
                    while ops.last().is_some_and(|&t| precedes(t, o)) {
                        self.pop_operator(e, &mut ops, &mut operands);
                    }
                    ops.push(o);
                    last_was_operand = false;
                    continue;
                }
            } else if last_was_operand || self.parse_operator(e, !last_was_operand, false).is_some()
            {
                // An argument's delimiter.
                break;
            } else {
                let Some(mut operand) = self.parse_operand(e, ops.last().copied()) else {
                    return IV;
                };
                if operand.code() == IV {
                    return IV;
                }
                if let Tok::Command {
                    idx, from_stack, ..
                } = &mut operand
                {
                    if ops.last() == Some(&op::DOT) {
                        if *idx != 0 {
                            self.compile_error(e, b"Parsing failure when interpreting dot syntax (token has ref index while dot operator is on operator stack)");
                            return IV;
                        }
                        // The calling reference is the stack's top at run time.
                        *from_stack = true;
                    }
                }
                if !operand.write(e, self.line) {
                    self.compile_error(e, b"Failed to compile token.");
                    return IV;
                }
                let mut t = operand.code();
                if let Tok::Command {
                    record, from_stack, ..
                } = operand
                {
                    let opcode = e.mem.u32(record + 8);
                    t = match return_type(e, opcode) {
                        Returns::String => tt::STRING,
                        Returns::Array => tt::ARRAY,
                        Returns::Form => tt::FORM,
                        Returns::Default => tt::NUMBER,
                        Returns::ArrayIndex | Returns::Ambiguous => t,
                    };
                    if from_stack {
                        state(e).last_command_type = Some(t);
                        // The dot's right-hand rule takes a command.
                        t = tt::COMMAND;
                    }
                    state(e).depth += 1;
                    let parsed = self.parse_function_call(e, record);
                    state(e).depth -= 1;
                    if !parsed {
                        self.message(e, Msg::CantParse, &[]);
                        return IV;
                    }
                }
                operand_type = t;
                last_was_operand = true;
            }
            if operand_type == IV {
                self.message(e, Msg::CantParse, &[]);
                return IV;
            }
            operands.push(operand_type);
        }
        while !ops.is_empty() {
            if self.pop_operator(e, &mut ops, &mut operands) == IV {
                return IV;
            }
        }
        self.restore_line(e);
        if operands.len() > 1 {
            self.message(e, Msg::TooManyOperands, &[]);
            return IV;
        }
        operands.last().copied().unwrap_or(tt::EMPTY)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::PopOperator.
    /// `PopOperator`: the top operator applied to its operands' types and written.
    fn pop_operator(&self, e: &mut Engine, ops: &mut Vec<u8>, operands: &mut Vec<Code>) -> Code {
        let Some(top) = ops.pop() else {
            return IV;
        };
        let count = operand_count(top);
        if operands.len() < count {
            self.message(e, Msg::TooManyOperators, &[]);
            return IV;
        }
        let (lhs, rhs) = match count {
            2 => {
                let rhs = operands.pop().unwrap_or(IV);
                (operands.pop().unwrap_or(IV), rhs)
            }
            1 => (operands.pop().unwrap_or(IV), IV),
            _ => {
                self.message(e, Msg::CantParse, &[]);
                return IV;
            }
        };
        let mut result = result_type(top, lhs, rhs);
        if result == IV {
            self.message(e, Msg::InvalidOperands, &[A::S(symbol(top))]);
            return IV;
        }
        if top == op::DOT {
            let Some(last) = state(e).last_command_type else {
                self.compile_error(
                    e,
                    b"Failure parsing chained dot syntax: the command return type was not saved.",
                );
                return IV;
            };
            result = last;
        }
        operands.push(result);
        write(e, self.line, &[top]);
        result
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseArgument.
    /// `ParseArgument`: an expression up to `end`, after its length word.
    fn parse_argument(&mut self, e: &mut Engine, end: u32) -> Code {
        let start = data_len(e, self.line);
        set_data_len(e, self.line, start.wrapping_add(2));
        let len = end.wrapping_sub(self.offset(e));
        let t = self.sub_expression(e, len);
        let total = data_len(e, self.line).wrapping_sub(start);
        patch_u16(e, self.line, start, total as u16);
        t
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseFunctionCall; Hooks_Script.cpp: ParseNestedFunction.
    /// `ParseFunctionCall`: a command inside an expression, parsed by its own parse routine on
    /// the text after it as if it were the only command on the line (the game's end-of-line
    /// check off meanwhile), its arguments' length before them.
    fn parse_function_call(&self, e: &mut Engine, record: u32) -> bool {
        let line = self.line;
        let old_offset = self.offset(e);
        let old_opcode = e.mem.u32(line + ln::OPCODE);
        let old_ref = e.mem.u16(line + ln::CALLING_REF);
        let old_data = data_len(e, line) as u16;
        set_data_len(e, line, data_len(e, line).wrapping_add(2));
        let old_len = e.mem.u32(line + ln::TEXT_LEN);
        let old_text = e.mem.bytes(line + ln::TEXT, ln::SIZE);
        let mut rigged = vec![0u8; ln::SIZE as usize];
        let from = (old_offset as usize).min(rigged.len());
        let n = rigged.len() - from;
        rigged[..n].copy_from_slice(&old_text[from..]);
        e.mem.write(line + ln::TEXT, &rigged);
        e.mem.set_u32(line + ln::OPCODE, e.mem.u32(record + 8));
        e.mem.set_u32(line + ln::CALLING_REF, 0);
        e.mem.set_u32(line + ln::OFFSET, 0);
        let len = text_len(e, line, 0);
        e.mem.set_u32(line + ln::TEXT_LEN, len);
        let parsed = parse_nested(e, record, line, self.buf);
        e.mem.set_u32(line + ln::CALLING_REF, u32::from(old_ref));
        let o = e.mem.u32(line + ln::OFFSET);
        e.mem.set_u32(line + ln::OFFSET, o.wrapping_add(old_offset));
        e.mem.set_u32(line + ln::TEXT_LEN, old_len);
        let total = data_len(e, line).wrapping_sub(u32::from(old_data));
        patch_u16(e, line, u32::from(old_data), total as u16);
        e.mem.set_u32(line + ln::OPCODE, old_opcode);
        e.mem.write(line + ln::TEXT, &old_text);
        parsed
    }

    // ---- arguments ----

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseArgs.
    /// `ParseArgs`: the arguments of a command, each an expression checked against its
    /// parameter (xNVSE's types: every run-time caller passes them), after a count byte; the
    /// cursor is left past the arguments (the whole line's parse: no run-time caller asks for
    /// less).
    pub(crate) fn parse_args(&mut self, e: &mut Engine, params: &Params) -> bool {
        let count_at = data_len(e, self.line);
        set_data_len(e, self.line, count_at.wrapping_add(1));
        let num_params = params.count();
        let mut args_end = self.len;
        if num_params > 1 {
            // Comparisons and `&&`/`||` outside parentheses and strings end the arguments
            // (from the start of the text, as xNVSE scans it).
            let mut depth = 0i32;
            let mut in_string = false;
            let mut i = 0u32;
            loop {
                let ch = self.peek_at(e, i);
                i += 1;
                if ch == 0 || i >= args_end {
                    break;
                }
                if ch == b'(' && !in_string {
                    depth += 1;
                } else if ch == b')' && !in_string {
                    depth -= 1;
                } else if ch == b'"' {
                    in_string = !in_string;
                } else if depth == 0 && !in_string {
                    let old = self.offset(e);
                    self.set_offset(e, i);
                    let found = self.parse_operator(e, true, false);
                    self.set_offset(e, old);
                    if found.is_some_and(|o| {
                        (op::EQUALS..=op::LESS_OR_EQUAL).contains(&o)
                            || o == op::LOGICAL_AND
                            || o == op::LOGICAL_OR
                    }) {
                        args_end = i - 1;
                        while is_space(self.peek_at(e, args_end)) {
                            args_end = args_end.wrapping_sub(1);
                        }
                        break;
                    }
                } else if depth == -1 {
                    break;
                }
            }
        }
        self.skip_peeked_spaces(e);
        let mut start = self.offset(e);
        let mut end_offset = args_end;
        if !self.handle_macros(e) {
            return false;
        }
        if self.peek(e) == b'{' {
            // Parsing held to the text inside the braces.
            loop {
                let ch = self.peek(e);
                if ch == 0 {
                    break;
                }
                if ch == b'{' {
                    start += 1;
                    self.advance(e, 1);
                    let Some(close) = self.bracket_end(e, LEFT_BRACE) else {
                        return false;
                    };
                    self.set_offset(e, close);
                } else if ch == b'}' {
                    end_offset = self.offset(e);
                    args_end = end_offset.wrapping_sub(1);
                    break;
                } else {
                    self.advance(e, 1);
                }
            }
        }
        self.set_offset(e, start);
        self.skip_peeked_spaces(e);
        while self.args_parsed < num_params && self.offset(e) < args_end {
            let t = self.parse_argument(e, args_end);
            if t == IV {
                return false;
            }
            if t == tt::EMPTY {
                break;
            }
            let p = params.get(e, self.args_parsed);
            if !validate_arg_type(p.kind, t) {
                let text = format(
                    "Invalid expression for parameter %d. Expected %s.",
                    &[A::D(i64::from(self.args_parsed) + 1), A::S(&p.name)],
                    0x200,
                );
                compiler_error(e, self.line, &text);
                return false;
            }
            self.args_parsed += 1;
        }
        if self.offset(e) < args_end && state(e).depth == 1 {
            self.message(e, Msg::TooManyArgs, &[]);
            return false;
        }
        self.set_offset(e, end_offset.wrapping_add(1));
        let mut expected = 0;
        while expected < num_params && !params.get(e, expected).optional {
            expected += 1;
        }
        if expected > self.args_parsed {
            let missing = params.get(e, self.args_parsed);
            self.message(
                e,
                Msg::MissingParam,
                &[A::S(&missing.name), A::D(i64::from(self.args_parsed) + 1)],
            );
            return false;
        }
        patch_u8(e, self.line, count_at, self.args_parsed as u8);
        true
    }

    /// Passes spaces at the cursor (`isspace(Peek(Offset()))`).
    fn skip_peeked_spaces(&self, e: &mut Engine) {
        loop {
            let ch = self.peek(e);
            if ch == 0 || !is_space(ch) {
                break;
            }
            self.advance(e, 1);
        }
    }

    // ---- user functions ----

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseUserFunctionParameters.
    /// `ParseUserFunctionParameters`: a function script's parameters from its text's
    /// `begin function {...}` line.
    fn user_function_parameters(
        &self,
        e: &mut Engine,
        text: &[u8],
        vars_head: u32,
        script: u32,
    ) -> Option<Vec<(u32, u8)>> {
        if text.starts_with(b"name") {
            // The new script language's function header (`Compiler::PreProcessNVSEScript`):
            // that compiler is not built (plan step 3.8, deferred), so it does not parse here.
            return None;
        }
        let names = crate::compile_hooks::user_function_param_names(text);
        let params = names.and_then(|n| {
            crate::compile_hooks::user_function_params(e, self, &n, vars_head, text, script)
        });
        if params.is_none() {
            self.message(e, Msg::UserFunctionParamsUndefined, &[]);
        }
        params
    }

    /// `CreateVariable` for the user function parameter code.
    pub(crate) fn create_variable_named(&self, e: &mut Engine, name: &[u8], var_type: u8) -> u32 {
        self.create_variable(e, name, var_type)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseUserFunctionCall.
    /// `ParseUserFunctionCall` (`Call`): the version byte, the function (a script's name or a
    /// lambda as its length and token, or any expression giving a form), then its arguments.
    fn user_function_call(&mut self, e: &mut Engine) -> bool {
        write(e, self.line, &[USER_FUNCTION_VERSION]);
        let param_len = text_len(e, self.line, 0);
        while is_space(self.peek(e)) {
            self.advance(e, 1);
            if self.offset(e) >= param_len {
                self.message(e, Msg::CantParse, &[]);
                return false;
            }
        }
        let (operand, read) = self.peek_operand(e);
        let form = operand.as_ref().map_or(0, |t| t.clone().form(e));
        let function = if form != 0 && form_type(e, form) == 0x11 {
            form
        } else {
            0
        };
        if function != 0 {
            let start = data_len(e, self.line);
            set_data_len(e, self.line, start.wrapping_add(2));
            if let Some(t) = &operand {
                t.write(e, self.line);
            }
            self.advance(e, read);
            let total = data_len(e, self.line).wrapping_sub(start);
            patch_u16(e, self.line, start, total as u16);
        } else {
            let t = self.parse_argument(e, self.len);
            if !convert(t, tt::FORM) {
                self.message(e, Msg::ExpectedUserFunction, &[]);
                return false;
            }
        }
        while (is_space(self.peek(e)) || self.peek(e) == b',') && self.offset(e) < param_len {
            self.advance(e, 1);
        }
        if function != 0 && e.mem.u32(function + sc::TEXT) != 0 {
            let mut text = guest_text(e, e.mem.u32(function + sc::TEXT));
            let mut vars = function + sc::VARS;
            let mut script = function;
            // xNVSE compares the function's editor id, which the run-time build does not keep
            // (null), with this script's name: the same only when this one has no name.
            if string_data(e, self.buf + sb::NAME).is_none() {
                text = guest_text(e, e.mem.u32(self.buf + sb::TEXT));
                vars = self.vars_head();
                script = self.script;
            }
            let Some(params) = self.user_function_parameters(e, &text, vars, script) else {
                return false;
            };
            let dynamic = dynamic_params(&params);
            return with_parser(e, self.buf, self.line, |e, p| {
                p.parse_args(e, &Params::Host(dynamic))
            });
        }
        let params = Params::Host(default_user_function_params());
        self.parse_args(e, &params)
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
    // ExpressionParser::ParseUserFunctionDefinition.
    /// `ParseUserFunctionDefinition` (`Begin Function {...}`): the version, the parameters
    /// (variable index and type each), and no local arrays (xNVSE 6 collects them itself).
    fn user_function_definition(&self, e: &mut Engine) -> bool {
        write(e, self.line, &[USER_FUNCTION_VERSION]);
        let text = guest_text(e, e.mem.u32(self.buf + sb::TEXT));
        let Some(params) = self.user_function_parameters(e, &text, self.vars_head(), self.script)
        else {
            return false;
        };
        write(e, self.line, &[params.len() as u8]);
        for (idx, var_type) in &params {
            write(e, self.line, &(*idx as u16).to_le_bytes());
            write(e, self.line, &[*var_type]);
        }
        write(e, self.line, &[0]);
        true
    }
}

/// `DynamicParamInfo`: a function's parameters by type (`kDynamicParams`), at most 15.
fn dynamic_params(params: &[(u32, u8)]) -> Vec<Param> {
    params
        .iter()
        .take(MAX_UDF_PARAMS)
        .map(|&(_, t)| {
            let (name, kind): (&[u8], u32) = match t {
                vt::FLOAT => (b"float", NVSE_NUMBER),
                vt::INTEGER => (b"integer", NVSE_NUMBER),
                vt::STRING => (b"string", NVSE_STRING),
                vt::ARRAY => (b"array", NVSE_ARRAY),
                _ => (b"object", NVSE_FORM),
            };
            Param {
                name: name.to_vec(),
                kind,
                optional: false,
            }
        })
        .collect()
}

/// The one-line lambda macro's rewrite of a text, if it applies: `^\{([^{}]*)\}\s*=>\s*(.*)`
/// (`std::regex`, ECMAScript: `.` stops at a line break).
fn one_line_lambda(text: &[u8]) -> Option<Vec<u8>> {
    if text.first() != Some(&b'{') {
        return None;
    }
    let close = text[1..].iter().position(|&c| c == b'{' || c == b'}')? + 1;
    if text[close] != b'}' {
        return None;
    }
    let args = &text[1..close];
    let mut i = close + 1;
    while i < text.len() && is_space(text[i]) {
        i += 1;
    }
    if !text[i..].starts_with(b"=>") {
        return None;
    }
    i += 2;
    while i < text.len() && is_space(text[i]) {
        i += 1;
    }
    let body_end = text[i..]
        .iter()
        .position(|&c| c == b'\n' || c == b'\r')
        .map_or(text.len(), |p| i + p);
    let mut out = b"begin function {".to_vec();
    out.extend_from_slice(args);
    out.extend_from_slice(b"}\r\nSetFunctionValue ");
    out.extend_from_slice(&text[i..body_end]);
    out.extend_from_slice(b"\r\nend");
    Some(out)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp: VariableTypeNameToType.
/// `VariableTypeNameToType` (`StrCompare`, the game's case table).
pub(crate) fn variable_type_by_name(e: &mut Engine, name: &[u8]) -> u8 {
    let is = |e: &mut Engine, n: &[u8]| str_equal(e, Some(name), Some(n));
    if is(e, b"string_var") {
        vt::STRING
    } else if is(e, b"array_var") {
        vt::ARRAY
    } else if is(e, b"float") {
        vt::FLOAT
    } else if is(e, b"long") || is(e, b"int") || is(e, b"short") {
        vt::INTEGER
    } else if is(e, b"ref") || is(e, b"reference") {
        vt::REF
    } else {
        vt::INVALID
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp: ScriptBuffer::ResolveRef.
/// `ScriptBuffer::ResolveRef`: the reference list's entry for a name (made and appended when
/// new): a local reference variable, the player, or a form by its editor id (a reference only
/// when persistent); 0 for none.
pub(crate) fn resolve_ref(e: &mut Engine, buf: u32, name: &[u8], script: u32) -> u32 {
    let mut found = 0;
    for r in list_items(e, buf + sb::REFS) {
        let n = string_data(e, r);
        if n.as_deref().is_some_and(|n| ascii_eq_ci(n, name)) {
            found = r;
            break;
        }
    }
    let mut made = found;
    let info = variable_by_name(e, buf + sb::VARS, name);
    if info != 0 && variable_type(e, buf, info, 0, script) == vt::REF {
        if made == 0 {
            made = new_ref_variable(e);
        }
        let idx = e.mem.u32(info);
        e.mem.set_u32(made + 0xc, idx);
    } else {
        let form = if ascii_eq_ci(name, b"player") {
            e.call(LOOKUP_FORM, &args![0x14u32]).u32()
        } else {
            form_by_editor_id(e, name)
        };
        if form != 0 {
            let refr = cast(e, form, RTTI_TES_OBJECT_REFR);
            if refr != 0 && e.mem.u32(refr + 8) & PERSISTENT == 0 {
                return 0;
            }
            if made == 0 {
                made = new_ref_variable(e);
            }
            e.mem.set_u32(made + 8, form);
        }
    }
    if found == 0 && made != 0 {
        string_set(e, made, name);
        list_append(e, buf + sb::REFS, made);
        let n = e.mem.u32(buf + sb::NUM_REFS);
        e.mem.set_u32(buf + sb::NUM_REFS, n.wrapping_add(1));
    }
    made
}

/// `New<Script::RefVariable>()`: 0x10 zeroed bytes of the heap.
fn new_ref_variable(e: &mut Engine) -> u32 {
    let at = e.call(FORM_HEAP_ALLOCATE, &args![0x10u32]).u32();
    e.mem.write(at, &[0; 0x10]);
    at
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp: ScriptBuffer::GetRefIdx.
/// `GetRefIdx`: the entry's place in the list from 1 (the list's length when it is not in it).
pub(crate) fn ref_index(e: &Engine, buf: u32, ref_var: u32) -> u32 {
    let mut idx = 0;
    let mut node = buf + sb::REFS;
    for _ in 0..0x10_0000 {
        if node == 0 || e.mem.u32(node) == 0 {
            break;
        }
        idx += 1;
        if e.mem.u32(node) == ref_var {
            break;
        }
        node = e.mem.u32(node + 4);
    }
    idx
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp:
// ScriptBuffer::GetVariableType, Script::GetVariableType.
/// `ScriptBuffer::GetVariableType`: a local's declared type, from the text of the script that
/// has it (another script's through a reference's or a quest's form; none for a reference
/// variable).
pub(crate) fn variable_type(e: &mut Engine, buf: u32, info: u32, ref_var: u32, script: u32) -> u8 {
    let mut text = e.mem.u32(buf + sb::TEXT);
    let mut script = script;
    if ref_var != 0 {
        let form = e.mem.u32(ref_var + 8);
        if form == 0 {
            return vt::INVALID;
        }
        let scriptable = match form_type(e, form) {
            FORM_REFR => {
                let refr = cast(e, form, RTTI_TES_OBJECT_REFR);
                let base = crate::commands::base_form(e, refr);
                cast(e, base, RTTI_TES_SCRIPTABLE_FORM)
            }
            FORM_QUEST => cast(e, form, RTTI_TES_SCRIPTABLE_FORM),
            _ => 0,
        };
        let other = if scriptable != 0 {
            e.mem.u32(scriptable + 4)
        } else {
            0
        };
        if other != 0 {
            let other_text = e.mem.u32(other + sc::TEXT);
            if other_text != 0 {
                script = other;
                text = other_text;
            } else {
                // `Script::GetVariableType` without a text: a reference when the reference
                // list names its index, else the variable's own type byte.
                let idx = e.mem.u32(info);
                let named = list_items(e, other + sc::REFS)
                    .into_iter()
                    .any(|r| e.mem.u32(r + 0xc) == idx);
                return if named {
                    vt::REF
                } else {
                    e.mem.u8(info + 0x10)
                };
            }
        }
    }
    let name = string_data(e, info + 0x18).unwrap_or_default();
    let text = opt_text(e, text).unwrap_or_default();
    crate::compile_hooks::declared_variable_type(e, &name, &text, script)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp: CreateVariable,
// ValidateVariable.
/// `CreateVariable`: the variable `name` of the buffer (its type checked against the type the
/// compile saw first), or a new one numbered after the last (or with the script's own index for
/// it), appended to the buffer's variables and named in the reference list if a reference; 0
/// after an error.
pub(crate) fn create_variable(
    e: &mut Engine,
    script: u32,
    buf: u32,
    name: &[u8],
    var_type: u8,
    line: u32,
) -> u32 {
    if script == 0 {
        return 0;
    }
    for info in list_items(e, buf + sb::VARS) {
        let n = string_cstr(e, info + 0x18);
        if ascii_eq_ci(name, &n) {
            if !crate::compile_hooks::validate_variable(e, name, var_type, script) {
                return 0;
            }
            return info;
        }
    }
    if name.is_empty() {
        compiler_error(e, line, b"Got empty string for variable name");
        return 0;
    }
    if form_by_editor_id(e, name) != 0 {
        let mut m = b"Invalid variable name ".to_vec();
        m.extend_from_slice(name);
        m.extend_from_slice(b": Form with that Editor ID already exists.");
        compiler_error(e, line, &m);
        return 0;
    }
    if command_by_name(e, name).is_some() {
        let mut m = b"Invalid variable name ".to_vec();
        m.extend_from_slice(name);
        m.extend_from_slice(b": Command with that name already exists.");
        compiler_error(e, line, &m);
        return 0;
    }
    let info = e.call(FORM_HEAP_ALLOCATE, &args![0x20u32]).u32();
    e.mem.write(info, &[0; 0x20]);
    let existing = variable_by_name(e, script + sc::VARS, name);
    let idx = if existing != 0 {
        e.mem.u32(existing)
    } else {
        let n = e.mem.u32(buf + sb::VAR_COUNT).wrapping_add(1);
        e.mem.set_u32(buf + sb::VAR_COUNT, n);
        n
    };
    e.mem.set_u32(info, idx);
    string_set(e, info + 0x18, name);
    e.mem.set_u8(info + 0x10, var_type);
    list_append(e, buf + sb::VARS, info);
    crate::compile_hooks::save_var_type(e, script, name, var_type);
    resolve_ref(e, buf, name, script);
    info
}

/// `ParseNestedFunction`: a command's parse routine called with the end-of-line check off (by
/// the first caller that turned it off).
fn parse_nested(e: &mut Engine, record: u32, line: u32, buf: u32) -> bool {
    let turned_off = !state(e).eol_check_off;
    if turned_off {
        state(e).eol_check_off = true;
    }
    let parse = e.mem.u32(record + 0x1c);
    let count = u32::from(e.mem.u16(record + 0x12));
    let params = e.mem.u32(record + 0x14);
    // A record with no parse routine (xNVSE would call address 0): the command does not parse.
    let parsed = parse != 0 && e.call(parse, &args![count, params, line, buf]).bool();
    if turned_off {
        state(e).eol_check_off = false;
    }
    parsed
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp:
// ExpressionParser::ParseLambda (its compile).
/// The lambda's compile: a script buffer of its own on the lambda's text (a partial script,
/// named `<script>LambdaAtLine<n>`, its data starting with a fake `scn`), sharing this buffer's
/// variable and reference lists and counts, compiled by the game's compiler with the open
/// block's jump saved and the reference check off; its token, or `None` when it failed.
fn compile_lambda(e: &mut Engine, p: &mut Parser, begin_at: u32, text: u32) -> Option<Tok> {
    let lambda_buf = e.call(FORM_HEAP_ALLOCATE, &args![sb::SIZE]).u32();
    e.call(BUFFER_CONSTRUCTOR, &args![lambda_buf]);
    let memory = e.call(FORM_HEAP_ALLOCATE, &args![sc::SIZE]).u32();
    let lambda = e.call(SCRIPT_CONSTRUCTOR, &args![memory]).u32();
    let var_count = e.mem.u32(p.buf + sb::VAR_COUNT);
    let num_refs = e.mem.u32(p.buf + sb::NUM_REFS);
    e.mem.set_u8(lambda_buf + sb::PARTIAL, 1);
    e.mem.set_u32(lambda + sc::VAR_COUNT, var_count);
    e.mem.set_u32(lambda + sc::UNUSED_VARS, var_count);
    e.mem.set_u32(lambda + sc::NUM_REFS, num_refs);
    e.mem.set_u32(lambda_buf + sb::VAR_COUNT, var_count);
    e.mem.set_u32(lambda_buf + sb::UNUSED_VARS, var_count);
    e.mem.set_u32(lambda_buf + sb::NUM_REFS, num_refs);
    // Line breaks before the lambda give its line (one less unless the macro made it).
    let breaks = (0..begin_at.min(ln::SIZE))
        .filter(|&i| p.text_byte(e, i) == b'\n')
        .count() as i32;
    let one_line = p.macros & MACRO_ONE_LINE_LAMBDA != 0;
    let breaks = if one_line { breaks } else { breaks - 1 };
    let number = (e.mem.u32(p.line + ln::NUMBER) as i32).wrapping_add(breaks);
    e.mem.set_u32(lambda_buf + sb::CUR_LINE, number as u32);
    let mut name = string_cstr(e, p.buf + sb::NAME);
    name.extend_from_slice(format!("LambdaAtLine{number}").as_bytes());
    string_set(e, lambda_buf + sb::NAME, &name);
    if one_line {
        e.mem
            .set_u32(lambda_buf + sb::CUR_LINE, (number - 2) as u32);
        p.macros &= !MACRO_ONE_LINE_LAMBDA;
    }
    // A fake `scn` (1D 00 00 00) so the data starts as other scripts' do.
    let data = e.mem.u32(lambda_buf + sb::DATA);
    e.mem.set_u8(data, 0x1d);
    e.mem.set_u32(lambda_buf + sb::DATA_OFFSET, 4);
    e.mem.set_u32(lambda_buf + sb::TEXT, text);
    e.mem.set_u32(lambda + sc::TEXT, text);
    // The lists' head nodes copied, so the lambda's compile adds onto this buffer's lists.
    let copy8 = |e: &mut Engine, from: u32, to: u32| {
        let b = e.mem.bytes(from, 8);
        e.mem.write(to, &b);
    };
    copy8(e, p.buf + sb::REFS, lambda_buf + sb::REFS);
    copy8(e, p.buf + sb::VARS, lambda_buf + sb::VARS);
    let saved_jump = e.mem.u32(BLOCK_JUMP);
    e.mem.set_u32(BLOCK_JUMP, 0);
    let parent = p.script;
    state(e).lambda_parents.push((lambda, parent));
    state(e).skip_ref_check = true;
    let manager = e.call(CONSOLE_MANAGER, &args![1u32]).u32();
    let compiled = if manager == 0 {
        false
    } else {
        let compiler = e.mem.u32(manager);
        e.call(SCRIPT_COMPILE, &args![compiler, lambda, lambda_buf])
            .bool()
    };
    state(e).skip_ref_check = false;
    e.mem.set_u32(BLOCK_JUMP, saved_jump);
    copy8(e, lambda_buf + sb::REFS, p.buf + sb::REFS);
    copy8(e, lambda_buf + sb::VARS, p.buf + sb::VARS);
    e.mem.write(lambda_buf + sb::REFS, &[0; 8]);
    e.mem.write(lambda_buf + sb::VARS, &[0; 8]);
    e.mem.set_u32(lambda_buf + sb::TEXT, 0);
    for at in [sb::NUM_REFS, sb::VAR_COUNT, sb::UNUSED_VARS] {
        let v = e.mem.u32(lambda_buf + at);
        e.mem.set_u32(p.buf + at, v);
    }
    // A failed lambda's script goes first, then the buffer (their `unique_ptr`s' order).
    let token = if compiled {
        Some(Tok::Lambda(lambda))
    } else {
        state(e).lambda_parents.retain(|&(l, _)| l != lambda);
        e.mem.write(lambda + sc::REFS, &[0; 8]);
        e.mem.write(lambda + sc::VARS, &[0; 8]);
        delete_script(e, lambda);
        None
    };
    e.call(BUFFER_DESTRUCTOR, &args![lambda_buf]);
    e.call(FORM_HEAP_FREE, &args![lambda_buf]);
    token
}

/// The parent of a lambda made while compiling (`GetLambdaParentScript`), 0 for none.
pub(crate) fn lambda_parent(e: &mut Engine, lambda: u32) -> u32 {
    state(e)
        .lambda_parents
        .iter()
        .find(|&&(l, _)| l == lambda)
        .map_or(0, |&(_, p)| p)
}

// ---- the parse routines ----

/// The four words a parse routine gets: the parameter count, the parameters, the line, the
/// script buffer.
fn parse_words(w: &[u32]) -> (Params, u32, u32) {
    let get = |i: usize| w.get(i).copied().unwrap_or(0);
    (
        Params::Guest {
            at: get(1),
            count: get(0),
        },
        get(2),
        get(3),
    )
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp: Cmd_Expression_Parse.
/// `Cmd_Expression_Parse`.
pub fn expression_parse(e: &mut Engine, w: &[u32]) -> Ret {
    let (params, line, buf) = parse_words(w);
    let ok = with_parser(e, buf, line, |e, p| p.parse_args(e, &params));
    crate::handled(ok)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp: Cmd_Let_Parse.
/// `Cmd_Let_Parse`: the arguments, whose last byte must be an assignment operator (no message
/// at run time).
pub fn let_parse(e: &mut Engine, w: &[u32]) -> Ret {
    let (params, line, buf) = parse_words(w);
    if !with_parser(e, buf, line, |e, p| p.parse_args(e, &params)) {
        return crate::handled(false);
    }
    let last_at = data_len(e, line).wrapping_sub(1);
    let last = if last_at < ln::SIZE {
        e.mem.u8(line + ln::DATA + last_at)
    } else {
        0xff
    };
    let assignment = matches!(
        last,
        op::ASSIGNMENT
            | op::PLUS_EQUALS
            | op::TIMES_EQUALS
            | op::DIVIDED_EQUALS
            | op::EXPONENT_EQUALS
            | op::MINUS_EQUALS
            | op::BITWISE_OR_EQUALS
            | op::BITWISE_AND_EQUALS
            | op::MODULO_EQUALS
    );
    crate::handled(assignment)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_BeginLoop_Parse; Hooks_Script.cpp: RegisterLoopStart.
/// `Cmd_BeginLoop_Parse` (`while`, `ForEach`): room for the jump past the loop (where the
/// script's data will hold it is remembered for its `loop`), then the condition.
///
/// A deliberate difference: xNVSE remembers the jump word's address, which the script data's
/// growth (xNVSE's own size hook, 005B0F68) frees while the loop is open, so its `loop` writes
/// into freed memory and the copy keeps a jump of 0. The port remembers the word's offset in the
/// data and writes at the data's current place: the bytes xNVSE meant to write.
pub fn begin_loop_parse(e: &mut Engine, w: &[u32]) -> Ret {
    let (params, line, buf) = parse_words(w);
    let at = e
        .mem
        .u32(buf + sb::DATA_OFFSET)
        .wrapping_add(data_len(e, line))
        .wrapping_add(4);
    state(e).loop_starts.push(at);
    set_data_len(e, line, data_len(e, line).wrapping_add(4));
    let ok = with_parser(e, buf, line, |e, p| p.parse_args(e, &params));
    crate::handled(ok)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp: Cmd_Loop_Parse;
// Hooks_Script.cpp: HandleLoopEnd.
/// `Cmd_Loop_Parse`: the open loop's jump word (at its offset in the script's data as it is
/// now; see [`begin_loop_parse`]) gets the offset of the instruction after this `loop`; false
/// with no open loop (no message at run time).
pub fn loop_parse(e: &mut Engine, w: &[u32]) -> Ret {
    let (_, line, buf) = parse_words(w);
    let offset = e
        .mem
        .u32(buf + sb::DATA_OFFSET)
        .wrapping_add(data_len(e, line))
        .wrapping_add(4);
    let Some(at) = state(e).loop_starts.pop() else {
        return crate::handled(false);
    };
    let data = e.mem.u32(buf + sb::DATA);
    e.mem.set_u32(data.wrapping_add(at), offset);
    crate::handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp: Cmd_Null_Parse.
/// `Cmd_Null_Parse`: no arguments, no data.
pub fn null_parse(_: &mut Engine, _: &[u32]) -> Ret {
    crate::handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp:
// Cmd_Function_Parse.
/// `Cmd_Function_Parse`.
pub fn function_parse(e: &mut Engine, w: &[u32]) -> Ret {
    let (_, line, buf) = parse_words(w);
    let ok = with_parser(e, buf, line, |e, p| p.user_function_definition(e));
    crate::handled(ok)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Scripting.cpp: Cmd_Call_Parse.
/// `Cmd_Call_Parse`.
pub fn call_parse(e: &mut Engine, w: &[u32]) -> Ret {
    let (_, line, buf) = parse_words(w);
    let ok = with_parser(e, buf, line, |e, p| p.user_function_call(e));
    crate::handled(ok)
}

#[cfg(test)]
#[path = "compiler_tests.rs"]
pub(crate) mod tests;
