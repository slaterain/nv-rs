//! The game's classes JIP LN NVSE's form accessors read and write, with the fields they need, at
//! their PC offsets. The names are JIP's (`nvse/GameForms.h`, `nvse/GameData.h` (JIP LN NVSE));
//! each offset is checked against the exe, and the function it was read from is named at the
//! class. Classes `engine_core::types` (`TESForm`, `BSStringT`) or `crate::game` (`ListNode`)
//! already have are used from there.

pub use crate::game::ListNode;
use engine_core::layout;
use engine_core::ptr::Inline;
pub use engine_core::types::{BSStringT, TESForm};

/// `TESModel`'s virtual functions JIP calls (`GetModelPath`, `SetModelPath`): the slots at
/// `+0x14` and `+0x18` of `TESModel`'s vtable (`0101CF44`: `00408DA0`, which answers the path's
/// text or `""`, and `00489100`, which copies a text into the path) and of
/// `TESModelTextureSwap`'s (`0101D124`, the same two functions).
pub const GET_MODEL_PATH: u32 = 0x14;
pub const SET_MODEL_PATH: u32 = 0x18;
/// `TESModelTextureSwap`'s vtable (`kVtbl_TESModelTextureSwap` (JIP LN NVSE)) and size.
pub const TES_MODEL_TEXTURE_SWAP_VTBL: u32 = 0x0101_d124;
pub const TES_MODEL_TEXTURE_SWAP_SIZE: u32 = 0x20;
/// `SkyObject::Update` (`+0xC` of `Clouds`' vtable `0104EC08`: `Clouds::Update`, `00634110`,
/// `thiscall`: the sky and a float).
pub const SKY_OBJECT_UPDATE: u32 = 0x0c;
/// `TESForm::GetEditorID` (`+0x130` of a form's vtable, as JIP's `DoConsolePrint` calls it).
pub const GET_EDITOR_ID: u32 = 0x130;
/// `TESForm::kFormFlag_Destructible` (JIP LN NVSE).
pub const FORM_FLAG_DESTRUCTIBLE: u32 = 0x0100_0000;

