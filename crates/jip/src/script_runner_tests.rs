//! Tests of JIP's script runner, from JIP LN NVSE 57.30's source (`JIPScriptRunner` in
//! `internal/jip_core.cpp`, `Script::Init`/`Create`/`Compile` in `nvse/GameScript.cpp`, the
//! listener's cases in `jip_nvse.cpp`): the game's compiler, constructors, `Script::Execute` and the
//! folder search are stand-ins that record what they were given; the mods' files seam is the
//! commands' test one.

use super::*;
use crate::commands::test_support::{engine, files};
use nvse::plugins::{msg, Message};
use std::cell::RefCell;

const AREA: u32 = 0x0600_0000;
const CONSOLE: u32 = AREA + 0x100;
const COMPILER: u32 = AREA + 0x200;
const VTABLE: u32 = AREA + 0x400;
const SET_EDITOR_ID_FN: u32 = 0x7eff_c000;
const DESTROY_FN: u32 = 0x7eff_c004;
/// A message string the stand-in compiler reports (`String`: data, length) and its text.
const STRING: u32 = AREA + 0x7f00;
const MESSAGE: u32 = AREA + 0x8000;
const CONSOLE_PRINT_FN: u32 = 0x0071_d0a0;

#[derive(Default)]
struct Log {
    /// Every call of interest, in order.
    calls: Vec<String>,
    /// The text of each script made, by its address.
    texts: Vec<(u32, String)>,
}

thread_local! {
    static LOG: RefCell<Log> = RefCell::new(Log::default());
}

fn with_log<T>(f: impl FnOnce(&mut Log) -> T) -> T {
    LOG.with(|l| f(&mut l.borrow_mut()))
}

fn note(s: String) {
    with_log(|l| l.calls.push(s));
}

