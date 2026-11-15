//! Group GP's game patches: G38 (the player's first-person model, the queued player's attach)
//! and H09 (the queued references' attach), from JIP's assembly case by case.

use super::*;
use crate::test_support::engine;
use engine_core::extend::jip_patches;

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// Stand-in code addresses: a node's `CreateClone`, `ProcessClone`, `DetachChild`, and a
/// reference's own slot `+0x100` (not the shared `return true`).
const CREATE_CLONE: u32 = 0x00e0_0010;
const PROCESS_CLONE: u32 = 0x00e0_0020;
const DETACH_CHILD: u32 = 0x00e0_0030;
const OWN_3D_TEST: u32 = 0x00e0_0040;

/// Every game function the patches call, recorded (the log) and answering 0, but the model
/// map's find (writes the model at `MODEL_OUT` into its out word), `CreateClone` (answers the
/// word at `COPY_WORD`) and TES's load (records its memory context).
fn world() -> Engine {
    let mut e = engine();
    for page in [
        0x011c_3000u32,
        0x011c_d000,
        0x011d_e000,
        0x011e_0000,
        0x0101_6000,
    ] {
        e.map(page, 0x1000);
    }
    for addr in [
        CLONING_CONSTRUCT,
        CLONING_DESTRUCT,
        LOAD_ADDON_NODES,
        REFERENCE_ATTACHED,
        QUEUED_COUNT_DOWN,
        CRITICAL_COUNT_DOWN,
        LOCK_ENTER,
        LOCK_LEAVE,
        SET_3D,
        BIPED_DELETE,
        ANIMATION_DELETE,
        REPARENT_WEAPON,
        PROCESS_CLONE,
        DETACH_CHILD,
        OWN_3D_TEST,
    ] {
        e.register(addr, |_, _| Ret::default());
    }
    // TES's load records the memory context it ran in (in the word at the task's +0x3C).
    e.register(TES_LOAD_REFERENCE, |e, a| {
        let context = e.tls() + TLS_MEMORY_CONTEXT;
        let seen = e.mem.u32(context);
        e.mem.set_u32(a[3] + 0x3c, seen);
        Ret::default()
    });
    e.register(MODEL_MAP_FIND, |e, a| {
        let model = e.mem.u32(MODEL_OUT);
        e.mem.set_u32(a[2], model);
        ret(u32::from(model != 0))
    });
    e.register(CREATE_CLONE, |e, _| ret(e.mem.u32(COPY_WORD)));
    e.call_log = Some(Vec::new());
    e
}

/// Where the tests keep the model the find answers and the copy `CreateClone` answers.
const MODEL_OUT: u32 = 0x011e_0f00;
const COPY_WORD: u32 = 0x011e_0f04;

fn calls(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
    e.call_log
        .as_ref()
        .unwrap()
        .iter()
        .filter(|(called, _)| *called == addr)
        .map(|(_, words)| words.clone())
        .collect()
}

/// The order of the calls to `addrs` in the log.
fn order(e: &Engine, addrs: &[u32]) -> Vec<u32> {
    e.call_log
        .as_ref()
        .unwrap()
        .iter()
        .map(|(called, _)| *called)
        .filter(|called| addrs.contains(called))
        .collect()
}

fn vtable(e: &mut Engine, slots: &[(u32, u32)]) -> u32 {
    let table = e.mem.alloc(0x200);
    for &(offset, target) in slots {
        e.mem.set_u32(table + offset, target);
    }
    table
}

// ---- install ----

