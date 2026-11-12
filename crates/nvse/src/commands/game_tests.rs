//! Tests of `Commands_Game.cpp` through the commands' records. The answers pinned are xNVSE's as
//! its source and the shipped DLL (`nvse_1_4.dll`, executes `10025250` to `10025D63`) give them;
//! none of these commands has been run in the real game yet.

use super::super::test_support::*;
use super::*;
use std::cell::RefCell;

const MANAGER: u32 = 0x0200_0000;
const CROSSHAIR: u32 = 0x0200_0400;
const SELECTION: u32 = 0x0200_0500;
const ITEM: u32 = 0x0200_0600;
const THIS: u32 = 0x0200_0700;
const HANDLER: u32 = 0x0200_1000;
const FILES: u32 = 0x0200_2000;
const SETTINGS: u32 = 0x0200_5000;
const PREFS: u32 = 0x0200_6000;
const INIS: u32 = 0x0200_6400;
const GMSTS: u32 = 0x0200_6800;
const TEXTS: u32 = 0x0200_8000;
const CONSOLE: u32 = 0x0200_f000;

/// A message box as the fake saw it: its words, and the texts of its message and buttons read
/// at the call (the buffer is on the stack).
struct MessageBox {
    words: Vec<u32>,
    message: String,
    buttons: Vec<String>,
}

/// What the fakes saw: console lines (format, first words), messages queued, message boxes.
#[derive(Default)]
struct Seen {
    lines: Vec<(Vec<u8>, Vec<u32>)>,
    messages: Vec<(Vec<u8>, Vec<u32>)>,
    boxes: Vec<MessageBox>,
    spells: u32,
    lookups: u32,
}

