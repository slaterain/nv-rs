//! The `ListBox<ItemChange *>` instance of the list box template (the Xbox PDB's
//! `fallout/interface/listbox` unit) and the item lists' helpers the engine map puts after the
//! interface manager (its unit `run_00719be0`): the translations are engine-fallout-interface's
//! `listbox.rs` and `run_00719be0.rs`, which these functions replace at the same addresses.
//!
//! - The list box's `RemoveItemGroup`, `AddItem`, `RemoveItem`, `Synchronize`,
//!   `UpdateHighlightBoxPos`, its node removal (`0071aaa0`), `SetTotalListHeight` and
//!   `NumUnfiltered`, and the two accessors of a tile's byte at `+0x35`. The layout is the template's
//!   ([`super::menu_manager_p2`] has the other six instances), and so are the bodies of the four
//!   methods every instance has, here over this instance's statics and its own methods.
//! - The fill of an item list box from a reference's inventory (`00719ef0`), the sorts'
//!   comparisons (by name, count, health), the filters by form type, a word setting's constructor
//!   and a `BSSimpleArray<Tile *>`'s destructor.
//!
//! **Native.** The exe's order of work, with its calls by address of functions no family types (the
//! tile's menu and template rendering, the trait ids by name, the value actions, the allocator and
//! free, the list's insert and appends, a node's and an item's deletes, the focus, the roundings,
//! the inventory walk and its iterator, the item's copy, the strings, the cast, the setting's base
//! and collection). `Tile::Value::CalculateValue` (`00a09410`), an item's mods test (`004bd820`) and
//! the iterator's deleting destructor (`004bf630`) are `overridable.tsv` rows: by address too. The
//! tile's getters and setters are the dispatch family's, `_ftol2` the C runtime's, the simple
//! array's clear the dispatch family's; the list box's flag test and item count and this
//! instance's own methods are this family's (all through `typed_call!`). What the exe calls that
//! is a plain read or store is done in place ([`IN_PLACE`] and the lists of the other files): a
//! list node's item and next, an item's data and count, the list's emptiness, the local list's and
//! the `ListBoxItem`'s constructors, the interface manager, a form's type and a name's characters,
//! a node's next stored, a list box flag set, the larger of two integers. (The tile's byte at
//! `+0x35`, `0071aa80` and `0071aa60`, are this file's, called through `typed_call!` here and by
//! the other instances.) A virtual call (an item
//! tile's deleting destructor, the list box's `RemoveAll` before a fill, a health form's health,
//! the setting collection's add) stays virtual: the object's class is not known here.
//!
//! **Floats.** The main thread's: the filtered count less one and the scroll steps as
//! [`super::menu_manager_p2`]'s instances do them (each step the float result).

use super::interface_cpp::in_place;
use super::interface_cpp_p2::name_characters;
use super::menu_manager_p2::{
    child_by_name, first_node, fn_007242c0, mark_tile, set_flag, simple_array_destruct,
    static_trait, tile_float, tile_set_float, tile_set_int, trait_by_name, ACTION_COPY,
    ADD_USER_TRAIT, CONTAINER, EDGE_TEXTS, FIRST_ITEM, FREE, HIGHLIGHTED, HIGHLIGHT_BOX_TEXT,
    HIGHLIGHT_Y_TEXT, INIT_TILE_POSITION, ITEM_DATA, ITEM_DELETE, ITEM_FILTERED, NEXT_INDEX,
    NODE_DELETE, NUMBER_OF_ITEMS_TEXT, NUM_FILTERED_TEXT, ROUND_FLOAT, SCROLL_BAR,
    SCROLL_DELTA_TEXT, SECOND_NODE, SELECTED_HEIGHT_TEXT, TEMPLATE_NAME, TILE_BYTE_35, TOP_TEXT,
    TOTAL_HEIGHT, TRAIT_HEIGHT, TRAIT_INDEX, TRAIT_Y, VALUE_ADD_ACTION, VALUE_CALCULATE,
};
use crate::dispatch;
use engine_core::prelude::*;
use engine_core::typed_call;
use engine_core::x87::fistp_chop64;

/// What this file does in place besides the other files' lists: plain reads and stores the exe
/// calls (the differential tests leave these out of the translation's calls).
pub const IN_PLACE: &[u32] = &[
    0x0082_56d0, // a list's emptiness: both its words zero
    0x007a_62e0, // the `ListBoxItem` constructor: the tile, the data, a zero byte
    0x0040_4920, // the word setting's base constructor: three stores
];

