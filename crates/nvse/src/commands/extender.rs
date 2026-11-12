//! xNVSE's commands about itself: its version (`GetNVSEVersion`, `GetNVSERevision`,
//! `GetNVSEBeta`, `CommandTable.cpp` (xNVSE)) and its plugins (`IsPluginInstalled`,
//! `GetPluginVersion`, `PluginManager.cpp` (xNVSE)), what nearly every extender mod asks first.
//!
//! - **The version** is that of the xNVSE release the built-in matches, 6.4.9: [`crate::VERSION`],
//!   from `nvse_version.h`'s `NVSE_VERSION_INTEGER` (6), `NVSE_VERSION_INTEGER_MINOR` (4) and
//!   `NVSE_VERSION_INTEGER_BETA` (9). That constant is the one place to change when the built-in
//!   catches up with a newer release; the co-save header reads it too ([`crate::cosave`]).
//! - **Plugins:** xNVSE answers from the plugin DLLs it loaded (`PluginManager::GetInfoByName`).
//!   No plugin DLL ever loads in nv-rs, so the answer is what nv-rs builds in, [`BUILT_IN_PLUGINS`]:
//!   none yet, so every name answers "not installed" (`IsPluginInstalled` 0, `GetPluginVersion`
//!   -1, xNVSE's answer for a plugin it did not load). This changes when JIP LN NVSE's commands are
//!   built in (I-13): JIP puts its own versions of these two in xNVSE's records at load
//!   (`patches_cmd.h` (JIP LN NVSE)), which also answer for JIP's names.
//!
//! Read from xNVSE's source and checked in the shipped DLL (xNVSE 6.4.9's `nvse_1_4.dll`: the
//! version executes and evals `10056380` to `10056470`, the plugin executes `100ADB40` and
//! `100ADBE0`; records found by name).

use super::{console_print, is_console_mode, Read};
use crate::table::{Command, Param, Returns};
use crate::{handled, CommandArgs, EvalArgs};
use engine_core::prelude::*;

/// `kParams_OneString` (xNVSE).
const ONE_STRING: &[Param] = &[Param {
    name: "string",
    kind: 0x00,
    optional: false,
}];

/// The plugin commands' name buffer (`char pluginName[256]`).
const NAME_BUFFER: usize = 256;

const fn cmd(
    opcode: u32,
    name: &'static str,
    help: &'static str,
    params: &'static [Param],
    execute: AbiFn,
    eval: Option<AbiFn>,
) -> Command {
    Command {
        opcode,
        name,
        short: "",
        help,
        needs_ref: false,
        params,
        execute,
        eval,
        returns: Returns::Default,
    }
}

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x1400, "GetNVSEVersion", "returns the installed version of NVSE", &[], get_nvse_version, Some(get_nvse_version_eval)),
    cmd(0x1401, "GetNVSERevision", "returns the numbered revision of the installed version of NVSE", &[], get_nvse_revision, Some(get_nvse_revision_eval)),
    cmd(0x1402, "GetNVSEBeta", "returns the numbered beta of the installed version of NVSE", &[], get_nvse_beta, Some(get_nvse_beta_eval)),
    cmd(0x15df, "IsPluginInstalled", "returns 1 if the specified plugin is installed, else 0", ONE_STRING, is_plugin_installed, None),
    cmd(0x15e0, "GetPluginVersion", "returns the version of the specified plugin, or -1 if the plugin is not installed", ONE_STRING, get_plugin_version, None),
];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp: Cmd_GetNVSEVersion_Eval,
// Cmd_GetNVSERevision_Eval, Cmd_GetNVSEBeta_Eval (the executes call them).
/// A part of the version as a version command answers it: the result, and in the console its
/// line with the number (`%d`).
fn version_part(e: &mut Engine, result: u32, number: u32, line: &str) {
    e.mem.set_f64(result, f64::from(number));
    if is_console_mode(e) {
        console_print(e, line, &[number]);
    }
}

fn get_nvse_version(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    version_part(e, a.result, crate::VERSION.0, "NVSE version: %d");
    handled(true)
}

fn get_nvse_version_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    version_part(e, a.result, crate::VERSION.0, "NVSE version: %d");
    handled(true)
}

fn get_nvse_revision(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    version_part(e, a.result, crate::VERSION.1, "NVSE revision: %d");
    handled(true)
}

fn get_nvse_revision_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    version_part(e, a.result, crate::VERSION.1, "NVSE revision: %d");
    handled(true)
}

fn get_nvse_beta(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    version_part(e, a.result, crate::VERSION.2, "NVSE beta: %d");
    handled(true)
}

fn get_nvse_beta_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    version_part(e, a.result, crate::VERSION.2, "NVSE beta: %d");
    handled(true)
}

/// A plugin nv-rs builds in, as xNVSE's `PluginInfo` names it: its name and version.
pub struct Plugin {
    pub name: &'static str,
    pub version: u32,
}

/// The plugins nv-rs builds in: none yet (no plugin DLL ever loads in nv-rs; JIP LN NVSE's
/// commands come with I-13).
pub const BUILT_IN_PLUGINS: &[Plugin] = &[];

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), PluginManager.cpp:
// PluginManager::GetInfoByName.
/// The first plugin of `plugins` whose name is `name` without case (`_stricmp` in the C locale:
/// A to Z against a to z, every other byte as it is).
fn plugin_by_name<'a>(plugins: &'a [Plugin], name: &[u8]) -> Option<&'a Plugin> {
    plugins
        .iter()
        .find(|p| p.name.as_bytes().eq_ignore_ascii_case(name))
}

/// The built-in plugin a plugin asks for by name (`GetPluginInfoByName`, xNVSE's command table
/// interface (xNVSE), which is `PluginManager::GetInfoByName`): JIP LN NVSE's own versions of
/// the two plugin commands ask it.
pub fn plugin_info(name: &[u8]) -> Option<&'static Plugin> {
    plugin_by_name(BUILT_IN_PLUGINS, name)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), PluginManager.cpp:
// Cmd_IsPluginInstalled_Execute.
/// 1 for a plugin of that name, else 0 (written only after a read that answered). Its name is
/// read into a 256-byte buffer ([`super::arg_in_buffer`]).
fn is_plugin_installed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if read.ok {
        let name = super::arg_in_buffer(e, &read, 0, NAME_BUFFER, "IsPluginInstalled");
        let v = installed(BUILT_IN_PLUGINS, &name);
        a.set_result(e, v);
    }
    handled(true)
}

/// `IsPluginInstalled`'s answer over a list of plugins.
fn installed(plugins: &[Plugin], name: &[u8]) -> f64 {
    if plugin_by_name(plugins, name).is_some() {
        1.0
    } else {
        0.0
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), PluginManager.cpp:
// Cmd_GetPluginVersion_Execute.
/// The plugin's version (unsigned), -1 for none. Its name is read into a 256-byte buffer, as
/// [`is_plugin_installed`] reads it.
fn get_plugin_version(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if read.ok {
        let name = super::arg_in_buffer(e, &read, 0, NAME_BUFFER, "GetPluginVersion");
        if let Some(v) = version_of(BUILT_IN_PLUGINS, &name) {
            a.set_result(e, v);
        }
    }
    handled(true)
}

/// `GetPluginVersion`'s answer over a list of plugins, `None` (the result left at -1) for none.
fn version_of(plugins: &[Plugin], name: &[u8]) -> Option<f64> {
    plugin_by_name(plugins, name).map(|p| f64::from(p.version))
}

#[cfg(test)]
#[path = "extender_tests.rs"]
mod tests;
