//! The script command table in guest memory (ADR-0016 §2; build step I-2), as xNVSE builds it in
//! `CommandTable::Init` (`CommandTable.cpp` (xNVSE)): a copy of the exe's 0x280 script records
//! (`0x1000..0x127f`, from `0x01190910`; no static initializer writes them), xNVSE's padding
//! command up to `0x1400`, then xNVSE's 654 commands. The engine's
//! [`ScriptTables`] then point at the copy, which is what xNVSE's `ApplyPatch` does to the exe's
//! immediates: every reader (`GetFunctionDef`, the expression tokenizer, the condition code, the
//! parameter code) sees the extended table.
//!
//! A record (`CommandInfo` (xNVSE), 0x28 bytes): `+0x00` long name, `+0x04` short name, `+0x08`
//! opcode, `+0x0C` help text, `+0x10` needs a reference (u16), `+0x12` parameter count (u16),
//! `+0x14` parameters (`ParamInfo`, 12 bytes each: name, type, optional), `+0x18` execute,
//! `+0x1C` parse, `+0x20` eval, `+0x24` flags. Return types are kept beside the table, by opcode,
//! as xNVSE keeps them (`CommandMetadata`).
//!
//! A command not built yet keeps its record (its names, so the script error log names it) with no
//! handler and no parameters: a line that calls it fails and the script stops for that run, as
//! it does without the DLL. Its record is completed when its command is built.
//!
//! A plugin built in (JIP LN NVSE, `crates/jip`) grows the table past xNVSE's commands to its own
//! opcodes ([`grow_to`]) and writes its records there ([`add_plugin_record`]), as xNVSE's
//! `PluginManager` registers a plugin DLL's commands; the slots no built-in fills stay empty.

use crate::names::{FIRST_OPCODE, NAMES};
use crate::Nvse;
use engine_core::extend::{ScriptTables, EVAL_BASE, EXECUTE_BASE, ROUTINE_BASE};
use engine_core::mem::BUILTIN_DATA;
use engine_core::prelude::*;
use std::collections::HashMap;

/// Where the records go: the start of the built-ins' data.
pub const TABLE_AT: u32 = BUILTIN_DATA.start;
/// Room for records (0x3333 of them), then the strings and parameter lists.
pub const TABLE_ROOM: u32 = 0x8_0000;
/// The built-ins' strings and parameter lists, from here up.
pub const DATA_AT: u32 = TABLE_AT + TABLE_ROOM;

/// xNVSE's `Cmd_Default_Execute` (xNVSE): returns true. The execute word of its padding command.
pub const DEFAULT_EXECUTE: u32 = ROUTINE_BASE;
/// xNVSE's `Cmd_Default_Parse` (xNVSE): the game's default parse routine,
/// `g_defaultParseCommand` (`005b1ba0`), with the same four words. The parse word of xNVSE's
/// commands and of its padding command.
pub const DEFAULT_PARSE: u32 = ROUTINE_BASE + 4;
/// The exe's default parse routine.
const EXE_DEFAULT_PARSE: u32 = 0x005b_1ba0;

/// xNVSE's other parse routines, the parse words of its expression-format commands:
/// `Cmd_Expression_Parse`, `Cmd_Let_Parse`, `Cmd_BeginLoop_Parse` (`while`, `ForEach`,
/// `ForEachAlt`), `Cmd_Loop_Parse`, `Cmd_Null_Parse`, `Cmd_Function_Parse` and `Cmd_Call_Parse`
/// (`CommandTable.h`, `Commands_Scripting.cpp` (xNVSE)), built in [`crate::compiler`]: the game's
/// script compiler calls them through a record's parse word. The words are also read by the
/// decompiler ([`crate::decompile`]), which tells an expression-format command's argument bytes
/// from the game's by them, as xNVSE's `ScriptAnalyzer` does. (`Cmd_CallAfter_Parse` is the
/// parse word of no command.)
pub const EXPRESSION_PARSE: u32 = ROUTINE_BASE + 0x20;
pub const LET_PARSE: u32 = ROUTINE_BASE + 0x24;
pub const BEGIN_LOOP_PARSE: u32 = ROUTINE_BASE + 0x28;
pub const LOOP_PARSE: u32 = ROUTINE_BASE + 0x2c;
pub const NULL_PARSE: u32 = ROUTINE_BASE + 0x30;
pub const FUNCTION_PARSE: u32 = ROUTINE_BASE + 0x34;
pub const CALL_PARSE: u32 = ROUTINE_BASE + 0x38;
const PARSE_ROUTINES: [(u32, AbiFn); 7] = [
    (EXPRESSION_PARSE, crate::compiler::expression_parse),
    (LET_PARSE, crate::compiler::let_parse),
    (BEGIN_LOOP_PARSE, crate::compiler::begin_loop_parse),
    (LOOP_PARSE, crate::compiler::loop_parse),
    (NULL_PARSE, crate::compiler::null_parse),
    (FUNCTION_PARSE, crate::compiler::function_parse),
    (CALL_PARSE, crate::compiler::call_parse),
];
/// `Cmd_Expression_Plugin_Parse` (xNVSE `CommandTable.h`): the value a plugin puts in its record's
/// parse word to ask for xNVSE's expression parser, which `PluginManager::RegisterCommand` turns
/// into `Cmd_Expression_Parse`. A plugin record that asks for it gets [`EXPRESSION_PARSE`]
/// ([`PluginRecord::expression`]); nothing reads this value itself yet.
pub const PLUGIN_EXPRESSION_PARSE: u32 = 0x0800_0000;

