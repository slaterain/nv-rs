use super::*;
use crate::ini_write::tests::{engine as file_engine, seed};
use nvse::vars::{Container, Data};
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const SCRIPT: u32 = DATA + 0x2000;
const FILE: u32 = DATA + 0x3000;
const HANDLER: u32 = DATA + 0x4000;
const FORM: u32 = DATA + 0x5000;
const REMAPPED_FORM: u32 = FORM + 0x100;
#[derive(Default)]
struct Messages(Vec<Vec<u8>>);
fn engine() -> Engine {
    let mut e = file_engine();
    e.map(0x011c_3000, 0x1000);
    e.mem.set_u32(0x011c_3f2c, HANDLER);
    let vt = e.mem.alloc(0x200);
    e.mem.set_u32(SCRIPT, vt);
    e.mem.set_u32(vt + 0x130, DATA + 0x7000);
    e.register(DATA + 0x7000, |_, _| 0u32.into_ret());
    e.register(0x0071_b160, |_, _| (DATA + 0x7100).into_ret());
    e.register(0x0071_d0a0, |e, a| {
        let text = table::guest_text(e, e.mem.u32(a[2]));
        e.ext::<Messages>().0.push(text);
        Ret::default()
    });
    e.register(0x00ec_a7f4, |_, a| {
        u32::from((a[0] as u8).to_ascii_uppercase()).into_ret()
    });
    e.mem.set_u32(FORM + 0xc, 0x0100_2222);
    e.mem.set_u32(REMAPPED_FORM + 0xc, 0x0200_2222);
    e.register(GET_LIST_FILE, |e, a| {
        assert_eq!(a[0], HANDLER);
        let name = table::guest_text(e, a[1]);
        (if name.eq_ignore_ascii_case(b"TestMod.esp") {
            FILE
        } else {
            0
        })
        .into_ret()
    });
    e.register(LOOKUP_FORM, |e, a| {
        assert!(
            e.ext_free::<Cache>().0.try_lock().is_ok(),
            "JSON locks must end before form callbacks"
        );
        (match a[0] {
            0x0100_2222 => FORM,
            0x0200_2222 => REMAPPED_FORM,
            _ => 0,
        })
        .into_ret()
    });
    e.ext::<nvse::Nvse>().saveload.preload_mods = vec![0, 2];
    // Clearly marked fixture for the real portable WideCharToMultiByte seam:
    // these values exercise CP1252, C-string termination and a measured best fit.
    e.register(WIDE_TO_ANSI, |e, a| {
        assert!(
            e.ext_free::<Cache>().0.try_lock().is_ok(),
            "JSON locks must end before ACP conversion"
        );
        assert_eq!((a[0], a[1], a[3], a[6], a[7]), (0, 0, u32::MAX, 0, 0));
        let mut units = Vec::new();
        let mut at = a[2];
        while e.mem.u16(at) != 0 {
            units.push(e.mem.u16(at));
            at += 2;
        }
        let text = String::from_utf16(&units).unwrap();
        let mut bytes = text
            .chars()
            .map(|c| match c {
                '\u{20ac}' => 0x80,
                '\u{2013}' => 0x96,
                '\u{0153}' => 0x9c,
                '\u{0100}' => b'A',
                c if (c as u32) <= 0xff => c as u8,
                _ => b'?',
            })
            .collect::<Vec<_>>();
        bytes.push(0);
        if a[4] != 0 {
            assert!(a[5] >= bytes.len() as u32);
            e.mem.write(a[4], &bytes);
        }
        (bytes.len() as u32).into_ret()
    });
    e
}
fn string(s: &str) -> Vec<u8> {
    [
        vec![b'S'],
        (s.len() as u16).to_le_bytes().to_vec(),
        s.as_bytes().to_vec(),
    ]
    .concat()
}
fn number(n: f64) -> Vec<u8> {
    [vec![b'Z'], n.to_le_bytes().to_vec()].concat()
}
fn words(e: &mut Engine, args: &[Vec<u8>]) -> [u32; 8] {
    let mut body = vec![args.len() as u8];
    for arg in args {
        body.extend_from_slice(&((arg.len() + 2) as u16).to_le_bytes());
        body.extend_from_slice(arg);
    }
    e.mem.set_u16(DATA, OPCODE as u16);
    e.mem.set_u16(DATA + 2, body.len() as u16);
    e.mem.write(DATA + 4, &body);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, -123.0);
    let rec = table::record_of(e, OPCODE);
    [e.mem.u32(rec + 0x14), DATA, 0, 0, SCRIPT, 0, RESULT, OFFSET]
}
fn call(e: &mut Engine, args: &[Vec<u8>]) -> (f64, Returns) {
    let words = words(e, args);
    let rec = table::record_of(e, OPCODE);
    nvse::expr::with_evaluator(e, |e| {
        assert!(e.call(e.mem.u32(rec + 0x18), &words).bool());
        (
            e.mem.f64(RESULT),
            nvse::expr::expected_return_type(e).unwrap(),
        )
    })
}
fn read_at(e: &mut Engine, path: &str, pointer: &str, cache: bool) -> (f64, Returns) {
    call(
        e,
        &[
            string(path),
            string(pointer),
            number(0.0),
            number(f64::from(u8::from(cache))),
        ],
    )
}
fn value(e: &mut Engine, id: u32, key: &[u8]) -> Data {
    vars::get_element(e, id, &Key::Str(key.to_vec()))
        .unwrap()
        .data
}

