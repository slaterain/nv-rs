use super::*;
const AREA: u32 = 0x0300_0000;
const RESULT: u32 = AREA + 0x1000;
fn setup() -> Engine {
    let mut e = Engine::new();
    e.map(AREA, 0x4000);
    e.map(0x0118_e000, 0x9000);
    nvse::table::install(&mut e);
    install(&mut e);
    e
}
fn call(e: &mut Engine, input: f64, name: &[u8], mode: i32) -> f64 {
    let payload = [
        vec![b'z'],
        input.to_le_bytes().to_vec(),
        (name.len() as u16).to_le_bytes().to_vec(),
        name.to_vec(),
        vec![b'n'],
        mode.to_le_bytes().to_vec(),
    ]
    .concat();
    e.mem.set_u16(AREA, OP as u16);
    e.mem.set_u16(AREA + 2, (payload.len() + 2) as u16);
    e.mem.set_u16(AREA + 4, 3);
    e.mem.write(AREA + 6, &payload);
    e.mem.set_u32(RESULT + 8, 4);
    let r = table::record_of(e, OP);
    let words = [e.mem.u32(r + 0x14), AREA, 0, 0, 0, 0, RESULT, RESULT + 8];
    e.call(e.mem.u32(r + 0x18), &words);
    e.mem.f64(RESULT)
}
#[test]
fn actual_double_string_integer_contract_prefix_selection_and_invalid_input() {
    let mut e = setup();
    let r = table::record_of(&e, OP);
    assert_eq!(table::guest_text(&e, e.mem.u32(r + 4)), b"Ease");
    let p = e.mem.u32(r + 0x14);
    assert_eq!(e.mem.u16(r + 0x12), 3);
    for (i, kind) in [0x2c, 0, 1].into_iter().enumerate() {
        assert_eq!(e.mem.u32(p + i as u32 * 12 + 4), kind);
        assert_eq!(e.mem.u32(p + i as u32 * 12 + 8), 0);
    }
    assert_eq!(call(&mut e, 0.25, b"QuAdRaTiC", 0), 0.0625);
    assert_eq!(call(&mut e, 0.25, b"unknown", 0), -1.0);
    assert_eq!(call(&mut e, 0.25, b"quad", -1), -1.0);
    assert_eq!(call(&mut e, 0.25, b"quad", 3), -1.0);
    assert_eq!(call(&mut e, 2.0, b"quad", 0), 4.0, "no global input clamp");
}
#[test]
fn all_thirty_curves_retain_endpoints_and_source_halfway_anchors() {
    for family in 0..10 {
        for mode in 0..3 {
            assert!((ease(0.0, family, mode) - 0.0).abs() < 1e-14);
            assert!((ease(1.0, family, mode) - 1.0).abs() < 1e-14);
            assert!(ease(0.25, family, mode).is_finite());
        }
    }
    assert_eq!(ease(0.5, 1, 0), 0.25);
    assert_eq!(ease(0.5, 2, 0), 0.125);
    assert_eq!(ease(0.5, 3, 0), 0.0625);
    assert_eq!(ease(0.5, 4, 0), 0.03125);
    assert_eq!(ease(0.5, 6, 0), 0.03125);
    assert!((ease(0.5, 0, 0) - (1.0 - std::f64::consts::FRAC_1_SQRT_2)).abs() < 1e-15);
    assert!((ease(0.5, 9, 1) - 0.765625).abs() < 1e-15);
}
#[test]
fn source_circ_bounds_nan_and_back_elastic_overshoot_are_not_normalized_away() {
    assert_eq!(ease(2.0, 5, 0), 1.0);
    assert_eq!(ease(-1.0, 5, 1), 0.0);
    assert_eq!(ease(-1.0, 5, 2), 0.0);
    assert!(ease(f64::NAN, 5, 0).is_nan());
    assert!(ease(f64::NAN, 5, 1).is_nan());
    assert!(ease(0.25, 7, 0) < 0.0);
    assert!(ease(0.5, 8, 1) > 1.0);
}

