//! UIO end to end on an engine: the game's Windows calls and its two menu functions stood in by
//! doubles over a game folder in memory.

use crate::game::{COMPRESS_AND_STORE_XML, TILE_PARSE_FILE, TILE_READ_FILE};
use crate::lists::Files;
use crate::test_game::{self, item, items, Item};
use crate::*;
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Default)]
struct World {
    /// Loose files (lowercase path).
    loose: BTreeMap<Vec<u8>, Vec<u8>>,
    /// What the game's reader finds (archives too), by lowercase path.
    game: BTreeMap<Vec<u8>, Vec<u8>>,
    /// The public folder's search answers.
    listing: Vec<Vec<u8>>,
    /// Open handles: the file's bytes and the read position.
    open: Vec<(Vec<u8>, usize)>,
    find_at: usize,
    /// Each `Tile::ParseFile` call (none should come: UIO parses itself).
    parsed: Vec<Vec<u8>>,
    /// What `Tile::ReadFile` was asked: the path, the storage the seam answered, the keep byte.
    reads: Vec<(Vec<u8>, Option<u32>, bool)>,
    freed: Vec<u32>,
    /// At each read of the game's text: a release point, then the second engine ([`OTHER`])
    /// takes UIO's slot and counts the patched files it finds.
    other_reads: bool,
    seen: Vec<usize>,
    /// UIOLoad's `RegTraitID` calls (name, id) and its static `BSRandom` constructions.
    names: Vec<(Vec<u8>, u32)>,
    randoms: u32,
    /// `Font::CreateText` calls (`00A12880`).
    creates: u32,
}

thread_local! {
    static WORLD: RefCell<World> = RefCell::new(World::default());
    /// A second engine of the run (a real thread's), sharing the test engine's slots.
    static OTHER: RefCell<Option<Engine>> = const { RefCell::new(None) };
}

fn world<T>(f: impl FnOnce(&mut World) -> T) -> T {
    WORLD.with(|w| f(&mut w.borrow_mut()))
}

fn text(e: &Engine, at: u32) -> Vec<u8> {
    nvse::table::guest_text(e, at).to_ascii_lowercase()
}

use test_game::ret;

const PLUGINS_FOLDER: u32 = 0x0120_2e98;

