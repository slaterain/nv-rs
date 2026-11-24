//! Selected override text-key behavior from official kNVSE24 initial commit
//! 4640f0f0a8636669126408eecf866ea0e263cf24: commands_animation.cpp353–565,
//! AnimTime::~AnimTime287; main.cpp52–213,583–598,657–686; game_types.cpp55–78.
//! Only respectEndKey/respectTextKeys and noBlend are supported. Other custom
//! keys are diagnosed before an override is played, never accepted as no-ops.
//! Native references keep snapshot sequences and saved key arrays alive; native
//! calls never run while the shared state mutex is held.
use engine_core::prelude::*;
use nvse::commands::{dynamic_cast, rtti};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
const PLAYER: u32 = 0x011d_ea3c;
const ADD_REF: u32 = 0x0040_f6e0;
const RELEASE: u32 = 0x0040_1970;
const CUSTOM: &[&[u8]] = &[
    b"noBlend",
    b"respectEndKey",
    b"Script:",
    b"interruptLoop",
    b"burstFire",
    b"respectTextKeys",
    b"SoundPath:",
    b"blendToReloadLoop",
    b"scriptLine:",
    b"replaceWithGroup:",
    b"allowAttack",
];
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Pov {
    #[default]
    NotSet,
    First,
    Third,
}
#[derive(Clone)]
struct Track {
    id: u64,
    sequence: u32,
    actor_id: u32,
    first: bool,
    respect: bool,
    counterpart: u32,
    pov: Pov,
    weapon: u32,
}
#[derive(Clone, Copy)]
struct Saved {
    count: u32,
    keys: u32,
    group: u32,
}
#[derive(Default)]
struct State {
    active: BTreeMap<u32, Track>,
    retired: Vec<Track>,
    snapshots: usize,
    next_id: u64,
    saved: BTreeMap<Vec<u8>, Saved>,
    holster: u32,
}
#[derive(Default)]
struct Shared(Arc<Mutex<State>>);
fn shared(e: &mut Engine) -> Arc<Mutex<State>> {
    Arc::clone(&e.ext_free::<Shared>().0)
}
pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Shared>();
}
fn starts(text: &[u8], prefix: &[u8]) -> bool {
    text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix)
}
/// Source KeyStringCrashFixHook uses prefixes (including flags without ':').
pub(crate) fn is_custom_key(text: &[u8]) -> bool {
    CUSTOM.iter().any(|prefix| starts(text, prefix))
}
fn text_keys(e: &Engine, seq: u32) -> Vec<Vec<u8>> {
    let extra = e.mem.u32(seq + 0x20);
    if extra == 0 {
        return Vec::new();
    }
    let count = e.mem.u32(extra + 0xc);
    let array = e.mem.u32(extra + 0x10);
    (0..count)
        .map(|i| {
            let text = e.mem.u32(array + i * 8 + 4);
            if text == 0 {
                Vec::new()
            } else {
                e.mem.cstr(text)
            }
        })
        .collect()
}
pub(crate) fn validate(e: &Engine, seq: u32) -> Result<(), String> {
    for text in text_keys(e, seq) {
        if is_custom_key(&text)
            && ![b"respectEndKey".as_slice(), b"respectTextKeys", b"noBlend"]
                .iter()
                .any(|key| text.eq_ignore_ascii_case(key))
        {
            return Err(format!(
                "kNVSE24 override key is not implemented: {}",
                String::from_utf8_lossy(&text)
            ));
        }
    }
    Ok(())
}
fn player(e: &Engine) -> u32 {
    e.mem.u32(PLAYER)
}
fn third(e: &mut Engine, actor: u32) -> u32 {
    let process = e.mem.u32(actor + 0x68);
    if process == 0 {
        0
    } else {
        e.vcall(process, 0x1b8, &[]).u32()
    }
}
fn weapon(e: &mut Engine, actor: u32) -> u32 {
    let process = e.mem.u32(actor + 0x68);
    let info = e.vcall(process, 0x148, &[]).u32();
    if info == 0 {
        0
    } else {
        e.mem.u32(info + 8)
    }
}
fn name(e: &Engine, seq: u32) -> Vec<u8> {
    let text = e.mem.u32(seq + 8);
    if text == 0 {
        Vec::new()
    } else {
        e.mem.cstr(text)
    }
}
fn group(e: &Engine, seq: u32) -> u32 {
    e.mem.u32(seq + 0x74)
}
fn slot(e: &Engine, group: u32) -> u32 {
    let base = u32::from(e.mem.u8(group + 0x10));
    e.mem.u32(0x0119_77d8 + base * 0x24 + 8)
}
fn sequence(e: &Engine, animation: u32, slot: u32) -> u32 {
    e.mem.u32(animation + 0xe0 + slot * 4)
}
/// Source GetAnimTime uses Animation time+sequence offset (or remaining
/// transition time), not the version40 sequence-last-scaled clock.
fn time(e: &Engine, animation: u32, seq: u32) -> f32 {
    if e.mem.u32(seq + 0x44) == 5 {
        e.mem.f32(seq + 0x50) - e.mem.f32(animation + 0xd0)
    } else {
        e.mem.f32(seq + 0x48) + e.mem.f32(animation + 0xd0)
    }
}
fn update_track(e: &mut Engine, id: u64, f: impl FnOnce(&mut Track)) {
    let shared = shared(e);
    let mut state = shared.lock().unwrap();
    if let Some(track) = state.active.values_mut().find(|t| t.id == id) {
        f(track);
        return;
    }
    if let Some(track) = state.retired.iter_mut().find(|t| t.id == id) {
        f(track);
    }
}
/// Called only for the group-override route; initial24 ActivateAnim does not
/// call HandleExtraOperations. true is the original `applied` result.
pub(crate) fn on_override_play(e: &mut Engine, animation: u32, seq: u32) -> Result<bool, String> {
    if seq == 0 || group(e, seq) == 0 {
        return Ok(false);
    }
    validate(e, seq)?;
    let keys = text_keys(e, seq);
    let first = animation == e.mem.u32(player(e) + 0x690);
    let respect = first
        && keys.iter().any(|s| {
            s.eq_ignore_ascii_case(b"respectEndKey") || s.eq_ignore_ascii_case(b"respectTextKeys")
        });
    let no_blend = keys.iter().any(|s| s.eq_ignore_ascii_case(b"noBlend"));
    if no_blend {
        e.mem.set_u8(animation + 0x120, 1);
    }
    if !(respect || no_blend) {
        return Ok(false);
    }
    let actor = e.mem.u32(animation + 4);
    let actor_id = e.mem.u32(actor + 0xc);
    let equipped = if respect { weapon(e, actor) } else { 0 };
    // Keep a candidate reference across native calls and publication. Existing
    // entries keep their own reference; the unused candidate is released below.
    e.call(ADD_REF, &args![seq]);
    let inserted = {
        let shared = shared(e);
        let mut state = shared.lock().unwrap();
        let inserted = !state.active.contains_key(&seq);
        if inserted {
            state.next_id = state.next_id.wrapping_add(1);
            let id = state.next_id;
            state.active.insert(
                seq,
                Track {
                    id,
                    sequence: seq,
                    actor_id,
                    first,
                    respect: false,
                    counterpart: 0,
                    pov: Pov::NotSet,
                    weapon: 0,
                },
            );
        }
        let track = state.active.get_mut(&seq).unwrap();
        track.actor_id = actor_id;
        track.first = first;
        if respect {
            track.respect = true;
            track.pov = Pov::NotSet;
            track.weapon = equipped;
        }
        inserted
    };
    if !inserted {
        e.call(RELEASE, &args![seq]);
    }
    Ok(true)
}
fn save_original(e: &mut Engine, seq: u32) {
    let name = name(e, seq);
    let group = group(e, seq);
    if shared(e).lock().unwrap().saved.contains_key(&name) {
        return;
    }
    let saved = Saved {
        count: e.mem.u32(group + 0x14),
        keys: e.mem.u32(group + 0x18),
        group,
    };
    e.call(ADD_REF, &args![group]);
    let inserted = {
        let shared = shared(e);
        let mut state = shared.lock().unwrap();
        if let std::collections::btree_map::Entry::Vacant(entry) = state.saved.entry(name) {
            entry.insert(saved);
            true
        } else {
            false
        }
    };
    if !inserted {
        e.call(RELEASE, &args![group]);
    }
}
fn revert(e: &mut Engine, track: &mut Track) {
    if track.counterpart == 0 {
        return;
    }
    let original = {
        shared(e)
            .lock()
            .unwrap()
            .saved
            .get(&name(e, track.counterpart))
            .copied()
    };
    if let Some(saved) = original {
        let group = group(e, track.counterpart);
        e.mem.set_u32(group + 0x14, saved.count);
        e.mem.set_u32(group + 0x18, saved.keys);
    }
    if group(e, track.sequence) != 0 && e.mem.u8(group(e, track.sequence) + 0x10) == 0x19 {
        let set = {
            let shared = shared(e);
            let mut state = shared.lock().unwrap();
            if state.holster == 0 {
                state.holster = track.counterpart;
                true
            } else {
                false
            }
        };
        if set {
            e.call(ADD_REF, &args![track.counterpart]);
        }
    }
    track.pov = Pov::Third;
    update_track(e, track.id, |t| t.pov = Pov::Third);
}
fn cleanup(e: &mut Engine, mut track: Track) {
    revert(e, &mut track);
    if track.counterpart != 0 {
        e.call(RELEASE, &args![track.counterpart]);
    }
    e.call(RELEASE, &args![track.sequence]);
}
fn remove(e: &mut Engine, seq: u32) {
    let gone = {
        let shared = shared(e);
        let mut state = shared.lock().unwrap();
        let gone = state.active.remove(&seq);
        if state.snapshots != 0 {
            if let Some(track) = gone {
                state.retired.push(track);
            }
            None
        } else {
            gone
        }
    };
    if let Some(track) = gone {
        cleanup(e, track);
    }
}
pub(crate) fn on_sequence_destroy(e: &mut Engine, seq: u32) {
    remove(e, seq);
}
pub(crate) fn post_load(e: &mut Engine) {
    let gone = {
        let shared = shared(e);
        let mut state = shared.lock().unwrap();
        let tracks = std::mem::take(&mut state.active)
            .into_values()
            .collect::<Vec<_>>();
        if state.snapshots != 0 {
            state.retired.extend(tracks);
            Vec::new()
        } else {
            tracks
        }
    };
    for track in gone {
        cleanup(e, track);
    }
}
fn counterpart(e: &mut Engine, first: u32, third_animation: u32, sequence_slot: u32) -> u32 {
    let current = sequence(e, third_animation, sequence_slot);
    let wanted = e.mem.u16(group(e, first) + 0x10);
    if current != 0 && group(e, current) != 0 && e.mem.u8(group(e, current) + 0x10) == wanted as u8
    {
        return current;
    }
    let player = player(e);
    let third = third(e, player);
    let map = e.mem.u32(third + 0xdc);
    let base = e.with_stack(4, |e, out| {
        e.mem.set_u32(out.addr(), 0);
        if e.call(0x0049_c390, &args![map, u32::from(wanted), out])
            .bool()
        {
            e.mem.u32(out.addr())
        } else {
            0
        }
    });
    if base == 0 {
        0
    } else {
        e.vcall(base, 0x10, &args![0i32]).u32()
    }
}
/// main24::HandleAnimTimes' shouldErase predicate, shared with polled groups.
pub(crate) fn should_erase(e: &mut Engine, actor: u32) -> bool {
    actor == 0
        || e.vcall(actor, 0x22c, &args![true]).bool()
        || e.mem.u32(actor + 8) & 0x20 != 0
        || e.mem.u32(actor + 0x68) == 0
}
fn apply(e: &mut Engine, mut track: Track) {
    let seq = track.sequence;
    let own_group = group(e, seq);
    if own_group == 0 {
        remove(e, seq);
        return;
    }
    let actor = e.call(0x0048_39c0, &args![track.actor_id]).u32();
    let actor = dynamic_cast(e, actor, rtti::TES_FORM, rtti::ACTOR);
    if actor == 0 {
        remove(e, seq);
        return;
    }
    let sequence_slot = slot(e, own_group);
    let animation = if track.first {
        e.mem.u32(player(e) + 0x690)
    } else {
        third(e, actor)
    };
    if should_erase(e, actor) || animation == 0 {
        remove(e, seq);
        return;
    }
    if !track.respect
        && (e.mem.u32(seq + 0x44) == 0 || sequence(e, animation, sequence_slot) != seq)
    {
        remove(e, seq);
        return;
    }
    let _time = time(e, animation, seq); // Timed actions are not among this supported key slice.
    if !track.respect {
        return;
    }
    let player_actor = player(e);
    let third_animation = third(e, player_actor);
    let current = sequence(e, third_animation, sequence_slot);
    let current_weapon = weapon(e, actor);
    let changed = current_weapon != track.weapon
        || current == 0
        || group(e, current) == 0
        || (track.counterpart != 0
            && group(e, track.counterpart) != 0
            && e.mem.u16(group(e, current) + 0x10)
                != e.mem.u16(group(e, track.counterpart) + 0x10)
            && e.mem.u32(seq + 0x44) == 0);
    if changed {
        remove(e, seq);
        return;
    }
    if track.pov == Pov::NotSet {
        let player_actor = player(e);
        let third_animation = third(e, player_actor);
        let other = counterpart(e, seq, third_animation, sequence_slot);
        if other == 0 || group(e, other) == 0 {
            remove(e, seq);
            return;
        }
        if track.counterpart != other {
            e.call(ADD_REF, &args![other]);
            let old = track.counterpart;
            track.counterpart = other;
            update_track(e, track.id, |t| t.counterpart = other);
            if old != 0 {
                e.call(RELEASE, &args![old]);
            }
        }
        save_original(e, other);
    }
    if track.counterpart == 0 {
        remove(e, seq);
        return;
    }
    let third_person = e.mem.u8(player(e) + 0x64c) != 0;
    if third_person && track.pov != Pov::Third {
        revert(e, &mut track);
    } else if !third_person && track.pov != Pov::First {
        let other_group = group(e, track.counterpart);
        e.mem
            .set_u32(other_group + 0x14, e.mem.u32(own_group + 0x14));
        e.mem
            .set_u32(other_group + 0x18, e.mem.u32(own_group + 0x18));
        update_track(e, track.id, |t| t.pov = Pov::First);
    }
}
fn finish(e: &mut Engine) {
    let gone = {
        let shared = shared(e);
        let mut state = shared.lock().unwrap();
        state.snapshots -= 1;
        if state.snapshots == 0 {
            std::mem::take(&mut state.retired)
        } else {
            Vec::new()
        }
    };
    for track in gone {
        cleanup(e, track);
    }
}
/// Caller reproduces source MessageHandler's menu guard. Run before the
/// override registry's polled groups, as HandleAnimTimes does in version24.
pub(crate) fn tick(e: &mut Engine) {
    let snapshot = {
        let shared = shared(e);
        let mut state = shared.lock().unwrap();
        state.snapshots += 1;
        state.active.values().cloned().collect::<Vec<_>>()
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for track in snapshot {
            apply(e, track);
        }
    }));
    finish(e);
    if let Err(stop) = result {
        std::panic::resume_unwind(stop);
    }
}
/// Source HandleMisc's follow-up for a respectEndKey unequip reverted in
/// third person. Run after the registry's polled groups and queued work.
pub(crate) fn misc(e: &mut Engine) {
    let seq = shared(e).lock().unwrap().holster;
    if seq == 0 || e.mem.u8(player(e) + 0x64c) == 0 {
        return;
    }
    let actor = player(e);
    let animation = third(e, actor);
    let current = sequence(e, animation, 4);
    let refresh = current == 0 && e.mem.u32(seq + 0x44) == 0;
    if refresh || (current != 0 && e.mem.u8(group(e, current) + 0x10) != 0x19) {
        shared(e).lock().unwrap().holster = 0;
        if refresh {
            let process = e.mem.u32(actor + 0x68);
            let names = e.mem.u32(actor + 0x1b4);
            let animation = third(e, actor);
            e.call(0x0092_31d0, &args![process, false, names, animation, actor]);
        }
        e.call(RELEASE, &args![seq]);
    }
}
/// The host is shutting down this engine: release the port's ownership of
/// original key arrays and a pending holster, after reverting active plays.
pub(crate) fn shutdown(e: &mut Engine) {
    post_load(e);
    let (groups, holster) = {
        let shared = shared(e);
        let mut state = shared.lock().unwrap();
        let saved = std::mem::take(&mut state.saved);
        let holster = std::mem::take(&mut state.holster);
        (
            saved.into_values().map(|s| s.group).collect::<Vec<_>>(),
            holster,
        )
    };
    for group in groups {
        e.call(RELEASE, &args![group]);
    }
    if holster != 0 {
        e.call(RELEASE, &args![holster]);
    }
}
#[cfg(test)]
mod tests;
