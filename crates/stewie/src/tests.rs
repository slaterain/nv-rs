//! The reader's forms, the override rule in each case, the rows' keys and the install.

use super::*;
use crate::ini::{entries, long_value, strtol, values, Entry};
use crate::settings::{table, Settings, MISSING};
use engine_core::extend::stewie as bits;

// Each share's own tests, in files of their own (`rows.rs` has the rule).
mod ai;
mod forms;
mod menus;
mod world;

fn entry(section: Option<&str>, key: &str, value: &str) -> Entry {
    Entry {
        section: section.map(|s| s.as_bytes().to_vec()),
        key: key.as_bytes().to_vec(),
        value: value.as_bytes().to_vec(),
    }
}

/// SimpleIni's forms, one each (`FindEntry`, `AddEntry`).
#[test]
fn the_reader_follows_simple_ini() {
    let file = b"\xef\xbb\xbfbefore=1\r\n\
        ; a note\n\
        \t# another note\n\
        [ Tweaks ]  text after the heading\n\
        bA = 1 ; not a note\n\
        bB=\"1\"\n\
        bC\t=\t0x1F\t\r\n\
        no equals sign\n\
        =no key\n\
        \n\
        [broken\n\
        bLost=1\n\
        [TWEAKS]\n\
        BA=2\r\
        bEmpty=\n";
    assert_eq!(
        entries(file),
        [
            entry(Some(""), "before", "1"),
            entry(Some("Tweaks"), "bA", "1 ; not a note"),
            entry(Some("Tweaks"), "bB", "\"1\""),
            entry(Some("Tweaks"), "bC", "0x1F"),
            entry(None, "bLost", "1"),
            entry(Some("TWEAKS"), "BA", "2"),
            entry(Some("TWEAKS"), "bEmpty", ""),
        ]
    );
    let v = values(file);
    let get = |s: &str, k: &str| {
        v.get(&(s.as_bytes().to_vec(), k.as_bytes().to_vec()))
            .cloned()
    };
    // No case; the last of a key wins; the sections of one name are one.
    assert_eq!(get("tweaks", "ba"), Some(b"2".to_vec()));
    assert_eq!(get("tweaks", "bc"), Some(b"0x1F".to_vec()));
    // A key under a broken heading is found by no name.
    assert!(v.keys().all(|(_, k)| k != b"blost"));
    // The text ends at its first NUL: a UTF-16 file gives nothing.
    assert!(entries(b"\xff\xfe[\x00T\x00]\x00\na\x00=\x001\x00").is_empty());
    assert_eq!(entries(b"a=1\0b=2"), [entry(Some(""), "a", "1")]);
}

/// `GetLongValue`: decimal and `0x` hexadecimal through `strtol`, the default for anything left
/// over, an empty value, a bare `0x` or a value past its 64-byte buffer.
#[test]
fn numbers_read_as_get_long_value() {
    let read = |v: &str| long_value(Some(v.as_bytes()), -7);
    assert_eq!(read("1"), 1);
    assert_eq!(read("-12"), -12);
    assert_eq!(read("+5"), 5);
    assert_eq!(read("0x1f"), 31);
    assert_eq!(read("0X10"), 16);
    assert_eq!(read("0x0x10"), 16, "strtol's base 16 takes a 0x of its own");
    assert_eq!(read("0x"), -7);
    assert_eq!(read(""), -7);
    assert_eq!(read("1 ; note"), -7);
    assert_eq!(read("\"1\""), -7);
    assert_eq!(read("1.5"), -7);
    assert_eq!(read("abc"), -7);
    assert_eq!(read("99999999999"), i32::MAX);
    assert_eq!(read("-99999999999"), i32::MIN);
    assert_eq!(read(&"0".repeat(63)), 0);
    assert_eq!(read(&"0".repeat(64)), -7);
    assert_eq!(long_value(None, -7), -7);
    assert_eq!(strtol(b"  12x", 10), (12, 4));
    assert_eq!(strtol(b"x", 10), (0, 0));
}

#[test]
fn the_table_has_every_key_once_and_vnvs_values() {
    let t = table();
    assert_eq!(t.len(), 1499);
    let mut seen = std::collections::HashSet::new();
    for s in &t {
        assert!(
            seen.insert((s.section.to_ascii_lowercase(), s.key.to_ascii_lowercase())),
            "[{}] {} twice",
            s.section,
            s.key
        );
    }
    let find = |section: &str, key: &str| {
        *t.iter()
            .find(|s| s.section == section && s.key == key)
            .unwrap()
    };
    // A value VNV changes (PLAN.md 1b), one VNV turns on, one VNV lacks, one written as an expression.
    let music = find("Music Volume", "fDialogueMusicVolumeMult");
    assert_eq!((music.code, music.vnv), (Some("0.2"), Some("0.500000")));
    let autowalk = find("Tweaks", "bBetterAutoWalk");
    assert_eq!((autowalk.code, autowalk.vnv), (Some("0"), Some("1")));
    let dr = find("INI", "bFixCreatureDoubledDR");
    assert_eq!(
        (dr.code, dr.vnv, dr.port_default()),
        (Some("1"), None, Some("1"))
    );
    assert_eq!(find("Hold Breath", "iHoldBreathKey").code, None);
    // VNV sets 103 [Tweaks] keys to 1, 102 of them off in Tweaks' code (the other, bUseAnimVariants,
    // has a default written as an expression). PLAN.md 1b says 104, counted from its own script.
    let on = t
        .iter()
        .filter(|s| s.section == "Tweaks" && s.vnv == Some("1") && s.code == Some("0"))
        .count();
    assert_eq!(on, 102);
}

