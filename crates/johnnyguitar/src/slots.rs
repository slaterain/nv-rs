//! JohnnyGuitar NVSE's 408 command slots, as its 5.28 source (`c87905e`) registers them:
//! `JG/internal/JG/JohnnyCommands.cpp`'s `Init` (the opcode base `0x3100`, one `REG_CMD` or
//! `REG_TYPED_CMD` line per slot with its return type), each command's `DEFINE_*` line in
//! `JG/functions/fn_*.h` (its alias, whether it needs a reference, its parameter list, whether it
//! has a condition eval or xNVSE's expression parser) and the parameter lists of its copy of
//! `nvse/ParamInfos.h`. Data, not code: generated from the source by a script kept in the lane's
//! notes (`gen-jg-slots.pl`) and left as generated (`#[rustfmt::skip]` on the module). Do not edit
//! by hand. 408 slots, none empty.

use crate::table::Kind::{Command, Condition, Expression};
use crate::table::{cmd, Params, Slot, ARR, FORM, NUM, STR};
use nvse::table::Param;

/// The parameter types the lists use (`ParamType`, `nvse/CommandTable.h` (JohnnyGuitar NVSE)).
mod ty {
    pub const STRING: u32 = 0x00;
    pub const INTEGER: u32 = 0x01;
    pub const FLOAT: u32 = 0x02;
    pub const OBJECT_ID: u32 = 0x03;
    pub const OBJECT_REF: u32 = 0x04;
    pub const ACTOR_VALUE: u32 = 0x05;
    pub const TOPIC: u32 = 0x0d;
    pub const QUEST: u32 = 0x0e;
    pub const GLOBAL: u32 = 0x13;
    pub const TES_OBJECT: u32 = 0x15;
    pub const ACTOR_BASE: u32 = 0x19;
    pub const WORLD_SPACE: u32 = 0x1b;
    pub const WEATHER_ID: u32 = 0x21;
    pub const FORM_LIST: u32 = 0x25;
    pub const IMAGE_SPACE_MODIFIER: u32 = 0x2a;
    pub const SCRIPT_VARIABLE: u32 = 0x2d;
    pub const ANY_FORM: u32 = 0x3d;
    pub const REPUTATION: u32 = 0x3e;
    pub const CASINO: u32 = 0x3f;
    pub const NVSE_NUMBER: u32 = 0x8001;
    pub const NVSE_BOOLEAN: u32 = 0x8002;
    pub const NVSE_FORM: u32 = 0x8008;
    pub const NVSE_ARRAY: u32 = 0x8040;
}

/// A parameter JohnnyGuitar lists as needed: its name (`typeStr`) and type.
const fn req(name: &'static str, kind: u32) -> Param {
    Param { name, kind, optional: false }
}

/// A parameter JohnnyGuitar lists as optional.
const fn opt(name: &'static str, kind: u32) -> Param {
    Param { name, kind, optional: true }
}

