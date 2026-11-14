//! xNVSE's run-time compile entry and its user function files (ADR-0016 §I-14; plan step 3.8,
//! part 2): `CompileScriptEx` and the plugin interface's `CompileScript` and `CompileExpression`
//! (`GameScript.cpp`), `Script::RunScriptLine2` (the console interface), the cache of function
//! files (`CachedScripts.cpp`: `CompileAndCacheScript`, `CacheAllScriptsInPath`), the files'
//! compile at the first frame (`HandleMainLoopHook`, `Hooks_Gameplay.cpp`) and the
//! `CompileScript` command (`Commands_Script.cpp`).
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a).
//!
//! - **`CompileScriptEx`**: a script buffer and a `Script` made with the game's constructors (no
//!   form id assigned while the script is made), the buffer named (`nvse_partial_script_<n>`
//!   without a name), a partial script whose data starts with a fake `scn` (`1D 00 00 00`), in
//!   the console's run-time mode, compiled by the game's compiler (`ScriptCompiler::Compile`
//!   `005aeb90` on the console's compiler; with no console it fails), through xNVSE's compiler
//!   hooks ([`crate::compile_hooks`], [`crate::compiler`]). A failed script is destroyed; with
//!   `assign_form_id` a compiled one gets the game's next free form id.
//! - **The function files**: every file with an extension under
//!   `data/nvse/user_defined_functions/` (sub-folders too) is compiled at the first frame, after
//!   the plugins' `DeferredInit`, as a script named `nvseRuntimeScript<count><stem>`, and kept
//!   by its path below that folder (`sub\name.txt`), looked up by xNVSE's case-insensitive hash
//!   of the path. A file that fails prints "xNVSE: Failed to precompile script file at <path>" to
//!   the console. The folder is the game's working folder's, read through the game's own file
//!   calls (`FindFirstFileA`, `CreateFileA`, `ReadFile`), as xNVSE's `std::filesystem` and
//!   `std::ifstream` read it: in nv-rs that is the play build's own `Data` area, through the
//!   platform layer like every file the game reads. A file is read as `std::ifstream` reads one
//!   in text mode: each `\r\n` becomes `\n`, and a Ctrl+Z (`0x1A`) ends it.
//! - **`CompileScript path[, recompile]`** (alias `GetUDFFromFile`): the cached script of that
//!   path (`/` read as `\`); a path not cached yet (a file made after the first frame), or with
//!   `recompile`, is compiled and cached now. The result is the script's form id, 0 when the file
//!   is missing or does not compile.
//!
//! The success line of the first frame's compile and the failure line also go to `nvse.log`
//! ([`crate::log`]), as xNVSE's `_MESSAGE`/`_ERROR` write them there.
//!
//! Not here: xNVSE's compiled-script cache on disk (`ScriptDataCache`, `script_data_cache.bin`),
//! a speed cache only (see `compile_hooks.rs`), and the new script language
//! (`Compiler::CompileNVSEScript`: scripts starting with `name` do not compile; deferred).

use crate::compiler::{sb, sc, string_set};
use crate::table::{Command, Param, Returns};
use crate::{handled, CommandArgs, Nvse};
use engine_core::prelude::*;
use std::collections::HashMap;

/// `ScriptFilesPath`: the folder of the function files, relative to the game's working folder.
pub const SCRIPT_FILES_PATH: &[u8] = b"data/nvse/user_defined_functions/";

