//! The stats menu's unit (`fallout/interface/menus/statsmenu`, Xbox PDB), its first part: the
//! status data list's destructor and clean-ups, the fade object (made on first use, its groups,
//! their tiles and fade bytes, the fade callback), the idle, the pages, the actor values, perks,
//! healing options and mode, the SPECIAL and skill rows, the face image and the radiation meter, the
//! sorts and the availability callbacks, the status data list's `Register` and look-up, the list
//! walk with a callback, and the `AddItem`s of the perk rank and misc stat list boxes. The
//! translations are engine-fallout-interface's `statsmenu.rs`, which these replace at the same
//! addresses. (`007df1f0`, the float at `+0x20`, is the process family's.)
//!
//! **Native.** The exe's order of work, with its calls by address of functions no family types (the
//! strings' format, destructor and assignment, the tile's menu and template rendering, the fade
//! controls, the list helpers, the inventory and its counts, the actor value helpers, the player's
//! level and name, the Pip-Boy's knob and burst, the time, the menus' other functions of this unit
//! not yet typed). A virtual call (an actor value, the player's perk ranks and active effects, a
//! list box's `RemoveAll`) stays virtual: the object's class is not known here. What the exe calls
//! that is a plain read or store is done in place ([`IN_PLACE`] and the other files' lists): the
//! interface manager and its Pip-Boy, the Pip-Boy's query byte, a string's constructor and
//! characters, a setting's text and value, the hardcore test, a node's item and next, a list's
//! emptiness, the effect item's setting, key and flags, the archetype and actor value flag tables,
//! an active effect's kind and active byte, the smaller of two integers, the status entry's and the
//! list item's constructors, memset of the fade bytes and the six lists' vector constructor. The
//! tile's getters and setters are the dispatch family's, `_ftol2` the C runtime's, the karma's
//! alignment and title the forms family's, the larger float the drawing helpers'; the list boxes'
//! methods, `Interface::` wrappers and this file's functions are this family's (all through
//! `typed_call!`). `StatusDataList::Register` keeps JIP LN's G107 seam at the duration's call.
//!
//! **Floats.** The main thread's: each x87 step the `f64` result rounded once to the float (the
//! 24-bit step), the exe's double constants read where it reads them; a float becomes an integer
//! through `fistp_chop64` (a truncating `FISTP qword`) or `_ftol2`.

use super::interface_cpp::in_place;
use super::interface_cpp_p2::name_characters;
use super::menu_manager_p2::{
    self, first_node, ftol2, static_trait, tile_float, tile_set_float, tile_set_int, Of,
    ADD_USER_TRAIT, CONTAINER, FREE, INIT_TILE_POSITION, ITEM_DELETE, TEMPLATE_NAME,
    VALUE_ADD_ACTION, VALUE_CALCULATE,
};
use crate::{dispatch, draw_f, forms};
use engine_core::prelude::*;
use engine_core::typed_call;
use engine_core::x87::{fistp_chop64, x87_quiet};

/// What this file does in place besides the other files' lists: plain reads and stores the exe
/// calls (the differential tests leave these out of the translation's calls).
pub const IN_PLACE: &[u32] = &[
    0x0081_0fd0, // the word at +0x98 (an effect setting's archetype)
    0x0040_3c80, // the archetype flag table's bit
    0x0082_5c00, // the word at +0x14 (an effect item's setting)
    0x0040_3e60, // an effect setting's flag (its +0x58)
    0x0058_cba0, // (its body)
    0x0040_3ea0, // an effect item's key: its +0x10 when its archetype has bit 2, else -1
    0x0040_3ee0, // (the archetype test)
    0x0040_3c60, // (its bit 2)
    0x0040_3ba0, // the status entry's constructor: four zero words and -1
    0x0055_b980, // an active effect's kind (+0x2c)
    0x0083_3c20, // an active effect's active byte (+0x10)
];

/// The open stats menu (`StatsMenu::Create` stores it), the fade object made on first use, the time
/// the meters next update at, the hardcore state the idle last showed.
const OPEN_MENU: u32 = 0x011d_ace0;
const FADE_OBJECT: u32 = 0x011d_ace4;
const NEXT_UPDATE: u32 = 0x011d_ad74;
const HARDCORE_SHOWN: u32 = 0x011a_0ae1;
/// The player; its actor values (`+0xa4`, virtual: `+8` the value, `+0xc` as a float in `ST0`,
/// `+0x1c` the base value), its active effects (`+0x94`, virtual `+8` the list) and its perk ranks
/// (virtual `+0x4e8`).
const PLAYER: u32 = 0x011d_ea3c;
const ACTOR_VALUES: u32 = 0xa4;
const AV_VALUE: u32 = 0x08;
const AV_FLOAT: u32 = 0x0c;
const AV_BASE: u32 = 0x1c;
const ACTIVE_EFFECTS: u32 = 0x94;
const PERK_RANKS_SLOT: u32 = 0x4e8;
/// The player's hardcore word (`004d1360`: 1 in hardcore mode).
const HARDCORE: u32 = 0x7bc;

/// The stats menu's fields: the menu tile (`+4`), the healing options (four 0x10-byte records from
/// `+0x28`: the item form, its tile, the inventory entry, the availability callback), the 60
/// `pTiles` (`+0x90`), the page tiles, the lists (SPECIAL `+0x180`, skills `+0x1b0`, perks `+0x1e0`,
/// general `+0x210`), limb mode (`+0x2a0`) and the effects' once-a-frame byte (`+0x2a1`).
const MENU_TILE: u32 = 0x04;
const HEALING_OPTIONS: u32 = 0x28;
const TILES: u32 = 0x90;
const SKILL_PAGE_TILE: u32 = 0xa4;
const HEALING_TILE: u32 = 0xb4;
const FACE_TILE: u32 = 0xf0;
const ICON_TILE: u32 = 0xf4;
const PERK_TILE: u32 = 0x118;
const SPECIAL_LIST: u32 = 0x180;
const SKILL_LIST: u32 = 0x1b0;
const PERK_LIST: u32 = 0x1e0;
const GENERAL_LIST: u32 = 0x210;
const LIMB_MODE: u32 = 0x2a0;
const EFFECTS_UPDATED: u32 = 0x2a1;

/// The tile traits: `0xfa3` visible, `0xfa9` the fade's alpha, `0xfaa` id, `0xfac` the list index,
/// `0xfaf` enabled, `0xfb0` height, `0xfc4` string, `0xfcc` image, `0x1004` to `0x100c` the menu's
/// user traits.
const TRAIT_VISIBLE: i32 = 0xfa3;
const TRAIT_ALPHA: i32 = 0xfa9;
const TRAIT_ID: i32 = 0xfaa;
const TRAIT_INDEX: i32 = 0xfac;
const TRAIT_ENABLED: i32 = 0xfaf;
const TRAIT_HEIGHT: i32 = 0xfb0;
const TRAIT_STRING: i32 = 0xfc4;
const TRAIT_IMAGE: i32 = 0xfcc;
const USER_1: i32 = 0x1004;
const USER_2: i32 = 0x1005;
const USER_3: i32 = 0x1006;
const USER_4: i32 = 0x1007;
const USER_5: i32 = 0x1008;
const USER_LEVEL: i32 = 0x100c;

/// The strings' callees: `00406f60` (cdecl, the format), `004037d0` (the destructor) and `00438390`
/// (the assignment, `RET 4`). The constructor (`004037b0`) and the characters (`00559450`) are in
/// place.
const STRING_FORMAT: u32 = 0x0040_6f60;
const STRING_DESTROY: u32 = 0x0040_37d0;
const STRING_ASSIGN: u32 = 0x0043_8390;
/// The formats and texts: `"%d/%d"`, the face image path, the value of 0x14, the body part images
/// (above 0 and broken), `"_Value"`, `"(%d) %s"`, the player's name line, `"%s"`, `"%s (%d)"`,
/// `"_xbox_button"`, `"_enabled"`, the arrows up and down.
const VALUE_PAIR_FORMAT: u32 = 0x0107_3b14;
const FACE_FORMAT: u32 = 0x0107_3868;
const VALUE_14_FORMAT: u32 = 0x0107_74c4;
const BODY_PART_FORMAT: u32 = 0x0107_3850;
const BODY_PART_BROKEN_FORMAT: u32 = 0x0107_3830;
const VALUE_TEXT: u32 = 0x0106_ed18;
const HEALING_FORMAT: u32 = 0x0107_7548;
const NAME_FORMAT: u32 = 0x0107_7550;
const ONE_STRING_FORMAT: u32 = 0x0101_9f08;
const RANK_FORMAT: u32 = 0x0106_eb00;
const XBOX_BUTTON_TEXT: u32 = 0x0107_34d0;
const ENABLED_TEXT: u32 = 0x0107_0244;
const ARROW_UP: u32 = 0x0107_7560;
const ARROW_DOWN: u32 = 0x0107_755c;
/// The body part names by actor value (`011a0a80 + 4 * value`).
const BODY_PART_NAMES: u32 = 0x011a_0a80;

/// The game settings: the level label, the healing mode's texts (limb mode and not), the value of
/// 0x14's, the "max level" text, the Pip-Boy burst's two; the level cap's owner (`0043d4d0` on it:
/// its `+4`).
const LEVEL_SETTING: u32 = 0x011d_2028;
const LIMB_MODE_SETTING: u32 = 0x011d_4708;
const HEALING_MODE_SETTING: u32 = 0x011d_2658;
const VALUE_14_SETTING: u32 = 0x011d_0ad8;
const MAX_LEVEL_SETTING: u32 = 0x011d_472c;
const BURST_SETTING_A: u32 = 0x011d_b37c;
const BURST_SETTING_B: u32 = 0x011d_b458;
const LEVEL_CAP: u32 = 0x011d_0c64;
/// `UpdateHealingMode`'s function-local statics: the guard word, the `"_xbox_button"` (bit 1) and
/// `"_enabled"` (bit 2) ids.
const HEALING_GUARD: u32 = 0x011d_ad80;
const XBOX_BUTTON_TRAIT: u32 = 0x011d_ad7c;
const ENABLED_TRAIT: u32 = 0x011d_ad78;

