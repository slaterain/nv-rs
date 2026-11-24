use super::*;
const AREA: u32 = 0x0300_0000;
const RESULT: u32 = AREA + 0x100;
const ACTOR: u32 = AREA + 0x1000;
const NPC: u32 = AREA + 0x2000;
const OTHER: u32 = AREA + 0x3000;
const SCRIPT: u32 = AREA + 0x4000;
#[derive(Default)]
struct Seen {
    calls: Vec<(u32, Vec<u32>)>,
}
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_8000, 0x1000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011c_3000, 0x1000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.mem.set_u32(PLAYER, ACTOR);
    e.mem.set_u8(SCRIPT + 0xf, 5);
    let handler = e.mem.alloc(0x200);
    e.mem.set_u32(HANDLER, handler);
    for actor in [ACTOR, NPC, OTHER] {
        let vt = e.mem.alloc(0x500);
        let av = e.mem.alloc(0x40);
        e.mem.set_u32(actor, vt);
        e.mem.set_u32(actor + 0xa4, av);
        e.mem.set_u32(vt + 0x100, AREA + 0x8000);
        e.mem.set_u32(vt + 0x4a0, AREA + 0x8004);
        e.mem.set_u32(av + 0x28, AREA + 0x8008);
        e.mem.set_u16(actor + 0x500, 3);
    }
    e.register(AREA + 0x8000, |_, a| (a[0] != OTHER).into_ret());
    e.register(AREA + 0x8004, |e, a| {
        e.ext::<Seen>().calls.push((0x4a0, a.to_vec()));
        u32::from(e.mem.u8(a[1] + 0x70)).into_ret()
    });
    e.register(AREA + 0x8008, |e, a| {
        e.ext::<Seen>().calls.push((0x28, a.to_vec()));
        u32::from(e.mem.u16(a[0] - 0xa4 + 0x500)).into_ret()
    });
    e.register(0x005e_ba00, |e, a| {
        e.ext::<Seen>().calls.push((0x005e_ba00, a.to_vec()));
        (e.mem.u8(a[0] + 0x71) & 1 == 0).into_ret()
    });
    e.register(0x005e_b5e0, |e, a| {
        e.ext::<Seen>().calls.push((0x005e_b5e0, a.to_vec()));
        (e.mem.u8(a[0] + 0x71) & 2 == 0).into_ret()
    });
    e.register(0x0078_5150, |e, a| {
        e.ext::<Seen>().calls.push((0x0078_5150, a.to_vec()));
        (e.mem.u8(a[0] + 0x71) & 4 == 0).into_ret()
    });
    e.register(0x005e_23f0, |_, _| false.into_ret());
    e
}
fn perk(e: &mut Engine, id: u32, level: i8, rank: u8, owned: u8, trait_: bool, flags: u8) -> u32 {
    let p = e.mem.alloc(0x80);
    e.mem.set_u32(p + 0xc, id);
    e.mem.set_u8(p + 0x39, level as u8);
    e.mem.set_u8(p + 0x3a, rank);
    e.mem.set_u8(p + 0x70, owned);
    e.mem.set_u8(p + 0x38, u8::from(trait_));
    e.mem.set_u8(p + 0x71, flags);
    p
}
fn chain(e: &mut Engine, perks: &[u32]) {
    let mut node = e.mem.u32(HANDLER) + 0x178;
    for (i, p) in perks.iter().enumerate() {
        e.mem.set_u32(node, *p);
        let next = if i + 1 == perks.len() {
            0
        } else {
            e.mem.alloc(8)
        };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
}
fn call(e: &mut Engine, target: u32) -> Vec<u32> {
    let r = table::record_of(e, 0x31a6);
    let w = [0, 0, target, 0, SCRIPT, 0, RESULT, 0];
    e.call(e.mem.u32(r + 0x18), &w);
    let id = e.mem.f64(RESULT) as u32;
    assert_eq!(vars::array_owning_mod(e, id), 5);
    let vars::Container::Array(entries) = &vars::array(e, id).unwrap().elements else {
        panic!("must be packed");
    };
    entries
        .iter()
        .map(|entry| match entry.data {
            vars::Data::Form(id) => id,
            _ => panic!("form element expected"),
        })
        .collect()
}
#[test]
fn available_perks_uses_real_actor_rank_conditions_flags_signed_levels_and_order() {
    let mut e = setup();
    let mut ps = Vec::new();
    for (i, (level, ranks, owned, trait_, flags)) in [
        (1, 1, 0, false, 0),
        (0, 1, 0, false, 0),
        (-1, 1, 0, false, 0),
        (1, 1, 1, false, 0),
        (1, 1, 0, true, 0),
        (1, 1, 0, false, 1),
        (1, 1, 0, false, 2),
        (1, 1, 0, false, 4),
        (4, 1, 0, false, 0),
        (2, 2, 1, false, 0),
    ]
    .into_iter()
    .enumerate()
    {
        ps.push(perk(
            &mut e,
            0x500 + i as u32,
            level,
            ranks,
            owned,
            trait_,
            flags,
        ));
    }
    chain(&mut e, &ps);
    assert_eq!(call(&mut e, 0), [0x500, 0x509]);
    assert_eq!(
        e.ext::<Seen>()
            .calls
            .iter()
            .filter(|(a, _)| *a == 0x28)
            .count(),
        1
    );
    e.mem.set_u16(NPC + 0x500, 10);
    assert_eq!(call(&mut e, NPC), [0x500, 0x508, 0x509]);
    assert_eq!(call(&mut e, OTHER), [0x500, 0x509]);
}
#[test]
fn null_actor_still_returns_genuine_empty_array_and_metadata_has_array_return() {
    let mut e = setup();
    e.mem.set_u32(PLAYER, 0);
    assert!(call(&mut e, 0).is_empty());
    let r = table::record_of(&e, 0x31a6);
    assert_eq!(e.mem.u16(r + 0x12), 0);
    assert_eq!(table::return_type(&e, 0x31a6), Returns::Array);
    assert!(e.ext::<Seen>().calls.is_empty());
}
#[test]
fn pipboy_query_and_condition_read_actual_unsigned_mode_with_null_singleton() {
    let mut e = setup();
    let manager = e.mem.alloc(0x500);
    for (value, want) in [(0, 0.0), (0x8000_0000, 2147483648.0)] {
        e.mem.set_u32(INTERFACE, manager);
        e.mem.set_u32(manager + 0x4bc, value);
        pipboy(&mut e, &[0, 0, 0, 0, 0, 0, RESULT, 0]);
        assert_eq!(e.mem.f64(RESULT), want);
        pipboy_eval(&mut e, &[0, 0, 0, RESULT]);
        assert_eq!(e.mem.f64(RESULT), want);
    }
    e.mem.set_u32(INTERFACE, 0);
    pipboy_eval(&mut e, &[0, 0, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 0.0);
    let r = table::record_of(&e, 0x3168);
    assert_ne!(e.mem.u32(r + 0x1c), 0);
}
