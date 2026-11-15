//! Tests of the LN miscellaneous commands, at the raw offsets the module's doc lists (each from
//! the exe's translations or the Xbox PDB layouts), with expectations read from JIP's source.

use super::*;
use crate::commands::test_support::*;

const PLAYER_AT: u32 = FORMS + 0x1_0000;
const MANAGER: u32 = FORMS + 0x1_2000;
const HANDLER: u32 = FORMS + 0x1_3000;
const ARMO: u32 = FORMS + 0x2000;
const ARMO2: u32 = FORMS + 0x2400;
const WEAP: u32 = FORMS + 0x3000;
const LIST: u32 = FORMS + 0x3800;
const LIST2: u32 = FORMS + 0x3880;
const AMMO: u32 = FORMS + 0x3900;
const RACE: u32 = FORMS + 0x3a00;
const EYES: u32 = FORMS + 0x3b00;
const HAIR: u32 = FORMS + 0x3c00;
const PERK: u32 = FORMS + 0x3d00;
const IDLE: u32 = FORMS + 0x4000;
const ANIO: u32 = FORMS + 0x4100;
const CONT: u32 = FORMS + 0x4200;
const SOUND: u32 = FORMS + 0x4300;
const SOUND2: u32 = FORMS + 0x4380;
const ZONE: u32 = FORMS + 0x4400;
const IDS: u32 = FORMS + 0x4500;
const WATER: u32 = FORMS + 0x5000;
const SPELL: u32 = FORMS + 0x5200;
const ALCH: u32 = FORMS + 0x5400;
const NOTE: u32 = FORMS + 0x5500;
const NOTE2: u32 = FORMS + 0x5580;
const TERM: u32 = FORMS + 0x5600;
const KEY: u32 = FORMS + 0x5700;
const CELL: u32 = FORMS + 0x5800;
const COORDS: u32 = FORMS + 0x5900;
const WORLD: u32 = FORMS + 0x5a00;
const WORLD2: u32 = FORMS + 0x5b00;
const GLOBAL1: u32 = FORMS + 0x5c00;
const GLOBAL2: u32 = FORMS + 0x5c40;
const REFR: u32 = FORMS + 0x6000;
const CHAR: u32 = FORMS + 0x6100;
const BIPED: u32 = FORMS + 0x6200;
const PWATER: u32 = FORMS + 0x6400;
const EFFECT: u32 = FORMS + 0x6500;
const COND: u32 = FORMS + 0x6600;
const PERK_FORM: u32 = FORMS + 0x6700;
const NODES: u32 = FORMS + 0x7000;
const CHAR_VT: u32 = FORMS + 0x8000;
const IS_CHAR_FN: u32 = 0x7000_0010;
const BIPED_FN: u32 = 0x7000_0020;
const CAST_FN: u32 = 0x00ec_43fb;

fn setup() -> Engine {
    let mut e = engine();
    extra_stand_ins(&mut e);
    for page in [0x011d_e000, 0x011d_8000, 0x011a_c000, 0x011c_3000] {
        e.map(page, 0x1000);
    }
    e.mem.set_u32(0x011d_ea3c, PLAYER_AT);
    e.mem.set_u32(0x011d_8a80, MANAGER);
    e.mem.set_u32(0x011c_3f2c, HANDLER);
    form(&mut e, PLAYER_AT, 0x3b, 0x14);
    let forms = [
        (ARMO, 0x18, 0x101),
        (ARMO2, 0x18, 0x102),
        (WEAP, 0x28, 0x103),
        (LIST, 0x55, 0x104),
        (AMMO, 0x29, 0x105),
        (RACE, 0x0c, 0x106),
        (EYES, 0x0b, 0x107),
        (HAIR, 0x0a, 0x108),
        (PERK, 0x56, 0x109),
        (IDLE, 0x48, 0x10a),
        (ANIO, 0x4d, 0x10b),
        (CONT, 0x1b, 0x10c),
        (SOUND, 0x0d, 0x10d),
        (ZONE, 0x61, 0x10e),
        (IDS, 0x5f, 0x10f),
        (WATER, 0x4e, 0x110),
        (SPELL, 0x14, 0x111),
        (ALCH, 0x2f, 0x112),
        (NOTE, 0x31, 0x113),
        (TERM, 0x17, 0x114),
        (KEY, 0x2e, 0x115),
        (CELL, 0x39, 0x116),
        (WORLD, 0x41, 0x117),
        (WORLD2, 0x41, 0x118),
        (REFR, 0x3a, 0x119),
        (NOTE2, 0x31, 0x11a),
        (SOUND2, 0x0d, 0x11b),
        (LIST2, 0x55, 0x11c),
        (PERK_FORM, 0x56, 0x0003_1d6a),
    ];
    for (i, &(at, kind, id)) in forms.iter().enumerate() {
        form(&mut e, at, kind, id);
        set_ref(&mut e, i as u16 + 1, at);
    }
    e.mem.set_u32(ARMO, VTBL_ARMOR);
    e.mem.set_u32(ARMO2, VTBL_ARMOR);
    // `__RTDynamicCast` to `BGSPickupPutdownSounds`: a weapon has it at +0xE8.
    e.register(CAST_FN, |e, w| {
        assert_eq!((w[2], w[3]), (RTTI_TES_FORM, RTTI_PICKUP_PUTDOWN_SOUNDS));
        Ret {
            eax: if w[0] != 0 && e.mem.u8(w[0] + 4) == 0x28 {
                w[0] + 0xe8
            } else {
                0
            },
            ..Ret::default()
        }
    });
    e
}

