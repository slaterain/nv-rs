//! Share 1b's crash rows (forms, cells, scripts, saves and sound): each row's crashing input fed
//! to its translated function, row on and off, as in `stewie_fixes.rs`. Only share 1b edits this
//! file.

use super::{ret, LOCK};
use engine_core::extend::stewie;
use engine_core::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// `PlaceOcclusionPlane`'s command (`005bb9e0`) and the calls it makes that stand in here: the
/// parameters' parse, the 2D point's constructor, the occluder list's owner and list, the world's
/// animation matrix, the rotation and product, the plane's block and constructor, the list's add.
const PLACE_OCCLUSION_PLANE: u32 = 0x005b_b9e0;
const PARSE_PARAMETERS: u32 = 0x005a_ccb0;
const NI_POINT2_CONSTRUCT: u32 = 0x0045_2dc0;
const LIST_OWNER: u32 = 0x0045_0b80;
const OWNER_LIST: u32 = 0x0045_4b30;
const WORLD: u32 = 0x0045_c670;
const ANIMATION: u32 = 0x0066_29f0;
const ANIMATION_MATRIX: u32 = 0x0046_1130;
const MATRIX_FROM_Z_ANGLE: u32 = 0x004a_0c90;
const MATRIX_MULTIPLY: u32 = 0x0043_f8d0;
const NI_ALLOC: u32 = 0x00aa_13e0;
const PLANE_CONSTRUCT: u32 = 0x004a_5ff0;
const LIST_ADD: u32 = 0x00c5_ae70;
const INTERLOCKED_INCREMENT: u32 = 0x00fd_f1c0;
const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// The plane's deleting destructor (its vtable's slot +4, which the release calls at a count of 0).
const PLANE_DELETE: u32 = 0x00ff_5000;

static PLANE: AtomicU32 = AtomicU32::new(0);
static OWNER: AtomicU32 = AtomicU32::new(0);
static LIST: AtomicU32 = AtomicU32::new(0);
static SOURCE: AtomicU32 = AtomicU32::new(0);
static ADDED: AtomicU32 = AtomicU32::new(0);
static FREED: AtomicU32 = AtomicU32::new(0);

