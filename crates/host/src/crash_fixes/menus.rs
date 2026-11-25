//! The crash and leak fixes of the menus: the Pip-Boy inventory, containers, the repair menu, the
//! companion wheel ([`super`]).
//!
//! Written clean-room from behaviour specs; every address and offset below was checked in
//! FalloutNV.exe 1.4.0.525.

use super::Part;
use engine::prelude::*;
use engine::units::fallout_ai::actor_magic::actor_drink_potion;
use engine::units::fallout_interface::bartermenu::{fn_007308e0, list_box_item_change_remove_all};
use engine::units::fallout_interface::companionwheelmenu::companion_wheel_menu_do_use_stim;
use engine::units::fallout_interface::containermenu::container_menu_do_idle;
use engine::units::fallout_interface::inventorymenu::{
    inventory_menu_do_click, inventory_menu_drop_current_item, inventory_menu_select_current_item,
};
use std::cell::Cell;

/// `InventoryMenu::DoClick`, `DropCurrentItem` and `SelectCurrentItem` (Xbox PDB).
pub const DO_CLICK: u32 = 0x0078_0140;
pub const DROP_CURRENT_ITEM: u32 = 0x0078_0c50;
pub const SELECT_CURRENT_ITEM: u32 = 0x0078_0d60;
/// `ContainerMenu::DoIdle` (Xbox PDB).
pub const CONTAINER_DO_IDLE: u32 = 0x0075_eac0;
/// The item list box's "remove all" (`ListBox<ItemChange *>`, engine map) and its destructor,
/// which empties the list the same way first.
pub const LIST_REMOVE_ALL: u32 = 0x0073_0950;
pub const LIST_DESTRUCTOR: u32 = 0x0073_08e0;
/// `CompanionWheelMenu::Do_UseStim` (engine map) and `Actor::DrinkPotion` (Xbox PDB).
pub const DO_USE_STIM: u32 = 0x0075_6490;
pub const DRINK_POTION: u32 = 0x008c_1f80;
/// The item list box's fill from a container's inventory (unnamed; `thiscall`, seven words).
pub const LIST_FILL: u32 = 0x0071_9ef0;

/// The inventory's highlighted entry (an `ItemChange`; 0 when none is highlighted) and the
/// container menu's.
const INVENTORY_ENTRY: u32 = 0x011d_9ea8;
const CONTAINER_ENTRY: u32 = 0x011d_93fc;
/// An inventory entry's form (`0044ddc0`: the word at +8).
fn entry_form(e: &mut Engine, entry: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, entry))
}

/// The inventory's use button: equip, hot-key, or (with the keyring open) swap ammunition.
const USE_BUTTON: i32 = 0x1d;

/// 002: the use button with no entry highlighted.
pub const INVENTORY_USE: Part = Part {
    name: "inventory use with nothing selected",
    env: "NV_CRASH_FIX_INVENTORY_USE",
    addresses: &[DO_CLICK],
    summary: "the Pip-Boy inventory's use click (or its gamepad button) with no item selected does nothing instead of stopping the game",
    install: install_inventory_use,
};

fn install_inventory_use(e: &mut Engine) {
    e.register(DO_CLICK, inventory_click);
}

/// `DoClick`'s use case tests the highlighted entry, but its "no entry" branch falls into the
/// ammunition step after it, which reads the entry's form (`0x78062d`). With no entry the use
/// click does nothing more; every other click, and every click with an entry, is the game's.
fn inventory_click(e: &mut Engine, a: &[u32]) -> Ret {
    let (this, id, tile) = (a[0], a[1] as i32, a[2]);
    if id == USE_BUTTON && e.global::<u32>(INVENTORY_ENTRY) == 0 {
        return Ret::default();
    }
    inventory_menu_do_click(e, Ptr::new(this), id, tile);
    Ret::default()
}

/// 003: the quantity menu's drop after the stack is gone.
pub const QUANTITY_DROP: Part = Part {
    name: "quantity drop after the item is gone",
    env: "NV_CRASH_FIX_QUANTITY_DROP",
    addresses: &[DROP_CURRENT_ITEM],
    summary: "confirming the Pip-Boy's \"how many\" drop after the stack was taken away drops nothing instead of stopping the game",
    install: install_quantity_drop,
};

fn install_quantity_drop(e: &mut Engine) {
    e.register(DROP_CURRENT_ITEM, drop_current_entry);
}

/// The drop (the quantity menu's callback) reads the entry's form and extra list before any
/// test. With no entry nothing is dropped.
fn drop_current_entry(e: &mut Engine, a: &[u32]) -> Ret {
    if e.global::<u32>(INVENTORY_ENTRY) != 0 {
        inventory_menu_drop_current_item(e, a[0] as i32);
    }
    Ret::default()
}

/// 004: equip or unequip with no entry highlighted.
pub const INVENTORY_EQUIP: Part = Part {
    name: "inventory equip with nothing selected",
    env: "NV_CRASH_FIX_INVENTORY_EQUIP",
    addresses: &[SELECT_CURRENT_ITEM],
    summary: "equipping or unequipping from the Pip-Boy inventory with no item selected does nothing instead of stopping the game",
    install: install_inventory_equip,
};

