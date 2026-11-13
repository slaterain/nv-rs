//! `functions_jip/jip_fn_armor.h` (JIP LN NVSE): an armour's ragdoll constraint template, model
//! paths, audio template, class and alternate textures. All 8 of its commands. Nothing is
//! narrowed (paths, forms and the script's integers only).
//!
//! The armour is read as `kParamType_ObjectID` (an inventory object, the reader's check) or as
//! any form (`SetArmorRCT`); each command then makes JIP's `IS_TYPE(armor, TESObjectARMO)`, a
//! compare of the form's vtable with `kVtbl_TESObjectARMO` (`0102A62C`), so an armour addon
//! (`TESObjectARMA`, vtable `0102A31C`, the same form type's layout) is no armour here.

use super::forms::{
    TESModelTextureSwap, TESObjectARMO, TES_MODEL_TEXTURE_SWAP_SIZE, TES_OBJECT_ARMA_VTBL,
    TES_OBJECT_ARMO_VTBL,
};
use super::{
    assign_string, buffer_arg, formatted_in_buffer, is_type, list_remove_all, set_form_result,
    string_set, string_text, Handlers,
};
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x27bc, get_armor_rct),
    (0x27bd, set_armor_rct),
    (0x2808, set_biped_model_path_alt),
    (0x281f, get_armor_audio_template),
    (0x2820, set_armor_audio_template),
    (0x2835, is_armor_addon),
    (0x2896, set_armor_class),
    (0x28ba, clear_model_alt_textures),
];

/// JIP's path buffers (`char path[0x80]`, `char buffer[0x80]`).
const PATH_BUFFER: usize = 0x80;
/// `kCommandInfo_SetBipedModelPathAlt.numParams`: the armour, the path's index, the format
/// string and its 20 values.
const BIPED_PATH_PARAMS: u32 = 23;

/// `form` when it is an armour (`IS_TYPE(form, TESObjectARMO)`).
fn armor(e: &Engine, form: u32) -> Option<Ptr<TESObjectARMO>> {
    is_type(e, form, TES_OBJECT_ARMO_VTBL).then(|| Ptr::new(form))
}

/// Argument 0 when the read succeeded and it is an armour.
fn armor_arg(e: &Engine, read: &Read) -> Option<Ptr<TESObjectARMO>> {
    if read.ok {
        armor(e, read.form(0))
    } else {
        None
    }
}

/// The ragdoll constraint template's path (`bipedModel.modelRDT.nifPath`), a game string.
fn rct_path(a: Ptr<TESObjectARMO>) -> u32 {
    a.at(TESObjectARMO::modelRDTPath).addr()
}

/// Model `index` of the four in a row from `bipedModel.bipedModel[0]` (male and female biped,
/// then male and female ground model: JIP's `bipedModel[whichPath]` past 1 runs into
/// `groundModel`).
fn biped_model(a: Ptr<TESObjectARMO>, index: u32) -> Ptr<TESModelTextureSwap> {
    a.at(TESObjectARMO::bipedModels)
        .index(index, TES_MODEL_TEXTURE_SWAP_SIZE)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_armor.h:
// Cmd_GetArmorRCT_Execute.
/// The path's text; no armour or no buffer, `""`.
fn get_armor_rct(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let text = armor_arg(e, &read).and_then(|armor| string_text(e, rct_path(armor)));
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_armor.h:
// Cmd_SetArmorRCT_Execute.
fn set_armor_rct(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 1, PATH_BUFFER, "SetArmorRCT");
    if let Some(armor) = armor_arg(e, &read) {
        string_set(e, rct_path(armor), &path);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_armor.h:
// Cmd_SetBipedModelPathAlt_Execute.
/// The path of model 0-3 set to the formatted text (xNVSE's `ExtractFormatStringArgs`, the
/// format string third); a larger index, nothing.
fn set_biped_model_path_alt(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let f = nvse::commands::string::format_string_args_at(e, &a, 2, BIPED_PATH_PARAMS);
    if !f.ok {
        return handled(true);
    }
    let text = formatted_in_buffer(e, &f.text, PATH_BUFFER, "SetBipedModelPathAlt");
    let form = f
        .leading
        .first()
        .and_then(nvse::args::Arg::form)
        .unwrap_or(0);
    let which = f
        .leading
        .get(1)
        .and_then(nvse::args::Arg::int)
        .map_or(0, |v| v as u32);
    if let Some(armor) = armor(e, form).filter(|_| which <= 3) {
        let path = biped_model(armor, which).at(TESModelTextureSwap::nifPath);
        string_set(e, path.addr(), &text);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_armor.h:
// Cmd_GetArmorAudioTemplate_Execute.
/// The template, when the armour has no sounds of its own (the word at +0x188 is then the
/// template, else its list of sounds).
fn get_armor_audio_template(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(armor) = armor_arg(e, &read) {
        let template = e.get(armor, TESObjectARMO::audioTemplate);
        if e.get(armor, TESObjectARMO::overrideSounds) == 0 && template != 0 {
            set_form_result(e, &a, template);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_armor.h:
// Cmd_SetArmorAudioTemplate_Execute.
/// The template set (or cleared) on an armour without sounds of its own; the template must be an
/// armour with sounds of its own.
fn set_armor_audio_template(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let Some(armor) = armor_arg(e, &read) else {
        return handled(true);
    };
    if e.get(armor, TESObjectARMO::overrideSounds) != 0 {
        return handled(true);
    }
    let template = read.form(1);
    let fits = template == 0 || armor_with_sounds(e, template);
    if fits {
        e.set(armor, TESObjectARMO::audioTemplate, template);
    }
    handled(true)
}

fn armor_with_sounds(e: &Engine, form: u32) -> bool {
    armor(e, form).is_some_and(|t| e.get(t, TESObjectARMO::overrideSounds) != 0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_armor.h:
// Cmd_IsArmorAddon_Execute.
fn is_armor_addon(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok && is_type(e, read.form(0), TES_OBJECT_ARMA_VTBL) {
        a.set_result(e, 1.0);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_armor.h:
// Cmd_SetArmorClass_Execute.
/// Class 1 (light): both class bits cleared (medium 0x8, heavy 0x80); 2: medium; 3: heavy. 0 or
/// above 3, nothing.
fn set_armor_class(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let class = read.int(1, 0);
    if let Some(armor) = armor_arg(e, &read).filter(|_| class != 0 && class <= 3) {
        let mut flags = e.get(armor, TESObjectARMO::bipedFlags) & !0x88;
        if class > 1 {
            flags |= if class == 2 { 8 } else { 0x80 };
        }
        e.set(armor, TESObjectARMO::bipedFlags, flags);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_armor.h:
// Cmd_ClearModelAltTextures_Execute.
/// The alternate texture lists of the four models emptied (`tList::RemoveAll`: the nodes after
/// the first freed, the items kept).
fn clear_model_alt_textures(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(armor) = armor_arg(e, &read) {
        for index in 0..4 {
            let list = biped_model(armor, index).at(TESModelTextureSwap::textureList);
            list_remove_all(e, list);
        }
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_armor_tests.rs"]
mod tests;
