//! `functions_jip/jip_fn_script.h` (JIP LN NVSE): scripts' events, waits, delays and blocks. 11
//! of its 24 commands; the core track built `GetScriptDisabled`, `SetScriptDisabled`,
//! `SetScriptEventDisabled`, `ScriptWait`, `SetGameMainLoopCallback` and
//! `SetOnQuestStageEventHandler` (`ai_hooks.rs`, `world_hooks.rs`, `main_loop.rs`); the two
//! getters of their state are here.
//!
//! Left for later: `AddScriptVariable`, `HasVariableAdded`, `RemoveScriptVariable` and
//! `RemoveAllAddedVariables` keep JIP's added variables in its co-save (`s_scriptVariablesBuffer`,
//! the `JPSV` record, with `Script::AddVariable`'s new `VariableInfo` and `ScriptVar` lists): the
//! co-save framework does not carry that record yet. `RunScriptSnippet`, `RunBatchScript` and
//! `RegisterSRScript` compile script text (`Script::Create`, JIP's `JIPScriptRunner`) and the last
//! two read files from the game folder: the script compiler seam and the file layer are not built.
//!
//! `Script` (the exe's layout, JIP's `nvse/GameScript.h`): its data length `info.dataLength`
//! +0x20 and quest flag +0x28 (`Script::Run`, `005AC1E0`, reads +0x38 and +0x40 beside them;
//! `ai_hooks.rs`' `SetScriptDisabled` writes +0x20), its compiled data +0x30, its quest delay
//! counter +0x38 (`Script::Run` counts it down by the frame time and returns while it is above 0).
//! `TESQuest`'s script: `TESScriptableForm` at +0x18 (`kBaseComponentOffsets` byte 1 for type
//! 0x47: 6 words), the script +4 in it. `ExtraScript` (type 0x0D; its constructor `00432000`
//! stores the script at +0xC and clears the event list at +0x10).

use super::{buffer_arg, extra, type_of, Handlers};
use crate::game::JipFormFlags;
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};
use std::collections::BTreeMap;

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2622, get_delay_elapsed),
    (0x2623, set_delay_elapsed),
    (0x27e5, get_script_event_disabled),
    (0x27e7, fake_script_event),
    (0x2811, is_script_waiting),
    (0x2812, stop_script_waiting),
    (0x2814, get_script_block_disabled),
    (0x2815, set_script_block_disabled),
    (0x2828, has_script_block),
    (0x282b, disable_scripted_activate),
    (0x283a, execute_script),
];

/// `kFormType_Script`.
const SCRIPT: u8 = 0x11;
/// `Script::info.dataLength`, `Script::data`, `Script::questDelayTimeCounter`.
const DATA_LENGTH: u32 = 0x20;
const DATA: u32 = 0x30;
const QUEST_DELAY: u32 = 0x38;
/// `TESQuest::scriptable.script` (`TESScriptableForm` +0x18, its script +4).
const QUEST_SCRIPT: u32 = 0x1c;
/// `kXData_ExtraScript`; `ExtraScript::script`, `ExtraScript::eventList`.
const EXTRA_SCRIPT: u32 = 0x0d;
const XSCRIPT_SCRIPT: u32 = 0x0c;
const XSCRIPT_EVENTS: u32 = 0x10;
/// `kHookFormFlag6_EventDisabled`, `kHookFormFlag6_ActivateDisabled`,
/// `kHookFormFlag5_ScriptOnWait` (`internal/hooks.h` (JIP LN NVSE), the bits `world_hooks.rs`
/// sets).
const EVENT_DISABLED: u8 = 1 << 2;
const ACTIVATE_DISABLED: u8 = 1 << 3;
const SCRIPT_ON_WAIT: u8 = 1 << 3;
/// `ScriptLocals::SetActionFlag` (`005A8E20`, `thiscall`: the event list, the filter form, the
/// mask; answers whether an event entry took the flag): JIP's `FakeScriptEvent` call.
const SET_ACTION_FLAG: u32 = 0x005a_8e20;
/// `Script::Run` (`005AC1E0`, `thiscall`: the script, the reference, the event list, the
/// containing reference, the "always run" flag): JIP's `Script::Execute`.
const SCRIPT_RUN: u32 = 0x005a_c1e0;
/// The highest block type the block commands take.
const MAX_BLOCK_TYPE: u32 = 0x25;
/// The type a disabled block gets (`DisabledBlockInfo`'s constructor).
const DISABLED_TYPE: u8 = 0x0e;

/// `DisabledBlockInfo`: a block's own type and the address of its type byte (0x0E while
/// disabled).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisabledBlock {
    pub block_type: u8,
    pub type_at: u32,
}

