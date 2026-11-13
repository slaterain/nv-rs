//! Tests of JIP's UI hooks: nothing put in while JIP answers "not installed" (the twelve
//! commands' records as #502's table leaves them, no seam handler, no routine); installed, each
//! seam's handler as JIP's hook decides, the commands through their records with the hooks' user
//! counts, and the click routine in and out of a menu's vtable. Script calls are recorded
//! ([`crate::script_call`]). Layouts are written out at literal offsets, not the code's: a form's
//! type `+4` and JIP's flag byte `+6`; a reference's base form `+0x20` and parent cell `+0x40`;
//! an actor's JIP flags `+0x106`; a menu's id `+0x20`; the interface manager's menu under the
//! pointer `+0xD0` and crosshair reference `+0xFC`; the hacking menu's guess tile count `+0x17C`
//! and reference `+0x198`; the computers menu's reference `+0xAC`; a tile's children list `+4`
//! (head, tail, count; nodes next, previous, element), name `+0x20` (text, length) and parent
//! `+0x28`; and the exe's statics: `g_thePlayer` `0x011DEA3C`, `g_interfaceManager`
//! `0x011D8A80`, the shown bytes `0x011F308F + id`, the terminal holder `0x011DB63C` and path
//! `0x011A0BB0`, the computers and hacking menus `0x011D9334`, `0x011D95B8`, `TileMenu`'s vtable
//! `0x0106ED44`, the menus' vtables (MessageMenu `0x0107566C`, HUDMainMenu `0x01072DF4`), and the
//! vtable entry that answers true, `0x008D0360`.

use super::*;
use crate::script_call::calls;
use crate::test_support::{engine, form_arg, int_arg, line, s, script_ref, RESULT};

const AREA: u32 = 0x0500_0000;
const SCRIPT_FORM: u32 = AREA;
const SCRIPT_TWO: u32 = AREA + 0x40;
const PLAYER_REF: u32 = AREA + 0x100;
const MANAGER: u32 = AREA + 0x300;
const CELL: u32 = AREA + 0x400;
const MENU: u32 = AREA + 0x500;
const FORM: u32 = AREA + 0x600;
const BASE: u32 = AREA + 0x700;
const VTABLE: u32 = AREA + 0x800;
const TILES: u32 = AREA + 0x2000;
const NAMES: u32 = AREA + 0x3000;
const MESSAGE_MENU_SLOT: u32 = 0x0107_566c + 0xc;
const HUD_MENU_SLOT: u32 = 0x0107_2df4 + 0xc;
const EXE_CLICK: u32 = 0x007a_a070;

fn world() -> Engine {
    calls();
    let mut e = engine();
    e.map(AREA, 0x1_0000);
    e.map(0x011d_8000, 0x4000);
    e.map(0x011f_3000, 0x1000);
    e.map(0x011a_0000, 0x1000);
    e.map(0x0106_f000, 0x9000);
    e.mem.set_u8(SCRIPT_FORM + 4, 0x11);
    e.mem.set_u32(SCRIPT_FORM + 0xc, 0x0100_0900);
    e.mem.set_u8(SCRIPT_TWO + 4, 0x11);
    e.mem.set_u32(SCRIPT_TWO + 0xc, 0x0100_0901);
    e.mem.set_u32(PLAYER_REF + 0xc, 0x14);
    e.mem.set_u32(PLAYER_REF + 0x40, CELL);
    e.mem.set_u32(0x011d_ea3c, PLAYER_REF);
    e.mem.set_u32(0x011d_8a80, MANAGER);
    e.mem.set_u32(MESSAGE_MENU_SLOT, EXE_CLICK);
    e.mem.set_u32(HUD_MENU_SLOT, 0x008d_0600);
    e.mem.set_u32(0x011a_0bb0, 0x0101_1111);
    e.register(EXE_CLICK, |e, a| {
        e.mem.set_u32(AREA + 0xf000, a[1]);
        Ret::default()
    });
    crate::install_as(&mut e, true);
    script_ref(&mut e, 1, SCRIPT_FORM);
    script_ref(&mut e, 2, SCRIPT_TWO);
    e
}

