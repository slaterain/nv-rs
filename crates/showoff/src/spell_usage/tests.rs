use super::*;
const AREA: u32 = 0x0300_0000;
const ACTOR: u32 = AREA + 0x1000;
const MAGIC: u32 = AREA + 0x2000;
const MONITOR: u32 = AREA + 0x3000;
const DATA: u32 = AREA + 0x4000;
const RESULT: u32 = DATA + 0x800;
const SCRIPT: u32 = AREA + 0x5000;
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x10000);
    e.map(0x0118_e000, 0x9000);
    nvse::table::install(&mut e);
    install(&mut e);
    let vt = e.mem.alloc(0x110);
    let targetvt = e.mem.alloc(0x20);
    e.mem.set_u32(ACTOR, vt);
    e.mem.set_u32(ACTOR + 0x94, targetvt);
    e.mem.set_u32(vt + 0x100, AREA + 0x9000);
    e.mem.set_u32(targetvt + 8, AREA + 0x9004);
    e.register(AREA + 0x9000, |_, _| true.into_ret());
    e.register(AREA + 0x9004, |e, a| {
        assert_eq!(a, [ACTOR + 0x94]);
        e.mem.u32(DATA + 0x100).into_ret()
    });
    e.register(0x0040_6200, |e, a| {
        assert_eq!(a, [MAGIC + 0xc]);
        e.mem.set_u32(DATA + 0x104, e.mem.u32(DATA + 0x104) + 1);
        MONITOR.into_ret()
    });
    e
}
fn effects(e: &mut Engine, items: &[(f32, u32, u32)]) -> Vec<u32> {
    let head = e.mem.alloc(8);
    e.mem.set_u32(DATA + 0x100, head);
    let mut node = head;
    let mut effects = Vec::new();
    for (i, (magnitude, magic, effect)) in items.iter().enumerate() {
        let active = e.mem.alloc(0x48);
        e.mem.set_f32(active + 0x1c, *magnitude);
        e.mem.set_u32(active + 8, *magic);
        e.mem.set_u32(active + 0xc, *effect);
        e.mem.set_u32(node, active);
        let next = if i + 1 == items.len() {
            0
        } else {
            e.mem.alloc(8)
        };
        e.mem.set_u32(node + 4, next);
        node = next;
        effects.push(active);
    }
    effects
}
#[test]
fn real_monitor_and_active_effect_pointer_identity_match_both_fields_including_terminated_effects()
{
    let mut e = setup();
    let effects = effects(
        &mut e,
        &[
            (1.25, MAGIC, MONITOR),
            (9.0, MAGIC + 4, MONITOR),
            (9.0, MAGIC, MONITOR + 4),
            (2.5, MAGIC, MONITOR),
        ],
    );
    e.mem.set_u8(effects[3] + 0x11, 1);
    assert_eq!(usage(&mut e, ACTOR, MAGIC), 3.75);
    assert_eq!(usage(&mut e, 0, MAGIC), 0.0);
    e.register(AREA + 0x9000, |_, _| false.into_ret());
    assert_eq!(usage(&mut e, ACTOR, MAGIC), 0.0);
    assert_eq!(e.mem.u32(DATA + 0x104), 1);
}
#[test]
fn magnitude_sum_rounds_each_f32_step_and_missing_monitor_or_list_yields_zero() {
    let mut e = setup();
    effects(
        &mut e,
        &[
            (16777216.0, MAGIC, MONITOR),
            (1.0, MAGIC, MONITOR),
            (-16777216.0, MAGIC, MONITOR),
        ],
    );
    assert_eq!(usage(&mut e, ACTOR, MAGIC), 0.0);
    e.mem.set_u32(DATA + 0x100, 0);
    assert_eq!(usage(&mut e, ACTOR, MAGIC), 0.0);
    e.register(0x0040_6200, |_, _| 0u32.into_ret());
    assert_eq!(usage(&mut e, ACTOR, MAGIC), 0.0);
}
#[test]
fn real_magic_item_parameter_is_cast_before_usage_and_condition_receives_adjusted_pointer() {
    let mut e = setup();
    let form = MAGIC - 0x30;
    e.mem.set_u8(form + 4, 0x2f);
    e.register(0x00ec_43fb, |_, a| {
        assert_eq!(a, [MAGIC - 0x30, 0, 0x0118_3028, 0x0118_3140, 0]);
        MAGIC.into_ret()
    });
    effects(&mut e, &[(2.25, MAGIC, MONITOR)]);
    let refs = e.mem.alloc(16);
    e.mem.set_u32(refs + 8, form);
    e.mem.set_u32(SCRIPT + 0x44, refs);
    e.mem.set_u16(DATA, OP as u16);
    e.mem.set_u16(DATA + 2, 5);
    e.mem.set_u16(DATA + 4, 1);
    e.mem.write(DATA + 6, &[b'r', 1, 0]);
    e.mem.set_u32(RESULT + 8, 4);
    let r = table::record_of(&e, OP);
    let p = e.mem.u32(r + 0x14);
    assert_eq!(e.mem.u32(p + 4), 0xb);
    assert_eq!(e.mem.u32(p + 8), 0);
    command(&mut e, &[p, DATA, ACTOR, 0, SCRIPT, 0, RESULT, RESULT + 8]);
    assert_eq!(e.mem.f64(RESULT), 2.25);
    condition(&mut e, &[ACTOR, MAGIC, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 2.25);
}
