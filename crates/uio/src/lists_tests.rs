use super::*;
use std::collections::BTreeMap;

/// A game folder in memory: loose files (paths compared without case, as Windows does) and the
/// folder search's answers.
#[derive(Default)]
pub struct Fake {
    pub files: BTreeMap<Vec<u8>, Vec<u8>>,
    pub listing: Vec<Vec<u8>>,
    pub unreadable: Vec<Vec<u8>>,
    pub plugins: Vec<u8>,
}

fn key(p: &[u8]) -> Vec<u8> {
    p.to_ascii_lowercase()
}

impl Fake {
    pub fn file(&mut self, path: &str, text: &str) -> &mut Self {
        self.files
            .insert(key(path.as_bytes()), text.as_bytes().to_vec());
        self
    }
}

impl Files for Fake {
    fn exists(&mut self, path: &[u8]) -> bool {
        self.files.contains_key(&key(path))
    }
    fn read(&mut self, path: &[u8], limit: usize) -> Option<Vec<u8>> {
        if self.unreadable.contains(&key(path)) {
            return None;
        }
        let f = self.files.get(&key(path))?;
        (f.len() <= limit).then(|| f.clone())
    }
    fn list(&mut self, pattern: &[u8]) -> Vec<Vec<u8>> {
        assert_eq!(pattern, PUBLIC_PATTERN);
        self.listing.clone()
    }
    fn plugins_folder(&mut self) -> Vec<u8> {
        self.plugins.clone()
    }
}

#[test]
fn lines_are_runs_between_line_breaks_ending_at_the_mark() {
    assert_eq!(lines(b"a\r\nb\n\nc"), vec![&b"a"[..], b"b", b"c"]);
    assert_eq!(lines(b"\r\n\r\nfirst\r\n"), vec![&b"first"[..]]);
    // A 0 in the file splits a line too.
    assert_eq!(lines(b"a\0b"), vec![&b"a"[..], b"b"]);
    // A line starting with byte 3 ends the walk; a first line starting with it gives no line at
    // all (UIO's `if (!lineIter)`: the whole file is passed over).
    assert_eq!(lines(b"a\n\x03b\nc"), vec![&b"a"[..]]);
    assert!(lines(b"\x03a\nb").is_empty());
    assert!(lines(b"\r\n\0\x03a\nb").is_empty());
    // No line at all: nothing (UIO's walk lands on its end mark).
    assert!(lines(b"").is_empty());
    assert!(lines(b"\r\n\n").is_empty());
}

#[test]
fn next_token_splits_at_the_first_run() {
    assert_eq!(next_token(b"a::b:c", b':'), (&b"a"[..], &b"b:c"[..]));
    assert_eq!(next_token(b"abc", b':'), (&b"abc"[..], &b""[..]));
    assert_eq!(next_token(b"abc:", b':'), (&b"abc"[..], &b""[..]));
    assert_eq!(next_token(b"\tx", b'\t'), (&b""[..], &b"x"[..]));
}

#[test]
fn fix_spaces_keeps_file_names() {
    let fixed = |s: &str| {
        let mut b = s.as_bytes().to_vec();
        fix_spaces(&mut b);
        String::from_utf8(b).unwrap()
    };
    assert_eq!(
        fixed("?Project Nevada - Core.esm || ?Radar.esp"),
        "?Project Nevada - Core.esm\t||\t?Radar.esp"
    );
    assert_eq!(
        fixed("#my dir\\a b.xml && !darn"),
        "#my dir\\a b.xml\t&&\t!darn"
    );
    // The masked compare: `.XmL` and `.eSm` end a name too, `.txt` does not.
    assert_eq!(fixed("?a b.XmL c"), "?a b.XmL\tc");
    assert_eq!(fixed("?a b.txt c"), "?a b.txt c");
    // Outside a name: letters without bit 5 (capitals) pass, `!` and `(` do not start a name.
    assert_eq!(fixed("( TRUE )"), "(\tTRUE\t)");
    assert_eq!(fixed("! ?x y.esp"), "!\t?x y.esp");
    // A dot too near the end ends nothing.
    assert_eq!(fixed("?a .es"), "?a .es");
}

fn ctx() -> Context {
    let mut loaded_mods = Set::with_buckets(0x100);
    loaded_mods.insert(b"Radar.esp");
    loaded_mods.insert(b"JIP Companions Command & Control.esp");
    Context {
        darn_ui: true,
        vui_plus: false,
        one_hud: false,
        loaded_mods,
    }
}

