//! The classes of the geometry family, once each, at their PC offsets. Names marked (Xbox PDB) are the
//! prototype's (the PC's offsets of `NiGeometryData` match all 19 of its fields); the others say what
//! the code does with the field. A class read only as far as these functions read it says so; its size
//! is then where the last field read ends.

use crate::image;

image! {
    /// `NiGeometryData` (Xbox PDB), the object that owns a geometry's arrays.
    pub struct GeometryData / GeometryDataImage: 0x40 {
        0x00 vtable: u32,
        /// `m_uiRefCount` (Xbox PDB, of `NiRefObject`).
        0x04 m_uiRefCount: u32,
        /// `m_usVertices` (Xbox PDB): the vertex count every array is sized by.
        0x08 m_usVertices: u16,
        0x0a unknown_0a: u16,
        /// `m_usDataFlags` (Xbox PDB): bit 0 the texture set count, the bits 0xf000 the binormals and
        /// tangents stored after the normals (the normal array is then three normals long).
        0x0c m_usDataFlags: u16,
        /// `m_usDirtyFlags` (Xbox PDB): the low 12 bits the parts to pack again, the bits 0x7000 the
        /// consistency (0x4000 static).
        0x0e m_usDirtyFlags: u16,
        /// `m_kBound` (Xbox PDB): the centre and the radius.
        0x10 m_kBound: [u32; 4],
        /// `m_pkVertex` (Xbox PDB): 12 bytes a vertex.
        0x20 m_pkVertex: u32,
        /// `m_pkNormal` (Xbox PDB): 12 bytes a vertex, then the binormals and the tangents when the
        /// data flags say so.
        0x24 m_pkNormal: u32,
        /// `m_pkColor` (Xbox PDB): four floats a vertex.
        0x28 m_pkColor: u32,
        /// `m_pkTexture` (Xbox PDB): two floats a vertex for each texture set.
        0x2c m_pkTexture: u32,
        /// `m_spAdditionalGeomData` (Xbox PDB).
        0x30 m_spAdditionalGeomData: u32,
        /// `m_pkBuffData` (Xbox PDB): the renderer's buffer data.
        0x34 m_pkBuffData: u32,
        /// `m_ucKeepFlags` (Xbox PDB): which arrays outlive the renderer's packing (1 vertices,
        /// 2 normals, 4 colours, 8 texture coordinates, 0x10 the triangles).
        0x38 m_ucKeepFlags: u8,
        /// `m_ucCompressFlags` (Xbox PDB).
        0x39 m_ucCompressFlags: u8,
        /// `m_bVBLocked` and `m_bVBLockWrite` (Xbox PDB).
        0x3a m_bVBLocked: u8,
        0x3b m_bVBLockWrite: u8,
        /// `m_bSaveVertexData` (Xbox PDB).
        0x3c m_bSaveVertexData: u8,
        0x3d padding_3d: [u8; 3],
    }

    /// The packer entry a converter takes (`NiPackerEntry`, the Xbox PDB's name of the type the
    /// element pack builds on its stack), 0x30 bytes.
    pub struct PackerEntry / PackerEntryImage: 0x30 {
        /// The destination type (0 to 3: one to four floats; 4 a colour word; 5 four bytes; 6 and 7
        /// two and four 16-bit integers).
        0x00 uiType: u32,
        /// The source format (1 to 4: one to four floats; the higher ones integers of other widths).
        0x04 uiFormat: u32,
        /// The vertex count, read again after each vertex.
        0x08 usCount: u16,
        /// The upper half of the count's word: the element pack leaves whatever its stack held.
        0x0a usCountHigh: u16,
        /// The 16-bit index array a skin partition maps its vertices through, or 0.
        0x0c pusIndex: u32,
        /// The source array, or 0.
        0x10 pSource: u32,
        /// The bytes of a source element the plain copies take.
        0x14 uiElementSize: u32,
        /// The bytes from one source element to the next.
        0x18 uiSourceStride: u32,
        /// The bytes the destination type writes a vertex; the answer adds it once a vertex.
        0x1c uiWriteSize: u32,
        /// The bytes from one destination vertex to the next.
        0x20 uiDestinationStride: u32,
        /// The element pack's last word, which these converters do not read.
        0x24 uiLast: u32,
        /// The element pack's byte, which these converters do not read.
        0x28 ucFlag: u8,
        0x29 padding_29: [u8; 3],
        /// Where the first vertex's element is written.
        0x2c pDestination: u32,
    }

    /// An entry of a shader declaration's stream (0x1c bytes, as `00e76320` constructs it), as far as
    /// read.
    pub struct DeclarationEntry / DeclarationEntryImage: 0x10 {
        0x00 unknown_00: u32,
        /// The entry's offset in a vertex.
        0x04 uiOffset: u32,
        /// The usage: -1 none, 0 positions, 1 and 2 skin weights and bone indices, 3 normals, 4
        /// colours, 5 to 12 texture sets, 13 and 14 binormals and tangents, negative ones the
        /// additional data's streams.
        0x08 uiUsage: u32,
        /// The type (0x11: unused).
        0x0c uiType: u32,
    }

    /// A stream of a shader declaration (16 bytes), as far as read.
    pub struct StreamEntry / StreamEntryImage: 0x10 {
        0x00 unknown_00: u32,
        0x04 unknown_04: u32,
        /// The stream's entries (`DeclarationEntry`, 0x1c bytes each).
        0x08 pEntries: u32,
        /// The stream's stride.
        0x0c uiStride: u32,
    }

    /// `NiD3DShaderDeclaration` (Xbox PDB), as far as read.
    pub struct ShaderDeclaration / ShaderDeclarationImage: 0x28 {
        0x00 vtable: u32,
        0x04 m_uiRefCount: u32,
        /// `m_pkRenderer` (Xbox PDB).
        0x08 m_pkRenderer: u32,
        /// The renderer's vertex buffer manager.
        0x0c m_pkVBManager: u32,
        /// `m_pkD3DDevice` (Xbox PDB).
        0x10 m_pkD3DDevice: u32,
        0x14 unknown_14: [u32; 2],
        /// The entry count of every stream.
        0x1c m_uiMaxStreamEntryCount: u32,
        /// `m_uiStreamCount` (Xbox PDB).
        0x20 m_uiStreamCount: u32,
        /// `m_pkStreamEntries` (Xbox PDB): the streams, 16 bytes each.
        0x24 m_pkStreamEntries: u32,
    }

    /// `NiGeometryBufferData` (Xbox PDB), as far as read.
    pub struct GeometryBufferData / GeometryBufferDataImage: 0x2c {
        0x00 unknown_00: [u32; 5],
        /// `m_bSoftwareVP` (Xbox PDB).
        0x14 m_bSoftwareVP: u8,
        0x15 padding_15: [u8; 11],
        /// `m_uiStreamCount` (Xbox PDB).
        0x20 m_uiStreamCount: u32,
        /// `m_puiVertexStride` (Xbox PDB): a stride a stream.
        0x24 m_puiVertexStride: u32,
        /// `m_ppkVBChip` (Xbox PDB): a chip a stream.
        0x28 m_ppkVBChip: u32,
    }

    /// A vertex buffer chip (`NiVBChip`, the Xbox PDB's name), as far as read: what the manager's lock
    /// (`00e8bff0`: the buffer, the offset, the size, the flags) takes.
    pub struct VBChip / VBChipImage: 0x18 {
        0x00 unknown_00: [u32; 2],
        /// The Direct3D vertex buffer.
        0x08 pVB: u32,
        /// The chip's offset in the buffer.
        0x0c uiOffset: u32,
        /// The lock flags.
        0x10 uiLockFlags: u32,
        /// The chip's size.
        0x14 uiSize: u32,
    }

    /// `NiSkinPartition::Partition` (Xbox PDB), as far as read.
    pub struct SkinPartition / SkinPartitionImage: 0x2c {
        0x00 vtable: u32,
        /// `m_pusBones` (Xbox PDB).
        0x04 m_pusBones: u32,
        /// `m_pfWeights` (Xbox PDB).
        0x08 m_pfWeights: u32,
        /// `m_pusVertexMap` (Xbox PDB): the 16-bit index of each partition vertex in the data.
        0x0c m_pusVertexMap: u32,
        /// `m_pucBonePalette` (Xbox PDB).
        0x10 m_pucBonePalette: u32,
        0x14 unknown_14: [u32; 2],
        /// The partition's vertex count.
        0x1c m_usVertices: u16,
        0x1e unknown_1e: [u16; 3],
        /// The bones a vertex has (1 to 4).
        0x24 m_usBonesPerVertex: u16,
        0x26 padding_26: u16,
        /// The partition's buffer data.
        0x28 m_pkBuffData: u32,
    }

    /// The renderer's vertex buffer manager, as far as read: what its lock (`00e8bff0`) keeps until the
    /// unlock: the staging block the packers write, the address the buffer's lock answered and the size.
    pub struct VertexBufferManager / VertexBufferManagerImage: 0x98 {
        0x00 unknown_00: [u32; 16],
        /// The staging block and its capacity.
        0x40 pStaging: u32,
        0x44 uiStagingCapacity: u32,
        /// The address the vertex buffer's lock answered, and the bytes locked.
        0x48 pLocked: u32,
        0x4c uiLockedSize: u32,
        0x50 unknown_50: [u32; 12],
        /// The manager's lock (a `CRITICAL_SECTION`, 0x18 bytes), held from the lock to the unlock.
        0x80 kLock: [u32; 6],
    }
}
