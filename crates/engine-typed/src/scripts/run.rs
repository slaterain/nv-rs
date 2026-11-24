//! `Script::Run` and the run manager that every script run goes through: the quest script's delay
//! and the run itself (`Script::Run`, `005ac1e0`), the magic effect's three runs (`005ac340`,
//! `005ac380`, `005ac3c0`), the manager's instance and run (`005e24d0`, `005e2590`), a fresh runner
//! (`005e0c60`) and its busy test (`005e2850`), the action list's build through the manager
//! (`005e2730`), the forgetting of a run's referenced objects (`005ae2e0`), and the locals' two
//! writes a run makes: a variable's set (`ScriptLocals::SetVariable`, `005a9290`) and the clear of the
//! action list's flags (`005a8ea0`).
//!
//! What stays by address: the runner's line loop and the action list's run (`005e0d20`,
//! `005e1160`: xNVSE wraps both, so the wrap runs and its own call reaches the typed base), the run
//! count's interlocked add and subtract, the allocator and its free, the header and the elapsed
//! time, the quest's delay, the log and the save-game names of `SetVariable`'s reports.
//! `Script::Run` itself is in `overridable.tsv` (JIP LN and xNVSE call it): registered here at its
//! address, never called direct.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet;
use engine_core::{args, typed_call, Engine};

/// `Script`'s quest delay, the get-seconds buffer and the owner quest.
const QUEST_DELAY: u32 = 0x38;
const GET_SECONDS_BUFFER: u32 = 0x3c;
const OWNER_QUEST: u32 = 0x40;
/// `ScriptLocals`: the master script (+0), the action list (+8), the variables (+0xc).
const LOCALS_MASTER: u32 = 0;
const LOCALS_ACTIONS: u32 = 8;
/// `ScriptRunManager`: the cached runner (+0) and the current one (+4); its storage and the
/// constructed flag; the run count.
const MANAGER: u32 = 0x011c_aed0;
const MANAGER_CONSTRUCTED: u32 = 0x011c_aed8;
const RUN_DEPTH: u32 = 0x011c_aedc;
const MANAGER_CONSTRUCT: u32 = 0x0096_a2d0;
const AT_EXIT: u32 = 0x00ec_658f;
const MANAGER_DESTRUCT: u32 = 0x00fc_b1f0;
const RUNNER_SIZE: u32 = 0xa4;
/// The two wrapped runner functions (xNVSE), by address.
const RUN_LINES: u32 = 0x005e_0d20;
const BUILD_ACTION_LIST: u32 = 0x005e_1160;
const ALLOCATE: u32 = 0x0040_1000;
const FREE: u32 = 0x0040_1030;
const INTERLOCKED_INCREMENT: u32 = 0x0040_b460;
const INTERLOCKED_DECREMENT: u32 = 0x0040_19a0;
const FILL: u32 = 0x0040_3d30;
const SCRIPT_HEADER_OF: u32 = 0x0050_0940;
const ELAPSED_TIME: u32 = 0x0084_d030;
const ELAPSED_TIME_OBJECT: u32 = 0x011f_6394;
const QUEST_DELAY_ACCESSOR: u32 = 0x0040_3e20;
const QUEST_DELAY_SETTING: u32 = 0x011c_ac68;
const QUEST_DELAY_OF: u32 = 0x0079_2760;
const SCRIPT_REF_OBJECT_LIST: u32 = 0x005d_43c0;
const TLS_LAST_REF_SCRIPT: u32 = 0x288;
/// `SetVariable`'s reports: the log, the save game, its loading test and the form being loaded,
/// the form lookup, the cast and the names.
const SCRIPT_LOG: u32 = 0x005b_5e40;
const SAVE_LOAD_GAME: u32 = 0x011d_e45c;
const IS_LOADING_STUB: u32 = 0x0047_c850;
const LOADING_FORM_HEADER: u32 = 0x004f_d3c0;
const LOOKUP_FORM_BY_ID: u32 = 0x0048_39c0;
const DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_FORM: u32 = 0x0118_3028;
const RTTI_REFERENCE: u32 = 0x0118_41cc;
const FULL_NAME_OF_FORM: u32 = 0x0048_2720;
const FULL_NAME_OF_REFERENCE: u32 = 0x0055_d520;
const FORMAT_SET_VARIABLE_NO_SCRIPT: u32 = 0x0103_6e2c;
const FORMAT_SET_VARIABLE_FORM: u32 = 0x0103_6e68;
const FORMAT_SET_VARIABLE_NAMED: u32 = 0x0103_6f00;
const FORMAT_SET_VARIABLE_REF: u32 = 0x0103_6fa0;
const FORMAT_SET_VARIABLE_IN_SCRIPT: u32 = 0x0103_7040;