/// Not installed (with the row on): no seam handler, no slot written. Installed with the row
/// off (the vanilla preset): both seams and the four slots, whose old words were the exe's
/// attaches (patches of JIP's own, no fix rows).
#[test]
fn installed_writes_the_four_slots_and_the_two_seams() {
    let held = [0x0044_0ba0u32, 0x0044_1800, 0x0044_0ba0];
    let fill = |e: &mut Engine| {
        e.map(0x0101_6000, 0x1000);
        for (slot, word) in QUEUED_REFERENCE_SLOTS.iter().zip(held) {
            e.mem.set_u32(*slot, word);
        }
        e.mem.set_u32(QUEUED_PLAYER_SLOT, 0x0044_1c30);
    };
    let mut e = engine();
    fill(&mut e);
    crate::install_as(&mut e, false);
    super::super::set_row(&mut e, true);
    assert!(e.site_owners(FIRST_PERSON_MODEL.addr()).is_empty());
    assert!(e.site_owners(EYE_HEIGHT.addr()).is_empty());
    for (slot, word) in QUEUED_REFERENCE_SLOTS.iter().zip(held) {
        assert_eq!(e.mem.u32(*slot), word);
    }
    assert_eq!(e.mem.u32(QUEUED_PLAYER_SLOT), 0x0044_1c30);
    // The seams in a translation do nothing then (the word is 0).
    assert_eq!(e.jip_patches(), 0);

    let mut e = engine();
    fill(&mut e);
    crate::install_as(&mut e, true);
    assert_eq!(e.jip_patches(), jip_patches::INSTALLED);
    assert_eq!(e.site_owners(FIRST_PERSON_MODEL.addr()), [Owner::Jip]);
    assert_eq!(e.site_owners(EYE_HEIGHT.addr()), [Owner::Jip]);
    for slot in QUEUED_REFERENCE_SLOTS {
        assert_eq!(e.mem.u32(slot), QUEUED_REFERENCE_ROUTINE);
    }
    assert_eq!(e.mem.u32(QUEUED_PLAYER_SLOT), QUEUED_PLAYER_ROUTINE);
    assert!(e.is_registered(QUEUED_REFERENCE_ROUTINE));
    assert!(e.is_registered(QUEUED_PLAYER_ROUTINE));
    // QueuedActor's and QueuedTree's slots are not JIP's.
    assert_eq!(e.mem.u32(0x0101_6ce0), 0);
    assert_eq!(e.mem.u32(0x0101_6c98), 0);
}

// ---- G38: the first-person model ----

/// A model in the map with `refs` references and a node whose clone is a fresh object.
fn cached_model(e: &mut Engine, refs: i32) -> (u32, u32, u32) {
    let node = e.mem.alloc(0x100);
    let table = vtable(e, &[(0x48, CREATE_CLONE), (0x68, PROCESS_CLONE)]);
    e.mem.set_u32(node, table);
    let copy = e.mem.alloc(0x100);
    e.mem.set_u8(copy + 0x30, 0xff);
    e.mem.set_u8(copy + 0x32, 0x01);
    for i in 0..12 {
        e.mem.set_f32(copy + 0x34 + 4 * i, 7.0);
    }
    e.mem.set_u32(COPY_WORD, copy);
    let model = e.mem.alloc(0x20);
    e.mem.set_u32(model + 8, refs as u32);
    e.mem.set_u32(model + 0xc, node);
    e.mem.set_u32(MODEL_OUT, model);
    let loader = e.mem.alloc(0x10);
    e.mem.set_u32(loader, 0x00a0_0000);
    e.mem.set_u32(MODEL_LOADER, loader);
    e.mem.set_u32(FIRST_PERSON_PATH, 0x00a1_0000);
    (model, node, copy)
}

/// Found in the map: the find asked on the loader's map with the setting's text, the copy made
/// through a cloning process of 0x97 buckets and scale 1 (constructed, both clone calls with
/// it, destroyed), shown, identity rotation, no translation, flagged 0x200000 (byte +0x32 bit
/// 0x20, the rest kept), kept as JIP's node, answered.
#[test]
fn the_first_person_model_is_a_copy_of_the_cached_one() {
    let mut e = world();
    let (model, node, copy) = cached_model(&mut e, 3);
    assert_eq!(first_person_node(&mut e), 0);
    assert_eq!(first_person_model(&mut e, &[0x5000]), Some(copy));
    let find = calls(&e, MODEL_MAP_FIND);
    assert_eq!(find.len(), 1);
    assert_eq!(find[0][..2], [0x00a0_0000, 0x00a1_0000]);
    let info = calls(&e, CLONING_CONSTRUCT)[0][0];
    assert_eq!(calls(&e, CLONING_CONSTRUCT), vec![vec![info, 0x97]]);
    assert_eq!(calls(&e, CREATE_CLONE), vec![vec![node, info]]);
    assert_eq!(calls(&e, PROCESS_CLONE), vec![vec![node, info]]);
    assert_eq!(calls(&e, CLONING_DESTRUCT), vec![vec![info]]);
    assert_eq!(
        order(
            &e,
            &[
                CLONING_CONSTRUCT,
                CREATE_CLONE,
                PROCESS_CLONE,
                CLONING_DESTRUCT
            ]
        ),
        [
            CLONING_CONSTRUCT,
            CREATE_CLONE,
            PROCESS_CLONE,
            CLONING_DESTRUCT
        ]
    );
    assert_eq!(e.mem.u8(copy + 0x30), 0xfe, "shown");
    assert_eq!(e.mem.u8(copy + 0x32), 0x21, "flag 0x200000 added");
    let expected = [
        1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0,
    ];
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(e.mem.f32(copy + 0x34 + 4 * i as u32), *want, "word {i}");
    }
    assert_eq!(e.mem.u32(model + 8), 3, "a counted model keeps its count");
    assert_eq!(first_person_node(&mut e), copy);
}