fn eval(line: &str, files: &mut Fake) -> bool {
    evaluate(line.as_bytes(), &ctx(), files)
}

/// `vui+` and `ohud` each read their own UI mod.
#[test]
fn vui_and_ohud_read_their_own_mod() {
    let mut f = Fake::default();
    let vui = Context {
        darn_ui: false,
        vui_plus: true,
        one_hud: false,
        loaded_mods: Set::with_buckets(0x100),
    };
    assert!(evaluate(b"vui+", &vui, &mut f));
    assert!(!evaluate(b"ohud", &vui, &mut f));
    assert!(!evaluate(b"darn", &vui, &mut f));
    let ohud = Context {
        vui_plus: false,
        one_hud: true,
        ..vui
    };
    assert!(!evaluate(b"vui+", &ohud, &mut f));
    assert!(evaluate(b"ohud", &ohud, &mut f));
}

#[test]
fn conditions_read_left_to_right() {
    let mut f = Fake::default();
    f.file("Data\\menus\\prefabs\\here.xml", "");
    assert!(eval("true", &mut f));
    assert!(eval("TRUE", &mut f));
    assert!(eval("trueish", &mut f), "four bytes only");
    assert!(!eval("tru", &mut f));
    assert!(eval("darn", &mut f));
    assert!(!eval("!darn", &mut f));
    assert!(!eval("vui+", &mut f));
    assert!(!eval("ohud", &mut f));
    assert!(eval("?radar.esp", &mut f));
    assert!(eval("?JIP Companions Command & Control.esp", &mut f));
    assert!(!eval("?Other.esp", &mut f));
    assert!(eval("#here.xml", &mut f));
    assert!(!eval("#gone.xml", &mut f));
    assert!(!eval("nonsense", &mut f));
    assert!(eval("?Other.esp\t||\t?Radar.esp", &mut f));
    assert!(!eval("?Radar.esp\t&&\tvui+", &mut f));
    assert!(eval("?Radar.esp\t&&\t!vui+", &mut f));
    // An unknown word where `&&`/`||` is expected is passed over.
    assert!(eval("true\tand\t||\tfalse", &mut f));
}

#[test]
fn a_false_and_skips_the_rest_of_the_line() {
    let mut f = Fake::default();
    // UIO's skip count: `A && B || C` with A false stays false, C is never read.
    assert!(!eval("vui+\t&&\ttrue\t||\ttrue", &mut f));
    // A true `||` skips the rest likewise.
    assert!(eval("true\t||\tvui+\t&&\tvui+", &mut f));
}

#[test]
fn brackets_hold_a_level() {
    let mut f = Fake::default();
    assert!(eval("(\tvui+\t||\ttrue\t)", &mut f));
    assert!(eval("darn\t&&\t(\tvui+\t||\ttrue\t)", &mut f));
    assert!(!eval("darn\t&&\t(\tvui+\t||\tohud\t)", &mut f));
    // A `)` that ends a skip leaves the level open (UIO's walk): what follows writes level 1,
    // and the answer stays level 0's.
    assert!(!eval("(\tvui+\t&&\ttrue\t)\t||\ttrue", &mut f));
    // Only the first byte counts: `(x` is a bracket.
    assert!(eval("(x\ttrue\t)", &mut f));
    // An empty token (a leading tab) is a false operand.
    assert!(!eval("\ttrue", &mut f));
    // Deeper than UIO's 16 levels: no overrun here.
    let deep = format!("{}true{}", "(\t".repeat(40), "\t)".repeat(40));
    assert!(eval(&deep, &mut f));
}

