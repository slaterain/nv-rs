//! `NV_HIGH_LIST_CHECK`: diagnostic only, not exe behaviour and not a fix row (crash 0096c710,
//! 2026-11-26: "Game's main stopped ... (in 0096c710 < 0086e650 < 0086a850)" while flying fast
//! across the Mojave). Off by default; with the variable unset nothing is registered and nothing
//! runs.
//!
//! The after-draw stage's detection merge (`0096c710`) walks process level 0 of the process
//! lists' array and treats each actor's process (+0x68) as a `HighProcess` (`008f63e0` reads its
//! lists at +0x25c..+0x26c). Run with the block check, the route stopped reading the guard after a
//! 0x25c-byte block: an actor in level 0's range held a `MiddleHighProcess`. This check finds such
//! actors before the merge runs and names the last level changes that touched them:
//! - `0096c710` is registered over (the process family's native merge when that family is on, its
//!   translation when off; registering over it sends the family's direct calls by address while
//!   the check runs): before the merge, the player (whose process the merge treats the same way
//!   first) and every actor of level 0 whose process does not have `HighProcess`'s vtable
//!   ([`HIGH_PROCESS_VTABLE`]) are printed once per (actor, process) with the form id and type, the
//!   process's vtable and asked size, all four levels' ranges, the indexes the actor sits at, and
//!   its noted level changes;
//! - the four level changes (`00881d30` high, `00882b90`, `00883240`, `00883800` middle-high; the
//!   half-loaded row's own wrappers when that row is on) are registered over: each notes, per
//!   actor (a ring of [`RING`]), the function, the game thread, the after-draw pass it ran in, the
//!   time, the process before and after, its answer and the calls it was made from (the guest call
//!   stack, complete while the block check sends direct calls by address).
//!
//! Lines start with `NV_HIGH_LIST_CHECK:`; at most [`MAX_LINES`] findings. It reads game memory and
//! calls the actor's `IsActor` slot, as the merge itself does; it writes nothing the game reads.

use crate::half_loaded_move;
use engine::engine_typed::process::{after_draw, FAMILY};
use engine::prelude::*;
use engine::units::fallout_ai::actor::{fn_00881d30, fn_00882b90, fn_00883240, fn_00883800};
use engine::units::fallout_ai::run_0096ad00::fn_0096c710;
use engine_core::typed::Mode;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// The detection merge's address, and the four level changes' (high, then the three lower).
const MERGE: u32 = 0x0096_c710;
const LEVEL_CHANGES: [u32; 4] = half_loaded_move::LEVEL_CHANGES;
/// The player (`011dea3c`) and an actor's process (+0x68).
const PLAYER: u32 = 0x011d_ea3c;
const ACTOR_PROCESS: u32 = 0x68;
/// The vtables `HighProcess`'s constructor (`008d7510`) and `MiddleHighProcess`'s (`00913fe0`)
/// store.
const HIGH_PROCESS_VTABLE: u32 = 0x0108_7864;
const MIDDLE_HIGH_PROCESS_VTABLE: u32 = 0x0108_904c;
/// Calls kept of a level change's guest call stack, innermost first.
const CHAIN: usize = 10;
/// `IsActor` (actor slot +0x100).
const SLOT_IS_ACTOR: u32 = 0x100;
/// The process array (the lists' +4): its element block (+4), count (+0xa, a word), and the levels'
/// heads (+0x10) and tails (+0x20), four words each.
const ARRAY: u32 = 4;
const ELEMENTS: u32 = 4;
const COUNT: u32 = 0xa;
const HEADS: u32 = 0x10;
const TAILS: u32 = 0x20;
/// Level changes kept per actor, and findings printed at most.
const RING: usize = 6;
const MAX_LINES: u32 = 60;

/// Whether `NV_HIGH_LIST_CHECK` is set (read once).
pub fn on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("NV_HIGH_LIST_CHECK").is_some())
}