/// Every xNVSE command whose parse word is not `Cmd_Default_Parse`, from xNVSE 6.4.9's source:
/// the commands defined with `DEFINE_CMD_ALT_EXP`, `DEFINE_COMMAND_EXP` (and their `_VER` forms)
/// that `CommandTable::AddCommandsV*` registers, and the `CommandInfo` literals of
/// `Commands_Scripting.cpp`.
#[rustfmt::skip]
pub static PARSE_WORDS: &[(&str, u32)] = &[
    ("Let", LET_PARSE),
    ("while", BEGIN_LOOP_PARSE), ("ForEach", BEGIN_LOOP_PARSE), ("ForEachAlt", BEGIN_LOOP_PARSE),
    ("loop", LOOP_PARSE),
    ("continue", NULL_PARSE), ("break", NULL_PARSE),
    ("@PushExecutionContext", NULL_PARSE), ("@PopExecutionContext", NULL_PARSE),
    ("Function", FUNCTION_PARSE),
    ("Call", CALL_PARSE),
    ("eval", EXPRESSION_PARSE), ("testexpr", EXPRESSION_PARSE), ("ToString", EXPRESSION_PARSE),
    ("TypeOf", EXPRESSION_PARSE), ("Print", EXPRESSION_PARSE), ("PrintDebug", EXPRESSION_PARSE),
    ("PrintF", EXPRESSION_PARSE), ("PrintDebugF", EXPRESSION_PARSE),
    ("SetFunctionValue", EXPRESSION_PARSE), ("SetModLocalData", EXPRESSION_PARSE),
    ("GetModLocalData", EXPRESSION_PARSE), ("ModLocalDataExists", EXPRESSION_PARSE),
    ("RemoveModLocalData", EXPRESSION_PARSE),
    ("Assert", EXPRESSION_PARSE), ("CallAfterFrames", EXPRESSION_PARSE),
    ("CallAfterSeconds", EXPRESSION_PARSE), ("CallForSeconds", EXPRESSION_PARSE),
    ("CallWhen", EXPRESSION_PARSE), ("CallWhile", EXPRESSION_PARSE),
    ("CallWhilePerSeconds", EXPRESSION_PARSE), ("CompileScript", EXPRESSION_PARSE),
    ("CreateFormList", EXPRESSION_PARSE), ("DebugPrintVar", EXPRESSION_PARSE),
    ("DispatchEvent", EXPRESSION_PARSE), ("DispatchEventAlt", EXPRESSION_PARSE),
    ("DumpEventHandlers", EXPRESSION_PARSE), ("FireChallenge", EXPRESSION_PARSE),
    ("GetDoorSound", EXPRESSION_PARSE), ("GetEventHandlers", EXPRESSION_PARSE),
    ("GetFalloutDirectory", EXPRESSION_PARSE), ("GetHigherPriorityEventHandlers", EXPRESSION_PARSE),
    ("GetLowerPriorityEventHandlers", EXPRESSION_PARSE), ("GetRawFormIDString", EXPRESSION_PARSE),
    ("GetSelfAlt_OLD", EXPRESSION_PARSE), ("IsEventHandlerFirst", EXPRESSION_PARSE),
    ("IsEventHandlerLast", EXPRESSION_PARSE), ("Jmp", EXPRESSION_PARSE),
    ("Jmp_If_False", EXPRESSION_PARSE), ("Jmp_If_True", EXPRESSION_PARSE),
    ("MatchesAnyOf", EXPRESSION_PARSE), ("PluginVersion", EXPRESSION_PARSE),
    ("PrintVar", EXPRESSION_PARSE), ("RemoveEventHandler", EXPRESSION_PARSE),
    ("SetEventHandler", EXPRESSION_PARSE), ("SetEventHandlerAlt", EXPRESSION_PARSE),
    ("SetModelPath", EXPRESSION_PARSE), ("SetTexturePath", EXPRESSION_PARSE),
    ("Ternary", EXPRESSION_PARSE), ("TernaryUDF", EXPRESSION_PARSE),
    ("ToNumber", EXPRESSION_PARSE), ("ValidateRegex", EXPRESSION_PARSE),
    ("ar_All", EXPRESSION_PARSE), ("ar_Any", EXPRESSION_PARSE), ("ar_Append", EXPRESSION_PARSE),
    ("ar_BadNumericIndex", EXPRESSION_PARSE), ("ar_BadStringIndex", EXPRESSION_PARSE),
    ("ar_Cat", EXPRESSION_PARSE), ("ar_Copy", EXPRESSION_PARSE), ("ar_Count", EXPRESSION_PARSE),
    ("ar_CountWhere", EXPRESSION_PARSE), ("ar_CustomSort", EXPRESSION_PARSE),
    ("ar_DeepCopy", EXPRESSION_PARSE), ("ar_DeepEquals", EXPRESSION_PARSE),
    ("ar_Dump", EXPRESSION_PARSE), ("ar_DumpF", EXPRESSION_PARSE), ("ar_Erase", EXPRESSION_PARSE),
    ("ar_Exists", EXPRESSION_PARSE), ("ar_Filter", EXPRESSION_PARSE), ("ar_Find", EXPRESSION_PARSE),
    ("ar_FindWhere", EXPRESSION_PARSE), ("ar_First", EXPRESSION_PARSE),
    ("ar_ForEach", EXPRESSION_PARSE), ("ar_Generate", EXPRESSION_PARSE),
    ("ar_GetNth", EXPRESSION_PARSE), ("ar_HasKey", EXPRESSION_PARSE), ("ar_Init", EXPRESSION_PARSE),
    ("ar_Insert", EXPRESSION_PARSE), ("ar_InsertRange", EXPRESSION_PARSE),
    ("ar_Keys", EXPRESSION_PARSE), ("ar_Last", EXPRESSION_PARSE), ("ar_List", EXPRESSION_PARSE),
    ("ar_Map", EXPRESSION_PARSE), ("ar_MapTo", EXPRESSION_PARSE), ("ar_Next", EXPRESSION_PARSE),
    ("ar_Packed", EXPRESSION_PARSE), ("ar_Prev", EXPRESSION_PARSE), ("ar_Range", EXPRESSION_PARSE),
    ("ar_Resize", EXPRESSION_PARSE), ("ar_Size", EXPRESSION_PARSE), ("ar_Sort", EXPRESSION_PARSE),
    ("ar_SortAlpha", EXPRESSION_PARSE), ("ar_Unique", EXPRESSION_PARSE),
    ("sv_Destruct", EXPRESSION_PARSE), ("sv_Percentify", EXPRESSION_PARSE),
    ("sv_Split", EXPRESSION_PARSE), ("sv_ToLower", EXPRESSION_PARSE),
    ("sv_ToUpper", EXPRESSION_PARSE), ("sv_Trim", EXPRESSION_PARSE),
];

/// A command's parse word by its long name: its entry of [`PARSE_WORDS`], else the default.
pub fn parse_word(name: &str) -> u32 {
    PARSE_WORDS
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(DEFAULT_PARSE, |&(_, w)| w)
}

/// The console command table (`0x0118e8e0`, opcodes `0x100..0x1cc`): xNVSE re-exports 20 of
/// its records as script commands.
const CONSOLE_TABLE: u32 = 0x0118_e8e0;
const CONSOLE_COUNT: u32 = 0xcd;

/// The size of one record.
pub const RECORD: u32 = ScriptTables::RECORD;

