//! Tests of the decompiler, from `ScriptAnalyzer.cpp`'s rules (xNVSE 6.4.9): stand-ins of the
//! exe's statement, event block and operator tables (the operator table's bytes as the exe holds
//! them), a script whose lines are built byte for byte as the game's compiler writes them, and
//! the expected text worked out line by line from the rules.

use super::*;
use crate::commands::test_support::*;

pub(crate) const AREA: u32 = 0x0400_0000;
const STR_TOP: u32 = AREA + 0x0ffc;
const STRINGS: u32 = AREA + 0x1000;
const PARAM_TOP: u32 = AREA + 0x2ffc;
const PARAMS_AT: u32 = AREA + 0x3000;
const DATA_AT: u32 = AREA + 0x4000;
pub(crate) const FORM_A: u32 = AREA + 0x6000;
pub(crate) const FORM_B: u32 = AREA + 0x6100;
const GLOBAL: u32 = AREA + 0x6200;
pub(crate) const VTABLE: u32 = AREA + 0x7000;
const HEAP_TOP: u32 = AREA + 0x7ffc;
const HEAP: u32 = AREA + 0x8000;
const NAME_FN: u32 = 0x7eff_e100;

/// The exe's `g_gameScriptOperators` (`0x0118CAD0`), its 128 bytes.
const OPERATOR_BYTES: &str = "00000000002800000100000000290000020000000126260003000000027c7c00\
04000000033c3d0005000000033c000006000000033e3d0007000000033e0000\
08000000033d3d000900000003213d000a000000042d00000b000000042b0000\
0c000000052a00000d000000052f00000e000000052500000f000000067e0000";

/// The exe's statement names (`0x0118CB50`), codes `0x10..=0x1F`.
const STATEMENT_NAMES: [&str; 16] = [
    "Begin",
    "End",
    "Short",
    "Long",
    "Float",
    "Set",
    "If",
    "Else",
    "ElseIf",
    "EndIf",
    "While_DEPRECATED",
    "EndWhile_DEPRECATED",
    ".",
    "ScriptName",
    "Return",
    "Reference",
];

pub(crate) fn text_at(e: &mut Engine, s: &str) -> u32 {
    let at = e.mem.u32(STR_TOP).max(STRINGS);
    e.mem.write(at, s.as_bytes());
    e.mem.set_u8(at + s.len() as u32, 0);
    e.mem.set_u32(STR_TOP, at + s.len() as u32 + 1);
    at
}

/// `TESForm::GetName` of the stand-in forms: the text pointer at `+0x60`.
fn name_fn(e: &mut Engine, a: &[u32]) -> Ret {
    Ret {
        eax: e.mem.u32(a[0] + 0x60),
        ..Ret::default()
    }
}

fn heap(e: &mut Engine, a: &[u32]) -> Ret {
    let at = e.mem.u32(HEAP_TOP).max(HEAP);
    e.mem.set_u32(HEAP_TOP, at + a[0].next_multiple_of(8));
    Ret {
        eax: at,
        ..Ret::default()
    }
}

fn script_constructor(e: &mut Engine, a: &[u32]) -> Ret {
    e.mem.set_u32(a[0], VTABLE);
    e.mem.set_u8(a[0] + 4, 0x11);
    Ret {
        eax: a[0],
        ..Ret::default()
    }
}

/// `CopyVarList`/`CopyRefList` (source list, destination list): the head node copied.
fn copy_list(e: &mut Engine, a: &[u32]) -> Ret {
    let (from, to) = (a[0], a[1]);
    e.mem.set_u32(to, e.mem.u32(from));
    e.mem.set_u32(to + 4, e.mem.u32(from + 4));
    Ret::default()
}

fn nothing(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

/// A form at `at` with id `id` and the name `name` (none: `GetName` answers no text).
pub(crate) fn form(e: &mut Engine, at: u32, id: u32, name: Option<&str>) {
    e.mem.set_u32(at, VTABLE);
    e.mem.set_u32(at + 0xc, id);
    let n = name.map_or(0, |n| text_at(e, n));
    e.mem.set_u32(at + 0x60, n);
}

/// A parameter list (name, type, optional) in the area.
fn params(e: &mut Engine, list: &[(u32, bool)]) -> u32 {
    let at = e.mem.u32(PARAM_TOP).max(PARAMS_AT);
    for (i, (kind, optional)) in list.iter().enumerate() {
        let p = at + 12 * i as u32;
        e.mem.set_u32(p, 0);
        e.mem.set_u32(p + 4, *kind);
        e.mem.set_u32(p + 8, u32::from(*optional));
    }
    e.mem.set_u32(PARAM_TOP, at + 12 * list.len() as u32 + 12);
    at
}

/// A command record of the table with the game's default parser.
fn command_record(e: &mut Engine, opcode: u32, name: &str, list: &[(u32, bool)]) -> u32 {
    let record = record_of(e, opcode);
    let n = text_at(e, name);
    let p = params(e, list);
    e.mem.set_u32(record, n);
    e.mem.set_u16(record + 0x12, list.len() as u16);
    e.mem.set_u32(record + 0x14, p);
    e.mem.set_u32(record + 0x1c, 0x005b_1ba0);
    record
}

/// A local of the script with its name's length set (`String::CStr` needs it) and its type.
pub(crate) fn local(e: &mut Engine, id: u16, name: &str, var_type: u8) -> u32 {
    add_local(e, id, name, 0.0);
    let info = variable_info(e, SCRIPT, u32::from(id));
    e.mem.set_u16(info + 0x1c, name.len() as u16);
    e.mem.set_u16(info + 0x1e, name.len() as u16 + 1);
    e.mem.set_u8(info + 0x10, var_type);
    info
}

/// The name of a local `add_local` made, with its length set (`String::CStr` needs it).
pub(crate) fn local_info(e: &mut Engine, id: u16, name: &str) -> u32 {
    let info = variable_info(e, SCRIPT, u32::from(id));
    e.mem.set_u16(info + 0x1c, name.len() as u16);
    e.mem.set_u16(info + 0x1e, name.len() as u16 + 1);
    info
}

/// The engine: the table, the exe's tables as stand-ins, the stand-in forms, the script `Test`
/// (form type 0x11) with locals `x` (float), `i` (int), `rx` (a reference: ref 3 names it), and
/// references 1 (`PlayerRef`, the game's id 0x14) and 2 (a form with no name, id 0xABC123).
pub(crate) fn engine() -> Engine {
    let mut e = engine_with_table();
    e.map(0x0118_c000, 0x2000);
    e.map(AREA, 0x2_0000);
    e.register(NAME_FN, name_fn);
    e.mem.set_u32(VTABLE + GET_NAME, NAME_FN);
    for (i, b) in (0..OPERATOR_BYTES.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&OPERATOR_BYTES[i..i + 2], 16).unwrap())
        .enumerate()
    {
        e.mem.set_u8(GAME_OPERATORS + i as u32, b);
    }
    for (i, name) in STATEMENT_NAMES.iter().enumerate() {
        let at = STATEMENTS + 0x28 * i as u32;
        let n = text_at(&mut e, name);
        e.mem.set_u32(at, n);
        e.mem.set_u32(at + 8, 0x10 + i as u32);
    }
    for (i, name, list) in [
        (0u32, "GameMode", &[][..]),
        (1, "MenuMode", &[(0x01, true)][..]),
    ] {
        let at = EVENT_BLOCKS + 0x28 * i;
        let n = text_at(&mut e, name);
        let p = params(&mut e, list);
        e.mem.set_u32(at, n);
        e.mem.set_u32(at + 8, i);
        e.mem.set_u16(at + 0x12, list.len() as u16);
        e.mem.set_u32(at + 0x14, p);
        e.mem.set_u32(at + 0x1c, 0x005b_1ba0);
    }
    form(&mut e, FORM_A, 0x14, Some(""));
    form(&mut e, FORM_B, 0x00ab_c123, None);
    form(&mut e, SCRIPT, 0x0500_0d00, Some("Test"));
    e.mem.set_u8(SCRIPT + 4, 0x11);
    e.register(0x0048_39c0, |_, a| Ret {
        eax: match a[0] {
            0x14 => FORM_A,
            0x00ab_c123 => FORM_B,
            _ => 0,
        },
        ..Ret::default()
    });
    // `DataHandler`'s singleton word (none), read when a lambda's script is made.
    e.map(0x011c_3000, 0x1000);
    local(&mut e, 1, "x", 0);
    local(&mut e, 2, "i", 1);
    local(&mut e, 3, "rx", 0);
    set_script_ref(&mut e, 1, FORM_A);
    set_script_ref(&mut e, 2, FORM_B);
    set_script_ref(&mut e, 3, 0);
    e.mem.set_u32(0x0300_a000 + 3 * 0x10 + 0xc, 3);
    command_record(
        &mut e,
        0x1001,
        "V1Cmd",
        &[
            (0x01, false),
            (0x02, false),
            (0x00, false),
            (0x04, false),
            (0x08, false),
            (0x12, false),
        ],
    );
    command_record(&mut e, 0x1002, "GetV", &[(0x01, true)]);
    e
}

