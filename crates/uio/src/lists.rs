//! UI Organizer's lists (`UIOLoad`, `ui_organizer.h` (UIO), its first half): `Data\uio\
//! supported.txt` and every `Data\uio\public\*.txt`, the conditions on each entry, and the list
//! of what goes where (`injectLists`: a target file's tiles, each with its set of paths).
//!
//! Every file is read through [`Files`] as UIO reads it with the C library and Windows: loose
//! files in the game's folder, never the archives.

use crate::hash::{begins_ci, Map, Set};
use crate::reader::UIO_PATH;

/// What UIO asks of the file system (the game's folder; paths relative to it, as UIO gives them).
pub trait Files {
    /// `FileExists` (UIO): `GetFileAttributes` answers, and not a folder.
    fn exists(&mut self, path: &[u8]) -> bool;
    /// The whole file, as `fopen(path, "rb")` and `fread` read it; `None` when it does not open or
    /// is longer than `limit` bytes (the caller's buffer).
    fn read(&mut self, path: &[u8], limit: usize) -> Option<Vec<u8>>;
    /// `FindFirstFile(pattern)` then `FindNextFile` until it fails: the names in that order.
    fn list(&mut self, pattern: &[u8]) -> Vec<Vec<u8>>;
    /// The folder of the game's `plugins.txt` (the game's string at `0x1202E98`, ending in a
    /// separator).
    fn plugins_folder(&mut self) -> Vec<u8>;
}

/// `s_inclTempBuffer`'s size (`Init(0x100000)`), which `LineIterator` reads a whole file into,
/// with two bytes after it.
pub const LINE_BUFFER: usize = 0x10_0000;

/// UIO's paths (`ui_organizer.h` (UIO)).
pub const SUPPORTED: &[u8] = b"Data\\uio\\supported.txt";
pub const PUBLIC_PATTERN: &[u8] = b"Data\\uio\\public\\*.txt";
pub const PUBLIC_FOLDER: &[u8] = b"Data\\uio\\public\\";
pub const PREFABS: &[u8] = b"Data\\menus\\prefabs\\";
pub const MENUS: &[u8] = b"Data\\menus\\";

/// `kMenuNameToFile` (UIO): the 33 menus an entry may name, and their files (the exe's strings at
/// UIO's `PATH_*` addresses, read in Ghidra).
pub const MENU_FILES: [(&str, &str); 33] = [
    ("BarterMenu", "Data\\Menus\\barter_menu.xml"),
    ("BlackJackMenu", "Data\\Menus\\black_jack_menu.xml"),
    ("CaravanMenu", "Data\\Menus\\caravan_menu.xml"),
    ("CharGenMenu", "Data\\Menus\\CharGen\\char_gen_menu.xml"),
    (
        "CompanionWheelMenu",
        "Data\\Menus\\companion_wheel_menu.xml",
    ),
    ("ComputersMenu", "Data\\Menus\\computers_menu.xml"),
    ("ContainerMenu", "Data\\Menus\\container_menu.xml"),
    ("DialogMenu", "Data\\Menus\\Dialog\\dialog_menu.xml"),
    ("HackingMenu", "Data\\Menus\\hacking_menu.xml"),
    ("LevelUpMenu", "Data\\Menus\\levelup_menu.xml"),
    ("LoadingMenu", "Data\\Menus\\loading_menu.xml"),
    ("LockPickMenu", "Data\\Menus\\lockpick_menu.xml"),
    (
        "LoveTesterMenu",
        "Data\\Menus\\CharGen\\love_tester_menu.xml",
    ),
    ("MessageMenu", "Data\\Menus\\message_menu.xml"),
    ("QuantityMenu", "Data\\Menus\\quantity_menu.xml"),
    ("RaceSexMenu", "Data\\Menus\\CharGen\\race_sex_menu.xml"),
    ("RecipeMenu", "Data\\Menus\\recipe_menu.xml"),
    (
        "RepairServicesMenu",
        "Data\\Menus\\repair_services_menu.xml",
    ),
    ("RouletteMenu", "Data\\Menus\\roulette_menu.xml"),
    ("SleepWaitMenu", "Data\\Menus\\sleep_wait_menu.xml"),
    ("SlotMachineMenu", "Data\\Menus\\slot_machine_menu.xml"),
    (
        "SpecialBookMenu",
        "Data\\Menus\\CharGen\\SPECIALBookMenu.xml",
    ),
    ("StartMenu", "Data\\Menus\\Options\\start_menu.xml"),
    ("TraitMenu", "Data\\Menus\\trait_menu.xml"),
    ("TraitSelectMenu", "Data\\Menus\\trait_select_menu.xml"),
    ("TutorialMenu", "Data\\Menus\\tutorial_menu.xml"),
    ("VATSMenu", "Data\\Menus\\VATS_menu.xml"),
    ("HUDMainMenu", "Data\\Menus\\Main\\hud_main_menu.xml"),
    ("InventoryMenu", "Data\\Menus\\Main\\inventory_menu.xml"),
    ("ItemModMenu", "Data\\Menus\\item_mod_menu.xml"),
    ("MapMenu", "Data\\Menus\\Main\\map_menu.xml"),
    ("RepairMenu", "Data\\Menus\\repair_menu.xml"),
    ("StatsMenu", "Data\\Menus\\Main\\stats_menu.xml"),
];