/// A command's return type (`CommandReturnType` (xNVSE)): how `Let`, `eval` and the expression
/// evaluator read the result double.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Returns {
    /// A number (`kRetnType_Default`).
    #[default]
    Default,
    /// A form id in the low word (`kRetnType_Form`).
    Form,
    /// A string variable's id (`kRetnType_String`).
    String,
    /// An array variable's id (`kRetnType_Array`).
    Array,
    /// An array index (`kRetnType_ArrayIndex`).
    ArrayIndex,
    /// Decided per call (`kRetnType_Ambiguous`).
    Ambiguous,
}

/// One parameter (`ParamInfo` (xNVSE)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Param {
    /// `typeStr`: the parameter's name in the compiler's messages.
    pub name: &'static str,
    /// `typeID` (`kParamType_*`).
    pub kind: u32,
    /// `isOptional`.
    pub optional: bool,
}

/// A built command: its record's data and its Rust handlers.
#[derive(Clone, Copy, Debug)]
pub struct Command {
    pub opcode: u32,
    pub name: &'static str,
    /// Empty for none.
    pub short: &'static str,
    pub help: &'static str,
    pub needs_ref: bool,
    pub params: &'static [Param],
    /// `+0x18`: called with the eight words of [`crate::CommandArgs`].
    pub execute: AbiFn,
    /// `+0x20`: a condition's eval, called with [`crate::EvalArgs`]; `None` for a command that
    /// is not condition-capable.
    pub eval: Option<AbiFn>,
    pub returns: Returns,
}

/// The table's state on the engine.
#[derive(Default)]
pub struct State {
    /// The next free byte of the built-ins' data, and the end of what is mapped of it.
    top: u32,
    mapped: u32,
    /// Strings already written (`""` once for every empty name).
    strings: HashMap<Vec<u8>, u32>,
    /// Return types by opcode, from `0x1000`.
    pub returns: Vec<Returns>,
}

/// Vanilla commands xNVSE records as returning forms (`AddCommandsV1`, `CommandTable.cpp`
/// (xNVSE)). Read only by xNVSE's evaluator and `Let`, which vanilla content never reaches: in
/// every preset (ADR-0016 §2).
const VANILLA_FORM_RESULTS: [u32; 13] = [
    0x1025, // PlaceAtMe
    0x10cd, // GetActionRef
    0x10ce, // GetSelf
    0x10cf, // GetContainer
    0x10e8, // GetCombatTarget
    0x10e9, // GetPackageTarget
    0x1113, // GetParentRef
    0x116b, // GetLinkedRef
    0x11bd, // PlaceAtMeHealthPercent
    0x11cf, // GetPlayerGrabbedRef
    0x124e, // GetOwnerLastTarget
    0x1265, // ObjectUnderTheReticule
    0x11d1, // PlaceLeveledActorAtMe
];

/// `Cmd_Default_Execute` (xNVSE): true.
fn default_execute(_: &mut Engine, _: &[u32]) -> Ret {
    crate::handled(true)
}

/// `Cmd_Default_Parse` (xNVSE): `g_defaultParseCommand(numParams, paramInfo, lineBuf,
/// scriptBuf)`, the exe's `005b1ba0`.
fn default_parse(e: &mut Engine, words: &[u32]) -> Ret {
    let mut four = [0u32; 4];
    for (to, from) in four.iter_mut().zip(words) {
        *to = *from;
    }
    e.call(EXE_DEFAULT_PARSE, &four)
}

/// `count` bytes of the built-ins' data, aligned to 4, mapped as needed.
pub fn alloc(e: &mut Engine, count: u32) -> u32 {
    let state = &mut e.ext::<Nvse>().table;
    if state.top == 0 {
        state.top = DATA_AT;
        state.mapped = DATA_AT;
    }
    let at = state.top;
    let end = at + count.next_multiple_of(4);
    assert!(end <= BUILTIN_DATA.end, "the built-ins' data is full");
    let mapped = state.mapped;
    state.top = end;
    if end > mapped {
        let to = end.next_multiple_of(0x1000);
        e.ext::<Nvse>().table.mapped = to;
        e.map(mapped, to - mapped);
    }
    at
}

/// A NUL-terminated copy of `text` in the built-ins' data, written once per text.
pub fn guest_str(e: &mut Engine, text: &[u8]) -> u32 {
    if let Some(&at) = e.ext::<Nvse>().table.strings.get(text) {
        return at;
    }
    let at = alloc(e, text.len() as u32 + 1);
    for (i, b) in text.iter().enumerate() {
        e.mem.set_u8(at + i as u32, *b);
    }
    e.mem.set_u8(at + text.len() as u32, 0);
    e.ext::<Nvse>().table.strings.insert(text.to_vec(), at);
    at
}

/// A parameter list in the built-ins' data (12 bytes each: name, type, optional); 0 for none.
fn guest_params(e: &mut Engine, params: &[Param]) -> u32 {
    if params.is_empty() {
        return 0;
    }
    let at = alloc(e, 12 * params.len() as u32);
    for (i, p) in params.iter().enumerate() {
        let name = guest_str(e, p.name.as_bytes());
        let slot = at + 12 * i as u32;
        e.mem.set_u32(slot, name);
        e.mem.set_u32(slot + 4, p.kind);
        e.mem.set_u32(slot + 8, u32::from(p.optional));
    }
    at
}

/// The fields of one record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub long_name: u32,
    pub short_name: u32,
    pub opcode: u32,
    pub help: u32,
    pub needs_ref: u16,
    pub param_count: u16,
    pub params: u32,
    pub execute: u32,
    pub parse: u32,
    pub eval: u32,
    pub flags: u32,
}

impl Record {
    pub fn read(e: &Engine, at: u32) -> Record {
        Record {
            long_name: e.mem.u32(at),
            short_name: e.mem.u32(at + 4),
            opcode: e.mem.u32(at + 8),
            help: e.mem.u32(at + 0xc),
            needs_ref: e.mem.u16(at + 0x10),
            param_count: e.mem.u16(at + 0x12),
            params: e.mem.u32(at + 0x14),
            execute: e.mem.u32(at + 0x18),
            parse: e.mem.u32(at + 0x1c),
            eval: e.mem.u32(at + 0x20),
            flags: e.mem.u32(at + 0x24),
        }
    }