layout! {
    /// `TESModel` (JIP LN NVSE), 0x18 bytes: a vtable and the model's path.
    pub struct TESModel: 0x18 {
        /// `nifPath` (JIP LN NVSE): read and written only through the virtual functions.
        0x04 nifPath: Inline<BSStringT>,
    }

    /// `TESTexture` (JIP LN NVSE), 0x0C bytes: a vtable and the texture's path
    /// (`TESTexture::LoadTextureChunk` (`TESClimate::Load`, `0052AC90`) loads it at +4).
    pub struct TESTexture: 0x0C {
        0x04 ddsPath: Inline<BSStringT>,
    }

    /// `TESClimate` (JIP LN NVSE), checked in `TESClimate::Load` (`0052AC90`): `MODL` at +0x18,
    /// `WLST` at +0x30, `FNAM` and `GNAM` at +0x38 and +0x44, the six bytes of `TNAM` at +0x50.
    pub struct TESClimate: 0x58 {
        0x18 nightSkyModel: Inline<TESModel>,
        /// `weatherTypes` (JIP LN NVSE): a `tList<WeatherEntry>`, its first node inline.
        0x30 weatherTypes: Inline<ListNode>,
        0x38 sunTexture: Inline<TESTexture>,
        0x44 sunGlareTexture: Inline<TESTexture>,
        0x50 sunriseBegin: u8,
        0x51 sunriseEnd: u8,
        0x52 sunsetBegin: u8,
        0x53 sunsetEnd: u8,
        0x54 volatility: u8,
        /// `phaseLength` (JIP LN NVSE): the moons' phase length in its low six bits, the moons
        /// shown in its top two.
        0x55 phaseLength: u8,
    }

    /// `WeatherEntry` (JIP LN NVSE), 0x0C bytes: an item of a climate's weather list.
    pub struct WeatherEntry: 0x0C {
        0x00 weather: u32,
        0x04 chance: u32,
        0x08 global: u32,
    }

    /// `TESWeather` (JIP LN NVSE), checked in `TESWeather::Load` (`00580990`): the image space
    /// modifiers from +0x1C (`00581FB0`, through +0x18), the cloud textures `DNAM`, `CNAM`,
    /// `ANAM`, `BNAM` at +0x34, +0x40, +0x4C, +0x58, `ONAM` (4 bytes) at +0x64, `PNAM` at +0x68,
    /// `MODL` at +0xC8, `DATA` (15 bytes) at +0xE0, `FNAM` at +0xF0, `NAM0` at +0x108.
    pub struct TESWeather: 0x36C {
        /// `imageSpaceMods[6]` (JIP LN NVSE).
        0x01C imageSpaceMods: u32,
        /// `layerTextures[4]` (JIP LN NVSE), 0x0C bytes each.
        0x034 layerTextures: Inline<TESTexture>,
        /// `cloudSpeed[4]` (JIP LN NVSE).
        0x064 cloudSpeed: u8,
        /// `cloudColor[4][6]` (JIP LN NVSE): one colour per layer and time of day.
        0x068 cloudColor: u32,
        0x0C8 model: Inline<TESModel>,
        0x0E0 windSpeed: u8,
        0x0E3 transDelta: u8,
        /// `sunGlare` (JIP LN NVSE): the first of the seven bytes JIP's traits 6 to 12 address
        /// as `0xDE + trait` (sun glare to lightning frequency).
        0x0E4 sunGlare: u8,
        0x0EB weatherClassification: u8,
        0x0EC lightningColor: u32,
        /// `fogDistance[6]` (JIP LN NVSE).
        0x0F0 fogDistance: f32,
        /// `colors[10][6]` (JIP LN NVSE).
        0x108 colors: u32,
    }

    /// `Sky` (JIP LN NVSE), the fields its commands use; the PC offsets match the engine's
    /// own `Sky` layout (`engine-fallout-env`, checked in the sky's functions).
    pub struct Sky: 0x138 {
        0x00C currClimate: u32,
        0x010 currWeather: u32,
        /// `transWeather` (JIP LN NVSE): the weather fading out in a transition.
        0x014 transWeather: u32,
        0x02C clouds: u32,
        0x0D0 windDirection: f32,
        0x0F4 weatherPercent: f32,
        0x100 lightningFxPerc: f32,
    }

    /// `Clouds` (JIP LN NVSE), the field `ReloadCloudTextures` sets.
    pub struct Clouds: 0x5C {
        0x5A bForceUpdate: u8,
    }

    /// `BGSBodyPartData` (JIP LN NVSE), checked in its loader (`005E4D80`): `MODL` at +0x18, the
    /// parts (0xB0 bytes each, allocated there) from +0x34, `RAGA` at +0x70.
    pub struct BGSBodyPartData: 0x74 {
        0x18 model: Inline<TESModel>,
        /// `bodyParts[15]` (JIP LN NVSE).
        0x34 bodyParts: u32,
        0x70 ragDoll: u32,
    }

    /// `BGSBodyPart` (JIP LN NVSE), checked in `BGSBodyPart::Load` (`005E4100`): the limb
    /// replacement model at +0x2C, `BPND` read at +0x5C (`limbRepScale` +0xAC, `sevrDebrisScale`
    /// +0x84 and `explDebrisScale` +0x74 defaulted there); the order inside `BPND` is the
    /// record's, as JIP lists it.
    pub struct BGSBodyPart: 0xB0 {
        0x1C partName: Inline<BSStringT>,
        0x2C limbReplacement: Inline<TESModel>,
        0x5C damageMult: f32,
        0x60 flags: u8,
        0x62 healthPercent: u8,
        0x63 actorValue: u8,
        0x64 toHitChance: u8,
        0x65 explChance: u8,
        0x66 explDebrisCount: u8,
        0x68 explDebris: u32,
        0x6C explExplosion: u32,
        0x70 trackingMaxAngle: f32,
        0x74 explDebrisScale: f32,
        0x78 sevrDebrisCount: u8,
        0x7C sevrDebris: u32,
        0x80 sevrExplosion: u32,
        0x84 sevrDebrisScale: f32,
        /// `goreEffTranslate[3]` (JIP LN NVSE).
        0x88 goreEffTranslate: f32,
        /// `goreEffRotation[3]` (JIP LN NVSE), in radians.
        0x94 goreEffRotation: f32,
        0xA0 sevrImpactDS: u32,
        0xA4 explImpactDS: u32,
        0xA8 sevrDecalCount: u8,
        0xA9 explDecalCount: u8,
        0xAC limbRepScale: f32,
    }

    /// `BGSDestructibleObjectForm` (JIP LN NVSE), 8 bytes: a vtable and its data (the engine's
    /// `BGSDestructibleObjectForm::pData`, `engine-fallout-forms`).
    pub struct BGSDestructibleObjectForm: 0x08 {
        0x04 data: u32,
    }

    /// `DestructibleData` (JIP LN NVSE), 0x14 bytes (the engine's `DestructibleObjectData`).
    pub struct DestructibleData: 0x14 {
        0x00 health: u32,
        0x04 stageCount: u8,
        0x05 targetable: u8,
        /// `stages` (JIP LN NVSE): `DestructionStage**`.
        0x08 stages: u32,
        0x0C unk0C: u32,
        0x10 unk10: u32,
    }

    /// `DestructionStage` (JIP LN NVSE), 0x18 bytes (the engine's `DestructibleObjectStage`).
    pub struct DestructionStage: 0x18 {
        0x00 dmgStage: u8,
        0x01 healthPrc: u8,
        /// `flags` (JIP LN NVSE): a `UInt16` in JIP (the game's own `cFlags` is the low byte).
        0x02 flags: u16,
        0x04 selfDmgSec: u32,
        0x08 explosion: u32,
        0x0C debris: u32,
        0x10 debrisCount: u32,
        0x14 replacement: u32,
    }
}

/// `Sky::Get` (JIP's `g_currentSky`, read from `0x11DEA20` at JIP's deferred start; the game
/// makes its sky once, so reading the word each time gives the same pointer).
pub const SKY: u32 = 0x011d_ea20;

/// Form types (`kFormType_*` (JIP LN NVSE)), the byte at `TESForm +4`.
pub mod form_type {
    pub const TES_WEATHER: u8 = 0x35;
    pub const TES_CLIMATE: u8 = 0x36;
    pub const BGS_EXPLOSION: u8 = 0x51;
    pub const BGS_DEBRIS: u8 = 0x52;
    pub const BGS_BODY_PART_DATA: u8 = 0x57;
    pub const BGS_IMPACT_DATA_SET: u8 = 0x5f;
    pub const BGS_RAGDOLL: u8 = 0x63;
}

