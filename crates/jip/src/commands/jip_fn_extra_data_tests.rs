//! Tests of JIP's extra data commands: a reference and an inventory stack with extra data lists,
//! the game's extra list calls as `test_support`'s stand-ins written from the exe, the game's
//! random number, lock and list destructor as recording stand-ins, and the expression-format
//! arguments compiled as xNVSE writes them (a count byte, then each argument's length and
//! token). Expectations from JIP's source (functions_jip/jip_fn_misc_ref.h,
//! nvse/GameExtraData.h `ExtraJIPData::Set`), never from the port's output.

use super::*;
use crate::commands::test_support::*;
use crate::cosave::ExtraJipEntry;
use crate::test_support::{DATA, OFFSET};

const REF: u32 = FORMS + 0x1000;
const ACTOR: u32 = FORMS + 0x1400;
const WEAP: u32 = FORMS + 0x1800;
const FORM_A: u32 = FORMS + 0x1900;
const FILE: u32 = FORMS + 0x2000;
const CHANGES_X: u32 = FORMS + 0x3000;
const CHANGES_DATA: u32 = FORMS + 0x3040;
const ENTRIES: u32 = FORMS + 0x3080;
const ENTRY: u32 = FORMS + 0x3100;
const LISTS: u32 = FORMS + 0x3140;
const STACK: u32 = FORMS + 0x3200;
const COUNT_X: u32 = FORMS + 0x3280;
const OTHER_X: u32 = FORMS + 0x32c0;
const ALLOC: u32 = FORMS + 0x8000;
const ALLOC_AREA: u32 = FORMS + 0x9000;
const ACTOR_VT: u32 = FORMS + 0xa000;
const PLAIN_VT: u32 = FORMS + 0xa400;
const KEYS: u32 = FORMS + 0xb000;
const RANDOM_FN: u32 = 0x00aa_5230;
const ENTER_FN: u32 = 0x0040_fbf0;
const LEAVE_FN: u32 = 0x0040_fba0;
const LIST_VTBL: u32 = 0x0101_43e8;
const DESTROY_FN: u32 = 0x0041_0380;
const LOCK: u32 = 0x011c_3920;

