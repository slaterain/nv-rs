use super::*;
const AREA: u32 = 0x0300_0000;
const OPEN: u32 = AREA + 0x1000;
const ACTOR: u32 = AREA + 0x2000;
const DATA: u32 = AREA + 0x3000;
const RESULT: u32 = DATA + 0x800;
#[derive(Default)]
struct Seen {
    title: Vec<u8>,
    created: usize,
}
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_9000, 0x1000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011c_d000, 0x1000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.mem.set_u32(PLAYER, ACTOR);
    e.mem.set_u32(0x011c_d078, 2);
    e.mem.set_u16(ACTOR + 0x500, 3);
    e.mem.set_u32(OPEN + 4, AREA + 0x5000);
    e.mem.set_u32(OPEN + 0x2c, AREA + 0x5100);
    e.mem.set_u32(OPEN + 0x4c, AREA + 0x5200);
    e.mem.set_u32(OPEN + 0x5c, 15);
    e.mem.set_u32(OPEN + 0x60, 1);
    let p1 = AREA + 0x6000;
    let p2 = p1 + 0x100;
    let p3 = p2 + 0x100;
    let n1 = AREA + 0x6400;
    let n2 = n1 + 8;
    for (p, level, available) in [(p1, 1, 1), (p2, 4, 1), (p3, 1, 0)] {
        e.mem.set_u8(p + 0x39, level);
        e.mem.set_u8(p + 0x74, available);
    }
    e.mem.set_u32(OPEN + 0xc4, p1);
    e.mem.set_u32(OPEN + 0xc8, n1);
    e.mem.set_u32(n1, p2);
    e.mem.set_u32(n1 + 4, n2);
    e.mem.set_u32(n2, p3);
    e.register(0x0070_6270, |e, a| {
        assert!(a.is_empty());
        assert!(FORCE_LIST.with(Cell::get));
        e.ext::<Seen>().created += 1;
        match e.mem.u32(DATA + 0x200) {
            1 => e.mem.set_u32(MENU, 0),
            2 => {
                e.mem.set_u32(MENU, OPEN);
                e.mem.set_u32(OPEN + 0xc4, 0);
            }
            3 => panic!("native Create stop"),
            _ => e.mem.set_u32(MENU, OPEN),
        }
        0xdeadu32.into_ret()
    });
    for address in [0x0078_5830, 0x00a0_12d0, 0x0078_7850] {
        e.register(address, |_, _| Ret::default());
    }
    e.register(0x00a0_1350, |e, a| {
        assert_eq!(a.len(), 4);
        e.ext::<Seen>().title = table::guest_text(e, a[2]);
        Ret::default()
    });
    e.register(0x0078_5150, |e, a| {
        assert_eq!(a[1], ACTOR);
        e.mem.u8(a[0] + 0x74).into_ret()
    });
    e.register(0x0087_f9f0, |e, a| {
        assert_eq!(a, [ACTOR]);
        u32::from(e.mem.u16(ACTOR + 0x500)).into_ret()
    });
    e.call_log = Some(Vec::new());
    e
}
fn integer(v: i32) -> Vec<u8> {
    [vec![b'n'], v.to_le_bytes().to_vec()].concat()
}
fn string(v: &[u8]) -> Vec<u8> {
    [(v.len() as u16).to_le_bytes().to_vec(), v.to_vec()].concat()
}
fn call(e: &mut Engine, op: u32, args: &[Vec<u8>]) -> f64 {
    let data = args.concat();
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, (data.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    e.mem.write(DATA + 6, &data);
    e.mem.set_u32(RESULT + 8, 4);
    e.mem.set_f64(RESULT, 77.0);
    let r = table::record_of(e, op);
    let w = [e.mem.u32(r + 0x14), DATA, 0, 0, 0, 0, RESULT, RESULT + 8];
    assert_eq!(e.call(e.mem.u32(r + 0x18), &w).eax & 0xff, 1);
    e.mem.f64(RESULT)
}
fn calls(e: &Engine, address: u32) -> Vec<Vec<u32>> {
    e.call_log
        .as_ref()
        .unwrap()
        .iter()
        .filter(|(at, _)| *at == address)
        .map(|(_, a)| a.clone())
        .collect()
}
#[test]
fn points_queries_actual_four_unsigned_fields_and_empty_perk_list() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x3ce6, &[]), -1.0);
    e.mem.set_u32(MENU, OPEN);
    for (offset, value) in [(0x54, 4), (0x58, 2), (0x5c, 15), (0x60, 3)] {
        e.mem.set_u32(OPEN + offset, value);
    }
    for (args, want) in [
        (vec![], 15.0),
        (vec![integer(1)], 3.0),
        (vec![integer(0), integer(1)], 4.0),
        (vec![integer(1), integer(1)], 2.0),
    ] {
        assert_eq!(call(&mut e, 0x3ce6, &args), want);
    }
    e.mem.set_u32(OPEN + 0xc4, 0);
    assert_eq!(call(&mut e, 0x3ce6, &[integer(1)]), 0.0);
    assert_eq!(call(&mut e, 0x3ce6, &[integer(1), integer(1)]), 2.0);
}
#[test]
fn show_zero_point_browser_calls_native_create_page_and_tile_operations_with_original_defaults() {
    let mut e = setup();
    assert_eq!(
        call(&mut e, 0x3ceb, &[integer(0), string(b"Browse perks")]),
        1.0
    );
    assert!(!FORCE_LIST.with(Cell::get));
    assert_eq!(e.mem.u32(OPEN + 0x60), 0);
    assert_eq!(e.mem.u32(OPEN + 0x5c), 0);
    assert_eq!(calls(&e, 0x0078_5830), [vec![OPEN, 1]]);
    assert_eq!(calls(&e, 0x00a0_12d0), [vec![AREA + 0x5200, 0xfa3, 0, 1]]);
    assert_eq!(e.ext::<Seen>().title, b"Browse perks");
    assert_eq!(calls(&e, 0x0078_5150).len(), 3);
    assert_eq!(calls(&e, 0x0087_f9f0).len(), 2);
    assert_eq!(e.ext::<Seen>().created, 1);
}
#[test]
fn show_count_caps_selectable_perks_and_default_minus_one_preserves_existing_max() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x3ceb, &[integer(20)]), 1.0);
    assert_eq!(e.mem.u32(OPEN + 0x60), 1);
    assert!(calls(&e, 0x00a0_1350).is_empty());
    e.mem.set_u32(OPEN + 0x60, 7);
    e.call_log = Some(Vec::new());
    assert_eq!(call(&mut e, 0x3ceb, &[]), 1.0);
    assert_eq!(e.mem.u32(OPEN + 0x60), 7);
    assert!(calls(&e, 0x0078_5150).is_empty());
}
#[test]
fn failed_create_and_empty_list_close_native_menu_and_unwind_restores_scope() {
    let mut e = setup();
    e.mem.set_u32(DATA + 0x200, 1);
    assert_eq!(call(&mut e, 0x3ceb, &[]), 0.0);
    assert!(calls(&e, 0x0078_5830).is_empty());
    e.mem.set_u32(DATA + 0x200, 2);
    assert_eq!(call(&mut e, 0x3ceb, &[]), 0.0);
    assert_eq!(
        calls(&e, 0x00a0_12d0),
        [vec![AREA + 0x5000, 0xfa3, 1f32.to_bits(), 1]]
    );
    assert_eq!(calls(&e, 0x0078_5830), [vec![OPEN, 2]]);
    e.mem.set_u32(DATA + 0x200, 3);
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(&mut e, 0x3ceb, &[])));
    assert!(stop.is_err());
    assert!(!FORCE_LIST.with(Cell::get));
    with_forced_list(|| {
        assert!(FORCE_LIST.with(Cell::get));
        with_forced_list(|| assert!(FORCE_LIST.with(Cell::get)));
        assert!(FORCE_LIST.with(Cell::get));
    });
    assert!(!FORCE_LIST.with(Cell::get));
}
#[test]
fn persistent_native_hooks_initialize_and_limit_selection_before_original_alpha_read() {
    let mut e = setup();
    for address in [0x0078_5087, 0x0078_5acc, 0x0078_5b0f, 0x0078_5b2c] {
        assert_eq!(
            e.site(address, &[OPEN, 0, 0]),
            None,
            "query-only install preserves native branches"
        );
    }
    assert_eq!(call(&mut e, 0x3ceb, &[integer(0)]), 1.0);
    assert_eq!(e.site(0x0078_4f26, &[]), None);
    with_forced_list(|| assert_eq!(e.site(0x0078_4f26, &[]), Some(1)));
    assert_eq!(e.site(0x0078_5087, &[]), Some(0));
    e.mem.set_u16(ACTOR + 0x500, 4);
    assert_eq!(e.site(0x0078_5087, &[]), Some(1));
    e.mem.set_u32(MENU, OPEN);
    for (max, assigned, click, alpha, resets) in [
        (0, 0, 0, 0, 0),
        (0, 1, 1, 0, 0),
        (2, 2, 1, 0, 0),
        (2, 1, 1, 1, 0),
        (1, 1, 1, 1, 1),
    ] {
        e.call_log = Some(Vec::new());
        e.mem.set_u32(OPEN + 0x60, max);
        e.mem.set_u32(OPEN + 0x58, assigned);
        assert_eq!(e.site(0x0078_5acc, &[OPEN]), Some(click));
        assert_eq!(e.site(0x0078_5b0f, &[OPEN, 0, 0]), Some(alpha));
        assert_eq!(calls(&e, 0x0078_7850).len(), resets);
        assert_eq!(e.site(0x0078_5b2c, &[OPEN]), Some(1));
    }
}
#[test]
fn command_metadata_preserves_original_optional_contract_and_alias() {
    let e = setup();
    for (op, kinds) in [(0x3ce6, [1, 1]), (0x3ceb, [1, 0])] {
        let r = table::record_of(&e, op);
        assert_eq!(e.mem.u16(r + 0x12), 2);
        assert_eq!(e.mem.u16(r + 0x10), 0);
        let p = e.mem.u32(r + 0x14);
        for (i, kind) in kinds.into_iter().enumerate() {
            assert_eq!(e.mem.u32(p + i as u32 * 12 + 4), kind);
            assert_eq!(e.mem.u32(p + i as u32 * 12 + 8), 1);
        }
    }
    let r = table::record_of(&e, 0x3ceb);
    assert_eq!(
        table::guest_text(&e, e.mem.u32(r + 4)),
        b"IfIDecideToGoWithYourFunctionWhatAreThePerks"
    );
}
