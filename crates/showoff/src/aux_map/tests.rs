use super::*;
const DATA: u32 = 0x0300_0000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
const SCRIPT: u32 = DATA + 0x2000;
const FILE: u32 = DATA + 0x3000;
const FORM: u32 = DATA + 0x4000;

fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(DATA, 0x10000);
    nvse::table::install(&mut e);
    install(&mut e);
    e.register(0x00ec_a7f4, |_, a| Ret {
        eax: u32::from((a[0] as u8).to_ascii_uppercase()),
        ..Ret::default()
    });
    e.mem.set_u32(SCRIPT + 0xc, 0x0100_1234);
    e.mem.set_u32(SCRIPT + 0x10, FILE);
    e.mem.set_u8(FILE + 0x40c, 2);
    e.mem.set_u32(FORM + 0xc, 0x0200_3456);
    e.register(LOOKUP_FORM, |_, a| Ret {
        eax: if a[0] == 0x0200_3456 { FORM } else { 0 },
        ..Ret::default()
    });
    e
}
fn text(s: &str) -> Vec<u8> {
    [
        (s.len() as u16).to_le_bytes().to_vec(),
        s.as_bytes().to_vec(),
    ]
    .concat()
}
fn integer(n: u32) -> Vec<u8> {
    [vec![b'n'], n.to_le_bytes().to_vec()].concat()
}
fn double(n: f64) -> Vec<u8> {
    [vec![b'z'], n.to_le_bytes().to_vec()].concat()
}
fn command(e: &mut Engine, op: u32, args: &[Vec<u8>]) -> f64 {
    let bytes = args.concat();
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, (bytes.len() + 2) as u16);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    e.mem.write(DATA + 6, &bytes);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 1234.5);
    let rec = table::record_of(e, op);
    let words = [e.mem.u32(rec + 0x14), DATA, 0, 0, SCRIPT, 0, RESULT, OFFSET];
    assert!(e.call(e.mem.u32(rec + 0x18), &words).bool());
    e.mem.f64(RESULT)
}
fn set(e: &mut Engine, name: &str, key: &str, n: f64) {
    assert_eq!(
        command(e, 0x3cbb, &[text(name), text(key), double(n)]),
        1234.5
    );
}
fn get(e: &mut Engine, name: &str, key: &str) -> f64 {
    command(e, 0x3cb2, &[text(name), text(key)])
}