thread_local! {
    static CALLED: std::cell::RefCell<Vec<(u32, Vec<u32>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    /// The blocks freed (start, size): poisoned with 0xCD, and any extra list call on them or
    /// walk through them fails the test.
    static FREED: std::cell::RefCell<Vec<(u32, u32)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// The block `at` freed: filled with 0xCD and remembered.
fn poison(e: &mut Engine, at: u32, size: u32) {
    for i in 0..size {
        e.mem.set_u8(at + i, 0xcd);
    }
    FREED.with(|f| f.borrow_mut().push((at, size)));
}

/// Fails the test when `at` lies in a freed block.
fn live(at: u32) {
    FREED.with(|f| {
        for &(start, size) in f.borrow().iter() {
            assert!(
                !(start..start + size).contains(&at),
                "freed memory reached at {at:#x}"
            );
        }
    });
}

/// Whether the block `at` still holds only the poison (nothing written into it after its free).
fn still_poisoned(e: &Engine, at: u32, size: u32) -> bool {
    (0..size).all(|i| e.mem.u8(at + i) == 0xcd)
}

/// A call recorded where `test_support`'s stand-ins record theirs.
fn rec(addr: u32, w: &[u32]) {
    crate::commands::test_support::CALLS.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

/// The extra list calls as `test_support`'s, recorded there, with every list and extra they
/// reach checked against the freed blocks; `RemoveByType` frees the extra it takes off.
fn checked_extra_stand_ins(e: &mut Engine) {
    e.register(GET_BY_TYPE_FN, |e, w| {
        live(w[0]);
        let mut x = e.mem.u32(w[0] + 4);
        while x != 0 && u32::from(e.mem.u8(x + 4)) != w[1] {
            live(x);
            x = e.mem.u32(x + 8);
        }
        live(x);
        eax(x)
    });
    e.register(ADD_EXTRA_FN, |e, w| {
        rec(ADD_EXTRA_FN, w);
        let (list, extra) = (w[0], w[1]);
        live(list);
        let mut at = list + 4;
        while e.mem.u32(at) != 0 {
            live(e.mem.u32(at));
            at = e.mem.u32(at) + 8;
        }
        e.mem.set_u32(at, extra);
        let kind = u32::from(e.mem.u8(extra + 4));
        let bits = list + 8 + (kind >> 3);
        e.mem.set_u8(bits, e.mem.u8(bits) | (1 << (kind & 7)));
        eax(extra)
    });
    e.register(REMOVE_BY_TYPE_FN, |e, w| {
        rec(REMOVE_BY_TYPE_FN, w);
        let (list, kind) = (w[0], w[1]);
        live(list);
        let mut at = list + 4;
        loop {
            let x = e.mem.u32(at);
            if x == 0 {
                break;
            }
            live(x);
            if u32::from(e.mem.u8(x + 4)) == kind {
                e.mem.set_u32(at, e.mem.u32(x + 8));
                poison(e, x, 0x10);
                break;
            }
            at = x + 8;
        }
        let bits = list + 8 + (kind >> 3);
        e.mem.set_u8(bits, e.mem.u8(bits) & !(1 << (kind & 7)));
        eax(0)
    });
}

fn record(addr: u32, w: &[u32]) {
    CALLED.with(|c| c.borrow_mut().push((addr, w.to_vec())));
}

fn called(addr: u32) -> Vec<Vec<u32>> {
    CALLED.with(|c| {
        c.borrow()
            .iter()
            .filter(|x| x.0 == addr)
            .map(|x| x.1.clone())
            .collect()
    })
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// A plain reference `REF` (its list at +0x44 empty), the script mod 5's, the keys the random
/// number gives from `KEYS` (a word count at `KEYS`, then the words), the lock and destructor
/// recorded.
fn setup() -> Engine {
    CALLED.with(|c| c.borrow_mut().clear());
    FREED.with(|f| f.borrow_mut().clear());
    let mut e = engine();
    extra_stand_ins(&mut e);
    checked_extra_stand_ins(&mut e);
    e.map(0x0101_4000, 0x1000);
    e.mem.set_u32(LIST_VTBL, DESTROY_FN);
    form(&mut e, REF, 0x3a, 0x0001_0001);
    e.mem.set_u32(REF, PLAIN_VT);
    form(&mut e, FORM_A, 0x1f, 0x0002_0002);
    e.mem.set_u32(SCRIPT + 0x10, FILE);
    e.mem.set_u32(SCRIPT + 0x14, 0);
    e.mem.set_u8(FILE + 0x40c, 5);
    set_ref(&mut e, 1, FORM_A);
    keys(&mut e, &[0x1234]);
    e.register(RANDOM_FN, |e, w| {
        record(RANDOM_FN, w);
        let i = e.mem.u32(KEYS);
        e.mem.set_u32(KEYS, i + 1);
        eax(e.mem.u32(KEYS + 4 + 4 * i))
    });
    e.register(ENTER_FN, |_, w| {
        record(ENTER_FN, w);
        eax(0)
    });
    e.register(LEAVE_FN, |_, w| {
        record(LEAVE_FN, w);
        eax(0)
    });
    // The list's deleting destructor: the 0x20-byte list freed (poisoned).
    e.register(DESTROY_FN, |e, w| {
        record(DESTROY_FN, w);
        live(w[0]);
        poison(e, w[0], 0x20);
        eax(w[0])
    });
    // `toupper` and the case helper xNVSE's string tokens use.
    e.register(0x00ec_a7f4, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(0x0048_39c0, |_, a| eax(a[0]));
    e
}

fn keys(e: &mut Engine, words: &[u32]) {
    e.mem.set_u32(KEYS, 0);
    for (i, w) in words.iter().enumerate() {
        e.mem.set_u32(KEYS + 4 + 4 * i as u32, *w);
    }
}

fn num(v: f64) -> Vec<u8> {
    let mut b = vec![b'Z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
fn text(t: &str) -> Vec<u8> {
    let mut b = vec![b'S'];
    b.extend_from_slice(&(t.len() as u16).to_le_bytes());
    b.extend_from_slice(t.as_bytes());
    b
}
fn reff(idx: u16) -> Vec<u8> {
    let mut b = vec![b'R'];
    b.extend_from_slice(&idx.to_le_bytes());
    b
}

/// An expression-format call on `this`: the count byte, then each token with its length.
fn run(e: &mut Engine, opcode: u32, this: u32, tokens: &[Vec<u8>]) -> f64 {
    let mut bytes = vec![tokens.len() as u8];
    for t in tokens {
        bytes.extend_from_slice(&(t.len() as u16 + 2).to_le_bytes());
        bytes.extend_from_slice(t);
    }
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16);
    e.mem.write(DATA + 4, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    let record = nvse::table::record_of(e, opcode);
    let (params, execute) = (e.mem.u32(record + 0x14), e.mem.u32(record + 0x18));
    let ok = e.call(
        execute,
        &[
            params,
            DATA,
            this,
            0,
            SCRIPT,
            crate::test_support::LIST,
            RESULT,
            OFFSET,
        ],
    );
    assert_eq!(ok.eax & 0xff, 1);
    e.mem.f64(RESULT)
}

fn has(e: &mut Engine, this: u32, mod_index: i32) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call_on(e, 0x292e, this, &[n(mod_index)]).1
}

fn get(e: &mut Engine, this: u32, args: &[Vec<u8>]) -> f64 {
    run(e, 0x292f, this, args)
}

fn set(e: &mut Engine, this: u32, args: &[Vec<u8>]) -> f64 {
    run(e, 0x2930, this, args)
}

fn entry(e: &mut Engine, key: u32) -> Option<ExtraJipEntry> {
    e.ext::<crate::Jip>()
        .saved
        .extra_data
        .get(&crate::containers::Uint(key))
        .cloned()
}

fn changed(e: &mut Engine) -> u8 {
    e.ext::<crate::Jip>().saved.changed
}

/// The `ExtraJIP` (type 0x27) of a list, 0 for none.
fn xjip(e: &Engine, list: u32) -> u32 {
    let mut x = e.mem.u32(list + 4);
    while x != 0 && e.mem.u8(x + 4) != 0x27 {
        x = e.mem.u32(x + 8);
    }
    x
}

// ---- a reference ----

#[test]
fn set_then_has_and_get_on_a_reference() {
    let mut e = setup();
    assert_eq!(set(&mut e, REF, &[num(3.0), num(2.5)]), 0.0);
    let x = xjip(&e, REF + 0x44);
    assert_ne!(x, 0, "an ExtraJIP added");
    assert_eq!((e.mem.u32(x), e.mem.u32(x + 0xc)), (0x0101_58fc, 0x1234));
    assert_eq!(
        calls_to(ADD_CHANGE_FN),
        vec![vec![SAVE_LOAD, REF, 0x400, 0]]
    );
    assert_eq!(called(RANDOM_FN), vec![vec![0x011c_4180, 0xffff_ffff]]);
    let data = entry(&mut e, 0x1234).expect("the key's entry").data[&5].clone();
    assert_eq!((data.used, data.forms), (1 << 3, 0));
    assert_eq!(data.values[3], 2.5f32.to_bits());
    assert_eq!(changed(&mut e) & crate::cosave::changed::EXTRA_DATA, 4);
    assert_eq!(has(&mut e, REF, 0), 1.0, "the script's mod");
    assert_eq!(has(&mut e, REF, 5), 1.0);
    assert_eq!(has(&mut e, REF, 6), 0.0);
    assert_eq!(get(&mut e, REF, &[num(3.0)]), 2.5);
    assert_eq!(get(&mut e, REF, &[num(3.0), num(5.0)]), 2.5);
    assert_eq!(get(&mut e, REF, &[num(3.0), num(6.0)]), 0.0);
    assert_eq!(get(&mut e, REF, &[num(4.0)]), 0.0, "an unused index");
    // A second value: same ExtraJIP, same key, no new key or change mark.
    set(&mut e, REF, &[num(17.0), num(-1.0)]);
    assert_eq!(xjip(&e, REF + 0x44), x);
    assert_eq!(called(RANDOM_FN).len(), 1);
    assert_eq!(calls_to(ADD_CHANGE_FN).len(), 1);
    assert_eq!(get(&mut e, REF, &[num(17.0)]), -1.0);
}

#[test]
fn numbers_are_kept_as_floats_and_zero_unuses() {
    let mut e = setup();
    set(&mut e, REF, &[num(0.0), num(0.1)]);
    assert_eq!(get(&mut e, REF, &[num(0.0)]), f64::from(0.1f32));
    set(&mut e, REF, &[num(1.0), num(1e40)]);
    assert_eq!(get(&mut e, REF, &[num(1.0)]), f64::INFINITY);
    set(&mut e, REF, &[num(0.0), num(0.0)]);
    let data = entry(&mut e, 0x1234).unwrap().data[&5].clone();
    assert_eq!(data.used, 1 << 1, "index 0 unused again");
    // The index truncates toward zero (2.9 is 2).
    set(&mut e, REF, &[num(2.9), num(7.0)]);
    assert_eq!(get(&mut e, REF, &[num(2.0)]), 7.0);
    assert_eq!(get(&mut e, REF, &[num(2.7)]), 7.0);
}

#[test]
fn forms_are_kept_as_ids() {
    let mut e = setup();
    set(&mut e, REF, &[num(2.0), reff(1)]);
    let data = entry(&mut e, 0x1234).unwrap().data[&5].clone();
    assert_eq!((data.used, data.forms, data.values[2]), (4, 4, 0x0002_0002));
    get(&mut e, REF, &[num(2.0)]);
    assert_eq!(form_result(&e), 0x0002_0002);
    // A number over it: the form bit goes.
    set(&mut e, REF, &[num(2.0), num(3.0)]);
    let data = entry(&mut e, 0x1234).unwrap().data[&5].clone();
    assert_eq!((data.used, data.forms), (4, 0));
    assert_eq!(get(&mut e, REF, &[num(2.0)]), 3.0);
}

#[test]
fn a_null_form_unuses_and_keeps_other_form_bits_only_where_used() {
    let mut e = setup();
    set(&mut e, REF, &[num(2.0), reff(1)]);
    set(&mut e, REF, &[num(4.0), reff(1)]);
    // Index 4 left a form bit with no use: Set(4, 0) clears it through `typeBtf &= initBtf`.
    {
        let jip = e.ext::<crate::Jip>();
        let d = jip
            .saved
            .extra_data
            .get_mut(&crate::containers::Uint(0x1234))
            .unwrap()
            .data
            .get_mut(&5)
            .unwrap();
        d.forms |= 1 << 6;
    }
    set_ref(&mut e, 2, 0);
    // A reference holding no form reads as form 0.
    e.mem.set_u32(0x0300_a000 + 0x20 + 8, 0);
    set(&mut e, REF, &[num(4.0), reff(2)]);
    let data = entry(&mut e, 0x1234).unwrap().data[&5].clone();
    assert_eq!(data.used, 1 << 2);
    assert_eq!(data.forms, 1 << 2, "only the used form bit stays");
    assert_eq!(data.values[4], 0);
}

#[test]
fn texts_at_18_and_19() {
    let mut e = setup();
    set(&mut e, REF, &[num(18.0), text("Doc")]);
    set(&mut e, REF, &[num(19.0), text("Mitchell")]);
    // `Set(idx, const char*)` touches no bits (GameExtraData.h).
    let data = entry(&mut e, 0x1234).unwrap().data[&5].clone();
    assert_eq!((data.used, data.forms), (0, 0));
    let r = get(&mut e, REF, &[num(18.0)]);
    assert_eq!(string_result(&e, r), b"Doc");
    let r = get(&mut e, REF, &[num(19.0)]);
    assert_eq!(string_result(&e, r), b"Mitchell");
    // A number at a text index stores nothing (it cannot be a string), but marks the change.
    e.ext::<crate::Jip>().saved.changed = 0;
    set(&mut e, REF, &[num(18.0), num(5.0)]);
    let data = entry(&mut e, 0x1234).unwrap().data[&5].clone();
    assert_eq!(data.strings[0], b"Doc");
    assert_eq!(changed(&mut e), 4);
    // A text at a number index: JIP reads it as a number (0), unusing the index.
    set(&mut e, REF, &[num(1.0), num(9.0)]);
    set(&mut e, REF, &[num(1.0), text("x")]);
    assert_eq!(get(&mut e, REF, &[num(1.0)]), 0.0);
}

#[test]
fn indexes_past_19_and_missing_values_are_refused() {
    let mut e = setup();
    // 19 is a text index; 20 and a negative one (a huge unsigned index) are refused.
    set(&mut e, REF, &[num(20.0), num(1.0)]);
    set(&mut e, REF, &[num(-1.0), num(1.0)]);
    set(&mut e, REF, &[num(3.0)]);
    assert_eq!(xjip(&e, REF + 0x44), 0, "nothing made");
    assert!(called(ENTER_FN).is_empty(), "refused before the lock");
    set(&mut e, REF, &[num(19.0), text("ok")]);
    assert_ne!(xjip(&e, REF + 0x44), 0);
    assert_eq!(get(&mut e, REF, &[num(20.0)]), 0.0);
    assert_eq!(get(&mut e, REF, &[num(-1.0)]), 0.0);
    let r = get(&mut e, REF, &[num(19.0)]);
    assert_eq!(string_result(&e, r), b"ok");
    // No argument or more than two: nothing (deliberate differences: xNVSE's `ExtractArgsV`
    // leaves JIP's index unset, or writes past its two doubles).
    let enters = called(ENTER_FN).len();
    assert_eq!(get(&mut e, REF, &[]), 0.0);
    assert_eq!(get(&mut e, REF, &[num(19.0), num(5.0), num(1.0)]), 0.0);
    // An index that is no number (a pointer in JIP's unset double): nothing.
    assert_eq!(get(&mut e, REF, &[text("19")]), 0.0);
    assert_eq!(called(ENTER_FN).len(), enters, "refused before the lock");
}

/// A mod index `ExtractArgsV` writes as a pointer (a text, a form) into JIP's `fModIdx = 0`
/// reads as 0 (a denormal), so the script's own mod (`GetOverridingModIdx`).
#[test]
fn a_mod_index_that_is_no_number_is_the_scripts() {
    let mut e = setup();
    set(&mut e, REF, &[num(19.0), text("ok")]);
    set(&mut e, REF, &[num(3.0), num(7.0)]);
    let r = get(&mut e, REF, &[num(19.0), text("5")]);
    assert_eq!(string_result(&e, r), b"ok");
    assert_eq!(get(&mut e, REF, &[num(3.0), reff(1)]), 7.0);
}

#[test]
fn mod_indexes_0xff_and_up_are_refused() {
    let mut e = setup();
    set(&mut e, REF, &[num(3.0), num(1.0)]);
    let enters = called(ENTER_FN).len();
    assert_eq!(has(&mut e, REF, 0xff), 0.0);
    assert_eq!(has(&mut e, REF, 0x100), 0.0);
    assert_eq!(get(&mut e, REF, &[num(3.0), num(255.0)]), 0.0);
    assert_eq!(called(ENTER_FN).len(), enters, "no lock taken for them");
    // 0xFE is a mod index like any other.
    assert_eq!(has(&mut e, REF, 0xfe), 0.0);
    assert_eq!(called(ENTER_FN).len(), enters + 1);
    // A script of no mod (0xFF): SetRefExtraData does nothing at all.
    e.mem.set_u8(FILE + 0x40c, 0xff);
    set(&mut e, REF, &[num(4.0), num(1.0)]);
    assert_eq!(called(ENTER_FN).len(), enters + 1);
    e.mem.set_u8(FILE + 0x40c, 5);
    assert_eq!(get(&mut e, REF, &[num(4.0)]), 0.0);
}

#[test]
fn every_command_takes_the_extra_data_lock_once() {
    let mut e = setup();
    set(&mut e, REF, &[num(3.0), num(1.0)]);
    has(&mut e, REF, 0);
    get(&mut e, REF, &[num(3.0)]);
    set(&mut e, REF, &[]);
    assert_eq!(called(ENTER_FN), vec![vec![LOCK, 0]; 4]);
    assert_eq!(called(LEAVE_FN), vec![vec![LOCK]; 4]);
}

#[test]
fn erase_the_mod_and_the_last_mod_takes_the_extra_off() {
    let mut e = setup();
    set(&mut e, REF, &[num(3.0), num(1.0)]);
    e.mem.set_u8(FILE + 0x40c, 6);
    set(&mut e, REF, &[num(3.0), num(2.0)]);
    e.ext::<crate::Jip>()
        .saved
        .extra_data
        .get_mut(&crate::containers::Uint(0x1234))
        .unwrap()
        .ref_id = 0x0001_0001;
    // Mod 6's data erased: mod 5's stays, the extra and the entry's reference too.
    e.ext::<crate::Jip>().saved.changed = 0;
    set(&mut e, REF, &[]);
    assert_eq!(changed(&mut e), 4);
    let entry_now = entry(&mut e, 0x1234).unwrap();
    assert_eq!(entry_now.data.keys().copied().collect::<Vec<_>>(), vec![5]);
    assert_eq!(entry_now.ref_id, 0x0001_0001);
    assert_ne!(xjip(&e, REF + 0x44), 0);
    assert!(calls_to(REMOVE_BY_TYPE_FN).is_empty());
    // Again: nothing to erase, no change mark.
    e.ext::<crate::Jip>().saved.changed = 0;
    set(&mut e, REF, &[]);
    assert_eq!(changed(&mut e), 0);
    // Mod 5's: the last; the entry's reference cleared, the extra taken off.
    e.mem.set_u8(FILE + 0x40c, 5);
    set(&mut e, REF, &[]);
    let entry_now = entry(&mut e, 0x1234).unwrap();
    assert!(entry_now.data.is_empty());
    assert_eq!(entry_now.ref_id, 0);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![REF + 0x44, 0x27]]);
    assert_eq!(has(&mut e, REF, 5), 0.0);
}

#[test]
fn no_reference_does_nothing() {
    let mut e = setup();
    assert_eq!(set(&mut e, 0, &[num(3.0), num(1.0)]), 0.0);
    assert_eq!(has(&mut e, 0, 0), 0.0);
    assert_eq!(get(&mut e, 0, &[num(3.0)]), 0.0);
    assert!(called(ENTER_FN).is_empty());
}

// ---- an inventory stack ----

/// The actor `ACTOR` holding `WEAP` (its changes' entry `ENTRY`, count `count`, its list of
/// stacks `LISTS` holding `stack` when not 0); the inventory reference for it.
fn inventory(e: &mut Engine, count: i32, stack: u32) -> u32 {
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    form(e, ACTOR, 0x3b, 0x0003_0003);
    e.mem.set_u32(ACTOR, ACTOR_VT);
    form(e, WEAP, 0x28, 0x0004_0004);
    put_extra(e, ACTOR + 0x44, CHANGES_X, 0x15, CHANGES_DATA);
    e.mem.set_u32(CHANGES_DATA, ENTRIES);
    e.mem.set_u32(ENTRIES, ENTRY);
    e.mem.set_u32(ENTRIES + 4, 0);
    e.mem.set_u32(ENTRY + 4, count as u32);
    e.mem.set_u32(ENTRY + 8, WEAP);
    if stack != 0 {
        e.mem.set_u32(ENTRY, LISTS);
        e.mem.set_u32(LISTS, stack);
        e.mem.set_u32(LISTS + 4, 0);
        e.mem.set_u32(stack, LIST_VTBL);
    } else {
        e.mem.set_u32(ENTRY, 0);
    }
    e.mem.set_u32(ALLOC, ALLOC_AREA);
    e.register(0x0040_1000, |e, w| {
        let at = e.mem.u32(ALLOC);
        e.mem.set_u32(ALLOC, at + w[0].next_multiple_of(0x10));
        eax(at)
    });
    e.register(0x0040_1030, |_, _| Ret::default());
    // A temporary reference's id of its own, from its address.
    e.register(0x0055_a2f0, |e, w| {
        e.mem
            .set_u32(w[0] + 0xc, 0xff00_0000 | ((w[0] >> 4) & 0xffff));
        eax(w[0])
    });
    nvse::invref::create_entry(e, ACTOR, WEAP, count, stack)
}

fn stack_of(e: &mut Engine, refr: u32) -> u32 {
    let id = crate::commands::ref_id(e, refr);
    nvse::invref::get(e, id)
        .expect("an inventory reference")
        .data
        .xdata
}

#[test]
fn an_inventory_stack_keeps_the_data_and_the_container_is_marked() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    let x = xjip(&e, STACK);
    assert_ne!(x, 0, "on the stack");
    assert_eq!(
        calls_to(ADD_EXTRA_FN),
        vec![vec![STACK, x]],
        "added to the stack"
    );
    // The temporary reference's copy of the stack's words follows (`SetData` again).
    assert_eq!(xjip(&e, refr + 0x44), x);
    assert_eq!(
        calls_to(ADD_CHANGE_FN),
        vec![vec![SAVE_LOAD, ACTOR, 0x400, 0]]
    );
    assert_eq!(has(&mut e, refr, 0), 1.0);
    assert_eq!(get(&mut e, refr, &[num(3.0)]), 4.0);
}

#[test]
fn an_inventory_reference_without_a_stack_gets_one_with_its_count() {
    let mut e = setup();
    let refr = inventory(&mut e, 3, 0);
    // Before: nothing to find (the reference's own list is not read).
    put_extra(&mut e, refr + 0x44, OTHER_X, 0x27, 0x1234);
    assert_eq!(has(&mut e, refr, 0), 0.0);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    let stack = stack_of(&mut e, refr);
    assert_ne!(stack, 0);
    assert_eq!(e.mem.u32(stack), LIST_VTBL);
    assert_eq!(
        super::super::list_items(&e, Ptr::new(e.mem.u32(ENTRY))),
        vec![stack]
    );
    // The count extra first (3), then the ExtraJIP.
    let first = e.mem.u32(stack + 4);
    assert_eq!((e.mem.u8(first + 4), e.mem.u32(first + 0xc)), (0x24, 3));
    assert_eq!(e.mem.u8(e.mem.u32(first + 8) + 4), 0x27);
    // Two items: a count extra of 2 (`GetCount() > 1`).
    let mut e = setup();
    let refr = inventory(&mut e, 2, 0);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    let stack = stack_of(&mut e, refr);
    let first = e.mem.u32(stack + 4);
    assert_eq!((e.mem.u8(first + 4), e.mem.u32(first + 0xc)), (0x24, 2));
    // One item: no count extra.
    let mut e = setup();
    let refr = inventory(&mut e, 1, 0);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    let stack = stack_of(&mut e, refr);
    assert_eq!(e.mem.u8(e.mem.u32(stack + 4) + 4), 0x27);
}

#[test]
fn an_inventory_stack_left_with_only_its_count_goes() {
    let mut e = setup();
    let refr = inventory(&mut e, 2, STACK);
    put_extra(&mut e, STACK, COUNT_X, 0x24, 2);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    set(&mut e, refr, &[]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![STACK, 0x27]]);
    assert_eq!(super::super::list_items(&e, Ptr::new(LISTS)), vec![0]);
    assert_eq!(called(DESTROY_FN), vec![vec![STACK, 1]]);
}

#[test]
fn an_inventory_stack_with_more_than_its_count_stays() {
    let mut e = setup();
    let refr = inventory(&mut e, 2, STACK);
    put_extra(&mut e, STACK, OTHER_X, 0x16, 0);
    put_extra(&mut e, STACK, COUNT_X, 0x24, 2);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    set(&mut e, refr, &[]);
    assert_eq!(super::super::list_items(&e, Ptr::new(LISTS)), vec![STACK]);
    assert!(called(DESTROY_FN).is_empty());
    // Another mod's data left: the ExtraJIP stays and nothing is freed either.
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    e.mem.set_u8(FILE + 0x40c, 6);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    set(&mut e, refr, &[]);
    assert_ne!(xjip(&e, STACK), 0);
    assert!(called(DESTROY_FN).is_empty());
}

#[test]
fn an_emptied_stack_whose_item_has_no_entry_stays() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    e.mem.set_u32(ENTRY + 8, FORM_A);
    set(&mut e, refr, &[]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![STACK, 0x27]]);
    assert!(called(DESTROY_FN).is_empty());
    assert_eq!(super::super::list_items(&e, Ptr::new(LISTS)), vec![STACK]);
}

