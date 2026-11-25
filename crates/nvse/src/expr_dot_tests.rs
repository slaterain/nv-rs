use super::*;
use crate::table::{add_plugin_record, PluginRecord};

const DATA: u32 = 0x0200_0000;
const SCRIPT: u32 = DATA + 0x1000;
const QUEST: u32 = DATA + 0x2000;
const QUEST_LOCALS: u32 = DATA + 0x2100;
const CONTAINER: u32 = DATA + 0x3000;
const STACK: u32 = DATA + 0x3100;
const BASE: u32 = DATA + 0x3200;
const REF_VTABLE: u32 = DATA + 0x4000;
const BASE_VTABLE: u32 = DATA + 0x4100;
const RESULT_OFFSET: u32 = DATA + 0x5000;
const CONTAINER_ID: u32 = 0x0010_3e49;
const STACK_ID: u32 = 0xff00_0800;
const BASE_ID: u32 = 0x0001_d90d;
const STACK_COMMAND: u16 = 0x2200;
const ARRAY_COMMAND: u16 = 0x2201;

fn fixture() -> (Engine, crate::CommandArgs) {
    let mut e = crate::table::tests::engine_with_exe_tables();
    crate::table::install(&mut e);
    crate::table::grow_to(&mut e, u32::from(ARRAY_COMMAND) + 1);
    e.map(DATA, 0x1_0000);
    for (form, id, vtable) in [
        (CONTAINER, CONTAINER_ID, REF_VTABLE),
        (STACK, STACK_ID, REF_VTABLE),
        (BASE, BASE_ID, BASE_VTABLE),
    ] {
        e.mem.set_u32(form, vtable);
        e.mem.set_u32(form + 0xc, id);
    }
    e.mem.set_u32(REF_VTABLE + VT_GET_IS_REFERENCE, 0x008d_0360);
    e.mem
        .set_u32(BASE_VTABLE + VT_GET_IS_REFERENCE, 0x0043_a7b0);
    e.register_double(0x008d_0360, |_, words| {
        assert!(matches!(words, [CONTAINER | STACK]));
        crate::handled(true)
    });
    e.register_double(0x0043_a7b0, |_, words| {
        assert_eq!(words, [BASE]);
        crate::handled(false)
    });
    e.register_double(LOOKUP_FORM, |_, words| Ret {
        eax: match words[0] {
            CONTAINER_ID => CONTAINER,
            STACK_ID => STACK,
            BASE_ID => BASE,
            _ => 0,
        },
        ..Ret::default()
    });

    let quest_ref = DATA + 0x2200;
    let node = DATA + 0x2300;
    let quest_var = DATA + 0x2310;
    e.mem.set_u32(SCRIPT + 0x44, quest_ref);
    e.mem.set_u32(quest_ref + 8, QUEST);
    e.mem.set_u8(QUEST + 4, 0x47);
    e.mem.set_u32(QUEST + 0x5c, QUEST_LOCALS);
    e.mem.set_u32(QUEST_LOCALS + 0xc, node);
    e.mem.set_u32(node, quest_var);
    e.mem.set_u32(quest_var, 298);
    e.mem.set_u32(quest_var + 8, CONTAINER_ID);

    let args = crate::CommandArgs {
        data: DATA,
        script: SCRIPT,
        this_obj: BASE,
        containing_obj: BASE,
        event_list: DATA + 0x2400,
        opcode_offset: RESULT_OFFSET,
        ..Default::default()
    };
    (e, args)
}

fn command(e: &mut Engine, opcode: u16, execute: u32, returns: Returns) {
    add_plugin_record(
        e,
        u32::from(opcode),
        &PluginRecord {
            name: "StackCommand",
            short: "",
            needs_ref: true,
            param_count: 0,
            params: 0,
            execute,
            eval: 0,
            flags: 0,
            expression: false,
            returns,
        },
    );
}

fn stack_command(opcode: u16) -> [u8; 9] {
    let [lo, hi] = opcode.to_le_bytes();
    // Pending command, no static ref index, length 4 including its two-byte argument count.
    [b'x', 0, 0, lo, hi, 4, 0, 0, 0]
}