/// `UpdateHighlightBoxPos`'s statics' guard (bit 0 the `"_highlight_y"` id 4 bytes below it, bit 1
/// the `"_selected_height"` id 8 below) and `NumUnfiltered`'s (its `"_num_filtered"` id below it).
const HIGHLIGHT_GUARD: u32 = 0x011d_8cd4;
const NUM_FILTERED_GUARD: u32 = 0x011d_8cdc;
/// `"_number_of_visible_items"`.
const VISIBLE_ITEMS_TEXT: u32 = 0x0106_f860;
/// The traits `AddItem` reads and sets by id: the tile's id, its string; the focus trait.
const TRAIT_ID: i32 = 0xfaa;
const TRAIT_STRING: i32 = 0xfc4;
const TRAIT_FOCUS: u32 = 0xfc3;
/// The value action with a float operand (`Tile::Value::AddAction`, thiscall: action, float) and
/// the multiply operator (`CalculateValue`'s switch: `0x7d1` adds, `0x7d3` multiplies).
const VALUE_ADD_FLOAT: u32 = 0x00a0_9080;
const ACTION_MULTIPLY: u32 = 0x7d3;
/// `Tile::GetMenu` (thiscall) and `Menu::RenderTemplate` (thiscall: parent, template, 0).
const TILE_GET_MENU: u32 = 0x00a0_3c90;
const RENDER_TEMPLATE: u32 = 0x00a1_ddb0;
/// The allocator (cdecl: size).
const ALLOCATE: u32 = 0x0040_1000;
/// The list's insert after a node (thiscall: item, node) and append of the word at an address
/// (thiscall: address).
const LIST_INSERT_AFTER: u32 = 0x007a_7eb0;
const LIST_APPEND: u32 = 0x0090_5820;
/// The local list's append of the word at an address, its remove-all and destructor (thiscall).
const LOCAL_APPEND: u32 = 0x005a_e3d0;
const LOCAL_REMOVE_ALL: u32 = 0x0047_0470;
const LOCAL_DESTRUCT: u32 = 0x0046_ffb0;
/// `InterfaceManager::SetCurrentFocusTarget` (thiscall: tile, trait, 1).
const SET_FOCUS_TARGET: u32 = 0x0071_5860;

/// The inventory fill's callees: the reference's inventory walk (`00575510`, thiscall: the flags,
/// the iterator's and the changes' slots), its next item (`004ca330`, thiscall on the changes: the
/// iterator), the iterator's deleting destructor (`004bf630`, an `overridable.tsv` row), the
/// item's mods test (`004bd820`, one too), the new item's constructor and copy (`0076b630`,
/// `004bc810`), the name's length, copy and append (cdecl), `__RTDynamicCast` (cdecl, five words)
/// and the suffix of a modded item's name.
const INVENTORY_WALK: u32 = 0x0057_5510;
const NEXT_ITEM: u32 = 0x004c_a330;
const ITERATOR_DELETE: u32 = 0x004b_f630;
const ITEM_HAS_MODS: u32 = 0x004b_d820;
const ITEM_CONSTRUCT: u32 = 0x0076_b630;
const ITEM_COPY: u32 = 0x004b_c810;
const STRING_LENGTH: u32 = 0x0044_a670;
const STRING_COPY: u32 = 0x0040_6d30;
const STRING_APPEND: u32 = 0x0040_6d50;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const MODDED_SUFFIX: u32 = 0x0103_45e8;
/// The cast's source (`TESForm`) and targets (`TESFullName`, `TESHealthForm`) type descriptors.
const CAST_FORM: u32 = 0x0118_3108;
const CAST_FULL_NAME: u32 = 0x0118_3158;
const CAST_HEALTH_FORM: u32 = 0x0118_6c3c;
/// The list box's `RemoveAll` slot, a health form's health slot.
const REMOVE_ALL_SLOT: u32 = 0x1c;
const HEALTH_SLOT: u32 = 0x10;
/// The sorts' callees: `ItemChange::GetFullName` (thiscall) and the names' compare (cdecl).
const ITEM_FULL_NAME: u32 = 0x004b_e2d0;
const COMPARE_NAMES: u32 = 0x0046_9880;
/// The word setting's vtable and the collection the interface settings are added to (`004de490`,
/// its add the vtable's `+4`).
const SETTING_VTABLE: u32 = 0x0102_2738;
const SETTING_COLLECTION: u32 = 0x004d_e490;
const COLLECTION_ADD_SLOT: u32 = 4;
/// The `BSSimpleArray<Tile *>` vtable (RTTI).
const TILE_ARRAY_VTABLE: u32 = 0x0106_f798;
/// An item's count (`+4`, `00726070`) and form (`+8`, `0044ddc0`); a form's type byte (`+4`).
const ITEM_COUNT: u32 = 4;
const ITEM_FORM: u32 = 8;
const FORM_TYPE: u32 = 4;

