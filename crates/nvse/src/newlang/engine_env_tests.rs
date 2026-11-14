//! The game behind the new compiler, on the decompiler tests' engine (the table, the stand-in
//! event blocks `GameMode` and `MenuMode`, the script `Test` with its locals) and a few more
//! stand-ins of the exe's tables, from xNVSE 6.4.9's `Compiler/Utils.cpp` and `CommandTable.cpp`.

use super::*;
use crate::commands::test_support::SCRIPT;
use crate::decompile::tests::{engine as decompile_engine, text_at, FORM_A};
use crate::Nvse;
use std::cell::RefCell;

/// The tests' own memory: a heap and stand-in forms.
const AREA: u32 = 0x0500_0000;
const HEAP_TOP: u32 = AREA;
const HEAP: u32 = AREA + 0x1000;
const QUEST: u32 = AREA + 0x8000;
const REFR: u32 = AREA + 0x8100;
const BASE: u32 = AREA + 0x8200;

thread_local! {
    static PRINTED: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
}

fn engine() -> Engine {
    let mut e = decompile_engine();
    e.map(AREA, 0x1_0000);
    e.register(0x00ec_a7f4, |_, a| Ret {
        eax: u32::from((a[0] as u8).to_ascii_uppercase()),
        ..Ret::default()
    });
    e.register(compiler::FORM_HEAP_ALLOCATE, |e, a| {
        let at = e.mem.u32(HEAP_TOP).max(HEAP);
        e.mem
            .set_u32(HEAP_TOP, at + a[0].max(1).next_multiple_of(8));
        Ret {
            eax: at,
            ..Ret::default()
        }
    });
    e.register(compiler::FORM_HEAP_FREE, |_, _| Ret::default());
    e.register(0x0071_d0a0, |e, a| {
        let text = guest_text(e, e.mem.u32(a[2]));
        PRINTED.with(|p| p.borrow_mut().push(text));
        Ret::default()
    });
    // `IsStringInteger`: digits only.
    e.register(IS_STRING_INTEGER, |e, a| {
        let t = guest_text(e, a[0]);
        Ret {
            eax: u32::from(!t.is_empty() && t.iter().all(u8::is_ascii_digit)),
            ..Ret::default()
        }
    });
    // `__RTDynamicCast` for the stand-in forms: a quest and a base form are scriptable (their
    // component at +0x30), the reference is a reference, the script `Test` a script.
    e.register(0x00ec_43fb, |_, a| {
        let (object, to) = (a[0], a[3]);
        let eax = match (object, to) {
            (QUEST | BASE, rtti::TES_SCRIPTABLE_FORM) => object + 0x30,
            (REFR, RTTI_TES_OBJECT_REFR) => object,
            (SCRIPT, rtti::SCRIPT) => object,
            _ => 0,
        };
        Ret {
            eax,
            ..Ret::default()
        }
    });
    e.register(0x0071_b160, |_, _| Ret {
        eax: AREA + 0x9000,
        ..Ret::default()
    });
    // GetV's record carries its opcode, as the game's records do.
    let get_v = record_of(&e, 0x1002);
    e.mem.set_u32(get_v + 8, 0x1002);
    e.ext::<Nvse>().main_thread_recorded = true;
    PRINTED.with(|p| p.borrow_mut().clear());
    e
}

#[test]
fn event_blocks() {
    let mut e = engine();
    let mut env = engine_env(&mut e, 0);
    assert_eq!(
        env.block(b"menumode"),
        Some(BlockInfo {
            opcode: 1,
            params: vec![ParamInfo {
                name: Vec::new(),
                kind: 1,
                optional: true
            }]
        })
    );
    assert_eq!(env.block(b"GAMEMODE").map(|b| b.opcode), Some(0));
    assert_eq!(env.block(b"OnFoo"), None);
}

