//! Command-level checks of JG 59049a0's branches. The formula is a call-recording
//! double, not an expected DPS value copied from the implementation.
use super::*;

const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const SCRIPT: u32 = DATA + 0x2000;
const WEAPON: u32 = DATA + 0x3000;
const PLAYER_FORM: u32 = DATA + 0x4000;
const PROCESS: u32 = DATA + 0x5000;
const ITEM: u32 = DATA + 0x6000;
const AMMO_ENTRY: u32 = DATA + 0x6100;
const AMMO_LOADED: u32 = DATA + 0x6200;
const AMMO_DEFAULT: u32 = DATA + 0x6300;
const WORLD_REF: u32 = DATA + 0x7000;
const STACK: u32 = DATA + 0x7100;
const XDATA: u32 = DATA + 0x7200;
const INV_REF: u32 = DATA + 0x7300;
const NPC: u32 = DATA + 0x7400;
const SEEN: u32 = DATA + 0x8000;
const FALLBACK_COUNT: u32 = DATA + 0x8100;

fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    e.map(DATA, 0x10000);
    nvse::table::install(&mut e);
    crate::commands::install(&mut e);
    e.register(CONSOLE_OUTPUT, |_, _| Ret::default());
    e.mem.set_u8(WEAPON + 4, WEAPON_KIND);
    e.mem.set_u32(WORLD_REF + 0xc, 0x1001);
    e.mem.set_u32(WORLD_REF + 0x20, WEAPON);
    e.mem.set_u32(PLAYER, PLAYER_FORM);
    e.mem.set_u32(PLAYER_FORM + 0x68, PROCESS);
    e.mem.set_u32(PROCESS + 0x114, ITEM);
    e.mem.set_u32(ITEM + 8, WEAPON);
    e.mem.set_u32(AMMO_ENTRY + 8, AMMO_LOADED);
    let vt = e.mem.alloc(0x150);
    e.mem.set_u32(PROCESS, vt);
    e.mem.set_u32(vt + 0x148, DATA + 0x9000);
    e.mem.set_u32(vt + 0x14c, DATA + 0x9004);
    e.register(DATA + 0x9000, |_, args| {
        assert_eq!(args, [PROCESS]);
        Ret {
            eax: ITEM,
            ..Ret::default()
        }
    });
    e.register(DATA + 0x9004, |_, args| {
        assert_eq!(args, [PROCESS]);
        Ret {
            eax: AMMO_ENTRY,
            ..Ret::default()
        }
    });
    e.register(WEAPON_AMMO, |e, args| {
        assert_eq!(args, [WEAPON, 0]);
        e.mem.set_u32(FALLBACK_COUNT, e.mem.u32(FALLBACK_COUNT) + 1);
        Ret {
            eax: AMMO_DEFAULT,
            ..Ret::default()
        }
    });
    e.register(REFERENCE_HEALTH, |_, args| {
        assert_eq!(args, [WORLD_REF]);
        275.0f32.into_ret()
    });
    e.register(ITEM_HEALTH, |_, args| {
        assert_eq!(args, [STACK, 1]);
        37.5f32.into_ret()
    });
    e.register(WEAPON_DPS, |e, args| {
        assert_eq!(e.mem.u32(PROCESS + 0x114), 0);
        for (i, value) in args.iter().enumerate() {
            e.mem.set_u32(SEEN + 4 * i as u32, *value);
        }
        assert_eq!(args.len(), 13);
        17.25f32.into_ret()
    });
    let rv = e.mem.alloc(0x10);
    e.mem.set_u32(rv + 8, WEAPON);
    e.mem.set_u32(SCRIPT + 0x44, rv);
    e
}

