//! Dialogue: JIP LN NVSE's "Store parent TESTopic for every TESTopicInfo" (`InitGamePatches`,
//! `internal/patches_game.h` (JIP LN NVSE): `InitTopicInfoHook`, `LoadTopicInfoHook` and four
//! byte patches; step 3.9a, PLAN-3.9 G25), a fix.
//!
//! JIP makes every `TESTopicInfo` 4 bytes bigger (`SafeWrite8(0x468CE7, 0x54)`, the data handler's
//! allocation) and keeps in the new word (`+0x50`, `parentTopic`) the topic the info's load linked
//! it into: `TESTopicInfo::Load`'s call of `TESTopic::InsertInfo` (`0x61E086`) stores the topic
//! (`ECX`, the loading topic `0x011CBEB0`) first; the constructor clears it (`0x61CD16`);
//! `ClearData` reads it instead of asking `GetOwnerTopic` (`0x61D041`); and `GetOwnerTopic`
//! (`0x61A310`) is rewritten to answer it. So every caller of `GetOwnerTopic` in the game (the
//! dialogue's responses, the topic conditions, `ClearData`) gets the last topic the info was linked
//! into, where the exe walks every topic and answers the first holding the info; the two differ
//! only for an info a plugin links under two topics. JIP's `RunResultScript` hook and its dialogue
//! topic events read the same word.
//!
//! The port keeps the info's size: [`ParentTopics`] maps an info's address to that topic, filled
//! at `0x61E086` and dropped at the constructor (as JIP clears its word there) and in the
//! destructor (`0x61CEEE`, after `ClearData`; JIP's word goes with the object). An info never
//! linked answers 0, as JIP's cleared word does.
//!
//! **With the dialogue topic index** (`NV_TOPIC_INDEX`, host `topic_index.rs`): the index answers
//! the exe's walk fast; this answers JIP's last-linked topic. Both may be on: the index stays
//! registered at `0x61A310` and this wrap goes over it (the host installs its rows before JIP)
//! and never calls it, so with JIP the answer is JIP's; the load seam records and then the
//! translation calls `InsertInfo` by address as before, so the index still notes every insert.

use super::Area;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use std::collections::HashMap;
use std::sync::RwLock;

/// The seams: the constructor's conditions call, the destructor after `ClearData`, `ClearData`'s
/// `GetOwnerTopic` call, the load's `InsertInfo` call.
pub const CONSTRUCTED: SiteSlot = SiteSlot::new(Owner::Jip, 0x0061_cd16);
pub const DESTROYED: SiteSlot = SiteSlot::new(Owner::Jip, 0x0061_ceee);
pub const CLEAR_DATA: SiteSlot = SiteSlot::new(Owner::Jip, 0x0061_d041);
pub const LINKED: SiteSlot = SiteSlot::new(Owner::Jip, 0x0061_e086);
/// `TESTopic::GetOwnerTopic` (cdecl: the info).
pub const GET_OWNER_TOPIC: WrapSlot = WrapSlot::new(Owner::Jip, 0x0061_a310);

/// The area's rows.
pub const AREA: Area = Area {
    name: "dialogue",
    fix_sites: &[
        CONSTRUCTED.addr(),
        DESTROYED.addr(),
        CLEAR_DATA.addr(),
        LINKED.addr(),
    ],
    fix_wraps: &[GET_OWNER_TOPIC.addr()],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

/// The infos' parent topics (JIP's `TESTopicInfo::parentTopic`), by the info's address. Its own
/// small lock, not JIP's state: `GetOwnerTopic` is asked from any game thread, and the record is
/// written only while plugins load.
#[derive(Default)]
pub struct ParentTopics(RwLock<HashMap<u32, u32>>);

fn records(e: &mut Engine) -> &ParentTopics {
    e.ext_free::<ParentTopics>()
}

/// The topic recorded for `info`: the last one its load linked it into, 0 for none (JIP's word).
pub fn parent_topic(e: &mut Engine, info: u32) -> u32 {
    let map = records(e).0.read().unwrap_or_else(|p| p.into_inner());
    map.get(&info).copied().unwrap_or(0)
}

fn drop_record(e: &mut Engine, info: u32) {
    let mut map = records(e).0.write().unwrap_or_else(|p| p.into_inner());
    map.remove(&info);
}

fn install_fixes(e: &mut Engine) {
    // Made before the threads start, so every thread's engine shares it.
    e.ext_free::<ParentTopics>();
    e.on_site(CONSTRUCTED, constructed);
    e.on_site(DESTROYED, destroyed);
    e.on_site(CLEAR_DATA, clear_data);
    e.on_site(LINKED, linked);
    e.wrap(GET_OWNER_TOPIC, get_owner_topic);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitTopicInfoHook.
/// The constructor's conditions call (`0x61CD16`: the info): JIP clears the conditions' two words
/// and its parent topic word in place of the call; here the record goes and the call runs on (it
/// clears the same two words: `00680890` -> `0096a2d0`).
fn constructed(e: &mut Engine, a: &[u32]) -> Option<u32> {
    drop_record(e, a[0]);
    None
}

/// The destructor after `ClearData` (`0x61CEEE`: the info): the record goes with the info.
fn destroyed(e: &mut Engine, a: &[u32]) -> Option<u32> {
    drop_record(e, a[0]);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x61D041, "\x8B\x42\x50...")).
/// `ClearData`'s owner (`0x61D041`: the info): the parent topic, read in place of the call of
/// `GetOwnerTopic` (`mov eax, [edx+0x50]`); 0 leaves no topic to take the info out of.
fn clear_data(e: &mut Engine, a: &[u32]) -> Option<u32> {
    Some(parent_topic(e, a[0]))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: LoadTopicInfoHook.
/// The load's link (`0x61E086`: the loading topic, the info): the topic recorded, then
/// `InsertInfo` called as before, whatever it answers.
fn linked(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (topic, info) = (a[0], a[1]);
    let mut map = records(e).0.write().unwrap_or_else(|p| p.into_inner());
    map.insert(info, topic);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x61A310, "\x8B\x44\x24\x04\x8B\x40\x50\xC3")).
/// `GetOwnerTopic` (cdecl: the info) rewritten: the info's parent topic. JIP reads the word with
/// no null test (a null info stops the game there); the port's record answers 0.
fn get_owner_topic(e: &mut Engine, w: &[u32]) -> Ret {
    Ret {
        eax: parent_topic(e, w[0]),
        ..Ret::default()
    }
}