fn engine() -> Engine {
    world(|w| *w = World::default());
    let mut e = test_game::engine();
    e.map(PLUGINS_FOLDER & !0xfff, 0x1000);
    e.mem.write(PLUGINS_FOLDER, b"Local\\\0");
    // GetFileAttributesA
    e.register(0x00fd_f160, |e, a| {
        let p = text(e, a[0]);
        ret(if world(|w| w.loose.contains_key(&p)) {
            0x80
        } else {
            u32::MAX
        })
    });
    // CreateFileA, ReadFile, CloseHandle
    e.register(0x00fd_f088, |e, a| {
        let p = text(e, a[0]);
        ret(world(|w| match w.loose.get(&p).cloned() {
            Some(b) => {
                w.open.push((b, 0));
                w.open.len() as u32
            }
            None => u32::MAX,
        }))
    });
    e.register(0x00fd_f080, |e, a| {
        let (handle, buf, n, read) = (a[0], a[1], a[2], a[3]);
        let chunk = world(|w| {
            let (b, pos) = &mut w.open[handle as usize - 1];
            let end = (*pos + n as usize).min(b.len());
            let c = b[*pos..end].to_vec();
            *pos = end;
            c
        });
        e.mem.write(buf, &chunk);
        e.mem.set_u32(read, chunk.len() as u32);
        ret(1)
    });
    e.register(0x00fd_f07c, |_, _| ret(1));
    // FindFirstFileA, FindNextFileA, FindClose
    e.register(0x00fd_f070, |e, a| {
        let first = world(|w| {
            w.find_at = 1;
            w.listing.first().cloned()
        });
        match first {
            Some(name) => {
                e.mem.write(a[1] + 0x2c, &[name.as_slice(), b"\0"].concat());
                ret(0x77)
            }
            None => ret(u32::MAX),
        }
    });
    e.register(0x00fd_f068, |e, a| {
        let next = world(|w| {
            w.find_at += 1;
            w.listing.get(w.find_at - 1).cloned()
        });
        match next {
            Some(name) => {
                e.mem.write(a[1] + 0x2c, &[name.as_slice(), b"\0"].concat());
                ret(1)
            }
            None => ret(0),
        }
    });
    e.register(0x00fd_f064, |_, _| ret(1));
    // The game's heap: the doubles' heap, each free written down.
    e.register(0x00aa_4060, |_, a| {
        world(|w| w.freed.push(a[1]));
        ret(0)
    });
    // RegTraitID(name, id) and the static BSRandom (UIOLoad's start).
    e.register(0x009f_f8a0, |e, a| {
        let name = nvse::table::guest_text(e, a[0]);
        world(|w| w.names.push((name, a[1])));
        ret(0)
    });
    e.register(0x0047_6c00, |_, _| {
        world(|w| w.randoms += 1);
        ret(0x011c_4180)
    });
    // Menu::CompressAndStoreXML(path, text): the given text, else the game's text, or null.
    e.register(COMPRESS_AND_STORE_XML, |e, a| {
        if world(|w| w.other_reads) {
            engine_core::extend::release_held_for_test();
            let n = OTHER.with(|o| {
                o.borrow_mut()
                    .as_mut()
                    .map(|o| o.ext::<Uio>().patched.len())
            });
            engine_core::extend::release_held_for_test();
            world(|w| w.seen.extend(n));
        }
        let t = if a[1] != 0 {
            Some(nvse::table::guest_text(e, a[1]))
        } else {
            let p = text(e, a[0]);
            world(|w| w.game.get(&p).cloned())
        };
        match t {
            Some(t) => {
                let data = e.mem.alloc(t.len() as u32 + 1);
                e.mem.write(data, &[t.as_slice(), b"\0"].concat());
                let storage = e.mem.alloc(8);
                e.mem.set_u32(storage, t.len() as u32);
                e.mem.set_u32(storage + 4, data);
                ret(storage)
            }
            None => ret(0),
        }
    });
    // Tile::ParseFile: never called with UIO on.
    e.register(TILE_PARSE_FILE, |e, a| {
        world(|w| w.parsed.push(nvse::table::guest_text(e, a[0])));
        ret(0)
    });
    // Tile::ReadFile(parent, path) as the translation reaches the seam (its keep byte on the
    // stack): what the seam answered written down; a tile when it answered a storage.
    e.register(TILE_READ_FILE, |e, a| {
        let path = nvse::table::guest_text(e, a[1]);
        let (storage, keep) = e.with_stack(4, |e, k| {
            let s = e.site(0x00a0_1b87, &[a[1], k.addr()]);
            (s, e.mem.u8(k.addr()) != 0)
        });
        world(|w| w.reads.push((path, storage, keep)));
        ret(if storage.is_some() { 0x7117 } else { 0 })
    });
    e
}

fn loose(path: &str, t: &str) {
    world(|w| {
        w.loose.insert(
            path.to_ascii_lowercase().into_bytes(),
            t.as_bytes().to_vec(),
        )
    });
}

fn game(path: &str, t: &str) {
    world(|w| {
        w.game.insert(
            path.to_ascii_lowercase().into_bytes(),
            t.as_bytes().to_vec(),
        )
    });
}

type Shape = (u32, u32, Option<String>);

/// An item's type and id, and its text for a tile start (6): the tree the items make.
fn shape(items: &[Item]) -> Vec<Shape> {
    items
        .iter()
        .map(|i| {
            let name = if i.kind == 6 { i.text.clone() } else { None };
            (i.kind, i.id, name)
        })
        .collect()
}

fn start(id: u32, name: &str) -> Shape {
    (6, id, Some(name.to_string()))
}

fn end(id: u32) -> Shape {
    (7, id, None)
}