/// The exe's constants: the doubles 1000.0, 2.0, 0.0, 1.0, pi, pi / 2, 5.0 and 100.0, the floats the
/// fade adds to, the Pip-Boy's knob rate and burst scale, and `007ddef0`'s.
const THOUSAND: u32 = 0x0101_7b70;
const TWO: u32 = 0x0101_1590;
const ZERO: u32 = 0x0101_2060;
const ONE: u32 = 0x0101_2070;
const PI: u32 = 0x0101_ff40;
const HALF_PI: u32 = 0x0106_4f50;
const FIVE: u32 = 0x0102_0998;
const HUNDRED: u32 = 0x0101_7a40;
const FADE_TARGET: u32 = 0x0102_3cd8;
const KNOB_RATE: u32 = 0x0107_80a4;
const BURST_SCALE: u32 = 0x0101_6248;
const RETURNED_FLOAT: u32 = 0x0101_3974;

/// The allocator (cdecl: size).
const ALLOCATE: u32 = 0x0040_1000;
/// The lists' callees: the append of the pointed-to word (`005ae3d0`, `RET 4`), the first node's
/// removal (`0063f7b0`), the nodes' free (`00470470`), the destructor (`0046ffb0`).
const LIST_APPEND: u32 = 0x005a_e3d0;
const LIST_REMOVE_FIRST: u32 = 0x0063_f7b0;
const LIST_FREE_NODES: u32 = 0x0047_0470;
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
/// The `StatusDataList` vtable.
const STATUS_DATA_LIST_VTABLE: u32 = 0x0101_2650;
/// The fade controls: add (`00a07c60`, thiscall: trait, target, from, over, loop) and remove.
const ADD_FADE_CONTROL: u32 = 0x00a0_7c60;
const REMOVE_FADE_CONTROL: u32 = 0x00a0_7dc0;
/// `FadeControlCallback`, the callback the fade object's groups are walked with.
const FADE_CONTROL_CALLBACK: u32 = 0x007d_c040;

/// The idle's callees: `InterfaceManager::IsInPipboyMenu`, the time (milliseconds), the hardcore
/// meters, the skill page reset, limb mode's toggle.
const IS_IN_PIPBOY_MENU: u32 = 0x0071_78a0;
const TIME: u32 = 0x0045_7fe0;
const HARDCORE_METERS: [u32; 3] = [0x007d_dc20, 0x007d_dd10, 0x007d_de00];
const SET_SKILL_PAGE: u32 = 0x007e_0060;
const TOGGLE_LIMB_MODE: u32 = 0x007e_0230;
/// The item list callback walk (`007314c0`, `RET 0xc`) and the two row updates it is given.
const LIST_WALK: u32 = 0x0073_14c0;
const SPECIAL_UPDATE_FN: u32 = 0x007d_fa30;
const SKILLS_UPDATE_FN: u32 = 0x007d_fb00;
/// `ListBox<..>::GetTileFor` (`007a22d0`, `RET 4`: the address of the data).
const GET_TILE_FOR: u32 = 0x007a_22d0;
/// The actor value helpers: `GetClampedActorValue` (`RET 4`), `AVDeriveSkill` (cdecl, `ST0`), the
/// rounding (`00406d90`, cdecl: a float), a misc stat's value (`005a3370`).
const GET_CLAMPED_ACTOR_VALUE: u32 = 0x0066_ef20;
const AV_DERIVE_SKILL: u32 = 0x0064_3c90;
const ROUND: u32 = 0x0040_6d90;
const MISC_STAT_VALUE: u32 = 0x005a_3370;
/// The player's level (`0087f9f0`, `AX`) and name (`0055d520`); `GamePlayFormulas::
/// GetRequiredExperiencePoints` (cdecl).
const PLAYER_LEVEL: u32 = 0x0087_f9f0;
const PLAYER_NAME: u32 = 0x0055_d520;
const REQUIRED_EXPERIENCE: u32 = 0x0064_8b50;
/// The inventory: `GetInventoryItem` (`RET 8`), `GetItemCountinContainer` (`RET 4`).
const GET_INVENTORY_ITEM: u32 = 0x0057_6260;
const GET_ITEM_COUNT: u32 = 0x0057_5610;
/// `004b71d0` (no arguments, `AL`: a byte of the manager's at `011d8a84` and a test of its
/// object at `011d8c4c`).
const MANAGER_STATE_TEST: u32 = 0x004b_71d0;
/// The radiation meter's callees: `005c5420` (`ST0`), `004bd510` (cdecl, two floats, `ST0`: the
/// first rounded up to a whole multiple of the second), the Pip-Boy's knob (`007f86f0`, `RET 0xc`).
const METER_SCALE: u32 = 0x005c_5420;
const ROUND_UP_TO_MULTIPLE: u32 = 0x004b_d510;
const PIPBOY_KNOB: u32 = 0x007f_86f0;
/// The page's callees: `FORenderedMenu::TriggerBurst` (`RET 0xc`), `SetKnobPosition` (cdecl),
/// `SetIconAndDescription` (`RET 4`), the list's data (`007d86c0`, `RET 8`), `007dc9b0` (`RET 4`).
const TRIGGER_BURST: u32 = 0x007f_b150;
const SET_KNOB_POSITION: u32 = 0x007d_fc90;
const SET_ICON_AND_DESCRIPTION: u32 = 0x007d_c7e0;
const LIST_DATA_AT: u32 = 0x007d_86c0;
const SHOW_PERK: u32 = 0x007d_c9b0;
/// The names' compare (`00404dc0`, cdecl), `007e06b0` (no arguments).
const COMPARE_NAMES: u32 = 0x0040_4dc0;
const LIMB_VALUE: u32 = 0x007e_06b0;
/// The effect items: `GetMagnitude` and `GetDuration` (thiscall).
const EFFECT_GET_MAGNITUDE: u32 = 0x0040_4390;
const EFFECT_GET_DURATION: u32 = 0x0040_44d0;
/// The perk list's sort, `UpdatePerks` adds with.
const PERK_SORT: u32 = 0x007e_04e0;
/// `AddItem`'s callees: `Tile::GetMenu`, `Menu::RenderTemplate` (thiscall: parent, template, 0),
/// the list's insert (`007a7eb0`: item, compare) and append of the pointed-to word (`00905820`);
/// `"_number_of_visible_items"`, the value actions' add of a float (`00a09080`) and their actions.
const TILE_GET_MENU: u32 = 0x00a0_3c90;
const RENDER_TEMPLATE: u32 = 0x00a1_ddb0;
const LIST_INSERT: u32 = 0x007a_7eb0;
const LIST_APPEND_ITEM: u32 = 0x0090_5820;
const VISIBLE_ITEMS_TEXT: u32 = 0x0106_f860;
const VALUE_ADD_FLOAT: u32 = 0x00a0_9080;
const ACTION_COPY: u32 = 0x7d0;
const ACTION_ADD: u32 = 0x7d3;

/// The player.
#[inline(always)]
fn player(e: &Engine) -> u32 {
    e.mem.u32(PLAYER)
}

/// The player's actor values.
#[inline(always)]
fn actor_values(player: u32) -> u32 {
    player.wrapping_add(ACTOR_VALUES)
}

/// The open stats menu.
#[inline(always)]
fn open_menu(e: &Engine) -> u32 {
    e.mem.u32(OPEN_MENU)
}

/// `004d1360` on the player: hardcore mode.
#[inline(always)]
fn hardcore(e: &Engine, player: u32) -> bool {
    e.mem.u32(player.wrapping_add(HARDCORE)) == 1
}

/// A word of the menu.
#[inline(always)]
fn field(e: &Engine, this: u32, offset: u32) -> u32 {
    e.mem.u32(this.wrapping_add(offset))
}

/// Limb mode's byte.
#[inline(always)]
fn limb_mode(e: &Engine, this: u32) -> u8 {
    e.mem.u8(this.wrapping_add(LIMB_MODE))
}

/// A double constant of the exe.
#[inline(always)]
fn constant(e: &Engine, at: u32) -> f64 {
    f64::from_bits(e.mem.u64(at))
}

/// A float as `FLD dword` loads it (a signalling NaN quieted).
#[inline(always)]
fn load_float(e: &Engine, at: u32) -> f32 {
    x87_quiet(e.mem.f32(at))
}

/// `00403e20` on a constant setting object: its value (`+4`), loaded as a float.
#[inline(always)]
fn float_setting(e: &Engine, setting: u32) -> f32 {
    load_float(e, setting.wrapping_add(4))
}

/// `00403c80`: bit `flag` of the archetype flag table's entry `archetype` (16 bytes each).
#[inline(always)]
fn archetype_flag(e: &Engine, archetype: u32, flag: u32) -> bool {
    e.mem
        .u32(archetype.wrapping_shl(4).wrapping_add(0x0118_3328))
        & flag
        != 0
}

/// An effect item's setting (`00825c00`: its `+0x14`).
#[inline(always)]
fn effect_setting(e: &Engine, item: u32) -> u32 {
    e.mem.u32(item.wrapping_add(0x14))
}

