//! `functions_ln/ln_fn_cell.h` (JIP LN NVSE): a cell's water height and water form, image space,
//! lighting template and flags. All 8 of its commands.
//!
//! `TESObjectCELL` offsets as the cell loader `005422D0` reads them: the flags byte (`DATA`) at
//! +0x24, the water height (`XCLW`) at +0x50, the lighting template (`LTMP`) at +0xD8, the
//! inherited lighting flags (`LNAM`) at +0xDC, the worldspace at +0xC0 (whose first water form,
//! `TESWorldSpace +0x74`, stands in when the cell has no `ExtraCellWaterType`).
//! `ExtraCellImageSpace` (type 0x59) holds its image space at +0xC. The commands with an
//! optional cell take the player's parent cell (`g_thePlayer` `*011DEA3C`, +0x40) without one.

use super::extra::{self, kind};
use super::forms::TESWorldSpace;
use super::jip_fn_cell::{CELL_EXTRA_LIST, CELL_FLAGS, TES_OBJECT_CELL};
use super::{set_form_result, type_of, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x22f1, get_cell_water_height),
    (0x22f2, set_cell_water_height),
    (0x22be, get_cell_water_form),
    (0x22ea, get_cell_image_space),
    (0x2252, get_cell_lighting_template),
    (0x2253, set_cell_lighting_template),
    (0x22ec, get_cell_flag),
    (0x22ed, set_cell_flag),
];

const PLAYER: u32 = 0x011d_ea3c;
const PARENT_CELL: u32 = 0x40;
const WATER_HEIGHT: u32 = 0x50;
const WORLD_SPACE: u32 = 0xc0;
const LIGHTING_TEMPLATE: u32 = 0xd8;
const INHERIT_FLAGS: u32 = 0xdc;
const EXTRA_WORD: u32 = 0x0c;
/// `kFormType_BGSLightingTemplate`.
const BGS_LIGHTING_TEMPLATE: u8 = 0x65;

/// The optional cell argument `i`, else the player's parent cell; `None` when neither is a cell.
fn optional_cell(e: &Engine, read: &Read, i: usize) -> Option<u32> {
    let mut c = read.form(i);
    if c == 0 {
        c = e.mem.u32(e.mem.u32(PLAYER) + PARENT_CELL);
    }
    (c != 0 && type_of(e, c) == TES_OBJECT_CELL).then_some(c)
}

fn cell_arg(e: &Engine, read: &Read) -> Option<u32> {
    let c = read.form(0);
    (read.ok && type_of(e, c) == TES_OBJECT_CELL).then_some(c)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_cell.h:
// Cmd_GetCellWaterHeight_Execute.
fn get_cell_water_height(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = read.ok.then(|| optional_cell(e, &read, 0)).flatten() {
        let h = e.mem.f32(c + WATER_HEIGHT);
        a.set_result(e, f64::from(h));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_cell.h:
// Cmd_SetCellWaterHeight_Execute.
fn set_cell_water_height(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = read.ok.then(|| optional_cell(e, &read, 1)).flatten() {
        let h = read.float(0, 0.0);
        e.mem.set_f32(c + WATER_HEIGHT, h);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_cell.h:
// Cmd_GetCellWaterForm_Execute.
/// The cell's `ExtraCellWaterType` form, else its worldspace's first water form.
fn get_cell_water_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(c) = read.ok.then(|| optional_cell(e, &read, 0)).flatten() else {
        return handled(true);
    };
    let x = extra::get_by_type(e, c + CELL_EXTRA_LIST, kind::CELL_WATER_TYPE);
    let own = if x != 0 { e.mem.u32(x + EXTRA_WORD) } else { 0 };
    if own != 0 {
        set_form_result(e, &a, own);
    } else {
        let ws = e.mem.u32(c + WORLD_SPACE);
        if ws != 0 {
            let water = e.get(Ptr::<TESWorldSpace>::new(ws), TESWorldSpace::waterFormFirst);
            if water != 0 {
                set_form_result(e, &a, water);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_cell.h:
// Cmd_GetCellImageSpace_Execute.
fn get_cell_image_space(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = cell_arg(e, &read) {
        let x = extra::get_by_type(e, c + CELL_EXTRA_LIST, kind::CELL_IMAGE_SPACE);
        if x != 0 {
            let is = e.mem.u32(x + EXTRA_WORD);
            if is != 0 {
                set_form_result(e, &a, is);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_cell.h:
// Cmd_GetCellLightingTemplate_Execute.
fn get_cell_lighting_template(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = cell_arg(e, &read) {
        let t = e.mem.u32(c + LIGHTING_TEMPLATE);
        if t != 0 {
            set_form_result(e, &a, t);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_cell.h:
// Cmd_SetCellLightingTemplate_Execute.
/// The template (none clears it); another form type does nothing.
fn set_cell_lighting_template(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.form(1);
    if let Some(c) = cell_arg(e, &read) {
        if t == 0 || type_of(e, t) == BGS_LIGHTING_TEMPLATE {
            e.mem.set_u32(c + LIGHTING_TEMPLATE, t);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_cell.h: Cmd_GetCellFlag_Execute.
/// Flags 0 to 7 are the cell's flags byte's bits, 8 to 16 the inherited lighting flags' bits 0
/// to 8; above 16 nothing.
fn get_cell_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if let Some(c) = cell_arg(e, &read).filter(|_| flag <= 16) {
        let set = if flag <= 7 {
            e.mem.u8(c + CELL_FLAGS) & (1 << flag) != 0
        } else {
            e.mem.u32(c + INHERIT_FLAGS) & (1 << (flag - 8)) != 0
        };
        if set {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_cell.h: Cmd_SetCellFlag_Execute.
fn set_cell_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let (flag, value) = (read.int(1, 0), read.int(2, 0));
    if let Some(c) = cell_arg(e, &read).filter(|_| flag <= 16) {
        if flag <= 7 {
            let b = e.mem.u8(c + CELL_FLAGS);
            let bit = 1u8 << flag;
            e.mem
                .set_u8(c + CELL_FLAGS, if value != 0 { b | bit } else { b & !bit });
        } else {
            let f = e.mem.u32(c + INHERIT_FLAGS);
            let bit = 1u32 << (flag - 8);
            e.mem
                .set_u32(c + INHERIT_FLAGS, if value != 0 { f | bit } else { f & !bit });
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "ln_fn_cell_tests.rs"]
mod tests;
