//! xNVSE's `Commands_MiscRef.cpp` (xNVSE), the rest after the cell scans: the ownership tests,
//! the player's swimming and the free camera, an actor's foot IK, effect shaders playing on a
//! reference, editor ids, new form lists, the up/down angle to a target and the equipment
//! re-evaluation. (`Update3D` is left out: for the player xNVSE patches a byte of the game's code,
//! which the port's translated function does not read.)

use super::misc_ref::{owner_of, passed_over, rank_of};
use super::{console_print, double_words, dynamic_cast, is_console_mode, rtti, Read};
use crate::invref::{self, get_by_type};
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
/// Parameter types (xNVSE `CommandTable.h`, `ScriptUtils.h`): `kParamType_String`, `_Integer`,
/// `_ObjectRef`, `_Actor`, `_EffectShader`, `_AnyForm`, `kNVSEParamType_String`, `_Array`.
const STRING: u32 = 0x00;
const INT: u32 = 0x01;
const OBJECT_REF: u32 = 0x04;
const ACTOR: u32 = 0x06;
const EFFECT_SHADER: u32 = 0x24;
const ANY_FORM: u32 = 0x3d;
const NVSE_STRING: u32 = 0x8004;
const NVSE_ARRAY: u32 = 0x8040;
/// `kParams_OneOptionalObjectRef`, `kParams_OneOptionalActorRef`, `kParams_OneInt`,
/// `kParams_OneEffectShader`, `kParams_OneForm_OneString`,
/// `kNVSEParams_OneOptionalString_OneOptionalArray`, `kParams_OneObjectRef` (xNVSE).
const OPTIONAL_OBJECT_REF: &[Param] = &[p("ref", OBJECT_REF, true)];
const OPTIONAL_ACTOR_REF: &[Param] = &[p("actor reference", ACTOR, true)];
const ONE_INT: &[Param] = &[p("int", INT, false)];
const ONE_EFFECT_SHADER: &[Param] = &[p("effectShader", EFFECT_SHADER, false)];
const ONE_FORM_ONE_STRING: &[Param] = &[p("form", ANY_FORM, false), p("string", STRING, false)];
const OPTIONAL_STRING_OPTIONAL_ARRAY: &[Param] =
    &[p("string", NVSE_STRING, true), p("array", NVSE_ARRAY, true)];
const ONE_OBJECT_REF: &[Param] = &[p("ref", OBJECT_REF, false)];

