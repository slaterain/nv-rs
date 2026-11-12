//! `functions_jip/jip_fn_destruction_data.h` (JIP LN NVSE): a destructible form's health,
//! targetable flag and stages (their numbers, explosion, debris and replacement model), and
//! stages added and removed. All 15 of its commands. The form is any form with a destructible
//! component (`GetDestructibleForm`, by form type); the game reads the data when the object is
//! damaged, so a change takes effect from then on, as with JIP.

use super::forms::{
    form_type, BGSDestructibleObjectForm, DestructibleData, DestructionStage, TESForm,
    FORM_FLAG_DESTRUCTIBLE, TES_MODEL_TEXTURE_SWAP_SIZE, TES_MODEL_TEXTURE_SWAP_VTBL,
};
use super::{
    assign_string, buffer_arg, destructible_form, heap_alloc, heap_free, model_path,
    set_form_result, set_model_path, type_of, Handlers,
};
use engine_core::prelude::*;
use engine_core::ptr::Layout;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x26c1, get_destruction_data_health),
    (0x26c2, set_destruction_data_health),
    (0x26c3, get_destruction_data_targetable),
    (0x26c4, set_destruction_data_targetable),
    (0x26c5, get_destruction_data_num_stages),
    (0x26c6, get_nth_destruction_stage_trait),
    (0x26c7, set_nth_destruction_stage_trait),
    (0x26c8, get_nth_destruction_stage_explosion),
    (0x26c9, set_nth_destruction_stage_explosion),
    (0x26ca, get_nth_destruction_stage_debris),
    (0x26cb, set_nth_destruction_stage_debris),
    (0x26cc, get_nth_destruction_stage_replacement),
    (0x26cd, set_nth_destruction_stage_replacement),
    (0x26f3, add_destruction_stage),
    (0x26f4, remove_destruction_stage),
];

/// JIP's path buffer (`char path[0x80]`).
const PATH_BUFFER: usize = 0x80;
/// What `AddDestructionStage` writes over a new data's count, flag and two unknown bytes
/// (`ULNG(stageCount) = 0x160D0000`).
const NEW_DATA_COUNT_WORD: u32 = 0x160d_0000;

/// The data of argument 0's destructible component, when the read succeeded and the form has
/// both.
fn data_arg(e: &Engine, read: &Read) -> Option<Ptr<DestructibleData>> {
    if !read.ok {
        return None;
    }
    let destructible = destructible_form(e, read.form(0));
    if destructible == 0 {
        return None;
    }
    let data = e.get(
        Ptr::<BGSDestructibleObjectForm>::new(destructible),
        BGSDestructibleObjectForm::data,
    );
    (data != 0).then(|| Ptr::new(data))
}

