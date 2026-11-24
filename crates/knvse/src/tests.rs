use super::*;

const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const CAST_CALLS: u32 = DATA + 0x2000;
const ANIMATION_CALLS: u32 = CAST_CALLS + 4;
const ACTOR_VTABLE: u32 = DATA + 0x3000;
const GET_ANIMATION: u32 = DATA + 0x4000;
const ANIMATION_FIXTURE: u32 = 0x700;

pub(super) fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(0x0119_7000, 0x3000);
    e.map(0x011d_e000, 0x1000);
    e.map(DATA, 0x10000);
    nvse::table::install(&mut e);
    install(&mut e);
    // External engine calls are test seams, not alternate query logic. The
    // cast checks its exact source/destination RTTI and returns only fixture
    // actors. NPC animation selection must go through the actual virtual call.
    e.register(0x00ec_43fb, |e, w| {
        assert_eq!(w[1..], [0, rtti::TES_OBJECT_REFR, rtti::ACTOR, 0]);
        e.mem.set_u32(CAST_CALLS, e.mem.u32(CAST_CALLS) + 1);
        Ret {
            eax: if e.mem.u32(w[0]) == ACTOR_VTABLE {
                w[0]
            } else {
                0
            },
            ..Ret::default()
        }
    });
    e.register(GET_ANIMATION, |e, w| {
        e.mem
            .set_u32(ANIMATION_CALLS, e.mem.u32(ANIMATION_CALLS) + 1);
        Ret {
            eax: e.mem.u32(w[0] + ANIMATION_FIXTURE),
            ..Ret::default()
        }
    });
    e.mem
        .set_u32(ACTOR_VTABLE + VT_GET_ANIMATION, GET_ANIMATION);
    e
}

fn actor(e: &mut Engine) -> u32 {
    let actor = e.mem.alloc(0x800);
    e.mem.set_u32(actor, ACTOR_VTABLE);
    actor
}

fn playing(e: &mut Engine, group: u16, slot: u32) -> (u32, u32, u32) {
    let animation = e.mem.alloc(0x140);
    let sequence = e.mem.alloc(0x78);
    let anim_group = e.mem.alloc(0x3c);
    e.mem.set_u32(
        GROUP_INFOS + GROUP_INFO_SIZE * u32::from(group) + GROUP_SEQUENCE_TYPE,
        slot,
    );
    e.mem
        .set_u32(animation + CURRENT_SEQUENCES + 4 * slot, sequence);
    e.mem.set_u32(sequence + SEQUENCE_GROUP, anim_group);
    e.mem.set_u32(sequence + SEQUENCE_STATE, 1);
    e.mem.set_u16(anim_group + GROUP_ID, group);
    (animation, sequence, anim_group)
}

fn call(e: &mut Engine, subject: u32, group: Option<u16>) -> f64 {
    let payload = group
        .map(u16::to_le_bytes)
        .map(Vec::from)
        .unwrap_or_default();
    e.mem.set_u16(DATA, IS_ANIM_PLAYING_ALT as u16);
    e.mem.set_u16(DATA + 2, (payload.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, u16::from(group.is_some()));
    e.mem.write(DATA + 6, &payload);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 999.0);
    let record = table::record_of(e, IS_ANIM_PLAYING_ALT);
    let words = [
        e.mem.u32(record + 0x14),
        DATA,
        subject,
        0,
        0,
        0,
        RESULT,
        OFFSET,
    ];
    let out = e.call(e.mem.u32(record + 0x18), &words);
    assert_eq!(out.eax & 0xff, 1);
    e.mem.f64(RESULT)
}

#[test]
fn exact_command_metadata_does_not_claim_a_complete_plugin() {
    let mut e = engine();
    let record = table::record_of(&e, IS_ANIM_PLAYING_ALT);
    assert_eq!(
        table::guest_text(&e, e.mem.u32(record)),
        b"IsAnimPlayingAlt"
    );
    assert!(table::guest_text(&e, e.mem.u32(record + 4)).is_empty());
    assert_eq!(e.mem.u16(record + 0x10), 1); // reference required
    assert_eq!(e.mem.u16(record + 0x12), 1);
    let params = e.mem.u32(record + 0x14);
    assert_eq!(table::guest_text(&e, e.mem.u32(params)), b"anim group");
    assert_eq!(e.mem.u32(params + 4), 0x0a);
    assert_eq!(e.mem.u32(params + 8), 0);
    assert_eq!(
        table::return_type(&e, IS_ANIM_PLAYING_ALT),
        Returns::Default
    );
    assert!(nvse::commands::extender::plugin_info(&e, b"kNVSE").is_none());
    assert_eq!(call(&mut e, 0, Some(0)), 0.0);
    assert_eq!(e.mem.u32(CAST_CALLS), 0);
}

