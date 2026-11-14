//! The game behind the new compiler's [`Env`]: the command table, the event block records, forms
//! by editor id, the compiling script's variables, the vanilla enum names, as xNVSE reads them at
//! run time.
//!
//! Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a): `Compiler/Utils.cpp`
//! (`ResolveVanillaEum`, `DoesFormTypeMatchParamType`, `IsDefaultParse`), `CommandTable.cpp`
//! (`GetParentPlugin`), `GameForms.cpp` (`TESForm::GetEditorID`), `VariableResolution.cpp` (the
//! variable and reference lists made again), `Utilities.cpp` (`vShowRuntimeError`).

use super::ast::{BlockInfo, CmdInfo, ParamInfo, Parse};
use super::Env;
use crate::commands::{dynamic_cast, rtti};
use crate::compiler::{self, sc, tt, Code};
use crate::table::{self, guest_text, record_of, Record, Returns};
use engine_core::prelude::*;

/// `g_eventBlockCommandInfos` (`0x0118E2F0`, 38 records).
const EVENT_BLOCKS: u32 = 0x0118_e2f0;
const EVENT_BLOCK_COUNT: u32 = 38;
/// The game's parse routines (`g_gameParseCommands`) and the one `IsDefaultParse` adds.
const GAME_PARSERS: [u32; 6] = [
    0x005b_1ba0,
    0x005b_3c70,
    0x005b_3ca0,
    0x005b_3c40,
    0x005b_3cd0,
    0x005c_67e0,
];
/// `GetActorValueForString` and the misc stat lookup by name (`cdecl`, a text).
const ACTOR_VALUE_FOR_STRING: u32 = 0x0066_eb40;
const MISC_STAT_FOR_STRING: u32 = 0x004d_5eb0;
/// `IsStringInteger`.
const IS_STRING_INTEGER: u32 = 0x004b_1230;
/// `g_animGroupInfos` (245 records of 0x24 bytes, the name first); the two sex names
/// (pointers); the alignment, equip type and critical stage names (pointer arrays).
const ANIM_GROUPS: u32 = 0x0119_77d8;
const SEX_0: u32 = 0x0104_a0ec;
const SEX_1: u32 = 0x0104_a0f4;
const ALIGNMENT_NAMES: u32 = 0x0118_6cf4;
const EQUIP_TYPE_NAMES: u32 = 0x0118_69bc;
const CRITICAL_STAGE_NAMES: u32 = 0x0119_bbb0;
/// The map marker base form (`*(TESForm **)0x11CA224`) and `GetContainer` (`0x55D310`,
/// `thiscall`).
const MAP_MARKER: u32 = 0x011c_a224;
const GET_CONTAINER: u32 = 0x0055_d310;
const RTTI_TES_OBJECT: u32 = 0x0118_3128;
const RTTI_TES_OBJECT_REFR: u32 = 0x0118_41cc;
const RTTI_TES_FORM: u32 = 0x0118_3028;
/// `TESForm::GetName`, the run-time build's editor id.
const GET_NAME: u32 = 0x130;
/// `kNVSEOpcodeStart`, `kNVSEOpcodeTest`.
const NVSE_OPCODE_START: u32 = 0x1400;
const NVSE_OPCODE_TEST: u32 = 0x2000;
/// `vShowRuntimeError`'s message buffer.
const MESSAGE_BUFFER: usize = 0x800;

/// [`Env`] on the running game, compiling `script` (0: none).
pub struct EngineEnv<'e> {
    pub e: &'e mut Engine,
    pub script: u32,
}

/// The game's compiler env for `script` (0 for none, as `PreProcessNVSEScript` passes).
pub fn engine_env(e: &mut Engine, script: u32) -> EngineEnv<'_> {
    EngineEnv { e, script }
}

fn params_of(e: &Engine, at: u32, count: u16) -> Vec<ParamInfo> {
    (0..u32::from(count))
        .map(|i| {
            let p = at + 12 * i;
            ParamInfo {
                name: guest_text(e, e.mem.u32(p)),
                kind: e.mem.u32(p + 4),
                optional: e.mem.u32(p + 8) & 0xff != 0,
            }
        })
        .collect()
}

