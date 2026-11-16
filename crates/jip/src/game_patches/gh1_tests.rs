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
    let mut e = world(true);
    assert_eq!(e.jip_patches(), jip_patches::INSTALLED);
    for addr in ADDRESSES {
        assert_eq!(e.site_owners(addr), vec![Owner::Jip], "{addr:08x}");
    }
    assert_eq!(e.mem.u32(DESTROY_SLOT), DESTROY_ROUTINE);
    assert_eq!(e.mem.u32(COMPARE_SLOT), COMPARE_ROUTINE);
    assert_eq!(AREA.jip_sites, &ADDRESSES[..]);
    assert_eq!(AREA.fix_sites, &FIX_ADDRESSES[..]);
    assert_eq!(AREA.fix_wraps, &[0x0048_2090]);
    assert!(AREA.jip_wraps.is_empty());
    // The fixes are not in with the row off: no fix seam answers.
    for addr in FIX_ADDRESSES {
        assert_eq!(e.site(addr, &[HEAP, 0, 0]), None, "{addr:08x}");
    }
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

// ===================== The fixes =====================

const FIX_ADDRESSES: [u32; 12] = [
    0x0041_2675,
    0x0041_271e,
    0x0041_c833,
    0x004b_d820,
    0x004b_da70,
    0x004b_f630,
    0x004c_232c,
    0x004c_273a,
    0x004c_3976,
    0x004c_a200,
    0x004c_afa3,
    0x0095_400f,
];
const ITEMS_HEALTH_FN: u32 = 0x0048_2090;
/// Where the fix tests' forms and list nodes are (inside the world's mapped heap).
const FIX_FORMS: u32 = 0x0624_0000;
const NODES: u32 = 0x0625_0000;
/// What the stand-in of `00482B00` (the container list's copy) answers.
const COPIED: u32 = 0x0625_f000;

/// [`world`] with JIP installed and the fixes put in by hand (the word's `FIXES` bit and the
/// area's installer, as the row does; the whole-table test covers the row's own path).
fn fixes_world() -> Engine {
    let mut e = world(true);
    e.map(0x011d_e000, 0x1000);
    e.register(ITEMS_HEALTH_FN, |_, a| {
        log(ITEMS_HEALTH_FN, a);
        Ret::default()
    });
    e.register(CONTAINER_LIST_COPY, |_, a| {
        log(CONTAINER_LIST_COPY, a);
        eax(COPIED)
    });
    e.jip_patches_or(jip_patches::FIXES);
    install_fixes(&mut e);
    e
}

/// A form of `kind` at slot `n` of the test's forms.
fn form_of(e: &mut Engine, n: u32, kind: u8) -> u32 {
    let at = FIX_FORMS + 0x400 * n;
    e.mem.set_u8(at + 4, kind);
    at
}

/// A list of nodes (data, next) holding `items`, at slot `n` of the test's nodes.
fn nodes(e: &mut Engine, n: u32, items: &[u32]) -> u32 {
    let base = NODES + 0x100 * n;
    for (i, &item) in items.iter().enumerate() {
        let at = base + 8 * i as u32;
        e.mem.set_u32(at, item);
        let next = if i + 1 < items.len() { at + 8 } else { 0 };
        e.mem.set_u32(at + 4, next);
    }
    base
}

/// A changes entry at `at`: its extra lists' first node, its count, its form.
fn changes_entry(e: &mut Engine, at: u32, lists: u32, count: i32, form: u32) -> u32 {
    e.mem.set_u32(at, lists);
    e.mem.set_i32(at + 4, count);
    e.mem.set_u32(at + 8, form);
    at
}

/// The two jumps JIP writes (`0x41271E`, `0x412675`) answer; the temporary-form test answers
/// false for mod index 0xFF and leaves any other to the exe's call, at each of its three calls.
#[test]
fn the_jumps_and_the_temporary_form_test() {
    let mut e = fixes_world();
    assert_eq!(e.site(COMPARE_OWNED.addr(), &[]), Some(1));
    assert_eq!(e.site(HOTKEY_MOVED.addr(), &[]), Some(1));
    let form = form_of(&mut e, 0, 0x3b);
    for site in [PERSISTS_LIST, PERSISTS_PICKED, PERSISTS_MERGED] {
        for (index, answer) in [(0xffu8, Some(0)), (0xfe, None), (0, None)] {
            e.mem.set_u8(form + 0x0f, index);
            assert_eq!(e.site(site.addr(), &[form]), answer, "{index:02x}");
        }
    }
}

