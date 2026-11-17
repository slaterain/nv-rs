//! `functions_ln/ln_fn_utility.h` (JIP LN NVSE): a form list as an array, sorting
//! and counting forms by type, the time of day as text, and (lane files) whether a file is
//! there and a mod's INI files read and written (`FileExists`, `GetINIFloat`, `SetINIFloat`,
//! `GetINIString`, `SetINIString`, `GetINISection`, `SetINISection`, `GetINISectionNames`,
//! `RemoveINIKey`, `RemoveINISection`) through the core's seam `engine_core::mod_files`: a mod's
//! writes are kept in the port's own folder beside the saves, never in the game folder; and
//! `GetDefaultMessageTime`/`SetDefaultMessageTime`, the time JIP's `QueueUIMessageHook` (G09,
//! `crate::game_patches::ga`) gives the game's own messages (with JIP's fixes off it is kept but no
//! message uses it, as with JIP). 16 of its 20 commands (`GetLNVersion` is the core track's,
//! `identity.rs`).
//!
//! Left for later, each for its reason: `GetFilesInFolder`, `GetFoldersInFolder` list a folder
//! (`FindFirstFileA`; the seam has no folder listing yet); `Console` compiles and runs a script
//! line (`JIPScriptRunner`, not built yet).

use super::arrays::{create_array, create_string_map, set_element, Element};
use super::forms::ListNode;
use super::jip_fn_aux_variable::array_values;
use super::{list_items, type_of, Handlers};
use crate::cosave::lookup_form;
use engine_core::prelude::*;
use nvse::args::Arg;
use nvse::commands::Read;
use nvse::vars::{Container, Data, Key};
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2201, file_exists),
    (0x2202, get_time_stamp),
    (0x2212, list_to_array),
    (0x227a, get_ini_float),
    (0x227b, get_ini_string),
    (0x227c, set_ini_float),
    (0x227d, set_ini_string),
    (0x227e, get_ini_section),
    (0x227f, set_ini_section),
    (0x2285, get_ini_section_names),
    (0x22ae, remove_ini_key),
    (0x22af, remove_ini_section),
    (0x22ff, sort_forms_by_type),
    (0x2300, get_form_count_type),
    (0x2327, get_default_message_time),
    (0x2328, set_default_message_time),
];

/// A form list's forms (`BGSListForm::list`, a `tList` at +0x18).
const LIST_FORMS: u32 = 0x18;
/// The game's import slot of `GetLocalTime` (`00FDF0D8`; the save code calls it there).
const GET_LOCAL_TIME_IMPORT: u32 = 0x00fd_f0d8;
/// `IS_REFERENCE`: the vtable answers true (`008D0360`) at +0xF0.
const RETURN_TRUE: u32 = 0x008d_0360;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetTimeStamp_Execute; internal/utility.cpp: GetTimeStamp.
/// The local time as `HH-MM-SS` (JIP's `localtime_s` of the clock: the hour, minute and second
/// `GetLocalTime` gives), printed in the console when it is open (`IsConsoleOpen`) and given as
/// the string result.
fn get_time_stamp(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let (h, m, s) = e.with_stack(16, |e, st| {
        e.call(GET_LOCAL_TIME_IMPORT, &args![st]);
        let at = st.addr();
        (e.mem.u16(at + 8), e.mem.u16(at + 10), e.mem.u16(at + 12))
    });
    let text = format!("{h:02}-{m:02}-{s:02}").into_bytes();
    if nvse::commands::is_console_mode(e) {
        crate::console::console_print_line(e, &text);
    }
    super::assign_string(e, &a, Some(text));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_ListToArray_Execute.
/// The list's forms in order (empty nodes left out) as a new array.
fn list_to_array(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let list = read.form(0);
        let forms: Vec<Element> = list_items(e, Ptr::<ListNode>::new(list + LIST_FORMS))
            .into_iter()
            .filter(|&f| f != 0)
            .map(Element::Form)
            .collect();
        let id = create_array(e, a.script, &forms);
        a.set_result(e, f64::from(id as i32));
    }
    handled(true)
}