/// A record's opcode, names and parameters (name, type, optional).
type Record<'a> = (u32, &'a str, &'a str, &'a [(&'a str, u32, bool)]);

thread_local! {
    static SEEN: RefCell<Seen> = RefCell::new(Seen::default());
    /// The game settings the fake map lookup finds: (name, setting).
    static KNOWN_GMSTS: RefCell<Vec<(Vec<u8>, u32)>> = const { RefCell::new(Vec::new()) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn seen<T>(f: impl FnOnce(&mut Seen) -> T) -> T {
    SEEN.with(|s| f(&mut s.borrow_mut()))
}

fn lines() -> Vec<(Vec<u8>, Vec<u32>)> {
    seen(|s| std::mem::take(&mut s.lines))
}

/// `NiTMapBase::Lookup` over the case-insensitive map: the settings of [`KNOWN_GMSTS`].
fn lookup(e: &mut Engine, a: &[u32]) -> Ret {
    seen(|s| s.lookups += 1);
    assert_eq!(a[0], GMSTS + 0x10c, "the collection's map");
    let key = crate::table::guest_text(e, a[1]);
    let found = KNOWN_GMSTS.with(|g| {
        g.borrow()
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&key))
            .map(|&(_, s)| s)
    });
    match found {
        Some(setting) => {
            e.mem.set_u32(a[2], setting);
            eax(1)
        }
        None => eax(0),
    }
}

/// A `Setting` at `at`: its data word and name.
fn setting(e: &mut Engine, at: u32, name: &str, data: u32) -> u32 {
    let text = at + 0x10;
    e.mem.write(text, name.as_bytes());
    e.mem.set_u8(text + name.len() as u32, 0);
    e.mem.set_u32(at + 4, data);
    e.mem.set_u32(at + 8, text);
    at
}

/// A `tList` whose first node is at `head`, holding `items` (0 for an empty node).
fn tlist(e: &mut Engine, head: u32, items: &[u32], spare: u32) {
    let mut node = head;
    for (i, &item) in items.iter().enumerate() {
        e.mem.set_u32(node, item);
        let next = if i + 1 < items.len() {
            spare + 8 * i as u32
        } else {
            0
        };
        e.mem.set_u32(node + 4, next);
        node = next;
    }
}

/// The table, ASCII `toupper`, the console, the interface manager, three files (an empty node
/// between the first two), game settings, INI settings, the message functions.
fn setup() -> Engine {
    seen(|s| *s = Seen::default());
    let mut e = engine_with_table();
    for page in [
        INTERFACE_MANAGER,
        GRIDS_TO_LOAD,
        DATA_HANDLER,
        super::super::string::GAME_SETTINGS,
        super::super::string::INI_PREFS,
        super::super::string::INI_SETTINGS,
        MESSAGE_BOX_OWNER,
        MESSAGE_BOX_ANSWER,
    ] {
        e.map(page & !0xfff, 0x1000);
    }
    e.register(0x00ec_a7f4, |_, a| {
        eax(u32::from((a[0] as u8).to_ascii_uppercase()))
    });
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    e.register(0x0071_d0a0, |e, a| {
        let format = crate::table::guest_text(e, a[1]);
        let words = (0..3).map(|i| e.mem.u32(a[2] + 4 * i)).collect();
        seen(|s| s.lines.push((format, words)));
        eax(0)
    });
    e.register(0x0085_3130, lookup);
    e.register(QUEUE_UI_MESSAGE, |e, a| {
        let text = crate::table::guest_text(e, a[0]);
        seen(|s| s.messages.push((text, a[1..].to_vec())));
        eax(0)
    });
    e.register(SHOW_MESSAGE_BOX, |e, a| {
        let text =
            |e: &Engine, at: u32| String::from_utf8(crate::table::guest_text(e, at)).unwrap();
        let message = text(e, a[0]);
        let buttons = a[8..]
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| text(e, b))
            .collect();
        seen(|s| {
            s.boxes.push(MessageBox {
                words: a.to_vec(),
                message,
                buttons,
            })
        });
        eax(1)
    });
    e.register(ADD_SPELL, |e, a| {
        seen(|s| s.spells += 1);
        let text = crate::table::guest_str(e, b"Effect added");
        e.call(
            QUEUE_UI_MESSAGE,
            &args![text, 0u32, 0u32, 0u32, 2.0f32, 0u32],
        );
        // As the game's: the result 1.0 for an actor, its line answered.
        e.mem.set_f64(a[6], 1.0);
        eax(1)
    });
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    // The interface manager, without a crosshair reference or selection yet.
    e.set_global(INTERFACE_MANAGER, MANAGER);
    for (form, id) in [
        (CROSSHAIR, 0x0003_1234u32),
        (SELECTION, 0x0004_5678),
        (ITEM, 0x0a00_1234),
        (THIS, 0xff80_0001),
    ] {
        e.mem.set_u32(form + 0xc, id);
    }
    set_script_ref(&mut e, 1, ITEM);
    // The files: FalloutNV.esm, an empty node, Mod.esp, Other.esp.
    e.set_global(DATA_HANDLER, HANDLER);
    let names = ["FalloutNV.esm", "Mod.esp", "Other.esp"];
    let infos: Vec<u32> = names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let info = FILES + 0x100 * i as u32;
            e.mem.write(info + 0x20, name.as_bytes());
            e.mem.set_u8(info + 0x20 + name.len() as u32, 0);
            info
        })
        .collect();
    tlist(
        &mut e,
        HANDLER + 0x210,
        &[infos[0], 0, infos[1], infos[2]],
        FILES + 0x800,
    );
    // Game settings (found by the fake map) and INI settings (two lists, the preferences first).
    let gmst = [
        setting(&mut e, SETTINGS, "iCount", -5i32 as u32),
        setting(&mut e, SETTINGS + 0x40, "uCount", 0xffff_ffff),
        setting(&mut e, SETTINGS + 0x80, "bFlag", 7),
        setting(&mut e, SETTINGS + 0xc0, "fScale", 1.5f32.to_bits()),
        setting(&mut e, SETTINGS + 0x100, "sName", TEXTS),
        setting(&mut e, SETTINGS + 0x140, "xOther", 3),
    ];
    KNOWN_GMSTS.with(|g| {
        *g.borrow_mut() = gmst
            .iter()
            .map(|&s| (crate::table::guest_text(&e, e.mem.u32(s + 8)), s))
            .collect();
    });
    e.set_global(super::super::string::GAME_SETTINGS, GMSTS);
    let pref = setting(&mut e, SETTINGS + 0x200, "iSize:Display", 1);
    let no_name = SETTINGS + 0x240;
    let ini_size = setting(&mut e, SETTINGS + 0x280, "iSize:Display", 2);
    let gamma = setting(&mut e, SETTINGS + 0x2c0, "fGamma:Display", 2.2f32.to_bits());
    tlist(&mut e, PREFS + 0x10c, &[0, no_name, pref], SETTINGS + 0x400);
    tlist(&mut e, INIS + 0x10c, &[ini_size, gamma], SETTINGS + 0x440);
    e.set_global(super::super::string::INI_PREFS, PREFS);
    e.set_global(super::super::string::INI_SETTINGS, INIS);
    e
}