fn calls() -> Vec<String> {
    with_log(|l| l.calls.clone())
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn text_of(script: u32) -> String {
    with_log(|l| {
        l.texts
            .iter()
            .rev()
            .find(|t| t.0 == script)
            .map(|t| t.1.clone())
            .unwrap_or_default()
    })
}

/// The stand-in compiler: records the buffer; compiles to 4 bytes of data unless the text holds
/// `bad` (fails) or `empty` (compiles to nothing).
fn compile_stand_in(e: &mut Engine, a: &[u32]) -> Ret {
    assert_eq!(a[0], COMPILER, "the console's compiler");
    let (script, buf) = (a[1], a[2]);
    let text_at = e.mem.u32(buf + BUFFER_TEXT);
    assert_eq!(
        e.mem.u32(script + SCRIPT_TEXT),
        text_at,
        "the script's own text"
    );
    let text = String::from_utf8_lossy(&nvse::table::guest_text(e, text_at)).into_owned();
    let name = String::from_utf8_lossy(&nvse::table::guest_text(e, e.mem.u32(buf + BUFFER_NAME)))
        .into_owned();
    note(format!(
        "compile {name} mode {} partial {} current {}",
        e.mem.u32(buf + BUFFER_RUNTIME_MODE),
        e.mem.u8(buf + BUFFER_PARTIAL),
        e.mem.u32(buf + BUFFER_CURRENT_SCRIPT) == script
    ));
    with_log(|l| l.texts.push((script, text.clone())));
    if text.contains("bad") {
        // The game's ScriptError: its message string released at 005aeb66, read by the seam.
        let message = format!(
            "SCRIPTS: Script '{name}', line 1:
bad"
        );
        e.mem.write(MESSAGE, message.as_bytes());
        e.mem.set_u32(STRING, MESSAGE);
        e.mem.set_u16(STRING + 4, message.len() as u16);
        let _ = e.site(0x005a_eb66, &[STRING]);
        return eax(0);
    }
    let length = if text.contains("empty") { 0 } else { 4 };
    e.mem.set_u32(script + SCRIPT_DATA_LENGTH, length);
    eax(1)
}

fn runner_engine() -> Engine {
    LOG.with(|l| *l.borrow_mut() = Log::default());
    files::fresh();
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    e.mem.set_u32(CONSOLE, COMPILER);
    e.mem.set_u32(VTABLE + SET_EDITOR_ID, SET_EDITOR_ID_FN);
    e.mem.set_u32(VTABLE + DESTROY, DESTROY_FN);
    e.register(CONSOLE_MANAGER, |_, a| {
        assert_eq!(a[0], 1, "made if missing");
        eax(CONSOLE)
    });
    e.register(SCRIPT_COMPILE, compile_stand_in);
    e.register(BUFFER_CONSTRUCTOR, |_, a| {
        note("ctor-buffer".into());
        eax(a[0])
    });
    e.register(BUFFER_DESTRUCTOR, |_, _| {
        note("dtor-buffer".into());
        Ret::default()
    });
    e.register(SCRIPT_CONSTRUCTOR, |e, a| {
        e.mem.set_u32(a[0], VTABLE);
        e.mem.set_u32(a[0] + SCRIPT_DATA_LENGTH, 0);
        e.mem.set_u32(a[0] + SCRIPT_TEXT, 0);
        note("ctor-script".into());
        eax(a[0])
    });
    e.register(SCRIPT_DESTRUCTOR, |_, _| {
        note("dtor-script".into());
        Ret::default()
    });
    e.register(MARK_AS_TEMPORARY, |_, _| {
        note("temporary".into());
        Ret::default()
    });
    e.register(SET_EDITOR_ID_FN, |e, a| {
        let name = nvse::table::guest_text(e, a[1]);
        note(format!("editor-id {}", String::from_utf8_lossy(&name)));
        eax(1)
    });
    e.register(DESTROY_FN, |_, a| {
        note(format!("destroy {}", a[1]));
        Ret::default()
    });
    e.register(SCRIPT_EXECUTE, |e, a| {
        assert_eq!(&a[2..4], &[0, 0], "no list, no container");
        let text = text_of(a[0]);
        if a[1] == 0 {
            note(format!("run {text} {}", a[4]));
        } else {
            note(format!("run {text} {} on {:x}", a[4], a[1]));
        }
        // A script line `register <path>`: RegisterSRScript's step, as a gr_ file calls it.
        if let Some(path) = text.strip_prefix("register ") {
            let kept = register(e, path.as_bytes());
            note(format!("register {path} {kept}"));
        }
        eax(1)
    });
    e.register(CONSOLE_PRINT_FN, |e, a| {
        let line = nvse::table::guest_text(e, e.mem.u32(a[2]));
        note(format!("console {}", String::from_utf8_lossy(&line)));
        eax(0)
    });
    e
}

/// The folder's entries in the seam: each name's text as a file (a `/` at the end: a folder; no text:
/// not there).
fn folder(entries: &[(&str, Option<&str>)]) {
    for (name, text) in entries {
        let path = format!(
            "data\\nvse\\plugins\\scripts\\{}",
            name.trim_end_matches('/')
        );
        if name.ends_with('/') {
            files::folder(&path);
        } else if let Some(text) = text {
            files::put(&path, text.as_bytes());
        }
    }
}

fn runs() -> Vec<String> {
    calls()
        .into_iter()
        .filter(|c| c.starts_with("run "))
        .collect()
}

fn message(e: &mut Engine, kind: u32, data: u32) {
    crate::cosave::listen(e, &Message { kind, data, len: 0 });
}

#[test]
fn the_names_moments_are_the_seven_prefixes_without_case() {
    assert_eq!(RunOn::of(b"gl_a.txt"), Some(RunOn::LoadGame));
    assert_eq!(RunOn::of(b"XM_a.txt"), Some(RunOn::ExitToMainMenu));
    assert_eq!(RunOn::of(b"gn_a.txt"), Some(RunOn::NewGame));
    assert_eq!(RunOn::of(b"Ln_ISControl.txt"), Some(RunOn::LoadOrNewGame));
    assert_eq!(RunOn::of(b"gr_a.txt"), Some(RunOn::RestartGame));
    assert_eq!(RunOn::of(b"gs_a.txt"), Some(RunOn::SaveGame));
    assert_eq!(RunOn::of(b"gx_a.txt"), Some(RunOn::ExitGame));
    // The letters the other way round, no `_`, too short, another prefix.
    assert_eq!(RunOn::of(b"lg_a.txt"), None);
    assert_eq!(RunOn::of(b"gl-a.txt"), None);
    assert_eq!(RunOn::of(b"gl"), None);
    assert_eq!(RunOn::of(b"ab_a.txt"), None);
}

#[test]
fn deferred_start_runs_gr_files_and_keeps_the_others() {
    let mut e = runner_engine();
    folder(&[
        ("gr_start.txt", Some("print start")),
        ("ln_mod.txt", Some("print loadnew")),
        ("sub.txt/", None),
        ("readme.txt", Some("print never")),
        ("gs_save.txt", Some("print saved")),
        ("gl_bad.txt", Some("bad line")),
        ("gn_empty.txt", Some("empty")),
    ]);
    init_files(&mut e);
    let c = calls();
    // The gr_ file: a temporary script made, marked, compiled in the editor's mode as a partial
    // script, run once with the default flag, destroyed; no editor id.
    let start = c.iter().position(|s| s == "run print start 1").unwrap();
    assert_eq!(
        c[start - 5..=start + 1],
        [
            "ctor-script",
            "temporary",
            "ctor-buffer",
            "compile gr_start.txt mode 0 partial 1 current true",
            "dtor-buffer",
            "run print start 1",
            "dtor-script",
        ]
    );
    // Only the gr_ file ran.
    assert_eq!(runs(), ["run print start 1"]);
    // Kept: the ln_ and gs_ files, named by their files; the bad one destroyed, the empty one too;
    // the plain file never compiled.
    assert!(c.contains(&"editor-id ln_mod.txt".to_string()));
    assert!(c.contains(&"editor-id gs_save.txt".to_string()));
    assert_eq!(c.iter().filter(|s| *s == "destroy 1").count(), 2);
    assert!(!c.iter().any(|s| s.contains("readme")));
    let kept: Vec<Option<RunOn>> = e
        .ext::<Jip>()
        .script_runner
        .cached
        .iter()
        .map(|(_, k)| k.run_on)
        .collect();
    assert_eq!(kept.len(), 2);
    assert!(kept.contains(&Some(RunOn::LoadOrNewGame)) && kept.contains(&Some(RunOn::SaveGame)));
}

/// The runs `f` made, in name order.
fn runs_of(e: &mut Engine, f: impl FnOnce(&mut Engine)) -> Vec<String> {
    let before = runs().len();
    f(e);
    let mut new = runs()[before..].to_vec();
    new.sort();
    new
}

fn moments_folder(e: &mut Engine) {
    folder(&[
        ("gl_load.txt", Some("print load")),
        ("gn_new.txt", Some("print new")),
        ("ln_both.txt", Some("print both")),
        ("gs_save.txt", Some("print save")),
        ("xm_menu.txt", Some("print menu")),
        ("gx_exit.txt", Some("print exit")),
    ]);
    init_files(e);
    assert!(runs().is_empty());
}

#[test]
fn each_moment_runs_its_files() {
    let mut e = runner_engine();
    moments_folder(&mut e);
    let at = |e: &mut Engine, first, second| runs_of(e, |e| run(e, first, second));
    assert_eq!(
        at(&mut e, RunOn::LoadGame, Some(RunOn::LoadOrNewGame)),
        ["run print both 0", "run print load 0"]
    );
    assert_eq!(
        at(&mut e, RunOn::NewGame, Some(RunOn::LoadOrNewGame)),
        ["run print both 0", "run print new 0"]
    );
    assert_eq!(at(&mut e, RunOn::SaveGame, None), ["run print save 0"]);
    assert_eq!(
        at(&mut e, RunOn::ExitToMainMenu, None),
        ["run print menu 0"]
    );
    assert_eq!(at(&mut e, RunOn::ExitGame, None), ["run print exit 0"]);
}

/// The listener's cases that need no loaded game in these tests: a save, the two exits. (The load,
/// new game and main menu cases call the same `run` after their saved data's steps, which need a
/// game: `cosave_tests` covers those steps.)
#[test]
fn the_listener_runs_them_at_the_save_and_the_exits() {
    let mut e = runner_engine();
    moments_folder(&mut e);
    assert_eq!(
        runs_of(&mut e, |e| message(e, msg::SAVE_GAME, 0)),
        ["run print save 0"]
    );
    assert_eq!(
        runs_of(&mut e, |e| message(e, msg::EXIT_GAME, 0)),
        ["run print exit 0"]
    );
    assert_eq!(
        runs_of(&mut e, |e| message(e, msg::EXIT_GAME_CONSOLE, 0)),
        ["run print exit 0"]
    );
}

#[test]
fn a_name_kept_already_keeps_its_first_script() {
    let mut e = runner_engine();
    folder(&[("gl_same.txt", Some("print first"))]);
    init_files(&mut e);
    // The same name again (here a second start: the seam lists one spelling of a name).
    init_files(&mut e);
    // Both compiled (the second stays made, as in JIP); the first kept.
    let made: Vec<u32> = with_log(|l| l.texts.iter().map(|t| t.0).collect());
    assert_eq!(made.len(), 2);
    let kept: Vec<u32> = e
        .ext::<Jip>()
        .script_runner
        .cached
        .iter()
        .map(|(_, k)| k.script)
        .collect();
    assert_eq!(kept, [made[0]]);
}

#[test]
fn the_file_is_read_through_the_mod_files_seam_up_to_the_buffer() {
    let mut e = runner_engine();
    let long = format!("print long{}", "x".repeat(STR_BUFFER_SIZE));
    folder(&[
        ("gl_long.txt", Some(&long)),
        ("gl_nul.txt", Some("print a\0print b")),
    ]);
    init_files(&mut e);
    run(&mut e, RunOn::LoadGame, None);
    let r = runs();
    assert_eq!(r.len(), 2);
    let long_run = r.iter().find(|s| s.contains("long")).unwrap();
    // `run ` + the text + ` 0`: the text cut at STR_BUFFER_SIZE - 1 bytes.
    assert_eq!(long_run.len(), 4 + STR_BUFFER_SIZE - 1 + 2);
    // The compile reads up to the first NUL.
    assert!(r.contains(&"run print a 0".to_string()));
}

#[test]
fn nothing_runs_while_jip_answers_not_installed() {
    let mut e = runner_engine();
    folder(&[
        ("gr_start.txt", Some("print start")),
        ("gl_load.txt", Some("print load")),
    ]);
    e.ext::<Jip>().answers_installed = false;
    init_files(&mut e);
    run(&mut e, RunOn::LoadGame, None);
    assert!(calls().is_empty());
}

#[test]
fn no_folder_no_files() {
    let mut e = runner_engine();
    init_files(&mut e);
    assert!(e.ext::<Jip>().script_runner.cached.is_empty());
}

#[test]
fn register_keeps_a_file_below_the_folder_once() {
    let mut e = runner_engine();
    files::put("data\\nvse\\plugins\\scripts\\mymod\\gl_x.txt", b"print x");
    files::put("data\\nvse\\plugins\\scripts\\mymod\\gr_y.txt", b"print y");
    assert!(
        !register(&mut e, b"mymod/gl_x.txt"),
        "outside the deferred start"
    );
    e.ext::<Jip>().script_runner.init_in_progress = 1;
    assert!(register(&mut e, b"mymod/gl_x.txt"));
    assert!(!register(&mut e, b"mymod\\gl_x.txt"), "kept already");
    assert!(!register(&mut e, b"mymod/gr_y.txt"), "gr_ is never kept");
    assert!(!register(&mut e, b"gl_x.txt"), "no separator");
    assert!(!register(&mut e, b"mymod/gl_none.txt"), "no file");
    run(&mut e, RunOn::LoadGame, None);
    assert_eq!(runs(), ["run print x 0"]);
}

/// A gr_ file registers a file of a sub-folder while the files compile (RegisterSRScript); the
/// same call after the deferred start keeps nothing. The command answers 0 after the start and 1
/// during one.
#[test]
fn a_gr_file_registers_a_sub_folder_file_during_the_start_only() {
    let mut e = runner_engine();
    files::put(
        "data\\nvse\\plugins\\scripts\\mymod\\ln_main.txt",
        b"print main",
    );
    folder(&[("gr_mymod.txt", Some("register MyMod\\ln_Main.txt"))]);
    init_files(&mut e);
    assert!(calls().contains(&"register MyMod\\ln_Main.txt true".to_string()));
    assert_eq!(e.ext::<Jip>().script_runner.init_in_progress, 0);
    run(&mut e, RunOn::NewGame, Some(RunOn::LoadOrNewGame));
    assert!(runs().contains(&"run print main 0".to_string()));
    // After the start: refused, by the step and by the command.
    files::put(
        "data\\nvse\\plugins\\scripts\\mymod\\gl_late.txt",
        b"print late",
    );
    assert!(!register(&mut e, b"MyMod\\gl_Late.txt"));
    let path = crate::test_support::s(b"MyMod\\gl_Late.txt");
    assert_eq!(
        crate::test_support::call(&mut e, 0x2938, std::slice::from_ref(&path)).1,
        0.0
    );
    // The command during a start (as a gr_ file would call it): 1.
    e.ext::<Jip>().script_runner.init_in_progress = 1;
    assert_eq!(crate::test_support::call(&mut e, 0x2938, &[path]).1, 1.0);
}

/// A file that does not compile: the compiler's message read at the seam while the files
/// compile (JIP's console line once per start), and the file passed over; a compile error outside
/// the start is not read.
#[test]
fn a_file_that_does_not_compile_is_said() {
    let mut e = runner_engine();
    folder(&[
        ("gl_bad1.txt", Some("bad one")),
        ("gl_bad2.txt", Some("bad two")),
        ("gl_good.txt", Some("print good")),
    ]);
    init_files(&mut e);
    let consoles: Vec<String> = calls()
        .into_iter()
        .filter(|c| c.starts_with("console "))
        .collect();
    assert_eq!(
        consoles,
        [format!("console {CONSOLE_LINE}")],
        "once per start"
    );
    assert_eq!(e.ext::<Jip>().script_runner.init_in_progress, 0);
    assert_eq!(e.ext::<Jip>().script_runner.cached.len(), 1);
    let before = calls().len();
    assert_eq!(create(&mut e, b"bad again", b"x.txt"), 0);
    assert!(!calls()[before..].iter().any(|c| c.starts_with("console ")));
}

/// The runner's step for each message (what `cosave::listen` calls after each case's own steps):
/// a load that succeeded runs gl_ and ln_ files, a failed one nothing; a new game gn_ and ln_; a
/// save gs_; the main menu xm_; both exits gx_; the deferred start compiles the folder and runs
/// gr_ files; other messages nothing.
#[test]
fn each_message_runs_exactly_its_files() {
    let mut e = runner_engine();
    folder(&[
        ("gl_load.txt", Some("print load")),
        ("gn_new.txt", Some("print new")),
        ("ln_both.txt", Some("print both")),
        ("gs_save.txt", Some("print save")),
        ("xm_menu.txt", Some("print menu")),
        ("gx_exit.txt", Some("print exit")),
        ("gr_start.txt", Some("print start")),
    ]);
    let step = |e: &mut Engine, kind: u32, data: u32| runs_of(e, |e| on_message(e, kind, data));
    assert_eq!(step(&mut e, msg::DEFERRED_INIT, 0), ["run print start 1"]);
    assert_eq!(e.ext::<Jip>().script_runner.cached.len(), 6);
    assert_eq!(
        step(&mut e, msg::POST_LOAD_GAME, 1),
        ["run print both 0", "run print load 0"]
    );
    assert!(
        step(&mut e, msg::POST_LOAD_GAME, 0).is_empty(),
        "a failed load"
    );
    assert!(
        step(&mut e, msg::POST_LOAD_GAME, 0x100).is_empty(),
        "the low byte"
    );
    assert_eq!(
        step(&mut e, msg::NEW_GAME, 0),
        ["run print both 0", "run print new 0"]
    );
    assert_eq!(step(&mut e, msg::SAVE_GAME, 0), ["run print save 0"]);
    assert_eq!(
        step(&mut e, msg::EXIT_TO_MAIN_MENU, 0),
        ["run print menu 0"]
    );
    assert_eq!(step(&mut e, msg::EXIT_GAME, 0), ["run print exit 0"]);
    assert_eq!(
        step(&mut e, msg::EXIT_GAME_CONSOLE, 0),
        ["run print exit 0"]
    );
    assert!(step(&mut e, msg::PRE_LOAD_GAME, 0).is_empty());
    assert!(step(&mut e, msg::MAIN_GAME_LOOP, 0).is_empty());
}

/// The kept scripts run in JIP's map order (`UnorderedMap<const char*>`, 8 buckets: the bucket
/// of `StrHashCI(name) & 7`, bucket by bucket, each chain newest first), not the folder's.
#[test]
fn the_run_order_is_jips_map_order() {
    let mut e = runner_engine();
    let names = [
        "gl_a.txt", "gl_b.txt", "gl_c.txt", "gl_d.txt", "gl_e.txt", "gl_f.txt",
    ];
    let texts: Vec<String> = names.iter().map(|n| format!("print {n}")).collect();
    let entries: Vec<(&str, Option<&str>)> = names
        .iter()
        .zip(&texts)
        .map(|(n, t)| (*n, Some(t.as_str())))
        .collect();
    folder(&entries);
    init_files(&mut e);
    let before = runs().len();
    run(&mut e, RunOn::LoadGame, None);
    let order: Vec<String> = runs()[before..].to_vec();
    let mut expected: Vec<(u32, usize, &str)> = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            (
                crate::containers::str_hash_ci(n.as_bytes()) & 7,
                usize::MAX - i,
                *n,
            )
        })
        .collect();
    expected.sort();
    let expected: Vec<String> = expected
        .iter()
        .map(|(_, _, n)| format!("run print {n} 0"))
        .collect();
    assert_eq!(order, expected);
    // Not merely the folder's order (the test means something only if they differ).
    let folder_order: Vec<String> = names.iter().map(|n| format!("run print {n} 0")).collect();
    assert_ne!(order, folder_order);
}