#[inline]
fn list_item(e: &mut Engine, node: u32) -> u32 {
    let slot = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, node));
    e.mem.u32(slot)
}

#[inline]
fn list_next(e: &mut Engine, node: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, node))
}

#[inline]
fn list_is_empty(e: &mut Engine, node: u32) -> bool {
    typed_call!(leaf::FAMILY, 0x0082_56d0, leaf::both_words_zero(e, node))
}

#[inline]
fn header_of(e: &mut Engine, script: Ptr) -> u32 {
    e.call(SCRIPT_HEADER_OF, &args![script]).u32()
}

/// The FLD/FSTP double store of `SetVariable`: quiets a signalling NaN, keeping its sign and
/// payload (masked x87 exceptions).
#[inline]
fn quiet_double(value: f64) -> f64 {
    let bits = value.to_bits();
    let nan =
        bits & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000 && bits & 0x000f_ffff_ffff_ffff != 0;
    f64::from_bits(if nan {
        bits | 0x0008_0000_0000_0000
    } else {
        bits
    })
}

/// A double stored as a float (`FSTP dword`).
#[allow(clippy::cast_possible_truncation)]
#[inline]
fn to_float(value: f64) -> f32 {
    value as f32
}

/// A fresh runner: `ScriptRunner`'s constructor on a new 0xa4-byte block (none when the allocator
/// answers null).
#[inline]
fn new_runner(e: &mut Engine) -> u32 {
    let block = e.call(ALLOCATE, &args![RUNNER_SIZE]).u32();
    if block == 0 {
        0
    } else {
        typed_call!(FAMILY, 0x005e_0c60, construct_runner(e, Ptr::new(block))).addr()
    }
}

// Typed from 005ac1e0
/// `Script::Run`: runs the script through the run manager and answers its result. Without
/// `force` a script whose compiled data is 4 bytes or fewer does not run. A quest script with a
/// running delay first loses the elapsed time from it (and adds it to the get-seconds buffer), and
/// does not run while the delay is still above 0. After a run a quest script's delay is set again
/// (the owner quest's when above 0, else the setting's, added to what is left) and its buffer
/// zeroed; then the run's referenced objects are forgotten. The quest's floats: a double difference
/// or sum stored as a float, the same on every thread.
pub fn script_run(
    e: &mut Engine,
    this: Ptr,
    first: u32,
    second: u32,
    third: u32,
    force: u8,
) -> bool {
    let me = this.addr();
    if force == 0 {
        let header = header_of(e, this);
        if e.mem.u32(header + 8) <= 4 {
            return false;
        }
    }
    let header = header_of(e, this);
    if e.mem.u8(header + 0x10) != 0 && e.mem.f32(me + QUEST_DELAY) > 0.0 {
        let elapsed = e.call(ELAPSED_TIME, &args![ELAPSED_TIME_OBJECT]).f64();
        let delay = f64::from(e.mem.f32(me + QUEST_DELAY));
        e.mem.set_f32(me + QUEST_DELAY, to_float(delay - elapsed));
        let elapsed = e.call(ELAPSED_TIME, &args![ELAPSED_TIME_OBJECT]).f64();
        let buffer = f64::from(e.mem.f32(me + GET_SECONDS_BUFFER));
        e.mem
            .set_f32(me + GET_SECONDS_BUFFER, to_float(elapsed + buffer));
        if e.mem.f32(me + QUEST_DELAY) > 0.0 {
            return false;
        }
    }
    let manager = typed_call!(FAMILY, 0x005e_24d0, manager_instance(e));
    let result = typed_call!(
        FAMILY,
        0x005e_2590,
        manager_run(
            e,
            manager,
            this,
            Ptr::new(first),
            Ptr::new(second),
            Ptr::new(third),
            force,
            0,
            0,
            0.0
        )
    );
    let header = header_of(e, this);
    if e.mem.u8(header + 0x10) != 0 {
        let setting = e
            .call(QUEST_DELAY_ACCESSOR, &args![QUEST_DELAY_SETTING])
            .u32();
        if e.mem.f32(setting) > 0.0 {
            let owner = e.mem.u32(me + OWNER_QUEST);
            if owner != 0 && e.call(QUEST_DELAY_OF, &args![owner]).f64() > 0.0 {
                let owner = e.mem.u32(me + OWNER_QUEST);
                let quest_delay = e.call(QUEST_DELAY_OF, &args![owner]).f64();
                let delay = f64::from(e.mem.f32(me + QUEST_DELAY));
                e.mem
                    .set_f32(me + QUEST_DELAY, to_float(quest_delay + delay));
            } else {
                let setting = e
                    .call(QUEST_DELAY_ACCESSOR, &args![QUEST_DELAY_SETTING])
                    .u32();
                let delay = f64::from(e.mem.f32(me + QUEST_DELAY));
                let value = f64::from(e.mem.f32(setting));
                e.mem.set_f32(me + QUEST_DELAY, to_float(delay + value));
            }
            e.mem.set_f32(me + GET_SECONDS_BUFFER, 0.0);
        }
    }
    typed_call!(FAMILY, 0x005a_e2e0, forget_referenced_objects(e, this));
    result
}

