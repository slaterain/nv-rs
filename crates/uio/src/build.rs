//! UI Organizer's builder: the game's structures its XML reader makes and reads (`ResolveTrait`,
//! `AddTraitName`, `GetGMSTValue`, its `NiTMap::Lookup`/`Insert` and `String::Set`,
//! `ui_organizer.h` and `gameapi.cpp` (UIO)), on the game's memory:
//!
//! - the parse storage (`Tile::BuildStorage`, its constructor `00A0AC80`: the main template +0,
//!   the sub-templates' list +4, the template being defined +0xC); a template (`TileTemplate`):
//!   its owner storage +4, its items' list +8 (head, tail +0xC, count +0x10; a node: next +0,
//!   previous +4, item +8);
//! - an item (`TileTemplateItem`, `ParsedXMLTag`): type +0, number +4, text (`BSStringT`: data +8,
//!   length +0xC, size +0xE), id +0x10 (the game's own items, `00A0B950`, also have a line +0x14);
//!   spare items wait in the game's pool of ten (`0x11F329C`, count `0x11F32DC`, as the game's
//!   `00A0B8D0` and `00A0B950` keep it);
//! - the game's trait names map (`0x11F32F4`, `NiTMap`: vtable +4 the bucket of a key, +8 the key
//!   compare, bucket count +4, buckets +8, count +0xC; an entry: next +0, key +4, data +8), the
//!   next user trait id (`0x11A6D74`) and the user traits by id (`0x11F3308`, integer keys);
//! - the game settings (`*0x11C8048 + 0x10C`, the same map, name to setting; a setting's value
//!   +4), as the game's `Interface::TestConstantForGameSettings` (`007073D0`) finds them.

use crate::parse::{Builder, Value, TAG_NAME};
use engine_core::prelude::*;

/// The game's heap and its allocate and free (`thiscall`).
const GAME_HEAP: u32 = 0x011f_6238;
const HEAP_ALLOCATE: u32 = 0x00aa_3e40;
const HEAP_FREE: u32 = 0x00aa_4060;
/// `Tile::BuildStorage::BuildStorage` (`fastcall`, UIO's `ADDR_InitXMLtoTileData`).
pub const BUILD_STORAGE_CONSTRUCT: u32 = 0x00a0_ac80;
/// A list node from the game's node pool (UIO's `ADDR_GetAvailableLinkedNode`).
pub const LIST_NEW_NODE: u32 = 0x0043_a010;
/// `NiTPointerList::RemoveTail` (`fastcall` on the list): the tail's item.
pub const LIST_REMOVE_TAIL: u32 = 0x00ae_8010;
/// A storage's sub-template named so (`thiscall`, null when none), and a new one made and listed.
pub const FIND_TEMPLATE: u32 = 0x00a0_af10;
pub const NEW_TEMPLATE: u32 = 0x00a0_ae70;
pub const TRAIT_MAP: u32 = 0x011f_32f4;
pub const USER_NEXT_ID: u32 = 0x011a_6d74;
pub const TRAITS_BY_ID: u32 = 0x011f_3308;
pub const SETTINGS_OWNER: u32 = 0x011c_8048;
pub const SETTINGS_MAP: u32 = 0x10c;
pub const ITEM_POOL: u32 = 0x011f_329c;
pub const ITEM_POOL_COUNT: u32 = 0x011f_32dc;
const ITEM_POOL_SIZE: u32 = 10;
/// `eNone`: the id of an item that has none.
const NO_ID: u32 = 0x8000_0000;
/// 999 (`0x4479C000`): a `<template>` tag's id.
const TEMPLATE_ID: u32 = 0x4479_c000;
/// The first user trait id, and the trait and action id ranges `ResolveTrait` types by.
const FIRST_USER_ID: i32 = 10000;
const FIRST_TRAIT: i32 = 0xfa1;
const LAST_TRAIT: i32 = 0x101d;
const FIRST_ACTION: i32 = 0x7d0;
/// `kAction_MAX` (UIO, with its 17 new operators and three left unused).
const END_ACTION: i32 = 0x7fb;
const FIRST_TILE: i32 = 0x385;
const LAST_TILE: i32 = 0x38c;
/// The size of an item UIO makes. UIO makes 0x14 bytes; the game's pool, which UIO's items join,
/// also hands items to the game's own reader (`00A0B950`), which writes a line at +0x14: here
/// they are made at the game's 0x18 bytes (a deliberate difference, against that overrun).
const ITEM_SIZE: u32 = 0x18;
/// The most entries a map bucket is walked for (a broken map ends the walk: not found).
const MAX_WALK: u32 = 1 << 20;
/// The most items one file's parse makes; past it the parse stops (UIO has no limit: a huge
/// file would fill the game's memory).
pub const MAX_ITEMS: u32 = 1 << 20;

