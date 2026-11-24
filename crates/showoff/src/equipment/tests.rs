use super::*;
pub(crate) const AREA: u32 = 0x0300_0000;
pub(crate) const ACTOR: u32 = AREA + 0x1000;
pub(crate) const DATA: u32 = AREA + 0x2000;
pub(crate) const RESULT: u32 = DATA + 0x800;
const EXTRA: u32 = AREA + 0x3000;
const CHANGE_DATA: u32 = EXTRA + 0x100;
const REF_VT: u32 = AREA + 0x4000;
const ACTOR_VT: u32 = AREA + 0x4500;
#[derive(Default)]
pub(crate) struct Seen {
    pub entry: Vec<u32>,
    pub weight: f64,
    pub heavy: f32,
    pub pack: f32,
    pub calls: Vec<(u32, Vec<u32>)>,
}
pub(crate) fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0102_c000, 0x1000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011d_8000, 0x1000);
    e.map(0x011c_6000, 0x1000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.mem.set_u32(PLAYER, ACTOR);
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u8(ACTOR + 4, 0x3b);
    e.mem.set_u32(ACTOR_VT + 0x100, 0x008d_0360);
    e.mem.set_u32(WEAPON_VTABLE + 0x94, AREA + 0x8008);
    e.mem.set_u32(ACTOR + 0x68, AREA + 0x5000);
    e.mem.set_u32(AREA + 0x5000, AREA + 0x5400);
    e.mem.set_u32(AREA + 0x5400 + 0x148, AREA + 0x8000);
    e.mem.set_u32(EXTRA + 0xc, CHANGE_DATA);
    e.register(0x008d_0360, |_, _| true.into_ret());
    e.register(0x0040_1000, |e, a| e.mem.alloc(a[0]).into_ret());
    e.register(0x0040_1030, |_, _| Ret::default());
    e.mem.set_u32(REF_VT + 0xf0, 0x008d_0360);
    e.mem.set_u32(REF_VT + 0x10, AREA + 0x8004);
    e.register(AREA + 0x8004, |e, a| {
        e.ext::<Seen>().calls.push((0x10, a.to_vec()));
        Ret::default()
    });
    e.register(0x0055_a2f0, |e, a| {
        let id = e.mem.u32(DATA + 0x100).max(0xff00_0800);
        e.mem.set_u32(DATA + 0x100, id + 1);
        e.mem.set_u32(a[0], REF_VT);
        e.mem.set_u8(a[0] + 4, 0x3a);
        e.mem.set_u32(a[0] + 0xc, id);
        a[0].into_ret()
    });
    e.register(0x0041_0220, |_, a| {
        assert_eq!(a, [ACTOR + 0x44, 0x15]);
        EXTRA.into_ret()
    });
    e.register(0x00ec_43fb, |_, a| {
        assert_eq!(a[1..], [0, 0x0118_3b2c, 0x0118_4bdc, 0]);
        a[0].into_ret()
    });
    e.register(0x0048_0db0, |e, a| {
        if e.mem.u32(a[0]) == WEAPON_VTABLE {
            0u32.into_ret()
        } else {
            (a[0] + 0x100).into_ret()
        }
    });
    e.register(AREA + 0x8000, |e, _| e.mem.u32(DATA + 0x104).into_ret());
    e.register(0x004b_da70, |_, a| {
        assert_eq!(a[1], 4);
        true.into_ret()
    });
    e.register(0x004b_e380, |e, a| {
        assert_eq!(a[1], 1);
        e.ext::<Seen>().weight.into_ret()
    });
    e.register(0x0048_ebc0, |e, a| {
        e.ext::<Seen>().calls.push((0x0048_ebc0, a.to_vec()));
        f64::from(e.mem.f32(a[0] + 0x80)).into_ret()
    });
    e.register(0x005e_58f0, |e, a| {
        e.ext::<Seen>().calls.push((0x005e_58f0, a.to_vec()));
        let value = if a[0] == 73 {
            e.ext::<Seen>().heavy
        } else {
            assert_eq!(a[0], 55);
            e.ext::<Seen>().pack
        };
        e.mem.set_f32(a[2], value);
        Ret::default()
    });
    e.ext::<Seen>().heavy = 1.0;
    e.mem.set_f32(0x011c_647c, 2.0);
    e.mem.set_f32(0x011c_64ac, 0.5);
    e
}
pub(crate) fn item(e: &mut Engine, id: u32, mask: u32, weapon: bool) -> (u32, u32) {
    let item = e.mem.alloc(0x400);
    let vt = if weapon {
        WEAPON_VTABLE
    } else {
        let vt = e.mem.alloc(0x110);
        e.mem.set_u32(vt + 0x94, AREA + 0x8008);
        vt
    };
    e.mem.set_u32(item, vt);
    e.mem.set_u32(item + 0xc, id);
    e.mem.set_u8(item + 4, if weapon { 0x28 } else { 0x18 });
    e.mem.set_u32(item + 0x104, mask);
    e.register(AREA + 0x8008, |e, a| {
        (e.mem.u32(a[0] + 8) & 0x400 != 0).into_ret()
    });
    let xdata = e.mem.alloc(0x20);
    e.mem.set_u8(xdata + 0xa, 1 << 6);
    (item, xdata)
}
fn entries(e: &mut Engine, items: &[(u32, u32)]) -> Vec<u32> {
    let head = e.mem.alloc(8);
    e.mem.set_u32(CHANGE_DATA, head);
    let mut node = head;
    let mut entries = Vec::new();
    for (i, (form, xdata)) in items.iter().enumerate() {
        let entry = e.mem.alloc(12);
        let lists = e.mem.alloc(8);
        e.mem.set_u32(lists, *xdata);
        e.mem.set_u32(entry, lists);
        e.mem.set_u32(entry + 4, 7);
        e.mem.set_u32(entry + 8, *form);
        e.mem.set_u32(node, entry);
        let next = if i + 1 == items.len() {
            0
        } else {
            e.mem.alloc(8)
        };
        e.mem.set_u32(node + 4, next);
        node = next;
        entries.push(entry);
    }
    entries
}
pub(crate) fn integer(v: i32) -> Vec<u8> {
    [vec![b'n'], v.to_le_bytes().to_vec()].concat()
}
pub(crate) fn call(e: &mut Engine, op: u32, target: u32, args: &[Vec<u8>]) -> f64 {
    let data = args.concat();
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, (data.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    e.mem.write(DATA + 6, &data);
    e.mem.set_u32(RESULT + 8, 4);
    let r = table::record_of(e, op);
    let a = [
        e.mem.u32(r + 0x14),
        DATA,
        target,
        0,
        0,
        0,
        RESULT,
        RESULT + 8,
    ];
    e.call(e.mem.u32(r + 0x18), &a);
    e.mem.f64(RESULT)
}
#[test]
fn real_equipped_stack_filtering_defaults_flags_dedup_and_first_worn() {
    let mut e = setup();
    let a = item(&mut e, 1, 4, false);
    let b = item(&mut e, 2, 1, false);
    let c = item(&mut e, 3, 0, false);
    let w = item(&mut e, 4, 32, true);
    e.mem.set_u32(b.0 + 0x108, 0x40);
    e.mem.set_u32(c.0 + 8, 0x400);
    entries(&mut e, &[a, b, c, w, a]);
    assert_eq!(equipped(&mut e, ACTOR, 0), [a, c, w]);
    assert_eq!(equipped(&mut e, ACTOR, 4), [a, c]);
    assert_eq!(
        equipped(&mut e, ACTOR, (1 << 21) | (1 << 22) | 0xfffff),
        [a, b, w]
    );
    e.mem.set_u8(a.1 + 0xa, 0);
    assert_eq!(equipped(&mut e, ACTOR, 32), [c, w]);
}
#[test]
fn genuine_inventory_refs_hold_selected_extra_data_counts_and_release_on_nvse_cleanup() {
    let mut e = setup();
    let a = item(&mut e, 0x1001, 4, false);
    let w = item(&mut e, 0x1002, 32, true);
    let entries = entries(&mut e, &[a, w]);
    e.mem.set_u32(DATA + 0x104, entries[1]);
    let array = call(&mut e, 0x3cf7, ACTOR, &[]) as u32;
    let vars::Container::Array(items) = &vars::array(&e, array).unwrap().elements else {
        panic!()
    };
    let ids = items
        .iter()
        .map(|i| match i.data {
            vars::Data::Form(id) => id,
            _ => panic!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 2);
    for (id, (form, xdata)) in ids.into_iter().zip([a, w]) {
        let reference = invref::get(&mut e, id).unwrap();
        assert_eq!(reference.container, ACTOR);
        assert_eq!(reference.data.item, form);
        assert_eq!(reference.data.xdata, xdata);
        let entry = reference.data.entry;
        assert_eq!(e.mem.i32(entry + 4), 1);
    }
    call(&mut e, 0x3d32, ACTOR, &[]);
    let id = e.mem.u32(RESULT);
    let reference = invref::get(&mut e, id).unwrap();
    let entry = reference.data.entry;
    assert_eq!(reference.data.xdata, w.1);
    assert_eq!(e.mem.i32(entry + 4), 7);
    assert_eq!(table::return_type(&e, 0x3d32), Returns::Form);
    invref::clear(&mut e);
    assert!(!invref::any(&mut e));
    assert_eq!(
        e.ext::<Seen>()
            .calls
            .iter()
            .filter(|(at, _)| *at == 0x10)
            .count(),
        3
    );
}
#[test]
fn weight_uses_real_weapon_mod_heavy_packrat_threshold_and_float_accumulation() {
    let mut e = setup();
    let w = item(&mut e, 1, 32, true);
    let a = item(&mut e, 2, 4, false);
    let es = entries(&mut e, &[w, a]);
    e.mem.set_u32(DATA + 0x104, es[0]);
    e.mem.set_f32(a.0 + 0x80, 2.0);
    e.ext::<Seen>().weight = 12.0;
    e.ext::<Seen>().heavy = 0.5;
    e.ext::<Seen>().pack = 1.0;
    assert_eq!(calculated_weight(&mut e, ACTOR, 0, 0.0), 7.0);
    assert_eq!(calculated_weight(&mut e, ACTOR, 32, 0.0), 6.0);
    let zero_float = [vec![b'z'], 0.0f64.to_le_bytes().to_vec()].concat();
    assert_eq!(call(&mut e, 0x3cce, ACTOR, &[zero_float, integer(32)]), 6.0);
    assert_eq!(calculated_weight(&mut e, ACTOR, 0, 6.0), 6.0);
    let calls = &e.ext::<Seen>().calls;
    assert!(calls
        .iter()
        .any(|(at, a)| *at == 0x005e_58f0 && a[0] == 73 && a[1] == ACTOR));
    assert!(calls.iter().any(|(at, a)| *at == 0x0048_ebc0 && a[1] == 0));
}
