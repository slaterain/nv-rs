use super::*;

const DATA: u32 = 0x0300_0000;
const PROJECTILE: u32 = DATA + 0x1000;
const BASE: u32 = DATA + 0x1200;
const SOURCE: u32 = DATA + 0x1400;
const WEAPON: u32 = DATA + 0x1600;
const TARGET: u32 = DATA + 0x1800;
const SCRIPT: u32 = DATA + 0x2000;
const FIRST: u32 = DATA + 0x3000;
const NODE: u32 = DATA + 0x3100;
const SECOND: u32 = DATA + 0x3200;
const VTABLE: u32 = DATA + 0x4000;
const HEAP_TOP: u32 = DATA + 0x4ff0;
const POSITION: [u32; 3] = [0x8000_0000, 0x7fc1_2345, 0x3f80_0001];

#[derive(Default)]
struct Seen {
    args: Vec<Vec<u32>>,
    positions: Vec<[u32; 3]>,
}
fn alloc(e: &mut Engine, size: u32) -> u32 {
    let at = e.mem.u32(HEAP_TOP).max(DATA + 0x5000);
    e.mem.set_u32(HEAP_TOP, at + size.next_multiple_of(0x10));
    e.mem.write(at, &vec![0; size as usize]);
    at
}
fn position(e: &Engine) -> [u32; 3] {
    std::array::from_fn(|i| e.mem.u32(PROJECTILE + 0x30 + i as u32 * 4))
}
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(DATA, 0x20000);
    e.map(0x0118_e000, 0x9000);
    for page in [0x011c_3000, 0x011f_6000, 0x011d_a000] {
        e.map(page, 0x1000);
    }
    nvse::table::install(&mut e);
    nvse::functions::install(&mut e);
    install(&mut e);
    e.register(0x0040_1000, |e, a| alloc(e, a[0]).into_ret());
    for (form, id) in [
        (PROJECTILE, 0x1001),
        (BASE, 0x1002),
        (SOURCE, 0x1003),
        (WEAPON, 0x1004),
        (TARGET, 0x1005),
    ] {
        e.mem.set_u32(form, VTABLE);
        e.mem.set_u32(form + 0xc, id);
    }
    e.mem.set_u32(VTABLE + 0xf0, DATA + 0x4100);
    e.register(DATA + 0x4100, |_, a| {
        u32::from(a[0] != BASE && a[0] != WEAPON).into_ret()
    });
    e.register(0x0048_39c0, |e, a| {
        [PROJECTILE, BASE, SOURCE, WEAPON, TARGET, SCRIPT]
            .into_iter()
            .find(|&form| e.mem.u32(form + 0xc) == a[0])
            .unwrap_or(0)
            .into_ret()
    });
    e.mem.set_u32(PROJECTILE + 0x20, BASE);
    e.mem.set_u32(PROJECTILE + 0xf8, WEAPON);
    e.mem.set_u32(PROJECTILE + 0xfc, SOURCE);
    e.mem.set_u8(PROJECTILE + 0x90, 1);
    e.mem.set_u32(PROJECTILE + 0x88, FIRST);
    e.mem.set_u32(FIRST, TARGET);
    for (i, bits) in POSITION.into_iter().enumerate() {
        e.mem.set_u32(FIRST + 4 + i as u32 * 4, bits);
    }
    e
}