/// The list box's flag test (`007242c0`, the family's).
#[inline]
fn flag(e: &mut Engine, this: Ptr, bit: u32) -> bool {
    typed_call!(super::FAMILY, 0x0072_42c0, fn_007242c0(e, this, bit))
}

/// A new `ListBoxItem` (Xbox PDB) in `block` (`007a62e0`, in place): its tile, its data and the
/// filtered byte clear. `block`.
#[inline]
fn new_item(e: &mut Engine, block: u32, tile: u32, data: u32) -> u32 {
    e.mem.set_u32(block, tile);
    e.mem.set_u32(block.wrapping_add(ITEM_DATA), data);
    e.mem.set_u8(block.wrapping_add(ITEM_FILTERED), 0);
    block
}

/// A new `ListBoxItem` of `tile` and `data` in a fresh 12-byte block, or 0 when the allocator
/// answers 0.
#[inline]
pub(super) fn allocate_item(e: &mut Engine, tile: u32, data: u32) -> u32 {
    let block = e.call(ALLOCATE, &args![0xcu32]).u32();
    if block == 0 {
        0
    } else {
        new_item(e, block, tile, data)
    }
}

// Typed from 0071a200
/// `ListBox<ItemChange *>::RemoveItemGroup` (Xbox PDB): the items whose data's form (its word at
/// `+8`) is `group` are gathered in a local list (its constructor in place, `005ae3d0` given each
/// node, whose first word is the item), then each gathered item's tile is removed (`RemoveItem`),
/// and the local list is emptied and destroyed. Each walk stops at a node with no item.
pub fn list_box_remove_item_group(e: &mut Engine, this: Ptr, group: u32) {
    e.with_stack(8, |e, local| {
        let local = local.addr();
        super::menu_manager::empty_list(e, local);
        let mut node = first_node(this.addr());
        while node != 0 {
            let item = e.mem.u32(node);
            if item == 0 {
                break;
            }
            let data = e.mem.u32(item.wrapping_add(ITEM_DATA));
            if data != 0 && e.mem.u32(data.wrapping_add(ITEM_FORM)) == group {
                e.call(LOCAL_APPEND, &args![local, node]);
            }
            node = e.mem.u32(node.wrapping_add(4));
        }
        let mut node = local;
        while node != 0 {
            let item = e.mem.u32(node);
            if item == 0 {
                break;
            }
            let tile = e.mem.u32(item);
            typed_call!(
                super::FAMILY,
                0x0071_a5c0,
                list_box_remove_item(e, this, Ptr::new(tile))
            );
            node = e.mem.u32(node.wrapping_add(4));
        }
        e.call(LOCAL_REMOVE_ALL, &args![local]);
        e.call(LOCAL_DESTRUCT, &args![local]);
    });
}

// Typed from 0071a320
/// `ListBox<ItemChange *>::AddItem` (Xbox PDB): with a container and a template (`template`, else
/// the list's), the template rendered into the container (`Tile::GetMenu`,
/// `Menu::RenderTemplate`); the new tile gets the id -1 when it has none and the string `text` when
/// given; a new `ListBoxItem` of the tile and `data` is inserted after `after` (then `Synchronize`
/// when flag 0 is set) or appended (then, with flag 0, the tile's position and
/// `SetTotalListHeight`; the tile numbered with the next index). With exactly one item and a
/// `"_number_of_visible_items"` above 0.0, the container's height becomes the tile's height times
/// that number (two actions, then calculated). The new tile, or null.
pub fn list_box_add_item(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    text: Ptr,
    after: u32,
    template: Ptr,
) -> Ptr {
    let at = this.addr();
    let container = e.mem.u32(at.wrapping_add(CONTAINER));
    let own_template = e.mem.u32(at.wrapping_add(TEMPLATE_NAME));
    if container == 0 || (own_template == 0 && template.is_null()) {
        return Ptr::NULL;
    }
    let template = if template.is_null() {
        own_template
    } else {
        template.addr()
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
    if !text.is_null() {
        typed_call!(
            dispatch::FAMILY,
            0x00a0_1350,
            dispatch::tile::set_string(e, Ptr::new(tile), TRAIT_STRING, text, 1)
        );
    }
    let list = at.wrapping_add(4);
    if after != 0 {
        let item = allocate_item(e, tile, data);
        e.call(LIST_INSERT_AFTER, &args![list, item, after]);
        if flag(e, this, 0) {
            typed_call!(super::FAMILY, 0x0071_a670, list_box_synchronize(e, this));
        }
    } else {
        let item = allocate_item(e, tile, data);
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), item);
            e.call(LIST_APPEND, &args![list, slot]);
        });
        if flag(e, this, 0) {
            e.call(INIT_TILE_POSITION, &args![this, tile]);
            typed_call!(
                super::FAMILY,
                0x0071_ad30,
                list_box_set_total_list_height(e, this)
            );
        }
        let index = e.mem.u16(at.wrapping_add(NEXT_INDEX));
        e.mem
            .set_u16(at.wrapping_add(NEXT_INDEX), index.wrapping_add(1));
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
        let container = e.mem.u32(at.wrapping_add(CONTAINER));
        // `FCOMP` with the double 0.0 and `TEST AH,0x41`: on only when strictly above.
        if tile_float(e, container, visible) > 0.0 {
            let container = e.mem.u32(at.wrapping_add(CONTAINER));
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
            let container = e.mem.u32(at.wrapping_add(CONTAINER));
            let count = tile_float(e, container, visible);
            e.call(VALUE_ADD_FLOAT, &args![value, ACTION_MULTIPLY, count]);
            e.call(VALUE_CALCULATE, &args![value, 0u32]);
        }
    }
    Ptr::new(tile)
}

