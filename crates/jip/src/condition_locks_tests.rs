//! The guard against a condition's eval that takes a shared slot (the condition-locks sweep,
//! 2026-11-15): a condition's eval runs on whatever thread evaluates it (AI threads: package,
//! perk and idle conditions), where taking JIP's shared slot (or xNVSE's) can block the thread
//! behind the main thread's hold until its next safe point, or deadlock against a game lock taken
//! in the other order. Every eval JIP puts in the command table (found by comparing each
//! record's eval word with an engine without JIP: its conditions', its condition-only commands'
//! and its overrides', the vanilla row on, so a condition added later is walked too) is run on a
//! test engine while a second engine of the run (a real thread's, `share_slots_for_test`) holds
//! the shared slots: an eval that asks for one there is refused (two engines on one thread),
//! which the guard catches.
//!
//! What it covers: each eval on a small scene: no reference, and a form whose every vtable entry
//! answers true (`ADDR_ReturnTrue`), so the "is it an actor / a reference" tests pass, each with
//! the parameters 0, 1 and that form. A path the scene does not reach (an eval that stops at a
//! game call the test engine does not have, which panics otherwise) is not checked by this
//! guard; the sweep's table in `measure/jip-cmds/CONDITION-LOCKS.md` follows every path by hand.

use crate::commands::test_support::{engine, FORMS};
use crate::test_support::RESULT;
use crate::Jip;
use engine_core::prelude::*;

/// What `hold` (engine-core `extend.rs`) says when a second engine on the thread asks for a
/// shared slot another engine holds.
const SHARED_SLOT: &str = "a shared extension slot used by two engines on one thread";
/// `ADDR_ReturnTrue`, every vtable entry of [`ANY`].
const RETURN_TRUE: u32 = 0x008d_0360;
/// A form whose vtable answers true everywhere, and its vtable.
const ANY: u32 = FORMS + 0x8_0000;
const ANY_VT: u32 = FORMS + 0x8_1000;

/// An eval that asks for a shared slot it must not: xNVSE's string map, which
/// `AuxVarGetFltCond` reads for the variable's name (listed in the sweep, not made lock-free:
/// xNVSE's string variables change on every string write of every mod).
const TAKES_NVSE: &[u32] = &[0x27b1];

/// An engine with JIP installed answering "installed", the vanilla overrides' row on, and the
/// scene.
fn guard_engine() -> Engine {
    let mut e = engine();
    crate::overrides::set_vanilla_commands(&mut e, true);
    // xNVSE's state shared as `nvse::install` shares it (the test engine has only its table).
    e.share_ext::<nvse::Nvse>();
    // The game's globals the evals read (the player, the interface manager, menus, ...).
    e.map(0x011c_0000, 0x4_0000);
    for slot in 0..0x400 {
        e.mem.set_u32(ANY_VT + 4 * slot, RETURN_TRUE);
    }
    e.mem.set_u32(ANY, ANY_VT);
    engine_core::extend::release_held_for_test();
    e
}

/// Every opcode whose record's eval word JIP set (it differs from an engine without JIP), with
/// that word.
fn jip_evals() -> Vec<(u32, u32)> {
    let e = guard_engine();
    let plain = crate::test_support::engine();
    let t = e.script_tables();
    let mut out = Vec::new();
    for opcode in engine_core::extend::ScriptTables::FIRST_OPCODE..t.max_opcode {
        let at = e.mem.u32(t.record(opcode) + 0x20);
        let record = nvse::table::record_of(&plain, opcode);
        let before = if record == 0 {
            0
        } else {
            plain.mem.u32(record + 0x20)
        };
        if at != 0 && at != before {
            out.push((opcode, at));
        }
    }
    engine_core::extend::release_held_for_test();
    out
}

