//! Tests of `ln_fn_utility`: the time stamp (the `GetLocalTime` import as a
//! stand-in filling a `SYSTEMTIME`: hour +8, minute +10, second +12), form lists (forms at
//! +0x18, a `tList`) and form arrays.

use super::*;
use crate::commands::test_support::*;
use nvse::vars::{Data, Value as V};

const LIST: u32 = FORMS + 0x1000;
const NODE2: u32 = FORMS + 0x1100;
const NODE3: u32 = FORMS + 0x1110;
const WEAP: u32 = FORMS + 0x2000;
const ARMO: u32 = FORMS + 0x2100;
const MISC: u32 = FORMS + 0x2200;
const REFR: u32 = FORMS + 0x2300;
const REF_VT: u32 = FORMS + 0x3000;

fn setup() -> Engine {
    let mut e = engine();
    game_globals(&mut e);
    e.mem.set_u32(REF_VT + 0xf0, 0x008d_0360);
    form(&mut e, LIST, 0x55, 0x0000_0100);
    form(&mut e, WEAP, 0x28, 0x0000_0201);
    form(&mut e, ARMO, 0x18, 0x0000_0202);
    form(&mut e, MISC, 0x1f, 0x0000_0203);
    form(&mut e, REFR, 0x3a, 0x0000_0204);
    e.mem.set_u32(REFR, REF_VT);
    e.mem.set_u32(REFR + 0x20, ARMO);
    for f in [WEAP, ARMO, MISC] {
        e.mem.set_u32(f, REF_VT + 0x200);
    }
    for (i, (f, id)) in [(WEAP, 0x201), (ARMO, 0x202), (MISC, 0x203), (REFR, 0x204)]
        .iter()
        .enumerate()
    {
        in_form_map(&mut e, *f, *id, i as u32);
    }
    set_ref(&mut e, 1, LIST);
    set_ref(&mut e, 2, WEAP);
    e
}

#[test]
fn the_time_stamp() {
    let mut e = setup();
    e.register(0x00fd_f0d8, |e, w| {
        e.mem.set_u16(w[0] + 8, 7);
        e.mem.set_u16(w[0] + 10, 5);
        e.mem.set_u16(w[0] + 12, 32);
        Ret::default()
    });
    let (_, res) = call(&mut e, 0x2202, &[]);
    assert_eq!(string_result(&e, res), b"07-05-32");
    assert!(
        calls().iter().all(|c| c.0 != 0x0071_d0a0),
        "console closed: no print"
    );
    // Console open: the text printed (as the format).
    e.register(0x0071_b160, |_, _| Ret {
        eax: 0x0300_f000,
        ..Ret::default()
    });
    e.register(0x0071_d0a0, |e, w| {
        let t = nvse::table::guest_text(e, w[1]);
        CALLS.with(|c| {
            c.borrow_mut()
                .push((0x0071_d0a0, t.iter().map(|&b| u32::from(b)).collect()))
        });
        Ret::default()
    });
    console(&mut e, true);
    call(&mut e, 0x2202, &[]);
    let printed: Vec<u8> = calls_to(0x0071_d0a0)[0].iter().map(|&b| b as u8).collect();
    assert_eq!(printed, b"07-05-32");
}

#[test]
fn a_form_list_as_an_array() {
    let mut e = setup();
    // WEAP, an empty node, MISC.
    e.mem.set_u32(LIST + 0x18, WEAP);
    e.mem.set_u32(LIST + 0x1c, NODE2);
    e.mem.set_u32(NODE2 + 4, NODE3);
    e.mem.set_u32(NODE3, MISC);
    let (_, res) = call(&mut e, 0x2212, &[r(1)]);
    assert_eq!(
        array_data(&e, res),
        vec![Data::Form(0x201), Data::Form(0x203)]
    );
    // An empty list: an empty array.
    e.mem.set_u32(LIST + 0x18, 0);
    e.mem.set_u32(LIST + 0x1c, 0);
    let (_, res) = call(&mut e, 0x2212, &[r(1)]);
    assert!(res != 0.0 && array_data(&e, res).is_empty());
    // Not a form list: refused by the reader.
    assert_eq!(call(&mut e, 0x2212, &[r(2)]).1, 0.0);
}

