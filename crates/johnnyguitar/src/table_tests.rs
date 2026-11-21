//! Tests of JohnnyGuitar's slots: pinned to the source's list and as they are written into the table.

use super::*;
use crate::slots::SLOTS;
use crate::test_support::*;

/// The rows of the committed list (`data/source-5.28.tsv`): tab-separated.
fn rows(text: &str) -> Vec<Vec<&str>> {
    text.lines()
        .map(|l| l.trim_end_matches('\r').split('\t').collect())
        .collect()
}

/// Every slot is the source's: opcode, return type, name, macro (the kind; the `ALT` ones and only
/// they have an alias), alias, needing a reference, parameter list, file.
#[test]
fn every_slot_is_the_sources() {
    let rows = rows(include_str!("../data/source-5.28.tsv"));
    assert_eq!(rows.len(), 408);
    assert_eq!(SLOTS.len(), 408);
    for (slot, row) in SLOTS.iter().zip(&rows) {
        let opcode = u32::from_str_radix(row[0], 16).unwrap();
        assert_eq!(slot.opcode, opcode);
        let returns = match row[1] {
            "Default" => NUM,
            "Form" => FORM,
            "String" => STR,
            "Array" => ARR,
            "Ambiguous" => AMB,
            other => panic!("{opcode:04x}: {other}"),
        };
        let kind = match row[3] {
            "COMMAND_PLUGIN" | "COMMAND_ALT_PLUGIN" => Kind::Command,
            "COMMAND_PLUGIN_EXP" => Kind::Expression,
            "CMD_COND_PLUGIN" | "CMD_ALT_COND_PLUGIN" => Kind::Condition,
            other => panic!("{opcode:04x}: {other}"),
        };
        assert_eq!(
            (
                slot.name,
                slot.returns,
                slot.kind,
                slot.alias,
                slot.needs_ref,
                slot.params.name,
                slot.file
            ),
            (row[2], returns, kind, row[4], row[5] == "1", row[6], row[7]),
            "{opcode:04x}"
        );
    }
    assert_eq!(SLOTS[0].opcode, FIRST_OPCODE);
    assert_eq!(SLOTS[407].opcode, LAST_OPCODE);
}

/// Installed: each slot's record has its names, its parameter count and list, its flags and its
/// return type; a built command has its execute (and a condition its eval), an unbuilt one none.
#[test]
fn installed_the_records_are_written() {
    let e = engine(true);
    let lerp = nvse::table::record_of(&e, 0x3157);
    assert_eq!(nvse::table::guest_text(&e, e.mem.u32(lerp)), b"Lerp");
    assert_ne!(e.mem.u32(lerp + 0x18), 0, "built: an execute");
    let get_editor_id = nvse::table::record_of(&e, 0x3114);
    assert_eq!(
        nvse::table::guest_text(&e, e.mem.u32(get_editor_id + 4)),
        b"GetEdID"
    );
    assert_eq!(nvse::table::return_type(&e, 0x3114), STR);
    // An unbuilt one: named, no handler (a line calling it fails, as without the DLL).
    let jump = slot(0x310f).unwrap();
    let record = nvse::table::record_of(&e, jump.opcode);
    assert_eq!(
        nvse::table::guest_text(&e, e.mem.u32(record)),
        jump.name.as_bytes()
    );
    assert_eq!(e.mem.u32(record + 0x18), 0);
}

/// Not installed: no record is written (the opcodes stay xNVSE's empty padding).
#[test]
fn not_installed_no_record() {
    let e = engine(false);
    assert!(e.script_tables().max_opcode <= FIRST_OPCODE);
}