/// Runs `opcode` with `args` through its record (no calling reference).
fn run(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) {
    let words = line(e, opcode, args);
    e.mem.set_f64(RESULT, 0.0);
    let at = crate::test_support::execute_of(e, opcode);
    assert_eq!(e.call(at, &words).eax & 0xff, 1);
}

fn site(e: &mut Engine, addr: u32, args: &[u32]) -> Option<u32> {
    e.site(addr, args)
}

/// While JIP answers "not installed": each command's whole record (0x28 bytes and its parameter
/// list) is what #502's table leaves, no seam has a handler, and no hook has a user.
#[test]
fn not_installed_nothing_is_put_in() {
    let snapshot = |e: &Engine, op: u32| {
        let at = e.script_tables().record(op);
        let mut bytes = e.mem.bytes(at, 0x28);
        let (count, params) = (e.mem.u16(at + 0x12), e.mem.u32(at + 0x14));
        bytes.extend(e.mem.bytes(params, 12 * u32::from(count)));
        bytes
    };
    let mut table_only = engine();
    crate::table::install(&mut table_only, false);
    let mut off = engine();
    crate::install_as(&mut off, false);
    for op in COMMANDS {
        assert_eq!(snapshot(&off, op), snapshot(&table_only, op), "{op:04x}");
        assert_eq!(off.mem.u32(off.script_tables().record(op) + 0x18), 0);
    }
    for addr in SITES {
        assert!(off.site_owners(addr).is_empty(), "{addr:08x}");
    }
    assert!(off.ext::<Jip>().ui_hooks.map_markers_list == 0);
    assert_eq!(off.hook_users(Hook::MenuState), 0);
}

/// Installed: JIP's handler at each of the eleven seams; the commands named.
#[test]
fn installed_the_seams_have_jips_handlers() {
    let e = world();
    for addr in SITES {
        assert_eq!(e.site_owners(addr), [Owner::Jip], "{addr:08x}");
    }
    for op in COMMANDS {
        assert_ne!(
            e.mem.u32(e.script_tables().record(op) + 0x18),
            0,
            "{op:04x}"
        );
    }
}

/// The open and close events: a menu's first open or close script is one user of the menu state
/// hook (one more for another menu, none more for the same menu's close); the byte stored at
/// either seam; a change calls the scripts with the menu's id (not without a parent cell), the
/// same value nothing; removal gives the user back.
#[test]
fn menu_open_and_close_events() {
    let mut e = world();
    run(
        &mut e,
        SET_ON_MENU_OPEN,
        &[form_arg(1), int_arg(1), int_arg(0x3e9)],
    );
    assert_eq!(e.hook_users(Hook::MenuState), 1);
    run(
        &mut e,
        SET_ON_MENU_CLOSE,
        &[form_arg(2), int_arg(1), int_arg(0x3e9)],
    );
    assert_eq!(e.hook_users(Hook::MenuState), 1, "the same menu");
    run(
        &mut e,
        SET_ON_MENU_OPEN,
        &[form_arg(1), int_arg(1), int_arg(0x3ea)],
    );
    assert_eq!(e.hook_users(Hook::MenuState), 2);
    // A menu id JIP does not know (0x3ED is -1 in the table): nothing.
    run(
        &mut e,
        SET_ON_MENU_OPEN,
        &[form_arg(1), int_arg(1), int_arg(0x3ed)],
    );
    assert_eq!(e.hook_users(Hook::MenuState), 2);
    calls();
    assert_eq!(site(&mut e, 0x00a1_f2f9, &[0x3e9, 1]), Some(0));
    assert_eq!(e.mem.u8(0x011f_308f + 0x3e9), 1);
    assert_eq!(calls(), [(SCRIPT_FORM, 0, vec![0x3e9])]);
    assert_eq!(site(&mut e, 0x00a1_f2f9, &[0x3e9, 1]), Some(0));
    assert!(calls().is_empty(), "unchanged");
    assert_eq!(site(&mut e, 0x00a1_c573, &[0x3e9, 0]), Some(0));
    assert_eq!(e.mem.u8(0x011f_308f + 0x3e9), 0);
    assert_eq!(calls(), [(SCRIPT_TWO, 0, vec![0x3e9])]);
    e.mem.set_u32(PLAYER_REF + 0x40, 0);
    assert_eq!(site(&mut e, 0x00a1_f2f9, &[0x3e9, 1]), Some(0));
    assert_eq!(e.mem.u8(0x011f_308f + 0x3e9), 1, "stored without a cell");
    assert!(calls().is_empty(), "no events without a cell");
    run(
        &mut e,
        SET_ON_MENU_OPEN,
        &[form_arg(1), int_arg(0), int_arg(0x3e9)],
    );
    assert_eq!(e.hook_users(Hook::MenuState), 2, "the close script stays");
    run(
        &mut e,
        SET_ON_MENU_CLOSE,
        &[form_arg(2), int_arg(0), int_arg(0x3e9)],
    );
    assert_eq!(e.hook_users(Hook::MenuState), 1);
    // Removing what is not set (again, or never added) gives no user back: menu 0x3EA keeps its
    // open script and the one user.
    run(
        &mut e,
        SET_ON_MENU_CLOSE,
        &[form_arg(2), int_arg(0), int_arg(0x3e9)],
    );
    run(
        &mut e,
        SET_ON_MENU_OPEN,
        &[form_arg(2), int_arg(0), int_arg(0x3e9)],
    );
    run(
        &mut e,
        SET_ON_MENU_OPEN,
        &[form_arg(2), int_arg(0), int_arg(0x3ea)],
    );
    run(
        &mut e,
        SET_ON_MENU_CLOSE,
        &[form_arg(1), int_arg(0), int_arg(0x3eb)],
    );
    assert_eq!(e.hook_users(Hook::MenuState), 1);
}