/// The override rule in each case: no file; a partial file; the "as Tweaks" switch; a value Tweaks
/// would refuse.
#[test]
fn a_players_file_overlays_vnvs_values_key_by_key() {
    // No file: VNV's values.
    let none = Settings::new(&[], false);
    assert_eq!(none.long("Tweaks", "bBetterAutoWalk", -1), 1);
    assert_eq!(
        none.raw("Music Volume", "fDialogueMusicVolumeMult"),
        Some(b"0.500000".to_vec())
    );
    assert!(none.read.is_empty());
    // A partial file: its keys win (any case), the keys it lacks keep VNV's values.
    let file = b"[tweaks]\nbbetterautowalk=0\nbConsoleNumpadSupport = 1\n[Music Volume]\nfDialogueMusicVolumeMult=0.5\n";
    let partial = Settings::new(&[("p.ini", file)], false);
    assert_eq!(partial.long("Tweaks", "bBetterAutoWalk", -1), 0);
    assert_eq!(partial.long("Tweaks", "bConsoleNumpadSupport", -1), 1);
    assert_eq!(
        partial.long("Tweaks", "bDescriptiveMarkerAddedMessage", -1),
        1,
        "VNV's value kept"
    );
    // Two of its three keys change the text under them: 0 for VNV's 1, and 0.5 for VNV's 0.500000
    // (counted by the text, not the number); bConsoleNumpadSupport=1 is VNV's value already.
    assert_eq!(partial.read[0].keys, 3);
    assert_eq!(partial.read[0].changed, 2);
    assert_eq!(
        partial.log_lines(),
        ["lStewieAl's Tweaks: read p.ini: 2 of its 3 keys changed a value"]
    );
    // The switch: a key the file lacks takes Tweaks' code default.
    let as_tweaks = Settings::new(&[("p.ini", file)], true);
    assert_eq!(as_tweaks.long("Tweaks", "bConsoleNumpadSupport", -1), 1);
    assert_eq!(
        as_tweaks.long("Tweaks", "bDescriptiveMarkerAddedMessage", -1),
        0
    );
    assert_eq!(
        Settings::new(&[], true).long("Tweaks", "bBetterAutoWalk", -1),
        0
    );
    // A value Tweaks refuses (not a number, or its own "missing" mark) leaves the default under it.
    let refused = format!("[Tweaks]\nbBetterAutoWalk=yes\nbConsoleNumpadSupport={MISSING}\n");
    let refused = Settings::new(&[("r.ini", refused.as_bytes())], false);
    assert_eq!(refused.long("Tweaks", "bBetterAutoWalk", -1), 1);
    assert_eq!(refused.long("Tweaks", "bConsoleNumpadSupport", -1), 1);
    // A later file wins over an earlier one.
    let two = Settings::new(
        &[
            ("a", b"[Tweaks]\nbBetterAutoWalk=0"),
            ("b", b"[Tweaks]\nbBetterAutoWalk=1"),
        ],
        false,
    );
    assert_eq!(two.long("Tweaks", "bBetterAutoWalk", -1), 1);
    // A key in no table and no file: the caller's default.
    assert_eq!(none.long("Tweaks", "bNoSuchKey", -3), -3);
}

/// The S0079 row (`bDialogSpeechNullCheck`), which the key tests use.
const PROOF: &rows::Row = &rows::ROWS[28];

#[test]
fn every_row_has_its_own_bit_and_key() {
    let mut bits_seen = std::collections::HashSet::new();
    let mut keys_seen = std::collections::HashSet::new();
    let settings = table();
    assert_eq!(PROOF.plan, "S0079");
    assert!(
        rows::LISTS
            .iter()
            .all(|list| list.windows(2).all(|w| w[0].plan < w[1].plan)),
        "each list in PLAN.md's order"
    );
    for r in rows::all() {
        // The key is `b` and the function's name without its `patch` prefix; a block of a
        // function of several blocks (its PLAN number has a second part) adds a word after it.
        let name = r.function.strip_prefix("patch").unwrap_or(r.function);
        let named = &r.key[1..];
        let ok = if r.plan.contains('.') {
            named.len() > name.len() && named[..name.len()].eq_ignore_ascii_case(name)
        } else {
            named.eq_ignore_ascii_case(name)
        };
        assert!(ok && r.key.starts_with('b'), "{}", r.key);
        assert!(bits_seen.insert(r.bit), "{} shares a bit", r.function);
        assert!(
            keys_seen.insert(r.key.to_ascii_lowercase()),
            "{} twice",
            r.key
        );
        assert!(r.key.starts_with('b') && r.key != rows::AS_TWEAKS_KEY);
        assert!(r.plan.starts_with('S'));
        if let Some((section, key)) = r.tweaks {
            assert!(
                settings
                    .iter()
                    .any(|s| s.section == section && s.key == key),
                "{key}"
            );
        }
    }
}

/// One share: its rows and its two ranges (fixes, tweaks).
type Share = (&'static [rows::Row], [(u16, u16); 2]);

/// Each share's rows keep to its own bits (`engine_core::extend::stewie`'s `world.rs`, `forms.rs`,
/// `ai.rs`, `menus.rs`): a row put in another share's list, or numbered in another share's range, fails
/// here; and each share's pinned list names its own rows.
#[test]
fn each_shares_rows_keep_to_its_own_bits() {
    let shares: [Share; 4] = [
        (
            rows::world::ROWS,
            [
                (bits::WORLD_FIXES, bits::WORLD_FIXES_END),
                (bits::WORLD_TWEAKS, bits::WORLD_TWEAKS_END),
            ],
        ),
        (
            rows::forms::ROWS,
            [
                (bits::FORMS_FIXES, bits::FORMS_FIXES_END),
                (bits::FORMS_TWEAKS, bits::FORMS_TWEAKS_END),
            ],
        ),
        (
            rows::ai::ROWS,
            [
                (bits::AI_FIXES, bits::AI_FIXES_END),
                (bits::AI_TWEAKS, bits::AI_TWEAKS_END),
            ],
        ),
        (
            rows::menus::ROWS,
            [
                (bits::MENUS_FIXES, bits::MENUS_FIXES_END),
                (bits::MENUS_TWEAKS, bits::MENUS_TWEAKS_END),
            ],
        ),
    ];
    let inside = |bit: bits::Row, ranges: &[(u16, u16); 2]| {
        ranges
            .iter()
            .any(|&(first, end)| (first as usize..end as usize).contains(&bit.index()))
    };
    for (list, ranges) in &shares {
        for r in *list {
            assert!(inside(r.bit, ranges), "{} outside its share's bits", r.plan);
        }
    }
    // The four shares' ranges do not overlap: a bit is in one share's at most.
    for index in 0..bits::WORDS * 64 {
        let bit = bits::Row::at(0, index as u16);
        let owners = shares.iter().filter(|(_, r)| inside(bit, r)).count();
        assert!(owners <= 1, "bit {index} in two shares");
    }
    assert_eq!(rows::world::ROWS.len(), world::PINNED.len());
    assert_eq!(rows::forms::ROWS.len(), forms::PINNED.len());
    assert_eq!(rows::ai::ROWS.len(), ai::PINNED.len());
    assert_eq!(rows::menus::ROWS.len(), menus::PINNED.len());
    // A share's row past its range does not compile; one at its last bit does.
    assert_eq!(
        bits::Row::share(bits::AI_FIXES, bits::AI_FIXES_END, 169).index(),
        469
    );
}

/// `[nv-rs Stewie]`: a row's key turns it off (`0`) or leaves it on; a value other than 0 or 1 is
/// named and the default stands.
#[test]
fn a_rows_key_turns_it_off() {
    let ini = |text: &'static str| {
        move |key: &str| -> Option<String> {
            text.lines()
                .filter_map(|l| l.split_once('='))
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v.to_string())
        }
    };
    let settings = Settings::new(&[], false);
    let keys = rows::Keys::read(ini(""));
    assert_eq!(keys, rows::Keys::default());
    assert!(rows::on(PROOF, &keys, &settings));
    let off = rows::Keys::read(ini("bDialogSpeechNullCheck=0\nbMissingKeysAsTweaks=1"));
    assert!(!rows::on(PROOF, &off, &settings));
    assert!(off.as_tweaks);
    let bad = rows::Keys::read(ini("bDialogSpeechNullCheck=yes\nbMissingKeysAsTweaks="));
    assert!(rows::on(PROOF, &bad, &settings));
    assert!(!bad.as_tweaks);
    assert_eq!(bad.bad, ["bDialogSpeechNullCheck=yes"]);
}

