//! JohnnyGuitar NVSE (`JohnnyGuitarNVSE.dll` 5.28), the NVSE plugin VNV installs, built in piece
//! by piece (credits and licence in `NOTICE`). Its DLL patches the
//! old 32-bit program, so it cannot load into nv-rs: its changes are rebuilt here.
//!
//! The first piece is its editor-id restoration (`internal/JG/EditorIDRestoration.cpp`). The game
//! keeps the editor ids of a few form types only. For every other form, `SetEditorID` (vtable
//! +0x134) is a stub (`00401290`) and `GetEditorID` (+0x130) answers "" (`00401280`). So text
//! compiled while the game runs (the console, script runner files, `CompileScript`, lambdas
//! from text) finds `PlayerRef`, `Caps001` or any reference, item or script by name only with
//! JohnnyGuitar installed. With the row on, every loaded form keeps the name its record gives
//! it, as JG keeps it:
//! - JG's setter and getter go in the +0x134 / +0x130 slots of its 142 form classes, where those
//!   slots are still the stubs ([`vtables::FORM_VTABLES`]). JIP's light slots stay JIP's, as
//!   JG skips a slot that is not the stub.
//! - The nine classes that store their name already (cells, world spaces, globals, sounds, voice
//!   types, idles, quests, races, topics) are wrapped: their own setter runs, then JG's on success.
//! - The names live in `engine_core::editor_ids`, outside the 32-bit memory. The lookup by name
//!   (`GetFormByEditorID`, `00483A00`) answers from there, case-blind, under a reader-writer
//!   lock (JG's `SRWLOCK`). The game's own map is not used, so the load's insertion and the hand
//!   registrations JG removes are left out too. A name is copied into the game's memory only
//!   when game code asks for it as text.
//! - The getter's direct call sites, the console's selected-reference name, the two
//!   constructors that name forms directly and `SetTemporary` follow JG's.
//! - A name another form had is a conflict: logged (and counted) unless it is form 0x18E or
//!   `Wilderness`, and one console line at the deferred start, as JG prints it.
//!
//! It is a fixes row ("JohnnyGuitar's editor ids", `NV_JG_EDITOR_IDS`, on in the fixes preset,
//! off in vanilla). Off, nothing here is installed and every site runs the exe's path.
//! JohnnyGuitar is not listed as installed (its 408 commands are not built): a mod that checks
//! for it is told it is absent.

pub mod vtables;

// The unit tests' own program: no other crate's functions are linked in.
#[cfg(test)]
engine_core::assemble!();

#[cfg(test)]
mod tests;

use engine_core::editor_ids;
use engine_core::extend::{Owner, SiteSlot, WrapSlot, ROUTINE_BASE};
use engine_core::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// `PluginInfo::name`, its version and opcode base (`JohnnyPluginData`), and the DLL's file name.
pub const NAME: &str = "JohnnyGuitarNVSE";
pub const VERSION: u32 = 528;
pub const OPCODE_BASE: u32 = 0x3100;
pub const FILE: &str = "johnnyguitarnvse.dll";

/// The stubs JG replaces (`GetEditorID` answering the empty text, `SetEditorID` answering true).
pub const GETTER_STUB: u32 = 0x0040_1280;
pub const SETTER_STUB: u32 = 0x0040_1290;
/// The slots of a form's vtable: `GetEditorID`, `SetEditorID`.
pub const GETTER_SLOT: u32 = 0x130;
pub const SETTER_SLOT: u32 = 0x134;

/// JG's routines: the getter, the setter, the nine wrapped setters.
pub const ROUTINES: u32 = ROUTINE_BASE + 0xe000;
pub const GETTER: u32 = ROUTINES;
pub const SETTER: u32 = ROUTINES + 4;
pub const WRAPPED_SETTERS: u32 = ROUTINES + 0x10;

/// The nine classes that store their name (`CustomSetReplacer`): the setter slot of each.
pub const STORING_SETTER_SLOTS: [u32; 9] = [
    0x0102_eae8, // TESObjectCELL
    0x0103_1a90, // TESWorldSpace
    0x0103_6658, // TESGlobal
    0x0104_5130, // TESSound
    0x0104_7470, // BGSVoiceType
    0x0104_9e40, // TESIdleForm
    0x0104_ad78, // TESQuest
    0x0104_b5f0, // TESRace
    0x0104_d2d0, // TESTopic
];