/// The menu files UIO reads its UI-mod checks from (`PATH_HUDMainMenu` and the rest).
const HUD: &[u8] = b"Data\\Menus\\Main\\hud_main_menu.xml";
const INVENTORY: &[u8] = b"Data\\Menus\\Main\\inventory_menu.xml";
const MAP: &[u8] = b"Data\\Menus\\Main\\map_menu.xml";
const STATS: &[u8] = b"Data\\Menus\\Main\\stats_menu.xml";
const START: &[u8] = b"Data\\Menus\\Options\\start_menu.xml";

/// What goes where: a target file (its path as first named) -> a tile name ("" for the end of
/// the file) -> the paths below `Data\menus\prefabs\` (each as first named), in UIO's tables.
pub type InjectLists = Map<Map<Set>>;

// Translated from UIO 2.30 (426d963), utility.h/.cpp: LineIterator.
/// The lines of a file as UIO's `LineIterator` gives them: the bytes with each CR and LF made a
/// 0 and the two bytes 0, 3 put after; the walk starts at the first non-zero byte (`Init`) and a
/// line is a run of non-zero bytes; the walk ends when the next line starts with byte 3 (the end
/// mark, or a line of the file that starts with it). So a file whose first line starts with 3,
/// or with no line at all (empty, or only line breaks: the walk lands on the end mark), gives
/// none.
pub fn lines(file: &[u8]) -> Vec<&[u8]> {
    let mut starts = Vec::new();
    let is_break = |c: u8| c == b'\r' || c == b'\n' || c == 0;
    let n = file.len();
    let mut i = 0;
    while i < n && is_break(file[i]) {
        i += 1;
    }
    if i == n || file[i] == 3 {
        return Vec::new();
    }
    loop {
        let start = i;
        while i < n && !is_break(file[i]) {
            i += 1;
        }
        starts.push(&file[start..i]);
        while i < n && is_break(file[i]) {
            i += 1;
        }
        // The next line's first byte, or the end mark (3).
        if i == n || file[i] == 3 {
            break;
        }
    }
    starts
}

// Translated from UIO 2.30 (426d963), utility.cpp: GetNextToken.
/// UIO's `GetNextToken(str, delim)`: the text before the first `delim` (UIO ends it there with a
/// 0) and the text after the run of `delim`s that follows (empty when there is no `delim`).
pub fn next_token(s: &[u8], delim: u8) -> (&[u8], &[u8]) {
    match s.iter().position(|&c| c == delim) {
        None => (s, &s[s.len()..]),
        Some(at) => {
            let mut rest = at;
            while rest < s.len() && s[rest] == delim {
                rest += 1;
            }
            (&s[..at], &s[rest..])
        }
    }
}

// Translated from UIO 2.30 (426d963), utility.cpp: FixSpaces.
/// A public list's condition line with its spaces made tabs, except inside a file name: after a
/// `?` or `#` until an extension that UIO's masked compare takes for `.xml`, `.esp` or `.esm`
/// (the dot and the next three bytes ANDed with `2E 5D 5F 5D` equal to `.XML`, `.ESP` or `.ESM`).
/// Outside a name a byte without bit 5 (0x20) passes; a space becomes a tab; a byte above `?`
/// passes; `?` starts a name; any other byte below `?` starts one only when it is `#`.
pub fn fix_spaces(line: &mut [u8]) {
    let mut in_name = false;
    let mut i = 0;
    while i < line.len() {
        let c = line[i];
        i += 1;
        if in_name {
            if c == b'.' && i + 3 <= line.len() {
                let w = [b'.', line[i] & 0x5d, line[i + 1] & 0x5f, line[i + 2] & 0x5d];
                if matches!(&w, b".XML" | b".ESP" | b".ESM") {
                    in_name = false;
                    i += 3;
                }
            }
            continue;
        }
        if c & 0x20 == 0 {
            continue;
        }
        if c == b' ' {
            line[i - 1] = b'\t';
        } else if c <= b'?' {
            in_name = c == b'?' || c == b'#';
        }
    }
}