// Typed from 0071a5c0
/// `ListBox<ItemChange *>::RemoveItem` (Xbox PDB): false for a null `tile` or an empty list (both
/// its words zero, `008256d0` in place); else the node whose item's tile is `tile` (from the
/// first, the item read with no null test, as the exe does) and the node before it are removed
/// (`0071aaa0`).
pub fn list_box_remove_item(e: &mut Engine, this: Ptr, tile: Ptr) -> bool {
    if tile.is_null() {
        return false;
    }
    let list = this.addr().wrapping_add(4);
    if e.mem.u32(list.wrapping_add(4)) == 0 && e.mem.u32(list) == 0 {
        return false;
    }
    let mut node = first_node(this.addr());
    let mut previous = node;
    while node != 0 {
        if e.mem.u32(e.mem.u32(node)) == tile.addr() {
            break;
        }
        previous = node;
        node = e.mem.u32(node.wrapping_add(4));
    }
    typed_call!(
        super::FAMILY,
        0x0071_aaa0,
        fn_0071aaa0(e, this, node, previous)
    )
}

// Typed from 0071a670
/// `ListBox<ItemChange *>::Synchronize` (Xbox PDB): `usNextIndex` and `fTotalListHeight` cleared,
/// then each item not filtered (its byte at +8 clear) gives its tile the next index (trait `0xfac`)
/// and its position (`InitTilePosition`), stopping at a node with no item; then
/// `SetTotalListHeight` and `UpdateHighlightBoxPos`.
pub fn list_box_synchronize(e: &mut Engine, this: Ptr) {
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
    typed_call!(
        super::FAMILY,
        0x0071_ad30,
        list_box_set_total_list_height(e, this)
    );
    typed_call!(
        super::FAMILY,
        0x0071_a740,
        list_box_update_highlight_box_pos(e, this)
    );
}

// Typed from 0071a740
/// `ListBox<ItemChange *>::UpdateHighlightBoxPos` (Xbox PDB): the `"_highlight_y"` and
/// `"_selected_height"` ids (statics); with a container and no highlighted tile, those traits set
/// to -1 and 0; with one, the two values follow the highlighted tile's `0xfa2` and `0xfb0` (an
/// action `0x7d0`, then calculated) and the container's `"lb_highlight_box"` tile, its `"top"` and,
/// when `"top"` exists, its `"bottom"`, `"left"` and `"right"` (not checked for null), take the
/// highlighted tile's byte at +0x35 (as 1 or 0) and an update.
pub fn list_box_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let y_trait = static_trait(
        e,
        HIGHLIGHT_GUARD,
        1,
        HIGHLIGHT_GUARD.wrapping_sub(4),
        HIGHLIGHT_Y_TEXT,
    );
    let height_trait = static_trait(
        e,
        HIGHLIGHT_GUARD,
        2,
        HIGHLIGHT_GUARD.wrapping_sub(8),
        SELECTED_HEIGHT_TEXT,
    );
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
    let container = e.mem.u32(at.wrapping_add(CONTAINER));
    let height_value = typed_call!(
        dispatch::FAMILY,
        0x00a0_0f30,
        dispatch::tile::get_value(e, Ptr::new(container), height_trait)
    );
    for (value, from) in [
        (y_value, TRAIT_Y),
        (height_value, super::bits(TRAIT_HEIGHT)),
    ] {
        let highlighted = e.mem.u32(at.wrapping_add(HIGHLIGHTED));
        e.call(
            VALUE_ADD_ACTION,
            &args![value, ACTION_COPY, highlighted, from],
        );
        e.call(VALUE_CALCULATE, &args![value, 0u32]);
    }
    let highlighted = e.mem.u32(at.wrapping_add(HIGHLIGHTED));
    let mark = u8::from(
        typed_call!(
            super::FAMILY,
            0x0071_aa80,
            fn_0071aa80(e, Ptr::new(highlighted))
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
        mark_tile(e, top, mark);
        for name in EDGE_TEXTS {
            let edge = child_by_name(e, box_tile, name);
            mark_tile(e, edge, mark);
        }
    }
    mark_tile(e, box_tile, mark);
}

