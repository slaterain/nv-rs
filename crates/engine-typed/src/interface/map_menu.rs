//! The map menu unit (the Xbox PDB's `fallout/interface/menus/mapmenu` source), its first part: the
//! five `ListBox<T *>` instances the Pip-Boy's data pages hold (`TESQuest`, `BGSNote`,
//! `TESObjectREFR`, `BGSQuestObjective`, `TESChallenge`), the list box helpers emitted with them, and
//! the `BSSimpleArray<BSStringT<char>,1024>` the menu keeps its strings in. The translation is
//! engine-fallout-interface's `mapmenu.rs`, which these functions replace at the same addresses.
//!
//! - Each instance's constructor, destructor, `RemoveAll`, `Sort`, `Highlight`,
//!   `GetHighlightTarget`, `Synchronize`, `IsEnabled`, `UpdateHighlightBoxPos`,
//!   `SetTotalListHeight`, `NumUnfiltered`, `Insert` and deleting destructor are the template's
//!   bodies ([`super::menu_manager_p2`]'s, over an [`Of`]; `Insert` is the stats menu's
//!   [`super::stats_menu`] `AddItem` body). Four instances also have `SetEnabled`, four `Filter` and
//!   the last unfiltered tile; the notes' list saves and restores its state.
//! - The list box helpers: the highlighted item's data, flag 0 and any flag set or cleared, the
//!   container's trait `0xfa3`, the `ListBoxItem` constructor.
//! - The string array: its constructor and set-up, destructor and release, the elements' construction
//!   and destruction, the append of a string and the deleting destructors of the array and of a
//!   string.
//!
//! **Native.** The exe's order of work. The calls of functions no family types stay calls by address
//! (the trait ids by name, the value calls the tile family does not type, the item tile's position,
//! the focus, the menu's and menu tile's look-ups, the manager's state test, the array's slot append,
//! the string's assignment and destructor, a filter's callback). The tile's getters and setters are
//! the dispatch family's, `_ftol2` the C runtime's, the array's buffer free the pathing family's; the
//! instances' own methods, the list box's item count and the item data look-up are this family's
//! (all through `typed_call!`). Done in place: the placement `operator new` and `BSString`'s
//! constructor (the interface unit's lists), the smaller of two floats, a list box flag set or
//! cleared, the `ListBoxItem` constructor. The list box's `GetTileAt` and `Highlight` (vtable slots
//! `+0x14` and `+0`), the array's `Allocate` (`+4`) and a menu's `HandleClick`-style slot `+0x14` stay
//! virtual: the object's class is not known here.
//!
//! **Floats.** The main thread's: the restore's last index (the count, a whole number, less 1.0) is
//! the float result; the saved values are stored as read.

use super::interface_cpp::in_place;
use super::menu_manager_p2::{
    self as template, first_node, ftol2, set_flag, static_trait, tile_float, tile_set_int,
    trait_by_name, Of, CONTAINER, ENABLED_TEXT, FREE, HIGHLIGHTED, INIT_TILE_POSITION, ITEM_DATA,
    ITEM_FILTERED, NEXT_INDEX, NUM_FILTERED_TEXT, SAVED_HIGHLIGHT_INDEX, SAVED_SCROLLBAR_VALUE,
    SCROLL_BAR, TOTAL_HEIGHT, TRAIT_INDEX,
};
use crate::{dispatch, pathing};
use engine_core::prelude::*;
use engine_core::typed_call;
use engine_core::x87::fistp_chop64;

/// The tile's id trait (`0xfaa`), which the restore hands the menu.
const TRAIT_ID: i32 = 0xfaa;
/// The container's trait `007a5a30` sets.
const TRAIT_FA3: i32 = 0xfa3;
/// `Tile::SetIntegerValue`-style setter (`00715c60`, thiscall: trait, count) `Filter` gives the
/// container's `"_num_filtered"` with.
const TILE_SET_COUNT: u32 = 0x0071_5c60;
/// `"_current_value"` (the scroll bar's) and `"_SetInCode"`.
const CURRENT_VALUE_TEXT: u32 = 0x0107_02d8;
const SET_IN_CODE_TEXT: u32 = 0x0107_0250;
/// `Tile::GetMenuTile` and `Tile::GetMenu` (thiscall).
const TILE_GET_MENU_TILE: u32 = 0x00a0_3d40;
const TILE_GET_MENU: u32 = 0x00a0_3c90;
/// `InterfaceManager::SetCurrentFocusTarget` (thiscall: tile, trait, word) and the trait it takes.
const SET_FOCUS_TARGET: u32 = 0x0071_5860;
const TRAIT_FOCUS: u32 = 0xfc3;
/// The manager's state test the restore leaves early on (cdecl, `AL`).
const MANAGER_STATE_TEST: u32 = 0x004b_71d0;
/// The vtable slots the code calls on the list box: `Highlight` (+0) and `GetTileAt` (+0x14, the
/// index and 0); on the menu the restore calls slot +0x14 (the tile's id, the tile).
const HIGHLIGHT_SLOT: u32 = 0;
const TILE_AT_SLOT: u32 = 0x14;
const MENU_SLOT: u32 = 0x14;

/// `BSSimpleArray<BSStringT<char>,1024>`'s vtable (RTTI), its buffer (+4), size (+8) and reserved
/// size (+0xc); its `Allocate` slot (+4); a string's size (8 bytes).
const STRING_ARRAY_VTABLE: u32 = 0x0107_5530;
const BUFFER: u32 = 4;
const SIZE: u32 = 8;
const RESERVED: u32 = 0xc;
const ALLOCATE_SLOT: u32 = 4;
const STRING: u32 = 8;
/// The array's append of a slot (`00845460`, thiscall: the slot's index), `BSStringT::Set`
/// (`00501c10`, thiscall: the text) and `BSString`'s destructor (`004037d0`, thiscall).
const ADD_SLOT: u32 = 0x0084_5460;
const STRING_SET: u32 = 0x0050_1c10;
const STRING_DESTRUCT: u32 = 0x0040_37d0;

/// The function-local statics' guard words (each id 4 bytes below its guard, bit 0): the four
/// `SetEnabled`s' `"_enabled"` and the four `Filter`s' `"_num_filtered"`.
const QUEST_SET_ENABLED: u32 = 0x011d_a420;
const NOTE_SET_ENABLED: u32 = 0x011d_a430;
const OBJECTIVE_SET_ENABLED: u32 = 0x011d_a448;
const CHALLENGE_SET_ENABLED: u32 = 0x011d_a458;
const NOTE_FILTER: u32 = 0x011d_a428;
const REFERENCE_FILTER: u32 = 0x011d_a438;
const OBJECTIVE_FILTER: u32 = 0x011d_a440;
const CHALLENGE_FILTER: u32 = 0x011d_a450;

