//! The game's side of UIO's XML reader for the tests: its heap, its list nodes, the parse storage
//! and its templates, the trait names map (with a few of the game's names), the user traits by id
//! and the game settings, stood in by doubles that keep the game's layouts (`build.rs`).

use crate::build::*;
use crate::hash::str_hash;
use engine_core::prelude::*;

pub fn ret(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

/// The doubles' vtable functions (any free addresses).
const TEXT_HASH: u32 = 0x0600_0004;
const TEXT_EQUAL: u32 = 0x0600_0008;
const INT_HASH: u32 = 0x0600_0104;
const INT_EQUAL: u32 = 0x0600_0108;
const BUCKETS: u32 = 37;

/// The game names the tests use, with the game's ids (`Tile::LoadTextTable`).
pub const NAMES: &[(&str, u32)] = &[
    ("rect", 0x385),
    ("image", 0x386),
    ("text", 0x387),
    ("menu", 0x38c),
    ("template", 999),
    ("x", 0xfa1),
    ("y", 0xfa2),
    ("visible", 0xfa3),
    ("string", 0xfdf),
    ("filename", 0xfb9),
    ("copy", 0x7d0),
    ("add", 0x7d1),
    ("&true;", 2),
    ("&left;", 1),
];

fn lower_text(e: &Engine, at: u32) -> Vec<u8> {
    nvse::table::guest_text(e, at).to_ascii_lowercase()
}

/// A map at `map` (vtable, bucket count, buckets, count) with the given functions.
fn make_map(e: &mut Engine, map: u32, hash: u32, equal: u32) {
    let vtable = e.mem.alloc(0x20);
    e.mem.set_u32(vtable + 4, hash);
    e.mem.set_u32(vtable + 8, equal);
    e.mem.set_u32(map, vtable);
    e.mem.set_u32(map + 4, BUCKETS);
    let buckets = e.mem.alloc(BUCKETS * 4);
    e.mem.set_u32(map + 8, buckets);
    e.mem.set_u32(map + 0xc, 0);
}

/// An entry put into a text map as the game's own insert would (first in its bucket).
pub fn map_put(e: &mut Engine, map: u32, key: &str, data: u32) {
    let k = e.mem.alloc(key.len() as u32 + 1);
    e.mem.write(k, &[key.as_bytes(), b"\0"].concat());
    let bucket = str_hash(key.as_bytes()) % BUCKETS;
    let slot = e.mem.u32(map + 8) + bucket * 4;
    let entry = e.mem.alloc(0xc);
    e.mem.set_u32(entry, e.mem.u32(slot));
    e.mem.set_u32(entry + 4, k);
    e.mem.set_u32(entry + 8, data);
    e.mem.set_u32(slot, entry);
    e.mem.set_u32(map + 0xc, e.mem.u32(map + 0xc) + 1);
}

/// The data of `key` in a map (walking its buckets as the game does), for the checks.
pub fn map_get(e: &Engine, map: u32, key: u32, text: bool) -> Option<u32> {
    for b in 0..BUCKETS {
        let mut entry = e.mem.u32(e.mem.u32(map + 8) + b * 4);
        while entry != 0 {
            let k = e.mem.u32(entry + 4);
            let same = if text {
                lower_text(e, k) == lower_text(e, key)
            } else {
                k == key
            };
            if same {
                return Some(e.mem.u32(entry + 8));
            }
            entry = e.mem.u32(entry);
        }
    }
    None
}

/// The settings map's owner (`*0x11C8048`).
pub fn settings_map(e: &Engine) -> u32 {
    e.mem.u32(SETTINGS_OWNER) + SETTINGS_MAP
}

/// A game setting `name` with the value word `value` (+4 of the setting).
pub fn setting(e: &mut Engine, name: &str, value: u32) {
    let s = e.mem.alloc(0xc);
    e.mem.set_u32(s + 4, value);
    let map = settings_map(e);
    map_put(e, map, name, s);
}

/// A text value for a string setting.
pub fn text_value(e: &mut Engine, t: &str) -> u32 {
    let at = e.mem.alloc(t.len() as u32 + 1);
    e.mem.write(at, &[t.as_bytes(), b"\0"].concat());
    at
}

/// A new template (as `00A0C0C0` makes one): its name text +0 (a plain text here), owner +4, an
/// empty item list +8.
fn template(e: &mut Engine, name: u32, owner: u32) -> u32 {
    let t = e.mem.alloc(0x14);
    e.mem.set_u32(t, name);
    e.mem.set_u32(t + 4, owner);
    t
}

/// The engine with the game's side stood in.
pub fn engine() -> Engine {
    let mut e = Engine::new();
    for page in [0x011f_3000, 0x011a_6000, 0x011c_8000] {
        e.map(page, 0x1000);
    }
    // The game's heap.
    e.register(0x00aa_3e40, |e, a| ret(e.mem.alloc(a[1].max(1))));
    e.register(0x00aa_4060, |e, a| {
        if a[1] != 0 {
            e.mem.free(a[1]);
        }
        ret(0)
    });
    // A list node from the game's pool (`0043A010`): three zeroed words.
    e.register(LIST_NEW_NODE, |e, _| ret(e.mem.alloc(0xc)));
    // `RemoveTail` (`00AE8010`, fastcall on the list): the tail unlinked, its item answered.
    e.register(LIST_REMOVE_TAIL, |e, a| {
        let list = a[0];
        let node = e.mem.u32(list + 4);
        let prev = e.mem.u32(node + 4);
        e.mem.set_u32(list + 4, prev);
        if prev == 0 {
            e.mem.set_u32(list, 0);
        } else {
            e.mem.set_u32(prev, 0);
        }
        let item = e.mem.u32(node + 8);
        e.mem.free(node);
        e.mem.set_u32(list + 8, e.mem.u32(list + 8) - 1);
        ret(item)
    });
    // `BuildStorage::BuildStorage`: the main template (named `&-;`, its owner the storage), no
    // sub-templates, none being defined, the flag +0x10 set.
    e.register(BUILD_STORAGE_CONSTRUCT, |e, a| {
        let s = a[0];
        let main = template(e, 0, s);
        e.mem.set_u32(s, main);
        e.mem.set_u32(s + 4, 0);
        e.mem.set_u32(s + 8, 0);
        e.mem.set_u32(s + 0xc, 0);
        e.mem.set_u8(s + 0x10, 1);
        ret(s)
    });
    // The storage's sub-templates (+4: a node list, item +0, next +4): found by name, or made.
    e.register(FIND_TEMPLATE, |e, a| {
        let (s, name) = (a[0], lower_text(e, a[1]));
        let mut node = s + 4;
        while node != 0 && e.mem.u32(node) != 0 {
            let t = e.mem.u32(node);
            if lower_text(e, e.mem.u32(t)) == name {
                return ret(t);
            }
            node = e.mem.u32(node + 4);
        }
        ret(0)
    });
    e.register(NEW_TEMPLATE, |e, a| {
        let s = a[0];
        let text = nvse::table::guest_text(e, a[1]);
        let name = e.mem.alloc(text.len() as u32 + 1);
        e.mem.write(name, &text);
        let t = template(e, name, s);
        // Put last in the storage's list.
        let mut node = s + 4;
        while e.mem.u32(node) != 0 {
            let next = e.mem.u32(node + 4);
            if next == 0 {
                let n = e.mem.alloc(8);
                e.mem.set_u32(node + 4, n);
                node = n;
                break;
            }
            node = next;
        }
        e.mem.set_u32(node, t);
        ret(t)
    });
    // The maps' doubles: text keys hashed and compared without case, integer keys as numbers.
    e.register(TEXT_HASH, |e, a| {
        ret(str_hash(&nvse::table::guest_text(e, a[1])) % BUCKETS)
    });
    e.register(TEXT_EQUAL, |e, a| {
        ret((lower_text(e, a[1]) == lower_text(e, a[2])) as u32)
    });
    e.register(INT_HASH, |_, a| ret(a[1] % BUCKETS));
    e.register(INT_EQUAL, |_, a| ret((a[1] == a[2]) as u32));
    make_map(&mut e, TRAIT_MAP, TEXT_HASH, TEXT_EQUAL);
    make_map(&mut e, TRAITS_BY_ID, INT_HASH, INT_EQUAL);
    for (name, id) in NAMES {
        map_put(&mut e, TRAIT_MAP, name, *id);
    }
    e.mem.set_u32(USER_NEXT_ID, 10000);
    let settings = e.mem.alloc(0x200);
    e.mem.set_u32(SETTINGS_OWNER, settings);
    make_map(&mut e, settings + SETTINGS_MAP, TEXT_HASH, TEXT_EQUAL);
    e
}

/// One parsed item as the checks see it: type, number, text (`None`: no text buffer), id.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub kind: u32,
    pub num: f32,
    pub text: Option<String>,
    pub id: u32,
}