/// `kBaseComponentOffsets` (JIP LN NVSE, `nvse/GameForms.cpp`): per form type, four signed
/// bytes giving where a component lies in the form, in words (0 for none); byte 2 is the
/// destructible component (`GetDestructibleForm`), byte 3 the model (`GetModelPath`).
pub const BASE_COMPONENT_OFFSETS: [u32; 121] = [
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000006,
    0x00000006, 0x09000006, 0x09000006, 0x00000006, 0x00000006, 0x00000000, 0x00000000, 0x00000000,
    0x0600000E, 0x00000000, 0x00000000, 0x00000006, 0x00000006, 0x0F1A170C, 0x0F1A170C, 0x0F1A170C,
    0x1F530F0C, 0x0F271A0C, 0x00000000, 0x121F1A0F, 0x0F1A170C, 0x13251E0C, 0x0F251E0C, 0x0F211A0C,
    0x0C000000, 0x0C000000, 0x0CFE00FB, 0x0C000000, 0x0C000000, 0x0C150000, 0x00000000, 0x0F1A170C,
    0x0F2D1A0C, 0x0F22270C, 0x37413D34, 0x37413D34, 0x13000000, 0x13000000, 0x0F211A0C, 0x1329220C,
    0x00000000, 0x0C000012, 0x00000000, 0x0F16000C, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000006, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000006, 0x00000000, 0x00000000, 0x00000000, 0x00000006, 0x00000000, 0x0000060C,
    0x06000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x06000000, 0x00000006, 0x00000000,
    0x00000000, 0x0F00000C, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000006, 0x06000000,
    0x0C000000, 0x00000006, 0x00000000, 0x06000000, 0x00000000, 0x00000000, 0x06000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000006, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x0F231A0C,
    0x00000006, 0x00000000, 0x00000006, 0x00000006, 0x0F20000C, 0x00000006, 0x00000000, 0x00000006,
    0x00000006, 0x00000B06, 0x00000006, 0x0F00200C, 0x0F00000C, 0x00000006, 0x00000000, 0x00000000,
    0x00000000,
];

layout! {
    /// Any game object with a vtable: its vtable word (JIP's `IS_TYPE` compares it).
    pub struct Object: 0x04 {
        0x00 vtbl: u32,
    }

    /// `TESAmmo` (JIP LN NVSE), checked in its loader (`00502CD0`): `DATA` (8 bytes) at +0xA8,
    /// `DAT2` (0x14 bytes: projectiles per shot, projectile, weight, casing, casing chance) at
    /// +0xB0; the clip rounds component at +0x80 (its count at +4).
    pub struct TESAmmo: 0xDC {
        0xA8 speed: f32,
        /// `clipRounds.clipRounds` (JIP LN NVSE).
        0x84 clipRounds: u8,
        0xB0 projPerShot: u32,
        0xB4 projectile: u32,
        0xBC casingItem: u32,
        0xC0 casingChance: f32,
        /// `effectList` (JIP LN NVSE): a `tList<TESAmmoEffect>`, its first node inline.
        0xD4 effectList: Inline<ListNode>,
    }

    /// `TESAmmoEffect` (JIP LN NVSE), 0x30 bytes.
    pub struct TESAmmoEffect: 0x30 {
        0x24 effectType: u32,
        0x28 operation: u32,
        0x2C value: f32,
    }

    /// `TESClass` (JIP LN NVSE), checked in its loader (`005F6930`: `DATA`, 0x1C bytes, at
    /// +0x44: the four tag skills, the flags +0x54, the services +0x58, +0x5C, +0x5D);
    /// `TESAttributes` at +0x38, its seven bytes from +4.
    pub struct TESClass: 0x60 {
        /// `attributes.attributes[7]` (JIP LN NVSE).
        0x3C attributes: u8,
        /// `tagSkills[4]` (JIP LN NVSE).
        0x44 tagSkills: u32,
        0x54 classFlags: u32,
        0x58 services: u32,
    }

    /// `TESGrass` (JIP LN NVSE), checked in its loader (`00509F80`): `MODL` at +0x30, `DATA`
    /// (0x20 bytes) at +0x48.
    pub struct TESGrass: 0x68 {
        0x30 model: Inline<TESModel>,
        0x48 density: u8,
        0x49 minSlope: u8,
        0x4A maxSlope: u8,
        0x4C unitFromWaterAmount: u16,
        0x50 unitFromWaterType: u8,
        0x54 positionRange: f32,
        0x58 heightRange: f32,
        0x5C colorRange: f32,
        0x60 wavePeriod: f32,
        0x64 grassFlags: u8,
    }

    /// `TESObjectLIGH` (JIP LN NVSE), checked in its loader (`0050CAF0`): `DATA` (0x18 bytes:
    /// time, radius, colour, flags, falloff, field of view) at +0x9C, `FNAM` (fade) at +0xB4.
    pub struct TESObjectLIGH: 0xC8 {
        0xA0 radius: u32,
        0xA4 red: u8,
        0xA5 green: u8,
        0xA6 blue: u8,
        0xA8 lightFlags: u32,
        0xAC falloffExp: f32,
        0xB0 fov: f32,
        0xB4 fadeValue: f32,
    }

    /// `TESWaterForm` (JIP LN NVSE), checked in its loader (`0057EF80`: the noise texture
    /// `NNAM` at +0x64, `FNAM` at +0x71); the sound and drink effect as JIP's header gives them.
    pub struct TESWaterForm: 0x194 {
        0x064 noiseMap: Inline<TESTexture>,
        0x07C sound: u32,
        0x178 drinkEffect: u32,
    }

    /// `BGSTextureSet` (JIP LN NVSE), checked in its loader (`005928A0`: `TX00`-`TX05` at
    /// +0x38 in 0xC steps, `DNAM` at +0x84); the decal data (`DODT`) pointed to from +0x80.
    pub struct BGSTextureSet: 0xA0 {
        /// `textures[6]` (JIP LN NVSE).
        0x38 textures: Inline<TESTexture>,
        0x80 decalInfo: u32,
        0x84 texFlags: u16,
    }

    /// `DecalInfo` (JIP LN NVSE): seven floats (widths, heights, depth, shininess, parallax
    /// scale), the parallax passes, the flags and the colour.
    pub struct DecalInfo: 0x24 {
        0x00 minWidth: f32,
        0x1C parallaxPasses: u8,
        0x1D flags: u8,
        0x20 color: u32,
    }
}