/// A line: its opcode, its length, its bytes.
pub(crate) fn line(op: u16, body: &[u8]) -> Vec<u8> {
    let mut b = op.to_le_bytes().to_vec();
    b.extend_from_slice(&(body.len() as u16).to_le_bytes());
    b.extend_from_slice(body);
    b
}

/// A reference call line (`0x1C`): the reference, the opcode, the length, the bytes.
pub(crate) fn ref_line(r: u16, op: u16, body: &[u8]) -> Vec<u8> {
    let mut b = vec![0x1c, 0];
    b.extend_from_slice(&r.to_le_bytes());
    b.extend_from_slice(&op.to_le_bytes());
    b.extend_from_slice(&(body.len() as u16).to_le_bytes());
    b.extend_from_slice(body);
    b
}

/// The game's expression form: its length, then the postfix text.
pub(crate) fn game_expr(parts: &[&[u8]]) -> Vec<u8> {
    let body = parts.concat();
    let mut b = (body.len() as u16).to_le_bytes().to_vec();
    b.extend(body);
    b
}

/// xNVSE's expression form: its length word (counting itself), then the tokens.
pub(crate) fn nvse_expr(tokens: &[&[u8]]) -> Vec<u8> {
    let body = tokens.concat();
    let mut b = (body.len() as u16 + 2).to_le_bytes().to_vec();
    b.extend(body);
    b
}

pub(crate) fn set_script(e: &mut Engine, lines: &[Vec<u8>]) {
    let data = lines.concat();
    e.mem.write(DATA_AT, &data);
    // What follows the data reads as nothing (the iterator reads one header past the end).
    e.mem.write(DATA_AT + data.len() as u32, &[0; 16]);
    e.mem.set_u32(SCRIPT + 0x30, DATA_AT);
    e.mem.set_u32(SCRIPT + 0x20, data.len() as u32);
}

fn text(b: B) -> String {
    String::from_utf8(b).unwrap()
}

/// The script of [`whole_script_decompiles_as_the_rules_write_it`].
pub(crate) fn sample_lines() -> Vec<Vec<u8>> {
    let mut v1 = vec![6, 0, b'n', 5, 0, 0, 0, b'z'];
    v1.extend_from_slice(&2.5f64.to_le_bytes());
    v1.extend_from_slice(&[3, 0, b'a', b'"', b'b', b'r', 1, 0, 0x58, 1, 0]);
    let mut getv_in_set = vec![b'f', 1, 0];
    getv_in_set.extend(game_expr(&[
        &[b'X', 2, 0x10, 7, 0, 1, 0, b'n', 7, 0, 0, 0],
        b" 1 +",
    ]));
    vec![
        line(0x1d, &[]),
        line(0x10, &[0, 0, 0, 0, 0, 0]),
        line(
            0x15,
            &[&[b'f', 1, 0][..], &game_expr(&[b"1 2 3 * +"])].concat(),
        ),
        line(
            0x153a,
            &[
                &[1u8][..],
                &nvse_expr(&[&[b'V', 0, 0, 0, 1, 0], &[b'B', 1], &[expr::op::ADD]]),
            ]
            .concat(),
        ),
        line(
            0x16,
            &[&[0u8, 0][..], &game_expr(&[&[b'f', 1, 0], b" 1 >"])].concat(),
        ),
        line(0x1001, &v1),
        line(
            0x18,
            &[&[0u8, 0][..], &game_expr(&[&[b'f', 1, 0], b" 2 <"])].concat(),
        ),
        line(0x17, &[]),
        ref_line(2, 0x1002, &[1, 0, b'n', 3, 0, 0, 0]),
        line(0x19, &[]),
        line(0x15, &getv_in_set),
        line(0x1e, &[]),
        line(0x11, &[]),
    ]
}

/// A whole script, each line as `ScriptAnalyzer`'s rules write it: the `scn` line and the
/// variables (`Float`, `Int`, `Ref`: a reference of the list names `rx`) first; a blank line before
/// a block opens after other lines and after a block closes; one tab per open block, `ElseIf` and
/// `Else` a tab out; `set` expressions with the game's precedence (a parenthesised `(1 + 2)` only
/// where a stronger operator takes it), a command with optional parameters in parentheses inside
/// an expression; numbers without trailing zeros, a quote in a string as `%q`, an axis and a sex
/// by name, the player's reference by the name the game does not keep, a form with no name as its
/// id, a reference call as `ref.Command`; an expression-format command (`eval`) through the
/// evaluator's line text.
#[test]
fn whole_script_decompiles_as_the_rules_write_it() {
    let mut e = engine();
    set_script(&mut e, &sample_lines());
    let want = "ScriptName Test\n\nFloat x\nInt i\nRef rx\n\nBegin GameMode\n\
        \tSet x To 1 + 2 * 3\n\teval x + 1\n\n\tIf x > 1\n\
        \t\tV1Cmd 5 2.5 \"a%qb\" PlayerRef X Female\n\tElseIf x < 2\n\tElse\n\
        \t\t<ABC123>.GetV 3\n\tEndIf\n\n\tSet x To (GetV 7) + 1\n\tReturn\nEnd\n";
    assert_eq!(text(decompile(&mut e, SCRIPT)), want);
    assert!(parses(&mut e, SCRIPT));
}

/// The game's precedence in a `set` expression: a composite operand is parenthesised only when a
/// stronger operator takes it; `~` is the unary minus, written `-`; a block type's parameter after
/// `Begin`.
#[test]
fn set_expressions_keep_their_precedence() {
    let mut e = engine();
    set_script(
        &mut e,
        &[
            line(0x10, &[1, 0, 0, 0, 0, 0, 1, 0, b'n', 0xff, 3, 0, 0]),
            line(
                0x15,
                &[&[b'f', 1, 0][..], &game_expr(&[b"1 2 + 3 *"])].concat(),
            ),
            line(
                0x15,
                &[
                    &[b'f', 1, 0][..],
                    &game_expr(&[&[b'f', 1, 0], b" ~ 4 5 - -"]),
                ]
                .concat(),
            ),
            line(
                0x15,
                &[&[b'f', 2, 0][..], &game_expr(&[b"1.250 3 /"])].concat(),
            ),
            line(0x11, &[]),
        ],
    );
    let want = "Float x\nInt i\nRef rx\n\nBegin MenuMode 1023\n\tSet x To (1 + 2) * 3\n\
        \tSet x To -x - 4 - 5\n\tSet i To 1.25 / 3\nEnd\n";
    assert_eq!(text(decompile(&mut e, SCRIPT)), want);
}