fn install_inventory_equip(e: &mut Engine) {
    e.register(SELECT_CURRENT_ITEM, select_current_entry);
}

/// Equipping reads the entry's form and extra list before any test. With no entry: nothing
/// equipped, nothing unequipped, the menu not refreshed.
fn select_current_entry(e: &mut Engine, a: &[u32]) -> Ret {
    if e.global::<u32>(INVENTORY_ENTRY) != 0 {
        inventory_menu_select_current_item(e, Ptr::new(a[0]));
    }
    Ret::default()
}

/// 005: the container menu's hover over an entry whose item has no form.
pub const CONTAINER_HOVER: Part = Part {
    name: "container hover over an item without a form",
    env: "NV_CRASH_FIX_CONTAINER_HOVER",
    addresses: &[CONTAINER_DO_IDLE],
    summary: "pointing at a container or companion inventory entry whose item has no form any more shows no item card instead of stopping the game",
    install: install_container_hover,
};

fn install_container_hover(e: &mut Engine) {
    e.register(CONTAINER_DO_IDLE, container_idle);
}

/// `ContainerMenu`'s subtitle clear time (`DoIdle` reads `+0xfc`).
const SUBTITLE_CLEAR_TIME: u32 = 0xfc;
/// The clock (`00457fe0`), `Tile::GetChildByName`-like lookup (`00a08b20`), the subtitle tile's
/// name `"CM_Subtitle"`, the string trait setter (`00a01350`), the string trait and the empty
/// string.
const CURRENT_TIME: u32 = 0x0045_7fe0;
const TILE_FIND_CHILD: u32 = 0x00a0_8b20;
const SUBTITLE_NAME: u32 = 0x0107_21f4;
const SET_STRING_TRAIT: u32 = 0x00a0_1350;
const TRAIT_STRING: u32 = 0xfc4;
const EMPTY_STRING: u32 = 0x0101_1584;
const CONTAINER_MENU: u32 = 0x011d_93f8;

/// `DoIdle` first clears a subtitle whose time is up, then fills the item card of the hovered
/// entry: its form's type picks a weapon's card (`0x28`), then an armour's (`0x18`), and the type
/// is read through the form with no test. An entry whose form is gone ends `DoIdle` after the
/// subtitle step, as an entry that is neither does; any other idle is the game's.
fn container_idle(e: &mut Engine, a: &[u32]) -> Ret {
    let this = a[0];
    let entry = e.global::<u32>(CONTAINER_ENTRY);
    if entry != 0 && entry_form(e, entry) == 0 {
        clear_subtitle_when_due(e, this);
    } else {
        container_menu_do_idle(e, Ptr::new(this));
    }
    Ret::default()
}

/// `DoIdle`'s first step: a subtitle whose time is up is cleared.
fn clear_subtitle_when_due(e: &mut Engine, this: u32) {
    let due = e.mem.u32(this + SUBTITLE_CLEAR_TIME);
    if due == 0 || e.call(CURRENT_TIME, &args![]).u32() <= due {
        return;
    }
    e.mem.set_u32(this + SUBTITLE_CLEAR_TIME, 0);
    let menu = e.global::<u32>(CONTAINER_MENU);
    let menu_tile = typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, menu));
    let subtitle = e
        .call(TILE_FIND_CHILD, &args![menu_tile, SUBTITLE_NAME])
        .u32();
    if subtitle != 0 {
        e.call(
            SET_STRING_TRAIT,
            &args![subtitle, TRAIT_STRING, EMPTY_STRING, true],
        );
    }
}

/// 009: the repair menu's item freed with the list behind it.
pub const REPAIR_TARGET: Part = Part {
    name: "repair target kept while the repair menu is open",
    env: "NV_CRASH_FIX_REPAIR_TARGET",
    addresses: &[LIST_REMOVE_ALL, LIST_DESTRUCTOR],
    summary: "an item list rebuilt behind the open repair menu no longer frees the item being repaired, so the menu keeps the right item and values and the repair no longer stops the game",
    install: install_repair_target,
};

fn install_repair_target(e: &mut Engine) {
    e.register(LIST_REMOVE_ALL, remove_all_keeping_repair_target);
    e.register(LIST_DESTRUCTOR, destroy_keeping_repair_target);
}

/// `RepairMenu::pMe` and `RepairMenu::pBrokenItem` (Xbox PDB): the open repair menu and the
/// entry it repairs. The menu's destructor clears only the first.
const REPAIR_MENU: u32 = 0x011d_a75c;
const REPAIR_TARGET_ENTRY: u32 = 0x011d_a760;

/// Emptying an item list deletes every entry's data (`004459e0`, when the list owns it), the
/// open repair menu's target among them, and the menu then works on freed memory. While the
/// repair menu is open, the list item that holds the target gives it up first: the list empties
/// as the game's, every other entry's data freed, and the target stays where the menu keeps it.
/// The target is not freed later: the inventory's highlighted entry may name it too, and one
/// entry a rebuild is too small to matter.
fn remove_all_keeping_repair_target(e: &mut Engine, a: &[u32]) -> Ret {
    keep_repair_target(e, a[0]);
    list_box_item_change_remove_all(e, Ptr::new(a[0]));
    Ret::default()
}

