//! `functions_jip/jip_fn_ui.h` (JIP LN NVSE): `ProjectUITile`, a part of the interface drawn into
//! the texture of a reference's model block (a screen in the world showing a menu, say). The rest
//! of the file's commands are in `jip_fn_ui.rs` and the `jip_fn_ui_*` modules.
//!
//! The drawing is the game's own ([`super::rendered_textures::generate_rendered_ui_texture`]).
//!
//! Threads: on the script's thread, as JIP's.

use super::jip_fn_ui::target_component;
use super::jip_fn_ui_cursor::half_screen;
use super::rendered_textures::{generate_rendered_ui_texture, texture_ptr};
use super::Handlers;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2925, project_ui_tile),
];

/// JIP's `tilePath[0x100]` and `blockName[0x40]`.
const PATH_BUFFER: usize = 0x100;
const BLOCK_BUFFER: usize = 0x40;
/// `g_interfaceManager` (`*0x011D8A80`) and its `uiRootNode` (+0x80); a tile's node (+0x2C).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const IM_UI_ROOT: u32 = 0x80;
const TILE_NODE: u32 = 0x2c;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_ui.h:
// Cmd_ProjectUITile_Execute.
/// `ref.ProjectUITile path block x y width height`: the tile at `path` (`GetTargetComponent`; the
/// whole interface, the manager's root node, for an empty path) drawn into the texture of the
/// reference's geometry block `block` (`GetTexturePtr`), taking the screen area `x y width height`
/// (in the interface's units; JIP's `NiVector4 scrArea`). Result 1 when drawn; nothing when the
/// block has no texture place or the tile no node.
fn project_ui_tile(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let path = super::buffer_arg(e, &read, 0, PATH_BUFFER, "ProjectUITile");
    let block = super::buffer_arg(e, &read, 1, BLOCK_BUFFER, "ProjectUITile");
    let area = [
        read.float(2, 0.0),
        read.float(3, 0.0),
        read.float(4, 0.0),
        read.float(5, 0.0),
    ];
    // The command needs a reference (the script runner refuses a call without one).
    if a.this_obj == 0 {
        return handled(true);
    }
    let slot = texture_ptr(e, a.this_obj, &block);
    if slot == 0 {
        return handled(true);
    }
    let node = if path.is_empty() {
        let im = e.mem.u32(INTERFACE_MANAGER);
        if im == 0 {
            0
        } else {
            e.mem.u32(im + IM_UI_ROOT)
        }
    } else {
        match target_component(e, &path, false) {
            Some((tile, _)) => e.mem.u32(tile + TILE_NODE),
            None => 0,
        }
    };
    if node == 0 {
        return handled(true);
    }
    let (_, half_height) = half_screen(e);
    generate_rendered_ui_texture(e, node, area, slot, half_height);
    a.set_result(e, 1.0);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_ui_project_tests.rs"]
mod tests;
