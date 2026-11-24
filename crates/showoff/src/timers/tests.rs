use super::*;
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const PARENT: u32 = DATA + 0x2000;
const OWNER: u32 = DATA + 0x3000;
const PLAYER_FORM: u32 = DATA + 0x4000;
const CALLBACK: u32 = DATA + 0x5000;
const OTHER_CALLBACK: u32 = DATA + 0x6000;
const BASE: u32 = DATA + 0x7000;
const FILE: u32 = DATA + 0x9000;
const REF1: u32 = DATA + 0xa000;
const REF2: u32 = REF1 + 0x10;
const REF_NODE: u32 = REF1 + 0x20;
const VT: u32 = DATA + 0xb000;
const SET_ID: u32 = DATA + 0xb200;
const NAME_FN: u32 = DATA + 0xb300;
#[derive(Clone, Default)]
enum Action {
    #[default]
    None,
    Restart,
    RestartImplicit,
    RestartNaN,
    StopThenRestart,
    NewTimer,
    NewGameTimer,
    Flush,
}
#[derive(Default)]
struct Test {
    calls: Vec<(u32, Vec<u8>, u32)>,
    action: Action,
    menu: bool,
    vats: f64,
    global: f64,
    messages: Vec<Vec<u8>>,
    clock_calls: usize,
}
fn form(e: &mut Engine, at: u32, id: u32, vt: u32) {
    e.mem.set_u32(at, vt);
    e.mem.set_u32(at + 0xc, id);
}
fn function(e: &mut Engine, at: u32, id: u32) {
    let mut bytes = vec![0x1d, 0, 0, 0, 0x10, 0];
    let block = vec![0x0d, 0, 0, 0, 0, 0, 1, 2, 1, 0, 2, 2, 0, 4];
    bytes.extend_from_slice(&(block.len() as u16).to_le_bytes());
    bytes.extend(block);
    let code = e.mem.alloc(bytes.len() as u32);
    e.mem.write(code, &bytes);
    form(e, at, id, SCRIPT_VT);
    e.mem.set_u8(at + 4, 0x11);
    e.mem.set_u32(at + 0x30, code);
    e.mem.set_u32(at + 0x20, bytes.len() as u32);
    e.mem.set_u32(at + 0x10, FILE);
}
fn event_list(e: &mut Engine, script: u32) -> u32 {
    let list = e.mem.alloc(0x14);
    e.mem.set_u32(list, script);
    let mut next = 0;
    for id in [2, 1] {
        let var = e.mem.alloc(0x10);
        e.mem.set_u32(var, id);
        let node = e.mem.alloc(8);
        e.mem.set_u32(node, var);
        e.mem.set_u32(node + 4, next);
        next = node;
    }
    e.mem.set_u32(list + 0xc, next);
    list
}
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(DATA, 0x10000);
    e.map(0x0103_7000, 0x1000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011c_3000, 0x1000);
    e.map(0x011f_6000, 0x1000);
    nvse::table::install(&mut e);
    nvse::functions::install(&mut e);
    crate::install(&mut e);
    e.mem
        .set_u32(nvse::functions::FUNCTION_BLOCK_RECORD + 8, 0x0d);
    e.mem.set_u8(FILE + 0x40c, 5);
    e.mem.write(FILE + 0x20, b"TimersTest.esp\0");
    function(&mut e, PARENT, 0x0500_2000);
    function(&mut e, CALLBACK, 0x0500_5000);
    function(&mut e, OTHER_CALLBACK, 0x0500_6000);
    e.mem.set_u32(VT + 0xf0, RETURN_TRUE);
    form(&mut e, OWNER, 0x0500_3000, VT);
    form(&mut e, PLAYER_FORM, 0x14, VT);
    form(&mut e, BASE, 0x0500_7000, VT + 0x100);
    e.mem.set_u32(OWNER + 0x20, BASE);
    e.mem.set_u32(PLAYER, PLAYER_FORM);
    e.mem.set_u32(SCRIPT_VT + 0x128, SET_ID);
    e.mem.set_u32(SCRIPT_VT + 0x130, NAME_FN);
    e.register(NAME_FN, |_, _| 0u32.into_ret());
    e.register(SET_ID, |e, a| {
        e.mem.set_u32(a[0] + 0xc, a[1]);
        Ret::default()
    });
    e.register(nvse::functions::CREATE_EVENT_LIST, |e, a| {
        event_list(e, a[0]).into_ret()
    });
    e.register(nvse::functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(0x0040_1000, |e, a| e.mem.alloc(a[0]).into_ret());
    e.register(0x005a_a0f0, |e, a| {
        e.mem.set_u32(a[0], SCRIPT_VT);
        e.mem.set_u8(a[0] + 4, 0x11);
        a[0].into_ret()
    });
    for address in [0x0046_4d30, 0x005a_b930, 0x005a_b7f0] {
        e.register(address, |_, _| Ret::default());
    }
    e.register(LOOKUP, |_, a| {
        (match a[0] {
            0x0500_3000 => OWNER,
            0x14 => PLAYER_FORM,
            0x0500_7000 => BASE,
            _ => 0,
        })
        .into_ret()
    });
    e.register(0x0071_6440, |e, _| {
        e.ext::<Test>().clock_calls += 1;
        e.ext::<Test>().global.into_ret()
    });
    e.register(0x009c_8cc0, |e, a| {
        assert_eq!(a, [0x011f_2250]);
        e.ext::<Test>().vats.into_ret()
    });
    e.register(0x0070_2360, |e, _| {
        u32::from(e.ext::<Test>().menu).into_ret()
    });
    e.register(0x0071_b160, |_, _| (DATA + 0xc000).into_ret());
    e.register(0x0071_d0a0, |e, a| {
        let message = table::guest_text(e, e.mem.u32(a[2]));
        e.ext::<Test>().messages.push(message);
        Ret::default()
    });
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, a| {
        assert_eq!(a[1], 0, "timer callback thisObj is null");
        assert_eq!(&a[3..], [0, 0]);
        let name = nvse::args::event_list_variable(e, a[2], 1);
        let owner = nvse::args::event_list_variable(e, a[2], 2);
        let name = nvse::vars::string(e, e.mem.f64(name + 8) as u32).unwrap();
        let owner = e.mem.u32(owner + 8);
        e.ext::<Test>().calls.push((a[0], name.clone(), owner));
        let action = std::mem::take(&mut e.ext::<Test>().action);
        match action {
            Action::Restart => {
                start_call(e, &name, Some(0.5), Some(13));
            }
            Action::RestartImplicit => {
                start_call(e, &name, None, None);
            }
            Action::RestartNaN => {
                start_call(e, &name, Some(f64::NAN), Some(13));
            }
            Action::StopThenRestart => {
                stop_call(e, &name, false);
                start_call(e, &name, Some(0.5), Some(13));
            }
            Action::NewTimer => {
                start_call(e, b"*new", Some(0.01), Some(13));
            }
            Action::NewGameTimer => {
                start_call(e, b"*new", Some(0.25), Some(1038));
            }
            Action::Flush => {
                assert_eq!(nvse::lambda::saved_count(e, a[0]), 1);
                message(
                    e,
                    &Message {
                        kind: plugins::msg::PRE_LOAD_GAME,
                        data: 0,
                        len: 0,
                    },
                );
                assert_eq!(
                    nvse::lambda::saved_count(e, a[0]),
                    1,
                    "in-flight callback retains captures across flush"
                );
            }
            Action::None => {}
        }
        1u32.into_ret()
    });
    e.ext::<Test>().menu = true;
    e.ext::<Test>().vats = 1.0;
    e.ext::<Test>().global = 1.0;
    e.map(0x011d_0000, 0x1000);
    e.mem.set_f32(0x011d_0fac, 0.25);
    e.mem.set_f32(0x011f_63a0, 0.25);
    e
}
fn text(bytes: &[u8]) -> Vec<u8> {
    [(bytes.len() as u16).to_le_bytes().to_vec(), bytes.to_vec()].concat()
}
fn integer(n: u32) -> Vec<u8> {
    [vec![b'n'], n.to_le_bytes().to_vec()].concat()
}
fn double(n: f64) -> Vec<u8> {
    [vec![b'z'], n.to_le_bytes().to_vec()].concat()
}
fn ref_arg(e: &mut Engine, form: u32, index: u8) -> Vec<u8> {
    e.mem.set_u32(PARENT + 0x44, REF1);
    let at = if index == 1 {
        REF1
    } else {
        e.mem.set_u32(PARENT + 0x48, REF_NODE);
        e.mem.set_u32(REF_NODE, REF2);
        REF2
    };
    e.mem.set_u32(at + 8, form);
    vec![b'r', index, 0]
}
fn call(e: &mut Engine, op: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let data = args.concat();
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, (data.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    e.mem.write(DATA + 6, &data);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.5);
    let rec = table::record_of(e, op);
    let args = [
        e.mem.u32(rec + 0x14),
        DATA,
        this,
        0,
        PARENT,
        0,
        RESULT,
        OFFSET,
    ];
    assert!(e.call(e.mem.u32(rec + 0x18), &args).bool());
    e.mem.f64(RESULT)
}
fn start_call(e: &mut Engine, name: &[u8], time: Option<f64>, flags: Option<u32>) -> f64 {
    let mut args = vec![text(name)];
    if let Some(time) = time {
        args.push(double(time));
    }
    if let Some(flags) = flags {
        args.push(integer(flags));
    }
    call(e, 0x3d3e, OWNER, &args)
}
fn stop_call(e: &mut Engine, name: &[u8], fire: bool) -> f64 {
    call(e, 0x3d3f, OWNER, &[text(name), integer(u32::from(fire))])
}
fn register(e: &mut Engine, script: u32, name: &[u8], add: bool) {
    let script = ref_arg(e, script, 1);
    assert_eq!(
        call(
            e,
            0x3d45,
            OWNER,
            &[integer(u32::from(add)), script, text(name)]
        ),
        1234.5
    );
}
fn left(e: &mut Engine, name: &[u8]) -> Option<f64> {
    let key = scope(e, PARENT, 0x0500_3000, name);
    state(e).timer(key).filter(|t| !t.pending).map(|t| t.left)
}
fn frame(e: &mut Engine) {
    plugins::dispatch(e, plugins::msg::MAIN_GAME_LOOP, 0, 0);
}

