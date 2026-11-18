//! JIP LN NVSE's script runner (`JIPScriptRunner`, `internal/jip_core.cpp` (JIP LN NVSE)): the
//! script files mods ship in `Data\NVSE\plugins\scripts\`, compiled at JIP's deferred start and run
//! at the game's moments the file's name gives. Many mods are only such files (no plugin of their
//! own): without the runner they never run, and nothing says so.
//!
//! A file's first two letters (without case) and a `_` name its moment:
//! - `gr_`: run once, at the deferred start (xNVSE's `DeferredInit`, at the main menu), compiled and
//!   run on a temporary script with its lambda variables kept for the run (`RunScriptSource`);
//! - `gl_` each load of a saved game, `gn_` each new game, `ln_` both, `gs_` each save, `xm_` each
//!   exit to the main menu, `gx_` the exit from the game: compiled once at the deferred start into
//!   a script of their own (`Script::Create`), kept by name ([`State::cached`]) and run then
//!   ([`run`]), from JIP's listener ([`crate::cosave::listen`]).
//!
//! A file's text is compiled as a partial script (statements with no `scn` line and no block), in
//! the editor's mode (`runtimeMode` 0), by the game's compiler (`ScriptCompiler::Compile`,
//! `005aeb90`); one that does not compile, or compiles to nothing, is passed over. The files are
//! listed as JIP lists them, through the game's `FindFirstFileA` on `...\scripts\*.txt` (in nv-rs
//! the platform layer's, on every system), and read through the mods' files seam
//! ([`engine_core::mod_files`]: the port's own folder beside the saves first, then the game's Data;
//! the game's own file read when no platform is there), at most `STR_BUFFER_SIZE - 1` bytes, as JIP's
//! `FileToBuffer` reads into its string buffer. Nothing is written.
//!
//! Only while JIP answers "installed" ([`crate::set_answers_installed`]): without the DLL no file
//! runs.
//!
//! A file that does not compile is passed over as in JIP, and said: the compiler's own message
//! (JIP's `LogCompileError` over the string's release at `005aeb66` in `ScriptCompiler::ScriptError`,
//! [`COMPILE_ERROR`], while the files compile) and one line naming the file go to the port's log
//! (JIP writes the message to `jip_ln_nvse.log`, which the port does not keep), and JIP's console
//! line once per start ([`CONSOLE_LINE`], naming the port's log instead of JIP's file).
//!
//! `RegisterSRScript` ([`register`]) keeps a file below the folder only while the files compile
//! ([`State::init_in_progress`]): a `gr_` file calls it then, as with JIP.
//!
//! Not here: `ValidateOpcodeSample`, a check JIP makes at the same point of one particular mod's
//! file that switches one of its own commands off for that mod; it changes nothing for any other
//! mod and is left out (noted for review).

use crate::containers::{Name, UnorderedMap};
use crate::Jip;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// `scriptsPath`: the folder, and the search JIP makes in it.
pub const SCRIPTS_PATH: &[u8] = b"Data\\NVSE\\plugins\\scripts\\";
const SEARCH: &[u8] = b"Data\\NVSE\\plugins\\scripts\\*.txt";

/// `STR_BUFFER_SIZE` (JIP LN NVSE, `internal/utility.h`): `GetStrArgBuffer()`'s size; a file is
/// read into it up to one byte less.
const STR_BUFFER_SIZE: usize = 0x40000;

/// The game's calls: `Script`'s constructor and destructor, `TESForm::MarkAsTemporary`,
/// `Script::Execute` (`thiscall`: the script, `thisObj`, `eventList`, `containingObj`, a flag),
/// `ScriptBuffer`'s constructor and destructor, `ScriptCompiler::Compile` (on the console's
/// compiler, `ConsoleManager::GetSingleton(true)`'s first word: see [`compile`]).
const SCRIPT_CONSTRUCTOR: u32 = 0x005a_a0f0;
const SCRIPT_DESTRUCTOR: u32 = 0x005a_a1a0;
const MARK_AS_TEMPORARY: u32 = 0x0048_4490;
const SCRIPT_EXECUTE: u32 = 0x005a_c1e0;
const BUFFER_CONSTRUCTOR: u32 = 0x005a_e490;
const BUFFER_DESTRUCTOR: u32 = 0x005a_e5c0;
const SCRIPT_COMPILE: u32 = 0x005a_eb90;
const CONSOLE_MANAGER: u32 = 0x0071_b160;
/// `TESForm`'s virtual functions: `Destroy(bool)` (`+0x10`) and `SetEditorID` (`+0x134`).
const DESTROY: u32 = 0x10;
const SET_EDITOR_ID: u32 = 0x134;

