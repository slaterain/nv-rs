//! The added script variables, expectations from JIP's `ScriptVariableAction_Execute`,
//! `Cmd_RemoveAllAddedVariables_Execute` (`functions_jip/jip_fn_script.h`), `Script::AddVariable`
//! and `GetVariableAdded` (`internal/jip_core.cpp`), `Script::GetVariableByName`
//! (`nvse/GameScript.cpp`) and `ScriptLocals::GetVariable` (`nvse/GameAPI.cpp`).
//!
//! The owners: a quest (its script `+0x1C`, its event list `+0x5C`) and a reference with an
//! `ExtraScript` (script `+0xC`, event list `+0x10`); each script's variable list at `+0x4C`, its
//! last id at `+0x24`, each event list's variable list pointer at `+0xC`.

use super::*;
use crate::commands::test_support::{
    self as ts, allocations, extra_stand_ins, put_extra, r, s, set_ref, FORMS, HEAP, SCRIPT,
};

const QUEST: u32 = FORMS + 0x1000;
const QUEST2: u32 = FORMS + 0x1800;
const SCRIPT_A: u32 = FORMS + 0x2000;
const EVENTS_A: u32 = FORMS + 0x2100;
const EVENTS_A_LIST: u32 = FORMS + 0x2200;
const EVENTS_B: u32 = FORMS + 0x2300;
const EVENTS_B_LIST: u32 = FORMS + 0x2400;
const REFR: u32 = FORMS + 0x3000;
const XSCRIPT: u32 = FORMS + 0x3200;
const PLAIN_VT: u32 = FORMS + 0x4000;
const REF_VT: u32 = FORMS + 0x4400;
const FILE: u32 = FORMS + 0x5000;
const NOSCRIPT: u32 = FORMS + 0x6000;

const ADD: u32 = 0x2621;
const HAS: u32 = 0x268c;
const REMOVE: u32 = 0x268d;
const REMOVE_ALL: u32 = 0x2690;

/// Two quests sharing script A (ids 0x10A, 0x10B; event lists A and B), a reference (0x200) with
/// script A and event list A, a quest with no script; the calling script's mod 5.
fn engine() -> Engine {
    let mut e = ts::engine();
    extra_stand_ins(&mut e);
    e.mem.set_u32(REF_VT + 0xf0, 0x008d_0360);
    for (quest, id, events, list) in [
        (QUEST, 0x10a, EVENTS_A, EVENTS_A_LIST),
        (QUEST2, 0x10b, EVENTS_B, EVENTS_B_LIST),
    ] {
        ts::form(&mut e, quest, 0x47, id);
        e.mem.set_u32(quest, PLAIN_VT);
        e.mem.set_u32(quest + 0x1c, SCRIPT_A);
        e.mem.set_u32(quest + 0x5c, events);
        e.mem.set_u32(events, SCRIPT_A);
        e.mem.set_u32(events + 0xc, list);
    }
    ts::form(&mut e, SCRIPT_A, 0x11, 0x0300_0a00);
    e.mem.set_u32(SCRIPT_A + 0x24, 4);
    ts::form(&mut e, REFR, 0x3a, 0x200);
    e.mem.set_u32(REFR, REF_VT);
    put_extra(&mut e, REFR + 0x44, XSCRIPT, 0x0d, SCRIPT_A);
    e.mem.set_u32(XSCRIPT + 0x10, EVENTS_A);
    ts::form(&mut e, NOSCRIPT, 0x47, 0x300);
    e.mem.set_u32(NOSCRIPT, PLAIN_VT);
    set_mod(&mut e, 5);
    set_ref(&mut e, 1, QUEST);
    set_ref(&mut e, 2, QUEST2);
    set_ref(&mut e, 3, NOSCRIPT);
    e
}

/// The calling script's overriding mod: its one file's index (`+0x40C`).
fn set_mod(e: &mut Engine, index: u8) {
    e.mem.set_u32(SCRIPT + 0x10, FILE);
    e.mem.set_u8(FILE + 0x40c, index);
}

fn on(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    ts::call_on(e, opcode, this, args).1
}