/// The script's reference index of a form set up above.
fn idx(at: u32) -> u16 {
    [
        ARMO, ARMO2, WEAP, LIST, AMMO, RACE, EYES, HAIR, PERK, IDLE, ANIO, CONT, SOUND, ZONE, IDS,
        WATER, SPELL, ALCH, NOTE, TERM, KEY, CELL, WORLD, WORLD2, REFR, NOTE2, SOUND2, LIST2,
        PERK_FORM,
    ]
    .iter()
    .position(|&f| f == at)
    .expect("a set-up form") as u16
        + 1
}

fn rf(at: u32) -> Vec<u8> {
    r(idx(at))
}

/// A list of `items` from the node at `at` (the first inline), further nodes after it.
fn put_list(e: &mut Engine, at: u32, items: &[u32]) {
    e.mem.set_u32(at, 0);
    e.mem.set_u32(at + 4, 0);
    let mut node = at;
    for (i, &item) in items.iter().enumerate() {
        if i > 0 {
            let next = NODES + 0x100 * (at & 0xff) + 8 * i as u32;
            e.mem.set_u32(node + 4, next);
            node = next;
            e.mem.set_u32(node + 4, 0);
        }
        e.mem.set_u32(node, item);
    }
}

fn character(e: &mut Engine, biped: u32, is_char: bool) {
    e.mem.set_u32(CHAR, CHAR_VT);
    e.mem.set_u32(CHAR_VT + 0x218, IS_CHAR_FN);
    e.mem.set_u32(CHAR_VT + 0x1e8, BIPED_FN);
    e.mem.set_u32(CHAR + 0x400, u32::from(is_char));
    e.mem.set_u32(CHAR + 0x404, biped);
    e.register(IS_CHAR_FN, |e, w| Ret {
        eax: 0xffff_ff00 | e.mem.u32(w[0] + 0x400),
        ..Ret::default()
    });
    e.register(BIPED_FN, |e, w| Ret {
        eax: e.mem.u32(w[0] + 0x404),
        ..Ret::default()
    });
}