/// The parameter lists the slots use, by their names in `nvse/ParamInfos.h`; `nullptr` is no list
/// and no count.
pub static NO_PARAMS: Params = Params::new("nullptr", &[]);
pub static ONE_INT_THREE_SCRIPT_VARS: Params = Params::new("kParams_OneInt_ThreeScriptVars", &[req("int", ty::INTEGER), req("int", ty::SCRIPT_VARIABLE), req("int", ty::SCRIPT_VARIABLE), req("int", ty::SCRIPT_VARIABLE)]);
pub static ONE_INT: Params = Params::new("kParams_OneInt", &[req("int", ty::INTEGER)]);
pub static TWO_INTS: Params = Params::new("kParams_TwoInts", &[req("int", ty::INTEGER), req("int", ty::INTEGER)]);
pub static ONE_REF_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneRef_OneOptionalInt", &[req("target", ty::OBJECT_REF), opt("int", ty::INTEGER)]);
pub static ONE_OPTIONAL_INT: Params = Params::new("kParams_OneOptionalInt", &[opt("int", ty::INTEGER)]);
pub static ONE_INT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneInt_OneOptionalInt", &[req("int", ty::INTEGER), opt("int", ty::INTEGER)]);
pub static ONE_FLOAT: Params = Params::new("kParams_OneFloat", &[req("float", ty::FLOAT)]);
pub static ONE_STRING: Params = Params::new("kParams_OneString", &[req("string", ty::STRING)]);
pub static ONE_STRING_ONE_OPTIONAL_FLOAT: Params = Params::new("kParams_OneString_OneOptionalFloat", &[req("string", ty::STRING), opt("float", ty::FLOAT)]);
pub static ONE_STRING_ONE_OPTIONAL_OBJECT_REF: Params = Params::new("kParams_OneString_OneOptionalObjectRef", &[req("string", ty::STRING), opt("ref", ty::OBJECT_REF)]);
pub static ONE_STRING_ONE_OPTIONAL_FLOAT_THREE_OPTIONAL_INTS: Params = Params::new("kParams_OneString_OneOptionalFloat_ThreeOptionalInts", &[req("string", ty::STRING), opt("float", ty::FLOAT), opt("integer", ty::INTEGER), opt("integer", ty::INTEGER), opt("integer", ty::INTEGER)]);
pub static ONE_STRING_ONE_OPTIONAL_FLOAT_FOUR_OPTIONAL_INTS: Params = Params::new("kParams_OneString_OneOptionalFloat_FourOptionalInts", &[req("string", ty::STRING), opt("float", ty::FLOAT), opt("integer", ty::INTEGER), opt("integer", ty::INTEGER), opt("integer", ty::INTEGER), opt("integer", ty::INTEGER)]);
pub static TWO_FLOATS: Params = Params::new("kParams_TwoFloats", &[req("float", ty::FLOAT), req("float", ty::FLOAT)]);
pub static ONE_OBJECT_ID: Params = Params::new("kParams_OneObjectID", &[req("item", ty::OBJECT_ID)]);
pub static ONE_OPTIONAL_OBJECT_ID: Params = Params::new("kParams_OneOptionalObjectID", &[opt("item", ty::OBJECT_ID)]);
pub static ONE_ACTOR_VALUE: Params = Params::new("kParams_OneActorValue", &[req("actor value", ty::ACTOR_VALUE)]);
pub static ONE_STRING_ONE_OPTIONAL_OBJECT: Params = Params::new("kParams_OneString_OneOptionalObject", &[req("string", ty::STRING), opt("item", ty::TES_OBJECT)]);
pub static ONE_QUEST: Params = Params::new("kParams_OneQuest", &[req("quest", ty::QUEST)]);
pub static ONE_FORM_ONE_INT: Params = Params::new("kParams_OneForm_OneInt", &[req("form", ty::ANY_FORM), req("int", ty::INTEGER)]);
pub static TWO_FORMS_ONE_OPTIONAL_FLOAT: Params = Params::new("kParams_TwoForms_OneOptionalFloat", &[req("form", ty::ANY_FORM), req("form", ty::ANY_FORM), opt("float", ty::FLOAT)]);
pub static ONE_FORM: Params = Params::new("kParams_OneForm", &[req("form", ty::ANY_FORM)]);
pub static ONE_FORM_ONE_FLOAT: Params = Params::new("kParams_OneForm_OneFloat", &[req("form", ty::ANY_FORM), req("float", ty::FLOAT)]);
pub static ONE_FORM_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneForm_OneOptionalForm", &[req("form", ty::ANY_FORM), opt("form", ty::ANY_FORM)]);
pub static ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneOptionalForm", &[opt("form", ty::ANY_FORM)]);
pub static ONE_OPTIONAL_ACTOR_BASE: Params = Params::new("kParams_OneOptionalActorBase", &[opt("base actor", ty::ACTOR_BASE)]);
pub static ONE_REPUTATION: Params = Params::new("kParams_OneReputation", &[req("Reputation", ty::REPUTATION)]);
pub static GET_REPUTATION_FORM_ICON: Params = Params::new("kParams_GetReputationFormIcon", &[req("Reputation", ty::REPUTATION), req("Icon Type", ty::INTEGER)]);
pub static SET_REPUTATION_FORM_ICON: Params = Params::new("kParams_SetReputationFormIcon", &[req("Reputation", ty::REPUTATION), req("Icon Type", ty::INTEGER), req("Icon Path", ty::STRING)]);
pub static ONE_STRING_THREE_FLOATS: Params = Params::new("kParams_OneString_ThreeFloats", &[req("String", ty::STRING), req("float", ty::FLOAT), req("float", ty::FLOAT), req("float", ty::FLOAT)]);
pub static FOUR_STRINGS: Params = Params::new("kParams_FourStrings", &[req("String", ty::STRING), req("X_Out", ty::STRING), req("Y_Out", ty::STRING), req("Z_Out", ty::STRING)]);
pub static TWO_REFS_TWO_STRINGS: Params = Params::new("kParams_TwoRefs_TwoStrings", &[req("ref", ty::OBJECT_REF), req("ref", ty::OBJECT_REF), req("String", ty::STRING), req("String", ty::STRING)]);
pub static ONE_FORM_ONE_STRING: Params = Params::new("kParams_OneForm_OneString", &[req("Form", ty::ANY_FORM), req("String", ty::STRING)]);
pub static ONE_STRING_TWO_OPTIONAL_INTS: Params = Params::new("kParams_OneString_TwoOptionalInts", &[req("String", ty::STRING), opt("Integer", ty::INTEGER), opt("Integer", ty::INTEGER)]);
pub static TWO_OPTIONAL_INTS: Params = Params::new("kParams_TwoOptionalInts", &[opt("Integer", ty::INTEGER), opt("Integer", ty::INTEGER)]);
pub static TWO_STRINGS_ONE_OPTIONAL_STRING_FOUR_OPTIONAL_INTS: Params = Params::new("kParams_TwoStrings_OneOptionalString_FourOptionalInts", &[req("String", ty::STRING), req("String", ty::STRING), opt("String", ty::STRING), opt("Integer", ty::INTEGER), opt("Integer", ty::INTEGER), opt("Integer", ty::INTEGER), opt("Integer", ty::INTEGER)]);
pub static ONE_STRING_ONE_INT: Params = Params::new("kParams_OneString_OneInt", &[req("string", ty::STRING), req("Integer", ty::INTEGER)]);
pub static ONE_STRING_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_OneOptionalInt", &[req("string", ty::STRING), opt("Integer", ty::INTEGER)]);
pub static ONE_STRING_THREE_OPTIONAL_INTS: Params = Params::new("kParams_OneString_ThreeOptionalInts", &[req("string", ty::STRING), opt("Integer", ty::INTEGER), opt("Integer", ty::INTEGER), opt("Integer", ty::INTEGER)]);
pub static EVENT_ONE_FORM: Params = Params::new("kParams_Event_OneForm", &[req("setOrRemove", ty::INTEGER), req("Script", ty::ANY_FORM), opt("flags", ty::INTEGER), opt("Form", ty::ANY_FORM)]);
pub static EVENT_TWO_FORMS: Params = Params::new("kParams_Event_TwoForms", &[req("setOrRemove", ty::INTEGER), req("Script", ty::ANY_FORM), opt("flags", ty::INTEGER), opt("Form", ty::ANY_FORM), opt("Form", ty::ANY_FORM)]);
pub static EVENT: Params = Params::new("kParams_Event", &[req("setOrRemove", ty::INTEGER), req("Script", ty::ANY_FORM), opt("flags", ty::INTEGER)]);
pub static EVENT_OPTIONAL_FLAG: Params = Params::new("kParams_Event_OptionalFlag", &[req("setOrRemove", ty::INTEGER), req("Script", ty::ANY_FORM), opt("flags", ty::INTEGER), opt("optionalFlags", ty::INTEGER)]);
pub static EVENT_ONE_FORM_ONE_INT: Params = Params::new("kParams_Event_OneForm_OneInt", &[req("setOrRemove", ty::INTEGER), req("Script", ty::ANY_FORM), opt("flags", ty::INTEGER), opt("Form", ty::ANY_FORM), opt("Integer", ty::INTEGER)]);
pub static EVENT_ONE_INT: Params = Params::new("kParams_Event_OneInt", &[req("setOrRemove", ty::INTEGER), req("Script", ty::ANY_FORM), opt("flags", ty::INTEGER), opt("Integer", ty::INTEGER)]);
pub static ONE_STRING_ONE_FORM_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneString_OneForm_OneOptionalInt", &[req("String", ty::STRING), req("Form", ty::ANY_FORM), opt("Integer", ty::INTEGER)]);
pub static ONE_FORM_TWO_INTS_ONE_FORM: Params = Params::new("kParams_OneForm_TwoInts_OneForm", &[req("Form", ty::ANY_FORM), req("Integer", ty::INTEGER), req("Integer", ty::INTEGER), req("Form", ty::ANY_FORM)]);
pub static ONE_FORM_ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneForm_OneInt_OneFloat", &[req("Form", ty::ANY_FORM), req("Integer", ty::INTEGER), req("Float", ty::FLOAT)]);
pub static ONE_WORLD_SPACE: Params = Params::new("kParams_OneWorldSpace", &[req("WorldSpace", ty::WORLD_SPACE)]);
pub static ONE_FORM_ONE_INT_ONE_FORM: Params = Params::new("kParams_OneForm_OneInt_OneForm", &[req("Form", ty::ANY_FORM), req("Integer", ty::INTEGER), req("Form", ty::ANY_FORM)]);
pub static ONE_INT_ONE_FORM_LIST_ONE_INT: Params = Params::new("kParams_OneInt_OneFormList_OneInt", &[req("Integer", ty::INTEGER), req("Form List", ty::FORM_LIST), req("Integer", ty::INTEGER)]);
pub static ONE_FORM_ONE_INT_ONE_STRING: Params = Params::new("kParams_OneForm_OneInt_OneString", &[req("Form", ty::ANY_FORM), req("Integer", ty::INTEGER), req("String", ty::STRING)]);
pub static ONE_OPTIONAL_FLOAT: Params = Params::new("kParams_OneOptionalFloat", &[opt("Float", ty::FLOAT)]);
pub static THREE_FLOATS: Params = Params::new("kParams_ThreeFloats", &[req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT)]);
pub static THREE_FLOATS_ONE_INT_ONE_OPTIONAL_FLOAT: Params = Params::new("kParams_ThreeFloats_OneInt_OneOptionalFloat", &[req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Integer", ty::INTEGER), opt("Float", ty::FLOAT)]);
pub static FIVE_FLOATS: Params = Params::new("kParams_FiveFloats", &[req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT)]);
pub static ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneInt_OneFloat", &[req("int", ty::INTEGER), req("Float", ty::FLOAT)]);
pub static ONE_FORM_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneForm_OneOptionalInt", &[req("form", ty::ANY_FORM), opt("int", ty::INTEGER)]);
pub static ONE_FORM_ONE_INT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneForm_OneInt_OneOptionalInt", &[req("form", ty::ANY_FORM), req("int", ty::INTEGER), opt("int", ty::INTEGER)]);
pub static ONE_FORM_TWO_INTS: Params = Params::new("kParams_OneForm_TwoInts", &[req("form", ty::ANY_FORM), req("int", ty::INTEGER), req("int", ty::INTEGER)]);
pub static ONE_FORM_TWO_INTS_ONE_STRING: Params = Params::new("kParams_OneForm_TwoInts_OneString", &[req("form", ty::ANY_FORM), req("int", ty::INTEGER), req("int", ty::INTEGER), req("string", ty::STRING)]);
pub static SIX_FLOATS: Params = Params::new("kParams_SixFloats", &[req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT)]);
pub static ONE_FORM_THREE_INTS: Params = Params::new("kParams_OneForm_ThreeInts", &[req("form", ty::ANY_FORM), req("int", ty::INTEGER), req("int", ty::INTEGER), req("int", ty::INTEGER)]);
pub static ONE_FORM_FOUR_INTS_TWO_OPTIONAL_FORMS_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneForm_FourInts_TwoOptionalForms_OneOptionalInt", &[req("form", ty::ANY_FORM), req("int", ty::INTEGER), req("int", ty::INTEGER), req("int", ty::INTEGER), req("int", ty::INTEGER), opt("form", ty::ANY_FORM), opt("form", ty::ANY_FORM), opt("int", ty::INTEGER)]);
pub static TWO_FORMS: Params = Params::new("kParams_TwoForms", &[req("form", ty::ANY_FORM), req("form", ty::ANY_FORM)]);
pub static ONE_STRING_ONE_INT_ONE_FORM: Params = Params::new("kParams_OneString_OneInt_OneForm", &[req("string", ty::STRING), req("int", ty::INTEGER), req("form", ty::ANY_FORM)]);
pub static ONE_IMOD: Params = Params::new("kParams_OneIMOD", &[req("IMOD", ty::IMAGE_SPACE_MODIFIER)]);
pub static ONE_IMOD_ONE_INT: Params = Params::new("kParams_OneIMOD_OneInt", &[req("IMOD", ty::IMAGE_SPACE_MODIFIER), req("int", ty::INTEGER)]);
pub static ONE_ACTOR_VALUE_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneActorValue_OneOptionalInt", &[req("AV", ty::ACTOR_VALUE), opt("Integer", ty::INTEGER)]);
pub static BMP: Params = Params::new("kParams_BMP", &[req("filename", ty::STRING), req("R_Out", ty::STRING), req("G_Out", ty::STRING), req("B_Out", ty::STRING), req("INT", ty::INTEGER), req("INT", ty::INTEGER)]);
pub static ONE_FORM_ONE_WEATHER_ID_ONE_INT_ONE_OPTIONAL_GLOBAL: Params = Params::new("kParams_OneForm_OneWeatherID_OneInt_OneOptionalGlobal", &[req("Form", ty::ANY_FORM), req("Weather ID", ty::WEATHER_ID), req("Integer", ty::INTEGER), opt("Global", ty::GLOBAL)]);
pub static THREE_STRINGS_ONE_INT: Params = Params::new("kParams_ThreeStrings_OneInt", &[req("string", ty::STRING), req("string", ty::STRING), req("string", ty::STRING), req("Integer", ty::INTEGER)]);
pub static ONE_REF: Params = Params::new("kParams_OneRef", &[req("target", ty::OBJECT_REF)]);
pub static ONE_ACTOR_BASE_TWO_INTS_ONE_FLOAT: Params = Params::new("kParams_OneActorBase_TwoInts_OneFloat", &[req("target", ty::ACTOR_BASE), req("Property Type", ty::INTEGER), req("Property", ty::INTEGER), req("value", ty::FLOAT)]);
pub static ONE_ACTOR_BASE_TWO_INTS: Params = Params::new("kParams_OneActorBase_TwoInts", &[req("target", ty::ACTOR_BASE), req("Property Type", ty::INTEGER), req("Property", ty::INTEGER)]);
pub static SIX_SCRIPT_VARS: Params = Params::new("kParams_SixScriptVars", &[req("Script Var", ty::SCRIPT_VARIABLE), req("Script Var", ty::SCRIPT_VARIABLE), req("Script Var", ty::SCRIPT_VARIABLE), req("Script Var", ty::SCRIPT_VARIABLE), req("Script Var", ty::SCRIPT_VARIABLE), req("Script Var", ty::SCRIPT_VARIABLE)]);
pub static TWO_SCRIPT_VARS_ONE_OPTIONAL_SCRIPT_VAR: Params = Params::new("kParams_TwoScriptVars_OneOptionalScriptVar", &[req("Script Var", ty::SCRIPT_VARIABLE), req("Script Var", ty::SCRIPT_VARIABLE), opt("Script Var", ty::SCRIPT_VARIABLE)]);
pub static ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneForm_OneInt_OneOptionalForm", &[req("Form", ty::ANY_FORM), req("Integer", ty::INTEGER), opt("Form", ty::ANY_FORM)]);
pub static THREE_INTS: Params = Params::new("kParams_ThreeInts", &[req("Integer", ty::INTEGER), req("Integer", ty::INTEGER), req("Integer", ty::INTEGER)]);
pub static ONE_DIALOG_RES_ONE_TOPIC_ONE_INT_ONE_OPTIONAL_INT: Params = Params::new("kParams_OneDialogRes_OneTopic_OneInt_OneOptionalInt", &[req("Dialog Response", ty::ANY_FORM), req("Dialog Topic", ty::TOPIC), req("Integer", ty::INTEGER), opt("Integer", ty::INTEGER)]);
pub static ONE_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneInt_OneOptionalForm", &[req("index", ty::INTEGER), opt("form", ty::ANY_FORM)]);
pub static K_SPLATTER_PARAMS: Params = Params::new("kSplatterParams", &[req("Count", ty::INTEGER), req("Duration", ty::FLOAT), req("Size mult", ty::FLOAT), req("Opacity mult", ty::FLOAT), req("Alpha Texture", ty::STRING), req("Color Texture", ty::STRING), req("Flare Texture", ty::STRING), opt("No fade", ty::INTEGER)]);
pub static K_TRANSFORM_PARAMS: Params = Params::new("kTransformParams", &[req("Block name", ty::STRING), req("X", ty::FLOAT), req("Y", ty::FLOAT), req("Z", ty::FLOAT), req("W", ty::FLOAT), req("Is Rotation", ty::NVSE_BOOLEAN), req("World Data", ty::NVSE_BOOLEAN), req("Update", ty::NVSE_BOOLEAN)]);
pub static ONE_FORM_ONE_INT_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneForm_OneInt_OneOptionalInt_OneOptionalForm", &[req("form", ty::ANY_FORM), req("int", ty::INTEGER), opt("int", ty::INTEGER), opt("form", ty::ANY_FORM)]);
pub static ONE_FLOAT_ONE_OPTIONAL_FORM: Params = Params::new("kParams_OneFloat_OneOptionalForm", &[req("float", ty::FLOAT), opt("form", ty::ANY_FORM)]);
pub static EJECT_CASING: Params = Params::new("kParams_EjectCasing", &[opt("Target Node", ty::STRING), opt("Custom Casing Path", ty::STRING)]);
pub static ONE_CASINO: Params = Params::new("kParams_OneCasino", &[opt("Casino", ty::CASINO)]);
pub static ONE_CASINO_ONE_INT: Params = Params::new("kParams_OneCasinoOneInt", &[opt("Casino", ty::CASINO), opt("int", ty::INTEGER)]);
pub static ONE_CASINO_ONE_INT_ONE_STRING: Params = Params::new("kParams_OneCasinoOneIntOneString", &[opt("Casino", ty::CASINO), opt("int", ty::INTEGER), opt("string", ty::STRING)]);
pub static ONE_CASINO_ONE_FORM: Params = Params::new("kParams_OneCasinoOneForm", &[opt("Casino", ty::CASINO), opt("form", ty::ANY_FORM)]);
pub static ONE_BOOL_THREE_FLOATS: Params = Params::new("kParams_OneBoolThreeFloats", &[req("Enable", ty::INTEGER), req("X", ty::FLOAT), req("Y", ty::FLOAT), req("Z", ty::FLOAT)]);
pub static ONE_BOOL_ONE_INT_ONE_FLOAT: Params = Params::new("kParams_OneBoolOneIntOneFloat", &[req("Enable", ty::INTEGER), req("Axis/Mode", ty::INTEGER), req("Angle", ty::FLOAT), opt("Ref To Track", ty::OBJECT_REF)]);
pub static PATH_TO_REF: Params = Params::new("kParams_PathToRef", &[req("Target Reference", ty::ANY_FORM), opt("Target Radius", ty::FLOAT), opt("Face Target", ty::INTEGER)]);
pub static IS_NI_SEQUENCE_ACTIVE: Params = Params::new("kParams_IsNiSequenceActive", &[req("Sequence Name", ty::STRING), opt("Block Name", ty::STRING), opt("First Person", ty::INTEGER)]);
pub static STOP_NI_SEQUENCE: Params = Params::new("kParams_StopNiSequence", &[req("Sequence Name", ty::STRING), opt("Ease Out Time", ty::FLOAT), opt("Block Name", ty::STRING), opt("First Person", ty::INTEGER)]);
pub static GET_TOPIC_INFO: Params = Params::new("kParams_GetTopicInfo", &[req("form", ty::ANY_FORM), opt("int", ty::QUEST)]);
pub static THREE_FLOATS_TWO_OPTIONAL_FLOATS: Params = Params::new("kParams_ThreeFloatsTwoOptionalFloats", &[req("X", ty::FLOAT), req("Y", ty::FLOAT), req("Z", ty::FLOAT), opt("Target Radius", ty::FLOAT), opt("Target Angle", ty::FLOAT)]);
pub static SET_PROPERTY_VALUE: Params = Params::new("kParams_SetPropertyValue", &[req("Block Name", ty::STRING), req("Item", ty::INTEGER), req("Value", ty::INTEGER), opt("First Person", ty::INTEGER)]);
pub static GET_PROPERTY_VALUE: Params = Params::new("kParams_GetPropertyValue", &[req("Block Name", ty::STRING), req("Item", ty::INTEGER), opt("First Person", ty::INTEGER)]);
pub static SET_NI_LIGHT_VALUE: Params = Params::new("kParams_SetNiLightValue", &[req("Block Name", ty::STRING), req("Item", ty::INTEGER), req("Value", ty::FLOAT), opt("First Person", ty::INTEGER)]);
pub static SET_NI_LIGHT_COLOR: Params = Params::new("kParams_SetNiLightColor", &[req("Block Name", ty::STRING), req("Item", ty::INTEGER), req("Red", ty::FLOAT), req("Green", ty::FLOAT), req("Blue", ty::FLOAT), opt("First Person", ty::INTEGER)]);
pub static GET_NI_LIGHT_COLOR: Params = Params::new("kParams_GetNiLightColor", &[req("Block Name", ty::STRING), req("Item", ty::INTEGER), req("Red", ty::SCRIPT_VARIABLE), req("Green", ty::SCRIPT_VARIABLE), req("Blue", ty::SCRIPT_VARIABLE), opt("First Person", ty::INTEGER)]);
pub static SET_SWITCH_NODE_INDEX: Params = Params::new("kParams_SetSwitchNodeIndex", &[req("Block Name", ty::STRING), req("Index", ty::INTEGER), opt("First Person", ty::INTEGER)]);
pub static GET_SWITCH_NODE_INDEX: Params = Params::new("kParams_GetSwitchNodeIndex", &[req("Block Name", ty::STRING), opt("First Person", ty::INTEGER)]);
pub static SCENEGRAPH_UPDATE: Params = Params::new("kParams_ScenegraphUpdate", &[req("Type", ty::INTEGER), opt("Time", ty::FLOAT), opt("Update Controllers", ty::INTEGER), opt("Block Name", ty::STRING), opt("First Person", ty::INTEGER)]);
pub static GET_NI_BOUND: Params = Params::new("kParams_GetNiBound", &[opt("Block Name", ty::STRING), opt("First Person", ty::INTEGER)]);
pub static PROJECTION_ARGS_LEGACY: Params = Params::new("kParamsProjectionArgsLegacy", &[req("X_Out", ty::STRING), req("Y_Out", ty::STRING), req("Z_Out", ty::STRING), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("HandleMode", ty::INTEGER), opt("Object Ref", ty::OBJECT_REF)]);
pub static PROJECTION_ARGS: Params = Params::new("kParamsProjectionArgs", &[req("X_Out", ty::SCRIPT_VARIABLE), req("Y_Out", ty::SCRIPT_VARIABLE), req("Z_Out", ty::SCRIPT_VARIABLE), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("Float", ty::FLOAT), req("HandleMode", ty::INTEGER), opt("Object Ref", ty::OBJECT_REF)]);
pub static CALL_PER_REF: Params = Params::new("kParamsCallPerRef", &[req("Script", ty::ANY_FORM), opt("Type Filter", ty::INTEGER), opt("Distance Filter", ty::FLOAT), opt("Angle Filter", ty::FLOAT), opt("Cell", ty::ANY_FORM)]);
pub static CALL_PER_REF_EX: Params = Params::new("kParamsCallPerRefEx", &[req("Script", ty::NVSE_FORM), req("Type Filter Array", ty::NVSE_ARRAY), opt("Distance Filter", ty::NVSE_NUMBER), opt("Angle Filter", ty::NVSE_NUMBER), opt("Cell", ty::NVSE_FORM)]);
pub static CALL_PER_MOBILE_OBJECT: Params = Params::new("kParamsCallPerMobileObject", &[req("Script", ty::ANY_FORM), req("Process Level", ty::INTEGER), opt("Type Filter", ty::INTEGER), opt("Distance Filter", ty::FLOAT), opt("Angle Filter", ty::FLOAT), opt("Space", ty::ANY_FORM)]);
pub static CALL_PER_MOBILE_OBJECT_EX: Params = Params::new("kParamsCallPerMobileObjectEx", &[req("Script", ty::NVSE_FORM), req("Process Level", ty::NVSE_NUMBER), req("Type Filter Array", ty::NVSE_ARRAY), opt("Distance Filter", ty::NVSE_NUMBER), opt("Angle Filter", ty::NVSE_NUMBER), opt("Space", ty::NVSE_FORM)]);
pub static SET_NI_P_SYS_MODIFIER_VALUE: Params = Params::new("kParams_SetNiPSysModifierValue", &[req("Block Name", ty::STRING), req("Item", ty::INTEGER), req("Value", ty::FLOAT), opt("First Person", ty::INTEGER)]);
pub static GET_NI_P_SYS_MODIFIER_VALUE: Params = Params::new("kParams_GetNiPSysModifierValue", &[req("Block Name", ty::STRING), req("Item", ty::INTEGER), opt("First Person", ty::INTEGER)]);
pub static SET_PARTICLE_EMITTER_SPAWN_RATE: Params = Params::new("kParams_SetParticleEmitterSpawnRate", &[req("Block Name", ty::STRING), req("Value", ty::FLOAT), opt("First Person", ty::INTEGER)]);
pub static REMAP_LAND: Params = Params::new("kParamsRemapLand", &[req("Target Land FormID", ty::INTEGER), opt("Source WorldSpace", ty::WORLD_SPACE), opt("Source Cell X", ty::INTEGER), opt("Source Cell Y", ty::INTEGER)]);
pub static APPLY_MODEL_TEXTURE_SWAP: Params = Params::new("kParamsApplyModelTextureSwap", &[req("Base Form", ty::TES_OBJECT), opt("Block Name", ty::STRING), opt("Reference", ty::OBJECT_REF), opt("First Person", ty::INTEGER)]);