// Typed from 0071aa60
/// A tile's byte at `+0x35` (the highlight box's mark) stored.
pub fn fn_0071aa60(e: &mut Engine, this: Ptr, value: u8) {
    e.mem.set_u8(this.addr().wrapping_add(TILE_BYTE_35), value);
}

// Typed from 0071aa80
/// A tile's byte at `+0x35`, in `AL`.
pub fn fn_0071aa80(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr().wrapping_add(TILE_BYTE_35))
}

// Typed from 0071aaa0
/// The list box's node removal (thiscall, `RET 8`: the node, the node before it): false for a null
/// node. When the node's item's tile is the highlighted one, the highlight moves to the next node's
/// tile, else to the one before's (unless that is the node itself), else to none, and a new one is
/// focused (`SetCurrentFocusTarget` on the manager, in place). A filtered item lowers the
/// container's `"_num_filtered"` by one. The item's tile is deleted (its deleting destructor), its
/// data too when flag 1 is set (`004459e0` with 1), and the item freed. The first node (the list's
/// own, at `+4`) takes the second's item and next and the second is deleted, or it is emptied;
/// another node is unlinked from the one before and deleted. Then `Synchronize` when flag 0 is set.
/// What the exe reads again after a call is read again.
pub fn fn_0071aaa0(e: &mut Engine, this: Ptr, node: u32, previous: u32) -> bool {
    if node == 0 {
        return false;
    }
    let at = this.addr();
    let item = e.mem.u32(node);
    if e.mem.u32(item) == e.mem.u32(at.wrapping_add(HIGHLIGHTED)) {
        let next = e.mem.u32(node.wrapping_add(4));
        let highlight = if next != 0 {
            e.mem.u32(e.mem.u32(next))
        } else if previous != 0 && previous != node {
            e.mem.u32(e.mem.u32(previous))
        } else {
            0
        };
        e.mem.set_u32(at.wrapping_add(HIGHLIGHTED), highlight);
        if highlight != 0 {
            let manager = in_place::manager(e);
            e.call(
                SET_FOCUS_TARGET,
                &args![manager, highlight, TRAIT_FOCUS, 1u32],
            );
        }
    }
    if e.mem.u32(at.wrapping_add(CONTAINER)) != 0 {
        let item = e.mem.u32(node);
        if e.mem.u8(item.wrapping_add(ITEM_FILTERED)) != 0 {
            let filtered = trait_by_name(e, NUM_FILTERED_TEXT);
            let container = e.mem.u32(at.wrapping_add(CONTAINER));
            // `FSUB` of the double 1.0, then `FSTP float`: one float step.
            let lowered = tile_float(e, container, filtered) - 1.0;
            let filtered = trait_by_name(e, NUM_FILTERED_TEXT);
            let container = e.mem.u32(at.wrapping_add(CONTAINER));
            tile_set_float(e, container, filtered, lowered);
        }
    }
    let tile = e.mem.u32(e.mem.u32(node));
    if tile != 0 {
        e.vcall(tile, 0, &args![1u32]);
    }
    if flag(e, this, 1) {
        let data = e.mem.u32(e.mem.u32(node).wrapping_add(ITEM_DATA));
        if data != 0 {
            e.call(ITEM_DELETE, &args![data, 1u32]);
        }
    }
    let item = e.mem.u32(node);
    e.call(FREE, &args![item]);
    if node == first_node(at) {
        let second = e.mem.u32(at.wrapping_add(SECOND_NODE));
        if second == 0 {
            e.mem.set_u32(at.wrapping_add(FIRST_ITEM), 0);
        } else {
            let after = e.mem.u32(second.wrapping_add(4));
            e.mem.set_u32(at.wrapping_add(SECOND_NODE), after);
            let moved = e.mem.u32(second);
            e.mem.set_u32(at.wrapping_add(FIRST_ITEM), moved);
            // `006ecd40`, in place: the node's next stored.
            e.mem.set_u32(second.wrapping_add(4), 0);
            e.call(NODE_DELETE, &args![second, 1u32]);
        }
    } else {
        let after = e.mem.u32(node.wrapping_add(4));
        e.mem.set_u32(previous.wrapping_add(4), after);
        e.mem.set_u32(node.wrapping_add(4), 0);
        e.call(NODE_DELETE, &args![node, 1u32]);
    }
    if flag(e, this, 0) {
        typed_call!(super::FAMILY, 0x0071_a670, list_box_synchronize(e, this));
    }
    true
}

