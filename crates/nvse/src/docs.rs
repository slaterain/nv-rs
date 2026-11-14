//! xNVSE's command documentation (plan step 3.8, part 2): the dumps `DumpDocs`,
//! `DumpCommandWikiDoc` and `DumpCommandWikiDocs`, which write the script command table to
//! `nvse.log` ([`crate::log`]) as HTML or as GECK wiki pages, and the console's `help` listing of
//! script functions, which xNVSE replaces with its own (`PatchHelpCommand`, at `005BCBC2` in
//! `HelpFunction` `005bc950`).
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a): `CommandTable.cpp`
//! (`Cmd_DumpDocs_Execute`, `DumpCommandDocumentation`, `CommandInfo::DumpDocs`,
//! `DumpFunctionDef`, `DumpWikiDocs`, `GetDescription`, `IsDeprecated`, `GetOriginName`,
//! `GetWikiStyleOriginName`, `GetReturnTypeStr`, `StringForParamType`, `ParamInfo::GetAsString`,
//! `GetByOpcodeRange`, `GetParentPlugin`), `ScriptUtils.cpp` (`StringForNVSEParamType`),
//! `Commands_Script.cpp` (`Cmd_DumpCommandWikiDoc(s)_Execute`) and `Hooks_Script.cpp`
//! (`PatchHelpCommand::HandlePrintCommands`).
//!
//! The table is the whole script table, from opcode `0x1000` to the end: the game's commands,
//! xNVSE's padding to `0x1400`, xNVSE's commands and the plugins built in. Where nv-rs leaves a
//! plugin opcode empty (a slot no built-in fills, `table::grow_to`), xNVSE's table holds its
//! padding command (no name, help "command used for padding"), and these listings show it so.
//! A command's plugin is the built-in that answers "installed" whose opcode base is the highest at
//! or below the command's opcode (xNVSE records the base the plugin registered it under).

use crate::commands::{console_print, is_console_mode, Read};
use crate::names::{FIRST_OPCODE as NVSE_FIRST, NAMES, PLUGIN_OPCODES};
use crate::table::{self, Command, Param, Record, Returns, EXPRESSION_PARSE};
use crate::{handled, CommandArgs};
use engine_core::extend::{Owner, ScriptTables, SiteSlot};
use engine_core::prelude::*;

/// `HelpFunction`'s loop over the script functions, which xNVSE's listing replaces; the filter
/// text is its word.
pub const HELP_LISTING: SiteSlot = SiteSlot::new(Owner::Nvse, 0x005b_cbc2);

/// xNVSE's padding command's help text.
const PADDING_HELP: &[u8] = b"command used for padding";

