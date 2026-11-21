use super::*;
use nvse::table;

const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const SCRIPT: u32 = DATA + 0x2000;
const FORM: u32 = DATA + 0x4000;
const SUBJECT: u32 = DATA + 0x5000;
const STATE: u32 = DATA + 0x8000;

fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_8000, 0x7000);
    e.map(DATA, 0x10000);
    nvse::table::install(&mut e);
    crate::commands::install(&mut e);
    e.register(0x005e_23f0, |e, _| Ret {
        eax: e.mem.u32(STATE + 16),
        ..Ret::default()
    });
    e
}

fn num(value: f64) -> Vec<u8> {
    [vec![b'z'], value.to_le_bytes().to_vec()].concat()
}

fn form_arg(e: &mut Engine, form: u32) -> Vec<u8> {
    let entry = e.mem.alloc(0x10);
    e.mem.set_u32(entry + 8, form);
    e.mem.set_u32(SCRIPT + 0x44, entry);
    vec![b'r', 1, 0]
}

fn call(e: &mut Engine, opcode: u32, subject: u32, args: &[Vec<u8>]) -> f64 {
    let bytes = args.concat();
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, (bytes.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    e.mem.write(DATA + 6, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.0);
    let record = table::record_of(e, opcode);
    let words = [
        e.mem.u32(record + 0x14),
        DATA,
        subject,
        0,
        SCRIPT,
        0,
        RESULT,
        OFFSET,
    ];
    assert_eq!(e.call(e.mem.u32(record + 0x18), &words).eax & 0xff, 1);
    e.mem.f64(RESULT)
}

#[test]
fn effect_queries_require_the_exact_effect_setting_vtable_and_keep_minus_one_defaults() {
    let mut e = engine();
    e.mem.set_u32(FORM, EFFECT_SETTING_VTABLE);
    e.mem.set_u8(FORM + 4, 0x10);
    e.mem.set_u32(FORM + 0x98, 4);
    e.mem.set_u32(FORM + 0x9c, 16);
    e.register(EFFECT_HAS_ACTOR_VALUE, |e, a| {
        assert_eq!(a, [FORM]);
        Ret {
            eax: e.mem.u32(STATE),
            ..Ret::default()
        }
    });
    e.register(EFFECT_ACTOR_VALUE, |e, a| {
        assert_eq!(a, [FORM]);
        Ret {
            eax: e.mem.u32(FORM + 0x9c),
            ..Ret::default()
        }
    });
    e.register(EFFECT_ARCHETYPE, |e, a| {
        assert_eq!(a, [FORM]);
        Ret {
            eax: e.mem.u32(FORM + 0x98),
            ..Ret::default()
        }
    });
    let arg = form_arg(&mut e, FORM);
    assert_eq!(call(&mut e, 0x3103, 0, std::slice::from_ref(&arg)), -1.0);
    e.mem.set_u32(STATE, 1);
    assert_eq!(call(&mut e, 0x3103, 0, std::slice::from_ref(&arg)), 16.0);
    assert_eq!(call(&mut e, 0x3104, 0, std::slice::from_ref(&arg)), 4.0);
    e.mem.set_u32(FORM + 0x9c, u32::MAX);
    assert_eq!(call(&mut e, 0x3103, 0, std::slice::from_ref(&arg)), -1.0);
    // An object with the same type byte but a different vtable is rejected.
    e.mem.set_u32(FORM, EFFECT_SETTING_VTABLE + 4);
    assert_eq!(call(&mut e, 0x3103, 0, std::slice::from_ref(&arg)), -1.0);
    assert_eq!(call(&mut e, 0x3104, 0, std::slice::from_ref(&arg)), -1.0);
    for opcode in [0x3103, 0x3104] {
        assert_eq!(call(&mut e, opcode, 0, &[]), -1.0);
        let null = form_arg(&mut e, 0);
        assert_eq!(call(&mut e, opcode, 0, &[null]), -1.0);
    }
}

#[test]
fn pipboy_toggle_preserves_menu_filter_and_intermediate_mode_success() {
    let mut e = engine();
    e.mem.set_u32(INTERFACE_MANAGER, FORM);
    e.register(OPEN_PIPBOY, |e, a| {
        assert_eq!(&a[..2], [FORM, 0]);
        e.mem.set_u32(STATE, 1);
        e.mem.set_u32(STATE + 4, a[2]);
        Ret::default()
    });
    e.register(CLOSE_PIPBOY, |e, a| {
        assert_eq!(a, [FORM, 0]);
        e.mem.set_u32(STATE, 2);
        Ret::default()
    });
    for menu in [0, 1002, 1003, 1023] {
        e.mem.set_u32(FORM + PIPBOY_MODE, 0);
        assert_eq!(call(&mut e, 0x3109, 0, &[num(menu as f64)]), 1.0);
        assert_eq!(e.mem.u32(STATE), 1);
        assert_eq!(e.mem.u32(STATE + 4), menu);
        e.mem.set_u32(FORM + PIPBOY_MODE, 3);
        assert_eq!(call(&mut e, 0x3109, 0, &[num(menu as f64)]), 1.0);
        assert_eq!(e.mem.u32(STATE), 2);
        for mode in [1, 2, 4] {
            e.mem.set_u32(STATE, 0);
            e.mem.set_u32(FORM + PIPBOY_MODE, mode);
            assert_eq!(call(&mut e, 0x3109, 0, &[num(menu as f64)]), 1.0);
            assert_eq!(e.mem.u32(STATE), 0);
        }
    }
    e.mem.set_u32(FORM + PIPBOY_MODE, 0);
    assert_eq!(call(&mut e, 0x3109, 0, &[]), 1.0);
    assert_eq!(e.mem.u32(STATE + 4), 0);
    for menu in [1, 1004, 1008, u32::MAX] {
        e.mem.set_u32(STATE, 0);
        assert_eq!(call(&mut e, 0x3109, 0, &[num(menu as f64)]), 0.0);
        assert_eq!(e.mem.u32(STATE), 0);
    }
    e.mem.set_u32(INTERFACE_MANAGER, 0);
    assert_eq!(call(&mut e, 0x3109, 0, &[]), 0.0);
}

fn crime_engine() -> Engine {
    let mut e = engine();
    let vt = e.mem.alloc(0x110);
    e.mem.set_u32(SUBJECT, vt);
    e.mem.set_u32(vt + 0x100, 0x0301_0000);
    e.mem.set_u32(PLAYER, FORM);
    for (address, offset) in [
        (0x0301_0000, 0),
        (IS_CRIME_TO_ACTIVATE, 4),
        (IS_PLAYER_TEAMMATE, 8),
    ] {
        match offset {
            0 => e.register(address, |e, a| {
                assert_eq!(a, [SUBJECT]);
                Ret {
                    eax: e.mem.u32(STATE),
                    ..Ret::default()
                }
            }),
            4 => e.register(address, |e, a| {
                assert_eq!(a, [SUBJECT]);
                Ret {
                    eax: e.mem.u32(STATE + 4),
                    ..Ret::default()
                }
            }),
            _ => e.register(address, |e, a| {
                assert_eq!(a, [SUBJECT]);
                Ret {
                    eax: e.mem.u32(STATE + 8),
                    ..Ret::default()
                }
            }),
        }
    }
    e.register(IS_IN_COMBAT_WITH_ACTOR, |e, a| {
        assert_eq!(a, [SUBJECT, FORM]);
        Ret {
            eax: e.mem.u32(STATE + 12),
            ..Ret::default()
        }
    });
    e
}

#[test]
fn crime_query_distinguishes_ownership_teammates_combat_and_compass_hostility() {
    let mut e = crime_engine();
    assert_eq!(call(&mut e, 0x31a5, 0, &[]), 0.0);
    assert_eq!(call(&mut e, 0x31a5, SUBJECT, &[]), 0.0);
    e.mem.set_u32(STATE + 4, 1);
    assert_eq!(call(&mut e, 0x31a5, SUBJECT, &[]), 1.0);
    e.mem.set_u32(STATE, 1);
    assert_eq!(call(&mut e, 0x31a5, SUBJECT, &[]), 1.0);
    e.mem.set_u32(STATE + 8, 1);
    assert_eq!(call(&mut e, 0x31a5, SUBJECT, &[]), 0.0);
    e.mem.set_u32(STATE + 12, 1);
    assert_eq!(call(&mut e, 0x31a5, SUBJECT, &[]), 1.0);
    e.mem.set_u32(STATE + 12, 0);
    // A non-hostile matching entry must not hide a later hostile match.
    let nodes = e.mem.alloc(24);
    let items = e.mem.alloc(24);
    e.mem.set_u32(FORM + PERCEIVED_ACTORS, nodes);
    for (index, actor, hostile) in [(0, SUBJECT, 0), (1, FORM, 1), (2, SUBJECT, 1)] {
        e.mem.set_u32(nodes + index * 8, items + index * 8);
        e.mem.set_u32(
            nodes + index * 8 + 4,
            if index == 2 {
                0
            } else {
                nodes + (index + 1) * 8
            },
        );
        e.mem.set_u32(items + index * 8, actor);
        e.mem.set_u8(items + index * 8 + 4, hostile);
    }
    assert_eq!(call(&mut e, 0x31a5, SUBJECT, &[]), 1.0);
    e.mem.set_u8(items + 20, 0);
    assert_eq!(call(&mut e, 0x31a5, SUBJECT, &[]), 0.0);
    // Only actors take the combat/compass branch.
    e.mem.set_u32(STATE, 0);
    e.mem.set_u32(STATE + 4, 0);
    e.mem.set_u8(items + 20, 1);
    e.mem.set_u32(STATE + 12, 1);
    assert_eq!(call(&mut e, 0x31a5, SUBJECT, &[]), 0.0);
}

#[test]
fn crime_console_output_uses_the_game_echo_predicate_and_double_format() {
    let mut e = crime_engine();
    e.mem.set_u32(STATE + 4, 1);
    e.mem.set_u32(STATE + 16, 1);
    // The xNVSE console-open byte remains zero: JG asks the game's TLS-only
    // Script::GetConsoleOutput rather than IsConsoleMode.
    e.mem.set_u8(0x011d_ea2e, 0);
    e.register(0x0071_b160, |_, a| {
        assert_eq!(a, [1]);
        Ret {
            eax: DATA + 0x9000,
            ..Ret::default()
        }
    });
    e.register(0x0071_d0a0, |e, a| {
        assert_eq!(a[0], DATA + 0x9000);
        assert_eq!(table::guest_text(e, a[1]), b"IsCrimeOrEnemy >> %.f");
        assert_eq!(e.mem.f64(a[2]), 1.0);
        e.mem.set_u32(STATE + 20, 1);
        Ret::default()
    });
    assert_eq!(call(&mut e, 0x31a5, SUBJECT, &[]), 1.0);
    assert_eq!(e.mem.u32(STATE + 20), 1);
}