/// `00403e60` on an effect setting: its flags (`+0x58`, `0058cba0`) have `flag`.
#[inline(always)]
fn setting_flag(e: &Engine, setting: u32, flag: u32) -> bool {
    e.mem.u32(setting.wrapping_add(0x58)) & flag != 0
}

/// `00403ea0` on an effect item: its actor value (`+0x10`) when its setting's archetype (`+0x98`,
/// `00810fd0`) has bit 2 (`00403ee0`, `00403c60`), else -1.
#[inline(always)]
fn effect_key(e: &Engine, item: u32) -> u32 {
    let archetype = e.mem.u32(effect_setting(e, item).wrapping_add(0x98));
    if archetype_flag(e, archetype, 2) {
        e.mem.u32(item.wrapping_add(0x10))
    } else {
        u32::MAX
    }
}

/// The low byte of a word.
#[inline(always)]
fn low_byte(value: u32) -> u8 {
    value.to_le_bytes()[0]
}

/// `Tile::SetString` with the flag 1 (the dispatch family's).
#[inline]
fn set_text(e: &mut Engine, tile: u32, id: i32, text: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1350,
        dispatch::tile::set_string(e, Ptr::new(tile), id, Ptr::new(text), 1)
    );
}

/// `Tile::GetString` (the dispatch family's).
#[inline]
fn get_text(e: &mut Engine, tile: u32, id: i32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_11f0,
        dispatch::tile::get_string(e, Ptr::new(tile), id)
    )
    .addr()
}

/// The string's text on the menu tile at the trait `007dfdf0` gives for `value`.
fn show_on_menu_tile(e: &mut Engine, this: Ptr, text: u32, value: u32) {
    let string = e.mem.u32(text);
    let id = typed_call!(super::FAMILY, 0x007d_fdf0, fn_007dfdf0(e, this, value));
    let root = field(e, this.addr(), MENU_TILE);
    set_text(e, root, id as i32, string);
}

/// `"%d/%d"` of `value`'s value and base value on the menu tile.
fn show_value_pair(e: &mut Engine, this: Ptr, text: u32, values: u32, value: u32) {
    let base = e.vcall(values, AV_BASE, &args![value]).u32();
    let current = e.vcall(values, AV_VALUE, &args![value]).u32();
    e.call(
        STRING_FORMAT,
        &args![text, VALUE_PAIR_FORMAT, current, base],
    );
    show_on_menu_tile(e, this, text, value);
}

/// The change marker `SPECIALUpdateFn` and `SkillsUpdateFn` give a row: `0x1004` 1, the arrow
/// (`0x1006`; up for a rise) and the row's item tile (`+0x28`, `0045cd60`) its `0x1004` 1.
fn show_change(e: &mut Engine, tile: u32, up: bool) {
    tile_set_int(e, tile, USER_1, 1);
    set_text(e, tile, USER_3, if up { ARROW_UP } else { ARROW_DOWN });
    let item = e.mem.u32(tile.wrapping_add(0x28));
    tile_set_int(e, item, USER_1, 1);
}

// Typed from 00406de0
/// The `StatsMenu::StatusDataList` destructor: the vtable stored, the entries freed (`007e0850`),
/// the embedded list destroyed (`0046ffb0` on `+4`).
pub fn fn_00406de0(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), STATUS_DATA_LIST_VTABLE);
    typed_call!(super::FAMILY, 0x007e_0850, fn_007e0850(e, this));
    e.call(LIST_DESTRUCT, &args![this.addr().wrapping_add(4)]);
}

// Typed from 007db1c0
/// The fade object at `011dace4`, made on first use: 0x4c bytes from the allocator constructed by
/// `007db2c0`, or 0 when the allocation fails.
pub fn fn_007db1c0(e: &mut Engine) -> u32 {
    let object = e.mem.u32(FADE_OBJECT);
    if object != 0 {
        return object;
    }
    let block = e.call(ALLOCATE, &args![0x4cu32]).u32();
    let object = if block == 0 {
        0
    } else {
        typed_call!(super::FAMILY, 0x007d_b2c0, fn_007db2c0(e, Ptr::new(block))).addr()
    };
    e.mem.set_u32(FADE_OBJECT, object);
    object
}

/// The fade object's group list `group` (eight bytes each from `+4`).
#[inline(always)]
fn group_list(this: Ptr, group: u32) -> u32 {
    this.addr()
        .wrapping_add(group.wrapping_mul(8))
        .wrapping_add(4)
}

// Typed from 007db250
/// The fade object's `group` gains `tile` (`005ae3d0` given the address of a word holding it).
pub fn fn_007db250(e: &mut Engine, this: Ptr, group: u32, tile: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), tile);
        e.call(LIST_APPEND, &args![group_list(this, group), slot]);
    });
}

// Typed from 007db270
/// Byte `index` of the fade object's group word `group` (`+0x34`) becomes `value`'s low byte (the
/// shifts by `CL`: the count modulo 32).
pub fn fn_007db270(e: &mut Engine, this: Ptr, group: u32, index: u32, value: u32) {
    let at = this
        .addr()
        .wrapping_add(group.wrapping_mul(4))
        .wrapping_add(0x34);
    let shift = index.wrapping_shl(3);
    let word = e.mem.u32(at) & !0xffu32.wrapping_shl(shift);
    e.mem.set_u32(at, word | value.wrapping_shl(shift));
}

// Typed from 007db2c0
/// The fade object's constructor: its six group lists from `+4` empty (the vector constructor over
/// `0096a2d0`, in place), the current group 0, the six group words at `+0x34` all bytes 0xff
/// (`00403d30`, in place). `this`.
pub fn fn_007db2c0(e: &mut Engine, this: Ptr) -> Ptr {
    let at = this.addr();
    for group in 0..6 {
        super::menu_manager::empty_list(e, group_list(this, group));
    }
    e.mem.set_u32(at, 0);
    for offset in (0x34..0x4cu32).step_by(4) {
        e.mem.set_u32(at.wrapping_add(offset), u32::MAX);
    }
    this
}

// Typed from 007db340
/// Whether the stats menu's 60 `pTiles` (`+0x90`) are all set.
pub fn fn_007db340(e: &mut Engine, this: Ptr) -> bool {
    (0..0x3cu32).all(|index| field(e, this.addr(), TILES + 4 * index) != 0)
}

// Typed from 007dbfe0
/// `FadeControlCallback` with `flag` over the tiles of the fade object's current group (its first
/// word; `007e0e30` on the group's list).
pub fn fn_007dbfe0(e: &mut Engine, this: Ptr, flag: u32) {
    let group = e.mem.u32(this.addr());
    let list = Ptr::new(group_list(this, group));
    typed_call!(
        super::FAMILY,
        0x007e_0e30,
        fn_007e0e30(e, list, FADE_CONTROL_CALLBACK, flag)
    );
}

// Typed from 007dc040
/// `StatsMenu::FadeControlCallback` (Xbox PDB): with `flag`, a looping fade control on the tile's
/// alpha (`0xfa9`, to the float at `01023cd8` from 0.0 over 1.0); without it, the fade control
/// removed and the alpha 0xff. True.
pub fn stats_menu_fade_control_callback(e: &mut Engine, tile: u32, flag: u32) -> bool {
    if flag != 0 {
        let target = load_float(e, FADE_TARGET);
        e.call(
            ADD_FADE_CONTROL,
            &args![tile, super::bits(TRAIT_ALPHA), target, 0.0f32, 1.0f32, 1u32],
        );
    } else {
        e.call(REMOVE_FADE_CONTROL, &args![tile, super::bits(TRAIT_ALPHA)]);
        tile_set_int(e, tile, TRAIT_ALPHA, 0xff);
    }
    true
}

// Typed from 007dc0a0
/// The word at `value` plus one; `value`.
pub fn fn_007dc0a0(e: &mut Engine, value: Ptr) -> Ptr {
    let word = e.mem.u32(value.addr()).wrapping_add(1);
    e.mem.set_u32(value.addr(), word);
    value
}

// Typed from 007dc4f0
/// `StatsMenu::DoIdle` (Xbox PDB): `bActiveEffectsUpdatedThisFrame` cleared. With the Pip-Boy up
/// (`Interface::GetPipboy`, its query byte) and in the Pip-Boy menu, once the time reaches the next
/// update, the radiation meter (`007ddb00`) and in hardcore mode the three others, and the next
/// update the time plus the menu tile's `0x1007` times 1000 (truncated); in limb mode with the stats
/// menu not on top, limb mode toggled. On the first page with the skill page past 2 (5 in hardcore
/// mode) the skill page reset. A hardcore state changed since the last idle shows on the three
/// hardcore tiles (`0xfaf`, `0xfa3`), and the state is kept.
pub fn stats_menu_do_idle(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    e.mem.set_u8(at.wrapping_add(EFFECTS_UPDATED), 0);
    let pipboy = typed_call!(
        super::FAMILY,
        0x0070_5990,
        super::interface_cpp::interface_get_pipboy(e)
    );
    if e.mem.u8(pipboy.wrapping_add(in_place::PIPBOY_QUERY)) != 0 {
        if e.call(IS_IN_PIPBOY_MENU, &args![]).bool() {
            let time = e.call(TIME, &args![]).u32();
            if time >= e.mem.u32(NEXT_UPDATE) {
                typed_call!(super::FAMILY, 0x007d_db00, fn_007ddb00(e, this));
                let player = player(e);
                if player != 0 && hardcore(e, player) {
                    for meter in HARDCORE_METERS {
                        e.call(meter, &args![this]);
                    }
                }
                let time = e.call(TIME, &args![]).u32();
                let delay = tile_float(e, field(e, at, MENU_TILE), USER_4);
                let delay = super::narrow(f64::from(delay) * constant(e, THOUSAND));
                let next = super::narrow(f64::from(delay) + f64::from(time));
                let next = super::bits(super::low_dword_signed(fistp_chop64(f64::from(next))));
                e.mem.set_u32(NEXT_UPDATE, next);
            }
        }
        if limb_mode(e, at) != 0
            && typed_call!(
                super::FAMILY,
                0x0070_2450,
                super::interface_cpp::interface_is_top_menu_id(e, 0x3eb)
            ) == 0
        {
            e.call(TOGGLE_LIMB_MODE, &args![this]);
        }
    }
    let player = player(e);
    if typed_call!(super::FAMILY, 0x007d_cc00, fn_007dcc00(e, this)) == 0 {
        let page = typed_call!(super::FAMILY, 0x007d_cc40, fn_007dcc40(e, this));
        let limit = if hardcore(e, player) { 5 } else { 2 };
        if page > limit {
            e.call(SET_SKILL_PAGE, &args![this, 0u32]);
        }
    }
    let shown = e.mem.u8(HARDCORE_SHOWN) != 0;
    let now = hardcore(e, player);
    if player != 0 && now != shown {
        let value = i32::from(now);
        let menu = open_menu(e);
        for offset in [0x164u32, 0x174, 0x16c] {
            let tile = field(e, menu, offset);
            for id in [TRAIT_ENABLED, TRAIT_VISIBLE] {
                tile_set_int(e, tile, id, value);
            }
        }
    }
    e.mem.set_u8(HARDCORE_SHOWN, u8::from(now));
}