#[test]
fn forms_sorted_by_type() {
    let mut e = setup();
    let forms = array_of(
        &mut e,
        vec![
            V::Form(0x201),
            V::Form(0x203),
            V::Number(5.0),
            V::Form(0x204),
            V::Form(0x202),
            V::Form(0x0700_0000),
        ],
    );
    // Armour (0x18, also the reference by its base), then weapons (0x28).
    let types = array_of(&mut e, vec![V::Number(24.9), V::Number(40.0)]);
    let (_, res) = call(&mut e, 0x22ff, &[n(forms as i32), n(types as i32)]);
    // Each type's forms from the end back: ARMO before REFR; MISC left over at the end.
    assert_eq!(
        array_data(&e, res),
        vec![
            Data::Form(0x202),
            Data::Form(0x204),
            Data::Form(0x201),
            Data::Form(0x203)
        ]
    );
    // An empty type array: nothing.
    let none = array_of(&mut e, vec![]);
    assert_eq!(
        call(&mut e, 0x22ff, &[n(forms as i32), n(none as i32)]).1,
        0.0
    );
    assert_eq!(call(&mut e, 0x22ff, &[n(forms as i32), n(9999)]).1, 0.0);
}

#[test]
fn forms_counted_by_type() {
    let mut e = setup();
    let forms = array_of(
        &mut e,
        vec![
            V::Form(0x201),
            V::Form(0x204),
            V::Form(0x202),
            V::Number(40.0),
        ],
    );
    assert_eq!(call(&mut e, 0x2300, &[n(forms as i32), n(0x18)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2300, &[n(forms as i32), n(0x3a)]).1, 1.0);
    assert_eq!(call(&mut e, 0x2300, &[n(forms as i32), n(0x29)]).1, 0.0);
    // A map is refused.
    let map = nvse::vars::create_array(&mut e, nvse::vars::KEY_NUMBER, false, 0);
    nvse::vars::set_element(&mut e, map, &nvse::vars::Key::Number(3.0), V::Form(0x202));
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(call(&mut e, 0x2300, &[n(map as i32), n(0x18)]).1, 0.0);
}

// ---- Mods' files: FileExists and the INI commands ----

const MOD_INFO: u32 = FORMS + 0x4000;

/// The engine with the seam's stand-in, the script of mod 2 (`oHUD.esp`).
fn files_setup() -> Engine {
    let mut e = setup();
    files::fresh();
    loaded_mod(&mut e, 2, MOD_INFO, b"oHUD.esp");
    e.mem.set_u8(crate::test_support::SCRIPT + 0xf, 2);
    e
}

fn res_of(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> f64 {
    e.mem.set_f64(RESULT, 0.0);
    call(e, opcode, args).1
}

/// `GetINIPath`: `data\config\`, the name given (`/` made `\`) or the script's mod, the part
/// after the last `.` made `ini`.
#[test]
fn the_ini_file_a_command_names() {
    let mut e = files_setup();
    let script = crate::test_support::SCRIPT;
    fn path(e: &Engine, name: &[u8]) -> Option<Vec<u8>> {
        ini_path(e, crate::test_support::SCRIPT, name.to_vec())
    }
    assert_eq!(path(&e, b"oHUD").unwrap(), br"data\config\oHUD.ini");
    assert_eq!(path(&e, b"a/b.txt").unwrap(), br"data\config\a\b.ini");
    assert_eq!(path(&e, b"my.mod/cfg").unwrap(), br"data\config\my.ini");
    assert_eq!(path(&e, b"x.").unwrap(), br"data\config\x.ini");
    assert_eq!(path(&e, b"").unwrap(), br"data\config\oHUD.ini");
    // No file for the script's index, or index 0xFF: nothing.
    e.mem.set_u8(script + 0xf, 3);
    assert_eq!(path(&e, b""), None);
    e.mem.set_u8(script + 0xf, 0xff);
    assert_eq!(path(&e, b""), None);
    assert_eq!(ini_path(&e, 0, Vec::new()), None);
}

/// `GetValueDelim`: the first `:`, `/` or `\`; none, or an empty key, is nothing.
#[test]
fn a_value_name_splits_at_its_first_delimiter() {
    let split = split_value_name;
    assert_eq!(split(b"Main:Scale"), Some((&b"Main"[..], &b"Scale"[..])));
    assert_eq!(split(b"Main/a:b"), Some((&b"Main"[..], &b"a:b"[..])));
    assert_eq!(split(b"Main\\x"), Some((&b"Main"[..], &b"x"[..])));
    assert_eq!(split(b":x"), Some((&b""[..], &b"x"[..])));
    assert_eq!(split(b"Main"), None);
    assert_eq!(split(b"Main:"), None);
}

/// Floats and texts written and read back; the file's folders made first; a value cut to the
/// buffer's size; nothing for a missing key.
#[test]
fn ini_values_written_and_read_back() {
    let mut e = files_setup();
    assert_eq!(
        res_of(&mut e, 0x227c, &[s(b"Main:fScale"), z(1.25), s(b"")]),
        1.0
    );
    assert_eq!(
        files::get(r"data\config\oHUD.ini").unwrap(),
        b"[Main]\r\nfScale=1.25\r\n"
    );
    assert_eq!(
        files::folders(),
        vec![b"data".to_vec(), b"data\\config".to_vec()]
    );
    assert_eq!(res_of(&mut e, 0x227a, &[s(b"main:FSCALE"), s(b"")]), 1.25);
    assert_eq!(
        res_of(&mut e, 0x227d, &[s(b"Main/sName"), s(b"hello"), s(b"oHUD")]),
        1.0
    );
    let res = res_of(&mut e, 0x227b, &[s(b"Main\\sName"), s(b"oHUD.esp")]);
    assert_eq!(string_result(&e, res), b"hello");
    // Missing: no number, an empty text.
    assert_eq!(res_of(&mut e, 0x227a, &[s(b"Main:none"), s(b"")]), 0.0);
    let res = res_of(&mut e, 0x227b, &[s(b"Main:none"), s(b"")]);
    assert_eq!(string_result(&e, res), b"");
    // No delimiter: nothing written.
    let before = files::writes();
    assert_eq!(res_of(&mut e, 0x227c, &[s(b"Main"), z(2.0), s(b"")]), 0.0);
    assert_eq!(files::writes(), before);
    // A float reads its first 31 bytes; an empty value is no number.
    files::put(
        r"data\config\n.ini",
        b"[A]\r\nlong=0.1234567890123456789012345678901234\r\nbig=123456789012345678901234567890123\r\nempty=\r\n",
    );
    assert_eq!(
        res_of(&mut e, 0x227a, &[s(b"A:long"), s(b"n")]),
        123_456_789.0 * 1.0e-09
    );
    assert_eq!(
        res_of(&mut e, 0x227a, &[s(b"A:big"), s(b"n")]),
        4294967296.0
    );
    e.mem.set_f64(RESULT, 7.0);
    call(&mut e, 0x227a, &[s(b"A:empty"), s(b"n")]);
    assert_eq!(e.mem.f64(RESULT), 7.0, "an empty value leaves the result");
    // The 31-byte cut: a fraction whose tenth digit is the 32nd byte is not read past.
    files::put(
        r"data\config\c.ini",
        b"[A]\r\nk=000000000000000000000000000001.5\r\n",
    );
    assert_eq!(res_of(&mut e, 0x227a, &[s(b"A:k"), s(b"c")]), 1.0);
    // A text value cut to 0x3FFF bytes.
    let long = vec![b'x'; 0x5000];
    files::put(
        r"data\config\t.ini",
        &[&b"[A]\r\nk="[..], &long[..], &b"\r\n"[..]].concat(),
    );
    let res = res_of(&mut e, 0x227b, &[s(b"A:k"), s(b"t")]);
    assert_eq!(string_result(&e, res).len(), 0x3fff);
}

/// A name that leaves the game's folder is refused by the seam: nothing written, result 0, as
/// JIP answers for a file Windows could not write.
#[test]
fn a_refused_name_writes_nothing() {
    let mut e = files_setup();
    assert_eq!(
        res_of(&mut e, 0x227c, &[s(b"A:x"), z(1.0), s(b"..\\..\\evil")]),
        0.0
    );
    assert_eq!(
        res_of(&mut e, 0x227d, &[s(b"A:x"), s(b"v"), s(b"../x")]),
        0.0
    );
    assert_eq!(files::writes(), 0);
}

/// `GetINISection`: a string map of the entries with a value, text or numbers; empty when the
/// file or section is not there.
#[test]
fn an_ini_section_as_a_map() {
    let mut e = files_setup();
    files::put(
        r"data\config\oHUD.ini",
        b"[Main]\r\na = 1.5 \r\nb==2\r\nnovalue=\r\njunk\r\n;c=3\r\n=4\r\n[Other]\r\nz=9\r\n",
    );
    let entries = |e: &Engine, res: f64| -> Vec<(Vec<u8>, Data)> {
        let Container::StrMap(items) = &nvse::vars::array(e, res as u32).unwrap().elements else {
            panic!("a string map");
        };
        let mut v: Vec<(Vec<u8>, Data)> = items
            .iter()
            .map(|(k, el)| (k.clone(), el.data.clone()))
            .collect();
        v.sort_by(|x, y| x.0.cmp(&y.0));
        v
    };
    let res = res_of(&mut e, 0x227e, &[s(b"main"), s(b"")]);
    assert_eq!(
        entries(&e, res),
        vec![
            (b"".to_vec(), Data::Str(b"4".to_vec())),
            (b"a".to_vec(), Data::Str(b"1.5".to_vec())),
            (b"b".to_vec(), Data::Str(b"2".to_vec())),
        ]
    );
    let res = res_of(&mut e, 0x227e, &[s(b"Main"), s(b""), n(1)]);
    assert_eq!(
        entries(&e, res),
        vec![
            (b"".to_vec(), Data::Number(4.0)),
            (b"a".to_vec(), Data::Number(1.5)),
            (b"b".to_vec(), Data::Number(2.0)),
        ]
    );
    // No section, no file: an empty map.
    let res = res_of(&mut e, 0x227e, &[s(b"None"), s(b"")]);
    assert!(res != 0.0 && entries(&e, res).is_empty());
    let res = res_of(&mut e, 0x227e, &[s(b"Main"), s(b"nofile")]);
    assert!(res != 0.0 && entries(&e, res).is_empty());
}

/// `SetINISection`: the section written whole from a string map (numbers as `FltToStr`, other
/// values `0`); nothing for an array that is not a string map or is empty.
#[test]
fn an_ini_section_written_from_a_map() {
    let mut e = files_setup();
    files::put(
        r"data\config\oHUD.ini",
        b"[Main]\r\nold=1\r\n;keep\r\n[B]\r\nx=1\r\n",
    );
    let map = nvse::vars::create_array(&mut e, nvse::vars::KEY_STRING, false, 0);
    for (k, v) in [
        (&b"num"[..], V::Number(0.5)),
        (&b"text"[..], V::Str(b"hi".to_vec())),
        (&b"form"[..], V::Form(0x201)),
    ] {
        nvse::vars::set_element(&mut e, map, &Key::Str(k.to_vec()), v);
    }
    assert_eq!(
        res_of(&mut e, 0x227f, &[s(b"main"), n(map as i32), s(b"")]),
        1.0
    );
    let Container::StrMap(items) = &nvse::vars::array(&e, map).unwrap().elements else {
        unreachable!()
    };
    let lines: Vec<u8> = items
        .iter()
        .flat_map(|(k, el)| {
            let v: &[u8] = match &el.data {
                Data::Number(_) => b"0.5",
                Data::Str(_) => b"hi",
                _ => b"0",
            };
            [&k[..], b"=", v, b"\r\n"].concat()
        })
        .collect();
    assert_eq!(
        files::get(r"data\config\oHUD.ini").unwrap(),
        [
            &b"[Main]\r\n"[..],
            &lines[..],
            &b";keep\r\n[B]\r\nx=1\r\n"[..]
        ]
        .concat()
    );
    assert_eq!(lines.len(), b"num=0.5\r\ntext=hi\r\nform=0\r\n".len());
    // A packed array, an empty map, no array: nothing.
    let packed = array_of(&mut e, vec![V::Number(1.0)]);
    assert_eq!(
        res_of(&mut e, 0x227f, &[s(b"X"), n(packed as i32), s(b"")]),
        0.0
    );
    let empty = nvse::vars::create_array(&mut e, nvse::vars::KEY_STRING, false, 0);
    assert_eq!(
        res_of(&mut e, 0x227f, &[s(b"X"), n(empty as i32), s(b"")]),
        0.0
    );
    assert_eq!(res_of(&mut e, 0x227f, &[s(b"X"), n(9999), s(b"")]), 0.0);
    // A new file: its folders made, the section alone.
    assert_eq!(
        res_of(&mut e, 0x227f, &[s(b"S"), n(map as i32), s(b"sub/new")]),
        1.0
    );
    let made = files::get(r"data\config\sub\new.ini").unwrap();
    assert_eq!(made, [&b"[S]\r\n"[..], &lines[..]].concat());
    assert!(files::folders().contains(&b"data\\config\\sub".to_vec()));
}

/// `GetINISectionNames`: every heading in order; an empty array without a file.
#[test]
fn ini_section_names_as_an_array() {
    let mut e = files_setup();
    files::put(
        r"data\config\oHUD.ini",
        b"[Main]\r\nx=1\r\n[ Two ]\r\n[]\r\n[Main]\r\n",
    );
    let res = res_of(&mut e, 0x2285, &[s(b"")]);
    assert_eq!(
        array_data(&e, res),
        vec![
            Data::Str(b"Main".to_vec()),
            Data::Str(b"Two".to_vec()),
            Data::Str(Vec::new()),
            Data::Str(b"Main".to_vec())
        ]
    );
    let res = res_of(&mut e, 0x2285, &[s(b"none")]);
    assert!(res != 0.0 && array_data(&e, res).is_empty());
}

/// JIP's walk of a filled buffer: a cut last text kept, an empty text kept.
#[test]
fn a_filled_buffer_walked_as_jip_walks_it() {
    let items = vec![b"abc".to_vec(), Vec::new(), b"de".to_vec()];
    let (buf, count) = engine_core::mod_files::profile_list(&items, 0x100);
    assert_eq!(walked(&buf, count), items);
    let (buf, count) = engine_core::mod_files::profile_list(&items, 8);
    assert_eq!(
        walked(&buf, count),
        vec![b"abc".to_vec(), Vec::new(), b"d".to_vec()]
    );
    assert!(walked(&[0], 0).is_empty());
}

/// `RemoveINIKey`/`RemoveINISection`: 1 when the file is there (a missing key too); nothing for a
/// missing file, which is not made.
#[test]
fn ini_keys_and_sections_removed() {
    let mut e = files_setup();
    files::put(
        r"data\config\oHUD.ini",
        b"[A]\r\nx=1\r\ny=2\r\n[B]\r\nz=3\r\n",
    );
    assert_eq!(res_of(&mut e, 0x22ae, &[s(b"A:x"), s(b"")]), 1.0);
    assert_eq!(
        files::get(r"data\config\oHUD.ini").unwrap(),
        b"[A]\r\ny=2\r\n[B]\r\nz=3\r\n"
    );
    assert_eq!(res_of(&mut e, 0x22ae, &[s(b"A:none"), s(b"")]), 1.0);
    assert_eq!(res_of(&mut e, 0x22ae, &[s(b"A"), s(b"")]), 0.0);
    assert_eq!(res_of(&mut e, 0x22af, &[s(b"a"), s(b"")]), 1.0);
    assert_eq!(
        files::get(r"data\config\oHUD.ini").unwrap(),
        b"[B]\r\nz=3\r\n"
    );
    let before = files::writes();
    assert_eq!(res_of(&mut e, 0x22ae, &[s(b"A:x"), s(b"missing")]), 0.0);
    assert_eq!(res_of(&mut e, 0x22af, &[s(b"A"), s(b"missing")]), 0.0);
    assert_eq!(files::writes(), before);
    assert_eq!(files::get(r"data\config\missing.ini"), None);
}

const HASH_PATH_FN: u32 = 0x00af_d270;
const ARCHIVE_HAS_FILE_FN: u32 = 0x00af_9bf0;
const ARCHIVE_NODE: u32 = FORMS + 0x5000;
const ARCHIVE: u32 = FORMS + 0x5100;

/// `FileExists`: `data\` and the name; 1 there as asked, 0 there as the other; 2 for a file in
/// an opened archive of its extension's type (looked for without `data\`).
#[test]
fn files_and_folders_found() {
    let mut e = files_setup();
    files::put(r"data\textures\a.dds", b"x");
    files::folder(r"data\textures");
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"textures/a.dds")]), 1.0);
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"textures\\a.dds"), n(1)]), 0.0);
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"textures"), n(1)]), 1.0);
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"textures")]), 0.0);
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"none"), n(1)]), 0.0);
    // Archives: one opened archive of meshes (type 1) holding "meshes\x.nif".
    e.map(0x011f_8000, 0x1000);
    e.mem.set_u32(0x011f_8160, ARCHIVE_NODE);
    e.mem.set_u32(ARCHIVE_NODE, ARCHIVE);
    e.mem.set_u32(ARCHIVE_NODE + 4, 0);
    e.mem.set_u16(ARCHIVE + 0x180, 1);
    e.register(HASH_PATH_FN, |e, w| {
        let path = nvse::table::guest_text(e, w[0]);
        CALLS.with(|c| {
            c.borrow_mut()
                .push((HASH_PATH_FN, path.iter().map(|&b| u32::from(b)).collect()))
        });
        e.mem.set_u32(w[1], 0x1111);
        e.mem.set_u32(w[2], 0x2222);
        Ret::default()
    });
    e.register(ARCHIVE_HAS_FILE_FN, |e, w| {
        let path = nvse::table::guest_text(e, w[5]);
        let hit = w[0] == ARCHIVE
            && e.mem.u32(w[1]) == 0x1111
            && e.mem.u32(w[2]) == 0x2222
            && path == b"meshes\\x.nif";
        Ret {
            eax: u32::from(hit),
            ..Ret::default()
        }
    });
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"meshes/x.nif")]), 2.0);
    let hashed: Vec<u8> = calls_to(HASH_PATH_FN)[0].iter().map(|&b| b as u8).collect();
    assert_eq!(hashed, b"meshes\\x.nif");
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"meshes/y.nif")]), 0.0);
    // An archive of another type is not asked.
    e.mem.set_u16(ARCHIVE + 0x180, 2);
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"meshes/x.nif")]), 0.0);
    e.mem.set_u16(ARCHIVE + 0x180, 1);
    // An extension not in the table, or none: not looked for.
    let before = calls_to(HASH_PATH_FN).len();
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"meshes/x.esp")]), 0.0);
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"meshes/x")]), 0.0);
    assert_eq!(calls_to(HASH_PATH_FN).len(), before);
    // An upper-case extension: the same hash, looked for.
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"meshes/x.NIF")]), 0.0);
    assert_eq!(calls_to(HASH_PATH_FN).len(), before + 1);
    // Asked for a folder: archives are not looked in.
    assert_eq!(res_of(&mut e, 0x2201, &[s(b"meshes/x.nif"), n(1)]), 0.0);
    assert_eq!(calls_to(HASH_PATH_FN).len(), before + 1);
}
