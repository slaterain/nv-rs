//! The game patches' framework (the word, the row, the settings) and the patches of step 3.9a.

use super::settings::{section, switches, Switches, DEFAULTS};
use super::*;
use crate::test_support::engine;
use engine_core::extend::{jip_patches, Owner};
use std::cell::Cell;

/// KERNEL32's import slots the file read goes through (`nvse::saveload::read_file`).
const CREATE_FILE_A: u32 = 0x00fd_f088;
const READ_FILE: u32 = 0x00fd_f080;
const CLOSE_HANDLE: u32 = 0x00fd_f07c;
/// `TESTopic::GetOwnerTopic`.
const OWNER_TOPIC: u32 = 0x0061_a310;
/// A result word and two infos and topics.
const RESULT: u32 = 0x0300_8000;
const INFO: u32 = 0x0300_9000;
const OTHER_INFO: u32 = 0x0300_9100;
const TOPIC: u32 = 0x0300_a000;
const SECOND_TOPIC: u32 = 0x0300_a100;

thread_local! {
    /// The file the stand-in of `CreateFileA` serves (none: it answers no file), and the
    /// stand-in base at `GetOwnerTopic`'s calls.
    static FILE: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
    static AT: Cell<usize> = const { Cell::new(0) };
    static BASE_CALLS: Cell<u32> = const { Cell::new(0) };
}

fn ret(eax: u32) -> Ret {
    Ret {
        eax,
        ..Ret::default()
    }
}

/// An engine with xNVSE's table, the file calls served from [`FILE`] and a stand-in base at
/// `GetOwnerTopic` (the translation, or the topic index row's function), counting its calls.
fn base() -> Engine {
    FILE.with(|f| f.borrow_mut().take());
    BASE_CALLS.with(|c| c.set(0));
    let mut e = engine();
    e.register(CREATE_FILE_A, |e, w| {
        assert_eq!(e.mem.cstr(w[0]), settings::PATH, "JIP's relative path");
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
            let at = AT.with(Cell::get).min(file.len());
            let n = size.min(file.len() - at);
            AT.with(|a| a.set(at + n));
            file[at..at + n].to_vec()
        });
        e.mem.write(buffer, &chunk);
        e.mem.set_u32(read, chunk.len() as u32);
        ret(1)
    });
    e.register(CLOSE_HANDLE, |_, _| ret(1));
    e.register(OWNER_TOPIC, |_, _| {
        BASE_CALLS.with(|c| c.set(c.get() + 1));
        ret(TOPIC)
    });
    // xNVSE's script context, with its result zeroing at the three handler seams.
    for addr in [0x005e_0d20u32, 0x005e_1160, 0x005e_13a0, 0x005e_1550] {
        e.register(addr, |_, _| Ret::default());
    }
    nvse::context::install(&mut e);
    e
}

fn defaults() -> u32 {
    DEFAULTS
        .iter()
        .fold(0, |bits, &n| bits | jip_patches::optional(n))
}

/// Every area's fix rows have (or have not) JIP's handler; its patches of JIP's own are left
/// alone.
fn fixes_in(e: &Engine, expected: bool) {
    for area in AREAS {
        for &addr in area.fix_sites {
            assert_eq!(
                e.site_owners(addr).contains(&Owner::Jip),
                expected,
                "{} {addr:08x}",
                area.name
            );
        }
        for &addr in area.fix_wraps {
            assert_eq!(
                e.wraps_at(addr).contains(&Owner::Jip),
                expected,
                "{} {addr:08x}",
                area.name
            );
        }
    }
}

fn jip_in(e: &Engine, expected: bool) {
    for area in AREAS {
        for &addr in area.jip_sites {
            assert_eq!(e.site_owners(addr).contains(&Owner::Jip), expected);
        }
        for &addr in area.jip_wraps {
            assert_eq!(e.wraps_at(addr).contains(&Owner::Jip), expected);
        }
    }
}