/// The getter's direct call sites (`ReplaceCall(.., hk_GetFormEditorID)`).
pub const GETTER_SITES: [SiteSlot; 9] = [
    SiteSlot::new(Owner::JohnnyGuitar, 0x0045_1cba),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0048_6903),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0055_8796),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0055_d498),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0055_d683),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0055_df83),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0055_e076),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0055_e0cc),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0061_da92),
];
/// The console's selected-reference name (`GetNameForConsole`).
pub const CONSOLE_NAME_SITES: [SiteSlot; 2] = [
    SiteSlot::new(Owner::JohnnyGuitar, 0x0071_0bfc),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0071_b748),
];
/// The constructors' direct calls of the setter stub (`ActorValueInfo`, `BGSDefaultObjectManager`).
pub const SETTER_SITES: [SiteSlot; 2] = [
    SiteSlot::new(Owner::JohnnyGuitar, 0x0058_d7bc),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0066_ff57),
];
/// The insertions into the game's map JG takes out: the load's own and the eight by hand.
pub const SKIPPED_SITES: [SiteSlot; 9] = [
    SiteSlot::new(Owner::JohnnyGuitar, 0x0046_79f8),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0046_ca2b),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0046_cb0d),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0046_cbef),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0046_ccd1),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0046_cdaf),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0046_ce91),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0046_dc77),
    SiteSlot::new(Owner::JohnnyGuitar, 0x0061_a243),
];
/// `SetTemporary`'s call of `RemoveFromDataStructures` (the seam right after it).
pub const TEMPORARY_SITE: SiteSlot = SiteSlot::new(Owner::JohnnyGuitar, 0x0048_449a);
/// `GetFormEditorIDLength` and `GetFormByEditorID`, replaced.
pub const NAME_LENGTH: WrapSlot = WrapSlot::new(Owner::JohnnyGuitar, 0x0047_4cb0);
pub const FORM_BY_NAME: WrapSlot = WrapSlot::new(Owner::JohnnyGuitar, 0x0048_3a00);

/// The reference's full name (`TESObjectREFR::GetFullName`, `0055D520`, JIP's override included).
const REFR_FULL_NAME: u32 = 0x0055_d520;
/// The game's random number from 0 to 1 (`005C5420`), which JG's console line reads.
const RANDOM_01: u32 = 0x005c_5420;

/// JohnnyGuitar on this engine.
#[derive(Default)]
pub struct JohnnyGuitar {
    /// The host's row switched the editor ids on ([`set_on`]).
    pub on: bool,
    /// [`install`] ran with the row on.
    pub installed: bool,
}

/// The nine storing classes' own setters, as their slots held them: fixed at the install (before
/// the threads start) and read lock-free by every thread's engine.
#[derive(Default)]
struct StoringSetters([AtomicU32; 9]);