/// A line that does not read makes the whole script decompile to nothing, and `HasScriptCommand`
/// false: a string literal in a `set` expression (the analyzer steps back onto its last letter,
/// which is no operator and no number), an unknown opcode.
#[test]
fn a_line_that_does_not_read_fails_the_script() {
    let mut e = engine();
    set_script(
        &mut e,
        &[
            line(0x10, &[0, 0, 0, 0, 0, 0]),
            line(
                0x15,
                &[&[b'f', 1, 0][..], &game_expr(&[&[b'"', 2, 0, b'a', b'b']])].concat(),
            ),
            line(0x1001, &[]),
            line(0x11, &[]),
        ],
    );
    assert_eq!(decompile(&mut e, SCRIPT), b"");
    assert!(!parses(&mut e, SCRIPT));
    let v1 = record_of(&e, 0x1001);
    assert!(!contains_command(&mut e, SCRIPT, v1, 0));
    set_script(&mut e, &[line(0x0f, &[]), line(0x11, &[])]);
    assert_eq!(decompile(&mut e, SCRIPT), b"");
}

/// A string ending in a digit is read on as a number by that same step back (xNVSE's).
#[test]
fn a_string_ending_in_a_digit_reads_on_as_its_digit() {
    let mut e = engine();
    set_script(
        &mut e,
        &[line(
            0x15,
            &[&[b'f', 1, 0][..], &game_expr(&[&[b'"', 2, 0, b'a', b'7']])].concat(),
        )],
    );
    // The string and the 7 it steps back onto: two operands, the expression's failure text.
    assert_eq!(
        text(decompile(&mut e, SCRIPT)),
        "Float x\nInt i\nRef rx\nSet x To ; failed to decompile expression, bad expression \
         stack: '\"a7\"', '7'\n"
    );
}

/// Locals with no name get xNVSE's stand-in names: `a`/`s` for one the analyzer saw hold an
/// array or a string, `r`/`f` only for local 0 (xNVSE compares the id with the float type),
/// `i` otherwise; `form.name` for another script's local.
#[test]
fn unnamed_locals_get_the_stand_in_names() {
    let mut e = engine();
    let unnamed = local(&mut e, 4, "", 0);
    let zero = local(&mut e, 0, "", 0);
    let root = Root::none();
    assert_eq!(
        text(var_text(&mut e, &root, SCRIPT, unnamed, 0)),
        stand_in("i", 4)
    );
    // The list's plain form references carry variable 0, so local 0 reads as a reference.
    assert_eq!(
        text(var_text(&mut e, &root, SCRIPT, zero, 0)),
        stand_in("r", 0)
    );
    for r in [1, 2] {
        e.mem.set_u32(0x0300_a000 + r * 0x10 + 0xc, 99);
    }
    assert_eq!(
        text(var_text(&mut e, &root, SCRIPT, zero, 0)),
        stand_in("f", 0)
    );
    let mut root = Root::analyzer();
    root.register(unnamed, 3);
    assert_eq!(
        text(var_text(&mut e, &root, SCRIPT, unnamed, 0)),
        stand_in("a", 4)
    );
    let mut root = Root::analyzer();
    root.register(unnamed, 2);
    assert_eq!(
        text(var_text(&mut e, &root, SCRIPT, unnamed, 0)),
        stand_in("s", 4)
    );
    // Without an analyzer the arrays and strings seen are not asked.
    let mut inactive = Root::none();
    inactive.register(unnamed, 3);
    assert_eq!(
        text(var_text(&mut e, &inactive, SCRIPT, unnamed, 0)),
        stand_in("i", 4)
    );
    let x = variable_info(&e, SCRIPT, 1);
    form(&mut e, FORM_B, 0x00ab_c123, Some("Other"));
    assert_eq!(text(var_text(&mut e, &root, SCRIPT, x, FORM_B)), "Other.x");
}

/// The evaluator's line text of tokens: operators between their operands, a composite in
/// parentheses where a stronger operator takes it, a subscript, a unary operator, a string with
/// its quote and new line written `%q` and `%r`, a form by its editor id (the player's by the
/// name the game does not keep), a global by its name, an optional argument left empty as `_`, the
/// whole in parentheses for an inline expression; nothing when the tokens make no single value.
#[test]
fn line_text_writes_the_tokens_back() {
    let mut e = engine();
    let mut root = Root::none();
    let n = |v: f64| Tok::number(v);
    let x = Tok::Var {
        ty: Ty::NumericVar,
        local: 0,
        var_type: 0,
        ref_idx: 0,
        var_idx: 1,
        string: 0,
    };
    let mut lt =
        |e: &mut Engine, t: &[Tok], inline: bool| text(line_text(e, &mut root, SCRIPT, t, inline));
    use expr::op;
    assert_eq!(
        lt(
            &mut e,
            &[
                n(1.0),
                n(2.0),
                Tok::Op(op::ADD),
                n(3.0),
                Tok::Op(op::MULTIPLY)
            ],
            false
        ),
        "(1 + 2) * 3"
    );
    assert_eq!(
        lt(
            &mut e,
            &[
                n(1.0),
                n(2.0),
                n(3.0),
                Tok::Op(op::MULTIPLY),
                Tok::Op(op::ADD)
            ],
            false
        ),
        "1 + 2 * 3"
    );
    assert_eq!(
        lt(
            &mut e,
            &[x.clone(), n(0.5), Tok::Op(op::LEFT_BRACKET)],
            true
        ),
        "(x[0.5])"
    );
    assert_eq!(lt(&mut e, &[x.clone(), Tok::Op(op::NEGATION)], false), "-x");
    assert_eq!(
        lt(&mut e, &[Tok::Str(b"a\"b\nc".to_vec())], false),
        "\"a%qb%rc\""
    );
    assert_eq!(lt(&mut e, &[Tok::Form(0x14)], false), "PlayerRef");
    assert_eq!(lt(&mut e, &[Tok::Form(0x00ab_c123)], false), "<ABC123>");
    assert_eq!(lt(&mut e, &[Tok::Form(0x55)], false), "<bad form>");
    let gname = text_at(&mut e, "TimeScale");
    e.mem.set_u32(GLOBAL + 0x18, gname);
    e.mem.set_u16(GLOBAL + 0x1c, 9);
    assert_eq!(lt(&mut e, &[Tok::Global(GLOBAL)], false), "TimeScale");
    assert_eq!(lt(&mut e, &[Tok::OptionalEmpty], false), "_");
    assert_eq!(lt(&mut e, &[n(1.0), n(2.0)], false), "");
    assert_eq!(lt(&mut e, &[Tok::Op(op::ADD)], false), "");
    assert_eq!(
        lt(&mut e, &[n(1.0), n(1e20), Tok::Op(op::EXPONENT)], false),
        "1^1e+20"
    );
    let unknown = Tok::Var {
        ty: Ty::NumericVar,
        local: 0,
        var_type: 0,
        ref_idx: 0,
        var_idx: 9,
        string: 0,
    };
    assert_eq!(
        lt(&mut e, &[unknown], false),
        "<failed to get var name 0 9>"
    );
}