fn console_on(e: &mut Engine) {
    e.mem.set_u8(0x011d_ea2e, 1);
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, 1);
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, this, args);
    assert_eq!(ok, 1);
    result
}

/// Calls `opcode`'s execute once, as [`call`] does, but with the result double already holding
/// `start` and the script `script` (0 for none); answers the result's bits.
fn run_over(e: &mut Engine, opcode: u32, script: u32, args: &[Vec<u8>], start: f64) -> u64 {
    let bytes: Vec<u8> = args.concat();
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, bytes.len() as u16 + 2);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    e.mem.write(DATA + 6, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, start);
    let record = crate::table::record_of(e, opcode);
    let (params, execute) = (e.mem.u32(record + 0x14), e.mem.u32(record + 0x18));
    e.call(execute, &[params, DATA, 0, 0, script, LIST, RESULT, OFFSET]);
    e.mem.u64(RESULT)
}

/// Every record as xNVSE's DLL has it (`dllrec.pl` over `nvse_1_4.dll`): opcode, names, help
/// text, parameters (name, type, optional), no calling reference needed, not condition-capable;
/// and the return types `CommandTable.cpp` registers (`ADD_CMD_RET`): a form for
/// `GetCrosshairRef`, `GetDebugSelection` and `BuildRef`, a number for the rest.
#[test]
fn the_records_match_the_dll() {
    let e = engine_with_table();
    const S: (&str, u32, bool) = ("string", 0, false);
    const F: (&str, u32, bool) = ("float", 2, false);
    const I: (&str, u32, bool) = ("int", 1, false);
    const FORM: (&str, u32, bool) = ("form", 0x3d, true);
    let mut format = vec![("format string", 0, false)];
    format.extend([("variable", 2, true); 20]);
    let want: [(Record, Returns); 20] = [
        (
            (0x149a, "GetNumericGameSetting", "", &[S]),
            Returns::Default,
        ),
        (
            (0x149b, "SetNumericGameSetting", "", &[S, F]),
            Returns::Default,
        ),
        ((0x149c, "GetNumericIniSetting", "", &[S]), Returns::Default),
        (
            (0x149d, "SetNumericIniSetting", "", &[S, F]),
            Returns::Default,
        ),
        ((0x14a7, "GetCrosshairRef", "", &[]), Returns::Form),
        ((0x14a8, "GetGameRestarted", "", &[]), Returns::Default),
        ((0x14ac, "GetGameLoaded", "", &[]), Returns::Default),
        ((0x14ae, "IsModLoaded", "", &[S]), Returns::Default),
        ((0x14af, "GetModIndex", "", &[S]), Returns::Default),
        ((0x14b0, "GetNumLoadedMods", "", &[]), Returns::Default),
        ((0x14b1, "GetSourceModIndex", "", &[FORM]), Returns::Default),
        ((0x14b2, "GetDebugSelection", "", &[]), Returns::Form),
        ((0x14d8, "GetLocalRefIndex", "", &[FORM]), Returns::Default),
        ((0x14d9, "BuildRef", "", &[I, I]), Returns::Form),
        ((0x14db, "MessageEx", "", &format), Returns::Default),
        ((0x14dc, "MessageBoxEx", "", &format), Returns::Default),
        ((0x15e6, "GetGridsToLoad", "", &[]), Returns::Default),
        (
            (0x15e7, "OutputLocalMapPicturesOverride", "OLMPOR", &[]),
            Returns::Default,
        ),
        (
            (
                0x15e8,
                "SetOutputLocalMapPicturesGrids",
                "SetOLMPGrids",
                &[I],
            ),
            Returns::Default,
        ),
        (
            (0x15ef, "AddSpellNS", "", &[("spell", 7, false)]),
            Returns::Default,
        ),
    ];
    let helps = [
        "returns the value of a numeric game setting",
        "sets a numeric game setting",
        "returns the value of an ini setting",
        "sets an ini setting",
        "returns the reference under the crosshair",
        "returns 1 when the game is restarted",
        "returns 1 after a game is loaded",
        "returns the whether the specified mod is loaded",
        "returns the index of the specified mod",
        "returns the number of loaded mods",
        "returns the index of the mod associated with the form",
        "returns the current selected object in the console",
        "returns the index of the ref",
        "builds a reference from a mod and ref index",
        "prints a formatted message",
        "displays a formatted message box",
        "returns the effective value of the uGridsToLoad ini setting",
        "sets the value with which to override uGridsToLoad when generating local maps with OLMPOR",
        "identical to AddSpell but without the UI message",
    ];
    let text = |at: u32| crate::table::guest_text(&e, e.mem.u32(at));
    for (((opcode, name, short, params), returns), help) in want.into_iter().zip(helps) {
        let record = crate::table::record_of(&e, opcode);
        assert_eq!(text(record), name.as_bytes(), "{opcode:04x}");
        assert_eq!(text(record + 4), short.as_bytes(), "{name}");
        assert_eq!(text(record + 0xc), help.as_bytes(), "{name}");
        assert_eq!(
            e.mem.u16(record + 0x10),
            0,
            "{name}: no calling reference needed"
        );
        assert_eq!(
            u32::from(e.mem.u16(record + 0x12)),
            params.len() as u32,
            "{name}"
        );
        let at = e.mem.u32(record + 0x14);
        for (i, &(pname, kind, optional)) in params.iter().enumerate() {
            let p = at + 12 * i as u32;
            assert_eq!(text(p), pname.as_bytes(), "{name} {i}");
            assert_eq!(e.mem.u32(p + 4), kind, "{name} {i}");
            assert_eq!(e.mem.u32(p + 8), u32::from(optional), "{name} {i}");
        }
        assert_eq!(
            (e.mem.u32(record + 0x20), e.mem.u32(record + 0x24)),
            (0, 0),
            "{name}"
        );
        assert_eq!(crate::table::return_type(&e, opcode), returns, "{name}");
    }
}

