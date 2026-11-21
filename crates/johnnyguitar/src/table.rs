//! JohnnyGuitar NVSE's command slots in the script command table: the 408 slots it registers with
//! xNVSE (`JohnnyCommands::Init`, `JG/internal/JG/JohnnyCommands.cpp`) from its opcode base
//! `0x3100` (`JohnnyPluginData::JG_OPCODE_BASE`) to `0x3297`, none empty. xNVSE's table grows to
//! `0x3298` for them ([`nvse::table::grow_to`]).
//!
//! Each slot's record is what JohnnyGuitar registers ([`crate::slots`]): its names, whether it
//! needs a reference, its parameter list (written once per list into the built-ins' data), its
//! condition flag and its return type. A command not built yet has no handler and no eval: a script
//! line that calls it fails and the script stops for that run, as it does without the DLL; a
//! condition that names it is false. While JohnnyGuitar answers "not installed" (the host's row,
//! [`crate::set_answers_installed`]) no slot has a name either, so no name finds it (the console,
//! `help`), as without the DLL. Built commands get their handlers and names through
//! [`set_handlers`].

use crate::JohnnyGuitar;
use engine_core::prelude::*;
use nvse::table::{add_plugin_record, execute_address, Param, PluginRecord, Returns};
use std::collections::HashMap;

/// The first and last opcodes of JohnnyGuitar's slots.
pub const FIRST_OPCODE: u32 = 0x3100;
pub const LAST_OPCODE: u32 = 0x3297;

/// A parameter list of `nvse/ParamInfos.h` (JohnnyGuitar NVSE): its name there and its parameters.
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

/// What a slot's `DEFINE_*` line makes it (`nvse/CommandTable.h` (JohnnyGuitar NVSE)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `DEFINE_COMMAND_PLUGIN`, `DEFINE_COMMAND_ALT_PLUGIN`: an execute.
    Command,
    /// `DEFINE_COMMAND_PLUGIN_EXP`: an execute, and xNVSE's expression parser for its arguments.
    Expression,
    /// `DEFINE_CMD_COND_PLUGIN`, `DEFINE_CMD_ALT_COND_PLUGIN`: an execute and a condition eval
    /// (flags 1).
    Condition,
}

impl Kind {
    /// The record's flags: 1 for a command a condition can use.
    pub fn flags(self) -> u32 {
        u32::from(self == Kind::Condition)
    }
}

/// One slot as JohnnyGuitar registers it.
#[derive(Debug)]
pub struct Slot {
    pub opcode: u32,
    pub name: &'static str,
    /// The alias (the record's short name); empty for none.
    pub alias: &'static str,
    pub needs_ref: bool,
    pub params: &'static Params,
    pub returns: Returns,
    pub kind: Kind,
    /// The source file of its `DEFINE_*` line (`JG/functions/`), without `.h`. Commands are built
    /// in modules named after it.
    pub file: &'static str,
}

/// The return types, as the registration macros give them (`REG_CMD`: the default;
/// `REG_TYPED_CMD(name, Form | String | Array | Ambiguous)`).
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

/// The slot of `opcode`, if JohnnyGuitar has one there.
pub fn slot(opcode: u32) -> Option<&'static Slot> {
    if !(FIRST_OPCODE..=LAST_OPCODE).contains(&opcode) {
        return None;
    }
    crate::slots::SLOTS.get((opcode - FIRST_OPCODE) as usize)
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
    if let Some(&at) = e.ext::<JohnnyGuitar>().table.lists.get(params.name) {
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
    e.ext::<JohnnyGuitar>().table.lists.insert(params.name, at);
    at
}

/// A slot's record as JohnnyGuitar registers it, with the handler words given; its names only
/// when `named`.
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
        param_count: slot.params.list.len() as u16,
        params: guest_params(e, slot.params),
        execute,
        eval,
        flags: slot.kind.flags(),
        expression: slot.kind == Kind::Expression,
        returns: slot.returns,
    }
}

/// Grows xNVSE's table to JohnnyGuitar's last opcode and writes its 408 records, with no
/// handlers, named (only while JohnnyGuitar answers "installed": [`crate::install`]).
pub fn install(e: &mut Engine) {
    nvse::table::grow_to(e, LAST_OPCODE + 1);
    for slot in crate::slots::SLOTS.iter() {
        let record = plugin_record(e, slot, true, 0, 0);
        add_plugin_record(e, slot.opcode, &record);
    }
}

/// Gives a built JohnnyGuitar command its handlers and names, written into its record, once
/// JohnnyGuitar answers "installed" (until then nothing: every opcode stays as without the DLL):
/// its execute at its place in the extenders' code ([`execute_address`]) and, for a command a
/// condition can use, its eval ([`nvse::table::eval_address`]).
pub fn set_handlers(e: &mut Engine, opcode: u32, execute: AbiFn, eval: Option<AbiFn>) {
    let slot = slot(opcode).unwrap_or_else(|| panic!("{opcode:04x}: not a JohnnyGuitar slot"));
    assert_eq!(
        eval.is_some(),
        slot.kind == Kind::Condition,
        "{opcode:04x}: handlers that do not match its kind"
    );
    if !e.ext::<JohnnyGuitar>().answers_installed {
        return;
    }
    e.register(execute_address(opcode), execute);
    let mut words = (execute_address(opcode), 0);
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
