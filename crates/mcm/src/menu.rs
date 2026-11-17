//! The menu lookup MCM.dll carries: its own copy of NVSE 2's (beta 12 era) path code, not
//! xNVSE's (`crates/nvse` `commands/ui.rs`), which differs in small ways. From the DLL's code:
//! - the menu (`InterfaceManager::GetMenuByPath`): the text up to the first `\` or `/`, as
//!   `TraitNameToID("&" + name + ";")`; a menu only when that id less 1001 is at most 0x53 and
//!   below the menu array's count (its word at `011F3512`) and its entry is not null;
//! - the component (`Tile::GetComponent`): the rest cut at `\` and `/`, empty parts skipped; each
//!   part a child of the tile so far ([`child`]); a part that names no child is the trait, when it
//!   is the last part (a later part: nothing); when every part names a child there is no trait;
//! - the child (`Tile::GetChild`): `name[:n]` (a colon not first; `n` by `atoi`) for the `n`-th
//!   child of that name (none for a negative `n`), `*` alone for any child, the name compared as
//!   `_stricmp` compares (ASCII letters without case); the children walked as NVSE's `tList`
//!   walks the game's list (the first child, then the last back to the second);
//! - the value (`Tile::GetValue`): the tile's trait values searched in order for
//!   `TraitNameToID(trait)`.
//!
//! What a script cannot make the game do here, where the DLL could: a child without a name (the
//! DLL hands `_stricmp` a null: a crash) matches nothing; a list or value array that does not end
//! is walked [`MAX_WALK`] steps; a path of [`PATH_BUFFER`] bytes or more (the DLL copies it into
//! a 16 KB stack buffer without a bound) finds nothing.

use engine_core::prelude::*;

/// `TraitNameToID` (`0x00A01860`, `cdecl`), `Tile::SetFloat` (`0x00A012D0`) and
/// `Tile::SetString` (`0x00A01350`, `thiscall`: the trait's id, the value, propagate).
pub const TRAIT_NAME_TO_ID: u32 = 0x00a0_1860;
pub const SET_FLOAT: u32 = 0x00a0_12d0;
pub const SET_STRING: u32 = 0x00a0_1350;
/// The menus' tiles by menu id from 1001 (`*0x011F350C`) and the array's count (its word at
/// `0x011F3512`).
pub const MENU_ARRAY: u32 = 0x011f_350c;
pub const MENU_COUNT: u32 = 0x011f_3512;
pub const MENU_MIN: u32 = 1001;
pub const MENU_SPAN: u32 = 0x53;

/// A `Tile`: its children (`+4`: head, `+8`: tail; a node is next `+0`, previous `+4`, tile
/// `+8`), its trait values (data `+0x14`, count `+0x18`), its name (`+0x20`). A `Tile::Value`:
/// its id `+0`, its tile `+4`, its number `+8`.
const CHILDREN: u32 = 4;
const VALUES: u32 = 0x14;
const VALUE_COUNT: u32 = 0x18;
const NAME: u32 = 0x20;
const VALUE_TILE: u32 = 4;
const VALUE_NUM: u32 = 8;

/// The DLL's path buffers (`char[0x4000]` on its stack).
pub const PATH_BUFFER: usize = 0x4000;
/// The most steps a child list or a value array is walked (a broken one ends there).
pub const MAX_WALK: u32 = 1 << 16;

/// A guest copy of `text` for the length of `f` (a C string on the stack).
pub fn with_text<T>(e: &mut Engine, text: &[u8], f: impl FnOnce(&mut Engine, u32) -> T) -> T {
    let size = (text.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, at| {
        e.mem.write(at.addr(), text);
        e.mem.set_u8(at.addr() + text.len() as u32, 0);
        f(e, at.addr())
    })
}

/// `TraitNameToID(name)`.
pub fn trait_id(e: &mut Engine, name: &[u8]) -> u32 {
    with_text(e, name, |e, at| e.call(TRAIT_NAME_TO_ID, &args![at]).u32())
}

/// The first `\` or `/` (`strpbrk(path, "\\/")`).
fn separator(path: &[u8]) -> Option<usize> {
    path.iter().position(|&b| b == b'\\' || b == b'/')
}

/// The menu tile `path` names and where its first separator is; `None` without one or a menu.
pub fn menu_of(e: &mut Engine, path: &[u8]) -> Option<(u32, usize)> {
    let slash = separator(path)?;
    let name = [b"&", &path[..slash], b";"].concat();
    let index = trait_id(e, &name).wrapping_sub(MENU_MIN);
    if index > MENU_SPAN || index >= u32::from(e.mem.u16(MENU_COUNT)) {
        return None;
    }
    let array = e.mem.u32(MENU_ARRAY);
    if array == 0 {
        return None;
    }
    let menu = e.mem.u32(array.wrapping_add(4 * index));
    (menu != 0).then_some((menu, slash))
}