// ---- the commands on the runner: Console, RunScriptSnippet, RunBatchScript ----

const PLAYER_REF: u32 = AREA + 0xa000;
const CALLER: u32 = AREA + 0xa100;
const XSCRIPT: u32 = AREA + 0xa200;

fn command_engine() -> Engine {
    let mut e = runner_engine();
    crate::commands::test_support::extra_stand_ins(&mut e);
    if !e.mem.is_mapped(0x011d_e000) {
        e.map(0x011d_e000, 0x1000);
    }
    e.mem.set_u32(0x011d_ea3c, PLAYER_REF);
    e
}

fn cmd(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    crate::commands::test_support::call_on(e, opcode, this, args).1
}

fn s(t: &[u8]) -> Vec<u8> {
    crate::commands::test_support::s(t)
}

fn n(v: i32) -> Vec<u8> {
    crate::commands::test_support::n(v)
}

/// `Console "text"`: compiled as "Console" on a temporary script and run once: 1; nothing kept.
#[test]
fn console_runs_a_line_once() {
    let mut e = command_engine();
    assert_eq!(cmd(&mut e, 0x2326, 0, &[s(b"set x to 1")]), 1.0);
    assert_eq!(runs(), vec!["run set x to 1 1".to_string()]);
    assert!(calls()
        .iter()
        .any(|c| c.starts_with("compile Console mode 0 partial 1")));
    assert!(calls().iter().any(|c| c == "dtor-script"));
    assert_eq!(
        cmd(&mut e, 0x2326, 0, &[s(b"bad line")]),
        0.0,
        "does not compile: 0"
    );
    assert_eq!(runs().len(), 1);
    assert_eq!(e.ext::<Jip>().script_runner.cached.len(), 0);
}

