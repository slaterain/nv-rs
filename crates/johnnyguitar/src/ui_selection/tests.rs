use super::*;
const AREA: u32 = 0x0300_0000;
const SCRIPT: u32 = AREA + 0x1000;
const PARENT: u32 = AREA + 0x1200;
const MANAGER: u32 = AREA + 0x2000;
const MENU: u32 = AREA + 0x3000;
const DATA: u32 = AREA + 0x4000;
const RESULT: u32 = DATA + 0x800;
#[derive(Default)]
struct Seen {
    native: Vec<u32>,
    calls: Vec<f64>,
    front: u32,
    remove: bool,
}
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0103_7000, 0x1000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011c_3000, 0x1000);
    nvse::table::install(&mut e);
    nvse::functions::install(&mut e);
    install(&mut e);
    e.mem
        .set_u32(nvse::functions::FUNCTION_BLOCK_RECORD + 8, 0xd);
    let data = e.mem.alloc(0x40);
    e.mem.write(
        data,
        &[
            0x1d, 0, 0, 0, 0x10, 0, 11, 0, 0xd, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1,
        ],
    );
    e.mem.set_u32(SCRIPT, 0x0103_7094);
    e.mem.set_u8(SCRIPT + 4, 0x11);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0100);
    e.mem.set_u32(SCRIPT + 0x20, 19);
    e.mem.set_u32(SCRIPT + 0x30, data);
    e.mem.set_u32(PARENT + 0xc, 0x0500_0120);
    let vt = e.mem.alloc(0x40);
    e.mem.set_u32(MENU, vt);
    e.mem.set_u32(vt + 0x34, AREA + 0x9000);
    e.mem.set_u32(MENU + 0x20, 1002);
    e.ext::<Seen>().front = MENU;
    e.register(0x0071_e290, |e, a| {
        assert_eq!(a, [1]);
        e.ext::<Seen>().native.push(0x0071_e290);
        0x1234u32.into_ret()
    });
    e.register(0x0072_0e60, |e, a| {
        assert_eq!(a, [0x1234]);
        e.ext::<Seen>().native.push(0x0072_0e60);
        e.ext::<Seen>().front.into_ret()
    });
    e.register(AREA + 0x9000, |e, a| {
        assert_eq!(a, [MENU]);
        e.ext::<Seen>().native.push(0x34);
        e.mem.u32(MENU + 0x20).into_ret()
    });
    e.register(nvse::functions::CREATE_EVENT_LIST, |e, a| {
        let list = e.mem.alloc(0x14);
        e.mem.set_u32(list, a[0]);
        let local = e.mem.alloc(0x10);
        e.mem.set_u32(local, 1);
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, local);
        e.mem.set_u32(list + 0xc, node);
        list.into_ret()
    });
    e.register(nvse::functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, a| {
        assert_eq!(a[1], 0);
        assert_eq!(a[3..], [0, 0]);
        let local = nvse::args::event_list_variable(e, a[2], 1);
        let value = e.mem.f64(local + 8);
        e.ext::<Seen>().calls.push(value);
        if e.ext::<Seen>().remove {
            e.ext::<Seen>().remove = false;
            change(e, SCRIPT, 0, false);
            tick(e);
        }
        Ret::default()
    });
    e
}
fn call(e: &mut Engine, toggle: i32, filter: i32) {
    let refs = e.mem.alloc(16);
    e.mem.set_u32(refs + 8, SCRIPT);
    e.mem.set_u32(PARENT + 0x44, refs);
    let mut args = vec![b'n'];
    args.extend(toggle.to_le_bytes());
    args.extend([b'r', 1, 0, b'n']);
    args.extend(123i32.to_le_bytes());
    args.push(b'n');
    args.extend(filter.to_le_bytes());
    e.mem.set_u16(DATA, OP as u16);
    e.mem.set_u16(DATA + 2, (args.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, 4);
    e.mem.write(DATA + 6, &args);
    e.mem.set_u32(RESULT + 8, 4);
    e.mem.set_f64(RESULT, 56.75);
    let r = table::record_of(e, OP);
    let p = e.mem.u32(r + 0x14);
    command(e, &[p, DATA, 0, 0, PARENT, 0, RESULT, RESULT + 8]);
    assert_eq!(e.mem.f64(RESULT), 56.75);
}
#[test]
fn actual_command_metadata_unsigned_toggle_flags_ignored_and_queued_dedup() {
    let mut e = setup();
    let r = table::record_of(&e, OP);
    assert_eq!(
        table::guest_text(&e, e.mem.u32(r + 4)),
        b"SetOnKBCTrlUIDeltaHandler"
    );
    assert_eq!(e.mem.u16(r + 0x12), 4);
    call(&mut e, -1, 1002);
    call(&mut e, 1, 1002);
    assert_eq!(site(&mut e, &[MANAGER, 1, 0]), None);
    tick(&mut e);
    assert_eq!(
        e.ext_get::<Selection>()
            .unwrap()
            .lists
            .lock()
            .unwrap()
            .active
            .len(),
        1
    );
    assert_eq!(site(&mut e, &[MANAGER, 1, 0]), Some(MENU));
    site(&mut e, &[MANAGER, MENU, 1]);
    assert_eq!(e.ext::<Seen>().calls, [0.0]);
    call(&mut e, 0, 1002);
    dispatch(&mut e, MENU);
    assert_eq!(e.ext::<Seen>().calls.len(), 2);
    tick(&mut e);
    assert_eq!(site(&mut e, &[MANAGER, 2, 0]), None);
}
#[test]
fn pre_captures_menu_before_original_and_after_reads_its_id_after_original_for_both_sites() {
    let mut e = setup();
    call(&mut e, 1, 1003);
    tick(&mut e);
    for address in [0x0071_5cd5, 0x0071_8059] {
        e.mem.set_u32(MANAGER + 0xbc, 1);
        let captured = e.site(address, &[MANAGER, 2, 0]).unwrap();
        assert_eq!(captured, MENU);
        assert_eq!(e.ext::<Seen>().native.last(), Some(&0x0072_0e60));
        e.ext::<Seen>().front = 0;
        e.mem.set_u32(MENU + 0x20, 1003);
        e.site(address, &[MANAGER, captured, 1]);
        assert_eq!(e.ext::<Seen>().native.last(), Some(&0x34));
        e.ext::<Seen>().front = MENU;
    }
    assert_eq!(e.ext::<Seen>().calls.len(), 2);
    e.mem.set_u32(MANAGER + 0xbc, 2);
    assert_eq!(site(&mut e, &[MANAGER, 2, 0]), None);
}
#[test]
fn zero_filter_matches_null_menu_minus_one_matches_nothing_and_nested_update_keeps_snapshot() {
    let mut e = setup();
    for filter in [0, -1, 1002] {
        call(&mut e, 1, filter);
    }
    tick(&mut e);
    dispatch(&mut e, 0);
    assert_eq!(e.ext::<Seen>().calls.len(), 1);
    e.ext::<Seen>().calls.clear();
    e.ext::<Seen>().remove = true;
    dispatch(&mut e, MENU);
    assert_eq!(e.ext::<Seen>().calls.len(), 2);
    assert_eq!(
        e.ext_get::<Selection>()
            .unwrap()
            .lists
            .lock()
            .unwrap()
            .dispatches,
        0
    );
    assert_eq!(
        e.ext_get::<Selection>()
            .unwrap()
            .lists
            .lock()
            .unwrap()
            .active
            .len(),
        2
    );
}