/// A row with Tweaks' own key obeys it too: on only while the setting reads as a number other
/// than 0.
#[test]
fn a_row_with_a_tweaks_key_obeys_it() {
    let row = rows::Row {
        tweaks: Some(("Tweaks", "bBetterAutoWalk")),
        key: "bTestRow",
        ..*PROOF
    };
    let keys = rows::Keys::default();
    assert!(
        rows::on(&row, &keys, &Settings::new(&[], false)),
        "VNV has it on"
    );
    assert!(
        !rows::on(&row, &keys, &Settings::new(&[], true)),
        "Tweaks' code has it off"
    );
    let off = Settings::new(&[("p", b"[Tweaks]\nbBetterAutoWalk=0")], false);
    assert!(!rows::on(&row, &keys, &off));
    let off_by_key = rows::Keys {
        rows: vec![("bTestRow", false)],
        ..Default::default()
    };
    assert!(!rows::on(&row, &off_by_key, &Settings::new(&[], false)));
}

/// Engine Optimizations' row S1016.845558 follows Tweaks' own switch for its block,
/// `[Inlines] bSaveLoad` (`settings.h`: `g_bInlineSaveLoad`, on in Tweaks' code and in VNV): on by
/// default, off when the player's INI sets it to 0, and off by its own key too.
#[test]
fn the_changes_map_row_obeys_tweaks_inlines_save_load() {
    let row = rows::all().find(|r| r.plan == "S1016.845558").unwrap();
    assert_eq!(row.tweaks, Some(("Inlines", "bSaveLoad")));
    let keys = rows::Keys::default();
    assert!(
        rows::on(row, &keys, &Settings::new(&[], false)),
        "VNV's value"
    );
    assert!(
        rows::on(row, &keys, &Settings::new(&[], true)),
        "Tweaks' code default"
    );
    let off = Settings::new(&[("p", b"[Inlines]\nbSaveLoad=0")], false);
    assert!(!rows::on(row, &keys, &off));
    let off_by_key = rows::Keys {
        rows: vec![("bInlinesChangesMapBuckets", false)],
        ..Default::default()
    };
    assert!(!rows::on(row, &off_by_key, &Settings::new(&[], false)));
}

/// KERNEL32's import slots the game's file read goes through (`nvse::saveload::read_file`), which
/// the install reads Tweaks' INI with when no platform answers the mods' files seam.
const CREATE_FILE_A: u32 = 0x00fd_f088;
const READ_FILE: u32 = 0x00fd_f080;
const CLOSE_HANDLE: u32 = 0x00fd_f07c;

thread_local! {
    /// The INI the stand-in of `CreateFileA` serves (none: no file), and how far it was read.
    static FILE: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
    static AT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// An engine whose file calls serve `file` as Tweaks' INI.
fn engine(file: Option<&[u8]>) -> Engine {
    FILE.with(|f| *f.borrow_mut() = file.map(<[u8]>::to_vec));
    let mut e = Engine::new();
    e.register(CREATE_FILE_A, |e, w| {
        assert_eq!(e.mem.cstr(w[0]), INI, "Tweaks' path, beside its DLL");
        AT.with(|a| a.set(0));
        ret(if FILE.with(|f| f.borrow().is_some()) {
            7
        } else {
            0xffff_ffff
        })
    });
    e.register(READ_FILE, |e, w| {
        let (buffer, size, read) = (w[1], w[2] as usize, w[3]);
        let chunk = FILE.with(|f| {
            let f = f.borrow();
            let file = f.as_deref().unwrap_or(&[]);
            let at = AT.with(std::cell::Cell::get).min(file.len());
            let n = size.min(file.len() - at);
            AT.with(|a| a.set(at + n));
            file[at..at + n].to_vec()
        });
        e.mem.write(buffer, &chunk);
        e.mem.set_u32(read, chunk.len() as u32);
        ret(1)
    });
    e.register(CLOSE_HANDLE, |_, _| ret(1));
    e
}

/// Tweaks' co-save callbacks as xNVSE gets them: the `AMID` reader at the load AFTER the game's
/// (`load`, not `pre_load`), the save writer, and the listener whose PreLoadGame clears the saved
/// count (S0089.01) before the next save is read, so it never reaches another load.
#[test]
fn the_install_registers_tweaks_co_save_callbacks_and_listener() {
    let mut e = engine(None);
    set_on(&mut e, true);
    install(&mut e);
    let tweaks = nvse::plugins::by_opcode_base(&e, OPCODE_BASE).unwrap();
    assert!(tweaks.load.is_some(), "AMID is read after the game's load");
    assert!(tweaks.pre_load.is_none(), "not before it");
    assert!(tweaks.save.is_some());
    assert!(tweaks.listener.is_some());
    let kept = e.stewie_kept(bits::kept::SAVED_AMMO_COUNT, 4);
    e.mem.set_u32(kept, 8);
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::LOAD_GAME, 0, 0);
    assert_eq!(e.mem.u32(kept), 8, "only PreLoadGame clears it");
    nvse::plugins::dispatch(&mut e, nvse::plugins::msg::PRE_LOAD_GAME, 0, 0);
    assert_eq!(e.mem.u32(kept), 0);
}

/// The install: off, nothing; on, the rows' bits reach the engine (the bit the seam tests), Tweaks is
/// listed with xNVSE, and a row's key keeps its bit clear.
#[test]
fn the_install_sets_the_bits_and_lists_tweaks() {
    let mut e = engine(None);
    install(&mut e);
    assert!(!on(&e));
    assert!(!e.stewie(bits::DIALOG_SPEECH_NULL_CHECK));
    assert!(nvse::plugins::built_ins(&e).is_empty());

    let mut e = engine(None);
    set_on(&mut e, true);
    install(&mut e);
    assert!(on(&e));
    assert!(e.stewie(bits::DIALOG_SPEECH_NULL_CHECK));
    assert_eq!(e.stewie_rows_on(), rows::all().count() as u32);
    let listed = nvse::plugins::built_ins(&e);
    assert_eq!(listed.len(), 1);
    assert_eq!(
        (listed[0].name, listed[0].version, listed[0].listed),
        (NAME, 1005, false)
    );
    // Not reported as installed while its script commands are not built (IsPluginInstalled 0,
    // GetPluginVersion -1: what xNVSE answers for a plugin it did not load).
    assert!(nvse::commands::extender::plugin_info(&e, NAME.as_bytes()).is_none());
    assert!(nvse::commands::extender::loaded_plugins(&e).is_empty());
    // A second install does nothing.
    install(&mut e);
    assert_eq!(nvse::plugins::built_ins(&e).len(), 1);

    let mut e = engine(None);
    set_on(&mut e, true);
    set_keys(
        &mut e,
        rows::Keys {
            rows: vec![("bDialogSpeechNullCheck", false)],
            ..Default::default()
        },
    );
    install(&mut e);
    assert!(on(&e));
    assert!(!e.stewie(bits::DIALOG_SPEECH_NULL_CHECK));
}