/// `RunScriptSnippet 0 "text"`: line feeds made carriage returns, a new script "RunScriptSnippet"
/// run on the caller (the player with none), then destroyed.
#[test]
fn a_snippet_runs_now_on_the_caller_and_is_destroyed() {
    let mut e = command_engine();
    cmd(&mut e, 0x280f, 0, &[n(0), s(b"a\nb")]);
    assert_eq!(runs(), vec![format!("run a\rb 1 on {PLAYER_REF:x}")]);
    let c = calls();
    assert!(c.iter().any(|c| c == "editor-id RunScriptSnippet"));
    assert_eq!(c.last().unwrap(), "destroy 1", "destroyed after the run");
    cmd(&mut e, 0x280f, CALLER, &[n(0), s(b"x")]);
    assert_eq!(runs().last().unwrap(), &format!("run x 1 on {CALLER:x}"));
    let before = calls().len();
    cmd(&mut e, 0x280f, 0, &[n(0), s(b"bad")]);
    assert!(
        !calls()[before..].iter().any(|c| c.starts_with("run ")),
        "not compiled: nothing"
    );
}

/// With a delay: run once by the main loop after that many frames, the script kept (JIP never
/// destroys it).
#[test]
fn a_delayed_snippet_runs_after_its_frames() {
    let mut e = command_engine();
    cmd(&mut e, 0x280f, CALLER, &[n(3), s(b"later")]);
    assert!(runs().is_empty());
    crate::main_loop::cycle(&mut e);
    crate::main_loop::cycle(&mut e);
    assert!(runs().is_empty(), "two frames: not yet");
    crate::main_loop::cycle(&mut e);
    assert_eq!(runs(), vec![format!("run later 1 on {CALLER:x}")]);
    crate::main_loop::cycle(&mut e);
    assert_eq!(runs().len(), 1, "once");
    assert!(
        !calls().iter().any(|c| c.starts_with("destroy")),
        "kept, as JIP"
    );
}