// ---- The template's methods these instances add ------------------------------------------------

/// `ListBox<T>::SetEnabled` (Xbox PDB): the container's `"_enabled"` trait (a function-local static
/// id, the guard `guard`) set to `value`; the container is not checked.
fn set_enabled(e: &mut Engine, guard: u32, this: Ptr, value: u8) {
    let enabled = static_trait(e, guard, 1, guard.wrapping_sub(4), ENABLED_TEXT);
    let container = e.mem.u32(this.addr().wrapping_add(CONTAINER));
    tile_set_int(e, container, enabled, i32::from(value));
}

/// The tile of the last unfiltered item (`007a6220` and the three like it): `NumUnfiltered`'s body
/// (inlined there, with the instance's static) less one, through `GetTileAt` with 0.
fn last_tile(e: &mut Engine, of: Of, this: Ptr) -> u32 {
    let count = template::num_unfiltered(e, of, this);
    e.vcall(
        this.addr(),
        TILE_AT_SLOT,
        &args![count.wrapping_sub(1), 0u32],
    )
    .u32()
}

/// `ListBox<T>::Filter` (Xbox PDB): the index and height cleared; each item marked filtered when
/// `callback` (cdecl, the item's data) answers true (none: no item is). A filtered item's tile gets
/// index -1 and is counted, and when it was the highlighted one its index is kept and the highlight
/// cleared; an unfiltered one becomes the highlight when its new index is the kept one, then gets
/// the next index and its position. Then the container's `"_num_filtered"` (a function-local static
/// id, the guard `guard`) is the count, the height is recomputed, a lost highlight with a kept index
/// goes to the tile at that index, else to the last unfiltered one (`last`), and the highlight box
/// moves. The item is read again after each call, as the code does.
fn filter(
    e: &mut Engine,
    of: Of,
    guard: u32,
    this: Ptr,
    callback: u32,
    last: fn(&mut Engine, Ptr) -> u32,
) {
    let at = this.addr();
    let mut filtered = 0u32;
    let mut kept = -1i32;
    static_trait(e, guard, 1, guard.wrapping_sub(4), NUM_FILTERED_TEXT);
    e.mem.set_u16(at.wrapping_add(NEXT_INDEX), 0);
    e.mem.set_f32(at.wrapping_add(TOTAL_HEIGHT), 0.0);
    let mut node = first_node(at);
    while node != 0 && e.mem.u32(node) != 0 {
        let out = callback != 0 && {
            let data = e.mem.u32(e.mem.u32(node).wrapping_add(ITEM_DATA));
            e.call(callback, &args![data]).bool()
        };
        let item = e.mem.u32(node);
        e.mem
            .set_u8(item.wrapping_add(ITEM_FILTERED), u8::from(out));
        let tile = e.mem.u32(item);
        if out {
            if tile == e.mem.u32(at.wrapping_add(HIGHLIGHTED)) {
                let index = tile_float(e, tile, TRAIT_INDEX);
                kept = ftol2(e, f64::from(index));
                e.mem.set_u32(at.wrapping_add(HIGHLIGHTED), 0);
            }
            let tile = e.mem.u32(e.mem.u32(node));
            tile_set_int(e, tile, TRAIT_INDEX, -1);
            filtered = filtered.wrapping_add(1);
        } else {
            let index = e.mem.u16(at.wrapping_add(NEXT_INDEX));
            if kept >= 0 && i32::from(index) == kept {
                e.mem.set_u32(at.wrapping_add(HIGHLIGHTED), tile);
            }
            e.mem
                .set_u16(at.wrapping_add(NEXT_INDEX), index.wrapping_add(1));
            tile_set_int(e, tile, TRAIT_INDEX, i32::from(index));
            let tile = e.mem.u32(e.mem.u32(node));
            e.call(INIT_TILE_POSITION, &args![this, tile]);
        }
        node = e.mem.u32(node.wrapping_add(4));
    }
    let num_filtered = e.mem.u32(guard.wrapping_sub(4));
    let container = e.mem.u32(at.wrapping_add(CONTAINER));
    e.call(TILE_SET_COUNT, &args![container, num_filtered, filtered]);
    template::set_total_list_height_of(e, of, this);
    if kept >= 0 && e.mem.u32(at.wrapping_add(HIGHLIGHTED)) == 0 {
        let tile = e.vcall(at, TILE_AT_SLOT, &args![kept, 0u32]).u32();
        e.mem.set_u32(at.wrapping_add(HIGHLIGHTED), tile);
    }
    if kept >= 0 && e.mem.u32(at.wrapping_add(HIGHLIGHTED)) == 0 {
        let tile = last(e, this);
        e.mem.set_u32(at.wrapping_add(HIGHLIGHTED), tile);
    }
    template::update_highlight_box_pos_of(e, of, this);
}

/// A deleting destructor's tail: the free when bit 0 of `flags` is set. `this`.
#[inline]
fn free_if_flagged(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    if flags & 1 != 0 {
        e.call(FREE, &args![this]);
    }
    this
}

// ---- ListBox<TESQuest *> ---------------------------------------------------------------------

// Typed from 007a1c90
/// The `ListBox<TESQuest *>` constructor. `this`.
pub fn fn_007a1c90(e: &mut Engine, this: Ptr) -> Ptr {
    template::construct(e, Of::Quest, this)
}

// Typed from 007a1d60
/// The `ListBox<TESQuest *>` destructor.
pub fn fn_007a1d60(e: &mut Engine, this: Ptr) {
    template::destruct(e, Of::Quest, this);
}

// Typed from 007a1dd0
/// `ListBox<TESQuest *>::Insert`: the template's `AddItem`. The new tile, or 0.
pub fn fn_007a1dd0(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    text: u32,
    compare: u32,
    template: u32,
) -> u32 {
    super::stats_menu::add_item_of(e, Of::Quest, this, data, text, compare, template)
}

// Typed from 007a2070
/// `ListBox<TESQuest_P>::RemoveAll` (engine map).
pub fn list_box_tes_quest_remove_all(e: &mut Engine, this: Ptr) {
    template::remove_all(e, Of::Quest, this);
}

// Typed from 007a22a0
/// `ListBox<TESQuest_P>::Sort` (Xbox PDB).
pub fn list_box_tes_quest_sort(e: &mut Engine, this: Ptr, compare: u32) {
    template::sort(e, Of::Quest, this, compare);
}

// Typed from 007a2350
/// `ListBox<TESQuest_P>::SetEnabled` (Xbox PDB).
pub fn list_box_tes_quest_set_enabled(e: &mut Engine, this: Ptr, value: u8) {
    set_enabled(e, QUEST_SET_ENABLED, this, value);
}