/// The magic effect's runs: the manager's run with the effect's start, finish or elapsed time.
#[inline]
fn effect_run(e: &mut Engine, this: Ptr, first: u32, second: u32, flags: (u8, u8), delta: f32) {
    let manager = typed_call!(FAMILY, 0x005e_24d0, manager_instance(e));
    typed_call!(
        FAMILY,
        0x005e_2590,
        manager_run(
            e,
            manager,
            this,
            Ptr::new(first),
            Ptr::new(second),
            Ptr::NULL,
            0,
            flags.0,
            flags.1,
            delta
        )
    );
}

// Typed from 005ac340
/// `Script::RunScriptEffectStart`: the manager's run with the effect-start flag.
pub fn run_effect_start(e: &mut Engine, this: Ptr, first: u32, second: u32) {
    effect_run(e, this, first, second, (1, 0), 0.0);
}

// Typed from 005ac380
/// `Script::RunScriptEffectFinish`: the manager's run with the effect-finish flag.
pub fn run_effect_finish(e: &mut Engine, this: Ptr, first: u32, second: u32) {
    effect_run(e, this, first, second, (0, 1), 0.0);
}

// Typed from 005ac3c0
/// `Script::RunScriptEffectUpdate`: the manager's run with the elapsed time `delta`.
pub fn run_effect_update(e: &mut Engine, this: Ptr, first: u32, second: u32, delta: f32) {
    effect_run(e, this, first, second, (0, 0), delta);
}

// Typed from 005e24d0
/// `ScriptRunManager::Instance`: the manager, constructed on first use (its flag set first, its
/// destructor queued at exit).
pub fn manager_instance(e: &mut Engine) -> Ptr {
    let flags = e.mem.u32(MANAGER_CONSTRUCTED);
    if flags & 1 == 0 {
        e.mem.set_u32(MANAGER_CONSTRUCTED, flags | 1);
        e.call(MANAGER_CONSTRUCT, &args![MANAGER]);
        e.call(AT_EXIT, &args![MANAGER_DESTRUCT]);
    }
    Ptr::new(MANAGER)
}

