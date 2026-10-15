//! `NiMain/nioldscreenspace.cpp` (Xbox PDB source unit), subsystem `NiMain`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).

#[allow(unused_imports)]
use crate::prelude::*;

/// `NiScreenGeometry::ms_RTTI` (the class's `NiRTTI` record).
const NI_SCREEN_GEOMETRY_RTTI: u32 = 0x011f_5e5c;

/// `NiScreenPolygon::ms_RTTI` (the class's `NiRTTI` record).
const NI_SCREEN_POLYGON_RTTI: u32 = 0x011f_5e64;

/// `NiScreenGeometryData::ms_RTTI` (the class's `NiRTTI` record).
const NI_SCREEN_GEOMETRY_DATA_RTTI: u32 = 0x011f_5e54;

/// `NiScreenSpaceCamera::ms_RTTI` (the class's `NiRTTI` record).
const NI_SCREEN_SPACE_CAMERA_RTTI: u32 = 0x011f_5e6c;

// Translated from 00a8ff70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenGeometry::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_screen_geometry_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_SCREEN_GEOMETRY_RTTI)
}

// Translated from 00a8ffb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenPolygon::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_screen_polygon_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_SCREEN_POLYGON_RTTI)
}

// Translated from 00a90ab0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenGeometryData::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_screen_geometry_data_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_SCREEN_GEOMETRY_DATA_RTTI)
}

// Translated from 00a91690 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenSpaceCamera::GetRTTI` (Xbox PDB): the address of the class's `NiRTTI` record.
pub fn ni_screen_space_camera_get_rtti(_e: &mut Engine) -> Ptr {
    Ptr::new(NI_SCREEN_SPACE_CAMERA_RTTI)
}

/// `00a98ea0`: the jump target of `00a877d0`.
const JUMP_TARGET_00A877D0: u32 = 0x00a9_8ea0;

/// `00a71d10`: the jump target of `00a877e0`.
const JUMP_TARGET_00A877E0: u32 = 0x00a7_1d10;

/// `00a71450`: the jump target of `00a90000`.
const JUMP_TARGET_00A90000: u32 = 0x00a7_1450;

/// `00a716c0`: the jump target of `00a90020`.
const JUMP_TARGET_00A90020: u32 = 0x00a7_16c0;

/// `00a6ff20`: the jump target of `00a90030`.
const JUMP_TARGET_00A90030: u32 = 0x00a6_ff20;

// Translated from 00a877d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenGeometry::LoadBinary` (Xbox PDB): a 5-byte jump to `00a98ea0` (engine map: `NiParticleMeshes::LoadBinary`), passing on `this` and its one argument. Return type (void): Xbox PDB public name `?LoadBinary@NiScreenGeometry@@UAAXAAVNiStream@@@Z`.
pub fn ni_screen_geometry_load_binary(e: &mut Engine, this: Ptr, stream: Ptr) {
    e.call(JUMP_TARGET_00A877D0, &args![this, stream]);
}

// Translated from 00a877e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenGeometry::SaveBinary` (Xbox PDB): a 5-byte jump to `00a71d10` (engine map: `NiParticleMeshes::SaveBinary`), passing on `this` and its one argument. Return type (void): Xbox PDB public name `?SaveBinary@NiScreenGeometry@@UAAXAAVNiStream@@@Z`.
pub fn ni_screen_geometry_save_binary(e: &mut Engine, this: Ptr, stream: Ptr) {
    e.call(JUMP_TARGET_00A877E0, &args![this, stream]);
}

// Translated from 00a90000 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenSpaceCamera::LoadBinary` (Xbox PDB): a 5-byte jump to `00a71450` (engine map: `NiCamera::LoadBinary`), passing on `this` and its one argument. Return type (void): Xbox PDB public name `?LoadBinary@NiScreenSpaceCamera@@UAAXAAVNiStream@@@Z`.
pub fn ni_screen_space_camera_load_binary(e: &mut Engine, this: Ptr, stream: Ptr) {
    e.call(JUMP_TARGET_00A90000, &args![this, stream]);
}

// Translated from 00a90020 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenSpaceCamera::SaveBinary` (Xbox PDB): a 5-byte jump to `00a716c0` (engine map: `NiCamera::SaveBinary`), passing on `this` and its one argument. Return type (void): Xbox PDB public name `?SaveBinary@NiScreenSpaceCamera@@UAAXAAVNiStream@@@Z`.
pub fn ni_screen_space_camera_save_binary(e: &mut Engine, this: Ptr, stream: Ptr) {
    e.call(JUMP_TARGET_00A90020, &args![this, stream]);
}

