//! Append-only Menu Search editor and the Container/LevelUp/Recipe consumers.
//! Owned Tweaks10.05 10040730..100408DF,10041C00..100428E8.
use super::*;
const N: usize = 5;
const BASE: u32 = KEY + 0x20;
const SINGLETONS: [u32; N] = [
    0x011d_93f8,
    0x011d_9fdc,
    0x011d_8e90,
    0x011d_8fa4,
    0x011d_aac0,
];
const SLOTS: [u32; N] = [
    0x0107_21dc,
    0x0107_3d0c,
    0x0107_04bc,
    0x0107_071c,
    0x0107_6d4c,
];
const SETUP: [u32; N] = [
    0x0075_b46f,
    0x0078_4db8,
    0x0072_72f2,
    0x0072_d3b3,
    0x007c_ba5a,
];
const CLEAN: [u32; N] = [
    0x0075_ba66,
    0x0078_51d3,
    0x0072_6f2a,
    0x0072_d763,
    0x007d_1076,
];
const CONTAINER_FILTERS: [u32; 8] = [
    0x0075_c12b,
    0x0075_c1d9,
    0x0075_c9dc,
    0x0075_ca31,
    0x0075_c3ce,
    0x0075_c514,
    0x0075_c584,
    0x0075_c5a8,
];
const RECIPE_FILTER: u32 = 0x0072_7637;
const MODE_FILTER: u32 = BASE + 0x40;
const FIRST_FILTER: u32 = BASE + 0x44;
const PERK_FILTER: u32 = BASE + 0x48;
const RECIPE_PREDICATE: u32 = BASE + 0x4c;
const BARTER_POINTERS: [u32; 4] = [0x0072_da1d, 0x0072_dacb, 0x0072_e1bf, 0x0072_e208];
const BARTER_FILTER_BASE: u32 = BASE + 0x50;
const SAVE_FILTER: u32 = BASE + 0x60;
const SAVE_NULL_FILTER: u32 = BASE + 0x64;
#[derive(Clone, Default)]
struct Simple {
    enabled: bool,
    tile: u32,
    text: Vec<u8>,
    active: bool,
    hooked: bool,
    original: u32,
    filters: [u32; 4],
}
#[derive(Default)]
struct SimpleState {
    menus: [Simple; N],
    remove: bool,
    clear: bool,
    installed: bool,
    prepared: bool,
}
#[derive(Default)]
struct Other(Mutex<SimpleState>);
fn state(e: &mut Engine, f: impl FnOnce(&mut SimpleState)) {
    f(&mut e.ext_free::<Other>().0.lock().unwrap());
}
fn get(e: &Engine, index: usize) -> Simple {
    e.ext_get::<Other>()
        .map(|o| o.0.lock().unwrap().menus[index].clone())
        .unwrap_or_default()
}
pub(super) fn configure(e: &mut Engine, settings: &Settings) {
    state(e, |s| {
        s.remove = settings.long("Menu Search", "bRemoveFilterWhenClosingSearch", 1) != 0;
        s.clear =
            s.remove || settings.long("Menu Search", "bClearInputWhenReopeningSearch", 1) != 0;
        for (i, key) in [
            "bContainerMenu",
            "bLevelUpMenuSearch",
            "bRecipeMenuSearch",
            "bBarterMenu",
            "bSaveMenuSearch",
        ]
        .into_iter()
        .enumerate()
        {
            s.menus[i].enabled = settings.long("Menu Search", key, 1) != 0;
        }
    });
}
pub(super) fn install(e: &mut Engine) {
    let mut installed = false;
    state(e, |s| {
        installed = s.installed;
        s.installed = true;
    });
    if installed {
        return;
    }
    for (index, slot) in SLOTS.into_iter().enumerate() {
        if !get(e, index).enabled || !e.mem.is_mapped(slot) {
            continue;
        }
        if index == 4 {
            if e.mem.u32(SINGLETONS[4]) == 0 {
                state(e, |s| s.menus[4].enabled = false);
                continue;
            }
            setup::<4>(e, &[]);
            if get(e, 4).tile == 0 {
                state(e, |s| s.menus[4].enabled = false);
                continue;
            }
        }
        let original = e.mem.u32(slot);
        state(e, |s| s.menus[index].original = original);
        e.mem.set_u32(slot, BASE + index as u32 * 4);
    }
}