/// Runs the eval at `at` with `words` on `e` while a second engine holds JIP's shared slot (and
/// xNVSE's, with `nvse`): `Err` when the eval asked for a held slot; any other stop (a game call
/// the test engine lacks) is `Ok(false)`, a finished eval `Ok(true)`.
fn run_held(e: &mut Engine, at: u32, words: [u32; 4], with_nvse: bool) -> Result<bool, String> {
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    engine_core::extend::release_held_for_test();
    assert!(
        other.ext::<Jip>().installed,
        "the other engine holds JIP's state"
    );
    if with_nvse {
        assert!(other.ext_get::<nvse::Nvse>().is_some(), "and xNVSE's");
    }
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        e.call(at, &words);
    }));
    engine_core::extend::release_held_for_test();
    match run {
        Ok(()) => Ok(true),
        Err(p) => {
            let text = p
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            if text.contains(SHARED_SLOT) {
                Err(text)
            } else {
                Ok(false)
            }
        }
    }
}

/// Every combination of the scene for one eval; the ones where it asked for a held slot, and
/// how many ran to the end.
fn sweep(opcode: u32, at: u32, with_nvse: bool) -> (Vec<String>, usize) {
    let mut e = guard_engine();
    let (mut found, mut finished) = (Vec::new(), 0);
    for this in [0, ANY] {
        for arg1 in [0, 1, ANY] {
            for arg2 in [0, 1] {
                match run_held(&mut e, at, [this, arg1, arg2, RESULT], with_nvse) {
                    Ok(true) => finished += 1,
                    Ok(false) => e = guard_engine(),
                    Err(text) => {
                        found.push(format!("{opcode:04x} ({this:x}, {arg1:x}, {arg2}): {text}"));
                        e = guard_engine();
                    }
                }
            }
        }
    }
    (found, finished)
}

/// No eval JIP puts takes JIP's shared slot; none but the listed one takes xNVSE's.
#[test]
fn no_condition_eval_takes_a_shared_slot() {
    let evals = jip_evals();
    // The 51 of the conditions' lists and the 7 overrides (0x1024, 0x102F, 0x1174, 0x11C1, 0x1453,
    // 0x146B, 0x1511), at the least.
    let listed: usize = crate::commands::ALL_CONDITIONS
        .iter()
        .map(|l| l.len())
        .sum::<usize>()
        + crate::commands::ALL_CONDITIONS_ONLY
            .iter()
            .map(|l| l.len())
            .sum::<usize>();
    assert!(evals.len() >= listed + 7, "{} evals found", evals.len());
    for op in [
        0x1024, 0x102f, 0x1174, 0x11c1, 0x1453, 0x146b, 0x1511, 0x27b1, 0x2204,
    ] {
        assert!(evals.iter().any(|&(o, _)| o == op), "{op:04x} walked");
    }
    let mut found = Vec::new();
    let mut finished = 0;
    for &(opcode, at) in &evals {
        let (f, n) = sweep(opcode, at, false);
        found.extend(f);
        finished += n;
        if !TAKES_NVSE.contains(&opcode) {
            let (f, n) = sweep(opcode, at, true);
            found.extend(f);
            finished += n;
        }
    }
    assert!(
        found.is_empty(),
        "evals that take a shared slot:\n{}",
        found.join("\n")
    );
    // Most of the runs finish on the scene (the guard is not passing for want of reaching code).
    assert!(finished * 2 > evals.len() * 12, "{finished} runs finished");
}

/// The guard catches an eval that takes JIP's shared slot, and one that takes xNVSE's.
#[test]
fn the_guard_catches_an_eval_that_takes_a_shared_slot() {
    let at = 0x0700_2000;
    let mut e = guard_engine();
    e.register(at, |e, _| {
        e.ext::<Jip>();
        Ret::default()
    });
    let held = run_held(&mut e, at, [0, 0, 0, RESULT], false);
    assert!(held.is_err_and(|t| t.contains(SHARED_SLOT)), "JIP's");
    let mut e = guard_engine();
    e.register(at, |e, _| {
        e.ext::<nvse::Nvse>();
        Ret::default()
    });
    assert_eq!(
        run_held(&mut e, at, [0, 0, 0, RESULT], false),
        Ok(true),
        "not held"
    );
    let mut e = guard_engine();
    e.register(at, |e, _| {
        e.ext::<nvse::Nvse>();
        Ret::default()
    });
    let held = run_held(&mut e, at, [0, 0, 0, RESULT], true);
    assert!(held.is_err_and(|t| t.contains(SHARED_SLOT)), "xNVSE's");
}