/// Vtables JIP's `IS_TYPE` compares (`kVtbl_*` (JIP LN NVSE), `internal/class_vtbls.h`).
pub const TES_AMMO_VTBL: u32 = 0x0102_6064;

/// More form types (`kFormType_*` (JIP LN NVSE)).
pub mod form_type_more {
    pub const BGS_TEXTURE_SET: u8 = 0x04;
    pub const TES_SOUND: u8 = 0x0d;
    pub const SPELL_ITEM: u8 = 0x14;
    pub const TES_OBJECT_LIGH: u8 = 0x1e;
    pub const TES_GRASS: u8 = 0x24;
    pub const BGS_PROJECTILE: u8 = 0x33;
    pub const TES_WATER_FORM: u8 = 0x4e;
    pub const TES_AMMO_EFFECT: u8 = 0x72;
    pub const TES_QUEST: u8 = 0x47;
    pub const BGS_PERK: u8 = 0x56;
    pub const TES_LAND_TEXTURE: u8 = 0x12;
    pub const ENCHANTMENT_ITEM: u8 = 0x13;
    pub const TES_WORLD_SPACE: u8 = 0x41;
    pub const TES_IMAGE_SPACE: u8 = 0x53;
    pub const TES_IMAGE_SPACE_MODIFIER: u8 = 0x54;
    pub const BGS_IMPACT_DATA: u8 = 0x5e;
}

/// `kInventoryType` (JIP LN NVSE, `internal/jip_core.cpp`): per form type, whether it is an
/// item that can be in an inventory.
pub const INVENTORY_TYPE: [bool; 121] = {
    let mut t = [false; 121];
    let items = [
        0x18, 0x19, 0x1a, 0x1d, 0x1e, 0x1f, 0x28, 0x29, 0x2e, 0x2f, 0x31, 0x32, 0x34, 0x67, 0x6c,
        0x73, 0x74,
    ];
    let mut i = 0;
    while i < items.len() {
        t[items[i]] = true;
        i += 1;
    }
    t
};

