//! JIP LN NVSE's 1,150 command slots (JIP LN NVSE), as its 57.30 source (`5a30ac4`) registers
//! them: `jip_nvse.cpp`'s `NVSEPlugin_Load` (the opcode bases `0x2200` and `0x2600`, a `REG_CMD`
//! line or `REG_EMPTY` per slot, the return type of each), each command's `DEFINE_*` line (its
//! alias, whether it needs a reference, its parameter list, whether it has a condition eval or
//! xNVSE's expression parser) and the parameter lists of `nvse/ParamInfos.h`. Data, not code:
//! generated from the source by a script kept beside the lead's tools (`measure/jip/gen-slots.pl`)
//! and left as generated (`#[rustfmt::skip]` on the module); the tests in `table.rs` pin it to
//! the source's list (`data/source-57.30.tsv`) and to what the shipped `jip_nvse.dll` 57.30
//! registers (`data/dll-57.30.tsv`). Do not edit by hand.

use crate::table::Kind::{Command, Condition, ConditionOnly, Expression};
use crate::table::{cmd, empty, Params, Slot, AMB, ARR, FORM, NUM, STR};
use nvse::table::Param;

/// The parameter types the lists use (`ParamType` (JIP LN NVSE), `nvse/CommandTable.h`).
mod ty {
    pub const STRING: u32 = 0x00;
    pub const INTEGER: u32 = 0x01;
    pub const FLOAT: u32 = 0x02;
    pub const OBJECT_ID: u32 = 0x03;
    pub const OBJECT_REF: u32 = 0x04;
    pub const ACTOR_VALUE: u32 = 0x05;
    pub const ACTOR: u32 = 0x06;
    pub const SPELL_ITEM: u32 = 0x07;
    pub const AXIS: u32 = 0x08;
    pub const ANIMATION_GROUP: u32 = 0x0a;
    pub const MAGIC_ITEM: u32 = 0x0b;
    pub const SOUND: u32 = 0x0c;
    pub const TOPIC: u32 = 0x0d;
    pub const QUEST: u32 = 0x0e;
    pub const CLASS: u32 = 0x10;
    pub const FACTION: u32 = 0x11;
    pub const GLOBAL: u32 = 0x13;
    pub const BOUND_OBJECT: u32 = 0x15;
    pub const ACTOR_BASE: u32 = 0x19;
    pub const CONTAINER: u32 = 0x1a;
    pub const WORLD_SPACE: u32 = 0x1b;
    pub const AI_PACKAGE: u32 = 0x1d;
    pub const COMBAT_STYLE: u32 = 0x1e;
    pub const MAGIC_EFFECT: u32 = 0x1f;
    pub const WEATHER_ID: u32 = 0x21;
    pub const FORM_LIST: u32 = 0x25;
    pub const PERK: u32 = 0x27;
    pub const NOTE: u32 = 0x28;
    pub const IMAGE_SPACE_MODIFIER: u32 = 0x2a;
    pub const IMAGE_SPACE: u32 = 0x2b;
    pub const DOUBLE: u32 = 0x2c;
    pub const SCRIPT_VARIABLE: u32 = 0x2d;
    pub const IDLE_FORM: u32 = 0x30;
    pub const MESSAGE: u32 = 0x31;
    pub const INV_OBJ_OR_FORM_LIST: u32 = 0x32;
    pub const ANY_FORM: u32 = 0x3d;
    pub const REPUTATION: u32 = 0x3e;
    pub const CASINO: u32 = 0x3f;
    pub const CHALLENGE: u32 = 0x41;
    pub const NVSE_NUMBER: u32 = 0x8001;
    pub const NVSE_BASIC_TYPE: u32 = 0x804d;
}

/// A parameter JIP lists as needed: no name (`ParamInfo`'s one- and two-argument constructors
/// leave `typeStr` empty) and its type.
const fn req(kind: u32) -> Param {
    Param { name: "", kind, optional: false }
}

/// A parameter JIP lists as optional.
const fn opt(kind: u32) -> Param {
    Param { name: "", kind, optional: true }
}