/// The cloning process's scale words are 1 when the clone runs (JIP's constructor sets them).
#[test]
fn the_copy_runs_with_scale_one() {
    let mut e = world();
    cached_model(&mut e, 1);
    e.register(CREATE_CLONE, |e, a| {
        let info = a[1];
        let scale: Vec<f32> = (0..3).map(|i| e.mem.f32(info + 0x10 + 4 * i)).collect();
        assert_eq!(scale, [1.0, 1.0, 1.0]);
        ret(e.mem.u32(COPY_WORD))
    });
    assert!(first_person_model(&mut e, &[0]).is_some());
}

/// The model's count: 0 becomes 1, -1 becomes 0 (one `lock inc`), 1 stays.
#[test]
fn an_uncounted_model_gets_one_reference() {
    for (before, after) in [(0, 1), (-1, 0), (1, 1), (i32::MIN, i32::MIN + 1)] {
        let mut e = world();
        let (model, _, _) = cached_model(&mut e, before);
        first_person_model(&mut e, &[0]);
        assert_eq!(e.mem.u32(model + 8) as i32, after, "{before}");
    }
}

/// Not in the map: JIP would read a null model; no answer here (the exe's load), no copy, the
/// kept node unchanged.
#[test]
fn a_model_not_in_the_map_leaves_the_exes_load() {
    let mut e = world();
    cached_model(&mut e, 1);
    e.mem.set_u32(MODEL_OUT, 0);
    assert_eq!(first_person_model(&mut e, &[0]), None);
    assert!(calls(&e, CLONING_CONSTRUCT).is_empty());
    assert_eq!(first_person_node(&mut e), 0);
}

/// A find that misses and writes nothing: the out word is zeroed before the find (JIP pushes a
/// 0), so stale stack contents never pass for a model: no answer, the exe's load.
#[test]
fn a_miss_that_writes_nothing_reads_the_zeroed_out_word() {
    let mut e = world();
    cached_model(&mut e, 1);
    // Leave a non-zero word where the out word will sit.
    e.with_stack(4, |e, out| e.mem.set_u32(out.addr(), 0x00de_ad00));
    e.register(MODEL_MAP_FIND, |e, a| {
        assert_eq!(e.mem.u32(a[2]), 0, "zeroed before the find");
        ret(0)
    });
    assert_eq!(first_person_model(&mut e, &[0]), None);
    assert!(calls(&e, CLONING_CONSTRUCT).is_empty());
    assert_eq!(first_person_node(&mut e), 0);
}

/// The eye height's seam always answers (JIP's jump has no condition).
#[test]
fn the_eye_height_is_always_skipped() {
    let mut e = world();
    assert_eq!(eye_height(&mut e, &[0x5000]), Some(1));
}

// ---- H09: the queued reference's attach ----

struct Task {
    task: u32,
    refr: u32,
    cell: u32,
    model: u32,
    node: u32,
}

/// A task (state 0, its memory context 0x77) for a reference with a parent cell in state 2,
/// a model, a fade node as 3D (`+0x10` the shared `return this`), its slot `+0x100` its own.
fn task(e: &mut Engine) -> Task {
    let task = e.mem.alloc(0x40);
    // A player's size: the player tests reach `+0xDD4`.
    let refr = e.mem.alloc(0x1000);
    let cell = e.mem.alloc(0x100);
    let model = e.mem.alloc(0x20);
    let model_node = e.mem.alloc(0x10);
    e.mem.set_u32(model + 0xc, model_node);
    e.mem.set_u32(task + 0x18, 0x77);
    e.mem.set_u32(task + 0x28, refr);
    e.mem.set_u32(task + 0x30, model);
    let refr_table = vtable(e, &[(0x100, OWN_3D_TEST)]);
    e.mem.set_u32(refr, refr_table);
    e.mem.set_u32(refr + 0x40, cell);
    e.mem.set_u8(cell + 0x26, 2);
    let node = e.mem.alloc(0x100);
    let node_table = vtable(e, &[(0x10, RETURN_THIS)]);
    e.mem.set_u32(node, node_table);
    e.mem.set_f32(node + 0xb4, 0.25);
    e.mem.set_f32(node + 0xb8, 0.5);
    let loaded = e.mem.alloc(0x20);
    e.mem.set_u32(loaded + 0x14, node);
    e.mem.set_u32(refr + 0x64, loaded);
    let loader = e.mem.alloc(0x10);
    e.mem.set_u32(loader + 8, 0x00b0_0000);
    e.mem.set_u32(MODEL_LOADER, loader);
    e.mem.set_u32(TES, 0x00c0_0000);
    Task {
        task,
        refr,
        cell,
        model: model_node,
        node,
    }
}

