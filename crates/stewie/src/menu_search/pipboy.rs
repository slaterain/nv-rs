//! Stats and Map MenuSearch, owned Tweaks10.05 100408F0..10041BF2,
//! custom tile-list filter10043D20 and objective rebuild10079610.
//! Independent Map Recenter/ModSelector tweaks are not enabled by this module.
use super::*;
const BASE: u32 = KEY + 0x100;
const GLOBALS: [u32; 2] = [0x011d_a368, 0x011d_ace0];
const SLOTS: [u32; 2] = [0x0107_4d74, 0x0107_0004];
const AV: u32 = BASE + 0x10;
const PERK: u32 = BASE + 0x14;
const FORM: u32 = BASE + 0x18;
const EFFECT: u32 = BASE + 0x1c;
const QUEST: u32 = BASE + 0x20;
const NOTE: u32 = BASE + 0x24;
const CHALLENGE: u32 = BASE + 0x28;
const RADIO: u32 = BASE + 0x2c;
const ALL: u32 = BASE + 0x30;
#[derive(Clone, Default)]
struct Full {
    enabled: bool,
    installed: bool,
    tile: u32,
    original: u32,
    editor: Editor,
    dirty: bool,
    hooked: bool,
}
#[derive(Default)]
struct Values {
    menus: [Full; 2],
    remove: bool,
    clear: bool,
    objectives: bool,
    completed: bool,
    skip_center: bool,
    skip_world: bool,
    prepared: bool,
}
#[derive(Default)]
struct Pipboy(Mutex<Values>);
fn state(e: &mut Engine, f: impl FnOnce(&mut Values)) {
    f(&mut e.ext_free::<Pipboy>().0.lock().unwrap());
}
fn get(e: &Engine, i: usize) -> Full {
    e.ext_get::<Pipboy>()
        .map(|s| s.0.lock().unwrap().menus[i].clone())
        .unwrap_or_default()
}
pub(super) fn configure(e: &mut Engine, settings: &Settings) {
    state(e, |s| {
        s.remove = settings.long("Menu Search", "bRemoveFilterWhenClosingSearch", 1) != 0;
        s.clear =
            s.remove || settings.long("Menu Search", "bClearInputWhenReopeningSearch", 1) != 0;
        s.objectives = settings.long("Menu Search", "bQuestIncludeObjectives", 1) != 0;
        s.completed = settings.long("Menu Search", "bQuestIncludeCompletedObjectives", 1) != 0;
        s.menus[0].enabled = settings.long("Menu Search", "bMapMenu", 1) != 0;
        s.menus[1].enabled = settings.long("Menu Search", "bStatsMenu", 1) != 0;
    });
}
pub(super) fn install(e: &mut Engine) {
    for i in 0..2 {
        let s = get(e, i);
        if !s.enabled || s.installed {
            continue;
        }
        state(e, |s| s.menus[i].installed = true);
        let menu = e.mem.u32(GLOBALS[i]);
        if menu == 0 || !e.mem.is_mapped(SLOTS[i]) {
            continue;
        }
        let which = if i == 0 { b"Map".as_slice() } else { b"Stats" };
        let path = [
            b"data\\menus\\prefabs\\lStewieAl\\MenuSearch\\".as_slice(),
            which,
            b".xml",
        ]
        .concat();
        let path = nvse::table::guest_str(e, &path);
        let tile = e.call(0x00a0_1b00, &args![e.mem.u32(menu + 4), path]).u32();
        if tile == 0 {
            continue;
        }
        let original = e.mem.u32(SLOTS[i]);
        state(e, |s| {
            s.menus[i].tile = tile;
            s.menus[i].original = original;
        });
        e.mem.set_u32(SLOTS[i], BASE + i as u32 * 4);
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
    for i in 0..2 {
        if !get(e, i).enabled {
            continue;
        }
        let handler = if i == 0 {
            key::<0> as engine_core::AbiFn
        } else {
            key::<1>
        };
        e.register(BASE + i as u32 * 4, handler);
        e.on_site(
            SiteSlot::new(
                Owner::Stewie,
                if i == 0 { 0x0079_a221 } else { 0x007d_c504 },
            ),
            if i == 0 { idle::<0> } else { idle::<1> },
        );
        if i == 0 {
            for (addr, f) in [
                (0x0079_af0d, map_page as engine_core::extend::SiteFn),
                (0x0079_d0d9, skip_world),
                (0x0079_df73, skip_center),
                (0x0079_d18f, marker),
                (0x0079_d90a, door),
                (0x0079_bb86, quest_enabled),
                (0x0079_b331, note_pointer),
                (0x0079_b6ea, challenge_pointer),
                (0x0079_c015, radio_pointer),
            ] {
                e.on_site(SiteSlot::new(Owner::Stewie, addr), f);
            }
        } else {
            e.on_site(SiteSlot::new(Owner::Stewie, 0x007e_0051), stats_page);
        }
    }
    e.register(AV, av_filter);
    e.register(PERK, perk_filter);
    e.register(FORM, form_filter);
    e.register(EFFECT, effect_filter);
    e.register(QUEST, quest_filter);
    e.register(NOTE, note_filter);
    e.register(CHALLENGE, challenge_filter);
    e.register(RADIO, radio_filter);
    e.register(ALL, |_, _| ret(1));
}
fn display<const I: usize>(e: &mut Engine) {
    let s = get(e, I);
    if s.tile == 0 {
        return;
    }
    let mut text = s.editor.text;
    if s.editor.active && s.editor.cursor >= 0 && (s.editor.cursor as usize) <= text.len() {
        text.insert(
            s.editor.cursor as usize,
            if s.editor.phase { b'|' } else { b' ' },
        );
        trait_float(e, s.tile, b"_CaretIndex", f32::from(s.editor.cursor));
    }
    string(e, s.tile, &text);
}
fn focus<const I: usize>(e: &mut Engine, on: bool) {
    state(e, |s| {
        let edit = &mut s.menus[I].editor;
        edit.active = on;
        edit.phase = false;
        edit.last = 0;
    });
    trait_float(e, get(e, I).tile, b"_IsActive", if on { 1. } else { 0. });
    if !on {
        display::<I>(e);
    }
}
fn key<const I: usize>(e: &mut Engine, w: &[u32]) -> Ret {
    let control = ctrl(e);
    let key = w[1];
    let s = get(e, I);
    if control && matches!(key | 0x20, 0x66 | 0x72) {
        if key | 0x20 == 0x72 {
            state(e, |s| {
                s.menus[I].editor.text.clear();
                s.menus[I].editor.cursor = 0;
            });
            string(e, s.tile, b"");
            focus::<I>(e, false);
            refresh::<I>(e, w[0]);
            return ret(1);
        }
        let (mut clear, mut remove) = (false, false);
        state(e, |s| {
            clear = s.clear;
            remove = s.remove;
        });
        if (!s.editor.active && clear) || remove {
            state(e, |s| {
                s.menus[I].editor.text.clear();
                s.menus[I].editor.cursor = 0;
            });
            string(e, s.tile, b"");
        }
        let on = !s.editor.active;
        focus::<I>(e, on);
        if I == 0 && on {
            state(e, |s| s.menus[0].hooked = true);
        }
        if get(e, I).editor.active || remove {
            refresh::<I>(e, w[0]);
        }
        return ret(1);
    }
    if s.editor.active {
        let paste = if control && key | 0x20 == 0x76 {
            engine_core::clipboard::read_ansi_prefix(e, 200)
        } else {
            None
        };
        let mut editor = s.editor;
        if edit(&mut editor, key, control, paste.as_deref()) {
            let active = editor.active;
            state(e, |s| s.menus[I].editor = editor);
            if !active {
                focus::<I>(e, false);
            } else if key < 0x80000000 || matches!(key, 0x80000000 | 0x80000007) {
                string(e, s.tile, &get(e, I).editor.text);
            }
            refresh::<I>(e, w[0]);
            return ret(1);
        }
    }
    e.call(s.original, w)
}
fn idle<const I: usize>(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let s = get(e, I);
    if s.editor.active {
        let now = e.call(0x0045_7fe0, &args![]).u32();
        if now.wrapping_sub(s.editor.last) > 530 {
            state(e, |s| {
                s.menus[I].editor.phase = !s.menus[I].editor.phase;
                s.menus[I].editor.last = now;
            });
            display::<I>(e);
        }
    }
    None
}
pub(super) fn close(e: &mut Engine) {
    for i in 0..2 {
        let old = get(e, i);
        if old.tile == 0 {
            continue;
        }
        state(e, |s| {
            s.menus[i].editor = Editor::default();
            s.menus[i].hooked = false;
            s.menus[i].dirty = false;
        });
        string(e, old.tile, b"");
        if i == 0 {
            focus::<0>(e, false);
        } else {
            focus::<1>(e, false);
            if old.dirty {
                let menu = e.mem.u32(GLOBALS[1]);
                for (offset, filter) in [
                    (0x180, AV),
                    (0x1b0, AV),
                    (0x1e0, PERK),
                    (0x270, FORM),
                    (0x240, EFFECT),
                ] {
                    e.call(
                        if offset == 0x180 || offset == 0x1b0 {
                            0x0073_0bb0
                        } else {
                            0x0072_9fe0
                        },
                        &args![menu + offset, filter],
                    );
                }
                filter_tiles(e, menu + 0x210);
            }
        }
    }
}
fn refresh<const I: usize>(e: &mut Engine, menu: u32) {
    if I == 0 {
        refresh_map(e, menu);
    } else {
        state(e, |s| s.menus[1].dirty = true);
        let page = e
            .call(0x00a0_11b0, &args![e.mem.u32(menu + 4), 0x1004u32])
            .st0 as f32;
        let page = engine_core::x87::fistp_chop(f64::from(page));
        match page {
            0 => {
                e.call(0x0072_9fe0, &args![menu + 0x240, EFFECT]);
            }
            1 | 2 => {
                e.call(
                    0x0073_0bb0,
                    &args![menu + if page == 1 { 0x180 } else { 0x1b0 }, AV],
                );
            }
            3 => {
                e.call(0x0072_9fe0, &args![menu + 0x1e0, PERK]);
            }
            4 => {
                filter_tiles(e, menu + 0x210);
                e.call(0x0072_9fe0, &args![menu + 0x270, FORM]);
            }
            _ => {}
        }
    }
}
fn stats_page(e: &mut Engine, w: &[u32]) -> Option<u32> {
    if get(e, 1).dirty {
        refresh::<1>(e, w[0]);
    }
    None
}
fn av_filter(e: &mut Engine, w: &[u32]) -> Ret {
    let ptr = if w[0] <= 76 {
        e.call(0x0066_eac0, w).u32()
    } else {
        0
    };
    let title = if ptr == 0 {
        b"unknown".to_vec()
    } else {
        e.mem.cstr(ptr)
    };
    ret(u32::from(!substring(&title, &get(e, 1).editor.text)))
}
fn perk_filter(e: &mut Engine, w: &[u32]) -> Ret {
    let form = e.mem.u32(w[0]);
    if form == 0 {
        return ret(1);
    }
    let term = get(e, 1).editor.text;
    let title = name(e, form);
    if substring(&title, &term) {
        return ret(0);
    }
    let text = simple::description(e, form);
    ret(u32::from(!substring(&text, &term)))
}
fn form_filter(e: &mut Engine, w: &[u32]) -> Ret {
    let title = name(e, w[0]);
    ret(u32::from(!substring(&title, &get(e, 1).editor.text)))
}
fn effect_filter(e: &mut Engine, w: &[u32]) -> Ret {
    form_filter(e, &[e.mem.u32(w[0])])
}
fn set_float(e: &mut Engine, tile: u32, id: u32, value: f32) {
    e.call(0x00a0_12d0, &args![tile, id, value.to_bits(), 1u32]);
}
/// Source10043D20 differs from normal ListBox::Filter by feeding the row tile,
/// not its data. Keep the source's highlight fallback, u16 index and float heights.
fn filter_tiles(e: &mut Engine, list: u32) {
    let trait_name = nvse::table::guest_str(e, b"_num_filtered");
    let trait_id = e.call(0x00a0_0940, &args![trait_name, u32::MAX]).u32();
    e.mem.set_u16(list + 0x1c, 0);
    e.mem.set_u32(list + 0x20, 0);
    let (mut filtered, mut count, mut remembered) = (0u32, 0u32, -1i32);
    let mut node = list + 4;
    while node != 0 {
        let row = e.mem.u32(node);
        if row == 0 {
            break;
        }
        count = count.wrapping_add(1);
        let tile = e.mem.u32(row);
        let value = trait_value(e, tile, 0xfc4);
        let title = if value == 0 {
            Vec::new()
        } else {
            let ptr = e.mem.u32(value + 0xc);
            if ptr == 0 {
                Vec::new()
            } else {
                e.mem.cstr(ptr)
            }
        };
        let excluded = !substring(&title, &get(e, 1).editor.text);
        e.mem.set_u8(row + 8, u8::from(excluded));
        if excluded {
            if tile == e.mem.u32(list + 0x10) {
                let index = e.call(0x00a0_11b0, &args![tile, 0xfacu32]).st0 as f32;
                remembered = engine_core::x87::fistp_chop(f64::from(index));
                e.mem.set_u32(list + 0x10, 0);
            }
            set_float(e, tile, 0xfac, -1.);
            filtered = filtered.wrapping_add(1);
        } else {
            let index = e.mem.u16(list + 0x1c);
            if remembered >= 0 && i32::from(index) == remembered {
                e.mem.set_u32(list + 0x10, tile);
            }
            e.mem.set_u16(list + 0x1c, index.wrapping_add(1));
            set_float(e, tile, 0xfac, f32::from(index));
            e.call(0x0072_69d0, &args![list, tile]);
        }
        node = e.mem.u32(node + 4);
    }
    set_float(e, e.mem.u32(list + 0xc), trait_id, filtered as f32);
    e.call(0x0071_ad30, &args![list]);
    if e.mem.u32(list + 0x10) == 0 && remembered >= 0 {
        let tile = e.vcall(list, 0x14, &args![remembered, 0u32]).u32();
        e.mem.set_u32(list + 0x10, tile);
        if tile == 0 {
            let tile = e
                .vcall(
                    list,
                    0x14,
                    &args![count.wrapping_sub(filtered).wrapping_sub(1), 0u32],
                )
                .u32();
            e.mem.set_u32(list + 0x10, tile);
        }
    }
    e.call(0x0071_a740, &args![list]);
}
fn scope(e: &mut Engine, world: bool, f: impl FnOnce(&mut Engine)) {
    state(e, |s| s.skip_center = true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(e)));
    state(e, |s| {
        s.skip_center = false;
        if world {
            s.skip_world = false;
        }
    });
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
fn refresh_map(e: &mut Engine, menu: u32) {
    match e.mem.u8(menu + 0x80) {
        0x20 => {
            e.call(0x00a0_4150, &args![e.mem.u32(menu + 0x30)]);
            scope(e, false, |e| {
                e.call(0x0079_d410, &args![menu]);
            });
        }
        0x21 => {
            e.call(0x00a0_4150, &args![e.mem.u32(menu + 0x38)]);
            let mut node = e.mem.u32(menu + 0xdc);
            e.mem.set_u32(menu + 0xd8, 0);
            e.mem.set_u32(menu + 0xdc, 0);
            while node != 0 {
                let next = e.mem.u32(node + 4);
                e.call(0x00aa_4060, &args![0x011f_6238u32, node]);
                node = next;
            }
            scope(e, false, |e| {
                e.call(0x0079_cdb0, &args![menu]);
            });
        }
        0x22 => {
            e.call(0x0072_9fe0, &args![menu + 0x130, QUEST]);
            if e.call(0x0071_ae60, &args![menu + 0x130]).u32() == 0 {
                e.call(0x0072_9fe0, &args![menu + 0x1c0, ALL]);
            }
            rebuild_objectives(e, menu);
        }
        0x23 => {
            e.call(0x0072_9fe0, &args![menu + 0x160, NOTE]);
            e.call(0x0072_9fe0, &args![menu + 0x1f0, CHALLENGE]);
        }
        0x24 => {
            e.call(0x0072_9fe0, &args![menu + 0x190, RADIO]);
        }
        _ => {}
    }
}
fn skip_center(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let mut yes = false;
    state(e, |s| yes = s.skip_center);
    yes.then_some(1)
}
fn skip_world(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let mut yes = false;
    state(e, |s| yes = s.skip_world);
    yes.then_some(1)
}
fn map_page(e: &mut Engine, w: &[u32]) -> Option<u32> {
    if get(e, 0).tile == 0 {
        return None;
    }
    if w[1] == 0 {
        if e.mem.u8(w[0] + 0x80) == 0x21 {
            state(e, |s| s.skip_world = true);
        }
    } else {
        state(e, |s| s.skip_world = false);
        if e.mem.u8(w[0] + 0x80) != 0x20 {
            refresh_map(e, w[0]);
        }
    }
    None
}
fn note_pointer(e: &mut Engine, _: &[u32]) -> Option<u32> {
    get(e, 0).hooked.then_some(NOTE)
}
fn challenge_pointer(e: &mut Engine, _: &[u32]) -> Option<u32> {
    get(e, 0).hooked.then_some(CHALLENGE)
}
fn radio_pointer(e: &mut Engine, _: &[u32]) -> Option<u32> {
    get(e, 0).hooked.then_some(RADIO)
}
fn quest_enabled(e: &mut Engine, w: &[u32]) -> Option<u32> {
    if get(e, 0).hooked {
        e.call(0x0072_9fe0, &args![w[0], QUEST]);
    }
    None
}
fn marker(e: &mut Engine, w: &[u32]) -> Option<u32> {
    if !get(e, 0).hooked {
        return None;
    }
    let data = w[0];
    if e.mem.u8(data + 0xc) & 1 == 0 {
        return Some(0);
    }
    let p = e.mem.u32(data + 4);
    let title = if p != 0 && e.mem.u16(data + 8) != 0 {
        e.mem.cstr(p)
    } else {
        Vec::new()
    };
    Some(u32::from(substring(&title, &get(e, 0).editor.text)))
}
fn door(e: &mut Engine, w: &[u32]) -> Option<u32> {
    if !get(e, 0).hooked {
        return None;
    }
    if e.call(0x0057_b4b0, w).bool() {
        return Some(1);
    }
    let extra = e.call(0x0041_0220, &args![w[0] + 0x44, 0x2bu32]).u32();
    if extra == 0 {
        return Some(1);
    }
    let title = e.with_stack(8, |e, out| {
        let ptr = e.call(0x00aa_3e40, &args![0x011f_6238u32, 1u32]).u32();
        e.mem.set_u8(ptr, 0);
        e.mem.set_u32(out.addr(), ptr);
        e.mem.set_u32(out.addr() + 4, 0);
        e.call(0x0043_a2e0, &args![e.mem.u32(extra + 0xc), out]);
        let p = e.mem.u32(out.addr());
        let text = if p != 0 && e.mem.u16(out.addr() + 4) != 0 {
            e.mem.cstr(p)
        } else {
            Vec::new()
        };
        if p != 0 {
            e.call(0x00aa_4060, &args![0x011f_6238u32, p]);
        }
        text
    });
    Some(u32::from(!substring(&title, &get(e, 0).editor.text)))
}
fn objective_text(e: &Engine, o: u32) -> Vec<u8> {
    let p = e.mem.u32(o + 8);
    if p != 0 && e.mem.u16(o + 0xc) != 0 {
        e.mem.cstr(p)
    } else {
        Vec::new()
    }
}
fn shown(e: &mut Engine, o: u32) -> bool {
    if e.mem.u8(o + 0x20) & 1 == 0 {
        return false;
    }
    !name(e, e.mem.u32(o + 0x10)).is_empty() || !objective_text(e, o).is_empty()
}
fn quest_filter(e: &mut Engine, w: &[u32]) -> Ret {
    let quest = w[0];
    let term = get(e, 0).editor.text;
    if term.is_empty() {
        return ret(0);
    }
    let (mut include, mut completed) = (false, false);
    state(e, |s| {
        include = s.objectives;
        completed = s.completed;
    });
    if include {
        let mut node = quest + 0x4c;
        while node != 0 {
            let o = e.mem.u32(node);
            if o != 0
                && e.mem.u32(o) == 0x0104_7088
                && shown(e, o)
                && (e.mem.u32(o + 0x20) & 2 == 0 || completed)
                && substring(&objective_text(e, o), &term)
            {
                return ret(0);
            }
            node = e.mem.u32(node + 4);
        }
    }
    let title = name(e, quest);
    ret(u32::from(!substring(&title, &term)))
}
fn note_filter(e: &mut Engine, w: &[u32]) -> Ret {
    if e.call(0x0079_b390, w).bool() {
        return ret(1);
    }
    let note = w[0];
    let term = get(e, 0).editor.text;
    let title = if e.mem.u32(note + 0x4c) != 0 && e.mem.u16(note + 0x50) != 0 {
        e.mem.cstr(e.mem.u32(note + 0x4c))
    } else {
        Vec::new()
    };
    if substring(&title, &term) {
        return ret(0);
    }
    let text = simple::description(e, note);
    ret(u32::from(!substring(&text, &term)))
}
fn challenge_filter(e: &mut Engine, w: &[u32]) -> Ret {
    if e.call(0x0079_b720, w).bool() {
        return ret(1);
    }
    let item = w[0];
    let term = get(e, 0).editor.text;
    let title = if e.mem.u32(item + 0x1c) != 0 && e.mem.u16(item + 0x20) != 0 {
        e.mem.cstr(e.mem.u32(item + 0x1c))
    } else {
        Vec::new()
    };
    if substring(&title, &term) {
        return ret(0);
    }
    let text = simple::description(e, item);
    ret(u32::from(!substring(&text, &term)))
}
fn radio_filter(e: &mut Engine, w: &[u32]) -> Ret {
    if e.call(0x0079_be30, w).bool() {
        return ret(1);
    }
    let p = e.call(0x0055_d520, w).u32();
    let text = if p == 0 { Vec::new() } else { e.mem.cstr(p) };
    ret(u32::from(!substring(&text, &get(e, 0).editor.text)))
}
/// 10079610; the copied AddItem specialization55940 is the native objective
/// Insert7A44E0 with compare/template0 on this freshly cleared list.
fn rebuild_objectives(e: &mut Engine, menu: u32) {
    let list = menu + 0x1c0;
    e.vcall(list, 0x1c, &args![]);
    let highlight = e.mem.u32(menu + 0x140);
    let mut selected = 0;
    let mut node = menu + 0x134;
    while node != 0 {
        let row = e.mem.u32(node);
        if row != 0 && e.mem.u32(row) == highlight {
            selected = e.mem.u32(row + 4);
            break;
        }
        node = e.mem.u32(node + 4);
    }
    let mut node = e.mem.u32(0x011d_ea3c) + 0x6bc;
    while node != 0 {
        let o = e.mem.u32(node);
        if o != 0 && e.mem.u32(o + 0x10) == selected && shown(e, o) {
            let text = nvse::table::guest_str(e, &objective_text(e, o));
            let tile = e.call(0x007a_44e0, &args![list, o, text, 0u32, 0u32]).u32();
            let child = e.mem.u32(e.mem.u32(tile + 8) + 8);
            set_float(e, child, 0xfba, 320.);
            trait_float(e, tile, b"_ShowEmptyMarker", 1.);
            trait_float(e, tile, b"_enabled", 0.);
            let done = e.mem.u32(o + 0x20) & 2 != 0;
            if done {
                set_float(e, tile, e.mem.u32(0x011d_a348), 1.);
            }
            let alpha = if done || (e.mem.u8(list + 0x3c) & 2 != 0) {
                128.
            } else {
                255.
            };
            set_float(e, tile, e.mem.u32(0x011d_a344), alpha);
        }
        node = e.mem.u32(node + 4);
    }
    // Tweaks 10.05 reads the immediate operand of the game's push at 0079BB43.
    // Native code passes that comparator directly because executable text is unmapped.
    e.vcall(list, 0x20, &args![0x0079_bc90u32]);
}
#[cfg(test)]
mod tests;

// 10005490: the DLL's read-only binary trait lookup (no creation).
fn trait_value(e: &Engine, tile: u32, id: u32) -> u32 {
    let ptr = e.mem.u32(tile + 0x14);
    let (mut lo, mut hi) = (0u32, e.mem.u32(tile + 0x18));
    while lo < hi {
        let mid = lo.wrapping_add(hi) / 2;
        let value = e.mem.u32(ptr + mid * 4);
        match e.mem.u32(value).cmp(&id) {
            std::cmp::Ordering::Equal => return value,
            std::cmp::Ordering::Less => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid,
        }
    }
    0
}
