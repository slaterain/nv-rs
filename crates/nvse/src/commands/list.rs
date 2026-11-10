//! xNVSE's `Commands_List.cpp` (xNVSE): form lists (count, nth form, index, add, remove, replace,
//! clear, a function called on each form, the count of forms scripts added) and a leveled list's
//! count of entries scripts added.
//!
//! Read from xNVSE's source and checked command by command in the shipped DLL (xNVSE 6.4.9's
//! `nvse_1_4.dll`, executes `10031270` to `100320F6`; `BGSListForm`'s helpers of `GameForms.h` are
//! inlined there, `tList::AddAt` at `10032180` and `tList::RemoveNth` at `10032100`).

use super::misc_forms::{add_at, count, nth_item, remove_nth, replace_nth};
use super::{console_print, double_words, dynamic_cast, is_console_mode, rtti, Read};
use crate::table::{Command, Param, Returns};
use crate::{handled, CommandArgs, EvalArgs};
use engine_core::prelude::*;

const fn p(name: &'static str, kind: u32, optional: bool) -> Param {
    Param {
        name,
        kind,
        optional,
    }
}
/// Parameter types (xNVSE `CommandTable.h`): `kParamType_Integer`, `_FormList`, `_AnyForm`.
const INT: u32 = 0x01;
const FORM_LIST: u32 = 0x25;
const ANY_FORM: u32 = 0x3d;
/// `kParams_FormList`, `kParams_FormListInteger`, `kParams_FormListForm`, `kParams_AddToFormList`,
/// `kParams_FormList_OptionalInt`, `kParams_FormList_TwoOptionalInts`, `kParams_ReplaceNthForm`,
/// `kParams_ReplaceForm`, `kParams_OneFormList_OneFunction`, `kParams_OneForm` (xNVSE), as the
/// DLL's records hold them.
const ONE_LIST: &[Param] = &[p("form list", FORM_LIST, false)];
const LIST_INT: &[Param] = &[p("form list", FORM_LIST, false), p("index", INT, false)];
const LIST_FORM: &[Param] = &[p("form list", FORM_LIST, false), p("form", ANY_FORM, true)];
const ADD_TO_LIST: &[Param] = &[
    p("form list", FORM_LIST, false),
    p("form", ANY_FORM, false),
    p("index", INT, true),
    p("bCheckForDuplicates", INT, true),
];
const LIST_OPTIONAL_INT: &[Param] = &[p("form list", FORM_LIST, false), p("index", INT, true)];
const LIST_TWO_OPTIONAL_INTS: &[Param] = &[
    p("form list", FORM_LIST, false),
    p("index", INT, true),
    p("bCheckForDuplicates", INT, true),
];
const REPLACE_NTH: &[Param] = &[
    p("form list", FORM_LIST, false),
    p("replaceWith", ANY_FORM, false),
    p("formIndex", INT, true),
];
const REPLACE: &[Param] = &[
    p("form list", FORM_LIST, false),
    p("replaceWith", ANY_FORM, false),
    p("form", ANY_FORM, false),
];
const LIST_FUNCTION: &[Param] = &[
    p("form list", FORM_LIST, false),
    p("condition user defined function", ANY_FORM, false),
];
const ONE_FORM: &[Param] = &[p("form", ANY_FORM, false)];

/// `TESForm`: its type byte (`+4`), `refID` (`+0xC`). `BGSListForm`: its forms (`+0x18`, a
/// `tList`) and `numAddedObjects` (`+0x20`). `TESObjectREFR::baseForm` (`+0x20`). A leveled list's
/// `scriptAddedObjects` (`+0x14` of the `TESLeveledList`).
const FORM_TYPE: u32 = 4;
const FORM_ID: u32 = 0xc;
const LIST_FORMS: u32 = 0x18;
const LIST_ADDED: u32 = 0x20;
const REFR_BASE_FORM: u32 = 0x20;
const LEVELED_SCRIPT_ADDED: u32 = 0x14;
/// `kFormType_BGSListForm`, `kFormType_Script` (the byte the DLL compares, `10031F3B`).
const TYPE_LIST: u8 = 0x55;
const TYPE_SCRIPT: u8 = 0x11;
/// `TESLeveledList::GetFormAsLeveledList` (`00487BE0`, `cdecl`: the form's leveled list, 0 for a
/// form that has none).
const AS_LEVELED_LIST: u32 = 0x0048_7be0;
/// `FormHeap_Free`.
const FORM_HEAP_FREE: u32 = 0x0040_1030;
/// `eListEnd`, `eListInvalid` (xNVSE `GameTypes.h`).
const LIST_END: i32 = -2;
const LIST_INVALID: i32 = -1;

