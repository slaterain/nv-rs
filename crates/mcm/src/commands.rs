//! MCM.dll's nine commands at their opcodes (`0x21C0` up), from the DLL's code
//! (`build43-scratch/measure/mcm/NOTES.md`, sections 1 to 3). Each answers true, also when its
//! arguments cannot be read (the script goes on). The arguments are read as the DLL reads them:
//! through the exe's `ExtractArgs`, which xNVSE (and nv-rs's `crates/nvse`) turns into its own
//! reader for every extender opcode ([`Read`]); the format strings through
//! `ExtractFormatStringArgs` (the DLL's own copy of NVSE 2's; here xNVSE's, which reads `%g`,
//! `%.Nf` and plain text, all MCM's menus use, the same way).
//!
//! What differs from the DLL, each where the DLL would write past a buffer, hang or crash: a text
//! of [`PATH_BUFFER`] bytes or more (the DLL's 16 KB stack buffers) does nothing; an INI name,
//! section or key of 100 bytes or more (its 100-byte buffers) does nothing; a name of separators
//! only (the DLL copies from a null) does nothing; the `Mass` commands stop after [`MASS_STEPS`]
//! indices (the DLL's loop runs up to four billion times). And `GetMCMFloat`'s console line
//! prints the number: the DLL hands the console the result's address where its format wants the
//! number, so it prints an unrelated value.

use crate::menu::{self, PATH_BUFFER};
use engine_core::mod_files;
use engine_core::prelude::*;
use nvse::args::Arg;
use nvse::commands::string::format_string_args_at;
use nvse::commands::{console_print, double_words, Read};
use nvse::table::{self, Param, PluginRecord, Returns};
use nvse::{handled, CommandArgs};

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}

/// `kParamType_String`, `_Integer`, `_Float`, `_TESObject` (0x15).
const STRING: u32 = 0x00;
const INT: u32 = 0x01;
const FLOAT: u32 = 0x02;
const OBJECT: u32 = 0x15;

/// The DLL's parameter lists.
const ONE_STRING: &[Param] = &[p("string", STRING, false)];
const ONE_STRING_ONE_FLOAT: &[Param] = &[p("string", STRING, false), p("float", FLOAT, false)];
const GET_FLOAT: &[Param] = &[
    p("Child", INT, false),
    p("Grandchild", INT, false),
    p("Setting", STRING, false),
];
const SET_FLOAT: &[Param] = &[
    p("Child", INT, false),
    p("Grandchild", INT, false),
    p("Setting", STRING, false),
    p("Value", FLOAT, true),
];
const SET_FLOAT_MASS: &[Param] = &[
    p("Child", INT, false),
    p("Min", INT, false),
    p("Max", INT, false),
    p("Setting", STRING, false),
    p("Value", FLOAT, true),
];
const SET_STRING: [Param; 24] = formatted(&[
    p("Child", INT, false),
    p("Grandchild", INT, false),
    p("Setting", STRING, false),
]);
const SET_STRING_MASS: [Param; 25] = formatted(&[
    p("Child", INT, false),
    p("Min", INT, false),
    p("Max", INT, false),
    p("Setting", STRING, false),
]);
const MOD_LIST: &[Param] = &[
    p("Mod Number", INT, false),
    p("Enable", INT, false),
    p("Index", INT, true),
    p("Index2", INT, true),
    p("Name Ref", OBJECT, true),
];
const LIST_WIDTH: &[Param] = &[p("int", INT, false), p("int", INT, false)];

/// `leading`, then the format string and its twenty optional floats.
const fn formatted<const N: usize>(leading: &[Param]) -> [Param; N] {
    let mut out = [p("variable", FLOAT, true); N];
    let mut i = 0;
    while i < leading.len() {
        out[i] = leading[i];
        i += 1;
    }
    out[leading.len()] = p("format string", STRING, false);
    out
}

/// One command as the DLL registers it.
pub struct Cmd {
    pub opcode: u32,
    pub name: &'static str,
    pub short: &'static str,
    /// The help text, its quotes part of it as in the DLL.
    pub help: &'static str,
    pub params: &'static [Param],
    pub execute: AbiFn,
}

