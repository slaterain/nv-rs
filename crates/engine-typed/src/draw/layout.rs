//! The classes the draw family's geometry paths read, once each, at their PC offsets (the drivers and
//! the alpha pass read the shader family's, `crate::shader::layout`). Names marked (Xbox PDB) are the
//! prototype's; the others say what the code does with the field. A class read only as far as these
//! functions read it says so.

use crate::image;

image! {
    /// `BSSegmentedTriShape` (Xbox PDB), 0xd0 bytes on PC, as far as read: its segments.
    pub struct BSSegmentedTriShape / BSSegmentedTriShapeImage: 0xd0 {
        0x00 unknown_00: [u32; 49],
        /// `pSegments` (Xbox PDB): 0x14 bytes each.
        0xc4 pSegments: u32,
        /// `iNumSegments` (Xbox PDB).
        0xc8 iNumSegments: u32,
        /// `bSegmentsChanged` (Xbox PDB).
        0xcc bSegmentsChanged: u8,
        0xcd padding_cd: [u8; 3],
    }

    /// `BSSegmentedTriShape::Segment` (Xbox PDB), 0x14 bytes.
    pub struct Segment / SegmentImage: 0x14 {
        /// `iStartIndex` (Xbox PDB).
        0x00 iStartIndex: u32,
        /// `iNumPrimitives` (Xbox PDB).
        0x04 iNumPrimitives: u32,
        /// `bEnabled` (Xbox PDB): what the game asks for.
        0x08 bEnabled: u8,
        0x09 padding_09: [u8; 3],
        /// `iNumPrimitivesMerged` (Xbox PDB).
        0x0c iNumPrimitivesMerged: u32,
        /// `bEnabledPostMerge` (Xbox PDB): what the merge leaves for the draw.
        0x10 bEnabledPostMerge: u8,
        0x11 padding_11: [u8; 3],
    }
}