/// One record of the table as these listings read it.
struct Info {
    opcode: u32,
    long: Vec<u8>,
    short: Vec<u8>,
    help: Vec<u8>,
    needs_ref: bool,
    /// Each parameter's name (`typeStr`), type and whether it is optional.
    params: Vec<(Vec<u8>, u32, bool)>,
    execute: bool,
    eval: bool,
    expression: bool,
    returns: Returns,
    /// The plugin the command was registered under (`metadata->parentPlugin`): its name and
    /// version.
    plugin: Option<(&'static str, u32)>,
}

/// The opcode past xNVSE's own commands.
fn nvse_end() -> u32 {
    NVSE_FIRST + NAMES.len() as u32
}

/// The listed built-in a plugin opcode was registered under.
fn plugin_of(e: &Engine, opcode: u32) -> Option<(&'static str, u32)> {
    crate::plugins::built_ins(e)
        .into_iter()
        .filter(|p| p.listed && p.opcode_base <= opcode)
        .max_by_key(|p| p.opcode_base)
        .map(|p| (p.name, p.version))
}

fn info(e: &Engine, opcode: u32) -> Info {
    let r = Record::read(e, e.script_tables().record(opcode));
    let long = table::guest_text(e, r.long_name);
    let padding = opcode >= nvse_end() && long.is_empty();
    let params = (0..u32::from(r.param_count))
        .map(|i| {
            let p = r.params + 12 * i;
            (
                table::guest_text(e, e.mem.u32(p)),
                e.mem.u32(p + 4),
                e.mem.u8(p + 8) != 0,
            )
        })
        .collect();
    Info {
        opcode,
        short: table::guest_text(e, r.short_name),
        help: if padding {
            PADDING_HELP.to_vec()
        } else {
            table::guest_text(e, r.help)
        },
        needs_ref: r.needs_ref > 0,
        params,
        execute: r.execute != 0,
        eval: r.eval != 0,
        expression: r.parse == EXPRESSION_PARSE,
        returns: table::return_type(e, opcode),
        plugin: if opcode >= PLUGIN_OPCODES && !padding {
            plugin_of(e, opcode)
        } else {
            None
        },
        long,
    }
}

/// Every record of the table, in opcode order (`m_commands`).
fn all(e: &Engine) -> Vec<Info> {
    (ScriptTables::FIRST_OPCODE..e.script_tables().max_opcode)
        .map(|op| info(e, op))
        .collect()
}

// ---- the texts ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp: StringForParamType.
/// `StringForParamType`.
fn param_type_name(kind: u32) -> &'static str {
    match kind {
        0x00 => "String",
        0x01 => "Integer",
        0x02 => "Float",
        0x03 => "ObjectID",
        0x04 => "ObjectRef",
        0x05 => "ActorValue",
        0x06 => "Actor",
        0x07 => "SpellItem",
        0x08 => "Axis",
        0x09 => "Cell",
        0x0a => "AnimationGroup",
        0x0b => "MagicItem",
        0x0c => "Sound",
        0x0d => "Topic",
        0x0e => "Quest",
        0x0f => "Race",
        0x10 => "Class",
        0x11 => "Faction",
        0x12 => "Sex",
        0x13 => "Global",
        0x14 => "Furniture",
        0x15 => "Object",
        0x16 => "VariableName",
        0x17 => "QuestStage",
        0x18 => "MapMarker",
        0x19 => "ActorBase",
        0x1a => "Container",
        0x1b => "WorldSpace",
        0x1c => "CrimeType",
        0x1d => "AIPackage",
        0x1e => "CombatStyle",
        0x1f => "MagicEffect",
        0x20 => "FormType",
        0x21 => "WeatherID",
        0x22 => "NPC",
        0x23 => "Owner",
        0x24 => "EffectShader",
        0x25 => "FormList",
        0x26 => "MenuIcon",
        0x27 => "Perk",
        0x28 => "Note",
        0x29 => "MiscStat",
        0x2a => "ImageSpaceModifier",
        0x2b => "ImageSpace",
        0x2c => "Double",
        0x2d => "ScriptVar",
        0x2e => "unk2E",
        0x2f => "EncounterZone",
        0x30 => "IdleForm",
        0x31 => "Message",
        0x32 => "InvObjectOrFormList",
        0x33 => "Alignment",
        0x34 => "EquipType",
        0x35 => "NonFormList",
        0x36 => "SoundFile",
        0x37 => "CriticalStage",
        0x38 => "LeveledOrBaseChar",
        0x39 => "LeveledOrBaseCreature",
        0x3a => "LeveledChar",
        0x3b => "LeveledCreature",
        0x3c => "LeveledItem",
        0x3d => "AnyForm",
        0x3e => "Reputation",
        0x3f => "Casino",
        0x40 => "CasinoChip",
        0x41 => "Challenge",
        0x42 => "CaravanMoney",
        0x43 => "CaravanCard",
        0x44 => "CaravanDeck",
        0x45 => "Region",
        0x100 => "ArrayVar",
        _ => "<unknown>",
    }
}