/// The whole way: context set (seen by TES's load) and put back, the lock taken, addon nodes
/// with the model's node, TES's load, the loader told, both counts, the lock let go, in JIP's
/// order; no mark: the fade untouched.
#[test]
fn the_attach_loads_the_reference_in_jips_order() {
    let mut e = world();
    let t = task(&mut e);
    let context = e.tls() + TLS_MEMORY_CONTEXT;
    e.mem.set_u32(context, 0x55);
    queued_reference(&mut e, t.task);
    assert_eq!(
        e.mem.u32(t.task + 0x3c),
        0x77,
        "TES's load in the task's context"
    );
    assert_eq!(e.mem.u32(context), 0x55, "put back");
    let lock = t.cell + 0x80;
    assert_eq!(calls(&e, LOCK_ENTER), vec![vec![lock, 0]]);
    assert_eq!(
        calls(&e, LOAD_ADDON_NODES),
        vec![vec![e.mem.u32(MODEL_LOADER), t.refr, t.model]]
    );
    assert_eq!(
        calls(&e, TES_LOAD_REFERENCE),
        vec![vec![0x00c0_0000, t.refr, t.cell, t.task, 0]]
    );
    assert_eq!(
        calls(&e, REFERENCE_ATTACHED),
        vec![vec![0x00b0_0000, t.refr]]
    );
    assert_eq!(calls(&e, QUEUED_COUNT_DOWN), vec![vec![t.cell]]);
    assert_eq!(calls(&e, CRITICAL_COUNT_DOWN), vec![vec![t.cell]]);
    assert_eq!(calls(&e, LOCK_LEAVE), vec![vec![lock]]);
    assert_eq!(
        order(
            &e,
            &[
                LOCK_ENTER,
                LOAD_ADDON_NODES,
                TES_LOAD_REFERENCE,
                REFERENCE_ATTACHED,
                QUEUED_COUNT_DOWN,
                CRITICAL_COUNT_DOWN,
                LOCK_LEAVE
            ]
        ),
        [
            LOCK_ENTER,
            LOAD_ADDON_NODES,
            TES_LOAD_REFERENCE,
            REFERENCE_ATTACHED,
            QUEUED_COUNT_DOWN,
            CRITICAL_COUNT_DOWN,
            LOCK_LEAVE
        ]
    );
    assert_eq!(e.mem.f32(t.node + 0xb4), 0.25);
    assert_eq!(e.mem.f32(t.node + 0xb8), 0.5);
}

/// A task in state 6: nothing at all, the context untouched.
#[test]
fn a_cancelled_task_does_nothing() {
    let mut e = world();
    let t = task(&mut e);
    e.mem.set_u32(t.task + 0xc, 6);
    e.mem.set_u8(t.refr + 0x5f, REF_FLAG_UPDATE_3D);
    let context = e.tls() + TLS_MEMORY_CONTEXT;
    e.mem.set_u32(context, 0x55);
    queued_reference(&mut e, t.task);
    assert!(e.call_log.as_ref().unwrap().is_empty());
    assert_eq!(e.mem.u8(t.refr + 0x5f), REF_FLAG_UPDATE_3D);
    // Every other state runs.
    for state in [0u32, 5, 7] {
        let mut e = world();
        let t = task(&mut e);
        e.mem.set_u32(t.task + 0xc, state);
        queued_reference(&mut e, t.task);
        assert_eq!(calls(&e, TES_LOAD_REFERENCE).len(), 1, "{state}");
    }
}

/// A reference flagged 0x800 or 0x20: no lock, no load; the loader told and both counts
/// lowered; the lock not let go (JIP lets it go: a deliberate difference).
#[test]
fn a_flagged_reference_is_not_loaded() {
    for flag in [0x800u32, 0x20, 0x820] {
        let mut e = world();
        let t = task(&mut e);
        e.mem.set_u32(t.refr + 8, flag | 0x4);
        queued_reference(&mut e, t.task);
        assert!(calls(&e, LOCK_ENTER).is_empty(), "{flag:x}");
        assert!(calls(&e, TES_LOAD_REFERENCE).is_empty());
        assert!(calls(&e, LOAD_ADDON_NODES).is_empty());
        assert_eq!(calls(&e, REFERENCE_ATTACHED).len(), 1);
        assert_eq!(calls(&e, QUEUED_COUNT_DOWN), vec![vec![t.cell]]);
        assert!(calls(&e, LOCK_LEAVE).is_empty());
    }
    // Other form flags: loaded.
    let mut e = world();
    let t = task(&mut e);
    e.mem.set_u32(t.refr + 8, !0x820);
    queued_reference(&mut e, t.task);
    assert_eq!(calls(&e, TES_LOAD_REFERENCE).len(), 1);
}

