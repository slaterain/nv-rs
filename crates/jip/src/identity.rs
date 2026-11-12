//! JIP LN NVSE's identity: what a mod asks to learn whether JIP is there and which version.
//! `GetLNVersion` (`0x2200`, `functions_ln/ln_fn_utility.h` (JIP LN NVSE)), `GetNVSEVersionFull`
//! (`0x2210`) and JIP's own versions of xNVSE's `IsPluginInstalled` (`0x15DF`) and
//! `GetPluginVersion` (`0x15E0`), which JIP puts in xNVSE's records at its start and which also
//! answer for JIP's names (`InitCmdPatches`, `internal/patches_cmd.h`).
//!
//! All of it is there only while JIP answers "installed" ([`crate::ANSWERS_INSTALLED`]): until
//! then `GetLNVersion` and `GetNVSEVersionFull` have no handler (a call fails, as without the
//! DLL) and xNVSE's own plugin commands answer, for which no plugin is installed. JIP's copy of
//! `GetNVSEVersionFull` over the exe's `0x127D` changes a vanilla command, so it is a fix row of
//! its own (the plan's step 3.9), not part of this.

use crate::console::print_result;
use crate::utility::{str_begins_ci, str_compare_ci};
use engine_core::extend::ROUTINE_BASE;
use engine_core::prelude::*;
use nvse::args::Arg;
use nvse::commands::array::{fix_on, LIST_GUARD};
use nvse::commands::extender::plugin_info;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

/// JIP LN NVSE's version, 57.30 (`JIP_LN_VERSION_MAJ`, `_MIN`, `internal/version.h`).
pub const VERSION_MAJ: u32 = 57;
pub const VERSION_MIN: u32 = 30;
/// `JIP_LN_VERSION_INT`, the version JIP gives xNVSE's plugin list: 5730.
pub const VERSION_INT: u32 = VERSION_MIN + VERSION_MAJ * 100;

/// `JIP_LN_VERSION` as JIP computes it (`57 + 30 * 0.01`), what its commands answer: 57.3, the
/// double the shipped DLL loads (`0x404CA66666666666`).
pub fn version() -> f64 {
    f64::from(VERSION_MAJ) + f64::from(VERSION_MIN) * 0.01
}

/// xNVSE's version as JIP keeps it (`s_nvseVersion`, set in `NVSEPlugin_Query` (`jip_nvse.cpp`)
/// from the number xNVSE gives its plugins): the top byte, plus the next one times 0.1, plus the
/// low byte's top four bits times 0.01. For xNVSE 6.4.9 (`0x06040090`), 6.49.
pub fn nvse_version_full() -> f64 {
    let version = nvse::PACKED_VERSION as i32;
    f64::from(version >> 24)
        + f64::from((version >> 16) & 0xff) * 0.1
        + f64::from((version & 0xff) >> 4) * 0.01
}

/// The routines JIP puts in xNVSE's records of `IsPluginInstalled` and `GetPluginVersion`: JIP's
/// named code, in the extenders' routine range above xNVSE's (which end below `+0x1000`).
pub const ROUTINES: u32 = ROUTINE_BASE + 0x1000;
pub const IS_PLUGIN_INSTALLED: u32 = ROUTINES;
pub const GET_PLUGIN_VERSION: u32 = ROUTINES + 4;

// Above xNVSE's last routines (its fake DirectInput device's 32 slots) and inside the range.
const _: () = assert!(
    nvse::input::DEVICE_CODE + 4 * 32 <= ROUTINES
        && GET_PLUGIN_VERSION < engine_core::mem::EXTENDER_CODE.end
);

/// The opcodes: xNVSE's two plugin commands and JIP's two version commands.
pub const XNVSE_IS_PLUGIN_INSTALLED: u32 = 0x15df;
pub const XNVSE_GET_PLUGIN_VERSION: u32 = 0x15e0;
pub const GET_LN_VERSION: u32 = 0x2200;
pub const GET_NVSE_VERSION_FULL: u32 = 0x2210;

