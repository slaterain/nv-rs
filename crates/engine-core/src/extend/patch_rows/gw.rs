//! Group GW's seams and wraps (step 3.9: world, sky, sound, saves, start-up). Only group GW's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GW". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G113 (a fix), TESRegionDataSound::LoadRegionDataSound 004f4860: the RDSI media set id resolved through the masters (LoadMediaSetIDHook).
    Entry {
        addr: 0x004f_49ab,
        owners: JIP,
        step: "3.9-GW",
    },
    // G113 (a fix), TESRegionDataSound::LoadRegionDataSound 004f4860: an RDSB media set id resolved through the masters (LoadMediaSetIDHook).
    Entry {
        addr: 0x004f_49d2,
        owners: JIP,
        step: "3.9-GW",
    },
    // G74 (a fix), fn_0063c490: the current weather's wind speed (GetWindSpeedHook).
    Entry {
        addr: 0x0063_c4d6,
        owners: JIP,
        step: "3.9-GW",
    },
    // G74 (a fix), fn_0063c490: the last weather's wind speed (GetWindSpeedHook).
    Entry {
        addr: 0x0063_c52c,
        owners: JIP,
        step: "3.9-GW",
    },
    // H17 (JIP's own, InitJIPHooks), Sky::SetCurrentClimate 0063c8f0: the forced climate (SkyRefreshClimateHook).
    Entry {
        addr: 0x0063_c918,
        owners: JIP,
        step: "3.9-GW",
    },
    // G15 (a fix), fn_0063e2f0: the sun's glare from the climate's glare texture.
    Entry {
        addr: 0x0063_e705,
        owners: JIP,
        step: "3.9-GW",
    },
    // G19 (a fix), Sky::UpdateHDRValues 0063ef20: the morning's image-space forms swapped (high-noon fix).
    Entry {
        addr: 0x0063_f385,
        owners: JIP,
        step: "3.9-GW",
    },
    // G19 (a fix), Sky::UpdateHDRValues 0063ef20: the afternoon's image-space forms swapped (high-noon fix).
    Entry {
        addr: 0x0063_f3f7,
        owners: JIP,
        step: "3.9-GW",
    },
    // G108 (a fix), FalloutAudio::ApplySettingsFromMasterFile 0082d400: the sound form posted to the audio thread (FillGameSoundPropsHook).
    Entry {
        addr: 0x0082_d416,
        owners: JIP,
        step: "3.9-GW",
    },
    // G104 (a fix), FalloutAudio::UpdateRegionSounds 0082d7c0: the pending acoustic space applied once (SetAcousticSpaceFixHook).
    Entry {
        addr: 0x0082_d8ca,
        owners: JIP,
        step: "3.9-GW",
    },
    // G94 (a fix), fn_0083ac30: a silenced weapon's attack sound for any actor (PlayAttackSoundHook).
    Entry {
        addr: 0x0083_ad97,
        owners: JIP,
        step: "3.9-GW",
    },
    // G115 (a fix), main 0086a850: FalloutCustom.ini applied before the launcher test (ProcessCustomINI).
    Entry {
        addr: 0x0086_ac60,
        owners: JIP,
        step: "3.9-GW",
    },
    // G89 (a fix), the IntSeenData constructor 0087a100: the word +0x26 cleared.
    Entry {
        addr: 0x0087_a12a,
        owners: JIP,
        step: "3.9-GW",
    },
    // G108 (a fix), BSAudioManager 00adf080, message kind 54 (jump table word 0xAE5648, group GA's seam): the game sound's form and frequency factor (SetSourceSoundRequestHook).
    Entry {
        addr: 0x00ae_5648,
        owners: JIP,
        step: "3.9-GW",
    },
    // G108 (a fix), the BSWin32GameSound constructor 00aeac20: the form field and the factor 1.0 (SAFE_WRITE_BUF 0xAEACAA).
    Entry {
        addr: 0x00ae_acaa,
        owners: JIP,
        step: "3.9-GW",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // G113 (a fix): TESRegionDataSound::GetMediaSetBattle 004f4f90 (the vtable word 0x1023DD8), never the last battle music again (PickBattleMediaSetHook).
    Entry {
        addr: 0x004f_4f90,
        owners: JIP,
        step: "3.9-GW",
    },
    // G113 (a fix): MediaLocationController's pick 00595560, a shuffled bag per list (PickMediaSetHook).
    Entry {
        addr: 0x0059_5560,
        owners: JIP,
        step: "3.9-GW",
    },
    // D05 (a fix): the folder sound picker 005e3630 (BSWin32Audio::PickSoundFileFromFolder), .wav and .ogg together (PickSoundFileFromFolderHook).
    Entry {
        addr: 0x005e_3630,
        owners: JIP,
        step: "3.9-GW",
    },
    // O5 (bGameDaysPassedFix): GameTimeGlobals::Update 00867a40 in doubles (UpdateTimeGlobalsHook).
    Entry {
        addr: 0x0086_7a40,
        owners: JIP,
        step: "3.9-GW",
    },
    // G108 (a fix): BSWin32GameSound::SetFrequency 00af01e0 (the vtable word 0x10A3C58), with the sound's factor (AdjustSoundFrequencyHook).
    Entry {
        addr: 0x00af_01e0,
        owners: JIP,
        step: "3.9-GW",
    },
    // G108 (a fix): BSWin32GameSound::GetFrequency 00af0290 (the vtable word 0x10A3C5C), over the sound's factor (GetSoundFrequencyPercHook).
    Entry {
        addr: 0x00af_0290,
        owners: JIP,
        step: "3.9-GW",
    },
];