/// Menu id 0 sets all 36 menus: 36 users; removing from all gives them back.
#[test]
fn menu_id_zero_is_every_menu() {
    let mut e = world();
    run(
        &mut e,
        SET_ON_MENU_OPEN,
        &[form_arg(1), int_arg(1), int_arg(0)],
    );
    assert_eq!(e.hook_users(Hook::MenuState), 36);
    run(&mut e, SET_ON_MENU_OPEN, &[form_arg(1), int_arg(0)]);
    assert_eq!(e.hook_users(Hook::MenuState), 0);
}

/// The mouseover events: the menu under the pointer (`+0xD0`, its id `+0x20`) is remembered; with
/// no menu the last one's scripts are called again.
#[test]
fn mouseover_events() {
    let mut e = world();
    run(
        &mut e,
        SET_ON_MOUSEOVER_CHANGE,
        &[form_arg(1), int_arg(1), int_arg(0x3ea)],
    );
    assert_eq!(e.hook_users(Hook::MenuHandleMouseover), 1);
    assert_eq!(e.hook_users(Hook::MenuState), 0);
    e.mem.set_u32(MENU + 0x20, 0x3ea);
    e.mem.set_u32(MANAGER + 0xd0, MENU);
    calls();
    assert_eq!(site(&mut e, 0x0070_d642, &[MANAGER]), None);
    assert_eq!(calls(), [(SCRIPT_FORM, 0, vec![0x3ea])]);
    e.mem.set_u32(MANAGER + 0xd0, 0);
    assert_eq!(site(&mut e, 0x0070_d642, &[MANAGER]), None);
    assert_eq!(calls(), [(SCRIPT_FORM, 0, vec![0x3ea])], "the last menu");
    e.mem.set_u32(MENU + 0x20, 0x3e9);
    e.mem.set_u32(MANAGER + 0xd0, MENU);
    assert_eq!(site(&mut e, 0x0070_d642, &[MANAGER]), None);
    assert!(calls().is_empty(), "another menu, no scripts");
    // A second menu's script; removing one twice gives back one user, not two.
    run(
        &mut e,
        SET_ON_MOUSEOVER_CHANGE,
        &[form_arg(1), int_arg(1), int_arg(0x3e9)],
    );
    assert_eq!(e.hook_users(Hook::MenuHandleMouseover), 2);
    for _ in 0..2 {
        run(
            &mut e,
            SET_ON_MOUSEOVER_CHANGE,
            &[form_arg(1), int_arg(0), int_arg(0x3e9)],
        );
    }
    run(
        &mut e,
        SET_ON_MOUSEOVER_CHANGE,
        &[form_arg(2), int_arg(0), int_arg(0x3ea)],
    );
    assert_eq!(e.hook_users(Hook::MenuHandleMouseover), 1);
}