    pub fn write(self, e: &mut Engine, at: u32) {
        e.mem.set_u32(at, self.long_name);
        e.mem.set_u32(at + 4, self.short_name);
        e.mem.set_u32(at + 8, self.opcode);
        e.mem.set_u32(at + 0xc, self.help);
        e.mem.set_u16(at + 0x10, self.needs_ref);
        e.mem.set_u16(at + 0x12, self.param_count);
        e.mem.set_u32(at + 0x14, self.params);
        e.mem.set_u32(at + 0x18, self.execute);
        e.mem.set_u32(at + 0x1c, self.parse);
        e.mem.set_u32(at + 0x20, self.eval);
        e.mem.set_u32(at + 0x24, self.flags);
    }
}

/// The NUL-terminated text at `at` (0: none).
pub fn guest_text(e: &Engine, at: u32) -> Vec<u8> {
    let mut out = Vec::new();
    if at == 0 {
        return out;
    }
    let mut p = at;
    loop {
        let b = e.mem.u8(p);
        if b == 0 {
            return out;
        }
        out.push(b);
        p += 1;
    }
}

/// The console record whose long name is `name`, compared without case (xNVSE's
/// `g_consoleCommands.GetByName`).
fn console_record(e: &Engine, name: &[u8]) -> Option<u32> {
    (0..CONSOLE_COUNT)
        .map(|i| CONSOLE_TABLE + i * RECORD)
        .find(|&at| guest_text(e, e.mem.u32(at)).eq_ignore_ascii_case(name))
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp: CommandTable::Add's
// g_cmdTableCache, CommandTable::GetByName and GetByOpcode.
/// The script command `name` names, as xNVSE's one name map finds it. Every record's long and
/// short name goes into the map under its case-insensitive hash (`StrHashCI`), the first record to
/// bring a hash keeping it: the console table's records first (they come in first, from opcode
/// 0x100), then the script table's in opcode order. A missing short name is the empty name. A name
/// the console table brought first maps to a console opcode, which the script table does not have:
/// `None`, as for no name.
pub fn opcode_by_name(e: &Engine, name: &[u8]) -> Option<u32> {
    let want = crate::events::hash_name(name);
    let names = |at: u32| [e.mem.u32(at), e.mem.u32(at + 4)];
    let matches = |ptr: u32| {
        let text = if ptr == 0 {
            Vec::new()
        } else {
            guest_text(e, ptr)
        };
        crate::events::hash_name(&text) == want
    };
    for i in 0..CONSOLE_COUNT {
        if names(CONSOLE_TABLE + i * RECORD).into_iter().any(matches) {
            return None;
        }
    }
    let tables = e.script_tables();
    (ScriptTables::FIRST_OPCODE..tables.max_opcode)
        .find(|&opcode| names(tables.record(opcode)).into_iter().any(matches))
}

/// [`opcode_by_name`]'s answers all at once, as xNVSE's `g_cmdTableCache` holds them: each name's
/// hash and the record that brought it first (`None` for a console command's), for a compile's
/// many lookups (the table does not change while a script compiles).
pub fn name_map(e: &Engine) -> HashMap<u32, Option<u32>> {
    let mut map = HashMap::new();
    let hash = |ptr: u32| {
        let text = if ptr == 0 {
            Vec::new()
        } else {
            guest_text(e, ptr)
        };
        crate::events::hash_name(&text)
    };
    for i in 0..CONSOLE_COUNT {
        let at = CONSOLE_TABLE + i * RECORD;
        for ptr in [e.mem.u32(at), e.mem.u32(at + 4)] {
            map.entry(hash(ptr)).or_insert(None);
        }
    }
    let tables = e.script_tables();
    for opcode in ScriptTables::FIRST_OPCODE..tables.max_opcode {
        let at = tables.record(opcode);
        for ptr in [e.mem.u32(at), e.mem.u32(at + 4)] {
            map.entry(hash(ptr)).or_insert(Some(opcode));
        }
    }
    map
}

/// The execute address of built command `opcode` of xNVSE's range.
pub fn execute_address(opcode: u32) -> u32 {
    EXECUTE_BASE + 4 * (opcode - FIRST_OPCODE)
}

/// The eval address of built command `opcode` of xNVSE's range.
pub fn eval_address(opcode: u32) -> u32 {
    EVAL_BASE + 4 * (opcode - FIRST_OPCODE)
}

/// Builds the table (xNVSE's `CommandTable::Init`, the script table's part) and moves the
/// engine's [`ScriptTables`] onto it.
pub fn install(e: &mut Engine) {
    e.register(DEFAULT_EXECUTE, default_execute);
    e.register(DEFAULT_PARSE, default_parse);
    for (routine, f) in PARSE_ROUTINES {
        e.register(routine, f);
    }
    let exe = ScriptTables::EXE;
    let vanilla = (exe.max_opcode - ScriptTables::FIRST_OPCODE) as usize;
    let max_opcode = FIRST_OPCODE + NAMES.len() as u32;
    let count = max_opcode - ScriptTables::FIRST_OPCODE;
    // The records and the one past the end, which the exe's `ScriptCompiler` constructor
    // (`005aea50`) reads (`end + 8`).
    let size = (count + 1) * RECORD;
    assert!(size <= TABLE_ROOM);
    // The whole room is mapped, zeros past the table's end: the condition code (`00681600`) and
    // a condition's `InitItem` (`006819e0`) read a function's record by its number with no test
    // of the table's end, so a condition naming a function of a plugin nv-rs does not have (a
    // VNV mod's JohnnyGuitar or ShowOff condition) reads a record with no eval and is false, not
    // memory that is not there.
    e.map(TABLE_AT, TABLE_ROOM);
    // `g_scriptCommands.Read`: the exe's records as they are.
    let bytes = e.mem.bytes(exe.start, vanilla as u32 * RECORD);
    for (i, b) in bytes.iter().enumerate() {
        e.mem.set_u8(TABLE_AT + i as u32, *b);
    }
    let mut returns = vec![Returns::Default; count as usize];
    for opcode in VANILLA_FORM_RESULTS {
        returns[(opcode - ScriptTables::FIRST_OPCODE) as usize] = Returns::Form;
    }
    // `PadTo(kNVSEOpcodeStart)`: xNVSE's padding command.
    let empty = guest_str(e, b"");
    let padding_help = guest_str(e, b"command used for padding");
    for opcode in exe.max_opcode..FIRST_OPCODE {
        Record {
            long_name: empty,
            short_name: empty,
            opcode,
            help: padding_help,
            execute: DEFAULT_EXECUTE,
            parse: DEFAULT_PARSE,
            ..Record::default()
        }
        .write(e, TABLE_AT + (opcode - ScriptTables::FIRST_OPCODE) * RECORD);
    }
    // xNVSE's commands, in opcode order.
    let built = crate::commands::all();
    for (i, (name, short)) in NAMES.iter().enumerate() {
        let opcode = FIRST_OPCODE + i as u32;
        let at = TABLE_AT + (opcode - ScriptTables::FIRST_OPCODE) * RECORD;
        let record = if let Some(cmd) = built.iter().find(|c| c.opcode == opcode) {
            assert_eq!(
                (cmd.name, cmd.short),
                (*name, *short),
                "{opcode:04x}: a command's names differ from xNVSE's"
            );
            e.register(execute_address(opcode), cmd.execute);
            let eval = cmd.eval.map_or(0, |f| {
                e.register(eval_address(opcode), f);
                eval_address(opcode)
            });
            returns[(opcode - ScriptTables::FIRST_OPCODE) as usize] = cmd.returns;
            Record {
                long_name: guest_str(e, cmd.name.as_bytes()),
                short_name: guest_str(e, cmd.short.as_bytes()),
                opcode,
                help: guest_str(e, cmd.help.as_bytes()),
                needs_ref: u16::from(cmd.needs_ref),
                param_count: cmd.params.len() as u16,
                params: guest_params(e, cmd.params),
                execute: execute_address(opcode),
                parse: parse_word(name),
                eval,
                // `DEFINE_CMD_ALT_COND_ANY` (xNVSE) marks a condition-capable command with 1.
                flags: u32::from(cmd.eval.is_some()),
            }
        } else if let Some(console) = name
            .strip_prefix("con_")
            .and_then(|n| console_record(e, n.as_bytes()))
        {
            // `ImportConsoleCommand` (xNVSE): the console record with the new long name, no
            // short name and the script opcode.
            Record {
                long_name: guest_str(e, name.as_bytes()),
                short_name: empty,
                opcode,
                ..Record::read(e, console)
            }
        } else {
            Record {
                long_name: guest_str(e, name.as_bytes()),
                short_name: guest_str(e, short.as_bytes()),
                opcode,
                help: empty,
                parse: parse_word(name),
                ..Record::default()
            }
        };
        record.write(e, at);
    }
    // The record past the end holds the end opcode the constructor checks.
    Record {
        opcode: max_opcode,
        ..Record::default()
    }
    .write(e, TABLE_AT + count * RECORD);
    e.ext::<Nvse>().table.returns = returns;
    e.set_script_tables(ScriptTables {
        start: TABLE_AT,
        end: TABLE_AT + count * RECORD,
        max_opcode,
    });
}

/// One past the highest opcode the table has room for: the records from `0x1000` below it and
/// the one past the end fill [`TABLE_ROOM`] (ITR's commands, the highest of the plugins VNV
/// installs, end at `0x410E`).
pub const ROOM_END: u32 = ScriptTables::FIRST_OPCODE + TABLE_ROOM / RECORD - 1;

/// Grows the table for a plugin built in, to `max_opcode` (one past its highest opcode), as
/// xNVSE's `PluginManager::SetOpcodeBase` pads it up to a plugin's base (`CommandTable::PadTo`
/// (xNVSE)) with one difference: the new slots are empty (no names, no parameters, no handler),
/// not xNVSE's padding command, so a call to an opcode no built-in fills fails, as it does in
/// the game without the plugin that owns it (ADR-0016 §2). The record past the end moves with
/// the end and the engine's [`ScriptTables`] with it. A table already as long stays as it is.
pub fn grow_to(e: &mut Engine, max_opcode: u32) {
    let t = e.script_tables();
    assert_eq!(
        t.start, TABLE_AT,
        "xNVSE's table is built before a plugin's"
    );
    assert!(
        max_opcode <= ROOM_END,
        "{max_opcode:04x}: past the table's room"
    );
    if max_opcode <= t.max_opcode {
        return;
    }
    // The room is mapped already (`install`).
    let count = max_opcode - ScriptTables::FIRST_OPCODE;
    let empty = guest_str(e, b"");
    for opcode in t.max_opcode..max_opcode {
        Record {
            long_name: empty,
            short_name: empty,
            opcode,
            help: empty,
            parse: DEFAULT_PARSE,
            ..Record::default()
        }
        .write(e, TABLE_AT + (opcode - ScriptTables::FIRST_OPCODE) * RECORD);
    }
    Record {
        opcode: max_opcode,
        ..Record::default()
    }
    .write(e, TABLE_AT + count * RECORD);
    e.ext::<Nvse>()
        .table
        .returns
        .resize(count as usize, Returns::Default);
    e.set_script_tables(ScriptTables {
        start: TABLE_AT,
        end: TABLE_AT + count * RECORD,
        max_opcode,
    });
}

/// A plugin command's record as the plugin hands it to xNVSE (`PluginManager::RegisterCommand`
/// and `RegisterTypedCommand` (xNVSE)): its `CommandInfo` fields, guest addresses for the
/// parameter list and the routines, and the return type it registers with. xNVSE gives it an
/// empty help text and short name where the plugin has none, and its default parser.
#[derive(Clone, Copy, Debug)]
pub struct PluginRecord<'a> {
    pub name: &'a str,
    /// Empty for none.
    pub short: &'a str,
    pub needs_ref: bool,
    pub param_count: u16,
    /// The parameter list (0: none).
    pub params: u32,
    /// The execute (0: none yet, so a call fails).
    pub execute: u32,
    /// The condition eval (0: none).
    pub eval: u32,
    pub flags: u32,
    /// The plugin asks for xNVSE's expression parser (`Cmd_Expression_Plugin_Parse`), which
    /// xNVSE gives as `Cmd_Expression_Parse`: the record gets [`EXPRESSION_PARSE`], the parse word of
    /// xNVSE's expression compiler ([`crate::compiler`]).
    pub expression: bool,
    pub returns: Returns,
}