fn allocate(e: &mut Engine, size: u32) -> u32 {
    e.call(HEAP_ALLOCATE, &args![GAME_HEAP, size]).u32()
}

fn free(e: &mut Engine, block: u32) {
    e.call(HEAP_FREE, &args![GAME_HEAP, block]);
}

/// The longest name put on the game's stack for a call ([`with_text`]); a longer one goes on the
/// game's heap.
const STACK_TEXT: usize = 0x1000;

/// `f` given `text` with its ending zero in the game's memory: on the game's stack for a short
/// text, else in a block of the game's heap freed after the call. UIO hands the game a pointer
/// into its own buffer and copies nothing, so a name of any length a file holds (a text is up to
/// 64 MiB) must reach the game: on the stack (1 MiB for locals) a long one would stop the game.
/// `R::default()` (not found, no template) when the heap gives no block.
fn with_text<R: Default>(e: &mut Engine, text: &[u8], f: impl FnOnce(&mut Engine, u32) -> R) -> R {
    if text.len() <= STACK_TEXT {
        return nvse::events::with_guest_text(e, text, f);
    }
    let block = allocate(e, text.len() as u32 + 1);
    if block == 0 {
        return R::default();
    }
    e.mem.write(block, text);
    e.mem.set_u8(block + text.len() as u32, 0);
    let r = f(e, block);
    free(e, block);
    r
}

fn f32_at(e: &Engine, at: u32) -> f32 {
    f32::from_bits(e.mem.u32(at))
}

/// An item's text (`BSStringT` data +8), `None` for a null pointer.
fn text_of(e: &Engine, item: u32) -> Option<Vec<u8>> {
    let data = e.mem.u32(item + 8);
    (data != 0).then(|| nvse::table::guest_text(e, data))
}

/// `cvttss2si`: a float to an integer, truncated (0x80000000 when it does not fit).
fn truncate(v: f32) -> i32 {
    if v.is_nan() || v >= 2147483648.0 || v < -2147483648.0 {
        i32::MIN
    } else {
        v as i32
    }
}

// Translated from UIO 2.30 (426d963), gameapi.cpp: NiTMap::Lookup.
/// The entry of `key` (a text) in the map at `map`: its bucket from the map's hash, then the
/// map's compare along the bucket; 0 when it is not there.
pub fn map_lookup(e: &mut Engine, map: u32, key: &[u8]) -> u32 {
    let vtable = e.mem.u32(map);
    if vtable == 0 {
        return 0;
    }
    with_text(e, key, |e, k| lookup_at(e, map, vtable, k))
}

fn lookup_at(e: &mut Engine, map: u32, vtable: u32, key: u32) -> u32 {
    let (hash, equal) = (e.mem.u32(vtable + 4), e.mem.u32(vtable + 8));
    let bucket = e.call(hash, &args![map, key]).u32();
    let buckets = e.mem.u32(map + 8);
    let mut entry = e.mem.u32(buckets.wrapping_add(bucket.wrapping_mul(4)));
    for _ in 0..MAX_WALK {
        if entry == 0 {
            return 0;
        }
        let entry_key = e.mem.u32(entry + 4);
        if e.call(equal, &args![map, key, entry_key]).u32() & 0xff != 0 {
            return entry;
        }
        entry = e.mem.u32(entry);
    }
    0
}