/// The command's world: a plane object (vtable, count 0 at `+4`) the block allocation hands out,
/// the occluder list whose add is recorded; the `NiPointer` (`00633c90`, `0045cec0`) and the
/// reference count's add and release (`0040f6e0`, `00401970`) run translated.
fn occlusion_world() -> Engine {
    let mut e = Engine::new();
    for page in [0x0101_2000u32, 0x0101_b000, 0x0103_9000] {
        e.map(page, 0x1000);
    }
    e.mem.set_f32(0x0101_2054, -1.0);
    e.mem.set_f32(0x0101_b268, 50.0);
    let vtable = e.mem.alloc(0x40);
    e.mem.set_u32(vtable + 4, PLANE_DELETE);
    let plane = e.mem.alloc(0x100);
    e.mem.set_u32(plane, vtable);
    PLANE.store(plane, Relaxed);
    // The list's owner: the command reads its position at `+0x1f0` (`005bb9b0`).
    OWNER.store(e.mem.alloc(0x200), Relaxed);
    LIST.store(e.mem.alloc(0x40), Relaxed);
    SOURCE.store(e.mem.alloc(0x40), Relaxed);
    ADDED.store(0, Relaxed);
    FREED.store(0, Relaxed);
    e.register(PARSE_PARAMETERS, |e, a| {
        e.mem.set_f32(a[7], 2.0);
        e.mem.set_f32(a[8], 3.0);
        ret(1)
    });
    e.register(NI_POINT2_CONSTRUCT, |_, a| ret(a[0]));
    e.register(LIST_OWNER, |_, _| ret(OWNER.load(Relaxed)));
    e.register(OWNER_LIST, |_, _| ret(LIST.load(Relaxed)));
    e.register(WORLD, |_, _| ret(0x6100));
    e.register(ANIMATION, |_, _| ret(0x6200));
    e.register(ANIMATION_MATRIX, |_, _| ret(SOURCE.load(Relaxed)));
    e.register(MATRIX_FROM_Z_ANGLE, |_, a| ret(a[0]));
    e.register(MATRIX_MULTIPLY, |_, a| ret(a[1]));
    e.register(NI_ALLOC, |_, _| ret(PLANE.load(Relaxed)));
    e.register(PLANE_CONSTRUCT, |_, a| ret(a[0]));
    e.register(LIST_ADD, |_, a| {
        assert_eq!(a[0], LIST.load(Relaxed));
        ADDED.store(a[1], Relaxed);
        Ret::default()
    });
    // The Windows imports the count's add and release reach: `InterlockedIncrement` and
    // `InterlockedDecrement` (their new value).
    e.register(INTERLOCKED_INCREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(INTERLOCKED_DECREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(PLANE_DELETE, |_, a| {
        FREED.store(a[0], Relaxed);
        Ret::default()
    });
    e
}

/// S0236 `PlaceOcclusionPlaneFix`: the exe holds the new plane in a `NiPointer` while it adds it to
/// the occluder list (which takes no reference), so the pointer's release at the command's end
/// takes the count back to 0 and deletes the plane the list still holds (the later crash). With the
/// row the plane goes into the list with no pointer: its count stays 0 and nothing deletes it.
#[test]
fn s0236_place_occlusion_plane_keeps_its_plane() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    for on in [false, true] {
        let mut e = occlusion_world();
        e.set_stewie(stewie::OCCLUSION_PLANE_KEPT, on);
        let result = e.mem.alloc(8);
        let words = [1u32, 2, 0, 0, 5, 6, result, 8];
        assert!(e.call(PLACE_OCCLUSION_PLANE, &words).bool(), "row {on}");
        let plane = PLANE.load(Relaxed);
        assert_eq!(ADDED.load(Relaxed), plane, "row {on}: the plane is listed");
        assert_eq!(e.mem.u32(plane + 4), 0, "row {on}: its count");
        let freed = if on { 0 } else { plane };
        assert_eq!(FREED.load(Relaxed), freed, "row {on}: deleted while listed");
    }
}

// ---- S0093.01 to .06: the perk menu's requirement line (`BGSPerk::BuildDescriptionString`) ----

/// `BGSPerk::BuildDescriptionString` (`005ebac0`, `thiscall`: the perk, the buffer, its size).
const BUILD_DESCRIPTION: u32 = 0x005e_bac0;
/// The exe's strings the line is made of (as FalloutNV.exe 1.4.0.525 holds them).
const EXE_TEXTS: [(u32, &[u8]); 13] = [
    (0x0101_2630, b", "),
    (0x0101_2648, b"\n"),
    (0x0101_9f08, b"%s"),
    (0x0102_0770, b" "),
    (0x0104_6ffc, b"%s: %d\n\n"),
    (0x0104_7008, b"--"),
    (0x0104_700c, b"%s %d"),
    (0x0104_7014, b"%s%s%d"),
    (0x0104_701c, b"%s%s%d, "),
    (0x0104_7028, b" %s%s%d %s "),
    (0x0104_7034, b" < "),
    (0x0104_7038, b"%s: "),
    (0x0104_7040, b"%s: %s %d"),
];
/// The settings the line names (a `Setting`'s text at `+4`): the requirements, the ranks, "OR"
/// (`sOr`), the level.
const SETTINGS: [(u32, &[u8]); 4] = [
    (0x011d_4678, b"Req"),
    (0x011d_3b2c, b"Ranks"),
    (0x011d_324c, b"OR"),
    (0x011d_40e4, b"Level"),
];
/// `g_actorValueInfoArray` (the info's abbreviation at `+0x3C`, its length at `+0x40`).
const ACTOR_VALUE_INFOS: u32 = 0x011d_61c8;
/// The description's text getter (the perk's `TESDescription` at `+0x24`, vtable slot `+0x10`).
const DESCRIPTION_TEXT: u32 = 0x00ff_6000;
/// The C runtime's `vsprintf_s`.
const VSPRINTF_S: u32 = 0x00ec_6bb5;
/// The `PerkMenuDescriptionTypoFix` rows, S0093.01 to .06, then .07.
const PERK_ROWS: [stewie::Row; 7] = [
    stewie::PERK_TEXT_NO_DASHES,
    stewie::PERK_TEXT_NO_EMPTY_COMMA,
    stewie::PERK_TEXT_INDEX_BY_ONE,
    stewie::PERK_TEXT_SKIP_OTHER,
    stewie::PERK_TEXT_COMMAS,
    stewie::PERK_TEXT_PERK_NOT_OR,
    stewie::PERK_DESCRIPTION_TOO_LONG,
];

/// A perk requirement (a `Condition`, `0x1C` bytes): `GetPermanentActorValue` (`0x1EF`) of an
/// actor value code, `HasPerk` (`0x1C1`) of a named perk and a rank, or another function.
#[derive(Clone, Copy)]
enum Need {
    Value(u32),
    Perk(&'static [u8], u32),
    Other(u32),
}

/// A condition: what it needs, its compare operator (`== 0`, `!= 1`, `>= 3`, ...), the value, and
/// whether it is joined to the next by OR (the type byte's bit 0).
struct Condition(Need, u8, f32, bool);

fn text_at(e: &mut Engine, text: &[u8]) -> u32 {
    let at = e.mem.alloc(text.len() as u32 + 1);
    e.mem.write(at, text);
    e.mem.set_u8(at + text.len() as u32, 0);
    at
}

/// The description (requirement line, level 2, the text "Text.") of a perk of `ranks` ranks
/// with `conditions`, built with the rows `on` and the other perk rows off.
fn perk_text(conditions: &[Condition], ranks: u8, on: &[stewie::Row]) -> String {
    let mut e = Engine::new();
    for page in [
        0x0101_1000u32,
        0x0101_2000,
        0x0101_9000,
        0x0102_0000,
        0x0104_6000,
        0x0104_7000,
        0x011d_3000,
        0x011d_4000,
        0x011d_6000,
    ] {
        e.map(page, 0x1000);
    }
    for (at, text) in EXE_TEXTS {
        e.mem.write(at, text);
        e.mem.set_u8(at + text.len() as u32, 0);
    }
    for (at, text) in SETTINGS {
        let text = text_at(&mut e, text);
        e.mem.set_u32(at + 4, text);
    }
    // Strength (5) and Agility (10) have abbreviations; karma (0x17) too, which the line skips.
    for (code, name) in [(5u32, &b"STR"[..]), (10, b"AGI"), (0x17, b"KRM")] {
        let info = e.mem.alloc(0x50);
        let text = text_at(&mut e, name);
        e.mem.set_u32(info + 0x3c, text);
        e.mem.set_u16(info + 0x40, name.len() as u16);
        e.mem.set_u32(ACTOR_VALUE_INFOS + 4 * code, info);
    }
    let perk = e.mem.alloc(0x50);
    let vtable = e.mem.alloc(0x20);
    e.mem.set_u32(vtable + 0x10, DESCRIPTION_TEXT);
    e.mem.set_u32(perk + 0x24, vtable);
    let description = text_at(&mut e, b"Text.");
    e.register_double(DESCRIPTION_TEXT, move |_, _| ret(description));
    e.mem.set_u8(perk + 0x39, ranks);
    e.mem.set_u8(perk + 0x3a, 2);
    // The condition list (`tList`: the data word, the next node), its head in the perk at `+0x40`.
    let mut node = perk + 0x40;
    for (i, Condition(need, operator, value, or)) in conditions.iter().enumerate() {
        let c = e.mem.alloc(0x1c);
        e.mem.set_u8(c, (operator << 5) | u8::from(*or));
        e.mem.set_f32(c + 4, *value);
        let (function, first, second) = match *need {
            Need::Value(code) => (0x1ef, code, 0),
            Need::Perk(name, rank) => {
                let form = e.mem.alloc(0x50);
                let name = text_at(&mut e, name);
                // Its `TESFullName` at `+0x18`, the name's text at `+4` of it.
                e.mem.set_u32(form + 0x1c, name);
                (0x1c1, form, rank)
            }
            Need::Other(function) => (function, 0, 0),
        };
        e.mem.set_u32(c + 8, function);
        e.mem.set_u32(c + 0xc, first);
        e.mem.set_u32(c + 0x10, second);
        if i > 0 {
            let next = e.mem.alloc(8);
            e.mem.set_u32(node + 4, next);
            node = next;
        }
        e.mem.set_u32(node, c);
    }
    // The C runtime's `vsprintf_s` (`00ec6bb5`: buffer, size, format, argument list), which the
    // string format (`00406d00`, translated) calls: its own body reaches the thread data and the
    // open `GetLastError` import, so it stands in for the two conversions the line uses, `%s` and
    // `%d`.
    e.register(VSPRINTF_S, |e, a| {
        let (buffer, size, format, mut next) = (a[0], a[1], a[2], a[3]);
        let mut text = Vec::new();
        let mut chars = e.mem.cstr(format).into_iter();
        while let Some(c) = chars.next() {
            if c != b'%' {
                text.push(c);
                continue;
            }
            let word = e.mem.u32(next);
            next += 4;
            match chars.next() {
                Some(b's') => text.extend(e.mem.cstr(word)),
                Some(b'd') => text.extend((word as i32).to_string().into_bytes()),
                other => panic!("conversion {other:?} not stood in"),
            }
        }
        assert!((text.len() as u32) < size, "vsprintf_s overflow");
        e.mem.write(buffer, &text);
        e.mem.set_u8(buffer + text.len() as u32, 0);
        ret(text.len() as u32)
    });
    for row in PERK_ROWS {
        e.set_stewie(row, on.contains(&row));
    }
    let buffer = e.mem.alloc(0x200);
    e.call(BUILD_DESCRIPTION, &[perk, buffer, 0x200]);
    String::from_utf8(e.mem.cstr(buffer)).unwrap()
}

const fn value(code: u32, operator: u8, at_least: f32, or: bool) -> Condition {
    Condition(Need::Value(code), operator, at_least, or)
}

const fn has_perk(name: &'static [u8], rank: u32, operator: u8, v: f32, or: bool) -> Condition {
    Condition(Need::Perk(name, rank), operator, v, or)
}

/// Strength 6 OR Agility 6, and the perk Toughness. The exe's index steps by two after the first
/// condition, so it never reaches Toughness; and it puts two spaces before Strength.
fn strength_or_agility_and_toughness() -> Vec<Condition> {
    vec![
        value(5, 3, 6.0, true),
        value(10, 3, 6.0, false),
        has_perk(b"Toughness", 1, 0, 1.0, false),
    ]
}

/// A perk of two ranks needing NOT Toughness OR Lead Belly.
fn not_toughness_or_lead_belly() -> Vec<Condition> {
    vec![
        has_perk(b"Toughness", 1, 0, 0.0, true),
        has_perk(b"Lead Belly", 0, 0, 1.0, false),
    ]
}

/// A condition the perk menu cannot show (another function).
fn other() -> [Condition; 1] {
    [Condition(Need::Other(0x48), 0, 1.0, false)]
}

const END: &str = "\nLevel: 2\n\nText.";

/// The exe's lines, every row off: the two spaces and the skipped third condition, the "--" of a
/// perk with nothing listed, the ", " of a condition the menu cannot show, the perks run together.
#[test]
fn s0093_the_exes_requirement_lines() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    assert_eq!(
        perk_text(&strength_or_agility_and_toughness(), 1, &[]),
        format!("Req:  STR 6 OR AGI 6{END}")
    );
    assert_eq!(perk_text(&[], 1, &[]), format!("Req: --{END}"));
    assert_eq!(perk_text(&other(), 2, &[]), format!("Req: Ranks 2, {END}"));
    assert_eq!(
        perk_text(&not_toughness_or_lead_belly(), 2, &[]),
        format!("Req: Ranks 2, Toughness 1Lead Belly{END}")
    );
}

/// Each row alone, on the input it changes.
#[test]
fn s0093_each_text_row_alone() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    // .01: no "--".
    assert_eq!(
        perk_text(&[], 1, &[stewie::PERK_TEXT_NO_DASHES]),
        format!("Req: {END}")
    );
    // .02: no ", " for a condition the menu cannot show; .04: it is not listed at all, so the
    // exe's "--" follows.
    assert_eq!(
        perk_text(&other(), 2, &[stewie::PERK_TEXT_NO_EMPTY_COMMA]),
        format!("Req: Ranks 2{END}")
    );
    assert_eq!(
        perk_text(&other(), 2, &[stewie::PERK_TEXT_SKIP_OTHER]),
        format!("Req: Ranks 2--{END}")
    );
    // .03: every condition read, Agility followed by Toughness.
    assert_eq!(
        perk_text(
            &strength_or_agility_and_toughness(),
            1,
            &[stewie::PERK_TEXT_INDEX_BY_ONE]
        ),
        format!("Req:  STR 6 OR AGI 6, Toughness 1{END}")
    );
    // .05: no ", " before a condition the menu does not show (karma).
    let hidden_after = [value(5, 3, 6.0, false), value(0x17, 3, 1.0, false)];
    assert_eq!(
        perk_text(&hidden_after, 1, &[]),
        format!("Req: STR 6, {END}")
    );
    assert_eq!(
        perk_text(&hidden_after, 1, &[stewie::PERK_TEXT_COMMAS]),
        format!("Req: STR 6{END}")
    );
    // .06: a comma after the ranks and after each perk, NOT and OR on perks (alone it leaves the
    // doubled and the trailing comma that .05 takes away).
    assert_eq!(
        perk_text(
            &not_toughness_or_lead_belly(),
            2,
            &[stewie::PERK_TEXT_PERK_NOT_OR]
        ),
        format!("Req: Ranks 2, , NOT Toughness 1 OR Lead Belly, {END}")
    );
}

