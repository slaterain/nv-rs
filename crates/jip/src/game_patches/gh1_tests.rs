//! Group GH1's patches: G44, the ExtraJIP framework, case by case from JIP's routines.

use super::*;
use crate::cosave::{ExtraJipData, ExtraJipEntry};
use crate::test_support::engine;
use engine_core::extend::jip_patches;
use std::cell::RefCell;
use std::collections::BTreeMap;

/// The game's functions the routines call, as JIP names them.
const GET_BY_TYPE: u32 = 0x0041_0220;
const ADD_EXTRA: u32 = 0x0040_ff60;
const REMOVE_BY_TYPE: u32 = 0x0041_0140;
const HEAP_ALLOCATE: u32 = 0x00aa_3e40;
const HEAP_FREE: u32 = 0x00aa_4060;
/// Where the test's blocks, lists and buffers are.
const HEAP: u32 = 0x0600_0000;
const LIST: u32 = 0x0610_0000;
const SOURCE_LIST: u32 = 0x0610_0100;
const BUFFER: u32 = 0x0620_0000;
const CHUNK: u32 = 0x0621_0000;
const FORM: u32 = 0x0622_0000;
/// The game's map of forms and the save's map of changes (`cosave.rs`'s `ALL_FORMS`, the
/// `BGSSaveLoadGame` with its changes map at +0).
const ALL_FORMS: u32 = 0x011c_54c0;
const SAVE_LOAD_GAME: u32 = 0x011d_df38;
const SL: u32 = 0x0623_0000;
const FORMS: u32 = 0x0623_1000;
const CHANGES: u32 = 0x0623_2000;
const ENTRIES: u32 = 0x0623_3000;
/// What the test's stand-ins of the save buffer's grow and the header's id answer.
const HEADER_ID: u32 = 0x0012_3456;

thread_local! {
    /// The calls of the stand-ins, in order; the random words still to answer; the next block.
    static CALLS: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
    static RANDOM: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static NEXT: RefCell<u32> = const { RefCell::new(HEAP) };
}

fn log(addr: u32, a: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, a.to_vec())));
}

fn calls(addr: u32) -> Vec<Vec<u32>> {
    CALLS.with(|c| {
        c.borrow()
            .iter()
            .filter(|(at, _)| *at == addr)
            .map(|(_, a)| a.clone())
            .collect()
    })
}