/// No parent cell: no lock, no load, the loader told, no counts, no fade work.
#[test]
fn a_reference_with_no_cell_is_only_told() {
    let mut e = world();
    let t = task(&mut e);
    e.mem.set_u32(t.refr + 0x40, 0);
    e.mem.set_u8(t.refr + 0x5f, REF_FLAG_UPDATE_3D);
    queued_reference(&mut e, t.task);
    assert!(calls(&e, LOCK_ENTER).is_empty());
    assert!(calls(&e, TES_LOAD_REFERENCE).is_empty());
    assert_eq!(calls(&e, REFERENCE_ATTACHED).len(), 1);
    assert!(calls(&e, QUEUED_COUNT_DOWN).is_empty());
    assert!(calls(&e, LOCK_LEAVE).is_empty());
    assert_eq!(e.mem.u8(t.refr + 0x5f), REF_FLAG_UPDATE_3D, "kept");
}

/// The cell's state: below 2 (0, 1) the lock is taken and let go but nothing loads; 2 and
/// above (3, 0xFF) load.
#[test]
fn only_a_cell_in_state_two_or_more_loads() {
    for (state, loads) in [(0u8, false), (1, false), (2, true), (3, true), (0xff, true)] {
        let mut e = world();
        let t = task(&mut e);
        e.mem.set_u8(t.cell + 0x26, state);
        queued_reference(&mut e, t.task);
        assert_eq!(
            calls(&e, TES_LOAD_REFERENCE).len(),
            usize::from(loads),
            "{state}"
        );
        assert_eq!(calls(&e, LOAD_ADDON_NODES).len(), usize::from(loads));
        assert_eq!(calls(&e, LOCK_ENTER).len(), 1);
        assert_eq!(calls(&e, LOCK_LEAVE).len(), 1);
    }
}

/// The lock let go is the one taken, also when TES's load moves the reference to another cell
/// (the counts go to the new cell, read again as JIP does) or to none (the lock still let go;
/// JIP and the exe let go the re-read cell's lock: a deliberate difference).
#[test]
fn the_lock_let_go_is_the_one_taken() {
    let mut e = world();
    let t = task(&mut e);
    let other = e.mem.alloc(0x100);
    e.mem.set_u32(0x011e_0f08, other);
    e.register(TES_LOAD_REFERENCE, |e, a| {
        let other = e.mem.u32(0x011e_0f08);
        e.mem.set_u32(a[1] + 0x40, other);
        Ret::default()
    });
    queued_reference(&mut e, t.task);
    assert_eq!(calls(&e, LOCK_ENTER), vec![vec![t.cell + 0x80, 0]]);
    assert_eq!(calls(&e, LOCK_LEAVE), vec![vec![t.cell + 0x80]]);
    assert_eq!(calls(&e, QUEUED_COUNT_DOWN), vec![vec![other]]);

    let mut e = world();
    let t = task(&mut e);
    e.register(TES_LOAD_REFERENCE, |e, a| {
        e.mem.set_u32(a[1] + 0x40, 0);
        Ret::default()
    });
    queued_reference(&mut e, t.task);
    assert!(calls(&e, QUEUED_COUNT_DOWN).is_empty());
    assert_eq!(calls(&e, LOCK_LEAVE), vec![vec![t.cell + 0x80]]);
}

/// No model: TES's load without the addon nodes.
#[test]
fn a_task_with_no_model_skips_the_addon_nodes() {
    let mut e = world();
    let t = task(&mut e);
    e.mem.set_u32(t.task + 0x30, 0);
    queued_reference(&mut e, t.task);
    assert!(calls(&e, LOAD_ADDON_NODES).is_empty());
    assert_eq!(calls(&e, TES_LOAD_REFERENCE).len(), 1);
}

/// The critical count: lowered unless the reference's slot `+0x100` is the shared `return true`
/// (compared as a word, never called).
#[test]
fn the_critical_count_follows_the_slot_word() {
    let mut e = world();
    let t = task(&mut e);
    let table = e.mem.u32(t.refr);
    e.mem.set_u32(table + 0x100, RETURN_TRUE);
    queued_reference(&mut e, t.task);
    assert!(calls(&e, CRITICAL_COUNT_DOWN).is_empty());
    assert_eq!(calls(&e, QUEUED_COUNT_DOWN).len(), 1);
    assert!(calls(&e, OWN_3D_TEST).is_empty());
    assert!(calls(&e, RETURN_TRUE).is_empty());
}