/// The DLL's commands, in its order of registration (opcodes from `0x21C0`).
pub static COMMANDS: [Cmd; 9] = [
    Cmd {
        opcode: 0x21c0,
        name: "GetModINISetting",
        short: "GetModINI",
        help: "\"Gets INI setting\"",
        params: ONE_STRING,
        execute: get_mod_ini_setting,
    },
    Cmd {
        opcode: 0x21c1,
        name: "SetModINISetting",
        short: "SetModINI",
        help: "\"Sets INI setting\"",
        params: ONE_STRING_ONE_FLOAT,
        execute: set_mod_ini_setting,
    },
    Cmd {
        opcode: 0x21c2,
        name: "GetMCMFloat",
        short: "",
        help: "\"Gets MCM float value\"",
        params: GET_FLOAT,
        execute: get_mcm_float,
    },
    Cmd {
        opcode: 0x21c3,
        name: "SetMCMFloat",
        short: "",
        help: "\"Sets MCM float value\"",
        params: SET_FLOAT,
        execute: set_mcm_float,
    },
    Cmd {
        opcode: 0x21c4,
        name: "SetMCMString",
        short: "",
        help: "\"Sets MCM string value\"",
        params: &SET_STRING,
        execute: set_mcm_string,
    },
    Cmd {
        opcode: 0x21c5,
        name: "SetMCMFloatMass",
        short: "",
        help: "\"Sets multiple MCM float values\"",
        params: SET_FLOAT_MASS,
        execute: set_mcm_float_mass,
    },
    Cmd {
        opcode: 0x21c6,
        name: "SetMCMStringMass",
        short: "",
        help: "\"Sets multiple MCM string values\"",
        params: &SET_STRING_MASS,
        execute: set_mcm_string_mass,
    },
    Cmd {
        opcode: 0x21c7,
        name: "SetMCMModList",
        short: "",
        help: "\"Sets ModList info\"",
        params: MOD_LIST,
        execute: set_mcm_mod_list,
    },
    Cmd {
        opcode: 0x21c8,
        name: "GetMCMListWidth",
        short: "",
        help: "\"Gets ModList width\"",
        params: LIST_WIDTH,
        execute: get_mcm_list_width,
    },
];

/// The record's help word (`CommandInfo::helpText`, `+0xC`).
const RECORD_HELP: u32 = 0xc;

/// A parameter list in the built-ins' data (12 bytes a parameter: name, type, optional).
fn guest_params(e: &mut Engine, params: &[Param]) -> u32 {
    let at = table::alloc(e, 12 * params.len() as u32);
    for (i, param) in params.iter().enumerate() {
        let name = table::guest_str(e, param.name.as_bytes());
        let slot = at + 12 * i as u32;
        e.mem.set_u32(slot, name);
        e.mem.set_u32(slot + 4, param.kind);
        e.mem.set_u32(slot + 8, u32::from(param.optional));
    }
    at
}

/// Registers the nine commands at their opcodes (`RegisterCommand`): the table grown to them
/// when JIP has not already grown it past, each record with its names, help, parameters and
/// execute.
pub fn install(e: &mut Engine) {
    table::grow_to(e, crate::OPCODE_BASE + COMMANDS.len() as u32);
    for c in &COMMANDS {
        let execute = table::execute_address(c.opcode);
        e.register(execute, c.execute);
        let record = PluginRecord {
            name: c.name,
            short: c.short,
            needs_ref: false,
            param_count: c.params.len() as u16,
            params: guest_params(e, c.params),
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        };
        table::add_plugin_record(e, c.opcode, &record);
        let help = table::guest_str(e, c.help.as_bytes());
        let at = table::record_of(e, c.opcode);
        e.mem.set_u32(at + RECORD_HELP, help);
    }
}

/// The most indices a `Mass` command walks.
pub const MASS_STEPS: u32 = 1 << 16;

/// A text argument the DLL holds in a 16 KB buffer, as a C string (up to its first 0): `None`
/// when it would not fit.
fn fits(text: Vec<u8>) -> Option<Vec<u8>> {
    if text.len() >= PATH_BUFFER {
        return None;
    }
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    Some(text[..end].to_vec())
}

/// An integer argument of a formatted read's leading ones (0 when not read).
fn leading_int(args: &[Arg], i: usize) -> i32 {
    args.get(i).and_then(Arg::int).unwrap_or(0)
}

/// A text argument of a formatted read's leading ones.
fn leading_text(args: &[Arg], i: usize) -> Vec<u8> {
    let text = args.get(i).and_then(Arg::text).unwrap_or_default();
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    text[..end].to_vec()
}

