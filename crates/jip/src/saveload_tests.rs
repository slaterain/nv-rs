//! xNVSE's save and load passes (`nvse::saveload`) with JIP built in, the real plugin
//! ([`crate::install_as`]), over stand-ins of the game's file calls, save folder and file list:
//! JIP's section through a whole co-save, beside xNVSE's own and another plugin's, in both
//! presets.

use crate::cosave::{kind, set_johnny_guitar_fixes};
use crate::test_support::{unhex, REAL_COSAVES};
use engine_core::prelude::*;
use nvse::cosave::{self, tag, Chunk, Plugin, Reader};
use nvse::saveload;
use std::cell::RefCell;
use std::collections::HashMap;

// The game's imports and data the co-save passes use (the same addresses as `nvse::saveload`'s).
const CREATE_FILE_A: u32 = 0x00fd_f088;
const READ_FILE: u32 = 0x00fd_f080;
const WRITE_FILE: u32 = 0x00fd_f0dc;
const CLOSE_HANDLE: u32 = 0x00fd_f07c;
const DELETE_FILE_A: u32 = 0x00fd_f0d4;
const CRT_RENAME: u32 = 0x00ec_862c;
const TOUPPER: u32 = 0x00ec_a7f4;
const CONSTRUCT_SAVEGAME_PATH: u32 = 0x0084_ff30;
const GENERIC_WRITE: u32 = 0x4000_0000;
const INVALID_HANDLE_VALUE: u32 = 0xffff_ffff;
const DATA_HANDLER: u32 = 0x011c_3f2c;
const MOD_INFO_LIST: u32 = 0x210;
const LOADED_COUNT: u32 = 0x218;
const LOADED_MODS: u32 = 0x21c;
const MOD_NAME: u32 = 0x20;

const AREA: u32 = 0x0400_0000;
const HANDLER: u32 = AREA + 0x1000;
const INFOS: u32 = AREA + 0x4000;
const NODES: u32 = AREA + 0x7000;
const NAME: u32 = AREA + 0x8000;
const CO_SAVE: &[u8] = b"C:\\Saves\\Save 7.nvse";

/// The files `nvsecos4.nvse` lists, in its order.
const FILES_SAVED: [&str; 10] = [
    "FalloutNV.esm",
    "DeadMoney.esm",
    "HonestHearts.esm",
    "OldWorldBlues.esm",
    "LonesomeRoad.esm",
    "TribalPack.esm",
    "MercenaryPack.esm",
    "ClassicPack.esm",
    "CaravanPack.esm",
    "GunRunnersArsenal.esm",
];

thread_local! {
    static FILES: RefCell<HashMap<Vec<u8>, Vec<u8>>> = RefCell::new(HashMap::new());
    static HANDLES: RefCell<HashMap<u32, (Vec<u8>, usize)>> = RefCell::new(HashMap::new());
}

fn create_file(e: &mut Engine, a: &[u32]) -> Ret {
    let path = nvse::table::guest_text(e, a[0]);
    let write = a[1] & GENERIC_WRITE != 0;
    let exists = FILES.with(|f| f.borrow().contains_key(&path));
    if !write && !exists {
        return Ret {
            eax: INVALID_HANDLE_VALUE,
            ..Ret::default()
        };
    }
    if write {
        FILES.with(|f| f.borrow_mut().insert(path.clone(), Vec::new()));
    }
    let h = HANDLES.with(|h| {
        let mut h = h.borrow_mut();
        let id = 0x100 + h.len() as u32;
        h.insert(id, (path, 0));
        id
    });
    Ret {
        eax: h,
        ..Ret::default()
    }
}