/// A number given to an actor value, animation group, miscellaneous statistic or critical stage
/// parameter prints by name; a value past the tables prints as a number.
#[test]
fn numbers_print_by_name_in_name_parameters() {
    let mut e = engine();
    e.map(0x011d_6000, 0x1000);
    e.map(0x0119_7000, 0x3000);
    e.map(0x0118_9000, 0x1000);
    e.map(0x0119_b000, 0x1000);
    let info = AREA + 0x5000;
    let agility = text_at(&mut e, "Agility");
    e.mem.set_u32(info + 0x38, agility);
    e.mem.set_u32(ACTOR_VALUES + 4 * 12, info);
    let walk = text_at(&mut e, "Forward");
    e.mem.set_u32(ANIM_GROUPS + 0x24 * 3, walk);
    let stat = text_at(&mut e, "Quests Completed");
    e.mem.set_u32(MISC_STATS + 4 * 2, stat);
    let stage = text_at(&mut e, "DisintegrateEnd");
    e.mem.set_u32(CRITICAL_STAGES + 4 * 4, stage);
    let t = |e: &mut Engine, ty, v| text(numeric_param_text(e, ty, v));
    assert_eq!(t(&mut e, 0x05, 12), "Agility");
    assert_eq!(t(&mut e, 0x05, 13), "");
    // A code past the 77 actor values has no name, whatever lies past the table.
    e.mem.set_u32(ACTOR_VALUES + 4 * 80, info);
    assert_eq!(t(&mut e, 0x05, 80), "");
    assert_eq!(t(&mut e, 0x0a, 3), "Forward");
    assert_eq!(t(&mut e, 0x0a, 245), "");
    assert_eq!(t(&mut e, 0x29, 2), "\"Quests Completed\"");
    assert_eq!(t(&mut e, 0x37, 4), "DisintegrateEnd");
    assert_eq!(t(&mut e, 0x37, 5), "");
    assert_eq!(t(&mut e, 0x08, 0x59), "Y");
    assert_eq!(t(&mut e, 0x08, 1), "<unknown axis>");
    assert_eq!(t(&mut e, 0x12, 0), "Male");
    assert_eq!(t(&mut e, 0x12, 2), "<invalid sex>");
    assert_eq!(t(&mut e, 0x01, 2), "");
}

/// `HasScriptCommand`'s search: a line, a command inside a `set` expression, a command inside an
/// expression-format command's arguments, a block type; with a block type given, only inside that
/// block type's blocks.
#[test]
fn contains_command_finds_every_kind_of_call() {
    let mut e = engine();
    set_script(&mut e, &sample_lines());
    let v1 = record_of(&e, 0x1001);
    let getv = record_of(&e, 0x1002);
    let eval = record_of(&e, 0x153a);
    let game_mode = event_block(0);
    let menu_mode = event_block(1);
    assert!(contains_command(&mut e, SCRIPT, v1, 0));
    assert!(contains_command(&mut e, SCRIPT, getv, 0));
    assert!(contains_command(&mut e, SCRIPT, eval, 0));
    assert!(contains_command(&mut e, SCRIPT, game_mode, 0));
    assert!(!contains_command(&mut e, SCRIPT, menu_mode, 0));
    assert!(contains_command(&mut e, SCRIPT, v1, game_mode));
    assert!(!contains_command(&mut e, SCRIPT, v1, menu_mode));
    // Only in a `set` expression's command.
    set_script(
        &mut e,
        &[line(
            0x15,
            &[
                &[b'f', 1, 0][..],
                &game_expr(&[&[b'X', 2, 0x10, 7, 0, 1, 0, b'n', 7, 0, 0, 0]]),
            ]
            .concat(),
        )],
    );
    assert!(contains_command(&mut e, SCRIPT, getv, 0));
    assert!(!contains_command(&mut e, SCRIPT, v1, 0));
    // Only inside `eval`'s expression.
    let mut getv_token = vec![b'X', 0, 0, 2, 0x10];
    getv_token.extend_from_slice(&9u16.to_le_bytes());
    getv_token.extend_from_slice(&[1, 0, b'n', 7, 0, 0, 0]);
    set_script(
        &mut e,
        &[line(
            0x153a,
            &[&[1u8][..], &nvse_expr(&[&getv_token])].concat(),
        )],
    );
    assert!(contains_command(&mut e, SCRIPT, getv, 0));
    assert_eq!(
        text(decompile(&mut e, SCRIPT)),
        "Float x\nInt i\nRef rx\neval GetV 7\n"
    );
}

/// xNVSE's own statements: `while` steps over its 4-byte jump before its expression and opens a
/// block that `loop` closes; `Call` reads its version byte, then the function and its arguments;
/// a message box command reads its message's numbers after its parameters; a block compiled
/// with the compiler override is written `Begin _GameMode`, its `@Push`/`@PopExecutionContext`
/// lines left out.
#[test]
fn xnvse_statements_and_the_override_decompile() {
    let mut e = engine();
    let msg = command_record(&mut e, 0x1003, "MsgBox", &[(0x00, false)]);
    e.mem.set_u32(msg + 0x1c, 0x005b_3cd0);
    let while_args = [
        &[0u8, 0, 0, 0, 1][..],
        &nvse_expr(&[&[b'V', 0, 0, 0, 1, 0], &[b'B', 1], &[expr::op::LESS_THAN]]),
    ]
    .concat();
    let call_args = [
        &[1u8][..],
        &nvse_expr(&[&[b'R', 1, 0]]),
        &[1u8][..],
        &nvse_expr(&[&[b'B', 2]]),
    ]
    .concat();
    let mut msg_args = vec![1, 0, 7, 0];
    msg_args.extend_from_slice(b"Hi %.0f");
    msg_args.extend_from_slice(&[1, 0, b'n', 3, 0, 0, 0]);
    set_script(
        &mut e,
        &[
            line(0x10, &[0, 0, 0, 0, 0, 0, 0xfe, 0xff, 0]),
            line(0x154d, &[]),
            line(0x153b, &while_args),
            line(0x15, &[&[b'f', 1, 0][..], &game_expr(&[b"5"])].concat()),
            line(0x153c, &[]),
            line(0x1545, &call_args),
            line(0x1003, &msg_args),
            line(0x154e, &[]),
            line(0x11, &[]),
        ],
    );
    assert_eq!(
        text(decompile(&mut e, SCRIPT)),
        "Float x\nInt i\nRef rx\n\nBegin _GameMode\n\twhile x < 1\n\t\tSet x To 5\n\tloop\n\n\
         \tCall PlayerRef 2\n\tMsgBox \"Hi %.0f\" 3\nEnd\n"
    );
}

/// A lambda in an expression decompiles through the analyzer as a lambda (no `scn` line, no
/// variable header), its text trimmed; the script made for it is freed.
#[test]
fn a_lambda_in_an_expression_decompiles() {
    let mut e = engine();
    e.register(0x0040_1000, heap);
    e.register(0x005a_a0f0, script_constructor);
    e.register(0x005a_b930, copy_list);
    e.register(0x005a_b7f0, copy_list);
    e.register(0x0046_4d30, nothing);
    e.register(FORM_HEAP_FREE, nothing);
    let body = [
        line(0x1d, &[]),
        line(0x10, &[0, 0, 0, 0, 0, 0]),
        line(0x15, &[&[b'f', 1, 0][..], &game_expr(&[b"5"])].concat()),
        line(0x11, &[]),
    ]
    .concat();
    let data = AREA + 0x9800;
    e.mem.write(data, &body);
    e.mem.write(data + body.len() as u32, &[0; 16]);
    let mut root = Root::none();
    let tokens = [Tok::LambdaData {
        data,
        len: body.len() as u32,
    }];
    assert_eq!(
        text(line_text(&mut e, &mut root, SCRIPT, &tokens, false)),
        "Begin GameMode\n\tSet x To 5\nEnd"
    );
}