fn eax(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// The list's extras in chain order (head at +4, next at +8).
fn chain(e: &Engine, list: u32) -> Vec<u32> {
    let mut out = vec![];
    let mut at = e.mem.u32(list + 4);
    while at != 0 {
        out.push(at);
        at = e.mem.u32(at + 8);
    }
    out
}

/// An engine with the game's list, heap, random and save buffer functions stood in, the form
/// and change maps empty, and JIP installed (`answers_installed`) with the row off.
fn world(answers_installed: bool) -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    RANDOM.with(|r| r.borrow_mut().clear());
    NEXT.with(|n| *n.borrow_mut() = HEAP);
    let mut e = engine();
    e.map(HEAP, 0x0030_0000);
    for page in [0x011c_5000, 0x011d_d000, 0x011c_4000] {
        e.map(page, 0x1000);
    }
    e.mem.set_u32(SAVE_LOAD_GAME, SL);
    e.mem.set_u32(SL, CHANGES);
    for map in [CHANGES, FORMS] {
        e.mem.set_u32(map + 4, 16);
        e.mem.set_u32(map + 8, map + 0x100);
    }
    e.mem.set_u32(ALL_FORMS, FORMS);
    // The vtable's two words as the exe holds them.
    e.mem.set_u32(DESTROY_SLOT, 0x0041_b680);
    e.mem.set_u32(COMPARE_SLOT, 0x0043_1ef0);
    e.register(GET_BY_TYPE, |e, a| {
        let found = chain(e, a[0])
            .into_iter()
            .find(|&x| u32::from(e.mem.u8(x + 4)) == a[1])
            .unwrap_or(0);
        eax(found)
    });
    e.register(ADD_EXTRA, |e, a| {
        log(ADD_EXTRA, a);
        let (list, extra) = (a[0], a[1]);
        match chain(e, list).last() {
            Some(&last) => e.mem.set_u32(last + 8, extra),
            None => e.mem.set_u32(list + 4, extra),
        }
        eax(extra)
    });
    e.register(REMOVE_BY_TYPE, |e, a| {
        log(REMOVE_BY_TYPE, a);
        let (list, kind) = (a[0], a[1]);
        let mut previous = 0;
        for x in chain(e, list) {
            if u32::from(e.mem.u8(x + 4)) == kind {
                let next = e.mem.u32(x + 8);
                if previous == 0 {
                    e.mem.set_u32(list + 4, next);
                } else {
                    e.mem.set_u32(previous + 8, next);
                }
                break;
            }
            previous = x;
        }
        Ret::default()
    });
    e.register(HEAP_ALLOCATE, |_, a| {
        log(HEAP_ALLOCATE, a);
        let at = NEXT.with(|n| {
            let at = *n.borrow();
            *n.borrow_mut() = at + 0x20;
            at
        });
        eax(at)
    });
    e.register(HEAP_FREE, |_, a| {
        log(HEAP_FREE, a);
        Ret::default()
    });
    e.register(GET_RANDOM_INT, |_, a| {
        log(GET_RANDOM_INT, a);
        eax(RANDOM.with(|r| r.borrow_mut().remove(0)))
    });
    e.register(SAVE_BUFFER_GROW, |e, a| {
        log(SAVE_BUFFER_GROW, a);
        let size = e.mem.u32(a[0] + 8);
        e.mem.set_u32(a[0] + 8, size + 0x100);
        Ret::default()
    });
    e.register(HEADER_REF_ID, |_, a| {
        log(HEADER_REF_ID, a);
        eax(HEADER_ID)
    });
    crate::install_as(&mut e, answers_installed);
    e
}

/// One of the game's maps (`NiTMap`, buckets at +8, count at +4) given `id` -> `value`.
fn insert(e: &mut Engine, map: u32, id: u32, value: u32) {
    let at = ENTRIES + 0x10 * (e.mem.u32(ENTRIES + 0xff0) + 1);
    e.mem
        .set_u32(ENTRIES + 0xff0, e.mem.u32(ENTRIES + 0xff0) + 1);
    let bucket = e.mem.u32(map + 8) + 4 * (id % e.mem.u32(map + 4));
    e.mem.set_u32(at, e.mem.u32(bucket));
    e.mem.set_u32(at + 4, id);
    e.mem.set_u32(at + 8, value);
    e.mem.set_u32(bucket, at);
}

/// A guest extra of `kind` with `word` at +0xC (not in a list).
fn extra_of(e: &mut Engine, kind: u8, word: u32) -> u32 {
    let at = NEXT.with(|n| {
        let at = *n.borrow();
        *n.borrow_mut() = at + 0x20;
        at
    });
    e.mem.set_u32(at, VTABLE);
    e.mem.set_u8(at + 4, kind);
    e.mem.set_u32(at + 8, 0);
    e.mem.set_u32(at + 0x0c, word);
    at
}

/// An extra put at the end of `list`.
fn add(e: &mut Engine, list: u32, extra: u32) {
    match chain(e, list).last() {
        Some(&last) => e.mem.set_u32(last + 8, extra),
        None => e.mem.set_u32(list + 4, extra),
    }
}

/// One mod's data: index 0 the number `value`, index 1 a form id, the first string `text`.
fn data(value: f32, form: u32, text: &[u8]) -> ExtraJipData {
    let mut values = [0; 18];
    values[0] = value.to_bits();
    values[1] = form;
    ExtraJipData {
        used: 3,
        forms: 2,
        values,
        strings: [text.to_vec(), vec![]],
    }
}

fn entry(ref_id: u32, mods: &[(u32, ExtraJipData)]) -> ExtraJipEntry {
    ExtraJipEntry {
        ref_id,
        data: mods.iter().cloned().collect::<BTreeMap<_, _>>(),
    }
}