/// A command's record: its parse kind (the game's parsers and xNVSE's `Cmd_Default_Parse` are
/// the default, `Cmd_Call_Parse` its own), its parent plugin (none for the game's, xNVSE's own
/// info "NVSE" 6 for xNVSE's range), its return type.
#[test]
fn commands() {
    let mut e = engine();
    let mut env = engine_env(&mut e, 0);
    let get_v = env.command(b"getv").unwrap();
    assert_eq!(get_v.opcode, 0x1002);
    assert_eq!(get_v.long_name, b"GetV");
    assert_eq!(get_v.parse, Parse::Default);
    assert_eq!(get_v.plugin, None);
    assert_eq!(get_v.params.len(), 1);
    assert_eq!(get_v.return_type, tt::NUMBER);
    let let_cmd = env.command(b"Let").unwrap();
    assert_eq!(let_cmd.parse, Parse::Other);
    assert_eq!(let_cmd.plugin, Some((b"NVSE".to_vec(), 6)));
    assert_eq!(env.command(b"Call").unwrap().parse, Parse::Call);
    assert_eq!(env.command(b"ar_List").unwrap().return_type, tt::ARRAY);
    assert_eq!(env.command(b"ToString").unwrap().return_type, tt::STRING);
    // xNVSE's own default-parsed commands.
    assert_eq!(
        env.command(b"GetNVSEVersion").unwrap().parse,
        Parse::Default
    );
    assert_eq!(env.command(b"NoSuchCommand"), None);
}

#[test]
fn vanilla_enums() {
    let mut e = engine();
    // The sex names and the critical stage, alignment and equip type name tables.
    for page in [0x0104_a000, 0x0119_b000, 0x0118_6000] {
        e.map(page, 0x1000);
    }
    let male = text_at(&mut e, "Male");
    let female = text_at(&mut e, "Female");
    e.mem.set_u32(SEX_0, male);
    e.mem.set_u32(SEX_1, female);
    for (i, n) in [
        "CritStage1",
        "CritStage2",
        "CritStage3",
        "CritStage4",
        "CritStage5",
    ]
    .iter()
    .enumerate()
    {
        let t = text_at(&mut e, n);
        e.mem.set_u32(CRITICAL_STAGE_NAMES + 4 * i as u32, t);
    }
    let evil = text_at(&mut e, "Evil");
    e.mem.set_u32(ALIGNMENT_NAMES + 8, evil);
    let mut env = engine_env(&mut e, 0);
    assert_eq!(env.vanilla_enum(0x08, b"y"), (u32::from(b'Y'), 1));
    assert_eq!(env.vanilla_enum(0x08, b"w"), (u32::MAX, 2));
    assert_eq!(env.vanilla_enum(0x08, b"xy"), (u32::MAX, 2));
    assert_eq!(env.vanilla_enum(0x12, b"FEMALE"), (1, 2));
    assert_eq!(env.vanilla_enum(0x12, b"male"), (0, 2));
    assert_eq!(env.vanilla_enum(0x37, b"critstage3"), (2, 2));
    assert_eq!(env.vanilla_enum(0x37, b"Nope"), (u32::MAX, 2));
    assert_eq!(env.vanilla_enum(0x33, b"evil"), (2, 2));
    assert_eq!(env.vanilla_enum(0x34, b"evil"), (u32::MAX, 2));
    assert_eq!(env.vanilla_enum(0x20, b"weapon"), (40, 2));
    assert_eq!(env.vanilla_enum(0x20, b"Caravan Deck"), (117, 2));
    assert_eq!(env.vanilla_enum(0x1c, b"3"), (3, 2));
    assert_eq!(env.vanilla_enum(0x1c, b"7"), (u32::MAX, 2));
    assert_eq!(env.vanilla_enum(0x1c, b"x"), (u32::MAX, 2));
    // Another type: no enum.
    assert_eq!(env.vanilla_enum(0x01, b"3"), (u32::MAX, 2));
}

#[test]
fn form_parameter_types() {
    let mut e = engine();
    e.mem.set_u8(FORM_A + 4, 0x2a);
    let mut env = engine_env(&mut e, 0);
    assert!(env.form_matches_param(FORM_A, 0x19));
    assert!(env.form_matches_param(FORM_A, 0x22));
    assert!(env.form_matches_param(FORM_A, 0x23));
    assert!(!env.form_matches_param(FORM_A, 0x0e));
    // The run-time build's "non form list" asks for a form list.
    assert!(!env.form_matches_param(FORM_A, 0x35));
    assert!(!env.form_matches_param(0, 0x3d));
    assert!(env.form_matches_param(FORM_A, 0x3d));
    env.e.mem.set_u8(FORM_A + 4, 0x55);
    assert!(env.form_matches_param(FORM_A, 0x35));
    assert!(env.form_matches_param(FORM_A, 0x25));
    assert!(env.form_matches_param(FORM_A, 0x32));
    assert!(env.form_matches_param(REFR, 0x04));
    assert!(!env.form_matches_param(QUEST, 0x04));
}

