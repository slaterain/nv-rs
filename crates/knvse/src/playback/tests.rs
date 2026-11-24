use super::*;
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const CALL: u32 = DATA + 0x9000;
const CALLS: u32 = CALL + 0x40;
const GET_PROCESS_ANIMATION: u32 = DATA + 0x9500;
fn string(s: &str) -> Vec<u8> {
    [(s.len() as u16).to_le_bytes().as_slice(), s.as_bytes()].concat()
}
fn integer(i: i32) -> Vec<u8> {
    [&b"n"[..], &i.to_le_bytes()].concat()
}
fn float(v: f64) -> Vec<u8> {
    [&b"z"[..], &v.to_le_bytes()].concat()
}
fn call(e: &mut Engine, op: u32, subject: u32, arguments: &[Vec<u8>]) -> f64 {
    let payload = arguments.concat();
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, (payload.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, arguments.len() as u16);
    e.mem.write(DATA + 6, &payload);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 999.0);
    let r = table::record_of(e, op);
    let w = [e.mem.u32(r + 0x14), DATA, subject, 0, 0, 0, RESULT, OFFSET];
    assert_eq!(e.call(e.mem.u32(r + 0x18), &w).eax & 0xff, 1);
    e.mem.f64(RESULT)
}
fn fixture() -> (Engine, u32, u32, u32) {
    let mut e = crate::loader::tests::fixture();
    let actor = e.mem.alloc(0x800);
    e.mem.set_u32(actor, DATA + 0x3000);
    e.mem.set_u32(PLAYER, actor);
    let process = e.mem.alloc(0x200);
    let vt = e.mem.alloc(0x200);
    e.mem.set_u32(process, vt);
    e.mem.set_u32(vt + 0x1b8, GET_PROCESS_ANIMATION);
    e.mem.set_u32(actor + 0x68, process);
    let (first, _) = crate::loader::tests::animation(&mut e);
    let (third, _) = crate::loader::tests::animation(&mut e);
    e.mem.set_u32(actor + 0x690, first);
    e.mem.set_u32(process + 0x1c0, third);
    e.register(GET_PROCESS_ANIMATION, |e, w| Ret {
        eax: e.mem.u32(w[0] + 0x1c0),
        ..Ret::default()
    });
    e.register(0x0047_aab0, |e, w| {
        e.mem.set_u32(CALLS, e.mem.u32(CALLS) + 1);
        for (i, &v) in w.iter().enumerate() {
            e.mem.set_u32(CALL + i as u32 * 4, v);
        }
        if e.mem.u32(w[1] + 0x44) != 0 {
            return Ret::default();
        }
        e.mem.set_u32(w[1] + 0x44, 1);
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    e.register(0x0047_b220, |e, w| {
        e.mem.set_u32(CALLS, e.mem.u32(CALLS) + 1);
        for (i, &v) in w.iter().enumerate() {
            e.mem.set_u32(CALL + i as u32 * 4, v);
        }
        e.mem.set_u32(w[1] + 0x44, 0);
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    (e, actor, first, third)
}
#[test]
fn metadata_is_the_pinned_initial_twenty_four_contract_not_forty() {
    let (e, _, _, _) = fixture();
    let r = table::record_of(&e, 0x3933);
    assert_eq!(e.mem.u16(r + 0x10), 1);
    assert_eq!(e.mem.u16(r + 0x12), 7);
    let p = e.mem.u32(r + 0x14);
    let want = [(0, 0), (1, 0), (1, 1), (1, 1), (2, 1), (2, 1), (1, 1)];
    for (i, (kind, optional)) in want.into_iter().enumerate() {
        assert_eq!(e.mem.u32(p + i as u32 * 12 + 4), kind);
        assert_eq!(e.mem.u32(p + i as u32 * 12 + 8), optional);
    }
    assert!(nvse::commands::extender::plugin_info(&e, b"kNVSE").is_none());
}
#[test]
fn activate_calls_actual_manager_synchronously_with_original_defaults_and_no_restart() {
    let (mut e, actor, first, _) = fixture();
    assert_eq!(
        call(&mut e, 0x3933, actor, &[string("EM_NoBob.kf"), integer(1)]),
        1.0
    );
    let loaded = crate::loader::cached(&e, first, b"EM_NoBob.kf").unwrap();
    assert_eq!(e.mem.u32(CALL), e.mem.u32(first + 0xd8));
    assert_eq!(e.mem.u32(CALL + 4), loaded.sequence);
    assert_eq!(e.mem.u32(CALL + 8), 0);
    assert_eq!(e.mem.u32(CALL + 12), 1);
    assert_eq!(e.mem.f32(CALL + 16), 0.75);
    assert_eq!(e.mem.u32(CALL + 20), 0);
    assert_eq!(e.mem.u32(CALL + 24), 0);
    assert_eq!(e.mem.u32(CALLS), 1);
    assert_eq!(
        call(&mut e, 0x3933, actor, &[string("EM_NoBob.kf"), integer(1)]),
        0.0
    );
    assert_eq!(e.mem.u32(CALLS), 2);
    assert_eq!(e.mem.u32(loaded.sequence + 0x44), 1);
}
#[test]
fn explicit_arguments_pov_default_and_failed_subject_use_actual_extraction() {
    let (mut e, actor, first, third) = fixture();
    e.mem.set_u8(actor + 0x64c, 1);
    assert_eq!(
        call(
            &mut e,
            0x3933,
            actor,
            &[
                string("EM_NoBob.kf"),
                integer(-1),
                integer(-4),
                integer(0),
                float(0.25),
                float(0.125)
            ]
        ),
        1.0
    );
    assert!(crate::loader::cached(&e, first, b"EM_NoBob.kf").is_none());
    assert!(crate::loader::cached(&e, third, b"EM_NoBob.kf").is_some());
    assert_eq!(e.mem.u32(CALL + 8), (-4i32) as u32);
    assert_eq!(e.mem.u32(CALL + 12), 0);
    assert_eq!(e.mem.f32(CALL + 16), 0.25);
    assert_eq!(e.mem.f32(CALL + 20), 0.125);
    assert_eq!(
        call(&mut e, 0x3933, 0, &[string("EM_NoBob.kf"), integer(1)]),
        0.0
    );
    e.mem.set_u32(actor, 0);
    assert_eq!(
        call(&mut e, 0x3933, actor, &[string("EM_NoBob.kf"), integer(1)]),
        0.0
    );
    assert_eq!(e.mem.u32(CALLS), 1);
}
#[test]
fn deactivate_searches_third_before_first_even_when_first_person_is_visible() {
    let (mut e, actor, first, third) = fixture();
    assert_eq!(
        call(&mut e, 0x3933, actor, &[string("EM_NoBob.kf"), integer(1)]),
        1.0
    );
    assert_eq!(
        call(&mut e, 0x3933, actor, &[string("EM_NoBob.kf"), integer(0)]),
        1.0
    );
    let one = crate::loader::cached(&e, first, b"EM_NoBob.kf").unwrap();
    let three = crate::loader::cached(&e, third, b"EM_NoBob.kf").unwrap();
    assert_eq!(call(&mut e, 0x3934, actor, &[string("EM_NoBob.kf")]), 1.0);
    assert_eq!(e.mem.u32(CALL + 4), three.sequence);
    assert_eq!(e.mem.u32(CALL + 8), 0);
    assert_eq!(e.mem.u32(one.sequence + 0x44), 1);
    assert_eq!(e.mem.u32(three.sequence + 0x44), 0);
    // In24 the cache lookup does not require an active state, despite the
    // helper's name; another call still finds the inactive third-person one.
    assert_eq!(
        call(&mut e, 0x3934, actor, &[string("EM_NoBob.kf"), float(0.75)]),
        1.0
    );
    assert_eq!(e.mem.u32(CALL + 4), three.sequence);
    assert_eq!(e.mem.f32(CALL + 8), 0.75);
}
#[test]
fn deactivate_falls_back_to_first_person_when_third_has_no_matching_sequence() {
    let (mut e, actor, first, _) = fixture();
    assert_eq!(
        call(&mut e, 0x3933, actor, &[string("EM_NoBob.kf"), integer(1)]),
        1.0
    );
    let one = crate::loader::cached(&e, first, b"EM_NoBob.kf").unwrap();
    assert_eq!(call(&mut e, 0x3934, actor, &[string("EM_NoBob.kf")]), 1.0);
    assert_eq!(e.mem.u32(CALL + 4), one.sequence);
}