/// `Tile::ReadFile` reaching UIO's seam with `path`: the storage answered, the keep byte, and the
/// storage's main items.
fn read_file(e: &mut Engine, path: &str) -> (Option<u32>, bool, Vec<Item>) {
    let at = e.mem.alloc(path.len() as u32 + 1);
    e.mem.write(at, &[path.as_bytes(), b"\0"].concat());
    let (storage, keep) = if e.uio() {
        e.with_stack(4, |e, k| {
            let s = e.site(0x00a0_1b87, &[at, k.addr()]);
            (s, e.mem.u8(k.addr()) != 0)
        })
    } else {
        (None, false)
    };
    let got = storage.map(|s| items(e, e.mem.u32(s))).unwrap_or_default();
    (storage, keep, got)
}

const HUD: &str = "Data\\Menus\\Main\\hud_main_menu.xml";
const HUD_TEXT: &str = r#"<menu name="HUDMainMenu"><rect name="JHUD"></rect></menu>"#;
const MENU: u32 = 0x38c;
const RECT: u32 = 0x385;
const IMAGE: u32 = 0x386;
const TEXT: u32 = 0x387;

fn a_mod_installed() {
    loose("Local\\plugins.txt", "FalloutNV.esm\r\nRadar.esp\r\n");
    loose("Data\\uio\\supported.txt", "radar\\radarhud.xml:hudmainmenu\r\n?Radar.esp\r\nsc\\sc_hud.xml:hudmainmenu\r\n?Stimpak Counter.esp\r\n");
    loose("Data\\menus\\prefabs\\radar\\radarhud.xml", "");
    loose("Data\\menus\\prefabs\\sc\\sc_hud.xml", "");
    loose("Data\\menus\\prefabs\\mine\\a b.xml", "");
    loose(
        "Data\\uio\\public\\mine.txt",
        "mine\\a b.xml:HUDMainMenu:JHUD\r\ntrue\r\n",
    );
    world(|w| w.listing = vec![b"mine.txt".to_vec()]);
    game(HUD, HUD_TEXT);
    game(
        "data\\menus\\prefabs\\radar\\radarhud.xml",
        r#"<image name="radar"></image>"#,
    );
    game(
        "data\\menus\\prefabs\\mine\\a b.xml",
        r#"<text name="mine"></text>"#,
    );
}

#[test]
fn off_nothing_is_installed() {
    let mut e = engine();
    a_mod_installed();
    install(&mut e);
    assert!(!e.uio());
    assert!(e.site_owners(0x00a0_1b87).is_empty());
    assert!(nvse::plugins::listed(&e, NAME.as_bytes()).is_none());
    // Part 3's seams: none has a handler.
    let part3 = [OPERATOR_SITE, FINAL_POST_PARSE_SITE, TEXT_SCALE_SITE];
    for slot in part3.iter().chain(HUD_ZOOM_SITES.iter()) {
        assert!(e.site_owners(slot.addr()).is_empty(), "{:08x}", slot.addr());
    }
}

