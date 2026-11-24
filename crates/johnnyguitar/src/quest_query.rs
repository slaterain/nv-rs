//! `GetQuestFailed` / `GetQF`, translated from JohnnyGuitarNVSE 59049a0:
//! `JG/functions/fn_form.cpp` (1420–1436), `fn_form.h` (40), and
//! `nvse/nvse/ParamInfos.h` (`kParams_OneQuest`). The partial plugin stays unlisted.
use engine_core::prelude::*;
use nvse::commands::{console_print, double_words, Read};
use nvse::table::{self, PluginRecord, Returns};
use nvse::{handled, CommandArgs, EvalArgs};

const OPCODE: u32 = 0x3165;
/// `TESForm::GetFormType` reads its type byte at +4 (00401170), not its vtable.
const QUEST_KIND: u8 = 0x47;
/// `TESQuest::GetFailed` (0077A590) calls 005A8080 and tests flag 0x40.
const GET_FAILED: u32 = 0x0077_a590;
/// `Script::GetConsoleOuput` (005E23F0): TLS +268, without a console-open check.
const CONSOLE_OUTPUT: u32 = 0x005e_23f0;

pub(crate) fn install(e: &mut Engine) {
    table::grow_to(e, OPCODE + 1);
    let params = table::alloc(e, 12);
    let name = table::guest_str(e, b"quest");
    e.mem.set_u32(params, name);
    e.mem.set_u32(params + 4, 0x0e); // kParamType_Quest
    e.mem.set_u32(params + 8, 0); // Required, as kParams_OneQuest.
    let execute_address = table::execute_address(OPCODE);
    let eval_address = table::eval_address(OPCODE);
    e.register(execute_address, execute);
    e.register(eval_address, eval);
    table::add_plugin_record(
        e,
        OPCODE,
        &PluginRecord {
            name: "GetQuestFailed",
            short: "GetQF",
            needs_ref: false,
            param_count: 1,
            params,
            execute: execute_address,
            eval: eval_address,
            flags: 0,
            expression: false,
            returns: Returns::Default,
        },
    );
}

fn failed(e: &mut Engine, quest: u32) -> f64 {
    if quest != 0 && e.mem.u8(quest + 4) == QUEST_KIND {
        f64::from(e.call(GET_FAILED, &args![quest]).bool())
    } else {
        0.0
    }
}

fn execute(e: &mut Engine, words: &[u32]) -> Ret {
    let a = CommandArgs::new(words);
    let read = Read::of(e, &a);
    a.set_result(e, 0.0);
    // JG ignores ExtractArgsEx's return: the pointer starts null and any argument
    // already stored is still passed to Eval. Missing/invalid reads thus answer 0.
    let value = failed(e, read.form(0));
    a.set_result(e, value);
    if e.call(CONSOLE_OUTPUT, &[]).bool() {
        console_print(e, "GetQuestFailed >> %.2f", &double_words(value));
    }
    handled(true)
}

fn eval(e: &mut Engine, words: &[u32]) -> Ret {
    let a = EvalArgs::new(words);
    e.mem.set_f64(a.result, 0.0);
    let value = failed(e, a.arg1);
    e.mem.set_f64(a.result, value);
    handled(true)
}

#[cfg(test)]
mod tests;