fn put(e: &mut Engine, key: u32, value: ExtraJipEntry) {
    *e.ext::<Jip>().saved.extra_data.emplace(Uint(key)) = value;
}

fn get(e: &mut Engine, key: u32) -> Option<ExtraJipEntry> {
    e.ext::<Jip>().saved.extra_data.get(&Uint(key)).cloned()
}

fn changed_flag(e: &mut Engine) -> u8 {
    e.ext::<Jip>().saved.changed & changed::EXTRA_DATA
}

const ADDRESSES: [u32; 9] = [
    0x0040_f9d7,
    0x0041_1978,
    0x0041_d0f6,
    0x0041_d212,
    0x0041_d25a,
    0x0042_7f30,
    0x0042_c328,
    0x004c_21eb,
    0x004c_2291,
];

/// Not installed (today): no seam has a handler and the vtable keeps the exe's words, so every
/// seam asked does nothing.
#[test]
fn not_installed_nothing_registered() {
    let mut e = world(false);
    assert_eq!(e.jip_patches(), 0);
    for addr in ADDRESSES {
        assert!(e.site_owners(addr).is_empty(), "{addr:08x}");
        assert_eq!(e.site(addr, &[LIST, 0, 0, 0]), None);
    }
    assert_eq!(e.mem.u32(DESTROY_SLOT), 0x0041_b680);
    assert_eq!(e.mem.u32(COMPARE_SLOT), 0x0043_1ef0);
    assert!(!e.is_registered(DESTROY_ROUTINE));
}

/// Installed with the row off (the vanilla preset): the framework is JIP's own, so its nine
/// seams have JIP's handler, its two slots JIP's routines; the word is only `INSTALLED`.
#[test]
fn installed_with_the_row_off_puts_the_framework_in() {
    let e = world(true);
    assert_eq!(e.jip_patches(), jip_patches::INSTALLED);
    for addr in ADDRESSES {
        assert_eq!(e.site_owners(addr), vec![Owner::Jip], "{addr:08x}");
    }
    assert_eq!(e.mem.u32(DESTROY_SLOT), DESTROY_ROUTINE);
    assert_eq!(e.mem.u32(COMPARE_SLOT), COMPARE_ROUTINE);
    assert_eq!(AREA.jip_sites, &ADDRESSES[..]);
    assert!(AREA.fix_sites.is_empty() && AREA.fix_wraps.is_empty() && AREA.jip_wraps.is_empty());
}

/// The four table entries: the head of the chain (`0x40F9D7`) and the scans ended with false.
#[test]
fn the_tables_answer_jips_entries() {
    let mut e = world(true);
    for addr in [0x0040_f9d7, 0x0041_d0f6, 0x0041_d212, 0x0041_d25a] {
        assert_eq!(e.site(addr, &[LIST]), Some(1), "{addr:08x}");
    }
}

/// The copy arm with no data for the source's key: the arm is JIP's (answered), nothing is added
/// and the record is not marked changed.
#[test]
fn copy_without_data_copies_nothing() {
    let mut e = world(true);
    let source = extra_of(&mut e, 0x27, 0x77);
    assert_eq!(e.site(COPY.addr(), &[LIST, source, 0, 0]), Some(0));
    assert!(chain(&e, LIST).is_empty());
    assert!(calls(ADD_EXTRA).is_empty());
    assert_eq!(changed_flag(&mut e), 0);
}