/// The indices a `Mass` command sets: `max` down to `min` (none when `max < min`), at most
/// [`MASS_STEPS`].
pub fn mass_indices(min: i32, max: i32) -> impl Iterator<Item = i32> {
    let count = if max < min {
        0
    } else {
        (i64::from(max) - i64::from(min) + 1).min(i64::from(MASS_STEPS))
    };
    (0..count).map(move |k| (i64::from(max) - k) as i32)
}

// Translated from MCM.dll (MCM 1.5): GetMCMFloat.
/// `GetMCMFloat child grandchild setting`: the number at `GetMenuPath(child, grandchild,
/// setting)` ([`menu::path`]), 0 when it is not there.
fn get_mcm_float(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (kind, index) = (read.int(0, 0) as i32, read.int(1, 0) as i32);
    let Some(setting) = fits(read.text(2)) else {
        return handled(true);
    };
    let num = menu::get_float(e, &menu::path(kind, index, &setting));
    a.set_result(e, f64::from(num));
    if crate::from_console(e) {
        let [lo, hi] = double_words(f64::from(num));
        menu::with_text(e, &setting, |e, text| {
            console_print(
                e,
                "GetMCMFloat %i %i %s = %.4f",
                &[kind as u32, index as u32, text, lo, hi],
            );
        });
    }
    handled(true)
}

// Translated from MCM.dll (MCM 1.5): SetMCMFloat.
/// `SetMCMFloat child grandchild setting [value]`: the trait set (0 without a value).
fn set_mcm_float(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let (kind, index) = (read.int(0, 0) as i32, read.int(1, 0) as i32);
    let Some(setting) = fits(read.text(2)) else {
        return handled(true);
    };
    let value = read.float(3, 0.0);
    menu::set_float(e, &menu::path(kind, index, &setting), value);
    handled(true)
}

// Translated from MCM.dll (MCM 1.5): SetMCMString.
/// `SetMCMString child grandchild setting format ...`: the trait set to the formatted text.
fn set_mcm_string(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = format_string_args_at(e, &a, 3, 0x15);
    if !f.ok {
        return handled(true);
    }
    let (kind, index) = (leading_int(&f.leading, 0), leading_int(&f.leading, 1));
    let (Some(setting), Some(text)) = (fits(leading_text(&f.leading, 2)), fits(f.text)) else {
        return handled(true);
    };
    menu::set_string(e, &menu::path(kind, index, &setting), &text);
    handled(true)
}

// Translated from MCM.dll (MCM 1.5): SetMCMFloatMass.
/// `SetMCMFloatMass child min max setting [value]`: the trait set for every index from `max`
/// down to `min` ([`mass_indices`]).
fn set_mcm_float_mass(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let kind = read.int(0, 0) as i32;
    let (min, max) = (read.int(1, 0) as i32, read.int(2, 0) as i32);
    let Some(setting) = fits(read.text(3)) else {
        return handled(true);
    };
    let value = read.float(4, 0.0);
    for index in mass_indices(min, max) {
        menu::set_float(e, &menu::path(kind, index, &setting), value);
    }
    handled(true)
}

// Translated from MCM.dll (MCM 1.5): SetMCMStringMass.
/// `SetMCMStringMass child min max setting format ...`: the trait set to the formatted text for
/// every index from `max` down to `min`.
fn set_mcm_string_mass(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = format_string_args_at(e, &a, 4, 0x15);
    if !f.ok {
        return handled(true);
    }
    let kind = leading_int(&f.leading, 0);
    let (min, max) = (leading_int(&f.leading, 1), leading_int(&f.leading, 2));
    let (Some(setting), Some(text)) = (fits(leading_text(&f.leading, 3)), fits(f.text)) else {
        return handled(true);
    };
    for index in mass_indices(min, max) {
        menu::set_string(e, &menu::path(kind, index, &setting), &text);
    }
    handled(true)
}

/// An integer argument as the DLL uses it: its low 16 bits, signed (`MOVSX` of the word).
fn word(v: u32) -> i16 {
    v as u16 as i16
}