thread_local! {
    /// [`form_by_name`]'s name, read from the game's memory: one buffer per thread, kept.
    static NAME_BUFFER: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The host's row: JohnnyGuitar's editor ids for this run (before [`install`]).
pub fn set_on(e: &mut Engine, on: bool) {
    e.ext::<JohnnyGuitar>().on = on;
    // Before JIP's install, which leaves its H04 out while the store is on, as JIP does with
    // JohnnyGuitar loaded.
    editor_ids::store(e).set_on(on);
}

/// Whether the editor ids are installed.
pub fn on(e: &Engine) -> bool {
    e.ext_get::<JohnnyGuitar>().is_some_and(|j| j.installed)
}

/// xNVSE's record of JohnnyGuitar: not listed (its commands are not built), its listener for the
/// deferred start.
fn plugin() -> nvse::plugins::BuiltIn {
    nvse::plugins::BuiltIn {
        file: FILE,
        name: NAME,
        version: VERSION,
        opcode_base: OPCODE_BASE,
        listed: false,
        listener: Some(listener),
        save: None,
        load: None,
        pre_load: None,
        new_game: None,
    }
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// InitHooks.
/// With the row on: the store answers; JG's getter and setter in each listed class's slots that
/// are still the stubs; the nine storing classes' setters wrapped; the seams and the two wraps;
/// JG's record in xNVSE's list (not listed). Off: nothing (the exe's path everywhere). After JIP
/// (its light slots stay its own, as JG finds them not stubs), before lStewieAl's Tweaks.
pub fn install(e: &mut Engine) {
    // Made before the threads start, row on or off, so every thread's engine shares them (xNVSE's
    // removal and JIP's two readers ask the store whether it is on from any thread).
    editor_ids::store(e);
    e.ext_free::<editor_ids::ConflictLog>();
    e.ext_free::<StoringSetters>();
    if !e.ext::<JohnnyGuitar>().on {
        return;
    }
    editor_ids::store(e).set_on(true);
    e.register(GETTER, getter);
    e.register(SETTER, setter);
    for &(vtable, _) in vtables::FORM_VTABLES.iter() {
        // nv-rs's own: an engine without the exe's data (a test's) has no vtable there.
        if !e.mem.is_mapped(vtable + SETTER_SLOT) {
            continue;
        }
        if e.mem.u32(vtable + GETTER_SLOT) == GETTER_STUB {
            e.mem.set_u32(vtable + GETTER_SLOT, GETTER);
        }
        if e.mem.u32(vtable + SETTER_SLOT) == SETTER_STUB {
            e.mem.set_u32(vtable + SETTER_SLOT, SETTER);
        }
    }
    for (i, &slot) in STORING_SETTER_SLOTS.iter().enumerate() {
        if !e.mem.is_mapped(slot) {
            continue;
        }
        let own = e.mem.u32(slot);
        e.ext_free::<StoringSetters>().0[i].store(own, Ordering::Release);
        let routine = WRAPPED_SETTERS + 4 * i as u32;
        e.register(routine, WRAPPED[i]);
        e.mem.set_u32(slot, routine);
    }
    for slot in GETTER_SITES {
        e.on_site(slot, getter_site);
    }
    for slot in CONSOLE_NAME_SITES {
        e.on_site(slot, console_name_site);
    }
    for slot in SETTER_SITES {
        e.on_site(slot, setter_site);
    }
    for slot in SKIPPED_SITES {
        e.on_site(slot, skipped_site);
    }
    e.on_site(TEMPORARY_SITE, temporary_site);
    e.wrap(NAME_LENGTH, name_length);
    e.wrap(FORM_BY_NAME, form_by_name);
    e.ext::<JohnnyGuitar>().installed = true;
    nvse::plugins::register(e, plugin());
    // The game's state, not one engine's: real threads' engines share it.
    e.share_ext::<JohnnyGuitar>();
}

// ---- the routines ----

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// TESFormEx::hk_GetFormEditorID.
/// The getter slot (`thiscall`, the form): its editor id as guest text, "" for none.
fn getter(e: &mut Engine, w: &[u32]) -> Ret {
    editor_ids::guest_text(e, w[0]).into_ret()
}

/// The setter slot (`thiscall`, the form and the text; `RET 4`): [`editor_ids::set_form_editor_id`].
fn setter(e: &mut Engine, w: &[u32]) -> Ret {
    u32::from(editor_ids::set_form_editor_id(e, w[0], w[1])).into_ret()
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// CustomSetReplacer::SetFormEditorID.
/// A storing class's setter wrapped: its own setter (as its slot held it) first, then, when that
/// answers true, JG's; answers the class's own answer.
fn wrapped_setter(e: &mut Engine, index: usize, w: &[u32]) -> Ret {
    let own = e.ext_free::<StoringSetters>().0[index].load(Ordering::Acquire);
    let ok = e.call(own, &args![w[0], w[1]]).u32() & 0xff != 0;
    if ok {
        editor_ids::set_form_editor_id(e, w[0], w[1]);
    }
    u32::from(ok).into_ret()
}

/// One routine per storing class (a routine is a plain function).
const WRAPPED: [AbiFn; 9] = [
    |e, w| wrapped_setter(e, 0, w),
    |e, w| wrapped_setter(e, 1, w),
    |e, w| wrapped_setter(e, 2, w),
    |e, w| wrapped_setter(e, 3, w),
    |e, w| wrapped_setter(e, 4, w),
    |e, w| wrapped_setter(e, 5, w),
    |e, w| wrapped_setter(e, 6, w),
    |e, w| wrapped_setter(e, 7, w),
    |e, w| wrapped_setter(e, 8, w),
];

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// TESFormEx::hk_GetFormEditorIDLength.
/// `GetFormEditorIDLength` (`thiscall`, the form): its editor id's length, 0 for none.
fn name_length(e: &mut Engine, w: &[u32]) -> Ret {
    editor_ids::store(e).name_len(w[0]).into_ret()
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// TESFormEx::hk_GetFormByEditorID.
/// `GetFormByEditorID` (`cdecl`, the name): the form the store names, 0 for no name, an empty one
/// or none (case-blind).
fn form_by_name(e: &mut Engine, w: &[u32]) -> Ret {
    let text = w[0];
    if text == 0 || e.mem.u8(text) == 0 {
        return 0u32.into_ret();
    }
    // Every frame on main (scripts asking a form by name): the name read into this thread's
    // buffer, no allocation.
    NAME_BUFFER
        .with_borrow_mut(|name| {
            name.clear();
            let mut at = text;
            loop {
                let b = e.mem.u8(at);
                if b == 0 {
                    break;
                }
                name.push(b);
                at = at.wrapping_add(1);
            }
            editor_ids::store(e).lookup(name)
        })
        .into_ret()
}

// ---- the seams ----

/// A getter call site (words: the form): JG's getter's answer.
fn getter_site(e: &mut Engine, w: &[u32]) -> Option<u32> {
    Some(editor_ids::guest_text(e, w[0]))
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// TESObjectREFREx::GetNameForConsole.
/// The console's selected reference (words: the reference): its full name, or its editor id
/// (its +0x130) when that is none or empty.
fn console_name_site(e: &mut Engine, w: &[u32]) -> Option<u32> {
    let name = e.call(REFR_FULL_NAME, &args![w[0]]).u32();
    if name != 0 && e.mem.u8(name) != 0 {
        return Some(name);
    }
    Some(e.vcall(w[0], GETTER_SLOT, &args![]).u32())
}

/// A constructor's direct call of the setter stub (words: the form, the text): JG's setter.
fn setter_site(e: &mut Engine, w: &[u32]) -> Option<u32> {
    Some(u32::from(editor_ids::set_form_editor_id(e, w[0], w[1])))
}

/// An insertion into the game's map JG takes out (no words): skipped.
fn skipped_site(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(0)
}

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// TESFormEx::hk_DetachEditorIDs.
/// After `RemoveFromDataStructures` in `SetTemporary` (words: the form): the form's names leave
/// the map (they stay its own). The exe goes on.
fn temporary_site(e: &mut Engine, w: &[u32]) -> Option<u32> {
    editor_ids::detach_form(e, w[0]);
    None
}

// ---- the deferred start ----

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// PrintErrors.
/// At the deferred start: the store's counts in the start-up log (what the load named and what it
/// takes in memory), and, after conflicts, JG's console line ("mod", or "slop" one time in a
/// thousand, by the game's random number).
fn listener(e: &mut Engine, m: &nvse::plugins::Message) {
    if m.kind != nvse::plugins::msg::DEFERRED_INIT {
        return;
    }
    eprintln!("{}", load_line(e));
    if editor_ids::store(e).counts().conflicts == 0 {
        return;
    }
    write_log(e);
    let random = e.call(RANDOM_01, &args![]).f32();
    let who = if random < 0.001 { "slop" } else { "mod" };
    let who = nvse::table::guest_str(e, who.as_bytes());
    nvse::commands::console_print(
        e,
        "Some EDIDs are conflicting! Check JohnnyGuitarNVSE.log for details. (Ignore this message if you are not a %s author.)",
        &[who],
    );
}

/// JohnnyGuitar's log, which its console line names.
pub const LOG_FILE: &[u8] = b"JohnnyGuitarNVSE.log";

// Translated from JohnnyGuitar NVSE 5.28 (c87905e), internal/JG/EditorIDRestoration.cpp:
// LogEDIDConflict's lines to JohnnyGuitarNVSE.log.
/// The conflict lines to [`LOG_FILE`] in one write at the deferred start (batched: a large list
/// has thousands), through the game's file calls as `nvse.log` is, so the file lands with the
/// port's other mod files beside the saves, never in the game folder; one start-up log line says
/// how many.
fn write_log(e: &mut Engine) {
    let mut text = Vec::new();
    for line in editor_ids::conflict_lines(e) {
        text.extend_from_slice(line.as_bytes());
        text.push(b'\n');
    }
    let lines = text.iter().filter(|&&b| b == b'\n').count();
    if nvse::log::write_file(e, LOG_FILE, &text) {
        eprintln!("JohnnyGuitar's editor ids: {lines} conflicts written to JohnnyGuitarNVSE.log");
    }
}

/// The load's log line: forms named, names, map entries, the store's bytes, conflicts.
pub fn load_line(e: &mut Engine) -> String {
    let c = editor_ids::store(e).counts();
    format!(
        "JohnnyGuitar's editor ids: {} forms named ({} names), {} in the map, {} bytes of names, {} bytes in all, {} conflicts",
        c.forms, c.named, c.entries, c.arena_bytes, c.heap_bytes, c.conflicts
    )
}
