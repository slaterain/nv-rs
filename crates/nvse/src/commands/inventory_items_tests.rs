//! Tests of the batch through the commands' records, on forms laid out where the test's cast puts
//! their components, extra data lists answered by a stand-in of `BaseExtraList::GetByType` (a
//! list's data of type `t` at `+0x100 + 4t`, its presence bits at `+8`), and the game functions
//! the commands call recorded.

use super::super::test_support::*;
use super::*;
use std::cell::RefCell;

const ARMOR: u32 = 0x0200_1000;
const WEAPON: u32 = 0x0200_2000;
const AMMO: u32 = 0x0200_3000;
const REF: u32 = 0x0200_4000;
const PLAYER_REF: u32 = 0x0200_5000;
const CHANGES: u32 = 0x0200_6000;
const XDATA: [u32; 3] = [0x0200_6400, 0x0200_6800, 0x0200_6c00];
const NODES: u32 = 0x0200_7400;
const PROCESS: u32 = 0x0200_8000;
const AMMO_INFO: u32 = 0x0200_8100;
const VTABLE: u32 = 0x0200_8200;
const HEAP: u32 = 0x0300_c000;
/// Functions of the stand-in vtables.
const FN_AMMO_INFO: u32 = 0x0060_0100;
const FN_COPY_FROM: u32 = 0x0060_0108;
const FN_SET_REF_ID: u32 = 0x0060_0128;

