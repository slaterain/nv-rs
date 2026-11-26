//! The menu manager unit (the Xbox PDB's `fallout/interface/menumanager` source) from `00721060` to
//! its end (the translation is engine-fallout-interface's `menumanager.rs`, which these functions
//! replace at the same addresses): the six `ListBox<T>` instances the stats and VATS menus hold
//! (`ActorValue::Index`, `PerkRankData *`, `MiscStatManager::MiscStatID`,
//! `StatsMenu::StatusEffect *`, `TESReputation *`, `unsigned int`), each with its constructor,
//! destructor, `RemoveAll`, `Sort`, `Highlight`, `GetHighlightTarget`, `Synchronize`, `IsEnabled`,
//! `UpdateHighlightBoxPos`, `SetTotalListHeight`, `NumUnfiltered` and deleting destructor; the
//! `ListBoxBase` constructor, the list box collection's add and remove, the flag and tile tests;
//! and the `BSSimpleArray`s of the S.P.E.C.I.A.L. book's and love tester's pages (constructors,
//! destructors and deleting destructors). The word at +4 (`00726070`) is the core's leaf family's.
//!
//! **Native.** The six instances' code is the same apart from the vtable, the instance's own methods
//! and the function-local statics that keep trait ids, so each method has one body here over an
//! [`Of`], and a body calls the instance's own methods (as the exe does, directly) through
//! `typed_call!` with that instance's address. The calls of functions no family types stay calls by
//! address: the embedded list's constructor, destructor and sort, the
//! collection, a node's and an item's deletes, the free, the trait ids by name, the item tile's
//! position, the value actions and the update queue, the two roundings.
//! `Tile::Value::CalculateValue` (`00a09410`) is an `overridable.tsv` row: by address too. The
//! tile's trait getters, test, setters, value and children by name and path are the dispatch
//! family's, `_ftol2` the C runtime's, the simple array's set-up the pathing family's and its clear
//! the dispatch family's (all through `typed_call!`). A virtual call (an item tile's deleting
//! destructor, `GetTileAt`) stays virtual: its object's class is not known here. What the exe calls
//! that is a plain read or store is done in place ([`IN_PLACE`]; a list node's item and next and the
//! word at +4 are in the interface unit's lists), and so are three of this file's own functions,
//! a vtable store and the two collection calls the constructors and destructors make (`007240c0`,
//! `00721130`, `007211c0`, which the translation calls as plain functions too).

use crate::{crt, dispatch, pathing};
use engine_core::prelude::*;
use engine_core::typed_call;
use engine_core::x87::fistp_chop64;

/// What this file does in place besides the interface unit's lists ([`super::interface_cpp`]'s
/// `in_place::ADDRESSES`, [`super::interface_cpp_p2::IN_PLACE`]): plain reads and stores the exe
/// calls (the differential tests leave these out of the translation's calls).
pub const IN_PLACE: &[u32] = &[
    0x006e_cd40, // a list node's next stored (its +4)
    0x0064_7b70, // the larger of two integers (signed)
    0x007a_1950, // a list box flag (usFlags bit) set or cleared
];

/// The allocator's free (cdecl).
pub(super) const FREE: u32 = 0x0040_1030;
/// The embedded list's destructor and sort (thiscall on the list box's +4; its constructor, two
/// zero words, is done in place).
const LIST_DESTRUCT: u32 = 0x0046_ffb0;
const LIST_SORT: u32 = 0x0083_fd60;
/// A list node's deleting destructor (thiscall, flags) and a `ListBoxItem`'s data delete
/// (`004459e0`, thiscall with 1).
pub(super) const NODE_DELETE: u32 = 0x0047_02f0;
pub(super) const ITEM_DELETE: u32 = 0x0044_59e0;
/// `ListBoxBase`'s vtable (RTTI `.?AVListBoxBase@@`).
const LIST_BOX_BASE_VTABLE: u32 = 0x0107_0418;
/// The object every list box is added to (`005ae3d0`) and removed from (`00905330`), each given
/// the address of a word holding the list box (thiscall).
const LIST_BOX_COLLECTION: u32 = 0x011d_8b54;
const LIST_BOX_COLLECTION_ADD: u32 = 0x005a_e3d0;
const LIST_BOX_COLLECTION_REMOVE: u32 = 0x0090_5330;
/// While this byte is set, `RemoveAll` leaves the items' tiles alone.
const KEEP_ITEM_TILES: u32 = 0x011d_8909;
/// `Tile::TextToTrait` (a trait's id by name, cdecl) and `Tile::AddUserTrait` (cdecl: name, -1).
pub(super) const TEXT_TO_TRAIT: u32 = 0x00a0_1860;
pub(super) const ADD_USER_TRAIT: u32 = 0x00a0_0940;
/// The trait names: `"_num_filtered"`, `"_scroll_delta"`, `"_enabled"`, `"_highlight_y"`,
/// `"_selected_height"`, `"_number_of_items"`.
pub(super) const NUM_FILTERED_TEXT: u32 = 0x0106_f8d0;
pub(super) const SCROLL_DELTA_TEXT: u32 = 0x0106_f90c;
const ENABLED_TEXT: u32 = 0x0107_0244;
pub(super) const HIGHLIGHT_Y_TEXT: u32 = 0x0106_f8c0;
pub(super) const SELECTED_HEIGHT_TEXT: u32 = 0x0106_f8ac;
pub(super) const NUMBER_OF_ITEMS_TEXT: u32 = 0x0106_f8f8;
/// `"lb_highlight_box"` and its edges' names, `"top"`, `"bottom"`, `"left"`, `"right"`.
pub(super) const HIGHLIGHT_BOX_TEXT: u32 = 0x0106_f898;
pub(super) const TOP_TEXT: u32 = 0x0106_f894;
pub(super) const EDGE_TEXTS: [u32; 3] = [0x0106_f88c, 0x0106_f884, 0x0106_f87c];
/// The traits read by id: an item tile's index, a tile's height.
pub(super) const TRAIT_INDEX: i32 = 0xfac;
pub(super) const TRAIT_HEIGHT: i32 = 0xfb0;
/// The highlighted tile's traits the highlight box follows, and the action it adds.
pub(super) const TRAIT_Y: u32 = 0xfa2;
pub(super) const ACTION_COPY: u32 = 0x7d0;
/// `Tile::Value::AddAction` (thiscall: action, tile, trait) and `Tile::Value::CalculateValue`
/// (thiscall, a word; an `overridable.tsv` row).
pub(super) const VALUE_ADD_ACTION: u32 = 0x00a0_9130;
pub(super) const VALUE_CALCULATE: u32 = 0x00a0_9410;
/// `Tile::AddNeedsUpdate` (thiscall, flags).
const TILE_ADD_NEEDS_UPDATE: u32 = 0x00a0_7530;
/// A tile's byte at +0x35 (`0071aa80` reads it, `0071aa60` stores it).
pub(super) const TILE_BYTE_35: u32 = 0x35;
/// `ListBox<StartMenu::Option *>::InitTilePosition` (thiscall on the list box: the tile).
pub(super) const INIT_TILE_POSITION: u32 = 0x0072_69d0;
/// The page step's rounding (`004bd510`, cdecl: the step, 1.0) and `SetTotalListHeight`'s
/// (`004e4430`, cdecl: a float), each a float in `ST0`.
const SCROLL_ROUND: u32 = 0x004b_d510;
pub(super) const ROUND_FLOAT: u32 = 0x004e_4430;

/// `ListBox` (Xbox PDB): its embedded list (`+4` the first node's item, `+8` its next) and fields.
pub(super) const FIRST_ITEM: u32 = 0x04;
pub(super) const SECOND_NODE: u32 = 0x08;
pub(super) const CONTAINER: u32 = 0x0c;
pub(super) const HIGHLIGHTED: u32 = 0x10;
pub(super) const SCROLL_BAR: u32 = 0x14;
pub(super) const TEMPLATE_NAME: u32 = 0x18;
pub(super) const NEXT_INDEX: u32 = 0x1c;
pub(super) const TOTAL_HEIGHT: u32 = 0x20;
const SAVED_HIGHLIGHT_INDEX: u32 = 0x24;
const SAVED_SCROLLBAR_VALUE: u32 = 0x28;
const FLAGS: u32 = 0x2c;
/// A `ListBoxItem`'s (Xbox PDB) data (`+4`) and filtered byte (`+8`); its tile is its first word.
pub(super) const ITEM_DATA: u32 = 4;
pub(super) const ITEM_FILTERED: u32 = 8;

/// The `BSSimpleArray` vtables (RTTI): of `SPECIALBookMenu::SpecialBookTextItem`,
/// `SpecialBookButton`, `LoveTesterMenu::LoveTesterTextItem`, `LoveTesterLightBulbGlows` and
/// `LoveTesterButton` pointers.
const BOOK_TEXT_ITEMS_VTABLE: u32 = 0x0107_03b4;
const BOOK_BUTTONS_VTABLE: u32 = 0x0107_03c8;
const LOVE_TESTER_TEXT_ITEMS_VTABLE: u32 = 0x0107_03dc;
const LOVE_TESTER_GLOWS_VTABLE: u32 = 0x0107_03f0;
const LOVE_TESTER_BUTTONS_VTABLE: u32 = 0x0107_0404;