/// The slots in opcode order from `0x3100`. Each: opcode, name, alias (empty for none), needs a
/// reference, parameters, return type, kind, and the source file (without `.h`) of its `DEFINE_*` line.
pub static SLOTS: [Slot; 408] = [
    cmd(0x3100, "JGLegacyWorldToScreen", "", false, &PROJECTION_ARGS_LEGACY, NUM, Command, "fn_math"),
    cmd(0x3101, "ToggleLevelUpMenu", "", false, &ONE_INT, NUM, Command, "fn_gameplay"),
    cmd(0x3102, "IsLevelUpMenuEnabled", "", false, &NO_PARAMS, NUM, Condition, "fn_gameplay"),
    cmd(0x3103, "GetBaseEffectAV", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3104, "GetBaseEffectArchetype", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3105, "IsCellVisited", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3106, "IsCellExpired", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3107, "MD5File", "", false, &ONE_STRING, STR, Command, "fn_file"),
    cmd(0x3108, "SHA1File", "", false, &ONE_STRING, STR, Command, "fn_file"),
    cmd(0x3109, "TogglePipBoy", "", false, &ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x310a, "GetCalculatedWeaponDPS", "", false, &ONE_OPTIONAL_OBJECT_ID, NUM, Command, "fn_form"),
    cmd(0x310b, "GetInteriorLightingTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x310c, "SetInteriorLightingTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "fn_form"),
    cmd(0x310d, "GetPixelFromBMP", "", false, &BMP, NUM, Command, "fn_file"),
    cmd(0x310e, "GetWorldSpaceMapTexture", "", false, &ONE_FORM, STR, Command, "fn_ui"),
    cmd(0x310f, "Jump", "", true, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3110, "SetCameraShake", "", false, &TWO_FLOATS, NUM, Command, "fn_gameplay"),
    cmd(0x3111, "StopVATSCam", "", false, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3112, "GetIMODAnimatable", "", false, &ONE_IMOD, NUM, Command, "fn_form"),
    cmd(0x3113, "SetIMODAnimatable", "", false, &ONE_IMOD_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x3114, "GetEditorID", "GetEdID", false, &ONE_OPTIONAL_FORM, STR, Command, "fn_utility"),
    cmd(0x3115, "GetJohnnyPatch", "", false, &ONE_INT, NUM, Condition, "fn_utility"),
    cmd(0x3116, "SetVelEx", "", true, &THREE_FLOATS, NUM, Command, "fn_gameplay"),
    cmd(0x3117, "NullArgs", "", false, &ONE_OPTIONAL_INT, NUM, Command, "fn_utility"),
    cmd(0x3118, "GetMediaSetTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_mediaset"),
    cmd(0x3119, "SetMediaSetTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "fn_mediaset"),
    cmd(0x311a, "GetMediaSetTraitString", "", false, &ONE_FORM_ONE_INT, STR, Command, "fn_mediaset"),
    cmd(0x311b, "SetMediaSetTraitString", "", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "fn_mediaset"),
    cmd(0x311c, "GetMediaSetTraitSound", "", false, &ONE_FORM_ONE_INT, FORM, Command, "fn_mediaset"),
    cmd(0x311d, "SetMediaSetTraitSound", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "fn_mediaset"),
    cmd(0x311e, "GetWeapon1stPersonModel", "", false, &ONE_FORM_ONE_INT, FORM, Command, "fn_form"),
    cmd(0x311f, "SetWeapon1stPersonModel", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x3120, "GetBufferedCellsAlt", "", false, &ONE_INT, ARR, Command, "fn_form"),
    cmd(0x3121, "GetTimePlayed", "", false, &ONE_OPTIONAL_INT, NUM, Condition, "fn_utility"),
    cmd(0x3122, "GetActorValueModifierAlt", "GetAVModAlt", true, &ONE_ACTOR_VALUE_ONE_OPTIONAL_INT, NUM, Command, "fn_form"),
    cmd(0x3123, "AsmBreak", "", false, &NO_PARAMS, NUM, Command, "fn_utility"),
    cmd(0x3124, "RefAddr", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "fn_utility"),
    cmd(0x3125, "GetMusicTypePath", "", false, &ONE_FORM, STR, Command, "fn_form"),
    cmd(0x3126, "GetMusicTypeDB", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3127, "SetMusicTypeDB", "", false, &ONE_FORM_ONE_FLOAT, NUM, Command, "fn_form"),
    cmd(0x3128, "EditorIDToFormID", "", false, &ONE_STRING, FORM, Command, "fn_utility"),
    cmd(0x3129, "GetRegionWeatherOverride", "", false, &ONE_FORM, NUM, Command, "fn_region"),
    cmd(0x312a, "SetRegionWeatherOverride", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_region"),
    cmd(0x312b, "GetRegionWeatherPriority", "", false, &ONE_FORM, NUM, Command, "fn_region"),
    cmd(0x312c, "SetRegionWeatherPriority", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_region"),
    cmd(0x312d, "IsWeatherInRegion", "", false, &TWO_FORMS, NUM, Command, "fn_region"),
    cmd(0x312e, "RemoveRegionWeather", "", false, &TWO_FORMS, NUM, Command, "fn_region"),
    cmd(0x312f, "AddRegionWeather", "", false, &ONE_FORM_ONE_WEATHER_ID_ONE_INT_ONE_OPTIONAL_GLOBAL, NUM, Command, "fn_region"),
    cmd(0x3130, "GetRegionWeathers", "", false, &ONE_FORM, ARR, Command, "fn_region"),
    cmd(0x3131, "ClearRegionWeathers", "", false, &ONE_FORM, NUM, Command, "fn_region"),
    cmd(0x3132, "StopSoundAlt", "", false, &TWO_FORMS_ONE_OPTIONAL_FLOAT, NUM, Command, "fn_sound"),
    cmd(0x3133, "RemovePrimitive", "", true, &NO_PARAMS, NUM, Command, "fn_form"),
    cmd(0x3134, "GetPrimitiveType", "", true, &NO_PARAMS, NUM, Command, "fn_form"),
    cmd(0x3135, "GetBaseScale", "GetBasedScale", false, &ONE_OPTIONAL_FORM, NUM, Condition, "fn_form"),
    cmd(0x3136, "GetCustomMapMarker", "", false, &NO_PARAMS, FORM, Command, "fn_ui"),
    cmd(0x3137, "UnsetAV", "", true, &ONE_ACTOR_VALUE, NUM, Command, "fn_gameplay"),
    cmd(0x3138, "UnforceAV", "", true, &ONE_ACTOR_VALUE, NUM, Command, "fn_gameplay"),
    cmd(0x3139, "ToggleNthPipboyLight", "", false, &TWO_INTS, NUM, Command, "fn_gameplay"),
    cmd(0x313a, "SetBipedIconPathAlt", "", false, &ONE_STRING_ONE_INT_ONE_FORM, NUM, Command, "fn_ui"),
    cmd(0x313b, "GetFacegenModelFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "fn_form"),
    cmd(0x313c, "SetFacegenModelFlag", "", false, &ONE_FORM_THREE_INTS, NUM, Command, "fn_form"),
    cmd(0x313d, "GetRaceBodyModelPath", "", false, &ONE_FORM_TWO_INTS, STR, Command, "fn_form"),
    cmd(0x313e, "SetEquipType", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x313f, "GetFactionMembers", "", false, &ONE_FORM_ONE_OPTIONAL_INT, ARR, Command, "fn_form"),
    cmd(0x3140, "GetRaceHeadModelPath", "", false, &ONE_FORM_TWO_INTS, STR, Command, "fn_form"),
    cmd(0x3141, "GetDefaultHeapSize", "", false, &NO_PARAMS, NUM, Command, "fn_utility"),
    cmd(0x3142, "Get3DDistanceBetweenNiNodes", "", false, &TWO_REFS_TWO_STRINGS, NUM, Command, "fn_math"),
    cmd(0x3143, "Get3DDistanceToNiNode", "", true, &ONE_STRING_THREE_FLOATS, NUM, Command, "fn_math"),
    cmd(0x3144, "Get3DDistanceFromHitToNiNode", "", true, &ONE_STRING, NUM, Command, "fn_math"),
    cmd(0x3145, "GetVector3DDistance", "", false, &SIX_FLOATS, NUM, Command, "fn_math"),
    cmd(0x3146, "GetLinearVelocity", "", true, &FOUR_STRINGS, NUM, Command, "fn_utility"),
    cmd(0x3147, "GetLifeState", "", true, &NO_PARAMS, NUM, Condition, "fn_form"),
    cmd(0x3148, "GetRaceFlag", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x3149, "SetRaceFlag", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "fn_form"),
    cmd(0x314a, "GetContainerSound", "", false, &ONE_FORM_ONE_INT, FORM, Command, "fn_form"),
    cmd(0x314b, "SetContainerSound", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x314c, "SetJohnnyOnDyingEventHandler", "SetOnDyingEventHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x314d, "SetJohnnyOnStartQuestEventHandler", "SetOnStartQuestEventHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x314e, "SetJohnnyOnStopQuestEventHandler", "SetOnStopQuestEventHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x314f, "DisableMuzzleFlashLights", "", false, &ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x3150, "SetCustomMapMarkerIcon", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "fn_ui"),
    cmd(0x3151, "GetCreatureCombatSkill", "", false, &ONE_OPTIONAL_ACTOR_BASE, NUM, Command, "fn_form"),
    cmd(0x3152, "SetExplosionSound", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3153, "SetProjectileSound", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3154, "SetWeaponWorldModelPath", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "fn_form"),
    cmd(0x3155, "Clamp", "", false, &THREE_FLOATS, NUM, Command, "fn_math"),
    cmd(0x3156, "Remap", "", false, &FIVE_FLOATS, NUM, Command, "fn_math"),
    cmd(0x3157, "Lerp", "", false, &THREE_FLOATS, NUM, Command, "fn_math"),
    cmd(0x3158, "SetJohnnySeenDataEventHandler", "SetSeenDataEventHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x3159, "SetJohnnyOnLimbGoneEventHandler", "SetOnLimbGoneEventHandler", false, &EVENT_ONE_FORM_ONE_INT, NUM, Command, "fn_event"),
    cmd(0x315a, "Sign", "", false, &ONE_FLOAT, NUM, Command, "fn_math"),
    cmd(0x315b, "AddTerminalMenuItem", "", false, &ONE_FORM, NUM, Command, "fn_terminal"),
    cmd(0x315c, "GetTerminalMenuItemText", "", false, &ONE_FORM_TWO_INTS, STR, Command, "fn_terminal"),
    cmd(0x315d, "SetTerminalMenuItemText", "", false, &ONE_FORM_TWO_INTS_ONE_STRING, NUM, Command, "fn_terminal"),
    cmd(0x315e, "GetTerminalMenuItemNote", "", false, &ONE_FORM_ONE_INT, FORM, Command, "fn_terminal"),
    cmd(0x315f, "SetTerminalMenuItemNote", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "fn_terminal"),
    cmd(0x3160, "GetTerminalMenuItemSubmenu", "", false, &ONE_FORM_ONE_INT, FORM, Command, "fn_terminal"),
    cmd(0x3161, "SetTerminalMenuItemSubmenu", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "fn_terminal"),
    cmd(0x3162, "GetRunSpeed", "", true, &NO_PARAMS, NUM, Condition, "fn_gameplay"),
    cmd(0x3163, "DisableMenuArrowKeys", "", false, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3164, "EnableMenuArrowKeys", "", false, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3165, "GetQuestFailed", "GetQF", false, &ONE_QUEST, NUM, Condition, "fn_form"),
    cmd(0x3166, "SetJohnnyOnChallengeCompleteEventHandler", "SetOnChallengeCompleteEventHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x3167, "GetTerminalMenuItemCount", "", false, &ONE_FORM, NUM, Command, "fn_terminal"),
    cmd(0x3168, "GetPipBoyMode", "", false, &NO_PARAMS, NUM, Condition, "fn_utility"),
    cmd(0x3169, "GetWeaponVATSTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x316a, "SetWeaponVATSTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "fn_form"),
    cmd(0x316b, "RemoveTerminalMenuItem", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_terminal"),
    cmd(0x316c, "SetWorldSpaceMapTexture", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "fn_ui"),
    cmd(0x316d, "GetFormOverrideIndex", "", false, &ONE_FORM, NUM, Command, "fn_utility"),
    cmd(0x316e, "SetJohnnyOnCrosshairEventHandler", "SetOnCrosshairEventHandler", false, &EVENT_ONE_FORM_ONE_INT, NUM, Command, "fn_event"),
    cmd(0x316f, "GetSequenceAnimGroup", "", true, &ONE_INT, NUM, Condition, "fn_utility"),
    cmd(0x3170, "QueueObjectiveText", "", false, &ONE_STRING_TWO_OPTIONAL_INTS, NUM, Command, "fn_ui"),
    cmd(0x3171, "QueueCinematicText", "", false, &TWO_STRINGS_ONE_OPTIONAL_STRING_FOUR_OPTIONAL_INTS, NUM, Command, "fn_ui"),
    cmd(0x3172, "ar_SortEditor", "", false, &ONE_INT_ONE_OPTIONAL_INT, ARR, Command, "fn_utility"),
    cmd(0x3173, "SetUIUpdateSound", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_utility"),
    cmd(0x3174, "GetActorValueAlt", "GetAVAlt", true, &ONE_INT, NUM, Command, "fn_av"),
    cmd(0x3175, "ModActorValueAlt", "ModAVAlt", true, &TWO_INTS, NUM, Command, "fn_av"),
    cmd(0x3176, "SetActorValueAlt", "SetAVAlt", true, &TWO_INTS, NUM, Command, "fn_av"),
    cmd(0x3177, "ForceActorValueAlt", "ForceAVAlt", true, &TWO_INTS, NUM, Command, "fn_av"),
    cmd(0x3178, "DamageActorValueAlt", "DamageAVAlt", true, &ONE_INT_ONE_FLOAT, NUM, Command, "fn_av"),
    cmd(0x3179, "RestoreActorValueAlt", "RestoreAVAlt", true, &ONE_INT_ONE_FLOAT, NUM, Command, "fn_av"),
    cmd(0x317a, "HighlightBodyPartAlt", "", true, &ONE_OPTIONAL_FLOAT, NUM, Command, "fn_gameplay"),
    cmd(0x317b, "DeactivateAllHighlightsAlt", "", true, &ONE_OPTIONAL_FLOAT, NUM, Command, "fn_gameplay"),
    cmd(0x317c, "SetJohnnyOnCompleteQuestEventHandler", "SetOnCompleteQuestEventHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x317d, "SetJohnnyOnFailQuestEventHandler", "SetOnFailQuestEventHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x317e, "IsDLLLoaded", "", false, &ONE_STRING_ONE_OPTIONAL_INT, NUM, Command, "fn_utility"),
    cmd(0x317f, "SetJohnnyOnSettingsUpdateEventHandler", "SetOnSettingsUpdateEventHandler", false, &EVENT, NUM, Command, "fn_event"),
    cmd(0x3180, "GetQuestDelay", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3181, "GetNearestCompassHostileDirection", "", false, &ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x3182, "GetNearestCompassHostile", "", false, &ONE_OPTIONAL_INT, FORM, Command, "fn_gameplay"),
    cmd(0x3183, "RefreshIdle", "", true, &ONE_OPTIONAL_INT, NUM, Command, "fn_utility"),
    cmd(0x3184, "SetNoteRead", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "fn_form"),
    cmd(0x3185, "SetDisablePlayerControlsHUDVisibilityFlags", "SetDPCHUDFlags", false, &ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x3186, "GetCameraTranslation", "", false, &THREE_STRINGS_ONE_INT, NUM, Command, "fn_math"),
    cmd(0x3187, "IsCompassHostile", "", true, &NO_PARAMS, NUM, Condition, "fn_gameplay"),
    cmd(0x3188, "SetMessageIconPath", "", false, &ONE_STRING_ONE_FORM_ONE_OPTIONAL_INT, NUM, Command, "fn_form"),
    cmd(0x3189, "GetMessageIconPath", "", false, &ONE_FORM_ONE_OPTIONAL_INT, STR, Command, "fn_form"),
    cmd(0x318a, "ExitGameAlt", "", false, &NO_PARAMS, NUM, Command, "fn_utility"),
    cmd(0x318b, "ToggleCombatMusic", "", false, &ONE_INT, NUM, Command, "fn_gameplay"),
    cmd(0x318c, "IsCombatMusicEnabled", "", false, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x318d, "SetJohnnyOnAddPerkEventHandler", "SetOnAddPerkEventHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x318e, "SetJohnnyOnRemovePerkEventHandler", "SetOnRemovePerkEventHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x318f, "IsHostilesNearby", "", false, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3190, "ModNthTempEffectTimeLeft", "", true, &ONE_INT_ONE_FLOAT, NUM, Command, "fn_gameplay"),
    cmd(0x3191, "GetWeaponWorldModelPath", "", false, &ONE_FORM, STR, Command, "fn_form"),
    cmd(0x3192, "GetBodyPartTraitString", "", false, &ONE_FORM_TWO_INTS, STR, Command, "fn_form"),
    cmd(0x3193, "GetActorEffectType", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3194, "GetTextureWidth", "", false, &ONE_STRING_ONE_OPTIONAL_INT, NUM, Command, "fn_file"),
    cmd(0x3195, "GetTextureHeight", "", false, &ONE_STRING, NUM, Command, "fn_file"),
    cmd(0x3196, "GetTextureFormat", "", false, &ONE_STRING, NUM, Command, "fn_file"),
    cmd(0x3197, "GetTextureMipMapCount", "", false, &ONE_STRING, NUM, Command, "fn_file"),
    cmd(0x3198, "GetCalculatedSpread", "", true, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3199, "SendStealingAlarm", "", true, &ONE_REF_ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x319a, "ApplyWeaponPoison", "", true, &ONE_OPTIONAL_FORM, NUM, Command, "fn_gameplay"),
    cmd(0x319b, "GetTalkingActivatorActor", "", false, &ONE_FORM, FORM, Command, "fn_form"),
    cmd(0x319c, "GetPlayerKarmaTitle", "", false, &ONE_OPTIONAL_INT, STR, Command, "fn_form"),
    cmd(0x319d, "GetCompassHostiles", "", false, &TWO_OPTIONAL_INTS, ARR, Command, "fn_gameplay"),
    cmd(0x319e, "ToggleDisableSaves", "", false, &ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x319f, "SetJohnnyOnRenderUpdateEventHandler", "SetOnRenderUpdateEventHandler", false, &EVENT_OPTIONAL_FLAG, NUM, Command, "fn_event"),
    cmd(0x31a0, "WorldToScreen", "", false, &PROJECTION_ARGS, NUM, Command, "fn_math"),
    cmd(0x31a1, "FaceGenGetNthProperty", "FaceGGetNth", false, &ONE_ACTOR_BASE_TWO_INTS, NUM, Command, "fn_form"),
    cmd(0x31a2, "FaceGenSetNthProperty", "FaceGSetNth", false, &ONE_ACTOR_BASE_TWO_INTS_ONE_FLOAT, NUM, Command, "fn_form"),
    cmd(0x31a3, "FaceGenRefreshAppearance", "FaceGRefresh", true, &NO_PARAMS, NUM, Command, "fn_form"),
    cmd(0x31a4, "SendTrespassAlarmAlt", "", true, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x31a5, "IsCrimeOrEnemy", "", true, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x31a6, "GetAvailablePerks", "", false, &NO_PARAMS, ARR, Command, "fn_form"),
    cmd(0x31a7, "GetThresholdedActorValue", "GetThresholdAV", true, &ONE_ACTOR_VALUE, NUM, Condition, "fn_av"),
    cmd(0x31a8, "GetEffectShaderTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x31a9, "SetEffectShaderTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "fn_form"),
    cmd(0x31aa, "GetEffectShaderTexturePath", "", false, &ONE_FORM_ONE_INT, STR, Command, "fn_form"),
    cmd(0x31ab, "SetEffectShaderTexturePath", "", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "fn_form"),
    cmd(0x31ac, "GetSystemColor", "", false, &ONE_INT, NUM, Command, "fn_ui"),
    cmd(0x31ad, "RGBtoHSV", "", false, &SIX_SCRIPT_VARS, NUM, Command, "fn_math"),
    cmd(0x31ae, "HSVtoRGB", "", false, &SIX_SCRIPT_VARS, NUM, Command, "fn_math"),
    cmd(0x31af, "GetLocationSpecificLoadScreensOnly", "", false, &NO_PARAMS, NUM, Condition, "fn_gameplay"),
    cmd(0x31b0, "GetArmorAltTextures", "", false, &ONE_FORM_ONE_INT, ARR, Command, "fn_form"),
    cmd(0x31b1, "GetIdleMarkerTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x31b2, "GetIdleMarkerAnimations", "", false, &ONE_FORM, ARR, Command, "fn_form"),
    cmd(0x31b3, "SetIdleMarkerTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "fn_form"),
    cmd(0x31b4, "SetIdleMarkerAnimation", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x31b5, "SetIdleMarkerAnimations", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x31b6, "GetWeaponAltTextures", "", false, &ONE_FORM, ARR, Command, "fn_form"),
    cmd(0x31b7, "GetRefActivationPromptOverride", "GetXATO", true, &NO_PARAMS, STR, Command, "fn_form"),
    cmd(0x31b8, "SetRefActivationPromptOverride", "SetXATO", true, &ONE_STRING, NUM, Command, "fn_form"),
    cmd(0x31b9, "GetTerminalMenuItemFlags", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_terminal"),
    cmd(0x31ba, "SetTerminalMenuItemFlags", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "fn_terminal"),
    cmd(0x31bb, "GetLocationName", "", true, &NO_PARAMS, STR, Command, "fn_gameplay"),
    cmd(0x31bc, "GetRegionMapName", "", false, &ONE_FORM, STR, Command, "fn_region"),
    cmd(0x31bd, "SetRegionMapName", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "fn_region"),
    cmd(0x31be, "GetRGBColor", "", false, &THREE_INTS, NUM, Command, "fn_math"),
    cmd(0x31bf, "GetPlayingEffectShaders", "", true, &NO_PARAMS, ARR, Command, "fn_gameplay"),
    cmd(0x31c0, "GetBookFlags", "", false, &ONE_FORM, NUM, Command, "fn_book"),
    cmd(0x31c1, "SetBookFlags", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_book"),
    cmd(0x31c2, "GetBookSkill", "", false, &ONE_FORM, NUM, Command, "fn_book"),
    cmd(0x31c3, "SetBookSkill", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_book"),
    cmd(0x31c4, "SetOnActorValueChangeEventHandler", "SetJohnnyOnActorValueChangeEventHandler", false, &EVENT_ONE_INT, NUM, Command, "fn_event"),
    cmd(0x31c5, "RefreshTerminalMenu", "rtm", false, &NO_PARAMS, NUM, Command, "fn_terminal"),
    cmd(0x31c6, "SetRefEncounterZone", "SetRefZone", true, &ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x31c7, "GetRefEncounterZone", "GetRefZone", true, &NO_PARAMS, FORM, Command, "fn_form"),
    cmd(0x31c8, "SetCellEncounterZone", "SetCellZone", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x31c9, "SetWorldspaceEncounterZone", "SetWorldZone", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x31ca, "GetWorldspaceEncounterZone", "GetWorldZone", false, &ONE_FORM, FORM, Command, "fn_form"),
    cmd(0x31cb, "UpdateCrosshairPrompt", "", false, &NO_PARAMS, NUM, Command, "fn_utility"),
    cmd(0x31cc, "GetLightingTemplateTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x31cd, "SetLightingTemplateTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "fn_form"),
    cmd(0x31ce, "GetLightingTemplateCell", "", false, &ONE_FORM, FORM, Command, "fn_form"),
    cmd(0x31cf, "SetLightingTemplateCell", "", false, &TWO_FORMS, NUM, Command, "fn_form"),
    cmd(0x31d0, "RemoveScopeModelPath", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x31d1, "GetNthRegionWeatherType", "", false, &ONE_FORM_ONE_INT, FORM, Command, "fn_region"),
    cmd(0x31d2, "GetNthRegionWeatherChance", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_region"),
    cmd(0x31d3, "GetNthRegionWeatherGlobal", "", false, &ONE_FORM_ONE_INT, FORM, Command, "fn_region"),
    cmd(0x31d4, "PlaySoundFile", "", false, &ONE_STRING_THREE_OPTIONAL_INTS, NUM, Command, "fn_sound"),
    cmd(0x31d5, "StopSoundFile", "", false, &NO_PARAMS, NUM, Command, "fn_sound"),
    cmd(0x31d6, "StopSoundLooping", "", false, &ONE_FORM, NUM, Command, "fn_sound"),
    cmd(0x31d7, "GetSystemColorAlt", "", false, &ONE_INT_THREE_SCRIPT_VARS, NUM, Command, "fn_ui"),
    cmd(0x31d8, "SetCustomReputationChangeIcon", "scrci", false, &ONE_FORM_ONE_INT_ONE_STRING, NUM, Command, "fn_ui"),
    cmd(0x31d9, "SetArmorAltTexture", "", false, &ONE_FORM_TWO_INTS_ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x31da, "SetWeaponAltTexture", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x31db, "ClearWeaponAltTexture", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x31dc, "ClearArmorAltTexture", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "fn_form"),
    cmd(0x31dd, "AddNavmeshObstacle", "", true, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x31de, "RemoveNavmeshObstacle", "", true, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x31df, "RollCredits", "", false, &NO_PARAMS, NUM, Command, "fn_utility"),
    cmd(0x31e0, "GetFactionFlags", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x31e1, "SetFactionFlags", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x31e2, "GetLandTextureUnderFeet", "", true, &NO_PARAMS, FORM, Command, "fn_gameplay"),
    cmd(0x31e3, "SetOnProcessLevelChangeEventHandler", "", false, &EVENT_ONE_FORM_ONE_INT, NUM, Command, "fn_event"),
    cmd(0x31e4, "GetExtraMiscStat", "", false, &ONE_STRING, NUM, Command, "fn_ui"),
    cmd(0x31e5, "ModExtraMiscStat", "", false, &ONE_STRING_ONE_INT, NUM, Command, "fn_ui"),
    cmd(0x31e6, "GetMoonPhase", "", false, &NO_PARAMS, NUM, Condition, "fn_gameplay"),
    cmd(0x31e7, "GetFormRecipesAlt", "", false, &ONE_FORM, ARR, Command, "fn_form"),
    cmd(0x31e8, "RewardKarmaAlt", "", false, &ONE_INT, NUM, Command, "fn_gameplay"),
    cmd(0x31e9, "GetPackedPlayerFOV", "", false, &TWO_SCRIPT_VARS_ONE_OPTIONAL_SCRIPT_VAR, NUM, Command, "fn_math"),
    cmd(0x31ea, "DialogResponseAddRelatedTopic", "", false, &ONE_DIALOG_RES_ONE_TOPIC_ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "fn_dial"),
    cmd(0x31eb, "DialogResponseRelatedGetAll", "", false, &ONE_FORM_ONE_INT, ARR, Command, "fn_dial"),
    cmd(0x31ec, "GetPlayerCamFOV", "", false, &ONE_INT, NUM, Command, "fn_math"),
    cmd(0x31ed, "ShowBarberMenuEx", "", false, &ONE_INT_ONE_OPTIONAL_FORM, NUM, Command, "fn_ui"),
    cmd(0x31ee, "InitExtraMiscStat", "", false, &ONE_STRING, NUM, Command, "fn_ui"),
    cmd(0x31ef, "TriggerScreenSplatterEx", "", false, &K_SPLATTER_PARAMS, NUM, Command, "fn_utility"),
    cmd(0x31f0, "SetViewmodelClipDistance", "", false, &ONE_FLOAT, NUM, Command, "fn_utility"),
    cmd(0x31f1, "GetViewmodelClipDistance", "", false, &NO_PARAMS, NUM, Command, "fn_utility"),
    cmd(0x31f2, "SetBlockTransform", "", true, &K_TRANSFORM_PARAMS, NUM, Command, "fn_utility"),
    cmd(0x31f3, "RefAddrxData", "", true, &ONE_INT, NUM, Command, "fn_utility"),
    cmd(0x31f4, "AudioMarkerGetController", "AMKGetCtrl", true, &NO_PARAMS, NUM, Condition, "fn_mediaset"),
    cmd(0x31f5, "AudioMarkerSetController", "AMKSetCtrl", true, &ONE_FORM, NUM, Command, "fn_mediaset"),
    cmd(0x31f6, "AudioMarkerGetProperty", "AMKGetProp", true, &ONE_INT, NUM, Command, "fn_mediaset"),
    cmd(0x31f7, "AudioMarkerSetProperty", "AMKSetProp", true, &ONE_INT_ONE_FLOAT, NUM, Command, "fn_mediaset"),
    cmd(0x31f8, "IsRadioRefPlaying", "", true, &NO_PARAMS, NUM, Condition, "fn_form"),
    cmd(0x31f9, "TuneRadioRef", "", true, &ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x31fa, "GetAllGameRadios", "", false, &NO_PARAMS, ARR, Command, "fn_utility"),
    cmd(0x31fb, "GetAvailableRadios", "", true, &NO_PARAMS, ARR, Command, "fn_utility"),
    cmd(0x31fc, "SetJohnnyOnRadioPostSoundAttachEventHandler", "SetOnRadioPostSoundHandler", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x31fd, "AudioMarkerGetCurrent", "AMKGetCur", false, &NO_PARAMS, NUM, Condition, "fn_mediaset"),
    cmd(0x31fe, "HideItemBarterEx", "", false, &ONE_FORM_ONE_INT_ONE_OPTIONAL_INT_ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x31ff, "IsItemBarterHiddenEx", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x3200, "GetCurrentFurnitureRef", "", true, &NO_PARAMS, NUM, Command, "fn_form"),
    cmd(0x3201, "SetCameraShakeNoHUDShudder", "CamShakeNHUD", false, &TWO_FLOATS, NUM, Command, "fn_gameplay"),
    cmd(0x3202, "GameGetSecondsPassed", "GGetSecPass", false, &NO_PARAMS, NUM, Condition, "fn_utility"),
    cmd(0x3203, "SetJohnnyOnKeyboardControllerSelectionChangeEventHandler", "SetOnKBCTrlUIDeltaHandler", false, &EVENT_ONE_INT, NUM, Command, "fn_event"),
    cmd(0x3204, "IsBSALoaded", "", false, &ONE_STRING, NUM, Command, "fn_file"),
    cmd(0x3205, "GetTempIngestibleEffects", "", false, &NO_PARAMS, ARR, Command, "fn_gameplay"),
    cmd(0x3206, "PlaySoundFade", "", false, &ONE_FORM_ONE_FLOAT, NUM, Command, "fn_sound"),
    cmd(0x3207, "GetPointInNavMesh", "", false, &THREE_FLOATS_ONE_INT_ONE_OPTIONAL_FLOAT, ARR, Command, "fn_gameplay"),
    cmd(0x3208, "GetNearestNavMeshTriangle", "", false, &THREE_FLOATS_ONE_INT_ONE_OPTIONAL_FLOAT, ARR, Command, "fn_gameplay"),
    cmd(0x3209, "GetAltTexturesEx", "", false, &ONE_FORM_ONE_OPTIONAL_INT, ARR, Command, "fn_form"),
    cmd(0x320a, "HasHealthDamageEffect", "", true, &NO_PARAMS, NUM, Condition, "fn_gameplay"),
    cmd(0x320b, "SetAlwaysRun", "", false, &ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x320c, "SetAutoMove", "", false, &ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x320d, "SetActorMovementFlags", "SetPlayerMovementFlags", false, &ONE_INT, NUM, Command, "fn_gameplay"),
    cmd(0x320e, "PushUIQuestToTop", "PushSelectedQuestInterfaceListEntryToTheTopOfThePipBoyQuestLogInterfaceImmediately", false, &ONE_QUEST, NUM, Command, "fn_ui"),
    cmd(0x320f, "SetExtraAccuracyPenaltyMult", "", false, &ONE_FLOAT_ONE_OPTIONAL_FORM, NUM, Command, "fn_gameplay"),
    cmd(0x3210, "RemoveExtraAccuracyPenaltyMult", "", false, &ONE_OPTIONAL_FORM, NUM, Command, "fn_gameplay"),
    cmd(0x3211, "SetJohnnyOnSleepWaitEventHandler", "SetONSleepWEventHandler", false, &EVENT_ONE_INT, NUM, Command, "fn_event"),
    cmd(0x3212, "GetExtraAccuracyPenaltyMult", "", false, &ONE_OPTIONAL_FORM, NUM, Condition, "fn_gameplay"),
    cmd(0x3213, "SetCustomMapMarker", "", false, &THREE_FLOATS, NUM, Command, "fn_gameplay"),
    cmd(0x3214, "ClearCustomMapMarker", "", false, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3215, "EjectCasing", "", false, &EJECT_CASING, NUM, Command, "fn_gameplay"),
    cmd(0x3216, "GetNoteSpeaker", "", false, &ONE_FORM, FORM, Command, "fn_form"),
    cmd(0x3217, "SetNoteSpeaker", "", false, &TWO_FORMS, NUM, Command, "fn_form"),
    cmd(0x3218, "GetNoteType", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3219, "SetNoteType", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x321a, "GetNoteSound", "", false, &ONE_FORM, FORM, Command, "fn_form"),
    cmd(0x321b, "SetNoteSound", "", false, &TWO_FORMS, NUM, Command, "fn_form"),
    cmd(0x321c, "GetNoteTopic", "", false, &ONE_FORM, FORM, Command, "fn_form"),
    cmd(0x321d, "SetNoteTopic", "", false, &TWO_FORMS, NUM, Command, "fn_form"),
    cmd(0x321e, "GetNoteImage", "", false, &ONE_FORM, STR, Command, "fn_form"),
    cmd(0x321f, "SetNoteImage", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "fn_form"),
    cmd(0x3220, "GetNoteQuestList", "", false, &ONE_FORM, ARR, Command, "fn_form"),
    cmd(0x3221, "AddNoteQuest", "", false, &TWO_FORMS, NUM, Command, "fn_form"),
    cmd(0x3222, "RemoveNoteQuest", "", false, &TWO_FORMS, NUM, Command, "fn_form"),
    cmd(0x3223, "SetHUDShudderPower", "", false, &ONE_FLOAT, NUM, Command, "fn_gameplay"),
    cmd(0x3224, "GetHUDShudderPower", "", false, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3225, "SetDialogResponseOverrideValues", "", false, &ONE_FORM_FOUR_INTS_TWO_OPTIONAL_FORMS_ONE_OPTIONAL_INT, NUM, Command, "fn_dial"),
    cmd(0x3226, "SetMediaLocationControllerOverride", "SetMLCOverride", false, &ONE_FORM, NUM, Command, "fn_gameplay"),
    cmd(0x3227, "ClearMediaLocationControllerOverride", "ClearMLCOverride", false, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3228, "GetCasinoWinnings", "", false, &ONE_CASINO, NUM, Command, "fn_gameplay"),
    cmd(0x3229, "SetCasinoWinnings", "", false, &ONE_CASINO_ONE_INT, NUM, Command, "fn_gameplay"),
    cmd(0x322a, "GetAcousticSpace", "", true, &ONE_FORM, NUM, Command, "fn_mediaset"),
    cmd(0x322b, "SetAcousticSpace", "", true, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "fn_mediaset"),
    cmd(0x322c, "SetCameraTranslate", "", false, &ONE_BOOL_THREE_FLOATS, NUM, Command, "fn_utility"),
    cmd(0x322d, "SetCameraRotate", "", false, &ONE_BOOL_ONE_INT_ONE_FLOAT, NUM, Command, "fn_utility"),
    cmd(0x322e, "PlayHolotape", "", false, &ONE_FORM_ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x322f, "StopHolotape", "", false, &ONE_OPTIONAL_INT, NUM, Command, "fn_gameplay"),
    cmd(0x3230, "SetOnTakeBackItemEventHandler", "", false, &EVENT_TWO_FORMS, NUM, Command, "fn_event"),
    cmd(0x3231, "GetCasinoDeckTexture", "", false, &ONE_CASINO_ONE_INT, STR, Command, "fn_gameplay"),
    cmd(0x3232, "SetCasinoDeckTexture", "", false, &ONE_CASINO_ONE_INT_ONE_STRING, NUM, Command, "fn_gameplay"),
    cmd(0x3233, "GetCasinoChip", "", false, &ONE_CASINO, FORM, Command, "fn_gameplay"),
    cmd(0x3234, "SetCasinoChip", "", false, &ONE_CASINO_ONE_FORM, NUM, Command, "fn_gameplay"),
    cmd(0x3235, "GetCustomMapMarkerIcon", "", true, &NO_PARAMS, STR, Command, "fn_ui"),
    cmd(0x3236, "GetSleepWaitMenuState", "", false, &NO_PARAMS, NUM, Command, "fn_ui"),
    cmd(0x3237, "SetOnNPCResponseEventHandler", "", false, &EVENT_ONE_INT, NUM, Command, "fn_event"),
    cmd(0x3238, "PlaySoundFromPath", "", false, &ONE_STRING_ONE_OPTIONAL_FLOAT_FOUR_OPTIONAL_INTS, NUM, Command, "fn_sound"),
    cmd(0x3239, "PlaySound3DFromPath", "", true, &ONE_STRING_ONE_OPTIONAL_FLOAT_THREE_OPTIONAL_INTS, NUM, Command, "fn_sound"),
    cmd(0x323a, "StopSoundFromPath", "", false, &ONE_STRING_ONE_OPTIONAL_FLOAT, NUM, Command, "fn_sound"),
    cmd(0x323b, "StopSound3DFromPath", "", true, &ONE_STRING_ONE_OPTIONAL_FLOAT, NUM, Command, "fn_sound"),
    cmd(0x323c, "IsSoundPlayingFromPath", "", false, &ONE_STRING_ONE_OPTIONAL_OBJECT_REF, NUM, Command, "fn_sound"),
    cmd(0x323d, "SetOnGeneralSubtitleEventHandler", "", false, &EVENT_ONE_INT, NUM, Command, "fn_event"),
    cmd(0x323e, "PathToRef", "", true, &PATH_TO_REF, NUM, Command, "fn_gameplay"),
    cmd(0x323f, "SetOnReputationChangeEventHandler", "", false, &EVENT_ONE_FORM, NUM, Command, "fn_event"),
    cmd(0x3240, "IsNiSequenceActive", "", true, &IS_NI_SEQUENCE_ACTIVE, NUM, Command, "fn_gamebryo"),
    cmd(0x3241, "GetHotkeySlot", "", true, &NO_PARAMS, NUM, Command, "fn_form"),
    cmd(0x3242, "SetOnNPCActorValueChangeEventHandler", "SetJohnnyOnNPCActorValueEventHandler", false, &EVENT_ONE_FORM_ONE_INT, NUM, Command, "fn_event"),
    cmd(0x3243, "GetGrenadeHoldTime", "", false, &NO_PARAMS, NUM, Condition, "fn_gameplay"),
    cmd(0x3244, "RemoveHighlightedRef", "", true, &NO_PARAMS, NUM, Command, "fn_gameplay"),
    cmd(0x3245, "GetSaidOnce", "", false, &ONE_FORM, NUM, Condition, "fn_dial"),
    cmd(0x3246, "SetSaidOnce", "", false, &ONE_FORM_TWO_INTS, NUM, Command, "fn_dial"),
    cmd(0x3247, "GetTopicInfo", "", false, &GET_TOPIC_INFO, ARR, Command, "fn_dial"),
    cmd(0x3248, "IsMenuPaused", "", false, &ONE_OPTIONAL_INT, NUM, Condition, "fn_ui"),
    cmd(0x3249, "IsInDialogueWithPlayer", "", true, &NO_PARAMS, NUM, Condition, "fn_gameplay"),
    cmd(0x324a, "SetHUDVisibilityOverride", "", false, &ONE_INT, NUM, Command, "fn_ui"),
    cmd(0x324b, "GetHUDVisibilityOverride", "", false, &NO_PARAMS, NUM, Command, "fn_ui"),
    cmd(0x324c, "UpdateRepairMenu", "", false, &NO_PARAMS, NUM, Command, "fn_ui"),
    cmd(0x324d, "GetWeaponsForMod", "", false, &ONE_OBJECT_ID, ARR, Command, "fn_gameplay"),
    cmd(0x324e, "GetMineArmedEx", "", true, &NO_PARAMS, NUM, Command, "fn_form"),
    cmd(0x324f, "SetMusicTypePath", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "fn_form"),
    cmd(0x3250, "GetParentTopic", "", false, &ONE_FORM, FORM, Command, "fn_dial"),
    cmd(0x3251, "GetCameraShotTraitNumeric", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x3252, "SetCameraShotTraitNumeric", "", false, &ONE_FORM_ONE_INT_ONE_FLOAT, NUM, Command, "fn_form"),
    cmd(0x3253, "GetCameraShotFlags", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3254, "SetCameraShotFlags", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x3255, "GetCameraShotPath", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3256, "SetCameraShotPath", "", false, &ONE_FORM_ONE_STRING, NUM, Command, "fn_form"),
    cmd(0x3257, "GetCameraShotImageSpaceModifier", "", false, &ONE_FORM, FORM, Command, "fn_form"),
    cmd(0x3258, "SetCameraShotImageSpaceModifier", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x3259, "SetAlphaPropertyValue", "", true, &SET_PROPERTY_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x325a, "GetAlphaPropertyValue", "", true, &GET_PROPERTY_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x325b, "SetStencilPropertyValue", "", true, &SET_PROPERTY_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x325c, "GetStencilPropertyValue", "", true, &GET_PROPERTY_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x325d, "PathToPoint", "", true, &THREE_FLOATS_TWO_OPTIONAL_FLOATS, NUM, Command, "fn_gameplay"),
    cmd(0x325e, "GetYieldTimer", "", false, &NO_PARAMS, NUM, Condition, "fn_gameplay"),
    cmd(0x325f, "SetYieldTimer", "", false, &ONE_FLOAT, NUM, Command, "fn_gameplay"),
    cmd(0x3260, "SetSwitchNodeIndex", "", true, &SET_SWITCH_NODE_INDEX, NUM, Command, "fn_gamebryo"),
    cmd(0x3261, "GetSwitchNodeIndex", "", true, &GET_SWITCH_NODE_INDEX, NUM, Command, "fn_gamebryo"),
    cmd(0x3262, "SetNiLODLevel", "", false, &ONE_INT, NUM, Command, "fn_gamebryo"),
    cmd(0x3263, "GetNiLODLevel", "", false, &NO_PARAMS, NUM, Command, "fn_gamebryo"),
    cmd(0x3264, "SetWeaponScopeUIModel", "", false, &ONE_STRING_ONE_OPTIONAL_OBJECT, NUM, Command, "fn_ui"),
    cmd(0x3265, "ToggleWeaponScopeUIModel", "", false, &ONE_INT, NUM, Command, "fn_ui"),
    cmd(0x3266, "ClearWeaponScopeUIModel", "", false, &NO_PARAMS, NUM, Command, "fn_ui"),
    cmd(0x3267, "UpdateScenegraph", "", true, &SCENEGRAPH_UPDATE, NUM, Command, "fn_gamebryo"),
    cmd(0x3268, "GetNiBound", "", true, &GET_NI_BOUND, ARR, Command, "fn_gamebryo"),
    cmd(0x3269, "CallPerRef", "", false, &CALL_PER_REF, NUM, Command, "fn_form"),
    cmd(0x326a, "CallPerRefEx", "", false, &CALL_PER_REF_EX, NUM, Expression, "fn_form"),
    cmd(0x326b, "CallPerMobileObject", "", false, &CALL_PER_MOBILE_OBJECT, NUM, Command, "fn_form"),
    cmd(0x326c, "CallPerMobileObjectEx", "", false, &CALL_PER_MOBILE_OBJECT_EX, NUM, Expression, "fn_form"),
    cmd(0x326d, "Update3DAlt", "", true, &ONE_INT, NUM, Command, "fn_form"),
    cmd(0x326e, "SetNiPSysModifierValue", "", true, &SET_NI_P_SYS_MODIFIER_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x326f, "GetNiPSysModifierValue", "", true, &GET_NI_P_SYS_MODIFIER_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x3270, "ar_Shuffle", "", false, &ONE_INT, ARR, Command, "fn_utility"),
    cmd(0x3271, "GetRecipeCategoryFlags", "", false, &ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3272, "GetCurrentSkyColor", "", false, &ONE_INT_THREE_SCRIPT_VARS, NUM, Command, "fn_utility"),
    cmd(0x3273, "RemapLand", "", false, &REMAP_LAND, NUM, Command, "fn_form"),
    cmd(0x3274, "SetParticleEmitterSpawnRate", "", true, &SET_PARTICLE_EMITTER_SPAWN_RATE, NUM, Command, "fn_gamebryo"),
    cmd(0x3275, "GetParticleEmitterSpawnRate", "", true, &GET_SWITCH_NODE_INDEX, NUM, Command, "fn_gamebryo"),
    cmd(0x3276, "GetItemEffectString", "", false, &ONE_OPTIONAL_OBJECT_ID, STR, Command, "fn_form"),
    cmd(0x3277, "ApplyModelTextureSwap", "", true, &APPLY_MODEL_TEXTURE_SWAP, NUM, Command, "fn_form"),
    cmd(0x3278, "StopIdleLoop", "ForceDeactivateCurrentAnimationLoopForTargetReferenceButDontCancelTheAnimationItself", true, &ONE_OPTIONAL_INT, NUM, Command, "fn_utility"),
    cmd(0x3279, "SetIKState", "", true, &TWO_INTS, NUM, Command, "fn_form"),
    cmd(0x327a, "GetIKState", "", true, &ONE_INT, NUM, Condition, "fn_form"),
    cmd(0x327b, "IsCarryable", "", false, &ONE_OPTIONAL_OBJECT_ID, NUM, Condition, "fn_form"),
    cmd(0x327c, "PickIdleEx", "", true, &ONE_REF, NUM, Command, "fn_form"),
    cmd(0x327d, "GetUsedItemHeight", "", false, &NO_PARAMS, NUM, Condition, "fn_form"),
    cmd(0x327e, "GetMenuItemListIndex", "", true, &ONE_OPTIONAL_INT, NUM, Command, "fn_ui"),
    cmd(0x327f, "SelectMenuItemListIndex", "", false, &ONE_INT_ONE_OPTIONAL_INT, NUM, Command, "fn_ui"),
    cmd(0x3280, "StopNiSequence", "", true, &STOP_NI_SEQUENCE, NUM, Command, "fn_gamebryo"),
    cmd(0x3281, "GetPCInRootWorldspace", "", false, &ONE_WORLD_SPACE, NUM, Condition, "fn_gameplay"),
    cmd(0x3282, "GetPCRootWorldspace", "", false, &NO_PARAMS, FORM, Command, "fn_gameplay"),
    cmd(0x3283, "GetAltTextures", "", false, &ONE_FORM, ARR, Command, "fn_form"),
    cmd(0x3284, "SetAltTexture", "", false, &ONE_FORM_ONE_INT_ONE_FORM, NUM, Command, "fn_form"),
    cmd(0x3285, "ClearAltTexture", "", false, &ONE_FORM_ONE_INT, NUM, Command, "fn_form"),
    cmd(0x3286, "SetNiLightValue", "", true, &SET_NI_LIGHT_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x3287, "GetNiLightValue", "", true, &GET_PROPERTY_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x3288, "SetNiLightColor", "", true, &SET_NI_LIGHT_COLOR, NUM, Command, "fn_gamebryo"),
    cmd(0x3289, "GetNiLightColor", "", true, &GET_NI_LIGHT_COLOR, NUM, Command, "fn_gamebryo"),
    cmd(0x328a, "ReloadEquippedModelsAlt", "ReloadModelsAlt", true, &ONE_OPTIONAL_INT, NUM, Command, "fn_form"),
    cmd(0x328b, "ar_IsFormInList", "", false, &ONE_INT_ONE_FORM_LIST_ONE_INT, NUM, Command, "fn_utility"),
    cmd(0x328c, "GetExternalEmittanceSource", "", false, &ONE_OPTIONAL_FORM, FORM, Command, "fn_form"),
    cmd(0x328d, "SetExternalEmittanceSource", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x328e, "GetProjectileMuzzleFlashLight", "", false, &ONE_FORM, FORM, Command, "fn_form"),
    cmd(0x328f, "SetProjectileMuzzleFlashLight", "", false, &ONE_FORM_ONE_OPTIONAL_FORM, NUM, Command, "fn_form"),
    cmd(0x3290, "SetShaderPropertyFlag", "", true, &SET_PROPERTY_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x3291, "GetShaderPropertyFlag", "", true, &GET_PROPERTY_VALUE, NUM, Command, "fn_gamebryo"),
    cmd(0x3292, "GetCustomReputationChangeIcon", "gcrci", false, &ONE_FORM_ONE_INT, STR, Command, "fn_ui"),
    cmd(0x3293, "GetReputationTitle", "", false, &ONE_REPUTATION, STR, Command, "fn_form"),
    cmd(0x3294, "GetReputationIcon", "", false, &ONE_REPUTATION, STR, Command, "fn_form"),
    cmd(0x3295, "GetReputationFormIcon", "", false, &GET_REPUTATION_FORM_ICON, STR, Command, "fn_form"),
    cmd(0x3296, "SetReputationFormIcon", "", false, &SET_REPUTATION_FORM_ICON, NUM, Command, "fn_form"),
    cmd(0x3297, "ToggleAddonNodes", "", true, &GET_PROPERTY_VALUE, NUM, Command, "fn_gamebryo"),
];
