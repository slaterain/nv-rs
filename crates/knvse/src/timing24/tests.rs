use super::*;
const DATA: u32 = 0x0300_0000;
const ACTOR: u32 = DATA + 0x1000;
const PROCESS: u32 = DATA + 0x2000;
const FIRST: u32 = DATA + 0x3000;
const THIRD: u32 = DATA + 0x4000;
const SEQ: u32 = DATA + 0x5000;
const OTHER: u32 = DATA + 0x5100;
const G1: u32 = DATA + 0x6000;
const G3: u32 = DATA + 0x6100;
const ACTOR_VT: u32 = DATA + 0x7000;
const PROCESS_VT: u32 = DATA + 0x7400;
const WEAPON_INFO: u32 = DATA + 0x8000;
const MAP_BASE: u32 = DATA + 0x8200;
const MAP_VT: u32 = DATA + 0x8300;
const EXTRA: u32 = DATA + 0x8500;
const GET_THIRD: u32 = DATA + 0x9000;
const GET_WEAPON: u32 = DATA + 0x9010;
const DYING: u32 = DATA + 0x9020;
const GET_SEQUENCE: u32 = DATA + 0x9030;
#[derive(Default)]
struct Seen {
    refs: BTreeMap<u32, i32>,
    ref_calls: Vec<(u32, u32)>,
    weapon: u32,
    dead: bool,
    fallback: bool,
    holsters: Vec<Vec<u32>>,
    retire_on_dying: bool,
}
fn keys(e: &mut Engine, seq: u32, texts: &[&str]) {
    let extra = e.mem.alloc(0x14);
    let array = e.mem.alloc(texts.len() as u32 * 8);
    e.mem.set_u32(seq + 0x20, extra);
    e.mem.set_u32(extra + 0xc, texts.len() as u32);
    e.mem.set_u32(extra + 0x10, array);
    for (i, text) in texts.iter().enumerate() {
        let ptr = e.mem.alloc(text.len() as u32 + 1);
        e.mem.set_cstr(ptr, text.as_bytes());
        e.mem.set_f32(array + i as u32 * 8, i as f32);
        e.mem.set_u32(array + i as u32 * 8 + 4, ptr);
    }
}
fn name(e: &mut Engine, seq: u32, text: &str) {
    let ptr = e.mem.alloc(text.len() as u32 + 1);
    e.mem.set_cstr(ptr, text.as_bytes());
    e.mem.set_u32(seq + 8, ptr);
}
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(DATA, 0x20000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x0119_7000, 0x3000);
    install(&mut e);
    e.mem.set_u32(PLAYER, ACTOR);
    e.mem.set_u32(ACTOR, ACTOR_VT);
    e.mem.set_u32(ACTOR + 0xc, 0x14);
    e.mem.set_u32(ACTOR + 0x68, PROCESS);
    e.mem.set_u32(ACTOR + 0x690, FIRST);
    e.mem.set_u32(ACTOR + 0x1b4, EXTRA);
    e.mem.set_u32(ACTOR_VT + 0x22c, DYING);
    e.mem.set_u32(PROCESS, PROCESS_VT);
    e.mem.set_u32(PROCESS_VT + 0x148, GET_WEAPON);
    e.mem.set_u32(PROCESS_VT + 0x1b8, GET_THIRD);
    e.mem.set_u32(FIRST + 4, ACTOR);
    e.mem.set_u32(THIRD + 4, ACTOR);
    e.mem.set_u32(FIRST + 0xf0, SEQ);
    e.mem.set_u32(THIRD + 0xf0, OTHER);
    e.mem.set_u32(THIRD + 0xdc, EXTRA);
    e.mem.set_u32(SEQ + 0x74, G1);
    e.mem.set_u32(OTHER + 0x74, G3);
    e.mem.set_u32(SEQ + 0x44, 1);
    e.mem.set_u32(OTHER + 0x44, 1);
    e.mem.set_u16(G1 + 0x10, 17);
    e.mem.set_u16(G3 + 0x10, 17);
    e.mem.set_u32(G1 + 0x14, 7);
    e.mem.set_u32(G1 + 0x18, 0x1234);
    e.mem.set_u32(G3 + 0x14, 3);
    e.mem.set_u32(G3 + 0x18, 0x5678);
    e.mem.set_u32(0x0119_77d8 + 17 * 0x24 + 8, 4);
    name(&mut e, SEQ, "first.kf");
    name(&mut e, OTHER, "third.kf");
    e.ext::<Seen>().weapon = 0xabc;
    e.mem.set_u32(WEAPON_INFO + 8, 0xabc);
    e.register(GET_THIRD, |_, w| {
        assert_eq!(w, [PROCESS]);
        THIRD.into_ret()
    });
    e.register(GET_WEAPON, |e, w| {
        assert_eq!(w, [PROCESS]);
        let equipped = e.ext::<Seen>().weapon;
        e.mem.set_u32(WEAPON_INFO + 8, equipped);
        WEAPON_INFO.into_ret()
    });
    e.register(DYING, |e, w| {
        assert_eq!(w, [ACTOR, 1]);
        if e.ext::<Seen>().retire_on_dying {
            e.ext::<Seen>().retire_on_dying = false;
            on_sequence_destroy(e, SEQ);
        }
        u32::from(e.ext::<Seen>().dead).into_ret()
    });
    e.register(0x0048_39c0, |_, w| {
        assert_eq!(w, [0x14]);
        ACTOR.into_ret()
    });
    e.register(0x00ec_43fb, |_, w| {
        assert_eq!(w, [ACTOR, 0, rtti::TES_FORM, rtti::ACTOR, 0]);
        ACTOR.into_ret()
    });
    e.register(ADD_REF, |e, w| {
        *e.ext::<Seen>().refs.entry(w[0]).or_default() += 1;
        e.ext::<Seen>().ref_calls.push((ADD_REF, w[0]));
        Ret::default()
    });
    e.register(RELEASE, |e, w| {
        let count = e.ext::<Seen>().refs.entry(w[0]).or_default();
        *count -= 1;
        assert!(*count >= 0, "owned reference underflow {:08x}", w[0]);
        e.ext::<Seen>().ref_calls.push((RELEASE, w[0]));
        Ret::default()
    });
    e.register(0x0049_c390, |e, w| {
        assert_eq!(w[0], EXTRA);
        if e.ext::<Seen>().fallback {
            e.mem.set_u32(w[2], MAP_BASE);
            1u32.into_ret()
        } else {
            0u32.into_ret()
        }
    });
    e.mem.set_u32(MAP_BASE, MAP_VT);
    e.mem.set_u32(MAP_VT + 0x10, GET_SEQUENCE);
    e.register(GET_SEQUENCE, |_, w| {
        assert_eq!(w, [MAP_BASE, 0]);
        OTHER.into_ret()
    });
    e.register(0x0092_31d0, |e, w| {
        e.ext::<Seen>().holsters.push(w.to_vec());
        Ret::default()
    });
    e
}
fn count(e: &mut Engine) -> usize {
    shared(e).lock().unwrap().active.len()
}
#[test]
fn unsupported_custom_keys_are_explicit_and_standard_engine_keys_remain_accepted() {
    let mut e = engine();
    for key in [
        "Script: x",
        "scriptLine: AllowAttack",
        "SoundPath: test.wav",
        "interruptLoop",
        "burstFire",
        "allowAttack",
        "blendToReloadLoop",
        "replaceWithGroup: Aim",
        "noBlendSuffix",
    ] {
        keys(&mut e, SEQ, &[key]);
        assert!(validate(&e, SEQ).is_err());
        assert!(on_override_play(&mut e, FIRST, SEQ).is_err());
        assert_eq!(count(&mut e), 0);
        assert_eq!(e.mem.u8(FIRST + 0x120), 0);
    }
    keys(
        &mut e,
        SEQ,
        &[
            "start",
            "Blend:3",
            "Enum: Left",
            "Sound: TNWFootstep1stPersonL",
            "end",
        ],
    );
    assert_eq!(on_override_play(&mut e, FIRST, SEQ), Ok(false));
    assert_eq!(count(&mut e), 0);
    assert!(is_custom_key(b"RESPECTENDKEY"));
    assert!(is_custom_key(b"scriptLine: x"));
    assert!(!is_custom_key(b"Sound: x"));
}
#[test]
fn no_blend_sets_original_byte_tracks_until_section_changes_and_balances_sequence_reference() {
    let mut e = engine();
    keys(&mut e, SEQ, &["NoBlend"]);
    assert_eq!(on_override_play(&mut e, FIRST, SEQ), Ok(true));
    assert_eq!(e.mem.u8(FIRST + 0x120), 1);
    assert_eq!(count(&mut e), 1);
    on_override_play(&mut e, FIRST, SEQ).unwrap();
    assert_eq!(
        e.ext::<Seen>().refs[&SEQ],
        1,
        "repeated play keeps one owning reference"
    );
    tick(&mut e);
    assert_eq!(count(&mut e), 1);
    e.mem.set_u32(FIRST + 0xf0, 0);
    tick(&mut e);
    assert_eq!(count(&mut e), 0);
    assert_eq!(e.ext::<Seen>().refs[&SEQ], 0);
    assert_eq!(
        e.mem.u8(FIRST + 0x120),
        1,
        "original does not clear flag at cleanup"
    );
    keys(&mut e, SEQ, &["respectEndKey"]);
    assert_eq!(
        on_override_play(&mut e, THIRD, SEQ),
        Ok(false),
        "respect applies only to first-person skeleton"
    );
}
#[test]
fn respect_end_swaps_key_arrays_on_pov_changes_and_reverts_on_destroy_and_postload() {
    let mut e = engine();
    keys(&mut e, SEQ, &["respectEndKey"]);
    on_override_play(&mut e, FIRST, SEQ).unwrap();
    tick(&mut e);
    assert_eq!((e.mem.u32(G3 + 0x14), e.mem.u32(G3 + 0x18)), (7, 0x1234));
    assert_eq!(count(&mut e), 1);
    assert_eq!(e.ext::<Seen>().refs[&OTHER], 1);
    assert_eq!(e.ext::<Seen>().refs[&G3], 1);
    e.mem.set_u8(ACTOR + 0x64c, 1);
    tick(&mut e);
    assert_eq!((e.mem.u32(G3 + 0x14), e.mem.u32(G3 + 0x18)), (3, 0x5678));
    e.mem.set_u8(ACTOR + 0x64c, 0);
    tick(&mut e);
    assert_eq!(e.mem.u32(G3 + 0x14), 7);
    // Re-registering resets POV but never saves the already-swapped times as originals.
    on_override_play(&mut e, FIRST, SEQ).unwrap();
    tick(&mut e);
    on_sequence_destroy(&mut e, SEQ);
    assert_eq!((e.mem.u32(G3 + 0x14), e.mem.u32(G3 + 0x18)), (3, 0x5678));
    assert_eq!(e.ext::<Seen>().refs[&SEQ], 0);
    assert_eq!(e.ext::<Seen>().refs[&OTHER], 0);
    on_override_play(&mut e, FIRST, SEQ).unwrap();
    tick(&mut e);
    post_load(&mut e);
    assert_eq!(e.mem.u32(G3 + 0x14), 3);
    assert_eq!(count(&mut e), 0);
    assert_eq!(
        e.ext::<Seen>().refs[&G3],
        1,
        "saved original table persists across load"
    );
    shutdown(&mut e);
    assert!(e.ext::<Seen>().refs.values().all(|&n| n == 0));
}
#[test]
fn inactive_respect_play_waits_for_third_group_change_but_weapon_change_ends_it() {
    let mut e = engine();
    keys(&mut e, SEQ, &["respectTextKeys"]);
    on_override_play(&mut e, FIRST, SEQ).unwrap();
    tick(&mut e);
    e.mem.set_u32(SEQ + 0x44, 0);
    e.mem.set_u32(FIRST + 0xf0, 0);
    tick(&mut e);
    assert_eq!(
        count(&mut e),
        1,
        "respect special lifetime ignores own section and inactive state alone"
    );
    e.mem.set_u32(DATA + 0x5200 + 0x74, DATA + 0x6200);
    e.mem.set_u16(DATA + 0x6200 + 0x10, 18);
    e.mem.set_u32(THIRD + 0xf0, DATA + 0x5200);
    tick(&mut e);
    assert_eq!(count(&mut e), 0);
    assert_eq!(e.mem.u32(G3 + 0x14), 3);
    e.mem.set_u32(SEQ + 0x44, 1);
    e.mem.set_u32(THIRD + 0xf0, OTHER);
    on_override_play(&mut e, FIRST, SEQ).unwrap();
    tick(&mut e);
    e.ext::<Seen>().weapon = 0xdef;
    tick(&mut e);
    assert_eq!(count(&mut e), 0);
    assert_eq!(e.mem.u32(G3 + 0x14), 3);
    shutdown(&mut e);
}
#[test]
fn counterpart_fallback_and_unequip_holster_follow_original_contract() {
    let mut e = engine();
    keys(&mut e, SEQ, &["respectEndKey"]);
    e.mem.set_u16(G1 + 0x10, 0x19);
    e.mem.set_u16(G3 + 0x10, 17);
    e.mem.set_u32(0x0119_77d8 + 0x19 * 0x24 + 8, 4);
    e.ext::<Seen>().fallback = true;
    on_override_play(&mut e, FIRST, SEQ).unwrap();
    tick(&mut e);
    assert_eq!(
        e.mem.u32(G3 + 0x14),
        7,
        "index0 counterpart fallback accepted even different group"
    );
    e.mem.set_u8(ACTOR + 0x64c, 1);
    tick(&mut e);
    assert_eq!(e.mem.u32(G3 + 0x14), 3);
    assert_eq!(shared(&mut e).lock().unwrap().holster, OTHER);
    e.mem.set_u32(THIRD + 0xf0, 0);
    e.mem.set_u32(OTHER + 0x44, 0);
    misc(&mut e);
    assert_eq!(
        e.ext::<Seen>().holsters,
        [vec![PROCESS, 0, EXTRA, THIRD, ACTOR]]
    );
    assert_eq!(shared(&mut e).lock().unwrap().holster, 0);
    shutdown(&mut e);
}
#[test]
fn invalid_actor_cleanup_and_version24_clock_do_not_use_version40_last_scaled_time() {
    let mut e = engine();
    e.mem.set_f32(SEQ + 0x48, 2.0);
    e.mem.set_f32(FIRST + 0xd0, 3.0);
    e.mem.set_f32(SEQ + 0x3c, 90.0);
    assert_eq!(time(&e, FIRST, SEQ), 5.0);
    e.mem.set_u32(SEQ + 0x44, 5);
    e.mem.set_f32(SEQ + 0x50, 8.0);
    assert_eq!(time(&e, FIRST, SEQ), 5.0);
    keys(&mut e, SEQ, &["noBlend"]);
    on_override_play(&mut e, FIRST, SEQ).unwrap();
    e.ext::<Seen>().dead = true;
    tick(&mut e);
    assert_eq!(count(&mut e), 0);
    assert_eq!(e.ext::<Seen>().refs[&SEQ], 0);
}

#[test]
fn native_reentrancy_retains_snapshot_until_real_key_reversion_and_reference_release() {
    let mut e = engine();
    keys(&mut e, SEQ, &["respectEndKey"]);
    on_override_play(&mut e, FIRST, SEQ).unwrap();
    e.ext::<Seen>().retire_on_dying = true;
    tick(&mut e);
    assert_eq!(count(&mut e), 0);
    assert_eq!((e.mem.u32(G3 + 0x14), e.mem.u32(G3 + 0x18)), (3, 0x5678));
    assert_eq!(e.ext::<Seen>().refs[&SEQ], 0);
    assert_eq!(e.ext::<Seen>().refs[&OTHER], 0);
    assert_eq!(shared(&mut e).lock().unwrap().snapshots, 0);
    shutdown(&mut e);
    assert!(e.ext::<Seen>().refs.values().all(|&n| n == 0));
}