#[test]
fn armor_class_of_an_armor_and_of_a_character() {
    let mut e = setup();
    e.mem.set_u32(ARMO + 0x78, 0x80 | 8);
    assert_eq!(
        call(&mut e, 0x2264, &[rf(ARMO)]).1,
        3.0,
        "power armour first"
    );
    e.mem.set_u32(ARMO + 0x78, 8);
    assert_eq!(call(&mut e, 0x2264, &[rf(ARMO)]).1, 2.0);
    e.mem.set_u32(ARMO + 0x78, 0x40);
    assert_eq!(call(&mut e, 0x2264, &[rf(ARMO)]).1, 1.0);
    // A weapon (no armour vtable) returns at once.
    assert_eq!(call(&mut e, 0x2264, &[rf(WEAP)]).1, 0.0);
    // A form of type 0x18 with another vtable is no armour (IS_TYPE).
    e.mem.set_u32(ARMO, 0);
    assert_eq!(call(&mut e, 0x2264, &[rf(ARMO)]).1, 0.0);
    e.mem.set_u32(ARMO, VTBL_ARMOR);
    // A character: the first armour covering the upper body (part mask bit 4).
    character(&mut e, BIPED, true);
    e.mem.set_u32(ARMO + 0x74, 2);
    e.mem.set_u32(ARMO2 + 0x74, 4 | 1);
    e.mem.set_u32(ARMO2 + 0x78, 8);
    e.mem.set_u32(BIPED + 0x2c, WEAP);
    e.mem.set_u32(BIPED + 0x2c + 0x10, ARMO);
    e.mem.set_u32(BIPED + 0x2c + 0x10 * 19, ARMO2);
    assert_eq!(call_on(&mut e, 0x2264, CHAR, &[]).1, 2.0);
    e.mem.set_u32(BIPED + 0x2c + 0x10 * 19, 0);
    assert_eq!(call_on(&mut e, 0x2264, CHAR, &[]).1, 0.0, "none covers it");
    character(&mut e, 0, true);
    assert_eq!(
        call_on(&mut e, 0x2264, CHAR, &[]).1,
        1.0,
        "no biped data: true"
    );
    // Not a character (only the answer's low byte counts): 0 though it wears the armour.
    character(&mut e, BIPED, false);
    e.mem.set_u32(BIPED + 0x2c + 0x10 * 19, ARMO2);
    assert_eq!(call_on(&mut e, 0x2264, CHAR, &[]).1, 0.0, "not a character");
    // The eval reads the reference.
    character(&mut e, BIPED, true);
    e.mem.set_u32(BIPED + 0x2c + 0x10 * 19, ARMO2);
    let eval = e.mem.u32(nvse::table::record_of(&e, 0x2264) + 0x20);
    e.mem.set_f64(RESULT, 9.0);
    e.call(eval, &[CHAR, 0, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 2.0);
    e.call(eval, &[0, 0, 0, RESULT]);
    assert_eq!(e.mem.f64(RESULT), 0.0);
}

#[test]
fn armor_class_echoes_in_the_console() {
    let mut e = setup();
    e.mem.set_u32(ARMO + 0x78, 8);
    let (res, lines) = echoed(&mut e, 0x2264, 0, &[rf(ARMO)]);
    assert_eq!(res, 2.0);
    assert_eq!(lines, vec![b"GetArmorClass >> 2".to_vec()]);
    let (_, lines) = echoed(&mut e, 0x2264, 0, &[rf(WEAP)]);
    assert!(lines.is_empty(), "no echo for a form that is no armour");
}

#[test]
fn set_global_value_by_name() {
    let mut e = setup();
    put_list(&mut e, HANDLER + 0xe8, &[0, GLOBAL1, GLOBAL2]);
    string(&mut e, GLOBAL1 + 0x18, Some(b"Other"));
    string(&mut e, GLOBAL2 + 0x18, Some(b"MyVar"));
    call(&mut e, 0x221c, &[s(b"myvar"), z(2.5)]);
    assert_eq!(e.mem.f32(GLOBAL2 + 0x24), 2.5);
    assert_eq!(e.mem.f32(GLOBAL1 + 0x24), 0.0);
    // Only the first match.
    string(&mut e, GLOBAL1 + 0x18, Some(b"MYVAR"));
    call(&mut e, 0x221c, &[s(b"MyVar"), z(4.0)]);
    assert_eq!(e.mem.f32(GLOBAL1 + 0x24), 4.0);
    assert_eq!(e.mem.f32(GLOBAL2 + 0x24), 2.5);
    // A global with no name buffer is passed over.
    string(&mut e, GLOBAL1 + 0x18, None);
    call(&mut e, 0x221c, &[s(b""), z(1.0)]);
    assert_eq!(e.mem.f32(GLOBAL1 + 0x24), 4.0);
    assert_eq!(e.mem.f32(GLOBAL2 + 0x24), 2.5);
}

#[test]
fn worldspace_parent() {
    let mut e = setup();
    call(&mut e, 0x2262, &[rf(WORLD)]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(WORLD + 0x70, WORLD2);
    call(&mut e, 0x2262, &[rf(WORLD)]);
    assert_eq!(form_result(&e), 0x118);
}

fn map_xy(e: &Engine, id: f64) -> (Vec<u8>, Vec<u8>, nvse::vars::Data, nvse::vars::Data) {
    let a = nvse::vars::array(e, id as u32).expect("a map");
    match &a.elements {
        nvse::vars::Container::StrMap(v) => (
            v[0].0.clone(),
            v[1].0.clone(),
            v[0].1.data.clone(),
            v[1].1.data.clone(),
        ),
        _ => panic!("a string map"),
    }
}

#[test]
fn cell_coordinates() {
    use nvse::vars::Data::Number;
    let mut e = setup();
    e.mem.set_u32(CELL + 0x48, COORDS);
    e.mem.set_u32(COORDS, (-3i32) as u32);
    e.mem.set_u32(COORDS + 4, 7);
    let (_, res) = call(&mut e, 0x224a, &[rf(CELL)]);
    assert_eq!(
        map_xy(&e, res),
        (b"x".to_vec(), b"y".to_vec(), Number(-3.0), Number(7.0))
    );
    // An interior answers 0, 0.
    e.mem.set_u8(CELL + 0x24, 1);
    let (_, res) = call(&mut e, 0x224a, &[rf(CELL)]);
    assert_eq!(map_xy(&e, res).2, Number(0.0));
    e.mem.set_u8(CELL + 0x24, 0);
    // Without a cell: the calling reference's, else the player's.
    e.mem.set_u32(REFR + 0x40, CELL);
    let (_, res) = call_on(&mut e, 0x224a, REFR, &[]);
    assert_eq!(map_xy(&e, res).3, Number(7.0));
    let (_, res) = call(&mut e, 0x224a, &[]);
    assert_eq!(map_xy(&e, res).2, Number(0.0), "the player has no cell");
    e.mem.set_u32(PLAYER_AT + 0x40, CELL);
    let (_, res) = call(&mut e, 0x224a, &[]);
    assert_eq!(map_xy(&e, res).2, Number(-3.0));
    // A form that is not a cell makes no map.
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(call(&mut e, 0x224a, &[rf(WORLD)]).1, 0.0);
    // The console line for an exterior only.
    let (_, lines) = echoed(&mut e, 0x224a, 0, &[rf(CELL)]);
    assert_eq!(lines, vec![b"GetCellCoords >> -3, 7".to_vec()]);
    e.mem.set_u8(CELL + 0x24, 1);
    let (_, lines) = echoed(&mut e, 0x224a, 0, &[rf(CELL)]);
    assert!(lines.is_empty());
}

#[test]
fn biped_model_lists() {
    let mut e = setup();
    call(&mut e, 0x2219, &[rf(ARMO), rf(LIST)]);
    assert_eq!(e.mem.u32(ARMO + 0x168), LIST);
    call(&mut e, 0x2219, &[rf(WEAP), rf(LIST2)]);
    assert_eq!(e.mem.u32(WEAP + 0xe4), LIST2);
    call(&mut e, 0x2219, &[rf(WEAP), rf(SOUND)]);
    assert_eq!(e.mem.u32(WEAP + 0xe4), LIST2, "a sound is no form list");
    call(&mut e, 0x2218, &[rf(ARMO)]);
    assert_eq!(form_result(&e), 0x104);
    e.mem.set_u32(REFR + 0x20, WEAP);
    call_on(&mut e, 0x2218, REFR, &[]);
    assert_eq!(form_result(&e), 0x11c, "the calling reference's base");
    call(&mut e, 0x2219, &[rf(WEAP)]);
    assert_eq!(e.mem.u32(WEAP + 0xe4), 0, "cleared");
    // Another form: nothing read or written.
    e.mem.set_u32(SOUND + 0x168, 0xa5a5_a5a5);
    call(&mut e, 0x2219, &[rf(SOUND), rf(LIST)]);
    assert_eq!(e.mem.u32(SOUND + 0x168), 0xa5a5_a5a5);
    e.mem.set_u32(RESULT, 0);
    call(&mut e, 0x2218, &[rf(SOUND)]);
    assert_eq!(form_result(&e), 0);
}

#[test]
fn player_flags_and_active_quest() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x22aa, &[]).1, 0.0);
    e.mem.set_u8(PLAYER_AT + 0x7c7, 1);
    assert_eq!(call(&mut e, 0x22aa, &[]).1, 1.0);
    assert_eq!(call(&mut e, 0x2338, &[]).1, 0.0);
    e.mem.set_u8(PLAYER_AT + 0x240, 1);
    assert_eq!(call(&mut e, 0x2338, &[]).1, 1.0);
    call(&mut e, 0x2279, &[]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(PLAYER_AT + 0x6b8, ZONE);
    call(&mut e, 0x2279, &[]);
    assert_eq!(form_result(&e), 0x10e);
}