/// One of the six `ListBox<T>` instances.
#[derive(Clone, Copy)]
pub(super) enum Of {
    ActorValue,
    PerkRankData,
    MiscStatId,
    StatusEffect,
    Reputation,
    UnsignedInt,
}

impl Of {
    /// The instance's vtable (RTTI `.?AV?$ListBox@...`) and its methods' statics' guard words:
    /// `IsEnabled`'s (its `"_enabled"` id just below), `UpdateHighlightBoxPos`'s (bit 0 the
    /// `"_highlight_y"` id 4 bytes below, bit 1 the `"_selected_height"` id 8 below) and
    /// `NumUnfiltered`'s (its `"_num_filtered"` id just below), as each method's code has them.
    const fn layout(self) -> (u32, [u32; 3]) {
        match self {
            Of::ActorValue => (0x0107_0220, [0x011d_8dd0, 0x011d_8ddc, 0x011d_8e48]),
            Of::PerkRankData => (0x0107_02ec, [0x011d_8de4, 0x011d_8df0, 0x011d_8e50]),
            Of::MiscStatId => (0x0107_0314, [0x011d_8df8, 0x011d_8e04, 0x011d_8e58]),
            Of::StatusEffect => (0x0107_033c, [0x011d_8e0c, 0x011d_8e18, 0x011d_8e60]),
            Of::Reputation => (0x0107_0364, [0x011d_8e20, 0x011d_8e2c, 0x011d_8e68]),
            Of::UnsignedInt => (0x0107_038c, [0x011d_8e34, 0x011d_8e40, 0x011d_8e70]),
        }
    }

    const fn vtable(self) -> u32 {
        self.layout().0
    }

    const fn guards(self) -> [u32; 3] {
        self.layout().1
    }
}

/// The instance's `RemoveAll`.
fn remove_all_of(e: &mut Engine, of: Of, this: Ptr) {
    match of {
        Of::ActorValue => typed_call!(
            super::FAMILY,
            0x0072_11e0,
            list_box_actor_value_remove_all(e, this)
        ),
        Of::PerkRankData => typed_call!(
            super::FAMILY,
            0x0072_1990,
            list_box_perk_rank_data_remove_all(e, this)
        ),
        Of::MiscStatId => typed_call!(
            super::FAMILY,
            0x0072_2140,
            list_box_misc_stat_id_remove_all(e, this)
        ),
        Of::StatusEffect => typed_call!(
            super::FAMILY,
            0x0072_28f0,
            list_box_status_effect_remove_all(e, this)
        ),
        Of::Reputation => typed_call!(
            super::FAMILY,
            0x0072_30e0,
            list_box_tes_reputation_remove_all(e, this)
        ),
        Of::UnsignedInt => typed_call!(
            super::FAMILY,
            0x0072_3890,
            list_box_unsigned_int_remove_all(e, this)
        ),
    }
}

/// The instance's `Synchronize`.
pub(super) fn synchronize_of(e: &mut Engine, of: Of, this: Ptr) {
    match of {
        Of::ActorValue => typed_call!(
            super::FAMILY,
            0x0072_4360,
            list_box_actor_value_synchronize(e, this)
        ),
        Of::PerkRankData => typed_call!(
            super::FAMILY,
            0x0072_4930,
            list_box_perk_rank_data_synchronize(e, this)
        ),
        Of::MiscStatId => typed_call!(
            super::FAMILY,
            0x0072_4f00,
            list_box_misc_stat_id_synchronize(e, this)
        ),
        Of::StatusEffect => typed_call!(
            super::FAMILY,
            0x0072_54d0,
            list_box_status_effect_synchronize(e, this)
        ),
        Of::Reputation => typed_call!(
            super::FAMILY,
            0x0072_5aa0,
            list_box_tes_reputation_synchronize(e, this)
        ),
        Of::UnsignedInt => typed_call!(
            super::FAMILY,
            0x0072_6090,
            list_box_unsigned_int_synchronize(e, this)
        ),
    }
}

/// The instance's `IsEnabled`.
fn is_enabled_of(e: &mut Engine, of: Of, this: Ptr) -> bool {
    match of {
        Of::ActorValue => typed_call!(
            super::FAMILY,
            0x0072_4430,
            list_box_actor_value_is_enabled(e, this)
        ),
        Of::PerkRankData => typed_call!(
            super::FAMILY,
            0x0072_4a00,
            list_box_perk_rank_data_is_enabled(e, this)
        ),
        Of::MiscStatId => typed_call!(
            super::FAMILY,
            0x0072_4fd0,
            list_box_misc_stat_id_is_enabled(e, this)
        ),
        Of::StatusEffect => typed_call!(
            super::FAMILY,
            0x0072_55a0,
            list_box_status_effect_is_enabled(e, this)
        ),
        Of::Reputation => typed_call!(
            super::FAMILY,
            0x0072_5b70,
            list_box_tes_reputation_is_enabled(e, this)
        ),
        Of::UnsignedInt => typed_call!(
            super::FAMILY,
            0x0072_6160,
            list_box_unsigned_int_is_enabled(e, this)
        ),
    }
}

/// The instance's `UpdateHighlightBoxPos`.
fn update_highlight_box_pos_of(e: &mut Engine, of: Of, this: Ptr) {
    match of {
        Of::ActorValue => typed_call!(
            super::FAMILY,
            0x0072_44e0,
            list_box_actor_value_update_highlight_box_pos(e, this)
        ),
        Of::PerkRankData => typed_call!(
            super::FAMILY,
            0x0072_4ab0,
            list_box_perk_rank_data_update_highlight_box_pos(e, this)
        ),
        Of::MiscStatId => typed_call!(
            super::FAMILY,
            0x0072_5080,
            list_box_misc_stat_id_update_highlight_box_pos(e, this)
        ),
        Of::StatusEffect => typed_call!(
            super::FAMILY,
            0x0072_5650,
            list_box_status_effect_update_highlight_box_pos(e, this)
        ),
        Of::Reputation => typed_call!(
            super::FAMILY,
            0x0072_5c20,
            list_box_tes_reputation_update_highlight_box_pos(e, this)
        ),
        Of::UnsignedInt => typed_call!(
            super::FAMILY,
            0x0072_6210,
            list_box_unsigned_int_update_highlight_box_pos(e, this)
        ),
    }
}

/// The instance's `SetTotalListHeight`.
pub(super) fn set_total_list_height_of(e: &mut Engine, of: Of, this: Ptr) {
    match of {
        Of::ActorValue => typed_call!(
            super::FAMILY,
            0x0072_4800,
            list_box_actor_value_set_total_list_height(e, this)
        ),
        Of::PerkRankData => typed_call!(
            super::FAMILY,
            0x0072_4dd0,
            list_box_perk_rank_data_set_total_list_height(e, this)
        ),
        Of::MiscStatId => typed_call!(
            super::FAMILY,
            0x0072_53a0,
            list_box_misc_stat_id_set_total_list_height(e, this)
        ),
        Of::StatusEffect => typed_call!(
            super::FAMILY,
            0x0072_5970,
            list_box_status_effect_set_total_list_height(e, this)
        ),
        Of::Reputation => typed_call!(
            super::FAMILY,
            0x0072_5f40,
            list_box_tes_reputation_set_total_list_height(e, this)
        ),
        Of::UnsignedInt => typed_call!(
            super::FAMILY,
            0x0072_6530,
            list_box_unsigned_int_set_total_list_height(e, this)
        ),
    }
}

/// The instance's `NumUnfiltered`.
fn num_unfiltered_of(e: &mut Engine, of: Of, this: Ptr) -> i32 {
    match of {
        Of::ActorValue => typed_call!(
            super::FAMILY,
            0x0072_6660,
            list_box_actor_value_num_unfiltered(e, this)
        ),
        Of::PerkRankData => typed_call!(
            super::FAMILY,
            0x0072_6710,
            list_box_perk_rank_data_num_unfiltered(e, this)
        ),
        Of::MiscStatId => typed_call!(
            super::FAMILY,
            0x0072_67c0,
            list_box_misc_stat_id_num_unfiltered(e, this)
        ),
        Of::StatusEffect => typed_call!(
            super::FAMILY,
            0x0072_6870,
            list_box_status_effect_num_unfiltered(e, this)
        ),
        Of::Reputation => typed_call!(
            super::FAMILY,
            0x0072_6920,
            list_box_tes_reputation_num_unfiltered(e, this)
        ),
        Of::UnsignedInt => typed_call!(
            super::FAMILY,
            0x0072_6b40,
            list_box_unsigned_int_num_unfiltered(e, this)
        ),
    }
}

/// Frees `this` when bit 0 of `flags` is set (the deleting destructors' tail).
#[inline]
fn free_if_flagged(e: &mut Engine, this: Ptr, flags: u32) {
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
}

/// The list box collection's add or remove (`00721130`, `007211c0`): `method` given the address of
/// a stack word holding `list_box`.
#[inline]
fn collection(e: &mut Engine, method: u32, list_box: u32) {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), list_box);
        e.call(method, &args![LIST_BOX_COLLECTION, slot]);
    });
}

/// A tile's float trait (`Tile::GetFloat`, the dispatch family's).
#[inline]
pub(super) fn tile_float(e: &mut Engine, tile: u32, id: i32) -> f32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_11b0,
        dispatch::tile::get_float(e, Ptr::new(tile), id)
    )
}