#[test]
fn the_start_lines_name_what_was_read() {
    let keys = rows::Keys::default();
    let (settings, on) = settings_and_rows(None, &[], &keys);
    // Every row built is on with VNV's values (the count is the table's, so a share adding rows
    // does not edit this line).
    let built = rows::all().count();
    assert_eq!(on.len(), built);
    assert_eq!(
        start_lines(false, &settings, &keys, on.len()),
        [
            "lStewieAl's Tweaks: no Data\\NVSE\\Plugins\\nvse_stewie_tweaks.ini: VNV's values"
                .to_string(),
            format!("lStewieAl's Tweaks: {built} of its {built} rows built are on"),
        ]
    );
    let (settings, on) = settings_and_rows(Some(b"[Tweaks]\nbBetterAutoWalk=1\n"), &[], &keys);
    assert_eq!(
        start_lines(true, &settings, &keys, on.len())[0],
        "lStewieAl's Tweaks: read Data\\NVSE\\Plugins\\nvse_stewie_tweaks.ini: 0 of its 1 keys changed a value"
    );
}

/// The install reads the player's INI (here through the game's own file calls: no platform answers
/// the mods' files seam in a unit test) and keeps the settings for the rows that read them.
#[test]
fn the_install_reads_the_players_ini() {
    let mut e = engine(Some(b"[Tweaks]\r\nbBetterAutoWalk=0\r\n"));
    set_on(&mut e, true);
    install(&mut e);
    let state = e.ext::<Stewie>();
    assert_eq!(state.settings.long("Tweaks", "bBetterAutoWalk", -1), 0);
    assert_eq!(
        state.settings.long("Tweaks", "bConsoleNumpadSupport", -1),
        1
    );
    assert_eq!(state.settings.read.len(), 1);
    assert_eq!(
        (state.settings.read[0].keys, state.settings.read[0].changed),
        (1, 1)
    );
}

/// Row S0023's line obeys `[Tweaks] bInvalidBSFadeNodePrints` (on in Tweaks' code; VNV's INI does
/// not list it): on without a file, off when the player's file sets it to 0.
#[test]
fn the_fade_node_line_obeys_its_tweaks_key() {
    let mut e = engine(None);
    set_on(&mut e, true);
    install(&mut e);
    assert!(e.stewie_fade_node_prints());
    let mut e = engine(Some(b"[Tweaks]\r\nbInvalidBSFadeNodePrints=0\r\n"));
    set_on(&mut e, true);
    install(&mut e);
    assert!(!e.stewie_fade_node_prints());
    assert!(e.stewie(bits::INVALID_FADE_NODE), "the fix stays on");
}

fn listed(name: &str, is_folder: bool) -> engine_core::mod_files::Listed {
    engine_core::mod_files::Listed {
        name: name.as_bytes().to_vec(),
        is_folder,
    }
}

/// The overlay folder: its `.ini` files (any case, a file named just `.ini` too, as Windows' `*.ini`
/// gives it) in the listing's order; folders and other files left out.
#[test]
fn the_overlay_files_are_the_folders_ini_files_in_order() {
    let names = overlay_names(&[
        listed("b.ini", false),
        listed("A.INI", false),
        listed("sub.ini", true),
        listed("notes.txt", false),
        listed(".ini", false),
        listed("old.ini_bak", false),
        listed("c.Ini", false),
    ]);
    assert_eq!(
        names,
        [
            b"b.ini".to_vec(),
            b"A.INI".to_vec(),
            b".ini".to_vec(),
            b"c.Ini".to_vec()
        ]
    );
}

/// `[INI] bMultiINISupport` of the main file decides whether the folder is read (default 1, VNV's 1).
#[test]
fn the_main_file_can_turn_the_overlays_off() {
    assert!(multi_ini(None, false));
    assert!(multi_ini(None, true));
    assert!(multi_ini(Some(b"[INI]\nbMultiINISupport=1\n"), false));
    assert!(!multi_ini(Some(b"[INI]\nbMultiINISupport=0\n"), false));
    assert!(!multi_ini(Some(b"[ini]\nbmultiinisupport = 0\n"), true));
}

/// The overlays go over the main file in their order, key by key; a key they lack keeps the value
/// under it; each is named in the log with the keys it changed.
#[test]
fn the_overlays_go_over_the_main_file_in_order() {
    let overlays = vec![
        (
            r"Data\NVSE\Plugins\Tweaks\INIs\a.ini".to_string(),
            b"[Tweaks]\nbBetterAutoWalk=0\nbConsoleNumpadSupport=0\n".to_vec(),
        ),
        (
            r"Data\NVSE\Plugins\Tweaks\INIs\b.ini".to_string(),
            b"[Tweaks]\nbConsoleNumpadSupport=1\n".to_vec(),
        ),
    ];
    let keys = rows::Keys::default();
    let main = b"[Tweaks]\nbBetterAutoWalk=1\nbDescriptiveMarkerAddedMessage=0\n";
    let (settings, _) = settings_and_rows(Some(main), &overlays, &keys);
    assert_eq!(
        settings.long("Tweaks", "bBetterAutoWalk", -1),
        0,
        "a.ini over the main file"
    );
    assert_eq!(
        settings.long("Tweaks", "bConsoleNumpadSupport", -1),
        1,
        "b.ini over a.ini"
    );
    assert_eq!(
        settings.long("Tweaks", "bDescriptiveMarkerAddedMessage", -1),
        0,
        "the main file's"
    );
    assert_eq!(
        settings.log_lines(),
        [
            r"lStewieAl's Tweaks: read Data\NVSE\Plugins\nvse_stewie_tweaks.ini: 1 of its 2 keys changed a value",
            r"lStewieAl's Tweaks: read Data\NVSE\Plugins\Tweaks\INIs\a.ini: 2 of its 2 keys changed a value",
            r"lStewieAl's Tweaks: read Data\NVSE\Plugins\Tweaks\INIs\b.ini: 1 of its 1 keys changed a value",
        ]
    );
}

fn overlay(text: &str) -> (String, Vec<u8>) {
    (
        r"Data\NVSE\Plugins\Tweaks\INIs\m.ini".to_string(),
        text.as_bytes().to_vec(),
    )
}

