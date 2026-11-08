//! Tests of the co-save's save and load (I-10), with the game's file calls, its save folder and
//! its file list stood in.

use super::*;
use crate::vars::{Key, Value, KEY_NUMBER, KEY_STRING};
use std::cell::RefCell;
use std::collections::HashMap;

const AREA: u32 = 0x0200_0000;
const HANDLER: u32 = AREA + 0x1000;
const INFOS: u32 = AREA + 0x4000;
const NODES: u32 = AREA + 0x6000;
const NAME: u32 = AREA + 0x8000;

thread_local! {
    /// The stand-in file system: files by path, open handles (path, position, writing), the
    /// calls seen.
    static FILES: RefCell<HashMap<Vec<u8>, Vec<u8>>> = RefCell::new(HashMap::new());
    static HANDLES: RefCell<HashMap<u32, (Vec<u8>, usize)>> = RefCell::new(HashMap::new());
    static CALLS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn text(e: &Engine, at: u32) -> Vec<u8> {
    crate::table::guest_text(e, at)
}

fn create_file(e: &mut Engine, a: &[u32]) -> Ret {
    let path = text(e, a[0]);
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

fn read_file_stub(e: &mut Engine, a: &[u32]) -> Ret {
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

fn write_file_stub(e: &mut Engine, a: &[u32]) -> Ret {
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

fn delete_file_stub(e: &mut Engine, a: &[u32]) -> Ret {
    let path = text(e, a[0]);
    CALLS.with(|c| {
        c.borrow_mut()
            .push(format!("delete {}", String::from_utf8_lossy(&path)))
    });
    FILES.with(|f| f.borrow_mut().remove(&path));
    Ret::default()
}

fn rename_stub(e: &mut Engine, a: &[u32]) -> Ret {
    let (from, to) = (text(e, a[0]), text(e, a[1]));
    CALLS.with(|c| {
        c.borrow_mut().push(format!(
            "rename {} {}",
            String::from_utf8_lossy(&from),
            String::from_utf8_lossy(&to)
        ))
    });
    FILES.with(|f| {
        let mut f = f.borrow_mut();
        if let Some(d) = f.remove(&from) {
            f.insert(to, d);
        }
    });
    Ret::default()
}

fn save_dir_stub(e: &mut Engine, a: &[u32]) -> Ret {
    e.mem.write(a[1], b"C:\\Saves\\\0");
    Ret::default()
}

fn upper(_: &mut Engine, a: &[u32]) -> Ret {
    Ret {
        eax: u32::from((a[0] as u8).to_ascii_uppercase()),
        ..Ret::default()
    }
}

/// The game's files: `names` in the list of files, the loaded ones in that order.
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

fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x1_0000);
    e.map(DATA_HANDLER & !0xfff, 0x1000);
    e.map(SAVE_GAME_MANAGER & !0xfff, 0x1000);
    e.register(0x00ec_a7f4, upper);
    e.register(CREATE_FILE_A, create_file);
    e.register(READ_FILE, read_file_stub);
    e.register(WRITE_FILE, write_file_stub);
    e.register(CLOSE_HANDLE, close_handle);
    e.register(DELETE_FILE_A, delete_file_stub);
    e.register(CRT_RENAME, rename_stub);
    e.register(CONSTRUCT_SAVEGAME_PATH, save_dir_stub);
    install(&mut e);
    e.mem.write(NAME, b"Save 7.fos\0");
    e
}

fn file(path: &str) -> Option<Vec<u8>> {
    FILES.with(|f| f.borrow().get(path.as_bytes()).cloned())
}

/// The co-save's name: `.nvse` for the last extension, `.bak` suffixes kept, the save folder
/// before a bare name.
#[test]
fn co_save_names_as_xnvse() {
    let dir = || b"C:\\Saves\\".to_vec();
    assert_eq!(co_save_name(b"Save 1.fos", dir), b"C:\\Saves\\Save 1.nvse");
    assert_eq!(
        co_save_name(b"D:\\x\\a.b.fos.bak.bak", dir),
        b"D:\\x\\a.b.nvse.bak.bak"
    );
    assert_eq!(co_save_name(b"plain", dir), b"C:\\Saves\\plain.nvse");
}

/// xNVSE's map order: a key's bucket `(k * 13) ^ (k >> 15)` of 8, newest first in a bucket,
/// doubled once more entries than buckets are in.
#[test]
fn hash_order_follows_the_unordered_map() {
    let mut o = vars::HashOrder::default();
    for k in [1, 2, 3, 9] {
        o.insert(k);
    }
    // Buckets 13 & 7 = 5, 26 & 7 = 2, 39 & 7 = 7, 117 & 7 = 5.
    assert_eq!(o.keys(), [2, 9, 1, 3]);
    o.remove(9);
    assert_eq!(o.keys(), [2, 1, 3]);
    let mut o = vars::HashOrder::default();
    for k in 1..=10 {
        o.insert(k);
    }
    // The tenth insert finds 9 entries in 8 buckets: 16 buckets, bucket = 13k & 15.
    let mut want: Vec<(u32, u32)> = (1..=10).map(|k| ((13 * k) & 15, k)).collect();
    want.sort_by_key(|&(b, _)| b);
    assert_eq!(o.keys(), want.iter().map(|&(_, k)| k).collect::<Vec<_>>());
}

/// A save writes the core plugin (files, arrays, strings; temporaries, unreferenced arrays and
/// strings of no file left out) and a load before the `.fos` reads it back with the same ids;
/// a later load maps owners and form ids to today's file order and drops a removed file's
/// strings.
#[test]
fn variables_survive_a_save_and_a_load() {
    let mut e = engine();
    set_files(&mut e, &["FalloutNV.esm", "A.esp", "B.esp"]);
    let kept = vars::add_string(&mut e, 1, b"kept", false);
    let temp = vars::add_string(&mut e, 1, b"temp", true);
    let runtime = vars::add_string(&mut e, 0xff, b"console", false);
    let of_b = vars::add_string(&mut e, 2, b"of b", false);
    let arr = vars::create_array(&mut e, KEY_NUMBER, true, 1);
    vars::set_element(&mut e, arr, &Key::Number(0.0), Value::Number(2.5));
    vars::set_element(&mut e, arr, &Key::Number(1.0), Value::Str(b"x".to_vec()));
    vars::set_element(&mut e, arr, &Key::Number(2.0), Value::Form(0x0000_0abc));
    let map = vars::create_array(&mut e, KEY_STRING, false, 2);
    vars::set_element(&mut e, map, &Key::Str(b"Key".to_vec()), Value::Number(1.0));
    let mut held = 0;
    vars::add_reference(&mut e, &mut held, arr, 1);
    let mut held2 = 0;
    vars::add_reference(&mut e, &mut held2, map, 2);
    let unheld = vars::create_array(&mut e, KEY_NUMBER, false, 1);
    save_game(&mut e, NAME);
    let bytes = file("C:\\Saves\\Save 7.nvse").expect("written");
    let mut r = Reader::open(&bytes).unwrap();
    let p = r.next_plugin().unwrap().unwrap();
    assert_eq!(p.opcode_base, 0x1400);
    let mut kinds = Vec::new();
    while let Some((k, _, _)) = r.next_record().unwrap() {
        kinds.push(k.to_be_bytes());
    }
    assert_eq!(
        kinds,
        [
            *b"MODS", *b"ARVS", *b"ARVR", *b"ARVR", *b"ARVE", *b"STVS", *b"STVR", *b"STVR",
            *b"STVE"
        ]
    );
    assert!(!vars::array_exists(&e, unheld), "cleaned before the save");
    // A load: reset, then the same variables back.
    vars::add_string(&mut e, 1, b"from the game before", false);
    pre_load(&mut e, NAME);
    assert_eq!(vars::string(&e, kept), Some(b"kept".to_vec()));
    assert_eq!(vars::string(&e, of_b), Some(b"of b".to_vec()));
    assert!(vars::string(&e, temp).is_none());
    assert!(vars::string(&e, runtime).is_none());
    let a = vars::array(&e, arr).unwrap();
    assert_eq!((a.owning_mod, a.refs.clone(), a.len()), (1, vec![1], 3));
    assert_eq!(
        vars::get_element(&mut e, arr, &Key::Number(1.0))
            .unwrap()
            .data,
        Data::Str(b"x".to_vec())
    );
    assert_eq!(
        vars::get_element(&mut e, map, &Key::Str(b"KEY".to_vec()))
            .unwrap()
            .data,
        Data::Number(1.0)
    );
    // Today: B.esp is gone and A.esp moved to index 2.
    set_files(&mut e, &["FalloutNV.esm", "C.esp", "A.esp"]);
    pre_load(&mut e, NAME);
    assert!(vars::string(&e, of_b).is_none(), "its file is gone");
    assert_eq!(e.ext::<Nvse>().vars.strings[&kept].owning_mod, 2);
    assert_eq!(vars::array(&e, arr).unwrap().refs, vec![2]);
    assert_eq!(
        vars::get_element(&mut e, arr, &Key::Number(2.0))
            .unwrap()
            .data,
        Data::Form(0x0000_0abc),
        "an id of FalloutNV.esm stays"
    );
    assert!(
        !vars::array_exists(&e, map),
        "referred to by no loaded file"
    );
}

/// No co-save: the load before the `.fos` changes nothing (xNVSE: the game before's variables
/// stay); the load after it is a new game (temporaries cleaned).
#[test]
fn no_co_save_keeps_the_variables_and_is_a_new_game_after() {
    let mut e = engine();
    set_files(&mut e, &["FalloutNV.esm"]);
    let kept = vars::add_string(&mut e, 0, b"kept", false);
    let temp = vars::add_string(&mut e, 0, b"temp", true);
    pre_load(&mut e, NAME);
    assert!(vars::string(&e, kept).is_some() && vars::string(&e, temp).is_some());
    load(&mut e, NAME);
    assert!(vars::string(&e, kept).is_some());
    assert!(vars::string(&e, temp).is_none());
}

/// Arrays read out of id order: xNVSE marks the ids below each one it reads free from the last
/// one read, so a lower id read after a higher leaves a gap unmarked.
#[test]
fn array_ids_freed_as_xnvse_reads_them() {
    let mut e = engine();
    set_files(&mut e, &["FalloutNV.esm", "A.esp"]);
    let arvr = |id: u32| {
        let mut w = ChunkWriter::default();
        w.u8(1);
        w.u32(id);
        w.u8(KEY_NUMBER);
        w.u8(1);
        w.u32(1);
        w.u8(1);
        w.u32(0);
        Chunk {
            kind: tag(b"ARVR"),
            version: 2,
            data: w.data,
        }
    };
    let mut mods = ChunkWriter::default();
    mods.u8(2);
    for n in ["FalloutNV.esm", "A.esp"] {
        mods.u16(n.len() as u16);
        mods.bytes(n.as_bytes());
    }
    let marker = |t: &[u8; 4]| Chunk {
        kind: tag(t),
        version: 2,
        data: Vec::new(),
    };
    let bytes = cosave::write(&[Plugin {
        opcode_base: 0x1400,
        chunks: vec![
            Chunk {
                kind: tag(b"MODS"),
                version: 0,
                data: mods.data,
            },
            marker(b"ARVS"),
            arvr(5),
            arvr(3),
            marker(b"ARVE"),
        ],
    }]);
    FILES.with(|f| {
        f.borrow_mut()
            .insert(b"C:\\Saves\\Save 7.nvse".to_vec(), bytes)
    });
    pre_load(&mut e, NAME);
    let ids = &e.ext::<Nvse>().vars.array_ids;
    assert_eq!(ids.available.iter().copied().collect::<Vec<_>>(), [1, 2, 4]);
    assert!(vars::array_exists(&e, 5) && vars::array_exists(&e, 3));
}

/// Deleting or renaming a save does the same to its co-save first.
#[test]
fn delete_and_rename_take_the_co_save_along() {
    let mut e = engine();
    e.mem.write(NAME, b"C:\\Saves\\Old.fos\0");
    e.mem.write(NAME + 0x100, b"C:\\Saves\\Old.fos.bak\0");
    e.site(DELETE.addr(), &[NAME]);
    e.site(RENAME.addr(), &[NAME, NAME + 0x100]);
    let calls = CALLS.with(|c| c.borrow().clone());
    assert_eq!(
        calls,
        [
            "delete C:\\Saves\\Old.nvse",
            "delete C:\\Saves\\Old.nvse.bak",
            "rename C:\\Saves\\Old.nvse C:\\Saves\\Old.nvse.bak"
        ]
    );
}