fn read_file(e: &mut Engine, a: &[u32]) -> Ret {
    let (h, buf, n, read) = (a[0], a[1], a[2] as usize, a[3]);
    let (path, pos) = HANDLES.with(|hs| hs.borrow()[&h].clone());
    let data = FILES.with(|f| f.borrow()[&path].clone());
    let piece = &data[pos.min(data.len())..(pos + n).min(data.len())];
    e.mem.write(buf, piece);
    e.mem.set_u32(read, piece.len() as u32);
    HANDLES.with(|hs| hs.borrow_mut().get_mut(&h).unwrap().1 += piece.len());
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

fn write_file(e: &mut Engine, a: &[u32]) -> Ret {
    let (h, buf, n, written) = (a[0], a[1], a[2], a[3]);
    let path = HANDLES.with(|hs| hs.borrow()[&h].0.clone());
    let bytes = e.mem.bytes(buf, n);
    FILES.with(|f| f.borrow_mut().get_mut(&path).unwrap().extend(bytes));
    e.mem.set_u32(written, n);
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

fn close_handle(_: &mut Engine, a: &[u32]) -> Ret {
    HANDLES.with(|hs| hs.borrow_mut().remove(&a[0]));
    Ret {
        eax: 1,
        ..Ret::default()
    }
}

fn nothing(_: &mut Engine, _: &[u32]) -> Ret {
    Ret::default()
}

fn save_dir(e: &mut Engine, a: &[u32]) -> Ret {
    e.mem.write(a[1], b"C:\\Saves\\\0");
    Ret::default()
}

fn upper(_: &mut Engine, a: &[u32]) -> Ret {
    Ret {
        eax: u32::from((a[0] as u8).to_ascii_uppercase()),
        ..Ret::default()
    }
}

/// The game's list of loaded files.
fn set_files(e: &mut Engine, names: &[&str]) {
    e.mem.set_u32(DATA_HANDLER, HANDLER);
    e.mem.set_u8(HANDLER + LOADED_COUNT, names.len() as u8);
    for (i, name) in names.iter().enumerate() {
        let i = i as u32;
        let info = INFOS + i * 0x200;
        e.mem.write(info + MOD_NAME, name.as_bytes());
        e.mem.set_u8(info + MOD_NAME + name.len() as u32, 0);
        e.mem.set_u32(HANDLER + LOADED_MODS + 4 * i, info);
        let node = if i == 0 {
            HANDLER + MOD_INFO_LIST
        } else {
            NODES + i * 8
        };
        e.mem.set_u32(node, info);
        e.mem.set_u32(node + 4, 0);
        if i > 0 {
            let prev = if i == 1 {
                HANDLER + MOD_INFO_LIST
            } else {
                NODES + (i - 1) * 8
            };
            e.mem.set_u32(prev + 4, node);
        }
    }
}

/// xNVSE's table and co-save passes, JIP built in (not answering "installed"), the presets'
/// two rows as given, the files `nvsecos4` lists loaded.
fn engine(keep_row: bool, jg_row: bool) -> Engine {
    FILES.with(|f| f.borrow_mut().clear());
    HANDLES.with(|h| h.borrow_mut().clear());
    let mut e = crate::test_support::engine();
    e.map(AREA, 0x1_0000);
    for page in [0x011c_3000, 0x011d_d000, 0x011d_e000, 0x011c_5000] {
        e.map(page, 0x1000);
    }
    e.register(TOUPPER, upper);
    e.register(CREATE_FILE_A, create_file);
    e.register(READ_FILE, read_file);
    e.register(WRITE_FILE, write_file);
    e.register(CLOSE_HANDLE, close_handle);
    e.register(DELETE_FILE_A, nothing);
    e.register(CRT_RENAME, nothing);
    e.register(CONSTRUCT_SAVEGAME_PATH, save_dir);
    saveload::install(&mut e);
    crate::install_as(&mut e, false);
    saveload::set_keep_other_plugins(&mut e, keep_row);
    set_johnny_guitar_fixes(&mut e, jg_row);
    e.mem.write(NAME, b"Save 7.fos\0");
    // The player (`PlayerCharacter`, 0x011DEA3C), which JIP's pre-load and save read.
    e.mem.set_u32(0x011d_ea3c, AREA + 0x9000);
    set_files(&mut e, &FILES_SAVED);
    e
}

/// A load from the menu: before the `.fos`, after it, its end.
fn load_game(e: &mut Engine, file: &[u8]) {
    FILES.with(|f| f.borrow_mut().insert(CO_SAVE.to_vec(), file.to_vec()));
    saveload::pre_load(e, NAME);
    e.site(saveload::LOAD.addr(), &[NAME]);
    e.site(saveload::POST_LOAD.addr(), &[1]);
}

/// The co-save a save writes.
fn save_game(e: &mut Engine) -> Vec<u8> {
    saveload::save_game(e, NAME);
    FILES.with(|f| f.borrow()[CO_SAVE].clone())
}

/// A co-save's sections: opcode base and bytes (chunk headers and data).
fn sections(file: &[u8]) -> Vec<(u32, Vec<u8>)> {
    let mut r = Reader::open(file).unwrap();
    let mut out = Vec::new();
    while let Some(p) = r.next_plugin().unwrap() {
        out.push((p.opcode_base, r.take_plugin(p.length).unwrap()));
        r.end_plugin();
    }
    out
}

fn section_of(file: &[u8], base: u32) -> Option<Vec<u8>> {
    sections(file)
        .into_iter()
        .find(|(b, _)| *b == base)
        .map(|(_, s)| s)
}

fn chunk(kind: u32, version: u32, data: &[u8]) -> Chunk {
    Chunk {
        kind,
        version,
        data: data.to_vec(),
    }
}

/// The real `nvsecos4.nvse` with JIP's section extended: its `JPGF`, then `JPNP` (NPC perks,
/// which TTW turns on) and a record type JIP does not know, and a section of another plugin
/// (JohnnyGuitar's base) after it.
fn test_file() -> (Vec<u8>, Vec<Chunk>, Plugin) {
    let real = unhex(REAL_COSAVES[1].1);
    let mut r = Reader::open(&real).unwrap();
    let mut core = Vec::new();
    let mut jpgf = None;
    while let Some(p) = r.next_plugin().unwrap() {
        while let Some((k, v, len)) = r.next_record().unwrap() {
            let c = chunk(k, v, &r.read_data(len).unwrap());
            if p.opcode_base == 0x1400 {
                core.push(c);
            } else {
                jpgf = Some(c);
            }
        }
        r.end_plugin();
    }
    let jip = vec![
        jpgf.unwrap(),
        chunk(
            kind::NPC_PERKS,
            10,
            &[1, 0, 0x34, 0x12, 0, 6, 1, 0x78, 0x56, 0, 6, 2],
        ),
        chunk(tag(b"ZZPJ"), 3, b"later"),
    ];
    let other = Plugin {
        opcode_base: 0x3100,
        chunks: vec![chunk(tag(b"XXGJ"), 1, b"johnny")],
    };
    let file = cosave::write(&[
        Plugin {
            opcode_base: 0x1400,
            chunks: core,
        },
        Plugin {
            opcode_base: 0x2200,
            chunks: jip.clone(),
        },
        other.clone(),
    ]);
    (file, jip, other)
}

/// The fixes preset (keep row and JohnnyGuitar row on) and vanilla (both off): JIP's section,
/// `JPNP` and the unknown record included, comes back byte for byte in both; xNVSE's own section
/// is written; the other plugin's section comes back with the keep row and goes without it, as
/// with xNVSE.
#[test]
fn jips_section_through_a_whole_co_save_in_both_presets() {
    let (file, _, _) = test_file();
    let jip_in = section_of(&file, 0x2200).unwrap();
    let other_in = section_of(&file, 0x3100).unwrap();
    for (keep_row, jg_row) in [(true, true), (false, false), (true, false), (false, true)] {
        let mut e = engine(keep_row, jg_row);
        load_game(&mut e, &file);
        let out = save_game(&mut e);
        let what = format!("keep {keep_row} jg {jg_row}");
        assert_eq!(section_of(&out, 0x2200), Some(jip_in.clone()), "{what}");
        assert!(section_of(&out, 0x1400).is_some(), "{what}");
        assert_eq!(
            section_of(&out, 0x3100),
            keep_row.then(|| other_in.clone()),
            "{what}"
        );
    }
}

/// The keep row's rule: once today's files are not the ones the co-save lists, the carried
/// records (whose mod indexes name the saved files) are not written; JIP's own `JPGF` is.
#[test]
fn carried_records_go_when_the_files_change() {
    let (file, jip, _) = test_file();
    let mut e = engine(true, true);
    load_game(&mut e, &file);
    let mut moved = FILES_SAVED.to_vec();
    moved.insert(1, "New.esp");
    set_files(&mut e, &moved);
    let out = save_game(&mut e);
    let only_flags = cosave::write(&[Plugin {
        opcode_base: 0x2200,
        chunks: vec![jip[0].clone()],
    }]);
    assert_eq!(
        section_of(&out, 0x2200),
        section_of(&only_flags, 0x2200),
        "JPGF alone"
    );
}

/// A JIP section cut short by the file's end (its last record says 1,000 bytes, 5 are there):
/// no panic; xNVSE's own section, read before the `.fos`, is still there at the next save
/// with its arrays and strings, and JIP keeps what it read before the break (`JPGF`).
#[test]
fn a_damaged_jip_section_does_not_stop_the_load() {
    let (file, jip, _) = test_file();
    let core_in = section_of(&file, 0x1400).unwrap();
    // The core section, then JIP's header, its `JPGF`, and a `JPNP` header claiming 1,000 bytes.
    let real = unhex(REAL_COSAVES[1].1);
    let core_end = cosave::HEADER_SIZE + cosave::PLUGIN_HEADER_SIZE + core_in.len();
    let mut cut = real[..core_end].to_vec();
    cut[16..20].copy_from_slice(&2u32.to_le_bytes());
    let mut body = Vec::new();
    let c = &jip[0];
    body.extend_from_slice(&c.kind.to_le_bytes());
    body.extend_from_slice(&c.version.to_le_bytes());
    body.extend_from_slice(&(c.data.len() as u32).to_le_bytes());
    body.extend_from_slice(&c.data);
    body.extend_from_slice(&kind::NPC_PERKS.to_le_bytes());
    body.extend_from_slice(&10u32.to_le_bytes());
    body.extend_from_slice(&1000u32.to_le_bytes());
    body.extend_from_slice(&[1, 0, 0, 0, 0]);
    cut.extend_from_slice(&0x2200u32.to_le_bytes());
    cut.extend_from_slice(&2u32.to_le_bytes());
    cut.extend_from_slice(&(body.len() as u32 + 995).to_le_bytes());
    cut.extend_from_slice(&body);
    let mut e = engine(true, true);
    load_game(&mut e, &cut);
    let out = save_game(&mut e);
    let core_out = section_of(&out, 0x1400).unwrap();
    let records = |s: &[u8]| {
        let mut n = Vec::new();
        let mut at = 0;
        while at + 12 <= s.len() {
            n.push(s[at..at + 4].to_vec());
            at += 12 + u32::from_le_bytes(s[at + 8..at + 12].try_into().unwrap()) as usize;
        }
        n
    };
    let count = |s: &[u8], t: &[u8; 4]| records(s).iter().filter(|r| r[..] == t[..]).count();
    assert_eq!(count(&core_out, b"RVRA"), 3, "the three arrays");
    assert_eq!(count(&core_out, b"RVTS"), 3, "the three strings");
    let flags_only = cosave::write(&[Plugin {
        opcode_base: 0x2200,
        chunks: vec![jip[0].clone()],
    }]);
    assert_eq!(section_of(&out, 0x2200), section_of(&flags_only, 0x2200));
}