#[test]
fn ui_mods_and_plugins_are_found_as_uio_finds_them() {
    let mut f = Fake {
        plugins: b"C:\\Local\\FalloutNV\\".to_vec(),
        ..Fake::default()
    };
    f.file(
        "C:\\Local\\FalloutNV\\plugins.txt",
        "FalloutNV.esm\r\noHUD.esm\r\n",
    );
    let c = context(&mut f);
    assert!(c.loaded_mods.contains(b"falloutnv.esm"));
    // No loose HUD file: none of the three.
    assert!(!c.one_hud && !c.darn_ui && !c.vui_plus);
    f.file("Data\\Menus\\Main\\hud_main_menu.xml", "");
    let c = context(&mut f);
    assert!(c.one_hud && !c.darn_ui);
    for m in ["inventory_menu", "map_menu", "stats_menu"] {
        f.file(&format!("Data\\Menus\\Main\\{m}.xml"), "");
    }
    f.file("Data\\Menus\\Options\\start_menu.xml", "");
    f.file("Data\\menus\\prefabs\\duinvsettings.xml", "");
    let c = context(&mut f);
    assert!(c.darn_ui && !c.vui_plus);
    // VUI+'s settings file, and no oHUD in plugins.txt.
    f.file("Data\\menus\\prefabs\\VUI+\\settings.xml", "");
    f.file("C:\\Local\\FalloutNV\\plugins.txt", "FalloutNV.esm\r\n");
    let c = context(&mut f);
    assert!(c.vui_plus && !c.one_hud && c.darn_ui);
    // A plugins.txt whose first line starts with byte 3: UIO reads none of it.
    f.file(
        "C:\\Local\\FalloutNV\\plugins.txt",
        "\x03oHUD.esm\r\noHUD.esm\r\n",
    );
    let c = context(&mut f);
    assert!(!c.one_hud && c.loaded_mods.is_empty());
}