/// `TESForm::refID` (`+0xC`). `TESObjectREFR`: `baseForm` (`+0x20`), `rotX` (`+0x24`), `posX`
/// (`+0x30`), `parentCell` (`+0x40`), `extraDataList` (`+0x44`). `TESObjectCELL`:
/// `extraDataList` (`+0x28`), `worldSpace` (`+0xC0`).
const FORM_ID: u32 = 0xc;
const REFR_BASE_FORM: u32 = 0x20;
const REFR_ROT_X: u32 = 0x24;
const REFR_POS: u32 = 0x30;
const REFR_PARENT_CELL: u32 = 0x40;
const REFR_EXTRA: u32 = invref::REFR_EXTRA;
const CELL_EXTRA: u32 = 0x28;
const CELL_WORLD: u32 = 0xc0;
/// `ExtraPersistentCell` (`+0xC` the cell) and `ExtraFactionChanges` (`+0xC` its list: each
/// entry the faction `+0` and a rank byte `+4`).
const EXTRA_PERSISTENT_CELL: u32 = 0x0c;
const EXTRA_FACTION_CHANGES: u32 = 0x5e;
const EXTRA_VALUE: u32 = 0xc;
/// `TESActorBaseData::factionList` (`+0x2C`, the run-time layout: the editor's is `+0x28`).
const ACTOR_BASE_FACTIONS: u32 = 0x2c;
/// `PlayerCharacter::GetSingleton` (`*0x011DEA3C`); `Actor::actorMover` (`+0x190`), whose
/// vtable's ninth entry (`+0x20`) is the player mover's movement flags (`PlayerMover`'s vtable
/// `0109297C` holds `009185E0` there, which answers `MoveMode` at `+0x94`); bit 11 is swimming.
const PLAYER: u32 = 0x011d_ea3c;
const ACTOR_MOVER: u32 = 0x190;
const MOVEMENT_FLAGS: u32 = 0x20;
const SWIMMING_BIT: u32 = 11;
/// `g_osGlobals` (`*0x011DEA0C`), its `isFlycam` byte (`+6`).
const OS_GLOBALS: u32 = 0x011d_ea0c;
const IS_FLYCAM: u32 = 6;
/// `Actor::ragDollController` (`+0xAC`); `bhkRagdollController`'s byte `+0x21F` (FIK can be set)
/// and `fikStatus` (`+0x220`).
const RAGDOLL: u32 = 0xac;
const RAGDOLL_FIK_SETTABLE: u32 = 0x21f;
const RAGDOLL_FIK: u32 = 0x220;
/// `g_actorProcessManager` (`0x011E0E80`) and its `tempEffects` (`+0x60`, a `tList`);
/// `MagicHitEffect::target` (`+0x1C`), `MagicShaderHitEffect::effectShader` (`+0x30`).
const PROCESS_MANAGER: u32 = 0x011e_0e80;
const TEMP_EFFECTS: u32 = 0x60;
const HIT_TARGET: u32 = 0x1c;
const HIT_SHADER: u32 = 0x30;
/// `TESForm::SetEditorID_AtRuntime`, the `+0x134` virtual function (`BGSListForm`'s vtable
/// `010334B4` holds `00401290` there, answering 1; `+0x130`, the editor id's getter, answers a
/// fixed empty text).
const SET_EDITOR_ID: u32 = 0x134;
/// `TESObjectREFR::IsCharacter` (index 0x86, `+0x218`: `Character`'s vtable `0108AA3C` holds
/// `008D0360`, answering 1) and `GetValidBip01Names` (index 0x7A, `+0x1E8`: `00950A90`).
const IS_CHARACTER: u32 = 0x218;
const VALID_BIP01_NAMES: u32 = 0x1e8;
/// The equipment re-evaluation xNVSE calls (`0x606540`, `thiscall` on the base form: the actor,
/// its bip names, 1).
const EVALUATE_EQUIPMENT: u32 = 0x0060_6540;
/// `FormHeap_Allocate`, `BGSListForm`'s constructor (`0x58F9D0`) and size (0x24), its list
/// (`+0x18`) and `numAddedObjects` (`+0x20`); `LookupFormByID`.
const FORM_HEAP_ALLOCATE: u32 = 0x0040_1000;
const LIST_FORM_CTOR: u32 = 0x0058_f9d0;
const LIST_FORM_SIZE: u32 = 0x24;
const LIST_FORM_LIST: u32 = 0x18;
const LOOKUP_FORM: u32 = 0x0048_39c0;
/// `Flt180dPI` (`utility.h` (xNVSE), a `float`).
const FLT_180_D_PI: f32 = 57.295_78;

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
        returns: Returns::Default,
    }
}

pub static COMMANDS: &[Command] = &[
    cmd(
        0x14f7,
        "HasOwnership",
        "",
        "check if an NPC has direct or indirect ownership",
        true,
        OPTIONAL_OBJECT_REF,
        has_ownership,
        Some(has_ownership_eval),
    ),
    cmd(
        0x14f8,
        "IsOwned",
        "",
        "check if an object is directly or indirectly owned by an NPC",
        true,
        OPTIONAL_ACTOR_REF,
        is_owned,
        Some(is_owned_eval),
    ),
    cmd(
        0x15a0,
        "IsPlayerSwimming",
        "",
        "check if the player is swimming",
        false,
        &[],
        is_player_swimming,
        Some(is_player_swimming_eval),
    ),
    cmd(
        0x15a1,
        "GetTFC",
        "",
        "returns TFC status",
        false,
        &[],
        get_tfc,
        Some(get_tfc_eval),
    ),
    cmd(
        0x1605,
        "GetActorFIKstatus",
        "GetFIK",
        "get an actor Foot IK status",
        true,
        &[],
        get_actor_fik_status,
        Some(get_actor_fik_status_eval),
    ),
    cmd(
        0x1606,
        "SetActorFIKstatus",
        "SetFIK",
        "set an actor Foot IK status",
        true,
        ONE_INT,
        set_actor_fik_status,
        None,
    ),
    cmd(
        0x1608,
        "HasEffectShader",
        "",
        "returns 1 if the reference is playing the effect shader",
        true,
        ONE_EFFECT_SHADER,
        has_effect_shader,
        None,
    ),
    cmd(
        0x1645,
        "SetEditorID",
        "",
        "\"sets editor id of form\"",
        false,
        ONE_FORM_ONE_STRING,
        set_editor_id,
        None,
    ),
    Command {
        returns: Returns::Form,
        ..cmd(
            0x164c,
            "CreateFormList",
            "",
            "\"creates a formList, optionally set with an editorID and filled by an array.\"",
            false,
            OPTIONAL_STRING_OPTIONAL_ARRAY,
            create_form_list,
            None,
        )
    },
    cmd(
        0x165e,
        "GetHeadingAngleX",
        "",
        "\"Gets the up/down angle the calling reference is relative to a target.\"",
        true,
        ONE_OBJECT_REF,
        get_heading_angle_x,
        None,
    ),
    cmd(
        0x1674,
        "EvaluateInventory",
        "EvalInv",
        "\"Force the actor to re-evaluate their equipped items\"",
        true,
        &[],
        evaluate_inventory,
        None,
    ),
];

