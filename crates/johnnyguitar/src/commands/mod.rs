//! JohnnyGuitar NVSE's commands built so far, by source file (`JG/functions/fn_*.cpp`). Each module
//! lists its built commands with their handlers; [`install`] gives them to the table
//! ([`crate::table::set_handlers`]), which writes nothing while JohnnyGuitar answers "not
//! installed".

use engine_core::prelude::*;

pub mod fn_math;
pub mod fn_utility;

/// A module's built commands: opcode, execute, and the condition eval of a command a condition can
/// use.
pub type Handlers = &'static [(u32, AbiFn, Option<AbiFn>)];

/// Every module's list.
pub fn all() -> impl Iterator<Item = &'static (u32, AbiFn, Option<AbiFn>)> {
    fn_math::COMMANDS.iter().chain(fn_utility::COMMANDS)
}

/// How many commands are built.
pub fn built() -> usize {
    all().count()
}

/// Every built command's handlers into the table.
pub fn install(e: &mut Engine) {
    for &(opcode, execute, eval) in all() {
        crate::table::set_handlers(e, opcode, execute, eval);
    }
}

/// `Script::GetConsoleOuput` (`005E23F0`, `cdecl`: the running line's console flag, the thread's
/// TLS byte +0x268) and `Interface::PrintLine` (`00703C00`, `cdecl`, a format and its arguments).
const CONSOLE_OUTPUT: u32 = 0x005e_23f0;
const PRINT_LINE: u32 = 0x0070_3c00;

/// JohnnyGuitar's `if (Script::GetConsoleOuput()) Interface::PrintLine(format, ...)`: the format
/// (copied into the game's memory) and the words of its arguments (a double is two).
pub fn console_print(e: &mut Engine, format: &[u8], words: &[u32]) {
    if e.call(CONSOLE_OUTPUT, &args![]).u32() & 0xff == 0 {
        return;
    }
    // The format in a frame of the stack (the built-ins' data is never freed).
    let size = (format.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, text| {
        e.mem.write(text.addr(), format);
        e.mem.set_u8(text.addr() + format.len() as u32, 0);
        let mut all = vec![text.addr()];
        all.extend_from_slice(words);
        e.call(PRINT_LINE, &all);
    });
}