/// The items of the list from `node`, in order (empty nodes passed over).
fn items(e: &Engine, mut node: u32) -> Vec<u32> {
    let mut out = Vec::new();
    while node != 0 {
        if e.mem.u32(node) != 0 {
            out.push(e.mem.u32(node));
        }
        node = e.mem.u32(node + 4);
    }
    out
}

/// The script's variables: id and name.
fn infos(e: &Engine) -> Vec<(u32, Vec<u8>)> {
    items(e, SCRIPT_A + 0x4c)
        .into_iter()
        .map(|i| {
            (
                e.mem.u32(i),
                nvse::table::guest_text(e, e.mem.u32(i + 0x18)),
            )
        })
        .collect()
}

/// An event list's variables: id and value.
fn vars(e: &Engine, list: u32) -> Vec<(u32, f64)> {
    items(e, list)
        .into_iter()
        .map(|v| (e.mem.u32(v), e.mem.f64(v + 8)))
        .collect()
}

fn saved_entry(e: &mut Engine, owner: u32, name: &[u8]) -> Option<ScriptVar> {
    e.ext::<Jip>()
        .saved
        .script_vars
        .get(&owner)
        .and_then(|v| v.get(&Name(name.to_vec())))
        .copied()
}

fn live_var(e: &mut Engine, owner: u32, name: &[u8]) -> Option<u32> {
    e.ext::<Jip>()
        .script_vars
        .live
        .get(&owner)
        .and_then(|v| v.get(&Name(name.to_vec())))
        .copied()
}

#[test]
fn add_makes_the_variable_in_the_script_and_the_event_list() {
    let mut e = engine();
    assert_eq!(on(&mut e, ADD, 0, &[s(b"Speed"), r(1)]), 1.0);
    // A VariableInfo (0x20), its name's buffer (6), a ScriptVar (0x18); both lists' first nodes
    // were empty (no node).
    assert_eq!(allocations(), [0x20, 6, 0x18]);
    let info = HEAP;
    assert_eq!(e.mem.u32(SCRIPT_A + 0x24), 5, "the script's next id");
    assert_eq!(infos(&e), [(5, b"Speed".to_vec())]);
    // Zeroed but for the id and the name.
    for at in [4u32, 8, 0xc, 0x10, 0x14] {
        assert_eq!(e.mem.u32(info + at), 0, "+{at:x}");
    }
    assert_eq!(e.mem.u16(info + 0x1c), 5, "the name's length");
    let var = HEAP + 0x28;
    assert_eq!(items(&e, EVENTS_A_LIST), [var]);
    assert_eq!(vars(&e, EVENTS_A_LIST), [(5, 0.0)]);
    assert_eq!(e.mem.u8(var + 0x10), 0, "a float variable");
    assert_eq!(
        saved_entry(&mut e, 0x10a, b"speed"),
        Some(ScriptVar {
            mod_index: 5,
            value: 0
        })
    );
    assert_eq!(live_var(&mut e, 0x10a, b"Speed"), Some(var));
    assert!(variable_added(&mut e, 0x0300_0a00, b"SPEED"));
    assert!(!variable_added(&mut e, 0x0300_0a00, b"Other"));
    assert!(!variable_added(&mut e, 0x0300_0a01, b"Speed"));
}

/// The owner's script already has the name JIP added: the same `VariableInfo`; its event list
/// already has the variable: the same `ScriptVar`; nothing allocated. Another name: first in both
/// lists (a node each).
#[test]
fn add_again_reuses_and_another_goes_first() {
    let mut e = engine();
    on(&mut e, ADD, 0, &[s(b"Speed"), r(1)]);
    let before = allocations().len();
    e.mem.set_f64(HEAP + 0x28 + 8, 3.5);
    assert_eq!(on(&mut e, ADD, 0, &[s(b"SPEED"), r(1)]), 1.0);
    assert_eq!(allocations().len(), before);
    assert_eq!(vars(&e, EVENTS_A_LIST), [(5, 3.5)]);
    assert_eq!(
        saved_entry(&mut e, 0x10a, b"speed").unwrap().value,
        3.5f64.to_bits()
    );
    assert_eq!(on(&mut e, ADD, 0, &[s(b"Count"), r(1)]), 1.0);
    assert_eq!(&allocations()[before..], [0x20, 6, 8, 0x18, 8]);
    assert_eq!(infos(&e), [(6, b"Count".to_vec()), (5, b"Speed".to_vec())]);
    assert_eq!(vars(&e, EVENTS_A_LIST), [(6, 0.0), (5, 3.5)]);
}