fn destroy_keeping_repair_target(e: &mut Engine, a: &[u32]) -> Ret {
    keep_repair_target(e, a[0]);
    fn_007308e0(e, Ptr::new(a[0]));
    Ret::default()
}

/// The list items "remove all" reaches (the list's first item at +4, then the nodes from +8,
/// item at +0 and next at +4, up to the first node with no item, as its walk goes): the one
/// whose data (+4) is the repair menu's target forgets it.
fn keep_repair_target(e: &mut Engine, list: u32) {
    let target = e.global::<u32>(REPAIR_TARGET_ENTRY);
    if e.global::<u32>(REPAIR_MENU) == 0 || target == 0 {
        return;
    }
    let mut items = Vec::new();
    let mut node = e.mem.u32(list + 8);
    while node != 0 {
        let item = e.mem.u32(node);
        if item == 0 {
            break;
        }
        items.push(item);
        node = e.mem.u32(node + 4);
    }
    items.push(e.mem.u32(list + 4));
    for item in items.into_iter().filter(|&i| i != 0) {
        if e.mem.u32(item + 4) == target {
            e.mem.set_u32(item + 4, 0);
        }
    }
}

/// 055: the wheel's stimpak taken from the companion too.
pub const WHEEL_STIMPAK: Part = Part {
    name: "companion wheel stimpak taken once",
    env: "NV_CRASH_FIX_WHEEL_STIMPAK",
    addresses: &[DO_USE_STIM, DRINK_POTION],
    summary: "\"Use Stimpak\" in the companion wheel takes one stimpak, the player's, and no longer one of the companion's own as well",
    install: install_wheel_stimpak,
};

fn install_wheel_stimpak(e: &mut Engine) {
    e.register(DO_USE_STIM, wheel_use_stimpak);
    e.register(DRINK_POTION, drink_potion);
}

thread_local! {
    /// Set while the wheel's stimpak use runs on this thread.
    static IN_WHEEL_STIMPAK: Cell<bool> = const { Cell::new(false) };
}

/// Clears [`IN_WHEEL_STIMPAK`] when the use ends, a stop included.
struct WheelStimpak;

impl WheelStimpak {
    fn begin() -> WheelStimpak {
        IN_WHEEL_STIMPAK.with(|c| c.set(true));
        WheelStimpak
    }
}

impl Drop for WheelStimpak {
    fn drop(&mut self) {
        IN_WHEEL_STIMPAK.with(|c| c.set(false));
    }
}

fn wheel_use_stimpak(e: &mut Engine, a: &[u32]) -> Ret {
    let _use = WheelStimpak::begin();
    companion_wheel_menu_do_use_stim(e, Ptr::new(a[0]));
    Ret::default()
}

/// The player (`011dea3c`), an actor's magic caster (+0x88), the caster's cast slot (+0x40,
/// given the potion's effect list at +0x30) and `MagicCaster::ReleaseCast` (`00815870`).
const PLAYER: u32 = 0x011d_ea3c;
const MAGIC_CASTER: u32 = 0x88;
const CAST_SLOT: u32 = 0x40;
const POTION_EFFECTS: u32 = 0x30;
const RELEASE_CAST: u32 = 0x0081_5870;

/// `Do_UseStim` makes the companion drink the player's stimpak, and for an actor other than the
/// player `DrinkPotion` casts the potion and then removes one of it from the drinker (its
/// `+0x17c`), the companion; the wheel then removes the player's. Inside the wheel's use the
/// companion's drink casts as the game's and removes nothing; every other drink is the game's.
fn drink_potion(e: &mut Engine, a: &[u32]) -> Ret {
    let (actor, potion, extra, consume_now) = (a[0], a[1], a[2], a[3] as u8 != 0);
    let wheel = IN_WHEEL_STIMPAK.with(Cell::get);
    if !wheel || actor == e.global::<u32>(PLAYER) {
        let drunk = actor_drink_potion(
            e,
            Ptr::new(actor),
            Ptr::new(potion),
            Ptr::new(extra),
            consume_now,
        );
        return u32::from(drunk).into_ret();
    }
    if potion == 0 {
        return 0u32.into_ret();
    }
    let caster = actor + MAGIC_CASTER;
    e.vcall(caster, CAST_SLOT, &args![potion + POTION_EFFECTS]);
    e.call(RELEASE_CAST, &args![caster, 0u32]);
    1u32.into_ret()
}

/// 059: an item list with an item whose form has no name.
pub const NAMELESS_ITEM: Part = Part {
    name: "item list with a nameless item",
    env: "NV_CRASH_FIX_NAMELESS_ITEM",
    addresses: &[LIST_FILL],
    summary: "a container, barter or Pip-Boy list with an item whose form has no name lists it blank instead of stopping the game",
    install: install_nameless_item,
};

fn install_nameless_item(e: &mut Engine) {
    e.register(LIST_FILL, fill_list);
}