/// `Tile::SetInt` (the dispatch family's).
#[inline]
pub(super) fn tile_set_int(e: &mut Engine, tile: u32, id: i32, value: i32) {
    typed_call!(
        dispatch::FAMILY,
        0x0070_0320,
        dispatch::tile::set_int(e, Ptr::new(tile), id, value)
    );
}

/// `Tile::SetFloat` with the flag 1 (the dispatch family's).
#[inline]
pub(super) fn tile_set_float(e: &mut Engine, tile: u32, id: i32, value: f32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_12d0,
        dispatch::tile::set_float(e, Ptr::new(tile), id, value, 1)
    );
}

/// `_ftol2` of `ST0` (the C runtime family's): its `EAX`.
#[inline]
pub(super) fn ftol2(e: &mut Engine, value: f64) -> i32 {
    super::low_dword(typed_call!(
        crt::FAMILY,
        0x00ec_62c0,
        crt::math::ftol2_sse(e, value)
    )) as i32
}

/// A trait's id by name (`Tile::TextToTrait`, by address).
#[inline]
pub(super) fn trait_by_name(e: &mut Engine, name: u32) -> i32 {
    e.call(TEXT_TO_TRAIT, &args![name]).i32()
}

/// `Tile::IsTrue(tile, Tile::AddUserTrait("_enabled", -1))`.
fn tile_enabled(e: &mut Engine, tile: u32) -> bool {
    let enabled = e.call(ADD_USER_TRAIT, &args![ENABLED_TEXT, u32::MAX]).i32();
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1230,
        dispatch::tile::is_true(e, Ptr::new(tile), enabled)
    )
}

/// A function-local static trait id: the first time (bit `bit` of the word at `guard` clear) the
/// bit is set, then the id `Tile::AddUserTrait(name, -1)` answers is kept at `cache`. The id.
#[inline]
pub(super) fn static_trait(e: &mut Engine, guard: u32, bit: u32, cache: u32, name: u32) -> i32 {
    let word = e.mem.u32(guard);
    if word & bit != 0 {
        return e.mem.i32(cache);
    }
    e.mem.set_u32(guard, word | bit);
    let id = e.call(ADD_USER_TRAIT, &args![name, u32::MAX]).u32();
    e.mem.set_u32(cache, id);
    id as i32
}

/// The first node of the embedded list (`this + 4`; the code guards a null `this`).
#[inline]
pub(super) fn first_node(this: u32) -> u32 {
    if this == 0 {
        0
    } else {
        this.wrapping_add(FIRST_ITEM)
    }
}

/// The list box's flag `bit` (`usFlags`, `+0x2c`) set or cleared (`007a1950`, thiscall: the bit,
/// a byte; in place): `1 << bit` (the shift count masked to 5 bits, as `SHL` does) or-ed in, or its
/// complement and-ed, and the low 16 bits stored.
#[inline]
pub(super) fn set_flag(e: &mut Engine, at: u32, bit: u32, set: bool) {
    let flags = e.mem.u16(at.wrapping_add(FLAGS));
    // The mask's low 16 bits: none for a shift of 16 to 31.
    let mask = match bit & 0x1f {
        shift @ 0..=15 => 1u16 << shift,
        _ => 0,
    };
    let flags = if set { flags | mask } else { flags & !mask };
    e.mem.set_u16(at.wrapping_add(FLAGS), flags);
}

/// `ListBox<T>::ListBox` (Xbox PDB): the `ListBoxBase` vtable (`007240c0`), the embedded list, the
/// instance's vtable, the fields cleared, the flag 0 set (`007a1950` with 0 and 1, in place), the
/// list box added to the collection (`00721130`). `this`.
fn construct(e: &mut Engine, of: Of, this: Ptr) -> Ptr {
    let at = this.addr();
    e.mem.set_u32(at, LIST_BOX_BASE_VTABLE);
    super::menu_manager::empty_list(e, at.wrapping_add(4));
    e.mem.set_u32(at, of.vtable());
    e.mem.set_u32(at.wrapping_add(SCROLL_BAR), 0);
    e.mem.set_u32(at.wrapping_add(HIGHLIGHTED), 0);
    e.mem.set_u32(at.wrapping_add(CONTAINER), 0);
    e.mem.set_u32(at.wrapping_add(TEMPLATE_NAME), 0);
    e.mem.set_u16(at.wrapping_add(FLAGS), 0);
    e.mem.set_u16(at.wrapping_add(NEXT_INDEX), 0);
    e.mem.set_f32(at.wrapping_add(SAVED_SCROLLBAR_VALUE), 0.0);
    e.mem.set_f32(at.wrapping_add(SAVED_HIGHLIGHT_INDEX), 0.0);
    e.mem.set_f32(at.wrapping_add(TOTAL_HEIGHT), 0.0);
    set_flag(e, at, 0, true);
    collection(e, LIST_BOX_COLLECTION_ADD, at);
    this
}

/// `ListBox<T>::~ListBox` (Xbox PDB): the vtable, `RemoveAll`, the list box removed from the
/// collection (`007211c0`), the embedded list destroyed.
fn destruct(e: &mut Engine, of: Of, this: Ptr) {
    let at = this.addr();
    e.mem.set_u32(at, of.vtable());
    remove_all_of(e, of, this);
    collection(e, LIST_BOX_COLLECTION_REMOVE, at);
    e.call(LIST_DESTRUCT, &args![at.wrapping_add(4)]);
}

/// The item `RemoveAll` deletes: the second node's (through the node the list holds at `+8` now) or
/// the first's (`+4`).
#[inline]
fn item_of(e: &Engine, at: u32, second: bool) -> u32 {
    if second {
        e.mem.u32(e.mem.u32(at.wrapping_add(SECOND_NODE)))
    } else {
        e.mem.u32(at.wrapping_add(FIRST_ITEM))
    }
}

/// Deletes a `ListBoxItem` (`pTile` +0, `data` +4, Xbox PDB) of the list ([`item_of`]): the tile
/// through its deleting destructor (slot 0, flag 1) unless the tiles are kept, the data
/// (`004459e0` with 1) when the list's flag bit 1 is set, then the item freed. `item` is the item
/// as the code last read it; after a call the item is read again, as the code does.
fn delete_item(e: &mut Engine, this: Ptr, second: bool, item: u32) {
    let at = this.addr();
    if e.mem.u8(KEEP_ITEM_TILES) == 0 {
        let tile = e.mem.u32(item);
        if tile != 0 {
            e.vcall(tile, 0, &args![1u32]);
        }
    }
    if typed_call!(super::FAMILY, 0x0072_42c0, fn_007242c0(e, this, 1)) {
        let data = e.mem.u32(item_of(e, at, second).wrapping_add(ITEM_DATA));
        if data != 0 {
            e.call(ITEM_DELETE, &args![data, 1u32]);
        }
    }
    let item = item_of(e, at, second);
    e.call(FREE, &args![item]);
}

/// `ListBox<T>::RemoveAll` (Xbox PDB): while there is a second node with an item, its item deleted,
/// the node's next cleared (`006ecd40`), the node deleted and the list pointing at the following
/// node; then the first item deleted; the first item, `usNextIndex` and `fTotalListHeight`
/// cleared, the container's `"_num_filtered"` set to 0, the height recomputed, no item highlighted
/// and the highlight box moved. A field the code reads again after a call is read again.
fn remove_all(e: &mut Engine, of: Of, this: Ptr) {
    let at = this.addr();
    let second = at.wrapping_add(SECOND_NODE);
    loop {
        let node = e.mem.u32(second);
        if node == 0 {
            break;
        }
        let item = e.mem.u32(node);
        if item == 0 {
            break;
        }
        let next = e.mem.u32(node.wrapping_add(4));
        delete_item(e, this, true, item);
        let node = e.mem.u32(second);
        e.mem.set_u32(node.wrapping_add(4), 0);
        if node != 0 {
            e.call(NODE_DELETE, &args![node, 1u32]);
        }
        e.mem.set_u32(second, next);
    }
    let first = at.wrapping_add(FIRST_ITEM);
    let item = e.mem.u32(first);
    if item != 0 {
        delete_item(e, this, false, item);
    }
    e.mem.set_u32(first, 0);
    e.mem.set_u16(at.wrapping_add(NEXT_INDEX), 0);
    e.mem.set_f32(at.wrapping_add(TOTAL_HEIGHT), 0.0);
    if e.mem.u32(at.wrapping_add(CONTAINER)) != 0 {
        let num_filtered = trait_by_name(e, NUM_FILTERED_TEXT);
        let container = e.mem.u32(at.wrapping_add(CONTAINER));
        tile_set_float(e, container, num_filtered, 0.0);
    }
    set_total_list_height_of(e, of, this);
    e.mem.set_u32(at.wrapping_add(HIGHLIGHTED), 0);
    update_highlight_box_pos_of(e, of, this);
}

/// `ListBox<T>::Sort` (Xbox PDB): the embedded list sorted with `compare`, then `Synchronize`.
fn sort(e: &mut Engine, of: Of, this: Ptr, compare: u32) {
    e.call(LIST_SORT, &args![this.addr().wrapping_add(4), compare]);
    synchronize_of(e, of, this);
}

