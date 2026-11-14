//! `nvse.log`, xNVSE's log (`IDebugLog gLog("nvse.log")`, `common/IDebugLog.cpp` (xNVSE)), for
//! the lines the built-in writes there: the documentation dumps ([`crate::docs`]) and the first
//! frame's compile of the function files ([`crate::compile_script`]).
//!
//! As xNVSE writes it: the file `nvse.log` in the game's working folder, opened `"w"` (made anew
//! each run) and in text mode, each message followed by a new line (`IDebugLog::Message` with no
//! indent and no source header: every `\n`, the message's own too, is written `\r\n`). It goes
//! through the game's own file calls, so in nv-rs it lands where the platform layer keeps every
//! file the game writes: the run's in-memory disk, not the player's folder (as `DecompileScript`'s
//! files and `ar_DumpF`'s). The other messages xNVSE logs are not written (the port keeps no
//! other log of xNVSE's), and the file is made at its first message rather than at the start.

use crate::Nvse;
use engine_core::prelude::*;

/// The log's path, relative to the game's working folder.
pub const PATH: &[u8] = b"nvse.log";

/// Whether this run made the log yet.
#[derive(Default)]
pub struct State {
    pub made: bool,
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), common/IDebugLog.cpp: IDebugLog::Message,
// PrintText, NewLine.
/// Writes one message and a new line to `nvse.log` (text mode: `\n` written as `\r\n`).
pub fn message(e: &mut Engine, text: &[u8]) {
    let mut out = Vec::with_capacity(text.len() + 2);
    for &b in text {
        if b == b'\n' {
            out.push(b'\r');
        }
        out.push(b);
    }
    out.extend_from_slice(b"\r\n");
    let append = e.ext::<Nvse>().log.made;
    if crate::commands::array::write_text_file(e, PATH, append, &out) {
        e.ext::<Nvse>().log.made = true;
    }
}