/// JIP's plugin-name buffer (`char pluginName[0x80]`).
const NAME_BUFFER: usize = 0x80;

/// Puts the identity in, when JIP answers "installed": the two version commands' handlers, and
/// JIP's two plugin routines in xNVSE's records (as `InitCmdPatches` writes them at xNVSE's
/// post-load message).
pub fn install(e: &mut Engine, answers_installed: bool) {
    if !answers_installed {
        return;
    }
    crate::table::set_handlers(e, GET_LN_VERSION, Some(get_ln_version), None);
    crate::table::set_handlers(e, GET_NVSE_VERSION_FULL, Some(get_nvse_version_full), None);
    e.register(IS_PLUGIN_INSTALLED, is_plugin_installed);
    e.register(GET_PLUGIN_VERSION, get_plugin_version);
    let t = e.script_tables();
    e.mem.set_u32(
        t.record(XNVSE_IS_PLUGIN_INSTALLED) + 0x18,
        IS_PLUGIN_INSTALLED,
    );
    e.mem.set_u32(
        t.record(XNVSE_GET_PLUGIN_VERSION) + 0x18,
        GET_PLUGIN_VERSION,
    );
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_ln/ln_fn_utility.h:
// Cmd_GetLNVersion_Execute.
fn get_ln_version(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, version());
    print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h:
// Cmd_GetNVSEVersionFull_Execute.
fn get_nvse_version_full(e: &mut Engine, w: &[u32]) -> Ret {
    CommandArgs::new(w).set_result(e, nvse_version_full());
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h: IsJIPAlias.
/// JIP's own names: `JIP NVSE Plugin` and `lutana_nvse` without case, and any name that begins
/// with `JIP LN NVSE` (JIP's compares, [`str_compare_ci`]).
pub fn is_jip_alias(name: &[u8]) -> bool {
    str_compare_ci(name, b"JIP NVSE Plugin").0 == 0
        || str_compare_ci(name, b"lutana_nvse").0 == 0
        || str_begins_ci(name, b"JIP LN NVSE") != 0
}

/// String argument 0 as JIP's 128-byte name buffer holds it: its text up to the first 0. A
/// text of 128 bytes or more runs past the buffer into JIP's frame; the rule xNVSE's buffers have
/// (`nvse::commands::arg_in_buffer`): with the list guard row it is read whole, without it the
/// port stops there.
fn name_in_buffer(e: &Engine, read: &Read, command: &str) -> Vec<u8> {
    let written = match read.args.first() {
        Some(Arg::Str { raw, text }) => raw.len().max(text.len()),
        _ => 0,
    };
    if written >= NAME_BUFFER && !fix_on(e, LIST_GUARD) {
        panic!(
            "{command}: a text of {written} bytes runs past JIP's {NAME_BUFFER}-byte buffer into \
             its frame: the port stops here (NV_NVSE_LIST_GUARD reads it whole)"
        );
    }
    let text = read.text(0);
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    text[..end].to_vec()
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h:
// Hook_IsPluginInstalled_Execute.
/// 1 for a built-in plugin of that name or one of JIP's own names; otherwise the result is left
/// as it is (xNVSE zeroes it before an extender command runs).
fn is_plugin_installed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let name = name_in_buffer(e, &read, "IsPluginInstalled");
        if plugin_info(&name).is_some() || is_jip_alias(&name) {
            a.set_result(e, 1.0);
        }
    }
    print_result(e, &a);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_cmd.h:
// Hook_GetPluginVersion_Execute.
/// -1, or JIP's version for one of its own names, or a built-in plugin's version as a signed
/// integer.
fn get_plugin_version(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if read.ok {
        let name = name_in_buffer(e, &read, "GetPluginVersion");
        if is_jip_alias(&name) {
            a.set_result(e, version());
        } else if let Some(plugin) = plugin_info(&name) {
            a.set_result(e, f64::from(plugin.version as i32));
        }
    }
    print_result(e, &a);
    handled(true)
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
