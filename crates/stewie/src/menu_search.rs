//! Inventory part of Menu Search, reconstructed from owned Tweaks 10.05:
//! 10042DE0 installer, 10041470..100417B3 inventory, 100527C0..10053020 editor,
//! 10065380 substring. The DLL's private CRT uses its default C character locale.
//! The other menu handlers live in `pipboy` and `simple`.
mod pipboy;
mod simple;
use crate::settings::Settings;
use engine_core::{
    extend::{Owner, SiteSlot, ROUTINE_BASE},
    prelude::*,
};
use std::sync::Mutex;

const FILTERS: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Stewie, 0x0078_028e),
    SiteSlot::new(Owner::Stewie, 0x0078_0894),
    SiteSlot::new(Owner::Stewie, 0x0078_3214),
];
const IDLE: SiteSlot = SiteSlot::new(Owner::Stewie, 0x0078_1bbd);
const CLOSE: SiteSlot = SiteSlot::new(Owner::Stewie, 0x0070_f6d3);
const KEY_SLOT: u32 = 0x0107_39e4;
const KEY: u32 = ROUTINE_BASE + 0xf000;
const FILTER: u32 = KEY + 4;
const KEY_FILTER: u32 = KEY + 8;
const MENU: u32 = 0x011d_9ea4;
const MAP: u32 = 0x011d_a368;
const ORIGINAL_FILTER: u32 = 0x0078_2620;
const PATH: &[u8] = b"data\\menus\\prefabs\\lStewieAl\\MenuSearch\\Inventory.xml";
#[derive(Clone, Default)]
struct Editor {
    text: Vec<u8>,
    cursor: i16,
    active: bool,
    phase: bool,
    last: u32,
}
#[derive(Clone, Default)]
struct Inner {
    enabled: bool,
    attempted: bool,
    remove: bool,
    clear: bool,
    tile: u32,
    original_key: u32,
    hooked: bool,
    refreshing: bool,
    editor: Editor,
}
#[derive(Default)]
struct State(Mutex<Inner>);
fn snapshot(e: &Engine) -> Inner {
    e.ext_get::<State>()
        .map(|s| s.0.lock().unwrap().clone())
        .unwrap_or_default()
}
fn change(e: &mut Engine, f: impl FnOnce(&mut Inner)) {
    f(&mut e.ext_free::<State>().0.lock().unwrap());
}
fn ctrl(e: &mut Engine) -> bool {
    [29, 157]
        .into_iter()
        .any(|k| nvse::input::key_states(e, k) & 2 != 0)
}
fn ret(n: u32) -> Ret {
    n.into_ret()
}