// ---- review round: the stack's free / keep rule and the references standing for it ----

const REF2: u32 = FORMS + 0x1100;
const NODE2: u32 = FORMS + 0x3180;
const STACK2: u32 = FORMS + 0x3300;

fn id_of(e: &mut Engine, refr: u32) -> u32 {
    crate::commands::ref_id(e, refr)
}

/// The inventory reference made to stand for its stack again (`SetData`, as a `ForEach` loop's
/// step does): its temporary reference's copy of the stack's words taken now.
fn sync(e: &mut Engine, refr: u32) {
    let id = id_of(e, refr);
    let data = nvse::invref::get(e, id)
        .expect("an inventory reference")
        .data;
    nvse::invref::set_data(e, id, data);
}

/// A list's 0x1C bytes of words (`m_data` and the presence bits).
fn words(e: &Engine, list: u32) -> Vec<u8> {
    let mut w = vec![0u8; 0x1c];
    e.mem.read(list + 4, &mut w);
    w
}

/// A stack left with only its count freed: the reference stands for no stack (its temporary
/// reference's copy cleared), the loop's write-back writes nothing into the freed list, and the
/// next commands on the reference reach no freed memory (a new stack is made).
#[test]
fn an_emptied_stack_is_forgotten_by_its_reference() {
    let mut e = setup();
    let refr = inventory(&mut e, 2, STACK);
    put_extra(&mut e, STACK, COUNT_X, 0x24, 2);
    sync(&mut e, refr);
    let id = id_of(&mut e, refr);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    set(&mut e, refr, &[]);
    assert_eq!(called(DESTROY_FN), vec![vec![STACK, 1]]);
    assert_eq!(stack_of(&mut e, refr), 0, "stands for no stack");
    assert_eq!(words(&e, refr + 0x44), vec![0u8; 0x1c], "its copy cleared");
    assert!(nvse::invref::write_ref_data_to_container(&mut e, id));
    assert!(
        still_poisoned(&e, STACK, 0x20),
        "nothing written into the freed list"
    );
    assert_eq!(has(&mut e, refr, 0), 0.0);
    // The old key stays in the map (its data emptied), so `MakeKey` draws again.
    keys(&mut e, &[0x1234, 0x5678]);
    set(&mut e, refr, &[num(3.0), num(5.0)]);
    assert!(entry(&mut e, 0x5678).is_some());
    let stack = stack_of(&mut e, refr);
    assert!(stack != 0 && stack != STACK, "a new stack");
    assert_eq!(get(&mut e, refr, &[num(3.0)]), 5.0);
    assert!(still_poisoned(&e, STACK, 0x20));
}

