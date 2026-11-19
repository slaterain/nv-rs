//! `CCCOnLoad`, `CCCSetFloat`, `CCCSetString`, `CCCSetTrait`, `CCCSavedForm` and `CCCSMS`, called
//! by their opcodes. Expectations from JIP's bodies (functions_jip/jip_fn_ccc.h; the tile walk
//! from `Cmd_CCCOnLoad_Execute` line by line), never from the port's output. The HUD component
//! is built as JIP walks it: a tile's children are counted from the last (`DList::Tail`, then
//! `prev`).

use super::*;
use crate::commands::test_support::*;
use std::cell::{Cell, RefCell};

const AREA: u32 = 0x0600_0000;
const MAP_VT: u32 = AREA;
const BUCKET: u32 = AREA + 0x10;
const HASH_FN: u32 = 0x0dea_6000;
const EQUAL_FN: u32 = 0x0dea_6004;
const REFRESH_FN: u32 = 0x00a0_9410;
const CONSOLE: u32 = AREA + 0x20;
const VERSION: u32 = FORMS + 0x1100;
const FLAG: u32 = FORMS + 0x1200;
const FACTION: u32 = FORMS + 0x1300;
const PACKAGE: u32 = FORMS + 0x1400;
const LOCATION: u32 = FORMS + 0x1480;
const NPC: u32 = FORMS + 0x2000;
const NPC_BASE: u32 = FORMS + 0x2400;
const PROC: u32 = FORMS + 0x2800;
const AV_VT: u32 = FORMS + 0x2c00;
const NAME_TEXT: u32 = FORMS + 0x2e00;
const MOD_INFOS: u32 = FORMS + 0x3_0000;
const EFFECTS: u32 = FORMS + 0x4000;
const SHADER: u32 = FORMS + 0x4800;
const NODES: u32 = FORMS + 0x4c00;
const GET_AV_FN: u32 = 0x0dea_6010;
const GET_BASE_AV_FN: u32 = 0x0dea_6014;
/// CC&C's file index.
const CCC: u32 = 7;