// Typed from 007a23e0
/// `ListBox<TESQuest_P>::Highlight` (engine map).
pub fn list_box_tes_quest_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    template::highlight(e, Of::Quest, this, tile)
}

// Typed from 007a2450
/// `ListBox<TESQuest_P>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_tes_quest_get_highlight_target(e: &mut Engine, this: Ptr, direction: u32) -> u32 {
    template::get_highlight_target(e, Of::Quest, this, direction)
}

// Typed from 007a6130
/// `ListBox<TESQuest_P>`'s scalar deleting destructor (engine map). `this`.
pub fn fn_007a6130(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x007a_1d60, fn_007a1d60(e, this));
    free_if_flagged(e, this, flags)
}

// Typed from 007a6480
/// `ListBox<TESQuest_P>::Synchronize` (Xbox PDB).
pub fn list_box_tes_quest_synchronize(e: &mut Engine, this: Ptr) {
    template::synchronize(e, Of::Quest, this);
}

// Typed from 007a6550
/// `ListBox<TESQuest_P>::IsEnabled` (engine map).
pub fn list_box_tes_quest_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    template::is_enabled(e, Of::Quest, this)
}

// Typed from 007a6600
/// `ListBox<TESQuest_P>::UpdateHighlightBoxPos` (engine map).
pub fn list_box_tes_quest_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    template::update_highlight_box_pos(e, Of::Quest, this);
}

// Typed from 007a6920
/// `ListBox<TESQuest_P>::SetTotalListHeight` (engine map).
pub fn list_box_tes_quest_set_total_list_height(e: &mut Engine, this: Ptr) {
    template::set_total_list_height(e, Of::Quest, this);
}

// Typed from 007a86e0
/// `ListBox<TESQuest_P>::NumUnfiltered` (Xbox PDB).
pub fn list_box_tes_quest_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    template::num_unfiltered(e, Of::Quest, this)
}

// ---- ListBox<BGSNote *> ----------------------------------------------------------------------

// Typed from 007a27f0
/// The `ListBox<BGSNote *>` constructor. `this`.
pub fn fn_007a27f0(e: &mut Engine, this: Ptr) -> Ptr {
    template::construct(e, Of::Note, this)
}

// Typed from 007a28c0
/// The `ListBox<BGSNote *>` destructor.
pub fn fn_007a28c0(e: &mut Engine, this: Ptr) {
    template::destruct(e, Of::Note, this);
}

// Typed from 007a2930
/// `ListBox<BGSNote *>::Insert`: the template's `AddItem`. The new tile, or 0.
pub fn fn_007a2930(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    text: u32,
    compare: u32,
    template: u32,
) -> u32 {
    super::stats_menu::add_item_of(e, Of::Note, this, data, text, compare, template)
}

// Typed from 007a2bd0
/// `ListBox<BGSNote_P>::RemoveAll` (engine map).
pub fn list_box_bgs_note_remove_all(e: &mut Engine, this: Ptr) {
    template::remove_all(e, Of::Note, this);
}

// Typed from 007a2e00
/// `ListBox<BGSNote_P>::Sort` (Xbox PDB).
pub fn list_box_bgs_note_sort(e: &mut Engine, this: Ptr, compare: u32) {
    template::sort(e, Of::Note, this, compare);
}

// Typed from 007a2e30
/// `ListBox<BGSNote_P>::Filter` (Xbox PDB).
pub fn list_box_bgs_note_filter(e: &mut Engine, this: Ptr, callback: u32) {
    filter(e, Of::Note, NOTE_FILTER, this, callback, note_last_tile);
}

// Typed from 007a30e0
/// `ListBox<BGSNote_P>::SetEnabled` (Xbox PDB).
pub fn list_box_bgs_note_set_enabled(e: &mut Engine, this: Ptr, value: u8) {
    set_enabled(e, NOTE_SET_ENABLED, this, value);
}

// Typed from 007a3170
/// `ListBox<BGSNote_P>::Highlight` (engine map).
pub fn list_box_bgs_note_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    template::highlight(e, Of::Note, this, tile)
}

// Typed from 007a31e0
/// `ListBox<BGSNote_P>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_bgs_note_get_highlight_target(e: &mut Engine, this: Ptr, direction: u32) -> u32 {
    template::get_highlight_target(e, Of::Note, this, direction)
}

// Typed from 007312e0
/// `ListBox<BGSNote_P>::SaveListState` (engine map): the highlighted tile's index (trait `0xfac`;
/// -1.0 without one) and the scroll bar's `"_current_value"` (-1.0 without a scroll bar) kept.
pub fn list_box_bgs_note_save_list_state(e: &mut Engine, this: Ptr) {
    let at = this.addr();
    let highlighted = e.mem.u32(at.wrapping_add(HIGHLIGHTED));
    let index = if highlighted == 0 {
        -1.0
    } else {
        tile_float(e, highlighted, TRAIT_INDEX)
    };
    e.mem.set_f32(at.wrapping_add(SAVED_HIGHLIGHT_INDEX), index);
    let value = if e.mem.u32(at.wrapping_add(SCROLL_BAR)) == 0 {
        -1.0
    } else {
        let current_value = trait_by_name(e, CURRENT_VALUE_TEXT);
        let scroll_bar = e.mem.u32(at.wrapping_add(SCROLL_BAR));
        tile_float(e, scroll_bar, current_value)
    };
    e.mem.set_f32(at.wrapping_add(SAVED_SCROLLBAR_VALUE), value);
}

