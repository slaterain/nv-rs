//! `TESObjectREFR::InitScript` (`00565730`, Xbox PDB): the reference's script made ready when it
//! comes into the scene. `TES`'s load of a reference into the scene ([`super::scene`]) calls it for
//! every reference it loads; the exe's other callers (`InitItem`, the actor base template code, the
//! player's set-up and the script compiler's `GetVariable`) call it by address and reach it
//! through its registration.
//!
//! For a reference that is not deleted (form flag `0x20`), inside the memory context `0x15` (named
//! by the reference unit's source file name and a line): unless its extra data list already holds a
//! script extra data (type `0xd`) with a script (+0xc), the script of the reference's form (its
//! slot 0x1a4, else its base form) goes into the list with an event list made for it, and a
//! reference with a container has its inventory changes' scripts set up.
//!
//! **Native.** The exe's order of work and every call that does work stay the exe's. Done in place
//! or as typed calls: the deleted flag (`00440d80`), the extra data list (`005d43c0`, the reference
//! plus 0x44, three times in the exe), the extra data lookup (`00410220`, the dispatch family's),
//! the base form (`007af430`), the slot 0x1a4 when it holds one of the exe's two bodies (0 for
//! `TESObjectREFR`'s and the projectiles' and explosions' `00acbb70`; the word at +0x148 for the
//! actor classes' `008ae600`, typed here), the form's script (`004826d0`, typed here: its
//! `TESScriptableForm` by `__RTDynamicCast`, by address, then the word at +4) and the reference's
//! container (`0055d310`, typed here: the base form's component by its type). Called by address:
//! the memory context's entry and exit (they set the context the allocator files blocks under), the
//! script's set into the list (`00419ed0`), the event list made (`005abf60`) and set (`00419f80`),
//! the inventory changes (`004bf220`, which may make them) and their scripts' set-up (`004d1960`).
//!
//! **JIP LN NVSE.** Nothing of JIP's (or of the other built-in extenders) replaces these functions
//! or hooks a site in them; the extenders call `0055d310` and `004826d0` by address, which reaches
//! the typed bodies. The same code runs with JIP on and off.
//!
//! **Which thread.** Main for the cell's load and the scripts. Integers only: no float rule
//! applies.

use super::{base_form, FAMILY, REFERENCE_EXTRA_LIST};
use crate::{dispatch, slot_target};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The memory context's entry (`thiscall(scope, context, 1, source file, line)`, `ret 0x10`) and
/// exit (`thiscall(scope)`), the context `0x15`, named by the reference unit's source file name
/// (`0102fc20`) and the line 0x196e.
const SCOPE_BEGIN: u32 = 0x0040_4eb0;
const SCOPE_END: u32 = 0x0040_4ee0;
const SCOPE_MODE: u32 = 1;
const CONTEXT_SCRIPT: u32 = 0x15;
const REFERENCE_SOURCE: u32 = 0x0102_fc20;
const LINE_INIT_SCRIPT: u32 = 0x196e;
/// The script extra data's type (`ExtraScript`) and its script (+0xc).
const EXTRA_SCRIPT: u8 = 0xd;
const EXTRA_SCRIPT_SCRIPT: u32 = 0xc;
/// The reference's slot 0x1a4 (the form the reference's script comes from) and the exe's bodies in
/// it: 0 (`TESObjectREFR`'s, `MobileObject`'s, the projectiles' and `Explosion`'s) and the actor
/// classes' word at +0x148 (`Actor`, `Character`, `Creature`, `PlayerCharacter`).
const SLOT_SCRIPT_FORM: u32 = 0x1a4;
const ANSWERS_ZERO: u32 = 0x00ac_bb70;
const ACTOR_SCRIPT_FORM: u32 = 0x008a_e600;
const ACTOR_FORM_148: u32 = 0x148;
/// `__RTDynamicCast` (`cdecl(object, 0, source type, target type, 0)`) and the type descriptors of
/// the cast of a form to its `TESScriptableForm`.
const RT_DYNAMIC_CAST: u32 = 0x00ec_43fb;
const RTTI_TES_FORM: u32 = 0x0118_3028;
const RTTI_SCRIPTABLE_FORM: u32 = 0x0118_3254;
/// The extra data list's script set (`ExtraDataList::SetScript`, Xbox PDB, `thiscall(list,
/// script)`), the script's event list made (`thiscall(script)`, the xNVSE port's
/// `CREATE_EVENT_LIST`) and set into the list (`thiscall(list, event list)`).
const EXTRA_LIST_SET_SCRIPT: u32 = 0x0041_9ed0;
const SCRIPT_CREATE_EVENT_LIST: u32 = 0x005a_bf60;
const EXTRA_LIST_SET_EVENT_LIST: u32 = 0x0041_9f80;
/// The reference's inventory changes (`cdecl(reference)`, made when there are none) and their
/// scripts' set-up (`thiscall(changes)`).
const INVENTORY_CHANGES: u32 = 0x004b_f220;
const INVENTORY_CHANGES_INIT_SCRIPTS: u32 = 0x004d_1960;
/// The base form types with a container: `TESObjectCONT` (its component at +0x30) and the actor
/// bases `TESNPC` and `TESCreature` (+0x64).
const FORM_TYPE_CONTAINER: u8 = 0x1b;
const FORM_TYPE_NPC: u8 = 0x2a;
const FORM_TYPE_CREATURE: u8 = 0x2b;
const CONTAINER_OF_CONTAINER: u32 = 0x30;
const CONTAINER_OF_ACTOR_BASE: u32 = 0x64;