/// The iterator's destructor does nothing (`retn 4`): nothing freed, `this` answered.
#[test]
fn the_iterator_destructor_does_nothing() {
    let mut e = fixes_world();
    assert_eq!(e.site(ITERATOR_DESTROY.addr(), &[LIST, 1]), Some(LIST));
    assert!(calls(HEAP_FREE).is_empty());
}

/// The start: one block (0x2C bytes) made at the first iteration and kept; the 0x28 bytes
/// cleared, +0x28 the changes' list, the map emptied; no owner: no container; an owner whose base
/// is a container: the container (+0x10), its list (+8) and the copy (+0, +4); a base with no
/// container gets none; the next start frees the last copy (its nodes after the head, the head).
#[test]
fn the_iterator_start_resets_jips_iterator() {
    let mut e = fixes_world();
    let changes = FIX_FORMS + 0x3000;
    e.mem.set_u32(changes, 0x0626_0000);
    let block = e.site(ITERATOR_START.addr(), &[changes]).unwrap();
    assert_eq!(calls(HEAP_ALLOCATE), vec![vec![0x011f_6238, 0x2c]]);
    for offset in (0..0x28).step_by(4) {
        assert_eq!(e.mem.u32(block + offset), 0, "{offset:x}");
    }
    assert_eq!(e.mem.u32(block + 0x28), 0x0626_0000);
    assert!(calls(CONTAINER_LIST_COPY).is_empty());
    // An owner whose base form is a container (type 0x1B: its container at +0x30).
    let base = form_of(&mut e, 1, 0x1b);
    // Its vtable: no slot answers "actor base" (`TESForm::GetContainer`'s second test).
    e.mem.set_u32(base, FIX_FORMS + 0x3800);
    let owner = FIX_FORMS + 0x3100;
    e.mem.set_u32(owner + 0x20, base);
    e.mem.set_u32(changes + 4, owner);
    iterator_state(&mut e).handed.insert(0x1234, 5);
    e.mem.set_u32(block + 0x0c, 0x77);
    assert_eq!(e.site(ITERATOR_START.addr(), &[changes]), Some(block));
    assert_eq!(calls(HEAP_ALLOCATE).len(), 1, "the block is kept");
    assert!(iterator_state(&mut e).handed.is_empty());
    assert_eq!(e.mem.u32(block + 0x0c), 0);
    assert_eq!(e.mem.u32(block + 0x10), base + 0x30);
    assert_eq!(e.mem.u32(block + 8), base + 0x34);
    assert_eq!(e.mem.u32(block), COPIED);
    assert_eq!(e.mem.u32(block + 4), COPIED);
    assert_eq!(calls(CONTAINER_LIST_COPY), vec![vec![base + 0x30]]);
    // The copy holds two nodes after its head: freed with the head at the next start; a base
    // with no container (type 0x1F) gets none.
    let tail = nodes(&mut e, 0, &[0x41, 0x42]);
    e.mem.set_u32(COPIED, 0x40);
    e.mem.set_u32(COPIED + 4, tail);
    e.mem.set_u8(base + 4, 0x1f);
    e.site(ITERATOR_START.addr(), &[changes]);
    let freed: Vec<u32> = calls(HEAP_FREE).iter().map(|c| c[1]).collect();
    assert_eq!(freed, vec![tail, tail + 8, COPIED]);
    assert_eq!((e.mem.u32(COPIED), e.mem.u32(COPIED + 4)), (0, 0));
    assert_eq!(e.mem.u32(block), 0);
    assert_eq!(e.mem.u32(block + 0x10), 0);
    assert_eq!(e.mem.u32(block + 8), 0);
    assert_eq!(calls(CONTAINER_LIST_COPY).len(), 1);
}