/// Three reference locals, dispatched through the real xNVSE UDF binder.
fn listener(e: &mut Engine) {
    let data = SCRIPT + 0x100;
    let mut code = vec![0x1d, 0, 0, 0, 0x10, 0];
    let block = [0x0d, 0, 0, 0, 0, 0, 1, 3, 1, 0, 4, 2, 0, 4, 3, 0, 4];
    code.extend_from_slice(&(block.len() as u16).to_le_bytes());
    code.extend(block);
    code.resize(0x40, 0);
    e.mem.write(data, &code);
    e.mem.set_u8(SCRIPT + 4, 0x11);
    e.mem.set_u32(SCRIPT + 0xc, 0x1006);
    e.mem.set_u32(SCRIPT + 0x30, data);
    e.mem.set_u32(SCRIPT + 0x20, code.len() as u32);
    e.register(nvse::functions::CREATE_EVENT_LIST, |e, a| {
        let list = alloc(e, 0x14);
        e.mem.set_u32(list, a[0]);
        let mut next = 0;
        for id in [3, 2, 1] {
            let local = alloc(e, 0x10);
            e.mem.set_u32(local, id);
            let node = alloc(e, 8);
            e.mem.set_u32(node, local);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
        e.mem.set_u32(list + 0xc, next);
        list.into_ret()
    });
    e.register(nvse::functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, a| {
        let mut args = vec![a[1]];
        for id in [1, 2, 3] {
            let local = nvse::args::event_list_variable(e, a[2], id);
            args.push(e.mem.u32(local + 8));
        }
        let position = position(e);
        e.ext::<Seen>().args.push(args);
        e.ext::<Seen>().positions.push(position);
        Ret::default()
    });
    let mut callback = events::Callback::new(SCRIPT);
    // NVSEPlus uses 0::projectile-list and 2::weapon-list filters.
    callback.filters.insert(0, nvse::vars::Data::Form(0x1002));
    callback.filters.insert(2, nvse::vars::Data::Form(0x1004));
    assert!(events::set_handler(e, EVENT, callback, 1, None));
}

#[test]
fn typed_dispatch_uses_projectile_calling_ref_and_corrects_position_before_callbacks() {
    let mut e = setup();
    listener(&mut e);
    let at = events::find(&mut e, EVENT).unwrap();
    let info = events::state_info(&mut e, at);
    assert_eq!(
        info.params,
        [
            events::param::REFERENCE,
            events::param::BASE_FORM,
            events::param::REFERENCE
        ]
    );
    assert_eq!(info.flags, events::FLAG_FLUSH_ON_LOAD);
    assert_eq!(e.site_owners(IMPACT.addr()), [Owner::ShowOff]);
    assert_eq!(e.site(IMPACT.addr(), &[PROJECTILE, 1]), None);
    assert_eq!(
        e.ext::<Seen>().args,
        [vec![PROJECTILE, 0x1003, 0x1004, 0x1005]]
    );
    assert_eq!(e.ext::<Seen>().positions, [POSITION]);
    e.mem.set_u32(PROJECTILE + 0xf8, BASE);
    e.site(IMPACT.addr(), &[PROJECTILE, 1]);
    assert_eq!(
        e.ext::<Seen>().args.len(),
        1,
        "typed weapon filter rejects another base form"
    );
    events::clear_flush_on_load(&mut e);
    e.mem.set_u32(PROJECTILE + 0xf8, WEAPON);
    e.site(IMPACT.addr(), &[PROJECTILE, 1]);
    assert_eq!(
        e.ext::<Seen>().args.len(),
        1,
        "native event flushes even ordinary handlers"
    );
}

#[test]
fn head_position_and_first_nonnull_reference_are_separate_and_impacted_gate_is_exact() {
    let mut e = setup();
    listener(&mut e);
    e.mem.set_u32(FIRST, 0);
    e.mem.set_u32(PROJECTILE + 0x8c, NODE);
    e.mem.set_u32(NODE, SECOND);
    e.mem.set_u32(SECOND, TARGET);
    e.site(IMPACT.addr(), &[PROJECTILE, 1]);
    assert_eq!(
        e.ext::<Seen>().args[0][3],
        0x1005,
        "skip null refr in nonnull first data"
    );
    assert_eq!(
        position(&e),
        POSITION,
        "position still comes from head data"
    );
    e.mem.set_u32(PROJECTILE + 0x88, 0);
    e.site(IMPACT.addr(), &[PROJECTILE, 1]);
    assert_eq!(
        e.ext::<Seen>().args[1][3],
        0x1005,
        "skip a null data list node"
    );
    e.mem.set_u8(PROJECTILE + 0x90, 0);
    e.site(IMPACT.addr(), &[PROJECTILE, 1]);
    assert_eq!(
        e.ext::<Seen>().args[2][3],
        0,
        "hasImpacted controls GetImpactRef only"
    );
}

#[test]
fn nonimpact_never_reads_projectile_and_type_two_does_not_reposition() {
    let mut e = setup();
    assert_eq!(e.site(IMPACT.addr(), &[0, 0]), None);
    listener(&mut e);
    e.mem.set_u16(BASE + 0x62, 2);
    e.site(IMPACT.addr(), &[PROJECTILE, 1]);
    assert_eq!(position(&e), [0, 0, 0]);
    assert_eq!(e.ext::<Seen>().args.len(), 1);
    e.mem.set_u16(BASE + 0x62, 0);
    e.mem.set_u32(PROJECTILE + 0x88, 0);
    e.site(IMPACT.addr(), &[PROJECTILE, 1]);
    assert_eq!(
        position(&e),
        [0, 0, 0],
        "null head data leaves position unchanged"
    );
    assert_eq!(e.ext::<Seen>().args[1][3], 0);
}
