use super::*;
const AREA: u32 = 0x0300_0000;
const ACTOR: u32 = AREA + 0x1000;
const DATA: u32 = AREA + 0x3000;
const RESULT: u32 = DATA + 0x800;
const SCRIPT: u32 = AREA + 0x4000;
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.mem.set_u32(PLAYER, ACTOR);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0100);
    e.register(0x005e_23f0, |_, _| false.into_ret());
    e
}
fn int(v: i32) -> Vec<u8> {
    [vec![b'n'], v.to_le_bytes().to_vec()].concat()
}
fn call(e: &mut Engine, op: u32, target: u32, args: &[Vec<u8>]) -> f64 {
    let data = args.concat();
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, (data.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    e.mem.write(DATA + 6, &data);
    e.mem.set_u32(RESULT + 8, 4);
    e.mem.set_f64(RESULT, 998.25);
    let r = table::record_of(e, op);
    let words = [
        e.mem.u32(r + 0x14),
        DATA,
        target,
        0,
        SCRIPT,
        0,
        RESULT,
        RESULT + 8,
    ];
    e.call(e.mem.u32(r + 0x18), &words);
    e.mem.f64(RESULT)
}
#[test]
fn actual_editor_id_record_returns_real_string_and_uses_form_virtual_not_placeholder() {
    let mut e = setup();
    let form = e.mem.alloc(0x80);
    let vt = e.mem.alloc(0x140);
    e.mem.set_u32(form, vt);
    e.mem.set_u32(vt + 0x130, AREA + 0x8000);
    let text = table::guest_str(&mut e, b"WeapHuntingRifle");
    e.register(AREA + 0x8000, |e, _| e.mem.u32(DATA + 0x100).into_ret());
    e.mem.set_u32(DATA + 0x100, text);
    let id = call(&mut e, 0x3114, form, &[]) as u32;
    assert_eq!(nvse::vars::string(&e, id).unwrap(), b"WeapHuntingRifle");
    let id = call(&mut e, 0x3114, 0, &[]) as u32;
    assert!(nvse::vars::string(&e, id).unwrap().is_empty());
    let r = table::record_of(&e, 0x3114);
    assert_eq!(table::return_type(&e, 0x3114), Returns::String);
    assert_eq!(table::guest_text(&e, e.mem.u32(r + 4)), b"GetEdID");
}
#[test]
fn editor_id_explicit_form_overrides_calling_reference_and_bad_extraction_leaves_result() {
    let mut e = setup();
    let form = e.mem.alloc(0x80);
    let vt = e.mem.alloc(0x140);
    e.mem.set_u32(form, vt);
    e.mem.set_u32(vt + 0x130, AREA + 0x8000);
    e.register(AREA + 0x8000, |e, a| {
        assert_eq!(a, [e.mem.u32(DATA + 0x100)]);
        e.mem.u32(DATA + 0x104).into_ret()
    });
    let text = table::guest_str(&mut e, b"EM_Main");
    e.mem.set_u32(DATA + 0x100, form);
    e.mem.set_u32(DATA + 0x104, text);
    let refs = e.mem.alloc(16);
    e.mem.set_u32(refs + 8, form);
    e.mem.set_u32(SCRIPT + 0x44, refs);
    let id = call(&mut e, 0x3114, ACTOR, &[vec![b'r', 1, 0]]) as u32;
    assert_eq!(nvse::vars::string(&e, id).unwrap(), b"EM_Main");
    assert_eq!(call(&mut e, 0x3114, ACTOR, &[vec![b'x']]), 998.25);
}
#[test]
fn always_run_preserves_other_mover_bits_calls_actual_virtual_and_negative_or_missing_does_nothing()
{
    let mut e = setup();
    let mover = e.mem.alloc(0xa0);
    let vt = e.mem.alloc(0x20);
    e.mem.set_u32(ACTOR + 0x190, mover);
    e.mem.set_u32(mover, vt);
    e.mem.set_u32(vt + 0xc, AREA + 0x8000);
    e.mem.set_u32(mover + 0x94, 0xa5a5_5a5a);
    e.register(AREA + 0x8000, |e, a| {
        e.mem.set_u32(DATA + 0x100, a[1]);
        e.mem.set_u32(DATA + 0x104, e.mem.u32(DATA + 0x104) + 1);
        Ret::default()
    });
    assert_eq!(call(&mut e, 0x320b, 0, &[]), 0.0);
    assert_eq!(call(&mut e, 0x320b, 0, &[int(-1), int(1)]), 0.0);
    assert_eq!(call(&mut e, 0x320b, 0, &[int(1)]), 1.0);
    assert_eq!(e.mem.u8(ACTOR + 0x651), 1);
    assert_eq!(e.mem.u32(DATA + 0x104), 0);
    assert_eq!(call(&mut e, 0x320b, 0, &[int(0), int(-1)]), 1.0);
    assert_eq!(e.mem.u8(ACTOR + 0x651), 0);
    assert_eq!(e.mem.u32(DATA + 0x100), 0xa5a5_585a);
    assert_eq!(
        e.mem.u32(mover + 0x94),
        0xa5a5_5a5a,
        "only ForceMoveMode owns mover writes"
    );
}
#[test]
fn compass_hostility_scans_later_duplicate_after_nonhostile_and_ignores_los_flag() {
    let mut e = setup();
    let head = e.mem.alloc(8);
    e.mem.set_u32(ACTOR + 0xd48, head);
    let mut node = head;
    for (i, (actor, hostile)) in [(0x1234, 0), (0x7777, 1), (0x1234, 1)]
        .into_iter()
        .enumerate()
    {
        let item = e.mem.alloc(8);
        e.mem.set_u32(item, actor);
        e.mem.set_u8(item + 4, hostile);
        e.mem.set_u8(item + 5, 0);
        e.mem.set_u32(node, item);
        let next = if i == 2 { 0 } else { e.mem.alloc(8) };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
    assert_eq!(call(&mut e, 0x3187, 0x1234, &[]), 1.0);
    assert_eq!(call(&mut e, 0x3187, 0x8888, &[]), 0.0);
    compass_eval(&mut e, &[0x7777, 0, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 1.0);
}