/// The next stack: one handed out adds its count to its form's (a new form starting at 0) and
/// is answered; none: the pass byte becomes 2 and the forms are taken from the highest down: one
/// whose count is above what was handed out answers a new stack of the rest, one with nothing
/// left is passed over, and none left answers 0.
#[test]
fn the_iterator_next_gives_the_stacks_left() {
    let mut e = fixes_world();
    let iterator = FIX_FORMS + 0x4000;
    let (low, high) = (FIX_FORMS + 0x100, FIX_FORMS + 0x200);
    let out = changes_entry(&mut e, FIX_FORMS + 0x4100, 0, 2, low);
    e.mem.set_u32(iterator + 0x0c, out);
    assert_eq!(e.site(ITERATOR_NEXT.addr(), &[iterator]), Some(out));
    assert_eq!(e.site(ITERATOR_NEXT.addr(), &[iterator]), Some(out));
    let out2 = changes_entry(&mut e, FIX_FORMS + 0x4200, 0, -1, high);
    e.mem.set_u32(iterator + 0x0c, out2);
    e.site(ITERATOR_NEXT.addr(), &[iterator]);
    assert_eq!(
        iterator_state(&mut e)
            .handed
            .clone()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![(low, 4), (high, -1)]
    );
    // The container holds 5 of `low` and none of `high`; the changes add 1 of `high`.
    let object = FIX_FORMS + 0x4300;
    e.mem.set_i32(object, 5);
    e.mem.set_u32(object + 4, low);
    let list = nodes(&mut e, 1, &[object]);
    let delta = changes_entry(&mut e, FIX_FORMS + 0x4400, 0, 1, high);
    let changes = nodes(&mut e, 2, &[delta]);
    e.mem.set_u32(iterator + 8, list);
    e.mem.set_u32(iterator + 0x28, changes);
    e.mem.set_u32(iterator + 0x0c, 0);
    e.mem.set_u32(iterator + 0x18, 1);
    // `high` first: 1 - (-1) = 2 left.
    let made = e.site(ITERATOR_NEXT.addr(), &[iterator]).unwrap();
    assert_eq!(e.mem.u32(iterator + 0x18), 2);
    assert_eq!(
        (e.mem.u32(made), e.mem.i32(made + 4), e.mem.u32(made + 8)),
        (0, 2, high)
    );
    // Then `low`: 5 - 4 = 1 left.
    let made = e.site(ITERATOR_NEXT.addr(), &[iterator]).unwrap();
    assert_eq!((e.mem.i32(made + 4), e.mem.u32(made + 8)), (1, low));
    assert_eq!(e.site(ITERATOR_NEXT.addr(), &[iterator]), Some(0));
    // A form with nothing left is passed over.
    iterator_state(&mut e).handed.insert(low, 5);
    assert_eq!(e.site(ITERATOR_NEXT.addr(), &[iterator]), Some(0));
    assert!(iterator_state(&mut e).handed.is_empty());
}

/// The mod flags: 0 for no stack, a form that is not a weapon, no extra lists or a null first
/// list, or no flags extra; else the flags of the first list, or of the dropper reference's list
/// when the first list names one (a dropper extra naming no reference: the first list's).
#[test]
fn the_mod_flags_come_from_the_dropper_or_the_first_list() {
    let mut e = fixes_world();
    let weapon = form_of(&mut e, 0, FORM_WEAPON);
    let armor = form_of(&mut e, 1, FORM_ARMOR);
    let flags_of = |e: &mut Engine, entry: u32| e.site(MOD_FLAGS.addr(), &[entry]);
    assert_eq!(flags_of(&mut e, 0), Some(0));
    let entry = changes_entry(&mut e, FIX_FORMS + 0x5000, 0, 1, armor);
    assert_eq!(flags_of(&mut e, entry), Some(0));
    e.mem.set_u32(entry + 8, weapon);
    assert_eq!(flags_of(&mut e, entry), Some(0), "no extra lists");
    let node = nodes(&mut e, 0, &[0]);
    e.mem.set_u32(entry, node);
    assert_eq!(flags_of(&mut e, entry), Some(0), "a null first list");
    let first = FIX_FORMS + 0x5100;
    e.mem.set_u32(node, first);
    assert_eq!(flags_of(&mut e, entry), Some(0), "no flags extra");
    let flags = extra_of(&mut e, 0x8d, 0x0000_0105);
    add(&mut e, first, flags);
    assert_eq!(flags_of(&mut e, entry), Some(5));
    let dropper = extra_of(&mut e, 0x39, 0);
    add(&mut e, first, dropper);
    assert_eq!(
        flags_of(&mut e, entry),
        Some(5),
        "a dropper naming no reference"
    );
    let refr = FIX_FORMS + 0x5200;
    e.mem.set_u32(dropper + 0x0c, refr);
    assert_eq!(
        flags_of(&mut e, entry),
        Some(0),
        "the dropper's list has no flags"
    );
    let theirs = extra_of(&mut e, 0x8d, 2);
    add(&mut e, refr + 0x44, theirs);
    assert_eq!(flags_of(&mut e, entry), Some(2));
}