/// Rows and handlers come together, in the row's home group: every site and wrap an area lists has a
/// row with owner `Jip`, and every row of the game patches' tables is some area's (a seam another
/// group adds before the row exists has no row: it is tested with `Engine::on_site_for_test`).
#[test]
fn every_area_row_has_an_area() {
    let mine: Vec<u32> = AREAS
        .iter()
        .flat_map(|a| a.fix_sites.iter().chain(a.jip_sites))
        .copied()
        .collect();
    let wraps: Vec<u32> = AREAS
        .iter()
        .flat_map(|a| a.fix_wraps.iter().chain(a.jip_wraps))
        .copied()
        .collect();
    for table in engine_core::extend::PATCH_SITES {
        for row in *table {
            assert!(mine.contains(&row.addr), "{:08x}", row.addr);
        }
    }
    for table in engine_core::extend::PATCH_WRAPS {
        for row in *table {
            assert!(wraps.contains(&row.addr), "{:08x}", row.addr);
        }
    }
    for addr in mine {
        assert!(
            engine_core::extend::all_sites()
                .any(|r| r.addr == addr && r.owners.contains(&Owner::Jip)),
            "{addr:08x}"
        );
    }
}

/// While JIP answers "not installed" (today): the word stays 0 and nothing is put in, with the
/// row on or off, before or after JIP.
#[test]
fn not_installed_nothing_is_put_in() {
    let mut e = base();
    set_row(&mut e, true);
    crate::install_as(&mut e, false);
    set_row(&mut e, true);
    assert_eq!(e.jip_patches(), 0);
    fixes_in(&e, false);
    jip_in(&e, false);
    assert!(e.wraps_at(OWNER_TOPIC).is_empty());
    assert_eq!(e.ext::<Jip>().game_patches.settings, Switches::default());
    // The game's own answers stand.
    assert_eq!(e.call(OWNER_TOPIC, &[INFO]).u32(), TOPIC);
    e.mem.set_f64(RESULT, 5.0);
    e.site(0x005e_2329, &[0x1000, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 5.0);
}

/// Installed with the row off (the vanilla preset): only the `INSTALLED` bit, no fix in, no file
/// read.
#[test]
fn installed_with_the_row_off_puts_no_fix_in() {
    let mut e = base();
    crate::install_as(&mut e, true);
    set_row(&mut e, false);
    assert_eq!(e.jip_patches(), jip_patches::INSTALLED);
    fixes_in(&e, false);
    jip_in(&e, true);
    assert_eq!(e.call(OWNER_TOPIC, &[INFO]).u32(), TOPIC);
    assert_eq!(BASE_CALLS.with(Cell::get), 1);
}

/// Installed with the row on (the fixes preset), with no `jip_nvse.ini`: JIP's defaults, every
/// fix in, once (the row set again or first changes nothing).
#[test]
fn installed_with_the_row_on_puts_every_fix_in_once() {
    let mut e = base();
    crate::install_as(&mut e, true);
    set_row(&mut e, true);
    assert_eq!(
        e.jip_patches(),
        jip_patches::INSTALLED | jip_patches::FIXES | defaults()
    );
    fixes_in(&e, true);
    set_row(&mut e, true);
    assert_eq!(e.wraps_at(OWNER_TOPIC), [Owner::Jip]);
    // The row before JIP: put in as JIP installs.
    let mut e = base();
    set_row(&mut e, true);
    crate::install_as(&mut e, true);
    assert_ne!(e.jip_patches() & jip_patches::FIXES, 0);
    fixes_in(&e, true);
    assert!(e.unlisted_wraps().is_empty());
}

/// The player's file decides the switches; the published file gives JIP's defaults and the three
/// chances.
#[test]
fn the_players_file_decides_the_switches() {
    let mut e = base();
    FILE.with(|f| *f.borrow_mut() = Some(PUBLISHED.to_vec()));
    crate::install_as(&mut e, true);
    set_row(&mut e, true);
    let read = e.ext::<Jip>().game_patches.settings;
    assert_eq!(read.on, defaults());
    assert_eq!(read.weapon_mod_chance, [2, 10, 60]);
    assert!(!read.npc_perks_auto_add);
    let mut e = base();
    FILE.with(|f| {
        *f.borrow_mut() =
            Some(b"[GamePatches]\r\nbIgnoreDTDRFix=0\r\nbCreatureSpreadFix=1\r\n".to_vec())
    });
    crate::install_as(&mut e, true);
    set_row(&mut e, true);
    assert_eq!(
        e.jip_patches(),
        jip_patches::INSTALLED | jip_patches::FIXES | jip_patches::CREATURE_SPREAD_FIX
    );
}

/// The published "JIP LN Settings INI" 56.24's key lines (its notes left out).
const PUBLISHED: &[u8] = b"[GamePatches]\r\nbIgnoreDTDRFix=1\r\n;\tFixes the Ignore DT/DR effect\r\n\r\n\
bEnableFO3Repair=0\r\nbEnableBigGunsSkill=0\r\nbProjImpactDmgFix=1\r\nbGameDaysPassedFix=1\r\n\
bHardcoreNeedsFix=1\r\nbNoFailedScriptLocks=1\r\nbDoublePrecision=1\r\nbQttSelectShortKeys=1\r\n\
bLocalizedDTDR=0\r\nbVoiceModulationFix=1\r\nbSneakBoundingBoxFix=1\r\nbEnableNVACAlerts=0\r\n\
bLoadScreenFix=0\r\nbNPCWeaponMods=0\r\nuWMChancePerLevel=2\r\nuWMChanceMin=10\r\nuWMChanceMax=60\r\n\
uNPCPerks=0\r\nbCreatureSpreadFix=0\r\n";

fn lines(text: &[&str]) -> Vec<Vec<u8>> {
    text.iter().map(|l| l.as_bytes().to_vec()).collect()
}

/// `GetPrivateProfileSectionA`'s answers, as measured on scratch files (2026-11-14).
#[test]
fn the_section_is_read_as_windows_reads_it() {
    let file = b"; top comment\r\n[gamepatches]\r\nbIgnoreDTDRFix=1\r\n;\tcomment line\r\n\r\n  \
bEnableFO3Repair = 1  \r\nbEnableBigGunsSkill=\r\nbProjImpactDmgFix\r\nuNPCPerks=\"2\"\r\n\
\tbLoadScreenFix\t=\t0\r\nbIgnoreDTDRFix=0\r\nnoequals line here\r\n=7\r\n[Other]\r\nx=1\r\n\
[GamePatches]\r\nbCreatureSpreadFix=1\r\n";
    assert_eq!(
        section(file, b"GamePatches"),
        lines(&[
            "bIgnoreDTDRFix=1",
            "bEnableFO3Repair=1",
            "bEnableBigGunsSkill=",
            "bProjImpactDmgFix",
            "uNPCPerks=\"2\"",
            "bLoadScreenFix=0",
            "bIgnoreDTDRFix=0",
            "noequals line here",
            "=7",
        ])
    );
    let file = b"[GamePatches]\n  ;x=1\n#y=1\nb X = 1\n;\nlast=2";
    assert_eq!(
        section(file, b"GamePatches"),
        lines(&["#y=1", "b X=1", "last=2"])
    );
    // An empty section, one of comments only, a missing section or file: nothing.
    assert!(section(b"[GamePatches]\r\n[Next]\r\na=1\r\n", b"GamePatches").is_empty());
    assert!(section(b"[GamePatches]\r\n; only a comment\r\n\r\n", b"GamePatches").is_empty());
    assert!(section(b"[Other]\r\na=1\r\n", b"GamePatches").is_empty());
    assert!(section(b"", b"GamePatches").is_empty());
    // A UTF-8 byte order mark spoils the heading; UTF-16 is read.
    assert!(section(
        b"\xef\xbb\xbf[GamePatches]\r\nbIgnoreDTDRFix=0\r\n",
        b"GamePatches"
    )
    .is_empty());
    let utf16: Vec<u8> = [0xfeffu16]
        .into_iter()
        .chain("[GamePatches]\r\nb=0\r\n".encode_utf16())
        .flat_map(u16::to_le_bytes)
        .collect();
    assert_eq!(section(&utf16, b"GamePatches"), lines(&["b=0"]));
}

/// JIP's loop over the lines: defaults only with no line; a switch on unless its value starts with
/// `0`; keys without case; the chances' numbers; `uNPCPerks=2`.
#[test]
fn jip_reads_each_line_as_its_loop_does() {
    assert_eq!(switches(&[]).on, defaults());
    assert_eq!(
        switches(&[]).weapon_mod_chance,
        [0, 0, 0],
        "JIP's code: 0 without the file"
    );
    // Lines, all off: nothing on (not the defaults).
    assert_eq!(switches(&lines(&["bIgnoreDTDRFix=0"])).on, 0);
    let read = switches(&lines(&[
        "BIGNOREDTDRFIX=1",
        "bEnableFO3Repair=",
        "bEnableBigGunsSkill",
        "bProjImpactDmgFix=01",
        "bGameDaysPassedFix==1",
        "bHardcoreNeedsFix=\"0\"",
        "bUnknown=1",
        "uWMChancePerLevel=-5",
        "uWMChanceMin=12x",
        "uNPCPerks=2",
        "bLocalizedDTDR=1",
        "bLocalizedDTDR=0",
    ]));
    assert_eq!(
        read.on,
        jip_patches::IGNORE_DTDR_FIX
            | jip_patches::ENABLE_FO3_REPAIR
            | jip_patches::ENABLE_BIG_GUNS_SKILL
            | jip_patches::GAME_DAYS_PASSED_FIX
            | jip_patches::HARDCORE_NEEDS_FIX
            | jip_patches::NPC_PERKS
            | jip_patches::LOCALIZED_DTDR
    );
    assert_eq!(read.weapon_mod_chance, [(-5i32) as u32, 12, 0]);
    assert!(read.npc_perks_auto_add);
    let read = switches(&lines(&["uNPCPerks=1"]));
    assert_eq!(read.on, jip_patches::NPC_PERKS);
    assert!(!read.npc_perks_auto_add);
    let read = switches(&lines(&["uNPCPerks=0", "uWMChanceMax=60"]));
    assert_eq!((read.on, read.weapon_mod_chance), (0, [0, 0, 60]));
}

/// The fixed engine of the patches of 3.9a.
fn fixed() -> Engine {
    let mut e = base();
    crate::install_as(&mut e, true);
    set_row(&mut e, true);
    e
}

/// "Store parent TESTopic": each link records the topic (the last one wins), `GetOwnerTopic`
/// answers it without calling what is under it (the translation or the topic index), `ClearData`'s
/// seam answers it, the constructor and the destructor drop it, an info never linked answers 0.
#[test]
fn an_info_answers_the_last_topic_its_load_linked() {
    let mut e = fixed();
    assert_eq!(
        e.site(0x0061_e086, &[TOPIC, INFO]),
        None,
        "the call goes on"
    );
    assert_eq!(e.site(0x0061_e086, &[SECOND_TOPIC, INFO]), None);
    assert_eq!(e.call(OWNER_TOPIC, &[INFO]).u32(), SECOND_TOPIC);
    assert_eq!(e.call(OWNER_TOPIC, &[OTHER_INFO]).u32(), 0);
    assert_eq!(BASE_CALLS.with(Cell::get), 0, "the walk never runs");
    assert_eq!(e.site(0x0061_d041, &[INFO]), Some(SECOND_TOPIC));
    assert_eq!(e.site(0x0061_d041, &[OTHER_INFO]), Some(0));
    assert_eq!(dialogue::parent_topic(&mut e, INFO), SECOND_TOPIC);
    // A new info at the same address starts with none.
    assert_eq!(e.site(0x0061_cd16, &[INFO]), None);
    assert_eq!(e.call(OWNER_TOPIC, &[INFO]).u32(), 0);
    // The destructor drops it; another info's record stays.
    e.site(0x0061_e086, &[TOPIC, INFO]);
    e.site(0x0061_e086, &[TOPIC, OTHER_INFO]);
    assert_eq!(e.site(0x0061_ceee, &[INFO]), None);
    assert_eq!(e.call(OWNER_TOPIC, &[INFO]).u32(), 0);
    assert_eq!(e.call(OWNER_TOPIC, &[OTHER_INFO]).u32(), TOPIC);
    assert_eq!(e.site(0x0061_d041, &[INFO]), Some(0));
}

/// The record is lock-free of JIP's state and shared: a thread's engine reads what main wrote,
/// while another engine holds JIP's slot.
#[test]
fn the_record_is_read_without_jips_slot() {
    let mut e = fixed();
    e.site(0x0061_e086, &[TOPIC, INFO]);
    let mut other = Engine::new_untyped_for_test();
    e.share_slots_for_test(&mut other);
    engine_core::extend::release_held_for_test();
    assert!(other.ext::<Jip>().installed, "the other engine holds it");
    assert_eq!(e.call(OWNER_TOPIC, &[INFO]).u32(), TOPIC);
    assert_eq!(e.site(0x0061_d041, &[INFO]), Some(TOPIC));
    assert_eq!(
        dialogue::parent_topic(&mut other, INFO),
        TOPIC,
        "one record for the run"
    );
    engine_core::extend::release_held_for_test();
}

/// Result zeroing: JIP's handler after xNVSE's at the three seams writes 0.0 for every opcode;
/// without the row only xNVSE's extender opcodes are zeroed (and not at the block seam).
#[test]
fn every_handler_result_starts_at_zero() {
    let seams = [0x005a_cb9d, 0x005e_16c3, 0x005e_2329];
    let mut e = fixed();
    for addr in seams {
        assert_eq!(e.site_owners(addr), [Owner::Nvse, Owner::Jip], "{addr:08x}");
        for opcode in [0x1000, 0x10f0, 0x1400, 0x2600] {
            e.mem.set_f64(RESULT, 5.0);
            assert_eq!(e.site(addr, &[opcode, 0, RESULT]), None, "the call goes on");
            assert_eq!(e.mem.f64(RESULT).to_bits(), 0, "{addr:08x} {opcode:04x}");
        }
        // No result word: nothing written.
        assert_eq!(e.site(addr, &[0x1000, 0, 0]), None);
    }
    let mut e = base();
    crate::install_as(&mut e, true);
    set_row(&mut e, false);
    for addr in seams {
        assert_eq!(e.site_owners(addr), [Owner::Nvse]);
        e.mem.set_f64(RESULT, 5.0);
        e.site(addr, &[0x1000, 0, RESULT]);
        assert_eq!(e.mem.f64(RESULT), 5.0, "vanilla opcode {addr:08x}");
    }
    e.mem.set_f64(RESULT, 5.0);
    e.site(0x005e_2329, &[0x1400, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 0.0, "xNVSE zeroes its own");
}

/// A slot write (3.5a's way): the routine is registered at the group's own address and written into
/// the guest slot, the word it held is answered; each group's 64 addresses are its own, past JIP's
/// other routines.
#[test]
fn a_slot_gets_a_routine_of_its_group() {
    use super::slots::{routine, write_slot, Group, PER_GROUP};
    let mut e = engine();
    const SLOT: u32 = 0x0300_7000;
    e.mem.set_u32(SLOT, 0x0040_1290);
    let at = routine(Group::GF, 3);
    assert_eq!(write_slot(&mut e, SLOT, at, |_, _| ret(0x77)), 0x0040_1290);
    assert_eq!(e.mem.u32(SLOT), at);
    assert_eq!(e.call(at, &[]).u32(), 0x77);
    assert_eq!(routine(Group::GF, 0) + 4 * PER_GROUP, routine(Group::GS, 0));
    assert_eq!(
        routine(Group::GA, 0),
        routine(Group::A39, PER_GROUP - 1) + 4
    );
    assert!(routine(Group::A39, 0) > crate::actor_hooks::CLEAR_HIT_DATA_ROUTINE + 4);
    assert!(std::panic::catch_unwind(|| routine(Group::GX, PER_GROUP)).is_err());
    // A routine outside the block, and a second patch on the same slot, are refused.
    let mut e = engine();
    e.mem.set_u32(SLOT, 0x0040_1290);
    let outside = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        write_slot(&mut e, SLOT, 0x0040_1000, |_, _| ret(0))
    }));
    assert!(outside.is_err());
    write_slot(&mut e, SLOT, routine(Group::GA, 0), |_, _| ret(0));
    let twice = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        write_slot(&mut e, SLOT, routine(Group::GB, 0), |_, _| ret(0))
    }));
    assert!(twice.is_err());
}