/// Every inventory reference standing for the freed stack forgets it, not only the command's.
#[test]
fn every_reference_on_a_freed_stack_forgets_it() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    let other = nvse::invref::create_entry(&mut e, ACTOR, WEAP, 1, STACK);
    assert!(id_of(&mut e, refr) < id_of(&mut e, other));
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    set(&mut e, refr, &[]);
    assert_eq!(called(DESTROY_FN), vec![vec![STACK, 1]]);
    assert_eq!(stack_of(&mut e, refr), 0);
    assert_eq!(stack_of(&mut e, other), 0);
    assert_eq!(words(&e, other + 0x44), vec![0u8; 0x1c]);
    assert_eq!(has(&mut e, other, 0), 0.0);
}

/// A stack the entry's list does not hold is not freed (`tList::Remove` found nothing).
#[test]
fn an_emptied_stack_the_entry_does_not_hold_is_not_freed() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    e.mem.set_u32(LISTS, STACK2);
    set(&mut e, refr, &[]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![STACK, 0x27]]);
    assert!(called(DESTROY_FN).is_empty());
    assert_eq!(stack_of(&mut e, refr), STACK);
    assert_eq!(super::super::list_items(&e, Ptr::new(LISTS)), vec![STACK2]);
}

/// A stack a deferred equip is still to run on stays (holding only its count) and in the entry.
#[test]
fn an_emptied_stack_with_a_deferred_action_on_it_is_not_freed() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    let id = id_of(&mut e, refr);
    assert!(nvse::invref::set_equipped(&mut e, id, true));
    set(&mut e, refr, &[]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![STACK, 0x27]]);
    assert!(called(DESTROY_FN).is_empty());
    assert_eq!(super::super::list_items(&e, Ptr::new(LISTS)), vec![STACK]);
    assert_eq!(stack_of(&mut e, refr), STACK);
}