/// Writes a plugin command's record at `opcode` in the grown table (`CommandTable::Add` (xNVSE),
/// at the plugin's current opcode) and its return type beside it. As xNVSE does, whatever a
/// plugin registers at `0x22C0` (JIP LN NVSE's `ar_Cat`, whose names xNVSE's own `ar_Cat` has)
/// is renamed `JIP_ar_Cat`, short name `JIP_pinto_Cat`; a record written with no name (a plugin
/// that answers "not installed" keeps its unbuilt commands' names out of the table) stays so.
pub fn add_plugin_record(e: &mut Engine, opcode: u32, r: &PluginRecord) {
    let t = e.script_tables();
    assert!(
        (crate::names::PLUGIN_OPCODES..t.max_opcode).contains(&opcode),
        "{opcode:04x}: not a plugin opcode of the table"
    );
    let (name, short) = if opcode == 0x22c0 && !r.name.is_empty() {
        ("JIP_ar_Cat", "JIP_pinto_Cat")
    } else {
        (r.name, r.short)
    };
    Record {
        long_name: guest_str(e, name.as_bytes()),
        short_name: guest_str(e, short.as_bytes()),
        opcode,
        help: guest_str(e, b""),
        needs_ref: u16::from(r.needs_ref),
        param_count: r.param_count,
        params: r.params,
        execute: r.execute,
        parse: if r.expression {
            EXPRESSION_PARSE
        } else {
            DEFAULT_PARSE
        },
        eval: r.eval,
        flags: r.flags,
    }
    .write(e, t.record(opcode));
    e.ext::<Nvse>().table.returns[(opcode - ScriptTables::FIRST_OPCODE) as usize] = r.returns;
}