/// JIP's walk goes on at the token after a key: a value that names a switch turns it on (its own
/// value is the empty rest), and an extra `=` is an empty token looked up too.
#[test]
fn jip_walks_every_token() {
    assert_eq!(
        switches(&lines(&["x=bIgnoreDTDRFix"])).on,
        jip_patches::IGNORE_DTDR_FIX
    );
    assert_eq!(
        switches(&lines(&["bLoadScreenFix=0=bCreatureSpreadFix"])).on,
        jip_patches::CREATURE_SPREAD_FIX
    );
    assert_eq!(switches(&lines(&["bLoadScreenFix==0"])).on, 0);
    let read = switches(&lines(&["uWMChanceMin=uWMChanceMax=7"]));
    assert_eq!(read.weapon_mod_chance, [0, 0, 7]);
}

/// The section read stops at 65,534 bytes (the lines with their NULs, measured): a switch past it
/// is not read.
#[test]
fn the_section_read_stops_at_the_buffer() {
    let mut text: Vec<String> = (0..4369).map(|i| format!("key{i:06}=1234")).collect();
    text.push("bCreatureSpreadFix=1".to_string());
    let long: Vec<Vec<u8>> = text.iter().map(|l| l.as_bytes().to_vec()).collect();
    assert_eq!(super::settings::section_buffer(&long).len(), 0xfffe);
    assert_eq!(switches(&long).on, 0);
    let mut short = long[..4000].to_vec();
    short.push(b"bCreatureSpreadFix=1".to_vec());
    assert_eq!(switches(&short).on, jip_patches::CREATURE_SPREAD_FIX);
}

