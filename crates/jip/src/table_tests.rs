//! Tests of JIP's slots: pinned to its source's list and to what its shipped DLL registers, and
//! as they are written into the table.

use super::*;
use crate::slots::SLOTS;
use crate::test_support::*;
use nvse::table::{guest_text, record_of, return_type, Record, DEFAULT_PARSE};

/// The rows of a committed list: tab-separated, after its `#` lines and its header.
fn rows(text: &str) -> Vec<Vec<&str>> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .skip(1)
        .map(|l| l.split('\t').collect())
        .collect()
}

fn hex(text: &str) -> u32 {
    u32::from_str_radix(text, 16).unwrap()
}

/// Every slot is the source's, as a list made apart from the generator has it
/// (`data/source-57.30.tsv`): opcode, registration (the return type), name, `DEFINE_*` macro (the
/// kind; the `ALT` ones and only they have an alias), alias, needing a reference, parameter list
/// and file.
#[test]
fn every_slot_is_the_sources() {
    let rows = rows(include_str!("../data/source-57.30.tsv"));
    assert_eq!(rows.len(), SLOTS.len());
    for (slot, row) in SLOTS.iter().zip(&rows) {
        let opcode = hex(row[0]);
        assert_eq!(slot.opcode, opcode);
        if row[1] == "EMPTY" {
            assert_eq!(
                (slot.kind, slot.name),
                (Kind::Empty, "NONE"),
                "{opcode:04x}"
            );
            continue;
        }
        let returns = match row[1] {
            "CMD" => NUM,
            "CMD_FRM" => FORM,
            "CMD_STR" => STR,
            "CMD_ARR" => ARR,
            "CMD_AMB" => AMB,
            other => panic!("{opcode:04x}: {other}"),
        };
        let kind = match row[3] {
            "COMMAND_PLUGIN" | "COMMAND_ALT_PLUGIN" => Kind::Command,
            "COMMAND_PLUGIN_EXP" | "COMMAND_ALT_PLUGIN_EXP" => Kind::Expression,
            "CMD_COND_PLUGIN" | "CMD_ALT_COND_PLUGIN" => Kind::Condition,
            "CMD_COND_ONLY" => Kind::ConditionOnly,
            other => panic!("{opcode:04x}: {other}"),
        };
        assert_eq!(
            (
                slot.name,
                slot.alias,
                slot.needs_ref,
                slot.params.name,
                slot.returns,
                slot.kind,
                slot.file
            ),
            (row[2], row[4], row[5] == "1", row[6], returns, kind, row[7]),
            "{opcode:04x}"
        );
        assert_eq!(
            row[3].contains("_ALT_"),
            !slot.alias.is_empty(),
            "{opcode:04x}"
        );
    }
}

/// Every slot is what the shipped `jip_nvse.dll` 57.30 registers (`data/dll-57.30.tsv`, read
/// once from it): opcode, names, needing a reference, the counted parameters (type and
/// optional), whether a list is set, the execute (JIP's own, none, or the exe's for an empty
/// slot), the expression parser, the eval, the flags and the return type.
#[test]
fn every_slot_is_what_the_dll_registers() {
    let rows = rows(include_str!("../data/dll-57.30.tsv"));
    assert_eq!(rows.len(), SLOTS.len());
    for (slot, row) in SLOTS.iter().zip(&rows) {
        let opcode = hex(row[0]);
        assert_eq!(slot.opcode, opcode);
        let params: Vec<String> = slot.params.list[..usize::from(slot.param_count())]
            .iter()
            .map(|p| format!("{:x}:{}", p.kind, u8::from(p.optional)))
            .collect();
        let execute = match slot.kind {
            Kind::ConditionOnly => "0".to_string(),
            Kind::Empty => format!("{EMPTY_EXECUTE:08x}"),
            _ => "dll".to_string(),
        };
        let returns = match slot.returns {
            Returns::Default => "0",
            Returns::Form => "1",
            Returns::String => "2",
            Returns::Array => "3",
            Returns::Ambiguous => "5",
            Returns::ArrayIndex => "4",
        };
        let parse = match slot.kind {
            Kind::Expression => "expression",
            _ => "0",
        };
        let eval = match slot.kind.flags() {
            1 => "dll",
            _ => "0",
        };
        let ours = [
            slot.name.to_string(),
            slot.alias.to_string(),
            u8::from(slot.needs_ref).to_string(),
            slot.param_count().to_string(),
            params.join(","),
            u8::from(!slot.params.list.is_empty()).to_string(),
            execute,
            parse.to_string(),
            eval.to_string(),
            slot.kind.flags().to_string(),
            returns.to_string(),
        ];
        assert_eq!(ours[..], row[1..], "{opcode:04x}");
    }
}