/// A game setting's number by its type letter: a bool's and an integer's word signed, an
/// unsigned's unsigned, a float's; -1 for a string or another type, an unknown name or an empty
/// name (not looked up). The console prints `%g` or SETTING NOT FOUND.
#[test]
fn numeric_game_settings_read_by_type() {
    let mut e = setup();
    for (name, want) in [
        ("icount", -5.0),
        ("uCount", 4_294_967_295.0),
        ("bFlag", 7.0),
        ("fScale", 1.5),
        ("sName", -1.0),
        ("xOther", -1.0),
        ("Missing", -1.0),
    ] {
        assert_eq!(run(&mut e, 0x149a, 0, &[s(name)]), want, "{name}");
    }
    let asked = seen(|s| s.lookups);
    assert_eq!(run(&mut e, 0x149a, 0, &[s("")]), -1.0);
    assert_eq!(seen(|s| s.lookups), asked, "an empty name is not looked up");
    assert!(lines().is_empty(), "no console");
    console_on(&mut e);
    run(&mut e, 0x149a, 0, &[s("fScale")]);
    run(&mut e, 0x149a, 0, &[s("sName")]);
    let l = lines();
    assert_eq!(l.len(), 2);
    assert_eq!(l[0].0, b"GetNumericGameSetting >> %g");
    assert_eq!(l[0].1[..2], double_words(1.5));
    assert_eq!(l[1].0, b"GetNumericGameSetting >> SETTING NOT FOUND");
}

/// Setting a game setting converts the float as the DLL does: a bool 1 for anything but 0 (NaN
/// too), an integer truncated (`0x80000000` out of range), an unsigned through `__dtol3`'s low
/// word, a float as it is; a string setting is left and answers 0; an unknown name prints NOT
/// FOUND in the console.
#[test]
fn numeric_game_settings_set_as_the_dll_converts() {
    let mut e = setup();
    let data = |e: &Engine, i: u32| e.mem.u32(SETTINGS + 0x40 * i + 4);
    for (name, value, i, want) in [
        ("iCount", 2.7, 0, 2u32),
        ("iCount", -2.7, 0, -2i32 as u32),
        ("iCount", 3e9, 0, 0x8000_0000),
        ("uCount", -1.0, 1, 0xffff_ffff),
        ("uCount", 3e9, 1, 3_000_000_000),
        ("uCount", 5e9, 1, 705_032_704),
        ("uCount", 1e19, 1, 0),
        ("bFlag", 0.5, 2, 1),
        ("bFlag", -0.0, 2, 0),
        ("bFlag", f64::NAN, 2, 1),
        ("fScale", 0.1, 3, 0.1f32.to_bits()),
    ] {
        assert_eq!(
            run(&mut e, 0x149b, 0, &[s(name), z(value)]),
            1.0,
            "{name} {value}"
        );
        assert_eq!(data(&e, i), want, "{name} {value}");
    }
    assert_eq!(run(&mut e, 0x149b, 0, &[s("sName"), z(1.0)]), 0.0);
    assert_eq!(data(&e, 4), TEXTS, "a string setting is left");
    assert_eq!(run(&mut e, 0x149b, 0, &[s("Missing"), z(1.0)]), 0.0);
    assert!(lines().is_empty());
    console_on(&mut e);
    run(&mut e, 0x149b, 0, &[s("Missing"), z(1.0)]);
    run(&mut e, 0x149b, 0, &[s("sName"), z(1.0)]);
    let l = lines();
    assert_eq!(l.len(), 1, "a setting that holds no number prints nothing");
    assert_eq!(l[0].0, b"SetNumericGameSetting >> NOT FOUND");
}

