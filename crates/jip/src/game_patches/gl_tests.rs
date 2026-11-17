//! Group GL's game patches, part 1: G66, G67, G86, G107.

use super::*;
use crate::game_patches::set_row;
use crate::test_support::engine;
use std::cell::RefCell;

/// A test page for effect items, settings, lists and actors.
const AT: u32 = 0x0650_0000;
const ITEM: u32 = AT;
const SETTING: u32 = AT + 0x100;
const LIST: u32 = AT + 0x200;
const ALCHEMY: u32 = AT + 0x300;
const ACTIVE: u32 = AT + 0x400;
const ACTOR: u32 = AT + 0x500;

thread_local! {
    /// The calls the stand-ins saw: (address, words).
    static CALLS: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
}

fn log(addr: u32, words: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((addr, words.to_vec())));
}

fn calls(addr: u32) -> Vec<Vec<u32>> {
    CALLS.with(|c| {
        c.borrow()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, w)| w.clone())
            .collect()
    })
}

/// The row's handlers in, the test page mapped, the player at `ACTOR`, and stand-ins: Medicine
/// 1.5, Survival 2.0, the perk entries double the value, `AdjustEffectiveness` logged.
fn world() -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    let mut e = engine();
    e.map(AT, 0x1000);
    for page in [0x011c_3000, 0x011d_e000] {
        if !e.mem.is_mapped(page) {
            e.map(page, 0x1000);
        }
    }
    crate::install_as(&mut e, true);
    install_fixes(&mut e);
    e.mem.set_u32(PLAYER, ACTOR);
    e.mem.set_u32(ITEM + ITEM_SETTING, SETTING);
    e.register(MEDICINE_MULT, |_, a| {
        log(MEDICINE_MULT, a);
        Ret {
            st0: 1.5,
            ..Ret::default()
        }
    });
    e.register(SURVIVAL_MULT, |_, a| {
        log(SURVIVAL_MULT, a);
        Ret {
            st0: 2.0,
            ..Ret::default()
        }
    });
    e.register(APPLY_PERK_MODIFIERS, |e, a| {
        log(APPLY_PERK_MODIFIERS, &a[..2]);
        let v = e.mem.f32(a[2]) * 2.0;
        e.mem.set_f32(a[2], v);
        Ret::default()
    });
    e.register(ADJUST_EFFECTIVENESS, |_, a| {
        log(ADJUST_EFFECTIVENESS, a);
        Ret::default()
    });
    e
}

/// JIP's `InitGamePatches` addresses: `WriteRelCall(0x406720, ..)`, the port's list seam at
/// `BuildMenuString`'s `Register` call (`0x40686D`), `WriteRelJump(0x4069D4, ..)`,
/// `WriteRelCall(0x7E0C1F, ..)`, `SAFE_WRITE_BUF(0x80ECA8, ..)`, `WriteRelJump(0x816519, ..)`,
/// `WriteRelJump(0x8236FD, ..)`; all fixes.
#[test]
fn gl1_rows_are_jips_addresses() {
    assert_eq!(
        AREA.fix_sites,
        [
            0x0040_6720,
            0x0040_686d,
            0x0040_69d4,
            0x007e_0c1f,
            0x0080_eca8,
            0x0081_6519,
            0x0082_36fd
        ]
    );
    assert!(AREA.jip_sites.is_empty());
}

/// Not installed, or installed with the row off: no handler, every seam answers nothing.
#[test]
fn gl1_needs_the_row() {
    for (installed, row) in [(false, true), (true, false)] {
        let mut e = engine();
        crate::install_as(&mut e, installed);
        set_row(&mut e, row);
        for &site in AREA.fix_sites {
            assert_eq!(e.site(site, &[0, 0, 0, 0, 0]), None, "{site:08x}");
        }
    }
}

/// G66: no node, the attach is skipped; a node, the exe's call.
#[test]
fn g66_skips_the_attach_without_a_node() {
    let mut e = world();
    assert_eq!(e.site(ATTACH_LIGHT.addr(), &[0]), Some(1));
    assert_eq!(e.site(ATTACH_LIGHT.addr(), &[0x1234]), None);
}

