//! `functions_jip/jip_fn_cell.h` (JIP LN NVSE): a cell's water form, climate, noise texture,
//! owner and north rotation. All 7 of its commands.
//!
//! `TESObjectCELL` (form type 0x39; offsets as the cell loader `005422D0` reads them, translated
//! in engine-fallout-cell `tesobjectcell.rs`): the flags at +0x24 (bit 0 interior), the extra data
//! list at +0x28, the water noise texture (`TESTexture`) at +0x58 with its path at +0x5C. Extras:
//! `ExtraCellWaterType` (type 0x03) and `ExtraCellClimate` (0x08) hold their form at +0xC, made by
//! JIP's `Create` (`CreateExtraType`, the vtables of the exe's constructors `0101_4270`,
//! `0101_424C`); `ExtraNorthRotation` (0x43) a float at +0xC.

use super::extra::{self, kind, vtbl};
use super::DBL_180_D_PI;
use super::{
    assign_string, buffer_arg, set_form_result, string_set, string_text, type_of, Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27dd, set_cell_water_form),
    (0x27de, get_cell_climate),
    (0x27df, set_cell_climate),
    (0x27e0, get_cell_noise_texture),
    (0x27e1, set_cell_noise_texture),
    (0x28ab, get_owner_of_cell),
    (0x292d, get_cell_north_rotation),
];

/// `kFormType_TESObjectCELL`, `kFormType_TESWaterForm`, `kFormType_TESClimate`.
pub const TES_OBJECT_CELL: u8 = 0x39;
const TES_WATER_FORM: u8 = 0x4e;
const TES_CLIMATE: u8 = 0x36;
/// The cell's flags (+0x24), its extra data list (+0x28), its noise texture's path (+0x5C).
pub const CELL_FLAGS: u32 = 0x24;
pub const CELL_EXTRA_LIST: u32 = 0x28;
const NOISE_TEXTURE_PATH: u32 = 0x5c;
const EXTRA_WORD: u32 = 0x0c;
/// `TESObjectCELL::GetOwner` (`00546A40`, `thiscall`), which JIP calls.
const CELL_GET_OWNER: u32 = 0x0054_6a40;
/// JIP's `char path[0x80]`.
const PATH_BUFFER: usize = 0x80;

fn cell_arg(e: &Engine, read: &Read) -> Option<u32> {
    let c = read.form(0);
    (read.ok && type_of(e, c) == TES_OBJECT_CELL).then_some(c)
}

/// The cell's extra of `kind` holding a form: set to the new form, removed for none, or made
/// when missing (nothing to do for none).
fn set_form_extra(e: &mut Engine, cell: u32, form: u32, kind: u32, vtable: u32) {
    let list = cell + CELL_EXTRA_LIST;
    let x = extra::get_by_type(e, list, kind);
    if x != 0 {
        if form != 0 {
            e.mem.set_u32(x + EXTRA_WORD, form);
        } else {
            extra::remove_by_type(e, list, kind);
        }
    } else if form != 0 {
        let made = extra::create(e, vtable, kind);
        e.mem.set_u32(made + EXTRA_WORD, form);
        extra::add_extra(e, list, made);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_cell.h:
// Cmd_SetCellWaterForm_Execute.
/// The cell's water form (none removes it); another form type does nothing.
fn set_cell_water_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let water = read.form(1);
    if let Some(c) = cell_arg(e, &read) {
        if water == 0 || type_of(e, water) == TES_WATER_FORM {
            set_form_extra(e, c, water, kind::CELL_WATER_TYPE, vtbl::CELL_WATER_TYPE);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_cell.h:
// Cmd_GetCellClimate_Execute.
fn get_cell_climate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = cell_arg(e, &read) {
        let x = extra::get_by_type(e, c + CELL_EXTRA_LIST, kind::CELL_CLIMATE);
        if x != 0 {
            let climate = e.mem.u32(x + EXTRA_WORD);
            if climate != 0 {
                set_form_result(e, &a, climate);
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_cell.h:
// Cmd_SetCellClimate_Execute.
fn set_cell_climate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let climate = read.form(1);
    if let Some(c) = cell_arg(e, &read) {
        if climate == 0 || type_of(e, climate) == TES_CLIMATE {
            set_form_extra(e, c, climate, kind::CELL_CLIMATE, vtbl::CELL_CLIMATE);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_cell.h:
// Cmd_GetCellNoiseTexture_Execute.
fn get_cell_noise_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = cell_arg(e, &read).and_then(|c| string_text(e, c + NOISE_TEXTURE_PATH));
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_cell.h:
// Cmd_SetCellNoiseTexture_Execute.
fn set_cell_noise_texture(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    // The reader fills JIP's buffer before the type is checked.
    let path = read
        .ok
        .then(|| buffer_arg(e, &read, 1, PATH_BUFFER, "SetCellNoiseTexture"));
    if let (Some(c), Some(path)) = (cell_arg(e, &read), path) {
        string_set(e, c + NOISE_TEXTURE_PATH, &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_cell.h:
// Cmd_GetOwnerOfCell_Execute.
/// The game's `TESObjectCELL::GetOwner`: the cell's owner (or its worldspace's), nothing for none.
fn get_owner_of_cell(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = cell_arg(e, &read) {
        let owner = e.call(CELL_GET_OWNER, &args![c]).u32();
        if owner != 0 {
            set_form_result(e, &a, owner);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_cell.h:
// Cmd_GetCellNorthRotation_Execute.
/// An interior's north rotation in degrees (the float times `Dbl180dPI`), nothing for an
/// exterior or no rotation.
fn get_cell_north_rotation(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(c) = cell_arg(e, &read).filter(|&c| e.mem.u8(c + CELL_FLAGS) & 1 != 0) {
        let x = extra::get_by_type(e, c + CELL_EXTRA_LIST, kind::NORTH_ROTATION);
        if x != 0 {
            let r = f64::from(e.mem.f32(x + EXTRA_WORD)) * DBL_180_D_PI;
            a.set_result(e, r);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_cell_tests.rs"]
mod tests;
