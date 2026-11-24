//! UIO's reader building the game's parse storage, byte for byte: each expected item traced by
//! hand through `ResolveTrait` (the pool of ten spare items included: an item taken from it keeps
//! its old text when its new value has none, as UIO leaves it).

use super::*;
use crate::test_game::*;

/// `float(0x80000000)`: the number of an item with no id.
const NO_NUM: f32 = -2147483648.0;

#[test]
fn a_named_tile_with_a_trait() {
    let mut e = engine();
    assert_eq!(
        parsed(&mut e, r#"<rect name="a"><x>5</x></rect>"#),
        vec![
            // The tile starts with its name (6); `name="a"`'s item went to the pool.
            item(6, 901.0, Some("a"), 0x385),
            // `<x>5</x>` is one simple trait (8): the trait start (taken from the pool, its old
            // text "a") takes the value's number and its null text (the buffer freed).
            item(8, 5.0, None, 0xfa1),
            // The tile ends (7): the close item taken back from the pool.
            item(7, 901.0, None, 0x385),
        ]
    );
    // Left in the pool: the value's item.
    assert_eq!(pool(&e).len(), 1);
    assert_eq!(e.mem.u32(pool(&e)[0]), 0xbb9);
}

#[test]
fn an_action_link_and_items_from_the_pool_keep_their_old_texts() {
    let mut e = engine();
    assert_eq!(
        parsed(
            &mut e,
            r#"<rect name="a"><y><copy src="me()" trait="x"/></y></rect>"#
        ),
        vec![
            item(6, 901.0, Some("a"), 0x385),
            // The trait start, from the pool: "a" still in its text.
            item(4, 4002.0, Some("a"), 0xfa2),
            // `<copy src trait/>`: one link (10): the trait's id as its number, the src as text.
            item(10, 4001.0, Some("me()"), 0x7d0),
            item(5, 4002.0, None, 0xfa2),
            // From the pool: the src item, "me()" still in its text.
            item(7, 901.0, Some("me()"), 0x385),
        ]
    );
    assert_eq!(pool(&e).len(), 1);
}

#[test]
fn an_action_in_a_trait() {
    let mut e = engine();
    assert_eq!(
        parsed(&mut e, "<x><copy>3</copy></x>"),
        vec![
            item(4, 4001.0, None, 0xfa1),
            item(9, 3.0, None, 0x7d0),
            item(5, 4001.0, None, 0xfa1),
        ]
    );
}

#[test]
fn game_setting_values() {
    let mut e = engine();
    let hi = text_value(&mut e, "Hi");
    setting(&mut e, "sHello", hi);
    setting(&mut e, "fHalf", 0.5f32.to_bits());
    setting(&mut e, "iSeven", 7);
    setting(&mut e, "sEmpty", 0);
    setting(&mut e, "bFlag", 1);
    assert_eq!(
        parsed(
            &mut e,
            r#"<rect name="r"><string>&-sHello;</string></rect>"#
        ),
        vec![
            item(6, 901.0, Some("r"), 0x385),
            // A text value: the number stays "none" and the text is set (the pool item's "r"
            // buffer, too small, replaced).
            item(8, NO_NUM, Some("Hi"), 0xfdf),
            item(7, 901.0, None, 0x385),
        ]
    );
    let mut e2 = engine();
    setting(&mut e2, "fHalf", 0.5f32.to_bits());
    setting(&mut e2, "iSeven", 7);
    assert_eq!(
        parsed(&mut e2, "<x>&-fHalf;</x><y>&-iSeven;</y>"),
        vec![item(8, 0.5, None, 0xfa1), item(8, 7.0, None, 0xfa2)]
    );
    // A `b` setting and an unknown one give no value: the trait start and end stay.
    let mut e3 = engine();
    setting(&mut e3, "bFlag", 1);
    assert_eq!(
        parsed(&mut e3, "<x>&-bFlag;</x>"),
        vec![item(4, 4001.0, None, 0xfa1), item(5, 4001.0, None, 0xfa1)]
    );
    // A text setting with no text: the item's text stays null.
    let mut e4 = engine();
    setting(&mut e4, "sEmpty", 0);
    assert_eq!(
        parsed(&mut e4, "<x>&-sEmpty;</x>"),
        vec![item(8, NO_NUM, None, 0xfa1)]
    );
}

#[test]
fn trait_constants_and_text() {
    let mut e = engine();
    assert_eq!(
        parsed(&mut e, "<x>&true;</x><y>&nope;</y>"),
        vec![
            item(8, 2.0, None, 0xfa1),
            item(8, NO_NUM, Some("&nope;"), 0xfa2)
        ]
    );
}

#[test]
fn user_traits_are_registered_with_the_next_id_and_their_index() {
    let mut e = engine();
    assert_eq!(
        parsed(&mut e, "<_val_3>2</_val_3>"),
        vec![item(8, 2.0, None, 10000)]
    );
    let key = text_value(&mut e, "_val_3");
    assert_eq!(map_get(&e, TRAIT_MAP, key, true), Some(10000));
    assert_eq!(map_get(&e, TRAITS_BY_ID, 10000, false), Some(3));
    assert_eq!(e.mem.u32(USER_NEXT_ID), 10001);
    // The other name forms: `_b_` (-1), `__x` (the `_` after the first: none), `_c_x` (0),
    // `_d` (none); a name met again keeps its id.
    let storage = parse(&mut e, b"");
    let mut b = GameBuilder::new(&mut e, storage);
    assert_eq!(b.add_trait_name(b"_b_"), Some(10001));
    assert_eq!(b.add_trait_name(b"__x"), Some(10002));
    assert_eq!(b.add_trait_name(b"_c_x"), Some(10003));
    assert_eq!(b.add_trait_name(b"_d"), Some(10004));
    assert_eq!(b.add_trait_name(b"_B_"), Some(10001));
    assert_eq!(b.add_trait_name(b"rect"), Some(0x385));
    assert_eq!(b.add_trait_name(b"nothing"), None);
    assert_eq!(map_get(&e, TRAITS_BY_ID, 10001, false), Some(u32::MAX));
    assert_eq!(map_get(&e, TRAITS_BY_ID, 10002, false), None);
    assert_eq!(map_get(&e, TRAITS_BY_ID, 10003, false), Some(0));
    assert_eq!(map_get(&e, TRAITS_BY_ID, 10004, false), None);
    assert_eq!(e.mem.u32(USER_NEXT_ID), 10005);
    // The new names' keys are copies on the game's heap.
    assert_eq!(e.mem.u32(TRAIT_MAP + 0xc) as usize, NAMES.len() + 5);
}

#[test]
fn a_template_definition_goes_to_its_own_template() {
    let mut e = engine();
    let storage = parse(
        &mut e,
        br#"<template name="T"><rect name="r"></rect></template>"#,
    );
    // The main template has nothing; the definition ended.
    assert_eq!(items(&e, e.mem.u32(storage)), vec![]);
    assert_eq!(e.mem.u32(storage + 0xc), 0);
    let t = e.mem.u32(storage + 4);
    assert_ne!(t, 0);
    assert_eq!(nvse::table::guest_text(&e, e.mem.u32(t)), b"T");
    assert_eq!(
        items(&e, t),
        vec![
            // From the pool: `<template>`'s name item ("T", overwritten by "r").
            item(6, 901.0, Some("r"), 0x385),
            item(7, 901.0, Some("r"), 0x385),
        ]
    );
    // Two definitions in one file: two templates, in order (the pool now shared with the first
    // parse: the trait start takes U's name item, whose text the null value then frees).
    let storage = parse(
        &mut e,
        br#"<template name="T"></template><template name="U"><x>1</x></template>"#,
    );
    let t = e.mem.u32(storage + 4);
    let u = e.mem.u32(e.mem.u32(storage + 8));
    assert_eq!(nvse::table::guest_text(&e, e.mem.u32(t)), b"T");
    assert_eq!(nvse::table::guest_text(&e, e.mem.u32(u)), b"U");
    assert_eq!(items(&e, t), vec![]);
    assert_eq!(items(&e, u), vec![item(8, 1.0, None, 0xfa1)]);
    assert_eq!(items(&e, e.mem.u32(storage)), vec![]);
}

#[test]
fn a_template_inside_a_definition_is_dropped() {
    let mut e = engine();
    let storage = parse(
        &mut e,
        br#"<template name="T"><template name="U"></template><x>1</x>"#,
    );
    // The nested name: its tag and item dropped, the definition still T's; the first
    // `</template>` ends it, so `<x>` goes to the main template.
    let t = e.mem.u32(storage + 4);
    assert_eq!(nvse::table::guest_text(&e, e.mem.u32(t)), b"T");
    assert_eq!(e.mem.u32(t + 0x10), 0);
    assert_eq!(e.mem.u32(storage + 0xc), 0);
    assert_eq!(
        items(&e, e.mem.u32(storage)),
        // The trait start is the nested name's item from the pool; its "U" freed by the null
        // value's text.
        vec![item(8, 1.0, None, 0xfa1)]
    );
}

#[test]
fn the_pool_holds_ten_items_and_frees_the_rest() {
    let mut e = engine();
    // A parse takes items before it gives any back, so it never fills the pool: `<x>1</x>`
    // leaves two.
    parsed(&mut e, &"<x>1</x>".repeat(12));
    assert_eq!(pool(&e).len(), 2);
    // Ten waiting (the game's own reader gives items back too): one more is freed, its text too.
    let storage = parse(&mut e, b"");
    let mut b = GameBuilder::new(&mut e, storage);
    let mut spare = Vec::new();
    for _ in 0..8 {
        let i = b.e.mem.alloc(0x18);
        spare.push(i);
        b.release(i);
    }
    assert_eq!(pool(b.e).len(), 10);
    let extra = b.e.mem.alloc(0x18);
    let text = b.e.mem.alloc(4);
    b.e.mem.set_u32(extra + 8, text);
    b.release(extra);
    assert_eq!(pool(b.e).len(), 10);
    assert_eq!(b.e.mem.block_size(extra), None);
    assert_eq!(b.e.mem.block_size(text), None);
    // With ten waiting, `<x>1</x>` takes three and gives two back.
    let got = parsed(&mut e, "<x>1</x>");
    assert_eq!(got, vec![item(8, 1.0, None, 0xfa1)]);
    assert_eq!(pool(&e).len(), 9);
}

#[test]
fn string_set_as_uio_sets_it() {
    let mut e = engine();
    let s = e.mem.alloc(8);
    string_set(&mut e, s, Some(b"abc"));
    let data = e.mem.u32(s);
    assert_eq!(e.mem.bytes(data, 4), b"abc\0");
    assert_eq!((e.mem.u16(s + 4), e.mem.u16(s + 6)), (3, 3));
    // A shorter text reuses the buffer.
    string_set(&mut e, s, Some(b"z"));
    assert_eq!(e.mem.u32(s), data);
    assert_eq!(e.mem.bytes(data, 2), b"z\0");
    assert_eq!((e.mem.u16(s + 4), e.mem.u16(s + 6)), (1, 3));
    // Empty: the buffer kept, emptied.
    string_set(&mut e, s, Some(b""));
    assert_eq!((e.mem.u32(s), e.mem.u8(data)), (data, 0));
    // No text: the buffer freed.
    string_set(&mut e, s, None);
    assert_eq!(
        (e.mem.u32(s), e.mem.u16(s + 4), e.mem.u16(s + 6)),
        (0, 0, 0)
    );
    // Over 65,535 bytes: the length's low 16 bits, the whole text and its end copied.
    let long = vec![b'q'; 0x1_0002];
    string_set(&mut e, s, Some(&long));
    assert_eq!(e.mem.u16(s + 4), 2);
    let data = e.mem.u32(s);
    assert_eq!(e.mem.u8(data + 0x1_0001), b'q');
    assert_eq!(e.mem.u8(data + 0x1_0002), 0);
}

#[test]
fn no_trait_map_or_settings_gives_nothing_and_no_storage_memory_gives_none() {
    let mut e = engine();
    e.mem.set_u32(TRAIT_MAP, 0);
    e.mem.set_u32(SETTINGS_OWNER, 0);
    assert_eq!(
        parsed(&mut e, "<x>&-fA;</x><_u>1</_u>"),
        vec![item(0xbb9, 1.0, None, 0x8000_0000)]
    );
    e.register(0x00aa_3e40, |_, _| ret(0));
    assert_eq!(parse(&mut e, b"<x>1</x>"), 0);
}

#[test]
fn malformed_text_never_stops_the_game() {
    // Random texts of up to 4 KiB: half of the bytes from XML's own characters and names, half
    // any byte 0x00-0xFF: the scan ends, whatever comes.
    let alphabet = b"<>/= \"&-_.;0123456789rectnamsrixy\t";
    let mut seed: u32 = 0x1234_5678;
    let mut next = || {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
        seed >> 8
    };
    let mut e = engine();
    let mut seen = [false; 256];
    for round in 0..600 {
        let len = if round % 4 == 0 {
            next() % 4097
        } else {
            next() % 97
        };
        let mut t = Vec::new();
        for _ in 0..len {
            let r = next();
            let c = if r & 1 == 0 {
                alphabet[(r >> 1) as usize % alphabet.len()]
            } else {
                (r >> 1) as u8
            };
            seen[c as usize] = true;
            t.push(c);
        }
        let storage = parse(&mut e, &t);
        assert_ne!(storage, 0);
        items_any(&e, e.mem.u32(storage));
    }
    assert!(seen.iter().all(|&s| s), "every byte value drawn");
    // Fixed cases: a 2 MiB token, a 70,000-byte value (beyond the text's 16-bit length) and a
    // text reaching the item cap.
    let mut e = engine();
    for t in [
        "a".repeat(HUGE),
        format!("<{}", "a".repeat(HUGE)),
        format!("<x name=\"{}\">", "a".repeat(HUGE)),
        format!("<x>{}</x>", "v".repeat(70_000)),
    ] {
        let storage = parse(&mut e, t.as_bytes());
        assert_ne!(storage, 0);
        items_any(&e, e.mem.u32(storage));
    }
    let mut e = engine();
    let got = parsed(&mut e, &format!("<x>{}</x>", "v".repeat(70_000)));
    assert_eq!(got[0].text.as_ref().map(String::len), Some(70_000));
    let mut e = engine();
    let storage = parse(&mut e, "<x>".repeat(MAX_ITEMS as usize + 5).as_bytes());
    assert_eq!(e.mem.u32(e.mem.u32(storage) + 0x10), MAX_ITEMS);
}

/// The items' count walked (any bytes in their texts).
fn items_any(e: &Engine, template: u32) -> usize {
    let mut n = 0;
    let mut node = e.mem.u32(template + 8);
    while node != 0 {
        n += 1;
        node = e.mem.u32(node);
    }
    assert_eq!(e.mem.u32(template + 0x10) as usize, n, "the count");
    n
}

/// 2 MiB: twice the game's whole stack for locals.
const HUGE: usize = 2 << 20;

/// A tag name, an `&entity`, an `&-setting` name and a `<template name>` of 2 MiB each (a file's
/// text may be 64 MiB) reach the game's maps and template calls without stopping the game, and a
/// long name is still found as UIO finds it (its text in the game's memory, not cut).
#[test]
fn names_of_two_megabytes_never_stop_the_game() {
    let a = "a".repeat(HUGE);
    let mut e = engine();
    // An unknown tag name: no item.
    assert_eq!(parsed(&mut e, &format!("<{a}>")), vec![]);
    // An unknown `&name`: its text kept whole.
    let got = parsed(&mut e, &format!("<x>&{a}</x>"));
    assert_eq!(got.len(), 1);
    assert_eq!((got[0].kind, got[0].id), (8, 0xfa1));
    assert_eq!(got[0].text.as_ref().map(String::len), Some(HUGE + 1));
    // An unknown setting: no value (each case on a fresh engine: no pool items carried over).
    let mut e = engine();
    assert_eq!(
        parsed(&mut e, &format!("<x>&-s{a};</x>")),
        vec![item(4, 4001.0, None, 0xfa1), item(5, 4001.0, None, 0xfa1)]
    );
    // A template named so: made, with the whole name.
    let mut e = engine();
    let storage = parse(
        &mut e,
        format!(r#"<template name="{a}"><x>1</x></template>"#).as_bytes(),
    );
    let t = e.mem.u32(storage + 4);
    assert_eq!(nvse::table::guest_text(&e, e.mem.u32(t)).len(), HUGE);
    assert_eq!(items(&e, t), vec![item(8, 1.0, None, 0xfa1)]);
    // Long names that are there: a user trait registered and met again, a setting found.
    let mut e = engine();
    let user = format!("_{a}");
    let text = format!("<{user}>1</{user}><{user}>2</{user}>");
    assert_eq!(
        parsed(&mut e, &text),
        vec![item(8, 1.0, None, 10000), item(8, 2.0, None, 10000)]
    );
    let mut e = engine();
    let name = format!("f{a}");
    setting(&mut e, &name, 0.25f32.to_bits());
    assert_eq!(
        parsed(&mut e, &format!("<x>&-{name};</x>")),
        vec![item(8, 0.25, None, 0xfa1)]
    );
    let mut e = engine();
    let key = format!("&{a}");
    map_put(&mut e, TRAIT_MAP, &key, 2);
    assert_eq!(
        parsed(&mut e, &format!("<x>{key}</x>")),
        vec![item(8, 2.0, None, 0xfa1)]
    );
}

/// The heap blocks a long name is put in are freed after each call; no heap block: not found.
#[test]
fn a_long_names_heap_block_is_freed_and_none_means_not_found() {
    let mut e = engine();
    let name = vec![b'_'; STACK_TEXT + 1];
    let before = e.mem.u32(TRAIT_MAP + 0xc);
    let storage = parse(&mut e, b"");
    let mut b = GameBuilder::new(&mut e, storage);
    assert_eq!(b.add_trait_name(&name), Some(10000));
    assert_eq!(b.e.mem.u32(TRAIT_MAP + 0xc), before + 1);
    // A name of exactly the stack's bound goes on the stack (no heap call); one byte more on the heap.
    let calls = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let c = calls.clone();
    b.e.register_double(0x00aa_3e40, move |e, a| {
        c.set(c.get() + 1);
        ret(e.mem.alloc(a[1].max(1)))
    });
    assert_eq!(b.lookup_trait(&vec![b'q'; STACK_TEXT]), None);
    assert_eq!(calls.get(), 0);
    assert_eq!(b.lookup_trait(&name), Some(10000));
    assert_eq!(calls.get(), 1);
    b.e.register(0x00aa_3e40, |_, _| ret(0));
    assert_eq!(b.lookup_trait(&name), None);
    assert_eq!(b.add_trait_name(&[name.as_slice(), b"x"].concat()), None);
}

/// A parse stops at its item cap: the items before it are kept.
#[test]
fn a_parse_stops_at_its_item_cap() {
    let mut e = engine();
    let storage = parse_capped(&mut e, b"<x>1</x><y>2</y><x>3</x>", 3);
    // `<x>`, `1`, `</x>` (folded into one), then `<y>` is the fourth: refused, the scan ends.
    assert_eq!(
        items(&e, e.mem.u32(storage)),
        vec![item(8, 1.0, None, 0xfa1)]
    );
    let storage = parse_capped(&mut e, b"<x>1</x><y>2</y><x>3</x>", 4);
    assert_eq!(
        items(&e, e.mem.u32(storage)),
        vec![item(8, 1.0, None, 0xfa1), item(4, 4002.0, None, 0xfa2)]
    );
}

/// A map bucket whose entry points to itself: lookup and insert end.
#[test]
fn a_looped_map_bucket_ends_lookup_and_insert() {
    let mut e = engine();
    map_put(&mut e, TRAIT_MAP, "loopy", 5);
    let bucket = crate::hash::str_hash(b"loopy") % 37;
    let entry = e.mem.u32(e.mem.u32(TRAIT_MAP + 8) + bucket * 4);
    e.mem.set_u32(entry, entry);
    // Another name in the same bucket.
    let other = (0..)
        .map(|i| format!("_k{i}"))
        .find(|n| crate::hash::str_hash(n.as_bytes()) % 37 == bucket)
        .unwrap();
    // The map's compare counted: a walk past its bound fails the test instead of hanging it.
    let equal = e.mem.u32(e.mem.u32(TRAIT_MAP) + 8);
    let calls = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let c = calls.clone();
    e.register_double(equal, move |e, a| {
        c.set(c.get() + 1);
        assert!(c.get() <= MAX_WALK, "the walk passed its bound");
        let same = nvse::table::guest_text(e, a[1])
            .eq_ignore_ascii_case(&nvse::table::guest_text(e, a[2]));
        ret(same as u32)
    });
    assert_eq!(map_lookup(&mut e, TRAIT_MAP, other.as_bytes()), 0);
    assert_eq!(calls.get(), MAX_WALK);
    calls.set(0);
    // Still found when it is the looped entry itself.
    assert_ne!(map_lookup(&mut e, TRAIT_MAP, b"loopy"), 0);
    let storage = parse(&mut e, b"");
    let mut b = GameBuilder::new(&mut e, storage);
    // The insert walks the loop to its bound, then puts the new name first in the bucket.
    calls.set(0);
    assert_eq!(b.add_trait_name(other.as_bytes()), Some(10000));
    assert_eq!(calls.get(), MAX_WALK);
}

/// A text over 65,535 bytes on a string whose size field is full: a new buffer of the text's
/// length and its end (the old one freed); the 16-bit fields keep the low bits.
#[test]
fn string_set_of_seventy_thousand_bytes() {
    let mut e = engine();
    let s = e.mem.alloc(8);
    let old = e.mem.alloc(0x1_0000);
    e.mem.set_u32(s, old);
    e.mem.set_u16(s + 4, 0xffff);
    e.mem.set_u16(s + 6, 0xffff);
    let asked = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let k = asked.clone();
    e.register_double(0x00aa_3e40, move |e, a| {
        k.set(a[1]);
        ret(e.mem.alloc(a[1].max(1)))
    });
    let freed = std::rc::Rc::new(std::cell::Cell::new(0u32));
    let g = freed.clone();
    e.register_double(0x00aa_4060, move |e, a| {
        g.set(a[1]);
        e.mem.free(a[1]);
        ret(0)
    });
    let long = vec![b'v'; 70_000];
    string_set(&mut e, s, Some(&long));
    // The old buffer freed first (the new one may take its place), then the new one asked for.
    assert_eq!((freed.get(), asked.get()), (old, 70_001));
    let data = e.mem.u32(s);
    assert_eq!(e.mem.bytes(data, 70_001)[69_999..], *b"v\0");
    assert_eq!(
        (e.mem.u16(s + 4), e.mem.u16(s + 6)),
        ((70_000 & 0xffff) as u16, (70_000 & 0xffff) as u16)
    );
}