/// `kNVSEParamType_*`: a bit per token type (`kTokenType_*`), with the ambiguous type's bit
/// (15) on the basic types.
mod nvse_param {
    const AMBIGUOUS: u32 = 1 << 15;
    pub const NUMBER: u32 = 1 | AMBIGUOUS;
    pub const BOOLEAN: u32 = 1 << 1 | AMBIGUOUS;
    pub const STRING: u32 = 1 << 2 | AMBIGUOUS;
    pub const FORM: u32 = 1 << 3 | AMBIGUOUS;
    pub const ARRAY: u32 = 1 << 6 | AMBIGUOUS;
    pub const ARRAY_ELEMENT: u32 = 1 << 7 | AMBIGUOUS;
    pub const SLICE: u32 = 1 << 8;
    pub const COMMAND: u32 = 1 << 9;
    pub const VARIABLE: u32 = 1 << 10;
    pub const NUMERIC_VAR: u32 = 1 << 11;
    pub const REF_VAR: u32 = 1 << 12;
    pub const STRING_VAR: u32 = 1 << 13;
    pub const ARRAY_VAR: u32 = 1 << 14;
    pub const FOR_EACH_CONTEXT: u32 = 1 << 17;
    pub const COLLECTION: u32 = ARRAY | STRING;
    pub const ARRAY_VAR_OR_ELEMENT: u32 = ARRAY_VAR | ARRAY_ELEMENT;
    pub const BASIC_TYPE: u32 = ARRAY | STRING | NUMBER | FORM;
    pub const NO_TYPE_CHECK: u32 = 0;
    pub const FORM_OR_NUMBER: u32 = FORM | NUMBER;
    pub const STRING_OR_NUMBER: u32 = STRING | NUMBER;
    pub const PAIR: u32 = 1 << 21;
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), ScriptUtils.cpp: StringForNVSEParamType.
/// `StringForNVSEParamType`.
fn nvse_param_type_name(kind: u32) -> &'static str {
    use nvse_param::*;
    match kind {
        NUMBER => "Number",
        BOOLEAN => "Bool",
        STRING => "String",
        FORM => "Form",
        ARRAY => "Array",
        ARRAY_ELEMENT => "ArrayElement",
        SLICE => "Slice",
        COMMAND => "Command",
        VARIABLE => "Variable",
        NUMERIC_VAR => "NumericVar",
        REF_VAR => "RefVar",
        STRING_VAR => "StringVar",
        ARRAY_VAR => "ArrayVar",
        FOR_EACH_CONTEXT => "ForEachContext",
        COLLECTION => "Collection",
        ARRAY_VAR_OR_ELEMENT => "ArrayVarOrArrayElement",
        BASIC_TYPE => "AnyType",
        NO_TYPE_CHECK => "AnyType",
        FORM_OR_NUMBER => "FormOrNumber",
        STRING_OR_NUMBER => "StringOrNumber",
        PAIR => "Pair",
        _ => "<unknown>",
    }
}