/// `ToTokenType(CommandReturnType)`.
fn return_token_type(r: Returns) -> Code {
    match r {
        Returns::Default => tt::NUMBER,
        Returns::Form => tt::FORM,
        Returns::String => tt::STRING,
        Returns::Array => tt::ARRAY,
        Returns::ArrayIndex | Returns::Ambiguous => tt::INVALID,
    }
}

/// `StrCompare(l, r) == 0` with a null text for an empty pointer.
fn str_equal_at(e: &mut Engine, at: u32, text: &[u8]) -> bool {
    let l = (at != 0).then(|| guest_text(e, at));
    compiler::str_equal(e, l.as_deref(), Some(text))
}

/// A text on the guest stack for a `cdecl` call taking it.
fn call_with_text(e: &mut Engine, f: u32, text: &[u8]) -> u32 {
    let size = (text.len() as u32 + 1).next_multiple_of(4);
    e.with_stack(size, |e, at| {
        e.mem.write(at.addr(), text);
        e.mem.set_u8(at.addr() + text.len() as u32, 0);
        e.call(f, &args![at.addr()]).u32()
    })
}

/// `atoi` (MSVC's: blanks, a sign, digits, held to the `long` range).
fn atoi(text: &[u8]) -> i32 {
    let mut i = 0;
    while i < text.len() && matches!(text[i], b' ' | 0x09..=0x0d) {
        i += 1;
    }
    let negative = text.get(i) == Some(&b'-');
    if matches!(text.get(i), Some(b'+' | b'-')) {
        i += 1;
    }
    let mut v: i64 = 0;
    while i < text.len() && text[i].is_ascii_digit() {
        v = (v * 10 + i64::from(text[i] - b'0')).min(i64::from(i32::MAX) + 1);
        i += 1;
    }
    let v = if negative { -v } else { v };
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// `g_formTypeNames` (xNVSE's map, walked in key order).
const FORM_TYPE_NAMES: [(u8, &str); 88] = [
    (4, "Texture Set"),
    (5, "Menu Icon"),
    (6, "Global"),
    (7, "Class"),
    (8, "Faction"),
    (9, "Head Part"),
    (11, "Eyes"),
    (12, "Race"),
    (13, "Sound"),
    (14, "Acoustic Space"),
    (16, "Base Effect"),
    (17, "Script"),
    (18, "Land Texture"),
    (19, "Object Effect"),
    (20, "Actor Effect"),
    (21, "Activator"),
    (22, "Talking Activator"),
    (23, "Terminal"),
    (24, "Armor"),
    (25, "Book"),
    (27, "Container"),
    (28, "Door"),
    (29, "Ingredient"),
    (30, "Light"),
    (31, "Misc Item"),
    (32, "Static"),
    (33, "Static Collection"),
    (34, "Movable Static"),
    (35, "Placeable Water"),
    (36, "Grass"),
    (37, "Tree"),
    (38, "Flora"),
    (39, "Furniture"),
    (40, "Weapon"),
    (41, "Ammo"),
    (42, "NPC"),
    (43, "Creature"),
    (44, "Leveled Creature"),
    (45, "LeveledCharacter"),
    (46, "Key"),
    (47, "Ingestible"),
    (48, "Idle Marker"),
    (49, "Note"),
    (50, "Constructible Object"),
    (51, "Projectile"),
    (52, "Leveled Item"),
    (53, "Weather"),
    (54, "Climate"),
    (71, "Quest"),
    (73, "Package"),
    (74, "Combat Style"),
    (75, "Load Screen"),
    (76, "Leveled Spell"),
    (77, "Anim Object"),
    (78, "Water Type"),
    (79, "Effect Shader"),
    (81, "Explosion"),
    (82, "Debris"),
    (83, "Imagespace"),
    (84, "Imagespace Modifier"),
    (85, "Form List"),
    (86, "Perk"),
    (87, "Body Part Data"),
    (88, "AddOn Node"),
    (91, "Camera Shot"),
    (93, "Voice Type"),
    (94, "Impact Data"),
    (95, "Impact Data Set"),
    (96, "Armor Addon"),
    (97, "Encounter Zone"),
    (98, "Message"),
    (99, "Ragdoll"),
    (102, "Music Type"),
    (103, "Item Mod"),
    (104, "Reputation"),
    (106, "Recipe"),
    (107, "Recipe Category"),
    (108, "Casino Chip"),
    (109, "Casino"),
    (110, "Load Screen Type"),
    (111, "Media Set"),
    (112, "Media Location Controller"),
    (113, "Challenge"),
    (114, "Ammo Effect"),
    (115, "Caravan Card"),
    (116, "Caravan Money"),
    (117, "Caravan Deck"),
    (255, ""),
];

/// `kInventoryType`.
fn is_inventory_type(t: u8) -> bool {
    crate::args::is_inventory_type(t)
}

impl EngineEnv<'_> {
    /// The index of the first of `count` names (pointers from `table`, `stride` apart) equal to
    /// `text`, or `count`.
    fn name_index(&mut self, table: u32, stride: u32, count: u32, text: &[u8]) -> u32 {
        (0..count)
            .find(|&i| {
                let at = self.e.mem.u32(table + stride * i);
                str_equal_at(self.e, at, text)
            })
            .unwrap_or(count)
    }
}

