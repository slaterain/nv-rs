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
