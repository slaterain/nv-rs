//! JIP LN NVSE's `SetLinkedReference` (`functions_jip/jip_fn_misc_ref.h`; in both of JIP's
//! blocks, `0x2281` and `0x2634`, one execute) with JIP's `TESObjectREFR::SetLinkedRef`,
//! `SetLinkedRefID` (`internal/jip_core.cpp`) and `RestoreLinkedRefs`
//! (`internal/serialization.h`), and the `JPLR` co-save record applied at a load.
//!
//! JIP keeps two maps: the links it changed (`s_linkedRefModified`, saved in `JPLR`:
//! [`crate::cosave::Saved::linked_refs`]) and each changed reference's link before its first
//! change in the session (`s_linkedRefDefault`, 0 when it had none:
//! [`crate::cosave::Saved::linked_ref_defaults`]). A load sets each saved link again
//! (`SetLinkedRefID`) and notes it in `s_linkedRefsTemp`; once the game has loaded
//! (`DoLoadGameHousekeeping`), when the linked references changed since the save last loaded or
//! saved, every reference with a default the load did not set again gets its default back; a
//! return to the main menu gives every one its default back.
//!
//! The game's parts: `BaseExtraList::GetByType`, `AddExtra` and `RemoveByType` (`extra`), and
//! `ExtraLinkedRef` as JIP's `Create` makes it: 0x10 bytes of the game's heap with the vtable
//! `0x01015CC0` and type 0x51 (the exe's constructor `00433640` stores the same, linked
//! reference +0xC).

use super::{extra, Handlers};
use crate::cosave::{changed, lookup_form, Saved};
use crate::Jip;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

#[rustfmt::skip]
pub static COMMANDS: Handlers = &[
    (0x2281, set_linked_reference),
    (0x2634, set_linked_reference),
];

/// `ExtraLinkedRef`: type 0x51, vtable `0x01015CC0` (constructor `00433640`), its reference +0xC.
const EXTRA_LINKED_REF: u32 = 0x51;
const VTBL_LINKED_REF: u32 = 0x0101_5cc0;
const LINKED_REF: u32 = 0x0c;
/// `TESForm::refID` (+0xC); a vtable's `IsReference` (+0xF0) answering true (`008D0360`).
const REF_ID: u32 = 0x0c;
const VT_IS_REFERENCE: u32 = 0xf0;
const RETURN_TRUE: u32 = 0x008d_0360;

fn saved(e: &mut Engine) -> &mut Saved {
    &mut e.ext::<Jip>().saved
}

fn is_reference(e: &Engine, form: u32) -> bool {
    form != 0 && {
        let vtbl = e.mem.u32(form);
        vtbl != 0 && e.mem.u32(vtbl + VT_IS_REFERENCE) == RETURN_TRUE
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: TESObjectREFR::SetLinkedRef.
/// `link` 0: the reference's default link back (its extra removed when it had none, else the
/// default's reference set when it is still a reference), the default and the change forgotten;
/// true. Otherwise: without the extra, one made with `link` (default 0); with it, false when it
/// holds none, else its link noted as the default and replaced; the change noted with `mod_index`;
/// true. (Each change notes the link it replaced as the default, as JIP's map assignment does.)
pub(crate) fn set_linked_ref(e: &mut Engine, refr: u32, link: u32, mod_index: u8) -> bool {
    let id = e.mem.u32(refr + REF_ID);
    let list = refr + extra::REFR_EXTRA_LIST;
    let x = extra::get_by_type(e, list, EXTRA_LINKED_REF);
    if link == 0 {
        if let Some(&default) = saved(e).linked_ref_defaults.get(&id) {
            if x != 0 {
                if default == 0 {
                    extra::remove_by_type(e, list, EXTRA_LINKED_REF);
                } else {
                    let form = lookup_form(e, default);
                    if is_reference(e, form) {
                        e.mem.set_u32(x + LINKED_REF, form);
                    }
                }
            }
            saved(e).linked_ref_defaults.erase(&id);
        }
        saved(e).linked_refs.erase(&id);
        return true;
    }
    if x == 0 {
        let created = extra::create(e, VTBL_LINKED_REF, EXTRA_LINKED_REF);
        e.mem.set_u32(created + LINKED_REF, link);
        extra::add_extra(e, list, created);
        *saved(e).linked_ref_defaults.emplace(id) = 0;
    } else {
        let old = e.mem.u32(x + LINKED_REF);
        if old == 0 {
            return false;
        }
        let old_id = e.mem.u32(old + REF_ID);
        *saved(e).linked_ref_defaults.emplace(id) = old_id;
        e.mem.set_u32(x + LINKED_REF, link);
    }
    let link_id = e.mem.u32(link + REF_ID);
    *saved(e).linked_refs.emplace(id) = crate::cosave::LinkedRef { link_id, mod_index };
    true
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: SetLinkedRefID.
/// The link of the reference `this_id` set to the reference `link_id` (0: its default back);
/// false when either id is not a loaded reference or [`set_linked_ref`] refuses.
pub(crate) fn set_linked_ref_id(e: &mut Engine, this_id: u32, link_id: u32, mod_index: u8) -> bool {
    let this = lookup_form(e, this_id);
    if !is_reference(e, this) {
        return false;
    }
    if link_id == 0 {
        return set_linked_ref(e, this, 0, 0xff);
    }
    let link = lookup_form(e, link_id);
    is_reference(e, link) && set_linked_ref(e, this, link, mod_index)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/serialization.h: RestoreLinkedRefs.
/// Every reference with a default link gets it back ([`set_linked_ref_id`] with 0), except, with
/// `keep_temp`, those the load set again (`s_linkedRefsTemp`). The keys are taken first, as JIP
/// steps its iterator before each call.
pub(crate) fn restore_linked_refs(e: &mut Engine, keep_temp: bool) {
    let s = saved(e);
    let keys: Vec<u32> = s
        .linked_ref_defaults
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| !keep_temp || s.linked_refs_temp.get(k).is_none())
        .collect();
    for key in keys {
        set_linked_ref_id(e, key, 0, 0xff);
    }
}

/// `DoLoadGameHousekeeping`'s part: with the linked references changed since the save last loaded
/// or saved, the defaults the load did not set again restored.
pub(crate) fn after_load(e: &mut Engine) {
    if saved(e).changed & changed::LINKED_REFS != 0 {
        restore_linked_refs(e, true);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_misc_ref.h:
// Cmd_SetLinkedReference_Execute.
/// `ref.SetLinkedReference [linkRef]`: JIP's `SetLinkedRef` with the calling script's overriding
/// mod; on success result 1 and the linked references marked changed. No link: the default back.
fn set_linked_reference(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let link = read.form(0);
    let mod_index = super::jip_fn_aux_variable::overriding_mod(e, a.script) as u8;
    if set_linked_ref(e, a.this_obj, link, mod_index) {
        a.set_result(e, 1.0);
        saved(e).changed |= changed::LINKED_REFS;
    }
    handled(true)
}

#[cfg(test)]
#[path = "jip_fn_linked_ref_tests.rs"]
mod tests;