/// INI settings: the preferences' list first, then the INI's, by name without case; empty nodes
/// and nameless settings passed over; an empty name is not found when read; set in memory.
#[test]
fn numeric_ini_settings_prefs_first() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x149c, 0, &[s("isize:display")]), 1.0);
    assert_eq!(
        run(&mut e, 0x149c, 0, &[s("FGAMMA:DISPLAY")]),
        f64::from(2.2f32)
    );
    assert_eq!(run(&mut e, 0x149c, 0, &[s("")]), -1.0);
    assert_eq!(run(&mut e, 0x149c, 0, &[s("Missing")]), -1.0);
    assert_eq!(run(&mut e, 0x149d, 0, &[s("iSize:Display"), z(9.0)]), 1.0);
    assert_eq!(e.mem.u32(SETTINGS + 0x204), 9, "the preferences' one");
    assert_eq!(e.mem.u32(SETTINGS + 0x284), 2);
    assert_eq!(run(&mut e, 0x149d, 0, &[s("Missing"), z(9.0)]), 0.0);
    console_on(&mut e);
    run(&mut e, 0x149c, 0, &[s("iSize:Display")]);
    run(&mut e, 0x149c, 0, &[s("Missing")]);
    run(&mut e, 0x149d, 0, &[s("Missing"), z(9.0)]);
    let l = lines();
    assert_eq!(l[0].0, b"GetNumericIniSetting >> %g");
    assert_eq!(l[0].1[..2], double_words(9.0));
    assert_eq!(l[1].0, b"GetNumericIniSetting >> SETTING NOT FOUND");
    assert_eq!(l[2].0, b"SetNumericIniSetting >> NOT FOUND");
}

/// The crosshair reference and the console's selection: their ids, 0 for none or no manager.
/// `GetCrosshairRef` writes only the result's low word (and prints it with `%08x`);
/// `GetDebugSelection` zeroes the whole double first.
#[test]
fn crosshair_and_selection() {
    let mut e = setup();
    let start = f64::from_bits(0x1234_5678_9abc_def0);
    assert_eq!(
        run_over(&mut e, 0x14a7, SCRIPT, &[], start),
        0x1234_5678_0000_0000
    );
    assert_eq!(run_over(&mut e, 0x14b2, SCRIPT, &[], start), 0);
    e.mem.set_u32(MANAGER + 0xfc, CROSSHAIR);
    e.mem.set_u32(MANAGER + 0xf0, SELECTION);
    assert_eq!(
        run_over(&mut e, 0x14a7, SCRIPT, &[], start),
        0x1234_5678_0003_1234
    );
    assert_eq!(run_over(&mut e, 0x14b2, SCRIPT, &[], start), 0x0004_5678);
    e.set_global(INTERFACE_MANAGER, 0u32);
    assert_eq!(
        run_over(&mut e, 0x14a7, SCRIPT, &[], start),
        0x1234_5678_0000_0000
    );
    e.set_global(INTERFACE_MANAGER, MANAGER);
    console_on(&mut e);
    run(&mut e, 0x14a7, 0, &[]);
    let l = lines();
    assert_eq!(l[0].0, b"GetCrosshairRef >> %08x");
    assert_eq!(l[0].1[0], 0x0003_1234);
}

/// `GetGameLoaded` answers 1 once per script (by id) until a game is loaded or started again;
/// `GetGameRestarted` once per script while the game runs. Without a script both answer 0.
#[test]
fn loaded_and_restarted_once_per_script() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x14ac, 0, &[]), 1.0);
    assert_eq!(run(&mut e, 0x14ac, 0, &[]), 0.0);
    assert_eq!(run(&mut e, 0x14a8, 0, &[]), 1.0);
    assert_eq!(run(&mut e, 0x14a8, 0, &[]), 0.0);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d01);
    assert_eq!(run(&mut e, 0x14ac, 0, &[]), 1.0, "another script");
    game_loaded(&mut e);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    assert_eq!(run(&mut e, 0x14ac, 0, &[]), 1.0, "after a load");
    assert_eq!(run(&mut e, 0x14a8, 0, &[]), 0.0, "a load is no restart");
    assert_eq!(run_over(&mut e, 0x14ac, 0, &[], 5.0), 0);
    assert_eq!(run_over(&mut e, 0x14a8, 0, &[], 5.0), 0);
    console_on(&mut e);
    game_loaded(&mut e);
    run(&mut e, 0x14ac, 0, &[]);
    run_over(&mut e, 0x14ac, 0, &[], 0.0);
    let l = lines();
    assert_eq!(l.len(), 1, "no script, no line");
    assert_eq!(l[0].0, b"GetGameLoaded >> %.0f");
    assert_eq!(l[0].1[..2], double_words(1.0));
}