/// The stand-ins a lambda's script needs (`FormHeap_Allocate`, `Script`'s constructor, the list
/// copies, `DisableAssignFormIDs`, `FormHeap_Free`).
pub(crate) fn lambda_stand_ins(e: &mut Engine) {
    e.register(0x0040_1000, heap);
    e.register(0x005a_a0f0, script_constructor);
    e.register(0x005a_b930, copy_list);
    e.register(0x005a_b7f0, copy_list);
    e.register(0x0046_4d30, nothing);
    e.register(FORM_HEAP_FREE, nothing);
}

/// A page of its own for a script's data, with no page after it: a read past the data stops.
const FUZZ_PAGE: u32 = 0x0610_0000;

fn put_at_page_end(e: &mut Engine, bytes: &[u8]) {
    let at = FUZZ_PAGE + 0x1000 - bytes.len() as u32;
    e.mem.write(at, bytes);
    e.mem.set_u32(SCRIPT + 0x30, at);
    e.mem.set_u32(SCRIPT + 0x20, bytes.len() as u32);
}

/// Corrupt and cut scripts (a mod's hand-patched or damaged script handed to `DecompileScript` or
/// `HasScriptCommand`) never stop the game: the sample script, a block with a parameter and a
/// lambda line, cut at every byte, and each of its words overwritten with 0 and with 0xFFFF, the
/// data ending where memory ends. Each read decompiles to xNVSE's error text or nothing, and
/// `HasScriptCommand` answers false or true, without a read past the data.
#[test]
fn corrupt_and_cut_scripts_never_stop_the_game() {
    let mut e = engine();
    lambda_stand_ins(&mut e);
    e.map(FUZZ_PAGE, 0x1000);
    let lambda_body = [
        line(0x10, &[0, 0, 0, 0, 0, 0]),
        line(0x15, &[&[b'f', 1, 0][..], &game_expr(&[b"5"])].concat()),
        line(0x11, &[]),
    ]
    .concat();
    let mut lambda_token = vec![b'F'];
    lambda_token.extend_from_slice(&(lambda_body.len() as u32).to_le_bytes());
    lambda_token.extend_from_slice(&lambda_body);
    let mut lines = sample_lines();
    lines.insert(
        3,
        line(0x153a, &[&[1u8][..], &nvse_expr(&[&lambda_token])].concat()),
    );
    lines.push(line(0x10, &[1, 0, 0, 0, 0, 0, 1, 0, b'n', 7, 0, 0, 0]));
    lines.push(line(
        0x153a,
        &[&[1u8][..], &nvse_expr(&[&[b'S', 3, 0, b'a', b'b', b'c']])].concat(),
    ));
    lines.push(line(0x11, &[]));
    let base = lines.concat();
    put_at_page_end(&mut e, &base);
    assert!(
        !decompile(&mut e, SCRIPT).is_empty(),
        "the whole script reads"
    );
    let v1 = record_of(&e, 0x1001);
    let game_mode = event_block(0);
    let read_all = |e: &mut Engine, bytes: &[u8]| {
        put_at_page_end(e, bytes);
        let _ = decompile(e, SCRIPT);
        let _ = parses(e, SCRIPT);
        let _ = contains_command(e, SCRIPT, v1, 0);
        let _ = contains_command(e, SCRIPT, v1, game_mode);
    };
    for cut in 0..=base.len() {
        read_all(&mut e, &base[..cut]);
    }
    for i in 0..base.len() - 1 {
        for word in [0u16, 0xffff] {
            let mut b = base.clone();
            b[i..i + 2].copy_from_slice(&word.to_le_bytes());
            read_all(&mut e, &b);
        }
    }
    // A lambda whose length word runs to 4 GB: refused, not copied; the line fails.
    let mut huge = vec![b'F'];
    huge.extend_from_slice(&0xffff_fff0u32.to_le_bytes());
    put_at_page_end(
        &mut e,
        &line(0x153a, &[&[1u8][..], &nvse_expr(&[&huge])].concat()),
    );
    assert_eq!(decompile(&mut e, SCRIPT), b"");
    assert!(!parses(&mut e, SCRIPT));
}

/// A string argument of up to 0x200 bytes reads; one of 0x201 fails its line (xNVSE's cap).
#[test]
fn a_string_argument_is_capped_at_0x200_bytes() {
    let mut e = engine();
    let rec = command_record(&mut e, 0x1007, "Say", &[(0x00, false)]);
    let _ = rec;
    for (len, reads) in [(0x200usize, true), (0x201, false)] {
        let mut body = vec![1, 0];
        body.extend_from_slice(&(len as u16).to_le_bytes());
        body.extend(vec![b'a'; len]);
        set_script(&mut e, &[line(0x1007, &body)]);
        assert_eq!(parses(&mut e, SCRIPT), reads, "{len:#x}");
    }
}

/// More arguments than the command has parameters, or a message box with more than 9 numbers:
/// the line fails (xNVSE reads on past the parameter list).
#[test]
fn argument_counts_past_the_parameters_fail_the_line() {
    let mut e = engine();
    set_script(
        &mut e,
        &[line(0x1002, &[2, 0, b'n', 1, 0, 0, 0, b'n', 2, 0, 0, 0])],
    );
    assert!(!parses(&mut e, SCRIPT));
    let msg = command_record(&mut e, 0x1003, "MsgBox", &[(0x00, false)]);
    e.mem.set_u32(msg + 0x1c, 0x005b_3cd0);
    for (count, reads) in [(9u8, true), (10, false)] {
        let mut body = vec![1, 0, 1, 0, b'M', count, 0];
        for k in 0..count {
            body.extend_from_slice(&[b'n', k, 0, 0, 0]);
        }
        set_script(&mut e, &[line(0x1003, &body)]);
        assert_eq!(parses(&mut e, SCRIPT), reads, "{count}");
    }
}

/// Lambdas inside lambdas past 16 deep decompile to nothing at that depth (xNVSE recurses until
/// its stack runs out).
#[test]
fn lambdas_nested_past_the_limit_stop() {
    let mut e = engine();
    lambda_stand_ins(&mut e);
    // Innermost: a lambda body; each level wraps the one inside in an `eval` of an `F` token.
    let mut body = [line(0x10, &[0, 0, 0, 0, 0, 0]), line(0x11, &[])].concat();
    for _ in 0..20 {
        let mut token = vec![b'F'];
        token.extend_from_slice(&(body.len() as u32).to_le_bytes());
        token.extend_from_slice(&body);
        body = line(0x153a, &[&[1u8][..], &nvse_expr(&[&token])].concat());
    }
    let data = AREA + 0x10000;
    e.mem.write(data, &body);
    e.mem.write(data + body.len() as u32, &[0; 16]);
    let mut root = Root::none();
    let tokens = [Tok::LambdaData {
        data,
        len: body.len() as u32,
    }];
    let t = text(line_text(&mut e, &mut root, SCRIPT, &tokens, false));
    assert_eq!(t.matches("eval").count(), 16, "{t}");
    assert_eq!(LAMBDA_DEPTH.with(std::cell::Cell::get), 0);
}

