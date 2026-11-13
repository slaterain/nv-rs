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
        /// `pParentCell` (Xbox PDB; JIP's `parentCell`, `// 40`): the exe's
        /// `TESObjectREFR::GetParentCell`, `008d6f30`, returns `[ecx+0x40]` (engine-fallout-cell
        /// `tesobjectrefr.rs`).
        0x40 pParentCell: u32,
        /// `ExtraData` (Xbox PDB): the reference's extra data list.
        0x44 ExtraData: Inline<BaseExtraList>,
        /// `jipRefFlags5F` (JIP LN NVSE): JIP's flags (`kHookRefFlag5F_*`, `internal/hooks.h`) in
        /// byte `+0x1B` of the extra data list above: presence bits past the last extra data type,
        /// which the game never sets.
        0x5F jipRefFlags5F: u8,
        /// `pLoadedData` (Xbox PDB): `LOADED_REF_DATA*`, 0 while the reference has no 3D.
        0x64 pLoadedData: u32,
    }

    /// `Actor` (Xbox PDB), the fields JIP reads; 0x1B4 bytes on PC (engine-fallout-ai `Actor`).
    pub struct Actor: 0x1B4 {
        /// `pCurrentProcess` (Xbox PDB): `BaseProcess*`.
        0x68 pCurrentProcess: u32,
        /// `bPlayerTeammate` (Xbox PDB).
        0x18D bPlayerTeammate: u8,
    }

    /// `LOADED_REF_DATA` (Xbox PDB), 0x1C bytes, the field JIP reads (the exe reads the 3D at
    /// `+0x14` of `pLoadedData`: engine-fallout-cell `tesobjectrefr_p4.rs`).
    pub struct LoadedRefData: 0x1C {
        /// The reference's 3D root.
        0x14 p3D: u32,
    }

    /// `NiNode` (Xbox PDB): its children (`m_kChildren`, an `NiTArray` at +0x9C).
    pub struct NiNode: 0xAC {
        /// `m_kChildren.m_pBase` (Xbox PDB).
        0xA0 m_pChildren: u32,
        /// `m_kChildren.m_usSize` (Xbox PDB): the slots in use (some may be empty).
        0xA6 m_usChildCount: u16,
    }

    /// `InterfaceManager` (Xbox PDB), the fields JIP reads, as the exe uses them
    /// (engine-fallout-interface `interfacemanager.rs`: `cMenuMode`, `iEnterStack`, and the
    /// Pip-Boy state machine of `InterfaceManager::UpdatePipboy`, `0070ee80`).
    pub struct InterfaceManager: 0x580 {
        /// `iCurrentMode` (JIP's `currentMode`): 1 in game mode, 2 in menu mode.
        0x00C currentMode: u32,
        /// `pActivateRef` (Xbox PDB; JIP's `crosshairRef`, `// 0FC`): the reference under the
        /// crosshair; `InterfaceManager::InterfaceManager`, `0070a130`, zeroes `[edx+0xfc]`
        /// (engine-fallout-interface `interfacemanager.rs`).
        0x0FC pActivateRef: u32,
        /// `menuStack[10]` (JIP LN NVSE): the open menus' ids, 0 after the last.
        0x114 menuStack: u32,
        /// `pipBoyMode` (JIP LN NVSE): 3 while the Pip-Boy is up.
        0x4BC pipBoyMode: u32,
    }

    /// `PlayerCharacter` (Xbox PDB), 0xE50 bytes (`PlayerCharacter::PlayerCharacter`, `00938180`,
    /// engine-fallout-ai `playercharacter.rs`), the field JIP reads.
    pub struct PlayerCharacter: 0xE50 {
        /// `pGrabbedObject` (Xbox PDB; JIP's `grabbedRef`, `// 638`): the reference the player
        /// holds; the constructor `00938180` clears it.
        0x638 pGrabbedObject: u32,
    }

    /// `OSInputGlobals` (JIP LN NVSE; the exe's `Controls`, `*0x011F35CC`), 0x1C04 bytes: the
    /// button bound to each of the 28 controls, one byte each, 0xFF for none, per device. The
    /// exe's action clear `00a25410` reads `[this + 0x1b94 + device * 0x1c + action]` for devices
    /// 0 to 3 (engine-bs `xcontrols.rs`). Each field is a binding array's first byte.
    pub struct OSInputGlobals: 0x1C04 {
        /// `keyBinds[28]` (JIP LN NVSE, `// 1B94`): device 0, the keyboard.
        0x1B94 keyBinds: u8,
        /// `mouseBinds[28]` (`// 1BB0`): device 1.
        0x1BB0 mouseBinds: u8,
        /// `joystickBinds[28]` (`// 1BCC`): device 2.
        0x1BCC joystickBinds: u8,
        /// `controllerBinds[28]` (`// 1BE8`): device 3, the XInput pad.
        0x1BE8 controllerBinds: u8,
    }

    /// `BGSListForm` (Xbox PDB): `ListOfForms`, a `BSSimpleList<TESForm *>` with its first node
    /// inline; 0x24 bytes on PC (`CreateFormOfType`; engine-fallout-forms `bgslistform.rs`).
    pub struct BGSListForm: 0x24 {
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

    /// `ContainerObject` (Xbox PDB; JIP's `FormCount`): a count and a form (and the item's extra
    /// data at `+8`); engine-fallout-forms `tescontainer.rs`.
    pub struct ContainerObject: 0x0C {
        0x00 count: i32,
        0x04 form: u32,
    }

    /// `ExtraContainerChanges` (Xbox PDB): `pChanges`, an `InventoryChanges*`.
    pub struct ExtraContainerChanges: 0x10 {
        0x0C data: u32,
    }

    /// `InventoryChanges` (Xbox PDB; JIP's `ExtraContainerChanges::Data`): `pListofChanges`, a
    /// `BSSimpleList<ItemChange *>*`: the list's first node itself (engine-fallout-forms
    /// `inventorychanges.rs`).
    pub struct ContainerChangesData: 0x14 {
        0x00 objList: u32,
    }

    /// `ItemChange` (Xbox PDB; JIP's `ContChangesEntry`): its extra data lists, count change, form.
    pub struct ContChangesEntry: 0x0C {
        /// `pExtraObjectList` (Xbox PDB): a `BSSimpleList<ExtraDataList *>*`, the list's first
        /// node itself: `ItemChange::GetExtraTotalDefaultCount` (`004bc980`) walks from it.
        0x00 extendData: u32,
        0x04 countDelta: i32,
        0x08 form: u32,
    }

    /// `HitData` (Xbox PDB; JIP's `ActorHitData`), 0x64 bytes (engine-fallout-ai `hitdata.rs`).
    pub struct ActorHitData: 0x64 {
        /// `iRefCount` in the Xbox PDB; JIP calls it unused (`unk60`) and its `CopyHitDataHook`
        /// (step 3.5) writes the hit location there. Until that hook exists this is the game's
        /// own word, not a location.
        0x60 hitLocation: i32,
    }

    /// `Setting` (Xbox PDB): its value and name (the name's first letter is its type).
    pub struct Setting: 0x0C {
        0x04 data: u32,
        0x08 name: u32,
    }

    /// `GameSettingCollection` (Xbox PDB): its `SettingCollectionMap`'s map of settings by name
    /// at `+0x10C` (engine-fallout-magic `effectitem.rs`, checked at `0040528b`).
    pub struct GameSettingCollection: 0x120 {
        0x110 m_uiHashSize: u32,
        0x114 m_ppkHashTable: u32,
    }

    /// `IniSettingCollection` (Xbox PDB): its list of settings at `+0x10C`
    /// (`SettingCollectionList<Setting>::Add`, `0044f770`).
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
        /// `data.isTrait` (JIP LN NVSE): the perk loader `005EADA0` reads `DATA`'s
        /// five bytes from +0x38 (trait, minimum level, ranks, playable, hidden).
        0x38 isTrait: u8,
        /// `data.cNumRanks` (Xbox PDB).
        0x3A cNumRanks: u8,
        /// `data.isPlayable` (JIP LN NVSE).
        0x3B isPlayable: u8,
        /// `data.isHidden` (JIP LN NVSE).
        0x3C isHidden: u8,
    }
}

/// `saveMods[index]`.
pub const fn save_mod(index: u8) -> u32 {
    BGSSaveLoadGame::saveMods.off + index as u32
}