/// A second owner of the same script: the `VariableInfo` JIP added is used, its own event list
/// gets a `ScriptVar`, its own saved entry, with the mod calling now.
#[test]
fn add_for_a_second_owner_of_the_script() {
    let mut e = engine();
    on(&mut e, ADD, 0, &[s(b"Speed"), r(1)]);
    set_mod(&mut e, 9);
    let before = allocations().len();
    assert_eq!(on(&mut e, ADD, 0, &[s(b"Speed"), r(2)]), 1.0);
    assert_eq!(&allocations()[before..], [0x18]);
    assert_eq!(infos(&e).len(), 1);
    assert_eq!(vars(&e, EVENTS_B_LIST), [(5, 0.0)]);
    assert_eq!(saved_entry(&mut e, 0x10b, b"Speed").unwrap().mod_index, 9);
    assert_eq!(saved_entry(&mut e, 0x10a, b"Speed").unwrap().mod_index, 5);
}

/// A name the script has that JIP did not add: nothing (`AddVariable` gives null). The walk
/// passes over an empty node and a variable with no name.
#[test]
fn add_a_name_the_script_has_does_nothing() {
    let mut e = engine();
    let (info, name) = (FORMS + 0x7000, FORMS + 0x7100);
    e.mem.set_u32(info, 2);
    e.mem.set_u32(info + 0x18, name);
    e.mem.write(name, b"health\0");
    e.mem.set_u32(SCRIPT_A + 0x4c, info);
    assert_eq!(on(&mut e, ADD, 0, &[s(b"Health"), r(1)]), 0.0);
    assert!(allocations().is_empty());
    assert!(vars(&e, EVENTS_A_LIST).is_empty());
    assert!(saved_entry(&mut e, 0x10a, b"Health").is_none());
    let (empty, nameless) = (FORMS + 0x7200, FORMS + 0x7300);
    e.mem.set_u32(nameless, 1);
    e.mem.set_u32(SCRIPT_A + 0x4c, 0);
    e.mem.set_u32(SCRIPT_A + 0x50, empty);
    e.mem.set_u32(empty + 4, FORMS + 0x7400);
    e.mem.set_u32(FORMS + 0x7400, nameless);
    e.mem.set_u32(FORMS + 0x7404, FORMS + 0x7500);
    e.mem.set_u32(FORMS + 0x7500, info);
    assert_eq!(variable_by_name(&e, SCRIPT_A, b"HEALTH"), info);
    assert_eq!(variable_by_name(&e, SCRIPT_A, b"health2"), 0);
    assert_eq!(variable_by_name(&e, SCRIPT_A, b"heal"), 0);
}

/// A temporary name (`*` first): made in the script and the event list, not kept.
#[test]
fn add_a_temporary_name_is_not_kept() {
    let mut e = engine();
    assert_eq!(on(&mut e, ADD, 0, &[s(b"*tmp"), r(1)]), 1.0);
    assert_eq!(infos(&e), [(5, b"*tmp".to_vec())]);
    assert_eq!(vars(&e, EVENTS_A_LIST), [(5, 0.0)]);
    assert!(e.ext::<Jip>().saved.script_vars.is_empty());
    assert!(e.ext::<Jip>().script_vars.live.is_empty());
}