// Typed from 0071ad30
/// `ListBox<ItemChange *>::SetTotalListHeight` (Xbox PDB): with a container, its visible height
/// (trait `0xfb0`) and the total list height decide the number of scroll steps: 1, unless both are
/// above 0.0 and the visible height is below the total, then
/// `max(_ftol2(004e4430(unfiltered * (total - visible) / total)) + 1, 2)`; with both above 0.0 and
/// more than one step, the container's `"_scroll_delta"` becomes `(total - visible) / (steps - 1)`;
/// with a scroll bar its `"_number_of_items"` becomes the steps. Each step rounds to a float (the
/// main thread's precision), as the other instances' (while `_num_filtered` is no more than the
/// list's count, the unfiltered count is far below 2^29, so its product with a float is exact in an
/// `f64` and rounded once).
pub fn list_box_set_total_list_height(e: &mut Engine, this: Ptr) {
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
            let count =
                typed_call!(super::FAMILY, 0x0071_ae60, list_box_num_unfiltered(e, this)) as u32;
            let total = e.mem.f32(at.wrapping_add(TOTAL_HEIGHT));
            let product = super::narrow(f64::from(count) * f64::from(total - visible));
            let ratio = product / total;
            let rounded = e.call(ROUND_FLOAT, &args![ratio]).f64();
            // `00647b70`: the larger (signed), done in place.
            steps = super::menu_manager_p2::ftol2(e, rounded)
                .wrapping_add(1)
                .max(2);
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

// Typed from 0071ae60
/// `ListBox<ItemChange *>::NumUnfiltered` (Xbox PDB): the list's count (`00726c40`) less the
/// container's `"_num_filtered"` (the id a function-local static), truncated by a 64-bit `FISTP`
/// and taken to 32 bits.
pub fn list_box_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    let num_filtered = static_trait(
        e,
        NUM_FILTERED_GUARD,
        1,
        NUM_FILTERED_GUARD.wrapping_sub(4),
        NUM_FILTERED_TEXT,
    );
    let count = typed_call!(
        super::FAMILY,
        0x0072_6c40,
        super::small_units::fn_00726c40(e, this)
    ) as i32;
    let container = e.mem.u32(this.addr().wrapping_add(CONTAINER));
    let filtered = tile_float(e, container, num_filtered);
    count.wrapping_sub(super::low_dword_signed(fistp_chop64(f64::from(filtered))))
}

// Typed from 00719be0
/// The `BSSimpleArray<Tile *>` destructor: its vtable, then the clear with 1 (the dispatch
/// family's).
pub fn fn_00719be0(e: &mut Engine, this: Ptr) {
    simple_array_destruct(e, this, TILE_ARRAY_VTABLE);
}

// Typed from 00719c00
/// A word setting of the interface settings (thiscall, `RET 8`: the name, the value): the word
/// setting's base (`00404920`, in place: its name at `+8` and its value at `+4`; the base vtable it
/// stores first is overwritten at once), its vtable, then added to the collection (`004de490`, its
/// vtable's `+4`). `this`.
pub fn fn_00719c00(e: &mut Engine, this: Ptr, name: u32, value: u32) -> Ptr {
    let at = this.addr();
    e.mem.set_u32(at.wrapping_add(8), name);
    e.mem.set_u32(at.wrapping_add(4), value);
    e.mem.set_u32(at, SETTING_VTABLE);
    let collection = e.call(SETTING_COLLECTION, &[]).u32();
    e.vcall(collection, COLLECTION_ADD_SLOT, &args![this]);
    this
}

/// A form's full name (`__RTDynamicCast` to `TESFullName`) or health form (`TESHealthForm`).
#[inline]
fn cast(e: &mut Engine, form: u32, target: u32) -> u32 {
    e.call(DYNAMIC_CAST, &args![form, 0u32, CAST_FORM, target, 0u32])
        .u32()
}