fn run(e: &mut Engine, subject: u32, named: bool) -> f64 {
    let bytes = if named { vec![b'r', 1, 0] } else { vec![] };
    e.mem.set_u16(DATA, 0x310a);
    e.mem.set_u16(DATA + 2, (bytes.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, u16::from(named));
    e.mem.write(DATA + 6, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.0);
    let record = nvse::table::record_of(e, 0x310a);
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
    assert_eq!(execute(e, &words).eax & 0xff, 1);
    e.mem.f64(RESULT)
}

fn assert_call(e: &Engine, condition: f32, ammo: u32) {
    let expected = [
        PLAYER_FORM + 0xa4,
        WEAPON,
        condition.to_bits(),
        1,
        ITEM,
        0,
        0,
        u32::MAX,
        0,
        0,
        0,
        0,
        ammo,
    ];
    let seen: Vec<u32> = (0..13).map(|i| e.mem.u32(SEEN + 4 * i)).collect();
    assert_eq!(seen, expected);
    assert_eq!(e.mem.u32(PROCESS + 0x114), ITEM);
}

fn inventory(e: &mut Engine, xdata: bool) {
    e.register(0x0040_1000, |_, _| Ret {
        eax: INV_REF,
        ..Ret::default()
    });
    e.register(0x0055_a2f0, |e, args| {
        e.mem.set_u32(args[0] + 0xc, 0xff00_0001);
        Ret::default()
    });
    nvse::invref::create(
        e,
        NPC,
        nvse::invref::Data {
            item: WEAPON,
            entry: STACK,
            xdata: if xdata { XDATA } else { 0 },
        },
        false,
    );
}

#[test]
fn named_weapon_uses_players_loaded_ammo_and_full_condition() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0, true), 17.25);
    assert_call(&e, 1.0, AMMO_LOADED);
    assert_eq!(e.mem.u32(FALLBACK_COUNT), 0);
}

#[test]
fn other_equipped_weapon_uses_requested_weapons_default_ammo() {
    let mut e = setup();
    e.mem.set_u32(ITEM + 8, WEAPON + 0x100);
    assert_eq!(run(&mut e, 0, true), 17.25);
    assert_call(&e, 1.0, AMMO_DEFAULT);
    assert_eq!(e.mem.u32(FALLBACK_COUNT), 1);
}

#[test]
fn world_reference_passes_raw_health_without_normalizing() {
    let mut e = setup();
    assert_eq!(run(&mut e, WORLD_REF, false), 17.25);
    assert_call(&e, 275.0, AMMO_LOADED);
}

#[test]
fn npc_inventory_stack_uses_players_context_and_its_own_condition() {
    let mut e = setup();
    inventory(&mut e, true);
    assert_eq!(run(&mut e, INV_REF, false), 17.25);
    assert_call(&e, 0.375, AMMO_DEFAULT);
}

#[test]
fn inventory_stack_without_extra_data_keeps_full_condition_and_loaded_ammo() {
    let mut e = setup();
    inventory(&mut e, false);
    assert_eq!(run(&mut e, INV_REF, false), 17.25);
    assert_call(&e, 1.0, AMMO_LOADED);
}

#[test]
fn invalid_or_missing_weapon_returns_zero_without_calling_formula() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0, false), 0.0);
    e.mem.set_u8(WEAPON + 4, 0x1f);
    assert_eq!(run(&mut e, 0, true), 0.0);
    assert_eq!(run(&mut e, WORLD_REF, false), 0.0);
    assert_eq!(e.mem.u32(SEEN), 0);
}

#[test]
fn process_weapon_is_restored_when_formula_stops() {
    let mut e = setup();
    e.register(WEAPON_DPS, |_, _| panic!("formula stop fixture"));
    assert!(catch_unwind(AssertUnwindSafe(|| run(&mut e, 0, true))).is_err());
    assert_eq!(e.mem.u32(PROCESS + 0x114), ITEM);
}

#[test]
fn console_echo_uses_scripts_tls_gate_even_with_console_window_closed() {
    let mut e = setup();
    // Script::GetConsoleOutput and xNVSE's IsConsoleMode differ here: the
    // former's TLS echo bit is on while the console-open global is off.
    assert_eq!(e.mem.u8(0x011d_ea2e), 0);
    e.register(CONSOLE_OUTPUT, |_, args| {
        assert!(args.is_empty());
        Ret {
            eax: 1,
            ..Ret::default()
        }
    });
    e.register(0x0071_b160, |_, args| {
        assert_eq!(args, [1]);
        Ret {
            eax: DATA + 0x9100,
            ..Ret::default()
        }
    });
    e.register(0x0071_d0a0, |e, args| {
        assert_eq!(args[0], DATA + 0x9100);
        assert_eq!(
            nvse::table::guest_text(e, args[1]),
            b"GetCalculatedWeaponDPS >> %f"
        );
        assert_eq!(e.mem.f64(args[2]), 17.25);
        e.mem.set_u32(SEEN + 0x60, 1);
        Ret::default()
    });
    assert_eq!(run(&mut e, 0, true), 17.25);
    assert_eq!(e.mem.u32(SEEN + 0x60), 1);
}