/// Has a mod: no flags: false; a set slot whose action type (the weapon's byte at +0x180,
/// +0x184, +0x188) is the effect: true; a matching byte in a slot not set does not count.
#[test]
fn has_mod_tests_the_set_slots_in_order() {
    let mut e = fixes_world();
    let weapon = form_of(&mut e, 0, FORM_WEAPON);
    let entry = changes_entry(&mut e, FIX_FORMS + 0x5000, 0, 1, weapon);
    let has = |e: &mut Engine, effect: u32| e.site(HAS_MOD.addr(), &[entry, effect]);
    assert_eq!(has(&mut e, 3), Some(0));
    let first = FIX_FORMS + 0x5100;
    let node = nodes(&mut e, 0, &[first]);
    e.mem.set_u32(entry, node);
    let flags = extra_of(&mut e, 0x8d, 0);
    add(&mut e, first, flags);
    e.mem.set_u8(weapon + 0x180, 3);
    e.mem.set_u8(weapon + 0x184, 4);
    e.mem.set_u8(weapon + 0x188, 5);
    assert_eq!(has(&mut e, 3), Some(0), "flags 0");
    for (bits, effect, answer) in [
        (1u32, 3u32, 1u32),
        (1, 4, 0),
        (2, 4, 1),
        (2, 3, 0),
        (4, 5, 1),
        (6, 5, 1),
        (6, 3, 0),
        (7, 0x103, 1),
    ] {
        e.mem.set_u32(flags + 0x0c, bits);
        assert_eq!(has(&mut e, effect), Some(answer), "{bits} {effect}");
    }
}

/// The items' health: clamped to 0..0.999 (not a number: 0.999); armour and a weapon of type
/// below 10 get it (a new extra, or the lower of the two); a weapon of type 10 and other forms
/// get 1.0, their extra freed when its owner word is 0; empty nodes passed over.
#[test]
fn the_items_health_is_jips() {
    let mut e = fixes_world();
    let armor = form_of(&mut e, 0, FORM_ARMOR);
    let gun = form_of(&mut e, 1, FORM_WEAPON);
    e.mem.set_u8(gun + 0xf4, 9);
    let rifle = form_of(&mut e, 2, FORM_WEAPON);
    e.mem.set_u8(rifle + 0xf4, 10);
    let misc = form_of(&mut e, 3, FORM_MISC);
    let item = |e: &mut Engine, at: u32, form: u32, extra: u32| {
        e.mem.set_u32(at + 4, form);
        e.mem.set_u32(at + 8, extra);
        at
    };
    let base = FIX_FORMS + 0x6000;
    let kept = base + 0x400;
    e.mem.set_f32(kept + 8, 0.25);
    let owned = base + 0x440;
    e.mem.set_u32(owned, 0x99);
    let unowned = base + 0x480;
    let items = [
        item(&mut e, base, armor, 0),
        0,
        item(&mut e, base + 0x40, gun, kept),
        item(&mut e, base + 0x80, rifle, unowned),
        item(&mut e, base + 0xc0, misc, owned),
    ];
    let container = FIX_FORMS + 0x7000;
    let list = nodes(&mut e, 0, &items);
    e.mem.set_u32(container + 4, e.mem.u32(list));
    e.mem.set_u32(container + 8, e.mem.u32(list + 4));
    e.call(ITEMS_HEALTH_FN, &[container, 0.5f32.to_bits()]);
    assert!(calls(ITEMS_HEALTH_FN).is_empty(), "rewritten, not run");
    let made = e.mem.u32(base + 8);
    assert_eq!(
        (e.mem.u32(made), e.mem.u32(made + 4), e.mem.f32(made + 8)),
        (0, 0, 0.5)
    );
    assert_eq!(e.mem.f32(kept + 8), 0.25, "the lower kept");
    assert_eq!(e.mem.u32(base + 0x88), 0, "freed");
    assert_eq!(e.mem.f32(unowned + 8), 1.0);
    assert_eq!(calls(HEAP_FREE), vec![vec![0x011f_6238, unowned]]);
    assert_eq!(e.mem.u32(base + 0xc8), owned, "an owned extra is kept");
    assert_eq!(e.mem.f32(owned + 8), 1.0);
    for (given, health) in [
        (f32::NAN, f32::from_bits(0x3f7f_be77)),
        (2.0, f32::from_bits(0x3f7f_be77)),
        (-1.0, 0.0),
        (0.1, 0.1),
    ] {
        e.mem.set_f32(kept + 8, 1.0);
        e.call(ITEMS_HEALTH_FN, &[container, given.to_bits()]);
        assert_eq!(e.mem.f32(kept + 8), health, "{given}");
    }
}