/// `ListBox<T>::Highlight` (Xbox PDB): with the list enabled and `tile` null, or one of the list's
/// (`007242f0`) and `"_enabled"`, `tile` becomes the highlighted item, the highlight box moves and
/// the answer is true; otherwise false.
fn highlight(e: &mut Engine, of: Of, this: Ptr, tile: u32) -> bool {
    if !is_enabled_of(e, of, this) {
        return false;
    }
    if tile != 0
        && !(typed_call!(super::FAMILY, 0x0072_42f0, fn_007242f0(e, this, tile))
            && tile_enabled(e, tile))
    {
        return false;
    }
    e.mem.set_u32(this.addr().wrapping_add(HIGHLIGHTED), tile);
    update_highlight_box_pos_of(e, of, this);
    true
}

/// `GetHighlightTarget`'s page step: 0 without a container, else the container's height (negated
/// for a page up) over its `"_scroll_delta"`, a float, rounded (`004bd510` with 1.0) and taken to
/// an integer by `_ftol2`.
fn page_step(e: &mut Engine, this: Ptr, up: bool) -> i32 {
    let at = this.addr();
    let container = e.mem.u32(at.wrapping_add(CONTAINER));
    if container == 0 {
        return 0;
    }
    let extent = tile_float(e, container, TRAIT_HEIGHT);
    let numerator = if up { -extent } else { extent };
    let scroll_delta = trait_by_name(e, SCROLL_DELTA_TEXT);
    let container = e.mem.u32(at.wrapping_add(CONTAINER));
    let delta = tile_float(e, container, scroll_delta);
    let step = numerator / delta;
    let rounded = e.call(SCROLL_ROUND, &args![step, 1.0f32]).f64();
    ftol2(e, rounded)
}

/// `ListBox<T>::GetHighlightTarget` (Xbox PDB): the tile a move in `direction` highlights (1 up, 2
/// down, 0xF a page up, 0x10 a page down; anything else does not move), or 0 when there is none or
/// it is the one already highlighted. From the highlighted item's index moved by the step (kept in
/// the list), or the first or last item without one, the item tile with that index (trait `0xfac`)
/// is the answer when it is `"_enabled"`; else the search goes on one further, and past the end
/// (or before the start) walks back (or on) through `GetTileAt` (vtable slot +0x14) to an enabled
/// tile.
fn get_highlight_target(e: &mut Engine, of: Of, this: Ptr, direction: u32) -> u32 {
    let at = this.addr();
    let step = match direction {
        1 => -1,
        2 => 1,
        0xf => page_step(e, this, true),
        0x10 => page_step(e, this, false),
        _ => 0,
    };
    let mut target = 0u32;
    let count = if is_enabled_of(e, of, this) && step != 0 {
        e.mem.u16(at.wrapping_add(NEXT_INDEX))
    } else {
        0
    };
    if count != 0 {
        let last = i32::from(count) - 1;
        let highlighted = e.mem.u32(at.wrapping_add(HIGHLIGHTED));
        let mut index = if highlighted != 0 {
            let value = tile_float(e, highlighted, TRAIT_INDEX);
            let moved = ftol2(e, f64::from(value)).wrapping_add(step);
            if moved < 0 {
                0
            } else if moved > last {
                last
            } else {
                moved
            }
        } else if step > 0 {
            0
        } else {
            last
        };
        'search: loop {
            // The item tile whose index trait equals `index`.
            let mut node = first_node(at);
            while target == 0 && node != 0 {
                let item = e.mem.u32(node);
                if item == 0 {
                    break;
                }
                let tile = e.mem.u32(item);
                if f64::from(index) == f64::from(tile_float(e, tile, TRAIT_INDEX)) {
                    target = tile;
                }
                node = e.mem.u32(node.wrapping_add(4));
            }
            if target == 0 || tile_enabled(e, target) {
                break;
            }
            target = 0;
            if step > 0 {
                index = index.wrapping_add(1);
                if index > last {
                    // Past the end: back from the one before the last.
                    index = last - 1;
                    loop {
                        if target != 0 && tile_enabled(e, target) {
                            break 'search;
                        }
                        if index < 0 {
                            break 'search;
                        }
                        target = e.vcall(at, 0x14, &args![index, 0u32]).u32();
                        index -= 1;
                    }
                }
            } else if step < 0 {
                index = index.wrapping_sub(1);
                if index < 0 {
                    // Before the start: on from the second.
                    index = 1;
                    loop {
                        if target != 0 && tile_enabled(e, target) {
                            break 'search;
                        }
                        if index > last {
                            break 'search;
                        }
                        target = e.vcall(at, 0x14, &args![index, 0u32]).u32();
                        index += 1;
                    }
                }
            }
        }
    }
    if target == e.mem.u32(at.wrapping_add(HIGHLIGHTED)) {
        0
    } else {
        target
    }
}