// Typed from 005e2590
/// `ScriptRunManager::Run`: the outermost run (the run count 1 after its add) uses the cached
/// runner, made on first use, current for the call; a nested run makes a temporary runner, current
/// for the call, then restores the runner that was current and frees the temporary. Both pass the
/// eight words on to the runner's line loop (by address: xNVSE wraps it); the float is quieted as
/// its `FLD`/`FSTP` does.
#[allow(clippy::too_many_arguments)]
pub fn manager_run(
    e: &mut Engine,
    this: Ptr,
    script: Ptr,
    reference: Ptr,
    locals: Ptr,
    container: Ptr,
    keep_actions: u8,
    effect_start: u8,
    effect_finish: u8,
    elapsed: f32,
) -> bool {
    let me = this.addr();
    let quiet = x87_quiet(elapsed);
    let words = |runner: u32| {
        args![
            runner,
            script,
            reference,
            locals,
            container,
            u32::from(keep_actions),
            u32::from(effect_start),
            u32::from(effect_finish),
            quiet
        ]
    };
    let result = if e.call(INTERLOCKED_INCREMENT, &args![RUN_DEPTH]).u32() == 1 {
        if e.mem.u32(me) == 0 {
            let runner = new_runner(e);
            e.mem.set_u32(me, runner);
        }
        let cached = e.mem.u32(me);
        e.mem.set_u32(me + 4, cached);
        let runner = e.mem.u32(me);
        let result = e.call(RUN_LINES, &words(runner)).bool();
        e.mem.set_u32(me + 4, 0);
        result
    } else {
        let temporary = new_runner(e);
        let previous = e.mem.u32(me + 4);
        e.mem.set_u32(me + 4, temporary);
        let result = e.call(RUN_LINES, &words(temporary)).bool();
        e.mem.set_u32(me + 4, previous);
        e.call(FREE, &args![temporary]);
        result
    };
    e.call(INTERLOCKED_DECREMENT, &args![RUN_DEPTH]);
    result
}

// Typed from 005e0c60
/// `ScriptRunner::ScriptRunner`: zeroes the runner's words and its three 0x28-byte tables, and its
/// two bytes (the padding stays).
pub fn construct_runner(e: &mut Engine, this: Ptr) -> Ptr {
    let me = this.addr();
    for offset in [0x20, 0x4c, 0x18, 0x1c] {
        e.mem.set_u32(me + offset, 0);
    }
    for offset in [0x24, 0x50, 0x78] {
        e.call(FILL, &args![me + offset, 0u32, 0x28u32]);
    }
    for offset in [0, 4, 8, 0xc, 0x10, 0x14] {
        e.mem.set_u32(me + offset, 0);
    }
    e.mem.set_u8(me + 0xa0, 0);
    e.mem.set_u8(me + 0xa1, 0);
    this
}

// Typed from 005e2850
/// Whether a script is running in the runner.
pub fn runner_is_busy(e: &mut Engine, this: Ptr) -> bool {
    e.mem.u32(this.addr() + 0x14) != 0
}

// Typed from 005e2730
/// `ScriptRunManager::InitActionList`: builds the action list (by address: xNVSE wraps it) in the
/// cached runner (made on first use), or in a temporary one while the cached one is busy.
///
/// lStewieAl's Tweaks patches the two calls of the action list's run here (`005e280a` and
/// `005e2834`); that tweak is not built in, so no seam stands there yet: when it is, both calls go
/// through one.
pub fn manager_init_action_list(
    e: &mut Engine,
    this: Ptr,
    script: Ptr,
    reference: Ptr,
    locals: Ptr,
) -> bool {
    let me = this.addr();
    if e.mem.u32(me) == 0 {
        let runner = new_runner(e);
        e.mem.set_u32(me, runner);
    }
    let runner = Ptr::new(e.mem.u32(me));
    if typed_call!(FAMILY, 0x005e_2850, runner_is_busy(e, runner)) {
        let temporary = new_runner(e);
        e.call(
            BUILD_ACTION_LIST,
            &args![temporary, script, reference, locals],
        );
        e.call(FREE, &args![temporary]);
    } else {
        let runner = e.mem.u32(me);
        e.call(BUILD_ACTION_LIST, &args![runner, script, reference, locals]);
    }
    true
}

// Typed from 005ae2e0
/// Forgets this script as the last referenced-object search's script (TLS) and clears the form of
/// each of its referenced objects that has a variable id, so the next lookup reloads it.
pub fn forget_referenced_objects(e: &mut Engine, this: Ptr) {
    let tls = e.tls();
    if e.mem.u32(tls + TLS_LAST_REF_SCRIPT) == this.addr() {
        e.mem.set_u32(tls + TLS_LAST_REF_SCRIPT, 0);
    }
    let mut node = e.call(SCRIPT_REF_OBJECT_LIST, &args![this]).u32();
    while node != 0 && !list_is_empty(e, node) {
        let object = list_item(e, node);
        if object != 0 && e.mem.u32(object + 0xc) != 0 {
            e.mem.set_u32(object + 8, 0);
        }
        node = list_next(e, node);
    }
}

