//! Layouts used by every subsystem: Gamebryo's and Bethesda's containers,
//! strings and reference-counted bases (Xbox PDB, sizes and offsets checked
//! against PC code). Lead-owned: translators use these and ask for
//! additions in their report instead of declaring their own copies.
//!
//! Templates are declared once; the element type is the translator's to
//! know (a `NiTArray<Actor *>`'s `m_pBase` holds `Actor` pointers).

use crate::layout;

layout! {
    /// `NiRefObject` (Xbox PDB): vtable at +0, reference count at +4.
    pub struct NiRefObject: 0x08 {
        /// `m_uiRefCount` (Xbox PDB).
        0x04 m_uiRefCount: u32,
    }

    /// `NiTArray<T, Interface>` (Xbox PDB), 0x10 bytes for every `T`:
    /// vtable, element pointer, then four 16-bit counts.
    pub struct NiTArray: 0x10 {
        /// `m_pBase` (Xbox PDB): the elements, 4 bytes each for pointer `T`.
        0x04 m_pBase: u32,
        /// `m_usMaxSize` (Xbox PDB): capacity.
        0x08 m_usMaxSize: u16,
        /// `m_usSize` (Xbox PDB): one past the highest used index.
        0x0A m_usSize: u16,
        /// `m_usESize` (Xbox PDB): number of non-null elements.
        0x0C m_usESize: u16,
        /// `m_usGrowBy` (Xbox PDB).
        0x0E m_usGrowBy: u16,
    }

    /// `BSSimpleArray<T, 1024>` (Xbox PDB), 0x10 bytes for every `T`.
    pub struct BSSimpleArray: 0x10 {
        /// `pBuffer` (Xbox PDB).
        0x04 pBuffer: u32,
        /// `iSize` (Xbox PDB).
        0x08 iSize: u32,
        /// `iReservedSize` (Xbox PDB).
        0x0C iReservedSize: u32,
    }

    /// `BSSimpleList<T>` (Xbox PDB): one node, item then next.
    pub struct BSSimpleList: 0x08 {
        /// `m_item` (Xbox PDB).
        0x00 m_item: u32,
        /// `m_pkNext` (Xbox PDB).
        0x04 m_pkNext: u32,
    }

    /// `BSStringT<char>` (Xbox PDB), the game's `BSString`.
    pub struct BSStringT: 0x08 {
        /// `pString` (Xbox PDB).
        0x00 pString: u32,
        /// `sLen` (Xbox PDB).
        0x04 sLen: u16,
        /// `sMaxLen` (Xbox PDB).
        0x06 sMaxLen: u16,
    }

    /// `NiFixedString` (Xbox PDB): a handle to a pooled string.
    pub struct NiFixedString: 0x04 {
        /// `m_kHandle` (Xbox PDB).
        0x00 m_kHandle: u32,
    }
}
