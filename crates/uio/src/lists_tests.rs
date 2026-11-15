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