#[test]
fn on_every_part_3_seam_has_uios_handler_and_uioload_registers_the_operators() {
    let mut e = engine();
    set_on(&mut e, true);
    install(&mut e);
    let part3 = [OPERATOR_SITE, FINAL_POST_PARSE_SITE, TEXT_SCALE_SITE];
    for slot in part3.iter().chain(HUD_ZOOM_SITES.iter()) {
        assert_eq!(
            e.site_owners(slot.addr()),
            vec![Owner::Uio],
            "{:08x}",
            slot.addr()
        );
    }
    // The patches made in UIOLoad answer nothing before it ran (the exe's code).
    assert_eq!(e.site(0x00a0_3923, &[0xfb8]), None);
    assert_eq!(e.site(0x0076_f688, &[0x164]), None);
    assert_eq!(e.site(0x0070_a049, &[0]), None);
    // UIOLoad: the 20 names in UIO's order, each a text of its own, then the static BSRandom.
    let names = world(|w| w.names.clone());
    let expected: Vec<(Vec<u8>, u32)> = [
        ("ge", 0x7dd),
        ("ne", 0x7df),
        ("le", 0x7e1),
        ("neg", 0x7ea),
        ("recipr", 0x7eb),
        ("land", 0x7ec),
        ("lor", 0x7ed),
        ("shl", 0x7ee),
        ("shr", 0x7ef),
        ("bt", 0x7f0),
        ("sqrt", 0x7f1),
        ("pow", 0x7f2),
        ("sin", 0x7f3),
        ("cos", 0x7f4),
        ("tan", 0x7f5),
        ("asin", 0x7f6),
        ("acos", 0x7f7),
        ("atan", 0x7f8),
        ("log", 0x7f9),
        ("rand", 0x7fa),
    ]
    .iter()
    .map(|(n, id)| (n.as_bytes().to_vec(), *id))
    .collect();
    assert_eq!(names, expected);
    assert_eq!(world(|w| w.randoms), 1);
    // After it: the zoom joins FinalPostParse's test; the HUD's zooms are skipped.
    assert_eq!(e.site(0x00a0_3923, &[0xfb8]), Some(1));
    for slot in HUD_ZOOM_SITES {
        assert_eq!(e.site(slot.addr(), &[0]), Some(1));
    }
    // Each handler at its own site (the owner alone does not tell which function sits there).
    // FinalPostParse: a trait outside UIO's range (x, 0xFA1) is not taken in.
    assert_eq!(e.site(FINAL_POST_PARSE_SITE.addr(), &[0xfa1]), None);
    // The jump table: add runs on the value and the flag comes back.
    let v = e.mem.alloc(0x14);
    e.mem.set_f32(v + 8, 10.0);
    assert_eq!(
        e.site(OPERATOR_SITE.addr(), &[v, 0x7d1, 5f32.to_bits(), 0, 1]),
        Some(1)
    );
    assert_eq!(e.mem.f32(v + 8), 15.0);
    // MakeNode's text: UIO makes the one Font::CreateText call (a tile without a zoom value).
    world(|w| w.creates = 0);
    e.register(0x00a1_2880, |_, _| {
        world(|w| w.creates += 1);
        ret(0)
    });
    let tile = e.mem.alloc(0x40);
    let frame = e.mem.alloc(0x20);
    let (width_at, limit_at, colour_at, shape_at, icons_at) =
        (frame, frame + 4, frame + 8, frame + 0x18, frame + 0x1c);
    e.mem.set_i32(width_at, 400);
    e.mem.set_i32(limit_at, 30);
    let words = [
        tile, 0x5000, 0x6000, width_at, limit_at, 3, 1, colour_at, shape_at, icons_at,
    ];
    assert_eq!(e.site(TEXT_SCALE_SITE.addr(), &words), Some(1));
    assert_eq!(world(|w| w.creates), 1);
}

#[test]
fn on_the_lists_patch_the_hud_and_uio_parses_it() {
    let mut e = engine();
    a_mod_installed();
    set_on(&mut e, true);
    install(&mut e);
    install(&mut e);
    assert!(e.uio());
    assert_eq!(
        nvse::plugins::listed(&e, b"ui organizer plugin"),
        Some((NAME, 230))
    );
    // UIOLoad's place: the exe goes on.
    assert_eq!(e.site(0x0070_a049, &[0]), None);
    assert!(e.ext::<Uio>().loaded);
    // The HUD file (read from the game, patched): Radar's HUD into the menu, the public list's
    // into JHUD; the includes expanded by UIO's reader; Stimpak Counter not loaded: not added;
    // the text parsed by UIO's reader.
    let (storage, keep, got) = read_file(&mut e, HUD);
    assert!(storage.is_some());
    assert_eq!(
        shape(&got),
        vec![
            start(MENU, "HUDMainMenu"),
            start(RECT, "JHUD"),
            start(TEXT, "mine"),
            end(TEXT),
            end(RECT),
            start(IMAGE, "radar"),
            end(IMAGE),
            end(MENU),
        ]
    );
    // The HUD is one of the seven menus whose parse UIO does not keep: no keep byte, and a later
    // read takes the file again (unpatched, as UIO reads it).
    assert!(!keep);
    let (again, keep, got) = read_file(&mut e, HUD);
    assert_ne!(again, storage);
    assert!(!keep);
    assert_eq!(
        shape(&got),
        vec![
            start(MENU, "HUDMainMenu"),
            start(RECT, "JHUD"),
            end(RECT),
            end(MENU)
        ]
    );
    // The game's own parser never ran; the game's texts were freed on the game's heap.
    assert!(world(|w| w.parsed.is_empty()));
    assert!(world(|w| w.freed.len()) >= 3);
}