/// The six rows together (as the fixes preset has them): Tweaks' lines.
#[test]
fn s0093_the_rows_together() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let all = &PERK_ROWS[..6];
    assert_eq!(
        perk_text(&strength_or_agility_and_toughness(), 1, all),
        format!("Req: STR 6 OR AGI 6, Toughness 1{END}")
    );
    assert_eq!(
        perk_text(&not_toughness_or_lead_belly(), 2, all),
        format!("Req: Ranks 2, NOT Toughness 1 OR Lead Belly{END}")
    );
    assert_eq!(perk_text(&[], 1, all), format!("Req: {END}"));
    assert_eq!(perk_text(&other(), 2, all), format!("Req: Ranks 2{END}"));
}

/// The OR look-ahead (S0093.05 at `0x5EBCFB` for a value, `OnFormatPerkHandleConditions` for a
/// perk): an OR followed only by a condition the menu does not show (karma, another function) or by
/// nothing gets no "OR". The exe leaves the dangling " OR ".
#[test]
fn s0093_an_or_with_nothing_shown_after_it() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let all = &PERK_ROWS[..6];
    let value_then_karma = [value(5, 3, 6.0, true), value(0x17, 3, 1.0, false)];
    assert_eq!(
        perk_text(&value_then_karma, 1, &[]),
        format!("Req:  STR 6 OR {END}")
    );
    assert_eq!(
        perk_text(&value_then_karma, 1, &[stewie::PERK_TEXT_COMMAS]),
        format!("Req: STR 6{END}")
    );
    assert_eq!(
        perk_text(&value_then_karma, 1, all),
        format!("Req: STR 6{END}")
    );
    let perk_then_other = [
        has_perk(b"Toughness", 1, 0, 1.0, true),
        Condition(Need::Other(0x48), 0, 1.0, false),
    ];
    assert_eq!(
        perk_text(&perk_then_other, 1, all),
        format!("Req: Toughness 1{END}")
    );
    let perk_alone = [has_perk(b"Toughness", 1, 0, 1.0, true)];
    assert_eq!(
        perk_text(&perk_alone, 1, all),
        format!("Req: Toughness 1{END}")
    );
}