// Typed from 007a3580
/// The notes' list's state restored (unnamed in the engine map): with a container and a scroll bar,
/// the saved scroll value back as the scroll bar's `"_current_value"` (flag 0) and its
/// `"_SetInCode"` poked with 1.0; the tile at the saved index (at most the last unfiltered one,
/// truncated) gets the focus with `focus` and is highlighted; unless the manager's state test
/// answers, the menu's slot +0x14 is told the tile's id and the tile (when there is one), and the
/// highlight is cleared.
pub fn fn_007a3580(e: &mut Engine, this: Ptr, focus: u8) {
    let at = this.addr();
    if e.mem.u32(at.wrapping_add(CONTAINER)) == 0 || e.mem.u32(at.wrapping_add(SCROLL_BAR)) == 0 {
        return;
    }
    let saved = e.mem.f32(at.wrapping_add(SAVED_SCROLLBAR_VALUE));
    let current_value = trait_by_name(e, CURRENT_VALUE_TEXT);
    let scroll_bar = e.mem.u32(at.wrapping_add(SCROLL_BAR));
    typed_call!(
        dispatch::FAMILY,
        0x00a0_12d0,
        dispatch::tile::set_float(e, Ptr::new(scroll_bar), current_value, saved, 0)
    );
    let set_in_code = trait_by_name(e, SET_IN_CODE_TEXT);
    let scroll_bar = e.mem.u32(at.wrapping_add(SCROLL_BAR));
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1290,
        dispatch::tile::poke(e, Ptr::new(scroll_bar), set_in_code, 1.0)
    );
    let count = typed_call!(super::FAMILY, 0x007a_6a50, fn_007a6a50(e, this));
    // The count a whole number (`FILD qword`, its high word zero) less the double 1.0: the float.
    let last = super::narrow(f64::from(super::bits(count)) - 1.0);
    let index = e.mem.f32(at.wrapping_add(SAVED_HIGHLIGHT_INDEX));
    // `0040ebd0` (in place): the last unless it is above the saved index; truncated by `FISTP qword`.
    let chosen = if last > index { index } else { last };
    let chosen = super::bits(super::low_dword_signed(fistp_chop64(f64::from(chosen))));
    let tile = e.vcall(at, TILE_AT_SLOT, &args![chosen, 0u32]).u32();
    let manager = in_place::manager(e);
    e.call(
        SET_FOCUS_TARGET,
        &args![manager, tile, TRAIT_FOCUS, u32::from(focus)],
    );
    e.vcall(at, HIGHLIGHT_SLOT, &args![tile]);
    if e.call(MANAGER_STATE_TEST, &[]).bool() {
        return;
    }
    if tile != 0 {
        let container = e.mem.u32(at.wrapping_add(CONTAINER));
        let menu_tile = e.call(TILE_GET_MENU_TILE, &args![container]).u32();
        let menu = e.call(TILE_GET_MENU, &args![menu_tile]).u32();
        let id = tile_float(e, tile, TRAIT_ID);
        let id = ftol2(e, f64::from(id));
        e.vcall(menu, MENU_SLOT, &args![id, tile]);
    }
    e.vcall(at, HIGHLIGHT_SLOT, &args![0u32]);
}

// Typed from 007a6160
/// `ListBox<BGSNote_P>`'s scalar deleting destructor (engine map). `this`.
pub fn fn_007a6160(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x007a_28c0, fn_007a28c0(e, this));
    free_if_flagged(e, this, flags)
}

// Typed from 007a6220
/// The `ListBox<BGSNote *>`'s last unfiltered tile.
pub fn fn_007a6220(e: &mut Engine, this: Ptr) -> u32 {
    last_tile(e, Of::Note, this)
}

/// [`fn_007a6220`], as `Filter` calls it.
fn note_last_tile(e: &mut Engine, this: Ptr) -> u32 {
    typed_call!(super::FAMILY, 0x007a_6220, fn_007a6220(e, this))
}

// Typed from 007a6a50
/// `ListBox<BGSNote *>::NumUnfiltered` (by the body).
pub fn fn_007a6a50(e: &mut Engine, this: Ptr) -> i32 {
    template::num_unfiltered(e, Of::Note, this)
}

// Typed from 007a6b00
/// `ListBox<BGSNote_P>::Synchronize` (Xbox PDB).
pub fn list_box_bgs_note_synchronize(e: &mut Engine, this: Ptr) {
    template::synchronize(e, Of::Note, this);
}

// Typed from 007a6bd0
/// `ListBox<BGSNote_P>::IsEnabled` (engine map).
pub fn list_box_bgs_note_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    template::is_enabled(e, Of::Note, this)
}

// Typed from 007a6c80
/// `ListBox<BGSNote_P>::UpdateHighlightBoxPos` (engine map).
pub fn list_box_bgs_note_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    template::update_highlight_box_pos(e, Of::Note, this);
}

// Typed from 007a7080
/// `ListBox<BGSNote_P>::SetTotalListHeight` (engine map).
pub fn list_box_bgs_note_set_total_list_height(e: &mut Engine, this: Ptr) {
    template::set_total_list_height(e, Of::Note, this);
}

// ---- ListBox<TESObjectREFR *> ----------------------------------------------------------------

// Typed from 007a36e0
/// The `ListBox<TESObjectREFR *>` constructor. `this`.
pub fn fn_007a36e0(e: &mut Engine, this: Ptr) -> Ptr {
    template::construct(e, Of::Reference, this)
}

// Typed from 007a37b0
/// The `ListBox<TESObjectREFR *>` destructor.
pub fn fn_007a37b0(e: &mut Engine, this: Ptr) {
    template::destruct(e, Of::Reference, this);
}

// Typed from 007a3820
/// `ListBox<TESObjectREFR *>::Insert`: the template's `AddItem`. The new tile, or 0.
pub fn fn_007a3820(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    text: u32,
    compare: u32,
    template: u32,
) -> u32 {
    super::stats_menu::add_item_of(e, Of::Reference, this, data, text, compare, template)
}

// Typed from 007a3ac0
/// `ListBox<TESObjectREFR_P>::RemoveAll` (engine map).
pub fn list_box_tes_object_refr_remove_all(e: &mut Engine, this: Ptr) {
    template::remove_all(e, Of::Reference, this);
}

// Typed from 007a3cf0
/// `ListBox<TESObjectREFR_P>::Sort` (Xbox PDB).
pub fn list_box_tes_object_refr_sort(e: &mut Engine, this: Ptr, compare: u32) {
    template::sort(e, Of::Reference, this, compare);
}

// Typed from 007a3d20
/// `ListBox<TESObjectREFR_P>::Filter` (Xbox PDB).
pub fn list_box_tes_object_refr_filter(e: &mut Engine, this: Ptr, callback: u32) {
    filter(
        e,
        Of::Reference,
        REFERENCE_FILTER,
        this,
        callback,
        reference_last_tile,
    );
}

// Typed from 007a3f90
/// `ListBox<TESObjectREFR_P>::Highlight` (engine map).
pub fn list_box_tes_object_refr_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    template::highlight(e, Of::Reference, this, tile)
}

// Typed from 007a4000
/// `ListBox<TESObjectREFR_P>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_tes_object_refr_get_highlight_target(
    e: &mut Engine,
    this: Ptr,
    direction: u32,
) -> u32 {
    template::get_highlight_target(e, Of::Reference, this, direction)
}

// Typed from 007a6190
/// `ListBox<TESObjectREFR_P>`'s scalar deleting destructor (engine map). `this`.
pub fn fn_007a6190(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x007a_37b0, fn_007a37b0(e, this));
    free_if_flagged(e, this, flags)
}

// Typed from 007a6250
/// The `ListBox<TESObjectREFR *>`'s last unfiltered tile.
pub fn fn_007a6250(e: &mut Engine, this: Ptr) -> u32 {
    last_tile(e, Of::Reference, this)
}

/// [`fn_007a6250`], as `Filter` calls it.
fn reference_last_tile(e: &mut Engine, this: Ptr) -> u32 {
    typed_call!(super::FAMILY, 0x007a_6250, fn_007a6250(e, this))
}