/// What the conditions read besides files: the three UI mods UIO looks for and the plugins.txt
/// lines.
pub struct Context {
    pub darn_ui: bool,
    pub vui_plus: bool,
    pub one_hud: bool,
    /// Every line of `plugins.txt` (UIO's `loadedMods`, by hash).
    pub loaded_mods: Set,
}

/// The first four bytes of `token` with each ORed with 0x20 (UIO's `*(UInt32*)p | 0x20202020`);
/// a shorter token never matches, since its ending 0 becomes a space.
fn word4(token: &[u8]) -> Option<[u8; 4]> {
    (token.len() >= 4).then(|| {
        [
            token[0] | 0x20,
            token[1] | 0x20,
            token[2] | 0x20,
            token[3] | 0x20,
        ]
    })
}

// Translated from UIO 2.30 (426d963), ui_organizer.h: UIOLoad (the condition loop).
/// A condition line's answer (`cndEvalStack[0]`): tab-separated tokens, left to right. `(` and
/// `)` (their first byte) open and close a level; after an operand `&&` sets the skip count to
/// "the level is false" and `||` to "the level is true" (another token there is passed over); while
/// the count is not 0 every token but the brackets is passed over, `(` raising and `)` lowering
/// it. An operand: `!` first negates; `true`, `darn`, `vui+`, `ohud` (four bytes); `#file` when
/// `Data\menus\prefabs\file` exists; `?name` when plugins.txt lists it; anything else false. UIO's
/// stack holds 16 levels and has no check; here it grows (deeper lines run past UIO's array).
pub fn evaluate(line: &[u8], ctx: &Context, files: &mut dyn Files) -> bool {
    let mut stack = vec![false];
    let mut level = 0usize;
    let mut skip = 0u32;
    let mut expect_and_or = false;
    let mut rest = line;
    while !rest.is_empty() {
        let (token, after) = next_token(rest, b'\t');
        rest = after;
        match token.first() {
            Some(b'(') => {
                expect_and_or = false;
                if skip != 0 {
                    skip += 1;
                } else {
                    level += 1;
                    if stack.len() <= level {
                        stack.resize(level + 1, false);
                    }
                    stack[level] = false;
                }
                continue;
            }
            Some(b')') => {
                expect_and_or = true;
                if skip != 0 {
                    skip -= 1;
                } else if level != 0 {
                    stack[level - 1] = stack[level];
                    level -= 1;
                }
                continue;
            }
            _ => {}
        }
        if skip != 0 {
            continue;
        }
        if expect_and_or {
            if token.starts_with(b"&&") {
                skip = u32::from(!stack[level]);
            } else if token.starts_with(b"||") {
                skip = u32::from(stack[level]);
            } else {
                continue;
            }
        } else {
            let negate = token.first() == Some(&b'!');
            let t = if negate { &token[1..] } else { token };
            let result = match word4(t) {
                Some(w) if &w == b"true" => true,
                Some(w) if &w == b"darn" => ctx.darn_ui,
                Some(w) if &w == b"vui+" => ctx.vui_plus,
                Some(w) if &w == b"ohud" => ctx.one_hud,
                _ => match t.first() {
                    Some(b'#') => prefab_exists(files, &t[1..]),
                    Some(b'?') => ctx.loaded_mods.contains(&t[1..]),
                    _ => false,
                },
            };
            stack[level] = result ^ negate;
        }
        expect_and_or = !expect_and_or;
    }
    stack[0]
}

/// `FileExists` of `Data\menus\prefabs\<path>` (UIO copies the path into `prefabsPath[0x80]`): a
/// path too long for that buffer ([`UIO_PATH`]) names no file, before any call.
fn prefab_exists(files: &mut dyn Files, path: &[u8]) -> bool {
    PREFABS.len() + path.len() <= UIO_PATH && files.exists(&[PREFABS, path].concat())
}