// Translated from UIO 2.30 (426d963), gameapi.cpp: NiTMap::Insert.
/// The entry of `key` in the map, made when it is not there (a node from the game's pool, data 0,
/// the key copied onto the game's heap, first in its bucket, the count raised): the entry and
/// whether it is new. `key` is a guest text, or for the by-id map ([`TRAITS_BY_ID`]) the number.
fn map_insert(e: &mut Engine, map: u32, key: u32) -> (u32, bool) {
    let vtable = e.mem.u32(map);
    let (hash, equal) = (e.mem.u32(vtable + 4), e.mem.u32(vtable + 8));
    let bucket = e.call(hash, &args![map, key]).u32();
    let buckets = e.mem.u32(map + 8);
    let slot = buckets.wrapping_add(bucket.wrapping_mul(4));
    let mut entry = e.mem.u32(slot);
    for _step in 0..MAX_WALK {
        if entry == 0 {
            break;
        }
        let entry_key = e.mem.u32(entry + 4);
        if e.call(equal, &args![map, key, entry_key]).u32() & 0xff != 0 {
            return (entry, false);
        }
        entry = e.mem.u32(entry);
    }
    let node = e.call(LIST_NEW_NODE, &args![map]).u32();
    if node == 0 {
        return (0, false);
    }
    e.mem.set_u32(node + 8, 0);
    let stored = if map == TRAITS_BY_ID {
        key
    } else {
        let text = nvse::table::guest_text(e, key);
        let copy = allocate(e, text.len() as u32 + 1);
        if copy != 0 {
            e.mem.write(copy, &text);
            e.mem.set_u8(copy + text.len() as u32, 0);
        }
        copy
    };
    e.mem.set_u32(node + 4, stored);
    e.mem.set_u32(node, e.mem.u32(slot));
    e.mem.set_u32(slot, node);
    e.mem
        .set_u32(map + 0xc, e.mem.u32(map + 0xc).wrapping_add(1));
    (node, true)
}

// Translated from UIO 2.30 (426d963), gameapi.cpp: String::Set.
/// UIO's `BSStringT` set: the length (16 bits) written; a text that fits the buffer copied into
/// it, else a new buffer of the text's size on the game's heap (the old one freed); an empty text
/// leaves an empty buffer; no text (a null pointer) frees the buffer. A text over 65,535 bytes:
/// UIO copies the 16-bit length's bytes and no end; here the whole text and its end are copied (a
/// deliberate difference).
pub fn string_set(e: &mut Engine, string: u32, text: Option<&[u8]>) {
    let Some(text) = text else {
        e.mem.set_u16(string + 4, 0);
        e.mem.set_u16(string + 6, 0);
        let data = e.mem.u32(string);
        if data != 0 {
            e.mem.set_u32(string, 0);
            free(e, data);
        }
        return;
    };
    let len = text.len() as u32;
    e.mem.set_u16(string + 4, len as u16);
    let data = e.mem.u32(string);
    if len == 0 {
        if data != 0 {
            e.mem.set_u8(data, 0);
        }
        return;
    }
    let mut at = data;
    let fits = e.mem.u16(string + 6) as u32 >= len && len <= 0xffff;
    if !fits {
        e.mem.set_u16(string + 6, len as u16);
        if data != 0 {
            free(e, data);
        }
        at = allocate(e, len + 1);
        e.mem.set_u32(string, at);
        if at == 0 {
            e.mem.set_u32(string + 4, 0);
            return;
        }
    }
    e.mem.write(at, text);
    e.mem.set_u8(at + len, 0);
}

/// The builder on a parse storage.
pub struct GameBuilder<'a> {
    pub e: &'a mut Engine,
    pub storage: u32,
    /// Items made so far, and the most one parse makes ([`MAX_ITEMS`]; smaller in the tests).
    items: u32,
    max_items: u32,
}

impl<'a> GameBuilder<'a> {
    pub fn new(e: &'a mut Engine, storage: u32) -> Self {
        GameBuilder {
            e,
            storage,
            items: 0,
            max_items: MAX_ITEMS,
        }
    }

