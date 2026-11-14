//! The new language's compile on the decompiler tests' engine, through the compile's seam
//! (`005AEBB1`, `HandleBeginCompile`): the script `Test` (locals `x`, `i`, `rx`) given a text in
//! the new language, compiled, finished, and read back by the decompiler; and a user function's
//! parameters from its text.

use super::*;
use crate::commands::test_support::SCRIPT;
use crate::compiler::{sb, sc};
use crate::decompile::tests::{text_at, FORM_A, VTABLE};
use crate::decompile::{decompile, parses};
use crate::newlang::engine_env::tests::{engine as env_engine, AREA, PRINTED, QUEST};
use crate::newlang::test_env::TestEnv;
use engine_core::prelude::*;
use std::cell::RefCell;

const BUF: u32 = AREA + 0xa000;
const TEXT: u32 = AREA + 0xb000;
const GLOBAL: u32 = AREA + 0x8300;
const CAPS: u32 = AREA + 0x8400;
const LOOSE: u32 = AREA + 0x8500;
/// The stand-in of the script's virtual functions the finish calls.
const SCRIPT_FN: u32 = 0x7eff_e200;
/// A stand-in function.
type Handler = fn(&mut Engine, &[u32]) -> Ret;

thread_local! {
    static CALLS: RefCell<Vec<(u32, Vec<u8>)>> = const { RefCell::new(Vec::new()) };
}

fn init_item(_: &mut Engine, _: &[u32]) -> Ret {
    record(0x88, Vec::new())
}
fn mark_for_deletion(_: &mut Engine, a: &[u32]) -> Ret {
    record(0xc4, vec![a[1] as u8])
}
fn set_altered(_: &mut Engine, a: &[u32]) -> Ret {
    record(0xc8, vec![a[1] as u8])
}
fn set_editor_id(e: &mut Engine, a: &[u32]) -> Ret {
    let name = crate::table::guest_text(e, a[1]);
    record(0x134, name)
}

/// `__RTDynamicCast` for the stand-in forms: the quest and the reference's base scriptable (their
/// component at +0x30), `PlayerRef` a reference, `Test` a script.
fn cast(_: &mut Engine, a: &[u32]) -> Ret {
    let (object, to) = (a[0], a[3]);
    let eax = match (object, to) {
        (QUEST, crate::commands::rtti::TES_SCRIPTABLE_FORM) => object + 0x30,
        (FORM_A, 0x0118_41cc) => object,
        (LOOSE, 0x0118_41cc) => object,
        (SCRIPT, crate::commands::rtti::SCRIPT) => object,
        (GLOBAL, crate::commands::rtti::TES_GLOBAL) => object,
        (QUEST, crate::commands::rtti::TES_QUEST) => object,
        _ => 0,
    };
    Ret {
        eax,
        ..Ret::default()
    }
}

