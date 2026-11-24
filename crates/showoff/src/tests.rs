use super::*;
const DATA: u32 = 0x03000000;
const RESULT: u32 = DATA + 0x1000;
const OFFSET: u32 = RESULT + 8;
fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118e000, 0x9000);
    e.map(DATA, 0x10000);
    nvse::table::install(&mut e);
    install(&mut e);
    e
}
fn text(s: &str) -> Vec<u8> {
    [
        vec![b'S'],
        (s.len() as u16).to_le_bytes().to_vec(),
        s.as_bytes().to_vec(),
    ]
    .concat()
}
fn num(n: f64) -> Vec<u8> {
    [vec![b'Z'], n.to_le_bytes().to_vec()].concat()
}
fn call(e: &mut Engine, op: u32, tokens: &[Vec<u8>]) -> f64 {
    let mut body = vec![tokens.len() as u8];
    for t in tokens {
        body.extend_from_slice(&((t.len() + 2) as u16).to_le_bytes());
        body.extend_from_slice(t)
    }
    e.mem.set_u16(DATA, op as u16);
    e.mem.set_u16(DATA + 2, body.len() as u16);
    e.mem.write(DATA + 4, &body);
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, -999.);
    let rec = table::record_of(e, op);
    let words = [e.mem.u32(rec + 0x14), DATA, 0, 0, 0, 0, RESULT, OFFSET];
    assert_eq!(e.call(e.mem.u32(rec + 0x18), &words).eax & 0xff, 1);
    e.mem.f64(RESULT)
}
#[test]
fn real_expression_calls_read_defaults_and_close_the_same_cache() {
    let mut e = engine();
    e.ext_free::<Cache>().0.lock().unwrap().insert(
        b"aguafria.ini".to_vec(),
        simple_ini::values(b"[Stats]\niCarryWeightBase=50\nfloat=1.25junk\ninvalid=abc\nempty=\n"),
    );
    assert_eq!(
        call(
            &mut e,
            0x3d29,
            &[
                text("Stats:iCarryWeightBase"),
                text("AguaFria.ini"),
                num(150.)
            ]
        ),
        50.
    );
    assert_eq!(
        call(&mut e, 0x3d29, &[text("STATS:float"), text("AguaFria.ini")]),
        1.25
    );
    assert_eq!(
        call(
            &mut e,
            0x3d29,
            &[text("Stats:invalid"), text("AguaFria.ini"), num(7.)]
        ),
        0.
    );
    for key in ["Stats:empty", "Stats:absent", "no_separator"] {
        assert_eq!(
            call(&mut e, 0x3d29, &[text(key), text("AguaFria.ini"), num(-1.)]),
            -1.
        );
    }
    assert_eq!(call(&mut e, 0x3d2c, &[text("AguaFria.ini"), num(0.)]), 1.);
    assert_eq!(call(&mut e, 0x3d2c, &[text("AguaFria.ini"), num(0.)]), 0.);
    assert_eq!(
        call(
            &mut e,
            0x3d29,
            &[
                text("Stats:iCarryWeightBase"),
                text("AguaFria.ini"),
                num(150.)
            ]
        ),
        150.
    );
    assert_eq!(
        table::opcode_by_name(&e, b"GetINIFloat_Cached"),
        Some(0x3d29)
    );
    assert_eq!(table::opcode_by_name(&e, b"CloseFileSO"), Some(0x3d2c));
}
#[test]
fn simpleini_number_and_filename_rules_match_showoff_source() {
    assert_eq!(full_path(b"mod"), b"data\\config\\mod.ini");
    assert_eq!(
        full_path(b"folder/mod.txt"),
        b"data\\config\\folder\\mod.txt"
    );
    assert_eq!(number(Some(&vec![b'1'; 64]), 7.), 7.);
    assert_eq!(number(Some(&b"2e3; trailing".to_vec()), 7.), 2000.);
    assert_eq!(number(Some(&Vec::new()), 7.), 7.);
}