/// `ListBox<T>::Synchronize` (Xbox PDB): `usNextIndex` and `fTotalListHeight` cleared, then each
/// item not filtered (its byte at +8 clear) gives its tile the next index (trait `0xfac`) and its
/// position (`InitTilePosition`); then `SetTotalListHeight` and `UpdateHighlightBoxPos`.
fn synchronize(e: &mut Engine, of: Of, this: Ptr) {
    let at = this.addr();
    e.mem.set_u16(at.wrapping_add(NEXT_INDEX), 0);
    e.mem.set_f32(at.wrapping_add(TOTAL_HEIGHT), 0.0);
    let mut node = first_node(at);
    while node != 0 {
        let item = e.mem.u32(node);
        if item == 0 {
            break;
        }
        if e.mem.u8(item.wrapping_add(ITEM_FILTERED)) == 0 {
            let index = e.mem.u16(at.wrapping_add(NEXT_INDEX));
            e.mem
                .set_u16(at.wrapping_add(NEXT_INDEX), index.wrapping_add(1));
            let tile = e.mem.u32(item);
            tile_set_int(e, tile, TRAIT_INDEX, i32::from(index));
            let tile = e.mem.u32(e.mem.u32(node));
            e.call(INIT_TILE_POSITION, &args![this, tile]);
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    set_total_list_height_of(e, of, this);
    update_highlight_box_pos_of(e, of, this);
}

/// `ListBox<T>::IsEnabled` (Xbox PDB): there is a container and its `"_enabled"` trait (the id a
/// function-local static) is true.
fn is_enabled(e: &mut Engine, of: Of, this: Ptr) -> bool {
    let guard = of.guards()[0];
    let enabled = static_trait(e, guard, 1, guard.wrapping_sub(4), ENABLED_TEXT);
    let container = e.mem.u32(this.addr().wrapping_add(CONTAINER));
    container != 0
        && typed_call!(
            dispatch::FAMILY,
            0x00a0_1230,
            dispatch::tile::is_true(e, Ptr::new(container), enabled)
        )
}

/// The tile's byte at +0x35 set to `flag` (`0071aa60`, the family's) and the tile queued for an
/// update (8).
#[inline]
pub(super) fn mark_tile(e: &mut Engine, tile: u32, flag: u8) {
    typed_call!(
        super::FAMILY,
        0x0071_aa60,
        super::item_list_box::fn_0071aa60(e, Ptr::new(tile), flag)
    );
    e.call(TILE_ADD_NEEDS_UPDATE, &args![tile, 8u32]);
}

/// `ListBox<T>::UpdateHighlightBoxPos` (Xbox PDB): the `"_highlight_y"` and `"_selected_height"` ids
/// (statics); with a container and no highlighted item, those traits set to -1 and 0; with one, the
/// two values follow the highlighted tile's `0xfa2` and `0xfb0` (an action `0x7d0`, then
/// calculated) and the container's `"lb_highlight_box"` tile, its `"top"` and, when `"top"` exists,
/// its `"bottom"`, `"left"` and `"right"` (not checked for null), take the highlighted tile's byte
/// at +0x35 (as 1 or 0) and an update.
fn update_highlight_box_pos(e: &mut Engine, of: Of, this: Ptr) {
    let at = this.addr();
    let guard = of.guards()[1];
    let y_trait = static_trait(e, guard, 1, guard.wrapping_sub(4), HIGHLIGHT_Y_TEXT);
    let height_trait = static_trait(e, guard, 2, guard.wrapping_sub(8), SELECTED_HEIGHT_TEXT);
    let container = e.mem.u32(at.wrapping_add(CONTAINER));
    if container == 0 {
        return;
    }
    let highlighted = e.mem.u32(at.wrapping_add(HIGHLIGHTED));
    if highlighted == 0 {
        tile_set_int(e, container, y_trait, -1);
        let container = e.mem.u32(at.wrapping_add(CONTAINER));
        tile_set_int(e, container, height_trait, 0);
        return;
    }
    let y_value = typed_call!(
        dispatch::FAMILY,
        0x00a0_0f30,
        dispatch::tile::get_value(e, Ptr::new(container), y_trait)
    );
    let height_value = typed_call!(
        dispatch::FAMILY,
        0x00a0_0f30,
        dispatch::tile::get_value(e, Ptr::new(container), height_trait)
    );
    for (value, from) in [(y_value, TRAIT_Y), (height_value, 0xfb0)] {
        let highlighted = e.mem.u32(at.wrapping_add(HIGHLIGHTED));
        e.call(
            VALUE_ADD_ACTION,
            &args![value, ACTION_COPY, highlighted, from],
        );
        e.call(VALUE_CALCULATE, &args![value, 0u32]);
    }
    let highlighted = e.mem.u32(at.wrapping_add(HIGHLIGHTED));
    let flag = u8::from(
        typed_call!(
            super::FAMILY,
            0x0071_aa80,
            super::item_list_box::fn_0071aa80(e, Ptr::new(highlighted))
        ) != 0,
    );
    let container = e.mem.u32(at.wrapping_add(CONTAINER));
    let box_tile = typed_call!(
        dispatch::FAMILY,
        0x00a0_8b20,
        dispatch::tile::find_by_path(e, Ptr::new(container), Ptr::new(HIGHLIGHT_BOX_TEXT))
    )
    .addr();
    if box_tile == 0 {
        return;
    }
    let top = child_by_name(e, box_tile, TOP_TEXT);
    if top != 0 {
        mark_tile(e, top, flag);
        for name in EDGE_TEXTS {
            let edge = child_by_name(e, box_tile, name);
            mark_tile(e, edge, flag);
        }
    }
    mark_tile(e, box_tile, flag);
}

/// `Tile::GetChildByName` (the dispatch family's).
#[inline]
pub(super) fn child_by_name(e: &mut Engine, tile: u32, name: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_3da0,
        dispatch::tile::child_by_name(e, Ptr::new(tile), Ptr::new(name))
    )
    .addr()
}

/// `ListBox<T>::SetTotalListHeight` (Xbox PDB): with a container, its visible height (trait
/// `0xfb0`, a float) and the total list height decide the number of scroll steps: 1, unless both
/// are above 0.0 and the visible height is below the total, then
/// `max(_ftol2(004e4430(unfiltered * (total - visible) / total)) + 1, 2)` (the ratio a float); with
/// both above 0.0 and more than one step, the container's `"_scroll_delta"` becomes
/// `(total - visible) / (steps - 1)`; with a scroll bar its `"_number_of_items"` becomes the steps.
/// Each step rounds to a float (the main thread's precision): the product of the whole count is
/// exact in an `f64` and rounded once, as is the quotient by the whole number of steps.
fn set_total_list_height(e: &mut Engine, of: Of, this: Ptr) {
    let at = this.addr();
    let container = e.mem.u32(at.wrapping_add(CONTAINER));
    if container == 0 {
        return;
    }
    let visible = tile_float(e, container, TRAIT_HEIGHT);
    let mut steps = 1i32;
    let total = e.mem.f32(at.wrapping_add(TOTAL_HEIGHT));
    // `FCOMP` with the double 0.0 and `TEST AH,0x41`: on only when strictly above (not a NaN).
    if total > 0.0 && visible > 0.0 {
        // `FCOMPP`, `TEST AH,5`: on only when the visible height is below the total.
        if visible < total {
            // `FILD qword` of the count with a zero high word: the count unsigned.
            let count = num_unfiltered_of(e, of, this) as u32;
            let total = e.mem.f32(at.wrapping_add(TOTAL_HEIGHT));
            let product = super::narrow(f64::from(count) * f64::from(total - visible));
            let ratio = product / total;
            let rounded = e.call(ROUND_FLOAT, &args![ratio]).f64();
            // `00647b70`: the larger (signed), done in place.
            steps = ftol2(e, rounded).wrapping_add(1).max(2);
        }
        if steps > 1 {
            let total = e.mem.f32(at.wrapping_add(TOTAL_HEIGHT));
            let delta = super::narrow(f64::from(total - visible) / f64::from(steps - 1));
            let scroll_delta = trait_by_name(e, SCROLL_DELTA_TEXT);
            let container = e.mem.u32(at.wrapping_add(CONTAINER));
            tile_set_float(e, container, scroll_delta, delta);
        }
    }
    if e.mem.u32(at.wrapping_add(SCROLL_BAR)) != 0 {
        let items = trait_by_name(e, NUMBER_OF_ITEMS_TEXT);
        let scroll_bar = e.mem.u32(at.wrapping_add(SCROLL_BAR));
        tile_set_int(e, scroll_bar, items, steps);
    }
}

/// `ListBox<T>::NumUnfiltered` (Xbox PDB): the list's count (`00726c40`) less the container's
/// `"_num_filtered"` (the id a function-local static), truncated by a 64-bit `FISTP` and taken to
/// 32 bits.
fn num_unfiltered(e: &mut Engine, of: Of, this: Ptr) -> i32 {
    let guard = of.guards()[2];
    let num_filtered = static_trait(e, guard, 1, guard.wrapping_sub(4), NUM_FILTERED_TEXT);
    let count = typed_call!(
        super::FAMILY,
        0x0072_6c40,
        super::small_units::fn_00726c40(e, this)
    ) as i32;
    let container = e.mem.u32(this.addr().wrapping_add(CONTAINER));
    let filtered = tile_float(e, container, num_filtered);
    count.wrapping_sub(super::low_dword_signed(fistp_chop64(f64::from(filtered))))
}

/// A `BSSimpleArray` constructor: the vtable, then the set-up with 0 and 0 (`006b3eb0`, the pathing
/// family's). `this`.
fn simple_array_construct(e: &mut Engine, this: Ptr, vtable: u32) -> Ptr {
    e.mem.set_u32(this.addr(), vtable);
    typed_call!(
        pathing::FAMILY,
        0x006b_3eb0,
        pathing::search::init_vertices(e, Ptr::new(this.addr()), 0, 0)
    );
    this
}

/// A `BSSimpleArray` destructor: the vtable, then the clear with 1 (`008454f0`, the dispatch
/// family's).
pub(super) fn simple_array_destruct(e: &mut Engine, this: Ptr, vtable: u32) {
    e.mem.set_u32(this.addr(), vtable);
    typed_call!(
        dispatch::FAMILY,
        0x0084_54f0,
        dispatch::refs::array_clear(e, this, 1)
    );
}

// Typed from 00721060
/// The `ListBox<ActorValue::Index>` constructor. `this`.
pub fn fn_00721060(e: &mut Engine, this: Ptr) -> Ptr {
    construct(e, Of::ActorValue, this)
}

// Typed from 00721130
/// The list box collection's add (cdecl): the address of its argument passed to `005ae3d0` on the
/// collection.
pub fn fn_00721130(e: &mut Engine, list_box: u32) {
    collection(e, LIST_BOX_COLLECTION_ADD, list_box);
}

// Typed from 00721150
/// The `ListBox<ActorValue::Index>` destructor.
pub fn fn_00721150(e: &mut Engine, this: Ptr) {
    destruct(e, Of::ActorValue, this);
}

// Typed from 007211c0
/// The list box collection's remove (cdecl): the address of its argument passed to `00905330` on
/// the collection.
pub fn fn_007211c0(e: &mut Engine, list_box: u32) {
    collection(e, LIST_BOX_COLLECTION_REMOVE, list_box);
}

// Typed from 007211e0
/// `ListBox<enum ActorValue::Index>::RemoveAll` (Xbox PDB).
pub fn list_box_actor_value_remove_all(e: &mut Engine, this: Ptr) {
    remove_all(e, Of::ActorValue, this);
}

// Typed from 00721410
/// `ListBox<enum ActorValue::Index>::Sort` (Xbox PDB).
pub fn list_box_actor_value_sort(e: &mut Engine, this: Ptr, compare: u32) {
    sort(e, Of::ActorValue, this, compare);
}

// Typed from 00721440
/// `ListBox<enum ActorValue::Index>::Highlight` (Xbox PDB).
pub fn list_box_actor_value_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    highlight(e, Of::ActorValue, this, tile)
}

// Typed from 007214b0
/// `ListBox<enum ActorValue::Index>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_actor_value_get_highlight_target(e: &mut Engine, this: Ptr, direction: u32) -> u32 {
    get_highlight_target(e, Of::ActorValue, this, direction)
}

// Typed from 00721850
/// The `ListBox<PerkRankData *>` constructor. `this`.
pub fn fn_00721850(e: &mut Engine, this: Ptr) -> Ptr {
    construct(e, Of::PerkRankData, this)
}

// Typed from 00721920
/// The `ListBox<PerkRankData *>` destructor.
pub fn fn_00721920(e: &mut Engine, this: Ptr) {
    destruct(e, Of::PerkRankData, this);
}

// Typed from 00721990
/// `ListBox<PerkRankData *>::RemoveAll` (Xbox PDB).
pub fn list_box_perk_rank_data_remove_all(e: &mut Engine, this: Ptr) {
    remove_all(e, Of::PerkRankData, this);
}

// Typed from 00721bc0
/// `ListBox<PerkRankData *>::Sort` (Xbox PDB).
pub fn list_box_perk_rank_data_sort(e: &mut Engine, this: Ptr, compare: u32) {
    sort(e, Of::PerkRankData, this, compare);
}

// Typed from 00721bf0
/// `ListBox<PerkRankData *>::Highlight` (Xbox PDB).
pub fn list_box_perk_rank_data_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    highlight(e, Of::PerkRankData, this, tile)
}

// Typed from 00721c60
/// `ListBox<PerkRankData *>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_perk_rank_data_get_highlight_target(
    e: &mut Engine,
    this: Ptr,
    direction: u32,
) -> u32 {
    get_highlight_target(e, Of::PerkRankData, this, direction)
}