/// Keys Tweaks reads from the main INI only (inside `if (isDefaultINI)`) keep the main file's value
/// whatever an overlay says, and a `GET_TWEAKH` key is ORed across the files: an overlay turns it on,
/// never off. Other keys: the last file wins.
#[test]
fn overlays_follow_tweaks_rules_for_main_only_and_ored_keys() {
    let keys = rows::Keys::default();
    // Main-only: [INI] bFixCreatureDoubledDR, [Hotkeys] iClearConsoleKey, [Tweaks]
    // bGodmodeAllowsFastTravelEnemiesNearby.
    let main = b"[INI]\nbFixCreatureDoubledDR=1\n[Hotkeys]\niClearConsoleKey=29\n\
        [Tweaks]\nbGodmodeAllowsFastTravelEnemiesNearby=1\n";
    let o = overlay(
        "[INI]\nbFixCreatureDoubledDR=0\n[Hotkeys]\niClearConsoleKey=0\n\
         [Tweaks]\nbGodmodeAllowsFastTravelEnemiesNearby=0\n",
    );
    let (settings, _) = settings_and_rows(Some(main), std::slice::from_ref(&o), &keys);
    assert_eq!(settings.long("INI", "bFixCreatureDoubledDR", -1), 1);
    assert_eq!(settings.long("Hotkeys", "iClearConsoleKey", -1), 29);
    assert_eq!(
        settings.long("Tweaks", "bGodmodeAllowsFastTravelEnemiesNearby", -1),
        1
    );
    assert_eq!(
        settings.raw("INI", "bFixCreatureDoubledDR"),
        Some(b"1".to_vec())
    );
    // The overlay's main-only keys count as not read.
    assert_eq!((settings.read[1].keys, settings.read[1].changed), (0, 0));
    // Without a main INI an overlay's main-only key is ignored too: the default stands.
    let five = overlay("[Hotkeys]\niClearConsoleKey=5\n");
    let (settings, _) = settings_and_rows(None, std::slice::from_ref(&five), &keys);
    assert_eq!(settings.long("Hotkeys", "iClearConsoleKey", -1), 0);

    // ORed (GET_TWEAKH): bInlineVanillaFunctions, bNoRecurringChallenges.
    let main = b"[Tweaks]\nbInlineVanillaFunctions=1\nbNoRecurringChallenges=0\n";
    let overlays = [
        overlay("[Tweaks]\nbInlineVanillaFunctions=0\nbNoRecurringChallenges=1\n"),
        overlay("[Tweaks]\nbNoRecurringChallenges=0\n"),
    ];
    let (settings, _) = settings_and_rows(Some(main), &overlays, &keys);
    assert_eq!(
        settings.long("Tweaks", "bInlineVanillaFunctions", -1),
        1,
        "main 1, overlay 0: stays 1"
    );
    assert_eq!(
        settings.long("Tweaks", "bNoRecurringChallenges", -1),
        1,
        "an overlay turns it on, a later 0 does not turn it off"
    );
    // A main file without the key: the default under it, then the overlays ORed in.
    let (settings, _) = settings_and_rows(Some(b"[Tweaks]\n"), &overlays[1..], &keys);
    assert_eq!(settings.long("Tweaks", "bNoRecurringChallenges", -1), 0);
    let (settings, _) = settings_and_rows(Some(b"[Tweaks]\n"), &overlays[..1], &keys);
    assert_eq!(settings.long("Tweaks", "bNoRecurringChallenges", -1), 1);
    // An ordinary key: the last file wins (an overlay turns it off).
    let (settings, _) = settings_and_rows(
        Some(b"[Tweaks]\nbBetterAutoWalk=1\n"),
        &[overlay("[Tweaks]\nbBetterAutoWalk=0\n")],
        &keys,
    );
    assert_eq!(settings.long("Tweaks", "bBetterAutoWalk", -1), 0);
}

/// Every key the rules name is in the table (28 main-only keys, 69 ORed).
#[test]
fn the_key_rules_name_keys_of_the_table() {
    let table: std::collections::HashSet<(Vec<u8>, Vec<u8>)> = table()
        .iter()
        .map(|s| {
            (
                s.section.as_bytes().to_ascii_lowercase(),
                s.key.as_bytes().to_ascii_lowercase(),
            )
        })
        .collect();
    let rules = crate::settings::rules();
    assert_eq!(rules.len(), 28 + 69);
    let main_only = rules
        .values()
        .filter(|r| **r == crate::settings::Rule::MainOnly)
        .count();
    assert_eq!(main_only, 28);
    for k in rules.keys() {
        assert!(table.contains(k), "{k:?}");
    }
}

// ---- The install's overlay path, through a stand-in of the mods' files seam ----

type ModFiles = (Vec<(Vec<u8>, Vec<u8>)>, Vec<engine_core::mod_files::Listed>);

thread_local! {
    /// This test thread's mod files (path, bytes) and the overlay folder's listing; `None`: the
    /// seam answers as without a platform (the other tests run as before).
    static MOD_FILES: std::cell::RefCell<Option<ModFiles>> = const { std::cell::RefCell::new(None) };
    /// The folders the seam was asked to list.
    static LISTED: std::cell::RefCell<Vec<Vec<u8>>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn seam_read(_: &Engine, name: &[u8]) -> Result<Vec<u8>, u32> {
    use engine_core::mod_files::{ERROR_FILE_NOT_FOUND, ERROR_NOT_SUPPORTED};
    MOD_FILES.with(|m| match &*m.borrow() {
        None => Err(ERROR_NOT_SUPPORTED),
        Some((files, _)) => files
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, b)| b.clone())
            .ok_or(ERROR_FILE_NOT_FOUND),
    })
}

fn seam_exists(e: &Engine, name: &[u8]) -> bool {
    seam_read(e, name).is_ok()
}

fn seam_no_folder(_: &Engine, _: &[u8]) -> bool {
    false
}

fn seam_no_create(_: &mut Engine, _: &[u8]) -> Result<(), u32> {
    Err(engine_core::mod_files::ERROR_NOT_SUPPORTED)
}

fn seam_no_write(_: &mut Engine, _: &[u8], _: &[u8]) -> Result<(), u32> {
    Err(engine_core::mod_files::ERROR_NOT_SUPPORTED)
}

fn seam_read_head(e: &Engine, name: &[u8], max: usize) -> Result<Vec<u8>, u32> {
    seam_read(e, name).map(|mut b| {
        b.truncate(max);
        b
    })
}

fn seam_list(_: &Engine, folder: &[u8]) -> Vec<engine_core::mod_files::Listed> {
    MOD_FILES.with(|m| match &*m.borrow() {
        None => Vec::new(),
        Some((_, listing)) => {
            LISTED.with(|l| l.borrow_mut().push(folder.to_vec()));
            if folder.eq_ignore_ascii_case(b"Data\\NVSE\\Plugins\\Tweaks\\INIs\\") {
                listing.clone()
            } else {
                Vec::new()
            }
        }
    })
}