/// The lines of a file UIO reads with `LineIterator`, or `None` when it does not open (or does
/// not fit UIO's buffer: a file of `LINE_BUFFER - 1` bytes or more overruns it in UIO, a
/// deliberate difference: here it is a file that did not open).
pub fn read_lines(files: &mut dyn Files, path: &[u8]) -> Option<Vec<u8>> {
    files.read(path, LINE_BUFFER - 2)
}

// Translated from UIO 2.30 (426d963), ui_organizer.h: UIOLoad (the checks and the list loop).
/// The UI mods UIO looks for and the plugins.txt lines.
pub fn context(files: &mut dyn Files) -> Context {
    let mut loaded_mods = Set::with_buckets(0x100);
    let mut path = files.plugins_folder();
    path.extend_from_slice(b"plugins.txt");
    if let Some(text) = read_lines(files, &path) {
        for line in lines(&text) {
            loaded_mods.insert(line);
        }
    }
    let (mut darn_ui, mut vui_plus, mut one_hud) = (false, false, false);
    if files.exists(HUD) {
        one_hud = loaded_mods.contains(b"oHUD.esm");
        if files.exists(INVENTORY)
            && files.exists(MAP)
            && files.exists(STATS)
            && files.exists(START)
        {
            darn_ui = files.exists(b"Data\\menus\\prefabs\\duinvsettings.xml");
            vui_plus = files.exists(b"Data\\menus\\prefabs\\VUI+\\settings.xml");
        }
    }
    Context {
        darn_ui,
        vui_plus,
        one_hud,
        loaded_mods,
    }
}

/// UIO's table of menu names (its 33 pairs inserted in order into a table of 33 buckets, aligned
/// to 64), looked up by hash.
fn menu_file(name: &[u8]) -> Option<&'static str> {
    let hash = crate::hash::str_hash(name);
    // A later pair of the same hash would replace the earlier (`InsertList`); none collide.
    MENU_FILES
        .iter()
        .rev()
        .find(|(n, _)| crate::hash::str_hash(n.as_bytes()) == hash)
        .map(|(_, f)| *f)
}

/// One list file's entries added to `lists` (`fix` for the public lists: their spaces become
/// tabs in the condition lines).
pub fn read_list(
    lines: &[&[u8]],
    fix: bool,
    ctx: &Context,
    files: &mut dyn Files,
    lists: &mut InjectLists,
) {
    // The entry a path line set up, for its condition line: the target file, the tile, the path.
    let mut pending: Option<(Vec<u8>, Vec<u8>, Vec<u8>)> = None;
    let mut cond_line = false;
    for &line in lines {
        if line.first() == Some(&b';') {
            continue;
        }
        if !cond_line {
            let (path, target) = next_token(line, b':');
            if target.is_empty() {
                continue;
            }
            cond_line = true;
            pending = None;
            if !prefab_exists(files, path) {
                continue;
            }
            let (menu, tile) = next_token(target, b':');
            let (file, tile) = if menu.first() == Some(&b'@') {
                if tile.is_empty() && !begins_ci(&menu[1..], b"prefabs\\") {
                    continue;
                }
                let file = [MENUS, &menu[1..]].concat();
                // UIO copies it into `menusPath[0x80]`.
                if file.len() > UIO_PATH {
                    continue;
                }
                (file, tile)
            } else {
                let tile = if tile.is_empty() { menu } else { tile };
                match menu_file(menu) {
                    Some(f) => (f.as_bytes().to_vec(), tile),
                    None => continue,
                }
            };
            pending = Some((file, tile.to_vec(), path.to_vec()));
        } else {
            cond_line = false;
            let Some((file, tile, path)) = pending.take() else {
                continue;
            };
            let mut cond = line.to_vec();
            if fix {
                fix_spaces(&mut cond);
            }
            if evaluate(&cond, ctx, files) {
                lists.entry(&file).entry(&tile).insert(&path);
            }
        }
    }
}

/// UIO's whole scan: supported.txt, then each public list in the order the folder search gives
/// them; a file that does not open is passed over.
pub fn scan(files: &mut dyn Files) -> InjectLists {
    let ctx = context(files);
    let mut lists = InjectLists::default();
    if let Some(text) = read_lines(files, SUPPORTED) {
        read_list(&lines(&text), false, &ctx, files, &mut lists);
    }
    for name in files.list(PUBLIC_PATTERN) {
        let path = [PUBLIC_FOLDER, &name].concat();
        if let Some(text) = read_lines(files, &path) {
            read_list(&lines(&text), true, &ctx, files, &mut lists);
        }
    }
    lists
}

#[cfg(test)]
#[path = "lists_tests.rs"]
mod tests;