thread_local! {
    static NEXT: Cell<u32> = const { Cell::new(0) };
    static TRAITS: RefCell<Vec<(Vec<u8>, u32)>> = const { RefCell::new(Vec::new()) };
    static PRINTED: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    /// A HUD part `hud_scene` leaves out (as another HUD mod that renamed it), and whether the
    /// HUD root has no `_DUIF3` value.
    static SKIP_PART: Cell<Option<&'static str>> = const { Cell::new(None) };
    static NO_DUIF3: Cell<bool> = const { Cell::new(false) };
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn alloc(size: u32) -> u32 {
    NEXT.with(|n| {
        let at = n.get();
        n.set(at + size.next_multiple_of(0x10));
        at
    })
}

/// The id of the trait `name` (the game's own ids for `x`, `y`, `visible`, `string` and
/// `filename`; the others from 0x3000), put in the stand-in trait map.
fn trait_of(e: &mut Engine, name: &[u8]) -> u32 {
    if let Some(id) = TRAITS.with(|t| t.borrow().iter().find(|(n, _)| n == name).map(|x| x.1)) {
        return id;
    }
    let id = match name {
        b"x" => 0xfa1,
        b"y" => 0xfa2,
        b"visible" => 0xfa3,
        b"string" => 0xfc4,
        b"filename" => 0xfcc,
        _ => 0x3000 + TRAITS.with(|t| t.borrow().len() as u32),
    };
    let key = alloc(name.len() as u32 + 1);
    e.mem.write(key, name);
    let item = alloc(12);
    e.mem.set_u32(item, e.mem.u32(BUCKET));
    e.mem.set_u32(item + 4, key);
    e.mem.set_u32(item + 8, id);
    e.mem.set_u32(BUCKET, item);
    TRAITS.with(|t| t.borrow_mut().push((name.to_vec(), id)));
    id
}

/// A new tile under `parent` (appended to its children), named `name` when given.
fn new_tile(e: &mut Engine, parent: u32, name: Option<&[u8]>) -> u32 {
    let tile = alloc(0x40);
    e.mem.set_u32(tile + 0x28, parent);
    if let Some(name) = name {
        let text = alloc(name.len() as u32 + 1);
        e.mem.write(text, name);
        e.mem.set_u32(tile + 0x20, text);
    }
    if parent != 0 {
        let node = alloc(12);
        let tail = e.mem.u32(parent + 8);
        e.mem.set_u32(node + 4, tail);
        e.mem.set_u32(node + 8, tile);
        if tail != 0 {
            e.mem.set_u32(tail, node);
        } else {
            e.mem.set_u32(parent + 4, node);
        }
        e.mem.set_u32(parent + 8, node);
    }
    tile
}

/// `n` children of `parent`, in the order JIP walks them (the last child first).
fn kids(e: &mut Engine, parent: u32, n: usize) -> Vec<u32> {
    let mut made: Vec<u32> = (0..n).map(|_| new_tile(e, parent, None)).collect();
    made.reverse();
    made
}

/// The tile's values of the traits named, sorted by id; each value's address in `want` at the
/// index given.
fn values(e: &mut Engine, tile: u32, list: &[(&[u8], Option<usize>)], want: &mut [u32]) {
    let mut made: Vec<(u32, u32)> = list
        .iter()
        .map(|(name, index)| {
            let id = trait_of(e, name);
            let v = alloc(0x10);
            e.mem.set_u32(v, id);
            if let Some(i) = index {
                want[*i] = v;
            }
            (id, v)
        })
        .collect();
    made.sort();
    let data = alloc(4 * made.len() as u32);
    for (i, (_, v)) in made.iter().enumerate() {
        e.mem.set_u32(data + 4 * i as u32, *v);
    }
    e.mem.set_u32(tile + 0x14, data);
    e.mem.set_u32(tile + 0x18, made.len() as u32);
}

/// The HUD with CC&C's component as JIP walks it; the 119 values' addresses by index.
fn hud_scene(e: &mut Engine, duif3: bool) -> Vec<u32> {
    let mut want = vec![0u32; UI_ELEMENTS];
    let root = new_tile(e, 0, None);
    let menu = alloc(0x10);
    e.mem.set_u32(menu + 4, root);
    e.mem.set_u32(0x011d_96c0, menu);
    let display = new_tile(e, root, Some(b"JIPCCCHUDDisplay"));
    values(
        e,
        display,
        &[
            (b"_AltStyle", Some(0)),
            (b"_SelectMode", Some(1)),
            (b"_Adjust", Some(2)),
        ],
        &mut want,
    );
    let parts = kids(e, display, 5);
    let (a, b, c, d, f) = (parts[0], parts[1], parts[2], parts[3], parts[4]);
    values(
        e,
        a,
        &[
            (b"_TotalAvatars", Some(3)),
            (b"_NumHired", Some(4)),
            (b"_Layout", Some(5)),
            (b"_Size", Some(6)),
            (b"_Anchor", Some(7)),
            (b"_Selected", Some(8)),
            (b"visible", Some(9)),
        ],
        &mut want,
    );
    for (i, slot) in kids(e, a, 10).into_iter().enumerate() {
        values(
            e,
            slot,
            &[
                (b"_HealthFill", Some(10 + i)),
                (b"_Distance", Some(20 + i)),
                (b"_Action", Some(30 + i)),
            ],
            &mut want,
        );
        let inner = kids(e, slot, 5);
        values(e, inner[2], &[(b"filename", Some(40 + i))], &mut want);
        values(e, inner[4], &[(b"string", Some(50 + i))], &mut want);
    }
    values(
        e,
        b,
        &[
            (b"_Size", Some(60)),
            (b"_Selected", Some(61)),
            (b"_Anchor", Some(62)),
            (b"_IsWaiting", Some(63)),
            (b"_IsFollowingLong", Some(64)),
            (b"_IsUsingMelee", Some(65)),
            (b"_IsAggressive", Some(66)),
            (b"_LabelsON", Some(67)),
            (b"x", Some(68)),
        ],
        &mut want,
    );
    let bk = kids(e, b, 9);
    for (i, &k) in bk[3..8].iter().enumerate() {
        values(e, k, &[(b"_CmdType", Some(69 + i))], &mut want);
    }
    values(e, bk[8], &[(b"string", Some(74))], &mut want);
    values(
        e,
        c,
        &[
            (b"_Menu", Some(75)),
            (b"_Selected", Some(76)),
            (b"_HasHome", Some(77)),
        ],
        &mut want,
    );
    let ck = kids(e, c, 6);
    let k3 = kids(e, ck[3], 7);
    for (i, &k) in k3[..3].iter().enumerate() {
        values(e, k, &[(b"_Value", Some(78 + i))], &mut want);
    }
    values(e, k3[6], &[(b"string", Some(81))], &mut want);
    let k5 = kids(e, ck[5], 29);
    values(e, k5[0], &[(b"string", Some(82))], &mut want);
    values(e, k5[1], &[(b"string", Some(83))], &mut want);
    for (i, &k) in k5[2..28].iter().enumerate() {
        values(e, k, &[(b"_Value", Some(84 + i))], &mut want);
    }
    let last = kids(e, k5[28], 1)[0];
    values(e, last, &[(b"string", Some(110))], &mut want);
    values(
        e,
        d,
        &[(b"_Targeting", Some(111)), (b"string", Some(112))],
        &mut want,
    );
    let fk = kids(e, f, 1)[0];
    values(e, fk, &[(b"string", Some(113))], &mut want);
    // The HUD's own parts, as the two layouts name them.
    let part = |e: &mut Engine, path: &[&str], trait_: &[u8], index: usize, want: &mut [u32]| {
        if SKIP_PART.with(|s| s.get()) == path.last().copied() {
            return;
        }
        let mut at = root;
        for name in path {
            at = new_tile(e, at, Some(name.as_bytes()));
        }
        values(e, at, &[(trait_, Some(index))], want);
    };
    part(e, &["SneakMeter"], b"y", 114, &mut want);
    if duif3 {
        part(e, &["QuestWrap", "QuestReminder"], b"x", 115, &mut want);
        part(e, &["XPWrap"], b"x", 116, &mut want);
        part(e, &["EH_Wrap", "EnemyHealth"], b"y", 118, &mut want);
        part(e, &["InfoWrap", "Info"], b"y", 117, &mut want);
    } else {
        part(e, &["QuestReminder"], b"x", 115, &mut want);
        part(e, &["XPMeter"], b"x", 116, &mut want);
        part(e, &["Info"], b"y", 117, &mut want);
        part(e, &["EnemyHealth"], b"y", 118, &mut want);
    }
    if !NO_DUIF3.with(|d| d.get()) {
        let mut dummy = vec![0u32; 1];
        values(e, root, &[(b"_DUIF3", Some(0))], &mut dummy);
        e.mem.set_f32(dummy[0] + 8, if duif3 { 1.0 } else { 0.0 });
    }
    want
}

fn setup() -> Engine {
    NEXT.with(|n| n.set(AREA + 0x100));
    TRAITS.with(|t| t.borrow_mut().clear());
    PRINTED.with(|p| p.borrow_mut().clear());
    SKIP_PART.with(|s| s.set(None));
    NO_DUIF3.with(|d| d.set(false));
    files::fresh();
    let mut e = engine();
    game_globals(&mut e);
    e.map(AREA, 0x10_0000);
    e.map(0x011d_9000, 0x1000);
    e.map(0x011d_e000, 0x1000);
    e.map(0x011e_0000, 0x1000);
    if !e.mem.is_mapped(0x011f_3000) {
        e.map(0x011f_3000, 0x1000);
    }
    e.mem.set_u32(0x011f_32f4, MAP_VT);
    e.mem.set_u32(0x011f_32f4 + 4, 1);
    e.mem.set_u32(0x011f_32f4 + 8, BUCKET);
    e.mem.set_u32(MAP_VT + 4, HASH_FN);
    e.mem.set_u32(MAP_VT + 8, EQUAL_FN);
    e.register(HASH_FN, |_, _| eax(0));
    e.register(EQUAL_FN, |e, w| {
        let (a, b) = (
            nvse::table::guest_text(e, w[1]),
            nvse::table::guest_text(e, w[2]),
        );
        eax(u32::from(a == b))
    });
    e.register(REFRESH_FN, |_, w| {
        CALLS.with(|c| c.borrow_mut().push((REFRESH_FN, w.to_vec())));
        eax(0)
    });
    e.register(0x0071_b160, |_, _| eax(CONSOLE));
    e.register(0x0071_d0a0, |e, w| {
        let line = nvse::table::guest_text(e, w[1]);
        PRINTED.with(|p| p.borrow_mut().push(line));
        eax(0)
    });
    e.register(SHOW_MESSAGE_BOX, |e, w| {
        assert_eq!(w.len(), 10, "ShowMessageBox's words");
        let mut words = w.to_vec();
        words[0] = u32::from(nvse::table::guest_text(e, w[0]) == OLD_VERSION);
        words[8] = u32::from(nvse::table::guest_text(e, w[8]) == b"OK");
        CALLS.with(|c| c.borrow_mut().push((SHOW_MESSAGE_BOX, words)));
        eax(0)
    });
    // The calling script is CC&C's; its version global 2.60.
    e.mem.set_u8(crate::test_support::SCRIPT + 0xf, CCC as u8);
    form(&mut e, VERSION, 6, (CCC << 24) | 0x5000);
    e.mem.set_f32(VERSION + 0x24, 260.0);
    in_form_map(&mut e, VERSION, (CCC << 24) | 0x5000, 0);
    form(&mut e, FLAG, 6, (CCC << 24) | 0x8f6d);
    in_form_map(&mut e, FLAG, (CCC << 24) | 0x8f6d, 1);
    form(&mut e, FACTION, 8, (CCC << 24) | 0x2d501);
    in_form_map(&mut e, FACTION, (CCC << 24) | 0x2d501, 2);
    e
}

fn on_load(e: &mut Engine) -> u32 {
    call(e, 0x26fd, &[]).0
}

fn printed() -> Vec<Vec<u8>> {
    PRINTED.with(|p| p.borrow().clone())
}

/// The 119 values in JIP's order, found once; a second call does nothing.
#[test]
fn on_load_finds_the_hud_values_in_jips_order() {
    for duif3 in [false, true] {
        let mut e = setup();
        let want = hud_scene(&mut e, duif3);
        assert_eq!(on_load(&mut e), 1);
        let h = &e.ext::<Jip>().ccc.hud;
        assert!(h.loaded, "duif3 {duif3}");
        assert_eq!(h.elements, want, "duif3 {duif3}");
        assert_eq!(h.mod_index, CCC as u8);
        assert_eq!(h.task_faction, FACTION);
        assert!(printed().is_empty());
        // Once a session: a second call reads nothing and changes nothing.
        let before = calls().len();
        assert_eq!(on_load(&mut e), 1);
        assert_eq!(calls().len(), before, "duif3 {duif3}: nothing called");
        assert_eq!(e.ext::<Jip>().ccc.hud.elements, want);
        assert!(printed().is_empty());
    }
}

/// A tile missing on the way (the display's last part without its child, where value 113 is):
/// the load error, the values unused.
#[test]
fn a_missing_tile_is_the_load_error() {
    let mut e = setup();
    hud_scene(&mut e, false);
    let root = e.mem.u32(e.mem.u32(0x011d_96c0) + 4);
    let display = e.mem.u32(e.mem.u32(root + 4) + 8);
    let first_part = e.mem.u32(e.mem.u32(display + 4) + 8);
    e.mem.set_u32(first_part + 4, 0);
    e.mem.set_u32(first_part + 8, 0);
    assert_eq!(on_load(&mut e), 1);
    assert!(!e.ext::<Jip>().ccc.hud.loaded);
    assert_eq!(printed(), vec![LOAD_ERROR.to_vec()]);
    // Once a session: no second try.
    on_load(&mut e);
    assert_eq!(printed().len(), 1);
}

/// No component at all: the load error; the setters do nothing.
#[test]
fn no_component_is_the_load_error() {
    let mut e = setup();
    let root = new_tile(&mut e, 0, None);
    let menu = alloc(0x10);
    e.mem.set_u32(menu + 4, root);
    e.mem.set_u32(0x011d_96c0, menu);
    assert_eq!(on_load(&mut e), 1);
    assert_eq!(printed(), vec![LOAD_ERROR.to_vec()]);
    call(&mut e, 0x26fe, &[n(3), z(2.0)]);
    assert!(calls_to(REFRESH_FN).is_empty());
}

/// Older than 2.60: the flag global set to 1, the message box, the command fails, nothing else.
#[test]
fn an_old_version_says_so_and_fails() {
    let mut e = setup();
    hud_scene(&mut e, false);
    e.mem.set_f32(VERSION + 0x24, 259.0);
    assert_eq!(on_load(&mut e), 0, "fails");
    assert_eq!(e.mem.f32(FLAG + 0x24), 1.0);
    assert_eq!(
        calls_to(SHOW_MESSAGE_BOX),
        vec![vec![1, 0, 0, MESSAGE_BOX_CALLBACK, 0, 0x17, 0, 0, 1, 0]]
    );
    assert_eq!(e.ext::<Jip>().ccc.hud.mod_index, CCC as u8);
    // Once a session: the second call does nothing and succeeds.
    assert_eq!(on_load(&mut e), 1);
    assert_eq!(calls_to(SHOW_MESSAGE_BOX).len(), 1);
    assert!(!e.ext::<Jip>().ccc.hud.loaded);
}

/// The saved forms of the companion mods loaded; Companion Core's package radius 0.
#[test]
fn saved_forms_of_the_mods_loaded() {
    let mut e = setup();
    hud_scene(&mut e, false);
    loaded_mod(&mut e, 3, MOD_INFOS, b"Companion Core.esm");
    loaded_mod(&mut e, 9, MOD_INFOS + 0x800, b"EZ_CompanionNVSE.esm");
    loaded_mod(&mut e, 5, MOD_INFOS + 0x1000, b"NosCo Companion System.esm");
    form(&mut e, PACKAGE, 0x49, 0x0300_080e);
    in_form_map(&mut e, PACKAGE, 0x0300_080e, 3);
    e.mem.set_u32(PACKAGE + 0x2c, LOCATION);
    e.mem.set_u32(LOCATION + 4, 500);
    let saved = |e: &mut Engine, i: i32| {
        call(e, 0x2708, &[n(i)]);
        e.mem.u32(RESULT)
    };
    assert_eq!(saved(&mut e, 0), 0, "before CCCOnLoad");
    on_load(&mut e);
    let all: Vec<u32> = (0..10).map(|i| saved(&mut e, i)).collect();
    assert_eq!(
        all,
        [
            0x0300_0812,
            0x0300_0810,
            0x0300_950d,
            0x0305_b3b4,
            0x0500_0ae0,
            0x0500_0ae2,
            0x0500_86a7,
            0x0500_0ae5,
            0x0500_0add,
            0x0900_0dc3
        ]
    );
    assert_eq!(e.mem.u32(LOCATION + 4), 0);
    e.mem.set_u32(RESULT, 5);
    call(&mut e, 0x2708, &[n(10)]);
    assert_eq!(e.mem.u32(RESULT), 0, "past the ten: the result as given");
}

fn loaded(duif3: bool) -> (Engine, Vec<u32>) {
    let mut e = setup();
    let want = hud_scene(&mut e, duif3);
    on_load(&mut e);
    assert!(e.ext::<Jip>().ccc.hud.loaded);
    (e, want)
}

fn text_of(e: &Engine, value: u32) -> Vec<u8> {
    nvse::table::guest_text(e, e.mem.u32(value + 0xc))
}

#[test]
fn set_float_and_set_string_write_a_value() {
    let (mut e, want) = loaded(false);
    call(&mut e, 0x26fe, &[n(68), z(12.5)]);
    assert_eq!(e.mem.f32(want[68] + 8), 12.5);
    assert_eq!(calls_to(REFRESH_FN), vec![vec![want[68], 1]]);
    call(&mut e, 0x26ff, &[n(112), s(b"Boone")]);
    assert_eq!(text_of(&e, want[112]), b"Boone");
    assert_eq!(
        call(&mut e, 0x26fe, &[n(119), z(1.0)]).0,
        1,
        "past the 119: handled, nothing written"
    );
    call(&mut e, 0x26fe, &[n(-1), z(1.0)]);
    assert_eq!(calls_to(REFRESH_FN).len(), 2, "past the values: nothing");
    // Another trait: value trait + slot.
    call(&mut e, 0x2700, &[n(60), n(1), n(7)]);
    assert_eq!(e.mem.f32(want[61] + 8), 7.0);
}

/// An actor of `name` (female when asked; a creature when asked) calling slot 2.
fn named_npc(e: &mut Engine, name: &[u8], female: bool, creature: bool) {
    form(e, NPC, 0x3b, 0x7000);
    e.mem.set_u32(NPC, FORMS + 0x5000);
    e.mem.set_u32(FORMS + 0x5000 + 0x100, 0x008d_0360);
    e.mem.set_u32(NPC + 0x20, NPC_BASE);
    form(e, NPC_BASE, if creature { 0x2b } else { 0x2a }, 0x7100);
    e.mem.set_u32(NPC_BASE + 0x34, u32::from(female));
    e.mem.write(NAME_TEXT, name);
    e.mem.set_u8(NAME_TEXT + name.len() as u32, 0);
    e.mem.set_u32(NPC_BASE + 0xd4, NAME_TEXT);
}

/// The avatar pictures: the ini file's lines (`name=picture`), the folder's `avatar_*.dds`; a
/// female without one, a creature by a common picture in its name, the male picture otherwise.
#[test]
fn avatars_by_name_folder_and_kind() {
    let mut e = setup();
    let want = hud_scene(&mut e, false);
    let mut ini = b"Boone=boone\r\nCass=cass\n\n=empty\n".to_vec();
    for i in 0..130 {
        ini.extend_from_slice(format!("filler{i}=f\n").as_bytes());
    }
    ini.extend_from_slice(b"Cyberdog=dog\nNightkin=nk\n");
    files::put(r"data\config\ccc_avatars.ini", &ini);
    files::put(r"data\textures\jazzisparis\ccc\avatar_Veronica.dds", b"");
    files::put(r"data\textures\jazzisparis\ccc\other.dds", b"");
    on_load(&mut e);
    let avatar = |e: &mut Engine, name: &[u8], female: bool, creature: bool| {
        named_npc(e, name, female, creature);
        e.ext::<Jip>().ccc.hud.path_for_id.clear();
        call_on(e, 0x2700, NPC, &[n(40), n(2)]);
        (text_of(e, want[42]), text_of(e, want[52]))
    };
    assert_eq!(
        avatar(&mut e, b"boone", false, false),
        (
            br"jazzisparis\ccc\avatar_boone.dds".to_vec(),
            b"boone".to_vec()
        ),
        "by name, without case"
    );
    assert_eq!(
        avatar(&mut e, b"Veronica", false, false).0,
        br"jazzisparis\ccc\avatar_Veronica.dds",
        "the folder's file (names as listed)"
    );
    assert_eq!(avatar(&mut e, b"Rose", true, false).0, AVATAR_FEMALE);
    assert_eq!(avatar(&mut e, b"Rex", false, false).0, AVATAR_MALE);
    assert_eq!(
        avatar(&mut e, b"Rex the Cyberdog", false, true).0,
        br"jazzisparis\ccc\avatar_dog.dds",
        "a creature: a common picture in its name"
    );
    assert_eq!(
        avatar(&mut e, b"Rex the cyberdog", false, true).0,
        AVATAR_MALE,
        "the name's case counts there"
    );
    // Chosen once a base: a later file does not change it.
    named_npc(&mut e, b"Rose", true, false);
    e.ext::<Jip>().ccc.hud.path_for_id.clear();
    call_on(&mut e, 0x2700, NPC, &[n(40), n(1)]);
    e.ext::<Jip>()
        .ccc
        .hud
        .avatar_paths
        .insert(crate::containers::str_hash_ci(b"Rose"), b"x".to_vec());
    call_on(&mut e, 0x2700, NPC, &[n(40), n(1)]);
    assert_eq!(text_of(&e, want[41]), AVATAR_FEMALE);
    // No actor: the male picture, the name left.
    call(&mut e, 0x2700, &[n(40), n(3)]);
    assert_eq!(text_of(&e, want[43]), AVATAR_MALE);
    assert_eq!(e.mem.u32(want[53] + 0xc), 0);
    // An empty name: nothing.
    named_npc(&mut e, b"", false, false);
    call_on(&mut e, 0x2700, NPC, &[n(40), n(4)]);
    assert_eq!(e.mem.u32(want[44] + 0xc), 0);
}

/// Trait 0: the health bar (64 x health / base health), far from the player, the action.
#[test]
fn trait_0_health_distance_and_action() {
    let (mut e, want) = loaded(false);
    // The player: a reference in no cell.
    e.mem.set_u32(0x011d_ea3c, FORMS + 0x6000);
    extra_stand_ins(&mut e);
    named_npc(&mut e, b"Boone", false, false);
    e.mem.set_u32(NPC + 0xa4, AV_VT);
    e.mem.set_u32(AV_VT + 0xc, GET_AV_FN);
    e.mem.set_u32(AV_VT + 4, GET_BASE_AV_FN);
    e.register(GET_AV_FN, |_, w| {
        assert_eq!(w[1], 0x10);
        f32::into_ret(30.0)
    });
    e.register(GET_BASE_AV_FN, |_, w| {
        assert_eq!(w[1], 0x10, "base health");
        f32::into_ret(120.0)
    });
    e.mem.set_u32(NPC + 0x68, PROC);
    e.mem.set_u32(PROC + 4, PACKAGE);
    e.mem.set_u32(PROC + 0xc, 0x0bad_0000);
    e.mem.set_u8(PACKAGE + 0x20, 1);
    // Not in a cell: the largest distance, so far.
    call_on(&mut e, 0x2700, NPC, &[n(0), n(5)]);
    assert_eq!(e.mem.f32(want[15] + 8), 64.0 * 30.0 / 120.0);
    assert_eq!(e.mem.f32(want[25] + 8), 1.0, "far");
    assert!(
        calls_to(REFRESH_FN).iter().all(|c| c[0] != want[35]),
        "a follow package on someone else: no action"
    );
    // The package on the player: 1.
    e.mem.set_u32(PROC + 0xc, e.mem.u32(0x011d_ea3c));
    call_on(&mut e, 0x2700, NPC, &[n(0), n(5)]);
    assert_eq!(e.mem.f32(want[35] + 8), 1.0);
    // Waiting 2, sandbox 6, use weapon 13.
    for (kind, action) in [(9u8, 2.0f32), (22, 6.0), (12, 13.0)] {
        e.mem.set_u8(PACKAGE + 0x20, kind);
        call_on(&mut e, 0x2700, NPC, &[n(0), n(5)]);
        assert_eq!(e.mem.f32(want[35] + 8), action, "{kind}");
    }
    // Knocked out: -1, the distance left.
    e.mem.set_u8(PROC + 0x13c, 3);
    call_on(&mut e, 0x2700, NPC, &[n(0), n(6)]);
    assert_eq!(e.mem.f32(want[16] + 8), -1.0);
    assert_eq!(e.mem.f32(want[26] + 8), 0.0);
}

/// `CCCSMS`: CC&C's shader effects on the reference told to stop; others left.
#[test]
fn sms_stops_cccs_shaders_on_the_reference() {
    let (mut e, _) = loaded(false);
    named_npc(&mut e, b"Boone", false, false);
    set_ref(&mut e, 1, NPC);
    form(&mut e, SHADER, 0x55, (CCC << 24) | 0x900);
    form(&mut e, SHADER + 0x40, 0x55, 0x0000_0900);
    let effect = |e: &mut Engine, i: u32, vt: u32, target: u32, shader: u32| {
        let at = EFFECTS + 0x80 * i;
        e.mem.set_u32(at, vt);
        e.mem.set_u32(at + 0x1c, target);
        e.mem.set_u32(at + 0x30, shader);
        at
    };
    let vt = MAGIC_SHADER_HIT_EFFECT_VTBL;
    let mine = effect(&mut e, 0, vt, NPC, SHADER);
    let other_mod = effect(&mut e, 1, vt, NPC, SHADER + 0x40);
    let other_ref = effect(&mut e, 2, vt, FORMS + 0x9000, SHADER);
    let other_kind = effect(&mut e, 3, vt + 4, NPC, SHADER);
    let mine_too = effect(&mut e, 4, vt, NPC, SHADER);
    // tempEffects: the inline first node, then four more; CC&C's effects second and last.
    let list = [other_mod, mine, other_ref, other_kind, mine_too];
    e.mem.set_u32(0x011e_0e80 + 0x60, list[0]);
    let mut prev = 0x011e_0e80 + 0x60;
    for (i, &x) in list[1..].iter().enumerate() {
        let node = NODES + 8 * i as u32;
        e.mem.set_u32(node, x);
        e.mem.set_u32(prev + 4, node);
        prev = node;
    }
    call(&mut e, 0x2771, &[r(1)]);
    let flags: Vec<u8> = list.iter().map(|&x| e.mem.u8(x + 0x24)).collect();
    assert_eq!(flags, [0, 1, 0, 0, 1]);
}

/// A HUD value missing while every tile is there (another HUD mod renamed `SneakMeter`): the
/// load error, the values unused, `CCCSetFloat 114` writes nothing.
#[test]
fn a_hud_value_missing_is_the_load_error() {
    let mut e = setup();
    SKIP_PART.with(|s| s.set(Some("SneakMeter")));
    hud_scene(&mut e, false);
    assert_eq!(on_load(&mut e), 1);
    assert!(!e.ext::<Jip>().ccc.hud.loaded);
    assert_eq!(printed(), vec![LOAD_ERROR.to_vec()]);
    call(&mut e, 0x26fe, &[n(114), z(1.0)]);
    assert!(calls_to(REFRESH_FN).is_empty());
}

/// A HUD root without `_DUIF3`: the load error.
#[test]
fn no_duif3_is_the_load_error() {
    for duif3 in [false, true] {
        let mut e = setup();
        NO_DUIF3.with(|d| d.set(true));
        hud_scene(&mut e, duif3);
        assert_eq!(on_load(&mut e), 1);
        assert!(!e.ext::<Jip>().ccc.hud.loaded, "duif3 {duif3}");
        assert_eq!(printed(), vec![LOAD_ERROR.to_vec()]);
    }
}

/// No HUD tile (the HUD menu's tile 0, or no HUD menu): the load error, not a read through it.
#[test]
fn no_hud_root_is_the_load_error() {
    let mut e = setup();
    let menu = alloc(0x10);
    e.mem.set_u32(0x011d_96c0, menu);
    assert_eq!(on_load(&mut e), 1);
    assert_eq!(printed(), vec![LOAD_ERROR.to_vec()]);
    let mut e = setup();
    e.mem.set_u32(0x011d_96c0, 0);
    assert_eq!(on_load(&mut e), 1);
    assert_eq!(printed(), vec![LOAD_ERROR.to_vec()]);
}

/// JIP's `LineIterator`: CR and LF end lines only before the file's first 0 byte; after it only
/// 0 bytes do, so a CR LF stays inside its piece; a piece starting with byte 3 ends the walk.
#[test]
fn lines_as_jips_line_iterator() {
    assert_eq!(
        lines(b"a=b\r\n\r\nc=d"),
        vec![b"a=b".to_vec(), b"c=d".to_vec()]
    );
    assert_eq!(
        lines(b"a=b\0x=y\r\nc=d\0\0e=f"),
        vec![b"a=b".to_vec(), b"x=y\r\nc=d".to_vec(), b"e=f".to_vec()]
    );
    assert_eq!(lines(b"a=b\n\x03rest\nc=d"), vec![b"a=b".to_vec()]);
}

/// The ini file's first 129 lines are found by name, the 130th and later only as common
/// pictures (in a creature's name, in JIP's case-blind key order, a key given twice keeping the
/// later path); the folder's `avatar_*.dds` files without case (the name as listed), one with
/// no underscore skipped, and a folder file over the ini's line of the same name.
#[test]
fn avatar_ini_split_order_and_folder() {
    let mut e = setup();
    let want = hud_scene(&mut e, false);
    let mut ini = Vec::new();
    for i in 0..127 {
        ini.extend_from_slice(format!("named{i}=n{i}\n").as_bytes());
    }
    // Lines 127 and 128 (the 128th and 129th): by name. Line 129: the first common one.
    ini.extend_from_slice(b"Boone=ini_boone\nLastNamed=last\nFirstCommon=first\n");
    ini.extend_from_slice(b"Golden Gecko=golden\nGecko=gecko\nDeathclaw=dc1\ndeathclaw=dc2\n");
    files::put(r"data\config\ccc_avatars.ini", &ini);
    files::put(r"data\textures\jazzisparis\ccc\AVATAR_Rex.DDS", b"");
    files::put(r"data\textures\jazzisparis\ccc\avatarX.dds", b"");
    files::put(r"data\textures\jazzisparis\ccc\avatar_Boone.dds", b"");
    on_load(&mut e);
    let avatar = |e: &mut Engine, name: &[u8], female: bool, creature: bool| {
        named_npc(e, name, female, creature);
        e.ext::<Jip>().ccc.hud.path_for_id.clear();
        call_on(e, 0x2700, NPC, &[n(40), n(2)]);
        text_of(e, want[42])
    };
    assert_eq!(
        avatar(&mut e, b"named126", false, false),
        br"jazzisparis\ccc\avatar_n126.dds"
    );
    assert_eq!(
        avatar(&mut e, b"LastNamed", false, false),
        br"jazzisparis\ccc\avatar_last.dds",
        "line 128: by name"
    );
    assert_eq!(
        avatar(&mut e, b"FirstCommon", false, false),
        AVATAR_MALE,
        "line 129: not by name"
    );
    assert_eq!(
        avatar(&mut e, b"The FirstCommon", false, true),
        br"jazzisparis\ccc\avatar_first.dds",
        "line 129: common, in a creature's name"
    );
    // "Gecko" sorts before "Golden Gecko" (StrCompareCI): a Golden Gecko gets the gecko picture.
    assert_eq!(
        avatar(&mut e, b"Golden Gecko", false, true),
        br"jazzisparis\ccc\avatar_gecko.dds"
    );
    assert_eq!(
        avatar(&mut e, b"Deathclaw", true, true),
        br"jazzisparis\ccc\avatar_dc2.dds",
        "a key given twice keeps the later path; a creature's female bit is not read"
    );
    assert_eq!(
        avatar(&mut e, b"Mole Rat", true, true),
        AVATAR_MALE,
        "a female creature without a picture"
    );
    assert_eq!(
        avatar(&mut e, b"Rex", false, false),
        br"jazzisparis\ccc\AVATAR_Rex.DDS",
        "the folder without case, the name as listed"
    );
    assert_eq!(
        avatar(&mut e, b"X", false, false),
        AVATAR_MALE,
        "no underscore: skipped"
    );
    assert_eq!(
        avatar(&mut e, b"Boone", false, false),
        br"jazzisparis\ccc\avatar_Boone.dds",
        "the folder over the ini"
    );
}

/// The actor and its avatar and health stand-ins for the trait 0 tests, the player at `player`.
fn trait_0_actor(e: &mut Engine, player: u32) {
    e.mem.set_u32(0x011d_ea3c, player);
    extra_stand_ins(e);
    named_npc(e, b"Boone", false, false);
    e.mem.set_u32(NPC + 0xa4, AV_VT);
    e.mem.set_u32(AV_VT + 0xc, GET_AV_FN);
    e.mem.set_u32(AV_VT + 4, GET_BASE_AV_FN);
    e.register(GET_AV_FN, |_, _| f32::into_ret(30.0));
    e.register(GET_BASE_AV_FN, |_, _| f32::into_ret(120.0));
}

/// Trait 0's far mark in the same cell: 8000 units away not far (0), 8200 far (1).
#[test]
fn trait_0_far_at_8192_units() {
    let (mut e, want) = loaded(false);
    let player = FORMS + 0x6000;
    let cell = FORMS + 0x6800;
    trait_0_actor(&mut e, player);
    e.mem.set_u32(player + 0x40, cell);
    e.mem.set_u32(NPC + 0x40, cell);
    for (x, far) in [(8000.0f32, 0.0f32), (8200.0, 1.0), (8000.0, 0.0)] {
        e.mem.set_f32(NPC + 0x30, x);
        call_on(&mut e, 0x2700, NPC, &[n(0), n(5)]);
        assert_eq!(e.mem.f32(want[25] + 8), far, "{x}");
    }
}

thread_local! {
    static FLEE: Cell<u32> = const { Cell::new(0) };
}

/// Trait 0's action for every package kind JIP lists, a fleeing package by its flee flag, an
/// interrupt package over the current one, and the task faction's rank from 7.
#[test]
fn trait_0_actions() {
    let (mut e, want) = loaded(false);
    let player = FORMS + 0x6000;
    trait_0_actor(&mut e, player);
    e.register(GET_FLEE_COMBAT, |_, w| {
        assert_eq!(w, &[PACKAGE]);
        eax(FLEE.with(|f| f.get()))
    });
    e.mem.set_u32(NPC + 0x68, PROC);
    e.mem.set_u8(PROC + 0x28, 1);
    e.mem.set_u32(PROC + 4, PACKAGE);
    e.mem.set_u32(PROC + 0xc, player);
    let action = |e: &mut Engine| {
        e.mem.set_f32(want[35] + 8, -9.0);
        call_on(e, 0x2700, NPC, &[n(0), n(5)]);
        e.mem.f32(want[35] + 8)
    };
    for (kind, expected) in [
        (1u8, 1.0f32),
        (2, 1.0),
        (6, 1.0),
        (7, 1.0),
        (5, 2.0),
        (9, 2.0),
        (13, 2.0),
        (14, 2.0),
        (10, 6.0),
        (22, 6.0),
        (12, 13.0),
        (3, -9.0),
        (0, -9.0),
    ] {
        e.mem.set_u8(PACKAGE + 0x20, kind);
        assert_eq!(action(&mut e), expected, "kind {kind}");
    }
    e.mem.set_u8(PACKAGE + 0x20, 18);
    FLEE.with(|f| f.set(0));
    assert_eq!(action(&mut e), 5.0);
    FLEE.with(|f| f.set(1));
    assert_eq!(action(&mut e), 6.0);
    // An interrupt package (a middle or high process's +0xE4) comes first.
    let other = FORMS + 0x7400;
    e.mem.set_u8(other + 0x20, 12);
    e.mem.set_u32(PROC + 0xe4, other);
    assert_eq!(action(&mut e), 13.0, "the interrupt package");
    e.mem.set_u32(PROC + 0xe4, 0);
    // The task faction's rank 7 and up stands for the action.
    let data = FORMS + 0x7800;
    e.mem.set_u32(data + 0x10, data + 0x20);
    e.mem.set_u32(data + 0x14, 0);
    e.mem.set_u32(data + 0x20, FACTION);
    put_extra(&mut e, NPC + 0x44, data, 0x5e, data + 0x10);
    for rank in [7u8, 8] {
        e.mem.set_u8(data + 0x24, rank);
        assert_eq!(action(&mut e), f32::from(rank), "rank {rank}");
    }
    e.mem.set_u8(data + 0x24, 6);
    assert_eq!(action(&mut e), 6.0, "rank 6: the package (fleeing)");
}