layout! {
    /// `TESModelTextureSwap` (JIP LN NVSE), 0x20 bytes: a `TESModel` and its list of alternate
    /// textures.
    pub struct TESModelTextureSwap: 0x20 {
        0x04 nifPath: Inline<BSStringT>,
        /// `textureList` (JIP LN NVSE): a `tList<Texture>`, its first node inline.
        0x18 textureList: Inline<ListNode>,
    }

    /// `TESObjectARMO` (JIP LN NVSE), 0x190 bytes, checked in the armour loader (`00512FD0`):
    /// `BMDT` (part mask, flags) at +0x74/+0x78; `MODL`/`MODS` into +0x7C, `MOD3`/`MO3S` +0x9C,
    /// `MOD2`/`MO2S` +0xBC, `MOD4`/`MO4S` +0xDC (the four models in a row, 0x20 bytes each);
    /// `BMCT` set through the virtual `SetModelPath` of the model at +0x134; the override-sounds
    /// byte at +0x18C deciding whether `TNAM` (the template) or the sound list is the word at
    /// +0x188.
    pub struct TESObjectARMO: 0x190 {
        /// `bipedModel.bipedFlags` (JIP LN NVSE).
        0x078 bipedFlags: u32,
        /// `bipedModel.bipedModel[0]` (JIP LN NVSE): the first of the four models.
        0x07C bipedModels: Inline<TESModelTextureSwap>,
        /// `bipedModel.modelRDT.nifPath` (JIP LN NVSE).
        0x138 modelRDTPath: Inline<BSStringT>,
        /// `audioTemplate` (JIP LN NVSE), or the movement sounds' list when the armour overrides
        /// its sounds.
        0x188 audioTemplate: u32,
        0x18C overrideSounds: u8,
    }

    /// `BGSExplosion` (JIP LN NVSE), 0xA8 bytes, checked in the explosion loader (`004FB0D0`,
    /// engine-fallout-forms `bgsexplosion.rs`): the enchantable component at +0x54, the image
    /// space modifiable one at +0x68, the placed object at +0x70, `DATA` (0x34 bytes) at +0x74.
    pub struct BGSExplosion: 0xA8 {
        /// `enchantable.enchantItem` (JIP LN NVSE).
        0x58 enchantItem: u32,
        /// `imageSpaceModForm.imod` (JIP LN NVSE).
        0x6C imod: u32,
        0x70 placedObj: u32,
        0x74 force: f32,
        0x78 damage: f32,
        0x7C radius: f32,
        0x80 light: u32,
        0x84 sound1: u32,
        0x88 explFlags: u32,
        0x8C ISradius: f32,
        0x90 impactDataSet: u32,
        0x94 sound2: u32,
        0x98 RADlevel: f32,
        0x9C dissipationTime: f32,
        0xA0 RADradius: f32,
        /// `soundLevel` (JIP LN NVSE): a `UInt8` in JIP (the game's word, read as its low byte).
        0xA4 soundLevel: u8,
    }

    /// `BGSImpactData` (JIP LN NVSE), 0x78 bytes, checked in the impact data loader (`0058DEA0`:
    /// `MODL` at +0x18, `DATA` (0x18 bytes) at +0x30, `DODT` read into the decal from +0x54).
    pub struct BGSImpactData: 0x78 {
        0x18 model: Inline<TESModel>,
        0x30 effectDuration: f32,
        0x34 effectOrientation: u8,
        0x38 angleThreshold: f32,
        0x40 soundLevel: u8,
        0x44 noDecalData: u8,
        /// `textureSet` (JIP LN NVSE), then `sound1` and `sound2` at +0x4C and +0x50.
        0x48 textureSet: u32,
        0x54 decalInfo: Inline<DecalInfo>,
    }

    /// `BGSImpactDataSet` (JIP LN NVSE), 0x4C bytes, checked in its loader (`0058EC00`: `DATA`,
    /// 0x30 bytes, at +0x1C).
    pub struct BGSImpactDataSet: 0x4C {
        /// `impactDatas[12]` (JIP LN NVSE), one per material.
        0x1C impactDatas: u32,
    }

    /// `TESLandTexture` (JIP LN NVSE), 0x28 bytes, checked in the land texture loader
    /// (`005412A0`: `TNAM` at +0x18, `HNAM` (3 bytes) at +0x1C, `SNAM` at +0x1F).
    pub struct TESLandTexture: 0x28 {
        0x18 textureSet: u32,
        0x1C materialType: u8,
        0x1D friction: u8,
        0x1E restitution: u8,
        0x1F specularExponent: u8,
        /// `grasses` (JIP LN NVSE): a `tList<TESGrass>`, its first node inline.
        0x20 grasses: Inline<ListNode>,
    }

    /// `TESCombatStyle` (JIP LN NVSE), 0x108 bytes, checked in the combat style loader
    /// (`00505180`: `CSSD` at +0x18, `CSTD` at +0x58, `CSAD` at +0xB4). Each field below is the
    /// first of a run JIP indexes (`jip_fn_combat_style`).
    pub struct TESCombatStyle: 0x108 {
        0x18 coverSearchRadius: f32,
        0x40 weaponRestrictions: u8,
        0x44 rangedWeapRangeMultMax: f32,
        0x58 dodgeChance: u8,
        0x5C dodgeLRTimerMin: f32,
        0x7C blockChance: u8,
        0x80 staggerBonusToAttack: f32,
        0x8C powerAttackChance: u8,
        0x90 staggerBonusToPower: f32,
        0x98 powerAttackN: u8,
        0xA0 holdTimerMin: f32,
        0xA8 csFlags: u16,
        0xAC acrobaticDodgeChance: u8,
        0xB0 rushAttackDistMult: f32,
    }

    /// `TESGlobal` (JIP LN NVSE), 0x28 bytes, checked in the global loader (`005A64B0`: `FNAM`
    /// at +0x20, `FLTV` at +0x24).
    pub struct TESGlobal: 0x28 {
        /// JIP's union of `float data` and `UInt32 uRefID` (a form id, for JIP's global refs):
        /// the word, its float read and written through its bits.
        0x24 data: u32,
    }

    /// `TESFaction` (JIP LN NVSE), checked in the faction loader (`005FCB70`: the reputation id
    /// stored at +0x38).
    pub struct TESFaction: 0x4C {
        0x38 reputation: u32,
    }

    /// `TESPackage` (JIP LN NVSE), checked in the package loader (`00673A40`: `PKDT`'s first word
    /// at +0x1C).
    pub struct TESPackage: 0x80 {
        0x1C packageFlags: u32,
    }

    /// `TESWorldSpace` (JIP LN NVSE), 0xEC bytes, checked in the worldspace loader (`00583560`:
    /// `DATA` (a byte) at +0x4C, `PNAM` at +0x4E); the other fields as JIP's header gives them.
    pub struct TESWorldSpace: 0xEC {
        /// `cell` (JIP LN NVSE): the persistent cell.
        0x34 cell: u32,
        0x40 climate: u32,
        0x44 imageSpace: u32,
        0x4C worldFlags: u8,
        0x4E parentFlags: u16,
        0x70 parent: u32,
        0x74 waterFormFirst: u32,
        0xE0 waterNoiseTexture: Inline<TESTexture>,
    }

    /// `TESObjectCELL` (JIP LN NVSE), the field the worldspace commands read (`worldSpace`
    /// +0xC0, as xNVSE's cell commands read it).
    pub struct TESObjectCELL: 0xE0 {
        0xC0 worldSpace: u32,
    }

    /// `TESImageSpace` (JIP LN NVSE), checked in the image space loader (`0052B440`).
    pub struct TESImageSpace: 0xB0 {
        /// `traitValues[33]` (JIP LN NVSE).
        0x18 traitValues: f32,
    }

    /// `TESImageSpaceModifier` (JIP LN NVSE), checked in the modifier loader (`0052D510`: the key
    /// arrays at +0x654, +0x704, +0x70C). The interpolator values are the first interpolator's
    /// (`value`, +0xC in each).
    pub struct TESImageSpaceModifier: 0x730 {
        0x020 animable: u8,
        0x024 duration: f32,
        0x0E8 radBlurUseTarget: u8,
        0x0EC radialBlurCentreX: f32,
        0x0F0 radialBlurCentreY: f32,
        0x100 DoFUseTarget: u8,
        0x101 DoFMode: u8,
        /// `fltIntrpl1[0].value` (JIP LN NVSE); 0x18 bytes per interpolator.
        0x120 fltIntrpl1Value: f32,
        /// `clrIntrpl[0].value[0]` (JIP LN NVSE); 0x24 bytes per interpolator.
        0x540 clrIntrplValue: f32,
        /// `fltIntrpl2[0].value` (JIP LN NVSE).
        0x588 fltIntrpl2Value: f32,
        /// `data654[44]` (JIP LN NVSE): `InterpKey<float>*`.
        0x654 data654: u32,
        /// `data704[2]` (JIP LN NVSE): `InterpKey<NiColorAlpha>*`.
        0x704 data704: u32,
        /// `data70C[9]` (JIP LN NVSE): `InterpKey<float>*`.
        0x70C data70C: u32,
    }

    /// `InterpKey<float>` (JIP LN NVSE): a time and a value.
    pub struct FloatKey: 0x08 {
        0x04 value: f32,
    }

    /// `InterpKey<NiColorAlpha>` (JIP LN NVSE): a time and four values.
    pub struct ColorKey: 0x14 {
        0x04 value: f32,
    }
}

