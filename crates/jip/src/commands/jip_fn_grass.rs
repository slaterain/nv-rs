//! `functions_jip/jip_fn_grass.h` (JIP LN NVSE): a grass's numbers and model. All 4 of its
//! commands.
//!
//! Casts in `SetGrassTraitNumeric`, from JIP's source (MSVC, SSE2): a negative float is refused
//! (`fVal < 0`; NaN passes, as the comparison is false); `int iVal = fVal` (`cvttss2si`: toward
//! zero, `INT_MIN` for NaN and 2^31 up); JIP's `GetMin`/`GetMax` compare as `int` and answer the
//! type of their FIRST argument, then the store narrows: density `GetMin(iVal, 100)` and the
//! flags and water type `GetMin(iVal, 7)` to a byte, the water distance `GetMin(iVal, 0xFFFF)`
//! to a word; the minimum slope `GetMin(maxSlope, iVal)` is `maxSlope` (a byte) or `(UInt8)iVal`;
//! the maximum slope `GetMax(minSlope, GetMin(iVal, 90))` is `minSlope` or `(UInt8)` of the
//! `int`. The four ranges take the float.

use super::forms::{form_type_more, TESGrass};
use super::{
    assign_string, buffer_arg, cvttss2si, model_path, set_model_path, type_of, Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2724, get_grass_trait_numeric),
    (0x2725, set_grass_trait_numeric),
    (0x2726, get_grass_model),
    (0x2727, set_grass_model),
];

const PATH_BUFFER: usize = 0x80;

fn grass_arg(e: &Engine, read: &Read) -> Option<Ptr<TESGrass>> {
    let form = read.form(0);
    (read.ok && type_of(e, form) == form_type_more::TES_GRASS).then(|| Ptr::new(form))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_grass.h:
// Cmd_GetGrassTraitNumeric_Execute.
/// Trait 0: density; 1, 2: minimum and maximum slope; 3: distance from water; 4: its type;
/// 5-8: position, height and colour ranges and wave period; 9: flags.
fn get_grass_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(g) = grass_arg(e, &read) {
        let value = match read.int(1, 0) {
            0 => Some(f64::from(e.get(g, TESGrass::density))),
            1 => Some(f64::from(e.get(g, TESGrass::minSlope))),
            2 => Some(f64::from(e.get(g, TESGrass::maxSlope))),
            3 => Some(f64::from(e.get(g, TESGrass::unitFromWaterAmount))),
            4 => Some(f64::from(e.get(g, TESGrass::unitFromWaterType))),
            5 => Some(f64::from(e.get(g, TESGrass::positionRange))),
            6 => Some(f64::from(e.get(g, TESGrass::heightRange))),
            7 => Some(f64::from(e.get(g, TESGrass::colorRange))),
            8 => Some(f64::from(e.get(g, TESGrass::wavePeriod))),
            9 => Some(f64::from(e.get(g, TESGrass::grassFlags))),
            _ => None,
        };
        if let Some(v) = value {
            a.set_result(e, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_grass.h:
// Cmd_SetGrassTraitNumeric_Execute.
fn set_grass_trait_numeric(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let f_val = read.float(2, 0.0);
    let Some(g) = grass_arg(e, &read) else {
        return handled(true);
    };
    if f_val < 0.0 {
        return handled(true);
    }
    let i_val = cvttss2si(f_val);
    match read.int(1, 0) {
        0 => e.set(g, TESGrass::density, i_val.min(100) as u8),
        1 => {
            let max = e.get(g, TESGrass::maxSlope);
            let min = if i32::from(max) < i_val { max } else { i_val as u8 };
            e.set(g, TESGrass::minSlope, min);
        }
        2 => {
            let min = e.get(g, TESGrass::minSlope);
            let capped = i_val.min(90);
            let max = if i32::from(min) > capped { min } else { capped as u8 };
            e.set(g, TESGrass::maxSlope, max);
        }
        3 => e.set(g, TESGrass::unitFromWaterAmount, i_val.min(0xffff) as u16),
        4 => e.set(g, TESGrass::unitFromWaterType, i_val.min(7) as u8),
        5 => e.set(g, TESGrass::positionRange, f_val),
        6 => e.set(g, TESGrass::heightRange, f_val),
        7 => e.set(g, TESGrass::colorRange, f_val),
        8 => e.set(g, TESGrass::wavePeriod, f_val),
        9 => e.set(g, TESGrass::grassFlags, i_val.min(7) as u8),
        _ => {}
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_grass.h:
// Cmd_GetGrassModel_Execute.
fn get_grass_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = match grass_arg(e, &read) {
        Some(g) => model_path(e, g.at(TESGrass::model).addr()),
        None => None,
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_grass.h:
// Cmd_SetGrassModel_Execute.
fn set_grass_model(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetGrassModel");
    if let Some(g) = grass_arg(e, &read) {
        set_model_path(e, g.at(TESGrass::model).addr(), &path);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_grass_tests.rs"]
mod tests;
