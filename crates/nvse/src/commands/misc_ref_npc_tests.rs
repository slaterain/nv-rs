//! Tests of the batch through the commands' records, on forms laid out where the test's cast puts
//! their components.

use super::super::test_support::*;
use super::*;

const PLAYER_NPC: u32 = 0x0200_1000;
const DOC_NPC: u32 = 0x0200_2000;
const DOC: u32 = 0x0200_3000;
const EYES: u32 = 0x0200_4000;
const HAIR: u32 = 0x0200_4800;
const DOC_EYES: u32 = 0x0200_5000;
const DOC_HAIR: u32 = 0x0200_5800;

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, this, args);
    assert_eq!(ok, 1);
    result
}

fn form_of(v: f64) -> u32 {
    v.to_bits() as u32
}

/// The player's base NPC and Doc Mitchell's as the real game read them (run `inv6r1`, after
/// `nvsecos4` loads): the player's eyes 00004252 (flags 1), hair 0002BFDB (flags 5), hair length
/// 0.98, hair colour 0x36495C, weight 0, height 1; Doc's eyes 00004253 and hair 0002A185, a
/// reference to Doc. The script's references 1-6 name the player's NPC, Doc's NPC, Doc, the
/// player's eyes and hair, and Doc's eyes.
fn setup() -> Engine {
    let mut e = engine_with_table();
    for (form, kind, id) in [
        (PLAYER_NPC, 0x2a, 0x0000_0007),
        (DOC_NPC, 0x2a, 0x0010_4c0e),
        (DOC, 0x3b, 0x0010_4c0f),
        (EYES, 0x0b, 0x0000_4252),
        (HAIR, 0x0a, 0x0002_bfdb),
        (DOC_EYES, 0x0b, 0x0000_4253),
        (DOC_HAIR, 0x0a, 0x0002_a185),
    ] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, id);
    }
    for npc in [PLAYER_NPC, DOC_NPC] {
        e.mem.set_u32(npc + 0x7f0, 1 << 29);
    }
    e.mem.set_u32(EYES + 0x7f0, 1 << 30);
    e.mem.set_u32(DOC_EYES + 0x7f0, 1 << 30);
    e.mem.set_u32(HAIR + 0x7f0, 1 << 31);
    e.mem.set_u32(DOC_HAIR + 0x7f0, 1 << 31);
    e.mem.set_u8(EYES + 0x30, 1);
    e.mem.set_u8(HAIR + 0x48, 5);
    e.mem.set_u32(PLAYER_NPC + 0x1c0, EYES);
    e.mem.set_u32(PLAYER_NPC + 0x1b8, HAIR);
    e.mem.set_f32(PLAYER_NPC + 0x1bc, 0.98);
    e.mem.set_u32(PLAYER_NPC + 0x1d8, 0x0036_495c);
    e.mem.set_f32(PLAYER_NPC + 0x1f8, 0.0);
    e.mem.set_f32(PLAYER_NPC + 0x1f4, 1.0);
    e.mem.set_u32(DOC_NPC + 0x1c0, DOC_EYES);
    e.mem.set_u32(DOC_NPC + 0x1b8, DOC_HAIR);
    e.mem.set_f32(DOC_NPC + 0x1f4, 1.0);
    e.mem.set_u32(DOC + 0x7f0, 1 | (1 << 21));
    e.mem.set_u32(DOC + 0x20, DOC_NPC);
    for (i, form) in [PLAYER_NPC, DOC_NPC, DOC, EYES, HAIR, DOC_EYES]
        .into_iter()
        .enumerate()
    {
        set_script_ref(&mut e, i as u16 + 1, form);
    }
    e
}

/// The getters on the player's NPC as the real game answered them (run `inv6r1`): eyes, hair,
/// hair length, the hair colour's word and its three bytes (code 1 the lowest), weight and
/// height; on the calling reference's base form when no NPC is given; 0 for a reference whose
/// base is no NPC.
#[test]
fn the_real_games_answers_for_the_getters() {
    let mut e = setup();
    assert_eq!(form_of(run(&mut e, 0x1522, 0, &[r(1)])), 0x0000_4252);
    assert_eq!(form_of(run(&mut e, 0x1524, 0, &[r(1)])), 0x0002_bfdb);
    assert_eq!(run(&mut e, 0x1525, 0, &[r(1)]), f64::from(0.98f32));
    let colors: Vec<f64> = (0..4)
        .map(|code| run(&mut e, 0x1527, 0, &[n(code), r(1)]))
        .collect();
    assert_eq!(colors, [3_557_724.0, 92.0, 73.0, 54.0]);
    assert_eq!(run(&mut e, 0x1529, 0, &[r(1)]), 0.0);
    assert_eq!(run(&mut e, 0x152b, 0, &[r(1)]), 1.0);
    // Doc, through the calling reference.
    assert_eq!(form_of(run(&mut e, 0x1522, DOC, &[])), 0x0000_4253);
    assert_eq!(form_of(run(&mut e, 0x1524, DOC, &[])), 0x0002_a185);
    assert_eq!(run(&mut e, 0x152b, DOC, &[]), 1.0);
    assert_eq!(run(&mut e, 0x1522, 0, &[]), 0.0);
    e.mem.set_u32(DOC + 0x20, EYES);
    assert_eq!(run(&mut e, 0x1522, DOC, &[]), 0.0, "a base that is no NPC");
}