// Typed from 00722000
/// The `ListBox<MiscStatManager::MiscStatID>` constructor. `this`.
pub fn fn_00722000(e: &mut Engine, this: Ptr) -> Ptr {
    construct(e, Of::MiscStatId, this)
}

// Typed from 007220d0
/// The `ListBox<MiscStatManager::MiscStatID>` destructor.
pub fn fn_007220d0(e: &mut Engine, this: Ptr) {
    destruct(e, Of::MiscStatId, this);
}

// Typed from 00722140
/// `ListBox<enum MiscStatManager::MiscStatID>::RemoveAll` (Xbox PDB).
pub fn list_box_misc_stat_id_remove_all(e: &mut Engine, this: Ptr) {
    remove_all(e, Of::MiscStatId, this);
}

// Typed from 00722370
/// `ListBox<enum MiscStatManager::MiscStatID>::Sort` (Xbox PDB).
pub fn list_box_misc_stat_id_sort(e: &mut Engine, this: Ptr, compare: u32) {
    sort(e, Of::MiscStatId, this, compare);
}

// Typed from 007223a0
/// `ListBox<enum MiscStatManager::MiscStatID>::Highlight` (Xbox PDB).
pub fn list_box_misc_stat_id_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    highlight(e, Of::MiscStatId, this, tile)
}

// Typed from 00722410
/// `ListBox<enum MiscStatManager::MiscStatID>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_misc_stat_id_get_highlight_target(
    e: &mut Engine,
    this: Ptr,
    direction: u32,
) -> u32 {
    get_highlight_target(e, Of::MiscStatId, this, direction)
}

// Typed from 007227b0
/// The `ListBox<StatsMenu::StatusEffect *>` constructor. `this`.
pub fn fn_007227b0(e: &mut Engine, this: Ptr) -> Ptr {
    construct(e, Of::StatusEffect, this)
}

// Typed from 00722880
/// The `ListBox<StatsMenu::StatusEffect *>` destructor.
pub fn fn_00722880(e: &mut Engine, this: Ptr) {
    destruct(e, Of::StatusEffect, this);
}

// Typed from 007228f0
/// `ListBox<StatsMenu::StatusEffect *>::RemoveAll` (Xbox PDB).
pub fn list_box_status_effect_remove_all(e: &mut Engine, this: Ptr) {
    remove_all(e, Of::StatusEffect, this);
}

// Typed from 00722b20
/// `ListBox<StatsMenu::StatusEffect *>::Sort` (Xbox PDB).
pub fn list_box_status_effect_sort(e: &mut Engine, this: Ptr, compare: u32) {
    sort(e, Of::StatusEffect, this, compare);
}

// Typed from 00722b50
/// `ListBox<StatsMenu::StatusEffect *>::Highlight` (Xbox PDB).
pub fn list_box_status_effect_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    highlight(e, Of::StatusEffect, this, tile)
}

// Typed from 00722bc0
/// `ListBox<StatsMenu::StatusEffect *>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_status_effect_get_highlight_target(
    e: &mut Engine,
    this: Ptr,
    direction: u32,
) -> u32 {
    get_highlight_target(e, Of::StatusEffect, this, direction)
}

// Typed from 00722f60
/// The `ListBox<TESReputation *>` constructor. `this`.
pub fn fn_00722f60(e: &mut Engine, this: Ptr) -> Ptr {
    construct(e, Of::Reputation, this)
}

// Typed from 00723070
/// The `ListBox<TESReputation *>` destructor.
pub fn fn_00723070(e: &mut Engine, this: Ptr) {
    destruct(e, Of::Reputation, this);
}

// Typed from 007230e0
/// `ListBox<TESReputation *>::RemoveAll` (Xbox PDB).
pub fn list_box_tes_reputation_remove_all(e: &mut Engine, this: Ptr) {
    remove_all(e, Of::Reputation, this);
}

// Typed from 00723310
/// `ListBox<TESReputation *>::Sort` (Xbox PDB).
pub fn list_box_tes_reputation_sort(e: &mut Engine, this: Ptr, compare: u32) {
    sort(e, Of::Reputation, this, compare);
}

// Typed from 00723340
/// `ListBox<TESReputation *>::Highlight` (Xbox PDB).
pub fn list_box_tes_reputation_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    highlight(e, Of::Reputation, this, tile)
}

// Typed from 007233b0
/// `ListBox<TESReputation *>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_tes_reputation_get_highlight_target(
    e: &mut Engine,
    this: Ptr,
    direction: u32,
) -> u32 {
    get_highlight_target(e, Of::Reputation, this, direction)
}

// Typed from 00723750
/// The `ListBox<unsigned int>` constructor. `this`.
pub fn fn_00723750(e: &mut Engine, this: Ptr) -> Ptr {
    construct(e, Of::UnsignedInt, this)
}

// Typed from 00723820
/// The `ListBox<unsigned int>` destructor.
pub fn fn_00723820(e: &mut Engine, this: Ptr) {
    destruct(e, Of::UnsignedInt, this);
}

// Typed from 00723890
/// `ListBox<unsigned int>::RemoveAll` (Xbox PDB).
pub fn list_box_unsigned_int_remove_all(e: &mut Engine, this: Ptr) {
    remove_all(e, Of::UnsignedInt, this);
}

// Typed from 00723ac0
/// `ListBox<unsigned int>::Sort` (Xbox PDB).
pub fn list_box_unsigned_int_sort(e: &mut Engine, this: Ptr, compare: u32) {
    sort(e, Of::UnsignedInt, this, compare);
}

// Typed from 00723af0
/// `ListBox<unsigned int>::Highlight` (Xbox PDB).
pub fn list_box_unsigned_int_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    highlight(e, Of::UnsignedInt, this, tile)
}

// Typed from 00723b60
/// `ListBox<unsigned int>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_unsigned_int_get_highlight_target(
    e: &mut Engine,
    this: Ptr,
    direction: u32,
) -> u32 {
    get_highlight_target(e, Of::UnsignedInt, this, direction)
}

// Typed from 00723f00
/// The `BSSimpleArray<SPECIALBookMenu::SpecialBookTextItem *>` constructor. `this`.
pub fn fn_00723f00(e: &mut Engine, this: Ptr) -> Ptr {
    simple_array_construct(e, this, BOOK_TEXT_ITEMS_VTABLE)
}

// Typed from 00723f30
/// The `BSSimpleArray<SPECIALBookMenu::SpecialBookTextItem *>` destructor.
pub fn fn_00723f30(e: &mut Engine, this: Ptr) {
    simple_array_destruct(e, this, BOOK_TEXT_ITEMS_VTABLE);
}

// Typed from 00723f50
/// The `BSSimpleArray<SPECIALBookMenu::SpecialBookButton *>` constructor. `this`.
pub fn fn_00723f50(e: &mut Engine, this: Ptr) -> Ptr {
    simple_array_construct(e, this, BOOK_BUTTONS_VTABLE)
}

// Typed from 00723f80
/// The `BSSimpleArray<SPECIALBookMenu::SpecialBookButton *>` destructor.
pub fn fn_00723f80(e: &mut Engine, this: Ptr) {
    simple_array_destruct(e, this, BOOK_BUTTONS_VTABLE);
}

// Typed from 00723fa0
/// The `BSSimpleArray<LoveTesterMenu::LoveTesterTextItem *>` constructor. `this`.
pub fn fn_00723fa0(e: &mut Engine, this: Ptr) -> Ptr {
    simple_array_construct(e, this, LOVE_TESTER_TEXT_ITEMS_VTABLE)
}

// Typed from 00723fd0
/// The `BSSimpleArray<LoveTesterMenu::LoveTesterTextItem *>` destructor.
pub fn fn_00723fd0(e: &mut Engine, this: Ptr) {
    simple_array_destruct(e, this, LOVE_TESTER_TEXT_ITEMS_VTABLE);
}

// Typed from 00723ff0
/// The `BSSimpleArray<LoveTesterMenu::LoveTesterLightBulbGlows *>` constructor. `this`.
pub fn fn_00723ff0(e: &mut Engine, this: Ptr) -> Ptr {
    simple_array_construct(e, this, LOVE_TESTER_GLOWS_VTABLE)
}

// Typed from 00724020
/// The `BSSimpleArray<LoveTesterMenu::LoveTesterLightBulbGlows *>` destructor.
pub fn fn_00724020(e: &mut Engine, this: Ptr) {
    simple_array_destruct(e, this, LOVE_TESTER_GLOWS_VTABLE);
}

// Typed from 00724040
/// The `BSSimpleArray<LoveTesterMenu::LoveTesterButton *>` constructor. `this`.
pub fn fn_00724040(e: &mut Engine, this: Ptr) -> Ptr {
    simple_array_construct(e, this, LOVE_TESTER_BUTTONS_VTABLE)
}

// Typed from 00724070
/// The `BSSimpleArray<LoveTesterMenu::LoveTesterButton *>` destructor.
pub fn fn_00724070(e: &mut Engine, this: Ptr) {
    simple_array_destruct(e, this, LOVE_TESTER_BUTTONS_VTABLE);
}

// Typed from 00724090
/// `ListBox<enum ActorValue::Index>`'s deleting destructor (Xbox PDB). `this`.
pub fn list_box_actor_value_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_1150, fn_00721150(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 007240c0
/// The `ListBoxBase` constructor: its vtable. `this`.
pub fn fn_007240c0(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), LIST_BOX_BASE_VTABLE);
    this
}

