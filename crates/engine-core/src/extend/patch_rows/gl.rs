//! Group GL's seams and wraps (step 3.9: lights and magic effects). Only group GL's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GL". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "3.9-GL",
    }
}

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G86 (`WriteRelCall(0x406720, GetEffectHiddenHook)`): `EffectItemList::BuildMenuString`
    // 00406620 leaves out effects with no setting or a hidden one.
    row(0x0040_6720),
    // G107, port-side: `BuildMenuString`'s call of `StatsMenu::StatusDataList::Register`
    // (`0x40686D`), the frame JIP's `EffectItemGetModifiedDuration` reads (`[[EBP]-0xD4]`): the
    // list is handed over here.
    row(0x0040_686d),
    // G67 (`WriteRelJump(0x4069D4, EffectSkillModifiers1Hook)`): `BuildMenuString`'s Medicine and
    // Survival scaling of the effect text.
    row(0x0040_69d4),
    // H06 (`WriteRelJump(0x447155, LoadNifRetnNodeHook)`): `ModelLoader::LoadFile` 00447080's
    // model sets up its point lights tagged `LIGH_EDID` once (seam in `modelloader_p3.rs`).
    row(0x0044_7155),
    // H04 (`SafeWrite8(0x465488, 0xCC)`): `TESDataHandler::CreateFormOfType` 00465110 allocates a
    // light form at 0xCC bytes (its editor id's word at +0xC8; seam in `tesdatahandler.rs`).
    row(0x0046_5488),
    // H03 (`SafeWrite32(addr, 0x110)`, twelve places): every point light allocated at 0x110 bytes,
    // JIP's light data after the game's object. `TESObjectLIGH::GenDynamic` 0050d810.
    row(0x0050_da73),
    // H06's companion (`WriteRelJump(0x50EF46, CreateObjectNodeHook)`): `TESBoundObject::Clone3D`
    // 0050ee50 clones, never shares, a model whose root holds JIP's point lights (seam in
    // `tesobject.rs`).
    row(0x0050_ef46),
    // H03: `LoveTesterMenu::InitMachine` 007928e0.
    row(0x0079_2dc0),
    // H03: `SPECIALBookMenu::InitBook` 007c6ce0.
    row(0x007c_6f86),
    // G107 (`WriteRelCall(0x7E0C1F, EffectItemGetModifiedDuration)`): `Register` 007e0b20's
    // duration, a chem's after `ModifyPositiveChemDuration` (seam in `statsmenu.rs`).
    row(0x007e_0c1f),
    // H03: `FORenderedMenu::CreateMenuLight` 007fc400.
    row(0x007f_c4da),
    // H03: `FORenderedMenuRaceSex::Initialize` 007fc970.
    row(0x007f_d49f),
    // H03: `LightEffect::AttachLight` 0080e970.
    row(0x0080_ea39),
    // G66 (26 bytes at `0x80ECA8`): `LightEffect::AttachLight` 0080e970 attaches only to a node.
    row(0x0080_eca8),
    // G67 (`WriteRelJump(0x816519, EffectSkillModifiers2Hook)`): `MagicCaster::FindTargets`
    // 00815d00's alchemy Medicine and Survival scaling.
    row(0x0081_6519),
    // H03: `MagicLight::Init` 0081c030.
    row(0x0081_c095),
    // G107 (`WriteRelJump(0x8236FD, ModPositiveChemDurationHook)`): `MagicTarget::CheckAddEffect`
    // 00823210 applies `ModifyPositiveChemDuration` only to chems (types 10 and 11).
    row(0x0082_36fd),
    // H12 (`WriteRelCall(0x8C7C4E / 0x8C7E18, UpdateAnimatedLightsHook)`): the frame's two task
    // sequences 008c7bd0 and 008c7da0 update the animated lights, then JIP's own point lights
    // (seams in `run_008c7790.rs`).
    row(0x008c_7c4e),
    row(0x008c_7e18),
    // H03: `VATS::SetMode` 009c6c30, modes 3 and 2.
    row(0x009c_6cb6),
    row(0x009c_6d3d),
    // H05 (12 bytes at `0xA68D6B`): `NiDynamicEffect`'s member copy 00a68d40 copies the word
    // +0x9C (JIP's two light bytes with it; seam in `run_00a68c80.rs`).
    row(0x00a6_8d6b),
    // H03: `NiPointLight`'s create function 00a7d6e0 (stream loads) and `CreateClone` 00a7d7c0.
    row(0x00a7_d6e2),
    row(0x00a7_d7c3),
    // H03: `ShadowSceneNode`'s dynamic light 00b5cd00.
    row(0x00b5_cd2a),
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // H12 (`WriteRelJump(0x50DE20, UpdateAnimatedLightHook)`): a light form's animation of its
    // point light (flicker, pulse, JIP's colour shift) is JIP's.
    row(0x0050_de20),
];
