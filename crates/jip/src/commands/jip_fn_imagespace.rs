//! `functions_jip/jip_fn_imagespace.h` (JIP LN NVSE): an image space's 33 values and an image
//! space modifier's values, and the running modifiers. All 5 of its commands. The forms are read as `kParamType_ImageSpace` and
//! `kParamType_ImageSpaceModifier`, whose type checks xNVSE's reader makes.
//!
//! An image space's values are 33 floats from +0x18 (`traitValues`; the image space loader
//! `0052B440` reads `DNAM` into them). The night eye tint (traits 21-23) and three cinematic
//! colours (29-31) are kept as 0-1 and shown as 0-255: `value * 255` and `value * (1 / 255.0F)`,
//! both in floats.
//!
//! A modifier keeps each animated value twice: in an interpolator (`fltIntrpl1[44]` from +0x114,
//! 0x18 bytes each, its value at +0xC; `clrIntrpl[2]` from +0x534, 0x24 bytes, four values from
//! +0xC; `fltIntrpl2[9]` from +0x57C) and in the first key of its key array (`data654[44]`,
//! `data704[2]`, `data70C[9]`: pointers to keys whose value follows the time at +4; the modifier
//! loader `0052D510` puts the arrays it reads at +0x654, +0x704 and +0x70C). JIP reads the key
//! and writes both. Traits: 0 duration, 1-2 the radial blur centre, 3-46 `data654`, 47-50 and
//! 51-54 the two colours (0-255 as above), 55-63 `data70C`, 64-67 the bytes `animable`,
//! `radBlurUseTarget`, `DoFUseTarget`, `DoFMode`.
//!
//! Cast of the script's float into those bytes, from JIP's source: `UInt8(value) & 3`
//! (`cvttss2si`, its low byte, then the low two bits).

use super::arrays::{create_array, Element, TempElements};
use super::forms::{
    ColorKey, FloatKey, ImageSpaceModifierInstanceForm, TESImageSpace, TESImageSpaceModifier, Tes,
    IMOD_INSTANCE_FORM_VTBL,
};
use super::{cvttss2si, is_type, list_items, Handlers};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27cc, get_image_space_trait),
    (0x27cd, set_image_space_trait),
    (0x27ce, get_image_space_mod_trait),
    (0x27cf, set_image_space_mod_trait),
    (0x28a0, get_active_imods),
];

/// The size of a float interpolator and of a colour interpolator.
const FLOAT_INTERPOLATOR: u32 = 0x18;
const COLOR_INTERPOLATOR: u32 = 0x24;