/// The caller's own script's event list is taken away while the snippet runs and put back.
#[test]
fn the_callers_event_list_is_set_aside_for_the_run() {
    let mut e = command_engine();
    let list = CALLER + 0x44;
    crate::commands::test_support::put_extra(&mut e, list, XSCRIPT, 0x0d, 0);
    e.mem.set_u32(XSCRIPT + 0x10, 0xe7e7);
    e.register(SCRIPT_EXECUTE, |e, a| {
        note(format!("events {:x}", e.mem.u32(XSCRIPT + 0x10)));
        let _ = a;
        eax(1)
    });
    cmd(&mut e, 0x280f, CALLER, &[n(0), s(b"x")]);
    assert!(
        calls().contains(&"events 0".to_string()),
        "none while it runs"
    );
    assert_eq!(e.mem.u32(XSCRIPT + 0x10), 0xe7e7, "put back");
}

/// `RunBatchScript path`: the file read and compiled once under its name (after the last `\`,
/// `/` made `\`), kept by the runner, run on the caller each call: 1. A missing file: 0.
#[test]
fn a_batch_script_is_compiled_once_and_kept() {
    let mut e = command_engine();
    files::put(r"data\batch\b.txt", b"do it");
    assert_eq!(cmd(&mut e, 0x2834, 0, &[s(b"Data/batch/b.txt")]), 1.0);
    assert_eq!(runs(), vec![format!("run do it 1 on {PLAYER_REF:x}")]);
    assert!(calls().iter().any(|c| c == "editor-id b.txt"));
    let compiles = |c: &[String]| c.iter().filter(|c| c.starts_with("compile")).count();
    assert_eq!(compiles(&calls()), 1);
    assert_eq!(
        cmd(&mut e, 0x2834, CALLER, &[s(b"other/B.TXT")]),
        1.0,
        "the name kept, any case"
    );
    assert_eq!(compiles(&calls()), 1, "not compiled again");
    assert_eq!(runs().len(), 2);
    let kept = e
        .ext::<Jip>()
        .script_runner
        .cached
        .get(&Name(b"b.txt".to_vec()))
        .copied();
    assert_eq!(kept.map(|k| k.run_on), Some(None), "no moment");
    assert_eq!(cmd(&mut e, 0x2834, 0, &[s(b"Data/batch/none.txt")]), 0.0);
}