/// The flags of eyes and hair (run `inv6r1`): the player's eyes 1 (masked with 1: 1), the hair 5;
/// `SetEyesFlags 6` reads 6, `SetEyesFlags 255` is refused (still 6), as is no mask; another
/// form answers 0.
#[test]
fn eyes_and_hair_flags() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x1600, 0, &[r(4)]), 1.0);
    assert_eq!(run(&mut e, 0x1600, 0, &[r(4), n(1)]), 1.0);
    assert_eq!(run(&mut e, 0x1600, 0, &[r(4), n(2)]), 0.0);
    assert_eq!(run(&mut e, 0x1602, 0, &[r(5)]), 5.0);
    assert_eq!(run(&mut e, 0x1602, 0, &[r(5), n(4)]), 4.0);
    run(&mut e, 0x1601, 0, &[r(4), n(6)]);
    assert_eq!(run(&mut e, 0x1600, 0, &[r(4)]), 6.0);
    run(&mut e, 0x1601, 0, &[r(4), n(255)]);
    run(&mut e, 0x1601, 0, &[r(4)]);
    assert_eq!(e.mem.u8(EYES + 0x30), 6);
    run(&mut e, 0x1603, 0, &[r(5), n(0xfe)]);
    assert_eq!(e.mem.u8(HAIR + 0x48), 0xfe);
    assert_eq!(run(&mut e, 0x1600, 0, &[r(5)]), 0.0, "hair is no eyes");
    run(&mut e, 0x1603, 0, &[r(4), n(3)]);
    assert_eq!(e.mem.u8(EYES + 0x30 + 0x18), 0, "eyes are no hair");
}

/// The setters on Doc's NPC as the real game answered them (run `inv6r1`): `SetHairLength 0.25`
/// answers 1 and reads 0.25; `SetHairColor 16909060` (0x01020304) answers 1 and reads 131844
/// (0x020304); `SetNPCWeight 42.5` and, on the calling reference, `SetNPCHeight 1.125` answer 1;
/// `SetEyes` with the player's eyes answers 1 and with an NPC form 0; `SetHair` on the calling
/// reference with the player's hair answers 1.
#[test]
fn the_real_games_answers_for_the_setters() {
    let mut e = setup();
    assert_eq!(run(&mut e, 0x1526, 0, &[z(0.25), r(2)]), 1.0);
    assert_eq!(run(&mut e, 0x1525, 0, &[r(2)]), 0.25);
    assert_eq!(run(&mut e, 0x1528, 0, &[n(16_909_060), r(2)]), 1.0);
    assert_eq!(run(&mut e, 0x1527, 0, &[n(0), r(2)]), 131_844.0);
    assert_eq!(run(&mut e, 0x152a, 0, &[z(42.5), r(2)]), 1.0);
    assert_eq!(run(&mut e, 0x1529, 0, &[r(2)]), 42.5);
    assert_eq!(run(&mut e, 0x152c, DOC, &[z(1.125)]), 1.0);
    assert_eq!(run(&mut e, 0x152b, 0, &[r(2)]), 1.125);
    assert_eq!(run(&mut e, 0x1521, 0, &[r(4), r(2)]), 1.0);
    assert_eq!(form_of(run(&mut e, 0x1522, 0, &[r(2)])), 0x0000_4252);
    assert_eq!(
        run(&mut e, 0x1521, 0, &[r(1), r(2)]),
        0.0,
        "an NPC is no eyes"
    );
    assert_eq!(run(&mut e, 0x1523, DOC, &[r(5)]), 1.0);
    assert_eq!(form_of(run(&mut e, 0x1524, DOC, &[])), 0x0002_bfdb);
    // No NPC to set: no calling reference, or one whose base is no NPC.
    assert_eq!(run(&mut e, 0x1526, 0, &[z(0.5)]), 0.0);
    e.mem.set_u32(DOC + 0x20, EYES);
    assert_eq!(run(&mut e, 0x1526, DOC, &[z(0.5)]), 0.0);
    assert_eq!(e.mem.f32(EYES + 0x1bc), 0.0);
}

/// The parameter names are xNVSE's (`Commands_MiscRef.h`): the script compiler's messages and
/// plugins reading the command table see them. `SetNPCHeight` takes `SetNPCWeight`'s list, as
/// xNVSE registers it.
#[test]
fn parameter_names_are_xnvses() {
    let names = |opcode: u32| -> Vec<&str> {
        let command = COMMANDS.iter().find(|c| c.opcode == opcode).unwrap();
        command.params.iter().map(|p| p.name).collect()
    };
    assert_eq!(names(0x1521), ["body data", "base NPC"]);
    assert_eq!(names(0x1522), ["NPC"]);
    assert_eq!(names(0x1526), ["hair length", "base NPC"]);
    assert_eq!(names(0x1527), ["hair color code", "base NPC"]);
    assert_eq!(names(0x1528), ["hair color", "base NPC"]);
    assert_eq!(names(0x152a), ["weight", "base NPC"]);
    assert_eq!(names(0x152c), ["weight", "base NPC"]);
    assert_eq!(names(0x1600), ["eyes", "mask"]);
    assert_eq!(names(0x1603), ["hair", "mask"]);
}