/// The half-loaded row was on when the check installed: the level changes go through its wrappers.
static HALF_LOADED: AtomicBool = AtomicBool::new(false);
/// The after-draw passes seen (each `0096c710` call), and findings printed.
static PASSES: AtomicU64 = AtomicU64::new(0);
static LINES: AtomicU32 = AtomicU32::new(0);

fn started() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// One level change of an actor.
#[derive(Clone, Debug, PartialEq)]
struct Note {
    function: u32,
    thread: usize,
    pass: u64,
    ms: u64,
    before: u32,
    before_size: Option<u32>,
    after: u32,
    after_size: Option<u32>,
    answer: bool,
    /// The guest call stack at the change, innermost first.
    chain: String,
}

/// The innermost [`CHAIN`] calls of `stack` (innermost last), innermost first.
fn chain_text(stack: &[u32]) -> String {
    let calls: Vec<String> = stack
        .iter()
        .rev()
        .take(CHAIN)
        .map(|a| format!("{a:08x}"))
        .collect();
    if calls.is_empty() {
        "no calls by address".to_string()
    } else {
        calls.join(" < ")
    }
}

/// A process's vtable named.
fn vtable_name(vtable: u32) -> &'static str {
    match vtable {
        HIGH_PROCESS_VTABLE => "HighProcess",
        MIDDLE_HIGH_PROCESS_VTABLE => "MiddleHighProcess",
        _ => "another class",
    }
}

/// The rings, by (kernel, actor): run-wide state keyed on the kernel, as real threads need.
fn rings() -> std::sync::MutexGuard<'static, HashMap<(usize, u32), VecDeque<Note>>> {
    static RINGS: OnceLock<Mutex<HashMap<(usize, u32), VecDeque<Note>>>> = OnceLock::new();
    RINGS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// The (actor, process) pairs already printed.
fn printed() -> std::sync::MutexGuard<'static, HashSet<(usize, u32, u32)>> {
    static PRINTED: OnceLock<Mutex<HashSet<(usize, u32, u32)>>> = OnceLock::new();
    PRINTED
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|p| p.into_inner())
}

/// The process class an asked size belongs to (the four constructors' `operator new` sizes).
fn class_of(size: Option<u32>) -> &'static str {
    match size {
        Some(0x46c) => "HighProcess",
        Some(0x25c) => "MiddleHighProcess",
        Some(0xc8) => "MiddleLowProcess",
        Some(0xb4) => "LowProcess",
        Some(_) => "another size",
        None => "size unknown",
    }
}

fn size_text(size: Option<u32>) -> String {
    match size {
        Some(s) => format!("{s:#x} ({})", class_of(size)),
        None => class_of(None).to_string(),
    }
}

fn function_name(function: u32) -> &'static str {
    match function {
        0x0088_1d30 => "00881d30 (to high)",
        0x0088_2b90 => "00882b90",
        0x0088_3240 => "00883240",
        0x0088_3800 => "00883800 (to middle-high)",
        _ => "?",
    }
}

fn process_size(e: &Engine, process: u32) -> Option<u32> {
    (process != 0).then(|| e.mem.asked_size(process)).flatten()
}