// Typed from 007a71b0
/// `ListBox<TESObjectREFR_P>::NumUnfiltered` (Xbox PDB).
pub fn list_box_tes_object_refr_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    template::num_unfiltered(e, Of::Reference, this)
}

// Typed from 007a7260
/// `ListBox<TESObjectREFR_P>::Synchronize` (Xbox PDB).
pub fn list_box_tes_object_refr_synchronize(e: &mut Engine, this: Ptr) {
    template::synchronize(e, Of::Reference, this);
}

// Typed from 007a7330
/// `ListBox<TESObjectREFR_P>::IsEnabled` (engine map).
pub fn list_box_tes_object_refr_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    template::is_enabled(e, Of::Reference, this)
}

// Typed from 007a73e0
/// `ListBox<TESObjectREFR_P>::UpdateHighlightBoxPos` (engine map).
pub fn list_box_tes_object_refr_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    template::update_highlight_box_pos(e, Of::Reference, this);
}

// Typed from 007a7700
/// `ListBox<TESObjectREFR_P>::SetTotalListHeight` (engine map).
pub fn list_box_tes_object_refr_set_total_list_height(e: &mut Engine, this: Ptr) {
    template::set_total_list_height(e, Of::Reference, this);
}

// ---- ListBox<BGSQuestObjective *> ------------------------------------------------------------

// Typed from 007a43a0
/// The `ListBox<BGSQuestObjective *>` constructor. `this`.
pub fn fn_007a43a0(e: &mut Engine, this: Ptr) -> Ptr {
    template::construct(e, Of::Objective, this)
}

// Typed from 007a4470
/// The `ListBox<BGSQuestObjective *>` destructor.
pub fn fn_007a4470(e: &mut Engine, this: Ptr) {
    template::destruct(e, Of::Objective, this);
}

// Typed from 007a44e0
/// `ListBox<BGSQuestObjective *>::Insert`: the template's `AddItem`. The new tile, or 0.
pub fn fn_007a44e0(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    text: u32,
    compare: u32,
    template: u32,
) -> u32 {
    super::stats_menu::add_item_of(e, Of::Objective, this, data, text, compare, template)
}

// Typed from 007a4780
/// `ListBox<BGSQuestObjective_P>::RemoveAll` (engine map).
pub fn list_box_bgs_quest_objective_remove_all(e: &mut Engine, this: Ptr) {
    template::remove_all(e, Of::Objective, this);
}

// Typed from 007a49b0
/// `ListBox<BGSQuestObjective_P>::Sort` (Xbox PDB).
pub fn list_box_bgs_quest_objective_sort(e: &mut Engine, this: Ptr, compare: u32) {
    template::sort(e, Of::Objective, this, compare);
}

// Typed from 007a49e0
/// `ListBox<BGSQuestObjective_P>::Filter` (Xbox PDB).
pub fn list_box_bgs_quest_objective_filter(e: &mut Engine, this: Ptr, callback: u32) {
    filter(
        e,
        Of::Objective,
        OBJECTIVE_FILTER,
        this,
        callback,
        objective_last_tile,
    );
}

// Typed from 007a4c50
/// `ListBox<BGSQuestObjective_P>::SetEnabled` (Xbox PDB).
pub fn list_box_bgs_quest_objective_set_enabled(e: &mut Engine, this: Ptr, value: u8) {
    set_enabled(e, OBJECTIVE_SET_ENABLED, this, value);
}

// Typed from 007a4ce0
/// `ListBox<BGSQuestObjective_P>::Highlight` (engine map).
pub fn list_box_bgs_quest_objective_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    template::highlight(e, Of::Objective, this, tile)
}

// Typed from 007a4d50
/// `ListBox<BGSQuestObjective_P>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_bgs_quest_objective_get_highlight_target(
    e: &mut Engine,
    this: Ptr,
    direction: u32,
) -> u32 {
    template::get_highlight_target(e, Of::Objective, this, direction)
}

// Typed from 007a61c0
/// `ListBox<BGSQuestObjective_P>`'s scalar deleting destructor (engine map). `this`.
pub fn fn_007a61c0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x007a_4470, fn_007a4470(e, this));
    free_if_flagged(e, this, flags)
}

// Typed from 007a6280
/// The `ListBox<BGSQuestObjective *>`'s last unfiltered tile.
pub fn fn_007a6280(e: &mut Engine, this: Ptr) -> u32 {
    last_tile(e, Of::Objective, this)
}

/// [`fn_007a6280`], as `Filter` calls it.
fn objective_last_tile(e: &mut Engine, this: Ptr) -> u32 {
    typed_call!(super::FAMILY, 0x007a_6280, fn_007a6280(e, this))
}

// Typed from 007a7830
/// `ListBox<BGSQuestObjective *>::NumUnfiltered` (by the body).
pub fn fn_007a7830(e: &mut Engine, this: Ptr) -> i32 {
    template::num_unfiltered(e, Of::Objective, this)
}

// Typed from 007a78e0
/// `ListBox<BGSQuestObjective_P>::Synchronize` (Xbox PDB).
pub fn list_box_bgs_quest_objective_synchronize(e: &mut Engine, this: Ptr) {
    template::synchronize(e, Of::Objective, this);
}

// Typed from 007a79b0
/// `ListBox<BGSQuestObjective_P>::IsEnabled` (engine map).
pub fn list_box_bgs_quest_objective_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    template::is_enabled(e, Of::Objective, this)
}

// Typed from 007a7a60
/// `ListBox<BGSQuestObjective_P>::UpdateHighlightBoxPos` (engine map).
pub fn list_box_bgs_quest_objective_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    template::update_highlight_box_pos(e, Of::Objective, this);
}

// Typed from 007a7d80
/// `ListBox<BGSQuestObjective_P>::SetTotalListHeight` (engine map).
pub fn list_box_bgs_quest_objective_set_total_list_height(e: &mut Engine, this: Ptr) {
    template::set_total_list_height(e, Of::Objective, this);
}

// ---- ListBox<TESChallenge *> -----------------------------------------------------------------

// Typed from 007a50f0
/// The `ListBox<TESChallenge *>` constructor. `this`.
pub fn fn_007a50f0(e: &mut Engine, this: Ptr) -> Ptr {
    template::construct(e, Of::Challenge, this)
}

// Typed from 007a51c0
/// The `ListBox<TESChallenge *>` destructor.
pub fn fn_007a51c0(e: &mut Engine, this: Ptr) {
    template::destruct(e, Of::Challenge, this);
}

