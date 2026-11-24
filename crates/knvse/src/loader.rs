//! kNVSE24 4640f0f commands_animation.cpp: LoadAnimation,
//! LoadCustomAnimation, FindOrLoadAnim and HandleOnAnimDataDelete.
//! The original AddAnimation handles real KF cloning, references, binding and
//! root motion. Its private map selection is scoped in native thread context;
//! Animation+DC is never temporarily published to the other game threads.
use engine_core::{
    animation_maps,
    extend::{Owner, SiteSlot},
    prelude::*,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};
const DELETE: SiteSlot = SiteSlot::new(Owner::Knvse, 0x0048_fb82);
const SEQUENCE_DELETE: SiteSlot = SiteSlot::new(Owner::Knvse, 0x004e_eb4b);
const MODEL_LOADER: u32 = 0x011c_3b3c;
const LOAD_KF: u32 = 0x0044_71c0;
const MAP_GET: u32 = 0x0049_c390;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct Loaded {
    pub sequence: u32,
    pub base: u32,
}
#[derive(Default)]
struct State {
    maps: HashMap<u32, u32>,
    // Source std::map determines teardown order by path, then Animation.
    cached: BTreeMap<(Vec<u8>, u32), Loaded>,
}
#[derive(Default)]
struct Shared {
    state: Mutex<State>,
}
#[derive(Default)]
struct Loader(Arc<Shared>);

pub(crate) fn install(e: &mut Engine) {
    e.ext_free::<Loader>();
    e.on_site(DELETE, |e, a| {
        release(e, a[0]);
        None
    });
    e.on_site(SEQUENCE_DELETE, |e, a| {
        sequence_destroyed(e, a[0]);
        None
    });
}
fn shared(e: &Engine) -> Arc<Shared> {
    Arc::clone(
        &e.ext_get::<Loader>()
            .expect("kNVSE loader installed at startup")
            .0,
    )
}
pub(crate) fn with_path<T>(
    e: &mut Engine,
    path: &[u8],
    f: impl FnOnce(&mut Engine, u32) -> T,
) -> T {
    e.with_stack(path.len() as u32 + 1, |e, text| {
        e.mem.write(text.addr(), path);
        e.mem.set_u8(text.addr() + path.len() as u32, 0);
        f(e, text.addr())
    })
}
/// NiTPointerMap_t::Lookup from upstream NiTypes.h: preserve the actual map's
/// virtual hash/equality and character rules, rather than scanning sequences.
pub(crate) fn find(e: &mut Engine, manager: u32, path: &[u8]) -> u32 {
    with_path(e, path, |e, key| {
        let map = manager + 0x50;
        let bucket = e.vcall(map, 4, &args![key]).u32();
        let mut item = e.mem.u32(e.mem.u32(map + 8) + bucket * 4);
        while item != 0 {
            if e.vcall(map, 8, &args![e.mem.u32(item + 4), key]).bool() {
                return e.mem.u32(item + 8);
            }
            item = e.mem.u32(item);
        }
        0
    })
}
fn map_get(e: &mut Engine, map: u32, group: u16) -> u32 {
    e.with_stack(4, |e, out| {
        e.mem.set_u32(out.addr(), 0);
        if e.call(MAP_GET, &args![map, group, out]).bool() {
            e.mem.u32(out.addr())
        } else {
            0
        }
    })
}
pub(crate) fn kf(e: &mut Engine, path: &[u8]) -> u32 {
    let loader = e.mem.u32(MODEL_LOADER);
    with_path(e, path, |e, text| {
        e.call(LOAD_KF, &args![loader, text]).u32()
    })
}
pub(crate) fn find_or_load(e: &mut Engine, animation: u32, path: &[u8]) -> u32 {
    let manager = e.mem.u32(animation + 0xd8);
    let found = find(e, manager, path);
    if found != 0 {
        found
    } else {
        load(e, animation, path).sequence
    }
}
fn custom_map(e: &mut Engine, animation: u32, state: &Shared) -> u32 {
    if let Some(map) = state.state.lock().unwrap().maps.get(&animation).copied() {
        return map;
    }
    let map = e.call(0x0040_1000, &args![0x10u32]).u32();
    let map = e.call(0x0049_c050, &args![map, 0x65u32]).u32();
    state.state.lock().unwrap().maps.insert(animation, map);
    map
}
pub(crate) fn load(e: &mut Engine, animation: u32, path: &[u8]) -> Loaded {
    let state = shared(e);
    let map = custom_map(e, animation, &state);
    animation_maps::with_private_map(e, animation, map, |e| {
        load_in_map(e, animation, map, path, &state)
    })
}
fn load_in_map(e: &mut Engine, animation: u32, map: u32, path: &[u8], state: &Shared) -> Loaded {
    let key = (path.to_vec(), animation); // source std::string cache is case-sensitive
    if let Some(found) = state.state.lock().unwrap().cached.get(&key).copied() {
        return found;
    }
    let model = kf(e, path);
    if model == 0 {
        return Loaded::default();
    }
    let group = e.mem.u32(model + 8);
    if group == 0 || animation == 0 {
        return Loaded::default();
    }
    let group_id = e.mem.u16(group + 0x10);
    let old_base = map_get(e, map, group_id);
    if old_base != 0 {
        let old_sequence = e.vcall(old_base, 0x10, &args![u32::MAX]).u32();
        if old_sequence != 0
            && nvse::table::guest_text(e, e.mem.u32(old_sequence + 8)).eq_ignore_ascii_case(path)
        {
            return Loaded {
                sequence: old_sequence,
                base: old_base,
            };
        }
        // Keep the unlinked base alive in cached: its sequence may still blend.
        e.call(0x0049_c250, &args![map, group_id]);
    }
    if !e.call(0x0049_0500, &args![animation, model, 0u32]).bool() {
        return Loaded::default();
    }
    let base = map_get(e, map, group_id);
    if base == 0 {
        return Loaded::default();
    }
    let sequence = e.vcall(base, 0x10, &args![u32::MAX]).u32();
    if sequence == 0 {
        return Loaded::default();
    }
    let loaded = Loaded { sequence, base };
    // BSAnimationContext is raw pointers in24: no extra NiPointer reference.
    state.state.lock().unwrap().cached.insert(key, loaded);
    loaded
}
pub(crate) fn cached(e: &Engine, animation: u32, path: &[u8]) -> Option<Loaded> {
    shared(e)
        .state
        .lock()
        .unwrap()
        .cached
        .get(&(path.to_vec(), animation))
        .copied()
}
pub(crate) fn sequence_destroyed(e: &mut Engine, sequence: u32) {
    super::timing24::on_sequence_destroy(e, sequence);
    super::overrides::sequence_destroyed(e, sequence);
    shared(e)
        .state
        .lock()
        .unwrap()
        .cached
        .retain(|_, loaded| loaded.sequence != sequence);
}
pub(crate) fn release(e: &mut Engine, animation: u32) {
    let state = shared(e);
    loop {
        let next = state
            .state
            .lock()
            .unwrap()
            .cached
            .iter()
            .find(|((_, a), _)| *a == animation)
            .map(|(_, &value)| value);
        let Some(next) = next else {
            break;
        };
        // A base may destroy its sequence, whose native hook re-enters the
        // cache: no state lock is held across the game call.
        e.vcall(next.base, 0, &args![1u32]);
        sequence_destroyed(e, next.sequence);
    }
    let map = state.state.lock().unwrap().maps.remove(&animation);
    if let Some(map) = map {
        e.call(0x0049_c080, &args![map, 1u32]);
    }
}
#[cfg(test)]
pub(crate) mod tests;
