//! Group GB's game patches (step 3.9: UI: item lists, repair, pad dead zones): its handlers, wraps and slot routines. Only group
//! GB's PR edits this file (and its own tests file beside it).
//!
//! **D04, the pad dead zones** (`DeferredInit`, `internal/patches_game.h` (JIP LN NVSE), only when
//! `HookXInput` succeeded: the port's XInput wrap, in whenever JIP answers "installed",
//! [`crate::input`]). JIP points the operand of eleven `FCOMP double ptr [0x010718F0]` (the exe's
//! dead zone 7849.0, after `FILD axis; FSTP; CALL 00408820` (fabs)) at its own doubles
//! `s_deadZoneLSg` (nine: the caravan, lockpick, map and roulette menus' left stick, the free
//! camera's move axes) and `s_deadZoneRSg` (two: the free camera's look axes), which start at
//! 7849.0 and 8689.0 (`internal/xinput.cpp`; `SetDeadZoneLS/RS`, not built, change them). Each
//! seam answers the guest address of the double to compare with; the doubles live in the
//! built-ins' data, made as JIP installs from [`crate::input::State`]'s dead zones.
//!
//! All eleven are JIP's own (`INSTALLED`, the lead's call): JIP points them whenever its XInput hook
//! is in, whatever the game-patch row says. With JIP's 7849 the left-stick seams answer as the
//! exe; the right-stick seams change the free camera's look for every pad player with JIP
//! installed (8689 where the exe uses 7849). The free camera's four seams sit in group GP's
//! `playercharacter.rs` (GP adds them); the rows and these handlers are GB's.
//!
//! JIP's `SetDeadZoneLS/RS` (not built) write `s_deadZoneLSg`/`s_deadZoneRSg` with the new
//! integer and 19 immediates in the exe each: when they are built they must write the two doubles
//! these seams answer ([`doubles`]: `e.mem.set_f64`) as well as the input state (the dead-zone
//! atomics of [`crate::input::State`], the lock-free slot the gamepad conditions read) and the
//! immediates.
//!
//! **G87, the map menu's skip of its centring** (`InitGamePatches`: `WriteRelJump(0x79DF73,
//! RefreshMapMenuHook)`, JIP's own, `INSTALLED`). In `MapMenu::AddMapIcons` (`0079dbb0`), where
//! the map tile is centred on the player icon (traits `0xFA1`/`0xFA2`), JIP's hook tests its flag
//! `s_mapMenuSkipSetXY`: set, it clears it and jumps past both sets (`0x79E053`); clear, it does
//! the exe's work. Only JIP's item-list refresh (`RefreshItemListBox`, for `RefreshItemsList` and
//! `RemoveMe`'s G60, neither built yet) sets the flag, on the world map tab, just before it calls
//! `AddMapIcons`; [`skip_map_centring_once`] is that set.
//!
//! **G88, JIP's fade types 5 and 6** (`InitGamePatches`: `SafeWrite8(0xA081A8, 6)` and
//! `ProcessGradualSetFloatHook` in the two jump-table words after `Tile::UpdateFadeControls`'
//! table, JIP's own, `INSTALLED`). The exe passes types above 4 over; JIP's arm moves the value
//! from the start to the end over the duration and, at the end, sets the start value again: type
//! 5 starts over (forever), type 6 ends (its control is listed as finished). Only JIP's
//! `SetUIFloatGradual` (built: modes 3 and 4) makes controls of these types.
//!
//! **GB-3, fixes of `InitGamePatches`** (in with the game-patch row `NV_JIP_GAME_PATCHES`): the
//! recipe menu shows an item's plain value and a weight below 1 with two decimals (G13), lists
//! ammunition's effects (G68) and tests a crafted weapon's type itself (G100); the quantity menu
//! takes the mouse wheel (G69); the barter menu titles the vendor's tab as it opens (G56) and,
//! before taking a vendor's caps, splits a count over 32,767 into stacks (G64).
//!
//! Threads: every handler here runs where its menu code runs, on the main thread (the free
//! camera's dead-zone seams: the main thread's player update). None takes JIP's shared slot: the
//! dead zones and G87's flag are lock-free `ext_free` words made at JIP's install, G88 reads and
//! writes only the fade control and its tile in guest memory, GB-3's handlers only guest memory
//! and the exe's functions.

