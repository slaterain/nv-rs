use super::*;
use crate::equipment::tests::{self as fixture, Seen, ACTOR, DATA, RESULT};
fn setup() -> Engine {
    let mut e = fixture::setup();
    install(&mut e);
    e.register(0x0048_1f30, |_, a| {
        (a[0] == 0x18 || a[0] == 0x28).into_ret()
    });
    e.register(0x004b_d400, |e, a| {
        let values = (0..3).map(|i| e.mem.u32(a[0] + i * 4)).collect();
        e.ext::<Seen>().entry = values;
        123.75f64.into_ret()
    });
    e.register(0x0072_ed00, |e, a| {
        e.ext::<Seen>().entry = a.to_vec();
        (-42.125f64).into_ret()
    });
    e
}
fn world(e: &mut Engine, item: u32) -> u32 {
    let reference = e.mem.alloc(0x68);
    e.mem.set_u32(reference + 0xc, 0x500);
    e.mem.set_u32(reference + 0x20, item);
    reference
}
#[test]
fn world_reference_builds_real_entry_and_inventory_reference_copies_actual_count_and_extra_lists() {
    let mut e = setup();
    let (item, xdata) = fixture::item(&mut e, 0x100, 4, false);
    let reference = world(&mut e, item);
    assert_eq!(value(&mut e, reference, false, 0), 123.75);
    let entry = e.ext::<Seen>().entry.clone();
    assert_eq!(&entry[1..], [1, item]);
    assert_eq!(e.mem.u32(entry[0]), reference + 0x44);
    assert_eq!(e.mem.u32(entry[0] + 4), 0);
    let reference = invref::create_entry(&mut e, ACTOR, item, 7, xdata);
    assert_eq!(fixture::call(&mut e, OP, reference, &[]), 123.75);
    let entry = e.ext::<Seen>().entry.clone();
    assert_eq!(&entry[1..], [7, item]);
    assert_eq!(e.mem.u32(entry[0]), xdata);
}
#[test]
fn barter_source_uses_selected_tile_but_prices_requested_inventory_entry() {
    let mut e = setup();
    let (item, xdata) = fixture::item(&mut e, 0x100, 4, false);
    let reference = invref::create_entry(&mut e, ACTOR, item, 7, xdata);
    let id = e.mem.u32(reference + 0xc);
    let entry = invref::get(&mut e, id).unwrap().data.entry;
    let menu = e.mem.alloc(0x200);
    let list = e.mem.alloc(0x30);
    let n2 = e.mem.alloc(8);
    let i1 = e.mem.alloc(8);
    let i2 = e.mem.alloc(8);
    e.mem.set_u32(0x011d_8fa4, menu);
    e.mem.set_u32(menu + 0x108, list);
    e.mem.set_u32(list + 4, i1);
    e.mem.set_u32(list + 8, n2);
    e.mem.set_u32(n2, i2);
    e.mem.set_u32(i1, 0x91);
    e.mem.set_u32(i1 + 4, 0x1111);
    e.mem.set_u32(i2, 0x92);
    e.mem.set_u32(i2 + 4, 0x2222);
    e.mem.set_u32(list + 0x10, 0x92);
    assert_eq!(value(&mut e, reference, true, 0), -42.125);
    assert_eq!(e.ext::<Seen>().entry, [0x92, entry]);
    e.mem.set_u32(menu + 0x108, 0);
    assert_eq!(value(&mut e, reference, true, 0), -1.0);
}
#[test]
fn invalid_base_returns_minus_one_and_condition_uses_optional_base_priority() {
    let mut e = setup();
    let (a, _) = fixture::item(&mut e, 0x100, 4, false);
    let (b, _) = fixture::item(&mut e, 0x101, 4, false);
    let reference = world(&mut e, a);
    let vt = e.mem.u32(b);
    e.mem.set_u32(vt + 0xf0, DATA + 0x900);
    e.register(DATA + 0x900, |_, _| false.into_ret());
    condition(&mut e, &[reference, 0, b, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 123.75);
    assert_eq!(e.ext::<Seen>().entry[2], b);
    e.mem.set_u8(a + 4, 0x47);
    assert_eq!(value(&mut e, reference, false, 0), -1.0);
}
#[test]
fn record_optional_contract_and_eval_are_available() {
    let e = setup();
    let r = table::record_of(&e, OP);
    assert_eq!(e.mem.u16(r + 0x10), 0);
    assert_eq!(e.mem.u16(r + 0x12), 2);
    let p = e.mem.u32(r + 0x14);
    assert_eq!(e.mem.u32(p + 4), 1);
    assert_eq!(e.mem.u32(p + 16), 0x15);
    assert_eq!(e.mem.u32(p + 8), 1);
    assert_eq!(e.mem.u32(p + 20), 1);
    assert_ne!(e.mem.u32(r + 0x1c), 0);
}