/// More vtables `IS_TYPE` compares (`kVtbl_*` (JIP LN NVSE)).
pub const TES_OBJECT_ARMO_VTBL: u32 = 0x0102_a62c;
pub const TES_OBJECT_ARMA_VTBL: u32 = 0x0102_a31c;

layout! {
    /// `TESCasino` (JIP LN NVSE), 0x228 bytes, checked in the casino loader (`005048D0`: `DATA`,
    /// 0x38 bytes, at +0x1E8).
    pub struct TESCasino: 0x228 {
        0x1E8 shufflePercent: f32,
        0x1EC blackjackPayout: f32,
        /// `reelStops[7]` (JIP LN NVSE).
        0x1F0 reelStops: u32,
        0x20C numDecks: u32,
        0x210 maxWinnings: u32,
        0x21C casinoFlags: u32,
    }

    /// `TESObjectWEAP` (JIP LN NVSE), the fields the LN weapon commands use, checked in the
    /// weapon loader (`0051F630`: `DNAM` at +0xF4, `CRDT` at +0x1C0, `VATS` at +0x370).
    pub struct TESObjectWEAP: 0x388 {
        /// `textureSwap` (JIP LN NVSE): the weapon's own model.
        0x03C textureSwap: Inline<TESModelTextureSwap>,
        /// `pickupPutdownSounds` (+0xE8, JIP LN NVSE): its pickup and putdown sounds.
        0x0EC pickupSound: u32,
        0x0F0 putdownSound: u32,
        0x11F embedWeaponAV: u8,
        0x128 onHit: u32,
        /// `semiAutoFireDelay[2]` (JIP LN NVSE): minimum, maximum (the loader's defaults 0.1, 0.5).
        0x174 semiAutoFireDelay: f32,
        /// `effectMods[3]` (JIP LN NVSE).
        0x180 effectMods: u32,
        /// `value1Mod[3]` (JIP LN NVSE).
        0x18C value1Mod: f32,
        0x1A1 modReloadAnim: u8,
        0x1A8 killImpulse: f32,
        0x1B8 impulseDist: f32,
        0x1C8 critDamageFlags: u8,
        0x1D0 shellCasingModel: Inline<TESModel>,
        /// `sounds[12]` (JIP LN NVSE).
        0x21C sounds: u32,
        /// `modModels[7]` (JIP LN NVSE).
        0x270 modModels: Inline<TESModelTextureSwap>,
        /// `itemMod[3]` (JIP LN NVSE).
        0x350 itemMod: u32,
        0x35C embeddedNodeName: Inline<BSStringT>,
        0x364 soundLevel: u32,
        0x370 VATSEffect: u32,
    }

    /// `TESRace` (JIP LN NVSE), checked in the race loader (`00610CD0`: the voices through
    /// `00610C70`, `ONAM` +0x504, `YNAM` +0x508).
    pub struct TESRace: 0x524 {
        /// `voiceTypes[2]` (JIP LN NVSE): male, female.
        0x4FC voiceTypes: u32,
        /// `ageRace[2]` (JIP LN NVSE): older, younger.
        0x504 ageRace: u32,
    }

    /// `TESNPC` (JIP LN NVSE), the race (`race.race`): the NPC loader (`00602190`) stores `RNAM`
    /// through `006ECD40` (which writes `[ecx+4]`) on the race component at +0x10C.
    pub struct TESNPC: 0x20C {
        0x110 race: u32,
    }
}