/// The player at [`PLAYER`] with the changes list `entries` (his list's container changes
/// extra).
fn player_with(e: &mut Engine, entries: &[u32]) {
    let player = FIX_FORMS + 0x8000;
    e.mem.set_u32(PLAYER, player);
    let data = FIX_FORMS + 0x8100;
    let list = nodes(e, 7, entries);
    e.mem.set_u32(data, list);
    let changes = extra_of(e, 0x15, data);
    e.mem.set_u32(player + 0x44 + 4, 0);
    add(e, player + 0x44, changes);
}

/// A list at `at` holding a hotkey of `key`.
fn hotkey_list(e: &mut Engine, at: u32, key: u32) -> u32 {
    let hotkey = extra_of(e, 0x4a, key);
    add(e, at, hotkey);
    at
}

/// The hotkey test: a list without a hotkey asks nothing; a key one of the player's stacks
/// already holds (an aid item's) leaves the given list; the same key on ammo, a misc item or a
/// form above `AlchemyItem`, another key, or a stack without extra lists does not.
#[test]
fn a_hotkey_the_player_uses_leaves_the_list() {
    let mut e = fixes_world();
    let weapon = form_of(&mut e, 0, FORM_WEAPON);
    let ammo = form_of(&mut e, 1, FORM_AMMO);
    let misc = form_of(&mut e, 2, FORM_MISC);
    let note = form_of(&mut e, 3, 0x31);
    let aid = form_of(&mut e, 4, FORM_ALCHEMY);
    let lists = |e: &mut Engine, n: u32, key: u32| {
        let list = hotkey_list(e, FIX_FORMS + 0x9000 + 0x40 * n, key);
        nodes(e, 10 + n, &[0, list])
    };
    let ammo_lists = lists(&mut e, 0, 3);
    let misc_lists = lists(&mut e, 1, 3);
    let note_lists = lists(&mut e, 2, 3);
    let other_key = lists(&mut e, 3, 4);
    let entries = [
        0,
        changes_entry(&mut e, FIX_FORMS + 0xa000, 0, 1, weapon),
        changes_entry(&mut e, FIX_FORMS + 0xa010, ammo_lists, 1, ammo),
        changes_entry(&mut e, FIX_FORMS + 0xa020, misc_lists, 1, misc),
        changes_entry(&mut e, FIX_FORMS + 0xa030, note_lists, 1, note),
        changes_entry(&mut e, FIX_FORMS + 0xa040, other_key, 1, weapon),
    ];
    player_with(&mut e, &entries);
    let given = hotkey_list(&mut e, FIX_FORMS + 0xb000, 3);
    clear_hotkey_if_used(&mut e, given);
    assert!(calls(REMOVE_BY_TYPE).is_empty());
    clear_hotkey_if_used(&mut e, FIX_FORMS + 0xb100);
    assert!(calls(REMOVE_BY_TYPE).is_empty());
    let aid_lists = lists(&mut e, 4, 3);
    let mut with_aid = entries.to_vec();
    with_aid.push(changes_entry(&mut e, FIX_FORMS + 0xa050, aid_lists, 1, aid));
    player_with(&mut e, &with_aid);
    clear_hotkey_if_used(&mut e, given);
    assert_eq!(calls(REMOVE_BY_TYPE), vec![vec![given, 0x4a]]);
    assert!(chain(&e, given).is_empty());
}