/// Files by name without case: loaded or not, the index (the empty node counts; 255 for none),
/// the count of the list's files as a byte.
#[test]
fn files_by_name() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x14ae, 0, &[s("mod.ESP")]), 1.0);
    assert_eq!(run(&mut e, 0x14ae, 0, &[s("Missing.esp")]), 0.0);
    assert_eq!(run(&mut e, 0x14af, 0, &[s("mod.esp")]), 2.0);
    assert_eq!(run(&mut e, 0x14af, 0, &[s("falloutnv.esm")]), 0.0);
    assert_eq!(run(&mut e, 0x14af, 0, &[s("Missing.esp")]), 255.0);
    assert_eq!(run(&mut e, 0x14b0, 0, &[]), 3.0);
    console_on(&mut e);
    run(&mut e, 0x14ae, 0, &[s("Other.esp")]);
    run(&mut e, 0x14ae, 0, &[s("Missing.esp")]);
    run(&mut e, 0x14af, 0, &[s("Other.esp")]);
    run(&mut e, 0x14b0, 0, &[]);
    let l = lines();
    assert_eq!(l[0].0, b"Mod Loaded");
    assert_eq!(l[1].0, b"Mod not loaded");
    assert_eq!((l[2].0.as_slice(), l[2].1[0]), (&b"Mod Index: %02X"[..], 3));
    assert_eq!(l[3].0, b"Mods Loaded: %.0f");
    assert_eq!(l[3].1[..2], double_words(3.0));
    // 256 files count as 0 (`UInt8`).
    let node = FILES + 0x1000;
    let mut items = vec![FILES; 256];
    items[255] = FILES + 0x100;
    tlist(&mut e, HANDLER + 0x210, &items, node);
    assert_eq!(run(&mut e, 0x14b0, 0, &[]), 0.0);
}

/// Form ids: the mod index and the local index of the form given, else of the calling
/// reference, -1 with neither; `BuildRef` puts the low byte and the low 24 bits together in the
/// low word over a zeroed double.
#[test]
fn form_ids() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x14b1, 0, &[r(1)]), 10.0);
    assert_eq!(run(&mut e, 0x14b1, THIS, &[]), 255.0);
    assert_eq!(run(&mut e, 0x14b1, 0, &[]), -1.0);
    assert_eq!(run(&mut e, 0x14d8, 0, &[r(1)]), f64::from(0x1234));
    assert_eq!(run(&mut e, 0x14d8, THIS, &[]), f64::from(0x0080_0001));
    assert_eq!(run(&mut e, 0x14d8, 0, &[]), -1.0);
    let start = f64::from_bits(0x1234_5678_9abc_def0);
    assert_eq!(
        run_over(&mut e, 0x14d9, SCRIPT, &[n(0x1a), n(0x1234_5678)], start),
        0x1a34_5678
    );
    assert_eq!(
        run_over(&mut e, 0x14d9, SCRIPT, &[n(0x1ff), n(5)], start),
        0xff00_0005
    );
}

/// `MessageEx` queues the formatted text for two seconds (`QueueUIMessage(text, 0, 0, 0, 2.0,
/// 0)`) and answers 1.
#[test]
fn message_ex_queues_the_text() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x14db, 0, &[s("Hello %g"), z(3.0)]), 1.0);
    let m = seen(|s| std::mem::take(&mut s.messages));
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].0, b"Hello 3");
    assert_eq!(m[0].1, [0, 0, 0, 2.0f32.to_bits(), 0]);
}