/// A copy from any other caller: the list gets an ExtraJIP (key 0, then a new key: the random
/// word skips 0 and a key in use) holding a copy of the data; the source keeps its key and data,
/// the new entry no reference.
#[test]
fn copy_makes_a_new_key() {
    let mut e = world(true);
    let d = data(2.5, 0x0001_0203, b"abc");
    put(&mut e, 0x77, entry(0x99, &[(4, d.clone())]));
    put(&mut e, 0x55, entry(0, &[]));
    let source = extra_of(&mut e, 0x27, 0x77);
    RANDOM.with(|r| *r.borrow_mut() = vec![0, 0x55, 0x1234]);
    assert_eq!(
        e.site(COPY.addr(), &[LIST, source, 0x0041_2000, 0]),
        Some(0)
    );
    let added = chain(&e, LIST);
    assert_eq!(added.len(), 1);
    let x = added[0];
    assert_eq!(e.mem.u32(x), VTABLE);
    assert_eq!(e.mem.u32(x + 4), 0x27, "the type as a whole word");
    assert_eq!(e.mem.u32(x + 0x0c), 0x1234);
    assert_eq!(calls(GET_RANDOM_INT), vec![vec![GAME_RNG, 0xffff_ffff]; 3]);
    assert_eq!(get(&mut e, 0x1234), Some(entry(0, &[(4, d.clone())])));
    assert_eq!(get(&mut e, 0x77), Some(entry(0x99, &[(4, d)])));
    assert_eq!(e.mem.u32(source + 0x0c), 0x77);
    assert_eq!(changed_flag(&mut e), changed::EXTRA_DATA);
}

/// A copy into a list that has an ExtraJIP with a key: no new extra or key; the data replaces
/// that key's, whose reference stays.
#[test]
fn copy_into_a_key_keeps_its_reference() {
    let mut e = world(true);
    let (d1, d2) = (data(1.0, 0, b""), data(9.0, 0, b"x"));
    put(&mut e, 0x77, entry(0x99, &[(1, d1.clone())]));
    put(&mut e, 0x88, entry(0x44, &[(2, d2)]));
    let target = extra_of(&mut e, 0x27, 0x88);
    add(&mut e, LIST, target);
    let source = extra_of(&mut e, 0x27, 0x77);
    e.site(COPY.addr(), &[LIST, source, 0, 0]);
    assert_eq!(chain(&e, LIST), vec![target]);
    assert!(calls(GET_RANDOM_INT).is_empty());
    assert_eq!(get(&mut e, 0x88), Some(entry(0x44, &[(1, d1)])));
}

/// The two moving callers: `CopyListForContainer` moving (flag 0) and `CopyListForReference`
/// removing from the source (flag 1) swap the keys and clear the data's reference; the other
/// flag of each copies.
#[test]
fn copy_moves_the_key_for_a_moved_stack() {
    for (caller, flag, moved) in [
        (FROM_CONTAINER, 0, true),
        (FROM_CONTAINER, 1, false),
        (FROM_REFERENCE, 1, true),
        (FROM_REFERENCE, 0, false),
        // Only the low byte of the flag counts (`dl`).
        (FROM_CONTAINER, 0x100, true),
    ] {
        let mut e = world(true);
        let d = data(3.0, 0, b"");
        put(&mut e, 0x77, entry(0x99, &[(0, d.clone())]));
        let source = extra_of(&mut e, 0x27, 0x77);
        RANDOM.with(|r| *r.borrow_mut() = vec![0x4321]);
        e.site(COPY.addr(), &[LIST, source, caller, flag]);
        let x = chain(&e, LIST)[0];
        if moved {
            assert_eq!(e.mem.u32(x + 0x0c), 0x77, "{caller:x} {flag}");
            assert_eq!(e.mem.u32(source + 0x0c), 0, "the target's old key");
            assert_eq!(get(&mut e, 0x77), Some(entry(0, &[(0, d)])));
            assert!(calls(GET_RANDOM_INT).is_empty());
        } else {
            assert_eq!(e.mem.u32(x + 0x0c), 0x4321, "{caller:x} {flag}");
            assert_eq!(e.mem.u32(source + 0x0c), 0x77);
            assert_eq!(get(&mut e, 0x77), Some(entry(0x99, &[(0, d)])));
        }
        assert_eq!(changed_flag(&mut e), changed::EXTRA_DATA);
    }
}

/// A save buffer at [`BUFFER`] (`BGSSaveFormBuffer`): its chunk, size, used bytes, value count
/// and form.
fn buffer(e: &mut Engine, size: u32, used: u32, form: u32) {
    e.mem.set_u32(BUFFER + 4, CHUNK);
    e.mem.set_u32(BUFFER + 8, size);
    e.mem.set_u32(BUFFER + 0x0c, used);
    e.mem.set_u32(BUFFER + 0x10, 7);
    e.mem.set_u32(BUFFER + 0x20, form);
}