impl Env for EngineEnv<'_> {
    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utilities.cpp: vShowRuntimeError
    // (with no script).
    fn error(&mut self, message: &[u8]) {
        let m = &message[..message.len().min(MESSAGE_BUFFER - 1)];
        crate::errors::show_bytes(self.e, 0, m);
    }

    fn block(&mut self, name: &[u8]) -> Option<BlockInfo> {
        (0..EVENT_BLOCK_COUNT)
            .map(|i| EVENT_BLOCKS + i * 0x28)
            .find(|&at| {
                let long = guest_text(self.e, self.e.mem.u32(at));
                long.eq_ignore_ascii_case(name)
            })
            .map(|at| {
                let r = Record::read(self.e, at);
                BlockInfo {
                    opcode: r.opcode as u16,
                    params: params_of(self.e, r.params, r.param_count),
                }
            })
    }

    fn plugin_listed(&mut self, name: &[u8]) -> bool {
        crate::plugins::listed(self.e, name).is_some()
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), CommandTable.cpp:
    // CommandTable::GetParentPlugin; Utils.cpp: IsDefaultParse.
    fn command(&mut self, name: &[u8]) -> Option<CmdInfo> {
        let opcode = compiler::command_by_name(self.e, name)?;
        let at = record_of(self.e, opcode);
        if at == 0 {
            return None;
        }
        let r = Record::read(self.e, at);
        let parse = if GAME_PARSERS.contains(&r.parse) || r.parse == table::DEFAULT_PARSE {
            Parse::Default
        } else if r.parse == table::CALL_PARSE {
            Parse::Call
        } else {
            Parse::Other
        };
        // xNVSE's own commands belong to its own plugin info ("NVSE", version 6); a built-in
        // plugin's to the plugin that registered from the greatest base at or below the opcode.
        let plugin = if r.opcode == 0 || r.opcode < NVSE_OPCODE_START {
            None
        } else if r.opcode < NVSE_OPCODE_TEST {
            Some((b"NVSE".to_vec(), crate::VERSION.0))
        } else {
            crate::plugins::built_ins(self.e)
                .into_iter()
                .filter(|p| p.opcode_base <= r.opcode)
                .max_by_key(|p| p.opcode_base)
                .map(|p| (p.name.as_bytes().to_vec(), p.version))
        };
        Some(CmdInfo {
            opcode: r.opcode as u16,
            long_name: guest_text(self.e, r.long_name),
            needs_parent: r.needs_ref != 0,
            params: params_of(self.e, r.params, r.param_count),
            parse,
            return_type: return_token_type(table::return_type(self.e, opcode)),
            plugin,
        })
    }

    fn form_by_name(&mut self, name: &[u8]) -> u32 {
        compiler::form_by_editor_id(self.e, name)
    }

    fn form_by_id(&mut self, id: u32) -> u32 {
        self.e.call(compiler::LOOKUP_FORM, &args![id]).u32()
    }

    fn form_type(&mut self, form: u32) -> u8 {
        self.e.mem.u8(form + 4)
    }

    fn script_form_type(&mut self, form: u32) -> Option<u16> {
        let script = dynamic_cast(self.e, form, RTTI_TES_FORM, rtti::SCRIPT);
        (script != 0).then(|| self.e.mem.u16(script + SCRIPT_TYPE))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), TypeChecker.cpp:
    // TypeChecker::VisitGetExpr (the script's variable by name).
    fn form_script_variable(&mut self, form: u32, name: &[u8]) -> Option<Option<(u8, u16)>> {
        let scriptable = match self.e.mem.u8(form + 4) {
            crate::newlang::passes::types::FORM_REFR => {
                let r = dynamic_cast(self.e, form, RTTI_TES_FORM, RTTI_TES_OBJECT_REFR);
                let base = crate::commands::base_form(self.e, r);
                dynamic_cast(self.e, base, RTTI_TES_FORM, rtti::TES_SCRIPTABLE_FORM)
            }
            _ => dynamic_cast(self.e, form, RTTI_TES_FORM, rtti::TES_SCRIPTABLE_FORM),
        };
        if scriptable == 0 {
            return None;
        }
        let script = self.e.mem.u32(scriptable + 4);
        if script == 0 {
            return None;
        }
        let info = compiler::variable_by_name(self.e, script + sc::VARS, name);
        Some((info != 0).then(|| (self.e.mem.u8(info + 0x10), self.e.mem.u32(info) as u16)))
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameForms.cpp: TESForm::GetEditorID.
    fn editor_id(&mut self, form: u32) -> Vec<u8> {
        let fixed: &[u8] = match self.e.mem.u32(form + 0xc) {
            0x7 => b"Player",
            0x14 => b"PlayerRef",
            0xa => b"LockPick",
            0x2d => b"MaleAdult01Default",
            0x2e => b"FemaleAdult01Default",
            0x33 => b"RadiationMarker",
            0x3d => b"DefaultCombatStyle",
            0x147 => b"PipBoyLight",
            0x163 => b"HelpManual",
            0x1f5 => b"DefaultWaterExplosion",
            0x1f6 => b"GasTrapDummy",
            _ => {
                let at = self.e.vcall(form, GET_NAME, &args![]).u32();
                return guest_text(self.e, at);
            }
        };
        fixed.to_vec()
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utils.cpp: ResolveVanillaEum.
    fn vanilla_enum(&mut self, kind: u32, text: &[u8]) -> (u32, u32) {
        let none = (u32::MAX, 2);
        match kind {
            // kParamType_ActorValue
            0x05 => {
                let i = call_with_text(self.e, ACTOR_VALUE_FOR_STRING, text);
                (if i < 77 { i } else { u32::MAX }, 2)
            }
            // kParamType_Axis
            0x08 => {
                if text.len() == 1 {
                    let c = text[0] & 0xdf;
                    if (b'X'..=b'Z').contains(&c) {
                        return (u32::from(c), 1);
                    }
                }
                none
            }
            // kParamType_AnimationGroup
            0x0a => {
                let i = self.name_index(ANIM_GROUPS, 0x24, 245, text);
                (if i < 245 { i } else { u32::MAX }, 2)
            }
            // kParamType_Sex
            0x12 => {
                let mut v = u32::MAX;
                let male = self.e.mem.u32(SEX_0);
                if str_equal_at(self.e, male, text) {
                    v = 0;
                }
                let female = self.e.mem.u32(SEX_1);
                if str_equal_at(self.e, female, text) {
                    v = 1;
                }
                (v, 2)
            }
            // kParamType_CrimeType
            0x1c => {
                if call_with_text(self.e, IS_STRING_INTEGER, text) & 0xff != 0 {
                    let i = atoi(text) as u32;
                    if i <= 4 {
                        return (i, 2);
                    }
                }
                none
            }
            // kParamType_FormType
            0x20 => {
                let mut v = u32::MAX;
                for (id, name) in FORM_TYPE_NAMES.iter().filter(|(_, n)| !n.is_empty()) {
                    let upper = crate::vars::upper_table(self.e);
                    if crate::vars::str_compare(&upper, name.as_bytes(), text)
                        == std::cmp::Ordering::Equal
                    {
                        v = u32::from(*id);
                    }
                }
                (v, 2)
            }
            // kParamType_MiscellaneousStat
            0x29 => {
                let i = call_with_text(self.e, MISC_STAT_FOR_STRING, text);
                (if i < 43 { i } else { u32::MAX }, 2)
            }
            // kParamType_Alignment
            0x33 => {
                let i = self.name_index(ALIGNMENT_NAMES, 4, 5, text);
                (if i < 5 { i } else { u32::MAX }, 2)
            }
            // kParamType_EquipType
            0x34 => {
                let i = self.name_index(EQUIP_TYPE_NAMES, 4, 14, text);
                (if i < 14 { i } else { u32::MAX }, 2)
            }
            // kParamType_CriticalStage
            0x37 => {
                let i = self.name_index(CRITICAL_STAGE_NAMES, 4, 5, text);
                (if i == 5 { u32::MAX } else { i }, 2)
            }
            _ => none,
        }
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), Utils.cpp: DoesFormTypeMatchParamType
    // (the run-time build's checks).
    fn form_matches_param(&mut self, form: u32, kind: u32) -> bool {
        use crate::args::form_type::*;
        if form == 0 {
            return false;
        }
        let t = self.e.mem.u8(form + 4);
        let is = |x: u8| t == x;
        let e = &mut *self.e;
        let cast = |e: &mut Engine, to: u32| dynamic_cast(e, form, RTTI_TES_FORM, to) != 0;
        match kind {
            0x03 => is_inventory_type(t),
            0x04 => cast(e, RTTI_TES_OBJECT_REFR),
            0x06 => e.vcall(form, crate::vtable::IS_ACTOR, &args![]).bool(),
            0x18 => {
                is(TES_OBJECT_REFR) && crate::commands::base_form(e, form) == e.mem.u32(MAP_MARKER)
            }
            0x1a => cast(e, RTTI_TES_OBJECT_REFR) && e.call(GET_CONTAINER, &args![form]).u32() != 0,
            0x07 => is(SPELL_ITEM) || is(TES_OBJECT_BOOK),
            0x09 => is(TES_OBJECT_CELL),
            0x0b => cast(e, rtti::MAGIC_ITEM),
            0x0c => is(TES_SOUND),
            0x0d => is(TES_TOPIC),
            0x0e => is(TES_QUEST),
            0x0f => is(TES_RACE),
            0x10 => is(TES_CLASS),
            0x11 => is(TES_FACTION),
            0x13 => is(TES_GLOBAL),
            0x14 => is(TES_FURNITURE) || is(BGS_LIST_FORM),
            0x15 => cast(e, RTTI_TES_OBJECT),
            0x19 => is(TES_NPC) || is(TES_CREATURE),
            0x1b => is(TES_WORLD_SPACE),
            0x1d => is(TES_PACKAGE),
            0x1e => is(TES_COMBAT_STYLE),
            0x1f => is(EFFECT_SETTING),
            0x21 => is(TES_WEATHER),
            0x22 => is(TES_NPC),
            0x23 => is(TES_FACTION) || is(TES_NPC),
            0x24 => is(TES_EFFECT_SHADER),
            0x25 => is(BGS_LIST_FORM),
            0x26 => is(BGS_MENU_ICON),
            0x27 => is(BGS_PERK),
            0x28 => is(BGS_NOTE),
            0x2a => is(TES_IMAGE_SPACE_MODIFIER),
            0x2b => is(TES_IMAGE_SPACE),
            0x2f => is(BGS_ENCOUNTER_ZONE),
            0x30 => is(TES_IDLE_FORM),
            0x31 => is(BGS_MESSAGE),
            0x32 => is(BGS_LIST_FORM) || is_inventory_type(t),
            // The run-time build asks for a form list here, where the editor's refuses one.
            0x35 => is(BGS_LIST_FORM),
            0x36 => is(BGS_MUSIC_TYPE),
            0x38 => is(TES_NPC) || is(TES_LEV_CHARACTER),
            0x39 => is(TES_CREATURE) || is(TES_LEV_CREATURE),
            0x3a => is(TES_LEV_CHARACTER),
            0x3b => is(TES_LEV_CREATURE),
            0x3c => is(TES_LEV_ITEM),
            0x3e => is(TES_REPUTATION),
            0x3f => is(TES_CASINO),
            0x40 => is(TES_CASINO_CHIPS),
            0x41 => is(TES_CHALLENGE),
            0x42 => is(TES_CARAVAN_MONEY),
            0x43 => is(TES_CARAVAN_CARD),
            0x44 => is(TES_CARAVAN_DECK),
            0x45 => is(TES_REGION),
            _ => true,
        }
    }

    fn script_variables(&mut self) -> Option<Vec<(Vec<u8>, u8)>> {
        if self.script == 0 {
            return None;
        }
        let infos = compiler::list_items(self.e, self.script + sc::VARS);
        Some(
            infos
                .into_iter()
                .map(|info| {
                    (
                        compiler::string_cstr(self.e, info + 0x18),
                        self.e.mem.u8(info + 0x10),
                    )
                })
                .collect(),
        )
    }

    // Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), VariableResolution.cpp:
    // VariableResolution::Visit (the engine var list made again).
    /// The variable and reference lists emptied (`DeleteAll`: their items and extra nodes
    /// freed), then a `VariableInfo` (0x20 bytes) per variable appended, and a `RefVariable`
    /// (0x10 bytes) per reference variable; the counts set.
    fn set_script_variables(&mut self, vars: &[(Vec<u8>, u8, u32)]) {
        if self.script == 0 {
            return;
        }
        let e = &mut *self.e;
        let (var_head, ref_head) = (self.script + sc::VARS, self.script + sc::REFS);
        delete_all(e, var_head, true);
        delete_all(e, ref_head, false);
        for (name, ty, index) in vars {
            let info = e.call(compiler::FORM_HEAP_ALLOCATE, &args![0x20u32]).u32();
            e.mem.write(info, &[0; 0x20]);
            e.mem.set_u32(info, *index);
            e.mem.set_u8(info + 0x10, *ty);
            compiler::string_set(e, info + 0x18, name);
            compiler::list_append(e, var_head, info);
            if *ty == compiler::vt::REF {
                let r = e.call(compiler::FORM_HEAP_ALLOCATE, &args![0x10u32]).u32();
                e.mem.write(r, &[0; 0x10]);
                compiler::string_set(e, r, name);
                e.mem.set_u32(r + 0xc, *index);
                compiler::list_append(e, ref_head, r);
            }
        }
        let count = compiler::list_items(e, var_head).len() as u32;
        let refs = compiler::list_items(e, ref_head).len() as u32;
        e.mem.set_u32(self.script + sc::VAR_COUNT, count);
        e.mem.set_u32(self.script + sc::NUM_REFS, refs);
    }

    fn script_type(&mut self) -> Option<u16> {
        (self.script != 0).then(|| self.e.mem.u16(self.script + SCRIPT_TYPE))
    }
}

/// `Script::info.type`.
const SCRIPT_TYPE: u32 = 0x28;

// Translated from xNVSE 6.4.9 (xNVSE/NVSE at 0ccd23a), GameTypes.h: tList::DeleteAll.
/// `tList::DeleteAll`: the head's item freed (its destructor not run), then each later item's
/// destructor run (its name `String` freed), the item and its node freed; the head emptied.
fn delete_all(e: &mut Engine, head: u32, variables: bool) {
    let name_at = if variables { 0x18 } else { 0 };
    let first = e.mem.u32(head);
    let mut node = e.mem.u32(head + 4);
    e.call(compiler::FORM_HEAP_FREE, &args![first]);
    e.mem.set_u32(head, 0);
    e.mem.set_u32(head + 4, 0);
    for _ in 0..0x10_0000 {
        if node == 0 {
            break;
        }
        let next = e.mem.u32(node + 4);
        let item = e.mem.u32(node);
        let text = e.mem.u32(item + name_at);
        if text != 0 {
            e.call(compiler::FORM_HEAP_FREE, &args![text]);
        }
        e.call(compiler::FORM_HEAP_FREE, &args![item]);
        e.call(compiler::FORM_HEAP_FREE, &args![node]);
        node = next;
    }
}

#[cfg(test)]
#[path = "engine_env_tests.rs"]
mod tests;