/// S0093.06's NOT for the `!=` operator: `!= 1` (the perk missing) gets "NOT", `!= 0` (the perk
/// had) does not.
#[test]
fn s0093_not_for_the_not_equal_operator() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let all = &PERK_ROWS[..6];
    assert_eq!(
        perk_text(&[has_perk(b"Toughness", 1, 1, 1.0, false)], 1, all),
        format!("Req: NOT Toughness 1{END}")
    );
    assert_eq!(
        perk_text(&[has_perk(b"Toughness", 1, 1, 0.0, false)], 1, all),
        format!("Req: Toughness 1{END}")
    );
}

/// S0093.03's skip step: a value requirement of 0 is passed over. The exe's step of two then also
/// passes over the condition after it ("Req: AGI 6", Toughness never read); with the rows each
/// condition is read.
#[test]
fn s0093_a_zero_requirement_is_skipped_by_one() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let conditions = [
        value(5, 3, 0.0, false),
        value(10, 3, 6.0, false),
        has_perk(b"Toughness", 1, 0, 1.0, false),
    ];
    assert_eq!(perk_text(&conditions, 1, &[]), format!("Req: AGI 6{END}"));
    assert_eq!(
        perk_text(&conditions, 1, &PERK_ROWS[..6]),
        format!("Req: AGI 6, Toughness 1{END}")
    );
}

