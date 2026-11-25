//! The actors' package and fight questions, each as the exe answers it.
//!
//! A virtual call whose slot holds one of this family's functions (or the leaf that answers 0) is
//! that function's typed call: the target decides, so a class whose slot holds anything else is
//! called through its slot as the exe calls it. The process classes' slots, as the exe's vtables
//! hold them (`BaseProcess` `01085e2c`, `LowProcess` `010886e4`, `MiddleLowProcess` `01089bcc`,
//! `MiddleHighProcess` `0108904c`, `HighProcess` `01087864`): 0x22c is `007fa950` on all five;
//! 0x27c is `00859670` on the first three; 0x128 is `00acbb70` on `BaseProcess` and `008d6f30` on
//! the others; 0x35c is `0047c850` on the first three. The actor classes (`Actor` `01084254`,
//! `Character` `01086a6c`, `Creature` `010870ac`, `PlayerCharacter` `0108aa3c`): 0x2c8 is
//! `008815a0` on all four; 0x428 is `008a02d0` on the first three and `00acbb70` on the player.

use super::FAMILY;
use crate::{draw, slot_target};
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The player, a global.
const PLAYER: u32 = 0x011d_ea3c;
/// `CombatController::IsFleeing` (Xbox PDB), called by address.
const COMBAT_CONTROLLER_IS_FLEEING: u32 = 0x0098_1990;
/// The procedure type of a flee package, which `Actor::IsFleeing` tests first.
const FLEE: i32 = 0x16;
/// The procedure type of the object `Actor::GetCombatController` answers (a combat controller).
const COMBAT: i32 = 0x12;

/// An actor's process (`Actor::pCurrentProcess`, +0x68).
fn process_of(e: &mut Engine, actor: u32) -> u32 {
    e.mem.u32(actor.wrapping_add(0x68))
}

// Typed from 0041ca90
/// A package's procedure type: the signed byte at +0x20, widened (`MOVSX`).
#[inline]
pub fn package_type(e: &mut Engine, this: u32) -> i32 {
    i32::from(e.mem.i8(this.wrapping_add(0x20)))
}

// Typed from 00493bb0
/// The byte at +0x104 (an actor's, in the callers here: the test `Actor::GetCombatController` and
/// the slot 0x2c8 make before they look at the fight).
#[inline]
pub fn byte_at_104(e: &mut Engine, this: u32) -> u8 {
    e.mem.u8(this.wrapping_add(0x104))
}

// Typed from 0055b980
/// The word at +0x2c (a package's location, in the process family's clean-up).
#[inline]
pub fn word_at_2c(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(0x2c))
}

// Typed from 008d6f30
/// The word at +0x40: a reference's parent cell, the getter most of its callers use it as, and a
/// process's current target (its slot 0x128 on every class but `BaseProcess`).
#[inline]
pub fn word_at_40(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(0x40))
}

// Typed from 0067f390
/// `PackageLocation::GetLocReference` (Xbox PDB): the reference (+8) when the location's type byte
/// (+0) is 0, a reference; otherwise 0.
#[inline]
pub fn location_reference(e: &mut Engine, this: u32) -> u32 {
    if e.mem.u8(this) == 0 {
        e.mem.u32(this.wrapping_add(8))
    } else {
        0
    }
}

// Typed from 007fa950
/// A process's slot 0x22c (every process class): the word at +4, read through `00559450` (the leaf
/// whose null check, JIP LN NVSE's, only a `this` of `fffffffc` would reach).
#[inline]
pub fn held_package(e: &mut Engine, this: u32) -> u32 {
    typed_call!(
        leaf::FAMILY,
        0x0055_9450,
        leaf::word_at_0(e, this.wrapping_add(4))
    )
}

/// A process's slot 0x22c: [`held_package`] when the slot holds it.
pub fn process_slot_22c(e: &mut Engine, process: u32) -> u32 {
    if slot_target(e, process, 0x22c) == 0x007f_a950 {
        typed_call!(FAMILY, 0x007f_a950, held_package(e, process))
    } else {
        e.vcall(process, 0x22c, &[]).u32()
    }
}

/// A process's slot 0x128: [`word_at_40`], or the leaf that answers 0, when the slot holds it.
pub fn process_slot_128(e: &mut Engine, process: u32) -> u32 {
    match slot_target(e, process, 0x128) {
        0x008d_6f30 => typed_call!(FAMILY, 0x008d_6f30, word_at_40(e, process)),
        0x00ac_bb70 => typed_call!(leaf::FAMILY, 0x00ac_bb70, leaf::zero_of_this(e, process)),
        _ => e.vcall(process, 0x128, &[]).u32(),
    }
}

/// An actor's slot 0x428: [`combat_controller`], or the leaf that answers 0 (the player's), when
/// the slot holds it.
pub fn actor_slot_428(e: &mut Engine, actor: u32) -> u32 {
    match slot_target(e, actor, 0x428) {
        0x008a_02d0 => typed_call!(FAMILY, 0x008a_02d0, combat_controller(e, actor)),
        0x00ac_bb70 => typed_call!(leaf::FAMILY, 0x00ac_bb70, leaf::zero_of_this(e, actor)),
        _ => e.vcall(actor, 0x428, &[]).u32(),
    }
}