/// The list's own slot `+0x1c`, `007a1950` and `007242c0` (its state), the inventory iterator
/// (`00575510`) and its next item (`004ca330`) and delete (`004bf630`), `__RTDynamicCast` from
/// `TESBoundObject` (`01183108`) to `TESFullName` (`01183158`), the name (`00408da0`), its
/// length (`0044a670`), the copy and append (`00406d30`, `00406d50`) and the modded suffix,
/// the item's "has mods" (`004bd820`), a new item (`00401000`, `0076b630`, `004bc810`), its count
/// (`006ecd40`), the list's add (`0071a320`) and synchronize (`0071a670`).
const LIST_SET: u32 = 0x007a_1950;
const LIST_STATE: u32 = 0x0072_42c0;
const INVENTORY_ITERATOR: u32 = 0x0057_5510;
const NEXT_ITEM: u32 = 0x004c_a330;
const ITERATOR_DELETE: u32 = 0x004b_f630;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const BOUND_OBJECT_TYPE: u32 = 0x0118_3108;
const FULL_NAME_TYPE: u32 = 0x0118_3158;
const NAME: u32 = 0x0040_8da0;
const STRING_LENGTH: u32 = 0x0044_a670;
const STRING_COPY: u32 = 0x0040_6d30;
const STRING_APPEND: u32 = 0x0040_6d50;
const MODDED_SUFFIX: u32 = 0x0103_45e8;
const ITEM_HAS_MODS: u32 = 0x004b_d820;
const ALLOCATE: u32 = 0x0040_1000;
const ITEM_CONSTRUCTOR: u32 = 0x0076_b630;
const ITEM_COPY: u32 = 0x004b_c810;
const ITEM_SET_COUNT: u32 = 0x006e_cd40;
const LIST_ADD_ITEM: u32 = 0x0071_a320;
const LIST_SYNCHRONIZE: u32 = 0x0071_a670;

/// The fill, as the game's, step for step, except that the name of an item whose form has no
/// name component (the cast gives none) reads as the empty string in both places the game reads
/// it (`0x71a019` and `0x71a02d`, through a null `TESFullName`): the item is listed, blank.
fn fill_list(e: &mut Engine, a: &[u32]) -> Ret {
    let (this, reference, filter, word, callback) = (a[0], a[1], a[2], a[3], a[4]);
    let (split, flags) = (a[5] as u8 != 0, a[6] as u8);
    if reference == 0 || e.mem.u32(this + 0xc) == 0 || e.mem.u32(this + 0x18) == 0 {
        return Ret::default();
    }
    e.vcall(this, 0x1c, &args![]);
    e.call(LIST_SET, &args![this, 1u32, 1u32]);
    let kept = e.call(LIST_STATE, &args![this, 0u32]).u8();
    e.call(LIST_SET, &args![this, 0u32, 0u32]);
    e.with_stack(0x88, |e, locals| {
        let iterator_slot = locals.addr();
        let changes_slot = locals.addr() + 4;
        let buffer = locals.addr() + 8;
        e.call(
            INVENTORY_ITERATOR,
            &args![reference, u32::from(flags), iterator_slot, changes_slot],
        );
        let iterator = e.mem.u32(iterator_slot);
        let changes = e.mem.u32(changes_slot);
        if iterator == 0 || changes == 0 {
            return;
        }
        let mut item = e.call(NEXT_ITEM, &args![changes, iterator]).u32();
        while item != 0 {
            if filter == 0 || e.call(filter, &args![item]).u8() == 0 {
                list_item(e, this, item, buffer, word, callback, split);
            }
            item = e.call(NEXT_ITEM, &args![changes, iterator]).u32();
        }
        let iterator = e.mem.u32(iterator_slot);
        if iterator != 0 {
            e.call(ITERATOR_DELETE, &args![iterator, 1u32]);
        }
    });
    if word != 0 {
        e.call(LIST_SYNCHRONIZE, &args![this]);
    }
    e.call(LIST_SET, &args![this, 0u32, u32::from(kept)]);
    Ret::default()
}

