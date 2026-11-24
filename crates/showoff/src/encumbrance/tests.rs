use super::*;
const AREA: u32 = 0x0300_0000;
const ACTOR: u32 = AREA + 0x1000;
const DATA: u32 = AREA + 0x2000;
const RESULT: u32 = DATA + 0x800;
#[derive(Default)]
struct Values {
    god: u32,
    weight: i32,
    max: f32,
    weight_calls: usize,
    max_calls: usize,
    god_calls: usize,
}
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0118_e000, 0x9000);
    e.map(0x011d_e000, 0x1000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.mem.set_u32(PLAYER, ACTOR);
    let vt = e.mem.alloc(0x20);
    e.mem.set_u32(ACTOR + 0xa4, vt);
    e.mem.set_u32(vt + 8, AREA + 0x8000);
    e.register(AREA + 0x8000, |e, a| {
        assert_eq!(a, [ACTOR + 0xa4, 0x2e]);
        e.ext::<Values>().weight_calls += 1;
        e.ext::<Values>().weight.into_ret()
    });
    e.register(0x008a_0c20, |e, a| {
        assert_eq!(a, [ACTOR]);
        e.ext::<Values>().max_calls += 1;
        e.ext::<Values>().max.into_ret()
    });
    e.register(0x0095_26b0, |e, a| {
        assert!(a.is_empty());
        e.ext::<Values>().god_calls += 1;
        e.ext::<Values>().god.into_ret()
    });
    e
}
fn call(e: &mut Engine, argument: Option<i32>) -> f64 {
    let mut payload = Vec::new();
    if let Some(v) = argument {
        payload.push(b'n');
        payload.extend(v.to_le_bytes());
    }
    e.mem.set_u16(DATA, OPCODE as u16);
    e.mem.set_u16(DATA + 2, (payload.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, u16::from(argument.is_some()));
    e.mem.write(DATA + 6, &payload);
    e.mem.set_u32(RESULT + 8, 4);
    let r = table::record_of(e, OPCODE);
    let words = [e.mem.u32(r + 0x14), DATA, 0, 0, 0, 0, RESULT, RESULT + 8];
    assert_eq!(e.call(e.mem.u32(r + 0x18), &words).eax & 0xff, 1);
    e.mem.f64(RESULT)
}
#[test]
fn metadata_alias_and_real_command_defaults_god_mode_and_unsigned_override() {
    let mut e = setup();
    let r = table::record_of(&e, OPCODE);
    assert_eq!(
        table::guest_text(&e, e.mem.u32(r)),
        b"GetIsPlayerOverencumbered"
    );
    assert_eq!(
        table::guest_text(&e, e.mem.u32(r + 4)),
        b"IsPCOverencumbered"
    );
    assert_eq!(e.mem.u16(r + 0x12), 1);
    assert_eq!(e.mem.u16(r + 0x10), 0);
    let p = e.mem.u32(r + 0x14);
    assert_eq!(e.mem.u32(p + 4), 1);
    assert_eq!(e.mem.u32(p + 8), 1);
    assert_ne!(e.mem.u32(r + 0x1c), 0);
    e.ext::<Values>().weight = 250;
    e.ext::<Values>().max = 200.0;
    assert_eq!(call(&mut e, None), 1.0);
    e.ext::<Values>().god = 1;
    assert_eq!(call(&mut e, None), 0.0);
    assert_eq!(e.ext::<Values>().weight_calls, 1);
    assert_eq!(call(&mut e, Some(-1)), 1.0);
    assert_eq!(e.ext::<Values>().weight_calls, 2);
    assert_eq!(e.ext::<Values>().god_calls, 2);
}
#[test]
fn condition_preserves_float_comparison_equality_nan_rounding_and_signed_weight() {
    let mut e = setup();
    for (weight, max, want) in [
        (200, 200.0, 0.0),
        (201, 200.0, 1.0),
        (-1, 0.0, 0.0),
        (0, -1.0, 1.0),
        (i32::MAX, 2147483648.0, 0.0),
        (10, f32::NAN, 0.0),
    ] {
        e.ext::<Values>().weight = weight;
        e.ext::<Values>().max = max;
        condition(&mut e, &[0, 1, 0, RESULT]);
        assert_eq!(e.mem.f64(RESULT), want);
    }
    assert_eq!(e.ext::<Values>().god_calls, 0);
    assert_eq!(e.ext::<Values>().max_calls, 6);
}
#[test]
fn malformed_optional_argument_does_not_skip_original_query() {
    let mut e = setup();
    e.ext::<Values>().weight = 201;
    e.ext::<Values>().max = 200.0;
    e.mem.set_u16(DATA, OPCODE as u16);
    e.mem.set_u16(DATA + 2, 3);
    e.mem.set_u16(DATA + 4, 1);
    e.mem.set_u8(DATA + 6, b'x');
    e.mem.set_u32(RESULT + 8, 4);
    let r = table::record_of(&e, OPCODE);
    let params = e.mem.u32(r + 0x14);
    command(&mut e, &[params, DATA, 0, 0, 0, 0, RESULT, RESULT + 8]);
    assert_eq!(e.mem.f64(RESULT), 1.0);
    assert_eq!(e.ext::<Values>().weight_calls, 1);
}