impl Info {
    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
    // ParamInfo::GetArgTypeAsString.
    /// A parameter's type as text: an expression command's by xNVSE's types, another's by the
    /// game's.
    fn arg_type(&self, kind: u32) -> &'static str {
        if self.expression {
            nvse_param_type_name(kind)
        } else {
            param_type_name(kind)
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp: ParamInfo::GetAsString.
    /// `name:Type`, or `Type` for a parameter with no name.
    fn param_text(&self, name: &[u8], kind: u32) -> Vec<u8> {
        let mut out = Vec::new();
        if !name.is_empty() {
            out.extend_from_slice(name);
            out.push(b':');
        }
        out.extend_from_slice(self.arg_type(kind).as_bytes());
        out
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
    // CommandInfo::IsDeprecated.
    /// `EmptyCommand`, or a name ending in `OLD` or `BROKEN` (JIP's, ShowOff's and JG's way).
    fn deprecated(&self) -> bool {
        self.long == b"EmptyCommand"
            || self.long.ends_with(b"OLD")
            || self.long.ends_with(b"BROKEN")
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
    // CommandInfo::GetDescription.
    /// The help text with a full stop and a capital first letter; `|DEPRECATED|` (`Deprecated.` on
    /// the wiki) for a deprecated command, `none` (`[TO DOCUMENT]`) with no help.
    fn description(&self, for_wiki: bool) -> Vec<u8> {
        if self.deprecated() {
            return if for_wiki {
                b"Deprecated.".to_vec()
            } else {
                b"|DEPRECATED|".to_vec()
            };
        }
        if !self.help.is_empty() {
            let mut desc = self.help.clone();
            if !desc.ends_with(b".") {
                desc.push(b'.');
            }
            desc[0] = desc[0].to_ascii_uppercase();
            return desc;
        }
        if for_wiki {
            b"[TO DOCUMENT]".to_vec()
        } else {
            b"none".to_vec()
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp: GetReturnTypeStr,
    // CommandReturnTypeToString.
    /// `1/0` for a name starting with `Is` or `GetIs` (without case), else its return type.
    fn return_text(&self) -> &'static str {
        let starts = |pre: &[u8]| {
            self.long.len() >= pre.len() && self.long[..pre.len()].eq_ignore_ascii_case(pre)
        };
        if starts(b"Is") || starts(b"GetIs") {
            return "1/0";
        }
        match self.returns {
            Returns::Ambiguous => "Ambiguous",
            Returns::Default => "Float",
            Returns::Form => "Form",
            Returns::String => "String",
            Returns::Array => "Array",
            Returns::ArrayIndex => "Array index",
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
    // CommandInfo::GetOriginName.
    fn origin(&self) -> Vec<u8> {
        if let Some((name, _)) = self.plugin {
            return name.as_bytes().to_vec();
        }
        if self.opcode < NVSE_FIRST {
            b"Base game".to_vec()
        } else {
            b"NVSE".to_vec()
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
    // CommandInfo::GetWikiStyleOriginName.
    /// The plugin's name as the GECK wiki's origin template (`origin`) or its category
    /// names it.
    fn wiki_origin(&self, origin: bool) -> Vec<u8> {
        if self.opcode < NVSE_FIRST {
            return b"Base game (cannot guess from which version it was introduced)".to_vec();
        }
        let Some((name, _)) = self.plugin else {
            return b"NVSE".to_vec();
        };
        let mut name = name.as_bytes().to_vec();
        if name.starts_with(b"JIP") {
            return b"JIP".to_vec();
        }
        if name == b"SUP NVSE Plugin" {
            return if origin { b"SUP".to_vec() } else { name };
        }
        if name.starts_with(b"kNVSE") {
            return name;
        }
        if name.ends_with(b" Plugin") {
            name.truncate(name.len() - b" Plugin".len());
        }
        if origin {
            if name.ends_with(b"NVSE") || name.ends_with(b"nvse") {
                name.truncate(name.len() - 4);
            }
            if name.ends_with(b" ") {
                name.pop();
            }
        } else if name.ends_with(b"NVSE") && !name.ends_with(b" NVSE") && name.len() > 4 + 4 {
            name.truncate(name.len() - 4);
            name.extend_from_slice(b" NVSE");
        }
        name
    }
}

/// `%#x`: `0x` and the lower-case digits, `0` for zero.
fn alt_hex(v: u32) -> String {
    if v == 0 {
        "0".to_string()
    } else {
        format!("{v:#x}")
    }
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

// ---- DumpDocs ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
// CommandInfo::DumpFunctionDef.
fn dump_function_def(e: &mut Engine, c: &Info) {
    let reference: &[u8] = if c.needs_ref { b"reference." } else { b"" };
    crate::log::message(
        e,
        &cat(&[
            b"<br>(",
            c.return_text().as_bytes(),
            b") ",
            reference,
            b"<a href=\"#",
            &c.long,
            b"\">",
            &c.long,
            b"</a> ",
        ]),
    );
    for (name, kind, optional) in &c.params {
        let text = c.param_text(name, *kind);
        let line = if *optional {
            cat(&[b"<i>", &text, b"</i> "])
        } else {
            cat(&[&text, b" "])
        };
        crate::log::message(e, &line);
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp: CommandInfo::DumpDocs.
fn dump_docs_of(e: &mut Engine, c: &Info) {
    crate::log::message(
        e,
        &cat(&[b"<p><a name=\"", &c.long, b"\"></a><b>", &c.long, b"</b> "]),
    );
    let alias: &[u8] = if c.short.is_empty() {
        b"none"
    } else {
        &c.short
    };
    crate::log::message(
        e,
        &cat(&[
            b"<br><b>Alias:</b> ",
            alias,
            b"<br><b>Parameters:</b>",
            c.params.len().to_string().as_bytes(),
        ]),
    );
    for (name, kind, optional) in &c.params {
        let text = c.param_text(name, *kind);
        let line = if *optional {
            cat(&[b"<br>&nbsp;&nbsp;&nbsp;<i>", &text, b"</i> "])
        } else {
            cat(&[b"<br>&nbsp;&nbsp;&nbsp;", &text, b" "])
        };
        crate::log::message(e, &line);
    }
    let yes_no = |b: bool| -> &'static [u8] {
        if b {
            b"Yes"
        } else {
            b"No"
        }
    };
    let line = cat(&[
        b"<br><b>Return Type:</b> ",
        c.return_text().as_bytes(),
        b"<br><b>Opcode:</b> ",
        format!("{:>4} ({})", alt_hex(c.opcode), c.opcode).as_bytes(),
        b"<br><b>Origin:</b> ",
        &c.origin(),
        b"<br><b>Condition Function:</b> ",
        yes_no(c.eval),
        b"<br><b>Requires calling reference:</b> ",
        yes_no(c.needs_ref),
        b"<br><b>Description:</b> ",
        &c.description(false),
        b"</p>",
    ]);
    crate::log::message(e, &line);
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
// CommandTable::DumpCommandDocumentation.
/// The table's commands from `start` on, as a quick list or in detail: named commands only,
/// deprecated ones only with `show_deprecated`, condition-only ones (no execute) only with
/// `show_condition_only`.
pub fn dump_command_documentation(
    e: &mut Engine,
    quick_list: bool,
    start: u32,
    show_condition_only: bool,
    show_deprecated: bool,
) {
    let nvse: &str = if start == NVSE_FIRST { "NVSE " } else { "" };
    let head = format!("{nvse}Commands starting from opcode {}:", alt_hex(start));
    crate::log::message(e, head.as_bytes());
    crate::log::message(
        e,
        if quick_list {
            b"<br><b>Function Quick Reference</b>"
        } else {
            b"<hr><br><b>Functions In Detail</b>"
        },
    );
    for c in all(e) {
        if c.long.is_empty() || (!show_deprecated && c.deprecated()) || c.opcode < start {
            continue;
        }
        if !(c.execute || show_condition_only) {
            continue;
        }
        if quick_list {
            dump_function_def(e, &c);
        } else {
            dump_docs_of(e, &c);
        }
    }
}

// ---- the wiki pages ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
// CommandInfo::DumpWikiDocs.
/// A command's GECK wiki page (the `{{Function}}` template, its categories and, for a command
/// with an alias, the redirect page to make), `version` its `originVersion`.
fn dump_wiki_docs(e: &mut Engine, c: &Info, version: &[u8]) {
    if c.long.is_empty() {
        return;
    }
    let msg = |e: &mut Engine, parts: &[&[u8]]| crate::log::message(e, &cat(parts));
    if c.deprecated() {
        msg(e, &[b"{{Deprecated}}"]);
    }
    msg(e, &[b"{{Function"]);
    msg(e, &[b" |origin = ", &c.wiki_origin(true)]);
    let version: &[u8] = if version.is_empty() {
        b"[TO SPECIFY]"
    } else {
        version
    };
    msg(e, &[b" |originVersion = ", version]);
    msg(e, &[b" |summary = ", &c.description(true)]);
    msg(e, &[b" |name = ", &c.long]);
    if !c.short.is_empty() {
        msg(e, &[b" |alias = ", &c.short]);
    }
    msg(
        e,
        &[b" |returnVal = [Insert return val name here, or remove this arg]"],
    );
    msg(
        e,
        &[
            b" |returnType = ",
            c.return_text().as_bytes(),
            b" [remove this arg if func returns nothing]",
        ],
    );
    if c.needs_ref {
        msg(e, &[b" |referenceType = reference"]);
    }
    let condition: &[u8] = match (c.eval, c.execute) {
        (true, false) => b"Condition",
        (true, true) => b"Both",
        (false, true) => b"Script",
        (false, false) => b"None (should never happen)",
    };
    msg(e, &[b" |conditionFunc = ", condition]);
    let (mut array_arg, mut string_arg) = (false, false);
    if !c.params.is_empty() {
        msg(e, &[b" |arguments ="]);
        msg(e, &[b"  {{FunctionArgument"]);
        let upper = crate::vars::upper_table(e);
        for (i, (name, kind, optional)) in c.params.iter().enumerate() {
            let arg_type = c.arg_type(*kind).as_bytes();
            let same =
                crate::vars::str_compare(&upper, arg_type, name) == std::cmp::Ordering::Equal;
            let arg_name: &[u8] = if !name.is_empty() && !same {
                name
            } else {
                b"[Enter Arg Name]"
            };
            msg(e, &[b"   |Name = ", arg_name]);
            let contains = |what: &[u8]| arg_type.windows(what.len()).any(|w| w == what);
            array_arg |= contains(b"Array");
            string_arg |= contains(b"String");
            msg(e, &[b"   |Type = ", arg_type]);
            if *optional {
                msg(e, &[b"   |Optional = y"]);
                msg(e, &[b"   |Value = [INSERT DEFAULT VALUE]"]);
            }
            if i + 1 < c.params.len() {
                msg(e, &[b"  }}{{FunctionArgument"]);
            } else {
                msg(e, &[b"  }}"]);
            }
        }
    }
    msg(e, &[b"}}"]);
    msg(
        e,
        &[b"[[Category:Functions (", &c.wiki_origin(false), b")]]"],
    );
    msg(e, &[b"[[Category:? [Add other relevant categories] ]]"]);
    if c.deprecated() {
        msg(e, &[b"[[Category:Deprecated Functions]]"]);
    }
    if c.expression {
        msg(e, &[b"[[Category:NVSE-Aware Functions]]"]);
    }
    if array_arg || matches!(c.returns, Returns::Array | Returns::ArrayIndex) {
        msg(e, &[b"[[Category:Array Functions]]"]);
    }
    if string_arg || c.returns == Returns::String {
        msg(e, &[b"[[Category:String Functions]]"]);
    }
    msg(e, &[b""]);
    if !c.short.is_empty() {
        msg(
            e,
            &[
                b"!!! Create a page with the name \"",
                &c.short,
                b"\" and the following text:",
            ],
        );
        msg(
            e,
            &[
                b"#redirect [[",
                &c.long,
                b"]]\n[[Category:Function Alias]]\n",
            ],
        );
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
// CommandTable::DumpWikiDocs.
/// Every command's wiki page, each followed by two empty lines.
pub fn dump_all_wiki_docs(e: &mut Engine) {
    crate::log::message(e, b"## Command List\n");
    for c in all(e) {
        dump_wiki_docs(e, &c, b"");
        crate::log::message(e, b"\n\n");
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
// CommandTable::GetByOpcodeRange.
/// The opcodes from `start` to `stop` (both included), none when the range is reversed, ends past
/// the table, starts below it or is longer than the table.
fn opcode_range(e: &Engine, start: u32, stop: u32) -> Vec<u32> {
    let base = ScriptTables::FIRST_OPCODE;
    let size = e.script_tables().max_opcode - base;
    let last = stop.wrapping_sub(base);
    if last >= size || start > stop {
        return Vec::new();
    }
    let count = stop.wrapping_sub(start).wrapping_add(1);
    if size < count {
        return Vec::new();
    }
    let first = start.wrapping_sub(base);
    if first > last {
        return Vec::new();
    }
    (first..=last).map(|i| base + i).collect()
}

// ---- the commands ----

/// `kParamType_Integer`, `kParamType_String`.
const INTEGER: u32 = 0x01;
const STRING: u32 = 0x00;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}

const FOUR_OPTIONAL_INTS: &[Param] = &[
    p("int", INTEGER, true),
    p("int", INTEGER, true),
    p("int", INTEGER, true),
    p("int", INTEGER, true),
];
const ONE_STRING: &[Param] = &[p("string", STRING, false)];
const TWO_INTS_ONE_OPTIONAL_STRING: &[Param] = &[
    p("int", INTEGER, false),
    p("int", INTEGER, false),
    p("string", STRING, true),
];

const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref: false,
        params,
        execute,
        eval: None,
        returns: Returns::Default,
    }
}

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x166a, "DumpDocs", "", "", FOUR_OPTIONAL_INTS, dump_docs),
    cmd(0x166b, "DumpCommandWikiDoc", "DumpWikiDoc", "dumps wiki - style documentation for a command", ONE_STRING, dump_command_wiki_doc),
    cmd(0x166c, "DumpCommandWikiDocs", "DumpWikiDocs", "dumps wiki - style documentation for multiple commands", TWO_INTS_ONE_OPTIONAL_STRING, dump_command_wiki_docs),
];

/// A line on the console, in console mode only.
fn console_line(e: &mut Engine, text: &str) {
    if is_console_mode(e) {
        console_print(e, text, &[]);
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp: Cmd_DumpDocs_Execute.
/// `DumpDocs [quickList] [startOpcode] [conditionOnly] [deprecated]`: the table's documentation
/// to `nvse.log` (from xNVSE's first opcode, `0x1400`, by default).
fn dump_docs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let quick_list = read.int(0, 0) as i32 != 0;
    let start = read.int(1, NVSE_FIRST);
    let condition_only = read.int(2, 0) as i32 != 0;
    let deprecated = read.int(3, 0) as i32 != 0;
    console_line(e, "Dumping Command Docs");
    dump_command_documentation(e, quick_list, start, condition_only, deprecated);
    console_line(e, "Done Dumping Command Docs");
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_DumpCommandWikiDoc_Execute.
/// `DumpCommandWikiDoc name`: the command's wiki page to `nvse.log`.
fn dump_command_wiki_doc(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let name = crate::commands::arg_in_buffer(e, &read, 0, 0x400, "DumpCommandWikiDoc");
    match table::opcode_by_name(e, &name) {
        Some(opcode) => {
            console_line(e, "Dumping wiki-style documentation for function.");
            let c = info(e, opcode);
            dump_wiki_docs(e, &c, b"");
            console_line(e, "Finished dumping wiki-style documentation for function.");
        }
        None => console_line(
            e,
            "Unable to find command with that name. Double-check for a spelling error, or if it's \
             being registered.",
        ),
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_Script.cpp:
// Cmd_DumpCommandWikiDocs_Execute.
/// `DumpCommandWikiDocs start end [version]`: the wiki pages of the commands from `start` to
/// `end` (all of them, with a heading, for `0 0`) to `nvse.log`.
fn dump_command_wiki_docs(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let start = read.int(0, 0);
    let end = read.int(1, 0);
    let version = crate::commands::arg_in_buffer(e, &read, 2, 0x100, "DumpCommandWikiDocs");
    if start == 0 && end == 0 {
        console_line(e, "Dumping wiki-style documentation for all functions.");
        dump_all_wiki_docs(e);
        console_line(
            e,
            "Finished dumping wiki-style documentation for all functions.",
        );
        return handled(true);
    }
    let opcodes = opcode_range(e, start, end);
    if opcodes.is_empty() {
        console_line(e, "Unable to find commands.");
        return handled(true);
    }
    console_line(
        e,
        "Dumping wiki-style documentation for functions in opcode range.",
    );
    for opcode in opcodes {
        let c = info(e, opcode);
        dump_wiki_docs(e, &c, &version);
    }
    console_line(
        e,
        "Finished dumping wiki-style documentation for functions.",
    );
    handled(true)
}

// ---- the console's help listing ----

/// `SubStrCI(src, sub)`: whether `sub` is in `src`, bytes compared through the game's
/// `tolower`; never for an empty `src` or `sub`.
fn contains_ci(src: &[u8], sub: &[u8]) -> bool {
    if src.is_empty() || sub.is_empty() || sub.len() > src.len() {
        return false;
    }
    src.windows(sub.len())
        .any(|w| w.iter().zip(sub).all(|(a, b)| a.eq_ignore_ascii_case(b)))
}

/// `CommandTable::GetParentPlugin`: none below xNVSE's range, xNVSE (`NVSE`, version 6) in it up to
/// the plugins' range, else the plugin the command was registered under.
fn parent_plugin(c: &Info) -> Option<(&'static str, u32)> {
    if c.opcode == 0 || c.opcode < NVSE_FIRST {
        None
    } else if c.opcode < PLUGIN_OPCODES {
        Some(("NVSE", crate::VERSION.0))
    } else {
        c.plugin
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Hooks_Script.cpp:
// PatchHelpCommand::HandlePrintCommands.
/// The listing's line of a command, `None` when the filter does not match it.
fn help_line(c: &Info, filter: &[u8]) -> Option<Vec<u8>> {
    let parent = parent_plugin(c);
    let matches = filter.is_empty()
        || contains_ci(&c.help, filter)
        || contains_ci(&c.long, filter)
        || contains_ci(&c.short, filter)
        || parent.is_some_and(|(name, _)| contains_ci(name.as_bytes(), filter));
    if !matches {
        return None;
    }
    let version = match parent {
        Some((name, _)) if name.eq_ignore_ascii_case("nvse") => {
            let (major, minor, beta) = crate::VERSION;
            format!("[{name} {major}.{minor}.{beta}] ")
        }
        Some((name, version)) => format!("[{name} {version}] "),
        None => String::new(),
    };
    let mut line = version.into_bytes();
    line.extend_from_slice(&c.long);
    match (c.help.is_empty(), c.short.is_empty()) {
        (false, false) => line.extend(cat(&[b" (", &c.short, b") -> ", &c.help])),
        (false, true) => line.extend(cat(&[b" -> ", &c.help])),
        (true, false) => line.extend(cat(&[b" (", &c.short, b")"])),
        (true, true) => {}
    }
    Some(line)
}

/// `005BCBC2`, words (the filter text): xNVSE's listing of the script functions in place of the
/// game's loop; 1, the loop is passed over.
fn help_listing(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let filter = table::guest_text(e, w.first().copied().unwrap_or(0));
    for c in all(e) {
        if let Some(line) = help_line(&c, &filter) {
            crate::events::with_guest_text(e, &line, |e, at| console_print(e, "%s", &[at]));
        }
    }
    Some(1)
}

/// Registers the help listing's seam.
pub fn install(e: &mut Engine) {
    e.on_site(HELP_LISTING, help_listing);
}

#[cfg(test)]
#[path = "docs_tests.rs"]
mod tests;
