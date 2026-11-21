//! Form, Pip-Boy and crime commands, translated from JohnnyGuitarNVSE
//! 59049a0, JG/functions/{fn_form,fn_gameplay}.cpp. Command registration lives
//! in `commands`; this does not advertise a complete JohnnyGuitar installation.
use engine_core::prelude::*;
use nvse::commands::{console_print, double_words, Read};
use nvse::{handled, CommandArgs};

/// `IS_TYPE(form, EffectSetting)` is a vtable comparison (nvse/GameForms.h),
/// unlike `IS_ID`, which compares the form's type byte.
const EFFECT_SETTING_VTABLE: u32 = 0x0101_2834;
/// `EffectSetting::IsAssociatedActorValueUsed` (00403c60), the archetype's flag
/// 2 in the game's table at 01183328. The getters are folded engine code:
/// `EffectSetting::GetAssociatedActorValue` (00586150) reads +9c, and
/// `EffectSetting::GetEffectArchetype` (00810fd0) reads +98.
const EFFECT_HAS_ACTOR_VALUE: u32 = 0x0040_3c60;
const EFFECT_ACTOR_VALUE: u32 = 0x0058_6150;
const EFFECT_ARCHETYPE: u32 = 0x0081_0fd0;

pub(crate) fn base_effect_av(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, -1.0);
    let r = Read::of(e, &a);
    let form = r.form(0);
    if r.ok
        && form != 0
        && e.mem.u32(form) == EFFECT_SETTING_VTABLE
        && e.call(EFFECT_HAS_ACTOR_VALUE, &args![form]).bool()
    {
        let value = e.call(EFFECT_ACTOR_VALUE, &args![form]).i32();
        a.set_result(e, f64::from(value));
    }
    handled(true)
}

pub(crate) fn base_effect_archetype(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, -1.0);
    let r = Read::of(e, &a);
    let form = r.form(0);
    if r.ok && form != 0 && e.mem.u32(form) == EFFECT_SETTING_VTABLE {
        let value = e.call(EFFECT_ARCHETYPE, &args![form]).i32();
        a.set_result(e, f64::from(value));
    }
    handled(true)
}

/// InterfaceManager::GetSingleton, pipBoyMode, OpenPipboy and ClosePipboy
/// (nvse/GameUI.{h,cpp}, read against engine-fallout-interface interfacemanager).
const INTERFACE_MANAGER: u32 = 0x011d_8a80;
const PIPBOY_MODE: u32 = 0x4bc;
const OPEN_PIPBOY: u32 = 0x0070_f4e0;
const CLOSE_PIPBOY: u32 = 0x0070_f690;

pub(crate) fn toggle_pipboy(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let r = Read::of(e, &a);
    // The original ignores ExtractArgsEx's success: its local is initialized
    // to NoMenu (0), so a missing or rejected argument uses that default.
    let menu = r.int(0, 0);
    if matches!(menu, 0 | 1002 | 1003 | 1023) {
        let manager = e.mem.u32(INTERFACE_MANAGER);
        if manager != 0 {
            match e.mem.u32(manager + PIPBOY_MODE) {
                0 => {
                    e.call(OPEN_PIPBOY, &args![manager, 0u32, menu]);
                }
                3 => {
                    e.call(CLOSE_PIPBOY, &args![manager, 0u32]);
                }
                _ => {}
            }
            // Success includes the opening/closing intermediate modes.
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

/// The actor and player layouts are also in engine-fallout-ai's actor.rs and
/// playercharacter.rs: IsActor vtable +100, teammate +18d, perceived list +d48,
/// list nodes {item,next}, perceived items {actor,hostile byte,undetected byte}.
const PLAYER: u32 = 0x011d_ea3c;
const PERCEIVED_ACTORS: u32 = 0xd48;
const IS_CRIME_TO_ACTIVATE: u32 = 0x0057_9690;
const IS_PLAYER_TEAMMATE: u32 = 0x0056_6950;
const IS_IN_COMBAT_WITH_ACTOR: u32 = 0x008b_c700;

fn hostile_compass_target(e: &Engine, player: u32, target: u32) -> bool {
    let mut node = e.mem.u32(player + PERCEIVED_ACTORS);
    while node != 0 {
        let item = e.mem.u32(node);
        let next = e.mem.u32(node + 4);
        // BSSimpleList::IsEmpty checks both words. Do not replace this search
        // with 0093dee0: that returns the first matching entry's hostile byte;
        // JohnnyGuitar looks for any matching hostile entry.
        if next == 0 && item == 0 {
            break;
        }
        if e.mem.u8(item + 4) != 0 && e.mem.u32(item) == target {
            return true;
        }
        node = next;
    }
    false
}

pub(crate) fn is_crime_or_enemy(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    if a.this_obj != 0 {
        let actor = e.vcall(a.this_obj, 0x100, &[]).bool();
        let crime = e.call(IS_CRIME_TO_ACTIVATE, &args![a.this_obj]).bool()
            && (!actor || !e.call(IS_PLAYER_TEAMMATE, &args![a.this_obj]).bool());
        let enemy = if !crime && actor {
            let player = e.mem.u32(PLAYER);
            e.call(IS_IN_COMBAT_WITH_ACTOR, &args![a.this_obj, player])
                .bool()
                || hostile_compass_target(e, player, a.this_obj)
        } else {
            false
        };
        let result = f64::from(crime || enemy);
        a.set_result(e, result);
        // JG asks Script::GetConsoleOutput (005e23f0, TLS +268), which does
        // not additionally require the console-open global used by xNVSE's
        // IsConsoleMode helper.
        if e.call(0x005e_23f0, &[]).bool() {
            console_print(e, "IsCrimeOrEnemy >> %.f", &double_words(result));
        }
    }
    handled(true)
}

#[cfg(test)]
mod tests;