/// The DYNAMIC_CAST the DLL calls (`0x00EC43FB`: object, 0, from, to, 0) and the type
/// descriptors it names: `TESForm`, `TESObjectREFR`, `TESObjectCELL`, `TESFullName`.
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_FORM: u32 = 0x0118_3028;
const RTTI_REFR: u32 = 0x0118_41cc;
const RTTI_CELL: u32 = 0x0118_3fb4;
const RTTI_FULL_NAME: u32 = 0x0118_3158;

fn cast(e: &mut Engine, form: u32, to: u32) -> u32 {
    if form == 0 {
        return 0;
    }
    e.call(DYNAMIC_CAST, &args![form, 0u32, RTTI_FORM, to, 0u32])
        .u32()
}

// Translated from MCM.dll (MCM 1.5): its copy of NVSE 2's GetFullName.
/// The form's full name: a reference's (types 0x3A to 0x3C) base form's; for a cell (0x39) with
/// a worldspace and no name of its own, the worldspace's; `<no name>` without one or when empty.
pub fn full_name(e: &mut Engine, form: u32) -> Vec<u8> {
    let mut named = form;
    match e.mem.u8(form + 4) {
        0x3a..=0x3c => {
            let refr = cast(e, form, RTTI_REFR);
            // The DLL reads the base form without a check (a reference type always casts).
            named = if refr == 0 { 0 } else { e.mem.u32(refr + 0x20) };
        }
        0x39 => {
            let cell = cast(e, form, RTTI_CELL);
            if cell != 0 {
                let world = e.mem.u32(cell + 0xc0);
                if world != 0 && (e.mem.u32(cell + 0x1c) == 0 || e.mem.u16(cell + 0x20) == 0) {
                    named = world;
                }
            }
        }
        _ => {}
    }
    let name = cast(e, named, RTTI_FULL_NAME);
    if name != 0 && e.mem.u32(name + 4) != 0 && e.mem.u16(name + 8) != 0 {
        return table::guest_text(e, e.mem.u32(name + 4));
    }
    b"<no name>".to_vec()
}

// Translated from MCM.dll (MCM 1.5): SetMCMModList.
/// `SetMCMModList number enable [index index2 nameRef]` (each number its low 16 bits, signed):
/// `Mod<number>/_enable`; with a reference, `_modIndex`, `_modIndex2`, `text/string` (its full
/// name) and `MCM_ModTitle/Title<number>/string`.
fn set_mcm_mod_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let number = word(read.int(0, 0));
    let enable = word(read.int(1, 0));
    let (index, index2) = (word(read.int(2, 0)), word(read.int(3, 0)));
    let form = read.form(4);
    let base = format!("StartMenu/MCM/MCM_ModList/Mod{number}");
    let at = |tail: &str| format!("{base}{tail}").into_bytes();
    menu::set_float(e, &at("/_enable"), f32::from(enable));
    if form != 0 {
        let name = full_name(e, form);
        menu::set_float(e, &at("/_modIndex"), f32::from(index));
        menu::set_float(e, &at("/_modIndex2"), f32::from(index2));
        menu::set_string(e, &at("/text/string"), &name);
        let title = format!("StartMenu/MCM/MCM_ModTitle/Title{number}/string");
        menu::set_string(e, title.as_bytes(), &name);
    }
    handled(true)
}

// Translated from MCM.dll (MCM 1.5): GetMCMListWidth.
/// `GetMCMListWidth which number`: the width of `Mod<number>`'s text (`which` 1) or
/// `SubMenu<number>`'s (2), each number its low 16 bits; 0 otherwise.
fn get_mcm_list_width(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let which = read.int(0, 0) as u16;
    let number = word(read.int(1, 0));
    let item = match which {
        1 => "Mod",
        2 => "SubMenu",
        _ => return handled(true),
    };
    let path = format!("StartMenu/MCM/MCM_ModList/{item}{number}/text/width");
    let num = menu::get_float(e, path.as_bytes());
    a.set_result(e, f64::from(num));
    handled(true)
}

/// The DLL's INI folder (relative to the game's folder).
const CONFIG: &[u8] = b"Data\\Config\\";
/// Its name, section and key buffers (`char[100]`) and the value's (`GetPrivateProfileStringA`'s
/// size 32).
const NAME_BUFFER: usize = 100;
const VALUE_BUFFER: usize = 32;