/// The save arm: 5 bytes reserved (the key and the spacer), the value count raised; the data's
/// reference is the buffer's form's id, or the header's without a form; a key with no data is
/// saved as 0; a full buffer is grown once first.
#[test]
fn save_writes_the_key_and_its_reference() {
    let mut e = world(true);
    put(&mut e, 0x77, entry(0, &[]));
    let x = extra_of(&mut e, 0x27, 0x77);
    e.mem.set_u32(FORM + 0x0c, 0x0001_0aaa);
    buffer(&mut e, 0x100, 0x10, FORM);
    assert_eq!(e.site(SAVE.addr(), &[BUFFER, x]), Some(0));
    assert_eq!(e.mem.u32(CHUNK + 0x10), 0x77);
    assert_eq!(e.mem.u8(CHUNK + 0x14), b'|');
    assert_eq!(e.mem.u32(BUFFER + 0x0c), 0x15);
    assert_eq!(e.mem.u32(BUFFER + 0x10), 8);
    assert_eq!(get(&mut e, 0x77).unwrap().ref_id, 0x0001_0aaa);
    assert!(calls(SAVE_BUFFER_GROW).is_empty());
    assert!(calls(HEADER_REF_ID).is_empty());
    // No form: the header's id; the buffer exactly full is not grown, one byte short is.
    buffer(&mut e, 0x15, 0x10, 0);
    e.site(SAVE.addr(), &[BUFFER, x]);
    assert_eq!(calls(HEADER_REF_ID), vec![vec![BUFFER + 0x14]]);
    assert_eq!(get(&mut e, 0x77).unwrap().ref_id, HEADER_ID);
    assert!(calls(SAVE_BUFFER_GROW).is_empty());
    buffer(&mut e, 0x14, 0x10, 0);
    e.site(SAVE.addr(), &[BUFFER, x]);
    assert_eq!(calls(SAVE_BUFFER_GROW), vec![vec![BUFFER]]);
    // A key with no data: 0, the spacer, no reference asked.
    let other = extra_of(&mut e, 0x27, 0x78);
    buffer(&mut e, 0x100, 0x20, FORM);
    e.mem.set_u32(CHUNK + 0x20, 0xdead_beef);
    e.site(SAVE.addr(), &[BUFFER, other]);
    assert_eq!(e.mem.u32(CHUNK + 0x20), 0);
    assert_eq!(e.mem.u8(CHUNK + 0x24), b'|');
    assert_eq!(e.mem.u32(BUFFER + 0x0c), 0x25);
}

/// The load arm: the key read where the buffer stands, 5 bytes passed over; 0 loads nothing; a
/// list with no ExtraJIP gets one; the same key changes nothing; another key replaces the old
/// one, whose data goes.
#[test]
fn load_reads_the_key_into_the_list() {
    let mut e = world(true);
    let at = |e: &mut Engine, used: u32, key: u32| {
        e.mem.set_u32(BUFFER + 4, CHUNK);
        e.mem.set_u32(BUFFER + 0x0c, used);
        e.mem.set_u32(CHUNK + used, key);
    };
    at(&mut e, 8, 0);
    assert_eq!(e.site(LOAD.addr(), &[LIST, BUFFER]), Some(0));
    assert_eq!(e.mem.u32(BUFFER + 0x0c), 13);
    assert!(chain(&e, LIST).is_empty());
    at(&mut e, 0x20, 0x77);
    e.site(LOAD.addr(), &[LIST, BUFFER]);
    assert_eq!(e.mem.u32(BUFFER + 0x0c), 0x25);
    let x = chain(&e, LIST)[0];
    assert_eq!((e.mem.u8(x + 4), e.mem.u32(x + 0x0c)), (0x27, 0x77));
    put(&mut e, 0x77, entry(1, &[]));
    at(&mut e, 0x30, 0x77);
    e.site(LOAD.addr(), &[LIST, BUFFER]);
    assert_eq!(chain(&e, LIST), vec![x]);
    assert!(get(&mut e, 0x77).is_some(), "the same key keeps its data");
    put(&mut e, 0x88, entry(2, &[]));
    at(&mut e, 0x40, 0x88);
    e.site(LOAD.addr(), &[LIST, BUFFER]);
    assert_eq!(chain(&e, LIST), vec![x]);
    assert_eq!(e.mem.u32(x + 0x0c), 0x88);
    assert!(get(&mut e, 0x77).is_none());
    assert!(get(&mut e, 0x88).is_some());
}