#[test]
fn real_records_aliases_and_typed_parameter_contracts() {
    let e = engine();
    for (op, alias, types, returns) in [
        (
            0x3cb2,
            "AuxStringMapGetFlt",
            &[(0, false), (0, false)][..],
            Returns::Default,
        ),
        (
            0x3cb4,
            "AuxStringMapGetStr",
            &[(0, false), (0, false)][..],
            Returns::String,
        ),
        (
            0x3cb9,
            "AuxStringMapGetAsArr",
            &[(0, false)][..],
            Returns::Array,
        ),
        (
            0x3cba,
            "AuxStringMapSetFromArr",
            &[(0, false), (1, false), (1, true)][..],
            Returns::Default,
        ),
        (
            0x3cbb,
            "AuxStringMapSetFlt",
            &[(0, false), (0, false), (0x2c, false)][..],
            Returns::Default,
        ),
        (
            0x3cbe,
            "AuxStringMapEraseKey",
            &[(0, false), (0, false)][..],
            Returns::Default,
        ),
        (
            0x3cc0,
            "AuxStringMapDestroy",
            &[(0, false)][..],
            Returns::Default,
        ),
    ] {
        let rec = table::record_of(&e, op);
        assert_ne!(rec, 0);
        assert_eq!(table::opcode_by_name(&e, alias.as_bytes()), Some(op));
        assert_eq!(e.mem.u16(rec + 0x10), 0);
        assert_eq!(e.mem.u16(rec + 0x12), types.len() as u16);
        assert_eq!(table::return_type(&e, op), returns);
        let params = e.mem.u32(rec + 0x14);
        for (i, (kind, optional)) in types.iter().enumerate() {
            assert_eq!(e.mem.u32(params + 12 * i as u32 + 4), *kind);
            assert_eq!(e.mem.u32(params + 12 * i as u32 + 8), u32::from(*optional));
        }
    }
    assert!(
        !plugins::built_ins(&e)
            .iter()
            .find(|p| p.opcode_base == 0x3c93)
            .unwrap()
            .listed
    );
    assert_eq!(table::opcode_by_name(&e, b"ReadFromJSON"), None);
    assert_eq!(table::opcode_by_name(&e, b"AuxStringMapGetRef"), None);
}
#[test]
fn scope_uses_override_mod_prefixes_case_and_exact_double() {
    let mut e = engine();
    let precise = 1.23456789012345;
    set(&mut e, "*private", "MixedKey", precise);
    assert_eq!(get(&mut e, "*PRIVATE", "mixedkey"), precise);
    assert!(!state(&mut e).changed);
    e.mem.set_u8(FILE + 0x40c, 3);
    assert_eq!(get(&mut e, "*private", "MixedKey"), 0.0);
    set(&mut e, "*_shared", "key", 5.0);
    e.mem.set_u8(FILE + 0x40c, 4);
    assert_eq!(get(&mut e, "*_shared", "key"), 5.0);
    set(&mut e, "_permanent", "key", 7.0);
    assert!(state(&mut e).changed);
    set(&mut e, "", "key", 8.0);
    set(&mut e, "*private", "", 8.0);
    assert_eq!(get(&mut e, "", "key"), 0.0);
    assert_eq!(state(&mut e).maps.len(), 3);
}
#[test]
fn array_copy_is_independent_resolves_forms_and_keeps_only_string_keys() {
    let mut e = engine();
    let id = vars::create_array(&mut e, vars::KEY_STRING, false, 1);
    let inner = vars::create_array(&mut e, vars::KEY_NUMBER, true, 1);
    for (key, value) in [
        ("n", Value::Number(9.25)),
        ("str", Value::Str(b"hello".to_vec())),
        ("ref", Value::Form(0x0200_3456)),
        ("missing", Value::Form(0x0200_9999)),
        ("arr", Value::Array(inner)),
    ] {
        assert!(vars::set_element(
            &mut e,
            id,
            &Key::Str(key.as_bytes().to_vec()),
            value
        ));
    }
    assert_eq!(
        command(&mut e, 0x3cba, &[text("*values"), integer(id)]),
        0.0
    );
    vars::set_element(&mut e, id, &Key::Str(b"n".to_vec()), Value::Number(0.0));
    assert_eq!(get(&mut e, "*values", "n"), 9.25);
    let string = command(&mut e, 0x3cb4, &[text("*values"), text("str")]) as u32;
    assert_eq!(vars::string(&e, string), Some(b"hello".to_vec()));
    let result = command(&mut e, 0x3cb9, &[text("*values")]) as u32;
    let Container::StrMap(entries) = &vars::array(&e, result).unwrap().elements else {
        panic!()
    };
    let value = |name: &[u8]| {
        entries
            .iter()
            .find(|(k, _)| k == name)
            .unwrap()
            .1
            .data
            .clone()
    };
    assert_eq!(value(b"ref"), Data::Form(0x0200_3456));
    assert_eq!(value(b"missing"), Data::Form(0));
    assert_eq!(
        value(b"arr"),
        Data::Number(f64::from_bits(u64::from(inner)))
    );
    assert_eq!(
        vars::array(&e, result).unwrap().owning_mod,
        1,
        "array owner is script base mod, not override mod"
    );
    command(
        &mut e,
        0x3cba,
        &[text("*values"), integer(inner), integer(1)],
    );
    assert_eq!(
        get(&mut e, "*values", "n"),
        9.25,
        "append of numeric array leaves old data"
    );
    command(&mut e, 0x3cba, &[text("*values"), integer(0)]);
    assert_eq!(
        command(&mut e, 0x3cb9, &[text("*values")]),
        0.0,
        "clear precedes invalid array lookup"
    );
}
#[test]
fn erase_destroy_and_invalid_reads_match_result_defaults() {
    let mut e = engine();
    set(&mut e, "*a", "one", 1.0);
    set(&mut e, "*a", "two", 2.0);
    assert_eq!(command(&mut e, 0x3cbe, &[text("*a"), text("one")]), 1.0);
    assert_eq!(command(&mut e, 0x3cbe, &[text("*a"), text("absent")]), 0.0);
    assert_eq!(command(&mut e, 0x3cbe, &[text("*a"), text("two")]), 0.0);
    assert_eq!(command(&mut e, 0x3cb9, &[text("*a")]), 0.0);
    set(&mut e, "*a", "three", 3.0);
    assert_eq!(command(&mut e, 0x3cc0, &[text("*a")]), 1234.5);
    // A double beginning with an invalid numeric token makes ExtractArgsEx fail.
    assert_eq!(
        command(&mut e, 0x3cbb, &[text("*a"), text("x"), vec![0]]),
        1234.5
    );
    assert_eq!(get(&mut e, "*a", "x"), 0.0);
    let empty = command(&mut e, 0x3cb4, &[text("absent"), text("x")]) as u32;
    assert_eq!(vars::string(&e, empty), Some(Vec::new()));
}

