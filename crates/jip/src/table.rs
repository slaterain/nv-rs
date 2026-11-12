//! JIP LN NVSE's command slots in the script command table (ADR-0016 §2, plan step 3.1): the
//! 1,150 slots JIP registers with xNVSE (`NVSEPlugin_Load` in `jip_nvse.cpp` (JIP LN NVSE)), at
//! their real opcodes: `0x2200..=0x233A`, the former lutana plugin's block (299 commands and 16
//! empty slots), and `0x2600..=0x2942`, JIP's own (819 commands and 16 empty slots;
//! `SetLinkedReference` is in both, at `0x2281` and `0x2634`). xNVSE's table grows to
//! `0x2943` for them ([`nvse::table::grow_to`]); the slots between its last command and
//! `0x2200`, and between the two blocks, stay empty, so the other plugins' bases (Stewie's Tweaks
//! at `0x2132`, JohnnyGuitar at `0x3100`, ...) are free for them.
//!
//! Each slot's record is what JIP registers ([`crate::slots`]): its names, whether it needs a
//! reference, its parameter list (written once per list into the built-ins' data, shared as
//! JIP's records share them), its condition flag and its return type. A command not built yet
//! has no handler and no eval: a script line that calls it fails and the script stops for that
//! run, as it does without the DLL; a condition that names it is false. While JIP answers "not
//! installed" ([`crate::ANSWERS_INSTALLED`]) an unbuilt command has no name either, so no name
//! finds it (the console, `help`), as without the DLL. Built commands get their handlers and
//! names through [`set_handlers`]. An empty slot holds `kEmptyCommand` (`internal/jip_core.cpp`):
//! named `NONE`, three optional integers listed but none counted, and the exe's `005d4a40`
//! (which answers true) as its execute once JIP answers "installed".

use crate::Jip;
use engine_core::prelude::*;
use nvse::table::{add_plugin_record, execute_address, Param, PluginRecord, Returns};
use std::collections::HashMap;

/// The first and last opcodes of JIP's slots, and the slots of each block.
pub const FIRST_OPCODE: u32 = 0x2200;
pub const LAST_OPCODE: u32 = 0x2942;
pub const LN_BLOCK: std::ops::RangeInclusive<u32> = 0x2200..=0x233a;
pub const JIP_BLOCK: std::ops::RangeInclusive<u32> = 0x2600..=0x2942;

/// `kEmptyCommand`'s execute: the exe's `005d4a40`, which only answers true (`MOV AL,1; RET`).
pub const EMPTY_EXECUTE: u32 = 0x005d_4a40;

/// A parameter list of `nvse/ParamInfos.h` (JIP LN NVSE): its name there and its parameters.
#[derive(Debug)]
pub struct Params {
    pub name: &'static str,
    pub list: &'static [Param],
}

impl Params {
    pub const fn new(name: &'static str, list: &'static [Param]) -> Params {
        Params { name, list }
    }
}

/// What a slot's `DEFINE_*` line makes it (`nvse/CommandTable.h` (JIP LN NVSE)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `DEFINE_COMMAND_PLUGIN`, `DEFINE_COMMAND_ALT_PLUGIN`: an execute.
    Command,
    /// `DEFINE_COMMAND_PLUGIN_EXP`, `DEFINE_COMMAND_ALT_PLUGIN_EXP`: an execute, and xNVSE's
    /// expression parser for its arguments.
    Expression,
    /// `DEFINE_CMD_COND_PLUGIN`, `DEFINE_CMD_ALT_COND_PLUGIN`: an execute and a condition eval
    /// (flags 1).
    Condition,
    /// `DEFINE_CMD_COND_ONLY`: a condition eval only, needing a reference (flags 1).
    ConditionOnly,
    /// `REG_EMPTY`: `kEmptyCommand`.
    Empty,
}

impl Kind {
    /// The record's flags: 1 for a command a condition can use.
    pub fn flags(self) -> u32 {
        u32::from(matches!(self, Kind::Condition | Kind::ConditionOnly))
    }
}

/// One slot as JIP registers it.
#[derive(Debug)]
pub struct Slot {
    pub opcode: u32,
    /// The long name: `NONE` for an empty slot.
    pub name: &'static str,
    /// The alias (the record's short name); empty for none.
    pub alias: &'static str,
    pub needs_ref: bool,
    pub params: &'static Params,
    pub returns: Returns,
    pub kind: Kind,
    /// The source file of its `DEFINE_*` line, without `.h` (`functions_jip/`, `functions_ln/`
    /// or `internal/`); empty for an empty slot. Commands are built in modules named after it.
    pub file: &'static str,
}

impl Slot {
    /// The record's parameter count: the list's length (none for an empty slot, whose list
    /// pointer is set all the same).
    pub fn param_count(&self) -> u16 {
        if self.kind == Kind::Empty {
            0
        } else {
            self.params.list.len() as u16
        }
    }
}

/// The return types, as the registration macros give them (`REG_CMD`, `REG_CMD_FRM`,
/// `REG_CMD_STR`, `REG_CMD_ARR`, `REG_CMD_AMB` (JIP LN NVSE)).
pub const NUM: Returns = Returns::Default;
pub const FORM: Returns = Returns::Form;
pub const STR: Returns = Returns::String;
pub const ARR: Returns = Returns::Array;
pub const AMB: Returns = Returns::Ambiguous;

