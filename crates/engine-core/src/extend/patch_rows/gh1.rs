//! Group GH1's seams and wraps (step 3.9: extra data and inventory stacks). Only group GH1's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GH1". Rows of the patches this group
//! writes the handlers of (its home rows), seams in other groups' files included.

use super::super::Entry;
use super::JIP;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[
    // G44 (the ExtraJIP framework): `0040f980`, the type byte table's entry for 0x27 (0 for 1): an
    // ExtraJIP goes to the head of its list's chain.
    Entry {
        addr: 0x0040_f9d7,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G44: `ExtraDataList::AddExtraCopy` 004103d0, the jump table word of type 0x27: JIP's
    // ExtraJIPCreateCopyHook copies or moves the ExtraJIP's keyed data.
    Entry {
        addr: 0x0041_1978,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G76 (the hotkey fix): `ExtraDataList::CopyListForReference` 00412490, the byte table's entry
    // for 0x4A (2 for 1): a hotkey is copied (and moved) with its list.
    Entry {
        addr: 0x0041_2675,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G46 (the invisible stack fix): `ExtraDataList::CompareListForContainer` 004126c0, the `jnz`
    // after the ownership test becomes `jmp`: an owner extra is no difference at once.
    Entry {
        addr: 0x0041_271e,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G61: `0041c7f0`, its call of `TESObjectREFR::GetRefPersists`: JIP's IsPersistentHook
    // answers false for a temporary form.
    Entry {
        addr: 0x0041_c833,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G44: `0041d030`, the type byte table's entry for 0x27 (0 for 3): an ExtraJIP ends the
    // ownership-only scan with false.
    Entry {
        addr: 0x0041_d0f6,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G44: `ExtraDataList::IsExtraDefaultforContainer` 0041d120, the worn-allowed table's entry for
    // 0x27 (0 for 1): an ExtraJIP makes the list not the container's default.
    Entry {
        addr: 0x0041_d212,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G44: the same, the other table (worn not allowed).
    Entry {
        addr: 0x0041_d25a,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G44: `ExtraDataList::SaveGame_ov2` 00426a30, the jump table word of type 0x27: JIP's
    // ExtraJIPSaveGameHook saves the key.
    Entry {
        addr: 0x0042_7f30,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G44: `ExtraDataList::LoadGame_ov2` 00428150, the jump table word of type 0x27: JIP's
    // ExtraJIPLoadGameHook loads the key.
    Entry {
        addr: 0x0042_c328,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G79: `ItemChange::GetModSlots` 004bd820, its head (JIP's GetEntryDataModFlagsHook; a seam,
    // as Rust calls the translation directly).
    Entry {
        addr: 0x004b_d820,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G79: `ItemChange::HasModEffectActive_ov2` 004bda70, its head (GetEntryDataHasModHook).
    Entry {
        addr: 0x004b_da70,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G46: `OEI_Fast_InventoryIterator`'s scalar deleting destructor 004bf630, its head (`retn 4`).
    Entry {
        addr: 0x004b_f630,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G44: `004c1c90` (a reference added to the changes), its call of `RemoveExtraCount`: JIP's
    // SetPickedCountHook takes the count from the reference's count extra first.
    Entry {
        addr: 0x004c_21eb,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G44: the same function, its call of `CopyListForContainer`: JIP's CopyExtraListHook gives
    // the new list a count extra first when the count is above 1.
    Entry {
        addr: 0x004c_2291,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G61: the same function, its first call of `GetRefPersists` (IsPersistentHook).
    Entry {
        addr: 0x004c_232c,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G61: the same function, its second call.
    Entry {
        addr: 0x004c_273a,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G75 (the hotkey fix): `004c37d0` (an item removed), its hotkey block: JIP's
    // HotkeyFixRemoveItemHook.
    Entry {
        addr: 0x004c_3976,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G46: `InventoryChanges::StartOEIFastInventoryIteration` 004ca200, its head
    // (CreateInventoryIteratorHook).
    Entry {
        addr: 0x004c_a200,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G46: `InventoryChanges::GetNextOEIFastInventoryItem` 004ca330, its end
    // (InventoryIterGetNextHook).
    Entry {
        addr: 0x004c_afa3,
        owners: JIP,
        step: "3.9-GH1",
    },
    // G75: `00953ff0` (the player picks up a reference), the base form's type read: JIP's
    // HotkeyFixPlayerActivateHook (the seam is group GP's, in `playercharacter_p2.rs`).
    Entry {
        addr: 0x0095_400f,
        owners: JIP,
        step: "3.9-GH1",
    },
];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[
    // G50: `00482090` (a container's items' health), only called by address: JIP's
    // SetContainerItemsHealthHook rewrites it.
    Entry {
        addr: 0x0048_2090,
        owners: JIP,
        step: "3.9-GH1",
    },
];