#[test]
fn time_left_real_record_keeps_double_precision_pending_missing_and_owner_semantics() {
    let mut e = engine();
    assert_eq!(call(&mut e, 0x3d43, OWNER, &[text(b"*missing")]), -1.0);
    assert_eq!(call(&mut e, 0x3d43, OWNER, &[]), -1.0);
    assert_eq!(call(&mut e, 0x3d43, OWNER, &[text(b"")]), -1.0);
    let exact = 1.123456789012345;
    assert_eq!(start_call(&mut e, b"*precise", Some(exact), Some(14)), 1.0);
    assert_eq!(call(&mut e, 0x3d43, OWNER, &[text(b"*PRECISE")]), exact);
    assert_eq!(
        call(&mut e, 0x3d43, 0, &[text(b"*precise")]),
        -1.0,
        "nullthis defaults to player, not OWNER"
    );
    stop_call(&mut e, b"*precise", false);
    assert_eq!(
        call(&mut e, 0x3d43, OWNER, &[text(b"*precise")]),
        -1.0,
        "pending removals look absent before cleanup"
    );
    let base = ref_arg(&mut e, BASE, 1);
    call(
        &mut e,
        0x3d3e,
        0,
        &[text(b"*base"), double(-2.5), integer(526), base.clone()],
    );
    assert_eq!(
        call(&mut e, 0x3d43, OWNER, &[text(b"*base"), base]),
        -2.5,
        "explicitform determines owner; negative remainingtime is not clamped"
    );
    call(
        &mut e,
        0x3d3e,
        0,
        &[text(b"*player"), double(exact), integer(1038)],
    );
    assert_eq!(call(&mut e, 0x3d43, 0, &[text(b"*player")]), exact);
    let rec = table::record_of(&e, 0x3d43);
    assert_eq!(
        e.mem.u32(rec + 0x20),
        0,
        "source query has no condition eval handler"
    );
}