/// A tile's trait `0x1004` truncated (`FISTP qword` with chopping; the low word).
fn page_of(e: &mut Engine, tile: u32) -> i32 {
    let value = tile_float(e, tile, USER_1);
    super::low_dword_signed(fistp_chop64(f64::from(value)))
}

// Typed from 007dcc00
/// The stats menu's current page: the menu tile's `0x1004`, truncated.
pub fn fn_007dcc00(e: &mut Engine, this: Ptr) -> i32 {
    page_of(e, field(e, this.addr(), MENU_TILE))
}

// Typed from 007dcc40
/// The skill page: the `0x1004` of the tile at `+0xa4`, truncated.
pub fn fn_007dcc40(e: &mut Engine, this: Ptr) -> i32 {
    page_of(e, field(e, this.addr(), SKILL_PAGE_TILE))
}

// Typed from 007dcfc0
/// `StatsMenu::UpdateAllActorValues` (Xbox PDB): `UpdateActorValue` of 0x10, 0xc, 0x18, 0x36, 0x14
/// and 0x19 to 0x1f; the SPECIAL and skill pages reset (`0x1004` of `+0xa8` and `+0xac` 0) and
/// their lists' rows updated (`007314c0` with `SPECIALUpdateFn` and `SkillsUpdateFn`, every index);
/// then `UpdateActorValue(0x17)`.
pub fn stats_menu_update_all_actor_values(e: &mut Engine, this: Ptr) {
    for value in [0x10u32, 0xc, 0x18, 0x36, 0x14]
        .into_iter()
        .chain(0x19..0x20)
    {
        typed_call!(
            super::FAMILY,
            0x007d_d090,
            stats_menu_update_actor_value(e, this, value)
        );
    }
    for (page, list, update) in [
        (0xa8u32, SPECIAL_LIST, SPECIAL_UPDATE_FN),
        (0xac, SKILL_LIST, SKILLS_UPDATE_FN),
    ] {
        tile_set_int(e, field(e, this.addr(), page), USER_1, 0);
        e.call(
            LIST_WALK,
            &args![this.addr().wrapping_add(list), update, 0u32, 0x7fff_ffffu32],
        );
    }
    typed_call!(
        super::FAMILY,
        0x007d_d090,
        stats_menu_update_actor_value(e, this, 0x17)
    );
}

/// The karma icons and the game settings with their names, by alignment (`UpdateActorValue`'s jump
/// table).
const KARMA: [(u32, u32); 5] = [
    (0x0107_7450, 0x011d_41b0),
    (0x0107_7488, 0x011d_3204),
    (0x0107_7418, 0x011d_457c),
    (0x0107_73dc, 0x011d_4ffc),
    (0x0107_73a4, 0x011d_31d4),
];

// Typed from 007dd090
/// `StatsMenu::UpdateActorValue` (Xbox PDB), with a local string: 0x10 the face image (tile
/// `+0xf0`), the value pair and `UpdateHealingOptions(0)`; 0xc the value pair; 0x18 the experience
/// and the next level's, or at the level cap the "max level" text; 0x36 the radiation meter,
/// `UpdateHealingOptions(2)` and `UpdateActiveEffects`; 0x14 the smaller of its truncated setting
/// and its clamped value (tile `+0x128`), then `UpdateActiveEffects`; 0x17 the karma's icon and
/// name (tiles `+0x120`, `+0x11c`), then `UpdatePlayerTitle`; 0x19 to 0x1e a body part's image
/// (broken unless its condition is above 0) and its condition over 100 as `"_Value"`, then in limb
/// mode `UpdateHealingOptions(0)`; any other value its SPECIAL and skill rows.
pub fn stats_menu_update_actor_value(e: &mut Engine, this: Ptr, value: u32) {
    let at = this.addr();
    e.with_stack(0xc, |e, frame| {
        let text = frame.addr();
        let value_slot = text.wrapping_add(8);
        e.mem.set_u32(value_slot, value);
        in_place::empty_string(e, text);
        let player = player(e);
        let values = actor_values(player);
        match value {
            0x10 => {
                let face = typed_call!(super::FAMILY, 0x007d_fe40, fn_007dfe40(e, this));
                e.call(STRING_FORMAT, &args![text, FACE_FORMAT, face]);
                let string = e.mem.u32(text);
                set_text(e, field(e, at, FACE_TILE), TRAIT_IMAGE, string);
                show_value_pair(e, this, text, values, 0x10);
                typed_call!(
                    super::FAMILY,
                    0x007d_f230,
                    stats_menu_update_healing_options(e, this, 0)
                );
            }
            0xc => show_value_pair(e, this, text, values, 0xc),
            0x18 => {
                let level = e.call(PLAYER_LEVEL, &args![player]).u16();
                if i32::from(level) < e.mem.i32(LEVEL_CAP) {
                    let level = e.call(PLAYER_LEVEL, &args![player]).u16();
                    let required = e
                        .call(REQUIRED_EXPERIENCE, &args![u32::from(level) + 1])
                        .u32();
                    let current = e.vcall(values, AV_VALUE, &args![0x18u32]).u32();
                    e.call(
                        STRING_FORMAT,
                        &args![text, VALUE_PAIR_FORMAT, current, required],
                    );
                } else {
                    let label = in_place::setting_text(e, MAX_LEVEL_SETTING);
                    e.call(STRING_FORMAT, &args![text, ONE_STRING_FORMAT, label]);
                }
                show_on_menu_tile(e, this, text, 0x18);
            }
            0x36 => {
                typed_call!(super::FAMILY, 0x007d_db00, fn_007ddb00(e, this));
                typed_call!(
                    super::FAMILY,
                    0x007d_f230,
                    stats_menu_update_healing_options(e, this, 2)
                );
                e.call(0x007d_df00, &args![this]);
            }
            0x14 => {
                let clamped = e
                    .call(GET_CLAMPED_ACTOR_VALUE, &args![values, 0x14u32])
                    .i32();
                let setting = float_setting(e, VALUE_14_SETTING);
                let whole = ftol2(e, f64::from(setting));
                // `004a8f20`, in place: the smaller (signed).
                let shown = whole.min(clamped);
                e.call(STRING_FORMAT, &args![text, VALUE_14_FORMAT, shown]);
                let string = e.mem.u32(text);
                set_text(e, field(e, at, 0x128), TRAIT_STRING, string);
                e.call(0x007d_df00, &args![this]);
            }
            0x17 => {
                let karma = super::narrow(e.vcall(values, AV_FLOAT, &args![0x17u32]).f64());
                let alignment = typed_call!(
                    forms::FAMILY,
                    0x0047_e040,
                    forms::actor_base_data::alignment_for_karma(e, karma)
                );
                if let Some(&(icon, setting)) = usize::try_from(alignment)
                    .ok()
                    .and_then(|index| KARMA.get(index))
                {
                    set_text(e, field(e, at, 0x120), TRAIT_IMAGE, icon);
                    let name = in_place::setting_text(e, setting);
                    set_text(e, field(e, at, 0x11c), TRAIT_STRING, name);
                }
                typed_call!(
                    super::FAMILY,
                    0x007d_f5d0,
                    stats_menu_update_player_title(e)
                );
            }
            0x19..=0x1e => {
                let image = 0xc + 2 * (value - 0x19);
                let condition = e.vcall(values, AV_FLOAT, &args![value]).f64();
                let fraction = super::narrow(condition / constant(e, HUNDRED));
                let format = if f64::from(fraction) > constant(e, ZERO) {
                    BODY_PART_FORMAT
                } else {
                    BODY_PART_BROKEN_FORMAT
                };
                let name = e.mem.u32(BODY_PART_NAMES.wrapping_add(4 * value));
                e.call(STRING_FORMAT, &args![text, format, name]);
                let string = e.mem.u32(text);
                set_text(e, field(e, at, TILES + 4 * image), TRAIT_IMAGE, string);
                let value_trait = e.call(ADD_USER_TRAIT, &args![VALUE_TEXT, u32::MAX]).i32();
                tile_set_float(
                    e,
                    field(e, at, TILES + 4 * (image + 1)),
                    value_trait,
                    x87_quiet(fraction),
                );
                if limb_mode(e, at) != 0 {
                    typed_call!(
                        super::FAMILY,
                        0x007d_f230,
                        stats_menu_update_healing_options(e, this, 0)
                    );
                }
            }
            _ => {
                let row = e
                    .call(
                        GET_TILE_FOR,
                        &args![at.wrapping_add(SPECIAL_LIST), value_slot],
                    )
                    .u32();
                typed_call!(
                    super::FAMILY,
                    0x007d_fa30,
                    stats_menu_special_update_fn(e, row, value)
                );
                let value = e.mem.u32(value_slot);
                let row = e
                    .call(
                        GET_TILE_FOR,
                        &args![at.wrapping_add(SKILL_LIST), value_slot],
                    )
                    .u32();
                typed_call!(
                    super::FAMILY,
                    0x007d_fb00,
                    stats_menu_skills_update_fn(e, row, value)
                );
            }
        }
        e.call(STRING_DESTROY, &args![text]);
    });
}