/// The location and crafting events: a user per script, the scripts called with the marker, and
/// with the form and count.
#[test]
fn location_and_crafting_events() {
    let mut e = world();
    run(&mut e, SET_ON_LOCATION_DISCOVER, &[form_arg(1), int_arg(1)]);
    run(&mut e, SET_ON_LOCATION_DISCOVER, &[form_arg(2), int_arg(1)]);
    run(&mut e, SET_ON_LOCATION_DISCOVER, &[form_arg(2), int_arg(1)]);
    assert_eq!(e.hook_users(Hook::LocationDiscover), 2);
    run(&mut e, SET_ON_CRAFTING, &[form_arg(1), int_arg(1)]);
    assert_eq!(e.hook_users(Hook::ItemCrafted), 1);
    calls();
    assert_eq!(site(&mut e, 0x0077_955d, &[FORM]), None);
    assert_eq!(
        calls(),
        [(SCRIPT_FORM, 0, vec![FORM]), (SCRIPT_TWO, 0, vec![FORM])]
    );
    assert_eq!(site(&mut e, 0x0072_89ff, &[FORM, 6]), None);
    assert_eq!(calls(), [(SCRIPT_FORM, 0, vec![FORM, 6])]);
    run(&mut e, SET_ON_LOCATION_DISCOVER, &[form_arg(1), int_arg(0)]);
    run(&mut e, SET_ON_LOCATION_DISCOVER, &[form_arg(1), int_arg(0)]);
    assert_eq!(e.hook_users(Hook::LocationDiscover), 1);
    run(&mut e, SET_ON_CRAFTING, &[form_arg(2), int_arg(0)]);
    assert_eq!(
        e.hook_users(Hook::ItemCrafted),
        1,
        "never added: no user back"
    );
}

/// The HUD's reference: kept, or none for JIP's no-activation flag (bit 1 of `+6`) on it or its
/// base, or a disabled base type.
#[test]
fn a_reference_the_player_may_not_activate() {
    let mut e = world();
    e.mem.set_u32(FORM + 0x20, BASE);
    e.mem.set_u8(BASE + 4, 0x15);
    assert_eq!(site(&mut e, 0x0077_5b9b, &[0]), None);
    assert_eq!(site(&mut e, 0x0077_5b9b, &[FORM]), None);
    e.mem.set_u8(FORM + 6, 2);
    assert_eq!(site(&mut e, 0x0077_5b9b, &[FORM]), Some(0));
    e.mem.set_u8(FORM + 6, 1);
    assert_eq!(site(&mut e, 0x0077_5b9b, &[FORM]), None, "another flag");
    e.mem.set_u8(BASE + 6, 2);
    assert_eq!(site(&mut e, 0x0077_5b9b, &[FORM]), Some(0));
    e.mem.set_u8(BASE + 6, 0);
    e.ext::<Jip>().ui_hooks.activation_disabled_types[0x15] = true;
    assert_eq!(site(&mut e, 0x0077_5b9b, &[FORM]), Some(0));
}

/// A disabled message (bit 0 of `+6`) answers true; another passes.
#[test]
fn a_disabled_message() {
    let mut e = world();
    assert_eq!(site(&mut e, 0x005b_46aa, &[FORM]), None);
    assert_eq!(site(&mut e, 0x007e_88b6, &[FORM]), None);
    e.mem.set_u8(FORM + 6, 1);
    assert_eq!(site(&mut e, 0x005b_46aa, &[FORM]), Some(1));
    assert_eq!(site(&mut e, 0x007e_88b6, &[FORM]), Some(1));
    e.mem.set_u8(FORM + 6, 2);
    assert_eq!(site(&mut e, 0x005b_46aa, &[FORM]), None);
}