// Translated from 00a90030 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiScreenSpaceCamera::IsEqual` (Xbox PDB): a 5-byte jump to `00a6ff20` (engine map: `NiCamera::IsEqual`), passing on `this` and its one argument; returns the target result. Return type (bool): Xbox PDB public name `?IsEqual@NiScreenSpaceCamera@@UAA_NPAVNiObject@@@Z`.
pub fn ni_screen_space_camera_is_equal(e: &mut Engine, this: Ptr, other: Ptr) -> bool {
    e.call(JUMP_TARGET_00A90030, &args![this, other]).bool()
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00a8ff70, ni_screen_geometry_get_rtti() -> Ptr),
        entry!(0x00a8ffb0, ni_screen_polygon_get_rtti() -> Ptr),
        entry!(0x00a90ab0, ni_screen_geometry_data_get_rtti() -> Ptr),
        entry!(0x00a91690, ni_screen_space_camera_get_rtti() -> Ptr),
        entry!(0x00a8_77d0, ni_screen_geometry_load_binary(Ptr, Ptr)),
        entry!(0x00a8_77e0, ni_screen_geometry_save_binary(Ptr, Ptr)),
        entry!(0x00a9_0000, ni_screen_space_camera_load_binary(Ptr, Ptr)),
        entry!(0x00a9_0020, ni_screen_space_camera_save_binary(Ptr, Ptr)),
        entry!(0x00a9_0030, ni_screen_space_camera_is_equal(Ptr, Ptr) -> bool),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ni_screen_geometry_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a8ff70, &args![]).u32(), NI_SCREEN_GEOMETRY_RTTI);
    }

    #[test]
    fn ni_screen_polygon_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(e.call(0x00a8ffb0, &args![]).u32(), NI_SCREEN_POLYGON_RTTI);
    }

    #[test]
    fn ni_screen_geometry_data_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a90ab0, &args![]).u32(),
            NI_SCREEN_GEOMETRY_DATA_RTTI
        );
    }

    #[test]
    fn ni_screen_space_camera_get_rtti_returns_the_class_rtti() {
        let mut e = Engine::new();
        assert_eq!(
            e.call(0x00a91690, &args![]).u32(),
            NI_SCREEN_SPACE_CAMERA_RTTI
        );
    }

    #[test]
    fn thunk_00a877d0_forwards_its_words_to_the_jump_target() {
        const THUNK: u32 = 0x00a8_77d0;
        const TARGET: u32 = 0x00a9_8ea0;
        let mut e = Engine::new();
        e.register(TARGET, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(THUNK, &args![0x1000u32, 0x2000u32]);
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log.iter().filter(|call| call.0 == TARGET).collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, args![0x1000u32, 0x2000u32]);
    }

    #[test]
    fn thunk_00a877e0_forwards_its_words_to_the_jump_target() {
        const THUNK: u32 = 0x00a8_77e0;
        const TARGET: u32 = 0x00a7_1d10;
        let mut e = Engine::new();
        e.register(TARGET, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(THUNK, &args![0x1000u32, 0x2000u32]);
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log.iter().filter(|call| call.0 == TARGET).collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, args![0x1000u32, 0x2000u32]);
    }

    #[test]
    fn thunk_00a90000_forwards_its_words_to_the_jump_target() {
        const THUNK: u32 = 0x00a9_0000;
        const TARGET: u32 = 0x00a7_1450;
        let mut e = Engine::new();
        e.register(TARGET, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(THUNK, &args![0x1000u32, 0x2000u32]);
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log.iter().filter(|call| call.0 == TARGET).collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, args![0x1000u32, 0x2000u32]);
    }

    #[test]
    fn thunk_00a90020_forwards_its_words_to_the_jump_target() {
        const THUNK: u32 = 0x00a9_0020;
        const TARGET: u32 = 0x00a7_16c0;
        let mut e = Engine::new();
        e.register(TARGET, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        e.call(THUNK, &args![0x1000u32, 0x2000u32]);
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log.iter().filter(|call| call.0 == TARGET).collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, args![0x1000u32, 0x2000u32]);
    }

    #[test]
    fn thunk_00a90030_forwards_its_words_and_returns_the_target_result() {
        const THUNK: u32 = 0x00a9_0030;
        const TARGET: u32 = 0x00a6_ff20;
        let mut e = Engine::new();
        e.register(TARGET, |_, _| Ret {
            eax: 1,
            ..Ret::default()
        });
        e.call_log = Some(vec![]);
        assert_eq!(e.call(THUNK, &args![0x1000u32, 0x2000u32]).u32(), 1);
        let log = e.call_log.take().unwrap();
        let hits: Vec<_> = log.iter().filter(|call| call.0 == TARGET).collect();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].1, args![0x1000u32, 0x2000u32]);
        e.register(TARGET, |_, _| Ret::default());
        assert_eq!(e.call(THUNK, &args![0x1000u32, 0x2000u32]).u32(), 0);
    }
}