thread_local! {
    static CALLS: RefCell<Vec<(u32, Vec<u32>)>> = const { RefCell::new(Vec::new()) };
    static TAKEN: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

fn record(at: u32, a: &[u32]) {
    CALLS.with(|c| c.borrow_mut().push((at, a.to_vec())));
}

/// The recorded calls to `at`, and the record cleared of them.
fn calls_to(at: u32) -> Vec<Vec<u32>> {
    CALLS.with(|c| {
        let mut c = c.borrow_mut();
        let out = c.iter().filter(|(f, _)| *f == at).map(|(_, a)| a.clone()).collect();
        c.retain(|(f, _)| *f != at);
        out
    })
}

fn eax(v: u32) -> Ret {
    Ret {
        eax: v,
        ..Ret::default()
    }
}

fn get_by_type(e: &mut Engine, a: &[u32]) -> Ret {
    eax(e.mem.u32(a[0] + 0x100 + 4 * a[1]))
}

/// Marks `kind` present in a list with `data` as its extra data.
fn put_extra(e: &mut Engine, list: u32, kind: u32, data: u32) {
    let at = list + 8 + (kind >> 3);
    let bits = e.mem.u8(at) | (1 << (kind & 7));
    e.mem.set_u8(at, bits);
    e.mem.set_u32(list + 0x100 + 4 * kind, data);
}

fn take_extra(e: &mut Engine, list: u32, kind: u32) {
    let at = list + 8 + (kind >> 3);
    let bits = e.mem.u8(at) & !(1 << (kind & 7));
    e.mem.set_u8(at, bits);
    e.mem.set_u32(list + 0x100 + 4 * kind, 0);
}

/// An armor, a weapon (a pistol, type 3) and ammo (form ids `0x0001000N`), a reference to the
/// armor, the player; the form heap a bump allocator (0x400 a block, room for the stand-in lists),
/// `BaseExtraList::Add`, `Remove` and `RemoveByType` on the stand-in lists.
fn setup() -> Engine {
    CALLS.with(|c| c.borrow_mut().clear());
    let mut e = engine_with_table();
    e.register(0x0041_0220, get_by_type);
    for (form, kind, id) in [(ARMOR, 0x18, 1), (WEAPON, 0x28, 2), (AMMO, 0x29, 3)] {
        e.mem.set_u8(form + 4, kind);
        e.mem.set_u32(form + 0xc, 0x0001_0000 + id);
    }
    e.mem.set_u32(WEAPON + 0x7f0, 256);
    e.mem.set_u8(WEAPON + 0xf4, 3);
    e.mem.set_u8(REF + 4, 0x3a);
    e.mem.set_u32(REF + 0x7f0, 1);
    e.mem.set_u32(REF + 0x20, ARMOR);
    e.mem.set_u32(REF + 0xc, 0x0001_0010);
    e.mem.set_u8(PLAYER_REF + 4, 0x3b);
    e.mem.set_u32(PLAYER_REF + 0x7f0, 1 | (1 << 21));
    e.mem.set_u32(PLAYER_REF + 0xc, 0x14);
    e.set_global::<u32>(0x011d_ea3c, PLAYER_REF);
    for (i, form) in [ARMOR, WEAPON, AMMO].into_iter().enumerate() {
        set_script_ref(&mut e, i as u16 + 1, form);
    }
    e.register(0x0040_1000, |e, a| {
        let at = e.mem.u32(HEAP).max(HEAP + 0x400);
        e.mem.set_u32(HEAP, at + a[0].next_multiple_of(0x400));
        eax(at)
    });
    e.register(0x0040_1030, |_, _| Ret::default());
    e.register(0x0040_ff60, |e, a| {
        let kind = u32::from(e.mem.u8(a[1] + 4));
        put_extra(e, a[0], kind, a[1]);
        Ret::default()
    });
    e.register(0x0041_0020, |e, a| {
        record(0x0041_0020, a);
        let kind = u32::from(e.mem.u8(a[1] + 4));
        take_extra(e, a[0], kind);
        Ret::default()
    });
    e.register(0x0041_0140, |e, a| {
        record(0x0041_0140, a);
        take_extra(e, a[0], a[1]);
        Ret::default()
    });
    e.register(0x0088_c650, |_, a| {
        record(0x0088_c650, a);
        Ret::default()
    });
    e.register(0x0088_c790, |_, a| {
        record(0x0088_c790, a);
        Ret::default()
    });
    e
}

fn run(e: &mut Engine, opcode: u32, this: u32, args: &[Vec<u8>]) -> f64 {
    let (ok, result) = call(e, opcode, this, args);
    assert_eq!(ok, 1);
    result
}

fn form_of(v: f64) -> u32 {
    v.to_bits() as u32
}

/// The player's changes: entries {extend list, count delta, item} for the weapon (its lists
/// `XDATA[0]` and `XDATA[1]`, the second worn), the armor (no lists, 1) and the ammo (`XDATA[2]`,
/// 20).
fn player_inventory(e: &mut Engine) -> (u32, u32, u32) {
    put_extra(e, PLAYER_REF + 0x44, 0x15, CHANGES);
    e.mem.set_u32(CHANGES + 0xc, CHANGES + 0x20);
    e.mem.set_u32(CHANGES + 0x20, NODES);
    let entries = [NODES + 0x100, NODES + 0x110, NODES + 0x120];
    for (i, entry) in entries.into_iter().enumerate() {
        let node = NODES + 8 * i as u32;
        e.mem.set_u32(node, entry);
        e.mem
            .set_u32(node + 4, if i < 2 { node + 8 } else { 0 });
    }
    let [weapon, armor, ammo] = entries;
    e.mem.set_u32(weapon, NODES + 0x40);
    e.mem.set_u32(weapon + 4, 2);
    e.mem.set_u32(weapon + 8, WEAPON);
    e.mem.set_u32(NODES + 0x40, XDATA[0]);
    e.mem.set_u32(NODES + 0x44, NODES + 0x48);
    e.mem.set_u32(NODES + 0x48, XDATA[1]);
    put_extra(e, XDATA[1], 0x16, 0);
    e.mem.set_u32(armor + 4, 1);
    e.mem.set_u32(armor + 8, ARMOR);
    e.mem.set_u32(ammo, NODES + 0x60);
    e.mem.set_u32(ammo + 4, 20);
    e.mem.set_u32(ammo + 8, AMMO);
    e.mem.set_u32(NODES + 0x60, XDATA[2]);
    (weapon, armor, ammo)
}

/// The player's process answers an ammo info of 9 rounds of the ammo.
fn with_ammo_info(e: &mut Engine) {
    e.mem.set_u32(PLAYER_REF + 0x68, PROCESS);
    e.mem.set_u32(PROCESS, VTABLE);
    e.mem.set_u32(VTABLE + 0x14c, FN_AMMO_INFO);
    e.register(FN_AMMO_INFO, |_, _| eax(AMMO_INFO));
    e.mem.set_u32(AMMO_INFO + 4, 9);
    e.mem.set_u32(AMMO_INFO + 8, AMMO);
}

/// `GetPlayerCurrentAmmoRounds`, `SetPlayerCurrentAmmoRounds` and `GetPlayerCurrentAmmo` read and
/// write the player's ammo info; no ammo info is 0.
#[test]
fn the_players_ammo_info() {
    let mut e = setup();
    with_ammo_info(&mut e);
    assert_eq!(run(&mut e, 0x14c5, 0, &[]), 9.0);
    assert_eq!(form_of(run(&mut e, 0x14c7, 0, &[])), 0x0001_0003);
    run(&mut e, 0x14c6, 0, &[n(30)]);
    assert_eq!(e.mem.u32(AMMO_INFO + 4), 30);
    assert_eq!(run(&mut e, 0x14c5, REF, &[]), 30.0);
    e.register(FN_AMMO_INFO, |_, _| eax(0));
    assert_eq!(run(&mut e, 0x14c5, 0, &[]), 0.0);
    assert_eq!(run(&mut e, 0x14c7, 0, &[]), 0.0);
}

/// The equipped weapon's mod flags: read from its worn list (also as a condition), set 1-7 (made,
/// then changed), removed and freed for 0; 8 and up change nothing.
#[test]
fn equipped_weapon_mod_flags() {
    let mut e = setup();
    player_inventory(&mut e);
    assert_eq!(run(&mut e, 0x14f2, PLAYER_REF, &[]), 0.0);
    run(&mut e, 0x14f3, PLAYER_REF, &[n(5)]);
    let flags = e.mem.u32(XDATA[1] + 0x100 + 4 * 0x8d);
    assert_ne!(flags, 0, "made on the worn list");
    assert_eq!(e.mem.u32(flags), 0x0101_59a4);
    assert_eq!(e.mem.u8(flags + 4), 0x8d);
    assert_eq!(run(&mut e, 0x14f2, PLAYER_REF, &[]), 5.0);
    assert_eq!(eval(&mut e, 0x14f2, PLAYER_REF, 0, 0), 5.0);
    run(&mut e, 0x14f3, PLAYER_REF, &[n(8)]);
    assert_eq!(run(&mut e, 0x14f2, PLAYER_REF, &[]), 5.0);
    run(&mut e, 0x14f3, PLAYER_REF, &[n(3)]);
    assert_eq!(e.mem.u8(flags + 0xc), 3);
    run(&mut e, 0x14f3, PLAYER_REF, &[n(0)]);
    assert_eq!(calls_to(0x0041_0020), vec![vec![XDATA[1], flags, 1]]);
    assert_eq!(run(&mut e, 0x14f2, PLAYER_REF, &[]), 0.0);
    assert_eq!(eval(&mut e, 0x14f2, 0, 0, 0), 0.0);
    assert_eq!(run(&mut e, 0x14f2, 0, &[]), 0.0);
}

/// A full name component on `form` (its `String` at `+0x7D4`), holding `text` in its own buffer.
fn named(e: &mut Engine, form: u32, text: &str) {
    let bits = e.mem.u32(form + 0x7f0) | (1 << 20);
    e.mem.set_u32(form + 0x7f0, bits);
    let at = form + 0x7a0;
    e.mem.write(at, text.as_bytes());
    e.mem.set_u8(at + text.len() as u32, 0);
    e.mem.set_u32(form + 0x7d4, at);
    e.mem.set_u16(form + 0x7d8, text.len() as u16);
    e.mem.set_u16(form + 0x7da, text.len() as u16);
}

fn name_of(e: &Engine, form: u32) -> String {
    let data = e.mem.u32(form + 0x7d4);
    String::from_utf8_lossy(&e.mem.bytes(data, u32::from(e.mem.u16(form + 0x7d8)))).into()
}

/// `SetName`: the given form's name, a reference's base form's, else the caller's base form's; a
/// form without a name component is left alone. `SetNameEx` formats the name and names the
/// calling reference's base form.
#[test]
fn names_are_set() {
    let mut e = setup();
    named(&mut e, ARMOR, "Leather Armor");
    named(&mut e, WEAPON, "9mm Pistol");
    run(&mut e, 0x144c, 0, &[s("Old Armor"), r(1)]);
    assert_eq!(name_of(&e, ARMOR), "Old Armor");
    run(&mut e, 0x144c, REF, &[s("A much longer armor name")]);
    assert_eq!(name_of(&e, ARMOR), "A much longer armor name");
    set_script_ref(&mut e, 4, REF);
    run(&mut e, 0x144c, 0, &[s("Via ref"), r(4)]);
    assert_eq!(name_of(&e, ARMOR), "Via ref");
    run(&mut e, 0x144c, 0, &[s("Nothing")]);
    run(&mut e, 0x144c, 0, &[s("No name"), r(3)]);
    run(&mut e, 0x14da, 0, &[s("%g mines"), z(3.0), r(2)]);
    assert_eq!(name_of(&e, WEAPON), "3 mines");
    run(&mut e, 0x14da, REF, &[s("Worn")]);
    assert_eq!(name_of(&e, ARMOR), "Worn");
}

/// `TempCloneForm`'s stand-ins: the game makes a form of the type asked at `NEW` (bounds and a
/// vtable whose `CopyFrom` and `SetRefID` are recorded), forms with ids in `taken` exist, and the
/// script's id is `script`.
const NEW: u32 = 0x0200_9000;
fn with_cloning(e: &mut Engine, script: u32, taken: &[u32]) {
    TAKEN.with(|t| *t.borrow_mut() = taken.to_vec());
    e.map(0x011c_3000, 0x1000);
    e.mem.set_u32(SCRIPT + 0xc, script);
    e.register(0x0046_5110, |e, a| {
        record(0x0046_5110, a);
        e.mem.set_u8(NEW + 4, a[0] as u8);
        e.mem.set_u32(NEW, VTABLE + 0x400);
        e.mem.set_u32(NEW + 0xc, 0xff00_0801);
        e.mem.set_u32(NEW + 0x7f0, 1 << 22);
        eax(NEW)
    });
    e.mem.set_u32(VTABLE + 0x400 + 0x108, FN_COPY_FROM);
    e.mem.set_u32(VTABLE + 0x400 + 0x128, FN_SET_REF_ID);
    e.register(FN_COPY_FROM, |_, a| {
        record(FN_COPY_FROM, a);
        Ret::default()
    });
    e.register(FN_SET_REF_ID, |e, a| {
        record(FN_SET_REF_ID, a);
        e.mem.set_u32(a[0] + 0xc, a[1]);
        Ret::default()
    });
    e.set_global::<u32>(0x011c_3f2c, 0x0200_9800);
    e.register(0x0046_03b0, |_, a| {
        record(0x0046_03b0, a);
        eax(1)
    });
    e.register(0x0048_39c0, |_, a| eax(u32::from(TAKEN.with(|t| t.borrow().contains(&a[0])))));
}

/// `TempCloneForm`: a form of the same type, copied, with the bounds of a bound object, added to
/// the data handler; its id the result. With `bInheritModIndex` the first free id after the
/// script's, in the script's file; `IsClonedForm` answers 1 for that and for file 0xFF.
#[test]
fn clones_and_cloned_forms() {
    let mut e = setup();
    with_cloning(&mut e, 0x0100_0800, &[0x0100_0801, 0x0100_0802]);
    e.mem.set_u32(ARMOR + 0x7f0, 1 << 22);
    for i in 0..6 {
        e.mem.set_u16(ARMOR + 0x24 + 2 * i, 100 + i as u16);
    }
    assert_eq!(form_of(run(&mut e, 0x14dd, 0, &[r(1)])), 0xff00_0801);
    assert_eq!(calls_to(0x0046_5110), vec![vec![0x18]]);
    assert_eq!(calls_to(FN_COPY_FROM), vec![vec![NEW, ARMOR]]);
    assert_eq!(calls_to(0x0046_03b0), vec![vec![0x0200_9800, NEW]]);
    assert_eq!(e.mem.u16(NEW + 0x24 + 10), 105, "the bounds");
    assert_eq!(run(&mut e, 0x14de, 0, &[r(1)]), 0.0);
    set_script_ref(&mut e, 4, NEW);
    assert_eq!(run(&mut e, 0x14de, 0, &[r(4)]), 1.0, "file 0xFF");
    assert_eq!(form_of(run(&mut e, 0x14dd, REF, &[r(1), n(1)])), 0x0100_0803);
    assert_eq!(calls_to(FN_SET_REF_ID), vec![vec![NEW, 0x0100_0803, 1]]);
    assert_eq!(run(&mut e, 0x14de, 0, &[r(4)]), 1.0, "inherited");
    // The next free id leaves the script's file: the clone keeps its own.
    let mut e = setup();
    with_cloning(&mut e, 0x01ff_ffff, &[]);
    assert_eq!(form_of(run(&mut e, 0x14dd, 0, &[r(2), n(1)])), 0xff00_0801);
    assert!(calls_to(FN_SET_REF_ID).is_empty());
}

/// `EquipItem2`: the first list of the actor's stack, one weapon (a pistol) or the whole stack of
/// ammo and of a thrown weapon; the lock and message flags (no message by default); nothing for a
/// form type that cannot be equipped, an item the actor does not hold, or a non-actor.
#[test]
fn equip_item2_equips_from_the_stack() {
    let mut e = setup();
    player_inventory(&mut e);
    run(&mut e, 0x15b6, PLAYER_REF, &[r(2)]);
    run(&mut e, 0x15b6, PLAYER_REF, &[r(3), n(1), n(0)]);
    run(&mut e, 0x15b6, PLAYER_REF, &[r(1)]);
    e.mem.set_u8(WEAPON + 0xf4, 10);
    run(&mut e, 0x15b6, PLAYER_REF, &[r(2)]);
    assert_eq!(
        calls_to(0x0088_c650),
        vec![
            vec![PLAYER_REF, WEAPON, 1, XDATA[0], 1, 0, 1],
            vec![PLAYER_REF, AMMO, 20, XDATA[2], 1, 1, 0],
            vec![PLAYER_REF, ARMOR, 1, 0, 1, 0, 1],
            vec![PLAYER_REF, WEAPON, 2, XDATA[0], 1, 0, 1],
        ]
    );
    e.mem.set_u8(AMMO + 4, 0x1f);
    run(&mut e, 0x15b6, PLAYER_REF, &[r(3)]);
    run(&mut e, 0x15b6, REF, &[r(2)]);
    e.mem.set_u32(NODES + 0x10, 0);
    e.mem.set_u8(AMMO + 4, 0x29);
    run(&mut e, 0x15b6, PLAYER_REF, &[r(3)]);
    assert!(calls_to(0x0088_c650).is_empty());
}

/// Calls `opcode` as `call` does, with a containing reference.
fn call_in(e: &mut Engine, opcode: u32, this: u32, containing: u32) {
    e.mem.set_u16(DATA, opcode as u16);
    e.mem.set_u16(DATA + 2, 2);
    e.mem.set_u16(DATA + 4, 0);
    e.mem.set_u32(OFFSET, 4);
    let record = crate::table::record_of(e, opcode);
    let execute = e.mem.u32(record + 0x18);
    let params = e.mem.u32(record + 0x14);
    let ok = e
        .call(
            execute,
            &[params, DATA, this, containing, SCRIPT, LIST, RESULT, OFFSET],
        )
        .eax;
    assert_eq!(ok & 0xff, 1);
}

/// `EquipMe` and `UnequipMe` on an object whose containing reference is an actor: equipped with
/// its own extra data list, unequipped by its base form alone; nothing without an owner that is
/// an actor (and no inventory reference), or without a calling object.
#[test]
fn equip_me_and_unequip_me_use_the_owner() {
    let mut e = setup();
    call_in(&mut e, 0x15b7, REF, PLAYER_REF);
    call_in(&mut e, 0x15b8, REF, PLAYER_REF);
    assert_eq!(
        calls_to(0x0088_c650),
        vec![vec![PLAYER_REF, ARMOR, 1, REF + 0x44, 1, 0, 1]]
    );
    assert_eq!(
        calls_to(0x0088_c790),
        vec![vec![PLAYER_REF, ARMOR, 1, 0, 1, 0, 1]]
    );
    call_in(&mut e, 0x15b7, REF, REF);
    call_in(&mut e, 0x15b8, REF, 0);
    call_in(&mut e, 0x15b7, 0, PLAYER_REF);
    assert!(calls_to(0x0088_c650).is_empty());
    assert!(calls_to(0x0088_c790).is_empty());
}

fn hotkey_of(e: &Engine, list: u32) -> Option<u8> {
    let x = e.mem.u32(list + 0x100 + 4 * 0x4a);
    (x != 0).then(|| e.mem.u8(x + 0xc))
}

/// The player's hotkeys: set on the item's first list (made with its vtable), read back, moved to
/// another item (taken off the first; the armor's stack had no lists, so it gets one and counts
/// one more item, as in xNVSE), cleared. Hotkey 0 stores 255 when set and finds nothing when read.
#[test]
fn hotkeys_move_between_the_players_stacks() {
    let mut e = setup();
    let (_, armor, _) = player_inventory(&mut e);
    run(&mut e, 0x15bb, 0, &[n(3), r(2)]);
    assert_eq!(hotkey_of(&e, XDATA[0]), Some(2));
    let made = e.mem.u32(XDATA[0] + 0x100 + 4 * 0x4a);
    assert_eq!(e.mem.u32(made), 0x0101_592c);
    assert_eq!(form_of(run(&mut e, 0x144d, 0, &[n(3)])), 0x0001_0002);
    assert_eq!(run(&mut e, 0x144d, 0, &[n(4)]), 0.0);
    run(&mut e, 0x15bb, 0, &[n(3), r(1)]);
    assert_eq!(hotkey_of(&e, XDATA[0]), None, "taken off the weapon");
    let list = e.mem.u32(e.mem.u32(armor));
    assert_ne!(list, 0);
    assert_eq!(e.mem.u32(list), 0, "xNVSE's list has no vtable");
    assert_eq!(hotkey_of(&e, list), Some(2));
    assert_eq!(e.mem.u32(armor + 4), 2, "one more armor");
    assert_eq!(form_of(run(&mut e, 0x144d, 0, &[n(3)])), 0x0001_0001);
    // A second hotkey on the armor changes the one it has.
    run(&mut e, 0x15bb, 0, &[n(6), r(1)]);
    assert_eq!(hotkey_of(&e, list), Some(5));
    assert_eq!(run(&mut e, 0x144d, 0, &[n(3)]), 0.0);
    run(&mut e, 0x15bc, 0, &[n(6)]);
    assert_eq!(hotkey_of(&e, list), None);
    assert_eq!(calls_to(0x0041_0140).len(), 2);
    run(&mut e, 0x15bb, 0, &[n(0), r(3)]);
    assert_eq!(hotkey_of(&e, XDATA[2]), Some(255));
    assert_eq!(run(&mut e, 0x144d, 0, &[n(0)]), 0.0);
    run(&mut e, 0x15bb, 0, &[n(9), r(3)]);
    run(&mut e, 0x15bc, 0, &[n(9)]);
    assert_eq!(hotkey_of(&e, XDATA[2]), Some(255));
}

/// A player's entry list that runs back into itself: the hotkey walk stops the game, or ends with
/// the list guard row.
#[test]
fn a_cyclic_entry_list_stops_the_game_or_ends_the_walk() {
    let mut e = setup();
    player_inventory(&mut e);
    e.mem.set_u32(NODES + 0x14, NODES);
    let stop = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        call(&mut e, 0x144d, 0, &[n(8)]);
    }));
    assert!(stop.is_err());
    let mut e = setup();
    player_inventory(&mut e);
    e.mem.set_u32(NODES + 0x14, NODES);
    crate::commands::array::set_fix(&mut e, crate::commands::array::LIST_GUARD, true);
    assert_eq!(run(&mut e, 0x144d, 0, &[n(8)]), 0.0);
    run(&mut e, 0x15bb, 0, &[n(8), r(3)]);
    run(&mut e, 0x15bc, 0, &[n(8)]);
}
