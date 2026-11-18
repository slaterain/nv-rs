//! `functions_jip/jip_fn_actor.h` (JIP LN NVSE): `ReloadCharController`, an actor's character
//! controller made again. The rest of the file's commands are in `jip_fn_actor.rs` and
//! `jip_fn_actor_p2.rs` (`RefreshAnimData` with JIP's `Actor::RefreshAnimData`, which
//! `ReloadEquippedModels` also calls, is the other lane's, in `jip_fn_actor_p2.rs`).
//!
//! What it does (every call checked in the exe, Ghidra, FalloutNV.exe 1.4.0.525): on an actor
//! (`IS_ACTOR`, vtable +0x100 answering `008D0360`) whose process (+0x68) has a level (+0x28) of
//! 0 or 1 (unsigned), the process's `SetCharacterController(null)` (vtable +0x290, one argument),
//! then the actor's vtable +0x1C4 (JIP's `AnimateNiNode`; `MobileObject::InitHavok` in the Xbox
//! PDB: `Character` `00930C70`, `PlayerCharacter` `0095D3F0`, each `RET`).
//!
//! Threads: on the script's thread, as JIP's (no lock).

use super::nif_nodes as nif;
use super::{is_actor, Handlers};
use engine_core::prelude::*;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x28dd, reload_char_controller),
];

/// `Actor::baseProcess` (+0x68) and a process's level (+0x28).
const PROCESS: u32 = 0x68;
const PROCESS_LEVEL: u32 = 0x28;
/// The process's `SetCharacterController` (vtable +0x290) and the actor's +0x1C4.
const SET_CHARACTER_CONTROLLER_SLOT: u32 = 0x290;
const ANIMATE_NI_NODE_SLOT: u32 = 0x1c4;

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_ReloadCharController_Execute.
/// On an actor with a high or middle-high process: its character controller dropped and its
/// Havok set up again (see the module's doc). No result.
fn reload_char_controller(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let actor = a.this_obj;
    if !is_actor(e, actor) {
        return handled(true);
    }
    let Some(process) = nif::ptr(e, actor.wrapping_add(PROCESS)) else {
        return handled(true);
    };
    if e.mem.u32(process.wrapping_add(PROCESS_LEVEL)) > 1 {
        return handled(true);
    }
    e.vcall(process, SET_CHARACTER_CONTROLLER_SLOT, &args![0u32]);
    e.vcall(actor, ANIMATE_NI_NODE_SLOT, &[]);
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_actor_anim_tests.rs"]
mod tests;