#[test]
fn a_kept_parse_is_answered_again_with_the_keep_byte() {
    let mut e = engine();
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    game(
        "Data\\Menus\\x.xml",
        r#"<rect name="a"><include src="in.xml"/></rect>"#,
    );
    game("data\\menus\\prefabs\\in.xml", r#"<x>1</x>"#);
    let (storage, keep, got) = read_file(&mut e, "Data\\Menus\\x.xml");
    assert!(keep);
    assert_eq!(
        got,
        vec![
            item(6, 901.0, Some("a"), RECT),
            item(8, 1.0, None, 0xfa1),
            item(7, 901.0, None, RECT),
        ]
    );
    // Read again (in any case): the same storage, kept; the file not read again.
    game("Data\\Menus\\x.xml", "<rect></rect>");
    let (again, keep, _) = read_file(&mut e, "DATA\\menus\\X.XML");
    assert_eq!((again, keep), (storage, true));
    assert_eq!(
        e.ext::<Uio>()
            .parsed
            .get(&crate::hash::str_hash(b"data\\menus\\x.xml")),
        Some(&Parsed {
            storage: storage.unwrap(),
            cached: true
        })
    );
}

#[test]
fn a_file_the_game_cannot_find_goes_to_the_exes_parse() {
    let mut e = engine();
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    assert_eq!(
        read_file(&mut e, "Data\\Menus\\nothing.xml"),
        (None, false, vec![])
    );
    // Found later (UIO's entry holds no storage): parsed then.
    game("Data\\Menus\\nothing.xml", "<x>2</x>");
    let (storage, keep, got) = read_file(&mut e, "Data\\Menus\\nothing.xml");
    assert!(storage.is_some() && keep);
    assert_eq!(got, vec![item(8, 2.0, None, 0xfa1)]);
}

#[test]
fn the_parse_cache_entries_uioload_makes() {
    let mut e = engine();
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    let parsed = &e.ext::<Uio>().parsed;
    assert_eq!(parsed.len(), 8);
    for path in [
        &b"Data\\Menus\\globals.xml"[..],
        b"data\\menus\\main\\STATS_MENU.xml",
        b"jip_temp.xml",
    ] {
        assert_eq!(
            parsed.get(&crate::hash::str_hash(path)),
            Some(&Parsed {
                storage: 0,
                cached: false
            })
        );
    }
}

#[test]
fn an_injected_component_is_read_from_memory_under_jip_temp() {
    let mut e = engine();
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    game("data\\menus\\prefabs\\in.xml", r#"<x>3</x>"#);
    let text = br#"<rect name="J"><include src="in.xml"/></rect>"#;
    assert_eq!(inject_component(&mut e, 0x4000, text), 0x7117);
    let reads = world(|w| w.reads.clone());
    assert_eq!(reads.len(), 1);
    let (path, storage, keep) = &reads[0];
    assert_eq!(path, b"jip_temp.xml");
    // Never kept (UIOLoad's entry): ReadFile deletes it.
    assert!(!keep);
    let got = items(&e, e.mem.u32(storage.unwrap()));
    assert_eq!(
        got,
        vec![
            item(6, 901.0, Some("J"), RECT),
            item(8, 3.0, None, 0xfa1),
            item(7, 901.0, None, RECT),
        ]
    );
    // Nothing is left for a later read, and no file was read for it.
    assert_eq!(e.ext::<Uio>().extern_text, None);
    // An empty text: nothing read, 0.
    world(|w| w.reads.clear());
    assert_eq!(inject_component(&mut e, 0x4000, b""), 0);
    assert_eq!(inject_component(&mut e, 0x4000, b"\0<x>1</x>"), 0);
    assert!(world(|w| w.reads.is_empty()));
    // Too long: nothing.
    let long = vec![b' '; game::MAX_INJECTED + 1];
    assert_eq!(inject_component(&mut e, 0x4000, &long), 0);
    assert!(world(|w| w.reads.is_empty()));
    // Exactly the cap: read.
    let mut capped = br#"<rect name="K"></rect>"#.to_vec();
    capped.resize(game::MAX_INJECTED, b' ');
    assert_eq!(inject_component(&mut e, 0x4000, &capped), 0x7117);
    let reads = world(|w| w.reads.clone());
    assert_eq!(reads.len(), 1);
    let got = items(&e, e.mem.u32(reads[0].1.unwrap()));
    assert_eq!(
        got,
        vec![
            item(6, 901.0, Some("K"), RECT),
            // From the pool: the name's item, "K" still in its text.
            item(7, 901.0, Some("K"), RECT)
        ]
    );
}

/// A tag name or `&entity` filling the whole injected text (the cap, 1 MiB) is read without
/// stopping the game.
#[test]
fn an_injected_text_of_one_long_name_never_stops_the_game() {
    let mut e = engine();
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    let mut tag = vec![b'<'];
    tag.resize(game::MAX_INJECTED - 1, b'a');
    tag.push(b'>');
    assert_eq!(inject_component(&mut e, 0x4000, &tag), 0x7117);
    let mut entity = b"<x>&".to_vec();
    entity.resize(game::MAX_INJECTED - 4, b'a');
    entity.extend(b"</x>");
    assert_eq!(entity.len(), game::MAX_INJECTED);
    assert_eq!(inject_component(&mut e, 0x4000, &entity), 0x7117);
    let reads = world(|w| w.reads.clone());
    assert_eq!(reads.len(), 2);
    assert_eq!(items(&e, e.mem.u32(reads[0].1.unwrap())), vec![]);
    let got = items(&e, e.mem.u32(reads[1].1.unwrap()));
    assert_eq!(got.len(), 1);
    assert_eq!(
        got[0].text.as_ref().map(String::len),
        Some(game::MAX_INJECTED - 7)
    );
}

#[test]
fn no_lists_at_all_patch_nothing() {
    let mut e = engine();
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    assert!(e.ext::<Uio>().patched.is_empty());
}

#[test]
fn a_public_list_folder_entry_that_is_a_folder_or_too_long_is_passed_over() {
    let mut e = engine();
    a_mod_installed();
    // A folder named like a list and a name with characters the code page lacks: they do not
    // open, and the rest is read.
    world(|w| {
        w.listing = vec![
            b"folder.txt".to_vec(),
            b"?bad?.txt".to_vec(),
            b"mine.txt".to_vec(),
        ]
    });
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    let (_, _, got) = read_file(&mut e, HUD);
    assert!(shape(&got).contains(&start(TEXT, "mine")));
}

/// Paths of about 1 MiB (a list's path line and `#file` token, an include's src, the path
/// `Tile::ReadFile` is given) never reach the game's stack: the lists and reads go on.
#[test]
fn paths_of_a_megabyte_never_stop_the_game() {
    let mut e = engine();
    a_mod_installed();
    let huge = format!("{}.xml", "h".repeat(1_000_000));
    loose(&format!("Data\\menus\\prefabs\\{huge}"), "");
    loose(
        "Data\\uio\\public\\long1.txt",
        &format!("{huge}:HUDMainMenu\r\ntrue\r\n"),
    );
    loose(
        "Data\\uio\\public\\long2.txt",
        &format!("mine\\a b.xml:HUDMainMenu\r\n#{huge}\t||\ttrue\r\n"),
    );
    world(|w| {
        w.listing = vec![
            b"long1.txt".to_vec(),
            b"long2.txt".to_vec(),
            b"mine.txt".to_vec(),
        ]
    });
    game(
        "data\\menus\\prefabs\\mine\\a b.xml",
        &format!(r#"<text name="mine"></text><include src="{huge}"/>"#),
    );
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    let (_, _, got) = read_file(&mut e, HUD);
    let mine = vec![start(TEXT, "mine"), end(TEXT)];
    assert!(shape(&got).windows(2).any(|w| w == mine.as_slice()));
    assert!(got.iter().all(|i| !format!("{i:?}").contains("hhhh")));
    assert_eq!(
        read_file(&mut e, &format!("Data\\Menus\\{huge}")),
        (None, false, Vec::new())
    );
}

/// The game's Windows calls get no path longer than `MAX_PATH` less its 0, and a file read
/// stops at the caller's limit: a file of exactly the limit is read, one byte more is not.
#[test]
fn the_folder_is_asked_for_paths_up_to_max_path_and_reads_up_to_the_limit() {
    let mut e = engine();
    let at = |n: usize| format!("{}.txt", "p".repeat(n - 4));
    let (fits, over) = (at(game::MAX_PATH_LEN), at(game::MAX_PATH_LEN + 1));
    let body = "b".repeat(0x1800);
    loose(&fits, &body);
    loose(&over, &body);
    let mut files = game::GameFiles(&mut e);
    assert!(files.exists(fits.as_bytes()));
    assert!(!files.exists(over.as_bytes()));
    assert_eq!(
        files.read(fits.as_bytes(), 0x1800),
        Some(body.clone().into_bytes())
    );
    assert_eq!(files.read(fits.as_bytes(), 0x17ff), None);
    assert_eq!(files.read(over.as_bytes(), 0x1800), None);
}

/// Another engine of the run (a real thread's) that takes UIO's shared slot while a read is in
/// the game's calls finds every patched file there: the read holds no part of them across a call.
#[test]
fn a_read_leaves_the_patched_files_in_the_shared_slot_across_the_games_calls() {
    let mut e = engine();
    a_mod_installed();
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    let count = e.ext::<Uio>().patched.len();
    assert!(count > 0);
    let mut other = Engine::new();
    e.share_slots_for_test(&mut other);
    OTHER.with(|o| *o.borrow_mut() = Some(other));
    world(|w| w.other_reads = true);
    let (_, _, got) = read_file(&mut e, HUD);
    assert!(shape(&got).contains(&start(IMAGE, "radar")));
    let seen = world(|w| std::mem::take(&mut w.seen));
    assert!(!seen.is_empty(), "the includes were read from the game");
    assert!(seen.iter().all(|&n| n == count), "{seen:?}");
    OTHER.with(|o| o.borrow_mut().take());
    assert_eq!(e.ext::<Uio>().patched.len(), count);
}

/// The game's text is copied out when it fits the room exactly and refused one byte short; the
/// text and its record are freed on the game's heap either way.
#[test]
fn a_game_text_of_exactly_the_room_is_read_and_both_frees_happen_either_way() {
    let mut e = engine();
    game("Data\\Menus\\n.xml", "abcdef");
    for (room, want) in [
        (6, reader::Fetch::Text(b"abcdef".to_vec())),
        (5, reader::Fetch::TooLarge),
    ] {
        world(|w| w.freed.clear());
        assert_eq!(game::game_text(&mut e, b"Data\\Menus\\n.xml", room), want);
        let freed = world(|w| w.freed.clone());
        assert_eq!(freed.len(), 2, "room {room}: the text and its record");
        assert!(freed.iter().all(|&a| a != 0) && freed[0] != freed[1]);
    }
}