    // Translated from UIO 2.30 (426d963), ui_organizer.h: FreeExtraString.
    /// An item given back: to the game's pool while it has room, else freed with its text.
    fn release(&mut self, item: u32) {
        let e = &mut *self.e;
        let count = e.mem.u32(ITEM_POOL_COUNT);
        if count < ITEM_POOL_SIZE {
            e.mem.set_u32(ITEM_POOL + count * 4, item);
            e.mem.set_u32(ITEM_POOL_COUNT, count + 1);
            return;
        }
        if item == 0 {
            return;
        }
        let text = e.mem.u32(item + 8);
        free(e, item);
        if text != 0 {
            free(e, text);
        }
    }

    /// The list's tail removed (`00AE8010`) and its item given back.
    fn pop_tail(&mut self, list: u32) {
        let item = self.e.call(LIST_REMOVE_TAIL, &args![list]).u32();
        self.release(item);
    }

    /// The item put at the list's end (a node from the game's pool).
    fn append(&mut self, list: u32, item: u32) -> bool {
        let e = &mut *self.e;
        let node = e.call(LIST_NEW_NODE, &args![list]).u32();
        if node == 0 {
            return false;
        }
        e.mem.set_u32(node + 8, item);
        e.mem.set_u32(node, 0);
        let tail = e.mem.u32(list + 4);
        e.mem.set_u32(node + 4, tail);
        let link = if tail == 0 { list } else { tail };
        e.mem.set_u32(link, node);
        e.mem.set_u32(list + 4, node);
        e.mem.set_u32(list + 8, e.mem.u32(list + 8).wrapping_add(1));
        true
    }
}