#[allow(clippy::too_many_arguments)]
const fn cmd(
    opcode: u32,
    name: &'static str,
    short: &'static str,
    help: &'static str,
    needs_ref: bool,
    params: &'static [Param],
    execute: AbiFn,
    eval: Option<AbiFn>,
    returns: Returns,
) -> Command {
    Command {
        opcode,
        name,
        short,
        help,
        needs_ref,
        params,
        execute,
        eval,
        returns,
    }
}

use Returns::{Default as RD, Form as RF};

#[rustfmt::skip]
pub static COMMANDS: &[Command] = &[
    cmd(0x143f, "ListGetCount", "", "\"returns the count of items in the form list\"", false, ONE_LIST, list_get_count, None, RD),
    cmd(0x1440, "ListGetNthForm", "", "returns the nth form in the form list", false, LIST_INT, list_get_nth_form, None, RF),
    cmd(0x1441, "ListGetFormIndex", "", "returns the index for the specified form", false, LIST_FORM, list_get_form_index, Some(list_get_form_index_eval), RD),
    cmd(0x1442, "ListAddForm", "", "adds the form to the list at the given index (or at the end if not provided)", false, ADD_TO_LIST, list_add_form, None, RD),
    cmd(0x1443, "ListAddReference", "ListAddRef", "adds the calling reference at the given index (or at the end if not provided)", true, LIST_TWO_OPTIONAL_INTS, list_add_reference, None, RD),
    cmd(0x1444, "ListRemoveNthForm", "ListRemoveNth", "removes the nth form from the list", false, LIST_OPTIONAL_INT, list_remove_nth_form, None, RD),
    cmd(0x1445, "ListRemoveForm", "", "removes the specified from from the list.", false, LIST_FORM, list_remove_form, None, RD),
    cmd(0x1446, "ListReplaceNthForm", "ListReplaceNth", "replaces the nth form of the list with the specified form", false, REPLACE_NTH, list_replace_nth_form, None, RD),
    cmd(0x1447, "ListReplaceForm", "", "replaces the specified from with another.", false, REPLACE, list_replace_form, None, RD),
    cmd(0x1448, "ListClear", "", "removes all entries from the list", false, ONE_LIST, list_clear, None, RD),
    cmd(0x1511, "IsRefInList", "", "returns the index for the specified form or base Form or permanent base Form", false, LIST_FORM, is_ref_in_list, Some(is_ref_in_list_eval), RD),
    cmd(0x1638, "ForEachInList", "", "\"invokes a UDF that is called on each form in a formlist.\"", false, LIST_FUNCTION, for_each_in_list, None, RD),
    cmd(0x168c, "ListGetSaveBakedObjectCount", "", "\"returns the count of savebaked items in the form list (Added with AddFormToFormList). The number is also the end range of said added forms\"", false, ONE_LIST, list_get_save_baked_object_count, None, RD),
    cmd(0x168d, "GetNumLevSaveBakedItems", "LListGetSaveBakedCount", "\"returns the count of savebaked items in the leveled list (Added with AddItemToLeveledList or AddNPCToLeveledList)\"", false, ONE_FORM, get_num_lev_save_baked_items, None, RD),
];

fn form_id(e: &Engine, form: u32) -> u32 {
    e.mem.u32(form + FORM_ID)
}

/// The list's forms (its `tList` head).
fn forms(list: u32) -> u32 {
    list + LIST_FORMS
}

fn added(e: &Engine, list: u32) -> u32 {
    e.mem.u32(list + LIST_ADDED)
}

