use super::*;
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const SCRIPT: u32 = DATA + 0x2000;
const REF: u32 = SCRIPT + 0x100;
const THIS: u32 = DATA + 0x3000;
const NPC: u32 = DATA + 0x4000;
const CREATURE: u32 = DATA + 0x5000;
const TEMPLATE: u32 = DATA + 0x6000;
const VT: u32 = DATA + 0x7000;
const PREDICATE: u32 = DATA + 0x8000;
#[derive(Default)]
struct Calls(Vec<u32>);
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(DATA, 0x10000);
    e.map(0x0118_e000, 0x9000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.mem.set_u32(SCRIPT + 0x44, REF);
    e.mem.set_u32(THIS + 0x20, NPC);
    for (base, kind) in [(NPC, 0x2a), (CREATURE, 0x2b), (TEMPLATE, 0x2a)] {
        e.mem.set_u32(base, VT);
        e.mem.set_u8(base + 4, kind);
    }
    e.mem.set_u32(VT + IS_ACTOR_BASE, PREDICATE);
    e.register(PREDICATE, |e, a| {
        e.ext::<Calls>().0.push(a[0]);
        u32::from(e.mem.u8(a[0] + 0x80) != 0).into_ret()
    });
    e.mem.set_u8(NPC + 0x80, 1);
    e.mem.set_u8(CREATURE + 0x80, 1);
    e.mem.set_u8(TEMPLATE + 0x80, 1);
    e
}
fn execute_call(e: &mut Engine, this: u32, bit: u32, given: Option<u32>) -> f64 {
    let mut bytes = vec![b'n'];
    bytes.extend_from_slice(&bit.to_le_bytes());
    if let Some(given) = given {
        e.mem.set_u32(REF + 8, given);
        bytes.extend([b'r', 1, 0]);
    }
    e.mem.set_u16(DATA, OPCODE as u16);
    e.mem.set_u16(DATA + 2, (bytes.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, if given.is_some() { 2 } else { 1 });
    e.mem.write(DATA + 6, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, -1.0);
    let rec = table::record_of(e, OPCODE);
    let words = [
        e.mem.u32(rec + 0x14),
        DATA,
        this,
        0,
        SCRIPT,
        0,
        RESULT,
        OFFSET,
    ];
    assert!(e.call(e.mem.u32(rec + 0x18), &words).bool());
    e.mem.f64(RESULT)
}
fn eval_call(e: &mut Engine, this: u32, bit: u32, form: u32) -> f64 {
    e.mem.set_f64(RESULT, -1.0);
    let rec = table::record_of(e, OPCODE);
    assert!(e
        .call(e.mem.u32(rec + 0x20), &[this, bit, form, RESULT])
        .bool());
    e.mem.f64(RESULT)
}
#[test]
fn real_record_integer_optional_actor_base_and_eval_are_exact() {
    let e = engine();
    assert_eq!(table::opcode_by_name(&e, b"ActorHasBaseFlag"), Some(OPCODE));
    assert_eq!(
        table::opcode_by_name(&e, b"GetActorHasBaseFlag"),
        Some(OPCODE)
    );
    let rec = table::record_of(&e, OPCODE);
    assert_eq!(e.mem.u16(rec + 0x10), 0);
    assert_eq!(e.mem.u16(rec + 0x12), 2);
    assert_eq!(table::return_type(&e, OPCODE), Returns::Default);
    let p = e.mem.u32(rec + 0x14);
    assert_eq!((e.mem.u32(p + 4), e.mem.u32(p + 8)), (1, 0));
    assert_eq!((e.mem.u32(p + 16), e.mem.u32(p + 20)), (0x19, 1));
    assert_ne!(e.mem.u32(rec + 0x20), 0);
}
#[test]
fn selected_stored_base_flags_do_not_follow_template_or_extra_leveled_actor_data() {
    let mut e = engine();
    e.mem.set_u32(NPC + 0x34, 1 << 28);
    e.mem.set_u32(CREATURE + 0x34, 0);
    e.mem.set_u32(THIS + 0x44, 0xdead_beef);
    assert_eq!(execute_call(&mut e, THIS, 28, None), 1.0);
    assert_eq!(execute_call(&mut e, THIS, 28, Some(CREATURE)), 0.0);
    assert_eq!(eval_call(&mut e, THIS, 28, CREATURE), 0.0);
    e.mem.set_u32(NPC + 0x34, 0);
    e.mem.set_u16(NPC + 0x4a, u16::MAX);
    e.mem.set_u32(NPC + 0x54, TEMPLATE);
    e.mem.set_u32(TEMPLATE + 0x34, 1 << 28);
    assert_eq!(execute_call(&mut e, THIS, 28, None), 0.0);
    assert_eq!(e.ext::<Calls>().0, [NPC, CREATURE, CREATURE, NPC]);
}
#[test]
fn eval_does_not_normalize_given_reference_and_standard_extractor_rejects_it() {
    let mut e = engine();
    e.mem.set_u32(THIS, VT);
    e.mem.set_u8(THIS + 4, 0x3b);
    e.mem.set_u8(THIS + 0x80, 0);
    e.mem.set_u32(NPC + 0x34, 1 << 28);
    assert_eq!(eval_call(&mut e, THIS, 28, THIS), 0.0);
    assert_eq!(e.ext::<Calls>().0, [THIS]);
    e.ext::<Calls>().0.clear();
    assert_eq!(execute_call(&mut e, THIS, 28, Some(THIS)), 0.0);
    assert!(e.ext::<Calls>().0.is_empty());
    // Eval's virtual predicate, rather than a guessed NPC/CREA type check.
    e.mem.set_u8(CREATURE + 4, 8);
    e.mem.set_u32(CREATURE + 0x34, 1 << 28);
    assert_eq!(eval_call(&mut e, THIS, 28, CREATURE), 1.0);
    assert_eq!(execute_call(&mut e, THIS, 28, Some(CREATURE)), 0.0);
}
#[test]
fn all_bits_and_out_of_range_guards_preserve_boolean_results() {
    let mut e = engine();
    e.mem.set_u32(NPC + 0x34, u32::MAX);
    for bit in 0..32 {
        assert_eq!(execute_call(&mut e, THIS, bit, None), 1.0);
    }
    e.ext::<Calls>().0.clear();
    for bit in [32, 100, u32::MAX] {
        assert_eq!(eval_call(&mut e, 0xffff_fff0, bit, 0), 0.0);
        assert_eq!(execute_call(&mut e, THIS, bit, None), 0.0);
    }
    assert!(e.ext::<Calls>().0.is_empty());
    assert_eq!(eval_call(&mut e, 0, 0, 0), 0.0);
}