/// The batch script belongs to the calling script's file: its `mods` list (+0x10) that file.
#[test]
fn a_batch_script_belongs_to_the_callers_file() {
    use crate::commands::test_support::{game_globals, loaded_mod, FORMS, SCRIPT};
    let mut e = command_engine();
    game_globals(&mut e);
    let info = FORMS + 0x9_0000;
    loaded_mod(&mut e, 3, info, b"Mod.esp");
    e.mem.set_u32(SCRIPT + 0x10, info);
    e.mem.set_u32(SCRIPT + 0x14, 0);
    files::put("c.txt", b"go");
    assert_eq!(cmd(&mut e, 0x2834, 0, &[s(b"c.txt")]), 1.0);
    let made = e
        .ext::<Jip>()
        .script_runner
        .cached
        .get(&Name(b"c.txt".to_vec()))
        .unwrap()
        .script;
    assert_eq!((e.mem.u32(made + 0x10), e.mem.u32(made + 0x14)), (info, 0));
}

/// A delayed `RunScriptSnippet` at a load (JIP's `CleanMLCallbacks`): its script destroyed and its
/// callback gone, so it never runs on a reference the load may have freed.
#[test]
fn a_load_cancels_a_delayed_snippet() {
    let mut e = command_engine();
    cmd(&mut e, 0x280f, CALLER, &[n(3), s(b"later")]);
    crate::main_loop::clean(&mut e);
    let destroys = || calls().iter().filter(|c| c.starts_with("destroy")).count();
    assert_eq!(destroys(), 1, "destroyed at the load");
    for _ in 0..4 {
        crate::main_loop::cycle(&mut e);
    }
    assert!(runs().is_empty(), "never run");
    assert_eq!(destroys(), 1, "once");
    assert!(e.ext::<Jip>().main_loop.callbacks.is_empty());
}