/// `MessageBoxEx`: the text cut at `|` into the message and up to ten buttons ("Ok" for none),
/// `@` in a console line of no mod; the box is the script's unless a calling reference that is
/// not temporary owns it; the answer reset to 0xFF; the game's own callback. The result stays 0.
#[test]
fn message_box_ex_buttons_and_owner() {
    let mut e = setup();
    e.set_global(MESSAGE_BOX_ANSWER, 3u8);
    assert_eq!(run(&mut e, 0x14dc, 0, &[s("Pick|Yes|No")]), 0.0);
    let b = seen(|s| std::mem::take(&mut s.boxes));
    let w = &b[0].words;
    assert_eq!(w.len(), 19);
    assert_eq!(b[0].message, "Pick");
    assert_eq!(w[1..8], [0, 0, MESSAGE_BOX_CALLBACK, 0, 0x17, 0, 0]);
    assert_eq!(b[0].buttons, ["Yes", "No"]);
    assert!(w[10..].iter().all(|&x| x == 0));
    assert_eq!(e.global::<u32>(MESSAGE_BOX_OWNER), 0x0500_0d00);
    assert_eq!(e.global::<u8>(MESSAGE_BOX_ANSWER), 0xff);
    run(&mut e, 0x14dc, 0, &[s("t|1|2|3|4|5|6|7|8|9|10|11")]);
    run(&mut e, 0x14dc, 0, &[s("Just this")]);
    e.mem.set_u32(THIS + 8, 0);
    run(&mut e, 0x14dc, THIS, &[s("Mine")]);
    let owner = e.global::<u32>(MESSAGE_BOX_OWNER);
    e.mem.set_u32(THIS + 8, 0x4000);
    run(&mut e, 0x14dc, THIS, &[s("Temporary")]);
    let b = seen(|s| std::mem::take(&mut s.boxes));
    assert_eq!(b[0].message, "t");
    assert_eq!(
        b[0].buttons,
        ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10|11"],
        "the tenth button keeps the rest"
    );
    assert_eq!(b[1].buttons, ["Ok"]);
    assert_eq!(owner, 0xff80_0001, "a reference's own box");
    assert_eq!(
        e.global::<u32>(MESSAGE_BOX_OWNER),
        0x0500_0d00,
        "a temporary one's is the script's"
    );
    // In the console, a script of no mod (index 0xFF) cuts at `@`.
    console_on(&mut e);
    e.mem.set_u32(SCRIPT + 0xc, 0xff00_0d00);
    run(&mut e, 0x14dc, 0, &[s("Pick@Yes|No")]);
    e.mem.set_u32(SCRIPT + 0xc, 0x0500_0d00);
    run(&mut e, 0x14dc, 0, &[s("Pick@Yes|No")]);
    let b = seen(|s| std::mem::take(&mut s.boxes));
    assert_eq!(
        (b[0].message.as_str(), b[0].buttons.as_slice()),
        ("Pick", &["Yes|No".to_string()][..])
    );
    assert_eq!(
        (b[1].message.as_str(), b[1].buttons.as_slice()),
        ("Pick@Yes", &["No".to_string()][..])
    );
}

/// The grid count unsigned; the local map override for an odd count from 1 to `uGridsToLoad`
/// (compared unsigned), its square kept as a signed byte (13 keeps -87).
#[test]
fn grids_and_their_override() {
    let mut e = setup();
    e.set_global(GRIDS_TO_LOAD, 0xffff_ffffu32);
    assert_eq!(run(&mut e, 0x15e6, 0, &[]), 4_294_967_295.0);
    e.set_global(GRIDS_TO_LOAD, 5u32);
    let grids = |e: &mut Engine| e.ext::<Nvse>().game.grids_override;
    assert_eq!(grids(&mut e), -1, "none at first");
    for (count, want) in [(4, 0.0), (7, 0.0), (0, 0.0), (-1, 0.0)] {
        assert_eq!(run(&mut e, 0x15e8, 0, &[n(count)]), want, "{count}");
    }
    assert_eq!(grids(&mut e), -1);
    assert_eq!(run(&mut e, 0x15e8, 0, &[n(3)]), 1.0);
    assert_eq!(grids(&mut e), 9);
    e.set_global(GRIDS_TO_LOAD, 13u32);
    assert_eq!(run(&mut e, 0x15e8, 0, &[n(13)]), 1.0);
    assert_eq!(grids(&mut e), -87);
}