pub fn item(kind: u32, num: f32, text: Option<&str>, id: u32) -> Item {
    Item {
        kind,
        num,
        text: text.map(str::to_string),
        id,
    }
}

/// The items of a template in list order (the list's count checked against the walk).
pub fn items(e: &Engine, template: u32) -> Vec<Item> {
    let mut out = Vec::new();
    let mut node = e.mem.u32(template + 8);
    let mut prev = 0;
    while node != 0 {
        assert_eq!(e.mem.u32(node + 4), prev, "the previous link");
        let at = e.mem.u32(node + 8);
        let data = e.mem.u32(at + 8);
        let text = (data != 0).then(|| {
            let t = nvse::table::guest_text(e, data);
            assert_eq!(e.mem.u16(at + 0xc) as usize, t.len() & 0xffff, "the length");
            String::from_utf8(t).unwrap()
        });
        out.push(Item {
            kind: e.mem.u32(at),
            num: f32::from_bits(e.mem.u32(at + 4)),
            text,
            id: e.mem.u32(at + 0x10),
        });
        prev = node;
        node = e.mem.u32(node);
    }
    assert_eq!(e.mem.u32(template + 0xc), prev, "the tail");
    assert_eq!(e.mem.u32(template + 0x10) as usize, out.len(), "the count");
    out
}

/// The items `text` parses into (the main template's).
pub fn parsed(e: &mut Engine, text: &str) -> Vec<Item> {
    let storage = parse(e, text.as_bytes());
    assert_ne!(storage, 0);
    let main = e.mem.u32(storage);
    items(e, main)
}

/// The pool's count and items.
pub fn pool(e: &Engine) -> Vec<u32> {
    let n = e.mem.u32(ITEM_POOL_COUNT);
    (0..n).map(|i| e.mem.u32(ITEM_POOL + i * 4)).collect()
}
