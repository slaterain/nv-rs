//! `GetCalculatedWeaponDPS`, translated from JohnnyGuitarNVSE 59049a0,
//! JG/functions/fn_form.cpp:58 and Cmd_GetCalculatedWeaponDPS_Execute.
//! The original game's formula stays at 00645380; this is its command's
//! reference/condition/ammo selection, not a second damage formula.

use engine_core::prelude::*;
use nvse::commands::{console_print, double_words, Read};
use nvse::{handled, CommandArgs};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

const PLAYER: u32 = 0x011d_ea3c;
const WEAPON_KIND: u8 = 0x28;
const WEAPON_DPS: u32 = 0x0064_5380;
const REFERENCE_HEALTH: u32 = 0x0056_8ad0;
const ITEM_HEALTH: u32 = 0x004b_cdb0;
const WEAPON_AMMO: u32 = 0x0052_5980;
/// JG's Script::GetConsoleOutput: TLS +268, without the console-open flag.
const CONSOLE_OUTPUT: u32 = 0x005e_23f0;

pub(crate) fn execute(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut weapon = read.form(0);
    let mut condition = 1.0f32;
    let mut xdata = 0;
    if weapon == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        let id = e.mem.u32(a.this_obj + 0xc);
        let inv = if nvse::invref::for_ref_id(e, id) {
            nvse::invref::get(e, id).map(|r| r.data)
        } else {
            None
        };
        if let Some(data) = inv {
            if data.item != 0 && e.mem.u8(data.item + 4) == WEAPON_KIND {
                weapon = data.item;
                if data.xdata != 0 {
                    // JG's `pItemChange->GetItemHealth(true) / 100.0F`.
                    condition = e.call(ITEM_HEALTH, &args![data.entry, 1u32]).f32() / 100.0;
                    xdata = data.xdata;
                }
            }
        } else {
            weapon = e.mem.u32(a.this_obj + 0x20);
            if weapon == 0 || e.mem.u8(weapon + 4) != WEAPON_KIND {
                return handled(true);
            }
            // JG passes GetHealth's raw float here, unlike its inventory path.
            condition = e.call(REFERENCE_HEALTH, &args![a.this_obj]).f32();
        }
    }
    if weapon == 0 || e.mem.u8(weapon + 4) != WEAPON_KIND {
        return handled(true);
    }
    // Original precondition: the running game's player has a live process.
    // Always the player, even for an inventory reference held by an NPC.
    let player = e.mem.u32(PLAYER);
    let process = e.mem.u32(player + 0x68);
    let item = e.vcall(process, 0x148, &args![]).u32();
    let mut ammo = 0;
    if xdata == 0 && item != 0 && e.mem.u32(item + 8) == weapon {
        let loaded = e.vcall(process, 0x14c, &args![]).u32();
        if loaded != 0 {
            ammo = e.vcall(process, 0x14c, &args![]).u32();
            ammo = e.mem.u32(ammo + 8);
        }
    }
    if ammo == 0 {
        ammo = e.call(WEAPON_AMMO, &args![weapon, 0u32]).u32();
    }
    e.mem.set_u32(process + 0x114, 0);
    // The implicit C++ PlayerCharacter* -> ActorValueOwner* conversion adds
    // 0xa4 (00645380 callers, e.g. recipemenu 00728da0, FalloutNV 1.4.0.525).
    // Restore even when the translated formula stops/panics; then propagate
    // that stop rather than leaving the process in the temporary state.
    let value = catch_unwind(AssertUnwindSafe(|| {
        e.call(
            WEAPON_DPS,
            &args![
                player + 0xa4,
                weapon,
                condition,
                1u32,
                item,
                0u32,
                0u32,
                u32::MAX,
                0.0f32,
                0.0f32,
                0u32,
                0u32,
                ammo
            ],
        )
        .f32()
    }));
    e.mem.set_u32(process + 0x114, item);
    let value = match value {
        Ok(value) => f64::from(value),
        Err(stop) => resume_unwind(stop),
    };
    a.set_result(e, value);
    if e.call(CONSOLE_OUTPUT, &[]).bool() {
        console_print(e, "GetCalculatedWeaponDPS >> %f", &double_words(value));
    }
    handled(true)
}

#[cfg(test)]
mod tests;