impl Builder for GameBuilder<'_> {
    fn lookup_trait(&mut self, name: &[u8]) -> Option<u32> {
        let entry = map_lookup(self.e, TRAIT_MAP, name);
        (entry != 0).then(|| self.e.mem.u32(entry + 8))
    }

    // Translated from UIO 2.30 (426d963), ui_organizer.h: AddTraitName.
    fn add_trait_name(&mut self, name: &[u8]) -> Option<u32> {
        if name.first() != Some(&b'_') {
            return self.lookup_trait(name);
        }
        let e = &mut *self.e;
        if e.mem.u32(TRAIT_MAP) == 0 {
            return None;
        }
        let (entry, new) = with_text(e, name, |e, k| map_insert(e, TRAIT_MAP, k));
        if entry == 0 {
            return None;
        }
        if new {
            let id = e.mem.u32(USER_NEXT_ID);
            e.mem.set_u32(USER_NEXT_ID, id.wrapping_add(1));
            e.mem.set_u32(entry + 8, id);
            // The number after the name's last `_` (not its first byte): -1 when nothing follows.
            if let Some(at) = name[1..].iter().rposition(|&c| c == b'_') {
                let at = at + 1;
                if at != 1 {
                    let tail = &name[at + 1..];
                    let index = if tail.is_empty() {
                        -1
                    } else {
                        crate::parse::str_to_int(tail)
                    };
                    let (by_id, _) = map_insert(e, TRAITS_BY_ID, id);
                    if by_id != 0 {
                        e.mem.set_u32(by_id + 8, index as u32);
                    }
                }
            }
        }
        Some(e.mem.u32(entry + 8))
    }

    // Translated from UIO 2.30 (426d963), ui_organizer.h: GetGMSTValue.
    fn game_setting(&mut self, name: &[u8]) -> Option<Value> {
        let e = &mut *self.e;
        let owner = e.mem.u32(SETTINGS_OWNER);
        if owner == 0 {
            return None;
        }
        let entry = map_lookup(e, owner + SETTINGS_MAP, name);
        if entry == 0 {
            return None;
        }
        let setting = e.mem.u32(entry + 8);
        if setting == 0 {
            return None;
        }
        let value = e.mem.u32(setting + 4);
        match name.first().map(|c| c | 0x20) {
            Some(b's') if value == 0 => Some(Value::NoText),
            Some(b's') => Some(Value::Str(nvse::table::guest_text(e, value))),
            Some(b'f') => Some(Value::Num(f32::from_bits(value))),
            Some(b'i') => Some(Value::Num(value as i32 as f32)),
            _ => None,
        }
    }

    // Translated from UIO 2.30 (426d963), ui_organizer.h: ResolveTrait.
    fn resolve_trait(&mut self, value_type: u32, value: Value) -> bool {
        if self.items >= self.max_items {
            return false;
        }
        self.items += 1;
        let storage = self.storage;
        let current = self.e.mem.u32(storage + 0xc);
        let template = if current != 0 {
            current
        } else {
            self.e.mem.u32(storage)
        };
        if template == 0 {
            return false;
        }
        let id = match value {
            Value::Id(id) => id,
            _ => NO_ID,
        };
        let number = (id as i32 as f32).to_bits();
        // An item from the pool, else a new one with an empty text.
        let item = {
            let e = &mut *self.e;
            let count = e.mem.u32(ITEM_POOL_COUNT);
            if count != 0 && count <= ITEM_POOL_SIZE {
                e.mem.set_u32(ITEM_POOL_COUNT, count - 1);
                e.mem.u32(ITEM_POOL + (count - 1) * 4)
            } else {
                let item = allocate(e, ITEM_SIZE);
                if item == 0 {
                    return false;
                }
                e.mem.set_u32(item + 8, 0);
                e.mem.set_u32(item + 0xc, 0);
                e.mem.set_u32(item + 0x14, 0);
                item
            }
        };
        if item == 0 {
            return false;
        }
        {
            let e = &mut *self.e;
            e.mem.set_u32(item, value_type);
            e.mem.set_u32(item + 4, number);
            e.mem.set_u32(item + 0x10, id);
            match &value {
                Value::Id(_) => {}
                Value::Num(n) => {
                    e.mem.set_u32(item + 4, n.to_bits());
                    let text = e.mem.u32(item + 8);
                    if text != 0 {
                        e.mem.set_u32(item + 8, 0);
                        e.mem.set_u32(item + 0xc, 0);
                        free(e, text);
                    }
                }
                Value::Str(s) => string_set(e, item + 8, Some(s)),
                Value::NoText => string_set(e, item + 8, None),
            }
        }
        let e = &mut *self.e;
        // The template's last three items.
        let tail = e.mem.u32(template + 0xc);
        let mut node = tail;
        let mut last = [0u32; 3];
        for slot in last.iter_mut() {
            if node == 0 {
                break;
            }
            *slot = e.mem.u32(node + 8);
            node = e.mem.u32(node + 4);
        }
        let [last1, last2, last3] = last;
        let owner = e.mem.u32(template + 4);
        let list = template + 8;
        // The number local (`[ebp-8]`): the id as a float, not the item's number.
        let my_number = number;
        if value_type == TAG_NAME {
            if last1 != 0 && e.mem.u32(last1) == 0 {
                if e.mem.u32(last1 + 4) == TEMPLATE_ID {
                    // `name="x"` on `<template>`: the storage's sub-template of that name (made
                    // when none), unless one is being defined.
                    if e.mem.u32(owner + 0xc) == 0 {
                        let name = match &value {
                            Value::Str(s) => s.clone(),
                            _ => Vec::new(),
                        };
                        let found = with_text(e, &name, |e, n| {
                            let t = e.call(FIND_TEMPLATE, &args![owner, n]).u32();
                            if t != 0 {
                                t
                            } else {
                                e.call(NEW_TEMPLATE, &args![owner, n]).u32()
                            }
                        });
                        e.mem.set_u32(owner + 0xc, found);
                    }
                    self.pop_tail(list);
                    self.release(item);
                    return true;
                }
                let kind = truncate(f32_at(e, last1 + 4));
                if (FIRST_TILE..=LAST_TILE).contains(&kind) {
                    // `name="x"` on a tile's tag: the tile starts here, with that name.
                    e.mem.set_u32(last1, 6);
                    let name = match &value {
                        Value::Str(s) => Some(s.as_slice()),
                        _ => None,
                    };
                    string_set(e, last1 + 8, name);
                    self.release(item);
                    return true;
                }
            }
            e.mem.set_u32(item, 8);
            e.mem.set_u32(item + 0x10, TAG_NAME);
            return self.append(list, item);
        }
        if value_type == 1 {
            if my_number == TEMPLATE_ID && e.mem.u32(owner + 0xc) != 0 {
                // `</template>`: the definition ends.
                e.mem.set_u32(owner + 0xc, 0);
                self.release(item);
                return true;
            }
            if last1 != 0 {
                let t1 = e.mem.u32(last1);
                let t2 = if last2 != 0 { e.mem.u32(last2) } else { 0 };
                if t1 == crate::parse::TAG_VALUE
                    && last2 != 0
                    && (t2 == 4 || t2 == 2)
                    && e.mem.u32(last2 + 4) == my_number
                {
                    // `<trait>value</trait>`: one simple trait (8) or simple action (9).
                    let kind = if e.mem.i32(item + 0x10) >= FIRST_USER_ID {
                        8
                    } else {
                        let n = truncate(f32_at(e, item + 4));
                        if n < FIRST_TRAIT {
                            if (FIRST_ACTION..END_ACTION).contains(&n) {
                                9
                            } else {
                                u32::MAX
                            }
                        } else if n > LAST_TRAIT {
                            u32::MAX
                        } else {
                            8
                        }
                    };
                    e.mem.set_u32(last2, kind);
                    let n = truncate(f32_at(e, last2 + 4));
                    e.mem.set_u32(last2 + 0x10, n as u32);
                    e.mem.set_u32(last2 + 4, e.mem.u32(last1 + 4));
                    let s = text_of(e, last1);
                    string_set(e, last2 + 8, s.as_deref());
                    self.pop_tail(list);
                    self.release(item);
                    return true;
                }
                if t1 == crate::parse::TAG_TRAIT
                    && last2 != 0
                    && t2 == crate::parse::TAG_SRC
                    && last3 != 0
                    && e.mem.u32(last3) == 2
                    && e.mem.u32(last3 + 4) == my_number
                {
                    // `<action src="x" trait="y"/>`: one link (10).
                    e.mem.set_u32(last3, 10);
                    let n = truncate(f32_at(e, last3 + 4));
                    e.mem.set_u32(last3 + 0x10, n as u32);
                    e.mem.set_u32(last3 + 4, e.mem.u32(last1 + 4));
                    let s = text_of(e, last2);
                    string_set(e, last3 + 8, s.as_deref());
                    self.pop_tail(list);
                    self.pop_tail(list);
                    self.release(item);
                    return true;
                }
            }
        }
        // `jmpLbl_11`: an open or close tag typed by its id.
        let e = &mut *self.e;
        let t = e.mem.u32(item);
        if t <= 1 {
            let id = e.mem.i32(item + 0x10);
            let trait_range = id >= FIRST_USER_ID || (FIRST_TRAIT..=LAST_TRAIT).contains(&id);
            if trait_range {
                e.mem.set_u32(item, if t == 0 { 4 } else { 5 });
            } else if (FIRST_ACTION..END_ACTION).contains(&id) {
                if t == 0 {
                    e.mem.set_u32(item, 2);
                } else if last1 != 0 {
                    e.mem.set_u32(item, 3);
                }
            } else if t != 0 {
                let n = truncate(f32_at(e, item + 4));
                if (FIRST_TILE..=LAST_TILE).contains(&n) {
                    e.mem.set_u32(item, 7);
                }
            }
        }
        self.append(list, item)
    }
}

/// UIO's parse of `text` into a new storage (`ResolveXMLFileHook` from `notDebug`): the storage
/// made on the game's heap and constructed, then the scan; 0 when no storage can be made.
pub fn parse(e: &mut Engine, text: &[u8]) -> u32 {
    parse_capped(e, text, MAX_ITEMS)
}

/// [`parse`] making at most `max_items` items.
fn parse_capped(e: &mut Engine, text: &[u8], max_items: u32) -> u32 {
    let block = allocate(e, 0x14);
    if block == 0 {
        return 0;
    }
    let storage = e.call(BUILD_STORAGE_CONSTRUCT, &args![block]).u32();
    if storage == 0 || e.mem.u32(storage) == 0 {
        return storage;
    }
    let mut b = GameBuilder::new(e, storage);
    b.max_items = max_items;
    crate::parse::scan(text, &mut b);
    storage
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod tests;