// Typed from 00719ef0
/// Fills an item list box from a reference's inventory (thiscall, `RET 0x18`: the reference, a
/// filter, a word given to `AddItem` (a list with one is synchronized at the end), a callback, a
/// split flag, the walk's flag byte). With a reference, a container and a template: the list
/// emptied (its `RemoveAll`, virtual), flag 1 set, flag 0 kept and cleared; then each item of the
/// walk (`00575510`, `004ca330`) the filter (cdecl, when given) does not refuse is added with its
/// name (its form's full name copied into a 128-byte buffer, the modded suffix appended when it has
/// mods) and given to the callback (cdecl: the new tile, the item). With the split flag an item of
/// several becomes one entry per unit, each later one a new copy (`0076b630`, `004bc810`), every
/// entry's count 1 (`006ecd40`, in place). The iterator is deleted, a list given the word is
/// synchronized, and flag 0 gets back its kept value.
#[allow(clippy::too_many_arguments)]
pub fn fn_00719ef0(
    e: &mut Engine,
    this: Ptr,
    reference: u32,
    filter: u32,
    word: u32,
    callback: u32,
    split: u8,
    flags: u8,
) {
    let at = this.addr();
    if reference == 0
        || e.mem.u32(at.wrapping_add(CONTAINER)) == 0
        || e.mem.u32(at.wrapping_add(TEMPLATE_NAME)) == 0
    {
        return;
    }
    e.vcall(at, REMOVE_ALL_SLOT, &args![]);
    set_flag(e, at, 1, true);
    let kept = flag(e, this, 0);
    set_flag(e, at, 0, false);
    e.with_stack(0x88, |e, locals| {
        let iterator_slot = locals.addr();
        let changes_slot = iterator_slot.wrapping_add(4);
        let buffer = iterator_slot.wrapping_add(8);
        e.call(
            INVENTORY_WALK,
            &args![reference, u32::from(flags), iterator_slot, changes_slot],
        );
        let iterator = e.mem.u32(iterator_slot);
        let changes = e.mem.u32(changes_slot);
        if iterator == 0 || changes == 0 {
            return;
        }
        let mut item = e.call(NEXT_ITEM, &args![changes, iterator]).u32();
        while item != 0 {
            let refused = filter != 0 && e.call(filter, &args![item]).u8() != 0;
            if !refused {
                let mut copies = 0i32;
                let count = e.mem.i32(item.wrapping_add(ITEM_COUNT));
                loop {
                    let form = e.mem.u32(item.wrapping_add(ITEM_FORM));
                    let name = cast(e, form, CAST_FULL_NAME);
                    let characters = name_characters(e, name);
                    let length = e.call(STRING_LENGTH, &args![characters]).u32();
                    let characters = name_characters(e, name);
                    e.call(
                        STRING_COPY,
                        &args![buffer, length.wrapping_add(1), characters],
                    );
                    if e.call(ITEM_HAS_MODS, &args![item]).u8() != 0 {
                        e.call(
                            STRING_APPEND,
                            &args![buffer, length.wrapping_add(2), MODDED_SUFFIX],
                        );
                    }
                    if split != 0 {
                        if copies > 0 {
                            let block = e.call(ALLOCATE, &args![0xcu32]).u32();
                            let copy = if block == 0 {
                                0
                            } else {
                                e.call(ITEM_CONSTRUCT, &args![block]).u32()
                            };
                            e.call(ITEM_COPY, &args![copy, item]);
                            item = copy;
                            e.mem.set_u32(item.wrapping_add(ITEM_COUNT), 1);
                        } else if count > 1 {
                            e.mem.set_u32(item.wrapping_add(ITEM_COUNT), 1);
                        }
                        copies += 1;
                    }
                    let entry = typed_call!(
                        super::FAMILY,
                        0x0071_a320,
                        list_box_add_item(e, this, item, Ptr::new(buffer), word, Ptr::NULL)
                    );
                    if callback != 0 {
                        e.call(callback, &args![entry, item]);
                    }
                    if split == 0 || copies >= count {
                        break;
                    }
                }
            }
            item = e.call(NEXT_ITEM, &args![changes, iterator]).u32();
        }
        let iterator = e.mem.u32(iterator_slot);
        if iterator != 0 {
            e.call(ITERATOR_DELETE, &args![iterator, 1u32]);
        }
    });
    if word != 0 {
        typed_call!(super::FAMILY, 0x0071_a670, list_box_synchronize(e, this));
    }
    set_flag(e, at, 0, kept);
}

// Typed from 00719c80
/// A sort's comparison (cdecl, two items): the names' compare (`00469880`) of the two items' full
/// names (`ItemChange::GetFullName`, the second item's asked first).
pub fn fn_00719c80(e: &mut Engine, first: u32, second: u32) -> i32 {
    let second_name = e.call(ITEM_FULL_NAME, &args![second]).u32();
    let first_name = e.call(ITEM_FULL_NAME, &args![first]).u32();
    e.call(COMPARE_NAMES, &args![first_name, second_name]).i32()
}