/// The ExtraJIP taken off (and freed) from a stack that stays, as its first extra: the
/// reference's copy whose first extra it was is unlinked as the stack was and its bit cleared,
/// so the loop's write-back does not put the freed extra back.
#[test]
fn an_extra_jip_taken_off_a_kept_stack_leaves_the_references_copy() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    let id = id_of(&mut e, refr);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    let x = xjip(&e, STACK);
    assert_eq!(e.mem.u32(STACK + 4), x, "the stack's first extra");
    assert_eq!(e.mem.u32(refr + 0x48), x, "and its copy's");
    // A worn mark the game appends after it (the copy shares the chain, not the bit).
    e.mem.set_u8(OTHER_X + 4, 0x16);
    e.mem.set_u32(OTHER_X + 8, 0);
    e.mem.set_u32(x + 8, OTHER_X);
    e.mem.set_u8(STACK + 8 + 2, e.mem.u8(STACK + 8 + 2) | 0x40);
    set(&mut e, refr, &[]);
    assert!(called(DESTROY_FN).is_empty(), "a worn stack stays");
    assert_eq!(e.mem.u32(STACK + 4), OTHER_X);
    assert_eq!(
        e.mem.u32(refr + 0x48),
        OTHER_X,
        "the copy unlinked from the freed extra"
    );
    assert_eq!(
        e.mem.u8(refr + 0x44 + 8 + 4) & 0x80,
        0,
        "its ExtraJIP bit cleared"
    );
    assert!(nvse::invref::write_ref_data_to_container(&mut e, id));
    assert_eq!(e.mem.u32(STACK + 4), OTHER_X);
    assert_eq!(has(&mut e, refr, 0), 0.0);
}