/// `FormHeap_Allocate`, `FormHeap_Free`; `ScriptBuffer`'s constructor and destructor; `Script`'s
/// constructor and destructor; `ConsoleManager::GetSingleton` (`MenuConsole::Instance(true)`,
/// whose first word is the compiler `Script::Compile` runs on); `ScriptCompiler::Compile`.
const FORM_HEAP_ALLOCATE: u32 = 0x0040_1000;
const FORM_HEAP_FREE: u32 = 0x0040_1030;
const BUFFER_CONSTRUCTOR: u32 = 0x005a_e490;
const BUFFER_DESTRUCTOR: u32 = 0x005a_e5c0;
const SCRIPT_CONSTRUCTOR: u32 = 0x005a_a0f0;
const SCRIPT_DESTRUCTOR: u32 = 0x005a_a1a0;
const CONSOLE_MANAGER: u32 = 0x0071_b160;
const SCRIPT_COMPILE: u32 = 0x005a_eb90;
/// `DataHandler`'s singleton, `DisableAssignFormIDs` and `GetNextFreeFormID` (both `thiscall`).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const DISABLE_ASSIGN_FORM_IDS: u32 = 0x0046_4d30;
const NEXT_FREE_FORM_ID: u32 = 0x0046_9800;
/// `TESForm::SetRefID` (the `+0x128` virtual function) and `SetEditorID_AtRuntime` (`+0x134`).
const SET_REF_ID: u32 = 0x128;
const SET_EDITOR_ID: u32 = 0x134;
/// `TESForm::MarkAsTemporary` (`00484490`), `Script::SetText` (`005abe50`) and `Script::Run`
/// (`005ac400`), all `thiscall`.
const MARK_AS_TEMPORARY: u32 = 0x0048_4490;
const SCRIPT_SET_TEXT: u32 = 0x005a_be50;
const SCRIPT_RUN: u32 = 0x005a_c400;
/// `ScriptBuffer::runtimeMode` and its `kGameConsole`.
const BUFFER_RUNTIME_MODE: u32 = 8;
const GAME_CONSOLE: u32 = 1;
/// The fake `scn` a partial script's data starts with.
const FAKE_SCN: u8 = 0x1d;

/// KERNEL32's import slots the game's own file code calls.
const CREATE_FILE_A: u32 = 0x00fd_f088;
const READ_FILE: u32 = 0x00fd_f080;
const CLOSE_HANDLE: u32 = 0x00fd_f07c;
const FIND_FIRST_FILE_A: u32 = 0x00fd_f070;
const FIND_NEXT_FILE_A: u32 = 0x00fd_f068;
const FIND_CLOSE: u32 = 0x00fd_f064;
const GENERIC_READ: u32 = 0x8000_0000;
/// `FILE_SHARE_READ | FILE_SHARE_WRITE` (`_SH_DENYNO`, the share mode of `std::ifstream`).
const SHARE_READ_WRITE: u32 = 3;
const OPEN_EXISTING: u32 = 3;
const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
const INVALID_HANDLE_VALUE: u32 = 0xffff_ffff;
/// `WIN32_FIND_DATAA`: its size and where `cFileName` is.
const FIND_DATA_SIZE: u32 = 0x140;
const FIND_DATA_NAME: u32 = 0x2c;

/// The compile entry's state: `g_partialScriptCount` and `cachedFileUDFs`.
#[derive(Default)]
pub struct State {
    /// How many unnamed scripts were compiled (`nvse_partial_script_<n>`).
    pub partial_scripts: u32,
    /// The cached function scripts by xNVSE's hash of the path (`StrHashCI`): the path as first
    /// inserted (xNVSE's map copies the key once) and the script. Two paths of one hash are one
    /// entry, as in xNVSE's map, which compares the hashes only.
    pub cached: HashMap<u32, (Vec<u8>, u32)>,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Nvse>().compile_script
}

/// The cached script of `path` (a path below the function folder, `\` as the separator), 0 for
/// none.
pub fn cached(e: &mut Engine, path: &[u8]) -> u32 {
    let hash = crate::events::hash_name(path);
    state(e).cached.get(&hash).map_or(0, |&(_, s)| s)
}

/// `cachedFileUDFs[path] = script`: a path of a hash already there replaces its script and keeps
/// its first key.
fn cache(e: &mut Engine, path: &[u8], script: u32) {
    let hash = crate::events::hash_name(path);
    state(e)
        .cached
        .entry(hash)
        .and_modify(|entry| entry.1 = script)
        .or_insert_with(|| (path.to_vec(), script));
}

// ---- the compile entry ----