/// G86: no setting, the usage monitor or a hidden setting (flag 8) leave the effect out; any
/// other setting lists it (other flags too).
#[test]
fn g86_leaves_out_hidden_effects() {
    let mut e = world();
    let hidden = |e: &mut Engine| e.site(EFFECT_HIDDEN.addr(), &[ITEM]);
    e.mem.set_u32(ITEM + ITEM_SETTING, 0);
    assert_eq!(hidden(&mut e), Some(1), "no setting");
    e.mem.set_u32(ITEM + ITEM_SETTING, SETTING);
    e.mem.set_u32(USAGE_MONITOR, SETTING);
    assert_eq!(hidden(&mut e), Some(1), "usage monitor");
    e.mem.set_u32(USAGE_MONITOR, SETTING + 0x80);
    e.mem.set_u8(SETTING + SETTING_FLAGS, 0xf7);
    assert_eq!(hidden(&mut e), Some(0), "every other flag");
    e.mem.set_u8(SETTING + SETTING_FLAGS, 8);
    assert_eq!(hidden(&mut e), Some(1), "hidden");
}

/// A chem's list (an alchemy item's `EffectItemList` vtable, type `kind`) and an effect of
/// duration 25 with the setting's flags `flags`.
fn chem_list(e: &mut Engine, kind: u8, flags: u8) {
    e.mem.set_u32(LIST, ALCHEMY_LIST_VTABLE);
    e.mem.set_u8(LIST + LIST_ALCHEMY_TYPE, kind);
    e.mem.set_u32(ITEM + ITEM_DURATION, 25);
    e.mem.set_u8(SETTING + SETTING_FLAGS, flags);
}

fn duration(e: &mut Engine, list: bool) -> Option<u32> {
    if list {
        assert_eq!(e.site(MENU_LIST.addr(), &[LIST]), None);
    }
    e.site(MODIFIED_DURATION.addr(), &[ITEM])
}

/// G107 in the effect text: a chem's (type 10 or 11) non-hostile effect gets the player's
/// `ModifyPositiveChemDuration` (25 doubled: 50); flag 0x80 gives 0; a hostile effect, another
/// type, another list, or no list handed over: the plain duration. The list is taken once.
#[test]
fn g107_menu_duration() {
    let mut e = world();
    chem_list(&mut e, 0xa, 0);
    assert_eq!(duration(&mut e, true), Some(50));
    assert_eq!(calls(APPLY_PERK_MODIFIERS), vec![vec![0x19, ACTOR]]);
    assert_eq!(duration(&mut e, false), Some(25), "the list taken");
    chem_list(&mut e, 0xb, 0);
    assert_eq!(duration(&mut e, true), Some(50));
    chem_list(&mut e, 0xc, 0);
    assert_eq!(duration(&mut e, true), Some(25), "type 12");
    chem_list(&mut e, 9, 0);
    assert_eq!(duration(&mut e, true), Some(25), "type 9");
    chem_list(&mut e, 0xa, 1);
    assert_eq!(duration(&mut e, true), Some(25), "hostile");
    chem_list(&mut e, 0xa, 0x80);
    assert_eq!(duration(&mut e, true), Some(0), "no duration");
    chem_list(&mut e, 0xa, 0);
    e.mem.set_u32(LIST, ALCHEMY_LIST_VTABLE + 4);
    assert_eq!(
        duration(&mut e, true),
        Some(25),
        "not an alchemy item's list"
    );
    assert_eq!(calls(APPLY_PERK_MODIFIERS).len(), 2);
}

/// G107: the perks' float goes back toward zero (12.9 shows 12); NaN gives `cvttss2si`'s
/// 0x80000000.
#[test]
fn g107_menu_duration_truncates() {
    let mut e = world();
    chem_list(&mut e, 0xa, 0);
    e.register(APPLY_PERK_MODIFIERS, |e, a| {
        e.mem.set_f32(a[2], 12.9);
        Ret::default()
    });
    assert_eq!(duration(&mut e, true), Some(12));
    e.register(APPLY_PERK_MODIFIERS, |e, a| {
        e.mem.set_f32(a[2], f32::NAN);
        Ret::default()
    });
    assert_eq!(duration(&mut e, true), Some(0x8000_0000));
}

/// G107 when added: skip (1) unless the alchemy item's type is 10 or 11; then the effect
/// setting's hostile flag.
#[test]
fn g107_chem_duration() {
    let mut e = world();
    e.mem.set_u32(ACTIVE + ACTIVE_ITEM, ITEM);
    let skip = |e: &mut Engine| e.site(CHEM_DURATION.addr(), &[ALCHEMY, ACTIVE]);
    for (kind, flags, answer) in [
        (0xa, 0, 0),
        (0xb, 0xfe, 0),
        (0xa, 1, 1),
        (0xc, 0, 1),
        (9, 0, 1),
        (0, 0, 1),
    ] {
        e.mem.set_u8(ALCHEMY + ALCHEMY_TYPE, kind);
        e.mem.set_u8(SETTING + SETTING_FLAGS, flags);
        assert_eq!(skip(&mut e), Some(answer), "{kind} {flags}");
    }
}