/// A new stack made by the command: the reference's copy follows it, so the loop's write-back
/// keeps the count extra and the ExtraJIP (JIP's write-back would empty the stack).
#[test]
fn a_new_stack_survives_the_loops_write_back() {
    let mut e = setup();
    let refr = inventory(&mut e, 3, 0);
    let id = id_of(&mut e, refr);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    let stack = stack_of(&mut e, refr);
    assert_eq!(words(&e, refr + 0x44), words(&e, stack));
    assert!(nvse::invref::write_ref_data_to_container(&mut e, id));
    let first = e.mem.u32(stack + 4);
    assert_eq!((e.mem.u8(first + 4), e.mem.u32(first + 0xc)), (0x24, 3));
    assert_eq!(has(&mut e, refr, 0), 1.0);
    assert_eq!(get(&mut e, refr, &[num(3.0)]), 4.0);
}

/// An ExtraJIP added to a stack: a copy that matched the stack follows, so the write-back keeps
/// its presence bit; a copy the script had changed in the same step is left as it is.
#[test]
fn an_extra_jip_added_to_a_stack_survives_the_write_back_and_a_changed_copy_is_left() {
    let mut e = setup();
    let refr = inventory(&mut e, 2, STACK);
    put_extra(&mut e, STACK, COUNT_X, 0x24, 2);
    sync(&mut e, refr);
    let id = id_of(&mut e, refr);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    assert!(nvse::invref::write_ref_data_to_container(&mut e, id));
    assert_ne!(
        e.mem.u8(STACK + 8 + 4) & 0x80,
        0,
        "the stack's ExtraJIP bit kept"
    );
    let mut e = setup();
    let refr = inventory(&mut e, 2, STACK);
    put_extra(&mut e, STACK, COUNT_X, 0x24, 2);
    sync(&mut e, refr);
    e.mem.set_u8(refr + 0x44 + 8 + 2, 0x40);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    assert_eq!(
        e.mem.u8(refr + 0x44 + 8 + 2) & 0x40,
        0x40,
        "the change kept"
    );
    assert_eq!(e.mem.u8(refr + 0x44 + 8 + 4) & 0x80, 0, "not copied again");
}