/// No form: the calling reference (its `ExtraScript`'s script and event list; its id the
/// owner); neither: nothing; an empty name: nothing; an owner with no script: nothing.
#[test]
fn add_owner_rules() {
    let mut e = engine();
    assert_eq!(on(&mut e, ADD, REFR, &[s(b"Speed")]), 1.0);
    assert!(saved_entry(&mut e, 0x200, b"Speed").is_some());
    assert_eq!(vars(&e, EVENTS_A_LIST), [(5, 0.0)]);
    let mut e = engine();
    assert_eq!(on(&mut e, ADD, 0, &[s(b"Speed")]), 0.0);
    assert_eq!(on(&mut e, ADD, 0, &[s(b""), r(1)]), 0.0);
    assert_eq!(on(&mut e, ADD, 0, &[s(b"Speed"), r(3)]), 0.0);
    assert!(allocations().is_empty());
    assert!(e.ext::<Jip>().saved.script_vars.is_empty());
}

/// An event list with no variable list: the `VariableInfo` is made, no `ScriptVar` (JIP would
/// write through null): no result, nothing kept.
#[test]
fn add_to_an_event_list_without_a_list() {
    let mut e = engine();
    e.mem.set_u32(EVENTS_A + 0xc, 0);
    assert_eq!(on(&mut e, ADD, 0, &[s(b"Speed"), r(1)]), 0.0);
    assert_eq!(allocations(), [0x20, 6]);
    assert!(e.ext::<Jip>().saved.script_vars.is_empty());
}

#[test]
fn has_variable_added_needs_all_four() {
    let mut e = engine();
    assert_eq!(on(&mut e, HAS, 0, &[s(b"Speed"), r(1)]), 0.0, "not added");
    on(&mut e, ADD, 0, &[s(b"Speed"), r(1)]);
    assert_eq!(on(&mut e, HAS, 0, &[s(b"SPEED"), r(1)]), 1.0);
    // The saved entry removed, the script's and the event list's variables still there: 0.
    on(&mut e, REMOVE, 0, &[s(b"Speed"), r(1)]);
    assert_eq!(vars(&e, EVENTS_A_LIST), [(5, 0.0)]);
    assert_eq!(on(&mut e, HAS, 0, &[s(b"Speed"), r(1)]), 0.0);
    on(&mut e, ADD, 0, &[s(b"Speed"), r(1)]);
    assert_eq!(on(&mut e, HAS, 0, &[s(b"Speed"), r(1)]), 1.0);
    // The second quest shares the script but has no saved entry.
    assert_eq!(on(&mut e, HAS, 0, &[s(b"Speed"), r(2)]), 0.0);
    // Saved for the second quest, then its event list loses the variable.
    on(&mut e, ADD, 0, &[s(b"Speed"), r(2)]);
    assert_eq!(on(&mut e, HAS, 0, &[s(b"Speed"), r(2)]), 1.0);
    e.mem.set_u32(EVENTS_B_LIST, 0);
    assert_eq!(on(&mut e, HAS, 0, &[s(b"Speed"), r(2)]), 0.0);
    // The name gone from the script's list.
    e.mem.set_u32(SCRIPT_A + 0x4c, 0);
    assert_eq!(on(&mut e, HAS, 0, &[s(b"Speed"), r(1)]), 0.0);
    // Has makes nothing.
    assert_eq!(on(&mut e, HAS, 0, &[s(b"Other"), r(1)]), 0.0);
    assert!(saved_entry(&mut e, 0x10a, b"Other").is_none());
    // The calling reference by default.
    let mut e = engine();
    on(&mut e, ADD, REFR, &[s(b"Speed")]);
    assert_eq!(on(&mut e, HAS, REFR, &[s(b"Speed")]), 1.0);
    assert_eq!(on(&mut e, HAS, 0, &[s(b"Speed")]), 0.0);
}