// Typed from 007240e0
/// `ListBox<PerkRankData *>`'s deleting destructor (Xbox PDB). `this`.
pub fn list_box_perk_rank_data_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_1920, fn_00721920(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00724110
/// `ListBox<enum MiscStatManager::MiscStatID>`'s deleting destructor (Xbox PDB). `this`.
pub fn list_box_misc_stat_id_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_20d0, fn_007220d0(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00724140
/// `ListBox<StatsMenu::StatusEffect *>`'s deleting destructor (Xbox PDB). `this`.
pub fn list_box_status_effect_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_2880, fn_00722880(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00724170
/// `ListBox<TESReputation *>`'s deleting destructor (Xbox PDB). `this`.
pub fn list_box_tes_reputation_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_3070, fn_00723070(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 007241a0
/// `ListBox<unsigned int>`'s deleting destructor (Xbox PDB). `this`.
pub fn list_box_unsigned_int_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_3820, fn_00723820(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 007241d0
/// `BSSimpleArray<SPECIALBookMenu::SpecialBookTextItem *, 1024>`'s deleting destructor (Xbox
/// PDB). `this`.
pub fn simple_array_special_book_text_item_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_3f30, fn_00723f30(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00724200
/// `BSSimpleArray<SPECIALBookMenu::SpecialBookButton *, 1024>`'s deleting destructor (Xbox PDB).
/// `this`.
pub fn simple_array_special_book_button_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_3f80, fn_00723f80(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00724230
/// `BSSimpleArray<LoveTesterMenu::LoveTesterTextItem *, 1024>`'s deleting destructor (Xbox PDB).
/// `this`.
pub fn simple_array_love_tester_text_item_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_3fd0, fn_00723fd0(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00724260
/// `BSSimpleArray<LoveTesterMenu::LoveTesterLightBulbGlows *, 1024>`'s deleting destructor (Xbox
/// PDB). `this`.
pub fn simple_array_love_tester_light_bulb_glows_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_4020, fn_00724020(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 00724290
/// `BSSimpleArray<LoveTesterMenu::LoveTesterButton *, 1024>`'s deleting destructor (Xbox PDB).
/// `this`.
pub fn simple_array_love_tester_button_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    typed_call!(super::FAMILY, 0x0072_4070, fn_00724070(e, this));
    free_if_flagged(e, this, flags);
    this
}

// Typed from 007242c0
/// Whether bit `bit` (the shift count masked to 5 bits, as `SHL` does) of the list box's `usFlags`
/// is set.
pub fn fn_007242c0(e: &mut Engine, this: Ptr, bit: u32) -> bool {
    let flags = u32::from(e.mem.u16(this.addr().wrapping_add(FLAGS)));
    flags & (1u32 << (bit & 0x1f)) != 0
}

// Typed from 007242f0
/// Whether an item of the list box has `tile` as its tile: the nodes from the first while the node
/// and its item are not null.
pub fn fn_007242f0(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    let mut node = first_node(this.addr());
    while node != 0 {
        let item = e.mem.u32(node);
        if item == 0 {
            break;
        }
        if e.mem.u32(item) == tile {
            return true;
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    false
}

// Typed from 00724360
/// `ListBox<enum ActorValue::Index>::Synchronize` (Xbox PDB).
pub fn list_box_actor_value_synchronize(e: &mut Engine, this: Ptr) {
    synchronize(e, Of::ActorValue, this);
}

// Typed from 00724430
/// `ListBox<enum ActorValue::Index>::IsEnabled` (Xbox PDB).
pub fn list_box_actor_value_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    is_enabled(e, Of::ActorValue, this)
}

// Typed from 007244e0
/// `ListBox<enum ActorValue::Index>::UpdateHighlightBoxPos` (Xbox PDB).
pub fn list_box_actor_value_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    update_highlight_box_pos(e, Of::ActorValue, this);
}

// Typed from 00724800
/// `ListBox<enum ActorValue::Index>::SetTotalListHeight` (Xbox PDB).
pub fn list_box_actor_value_set_total_list_height(e: &mut Engine, this: Ptr) {
    set_total_list_height(e, Of::ActorValue, this);
}

// Typed from 00724930
/// `ListBox<PerkRankData *>::Synchronize` (Xbox PDB).
pub fn list_box_perk_rank_data_synchronize(e: &mut Engine, this: Ptr) {
    synchronize(e, Of::PerkRankData, this);
}

// Typed from 00724a00
/// `ListBox<PerkRankData *>::IsEnabled` (Xbox PDB).
pub fn list_box_perk_rank_data_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    is_enabled(e, Of::PerkRankData, this)
}

// Typed from 00724ab0
/// `ListBox<PerkRankData *>::UpdateHighlightBoxPos` (Xbox PDB).
pub fn list_box_perk_rank_data_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    update_highlight_box_pos(e, Of::PerkRankData, this);
}

// Typed from 00724dd0
/// `ListBox<PerkRankData *>::SetTotalListHeight` (Xbox PDB).
pub fn list_box_perk_rank_data_set_total_list_height(e: &mut Engine, this: Ptr) {
    set_total_list_height(e, Of::PerkRankData, this);
}

// Typed from 00724f00
/// `ListBox<enum MiscStatManager::MiscStatID>::Synchronize` (Xbox PDB).
pub fn list_box_misc_stat_id_synchronize(e: &mut Engine, this: Ptr) {
    synchronize(e, Of::MiscStatId, this);
}

// Typed from 00724fd0
/// `ListBox<enum MiscStatManager::MiscStatID>::IsEnabled` (Xbox PDB).
pub fn list_box_misc_stat_id_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    is_enabled(e, Of::MiscStatId, this)
}

// Typed from 00725080
/// `ListBox<enum MiscStatManager::MiscStatID>::UpdateHighlightBoxPos` (Xbox PDB).
pub fn list_box_misc_stat_id_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    update_highlight_box_pos(e, Of::MiscStatId, this);
}

// Typed from 007253a0
/// `ListBox<enum MiscStatManager::MiscStatID>::SetTotalListHeight` (Xbox PDB).
pub fn list_box_misc_stat_id_set_total_list_height(e: &mut Engine, this: Ptr) {
    set_total_list_height(e, Of::MiscStatId, this);
}

// Typed from 007254d0
/// `ListBox<StatsMenu::StatusEffect *>::Synchronize` (Xbox PDB).
pub fn list_box_status_effect_synchronize(e: &mut Engine, this: Ptr) {
    synchronize(e, Of::StatusEffect, this);
}

// Typed from 007255a0
/// `ListBox<StatsMenu::StatusEffect *>::IsEnabled` (Xbox PDB).
pub fn list_box_status_effect_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    is_enabled(e, Of::StatusEffect, this)
}

// Typed from 00725650
/// `ListBox<StatsMenu::StatusEffect *>::UpdateHighlightBoxPos` (Xbox PDB).
pub fn list_box_status_effect_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    update_highlight_box_pos(e, Of::StatusEffect, this);
}

// Typed from 00725970
/// `ListBox<StatsMenu::StatusEffect *>::SetTotalListHeight` (Xbox PDB).
pub fn list_box_status_effect_set_total_list_height(e: &mut Engine, this: Ptr) {
    set_total_list_height(e, Of::StatusEffect, this);
}

// Typed from 00725aa0
/// `ListBox<TESReputation *>::Synchronize` (Xbox PDB).
pub fn list_box_tes_reputation_synchronize(e: &mut Engine, this: Ptr) {
    synchronize(e, Of::Reputation, this);
}

// Typed from 00725b70
/// `ListBox<TESReputation *>::IsEnabled` (Xbox PDB).
pub fn list_box_tes_reputation_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    is_enabled(e, Of::Reputation, this)
}

// Typed from 00725c20
/// `ListBox<TESReputation *>::UpdateHighlightBoxPos` (Xbox PDB).
pub fn list_box_tes_reputation_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    update_highlight_box_pos(e, Of::Reputation, this);
}

// Typed from 00725f40
/// `ListBox<TESReputation *>::SetTotalListHeight` (Xbox PDB).
pub fn list_box_tes_reputation_set_total_list_height(e: &mut Engine, this: Ptr) {
    set_total_list_height(e, Of::Reputation, this);
}

// Typed from 00726090
/// `ListBox<unsigned int>::Synchronize` (Xbox PDB).
pub fn list_box_unsigned_int_synchronize(e: &mut Engine, this: Ptr) {
    synchronize(e, Of::UnsignedInt, this);
}

// Typed from 00726160
/// `ListBox<unsigned int>::IsEnabled` (Xbox PDB).
pub fn list_box_unsigned_int_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    is_enabled(e, Of::UnsignedInt, this)
}

// Typed from 00726210
/// `ListBox<unsigned int>::UpdateHighlightBoxPos` (Xbox PDB).
pub fn list_box_unsigned_int_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    update_highlight_box_pos(e, Of::UnsignedInt, this);
}

// Typed from 00726530
/// `ListBox<unsigned int>::SetTotalListHeight` (Xbox PDB).
pub fn list_box_unsigned_int_set_total_list_height(e: &mut Engine, this: Ptr) {
    set_total_list_height(e, Of::UnsignedInt, this);
}