/// `Script` (JIP LN NVSE, `nvse/GameScript.h`): its size, `info.dataLength` and `text`.
pub const SCRIPT_SIZE: u32 = 0x54;
const SCRIPT_DATA_LENGTH: u32 = 0x20;
const SCRIPT_TEXT: u32 = 0x2c;
/// `ScriptBuffer`: its size (`size 0x58?`), `scriptText`, `runtimeMode`, `scriptName`,
/// `partialScript`, `currScript`.
const BUFFER_SIZE: u32 = 0x58;
const BUFFER_TEXT: u32 = 0;
const BUFFER_RUNTIME_MODE: u32 = 8;
const BUFFER_NAME: u32 = 0xc;
const BUFFER_PARTIAL: u32 = 0x18;
const BUFFER_CURRENT_SCRIPT: u32 = 0x4c;

/// KERNEL32's import slots the game's file code calls (`DirectoryIterator`'s search), and
/// `WIN32_FIND_DATAA`: its size, its attributes (first word) and `cFileName`.
const FIND_FIRST_FILE_A: u32 = 0x00fd_f070;
const FIND_NEXT_FILE_A: u32 = 0x00fd_f068;
const FIND_CLOSE: u32 = 0x00fd_f064;
const FIND_DATA_SIZE: u32 = 0x140;
const FIND_DATA_NAME: u32 = 0x2c;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const INVALID_HANDLE_VALUE: u32 = 0xffff_ffff;

/// `JIPScriptRunner::ScriptRunOn`: a file's moment, the multi-character constant its first two
/// letters make (`*(UInt16*)fileName | 0x2020`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOn {
    /// `gl_` (`'lg'`).
    LoadGame,
    /// `xm_` (`'mx'`).
    ExitToMainMenu,
    /// `gn_` (`'ng'`).
    NewGame,
    /// `ln_` (`'nl'`).
    LoadOrNewGame,
    /// `gr_` (`'rg'`): run at the deferred start, never kept.
    RestartGame,
    /// `gs_` (`'sg'`).
    SaveGame,
    /// `gx_` (`'xg'`).
    ExitGame,
}

impl RunOn {
    /// The moment of a file named `name`: its third byte a `_` and its first two, with `0x20` set
    /// in each, one of the seven; else none.
    pub fn of(name: &[u8]) -> Option<RunOn> {
        if name.get(2) != Some(&b'_') {
            return None;
        }
        let word = u16::from_le_bytes([name[0], name[1]]) | 0x2020;
        Some(match word {
            0x6c67 => RunOn::LoadGame,
            0x6d78 => RunOn::ExitToMainMenu,
            0x6e67 => RunOn::NewGame,
            0x6e6c => RunOn::LoadOrNewGame,
            0x7267 => RunOn::RestartGame,
            0x7367 => RunOn::SaveGame,
            0x7867 => RunOn::ExitGame,
            _ => return None,
        })
    }
}

/// `CachedSRScript`: a compiled file's script and its moment (`kRunOn_Invalid`, none, for the
/// scripts `RunBatchScript` keeps in the same map).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cached {
    pub script: u32,
    pub run_on: Option<RunOn>,
}

/// The runner's state: `s_cachedScripts`, by the file's name (JIP's map of `const char*` keys:
/// hashed without case, the first of a name kept), and `initInProgress`.
#[derive(Default)]
pub struct State {
    pub cached: UnorderedMap<Name, Cached>,
    /// `JIPScriptRunner::initInProgress`: 0 outside [`init_files`]; 1 while the files compile; 2
    /// once a compile error was reported in this start.
    pub init_in_progress: u8,
    /// The port's own: how many [`run_script_nested`] runs are under way (the fixes preset's
    /// "script error loop guard" row caps it).
    pub nesting: u32,
}

