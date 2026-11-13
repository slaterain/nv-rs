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
    // The processes' vtables (HighProcess 0x01087864, MiddleHighProcess 0x0108904C), where JIP
    // writes its hit-data routines once installed.
    e.map(0x0108_7000, 0x3000);
    nvse::table::install(&mut e);
    // The platform's XInputGetState at its import slot, as the host registers it (no controller:
    // ERROR_DEVICE_NOT_CONNECTED), for JIP's wrap there.
    e.register(0x00fd_f394, |_, _| Ret {
        eax: 0x48f,
        ..Ret::default()
    });
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

/// Two co-saves the real game wrote with xNVSE 6.4.9 and JIP LN NVSE 57.30 (the scratch copy
/// `game-xnvse`, 2026-11-07: `nvsecos1.nvse`, and `nvsecos4.nvse`, byte for byte `nvsecos2.nvse`).
/// JIP's section in each holds only `JPGF` (no flags, a cap of 1.0): no co-save of JIP's other
/// records, or of JIP with JohnnyGuitar NVSE, has been made yet.
pub const REAL_COSAVES: [(&str, &[&str]); 2] = [
    (
        "nvsecos1",
        &[
            "4e5653450100000006000400d02000040200000000140000060000001c01000053444f4d00000000b20000000a0d0046",
            "616c6c6f75744e562e65736d0d00446561644d6f6e65792e65736d1000486f6e6573744865617274732e65736d11004f",
            "6c64576f726c64426c7565732e65736d10004c6f6e65736f6d65526f61642e65736d0e0054726962616c5061636b2e65",
            "736d11004d657263656e6172795061636b2e65736d0f00436c61737369635061636b2e65736d0f004361726176616e50",
            "61636b2e65736d150047756e52756e6e657273417273656e616c2e65736d535652410200000000000000525652410200",
            "000022000000ff03000000010101000000ff0200000001000000000000f83f0100000000000004404556524102000000",
            "000000005356545300000000000000004556545300000000000000000022000001000000140000004a50474601000000",
            "08000000000000000000803f",
        ],
    ),
    (
        "nvsecos4",
        &[
            "4e5653450100000006000400d020000402000000001400000b000000d401000053444f4d00000000b20000000a0d0046",
            "616c6c6f75744e562e65736d0d00446561644d6f6e65792e65736d1000486f6e6573744865617274732e65736d11004f",
            "6c64576f726c64426c7565732e65736d10004c6f6e65736f6d65526f61642e65736d0e0054726962616c5061636b2e65",
            "736d11004d657263656e6172795061636b2e65736d0f00436c61737369635061636b2e65736d0f004361726176616e50",
            "61636b2e65736d150047756e52756e6e657273417273656e616c2e65736d535652410200000000000000525652410200",
            "0000280000000205000000030001000000010200000001004101000000000000f03f0100620100000000000000405256",
            "5241020000002700000001020000000101010000000104000000010000000000000a4003010078023412000104050000",
            "0052565241020000002b00000003090000000100010000000302000000000000000000f0bf0214000000000000000000",
            "254003030074656e45565241020000000000000053565453000000000000000052565453000000000c00000000010000",
            "000500616c70686152565453000000000d000000010400000006004dfc6c6c657252565453000000000b000000040700",
            "00000400666f75724556545300000000000000000022000001000000140000004a504746010000000800000000000000",
            "0000803f",
        ],
    ),
];

pub fn unhex(lines: &[&str]) -> Vec<u8> {
    let hex: String = lines.concat();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

/// Script reference `index` (from 1) is `form`: the reference list at the script's `+0x44`, each
/// entry's form at `+8` (as `nvse::args` reads an `r` argument).
pub fn script_ref(e: &mut Engine, index: u16, form: u32) {
    let i = u32::from(index);
    let ref_var = 0x0300_a000 + i * 0x10;
    e.mem.set_u32(ref_var + 8, form);
    e.mem.set_u32(ref_var + 0xc, 0);
    let mut node = SCRIPT + 0x44;
    for at in 1..i {
        let next = e.mem.u32(node + 4);
        node = if next == 0 {
            let made = 0x0300_b000 + (at + 1) * 8;
            e.mem.set_u32(node + 4, made);
            made
        } else {
            next
        };
    }
    e.mem.set_u32(node, ref_var);
}

/// A form argument: script reference `index` (`r`).
pub fn form_arg(index: u16) -> Vec<u8> {
    let mut b = vec![b'r'];
    b.extend_from_slice(&index.to_le_bytes());
    b
}

/// An integer argument (`n`).
pub fn int_arg(v: i32) -> Vec<u8> {
    let mut b = vec![b'n'];
    b.extend_from_slice(&v.to_le_bytes());
    b
}