pub(crate) fn install(e: &mut Engine, settings: &Settings) {
    if settings.long("Tweaks", "bMenuSearch", 0) == 0 {
        return;
    }
    e.on_site(CLOSE, close);
    simple::configure(e, settings);
    pipboy::configure(e, settings);
    let remove = settings.long("Menu Search", "bRemoveFilterWhenClosingSearch", 1) != 0;
    change(e, |s| {
        s.enabled = settings.long("Menu Search", "bInventoryMenu", 1) != 0;
        s.remove = remove;
        s.clear = remove || settings.long("Menu Search", "bClearInputWhenReopeningSearch", 1) != 0;
    });
    // Handler tables and shared slots must be published before real threads.
    // The native startup boundary below only discovers live UI objects/assets.
    simple::prepare(e);
    pipboy::prepare(e);
    if snapshot(e).enabled {
        e.register(KEY, key);
        e.register(FILTER, filter);
        e.register(KEY_FILTER, key_filter);
        for slot in FILTERS {
            e.on_site(slot, filter_pointer);
        }
        e.on_site(IDLE, idle);
        for address in [0x0078_2f17, 0x0078_319c] {
            e.on_site(SiteSlot::new(Owner::Stewie, address), refresh_branch);
        }
    }
    e.on_site(SiteSlot::new(Owner::Stewie, 0x0086_b650), |e, _| {
        if e.mem.u32(MAP) != 0 {
            pipboy::install(e);
            simple::install(e);
            after_menus(e);
        }
        None
    });
}
/// The DLL's delayed setup (10061A20) waits for the persistent Map menu, then
/// attempts injection exactly once at the matching native startup boundary.
fn after_menus(e: &mut Engine) {
    let s = snapshot(e);
    if !s.enabled || s.attempted || e.mem.u32(MAP) == 0 {
        return;
    }
    change(e, |s| s.attempted = true);
    let menu = e.mem.u32(MENU);
    if menu == 0 || !e.mem.is_mapped(KEY_SLOT) {
        return;
    }
    let path = nvse::table::guest_str(e, PATH);
    let tile = e.call(0x00a0_1b00, &args![e.mem.u32(menu + 4), path]).u32();
    if tile == 0 {
        return;
    }
    let original = e.mem.u32(KEY_SLOT);
    change(e, |s| {
        s.tile = tile;
        s.original_key = original;
    });
    e.mem.set_u32(KEY_SLOT, KEY);
}
fn filter_pointer(e: &mut Engine, w: &[u32]) -> Option<u32> {
    snapshot(e)
        .hooked
        .then_some(FILTER)
        .or_else(|| w.first().copied())
}
fn substring(hay: &[u8], needle: &[u8]) -> bool {
    let hay = &hay[..hay.iter().position(|b| *b == 0).unwrap_or(hay.len())];
    let needle = &needle[..needle.iter().position(|b| *b == 0).unwrap_or(needle.len())];
    needle.is_empty()
        || hay
            .windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
}
/// 10004160: reference base, quest name/parent fallback, or TESFullName RTTI.
fn name(e: &mut Engine, mut form: u32) -> Vec<u8> {
    if e.vcall(form, 0xf0, &args![]).bool() {
        form = e.mem.u32(form + 0x20);
    }
    let component = if e.mem.u32(form) == 0x0102_e9b4 {
        let own = form + 0x18;
        if e.mem.u32(own + 4) != 0 && e.mem.u16(own + 8) != 0 {
            own
        } else {
            let parent = e.mem.u32(form + 0xc0);
            if parent != 0 {
                parent + 0x18
            } else {
                own
            }
        }
    } else {
        e.call(
            0x00ec_43fb,
            &args![form, 0u32, 0x0118_3028u32, 0x0118_3158u32, 0u32],
        )
        .u32()
    };
    if component == 0 || e.mem.u32(component + 4) == 0 || e.mem.u16(component + 8) == 0 {
        Vec::new()
    } else {
        e.mem.cstr(e.mem.u32(component + 4))
    }
}
fn filter(e: &mut Engine, w: &[u32]) -> Ret {
    if e.call(ORIGINAL_FILTER, w).bool() {
        return ret(1);
    }
    let item = w[0];
    let form = if item != 0 { e.mem.u32(item + 8) } else { 0 };
    let text = if form != 0 {
        name(e, form)
    } else {
        let p = e.mem.u32(0x011d_3b24);
        if p != 0 {
            e.mem.cstr(p)
        } else {
            Vec::new()
        }
    };
    ret(u32::from(!substring(&text, &snapshot(e).editor.text)))
}
fn key_filter(e: &mut Engine, w: &[u32]) -> Ret {
    let item = w[0];
    if item == 0 {
        return ret(1);
    }
    let form = e.mem.u32(item + 8);
    if e.mem.u8(form + 4) != 0x2e {
        return ret(1);
    }
    let text = name(e, form);
    ret(u32::from(!substring(&text, &snapshot(e).editor.text)))
}
fn string(e: &mut Engine, tile: u32, text: &[u8]) {
    let text = nvse::table::guest_str(e, text);
    e.call(0x00a0_1350, &args![tile, 0xfc4u32, text, 1u32]);
}
fn trait_float(e: &mut Engine, tile: u32, name: &[u8], value: f32) {
    let p = nvse::table::guest_str(e, name);
    let id = e.call(0x00a0_1860, &args![p]).u32();
    e.call(0x00a0_12d0, &args![tile, id, value.to_bits(), 1u32]);
}
fn display(e: &mut Engine) {
    let s = snapshot(e);
    if s.tile == 0 {
        return;
    }
    let mut text = s.editor.text;
    if s.editor.active && s.editor.cursor >= 0 && usize::from(s.editor.cursor as u16) <= text.len()
    {
        text.insert(
            s.editor.cursor as usize,
            if s.editor.phase { b'|' } else { b' ' },
        );
        trait_float(e, s.tile, b"_CaretIndex", f32::from(s.editor.cursor));
    }
    string(e, s.tile, &text);
}
fn focus(e: &mut Engine, on: bool) {
    change(e, |s| {
        s.editor.active = on;
        s.editor.phase = false;
        s.editor.last = 0;
    });
    let tile = snapshot(e).tile;
    trait_float(e, tile, b"_IsActive", if on { 1. } else { 0. });
    if !on {
        display(e);
    }
}
fn refresh(e: &mut Engine, menu: u32) {
    // 10041540: the existing item-use handler refreshes the closed keyring;
    // the open keyring uses its own filter and original sorting routine.
    let root = e.mem.u32(menu + 4);
    let keyring = e.mem.u32(0x011d_9eb8);
    if e.call(0x00a0_11b0, &args![root, keyring]).st0 != 0. {
        e.call(0x0072_9fe0, &args![menu + 0xb8, KEY_FILTER]);
        e.call(
            0x0073_14c0,
            &args![menu + 0xb8, 0x0078_0c00u32, u32::MAX, 0x7fff_ffffu32],
        );
    } else {
        force_refresh(e);
        let interface = e.mem.u32(0x011d_8a80);
        if e.mem.u32(interface + 0xbc) != 0 {
            e.vcall(menu, 0x10, &args![0x1du32, 0u32]);
        }
    }
}
fn refresh_branch(e: &mut Engine, _: &[u32]) -> Option<u32> {
    snapshot(e).refreshing.then_some(1)
}
fn force_refresh(e: &mut Engine) {
    change(e, |s| s.refreshing = true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        e.call(0x0070_4af0, &args![])
    }));
    change(e, |s| s.refreshing = false);
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
fn previous_word(text: &[u8], cursor: usize) -> usize {
    let mut p = cursor.min(text.len());
    while p > 0 && !text[p - 1].is_ascii_alphanumeric() {
        p -= 1;
    }
    while p > 0 && text[p - 1].is_ascii_alphanumeric() {
        p -= 1;
    }
    cursor - p
}
fn following_word(text: &[u8], cursor: usize) -> usize {
    let mut p = cursor.saturating_add(1).min(text.len());
    let start = p;
    while p < text.len() && text[p].is_ascii_alphanumeric() {
        p += 1;
    }
    while p < text.len() && !text[p].is_ascii_alphanumeric() {
        p += 1;
    }
    p - start
}
/// 10052BD0 / 10052EA0. Returns handled; changes the real text, never its caret copy.
fn edit(editor: &mut Editor, key: u32, control: bool, paste: Option<&[u8]>) -> bool {
    editor.phase = false;
    editor.last = 0;
    let cursor = editor.cursor.max(0) as usize;
    match key {
        0x8000_0000 => {
            if control {
                // The source calls its mutating RemoveWord twice, testing the first result.
                let mut buffer = editor.text.clone();
                buffer.push(0);
                let mut length = editor.text.len();
                let mut remove = || {
                    let count = previous_word(&buffer, cursor);
                    let result = cursor.saturating_sub(count);
                    let mut pos = cursor;
                    for _ in 0..count {
                        pos = pos.saturating_sub(1);
                        if pos < length {
                            buffer.copy_within(pos + 1..=length, pos);
                            length -= 1;
                        }
                    }
                    result
                };
                let first = remove();
                editor.cursor = if (first as i16) < 0 {
                    0
                } else {
                    remove() as i16
                };
                editor.text = buffer[..length].to_vec();
            } else if cursor > 0 {
                let at = (cursor - 1).min(editor.text.len());
                if at < editor.text.len() {
                    editor.text.remove(at);
                }
                editor.cursor = at as i16;
            }
        }
        0x8000_0007 => {
            let count = if control {
                following_word(&editor.text, cursor) + 1
            } else {
                1
            };
            let end = cursor.saturating_add(count).min(editor.text.len());
            if cursor < end {
                editor.text.drain(cursor..end);
            }
            editor.cursor = cursor.min(editor.text.len()) as i16;
        }
        0x8000_0001 => {
            editor.cursor = cursor.saturating_sub(if control {
                previous_word(&editor.text, cursor)
            } else {
                1
            }) as i16;
        }
        0x8000_0002 => {
            editor.cursor = cursor
                .saturating_add(if control {
                    following_word(&editor.text, cursor) + 1
                } else {
                    1
                })
                .min(editor.text.len()) as i16;
        }
        0x8000_0005 => editor.cursor = 0,
        0x8000_0006 => editor.cursor = editor.text.len() as i16,
        0x8000_0008 => editor.active = false,
        unknown if unknown >= 0x8000_0000 => return false,
        9 | 96 | 13 | 10 => return false,
        _ => {
            let ordinary = [key as u8];
            let input = if control {
                if key | 0x20 == u32::from(b'v') {
                    paste.unwrap_or_default()
                } else {
                    &[]
                }
            } else {
                &ordinary
            };
            let input = input
                .iter()
                .take(200)
                .copied()
                .filter(|b| !matches!(b, 9 | 96 | 13 | 10))
                .collect::<Vec<_>>();
            let at = cursor.min(editor.text.len());
            editor.text.splice(at..at, input.iter().copied());
            editor.cursor = (at + input.len()) as i16;
        }
    }
    true
}
fn key(e: &mut Engine, w: &[u32]) -> Ret {
    let menu = w[0];
    let key = w[1];
    let control = ctrl(e);
    let s = snapshot(e);
    if control && matches!(key | 0x20, 0x66 | 0x72) {
        if key | 0x20 == 0x72 {
            change(e, |s| s.editor.text.clear());
            string(e, s.tile, b"");
            change(e, |s| s.editor.cursor = 0);
            focus(e, false);
            refresh(e, menu);
            return ret(1);
        }
        if (!s.editor.active && s.clear) || s.remove {
            change(e, |s| {
                s.editor.text.clear();
                s.editor.cursor = 0;
            });
            string(e, s.tile, b"");
        }
        let active = !s.editor.active;
        focus(e, active);
        if active {
            change(e, |s| s.hooked = true);
        }
        let now = snapshot(e);
        if now.editor.active || now.remove {
            refresh(e, menu);
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
            change(e, |s| s.editor = editor);
            if !active {
                focus(e, false);
            } else if key < 0x8000_0000 || matches!(key, 0x8000_0000 | 0x8000_0007) {
                string(e, s.tile, &snapshot(e).editor.text);
            }
            refresh(e, menu);
            return ret(1);
        }
    }
    e.call(s.original_key, w)
}
fn idle(e: &mut Engine, _: &[u32]) -> Option<u32> {
    let s = snapshot(e);
    if s.editor.active {
        let now = e.call(0x0045_7fe0, &args![]).u32();
        if now.wrapping_sub(s.editor.last) > 530 {
            change(e, |s| {
                s.editor.phase = !s.editor.phase;
                s.editor.last = now;
            });
            display(e);
        }
    }
    None
}
fn close(e: &mut Engine, _: &[u32]) -> Option<u32> {
    pipboy::close(e);
    let tile = snapshot(e).tile;
    if tile != 0 {
        change(e, |s| {
            s.editor = Editor::default();
            s.hooked = false;
        });
        string(e, tile, b"");
        focus(e, false);
    }
    None
}
#[cfg(test)]
mod tests;

pub(super) fn recipe_lifecycle(e: &mut Engine, opening: bool, phase: u32) {
    simple::recipe_lifecycle(e, opening, phase);
}