// Typed from 007dd710
/// `StatsMenu::UpdatePerks` (Xbox PDB): with the open menu and its perk tile (`+0x118`), the perk
/// list (`+0x1e0`) emptied (its `RemoveAll` slot) and every perk rank of the player whose perk is
/// not hidden (`007dd8a0`) added (`007e0e70`, sorted by `007e04e0`) with its name, `"%s (%d)"` with
/// the rank above 1. The walk stops at a node with no rank.
pub fn stats_menu_update_perks(e: &mut Engine) {
    let menu = open_menu(e);
    if menu == 0 || field(e, menu, PERK_TILE) == 0 {
        return;
    }
    e.vcall(menu.wrapping_add(PERK_LIST), 0x1c, &args![]);
    let player = player(e);
    let mut node = e.vcall(player, PERK_RANKS_SLOT, &args![0u32]).u32();
    e.with_stack(8, |e, text| {
        let text = text.addr();
        while node != 0 {
            let rank = e.mem.u32(node);
            if rank == 0 {
                break;
            }
            let perk = e.mem.u32(rank);
            if typed_call!(super::FAMILY, 0x007d_d8a0, fn_007dd8a0(e, Ptr::new(perk))) == 0 {
                in_place::empty_string(e, text);
                let level = e.mem.u8(rank.wrapping_add(4));
                let name = name_characters(e, perk.wrapping_add(0x18));
                if level > 1 {
                    e.call(
                        STRING_FORMAT,
                        &args![text, RANK_FORMAT, name, u32::from(level)],
                    );
                } else {
                    e.call(STRING_FORMAT, &args![text, ONE_STRING_FORMAT, name]);
                }
                let string = e.mem.u32(text);
                let list = Ptr::new(open_menu(e).wrapping_add(PERK_LIST));
                typed_call!(
                    super::FAMILY,
                    0x007e_0e70,
                    fn_007e0e70(e, list, rank, string, PERK_SORT, 0)
                );
                e.call(STRING_DESTROY, &args![text]);
            }
            node = e.mem.u32(node.wrapping_add(4));
        }
    });
}

// Typed from 007dd8a0
/// A perk's hidden byte (`+0x3c`).
pub fn fn_007dd8a0(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr().wrapping_add(0x3c))
}

// Typed from 007ddad0
/// A misc stat's row in the general list (`+0x210`, `GetTileFor` given the address of a word
/// holding it) shows the stat's value (`007dfc60`).
pub fn fn_007ddad0(e: &mut Engine, this: Ptr, stat: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), stat);
        let list = this.addr().wrapping_add(GENERAL_LIST);
        let row = e.call(GET_TILE_FOR, &args![list, slot]).u32();
        typed_call!(super::FAMILY, 0x007d_fc60, fn_007dfc60(e, row, stat));
    });
}

// Typed from 007ddb00
/// The radiation meter: `a` the menu tile's `0x1008` times 1000, `c = 005c5420() * a - a / 2`, `d`
/// the larger of 0.0 and the player's radiation (actor value 0x36), the knob's angle
/// `pi/2 - (d + c) / 1000 * pi`. A `d` strictly between 0 and 1 shows as 1, any other as
/// `004bd510(d, 1.0)` (the menu tile's `0x1005`); the Pip-Boy's knob (`007f86f0` on the manager's
/// Pip-Boy) gets (1, the angle, the float at `010780a4`).
pub fn fn_007ddb00(e: &mut Engine, this: Ptr) {
    let root = field(e, this.addr(), MENU_TILE);
    let value = tile_float(e, root, USER_5);
    let a = super::narrow(f64::from(value) * constant(e, THOUSAND));
    // `FLD a; FCHS; FDIV 2.0`, kept as a double.
    let b = f64::from(super::narrow(-f64::from(a) / constant(e, TWO)));
    let scale = e.call(METER_SCALE, &args![]).f64();
    let c = super::narrow(f64::from(super::narrow(scale * f64::from(a))) + b);
    let rads = super::narrow(
        e.vcall(actor_values(player(e)), AV_FLOAT, &args![0x36u32])
            .f64(),
    );
    let d = typed_call!(
        draw_f::FAMILY,
        0x0040_4010,
        draw_f::helpers::float_max(e, 0.0, rads)
    );
    let sum = super::narrow(f64::from(d) + f64::from(c));
    let step = super::narrow(f64::from(sum) / constant(e, THOUSAND));
    let turn = super::narrow(f64::from(step) * constant(e, PI));
    let angle = super::narrow(constant(e, HALF_PI) - f64::from(turn));
    // `TEST AH,0x41` against 0.0, then `TEST AH,5` against 1.0: strictly between, ordered.
    let shown = if f64::from(d) > constant(e, ZERO) && f64::from(d) < constant(e, ONE) {
        1.0
    } else {
        super::narrow(
            e.call(ROUND_UP_TO_MULTIPLE, &args![x87_quiet(d), 1.0f32])
                .f64(),
        )
    };
    tile_set_float(e, root, USER_2, x87_quiet(shown));
    let rate = load_float(e, KNOB_RATE);
    let pipboy = in_place::pipboy(e, in_place::manager(e));
    e.call(PIPBOY_KNOB, &args![pipboy, 1u32, x87_quiet(angle), rate]);
}

// Typed from 007ddef0
/// The float at `01013974` (`FLD`, a signalling NaN quieted).
pub fn fn_007ddef0(e: &mut Engine) -> f32 {
    load_float(e, RETURNED_FLOAT)
}

// Typed from 007df1b0
/// Whether an effect setting's archetype (`+0x98`) has the archetype flag 0x40.
pub fn fn_007df1b0(e: &mut Engine, this: Ptr) -> bool {
    let archetype = e.mem.u32(this.addr().wrapping_add(0x98));
    archetype_flag(e, archetype, 0x40)
}

// Typed from 007df1d0
/// An active effect's setting: the effect item's (`+0xc`) word at `+0x14`.
pub fn fn_007df1d0(e: &mut Engine, this: Ptr) -> u32 {
    effect_setting(e, e.mem.u32(this.addr().wrapping_add(0xc)))
}

// Typed from 007df210
/// 1 when bit 7 of the word at `+0x18` is set, else 0.
pub fn fn_007df210(e: &mut Engine, this: Ptr) -> u32 {
    u32::from(e.mem.u32(this.addr().wrapping_add(0x18)) & 0x80 != 0)
}

// Typed from 007df230
/// `StatsMenu::UpdateHealingOptions` (Xbox PDB; an option 0 to 3, 4 for all). Nothing without a
/// player. Each chosen option's old inventory entry is deleted (`004459e0` with 1), the player's for
/// its form taken (`GetInventoryItem(form, 0)`), its tile's `0xfaf` the callback's answer; with an
/// entry the tile's text is `"(%d) %s"` of its count and the tile's `0x1004` (option 0 outside
/// hardcore mode, and option 3, enable the healing tile `+0xb4`), without one just that string. For
/// options 0, 3 and 4, with neither the first nor the fourth option's item carried (in hardcore mode
/// the fourth alone), limb mode ends (`007e0230`) and the healing tile is disabled.
pub fn stats_menu_update_healing_options(e: &mut Engine, this: Ptr, which: u32) {
    let player = player(e);
    if player == 0 {
        return;
    }
    let at = this.addr();
    e.with_stack(8, |e, text| {
        let text = text.addr();
        for index in 0..4u32 {
            if which != index && which != 4 {
                continue;
            }
            let record = at.wrapping_add(HEALING_OPTIONS + index * 0x10);
            in_place::empty_string(e, text);
            let old = field(e, record, 8);
            if old != 0 {
                e.call(ITEM_DELETE, &args![old, 1u32]);
            }
            let form = e.mem.u32(record);
            let entry = e.call(GET_INVENTORY_ITEM, &args![player, form, 0u32]).u32();
            e.mem.set_u32(record.wrapping_add(8), entry);
            let callback = field(e, record, 0xc);
            let available = e.call(callback, &args![record]).u8();
            let tile = field(e, record, 4);
            tile_set_int(e, tile, TRAIT_ENABLED, i32::from(available));
            let entry = field(e, record, 8);
            let tile = field(e, record, 4);
            let name = get_text(e, tile, USER_1);
            if entry != 0 {
                let count = e.mem.u32(entry.wrapping_add(4));
                e.call(STRING_FORMAT, &args![text, HEALING_FORMAT, count, name]);
                if (index == 0 && !hardcore(e, player)) || index == 3 {
                    tile_set_int(e, field(e, at, HEALING_TILE), TRAIT_ENABLED, 1);
                }
            } else {
                e.call(STRING_ASSIGN, &args![text, name]);
            }
            let string = e.mem.u32(text);
            set_text(e, tile, TRAIT_STRING, string);
            e.call(STRING_DESTROY, &args![text]);
        }
    });
    if matches!(which, 0 | 3 | 4) {
        let first = field(e, at, HEALING_OPTIONS);
        let first = e.call(GET_ITEM_COUNT, &args![player, first]).u32();
        let fourth = field(e, at, HEALING_OPTIONS + 0x30);
        let fourth = e.call(GET_ITEM_COUNT, &args![player, fourth]).u32();
        let unavailable = fourth == 0 && (hardcore(e, player) || first == 0);
        if unavailable {
            if limb_mode(e, at) != 0 {
                e.call(TOGGLE_LIMB_MODE, &args![this]);
            }
            tile_set_int(e, field(e, at, HEALING_TILE), TRAIT_ENABLED, 0);
        }
    }
}

