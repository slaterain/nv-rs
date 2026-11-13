//! `functions_jip/jip_fn_package.h` (JIP LN NVSE): an AI package's flags. All 2 of its commands.
//! The package is read as `kParamType_AIPackage`, whose type check xNVSE's reader makes. The flags
//! are the word at +0x1C (the package loader `00673A40` stores `PKDT`'s first word there).

use super::forms::TESPackage;
use super::Handlers;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2624, get_package_flag),
    (0x2625, set_package_flag),
];

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_package.h:
// Cmd_GetPackageFlag_Execute.
fn get_package_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if read.ok && flag <= 31 {
        let p = Ptr::<TESPackage>::new(read.form(0));
        if e.get(p, TESPackage::packageFlags) & (1 << flag) != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_package.h:
// Cmd_SetPackageFlag_Execute.
fn set_package_flag(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let flag = read.int(1, 0);
    if read.ok && flag <= 31 {
        let p = Ptr::<TESPackage>::new(read.form(0));
        let bit = 1u32 << flag;
        let flags = e.get(p, TESPackage::packageFlags);
        let flags = if read.int(2, 0) != 0 {
            flags | bit
        } else {
            flags & !bit
        };
        e.set(p, TESPackage::packageFlags, flags);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_package_tests.rs"]
mod tests;