/// The pick-up count: with no count extra the count stands and nothing is removed; with one its
/// signed 16-bit value is the count and it is removed.
#[test]
fn the_picked_count_is_the_references() {
    let mut e = world(true);
    assert_eq!(e.site(PICKED_COUNT.addr(), &[LIST, 3]), Some(3));
    assert!(calls(REMOVE_BY_TYPE).is_empty());
    for (word, count) in [(5u32, 5u32), (0x0003_ffff, 0xffff_ffff), (0x7fff, 0x7fff)] {
        let c = extra_of(&mut e, 0x24, word);
        add(&mut e, LIST, c);
        assert_eq!(e.site(PICKED_COUNT.addr(), &[LIST, 1]), Some(count));
        assert!(chain(&e, LIST).is_empty());
    }
    assert_eq!(calls(REMOVE_BY_TYPE), vec![vec![LIST, 0x24]; 3]);
}

/// The new stack's count: above 1 a count extra (at most 32,767) is added and the call goes on;
/// 1, 0, a negative count or no list add nothing.
#[test]
fn a_count_above_one_goes_on_the_new_list() {
    let mut e = world(true);
    for count in [1i32, 0, -3] {
        assert_eq!(e.site(COPY_COUNT.addr(), &[LIST, count as u32]), None);
    }
    e.site(COPY_COUNT.addr(), &[0, 5]);
    assert!(calls(ADD_EXTRA).is_empty());
    for (count, word) in [(2u32, 2u32), (40_000, 0x7fff)] {
        assert_eq!(e.site(COPY_COUNT.addr(), &[SOURCE_LIST, count]), None);
        let c = *chain(&e, SOURCE_LIST).last().unwrap();
        assert_eq!(e.mem.u32(c), COUNT_VTABLE);
        assert_eq!(e.mem.u32(c + 4), 0x24);
        assert_eq!(e.mem.u32(c + 0x0c), word);
    }
}

/// The destructor: the data of its key go (record marked changed) unless its reference is gone
/// while the save has its changes; a key of 0 or one with no data drops nothing; the block is
/// freed to the game's heap only with the flag's low byte set; `this` is answered.
#[test]
fn the_destructor_drops_the_data_unless_a_gone_reference_holds_it() {
    let mut e = world(true);
    let x = extra_of(&mut e, 0x27, 0);
    let r = e.call(DESTROY_ROUTINE, &[x, 0]);
    assert_eq!(r.eax, x);
    assert!(calls(HEAP_FREE).is_empty());
    e.call(DESTROY_ROUTINE, &[x, 0x101]);
    assert_eq!(calls(HEAP_FREE), vec![vec![0x011f_6238, x]]);
    e.call(DESTROY_ROUTINE, &[x, 0x100]);
    assert_eq!(calls(HEAP_FREE).len(), 1, "only the low byte");
    insert(&mut e, FORMS, 0x0002_0001, FORM);
    insert(&mut e, CHANGES, 0x0002_0002, 1);
    for (ref_id, dropped) in [
        (0, true),
        (0x0002_0001, true),
        (0x0002_0003, true),
        (0x0002_0002, false),
    ] {
        put(&mut e, 0x77, entry(ref_id, &[]));
        e.ext::<Jip>().saved.changed = 0;
        e.mem.set_u32(x + 0x0c, 0x77);
        e.call(DESTROY_ROUTINE, &[x, 0]);
        assert_eq!(get(&mut e, 0x77).is_none(), dropped, "{ref_id:08x}");
        assert_eq!(changed_flag(&mut e) != 0, dropped);
    }
    // A key with no data: nothing marked.
    e.ext::<Jip>().saved.changed = 0;
    e.mem.set_u32(x + 0x0c, 0x99);
    e.call(DESTROY_ROUTINE, &[x, 0]);
    assert_eq!(changed_flag(&mut e), 0);
}