#[test]
fn actual_record_defaults_alias_and_ambiguous_metadata() {
    let mut e = engine();
    let rec = table::record_of(&e, OPCODE);
    assert_eq!(table::opcode_by_name(&e, b"ReadFromJSON"), Some(OPCODE));
    assert_eq!(table::opcode_by_name(&e, b"ReadFromJSONFile"), Some(OPCODE));
    assert_eq!(table::return_type(&e, OPCODE), Returns::Ambiguous);
    assert_eq!(e.mem.u16(rec + 0x10), 0);
    assert_eq!(e.mem.u16(rec + 0x12), 4);
    let params = e.mem.u32(rec + 0x14);
    for (i, (kind, optional)) in [
        (0x8004, false),
        (0x8004, true),
        (0x8001, true),
        (0x8002, true),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(e.mem.u32(params + 12 * i as u32 + 4), kind);
        assert_eq!(e.mem.u32(params + 12 * i as u32 + 8), u32::from(optional));
    }
    seed("Data/MCM/test.json", b"42");
    assert_eq!(
        call(&mut e, &[string("Data/MCM/test.json")]),
        (42.0, Returns::Default)
    );
    assert_eq!(table::opcode_by_name(&e, b"WriteToJSON"), None);
    assert!(nvse::plugins::built_ins(&e)
        .iter()
        .all(|p| p.name != "ShowOffNVSE Plugin" || !p.listed));
}
#[test]
fn recursive_objects_arrays_scalars_and_source_encodings() {
    let mut e = engine();
    seed("Data/MCM/test.json",br#"{"a":[true,null,2.5,{"s":"\u20ac\u00e9\u2013\u0153\u0100\u3042\u0000ignored"}],"\u00e9key":7,"ref":"@TestMod.esp:00002222"}"#);
    let (root, kind) = read_at(&mut e, "Data/MCM/test.json", "", false);
    assert_eq!(kind, Returns::Array);
    let root = root as u32;
    assert_eq!(vars::array(&e, root).unwrap().owning_mod, 1);
    let Data::Array(array) = value(&mut e, root, b"a") else {
        panic!()
    };
    let Container::Array(entries) = &vars::array(&e, array).unwrap().elements else {
        panic!()
    };
    assert_eq!(entries[0].data, Data::Number(1.0));
    assert_eq!(entries[1].data, Data::Array(0));
    assert_eq!(entries[2].data, Data::Number(2.5));
    let Data::Array(sub) = entries[3].data else {
        panic!()
    };
    assert_eq!(
        value(&mut e, sub, b"s"),
        Data::Str(vec![0x80, 0xe9, 0x96, 0x9c, b'A', b'?'])
    );
    assert_eq!(
        value(&mut e, root, "ékey".as_bytes()),
        Data::Number(7.0),
        "object keys stay UTF8 rather than ACP"
    );
    assert_eq!(value(&mut e, root, b"ref"), Data::Form(0x0100_2222));
    let (result, kind) = read_at(&mut e, "Data/MCM/test.json", "/a/0", false);
    assert_eq!((result, kind), (1.0, Returns::Default));
    assert_eq!(
        read_at(&mut e, "Data/MCM/test.json", "/a/1", false),
        (0.0, Returns::Array)
    );
    let (result, kind) = read_at(&mut e, "Data/MCM/test.json", "/ref", false);
    assert_eq!(kind, Returns::Form);
    assert_eq!(result.to_bits() as u32, 0x0100_2222);
}
#[test]
fn strict_json_precision_first_duplicate_and_empty_containers() {
    let mut e = engine();
    seed("numbers.json",br#"{"n":9007199254740993,"n":1,"u":18446744073709551615,"i":-9223372036854775808,"neg":-0.0,"tiny":5e-324,"a":[],"o":{}}"#);
    assert_eq!(
        read_at(&mut e, "numbers.json", "/n", false).0,
        9007199254740992.0
    );
    assert_eq!(
        read_at(&mut e, "numbers.json", "/u", false).0,
        u64::MAX as f64
    );
    assert_eq!(
        read_at(&mut e, "numbers.json", "/i", false).0,
        i64::MIN as f64
    );
    assert_eq!(
        read_at(&mut e, "numbers.json", "/neg", false).0.to_bits(),
        (-0.0f64).to_bits()
    );
    assert_eq!(
        read_at(&mut e, "numbers.json", "/tiny", false).0,
        f64::from_bits(1)
    );
    for path in ["/a", "/o"] {
        let (id, kind) = read_at(&mut e, "numbers.json", path, false);
        assert_eq!(kind, Returns::Array);
        assert_ne!(id, 0.0);
    }
    for invalid in [
        "{\"x\":1,}",
        "[1,]",
        "// comment\n1",
        "NaN",
        "1e999",
        "0 1",
        "\u{feff}1",
        "\"\\uD800\"",
    ] {
        assert!(parse(invalid.as_bytes()).is_err(), "{invalid}");
    }
    // The source has no serde_json-specific128-level limit.
    let nested = format!("{}0{}", "[".repeat(140), "]".repeat(140));
    assert!(parse(nested.as_bytes()).is_ok());
}
#[test]
fn json_pointer_escaping_strict_array_indices_case_and_missing_targets() {
    let tree = parse(br#"{"a/b":{"~key":[10,20]},"":9,"Case":1,"case":2}"#).unwrap();
    assert_eq!(
        pointer(&tree, b"/a~1b/~0key/1").unwrap(),
        &Json::Number(20.0)
    );
    assert_eq!(pointer(&tree, b"/").unwrap(), &Json::Number(9.0));
    assert_eq!(pointer(&tree, b"/Case").unwrap(), &Json::Number(1.0));
    assert_eq!(pointer(&tree, b"/case").unwrap(), &Json::Number(2.0));
    for bad in [
        b"a".as_slice(),
        b"/a~2b",
        b"/a~1b/~0key/01",
        b"/a~1b/~0key/-",
        b"/a~1b/~0key/4294967295",
        b"/missing",
        b"/Case/0",
    ] {
        assert!(pointer(&tree, bad).is_err());
    }
}
#[test]
fn parsed_cache_uses_source_path_hash_and_returns_fresh_arrays_until_explicit_clear() {
    let mut e = engine();
    seed("Data/MCM/test.json", br#"{"x":1}"#);
    let (a, _) = read_at(&mut e, "Data/MCM/test.json", "", true);
    let a = a as u32;
    vars::set_element(&mut e, a, &Key::Str(b"x".to_vec()), Value::Number(99.0));
    seed("Data/MCM/test.json", br#"{"x":2}"#);
    let (b, _) = read_at(&mut e, "data\\mcm\\TEST.json", "", false);
    let b = b as u32;
    assert_ne!(a, b);
    assert_eq!(
        value(&mut e, b, b"x"),
        Data::Number(1.0),
        "cached tree reused even when cache arg false"
    );
    assert_ne!(
        hash(&normalized(b"Data/MCM/test.json")),
        hash(&normalized(b"./Data/MCM/test.json")),
        "source does not remove dot prefixes"
    );
    assert_eq!(
        normalized(b"/Data\\MCM/test.json"),
        b"\\Data\\MCM\\test.json"
    );
    assert!(clear(&mut e, b"DATA/MCM/test.json"));
    assert!(!clear(&mut e, b"Data/MCM/test.json"));
    assert_eq!(read_at(&mut e, "Data/MCM/test.json", "/x", false).0, 2.0);
    assert!(!e
        .ext_free::<Cache>()
        .0
        .lock()
        .unwrap()
        .trees
        .contains_key(&hash(b"Data\\MCM\\test.json")));
}
#[test]
fn source_hash_only_cache_collision_and_real_clear_command_are_preserved() {
    let mut e = engine();
    let first = "udzauxaf04k7.json";
    let second = "gyd7fsqfmyua.json";
    assert_ne!(first, second);
    assert_eq!(hash(first.as_bytes()), hash(second.as_bytes()));
    seed(first, b"1");
    seed(second, b"2");
    assert_eq!(read_at(&mut e, first, "", true).0, 1.0);
    assert_eq!(read_at(&mut e, second, "", false).0, 1.0);
    // Real ClearFileCacheShowOff expression record, mode 1, shares this cache.
    let mut body = vec![2];
    for arg in [string(second), number(1.0)] {
        body.extend_from_slice(&((arg.len() + 2) as u16).to_le_bytes());
        body.extend(arg);
    }
    e.mem.set_u16(DATA, 0x3d2c);
    e.mem.set_u16(DATA + 2, body.len() as u16);
    e.mem.write(DATA + 4, &body);
    e.mem.set_u32(OFFSET, 4);
    let record = table::record_of(&e, 0x3d2c);
    let words = [
        e.mem.u32(record + 0x14),
        DATA,
        0,
        0,
        SCRIPT,
        0,
        RESULT,
        OFFSET,
    ];
    assert!(e.call(e.mem.u32(record + 0x18), &words).bool());
    assert_eq!(e.mem.f64(RESULT), 1.0);
    assert_eq!(read_at(&mut e, second, "", false).0, 2.0);
}
#[test]
fn form_strings_use_showoff_first_eight_hex_mod_lookup_remap_and_cached_ids() {
    let mut e = engine();
    assert_eq!(hex(b"123456789"), 0x12345678);
    assert_eq!(hex(b"prefix12"), 0x12);
    assert_eq!(hex(b"0x1234"), 0x1234);
    for (json, expected) in [
        (br#""@TestMod.esp:00002222""#.as_slice(), 0x0100_2222),
        (br#""@01002222""#, 0x0200_2222),
        (br#""@Missing.esp:2222""#, 0),
        (br#""@notAnEditorID""#, 0),
    ] {
        seed("form.json", json);
        let (v, kind) = read_at(&mut e, "form.json", "", false);
        assert_eq!(kind, Returns::Form);
        assert_eq!(v.to_bits() as u32, expected);
    }
    e.mem.set_u8(FILE + 0x40c, 7);
    assert_eq!(
        string_to_ref(&mut e, b"TestMod.esp:00002222"),
        0x0100_2222,
        "source string-to-ref ID cache outlives load-order changes"
    );
    seed("s.json", br#""text""#);
    let (v, kind) = read_at(&mut e, "s.json", "", false);
    assert_eq!(kind, Returns::String);
    assert_eq!(vars::string(&e, v as u32), Some(b"text".to_vec()));
}
#[test]
fn source_string_ref_zero_placeholder_is_visible_to_a_reentrant_mod_lookup() {
    let mut e = engine();
    e.register(GET_LIST_FILE, |e, a| {
        assert_eq!(table::guest_text(e, a[1]), b"TestMod.esp");
        assert_eq!(string_to_ref(e, b"TestMod.esp:00002222"), 0);
        FILE.into_ret()
    });
    assert_eq!(string_to_ref(&mut e, b"TestMod.esp:00002222"), 0x0100_2222);
    assert_eq!(string_to_ref(&mut e, b"TestMod.esp:00002222"), 0x0100_2222);
}
#[test]
fn unsupported_modes_and_bad_data_report_real_native_runtime_errors_and_return_zero() {
    let mut e = engine();
    seed("mode.json", br#"{"x":1}"#);
    read_at(&mut e, "mode.json", "", true);
    e.ext::<nvse::Nvse>().main_thread_recorded = true;
    for mode in [1.0, 2.0] {
        assert_eq!(
            call(&mut e, &[string("mode.json"), string(""), number(mode)]).0,
            0.0
        );
    }
    let messages = &e.ext::<Messages>().0;
    assert!(messages
        .iter()
        .any(|m| String::from_utf8_lossy(m).contains("parser 1 is not implemented")));
    assert!(messages
        .iter()
        .any(|m| String::from_utf8_lossy(m).contains("parser 2 is not implemented")));
    seed("bad.json", b"[1,");
    assert_eq!(read_at(&mut e, "bad.json", "", false).0, 0.0);
    assert_eq!(read_at(&mut e, "missing.json", "", false).0, 0.0);
    assert_eq!(read_at(&mut e, "mode.json", "/absent", false).0, 0.0);
    let messages = &e.ext::<Messages>().0;
    for needle in [
        "Failed to parse file",
        "File not found",
        "Invalid JSON pointer",
    ] {
        assert!(
            messages
                .iter()
                .any(|m| String::from_utf8_lossy(m).contains(needle)),
            "{needle}"
        );
    }
}
#[test]
fn real_thread_views_share_parsed_trees_and_clear_without_holding_locks_across_conversion() {
    let mut owner = engine();
    let mut peer = engine();
    owner.share_slots_for_test(&mut peer);
    nvse::table::install(&mut peer);
    // Restore only JSON records, avoiding duplicate shared native hook installs.
    install(&mut peer);
    seed("shared.json", br#"{"n":1,"s":"text"}"#);
    read_at(&mut owner, "shared.json", "", true);
    seed("shared.json", br#"{"n":2,"s":"changed"}"#);
    assert_eq!(read_at(&mut peer, "shared.json", "/n", false).0, 1.0);
    assert!(clear(&mut peer, b"shared.json"));
    assert_eq!(read_at(&mut owner, "shared.json", "/n", false).0, 2.0);
}