fn load_records(e: &mut Engine, records: Records) {
    let has_records = !records.chunks.is_empty();
    let bytes = cosave::write(&[cosave::Plugin {
        opcode_base: 0x3c93,
        chunks: records.chunks,
    }]);
    let mut reader = Reader::open(&bytes).unwrap();
    if has_records {
        reader.next_plugin().unwrap().unwrap();
    } else {
        reader.no_records();
    }
    load(e, &mut reader).unwrap();
}
fn notify(e: &mut Engine, kind: u32, path: &str) {
    let address = table::guest_str(e, path.as_bytes());
    message(
        e,
        &Message {
            kind,
            data: address,
            len: path.len() as u32,
        },
    );
}
#[test]
fn permanent_smso_roundtrip_remaps_mods_and_forms_and_preserves_other_records() {
    let mut e = engine();
    set(&mut e, "saved", "n", 3.25);
    set(&mut e, "*session", "n", 8.0);
    let at = scope(&e, SCRIPT, b"saved");
    put(&mut e, at, b"saved", b"form", Scalar::Form(0x0200_3456));
    put(&mut e, at, b"saved", b"str", text_value(b"text"));
    state(&mut e).other.push(Chunk {
        kind: cosave::tag(b"TAOS"),
        version: 1,
        data: vec![1, 2, 3],
    });
    let mut records = Records::default();
    save(&mut e, &mut records);
    assert_eq!(records.chunks[0].kind, RECORD);
    assert_eq!(records.chunks[0].version, 10);
    assert_eq!(records.chunks[0].data[0..3], [1, 0, 2]);
    e.ext::<nvse::Nvse>().saveload.preload_mods = vec![0, 1, 5];
    e.mem.set_u8(FILE + 0x40c, 5);
    state(&mut e).changed = true;
    load_records(&mut e, records.clone());
    assert_eq!(get(&mut e, "saved", "n"), 3.25);
    assert_eq!(
        find(&mut e, SCRIPT, b"saved", b"form"),
        Some(Scalar::Form(0x0500_3456))
    );
    assert_eq!(
        find(&mut e, SCRIPT, b"saved", b"str"),
        Some(text_value(b"text"))
    );
    assert_eq!(state(&mut e).maps.len(), 2, "session map survives load");
    assert_eq!(state(&mut e).other, records.chunks[1..]);
    // An owner that was uninstalled is consumed and dropped, not loaded as FF.
    e.ext::<nvse::Nvse>().saveload.preload_mods[2] = 0xff;
    state(&mut e).changed = true;
    load_records(&mut e, records);
    assert_eq!(get(&mut e, "saved", "n"), 0.0);
    assert_eq!(state(&mut e).maps.len(), 1);
}
#[test]
fn absent_saved_owner_does_not_misalign_the_following_shared_map() {
    let mut e = engine();
    set(&mut e, "private", "n", 3.0);
    set(&mut e, "_shared", "n", 4.0);
    let mut records = Records::default();
    save(&mut e, &mut records);
    e.ext::<nvse::Nvse>().saveload.preload_mods = vec![0, 1, 0xff];
    state(&mut e).changed = true;
    load_records(&mut e, records);
    assert_eq!(get(&mut e, "private", "n"), 0.0);
    assert_eq!(get(&mut e, "_shared", "n"), 4.0);
}
#[test]
fn save_load_changed_flags_and_session_lifetime_follow_source_messages() {
    let mut e = engine();
    set(&mut e, "saved", "x", 1.0);
    set(&mut e, "*session", "x", 2.0);
    notify(&mut e, plugins::msg::SAVE_GAME, "Save1.fos");
    assert!(!state(&mut e).changed);
    notify(&mut e, plugins::msg::PRE_LOAD_GAME, "Save1.fos");
    assert!(!state(&mut e).changed);
    let mut saved = Records::default();
    save(&mut e, &mut saved);
    // Same-path load with unchanged data skips the SMSO record, retaining data.
    load_records(&mut e, saved);
    assert_eq!(get(&mut e, "saved", "x"), 1.0);
    notify(&mut e, plugins::msg::PRE_LOAD_GAME, "Save2.fos");
    assert!(state(&mut e).changed);
    load_records(&mut e, Records::default());
    assert_eq!(get(&mut e, "saved", "x"), 0.0);
    message(
        &mut e,
        &Message {
            kind: plugins::msg::POST_LOAD_GAME,
            data: 1,
            len: 1,
        },
    );
    assert!(!state(&mut e).changed);
    message(
        &mut e,
        &Message {
            kind: plugins::msg::EXIT_TO_MAIN_MENU,
            data: 0,
            len: 0,
        },
    );
    assert_eq!(get(&mut e, "*session", "x"), 2.0);
}