use super::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// The exe's dead zone double (7849.0), every patched `FCOMP`'s operand.
pub const EXE_DEAD_ZONE: u32 = 0x0107_18f0;

/// The left-stick seams (JIP's `SafeWrite32` addresses, the `FCOMP`s' operands):
/// `CaravanMenu::DoIdle` x and y, `LockPickMenu::RotateLockpick`, `MapMenu::DoGamepad`'s zoom,
/// `RouletteMenu::DoIdle`'s three, the free camera's move x and y (GP's file).
pub const LEFT_STICK: [SiteSlot; 9] = [
    SiteSlot::new(Owner::Jip, 0x0074_16d6),
    SiteSlot::new(Owner::Jip, 0x0074_1705),
    SiteSlot::new(Owner::Jip, 0x0079_0966),
    SiteSlot::new(Owner::Jip, 0x0079_9861),
    SiteSlot::new(Owner::Jip, 0x007b_d47d),
    SiteSlot::new(Owner::Jip, 0x007b_d4a0),
    SiteSlot::new(Owner::Jip, 0x007b_d4dd),
    SiteSlot::new(Owner::Jip, 0x0094_ab59),
    SiteSlot::new(Owner::Jip, 0x0094_ab88),
];
/// The right-stick seams: the free camera's look x and y (GP's file).
pub const RIGHT_STICK: [SiteSlot; 2] = [
    SiteSlot::new(Owner::Jip, 0x0094_a93c),
    SiteSlot::new(Owner::Jip, 0x0094_a95f),
];

const fn addrs<const N: usize>(slots: [SiteSlot; N]) -> [u32; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = slots[i].addr();
        i += 1;
    }
    out
}
const LEFT_ADDRS: [u32; 9] = addrs(LEFT_STICK);
const RIGHT_ADDRS: [u32; 2] = addrs(RIGHT_STICK);
/// All eleven, left then right.
const ALL_ADDRS: [u32; 11] = {
    let mut out = [0; 11];
    let mut i = 0;
    while i < 9 {
        out[i] = LEFT_ADDRS[i];
        i += 1;
    }
    out[9] = RIGHT_ADDRS[0];
    out[10] = RIGHT_ADDRS[1];
    out
};

/// G87's seam: `MapMenu::AddMapIcons` before the map tile is centred (JIP's jump at 0x79DF73).
pub const MAP_SKIP_CENTRING: SiteSlot = SiteSlot::new(Owner::Jip, 0x0079_df73);
/// G88's seams: `Tile::UpdateFadeControls`' jump-table words for fade types 5 and 6.
pub const FADE_TYPES: [SiteSlot; 2] = [
    SiteSlot::new(Owner::Jip, 0x00a0_8718),
    SiteSlot::new(Owner::Jip, 0x00a0_871c),
];

/// The group's JIP-own seams: D04's eleven, then G87's and G88's.
const JIP_SITES: [u32; 14] = {
    let mut out = [0; 14];
    let mut i = 0;
    while i < 11 {
        out[i] = ALL_ADDRS[i];
        i += 1;
    }
    out[11] = MAP_SKIP_CENTRING.addr();
    out[12] = FADE_TYPES[0].addr();
    out[13] = FADE_TYPES[1].addr();
    out
};