/// The parts of `Mod/Section/Key` as the DLL cuts them (`strtok` on `/`, `:` and `\`: empty
/// parts skipped). `None` when a part is missing (with the DLL's line for a missing section or
/// key) or would not fit its buffer.
fn ini_parts(e: &mut Engine, text: &[u8], set: bool) -> Option<[Vec<u8>; 3]> {
    let mut parts = text
        .split(|&b| matches!(b, b'/' | b':' | b'\\'))
        .filter(|p| !p.is_empty());
    let (first, second, third) = (parts.next(), parts.next(), parts.next());
    let in_buffer = |p: Option<&[u8]>| p.filter(|p| p.len() < NAME_BUFFER).map(<[u8]>::to_vec);
    let file = in_buffer(first)?;
    let who = if set { "SetConfig" } else { "GetConfig" };
    let Some(section) = second else {
        if crate::from_console(e) {
            menu::with_text(e, who.as_bytes(), |e, at_who| {
                menu::with_text(e, &file, |e, at_file| {
                    console_print(e, "%s ERROR: Missing appName for %s", &[at_who, at_file]);
                });
            });
        }
        return None;
    };
    let section = in_buffer(Some(section))?;
    let Some(key) = third else {
        if crate::from_console(e) {
            menu::with_text(e, who.as_bytes(), |e, at_who| {
                menu::with_text(e, &file, |e, at_file| {
                    menu::with_text(e, &section, |e, at_section| {
                        console_print(
                            e,
                            "%s ERROR: Missing keyName for %s.ini [%s]",
                            &[at_who, at_file, at_section],
                        );
                    });
                });
            });
        }
        return None;
    };
    let key = in_buffer(Some(key))?;
    Some([file, section, key])
}

/// `Data\Config\<file>.ini`.
fn ini_file(file: &[u8]) -> Vec<u8> {
    [CONFIG, file, b".ini"].concat()
}

/// The DLL's `GetConfig`/`SetConfig` console line, when the line runs from the console.
fn config_line(e: &mut Engine, format: &str, parts: &[Vec<u8>; 3], num: f32) {
    if !crate::from_console(e) {
        return;
    }
    let [lo, hi] = double_words(f64::from(num));
    let [file, section, key] = parts;
    menu::with_text(e, file, |e, f| {
        menu::with_text(e, section, |e, s| {
            menu::with_text(e, key, |e, k| console_print(e, format, &[f, s, k, lo, hi]));
        });
    });
}

// Translated from MCM.dll (MCM 1.5): GetModINISetting.
/// `GetModINI "Mod/Section/Key"`: the number in `Data\Config\<Mod>.ini` (the mod's own copy
/// first: `engine_core::mod_files`), read as the DLL reads it: `GetPrivateProfileStringA` with
/// the default `"0"` into 32 bytes, then `atof` ([`atof`]), as a float.
fn get_mod_ini_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(text) = fits(read.text(0)) else {
        return handled(true);
    };
    let Some(parts) = ini_parts(e, &text, false) else {
        return handled(true);
    };
    let file = ini_file(&parts[0]);
    let mut value =
        mod_files::profile_string(e, &file, &parts[1], &parts[2]).unwrap_or_else(|| b"0".to_vec());
    value.truncate(VALUE_BUFFER - 1);
    let num = atof(&value) as f32;
    config_line(e, "GetConfig %s.ini -> [%s] %s=%.4f", &parts, num);
    a.set_result(e, f64::from(num));
    handled(true)
}

// Translated from MCM.dll (MCM 1.5): SetModINISetting.
/// `SetModINI "Mod/Section/Key" value`: `Data\Config\` made when it is not there; the file
/// made as one backspace byte when it is not there (`fopen("w")`, `fputs("\b")`); the value
/// written as `%.4f` ([`fixed4`]) with `WritePrivateProfileStringA`. Writes go to the mod files
/// (`engine_core::mod_files`): the run and the port's folder beside the saves, never the game's
/// folder.
fn set_mod_ini_setting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let Some(text) = fits(read.text(0)) else {
        return handled(true);
    };
    let value = read.float(1, 0.0);
    let Some(parts) = ini_parts(e, &text, true) else {
        return handled(true);
    };
    if !mod_files::directory_exists(e, CONFIG) {
        mod_files::create_directory(e, CONFIG);
    }
    let file = ini_file(&parts[0]);
    if !mod_files::exists(e, &file) {
        mod_files::write(e, &file, b"\x08");
    }
    let text = fixed4(f64::from(value));
    // The DLL does not look at WritePrivateProfileStringA's answer.
    let _ = mod_files::write_profile_string(e, &file, &parts[1], &parts[2], &text);
    config_line(e, "SetConfig %s.ini -> [%s] %s=%.4f", &parts, value);
    handled(true)
}