/// `atoi` (the C runtime's: leading white space, a sign, digits; out of range: the limit).
pub fn atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = (v * 10 + i64::from(s[i] - b'0')).min(i64::from(i32::MAX) + 1);
        i += 1;
    }
    if negative {
        (-v).max(i64::from(i32::MIN)) as i32
    } else {
        v.min(i64::from(i32::MAX)) as i32
    }
}

/// The child of `tile` named `wanted` (`name[:n]`, `*`), 0 for none.
pub fn child(e: &mut Engine, tile: u32, wanted: &[u8]) -> u32 {
    let (name, index) = match wanted.iter().position(|&b| b == b':') {
        Some(at) if at > 0 => (&wanted[..at], atoi(&wanted[at + 1..])),
        _ => (wanted, 0),
    };
    let any = name == b"*";
    let mut seen: i32 = 0;
    for candidate in children(e, tile) {
        if any || name_is(e, candidate, name) {
            if seen == index {
                return candidate;
            }
            seen = seen.wrapping_add(1);
        }
    }
    0
}

/// `tile`'s children in the order the DLL walks them (NVSE's `tList` over the game's list: node
/// `+0` data, `+4` next, from `tile + 4`; the data's `+8` the child), at most [`MAX_WALK`] steps.
fn children(e: &Engine, tile: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut node = tile.wrapping_add(CHILDREN);
    for _ in 0..MAX_WALK {
        let (data, next) = (e.mem.u32(node), e.mem.u32(node.wrapping_add(4)));
        if data == 0 && next == 0 {
            break;
        }
        if data != 0 {
            let candidate = e.mem.u32(data.wrapping_add(8));
            if candidate != 0 {
                out.push(candidate);
            }
        }
        if next == 0 {
            break;
        }
        node = next;
    }
    out
}

/// The indices whose [`path`] in group `kind` can name a tile that is there: 0 (the group's own
/// place) and each `n` of a child of the group named `<item><n>` (`Option3`, `option3`: names are
/// compared without case; `n` as the path writes it, so `Option03` names nothing), `SubMenu<m>`
/// standing for `m + 23` in group 5. For a group whose path takes no index, `None`: every index
/// names the same place. An empty list when the group's tile is not there.
pub fn indices_with_tiles(e: &mut Engine, kind: i32) -> Option<Vec<i32>> {
    let items: &[(&[u8], i64)] = match kind {
        1 => &[(b"Option", 0)],
        3 => &[(b"Item", 0)],
        5 => &[(b"Mod", 0), (b"SubMenu", 23)],
        _ => return None,
    };
    let group: &[u8] = match kind {
        1 => b"MCM_Options",
        3 => b"MCM_List",
        _ => b"MCM_ModList",
    };
    let mut out = Vec::new();
    let Some((menu, _)) = menu_of(e, b"StartMenu/") else {
        return Some(out);
    };
    let mcm = child(e, menu, b"MCM");
    let tile = if mcm == 0 { 0 } else { child(e, mcm, group) };
    if tile == 0 {
        return Some(out);
    }
    out.push(0);
    for c in children(e, tile) {
        let text = e.mem.u32(c.wrapping_add(NAME));
        if text == 0 {
            continue;
        }
        let name = nvse::table::guest_text(e, text);
        for &(item, offset) in items {
            let Some(number) = name
                .get(item.len()..)
                .filter(|_| name[..item.len()].eq_ignore_ascii_case(item))
            else {
                continue;
            };
            let Some(n) = std::str::from_utf8(number)
                .ok()
                .and_then(|t| t.parse::<i32>().ok().filter(|n| n.to_string() == t))
            else {
                continue;
            };
            // Group 5: `Mod<i>` for i up to 23 (not 0), `SubMenu<i - 23>` above it.
            let index = i64::from(n) + offset;
            let named = match (kind, offset) {
                (5, 0) => index != 0 && index <= 23,
                (5, _) => index > 23,
                _ => index != 0,
            };
            if let (true, Ok(index)) = (named, i32::try_from(index)) {
                out.push(index);
            }
        }
    }
    Some(out)
}

/// Whether `tile`'s name is `wanted` as `_stricmp` compares (ASCII letters without case); a tile
/// without a name is none.
fn name_is(e: &Engine, tile: u32, wanted: &[u8]) -> bool {
    let text = e.mem.u32(tile.wrapping_add(NAME));
    if text == 0 {
        return false;
    }
    nvse::table::guest_text(e, text).eq_ignore_ascii_case(wanted)
}

/// The tile at `rest` below `tile` and the trait its last part names; `None` when a part on the
/// way names no child, or every part does (no trait).
pub fn component(e: &mut Engine, tile: u32, rest: &[u8]) -> Option<(u32, Vec<u8>)> {
    let parts: Vec<&[u8]> = rest
        .split(|&b| b == b'\\' || b == b'/')
        .filter(|p| !p.is_empty())
        .collect();
    let mut parent = tile;
    for (i, part) in parts.iter().enumerate() {
        let found = child(e, parent, part);
        if found != 0 {
            parent = found;
            continue;
        }
        return (i + 1 == parts.len()).then(|| (parent, part.to_vec()));
    }
    None
}