// Typed from 005a8ea0
/// Clears the flags of every entry of the locals' action list.
pub fn clear_action_flags(e: &mut Engine, this: Ptr) {
    let mut node = e.mem.u32(this.addr() + LOCALS_ACTIONS);
    while node != 0 {
        let item = list_item(e, node);
        if item == 0 {
            break;
        }
        node = list_next(e, node);
        e.mem.set_u32(item + 4, 0);
    }
}

// Typed from 005a9290
/// `ScriptLocals::SetVariable`: stores `value` in the variable `id`. A missing variable is
/// logged: with the master script's name when there is one; otherwise, while a save loads, with
/// the file and the form being loaded; otherwise with the id alone.
pub fn set_variable(e: &mut Engine, this: Ptr, id: u32, value: f64) {
    let mut node = typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, this));
    while node != 0 {
        if list_item(e, node) == 0 {
            break;
        }
        let variable = list_item(e, node);
        node = list_next(e, node);
        if e.mem.u32(variable) == id {
            e.mem.set_f64(variable + 8, quiet_double(value));
            return;
        }
    }
    let master = e.mem.u32(this.addr() + LOCALS_MASTER);
    if master != 0 {
        let name = e.vcall(master, 0x130, &args![]).u32();
        e.call(SCRIPT_LOG, &args![FORMAT_SET_VARIABLE_IN_SCRIPT, id, name]);
        return;
    }
    let save_load = e.mem.u32(SAVE_LOAD_GAME);
    if e.call(IS_LOADING_STUB, &args![save_load]).bool() {
        let header = e.call(LOADING_FORM_HEADER, &args![save_load]).u32();
        if header != 0 {
            let header_id = e.mem.u32(header);
            let form = e.call(LOOKUP_FORM_BY_ID, &args![header_id]).u32();
            let reference = e
                .call(
                    DYNAMIC_CAST,
                    &args![form, 0u32, RTTI_FORM, RTTI_REFERENCE, 0u32],
                )
                .u32();
            let name = if reference != 0 {
                e.call(FULL_NAME_OF_REFERENCE, &args![reference]).u32()
            } else {
                e.call(FULL_NAME_OF_FORM, &args![form]).u32()
            };
            let file_name = e.mem.u32(save_load + 0x1c4);
            if form != 0 && name != 0 {
                let format = if reference != 0 {
                    FORMAT_SET_VARIABLE_REF
                } else {
                    FORMAT_SET_VARIABLE_NAMED
                };
                e.call(SCRIPT_LOG, &args![format, file_name, id, name, header_id]);
            } else {
                e.call(
                    SCRIPT_LOG,
                    &args![FORMAT_SET_VARIABLE_FORM, file_name, id, header_id],
                );
            }
            return;
        }
    }
    e.call(SCRIPT_LOG, &args![FORMAT_SET_VARIABLE_NO_SCRIPT, id]);
}

// Typed from 005a90b0
/// The locals' action of `form` in the action list. With `create` and no match, the last entry
/// with no form instead. Null when there is no list.
pub fn find_action(e: &mut Engine, this: Ptr, form: u32, create: bool) -> Ptr {
    let mut node = e.mem.u32(this.addr() + LOCALS_ACTIONS);
    if node == 0 {
        return Ptr::NULL;
    }
    let mut empty_entry = 0u32;
    while node != 0 {
        let item = list_item(e, node);
        if item == 0 {
            break;
        }
        let entry_form = e.mem.u32(item);
        if entry_form == form {
            return Ptr::new(item);
        }
        if entry_form == 0 {
            empty_entry = item;
        }
        node = list_next(e, node);
    }
    if create {
        Ptr::new(empty_entry)
    } else {
        Ptr::NULL
    }
}

// Typed from 005a8e20
/// `ScriptLocals::SetActionFlag`: ORs `flags` into the action of `form` and into the entry with no
/// form (the "any form" entry), both found with the create flag. False when there is no action
/// list or neither entry exists.
pub fn set_locals_action_flag(e: &mut Engine, this: Ptr, form: u32, flags: u32) -> bool {
    if e.mem.u32(this.addr() + LOCALS_ACTIONS) == 0 {
        return false;
    }
    let mut found = false;
    for target in [form, 0] {
        let action = typed_call!(FAMILY, 0x005a_90b0, find_action(e, this, target, true));
        if !action.is_null() {
            let old = e.mem.u32(action.addr() + 4);
            e.mem.set_u32(action.addr() + 4, old | flags);
            found = true;
        }
    }
    found
}