#[test]
fn idle_anim_paths() {
    let mut e = setup();
    model(&mut e, IDLE + 0x18, Some(b"idle.kf"));
    let (_, res) = call(&mut e, 0x22ab, &[rf(IDLE)]);
    assert_eq!(string_result(&e, res), b"idle.kf");
    call(&mut e, 0x22ac, &[rf(IDLE), s(b"new.kf")]);
    assert_eq!(text_at(&e, IDLE + 0x18 + 4), b"new.kf");
    model(&mut e, ANIO + 0x18, Some(b"x.kf"));
    let (_, res) = call(&mut e, 0x22ab, &[rf(ANIO)]);
    assert_eq!(string_result(&e, res), b"", "not an idle");
    call(&mut e, 0x22ac, &[rf(ANIO), s(b"y.kf")]);
    assert_eq!(text_at(&e, ANIO + 0x18 + 4), b"x.kf");
}

#[test]
fn names() {
    let mut e = setup();
    // A weapon's full name is its component at 0x0C words (kBaseComponentOffsets byte 0); the
    // reference's vtable answers true at +0xF0 (its name is its base form's).
    let (form_vt, ref_vt) = (FORMS + 0xc000, FORMS + 0xc400);
    e.mem.set_u32(WEAP, form_vt);
    e.mem.set_u32(REFR, ref_vt);
    e.mem.set_u32(ref_vt + 0xf0, 0x008d_0360);
    let name = WEAP + 0x0c * 4;
    string(&mut e, name + 4, Some(b"Gun"));
    let (_, res) = call(&mut e, 0x22ad, &[rf(WEAP)]);
    assert_eq!(string_result(&e, res), b"Gun");
    call(&mut e, 0x22fc, &[s(b"Rifle"), rf(WEAP)]);
    assert_eq!(text_at(&e, name + 4), b"Rifle");
    // The calling reference: its base form's name ...
    e.mem.set_u32(REFR + 0x20, WEAP);
    let (_, res) = call_on(&mut e, 0x22ad, REFR, &[]);
    assert_eq!(string_result(&e, res), b"Rifle");
    // ... unless a script named it (JIP's reference names map).
    let block = FORMS + 0x9000;
    e.mem.write(block, b"Mine\0");
    crate::world_hooks::state(&mut e).ref_names.insert(
        REFR,
        crate::world_hooks::TextBlock {
            at: block,
            capacity: 8,
        },
    );
    let (_, res) = call_on(&mut e, 0x22ad, REFR, &[]);
    assert_eq!(string_result(&e, res), b"Mine");
    // A form given wins over the reference.
    let (_, res) = call_on(&mut e, 0x22ad, REFR, &[rf(WEAP)]);
    assert_eq!(string_result(&e, res), b"Rifle");
    // Setting on the reference sets its base form's name.
    call_on(&mut e, 0x22fc, REFR, &[s(b"Ref")]);
    assert_eq!(text_at(&e, name + 4), b"Ref");
    // No form and no reference: "".
    let (_, res) = call(&mut e, 0x22ad, &[]);
    assert_eq!(string_result(&e, res), b"");
}