/// JIP's mark with a fade node: the mark goes (JIP's other flags stay), the fade is (1, 1).
/// With a node whose `+0x10` is not `return this`: the mark goes, the fade stays.
#[test]
fn the_update_3d_mark_is_read_after_the_load() {
    let mut e = world();
    let t = task(&mut e);
    e.mem.set_u8(t.refr + 0x5f, REF_FLAG_UPDATE_3D | 0x1e);
    queued_reference(&mut e, t.task);
    assert_eq!(e.mem.u8(t.refr + 0x5f), 0x1e);
    assert_eq!(e.mem.f32(t.node + 0xb4), 1.0);
    assert_eq!(e.mem.f32(t.node + 0xb8), 1.0);
    assert_eq!(e.mem.u32(t.node + 0xbc), 0, "two floats only");

    let mut e = world();
    let t = task(&mut e);
    let table = e.mem.u32(t.node);
    e.mem.set_u32(table + 0x10, 0x00e0_0060);
    e.mem.set_u8(t.refr + 0x5f, REF_FLAG_UPDATE_3D);
    queued_reference(&mut e, t.task);
    assert_eq!(e.mem.u8(t.refr + 0x5f), 0);
    assert_eq!(e.mem.f32(t.node + 0xb4), 0.25);

    // Other flags alone: nothing changes.
    let mut e = world();
    let t = task(&mut e);
    e.mem.set_u8(t.refr + 0x5f, 0xfe);
    queued_reference(&mut e, t.task);
    assert_eq!(e.mem.u8(t.refr + 0x5f), 0xfe);
    assert_eq!(e.mem.f32(t.node + 0xb4), 0.25);
}

/// The mark stays while the reference has no 3D: no loaded data, or loaded data with no root.
#[test]
fn the_mark_waits_for_the_3d() {
    for no_root in [false, true] {
        let mut e = world();
        let t = task(&mut e);
        if no_root {
            let loaded = e.mem.u32(t.refr + 0x64);
            e.mem.set_u32(loaded + 0x14, 0);
        } else {
            e.mem.set_u32(t.refr + 0x64, 0);
        }
        e.mem.set_u8(t.refr + 0x5f, REF_FLAG_UPDATE_3D);
        queued_reference(&mut e, t.task);
        assert_eq!(e.mem.u8(t.refr + 0x5f), REF_FLAG_UPDATE_3D);
        assert_eq!(calls(&e, LOCK_LEAVE).len(), 1, "the lock still let go");
    }
}

/// Through the slot: the routine at the written address runs the attach with the task.
#[test]
fn the_slot_routines_run_the_attaches() {
    let mut e = world();
    super::install_jip(&mut e);
    let t = task(&mut e);
    let routine = e.mem.u32(QUEUED_REFERENCE_SLOTS[0]);
    let table = vtable(&mut e, &[(0x3c, routine)]);
    e.mem.set_u32(t.task, table);
    e.vcall(t.task, 0x3c, &args![]);
    assert_eq!(calls(&e, TES_LOAD_REFERENCE).len(), 1);
    e.mem.set_u32(table + 0x3c, e.mem.u32(QUEUED_PLAYER_SLOT));
    e.vcall(t.task, 0x3c, &args![]);
    assert_eq!(calls(&e, TES_LOAD_REFERENCE).len(), 2);
    assert!(
        calls(&e, SET_3D).is_empty(),
        "no mark: the reference's attach"
    );
}

// ---- G38: the queued player ----

struct Player {
    t: Task,
    biped: u32,
    animation: u32,
    first: u32,
    parent: u32,
    process: u32,
}

/// The player's task, marked, with a first-person biped, animation and node (in a parent with
/// a `DetachChild`), a high process holding a weapon, and the camera globals and `+0xD74` set.
fn player(e: &mut Engine) -> Player {
    let t = task(e);
    let p = t.refr;
    e.mem.set_u8(p + 0x5f, REF_FLAG_UPDATE_3D | 0x10);
    let biped = e.mem.alloc(0x10);
    let animation = e.mem.alloc(0x10);
    let first = e.mem.alloc(0x100);
    let parent = e.mem.alloc(0x100);
    let parent_table = vtable(e, &[(0xe8, DETACH_CHILD)]);
    e.mem.set_u32(parent, parent_table);
    e.mem.set_u32(first + 0x18, parent);
    e.mem.set_u8(first + 0x30, 0x41);
    e.mem.set_u8(t.node + 0x30, 0x40);
    e.mem.set_u32(p + 0x68c, biped);
    e.mem.set_u32(p + 0x690, animation);
    e.mem.set_u32(p + 0x694, first);
    e.mem.set_u32(p + 0x1b4, 0x00d0_0001);
    let process = e.mem.alloc(0x200);
    e.mem.set_u32(process + 0x114, 0x00d0_0002);
    e.mem.set_u32(process + 0x1c0, 0x00d0_0003);
    e.mem.set_u32(p + 0x68, process);
    for word in 0..4 {
        e.mem.set_u32(CAMERA_NODES + 4 * word, 0x99);
    }
    e.mem.set_u32(CAMERA_NODES + 0x10, 0x99);
    for word in 0..0x19 {
        e.mem.set_u32(p + 0xd74 + 4 * word, 0x99);
    }
    Player {
        t,
        biped,
        animation,
        first,
        parent,
        process,
    }
}