/// S0093.05's `AddCommaIfMissing` at the value site: a two-rank perk whose first requirement is a
/// value gets one ", " after the ranks (S0093.06's heading already ends with one).
#[test]
fn s0093_one_comma_after_the_ranks() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let strength = [value(5, 3, 6.0, false)];
    assert_eq!(
        perk_text(&strength, 2, &[]),
        format!("Req: Ranks 2, STR 6{END}")
    );
    assert_eq!(
        perk_text(&strength, 2, &PERK_ROWS[..6]),
        format!("Req: Ranks 2, STR 6{END}")
    );
    assert_eq!(
        perk_text(&strength, 2, &[stewie::PERK_TEXT_PERK_NOT_OR]),
        format!("Req: Ranks 2, , STR 6{END}")
    );
}

/// S0093.06 formats a perk requirement as Tweaks' `snprintf`: a mod's perk name of 130 characters
/// is cut to the work buffer (0x80 bytes: 127 characters and the 0), where the exe's `vsprintf_s`
/// would stop the game.
#[test]
fn s0093_a_long_perk_name_is_cut_to_fit() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let name: &'static [u8] = Box::leak(vec![b'N'; 130].into_boxed_slice());
    let text = perk_text(&[has_perk(name, 1, 0, 1.0, false)], 1, &PERK_ROWS[..6]);
    let cut = String::from_utf8(name[..127].to_vec()).unwrap();
    assert_eq!(text, format!("Req: {cut}{END}"));
}
