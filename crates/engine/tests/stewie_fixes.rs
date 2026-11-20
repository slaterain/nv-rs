//! lStewieAl's Tweaks' crash fixes (`crates/stewie`, docs/FIXES.md "lStewieAl's Tweaks"): each row's
//! crashing input fed to the translated function with every translation it reaches linked in, the
//! row on (no stop) and off (the exe's stop). The callers' other calls stand in; the call Tweaks
//! guards runs translated.

use engine as _;
use engine_core::extend::stewie;
use engine_core::prelude::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

// Each share's rows, in files of their own (`stewie_fixes/`), so the four agents never edit the
// same lines.
#[path = "stewie_fixes/ai.rs"]
mod ai;
#[path = "stewie_fixes/forms.rs"]
mod forms;
#[path = "stewie_fixes/menus.rs"]
mod menus;
#[path = "stewie_fixes/world.rs"]
mod world;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The shared actor's clear `00935d90` (S0079's place, `0x935E35`).
const SHARED_ACTOR_CLEAR: u32 = 0x0093_5d90;
/// The shared actor's word.
const SHARED: u32 = 0x011e_0548;

/// Stand-ins: the form lookup's actor, its "is it an actor" slot, the dialogue test, the reference
/// whose 3D (slot 0x1d0) is `node`, the process's two slots.
const LOOKUP: u32 = 0x0048_39c0;
const IN_DIALOGUE: u32 = 0x0057_4900;
const REFERENCE: u32 = 0x005e_3fa0;
const SLOT_FNS: u32 = 0x00ff_4000;

static NODE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static ACTOR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
static PROCESS_CALLS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// An object whose vtable's every slot calls the stand-in at `SLOT_FNS + slot`.
fn object(e: &mut Engine, size: u32) -> u32 {
    let vt = e.mem.alloc(0x800);
    for slot in (0..0x800).step_by(4) {
        e.mem.set_u32(vt + slot, SLOT_FNS + slot);
    }
    let o = e.mem.alloc(size);
    e.mem.set_u32(o, vt);
    o
}

/// The world: a speaker in dialogue whose reference has no 3D loaded (`node` 0) or a node with no
/// controllers.
fn speaker_world(node_is_null: bool) -> Engine {
    use std::sync::atomic::Ordering::Relaxed;
    let mut e = Engine::new();
    e.map(0x011e_0000, 0x1000);
    let shared = e.mem.alloc(0x100);
    e.mem.set_u32(SHARED, shared);
    let actor = object(&mut e, 0x200);
    let process = object(&mut e, 0x100);
    e.mem.set_u32(actor + 0x68, process);
    ACTOR.store(actor, Relaxed);
    // A node of the NiObjectNET layout with no controllers (`m_spControllers` 0).
    let node = if node_is_null { 0 } else { e.mem.alloc(0x40) };
    NODE.store(node, Relaxed);
    PROCESS_CALLS.store(0, Relaxed);
    e.register(LOOKUP, |_, _| ret(ACTOR.load(Relaxed)));
    e.register(IN_DIALOGUE, |_, _| ret(1));
    e.register(REFERENCE, |_, _| ret(ACTOR.load(Relaxed)));
    // The actor's slot 0xfc (an actor: yes), its slot 0x1d0 (the 3D), the process's 0x744 and 0x74c.
    e.register(SLOT_FNS + 0xfc, |_, _| ret(1));
    e.register(SLOT_FNS + 0x1d0, |_, _| ret(NODE.load(Relaxed)));
    e.register(SLOT_FNS + 0x744, |_, _| {
        PROCESS_CALLS.fetch_add(1, Relaxed);
        ret(0)
    });
    e.register(SLOT_FNS + 0x74c, |_, _| {
        PROCESS_CALLS.fetch_add(1, Relaxed);
        ret(0)
    });
    e
}

/// Runs the clear (the setting's restore suppressed); `Err` with the stop's text if it stopped.
fn clear(e: &mut Engine) -> Result<(), String> {
    catch_unwind(AssertUnwindSafe(|| {
        e.call(SHARED_ACTOR_CLEAR, &args![0x1234u32, 1u32]);
    }))
    .map_err(|stop| {
        e.call_stack.clear();
        stop.downcast_ref::<String>()
            .cloned()
            .or_else(|| stop.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default()
    })
}

#[test]
fn s0079_dialog_speech_null_check() {
    use std::sync::atomic::Ordering::Relaxed;
    // The tests of this file share the stand-ins' words: one test at a time.
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());

    // Off (vanilla, or the row's key at 0): the exe's GetController reads through the null node.
    let mut e = speaker_world(true);
    assert!(!e.stewie(stewie::DIALOG_SPEECH_NULL_CHECK));
    e.call_log = Some(Vec::new());
    let stop = clear(&mut e).expect_err("the exe stops on a speaker with no 3D");
    assert!(!stop.is_empty());
    assert!(asked_controller(&e), "it stopped in GetController");
    assert_eq!(
        PROCESS_CALLS.load(Relaxed),
        0,
        "it stopped before the process calls"
    );

    // On: no controller asked, and the clear goes on to the process's two calls.
    let mut e = speaker_world(true);
    e.set_stewie(stewie::DIALOG_SPEECH_NULL_CHECK, true);
    e.call_log = Some(Vec::new());
    let shared = e.mem.u32(SHARED);
    clear(&mut e).expect("no stop with the row on");
    assert!(!asked_controller(&e));
    assert_eq!(PROCESS_CALLS.load(Relaxed), 2);
    // The rest of the clear ran: the shared actor marked and let go.
    assert_eq!(e.mem.u8(shared + 0x7c), 1);
    assert_eq!(e.mem.u32(SHARED), 0);

    // A node that is there: the exe's call, the row on as off (here: no controllers).
    for on in [false, true] {
        let mut e = speaker_world(false);
        e.set_stewie(stewie::DIALOG_SPEECH_NULL_CHECK, on);
        e.call_log = Some(Vec::new());
        clear(&mut e).expect("a loaded node never stopped");
        assert!(asked_controller(&e), "row {on}");
        assert_eq!(PROCESS_CALLS.load(Relaxed), 2);
    }
}

/// Whether the clear reached `NiObjectNET::GetController` (`00a5c570`, translated).
fn asked_controller(e: &Engine) -> bool {
    e.call_log
        .as_ref()
        .is_some_and(|log| log.iter().any(|c| c.0 == 0x00a5_c570))
}

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