pub(super) fn prepare(e: &mut Engine) {
    let mut prepared = false;
    state(e, |s| {
        prepared = s.prepared;
        s.prepared = true;
    });
    if prepared {
        return;
    }
    for (index, handler) in [
        key::<0> as engine_core::AbiFn,
        key::<1>,
        key::<2>,
        key::<3>,
        key::<4>,
    ]
    .into_iter()
    .enumerate()
    {
        if get(e, index).enabled {
            e.register(BASE + index as u32 * 4, handler);
        }
    }
    e.register(MODE_FILTER, container_mode);
    e.register(FIRST_FILTER, container_first);
    e.register(PERK_FILTER, perk_filter);
    e.register(RECIPE_PREDICATE, recipe_filter);
    e.register(SAVE_FILTER, save_filter);
    e.register(SAVE_NULL_FILTER, |_, w| ret(u32::from(w[0] != 0)));
    for (i, f) in [
        barter_filter::<0> as engine_core::AbiFn,
        barter_filter::<1>,
        barter_filter::<2>,
        barter_filter::<3>,
    ]
    .into_iter()
    .enumerate()
    {
        e.register(BARTER_FILTER_BASE + i as u32 * 4, f);
    }
    for (index, setup, clean) in [
        (
            0,
            setup::<0> as engine_core::extend::SiteFn,
            cleanup::<0> as engine_core::extend::SiteFn,
        ),
        (1, setup::<1>, cleanup::<1>),
        (2, setup::<2>, cleanup::<2>),
        (3, setup::<3>, cleanup::<3>),
        (4, setup::<4>, cleanup::<4>),
    ] {
        if get(e, index).enabled {
            if index == 2 {
                crate::save_manager::install_recipe_hooks(e);
            } else {
                e.on_site(SiteSlot::new(Owner::Stewie, SETUP[index]), setup);
                e.on_site(SiteSlot::new(Owner::Stewie, CLEAN[index]), clean);
            }
        }
    }
    if get(e, 0).enabled {
        for address in [0x0075_c3f5, 0x0075_c443] {
            e.on_site(SiteSlot::new(Owner::Stewie, address), refresh_branch);
        }
        for address in CONTAINER_FILTERS {
            e.on_site(SiteSlot::new(Owner::Stewie, address), container_pointer);
        }
    }
    if get(e, 3).enabled {
        for address in [0x0072_dc9c, 0x0072_dcd3] {
            e.on_site(SiteSlot::new(Owner::Stewie, address), refresh_branch);
        }
        for (i, f) in [
            barter_pointer::<0> as engine_core::extend::SiteFn,
            barter_pointer::<1>,
            barter_pointer::<2>,
            barter_pointer::<3>,
        ]
        .into_iter()
        .enumerate()
        {
            e.on_site(SiteSlot::new(Owner::Stewie, BARTER_POINTERS[i]), f);
        }
    }
    if get(e, 2).enabled {
        e.on_site(SiteSlot::new(Owner::Stewie, RECIPE_FILTER), recipe_pointer);
    }
}