/// The heading forms and line ends measured (2026-11-14), and the one form not measured (UTF-16
/// big-endian: read as bytes, no section).
#[test]
fn headings_and_line_ends_as_measured() {
    for file in [
        &b"[GamePatches] trailing\r\na=1\r\n"[..],
        b"[ GamePatches ]\r\na=1\r\n",
        b"[GamePatches\r\na=1\r\n",
        b"  [GamePatches]x=9\r\na=1\r\n",
    ] {
        assert_eq!(section(file, b"GamePatches"), lines(&["a=1"]), "{file:?}");
    }
    assert_eq!(
        section(b"[GamePatches]\ra=1\rb=2\r", b"GamePatches"),
        lines(&["a=1", "b=2"])
    );
    let utf16be: Vec<u8> = [0xfeffu16]
        .into_iter()
        .chain("[GamePatches]\r\nb=0\r\n".encode_utf16())
        .flat_map(u16::to_be_bytes)
        .collect();
    assert!(section(&utf16be, b"GamePatches").is_empty());
}

/// Every owner at every seam and wrap of the tables, and every wrapped address's owners.
fn snapshot(e: &Engine) -> std::collections::BTreeSet<(u32, &'static str, String)> {
    let mut out = std::collections::BTreeSet::new();
    for row in engine_core::extend::all_sites() {
        for o in e.site_owners(row.addr) {
            out.insert((row.addr, "site", format!("{o:?}")));
        }
    }
    for row in engine_core::extend::all_wraps() {
        for o in e.wraps_at(row.addr) {
            out.insert((row.addr, "wrap", format!("{o:?}")));
        }
    }
    for (addr, owners) in e.wrapped() {
        for o in owners {
            out.insert((addr, "wrapped", format!("{o:?}")));
        }
    }
    out
}

