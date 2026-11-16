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
    let mut e = engine();
    extra_stand_ins(&mut e);
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
    e.register(DESTROY_FN, |_, w| {
        record(DESTROY_FN, w);
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
    // GetRefExtraData wants one or two numbers.
    assert_eq!(get(&mut e, REF, &[]), 0.0);
    assert_eq!(get(&mut e, REF, &[num(19.0), text("5")]), 0.0);
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
    e.register(0x0055_a2f0, |e, w| {
        e.mem.set_u32(w[0] + 0xc, 0xff00_0001);
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
    assert_eq!(xjip(&e, refr + 0x44), 0, "not on the temporary reference");
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