layout! {
    /// `TESChallenge` (JIP LN NVSE), 0x7C bytes, checked in the challenge loader (`005F51A0`:
    /// `DATA`, 0x18 bytes, at +0x54); `SNAM`/`XNAM` as JIP's header gives them.
    pub struct TESChallenge: 0x7C {
        /// `data.type` (JIP LN NVSE).
        0x54 kind: u32,
        0x58 threshold: u32,
        0x5C flags: u32,
        0x60 interval: u32,
        0x64 value1: u16,
        0x66 value2: u16,
        0x68 value3: u32,
        0x74 SNAM: u32,
        0x78 XNAM: u32,
    }
}

layout! {
    /// `ExtraMapMarker::MarkerData` (JIP LN NVSE), checked in `MapMarkerData::Load` (`00438CA0`:
    /// `FNAM` at +0xC, `FULL` into the name at +0, `TNAM` (16 bits) at +0xE, `WMI1` after it).
    pub struct MapMarkerData: 0x14 {
        /// `fullName.name` (JIP LN NVSE).
        0x04 name: Inline<BSStringT>,
        0x0C flags: u8,
        /// `type` (JIP LN NVSE).
        0x0E kind: u16,
        0x10 reputation: u32,
    }
}

/// Form types of the LN commands (`kFormType_*` (JIP LN NVSE)).
pub mod form_type_ln {
    pub const TES_RACE: u8 = 0x0c;
    pub const TES_OBJECT_WEAP: u8 = 0x28;
    pub const TES_NPC: u8 = 0x2a;
    pub const BGS_VOICE_TYPE: u8 = 0x5d;
    pub const TES_REPUTATION: u8 = 0x68;
    pub const TES_CASINO: u8 = 0x6d;
    pub const TES_OBJECT_ACTI: u8 = 0x15;
    pub const BGS_TALKING_ACTIVATOR: u8 = 0x16;
    pub const BGS_PLACEABLE_WATER: u8 = 0x23;
    pub const TES_OBJECT_IMOD: u8 = 0x67;
}

layout! {
    /// `TESObjectACTI` (JIP LN NVSE), checked in the activator loader (`00510A90`: `SNAM` +0x74,
    /// `VNAM` +0x78, `WNAM` +0x80, `RNAM` +0x84 through their setters; `XATO` into the prompt).
    pub struct TESObjectACTI: 0x90 {
        0x74 loopingSound: u32,
        0x78 activationSound: u32,
        0x80 waterType: u32,
        0x84 radioStation: u32,
        0x88 activationPrompt: Inline<BSStringT>,
    }

    /// `BGSPlaceableWater` (JIP LN NVSE), the water (+0x4C).
    pub struct BGSPlaceableWater: 0x50 {
        0x4C water: u32,
    }

    /// A perk entry of the quest or ability kind (JIP LN NVSE): its form (the quest or the spell,
    /// +8) and a quest entry's stage (+0xC); the loaders `005E9B00`/`005E9E20` read `DATA` at +8.
    pub struct BGSQuestPerkEntry: 0x10 {
        /// `quest` / `ability` (JIP LN NVSE).
        0x08 form: u32,
        0x0C stage: u8,
    }

    /// `BGSEntryPointPerkEntry` (JIP LN NVSE), checked in its loader (`005EA790`: `DATA`, 3 bytes,
    /// at +8; the function data at +0xC).
    pub struct BGSEntryPointPerkEntry: 0x14 {
        0x08 entryPoint: u8,
        0x0C data: u32,
    }

    /// An entry point's function data (JIP LN NVSE): one or two floats from +4, a leveled list at
    /// +4, or an activation choice's label at +4, by its vtable.
    pub struct EntryPointData: 0x14 {
        /// `value` / `value[0]` (JIP LN NVSE).
        0x04 value: f32,
        /// `value[1]` (JIP LN NVSE).
        0x08 value2: f32,
    }
}

#[allow(non_upper_case_globals)]
impl EntryPointData {
    /// `leveledList` (JIP LN NVSE): the word at +4 of leveled list data.
    pub const word: engine_core::ptr::Field<EntryPointData, u32> =
        engine_core::ptr::Field::new(0x04);
    /// `label` (JIP LN NVSE): the game string at +4 of activation choice data.
    pub const label: engine_core::ptr::Field<EntryPointData, Inline<BSStringT>> =
        engine_core::ptr::Field::new(0x04);
}

/// Form types of batch 6 (`kFormType_*` (JIP LN NVSE)).
pub mod form_type_b6 {
    pub const TES_RECIPE: u8 = 0x6a;
    pub const TES_RECIPE_CATEGORY: u8 = 0x6b;
}

/// `kVtbl_BGSQuestObjective` (JIP LN NVSE; the objective constructor `005EC090` stores it) and
/// `kVtbl_ImageSpaceModifierInstanceForm` (JIP LN NVSE).
pub const BGS_QUEST_OBJECTIVE_VTBL: u32 = 0x0104_7088;
pub const IMOD_INSTANCE_FORM_VTBL: u32 = 0x0102_d12c;