fn form_id(e: &Engine, form: u32) -> u32 {
    e.mem.u32(form + FORM_ID)
}

fn set(e: &mut Engine, result: u32, v: f64) {
    e.mem.set_f64(result, v);
}

// ---- ownership ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameExtraData.cpp: GetExtraFactionRank;
// GameForms.cpp: TESActorBaseData::GetFactionRank; Commands_MiscRef.cpp: GetFactionRank.
/// The rank of a reference in a faction: its own `ExtraFactionChanges` entry for it, else its base
/// form's (as `TESActorBaseData`) faction list; -1 for neither.
fn faction_rank(e: &mut Engine, refr: u32, faction: u32) -> i8 {
    if refr == 0 || faction == 0 {
        return -1;
    }
    let changes = get_by_type(e, refr + REFR_EXTRA, EXTRA_FACTION_CHANGES);
    if changes != 0 {
        let mut node = e.mem.u32(changes + EXTRA_VALUE);
        while node != 0 {
            let data = e.mem.u32(node);
            if data != 0 && e.mem.u32(data) == faction {
                return e.mem.u8(data + 4) as i8;
            }
            node = e.mem.u32(node + 4);
        }
    }
    let base = e.mem.u32(refr + REFR_BASE_FORM);
    let actor_base = dynamic_cast(e, base, rtti::TES_FORM, rtti::TES_ACTOR_BASE_DATA);
    if actor_base == 0 {
        return -1;
    }
    let mut node = actor_base + ACTOR_BASE_FACTIONS;
    while !(node == 0 || (e.mem.u32(node) == 0 && e.mem.u32(node + 4) == 0)) {
        let data = e.mem.u32(node);
        if data != 0 && e.mem.u32(data) == faction {
            return e.mem.u8(data + 4) as i8;
        }
        node = e.mem.u32(node + 4);
    }
    -1
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_HasOwnership_Eval, Cmd_IsOwned_Eval (their shared body).
/// Whether `who` owns `item`: the item's owner (else its cell's, with the cell's required rank;
/// the item's own rank with its own owner) is `who`'s base form, or a faction `who` holds that
/// rank in. A missing cell or base form is read through, as in xNVSE, unless [`passed_over`]:
/// then 0.
fn owns(e: &mut Engine, item: u32, who: u32) -> f64 {
    let mut rank: i8 = 0;
    let mut owner = owner_of(e, item + REFR_EXTRA);
    if owner == 0 {
        let cell = e.mem.u32(item + REFR_PARENT_CELL);
        if passed_over(e, cell) {
            return 0.0;
        }
        owner = owner_of(e, cell + CELL_EXTRA);
        if owner != 0 {
            rank = rank_of(e, cell + CELL_EXTRA) as i8;
        }
    } else {
        rank = rank_of(e, item + REFR_EXTRA) as i8;
    }
    if owner == 0 {
        return 0.0;
    }
    let base = e.mem.u32(who + REFR_BASE_FORM);
    if passed_over(e, base) {
        return 0.0;
    }
    if form_id(e, owner) == form_id(e, base) {
        return 1.0;
    }
    let faction = dynamic_cast(e, owner, rtti::TES_FORM, rtti::TES_FACTION);
    if faction != 0 && faction_rank(e, who, faction) >= rank {
        1.0
    } else {
        0.0
    }
}

/// The two evals: nothing without both references; the answer printed in console mode.
fn ownership_eval(e: &mut Engine, w: &[u32], is_owned: bool) -> Ret {
    let a = EvalArgs::new(w);
    set(e, a.result, 0.0);
    if a.this_obj == 0 || a.arg1 == 0 {
        return handled(true);
    }
    let v = if is_owned {
        owns(e, a.this_obj, a.arg1)
    } else {
        owns(e, a.arg1, a.this_obj)
    };
    set(e, a.result, v);
    if is_console_mode(e) {
        let format = if is_owned {
            "IsOwned >> %.2f"
        } else {
            "HasOwnership >> %.2f"
        };
        console_print(e, format, &double_words(v));
    }
    handled(true)
}

fn has_ownership_eval(e: &mut Engine, w: &[u32]) -> Ret {
    ownership_eval(e, w, false)
}

fn is_owned_eval(e: &mut Engine, w: &[u32]) -> Ret {
    ownership_eval(e, w, true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_HasOwnership_Execute, Cmd_IsOwned_Execute.
/// The eval with the reference read (none given: 0).
fn ownership(e: &mut Engine, w: &[u32], is_owned: bool) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    ownership_eval(e, &[a.this_obj, read.form(0), 0, a.result], is_owned)
}

fn has_ownership(e: &mut Engine, w: &[u32]) -> Ret {
    ownership(e, w, false)
}

fn is_owned(e: &mut Engine, w: &[u32]) -> Ret {
    ownership(e, w, true)
}

// ---- the player and the camera ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_IsPlayerSwimming_Eval; GameObjects.h: PlayerCharacter::IsPlayerSwimming.
fn swimming(e: &mut Engine, result: u32) -> Ret {
    let pc = e.global::<u32>(PLAYER);
    let mover = e.mem.u32(pc + ACTOR_MOVER);
    let flags = e.vcall(mover, MOVEMENT_FLAGS, &args![]).u32();
    let v = f64::from((flags >> SWIMMING_BIT) & 1);
    set(e, result, v);
    if is_console_mode(e) {
        console_print(e, "Player is swimming? %f", &double_words(v));
    }
    handled(true)
}

fn is_player_swimming_eval(e: &mut Engine, w: &[u32]) -> Ret {
    swimming(e, EvalArgs::new(w).result)
}

fn is_player_swimming(e: &mut Engine, w: &[u32]) -> Ret {
    swimming(e, CommandArgs::new(w).result)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp: Cmd_GetTFC_Eval.
fn tfc(e: &mut Engine, result: u32) -> Ret {
    let globals = e.global::<u32>(OS_GLOBALS);
    let v = f64::from(e.mem.u8(globals + IS_FLYCAM));
    set(e, result, v);
    if is_console_mode(e) {
        console_print(e, "GetTFC: %f", &double_words(v));
    }
    handled(true)
}

fn get_tfc_eval(e: &mut Engine, w: &[u32]) -> Ret {
    tfc(e, EvalArgs::new(w).result)
}

fn get_tfc(e: &mut Engine, w: &[u32]) -> Ret {
    tfc(e, CommandArgs::new(w).result)
}

// ---- foot IK ----

/// The actor's ragdoll controller, 0 for a reference that is not an actor or has none.
fn ragdoll(e: &mut Engine, refr: u32) -> u32 {
    let actor = dynamic_cast(e, refr, rtti::TES_OBJECT_REFR, rtti::ACTOR);
    if actor == 0 {
        0
    } else {
        e.mem.u32(actor + RAGDOLL)
    }
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetActorFIKstatus_Eval.
/// The ragdoll's foot IK byte; without a calling reference the handler answers false.
fn fik_status(e: &mut Engine, this_obj: u32, result: u32) -> Ret {
    set(e, result, 0.0);
    if this_obj == 0 {
        return handled(false);
    }
    let rag = ragdoll(e, this_obj);
    if rag != 0 {
        let v = f64::from(e.mem.u8(rag + RAGDOLL_FIK));
        set(e, result, v);
    }
    handled(true)
}

fn get_actor_fik_status_eval(e: &mut Engine, w: &[u32]) -> Ret {
    let a = EvalArgs::new(w);
    fik_status(e, a.this_obj, a.result)
}

fn get_actor_fik_status(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    fik_status(e, a.this_obj, a.result)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetActorFIKstatus_Execute.
/// Sets the foot IK byte (1 or 0) of a ragdoll whose byte `+0x21F` allows it; the result stays 0.
/// The read comes first; then without a calling reference the handler answers false.
fn set_actor_fik_status(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    if a.this_obj == 0 {
        return handled(false);
    }
    let rag = ragdoll(e, a.this_obj);
    if rag != 0 && e.mem.u8(rag + RAGDOLL_FIK_SETTABLE) != 0 {
        e.mem
            .set_u8(rag + RAGDOLL_FIK, u8::from(read.int(0, 0) != 0));
    }
    handled(true)
}

// ---- effect shaders ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_HasEffectShader_Execute, EffectShaderFinder.
/// How many of the process manager's temporary effects are shader hit effects of this shader on
/// the calling reference.
fn has_effect_shader(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    if a.this_obj == 0 {
        return handled(true);
    }
    let read = Read::of(e, &a);
    let shader = read.form(0);
    if !read.ok || shader == 0 {
        return handled(true);
    }
    let mut count = 0u32;
    let mut node = PROCESS_MANAGER + TEMP_EFFECTS;
    while node != 0 {
        let effect = e.mem.u32(node);
        if effect != 0 {
            let hit = dynamic_cast(
                e,
                effect,
                rtti::BS_TEMP_EFFECT,
                rtti::MAGIC_SHADER_HIT_EFFECT,
            );
            if hit != 0
                && e.mem.u32(hit + HIT_TARGET) == a.this_obj
                && e.mem.u32(hit + HIT_SHADER) == shader
            {
                count += 1;
            }
        }
        node = e.mem.u32(node + 4);
    }
    a.set_result(e, f64::from(count));
    handled(true)
}

// ---- editor ids and form lists ----

/// `TESForm::SetEditorID_AtRuntime(text)`.
fn set_form_editor_id(e: &mut Engine, form: u32, text: &[u8]) {
    crate::events::with_guest_text(e, text, |e, at| {
        e.vcall(form, SET_EDITOR_ID, &args![at]);
    });
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_SetEditorID_Execute.
/// The form's editor id set (through the form's own function, which most forms answer without
/// keeping it); 1 when the arguments were read.
fn set_editor_id(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let text = read.text(1);
    set_form_editor_id(e, read.form(0), &text);
    a.set_result(e, 1.0);
    handled(true)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_CreateFormList_Execute; GameForms.cpp: BGSListForm::MakeUnique; GameForms.h:
// BGSListForm::AddAtTemp.
/// A new form list (its id the result), with the editor id given (any first argument: a text, or
/// "" for anything else) and the forms of the array given, in its order. `AddAtTemp` puts each at
/// the end: xNVSE's `max(eListEnd, numAddedObjects)` compares -2 with an unsigned 0 as unsigned
/// and keeps -2 (`eListEnd`), so `numAddedObjects` stays 0. An element that is no form, or a form
/// id the game does not know, adds nothing.
fn create_form_list(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    super::string::with_eval(e, &a, |e, ev, toks| {
        let Some(toks) = toks else {
            return;
        };
        let list = e.call(FORM_HEAP_ALLOCATE, &args![LIST_FORM_SIZE]).u32();
        e.call(LIST_FORM_CTOR, &args![list]);
        if list == 0 {
            return;
        }
        let id = form_id(e, list);
        a.set_form_result(e, id);
        if let Some(first) = toks.first() {
            let edid = ev.get_string(e, first);
            set_form_editor_id(e, list, &edid);
            if let Some(second) = toks.get(1) {
                let array = ev.get_array_id(e, second);
                let keys = crate::vars::array(e, array).map(|v| v.keys());
                for key in keys.unwrap_or_default() {
                    let element = crate::vars::get_element(e, array, &key);
                    if let Some(crate::vars::Element {
                        data: crate::vars::Data::Form(fid),
                        ..
                    }) = element
                    {
                        let form = e.call(LOOKUP_FORM, &args![fid]).u32();
                        invref::list_append(e, list + LIST_FORM_LIST, form);
                    }
                }
            }
        }
    });
    handled(true)
}

// ---- angles ----

// Translated from JIP LN NVSE's code in xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameObjects.cpp:
// TESObjectREFR::GetInSameCellOrWorld.
/// Whether two references are in the same cell, or in cells of the same worldspace (a
/// reference in no cell counts its persistent cell).
fn same_cell_or_world(e: &mut Engine, a: u32, b: u32) -> bool {
    let cell_of = |e: &mut Engine, r: u32| {
        let cell = e.mem.u32(r + REFR_PARENT_CELL);
        if cell != 0 {
            return Some(cell);
        }
        let x = get_by_type(e, r + REFR_EXTRA, EXTRA_PERSISTENT_CELL);
        (x != 0).then(|| e.mem.u32(x + EXTRA_VALUE))
    };
    let Some(first) = cell_of(e, a) else {
        return false;
    };
    let Some(second) = cell_of(e, b) else {
        return false;
    };
    if first == second {
        return true;
    }
    let world = e.mem.u32(first + CELL_WORLD);
    world != 0 && world == e.mem.u32(second + CELL_WORLD)
}

// Translated from JIP LN NVSE's code in xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), NiTypes.cpp:
// Point3Distance.
/// The distance between two points as JIP's SSE code makes it: the squares summed pairwise
/// (x + y, then z), the hardware's reciprocal square root estimate and one Newton step; 0 (or a
/// NaN) as it is.
fn point_distance(e: &Engine, a: u32, b: u32) -> f32 {
    let d = |i: u32| e.mem.f32(a + 4 * i) - e.mem.f32(b + 4 * i);
    let (x, y, z) = (d(0), d(1), d(2));
    let s = (x * x + y * y) + z * z;
    if s == 0.0 || s.is_nan() {
        return s;
    }
    let r = engine_core::x87::rsqrt_estimate(s);
    let t = s * r * r;
    let k = (3.0 - t) * r * 0.5;
    s * k
}

// Translated from JIP LN NVSE's code in xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameObjects.cpp:
// TESObjectREFR::GetDistance.
/// The distance between two references; the largest `float` when they are not in the same cell
/// or worldspace.
fn distance(e: &mut Engine, a: u32, b: u32) -> f32 {
    if !same_cell_or_world(e, a, b) {
        return f32::MAX;
    }
    point_distance(e, a + REFR_POS, b + REFR_POS)
}

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_GetHeadingAngleX_Execute.
/// The up/down angle in degrees from the calling reference's x rotation to the target, in
/// `float`s as xNVSE computes it (`asin` of the height difference over the distance).
fn get_heading_angle_x(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    a.set_result(e, 0.0);
    let read = Read::of(e, &a);
    if !read.ok || a.this_obj == 0 {
        return handled(true);
    }
    let target = read.form(0);
    let opposite = e.mem.f32(a.this_obj + REFR_POS + 8) - e.mem.f32(target + REFR_POS + 8);
    let hypotenuse = distance(e, a.this_obj, target);
    let fraction = opposite / hypotenuse;
    let v = (fraction.asin() - e.mem.f32(a.this_obj + REFR_ROT_X)) * FLT_180_D_PI;
    a.set_result(e, f64::from(v));
    handled(true)
}

// ---- equipment ----

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Commands_MiscRef.cpp:
// Cmd_EvaluateInventory_Execute.
/// The game's equipment re-evaluation for a character (an actor whose `IsCharacter` answers 1);
/// the result is not touched.
fn evaluate_inventory(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    if dynamic_cast(e, a.this_obj, rtti::TES_OBJECT_REFR, rtti::ACTOR) == 0 {
        return handled(true);
    }
    if e.vcall(a.this_obj, IS_CHARACTER, &args![]).u32() & 0xff == 0 {
        return handled(true);
    }
    let base = e.mem.u32(a.this_obj + REFR_BASE_FORM);
    let names = e.vcall(a.this_obj, VALID_BIP01_NAMES, &args![]).u32();
    e.call(EVALUATE_EQUIPMENT, &args![base, a.this_obj, names, 1u32]);
    handled(true)
}

#[cfg(test)]
#[path = "misc_ref_more_tests.rs"]
mod tests;