#[test]
fn missing_cosave_drops_only_the_ports_opaque_records() {
    let mut e = engine();
    set(&mut e, "saved", "x", 1.0);
    set(&mut e, "*session", "x", 2.0);
    let opaque = Chunk {
        kind: cosave::tag(b"TAOS"),
        version: 1,
        data: vec![1, 2, 3],
    };
    state(&mut e).other.push(opaque.clone());
    let mut before = Records::default();
    save(&mut e, &mut before);
    assert_eq!(before.chunks[1], opaque);

    let name = table::guest_str(&mut e, b"Saves\\Save2.fos");
    e.register(0x00fd_f088, |e, args| {
        assert_eq!(table::guest_text(e, args[0]), b"Saves\\Save2.nvse");
        Ret {
            eax: u32::MAX,
            ..Ret::default()
        }
    });
    nvse::saveload::pre_load(&mut e, name);
    nvse::saveload::load(&mut e, name);

    let mut after = Records::default();
    save(&mut e, &mut after);
    assert_eq!(after.chunks, before.chunks[..1]);
    assert_eq!(get(&mut e, "saved", "x"), 1.0);
    assert_eq!(get(&mut e, "*session", "x"), 2.0);
}

#[test]
fn real_thread_engine_views_share_mutation_and_cosave_without_held_guards() {
    let mut owner = engine();
    let mut peer = engine();
    owner.share_slots_for_test(&mut peer);
    // Ordinary NVSE per-view state was not copied by for_thread. Restore the
    // real table/registrations in this view without replacing the free Store.
    nvse::table::install(&mut peer);
    install(&mut peer);
    set(&mut owner, "shared-by-thread", "owner", 1.0);
    assert_eq!(get(&mut peer, "shared-by-thread", "owner"), 1.0);
    set(&mut peer, "shared-by-thread", "peer", 2.5);
    assert_eq!(get(&mut owner, "shared-by-thread", "peer"), 2.5);
    let at = scope(&owner, SCRIPT, b"shared-by-thread");
    put(
        &mut owner,
        at,
        b"shared-by-thread",
        b"form",
        Scalar::Form(0x0200_3456),
    );
    // GetAsArray must release its snapshot lock before invoking form lookup.
    peer.register(LOOKUP_FORM, |e, _| {
        assert!(e.ext_free::<Store>().0.try_lock().is_ok());
        Ret {
            eax: FORM,
            ..Ret::default()
        }
    });
    assert!(command(&mut peer, 0x3cb9, &[text("shared-by-thread")]) > 0.0);
    let mut a = Records::default();
    let mut b = Records::default();
    save(&mut owner, &mut a);
    save(&mut peer, &mut b);
    assert_eq!(a, b, "co-save sees all mutations from either engine view");
    let mut loaded = engine();
    loaded.ext::<nvse::Nvse>().saveload.preload_mods = vec![0, 1, 2];
    state(&mut loaded).changed = true;
    load_records(&mut loaded, a);
    assert_eq!(get(&mut loaded, "shared-by-thread", "peer"), 2.5);
}
