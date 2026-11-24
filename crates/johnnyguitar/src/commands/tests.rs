use super::*;
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    e.map(DATA, 0x10000);
    nvse::table::install(&mut e);
    install(&mut e);
    e
}
fn num(v: f64) -> Vec<u8> {
    [vec![b'z'], v.to_le_bytes().to_vec()].concat()
}
fn string(v: &str) -> Vec<u8> {
    [
        (v.len() as u16).to_le_bytes().to_vec(),
        v.as_bytes().to_vec(),
    ]
    .concat()
}
fn call(e: &mut Engine, op: u32, subject: u32, args: &[Vec<u8>]) -> f64 {
    let bytes = args.concat();
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, (bytes.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    e.mem.write(DATA + 6, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.0);
    let record = table::record_of(e, op);
    let words = [
        e.mem.u32(record + 0x14),
        DATA,
        subject,
        0,
        0,
        0,
        RESULT,
        OFFSET,
    ];
    assert_eq!(e.call(e.mem.u32(record + 0x18), &words).eax & 0xff, 1);
    e.mem.f64(RESULT)
}
#[test]
fn clamp_uses_three_floats_and_original_branch_order() {
    let mut e = engine();
    for (v, lo, hi, want) in [
        (10., 25., 50., 25.),
        (30., 25., 50., 30.),
        (70., 25., 50., 50.),
        (30., 50., 25., 50.),
    ] {
        assert_eq!(call(&mut e, 0x3155, 0, &[num(v), num(lo), num(hi)]), want);
    }
    assert!(call(&mut e, 0x3155, 0, &[num(f64::NAN), num(1.), num(2.)]).is_nan());
    assert_eq!(
        call(
            &mut e,
            0x3155,
            0,
            &[num(16_777_217.), num(0.), num(20_000_000.)]
        ),
        16_777_216.
    );
    assert_eq!(call(&mut e, 0x3155, 0, &[]), 0.);
}
#[test]
fn editor_id_command_returns_form_bits_and_uses_game_lookup() {
    let mut e = engine();
    let form = e.mem.alloc(0x20);
    e.mem.set_u32(form + 0xc, 0x06001234);
    e.mem.set_u32(DATA + 0x8000, form);
    e.register(0x0048_3a00, |e, a| Ret {
        eax: if table::guest_text(e, a[0]) == b"AguaFriaQUEST" {
            e.mem.u32(DATA + 0x8000)
        } else {
            0
        },
        ..Ret::default()
    });
    call(&mut e, 0x3128, 0, &[string("AguaFriaQUEST")]);
    assert_eq!(e.mem.u32(RESULT), 0x06001234);
    call(&mut e, 0x3128, 0, &[string("missing")]);
    assert_eq!(e.mem.u64(RESULT), 0);
    call(&mut e, 0x3128, 0, &[string("")]);
    assert_eq!(e.mem.u64(RESULT), 0);
    assert_eq!(table::return_type(&e, 0x3128), Returns::Form);
}
#[test]
fn theft_command_rejects_non_actor_subject() {
    let mut e = engine();
    assert_eq!(call(&mut e, 0x3199, 0, &[]), 0.);
    let subject = e.mem.alloc(0x80);
    let vt = e.mem.alloc(0x110);
    e.mem.set_u32(subject, vt);
    e.mem.set_u32(vt + 0x100, 0x03010000);
    e.register(0x03010000, |_, _| Ret::default());
    assert_eq!(call(&mut e, 0x3199, subject, &[]), 0.);
}
#[test]
fn theft_raises_alarm_with_original_ownership_and_checks_inventory_stacks() {
    let mut e = engine();
    const SCRIPT: u32 = DATA + 0x2000;
    const TARGET: u32 = DATA + 0x4000;
    const ACTOR: u32 = DATA + 0x5000;
    const OWNER: u32 = DATA + 0x6000;
    const STATE: u32 = DATA + 0x7000;
    let vt = e.mem.alloc(0x110);
    e.mem.set_u32(ACTOR, vt);
    e.mem.set_u32(vt + 0x100, 0x03010000);
    e.register(0x03010000, |_, _| Ret {
        eax: 1,
        ..Ret::default()
    });
    e.mem.set_u8(TARGET + 4, 0x3a);
    let refvar = e.mem.alloc(0x10);
    e.mem.set_u32(refvar + 8, TARGET);
    e.mem.set_u32(SCRIPT + 0x44, refvar);
    e.register(0x00567790, |_, a| {
        assert_eq!(a, [TARGET]);
        Ret {
            eax: OWNER,
            ..Ret::default()
        }
    });
    e.register(0x008bfa40, |e, a| {
        assert_eq!(a, [ACTOR, TARGET, 0, 0, 1, OWNER]);
        e.mem.set_u32(STATE, 1);
        Ret::default()
    });
    e.register(0x00418520, |e, a| {
        assert_eq!(a, [ACTOR + 0x44]);
        Ret {
            eax: e.mem.u32(STATE + 4),
            ..Ret::default()
        }
    });
    e.register(0x00418660, |_, _| Ret {
        eax: OWNER,
        ..Ret::default()
    });
    let run = |e: &mut Engine, check: bool| {
        let refarg = vec![b'r', 1, 0];
        let encoded = [refarg, num(f64::from(check))].concat();
        e.mem.set_u16(DATA, 0x3199);
        e.mem.set_u16(DATA + 2, (encoded.len() + 2) as u16);
        e.mem.set_u16(DATA + 4, 2);
        e.mem.write(DATA + 6, &encoded);
        e.mem.set_u32(OFFSET, 4);
        let rec = table::record_of(e, 0x3199);
        let words = [
            e.mem.u32(rec + 0x14),
            DATA,
            ACTOR,
            0,
            SCRIPT,
            0,
            RESULT,
            OFFSET,
        ];
        assert_eq!(e.call(e.mem.u32(rec + 0x18), &words).eax & 0xff, 1);
        e.mem.f64(RESULT)
    };
    assert_eq!(run(&mut e, false), 1.);
    assert_eq!(e.mem.u32(STATE), 1);
    e.mem.set_u32(STATE, 0);
    assert_eq!(run(&mut e, true), 0.);
    assert_eq!(e.mem.u32(STATE), 0);
    let changes = e.mem.alloc(0x20);
    let node = e.mem.alloc(8);
    let entry = e.mem.alloc(12);
    let extra = e.mem.alloc(8);
    e.mem.set_u32(STATE + 4, changes);
    e.mem.set_u32(changes, node);
    e.mem.set_u32(node, entry);
    e.mem.set_u32(entry, extra);
    e.mem.set_u32(entry + 8, TARGET);
    e.mem.set_u32(extra, TARGET + 0x44);
    assert_eq!(run(&mut e, true), 1.);
    assert_eq!(e.mem.u32(STATE), 1);
}