/// Stage `idx` (argument 1) of argument 0's data, when the data has its stage array and `idx`
/// is below its count.
fn stage_arg(e: &Engine, read: &Read) -> Option<Ptr<DestructionStage>> {
    let data = data_arg(e, read)?;
    let stages = e.get(data, DestructibleData::stages);
    let idx = read.int(1, 0);
    if stages == 0 || idx >= u32::from(e.get(data, DestructibleData::stageCount)) {
        return None;
    }
    Some(Ptr::new(e.mem.u32(stages + 4 * idx)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_GetDestructionDataHealth_Execute.
fn get_destruction_data_health(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(data) = data_arg(e, &read) {
        let health = e.get(data, DestructibleData::health);
        a.set_result(e, f64::from(health as i32));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_SetDestructionDataHealth_Execute.
fn set_destruction_data_health(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(data) = data_arg(e, &read) {
        e.set(data, DestructibleData::health, read.int(1, 0));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_GetDestructionDataTargetable_Execute.
fn get_destruction_data_targetable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(data) = data_arg(e, &read) {
        if e.get(data, DestructibleData::targetable) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_SetDestructionDataTargetable_Execute.
fn set_destruction_data_targetable(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(data) = data_arg(e, &read) {
        let value = u8::from(read.int(1, 0) != 0);
        e.set(data, DestructibleData::targetable, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_GetDestructionDataNumStages_Execute.
fn get_destruction_data_num_stages(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(data) = data_arg(e, &read) {
        let count = e.get(data, DestructibleData::stageCount);
        a.set_result(e, f64::from(count));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_GetNthDestructionStageTrait_Execute.
/// Trait 0: health percent; 1: self damage per second (signed); 2: damage stage; 3: flags;
/// 4: debris count (signed). Any other leaves the result.
fn get_nth_destruction_stage_trait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(stage) = stage_arg(e, &read) {
        let value = match read.int(2, 0) {
            0 => Some(f64::from(e.get(stage, DestructionStage::healthPrc))),
            1 => Some(f64::from(e.get(stage, DestructionStage::selfDmgSec) as i32)),
            2 => Some(f64::from(e.get(stage, DestructionStage::dmgStage))),
            3 => Some(f64::from(e.get(stage, DestructionStage::flags))),
            4 => Some(f64::from(e.get(stage, DestructionStage::debrisCount) as i32)),
            _ => None,
        };
        if let Some(v) = value {
            a.set_result(e, v);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_SetNthDestructionStageTrait_Execute.
/// The same traits set: health percent modulo 101, damage stage modulo 9, flags' low three bits
/// (the 16-bit word, so the byte after the game's flags is cleared), the counts as given.
fn set_nth_destruction_stage_trait(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(stage) = stage_arg(e, &read) {
        let value = read.int(3, 0);
        match read.int(2, 0) {
            0 => e.set(stage, DestructionStage::healthPrc, (value % 101) as u8),
            1 => e.set(stage, DestructionStage::selfDmgSec, value),
            2 => e.set(stage, DestructionStage::dmgStage, (value % 9) as u8),
            3 => e.set(stage, DestructionStage::flags, (value & 7) as u16),
            4 => e.set(stage, DestructionStage::debrisCount, value),
            _ => {}
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_GetNthDestructionStageExplosion_Execute.
fn get_nth_destruction_stage_explosion(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(stage) = stage_arg(e, &read) {
        let explosion = e.get(stage, DestructionStage::explosion);
        if explosion != 0 {
            set_form_result(e, &a, explosion);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_SetNthDestructionStageExplosion_Execute.
/// The stage's explosion set to an explosion form; no form or another type clears it.
fn set_nth_destruction_stage_explosion(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(stage) = stage_arg(e, &read) {
        let form = read.form(2);
        let value = if form != 0 && type_of(e, form) == form_type::BGS_EXPLOSION {
            form
        } else {
            0
        };
        e.set(stage, DestructionStage::explosion, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_GetNthDestructionStageDebris_Execute.
fn get_nth_destruction_stage_debris(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(stage) = stage_arg(e, &read) {
        let debris = e.get(stage, DestructionStage::debris);
        if debris != 0 {
            set_form_result(e, &a, debris);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_SetNthDestructionStageDebris_Execute.
fn set_nth_destruction_stage_debris(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if let Some(stage) = stage_arg(e, &read) {
        let form = read.form(2);
        let value = if form != 0 && type_of(e, form) == form_type::BGS_DEBRIS {
            form
        } else {
            0
        };
        e.set(stage, DestructionStage::debris, value);
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_GetNthDestructionStageReplacement_Execute.
fn get_nth_destruction_stage_replacement(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let replacement = stage_arg(e, &read)
        .map(|stage| e.get(stage, DestructionStage::replacement))
        .unwrap_or(0);
    let text = if replacement != 0 {
        model_path(e, replacement)
    } else {
        None
    };
    assign_string(e, &a, text);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_SetNthDestructionStageReplacement_Execute.
/// The stage's replacement model's path set; a stage without one first gets a new
/// `TESModelTextureSwap` (0x20 bytes of the game's heap, zeroed, its vtable set). The result 1.
fn set_nth_destruction_stage_replacement(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let path = buffer_arg(e, &read, 2, PATH_BUFFER, "SetNthDestructionStageReplacement");
    if let Some(stage) = stage_arg(e, &read) {
        let mut replacement = e.get(stage, DestructionStage::replacement);
        if replacement == 0 {
            replacement = heap_alloc(e, TES_MODEL_TEXTURE_SWAP_SIZE);
            e.set(stage, DestructionStage::replacement, replacement);
            e.mem.write(replacement, &[0; TES_MODEL_TEXTURE_SWAP_SIZE as usize]);
            e.mem.set_u32(replacement, TES_MODEL_TEXTURE_SWAP_VTBL);
        }
        set_model_path(e, replacement, &path);
        a.set_result(e, 1.0);
    }
    handled(true)
}

fn form_flags(e: &Engine, form: u32) -> u32 {
    e.get(Ptr::<TESForm>::new(form), TESForm::iFormFlags)
}

fn set_form_flags(e: &mut Engine, form: u32, flags: u32) {
    e.set(Ptr::<TESForm>::new(form), TESForm::iFormFlags, flags);
}

/// `MemCopy` (JIP LN NVSE) of `words` words: overlapping copies come out as `memmove`'s.
fn move_words(e: &mut Engine, to: u32, from: u32, words: u32) {
    let values: Vec<u32> = (0..words).map(|i| e.mem.u32(from + 4 * i)).collect();
    for (i, v) in values.into_iter().enumerate() {
        e.mem.set_u32(to + 4 * i as u32, v);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_AddDestructionStage_Execute.
/// A new stage (0x18 bytes of the game's heap, zeroed, with the health percent and damage stage
/// given, stored as bytes) put at the index (a negative one: the end; past the count: nothing).
/// The result starts at -1 and becomes the index. A form without data gets new data first (its
/// count word `0x160D0000`); the stage array is made anew one longer, the old one freed; the form
/// is marked destructible.
fn add_destruction_stage(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let object = read.form(0);
    let destructible = destructible_form(e, object);
    if destructible == 0 {
        return handled(true);
    }
    let dest = Ptr::<BGSDestructibleObjectForm>::new(destructible);
    let mut idx = read.int(1, 0) as i32;
    let (health, damage_stage) = (read.int(2, 0), read.int(3, 0));
    let data = e.get(dest, BGSDestructibleObjectForm::data);
    let count = if data != 0 && e.get(Ptr::<DestructibleData>::new(data), DestructibleData::stages) != 0
    {
        i32::from(e.get(Ptr::<DestructibleData>::new(data), DestructibleData::stageCount))
    } else {
        0
    };
    if idx > count {
        return handled(true);
    }
    if idx < 0 {
        idx = count;
    }
    a.set_result(e, f64::from(idx));
    let stage = heap_alloc(e, DestructionStage::SIZE);
    e.mem.write(stage, &[0; DestructionStage::SIZE as usize]);
    let stage_p = Ptr::<DestructionStage>::new(stage);
    e.set(stage_p, DestructionStage::healthPrc, health as u8);
    e.set(stage_p, DestructionStage::dmgStage, damage_stage as u8);
    if e.get(dest, BGSDestructibleObjectForm::data) == 0 {
        let new = heap_alloc(e, DestructibleData::SIZE);
        let d = Ptr::<DestructibleData>::new(new);
        e.set(dest, BGSDestructibleObjectForm::data, new);
        e.set(d, DestructibleData::health, 0);
        // The count, the targetable flag and the two bytes after them, as one word.
        e.mem.set_u32(new + DestructibleData::stageCount.off, NEW_DATA_COUNT_WORD);
        e.set(d, DestructibleData::stages, 0);
        e.set(d, DestructibleData::unk0C, 0);
        e.set(d, DestructibleData::unk10, 0);
    }
    let d = Ptr::<DestructibleData>::new(e.get(dest, BGSDestructibleObjectForm::data));
    let stages = e.get(d, DestructibleData::stages);
    let n = u32::from(e.get(d, DestructibleData::stageCount));
    let idx = idx as u32;
    if stages != 0 {
        let new = heap_alloc(e, 4 * (n + 1));
        move_words(e, new, stages, idx);
        move_words(e, new + 4 * (idx + 1), stages + 4 * idx, n.wrapping_sub(idx));
        heap_free(e, stages);
        e.set(d, DestructibleData::stages, new);
        e.mem.set_u32(new + 4 * idx, stage);
    } else {
        let new = heap_alloc(e, 4);
        e.set(d, DestructibleData::stages, new);
        e.mem.set_u32(new, stage);
    }
    e.set(d, DestructibleData::stageCount, (n as u8).wrapping_add(1));
    let flags = form_flags(e, object) | FORM_FLAG_DESTRUCTIBLE;
    set_form_flags(e, object, flags);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_destruction_data.h:
// Cmd_RemoveDestructionStage_Execute.
/// Stage `idx` freed and the later ones moved down (the freed last slot cleared); the last stage
/// removed frees the array and clears the form's destructible flag. The result starts at -1 and
/// becomes the index.
fn remove_destruction_stage(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let object = read.form(0);
    let Some(data) = data_arg(e, &read) else {
        return handled(true);
    };
    let idx = read.int(1, 0);
    if idx >= u32::from(e.get(data, DestructibleData::stageCount)) {
        return handled(true);
    }
    let stages = e.get(data, DestructibleData::stages);
    if stages == 0 {
        return handled(true);
    }
    a.set_result(e, f64::from(idx as i32));
    let stage = e.mem.u32(stages + 4 * idx);
    heap_free(e, stage);
    let left = e.get(data, DestructibleData::stageCount).wrapping_sub(1);
    e.set(data, DestructibleData::stageCount, left);
    if left != 0 {
        let left = u32::from(left);
        move_words(e, stages + 4 * idx, stages + 4 * (idx + 1), left - idx);
        e.mem.set_u32(stages + 4 * left, 0);
    } else {
        heap_free(e, stages);
        e.set(data, DestructibleData::stages, 0);
        let flags = form_flags(e, object) & !FORM_FLAG_DESTRUCTIBLE;
        set_form_flags(e, object, flags);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_destruction_data_tests.rs"]
mod tests;