// Typed from 00726660
/// `ListBox<enum ActorValue::Index>::NumUnfiltered` (Xbox PDB).
pub fn list_box_actor_value_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    num_unfiltered(e, Of::ActorValue, this)
}

// Typed from 00726710
/// `ListBox<PerkRankData *>::NumUnfiltered` (Xbox PDB).
pub fn list_box_perk_rank_data_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    num_unfiltered(e, Of::PerkRankData, this)
}

// Typed from 007267c0
/// `ListBox<enum MiscStatManager::MiscStatID>::NumUnfiltered` (Xbox PDB).
pub fn list_box_misc_stat_id_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    num_unfiltered(e, Of::MiscStatId, this)
}

// Typed from 00726870
/// `ListBox<StatsMenu::StatusEffect *>::NumUnfiltered` (Xbox PDB).
pub fn list_box_status_effect_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    num_unfiltered(e, Of::StatusEffect, this)
}

// Typed from 00726920
/// `ListBox<TESReputation *>::NumUnfiltered` (Xbox PDB).
pub fn list_box_tes_reputation_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    num_unfiltered(e, Of::Reputation, this)
}

// Typed from 00726b40
/// `ListBox<unsigned int>::NumUnfiltered` (Xbox PDB).
pub fn list_box_unsigned_int_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    num_unfiltered(e, Of::UnsignedInt, this)
}

/// This file's typed functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0072_1060, fn_00721060(Ptr) -> Ptr),
        entry!(0x0072_1130, fn_00721130(u32)),
        entry!(0x0072_1150, fn_00721150(Ptr)),
        entry!(0x0072_11c0, fn_007211c0(u32)),
        entry!(0x0072_11e0, list_box_actor_value_remove_all(Ptr)),
        entry!(0x0072_1410, list_box_actor_value_sort(Ptr, u32)),
        entry!(0x0072_1440, list_box_actor_value_highlight(Ptr, u32) -> bool),
        entry!(0x0072_14b0, list_box_actor_value_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x0072_1850, fn_00721850(Ptr) -> Ptr),
        entry!(0x0072_1920, fn_00721920(Ptr)),
        entry!(0x0072_1990, list_box_perk_rank_data_remove_all(Ptr)),
        entry!(0x0072_1bc0, list_box_perk_rank_data_sort(Ptr, u32)),
        entry!(0x0072_1bf0, list_box_perk_rank_data_highlight(Ptr, u32) -> bool),
        entry!(0x0072_1c60, list_box_perk_rank_data_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x0072_2000, fn_00722000(Ptr) -> Ptr),
        entry!(0x0072_20d0, fn_007220d0(Ptr)),
        entry!(0x0072_2140, list_box_misc_stat_id_remove_all(Ptr)),
        entry!(0x0072_2370, list_box_misc_stat_id_sort(Ptr, u32)),
        entry!(0x0072_23a0, list_box_misc_stat_id_highlight(Ptr, u32) -> bool),
        entry!(0x0072_2410, list_box_misc_stat_id_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x0072_27b0, fn_007227b0(Ptr) -> Ptr),
        entry!(0x0072_2880, fn_00722880(Ptr)),
        entry!(0x0072_28f0, list_box_status_effect_remove_all(Ptr)),
        entry!(0x0072_2b20, list_box_status_effect_sort(Ptr, u32)),
        entry!(0x0072_2b50, list_box_status_effect_highlight(Ptr, u32) -> bool),
        entry!(0x0072_2bc0, list_box_status_effect_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x0072_2f60, fn_00722f60(Ptr) -> Ptr),
        entry!(0x0072_3070, fn_00723070(Ptr)),
        entry!(0x0072_30e0, list_box_tes_reputation_remove_all(Ptr)),
        entry!(0x0072_3310, list_box_tes_reputation_sort(Ptr, u32)),
        entry!(0x0072_3340, list_box_tes_reputation_highlight(Ptr, u32) -> bool),
        entry!(0x0072_33b0, list_box_tes_reputation_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x0072_3750, fn_00723750(Ptr) -> Ptr),
        entry!(0x0072_3820, fn_00723820(Ptr)),
        entry!(0x0072_3890, list_box_unsigned_int_remove_all(Ptr)),
        entry!(0x0072_3ac0, list_box_unsigned_int_sort(Ptr, u32)),
        entry!(0x0072_3af0, list_box_unsigned_int_highlight(Ptr, u32) -> bool),
        entry!(0x0072_3b60, list_box_unsigned_int_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x0072_3f00, fn_00723f00(Ptr) -> Ptr),
        entry!(0x0072_3f30, fn_00723f30(Ptr)),
        entry!(0x0072_3f50, fn_00723f50(Ptr) -> Ptr),
        entry!(0x0072_3f80, fn_00723f80(Ptr)),
        entry!(0x0072_3fa0, fn_00723fa0(Ptr) -> Ptr),
        entry!(0x0072_3fd0, fn_00723fd0(Ptr)),
        entry!(0x0072_3ff0, fn_00723ff0(Ptr) -> Ptr),
        entry!(0x0072_4020, fn_00724020(Ptr)),
        entry!(0x0072_4040, fn_00724040(Ptr) -> Ptr),
        entry!(0x0072_4070, fn_00724070(Ptr)),
        entry!(0x0072_4090, list_box_actor_value_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0072_40c0, fn_007240c0(Ptr) -> Ptr),
        entry!(0x0072_40e0, list_box_perk_rank_data_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0072_4110, list_box_misc_stat_id_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0072_4140, list_box_status_effect_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0072_4170, list_box_tes_reputation_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(0x0072_41a0, list_box_unsigned_int_scalar_deleting_destructor(Ptr, u32) -> Ptr),
        entry!(
            0x0072_41d0,
            simple_array_special_book_text_item_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(
            0x0072_4200,
            simple_array_special_book_button_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(
            0x0072_4230,
            simple_array_love_tester_text_item_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(
            0x0072_4260,
            simple_array_love_tester_light_bulb_glows_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(
            0x0072_4290,
            simple_array_love_tester_button_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x0072_42c0, fn_007242c0(Ptr, u32) -> bool),
        entry!(0x0072_42f0, fn_007242f0(Ptr, u32) -> bool),
        entry!(0x0072_4360, list_box_actor_value_synchronize(Ptr)),
        entry!(0x0072_4430, list_box_actor_value_is_enabled(Ptr) -> bool),
        entry!(
            0x0072_44e0,
            list_box_actor_value_update_highlight_box_pos(Ptr)
        ),
        entry!(0x0072_4800, list_box_actor_value_set_total_list_height(Ptr)),
        entry!(0x0072_4930, list_box_perk_rank_data_synchronize(Ptr)),
        entry!(0x0072_4a00, list_box_perk_rank_data_is_enabled(Ptr) -> bool),
        entry!(
            0x0072_4ab0,
            list_box_perk_rank_data_update_highlight_box_pos(Ptr)
        ),
        entry!(
            0x0072_4dd0,
            list_box_perk_rank_data_set_total_list_height(Ptr)
        ),
        entry!(0x0072_4f00, list_box_misc_stat_id_synchronize(Ptr)),
        entry!(0x0072_4fd0, list_box_misc_stat_id_is_enabled(Ptr) -> bool),
        entry!(
            0x0072_5080,
            list_box_misc_stat_id_update_highlight_box_pos(Ptr)
        ),
        entry!(
            0x0072_53a0,
            list_box_misc_stat_id_set_total_list_height(Ptr)
        ),
        entry!(0x0072_54d0, list_box_status_effect_synchronize(Ptr)),
        entry!(0x0072_55a0, list_box_status_effect_is_enabled(Ptr) -> bool),
        entry!(
            0x0072_5650,
            list_box_status_effect_update_highlight_box_pos(Ptr)
        ),
        entry!(
            0x0072_5970,
            list_box_status_effect_set_total_list_height(Ptr)
        ),
        entry!(0x0072_5aa0, list_box_tes_reputation_synchronize(Ptr)),
        entry!(0x0072_5b70, list_box_tes_reputation_is_enabled(Ptr) -> bool),
        entry!(
            0x0072_5c20,
            list_box_tes_reputation_update_highlight_box_pos(Ptr)
        ),
        entry!(
            0x0072_5f40,
            list_box_tes_reputation_set_total_list_height(Ptr)
        ),
        entry!(0x0072_6090, list_box_unsigned_int_synchronize(Ptr)),
        entry!(0x0072_6160, list_box_unsigned_int_is_enabled(Ptr) -> bool),
        entry!(
            0x0072_6210,
            list_box_unsigned_int_update_highlight_box_pos(Ptr)
        ),
        entry!(
            0x0072_6530,
            list_box_unsigned_int_set_total_list_height(Ptr)
        ),
        entry!(0x0072_6660, list_box_actor_value_num_unfiltered(Ptr) -> i32),
        entry!(0x0072_6710, list_box_perk_rank_data_num_unfiltered(Ptr) -> i32),
        entry!(0x0072_67c0, list_box_misc_stat_id_num_unfiltered(Ptr) -> i32),
        entry!(0x0072_6870, list_box_status_effect_num_unfiltered(Ptr) -> i32),
        entry!(0x0072_6920, list_box_tes_reputation_num_unfiltered(Ptr) -> i32),
        entry!(0x0072_6b40, list_box_unsigned_int_num_unfiltered(Ptr) -> i32),
    ]
}