/// `OutputLocalMapPicturesOverride`: without a kept square the game's own command with the same
/// arguments; with one, the busy flag set, the map menu reset, one picture per square cell, the
/// flag cleared. A square of 128 or more counts sign-extended (about four billion pictures, as in
/// xNVSE: not run here); with the list and scan guard row it counts unsigned.
#[test]
fn local_map_pictures_with_the_override() {
    thread_local! {
        static PICTURES: RefCell<(u32, u32, Vec<u32>)> = const { RefCell::new((0, 0, Vec::new())) };
    }
    let mut e = setup();
    e.map(0x0120_0000, 0x1000);
    e.register(0x005d_4580, |_, a| {
        PICTURES.with(|p| p.borrow_mut().2 = a[..8].to_vec());
        Ret::default()
    });
    e.register(0x005d_45c0, |e, _| {
        assert_eq!(e.mem.u8(0x0120_04fc), 1, "busy while it runs");
        PICTURES.with(|p| p.borrow_mut().0 += 1);
        Ret::default()
    });
    e.register(0x005d_45e0, |_, _| {
        let n = PICTURES.with(|p| {
            let mut p = p.borrow_mut();
            p.1 += 1;
            p.1
        });
        // Stands for the four billion pictures of a sign-extended count.
        assert!(n <= 1000, "past a thousand pictures");
        Ret::default()
    });
    let take = || PICTURES.with(|p| std::mem::take(&mut *p.borrow_mut()));
    run(&mut e, 0x15e7, 0, &[]);
    let (resets, pictures, args) = take();
    assert_eq!((resets, pictures), (0, 0));
    assert_eq!(args.len(), 8, "the game's command ran with the arguments");
    e.set_global(GRIDS_TO_LOAD, 13u32);
    run(&mut e, 0x15e8, 0, &[n(7)]);
    run(&mut e, 0x15e7, 0, &[]);
    assert_eq!(take(), (1, 49, Vec::new()));
    assert_eq!(e.mem.u8(0x0120_04fc), 0);
    run(&mut e, 0x15e8, 0, &[n(13)]);
    // Vanilla: 169 read as -87, sign-extended: the loop runs on past any square.
    let endless = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&mut e, 0x15e7, 0, &[]);
    }));
    assert!(endless.is_err());
    assert_eq!(take().1, 1001);
    e.mem.set_u8(0x0120_04fc, 0);
    super::super::array::set_fix(&mut e, super::super::array::LIST_GUARD, true);
    run(&mut e, 0x15e7, 0, &[]);
    assert_eq!(take(), (1, 169, Vec::new()));
}

/// `AddSpellNS` runs the game's AddSpell with its own arguments while `QueueUIMessage` returns at
/// once, and answers what AddSpell answers; the messages come back after it.
#[test]
fn add_spell_ns_without_its_message() {
    let mut e = setup();
    install(&mut e);
    assert_eq!(run(&mut e, 0x15ef, 0, &[r(1)]), 1.0, "AddSpell's result");
    assert_eq!(seen(|s| (s.spells, s.messages.len())), (1, 0));
    let text = crate::table::guest_str(&mut e, b"later");
    e.call(
        QUEUE_UI_MESSAGE,
        &args![text, 0u32, 0u32, 0u32, 2.0f32, 0u32],
    );
    assert_eq!(seen(|s| s.messages.len()), 1, "on again after AddSpellNS");
    assert_eq!(e.wraps_at(QUEUE_UI_MESSAGE), [Owner::Nvse]);
}

/// A name of 512 bytes or more runs past xNVSE's buffer: without the script list and scan guard
/// row the port stops the game; with it the name is read whole. 511 bytes and the terminator fit.
#[test]
fn a_long_name() {
    let long = "x".repeat(512);
    let mut e = setup();
    for (opcode, floats) in [
        (0x149a, 0),
        (0x149b, 1),
        (0x149c, 0),
        (0x149d, 1),
        (0x14ae, 0),
        (0x14af, 0),
    ] {
        let mut args = vec![s(&long)];
        args.extend((0..floats).map(|_| z(1.0)));
        let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            call(&mut e, opcode, 0, &args)
        }));
        assert!(stop.is_err(), "{opcode:04x}");
    }
    assert_eq!(
        run(&mut e, 0x149a, 0, &[s(&long[1..])]),
        -1.0,
        "511 bytes fit"
    );
    let mut e = setup();
    super::super::array::set_fix(&mut e, super::super::array::LIST_GUARD, true);
    let at = SETTINGS + 0x600;
    let name = format!("f{}", "y".repeat(600));
    setting(&mut e, at, &name, 4.0f32.to_bits());
    KNOWN_GMSTS.with(|g| g.borrow_mut().push((name.clone().into_bytes(), at)));
    assert_eq!(run(&mut e, 0x149a, 0, &[s(&name)]), 4.0);
    assert_eq!(run(&mut e, 0x14ae, 0, &[s(&long)]), 0.0);
}