// Typed from 007df4e0
/// `StatsMenu::UpdatePlayerName` (Xbox PDB): with the open menu and the player, tile `+0xfc` shows
/// `"%s - %s %hd"` of the player's name, the level label and the level, and the menu tile gets the
/// level as `0x100c`.
pub fn stats_menu_update_player_name(e: &mut Engine) {
    let menu = open_menu(e);
    let player = player(e);
    if menu == 0 || player == 0 {
        return;
    }
    e.with_stack(8, |e, text| {
        let text = text.addr();
        in_place::empty_string(e, text);
        let level = e.call(PLAYER_LEVEL, &args![player]).u16();
        let label = in_place::setting_text(e, LEVEL_SETTING);
        let name = e.call(PLAYER_NAME, &args![player]).u32();
        e.call(
            STRING_FORMAT,
            &args![text, NAME_FORMAT, name, label, u32::from(level)],
        );
        let string = e.mem.u32(text);
        set_text(e, field(e, menu, 0xfc), TRAIT_STRING, string);
        let level = e.call(PLAYER_LEVEL, &args![player]).u16();
        tile_set_int(e, field(e, menu, MENU_TILE), USER_LEVEL, i32::from(level));
        e.call(STRING_DESTROY, &args![text]);
    });
}

// Typed from 007df5d0
/// `StatsMenu::UpdatePlayerTitle` (Xbox PDB): with the open menu's title tile (`+0x124`), its text
/// the player's karmic title.
pub fn stats_menu_update_player_title(e: &mut Engine) {
    let menu = open_menu(e);
    if menu == 0 {
        return;
    }
    let tile = field(e, menu, 0x124);
    if tile == 0 {
        return;
    }
    let player = player(e);
    let title = typed_call!(
        forms::FAMILY,
        0x0047_e0e0,
        forms::actor_base_data::karmic_title(e, player)
    );
    set_text(e, tile, TRAIT_STRING, title);
}

// Typed from 007df620
/// `StatsMenu::UpdateHealingMode` (Xbox PDB): the `"_xbox_button"` and `"_enabled"` ids looked up
/// once (function-local statics). With the healing tile (`+0xb4`) and `004b71d0`: its button 10 in
/// limb mode, else 11, its text the mode's setting, and the eleven page tiles enabled outside limb
/// mode. Then the fade object's current group fades in limb mode (`007dbfe0`, given the mode byte
/// itself).
pub fn stats_menu_update_healing_mode(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let button = static_trait(e, HEALING_GUARD, 1, XBOX_BUTTON_TRAIT, XBOX_BUTTON_TEXT);
    let enabled = static_trait(e, HEALING_GUARD, 2, ENABLED_TRAIT, ENABLED_TEXT);
    let mode = limb_mode(e, at);
    let limb = mode != 0;
    let healing = field(e, at, HEALING_TILE);
    if healing != 0 && e.call(MANAGER_STATE_TEST, &args![]).bool() {
        tile_set_int(e, healing, button, if limb { 0xa } else { 0xb });
        let setting = if limb {
            LIMB_MODE_SETTING
        } else {
            HEALING_MODE_SETTING
        };
        let label = in_place::setting_text(e, setting);
        set_text(e, healing, TRAIT_STRING, label);
        for offset in [
            0x90u32, 0x94, 0x98, 0x9c, 0xa0, 0x100, 0x104, 0x164, 0x174, 0x16c, 0x108,
        ] {
            tile_set_int(e, field(e, at, offset), enabled, i32::from(!limb));
        }
    }
    let fade = typed_call!(super::FAMILY, 0x007d_b1c0, fn_007db1c0(e));
    typed_call!(
        super::FAMILY,
        0x007d_bfe0,
        fn_007dbfe0(e, Ptr::new(fade), u32::from(mode))
    );
}

// Typed from 007dfa30
/// `StatsMenu::SPECIALUpdateFn` (Xbox PDB): a row shows the clamped value (`0x1005`); one that
/// differs from the base value shows the change marker (up when above it), else `0x1004` is 0.
pub fn stats_menu_special_update_fn(e: &mut Engine, tile: u32, value: u32) {
    if tile == 0 {
        return;
    }
    let values = actor_values(player(e));
    let current = e.call(GET_CLAMPED_ACTOR_VALUE, &args![values, value]).i32();
    let base = e.vcall(values, AV_BASE, &args![value]).i32();
    tile_set_int(e, tile, USER_2, current);
    if base == current {
        tile_set_int(e, tile, USER_1, 0);
    } else {
        show_change(e, tile, current > base);
    }
}

// Typed from 007dfb00
/// `StatsMenu::SkillsUpdateFn` (Xbox PDB): the change is the clamped value less the base value plus
/// the rounded (`00406d90`) difference of the derived skill without and with modifiers
/// (`AVDeriveSkill(values, value, 0)` and `(.., 1)`, each a float, the difference a float); the row
/// shows the clamped value and, for a change, the change marker (up when positive).
pub fn stats_menu_skills_update_fn(e: &mut Engine, tile: u32, value: u32) {
    if tile == 0 {
        return;
    }
    let player = player(e);
    let values = actor_values(player);
    let current = e.call(GET_CLAMPED_ACTOR_VALUE, &args![values, value]).i32();
    let base = e.vcall(values, AV_BASE, &args![value]).i32();
    let owner = if player == 0 { 0 } else { values };
    let derived = super::narrow(e.call(AV_DERIVE_SKILL, &args![owner, value, 0u32]).f64());
    let modified = super::narrow(e.call(AV_DERIVE_SKILL, &args![owner, value, 1u32]).f64());
    let rounded = e.call(ROUND, &args![derived - modified]).i32();
    let change = current.wrapping_sub(base).wrapping_add(rounded);
    tile_set_int(e, tile, USER_2, current);
    if change == 0 {
        tile_set_int(e, tile, USER_1, 0);
    } else {
        show_change(e, tile, change > 0);
    }
}

// Typed from 007dfc60
/// A row (when there is one) shows a misc stat's value (`005a3370`) as `0x1005`.
pub fn fn_007dfc60(e: &mut Engine, tile: u32, stat: u32) {
    if tile != 0 {
        let value = e.call(MISC_STAT_VALUE, &args![stat]).i32();
        tile_set_int(e, tile, USER_2, value);
    }
}

// Typed from 007dfcd0
/// The low byte of the open stats menu's current page (`007dcc00`), 0 without one.
pub fn fn_007dfcd0(e: &mut Engine) -> u8 {
    let menu = open_menu(e);
    if menu == 0 {
        return 0;
    }
    let page = typed_call!(super::FAMILY, 0x007d_cc00, fn_007dcc00(e, Ptr::new(menu)));
    low_byte(super::bits(page))
}

// Typed from 007dfdf0
/// The trait an actor value's pair shows in: 0x10 `0x1009`, 0xc `0x100a`, 0x18 `0x100b`, anything
/// else `0x80000000`.
pub fn fn_007dfdf0(_e: &mut Engine, _this: Ptr, value: u32) -> u32 {
    match value {
        0xc => 0x100a,
        0x10 => 0x1009,
        0x18 => 0x100b,
        _ => 0x8000_0000,
    }
}