/// An actor's slot 0x2c8: [`current_target`] when the slot holds it.
pub fn actor_slot_2c8(e: &mut Engine, actor: u32) -> u32 {
    if slot_target(e, actor, 0x2c8) == 0x0088_15a0 {
        typed_call!(FAMILY, 0x0088_15a0, current_target(e, actor))
    } else {
        e.vcall(actor, 0x2c8, &[]).u32()
    }
}

// Typed from 00859670
/// `BaseProcess::GetPackageThatIsRunning` (Xbox PDB): the process's slot 0x22c.
pub fn package_that_is_running(e: &mut Engine, this: u32) -> u32 {
    process_slot_22c(e, this)
}

// Typed from 009344a0
/// `MobileObject::GetCurrentPackage` (Xbox PDB): its process's slot 0x27c, or 0 without a process.
pub fn current_package(e: &mut Engine, this: u32) -> u32 {
    let process = process_of(e, this);
    if process == 0 {
        return 0;
    }
    if slot_target(e, process, 0x27c) == 0x0085_9670 {
        typed_call!(FAMILY, 0x0085_9670, package_that_is_running(e, process))
    } else {
        e.vcall(process, 0x27c, &[]).u32()
    }
}

// Typed from 008a02d0
/// `Actor::GetCombatController` (the base body of the actors' slot 0x428): the object the
/// process's slot 0x22c holds, when the actor has a process, its byte at +0x104 is set and that
/// object's procedure type is combat (0x12); otherwise 0.
pub fn combat_controller(e: &mut Engine, this: u32) -> u32 {
    let process = process_of(e, this);
    if process == 0 || typed_call!(FAMILY, 0x0049_3bb0, byte_at_104(e, this)) == 0 {
        return 0;
    }
    let held = process_slot_22c(e, process);
    if held != 0 && typed_call!(FAMILY, 0x0041_ca90, package_type(e, held)) == COMBAT {
        held
    } else {
        0
    }
}

// Typed from 00881570
/// `Actor::IsRunningRunOnce` (Xbox PDB): the process's slot 0x35c (the process not checked).
pub fn is_running_run_once(e: &mut Engine, this: u32) -> u8 {
    let process = process_of(e, this);
    if slot_target(e, process, 0x35c) == 0x0047_c850 {
        u8::from(typed_call!(
            draw::FAMILY,
            0x0047_c850,
            draw::shapes::answers_false(e, process)
        ))
    } else {
        e.vcall(process, 0x35c, &[]).u8()
    }
}

// Typed from 008a6650
/// `Actor::IsFleeing` (Xbox PDB): whether the actor's current package is a flee package (for the
/// player, the process (`008d8520`) is told through its slot 0x234 and the answer is false), or,
/// when `skip_checks` is 0, a package of type 0xa, or of type 0x13 while the process's slot 0x280
/// answers 1, or the actor's combat controller (its slot 0x428) flees. The type is asked anew at
/// each test, as the exe asks it.
pub fn is_fleeing(e: &mut Engine, this: u32, skip_checks: u8) -> bool {
    let package = typed_call!(FAMILY, 0x0093_44a0, current_package(e, this));
    if package == 0 {
        return false;
    }
    if typed_call!(FAMILY, 0x0041_ca90, package_type(e, package)) == FLEE {
        let player = e.mem.u32(PLAYER);
        if player == this {
            let process = typed_call!(leaf::FAMILY, 0x008d_8520, leaf::word_at_68(e, player));
            e.vcall(process, 0x234, &[]);
            return false;
        }
        return true;
    }
    if skip_checks != 0 {
        return false;
    }
    if typed_call!(FAMILY, 0x0041_ca90, package_type(e, package)) == FLEE {
        return true;
    }
    if typed_call!(FAMILY, 0x0041_ca90, package_type(e, package)) == 0xa {
        return true;
    }
    if typed_call!(FAMILY, 0x0041_ca90, package_type(e, package)) == 0x13 {
        let process = process_of(e, this);
        if e.vcall(process, 0x280, &[]).i32() == 1 {
            return true;
        }
    }
    let controller = actor_slot_428(e, this);
    if controller == 0 {
        return false;
    }
    e.call(COMBAT_CONTROLLER_IS_FLEEING, &args![controller])
        .bool()
}

// Typed from 008815a0
/// The actors' slot 0x2c8 (every actor class): 0 without a process. With one: when the byte at
/// +0x104 is set and the actor is not running a run-once package, or is fleeing, the actor's slot
/// 0x42c when that is not 0; otherwise the process's slot 0x128.
pub fn current_target(e: &mut Engine, this: u32) -> u32 {
    let process = process_of(e, this);
    if process == 0 {
        return 0;
    }
    if typed_call!(FAMILY, 0x0049_3bb0, byte_at_104(e, this)) != 0 {
        let use_actor_slot = typed_call!(FAMILY, 0x0088_1570, is_running_run_once(e, this)) == 0
            || typed_call!(FAMILY, 0x008a_6650, is_fleeing(e, this, 0));
        if use_actor_slot {
            let value = e.vcall(this, 0x42c, &[]).u32();
            if value != 0 {
                return value;
            }
        }
    }
    process_slot_128(e, process)
}