/// G100's seam: `fn_007284f0` (the recipe menu's crafting), its `CALL 0x4C0BF0` on a crafted
/// weapon (words: the weapon).
pub const DISPOSABLE_WEAPON: SiteSlot = SiteSlot::new(Owner::Jip, 0x0072_880d);
/// G13's two seams in `fn_00728da0` (the recipe menu's item information): the value's
/// `FSTP; CALL 0x647C00` (no words) and the `"%.1f"` operand of the weight's format (no words).
pub const PLAIN_VALUE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0072_8e10);
pub const WEIGHT_FORMAT: SiteSlot = SiteSlot::new(Owner::Jip, 0x0072_9616);
/// G68's seam: `fn_00728da0`'s effects-list test (words: the form).
pub const AMMO_EFFECTS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0072_94dc);
/// G56's seam: `BarterMenu::Create`'s first list fill (words: the menu).
pub const BARTER_TAB_TITLE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0072_d661);
/// G64's seam: `BarterMenu::CompleteTransaction`'s `RemoveItemBarterContainer` of the vendor's
/// caps (words: the data handler, the buyer's inventory changes data).
pub const VENDOR_CAPS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0073_005c);
/// G69's seam: `QuantityMenu::DoIdle`'s start (words: the menu).
pub const QUANTITY_WHEEL: SiteSlot = SiteSlot::new(Owner::Jip, 0x007a_c179);
<<<<<<< HEAD
/// G82's seams: the barter, container and inventory menus' `InitFunc` at their count test (words:
/// the tile, the stack, its count). Answered: the name is set and the exe's block is skipped.
pub const ITEM_NAMES: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Jip, 0x0072_f33d),
    SiteSlot::new(Owner::Jip, 0x0075_d1a5),
    SiteSlot::new(Owner::Jip, 0x0078_2876),
];
/// O9's seams (`bQttSelectShortKeys`): the barter, container and inventory menus' quantity test
/// (words: the stack's count; answered: the count to take without the quantity menu), and the
/// container's weightless-item test (`0x75BF18`, no words; answered: skipped).
pub const QUANTITY_KEYS: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Jip, 0x0072_d8b4),
    SiteSlot::new(Owner::Jip, 0x0075_bf97),
    SiteSlot::new(Owner::Jip, 0x0078_0b09),
];
pub const WEIGHTLESS_TAKE: SiteSlot = SiteSlot::new(Owner::Jip, 0x0075_bf18);
=======
>>>>>>> origin/main

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GB",
    fix_sites: &[
        DISPOSABLE_WEAPON.addr(),
        PLAIN_VALUE.addr(),
        AMMO_EFFECTS.addr(),
        WEIGHT_FORMAT.addr(),
        BARTER_TAB_TITLE.addr(),
        VENDOR_CAPS.addr(),
        QUANTITY_WHEEL.addr(),
<<<<<<< HEAD
        ITEM_NAMES[0].addr(),
        ITEM_NAMES[1].addr(),
        ITEM_NAMES[2].addr(),
        QUANTITY_KEYS[0].addr(),
        QUANTITY_KEYS[1].addr(),
        QUANTITY_KEYS[2].addr(),
        WEIGHTLESS_TAKE.addr(),
=======
>>>>>>> origin/main
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &JIP_SITES,
    jip_wraps: &[],
    install_jip,
};

fn install_fixes(e: &mut Engine) {
<<<<<<< HEAD
    for slot in ITEM_NAMES {
        e.on_site(slot, item_entry_name);
    }
    for slot in QUANTITY_KEYS {
        e.on_site(slot, quantity_keys);
    }
    e.on_site(WEIGHTLESS_TAKE, weightless_take);
=======
>>>>>>> origin/main
    e.on_site(DISPOSABLE_WEAPON, disposable_weapon);
    e.on_site(PLAIN_VALUE, plain_value);
    e.on_site(WEIGHT_FORMAT, weight_format);
    e.on_site(AMMO_EFFECTS, ammo_effects);
    e.on_site(BARTER_TAB_TITLE, barter_tab_title);
    e.on_site(VENDOR_CAPS, fix_vendor_caps);
    e.on_site(QUANTITY_WHEEL, quantity_wheel);
}

/// A weapon's type byte (`TESObjectWEAP::eWeaponType`, +0xF4).
const WEAPON_TYPE: u32 = 0xf4;
/// A form's type byte (+4) and `kFormType_TESAmmo`.
const FORM_TYPE: u32 = 4;
const FORM_AMMO: u8 = 0x29;
/// The exe's `"%.2f"` (`0x1030018`, read in Ghidra: `25 2e 32 66 00`).
pub const TWO_DECIMALS: u32 = 0x0103_0018;
/// `BarterMenu::SetFilterModeTitle` (`007301f0`, `thiscall`, `RET 4`: the tab tile) and the tile
/// JIP passes, the menu's word at +0x48 (`pTiles[8]`).
const SET_FILTER_MODE_TITLE: u32 = 0x0073_01f0;
const BARTER_TAB_TILE: u32 = 0x48;
/// `g_interfaceManager` and its mouse wheel word (+0x44, signed: up above 0).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const MOUSE_WHEEL: u32 = 0x44;
/// A menu's vtable slot +0xC (`Menu::HandleClick(id, tile)`).
const MENU_HANDLE_CLICK: u32 = 0xc;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: IsDisposableWeaponHook.
/// G100: the weapon's type byte, signed, above 9 (JIP's `setg`). The exe's `004c0bf0` answers
/// types 10 to 13; the game has no weapon type above 13, so every weapon answers the same.
fn disposable_weapon(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let weapon = *words.first()?;
    Some(u32::from(e.mem.u8(weapon + WEAPON_TYPE) as i8 > 9))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x728E10, 8-byte NOP)).