fn record(slot: u32, data: Vec<u8>) -> Ret {
    CALLS.with(|c| c.borrow_mut().push((slot, data)));
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

/// The engine: the game behind the compiler's tests, the compile's seams, the script's virtual
/// functions, forms by editor id (`PlayerRef` a persistent reference, `GameHour` a global,
/// `MyQuest` a quest whose script is `Test`, `Caps001` a misc item).
fn engine() -> Engine {
    let mut e = env_engine();
    crate::compile_hooks::install(&mut e);
    let fns: [(u32, Handler); 4] = [
        (0x88, init_item),
        (0xc4, mark_for_deletion),
        (0xc8, set_altered),
        (0x134, set_editor_id),
    ];
    for (i, (slot, f)) in fns.into_iter().enumerate() {
        let at = SCRIPT_FN + 4 * i as u32;
        e.register(at, f);
        e.mem.set_u32(VTABLE + slot, at);
    }
    crate::decompile::tests::form(&mut e, GLOBAL, 0x0000_0200, Some("GameHour"));
    crate::decompile::tests::form(&mut e, QUEST, 0x0000_0201, Some("MyQuest"));
    crate::decompile::tests::form(&mut e, CAPS, 0x0000_000f, Some("Caps001"));
    e.mem.set_u8(CAPS + 4, 0x1f);
    crate::decompile::tests::form(&mut e, LOOSE, 0x0000_0202, Some("LooseRef"));
    e.mem.set_u8(LOOSE + 4, 0x3a);
    e.mem.set_u32(FORM_A + 8, 0x400);
    e.mem.set_u8(FORM_A + 4, 0x3a);
    e.mem.set_u8(GLOBAL + 4, 6);
    e.mem.set_u8(QUEST + 4, 0x47);
    e.mem.set_u32(QUEST + 0x34, SCRIPT);
    e.register(crate::compiler::FORM_BY_EDITOR_ID, |e, a| {
        let name = crate::table::guest_text(e, a[0]);
        let eax = match name.to_ascii_lowercase().as_slice() {
            b"playerref" => FORM_A,
            b"gamehour" => GLOBAL,
            b"myquest" => QUEST,
            b"caps001" => CAPS,
            b"looseref" => LOOSE,
            _ => 0,
        };
        Ret {
            eax,
            ..Ret::default()
        }
    });
    e.register(0x00ec_43fb, cast);
    CALLS.with(|c| c.borrow_mut().clear());
    e
}

/// The seam with `text` as the script buffer's text: its answer.
fn begin(e: &mut Engine, text: &str, partial: bool) -> Option<u32> {
    e.mem.write(BUF, &[0; sb::SIZE as usize]);
    e.mem.write(TEXT, text.as_bytes());
    e.mem.set_u8(TEXT + text.len() as u32, 0);
    e.mem.set_u32(BUF + sb::TEXT, TEXT);
    e.mem.set_u8(BUF + sb::PARTIAL, u8::from(partial));
    e.site(0x005a_ebb1, &[SCRIPT, BUF])
}

fn data(e: &Engine) -> Vec<u8> {
    let at = e.mem.u32(SCRIPT + sc::DATA);
    e.mem.bytes(at, e.mem.u32(SCRIPT + sc::DATA_LEN))
}

fn calls() -> Vec<(u32, Vec<u8>)> {
    CALLS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn printed() -> Vec<String> {
    PRINTED
        .with(|p| std::mem::take(&mut *p.borrow_mut()))
        .into_iter()
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .collect()
}

/// A script in the new language compiles at the seam, which answers 2 (compiled: the game's own
/// compile is passed over), after the script was finished as a plugin's would be.
#[test]
fn a_script_in_the_new_language_compiles_at_the_seam() {
    let mut e = engine();
    assert_eq!(
        begin(
            &mut e,
            "name NewName;\nint n;\nGameMode {\n  n = 5;\n}",
            false
        ),
        Some(2)
    );
    assert_eq!(e.mem.u8(SCRIPT + sc::COMPILED), 1);
    // The variables: the script's own three first, then `n`.
    let vars = crate::compiler::list_items(&e, SCRIPT + sc::VARS);
    let names: Vec<Vec<u8>> = vars
        .iter()
        .map(|&v| crate::compiler::string_cstr(&e, v + 0x18))
        .collect();
    assert_eq!(
        names,
        [b"x".to_vec(), b"i".to_vec(), b"rx".to_vec(), b"n".to_vec()]
    );
    assert_eq!(e.mem.u32(SCRIPT + sc::VAR_COUNT), 4);
    assert_eq!(e.mem.u32(SCRIPT + sc::UNUSED_VARS), 3);
    assert_eq!(e.mem.u32(SCRIPT + sc::NUM_REFS), 0);
    let mut want = vec![0x1d, 0, 0, 0];
    #[rustfmt::skip]
    want.extend_from_slice(&[
        0x76, 0x16, 0x11, 0x00, 0x02, 0x09, 0x00, b'S', 0x04, 0x00, b'n', b'v', b's', b'e',
        0x07, 0x00, b'L', 0x90, 0x00, 0x04, 0x06,
        0x10, 0x00, 0x06, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00,
        0x39, 0x15, 0x0c, 0x00, 0x01, 0x0b, 0x00, b'V', 0x01, 0x00, 0x00, 0x04, 0x00, b'B', 0x05, 0x00,
        0x11, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(data(&e), want);
    assert_eq!(
        calls(),
        [
            (0x134, b"NewName".to_vec()),
            (0x88, Vec::new()),
            (0xc4, vec![0]),
            (0xc8, vec![1])
        ]
    );
    // A partial script (the console's) keeps its editor id.
    assert_eq!(begin(&mut e, "name Other;\nGameMode {\n}", true), Some(2));
    assert!(calls().iter().all(|(slot, _)| *slot != 0x134));
}

/// A script that fails: the seam answers 0, the messages are printed, nothing is compiled.
#[test]
fn a_failed_script_answers_zero() {
    let mut e = engine();
    printed();
    assert_eq!(
        begin(&mut e, "name t;\nGameMode {\n  x = foo;\n}", false),
        Some(0)
    );
    assert_eq!(e.mem.u8(SCRIPT + sc::COMPILED), 0);
    let p = printed();
    assert_eq!(p.len(), 2);
    assert!(p[0].starts_with("Error in script 00000000 in mod Unknown or deleted script\n\r    | Unable to resolve identifier 'foo'."));
    assert_eq!(
        p[1],
        "Error in script 00000000 in mod Unknown or deleted script\n"
    );
    assert!(calls().is_empty());
    // `NAME` in any case goes to the new language; a parse error there.
    assert_eq!(begin(&mut e, "NAMEx t;", false), Some(0));
    assert_eq!(
        printed(),
        ["Error in script 00000000 in mod Unknown or deleted script\n[line 1:1] Expected 'name' as first statement of script."]
    );
}

/// The compiled script read back by the decompiler. The jump targets worked out by hand: the
/// block starts at 35 (25 bytes of name and requirement, 10 of `Begin`); each `Let` is 16 bytes,
/// each `Jmp_If_False` on a comparison 23, each `Jmp` 12, the `eval` of `n++` 26. So `if` jumps
/// to its `else` at 102 and the `if` part ends with a jump past it to 118, where the loop
/// starts; its inner `if` jumps past the `break` (a jump to the loop's end) to 202, the jump
/// back to 118 follows, and the loop ends at 214.
#[test]
fn compiled_scripts_decompile() {
    let mut e = engine();
    let text = "name Test;\nint n;\nGameMode {\n  n = 5;\n  if (n > 1) { n += 2; } else { n = 0; }\n  while (n < 10) { n++; if (n == 7) { break; } }\n}";
    assert_eq!(begin(&mut e, text, true), Some(2), "{:?}", printed());
    assert!(parses(&mut e, SCRIPT));
    let want = "ScriptName Test\n\nFloat x\nInt i\nFloat rx\nInt n\n\
        PluginVersion \"nvse\" 1.00926e+08\n\nBegin GameMode\n\tLet n := 5\n\
        \tJmp_If_False n > 1 102\n\tLet n += 2\n\tJmp 118\n\tLet n := 0\n\
        \tJmp_If_False n < 10 214\n\teval (n := n + 1) - 1\n\tJmp_If_False n == 7 202\n\
        \tJmp 214\n\tJmp 118\nEnd\n";
    assert_eq!(String::from_utf8_lossy(&decompile(&mut e, SCRIPT)), want);
    // More of the language, each line read back: a global, a method call, an array literal,
    // a for-in loop with `continue`, an interpolated string.
    let text = "name Test;\nGameMode {\n  float g = GameHour * 2;\n  player.GetV(3);\n  array a = [1, 2];\n  for (array v in a) { continue; }\n  string t = \"a${g}\";\n}";
    assert_eq!(begin(&mut e, text, true), Some(2), "{:?}", printed());
    assert!(parses(&mut e, SCRIPT));
    let out = String::from_utf8_lossy(&decompile(&mut e, SCRIPT)).into_owned();
    for line in [
        "\teval PlayerRef.(GetV (3))\n",
        "\tLet __temp_a_2 := (ar_List 1 2)\n",
        "\tForEach __temp_v_3 <- __temp_a_2\n\t\tcontinue\n\tloop\n",
        "\tLet __temp_t_4 := \"a\" + $__temp_g_1\n",
    ] {
        assert!(out.contains(line), "{line:?} in {out}");
    }
    // The global is `G` and its reference index (the first of the script's references).
    assert!(data(&e).windows(3).any(|w| w == [b'G', 1, 0]));
}

/// A user function's parameters from its text: its one block's (none for an event block); not
/// for two blocks or a text that fails to parse.
#[test]
fn user_function_parameters() {
    let mut env = TestEnv::default();
    assert_eq!(
        user_function_params(&mut env, b"name f;\nfn (int a, string b) {\n  return a;\n}"),
        Some(vec![(1, 1), (2, 2)])
    );
    assert_eq!(
        user_function_params(&mut env, b"name f;\nGameMode {}"),
        Some(Vec::new())
    );
    assert_eq!(
        user_function_params(&mut env, b"name f;\nGameMode {}\nMenuMode {}"),
        None
    );
    assert_eq!(
        user_function_params(&mut env, b"name f;\nfn (int a {}"),
        None
    );
    assert_eq!(
        env.take_errors(),
        [
            "[line 2:11] Expected ',' or ')'.",
            "[line 2:12] Expected variable declaration, block type (GameMode, MenuMode, ...), or 'fn'."
        ]
    );
    let _ = text_at;
}

/// xNVSE's own test scripts for the new compiler (`unit_tests/new_compiler/*.txt` of its
/// source, read from `NV_XNVSE_NEW_COMPILER_TESTS`, never copied into the repo): each compiles at
/// the seam on this engine and its data reads back. The engine has xNVSE's command table only
/// (no plugin's commands, a few stand-in forms): a script calling a plugin's command is reported,
/// not failed.
#[test]
#[ignore = "needs xNVSE's source (NV_XNVSE_NEW_COMPILER_TESTS: its unit_tests/new_compiler folder)"]
fn xnvses_own_test_scripts_compile() {
    let dir = std::env::var("NV_XNVSE_NEW_COMPILER_TESTS").expect("NV_XNVSE_NEW_COMPILER_TESTS");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    paths.sort();
    let mut failed = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).unwrap();
        let mut e = engine();
        printed();
        let r = begin(&mut e, &text, false);
        let messages = printed();
        let ok = r == Some(2) && parses(&mut e, SCRIPT);
        println!("{}: {r:?} parses {ok}", path.display());
        if ok {
            // The text names forms through functions this engine may not have: shown when it can.
            let text = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                decompile(&mut e, SCRIPT)
            }));
            match text {
                Ok(t) => println!("{}", String::from_utf8_lossy(&t)),
                Err(_) => println!("(its text names a form this engine cannot name)"),
            }
        } else {
            println!("{messages:#?}");
            let plugin_command = messages.iter().any(|m| m.contains("Not a valid command"));
            if !plugin_command {
                failed.push(path.display().to_string());
            }
        }
    }
    assert!(failed.is_empty(), "{failed:?}");
}

/// A reference that is not persistent, named in a script: xNVSE throws past every catch; the
/// compile fails here, with no message.
#[test]
fn a_reference_that_is_not_persistent_fails_the_compile() {
    let mut e = engine();
    printed();
    assert_eq!(
        begin(&mut e, "name t;\nGameMode {\n  LooseRef.GetV(1);\n}", true),
        Some(0)
    );
    assert!(printed().is_empty());
    assert_eq!(e.mem.u8(SCRIPT + sc::COMPILED), 0);
    // A persistent one compiles.
    assert_eq!(
        begin(&mut e, "name t;\nGameMode {\n  PlayerRef.GetV(1);\n}", true),
        Some(2)
    );
}