/// What the script commands keep: `s_disabledScriptBlocksMap` (by script, its disabled blocks
/// in the order disabled). Never saved, never cleared (JIP's `TempObject`).
#[derive(Default)]
pub struct State {
    pub disabled_blocks: BTreeMap<u32, Vec<DisabledBlock>>,
}

fn state(e: &mut Engine) -> &mut State {
    &mut e.ext::<Jip>().script
}

fn flags5(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<JipFormFlags>::new(form), JipFormFlags::jipFormFlags5)
}

fn flags6(e: &Engine, form: u32) -> u8 {
    e.get(Ptr::<JipFormFlags>::new(form), JipFormFlags::jipFormFlags6)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_GetDelayElapsed_Execute.
/// The quest script's delay counter (a float), when the quest has a script.
fn get_delay_elapsed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let quest = read.form(0);
    if read.ok && quest != 0 {
        let script = e.mem.u32(quest + QUEST_SCRIPT);
        if script != 0 {
            let delay = e.mem.f32(script + QUEST_DELAY);
            a.set_result(e, f64::from(delay));
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_SetDelayElapsed_Execute.
fn set_delay_elapsed(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let quest = read.form(0);
    if read.ok && quest != 0 {
        let script = e.mem.u32(quest + QUEST_SCRIPT);
        if script != 0 {
            let delay = read.float(1, 0.0);
            e.mem.set_f32(script + QUEST_DELAY, delay);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_GetScriptEventDisabled_Execute.
/// 1 when the named event (JIP's 0x40-byte buffer, the whole text one name) is disabled for the
/// form: an event with a mask by the form's flag and its disabled mask, "OnActivate" (mask 0) by
/// the activation flag. An unknown name: 0.
fn get_script_event_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let form = read.form(0);
    let name = buffer_arg(e, &read, 1, 0x40, "GetScriptEventDisabled");
    // JIP reads the form's flags unchecked: a null form never reaches here from compiled data
    // (the reader fails first); the port answers 0 for one.
    if form == 0 {
        return handled(true);
    }
    if let Some(mask) = crate::world_hooks::event_mask(&name) {
        let flags = flags6(e, form);
        let disabled = if mask != 0 {
            flags & EVENT_DISABLED != 0
                && e
                    .ext::<Jip>()
                    .world_hooks
                    .disabled_events
                    .get(&form)
                    .copied()
                    .unwrap_or(0)
                    & mask
                    != 0
        } else {
            flags & ACTIVATE_DISABLED != 0
        };
        if disabled {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_FakeScriptEvent_Execute.
/// The named event (a name with a mask: "OnActivate" and unknown names do nothing) set on the
/// calling reference's script's event list, with the filter form: the inventory reference's
/// own extra data when the calling reference is one, else the reference's. 1 when an entry of
/// the list took it.
fn fake_script_event(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let name = buffer_arg(e, &read, 0, 0x40, "FakeScriptEvent");
    let filter = read.form(1);
    let mask = crate::world_hooks::event_mask(&name).unwrap_or(0);
    if mask == 0 {
        return handled(true);
    }
    let id = super::ref_id(e, a.this_obj);
    let list = if nvse::invref::for_ref_id(e, id) {
        nvse::invref::get(e, id).map_or(0, |r| r.data.xdata)
    } else {
        a.this_obj + extra::REFR_EXTRA_LIST
    };
    if list == 0 {
        return handled(true);
    }
    let xscript = extra::get_by_type(e, list, EXTRA_SCRIPT);
    if xscript == 0 {
        return handled(true);
    }
    let events = e.mem.u32(xscript + XSCRIPT_EVENTS);
    if events != 0
        && e.call(SET_ACTION_FLAG, &args![events, filter, mask]).u32() & 0xff != 0
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

/// The owner a wait command names: the quest given, else the calling reference (0: none).
fn wait_owner(read: &Read, a: &CommandArgs) -> u32 {
    let owner = read.form(0);
    if owner != 0 {
        owner
    } else {
        a.this_obj
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_IsScriptWaiting_Execute.
/// 1 when the quest (or the calling reference) has a script waiting (`ScriptWait`'s flag).
fn is_script_waiting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let owner = wait_owner(&read, &a);
        if owner != 0 && flags5(e, owner) & SCRIPT_ON_WAIT != 0 {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_StopScriptWaiting_Execute.
/// A waiting owner's wait cut to its last block: the next count-down ends it.
fn stop_script_waiting(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if read.ok {
        let owner = wait_owner(&read, &a);
        if owner != 0 && flags5(e, owner) & SCRIPT_ON_WAIT != 0 {
            if let Some(wait) = e.ext::<Jip>().world_hooks.script_waits.get_mut(&owner) {
                wait.iterations = 1;
            }
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.h: ScriptBlockIterator.
/// The blocks of compiled script data: data that begins with `ScriptName` (0x1D), then each
/// `Begin` (0x10, its length word, the block type at the start of its operands, the length of
/// its body two bytes on) closed by an `End` (the word 0x11). A block that does not close ends
/// the walk.
struct Blocks {
    data: u32,
    length: u32,
    at: u32,
    type_at: u32,
}

impl Blocks {
    fn new(e: &Engine, data: u32, length: u32) -> Blocks {
        let mut b = Blocks {
            data,
            length,
            at: 4,
            type_at: 0,
        };
        if data == 0 || e.mem.u32(data) != 0x1d || !b.eval(e) {
            b.length = 0;
            b.type_at = 0;
        }
        b
    }

    fn eval(&mut self, e: &Engine) -> bool {
        if e.mem.u16(self.data.wrapping_add(self.at)) != 0x10 {
            return false;
        }
        self.at = self.at.wrapping_add(2);
        let op_size = u32::from(e.mem.u16(self.data.wrapping_add(self.at)));
        self.at = self.at.wrapping_add(2);
        self.type_at = self.data.wrapping_add(self.at);
        self.at = self.at.wrapping_add(2);
        let body = u32::from(e.mem.u16(self.data.wrapping_add(self.at)));
        self.at = self
            .at
            .wrapping_add(body.wrapping_add(op_size).wrapping_sub(6));
        if e.mem.u32(self.data.wrapping_add(self.at)) == 0x11 {
            self.at = self.at.wrapping_add(4);
            return true;
        }
        false
    }

    fn advance(&mut self, e: &Engine) {
        if self.at < self.length {
            if self.eval(e) {
                return;
            }
            self.type_at = 0;
        }
        self.length = 0;
    }

    /// The type bytes' addresses, in order.
    fn type_bytes(e: &Engine, data: u32, length: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let mut b = Blocks::new(e, data, length);
        while b.length != 0 {
            out.push(b.type_at);
            b.advance(e);
        }
        out
    }
}

/// A script's blocks' type bytes.
fn script_blocks(e: &Engine, script: u32) -> Vec<u32> {
    let data = e.mem.u32(script + DATA);
    let length = e.mem.u32(script + DATA_LENGTH);
    Blocks::type_bytes(e, data, length)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_GetScriptBlockDisabled_Execute.
/// 1 when a block of the type (at most 0x25) of the script is disabled.
fn get_script_block_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let script = read.form(0);
    let kind = read.int(1, 0);
    if read.ok && script != 0 && type_of(e, script) == SCRIPT && kind <= MAX_BLOCK_TYPE {
        let found = state(e)
            .disabled_blocks
            .get(&script)
            .is_some_and(|v| v.iter().any(|b| u32::from(b.block_type) == kind));
        if found {
            a.set_result(e, 1.0);
        }
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_SetScriptBlockDisabled_Execute; internal/jip_core.h: DisabledBlockInfo.
/// Disabling a block type (0 to 0x25) the script has none disabled of: each block of that type
/// gets type 0x0E (the script runner skips it), its own type kept. Enabling a type: every
/// disabled block of it gets its type back (from the last disabled to the first), the script
/// dropped when none is left; enabling with a negative type gives back all of the script's (in
/// the order disabled).
///
/// JIP removes the script's entry even when it has none (`findScript.Remove()` on an iterator
/// that found nothing: the count goes down and the bucket walk reads a null entry); the port
/// does nothing then.
fn set_script_block_disabled(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let script = read.form(0);
    if !read.ok || script == 0 || type_of(e, script) != SCRIPT {
        return handled(true);
    }
    let disable = read.int(1, 0) as i32;
    let kind = read.int(2, u32::MAX) as i32;
    if kind > MAX_BLOCK_TYPE as i32 {
        return handled(true);
    }
    if disable != 0 {
        let already = state(e)
            .disabled_blocks
            .get(&script)
            .is_some_and(|v| v.iter().any(|b| i32::from(b.block_type) == kind));
        if kind < 0 || already {
            return handled(true);
        }
        for type_at in script_blocks(e, script) {
            let block_type = e.mem.u8(type_at);
            if i32::from(block_type) != kind {
                continue;
            }
            e.mem.set_u8(type_at, DISABLED_TYPE);
            state(e)
                .disabled_blocks
                .entry(script)
                .or_default()
                .push(DisabledBlock {
                    block_type,
                    type_at,
                });
        }
        return handled(true);
    }
    let Some(mut blocks) = state(e).disabled_blocks.remove(&script) else {
        return handled(true);
    };
    if kind < 0 {
        for b in &blocks {
            e.mem.set_u8(b.type_at, b.block_type);
        }
        return handled(true);
    }
    let mut removed = false;
    for i in (0..blocks.len()).rev() {
        if i32::from(blocks[i].block_type) == kind {
            let b = blocks.remove(i);
            e.mem.set_u8(b.type_at, b.block_type);
            removed = true;
        }
    }
    if removed && blocks.is_empty() {
        return handled(true);
    }
    state(e).disabled_blocks.insert(script, blocks);
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_HasScriptBlock_Execute.
/// 1 when the script (the form itself, or its scriptable component's script; without a form the
/// calling reference's base form's) has a block of the type (at most 0x25). A disabled block
/// has type 0x0E while disabled.
fn has_script_block(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    let kind = read.int(0, 0);
    if !read.ok || kind > MAX_BLOCK_TYPE {
        return handled(true);
    }
    let mut form = read.form(1);
    if form == 0 {
        if a.this_obj == 0 {
            return handled(true);
        }
        form = e.mem.u32(a.this_obj + 0x20);
    }
    if form == 0 {
        return handled(true);
    }
    let script = if type_of(e, form) == SCRIPT {
        form
    } else {
        let scriptable = scriptable_form(e, form);
        if scriptable == 0 {
            return handled(true);
        }
        let script = e.mem.u32(scriptable + 4);
        if script == 0 {
            return handled(true);
        }
        script
    };
    if script_blocks(e, script)
        .into_iter()
        .any(|at| u32::from(e.mem.u8(at)) == kind)
    {
        a.set_result(e, 1.0);
    }
    handled(true)
}

/// `TESForm::GetScriptableForm` (JIP LN NVSE): `kBaseComponentOffsets` byte 1, 0 for none.
fn scriptable_form(e: &Engine, form: u32) -> u32 {
    let Some(&entry) = super::forms::BASE_COMPONENT_OFFSETS.get(usize::from(type_of(e, form)))
    else {
        return 0;
    };
    let words = (entry >> 8) as u8 as i8;
    if words == 0 {
        0
    } else {
        form.wrapping_add((i32::from(words) * 4) as u32)
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_DisableScriptedActivate_Execute.
/// In the calling reference's script (with an event list), the `OnActivate` block's `Activate`
/// lines (opcode 0x100D) become JIP's 0x2215 (disable) or back (enable): the walk goes line by
/// line from after `ScriptName`, an `OnActivate` `Begin` (0x10 with mode 2) starts the block, its
/// `End` (0x11) stops the walk, a line through an expression (0x1C) has its opcode one word on.
fn disable_scripted_activate(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let disable = read.int(0, 0) != 0;
    let xscript = extra::refr_extra(e, a.this_obj, EXTRA_SCRIPT);
    if xscript == 0 {
        return handled(true);
    }
    let script = e.mem.u32(xscript + XSCRIPT_SCRIPT);
    if script == 0 || e.mem.u32(xscript + XSCRIPT_EVENTS) == 0 {
        return handled(true);
    }
    let data = e.mem.u32(script + DATA);
    let end = data.wrapping_add(e.mem.u32(script + DATA_LENGTH));
    let (look_for, replace) = if disable {
        (0x100d, 0x2215)
    } else {
        (0x2215, 0x100d)
    };
    let mut at = data.wrapping_add(4);
    let mut on_activate = false;
    while at < end {
        let mut opcode_at = at;
        at = at.wrapping_add(2);
        let mut length = e.mem.u16(at);
        at = at.wrapping_add(2);
        let opcode = e.mem.u16(opcode_at);
        if opcode == 0x11 {
            if on_activate {
                break;
            }
        } else if opcode == 0x10 {
            on_activate = e.mem.u16(at) == 2;
        } else {
            if opcode == 0x1c {
                opcode_at = at;
                at = at.wrapping_add(2);
                length = e.mem.u16(at);
                at = at.wrapping_add(2);
            }
            if on_activate && e.mem.u16(opcode_at) == look_for {
                e.mem.set_u16(opcode_at, replace);
            }
        }
        at = at.wrapping_add(u32::from(length));
    }
    handled(true)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_script.h:
// Cmd_ExecuteScript_Execute.
/// The calling reference's script run now on its event list (`Script::Execute`, the "always
/// run" flag off).
fn execute_script(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if a.this_obj == 0 {
        return handled(true);
    }
    let xscript = extra::refr_extra(e, a.this_obj, EXTRA_SCRIPT);
    if xscript == 0 {
        return handled(true);
    }
    let script = e.mem.u32(xscript + XSCRIPT_SCRIPT);
    let events = e.mem.u32(xscript + XSCRIPT_EVENTS);
    if script != 0 && events != 0 {
        e.call(SCRIPT_RUN, &args![script, a.this_obj, events, 0u32, 0u32]);
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_script_tests.rs"]
mod tests;
