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

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x00a8ff70, ni_screen_geometry_get_rtti() -> Ptr),
        entry!(0x00a8ffb0, ni_screen_polygon_get_rtti() -> Ptr),
        entry!(0x00a90ab0, ni_screen_geometry_data_get_rtti() -> Ptr),
        entry!(0x00a91690, ni_screen_space_camera_get_rtti() -> Ptr),
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
}