thread_local! {
    static DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static ANSWERS: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
    static REENTER: RefCell<(u32, Vec<Vec<u8>>)> = const { RefCell::new((0, Vec::new())) };
}

/// The script run re-enters `opcode` with `args` (a batch file or snippet that runs itself), up
/// to 100 deep (the stand-in's own stop).
fn reentering(e: &mut Engine, opcode: u32, args: Vec<Vec<u8>>) {
    DEPTH.with(|d| d.set(0));
    ANSWERS.with(|a| a.borrow_mut().clear());
    REENTER.with(|r| *r.borrow_mut() = (opcode, args));
    e.register(SCRIPT_EXECUTE, |e, _| {
        note("run".into());
        let depth = DEPTH.with(|d| {
            d.set(d.get() + 1);
            d.get()
        });
        if depth < 100 {
            let (opcode, args) = REENTER.with(|r| r.borrow().clone());
            let answer = cmd(e, opcode, 0, &args);
            ANSWERS.with(|a| a.borrow_mut().push(answer));
        }
        DEPTH.with(|d| d.set(d.get() - 1));
        Ret::default()
    });
}

fn run_count() -> usize {
    calls().iter().filter(|c| *c == "run").count()
}

/// A batch file that runs itself: with the fixes row on, 64 nested runs and the 65th call answers
/// 0; with it off, JIP's way (no cap: here the stand-in's 100).
#[test]
fn a_batch_script_running_itself_stops_at_64_with_the_fixes_row() {
    let mut e = command_engine();
    files::put("loop.txt", b"again");
    nvse::events::set_error_guard(&mut e, true);
    reentering(&mut e, 0x2834, vec![s(b"loop.txt")]);
    cmd(&mut e, 0x2834, 0, &[s(b"loop.txt")]);
    assert_eq!(run_count(), 64);
    let answers = ANSWERS.with(|a| a.borrow().clone());
    assert_eq!(answers.first(), Some(&0.0), "the innermost call: 0");
    assert_eq!(answers.iter().filter(|a| **a == 0.0).count(), 1);
    assert_eq!(e.ext::<Jip>().script_runner.nesting, 0, "counted back down");
    // Off: no cap.
    let mut e = command_engine();
    files::put("loop.txt", b"again");
    reentering(&mut e, 0x2834, vec![s(b"loop.txt")]);
    cmd(&mut e, 0x2834, 0, &[s(b"loop.txt")]);
    assert_eq!(run_count(), 100);
}

