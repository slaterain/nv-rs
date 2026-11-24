use super::*;
use engine_core::mod_files::ModFileCalls;
use std::{cell::RefCell, collections::BTreeMap};
thread_local! {
    static FILES:RefCell<BTreeMap<Vec<u8>,Vec<u8>>>=RefCell::default();
    static WRITES:RefCell<usize>=const{RefCell::new(0)};
    static DENY:RefCell<bool>=const{RefCell::new(false)};
}
fn path(name: &[u8]) -> Vec<u8> {
    name.iter()
        .map(|b| {
            if *b == b'/' {
                b'\\'
            } else {
                b.to_ascii_lowercase()
            }
        })
        .collect()
}
pub(crate) fn read(_: &Engine, name: &[u8]) -> Result<Vec<u8>, u32> {
    FILES.with(|f| f.borrow().get(&path(name)).cloned().ok_or(2))
}
fn exists(e: &Engine, name: &[u8]) -> bool {
    read(e, name).is_ok()
}
fn directory(_: &Engine, _: &[u8]) -> bool {
    true
}
fn create(_: &mut Engine, _: &[u8]) -> Result<(), u32> {
    Ok(())
}
fn write(_: &mut Engine, name: &[u8], bytes: &[u8]) -> Result<(), u32> {
    if DENY.with(|v| *v.borrow()) {
        return Err(5);
    }
    FILES.with(|f| f.borrow_mut().insert(path(name), bytes.to_vec()));
    WRITES.with(|n| *n.borrow_mut() += 1);
    Ok(())
}
fn append(e: &mut Engine, name: &[u8], bytes: &[u8]) -> Result<(), u32> {
    let mut data = read(e, name).unwrap_or_default();
    data.extend_from_slice(bytes);
    write(e, name, &data)
}
fn read_head(e: &Engine, name: &[u8], n: usize) -> Result<Vec<u8>, u32> {
    read(e, name).map(|mut b| {
        b.truncate(n);
        b
    })
}
fn list(_: &Engine, _: &[u8]) -> Vec<mod_files::Listed> {
    Vec::new()
}
fn name_key(name: &[u8]) -> Vec<u16> {
    name.iter()
        .map(|b| u16::from(b.to_ascii_uppercase()))
        .collect()
}
static CALLS: ModFileCalls = ModFileCalls {
    read,
    exists,
    directory_exists: directory,
    create_directory: create,
    write,
    append,
    read_head,
    list,
    name_key,
};
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const SCRIPT: u32 = DATA + 0x2000;
const FILE: u32 = DATA + 0x3000;
pub(crate) fn engine() -> Engine {
    mod_files::install(&CALLS);
    FILES.with(|f| f.borrow_mut().clear());
    WRITES.with(|n| *n.borrow_mut() = 0);
    DENY.with(|v| *v.borrow_mut() = false);
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(DATA, 0x10000);
    nvse::table::install(&mut e);
    crate::install(&mut e);
    e.mem.set_u32(SCRIPT + 0xc, 0x0100_1234);
    e.mem.write(FILE + 0x20, b"TestMod.esp\0");
    e.mem.set_u8(FILE + 0x40c, 1);
    e.register(0x0048_4e60, |_, a| {
        assert_eq!(a, [SCRIPT, u32::MAX]);
        Ret {
            eax: FILE,
            ..Ret::default()
        }
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
fn invoke(e: &mut Engine, op: u32, tokens: &[Vec<u8>]) -> (bool, f64) {
    let mut body = vec![tokens.len() as u8];
    for token in tokens {
        body.extend_from_slice(&((token.len() + 2) as u16).to_le_bytes());
        body.extend_from_slice(token);
    }
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, body.len() as u16);
    e.mem.write(DATA + 4, &body);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.5);
    let rec = table::record_of(e, op);
    let words = [e.mem.u32(rec + 0x14), DATA, 0, 0, SCRIPT, 0, RESULT, OFFSET];
    let ok = e.call(e.mem.u32(rec + 0x18), &words).bool();
    (ok, e.mem.f64(RESULT))
}
fn call(e: &mut Engine, op: u32, tokens: &[Vec<u8>]) -> f64 {
    let (ok, result) = invoke(e, op, tokens);
    assert!(ok);
    result
}
fn get(e: &mut Engine, key: &str, file: &str, default: &str) -> Vec<u8> {
    let id = call(e, 0x3d2a, &[string(key), string(file), string(default)]) as u32;
    nvse::vars::string(e, id).unwrap()
}
pub(crate) fn seed(name: &str, data: &[u8]) {
    FILES.with(|f| f.borrow_mut().insert(path(name.as_bytes()), data.to_vec()));
}

#[test]
fn actual_expression_records_types_aliases_and_returns() {
    let e = engine();
    for (op, name, alias, spec, returns) in [
        (
            0x3d26,
            "SetINIValue_Cached",
            "SetINIValC",
            &[
                (0x8004, false),
                (0x8005, false),
                (0x8004, true),
                (0x8004, true),
            ][..],
            Returns::Default,
        ),
        (
            0x3d3a,
            "SetINIInteger_Cached",
            "SetINIIntC",
            &[
                (0x8004, false),
                (0x8001, false),
                (0x8004, true),
                (0x8004, true),
            ][..],
            Returns::Default,
        ),
        (
            0x3d2a,
            "GetINIStringOrDefault_Cached",
            "GetINIString_Cached",
            &[(0x8004, false), (0x8004, true), (0x8004, true)][..],
            Returns::String,
        ),
        (
            0x3d2d,
            "SaveCachedIniFile",
            "SaveCachedIni",
            &[(0x8004, true)][..],
            Returns::Default,
        ),
    ] {
        assert_eq!(table::opcode_by_name(&e, name.as_bytes()), Some(op));
        assert_eq!(table::opcode_by_name(&e, alias.as_bytes()), Some(op));
        let rec = table::record_of(&e, op);
        assert_eq!(table::return_type(&e, op), returns);
        assert_eq!(e.mem.u16(rec + 0x10), 0);
        assert_eq!(e.mem.u16(rec + 0x12), spec.len() as u16);
        let params = e.mem.u32(rec + 0x14);
        for (i, (kind, optional)) in spec.iter().enumerate() {
            assert_eq!(e.mem.u32(params + 12 * i as u32 + 4), *kind);
            assert_eq!(e.mem.u32(params + 12 * i as u32 + 8), u32::from(*optional));
        }
    }
    assert_eq!(table::opcode_by_name(&e, b"ReadFromJSON"), Some(0x3d1a));
    assert_eq!(
        table::opcode_by_name(&e, b"SetINIFloat_Cached"),
        None,
        "source has SetINIValC, not an invented alias"
    );
}
#[test]
fn cached_update_is_visible_to_numeric_and_string_reads_but_saved_only_explicitly() {
    let mut e = engine();
    seed(
        "data/config/mod.ini",
        b"; file\n\n[Stats]\n; retained\nValue=1\nOther=keep\n",
    );
    assert_eq!(
        call(&mut e, 0x3d29, &[string("Stats:Value"), string("mod")]),
        1.0
    );
    assert_eq!(
        call(
            &mut e,
            0x3d26,
            &[
                string("STATS:value"),
                number(1.23456789),
                string("mod"),
                string("ignored replacement comment")
            ]
        ),
        1.0
    );
    assert_eq!(get(&mut e, "Stats:Value", "mod", "fallback"), b"1.234568");
    assert_eq!(
        call(&mut e, 0x3d29, &[string("Stats:Value"), string("mod")]),
        1.234568
    );
    assert_eq!(WRITES.with(|n| *n.borrow()), 0);
    assert_eq!(call(&mut e, 0x3d2d, &[string("mod")]), 1.0);
    let saved = read(&e, b"data/config/mod.ini").unwrap();
    assert_eq!(
        saved,
        b"; file\r\n\r\n\r\n[Stats]\r\n\r\n; retained\r\nValue = 1.234568\r\nOther = keep\r\n"
    );
    assert_eq!(call(&mut e, 0x3d2c, &[string("MOD"), number(0.0)]), 1.0);
    assert_eq!(get(&mut e, "Stats:Value", "mod", "fallback"), b"1.234568");
}
#[test]
fn creates_cache_only_preserves_integer_cast_and_comments_and_clears_without_saving() {
    let mut e = engine();
    assert_eq!(
        call(
            &mut e,
            0x3d3a,
            &[
                string("New:Integer"),
                number(9.9),
                string("new"),
                string("new setting")
            ]
        ),
        2.0
    );
    assert_eq!(get(&mut e, "New:Integer", "new", ""), b"9");
    assert!(!exists(&e, b"data/config/new.ini"));
    assert_eq!(
        call(
            &mut e,
            0x3d26,
            &[string("New:Text"), string("literal"), string("new")]
        ),
        2.0
    );
    assert_eq!(
        call(
            &mut e,
            0x3d26,
            &[string("New:Double"), number(2.0), string("new")]
        ),
        2.0
    );
    assert_eq!(get(&mut e, "New:Double", "new", ""), b"2.");
    assert_eq!(call(&mut e, 0x3d2d, &[string("new")]), 1.0);
    let saved = read(&e, b"data/config/new.ini").unwrap();
    assert!(saved
        .windows(b";new setting\r\nInteger = 9".len())
        .any(|s| s == b";new setting\r\nInteger = 9"));
    assert_eq!(
        call(
            &mut e,
            0x3d3a,
            &[string("New:Integer"), number(-3.9), string("new")]
        ),
        1.0
    );
    assert_eq!(call(&mut e, 0x3d2c, &[string("new"), number(0.0)]), 1.0);
    assert_eq!(
        get(&mut e, "New:Integer", "new", ""),
        b"9",
        "unsaved change discarded by CloseFileSO"
    );
}
#[test]
fn default_filename_missing_empty_values_failed_save_and_relative_cache_identity() {
    let mut e = engine();
    assert_eq!(
        call(&mut e, 0x3d26, &[string("Section:Empty"), string("")]),
        2.0
    );
    assert_eq!(get(&mut e, "Section:Empty", "", "fallback"), b"");
    assert_eq!(get(&mut e, "Section:Missing", "", "fallback"), b"fallback");
    DENY.with(|v| *v.borrow_mut() = true);
    assert_eq!(call(&mut e, 0x3d2d, &[]), 0.0);
    DENY.with(|v| *v.borrow_mut() = false);
    assert_eq!(call(&mut e, 0x3d2d, &[]), 1.0);
    assert!(exists(&e, b"data/config/TestMod.ini"));
    assert_eq!(
        call(&mut e, 0x3d2d, &[string("TestMod")]),
        0.0,
        "cache uses original relative name, not normalized full path"
    );
    assert_eq!(
        call(
            &mut e,
            0x3d26,
            &[string("missing separator"), number(2.0), string("bad")]
        ),
        -1.0
    );
    assert_eq!(
        call(
            &mut e,
            0x3d26,
            &[string("Section:"), number(2.0), string("bad")]
        ),
        -1.0
    );
    e.mem.set_u8(FILE + 0x40c, 0xff);
    assert_eq!(invoke(&mut e, 0x3d2d, &[]), (false, 0.0));
}
#[test]
fn retained_document_matches_simpleini_values_duplicates_and_load_order() {
    let bytes=b"\xef\xbb\xbf; top\n\n[One]\n; first key\nKey=old\nKEY=new\n\n; next section\n[Two]\nempty=\n[one]\nOther=yes\n";
    let doc = Document::parse(bytes);
    assert_eq!(doc.values(), engine_core::simple_ini::values(bytes));
    assert_eq!(doc.sections.len(), 2);
    assert_eq!(doc.sections[0].entries[0].key, b"Key");
    assert_eq!(doc.sections[0].entries[0].comment, b"; first key");
    let saved = doc.bytes();
    assert!(!saved.starts_with(b"\xef\xbb\xbf"));
    assert_eq!(Document::parse(&saved).values(), doc.values());
    assert_eq!(float_text(-0.0), b"-0.");
    assert_eq!(float_text(f64::INFINITY), b"inf");
    assert_eq!(float_text(f64::NAN), b"nan");
}
#[test]
fn thread_engine_views_share_the_same_cached_document_and_explicit_save() {
    let mut owner = engine();
    let mut peer = engine();
    owner.share_slots_for_test(&mut peer);
    nvse::table::install(&mut peer);
    // Only restore this view's command records; shared native hooks already exist.
    install(&mut peer);
    assert_eq!(
        call(
            &mut owner,
            0x3d26,
            &[string("S:One"), number(1.0), string("shared")]
        ),
        2.0
    );
    assert_eq!(get(&mut peer, "S:One", "shared", ""), b"1.");
    assert_eq!(
        call(
            &mut peer,
            0x3d3a,
            &[string("S:Two"), number(2.0), string("shared")]
        ),
        2.0
    );
    assert_eq!(get(&mut owner, "S:Two", "shared", ""), b"2");
    assert_eq!(call(&mut peer, 0x3d2d, &[string("shared")]), 1.0);
    assert_eq!(
        read(&owner, b"data/config/shared.ini").unwrap(),
        b"[S]\r\nOne = 1.\r\nTwo = 2\r\n"
    );
}