/// The string release in `ScriptCompiler::ScriptError` (`005aeb66`: the message, "SCRIPTS: ..."),
/// where JIP's `LogCompileError` reads the message (words: the string).
pub const COMPILE_ERROR: SiteSlot = SiteSlot::new(Owner::Jip, 0x005a_eb66);

/// JIP's console line at the first compile error of a start, naming the port's log for the
/// details (JIP names its own `jip_ln_nvse.log`).
pub const CONSOLE_LINE: &str =
    "[SCRIPT RUNNER] One or more script files could not be compiled. For more info please refer to the nv-rs log.";

/// The handlers, while JIP answers "installed" ([`crate::install`]).
pub fn install(e: &mut Engine, answers_installed: bool) {
    if answers_installed {
        e.on_site(COMPILE_ERROR, compile_error);
    }
}

/// `"SCRIPTS: "`, the part of the message JIP leaves out (`m_data + 9`).
const MESSAGE_PREFIX: usize = 9;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: JIPScriptRunner::LogCompileError
// (the string's release itself stays the game's).
/// While the files compile: the compiler's message (longer than its 9-byte prefix) to the port's
/// log, after one heading and JIP's console line at the first of a start.
fn compile_error(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let string = w[0];
    let len = usize::from(e.mem.u16(string + 4));
    if state(e).init_in_progress == 0 || len <= MESSAGE_PREFIX {
        return None;
    }
    if state(e).init_in_progress == 1 {
        state(e).init_in_progress = 2;
        eprintln!("[SCRIPT RUNNER] Errors were found while compiling the following script files:");
        let line = nvse::table::guest_str(e, CONSOLE_LINE.as_bytes());
        nvse::commands::console_print(e, "%s", &[line]);
    }
    let data = e.mem.u32(string);
    let text = e
        .mem
        .bytes(data + MESSAGE_PREFIX as u32, (len - MESSAGE_PREFIX) as u32);
    eprintln!("{}", String::from_utf8_lossy(&text));
    None
}

/// The port's line for a file that did not compile (or compiled to nothing): passed over.
fn skipped(name: &[u8]) {
    eprintln!(
        "JIP script runner: {} did not compile; skipped",
        String::from_utf8_lossy(name)
    );
}

/// A script file's bytes, as `FileToBuffer` reads them: at most `STR_BUFFER_SIZE - 1`, `None` for
/// a file that cannot be read or is empty. Through the mods' files seam (the port's folder, then the
/// game's Data); the game's own read when no platform answers the seam.
pub(crate) fn read_file(e: &mut Engine, path: &[u8]) -> Option<Vec<u8>> {
    let max = STR_BUFFER_SIZE - 1;
    let bytes = match engine_core::mod_files::read_head_or_error(e, path, max) {
        Ok(bytes) => bytes,
        Err(engine_core::mod_files::ERROR_NOT_SUPPORTED) => {
            let mut bytes = nvse::saveload::read_file(e, path)?;
            bytes.truncate(max);
            bytes
        }
        Err(_) => return None,
    };
    (!bytes.is_empty()).then_some(bytes)
}