/// The return type of `opcode` (xNVSE's `GetReturnType`): `Default` outside the table.
pub fn return_type(e: &Engine, opcode: u32) -> Returns {
    e.ext_get::<Nvse>()
        .and_then(|n| {
            n.table
                .returns
                .get(opcode.wrapping_sub(ScriptTables::FIRST_OPCODE) as usize)
                .copied()
        })
        .unwrap_or_default()
}

/// The record of `opcode` in the current table, or 0 past its ends (xNVSE's `GetByOpcode`).
pub fn record_of(e: &Engine, opcode: u32) -> u32 {
    let t = e.script_tables();
    if (ScriptTables::FIRST_OPCODE..t.max_opcode).contains(&opcode) {
        t.record(opcode)
    } else {
        0
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// An engine with a stand-in of the exe's script and console tables: record `i` named
    /// `V<i>` with its opcode and an execute word, console records named `C<i>`, `SetVel` at
    /// console index 3.
    pub(crate) fn engine_with_exe_tables() -> Engine {
        let mut e = Engine::new();
        let exe = ScriptTables::EXE;
        e.map(0x0118_e000, 0x9000);
        e.map(0x0100_0000, 0x1_0000);
        let mut name_at = 0x0100_0000;
        let mut name = |e: &mut Engine, s: &str| {
            let at = name_at;
            for (i, b) in s.bytes().enumerate() {
                e.mem.set_u8(at + i as u32, b);
            }
            e.mem.set_u8(at + s.len() as u32, 0);
            name_at += s.len() as u32 + 1;
            at
        };
        for i in 0..0x280 {
            let at = exe.start + i * RECORD;
            let n = name(&mut e, &format!("V{i}"));
            e.mem.set_u32(at, n);
            e.mem.set_u32(at + 8, 0x1000 + i);
            e.mem.set_u32(at + 0x18, 0x0050_0000 + i);
        }
        // The record past the exe's table, as the constructor reads it.
        e.mem.set_u32(exe.end + 8, 0x1280);
        for i in 0..CONSOLE_COUNT {
            let at = CONSOLE_TABLE + i * RECORD;
            let text = if i == 3 {
                "SetVel".to_string()
            } else {
                format!("C{i}")
            };
            let n = name(&mut e, &text);
            e.mem.set_u32(at, n);
            e.mem.set_u32(at + 4, n);
            e.mem.set_u32(at + 8, 0x100 + i);
            e.mem.set_u16(at + 0x10, 1);
            e.mem.set_u32(at + 0x18, 0x0060_0000 + i);
        }
        e
    }

    /// The exe's records are copied unchanged; the table and its descriptor move; padding fills
    /// `0x1280..0x1400` as xNVSE's padding command.
    #[test]
    fn the_table_holds_the_exes_records_then_padding_then_xnvses() {
        let mut e = engine_with_exe_tables();
        assert_eq!(e.script_tables(), ScriptTables::EXE);
        install(&mut e);
        let t = e.script_tables();
        assert_eq!(t.start, TABLE_AT);
        assert_eq!(t.max_opcode, 0x1400 + 654);
        assert_eq!(t.end, TABLE_AT + (0x400 + 654) * RECORD);
        for i in [0u32, 0x59, 0x27f] {
            let copy = Record::read(&e, t.record(0x1000 + i));
            let exe = Record::read(&e, ScriptTables::EXE.record(0x1000 + i));
            assert_eq!(copy, exe, "{i:x}");
        }
        for opcode in [0x1280, 0x13ff] {
            let r = Record::read(&e, t.record(opcode));
            assert_eq!(
                (r.opcode, r.execute, r.parse),
                (opcode, DEFAULT_EXECUTE, DEFAULT_PARSE)
            );
            assert_eq!(guest_text(&e, r.long_name), b"");
            assert_eq!(guest_text(&e, r.help), b"command used for padding");
            assert_eq!(e.call(r.execute, &[0; 8]).eax, 1);
        }
        // The record past the end answers the constructor's check.
        assert_eq!(e.mem.u32(t.end + 8), t.max_opcode);
        assert_eq!(record_of(&e, t.max_opcode), 0);
        assert_eq!(record_of(&e, 0x1400), t.record(0x1400));
    }

    /// How many of xNVSE's 654 commands are in: the built ones, each at an opcode of its own, and
    /// the console commands xNVSE re-exports, which are copies of the console's records
    /// ([`every_console_re_export_is_its_console_record`]). README's count, in its system table,
    /// is their sum: change them together.
    #[test]
    fn the_count_of_built_commands() {
        let built = crate::commands::all();
        let mut opcodes: Vec<u32> = built.iter().map(|c| c.opcode).collect();
        opcodes.sort_unstable();
        opcodes.dedup();
        assert_eq!(opcodes.len(), built.len(), "two commands at one opcode");
        let console = NAMES.iter().filter(|(n, _)| n.starts_with("con_")).count();
        assert_eq!((built.len(), console), (630, 20));
        assert_eq!(built.len() + console, 650);
    }

    /// Every record of xNVSE's range carries xNVSE's parse word: each of the 109 commands of
    /// [`PARSE_WORDS`] its own, every other command (built or not, console copies aside) the
    /// default one. Each listed name is one of xNVSE's commands, once.
    #[test]
    fn every_record_has_xnvses_parse_word() {
        let mut e = engine_with_exe_tables();
        install(&mut e);
        let t = e.script_tables();
        assert_eq!(PARSE_WORDS.len(), 109);
        for (name, _) in PARSE_WORDS {
            assert_eq!(
                NAMES.iter().filter(|(n, _)| n == name).count(),
                1,
                "{name} is not one of xNVSE's commands"
            );
        }
        let kinds = |w: u32| PARSE_WORDS.iter().filter(|p| p.1 == w).count();
        assert_eq!(
            PARSE_ROUTINES.map(|(w, _)| kinds(w)),
            [98, 1, 3, 1, 4, 1, 1],
            "expression, let, begin loop, loop, null, function, call"
        );
        for (i, (name, _)) in NAMES.iter().enumerate() {
            if name.starts_with("con_") {
                continue;
            }
            let r = Record::read(&e, t.record(0x1400 + i as u32));
            let want = PARSE_WORDS
                .iter()
                .find(|p| p.0 == *name)
                .map_or(DEFAULT_PARSE, |p| p.1);
            assert_eq!(r.parse, want, "{name}");
        }
        assert_eq!(Record::read(&e, t.record(0x1539)).parse, LET_PARSE);
        assert_eq!(Record::read(&e, t.record(0x1646)).parse, EXPRESSION_PARSE);
        assert_eq!(Record::read(&e, t.record(0x1670)).parse, BEGIN_LOOP_PARSE);
    }

    fn exe_parse(e: &mut Engine, a: &[u32]) -> Ret {
        e.mem.set_u32(0x0100_f000, a[0]);
        e.mem.set_u32(0x0100_f004, a[1]);
        e.mem.set_u32(0x0100_f008, a[2]);
        e.mem.set_u32(0x0100_f00c, a[3]);
        Ret {
            eax: 0x5b,
            ..Ret::default()
        }
    }

    /// The default parse word calls the exe's default parser with the same four words and
    /// answers what it answers; each of the seven other kinds names its compiler routine (the
    /// compiler's own tests run them): `continue`'s takes nothing and writes nothing.
    #[test]
    fn the_parse_words_name_the_default_parser_and_the_compiler() {
        let mut e = engine_with_exe_tables();
        e.register(EXE_DEFAULT_PARSE, exe_parse);
        install(&mut e);
        let t = e.script_tables();
        let words = [3, 0x0100_2000, 0x0100_3000, 0x0100_4000];
        assert_eq!(e.call(DEFAULT_PARSE, &words).eax, 0x5b);
        for (i, w) in words.iter().enumerate() {
            assert_eq!(e.mem.u32(0x0100_f000 + 4 * i as u32), *w);
        }
        // eval, Let, while, loop, Function, Call, then continue.
        for (opcode, word) in [
            (0x153a, EXPRESSION_PARSE),
            (0x1539, LET_PARSE),
            (0x153b, BEGIN_LOOP_PARSE),
            (0x153c, LOOP_PARSE),
            (0x1544, FUNCTION_PARSE),
            (0x1545, CALL_PARSE),
        ]
        .into_iter()
        .chain(std::iter::once((
            0x1400 + NAMES.iter().position(|n| n.0 == "continue").unwrap() as u32,
            NULL_PARSE,
        ))) {
            assert_eq!(
                Record::read(&e, t.record(opcode)).parse,
                word,
                "{opcode:04x}"
            );
        }
        e.mem.write(0x0100_f000, &[0; 16]);
        assert_eq!(e.call(NULL_PARSE, &words).eax, 1);
        assert_eq!(e.mem.bytes(0x0100_f000, 16), [0; 16]);
    }

    /// Every xNVSE opcode has its names; a console re-export copies the console record with the
    /// script opcode, the `con_` name and no short name.
    #[test]
    fn xnvse_records_have_their_names_and_the_console_re_exports() {
        let mut e = engine_with_exe_tables();
        install(&mut e);
        let t = e.script_tables();
        for (i, (name, short)) in NAMES.iter().enumerate() {
            let r = Record::read(&e, t.record(0x1400 + i as u32));
            assert_eq!(r.opcode, 0x1400 + i as u32);
            assert_eq!(guest_text(&e, r.long_name), name.as_bytes());
            if !name.starts_with("con_") {
                assert_eq!(guest_text(&e, r.short_name), short.as_bytes());
            }
        }
        let set_vel = NAMES.iter().position(|(n, _)| *n == "con_SetVel").unwrap() as u32;
        let r = Record::read(&e, t.record(0x1400 + set_vel));
        let console = Record::read(&e, CONSOLE_TABLE + 3 * RECORD);
        assert_eq!(
            r,
            Record {
                long_name: r.long_name,
                short_name: guest_str(&mut e, b""),
                opcode: 0x1400 + set_vel,
                ..console
            }
        );
        assert_eq!(r.execute, 0x0060_0003);
    }

    /// Each of the 20 console re-exports is its console record under the script opcode, with the
    /// `con_` name and no short name (`ImportConsoleCommand`): the console's handler, parser and
    /// parameters, so a script runs it as the console does.
    #[test]
    fn every_console_re_export_is_its_console_record() {
        let mut e = engine_with_exe_tables();
        // The helper names record 3 "SetVel": rename it, so the names below are found on theirs.
        let other = guest_str(&mut e, b"C3");
        e.mem.set_u32(CONSOLE_TABLE + 3 * RECORD, other);
        let exports: Vec<(usize, &str)> = NAMES
            .iter()
            .enumerate()
            .filter_map(|(i, (n, _))| n.strip_prefix("con_").map(|c| (i, c)))
            .collect();
        assert_eq!(exports.len(), 20);
        // Each name on a console record of its own, its handler and parameters told apart.
        for (k, (_, name)) in exports.iter().enumerate() {
            let at = CONSOLE_TABLE + (10 + k as u32) * RECORD;
            let text = guest_str(&mut e, name.as_bytes());
            e.mem.set_u32(at, text);
            e.mem.set_u32(at + 0x14, 0x0070_0000 + k as u32);
            e.mem.set_u32(at + 0x18, 0x0060_1000 + k as u32);
            e.mem.set_u32(at + 0x1c, 0x0060_2000 + k as u32);
        }
        install(&mut e);
        let t = e.script_tables();
        for (k, (i, name)) in exports.iter().enumerate() {
            let opcode = 0x1400 + *i as u32;
            let r = Record::read(&e, t.record(opcode));
            let console = Record::read(&e, CONSOLE_TABLE + (10 + k as u32) * RECORD);
            assert_eq!(
                guest_text(&e, r.long_name),
                format!("con_{name}").as_bytes()
            );
            assert_eq!(guest_text(&e, r.short_name), b"");
            assert_eq!(
                r,
                Record {
                    long_name: r.long_name,
                    short_name: r.short_name,
                    opcode,
                    ..console
                },
                "{name}"
            );
        }
    }

    /// Return types: the 13 vanilla form commands, and numbers elsewhere.
    #[test]
    fn vanilla_form_commands_return_forms() {
        let mut e = engine_with_exe_tables();
        install(&mut e);
        assert_eq!(return_type(&e, 0x1025), Returns::Form);
        assert_eq!(return_type(&e, 0x1026), Returns::Default);
        assert_eq!(return_type(&e, 0x9000), Returns::Default);
    }

    /// A plugin's opcodes grow the table: the new slots are empty records (their opcode, empty
    /// names and help, the default parser, no handler, no parameters), the record past the end
    /// and the engine's descriptor move, xNVSE's records stay, and a shorter grow changes nothing.
    #[test]
    fn a_plugin_grows_the_table_with_empty_slots() {
        let mut e = engine_with_exe_tables();
        install(&mut e);
        let before = Record::read(&e, e.script_tables().record(0x15df));
        grow_to(&mut e, 0x2943);
        let t = e.script_tables();
        assert_eq!(
            (t.start, t.end, t.max_opcode),
            (TABLE_AT, TABLE_AT + 0x1943 * RECORD, 0x2943)
        );
        assert_eq!(e.mem.u32(t.end + 8), 0x2943, "the record past the end");
        assert_eq!(Record::read(&e, t.record(0x15df)), before);
        let empty = guest_str(&mut e, b"");
        for opcode in [0x168e, 0x1fff, 0x2132, 0x2942] {
            assert_eq!(
                Record::read(&e, t.record(opcode)),
                Record {
                    long_name: empty,
                    short_name: empty,
                    opcode,
                    help: empty,
                    parse: DEFAULT_PARSE,
                    ..Record::default()
                },
                "{opcode:04x}"
            );
            assert_eq!(return_type(&e, opcode), Returns::Default);
        }
        assert_eq!(record_of(&e, 0x2942), t.record(0x2942));
        assert_eq!(record_of(&e, 0x2943), 0);
        grow_to(&mut e, 0x2300);
        assert_eq!(e.script_tables(), t);
        // The table's room ends where its strings begin: the record past the end fits, one
        // more would not.
        grow_to(&mut e, ROOM_END);
        let end = e.script_tables().end;
        assert!(end + RECORD <= DATA_AT && end + 2 * RECORD > DATA_AT);
        assert_eq!(e.mem.u32(end + 8), ROOM_END);
        let past = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            grow_to(&mut e, ROOM_END + 1)
        }));
        assert!(past.is_err());
    }

    /// The whole room is mapped: the words the condition code (`00681600`: the eval at `+0x20`,
    /// the reference byte at `+0x10`) and a condition's `InitItem` (`006819e0`: the eval, then the
    /// name at `+0`) read for a function past the table's end, up to the room's last, are zeros,
    /// as are the parameters `00682320` reads (`+0x12`, `+0x14`).
    #[test]
    fn records_past_the_end_read_as_none() {
        let mut e = engine_with_exe_tables();
        install(&mut e);
        let t = e.script_tables();
        for opcode in [0x2100, 0x3100, 0x3c93, 0x4000, ROOM_END - 1] {
            let at = t.record(opcode);
            assert_eq!(
                (
                    e.mem.u32(at),
                    e.mem.u8(at + 0x10),
                    e.mem.u16(at + 0x12),
                    e.mem.u32(at + 0x14),
                    e.mem.u32(at + 0x20)
                ),
                (0, 0, 0, 0, 0),
                "{opcode:04x}"
            );
        }
    }

    /// A plugin's record holds what the plugin registers, with the default parser and an empty
    /// help text, and its return type beside it; xNVSE renames what is registered at `0x22C0`
    /// (not an unnamed record). Only opcodes from `0x2000` to the table's end take one.
    #[test]
    fn a_plugin_record_is_written_as_registered() {
        let mut e = engine_with_exe_tables();
        install(&mut e);
        grow_to(&mut e, 0x2600);
        let record = |execute| PluginRecord {
            name: "SomeCommand",
            short: "Some",
            needs_ref: true,
            param_count: 2,
            params: 0x7f12_3450,
            execute,
            eval: 0x7ec0_1234,
            flags: 1,
            expression: false,
            returns: Returns::Array,
        };
        add_plugin_record(&mut e, 0x2201, &record(0));
        let t = e.script_tables();
        let r = Record::read(&e, t.record(0x2201));
        assert_eq!(
            r,
            Record {
                long_name: r.long_name,
                short_name: r.short_name,
                opcode: 0x2201,
                help: guest_str(&mut e, b""),
                needs_ref: 1,
                param_count: 2,
                params: 0x7f12_3450,
                execute: 0,
                parse: DEFAULT_PARSE,
                eval: 0x7ec0_1234,
                flags: 1,
            }
        );
        assert_eq!(guest_text(&e, r.long_name), b"SomeCommand");
        assert_eq!(guest_text(&e, r.short_name), b"Some");
        assert_eq!(return_type(&e, 0x2201), Returns::Array);
        add_plugin_record(
            &mut e,
            0x22c0,
            &PluginRecord {
                name: "",
                short: "",
                ..record(0)
            },
        );
        let r = Record::read(&e, t.record(0x22c0));
        assert_eq!(guest_text(&e, r.long_name), b"", "unnamed: no rename");
        add_plugin_record(&mut e, 0x22c0, &record(0x7e80_0004));
        let r = Record::read(&e, t.record(0x22c0));
        assert_eq!(guest_text(&e, r.long_name), b"JIP_ar_Cat");
        assert_eq!(guest_text(&e, r.short_name), b"JIP_pinto_Cat");
        assert_eq!(r.execute, 0x7e80_0004);
        for opcode in [0x1fff, 0x2600] {
            let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                add_plugin_record(&mut e, opcode, &record(0))
            }));
            assert!(refused.is_err(), "{opcode:04x}");
        }
    }

    /// Strings are written once.
    #[test]
    fn guest_strings_are_shared() {
        let mut e = engine_with_exe_tables();
        let a = guest_str(&mut e, b"abc");
        assert_eq!(guest_str(&mut e, b"abc"), a);
        assert_eq!(guest_text(&e, a), b"abc");
        assert_ne!(guest_str(&mut e, b"abd"), a);
    }
}