/// SO_fn_Gameplay.cpp:763: exercise ordinary ExtractArgsEx records rather than
/// the cached INI commands' expression-only fixtures.
#[test]
fn temporary_ownership_preserves_defaults_target_selection_and_save_flags() {
    let mut e = engine();
    const OP: u32 = 0x3d10;
    const SCRIPT: u32 = DATA + 0x2000;
    const REFVAR: u32 = DATA + 0x2100;
    const OWNER: u32 = DATA + 0x3000;
    const PLAYER_REF: u32 = DATA + 0x3100;
    const CALLER: u32 = DATA + 0x3200;
    const TARGET: u32 = DATA + 0x3300;
    const OWNER_REF: u32 = DATA + 0x3400;
    const VT: u32 = DATA + 0x4000;
    const IS_REF: u32 = DATA + 0x5000;
    const MARK: u32 = DATA + 0x5004;
    const SEEN: u32 = DATA + 0x6000;
    const CELL_VT: u32 = 0x0102_e9b4;
    e.map(0x011d_e000, 0x1000);
    e.map(0x0102_e000, 0x1000);
    e.mem.set_u32(0x011d_ea3c, PLAYER_REF);
    e.mem.set_u32(PLAYER_REF + 0x20, OWNER);
    for form in [OWNER, PLAYER_REF, CALLER, TARGET, OWNER_REF] {
        e.mem.set_u32(form, VT);
    }
    e.mem.set_u8(OWNER + 4, 0x2a);
    e.mem.set_u8(OWNER_REF + 4, 0x3a);
    e.mem.set_u32(OWNER_REF + 0x20, OWNER);
    e.mem.set_u32(VT + 0xf0, IS_REF);
    e.mem.set_u32(CELL_VT + 0xf0, IS_REF);
    e.mem.set_u32(VT + 0x48, MARK);
    e.mem.set_u32(CELL_VT + 0x48, MARK);
    e.register(IS_REF, |e, w| Ret {
        eax: u32::from(e.mem.u8(w[0] + 4) == 0x3a),
        ..Ret::default()
    });
    e.register(MARK, |_, _| {
        panic!("temporary ownership must not mark save changes")
    });
    e.register(0x0041_9700, |e, w| {
        assert_eq!(w.len(), 2);
        e.mem.set_u32(SEEN, e.mem.u32(SEEN) + 1);
        e.mem.set_u32(SEEN + 4, w[0]);
        e.mem.set_u32(SEEN + 8, w[1]);
        Ret::default()
    });
    let rec = table::record_of(&e, OP);
    assert_eq!(table::opcode_by_name(&e, b"SetOwnershipTemp"), Some(OP));
    assert_eq!(table::opcode_by_name(&e, b"ImGoingToBorrowThis"), Some(OP));
    assert_eq!(e.mem.u16(rec + 0x10), 0);
    assert_eq!(e.mem.u16(rec + 0x12), 2);
    assert_eq!(table::return_type(&e, OP), Returns::Default);
    for i in 0..2 {
        let p = e.mem.u32(rec + 0x14) + i * 12;
        assert_eq!(e.mem.u32(p + 4), 0x3d);
        assert_eq!(e.mem.u32(p + 8), 1);
    }
    // Script reference list: owner -> target, with count/numbering from Read.
    e.mem.set_u32(SCRIPT + 0x44, REFVAR);
    e.mem.set_u32(SCRIPT + 0x48, REFVAR + 0x20);
    e.mem.set_u32(REFVAR + 0x20, REFVAR + 0x10);
    e.mem.set_u32(REFVAR + 0x24, 0);
    e.mem.set_u32(REFVAR + 8, OWNER);
    e.mem.set_u32(REFVAR + 0x10 + 8, TARGET);
    let run = |e: &mut Engine, caller: u32, count: u16, encoded: &[u8]| {
        e.mem.set_u16(DATA, OP as u16);
        e.mem.set_u16(DATA + 2, encoded.len() as u16 + 2);
        e.mem.set_u16(DATA + 4, count);
        e.mem.write(DATA + 6, encoded);
        e.mem.set_u32(OFFSET, 4);
        e.mem.set_f64(RESULT, -999.0);
        e.mem.set_u32(SEEN, 0);
        let words = [
            e.mem.u32(rec + 0x14),
            DATA,
            caller,
            0,
            SCRIPT,
            0,
            RESULT,
            OFFSET,
        ];
        assert_eq!(e.call(e.mem.u32(rec + 0x18), &words).eax & 0xff, 1);
        e.mem.f64(RESULT)
    };
    assert_eq!(run(&mut e, CALLER, 0, &[]), 1.0);
    assert_eq!(e.mem.u32(SEEN), 1);
    assert_eq!(e.mem.u32(SEEN + 4), CALLER + 0x44);
    assert_eq!(e.mem.u32(SEEN + 8), OWNER);
    assert_eq!(run(&mut e, 0, 0, &[]), 0.0);
    assert_eq!(e.mem.u32(SEEN), 0);
    for kind in [8, 0x2a, 0x2b] {
        e.mem.set_u8(OWNER + 4, kind);
        assert_eq!(run(&mut e, CALLER, 1, &[b'r', 1, 0]), 1.0);
        assert_eq!(e.mem.u32(SEEN + 8), OWNER);
    }
    e.mem.set_u8(OWNER + 4, 7);
    assert_eq!(run(&mut e, CALLER, 1, &[b'r', 1, 0]), 0.0);
    assert_eq!(e.mem.u32(SEEN), 0);
    e.mem.set_u8(OWNER + 4, 0x2a);
    e.mem.set_u32(REFVAR + 8, 0);
    assert_eq!(run(&mut e, CALLER, 1, &[b'r', 1, 0]), 0.0);
    assert_eq!(e.mem.u32(SEEN), 0);
    assert_eq!(run(&mut e, CALLER, 1, &[b'n', 1, 0, 0, 0]), 0.0);
    e.mem.set_u32(REFVAR + 8, OWNER_REF);
    assert_eq!(run(&mut e, CALLER, 1, &[b'r', 1, 0]), 1.0);
    assert_eq!(e.mem.u32(SEEN + 8), OWNER);
    e.mem.set_u32(REFVAR + 8, OWNER);
    e.mem.set_u32(TARGET, CELL_VT);
    let both = [b'r', 1, 0, b'r', 2, 0];
    assert_eq!(run(&mut e, 0, 2, &both), 1.0);
    assert_eq!(e.mem.u32(SEEN + 4), TARGET + 0x28);
    // A CELL type byte on another class is not IS_TYPE(TESObjectCELL).
    e.mem.set_u8(TARGET + 4, 0x39);
    e.mem.set_u32(TARGET, VT);
    assert_eq!(run(&mut e, CALLER, 2, &both), 1.0);
    assert_eq!(e.mem.u32(SEEN + 4), CALLER + 0x44);
    assert_eq!(run(&mut e, 0, 2, &both), 0.0);
    assert_eq!(e.mem.u32(SEEN), 0);
}

#[test]
fn engine_events_are_opt_in_and_helpers_do_not_change_vanilla_activation() {
    use engine_core::extend::Hook;
    for on in [false, true] {
        let mut e = Engine::new();
        e.map(0x0118_e000, 0x9000);
        nvse::table::install(&mut e);
        set_events_on(&mut e, on);
        install(&mut e);
        assert_eq!(events_on(&e), on);
        assert_eq!(e.hook_users(Hook::PreActivate), u32::from(on));
        assert_eq!(
            table::opcode_by_name(&e, b"GetAddedItemRefSO").is_some(),
            on
        );
        assert!(table::opcode_by_name(&e, b"SetOwnershipTemp").is_some());
        if !on {
            assert!(e.site(0x0057_3184, &[]).is_none());
        }
    }
}