/// VATS: not an actor (its `IsActor`, vtable `+0x100`, not the true answer) stays; an actor
/// stays unless JIP's not-targetable flag (bit 1 of `+0x106`) is set.
#[test]
fn an_actor_not_targetable() {
    let mut e = world();
    e.mem.set_u32(FORM, VTABLE);
    e.mem.set_u32(VTABLE + 0x100, 0x0040_0000);
    e.mem.set_u8(FORM + 0x106, 2);
    assert_eq!(site(&mut e, 0x007f_54b1, &[FORM]), Some(1));
    e.mem.set_u32(VTABLE + 0x100, 0x008d_0360);
    assert_eq!(site(&mut e, 0x007f_54b1, &[FORM]), Some(0));
    e.mem.set_u8(FORM + 0x106, 1);
    assert_eq!(site(&mut e, 0x007f_54b1, &[FORM]), Some(1));
}

/// The terminal model: released first; a holder still holding answers it; else the computers
/// menu's reference, or the hacking menu's (its own with guess tiles, else the crosshair's),
/// picks its base's own path or the game's, written to the path, answer 0.
#[test]
fn a_terminals_own_model() {
    let mut e = world();
    e.register(0x007f_fe00, |e, _| {
        e.mem.set_u32(AREA + 0xf010, e.mem.u32(AREA + 0xf010) + 1);
        Ret::default()
    });
    let alt = AREA + 0x5000;
    e.mem.set_u32(FORM + 0x20, BASE);
    e.ext::<Jip>().ui_hooks.terminal_models.insert(BASE, alt);
    e.mem.set_u32(0x011d_b63c, 0x7777);
    assert_eq!(site(&mut e, 0x007f_ef0b, &[]), Some(0x7777));
    assert_eq!(e.mem.u32(AREA + 0xf010), 1, "released");
    e.mem.set_u32(0x011d_b63c, 0);
    assert_eq!(site(&mut e, 0x007f_ef0b, &[]), Some(0), "no menu");
    assert_eq!(e.mem.u32(0x011a_0bb0), 0x0101_1111, "unchanged");
    // The computers menu.
    e.mem.set_u32(0x011d_9334, MENU);
    e.mem.set_u32(MENU + 0xac, FORM);
    assert_eq!(site(&mut e, 0x007f_ef0b, &[]), Some(0));
    assert_eq!(e.mem.u32(0x011a_0bb0), alt);
    // The hacking menu without guess tiles: the crosshair's reference, another base.
    e.mem.set_u32(0x011d_9334, 0);
    e.mem.set_u32(0x011d_95b8, MENU + 0x1000);
    e.mem.set_u32(MENU + 0x1000 + 0x198, FORM);
    e.mem.set_u32(MANAGER + 0xfc, AREA + 0x900);
    e.mem.set_u32(AREA + 0x900 + 0x20, BASE + 0x40);
    assert_eq!(site(&mut e, 0x007f_ef0b, &[]), Some(0));
    assert_eq!(e.mem.u32(0x011a_0bb0), 0x0101_1111, "the game's path");
    e.mem.set_u8(MENU + 0x1000 + 0x17c, 1);
    assert_eq!(site(&mut e, 0x007f_ef0b, &[]), Some(0));
    assert_eq!(e.mem.u32(0x011a_0bb0), alt, "its own reference");
}

/// The map's markers: JIP's list head, a guest word pair made at the start.
#[test]
fn the_map_markers_list() {
    let mut e = world();
    let list = e.ext::<Jip>().ui_hooks.map_markers_list;
    assert_ne!(list, 0);
    assert_eq!(site(&mut e, 0x0079_eba1, &[PLAYER_REF]), Some(list));
    assert_eq!(e.mem.u32(list), 0);
}

/// A tile with `name` under `parent` (appended to its children), at `TILES + 0x100 * n`.
fn tile(e: &mut Engine, n: u32, parent: u32, name: &[u8]) -> u32 {
    let t = TILES + 0x100 * n;
    let text = NAMES + 0x40 * n;
    for (i, b) in name.iter().enumerate() {
        e.mem.set_u8(text + i as u32, *b);
    }
    e.mem.set_u8(text + name.len() as u32, 0);
    e.mem.set_u32(t + 0x20, text);
    e.mem.set_u16(t + 0x24, name.len() as u16);
    e.mem.set_u32(t + 0x28, parent);
    if parent != 0 {
        let node = t + 0x80;
        let tail = e.mem.u32(parent + 8);
        e.mem.set_u32(node, 0);
        e.mem.set_u32(node + 4, tail);
        e.mem.set_u32(node + 8, t);
        if tail != 0 {
            e.mem.set_u32(tail, node);
        } else {
            e.mem.set_u32(parent + 4, node);
        }
        e.mem.set_u32(parent + 8, node);
        e.mem.set_u32(parent + 0xc, e.mem.u32(parent + 0xc) + 1);
    }
    t
}

