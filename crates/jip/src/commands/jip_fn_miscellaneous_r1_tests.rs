//! `ToggleHitEffects` and `ToggleBipedSlotVisibility`, called by their opcodes. Expectations from
//! JIP's command bodies (functions_jip/jip_fn_miscellaneous.h) and its state's first values
//! (`internal/patches_game.h`: `s_disableHitEffects = 0`; `internal/hooks.h`:
//! `s_bipedSlotVisibility = 0x7C`), never from the port's output.

use super::*;
use crate::commands::test_support::*;
use crate::game_patches::gp;

const HIT_EFFECTS: u32 = 0x28e9;
const BIPED: u32 = 0x28c4;

#[test]
fn hit_effects_answer_on_while_the_word_is_zero() {
    let mut e = engine();
    assert_eq!(gp::disable_hit_effects(&mut e), 0, "JIP's start");
    assert_eq!(call_on(&mut e, HIT_EFFECTS, 0, &[]), (1, 1.0));
    assert_eq!(gp::disable_hit_effects(&mut e), 0, "no argument: no change");
    // Off: bit 1 set; the answer is the state before.
    assert_eq!(call_on(&mut e, HIT_EFFECTS, 0, &[n(0)]), (1, 1.0));
    assert_eq!(gp::disable_hit_effects(&mut e), 1);
    assert_eq!(call_on(&mut e, HIT_EFFECTS, 0, &[]), (1, 0.0));
    // On (any value but 0): only immortal mode's bit 2 kept.
    assert_eq!(call_on(&mut e, HIT_EFFECTS, 0, &[n(5)]), (1, 0.0));
    assert_eq!(gp::disable_hit_effects(&mut e), 0);
    assert_eq!(call_on(&mut e, HIT_EFFECTS, 0, &[]), (1, 1.0));
}

#[test]
fn hit_effects_keep_immortal_modes_bit() {
    let mut e = engine();
    // Immortal mode 2 sets bit 2 (`ToggleImmortalMode 2`).
    call_on(&mut e, 0x28ce, 0, &[n(2)]);
    assert_eq!(gp::disable_hit_effects(&mut e), 2);
    assert_eq!(
        call_on(&mut e, HIT_EFFECTS, 0, &[]),
        (1, 0.0),
        "off while the word is not 0"
    );
    call_on(&mut e, HIT_EFFECTS, 0, &[n(0)]);
    assert_eq!(gp::disable_hit_effects(&mut e), 3);
    call_on(&mut e, HIT_EFFECTS, 0, &[n(1)]);
    assert_eq!(gp::disable_hit_effects(&mut e), 2, "on keeps bit 2");
    assert_eq!(call_on(&mut e, HIT_EFFECTS, 0, &[]), (1, 0.0));
}

#[test]
fn biped_slots_start_at_0x7c() {
    let mut e = engine();
    assert_eq!(biped_slot_visibility(&mut e), 0x7c);
    for slot in 0..20 {
        let on = (2..=6).contains(&slot);
        assert_eq!(
            call_on(&mut e, BIPED, 0, &[n(slot)]),
            (1, if on { 1.0 } else { 0.0 }),
            "slot {slot}"
        );
    }
    assert_eq!(
        biped_slot_visibility(&mut e),
        0x7c,
        "one argument only answers"
    );
}

#[test]
fn biped_slots_set_and_cleared_by_a_second_argument() {
    let mut e = engine();
    assert_eq!(
        call_on(&mut e, BIPED, 0, &[n(0), n(1)]),
        (1, 1.0),
        "the state after"
    );
    assert_eq!(biped_slot_visibility(&mut e), 0x7d);
    assert_eq!(call_on(&mut e, BIPED, 0, &[n(19), n(-2)]), (1, 1.0));
    assert_eq!(biped_slot_visibility(&mut e), 0x8_007d);
    assert_eq!(call_on(&mut e, BIPED, 0, &[n(4), n(0)]), (1, 0.0));
    assert_eq!(biped_slot_visibility(&mut e), 0x8_006d);
    assert_eq!(
        call_on(&mut e, BIPED, 0, &[n(4), n(0)]),
        (1, 0.0),
        "already clear"
    );
    assert_eq!(biped_slot_visibility(&mut e), 0x8_006d);
    // A slot past 19: nothing, no answer.
    assert_eq!(call_on(&mut e, BIPED, 0, &[n(20), n(1)]), (1, 0.0));
    assert_eq!(
        call_on(&mut e, BIPED, 0, &[n(-1), n(1)]),
        (1, 0.0),
        "unsigned: past 19"
    );
    assert_eq!(biped_slot_visibility(&mut e), 0x8_006d);
    // A line with no argument data (never compiled for a required parameter): JIP reads its
    // uninitialised slot word; the port reads slot 0 and only answers it.
    assert_eq!(call_on(&mut e, BIPED, 0, &[]), (1, 1.0));
    assert_eq!(biped_slot_visibility(&mut e), 0x8_006d);
}