#[test]
fn em_game_flags_use_source_mode_turbo_and_real_time_clocks_with_float_rounding() {
    let mut e = engine();
    e.ext::<Test>().vats = 0.3;
    e.ext::<Test>().global = 2.5;
    e.mem.set_f32(0x011f_63a0, 0.2);
    e.mem.set_f32(0x011d_0fac, 0.3);
    e.mem.set_u8(PLAYER_FORM + 0xe21, 1);
    for (name, flags) in [
        (b"*menu".as_slice(), 13),
        (b"*game".as_slice(), 14),
        (b"*turbo".as_slice(), 526),
        (b"*real".as_slice(), 1038),
    ] {
        assert_eq!(start_call(&mut e, name, Some(2.0), Some(flags)), 1.0);
    }
    frame(&mut e);
    let with = f64::from((f64::from(0.2f32) * 0.3) as f32);
    assert_eq!(left(&mut e, b"*menu"), Some(2.0 - with));
    for name in [
        b"*game".as_slice(),
        b"*turbo".as_slice(),
        b"*real".as_slice(),
    ] {
        assert_eq!(left(&mut e, name), Some(2.0));
    }
    e.ext::<Test>().menu = false;
    frame(&mut e);
    assert_eq!(left(&mut e, b"*menu"), Some(2.0 - with));
    assert_eq!(left(&mut e, b"*game"), Some(2.0 - with));
    let without_turbo = f64::from(((f64::from(0.2f32) * 0.3) / f64::from(0.3f32)) as f32);
    assert_eq!(left(&mut e, b"*turbo"), Some(2.0 - without_turbo));
    let real = f64::from((f64::from(0.2f32) / 2.5) as f32);
    assert_eq!(left(&mut e, b"*real"), Some(2.0 - real));
    e.mem.set_u8(PLAYER_FORM + 0xe21, 0);
    frame(&mut e);
    assert_eq!(left(&mut e, b"*turbo"), Some(2.0 - without_turbo - with));
}