/// A stack holding one extra that is no count (a worn mark) after the ExtraJIP goes stays
/// (`m_data->type == kXData_ExtraCount` fails).
#[test]
fn a_stack_with_one_other_extra_stays() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    put_extra(&mut e, STACK, OTHER_X, 0x16, 0);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    set(&mut e, refr, &[]);
    assert!(called(DESTROY_FN).is_empty());
    assert_eq!(super::super::list_items(&e, Ptr::new(LISTS)), vec![STACK]);
}

/// A stack holding only JIP's data goes (`!xData->m_data`).
#[test]
fn a_stack_holding_only_jip_data_goes() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    set(&mut e, refr, &[]);
    assert_eq!(calls_to(REMOVE_BY_TYPE_FN), vec![vec![STACK, 0x27]]);
    assert_eq!(super::super::list_items(&e, Ptr::new(LISTS)), vec![0]);
    assert_eq!(called(DESTROY_FN), vec![vec![STACK, 1]]);
}

/// Two stacks of the item: only the emptied one leaves the entry's list and is freed.
#[test]
fn two_stacks_only_the_emptied_one_goes() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, STACK);
    e.mem.set_u32(STACK2, LIST_VTBL);
    put_extra(&mut e, STACK2, OTHER_X, 0x16, 0);
    e.mem.set_u32(LISTS, STACK2);
    e.mem.set_u32(LISTS + 4, NODE2);
    e.mem.set_u32(NODE2, STACK);
    e.mem.set_u32(NODE2 + 4, 0);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    set(&mut e, refr, &[]);
    assert_eq!(super::super::list_items(&e, Ptr::new(LISTS)), vec![STACK2]);
    assert_eq!(called(DESTROY_FN), vec![vec![STACK, 1]]);
}