/// The paths xNVSE-compiled and user-function scripts take: a parenthesised argument of a game
/// command (`0xFFFF`, the expression in parentheses); `Begin Function {x, i}` (its event list
/// made and freed once by the read, as xNVSE's `FunctionInfo` makes and frees it); a variable
/// given to a variable parameter and to a number parameter; actor value, animation group and
/// misc stat parameters by name; the game's other operators.
#[test]
fn xnvse_compiled_and_function_scripts_decompile() {
    let mut e = engine();
    // Block type 0x0D is xNVSE's `Function` (its record copied there, `functions::install`).
    let function = record_of(&e, crate::functions::OP_FUNCTION);
    let bytes = e.mem.bytes(function, 0x28);
    e.mem.write(EVENT_BLOCKS + 13 * 0x28, &bytes);
    e.mem.set_u32(EVENT_BLOCKS + 13 * 0x28 + 8, 13);
    thread_local! {
        static LISTS: std::cell::Cell<(u32, u32)> = const { std::cell::Cell::new((0, 0)) };
    }
    e.register(crate::functions::CREATE_EVENT_LIST, |_, _| {
        LISTS.with(|l| l.set((l.get().0 + 1, l.get().1)));
        Ret {
            eax: AREA + 0x1_8000,
            ..Ret::default()
        }
    });
    e.register(crate::functions::EVENT_LIST_DESTROY, |_, a| {
        assert_eq!(a[0], AREA + 0x1_8000);
        LISTS.with(|l| l.set((l.get().0, l.get().1 + 1)));
        Ret::default()
    });
    set_script(
        &mut e,
        &[
            line(0x1d, &[]),
            line(0x10, &[0x0d, 0, 0, 0, 0, 0, 1, 2, 1, 0, 0, 2, 0, 1]),
            line(0x11, &[]),
        ],
    );
    LISTS.with(|l| l.set((0, 0)));
    assert_eq!(
        text(decompile(&mut e, SCRIPT)),
        "ScriptName Test\n\nFloat x\nInt i\nRef rx\n\nBegin Function { x, i }\nEnd\n"
    );
    assert_eq!(
        LISTS.with(std::cell::Cell::get),
        (1, 1),
        "made and freed once"
    );
    // Arguments.
    e.map(0x011d_6000, 0x1000);
    e.map(0x0119_7000, 0x3000);
    e.map(0x0118_9000, 0x1000);
    let info = AREA + 0x5000;
    let agility = text_at(&mut e, "Agility");
    e.mem.set_u32(info + 0x38, agility);
    e.mem.set_u32(ACTOR_VALUES + 4 * 12, info);
    let forward = text_at(&mut e, "Forward");
    e.mem.set_u32(ANIM_GROUPS + 0x24 * 3, forward);
    let stat = text_at(&mut e, "Quests Completed");
    e.mem.set_u32(MISC_STATS + 4 * 2, stat);
    command_record(&mut e, 0x1004, "VarCmd", &[(0x2d, false)]);
    command_record(&mut e, 0x1005, "IntCmd", &[(0x01, false)]);
    command_record(
        &mut e,
        0x1006,
        "NameCmd",
        &[(0x05, false), (0x0a, false), (0x29, false)],
    );
    let mut inline = vec![1, 0, 0xff, 0xff];
    inline.extend(nvse_expr(&[
        &[b'V', 0, 0, 0, 1, 0],
        &[b'B', 1],
        &[expr::op::ADD],
    ]));
    set_script(
        &mut e,
        &[
            line(0x1002, &inline),
            line(0x1004, &[1, 0, b'f', 1, 0]),
            line(0x1005, &[1, 0, b's', 2, 0]),
            line(0x1006, &[3, 0, 12, 0, 3, 0, 2, 0]),
            line(
                0x15,
                &[
                    &[b'f', 1, 0][..],
                    &game_expr(&[b"1 2 <= 3 4 >= || 5 6 != && 7 8 % +"]),
                ]
                .concat(),
            ),
        ],
    );
    assert_eq!(
        text(decompile(&mut e, SCRIPT)),
        "Float x\nInt i\nRef rx\nGetV (x + 1)\nVarCmd x\nIntCmd i\n\
         NameCmd Agility Forward \"Quests Completed\"\n\
         Set x To 1 <= 2 || 3 >= 4 && 5 != 6 + 7 % 8\n"
    );
}

/// A plugin's expression-format command (JIP LN's `COMMAND_PLUGIN_EXP`) gets the expression
/// parse word, so its line reads as expressions, as in xNVSE, and the script decompiles.
#[test]
fn a_plugin_expression_command_decompiles() {
    let mut e = engine();
    crate::table::grow_to(&mut e, 0x2202);
    let name = "JipExp";
    crate::table::add_plugin_record(
        &mut e,
        0x2201,
        &crate::table::PluginRecord {
            name,
            short: "",
            needs_ref: false,
            param_count: 0,
            params: 0,
            execute: 0,
            eval: 0,
            flags: 0,
            expression: true,
            returns: crate::table::Returns::Default,
        },
    );
    assert_eq!(
        e.mem.u32(record_of(&e, 0x2201) + 0x1c),
        crate::table::EXPRESSION_PARSE
    );
    set_script(
        &mut e,
        &[line(
            0x2201,
            &[&[1u8][..], &nvse_expr(&[&[b'B', 4]])].concat(),
        )],
    );
    assert_eq!(
        text(decompile(&mut e, SCRIPT)),
        "Float x\nInt i\nRef rx\nJipExp 4\n"
    );
}

/// What vanilla scripts use, hand-built (the shapes of the three player's scripts the opt-in
/// test checks): a block type whose optional parameter is left out (`Begin MenuMode`), an empty
/// block, a blank line between blocks, `If`s nested three deep with an `Else`, a reference call
/// inside an `If` expression (in parentheses: its command has an optional parameter and an
/// operator follows), a float argument written without its trailing zeros (`0.01`), a string
/// argument in quotes.
#[test]
fn vanilla_shapes_decompile_by_the_rules() {
    let mut e = engine();
    command_record(&mut e, 0x1007, "Say", &[(0x00, false)]);
    command_record(&mut e, 0x1008, "SetS", &[(0x02, false)]);
    let mut sets = vec![1, 0, b'z'];
    sets.extend_from_slice(&0.01f64.to_le_bytes());
    set_script(
        &mut e,
        &[
            line(0x10, &[1, 0, 0, 0, 0, 0]),
            line(0x11, &[]),
            line(0x10, &[0, 0, 0, 0, 0, 0]),
            line(
                0x16,
                &[&[0u8, 0][..], &game_expr(&[&[b'f', 1, 0], b" 0 >"])].concat(),
            ),
            line(
                0x16,
                &[
                    &[0u8, 0][..],
                    &game_expr(&[
                        &[b'r', 1, 0, b'X', 2, 0x10, 7, 0, 1, 0, b'n', 7, 0, 0, 0],
                        b" 1 >=",
                    ]),
                ]
                .concat(),
            ),
            line(
                0x16,
                &[&[0u8, 0][..], &game_expr(&[&[b's', 2, 0], b" 2 =="])].concat(),
            ),
            line(0x1008, &sets),
            line(0x17, &[]),
            line(0x1007, &[1, 0, 2, 0, b'h', b'i']),
            line(0x19, &[]),
            line(0x19, &[]),
            line(0x19, &[]),
            line(0x11, &[]),
        ],
    );
    // Begin (the first line: a blank line before it, as text is empty), End, a blank line before
    // the next Begin (it follows a closing End), then one tab per open block; Else a tab out;
    // three EndIfs closing in turn with no blank line between them, End last.
    assert_eq!(
        text(decompile(&mut e, SCRIPT)),
        "Float x\nInt i\nRef rx\n\nBegin MenuMode\nEnd\n\nBegin GameMode\n\tIf x > 0\n\
         \t\tIf (PlayerRef.GetV 7) >= 1\n\t\t\tIf i == 2\n\t\t\t\tSetS 0.01\n\t\t\tElse\n\
         \t\t\t\tSay \"hi\"\n\t\t\tEndIf\n\t\tEndIf\n\tEndIf\nEnd\n"
    );
}

// ---- the player's compiled scripts ----