fn menu(e: &mut Engine, entry: i32, magnitude: i32, av: u32, medicine: u32, survival: u32) -> i32 {
    let words = [entry as u32, magnitude as u32, av, medicine, survival];
    e.site(MENU_SKILL_MODIFIERS.addr(), &words).unwrap() as i32
}

/// G67 in the effect text: Medicine first (10 * 1.5 = 15, Health's perk doubles: 30), else
/// Survival (* 2.0, not radiation); neither flag or a negative entry: unchanged; the magnitude
/// (already negated) is what is scaled.
#[test]
fn g67_menu_scaling() {
    let mut e = world();
    assert_eq!(menu(&mut e, 10, 10, 0x10, 0, 0), 10, "neither");
    assert_eq!(menu(&mut e, -10, -10, 0x10, 1, 1), -10, "negative entry");
    assert!(calls(MEDICINE_MULT).is_empty());
    assert_eq!(menu(&mut e, 10, 10, 0x10, 1, 1), 30, "Medicine only, perk");
    assert_eq!(calls(SURVIVAL_MULT).len(), 0);
    assert_eq!(calls(APPLY_PERK_MODIFIERS), vec![vec![0xc, ACTOR]]);
    assert_eq!(calls(MEDICINE_MULT), vec![vec![ACTOR]]);
    assert_eq!(menu(&mut e, 5, 5, 7, 1, 0), 7, "7.5 toward zero");
    assert_eq!(menu(&mut e, 5, 5, 7, 0, 1), 10, "Survival");
    assert_eq!(menu(&mut e, 5, 5, 0x36, 0, 1), 5, "Survival, radiation");
    assert_eq!(menu(&mut e, 5, 5, 0x36, 1, 0), 7, "Medicine, radiation");
    assert_eq!(
        menu(&mut e, 0, -3, 7, 1, 0),
        -4,
        "entry 0, negated magnitude"
    );
    assert_eq!(calls(APPLY_PERK_MODIFIERS).len(), 1);
}

fn cast(e: &mut Engine, flags: u8, setting_flags: u8, av: u32, actor_id: u32) -> Vec<Vec<u32>> {
    CALLS.with(|c| c.borrow_mut().clear());
    e.mem.set_u8(ALCHEMY + ALCHEMY_FLAGS, flags);
    e.mem.set_u8(SETTING + SETTING_FLAGS, setting_flags);
    e.mem.set_u32(ITEM + ITEM_ACTOR_VALUE, av);
    e.mem.set_u32(ACTOR + FORM_ID, actor_id);
    e.mem.set_u32(ACTIVE + ACTIVE_ITEM, ITEM);
    let words = [ALCHEMY, ACTIVE, ACTOR, 2.0f32.to_bits()];
    assert_eq!(e.site(CAST_SKILL_MODIFIERS.addr(), &words), Some(1));
    calls(ADJUST_EFFECTIVENESS)
}

/// G67 when cast: always answered; Medicine first (2.0 * 1.5 = 3, the player's Health perk
/// doubles: 6), else Survival (* 2.0, not radiation); no alchemy item, neither flag, or a
/// detrimental setting (flag 4): nothing adjusted.
#[test]
fn g67_cast_scaling() {
    let mut e = world();
    let adjusted = |v: f32| vec![vec![ACTIVE, v.to_bits()]];
    assert_eq!(
        cast(&mut e, 6, 0, 0x10, 0x14),
        adjusted(6.0),
        "Medicine, player"
    );
    assert_eq!(calls(SURVIVAL_MULT).len(), 0);
    assert_eq!(calls(MEDICINE_MULT), vec![vec![ACTOR]]);
    assert_eq!(
        cast(&mut e, 4, 0, 0x10, 0x15),
        adjusted(3.0),
        "not the player"
    );
    assert_eq!(cast(&mut e, 2, 0, 7, 0x14), adjusted(4.0), "Survival");
    assert_eq!(
        cast(&mut e, 2, 0, 0x36, 0x14),
        vec![] as Vec<Vec<u32>>,
        "radiation"
    );
    assert_eq!(
        cast(&mut e, 4, 0, 0x36, 0x14),
        adjusted(3.0),
        "Medicine, radiation"
    );
    assert!(cast(&mut e, 4, 4, 0x10, 0x14).is_empty(), "detrimental");
    assert!(cast(&mut e, 1, 0, 0x10, 0x14).is_empty(), "neither");
    CALLS.with(|c| c.borrow_mut().clear());
    let none = [0, ACTIVE, ACTOR, 2.0f32.to_bits()];
    assert_eq!(e.site(CAST_SKILL_MODIFIERS.addr(), &none), Some(1));
    assert!(calls(ADJUST_EFFECTIVENESS).is_empty(), "no alchemy item");
}