/// The text up to its first NUL (what the game's compiler reads of the buffer).
fn c_text(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    &bytes[..end]
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameScript.cpp: Script::Compile.
/// The script's text compiled into it: a `ScriptBuffer` on the stack with the text, the editor's
/// mode (0), the name, the partial flag and the script, given to `ScriptCompiler::Compile`; its
/// answer. JIP calls the compile with no compiler object of its own (`StdCall`); the port gives it
/// the console's, as xNVSE's run-time compile does (`CompileScriptEx`), and with no console it
/// does not compile.
fn compile(e: &mut Engine, script: u32, name: &[u8]) -> bool {
    let console = e.call(CONSOLE_MANAGER, &args![1u32]).u32();
    if console == 0 {
        return false;
    }
    let compiler = e.mem.u32(console);
    e.with_stack(BUFFER_SIZE, |e, at| {
        let buffer = at.addr();
        e.call(BUFFER_CONSTRUCTOR, &args![buffer]);
        let text = e.mem.u32(script + SCRIPT_TEXT);
        e.mem.set_u32(buffer + BUFFER_TEXT, text);
        e.mem.set_u32(buffer + BUFFER_RUNTIME_MODE, 0);
        crate::commands::string_set(e, buffer + BUFFER_NAME, name);
        e.mem.set_u8(buffer + BUFFER_PARTIAL, 1);
        e.mem.set_u32(buffer + BUFFER_CURRENT_SCRIPT, script);
        let ok = e
            .call(SCRIPT_COMPILE, &args![compiler, script, buffer])
            .bool();
        e.call(BUFFER_DESTRUCTOR, &args![buffer]);
        ok
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameScript.cpp: Script::Init.
/// The script at `script` made (the game's constructor), marked temporary, compiled from `text`
/// under `name`: true when it compiled to some data. The text is held in a copy only for the
/// compile; the script keeps no text.
fn init(e: &mut Engine, script: u32, text: &[u8], name: &[u8]) -> bool {
    e.call(SCRIPT_CONSTRUCTOR, &args![script]);
    e.call(MARK_AS_TEMPORARY, &args![script]);
    let text = c_text(text);
    let at = e.mem.alloc(text.len() as u32 + 1);
    e.mem.write(at, text);
    e.mem.set_u8(at + text.len() as u32, 0);
    e.mem.set_u32(script + SCRIPT_TEXT, at);
    let ok = compile(e, script, name) && e.mem.u32(script + SCRIPT_DATA_LENGTH) != 0;
    e.mem.set_u32(script + SCRIPT_TEXT, 0);
    e.mem.free(at);
    ok
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameScript.cpp: Script::Create.
/// A new script of the game's heap compiled from `text` and given the editor id `name`, or 0 when
/// it does not compile (the script destroyed).
pub fn create(e: &mut Engine, text: &[u8], name: &[u8]) -> u32 {
    let script = crate::commands::heap_alloc(e, SCRIPT_SIZE);
    if init(e, script, text, name) {
        nvse::events::with_guest_text(e, name, |e, at| {
            e.vcall(script, SET_EDITOR_ID, &args![at]);
        });
        script
    } else {
        e.vcall(script, DESTROY, &args![1u32]);
        0
    }
}

/// `Script::Execute(thisObj, eventList, containingObj, arg3)` with no reference and no list.
fn execute(e: &mut Engine, script: u32, arg3: bool) {
    e.call(
        SCRIPT_EXECUTE,
        &args![script, 0u32, 0u32, 0u32, u32::from(arg3)],
    );
}

/// `ExtraScript` (type 0x0D) and its event list (+0x10).
const EXTRA_SCRIPT: u32 = 0x0d;
const XSCRIPT_EVENTS: u32 = 0x10;

/// The deepest [`run_script_nested`] goes while the fixes preset's "script error loop guard" row
/// is on.
pub const MAX_NESTING: u32 = 64;

/// [`run_script`] for `RunBatchScript` and `RunScriptSnippet` (`what`: the file or "snippet"),
/// counted: a script that runs itself again through either command (JIP has no cap: the host's
/// stack runs out and the game stops) is, with the fixes preset's "script error loop guard" row
/// on (`nvse::events::error_guard`), not run past [`MAX_NESTING`] deep: one log line and `false`.
/// Off, JIP's way. One count for the run.
pub fn run_script_nested(e: &mut Engine, script: u32, calling_ref: u32, what: &[u8]) -> bool {
    if nvse::events::error_guard(e) && state(e).nesting >= MAX_NESTING {
        eprintln!(
            "[SCRIPT RUNNER] {} not run: nested {MAX_NESTING} deep (it runs itself again)",
            String::from_utf8_lossy(what)
        );
        return false;
    }
    state(e).nesting += 1;
    run_script(e, script, calling_ref);
    let s = state(e);
    s.nesting = s.nesting.saturating_sub(1);
    true
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: JIPScriptRunner::RunScript.
/// `script` run on `calling_ref` (`Script::Execute(callingRef)`: no list, no container, `true`),
/// with the reference's own script's event list taken away while it runs and put back after.
pub fn run_script(e: &mut Engine, script: u32, calling_ref: u32) {
    let xscript = crate::commands::extra::refr_extra(e, calling_ref, EXTRA_SCRIPT);
    let events = if xscript == 0 {
        0
    } else {
        e.mem.u32(xscript + XSCRIPT_EVENTS)
    };
    if events != 0 {
        e.mem.set_u32(xscript + XSCRIPT_EVENTS, 0);
    }
    e.call(
        SCRIPT_EXECUTE,
        &args![script, calling_ref, 0u32, 0u32, 1u32],
    );
    if events != 0 {
        e.mem.set_u32(xscript + XSCRIPT_EVENTS, events);
    }
}

/// The script kept under `name` (`s_cachedScripts`), 0 for none.
pub fn cached_script(e: &mut Engine, name: &[u8]) -> u32 {
    state(e)
        .cached
        .get(&Name(name.to_vec()))
        .map_or(0, |c| c.script)
}

/// `s_cachedScripts->Emplace(name, script)` with no moment (`RunBatchScript`'s): the first of a
/// name kept.
pub fn keep_batch_script(e: &mut Engine, name: &[u8], script: u32) -> u32 {
    state(e)
        .cached
        .emplace_with(Name(name.to_vec()), || Cached {
            script,
            run_on: None,
        })
        .0
        .script
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: JIPScriptRunner::RunScriptSource.
/// `text` compiled under `name` into a temporary script on the stack and run once (with
/// `capture`, its lambda variables kept while it runs, xNVSE's `LambdaSaveVariableList`); the
/// script destroyed after. Whether it compiled.
pub fn run_script_source(e: &mut Engine, text: &[u8], name: &[u8], capture: bool) -> bool {
    e.with_stack(SCRIPT_SIZE, |e, at| {
        let script = at.addr();
        let success = init(e, script, text, name);
        if success {
            if capture {
                nvse::lambda::save_variables(e, script);
                execute(e, script, true);
                nvse::lambda::unsave_variables(e, script);
            } else {
                execute(e, script, true);
            }
        }
        e.call(SCRIPT_DESTRUCTOR, &args![script]);
        success
    })
}

/// The files `SEARCH` lists, in its order: each entry's name, and whether it is a file.
fn list(e: &mut Engine) -> Vec<(Vec<u8>, bool)> {
    let mut out = Vec::new();
    e.with_stack(FIND_DATA_SIZE, |e, data| {
        let data = data.addr();
        let handle = nvse::events::with_guest_text(e, SEARCH, |e, p| {
            e.call(FIND_FIRST_FILE_A, &args![p, data]).u32()
        });
        if handle == INVALID_HANDLE_VALUE {
            return;
        }
        loop {
            let name = nvse::table::guest_text(e, data + FIND_DATA_NAME);
            let is_file = e.mem.u32(data) & FILE_ATTRIBUTE_DIRECTORY == 0;
            out.push((name, is_file));
            // `DirectoryIterator::operator++`: any failure ends the search.
            if e.call(FIND_NEXT_FILE_A, &args![handle, data]).u32() == 0 {
                break;
            }
        }
        e.call(FIND_CLOSE, &args![handle]);
    });
    out
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().script_runner
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: JIPScriptRunner::Init.
/// At JIP's deferred start (its last step): every file of the folder with one of the seven
/// moments, read; a `gr_` file run at once, any other compiled and kept by its name (a name kept
/// already keeps its first script). Nothing while JIP answers "not installed".
pub fn init_files(e: &mut Engine) {
    if !e.ext::<Jip>().answers_installed {
        return;
    }
    state(e).init_in_progress = 1;
    for (name, is_file) in list(e) {
        if !is_file {
            continue;
        }
        let Some(run_on) = RunOn::of(&name) else {
            continue;
        };
        let mut path = SCRIPTS_PATH.to_vec();
        path.extend_from_slice(&name);
        let Some(text) = read_file(e, &path) else {
            continue;
        };
        if run_on == RunOn::RestartGame {
            if !run_script_source(e, &text, &name, true) {
                skipped(&name);
            }
        } else {
            let script = create(e, &text, &name);
            if script == 0 {
                skipped(&name);
            } else {
                // `Emplace`: a name kept already keeps its first script (the new one stays made,
                // as in JIP).
                state(e).cached.emplace_with(Name(name), || Cached {
                    script,
                    run_on: Some(run_on),
                });
            }
        }
    }
    state(e).init_in_progress = 0;
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: JIPScriptRunner::RegisterScript.
/// `RegisterSRScript`'s step: the file at `rel_path` below the folder (`/` read as `\`) compiled and
/// kept when its name (the part after its last `\`) has a moment other than `gr_` and is not kept
/// yet. Whether it was kept. A path with no `\` is never kept (JIP's test of the separator), and
/// nothing is kept outside [`init_files`] (`Cmd_RegisterSRScript_Execute` tests `initInProgress`).
pub fn register(e: &mut Engine, rel_path: &[u8]) -> bool {
    if state(e).init_in_progress == 0 {
        return false;
    }
    let rel: Vec<u8> = rel_path
        .iter()
        .map(|&b| if b == b'/' { b'\\' } else { b })
        .collect();
    let Some(at) = rel.iter().rposition(|&b| b == b'\\') else {
        return false;
    };
    let name = rel[at + 1..].to_vec();
    let Some(run_on) = RunOn::of(&name) else {
        return false;
    };
    if run_on == RunOn::RestartGame || state(e).cached.get(&Name(name.clone())).is_some() {
        return false;
    }
    let mut path = SCRIPTS_PATH.to_vec();
    path.extend_from_slice(&rel);
    let Some(text) = read_file(e, &path) else {
        return false;
    };
    let script = create(e, &text, &name);
    if script == 0 {
        skipped(&rel);
        return false;
    }
    state(e).cached.emplace_with(Name(name), || Cached {
        script,
        run_on: Some(run_on),
    });
    true
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: JIPScriptRunner::RunScripts.
/// Every kept script of moment `first` or `second`, in the map's order, run with no reference.
pub fn run(e: &mut Engine, first: RunOn, second: Option<RunOn>) {
    let scripts: Vec<u32> = state(e)
        .cached
        .iter()
        .filter(|(_, c)| c.run_on == Some(first) || (second.is_some() && c.run_on == second))
        .map(|(_, c)| c.script)
        .collect();
    for script in scripts {
        execute(e, script, false);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), jip_nvse.cpp: NVSEMessageHandler (the runner's
// calls in each case) and internal/patches_game.h: DeferredInit (its last step).
/// The runner's part of JIP's listener ([`crate::cosave::listen`], after each case's own steps):
/// the files compiled at the deferred start; at a load that succeeded (`fosLoaded`, the data's low
/// byte) the `gl_` and `ln_` ones run, at a new game the `gn_` and `ln_` ones, at a save the `gs_`
/// ones, at the exit to the main menu the `xm_` ones, at either exit from the game the `gx_` ones.
pub fn on_message(e: &mut Engine, kind: u32, data: u32) {
    use nvse::plugins::msg;
    match kind {
        msg::DEFERRED_INIT => init_files(e),
        msg::POST_LOAD_GAME if data & 0xff != 0 => {
            run(e, RunOn::LoadGame, Some(RunOn::LoadOrNewGame));
        }
        msg::NEW_GAME => run(e, RunOn::NewGame, Some(RunOn::LoadOrNewGame)),
        msg::SAVE_GAME => run(e, RunOn::SaveGame, None),
        msg::EXIT_TO_MAIN_MENU => run(e, RunOn::ExitToMainMenu, None),
        msg::EXIT_GAME | msg::EXIT_GAME_CONSOLE => run(e, RunOn::ExitGame, None),
        _ => {}
    }
}

#[cfg(test)]
#[path = "script_runner_tests.rs"]
mod tests;