/// Without the mark: the reference's attach and nothing of the player's.
#[test]
fn an_unmarked_player_is_attached_as_a_reference() {
    let mut e = world();
    let p = player(&mut e);
    e.mem.set_u8(p.t.refr + 0x5f, 0x10);
    queued_player(&mut e, p.t.task);
    assert!(calls(&e, SET_3D).is_empty());
    assert!(calls(&e, BIPED_DELETE).is_empty());
    assert!(calls(&e, REPARENT_WEAPON).is_empty());
    assert_eq!(calls(&e, TES_LOAD_REFERENCE).len(), 1);
    assert_eq!(e.mem.u32(p.t.refr + 0x68c), p.biped);
    assert_eq!(e.mem.u32(CAMERA_NODES), 0x99);
    assert_eq!(e.mem.u8(p.first + 0x30), 0x41);
}

/// Marked: the old models go first (Set3D(0, 1), the biped and the animation deleted and
/// cleared, the node taken from its parent), the camera globals and `+0xD74`'s 0x60 bytes
/// zeroed (only those), the mark cleared before the attach; then in first person the first-person
/// node shown and the third person one hidden, and the weapon put back on both skeletons.
#[test]
fn a_marked_player_rebuilds_his_models() {
    let mut e = world();
    let p = player(&mut e);
    let me = p.t.refr;
    // The mark is gone when TES loads, and the fresh first-person biped is what the second
    // ReparentWeapon gets.
    e.register(TES_LOAD_REFERENCE, |e, a| {
        assert_eq!(e.mem.u8(a[1] + 0x5f) & REF_FLAG_UPDATE_3D, 0);
        e.mem.set_u32(a[1] + 0x68c, 0x00d0_0011);
        e.mem.set_u32(a[1] + 0x690, 0x00d0_0012);
        Ret::default()
    });
    e.mem.set_u8(me + 0x64a, 0);
    queued_player(&mut e, p.t.task);
    assert_eq!(calls(&e, SET_3D), vec![vec![me, 0, 1]]);
    assert_eq!(calls(&e, BIPED_DELETE), vec![vec![p.biped, 1]]);
    assert_eq!(calls(&e, ANIMATION_DELETE), vec![vec![p.animation, 1]]);
    assert_eq!(calls(&e, DETACH_CHILD), vec![vec![p.parent, p.first]]);
    assert_eq!(
        order(
            &e,
            &[
                SET_3D,
                BIPED_DELETE,
                ANIMATION_DELETE,
                DETACH_CHILD,
                TES_LOAD_REFERENCE
            ]
        ),
        [
            SET_3D,
            BIPED_DELETE,
            ANIMATION_DELETE,
            DETACH_CHILD,
            TES_LOAD_REFERENCE
        ]
    );
    for word in 0..4 {
        assert_eq!(e.mem.u32(CAMERA_NODES + 4 * word), 0);
    }
    assert_eq!(e.mem.u32(CAMERA_NODES + 0x10), 0x99, "four globals only");
    for word in 0..0x18 {
        assert_eq!(e.mem.u32(me + 0xd74 + 4 * word), 0);
    }
    assert_eq!(e.mem.u32(me + 0xd74 + 0x60), 0x99, "0x60 bytes only");
    assert_eq!(e.mem.u8(me + 0x5f), 0x10);
    // First person: the first-person node shown, the third person 3D hidden.
    assert_eq!(e.mem.u8(p.first + 0x30), 0x40);
    assert_eq!(e.mem.u8(p.t.node + 0x30), 0x41);
    assert_eq!(
        calls(&e, REPARENT_WEAPON),
        vec![
            vec![p.process, 0, 0x00d0_0001, 0x00d0_0003, me],
            vec![p.process, 0, 0x00d0_0011, 0x00d0_0012, me],
        ]
    );
}