/// The tile's value of trait `id`, searched in order; 0 for none.
fn value_of(e: &Engine, tile: u32, id: u32) -> u32 {
    let data = e.mem.u32(tile.wrapping_add(VALUES));
    let count = e.mem.u32(tile.wrapping_add(VALUE_COUNT)).min(MAX_WALK);
    for i in 0..count {
        let value = e.mem.u32(data.wrapping_add(4 * i));
        if value != 0 && e.mem.u32(value) == id {
            return value;
        }
    }
    0
}

/// The `Tile::Value` at `path` (`Menu/tile/.../trait`), 0 when it is not there.
pub fn value_at(e: &mut Engine, path: &[u8]) -> u32 {
    if path.len() >= PATH_BUFFER {
        return 0;
    }
    let Some((menu, slash)) = menu_of(e, path) else {
        return 0;
    };
    let Some((tile, name)) = component(e, menu, &path[slash + 1..]) else {
        return 0;
    };
    let id = trait_id(e, &name);
    value_of(e, tile, id)
}

/// The DLL's "not found" line: in its log, and on the console when the line runs from the
/// console (the thread's TLS byte `+0x268`, all the DLL asks).
fn not_found(e: &mut Engine, path: &[u8]) {
    if crate::from_console(e) {
        with_text(e, path, |e, at| {
            nvse::commands::console_print(e, "Menu trait %s not found", &[at]);
        });
    }
}

/// The number of the trait at `path`; 0 (and the "not found" line) when it is not there.
pub fn get_float(e: &mut Engine, path: &[u8]) -> f32 {
    let value = value_at(e, path);
    if value == 0 {
        not_found(e, path);
        return 0.0;
    }
    e.mem.f32(value + VALUE_NUM)
}

/// `Tile::SetFloat(id, num, true)` on the trait at `path`; the "not found" line without it.
pub fn set_float(e: &mut Engine, path: &[u8], num: f32) {
    let value = value_at(e, path);
    if value == 0 {
        not_found(e, path);
        return;
    }
    let (tile, id) = (e.mem.u32(value + VALUE_TILE), e.mem.u32(value));
    e.call(SET_FLOAT, &args![tile, id, num.to_bits(), 1u32]);
}

/// `Tile::SetString(id, text, true)` on the trait at `path`; the "not found" line without it.
pub fn set_string(e: &mut Engine, path: &[u8], text: &[u8]) {
    let value = value_at(e, path);
    if value == 0 {
        not_found(e, path);
        return;
    }
    let (tile, id) = (e.mem.u32(value + VALUE_TILE), e.mem.u32(value));
    with_text(e, text, |e, at| {
        e.call(SET_STRING, &args![tile, id, at, 1u32]);
    });
}

/// The path MCM builds for a trait (`GetMenuPath`): the group `kind`'s place, the item `index`
/// in it where the group has items, then `setting`.
/// - 1: `MCM_Options/`, or `MCM_Options/Option<i>/` for an index other than 0;
/// - 2: `MCM_Scale/`; 4: `MCM_Trigger/`; 9: `MCM_Info/`; 15: `MCM_Input/`; 17: `MCM_Images/`;
/// - 3: `MCM_List/`, or `MCM_List/Item<i>/`;
/// - 5: `MCM_ModList/`; an index up to 23 (negative ones too) `MCM_ModList/Mod<i>/`, above it
///   `MCM_ModList/SubMenu<i-23>/`;
/// - any other: `StartMenu/MCM/` alone.
///
/// All under `StartMenu/MCM/`; `<i>` the index in decimal, a minus sign when negative.
pub fn path(kind: i32, index: i32, setting: &[u8]) -> Vec<u8> {
    let item = |group: &str, item: &str, i: i32| format!("StartMenu/MCM/{group}/{item}{i}/");
    let base = match kind {
        1 if index != 0 => item("MCM_Options", "Option", index),
        1 => "StartMenu/MCM/MCM_Options/".to_string(),
        2 => "StartMenu/MCM/MCM_Scale/".to_string(),
        3 if index != 0 => item("MCM_List", "Item", index),
        3 => "StartMenu/MCM/MCM_List/".to_string(),
        4 => "StartMenu/MCM/MCM_Trigger/".to_string(),
        5 if index == 0 => "StartMenu/MCM/MCM_ModList/".to_string(),
        5 if index > 23 => item("MCM_ModList", "SubMenu", index - 23),
        5 => item("MCM_ModList", "Mod", index),
        9 => "StartMenu/MCM/MCM_Info/".to_string(),
        15 => "StartMenu/MCM/MCM_Input/".to_string(),
        17 => "StartMenu/MCM/MCM_Images/".to_string(),
        _ => "StartMenu/MCM/".to_string(),
    };
    [base.as_bytes(), setting].concat()
}