/// What the plan counts: the LN block's 315 slots (299 commands, 16 empty) and JIP's 835 (819, 16
/// empty), one after another, with `SetLinkedReference` in both; 54 condition commands (2 of them
/// condition-only); 123 commands with an alias; 108 form, 48 string, 69 array and 3 ambiguous
/// results; 2 with xNVSE's expression parser.
#[test]
fn the_counts() {
    let count = |range: &std::ops::RangeInclusive<u32>, empty: bool| {
        SLOTS
            .iter()
            .filter(|s| range.contains(&s.opcode) && (s.kind == Kind::Empty) == empty)
            .count()
    };
    assert_eq!((count(&LN_BLOCK, false), count(&LN_BLOCK, true)), (299, 16));
    assert_eq!(
        (count(&JIP_BLOCK, false), count(&JIP_BLOCK, true)),
        (819, 16)
    );
    for (i, s) in SLOTS.iter().enumerate() {
        assert_eq!(slot(s.opcode).map(|t| t.opcode), Some(s.opcode), "{i}");
    }
    assert!(slot(0x233b).is_none() && slot(0x25ff).is_none() && slot(0x2943).is_none());
    let linked: Vec<u32> = SLOTS
        .iter()
        .filter(|s| s.name == "SetLinkedReference")
        .map(|s| s.opcode)
        .collect();
    assert_eq!(linked, [0x2281, 0x2634]);
    let mut names: Vec<&str> = SLOTS
        .iter()
        .filter(|s| s.kind != Kind::Empty)
        .map(|s| s.name)
        .collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 1117);
    let kinds = |k: Kind| SLOTS.iter().filter(|s| s.kind == k).count();
    assert_eq!(
        (kinds(Kind::Condition), kinds(Kind::ConditionOnly)),
        (52, 2)
    );
    assert_eq!(kinds(Kind::Expression), 2);
    let aliased: Vec<&str> = SLOTS
        .iter()
        .filter(|s| !s.alias.is_empty())
        .map(|s| s.name)
        .collect();
    assert_eq!(aliased.len(), 124, "SetLinkedReference's alias twice");
    let results = |r: Returns| SLOTS.iter().filter(|s| s.returns == r).count();
    assert_eq!(
        (
            results(FORM),
            results(STR),
            results(ARR),
            results(AMB),
            results(Returns::ArrayIndex)
        ),
        (108, 48, 69, 3, 0)
    );
}

/// Installed, xNVSE's table reaches `0x2943` and holds JIP's records as JIP registers them (its
/// names when it answers "installed", the empty help, needing a reference, the counted
/// parameters and the list they are in, no handler and no eval, the default parser, the flags,
/// the return type; `ar_Cat` renamed by xNVSE), the parameter lists shared as JIP's are, xNVSE's
/// own records unchanged, and the slots no built-in fills empty.
#[test]
fn installed_the_table_holds_jips_records() {
    for installed in [false, true] {
        installed_records(installed);
    }
}