#[test]
fn global_time_multiplier_echoes() {
    let mut e = setup();
    e.mem.set_f32(0x011a_c3a0, 1.5);
    assert_eq!(call(&mut e, 0x22b0, &[]).1, 1.5);
    let (res, lines) = echoed(&mut e, 0x22b0, 0, &[]);
    assert_eq!(res, 1.5);
    assert_eq!(lines, vec![b"GetGlobalTimeMultiplier >> 1.5".to_vec()]);
}

#[test]
fn water_effect() {
    let mut e = setup();
    call(&mut e, 0x22bf, &[rf(WATER)]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(WATER + 0x178, SPELL);
    call(&mut e, 0x22bf, &[rf(WATER)]);
    assert_eq!(form_result(&e), 0x111);
    e.mem.set_u32(RESULT, 0);
    e.mem.set_u32(SOUND + 0x178, SPELL);
    call(&mut e, 0x22bf, &[rf(SOUND)]);
    assert_eq!(form_result(&e), 0, "not a water form");
}

#[test]
fn form_flags() {
    let mut e = setup();
    e.mem.set_u32(SOUND + 8, 0x0000_0420);
    assert_eq!(call(&mut e, 0x22de, &[rf(SOUND), n(0x400)]).1, 1.0);
    assert_eq!(call(&mut e, 0x22de, &[rf(SOUND), n(0x800)]).1, 0.0);
    call(&mut e, 0x22df, &[rf(SOUND), n(0x8001), n(1)]);
    assert_eq!(e.mem.u32(SOUND + 8), 0x8421);
    call(&mut e, 0x22df, &[rf(SOUND), n(0x420), n(0)]);
    assert_eq!(e.mem.u32(SOUND + 8), 0x8001);
    let (res, lines) = echoed(&mut e, 0x22de, 0, &[rf(SOUND), n(1)]);
    assert_eq!(res, 1.0);
    assert_eq!(lines, vec![b"GetFormFlag >> 1".to_vec()]);
}

#[test]
fn ingestible_flags() {
    let mut e = setup();
    e.mem.set_u8(ALCH + 0xbc, 0x02);
    assert_eq!(call(&mut e, 0x22e2, &[rf(ALCH), n(2)]).1, 1.0);
    assert_eq!(call(&mut e, 0x22e2, &[rf(ALCH), n(0x101)]).1, 0.0);
    call(&mut e, 0x22e3, &[rf(ALCH), n(0x101), n(1)]);
    assert_eq!(e.mem.u8(ALCH + 0xbc), 0x03, "the low byte of the bits");
    assert_eq!(e.mem.u8(ALCH + 0xbd), 0);
    call(&mut e, 0x22e3, &[rf(ALCH), n(2), n(0)]);
    assert_eq!(e.mem.u8(ALCH + 0xbc), 0x01);
    // Another form: untouched, 0.
    e.mem.set_u8(SPELL + 0xbc, 0xff);
    assert_eq!(call(&mut e, 0x22e2, &[rf(SPELL), n(1)]).1, 0.0);
    call(&mut e, 0x22e3, &[rf(SPELL), n(1), n(0)]);
    assert_eq!(e.mem.u8(SPELL + 0xbc), 0xff);
}

#[test]
fn skill_magazines() {
    let mut e = setup();
    assert_eq!(call(&mut e, 0x22e4, &[rf(ALCH)]).1, 0.0);
    put_list(&mut e, ALCH + 0x40, &[0, EFFECT]);
    put_list(&mut e, EFFECT + 0x1c, &[COND]);
    e.mem.set_u32(COND + 8, 0x1c1);
    e.mem.set_u32(COND + 0xc, PERK);
    assert_eq!(call(&mut e, 0x22e4, &[rf(ALCH)]).1, 0.0, "another perk");
    e.mem.set_u32(COND + 0xc, PERK_FORM);
    assert_eq!(call(&mut e, 0x22e4, &[rf(ALCH)]).1, 1.0, "Retention");
    e.mem.set_u32(PERK_FORM + 0xc, 0x0003_1de1);
    assert_eq!(call(&mut e, 0x22e4, &[rf(ALCH)]).1, 1.0, "Comprehension");
    e.mem.set_u32(COND + 8, 0x1c2);
    assert_eq!(call(&mut e, 0x22e4, &[rf(ALCH)]).1, 0.0, "another function");
    e.mem.set_u32(COND + 8, 0x1c1);
    e.mem.set_u32(COND + 0xc, 0);
    assert_eq!(call(&mut e, 0x22e4, &[rf(ALCH)]).1, 0.0, "no parameter");
    assert_eq!(
        call(&mut e, 0x22e4, &[rf(SPELL)]).1,
        0.0,
        "not an ingestible"
    );
}

#[test]
fn player_has_key_terminal_and_lock() {
    let mut e = setup();
    let eval = e.mem.u32(nvse::table::record_of(&e, 0x22eb) + 0x20);
    let check = |e: &mut Engine| -> (f64, f64) {
        let x = call_on(e, 0x22eb, REFR, &[]).1;
        e.mem.set_f64(RESULT, 0.0);
        e.call(eval, &[REFR, 0, 0, RESULT]);
        (x, e.mem.f64(RESULT))
    };
    // A terminal: its password note among the player's notes.
    e.mem.set_u32(REFR + 0x20, TERM);
    e.mem.set_u32(TERM + 0xa0, NOTE);
    assert_eq!(check(&mut e), (0.0, 0.0));
    put_list(&mut e, PLAYER_AT + 0x5e4, &[NOTE2, NOTE]);
    assert_eq!(check(&mut e), (1.0, 1.0));
    e.mem.set_u32(TERM + 0xa0, 0);
    assert_eq!(check(&mut e), (0.0, 0.0), "no password");
    // A locked container: its key in the player's inventory.
    e.mem.set_u32(REFR + 0x20, CONT);
    let (lock, data) = (FORMS + 0xa000, FORMS + 0xa100);
    put_extra(&mut e, REFR + 0x44, lock, 0x2a, data);
    e.mem.set_u32(data + 4, KEY);
    assert_eq!(check(&mut e), (0.0, 0.0), "the player has no container");
    let pbase = FORMS + 0xb000;
    form(&mut e, pbase, 0x1b, 0x7);
    e.mem.set_u32(PLAYER_AT + 0x20, pbase);
    let entry = FORMS + 0xb100;
    put_list(&mut e, pbase + 0x34, &[entry]);
    e.mem.set_u32(entry, 1);
    e.mem.set_u32(entry + 4, KEY);
    assert_eq!(check(&mut e), (1.0, 1.0));
    e.mem.set_u32(data + 4, ALCH);
    assert_eq!(check(&mut e), (0.0, 0.0), "another key");
}

#[test]
fn anim_object_idle() {
    let mut e = setup();
    call(&mut e, 0x22f4, &[rf(ANIO), rf(IDLE)]);
    assert_eq!(e.mem.u32(ANIO + 0x38), IDLE);
    call(&mut e, 0x22f3, &[rf(ANIO)]);
    assert_eq!(form_result(&e), 0x10a);
    call(&mut e, 0x22f4, &[rf(ANIO), rf(SOUND)]);
    assert_eq!(e.mem.u32(ANIO + 0x38), IDLE, "a sound is no idle");
    e.mem.set_u32(SOUND + 0x38, 0xa5a5_a5a5);
    call(&mut e, 0x22f4, &[rf(SOUND), rf(IDLE)]);
    assert_eq!(e.mem.u32(SOUND + 0x38), 0xa5a5_a5a5);
    e.mem.set_u32(RESULT, 0);
    e.mem.set_u32(IDLE + 0x38, IDLE);
    call(&mut e, 0x22f3, &[rf(IDLE)]);
    assert_eq!(form_result(&e), 0, "not an animated object");
}

#[test]
fn pickup_and_putdown_sounds() {
    let mut e = setup();
    call(&mut e, 0x2302, &[rf(WEAP), rf(SOUND)]);
    call(&mut e, 0x2304, &[rf(WEAP), rf(SOUND2)]);
    assert_eq!(e.mem.u32(WEAP + 0xec), SOUND);
    assert_eq!(e.mem.u32(WEAP + 0xf0), SOUND2);
    call(&mut e, 0x2301, &[rf(WEAP)]);
    assert_eq!(form_result(&e), 0x10d);
    call(&mut e, 0x2303, &[rf(WEAP)]);
    assert_eq!(form_result(&e), 0x11b);
    call(&mut e, 0x2302, &[rf(WEAP), rf(IDLE)]);
    assert_eq!(e.mem.u32(WEAP + 0xec), SOUND, "an idle is no sound");
    // A form without the component: nothing.
    e.mem.set_u32(RESULT, 0);
    call(&mut e, 0x2301, &[rf(ARMO)]);
    assert_eq!(form_result(&e), 0);
    e.mem.set_u32(ARMO + 0xec, 0xa5a5_a5a5);
    call(&mut e, 0x2302, &[rf(ARMO), rf(SOUND)]);
    assert_eq!(e.mem.u32(ARMO + 0xec), 0xa5a5_a5a5);
}

#[test]
fn container_sounds() {
    let mut e = setup();
    call(&mut e, 0x2306, &[rf(SOUND), rf(CONT)]);
    assert_eq!(e.mem.u32(CONT + 0x8c), SOUND);
    e.mem.set_u32(REFR + 0x20, CONT);
    call_on(&mut e, 0x2308, REFR, &[rf(SOUND2)]);
    assert_eq!(
        e.mem.u32(CONT + 0x90),
        SOUND2,
        "the calling reference's base"
    );
    call(&mut e, 0x2305, &[rf(CONT)]);
    assert_eq!(form_result(&e), 0x10d);
    call_on(&mut e, 0x2307, REFR, &[]);
    assert_eq!(form_result(&e), 0x11b);
    call(&mut e, 0x2306, &[rf(IDLE), rf(CONT)]);
    assert_eq!(e.mem.u32(CONT + 0x8c), SOUND, "an idle is no sound");
    e.mem.set_u32(WEAP + 0x8c, 0xa5a5_a5a5);
    call(&mut e, 0x2306, &[rf(SOUND), rf(WEAP)]);
    assert_eq!(e.mem.u32(WEAP + 0x8c), 0xa5a5_a5a5, "not a container");
    e.mem.set_u32(RESULT, 0);
    call(&mut e, 0x2305, &[]);
    assert_eq!(form_result(&e), 0, "no container, no reference");
}

#[test]
fn player_regions() {
    use nvse::vars::Data::Form;
    let mut e = setup();
    let (_, res) = call(&mut e, 0x230a, &[]);
    assert!(res > 0.0, "an array even with none");
    assert!(array_data(&e, res).is_empty());
    put_list(&mut e, PLAYER_AT + 0x768, &[ZONE, 0, SOUND]);
    let (_, res) = call(&mut e, 0x230a, &[]);
    assert_eq!(array_data(&e, res), vec![Form(0x10e), Form(0x10d)]);
}

#[test]
fn zone_flags_and_zone() {
    let mut e = setup();
    e.mem.set_u8(ZONE + 0x1e, 4);
    assert_eq!(call(&mut e, 0x230d, &[rf(ZONE), n(4)]).1, 1.0);
    assert_eq!(call(&mut e, 0x230d, &[rf(ZONE), n(0x102)]).1, 0.0);
    call(&mut e, 0x230e, &[rf(ZONE), n(0x103), n(1)]);
    assert_eq!(e.mem.u8(ZONE + 0x1e), 7);
    call(&mut e, 0x230e, &[rf(ZONE), n(4), n(0)]);
    assert_eq!(e.mem.u8(ZONE + 0x1e), 3);
    e.mem.set_u8(SOUND + 0x1e, 0xff);
    assert_eq!(call(&mut e, 0x230d, &[rf(SOUND), n(1)]).1, 0.0);
    call(&mut e, 0x230e, &[rf(SOUND), n(1), n(0)]);
    assert_eq!(e.mem.u8(SOUND + 0x1e), 0xff);
    // GetZone: the encounter zone extra (0x74) of the calling reference's cell.
    call_on(&mut e, 0x231e, REFR, &[]);
    assert_eq!(form_result(&e), 0, "no cell");
    e.mem.set_u32(REFR + 0x40, CELL);
    let x = FORMS + 0xa200;
    put_extra(&mut e, CELL + 0x28, x, 0x74, ZONE);
    call_on(&mut e, 0x231e, REFR, &[]);
    assert_eq!(form_result(&e), 0x10e);
}

#[test]
fn impact_data_sets() {
    let mut e = setup();
    call(&mut e, 0x2317, &[rf(WEAP), rf(IDS)]);
    assert_eq!(e.mem.u32(WEAP + 0x24c), IDS);
    call(&mut e, 0x2316, &[rf(WEAP)]);
    assert_eq!(form_result(&e), 0x10f);
    call(&mut e, 0x2317, &[rf(WEAP), rf(SOUND)]);
    assert_eq!(e.mem.u32(WEAP + 0x24c), IDS);
    e.mem.set_u32(AMMO + 0x24c, 0xa5a5_a5a5);
    call(&mut e, 0x2317, &[rf(AMMO), rf(IDS)]);
    assert_eq!(e.mem.u32(AMMO + 0x24c), 0xa5a5_a5a5);
}

#[test]
fn playable_flags_by_type() {
    let mut e = setup();
    let cases = [
        (ARMO, 0x78, 0x40u8, true),
        (WEAP, 0x100, 0x80, true),
        (AMMO, 0xac, 2, true),
        (RACE, 0x70, 1, false),
        (EYES, 0x30, 1, false),
        (HAIR, 0x48, 1, false),
    ];
    for (at, field, bit, negative) in cases {
        e.mem.set_u8(at + field, 0x10);
        let playable = |e: &mut Engine| call(e, 0x231c, &[rf(at)]).1;
        assert_eq!(playable(&mut e), if negative { 1.0 } else { 0.0 }, "{at:x}");
        call(&mut e, 0x231d, &[n(1), rf(at)]);
        assert_eq!(
            e.mem.u8(at + field),
            if negative { 0x10 } else { 0x10 | bit }
        );
        assert_eq!(playable(&mut e), 1.0);
        call(&mut e, 0x231d, &[n(0), rf(at)]);
        assert_eq!(
            e.mem.u8(at + field),
            if negative { 0x10 | bit } else { 0x10 }
        );
        assert_eq!(playable(&mut e), 0.0);
    }
    // A perk's byte.
    assert_eq!(call(&mut e, 0x231c, &[rf(PERK)]).1, 0.0);
    call(&mut e, 0x231d, &[n(5), rf(PERK)]);
    assert_eq!(e.mem.u8(PERK + 0x3b), 1);
    assert_eq!(call(&mut e, 0x231c, &[rf(PERK)]).1, 1.0);
    // The calling reference's base; another form type: 0 and untouched.
    e.mem.set_u32(REFR + 0x20, PERK);
    call_on(&mut e, 0x231d, REFR, &[n(0)]);
    assert_eq!(e.mem.u8(PERK + 0x3b), 0);
    e.mem.set_u8(SOUND + 0x78, 0);
    assert_eq!(call(&mut e, 0x231c, &[rf(SOUND)]).1, 0.0);
    call(&mut e, 0x231d, &[n(0), rf(SOUND)]);
    assert_eq!(e.mem.u8(SOUND + 0x78), 0);
}

#[test]
fn notes_added_once() {
    let mut e = setup();
    call(&mut e, 0x22bd, &[rf(NOTE)]);
    assert_eq!(
        e.mem.u32(PLAYER_AT + 0x5e4),
        NOTE,
        "an empty list's first node"
    );
    assert!(allocations().is_empty());
    call(&mut e, 0x22bd, &[rf(NOTE2)]);
    assert_eq!(e.mem.u32(PLAYER_AT + 0x5e4), NOTE2, "put first");
    let next = e.mem.u32(PLAYER_AT + 0x5e8);
    assert_eq!(e.mem.u32(next), NOTE);
    assert_eq!(allocations(), vec![8]);
    call(&mut e, 0x22bd, &[rf(NOTE)]);
    assert_eq!(allocations(), vec![8], "already there");
}

#[test]
fn crosshair_reference_in_a_list() {
    let mut e = setup();
    put_list(&mut e, LIST + 0x18, &[SOUND, REFR, WEAP]);
    assert_eq!(
        call(&mut e, 0x2336, &[rf(LIST)]).1,
        0.0,
        "nothing under the crosshair"
    );
    e.mem.set_u32(MANAGER + 0xfc, REFR);
    e.mem.set_u32(REFR + 0x20, WEAP);
    assert_eq!(
        call(&mut e, 0x2336, &[rf(LIST)]).1,
        1.0,
        "the reference first"
    );
    put_list(&mut e, LIST + 0x18, &[WEAP, REFR]);
    assert_eq!(call(&mut e, 0x2336, &[rf(LIST)]).1, 2.0, "its base first");
    put_list(&mut e, LIST + 0x18, &[SOUND]);
    assert_eq!(call(&mut e, 0x2336, &[rf(LIST)]).1, 0.0);
    // A placeable water's water stands for its base.
    form(&mut e, PWATER, 0x23, 0x120);
    e.mem.set_u32(PWATER + 0x4c, WATER);
    e.mem.set_u32(REFR + 0x20, PWATER);
    put_list(&mut e, LIST + 0x18, &[PWATER, WATER]);
    assert_eq!(call(&mut e, 0x2336, &[rf(LIST)]).1, 2.0);
    put_list(&mut e, LIST + 0x18, &[PWATER]);
    assert_eq!(call(&mut e, 0x2336, &[rf(LIST)]).1, 0.0);
}

#[test]
fn water_traits() {
    let mut e = setup();
    e.mem.set_u8(WATER + 0x70, 40);
    e.mem.set_u8(WATER + 0x71, 2);
    e.mem.set_f32(WATER + 0x84, 0.25);
    e.mem.set_u32(WATER + 0x84 + 4 * 10, 0x0003_0201);
    e.mem.set_f32(WATER + 0x84 + 4 * 48, 9.5);
    let get = |e: &mut Engine, id: i32| call(e, 0x2339, &[rf(WATER), n(id)]).1;
    assert_eq!(get(&mut e, 0), 40.0);
    assert_eq!(get(&mut e, 1), 2.0);
    assert_eq!(get(&mut e, 2), 0.25);
    assert_eq!(get(&mut e, 12), 1_002_003.0);
    e.mem.set_u32(WATER + 0x84 + 4 * 12, 0x0001_0203);
    assert_eq!(get(&mut e, 14), 3_002_001.0, "the last colour");
    assert_eq!(get(&mut e, 50), 9.5);
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(get(&mut e, 51), 0.0, "past 50");
    let set = |e: &mut Engine, id: i32, v: f64| {
        call(e, 0x233a, &[rf(WATER), n(id), z(v)]);
    };
    set(&mut e, 0, 100.9);
    assert_eq!(e.mem.u8(WATER + 0x70), 100);
    set(&mut e, 0, 101.0);
    assert_eq!(e.mem.u8(WATER + 0x70), 100, "above 100");
    set(&mut e, 0, -1.0);
    assert_eq!(e.mem.u8(WATER + 0x70), 100, "negative is a large unsigned");
    set(&mut e, 1, 3.0);
    assert_eq!(e.mem.u8(WATER + 0x71), 3);
    set(&mut e, 1, 4.0);
    assert_eq!(e.mem.u8(WATER + 0x71), 3);
    set(&mut e, 14, 255_255_255.0);
    assert_eq!(e.mem.u32(WATER + 0x84 + 4 * 12), 0x00ff_ffff);
    set(&mut e, 14, 255_255_256.0);
    assert_eq!(e.mem.u32(WATER + 0x84 + 4 * 12), 0x00ff_ffff);
    set(&mut e, 3, -2.5);
    assert_eq!(e.mem.f32(WATER + 0x88), -2.5);
    set(&mut e, 51, 1.0);
    assert_eq!(e.mem.f32(WATER + 0x84 + 4 * 49), 0.0, "past 50");
    // Not a water form: nothing.
    e.mem.set_u8(SOUND + 0x70, 7);
    call(&mut e, 0x233a, &[rf(SOUND), n(0), z(1.0)]);
    assert_eq!(e.mem.u8(SOUND + 0x70), 7);
    e.mem.set_f64(RESULT, 0.0);
    assert_eq!(call(&mut e, 0x2339, &[rf(SOUND), n(0)]).1, 0.0);
}
