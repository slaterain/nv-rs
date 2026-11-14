//! Tests of the documentation dumps and the help listing, from xNVSE 6.4.9's source
//! (`CommandTable.cpp`, `Commands_Script.cpp`, `Hooks_Script.cpp`), on the command table's engine
//! with a built-in plugin's records added; `nvse.log` and the console are stand-ins that record
//! what they were given.

use super::*;
use crate::commands::test_support::{call, n, s};
use crate::plugins::BuiltIn;
use std::cell::RefCell;

const CONSOLE_INSTANCE: u32 = 0x0071_b160;
const CONSOLE_PRINT: u32 = 0x0071_d0a0;
const CREATE_FILE_A: u32 = 0x00fd_f088;
const SET_FILE_POINTER: u32 = 0x00fd_f240;
const WRITE_FILE: u32 = 0x00fd_f0dc;
const CLOSE_HANDLE: u32 = 0x00fd_f07c;
const TOUPPER: u32 = 0x00ec_a7f4;
const CONSOLE_OPEN: u32 = 0x011d_ea2e;
const AREA: u32 = 0x0610_0000;

thread_local! {
    static LOG: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static CONSOLE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static MADE: RefCell<u32> = const { RefCell::new(0) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// The table's engine, bounded to `max_opcode`, with `nvse.log` and the console recorded.
fn engine(max_opcode: u32) -> Engine {
    engine_with(max_opcode, |_| {})
}

/// [`engine`] with `before` run on the exe's tables before xNVSE's table is built from them.
fn engine_with(max_opcode: u32, before: impl FnOnce(&mut Engine)) -> Engine {
    LOG.with(|l| l.borrow_mut().clear());
    CONSOLE.with(|c| c.borrow_mut().clear());
    MADE.with(|m| *m.borrow_mut() = 0);
    let mut e = crate::commands::test_support::engine();
    e.map(0x0118_e000, 0x9000);
    e.map(0x0300_0000, 0x10000);
    before(&mut e);
    table::install(&mut e);
    e.map(AREA, 0x1000);
    e.ext::<crate::Nvse>().main_thread_recorded = true;
    let t = e.script_tables();
    e.set_script_tables(ScriptTables {
        max_opcode,
        end: t.start + (max_opcode - ScriptTables::FIRST_OPCODE) * table::RECORD,
        ..t
    });
    e.register(TOUPPER, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(CREATE_FILE_A, |e, a| {
        assert_eq!(table::guest_text(e, a[0]), b"nvse.log");
        if a[4] == 2 {
            LOG.with(|l| l.borrow_mut().clear());
            MADE.with(|m| *m.borrow_mut() += 1);
        }
        eax(0x44)
    });
    e.register(SET_FILE_POINTER, |_, _| eax(0));
    e.register(WRITE_FILE, |e, a| {
        let bytes = e.mem.bytes(a[1], a[2]);
        LOG.with(|l| l.borrow_mut().extend(bytes));
        eax(1)
    });
    e.register(CLOSE_HANDLE, |_, _| eax(1));
    e.register(CONSOLE_INSTANCE, |_, _| eax(AREA));
    e.register(CONSOLE_PRINT, |e, a| {
        let format = table::guest_text(e, a[1]);
        let text = if format == b"%s" {
            table::guest_text(e, e.mem.u32(a[2]))
        } else {
            format
        };
        CONSOLE.with(|c| {
            c.borrow_mut()
                .push(String::from_utf8_lossy(&text).into_owned())
        });
        Ret::default()
    });
    e
}

/// The log's lines (each ended by `\r\n`).
fn log_lines() -> Vec<String> {
    let text = LOG.with(|l| String::from_utf8_lossy(&l.borrow()).into_owned());
    assert!(text.is_empty() || text.ends_with("\r\n"), "{text:?}");
    let mut lines: Vec<String> = text.split("\r\n").map(str::to_string).collect();
    lines.pop();
    lines
}

fn console() -> Vec<String> {
    CONSOLE.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn console_mode(e: &mut Engine) {
    e.mem.set_u8(CONSOLE_OPEN, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
}

/// `DumpDocs 0 0x166B`: the head, then each named command from that opcode in detail, its
/// parameters `name:Type` (an optional one in italics), its return type, opcode in hex and
/// decimal, origin, condition and reference flags and the description (capital first letter,
/// a full stop); the console told only in console mode.
#[test]
fn dump_docs_in_detail() {
    let mut e = engine(0x166d);
    call(&mut e, 0x166a, 0, &[n(0), n(0x166b)]);
    assert_eq!(
        log_lines(),
        [
            "Commands starting from opcode 0x166b:",
            "<hr><br><b>Functions In Detail</b>",
            "<p><a name=\"DumpCommandWikiDoc\"></a><b>DumpCommandWikiDoc</b> ",
            "<br><b>Alias:</b> DumpWikiDoc<br><b>Parameters:</b>1",
            "<br>&nbsp;&nbsp;&nbsp;string:String ",
            "<br><b>Return Type:</b> Float<br><b>Opcode:</b> 0x166b (5739)<br><b>Origin:</b> NVSE<br><b>Condition Function:</b> No<br><b>Requires calling reference:</b> No<br><b>Description:</b> Dumps wiki - style documentation for a command.</p>",
            "<p><a name=\"DumpCommandWikiDocs\"></a><b>DumpCommandWikiDocs</b> ",
            "<br><b>Alias:</b> DumpWikiDocs<br><b>Parameters:</b>3",
            "<br>&nbsp;&nbsp;&nbsp;int:Integer ",
            "<br>&nbsp;&nbsp;&nbsp;int:Integer ",
            "<br>&nbsp;&nbsp;&nbsp;<i>string:String</i> ",
            "<br><b>Return Type:</b> Float<br><b>Opcode:</b> 0x166c (5740)<br><b>Origin:</b> NVSE<br><b>Condition Function:</b> No<br><b>Requires calling reference:</b> No<br><b>Description:</b> Dumps wiki - style documentation for multiple commands.</p>",
        ]
    );
    assert!(console().is_empty());
    console_mode(&mut e);
    call(&mut e, 0x166a, 0, &[n(1), n(0x166c)]);
    assert_eq!(
        console(),
        ["Dumping Command Docs", "Done Dumping Command Docs"]
    );
}

/// The quick list: `(Type) name` linked, each parameter its own line; no name, no line; with no
/// start, xNVSE's first opcode (`NVSE Commands ...`), the log made anew only once in a run.
#[test]
fn dump_docs_quick_list_and_default_start() {
    let mut e = engine(0x166d);
    call(&mut e, 0x166a, 0, &[n(1), n(0x166c)]);
    assert_eq!(
        log_lines(),
        [
            "Commands starting from opcode 0x166c:",
            "<br><b>Function Quick Reference</b>",
            "<br>(Float) <a href=\"#DumpCommandWikiDocs\">DumpCommandWikiDocs</a> ",
            "int:Integer ",
            "int:Integer ",
            "<i>string:String</i> ",
        ]
    );
    call(&mut e, 0x166a, 0, &[n(1)]);
    let lines = log_lines();
    assert_eq!(lines[6], "NVSE Commands starting from opcode 0x1400:");
    // CompileScript (an expression command: xNVSE's types; a form result).
    assert!(lines.contains(&"<br>(Form) <a href=\"#CompileScript\">CompileScript</a> ".to_string()));
    let at = lines
        .iter()
        .position(|l| l.contains("#CompileScript\""))
        .unwrap();
    assert_eq!(lines[at + 1], "string:String ");
    assert_eq!(lines[at + 2], "<i>bool:Bool</i> ");
    assert_eq!(MADE.with(|m| *m.borrow()), 1);
}

/// The start opcode 0 prints `0` (`%#x`), and a command with no execute is left out unless
/// condition-only ones are asked for.
#[test]
fn dump_docs_start_zero_and_condition_only() {
    let mut e = engine(0x166b);
    // DumpDocs (0x166A) has an execute; make it look condition-only.
    let record = e.script_tables().record(0x166a);
    e.mem.set_u32(record + 0x18, 0);
    dump_command_documentation(&mut e, true, 0x166a, false, false);
    assert_eq!(
        log_lines(),
        [
            "Commands starting from opcode 0x166a:",
            "<br><b>Function Quick Reference</b>"
        ]
    );
    dump_command_documentation(&mut e, true, 0, true, false);
    let lines = log_lines();
    assert_eq!(lines[2], "Commands starting from opcode 0:");
    assert!(lines.contains(&"<br>(Float) <a href=\"#DumpDocs\">DumpDocs</a> ".to_string()));
}

fn plugin(e: &mut Engine) {
    crate::plugins::register(
        e,
        BuiltIn {
            file: "jip_nvse.dll",
            name: "JIP LN NVSE",
            version: 57,
            opcode_base: 0x2200,
            listed: true,
            listener: None,
            save: None,
            load: None,
            pre_load: None,
            new_game: None,
        },
    );
    table::grow_to(e, 0x2203);
    let params = table::alloc(e, 12);
    let name = table::guest_str(e, b"");
    e.mem.set_u32(params, name);
    e.mem.set_u32(params + 4, 0x3d);
    e.mem.set_u32(params + 8, 1);
    table::add_plugin_record(
        e,
        0x2201,
        &table::PluginRecord {
            name: "IsFooOLD",
            short: "",
            needs_ref: true,
            param_count: 1,
            params,
            execute: 0x7e00_1000,
            eval: 0x7e00_2000,
            flags: 1,
            expression: false,
            returns: Returns::Array,
        },
    );
}

/// A wiki page: the plugin's origin (`JIP`), a deprecated command's marks, `1/0` for an `Is`
/// name, the reference type, both kinds of function, an unnamed argument, its categories.
#[test]
fn a_wiki_page() {
    let mut e = engine(0x166d);
    plugin(&mut e);
    LOG.with(|l| l.borrow_mut().clear());
    call(&mut e, 0x166b, 0, &[s("isfooold")]);
    assert_eq!(
        log_lines(),
        [
            "{{Deprecated}}",
            "{{Function",
            " |origin = JIP",
            " |originVersion = [TO SPECIFY]",
            " |summary = Deprecated.",
            " |name = IsFooOLD",
            " |returnVal = [Insert return val name here, or remove this arg]",
            " |returnType = 1/0 [remove this arg if func returns nothing]",
            " |referenceType = reference",
            " |conditionFunc = Both",
            " |arguments =",
            "  {{FunctionArgument",
            "   |Name = [Enter Arg Name]",
            "   |Type = AnyForm",
            "   |Optional = y",
            "   |Value = [INSERT DEFAULT VALUE]",
            "  }}",
            "}}",
            "[[Category:Functions (JIP)]]",
            "[[Category:? [Add other relevant categories] ]]",
            "[[Category:Deprecated Functions]]",
            "[[Category:Array Functions]]",
            "",
        ]
    );
}

/// An expression command's page: xNVSE's argument types, its alias page, the NVSE-aware and
/// string categories, the version given; the range form with a heading-less list; the console's
/// lines; a name that is no command.
#[test]
fn wiki_pages_by_name_and_range() {
    let mut e = engine(0x166d);
    console_mode(&mut e);
    call(&mut e, 0x166c, 0, &[n(0x1668), n(0x1668), s("6.4.9")]);
    assert_eq!(
        log_lines(),
        [
            "{{Function",
            " |origin = NVSE",
            " |originVersion = 6.4.9",
            " |summary = \"Returns a compiled script from a file, to call as a UDF.\".",
            " |name = CompileScript",
            " |alias = GetUDFFromFile",
            " |returnVal = [Insert return val name here, or remove this arg]",
            " |returnType = Form [remove this arg if func returns nothing]",
            " |conditionFunc = Script",
            " |arguments =",
            "  {{FunctionArgument",
            "   |Name = [Enter Arg Name]",
            "   |Type = String",
            "  }}{{FunctionArgument",
            "   |Name = [Enter Arg Name]",
            "   |Type = Bool",
            "   |Optional = y",
            "   |Value = [INSERT DEFAULT VALUE]",
            "  }}",
            "}}",
            "[[Category:Functions (NVSE)]]",
            "[[Category:? [Add other relevant categories] ]]",
            "[[Category:NVSE-Aware Functions]]",
            "[[Category:String Functions]]",
            "",
            "!!! Create a page with the name \"GetUDFFromFile\" and the following text:",
            "#redirect [[CompileScript]]",
            "[[Category:Function Alias]]",
            "",
        ]
    );
    assert_eq!(
        console(),
        [
            "Dumping wiki-style documentation for functions in opcode range.",
            "Finished dumping wiki-style documentation for functions."
        ]
    );
    call(&mut e, 0x166c, 0, &[n(0x166c), n(0x166b)]);
    call(&mut e, 0x166c, 0, &[n(0x166c), n(0x166d)]);
    call(&mut e, 0x166b, 0, &[s("NoSuchCommand")]);
    assert_eq!(
        console(),
        [
            "Unable to find commands.",
            "Unable to find commands.",
            "Unable to find command with that name. Double-check for a spelling error, or if it's being registered."
        ]
    );
    call(&mut e, 0x166b, 0, &[s("DumpWikiDoc")]);
    assert_eq!(
        console(),
        [
            "Dumping wiki-style documentation for function.",
            "Finished dumping wiki-style documentation for function."
        ]
    );
}

/// `0 0`: the heading, every record's page (none for an unnamed one) and two empty lines after
/// each.
#[test]
fn every_wiki_page() {
    let mut e = engine(0x166d);
    let r = e.script_tables().record(0x1001);
    let name = table::guest_str(&mut e, b"GetV");
    e.mem.set_u32(r, name);
    e.mem.set_u32(r + 0x18, 0x7e00_0000);
    call(&mut e, 0x166c, 0, &[n(0), n(0)]);
    let lines = log_lines();
    assert_eq!(&lines[..4], ["## Command List", "", "", ""]);
    let page = lines.iter().position(|l| l == " |name = GetV").unwrap();
    assert_eq!(
        lines[page - 3],
        " |origin = Base game (cannot guess from which version it was introduced)"
    );
    assert_eq!(&lines[lines.len() - 3..], ["", "", ""]);
    // GetV and every record of xNVSE's range up to 0x166C (all named).
    assert_eq!(
        lines.iter().filter(|l| *l == "{{Function").count(),
        1 + (0x166d - 0x1400)
    );
}

/// `GetByOpcodeRange`'s limits.
#[test]
fn opcode_ranges() {
    let e = engine(0x1010);
    assert_eq!(opcode_range(&e, 0x1002, 0x1004), [0x1002, 0x1003, 0x1004]);
    assert_eq!(opcode_range(&e, 0x100f, 0x100f), [0x100f]);
    assert!(opcode_range(&e, 0x1004, 0x1002).is_empty());
    assert!(opcode_range(&e, 0x1000, 0x1010).is_empty(), "past the end");
    assert!(
        opcode_range(&e, 0x0fff, 0x1002).is_empty(),
        "below the table"
    );
    assert_eq!(opcode_range(&e, 0x1000, 0x100f).len(), 16);
}

fn info_named(long: &str, help: &str) -> Info {
    Info {
        opcode: 0x2201,
        long: long.as_bytes().to_vec(),
        short: Vec::new(),
        help: help.as_bytes().to_vec(),
        needs_ref: false,
        params: Vec::new(),
        execute: true,
        eval: false,
        expression: false,
        returns: Returns::Default,
        plugin: None,
    }
}

/// The description, the deprecated names, the `Is` names' return text.
#[test]
fn descriptions_and_return_texts() {
    let c = info_named("GetThing", "gets a thing");
    assert_eq!(c.description(false), b"Gets a thing.");
    assert_eq!(info_named("A", "Done.").description(false), b"Done.");
    assert_eq!(info_named("A", "").description(false), b"none");
    assert_eq!(info_named("A", "").description(true), b"[TO DOCUMENT]");
    for name in ["EmptyCommand", "GetXOLD", "GetXBROKEN"] {
        assert!(info_named(name, "x").deprecated(), "{name}");
        assert_eq!(info_named(name, "x").description(false), b"|DEPRECATED|");
    }
    for name in ["emptycommand", "GetXold", "OLDGetX"] {
        assert!(!info_named(name, "x").deprecated(), "{name}");
    }
    for (name, text) in [
        ("IsX", "1/0"),
        ("isx", "1/0"),
        ("GETISX", "1/0"),
        ("I", "Float"),
        ("GetI", "Float"),
        ("XIs", "Float"),
    ] {
        assert_eq!(info_named(name, "").return_text(), text, "{name}");
    }
    let mut c = info_named("Get", "");
    for (r, text) in [
        (Returns::Ambiguous, "Ambiguous"),
        (Returns::Form, "Form"),
        (Returns::String, "String"),
        (Returns::Array, "Array"),
        (Returns::ArrayIndex, "Array index"),
    ] {
        c.returns = r;
        assert_eq!(c.return_text(), text);
    }
}

/// `GetWikiStyleOriginName` for plugins' names.
#[test]
fn wiki_origin_names() {
    let cases: [(&'static str, &str, &str); 7] = [
        ("JIP LN NVSE", "JIP", "JIP"),
        ("SUP NVSE Plugin", "SUP", "SUP NVSE Plugin"),
        ("kNVSE", "kNVSE", "kNVSE"),
        ("ShowOff NVSE Plugin", "ShowOff", "ShowOff NVSE"),
        ("JohnnyGuitarNVSE", "JohnnyGuitar", "JohnnyGuitar NVSE"),
        ("ppNVSE", "pp", "ppNVSE"),
        (
            "lStewieAl's Tweaks",
            "lStewieAl's Tweaks",
            "lStewieAl's Tweaks",
        ),
    ];
    for (plugin, origin, category) in cases {
        let mut c = info_named("X", "");
        c.plugin = Some((plugin, 1));
        assert_eq!(c.wiki_origin(true), origin.as_bytes(), "{plugin}");
        assert_eq!(c.wiki_origin(false), category.as_bytes(), "{plugin}");
    }
    let mut c = info_named("X", "");
    c.opcode = 0x1500;
    assert_eq!(c.wiki_origin(true), b"NVSE");
    c.opcode = 0x13ff;
    assert_eq!(c.origin(), b"Base game");
    c.opcode = 0x1400;
    assert_eq!(c.origin(), b"NVSE");
}

/// The parameter types by the game's and xNVSE's names.
#[test]
fn parameter_type_names() {
    assert_eq!(param_type_name(0x15), "Object");
    assert_eq!(param_type_name(0x29), "MiscStat");
    assert_eq!(param_type_name(0x32), "InvObjectOrFormList");
    assert_eq!(param_type_name(0x45), "Region");
    assert_eq!(param_type_name(0x100), "ArrayVar");
    assert_eq!(param_type_name(0x46), "<unknown>");
    assert_eq!(nvse_param_type_name(0x8001), "Number");
    assert_eq!(nvse_param_type_name(0x8005), "StringOrNumber");
    assert_eq!(nvse_param_type_name(0x804d), "AnyType");
    assert_eq!(nvse_param_type_name(0), "AnyType");
    assert_eq!(nvse_param_type_name(0xc080), "ArrayVarOrArrayElement");
    assert_eq!(nvse_param_type_name(0x8044), "Collection");
    assert_eq!(nvse_param_type_name(0x8009), "FormOrNumber");
    assert_eq!(nvse_param_type_name(1 << 21), "Pair");
    assert_eq!(nvse_param_type_name(0x8000), "<unknown>");
    let mut c = info_named("X", "");
    assert_eq!(c.param_text(b"count", 1), b"count:Integer");
    assert_eq!(c.param_text(b"", 1), b"Integer");
    c.expression = true;
    assert_eq!(c.param_text(b"n", 0x8001), b"n:Number");
}

/// The help listing: every record the filter matches (help, long or short name, or its
/// plugin's name, without case), with xNVSE's version for its own range, the plugin's version
/// for a plugin's, nothing for the game's; xNVSE's padding command where nv-rs leaves a plugin
/// slot empty; the game's loop passed over.
#[test]
fn the_help_listing() {
    let mut e = engine(nvse_end());
    plugin(&mut e);
    install(&mut e);
    let filter = table::guest_str(&mut e, b"WIKI");
    assert_eq!(e.site(0x005b_cbc2, &[filter]), Some(1));
    assert_eq!(
        console(),
        [
            "[NVSE 6.4.9] DumpCommandWikiDoc (DumpWikiDoc) -> dumps wiki - style documentation for a command",
            "[NVSE 6.4.9] DumpCommandWikiDocs (DumpWikiDocs) -> dumps wiki - style documentation for multiple commands",
        ]
    );
    let filter = table::guest_str(&mut e, b"jip ln");
    e.site(0x005b_cbc2, &[filter]);
    assert_eq!(console(), ["[JIP LN NVSE 57] IsFooOLD"]);
    let filter = table::guest_str(&mut e, b"padding");
    e.site(0x005b_cbc2, &[filter]);
    let lines = console();
    // xNVSE's padding to 0x1400 (the game's range: no plugin), then the empty plugin slots
    // from xNVSE's end: below 0x2000 xNVSE's, from 0x2000 none (no plugin registered them).
    assert_eq!(lines[0], " -> command used for padding");
    assert!(lines.contains(&"[NVSE 6.4.9]  -> command used for padding".to_string()));
    assert_eq!(lines.last().unwrap(), " -> command used for padding");
    // And ar_Resize, whose help names its padding.
    assert!(lines.iter().any(|l| l.contains("ar_Resize")));
    assert_eq!(
        lines.len() as u32,
        (0x1400 - 0x1280) + (0x2200 - (crate::names::FIRST_OPCODE + NAMES.len() as u32)) + 1 + 1
    );
    // No filter: every record, the game's unnamed ones as empty lines.
    let filter = table::guest_str(&mut e, b"");
    e.site(0x005b_cbc2, &[filter]);
    // xNVSE's table ends after the plugin's last registered record (0x2201).
    assert_eq!(console().len() as u32, 0x2202 - 0x1000);
}

/// A help line's four shapes.
#[test]
fn help_line_shapes() {
    let mut c = info_named("Get", "");
    c.opcode = 0x1001;
    assert_eq!(help_line(&c, b""), Some(b"Get".to_vec()));
    c.short = b"G".to_vec();
    assert_eq!(help_line(&c, b""), Some(b"Get (G)".to_vec()));
    c.help = b"gets".to_vec();
    assert_eq!(help_line(&c, b""), Some(b"Get (G) -> gets".to_vec()));
    c.short.clear();
    assert_eq!(help_line(&c, b"ET"), Some(b"Get -> gets".to_vec()));
    assert_eq!(help_line(&c, b"getx"), None);
    assert_eq!(help_line(&c, b"NVSE"), None, "no plugin below 0x1400");
    c.opcode = 0x1400;
    assert!(help_line(&c, b"nvse").is_some(), "xNVSE's name");
    assert!(contains_ci(b"abc", b"BC"));
    assert!(!contains_ci(b"", b"a"));
    assert!(!contains_ci(b"abc", b""));
    assert!(!contains_ci(b"ab", b"abc"));
}

/// The shipped xNVSE 6.4.9 DLL's sections: (virtual address, raw offset, raw size), the image
/// base.
fn dll_sections(dll: &[u8]) -> (Vec<(u32, u32, u32)>, u32) {
    let u16_at = |o: usize| u16::from_le_bytes([dll[o], dll[o + 1]]);
    let u32_at = |o: usize| u32::from_le_bytes(dll[o..o + 4].try_into().unwrap());
    let pe = u32_at(0x3c) as usize;
    let count = usize::from(u16_at(pe + 6));
    let optional = usize::from(u16_at(pe + 20));
    let base = u32_at(pe + 24 + 28);
    let first = pe + 24 + optional;
    let sections = (0..count)
        .map(|i| {
            let s = first + 40 * i;
            (u32_at(s + 12), u32_at(s + 20), u32_at(s + 16))
        })
        .collect();
    (sections, base)
}

/// Every built xNVSE command's help text, reference flag and parameter count against its
/// `CommandInfo` in the shipped DLL (`NV_XNVSE_DLL`: the path of xNVSE 6.4.9's `nvse_1_4.dll`;
/// read at run time, never kept). Opt in: `--ignored`.
#[test]
#[ignore = "needs NV_XNVSE_DLL, the shipped nvse_1_4.dll"]
fn help_texts_match_the_shipped_dll() {
    let path = std::env::var_os("NV_XNVSE_DLL").expect("NV_XNVSE_DLL");
    let dll = std::fs::read(path).expect("the DLL");
    let (sections, base) = dll_sections(&dll);
    let to_va = |off: usize| {
        sections
            .iter()
            .find(|&&(_, raw, size)| (raw as usize..(raw + size) as usize).contains(&off))
            .map(|&(va, raw, _)| base + va + (off as u32 - raw))
    };
    let to_off = |va: u32| {
        let rva = va.wrapping_sub(base);
        sections
            .iter()
            .find(|&&(v, _, size)| (v..v + size).contains(&rva))
            .map(|&(v, raw, _)| (raw + rva - v) as usize)
    };
    let text_at = |va: u32| -> Option<Vec<u8>> {
        let off = to_off(va)?;
        let end = dll[off..].iter().position(|&b| b == 0)?;
        Some(dll[off..off + end].to_vec())
    };
    let u32_at = |o: usize| u32::from_le_bytes(dll[o..o + 4].try_into().unwrap());
    let mut words: std::collections::HashMap<u32, Vec<usize>> = std::collections::HashMap::new();
    for o in (0..dll.len().saturating_sub(0x28)).step_by(4) {
        words.entry(u32_at(o)).or_default().push(o);
    }
    let mut strings: std::collections::HashMap<&[u8], Vec<u32>> = std::collections::HashMap::new();
    let mut start = 0;
    for (i, &b) in dll.iter().enumerate() {
        if b == 0 {
            if i > start {
                if let Some(va) = to_va(start) {
                    strings.entry(&dll[start..i]).or_default().push(va);
                }
            }
            start = i + 1;
        }
    }
    let mut wrong = Vec::new();
    let mut made_at_load = Vec::new();
    let mut found = 0;
    let mut dump = String::new();
    for c in crate::commands::all() {
        let names = strings.get(c.name.as_bytes()).cloned().unwrap_or_default();
        let record = names
            .iter()
            .flat_map(|va| words.get(va).cloned().unwrap_or_default())
            .find(|&o| {
                text_at(u32_at(o + 4)).is_some_and(|s| s == c.short.as_bytes())
                    && text_at(u32_at(o + 12)).is_some()
            });
        let Some(o) = record else {
            // A record no word of the file points to by its name is one the DLL fills in when it
            // loads (a dynamic initializer): nothing to compare in the file.
            if names.iter().any(|va| words.contains_key(va)) {
                wrong.push(format!("{}: no record found", c.name));
            } else {
                made_at_load.push(c.name);
            }
            continue;
        };
        found += 1;
        let help = text_at(u32_at(o + 12)).unwrap();
        let needs_ref = u16::from_le_bytes([dll[o + 0x10], dll[o + 0x11]]) != 0;
        let params = u16::from_le_bytes([dll[o + 0x12], dll[o + 0x13]]);
        let help_text = String::from_utf8(help.clone()).expect("ASCII help");
        assert!(!help_text.contains(['\t', '\n', '\r']), "{}", c.name);
        dump.push_str(&format!(
            "{:04x}\t{}\t{}\t{}\t{}\n",
            c.opcode,
            c.name,
            u8::from(needs_ref),
            params,
            help_text
        ));
        if help != c.help.as_bytes() {
            wrong.push(format!(
                "{}: help {:?}, the DLL's {:?}",
                c.name,
                c.help,
                String::from_utf8_lossy(&help)
            ));
        }
        if needs_ref != c.needs_ref || usize::from(params) != c.params.len() {
            wrong.push(format!(
                "{}: needs_ref {} params {}, the DLL's {needs_ref} {params}",
                c.name,
                c.needs_ref,
                c.params.len()
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{found} found; {} differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    // Every command but the two whose record the DLL makes when it loads (V3Length,
    // PrintToConsole on 2026-11-14).
    assert!(made_at_load.len() <= 2, "{made_at_load:?}");
    // `NV_XNVSE_DLL_DUMP`: the facts written as the checked-in table
    // (`xnvse_dll_records.tsv`, read by [`the_records_match_the_dlls_table`]).
    if let Some(out) = std::env::var_os("NV_XNVSE_DLL_DUMP") {
        let mut lines: Vec<&str> = dump.lines().collect();
        lines.sort_unstable();
        std::fs::write(out, lines.join("\n") + "\n").expect("the table");
    }
    assert_eq!(found + made_at_load.len(), crate::commands::all().len());
}

/// A plugin record belongs to the built-in that answers "installed" with the highest opcode
/// base at or below it: one that does not answer "installed" owns nothing (xNVSE loads no such
/// plugin), and xNVSE's range up to 0x2000 is xNVSE's.
#[test]
fn records_belong_to_the_installed_plugins() {
    let mut e = engine(nvse_end());
    plugin(&mut e);
    crate::plugins::register(
        &mut e,
        BuiltIn {
            file: "zz_silent.dll",
            name: "Silent",
            version: 3,
            opcode_base: 0x2201,
            listed: false,
            listener: None,
            save: None,
            load: None,
            pre_load: None,
            new_game: None,
        },
    );
    assert_eq!(info(&e, 0x2201).plugin, Some(("JIP LN NVSE", 57)));
    assert_eq!(info(&e, 0x2201).origin(), b"JIP LN NVSE");
    install(&mut e);
    let filter = table::guest_str(&mut e, b"padding");
    e.site(0x005b_cbc2, &[filter]);
    let nvse = console()
        .iter()
        .filter(|l| l.starts_with("[NVSE 6.4.9]  -> command used for padding"))
        .count() as u32;
    assert_eq!(nvse, PLUGIN_OPCODES - nvse_end());
}

/// xNVSE's help texts as its DLL holds them: a description given as a string is stringified
/// with its quotes (`#description`), so the text keeps them; `Jmp`'s is the word `nullptr`.
#[test]
fn help_texts_keep_xnvses_quotes() {
    let help = |name: &str| {
        crate::commands::all()
            .into_iter()
            .find(|c| c.name == name)
            .unwrap()
            .help
    };
    assert_eq!(help("ar_Filter"), "\"filters an array on a condition\"");
    assert_eq!(help("IsInventoryRef"), "\"\"");
    assert_eq!(help("Jmp"), "nullptr");
    assert_eq!(help("GetSelfAlt"), help("GetSelfAlt_OLD"));
    assert!(help("GetSelfAlt").starts_with("\"Unlike GetSelf"));
    assert_eq!(
        help("DumpCommandWikiDoc"),
        "dumps wiki - style documentation for a command"
    );
    let params = |name: &str| {
        crate::commands::all()
            .into_iter()
            .find(|c| c.name == name)
            .unwrap()
            .params
            .len()
    };
    assert_eq!(params("MatchesAnyOf"), 20);
}

/// An argument whose type names an array puts a command that returns a number among the array
/// functions (`ar_Size`: an `Array` argument, a number result).
#[test]
fn an_array_argument_makes_an_array_function() {
    let mut e = engine(0x166d);
    call(&mut e, 0x166c, 0, &[n(0x1550), n(0x1550)]);
    let lines = log_lines();
    assert!(lines.contains(&"   |Type = Array".to_string()), "{lines:?}");
    assert!(lines
        .contains(&" |returnType = Float [remove this arg if func returns nothing]".to_string()));
    assert!(lines.contains(&"[[Category:Array Functions]]".to_string()));
}

/// The shipped state: JIP LN grows the table to its last opcode but does not answer
/// "installed", so xNVSE's own table (no plugin loaded) ends after xNVSE's last command: help
/// lists nothing past it, the padding it lists is the game range's, the whole-table wiki dump
/// stops there and a range past it is not found.
#[test]
fn an_unlisted_plugin_adds_nothing_to_the_listings() {
    let mut e = engine(nvse_end());
    crate::plugins::register(
        &mut e,
        BuiltIn {
            file: "jip_nvse.dll",
            name: "JIP LN NVSE",
            version: 57,
            opcode_base: 0x2200,
            listed: false,
            listener: None,
            save: None,
            load: None,
            pre_load: None,
            new_game: None,
        },
    );
    table::grow_to(&mut e, 0x2943);
    assert_eq!(table_end(&e), nvse_end());
    install(&mut e);
    let filter = table::guest_str(&mut e, b"");
    e.site(0x005b_cbc2, &[filter]);
    let lines = console();
    assert_eq!(lines.len() as u32, nvse_end() - 0x1000);
    let filter = table::guest_str(&mut e, b"padding");
    e.site(0x005b_cbc2, &[filter]);
    let lines = console();
    let padding = lines
        .iter()
        .filter(|l| l.ends_with("command used for padding"))
        .count();
    assert_eq!(padding, 0x1400 - 0x1280);
    console_mode(&mut e);
    call(&mut e, 0x166c, 0, &[n(0x1400), n(0x2000)]);
    assert_eq!(console(), ["Unable to find commands."]);
    let last = nvse_end() - 1;
    call(&mut e, 0x166c, 0, &[n(last as i32), n(last as i32)]);
    assert_eq!(
        console()[0],
        "Dumping wiki-style documentation for functions in opcode range."
    );
    LOG.with(|l| l.borrow_mut().clear());
    call(&mut e, 0x166c, 0, &[n(0), n(0)]);
    // A page for each of xNVSE's named records (the stand-in game records have no names).
    let pages = log_lines().iter().filter(|l| *l == "{{Function").count();
    assert_eq!(pages as u32, nvse_end() - 0x1400);
}

/// `ShowRouletteMenu`'s short name: cleared in its record as xNVSE clears it, so `help` and the
/// dumps show none, while the name map xNVSE built before still finds the command by it.
#[test]
fn show_roulette_menu_loses_its_short_name() {
    const NAMES: u32 = 0x0200_8000;
    let mut e = engine_with(0x166d, |e| {
        e.mem.write(NAMES, b"ShowRouletteMenu\0SRMenu\0");
        let record = ScriptTables::EXE.start + (table::ROULETTE - 0x1000) * table::RECORD;
        e.mem.set_u32(record, NAMES);
        e.mem.set_u32(record + 4, NAMES + 17);
        e.mem.set_u32(record + 8, table::ROULETTE);
        e.mem.set_u32(record + 0x18, 0x7e00_0100);
    });
    let record = e.script_tables().record(table::ROULETTE);
    assert_eq!(e.mem.u32(record + 4), 0);
    assert_eq!(table::opcode_by_name(&e, b"srmenu"), Some(table::ROULETTE));
    assert_eq!(
        table::name_map(&e).get(&crate::events::hash_name(b"SRMenu")),
        Some(&Some(table::ROULETTE))
    );
    let c = info(&e, table::ROULETTE);
    assert!(c.short.is_empty());
    assert_eq!(help_line(&c, b""), Some(b"ShowRouletteMenu".to_vec()));
    call(
        &mut e,
        0x166c,
        0,
        &[n(table::ROULETTE as i32), n(table::ROULETTE as i32)],
    );
    let lines = log_lines();
    assert!(lines.contains(&" |name = ShowRouletteMenu".to_string()));
    assert!(
        !lines
            .iter()
            .any(|l| l.contains("alias") || l.contains("redirect")),
        "{lines:?}"
    );
    LOG.with(|l| l.borrow_mut().clear());
    dump_command_documentation(&mut e, false, 0, false, false);
    let lines = log_lines();
    let at = lines
        .iter()
        .position(|l| l.contains("name=\"ShowRouletteMenu\""))
        .unwrap();
    assert_eq!(
        lines[at + 1],
        "<br><b>Alias:</b> none<br><b>Parameters:</b>0"
    );
}

/// Every built xNVSE command's help text, reference flag and parameter count as the shipped
/// 6.4.9 DLL's `CommandInfo` holds them (`xnvse_dll_records.tsv`: opcode, name, needs a
/// reference, parameter count, help; text read from the DLL by
/// [`help_texts_match_the_shipped_dll`] with `NV_XNVSE_DLL_DUMP`; no code). The two commands
/// whose record the DLL fills in when it loads are not in it.
#[test]
fn the_records_match_the_dlls_table() {
    let table = include_str!("xnvse_dll_records.tsv");
    let mut seen = 0;
    for line in table.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let [opcode, name, needs_ref, params, help] = f[..] else {
            panic!("{line:?}");
        };
        let opcode = u32::from_str_radix(opcode, 16).unwrap();
        let c = crate::commands::all()
            .into_iter()
            .find(|c| c.opcode == opcode)
            .unwrap_or_else(|| panic!("{name}: not built"));
        assert_eq!(c.name, name);
        assert_eq!(c.help, help, "{name}");
        assert_eq!(u8::from(c.needs_ref).to_string(), needs_ref, "{name}");
        assert_eq!(c.params.len().to_string(), params, "{name}");
        seen += 1;
    }
    let mut missing: Vec<&str> = crate::commands::all()
        .into_iter()
        .filter(|c| !table.lines().any(|l| l.split('\t').nth(1) == Some(c.name)))
        .map(|c| c.name)
        .collect();
    missing.sort_unstable();
    assert_eq!(missing, ["PrintToConsole", "V3Length"]);
    assert_eq!(seen + missing.len(), crate::commands::all().len());
}

/// xNVSE's table ends past the last record an installed plugin registered: a named one, or one
/// of its empty commands (no name, an execute); without such a plugin, past xNVSE's commands.
#[test]
fn the_table_ends_past_the_last_registered_record() {
    let mut e = engine(nvse_end());
    assert_eq!(table_end(&e), nvse_end());
    plugin(&mut e);
    assert_eq!(table_end(&e), 0x2202);
    let empty = e.script_tables().record(0x2202);
    e.mem.set_u32(empty + 0x18, 0x7e00_3000);
    assert_eq!(table_end(&e), 0x2203);
}

/// A named record at a plugin opcode that no installed plugin registered (its built-in does not
/// answer "installed") does not lengthen xNVSE's table.
#[test]
fn an_unlisted_plugins_record_does_not_lengthen_the_table() {
    let mut e = engine(nvse_end());
    crate::plugins::register(
        &mut e,
        BuiltIn {
            file: "silent.dll",
            name: "Silent",
            version: 1,
            opcode_base: 0x2200,
            listed: false,
            listener: None,
            save: None,
            load: None,
            pre_load: None,
            new_game: None,
        },
    );
    table::grow_to(&mut e, 0x2202);
    let record = e.script_tables().record(0x2201);
    let name = table::guest_str(&mut e, b"Named");
    e.mem.set_u32(record, name);
    assert_eq!(table_end(&e), nvse_end());
}