fn installed_records(installed: bool) {
    let mut e = engine();
    let xnvse_before = Record::read(&e, record_of(&e, 0x15df));
    crate::install_as(&mut e, installed);
    let t = e.script_tables();
    assert_eq!(t.max_opcode, LAST_OPCODE + 1);
    assert_eq!(e.mem.u32(t.end + 8), LAST_OPCODE + 1);
    if !installed {
        assert_eq!(Record::read(&e, record_of(&e, 0x15df)), xnvse_before);
    }
    for slot in SLOTS.iter() {
        let at = record_of(&e, slot.opcode);
        let r = Record::read(&e, at);
        let (name, alias) = if !installed {
            ("", "")
        } else if slot.opcode == 0x22c0 {
            ("JIP_ar_Cat", "JIP_pinto_Cat")
        } else {
            (slot.name, slot.alias)
        };
        let o = slot.opcode;
        assert_eq!(guest_text(&e, r.long_name), name.as_bytes(), "{o:04x}");
        assert_eq!(guest_text(&e, r.short_name), alias.as_bytes(), "{o:04x}");
        assert_eq!(guest_text(&e, r.help), b"", "{o:04x}");
        assert_eq!(
            (r.opcode, r.needs_ref, r.param_count),
            (o, u16::from(slot.needs_ref), slot.param_count()),
            "{o:04x}"
        );
        let execute = if installed && slot.kind == Kind::Empty {
            EMPTY_EXECUTE
        } else if installed && (o == 0x2200 || o == 0x2210) {
            nvse::table::execute_address(o)
        } else {
            0
        };
        assert_eq!(
            (r.execute, r.parse, r.eval, r.flags),
            (execute, DEFAULT_PARSE, 0, slot.kind.flags()),
            "{o:04x}"
        );
        assert_eq!(r.params == 0, slot.params.list.is_empty(), "{o:04x}");
        for (i, p) in slot.params.list.iter().enumerate() {
            let at = r.params + 12 * i as u32;
            assert_eq!(guest_text(&e, e.mem.u32(at)), b"", "{o:04x}");
            assert_eq!(
                (e.mem.u32(at + 4), e.mem.u32(at + 8)),
                (p.kind, u32::from(p.optional)),
                "{o:04x}"
            );
        }
        assert_eq!(return_type(&e, o), slot.returns, "{o:04x}");
    }
    // `GetGameDaysPassed` lists what `kEmptyCommand` lists, in the same place.
    let days = Record::read(&e, record_of(&e, 0x282f));
    assert_eq!(days.params, Record::read(&e, record_of(&e, 0x2213)).params);
    for opcode in [0x168e, 0x2131, 0x2132, 0x21ff, 0x233b, 0x25ff] {
        let r = Record::read(&e, record_of(&e, opcode));
        assert_eq!(
            (guest_text(&e, r.long_name), r.opcode, r.execute, r.params),
            (Vec::new(), opcode, 0, 0),
            "{opcode:04x}"
        );
    }
    // A second install changes nothing.
    crate::install_as(&mut e, !installed);
    assert_eq!(e.script_tables(), t);
    let empty = if installed { EMPTY_EXECUTE } else { 0 };
    assert_eq!(execute_of(&e, 0x2213), empty);
}

/// Answering "not installed", a scan of the names in the plugins' part of the table (as the
/// console's and `help`'s name lookups make) finds none of JIP's: no name, alias or xNVSE's
/// rename of its `ar_Cat` (xNVSE's own `ar_Cat`, below, is xNVSE's).
#[test]
fn not_installed_no_name_finds_jip() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    let mut jip: Vec<&[u8]> = SLOTS
        .iter()
        .flat_map(|s| [s.name.as_bytes(), s.alias.as_bytes()])
        .filter(|n| !n.is_empty())
        .collect();
    jip.extend([&b"JIP_ar_Cat"[..], b"JIP_pinto_Cat"]);
    let t = e.script_tables();
    for opcode in nvse::names::PLUGIN_OPCODES..t.max_opcode {
        let r = Record::read(&e, t.record(opcode));
        for name in [r.long_name, r.short_name] {
            let text = guest_text(&e, name);
            assert!(
                text.is_empty() || !jip.iter().any(|j| j.eq_ignore_ascii_case(&text)),
                "{opcode:04x}: {}",
                String::from_utf8_lossy(&text)
            );
        }
    }
}

/// A condition naming a function past JIP's end reads what the condition code (`00681600`: the
/// eval at `+0x20`, the reference byte at `+0x10`) and a condition's `InitItem` (`006819e0`:
/// the eval, then the name at `+0`) read: zeros (no eval: the condition is false), up to the
/// room's last opcode, a slot between xNVSE's and JIP's an empty record. Opcodes `0x2100`
/// (between) and those of JohnnyGuitar (`0x3100`), ShowOff (`0x3C93`) and ITR (`0x4000`).
#[test]
fn other_plugins_conditions_read_no_eval() {
    let mut e = engine();
    crate::install_as(&mut e, false);
    let t = e.script_tables();
    for opcode in [0x2100, 0x3100, 0x31ff, 0x3c93, 0x4000, 0x410e, 0x4331] {
        let at = t.record(opcode);
        let (eval, reference) = (e.mem.u32(at + 0x20), e.mem.u8(at + 0x10));
        let name = guest_text(&e, e.mem.u32(at));
        assert_eq!((eval, reference, name), (0, 0, Vec::new()), "{opcode:04x}");
    }
}