/// The click path: names from the menu tile down, `:n` for `n` earlier siblings of the same name.
#[test]
fn a_tiles_component_path() {
    let mut e = world();
    let root = tile(&mut e, 0, 0, b"root");
    let menu = tile(&mut e, 1, root, b"MessageMenu");
    e.mem.set_u32(menu, 0x0106_ed44);
    let a = tile(&mut e, 2, menu, b"button");
    let _b = tile(&mut e, 3, menu, b"other");
    let c = tile(&mut e, 4, menu, b"button");
    let d = tile(&mut e, 5, menu, b"button");
    let x = tile(&mut e, 6, d, b"Text");
    assert_eq!(component_full_name(&e, a), b"MessageMenu/button");
    assert_eq!(
        component_full_name(&e, c),
        b"MessageMenu/button",
        "a different name between"
    );
    assert_eq!(component_full_name(&e, d), b"MessageMenu/button:1");
    assert_eq!(component_full_name(&e, x), b"MessageMenu/button:1/Text");
}

/// The click events: the routine in MessageMenu's vtable with its first event and the exe's
/// function back with its last; a click runs the path's scripts (lower case, the nearest path up)
/// and the tile id's, with the menu's id, the tile id and the name, then the exe's function.
#[test]
fn menu_click_events() {
    let mut e = world();
    let root = tile(&mut e, 0, 0, b"root");
    let menu_tile = tile(&mut e, 1, root, b"MessageMenu");
    e.mem.set_u32(menu_tile, 0x0106_ed44);
    let button = tile(&mut e, 2, menu_tile, b"Button");
    e.mem.set_u32(MENU + 0x20, 0x3e9);
    run(
        &mut e,
        SET_ON_MENU_CLICK,
        &[form_arg(1), int_arg(1), s(b"MessageMenu\\Button")],
    );
    assert_eq!(e.mem.u32(MESSAGE_MENU_SLOT), MENU_CLICK_ROUTINE);
    assert_eq!(
        e.mem.u32(HUD_MENU_SLOT),
        0x008d_0600,
        "another menu untouched"
    );
    run(
        &mut e,
        SET_ON_MENU_CLICK,
        &[form_arg(2), int_arg(1), s(b"MessageMenu#7")],
    );
    assert_eq!(
        e.ext::<Jip>().ui_hooks.clicks[0].filters[0].0,
        b"messagemenu/button"
    );
    calls();
    let name = e.mem.u32(button + 0x20);
    let words = [MENU, 7, button];
    e.call(e.mem.u32(MESSAGE_MENU_SLOT), &words);
    assert_eq!(
        calls(),
        [
            (SCRIPT_FORM, 0, vec![0x3e9, 7, name]),
            (SCRIPT_TWO, 0, vec![0x3e9, 7, name])
        ]
    );
    assert_eq!(e.mem.u32(AREA + 0xf000), 7, "the exe's function ran");
    // No tile clicked: the id's scripts with an empty name.
    e.call(MENU_CLICK_ROUTINE, &[MENU, 7, 0]);
    let got = calls();
    assert_eq!(got.len(), 1);
    assert_eq!(e.mem.u8(got[0].2[2]), 0);
    run(
        &mut e,
        SET_ON_MENU_CLICK,
        &[form_arg(1), int_arg(0), s(b"MessageMenu/Button")],
    );
    assert_eq!(
        e.mem.u32(MESSAGE_MENU_SLOT),
        MENU_CLICK_ROUTINE,
        "the id's event stays"
    );
    run(
        &mut e,
        SET_ON_MENU_CLICK,
        &[form_arg(2), int_arg(0), s(b"MessageMenu#7")],
    );
    assert_eq!(e.mem.u32(MESSAGE_MENU_SLOT), EXE_CLICK, "put back");
    assert!(e.ext::<Jip>().ui_hooks.clicks[0].is_empty());
}

