use super::*;

const DYING_OPCODE: u32 = 0x314c;
const SEEN_OPCODE: u32 = 0x3158;
const PLAYER: u32 = 0x011d_ea3c;
const DYING_SITE: u32 = 0x0089_f49f;
const EXTERIOR_SITE: u32 = 0x0055_678a;
const INTERIOR_SITE: u32 = 0x0055_7053;

fn record_callbacks(e: &mut Engine) {
    e.register(nvse::functions::SCRIPT_EXECUTE, |e, a| {
        assert_eq!(a[1], 0, "callback has no calling reference");
        assert_eq!(&a[3..], &[0, 0]);
        let local = nvse::args::event_list_variable(e, a[2], 1);
        let count = e.mem.u32(STATE);
        let slot = STATE + 0x40 + 8 * count;
        e.mem.set_u32(slot, a[0]);
        e.mem.set_u32(slot + 4, e.mem.u32(local + 8));
        e.mem.set_u32(STATE, count + 1);
        Ret::default()
    });
}

fn recorded(e: &Engine) -> Vec<(u32, u32)> {
    (0..e.mem.u32(STATE))
        .map(|i| {
            let slot = STATE + 0x40 + 8 * i;
            (e.mem.u32(slot), e.mem.u32(slot + 4))
        })
        .collect()
}

fn deliver(e: &mut Engine, site: u32, form: u32) {
    assert_eq!(e.site(site, &[form]), None);
}

#[test]
fn dying_site_checks_state_name_and_reference_filter_and_passes_the_actor() {
    let mut e = setup();
    record_callbacks(&mut e);
    e.mem.set_u32(ACTOR + 8, 1 << 3);
    e.mem.set_u32(ACTOR + 0x20, Q1);
    let name = e.mem.alloc(2);
    e.register_double(0x0055_d520, move |_, a| {
        assert_eq!(a, &[ACTOR]);
        name.into_ret()
    });
    e.register(0x0056_af40, |_, _| false.into_ret());

    command_call(&mut e, DYING_OPCODE, A, 1.0, Some(99.0), Some(Q1));
    command_call(&mut e, DYING_OPCODE, A, 1.0, Some(0.0), Some(Q1));
    command_call(&mut e, DYING_OPCODE, B, 1.0, Some(0.0), Some(Q2));
    e.mem.set_u32(ACTOR + 0x108, 1);
    e.mem.set_u8(name, b'A');
    deliver(&mut e, DYING_SITE, ACTOR);
    assert!(recorded(&e).is_empty(), "registration is pending");
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::MAIN_GAME_LOOP, 0, 0);

    e.mem.set_u32(ACTOR + 0x108, 2);
    deliver(&mut e, DYING_SITE, ACTOR);
    e.mem.set_u32(ACTOR + 0x108, 1);
    e.mem.set_u8(name, 0);
    deliver(&mut e, DYING_SITE, ACTOR);
    assert!(recorded(&e).is_empty());

    e.mem.set_u8(name, b'A');
    deliver(&mut e, DYING_SITE, ACTOR);
    e.mem.set_u8(name, 0);
    e.mem.set_u32(PLAYER, ACTOR);
    deliver(&mut e, DYING_SITE, ACTOR);
    command_call(&mut e, DYING_OPCODE, A, 0.0, Some(0.0), Some(Q1));
    deliver(&mut e, DYING_SITE, ACTOR);
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::MAIN_GAME_LOOP, 0, 0);
    deliver(&mut e, DYING_SITE, ACTOR);
    assert_eq!(recorded(&e), vec![(A, e.mem.u32(ACTOR + 0xc)); 3]);
}

#[test]
fn both_seen_sites_pass_the_cell_and_reset_flushes_only_active_dying_handlers() {
    let mut e = setup();
    record_callbacks(&mut e);
    form(&mut e, Q1, 0x39, 0x0500_7000);
    form(&mut e, Q2, 0x39, 0x0500_7100);
    e.mem.set_u32(PLAYER, ACTOR);
    e.mem.set_u32(ACTOR + 8, 1 << 3);
    e.mem.set_u32(ACTOR + 0x108, 1);
    command_call(&mut e, DYING_OPCODE, A, 1.0, None, None);
    command_call(&mut e, SEEN_OPCODE, A, 1.0, Some(0.0), Some(Q1));
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::MAIN_GAME_LOOP, 0, 0);
    command_call(&mut e, DYING_OPCODE, B, 1.0, None, None);
    command_call(&mut e, DYING_OPCODE, B, 0.0, None, None);
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::PRE_LOAD_GAME, 0, 0);
    deliver(&mut e, DYING_SITE, ACTOR);
    deliver(&mut e, EXTERIOR_SITE, Q1);
    deliver(&mut e, INTERIOR_SITE, Q2);
    let cell_id = e.mem.u32(Q1 + 0xc);
    assert_eq!(recorded(&e), vec![(A, cell_id)]);
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::MAIN_GAME_LOOP, 0, 0);
    deliver(&mut e, DYING_SITE, ACTOR);
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::NEW_GAME, 0, 0);
    deliver(&mut e, DYING_SITE, ACTOR);
    deliver(&mut e, INTERIOR_SITE, Q1);
    assert_eq!(
        recorded(&e),
        vec![(A, cell_id), (B, e.mem.u32(ACTOR + 0xc)), (A, cell_id)]
    );
}

#[test]
fn dying_reference_filter_matches_a_leveled_template_or_original_base() {
    let mut e = setup();
    record_callbacks(&mut e);
    e.mem.set_u32(PLAYER, ACTOR);
    e.mem.set_u32(ACTOR + 8, 1 << 3);
    e.mem.set_u32(ACTOR + 0x20, Q1);
    e.mem.set_u32(ACTOR + 0x108, 1);
    e.register(0x0056_af40, |_, _| true.into_ret());
    e.register(0x0041_0220, |e, a| {
        assert_eq!(a, &[ACTOR + 0x44, 0x2e]);
        e.mem.u32(STATE + 0x100).into_ret()
    });
    command_call(&mut e, DYING_OPCODE, A, 1.0, Some(0.0), Some(Q2));
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::MAIN_GAME_LOOP, 0, 0);
    deliver(&mut e, DYING_SITE, ACTOR);
    assert!(recorded(&e).is_empty(), "leveled extra is absent");

    let extra = e.mem.alloc(0x14);
    e.mem.set_u32(STATE + 0x100, extra);
    e.mem.set_u32(extra + 0x10, Q2);
    deliver(&mut e, DYING_SITE, ACTOR);
    e.mem.set_u32(extra + 0x10, 0);
    e.mem.set_u32(extra + 0xc, Q2);
    deliver(&mut e, DYING_SITE, ACTOR);
    assert_eq!(recorded(&e), vec![(A, e.mem.u32(ACTOR + 0xc)); 2]);
}