/// A JIP command not built yet fails as it does without the DLL: its record has no handler
/// (`EvaluateLine` and the operand reader fail a line whose record has none, the condition code
/// answers false for a record with no eval), and xNVSE's expression evaluator does not read it
/// as a command.
#[test]
fn an_unbuilt_command_fails() {
    let mut e = engine();
    crate::install_as(&mut e, true);
    let file_exists = Record::read(&e, record_of(&e, 0x2201));
    assert_eq!(guest_text(&e, file_exists.long_name), b"FileExists");
    assert_eq!((file_exists.execute, file_exists.eval), (0, 0));
    let button = Record::read(&e, record_of(&e, 0x2204));
    assert_eq!(guest_text(&e, button.long_name), b"IsButtonPressed");
    assert_eq!((button.execute, button.eval, button.flags), (0, 0, 1));
    let words = line(&mut e, 0x2201, &[]);
    let a = nvse::CommandArgs::new(&words);
    // An `X` token: no calling reference, the opcode, its arguments' length.
    let at = DATA + 4;
    for (i, b) in [b'X', 0, 0, 0x01, 0x22, 2, 0].iter().enumerate() {
        e.mem.set_u8(at + i as u32, *b);
    }
    let mut ev = nvse::expr::Eval::new(&e, &a);
    assert!(ev.read_one_token(&mut e).is_none());
    assert_eq!(ev.errors, ["Failed to resolve command with opcode 2201"]);
}

/// Answering "installed", an empty slot is `NONE` and runs `kEmptyCommand`'s execute, the exe's
/// `005d4a40` (true); not answering, it has neither a name nor an execute.
#[test]
fn empty_slots_run_as_jips_once_installed() {
    for (installed, execute, name) in [(false, 0, &b""[..]), (true, EMPTY_EXECUTE, b"NONE")] {
        let mut e = engine();
        crate::install_as(&mut e, installed);
        for s in SLOTS.iter().filter(|s| s.kind == Kind::Empty) {
            let r = Record::read(&e, record_of(&e, s.opcode));
            assert_eq!(guest_text(&e, r.long_name), name);
            assert_eq!((r.execute, r.param_count), (execute, 0));
            assert_ne!(r.params, 0, "three optional integers listed");
        }
    }
}

/// A built command's handlers go to its record, at the extenders' places for its opcode, and
/// its names with them (a built command answers whatever JIP does); handlers that do not match
/// its kind are refused.
#[test]
fn handlers_match_the_kind() {
    fn ok(_: &mut Engine, _: &[u32]) -> Ret {
        nvse::handled(true)
    }
    let mut e = engine();
    crate::install_as(&mut e, false);
    set_handlers(&mut e, 0x2204, Some(ok), Some(ok));
    let r = Record::read(&e, record_of(&e, 0x2204));
    assert_eq!(
        (r.execute, r.eval),
        (
            nvse::table::execute_address(0x2204),
            nvse::table::eval_address(0x2204)
        )
    );
    assert_eq!(guest_text(&e, r.long_name), b"IsButtonPressed");
    assert_eq!(r.param_count, 1);
    assert_eq!(e.call(r.execute, &[0; 8]).eax, 1);
    set_handlers(&mut e, 0x27b1, None, Some(ok));
    assert_eq!(execute_of(&e, 0x27b1), 0, "condition-only");
    for (opcode, execute, eval) in [
        (0x2201, Some(ok as AbiFn), Some(ok as AbiFn)),
        (0x2204, Some(ok), None),
        (0x27b1, Some(ok), Some(ok)),
        (0x2213, Some(ok), None),
        (0x2400, Some(ok), None),
    ] {
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            set_handlers(&mut e, opcode, execute, eval)
        }));
        assert!(refused.is_err(), "{opcode:04x}");
    }
}