/// `Script::Compile`: the console's compiler compiles the script from the buffer; false with no
/// console or no compiler (xNVSE's fix for a compile called too early, #281).
fn compile(e: &mut Engine, script: u32, buffer: u32) -> bool {
    let console = e.call(CONSOLE_MANAGER, &args![1u32]).u32();
    if console == 0 {
        return false;
    }
    let compiler = e.mem.u32(console);
    if compiler == 0 {
        return false;
    }
    e.call(SCRIPT_COMPILE, &args![compiler, script, buffer])
        .bool()
}

/// `Delete<T, destructor>`: the destructor, then the form heap's block.
fn delete(e: &mut Engine, object: u32, destructor: u32) {
    e.call(destructor, &args![object]);
    e.call(FORM_HEAP_FREE, &args![object]);
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp: CompileScriptEx.
/// `CompileScriptEx(text, name, assignFormID)`: the compiled script (a new `Script` form), or 0
/// when it does not compile. The text is the script's source (up to its first NUL); the compile
/// reads it from a copy in guest memory, which xNVSE's compile hooks may write into while they
/// work (the `#version` tags), as they write into xNVSE's caller's string.
pub fn compile_script_ex(e: &mut Engine, text: &[u8], name: Option<&[u8]>, assign_form_id: bool) -> u32 {
    let buffer = e.call(FORM_HEAP_ALLOCATE, &args![sb::SIZE]).u32();
    e.call(BUFFER_CONSTRUCTOR, &args![buffer]);
    let handler = e.mem.u32(DATA_HANDLER);
    e.call(DISABLE_ASSIGN_FORM_IDS, &args![handler, 1u32]);
    let memory = e.call(FORM_HEAP_ALLOCATE, &args![sc::SIZE]).u32();
    let script = e.call(SCRIPT_CONSTRUCTOR, &args![memory]).u32();
    e.call(DISABLE_ASSIGN_FORM_IDS, &args![handler, 0u32]);
    let name = match name {
        Some(n) => n.to_vec(),
        None => {
            let s = state(e);
            s.partial_scripts = s.partial_scripts.wrapping_add(1);
            format!("nvse_partial_script_{}", s.partial_scripts as i32).into_bytes()
        }
    };
    string_set(e, buffer + sb::NAME, &name);
    let text = crate::commands::string::c_str(text);
    let text_at = e.mem.alloc(text.len() as u32 + 1);
    e.mem.write(text_at, text);
    e.mem.set_u8(text_at + text.len() as u32, 0);
    e.mem.set_u32(buffer + sb::TEXT, text_at);
    e.mem.set_u32(script + sc::TEXT, text_at);
    e.mem.set_u8(buffer + sb::PARTIAL, 1);
    let data = e.mem.u32(buffer + sb::DATA);
    e.mem.set_u8(data, FAKE_SCN);
    e.mem.set_u32(buffer + sb::DATA_OFFSET, 4);
    e.mem.set_u32(buffer + BUFFER_RUNTIME_MODE, GAME_CONSOLE);
    e.mem.set_u32(buffer + sb::CURRENT_SCRIPT, script);
    let result = compile(e, script, buffer);
    e.mem.set_u32(buffer + sb::TEXT, 0);
    e.mem.set_u32(script + sc::TEXT, 0);
    e.mem.free(text_at);
    // The `unique_ptr`s go in the opposite order of their making: the script (unless released),
    // then the buffer.
    let out = if result {
        if assign_form_id {
            let handler = e.mem.u32(DATA_HANDLER);
            let id = e.call(NEXT_FREE_FORM_ID, &args![handler]).u32();
            e.vcall(script, SET_REF_ID, &args![id, 1u32]);
        }
        script
    } else {
        delete(e, script, SCRIPT_DESTRUCTOR);
        0
    };
    delete(e, buffer, BUFFER_DESTRUCTOR);
    out
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp: CompileScript.
/// The plugin interface's `CompileScript(text)`: an unnamed script, no form id.
pub fn compile_script(e: &mut Engine, text: &[u8]) -> u32 {
    compile_script_ex(e, text, None, false)
}

/// `FindStringCI`: `needle` in `haystack`, each byte compared through the game's `toupper`.
fn find_ci(e: &mut Engine, haystack: &[u8], needle: &[u8]) -> bool {
    let upper = crate::vars::upper_table(e);
    if needle.is_empty() {
        // `std::search` with an empty needle finds it at the start, which is not the end unless
        // the haystack is empty.
        return !haystack.is_empty();
    }
    haystack.windows(needle.len()).any(|w| {
        w.iter()
            .zip(needle)
            .all(|(a, b)| upper[usize::from(*a)] == upper[usize::from(*b)])
    })
}

/// `ReplaceAll(str, from, to)`: every `from`, left to right, past each replacement.
fn replace_all(text: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(text[i]);
            i += 1;
        }
    }
    out
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp: CompileExpression.
/// The plugin interface's `CompileExpression(text)`: a function script whose body is
/// `SetFunctionValue (<text>)`, or, when the text names `SetFunctionValue` itself (without case),
/// the text as the body with `%r` and `%R` made line breaks. The source is held to `FormatString`'s
/// 0x800-byte buffer.
pub fn compile_expression(e: &mut Engine, text: &[u8]) -> u32 {
    let condition = crate::commands::string::c_str(text);
    let source = if !find_ci(e, condition, b"SetFunctionValue") {
        crate::compiler::format(
            "begin function{}\nSetFunctionValue (%s)\nend\n",
            &[crate::compiler::A::S(condition)],
            0x800,
        )
    } else {
        let body = replace_all(condition, b"%r", b"\n");
        let body = replace_all(&body, b"%R", b"\n");
        crate::compiler::format(
            "begin function{}\n%s\nend\n",
            &[crate::compiler::A::S(&body)],
            0x800,
        )
    };
    compile_script(e, &source)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameScript.cpp: Script::RunScriptLine2.
/// The console interface's `RunScriptLine2(text, object, suppressOutput)`: a temporary `Script`
/// on the stack, marked temporary, given the text and run by the game (`Script::Run` on the
/// console's compiler, with `object` as the calling reference); its answer. The output switch is
/// not read (xNVSE's is commented out). With no console the line does not run (xNVSE would read
/// the console's word through a null pointer).
pub fn run_script_line2(e: &mut Engine, text: &[u8], object: u32, _suppress_output: bool) -> bool {
    let console = e.call(CONSOLE_MANAGER, &args![1u32]).u32();
    if console == 0 {
        return false;
    }
    let text = crate::commands::string::c_str(text).to_vec();
    crate::events::with_guest_text(e, &text, |e, text_at| {
        e.with_stack(sc::SIZE, |e, at| {
            let script = at.addr();
            e.call(SCRIPT_CONSTRUCTOR, &args![script]);
            e.call(MARK_AS_TEMPORARY, &args![script]);
            e.call(SCRIPT_SET_TEXT, &args![script, text_at]);
            let compiler = e.mem.u32(console);
            let result = e
                .call(SCRIPT_RUN, &args![script, compiler, 1u32, object])
                .bool();
            e.call(SCRIPT_DESTRUCTOR, &args![script]);
            result
        })
    })
}

/// `Script::RunScriptLine(text, object)`: [`run_script_line2`] with the output on.
pub fn run_script_line(e: &mut Engine, text: &[u8], object: u32) -> bool {
    run_script_line2(e, text, object, false)
}

// ---- the function files ----

/// The last part of a path (after its last `/` or `\`), as `std::filesystem::path::filename`.
fn file_name(path: &[u8]) -> &[u8] {
    match path.iter().rposition(|&b| b == b'/' || b == b'\\') {
        Some(at) => &path[at + 1..],
        None => path,
    }
}

/// `path::stem` and `path::extension` of a file name: the extension from its last dot, none for
/// `.`, `..`, a name with no dot or whose only dot starts it (`.profile`).
fn stem_and_extension(name: &[u8]) -> (&[u8], &[u8]) {
    if name == b"." || name == b".." {
        return (name, b"");
    }
    match name.iter().rposition(|&b| b == b'.') {
        Some(at) if at > 0 => (&name[..at], &name[at..]),
        _ => (name, b""),
    }
}

/// A file's bytes as `std::ifstream` reads them in text mode: `\r\n` becomes `\n` (a lone `\r`
/// stays), and a Ctrl+Z (`0x1A`) ends the file.
pub fn text_mode(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            0x1a => break,
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => {
                out.push(b'\n');
                i += 2;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    out
}

/// The file at `path` (relative to the game's working folder) read whole through the game's file
/// calls, as `std::ifstream` opens it (read, sharing read and write); `None` when it cannot be
/// opened (missing, or a folder).
fn read_file(e: &mut Engine, path: &[u8]) -> Option<Vec<u8>> {
    let handle = crate::events::with_guest_text(e, path, |e, p| {
        e.call(
            CREATE_FILE_A,
            &args![
                p,
                GENERIC_READ,
                SHARE_READ_WRITE,
                0u32,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                0u32
            ],
        )
        .u32()
    });
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut out = Vec::new();
    e.with_stack(0x1004, |e, frame| {
        let (buf, read) = (frame.addr(), frame.addr() + 0x1000);
        loop {
            e.mem.set_u32(read, 0);
            let ok = e
                .call(READ_FILE, &args![handle, buf, 0x1000u32, read, 0u32])
                .u32();
            let n = e.mem.u32(read);
            if ok == 0 || n == 0 {
                break;
            }
            out.extend(e.mem.bytes(buf, n));
        }
    });
    e.call(CLOSE_HANDLE, &args![handle]);
    Some(out)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CachedScripts.cpp:
// CompileAndCacheScript(std::filesystem::path, bool, const char*).
/// Compiles the file at `full_path` (relative to the game's working folder) as a function script
/// and caches it, by `rel_path` when given, else by the path below the function folder: the
/// script, or 0 for a file without an extension, one that cannot be read, or one that does not
/// compile (nothing is cached then).
fn compile_and_cache_path(e: &mut Engine, full_path: &[u8], rel_path: Option<&[u8]>) -> u32 {
    let (stem, extension) = stem_and_extension(file_name(full_path));
    if extension.is_empty() {
        return 0;
    }
    let stem = stem.to_vec();
    let Some(bytes) = read_file(e, full_path) else {
        return 0;
    };
    let text = text_mode(&bytes);
    let count = state(e).cached.len();
    let mut name = format!("nvseRuntimeScript{count}").into_bytes();
    name.extend_from_slice(&stem);
    let script = compile_script_ex(e, &text, Some(&name), true);
    if script == 0 {
        return 0;
    }
    crate::events::with_guest_text(e, &name, |e, at| {
        e.vcall(script, SET_EDITOR_ID, &args![at]);
    });
    let key = match rel_path {
        Some(rel) => rel.to_vec(),
        None => full_path
            .get(SCRIPT_FILES_PATH.len()..)
            .unwrap_or_default()
            .to_vec(),
    };
    cache(e, &key, script);
    script
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CachedScripts.cpp:
// CompileAndCacheScript(const char*).
/// Compiles and caches the function file at `rel_path` below the function folder: the script, or
/// 0 when the file is missing or does not compile.
pub fn compile_and_cache(e: &mut Engine, rel_path: &[u8]) -> u32 {
    let mut full = SCRIPT_FILES_PATH.to_vec();
    full.extend_from_slice(rel_path);
    compile_and_cache_path(e, &full, Some(rel_path))
}

/// One entry of a folder search: its name and its attributes.
struct Found {
    name: Vec<u8>,
    attributes: u32,
}

/// The entries of the folder `dir` (a path ending in a separator), as a search of `dir*` lists
/// them through the game's `FindFirstFileA`/`FindNextFileA`, without `.` and `..`; none when
/// the folder is missing.
fn list_folder(e: &mut Engine, dir: &[u8]) -> Vec<Found> {
    let mut pattern = dir.to_vec();
    pattern.push(b'*');
    let mut out = Vec::new();
    e.with_stack(FIND_DATA_SIZE, |e, data| {
        let data = data.addr();
        let handle = crate::events::with_guest_text(e, &pattern, |e, p| {
            e.call(FIND_FIRST_FILE_A, &args![p, data]).u32()
        });
        if handle == INVALID_HANDLE_VALUE {
            return;
        }
        loop {
            let name = crate::table::guest_text(e, data + FIND_DATA_NAME);
            if name != b"." && name != b".." {
                out.push(Found {
                    name,
                    attributes: e.mem.u32(data),
                });
            }
            if e.call(FIND_NEXT_FILE_A, &args![handle, data]).u32() == 0 {
                break;
            }
        }
        e.call(FIND_CLOSE, &args![handle]);
    });
    out
}

/// The regular files below `dir` in `std::filesystem::recursive_directory_iterator`'s order: each
/// folder's entries in the order the search answers them, a sub-folder's entries right after the
/// sub-folder (directory links are not followed). A file's path is the folder's and its name
/// (a sub-folder's children after a `\`).
fn walk(e: &mut Engine, dir: &[u8], files: &mut Vec<Vec<u8>>) {
    for found in list_folder(e, dir) {
        let mut path = dir.to_vec();
        path.extend_from_slice(&found.name);
        if found.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            continue;
        }
        if found.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
            path.push(b'\\');
            walk(e, &path, files);
        } else {
            files.push(path);
        }
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CachedScripts.cpp: CacheAllScriptsInPath.
/// Compiles and caches every function file below [`SCRIPT_FILES_PATH`] that has an extension;
/// one that fails is named on the console and in `nvse.log`, one that compiles in `nvse.log`.
pub fn cache_all_scripts(e: &mut Engine) {
    let mut files = Vec::new();
    walk(e, SCRIPT_FILES_PATH, &mut files);
    for path in files {
        let (_, extension) = stem_and_extension(file_name(&path));
        if extension.is_empty() {
            continue;
        }
        let shown = String::from_utf8_lossy(&path).into_owned();
        if compile_and_cache_path(e, &path, None) == 0 {
            let message = format!("xNVSE: Failed to precompile script file at {shown}");
            crate::errors::print_long(e, message.as_bytes());
            crate::log::message(e, message.as_bytes());
        } else {
            let message = format!("xNVSE: script file successfully precompiled at {shown}");
            crate::log::message(e, message.as_bytes());
        }
    }
}

// ---- the command ----

/// `kNVSEParamType_String` and `kNVSEParamType_Boolean`.
const NVSE_STRING: u32 = 0x8004;
const NVSE_BOOLEAN: u32 = 0x8002;
/// `CompileScript`'s opcode.
pub const OP_COMPILE_SCRIPT: u32 = 0x1668;

pub static COMMANDS: &[Command] = &[Command {
    opcode: OP_COMPILE_SCRIPT,
    name: "CompileScript",
    short: "GetUDFFromFile",
    // `DEFINE_CMD_ALT_EXP` stringifies the quoted description: the help text keeps its quotes
    // (as in the shipped DLL).
    help: "\"Returns a compiled script from a file, to call as a UDF.\"",
    needs_ref: false,
    params: &[
        Param {
            name: "string",
            kind: NVSE_STRING,
            optional: false,
        },
        Param {
            name: "bool",
            kind: NVSE_BOOLEAN,
            optional: true,
        },
    ],
    execute: compile_script_command,
    eval: None,
    returns: Returns::Form,
}];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_CompileScript_Execute.
/// `CompileScript path[, recompile]`: the function file's script (cached, or compiled now when it
/// is not cached or `recompile` is true), its form id in the result; 0 for an empty path, a
/// missing file or one that does not compile.
fn compile_script_command(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    crate::commands::string::with_eval(e, &a, |e, ev, toks| {
        let Some(toks) = toks else {
            return;
        };
        let Some(first) = toks.first() else {
            return;
        };
        let path = ev.get_string(e, first);
        let path = crate::commands::string::c_str(&path);
        if path.is_empty() {
            return;
        }
        let path = replace_all(path, b"/", b"\\");
        let recompile = toks.len() > 1 && ev.get_bool(e, &toks[1]);
        let script = if recompile {
            compile_and_cache(e, &path)
        } else {
            match cached(e, &path) {
                0 => compile_and_cache(e, &path),
                s => s,
            }
        };
        if script != 0 {
            let id = e.mem.u32(script + 0xc);
            a.set_form_result(e, id);
        }
    });
    handled(true)
}

#[cfg(test)]
#[path = "compile_script_tests.rs"]
mod tests;
