//! The game's classes JIP reads, with the fields JIP's translations have needed so far, at their
//! PC offsets (the names as the Xbox PDB has them; the engine crates declare the same classes for
//! their own code). Every read of a game object's field goes through these, so the 64-bit move
//! changes the layouts here and not the code that reads them.

use engine_core::layout;
use engine_core::ptr::Inline;
pub use engine_core::types::{BaseExtraList, NiTPointerMap, TESForm};

layout! {
    /// `BGSSaveLoadGame` (Xbox PDB), 0x24C bytes: JIP's `g_BGSSaveLoadGame`.
    pub struct BGSSaveLoadGame: 0x24C {
        /// `pChangesMap` (Xbox PDB): the save's changed forms, a map of ids.
        0x000 pChangesMap: u32,
        /// `saveMods` (JIP's name): 255 bytes, what each file of the save is loaded as today,
        /// 0xFF for not loaded. The first; [`save_mod`] reads the others.
        0x044 saveMods: u8,
    }

    /// An entry of one of the game's maps of ids (`NiTMapItem`): next, key, value.
    pub struct NiTMapItem: 0x0C {
        0x00 m_pkNext: u32,
        0x04 m_key: u32,
        0x08 m_val: u32,
    }

    /// `TESObjectREFR` (Xbox PDB), the fields JIP reads.
    pub struct TESObjectREFR: 0x68 {
        /// `data.pObjectReference` (Xbox PDB): the base form.
        0x20 pObjectReference: u32,
        /// `ExtraData` (Xbox PDB): the reference's extra data list.
        0x44 ExtraData: Inline<BaseExtraList>,
    }

    /// `ExtraScript` (Xbox PDB), 0x14 bytes.
    pub struct ExtraScript: 0x14 {
        /// `pScript` (Xbox PDB).
        0x0C pScript: u32,
        /// `pScriptVars` (Xbox PDB): the event list.
        0x10 pScriptVars: u32,
    }

    /// `ExtraLinkedRef` (Xbox PDB), 0x10 bytes.
    pub struct ExtraLinkedRef: 0x10 {
        /// `pLinkedRef` (Xbox PDB).
        0x0C pLinkedRef: u32,
    }

    /// `TESQuest` (Xbox PDB), 0x6C bytes.
    pub struct TESQuest: 0x6C {
        /// `TESScriptableForm::pFormScript` (Xbox PDB).
        0x1C pFormScript: u32,
        /// `pScriptLocals` (Xbox PDB): the event list.
        0x5C pScriptLocals: u32,
    }

    /// `BGSPerk` (Xbox PDB), the fields JIP reads.
    pub struct BGSPerk: 0x50 {
        /// `data.cNumRanks` (Xbox PDB).
        0x3A cNumRanks: u8,
    }
}

/// `saveMods[index]`.
pub const fn save_mod(index: u8) -> u32 {
    BGSSaveLoadGame::saveMods.off + index as u32
}