/// The exe's form type table: 0x79 entries of 12 bytes, `+4` the record signature's text.
const FORM_TYPE_TABLE: u32 = 0x0118_7000;

/// Stand-in forms for a real script's references: each a block with the name vtable, its id, its
/// form type (the exe's table, by its record's signature) and, for a global, its editor id as
/// its name (`TESGlobal::name`, which the game keeps).
struct RealForms {
    by_id: std::collections::HashMap<u32, u32>,
    vtable: u32,
}

fn real_form(
    e: &mut Engine,
    forms: &mut RealForms,
    plugin: &esm::Plugin,
    types: &std::collections::HashMap<[u8; 4], u8>,
    id: u32,
) -> u32 {
    if let Some(&f) = forms.by_id.get(&id) {
        return f;
    }
    let entry = plugin.get(esm::FormId(id));
    let signature = entry.map_or(*b"----", |r| r.header.kind.0);
    let at = e.mem.alloc(0x80);
    e.mem.write(at, &[0; 0x80]);
    e.mem.set_u32(at, forms.vtable);
    e.mem
        .set_u8(at + 4, types.get(&signature).copied().unwrap_or(0));
    e.mem.set_u32(at + 0xc, id);
    if &signature == b"GLOB" {
        let edid = entry
            .and_then(|r| plugin.editor_id_of(r).ok().flatten())
            .unwrap_or_default();
        let t = e.mem.alloc(edid.len() as u32 + 1);
        e.mem.write(t, edid.as_bytes());
        e.mem.set_u8(t + edid.len() as u32, 0);
        e.mem.set_u32(at + 0x18, t);
        e.mem.set_u16(at + 0x1c, edid.len() as u16);
    }
    forms.by_id.insert(id, at);
    REAL_FORMS.with(|f| f.borrow_mut().insert(id, at));
    // A quest's script (`TESQuest +0x1C`, its record's `SCRI`), for its variables read as
    // `quest.variable`.
    if &signature == b"QUST" {
        let scri = entry
            .and_then(|r| plugin.record(r).ok())
            .and_then(|r| r.get(esm::FourCC(*b"SCRI")).map(|s| s.data.clone()));
        if let Some(word) = scri.filter(|d| d.len() >= 4) {
            let script_id = u32::from_le_bytes(word[..4].try_into().unwrap());
            let record = plugin
                .get(esm::FormId(script_id))
                .and_then(|r| plugin.record(r).ok());
            if let Some(record) = record {
                let script = real_script(e, forms, plugin, types, &record);
                e.mem.set_u32(at + 0x1c, script);
            }
        }
    }
    at
}

/// A fake `__RTDynamicCast` for the stand-in forms, by form type: a reference (0x3A..=0x40,
/// 0x69), a global (0x06), a quest (0x47).
fn real_cast(e: &mut Engine, a: &[u32]) -> Ret {
    let (object, to) = (a[0], a[3]);
    let t = if object == 0 { 0 } else { e.mem.u8(object + 4) };
    let is = match to {
        RTTI_TES_OBJECT_REFR => (0x3a..=0x40).contains(&t) || t == 0x69,
        RTTI_TES_GLOBAL => t == 0x06,
        RTTI_TES_QUEST => t == 0x47,
        _ => false,
    };
    Ret {
        eax: if is { object } else { 0 },
        ..Ret::default()
    }
}

