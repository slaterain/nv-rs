//! `functions_jip/jip_fn_ui.h` (JIP LN NVSE): the two commands that put an interface tile's
//! scene node under a node of a reference's model, `AttachUIXML` (a menu XML file read into a
//! new tile) and `AttachUIComponent` (menu XML text made into a tile, as `InjectUIComponent`
//! makes it). The rest of the file's commands are in `jip_fn_ui.rs`.
//!
//! What each does, as JIP's command bodies (every offset and call checked in the exe):
//! - the node: JIP's `TESObjectREFR::GetNode` (nvse/GameObjects.cpp): the reference's root as
//!   seen (`GetNiNode`: for the player in first person its first-person node), then for a
//!   non-empty name the block of that name below it, kept only when it is a node (its vtable
//!   +0xC answers `006815C0`); an empty name keeps the root ([`nif::ni_block`]);
//! - the tile: made under the HUD's own tile (`g_HUDMainMenu`, `*0x011D96C0`, its tile at +4):
//!   `Tile::ReadFile` (`0x00A01B00`, `thiscall` (tile, path), `RET 4`; JIP's `Tile::ReadXML`) for
//!   `AttachUIXML`, JIP's `InjectUIComponent` for `AttachUIComponent`
//!   ([`super::jip_fn_ui::inject_component`]: the built-in UI Organizer's inject with its row
//!   on, else the text read through the same `ReadFile` as JIP's `jip_temp.xml`, with no file
//!   written);
//! - the new tile's node (`Tile::spModel`, +0x2C, engine-bs `tile.rs`), when it has one, added
//!   to the found node with the node's `AddObject` (vtable +0xDC, `thiscall` (node, child, 1):
//!   `00A5ED10` in `NiNode`'s and `BSFadeNode`'s vtables, `RET 8`); the result 1.
//!
//! Buffers: the node's name is JIP's `nodeName[0x40]`, the XML path `xmlPath[0x80]`,
//! `AttachUIComponent`'s text JIP's string argument buffer (`GetStrArgBuffer`); the crate's
//! buffer rule ([`super::buffer_arg`], [`super::formatted_in_buffer`]) for longer texts.
//!
//! Deliberate differences: with no HUD menu yet (JIP would read through null) the commands do
//! nothing; a node's vtable that cannot be read counts as "not a node".
//!
//! Threads: on the script's thread, as JIP's (no lock: JIP takes none).

use super::jip_fn_misc_ref_nodes::block_name;
use super::nif_nodes as nif;
use super::Handlers;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x28fe, attach_ui_xml),
    (0x28ff, attach_ui_component),
];

/// `g_HUDMainMenu` (`*0x011D96C0`) and a menu's tile (`Menu::pMenu`, +4).
const HUD_MAIN_MENU: u32 = 0x011d_96c0;
const MENU_TILE: u32 = 0x04;
/// `Tile::spModel` (+0x2C, engine-bs `tile.rs`).
const TILE_NODE: u32 = 0x2c;
/// `Tile::ReadFile` (`0x00A01B00`).
const TILE_READ_FILE: u32 = 0x00a0_1b00;
/// `NiNode::AddObject` (vtable +0xDC).
pub(crate) const ADD_OBJECT_SLOT: u32 = 0xdc;
/// JIP's `xmlPath[0x80]`.
const XML_BUFFER: usize = 0x80;
/// JIP's string argument buffer (`GetStrArgBuffer`).
const STR_ARG_BUFFER: usize = 0x4_0000;
/// `kCommandInfo_AttachUIComponent.numParams`: the node's name and xNVSE's 21 format parameters.
const ATTACH_PARAMS: u32 = 22;

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: TESObjectREFR::GetNode
// (asm).
/// The node named `name` under the reference's root as seen; the root for an empty name; 0 for a
/// block that is not a node, or none.
pub(crate) fn ref_get_node(e: &Engine, refr: u32, name: &[u8]) -> u32 {
    let block = nif::ni_block(e, refr, name);
    if block != 0 && !name.is_empty() && !nif::is_node(e, block) {
        return 0;
    }
    block
}

/// The HUD menu's tile (`g_HUDMainMenu->tile`); 0 before the HUD exists.
fn hud_tile(e: &Engine) -> u32 {
    nif::ptr(e, HUD_MAIN_MENU)
        .and_then(|menu| nif::rd32(e, menu.wrapping_add(MENU_TILE)))
        .unwrap_or(0)
}

/// `target->AddObject(tile->node, 1)` when the tile has a node; whether it was added.
fn attach_tile_node(e: &mut Engine, target: u32, tile: u32) -> bool {
    if tile == 0 {
        return false;
    }
    let Some(node) = nif::ptr(e, tile.wrapping_add(TILE_NODE)) else {
        return false;
    };
    e.vcall(target, ADD_OBJECT_SLOT, &[node, 1]);
    true
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h: Cmd_AttachUIXML_Execute.
/// The node's name and an XML file: the file read into a new tile under the HUD's tile and the
/// tile's node added under the named node; 1 when added.
fn attach_ui_xml(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = block_name(e, &read, 0, "AttachUIXML");
    let xml = super::buffer_arg(e, &read, 1, XML_BUFFER, "AttachUIXML");
    let target = ref_get_node(e, a.this_obj, &name);
    if target == 0 {
        return handled(true);
    }
    let hud = hud_tile(e);
    if hud == 0 {
        return handled(true);
    }
    let tile = super::with_guest_text(e, &xml, |e, at| {
        e.call(TILE_READ_FILE, &args![hud, at]).u32()
    });
    if attach_tile_node(e, target, tile) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_AttachUIComponent_Execute and InjectUIComponent.
/// The node's name and a formatted text of menu XML: the text made into a tile under the HUD's
/// tile by the built-in UI Organizer and the tile's node added under the named node; 1 when
/// added (the UI Organizer row on or off).
fn attach_ui_component(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 1, ATTACH_PARAMS);
    if !f.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = nvse::commands::args_in_buffer(e, &f.leading, 0, 0x40, "AttachUIComponent");
    let text = super::formatted_in_buffer(e, &f.text, STR_ARG_BUFFER, "AttachUIComponent");
    let target = ref_get_node(e, a.this_obj, &name);
    if target == 0 {
        return handled(true);
    }
    let hud = hud_tile(e);
    if hud == 0 {
        return handled(true);
    }
    let tile = super::jip_fn_ui::inject_component(e, hud, &text);
    if attach_tile_node(e, target, tile) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ui_nodes_tests.rs"]
mod tests;