fn evaluate(
    e: &mut Engine,
    args: &crate::CommandArgs,
    tokens: &[&[u8]],
) -> (Option<Tok>, Vec<String>) {
    let body = tokens.concat();
    e.mem.set_u32(args.opcode_offset, 4);
    e.mem.set_u16(DATA + 4, (body.len() + 2) as u16);
    e.mem.write(DATA + 6, &body);
    let mut ev = Eval::new(e, args);
    let answer = ev.evaluate(e);
    (answer, ev.errors)
}

#[test]
fn dot_calls_on_the_quest_reference_and_chains_its_result() {
    let (mut e, args) = fixture();
    let array = vars::create_array(&mut e, vars::KEY_NUMBER, true, 0);
    command(&mut e, STACK_COMMAND, 0x0201_0100, Returns::Form);
    command(&mut e, ARRAY_COMMAND, 0x0201_0200, Returns::Array);
    e.register_double(0x0201_0100, move |e, words| {
        let command = crate::CommandArgs::new(words);
        assert_eq!(command.this_obj, CONTAINER);
        assert_eq!(command.containing_obj, args.containing_obj);
        assert_eq!(command.event_list, args.event_list);
        assert_eq!(command.data, args.data);
        assert_eq!(command.script, args.script);
        assert_eq!(e.mem.u32(command.opcode_offset), 19);
        command.set_form_result(e, STACK_ID);
        crate::handled(true)
    });
    e.register_double(0x0201_0200, move |e, words| {
        let command = crate::CommandArgs::new(words);
        assert_eq!(command.this_obj, STACK);
        assert_eq!(e.mem.u32(command.opcode_offset), 29);
        command.set_result(e, f64::from(array));
        crate::handled(true)
    });

    // External ref token: quest ref-list entry 1, its variable 298 (JLM.rContainer).
    let quest_variable = [b'V', 4, 1, 0, 0x2a, 1];
    let result = evaluate(
        &mut e,
        &args,
        &[
            &quest_variable,
            &stack_command(STACK_COMMAND),
            &[op::DOT],
            &stack_command(ARRAY_COMMAND),
            &[op::DOT],
        ],
    );
    assert!(matches!(result.0, Some(Tok::Array(id)) if id == array));
    assert!(result.1.is_empty(), "{:?}", result.1);
}

#[test]
fn dot_reads_array_form_elements_and_rejects_a_base_or_null_form() {
    let (mut e, args) = fixture();
    let array = vars::create_array(&mut e, vars::KEY_NUMBER, true, 0);
    let event_node = DATA + 0x2410;
    let local = DATA + 0x2430;
    e.mem.set_u32(args.event_list + 0xc, event_node);
    e.mem.set_u32(event_node, local);
    e.mem.set_u32(local, 1);
    e.mem.set_f64(local + 8, f64::from(array));
    command(&mut e, STACK_COMMAND, 0x0201_0100, Returns::Default);
    e.register_double(0x0201_0100, |e, words| {
        let command = crate::CommandArgs::new(words);
        assert_eq!(command.this_obj, STACK);
        command.set_result(e, 7.0);
        crate::handled(true)
    });

    let array_variable = [b'V', 3, 0, 0, 1, 0];
    let index = [b'B', 0];
    let subscript = [op::LEFT_BRACKET];
    let call = stack_command(STACK_COMMAND);
    let dot = [op::DOT];
    let tokens = [&array_variable[..], &index, &subscript, &call, &dot];
    for (form, error) in [
        (STACK_ID, None),
        (
            BASE_ID,
            Some("Attempting to call a function on a base object (this must be a reference)"),
        ),
        (
            0,
            Some("Attempting to call a command on a non-form or a NULL reference"),
        ),
    ] {
        assert!(vars::set_element(
            &mut e,
            array,
            &Key::Number(0.0),
            Value::Form(form)
        ));
        let (answer, errors) = evaluate(&mut e, &args, &tokens);
        if let Some(error) = error {
            assert!(answer.is_none());
            assert_eq!(errors.first().map(String::as_str), Some(error));
        } else {
            assert!(matches!(answer, Some(Tok::Number { v, .. }) if v == 7.0));
            assert!(errors.is_empty());
        }
    }
}