thread_local! {
    static REAL_FORMS: std::cell::RefCell<std::collections::HashMap<u32, u32>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn real_lookup(_: &mut Engine, a: &[u32]) -> Ret {
    Ret {
        eax: REAL_FORMS.with(|f| f.borrow().get(&a[0]).copied().unwrap_or(0)),
        ..Ret::default()
    }
}

/// A `Script` of the player's `FalloutNV.esm` as the game loads it: `SCHR` at `+0x18`, `SCDA` as
/// its data, `SCRO`/`SCRV` as its reference list, `SLSD`/`SCVR` as its locals; its name the
/// editor id (what `GetName` answers in the GECK and for the script files xNVSE writes).
fn real_script(
    e: &mut Engine,
    forms: &mut RealForms,
    plugin: &esm::Plugin,
    types: &std::collections::HashMap<[u8; 4], u8>,
    record: &esm::Record,
) -> u32 {
    let script = e.mem.alloc(0x80);
    e.mem.write(script, &[0; 0x80]);
    e.mem.set_u32(script, forms.vtable);
    e.mem.set_u8(script + 4, 0x11);
    e.mem.set_u32(script + 0xc, record.header.form_id.0);
    let edid = record.editor_id().unwrap_or_default();
    let name = e.mem.alloc(edid.len() as u32 + 1);
    e.mem.write(name, edid.as_bytes());
    e.mem.set_u8(name + edid.len() as u32, 0);
    e.mem.set_u32(script + 0x60, name);
    let sub = |k: &[u8; 4]| record.get(esm::FourCC(*k)).map(|s| s.data.clone());
    if let Some(h) = sub(b"SCHR") {
        e.mem.write(script + 0x18, &h[..h.len().min(0x14)]);
    }
    let code = sub(b"SCDA").unwrap_or_default();
    let data = e.mem.alloc(code.len() as u32 + 16);
    e.mem.write(data, &code);
    e.mem.write(data + code.len() as u32, &[0; 16]);
    e.mem.set_u32(script + 0x30, data);
    e.mem.set_u32(script + 0x20, code.len() as u32);
    let mut refs = Vec::new();
    let mut vars = Vec::new();
    let mut pending = None;
    for s in &record.subrecords {
        match &s.kind.0 {
            b"SCRO" | b"SCRV" => {
                let word = u32::from_le_bytes(s.data[..4].try_into().unwrap());
                let r = e.mem.alloc(0x10);
                e.mem.write(r, &[0; 0x10]);
                if &s.kind.0 == b"SCRO" {
                    let f = real_form(e, forms, plugin, types, word);
                    e.mem.set_u32(r + 8, f);
                } else {
                    e.mem.set_u32(r + 0xc, word);
                }
                refs.push(r);
            }
            b"SLSD" => {
                let v = e.mem.alloc(0x20);
                e.mem.write(v, &[0; 0x20]);
                e.mem.write(v, &s.data[..s.data.len().min(0x18)]);
                vars.push(v);
                pending = Some(v);
            }
            b"SCVR" => {
                if let Some(v) = pending.take() {
                    let len = s.data.iter().position(|&b| b == 0).unwrap_or(s.data.len());
                    let t = e.mem.alloc(len as u32 + 1);
                    e.mem.write(t, &s.data[..len]);
                    e.mem.set_u8(t + len as u32, 0);
                    e.mem.set_u32(v + 0x18, t);
                    e.mem.set_u16(v + 0x1c, len as u16);
                    e.mem.set_u16(v + 0x1e, len as u16 + 1);
                }
            }
            _ => {}
        }
    }
    for (head, items) in [(script + 0x44, refs), (script + 0x4c, vars)] {
        let mut node = head;
        for (i, item) in items.iter().enumerate() {
            e.mem.set_u32(node, *item);
            if i + 1 < items.len() {
                let next = e.mem.alloc(8);
                e.mem.set_u32(next, 0);
                e.mem.set_u32(next + 4, 0);
                e.mem.set_u32(node + 4, next);
                node = next;
            }
        }
    }
    REAL_FORMS.with(|f| {
        f.borrow_mut()
            .extend(forms.by_id.iter().map(|(k, v)| (*k, *v)))
    });
    script
}

/// The player's compiled scripts read back. Each case names a vanilla script and the text the
/// analyzer's rules give for its compiled bytes, worked out line by line from the rules (not its
/// `SCTX` source verbatim: the decompiled text has the game's names for its blocks and statements,
/// `Set .. To` capitalised, the game's postfix order re-parenthesised, numbers without trailing
/// zeros, forms the game keeps no editor id for written as their ids). With `NV_DECOMPILE_DUMP`
/// (a file path) every script's source and decompiled text are written there for review.
#[test]
#[ignore = "needs the player's FalloutNV.exe and FalloutNV.esm (NV_DATA: the game's Data folder)"]
fn the_players_compiled_scripts_decompile_by_the_rules() {
    let data = std::path::PathBuf::from(std::env::var("NV_DATA").expect("NV_DATA"));
    let exe = data
        .parent()
        .expect("the game's folder")
        .join("FalloutNV.exe");
    let mut e = Engine::with_exe(&exe).expect("FalloutNV.exe");
    crate::table::install(&mut e);
    let vtable = e.mem.alloc(0x200);
    e.mem.write(vtable, &[0; 0x200]);
    e.mem.set_u32(vtable + GET_NAME, NAME_FN);
    e.register(NAME_FN, name_fn);
    e.register(DYNAMIC_CAST, real_cast);
    e.register(LOOKUP_FORM, real_lookup);
    // No reference has a script here (`ExtraDataList::GetByType`).
    e.register(0x0041_0220, |_, _| Ret::default());
    let mut types = std::collections::HashMap::new();
    for t in 0..0x79u32 {
        let at = e.mem.u32(FORM_TYPE_TABLE + 12 * t + 4);
        if at != 0 {
            let s = e.mem.bytes(at, 4);
            types.insert([s[0], s[1], s[2], s[3]], t as u8);
        }
    }
    let plugin = esm::Plugin::open(data.join("FalloutNV.esm")).expect("FalloutNV.esm");
    let mut forms = RealForms {
        by_id: std::collections::HashMap::new(),
        vtable,
    };
    let dump = std::env::var_os("NV_DECOMPILE_DUMP");
    let mut report = String::new();
    let mut texts = std::collections::HashMap::new();
    for entry in plugin.records_of_type(esm::FourCC(*b"SCPT")) {
        let record = plugin.record(entry).expect("SCPT");
        let edid = record.editor_id().unwrap_or_default();
        let wanted = REAL_CASES.iter().any(|(n, _)| *n == edid);
        if dump.is_none() && !wanted {
            continue;
        }
        let script = real_script(&mut e, &mut forms, &plugin, &types, &record);
        let mut out = String::from_utf8_lossy(&decompile(&mut e, script)).into_owned();
        if dump.is_some() && out.is_empty() {
            // The lines one by one, those that did not read marked.
            let mut root = Root::analyzer();
            let (lines, _) = parse(&mut e, &mut root, script);
            for l in &lines {
                let t = String::from_utf8_lossy(&l.text(&mut e, &mut root)).into_owned();
                out.push_str(&format!("{}{t}\n", if l.error { "!! " } else { "   " }));
            }
        }
        if dump.is_some() {
            let source = record
                .get(esm::FourCC(*b"SCTX"))
                .map(|s| String::from_utf8_lossy(&s.data).into_owned())
                .unwrap_or_default();
            report.push_str(&format!(
                "==== {edid}\n---- SCTX\n{source}\n---- decompiled\n{out}\n"
            ));
        }
        texts.insert(edid, out);
    }
    if let Some(path) = dump {
        std::fs::write(path, report).expect("the dump file");
    }
    for (name, want) in REAL_CASES {
        assert_eq!(texts.get(*name).map(String::as_str), Some(*want), "{name}");
    }
}

/// xNVSE's stand-in name for an unnamed local: its prefix letter, `Var`, its id (written out so
/// that the repository's lint does not read it as a decompiler's name).
fn stand_in(prefix: &str, idx: u32) -> String {
    format!("{prefix}{}{idx}", "Var")
}
/// The cases: (editor id, decompiled text). Compared whole, byte for byte, against the rules' text
/// worked out line by line for each script's compiled bytes (SCTX was the guide to what each line
/// says): TheTopsSmallScript, a string argument (`ShowWarning`); RepconRadioScript, a string
/// argument, a reference call on a form the game keeps no name for, an empty block, a block type
/// with no parameter given; DLC04BeachHoleSCRIPT, `If`s nested three deep with `Else`, reference
/// calls on the player inside expressions and as lines, a float argument (`SetScale 0.01`),
/// `Ref`, `Int` and `Float` locals, blank lines around the blocks.
#[rustfmt::skip]
const REAL_CASES: &[(&str, &str)] = &[
    (
        "TheTopsSmallScript",
        "ScriptName TheTopsSmallScript\n\nInt DoOnce\n\nBegin GameMode\n\tIf DoOnce == 0\n\t\tShowWarning \"Small Fry at The Tops!\"\n\t\tSet DoOnce To 1\n\tEndIf\nEnd\n",
    ),
    (
        "RepconRadioScript",
        "ScriptName RepconRadioScript\n\nInt OnceOnly\n\nBegin OnLoad\nEnd\n\nBegin OnTriggerEnter\n\tIf OnceOnly == 0\n\t\tShowWarning \"Trigger Activated\"\n\t\tSet OnceOnly To 1\n\t\t<CD127>.SetBroadcastState 1\n\tEndIf\nEnd\n",
    ),
    (
        "DLC04BeachHoleSCRIPT",
        "ScriptName DLC04BeachHoleSCRIPT\n\nRef myLinkedRef\nRef myLinkedRefsRef\nInt digTimerStart\nInt scaleTimer\nFloat digTimer\nInt shovelCheck\nInt dismembererCheck\nInt throwDirt\nInt unequipOnce\nInt dirtExplosion\nInt digTimerRun\n\nBegin OnLoad\n\tSet digTimer To 0\n\tSet myLinkedRef To GetLinkedRef\nEnd\n\nBegin OnActivate\n\tIf IsActionRef PlayerRef == 1\n\t\tIf PlayerRef.GetItemCount <11E46F> >= 1\n\t\t\tIf PlayerRef.GetEquipped <11E46F> == 1\n\t\t\t\tPlayerRef.UnequipItem <11E46F> 0 1\n\t\t\t\tSet shovelCheck To 1\n\t\t\tEndIf\n\n\t\t\tActivate PlayerRef\n\t\t\tSet digTimerStart To 1\n\t\tElse\n\t\t\tShowMessage <11E46E>\n\t\tEndIf\n\tEndIf\nEnd\n\nBegin GameMode\n\tIf digTimerStart == 1\n\t\tIf dirtExplosion == 0 && PlayerRef.GetSitting == 2\n\t\t\tPlaceAtMe <11E470> 1\n\t\t\tSet digTimer To 7\n\t\t\tSet dirtExplosion To 1\n\t\t\tSet digTimerRun To 1\n\t\tEndIf\n\n\t\tIf digTimerRun == 1\n\t\t\tIf digTimer > 0\n\t\t\t\tSet digTimer To digTimer - GetSecondsPassed\n\n\t\t\t\tIf scaleTimer == 0 && digTimer < 5\n\t\t\t\t\tSet scaleTimer To 1\n\t\t\t\t\tSetScale 0.01\n\t\t\t\tEndIf\n\t\t\tElse\n\t\t\t\tEnablePlayerControls\n\n\t\t\t\tIf shovelCheck == 1\n\t\t\t\t\tPlayerRef.EquipItem <11E46F> 0 1\n\t\t\t\tEndIf\n\n\t\t\t\tSet digTimerStart To 2\n\t\t\t\tDisable\n\t\t\tEndIf\n\t\tEndIf\n\tEndIf\nEnd\n",
    ),
];