/// An item removed: given to the player (form id 0x14), not dropped, with a list: the hotkey
/// test runs; dropped, no list, no target or another target: it does not; the exe's block is
/// skipped either way (answered). The player's pick-up: the reference's list (+0x44), no answer.
#[test]
fn the_hotkey_seams_test_the_items_given_to_the_player() {
    let mut e = fixes_world();
    let weapon = form_of(&mut e, 0, FORM_WEAPON);
    let held = hotkey_list(&mut e, FIX_FORMS + 0x9000, 3);
    let held_lists = nodes(&mut e, 10, &[held]);
    let entry = changes_entry(&mut e, FIX_FORMS + 0xa000, held_lists, 1, weapon);
    player_with(&mut e, &[entry]);
    let player = e.mem.u32(PLAYER);
    e.mem.set_u32(player + 0x0c, 0x14);
    let other = FIX_FORMS + 0xc000;
    e.mem.set_u32(other + 0x0c, 0x15);
    let given = hotkey_list(&mut e, FIX_FORMS + 0xb000, 3);
    for args in [
        [1, given, player],
        [0, 0, player],
        [0, given, 0],
        [0, given, other],
    ] {
        assert_eq!(e.site(HOTKEY_REMOVED.addr(), &args), Some(0), "{args:x?}");
    }
    assert!(calls(REMOVE_BY_TYPE).is_empty());
    assert_eq!(
        e.site(HOTKEY_REMOVED.addr(), &[0x100, given, player]),
        Some(0)
    );
    assert_eq!(
        calls(REMOVE_BY_TYPE),
        vec![vec![given, 0x4a]],
        "the flag's low byte"
    );
    let refr = FIX_FORMS + 0xd000;
    hotkey_list(&mut e, refr + 0x44, 3);
    assert_eq!(e.site(HOTKEY_ACTIVATED.addr(), &[refr]), None);
    assert_eq!(calls(REMOVE_BY_TYPE)[1], vec![refr + 0x44, 0x4a]);
}

// Today's behaviour, pinned (CONDITION-LOCKS.md): the ExtraJIP routines still take JIP's shared
// slot (`Saved::extra_data`) for a key holding data. The follow-up (the data out of JIP's slot)
// turns these into passing held tests. Under the default stepped threads they cannot stall (one
// engine); they block JIP together with `NV_THREADS=real`.

#[test]
#[should_panic(expected = "two engines on one thread")]
fn copy_takes_jips_slot_today() {
    let mut e = world(true);
    put(
        &mut e,
        0x77,
        entry(0x99, &[(4, data(2.5, 0x0001_0203, b"abc"))]),
    );
    let source = extra_of(&mut e, 0x27, 0x77);
    crate::test_support::with_jips_slot_held(&mut e, |e| {
        e.site(COPY.addr(), &[LIST, source, 0x0041_2000, 0])
    });
}

#[test]
#[should_panic(expected = "two engines on one thread")]
fn destroy_takes_jips_slot_today() {
    let mut e = world(true);
    put(&mut e, 1, entry(0x10, &[(3, data(1.5, 0x10, b"ab"))]));
    let x = extra_of(&mut e, 0x27, 1);
    crate::test_support::with_jips_slot_held(&mut e, |e| e.call(DESTROY_ROUTINE, &[x, 0]));
}

#[test]
#[should_panic(expected = "two engines on one thread")]
fn differs_takes_jips_slot_today() {
    let mut e = world(true);
    let d = data(1.5, 0x10, b"ab");
    put(&mut e, 1, entry(0x10, &[(3, d.clone())]));
    put(&mut e, 2, entry(0x20, &[(3, d)]));
    let (x, y) = (extra_of(&mut e, 0x27, 1), extra_of(&mut e, 0x27, 2));
    crate::test_support::with_jips_slot_held(&mut e, |e| e.call(COMPARE_ROUTINE, &[x, y]));
}