/// A command's slot ([`crate::slots::SLOTS`]).
#[allow(clippy::too_many_arguments)] // One line of the generated table per slot.
pub const fn cmd(
    opcode: u32,
    name: &'static str,
    alias: &'static str,
    needs_ref: bool,
    params: &'static Params,
    returns: Returns,
    kind: Kind,
    file: &'static str,
) -> Slot {
    Slot {
        opcode,
        name,
        alias,
        needs_ref,
        params,
        returns,
        kind,
        file,
    }
}

/// An empty slot (`REG_EMPTY`), with `kEmptyCommand`'s parameter list.
pub const fn empty(opcode: u32, params: &'static Params) -> Slot {
    Slot {
        opcode,
        name: "NONE",
        alias: "",
        needs_ref: false,
        params,
        returns: NUM,
        kind: Kind::Empty,
        file: "",
    }
}

/// The slot of `opcode`, if JIP has one there.
pub fn slot(opcode: u32) -> Option<&'static Slot> {
    let at = if LN_BLOCK.contains(&opcode) {
        opcode - LN_BLOCK.start()
    } else if JIP_BLOCK.contains(&opcode) {
        opcode - JIP_BLOCK.start() + (LN_BLOCK.end() - LN_BLOCK.start() + 1)
    } else {
        return None;
    };
    crate::slots::SLOTS.get(at as usize)
}

/// What the table keeps on the engine: the parameter lists written so far, by name.
#[derive(Default)]
pub struct State {
    lists: HashMap<&'static str, u32>,
}

/// A parameter list in the built-ins' data (12 bytes a parameter: name, type, optional), written
/// once per list; 0 for `nullptr`.
fn guest_params(e: &mut Engine, params: &'static Params) -> u32 {
    if params.list.is_empty() {
        return 0;
    }
    if let Some(&at) = e.ext::<Jip>().table.lists.get(params.name) {
        return at;
    }
    let at = nvse::table::alloc(e, 12 * params.list.len() as u32);
    for (i, p) in params.list.iter().enumerate() {
        let name = nvse::table::guest_str(e, p.name.as_bytes());
        let slot = at + 12 * i as u32;
        e.mem.set_u32(slot, name);
        e.mem.set_u32(slot + 4, p.kind);
        e.mem.set_u32(slot + 8, u32::from(p.optional));
    }
    e.ext::<Jip>().table.lists.insert(params.name, at);
    at
}

/// A parameter list's address in the built-ins' data, written once (JIP's lists that are not
/// a slot's, such as the ones its versions of xNVSE's commands take).
pub(crate) fn params_address(e: &mut Engine, params: &'static Params) -> u32 {
    guest_params(e, params)
}

/// A slot's record as JIP registers it, with the handler words given; its names only when
/// `named` (an unbuilt command of a JIP that answers "not installed" has none, so no name finds
/// it, as none would without the DLL).
fn plugin_record(
    e: &mut Engine,
    slot: &'static Slot,
    named: bool,
    execute: u32,
    eval: u32,
) -> PluginRecord<'static> {
    let (name, short) = if named {
        (slot.name, slot.alias)
    } else {
        ("", "")
    };
    PluginRecord {
        name,
        short,
        needs_ref: slot.needs_ref,
        param_count: slot.param_count(),
        params: guest_params(e, slot.params),
        execute,
        eval,
        flags: slot.kind.flags(),
        expression: slot.kind == Kind::Expression,
        returns: slot.returns,
    }
}

/// Grows xNVSE's table to JIP's last opcode and writes JIP's 1,150 records, with no handlers;
/// answering "installed", they are named and the empty slots get `kEmptyCommand`'s execute.
pub fn install(e: &mut Engine, answers_installed: bool) {
    nvse::table::grow_to(e, LAST_OPCODE + 1);
    for slot in crate::slots::SLOTS.iter() {
        let execute = if slot.kind == Kind::Empty && answers_installed {
            EMPTY_EXECUTE
        } else {
            0
        };
        let record = plugin_record(e, slot, answers_installed, execute, 0);
        add_plugin_record(e, slot.opcode, &record);
    }
}

/// Gives a built JIP command its handlers, written into its record with its names (a built
/// command answers whatever JIP does): its execute at its place in the extenders' code
/// ([`execute_address`], as xNVSE's commands have theirs) and, for a command a condition can use,
/// its eval ([`nvse::table::eval_address`]). A condition-only command has no execute.
pub fn set_handlers(e: &mut Engine, opcode: u32, execute: Option<AbiFn>, eval: Option<AbiFn>) {
    let slot = slot(opcode).unwrap_or_else(|| panic!("{opcode:04x}: not a JIP slot"));
    let fits = match slot.kind {
        Kind::Command | Kind::Expression => execute.is_some() && eval.is_none(),
        Kind::Condition => execute.is_some() && eval.is_some(),
        Kind::ConditionOnly => execute.is_none() && eval.is_some(),
        Kind::Empty => false,
    };
    assert!(fits, "{opcode:04x}: handlers that do not match its kind");
    let mut words = (0, 0);
    if let Some(f) = execute {
        e.register(execute_address(opcode), f);
        words.0 = execute_address(opcode);
    }
    if let Some(f) = eval {
        e.register(nvse::table::eval_address(opcode), f);
        words.1 = nvse::table::eval_address(opcode);
    }
    let record = plugin_record(e, slot, true, words.0, words.1);
    add_plugin_record(e, opcode, &record);
}

#[cfg(test)]
#[path = "table_tests.rs"]
mod tests;