fn expected(
    sites: &[u32],
    wraps: &[u32],
) -> std::collections::BTreeSet<(u32, &'static str, String)> {
    let jip = format!("{:?}", Owner::Jip);
    let mut out = std::collections::BTreeSet::new();
    for &a in sites {
        out.insert((a, "site", jip.clone()));
    }
    for &a in wraps {
        out.insert((a, "wrap", jip.clone()));
        out.insert((a, "wrapped", jip.clone()));
    }
    out
}

/// What JIP's install and the row add, owner by owner over every table address: at the game
/// patches' addresses JIP's install adds exactly the areas' `jip_*` lists; the row off adds
/// nothing; the row on adds exactly the `fix_*` lists, the same whether the host sets it before or
/// after JIP installs.
#[test]
fn the_row_adds_exactly_the_fix_lists() {
    let jip_sites: Vec<u32> = AREAS.iter().flat_map(|a| a.jip_sites).copied().collect();
    let jip_wraps: Vec<u32> = AREAS.iter().flat_map(|a| a.jip_wraps).copied().collect();
    let fix_sites: Vec<u32> = AREAS.iter().flat_map(|a| a.fix_sites).copied().collect();
    let fix_wraps: Vec<u32> = AREAS.iter().flat_map(|a| a.fix_wraps).copied().collect();
    let patch_addresses: Vec<u32> = engine_core::extend::PATCH_SITES
        .iter()
        .chain(engine_core::extend::PATCH_WRAPS)
        .flat_map(|t| t.iter().map(|r| r.addr))
        .chain(fix_sites.iter().chain(&fix_wraps).copied())
        .collect();
    let mut e = base();
    let bare = snapshot(&e);
    crate::install_as(&mut e, true);
    let installed: std::collections::BTreeSet<_> = snapshot(&e)
        .difference(&bare)
        .filter(|(a, _, _)| patch_addresses.contains(a))
        .cloned()
        .collect();
    assert_eq!(installed, expected(&jip_sites, &jip_wraps));
    let before = snapshot(&e);
    set_row(&mut e, false);
    assert_eq!(snapshot(&e), before, "the row off adds nothing");
    set_row(&mut e, true);
    let after = snapshot(&e);
    assert!(before.is_subset(&after), "nothing taken out");
    let added: std::collections::BTreeSet<_> = after.difference(&before).cloned().collect();
    assert_eq!(added, expected(&fix_sites, &fix_wraps));
    let mut first = base();
    set_row(&mut first, true);
    crate::install_as(&mut first, true);
    assert_eq!(snapshot(&first), after, "the host's order");
}