/// Each term of JIP's condition (`!refID || LookupFormByRefID(refID) || !HasChangeData(refID)`)
/// drops the data alone: a reference that still exists (a form in the all-forms table) although
/// the save holds changes of it, and a reference id of 0 although the changes map has an entry
/// for id 0; the record is marked changed both times.
#[test]
fn the_destructor_drops_the_data_of_a_reference_that_exists_or_of_id_0() {
    let mut e = world(true);
    let x = extra_of(&mut e, 0x27, 0x77);
    insert(&mut e, FORMS, 0x0002_0004, FORM);
    insert(&mut e, CHANGES, 0x0002_0004, 1);
    insert(&mut e, CHANGES, 0, 1);
    assert_eq!(cosave::lookup_form(&e, 0), 0);
    for ref_id in [0x0002_0004, 0] {
        assert!(cosave::has_change_data(&e, ref_id), "{ref_id:08x}");
        put(&mut e, 0x77, entry(ref_id, &[]));
        e.ext::<Jip>().saved.changed = 0;
        e.call(DESTROY_ROUTINE, &[x, 0]);
        assert!(get(&mut e, 0x77).is_none(), "{ref_id:08x}");
        assert_eq!(changed_flag(&mut e), changed::EXTRA_DATA, "{ref_id:08x}");
    }
}

/// `Compare`: another type, or a key with no data, differs; the same mods with the same data are
/// equal whatever their references; any field or mod changed differs; strings compare by length
/// and text up to a zero byte.
#[test]
fn compare_answers_whether_the_data_differ() {
    let mut e = world(true);
    let d = data(1.5, 0x10, b"ab");
    put(&mut e, 1, entry(0x10, &[(3, d.clone())]));
    put(&mut e, 2, entry(0x20, &[(3, d.clone())]));
    let (x, y) = (extra_of(&mut e, 0x27, 1), extra_of(&mut e, 0x27, 2));
    let differs = |e: &mut Engine, a: u32, b: u32| e.call(COMPARE_ROUTINE, &[a, b]).eax;
    assert_eq!(differs(&mut e, x, y), 0);
    let other_type = extra_of(&mut e, 0x24, 2);
    assert_eq!(differs(&mut e, x, other_type), 1);
    let no_data = extra_of(&mut e, 0x27, 3);
    assert_eq!(differs(&mut e, x, no_data), 1);
    assert_eq!(differs(&mut e, no_data, x), 1);
    let mut changes: Vec<ExtraJipEntry> = vec![];
    let mut f = d.clone();
    f.used = 1;
    changes.push(entry(0, &[(3, f)]));
    let mut f = d.clone();
    f.forms = 0;
    changes.push(entry(0, &[(3, f)]));
    let mut f = d.clone();
    f.values[17] = 1;
    changes.push(entry(0, &[(3, f)]));
    changes.push(entry(0, &[(3, data(1.5, 0x10, b"ac"))]));
    changes.push(entry(0, &[(3, data(1.5, 0x10, b"abc"))]));
    let mut f = d.clone();
    f.strings[1] = b"z".to_vec();
    changes.push(entry(0, &[(3, f)]));
    changes.push(entry(0, &[(4, d.clone())]));
    changes.push(entry(0, &[(3, d.clone()), (4, d.clone())]));
    changes.push(entry(0, &[]));
    for (i, change) in changes.into_iter().enumerate() {
        put(&mut e, 2, change);
        assert_eq!(differs(&mut e, x, y), 1, "change {i}");
    }
    // The same length, the same text before a zero byte: equal.
    put(&mut e, 1, entry(0, &[(3, data(1.5, 0x10, b"ab\0x"))]));
    put(&mut e, 2, entry(0, &[(3, data(1.5, 0x10, b"ab\0y"))]));
    assert_eq!(differs(&mut e, x, y), 0);
}