// Typed from 007a5230
/// `ListBox<TESChallenge *>::Insert`: the template's `AddItem`. The new tile, or 0.
pub fn fn_007a5230(
    e: &mut Engine,
    this: Ptr,
    data: u32,
    text: u32,
    compare: u32,
    template: u32,
) -> u32 {
    super::stats_menu::add_item_of(e, Of::Challenge, this, data, text, compare, template)
}

// Typed from 007a54d0
/// `ListBox<TESChallenge_P>::RemoveAll` (engine map).
pub fn list_box_tes_challenge_remove_all(e: &mut Engine, this: Ptr) {
    template::remove_all(e, Of::Challenge, this);
}

// Typed from 007a5700
/// `ListBox<TESChallenge_P>::Sort` (Xbox PDB).
pub fn list_box_tes_challenge_sort(e: &mut Engine, this: Ptr, compare: u32) {
    template::sort(e, Of::Challenge, this, compare);
}

// Typed from 007a5730
/// `ListBox<TESChallenge_P>::Filter` (Xbox PDB).
pub fn list_box_tes_challenge_filter(e: &mut Engine, this: Ptr, callback: u32) {
    filter(
        e,
        Of::Challenge,
        CHALLENGE_FILTER,
        this,
        callback,
        challenge_last_tile,
    );
}

// Typed from 007a59a0
/// `ListBox<TESChallenge_P>::SetEnabled` (Xbox PDB).
pub fn list_box_tes_challenge_set_enabled(e: &mut Engine, this: Ptr, value: u8) {
    set_enabled(e, CHALLENGE_SET_ENABLED, this, value);
}

// Typed from 007a5a60
/// `ListBox<TESChallenge_P>::Highlight` (engine map).
pub fn list_box_tes_challenge_highlight(e: &mut Engine, this: Ptr, tile: u32) -> bool {
    template::highlight(e, Of::Challenge, this, tile)
}

// Typed from 007a5ad0
/// `ListBox<TESChallenge_P>::GetHighlightTarget` (Xbox PDB).
pub fn list_box_tes_challenge_get_highlight_target(
    e: &mut Engine,
    this: Ptr,
    direction: u32,
) -> u32 {
    template::get_highlight_target(e, Of::Challenge, this, direction)
}

// Typed from 007a61f0
/// `ListBox<TESChallenge_P>`'s scalar deleting destructor (engine map). `this`.
pub fn fn_007a61f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x007a_51c0, fn_007a51c0(e, this));
    free_if_flagged(e, this, flags)
}

// Typed from 007a62b0
/// The `ListBox<TESChallenge *>`'s last unfiltered tile.
pub fn fn_007a62b0(e: &mut Engine, this: Ptr) -> u32 {
    last_tile(e, Of::Challenge, this)
}

/// [`fn_007a62b0`], as `Filter` calls it.
fn challenge_last_tile(e: &mut Engine, this: Ptr) -> u32 {
    typed_call!(super::FAMILY, 0x007a_62b0, fn_007a62b0(e, this))
}

// Typed from 007a8030
/// `ListBox<TESChallenge_P>::NumUnfiltered` (Xbox PDB).
pub fn list_box_tes_challenge_num_unfiltered(e: &mut Engine, this: Ptr) -> i32 {
    template::num_unfiltered(e, Of::Challenge, this)
}

// Typed from 007a80e0
/// `ListBox<TESChallenge_P>::Synchronize` (Xbox PDB).
pub fn list_box_tes_challenge_synchronize(e: &mut Engine, this: Ptr) {
    template::synchronize(e, Of::Challenge, this);
}

// Typed from 007a81b0
/// `ListBox<TESChallenge_P>::IsEnabled` (engine map).
pub fn list_box_tes_challenge_is_enabled(e: &mut Engine, this: Ptr) -> bool {
    template::is_enabled(e, Of::Challenge, this)
}

// Typed from 007a8260
/// `ListBox<TESChallenge_P>::UpdateHighlightBoxPos` (engine map).
pub fn list_box_tes_challenge_update_highlight_box_pos(e: &mut Engine, this: Ptr) {
    template::update_highlight_box_pos(e, Of::Challenge, this);
}

// Typed from 007a8580
/// `ListBox<TESChallenge_P>::SetTotalListHeight` (engine map).
pub fn list_box_tes_challenge_set_total_list_height(e: &mut Engine, this: Ptr) {
    template::set_total_list_height(e, Of::Challenge, this);
}

// ---- The list box helpers ----------------------------------------------------------------------

// Typed from 007a1910
/// The data of the list box's highlighted item (`007e1110` on the highlighted tile), or 0.
pub fn fn_007a1910(e: &mut Engine, this: Ptr) -> u32 {
    let highlighted = e.mem.u32(this.addr().wrapping_add(HIGHLIGHTED));
    typed_call!(
        super::FAMILY,
        0x007e_1110,
        super::stats_menu::fn_007e1110(e, this, highlighted)
    )
}

// Typed from 007a1930
/// The list box's flag 0 set or cleared by `value` (`007a1950`, in place).
pub fn fn_007a1930(e: &mut Engine, this: Ptr, value: u8) {
    set_flag(e, this.addr(), 0, value != 0);
}

// Typed from 007a1950
/// The list box's flag `bit` set (`value` non-zero) or cleared.
pub fn fn_007a1950(e: &mut Engine, this: Ptr, bit: u8, value: u8) {
    set_flag(e, this.addr(), u32::from(bit), value != 0);
}

// Typed from 007a5a30
/// The list box container's trait `0xfa3` set to `value` (the container is not checked).
pub fn fn_007a5a30(e: &mut Engine, this: Ptr, value: u8) {
    let container = e.mem.u32(this.addr().wrapping_add(CONTAINER));
    tile_set_int(e, container, TRAIT_FA3, i32::from(value));
}

// Typed from 007a62e0
/// A `ListBoxItem` (Xbox PDB): the tile, the word at `data`, the filtered byte clear. `this`.
pub fn fn_007a62e0(e: &mut Engine, this: Ptr, tile: u32, data: u32) -> Ptr {
    let at = this.addr();
    e.mem.set_u32(at, tile);
    let word = e.mem.u32(data);
    e.mem.set_u32(at.wrapping_add(ITEM_DATA), word);
    e.mem.set_u8(at.wrapping_add(ITEM_FILTERED), 0);
    this
}

// ---- BSSimpleArray<BSStringT<char>,1024> -------------------------------------------------------

// Typed from 007a1a70
/// The string array's constructor: the vtable, then the set-up with 0 and 0. `this`.
pub fn fn_007a1a70(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), STRING_ARRAY_VTABLE);
    typed_call!(super::FAMILY, 0x007a_63f0, fn_007a63f0(e, this, 0, 0));
    this
}

// Typed from 007a1aa0
/// The string array's destructor: the vtable, then the release with 1.
pub fn fn_007a1aa0(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), STRING_ARRAY_VTABLE);
    typed_call!(super::FAMILY, 0x007a_1c30, fn_007a1c30(e, this, 1));
}