/// A path's script is not added under a shorter path that has it, and replaces it in the deeper
/// paths below (removed where it was the last).
#[test]
fn click_paths_nest() {
    let mut e = world();
    let add = |e: &mut Engine, i: u16, path: &[u8]| {
        run(e, SET_ON_MENU_CLICK, &[form_arg(i), int_arg(1), s(path)]);
    };
    add(&mut e, 1, b"MessageMenu/a/b");
    add(&mut e, 2, b"MessageMenu/a/b");
    add(&mut e, 1, b"MessageMenu/a");
    let keys = |e: &mut Engine| -> Vec<(Vec<u8>, Vec<u32>)> {
        e.ext::<Jip>().ui_hooks.clicks[0]
            .filters
            .iter()
            .map(|(k, s)| (k.clone(), s.iter().copied().collect()))
            .collect()
    };
    assert_eq!(
        keys(&mut e),
        [
            (b"messagemenu/a".to_vec(), vec![SCRIPT_FORM]),
            (b"messagemenu/a/b".to_vec(), vec![SCRIPT_TWO]),
        ]
    );
    add(&mut e, 1, b"MessageMenu/a/b/c");
    assert_eq!(keys(&mut e).len(), 2, "under a path that has it");
    // An unknown menu name: nothing.
    add(&mut e, 1, b"NoSuchMenu/a");
    assert_eq!(keys(&mut e).len(), 2);
}

/// The key, control and trigger commands: into the LN events' maps; the triggers only with a
/// controller ready (none here), so nothing.
#[test]
fn key_control_and_trigger_events() {
    let mut e = world();
    run(
        &mut e,
        SET_ON_KEY_DOWN,
        &[form_arg(1), int_arg(1), int_arg(0x1e)],
    );
    run(
        &mut e,
        SET_ON_CONTROL_UP,
        &[form_arg(1), int_arg(1), int_arg(5)],
    );
    {
        let ln = &e.ext::<Jip>().ln_events;
        assert!(ln.keys[&0x1e].on_down.contains(&SCRIPT_FORM));
        assert!(ln.controls[&5].on_up.contains(&SCRIPT_FORM));
    }
    run(
        &mut e,
        SET_ON_TRIGGER_DOWN,
        &[form_arg(1), int_arg(1), int_arg(1)],
    );
    assert!(e
        .ext::<Jip>()
        .ln_events
        .triggers
        .iter()
        .all(|t| t.is_empty()));
}

/// Removing a key or control script that is not set leaves the set ones and the events' flags.
#[test]
fn key_and_control_removals_of_what_is_not_set() {
    use crate::ln_events::{id, mask};
    let mut e = world();
    run(
        &mut e,
        SET_ON_KEY_DOWN,
        &[form_arg(1), int_arg(1), int_arg(0x1e)],
    );
    run(
        &mut e,
        SET_ON_CONTROL_UP,
        &[form_arg(1), int_arg(1), int_arg(5)],
    );
    let flags = e.ext::<Jip>().ln_events.flags;
    let both = mask(id::KEY_DOWN) | mask(id::CONTROL_UP);
    assert_eq!(flags & both, both);
    run(
        &mut e,
        SET_ON_KEY_DOWN,
        &[form_arg(2), int_arg(0), int_arg(0x1e)],
    );
    run(
        &mut e,
        SET_ON_CONTROL_UP,
        &[form_arg(2), int_arg(0), int_arg(5)],
    );
    run(
        &mut e,
        SET_ON_KEY_UP,
        &[form_arg(1), int_arg(0), int_arg(0x1e)],
    );
    let ln = &e.ext::<Jip>().ln_events;
    assert_eq!(ln.flags, flags);
    assert!(ln.keys[&0x1e].on_down.contains(&SCRIPT_FORM));
    assert!(ln.controls[&5].on_up.contains(&SCRIPT_FORM));
}