// ---- BGSListForm (GameForms.h (xNVSE), inlined in the DLL) ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameForms.h: BGSListForm::GetIndexOf.
/// The index of the first node whose form has `form`'s id, every node counted (an empty one
/// too), the walk going to the last node (a range `for` over the nodes, as the DLL walks it,
/// `10031920`); -1 for none. `form`'s id is read only when a node holds a form: for no form at all
/// xNVSE reads through nothing and the game stops, as the port does without the script reference
/// checks row; with it nothing is found.
fn index_of(e: &mut Engine, list: u32, form: u32) -> i32 {
    let mut node = forms(list);
    let mut idx: i32 = 0;
    while node != 0 {
        let item = e.mem.u32(node);
        if item != 0 {
            if super::misc_ref::passed_over(e, form) {
                return LIST_INVALID;
            }
            if form_id(e, item) == form_id(e, form) {
                return idx;
            }
        }
        node = e.mem.u32(node + 4);
        idx = idx.wrapping_add(1);
    }
    LIST_INVALID
}

/// `BGSListForm::IsAddedObject`: 0 up to (not including) `numAddedObjects`, compared unsigned.
fn is_added(e: &Engine, list: u32, idx: i32) -> bool {
    idx >= 0 && (idx as u32) < added(e, list)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameForms.h: BGSListForm::AddAtTemp,
// BGSListForm::AddAt.
/// `form` added at `max(index, numAddedObjects)`, compared unsigned (`cmova` at `1003140A`: -2,
/// the end, stays -2; a smaller index goes after the forms scripts added). With `dupes`, a form
/// already in the list adds nothing and answers -1 (the DLL's check also takes a match at index
/// -1 as none, which no list reaches). An index that lands among the added forms counts one more.
fn add_at_temp(e: &mut Engine, list: u32, form: u32, index: i32, dupes: bool) -> i32 {
    let at = (index as u32).max(added(e, list)) as i32;
    if dupes && index_of(e, list, form) != LIST_INVALID {
        return LIST_INVALID;
    }
    let res = add_at(e, forms(list), form, at);
    if is_added(e, list, res) {
        let n = added(e, list) + 1;
        e.mem.set_u32(list + LIST_ADDED, n);
    }
    res
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameForms.h: BGSListForm::RemoveNthForm.
fn remove_nth_form(e: &mut Engine, list: u32, n: i32) -> u32 {
    let form = remove_nth(e, forms(list), n);
    if form != 0 && is_added(e, list, n) {
        let n = added(e, list) - 1;
        e.mem.set_u32(list + LIST_ADDED, n);
    }
    form
}

// ---- the commands ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp: Cmd_ListGetCount_Execute.
/// The list's count (`tList::Count`: 0 for an empty head, else every node), as an unsigned number.
fn list_get_count(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let list = read.form(0);
    if read.ok && list != 0 {
        let n = count(e, forms(list));
        a.set_result(e, f64::from(n));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListGetNthForm_Execute.
/// The form at `n` (-2 the last), its id in the low word; 0 for none.
fn list_get_nth_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let list = read.form(0);
    if read.ok && list != 0 {
        let form = nth_item(e, forms(list), read.int(1, 0) as i32);
        if form != 0 {
            let id = form_id(e, form);
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListGetFormIndex_Eval.
/// The form's index in the list (-1 for none, and with a list or form missing); the console
/// prints it ("Index: %d") whenever both were given.
fn index_eval(e: &mut Engine, list: u32, form: u32, result: u32) {
    e.mem.set_f64(result, -1.0);
    if list == 0 || form == 0 {
        return;
    }
    let index = index_of(e, list, form);
    e.mem.set_f64(result, f64::from(index));
    if is_console_mode(e) {
        console_print(e, "Index: %d", &[index as u32]);
    }
}

fn list_get_form_index_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    index_eval(e, a.arg1, a.arg2, a.result);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListGetFormIndex_Execute.
/// The eval with the two arguments read; -1 after a failed read.
fn list_get_form_index(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if read.ok {
        index_eval(e, read.form(0), read.form(1), a.result);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListAddForm_Execute.
/// The form added (see [`add_at_temp`]; the index -2 when not given, duplicates allowed when not
/// asked), the index it went to the result and, in the console, "Index: %d". The read's answer is
/// not looked at (`ExtractArgsEx`): without a list or a form nothing changes and the result is -1.
fn list_add_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, f64::from(LIST_INVALID));
    let read = Read::of(e, &a);
    let (list, form) = (read.form(0), read.form(1));
    let index = read.int(2, LIST_END as u32) as i32;
    let dupes = read.int(3, 0) != 0;
    if list != 0 && form != 0 {
        add_to_list(e, &a, list, form, index, dupes);
    }
    handled(true)
}

fn add_to_list(e: &mut Engine, a: &CommandArgs, list: u32, form: u32, index: i32, dupes: bool) {
    let index = add_at_temp(e, list, form, index, dupes);
    a.set_result(e, f64::from(index));
    if is_console_mode(e) {
        console_print(e, "Index: %d", &[index as u32]);
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListAddReference_Execute.
/// The calling reference added as `ListAddForm` adds a form; a failed read, no list or no calling
/// reference: nothing, -1.
fn list_add_reference(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, f64::from(LIST_INVALID));
    let read = Read::of(e, &a);
    let list = read.form(0);
    if read.ok && list != 0 && a.this_obj != 0 {
        let index = read.int(1, LIST_END as u32) as i32;
        let dupes = read.int(2, 0) != 0;
        add_to_list(e, &a, list, a.this_obj, index, dupes);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListRemoveNthForm_Execute.
/// The form at the index taken out (any index of 0 or less the head: so with no index, -2, the
/// FIRST form, `tList::RemoveNth`), its id in the low word; one fewer added form when it was one
/// of them (an index from 0).
fn list_remove_nth_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    let read = Read::of(e, &a);
    let list = read.form(0);
    if read.ok && list != 0 {
        let removed = remove_nth_form(e, list, read.int(1, LIST_END as u32) as i32);
        if removed != 0 {
            let id = form_id(e, removed);
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListReplaceNthForm_Execute.
/// The form at the index (-2, or no index, the last) replaced, the old one's id in the low word.
fn list_replace_nth_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_form_result(e, 0);
    let read = Read::of(e, &a);
    let (list, with) = (read.form(0), read.form(1));
    if read.ok && list != 0 && with != 0 {
        let replaced = replace_nth(e, forms(list), read.int(2, LIST_END as u32) as i32, with);
        if replaced != 0 {
            let id = form_id(e, replaced);
            a.set_form_result(e, id);
        }
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListRemoveForm_Execute; GameForms.h: BGSListForm::RemoveForm.
/// The first node with the form's id taken out; its index the result (-1 for none). The read's
/// answer is not looked at.
fn list_remove_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let (list, form) = (read.form(0), read.form(1));
    if list != 0 && form != 0 {
        let index = index_of(e, list, form);
        if index >= 0 {
            remove_nth_form(e, list, index);
        }
        a.set_result(e, f64::from(index));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListReplaceForm_Execute; GameForms.h: BGSListForm::ReplaceForm.
/// The first node with the form's id (the third argument) given the second; its index the result
/// (-1 for none). The read's answer is not looked at.
fn list_replace_form(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let (list, with, form) = (read.form(0), read.form(1), read.form(2));
    if list != 0 && form != 0 && with != 0 {
        let index = index_of(e, list, form);
        if index >= 0 {
            replace_nth(e, forms(list), index, with);
        }
        a.set_result(e, f64::from(index));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp: Cmd_ListClear_Execute;
// GameTypes.h: tList::RemoveAll.
/// Every node after the head freed, the head emptied, `numAddedObjects` 0. xNVSE does not look
/// for the list after a read that answered (`100318A3`): its reader refuses a missing form.
fn list_clear(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let list = read.form(0);
    let head = forms(list);
    let mut node = e.mem.u32(head + 4);
    e.mem.set_u32(head, 0);
    e.mem.set_u32(head + 4, 0);
    while node != 0 {
        let next = e.mem.u32(node + 4);
        e.call(FORM_HEAP_FREE, &args![node]);
        node = next;
    }
    e.mem.set_u32(list + LIST_ADDED, 0);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp: Cmd_IsRefInList_Eval;
// GameObjects.cpp: GetPermanentBaseForm.
/// The index of the form, else (for a reference) of its permanent base form, else of its base
/// form; with no form, of the calling reference's permanent base form, else its base form. -1
/// for none; nothing at all without a list (the result -1, no console line). A permanent base form
/// or base form that is missing (a reference without one) is read through by xNVSE when the list
/// has a form to compare with; the script reference checks row finds nothing there instead.
fn ref_in_list(e: &mut Engine, this_obj: u32, list: u32, form: u32, result: u32) {
    e.mem.set_f64(result, -1.0);
    if list == 0 {
        return;
    }
    let mut index = LIST_INVALID;
    if form != 0 {
        let obj = dynamic_cast(e, form, rtti::TES_FORM, rtti::TES_OBJECT_REFR);
        index = index_of(e, list, form);
        if index < 0 && obj != 0 {
            let permanent = super::misc_ref::permanent_base_form(e, obj);
            index = index_of(e, list, permanent);
            if index < 0 {
                let base = e.mem.u32(obj + REFR_BASE_FORM);
                index = index_of(e, list, base);
            }
        }
    } else if this_obj != 0 {
        let permanent = super::misc_ref::permanent_base_form(e, this_obj);
        index = index_of(e, list, permanent);
        if index < 0 {
            let base = e.mem.u32(this_obj + REFR_BASE_FORM);
            index = index_of(e, list, base);
        }
    }
    e.mem.set_f64(result, f64::from(index));
    if is_console_mode(e) {
        console_print(e, "Index: %d", &[index as u32]);
    }
}

fn is_ref_in_list_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    ref_in_list(e, a.this_obj, a.arg1, a.arg2, a.result);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_IsRefInList_Execute.
fn is_ref_in_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, -1.0);
    let read = Read::of(e, &a);
    if read.ok {
        ref_in_list(e, a.this_obj, read.form(0), read.form(1), a.result);
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ForEachInList_Execute, ExtractListFunctionContext.
/// The function (a script, form type 0x11) called once per node of the list (a form list, type
/// 0x55), with the node's form as its one argument (0 for an empty node before the last; the
/// list's own iterator ends at an empty last node), the calling reference and container passed
/// on; the next node read after each call. 1 when it ran, else 0.
fn for_each_in_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let (list, function) = (read.form(0), read.form(1));
    if !read.ok || list == 0 || function == 0 {
        return handled(true);
    }
    if e.mem.u8(list + FORM_TYPE) != TYPE_LIST || e.mem.u8(function + FORM_TYPE) != TYPE_SCRIPT {
        return handled(true);
    }
    let mut node = forms(list);
    while !(node == 0 || (e.mem.u32(node) == 0 && e.mem.u32(node + 4) == 0)) {
        let form = e.mem.u32(node);
        crate::functions::call(
            e,
            crate::functions::Caller::internal(function, a.this_obj, a.containing_obj, vec![form]),
        );
        node = e.mem.u32(node + 4);
    }
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_ListGetSaveBakedObjectCount_Execute.
/// `numAddedObjects`, unsigned; the console prints the result ("%f") after any read.
fn list_get_save_baked_object_count(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let list = read.form(0);
    if read.ok && list != 0 {
        let n = added(e, list);
        a.set_result(e, f64::from(n));
    }
    if is_console_mode(e) {
        let v = e.mem.f64(a.result);
        console_print(e, "ListGetSaveBakedObjectCount >> %f", &double_words(v));
    }
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_List.cpp:
// Cmd_GetNumLevSaveBakedItems_Execute.
/// The count of the form's leveled list's entries scripts added (the game's own lookup of a
/// form's leveled list); 0 for a form without one. The console prints the result ("%f").
fn get_num_lev_save_baked_items(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    let form = read.form(0);
    if read.ok && form != 0 {
        let leveled = e.call(AS_LEVELED_LIST, &args![form]).u32();
        if leveled != 0 {
            let n = count(e, leveled + LEVELED_SCRIPT_ADDED);
            a.set_result(e, f64::from(n));
        }
    }
    if is_console_mode(e) {
        let v = e.mem.f64(a.result);
        console_print(e, "GetNumLevSaveBakedItems >> %f", &double_words(v));
    }
    handled(true)
}

#[cfg(test)]
#[path = "list_tests.rs"]
mod tests;