layout! {
    /// `TESRecipe` (JIP LN NVSE), 0x5C bytes, checked in its loader (`005A8110`: `DATA`, 0x10
    /// bytes, at +0x24; `CTDA` into the condition list +0x34; `RCIL` components appended to +0x3C,
    /// `RCOD` to +0x44) and its save (`005A8490`: the category +0x54, the subcategory +0x58).
    pub struct TESRecipe: 0x5C {
        0x24 reqSkill: u32,
        0x28 reqSkillLevel: u32,
        0x2C categoryID: u32,
        0x30 subCategoryID: u32,
        0x34 conditions: Inline<ListNode>,
        0x3C inputs: Inline<ListNode>,
        0x44 outputs: Inline<ListNode>,
        0x54 category: u32,
        0x58 subCategory: u32,
    }

    /// `RecipeComponent` (JIP LN NVSE), 8 bytes: the loader's `RCQY` sets +0 (`00537E90`), the
    /// item +4 (`006ECD40`).
    pub struct RecipeComponent: 0x08 {
        0x00 quantity: u32,
        0x04 item: u32,
    }

    /// `Condition` (JIP LN NVSE; the exe's `TESConditionItem`), 0x1C bytes: the condition loader
    /// (`00680960`) allocates 0x1C and `00681950` reads the `CTDA` record into it whole (type,
    /// comparison value, function, two parameters, run on, reference).
    pub struct Condition: 0x1C {
        0x00 cType: u8,
        0x04 comparisonValue: f32,
        0x08 opcode: u32,
        0x0C parameter1: u32,
        0x10 parameter2: u32,
        0x14 runOnType: u32,
        0x18 reference: u32,
    }

    /// `BGSQuestObjective` (JIP LN NVSE), 0x24 bytes, checked in its constructor (`005EC090`,
    /// 0x24 allocated by the quest loader `0060CEB0`): the id +4, the text (a game string) +8,
    /// the quest +0x10, the targets (a `tList`) +0x14, the status +0x20.
    pub struct BGSQuestObjective: 0x24 {
        0x04 objectiveId: u32,
        0x08 displayText: Inline<BSStringT>,
        0x10 quest: u32,
        0x14 targets: Inline<ListNode>,
        0x20 status: u32,
    }

    /// `BGSQuestObjective::Target` (JIP LN NVSE), 0x48 bytes, checked in its constructor
    /// (`0060FF70`, 0x48 allocated by the quest loader): the conditions (a `tList`) +4, the
    /// target +0xC, the path data +0x10 whose teleport links (`BSSimpleArray`: data +4, size +8,
    /// 0x10-byte links, the door at +0) are at +0x20 (as `00952BA0`'s callers read them).
    pub struct ObjectiveTarget: 0x48 {
        0x04 conditions: Inline<ListNode>,
        0x0C target: u32,
        0x10 data: Inline<ObjectiveTargetData>,
    }

    /// `BGSQuestObjective::Target::Data` (JIP LN NVSE), 0x38 bytes: its teleport links, a
    /// `BSSimpleArray` at +0x10 (data +0x14, size +0x18).
    pub struct ObjectiveTargetData: 0x38 {
        0x14 linksData: u32,
        0x18 linksSize: u32,
    }

    /// `ImageSpaceModifierInstance(Form)` (JIP LN NVSE, from its header only: only read here):
    /// hidden +8; a form instance's modifier +0x1C.
    pub struct ImageSpaceModifierInstanceForm: 0x30 {
        0x08 hidden: u8,
        0x1C imageSpaceMod: u32,
    }

    /// `TES` (JIP LN NVSE, from its header only: only read here), `*0x011DEA10`: the active
    /// image space modifiers (a `tList`) +0x6C.
    pub struct Tes: 0xC4 {
        0x6C activeIMODs: Inline<ListNode>,
    }

    /// `DataHandler` (JIP LN NVSE, from its header only: only read here), `*0x011C3F2C`: the
    /// recipes (a `tList`) +0xA0.
    pub struct DataHandler: 0x63C {
        0xA0 recipeList: Inline<ListNode>,
    }
}

/// Form types of batch 7 (`kFormType_*` (JIP LN NVSE)).
pub mod form_type_b7 {
    pub const TES_GLOBAL: u8 = 0x06;
    pub const TES_LEV_CREATURE: u8 = 0x2c;
    pub const TES_LEV_CHARACTER: u8 = 0x2d;
    pub const TES_LEV_ITEM: u8 = 0x34;
}

/// `GetLvlList` (JIP LN NVSE): a leveled form's `TESLeveledList` component is at +0x30.
pub const LEVELED_LIST_COMPONENT: u32 = 0x30;

layout! {
    /// `TESLeveledList` (JIP LN NVSE), the component at +0x30 of a leveled form, checked in the
    /// leveled item loader (`0050B560`: `LVLD` into +0xC through `0042E680`, `LVLF` into +0xD,
    /// `LVLG` into +0x10, all of the form at +0x30) and the game's `AddItem` (`00487A60`, the
    /// entries' list at +4).
    pub struct TESLeveledList: 0x1C {
        0x04 list: Inline<ListNode>,
        0x0C chanceNone: u8,
        0x0D flags: u8,
        0x10 global: u32,
    }

    /// `TESLeveledList::ListData` (JIP LN NVSE), 0xC bytes, as the game's `AddItem` (`00487A60`)
    /// fills one: the form +0, the count +4, the level +6 (16 bits each), the extra +8.
    pub struct LeveledListData: 0x0C {
        0x00 form: u32,
        0x04 count: i16,
        0x06 level: i16,
        0x08 extra: u32,
    }

    /// `ContainerExtra` (JIP LN NVSE; the exe's `ContainerItemExtra`), 0xC bytes: its
    /// constructor (`0040E690`) clears the owner +0 and the global +4 and sets the health +8.
    pub struct ContainerExtra: 0x0C {
        0x00 ownerFaction: u32,
        0x04 globalVar: u32,
        0x08 health: f32,
    }
}