fn seam_name_key(name: &[u8]) -> Vec<u16> {
    name.iter()
        .map(|b| u16::from(b.to_ascii_uppercase()))
        .collect()
}

static SEAM: engine_core::mod_files::ModFileCalls = engine_core::mod_files::ModFileCalls {
    read: seam_read,
    exists: seam_exists,
    directory_exists: seam_no_folder,
    create_directory: seam_no_create,
    write: seam_no_write,
    append: seam_no_write,
    read_head: seam_read_head,
    list: seam_list,
    name_key: seam_name_key,
};

/// Installs the stand-in seam (once for the test program) with these files for this thread.
fn mod_files(files: &[(&str, &str)], listing: &[(&str, bool)]) {
    engine_core::mod_files::install(&SEAM);
    let files = files
        .iter()
        .map(|(n, b)| (n.as_bytes().to_vec(), b.as_bytes().to_vec()))
        .collect();
    let listing = listing.iter().map(|(n, f)| listed(n, *f)).collect();
    MOD_FILES.with(|m| *m.borrow_mut() = Some((files, listing)));
    LISTED.with(|l| l.borrow_mut().clear());
}

const MAIN_INI: &str = r"Data\NVSE\Plugins\nvse_stewie_tweaks.ini";

/// `install` itself: the main INI and then the folder's overlay files, in the listing's order (a
/// folder and a file that is not `.ini` left out), each named in the log by its path in the folder;
/// `[INI] bMultiINISupport=0` in the main INI leaves the folder unread.
#[test]
fn the_install_reads_the_overlay_folder_in_order() {
    let mut files = [
        (
            MAIN_INI,
            "[Tweaks]\nbBetterAutoWalk=1\nbConsoleNumpadSupport=1\nbDescriptiveMarkerAddedMessage=1\n",
        ),
        (
            r"Data\NVSE\Plugins\Tweaks\INIs\b.ini",
            "[Tweaks]\nbBetterAutoWalk=0\nbConsoleNumpadSupport=0\n",
        ),
        (
            r"Data\NVSE\Plugins\Tweaks\INIs\A.INI",
            "[Tweaks]\nbConsoleNumpadSupport=1\n",
        ),
        (
            r"Data\NVSE\Plugins\Tweaks\INIs\c.ini",
            "[Tweaks]\nbDescriptiveMarkerAddedMessage=0\n",
        ),
        (
            r"Data\NVSE\Plugins\Tweaks\INIs\notes.txt",
            "[Tweaks]\nbBetterAutoWalk=1\n",
        ),
    ];
    let listing = [
        ("b.ini", false),
        ("sub.ini", true),
        ("A.INI", false),
        ("notes.txt", false),
        ("c.ini", false),
    ];
    mod_files(&files, &listing);
    let mut e = engine(None);
    set_on(&mut e, true);
    install(&mut e);
    let state = e.ext::<Stewie>();
    let read: Vec<&str> = state
        .settings
        .read
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(
        read,
        [
            MAIN_INI,
            r"Data\NVSE\Plugins\Tweaks\INIs\b.ini",
            r"Data\NVSE\Plugins\Tweaks\INIs\A.INI",
            r"Data\NVSE\Plugins\Tweaks\INIs\c.ini",
        ]
    );
    // b.ini over the main file, A.INI over b.ini, c.ini last.
    assert_eq!(state.settings.long("Tweaks", "bBetterAutoWalk", -1), 0);
    assert_eq!(
        state.settings.long("Tweaks", "bConsoleNumpadSupport", -1),
        1
    );
    assert_eq!(
        state
            .settings
            .long("Tweaks", "bDescriptiveMarkerAddedMessage", -1),
        0
    );
    assert_eq!(
        state.log[..4],
        [
            r"lStewieAl's Tweaks: read Data\NVSE\Plugins\nvse_stewie_tweaks.ini: 0 of its 3 keys changed a value",
            r"lStewieAl's Tweaks: read Data\NVSE\Plugins\Tweaks\INIs\b.ini: 2 of its 2 keys changed a value",
            r"lStewieAl's Tweaks: read Data\NVSE\Plugins\Tweaks\INIs\A.INI: 1 of its 1 keys changed a value",
            r"lStewieAl's Tweaks: read Data\NVSE\Plugins\Tweaks\INIs\c.ini: 1 of its 1 keys changed a value",
        ]
    );
    assert_eq!(
        LISTED.with(|l| l.borrow().clone()),
        [b"Data\\NVSE\\Plugins\\Tweaks\\INIs\\".to_vec()]
    );

    // The main file turns the folder off: only the main file is read and the folder is not listed.
    files[0].1 = "[INI]\nbMultiINISupport=0\n[Tweaks]\nbBetterAutoWalk=1\n";
    mod_files(&files, &listing);
    let mut e = engine(None);
    set_on(&mut e, true);
    install(&mut e);
    let state = e.ext::<Stewie>();
    assert_eq!(state.settings.read.len(), 1);
    assert_eq!(state.settings.long("Tweaks", "bBetterAutoWalk", -1), 1);
    assert!(LISTED.with(|l| l.borrow().is_empty()));
    MOD_FILES.with(|m| *m.borrow_mut() = None);
}