/// A value that stores nothing (a number at a text index) on a reference with no data still
/// makes the mod's data, the ExtraJIP, its key and the change mark (`dataMap[modIdx]`).
#[test]
fn nothing_at_a_text_index_still_makes_the_data() {
    let mut e = setup();
    set(&mut e, REF, &[num(18.0), num(5.0)]);
    assert_ne!(xjip(&e, REF + 0x44), 0);
    assert_eq!(
        calls_to(ADD_CHANGE_FN),
        vec![vec![SAVE_LOAD, REF, 0x400, 0]]
    );
    assert_eq!(has(&mut e, REF, 0), 1.0);
    let data = entry(&mut e, 0x1234).unwrap().data[&5].clone();
    assert_eq!(data, ExtraJipData::default());
}

/// An inventory reference whose container has no changes: no stack can be made, nothing stored.
#[test]
fn a_container_without_changes_gets_nothing() {
    let mut e = setup();
    let refr = inventory(&mut e, 1, 0);
    e.mem.set_u32(ACTOR + 0x48, 0);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    assert_eq!(stack_of(&mut e, refr), 0);
    assert!(calls_to(ADD_CHANGE_FN).is_empty());
    assert!(calls_to(ADD_EXTRA_FN).is_empty());
    assert!(called(RANDOM_FN).is_empty());
    assert_eq!((called(ENTER_FN).len(), called(LEAVE_FN).len()), (1, 1));
}

/// An entry without a list of stacks: the emptied stack is left (JIP removes from the null
/// list). Address 0 is mapped and holds the stack here, so a removal through it would show.
#[test]
fn an_entry_without_a_list_of_stacks_keeps_the_stack() {
    let mut e = setup();
    e.map(0, 0x1000);
    let refr = inventory(&mut e, 1, STACK);
    set(&mut e, refr, &[num(3.0), num(4.0)]);
    e.mem.set_u32(ENTRY, 0);
    e.mem.set_u32(0, STACK);
    e.mem.set_u32(4, 0);
    set(&mut e, refr, &[]);
    assert!(called(DESTROY_FN).is_empty());
    assert_eq!(e.mem.u32(0), STACK);
    assert_eq!(stack_of(&mut e, refr), STACK);
}

/// An ExtraJIP whose key has no entry in the map (a stale key) has no data; the lock is taken and
/// left once.
#[test]
fn has_with_a_key_missing_from_the_map() {
    let mut e = setup();
    set(&mut e, REF, &[num(3.0), num(1.0)]);
    form(&mut e, REF2, 0x3a, 0x0001_0002);
    e.mem.set_u32(REF2, PLAIN_VT);
    put_extra(&mut e, REF2 + 0x44, OTHER_X, 0x27, 0x9999);
    let (enters, leaves) = (called(ENTER_FN).len(), called(LEAVE_FN).len());
    assert_eq!(has(&mut e, REF2, 0), 0.0);
    assert_eq!(called(ENTER_FN).len(), enters + 1);
    assert_eq!(called(LEAVE_FN).len(), leaves + 1);
}

// ---- review round: the getter's return type and mod boundary; texts as JIP's XString ----

/// The getter hands the evaluator the result's type: a form, a text, else the default.
#[test]
fn the_getter_sets_the_expected_return_type() {
    let mut e = setup();
    set(&mut e, REF, &[num(2.0), reff(1)]);
    set(&mut e, REF, &[num(18.0), text("Doc")]);
    set(&mut e, REF, &[num(3.0), num(1.5)]);
    let expected = |e: &mut Engine, index: f64| {
        nvse::expr::with_evaluator(e, |e| {
            get(e, REF, &[num(index)]);
            nvse::expr::expected_return_type(e)
        })
    };
    assert_eq!(expected(&mut e, 2.0), Some(Returns::Form));
    assert_eq!(expected(&mut e, 18.0), Some(Returns::String));
    assert_eq!(expected(&mut e, 3.0), Some(Returns::Default));
}

/// Mod index 0xFE is read under the lock (only 0xFF and up are refused).
#[test]
fn the_getter_takes_mod_index_0xfe() {
    let mut e = setup();
    set(&mut e, REF, &[num(3.0), num(1.0)]);
    let enters = called(ENTER_FN).len();
    assert_eq!(get(&mut e, REF, &[num(3.0), num(254.0)]), 0.0);
    assert_eq!(called(ENTER_FN).len(), enters + 1);
}

/// A text kept as JIP's `XString` holds it: up to its first zero byte, its length cut to 16 bits.
#[test]
fn texts_are_kept_as_jips_strings_hold_them() {
    let mut d = ExtraJipData::default();
    store(&mut d, 18, Stored::Text(vec![b'a'; 0x1_0005]));
    assert_eq!(d.strings[0], b"aaaaa");
    store(&mut d, 19, Stored::Text(vec![b'b'; 0x1_0000]));
    assert!(d.strings[1].is_empty());
    store(&mut d, 19, Stored::Text(vec![b'c'; 0xffff]));
    assert_eq!(d.strings[1].len(), 0xffff);
    let mut e = setup();
    set(&mut e, REF, &[num(18.0), text("Doc\0Mitchell")]);
    let data = entry(&mut e, 0x1234).unwrap().data[&5].clone();
    assert_eq!(data.strings[0], b"Doc");
    let r = get(&mut e, REF, &[num(18.0)]);
    assert_eq!(string_result(&e, r), b"Doc");
}
