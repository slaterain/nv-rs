//! What the crate's command tests share: an engine with xNVSE's table (over stand-ins of the exe's
//! two tables), JohnnyGuitar installed answering "installed" or not, and a call through a command's
//! record, as `EvaluateLine` makes it (the same shape as `crates/jip`'s test support).

use engine_core::prelude::*;

/// A call's guest data: the compiled line, its offset word, the result, the script, its event list.
pub const DATA: u32 = 0x0300_0000;
pub const OFFSET: u32 = 0x0300_0f00;
pub const RESULT: u32 = 0x0300_0f08;
pub const SCRIPT: u32 = 0x0300_1000;
pub const LIST: u32 = 0x0300_2000;

/// An engine with xNVSE's table and JohnnyGuitar installed (`installed`: answering "installed").
pub fn engine(installed: bool) -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(DATA, 0x1_0000);
    // The game functions JohnnyGuitar wraps or calls, which this crate's test program does not
    // link: `GetFormEditorIDLength`, `GetFormByEditorID` (the wraps' bases), the console flag
    // (off: nothing printed) and `PrintLine`.
    for at in [0x0047_4cb0u32, 0x0048_3a00, 0x005e_23f0, 0x0070_3c00] {
        if !e.is_registered(at) {
            e.register(at, |_, _| Ret::default());
        }
    }
    nvse::table::install(&mut e);
    crate::set_answers_installed(&mut e, installed);
    crate::install(&mut e);
    e
}

/// A string argument as the compiler writes it: its length, then its bytes.
pub fn s(text: &[u8]) -> Vec<u8> {
    let mut b = (text.len() as u16).to_le_bytes().to_vec();
    b.extend_from_slice(text);
    b
}

/// A float argument (`z` and the double, as the compiler writes a number literal).
pub fn num(v: f64) -> Vec<u8> {
    let mut b = vec![b'z'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// An integer argument (`n`).
pub fn int(v: i32) -> Vec<u8> {
    let mut b = vec![b'n'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}

/// The eight handler words of a call to `opcode` with `args` (its line at [`DATA`], the result 0).
pub fn line(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> [u32; 8] {
    e.mem.set_u16(DATA, opcode as u16);
    let bytes: Vec<u8> = args.concat();
    e.mem.set_u16(DATA + 2, bytes.len() as u16 + 2);
    e.mem.set_u16(DATA + 4, args.len() as u16);
    for (i, b) in bytes.iter().enumerate() {
        e.mem.set_u8(DATA + 6 + i as u32, *b);
    }
    e.mem.set_u32(OFFSET, 4);
    e.mem.set_f64(RESULT, 0.0);
    let record = nvse::table::record_of(e, opcode);
    let params = e.mem.u32(record + 0x14);
    [params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET]
}

/// Calls `opcode` through its record's execute: the handler's `al` and the result.
pub fn call(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> (u32, f64) {
    let words = line(e, opcode, args);
    let execute = e.mem.u32(nvse::table::record_of(e, opcode) + 0x18);
    let ok = e.call(execute, &words).eax & 0xff;
    (ok, e.mem.f64(RESULT))
}