/// Each row's plan number, off key and bit, pinned (crash groups A, B and C, Engine Optimizations'
/// rows): a key swapped with another row's, or two bits swapped, fails here. After the install,
/// each key at 0 clears exactly its own bit.
#[test]
fn each_rows_plan_number_key_and_bit_are_pinned() {
    use engine_core::extend::stewie::{Row, CRASH, FIXES};
    let pinned: &[(&'static str, &'static str, Row)] = &[
        (
            "S0018",
            "bFixUGridsCrashAndAllow3AsMinimum",
            Row::at(CRASH, 33),
        ),
        (
            "S0022.14",
            "bMenuFixesInventoryUseNoItem",
            Row::at(CRASH, 11),
        ),
        (
            "S0022.15",
            "bMenuFixesQuantityMenuCloseNoItem",
            Row::at(CRASH, 12),
        ),
        (
            "S0022.16",
            "bMenuFixesInventorySelectNoItem",
            Row::at(CRASH, 13),
        ),
        (
            "S0022.17",
            "bMenuFixesContainerMouseoverNoForm",
            Row::at(CRASH, 14),
        ),
        (
            "S0022.30",
            "bMenuFixesConsoleWhileMenusLoad",
            Row::at(CRASH, 15),
        ),
        ("S0022.36", "bMenuFixesListBoxNullItem", Row::at(CRASH, 16)),
        (
            "S0022.43",
            "bMenuFixesFontIconNoTexture",
            Row::at(CRASH, 68),
        ),
        ("S0022.48", "bMenuFixesRepairTargetKept", Row::at(CRASH, 69)),
        (
            "S0022.49",
            "bMenuFixesSelectableRefFreed",
            Row::at(CRASH, 70),
        ),
        (
            "S0022.56",
            "bMenuFixesTileLabelNullName",
            Row::at(CRASH, 17),
        ),
        ("S0023", "bPrintBSXCrash", Row::at(CRASH, 62)),
        (
            "S0025",
            "bFixCrashWhenControllerReconnectsInHackingMenu",
            Row::at(CRASH, 71),
        ),
        ("S0027", "bFixPipboyEquipCrash", Row::at(CRASH, 28)),
        ("S0031", "bAddNifShapeNullChecks", Row::at(CRASH, 63)),
        (
            "S0036",
            "bRemoveInvalidCasinoDataChipsInSlotMachineMenu",
            Row::at(CRASH, 49),
        ),
        ("S0037", "bFixNullParticleSystemsCrash", Row::at(CRASH, 55)),
        ("S0042", "bFixExplosionWaterNullCall", Row::at(CRASH, 1)),
        (
            "S0043",
            "bFixNullWeaponOrWeaponInfoReloadLoopCrash",
            Row::at(CRASH, 21),
        ),
        (
            "S0045",
            "bFixGetItemValueCrashForModdedWeapons",
            Row::at(CRASH, 34),
        ),
        ("S0048", "bFixNullFaceGenCrash", Row::at(CRASH, 56)),
        ("S0049", "bFixUnresolvedRaceCrash", Row::at(CRASH, 35)),
        ("S0057", "bFixLevelGreaterThan99Crash", Row::at(CRASH, 18)),
        (
            "S0064",
            "bFixHandleMultiboundsCrashWithNullCell",
            Row::at(CRASH, 2),
        ),
        (
            "S0067",
            "bFixGetWeaponDamageCrashOnThrowingWeapons",
            Row::at(CRASH, 36),
        ),
        ("S0068", "bFixHandleStealingCrash", Row::at(CRASH, 3)),
        ("S0077", "bBodyPartDataNullChecks", Row::at(CRASH, 37)),
        ("S0078", "bNullPlayerCellImposterCheck", Row::at(CRASH, 4)),
        ("S0079", "bDialogSpeechNullCheck", Row::at(CRASH, 0)),
        (
            "S0089.04",
            "bRememberAmmoCountOnLoadReloadNoAmmoEntry",
            Row::at(CRASH, 31),
        ),
        (
            "S0089.05",
            "bRememberAmmoCountOnLoadDropNoAmmoEntry",
            Row::at(CRASH, 32),
        ),
        (
            "S0093.07",
            "bPerkMenuDescriptionTypoFixTooLong",
            Row::at(CRASH, 38),
        ),
        (
            "S0106",
            "bFixNullHitLocationNodeExplosionCrash",
            Row::at(CRASH, 5),
        ),
        ("S0107.01", "bMemoryLeakFixesRumble", Row::at(CRASH, 60)),
        (
            "S0107.03",
            "bMemoryLeakFixesPackageTarget",
            Row::at(CRASH, 42),
        ),
        (
            "S0107.04",
            "bMemoryLeakFixesFocusSpeaker",
            Row::at(CRASH, 61),
        ),
        ("S0108", "bFixNullNifOwnerCrash", Row::at(CRASH, 57)),
        (
            "S0124",
            "bProcessActivateAddNullCheckForPackage",
            Row::at(CRASH, 6),
        ),
        (
            "S0126",
            "bFixNullProjectileInCombatStateUpdateInventory",
            Row::at(CRASH, 29),
        ),
        (
            "S0127",
            "bFixRecipeMenuMissingAmmoInfoNullCheck",
            Row::at(CRASH, 30),
        ),
        (
            "S0130.01",
            "bContinuousBeamProjectileCrashFixShooter",
            Row::at(CRASH, 43),
        ),
        (
            "S0130.03",
            "bContinuousBeamProjectileCrashFixLoad",
            Row::at(CRASH, 44),
        ),
        (
            "S0136",
            "bTryCatchingQueuedTextureLoadCrash",
            Row::at(CRASH, 67),
        ),
        (
            "S0139",
            "bFixCreditsCrashIfNoLoadedSave",
            Row::at(CRASH, 19),
        ),
        ("S0140", "bFixPatrolMissingNullCheck", Row::at(CRASH, 7)),
        ("S0150", "bFixMissingAnimNullCheck", Row::at(CRASH, 22)),
        ("S0155", "bNullAudioCrash", Row::at(CRASH, 65)),
        ("S0169", "bFixRareNullTeleportDoorCrash", Row::at(CRASH, 8)),
        (
            "S0172",
            "bFixAnimDataUpdateReloadLoopNullCheck",
            Row::at(CRASH, 23),
        ),
        ("S0173", "bAnimDataUpdateCrashCheck", Row::at(CRASH, 24)),
        (
            "S0174",
            "bFixSetActorRefEssentialNullCheck",
            Row::at(CRASH, 9),
        ),
        ("S0181", "bNullAnimRemoveRecurseFix", Row::at(CRASH, 25)),
        (
            "S0182",
            "bHolotapeEmptySubtitleCrashFix",
            Row::at(CRASH, 20),
        ),
        ("S0183", "bFollowerTopicUseAfterFreeFix", Row::at(CRASH, 45)),
        ("S0186", "bBipedAnimNullItemError", Row::at(CRASH, 26)),
        (
            "S0196.01",
            "bRouletteMenuFixesBetListDoubleFree",
            Row::at(CRASH, 50),
        ),
        (
            "S0196.02",
            "bRouletteMenuFixesBetsFreedOnClose",
            Row::at(CRASH, 51),
        ),
        (
            "S0196.03",
            "bRouletteMenuFixesBetFreedOnUse",
            Row::at(CRASH, 52),
        ),
        (
            "S0199.02",
            "bMiscCrashFixesDismemberLimb",
            Row::at(CRASH, 64),
        ),
        ("S0199.03", "bMiscCrashFixesAmmoEntry", Row::at(CRASH, 46)),
        (
            "S0199.04",
            "bMiscCrashFixesEntityActivate",
            Row::at(CRASH, 58),
        ),
        ("S0199.05", "bMiscCrashFixesNpcActivate", Row::at(CRASH, 39)),
        ("S0199.06", "bMiscCrashFixesRemoveWorn", Row::at(CRASH, 40)),
        (
            "S0199.07",
            "bMiscCrashFixesAddReference",
            Row::at(CRASH, 59),
        ),
        (
            "S0199.08",
            "bMiscCrashFixesCompanionStimpak",
            Row::at(CRASH, 53),
        ),
        (
            "S0199.09",
            "bMiscCrashFixesShowSceneGraph",
            Row::at(CRASH, 41),
        ),
        ("S0199.10", "bMiscCrashFixesUseDoor", Row::at(CRASH, 47)),
        ("S0199.11", "bMiscCrashFixesRegenAmmo", Row::at(CRASH, 48)),
        ("S0199.12", "bMiscCrashFixesListBoxName", Row::at(CRASH, 54)),
        (
            "S0202",
            "bEquipAmmoOnNullBaseProcessCrashFix",
            Row::at(CRASH, 27),
        ),
        ("S0225", "bSamplerStateLeakFix", Row::at(CRASH, 66)),
        ("S0881", "bFixSavingNullCellCrash", Row::at(CRASH, 10)),
        (
            "S1016.845558",
            "bInlinesChangesMapBuckets",
            Row::at(FIXES, 0),
        ),
    ];
    let pinned: Vec<(&str, &str, Row)> = [
        pinned,
        world::PINNED,
        forms::PINNED,
        ai::PINNED,
        menus::PINNED,
    ]
    .concat();
    let table: Vec<(&str, &str, Row)> = rows::all().map(|r| (r.plan, r.key, r.bit)).collect();
    assert_eq!(table, pinned);
    for (plan, key, bit) in &pinned {
        let mut e = engine(None);
        set_on(&mut e, true);
        set_keys(
            &mut e,
            rows::Keys {
                rows: vec![(*key, false)],
                ..Default::default()
            },
        );
        install(&mut e);
        for (_, _, other) in &pinned {
            assert_eq!(e.stewie(*other), other != bit, "{plan}'s key at 0");
        }
    }
}

