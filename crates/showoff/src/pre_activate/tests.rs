use super::*;
use nvse::{functions, table};
const DATA: u32 = 0x0300_0000;
const TARGET: u32 = DATA + 0x1000;
const PLAYER_REF: u32 = DATA + 0x2000;
const OTHER: u32 = DATA + 0x3000;
const BASE: u32 = DATA + 0x4000;
const MANAGER: u32 = DATA + 0x5000;
const VT: u32 = DATA + 0x6000;
const IS_ACTOR: u32 = DATA + 0x7000;
const IS_CHILD: u32 = DATA + 0x7100;
const A: u32 = DATA + 0x8000;
const B: u32 = DATA + 0x8200;
const C: u32 = DATA + 0x8400;
#[derive(Default)]
struct State {
    calls: Vec<(u32, u32, u32, f64)>,
    actor: bool,
    child: bool,
    can_child: bool,
}
fn function(e: &mut Engine, at: u32, id: u32, value: u8) {
    let mut bytes = vec![
        0x1d, 0, 0, 0, 0x10, 0, 14, 0, 0x0d, 0, 0, 0, 0, 0, 1, 2, 1, 0, 4, 2, 0, 1,
    ];
    bytes.resize(0x40, 0);
    bytes.extend([1, 4, 0, b'B', value]);
    e.mem.write(at + 0x100, &bytes);
    e.mem.set_u8(at + 4, 0x11);
    e.mem.set_u32(at + 0xc, id);
    e.mem.set_u32(at + 0x30, at + 0x100);
    e.mem.set_u32(at + 0x20, bytes.len() as u32);
}
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(DATA, 0x10000);
    for page in [
        0x0118_e000,
        0x011c_3000,
        0x011c_9000,
        0x011c_a000,
        0x011d_8000,
        0x011d_e000,
    ] {
        e.map(page, if page == 0x0118_e000 { 0x9000 } else { 0x1000 });
    }
    table::install(&mut e);
    functions::install(&mut e);
    install(&mut e);
    e.mem.set_u32(functions::FUNCTION_BLOCK_RECORD + 8, 0x0d);
    e.mem.set_u32(PLAYER, PLAYER_REF);
    e.mem.set_u32(INTERFACE, MANAGER);
    for (at, id) in [(TARGET, 0x100), (PLAYER_REF, 0x14), (OTHER, 0x101)] {
        e.mem.set_u32(at, VT);
        e.mem.set_u8(at + 4, 0x3b);
        e.mem.set_u32(at + 0xc, id);
        e.mem.set_u32(at + 0x20, BASE);
    }
    e.mem.set_u32(VT + 0x100, IS_ACTOR);
    e.mem.set_u32(VT + 0xf0, IS_ACTOR);
    e.mem.set_u32(VT + 0x1a0, IS_CHILD);
    e.ext::<State>().actor = true;
    e.register(IS_ACTOR, |e, _| {
        u32::from(e.ext::<State>().actor).into_ret()
    });
    e.register(IS_CHILD, |e, a| {
        assert_eq!(a, [PLAYER_REF, 0]);
        u32::from(e.ext::<State>().child).into_ret()
    });
    e.register(0x0088_59e0, |e, a| {
        assert_eq!(a, [PLAYER_REF, TARGET]);
        u32::from(e.ext::<State>().can_child).into_ret()
    });
    e.register(0x0048_39c0, |_, a| {
        (match a[0] {
            0x14 => PLAYER_REF,
            0x100 => TARGET,
            0x101 => OTHER,
            _ => 0,
        })
        .into_ret()
    });
    e.register(functions::CREATE_EVENT_LIST, |e, a| {
        let list = e.mem.alloc(0x14);
        e.mem.set_u32(list, a[0]);
        let mut head = 0;
        for id in [2, 1] {
            let var = e.mem.alloc(0x10);
            e.mem.set_u32(var, id);
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, var);
            e.mem.set_u32(node + 4, head);
            head = node;
        }
        e.mem.set_u32(list + 0xc, head);
        list.into_ret()
    });
    e.register(functions::EVENT_LIST_DESTROY, |_, _| Ret::default());
    e.register(functions::SCRIPT_EXECUTE, |e, a| {
        let first = nvse::args::event_list_variable(e, a[2], 1);
        let second = nvse::args::event_list_variable(e, a[2], 2);
        let run = (a[0], a[1], e.mem.u32(first + 8), e.mem.f64(second + 8));
        e.ext::<State>().calls.push(run);
        if a[0] != C {
            let rec = table::record_of(e, functions::OP_SET_FUNCTION_VALUE);
            e.with_stack(12, |e, f| {
                e.mem.set_u32(f.addr() + 8, 0x40);
                e.call(
                    e.mem.u32(rec + 0x18),
                    &[
                        e.mem.u32(rec + 0x14),
                        e.mem.u32(a[0] + 0x30),
                        a[1],
                        0,
                        a[0],
                        a[2],
                        f.addr(),
                        f.addr() + 8,
                    ],
                );
            });
        }
        Ret::default()
    });
    function(&mut e, A, 0x0500_8000, 0);
    function(&mut e, B, 0x0500_8200, 1);
    function(&mut e, C, 0x0500_8400, 0);
    e
}
fn handler(e: &mut Engine, script: u32, first: u32) {
    let mut cb = events::Callback::new(script);
    cb.source = first;
    assert!(events::set_handler(e, EVENT, cb, 1, None));
}
fn run(e: &mut Engine, activator: u32) -> u32 {
    e.site(ACTIVATE.addr(), &[TARGET, activator])
        .expect("installed real seam")
}
#[test]
fn real_udf_veto_is_monotonic_later_handlers_see_zero_and_player_first_filter_is_real() {
    let mut e = engine();
    handler(&mut e, A, PLAYER_REF);
    handler(&mut e, B, 0);
    handler(&mut e, C, 0);
    assert_eq!(run(&mut e, PLAYER_REF), 0);
    assert_eq!(
        e.ext::<State>().calls,
        [
            (A, TARGET, 0x14, 1.0),
            (B, TARGET, 0x14, 0.0),
            (C, TARGET, 0x14, 0.0),
        ]
    );
    e.ext::<State>().calls.clear();
    assert_eq!(run(&mut e, OTHER), 1);
    assert_eq!(
        e.ext::<State>().calls,
        [(B, TARGET, 0x101, 1.0), (C, TARGET, 0x101, 1.0)]
    );
    assert!(events::current_event_name(&mut e).is_empty());
}
#[test]
fn double_activation_ash_pile_and_player_camera_pipboy_early_results_match_source() {
    let mut e = engine();
    handler(&mut e, A, 0);
    for global in [0x011c_9350, 0x011c_92d0] {
        e.mem.set_u32(global, TARGET);
        assert_eq!(run(&mut e, PLAYER_REF), 1);
        e.mem.set_u32(global, 0);
    }
    for global in [0x011c_a27c, 0x011c_a280] {
        e.mem.set_u32(global, BASE);
        assert_eq!(run(&mut e, PLAYER_REF), 1);
        e.mem.set_u32(global, 0);
    }
    e.mem.set_u32(MANAGER + 0x4bc, 2);
    assert_eq!(run(&mut e, PLAYER_REF), 2);
    e.mem.set_u8(PLAYER_REF + 0x64a, 1);
    assert_eq!(run(&mut e, PLAYER_REF), 2, "pipboy precedes camera refusal");
    e.mem.set_u32(MANAGER + 0x4bc, 0);
    assert_eq!(run(&mut e, PLAYER_REF), 0);
    assert!(e.ext::<State>().calls.is_empty());
    assert_eq!(
        run(&mut e, OTHER),
        0,
        "nonplayer skips player-state checks and runs veto"
    );
    assert_eq!(e.ext::<State>().calls.len(), 1);
}
#[test]
fn child_and_destroyed_checks_short_circuit_exactly_and_empty_event_defaults_allow() {
    let mut e = engine();
    assert_eq!(run(&mut e, PLAYER_REF), 1);
    handler(&mut e, A, 0);
    e.ext::<State>().child = true;
    e.ext::<State>().can_child = true;
    assert_eq!(run(&mut e, PLAYER_REF), 1);
    e.ext::<State>().can_child = false;
    assert_eq!(
        run(&mut e, PLAYER_REF),
        0,
        "source does not bypass when child predicate returns false"
    );
    e.ext::<State>().calls.clear();
    e.mem.set_u32(TARGET + 8, 0x0080_0000);
    e.ext::<State>().actor = false;
    assert_eq!(run(&mut e, PLAYER_REF), 1);
    assert!(e.ext::<State>().calls.is_empty());
    e.ext::<State>().actor = true;
    assert_eq!(run(&mut e, PLAYER_REF), 0);
}