/// Whether image space trait `t` is a colour kept as 0-1 and shown as 0-255.
fn is_colour(t: u32) -> bool {
    (21..=31).contains(&t) && (t <= 23 || t >= 29)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_imagespace.h:
// Cmd_GetImageSpaceTrait_Execute.
fn get_image_space_trait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.int(1, 0);
    if read.ok && t <= 32 {
        let s = Ptr::<TESImageSpace>::new(read.form(0));
        let v = e.get(s.index(t, 4), TESImageSpace::traitValues);
        let v = if is_colour(t) { v * 255.0 } else { v };
        a.set_result(e, f64::from(v));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_imagespace.h:
// Cmd_SetImageSpaceTrait_Execute.
fn set_image_space_trait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.int(1, 0);
    if read.ok && t <= 32 {
        let s = Ptr::<TESImageSpace>::new(read.form(0));
        let value = read.float(2, 0.0);
        let v = if is_colour(t) {
            value * (1.0 / 255.0)
        } else {
            value
        };
        e.set(s.index(t, 4), TESImageSpace::traitValues, v);
    }
    handled(true)
}

/// The byte of trait 64 + `i`: `animable`, `radBlurUseTarget`, `DoFUseTarget`, `DoFMode`.
fn mod_byte(i: u32) -> Field<TESImageSpaceModifier, u8> {
    use TESImageSpaceModifier as M;
    match i {
        0 => M::animable,
        1 => M::radBlurUseTarget,
        2 => M::DoFUseTarget,
        _ => M::DoFMode,
    }
}

/// The float of traits 0-2: duration, radial blur centre x and y.
fn mod_float(t: u32) -> Field<TESImageSpaceModifier, f32> {
    use TESImageSpaceModifier as M;
    match t {
        0 => M::duration,
        1 => M::radialBlurCentreX,
        _ => M::radialBlurCentreY,
    }
}

/// The first key of the float key array at `keys` (index `i`).
fn float_key(
    e: &Engine,
    m: Ptr<TESImageSpaceModifier>,
    keys: Field<TESImageSpaceModifier, u32>,
    i: u32,
) -> Ptr<FloatKey> {
    Ptr::new(e.get(m.index(i, 4), keys))
}

/// The first key of colour `c`'s key array.
fn color_key(e: &Engine, m: Ptr<TESImageSpaceModifier>, c: u32) -> Ptr<ColorKey> {
    Ptr::new(e.get(m.index(c, 4), TESImageSpaceModifier::data704))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_imagespace.h:
// Cmd_GetImageSpaceModTrait_Execute.
fn get_image_space_mod_trait(e: &mut Engine, w: &[u32]) -> Ret {
    use TESImageSpaceModifier as M;
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.int(1, 0);
    if !(read.ok && t <= 67) {
        return handled(true);
    }
    let m = Ptr::<M>::new(read.form(0));
    let value = if t <= 2 {
        f64::from(e.get(m, mod_float(t)))
    } else if t > 63 {
        f64::from(e.get(m, mod_byte(t - 64)))
    } else if t <= 46 {
        let key = float_key(e, m, M::data654, t - 3);
        f64::from(e.get(key, FloatKey::value))
    } else if t <= 54 {
        let (c, i) = if t <= 50 { (0, t - 47) } else { (1, t - 51) };
        let key = color_key(e, m, c);
        f64::from(e.get(key.index(i, 4), ColorKey::value) * 255.0)
    } else {
        let key = float_key(e, m, M::data70C, t - 55);
        f64::from(e.get(key, FloatKey::value))
    };
    a.set_result(e, value);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_imagespace.h:
// Cmd_SetImageSpaceModTrait_Execute.
fn set_image_space_mod_trait(e: &mut Engine, w: &[u32]) -> Ret {
    use TESImageSpaceModifier as M;
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let t = read.int(1, 0);
    if !(read.ok && t <= 67) {
        return handled(true);
    }
    let m = Ptr::<M>::new(read.form(0));
    let value = read.float(2, 0.0);
    if t <= 2 {
        e.set(m, mod_float(t), value);
    } else if t > 63 {
        e.set(m, mod_byte(t - 64), (cvttss2si(value) as u8) & 3);
    } else if t <= 46 {
        let i = t - 3;
        e.set(m.index(i, FLOAT_INTERPOLATOR), M::fltIntrpl1Value, value);
        let key = float_key(e, m, M::data654, i);
        e.set(key, FloatKey::value, value);
    } else if t <= 54 {
        let (c, i) = if t <= 50 { (0, t - 47) } else { (1, t - 51) };
        let value = value * (1.0 / 255.0);
        e.set(
            m.index(c, COLOR_INTERPOLATOR).index(i, 4),
            M::clrIntrplValue,
            value,
        );
        let key = color_key(e, m, c);
        e.set(key.index(i, 4), ColorKey::value, value);
    } else {
        let i = t - 55;
        e.set(m.index(i, FLOAT_INTERPOLATOR), M::fltIntrpl2Value, value);
        let key = float_key(e, m, M::data70C, i);
        e.set(key, FloatKey::value, value);
    }
    handled(true)
}

/// `TES::GetSingleton` (`*0x011DEA10`).
const TES: u32 = 0x011d_ea10;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_imagespace.h:
// Cmd_GetActiveIMODs_Execute.
/// An array of the modifiers of the running form instances (vtable
/// `ImageSpaceModifierInstanceForm`) that are not hidden, in the list's order, a null modifier
/// skipped; no array when there are none. Without the `TES` (before the game makes it) JIP would
/// read through null; the port answers none.
fn get_active_imods(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let tes = e.mem.u32(TES);
    if tes == 0 {
        return handled(true);
    }
    let mut tmp = TempElements::new();
    for i in list_items(e, Ptr::<Tes>::new(tes).at(Tes::activeIMODs)) {
        if i == 0 || !is_type(e, i, IMOD_INSTANCE_FORM_VTBL) {
            continue;
        }
        let inst = Ptr::<ImageSpaceModifierInstanceForm>::new(i);
        if e.get(inst, ImageSpaceModifierInstanceForm::hidden) != 0 {
            continue;
        }
        let imod = e.get(inst, ImageSpaceModifierInstanceForm::imageSpaceMod);
        if imod != 0 {
            tmp.append(Element::Form(imod));
        }
    }
    if !tmp.is_empty() {
        let id = create_array(e, a.script, &tmp.0);
        a.set_result(e, f64::from(id));
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_imagespace_tests.rs"]
mod tests;