fn setup<const I: usize>(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let ready = e
        .ext_get::<Other>()
        .is_some_and(|s| s.0.lock().unwrap().installed);
    if !ready || !get(e, I).enabled {
        return None;
    }
    let menu = e.mem.u32(SINGLETONS[I]);
    let filename = [
        b"Container".as_slice(),
        b"LevelUp",
        b"Recipe",
        b"Barter",
        b"SaveLoad",
    ][I];
    let path = [
        b"data\\menus\\prefabs\\lStewieAl\\MenuSearch\\".as_slice(),
        filename,
        b".xml",
    ]
    .concat();
    let path = nvse::table::guest_str(e, &path);
    let root = e.mem.u32(menu + 4);
    let tile = e.call(0x00a0_1b00, &args![root, path]).u32();
    state(e, |s| s.menus[I].tile = tile);
    None
}
fn cleanup<const I: usize>(e: &mut Engine, _: &[u32]) -> Option<u32> {
    state(e, |s| {
        s.menus[I].text.clear();
        s.menus[I].hooked = false;
    });
    focus::<I>(e, false);
    if I == 1 && e.mem.u32(SINGLETONS[I]) != 0 {
        refresh::<I>(e, e.mem.u32(SINGLETONS[I]));
    }
    None
}
fn float(e: &mut Engine, tile: u32, id: u32, value: f32) {
    e.call(0x00a0_12d0, &args![tile, id, value.to_bits(), 1u32]);
}
fn focus<const I: usize>(e: &mut Engine, on: bool) {
    let s = get(e, I);
    if s.tile == 0 {
        return;
    }
    state(e, |s| s.menus[I].active = on);
    if on {
        float(e, s.tile, 0xfa9, 255.);
        string(e, s.tile, if s.text.is_empty() { b"_" } else { &s.text });
        float(e, s.tile, 0xfa3, 1.);
    } else {
        string(e, s.tile, &s.text);
        let mut remove = false;
        state(e, |s| remove = s.remove);
        if remove {
            float(e, s.tile, 0xfa3, 0.);
        } else {
            float(e, s.tile, 0xfa9, 128.);
        }
    }
}
fn append(text: &mut Vec<u8>, key: u32, control: bool) -> bool {
    if key < 0x8000_0000 {
        if matches!(key, 9 | 96) {
            return false;
        }
        text.push(key as u8);
        true
    } else if key == 0x8000_0000 {
        if control {
            text.clear();
        } else {
            text.pop();
        }
        true
    } else {
        false
    }
}
fn key<const I: usize>(e: &mut Engine, w: &[u32]) -> Ret {
    let control = ctrl(e);
    key_with_control::<I>(e, w, control)
}
fn key_with_control<const I: usize>(e: &mut Engine, w: &[u32], control: bool) -> Ret {
    let menu = w[0];
    let key = w[1];
    let old = get(e, I);
    if I == 4 && !e.call(0x007d_88a0, &args![menu + 0x174]).bool() {
        return e.call(old.original, w);
    }
    if control && matches!(key | 0x20, 0x66 | 0x72) {
        if key | 0x20 == 0x72 {
            state(e, |s| {
                s.menus[I].text.clear();
                if I == 0 {
                    s.menus[I].hooked = false;
                }
            });
            focus::<I>(e, false);
            refresh::<I>(e, menu);
            return ret(1);
        }
        let (mut remove, mut clear) = (false, false);
        state(e, |s| {
            remove = s.remove;
            clear = s.clear;
        });
        if (!old.active && clear) || remove {
            state(e, |s| s.menus[I].text.clear());
        }
        let on = !old.active && (I != 1 || e.mem.u32(menu + 0x28) == 1);
        focus::<I>(e, on);
        if on && get(e, I).tile != 0 {
            state(e, |s| s.menus[I].hooked = true);
        }
        if I == 3 || get(e, I).active || remove {
            refresh::<I>(e, menu);
        }
        return ret(1);
    }
    if old.active {
        let mut text = old.text;
        if append(&mut text, key, control) {
            state(e, |s| s.menus[I].text = text);
            let s = get(e, I);
            string(e, s.tile, if s.text.is_empty() { b"_" } else { &s.text });
            refresh::<I>(e, menu);
            return ret(1);
        }
    }
    e.call(old.original, w)
}
fn refresh<const I: usize>(e: &mut Engine, menu: u32) {
    match I {
        0 => force_refresh(e),
        1 => {
            e.call(0x0072_9fe0, &args![menu + 0x94, PERK_FILTER]);
        }
        2 => {
            e.call(0x0072_9fe0, &args![menu + 0x6c, RECIPE_PREDICATE]);
        }
        3 => force_refresh(e),
        4 => {
            let list = menu + 0x174;
            e.call(0x0072_9fe0, &args![list, SAVE_FILTER]);
            let last = e.call(0x007d_87b0, &args![list]).u32();
            e.call(0x0073_14c0, &args![list, 0x007d_67b0u32, 0u32, last]);
            e.mem.set_u32(menu + 0x1ac, 0);
            if e.call(0x0071_ae60, &args![list]).u32() == 0 {
                e.call(0x0072_9fe0, &args![list, SAVE_NULL_FILTER]);
            }
        }
        _ => unreachable!(),
    }
}
fn container_pointer(e: &mut Engine, w: &[u32]) -> Option<u32> {
    get(e, 0).hooked.then_some(if w[0] == 0x0075_cfc0 {
        FIRST_FILTER
    } else {
        MODE_FILTER
    })
}
fn recipe_pointer(e: &mut Engine, _: &[u32]) -> Option<u32> {
    get(e, 2).hooked.then_some(RECIPE_PREDICATE)
}
fn container(e: &mut Engine, w: &[u32], original: u32) -> Ret {
    if e.call(original, w).bool() {
        return ret(1);
    }
    let text = name(e, e.mem.u32(w[0] + 8));
    ret(u32::from(!substring(&text, &get(e, 0).text)))
}
fn container_mode(e: &mut Engine, w: &[u32]) -> Ret {
    container(e, w, 0x0075_e650)
}
fn container_first(e: &mut Engine, w: &[u32]) -> Ret {
    container(e, w, 0x0075_cfc0)
}
pub(super) fn description(e: &mut Engine, form: u32) -> Vec<u8> {
    let (component, tag) = if e.mem.u8(form + 4) == 0x31 {
        if e.mem.u8(form + 0x7c) != 1 {
            return Vec::new();
        }
        (e.mem.u32(form + 0x6c), 0x4d41_4e54u32)
    } else {
        (
            e.call(
                0x00ec_43fb,
                &args![form, 0u32, 0x0118_3028u32, 0x0118_38f8u32, 0u32],
            )
            .u32(),
            0x4353_4544,
        )
    };
    if component == 0 {
        return Vec::new();
    }
    let p = e.vcall(component, 0x10, &args![form, tag]).u32();
    if p == 0 {
        Vec::new()
    } else {
        e.mem.cstr(p)
    }
}
fn perk_filter(e: &mut Engine, w: &[u32]) -> Ret {
    let term = get(e, 1).text;
    let form = w[0];
    let title = name(e, form);
    if substring(&title, &term) {
        return ret(0);
    }
    let description = description(e, form);
    ret(u32::from(!substring(&description, &term)))
}
fn recipe_filter(e: &mut Engine, w: &[u32]) -> Ret {
    if e.call(0x0072_8d60, w).bool() {
        return ret(1);
    }
    let term = get(e, 2).text;
    let form = w[0];
    let mut node = form + 0x3c;
    while node != 0 {
        let entry = e.mem.u32(node);
        let next = e.mem.u32(node + 4);
        if entry == 0 && next == 0 {
            break;
        }
        if entry != 0 {
            let title = name(e, e.mem.u32(entry + 4));
            if substring(&title, &term) {
                return ret(0);
            }
        }
        node = next;
    }
    let p = e.mem.u32(form + 0x1c);
    let text = if p != 0 && e.mem.u16(form + 0x20) != 0 {
        e.mem.cstr(p)
    } else {
        Vec::new()
    };
    ret(u32::from(!substring(&text, &term)))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn engine() -> Engine {
        let mut e = Engine::new();
        e.map(0x011d8000, 0x30000);
        e.map(0x02000000, 0x5000);
        e.register(0x00a01350, |_, _| Ret::default());
        e.register(0x00a012d0, |_, _| Ret::default());
        e.register(0x00729fe0, |_, _| Ret::default());
        state(&mut e, |s| {
            s.remove = true;
            s.clear = true;
            for item in &mut s.menus {
                item.enabled = true;
                item.tile = 0x02001000;
                item.original = 0x00fc0100;
            }
        });
        e.register(0x00fc0100, |_, _| 99u32.into_ret());
        e.call_log = Some(vec![]);
        e
    }
    #[test]
    fn recipe_teardown_resets_state_without_touching_poisoned_deleted_tile() {
        let mut e = engine();
        let tile = e.mem.alloc(0x100);
        crate::save_manager::install_recipe_hooks(&mut e);
        state(&mut e, |s| {
            s.menus[2].tile = tile;
            s.menus[2].active = true;
            s.menus[2].hooked = true;
            s.menus[2].text = b"stimpak".to_vec();
        });
        for address in [0x00a01350, 0x00a012d0] {
            e.register(address, |_, _| {
                panic!("post-destructor UI call uses deleted tile")
            });
        }
        // The native Menu destructor has released this search-tile allocation.
        e.mem.write(tile, &[0xdd; 0x100]);
        e.mem.free(tile);
        e.site(0x0072_6f2a, &[1]);
        let s = get(&e, 2);
        assert_eq!(s.tile, 0);
        assert!(!s.active && !s.hooked && s.text.is_empty());
        e.site(0x0072_6f2a, &[1]);
    }

    #[test]
    fn append_only_editor_is_distinct_from_full_editor() {
        let mut text = b"ab".to_vec();
        assert!(append(&mut text, b'v' as u32, true));
        assert_eq!(text, b"abv");
        assert!(append(&mut text, 13, false));
        assert!(!append(&mut text, 9, false));
        assert!(!append(&mut text, 96, false));
        assert!(!append(&mut text, 0x80000007, false));
        assert!(append(&mut text, 0x80000000, false));
        assert_eq!(text, b"abv");
        assert!(append(&mut text, 0x80000000, true));
        assert!(text.is_empty());
    }
    #[test]
    fn levelup_ctrl_f_only_opens_on_perks_page_and_refreshes_real_list() {
        let mut e = engine();
        let menu = 0x02000000;
        e.mem.set_u32(menu + 0x28, 0);
        assert_eq!(
            key_with_control::<1>(&mut e, &[menu, b'f' as u32], true).eax,
            1
        );
        assert!(!get(&e, 1).active);
        e.mem.set_u32(menu + 0x28, 1);
        key_with_control::<1>(&mut e, &[menu, b'f' as u32], true);
        assert!(get(&e, 1).active);
        key_with_control::<1>(&mut e, &[menu, b'P' as u32], false);
        assert_eq!(get(&e, 1).text, b"P");
        assert!(e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .any(|(a, w)| *a == 0x00729fe0 && w == &[menu + 0x94, PERK_FILTER]));
        key_with_control::<1>(&mut e, &[menu, b'r' as u32], true);
        assert!(!get(&e, 1).active);
        assert!(get(&e, 1).text.is_empty());
    }
    #[test]
    fn closed_focus_can_keep_filter_and_cleanup_restores_original_pointer() {
        let mut e = engine();
        let menu = 0x02000000;
        state(&mut e, |s| {
            s.remove = false;
            s.clear = false;
            s.menus[2].active = true;
            s.menus[2].hooked = true;
            s.menus[2].text = b"Keep".to_vec();
        });
        key_with_control::<2>(&mut e, &[menu, b'f' as u32], true);
        assert!(!get(&e, 2).active);
        assert_eq!(get(&e, 2).text, b"Keep");
        assert_eq!(recipe_pointer(&mut e, &[]), Some(RECIPE_PREDICATE));
        cleanup::<2>(&mut e, &[]);
        assert!(get(&e, 2).text.is_empty());
        assert_eq!(recipe_pointer(&mut e, &[]), None);
    }
    #[test]
    fn recipe_and_container_filters_keep_vanilla_exclusions() {
        let mut e = engine();
        e.register(0x00728d60, |_, _| 1u32.into_ret());
        assert_eq!(recipe_filter(&mut e, &[0]).eax, 1);
        e.register(0x0075cfc0, |_, _| 1u32.into_ret());
        assert_eq!(container_first(&mut e, &[0]).eax, 1);
        e.register(0x00728d60, |_, _| 0u32.into_ret());
        let recipe = 0x02003000;
        let p = nvse::table::guest_str(&mut e, b"Recycling supplies");
        e.mem.set_u32(recipe + 0x1c, p);
        e.mem.set_u16(recipe + 0x20, 18);
        state(&mut e, |s| s.menus[2].text = b"SUPPLIES".to_vec());
        assert_eq!(recipe_filter(&mut e, &[recipe]).eax, 0);
        state(&mut e, |s| s.menus[2].text = b"missing".to_vec());
        assert_eq!(recipe_filter(&mut e, &[recipe]).eax, 1);
    }
}

