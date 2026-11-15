//! UIO end to end on an engine: the game's Windows calls and its two menu functions stood in by
//! doubles over a game folder in memory.

use crate::game::{COMPRESS_AND_STORE_XML, TILE_PARSE_FILE};
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
    /// The texts `Tile::ParseFile` was given: the path and the text.
    parsed: Vec<(Vec<u8>, Vec<u8>)>,
    freed: Vec<u32>,
}

thread_local! {
    static WORLD: RefCell<World> = RefCell::new(World::default());
}

fn world<T>(f: impl FnOnce(&mut World) -> T) -> T {
    WORLD.with(|w| f(&mut w.borrow_mut()))
}

fn text(e: &Engine, at: u32) -> Vec<u8> {
    nvse::table::guest_text(e, at).to_ascii_lowercase()
}

fn ret(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

const PLUGINS_FOLDER: u32 = 0x0120_2e98;

fn engine() -> Engine {
    world(|w| *w = World::default());
    let mut e = Engine::new();
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
    // The game's heap.
    e.register(0x00aa_3e40, |e, a| ret(e.mem.alloc(a[1].max(1))));
    e.register(0x00aa_4060, |_, a| {
        world(|w| w.freed.push(a[1]));
        ret(0)
    });
    // Menu::CompressAndStoreXML(path, NULL): the game's text, or null.
    e.register(COMPRESS_AND_STORE_XML, |e, a| {
        assert_eq!(a[1], 0);
        let p = text(e, a[0]);
        match world(|w| w.game.get(&p).cloned()) {
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
    // Tile::ParseFile(path, buffer, length): the text it was given.
    e.register(TILE_PARSE_FILE, |e, a| {
        let path = nvse::table::guest_text(e, a[0]);
        let t = e.mem.bytes(a[1], a[2]);
        assert_eq!(e.mem.u8(a[1] + a[2]), 0, "the buffer ends in a 0");
        world(|w| w.parsed.push((path, t)));
        ret(0x5150)
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

/// `Tile::ReadFile` reaching UIO's seam with `path`: the storage answered, and the text parsed.
fn read_file(e: &mut Engine, path: &str) -> (Option<u32>, Option<String>) {
    let at = e.mem.alloc(path.len() as u32 + 1);
    e.mem.write(at, &[path.as_bytes(), b"\0"].concat());
    world(|w| w.parsed.clear());
    let storage = if e.uio() {
        e.site(0x00a0_1b87, &[at])
    } else {
        None
    };
    let parsed = world(|w| {
        w.parsed
            .last()
            .map(|(_, t)| String::from_utf8(t.clone()).unwrap())
    });
    (storage, parsed)
}

const HUD: &str = "Data\\Menus\\Main\\hud_main_menu.xml";
const HUD_TEXT: &str = r#"<menu name="HUDMainMenu"><rect name="JHUD"></rect></menu>"#;

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
    game("data\\menus\\prefabs\\radar\\radarhud.xml", "<radar/>");
    game("data\\menus\\prefabs\\mine\\a b.xml", "<mine/>");
}

#[test]
fn off_nothing_is_installed() {
    let mut e = engine();
    a_mod_installed();
    install(&mut e);
    assert!(!e.uio());
    assert!(e.site_owners(0x00a0_1b87).is_empty());
    assert!(nvse::plugins::listed(&e, NAME.as_bytes()).is_none());
}

#[test]
fn on_the_lists_patch_the_hud_and_readfile_parses_uios_text() {
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
    // into JHUD; the includes expanded by UIO's reader; Stimpak Counter not loaded: not added.
    let (storage, parsed) = read_file(&mut e, HUD);
    assert_eq!(storage, Some(0x5150));
    assert_eq!(
        parsed.unwrap(),
        r#"<menu name="HUDMainMenu"><rect name="JHUD"><mine/></rect><radar/></menu>"#
    );
    // The HUD is one of the seven menus UIO reads again from the file after its first read.
    let (_, parsed) = read_file(&mut e, HUD);
    assert_eq!(parsed.unwrap(), HUD_TEXT);
    // The parse buffer and the game's texts were freed on the game's heap.
    assert!(world(|w| w.freed.len()) >= 3);
}

#[test]
fn a_file_the_game_cannot_find_goes_to_the_exes_parse() {
    let mut e = engine();
    set_on(&mut e, true);
    install(&mut e);
    e.site(0x0070_a049, &[0]);
    assert_eq!(read_file(&mut e, "Data\\Menus\\nothing.xml"), (None, None));
    // An ordinary file still goes through UIO's reader.
    game("Data\\Menus\\x.xml", r#"<a><include src="in.xml"/></a>"#);
    game("data\\menus\\prefabs\\in.xml", "IN");
    assert_eq!(
        read_file(&mut e, "Data\\Menus\\x.xml"),
        (Some(0x5150), Some("<a>IN</a>".to_string()))
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
    let (_, parsed) = read_file(&mut e, HUD);
    assert!(parsed.unwrap().contains("<mine/>"));
}