/// `atof` as the VC++ 2010 runtime (which the DLL links) reads a value (measured with its
/// `msvcr100.dll`, 2026-11-16, `measure/mcm/ini-measure/crt-2026-11-16.txt`): leading white
/// space, a sign, digits with one `.`, then an exponent `e`, `E`, `d` or `D` with a sign and
/// digits (without digits the letter ends the number); no `inf`, `nan` or hex; text after the
/// number ignored; nothing read is 0; too large is an infinity. The digits read are converted
/// correctly rounded.
pub fn atof(s: &[u8]) -> f64 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut number = String::new();
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        number.push(char::from(s[i]));
        i += 1;
    }
    let mut digits = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        number.push(char::from(s[i]));
        i += 1;
        digits += 1;
    }
    if i < s.len() && s[i] == b'.' {
        number.push('.');
        i += 1;
        while i < s.len() && s[i].is_ascii_digit() {
            number.push(char::from(s[i]));
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return 0.0;
    }
    if i < s.len() && matches!(s[i], b'e' | b'E' | b'd' | b'D') {
        let mut j = i + 1;
        let mut exponent = String::from("e");
        if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
            exponent.push(char::from(s[j]));
            j += 1;
        }
        let start = j;
        while j < s.len() && s[j].is_ascii_digit() {
            exponent.push(char::from(s[j]));
            j += 1;
        }
        if j > start {
            number.push_str(&exponent);
        }
    }
    if number.ends_with('.') {
        number.push('0');
    }
    if number.starts_with('.') || number.starts_with("-.") || number.starts_with("+.") {
        number = number.replacen('.', "0.", 1);
    }
    number.parse::<f64>().unwrap_or(0.0)
}

/// `sprintf("%.4f", v)` as the VC++ 2010 runtime writes it (measured with its `msvcr100.dll`,
/// 2026-11-16: 35,219 values, `measure/mcm/ini-measure/crt-f4-2026-11-16.txt`, all matched): the
/// value first rounded to 17 significant digits, then that decimal rounded to 4 places, half away
/// from zero; a minus sign for any negative value (`-0.0000`). Infinities `1.#INF`, `-1.#INF`; the
/// default NaN (`FFF8000000000000`) `-1.#IND`, any other NaN `1.#QNB` with its sign
/// (`crt-nan-2026-11-16.txt`).
pub fn fixed4(v: f64) -> Vec<u8> {
    let negative = v.is_sign_negative();
    let sign = if negative { "-" } else { "" };
    if v.is_infinite() {
        return format!("{sign}1.#INF").into_bytes();
    }
    if v.is_nan() {
        if v.to_bits() == 0xfff8_0000_0000_0000 {
            return b"-1.#IND".to_vec();
        }
        return format!("{sign}1.#QNB").into_bytes();
    }
    // 17 significant digits: d.dddddddddddddddde<exponent>.
    let sci = format!("{:.16e}", v.abs());
    let (mantissa, exponent) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
    // The digits up to the 4th place: (exponent + 1) before the point, 4 after.
    let keep = exponent + 1 + 4;
    let mut kept: Vec<u8> = Vec::new();
    let mut round_up = false;
    if keep >= digits.len() as i32 {
        kept.extend_from_slice(&digits);
        kept.resize(keep as usize, b'0');
    } else if keep >= 0 {
        kept.extend_from_slice(&digits[..keep as usize]);
        round_up = digits[keep as usize] >= b'5';
    }
    if round_up {
        let mut i = kept.len();
        loop {
            if i == 0 {
                kept.insert(0, b'1');
                break;
            }
            i -= 1;
            if kept[i] == b'9' {
                kept[i] = b'0';
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    // As a number of ten-thousandths: at least 5 digits ("0.0000").
    while kept.len() < 5 {
        kept.insert(0, b'0');
    }
    let point = kept.len() - 4;
    let mut out = sign.as_bytes().to_vec();
    out.extend_from_slice(&kept[..point]);
    out.push(b'.');
    out.extend_from_slice(&kept[point..]);
    out
}