/// The parameter lists the slots use, by their names in `nvse/ParamInfos.h`; `nullptr` is no list
/// and no count.
pub static NO_PARAMS: Params = Params::new("nullptr", &[]);
pub static ONE_INT: Params = Params::new("kParams_OneInt", &[req(ty::INTEGER)]);
pub static TWO_INTS: Params = Params::new("kParams_TwoInts", &[req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_OPTIONAL_INT: Params = Params::new("kParams_OneOptionalInt", &[opt(ty::INTEGER)]);
pub static ONE_INT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneInt_OneOptionalInt", &[req(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_FLOAT: Params = Params::new("kParams_OneFloat", &[req(ty::FLOAT)]);
pub static ONE_STRING: Params = Params::new("kParams_OneString", &[req(ty::STRING)]);
pub static ONE_STRING_ONE_FLOAT: Params = Params::new("kParams_OneString_OneFloat", &[req(ty::STRING), req(ty::FLOAT)]);
pub static TWO_FLOATS: Params = Params::new("kParams_TwoFloats", &[req(ty::FLOAT), req(ty::FLOAT)]);
pub static ONE_OBJECT_ID: Params = Params::new("kParams_OneObjectID", &[req(ty::OBJECT_ID)]);
pub static ONE_OPTIONAL_OBJECT_ID: Params = Params::new("kParams_OneOptionalObjectID", &[opt(ty::OBJECT_ID)]);
pub static ONE_OBJECT_ID_ONE_INT: Params = Params::new("kParams_OneObjectID_OneInt", &[req(ty::OBJECT_ID), req(ty::INTEGER)]);
pub static FORMAT_STRING: Params = Params::new("kParams_FormatString", &[req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_ACTOR: Params = Params::new("kParams_OneActor", &[req(ty::ACTOR)]);
pub static ONE_AXIS: Params = Params::new("kParams_OneAxis", &[req(ty::AXIS)]);
pub static FORM_LIST: Params = Params::new("kParams_FormList", &[req(ty::FORM_LIST)]);
pub static TWO_STRINGS: Params = Params::new("kParams_TwoStrings", &[req(ty::STRING), req(ty::STRING)]);
pub static ONE_BOUND_OBJECT: Params = Params::new("kParams_OneBoundObject", &[req(ty::BOUND_OBJECT)]);
pub static ONE_OPTIONAL_BOUND_OBJECT: Params = Params::new("kParams_OneOptionalBoundObject", &[opt(ty::BOUND_OBJECT)]);
pub static ONE_BOUND_OBJECT_ONE_OPTIONAL_BOUND_OBJECT: Params = Params::new("kParams_OneBoundObject_OneOptionalBoundObject", &[req(ty::BOUND_OBJECT), opt(ty::BOUND_OBJECT)]);
pub static ONE_BOUND_OBJECT_ONE_INT: Params = Params::new("kParams_OneBoundObject_OneInt", &[req(ty::BOUND_OBJECT), req(ty::INTEGER)]);
pub static ONE_INT_ONE_OPTIONAL_BOUND_OBJECT: Params = Params::new("kParams_OneInt_OneOptionalBoundObject", &[req(ty::INTEGER), opt(ty::BOUND_OBJECT)]);
pub static ONE_QUEST: Params = Params::new("kParams_OneQuest", &[req(ty::QUEST)]);
pub static ONE_OPTIONAL_OBJECT_REF: Params = Params::new("kParams_OneOptionalObjectRef", &[opt(ty::OBJECT_REF)]);
pub static ONE_FORM_ONE_INT: Params = Params::new("kParams_OneForm_OneInt", &[req(ty::ANY_FORM), req(ty::INTEGER)]);
pub static ONE_FORM: Params = Params::new("kParams_OneForm", &[req(ty::ANY_FORM)]);
pub static ONE_FORM_ONE_FLOAT: Params = Params::new("kParams_OneForm_OneFloat", &[req(ty::ANY_FORM), req(ty::FLOAT)]);
pub static ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneOptionalForm", &[opt(ty::ANY_FORM)]);
pub static ONE_OBJECT_ID_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneObjectID_TwoOptionalInts", &[req(ty::OBJECT_ID), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneOptionalActorBase", &[opt(ty::ACTOR_BASE)]);
pub static ONE_INT_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneInt_OneOptionalActorBase", &[req(ty::INTEGER), opt(ty::ACTOR_BASE)]);
pub static ONE_ITEM_OR_LIST_ONE_OPT_INT_ONE_OPT_FLOAT_TWO_OPT_INTS: Params = Params::new("kParams_OneItemOrList_OneOptInt_OneOptFloat_TwoOptInts", &[req(ty::INV_OBJ_OR_FORM_LIST), opt(ty::INTEGER), opt(ty::FLOAT), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_STRING_ONE_FORMAT_STRING: Params = Params::new("kParams_OneString_OneFormatString", &[req(ty::STRING), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_FORM_ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneForm_OneInt_OneFloat", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_FORM_TWO_INTS: Params = Params::new("kParams_OneForm_TwoInts", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::INTEGER)]);
pub static TWO_FORMS: Params = Params::new("kParams_TwoForms", &[req(ty::ANY_FORM), req(ty::ANY_FORM)]);
pub static ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneForm_OneInt_OneOptionalForm", &[req(ty::ANY_FORM), req(ty::INTEGER), opt(ty::ANY_FORM)]);
pub static ONE_STRING_ONE_OPTIONAL_QUEST: Params = Params::new("kParams_OneString_OneOptionalQuest", &[req(ty::STRING), opt(ty::QUEST)]);
pub static ONE_QUEST_ONE_FLOAT: Params = Params::new("kParams_OneQuest_OneFloat", &[req(ty::QUEST), req(ty::FLOAT)]);
pub static ONE_AI_PACKAGE_ONE_INT: Params = Params::new("kParams_OneAIPackage_OneInt", &[req(ty::AI_PACKAGE), req(ty::INTEGER)]);
pub static ONE_AI_PACKAGE_TWO_INTS: Params = Params::new("kParams_OneAIPackage_TwoInts", &[req(ty::AI_PACKAGE), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_GLOBAL: Params = Params::new("kParams_OneGlobal", &[req(ty::GLOBAL)]);
pub static ONE_GLOBAL_ONE_FLOAT: Params = Params::new("kParams_OneGlobal_OneFloat", &[req(ty::GLOBAL), req(ty::FLOAT)]);
pub static ONE_FACTION: Params = Params::new("kParams_OneFaction", &[req(ty::FACTION)]);
pub static ONE_FACTION_ONE_OPTIONAL_REPUTATION: Params = Params::new("kParams_OneFaction_OneOptionalReputation", &[req(ty::FACTION), opt(ty::REPUTATION)]);
pub static ONE_AXIS_ONE_OPTIONAL_BOUND_OBJECT: Params = Params::new("kParams_OneAxis_OneOptionalBoundObject", &[req(ty::AXIS), opt(ty::BOUND_OBJECT)]);
pub static TWO_DOUBLES_THREE_OPTIONAL_DOUBLES: Params = Params::new("kParams_TwoDoubles_ThreeOptionalDoubles", &[req(ty::DOUBLE), req(ty::DOUBLE), opt(ty::DOUBLE), opt(ty::DOUBLE), opt(ty::DOUBLE)]);
pub static ONE_COMBAT_STYLE_ONE_INT: Params = Params::new("kParams_OneCombatStyle_OneInt", &[req(ty::COMBAT_STYLE), req(ty::INTEGER)]);
pub static ONE_COMBAT_STYLE_ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneCombatStyle_OneInt_OneFloat", &[req(ty::COMBAT_STYLE), req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_COMBAT_STYLE_TWO_INTS: Params = Params::new("kParams_OneCombatStyle_TwoInts", &[req(ty::COMBAT_STYLE), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_STRING_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneString_TwoOptionalInts", &[req(ty::STRING), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_STRING_ONE_INT_ONE_FORMAT_STRING: Params = Params::new("kParams_OneString_OneInt_OneFormatString", &[req(ty::STRING), req(ty::INTEGER), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_CLASS_ONE_INT: Params = Params::new("kParams_OneClass_OneInt", &[req(ty::CLASS), req(ty::INTEGER)]);
pub static ONE_CLASS_TWO_INTS: Params = Params::new("kParams_OneClass_TwoInts", &[req(ty::CLASS), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_SOUND_ONE_INT: Params = Params::new("kParams_OneSound_OneInt", &[req(ty::SOUND), req(ty::INTEGER)]);
pub static ONE_SOUND_ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneSound_OneInt_OneFloat", &[req(ty::SOUND), req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_SOUND_TWO_INTS: Params = Params::new("kParams_OneSound_TwoInts", &[req(ty::SOUND), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_FORM_ONE_INT_ONE_OPTIONAL_SOUND: Params = Params::new("kParams_OneForm_OneInt_OneOptionalSound", &[req(ty::ANY_FORM), req(ty::INTEGER), opt(ty::SOUND)]);
pub static ONE_FORM_ONE_INT_ONE_OPTIONAL_OBJECT_ID: Params = Params::new("kParams_OneForm_OneInt_OneOptionalObjectID", &[req(ty::ANY_FORM), req(ty::INTEGER), opt(ty::OBJECT_ID)]);
pub static ONE_WEATHER_ID_ONE_INT: Params = Params::new("kParams_OneWeatherID_OneInt", &[req(ty::WEATHER_ID), req(ty::INTEGER)]);
pub static ONE_WEATHER_ID_ONE_INT_ONE_OPTIONAL_IMAGE_SPACE_MOD: Params = Params::new("kParams_OneWeatherID_OneInt_OneOptionalImageSpaceMod", &[req(ty::WEATHER_ID), req(ty::INTEGER), opt(ty::IMAGE_SPACE_MODIFIER)]);
pub static ONE_WEATHER_ID_ONE_INT_ONE_STRING: Params = Params::new("kParams_OneWeatherID_OneInt_OneString", &[req(ty::WEATHER_ID), req(ty::INTEGER), req(ty::STRING)]);
pub static ONE_WEATHER_ID: Params = Params::new("kParams_OneWeatherID", &[req(ty::WEATHER_ID)]);
pub static ONE_WEATHER_ID_ONE_STRING: Params = Params::new("kParams_OneWeatherID_OneString", &[req(ty::WEATHER_ID), req(ty::STRING)]);
pub static ONE_WEATHER_ID_ONE_INT_ONE_DOUBLE: Params = Params::new("kParams_OneWeatherID_OneInt_OneDouble", &[req(ty::WEATHER_ID), req(ty::INTEGER), req(ty::DOUBLE)]);
pub static ONE_WEATHER_ID_TWO_INTS_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneWeatherID_TwoInts_OneOptionalInt", &[req(ty::WEATHER_ID), req(ty::INTEGER), req(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_WEATHER_ID_THREE_INTS_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneWeatherID_ThreeInts_OneOptionalInt", &[req(ty::WEATHER_ID), req(ty::INTEGER), req(ty::INTEGER), req(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_FORM_ONE_INT_ONE_STRING: Params = Params::new("kParams_OneForm_OneInt_OneString", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::STRING)]);
pub static ONE_FORM_ONE_STRING: Params = Params::new("kParams_OneForm_OneString", &[req(ty::ANY_FORM), req(ty::STRING)]);
pub static ONE_FORM_ONE_WEATHER_ID_ONE_INT_ONE_OPTIONAL_GLOBAL: Params = Params::new("kParams_OneForm_OneWeatherID_OneInt_OneOptionalGlobal", &[req(ty::ANY_FORM), req(ty::WEATHER_ID), req(ty::INTEGER), opt(ty::GLOBAL)]);
pub static ONE_FORM_ONE_WEATHER_ID: Params = Params::new("kParams_OneForm_OneWeatherID", &[req(ty::ANY_FORM), req(ty::WEATHER_ID)]);
pub static ONE_AXIS_ONE_FLOAT: Params = Params::new("kParams_OneAxis_OneFloat", &[req(ty::AXIS), req(ty::FLOAT)]);
pub static ONE_FORM_THREE_INTS: Params = Params::new("kParams_OneForm_ThreeInts", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_ITEM_OR_LIST_ONE_CONTAINER_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneItemOrList_OneContainer_TwoOptionalInts", &[req(ty::INV_OBJ_OR_FORM_LIST), req(ty::CONTAINER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_INT_ONE_FLOAT_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneInt_OneFloat_OneOptionalActorBase", &[req(ty::INTEGER), req(ty::FLOAT), opt(ty::ACTOR_BASE)]);
pub static ONE_MAGIC_ITEM_ONE_INT: Params = Params::new("kParams_OneMagicItem_OneInt", &[req(ty::MAGIC_ITEM), req(ty::INTEGER)]);
pub static ONE_MAGIC_ITEM_TWO_INTS: Params = Params::new("kParams_OneMagicItem_TwoInts", &[req(ty::MAGIC_ITEM), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_MAGIC_ITEM_THREE_INTS: Params = Params::new("kParams_OneMagicItem_ThreeInts", &[req(ty::MAGIC_ITEM), req(ty::INTEGER), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_MAGIC_ITEM_ONE_INT_ONE_MAGIC_EFFECT: Params = Params::new("kParams_OneMagicItem_OneInt_OneMagicEffect", &[req(ty::MAGIC_ITEM), req(ty::INTEGER), req(ty::MAGIC_EFFECT)]);
pub static ONE_OPTIONAL_QUEST: Params = Params::new("kParams_OneOptionalQuest", &[opt(ty::QUEST)]);
pub static ONE_STRING_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneString_OneOptionalForm", &[req(ty::STRING), opt(ty::ANY_FORM)]);
pub static ONE_STRING_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneString_OneOptionalInt_OneOptionalForm", &[req(ty::STRING), opt(ty::INTEGER), opt(ty::ANY_FORM)]);
pub static ONE_STRING_ONE_DOUBLE_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneString_OneDouble_OneOptionalInt_OneOptionalForm", &[req(ty::STRING), req(ty::DOUBLE), opt(ty::INTEGER), opt(ty::ANY_FORM)]);
pub static ONE_STRING_ONE_FORM_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneString_OneForm_OneOptionalInt_OneOptionalForm", &[req(ty::STRING), req(ty::ANY_FORM), opt(ty::INTEGER), opt(ty::ANY_FORM)]);
pub static TWO_STRINGS_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_TwoStrings_OneOptionalInt_OneOptionalForm", &[req(ty::STRING), req(ty::STRING), opt(ty::INTEGER), opt(ty::ANY_FORM)]);
pub static ONE_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneInt_OneOptionalForm", &[req(ty::INTEGER), opt(ty::ANY_FORM)]);
pub static ONE_STRING_ONE_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneString_OneInt_OneOptionalForm", &[req(ty::STRING), req(ty::INTEGER), opt(ty::ANY_FORM)]);
pub static ONE_STRING_ONE_DOUBLE_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneString_OneDouble_OneOptionalForm", &[req(ty::STRING), req(ty::DOUBLE), opt(ty::ANY_FORM)]);
pub static ONE_STRING_ONE_FORM_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneString_OneForm_OneOptionalForm", &[req(ty::STRING), req(ty::ANY_FORM), opt(ty::ANY_FORM)]);
pub static TWO_STRINGS_ONE_OPTIONAL_FORM: Params = Params::new("kParams_TwoStrings_OneOptionalForm", &[req(ty::STRING), req(ty::STRING), opt(ty::ANY_FORM)]);
pub static FOUR_INTS: Params = Params::new("kParams_FourInts", &[req(ty::INTEGER), req(ty::INTEGER), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_FORM_TWO_INTS_ONE_FLOAT: Params = Params::new("kParams_OneForm_TwoInts_OneFloat", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_FORM_TWO_INTS_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneForm_TwoInts_OneOptionalForm", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::INTEGER), opt(ty::ANY_FORM)]);
pub static ONE_FORM_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneForm_OneOptionalForm", &[req(ty::ANY_FORM), opt(ty::ANY_FORM)]);
pub static ONE_SOUND: Params = Params::new("kParams_OneSound", &[req(ty::SOUND)]);
pub static ONE_SOUND_ONE_STRING: Params = Params::new("kParams_OneSound_OneString", &[req(ty::SOUND), req(ty::STRING)]);
pub static ONE_WORLDSPACE: Params = Params::new("kParams_OneWorldspace", &[req(ty::WORLD_SPACE)]);
pub static ONE_WORLDSPACE_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneWorldspace_OneOptionalForm", &[req(ty::WORLD_SPACE), opt(ty::ANY_FORM)]);
pub static ONE_FORM_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneForm_OneOptionalActorBase", &[req(ty::ANY_FORM), opt(ty::ACTOR_BASE)]);
pub static ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneInt_OneFloat", &[req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_INT_ONE_FORMAT_STRING: Params = Params::new("kParams_OneInt_OneFormatString", &[req(ty::INTEGER), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static TWO_INTS_ONE_OPTIONAL_INT: Params = Params::new("kParams_TwoInts_OneOptionalInt", &[req(ty::INTEGER), req(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_OBJECT_REF_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneObjectRef_OneOptionalInt", &[req(ty::OBJECT_REF), opt(ty::INTEGER)]);
pub static ONE_LIST_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneList_TwoOptionalInts", &[req(ty::FORM_LIST), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_FACTION_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneFaction_OneOptionalInt", &[req(ty::FACTION), opt(ty::INTEGER)]);
pub static ONE_QUEST_ONE_INT: Params = Params::new("kParams_OneQuest_OneInt", &[req(ty::QUEST), req(ty::INTEGER)]);
pub static ONE_WORLDSPACE_ONE_INT: Params = Params::new("kParams_OneWorldspace_OneInt", &[req(ty::WORLD_SPACE), req(ty::INTEGER)]);
pub static ONE_WORLDSPACE_TWO_INTS: Params = Params::new("kParams_OneWorldspace_TwoInts", &[req(ty::WORLD_SPACE), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_STRING_ONE_INT_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneString_OneInt_OneOptionalActorBase", &[req(ty::STRING), req(ty::INTEGER), opt(ty::ACTOR_BASE)]);
pub static ONE_STRING_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneString_OneOptionalActorBase", &[req(ty::STRING), opt(ty::ACTOR_BASE)]);
pub static ONE_OBJECT_REF: Params = Params::new("kParams_OneObjectRef", &[req(ty::OBJECT_REF)]);
pub static ONE_INT_THREE_FLOATS: Params = Params::new("kParams_OneInt_ThreeFloats", &[req(ty::INTEGER), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT)]);
pub static ONE_MAGIC_ITEM_ONE_EFFECT_TWO_INTS_THREE_OPTIONAL_INTS: Params = Params::new("kParams_OneMagicItem_OneEffect_TwoInts_ThreeOptionalInts", &[req(ty::MAGIC_ITEM), req(ty::MAGIC_EFFECT), req(ty::INTEGER), req(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_ITEM_OR_LIST_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneItemOrList_TwoOptionalInts", &[req(ty::INV_OBJ_OR_FORM_LIST), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static TWO_OPTIONAL_INTS: Params = Params::new("kParams_TwoOptionalInts", &[opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_ACTOR_VALUE_ONE_FLOAT_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneActorValue_OneFloat_OneOptionalActorBase", &[req(ty::ACTOR_VALUE), req(ty::FLOAT), opt(ty::ACTOR_BASE)]);
pub static ONE_FLOAT_ONE_FORMAT_STRING: Params = Params::new("kParams_OneFloat_OneFormatString", &[req(ty::FLOAT), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_FORM_ONE_FORMAT_STRING: Params = Params::new("kParams_OneForm_OneFormatString", &[req(ty::ANY_FORM), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_INT_ONE_STRING: Params = Params::new("kParams_OneInt_OneString", &[req(ty::INTEGER), req(ty::STRING)]);
pub static ONE_FORM_TWO_FLOATS_ONE_FORMAT_STRING: Params = Params::new("kParams_OneForm_TwoFloats_OneFormatString", &[req(ty::ANY_FORM), req(ty::FLOAT), req(ty::FLOAT), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_ACTOR_ONE_TOPIC_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneActor_OneTopic_TwoOptionalInts", &[req(ty::ACTOR), req(ty::TOPIC), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_TOPIC: Params = Params::new("kParams_OneTopic", &[req(ty::TOPIC)]);
pub static ONE_FORM_THREE_FLOATS: Params = Params::new("kParams_OneForm_ThreeFloats", &[req(ty::ANY_FORM), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT)]);
pub static FOUR_OPTIONAL_INTS_ONE_OPTIONAL_LIST: Params = Params::new("kParams_FourOptionalInts_OneOptionalList", &[opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::FORM_LIST)]);
pub static ONE_INT_ONE_OPTIONAL_ACTOR: Params = Params::new("kParams_OneInt_OneOptionalActor", &[req(ty::INTEGER), opt(ty::ACTOR)]);
pub static ONE_OBJECT_ID_ONE_ACTOR_VALUE: Params = Params::new("kParams_OneObjectID_OneActorValue", &[req(ty::OBJECT_ID), req(ty::ACTOR_VALUE)]);
pub static ONE_MESSAGE: Params = Params::new("kParams_OneMessage", &[req(ty::MESSAGE)]);
pub static ONE_MESSAGE_ONE_INT: Params = Params::new("kParams_OneMessage_OneInt", &[req(ty::MESSAGE), req(ty::INTEGER)]);
pub static ONE_SOUND_ONE_OPTIONAL_OBJECT_REF: Params = Params::new("kParams_OneSound_OneOptionalObjectRef", &[req(ty::SOUND), opt(ty::OBJECT_REF)]);
pub static ONE_OBJECT_REF_TWO_STRINGS_ONE_OTIONAL_INT: Params = Params::new("kParams_OneObjectRef_TwoStrings_OneOtionalInt", &[req(ty::OBJECT_REF), req(ty::STRING), req(ty::STRING), opt(ty::INTEGER)]);
pub static ONE_STRING_ONE_INT: Params = Params::new("kParams_OneString_OneInt", &[req(ty::STRING), req(ty::INTEGER)]);
pub static ONE_STRING_ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneString_OneInt_OneFloat", &[req(ty::STRING), req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_FORM_ONE_INT_ONE_FORM_ONE_INT: Params = Params::new("kParams_OneForm_OneInt_OneForm_OneInt", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::ANY_FORM), req(ty::INTEGER)]);
pub static ONE_FORM_ONE_INT_ONE_FORM_ONE_ANIM_GROUP: Params = Params::new("kParams_OneForm_OneInt_OneForm_OneAnimGroup", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::ANY_FORM), req(ty::ANIMATION_GROUP)]);
pub static ONE_FORM_ONE_INT_ONE_FORM: Params = Params::new("kParams_OneForm_OneInt_OneForm", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::ANY_FORM)]);
pub static ONE_ACTOR_VALUE_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneActorValue_OneOptionalActorBase", &[req(ty::ACTOR_VALUE), opt(ty::ACTOR_BASE)]);
pub static THREE_FLOATS: Params = Params::new("kParams_ThreeFloats", &[req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT)]);
pub static ONE_OPTIONAL_AXIS: Params = Params::new("kParams_OneOptionalAxis", &[opt(ty::AXIS)]);
pub static ONE_PERK_ONE_INT: Params = Params::new("kParams_OnePerk_OneInt", &[req(ty::PERK), req(ty::INTEGER)]);
pub static ONE_PERK_TWO_INTS: Params = Params::new("kParams_OnePerk_TwoInts", &[req(ty::PERK), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_LIST_ONE_INT_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneList_OneInt_TwoOptionalInts", &[req(ty::FORM_LIST), req(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_STRING_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_OneOptionalInt", &[req(ty::STRING), opt(ty::INTEGER)]);
pub static THREE_FORMS: Params = Params::new("kParams_ThreeForms", &[req(ty::ANY_FORM), req(ty::ANY_FORM), req(ty::ANY_FORM)]);
pub static TWO_FORMS_TWO_INTS_ONE_FLOAT: Params = Params::new("kParams_TwoForms_TwoInts_OneFloat", &[req(ty::ANY_FORM), req(ty::ANY_FORM), req(ty::INTEGER), req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_FORM_ONE_INT_ONE_FLOAT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneForm_OneInt_OneFloat_OneOptionalForm", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::FLOAT), opt(ty::ANY_FORM)]);
pub static ONE_INT_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneInt_TwoOptionalInts", &[req(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_OBJECT_ID_ONE_OPTIONAL_FLOAT_THREE_OPTIONAL_INTS: Params = Params::new("kParams_OneObjectID_OneOptionalFloat_ThreeOptionalInts", &[req(ty::OBJECT_ID), opt(ty::FLOAT), opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_FLOAT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneFloat_OneOptionalForm", &[req(ty::FLOAT), opt(ty::ANY_FORM)]);
pub static ONE_INT_ONE_FORM_ONE_OPTIONAL_FLOAT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneInt_OneForm_OneOptionalFloat_OneOptionalInt", &[req(ty::INTEGER), req(ty::ANY_FORM), opt(ty::FLOAT), opt(ty::INTEGER)]);
pub static ONE_FORM_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneForm_OneOptionalInt", &[req(ty::ANY_FORM), opt(ty::INTEGER)]);
pub static ONE_FORM_ONE_INT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneForm_OneInt_OneOptionalInt", &[req(ty::ANY_FORM), req(ty::INTEGER), opt(ty::INTEGER)]);
pub static TWO_FORMS_ONE_INT: Params = Params::new("kParams_TwoForms_OneInt", &[req(ty::ANY_FORM), req(ty::ANY_FORM), req(ty::INTEGER)]);
pub static FORM_CONDITION: Params = Params::new("kParams_FormCondition", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::ANY_FORM), req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_STRING_ONE_OPTIONAL_STRING: Params = Params::new("kParams_OneString_OneOptionalString", &[req(ty::STRING), opt(ty::STRING)]);
pub static ONE_STRING_ONE_DOUBLE_ONE_OPTIONAL_STRING: Params = Params::new("kParams_OneString_OneDouble_OneOptionalString", &[req(ty::STRING), req(ty::DOUBLE), opt(ty::STRING)]);
pub static TWO_STRING_ONE_OPTIONAL_STRING: Params = Params::new("kParams_TwoString_OneOptionalString", &[req(ty::STRING), req(ty::STRING), opt(ty::STRING)]);
pub static ONE_STRING_ONE_INT_ONE_OPTIONAL_STRING: Params = Params::new("kParams_OneString_OneInt_OneOptionalString", &[req(ty::STRING), req(ty::INTEGER), opt(ty::STRING)]);
pub static ONE_OPTIONAL_STRING: Params = Params::new("kParams_OneOptionalString", &[opt(ty::STRING)]);
pub static ONE_INT_ONE_OPTIONAL_OBJECT_REF: Params = Params::new("kParams_OneInt_OneOptionalObjectRef", &[req(ty::INTEGER), opt(ty::OBJECT_REF)]);
pub static ONE_ACTOR_BASE: Params = Params::new("kParams_OneActorBase", &[req(ty::ACTOR_BASE)]);
pub static ONE_ACTOR_BASE_ONE_INT: Params = Params::new("kParams_OneActorBase_OneInt", &[req(ty::ACTOR_BASE), req(ty::INTEGER)]);
pub static ONE_ACTOR_BASE_TWO_INTS: Params = Params::new("kParams_OneActorBase_TwoInts", &[req(ty::ACTOR_BASE), req(ty::INTEGER), req(ty::INTEGER)]);
pub static TWO_INTS_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_TwoInts_OneOptionalActorBase", &[req(ty::INTEGER), req(ty::INTEGER), opt(ty::ACTOR_BASE)]);
pub static ONE_FACTION_ONE_INT_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneFaction_OneInt_OneOptionalActorBase", &[req(ty::FACTION), req(ty::INTEGER), opt(ty::ACTOR_BASE)]);
pub static ONE_IMAGE_SPACE_ONE_INT: Params = Params::new("kParams_OneImageSpace_OneInt", &[req(ty::IMAGE_SPACE), req(ty::INTEGER)]);
pub static ONE_IMAGE_SPACE_ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneImageSpace_OneInt_OneFloat", &[req(ty::IMAGE_SPACE), req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_IMOD_ONE_INT: Params = Params::new("kParams_OneIMOD_OneInt", &[req(ty::IMAGE_SPACE_MODIFIER), req(ty::INTEGER)]);
pub static ONE_IMOD_ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneIMOD_OneInt_OneFloat", &[req(ty::IMAGE_SPACE_MODIFIER), req(ty::INTEGER), req(ty::FLOAT)]);
pub static ONE_FORM_THREE_OPTIONAL_INTS: Params = Params::new("kParams_OneForm_ThreeOptionalInts", &[req(ty::ANY_FORM), opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_GLOBAL_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneGlobal_OneOptionalForm", &[req(ty::GLOBAL), opt(ty::ANY_FORM)]);
pub static TWO_OPTIONAL_INTS_ONE_OPTIONAL_CONTAINER: Params = Params::new("kParams_TwoOptionalInts_OneOptionalContainer", &[opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::CONTAINER)]);
pub static ONE_FORM_ONE_STRING_ONE_INT: Params = Params::new("kParams_OneForm_OneString_OneInt", &[req(ty::ANY_FORM), req(ty::STRING), req(ty::INTEGER)]);
pub static ONE_MAGIC_ITEM: Params = Params::new("kParams_OneMagicItem", &[req(ty::MAGIC_ITEM)]);
pub static FOUR_OPTIONAL_FLOATS_ONE_OPTIONAL_INT: Params = Params::new("kParams_FourOptionalFloats_OneOptionalInt", &[opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::INTEGER)]);
pub static ONE_INT_ONE_FLOAT_TWO_OPTIONAL_FORMS: Params = Params::new("kParams_OneInt_OneFloat_TwoOptionalForms", &[req(ty::INTEGER), req(ty::FLOAT), opt(ty::ANY_FORM), opt(ty::ANY_FORM)]);
pub static ONE_FORM_ONE_INT_ONE_QUEST_ONE_INT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneForm_OneInt_OneQuest_OneInt_OneOptionalInt", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::QUEST), req(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_FORM_ONE_INT_THREE_OPTIONAL_FORMS: Params = Params::new("kParams_OneForm_OneInt_ThreeOptionalForms", &[req(ty::ANY_FORM), req(ty::INTEGER), opt(ty::ANY_FORM), opt(ty::ANY_FORM), opt(ty::ANY_FORM)]);
pub static ONE_OBJECT_ID_ONE_INT_ONE_FORMAT_STRING: Params = Params::new("kParams_OneObjectID_OneInt_OneFormatString", &[req(ty::OBJECT_ID), req(ty::INTEGER), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_OPTIONAL_OBJECT_REF_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneOptionalObjectRef_TwoOptionalInts", &[opt(ty::OBJECT_REF), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_OBJECT_REF_ONE_FLOAT: Params = Params::new("kParams_OneObjectRef_OneFloat", &[req(ty::OBJECT_REF), req(ty::FLOAT)]);
pub static ONE_QUEST_TWO_INTS_ONE_OBJECT_REF: Params = Params::new("kParams_OneQuest_TwoInts_OneObjectRef", &[req(ty::QUEST), req(ty::INTEGER), req(ty::INTEGER), req(ty::OBJECT_REF)]);
pub static ONE_QUEST_ONE_INT_ONE_OBJECT_REF: Params = Params::new("kParams_OneQuest_OneInt_OneObjectRef", &[req(ty::QUEST), req(ty::INTEGER), req(ty::OBJECT_REF)]);
pub static ONE_QUEST_ONE_INT_ONE_FORMAT_STRING: Params = Params::new("kParams_OneQuest_OneInt_OneFormatString", &[req(ty::QUEST), req(ty::INTEGER), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_OBJECT_ID_ONE_OPTIONAL_OBJECT_ID: Params = Params::new("kParams_OneObjectID_OneOptionalObjectID", &[req(ty::OBJECT_ID), opt(ty::OBJECT_ID)]);
pub static ONE_FORM_ONE_OPTIONAL_STRING: Params = Params::new("kParams_OneForm_OneOptionalString", &[req(ty::ANY_FORM), opt(ty::STRING)]);
pub static ONE_SPELL_ITEM_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneSpellItem_OneOptionalActorBase", &[req(ty::SPELL_ITEM), opt(ty::ACTOR_BASE)]);
pub static ONE_MAGIC_ITEM_ONE_OPTIONAL_ACTOR: Params = Params::new("kParams_OneMagicItem_OneOptionalActor", &[req(ty::MAGIC_ITEM), opt(ty::ACTOR)]);
pub static THREE_OPTIONAL_INTS: Params = Params::new("kParams_ThreeOptionalInts", &[opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_AI_PACKAGE_THREE_INTS: Params = Params::new("kParams_OneAIPackage_ThreeInts", &[req(ty::AI_PACKAGE), req(ty::INTEGER), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_FORM_ONE_INT_ONE_DOUBLE: Params = Params::new("kParams_OneForm_OneInt_OneDouble", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::DOUBLE)]);
pub static ONE_DOUBLE: Params = Params::new("kParams_OneDouble", &[req(ty::DOUBLE)]);
pub static FOUR_FLOATS_ONE_OPTIONAL_OBJECT_REF: Params = Params::new("kParams_FourFloats_OneOptionalObjectRef", &[req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), opt(ty::OBJECT_REF)]);
pub static ONE_ACTOR_BASE_ONE_OPTIONAL_STRING: Params = Params::new("kParams_OneActorBase_OneOptionalString", &[req(ty::ACTOR_BASE), opt(ty::STRING)]);
pub static ONE_ACTOR_BASE_ONE_STRING: Params = Params::new("kParams_OneActorBase_OneString", &[req(ty::ACTOR_BASE), req(ty::STRING)]);
pub static ONE_ACTOR_BASE_ONE_FORM: Params = Params::new("kParams_OneActorBase_OneForm", &[req(ty::ACTOR_BASE), req(ty::ANY_FORM)]);
pub static ONE_STRING_THREE_FLOATS_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_ThreeFloats_OneOptionalInt", &[req(ty::STRING), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), opt(ty::INTEGER)]);
pub static ONE_STRING_THREE_FLOATS_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneString_ThreeFloats_TwoOptionalInts", &[req(ty::STRING), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_STRING_ONE_OPTIONAL_AXIS: Params = Params::new("kParams_OneString_OneOptionalAxis", &[req(ty::STRING), opt(ty::AXIS)]);
pub static ONE_STRING_ONE_AXIS_ONE_FLOAT: Params = Params::new("kParams_OneString_OneAxis_OneFloat", &[req(ty::STRING), req(ty::AXIS), req(ty::FLOAT)]);
pub static ONE_FORM_ONE_INT_ONE_FORM_THREE_FLOATS: Params = Params::new("kParams_OneForm_OneInt_OneForm_ThreeFloats", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::ANY_FORM), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT)]);
pub static ONE_ACTOR_VALUE_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneActorValue_OneOptionalInt", &[req(ty::ACTOR_VALUE), opt(ty::INTEGER)]);
pub static THREE_GLOBALS_ONE_OPT_FLOAT_ONE_OPT_INT_ONE_OPT_STRING_ONE_OPT_FLOAT: Params = Params::new("kParams_ThreeGlobals_OneOptFloat_OneOptInt_OneOptString_OneOptFloat", &[req(ty::GLOBAL), req(ty::GLOBAL), req(ty::GLOBAL), opt(ty::FLOAT), opt(ty::INTEGER), opt(ty::STRING), opt(ty::FLOAT)]);
pub static ONE_OBJECT_REF_ONE_STRING_THREE_OPTIONAL_FLOATS: Params = Params::new("kParams_OneObjectRef_OneString_ThreeOptionalFloats", &[req(ty::OBJECT_REF), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_OBJECT_ID_ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneObjectID_OneOptionalActorBase", &[req(ty::OBJECT_ID), opt(ty::ACTOR_BASE)]);
pub static ONE_STRING_ONE_FORM_THREE_OPTIONAL_FLOATS: Params = Params::new("kParams_OneString_OneForm_ThreeOptionalFloats", &[req(ty::STRING), req(ty::ANY_FORM), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_EFFECT: Params = Params::new("kParams_OneEffect", &[req(ty::MAGIC_EFFECT)]);
pub static ONE_QUEST_TWO_INTS: Params = Params::new("kParams_OneQuest_TwoInts", &[req(ty::QUEST), req(ty::INTEGER), req(ty::INTEGER)]);
pub static ONE_FLOAT_THREE_OPTIONAL_FLOATS_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneFloat_ThreeOptionalFloats_OneOptionalInt", &[req(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::INTEGER)]);
pub static ONE_STRING_ONE_OPTIONAL_STRING_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_OneOptionalString_OneOptionalInt", &[req(ty::STRING), opt(ty::STRING), opt(ty::INTEGER)]);
pub static ONE_OBJECT_REF_ONE_FLOAT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneObjectRef_OneFloat_OneOptionalInt", &[req(ty::OBJECT_REF), req(ty::FLOAT), opt(ty::INTEGER)]);
pub static ONE_OBJECT_ID_ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneObjectID_OneInt_OneFloat", &[req(ty::OBJECT_ID), req(ty::INTEGER), req(ty::FLOAT)]);
pub static TWO_FLOATS_THREE_OPTIONAL_INTS: Params = Params::new("kParams_TwoFloats_ThreeOptionalInts", &[req(ty::FLOAT), req(ty::FLOAT), opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_NOTE: Params = Params::new("kParams_OneNote", &[req(ty::NOTE)]);
pub static ONE_FORM_ONE_OPTIONAL_SPELL_ITEM: Params = Params::new("kParams_OneForm_OneOptionalSpellItem", &[req(ty::ANY_FORM), opt(ty::SPELL_ITEM)]);
pub static TWO_FLOATS_ONE_OPTIONAL_OBJECT_REF_ONE_OPTIONAL_INT: Params = Params::new("kParams_TwoFloats_OneOptionalObjectRef_OneOptionalInt", &[req(ty::FLOAT), req(ty::FLOAT), opt(ty::OBJECT_REF), opt(ty::INTEGER)]);
pub static ONE_STRING_THREE_OPTIONAL_FLOATS_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_ThreeOptionalFloats_OneOptionalInt", &[req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::INTEGER)]);
pub static ONE_FORM_ONE_INT_ONE_FORMAT_STRING: Params = Params::new("kParams_OneForm_OneInt_OneFormatString", &[req(ty::ANY_FORM), req(ty::INTEGER), req(ty::STRING), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT), opt(ty::FLOAT)]);
pub static ONE_OPTIONAL_OBJECT_REF_ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT_ONE_OPTIONAL_STRING: Params = Params::new("kParams_OneOptionalObjectRef_OneOptionalInt_OneOptionalFloat_OneOptionalString", &[opt(ty::OBJECT_REF), opt(ty::INTEGER), opt(ty::FLOAT), opt(ty::STRING)]);
pub static ONE_OPTIONAL_INT_ONE_OPTIONAL_STRING: Params = Params::new("kParams_OneOptionalInt_OneOptionalString", &[opt(ty::INTEGER), opt(ty::STRING)]);
pub static ONE_OBJECT_ID_ONE_OPTIONAL_STRING: Params = Params::new("kParams_OneObjectID_OneOptionalString", &[req(ty::OBJECT_ID), opt(ty::STRING)]);
pub static TWO_SCRIPT_VARS: Params = Params::new("kParams_TwoScriptVars", &[req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE)]);
pub static ONE_STRING_ONE_FLOAT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_OneFloat_OneOptionalInt", &[req(ty::STRING), req(ty::FLOAT), opt(ty::INTEGER)]);
pub static ONE_STRING_ONE_INT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_OneInt_OneOptionalInt", &[req(ty::STRING), req(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_STRING_TWO_INTS_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_TwoInts_OneOptionalInt", &[req(ty::STRING), req(ty::INTEGER), req(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_OPTIONAL_FLOAT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneOptionalFloat_OneOptionalInt", &[opt(ty::FLOAT), opt(ty::INTEGER)]);
pub static ONE_OPTIONAL_STRING_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneOptionalString_TwoOptionalInts", &[opt(ty::STRING), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static THREE_SCRIPT_VARS: Params = Params::new("kParams_ThreeScriptVars", &[req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE)]);
pub static THREE_SCRIPT_VARS_ONE_OPTIONAL_INT: Params = Params::new("kParams_ThreeScriptVars_OneOptionalInt", &[req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), opt(ty::INTEGER)]);
pub static ONE_STRING_TWO_INTS_ONE_FLOAT: Params = Params::new("kParams_OneString_TwoInts_OneFloat", &[req(ty::STRING), req(ty::INTEGER), req(ty::INTEGER), req(ty::FLOAT)]);
pub static TWO_STRINGS_ONE_FLOAT_THREE_OPTIONAL_INTS: Params = Params::new("kParams_TwoStrings_OneFloat_ThreeOptionalInts", &[req(ty::STRING), req(ty::STRING), req(ty::FLOAT), opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static TWO_STRINGS_ONE_OPTIONAL_INT: Params = Params::new("kParams_TwoStrings_OneOptionalInt", &[req(ty::STRING), req(ty::STRING), opt(ty::INTEGER)]);
pub static ONE_PERK: Params = Params::new("kParams_OnePerk", &[req(ty::PERK)]);
pub static ONE_CHALLENGE: Params = Params::new("kParams_OneChallenge", &[req(ty::CHALLENGE)]);
pub static ONE_CHALLENGE_ONE_INT: Params = Params::new("kParams_OneChallenge_OneInt", &[req(ty::CHALLENGE), req(ty::INTEGER)]);
pub static ONE_CHALLENGE_ONE_FORM: Params = Params::new("kParams_OneChallenge_OneForm", &[req(ty::CHALLENGE), req(ty::ANY_FORM)]);
pub static ONE_CASINO: Params = Params::new("kParams_OneCasino", &[req(ty::CASINO)]);
pub static ONE_CASINO_ONE_INT: Params = Params::new("kParams_OneCasino_OneInt", &[req(ty::CASINO), req(ty::INTEGER)]);
pub static ONE_CASINO_ONE_FLOAT: Params = Params::new("kParams_OneCasino_OneFloat", &[req(ty::CASINO), req(ty::FLOAT)]);
pub static FIVE_FLOATS_THREE_SCRIPT_VARS_ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT: Params = Params::new("kParams_FiveFloats_ThreeScriptVars_OneOptionalInt_OneOptionalFloat", &[req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), opt(ty::INTEGER), opt(ty::FLOAT)]);
pub static THREE_FLOATS_ONE_OPTIONAL_INT: Params = Params::new("kParams_ThreeFloats_OneOptionalInt", &[req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), opt(ty::INTEGER)]);
pub static THREE_FLOATS_THREE_SCRIPT_VARS: Params = Params::new("kParams_ThreeFloats_ThreeScriptVars", &[req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE)]);
pub static ONE_IDLE_FORM: Params = Params::new("kParams_OneIdleForm", &[req(ty::IDLE_FORM)]);
pub static ONE_OPTIONAL_IDLE_FORM: Params = Params::new("kParams_OneOptionalIdleForm", &[opt(ty::IDLE_FORM)]);
pub static ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT: Params = Params::new("kParams_OneOptionalInt_OneOptionalFloat", &[opt(ty::INTEGER), opt(ty::FLOAT)]);
pub static ONE_STRING_THREE_SCRIPT_VARS_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneString_ThreeScriptVars_TwoOptionalInts", &[req(ty::STRING), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_STRING_THREE_SCRIPT_VARS_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_ThreeScriptVars_OneOptionalInt", &[req(ty::STRING), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), opt(ty::INTEGER)]);
pub static TWO_STRINGS_FOUR_FLOATS: Params = Params::new("kParams_TwoStrings_FourFloats", &[req(ty::STRING), req(ty::STRING), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT)]);
pub static TWO_STRINGS_FIVE_FLOATS: Params = Params::new("kParams_TwoStrings_FiveFloats", &[req(ty::STRING), req(ty::STRING), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT)]);
pub static ONE_OPTIONAL_FLOAT: Params = Params::new("kParams_OneOptionalFloat", &[opt(ty::FLOAT)]);
pub static THREE_SCRIPT_VARS_ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT: Params = Params::new("kParams_ThreeScriptVars_OneOptionalInt_OneOptionalFloat", &[req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), opt(ty::INTEGER), opt(ty::FLOAT)]);
pub static NINE_FLOATS_TWO_SCRIPT_VARS: Params = Params::new("kParams_NineFloats_TwoScriptVars", &[req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE)]);
pub static SIX_FLOATS_TWO_SCRIPT_VARS: Params = Params::new("kParams_SixFloats_TwoScriptVars", &[req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE)]);
pub static SIX_FLOATS_ONE_OPTIONAL_INT: Params = Params::new("kParams_SixFloats_OneOptionalInt", &[req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), req(ty::FLOAT), opt(ty::INTEGER)]);
pub static ONE_STRING_ONE_INT_ONE_FLOAT_THREE_SCRIPT_VARS: Params = Params::new("kParams_OneString_OneInt_OneFloat_ThreeScriptVars", &[req(ty::STRING), req(ty::INTEGER), req(ty::FLOAT), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE)]);
pub static ONE_STRING_ONE_INT_ONE_OPTIONAL_STRING_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_OneInt_OneOptionalString_OneOptionalInt", &[req(ty::STRING), req(ty::INTEGER), opt(ty::STRING), opt(ty::INTEGER)]);
pub static ONE_FLOAT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneFloat_OneOptionalInt", &[req(ty::FLOAT), opt(ty::INTEGER)]);
pub static THREE_SCRIPT_VARS_THREE_OPTIONAL_SCRIPT_VARS: Params = Params::new("kParams_ThreeScriptVars_ThreeOptionalScriptVars", &[req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), req(ty::SCRIPT_VARIABLE), opt(ty::SCRIPT_VARIABLE), opt(ty::SCRIPT_VARIABLE), opt(ty::SCRIPT_VARIABLE)]);
pub static NVSE_ONE_NUM_ONE_OPTIONAL_NUM: Params = Params::new("kParams_NVSE_OneNum_OneOptionalNum", &[req(ty::NVSE_NUMBER), opt(ty::NVSE_NUMBER)]);
pub static NVSE_ONE_OPTIONAL_NUM_ONE_OPTIONAL_BASIC_TYPE: Params = Params::new("kParams_NVSE_OneOptionalNum_OneOptionalBasicType", &[opt(ty::NVSE_NUMBER), opt(ty::NVSE_BASIC_TYPE)]);
pub static ONE_INT_THREE_OPTIONAL_INTS: Params = Params::new("kParams_OneInt_ThreeOptionalInts", &[req(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER), opt(ty::INTEGER)]);
pub static ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneOptionalInt_OneOptionalFloat_OneOptionalInt", &[opt(ty::INTEGER), opt(ty::FLOAT), opt(ty::INTEGER)]);

/// The slots in opcode order: `0x2200..=0x233A` (the former lutana plugin's block, mostly
/// `functions_ln/`), then `0x2600..=0x2942` (JIP's own). Each command: opcode, name, alias (empty for
/// none), needs a reference, parameters, return type, kind, and the source file (without `.h`) its
/// `DEFINE_*` line is in. An empty slot (`REG_EMPTY`) names the command it once held, where the source
/// says.
pub static SLOTS: [Slot; 1150] = [
    cmd(0x2200, "GetLNVersion", "GetJIPLNVersion", false, &NO_PARAMS, NUM, Command, "ln_fn_utility"),
    cmd(0x2201, "FileExists", "", false, &ONE_STRING_ONE_OPTIONAL_INT, NUM, Command, "ln_fn_utility"),
    cmd(0x2202, "GetTimeStamp", "", false, &NO_PARAMS, STR, Command, "ln_fn_utility"),
    cmd(0x2203, "GetController", "", false, &NO_PARAMS, NUM, Command, "ln_fn_gamepad"),
    cmd(0x2204, "IsButtonPressed", "", false, &ONE_INT, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x2205, "GetLeftStickX", "GetLSX", false, &NO_PARAMS, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x2206, "GetLeftStickY", "GetLSY", false, &NO_PARAMS, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x2207, "GetRightStickX", "GetRSX", false, &NO_PARAMS, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x2208, "GetRightStickY", "GetRSY", false, &NO_PARAMS, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x2209, "GetLeftTrigger", "GetLTr", false, &NO_PARAMS, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x220a, "GetRightTrigger", "GetRTr", false, &NO_PARAMS, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x220b, "GetDeadZoneLS", "", false, &NO_PARAMS, NUM, Command, "ln_fn_gamepad"),
    cmd(0x220c, "GetDeadZoneRS", "", false, &NO_PARAMS, NUM, Command, "ln_fn_gamepad"),
    cmd(0x220d, "SetDeadZoneLS", "", false, &ONE_DOUBLE, NUM, Command, "ln_fn_gamepad"),
    cmd(0x220e, "SetDeadZoneRS", "", false, &ONE_DOUBLE, NUM, Command, "ln_fn_gamepad"),
    cmd(0x220f, "GetPressedButtons", "GetButtons", false, &NO_PARAMS, NUM, Command, "ln_fn_gamepad"),
    cmd(0x2210, "GetNVSEVersionFull", "", false, &NO_PARAMS, NUM, Command, "patches_cmd"),
    cmd(0x2211, "GetLoadedType", "GLT", false, &ONE_LIST_ONE_INT_TWO_OPTIONAL_INTS, NUM, Command, "ln_fn_game_data"),
    cmd(0x2212, "ListToArray", "", false, &FORM_LIST, ARR, Command, "ln_fn_utility"),
    empty(0x2213, &THREE_OPTIONAL_INTS),
    cmd(0x2214, "GetCombatStyle", "", false, &ONE_OPTIONAL_ACTOR_BASE, FORM, Command, "ln_fn_actor"),
    empty(0x2215, &THREE_OPTIONAL_INTS),
    cmd(0x2216, "SaveHotkeys", "", false, &NO_PARAMS, ARR, Command, "ln_fn_inventory"),
    cmd(0x2217, "RestoreHotkeys", "", false, &ONE_OPTIONAL_INT, NUM, Command, "ln_fn_inventory"),
    cmd(0x2218, "GetBipedModelList", "GetBMList", false, &ONE_OPTIONAL_OBJECT_ID, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x2219, "SetBipedModelList", "SetBMList", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x221a, "GetModName", "", false, &ONE_INT_ONE_OPTIONAL_INT, STR, Command, "ln_fn_game_data"),
    cmd(0x221b, "HasPerkRank", "", true, &ONE_FORM_ONE_OPTIONAL_INT, NUM, Command, "ln_fn_perk"),
    cmd(0x221c, "SetGlobalValue", "SGV", false, &ONE_STRING_ONE_FLOAT, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x221d, "GetSelfModIndex", "", false, &NO_PARAMS, NUM, Command, "ln_fn_game_data"),
    cmd(0x221e, "LeveledListAddForm", "LListAddForm", false, &TWO_FORMS_TWO_INTS_ONE_FLOAT, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x221f, "LeveledListRemoveForm", "LListRemoveForm", false, &TWO_FORMS, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2220, "GetLevItemIndexByForm", "LListGetFormIndex", false, &TWO_FORMS, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2221, "GetLevItemIndexByLevel", "LListGetLevelIndex", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2222, "GetChanceNone", "", false, &ONE_FORM, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2223, "SetChanceNone", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2224, "GetNumLevItems", "LListGetCount", false, &ONE_FORM, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2225, "GetNthLevItem", "LListGetNthForm", false, &ONE_FORM_ONE_INT, FORM, Command, "ln_fn_leveled_list"),
    cmd(0x2226, "GetNthLevItemLevel", "LListGetNthLevel", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2227, "GetNthLevItemCount", "LListGetNthCount", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2228, "RemoveNthLevItem", "LListRemoveNthForm", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2229, "LeveledListClear", "LListClear", false, &ONE_FORM, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x222a, "DumpLevList", "LListDump", false, &ONE_FORM, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x222b, "GetChanceNoneGlobal", "", false, &ONE_FORM, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x222c, "ClearAllHotkeys", "", false, &NO_PARAMS, NUM, Command, "ln_fn_inventory"),
    cmd(0x222d, "GetMapMarkerRep", "", true, &NO_PARAMS, FORM, Command, "ln_fn_map_marker"),
    cmd(0x222e, "SetMapMarkerRep", "", true, &ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_map_marker"),
    cmd(0x222f, "GetMapMarkerName", "", true, &NO_PARAMS, STR, Command, "ln_fn_map_marker"),
    cmd(0x2230, "SetMapMarkerName", "", true, &ONE_STRING, NUM, Command, "ln_fn_map_marker"),
    cmd(0x2231, "GetMapMarkerType", "", true, &NO_PARAMS, NUM, Command, "ln_fn_map_marker"),
    cmd(0x2232, "SetMapMarkerType", "", true, &ONE_INT, NUM, Command, "ln_fn_map_marker"),
    empty(0x2233, &THREE_OPTIONAL_INTS),
    cmd(0x2234, "SetMapMarkerVisible", "", true, &ONE_INT, NUM, Command, "ln_fn_map_marker"),
    cmd(0x2235, "GetMapMarkerHidden", "", true, &NO_PARAMS, NUM, Command, "ln_fn_map_marker"),
    cmd(0x2236, "SetMapMarkerHidden", "", true, &ONE_INT, NUM, Command, "ln_fn_map_marker"),
    cmd(0x2237, "GetMapMarkerTravel", "", true, &NO_PARAMS, NUM, Command, "ln_fn_map_marker"),
    cmd(0x2238, "SetMapMarkerTravel", "", true, &ONE_INT, NUM, Command, "ln_fn_map_marker"),
    cmd(0x2239, "GetFormMods", "", false, &ONE_OPTIONAL_FORM, ARR, Command, "ln_fn_game_data"),
    cmd(0x223a, "IsFormOverridden", "", false, &ONE_FORM, NUM, Command, "ln_fn_game_data"),
    cmd(0x223b, "SetPerkRank", "", true, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "ln_fn_perk"),
    cmd(0x223c, "GetMerchantContainer", "", true, &NO_PARAMS, FORM, Command, "ln_fn_actor"),
    cmd(0x223d, "GetRadioBroadcastType", "", true, &NO_PARAMS, NUM, Command, "ln_fn_radio_ref"),
    cmd(0x223e, "SetRadioBroadcastType", "", true, &ONE_INT, NUM, Command, "ln_fn_radio_ref"),
    cmd(0x223f, "GetRadioRadius", "", true, &NO_PARAMS, NUM, Command, "ln_fn_radio_ref"),
    cmd(0x2240, "SetRadioRadius", "", true, &ONE_FLOAT, NUM, Command, "ln_fn_radio_ref"),
    cmd(0x2241, "GetRadioStatic", "", true, &NO_PARAMS, NUM, Command, "ln_fn_radio_ref"),
    cmd(0x2242, "SetRadioStatic", "", true, &ONE_FLOAT, NUM, Command, "ln_fn_radio_ref"),
    cmd(0x2243, "GetRadioPosRef", "", true, &NO_PARAMS, NUM, Command, "ln_fn_radio_ref"),
    cmd(0x2244, "SetRadioPosRef", "", true, &ONE_OBJECT_REF, NUM, Command, "ln_fn_radio_ref"),
    cmd(0x2245, "IsRadioRef", "", true, &NO_PARAMS, NUM, Command, "ln_fn_radio_ref"),
    cmd(0x2246, "Search", "", false, &ONE_STRING_ONE_OPTIONAL_STRING, NUM, Command, "ln_fn_game_data"),
    cmd(0x2247, "LNGetAggroRadius", "", false, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x2248, "LNSetAggroRadius", "", false, &ONE_INT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x2249, "IsMapMarker", "", true, &NO_PARAMS, NUM, Command, "ln_fn_map_marker"),
    cmd(0x224a, "GetCellCoords", "", false, &ONE_OPTIONAL_FORM, ARR, Command, "ln_fn_miscellaneous"),
    cmd(0x224b, "GetLockedOut", "", false, &ONE_OPTIONAL_OBJECT_REF, NUM, Command, "ln_fn_terminal"),
    cmd(0x224c, "SetLockedOut", "", false, &ONE_INT_ONE_OPTIONAL_OBJECT_REF, NUM, Command, "ln_fn_terminal"),
    cmd(0x224d, "GetCreatureType", "", false, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x224e, "GetLoadedTypeArray", "GLTA", false, &ONE_INT_ONE_OPTIONAL_INT, ARR, Command, "ln_fn_game_data"),
    cmd(0x224f, "GetPasswordNote", "", false, &ONE_OPTIONAL_FORM, FORM, Command, "ln_fn_terminal"),
    cmd(0x2250, "SetPasswordNote", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_terminal"),
    cmd(0x2251, "GetFormRefs", "", false, &ONE_FORM_ONE_OPTIONAL_INT, ARR, Command, "ln_fn_game_data"),
    cmd(0x2252, "GetCellLightingTemplate", "", false, &ONE_FORM, FORM, Command, "ln_fn_cell"),
    cmd(0x2253, "SetCellLightingTemplate", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_cell"),
    cmd(0x2254, "GetRecipeInputForms", "", false, &ONE_FORM, ARR, Command, "ln_fn_recipe"),
    cmd(0x2255, "GetRecipeOutputForms", "", false, &ONE_FORM, ARR, Command, "ln_fn_recipe"),
    cmd(0x2256, "GetRecipeInputCount", "", false, &TWO_FORMS, NUM, Command, "ln_fn_recipe"),
    cmd(0x2257, "SetRecipeInputCount", "", false, &TWO_FORMS_ONE_INT, NUM, Command, "ln_fn_recipe"),
    cmd(0x2258, "GetRecipeOutputCount", "", false, &TWO_FORMS, NUM, Command, "ln_fn_recipe"),
    cmd(0x2259, "SetRecipeOutputCount", "", false, &TWO_FORMS_ONE_INT, NUM, Command, "ln_fn_recipe"),
    cmd(0x225a, "GetRecipeRequiredSkill", "", false, &ONE_FORM, NUM, Command, "ln_fn_recipe"),
    cmd(0x225b, "SetRecipeRequiredSkill", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_recipe"),
    cmd(0x225c, "GetRecipeRequiredSkillLevel", "", false, &ONE_FORM, NUM, Command, "ln_fn_recipe"),
    cmd(0x225d, "SetRecipeRequiredSkillLevel", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_recipe"),
    cmd(0x225e, "ReplaceRecipeInputForm", "", false, &THREE_FORMS, NUM, Command, "ln_fn_recipe"),
    cmd(0x225f, "ReplaceRecipeOutputForm", "", false, &THREE_FORMS, NUM, Command, "ln_fn_recipe"),
    cmd(0x2260, "GetFormRecipes", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, ARR, Command, "ln_fn_recipe"),
    cmd(0x2261, "GetFormRecipeOutputs", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, ARR, Command, "ln_fn_recipe"),
    cmd(0x2262, "GetWorldspaceParentWorldspace", "GetWorldParentWorld", false, &ONE_WORLDSPACE, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x2263, "GetTerminalLock", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_terminal"),
    cmd(0x2264, "GetArmorClass", "GetAC", false, &ONE_OPTIONAL_OBJECT_ID, NUM, Condition, "ln_fn_miscellaneous"),
    cmd(0x2265, "IsRaceInList", "", true, &FORM_LIST, NUM, Condition, "ln_fn_actor"),
    cmd(0x2266, "GetFormFromMod", "GFFM", false, &TWO_STRINGS, FORM, Command, "ln_fn_game_data"),
    cmd(0x2267, "GetPerkEntryCount", "", false, &ONE_PERK, NUM, Command, "ln_fn_perk"),
    cmd(0x2268, "GetNthPerkEntryType", "", false, &ONE_PERK_ONE_INT, NUM, Command, "ln_fn_perk"),
    cmd(0x2269, "GetNthPerkEntryFunction", "", false, &ONE_PERK_ONE_INT, NUM, Command, "ln_fn_perk"),
    cmd(0x226a, "SetNthPerkEntryFunction", "", false, &ONE_PERK_TWO_INTS, NUM, Command, "ln_fn_perk"),
    cmd(0x226b, "GetNthPerkEntryForm", "", false, &ONE_PERK_ONE_INT, FORM, Command, "ln_fn_perk"),
    cmd(0x226c, "SetNthPerkEntryForm", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "ln_fn_perk"),
    cmd(0x226d, "GetNthPerkEntryValue1", "", false, &ONE_PERK_ONE_INT, NUM, Command, "ln_fn_perk"),
    cmd(0x226e, "SetNthPerkEntryValue1", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "ln_fn_perk"),
    cmd(0x226f, "GetNthPerkEntryValue2", "", false, &ONE_PERK_ONE_INT, NUM, Command, "ln_fn_perk"),
    cmd(0x2270, "SetNthPerkEntryValue2", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "ln_fn_perk"),
    cmd(0x2271, "GetNthPerkEntryString", "", false, &ONE_PERK_ONE_INT, STR, Command, "ln_fn_perk"),
    cmd(0x2272, "SetNthPerkEntryString", "", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "ln_fn_perk"),
    cmd(0x2273, "GetActivatorRadioStation", "", false, &ONE_OPTIONAL_FORM, FORM, Command, "ln_fn_activator"),
    cmd(0x2274, "SetActivatorRadioStation", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_activator"),
    cmd(0x2275, "GetActivatorPrompt", "", false, &ONE_OPTIONAL_FORM, STR, Command, "ln_fn_activator"),
    cmd(0x2276, "SetActivatorPrompt", "", false, &ONE_STRING_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_activator"),
    cmd(0x2277, "GetActivatorWaterType", "", false, &ONE_OPTIONAL_FORM, FORM, Command, "ln_fn_activator"),
    cmd(0x2278, "SetActivatorWaterType", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_activator"),
    cmd(0x2279, "GetActiveQuest", "", false, &NO_PARAMS, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x227a, "GetINIFloat", "", false, &ONE_STRING_ONE_OPTIONAL_STRING, NUM, Command, "ln_fn_utility"),
    cmd(0x227b, "GetINIString", "", false, &ONE_STRING_ONE_OPTIONAL_STRING, STR, Command, "ln_fn_utility"),
    cmd(0x227c, "SetINIFloat", "", false, &ONE_STRING_ONE_DOUBLE_ONE_OPTIONAL_STRING, NUM, Command, "ln_fn_utility"),
    cmd(0x227d, "SetINIString", "", false, &TWO_STRING_ONE_OPTIONAL_STRING, NUM, Command, "ln_fn_utility"),
    cmd(0x227e, "GetINISection", "", false, &ONE_STRING_ONE_OPTIONAL_STRING_ONE_OPTIONAL_INT, ARR, Command, "ln_fn_utility"),
    cmd(0x227f, "SetINISection", "", false, &ONE_STRING_ONE_INT_ONE_OPTIONAL_STRING, NUM, Command, "ln_fn_utility"),
    cmd(0x2280, "IsParentActivateOnly", "", false, &ONE_OPTIONAL_OBJECT_REF, NUM, Command, "ln_fn_misc_ref"),
    cmd(0x2281, "SetLinkedReference", "SetLinkedRef", true, &ONE_OPTIONAL_OBJECT_REF, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2282, "SetBaseForm", "", true, &ONE_BOUND_OBJECT, NUM, Command, "ln_fn_misc_ref"),
    cmd(0x2283, "GetStringSetting", "", false, &ONE_STRING, STR, Command, "ln_fn_game_data"),
    cmd(0x2284, "SetStringSetting", "", false, &TWO_STRINGS, NUM, Command, "ln_fn_game_data"),
    cmd(0x2285, "GetINISectionNames", "", false, &ONE_OPTIONAL_STRING, ARR, Command, "ln_fn_utility"),
    cmd(0x2286, "GetRaceVoice", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_race"),
    cmd(0x2287, "SetRaceVoice", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "ln_fn_race"),
    cmd(0x2288, "GetRaceAgeRace", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_race"),
    cmd(0x2289, "SetRaceAgeRace", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "ln_fn_race"),
    cmd(0x228a, "GetFilesInFolder", "GetFiles", false, &ONE_STRING_ONE_OPTIONAL_STRING, ARR, Command, "ln_fn_utility"),
    empty(0x228b, &THREE_OPTIONAL_INTS),
    empty(0x228c, &THREE_OPTIONAL_INTS),
    empty(0x228d, &THREE_OPTIONAL_INTS),
    cmd(0x228e, "GetLeveledListFlags", "LListGetFlags", false, &ONE_FORM, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x228f, "SetLeveledListFlags", "LListSetFlags", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2290, "GetActivatorSoundLooping", "", false, &ONE_OPTIONAL_FORM, FORM, Command, "ln_fn_activator"),
    cmd(0x2291, "SetActivatorSoundLooping", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_activator"),
    cmd(0x2292, "GetActivatorSoundActivate", "", false, &ONE_OPTIONAL_FORM, FORM, Command, "ln_fn_activator"),
    cmd(0x2293, "SetActivatorSoundActivate", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_activator"),
    cmd(0x2294, "EnableButton", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x2295, "DisableButton", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x2296, "IsButtonDisabled", "", false, &ONE_INT, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x2297, "HoldButton", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x2298, "ReleaseButton", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x2299, "IsButtonHeld", "", false, &ONE_INT, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x229a, "IsTrait", "", false, &ONE_PERK, NUM, Command, "ln_fn_perk"),
    cmd(0x229b, "GetLoadScreenTexture", "", false, &ONE_FORM, STR, Command, "ln_fn_load_screen"),
    cmd(0x229c, "SetLoadScreenTexture", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "ln_fn_load_screen"),
    cmd(0x229d, "GetLoadScreenType", "", false, &ONE_FORM, FORM, Command, "ln_fn_load_screen"),
    cmd(0x229e, "SetLoadScreenType", "", false, &TWO_FORMS, NUM, Command, "ln_fn_load_screen"),
    cmd(0x229f, "GetLoadScreenTypeTextRGB", "", false, &ONE_FORM, ARR, Command, "ln_fn_load_screen"),
    cmd(0x22a0, "SetLoadScreenTypeTextRGB", "", false, &ONE_FORM_THREE_FLOATS, NUM, Command, "ln_fn_load_screen"),
    cmd(0x22a1, "CopyFaceGenFrom", "", true, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x22a2, "SetRace", "", false, &TWO_FORMS, NUM, Command, "ln_fn_race"),
    cmd(0x22a3, "GetFoldersInFolder", "GetFolders", false, &ONE_STRING_ONE_OPTIONAL_STRING, ARR, Command, "ln_fn_utility"),
    cmd(0x22a4, "EnableTrigger", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x22a5, "DisableTrigger", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x22a6, "IsTriggerDisabled", "", false, &ONE_INT, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x22a7, "HoldTrigger", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x22a8, "ReleaseTrigger", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x22a9, "IsTriggerHeld", "", false, &ONE_INT, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x22aa, "GetPCCanUsePowerArmor", "", false, &NO_PARAMS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22ab, "GetIdleAnimPath", "", false, &ONE_FORM, STR, Command, "ln_fn_miscellaneous"),
    cmd(0x22ac, "SetIdleAnimPath", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22ad, "LNGetName", "", false, &ONE_OPTIONAL_FORM, STR, Command, "ln_fn_miscellaneous"),
    cmd(0x22ae, "RemoveINIKey", "", false, &ONE_STRING_ONE_OPTIONAL_STRING, NUM, Command, "ln_fn_utility"),
    cmd(0x22af, "RemoveINISection", "", false, &ONE_STRING_ONE_OPTIONAL_STRING, NUM, Command, "ln_fn_utility"),
    cmd(0x22b0, "GetGlobalTimeMultiplier", "ggtm", false, &NO_PARAMS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22b1, "GetCasinoMaxWinnings", "", false, &ONE_CASINO, NUM, Command, "ln_fn_casino"),
    cmd(0x22b2, "SetCasinoMaxWinnings", "", false, &ONE_CASINO_ONE_INT, NUM, Command, "ln_fn_casino"),
    cmd(0x22b3, "GetCasinoBJPayout", "", false, &ONE_CASINO, NUM, Command, "ln_fn_casino"),
    cmd(0x22b4, "SetCasinoBJPayout", "", false, &ONE_CASINO_ONE_FLOAT, NUM, Command, "ln_fn_casino"),
    cmd(0x22b5, "GetCasinoBJ17Stand", "", false, &ONE_CASINO, NUM, Command, "ln_fn_casino"),
    cmd(0x22b6, "SetCasinoBJ17Stand", "", false, &ONE_CASINO_ONE_INT, NUM, Command, "ln_fn_casino"),
    cmd(0x22b7, "GetCasinoNumDecks", "", false, &ONE_CASINO, NUM, Command, "ln_fn_casino"),
    cmd(0x22b8, "SetCasinoNumDecks", "", false, &ONE_CASINO_ONE_INT, NUM, Command, "ln_fn_casino"),
    cmd(0x22b9, "GetCasinoShufflePercent", "", false, &ONE_CASINO, NUM, Command, "ln_fn_casino"),
    cmd(0x22ba, "SetCasinoShufflePercent", "", false, &ONE_CASINO_ONE_FLOAT, NUM, Command, "ln_fn_casino"),
    cmd(0x22bb, "GetCasinoReelStops", "", false, &ONE_CASINO_ONE_INT, NUM, Command, "ln_fn_casino"),
    cmd(0x22bc, "SetCasinoReelStops", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "ln_fn_casino"),
    cmd(0x22bd, "AddNoteNS", "", false, &ONE_NOTE, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22be, "GetCellWaterForm", "", false, &ONE_OPTIONAL_FORM, FORM, Command, "ln_fn_cell"),
    cmd(0x22bf, "GetWaterFormEffect", "", false, &ONE_FORM, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x22c0, "ar_Cat", "pinto_Cat", false, &TWO_INTS_ONE_OPTIONAL_INT, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22c1, "GetInventoryWeight", "GetInvWeight", true, &ONE_OPTIONAL_INT, NUM, Command, "ln_fn_misc_ref"),
    cmd(0x22c2, "GetChallengeType", "", false, &ONE_CHALLENGE, NUM, Command, "ln_fn_challenge"),
    cmd(0x22c3, "GetChallengeThreshold", "", false, &ONE_CHALLENGE, NUM, Command, "ln_fn_challenge"),
    cmd(0x22c4, "SetChallengeThreshold", "", false, &ONE_CHALLENGE_ONE_INT, NUM, Command, "ln_fn_challenge"),
    cmd(0x22c5, "GetChallengeInterval", "", false, &ONE_CHALLENGE, NUM, Command, "ln_fn_challenge"),
    cmd(0x22c6, "SetChallengeInterval", "", false, &ONE_CHALLENGE_ONE_INT, NUM, Command, "ln_fn_challenge"),
    cmd(0x22c7, "GetChallengeValue1", "", false, &ONE_CHALLENGE, NUM, Command, "ln_fn_challenge"),
    cmd(0x22c8, "SetChallengeValue1", "", false, &ONE_CHALLENGE_ONE_INT, NUM, Command, "ln_fn_challenge"),
    cmd(0x22c9, "GetChallengeValue2", "", false, &ONE_CHALLENGE, NUM, Command, "ln_fn_challenge"),
    cmd(0x22ca, "SetChallengeValue2", "", false, &ONE_CHALLENGE_ONE_INT, NUM, Command, "ln_fn_challenge"),
    cmd(0x22cb, "GetChallengeValue3", "", false, &ONE_CHALLENGE, NUM, Command, "ln_fn_challenge"),
    cmd(0x22cc, "SetChallengeValue3", "", false, &ONE_CHALLENGE_ONE_INT, NUM, Command, "ln_fn_challenge"),
    cmd(0x22cd, "GetChallengeForm1", "", false, &ONE_CHALLENGE, FORM, Command, "ln_fn_challenge"),
    cmd(0x22ce, "SetChallengeForm1", "", false, &ONE_CHALLENGE_ONE_FORM, NUM, Command, "ln_fn_challenge"),
    cmd(0x22cf, "GetChallengeForm2", "", false, &ONE_CHALLENGE, FORM, Command, "ln_fn_challenge"),
    cmd(0x22d0, "SetChallengeForm2", "", false, &ONE_CHALLENGE_ONE_FORM, NUM, Command, "ln_fn_challenge"),
    cmd(0x22d1, "GetRecipeCategory", "", false, &ONE_FORM, FORM, Command, "ln_fn_recipe"),
    cmd(0x22d2, "SetRecipeCategory", "", false, &TWO_FORMS, NUM, Command, "ln_fn_recipe"),
    cmd(0x22d3, "GetRecipeSubcategory", "", false, &ONE_FORM, FORM, Command, "ln_fn_recipe"),
    cmd(0x22d4, "SetRecipeSubcategory", "", false, &TWO_FORMS, NUM, Command, "ln_fn_recipe"),
    empty(0x22d5, &THREE_OPTIONAL_INTS),
    cmd(0x22d6, "GetReferenceFlag", "GetRefFlag", true, &ONE_INT, NUM, Condition, "ln_fn_misc_ref"),
    cmd(0x22d7, "SetReferenceFlag", "SetRefFlag", true, &TWO_INTS, NUM, Command, "ln_fn_misc_ref"),
    cmd(0x22d8, "AddRecipeInputForm", "", false, &TWO_FORMS_ONE_INT, NUM, Command, "ln_fn_recipe"),
    cmd(0x22d9, "AddRecipeOutputForm", "", false, &TWO_FORMS_ONE_INT, NUM, Command, "ln_fn_recipe"),
    cmd(0x22da, "RemoveRecipeInputForm", "", false, &TWO_FORMS, NUM, Command, "ln_fn_recipe"),
    cmd(0x22db, "RemoveRecipeOutputForm", "", false, &TWO_FORMS, NUM, Command, "ln_fn_recipe"),
    cmd(0x22dc, "AddRecipeCondition", "", false, &FORM_CONDITION, NUM, Command, "ln_fn_recipe"),
    empty(0x22dd, &THREE_OPTIONAL_INTS),
    cmd(0x22de, "GetFormFlag", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22df, "SetFormFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22e0, "GetDistance2D", "", true, &ONE_OBJECT_REF, NUM, Condition, "ln_fn_math"),
    cmd(0x22e1, "GetDistance3D", "", true, &ONE_OBJECT_REF, NUM, Condition, "ln_fn_math"),
    cmd(0x22e2, "GetIngestibleFlag", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22e3, "SetIngestibleFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22e4, "IsSkillMagazine", "", false, &ONE_FORM, NUM, Command, "ln_fn_miscellaneous"),
    empty(0x22e5, &THREE_OPTIONAL_INTS),
    cmd(0x22e6, "GetServiceFlag", "", false, &ONE_ACTOR_BASE_ONE_INT, NUM, Command, "ln_fn_actor"),
    cmd(0x22e7, "SetServiceFlag", "", false, &ONE_ACTOR_BASE_TWO_INTS, NUM, Command, "ln_fn_actor"),
    cmd(0x22e8, "LeveledListReplaceForm", "LListReplaceForm", false, &THREE_FORMS, NUM, Command, "ln_fn_leveled_list"),
    empty(0x22e9, &THREE_OPTIONAL_INTS),
    cmd(0x22ea, "GetCellImageSpace", "", false, &ONE_FORM, FORM, Command, "ln_fn_cell"),
    cmd(0x22eb, "PlayerHasKey", "", true, &NO_PARAMS, NUM, Condition, "ln_fn_miscellaneous"),
    cmd(0x22ec, "GetCellFlag", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_cell"),
    cmd(0x22ed, "SetCellFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "ln_fn_cell"),
    empty(0x22ee, &THREE_OPTIONAL_INTS), // once GetENBFloat
    empty(0x22ef, &THREE_OPTIONAL_INTS), // once SetENBFloat
    cmd(0x22f0, "fsqrt", "", false, &ONE_DOUBLE, NUM, Command, "ln_fn_math"),
    cmd(0x22f1, "GetCellWaterHeight", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_cell"),
    cmd(0x22f2, "SetCellWaterHeight", "", false, &ONE_FLOAT_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_cell"),
    cmd(0x22f3, "GetAnimObjectIdle", "GetANIOIdle", false, &ONE_FORM, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x22f4, "SetAnimObjectIdle", "SetANIOIdle", false, &TWO_FORMS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22f5, "GetActorTemplateFlag", "", false, &ONE_INT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x22f6, "SetActorTemplateFlag", "", false, &TWO_INTS_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x22f7, "BaseGetItemCount", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_inventory"),
    cmd(0x22f8, "BaseAddItem", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_inventory"),
    cmd(0x22f9, "BaseRemoveItem", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_inventory"),
    cmd(0x22fa, "LNSetActorTemplate", "", false, &ONE_FORM_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x22fb, "BaseAddItemHealth", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_inventory"),
    cmd(0x22fc, "LNSetName", "", false, &ONE_STRING_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x22fd, "fSin", "", false, &ONE_FLOAT, NUM, Command, "ln_fn_math"),
    cmd(0x22fe, "fCos", "", false, &ONE_FLOAT, NUM, Command, "ln_fn_math"),
    cmd(0x22ff, "SortFormsByType", "", false, &TWO_INTS, ARR, Command, "ln_fn_utility"),
    cmd(0x2300, "GetFormCountType", "", false, &TWO_INTS, NUM, Command, "ln_fn_utility"),
    cmd(0x2301, "GetPickupSound", "", false, &ONE_FORM, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x2302, "SetPickupSound", "", false, &TWO_FORMS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x2303, "GetPutdownSound", "", false, &ONE_FORM, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x2304, "SetPutdownSound", "", false, &TWO_FORMS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x2305, "GetContainerOpenSound", "", false, &ONE_OPTIONAL_FORM, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x2306, "SetContainerOpenSound", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x2307, "GetContainerCloseSound", "", false, &ONE_OPTIONAL_FORM, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x2308, "SetContainerCloseSound", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_miscellaneous"),
    empty(0x2309, &THREE_OPTIONAL_INTS),
    cmd(0x230a, "GetPlayerRegions", "", false, &NO_PARAMS, ARR, Command, "ln_fn_miscellaneous"),
    cmd(0x230b, "fAtan", "", false, &ONE_FLOAT, NUM, Command, "ln_fn_math"),
    cmd(0x230c, "GetPerkLevel", "", false, &ONE_PERK, NUM, Command, "ln_fn_perk"),
    cmd(0x230d, "GetZoneFlag", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x230e, "SetZoneFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x230f, "fAsin", "", false, &ONE_FLOAT, NUM, Command, "ln_fn_math"),
    cmd(0x2310, "fAcos", "", false, &ONE_FLOAT, NUM, Command, "ln_fn_math"),
    cmd(0x2311, "SetNthLevItem", "LListsetNthForm", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2312, "SetNthLevItemLevel", "LListSetNthLevel", false, &ONE_FORM_TWO_INTS, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2313, "SetNthLevItemCount", "LListSetNthCount", false, &ONE_FORM_TWO_INTS, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x2314, "GetBaseFactionRank", "", false, &ONE_FORM_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x2315, "SetBaseFactionRank", "", false, &ONE_FACTION_ONE_INT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x2316, "GetImpactDataSet", "", false, &ONE_OBJECT_ID, FORM, Command, "ln_fn_miscellaneous"),
    cmd(0x2317, "SetImpactDataSet", "", false, &TWO_FORMS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x2318, "HasActionRef", "", true, &NO_PARAMS, NUM, Command, "ln_fn_misc_ref"),
    cmd(0x2319, "SetHotkey", "", false, &ONE_INT_ONE_FORM_ONE_OPTIONAL_FLOAT_ONE_OPTIONAL_INT, NUM, Command, "ln_fn_inventory"),
    cmd(0x231a, "GetChallengeFlags", "", false, &ONE_CHALLENGE, NUM, Command, "ln_fn_challenge"),
    cmd(0x231b, "SetChallengeFlags", "", false, &ONE_CHALLENGE_ONE_INT, NUM, Command, "ln_fn_challenge"),
    cmd(0x231c, "LNIsPlayable", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x231d, "LNSetIsPlayable", "", false, &ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x231e, "GetZone", "", true, &NO_PARAMS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x231f, "GetBaseKarma", "", false, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x2320, "SetBaseKarma", "", false, &ONE_INT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "ln_fn_actor"),
    cmd(0x2321, "ActorHasEffect", "", true, &ONE_EFFECT, NUM, Condition, "ln_fn_actor"),
    empty(0x2322, &THREE_OPTIONAL_INTS),
    cmd(0x2323, "GetEquippedData", "GetEqData", true, &ONE_INT, ARR, Command, "ln_fn_actor"),
    cmd(0x2324, "SetEquippedData", "SetEqData", true, &ONE_INT_TWO_OPTIONAL_INTS, NUM, Command, "ln_fn_actor"),
    cmd(0x2325, "EquipItemData", "", true, &ONE_OBJECT_ID_ONE_OPTIONAL_FLOAT_THREE_OPTIONAL_INTS, NUM, Command, "ln_fn_actor"),
    cmd(0x2326, "Console", "", false, &FORMAT_STRING, NUM, Command, "ln_fn_utility"),
    cmd(0x2327, "GetDefaultMessageTime", "GetDMT", false, &NO_PARAMS, NUM, Command, "ln_fn_utility"),
    cmd(0x2328, "SetDefaultMessageTime", "SetDMT", false, &ONE_FLOAT, NUM, Command, "ln_fn_utility"),
    empty(0x2329, &THREE_OPTIONAL_INTS),
    empty(0x232a, &THREE_OPTIONAL_INTS),
    cmd(0x232b, "GetWeaponKillImpulse", "", false, &ONE_OBJECT_ID, NUM, Command, "ln_fn_weapon"),
    cmd(0x232c, "SetWeaponKillImpulse", "", false, &ONE_FORM_ONE_FLOAT, NUM, Command, "ln_fn_weapon"),
    cmd(0x232d, "GetWeaponImpulseDistance", "", false, &ONE_OBJECT_ID, NUM, Command, "ln_fn_weapon"),
    cmd(0x232e, "SetWeaponImpulseDistance", "", false, &ONE_FORM_ONE_FLOAT, NUM, Command, "ln_fn_weapon"),
    cmd(0x232f, "GetWeaponVATSEffect", "", false, &ONE_OBJECT_ID, FORM, Command, "ln_fn_weapon"),
    cmd(0x2330, "SetWeaponVATSEffect", "", false, &TWO_FORMS, NUM, Command, "ln_fn_weapon"),
    cmd(0x2331, "GetWeaponCritFlags", "", false, &ONE_OBJECT_ID, NUM, Command, "ln_fn_weapon"),
    cmd(0x2332, "SetWeaponCritFlags", "", false, &ONE_OBJECT_ID_ONE_INT, NUM, Command, "ln_fn_weapon"),
    cmd(0x2333, "GetBodyPartData", "", false, &ONE_ACTOR_BASE, FORM, Command, "ln_fn_actor"),
    cmd(0x2334, "GetRadius", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_misc_ref"),
    cmd(0x2335, "SetRadius", "", false, &ONE_FLOAT_ONE_OPTIONAL_FORM, NUM, Command, "ln_fn_misc_ref"),
    cmd(0x2336, "CrosshairRefInList", "", false, &FORM_LIST, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x2337, "GetGameSettings", "", false, &ONE_OPTIONAL_INT, ARR, Command, "ln_fn_game_data"),
    cmd(0x2338, "GetShowQuestItems", "", false, &NO_PARAMS, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x2339, "GetWaterTrait", "", false, &ONE_FORM_ONE_INT, NUM, Command, "ln_fn_miscellaneous"),
    cmd(0x233a, "SetWaterTrait", "", false, &ONE_FORM_ONE_INT_ONE_DOUBLE, NUM, Command, "ln_fn_miscellaneous"),
    // JIP's own block (`SetOpcodeBase(0x2600)`).
    cmd(0x2600, "SetPersistent", "", true, &ONE_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2601, "GetActorTemplate", "", false, &ONE_OPTIONAL_ACTOR_BASE, FORM, Command, "jip_fn_actor"),
    cmd(0x2602, "GetLeveledActorBase", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x2603, "GetCreatureDamage", "", false, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x2604, "SetCreatureDamage", "", false, &ONE_INT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x2605, "GetIsPoisoned", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x2606, "AddItemAlt", "", true, &ONE_ITEM_OR_LIST_ONE_OPT_INT_ONE_OPT_FLOAT_TWO_OPT_INTS, NUM, Command, "jip_fn_inventory"),
    cmd(0x2607, "GetValueAlt", "", false, &ONE_OPTIONAL_OBJECT_ID, NUM, Command, "jip_fn_inventory"),
    cmd(0x2608, "IsComponentLoaded", "", false, &ONE_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x2609, "InjectUIXML", "", false, &TWO_STRINGS, NUM, Command, "jip_fn_ui"),
    cmd(0x260a, "InjectUIComponent", "", false, &ONE_STRING_ONE_FORMAT_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x260b, "GetAmmoTraitNumeric", "", false, &ONE_OBJECT_ID_ONE_INT, NUM, Command, "jip_fn_ammo"),
    cmd(0x260c, "SetAmmoTraitNumeric", "", false, &ONE_OBJECT_ID_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_ammo"),
    cmd(0x260d, "GetAmmoProjectile", "", false, &ONE_OBJECT_ID, FORM, Command, "jip_fn_ammo"),
    cmd(0x260e, "SetAmmoProjectile", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_ammo"),
    cmd(0x260f, "GetNumAmmoEffects", "", false, &ONE_FORM, NUM, Command, "jip_fn_ammo_effect"),
    cmd(0x2610, "GetNthAmmoEffect", "", false, &ONE_FORM_ONE_INT, FORM, Command, "jip_fn_ammo_effect"),
    cmd(0x2611, "AddAmmoEffect", "", false, &TWO_FORMS, NUM, Command, "jip_fn_ammo_effect"),
    cmd(0x2612, "RemoveAmmoEffect", "", false, &TWO_FORMS, NUM, Command, "jip_fn_ammo_effect"),
    cmd(0x2613, "GetAmmoEffectTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_ammo_effect"),
    cmd(0x2614, "SetAmmoEffectTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_ammo_effect"),
    cmd(0x2615, "GetProjectileTraitNumeric", "", false, &ONE_BOUND_OBJECT_ONE_INT, NUM, Command, "jip_fn_projectile"),
    cmd(0x2616, "SetProjectileTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_projectile"),
    cmd(0x2617, "GetProjectileFlag", "", false, &ONE_BOUND_OBJECT_ONE_INT, NUM, Command, "jip_fn_projectile"),
    cmd(0x2618, "SetProjectileFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_projectile"),
    cmd(0x2619, "GetProjectileExplosion", "", false, &ONE_BOUND_OBJECT, FORM, Command, "jip_fn_projectile"),
    cmd(0x261a, "SetProjectileExplosion", "", false, &ONE_BOUND_OBJECT_ONE_OPTIONAL_BOUND_OBJECT, NUM, Command, "jip_fn_projectile"),
    cmd(0x261b, "GetExplosionTraitNumeric", "", false, &ONE_BOUND_OBJECT_ONE_INT, NUM, Command, "jip_fn_explosion"),
    cmd(0x261c, "SetExplosionTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_explosion"),
    cmd(0x261d, "GetExplosionTraitForm", "", false, &ONE_BOUND_OBJECT_ONE_INT, FORM, Command, "jip_fn_explosion"),
    cmd(0x261e, "SetExplosionTraitForm", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_explosion"),
    cmd(0x261f, "GetExplosionFlag", "", false, &ONE_BOUND_OBJECT_ONE_INT, NUM, Command, "jip_fn_explosion"),
    cmd(0x2620, "SetExplosionFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_explosion"),
    cmd(0x2621, "AddScriptVariable", "", false, &ONE_STRING_ONE_OPTIONAL_QUEST, NUM, Command, "jip_fn_script"),
    cmd(0x2622, "GetDelayElapsed", "", false, &ONE_QUEST, NUM, Command, "jip_fn_script"),
    cmd(0x2623, "SetDelayElapsed", "", false, &ONE_QUEST_ONE_FLOAT, NUM, Command, "jip_fn_script"),
    cmd(0x2624, "GetPackageFlag", "", false, &ONE_AI_PACKAGE_ONE_INT, NUM, Command, "jip_fn_package"),
    cmd(0x2625, "SetPackageFlag", "", false, &ONE_AI_PACKAGE_TWO_INTS, NUM, Command, "jip_fn_package"),
    cmd(0x2626, "GetGlobalVariable", "", false, &ONE_GLOBAL, NUM, Command, "jip_fn_global_var"),
    cmd(0x2627, "SetGlobalVariable", "", false, &ONE_GLOBAL_ONE_FLOAT, NUM, Command, "jip_fn_global_var"),
    cmd(0x2628, "GetFactionReputationType", "", false, &ONE_FACTION, FORM, Command, "jip_fn_faction"),
    cmd(0x2629, "SetFactionReputationType", "", false, &ONE_FACTION_ONE_OPTIONAL_REPUTATION, NUM, Command, "jip_fn_faction"),
    cmd(0x262a, "RefToString", "", false, &ONE_OPTIONAL_FORM, STR, Command, "jip_fn_utility"),
    cmd(0x262b, "StringToRef", "", false, &ONE_STRING, FORM, Command, "jip_fn_utility"),
    cmd(0x262c, "GetObjectDimensions", "GetObjDim", false, &ONE_AXIS_ONE_OPTIONAL_BOUND_OBJECT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x262d, "GetIsItem", "", false, &ONE_OPTIONAL_OBJECT_ID, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x262e, "GetMinOf", "", false, &TWO_DOUBLES_THREE_OPTIONAL_DOUBLES, NUM, Command, "jip_fn_utility"),
    cmd(0x262f, "GetMaxOf", "", false, &TWO_DOUBLES_THREE_OPTIONAL_DOUBLES, NUM, Command, "jip_fn_utility"),
    cmd(0x2630, "GetCombatStyleTraitNumeric", "", false, &ONE_COMBAT_STYLE_ONE_INT, NUM, Command, "jip_fn_combat_style"),
    cmd(0x2631, "SetCombatStyleTraitNumeric", "", false, &ONE_COMBAT_STYLE_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_combat_style"),
    cmd(0x2632, "GetCombatStyleFlag", "", false, &ONE_COMBAT_STYLE_ONE_INT, NUM, Command, "jip_fn_combat_style"),
    cmd(0x2633, "SetCombatStyleFlag", "", false, &ONE_COMBAT_STYLE_TWO_INTS, NUM, Command, "jip_fn_combat_style"),
    cmd(0x2634, "SetLinkedReference", "SetLinkedRef", true, &ONE_OPTIONAL_OBJECT_REF, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2635, "GetEnableChildren", "", true, &NO_PARAMS, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x2636, "GetLinkedChildren", "", true, &NO_PARAMS, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x2637, "GetFollowers", "", true, &NO_PARAMS, ARR, Command, "jip_fn_actor"),
    cmd(0x2638, "GetCursorPos", "", false, &ONE_AXIS, NUM, Command, "jip_fn_ui"),
    cmd(0x2639, "GetUIString", "", false, &ONE_STRING, STR, Command, "jip_fn_ui"),
    cmd(0x263a, "ReadArrayFromFile", "", false, &ONE_STRING_ONE_OPTIONAL_INT, ARR, Command, "jip_fn_utility"),
    cmd(0x263b, "ReadStringFromFile", "", false, &ONE_STRING_TWO_OPTIONAL_INTS, STR, Command, "jip_fn_utility"),
    cmd(0x263c, "WriteArrayToFile", "", false, &ONE_STRING_TWO_INTS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_utility"),
    cmd(0x263d, "WriteStringToFile", "", false, &ONE_STRING_ONE_INT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_utility"),
    cmd(0x263e, "GetClassTraitNumeric", "", false, &ONE_CLASS_ONE_INT, NUM, Command, "jip_fn_class"),
    cmd(0x263f, "SetClassTraitNumeric", "", false, &ONE_CLASS_TWO_INTS, NUM, Command, "jip_fn_class"),
    cmd(0x2640, "GetClassFlag", "", false, &ONE_CLASS_ONE_INT, NUM, Command, "jip_fn_class"),
    cmd(0x2641, "SetClassFlag", "", false, &ONE_CLASS_TWO_INTS, NUM, Command, "jip_fn_class"),
    cmd(0x2642, "GetLightTraitNumeric", "", false, &ONE_BOUND_OBJECT_ONE_INT, NUM, Command, "jip_fn_light"),
    cmd(0x2643, "SetLightTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_light"),
    cmd(0x2644, "GetLightFlag", "", false, &ONE_BOUND_OBJECT_ONE_INT, NUM, Command, "jip_fn_light"),
    cmd(0x2645, "SetLightFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_light"),
    cmd(0x2646, "GetSoundTraitNumeric", "", false, &ONE_SOUND_ONE_INT, NUM, Command, "jip_fn_sound"),
    cmd(0x2647, "SetSoundTraitNumeric", "", false, &ONE_SOUND_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_sound"),
    cmd(0x2648, "GetSoundFlag", "", false, &ONE_SOUND_ONE_INT, NUM, Command, "jip_fn_sound"),
    cmd(0x2649, "SetSoundFlag", "", false, &ONE_SOUND_TWO_INTS, NUM, Command, "jip_fn_sound"),
    cmd(0x264a, "GetWeaponDetectionSoundLevel", "", false, &ONE_OBJECT_ID, NUM, Command, "jip_fn_weapon"),
    cmd(0x264b, "SetWeaponDetectionSoundLevel", "", false, &ONE_OBJECT_ID_ONE_INT, NUM, Command, "jip_fn_weapon"),
    cmd(0x264c, "IsEquippedWeaponSilenced", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_weapon"),
    cmd(0x264d, "IsEquippedWeaponScoped", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_weapon"),
    cmd(0x264e, "GetWeaponSound", "", false, &ONE_OBJECT_ID_ONE_INT, FORM, Command, "jip_fn_weapon"),
    cmd(0x264f, "SetWeaponSound", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_SOUND, NUM, Command, "jip_fn_weapon"),
    cmd(0x2650, "SetWeaponItemMod", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_OBJECT_ID, NUM, Command, "jip_fn_weapon"),
    cmd(0x2651, "SetWeaponItemModEffect", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_weapon"),
    cmd(0x2652, "SetWeaponItemModValue", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_weapon"),
    cmd(0x2653, "GetWeatherImageSpaceMod", "", false, &ONE_WEATHER_ID_ONE_INT, FORM, Command, "jip_fn_weather"),
    cmd(0x2654, "SetWeatherImageSpaceMod", "", false, &ONE_WEATHER_ID_ONE_INT_ONE_OPTIONAL_IMAGE_SPACE_MOD, NUM, Command, "jip_fn_weather"),
    cmd(0x2655, "GetWeatherTexture", "", false, &ONE_WEATHER_ID_ONE_INT, STR, Command, "jip_fn_weather"),
    cmd(0x2656, "SetWeatherTexture", "", false, &ONE_WEATHER_ID_ONE_INT_ONE_STRING, NUM, Command, "jip_fn_weather"),
    cmd(0x2657, "GetWeatherPrecipitationModel", "", false, &ONE_WEATHER_ID, STR, Command, "jip_fn_weather"),
    cmd(0x2658, "SetWeatherPrecipitationModel", "", false, &ONE_WEATHER_ID_ONE_STRING, NUM, Command, "jip_fn_weather"),
    cmd(0x2659, "GetWeatherTraitNumeric", "", false, &ONE_WEATHER_ID_ONE_INT, NUM, Command, "jip_fn_weather"),
    cmd(0x265a, "SetWeatherTraitNumeric", "", false, &ONE_WEATHER_ID_ONE_INT_ONE_DOUBLE, NUM, Command, "jip_fn_weather"),
    cmd(0x265b, "GetWeatherRGBColor", "", false, &ONE_WEATHER_ID_TWO_INTS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_weather"),
    cmd(0x265c, "SetWeatherRGBColor", "", false, &ONE_WEATHER_ID_THREE_INTS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_weather"),
    cmd(0x265d, "GetLandTextureTextureSet", "", false, &ONE_FORM, FORM, Command, "jip_fn_land_texture"),
    cmd(0x265e, "SetLandTextureTextureSet", "", false, &TWO_FORMS, NUM, Command, "jip_fn_land_texture"),
    cmd(0x265f, "GetLandTextureTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_land_texture"),
    cmd(0x2660, "SetLandTextureTraitNumeric", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_land_texture"),
    cmd(0x2661, "GetLandTextureNumGrasses", "", false, &ONE_FORM, NUM, Command, "jip_fn_land_texture"),
    cmd(0x2662, "GetLandTextureNthGrass", "", false, &ONE_FORM_ONE_INT, FORM, Command, "jip_fn_land_texture"),
    cmd(0x2663, "LandTextureAddGrass", "", false, &TWO_FORMS, NUM, Command, "jip_fn_land_texture"),
    cmd(0x2664, "LandTextureRemoveGrass", "", false, &TWO_FORMS, NUM, Command, "jip_fn_land_texture"),
    cmd(0x2665, "GetTextureSetTexture", "", false, &ONE_FORM_ONE_INT, STR, Command, "jip_fn_texture_set"),
    cmd(0x2666, "SetTextureSetTexture", "", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "jip_fn_texture_set"),
    cmd(0x2667, "GetTextureSetTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_texture_set"),
    cmd(0x2668, "SetTextureSetTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_DOUBLE, NUM, Command, "jip_fn_texture_set"),
    cmd(0x2669, "GetTextureSetFlag", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_texture_set"),
    cmd(0x266a, "SetTextureSetFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_texture_set"),
    cmd(0x266b, "GetClimateSunTexture", "", false, &ONE_FORM, STR, Command, "jip_fn_climate"),
    cmd(0x266c, "SetClimateSunTexture", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_climate"),
    cmd(0x266d, "GetClimateSunGlareTexture", "", false, &ONE_FORM, STR, Command, "jip_fn_climate"),
    cmd(0x266e, "SetClimateSunGlareTexture", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_climate"),
    cmd(0x266f, "GetClimateNightSkyModel", "", false, &ONE_FORM, STR, Command, "jip_fn_climate"),
    cmd(0x2670, "SetClimateNightSkyModel", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_climate"),
    cmd(0x2671, "GetClimateTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_climate"),
    cmd(0x2672, "SetClimateTraitNumeric", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_climate"),
    cmd(0x2673, "GetClimateNumWeatherTypes", "", false, &ONE_FORM, NUM, Command, "jip_fn_climate"),
    cmd(0x2674, "GetClimateNthWeatherType", "", false, &ONE_FORM_ONE_INT, FORM, Command, "jip_fn_climate"),
    cmd(0x2675, "GetClimateNthWeatherChance", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_climate"),
    cmd(0x2676, "GetClimateNthWeatherGlobal", "", false, &ONE_FORM_ONE_INT, FORM, Command, "jip_fn_climate"),
    cmd(0x2677, "ClimateAddWeatherType", "", false, &ONE_FORM_ONE_WEATHER_ID_ONE_INT_ONE_OPTIONAL_GLOBAL, NUM, Command, "jip_fn_climate"),
    cmd(0x2678, "ClimateRemoveWeatherType", "", false, &ONE_FORM_ONE_WEATHER_ID, NUM, Command, "jip_fn_climate"),
    cmd(0x2679, "GetPrimitiveBound", "", true, &ONE_AXIS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x267a, "SetPrimitiveBound", "", true, &ONE_AXIS_ONE_FLOAT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x267b, "GetNumEffects", "", false, &ONE_MAGIC_ITEM, NUM, Command, "jip_fn_effect"),
    cmd(0x267c, "GetNthEffectBase", "", false, &ONE_MAGIC_ITEM_ONE_INT, FORM, Command, "jip_fn_effect"),
    cmd(0x267d, "GetNthEffectTraitNumeric", "", false, &ONE_MAGIC_ITEM_TWO_INTS, NUM, Command, "jip_fn_effect"),
    cmd(0x267e, "SetNthEffectTraitNumeric", "", false, &ONE_MAGIC_ITEM_THREE_INTS, NUM, Command, "jip_fn_effect"),
    cmd(0x267f, "GetNumActorEffects", "", false, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_effect"),
    cmd(0x2680, "GetNthActorEffect", "", false, &ONE_INT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_effect"),
    cmd(0x2681, "GetActiveMenuMode", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ui"),
    cmd(0x2682, "GetActiveUIComponentName", "", false, &NO_PARAMS, STR, Command, "jip_fn_ui"),
    cmd(0x2683, "GetActiveUIComponentFullName", "", false, &NO_PARAMS, STR, Command, "jip_fn_ui"),
    cmd(0x2684, "GetActiveUIComponentID", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ui"),
    cmd(0x2685, "GetMenuTargetRef", "", false, &ONE_INT, FORM, Command, "jip_fn_ui"),
    cmd(0x2686, "GetMenuItemFilter", "", false, &ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x2687, "ClickMenuButton", "", false, &ONE_STRING_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x2688, "RemoveItemTarget", "", true, &ONE_ITEM_OR_LIST_ONE_CONTAINER_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_inventory"),
    cmd(0x2689, "GetActorLevelingData", "", false, &ONE_INT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x268a, "SetActorLevelingData", "", false, &ONE_INT_ONE_FLOAT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x268b, "SetNthEffectBase", "", false, &ONE_MAGIC_ITEM_ONE_INT_ONE_MAGIC_EFFECT, NUM, Command, "jip_fn_effect"),
    cmd(0x268c, "HasVariableAdded", "", false, &ONE_STRING_ONE_OPTIONAL_QUEST, NUM, Command, "jip_fn_script"),
    cmd(0x268d, "RemoveScriptVariable", "", false, &ONE_STRING_ONE_OPTIONAL_QUEST, NUM, Command, "jip_fn_script"),
    cmd(0x268e, "GetLoadOrderChanged", "", false, &NO_PARAMS, NUM, Command, "jip_fn_utility"),
    cmd(0x268f, "ValidateModIndex", "", false, &ONE_INT, NUM, Command, "jip_fn_utility"),
    cmd(0x2690, "RemoveAllAddedVariables", "", false, &ONE_OPTIONAL_QUEST, NUM, Command, "jip_fn_script"),
    cmd(0x2691, "AuxiliaryVariableGetSize", "AuxVarSize", false, &ONE_STRING_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_aux_variable"),
    cmd(0x2692, "AuxiliaryVariableGetType", "AuxVarType", false, &ONE_STRING_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_aux_variable"),
    cmd(0x2693, "AuxiliaryVariableGetFloat", "AuxVarGetFlt", false, &ONE_STRING_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_aux_variable"),
    cmd(0x2694, "AuxiliaryVariableSetFloat", "AuxVarSetFlt", false, &ONE_STRING_ONE_DOUBLE_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_aux_variable"),
    cmd(0x2695, "AuxiliaryVariableGetRef", "AuxVarGetRef", false, &ONE_STRING_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM, FORM, Command, "jip_fn_aux_variable"),
    cmd(0x2696, "AuxiliaryVariableSetRef", "AuxVarSetRef", false, &ONE_STRING_ONE_FORM_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_aux_variable"),
    cmd(0x2697, "AuxiliaryVariableGetString", "AuxVarGetStr", false, &ONE_STRING_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM, STR, Command, "jip_fn_aux_variable"),
    cmd(0x2698, "AuxiliaryVariableSetString", "AuxVarSetStr", false, &TWO_STRINGS_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_aux_variable"),
    cmd(0x2699, "AuxiliaryVariableGetAll", "AuxVarGetAll", false, &ONE_INT_ONE_OPTIONAL_FORM, ARR, Command, "jip_fn_aux_variable"),
    cmd(0x269a, "AuxiliaryVariableErase", "AuxVarErase", false, &ONE_STRING_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_aux_variable"),
    cmd(0x269b, "AuxiliaryVariableEraseAll", "AuxVarEraseAll", false, &ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_aux_variable"),
    cmd(0x269c, "RefMapArrayGetSize", "RefMapSize", false, &ONE_STRING, NUM, Command, "jip_fn_ref_map"),
    cmd(0x269d, "RefMapArrayGetType", "RefMapGetType", false, &ONE_STRING_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_ref_map"),
    cmd(0x269e, "RefMapArrayGetFirst", "RefMapFirst", false, &ONE_STRING, ARR, Command, "jip_fn_ref_map"),
    cmd(0x269f, "RefMapArrayGetNext", "RefMapNext", false, &ONE_STRING, ARR, Command, "jip_fn_ref_map"),
    cmd(0x26a0, "RefMapArrayGetKeys", "RefMapKeys", false, &ONE_STRING, ARR, Command, "jip_fn_ref_map"),
    cmd(0x26a1, "RefMapArrayGetAll", "RefMapGetAll", false, &ONE_INT, ARR, Command, "jip_fn_ref_map"),
    cmd(0x26a2, "RefMapArrayErase", "RefMapErase", false, &ONE_STRING_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_ref_map"),
    cmd(0x26a3, "RefMapArrayDestroy", "RefMapDestroy", false, &ONE_STRING, NUM, Command, "jip_fn_ref_map"),
    cmd(0x26a4, "RefMapArrayGetFloat", "RefMapGetFlt", false, &ONE_STRING_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_ref_map"),
    cmd(0x26a5, "RefMapArraySetFloat", "RefMapSetFlt", false, &ONE_STRING_ONE_DOUBLE_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_ref_map"),
    cmd(0x26a6, "RefMapArrayGetRef", "RefMapGetRef", false, &ONE_STRING_ONE_OPTIONAL_FORM, FORM, Command, "jip_fn_ref_map"),
    cmd(0x26a7, "RefMapArraySetRef", "RefMapSetRef", false, &ONE_STRING_ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_ref_map"),
    cmd(0x26a8, "RefMapArrayGetString", "RefMapGetStr", false, &ONE_STRING_ONE_OPTIONAL_FORM, STR, Command, "jip_fn_ref_map"),
    cmd(0x26a9, "RefMapArraySetString", "RefMapSetStr", false, &TWO_STRINGS_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_ref_map"),
    cmd(0x26aa, "ClearJIPSavedData", "", false, &FOUR_INTS, NUM, Command, "jip_fn_utility"),
    cmd(0x26ab, "SetValueAlt", "", false, &ONE_OBJECT_ID_ONE_INT, NUM, Command, "jip_fn_inventory"),
    cmd(0x26ac, "GetBodyPartDataSkeleton", "", false, &ONE_FORM, STR, Command, "jip_fn_body_part_data"),
    cmd(0x26ad, "SetBodyPartDataSkeleton", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26ae, "GetBodyPartDataRagdoll", "", false, &ONE_FORM, FORM, Command, "jip_fn_body_part_data"),
    cmd(0x26af, "SetBodyPartDataRagdoll", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26b0, "GetBodyPartDataHasPart", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26b1, "GetBodyPartTraitNumeric", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26b2, "SetBodyPartTraitNumeric", "", false, &ONE_FORM_TWO_INTS_ONE_FLOAT, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26b3, "GetBodyPartTraitForm", "", false, &ONE_FORM_TWO_INTS, FORM, Command, "jip_fn_body_part_data"),
    cmd(0x26b4, "SetBodyPartTraitForm", "", false, &ONE_FORM_TWO_INTS_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26b5, "GetBodyPartName", "", false, &ONE_FORM_ONE_INT, STR, Command, "jip_fn_body_part_data"),
    cmd(0x26b6, "SetBodyPartName", "", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26b7, "GetBodyPartReplacementModel", "", false, &ONE_FORM_ONE_INT, STR, Command, "jip_fn_body_part_data"),
    cmd(0x26b8, "SetBodyPartReplacementModel", "", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26b9, "GetBodyPartFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26ba, "SetBodyPartFlag", "", false, &ONE_FORM_THREE_INTS, NUM, Command, "jip_fn_body_part_data"),
    cmd(0x26bb, "GetWeaponModReloadAnim", "", false, &ONE_OBJECT_ID, NUM, Command, "jip_fn_weapon"),
    cmd(0x26bc, "SetWeaponModReloadAnim", "", false, &ONE_OBJECT_ID_ONE_INT, NUM, Command, "jip_fn_weapon"),
    cmd(0x26bd, "AuxiliaryVariableGetAsArray", "AuxVarGetAsArr", false, &ONE_STRING_ONE_OPTIONAL_FORM, ARR, Command, "jip_fn_aux_variable"),
    cmd(0x26be, "AuxiliaryVariableSetFromArray", "AuxVarSetFromArr", false, &ONE_STRING_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_aux_variable"),
    cmd(0x26bf, "GetSoundSourceFile", "", false, &ONE_SOUND, STR, Command, "jip_fn_sound"),
    cmd(0x26c0, "SetSoundSourceFile", "", false, &ONE_SOUND_ONE_STRING, NUM, Command, "jip_fn_sound"),
    cmd(0x26c1, "GetDestructionDataHealth", "", false, &ONE_FORM, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26c2, "SetDestructionDataHealth", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26c3, "GetDestructionDataTargetable", "", false, &ONE_FORM, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26c4, "SetDestructionDataTargetable", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26c5, "GetDestructionDataNumStages", "", false, &ONE_FORM, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26c6, "GetNthDestructionStageTrait", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26c7, "SetNthDestructionStageTrait", "", false, &ONE_FORM_THREE_INTS, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26c8, "GetNthDestructionStageExplosion", "", false, &ONE_FORM_ONE_INT, FORM, Command, "jip_fn_destruction_data"),
    cmd(0x26c9, "SetNthDestructionStageExplosion", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26ca, "GetNthDestructionStageDebris", "", false, &ONE_FORM_ONE_INT, FORM, Command, "jip_fn_destruction_data"),
    cmd(0x26cb, "SetNthDestructionStageDebris", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26cc, "GetNthDestructionStageReplacement", "", false, &ONE_FORM_ONE_INT, STR, Command, "jip_fn_destruction_data"),
    cmd(0x26cd, "SetNthDestructionStageReplacement", "", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26ce, "GetTeammates", "", false, &NO_PARAMS, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x26cf, "GetCurrentWeather", "", false, &NO_PARAMS, FORM, Command, "jip_fn_weather"),
    cmd(0x26d0, "GetCurrentClimate", "", false, &NO_PARAMS, FORM, Command, "jip_fn_climate"),
    cmd(0x26d1, "SetCurrentClimate", "", false, &ONE_FORM, NUM, Command, "jip_fn_climate"),
    cmd(0x26d2, "RefreshCurrentClimate", "", false, &NO_PARAMS, NUM, Command, "jip_fn_climate"),
    cmd(0x26d3, "GetWorldspaceClimate", "", false, &ONE_WORLDSPACE, FORM, Command, "jip_fn_worldspace"),
    cmd(0x26d4, "SetWorldspaceClimate", "", false, &ONE_WORLDSPACE_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_worldspace"),
    cmd(0x26d5, "GetActorVoiceType", "", false, &ONE_OPTIONAL_ACTOR_BASE, FORM, Command, "jip_fn_actor"),
    cmd(0x26d6, "SetActorVoiceType", "", false, &ONE_FORM_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x26d7, "GetCreatureReach", "", false, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x26d8, "GetIsImmobile", "", false, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x26d9, "PickFromList", "", true, &ONE_LIST_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_actor"),
    cmd(0x26da, "SetStageAlt", "", false, &ONE_QUEST_ONE_INT, NUM, Command, "jip_fn_quest"),
    cmd(0x26db, "ToggleNoZPosReset", "", true, &ONE_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x26dc, "GetProjectileRefSource", "", true, &NO_PARAMS, FORM, Command, "jip_fn_projectile"),
    cmd(0x26dd, "SetProjectileRefSource", "", true, &ONE_OPTIONAL_OBJECT_REF, NUM, Command, "jip_fn_projectile"),
    cmd(0x26de, "GetProjectileRefWeapon", "", true, &NO_PARAMS, FORM, Command, "jip_fn_projectile"),
    cmd(0x26df, "SetProjectileRefWeapon", "", true, &ONE_OBJECT_ID, NUM, Command, "jip_fn_projectile"),
    cmd(0x26e0, "GetProjectileRefLifeTime", "", true, &NO_PARAMS, NUM, Command, "jip_fn_projectile"),
    cmd(0x26e1, "GetProjectileRefDistanceTraveled", "", true, &NO_PARAMS, NUM, Command, "jip_fn_projectile"),
    cmd(0x26e2, "GetProjectileRefDamage", "", true, &NO_PARAMS, NUM, Command, "jip_fn_projectile"),
    cmd(0x26e3, "SetProjectileRefDamage", "", true, &ONE_FLOAT, NUM, Command, "jip_fn_projectile"),
    cmd(0x26e4, "GetProjectileRefSpeedMult", "", true, &NO_PARAMS, NUM, Command, "jip_fn_projectile"),
    cmd(0x26e5, "SetProjectileRefSpeedMult", "", true, &ONE_FLOAT, NUM, Command, "jip_fn_projectile"),
    empty(0x26e6, &THREE_OPTIONAL_INTS),
    cmd(0x26e7, "GetWorldspaceFlag", "", false, &ONE_WORLDSPACE_ONE_INT, NUM, Command, "jip_fn_worldspace"),
    cmd(0x26e8, "SetWorldspaceFlag", "", false, &ONE_WORLDSPACE_TWO_INTS, NUM, Command, "jip_fn_worldspace"),
    cmd(0x26e9, "GetIdleLoopTimes", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x26ea, "ToggleCreatureModel", "", false, &ONE_STRING_ONE_INT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x26eb, "CreatureHasModel", "", false, &ONE_STRING_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x26ec, "GetCreatureModels", "", false, &ONE_OPTIONAL_ACTOR_BASE, ARR, Command, "jip_fn_actor"),
    cmd(0x26ed, "DisableNavMeshAlt", "", false, &ONE_FORM, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x26ee, "EnableNavMeshAlt", "", false, &ONE_FORM, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x26ef, "GetTerrainHeight", "", false, &TWO_FLOATS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x26f0, "RefMapArrayValidate", "RefMapValidate", false, &ONE_STRING, NUM, Command, "jip_fn_ref_map"),
    cmd(0x26f1, "HasPrimitive", "", true, &NO_PARAMS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x26f2, "AddPrimitive", "", true, &ONE_INT_THREE_FLOATS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x26f3, "AddDestructionStage", "", false, &ONE_FORM_THREE_INTS, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26f4, "RemoveDestructionStage", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_destruction_data"),
    cmd(0x26f5, "GetWeaponShellCasingModel", "", false, &ONE_OBJECT_ID, STR, Command, "jip_fn_weapon"),
    cmd(0x26f6, "SetWeaponShellCasingModel", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_weapon"),
    cmd(0x26f7, "AddNewEffect", "", false, &ONE_MAGIC_ITEM_ONE_EFFECT_TWO_INTS_THREE_OPTIONAL_INTS, NUM, Command, "jip_fn_effect"),
    cmd(0x26f8, "RemoveNthEffect", "", false, &ONE_MAGIC_ITEM_ONE_INT, NUM, Command, "jip_fn_effect"),
    cmd(0x26f9, "SetObjectEffect", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_effect"),
    cmd(0x26fa, "GetActorUnarmedEffect", "", false, &ONE_OPTIONAL_ACTOR_BASE, FORM, Command, "jip_fn_effect"),
    cmd(0x26fb, "SetActorUnarmedEffect", "", false, &ONE_FORM_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_effect"),
    cmd(0x26fc, "IsPCInCombat", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x26fd, "CCCOnLoad", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ccc"),
    cmd(0x26fe, "CCCSetFloat", "", false, &ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_ccc"),
    cmd(0x26ff, "CCCSetString", "", false, &ONE_INT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_ccc"),
    cmd(0x2700, "CCCSetTrait", "", false, &TWO_INTS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ccc"),
    cmd(0x2701, "CCCGetDistance", "", true, &ONE_OBJECT_REF_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ccc"),
    cmd(0x2702, "CCCTaskPackageFlags", "", false, &ONE_AI_PACKAGE_THREE_INTS, NUM, Command, "jip_fn_ccc"),
    cmd(0x2703, "CCCSetEquipped", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ccc"),
    cmd(0x2704, "CCCInFaction", "", true, &ONE_FACTION_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ccc"),
    cmd(0x2705, "CCCSetNCCS", "", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ccc"),
    cmd(0x2706, "GetEncumbranceRate", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_ccc"),
    cmd(0x2707, "CCCLoadNCCS", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ccc"),
    cmd(0x2708, "CCCSavedForm", "", false, &ONE_INT, FORM, Command, "jip_fn_ccc"),
    cmd(0x2709, "CCCLocationName", "", true, &NO_PARAMS, STR, Command, "jip_fn_ccc"),
    cmd(0x270a, "CCCGetReputation", "", true, &NO_PARAMS, FORM, Command, "jip_fn_ccc"),
    cmd(0x270b, "GetSelectedItemRef", "", false, &NO_PARAMS, FORM, Command, "jip_fn_ui"),
    cmd(0x270c, "GetWeaponRefModFlags", "", true, &NO_PARAMS, NUM, Command, "jip_fn_inventory"),
    cmd(0x270d, "SetWeaponRefModFlags", "", true, &ONE_INT, NUM, Command, "jip_fn_inventory"),
    cmd(0x270e, "GetBarterItems", "", false, &ONE_INT, ARR, Command, "jip_fn_ui"),
    cmd(0x270f, "GetItemRefCurrentHealth", "", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_inventory"),
    cmd(0x2710, "SetItemRefCurrentHealth", "", true, &ONE_FLOAT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_inventory"),
    cmd(0x2711, "GetBarterGoldAlt", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ui"),
    cmd(0x2712, "GetRecipeMenuSelection", "", false, &NO_PARAMS, FORM, Command, "jip_fn_ui"),
    cmd(0x2713, "GetRecipeMenuCategory", "", false, &NO_PARAMS, FORM, Command, "jip_fn_ui"),
    cmd(0x2714, "UnlockRecipeMenuQuantity", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x2715, "GetRecipeMenuNumSubcategories", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ui"),
    cmd(0x2716, "SetHotkeyItemRef", "", true, &ONE_INT, NUM, Command, "jip_fn_inventory"),
    cmd(0x2717, "EquipItemAlt", "", true, &ONE_OBJECT_ID_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_inventory"),
    cmd(0x2718, "UnequipItemAlt", "", true, &ONE_OBJECT_ID_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_inventory"),
    cmd(0x2719, "DropAlt", "", true, &ONE_ITEM_OR_LIST_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_inventory"),
    cmd(0x271a, "DropMeAlt", "", true, &TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_inventory"),
    cmd(0x271b, "GetFormDescription", "", false, &ONE_FORM, STR, Command, "jip_fn_miscellaneous"),
    cmd(0x271c, "GetAddictionEffect", "", false, &ONE_FORM, FORM, Command, "jip_fn_effect"),
    cmd(0x271d, "SetAddictionEffect", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_effect"),
    cmd(0x271e, "GetAddictionChance", "", false, &ONE_FORM, NUM, Command, "jip_fn_effect"),
    cmd(0x271f, "SetAddictionChance", "", false, &ONE_FORM_ONE_FLOAT, NUM, Command, "jip_fn_effect"),
    cmd(0x2720, "GetIsPoison", "", false, &ONE_FORM, NUM, Command, "jip_fn_effect"),
    cmd(0x2721, "GetContainerRespawns", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2722, "SetContainerRespawns", "", false, &ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2723, "ModBaseActorValue", "ModBaseAV", false, &ONE_ACTOR_VALUE_ONE_FLOAT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x2724, "GetGrassTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_grass"),
    cmd(0x2725, "SetGrassTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_grass"),
    cmd(0x2726, "GetGrassModel", "", false, &ONE_FORM, STR, Command, "jip_fn_grass"),
    cmd(0x2727, "SetGrassModel", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_grass"),
    cmd(0x2728, "GetWheelDisabled", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x2729, "SetWheelDisabled", "", true, &ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x272a, "GetInterruptPackage", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x272b, "EnableImprovedRecipeMenu", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ui"),
    cmd(0x272c, "InitItemFilter", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ui"),
    cmd(0x272d, "SetItemFilter", "", false, &ONE_OPTIONAL_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x272e, "GetSessionTime", "", false, &NO_PARAMS, NUM, Command, "jip_fn_utility"),
    cmd(0x272f, "GetQuestTargetsChanged", "", false, &NO_PARAMS, NUM, Command, "jip_fn_quest"),
    cmd(0x2730, "GetWorldMapPosMults", "", true, &TWO_SCRIPT_VARS, NUM, Command, "jip_fn_ui"),
    empty(0x2731, &THREE_OPTIONAL_INTS),
    empty(0x2732, &THREE_OPTIONAL_INTS),
    cmd(0x2733, "GetActiveEffects", "", true, &ONE_OPTIONAL_INT, ARR, Command, "jip_fn_effect"),
    cmd(0x2734, "GetActorProcessingLevel", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x2735, "GetActorsByProcessingLevel", "", false, &ONE_INT, ARR, Command, "jip_fn_actor"),
    cmd(0x2736, "GetExteriorCell", "", false, &ONE_WORLDSPACE_TWO_INTS, FORM, Command, "jip_fn_miscellaneous"),
    cmd(0x2737, "GetCellBuffered", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2738, "MessageExAlt", "", false, &ONE_FLOAT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x2739, "GetTempEffects", "", true, &NO_PARAMS, ARR, Command, "jip_fn_effect"),
    cmd(0x273a, "RemoveNthTempEffect", "", true, &ONE_INT, NUM, Command, "jip_fn_effect"),
    cmd(0x273b, "GetActorLightAmount", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x273c, "GetActorAlpha", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x273d, "GetActorDiveBreath", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x273e, "SetActorDiveBreath", "", true, &ONE_FLOAT, NUM, Command, "jip_fn_actor"),
    cmd(0x273f, "GetGameDifficulty", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x2740, "ToggleFirstPerson", "", false, &ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2741, "StopIdle", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x2742, "GetCombatTargets", "", true, &NO_PARAMS, ARR, Command, "jip_fn_actor"),
    cmd(0x2743, "GetCombatAllies", "", true, &NO_PARAMS, ARR, Command, "jip_fn_actor"),
    cmd(0x2744, "GetDetectedActors", "", true, &NO_PARAMS, ARR, Command, "jip_fn_actor"),
    cmd(0x2745, "GetDetectingActors", "", true, &NO_PARAMS, ARR, Command, "jip_fn_actor"),
    cmd(0x2746, "ClearActiveQuest", "", false, &NO_PARAMS, NUM, Command, "jip_fn_quest"),
    cmd(0x2747, "SetFormDescription", "", false, &ONE_FORM_ONE_FORMAT_STRING, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2748, "GetPCFastTravelled", "", false, &NO_PARAMS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2749, "GetPCMovedCell", "", false, &NO_PARAMS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x274a, "GetPCDetectionState", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x274b, "GetFontFile", "", false, &ONE_INT, STR, Command, "jip_fn_ui"),
    cmd(0x274c, "SetFontFile", "", false, &ONE_INT_ONE_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x274d, "ShowTextInputMenu", "", false, &ONE_FORM_TWO_FLOATS_ONE_FORMAT_STRING, NUM, Command, "jip_fn_ui"),
    empty(0x274e, &THREE_OPTIONAL_INTS),
    empty(0x274f, &THREE_OPTIONAL_INTS),
    empty(0x2750, &THREE_OPTIONAL_INTS),
    cmd(0x2751, "GetPipboyRadio", "", false, &NO_PARAMS, FORM, Condition, "jip_fn_miscellaneous"),
    cmd(0x2752, "CCCSayTo", "", true, &ONE_ACTOR_ONE_TOPIC_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_ccc"),
    cmd(0x2753, "CCCRunResultScripts", "", true, &ONE_TOPIC, NUM, Command, "jip_fn_ccc"),
    cmd(0x2754, "GetCombatDisabled", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x2755, "SetCombatDisabled", "", true, &ONE_INT_ONE_OPTIONAL_ACTOR, NUM, Command, "jip_fn_actor"),
    cmd(0x2756, "ToggleNoHealthReset", "", false, &ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_actor"),
    cmd(0x2757, "GetCurrentStablePackage", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x2758, "MoveToCell", "", true, &ONE_FORM_THREE_FLOATS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2759, "MoveToEditorPosition", "", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x275a, "GetTeammateUsingAmmo", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x275b, "SetTeammateUsingAmmo", "", true, &ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x275c, "ToggleDetectionFix", "", true, &ONE_INT, NUM, Command, "jip_fn_actor"),
    empty(0x275d, &THREE_OPTIONAL_INTS), // once ClearModNVSEVars
    empty(0x275e, &THREE_OPTIONAL_INTS),
    cmd(0x275f, "ToggleIgnoreLockedDoors", "", false, &ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x2760, "CCCSetFollowState", "", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ccc"),
    cmd(0x2761, "IsInCombatWith", "", true, &ONE_ACTOR, NUM, Command, "jip_fn_actor"),
    cmd(0x2762, "RefToPosStr", "", true, &NO_PARAMS, STR, Command, "jip_fn_ccc"),
    cmd(0x2763, "MoveToPosStr", "", true, &ONE_STRING, NUM, Command, "jip_fn_ccc"),
    cmd(0x2764, "RefMapArrayGetValue", "RefMapGetVal", false, &ONE_STRING_ONE_OPTIONAL_FORM, ARR, Command, "jip_fn_ref_map"),
    cmd(0x2765, "RefMapArraySetValue", "RefMapSetVal", false, &ONE_STRING_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_ref_map"),
    cmd(0x2766, "GetAllItems", "", true, &FOUR_OPTIONAL_INTS_ONE_OPTIONAL_LIST, ARR, Command, "jip_fn_inventory"),
    cmd(0x2767, "GetAllItemRefs", "", true, &FOUR_OPTIONAL_INTS_ONE_OPTIONAL_LIST, ARR, Command, "jip_fn_inventory"),
    cmd(0x2768, "LockEquipment", "", true, &ONE_INT, NUM, Command, "jip_fn_ccc"),
    cmd(0x2769, "IsAttacking", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x276a, "GetPCUsingScope", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x276b, "HolsterWeapon", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x276c, "GetFallTimeElapsed", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x276d, "GetFallTimeRemaining", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x276e, "ResetFallTime", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x276f, "GetCenterPos", "", true, &ONE_AXIS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2770, "SetIdleLoopTimes", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2771, "CCCSMS", "", false, &ONE_OPTIONAL_OBJECT_REF, NUM, Command, "jip_fn_ccc"),
    cmd(0x2772, "GetDetonationTimer", "", true, &NO_PARAMS, NUM, Command, "jip_fn_projectile"),
    cmd(0x2773, "SetDetonationTimer", "", true, &ONE_FLOAT, NUM, Command, "jip_fn_projectile"),
    cmd(0x2774, "GetMineArmed", "", true, &NO_PARAMS, NUM, Command, "jip_fn_projectile"),
    cmd(0x2775, "GetRefType", "", false, &ONE_OPTIONAL_OBJECT_REF, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2776, "GetEffectFlag", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_effect"),
    cmd(0x2777, "SetEffectFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_effect"),
    cmd(0x2778, "GetBaseEffectFlag", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_effect"),
    cmd(0x2779, "SetBaseEffectFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_effect"),
    cmd(0x277a, "GetPCUsingIronSights", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x277b, "GetPCLastExteriorDoor", "", false, &NO_PARAMS, FORM, Command, "jip_fn_miscellaneous"),
    cmd(0x277c, "GetRadiationLevelAlt", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x277d, "IsInWater", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x277e, "SetEmbeddedWeaponNode", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_weapon"),
    cmd(0x277f, "SetEmbeddedWeaponAV", "", false, &ONE_OBJECT_ID_ONE_ACTOR_VALUE, NUM, Command, "jip_fn_weapon"),
    cmd(0x2780, "GetDroppedRefs", "", true, &NO_PARAMS, ARR, Command, "jip_fn_actor"),
    cmd(0x2781, "MoveAwayFromPlayer", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x2782, "FaceObject", "", true, &ONE_OBJECT_REF, NUM, Command, "jip_fn_actor"),
    cmd(0x2783, "Turn", "", true, &ONE_FLOAT, NUM, Command, "jip_fn_actor"),
    cmd(0x2784, "SetMessageDisabled", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x2785, "GetMessageDisabled", "", false, &ONE_MESSAGE, NUM, Command, "jip_fn_ui"),
    cmd(0x2786, "GetAshPileSource", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x2787, "GetMessageFlags", "", false, &ONE_MESSAGE, NUM, Command, "jip_fn_ui"),
    cmd(0x2788, "SetMessageFlags", "", false, &ONE_MESSAGE_ONE_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x2789, "ToggleObjectCollision", "TOC", true, &ONE_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x278a, "IsSoundPlaying", "", false, &ONE_SOUND_ONE_OPTIONAL_OBJECT_REF, NUM, Command, "jip_fn_sound"),
    cmd(0x278b, "SwapTextureEx", "", false, &ONE_OBJECT_REF_TWO_STRINGS_ONE_OTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x278c, "GetMaterialPropertyValue", "GetMatrPropVal", true, &ONE_STRING_ONE_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x278d, "SetMaterialPropertyValue", "SetMatrPropVal", true, &ONE_STRING_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x278e, "GetSoundPlayers", "", false, &ONE_SOUND, ARR, Command, "jip_fn_sound"),
    cmd(0x278f, "SetMessageDisplayTime", "", false, &ONE_MESSAGE_ONE_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x2790, "GetImpactDataModel", "", false, &ONE_FORM, STR, Command, "jip_fn_impact_data"),
    cmd(0x2791, "SetImpactDataModel", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_impact_data"),
    cmd(0x2792, "GetImpactDataTrait", "", false, &ONE_FORM_ONE_INT, AMB, Command, "jip_fn_impact_data"),
    cmd(0x2793, "SetImpactDataTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_DOUBLE, NUM, Command, "jip_fn_impact_data"),
    cmd(0x2794, "SetImpactDataTraitForm", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_impact_data"),
    cmd(0x2795, "sv_RegexMatch", "", false, &ONE_INT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_string"),
    cmd(0x2796, "sv_RegexSearch", "", false, &ONE_INT_ONE_FORMAT_STRING, ARR, Command, "jip_fn_string"),
    cmd(0x2797, "sv_RegexReplace", "", false, &ONE_INT_ONE_FORMAT_STRING, STR, Command, "jip_fn_string"),
    cmd(0x2798, "SetOnMenuClickEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x2799, "SetOnFastTravelEventHandler", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x279a, "SetOnMenuOpenEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x279b, "SetOnMenuCloseEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x279c, "SetOnAnimActionEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_FORM_ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x279d, "SetOnPlayGroupEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_FORM_ONE_ANIM_GROUP, NUM, Command, "jip_fn_actor"),
    cmd(0x279e, "SetOnHealthDamageEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "jip_fn_actor"),
    cmd(0x279f, "SetOnCrippledLimbEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "jip_fn_actor"),
    cmd(0x27a0, "SetOnFireWeaponEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_actor"),
    cmd(0x27a1, "GetCurrentAmmo", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x27a2, "GetCurrentAmmoRounds", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x27a3, "GetMoonTexture", "", false, &ONE_INT, STR, Command, "jip_fn_miscellaneous"),
    cmd(0x27a4, "SetMoonTexture", "", false, &ONE_INT_ONE_STRING, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x27a5, "SetFullNameAlt", "SetActorFullNameAlt", false, &ONE_STRING_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_actor"),
    cmd(0x27a6, "SetWeatherTransitionTimeOverride", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_weather"),
    cmd(0x27a7, "GetDetectionValue", "", true, &ONE_ACTOR, NUM, Command, "jip_fn_actor"),
    cmd(0x27a8, "GetBaseActorValueAlt", "GetBaseAVAlt", false, &ONE_ACTOR_VALUE_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x27a9, "SetOnMouseoverChangeEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x27aa, "GetAlwaysRun", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x27ab, "GetAutoMove", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x27ac, "SetSpeedMult", "", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x27ad, "GetIsRagdolled", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x27ae, "ForceActorDetectionValue", "", true, &TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_actor"),
    cmd(0x27af, "SetOnPCTargetChangeEventHandler", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x27b0, "FreePlayer", "GoodbyeWorldspace", false, &NO_PARAMS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x27b1, "AuxVarGetFltCond", "", true, &ONE_QUEST_ONE_INT, NUM, ConditionOnly, "jip_fn_aux_variable"),
    cmd(0x27b2, "GetLocalGravity", "", false, &ONE_AXIS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x27b3, "SetLocalGravityVector", "", false, &THREE_FLOATS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x27b4, "GetActorVelocity", "", true, &ONE_OPTIONAL_AXIS, NUM, Command, "jip_fn_actor"),
    cmd(0x27b5, "IsInAir", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x27b6, "GetHasContact", "", true, &ONE_FORM, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27b7, "GetContactRefs", "", true, &NO_PARAMS, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x27b8, "GetHasContactBase", "", true, &ONE_FORM, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27b9, "GetHasContactType", "", true, &ONE_INT, NUM, Condition, "jip_fn_misc_ref"),
    cmd(0x27ba, "GetObjectUnderFeet", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x27bb, "GetHasPhantom", "", true, &NO_PARAMS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27bc, "GetArmorRCT", "", false, &ONE_OBJECT_ID, STR, Command, "jip_fn_armor"),
    cmd(0x27bd, "SetArmorRCT", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_armor"),
    cmd(0x27be, "GetPerkFlag", "", false, &ONE_PERK_ONE_INT, NUM, Command, "jip_fn_perk"),
    cmd(0x27bf, "SetPerkFlag", "", false, &ONE_PERK_TWO_INTS, NUM, Command, "jip_fn_perk"),
    cmd(0x27c0, "ModLogPrint", "", false, &ONE_INT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_utility"),
    cmd(0x27c1, "GetBaseEffectScript", "", false, &ONE_FORM, FORM, Command, "jip_fn_effect"),
    cmd(0x27c2, "SetBaseEffectScript", "", false, &TWO_FORMS, NUM, Command, "jip_fn_effect"),
    cmd(0x27c3, "GetWaterNoiseTexture", "", false, &ONE_FORM, STR, Command, "jip_fn_water"),
    cmd(0x27c4, "SetWaterNoiseTexture", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_water"),
    cmd(0x27c5, "SetWaterFormEffect", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, FORM, Command, "jip_fn_water"),
    cmd(0x27c6, "GetWaterSound", "", false, &ONE_FORM, FORM, Command, "jip_fn_water"),
    cmd(0x27c7, "SetWaterSound", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_water"),
    cmd(0x27c8, "GetInteractionDisabled", "", false, &ONE_OPTIONAL_BOUND_OBJECT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27c9, "SetInteractionDisabled", "", false, &ONE_INT_ONE_OPTIONAL_BOUND_OBJECT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27ca, "GetInteractionDisabledType", "", false, &ONE_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27cb, "SetInteractionDisabledType", "", false, &TWO_INTS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27cc, "GetImageSpaceTrait", "", false, &ONE_IMAGE_SPACE_ONE_INT, NUM, Command, "jip_fn_imagespace"),
    cmd(0x27cd, "SetImageSpaceTrait", "", false, &ONE_IMAGE_SPACE_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_imagespace"),
    cmd(0x27ce, "GetImageSpaceModTrait", "", false, &ONE_IMOD_ONE_INT, NUM, Command, "jip_fn_imagespace"),
    cmd(0x27cf, "SetImageSpaceModTrait", "", false, &ONE_IMOD_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_imagespace"),
    cmd(0x27d0, "SetOnKeyDownEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_input"),
    cmd(0x27d1, "SetOnKeyUpEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_input"),
    cmd(0x27d2, "SetOnControlDownEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_input"),
    cmd(0x27d3, "SetOnControlUpEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_input"),
    cmd(0x27d4, "SetGameMainLoopCallback", "SGMLC", false, &ONE_FORM_THREE_OPTIONAL_INTS, NUM, Command, "jip_fn_script"),
    cmd(0x27d5, "GetGlobalRef", "", false, &ONE_GLOBAL, FORM, Command, "jip_fn_global_var"),
    cmd(0x27d6, "SetGlobalRef", "", false, &ONE_GLOBAL_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_global_var"),
    cmd(0x27d7, "GetWorldspaceNoiseTexture", "", false, &ONE_WORLDSPACE, STR, Command, "jip_fn_worldspace"),
    cmd(0x27d8, "SetWorldspaceNoiseTexture", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_worldspace"),
    cmd(0x27d9, "GetWorldspaceWaterType", "", false, &ONE_WORLDSPACE, FORM, Command, "jip_fn_worldspace"),
    cmd(0x27da, "SetWorldspaceWaterType", "", false, &ONE_WORLDSPACE_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_worldspace"),
    cmd(0x27db, "GetWorldspaceImagespace", "", false, &ONE_WORLDSPACE, FORM, Command, "jip_fn_worldspace"),
    cmd(0x27dc, "SetWorldspaceImagespace", "", false, &ONE_WORLDSPACE_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_worldspace"),
    cmd(0x27dd, "SetCellWaterForm", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_cell"),
    cmd(0x27de, "GetCellClimate", "", false, &ONE_FORM, FORM, Command, "jip_fn_cell"),
    cmd(0x27df, "SetCellClimate", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_cell"),
    cmd(0x27e0, "GetCellNoiseTexture", "", false, &ONE_FORM, STR, Command, "jip_fn_cell"),
    cmd(0x27e1, "SetCellNoiseTexture", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_cell"),
    cmd(0x27e2, "RemoveMeIRAlt", "", true, &TWO_OPTIONAL_INTS_ONE_OPTIONAL_CONTAINER, NUM, Command, "jip_fn_inventory"),
    cmd(0x27e3, "GetScriptDisabled", "", false, &ONE_FORM, NUM, Command, "jip_fn_script"),
    cmd(0x27e4, "SetScriptDisabled", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_script"),
    cmd(0x27e5, "GetScriptEventDisabled", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_script"),
    cmd(0x27e6, "SetScriptEventDisabled", "", false, &ONE_FORM_ONE_STRING_ONE_INT, NUM, Command, "jip_fn_script"),
    cmd(0x27e7, "FakeScriptEvent", "", true, &ONE_STRING_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_script"),
    cmd(0x27e8, "RefreshItemsList", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ui"),
    cmd(0x27e9, "AddRefMapMarker", "", true, &NO_PARAMS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27ea, "RemoveRefMapMarker", "", true, &NO_PARAMS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27eb, "RefHasMapMarker", "", true, &NO_PARAMS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27ec, "IsSpellTargetAlt", "", true, &ONE_MAGIC_ITEM, NUM, Condition, "jip_fn_effect"),
    cmd(0x27ed, "GetCalculatedWeaponDamage", "", false, &ONE_OPTIONAL_OBJECT_ID, NUM, Command, "jip_fn_weapon"),
    cmd(0x27ee, "GetOptionalPatch", "", false, &ONE_STRING, NUM, Command, "jip_fn_utility"),
    cmd(0x27ef, "SetOptionalPatch", "", false, &ONE_STRING_ONE_INT, NUM, Command, "jip_fn_utility"),
    cmd(0x27f0, "SetPosEx", "", true, &THREE_FLOATS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27f1, "GetReticlePos", "", false, &ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT, ARR, Command, "jip_fn_miscellaneous"),
    cmd(0x27f2, "GetReticleRange", "", false, &ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x27f3, "MoveToReticle", "", true, &FOUR_OPTIONAL_FLOATS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27f4, "SetRefName", "", true, &ONE_OPTIONAL_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27f5, "GetActorValueModifier", "GetAVMod", true, &ONE_ACTOR_VALUE_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x27f6, "GetPerkModifier", "", true, &ONE_INT_ONE_FLOAT_TWO_OPTIONAL_FORMS, NUM, Command, "jip_fn_actor"),
    cmd(0x27f7, "SetOnQuestStageEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_QUEST_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_script"),
    cmd(0x27f8, "SetAngleEx", "", true, &THREE_FLOATS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x27f9, "GetTeleportDoor", "", true, &NO_PARAMS, FORM, Command, "jip_fn_misc_ref"),
    cmd(0x27fa, "GetBarterPriceMult", "", false, &ONE_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x27fb, "SetBarterPriceMult", "", false, &ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_ui"),
    cmd(0x27fc, "SetOnDialogTopicEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x27fd, "TapButton", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x27fe, "TapTrigger", "", false, &ONE_INT, NUM, Command, "ln_fn_gamepad"),
    cmd(0x27ff, "SetOnTriggerDownEventHandler", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "ln_fn_gamepad"),
    cmd(0x2800, "SetOnTriggerUpEventHandler", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "ln_fn_gamepad"),
    cmd(0x2801, "SetOnCriticalHitEventHandler", "", false, &ONE_FORM_ONE_INT_THREE_OPTIONAL_FORMS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2802, "GetHitHealthDamage", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x2803, "GetHitLimbDamage", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x2804, "CrippleLimb", "", true, &ONE_INT_ONE_OPTIONAL_OBJECT_REF, NUM, Command, "jip_fn_actor"),
    cmd(0x2805, "HoldControl", "", false, &ONE_INT, NUM, Command, "jip_fn_input"),
    cmd(0x2806, "ReleaseControl", "", false, &ONE_INT, NUM, Command, "jip_fn_input"),
    cmd(0x2807, "PlayIdleEx", "", true, &ONE_OPTIONAL_IDLE_FORM, NUM, Command, "jip_fn_actor"),
    cmd(0x2808, "SetBipedModelPathAlt", "", false, &ONE_OBJECT_ID_ONE_INT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_armor"),
    cmd(0x2809, "GetKillXP", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    empty(0x280a, &THREE_OPTIONAL_INTS),
    cmd(0x280b, "GetKiller", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x280c, "KillActorAlt", "", true, &ONE_OPTIONAL_OBJECT_REF_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_actor"),
    cmd(0x280d, "ReloadEquippedModels", "ReloadModels", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x280e, "MoveToFadeDelay", "", true, &ONE_OBJECT_REF_ONE_FLOAT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x280f, "RunScriptSnippet", "", false, &ONE_INT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_script"),
    cmd(0x2810, "ScriptWait", "", false, &ONE_INT, NUM, Command, "jip_fn_script"),
    cmd(0x2811, "IsScriptWaiting", "", false, &ONE_OPTIONAL_QUEST, NUM, Command, "jip_fn_script"),
    cmd(0x2812, "StopScriptWaiting", "", false, &ONE_OPTIONAL_QUEST, NUM, Command, "jip_fn_script"),
    cmd(0x2813, "GetCrosshairWater", "", false, &NO_PARAMS, FORM, Command, "jip_fn_misc_ref"),
    cmd(0x2814, "GetScriptBlockDisabled", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_script"),
    cmd(0x2815, "SetScriptBlockDisabled", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_script"),
    cmd(0x2816, "GetPlayedIdle", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x2817, "IsIdlePlayingEx", "", true, &ONE_IDLE_FORM, NUM, Condition, "jip_fn_actor"),
    cmd(0x2818, "GetObjectiveTargets", "", false, &ONE_QUEST_ONE_INT, ARR, Command, "jip_fn_quest"),
    cmd(0x2819, "SetObjectiveNthTarget", "", false, &ONE_QUEST_TWO_INTS_ONE_OBJECT_REF, NUM, Command, "jip_fn_quest"),
    cmd(0x281a, "GetObjectiveHasTarget", "", false, &ONE_QUEST_ONE_INT_ONE_OBJECT_REF, NUM, Command, "jip_fn_quest"),
    cmd(0x281b, "AddObjectiveTarget", "", false, &ONE_QUEST_ONE_INT_ONE_OBJECT_REF, NUM, Command, "jip_fn_quest"),
    cmd(0x281c, "RemoveObjectiveTarget", "", false, &ONE_QUEST_ONE_INT_ONE_OBJECT_REF, NUM, Command, "jip_fn_quest"),
    cmd(0x281d, "GetObjectiveText", "", false, &ONE_QUEST_ONE_INT, STR, Command, "jip_fn_quest"),
    cmd(0x281e, "SetObjectiveText", "", false, &ONE_QUEST_ONE_INT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_quest"),
    cmd(0x281f, "GetArmorAudioTemplate", "", false, &ONE_OBJECT_ID, FORM, Command, "jip_fn_armor"),
    cmd(0x2820, "SetArmorAudioTemplate", "", false, &ONE_OBJECT_ID_ONE_OPTIONAL_OBJECT_ID, NUM, Command, "jip_fn_armor"),
    cmd(0x2821, "SetWeaponOut", "", true, &ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x2822, "SetTerminalUIModel", "", false, &ONE_FORM_ONE_OPTIONAL_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x2823, "AddBaseEffectListEffect", "", false, &ONE_SPELL_ITEM_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x2824, "RemoveBaseEffectListEffect", "", false, &ONE_SPELL_ITEM_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x2825, "ShowQuantityMenu", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x2826, "MessageBoxExAlt", "", false, &ONE_FORM_ONE_FORMAT_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x2827, "GetVATSTargets", "", false, &NO_PARAMS, ARR, Command, "jip_fn_ui"),
    cmd(0x2828, "HasScriptBlock", "", false, &ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_script"),
    cmd(0x2829, "CastImmediate", "", true, &ONE_MAGIC_ITEM_ONE_OPTIONAL_ACTOR, NUM, Command, "jip_fn_effect"),
    cmd(0x282a, "IsInCharGen", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_ui"),
    cmd(0x282b, "DisableScriptedActivate", "", true, &ONE_INT, NUM, Command, "jip_fn_script"),
    cmd(0x282c, "GetHitAttacker", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x282d, "GetHitProjectile", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x282e, "GetHitWeapon", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x282f, "GetGameDaysPassed", "", false, &THREE_OPTIONAL_INTS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2830, "GetWaterImmersionPerc", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x2831, "GetProjectileMuzzleFlash", "", false, &ONE_BOUND_OBJECT, STR, Command, "jip_fn_projectile"),
    cmd(0x2832, "SetProjectileMuzzleFlash", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_projectile"),
    cmd(0x2833, "DismemberLimb", "", true, &ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x2834, "RunBatchScript", "", false, &ONE_STRING, NUM, Command, "jip_fn_script"),
    cmd(0x2835, "IsArmorAddon", "", false, &ONE_OBJECT_ID, NUM, Command, "jip_fn_armor"),
    cmd(0x2836, "ToggleCraftingMessages", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x2837, "ToggleHardcoreTracking", "", false, &ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2838, "SetGameDifficulty", "", false, &ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2839, "GetEquippedItemRef", "", true, &ONE_INT, FORM, Command, "jip_fn_inventory"),
    cmd(0x283a, "ExecuteScript", "", true, &NO_PARAMS, NUM, Command, "jip_fn_script"),
    empty(0x283b, &THREE_OPTIONAL_INTS), // once ReloadENB
    cmd(0x283c, "IsFleeing", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x283d, "GetEnemyHealthTarget", "", false, &NO_PARAMS, FORM, Command, "jip_fn_miscellaneous"),
    cmd(0x283e, "SetCurrentAmmoRounds", "", true, &ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x283f, "IsAnimPlayingEx", "", true, &ONE_INT_THREE_OPTIONAL_INTS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2840, "AttachAshPileEx", "", true, &ONE_FORM, NUM, Command, "jip_fn_actor"),
    cmd(0x2841, "MarkActivatorAshPile", "", false, &ONE_FORM, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2842, "SetActorVelocity", "", true, &ONE_AXIS_ONE_FLOAT, NUM, Command, "jip_fn_actor"),
    cmd(0x2843, "GetRigidBodyMass", "", true, &NO_PARAMS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2844, "PushObject", "", true, &FOUR_FLOATS_ONE_OPTIONAL_OBJECT_REF, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2845, "GetSkeletonModel", "", false, &ONE_ACTOR_BASE, STR, Command, "jip_fn_actor"),
    cmd(0x2846, "SetNPCSkeletonModel", "", false, &ONE_ACTOR_BASE_ONE_OPTIONAL_STRING, NUM, Command, "jip_fn_actor"),
    cmd(0x2847, "SetCRESkeletonModel", "", false, &ONE_ACTOR_BASE_ONE_STRING, NUM, Command, "jip_fn_actor"),
    cmd(0x2848, "SetBodyPartData", "", false, &ONE_ACTOR_BASE_ONE_FORM, NUM, Command, "ln_fn_actor"),
    cmd(0x2849, "GetTargetUnreachable", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x284a, "MoveToContainer", "", true, &ONE_OBJECT_REF_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x284b, "GetNifBlockTranslation", "GetNifTrans", true, &ONE_STRING_TWO_OPTIONAL_INTS, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x284c, "SetNifBlockTranslation", "SetNifTrans", true, &ONE_STRING_THREE_FLOATS_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x284d, "GetNifBlockRotation", "GetNifRot", true, &ONE_STRING_TWO_OPTIONAL_INTS, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x284e, "SetNifBlockRotation", "SetNifRot", true, &ONE_STRING_THREE_FLOATS_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x284f, "GetNifBlockScale", "GetNifScale", true, &ONE_STRING_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2850, "SetNifBlockScale", "SetNifScale", true, &ONE_STRING_ONE_FLOAT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2851, "GetNifBlockFlag", "GetNifFlag", true, &ONE_STRING_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2852, "SetNifBlockFlag", "SetNifFlag", true, &ONE_STRING_TWO_INTS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2853, "GetBufferedCells", "", false, &ONE_INT, ARR, Command, "jip_fn_miscellaneous"),
    cmd(0x2854, "SetVATSTargetable", "", true, &ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x2855, "GetWindDirection", "", false, &NO_PARAMS, NUM, Command, "jip_fn_weather"),
    cmd(0x2856, "SetWindDirection", "", false, &ONE_FLOAT, NUM, Command, "jip_fn_weather"),
    cmd(0x2857, "GetCreatureWeaponList", "", false, &ONE_OPTIONAL_ACTOR_BASE, FORM, Command, "jip_fn_actor"),
    cmd(0x2858, "SetWindSpeedMult", "", false, &ONE_FLOAT, NUM, Command, "jip_fn_weather"),
    cmd(0x2859, "TriggerLightningFX", "", false, &NO_PARAMS, NUM, Command, "jip_fn_weather"),
    cmd(0x285a, "GetObjectVelocity", "", true, &ONE_OPTIONAL_AXIS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x285b, "GetAngularVelocity", "GetAngVel", true, &ONE_STRING_ONE_OPTIONAL_AXIS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x285c, "SetAngularVelocity", "SetAngVel", true, &ONE_STRING_ONE_AXIS_ONE_FLOAT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x285d, "PlaceAtCell", "", false, &ONE_FORM_ONE_INT_ONE_FORM_THREE_FLOATS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x285e, "GetRayCastPos", "", true, &THREE_GLOBALS_ONE_OPT_FLOAT_ONE_OPT_INT_ONE_OPT_STRING_ONE_OPT_FLOAT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x285f, "GetAnimSequenceFrequency", "GetAnimSeqFreq", true, &ONE_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2860, "SetAnimSequenceFrequency", "SetAnimSeqFreq", true, &ONE_STRING_ONE_FLOAT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2861, "StopSound", "", false, &ONE_SOUND_ONE_OPTIONAL_OBJECT_REF, NUM, Command, "jip_fn_sound"),
    cmd(0x2862, "IsMusicPlaying", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_sound"),
    cmd(0x2863, "SetMusicState", "", false, &ONE_INT, NUM, Command, "jip_fn_sound"),
    cmd(0x2864, "GetNoUnequip", "", true, &NO_PARAMS, NUM, Command, "jip_fn_inventory"),
    cmd(0x2865, "SetNoUnequip", "", true, &ONE_INT, NUM, Command, "jip_fn_inventory"),
    cmd(0x2866, "SetCursorPos", "", false, &TWO_FLOATS, NUM, Command, "jip_fn_ui"),
    cmd(0x2867, "MoveToNode", "", true, &ONE_OBJECT_REF_ONE_STRING_THREE_OPTIONAL_FLOATS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2868, "GetDeathItem", "", false, &ONE_OPTIONAL_ACTOR_BASE, FORM, Command, "jip_fn_actor"),
    cmd(0x2869, "SetDeathItem", "", false, &ONE_OBJECT_ID_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x286a, "GetActorLeveledList", "", true, &NO_PARAMS, FORM, Command, "jip_fn_actor"),
    cmd(0x286b, "GetPlayerPerks", "", false, &NO_PARAMS, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x286c, "UnloadUIComponent", "", false, &ONE_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x286d, "GetQuests", "", false, &ONE_OPTIONAL_INT, ARR, Command, "jip_fn_quest"),
    cmd(0x286e, "GetQuestObjectives", "", false, &ONE_QUEST_ONE_INT, ARR, Command, "jip_fn_quest"),
    cmd(0x286f, "GetActiveObjectives", "", false, &NO_PARAMS, ARR, Command, "jip_fn_quest"),
    cmd(0x2870, "GetObjectiveTeleportLinks", "", false, &ONE_QUEST_ONE_INT, ARR, Command, "jip_fn_quest"),
    cmd(0x2871, "GetNifBlockParentNodes", "GetNifParents", true, &ONE_STRING_ONE_OPTIONAL_INT, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x2872, "ClearMessageQueue", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ui"),
    cmd(0x2873, "InitMiniMap", "", false, &NO_PARAMS, NUM, Command, "jip_fn_minimap"),
    cmd(0x2874, "UpdateMiniMap", "", false, &ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_minimap"),
    cmd(0x2875, "GetCrosshairRefEx", "", false, &NO_PARAMS, FORM, Command, "jip_fn_misc_ref"),
    cmd(0x2876, "IsMobile", "", true, &NO_PARAMS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2877, "IsGrabbable", "", true, &NO_PARAMS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2878, "SetOnLocationDiscoverEventHandler", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2879, "AttachLight", "", true, &ONE_STRING_ONE_FORM_THREE_OPTIONAL_FLOATS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x287a, "RemoveLight", "", true, &ONE_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x287b, "SetOnCraftingEventHandler", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x287c, "IsInKillCam", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x287d, "SwapObjectLOD", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x287e, "ResetClouds", "", false, &NO_PARAMS, NUM, Command, "jip_fn_weather"),
    cmd(0x287f, "GetPluginHeaderVersion", "", false, &ONE_STRING, NUM, Command, "jip_fn_utility"),
    cmd(0x2880, "SetOnHitEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_actor"),
    cmd(0x2881, "SetSystemColor", "", false, &FOUR_INTS, NUM, Command, "jip_fn_ui"),
    empty(0x2882, &THREE_OPTIONAL_INTS), // once SetWobblesRotation
    cmd(0x2883, "GetArmourPenetrated", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x2884, "GetImpactMaterialType", "", false, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x2885, "SetImpactMaterialType", "", false, &ONE_INT_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x2886, "GetImpactDataSetForm", "", false, &ONE_FORM_ONE_INT, FORM, Command, "jip_fn_impact_data"),
    cmd(0x2887, "SetImpactDataSetForm", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_impact_data"),
    cmd(0x2888, "SetQuestFlag", "", false, &ONE_QUEST_TWO_INTS, NUM, Command, "jip_fn_quest"),
    cmd(0x2889, "GetQuestFlag", "", false, &ONE_QUEST_ONE_INT, NUM, Command, "jip_fn_quest"),
    cmd(0x288a, "PushActorAwayAlt", "", true, &ONE_FLOAT_THREE_OPTIONAL_FLOATS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x288b, "SetGameHour", "", false, &ONE_FLOAT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x288c, "GetGameVolume", "", false, &ONE_INT, NUM, Command, "jip_fn_sound"),
    cmd(0x288d, "SetGameVolume", "", false, &ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_sound"),
    cmd(0x288e, "MoveAwayFrom", "", true, &ONE_OBJECT_REF_ONE_FLOAT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x288f, "GetEquippedWeaponPoison", "", true, &NO_PARAMS, FORM, Command, "jip_fn_inventory"),
    cmd(0x2890, "SuppressQuestMessages", "", false, &NO_PARAMS, NUM, Command, "jip_fn_ui"),
    cmd(0x2891, "TravelToRef", "", true, &ONE_OBJECT_REF_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x2892, "GetIsLAA", "", false, &NO_PARAMS, NUM, Command, "jip_fn_utility"),
    cmd(0x2893, "fTan", "", false, &ONE_FLOAT, NUM, Command, "ln_fn_math"),
    cmd(0x2894, "fAtan2", "", false, &TWO_FLOATS, NUM, Command, "ln_fn_math"),
    cmd(0x2895, "DonnerReedKuruParty", "TheOtherOtherWhiteMeat", true, &ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x2896, "SetArmorClass", "", false, &ONE_OBJECT_ID_ONE_INT, NUM, Command, "jip_fn_armor"),
    cmd(0x2897, "GetEquippedEx", "", true, &FORM_LIST, FORM, Command, "jip_fn_actor"),
    cmd(0x2898, "TestEquippedSlots", "", true, &ONE_OBJECT_ID, NUM, Command, "jip_fn_actor"),
    cmd(0x2899, "GetWeaponOnHitSetting", "", false, &ONE_OBJECT_ID, NUM, Command, "jip_fn_weapon"),
    cmd(0x289a, "SetWeaponOnHitSetting", "", false, &ONE_OBJECT_ID_ONE_INT, NUM, Command, "jip_fn_weapon"),
    cmd(0x289b, "GetWeaponSemiAutoFireDelay", "", false, &ONE_OBJECT_ID_ONE_INT, NUM, Command, "jip_fn_weapon"),
    cmd(0x289c, "SetWeaponSemiAutoFireDelay", "", false, &ONE_OBJECT_ID_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_weapon"),
    cmd(0x289d, "IsStickDisabled", "", false, &ONE_INT, NUM, Condition, "ln_fn_gamepad"),
    cmd(0x289e, "SetStickDisabled", "", false, &TWO_INTS, NUM, Command, "ln_fn_gamepad"),
    empty(0x289f, &THREE_OPTIONAL_INTS), // once Sleep
    cmd(0x28a0, "GetActiveIMODs", "", false, &NO_PARAMS, ARR, Command, "jip_fn_imagespace"),
    cmd(0x28a1, "ToggleItemUnique", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_inventory"),
    cmd(0x28a2, "SetTextInputExtendedProps", "", false, &TWO_FLOATS_THREE_OPTIONAL_INTS, NUM, Command, "jip_fn_ui"),
    cmd(0x28a3, "SetTextInputString", "", false, &FORMAT_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x28a4, "ClickMenuTile", "", false, &ONE_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x28a5, "StringToActorValue", "", false, &ONE_STRING, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28a6, "GetFactions", "", false, &ONE_OPTIONAL_ACTOR_BASE, ARR, Command, "jip_fn_actor"),
    cmd(0x28a7, "GetBaseItems", "", false, &ONE_OPTIONAL_FORM, ARR, Command, "jip_fn_inventory"),
    cmd(0x28a8, "GetHit3DData", "", true, &ONE_INT, ARR, Command, "jip_fn_actor"),
    cmd(0x28a9, "GetWeaponModel", "", false, &ONE_OBJECT_ID_ONE_INT, STR, Command, "jip_fn_weapon"),
    cmd(0x28aa, "SetWeaponModel", "", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "jip_fn_weapon"),
    cmd(0x28ab, "GetOwnerOfCell", "", false, &ONE_FORM, FORM, Command, "jip_fn_cell"),
    cmd(0x28ac, "TogglePipBoyLight", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x28ad, "GetHardcoreTracking", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x28ae, "SetPerkLevel", "", false, &ONE_PERK_ONE_INT, NUM, Command, "ln_fn_perk"),
    cmd(0x28af, "GetProjectileRefImpactRef", "", true, &NO_PARAMS, FORM, Command, "jip_fn_projectile"),
    cmd(0x28b0, "SetOnProjectileImpactEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "jip_fn_projectile"),
    cmd(0x28b1, "GetNoteRead", "", false, &ONE_NOTE, NUM, Condition, "jip_fn_miscellaneous"),
    cmd(0x28b2, "SetOnNoteAddedEventHandler", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28b3, "SetOnUseAidItemEventHandler", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "jip_fn_inventory"),
    cmd(0x28b4, "ToggleVanityWheel", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_input"),
    cmd(0x28b5, "ToggleHUDCursor", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x28b6, "GetHardcoreStageThreshold", "", false, &ONE_FORM, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28b7, "SetHardcoreStageThreshold", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28b8, "GetHardcoreStageEffect", "", false, &ONE_FORM, FORM, Command, "jip_fn_miscellaneous"),
    cmd(0x28b9, "SetHardcoreStageEffect", "", false, &ONE_FORM_ONE_OPTIONAL_SPELL_ITEM, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28ba, "ClearModelAltTextures", "", false, &ONE_OBJECT_ID, NUM, Command, "jip_fn_armor"),
    cmd(0x28bb, "GetCreatureSoundsTemplate", "", false, &ONE_ACTOR_BASE, FORM, Command, "jip_fn_actor"),
    cmd(0x28bc, "SetCreatureSoundsTemplate", "", false, &ONE_FORM_ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "jip_fn_actor"),
    cmd(0x28bd, "GetDebugModeState", "", false, &ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28be, "GetProjectileRefImpactMaterial", "", true, &NO_PARAMS, NUM, Command, "jip_fn_projectile"),
    cmd(0x28bf, "FreezeTime", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28c0, "GetConditionDamagePenalty", "", false, &NO_PARAMS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28c1, "SetConditionDamagePenalty", "", false, &ONE_FLOAT, NUM, Command, "jip_fn_miscellaneous"),
    empty(0x28c2, &THREE_OPTIONAL_INTS), // once GetExtraFloat
    empty(0x28c3, &THREE_OPTIONAL_INTS), // once SetExtraFloat
    cmd(0x28c4, "ToggleBipedSlotVisibility", "", false, &ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28c5, "SetOnReloadWeaponEventHandler", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x28c6, "AddTileFromTemplate", "", false, &FORMAT_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x28c7, "UpdatePlayer3D", "", false, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x28c8, "GetArrayValue", "", false, &TWO_INTS, AMB, Command, "jip_fn_utility"),
    cmd(0x28c9, "SetOnRagdollEventHandler", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x28ca, "PushActorNoRagdoll", "", true, &TWO_FLOATS_ONE_OPTIONAL_OBJECT_REF_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x28cb, "SetUIFloatGradual", "", false, &ONE_STRING_THREE_OPTIONAL_FLOATS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x28cc, "CloseActiveMenu", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x28cd, "ReloadCloudTextures", "", false, &NO_PARAMS, NUM, Command, "jip_fn_weather"),
    cmd(0x28ce, "ToggleImmortalMode", "TIMM", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28cf, "GetBodyPartVATSTarget", "", false, &ONE_INT_ONE_OPTIONAL_ACTOR_BASE, STR, Command, "jip_fn_actor"),
    cmd(0x28d0, "SetLinearVelocity", "", true, &ONE_STRING_THREE_FLOATS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28d1, "GetEquippedArmorRefs", "", true, &NO_PARAMS, ARR, Command, "jip_fn_inventory"),
    cmd(0x28d2, "GetHitNode", "", true, &NO_PARAMS, STR, Command, "jip_fn_actor"),
    cmd(0x28d3, "InsertNode", "", false, &ONE_FORM_ONE_INT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28d4, "AttachModel", "", false, &ONE_FORM_ONE_INT_ONE_FORMAT_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28d5, "SynchronizePosition", "SyncPos", false, &ONE_OPTIONAL_OBJECT_REF_ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT_ONE_OPTIONAL_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28d6, "ToggleCameraCollision", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28d7, "InitRockItLauncher", "", false, &ONE_OBJECT_ID, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28d8, "ForceClimate", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "jip_fn_climate"),
    cmd(0x28d9, "GetInFactionList", "", true, &FORM_LIST, NUM, Condition, "jip_fn_actor"),
    cmd(0x28da, "LeveledListHasFormDeep", "LListHasFormDeep", false, &TWO_FORMS, NUM, Command, "ln_fn_leveled_list"),
    cmd(0x28db, "GetActorTiltAngle", "", true, &ONE_AXIS, NUM, Command, "jip_fn_actor"),
    cmd(0x28dc, "SetActorTiltAngle", "", true, &ONE_AXIS_ONE_FLOAT, NUM, Command, "jip_fn_actor"),
    cmd(0x28dd, "ReloadCharController", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x28de, "GetRandomInRange", "", false, &TWO_INTS, NUM, Command, "jip_fn_utility"),
    cmd(0x28df, "ModelHasBlock", "", false, &ONE_FORM_ONE_FORMAT_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28e0, "GetGroundMaterial", "", true, &NO_PARAMS, NUM, Condition, "jip_fn_actor"),
    cmd(0x28e1, "ShowLevelUpMenuEx", "", false, &ONE_INT, NUM, Command, "jip_fn_ui"),
    cmd(0x28e2, "GetRayCastRef", "", true, &ONE_OPTIONAL_INT_ONE_OPTIONAL_STRING, FORM, Command, "jip_fn_misc_ref"),
    cmd(0x28e3, "GetRayCastMaterial", "", true, &ONE_OPTIONAL_INT_ONE_OPTIONAL_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28e4, "FireWeaponEx", "", true, &ONE_OBJECT_ID_ONE_OPTIONAL_STRING, NUM, Command, "jip_fn_actor"),
    cmd(0x28e5, "GetArmorEffectiveDT", "", true, &NO_PARAMS, NUM, Command, "jip_fn_inventory"),
    cmd(0x28e6, "GetArmorEffectiveDR", "", true, &NO_PARAMS, NUM, Command, "jip_fn_inventory"),
    cmd(0x28e7, "GetActorGravityMult", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x28e8, "SetActorGravityMult", "", true, &ONE_FLOAT, NUM, Command, "jip_fn_actor"),
    cmd(0x28e9, "ToggleHitEffects", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28ea, "ToggleNoMovementCombat", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28eb, "ToggleTeammateKillable", "SetTeammateKillable", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x28ec, "ToggleNoGunWobble", "SetNoGunWobble", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x28ed, "RewardXPExact", "", false, &ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28ee, "GetHitExtendedFlag", "", true, &ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x28ef, "ClearDeadActors", "", false, &NO_PARAMS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28f0, "GetCameraMovement", "", false, &TWO_SCRIPT_VARS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x28f1, "GetHotkeyItemRef", "", false, &ONE_INT, FORM, Command, "jip_fn_inventory"),
    cmd(0x28f2, "GetCollisionNodes", "", true, &NO_PARAMS, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x28f3, "GetChildBlocks", "", true, &ONE_OPTIONAL_STRING_TWO_OPTIONAL_INTS, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x28f4, "GetBlockTextureSet", "", true, &ONE_STRING, ARR, Command, "jip_fn_misc_ref"),
    cmd(0x28f5, "GetReticleNode", "", false, &ONE_OPTIONAL_FLOAT_ONE_OPTIONAL_INT, STR, Command, "jip_fn_miscellaneous"),
    empty(0x28f6, &THREE_OPTIONAL_INTS),
    cmd(0x28f7, "EquippedWeaponHasModType", "", true, &ONE_INT, NUM, Condition, "jip_fn_weapon"),
    cmd(0x28f8, "GetPosEx", "", true, &THREE_SCRIPT_VARS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28f9, "GetAngleEx", "", true, &THREE_SCRIPT_VARS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28fa, "ToggleMouseMovement", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_input"),
    cmd(0x28fb, "SetTextureTransformKey", "SetTexTransKey", true, &ONE_STRING_TWO_INTS_ONE_FLOAT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28fc, "AttachExtraCamera", "AttachCam", true, &ONE_STRING_ONE_INT_ONE_OPTIONAL_STRING_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28fd, "ProjectExtraCamera", "ProjCam", false, &TWO_STRINGS_ONE_FLOAT_THREE_OPTIONAL_INTS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x28fe, "AttachUIXML", "", true, &TWO_STRINGS, NUM, Command, "jip_fn_ui"),
    cmd(0x28ff, "AttachUIComponent", "", true, &ONE_STRING_ONE_FORMAT_STRING, NUM, Command, "jip_fn_ui"),
    cmd(0x2900, "RenameNifBlock", "", true, &TWO_STRINGS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2901, "RemoveNifBlock", "", true, &ONE_STRING_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2902, "SetInternalMarker", "", false, &ONE_FORM_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2903, "FailQuest", "", false, &ONE_QUEST, NUM, Command, "jip_fn_quest"),
    cmd(0x2904, "PlayAnimSequence", "", true, &ONE_STRING_ONE_OPTIONAL_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2905, "RemoveAllPerks", "", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x2906, "GetPointRayCastPos", "", false, &FIVE_FLOATS_THREE_SCRIPT_VARS_ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2907, "TogglePlayerSneaking", "", false, &ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2908, "GetActorMovementFlags", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x2909, "GetTranslatedPos", "", true, &THREE_FLOATS_THREE_SCRIPT_VARS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x290a, "GetHitBaseWeaponDamage", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x290b, "GetHitFatigueDamage", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x290c, "RefreshAnimData", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x290d, "GetNifBlockTranslationAlt", "GetNifTransAlt", true, &ONE_STRING_THREE_SCRIPT_VARS_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x290e, "GetNifBlockRotationAlt", "GetNifRotAlt", true, &ONE_STRING_THREE_SCRIPT_VARS_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x290f, "GetLinearVelocityAlt", "GetLinVelAlt", true, &ONE_STRING_THREE_SCRIPT_VARS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2910, "GetAngularVelocityAlt", "GetAngVelAlt", true, &ONE_STRING_THREE_SCRIPT_VARS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2911, "SetAngularVelocityEx", "SetAngVelEx", true, &ONE_STRING_THREE_FLOATS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2912, "GetActorVelocityAlt", "", true, &THREE_SCRIPT_VARS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x2913, "GetCollisionObjProperty", "GetCollProp", true, &ONE_STRING_ONE_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2914, "SetCollisionObjProperty", "SetCollProp", true, &ONE_STRING_ONE_INT_ONE_FLOAT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2915, "GetCollisionObjLayerType", "GetCollLayer", true, &ONE_STRING, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2916, "SetCollisionObjLayerType", "SetCollLayer", true, &ONE_STRING_ONE_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2917, "SetRefrModelPath", "SetRefModel", true, &ONE_OPTIONAL_STRING, NUM, Command, "jip_fn_misc_ref"),
    empty(0x2918, &THREE_OPTIONAL_INTS), // once PlaceModel
    cmd(0x2919, "AttachLine", "", true, &TWO_STRINGS_FIVE_FLOATS, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x291a, "IsSpellTargetList", "", true, &FORM_LIST, NUM, Condition, "jip_fn_effect"),
    cmd(0x291b, "GetExcludedCombatActions", "", true, &NO_PARAMS, NUM, Command, "jip_fn_actor"),
    cmd(0x291c, "SetExcludedCombatActions", "", true, &ONE_INT, NUM, Command, "jip_fn_actor"),
    cmd(0x291d, "SetArmorConditionPenalty", "", false, &ONE_OPTIONAL_FLOAT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x291e, "PlaySound3DNode", "", true, &ONE_SOUND_ONE_STRING, NUM, Command, "jip_fn_sound"),
    cmd(0x291f, "GetReticlePosAlt", "", false, &THREE_SCRIPT_VARS_ONE_OPTIONAL_INT_ONE_OPTIONAL_FLOAT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2920, "GetLightAmountAtPoint", "", false, &THREE_FLOATS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2921, "TransformWorldToLocal", "", false, &NINE_FLOATS_TWO_SCRIPT_VARS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2922, "GetAnglesBetweenPoints", "", false, &SIX_FLOATS_TWO_SCRIPT_VARS, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2923, "GetP2PRayCastRange", "", false, &SIX_FLOATS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2924, "WeaponHasModType", "", false, &ONE_FORM_ONE_INT, NUM, Command, "jip_fn_weapon"),
    cmd(0x2925, "ProjectUITile", "", true, &TWO_STRINGS_FOUR_FLOATS, NUM, Command, "jip_fn_ui"),
    cmd(0x2926, "RotateAroundPoint", "", true, &SIX_FLOATS_ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2927, "GetStringUIDimensions", "", false, &ONE_STRING_ONE_INT_ONE_FLOAT_THREE_SCRIPT_VARS, NUM, Command, "jip_fn_ui"),
    cmd(0x2928, "ToggleNodeCollision", "", true, &ONE_STRING_ONE_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2929, "GetAllPerks", "", true, &TWO_OPTIONAL_INTS, ARR, Command, "jip_fn_actor"),
    cmd(0x292a, "GetEditorPosition", "", true, &THREE_SCRIPT_VARS_THREE_OPTIONAL_SCRIPT_VARS, FORM, Command, "jip_fn_misc_ref"),
    cmd(0x292b, "GetWorldspacePersistentCell", "", false, &ONE_WORLDSPACE, FORM, Command, "jip_fn_worldspace"),
    cmd(0x292c, "ScrollMouseWheel", "", false, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_input"),
    cmd(0x292d, "GetCellNorthRotation", "", false, &ONE_FORM, NUM, Command, "jip_fn_cell"),
    cmd(0x292e, "RefHasExtraData", "", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x292f, "GetRefExtraData", "", true, &NVSE_ONE_NUM_ONE_OPTIONAL_NUM, AMB, Expression, "jip_fn_misc_ref"),
    cmd(0x2930, "SetRefExtraData", "", true, &NVSE_ONE_OPTIONAL_NUM_ONE_OPTIONAL_BASIC_TYPE, NUM, Expression, "jip_fn_misc_ref"),
    cmd(0x2931, "TogglePurgeOnUnload", "TPOU", true, &ONE_OPTIONAL_INT, NUM, Command, "jip_fn_misc_ref"),
    cmd(0x2932, "GetProjectileLight", "", false, &ONE_BOUND_OBJECT, FORM, Command, "jip_fn_projectile"),
    cmd(0x2933, "SetProjectileLight", "", false, &ONE_BOUND_OBJECT_ONE_OPTIONAL_BOUND_OBJECT, NUM, Command, "jip_fn_projectile"),
    cmd(0x2934, "IsItemUnique", "", false, &ONE_OBJECT_ID, NUM, Command, "jip_fn_inventory"),
    cmd(0x2935, "GetMenuItemListRefs", "", false, &TWO_OPTIONAL_INTS, ARR, Command, "jip_fn_ui"),
    cmd(0x2936, "GetSelfAsInventoryRef", "", true, &NO_PARAMS, FORM, Command, "jip_fn_inventory"),
    cmd(0x2937, "SetDamageToArmorMaxPercent", "DTAMP", false, &ONE_OPTIONAL_FLOAT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x2938, "RegisterSRScript", "", false, &ONE_STRING, NUM, Command, "jip_fn_script"),
    cmd(0x2939, "SetAmmoCasing", "", false, &ONE_OBJECT_ID_ONE_OPTIONAL_OBJECT_ID, NUM, Command, "jip_fn_ammo"),
    cmd(0x293a, "AssignKeyword", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x293b, "HasKeyword", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x293c, "GetKeywordForms", "", false, &ONE_STRING, ARR, Command, "jip_fn_miscellaneous"),
    cmd(0x293d, "ToggleDepthClear", "", false, &ONE_INT, NUM, Command, "jip_fn_miscellaneous"),
    cmd(0x293e, "HasKeywordCond", "", true, &ONE_INT, NUM, ConditionOnly, "jip_fn_miscellaneous"),
    cmd(0x293f, "GetStringHash", "GetHash", false, &ONE_STRING_TWO_OPTIONAL_INTS, NUM, Command, "jip_fn_string"),
    cmd(0x2940, "CalculateShotsPerSec", "", true, &ONE_FORM, NUM, Command, "jip_fn_weapon"),
    cmd(0x2941, "IsAttackQueued", "", false, &NO_PARAMS, NUM, Condition, "jip_fn_weapon"),
    cmd(0x2942, "GetReticleTargetLimb", "", false, &ONE_OPTIONAL_FLOAT, NUM, Command, "jip_fn_miscellaneous"),
];