/// `GetElements` of a plugin: each value of the array in its order, a form looked up (0 when
/// gone), a number, else nothing a form or number reader takes.
fn plugin_forms(e: &Engine, values: &[Data]) -> Vec<u32> {
    values
        .iter()
        .map(|d| match d {
            Data::Form(id) => lookup_form(e, *id),
            _ => 0,
        })
        .collect()
}

fn plugin_number(d: &Data) -> f64 {
    match d {
        Data::Number(n) => *n,
        _ => 0.0,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_SortFormsByType_Execute.
/// The forms of the first array (non-null, in order), grouped by the types of the second: for
/// each type (its number as a byte), the forms of that type (a reference also by its base
/// form's type) taken from the end of what is left back to the start; then what is left in
/// order. Nothing when either array is missing or empty.
fn sort_forms_by_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (Some(forms), Some(types)) = (
        array_values(e, read.int(0, 0)),
        array_values(e, read.int(1, 0)),
    ) else {
        return handled(true);
    };
    if forms.is_empty() || types.is_empty() {
        return handled(true);
    }
    let mut left: Vec<u32> = plugin_forms(e, &forms)
        .into_iter()
        .filter(|&f| f != 0)
        .collect();
    let mut out = Vec::new();
    for t in &types {
        let kind = super::cvttsd2si(plugin_number(t)) as u8;
        let mut i = left.len();
        while i > 0 {
            i -= 1;
            let f = left[i];
            let matches = type_of(e, f) == kind
                || (e.mem.u32(e.mem.u32(f) + 0xf0) == RETURN_TRUE
                    && type_of(e, e.mem.u32(f + 0x20)) == kind);
            if matches {
                out.push(Element::Form(f));
                left.remove(i);
            }
        }
        if left.is_empty() {
            break;
        }
    }
    out.extend(left.into_iter().map(Element::Form));
    let id = create_array(e, a.script, &out);
    a.set_result(e, f64::from(id as i32));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetFormCountType_Execute.
/// How many forms of the type a packed array holds (a map, a missing or an empty array:
/// nothing).
fn get_form_count_type(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (id, kind) = (read.int(0, 0), read.int(1, 0));
    let packed =
        nvse::vars::array(e, id).is_some_and(|v| matches!(v.elements, Container::Array(_)));
    let Some(values) = array_values(e, id).filter(|v| packed && !v.is_empty()) else {
        return handled(true);
    };
    let count = plugin_forms(e, &values)
        .into_iter()
        .filter(|&f| f != 0 && u32::from(type_of(e, f)) == kind)
        .count();
    a.set_result(e, count as f64);
    handled(true)
}

// ---- Mods' files: FileExists and the INI commands ----
//
// Every name goes through the core's seam (`engine_core::mod_files`): a mod's writes are kept in
// the port's own folder beside the saves, never the game's folder; reads look there first, then
// the game's Data. A `..` takes off the part before it (`..\NVSE\Plugins\x.ini` from the config
// folder is Data's). A name the seam refuses (a `..` past the game's folder, another drive, a
// device) reads as a file that is not there and is never written (result 0, as JIP answers for a
// file Windows could not open).

/// The text `"data\\"` the exe holds at `010A4828` (`GetFileArchived`'s `StrBeginsCI`), and the
/// folder JIP puts before `FileExists`'s name.
const DATA_FOLDER: &[u8] = b"data\\";
/// `GetINIPath`'s folder: `"data\\config\\"` (12 bytes) before the script's name.
const CONFIG_FOLDER: &[u8] = b"data\\config\\";
/// JIP's buffers: `char dataPath[0x80]` (the name after its 5 bytes), `char configPath[0x80]`
/// (the name after its 12), `char valueName[0x80]`, `char secName[0x40]`, `char valStr[0x20]`;
/// `GetStrArgBuffer()` (`STR_BUFFER_SIZE`, 0x40000) and `kMaxMessageLength` (0x4000); the
/// section read's 0x10000.
const DATA_PATH_ARG: usize = 0x80 - 5;
const CONFIG_PATH_ARG: usize = 0x80 - 12;
const VALUE_NAME: usize = 0x80;
const SEC_NAME: usize = 0x40;
const VAL_STR: usize = 0x20;
const STR_ARG_BUFFER: usize = 0x40000;
const MAX_MESSAGE_LENGTH: usize = 0x4000;
const SECTION_BUFFER: usize = 0x10000;
/// `g_dataHandler` (`*011C3F2C`): its loaded files (`modList.loadedMods[255]`, +0x21C), a
/// file's name (`ModInfo::name`, +0x20).
const DATA_HANDLER: u32 = 0x011c_3f2c;
const LOADED_MODS: u32 = 0x21c;
const MOD_NAME: u32 = 0x20;
/// A script's mod index: its form id's high byte (`Script +0xF`).
const SCRIPT_MOD_INDEX: u32 = 0xf;
/// `GetFileArchived`'s game calls: the path hashed into a folder and a file hash (`00AFD270`,
/// `cdecl`, the path and the two 8-byte hashes), an archive's lookup (`00AF9BF0`, `thiscall`:
/// the hashes, two out words and the path); the opened archives (`*011F8160`, list nodes of
/// data +0 and next +4) and an archive's type flags (a word at +0x180).
const HASH_PATH: u32 = 0x00af_d270;
const ARCHIVE_HAS_FILE: u32 = 0x00af_9bf0;
const ARCHIVE_LIST: u32 = 0x011f_8160;
const ARCHIVE_TYPES: u32 = 0x180;

/// `s_fileExtToType` (JIP's `InitContainers`): the archive types a file of each extension is
/// looked for in. JIP's map keeps the extension's `StrHashCI` only, so a lookup compares hashes.
const FILE_EXT_TO_TYPE: [(&[u8], u16); 19] = [
    (b"nif", 1),
    (b"egm", 1),
    (b"egt", 1),
    (b"kf", 1),
    (b"psa", 1),
    (b"tri", 1),
    (b"dds", 2),
    (b"fnt", 2),
    (b"psd", 2),
    (b"tai", 2),
    (b"tex", 2),
    (b"wav", 8),
    (b"lip", 0x10),
    (b"ogg", 0x10),
    (b"spt", 0x40),
    (b"ctl", 0x100),
    (b"dat", 0x100),
    (b"dlodsettings", 0x100),
    (b"xml", 0x100),
];

/// `ReplaceChr(path, '/', '\\')`.
fn backslashes(path: &mut [u8]) {
    for b in path {
        if *b == b'/' {
            *b = b'\\';
        }
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: GetFileArchived.
/// Whether one of the opened archives holds `path` (with or without `data\` before it): looked
/// for only when its extension is in [`FILE_EXT_TO_TYPE`], only in archives of that type.
fn file_archived(e: &mut Engine, path: &[u8]) -> bool {
    let Some(dot) = path.iter().rposition(|&b| b == b'.') else {
        return false;
    };
    let hash = crate::containers::str_hash_ci(&path[dot + 1..]);
    let Some(&(_, kind)) = FILE_EXT_TO_TYPE
        .iter()
        .find(|(ext, _)| crate::containers::str_hash_ci(ext) == hash)
    else {
        return false;
    };
    let path = if crate::utility::str_begins_ci(path, DATA_FOLDER) != 0 {
        &path[DATA_FOLDER.len()..]
    } else {
        path
    };
    super::with_guest_text(e, path, |e, text| {
        // The frame: the folder hash (+0, 8 bytes), the file hash (+8), the two out words.
        e.with_stack(0x18, |e, frame| {
            let (dir, name) = (frame.addr(), frame.addr() + 8);
            let (out1, out2) = (frame.addr() + 0x10, frame.addr() + 0x14);
            e.call(HASH_PATH, &args![text, dir, name]);
            let mut node = e.mem.u32(ARCHIVE_LIST);
            while node != 0 {
                let archive = e.mem.u32(node);
                node = e.mem.u32(node + 4);
                if archive == 0 || e.mem.u16(archive + ARCHIVE_TYPES) & kind == 0 {
                    continue;
                }
                if e.call(
                    ARCHIVE_HAS_FILE,
                    &args![archive, dir, name, out1, out2, text],
                )
                .bool()
                {
                    return true;
                }
            }
            false
        })
    })
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: FileExistsEx.
/// `GetFileAttributesA` of `path` (`/` made `\`) through the seam: 1 when it is there as asked (a
/// folder or a file), 0 when it is there as the other; when it is not there, a file found in an
/// opened archive ([`file_archived`]) answers 2.
fn file_exists_ex(e: &mut Engine, path: &mut [u8], folder: bool) -> u32 {
    backslashes(path);
    if engine_core::mod_files::directory_exists(e, path) {
        return u32::from(folder);
    }
    if engine_core::mod_files::exists(e, path) {
        return u32::from(!folder);
    }
    if folder {
        return 0;
    }
    u32::from(file_archived(e, path)) << 1
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_FileExists_Execute.
/// Whether `data\<name>` is there: 1 as a file (or, asked for a folder, as a folder), 2 as a
/// file in an opened archive, 0 otherwise; printed in the console when it is open.
fn file_exists(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let mut path = DATA_FOLDER.to_vec();
        path.extend(super::buffer_arg(e, &read, 0, DATA_PATH_ARG, "FileExists"));
        let folder = read.int(1, 0) != 0;
        let found = file_exists_ex(e, &mut path, folder);
        a.set_result(e, f64::from(found));
    }
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h: GetINIPath.
/// The INI file a command names: `data\config\` and the name given (`/` made `\`), or, with
/// none, the name of the script's mod; the part after its last `.` made `ini`, or `.ini` added
/// when it has none. `None` without a name and with no mod (a script of no file, index 0xFF, or
/// a slot with no file).
fn ini_path(e: &Engine, script: u32, given: Vec<u8>) -> Option<Vec<u8>> {
    let mut name = given;
    if name.is_empty() {
        // JIP reads the script's index byte; a call without a script has none here.
        if script == 0 {
            return None;
        }
        let index = e.mem.u8(script + SCRIPT_MOD_INDEX);
        if index == 0xff {
            return None;
        }
        let info = e
            .mem
            .u32(e.mem.u32(DATA_HANDLER) + LOADED_MODS + 4 * u32::from(index));
        if info == 0 {
            return None;
        }
        name = nvse::table::guest_text(e, info + MOD_NAME);
    } else {
        backslashes(&mut name);
    }
    match name.iter().rposition(|&b| b == b'.') {
        Some(dot) => {
            name.truncate(dot + 1);
            name.extend_from_slice(b"ini");
        }
        None => name.extend_from_slice(b".ini"),
    }
    Some([CONFIG_FOLDER, &name].concat())
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h: GetValueDelim.
/// `section:key` split at the first `:`, `/` or `\`; `None` when there is none or the key is
/// empty (the commands then do nothing).
fn split_value_name(name: &[u8]) -> Option<(&[u8], &[u8])> {
    let at = name
        .iter()
        .position(|&b| matches!(b, b':' | b'/' | b'\\'))?;
    let key = &name[at + 1..];
    (!key.is_empty()).then_some((&name[..at], key))
}

/// `FileStream::MakeAllDirs(path)`: each folder before a `\` of the path made.
fn make_all_dirs(e: &mut Engine, path: &[u8]) {
    for (i, &b) in path.iter().enumerate() {
        if b == b'\\' {
            engine_core::mod_files::create_directory(e, &path[..i]);
        }
    }
}

/// `GetPrivateProfileStringA` into a buffer of `size` bytes: the value cut to `size - 1` bytes
/// (measured: Windows cuts after taking the quotes off); `None` when it is not there.
fn profile_string_in(
    e: &Engine,
    path: &[u8],
    section: &[u8],
    key: &[u8],
    size: usize,
) -> Option<Vec<u8>> {
    let mut value = engine_core::mod_files::profile_string(e, path, section, key)?;
    value.truncate(size - 1);
    Some(value)
}

/// The texts of a buffer Windows filled ([`engine_core::mod_files::profile_list`]), as JIP walks
/// it: while the count left is above 0, the text up to its 0, the count less its length and 1.
fn walked(buffer: &[u8], count: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let (mut left, mut at) = (count as isize, 0usize);
    while left > 0 {
        let len = buffer[at..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(buffer.len() - at);
        out.push(buffer[at..at + len].to_vec());
        left -= len as isize + 1;
        at += len + 1;
    }
    out
}

/// The INI file of argument `i` ([`ini_path`]).
fn ini_arg(e: &Engine, a: &CommandArgs, read: &Read, i: usize, command: &str) -> Option<Vec<u8>> {
    let given = super::buffer_arg(e, read, i, CONFIG_PATH_ARG, command);
    ini_path(e, a.script, given)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetINIFloat_Execute.
/// `section:key`'s value as a number (JIP's `StrToDbl` of its first 31 bytes); nothing when it
/// is not there or empty.
fn get_ini_float(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = super::buffer_arg(e, &read, 0, VALUE_NAME, "GetINIFloat");
    let Some(path) = ini_arg(e, &a, &read, 1, "GetINIFloat") else {
        return handled(true);
    };
    if let Some((section, key)) = split_value_name(&name) {
        if let Some(value) = profile_string_in(e, &path, section, key, VAL_STR) {
            if !value.is_empty() {
                a.set_result(e, crate::utility::str_to_dbl(&value));
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_SetINIFloat_Execute.
/// `section:key` written as JIP's `FltToStr` of the number (the file's folders made when the
/// file is not there); 1 when written.
fn set_ini_float(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = super::buffer_arg(e, &read, 0, VALUE_NAME, "SetINIFloat");
    let value = match read.args.get(1) {
        Some(Arg::Double(d)) => *d,
        _ => 0.0,
    };
    let Some(path) = ini_arg(e, &a, &read, 2, "SetINIFloat") else {
        return handled(true);
    };
    if let Some((section, key)) = split_value_name(&name) {
        set_ini_value(
            e,
            &a,
            &path,
            section,
            key,
            &crate::utility::flt_to_str(value),
        );
    }
    handled(true)
}

/// `WritePrivateProfileStringA` after `MakeAllDirs` when the file is not there: result 1 when
/// written.
fn set_ini_value(
    e: &mut Engine,
    a: &CommandArgs,
    path: &[u8],
    section: &[u8],
    key: &[u8],
    value: &[u8],
) {
    if !engine_core::mod_files::exists(e, path) {
        make_all_dirs(e, path);
    }
    if engine_core::mod_files::write_profile_string(e, path, section, key, value).is_ok() {
        a.set_result(e, 1.0);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetINIString_Execute.
/// `section:key`'s value as text (up to 0x3FFF bytes); `""` when it is not there.
fn get_ini_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let mut text = Vec::new();
    if read.ok {
        let name = super::buffer_arg(e, &read, 0, VALUE_NAME, "GetINIString");
        if let Some(path) = ini_arg(e, &a, &read, 1, "GetINIString") {
            if let Some((section, key)) = split_value_name(&name) {
                text = profile_string_in(e, &path, section, key, MAX_MESSAGE_LENGTH)
                    .unwrap_or_default();
            }
        }
    }
    super::assign_string(e, &a, Some(text));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_SetINIString_Execute.
/// `section:key` written as the text given; 1 when written.
fn set_ini_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = super::buffer_arg(e, &read, 0, VALUE_NAME, "SetINIString");
    let value = super::buffer_arg(e, &read, 1, STR_ARG_BUFFER, "SetINIString");
    let Some(path) = ini_arg(e, &a, &read, 2, "SetINIString") else {
        return handled(true);
    };
    if let Some((section, key)) = split_value_name(&name) {
        set_ini_value(e, &a, &path, section, key, &value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetINISection_Execute; internal/utility.cpp: GetNextToken (char).
/// The section's entries as a string map: each `key=value` with a value (the key up to the first
/// `=`, the value after the `=`s there), the value as text, or as a number (`StrToDbl`) when
/// asked; an empty map when the file or section is not there.
fn get_ini_section(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let section = super::buffer_arg(e, &read, 0, SEC_NAME, "GetINISection");
    let Some(path) = ini_arg(e, &a, &read, 1, "GetINISection") else {
        return handled(true);
    };
    let numeric = read.int(2, 0) != 0;
    let id = create_string_map(e, a.script, &[], &[]);
    let entries = engine_core::mod_files::profile_section(e, &path, &section).unwrap_or_default();
    let (buffer, count) = engine_core::mod_files::profile_list(&entries, SECTION_BUFFER);
    for line in walked(&buffer, count) {
        let Some(eq) = line.iter().position(|&b| b == b'=') else {
            continue;
        };
        let value_at = line[eq..]
            .iter()
            .position(|&b| b != b'=')
            .map_or(line.len(), |n| eq + n);
        let value = &line[value_at..];
        if value.is_empty() {
            continue;
        }
        let element = if numeric {
            Element::Number(crate::utility::str_to_dbl(value))
        } else {
            Element::Str(value.to_vec())
        };
        set_element(e, id, &Key::Str(line[..eq].to_vec()), &element);
    }
    a.set_result(e, f64::from(id as i32));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_SetINISection_Execute.
/// The section written whole from a string map: each `key=value`, a text value as it is, any
/// other as JIP's `FltToStr` of its number (a form or an array has none: `0`); 1 when written.
/// Nothing for a missing or empty array, or one that is not a string map.
fn set_ini_section(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let section = super::buffer_arg(e, &read, 0, SEC_NAME, "SetINISection");
    let id = read.int(1, 0);
    let Some(path) = ini_arg(e, &a, &read, 2, "SetINISection") else {
        return handled(true);
    };
    let lines: Vec<Vec<u8>> = match nvse::vars::array(e, id).map(|v| &v.elements) {
        Some(Container::StrMap(items)) if !items.is_empty() => items
            .iter()
            .map(|(key, el)| {
                let value = match &el.data {
                    Data::Str(text) => text.clone(),
                    Data::Number(v) => crate::utility::flt_to_str(*v),
                    _ => crate::utility::flt_to_str(0.0),
                };
                [&key[..], b"=", &value[..]].concat()
            })
            .collect(),
        _ => return handled(true),
    };
    if !engine_core::mod_files::exists(e, &path) {
        make_all_dirs(e, &path);
    }
    if engine_core::mod_files::write_profile_section(e, &path, &section, &lines).is_ok() {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetINISectionNames_Execute.
/// The file's section names, in order, as an array (empty when the file is not there).
fn get_ini_section_names(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(path) = ini_arg(e, &a, &read, 0, "GetINISectionNames") else {
        return handled(true);
    };
    let names = engine_core::mod_files::profile_section_names(e, &path).unwrap_or_default();
    let (buffer, count) = engine_core::mod_files::profile_list(&names, MAX_MESSAGE_LENGTH);
    let elements: Vec<Element> = walked(&buffer, count)
        .into_iter()
        .map(Element::Str)
        .collect();
    let id = create_array(e, a.script, &elements);
    a.set_result(e, f64::from(id as i32));
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_RemoveINIKey_Execute.
/// `section:key` taken out of the file when the file is there; 1 then (also when the key was
/// not there, as Windows answers).
fn remove_ini_key(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = super::buffer_arg(e, &read, 0, VALUE_NAME, "RemoveINIKey");
    let Some(path) = ini_arg(e, &a, &read, 1, "RemoveINIKey") else {
        return handled(true);
    };
    if !engine_core::mod_files::exists(e, &path) {
        return handled(true);
    }
    if let Some((section, key)) = split_value_name(&name) {
        if engine_core::mod_files::remove_profile_key(e, &path, section, key).is_ok() {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_RemoveINISection_Execute.
/// The section taken out of the file when the file is there; 1 then.
fn remove_ini_section(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let section = super::buffer_arg(e, &read, 0, SEC_NAME, "RemoveINISection");
    let Some(path) = ini_arg(e, &a, &read, 1, "RemoveINISection") else {
        return handled(true);
    };
    if engine_core::mod_files::exists(e, &path)
        && engine_core::mod_files::remove_profile_section(e, &path, &section).is_ok()
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_utility_tests.rs"]
mod tests;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetDefaultMessageTime_Execute.
/// The default message time (`s_msgDisplayTime`), echoed in the console (`DoConsolePrint`).
fn get_default_message_time(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let seconds = crate::game_patches::ga::default_message_time(e);
    a.set_result(e, f64::from(seconds));
    crate::console::print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_SetDefaultMessageTime_Execute.
/// Sets the default message time to its float when the argument is read; the result is left as
/// it was.
fn set_default_message_time(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let seconds = read.float(0, 0.0);
        crate::game_patches::ga::set_default_message_time(e, seconds);
    }
    handled(true)
}