// Typed from 00719ca0
/// A sort's comparison (cdecl, two items): -1, 1 or 0 as the first item's count (its `+4`) is below,
/// above or equal to the second's (signed).
pub fn fn_00719ca0(e: &mut Engine, first: u32, second: u32) -> i32 {
    let a = e.mem.i32(first.wrapping_add(ITEM_COUNT));
    let b = e.mem.i32(second.wrapping_add(ITEM_COUNT));
    match a.cmp(&b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

// Typed from 00719cf0
/// A sort's comparison (cdecl, two items): -1, 1 or 0 as the first item's health (its form as a
/// `TESHealthForm`, virtual slot `+0x10`) is below, above or equal to the second's, unsigned. The
/// exe asks each health twice when the first is not below (the calls are virtual: each is made).
pub fn fn_00719cf0(e: &mut Engine, first: u32, second: u32) -> i32 {
    let form = e.mem.u32(first.wrapping_add(ITEM_FORM));
    let first = cast(e, form, CAST_HEALTH_FORM);
    let form = e.mem.u32(second.wrapping_add(ITEM_FORM));
    let second = cast(e, form, CAST_HEALTH_FORM);
    let a = e.vcall(first, HEALTH_SLOT, &args![]).u32();
    let b = e.vcall(second, HEALTH_SLOT, &args![]).u32();
    if a < b {
        return -1;
    }
    let a = e.vcall(first, HEALTH_SLOT, &args![]).u32();
    let b = e.vcall(second, HEALTH_SLOT, &args![]).u32();
    i32::from(a > b)
}

/// Whether there is an item and its form's type (`00401170` of `0044ddc0`, in place) is one of
/// `types`.
#[inline]
fn form_type_is(e: &Engine, item: u32, types: &[u8]) -> bool {
    if item == 0 {
        return false;
    }
    let form = e.mem.u32(item.wrapping_add(ITEM_FORM));
    let kind = e.mem.u8(form.wrapping_add(FORM_TYPE));
    types.contains(&kind)
}

// Typed from 00719da0
/// A list filter (cdecl, one item): whether there is an item and its form's type is 0x28.
pub fn fn_00719da0(e: &mut Engine, item: u32) -> bool {
    form_type_is(e, item, &[0x28])
}

// Typed from 00719de0
/// A list filter (cdecl, one item): whether there is an item and its form's type is 0x1a or 0x18.
pub fn fn_00719de0(e: &mut Engine, item: u32) -> bool {
    form_type_is(e, item, &[0x1a, 0x18])
}

// Typed from 00719e30
/// A list filter (cdecl, one item): whether there is an item and its form's type is one of 0x1f,
/// 0x29, 0x67, 0x32, 0x6c, 0x74, 0x73.
pub fn fn_00719e30(e: &mut Engine, item: u32) -> bool {
    form_type_is(e, item, &[0x1f, 0x29, 0x67, 0x32, 0x6c, 0x74, 0x73])
}

/// The file's functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        engine_core::entry!(0x0071_a200, list_box_remove_item_group(Ptr, u32)),
        engine_core::entry!(
            0x0071_a320,
            list_box_add_item(Ptr, u32, Ptr, u32, Ptr) -> Ptr
        ),
        engine_core::entry!(0x0071_a5c0, list_box_remove_item(Ptr, Ptr) -> bool),
        engine_core::entry!(0x0071_a670, list_box_synchronize(Ptr)),
        engine_core::entry!(0x0071_a740, list_box_update_highlight_box_pos(Ptr)),
        engine_core::entry!(0x0071_aa60, fn_0071aa60(Ptr, u8)),
        engine_core::entry!(0x0071_aa80, fn_0071aa80(Ptr) -> u8),
        engine_core::entry!(0x0071_aaa0, fn_0071aaa0(Ptr, u32, u32) -> bool),
        engine_core::entry!(0x0071_ad30, list_box_set_total_list_height(Ptr)),
        engine_core::entry!(0x0071_ae60, list_box_num_unfiltered(Ptr) -> i32),
        engine_core::entry!(0x0071_9be0, fn_00719be0(Ptr)),
        engine_core::entry!(0x0071_9c00, fn_00719c00(Ptr, u32, u32) -> Ptr),
        engine_core::entry!(0x0071_9ef0, fn_00719ef0(Ptr, u32, u32, u32, u32, u8, u8)),
        engine_core::entry!(0x0071_9c80, fn_00719c80(u32, u32) -> i32),
        engine_core::entry!(0x0071_9ca0, fn_00719ca0(u32, u32) -> i32),
        engine_core::entry!(0x0071_9cf0, fn_00719cf0(u32, u32) -> i32),
        engine_core::entry!(0x0071_9da0, fn_00719da0(u32) -> bool),
        engine_core::entry!(0x0071_9de0, fn_00719de0(u32) -> bool),
        engine_core::entry!(0x0071_9e30, fn_00719e30(u32) -> bool),
    ]
}