/// The trigger events with a controller ready: into slot `(right << 1) + down`, the event's
/// flag set; removing one of two (and removing it again) keeps the flag, the last clears it;
/// the up events beside them untouched.
#[test]
fn trigger_events_with_a_controller() {
    use crate::ln_events::{id, mask};
    let mut e = world();
    e.ext::<Jip>().input.ready = true;
    let trig = |e: &mut Engine, op: u32, script: u16, add: i32, right: i32| {
        run(e, op, &[form_arg(script), int_arg(add), int_arg(right)]);
    };
    trig(&mut e, SET_ON_TRIGGER_DOWN, 1, 1, 0);
    trig(&mut e, SET_ON_TRIGGER_DOWN, 2, 1, 1);
    trig(&mut e, SET_ON_TRIGGER_UP, 1, 1, 1);
    {
        let ln = &e.ext::<Jip>().ln_events;
        assert!(ln.triggers[1].contains(&SCRIPT_FORM));
        assert!(ln.triggers[3].contains(&SCRIPT_TWO));
        assert!(ln.triggers[2].contains(&SCRIPT_FORM));
        assert!(ln.triggers[0].is_empty());
        let both = mask(id::TRIGGER_DOWN) | mask(id::TRIGGER_UP);
        assert_eq!(ln.flags & both, both);
    }
    let down = mask(id::TRIGGER_DOWN);
    trig(&mut e, SET_ON_TRIGGER_DOWN, 1, 0, 0);
    assert_ne!(
        e.ext::<Jip>().ln_events.flags & down,
        0,
        "the right one stays"
    );
    trig(&mut e, SET_ON_TRIGGER_DOWN, 1, 0, 0);
    assert_ne!(e.ext::<Jip>().ln_events.flags & down, 0, "again: nothing");
    trig(&mut e, SET_ON_TRIGGER_DOWN, 2, 0, 1);
    assert_eq!(e.ext::<Jip>().ln_events.flags & down, 0, "the last one");
    assert_ne!(
        e.ext::<Jip>().ln_events.flags & mask(id::TRIGGER_UP),
        0,
        "the up event untouched"
    );
}

/// JIP's InteractionDisabled hook has two sites; the activation one (`0x943279` in
/// `PlayerCharacter::Update`) has no seam yet (step 3.5c). Until it has, no command that adds a
/// user to the hook (`SetInteractionDisabled` 0x27C9, `SetInteractionDisabledType` 0x27CB) may
/// be built: half the hook would run.
#[test]
fn interaction_disabled_waits_for_its_activation_seam() {
    let e = world();
    let activation_seam = engine_core::extend::SITES
        .iter()
        .any(|s| s.addr == 0x0094_3279 && s.owners.iter().any(|o| matches!(o, Owner::Jip)));
    for op in [0x27c9u32, 0x27cb] {
        let built = e.mem.u32(e.script_tables().record(op) + 0x18) != 0;
        assert!(
            activation_seam || !built,
            "{op:04x} adds InteractionDisabled users but 0x943279 has no JIP seam"
        );
    }
}

/// The README's JIP count is the table's: the JIP opcodes whose record has a handler once JIP
/// answers "installed", the identity (`GetLNVersion`, `GetNVSEVersionFull`) and the empty slots
/// left out.
#[test]
fn the_readme_counts_the_built_commands() {
    let e = world();
    let built = crate::slots::SLOTS
        .iter()
        .filter(|s| s.kind != crate::table::Kind::Empty && s.opcode != 0x2200 && s.opcode != 0x2210)
        .filter(|s| {
            let record = e.script_tables().record(s.opcode);
            e.mem.u32(record + 0x18) != 0 || e.mem.u32(record + 0x20) != 0
        })
        .count();
    // The count in the system table's own JIP LN row (xNVSE's has a row of its own).
    let readme = include_str!("../../../README.md");
    let row = readme
        .lines()
        .find(|l| l.starts_with("| ") && l.contains("**Built-in script extender: JIP LN:**"))
        .expect("the README's JIP LN row");
    let at = row.find("(JIP LN: ").expect("the row's JIP count") + "(JIP LN: ".len();
    let stated: usize = row[at..]
        .split(' ')
        .next()
        .and_then(|n| n.parse().ok())
        .expect("a number");
    assert_eq!(stated, built, "README says {stated}, the table has {built}");
}
