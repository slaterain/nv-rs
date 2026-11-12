//! The game's classes JIP reads, with the fields JIP's translations have needed so far, at their
//! PC offsets (the names as the Xbox PDB has them; the engine crates declare the same classes for
//! their own code). Every read of a game object's field goes through these, so the 64-bit move
//! changes the layouts here and not the code that reads them.

use engine_core::layout;
use engine_core::ptr::Inline;
pub use engine_core::types::{BaseExtraList, NiAVObject, NiTPointerMap, TESForm};

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
        /// `jipRefFlags5F` (JIP LN NVSE): JIP's flags in a byte the game leaves unused
        /// (`kHookRefFlag5F_*`, `internal/hooks.h`).
        0x5F jipRefFlags5F: u8,
        /// `pLoadedData` (Xbox PDB): `LOADED_REF_DATA*`, 0 while the reference has no 3D.
        0x64 pLoadedData: u32,
    }

    /// `Actor` (Xbox PDB), the fields JIP reads.
    pub struct Actor: 0x1C0 {
        /// `pCurrentProcess` (Xbox PDB): `BaseProcess*`.
        0x68 pCurrentProcess: u32,
        /// `bIsTeammate` (Xbox PDB).
        0x18D bIsTeammate: u8,
    }

    /// `LOADED_REF_DATA` (Xbox PDB), the field JIP reads.
    pub struct LoadedRefData: 0x44 {
        /// `p3D` (Xbox PDB): the reference's root node.
        0x14 p3D: u32,
    }

    /// `NiNode` (Xbox PDB): its children (`m_kChildren`, an `NiTArray` at +0x9C).
    pub struct NiNode: 0xAC {
        /// `m_kChildren.m_pBase` (Xbox PDB).
        0xA0 m_pChildren: u32,
        /// `m_kChildren.m_usSize` (Xbox PDB): the slots in use (some may be empty).
        0xA6 m_usChildCount: u16,
    }

    /// `InterfaceManager` (Xbox PDB), the fields JIP reads.
    pub struct InterfaceManager: 0x580 {
        /// `iCurrentMode` (JIP's `currentMode`): 1 in game mode, 2 in menu mode.
        0x00C currentMode: u32,
        /// `menuStack[10]` (JIP LN NVSE): the open menus' ids, 0 after the last.
        0x114 menuStack: u32,
        /// `pipBoyMode` (JIP LN NVSE): 3 while the Pip-Boy is up.
        0x4BC pipBoyMode: u32,
    }

    /// `BGSListForm` (Xbox PDB): its list of forms (`tList<TESForm>`, the first node inline).
    pub struct BGSListForm: 0x2C {
        0x18 list: Inline<ListNode>,
    }

    /// A node of the game's `BSSimpleList`/`tList`: the item, the next node.
    pub struct ListNode: 0x08 {
        0x00 data: u32,
        0x04 next: u32,
    }

    /// `TESContainer` (Xbox PDB): its list of `ContainerObject`s (count, form).
    pub struct TESContainer: 0x0C {
        0x04 list: Inline<ListNode>,
    }

    /// `TESContainer::ContainerObject` (JIP's `FormCount`): a count and a form.
    pub struct ContainerObject: 0x08 {
        0x00 count: i32,
        0x04 form: u32,
    }

    /// `ExtraContainerChanges` (Xbox PDB): its data (`ExtraContainerChanges::Data*`).
    pub struct ExtraContainerChanges: 0x10 {
        0x0C data: u32,
    }

    /// `ExtraContainerChanges::Data` (Xbox PDB): the changed entries' list (`tList*`).
    pub struct ContainerChangesData: 0x14 {
        0x00 objList: u32,
    }

    /// `InventoryChanges` (JIP's `ContChangesEntry`): its extra data lists, count change, form.
    pub struct ContChangesEntry: 0x0C {
        /// `extendData` (JIP LN NVSE): `tList<ExtraDataList>*`.
        0x00 extendData: u32,
        0x04 countDelta: i32,
        0x08 form: u32,
    }

    /// `ActorHitData` (JIP LN NVSE), the field JIP's `GetHitLocation` reads.
    pub struct ActorHitData: 0x64 {
        /// `unk60` (JIP LN NVSE): unused by the game; JIP's `CopyHitDataHook` stores the hit
        /// location there.
        0x60 hitLocation: i32,
    }

    /// `Setting` (Xbox PDB): its value and name (the name's first letter is its type).
    pub struct Setting: 0x0C {
        0x04 data: u32,
        0x08 name: u32,
    }

    /// `GameSettingCollection` (Xbox PDB): its map of settings by name (an `NiTMap` at +0x10C).
    pub struct GameSettingCollection: 0x120 {
        0x110 m_uiHashSize: u32,
        0x114 m_ppkHashTable: u32,
    }

    /// `IniSettingCollection` (Xbox PDB): its list of settings (`tList<Setting>` at +0x10C).
    pub struct IniSettingCollection: 0x114 {
        0x10C settings: Inline<ListNode>,
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