fn barter_pointer<const J: usize>(e: &mut Engine, w: &[u32]) -> Option<u32> {
    if !get(e, 3).hooked {
        return None;
    }
    let routine = BARTER_FILTER_BASE + J as u32 * 4;
    if w[0] != routine {
        state(e, |s| s.menus[3].filters[J] = w[0]);
    }
    Some(routine)
}
fn barter_filter<const J: usize>(e: &mut Engine, w: &[u32]) -> Ret {
    let original = get(e, 3).filters[J];
    if e.call(original, w).bool() {
        return ret(1);
    }
    let title = name(e, e.mem.u32(w[0] + 8));
    ret(u32::from(!substring(&title, &get(e, 3).text)))
}
fn save_filter(e: &mut Engine, w: &[u32]) -> Ret {
    if w[0] == 0 {
        let menu = e.mem.u32(SINGLETONS[4]);
        return ret(u32::from(e.mem.u32(menu + 0x1a8) & 8 == 0));
    }
    let title = e.mem.cstr(e.mem.u32(w[0]));
    ret(u32::from(!substring(&title, &get(e, 4).text)))
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    #[test]
    fn barter_filter_pointer_wraps_each_original_and_restores_on_close() {
        let mut e = Engine::new();
        state(&mut e, |s| {
            s.menus[3].hooked = true;
            s.menus[3].enabled = true;
        });
        for (j, p) in [0x00fc0000, 0x00fc0004, 0x00fc0008, 0x00fc000c]
            .into_iter()
            .enumerate()
        {
            e.register(p, |_, w| {
                assert_eq!(w, &[0]);
                ret(1)
            });
            let result = match j {
                0 => barter_pointer::<0>(&mut e, &[p]),
                1 => barter_pointer::<1>(&mut e, &[p]),
                2 => barter_pointer::<2>(&mut e, &[p]),
                _ => barter_pointer::<3>(&mut e, &[p]),
            };
            assert_eq!(result, Some(BARTER_FILTER_BASE + j as u32 * 4));
        }
        assert_eq!(barter_filter::<0>(&mut e, &[0]).eax, 1);
        assert_eq!(barter_filter::<1>(&mut e, &[0]).eax, 1);
        assert_eq!(barter_filter::<2>(&mut e, &[0]).eax, 1);
        assert_eq!(barter_filter::<3>(&mut e, &[0]).eax, 1);
        cleanup::<3>(&mut e, &[]);
        assert_eq!(barter_pointer::<0>(&mut e, &[0x00fc0000]), None);
    }
    #[test]
    fn save_search_only_handles_enabled_save_list_and_preserves_placeholder_rule() {
        let mut e = Engine::new();
        e.map(0x011da000, 0x1000);
        e.map(0x02000000, 0x3000);
        let menu = 0x02000000;
        e.mem.set_u32(SINGLETONS[4], menu);
        state(&mut e, |s| {
            s.remove = true;
            s.clear = true;
            s.menus[4].tile = menu + 0x1000;
            s.menus[4].original = 0x00fc0000;
        });
        e.register(0x00fc0000, |_, _| ret(99));
        e.register(0x007d88a0, |_, _| ret(0));
        assert_eq!(
            key_with_control::<4>(&mut e, &[menu, b'f' as u32], true).eax,
            99
        );
        assert!(!get(&e, 4).active);
        e.register(0x007d88a0, |_, _| ret(1));
        for addr in [0x00a01350, 0x00a012d0, 0x00729fe0, 0x007314c0] {
            e.register(addr, |_, _| Ret::default());
        }
        e.register(0x007d87b0, |_, _| ret(7));
        e.register(0x0071ae60, |_, _| ret(0));
        e.call_log = Some(vec![]);
        key_with_control::<4>(&mut e, &[menu, b'f' as u32], true);
        assert!(get(&e, 4).active);
        let logs = e.call_log.as_ref().unwrap();
        assert!(logs
            .iter()
            .any(|(a, w)| *a == 0x007314c0 && w == &[menu + 0x174, 0x007d67b0, 0, 7]));
        assert!(logs
            .iter()
            .any(|(a, w)| *a == 0x00729fe0 && w == &[menu + 0x174, SAVE_NULL_FILTER]));
        e.mem.set_u32(menu + 0x1a8, 0);
        assert_eq!(save_filter(&mut e, &[0]).eax, 1);
        e.mem.set_u32(menu + 0x1a8, 8);
        assert_eq!(save_filter(&mut e, &[0]).eax, 0);
    }
}

pub(super) fn recipe_lifecycle(e: &mut Engine, opening: bool, phase: u32) {
    if phase == 1 && get(e, 2).enabled {
        if opening {
            setup::<2>(e, &[]);
        } else {
            // The native Recipe destructor has deleted its root/search tiles.
            // Reset owned state without reproducing the DLL's post-free UI writes.
            state(e, |s| {
                s.menus[2].text.clear();
                s.menus[2].active = false;
                s.menus[2].hooked = false;
                s.menus[2].tile = 0;
            });
        }
    }
}