#[test]
fn independent_original_header_equations_match_sixty_reference_samples() {
    // Originalf45d069 easing.h evaluated by a private C++20 harness. Clang
    // rejects the unusedvirtualbase/staticderiveddeclarations; the harness
    // removed only10 inheritanceclauses, leaving every equation unchanged.
    // This is equation-level comparison, not WindowsDLL/CRTbitidentity proof.
    const REFERENCE: [(usize, u32, f64, u64); 60] = [
        (0, 0, 0.25, 0x3fb37ca1866b95d0),
        (0, 0, 0.75, 0x3fe3c10eaca8ab4e),
        (0, 1, 0.25, 0x3fd87de2a6aea963),
        (0, 1, 0.75, 0x3fed906bcf328d46),
        (0, 2, 0.25, 0x3fc2bec333018866),
        (0, 2, 0.75, 0x3feb504f333f9de6),
        (1, 0, 0.25, 0x3fb0000000000000),
        (1, 0, 0.75, 0x3fe2000000000000),
        (1, 1, 0.25, 0x3fdc000000000000),
        (1, 1, 0.75, 0x3fee000000000000),
        (1, 2, 0.25, 0x3fc0000000000000),
        (1, 2, 0.75, 0x3fec000000000000),
        (2, 0, 0.25, 0x3f90000000000000),
        (2, 0, 0.75, 0x3fdb000000000000),
        (2, 1, 0.25, 0x3fe2800000000000),
        (2, 1, 0.75, 0x3fef800000000000),
        (2, 2, 0.25, 0x3fb0000000000000),
        (2, 2, 0.75, 0x3fee000000000000),
        (3, 0, 0.25, 0x3f70000000000000),
        (3, 0, 0.75, 0x3fd4400000000000),
        (3, 1, 0.25, 0x3fe5e00000000000),
        (3, 1, 0.75, 0x3fefe00000000000),
        (3, 2, 0.25, 0x3fa0000000000000),
        (3, 2, 0.75, 0x3fef000000000000),
        (4, 0, 0.25, 0x3f50000000000000),
        (4, 0, 0.75, 0x3fce600000000000),
        (4, 1, 0.25, 0x3fe8680000000000),
        (4, 1, 0.75, 0x3feff80000000000),
        (4, 2, 0.25, 0x3f90000000000000),
        (4, 2, 0.75, 0x3fef800000000000),
        (5, 0, 0.25, 0x3fa04214eb0b1260),
        (5, 0, 0.75, 0x3fd5ab00ac5a0e2c),
        (5, 1, 0.25, 0x3fe52a7fa9d2f8ea),
        (5, 1, 0.75, 0x3feefbdeb14f4eda),
        (5, 2, 0.25, 0x3fb126145e9ecd58),
        (5, 2, 0.75, 0x3feddb3d742c2655),
        (6, 0, 0.25, 0x3f76a09e667f3bcd),
        (6, 0, 0.75, 0x3fc6a09e667f3bcd),
        (6, 1, 0.25, 0x3fea57d86660310d),
        (6, 1, 0.75, 0x3fefd2bec3330188),
        (6, 2, 0.25, 0x3f90000000000000),
        (6, 2, 0.75, 0x3fef800000000000),
        (7, 0, 0.25, 0xbfb06b40f66a5509),
        (7, 0, 0.75, 0x3fc75f1e8e60806c),
        (7, 1, 0.25, 0x3fea28385c67dfe4),
        (7, 1, 0.75, 0x3ff106b40f66a551),
        (7, 2, 0.25, 0xbfb984bfd2e94680),
        (7, 2, 0.75, 0x3ff1984bfd2e9468),
        (8, 0, 0.25, 0xbf76a09e667f3bcd),
        (8, 0, 0.75, 0x3fb6a09e667f3bc4),
        (8, 1, 0.25, 0x3fed2bec33301887),
        (8, 1, 0.75, 0x3ff016a09e667f3c),
        (8, 2, 0.25, 0x3f88836fa2cf5040),
        (8, 2, 0.75, 0x3fef9df24174c2bf),
        (9, 0, 0.25, 0x3f9c000000000000),
        (9, 0, 0.75, 0x3fe0e00000000000),
        (9, 1, 0.25, 0x3fde400000000000),
        (9, 1, 0.75, 0x3fef200000000000),
        (9, 2, 0.25, 0x3fbe000000000000),
        (9, 2, 0.75, 0x3fec400000000000),
    ];
    for (family, mode, x, want) in REFERENCE {
        let actual = ease(x, family, mode);
        let want = f64::from_bits(want);
        assert!(
            (actual - want).abs() <= 1e-13 * want.abs().max(1.0),
            "family{family} mode{mode} x{x}: {actual} != {want}"
        );
    }
}