#[test]
fn npc_query_reads_group_info_slot_and_live_sequence_not_saved_group_ids() {
    let mut e = engine();
    let actor = actor(&mut e);
    let (animation, sequence, anim_group) = playing(&mut e, 17, 6);
    e.mem.set_u32(actor + ANIMATION_FIXTURE, animation);
    // Neither slot zero nor Animation::group[] describes the wanted group.
    e.mem.set_u16(animation + 0x4c + 12, 100);
    e.mem.set_u16(anim_group + GROUP_ID, 0x9211);
    assert_eq!(call(&mut e, actor, Some(17)), 1.0);
    assert_eq!(e.mem.u32(ANIMATION_CALLS), 1);
    assert_eq!(e.mem.u16(animation + 0x4c + 12), 100);
    assert_eq!(e.mem.u16(anim_group + GROUP_ID), 0x9211);
    assert_eq!(e.mem.u32(sequence + SEQUENCE_STATE), 1);

    // Ease-out and other nonzero states count too: this is not the different
    // IsAnimSequencePlaying command's inactive-or-easeout exclusion.
    for state in [2, 4, 5, 0xffff_ffff] {
        e.mem.set_u32(sequence + SEQUENCE_STATE, state);
        assert_eq!(call(&mut e, actor, Some(17)), 1.0);
    }
    e.mem.set_u32(sequence + SEQUENCE_STATE, 0);
    assert_eq!(call(&mut e, actor, Some(17)), 0.0);
    e.mem.set_u32(sequence + SEQUENCE_STATE, 1);
    e.mem.set_u16(anim_group + GROUP_ID, 18);
    assert_eq!(call(&mut e, actor, Some(17)), 0.0);
    e.mem.set_u32(sequence + SEQUENCE_GROUP, 0);
    assert_eq!(call(&mut e, actor, Some(17)), 0.0);
    e.mem.set_u32(animation + CURRENT_SEQUENCES + 24, 0);
    assert_eq!(call(&mut e, actor, Some(17)), 0.0);
}

#[test]
fn player_uses_consistent_pov_and_does_not_fall_back_to_the_other_view() {
    let mut e = engine();
    let player = actor(&mut e);
    e.mem.set_u32(PLAYER, player);
    let process = e.mem.alloc(0x200);
    e.mem.set_u32(player + PROCESS, process);
    let (first, _, _) = playing(&mut e, 17, 2);
    let (third, third_sequence, _) = playing(&mut e, 17, 2);
    e.mem.set_u32(player + FIRST_PERSON_ANIMATION, first);
    e.mem.set_u32(process + PROCESS_ANIMATION, third);
    e.mem.set_u32(third_sequence + SEQUENCE_STATE, 0);
    e.mem.set_u8(player + 0x64a, 1); // current camera differs from consistent POV
    assert_eq!(call(&mut e, player, Some(17)), 1.0);
    e.mem.set_u8(player + THIRD_PERSON_CONSISTENT, 1);
    e.mem.set_u8(player + 0x64a, 0);
    assert_eq!(call(&mut e, player, Some(17)), 0.0);
    e.mem.set_u32(third_sequence + SEQUENCE_STATE, 1);
    assert_eq!(call(&mut e, player, Some(17)), 1.0);
    e.mem.set_u8(player + THIRD_PERSON_CONSISTENT, 0);
    e.mem.set_u32(player + FIRST_PERSON_ANIMATION, 0);
    assert_eq!(call(&mut e, player, Some(17)), 0.0);
    assert_eq!(e.mem.u32(ANIMATION_CALLS), 0);
}

#[test]
fn omitted_arguments_cast_missing_animation_and_group_bounds_return_zero() {
    let mut e = engine();
    let actor = actor(&mut e);
    assert_eq!(call(&mut e, actor, None), 0.0);
    // ExtractArgsEx accepts a zero argument count; the command's initialized
    // group remains UINT_MAX. Upstream still casts and obtains animation
    // before rejecting that group, rather than treating this as parse failure.
    assert_eq!(e.mem.u32(CAST_CALLS), 1);
    assert_eq!(call(&mut e, actor, Some(17)), 0.0); // no animation
    let (animation, _, _) = playing(&mut e, 244, 7);
    e.mem.set_u32(actor + ANIMATION_FIXTURE, animation);
    assert_eq!(call(&mut e, actor, None), 0.0);
    assert_eq!(call(&mut e, actor, Some(244)), 1.0);
    for group in [245, 255, 0x111, u16::MAX] {
        assert_eq!(call(&mut e, actor, Some(group)), 0.0);
    }
    e.mem.set_u32(actor, 0); // fixture cast rejects this non-actor
    assert_eq!(call(&mut e, actor, Some(244)), 0.0);
}
