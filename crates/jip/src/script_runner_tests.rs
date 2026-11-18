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

#[derive(Default)]
struct Log {
    /// Every call of interest, in order.
    calls: Vec<String>,
    /// The names the folder search answers, with whether each is a folder.
    listing: Option<Vec<(String, bool)>>,
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
    e.register(SCRIPT_EXECUTE, |_, a| {
        assert_eq!(&a[1..4], &[0, 0, 0], "no reference, no list");
        note(format!("run {} {}", text_of(a[0]), a[4]));
        eax(1)
    });
    e.register(FIND_FIRST_FILE_A, |e, a| {
        let pattern = nvse::table::guest_text(e, a[0]);
        assert_eq!(pattern, SEARCH);
        let Some(mut entries) = with_log(|l| l.listing.clone()) else {
            return eax(INVALID_HANDLE_VALUE);
        };
        let first = entries.remove(0);
        found(e, a[1], &first);
        with_log(|l| l.listing = Some(entries));
        eax(0x900)
    });
    e.register(FIND_NEXT_FILE_A, |e, a| {
        assert_eq!(a[0], 0x900);
        let next = with_log(|l| {
            let rest = l.listing.as_mut().unwrap();
            (!rest.is_empty()).then(|| rest.remove(0))
        });
        match next {
            Some(f) => {
                found(e, a[1], &f);
                eax(1)
            }
            None => eax(0),
        }
    });
    e.register(FIND_CLOSE, |_, a| {
        note(format!("find-close {:x}", a[0]));
        eax(1)
    });
    e
}

fn found(e: &mut Engine, data: u32, (name, folder): &(String, bool)) {
    e.mem.set_u32(
        data,
        if *folder {
            FILE_ATTRIBUTE_DIRECTORY
        } else {
            0x20
        },
    );
    e.mem.write(data + FIND_DATA_NAME, name.as_bytes());
    e.mem.set_u8(data + FIND_DATA_NAME + name.len() as u32, 0);
}

/// The folder search answers these names (a `/` at the end: a folder); the files go in the seam.
fn folder(entries: &[(&str, Option<&str>)]) {
    let listing = entries
        .iter()
        .map(|(name, text)| {
            if let Some(text) = text {
                files::put(
                    &format!("data\\nvse\\plugins\\scripts\\{name}"),
                    text.as_bytes(),
                );
            }
            (name.trim_end_matches('/').to_string(), name.ends_with('/'))
        })
        .collect();
    with_log(|l| l.listing = Some(listing));
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
        ("gx_missing.txt", None),
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
    // the missing one and the plain file never compiled.
    assert!(c.contains(&"editor-id ln_mod.txt".to_string()));
    assert!(c.contains(&"editor-id gs_save.txt".to_string()));
    assert_eq!(c.iter().filter(|s| *s == "destroy 1").count(), 2);
    assert!(!c
        .iter()
        .any(|s| s.contains("readme") || s.contains("missing")));
    assert!(c.contains(&"find-close 900".to_string()));
    let kept: Vec<RunOn> = e
        .ext::<Jip>()
        .script_runner
        .cached
        .iter()
        .map(|(_, k)| k.run_on)
        .collect();
    assert_eq!(kept.len(), 2);
    assert!(kept.contains(&RunOn::LoadOrNewGame) && kept.contains(&RunOn::SaveGame));
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
    folder(&[
        ("gl_same.txt", Some("print first")),
        ("GL_SAME.TXT", Some("print second")),
    ]);
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
    assert!(!calls().iter().any(|c| c.starts_with("find-close")));
}

#[test]
fn register_keeps_a_file_below_the_folder_once() {
    let mut e = runner_engine();
    files::put("data\\nvse\\plugins\\scripts\\mymod\\gl_x.txt", b"print x");
    files::put("data\\nvse\\plugins\\scripts\\mymod\\gr_y.txt", b"print y");
    assert!(register(&mut e, b"mymod/gl_x.txt"));
    assert!(!register(&mut e, b"mymod\\gl_x.txt"), "kept already");
    assert!(!register(&mut e, b"mymod/gr_y.txt"), "gr_ is never kept");
    assert!(!register(&mut e, b"gl_x.txt"), "no separator");
    assert!(!register(&mut e, b"mymod/gl_none.txt"), "no file");
    run(&mut e, RunOn::LoadGame, None);
    assert_eq!(runs(), ["run print x 0"]);
}
