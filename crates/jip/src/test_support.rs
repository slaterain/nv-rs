//! What the crate's tests share: an engine with xNVSE's table (over a stand-in of the exe's two
//! tables) and a call through a command's record, as `EvaluateLine` makes it.

use engine_core::prelude::*;

/// A call's guest data: the compiled line, its offset word, the result, the script and its
/// event list (nothing in them: the identity commands read plain strings).
pub const DATA: u32 = 0x0300_0000;
pub const OFFSET: u32 = 0x0300_0f00;
pub const RESULT: u32 = 0x0300_0f08;
pub const SCRIPT: u32 = 0x0300_1000;
pub const LIST: u32 = 0x0300_2000;

/// The console's open byte (`0x011DEA2E`) and its line flag in the thread's TLS block (`+0x268`):
/// both set is the console mode JIP's echo asks for.
const CONSOLE_OPEN: u32 = 0x011d_ea2e;

/// An engine with xNVSE's table installed over empty stand-ins of the exe's console and script
/// tables, and room for a call's data; JIP not yet installed.
pub fn engine() -> Engine {
    let mut e = Engine::new();
    e.map(0x0118_e000, 0x9000);
    e.map(CONSOLE_OPEN & !0xfff, 0x1000);
    e.map(DATA, 0x1_0000);
    nvse::table::install(&mut e);
    e
}

/// Opens or closes the console as the echo sees it.
pub fn console(e: &mut Engine, open: bool) {
    e.mem.set_u8(CONSOLE_OPEN, u8::from(open));
    let tls = e.tls();
    e.mem.set_u8(tls + 0x268, u8::from(open));
}

/// A string argument as the compiler writes it: its length, then its bytes.
pub fn s(text: &[u8]) -> Vec<u8> {
    let mut b = (text.len() as u16).to_le_bytes().to_vec();
    b.extend_from_slice(text);
    b
}

/// The eight handler words of a call to `opcode` with `args`, its line written at [`DATA`] and
/// the result set to 0 (as xNVSE zeroes it before an extender command runs).
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
    assert_ne!(record, 0, "{opcode:04x}");
    let params = e.mem.u32(record + 0x14);
    [params, DATA, 0, 0, SCRIPT, LIST, RESULT, OFFSET]
}

/// The execute word of `opcode`'s record.
pub fn execute_of(e: &Engine, opcode: u32) -> u32 {
    e.mem.u32(nvse::table::record_of(e, opcode) + 0x18)
}

/// Calls `opcode` through its record's execute with `args`: the handler's `al` and the result.
pub fn call(e: &mut Engine, opcode: u32, args: &[Vec<u8>]) -> (u32, f64) {
    let words = line(e, opcode, args);
    let execute = execute_of(e, opcode);
    let ok = e.call(execute, &words).eax & 0xff;
    (ok, e.mem.f64(RESULT))
}