#[test]
fn em_expiry_callbacks_implicit_restart_and_deferred_insert_preserve_modes() {
    let mut e = engine();
    e.ext::<Test>().menu = false;
    register(&mut e, CALLBACK, b"*game", true);
    start_call(&mut e, b"*game", Some(0.25), Some(526));
    e.ext::<Test>().action = Action::RestartImplicit;
    frame(&mut e);
    assert_eq!(left(&mut e, b"*game"), Some(0.25));
    let key = scope(&e, PARENT, 0x0500_3000, b"*game");
    assert_eq!(state(&mut e).timer(key).unwrap().flags, 526);
    e.ext::<Test>().action = Action::NewGameTimer;
    frame(&mut e);
    assert_eq!(left(&mut e, b"*game"), None);
    assert_eq!(
        left(&mut e, b"*new"),
        Some(0.25),
        "new callback timer is inserted only after iteration"
    );
    let new = scope(&e, PARENT, 0x0500_3000, b"*new");
    assert_eq!(state(&mut e).timer(new).unwrap().flags, 1038);
    e.ext::<Test>().global = 2.0;
    frame(&mut e);
    assert_eq!(left(&mut e, b"*new"), Some(0.125));
    message(
        &mut e,
        &Message {
            kind: plugins::msg::PRE_LOAD_GAME,
            data: 0,
            len: 0,
        },
    );
    assert_eq!(
        left(&mut e, b"*new"),
        None,
        "all supported modes have auto-remove-on-load bit8"
    );
    assert_eq!(e.ext::<Test>().calls.len(), 2);
}

#[test]
fn supported_explicit_restart_changes_mode_but_implicit_flags_seven_retains_it() {
    let mut e = engine();
    start_call(&mut e, b"*mode", Some(1.0), Some(14));
    assert_eq!(start_call(&mut e, b"*mode", Some(-1.0), Some(1038)), 1.0);
    let key = scope(&e, PARENT, 0x0500_3000, b"*mode");
    assert_eq!(state(&mut e).timer(key).unwrap().flags, 1038);
    assert_eq!(start_call(&mut e, b"*mode", None, None), 1.0);
    assert_eq!(state(&mut e).timer(key).unwrap().flags, 1038);
    assert_eq!(start_call(&mut e, b"*mode", Some(2.0), Some(526)), 1.0);
    assert_eq!(state(&mut e).timer(key).unwrap().flags, 526);
    assert_eq!(left(&mut e, b"*mode"), Some(2.0));
}

