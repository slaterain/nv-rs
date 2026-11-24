use super::*;

const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const SCRIPT: u32 = DATA + 0x2000;
const QUEST: u32 = DATA + 0x4000;
const STATE: u32 = DATA + 0x8000;

#[derive(Default)]
struct Echoes(Vec<(Vec<u8>, f64)>);

fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    e.map(DATA, 0x10000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.mem.set_u8(QUEST + 4, QUEST_KIND);
    // This command must use the engine getter, not duplicate its private flag layout.
    e.register(GET_FAILED, |e, a| {
        assert_eq!(a, [QUEST]);
        assert_eq!(e.mem.f64(RESULT), 0.0);
        let calls = e.mem.u32(STATE + 4);
        e.mem.set_u32(STATE + 4, calls + 1);
        Ret {
            eax: e.mem.u32(STATE),
            ..Ret::default()
        }
    });
    e.register(CONSOLE_OUTPUT, |e, a| {
        assert!(a.is_empty());
        let tls = e.tls();
        Ret {
            eax: u32::from(e.mem.u8(tls + 0x268)),
            ..Ret::default()
        }
    });
    e.register(0x0071_b160, |_, a| {
        assert_eq!(a, [1]);
        Ret {
            eax: STATE,
            ..Ret::default()
        }
    });
    e.register(0x0071_d0a0, |e, a| {
        assert_eq!(a[0], STATE);
        let format = table::guest_text(e, a[1]);
        let value = e.mem.f64(a[2]);
        e.ext::<Echoes>().0.push((format, value));
        Ret::default()
    });
    e
}

fn form_arg(e: &mut Engine, form: u32) -> Vec<u8> {
    let entry = e.mem.alloc(0x10);
    e.mem.set_u32(entry + 8, form);
    e.mem.set_u32(SCRIPT + 0x44, entry);
    vec![b'r', 1, 0]
}

/// Real command record, real kParamType_Quest argument extraction; no compiler double.
fn command(e: &mut Engine, args: &[Vec<u8>]) -> f64 {
    let bytes = args.concat();
    e.mem.set_u16(DATA, OPCODE as u16);
    e.mem.set_u16(DATA + 2, (bytes.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    e.mem.write(DATA + 6, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.5);
    let record = table::record_of(e, OPCODE);
    let words = [
        e.mem.u32(record + 0x14),
        DATA,
        0,
        0,
        SCRIPT,
        0,
        RESULT,
        OFFSET,
    ];
    assert!(e.call(e.mem.u32(record + 0x18), &words).bool());
    e.mem.f64(RESULT)
}

fn condition(e: &mut Engine, form: u32) -> f64 {
    e.mem.set_f64(RESULT, 1234.5);
    let record = table::record_of(e, OPCODE);
    // The quest is arg1, independent of the condition's subject and unused arg2.
    assert!(e
        .call(e.mem.u32(record + 0x20), &[0x1234, form, 0x5678, RESULT])
        .bool());
    e.mem.f64(RESULT)
}

#[test]
fn record_has_required_quest_parameter_alias_and_condition_evaluator() {
    let e = engine();
    let record = table::record_of(&e, OPCODE);
    assert_eq!(table::guest_text(&e, e.mem.u32(record)), b"GetQuestFailed");
    assert_eq!(table::guest_text(&e, e.mem.u32(record + 4)), b"GetQF");
    assert_eq!(table::opcode_by_name(&e, b"GetQF"), Some(OPCODE));
    assert_eq!(e.mem.u16(record + 0x10), 0);
    assert_eq!(e.mem.u16(record + 0x12), 1);
    let params = e.mem.u32(record + 0x14);
    assert_eq!(table::guest_text(&e, e.mem.u32(params)), b"quest");
    assert_eq!(e.mem.u32(params + 4), 0x0e);
    assert_eq!(e.mem.u32(params + 8), 0);
    assert_eq!(e.mem.u32(record + 0x18), table::execute_address(OPCODE));
    assert_eq!(e.mem.u32(record + 0x20), table::eval_address(OPCODE));
    assert_eq!(table::return_type(&e, OPCODE), Returns::Default);
    assert!(!crate::plugin().listed);
}

#[test]
fn execute_and_eval_call_the_getter_and_guard_by_quest_kind_not_vtable() {
    let mut e = engine();
    // A synthetic vtable is intentionally irrelevant: JG uses GetFormType, not IS_TYPE.
    e.mem.set_u32(QUEST, 0x1234_5678);
    let quest = form_arg(&mut e, QUEST);
    for (returned, expected) in [(0, 0.0), (1, 1.0), (0x100, 0.0)] {
        e.mem.set_u32(STATE, returned);
        assert_eq!(command(&mut e, std::slice::from_ref(&quest)), expected);
        assert_eq!(condition(&mut e, QUEST), expected);
    }
    assert_eq!(e.mem.u32(STATE + 4), 6);
    assert_eq!(command(&mut e, &[]), 0.0);
    assert_eq!(condition(&mut e, 0), 0.0);
    let null = form_arg(&mut e, 0);
    assert_eq!(command(&mut e, &[null]), 0.0);
    e.mem.set_u8(QUEST + 4, 8); // A faction is refused by both real typed extraction and eval.
    let wrong = form_arg(&mut e, QUEST);
    assert_eq!(command(&mut e, &[wrong]), 0.0);
    assert_eq!(condition(&mut e, QUEST), 0.0);
    assert_eq!(e.mem.u32(STATE + 4), 6);
    assert!(e.ext::<Echoes>().0.is_empty());
}

#[test]
fn execute_echoes_zero_for_failed_reads_using_tls_only_and_eval_never_echoes() {
    let mut e = engine();
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
    e.mem.set_u8(0x011d_ea2e, 0); // Console-open is deliberately false.
    assert_eq!(command(&mut e, &[]), 0.0);
    e.mem.set_u8(QUEST + 4, 8);
    let wrong = form_arg(&mut e, QUEST);
    assert_eq!(command(&mut e, &[wrong]), 0.0);
    e.mem.set_u8(QUEST + 4, QUEST_KIND);
    e.mem.set_u32(STATE, 1);
    let quest = form_arg(&mut e, QUEST);
    assert_eq!(command(&mut e, &[quest]), 1.0);
    assert_eq!(
        std::mem::take(&mut e.ext::<Echoes>().0),
        vec![
            (b"GetQuestFailed >> %.2f".to_vec(), 0.0),
            (b"GetQuestFailed >> %.2f".to_vec(), 0.0),
            (b"GetQuestFailed >> %.2f".to_vec(), 1.0),
        ]
    );
    assert_eq!(condition(&mut e, QUEST), 1.0);
    assert_eq!(condition(&mut e, 0), 0.0);
    assert!(e.ext::<Echoes>().0.is_empty());
    e.mem.set_u8(tls + 0x268, 0);
    e.mem.set_u8(0x011d_ea2e, 1); // Open alone must not print.
    assert_eq!(command(&mut e, &[]), 0.0);
    assert!(e.ext::<Echoes>().0.is_empty());
}