// Typed from 007a1ac0
/// A string appended: a slot added at the end, a string constructed there and `text` assigned to
/// it. The slot's index.
pub fn fn_007a1ac0(e: &mut Engine, this: Ptr, text: u32) -> u32 {
    let at = this.addr();
    let index = e.call(ADD_SLOT, &args![this]).u32();
    let offset = index.wrapping_mul(STRING);
    let buffer = e.mem.u32(at.wrapping_add(BUFFER));
    typed_call!(
        super::FAMILY,
        0x007a_6310,
        fn_007a6310(e, this, buffer.wrapping_add(offset), 1)
    );
    let buffer = e.mem.u32(at.wrapping_add(BUFFER));
    e.call(STRING_SET, &args![buffer.wrapping_add(offset), text]);
    index
}

// Typed from 007a1c30
/// The string array's release: with a buffer, its `size` strings destroyed; with `release` set the
/// buffer freed (the pathing family's) and the buffer and reserved size cleared; then the size.
pub fn fn_007a1c30(e: &mut Engine, this: Ptr, release: u8) {
    let at = this.addr();
    let buffer = e.mem.u32(at.wrapping_add(BUFFER));
    if buffer == 0 {
        return;
    }
    let size = e.mem.u32(at.wrapping_add(SIZE));
    typed_call!(
        super::FAMILY,
        0x007a_63b0,
        fn_007a63b0(e, this, buffer, size)
    );
    if release != 0 {
        typed_call!(
            pathing::FAMILY,
            0x006a_8500,
            pathing::search::free_data(e, Ptr::new(at))
        );
        e.mem.set_u32(at.wrapping_add(BUFFER), 0);
        e.mem.set_u32(at.wrapping_add(RESERVED), 0);
    }
    e.mem.set_u32(at.wrapping_add(SIZE), 0);
}

// Typed from 007a6100
/// The string array's scalar deleting destructor (engine map). `this`.
pub fn fn_007a6100(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(super::FAMILY, 0x007a_1aa0, fn_007a1aa0(e, this));
    free_if_flagged(e, this, flags)
}

// Typed from 007a6310
/// `count` strings constructed from `buffer` on (the placement `operator new` and `BSString`'s
/// constructor, both in place: an empty string in each slot).
pub fn fn_007a6310(e: &mut Engine, this: Ptr, buffer: u32, count: u32) {
    let _ = this;
    for index in 0..count {
        let slot = buffer.wrapping_add(index.wrapping_mul(STRING));
        if slot != 0 {
            in_place::empty_string(e, slot);
        }
    }
}

// Typed from 007a63b0
/// `count` strings destroyed from `buffer` on (each the string's deleting destructor with 0).
pub fn fn_007a63b0(e: &mut Engine, this: Ptr, buffer: u32, count: u32) {
    let _ = this;
    for index in 0..count {
        let slot = buffer.wrapping_add(index.wrapping_mul(STRING));
        typed_call!(
            super::FAMILY,
            0x007a_86b0,
            fn_007a86b0(e, Ptr::new(slot), 0)
        );
    }
}

// Typed from 007a63f0
/// The string array's set-up: buffer, size and reserved size cleared; the larger of `reserve` and
/// `size` reserved through `Allocate` when not 0; then `size` strings constructed and the size set.
pub fn fn_007a63f0(e: &mut Engine, this: Ptr, reserve: u32, size: u32) {
    let at = this.addr();
    e.mem.set_u32(at.wrapping_add(BUFFER), 0);
    e.mem.set_u32(at.wrapping_add(SIZE), 0);
    e.mem.set_u32(at.wrapping_add(RESERVED), 0);
    let reserve = reserve.max(size);
    if reserve != 0 {
        let buffer = e.vcall(at, ALLOCATE_SLOT, &args![reserve]).u32();
        e.mem.set_u32(at.wrapping_add(BUFFER), buffer);
        e.mem.set_u32(at.wrapping_add(RESERVED), reserve);
    }
    if size != 0 {
        let buffer = e.mem.u32(at.wrapping_add(BUFFER));
        typed_call!(
            super::FAMILY,
            0x007a_6310,
            fn_007a6310(e, this, buffer, size)
        );
        e.mem.set_u32(at.wrapping_add(SIZE), size);
    }
}

// Typed from 007a86b0
/// `BSStringT<char>`'s scalar deleting destructor: the string's destructor, then the free when bit 0
/// of `flags` is set. `this`.
pub fn fn_007a86b0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(STRING_DESTRUCT, &args![this]);
    free_if_flagged(e, this, flags)
}