/// One item's entries (one, or one per unit with `split`, each unit after the first a copy).
fn list_item(
    e: &mut Engine,
    this: u32,
    mut item: u32,
    buffer: u32,
    word: u32,
    callback: u32,
    split: bool,
) {
    let mut copies = 0i32;
    let count = typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, item))
        .into_ret()
        .i32();
    loop {
        let form = entry_form(e, item);
        let full_name = e
            .call(
                DYNAMIC_CAST,
                &args![form, 0u32, BOUND_OBJECT_TYPE, FULL_NAME_TYPE, 0u32],
            )
            .u32();
        let length = if full_name == 0 {
            e.mem.set_u8(buffer, 0);
            0
        } else {
            let name = e.call(NAME, &args![full_name]).u32();
            let length = e.call(STRING_LENGTH, &args![name]).u32();
            let name = e.call(NAME, &args![full_name]).u32();
            e.call(STRING_COPY, &args![buffer, length.wrapping_add(1), name]);
            length
        };
        if e.call(ITEM_HAS_MODS, &args![item]).u8() != 0 {
            e.call(
                STRING_APPEND,
                &args![buffer, length.wrapping_add(2), MODDED_SUFFIX],
            );
        }
        if split {
            if copies > 0 {
                let block = e.call(ALLOCATE, &args![0xcu32]).u32();
                let copy = if block != 0 {
                    e.call(ITEM_CONSTRUCTOR, &args![block]).u32()
                } else {
                    0
                };
                e.call(ITEM_COPY, &args![copy, item]);
                item = copy;
                e.call(ITEM_SET_COUNT, &args![item, 1u32]);
            } else if count > 1 {
                e.call(ITEM_SET_COUNT, &args![item, 1u32]);
            }
            copies += 1;
        }
        let entry = e
            .call(LIST_ADD_ITEM, &args![this, item, buffer, word, 0u32])
            .u32();
        if callback != 0 {
            e.call(callback, &args![entry, item]);
        }
        if !(split && copies < count) {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::rc::Rc;

    /// A test engine with `part` installed or not, the pages of the globals mapped and every
    /// call logged.
    fn engine(part: &Part, fixed: bool) -> Engine {
        let mut e = Engine::new();
        if fixed {
            (part.install)(&mut e);
        }
        // The globals read, and the item tiles' keep flag (`011d8909`) and the drop position
        // (`011f426c`) the game's functions read.
        for page in [
            INVENTORY_ENTRY,
            CONTAINER_ENTRY,
            REPAIR_MENU,
            PLAYER,
            EMPTY_STRING,
            0x011d_8909,
            0x011f_426c,
        ] {
            e.map(page & !0xfff, 0x1000);
        }
        e.call_log = Some(vec![]);
        e
    }

    /// The stop's text, or `None` when `f` ran to its end.
    fn stop(f: impl FnOnce()) -> Option<String> {
        let out = catch_unwind(AssertUnwindSafe(f)).err()?;
        Some(
            out.downcast_ref::<String>()
                .cloned()
                .or_else(|| out.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default(),
        )
    }

    /// A call's answer, not looked at.
    fn answered(_: Ret) {}

    fn calls(e: &Engine) -> Vec<u32> {
        e.call_log.as_ref().unwrap().iter().map(|c| c.0).collect()
    }

    fn calls_to(e: &Engine, callee: u32) -> usize {
        calls(e).iter().filter(|&&c| c == callee).count()
    }

    /// Whether the run called nothing but `at` (the test's own call).
    fn only(e: &Engine, at: u32) -> bool {
        calls(e).iter().all(|&c| c == at)
    }

    /// An entry at memory nobody mapped: what any read through it stops at.
    const GONE: u32 = 0x0f00_0000;

    /// The use click with no entry: the game's falls into the ammunition step and reads the
    /// entry's form at address 8; fixed it returns having called nothing. An entry the click can
    /// read is the game's either way (here one at unmapped memory: the same stop).
    #[test]
    fn the_use_click_with_no_entry() {
        for fixed in [false, true] {
            let mut e = engine(&INVENTORY_USE, fixed);
            let this = e.mem.alloc(0x130);
            e.set_global(INVENTORY_ENTRY, 0u32);
            let stopped = stop(|| {
                e.call(DO_CLICK, &args![this, USE_BUTTON, 0u32]);
            });
            if fixed {
                assert_eq!(stopped, None);
                assert!(only(&e, DO_CLICK), "{:x?}", calls(&e));
            } else {
                assert!(stopped.unwrap().contains("00000008"));
            }
            let mut e = engine(&INVENTORY_USE, fixed);
            e.set_global(INVENTORY_ENTRY, GONE);
            let stopped = stop(|| {
                e.call(DO_CLICK, &args![this, USE_BUTTON, 0u32]);
            });
            assert!(stopped.is_some());
        }
    }

    /// The other buttons that need an entry test it themselves: the same answers either way.
    #[test]
    fn other_clicks_with_no_entry_are_the_games() {
        for fixed in [false, true] {
            for id in [7, 8, 0x13] {
                let mut e = engine(&INVENTORY_USE, fixed);
                let this = e.mem.alloc(0x130);
                assert_eq!(
                    stop(|| answered(e.call(DO_CLICK, &args![this, id, 0u32]))),
                    None
                );
                assert!(only(&e, DO_CLICK), "{:x?}", calls(&e));
            }
        }
    }

    /// The quantity menu's drop of 1 with the stack gone: the game's stops at the form read;
    /// fixed nothing is dropped and nothing called. With a count of 0 the same. An entry still
    /// there goes to the game's drop.
    #[test]
    fn the_quantity_drop_with_no_entry() {
        for count in [1i32, 0] {
            for fixed in [false, true] {
                let mut e = engine(&QUANTITY_DROP, fixed);
                let stopped = stop(|| answered(e.call(DROP_CURRENT_ITEM, &args![count])));
                if fixed {
                    assert_eq!(stopped, None);
                    assert!(only(&e, DROP_CURRENT_ITEM), "{:x?}", calls(&e));
                } else {
                    assert!(stopped.unwrap().contains("00000008"));
                }
            }
        }
        let mut e = engine(&QUANTITY_DROP, true);
        e.set_global(INVENTORY_ENTRY, GONE);
        assert!(stop(|| answered(e.call(DROP_CURRENT_ITEM, &args![1i32]))).is_some());
    }

    /// Equipping with no entry: the game's stops at the form read; fixed no equip, no unequip,
    /// no menu update.
    #[test]
    fn the_equip_with_no_entry() {
        for fixed in [false, true] {
            let mut e = engine(&INVENTORY_EQUIP, fixed);
            let this = e.mem.alloc(0x130);
            let stopped = stop(|| answered(e.call(SELECT_CURRENT_ITEM, &args![this])));
            if fixed {
                assert_eq!(stopped, None);
                assert!(only(&e, SELECT_CURRENT_ITEM), "{:x?}", calls(&e));
            } else {
                assert!(stopped.unwrap().contains("00000008"));
            }
        }
        let mut e = engine(&INVENTORY_EQUIP, true);
        let this = e.mem.alloc(0x130);
        e.set_global(INVENTORY_ENTRY, GONE);
        assert!(stop(|| answered(e.call(SELECT_CURRENT_ITEM, &args![this]))).is_some());
    }

    const LIST_COUNT: u32 = 0x0072_6c40;
    const ITEM_ENTERED: u32 = 0x011a_002c;
    const SET_ITEM_CARD: u32 = 0x0071_9790;

    /// A container menu whose list has items, an entry entered, the hovered entry's form `form`
    /// and a subtitle due at `due` (the clock answers 10).
    fn container(fixed: bool, form: u32, due: u32) -> (Engine, u32) {
        let mut e = engine(&CONTAINER_HOVER, fixed);
        e.map(ITEM_ENTERED & !0xfff, 0x1000);
        e.mem.set_u8(ITEM_ENTERED, 1);
        let this = e.mem.alloc(0x100);
        e.mem.set_u32(this + SUBTITLE_CLEAR_TIME, due);
        e.set_global(CONTAINER_MENU, this);
        let entry = e.mem.alloc(0xc);
        e.mem.set_u32(entry + 8, form);
        e.set_global(CONTAINER_ENTRY, entry);
        e.register(LIST_COUNT, |_, _| 1u32.into_ret());
        e.register(CURRENT_TIME, |_, _| 10u32.into_ret());
        e.register(TILE_FIND_CHILD, |_, _| 0u32.into_ret());
        e.register(SET_ITEM_CARD, |_, _| Ret::default());
        (e, this)
    }

    /// Hovering an entry whose form is gone: the game's stops reading the type at address 4;
    /// fixed `DoIdle` ends with no card, after clearing the subtitle that was due.
    #[test]
    fn the_container_hover_over_an_entry_without_a_form() {
        for fixed in [false, true] {
            let (mut e, this) = container(fixed, 0, 5);
            let stopped = stop(|| answered(e.call(CONTAINER_DO_IDLE, &args![this])));
            if fixed {
                assert_eq!(stopped, None);
                assert_eq!(calls_to(&e, SET_ITEM_CARD), 0);
                assert_eq!(e.mem.u32(this + SUBTITLE_CLEAR_TIME), 0);
            } else {
                assert!(stopped.unwrap().contains("00000004"));
            }
        }
        // A subtitle not yet due stays.
        let (mut e, this) = container(true, 0, 50);
        e.call(CONTAINER_DO_IDLE, &args![this]);
        assert_eq!(e.mem.u32(this + SUBTITLE_CLEAR_TIME), 50);
    }

    /// A weapon's entry gets its card either way, through the game's `DoIdle`.
    #[test]
    fn the_container_hover_over_a_weapon_is_the_games() {
        for fixed in [false, true] {
            let (mut e, this) = container(fixed, 0, 0);
            let weapon = e.mem.alloc(0x10);
            e.mem.set_u8(weapon + 4, 0x28);
            let entry = e.global::<u32>(CONTAINER_ENTRY);
            e.mem.set_u32(entry + 8, weapon);
            e.call(CONTAINER_DO_IDLE, &args![this]);
            assert_eq!(calls_to(&e, SET_ITEM_CARD), 1);
        }
    }

    const ITEM_DELETE: u32 = 0x0044_59e0;
    const LIST_HAS_FLAG: u32 = 0x0072_42c0;
    const FREE: u32 = 0x0040_1030;

    /// A list of three items whose data blocks are `data`, owning them; the repair menu open on
    /// `target` when given. The tiles are none, the nodes are freed by the test's free, the
    /// rest of "remove all" answers nothing. The data blocks deleted, in order.
    fn empty_list(fixed: bool, target: Option<usize>, destroy: bool) -> (Vec<u32>, Vec<u32>, u32) {
        let mut e = engine(&REPAIR_TARGET, fixed);
        let data: Vec<u32> = (0..3).map(|_| e.mem.alloc(0xc)).collect();
        let items: Vec<u32> = data
            .iter()
            .map(|&d| {
                let item = e.mem.alloc(8);
                e.mem.set_u32(item + 4, d);
                item
            })
            .collect();
        let list = e.mem.alloc(0x30);
        e.mem.set_u32(list + 4, items[0]);
        let mut next = 0;
        for &item in items[1..].iter().rev() {
            let node = e.mem.alloc(8);
            e.mem.set_u32(node, item);
            e.mem.set_u32(node + 4, next);
            next = node;
        }
        e.mem.set_u32(list + 8, next);
        if let Some(t) = target {
            let menu = e.mem.alloc(0x10);
            e.set_global(REPAIR_MENU, menu);
            e.set_global(REPAIR_TARGET_ENTRY, data[t]);
        }
        let deleted = Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = deleted.clone();
        e.register_double(ITEM_DELETE, move |_, a| {
            log.borrow_mut().push(a[0]);
            Ret::default()
        });
        e.register(LIST_HAS_FLAG, |_, _| 1u32.into_ret());
        // The item's free, the node's next and delete, the list's height and highlight, and the
        // destructor's own two (`007211c0`, `0046ffb0`).
        for stub in [
            FREE,
            0x006e_cd40,
            0x0047_02f0,
            0x0071_ad30,
            0x0071_a740,
            0x0072_11c0,
            0x0046_ffb0,
        ] {
            e.register(stub, |_, _| Ret::default());
        }
        let at = if destroy {
            LIST_DESTRUCTOR
        } else {
            LIST_REMOVE_ALL
        };
        e.call(at, &args![list]);
        assert_eq!(e.mem.u32(list + 4), 0, "the list ends empty");
        let kept = target.map_or(0, |t| e.mem.u32(data[t]));
        let deleted = deleted.borrow().clone();
        (deleted, data, kept)
    }

    /// The repair target as the first, middle and last entry: the game's deletes all three;
    /// fixed every other entry's data is deleted and the target's stays.
    #[test]
    fn emptying_a_list_behind_the_repair_menu_keeps_its_target() {
        for target in 0..3 {
            let (deleted, data, _) = empty_list(false, Some(target), false);
            let mut sorted = deleted.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, data);
            let (deleted, data, _) = empty_list(true, Some(target), false);
            let mut want = data.clone();
            want.remove(target);
            let mut sorted = deleted.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, want, "target {target}");
        }
    }

    /// With no repair menu open, emptying deletes every entry's data, fixed or not, and in the
    /// same order.
    #[test]
    fn emptying_a_list_with_no_repair_target_is_the_games() {
        let (game, _, _) = empty_list(false, None, false);
        let (fixed, _, _) = empty_list(true, None, false);
        assert_eq!(game.len(), 3);
        assert_eq!(game, fixed);
    }

    /// A list destroyed behind the open repair menu keeps the target too.
    #[test]
    fn destroying_a_list_behind_the_repair_menu_keeps_its_target() {
        let (deleted, data, _) = empty_list(true, Some(1), true);
        let mut sorted = deleted.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![data[0], data[2]]);
        let (deleted, _, _) = empty_list(false, Some(1), true);
        assert_eq!(deleted.len(), 3);
    }

    /// The companion's caster and removal slots (test vtables), what they were given.
    struct Drinker {
        e: Engine,
        companion: u32,
        potion: u32,
        casts: Rc<std::cell::RefCell<Vec<u32>>>,
        removals: Rc<std::cell::RefCell<Vec<u32>>>,
    }

    const CAST: u32 = 0x00c5_0040;
    const REMOVE: u32 = 0x00c5_117c;

    fn drinker(fixed: bool) -> Drinker {
        let mut e = engine(&WHEEL_STIMPAK, fixed);
        let companion = e.mem.alloc(0x200);
        let vtable = e.mem.alloc(0x200);
        e.mem.set_u32(companion, vtable);
        e.mem.set_u32(vtable + 0x17c, REMOVE);
        let caster_vtable = e.mem.alloc(0x100);
        e.mem.set_u32(companion + MAGIC_CASTER, caster_vtable);
        e.mem.set_u32(caster_vtable + CAST_SLOT, CAST);
        let player = e.mem.alloc(0x200);
        e.set_global(PLAYER, player);
        let potion = e.mem.alloc(0x40);
        let casts = Rc::new(std::cell::RefCell::new(Vec::new()));
        let removals = Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = casts.clone();
        e.register_double(CAST, move |_, a| {
            log.borrow_mut().push(a[1]);
            Ret::default()
        });
        let log = removals.clone();
        e.register_double(REMOVE, move |_, a| {
            log.borrow_mut().push(a[0]);
            Ret::default()
        });
        e.register(RELEASE_CAST, |_, _| Ret::default());
        Drinker {
            e,
            companion,
            potion,
            casts,
            removals,
        }
    }

    /// Inside the wheel's use the companion's drink casts the potion and takes none of the
    /// companion's; the game's (and any drink outside the wheel) casts and takes one.
    #[test]
    fn the_wheels_drink_takes_nothing_from_the_companion() {
        for fixed in [false, true] {
            let mut d = drinker(fixed);
            let (companion, potion) = (d.companion, d.potion);
            let drunk = {
                let _use = WheelStimpak::begin();
                d.e.call(DRINK_POTION, &args![companion, potion, 0u32, 1u32])
                    .u32()
            };
            assert_eq!(drunk, 1);
            assert_eq!(*d.casts.borrow(), vec![potion + POTION_EFFECTS]);
            assert_eq!(d.removals.borrow().len(), usize::from(!fixed));
            assert_eq!(calls_to(&d.e, RELEASE_CAST), 1);
            // Outside the wheel the drink is the game's.
            d.e.call(DRINK_POTION, &args![companion, potion, 0u32, 1u32]);
            assert_eq!(d.removals.borrow().len(), usize::from(!fixed) + 1);
        }
        assert!(!IN_WHEEL_STIMPAK.with(Cell::get));
    }

    /// No potion: no drink, inside the wheel or not.
    #[test]
    fn the_wheels_drink_of_no_potion_is_none() {
        let mut d = drinker(true);
        let companion = d.companion;
        let _use = WheelStimpak::begin();
        assert_eq!(
            d.e.call(DRINK_POTION, &args![companion, 0u32, 0u32, 1u32])
                .u32(),
            0
        );
        assert!(d.casts.borrow().is_empty() && d.removals.borrow().is_empty());
    }

    /// The wheel's use marks its own run and unmarks it when it ends, a stop included (a
    /// companion with no stimpak entry: the game's use reads the default object first).
    #[test]
    fn the_wheels_use_marks_only_its_own_run() {
        let mut e = engine(&WHEEL_STIMPAK, true);
        let seen = Rc::new(Cell::new(false));
        let log = seen.clone();
        e.register_double(0x0058_db10, move |_, _| {
            log.set(IN_WHEEL_STIMPAK.with(Cell::get));
            panic!("stop after the first call");
        });
        let menu = e.mem.alloc(0x100);
        let companion = e.mem.alloc(0x10);
        e.mem.set_u32(menu + 0x68, companion);
        e.set_global(PLAYER, companion + 4);
        assert!(stop(|| answered(e.call(DO_USE_STIM, &args![menu]))).is_some());
        assert!(seen.get());
        assert!(!IN_WHEEL_STIMPAK.with(Cell::get));
    }

    /// A list filled from one item whose name component is `name` (0: none), split or not:
    /// the texts the list was given.
    fn fill(fixed: bool, name: Option<&str>, count: i32, split: bool) -> Option<Vec<String>> {
        let mut e = engine(&NAMELESS_ITEM, fixed);
        let this = e.mem.alloc(0x30);
        let vtable = e.mem.alloc(0x40);
        e.mem.set_u32(this, vtable);
        e.mem.set_u32(vtable + 0x1c, 0x00c6_001c);
        e.mem.set_u32(this + 0xc, 1);
        e.mem.set_u32(this + 0x18, 1);
        let item = e.mem.alloc(0xc);
        e.mem.set_u32(item + 4, count as u32);
        let form = e.mem.alloc(0x10);
        e.mem.set_u32(item + 8, form);
        let full_name = match name {
            None => 0,
            Some(text) => {
                let chars = e.mem.alloc(0x40);
                for (i, b) in text.bytes().enumerate() {
                    e.mem.set_u8(chars + i as u32, b);
                }
                let full_name = e.mem.alloc(0xc);
                e.mem.set_u32(full_name + 4, chars);
                e.mem.set_u16(full_name + 8, text.len() as u16);
                full_name
            }
        };
        e.register_double(DYNAMIC_CAST, move |_, _| full_name.into_ret());
        for stub in [0x00c6_001c, LIST_SET, ITERATOR_DELETE, ITEM_SET_COUNT] {
            e.register(stub, |_, _| Ret::default());
        }
        e.register(LIST_STATE, |_, _| 0u32.into_ret());
        e.register(ITEM_HAS_MODS, |_, _| 0u32.into_ret());
        e.register(INVENTORY_ITERATOR, |e, a| {
            e.mem.set_u32(a[2], 1);
            e.mem.set_u32(a[3], 1);
            Ret::default()
        });
        let handed = Rc::new(Cell::new(false));
        let once = handed.clone();
        e.register_double(NEXT_ITEM, move |_, _| {
            if once.replace(true) {
                0u32.into_ret()
            } else {
                item.into_ret()
            }
        });
        e.register(ALLOCATE, |e, _| e.mem.alloc(0xc).into_ret());
        e.register(ITEM_CONSTRUCTOR, |_, a| a[0].into_ret());
        e.register(ITEM_COPY, |_, _| Ret::default());
        let texts = Rc::new(std::cell::RefCell::new(Vec::new()));
        let log = texts.clone();
        e.register_double(LIST_ADD_ITEM, move |e, a| {
            let mut text = String::new();
            let mut at = a[2];
            while e.mem.u8(at) != 0 {
                text.push(e.mem.u8(at) as char);
                at += 1;
            }
            log.borrow_mut().push(text);
            0u32.into_ret()
        });
        let args = args![this, 1u32, 0u32, 0u32, 0u32, u32::from(split), 0u32];
        let stopped = stop(|| answered(e.call(LIST_FILL, &args)));
        let out = texts.borrow().clone();
        stopped.is_none().then_some(out)
    }

    /// An item whose form has no name component: the game's fill stops reading the name at
    /// address 4; fixed it is listed blank, once per unit when split.
    #[test]
    fn a_nameless_item_is_listed_blank() {
        assert_eq!(fill(false, None, 1, false), None);
        assert_eq!(fill(true, None, 1, false), Some(vec![String::new()]));
        assert_eq!(fill(true, None, 3, true), Some(vec![String::new(); 3]));
    }

    /// A named item is listed by its name, fixed or not.
    #[test]
    fn a_named_item_is_listed_as_the_game_lists_it() {
        for split in [false, true] {
            let game = fill(false, Some("Stimpak"), 2, split);
            assert_eq!(fill(true, Some("Stimpak"), 2, split), game);
            let want = if split { 2 } else { 1 };
            assert_eq!(game, Some(vec!["Stimpak".to_string(); want]));
        }
    }
}