/// The deleted first-person biped and animation words are cleared before the load runs; and
/// when nothing is loaded (the cell in state 1) they stay 0, never pointing at freed memory.
#[test]
fn the_deleted_biped_and_animation_are_cleared() {
    let mut e = world();
    let p = player(&mut e);
    e.register(TES_LOAD_REFERENCE, |e, a| {
        assert_eq!(e.mem.u32(a[1] + 0x68c), 0, "biped cleared before the load");
        assert_eq!(
            e.mem.u32(a[1] + 0x690),
            0,
            "animation cleared before the load"
        );
        Ret::default()
    });
    queued_player(&mut e, p.t.task);
    assert_eq!(calls(&e, TES_LOAD_REFERENCE).len(), 1);

    let mut e = world();
    let p = player(&mut e);
    e.mem.set_u8(p.t.cell + 0x26, 1);
    queued_player(&mut e, p.t.task);
    assert!(calls(&e, TES_LOAD_REFERENCE).is_empty());
    assert_eq!(calls(&e, BIPED_DELETE), vec![vec![p.biped, 1]]);
    assert_eq!(calls(&e, ANIMATION_DELETE), vec![vec![p.animation, 1]]);
    assert_eq!(e.mem.u32(p.t.refr + 0x68c), 0);
    assert_eq!(e.mem.u32(p.t.refr + 0x690), 0);
}

/// In third person the other way round.
#[test]
fn in_third_person_the_third_person_model_is_shown() {
    for byte in [1u8, 0xff] {
        let mut e = world();
        let p = player(&mut e);
        e.mem.set_u8(p.t.refr + 0x64a, byte);
        e.mem.set_u8(p.t.node + 0x30, 0x41);
        e.mem.set_u8(p.first + 0x30, 0x40);
        queued_player(&mut e, p.t.task);
        assert_eq!(e.mem.u8(p.first + 0x30), 0x41);
        assert_eq!(e.mem.u8(p.t.node + 0x30), 0x40);
    }
}

/// The weapon goes back only for a high process (level 0) holding a weapon or with its byte
/// `+0x135` set (then passed as the first argument); no process, a lower level, or neither:
/// no call. The views are set either way.
#[test]
fn the_weapon_goes_back_only_for_a_high_process_with_one() {
    let cases: [(bool, u8, u32, u8, usize); 6] = [
        (true, 0, 0x00d0_0002, 0, 2),
        (true, 0, 0, 1, 2),
        (true, 0, 0, 0, 0),
        (true, 1, 0x00d0_0002, 0, 0),
        (true, 3, 0x00d0_0002, 1, 0),
        (false, 0, 0x00d0_0002, 1, 0),
    ];
    for (has, level, weapon, byte, count) in cases {
        let mut e = world();
        let p = player(&mut e);
        if !has {
            e.mem.set_u32(p.t.refr + 0x68, 0);
        }
        e.mem.set_u8(p.process + 0x28, level);
        e.mem.set_u32(p.process + 0x114, weapon);
        e.mem.set_u8(p.process + 0x135, byte);
        queued_player(&mut e, p.t.task);
        let made = calls(&e, REPARENT_WEAPON);
        assert_eq!(made.len(), count, "{has} {level} {weapon:x} {byte}");
        for call in made {
            assert_eq!(call[1], u32::from(byte));
        }
        assert_eq!(e.mem.u8(p.first + 0x30), 0x40, "shown either way");
    }
}

/// Nothing to throw away (no biped, animation, node, or a node with no parent): no call for
/// it; after the load no third person 3D or no first-person node: no view set, no weapon.
#[test]
fn a_marked_player_with_missing_parts() {
    let mut e = world();
    let p = player(&mut e);
    for offset in [0x68cu32, 0x690] {
        e.mem.set_u32(p.t.refr + offset, 0);
    }
    e.mem.set_u32(p.first + 0x18, 0);
    queued_player(&mut e, p.t.task);
    assert!(calls(&e, BIPED_DELETE).is_empty());
    assert!(calls(&e, ANIMATION_DELETE).is_empty());
    assert!(calls(&e, DETACH_CHILD).is_empty());
    assert_eq!(calls(&e, SET_3D).len(), 1);

    let mut e = world();
    let p = player(&mut e);
    e.mem.set_u32(p.t.refr + 0x694, 0);
    queued_player(&mut e, p.t.task);
    assert!(calls(&e, DETACH_CHILD).is_empty());
    assert!(calls(&e, REPARENT_WEAPON).is_empty());
    assert_eq!(e.mem.u8(p.t.node + 0x30), 0x40, "no view set");

    for no_root in [false, true] {
        let mut e = world();
        let p = player(&mut e);
        if no_root {
            let loaded = e.mem.u32(p.t.refr + 0x64);
            e.mem.set_u32(loaded + 0x14, 0);
        } else {
            e.mem.set_u32(p.t.refr + 0x64, 0);
        }
        queued_player(&mut e, p.t.task);
        assert!(calls(&e, REPARENT_WEAPON).is_empty());
        assert_eq!(e.mem.u8(p.first + 0x30), 0x41, "no view set");
    }
}