/// Registers the check over `0096c710` and the four level changes (after the rows, so the
/// half-loaded row's wrappers are what the level changes call when `half_loaded` says it is on).
pub fn install(e: &mut Engine, half_loaded: bool) {
    HALF_LOADED.store(half_loaded, Ordering::Relaxed);
    started();
    e.register(MERGE, |e, a| {
        check(e, a[0]);
        if e.typed_mode(FAMILY) == Mode::Off {
            fn_0096c710(e, Ptr::new(a[0]));
        } else {
            after_draw::merge_detection(e, Ptr::new(a[0]));
        }
        Ret::default()
    });
    e.register(LEVEL_CHANGES[0], |e, a| {
        noted(e, LEVEL_CHANGES[0], a[0], |e| {
            fn_00881d30(e, Ptr::new(a[0]))
        })
        .into_ret()
    });
    e.register(LEVEL_CHANGES[1], |e, a| {
        noted(e, LEVEL_CHANGES[1], a[0], |e| {
            fn_00882b90(e, Ptr::new(a[0]))
        })
        .into_ret()
    });
    e.register(LEVEL_CHANGES[2], |e, a| {
        noted(e, LEVEL_CHANGES[2], a[0], |e| {
            fn_00883240(e, Ptr::new(a[0]))
        })
        .into_ret()
    });
    e.register(LEVEL_CHANGES[3], |e, a| {
        noted(e, LEVEL_CHANGES[3], a[0], |e| {
            fn_00883800(e, Ptr::new(a[0]))
        })
        .into_ret()
    });
    eprintln!(
        "NV_HIGH_LIST_CHECK: on (diagnostic): 0096c710 checks level 0 before each merge; the four \
         level changes are noted per actor{}",
        if half_loaded {
            " (through the half-loaded row's wrappers)"
        } else {
            ""
        }
    );
}

/// A level change `function` of `actor`, run (through the half-loaded row when it is on) and noted.
fn noted(e: &mut Engine, function: u32, actor: u32, f: impl FnOnce(&mut Engine) -> bool) -> bool {
    let before = e.mem.u32(actor.wrapping_add(ACTOR_PROCESS));
    let before_size = process_size(e, before);
    let answer = if HALF_LOADED.load(Ordering::Relaxed) {
        half_loaded_move::level_change(e, actor, f)
    } else {
        f(e)
    };
    let after = e.mem.u32(actor.wrapping_add(ACTOR_PROCESS));
    let note = Note {
        function,
        thread: e.thread_index(),
        pass: PASSES.load(Ordering::Relaxed),
        ms: started().elapsed().as_millis() as u64,
        before,
        before_size,
        after,
        after_size: process_size(e, after),
        answer,
        chain: chain_text(&e.call_stack),
    };
    let mut rings = rings();
    let ring = rings.entry((e.kernel.id(), actor)).or_default();
    if ring.len() == RING {
        ring.pop_front();
    }
    ring.push_back(note);
    answer
}

/// The player and level 0 of the process lists `lists` checked before the merge.
fn check(e: &mut Engine, lists: u32) {
    let pass = PASSES.fetch_add(1, Ordering::Relaxed) + 1;
    if LINES.load(Ordering::Relaxed) >= MAX_LINES {
        return;
    }
    let player = e.mem.u32(PLAYER);
    if player == 0 {
        return;
    }
    let array = lists.wrapping_add(ARRAY);
    // The merge's first call is for the player's own process.
    let process = e.mem.u32(player.wrapping_add(ACTOR_PROCESS));
    if process != 0 && e.mem.u32(process) != HIGH_PROCESS_VTABLE {
        report(e, pass, array, player, process, None);
    }
    let elements = e.mem.u32(array.wrapping_add(ELEMENTS));
    let (head, tail) = level_range(e, array, 0);
    for index in head..tail.min(head.saturating_add(4096)) {
        let actor = e.mem.u32(elements.wrapping_add(index.wrapping_mul(4)));
        if actor == 0 || !e.vcall(actor, SLOT_IS_ACTOR, &args![]).bool() {
            continue;
        }
        let process = e.mem.u32(actor.wrapping_add(ACTOR_PROCESS));
        if process != 0 && e.mem.u32(process) == HIGH_PROCESS_VTABLE {
            continue;
        }
        report(e, pass, array, actor, process, Some(index));
    }
}

/// Level `level`'s head and tail in the process array `array`.
fn level_range(e: &Engine, array: u32, level: u32) -> (u32, u32) {
    (
        e.mem.u32(array.wrapping_add(HEADS + 4 * level)),
        e.mem.u32(array.wrapping_add(TAILS + 4 * level)),
    )
}