/// `Row::share` stops exactly at a share's end: the share's last bit is a row, the next one is
/// refused.
#[test]
#[should_panic(expected = "past the share's rows")]
fn a_share_row_at_its_end_is_refused() {
    let n = std::hint::black_box(bits::AI_FIXES_END - bits::AI_FIXES);
    let _ = bits::Row::share(bits::AI_FIXES, bits::AI_FIXES_END, n);
}

/// The last menus tweak is the table's bit 1023, the bit before the other switches' group.
#[test]
fn the_last_share_row_is_bit_1023() {
    let n = std::hint::black_box(bits::MENUS_TWEAKS_END - bits::MENUS_TWEAKS - 1);
    let last = bits::Row::share(bits::MENUS_TWEAKS, bits::MENUS_TWEAKS_END, n);
    assert_eq!(last.index(), 1023);
    assert_eq!(usize::from(bits::OTHER_TWEAKS), 1024);
}

/// The two Tweaks keys group 8's rows read inside their seams reach the engine at the install:
/// `[Tweaks] bRunAndGunAPInCombatOnly` (S0097) and `bVATSStopBurstIfTargetDead` (S0082.01), code
/// default 0 each.
#[test]
fn the_install_reads_the_run_and_gun_and_vats_burst_keys() {
    let mut e = engine(None);
    set_on(&mut e, true);
    install(&mut e);
    assert!(!e.stewie_run_and_gun_combat_only());
    assert!(!e.stewie_vats_stop_burst());

    let mut e = engine(Some(
        b"[Tweaks]\r\nbRunAndGunAPInCombatOnly=1\r\nbVATSStopBurstIfTargetDead=1\r\n",
    ));
    set_on(&mut e, true);
    install(&mut e);
    assert!(e.stewie_run_and_gun_combat_only());
    assert!(e.stewie_vats_stop_burst());
}

/// S0097 (`patchRunAndGunDontLetAPBeNegative`): with the row the install points `PlayerCharacter`'s
/// vtable slot `0108AD7C` at Tweaks' function; with its key `bRunAndGunDontLetAPBeNegative` off the
/// slot keeps `0093ab30`.
#[test]
fn s0097_the_install_rewrites_only_the_vtable_slot() {
    for row_on in [true, false] {
        let mut e = engine(None);
        e.map(run_and_gun::SLOT & !0xfff, 0x1000);
        e.mem.set_u32(run_and_gun::SLOT, 0x0093_ab30);
        set_on(&mut e, true);
        if !row_on {
            set_keys(
                &mut e,
                rows::Keys {
                    rows: vec![("bRunAndGunDontLetAPBeNegative", false)],
                    ..Default::default()
                },
            );
        }
        install(&mut e);
        let expected = if row_on {
            run_and_gun::ROUTINE
        } else {
            0x0093_ab30
        };
        assert_eq!(e.mem.u32(run_and_gun::SLOT), expected, "row {row_on}");
    }
}

/// S0097 through the game's virtual call (slot `+0x340` of a `PlayerCharacter` whose vtable holds
/// the slot `0108AD7C`): the action's cost (`0066dca0`: 30) is held to the player's action points
/// (`0066ef50` with 0xC) and spent (slot `0x33c`) when above 0; outside VATS out of combat (+0xDF0)
/// with `bRunAndGunAPInCombatOnly` nothing is taken; in VATS (+8 of `011f2250`) it is.
#[test]
fn s0097_the_virtual_call_spends_at_most_the_points_left() {
    const SPEND: u32 = 0x7d00_033c;
    // (the points, combat only, in combat, in VATS, spent)
    for (points, only, combat, vats, spent) in [
        (20.0f32, false, false, false, Some(20.0f32)),
        (40.0, false, false, false, Some(30.0)),
        (0.0, false, false, false, None),
        (20.0, true, false, false, None),
        (20.0, true, true, false, Some(20.0)),
        (20.0, true, false, true, Some(20.0)),
    ] {
        let mut e = Engine::new();
        e.set_stewie(bits::RUN_AND_GUN_AP, true);
        e.set_stewie_run_and_gun_combat_only(only);
        let table = run_and_gun::SLOT - 0x340;
        e.map(table & !0xfff, 0x2000);
        e.map(0x011d_e000, 0x1000);
        e.map(0x011f_2000, 0x1000);
        run_and_gun::install(&mut e);
        let player = e.mem.alloc(0xe00);
        e.mem.set_u32(player, table);
        e.mem.set_u8(player + 0xdf0, u8::from(combat));
        e.set_global(0x011d_ea3cu32, player);
        e.mem.set_u32(0x011f_2250 + 8, u32::from(vats));
        e.mem.set_u32(table + 0x33c, SPEND);
        e.register_double(SPEND, |_, _| Ret::default());
        e.register_double(0x0066_dca0, |_, _| Ret {
            st0: 30.0,
            ..Ret::default()
        });
        e.register_double(0x0066_ef50, move |_, _| Ret {
            st0: f64::from(points),
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        e.vcall(player, 0x340, &args![9u32]);
        let spends: Vec<Vec<u32>> = e
            .call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == SPEND)
            .map(|(_, w)| w.clone())
            .collect();
        let label = format!("{points} {only} {combat} {vats}");
        match spent {
            Some(v) => assert_eq!(spends, vec![vec![player, v.to_bits()]], "{label}"),
            None => assert!(spends.is_empty(), "{label}"),
        }
    }
}