/// G13: the value's call is left out (1: the form's plain value is shown).
fn plain_value(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite32(0x729616, 0x1030018)).
/// G13: the weight's format below 1, `"%.2f"`.
fn weight_format(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(TWO_DECIMALS)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RecipeMenuAmmoEffectsHook.
/// G68: 1 for ammunition (its effects text is built from the ammo, `00503a70`, whatever the
/// exe's effect list holds); anything else answers nothing (the exe's test).
fn ammo_effects(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let form = *words.first()?;
    (e.mem.u8(form + FORM_TYPE) == FORM_AMMO).then_some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitBarterMenuHook.
/// G56: `SetFilterModeTitle(pTiles[8])` before the first list fill; the translation goes on.
fn barter_tab_title(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let menu = *words.first()?;
    let tile = e.mem.u32(menu + BARTER_TAB_TILE);
    e.call(SET_FILTER_MODE_TITLE, &args![menu, tile]);
    None
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: QttMenuEnableWheelHook.
/// G69: with the mouse wheel moved, the menu's click on button 2 (wheel up) or 1 (down), as a
/// click on the plus or minus button; the translation goes on.
fn quantity_wheel(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let menu = *words.first()?;
    let manager = e.mem.u32(INTERFACE_MANAGER);
    let wheel = e.mem.u32(manager + MOUSE_WHEEL) as i32;
    if wheel != 0 {
        let button = if wheel > 0 { 2u32 } else { 1 };
        e.vcall(menu, MENU_HANDLE_CLICK, &args![button, 0u32]);
    }
    None
}

/// `kXData_ExtraOriginalReference` (0x20, its reference at +0xC) and `kXData_ExtraCount` (0x24, its
/// count at +0xC), with the vtables JIP's `Create` stores (`internal/class_vtbls.h`), and
/// `kVtbl_ExtraDataList`.
const EXTRA_ORIGINAL_REFERENCE: u32 = 0x20;
const EXTRA_COUNT: u32 = 0x24;
const VTBL_ORIGINAL_REFERENCE: u32 = 0x0101_5bc4;
const VTBL_COUNT: u32 = 0x0101_58d8;
const VTBL_EXTRA_DATA_LIST: u32 = 0x0101_43e8;
const EXTRA_VALUE: u32 = 0xc;
/// `DataHandler`'s word at +0x628 (the barter container's inventory changes data, as
/// `RemoveItemBarterContainer` reads it), the caps' form id (`g_capsItem`), a stack's limit.
const BARTER_CHANGES: u32 = 0x628;
const CAPS_ID: u32 = 0xf;
const STACK_MAX: i32 = 0x7fff;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameExtraData.cpp:
// ContChangesEntryList::FindForItem.
/// The first entry of the list (from its head node) whose item (+8) is `item`, 0 for none.
fn find_for_item(e: &Engine, mut node: u32, item: u32) -> u32 {
    while node != 0 {
        let entry = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if entry != 0 && e.mem.u32(entry + 8) == item {
            return entry;
        }
    }
    0
}

/// JIP's `CreateExtraType` with the word at +0xC (`ExtraOriginalReference::Create`,
/// `ExtraCount::Create` below `SHRT_MAX`).
fn extra_with(e: &mut Engine, vtable: u32, kind: u32, value: u32) -> u32 {
    let extra = crate::commands::extra::create(e, vtable, kind);
    e.mem.set_u32(extra + EXTRA_VALUE, value);
    extra
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: BarterSellFixHook,
// FixVendorCaps; nvse/GameBSExtraData.cpp: ExtraDataList::Create; nvse/GameTypes.h:
// tList::Prepend.
/// G64, before the vendor's caps are taken (words: the data handler, the buyer's inventory changes
/// data): when the barter container holds caps with a stack list and the buyer holds more than
/// 32,767 caps, the first stack of the barter container marked as the buyer's (its
/// `ExtraOriginalReference`) gets at most 32,767 of them in its count extra, and new stacks of the
/// buyer's caps (an original reference and a count of at most 32,767 each) are put at the head of
/// the list for the rest while more than one is left. Answers nothing (the exe's call follows).
///
/// nv-rs's own: a data handler without the barter container's data does nothing (JIP would read
/// through it).
fn fix_vendor_caps(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let (&handler, &changes) = (words.first()?, words.get(1)?);
    let barter = e.mem.u32(handler + BARTER_CHANGES);
    if barter == 0 {
        return None;
    }
    let caps = crate::cosave::lookup_form(e, CAPS_ID);
    let entry = find_for_item(e, e.mem.u32(barter), caps);
    if entry == 0 {
        return None;
    }
    let stacks = e.mem.u32(entry);
    if stacks == 0 {
        return None;
    }
    let owner = e.mem.u32(changes + 4);
    let held = find_for_item(e, e.mem.u32(changes), caps);
    if held == 0 {
        return None;
    }
    let mut left = e.mem.u32(held + 4) as i32;
    if left <= STACK_MAX {
        return None;
    }
    let mut node = stacks;
    while node != 0 {
        let list = e.mem.u32(node);
        node = e.mem.u32(node + 4);
        if list == 0 {
            continue;
        }
        let original = crate::commands::extra::get_by_type(e, list, EXTRA_ORIGINAL_REFERENCE);
        if original == 0 || e.mem.u32(original + EXTRA_VALUE) != owner {
            continue;
        }
        let count = crate::commands::extra::get_by_type(e, list, EXTRA_COUNT);
        if count != 0 {
            let part = left.min(STACK_MAX);
            e.mem.set_u32(count + EXTRA_VALUE, part as u32);
            left -= part;
        }
        break;
    }
    while left > 1 {
        let list = crate::commands::heap_alloc(e, 0x20);
        for at in (0..0x20).step_by(4) {
            e.mem.set_u32(list + at, 0);
        }
        e.mem.set_u32(list, VTBL_EXTRA_DATA_LIST);
        let original = extra_with(e, VTBL_ORIGINAL_REFERENCE, EXTRA_ORIGINAL_REFERENCE, owner);
        crate::commands::extra::add_extra(e, list, original);
        let part = left.min(STACK_MAX);
        left -= part;
        let count = extra_with(e, VTBL_COUNT, EXTRA_COUNT, part as u32);
        crate::commands::extra::add_extra(e, list, count);
        // `tList::Prepend`: an empty head takes the list; else a new node after the head takes
        // the head's item and the head the list.
        let head = e.mem.u32(stacks);
        if head == 0 {
            e.mem.set_u32(stacks, list);
        } else {
            let moved = crate::commands::heap_alloc(e, 8);
            e.mem.set_u32(moved, head);
            e.mem.set_u32(moved + 4, e.mem.u32(stacks + 4));
            e.mem.set_u32(stacks, list);
            e.mem.set_u32(stacks + 4, moved);
        }
    }
    None
}

/// Where JIP's two doubles are (0: not made yet). Lock-free: the seams are asked from any game
/// thread; written once, before the threads start.
#[derive(Default)]
pub struct DeadZones {
    left: AtomicU32,
    right: AtomicU32,
}

/// JIP's `s_deadZoneLSg` and `s_deadZoneRSg`: 8 bytes each in the built-ins' data, made once with
/// the input state's dead zones (7849 and 8689 until `SetDeadZoneLS/RS` are built).
pub fn doubles(e: &mut Engine) -> (u32, u32) {
    let zones = e.ext_free::<DeadZones>();
    let left = zones.left.load(Ordering::Relaxed);
    if left != 0 {
        return (left, zones.right.load(Ordering::Relaxed));
    }
    let at = nvse::table::alloc(e, 16);
    let (ls, rs) = (crate::input::dead_zone_ls(e), crate::input::dead_zone_rs(e));
    e.mem.set_f64(at, f64::from(ls));
    e.mem.set_f64(at + 8, f64::from(rs));
    let zones = e.ext_free::<DeadZones>();
    zones.left.store(at, Ordering::Relaxed);
    zones.right.store(at + 8, Ordering::Relaxed);
    (at, at + 8)
}

fn install_jip(e: &mut Engine) {
    doubles(e);
    e.ext_free::<MapSkipCentring>();
    for slot in LEFT_STICK {
        e.on_site(slot, left_stick);
    }
    for slot in RIGHT_STICK {
        e.on_site(slot, right_stick);
    }
    e.on_site(MAP_SKIP_CENTRING, map_skip_centring);
    for slot in FADE_TYPES {
        e.on_site(slot, gradual_set_float);
    }
}

/// JIP's `s_mapMenuSkipSetXY` (G87): lock-free, made at JIP's install.
#[derive(Default)]
pub struct MapSkipCentring(AtomicBool);

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: RefreshItemListBox (its
// `s_mapMenuSkipSetXY = true` before `AddMapIcons` on the world map tab).
/// The next `MapMenu::AddMapIcons` leaves the map tile where it is (once). For JIP's item-list
/// refresh (`RefreshItemsList`, G60), not built yet.
pub fn skip_map_centring_once(e: &mut Engine) {
    e.ext_free::<MapSkipCentring>()
        .0
        .store(true, Ordering::Relaxed);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RefreshMapMenuHook.
/// G87: 1 (skip the centring) when JIP's flag was set, which clears it; else 0 (the exe's path).
fn map_skip_centring(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(u32::from(
        e.ext_free::<MapSkipCentring>()
            .0
            .swap(false, Ordering::Relaxed),
    ))
}

/// `GetTickCount` through the exe's import slot (JIP calls the same system function through
/// its own import).
const TICK_COUNT_IMPORT: u32 = 0x00fd_f060;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h:
// ProcessGradualSetFloatHook, nvse/GameTiles.cpp: Tile::SetFloat.
/// G88, a fade control of type 5 or 6 (words: the control, its type). The progress is the ticks
/// since the start (a signed int, `cvtsi2ss`) over the duration, in floats; below 1 (or a NaN)
/// the value is `progress * (end - start) + start`; at 1 or above the start time becomes now
/// and the value the start value, and a type other than 5 answers 1 (listed as finished). The
/// value goes to the tile's trait as JIP's `Tile::SetFloat` does (nothing for a trait the tile
/// does not have, an equal value or a NaN). Answers 0 otherwise.
///
/// nv-rs's own: a control without a tile sets nothing (JIP would read through it).
fn gradual_set_float(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let (&control, &kind) = (words.first()?, words.get(1)?);
    let now = e.call(TICK_COUNT_IMPORT, &[]).u32();
    let elapsed = now.wrapping_sub(e.mem.u32(control + 8)) as i32 as f32;
    let progress = elapsed / e.mem.f32(control + 0xc);
    let start = e.mem.f32(control);
    let (value, finished) = if progress >= 1.0 {
        e.mem.set_u32(control + 8, now);
        (start, kind != 5)
    } else {
        (progress * (e.mem.f32(control + 4) - start) + start, false)
    };
    let (id, tile) = (e.mem.u32(control + 0x10), e.mem.u32(control + 0x14));
    if tile != 0 {
        let at = crate::commands::jip_fn_ui::value_of(e, tile, id);
        if at != 0 {
            crate::commands::jip_fn_ui::value_set_float(e, at, value);
        }
    }
    Some(u32::from(finished))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit
// (SafeWrite32 of &s_deadZoneLSg at the nine left-stick operands).
/// A left-stick `FCOMP`: compares with JIP's `s_deadZoneLSg`.
fn left_stick(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(e.ext_free::<DeadZones>().left.load(Ordering::Relaxed))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit
// (SafeWrite32 of &s_deadZoneRSg at the two right-stick operands).
/// A right-stick `FCOMP`: compares with JIP's `s_deadZoneRSg`.
fn right_stick(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(e.ext_free::<DeadZones>().right.load(Ordering::Relaxed))
}

/// `kTileValue_string`, a form's full name (`TESFullName::name` of the stack's form: text +0x34,
/// length +0x38, as JIP reads it), the `InitFunc`s' 0x80-byte name buffer.
const TRAIT_STRING: u32 = 0xfc4;
const FULL_NAME_TEXT: u32 = 0x34;
const FULL_NAME_LENGTH: u32 = 0x38;
const NAME_BUFFER: usize = 0x80;
/// A tile value's string (+0xC) and `Tile::Value::Refresh` (`00a09410`, `thiscall`, a flag).
const VALUE_STRING: u32 = 0xc;
const VALUE_REFRESH: u32 = 0x00a0_9410;
/// The keys JIP reads in xNVSE's key state (`g_DIHookCtrl`'s raw states at +0xCF, +0x44F,
/// +0x12A, +0x17E: `KeyInfo` is 7 bytes from +4): left and right Ctrl (one), left and right Shift
/// (all).
const KEYS_ONE: [u32; 2] = [0x1d, 0x9d];
const KEYS_ALL: [u32; 2] = [0x2a, 0x36];
/// The raw-state bit of [`nvse::input::key_states`].
const RAW_STATE: u8 = 2;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameTiles.cpp: Tile::SetString,
// Tile::Value::SetString; internal/utility.cpp: CopyCString.
/// The tile's value of `id` (none: nothing) gets `text`: nothing when it holds the same text
/// (case counts); else its old text is freed, a copy on the game's heap is kept and the value
/// refreshed.
fn tile_set_string(e: &mut Engine, tile: u32, id: u32, text: &[u8]) {
    let value = crate::commands::jip_fn_ui::value_of(e, tile, id);
    if value == 0 {
        return;
    }
    let old = e.mem.u32(value + VALUE_STRING);
    if old != 0 {
        if e.mem.cstr(old) == text {
            return;
        }
        crate::commands::heap_free(e, old);
        e.mem.set_u32(value + VALUE_STRING, 0);
    }
    let copy = crate::commands::heap_alloc(e, text.len() as u32 + 1);
    e.mem.write(copy, text);
    e.mem.set_u8(copy + text.len() as u32, 0);
    e.mem.set_u32(value + VALUE_STRING, copy);
    e.call(VALUE_REFRESH, &args![value, 1u32]);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: ConstructItemEntryNameHook.
/// G82 (words: the tile, the stack, its count): a stack whose weapon has mods (JIP's
/// `GetEntryDataModFlagsHook`, [`super::gh1::mod_flags`]) or of more than one gets the form's
/// name, "+" when modded and " (count)" when more than one, as the tile's string; nothing for
/// another stack or a form with an empty name. Answers 1 (the exe's block is skipped).
///
/// nv-rs's own: the text is cut to the 0x80-byte buffer's 0x7F characters (JIP copies the whole
/// name into it).
fn item_entry_name(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let (&tile, &item, &count) = (words.first()?, words.get(1)?, words.get(2)?);
    let count = count as i32;
    let plus = super::gh1::mod_flags(e, item) != 0;
    if !plus && count <= 1 {
        return Some(1);
    }
    let form = e.mem.u32(item + 8);
    let length = u32::from(e.mem.u16(form + FULL_NAME_LENGTH));
    if length == 0 {
        return Some(1);
    }
    let mut text = e.mem.bytes(e.mem.u32(form + FULL_NAME_TEXT), length);
    if plus {
        text.push(b'+');
    }
    if count > 1 {
        text.extend_from_slice(b" (");
        text.extend_from_slice(&crate::utility::int_to_str(count));
        text.push(b')');
    }
    text.truncate(NAME_BUFFER - 1);
    tile_set_string(e, tile, TRAIT_STRING, &text);
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: QttSelectInventoryHook,
// QttSelectContainerHook, QttSelectBarterHook.
/// O9 (words: the stack's count): Ctrl held answers 1, else Shift held answers the count, taken
/// without the quantity menu; neither: nothing (the exe's test). Ctrl first, as JIP.
fn quantity_keys(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let count = *words.first()?;
    let held = |e: &mut Engine, keys: [u32; 2]| {
        keys.iter()
            .any(|&k| nvse::input::key_states(e, k) & RAW_STATE != 0)
    };
    if held(e, KEYS_ONE) {
        Some(1)
    } else if held(e, KEYS_ALL) {
        Some(count)
    } else {
        None
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches (the O9
// list's `{0x75BF18, 0x63EB, 2, 0}`, `JMP 0x75BF7D`).
/// O9: the container's weightless-item test is skipped (1).
fn weightless_take(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

#[cfg(test)]
#[path = "gb_tests.rs"]
mod tests;