/// One finding printed, once per (actor, process): `index` is its place in level 0, `None` for
/// the player.
fn report(e: &mut Engine, pass: u64, array: u32, actor: u32, process: u32, index: Option<u32>) {
    if !printed().insert((e.kernel.id(), actor, process)) {
        return;
    }
    if LINES.fetch_add(1, Ordering::Relaxed) >= MAX_LINES {
        return;
    }
    let vtable = if process != 0 { e.mem.u32(process) } else { 0 };
    let elements = e.mem.u32(array.wrapping_add(ELEMENTS));
    let count = e.mem.u16(array.wrapping_add(COUNT)) as u32;
    let ranges: Vec<String> = (0..4)
        .map(|level| {
            let (h, t) = level_range(e, array, level);
            format!("{level}: [{h}, {t})")
        })
        .collect();
    let at: Vec<String> = (0..count.min(8192))
        .filter(|&i| e.mem.u32(elements.wrapping_add(i * 4)) == actor)
        .map(|i| i.to_string())
        .collect();
    let notes: Vec<String> = rings()
        .get(&(e.kernel.id(), actor))
        .map(|ring| {
            ring.iter()
                .map(|n| {
                    format!(
                        "{} on game thread {} at pass {} ({} ms): process {:08x} {} -> {:08x} {}, \
                         answered {}, from {}",
                        function_name(n.function),
                        n.thread,
                        n.pass,
                        n.ms,
                        n.before,
                        size_text(n.before_size),
                        n.after,
                        size_text(n.after_size),
                        n.answer,
                        n.chain
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let place = match index {
        Some(index) => format!("level 0 index {index} holds actor"),
        None => "the player is actor".to_string(),
    };
    eprintln!(
        "NV_HIGH_LIST_CHECK: after-draw pass {pass} ({} ms) on game thread {}: {place} {actor:08x} \
         (form {:08x}, type {:#x}) whose process {process:08x} has vtable {vtable:08x} ({}), size \
         {}; levels {}; count {count}; the actor is at indexes [{}]; its level changes, oldest \
         first: [{}]",
        started().elapsed().as_millis(),
        e.thread_index(),
        e.mem.u32(actor.wrapping_add(0xc)),
        e.mem.u32(actor.wrapping_add(4)) & 0xff,
        vtable_name(vtable),
        size_text(process_size(e, process)),
        ranges.join(", "),
        at.join(", "),
        if notes.is_empty() {
            "none noted".to_string()
        } else {
            notes.join("; ")
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_process_sizes_are_named() {
        assert_eq!(class_of(Some(0x46c)), "HighProcess");
        assert_eq!(class_of(Some(0x25c)), "MiddleHighProcess");
        assert_eq!(class_of(Some(0xc8)), "MiddleLowProcess");
        assert_eq!(class_of(Some(0xb4)), "LowProcess");
        assert_eq!(class_of(Some(0x30)), "another size");
        assert_eq!(class_of(None), "size unknown");
    }

    #[test]
    fn the_four_level_changes_are_the_half_loaded_rows() {
        assert_eq!(
            LEVEL_CHANGES,
            [0x0088_1d30, 0x0088_2b90, 0x0088_3240, 0x0088_3800]
        );
        assert_eq!(function_name(0x0088_3800), "00883800 (to middle-high)");
    }

    #[test]
    fn the_two_vtables_are_named() {
        assert_eq!(vtable_name(0x0108_7864), "HighProcess");
        assert_eq!(vtable_name(0x0108_904c), "MiddleHighProcess");
        assert_eq!(vtable_name(0), "another class");
    }

    #[test]
    fn a_chain_reads_innermost_first_and_is_capped() {
        assert_eq!(chain_text(&[]), "no calls by address");
        assert_eq!(
            chain_text(&[0x0086_a850, 0x0086_e650, 0x0088_3800]),
            "00883800 < 0086e650 < 0086a850"
        );
        let deep: Vec<u32> = (0..20).collect();
        assert_eq!(chain_text(&deep).split(" < ").count(), CHAIN);
        assert!(chain_text(&deep).starts_with("00000013 < "));
    }
}