/// This file's typed functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0073_12e0, list_box_bgs_note_save_list_state(Ptr)),
        entry!(0x007a_1910, fn_007a1910(Ptr) -> u32),
        entry!(0x007a_1930, fn_007a1930(Ptr, u8)),
        entry!(0x007a_1950, fn_007a1950(Ptr, u8, u8)),
        entry!(0x007a_1a70, fn_007a1a70(Ptr) -> Ptr),
        entry!(0x007a_1aa0, fn_007a1aa0(Ptr)),
        entry!(0x007a_1ac0, fn_007a1ac0(Ptr, u32) -> u32),
        entry!(0x007a_1c30, fn_007a1c30(Ptr, u8)),
        entry!(0x007a_1c90, fn_007a1c90(Ptr) -> Ptr),
        entry!(0x007a_1d60, fn_007a1d60(Ptr)),
        entry!(0x007a_1dd0, fn_007a1dd0(Ptr, u32, u32, u32, u32) -> u32),
        entry!(0x007a_2070, list_box_tes_quest_remove_all(Ptr)),
        entry!(0x007a_22a0, list_box_tes_quest_sort(Ptr, u32)),
        entry!(0x007a_2350, list_box_tes_quest_set_enabled(Ptr, u8)),
        entry!(0x007a_23e0, list_box_tes_quest_highlight(Ptr, u32) -> bool),
        entry!(0x007a_2450, list_box_tes_quest_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x007a_27f0, fn_007a27f0(Ptr) -> Ptr),
        entry!(0x007a_28c0, fn_007a28c0(Ptr)),
        entry!(0x007a_2930, fn_007a2930(Ptr, u32, u32, u32, u32) -> u32),
        entry!(0x007a_2bd0, list_box_bgs_note_remove_all(Ptr)),
        entry!(0x007a_2e00, list_box_bgs_note_sort(Ptr, u32)),
        entry!(0x007a_2e30, list_box_bgs_note_filter(Ptr, u32)),
        entry!(0x007a_30e0, list_box_bgs_note_set_enabled(Ptr, u8)),
        entry!(0x007a_3170, list_box_bgs_note_highlight(Ptr, u32) -> bool),
        entry!(0x007a_31e0, list_box_bgs_note_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x007a_3580, fn_007a3580(Ptr, u8)),
        entry!(0x007a_36e0, fn_007a36e0(Ptr) -> Ptr),
        entry!(0x007a_37b0, fn_007a37b0(Ptr)),
        entry!(0x007a_3820, fn_007a3820(Ptr, u32, u32, u32, u32) -> u32),
        entry!(0x007a_3ac0, list_box_tes_object_refr_remove_all(Ptr)),
        entry!(0x007a_3cf0, list_box_tes_object_refr_sort(Ptr, u32)),
        entry!(0x007a_3d20, list_box_tes_object_refr_filter(Ptr, u32)),
        entry!(0x007a_3f90, list_box_tes_object_refr_highlight(Ptr, u32) -> bool),
        entry!(0x007a_4000, list_box_tes_object_refr_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x007a_43a0, fn_007a43a0(Ptr) -> Ptr),
        entry!(0x007a_4470, fn_007a4470(Ptr)),
        entry!(0x007a_44e0, fn_007a44e0(Ptr, u32, u32, u32, u32) -> u32),
        entry!(0x007a_4780, list_box_bgs_quest_objective_remove_all(Ptr)),
        entry!(0x007a_49b0, list_box_bgs_quest_objective_sort(Ptr, u32)),
        entry!(0x007a_49e0, list_box_bgs_quest_objective_filter(Ptr, u32)),
        entry!(
            0x007a_4c50,
            list_box_bgs_quest_objective_set_enabled(Ptr, u8)
        ),
        entry!(0x007a_4ce0, list_box_bgs_quest_objective_highlight(Ptr, u32) -> bool),
        entry!(0x007a_4d50, list_box_bgs_quest_objective_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x007a_50f0, fn_007a50f0(Ptr) -> Ptr),
        entry!(0x007a_51c0, fn_007a51c0(Ptr)),
        entry!(0x007a_5230, fn_007a5230(Ptr, u32, u32, u32, u32) -> u32),
        entry!(0x007a_54d0, list_box_tes_challenge_remove_all(Ptr)),
        entry!(0x007a_5700, list_box_tes_challenge_sort(Ptr, u32)),
        entry!(0x007a_5730, list_box_tes_challenge_filter(Ptr, u32)),
        entry!(0x007a_59a0, list_box_tes_challenge_set_enabled(Ptr, u8)),
        entry!(0x007a_5a30, fn_007a5a30(Ptr, u8)),
        entry!(0x007a_5a60, list_box_tes_challenge_highlight(Ptr, u32) -> bool),
        entry!(0x007a_5ad0, list_box_tes_challenge_get_highlight_target(Ptr, u32) -> u32),
        entry!(0x007a_6100, fn_007a6100(Ptr, u32) -> Ptr),
        entry!(0x007a_6130, fn_007a6130(Ptr, u32) -> Ptr),
        entry!(0x007a_6160, fn_007a6160(Ptr, u32) -> Ptr),
        entry!(0x007a_6190, fn_007a6190(Ptr, u32) -> Ptr),
        entry!(0x007a_61c0, fn_007a61c0(Ptr, u32) -> Ptr),
        entry!(0x007a_61f0, fn_007a61f0(Ptr, u32) -> Ptr),
        entry!(0x007a_6220, fn_007a6220(Ptr) -> u32),
        entry!(0x007a_6250, fn_007a6250(Ptr) -> u32),
        entry!(0x007a_6280, fn_007a6280(Ptr) -> u32),
        entry!(0x007a_62b0, fn_007a62b0(Ptr) -> u32),
        entry!(0x007a_62e0, fn_007a62e0(Ptr, u32, u32) -> Ptr),
        entry!(0x007a_6310, fn_007a6310(Ptr, u32, u32)),
        entry!(0x007a_63b0, fn_007a63b0(Ptr, u32, u32)),
        entry!(0x007a_63f0, fn_007a63f0(Ptr, u32, u32)),
        entry!(0x007a_6480, list_box_tes_quest_synchronize(Ptr)),
        entry!(0x007a_6550, list_box_tes_quest_is_enabled(Ptr) -> bool),
        entry!(
            0x007a_6600,
            list_box_tes_quest_update_highlight_box_pos(Ptr)
        ),
        entry!(0x007a_6920, list_box_tes_quest_set_total_list_height(Ptr)),
        entry!(0x007a_6a50, fn_007a6a50(Ptr) -> i32),
        entry!(0x007a_6b00, list_box_bgs_note_synchronize(Ptr)),
        entry!(0x007a_6bd0, list_box_bgs_note_is_enabled(Ptr) -> bool),
        entry!(0x007a_6c80, list_box_bgs_note_update_highlight_box_pos(Ptr)),
        entry!(0x007a_7080, list_box_bgs_note_set_total_list_height(Ptr)),
        entry!(0x007a_71b0, list_box_tes_object_refr_num_unfiltered(Ptr) -> i32),
        entry!(0x007a_7260, list_box_tes_object_refr_synchronize(Ptr)),
        entry!(0x007a_7330, list_box_tes_object_refr_is_enabled(Ptr) -> bool),
        entry!(
            0x007a_73e0,
            list_box_tes_object_refr_update_highlight_box_pos(Ptr)
        ),
        entry!(
            0x007a_7700,
            list_box_tes_object_refr_set_total_list_height(Ptr)
        ),
        entry!(0x007a_7830, fn_007a7830(Ptr) -> i32),
        entry!(0x007a_78e0, list_box_bgs_quest_objective_synchronize(Ptr)),
        entry!(0x007a_79b0, list_box_bgs_quest_objective_is_enabled(Ptr) -> bool),
        entry!(
            0x007a_7a60,
            list_box_bgs_quest_objective_update_highlight_box_pos(Ptr)
        ),
        entry!(
            0x007a_7d80,
            list_box_bgs_quest_objective_set_total_list_height(Ptr)
        ),
        entry!(0x007a_8030, list_box_tes_challenge_num_unfiltered(Ptr) -> i32),
        entry!(0x007a_80e0, list_box_tes_challenge_synchronize(Ptr)),
        entry!(0x007a_81b0, list_box_tes_challenge_is_enabled(Ptr) -> bool),
        entry!(
            0x007a_8260,
            list_box_tes_challenge_update_highlight_box_pos(Ptr)
        ),
        entry!(
            0x007a_8580,
            list_box_tes_challenge_set_total_list_height(Ptr)
        ),
        entry!(0x007a_86b0, fn_007a86b0(Ptr, u32) -> Ptr),
        entry!(0x007a_86e0, list_box_tes_quest_num_unfiltered(Ptr) -> i32),
    ]
}