/// A snippet that runs itself with no delay: 64 nested runs with the fixes row on; every script
/// made is still destroyed (the skipped one too).
#[test]
fn a_snippet_running_itself_stops_at_64_with_the_fixes_row() {
    let mut e = command_engine();
    nvse::events::set_error_guard(&mut e, true);
    reentering(&mut e, 0x280f, vec![n(0), s(b"again")]);
    cmd(&mut e, 0x280f, 0, &[n(0), s(b"again")]);
    assert_eq!(run_count(), 64);
    let destroys = calls().iter().filter(|c| c.starts_with("destroy")).count();
    assert_eq!(destroys, 65);
    assert_eq!(e.ext::<Jip>().script_runner.nesting, 0);
}

/// The name test: `.txt` at the end, any case.
#[test]
fn the_name_test_is_a_txt_ending_in_any_case() {
    assert!(is_txt(b"GS_UPPER.TXT"));
    assert!(is_txt(b"gl_a.Txt"));
    assert!(is_txt(b".txt"));
    assert!(!is_txt(b"gr_backup.txt.bak"));
    assert!(!is_txt(b"gl_cfg.ini"));
    assert!(!is_txt(b"txt"));
    assert!(!is_txt(b"gl_atxt"));
}

/// Only names ending in `.txt` are taken: a `.txt.bak` or an `.ini` never compiles or runs; a
/// `.TXT` is kept and runs, named with its own spelling (the files stand-in lists each name as it
/// was put, as Windows does).
#[test]
fn only_txt_names_are_taken() {
    let mut e = runner_engine();
    folder(&[
        ("gr_backup.txt.bak", Some("print backup")),
        ("gl_cfg.ini", Some("print cfg")),
        ("GS_UPPER.TXT", Some("print upper")),
    ]);
    init_files(&mut e);
    let c = calls();
    assert!(!c.iter().any(|s| s.contains("backup") || s.contains("cfg")));
    assert!(c.contains(&"editor-id GS_UPPER.TXT".to_string()));
    run(&mut e, RunOn::SaveGame, None);
    assert_eq!(runs(), ["run print upper 0"]);
}

/// A listed file that cannot be read (here locked, and first in the listing) is passed over; the
/// files after it still compile and run.
#[test]
fn a_file_that_cannot_be_read_is_passed_over() {
    let mut e = runner_engine();
    files::put(
        "data\\nvse\\plugins\\scripts\\gl_alocked.txt",
        files::LOCKED,
    );
    folder(&[("gl_b.txt", Some("print b")), ("gl_c.txt", Some("print c"))]);
    init_files(&mut e);
    let c = calls();
    assert!(!c.iter().any(|s| s.contains("alocked")));
    run(&mut e, RunOn::LoadGame, None);
    // Both run (in JIP's map order, which the order test pins).
    let mut r = runs();
    r.sort();
    assert_eq!(r, ["run print b 0", "run print c 0"]);
}