/// The reference's slot 0x1a4: the exe's two bodies as their typed calls, any other through the
/// vtable.
#[inline]
fn script_form(e: &mut Engine, reference: u32) -> u32 {
    match slot_target(e, reference, SLOT_SCRIPT_FORM) {
        ANSWERS_ZERO => typed_call!(leaf::FAMILY, 0x00ac_bb70, leaf::zero_of_this(e, reference)),
        ACTOR_SCRIPT_FORM => typed_call!(FAMILY, 0x008a_e600, actor_form_148(e, reference)),
        _ => e.vcall(reference, SLOT_SCRIPT_FORM, &args![]).u32(),
    }
}

// Typed from 00565730
/// `TESObjectREFR::InitScript` (Xbox PDB), `thiscall(reference)`: nothing for a deleted reference
/// (form flag `0x20`). Otherwise, inside the memory context `0x15`: a reference whose extra data
/// list holds a script extra data (type `0xd`) with a script (+0xc) is left as it is; any other has
/// the script of its form (slot 0x1a4, its base form when that gives none, `004826d0`) set into its
/// list (`00419ed0`) when there is one, with the event list `005abf60` makes for it (`00419f80`),
/// and, when it has a container (`0055d310`), its inventory changes' scripts set up (`004bf220`,
/// `004d1960`).
pub fn init_script(e: &mut Engine, this: u32) {
    if typed_call!(leaf::FAMILY, 0x0044_0d80, leaf::form_flag_20(e, this)) {
        return;
    }
    let list = this.wrapping_add(REFERENCE_EXTRA_LIST);
    e.with_stack(4, |e, scope| {
        e.call(
            SCOPE_BEGIN,
            &args![
                scope,
                CONTEXT_SCRIPT,
                SCOPE_MODE,
                REFERENCE_SOURCE,
                LINE_INIT_SCRIPT
            ],
        );
        let extra = typed_call!(
            dispatch::FAMILY,
            0x0041_0220,
            dispatch::extra::get_extra_data(e, Ptr::new(list), EXTRA_SCRIPT)
        )
        .addr();
        if extra == 0 || e.mem.u32(extra.wrapping_add(EXTRA_SCRIPT_SCRIPT)) == 0 {
            set_up(e, this, list);
        }
        e.call(SCOPE_END, &args![scope]);
    });
}

/// The script of the reference's form set into its list with its event list, then the inventory
/// changes' scripts of a reference with a container.
#[inline]
fn set_up(e: &mut Engine, reference: u32, list: u32) {
    let mut form = script_form(e, reference);
    if form == 0 {
        form = base_form(e, reference);
    }
    let script = typed_call!(FAMILY, 0x0048_26d0, form_script(e, form));
    if script != 0 {
        e.call(EXTRA_LIST_SET_SCRIPT, &args![list, script]);
        let events = e.call(SCRIPT_CREATE_EVENT_LIST, &args![script]).u32();
        e.call(EXTRA_LIST_SET_EVENT_LIST, &args![list, events]);
    }
    if typed_call!(FAMILY, 0x0055_d310, has_container(e, reference)) != 0 {
        let changes = e.call(INVENTORY_CHANGES, &args![reference]).u32();
        e.call(INVENTORY_CHANGES_INIT_SCRIPTS, &args![changes]);
    }
}

// Typed from 004826d0
/// The script of a form (`cdecl(form)`): its `TESScriptableForm` (`__RTDynamicCast` from `TESForm`,
/// by address) and that one's script (`00726070`, the word at +4), or null when the form has none.
pub fn form_script(e: &mut Engine, form: u32) -> u32 {
    let scriptable = e
        .call(
            RT_DYNAMIC_CAST,
            &args![form, 0u32, RTTI_TES_FORM, RTTI_SCRIPTABLE_FORM, 0u32],
        )
        .u32();
    if scriptable == 0 {
        return 0;
    }
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, scriptable))
}

// Typed from 0055d310
/// `TESObjectREFR::HasContainer` (Xbox PDB), `thiscall(reference)`: the container component of the
/// reference's base form, or null: a `TESObjectCONT`'s (type 0x1b) at +0x30, a `TESNPC`'s or a
/// `TESCreature`'s (types 0x2a and 0x2b) at +0x64; none without a base form or for any other type.
/// The base form is read where the exe reads it (`007af430`, three times: the test, the type, the
/// component), each read tested again as the exe tests it.
pub fn has_container(e: &mut Engine, this: u32) -> u32 {
    if base_form(e, this) == 0 {
        return 0;
    }
    let base = base_form(e, this);
    let kind = e.mem.u8(base.wrapping_add(4));
    let offset = match kind {
        FORM_TYPE_CONTAINER => CONTAINER_OF_CONTAINER,
        FORM_TYPE_NPC | FORM_TYPE_CREATURE => CONTAINER_OF_ACTOR_BASE,
        _ => return 0,
    };
    match base_form(e, this) {
        0 => 0,
        base => base.wrapping_add(offset),
    }
}

// Typed from 008ae600
/// The actor classes' slot 0x1a4 (`thiscall(actor)`): the word at +0x148, a form that takes the
/// base form's place for the actor's script.
pub fn actor_form_148(e: &mut Engine, this: u32) -> u32 {
    e.mem.u32(this.wrapping_add(ACTOR_FORM_148))
}