#[test]
fn exact_records_required_optional_widths_aliases_and_absent_unsupported_queries() {
    let e = engine();
    for (op, alias, spec) in [
        (
            0x3d3e,
            "AuxTimerStart",
            &[(0, false), (0x2c, true), (1, true), (0x3d, true)][..],
        ),
        (
            0x3d3f,
            "AuxTimerErase",
            &[(0, false), (1, true), (0x3d, true)][..],
        ),
        (0x3d43, "AuxTimerTimeLeft", &[(0, false), (0x3d, true)][..]),
        (
            0x3d45,
            "SetOnTimerStopHandler",
            &[(1, false), (0x3d, false), (0, false), (0x3d, true)][..],
        ),
    ] {
        assert_eq!(table::opcode_by_name(&e, alias.as_bytes()), Some(op));
        let rec = table::record_of(&e, op);
        assert_eq!(e.mem.u16(rec + 0x10), 0);
        assert_eq!(e.mem.u16(rec + 0x12), spec.len() as u16);
        assert_eq!(table::return_type(&e, op), Returns::Default);
        let params = e.mem.u32(rec + 0x14);
        for (i, (kind, optional)) in spec.iter().enumerate() {
            assert_eq!(e.mem.u32(params + 12 * i as u32 + 4), *kind);
            assert_eq!(e.mem.u32(params + 12 * i as u32 + 8), u32::from(*optional));
        }
    }
    assert_eq!(table::opcode_by_name(&e, b"AuxTimerElapsed"), None);
    assert_eq!(table::opcode_by_name(&e, b"SetOnTimerUpdateHandler"), None);
}
#[test]
fn real_main_loop_uses_menu_clock_and_actual_alt_udf_string_owner_restart() {
    let mut e = engine();
    register(&mut e, CALLBACK, b"*key", true);
    assert_eq!(start_call(&mut e, b"*key", Some(0.5), Some(13)), 1.0);
    e.ext::<Test>().menu = false;
    frame(&mut e);
    assert_eq!(left(&mut e, b"*key"), Some(0.5));
    assert!(e.ext::<Test>().calls.is_empty());
    e.ext::<Test>().menu = true;
    e.ext::<Test>().global = 50.0;
    e.ext::<Test>().vats = 0.5;
    frame(&mut e);
    assert_eq!(
        left(&mut e, b"*key"),
        Some(0.375),
        "flags13 does not divide out global multiplier"
    );
    e.ext::<Test>().vats = 1.0;
    frame(&mut e);
    assert_eq!(left(&mut e, b"*key"), Some(0.125));
    e.ext::<Test>().action = Action::Restart;
    frame(&mut e);
    assert_eq!(
        e.ext::<Test>().calls,
        [(CALLBACK, b"*key".to_vec(), 0x0500_3000)]
    );
    assert_eq!(
        left(&mut e, b"*key"),
        Some(0.5),
        "expiry callback restart survives"
    );
    frame(&mut e);
    frame(&mut e);
    assert_eq!(e.ext::<Test>().calls.len(), 2);
    assert_eq!(left(&mut e, b"*key"), None);
}
#[test]
fn frame_delta_has_source_double_multiplication_then_float_rounding() {
    let mut e = engine();
    start_call(&mut e, b"*delta", Some(1.0), Some(13));
    e.mem.set_f32(0x011f_63a0, 0.2);
    e.ext::<Test>().vats = 0.333333333333;
    frame(&mut e);
    let expected = 1.0 - f64::from((f64::from(0.2f32) * 0.333333333333) as f32);
    assert_eq!(left(&mut e, b"*delta"), Some(expected));
}
#[test]
fn source_expiry_removes_a_nan_restart_while_an_initial_nan_timer_never_expires() {
    let mut e = engine();
    register(&mut e, CALLBACK, b"*key", true);
    start_call(&mut e, b"*key", Some(0.25), Some(13));
    e.ext::<Test>().action = Action::RestartNaN;
    frame(&mut e);
    assert_eq!(
        left(&mut e, b"*key"),
        None,
        "source preserves only remaining >0 after expiry callback"
    );
    start_call(&mut e, b"*key", Some(f64::NAN), Some(13));
    frame(&mut e);
    assert!(left(&mut e, b"*key").unwrap().is_nan());
    assert_eq!(e.ext::<Test>().calls.len(), 1);
}
#[test]
fn source_pending_mutations_avoid_mid_iteration_insertion_and_support_stop_restart() {
    let mut e = engine();
    register(&mut e, CALLBACK, b"*key", true);
    start_call(&mut e, b"*key", Some(0.25), Some(13));
    e.ext::<Test>().action = Action::StopThenRestart;
    frame(&mut e);
    assert_eq!(left(&mut e, b"*key"), Some(0.5));
    assert!(state(&mut e).insertions.is_empty());
    e.ext::<Test>().action = Action::NewTimer;
    frame(&mut e);
    frame(&mut e);
    assert_eq!(
        left(&mut e, b"*new"),
        Some(0.01),
        "new callback timer is not decremented in insertion frame"
    );
    frame(&mut e);
    assert_eq!(left(&mut e, b"*new"), None);
    start_call(&mut e, b"*key", Some(1.0), Some(13));
    assert_eq!(stop_call(&mut e, b"*key", false), 1.0);
    assert_eq!(start_call(&mut e, b"*key", Some(0.5), Some(13)), 1.0);
    frame(&mut e);
    assert_eq!(left(&mut e, b"*key"), Some(0.25));
    e.ext::<Test>().action = Action::Restart;
    assert_eq!(stop_call(&mut e, b"*key", true), 1.0);
    assert_eq!(
        left(&mut e, b"*key"),
        None,
        "explicit stop marks pending AFTER its callback, winning over restart"
    );
    frame(&mut e);
    assert_eq!(left(&mut e, b"*key"), None);
}
#[test]
fn handler_add_remove_duplicate_and_preload_flush_have_source_queue_order() {
    let mut e = engine();
    register(&mut e, CALLBACK, b"*key", true);
    register(&mut e, CALLBACK, b"*KEY", true);
    register(&mut e, CALLBACK, b"*key", false);
    assert_eq!(
        state(&mut e).queued.len(),
        1,
        "remove does not cancel pending add"
    );
    start_call(&mut e, b"*key", Some(1.0), Some(13));
    stop_call(&mut e, b"*key", true);
    assert!(e.ext::<Test>().calls.is_empty());
    frame(&mut e);
    start_call(&mut e, b"*key", Some(1.0), Some(13));
    register(&mut e, CALLBACK, b"*key", false);
    stop_call(&mut e, b"*key", true);
    assert_eq!(
        e.ext::<Test>().calls.len(),
        1,
        "deleted callback runs until next tick"
    );
    frame(&mut e);
    assert!(state(&mut e).active.is_empty());
    register(&mut e, CALLBACK, b"*key", true);
    message(
        &mut e,
        &Message {
            kind: plugins::msg::PRE_LOAD_GAME,
            data: 0,
            len: 0,
        },
    );
    assert_eq!(state(&mut e).queued.len(), 1);
    frame(&mut e);
    assert_eq!(state(&mut e).active.len(), 1);
    start_call(&mut e, b"*key", Some(1.0), Some(13));
    message(
        &mut e,
        &Message {
            kind: plugins::msg::PRE_LOAD_GAME,
            data: 0,
            len: 0,
        },
    );
    assert!(state(&mut e).active.is_empty());
    assert_eq!(left(&mut e, b"*key"), None);
    assert_eq!(
        e.ext::<Test>().calls.len(),
        1,
        "load removal fires no stop callbacks"
    );
}
#[test]
fn namespace_and_owner_filters_are_literal_ids_not_wildcards_or_reference_bases() {
    let mut e = engine();
    let other_file = e.mem.alloc(0x500);
    e.mem.set_u8(other_file + 0x40c, 6);
    e.mem.set_u32(OTHER_CALLBACK + 0x10, other_file);
    register(&mut e, OTHER_CALLBACK, b"*private", true);
    start_call(&mut e, b"*private", Some(0.25), Some(13));
    frame(&mut e);
    assert!(e.ext::<Test>().calls.is_empty());
    register(&mut e, OTHER_CALLBACK, b"*_public", true);
    start_call(&mut e, b"*_PUBLIC", Some(0.25), Some(13));
    frame(&mut e);
    assert_eq!(e.ext::<Test>().calls.len(), 1);
    let callback = ref_arg(&mut e, CALLBACK, 1);
    let target = ref_arg(&mut e, BASE, 2);
    call(
        &mut e,
        0x3d45,
        OWNER,
        &[integer(1), callback, text(b"*base"), target],
    );
    frame(&mut e);
    let target = ref_arg(&mut e, OWNER, 1);
    assert_eq!(
        call(
            &mut e,
            0x3d3e,
            OWNER,
            &[text(b"*base"), double(0.25), integer(13), target]
        ),
        1.0
    );
    frame(&mut e);
    assert_eq!(
        e.ext::<Test>().calls.last().unwrap(),
        &(CALLBACK, b"*base".to_vec(), 0x0500_7000)
    );
}
#[test]
fn unsupported_modes_permanent_storage_and_defaults_are_explicit_and_do_not_mutate() {
    let mut e = engine();
    e.ext::<nvse::Nvse>().main_thread_recorded = true;
    assert_eq!(start_call(&mut e, b"permanent", Some(1.0), Some(13)), 0.0);
    assert_eq!(start_call(&mut e, b"*game", Some(1.0), None), 0.0);
    assert_eq!(start_call(&mut e, b"*frames", Some(1.0), Some(1)), 0.0);
    assert!(state(&mut e).keys().is_empty());
    assert_eq!(e.ext::<Test>().messages.len(), 3);
    assert!(e
        .ext::<Test>()
        .messages
        .iter()
        .all(|m| String::from_utf8_lossy(m)
            .contains("only temporary seconds timers with flags13,14,526,1038")));
    start_call(&mut e, b"*key", Some(1.0), Some(13));
    frame(&mut e);
    assert_eq!(start_call(&mut e, b"*key", None, None), 1.0);
    assert_eq!(left(&mut e, b"*key"), Some(1.0));
    assert_eq!(start_call(&mut e, b"*missing", None, None), 0.0);
}
#[test]
fn source_map_bucket_head_and_growth_order_are_preserved() {
    let mut map = Map::new(false);
    for key in 0..10 {
        map.insert(key, || key);
    }
    assert_eq!(
        map.entries.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
        [0, 5, 4, 9, 3, 8, 2, 7, 1, 6]
    );
    let mut map = Map::new(true);
    for key in [0, 8, 16] {
        map.insert(key, || key);
    }
    assert_eq!(
        map.entries.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
        [16, 8, 0]
    );
}
#[test]
fn real_thread_engine_views_share_timers_and_handler_queue() {
    let mut owner = engine();
    let mut peer = engine();
    owner.share_slots_for_test(&mut peer);
    nvse::table::install(&mut peer);
    install(&mut peer);
    start_call(&mut peer, b"*shared", Some(1.0), Some(13));
    assert_eq!(left(&mut owner, b"*shared"), Some(1.0));
    register(&mut peer, CALLBACK, b"*shared", true);
    assert_eq!(state(&mut owner).queued.len(), 1);
    frame(&mut owner);
    assert_eq!(left(&mut peer, b"*shared"), Some(0.75));
}
#[test]
fn lambda_capture_lifetime_includes_queue_and_inflight_load_flush() {
    let mut e = engine();
    let parent_list = event_list(&mut e, PARENT);
    let data = e.mem.u32(CALLBACK + 0x30);
    let len = e.mem.u32(CALLBACK + 0x20);
    let lambda = nvse::lambda::create(&mut e, DATA + 0x180, data, len, PARENT, parent_list);
    register(&mut e, lambda, b"*key", true);
    register(&mut e, lambda, b"*key", true);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 1);
    start_call(&mut e, b"*key", Some(0.25), Some(13));
    e.ext::<Test>().action = Action::Flush;
    frame(&mut e);
    assert_eq!(nvse::lambda::saved_count(&e, lambda), 0);
    assert!(state(&mut e).active.is_empty());
    assert_eq!(state(&mut e).dispatches, 0);
}