#[test]
fn forms_and_their_scripts() {
    let mut e = engine();
    e.mem.set_u8(QUEST + 4, 0x47);
    e.mem.set_u32(QUEST + 0x34, SCRIPT);
    e.mem.set_u8(REFR + 4, 0x3a);
    e.mem.set_u32(REFR + 0x20, BASE);
    let mut env = engine_env(&mut e, SCRIPT);
    assert_eq!(env.form_script_variable(QUEST, b"I"), Some(Some((1, 2))));
    assert_eq!(env.form_script_variable(QUEST, b"nope"), Some(None));
    // The reference's base form has no script.
    assert_eq!(env.form_script_variable(REFR, b"i"), None);
    assert_eq!(env.script_form_type(SCRIPT), Some(0));
    assert_eq!(env.script_form_type(QUEST), None);
    assert_eq!(env.editor_id(SCRIPT), b"Test");
    // The ids xNVSE names itself.
    assert_eq!(env.editor_id(FORM_A), b"PlayerRef");
    assert_eq!(env.form_type(QUEST), 0x47);
    assert_eq!(env.form_by_id(0x14), FORM_A);
}

#[test]
fn the_scripts_variables() {
    let mut e = engine();
    let mut env = engine_env(&mut e, SCRIPT);
    assert_eq!(
        env.script_variables(),
        Some(vec![
            (b"x".to_vec(), 0),
            (b"i".to_vec(), 1),
            (b"rx".to_vec(), 0)
        ])
    );
    assert_eq!(env.script_type(), Some(0));
    env.set_script_variables(&[(b"a".to_vec(), 1, 1), (b"r".to_vec(), 4, 2)]);
    let e = &mut *env.e;
    let vars = compiler::list_items(e, SCRIPT + sc::VARS);
    let read: Vec<(u32, u8, Vec<u8>)> = vars
        .iter()
        .map(|&v| {
            (
                e.mem.u32(v),
                e.mem.u8(v + 0x10),
                compiler::string_cstr(e, v + 0x18),
            )
        })
        .collect();
    assert_eq!(read, [(1, 1, b"a".to_vec()), (2, 4, b"r".to_vec())]);
    let refs = compiler::list_items(e, SCRIPT + sc::REFS);
    assert_eq!(refs.len(), 1);
    assert_eq!(compiler::string_cstr(e, refs[0]), b"r");
    assert_eq!(e.mem.u32(refs[0] + 8), 0);
    assert_eq!(e.mem.u32(refs[0] + 0xc), 2);
    assert_eq!(e.mem.u32(SCRIPT + sc::VAR_COUNT), 2);
    assert_eq!(e.mem.u32(SCRIPT + sc::NUM_REFS), 1);
    // No script: nothing read or written.
    let mut env = engine_env(e, 0);
    assert_eq!(env.script_variables(), None);
    assert_eq!(env.script_type(), None);
}

/// Errors print as xNVSE's `ShowRuntimeError` with no script, held to its 0x800-byte buffer.
#[test]
fn errors_print_with_the_runtime_header() {
    let mut e = engine();
    let mut env = engine_env(&mut e, 0);
    env.error(b"[line 1:2] Expected ';'.");
    let long = vec![b'a'; 0x900];
    env.error(&long);
    let printed = PRINTED.with(|p| std::mem::take(&mut *p.borrow_mut()));
    assert_eq!(
        printed[0],
        b"Error in script 00000000 in mod Unknown or deleted script\n[line 1:2] Expected ';'."
    );
    let joined: usize = printed[1..]
        .iter()
        .map(|p| p.iter().filter(|&&c| c == b'a').count())
        .sum();
    assert_eq!(joined, 0x7ff);
}