#[test]
fn remove_script_variable_takes_the_saved_entry_out() {
    let mut e = engine();
    on(&mut e, ADD, 0, &[s(b"Speed"), r(1)]);
    on(&mut e, ADD, 0, &[s(b"Count"), r(1)]);
    assert_eq!(on(&mut e, REMOVE, 0, &[s(b"speed"), r(1)]), 1.0);
    assert!(saved_entry(&mut e, 0x10a, b"Speed").is_none());
    assert!(saved_entry(&mut e, 0x10a, b"Count").is_some());
    assert!(live_var(&mut e, 0x10a, b"Speed").is_none());
    // The script's and the event list's variables stay.
    assert_eq!(infos(&e).len(), 2);
    assert_eq!(vars(&e, EVENTS_A_LIST).len(), 2);
    assert_eq!(on(&mut e, REMOVE, 0, &[s(b"Speed"), r(1)]), 0.0, "gone");
    assert_eq!(on(&mut e, REMOVE, 0, &[s(b"Count"), r(1)]), 1.0);
    assert!(
        e.ext::<Jip>().saved.script_vars.get(&0x10a).is_none(),
        "the emptied owner"
    );
    assert_eq!(on(&mut e, REMOVE, 0, &[s(b"Count"), r(2)]), 0.0, "no owner");
    assert_eq!(
        on(&mut e, REMOVE, 0, &[s(b"Count")]),
        0.0,
        "no form or reference"
    );
    // Remove needs no script: an owner with none.
    e.ext::<Jip>()
        .saved
        .script_vars
        .emplace_with(0x300, || UnorderedMap::new(4))
        .0
        .emplace(Name(b"x".to_vec()));
    assert_eq!(on(&mut e, REMOVE, 0, &[s(b"X"), r(3)]), 1.0);
    assert!(e.ext::<Jip>().saved.script_vars.is_empty());
}

/// Only the calling script's mod's entries go; an emptied owner too; the variables stay.
#[test]
fn remove_all_added_variables_takes_the_mods_entries() {
    let mut e = engine();
    on(&mut e, ADD, 0, &[s(b"Speed"), r(1)]);
    on(&mut e, ADD, 0, &[s(b"Count"), r(1)]);
    set_mod(&mut e, 7);
    on(&mut e, ADD, 0, &[s(b"Other"), r(1)]);
    on(&mut e, REMOVE_ALL, 0, &[r(1)]);
    assert!(saved_entry(&mut e, 0x10a, b"Other").is_none());
    assert!(saved_entry(&mut e, 0x10a, b"Speed").is_some());
    assert!(saved_entry(&mut e, 0x10a, b"Count").is_some());
    assert!(live_var(&mut e, 0x10a, b"Other").is_none());
    assert!(live_var(&mut e, 0x10a, b"Speed").is_some());
    set_mod(&mut e, 5);
    assert_eq!(on(&mut e, REMOVE_ALL, 0, &[r(1)]), 0.0, "no result");
    assert!(e.ext::<Jip>().saved.script_vars.get(&0x10a).is_none());
    assert_eq!(infos(&e).len(), 3);
    // The calling reference by default; neither: nothing.
    let mut e = engine();
    on(&mut e, ADD, REFR, &[s(b"Speed")]);
    on(&mut e, REMOVE_ALL, 0, &[]);
    assert!(saved_entry(&mut e, 0x200, b"Speed").is_some());
    on(&mut e, REMOVE_ALL, REFR, &[]);
    assert!(saved_entry(&mut e, 0x200, b"Speed").is_none());
}

/// Before a save each entry's value is read from its variable; one whose memory cannot be read
/// keeps its value.
#[test]
fn refresh_reads_the_live_values() {
    let mut e = engine();
    on(&mut e, ADD, 0, &[s(b"Speed"), r(1)]);
    on(&mut e, ADD, 0, &[s(b"Count"), r(1)]);
    let var = |e: &Engine, id: u32| {
        items(e, EVENTS_A_LIST)
            .into_iter()
            .find(|&v| e.mem.u32(v) == id)
            .unwrap()
    };
    let (speed, count) = (var(&e, 5), var(&e, 6));
    e.mem.set_f64(speed + 8, 12.0);
    e.mem.set_f64(count + 8, -3.0);
    *e.ext::<Jip>()
        .script_vars
        .live
        .get_mut(&0x10a)
        .unwrap()
        .get_mut(&Name(b"Count".to_vec()))
        .unwrap() = 0xfff0_0000;
    refresh_saved_values(&mut e);
    assert_eq!(
        saved_entry(&mut e, 0x10a, b"Speed").unwrap().value,
        12.0f64.to_bits()
    );
    assert_eq!(saved_entry(&mut e, 0x10a, b"Count").unwrap().value, 0);
}