// Typed from 007dfe40
/// The face image's number: 10 while the player has an active effect of kind 0xa that is active
/// (the walk stops at a node with no effect); else `_ftol2((base - value) / base * 5)` of actor
/// value 0x10 (the base an integer stored as a float, the difference a float), within 0 to 4.
pub fn fn_007dfe40(e: &mut Engine, _this: Ptr) -> i32 {
    let player = player(e);
    let mut node = e
        .vcall(player.wrapping_add(ACTIVE_EFFECTS), 8, &args![])
        .u32();
    while node != 0 {
        let effect = e.mem.u32(node);
        if effect == 0 {
            break;
        }
        if e.mem.u32(effect.wrapping_add(0x2c)) == 0xa && e.mem.u8(effect.wrapping_add(0x10)) != 0 {
            return 0xa;
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    let values = actor_values(player);
    let base = e.vcall(values, AV_BASE, &args![0x10u32]).i32();
    let base = super::narrow(f64::from(base));
    let value = e.vcall(values, AV_FLOAT, &args![0x10u32]).f64();
    let missing = super::narrow(f64::from(base) - value);
    let share = missing / base;
    let ratio = super::narrow(f64::from(share) * constant(e, FIVE));
    ftol2(e, f64::from(ratio)).clamp(0, 4)
}

// Typed from 007dff40
/// `StatsMenu::SetCurrentPage` (Xbox PDB; a page 0 to 4, others nothing): the Pip-Boy bursts
/// (`FORenderedMenu::TriggerBurst` with two settings and the float at `01016248`), the menu tile's
/// `0x1004` the page, the knob moved with the Pip-Boy up (`SetKnobPosition(1)`), limb mode ended;
/// page 1 shows icon 5, page 2 icon 0x20, page 3 with perks the first perk's (`007dc9b0` of
/// `007d86c0(perks, 0, 0)`), any other hides the icon tile (`+0xf4`).
pub fn stats_menu_set_current_page(e: &mut Engine, this: Ptr, page: i32) {
    if !(0..=4).contains(&page) {
        return;
    }
    let at = this.addr();
    let burst = load_float(e, BURST_SCALE);
    let first = float_setting(e, BURST_SETTING_A);
    let second = float_setting(e, BURST_SETTING_B);
    let pipboy = in_place::pipboy(e, in_place::manager(e));
    e.call(TRIGGER_BURST, &args![pipboy, second, first, burst]);
    tile_set_int(e, field(e, at, MENU_TILE), USER_1, page);
    let pipboy = in_place::pipboy(e, in_place::manager(e));
    if e.mem.u8(pipboy.wrapping_add(in_place::PIPBOY_QUERY)) != 0 {
        e.call(SET_KNOB_POSITION, &args![1u32]);
    }
    if limb_mode(e, at) != 0 {
        e.call(TOGGLE_LIMB_MODE, &args![this]);
    }
    match page {
        1 => {
            e.call(SET_ICON_AND_DESCRIPTION, &args![this, 5u32]);
        }
        2 => {
            e.call(SET_ICON_AND_DESCRIPTION, &args![this, 0x20u32]);
        }
        _ => {
            let perks = Ptr::new(at.wrapping_add(PERK_LIST));
            if page == 3
                && typed_call!(
                    super::FAMILY,
                    0x0072_6710,
                    menu_manager_p2::list_box_perk_rank_data_num_unfiltered(e, perks)
                ) != 0
            {
                let perk = e.call(LIST_DATA_AT, &args![perks, 0u32, 0u32]).u32();
                e.call(SHOW_PERK, &args![this, perk]);
            } else {
                tile_set_int(e, field(e, at, ICON_TILE), TRAIT_VISIBLE, 0);
            }
        }
    }
}

/// A list item's perk's name: the characters of the name at `+0x18` of its data's (`+4`) first word.
#[inline]
fn perk_name(e: &Engine, item: u32) -> u32 {
    let perk = e.mem.u32(e.mem.u32(item.wrapping_add(4)));
    name_characters(e, perk.wrapping_add(0x18))
}

// Typed from 007e04e0
/// The perk list's sort: the names' compare (`00404dc0`), the second's looked up first.
pub fn fn_007e04e0(e: &mut Engine, first: u32, second: u32) -> i32 {
    let second = perk_name(e, second);
    let first = perk_name(e, first);
    e.call(COMPARE_NAMES, &args![first, second]).i32()
}

// Typed from 007e05f0
/// The first healing option's availability: false without an inventory entry (`+8`). With
/// `007e06b0`'s actor value (-1 for none): in hardcore mode any but -1 answers false; outside it the
/// value is used only when `004b71d0` says so, else 0x10. Whether the player's value is below its
/// base.
pub fn fn_007e05f0(e: &mut Engine, record: Ptr) -> bool {
    let player = player(e);
    let hardcore = hardcore(e, player);
    if field(e, record.addr(), 8) == 0 {
        return false;
    }
    let mut value = e.call(LIMB_VALUE, &args![]).u32();
    if hardcore && value != u32::MAX {
        return false;
    }
    if hardcore || !e.call(MANAGER_STATE_TEST, &args![]).bool() || value == u32::MAX {
        value = 0x10;
    }
    let values = actor_values(player);
    let current = e.vcall(values, AV_VALUE, &args![value]).i32();
    let base = e.vcall(values, AV_BASE, &args![value]).i32();
    current < base
}

// Typed from 007e07a0
/// The radiation option's availability: an inventory entry (`+8`) and radiation above 0.0.
pub fn fn_007e07a0(e: &mut Engine, record: Ptr) -> bool {
    if field(e, record.addr(), 8) == 0 {
        return false;
    }
    let rads = e
        .vcall(actor_values(player(e)), AV_FLOAT, &args![0x36u32])
        .f64();
    rads > constant(e, ZERO)
}

/// Whether an embedded list (`008256d0`) is empty: both its words zero.
#[inline(always)]
fn list_is_empty(e: &Engine, list: u32) -> bool {
    e.mem.u32(list) == 0 && e.mem.u32(list.wrapping_add(4)) == 0
}

// Typed from 007e07f0
/// A status effect list emptied: until its embedded list (`+4`) is empty, its first item deleted
/// (`0071f9c0` with 1, when there is one) and the first node removed (`0063f7b0`).
pub fn fn_007e07f0(e: &mut Engine, this: Ptr) {
    let list = this.addr().wrapping_add(4);
    while !list_is_empty(e, list) {
        let item = e.mem.u32(list);
        if item != 0 {
            typed_call!(
                super::FAMILY,
                0x0071_f9c0,
                super::menu_manager::fn_0071f9c0(e, Ptr::new(item), 1)
            );
        }
        e.call(LIST_REMOVE_FIRST, &args![list]);
    }
}

// Typed from 007e0850
/// A status data list emptied: until its embedded list is empty, its first item freed and the
/// first node removed.
pub fn fn_007e0850(e: &mut Engine, this: Ptr) {
    let list = this.addr().wrapping_add(4);
    while !list_is_empty(e, list) {
        let item = e.mem.u32(list);
        e.call(FREE, &args![item]);
        e.call(LIST_REMOVE_FIRST, &args![list]);
    }
}

// Typed from 007e09b0
/// The embedded list's nodes freed (`00470470` on `+4`).
pub fn fn_007e09b0(e: &mut Engine, this: Ptr) {
    e.call(LIST_FREE_NODES, &args![this.addr().wrapping_add(4)]);
}

/// `Register`'s `EffectItem::GetDuration` (`0x7E0C1F`, `CALL 0x4044D0`). JIP LN NVSE's G107 (a fix,
/// the row `NV_JIP_GAME_PATCHES`; `WriteRelCall(0x7E0C1F, EffectItemGetModifiedDuration)`) answers
/// it (words: the effect item; answered: the duration, a chem's after the player's
/// `ModifyPositiveChemDuration` perks). With the word's `FIXES` bit clear: the exe's call.
#[inline]
fn effect_duration(e: &mut Engine, effect: u32) -> u32 {
    if e.jip_patches() & engine_core::extend::jip_patches::FIXES != 0 {
        if let Some(duration) = e.site(0x007e_0c1f, &[effect]) {
            return duration;
        }
    }
    e.call(EFFECT_GET_DURATION, &args![effect]).u32()
}

// Typed from 007e0b20
/// `StatsMenu::StatusDataList::Register` (Xbox PDB; an effect item): an entry with the effect's
/// key (`007e0ce0`) gains its magnitude; otherwise a new 0x14-byte entry (`00403ba0`, in place)
/// takes the key and the magnitude (negated when the setting has flag 4), the duration unless the
/// setting has flag 2 (the G107 seam), and is appended to the list (`+4`).
pub fn stats_menu_status_data_list_register(e: &mut Engine, this: Ptr, effect: u32) {
    if effect == 0 {
        return;
    }
    let key = effect_key(e, effect);
    let found = typed_call!(super::FAMILY, 0x007e_0ce0, fn_007e0ce0(e, this, key));
    if found != 0 {
        let magnitude = e.call(EFFECT_GET_MAGNITUDE, &args![effect]).u32();
        let total = magnitude.wrapping_add(e.mem.u32(found));
        e.mem.set_u32(found, total);
        return;
    }
    let block = e.call(ALLOCATE, &args![0x14u32]).u32();
    if block != 0 {
        for offset in [0u32, 4, 8, 0xc] {
            e.mem.set_u32(block.wrapping_add(offset), 0);
        }
        e.mem.set_u32(block.wrapping_add(0x10), u32::MAX);
    }
    e.mem.set_u32(block.wrapping_add(0x10), key);
    let magnitude = e.call(EFFECT_GET_MAGNITUDE, &args![effect]).u32();
    let setting = effect_setting(e, effect);
    let magnitude = if setting_flag(e, setting, 4) {
        magnitude.wrapping_neg()
    } else {
        magnitude
    };
    e.mem.set_u32(block, magnitude);
    if !setting_flag(e, setting, 2) {
        let duration = effect_duration(e, effect);
        e.mem.set_u32(block.wrapping_add(8), duration);
    }
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), block);
        e.call(LIST_APPEND, &args![this.addr().wrapping_add(4), slot]);
    });
}