fn order(lists: &InjectLists, file: &str, tile: &str) -> Vec<String> {
    lists
        .get(file.as_bytes())
        .and_then(|t| t.get(tile.as_bytes()))
        .map(|s| {
            s.iter()
                .map(|p| String::from_utf8(p.to_vec()).unwrap())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn entries_need_their_file_a_target_and_a_true_condition() {
    let mut f = Fake::default();
    for p in [
        "a.xml", "b.xml", "c.xml", "d.xml", "e.xml", "f.xml", "g.xml",
    ] {
        f.file(&format!("Data\\menus\\prefabs\\{p}"), "");
    }
    let text = "\
; a comment
a.xml:HUDMainMenu
true
nofile.xml:HUDMainMenu
true
b.xml:NoSuchMenu
true
c.xml:hudmainmenu:Glow
; a comment between does not count as the condition
vui+
d.xml:@prefabs\\x.xml
true
e.xml:@main\\x.xml
true
f.xml:@main\\x.xml:Box
true
no colon here
g.xml:InventoryMenu
true";
    let mut lists = InjectLists::default();
    read_list(&lines(text.as_bytes()), false, &ctx(), &mut f, &mut lists);
    let hud = "Data\\Menus\\Main\\hud_main_menu.xml";
    assert_eq!(order(&lists, hud, "HUDMainMenu"), vec!["a.xml"]);
    assert!(order(&lists, hud, "Glow").is_empty(), "condition false");
    assert_eq!(
        order(&lists, "Data\\menus\\prefabs\\x.xml", ""),
        vec!["d.xml"]
    );
    // `@` with no tile outside prefabs: passed over.
    assert!(lists
        .get(b"Data\\menus\\main\\x.xml")
        .and_then(|t| t.get(b""))
        .is_none());
    assert_eq!(
        order(&lists, "Data\\menus\\main\\x.xml", "Box"),
        vec!["f.xml"]
    );
    // "no colon here" takes no condition line: g.xml's entry is read as usual.
    assert_eq!(
        order(
            &lists,
            "Data\\Menus\\Main\\inventory_menu.xml",
            "InventoryMenu"
        ),
        vec!["g.xml"]
    );
    // A path line whose file is missing takes its condition line with it.
    assert_eq!(lists.len(), 4);
}

#[test]
fn the_scan_reads_supported_then_the_public_lists_in_the_search_order() {
    let mut f = Fake::default();
    f.file("Data\\menus\\prefabs\\one.xml", "")
        .file("Data\\menus\\prefabs\\two.xml", "")
        .file("Data\\menus\\prefabs\\my mod\\three.xml", "")
        .file(
            "Data\\uio\\supported.txt",
            "one.xml:HUDMainMenu\r\ntrue\r\n",
        )
        // Public lists: spaces are separators outside names.
        .file(
            "Data\\uio\\public\\b.txt",
            "my mod\\three.xml:HUDMainMenu\r\n?Radar.esp || #one.xml\r\n",
        )
        .file(
            "Data\\uio\\public\\a.txt",
            "two.xml:HUDMainMenu\r\ntrue\r\none.xml:HUDMainMenu\r\ntrue\r\n",
        );
    f.listing = vec![b"a.txt".to_vec(), b"b.txt".to_vec(), b"gone.txt".to_vec()];
    let lists = scan(&mut f);
    let mut got = order(
        &lists,
        "Data\\Menus\\Main\\hud_main_menu.xml",
        "hudmainmenu",
    );
    got.sort();
    assert_eq!(got, vec!["my mod\\three.xml", "one.xml", "two.xml"]);
    // The first spelling of the tile is kept (supported.txt's).
    let tiles = lists.get(b"Data\\Menus\\Main\\hud_main_menu.xml").unwrap();
    assert_eq!(tiles.iter().next().unwrap().0, b"HUDMainMenu");
}

#[test]
fn a_list_too_big_for_uios_buffer_or_unreadable_is_passed_over() {
    let mut f = Fake::default();
    f.file("Data\\menus\\prefabs\\one.xml", "");
    let mut big = "one.xml:HUDMainMenu\r\ntrue\r\n".to_string();
    big.push_str(&" ".repeat(LINE_BUFFER));
    f.files.insert(key(SUPPORTED), big.into_bytes());
    assert!(scan(&mut f).is_empty());
    // One byte under the limit is read.
    let mut fits = "one.xml:HUDMainMenu\r\ntrue\r\n".to_string();
    let pad = LINE_BUFFER - 2 - fits.len();
    fits.push_str(&";".repeat(pad));
    f.files.insert(key(SUPPORTED), fits.clone().into_bytes());
    assert_eq!(scan(&mut f).len(), 1);
    // One byte more (LINE_BUFFER - 1 bytes) overruns UIO's buffer: passed over.
    let mut over = fits;
    over.push(';');
    assert_eq!(over.len(), LINE_BUFFER - 1);
    f.files.insert(key(SUPPORTED), over.into_bytes());
    assert!(scan(&mut f).is_empty());
    f.unreadable.push(key(SUPPORTED));
    assert!(scan(&mut f).is_empty());
}

/// A list file whose first line starts with byte 3 is passed over whole, as UIO passes it.
#[test]
fn a_list_whose_first_line_starts_with_3_is_passed_over() {
    let mut f = Fake::default();
    f.file("Data\\menus\\prefabs\\one.xml", "").file(
        "Data\\uio\\supported.txt",
        "\x03\r\none.xml:HUDMainMenu\r\ntrue\r\n",
    );
    assert!(scan(&mut f).is_empty());
    f.file(
        "Data\\uio\\supported.txt",
        "\r\none.xml:HUDMainMenu\r\ntrue\r\n",
    );
    assert_eq!(scan(&mut f).len(), 1);
}

/// UIO copies a list's path into `prefabsPath[0x80]` (also for `#file`) and an `@` target into
/// `menusPath[0x80]`: a path that fits with its 0 is read, a longer one (up to about 1 MiB in a
/// list) names no file and is never asked for.
#[test]
fn list_paths_longer_than_uios_buffers_name_no_file() {
    /// Counts what is asked of the folder.
    struct Counting(Fake, Vec<usize>);
    impl Files for Counting {
        fn exists(&mut self, path: &[u8]) -> bool {
            self.1.push(path.len());
            self.0.exists(path)
        }
        fn read(&mut self, path: &[u8], limit: usize) -> Option<Vec<u8>> {
            self.0.read(path, limit)
        }
        fn list(&mut self, pattern: &[u8]) -> Vec<Vec<u8>> {
            self.0.list(pattern)
        }
        fn plugins_folder(&mut self) -> Vec<u8> {
            self.0.plugins_folder()
        }
    }
    let name = |n: usize, c: &str| format!("{}.xml", c.repeat(n - 4));
    let room = UIO_PATH - PREFABS.len();
    let (fits, over) = (name(room, "f"), name(room + 1, "o"));
    let huge = name(1_000_000, "h");
    let mut f = Fake::default();
    for p in [&fits, &over, &huge] {
        f.file(&format!("Data\\menus\\prefabs\\{p}"), "");
    }
    let target_room = UIO_PATH - MENUS.len();
    let (t_fits, t_over) = (name(target_room, "t"), name(target_room + 1, "u"));
    let text = format!(
        "{fits}:HUDMainMenu\ntrue\n{over}:HUDMainMenu\ntrue\n{huge}:HUDMainMenu\ntrue\n\
         {fits}:@{t_fits}:T\ntrue\n{fits}:@{t_over}:T\ntrue\n\
         {fits}:StatsMenu\n#{fits}\n{fits}:MapMenu\n#{over}\n{fits}:LevelUpMenu\n#{huge}\n"
    );
    let mut c = Counting(f, Vec::new());
    let mut lists = InjectLists::default();
    read_list(&lines(text.as_bytes()), false, &ctx(), &mut c, &mut lists);
    let hud = "Data\\Menus\\Main\\hud_main_menu.xml";
    assert_eq!(order(&lists, hud, "HUDMainMenu"), vec![fits.clone()]);
    assert_eq!(
        order(&lists, &format!("Data\\menus\\{t_fits}"), "T"),
        vec![fits.clone()]
    );
    assert!(lists
        .get(format!("Data\\menus\\{t_over}").as_bytes())
        .is_none());
    assert_eq!(
        order(&lists, "Data\\Menus\\Main\\stats_menu.xml", "StatsMenu"),
        vec![fits.clone()]
    );
    assert!(lists.get(b"Data\\Menus\\Main\\map_menu.xml").is_none());
    assert!(lists.get(b"Data\\Menus\\levelup_menu.xml").is_none());
    assert_eq!(lists.len(), 3);
    assert!(
        c.1.iter().all(|&n| n <= UIO_PATH),
        "nothing longer asked for"
    );
}

#[test]
fn odd_bytes_in_lists_never_stop_the_scan() {
    let mut f = Fake::default();
    // Non-ASCII names (the system code page's bytes), stray separators, an empty target.
    f.files
        .insert(key(b"Data\\menus\\prefabs\\\xe9t\xe9.xml"), Vec::new());
    let mut text = b"\xe9t\xe9.xml:HUDMainMenu\r\ntrue || ?\xff\xfe\r\n:\r\n::\r\n".to_vec();
    text.extend_from_slice(b"\xe9t\xe9.xml:@\r\ntrue\r\n\x00\x00junk:\x01\x02\r\n(((((\r\n");
    let mut lists = InjectLists::default();
    read_list(&lines(&text), true, &ctx(), &mut f, &mut lists);
    let paths: Vec<Vec<u8>> = lists
        .get(b"Data\\Menus\\Main\\hud_main_menu.xml")
        .and_then(|t| t.get(b"HUDMainMenu"))
        .map(|s| s.iter().map(<[u8]>::to_vec).collect())
        .unwrap_or_default();
    assert_eq!(paths, vec![b"\xe9t\xe9.xml".to_vec()]);
    // `@` alone with no tile: passed over; the junk lines name no menu.
    assert_eq!(lists.len(), 1);
}

/// oHUD's only list, `Data\UIO\Public\oHUDPatches.txt`, byte for byte (CRLF lines, no line break
/// at the end): two entries, each a path line, a condition line and a stray `true` line.
pub const OHUD_PATCHES: &str = "UIOPatches\\GrenadeKey.xml::@Prefabs\\GrenadeKeyHUD.xml::GrenadeRECT\r\n#GrenadeKeyHUD.xml\r\ntrue\r\nUIOPatches\\MineKey.xml::@Prefabs\\MineKeyHUD.xml::MineRECT\r\n#MineKeyHUD.xml\r\ntrue";

/// A game folder with oHUD installed: its list and its two patches (and the other hotkey mod's
/// file of a different name, which the list does not ask for).
fn ohud_folder() -> Fake {
    let mut f = Fake::default();
    f.file("Data\\UIO\\Public\\oHUDPatches.txt", OHUD_PATCHES)
        .file("Data\\Menus\\Prefabs\\UIOPatches\\GrenadeKey.xml", "")
        .file("Data\\Menus\\Prefabs\\UIOPatches\\MineKey.xml", "")
        .file("Data\\Menus\\Prefabs\\GrenadeHotkeyHUD.xml", "");
    f.listing = vec![b"oHUDPatches.txt".to_vec()];
    f
}

/// The entries of oHUD's list (the built-in supported.txt's come first).
fn ohud_entries(seen: &Scan) -> Vec<Entry> {
    seen.entries
        .iter()
        .filter(|e| e.list == b"oHUDPatches.txt")
        .cloned()
        .collect()
}

#[test]
fn ohuds_list_adds_nothing_without_the_two_hotkey_huds() {
    let mut f = ohud_folder();
    let (lists, seen) = scan_noted(&mut f);
    // oHUD's list adds nothing; UIO's own list (built in: no supported.txt in this folder) adds
    // the other hotkey mod's HUD, which is installed.
    assert_eq!(seen.supported, Some(SupportedFrom::BuiltIn));
    assert_eq!(
        order(
            &lists,
            "Data\\Menus\\Main\\hud_main_menu.xml",
            "hudmainmenu"
        ),
        vec!["grenadehotkeyhud.xml"]
    );
    assert_eq!(lists.len(), 1);
    assert_eq!(scan(&mut ohud_folder()).len(), 1);
    assert_eq!(
        seen.lists,
        vec![
            ListRead {
                name: b"supported.txt".to_vec(),
                opened: true
            },
            ListRead {
                name: b"oHUDPatches.txt".to_vec(),
                opened: true
            },
        ]
    );
    // Two entries, both left out on their condition (the `#` file); each `true` line is read as a
    // path line with no `:` and passed over without a note.
    let expect = |path: &str, target: &str, file: &str, tile: &str, cond: &str| Entry {
        list: b"oHUDPatches.txt".to_vec(),
        path: path.as_bytes().to_vec(),
        target: target.as_bytes().to_vec(),
        file: file.as_bytes().to_vec(),
        tile: tile.as_bytes().to_vec(),
        condition: cond.as_bytes().to_vec(),
        outcome: Outcome::ConditionFalse,
    };
    assert_eq!(
        ohud_entries(&seen),
        vec![
            expect(
                "UIOPatches\\GrenadeKey.xml",
                "@Prefabs\\GrenadeKeyHUD.xml",
                "Data\\menus\\Prefabs\\GrenadeKeyHUD.xml",
                "GrenadeRECT",
                "#GrenadeKeyHUD.xml"
            ),
            expect(
                "UIOPatches\\MineKey.xml",
                "@Prefabs\\MineKeyHUD.xml",
                "Data\\menus\\Prefabs\\MineKeyHUD.xml",
                "MineRECT",
                "#MineKeyHUD.xml"
            ),
        ]
    );
}

#[test]
fn ohuds_list_adds_each_patch_into_its_hotkey_hud_when_that_is_installed() {
    // Both installed: each patch goes into its hotkey HUD's file at its tile.
    let mut f = ohud_folder();
    f.file("Data\\Menus\\Prefabs\\GrenadeKeyHUD.xml", "")
        .file("Data\\Menus\\Prefabs\\MineKeyHUD.xml", "");
    let (lists, seen) = scan_noted(&mut f);
    assert_eq!(
        order(
            &lists,
            "Data\\menus\\Prefabs\\GrenadeKeyHUD.xml",
            "GrenadeRECT"
        ),
        vec!["UIOPatches\\GrenadeKey.xml"]
    );
    assert_eq!(
        order(&lists, "Data\\menus\\Prefabs\\MineKeyHUD.xml", "MineRECT"),
        vec!["UIOPatches\\MineKey.xml"]
    );
    // With the HUD file the built-in list adds to (see the test above).
    assert_eq!(lists.len(), 3);
    let ohud = ohud_entries(&seen);
    let outcomes: Vec<&Outcome> = ohud.iter().map(|e| &e.outcome).collect();
    assert_eq!(outcomes, vec![&Outcome::Added, &Outcome::Added]);
    // One of the two installed: only its patch; the plain scan agrees.
    let mut g = ohud_folder();
    g.file("Data\\Menus\\Prefabs\\GrenadeKeyHUD.xml", "");
    let plain = scan(&mut g);
    assert_eq!(plain.len(), 2);
    assert_eq!(
        order(
            &plain,
            "Data\\menus\\Prefabs\\GrenadeKeyHUD.xml",
            "GrenadeRECT"
        ),
        vec!["UIOPatches\\GrenadeKey.xml"]
    );
    // oHUD's patches themselves missing: left out before the condition is read.
    let mut h = Fake::default();
    h.file("Data\\UIO\\Public\\oHUDPatches.txt", OHUD_PATCHES)
        .file("Data\\Menus\\Prefabs\\GrenadeKeyHUD.xml", "");
    h.listing = vec![b"oHUDPatches.txt".to_vec()];
    let (lists, seen) = scan_noted(&mut h);
    assert!(lists.is_empty());
    let ohud = ohud_entries(&seen);
    let outcomes: Vec<&Outcome> = ohud.iter().map(|e| &e.outcome).collect();
    assert_eq!(
        outcomes,
        vec![&Outcome::PrefabMissing, &Outcome::PrefabMissing]
    );
}

/// A public list's condition line with a leading space, its `#` file installed: UIO reads the
/// leading blank as a false operand, so the entry is left out, and the report says why (never
/// that the installed file is missing).
#[test]
fn a_condition_with_a_leading_space_is_false_and_the_report_says_why() {
    let mut f = ohud_folder();
    f.file(
        "Data\\UIO\\Public\\oHUDPatches.txt",
        &OHUD_PATCHES.replace("\n#GrenadeKeyHUD.xml", "\n #GrenadeKeyHUD.xml"),
    )
    .file("Data\\Menus\\Prefabs\\GrenadeKeyHUD.xml", "")
    .file("Data\\Menus\\Prefabs\\MineKeyHUD.xml", "");
    let (lists, seen) = scan_noted(&mut f);
    assert_eq!(
        order(&lists, "Data\\menus\\Prefabs\\MineKeyHUD.xml", "MineRECT"),
        vec!["UIOPatches\\MineKey.xml"]
    );
    // (and the HUD file the built-in list adds to)
    assert_eq!(lists.len(), 2);
    let grenade = &ohud_entries(&seen)[0];
    assert_eq!(grenade.outcome, Outcome::ConditionFalse);
    assert_eq!(grenade.condition, b"\t#GrenadeKeyHUD.xml");
    assert_eq!(
        crate::report::reason(grenade).unwrap(),
        "its condition line starts with a blank, which UIO reads as false (#GrenadeKeyHUD.xml)"
    );
}

#[test]
fn every_way_an_entry_is_left_out_is_noted() {
    let mut f = Fake::default();
    for p in ["a.xml", "b.xml", "c.xml", "d.xml"] {
        f.file(&format!("Data\\menus\\prefabs\\{p}"), "");
    }
    let long = "x".repeat(UIO_PATH);
    let text = format!(
        "a.xml:@main\\hud.xml\r\ntrue\r\nb.xml:NoSuchMenu\r\ntrue\r\nc.xml:@{long}:T\r\ntrue\r\n\
         d.xml:HUDMainMenu\r\ntrue\r\nd.xml:HUDMainMenu\r\ntrue\r\nmissing.xml:HUDMainMenu\r\ntrue\r\n"
    );
    let mut lists = InjectLists::default();
    let mut notes = Vec::new();
    read_list_noted(
        &lines(text.as_bytes()),
        true,
        &ctx(),
        &mut f,
        &mut lists,
        b"l.txt",
        &mut notes,
    );
    let got: Vec<(&[u8], &Outcome)> = notes
        .iter()
        .map(|e| (e.path.as_slice(), &e.outcome))
        .collect();
    assert_eq!(
        got,
        vec![
            (&b"a.xml"[..], &Outcome::EndOfFileNotPrefab),
            (&b"b.xml"[..], &Outcome::UnknownMenu),
            (&b"c.xml"[..], &Outcome::TargetTooLong),
            (&b"d.xml"[..], &Outcome::Added),
            (&b"d.xml"[..], &Outcome::AlreadyAdded),
            (&b"missing.xml"[..], &Outcome::PrefabMissing),
        ]
    );
    assert!(notes.iter().all(|e| e.list == b"l.txt"));
    assert_eq!(notes[0].target, b"@main\\hud.xml");
    assert_eq!(notes[1].target, b"NoSuchMenu");
    assert_eq!(notes[3].file, b"Data\\Menus\\Main\\hud_main_menu.xml");
    assert_eq!(notes[3].tile, b"HUDMainMenu");
    assert_eq!(notes[3].condition, b"true");
    // The notes change nothing: the plain reader adds the same.
    let mut plain = InjectLists::default();
    read_list(&lines(text.as_bytes()), true, &ctx(), &mut f, &mut plain);
    assert_eq!(plain.len(), lists.len());
    assert_eq!(
        order(
            &plain,
            "Data\\Menus\\Main\\hud_main_menu.xml",
            "HUDMainMenu"
        ),
        vec!["d.xml"]
    );
}

/// UIO 2.30's own supported.txt as built in: the download's file byte for byte, 35 entries.
#[test]
fn the_built_in_supported_txt_is_uios_own_list() {
    assert_eq!(BUILT_IN_SUPPORTED.len(), 2_433);
    assert!(BUILT_IN_SUPPORTED.starts_with(b"advreconhud.xml:hudmainmenu\r\n#advrecon\\"));
    assert!(BUILT_IN_SUPPORTED.ends_with(b"\r\n?Project Nevada - Core.esm"));
    let (lists, seen) = scan_noted(&mut Fake::default());
    assert!(lists.is_empty());
    assert_eq!(seen.supported, Some(SupportedFrom::BuiltIn));
    assert_eq!(seen.entries.len(), 35);
    assert!(seen.entries.iter().all(|e| e.list == SUPPORTED_NAME));
}

/// The built-in list reads exactly as the same file on disk would: the same lists, the same
/// entries in the same order, only where it came from differs.
#[test]
fn the_built_in_list_parses_to_the_same_entries_as_the_file() {
    let folder = |disk: bool| {
        let mut f = Fake::default();
        if disk {
            f.files.insert(key(SUPPORTED), BUILT_IN_SUPPORTED.to_vec());
        }
        f.plugins = b"Local\\".to_vec();
        f.file(
            "Local\\plugins.txt",
            "FalloutNV.esm\r\nCyberianDawn.esp\r\n",
        );
        // Some of the mods the list knows installed (their prefabs and condition files).
        f.file("Data\\menus\\prefabs\\arwen\\fatigue_hud.xml", "")
            .file(
                "Data\\menus\\prefabs\\cyberiandawn\\cyberiandawnhud.xml",
                "",
            )
            .file("Data\\menus\\prefabs\\advreconhud.xml", "")
            .file("Data\\menus\\prefabs\\advrecon\\advreconsettings.xml", "");
        f
    };
    let (built_in, seen_built_in) = scan_noted(&mut folder(false));
    let (disk, seen_disk) = scan_noted(&mut folder(true));
    assert_eq!(seen_built_in.supported, Some(SupportedFrom::BuiltIn));
    assert_eq!(seen_disk.supported, Some(SupportedFrom::Disk));
    assert_eq!(seen_built_in.entries, seen_disk.entries);
    assert_eq!(seen_built_in.lists, seen_disk.lists);
    assert_eq!(
        order(
            &built_in,
            "Data\\Menus\\Main\\hud_main_menu.xml",
            "hudmainmenu"
        ),
        order(&disk, "Data\\Menus\\Main\\hud_main_menu.xml", "hudmainmenu"),
    );
    let mut got = order(
        &built_in,
        "Data\\Menus\\Main\\hud_main_menu.xml",
        "hudmainmenu",
    );
    got.sort();
    assert_eq!(
        got,
        vec![
            "advreconhud.xml",
            "arwen\\fatigue_hud.xml",
            "cyberiandawn\\cyberiandawnhud.xml"
        ]
    );
    let added = seen_built_in
        .entries
        .iter()
        .filter(|e| e.outcome == Outcome::Added)
        .count();
    assert_eq!(added, 3);
}

/// A supported.txt in the game folder wins over the built-in one (a player's, or a newer UIO's),
/// and the public lists come after it either way: a public entry the list already has is
/// already added, as with UIO's file.
#[test]
fn a_supported_txt_on_disk_wins_and_public_lists_come_after_either_list() {
    let mut f = Fake::default();
    f.file(
        "Data\\uio\\supported.txt",
        "mine.xml:hudmainmenu\r\ntrue\r\n",
    )
    .file("Data\\menus\\prefabs\\mine.xml", "")
    .file("Data\\menus\\prefabs\\arwen\\fatigue_hud.xml", "")
    .file(
        "Data\\uio\\public\\p.txt",
        "arwen\\fatigue_hud.xml:hudmainmenu\r\ntrue\r\n",
    );
    f.listing = vec![b"p.txt".to_vec()];
    let (lists, seen) = scan_noted(&mut f);
    assert_eq!(seen.supported, Some(SupportedFrom::Disk));
    assert_eq!(seen.entries.len(), 2);
    let mut got = order(
        &lists,
        "Data\\Menus\\Main\\hud_main_menu.xml",
        "hudmainmenu",
    );
    got.sort();
    assert_eq!(got, vec!["arwen\\fatigue_hud.xml", "mine.xml"]);
    // Without the file on disk: the built-in list first (it has arwen's fatigue HUD), then the
    // public list's same entry, already added.
    f.files.remove(&key(SUPPORTED));
    let (lists, seen) = scan_noted(&mut f);
    assert_eq!(seen.supported, Some(SupportedFrom::BuiltIn));
    assert_eq!(
        order(
            &lists,
            "Data\\Menus\\Main\\hud_main_menu.xml",
            "hudmainmenu"
        ),
        vec!["arwen\\fatigue_hud.xml"]
    );
    let last = seen.entries.last().unwrap();
    assert_eq!(last.list, b"p.txt");
    assert_eq!(last.outcome, Outcome::AlreadyAdded);
}
