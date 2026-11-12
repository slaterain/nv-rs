//! The game's classes JIP LN NVSE's form accessors read and write, with the fields they need, at
//! their PC offsets. The names are JIP's (`nvse/GameForms.h`, `nvse/GameData.h` (JIP LN NVSE));
//! each offset is checked against the exe, and the function it was read from is named at the
//! class. Classes `engine_core::types` already has (`TESForm`, `BSStringT`, `BSSimpleList`) are
//! used from there.

use engine_core::layout;
use engine_core::ptr::Inline;
pub use engine_core::types::{BSSimpleList, BSStringT, TESForm};

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
        0x30 weatherTypes: Inline<BSSimpleList>,
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