// Typed from 007e0ce0
/// A status data list's first entry whose `+0x10` is `key`, or 0 (the walk stops at a node with no
/// entry).
pub fn fn_007e0ce0(e: &mut Engine, this: Ptr, key: u32) -> u32 {
    let mut node = first_node(this.addr());
    while node != 0 {
        let entry = e.mem.u32(node);
        if entry == 0 {
            break;
        }
        if e.mem.u32(entry.wrapping_add(0x10)) == key {
            return entry;
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    0
}

// Typed from 007e0e30
/// The callback (cdecl: the node's item and `value`) on a list node; while it answers true, the
/// same on the next node (`+4`) until there is none.
pub fn fn_007e0e30(e: &mut Engine, this: Ptr, callback: u32, value: u32) {
    let item = e.mem.u32(this.addr());
    if !e.call(callback, &args![item, value]).bool() {
        return;
    }
    let next = e.mem.u32(this.addr().wrapping_add(4));
    if next != 0 {
        typed_call!(
            super::FAMILY,
            0x007e_0e30,
            fn_007e0e30(e, Ptr::new(next), callback, value)
        );
    }
}

/// `ListBox<T>::AddItem` of the perk rank and misc stat instances (the template's body, as
/// the level-up menu's `ListBox<BGSPerk_P>` has it): with a container and a template (`template`,
/// else the list's), the template rendered into the container; the new tile's id -1 when it has
/// none and its string `text` when given; a new `ListBoxItem` of the tile and `data` (`007a62e0`,
/// in place) inserted by `compare` (then `Synchronize` with flag 0) or appended (then, with flag 0,
/// the tile's position and `SetTotalListHeight`; the tile numbered with the next index). With
/// exactly one item and a `"_number_of_visible_items"` above 0.0, the container's height becomes the
/// tile's height plus that number. The new tile, or 0.
fn add_item_of(
    e: &mut Engine,
    of: Of,
    this: Ptr,
    data: u32,
    text: u32,
    compare: u32,
    template: u32,
) -> u32 {
    let at = this.addr();
    let container = field(e, at, CONTAINER);
    let own_template = field(e, at, TEMPLATE_NAME);
    if container == 0 || (own_template == 0 && template == 0) {
        return 0;
    }
    let template = if template == 0 {
        own_template
    } else {
        template
    };
    let menu = e.call(TILE_GET_MENU, &args![container]).u32();
    let tile = e
        .call(RENDER_TEMPLATE, &args![menu, container, template, 0u32])
        .u32();
    let id = typed_call!(
        dispatch::FAMILY,
        0x00a0_0f30,
        dispatch::tile::get_value(e, Ptr::new(tile), TRAIT_ID)
    );
    if id.is_null() {
        tile_set_int(e, tile, TRAIT_ID, -1);
    }
    if text != 0 {
        set_text(e, tile, TRAIT_STRING, text);
    }
    let item = super::item_list_box::allocate_item(e, tile, data);
    let list = at.wrapping_add(4);
    if compare != 0 {
        e.call(LIST_INSERT, &args![list, item, compare]);
        if typed_call!(
            super::FAMILY,
            0x0072_42c0,
            menu_manager_p2::fn_007242c0(e, this, 0)
        ) {
            menu_manager_p2::synchronize_of(e, of, this);
        }
    } else {
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), item);
            e.call(LIST_APPEND_ITEM, &args![list, slot]);
        });
        if typed_call!(
            super::FAMILY,
            0x0072_42c0,
            menu_manager_p2::fn_007242c0(e, this, 0)
        ) {
            e.call(INIT_TILE_POSITION, &args![this, tile]);
            menu_manager_p2::set_total_list_height_of(e, of, this);
        }
        let index = e
            .mem
            .u16(at.wrapping_add(super::menu_manager_p2::NEXT_INDEX));
        e.mem.set_u16(
            at.wrapping_add(super::menu_manager_p2::NEXT_INDEX),
            index.wrapping_add(1),
        );
        tile_set_int(e, tile, TRAIT_INDEX, i32::from(index));
    }
    if typed_call!(
        super::FAMILY,
        0x0072_6c40,
        super::small_units::fn_00726c40(e, this)
    ) == 1
    {
        let visible = e
            .call(ADD_USER_TRAIT, &args![VISIBLE_ITEMS_TEXT, u32::MAX])
            .i32();
        let container = field(e, at, CONTAINER);
        // `FCOMP` with the double 0.0 and `TEST AH,0x41`: on only when strictly above.
        if f64::from(tile_float(e, container, visible)) > constant(e, ZERO) {
            let container = field(e, at, CONTAINER);
            let value = typed_call!(
                dispatch::FAMILY,
                0x00a0_0e90,
                dispatch::tile::get_value_q(e, Ptr::new(container), TRAIT_HEIGHT)
            );
            typed_call!(
                dispatch::FAMILY,
                0x00a0_9200,
                dispatch::tile::clear_actions(e, value)
            );
            e.call(
                VALUE_ADD_ACTION,
                &args![value, ACTION_COPY, tile, super::bits(TRAIT_HEIGHT)],
            );
            let visible = e
                .call(ADD_USER_TRAIT, &args![VISIBLE_ITEMS_TEXT, u32::MAX])
                .i32();
            let container = field(e, at, CONTAINER);
            let count = tile_float(e, container, visible);
            e.call(VALUE_ADD_FLOAT, &args![value, ACTION_ADD, count]);
            e.call(VALUE_CALCULATE, &args![value, 0u32]);
        }
    }
    tile
}

// Typed from 007e0e70
/// `ListBox<PerkRankData *>::AddItem`: the template's body ([`add_item_of`]) with its own
/// `Synchronize` and `SetTotalListHeight`.
pub fn fn_007e0e70(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    text: u32,
    compare: u32,
    template: u32,
) -> u32 {
    add_item_of(e, Of::PerkRankData, this, data, text, compare, template)
}

// Typed from 007e1110
/// The data of a list box's item whose tile is `tile`, or 0 (also for a null tile; the walk stops at
/// a node with no item).
pub fn fn_007e1110(e: &mut Engine, this: Ptr, tile: u32) -> u32 {
    if tile == 0 {
        return 0;
    }
    let mut node = first_node(this.addr());
    while node != 0 {
        let item = e.mem.u32(node);
        if item == 0 {
            break;
        }
        if e.mem.u32(item) == tile {
            return e.mem.u32(item.wrapping_add(4));
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    0
}

// Typed from 007e1190
/// `ListBox<enum MiscStatManager::MiscStatID>::AddItem`: the template's body ([`add_item_of`]) with
/// its own `Synchronize` and `SetTotalListHeight`.
pub fn list_box_misc_stat_id_add_item(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    text: u32,
    compare: u32,
    template: u32,
) -> u32 {
    add_item_of(e, Of::MiscStatId, this, data, text, compare, template)
}

/// The file's functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    use engine_core::entry;
    vec![
        entry!(0x0040_6de0, fn_00406de0(Ptr)),
        entry!(0x007d_b1c0, fn_007db1c0() -> u32),
        entry!(0x007d_b250, fn_007db250(Ptr, u32, u32)),
        entry!(0x007d_b270, fn_007db270(Ptr, u32, u32, u32)),
        entry!(0x007d_b2c0, fn_007db2c0(Ptr) -> Ptr),
        entry!(0x007d_b340, fn_007db340(Ptr) -> bool),
        entry!(0x007d_bfe0, fn_007dbfe0(Ptr, u32)),
        entry!(0x007d_c040, stats_menu_fade_control_callback(u32, u32) -> bool),
        entry!(0x007d_c0a0, fn_007dc0a0(Ptr) -> Ptr),
        entry!(0x007d_c4f0, stats_menu_do_idle(Ptr)),
        entry!(0x007d_cc00, fn_007dcc00(Ptr) -> i32),
        entry!(0x007d_cc40, fn_007dcc40(Ptr) -> i32),
        entry!(0x007d_cfc0, stats_menu_update_all_actor_values(Ptr)),
        entry!(0x007d_d090, stats_menu_update_actor_value(Ptr, u32)),
        entry!(0x007d_d710, stats_menu_update_perks()),
        entry!(0x007d_d8a0, fn_007dd8a0(Ptr) -> u8),
        entry!(0x007d_dad0, fn_007ddad0(Ptr, u32)),
        entry!(0x007d_db00, fn_007ddb00(Ptr)),
        entry!(0x007d_def0, fn_007ddef0() -> f32),
        entry!(0x007d_f1b0, fn_007df1b0(Ptr) -> bool),
        entry!(0x007d_f1d0, fn_007df1d0(Ptr) -> u32),
        entry!(0x007d_f210, fn_007df210(Ptr) -> u32),
        entry!(0x007d_f230, stats_menu_update_healing_options(Ptr, u32)),
        entry!(0x007d_f4e0, stats_menu_update_player_name()),
        entry!(0x007d_f5d0, stats_menu_update_player_title()),
        entry!(0x007d_f620, stats_menu_update_healing_mode(Ptr)),
        entry!(0x007d_fa30, stats_menu_special_update_fn(u32, u32)),
        entry!(0x007d_fb00, stats_menu_skills_update_fn(u32, u32)),
        entry!(0x007d_fc60, fn_007dfc60(u32, u32)),
        entry!(0x007d_fcd0, fn_007dfcd0() -> u8),
        entry!(0x007d_fdf0, fn_007dfdf0(Ptr, u32) -> u32),
        entry!(0x007d_fe40, fn_007dfe40(Ptr) -> i32),
        entry!(0x007d_ff40, stats_menu_set_current_page(Ptr, i32)),
        entry!(0x007e_04e0, fn_007e04e0(u32, u32) -> i32),
        entry!(0x007e_05f0, fn_007e05f0(Ptr) -> bool),
        entry!(0x007e_07a0, fn_007e07a0(Ptr) -> bool),
        entry!(0x007e_07f0, fn_007e07f0(Ptr)),
        entry!(0x007e_0850, fn_007e0850(Ptr)),
        entry!(0x007e_09b0, fn_007e09b0(Ptr)),
        entry!(0x007e_0b20, stats_menu_status_data_list_register(Ptr, u32)),
        entry!(0x007e_0ce0, fn_007e0ce0(Ptr, u32) -> u32),
        entry!(0x007e_0e30, fn_007e0e30(Ptr, u32, u32)),
        entry!(0x007e_0e70, fn_007e0e70(Ptr, u32, u32, u32, u32) -> u32),
        entry!(0x007e_1110, fn_007e1110(Ptr, u32) -> u32),
        entry!(
            0x007e_1190,
            list_box_misc_stat_id_add_item(Ptr, u32, u32, u32, u32) -> u32
        ),
    ]
}
