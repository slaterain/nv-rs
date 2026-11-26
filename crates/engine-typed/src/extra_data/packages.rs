//! A reference's package extra data and the rest of the unit's part 2 range (`0041c930` to
//! `0041da40`): the package (`ExtraPackage`) setter, getters and remover, the trespass package, the
//! player's crime list, the two scans that decide whether a list holds only default extra data,
//! the leveled item's index and flag, the persistent cell, the ragdoll data, the run-once packages,
//! the distant data's land normal and the enable state parent.

use super::{
    access, first_of_kind, give_lock, head_of, kind_of, list_add, list_remove, new_object,
    next_node, next_of, setters, take_lock, word_at, EXTRA_PAYLOAD, FAMILY, OPERATOR_DELETE,
};
use engine_core::{args, typed_call, Engine};

/// The extra data types (`cEtype`) of this file.
const KIND_PERSISTENT_CELL: u8 = 0x0c;
const KIND_DISTANT_DATA: u8 = 0x13;
const KIND_RAG_DOLL_DATA: u8 = 0x14;
const KIND_PACKAGE: u8 = 0x19;
const KIND_TRESPASS_PACKAGE: u8 = 0x1a;
const KIND_RUN_ONCE_PACKAGES: u8 = 0x1b;
const KIND_WORN: u8 = 0x16;
const KIND_OWNERSHIP: u8 = 0x21;
const KIND_LEVELED_ITEM: u8 = 0x2f;
const KIND_PLAYER_CRIME_LIST: u8 = 0x35;
const KIND_ENABLE_STATE_PARENT: u8 = 0x37;
/// JIP LN NVSE's keyed data (`ExtraJIP`), whose entries JIP writes in the scans' tables.
const JIP_KEYED_DATA_TYPE: u8 = 0x27;

/// The constructors (other units') the setters build their extra data with (`thiscall(block, ...)`).
const EXTRA_PACKAGE_INIT: u32 = 0x0043_2870;
const EXTRA_TRESPASS_PACKAGE_INIT: u32 = 0x0043_28d0;
const EXTRA_PLAYER_CRIME_LIST_INIT: u32 = 0x0043_2aa0;
const EXTRA_LEVELED_ITEM_INIT: u32 = 0x0043_2be0;
const EXTRA_PERSISTENT_CELL_INIT: u32 = 0x0043_2c20;
const EXTRA_RAG_DOLL_DATA_INIT: u32 = 0x0043_2cb0;
const EXTRA_RUN_ONCE_PACKAGES_INIT: u32 = 0x0043_3010;
const EXTRA_DISTANT_DATA_INIT: u32 = 0x0043_3260;
const EXTRA_ENABLE_STATE_PARENT_INIT: u32 = 0x0043_3300;
/// `RagDollData`'s constructor, `UpdateDataFromReference` and `Copy` (Xbox PDB, `thiscall(data,
/// source)`).
const RAG_DOLL_DATA_INIT: u32 = 0x004d_9330;
const RAG_DOLL_UPDATE_FROM_REFERENCE: u32 = 0x004d_9800;
const RAG_DOLL_COPY: u32 = 0x004d_9670;
/// `ExtraRunOncePacks`' add of a package and a flag (`thiscall(extra, package, flag)`).
const RUN_ONCE_PACKAGES_ADD: u32 = 0x0043_31c0;
/// `TESPackage::GetIsCreated`, `IsNeverToRun` and `SetIsCreated(bool)` (Xbox PDB).
const PACKAGE_GET_IS_CREATED: u32 = 0x0067_4d40;
const PACKAGE_IS_NEVER_TO_RUN: u32 = 0x0067_4e40;
const PACKAGE_SET_IS_CREATED: u32 = 0x0067_4d70;
/// The add at the head of a `BSSimpleList` of the word at an address (`thiscall(list, &word)`).
const LIST_ADD_HEAD: u32 = 0x005a_e3d0;
/// What the crime list's remover runs on each crime with its word (`thiscall(crime, word)`), the
/// list's node free and its scalar deleting destructor.
const CRIME_REMOVE_ENTRY: u32 = 0x009e_ba00;
const SIMPLE_LIST_FREE_NODES: u32 = 0x0047_0470;
const SIMPLE_LIST_DELETE: u32 = 0x0047_02f0;
/// `BSSimpleList::Remove` of the item equal to the word at an address (`thiscall(list, &word)`).
const SIMPLE_LIST_REMOVE_VALUE: u32 = 0x0090_5330;
/// A package's type (`0041ca90`'s signed byte at +0x20) and its time stamp (+0x38, `0041d8a0`).
const PACKAGE_TYPE: u32 = 0x20;
const PACKAGE_TIME_STAMP: u32 = 0x38;
/// An `ExtraPackage`'s index (+0x10), target (+0x14) and three flag bytes (+0x18 to +0x1a).
const PACKAGE_INDEX: u32 = 0x10;
const PACKAGE_TARGET: u32 = 0x14;
const PACKAGE_ACTION_COMPLETE: u32 = 0x18;
const PACKAGE_ACTIVATED: u32 = 0x19;
const PACKAGE_DONE_ONCE: u32 = 0x1a;

// ---- What the functions share ----------------------------------------------------------------

/// A package's type: the signed byte at +0x20 (`0041ca90`, in place).
#[inline]
fn package_type_of(e: &Engine, package: u32) -> i32 {
    i32::from(e.mem.u8(package.wrapping_add(PACKAGE_TYPE)) as i8)
}

/// The byte at `offset` of the type 0x19 extra data (the package), or 0.
fn package_byte(e: &mut Engine, list: u32, offset: u32) -> u8 {
    match first_of_kind(e, list, KIND_PACKAGE) {
        0 => 0,
        extra => e.mem.u8(extra.wrapping_add(offset)),
    }
}

/// The first extra data of `kind` removed and deleted, if there is one.
fn remove_found(e: &mut Engine, list: u32, kind: u8) {
    let extra = first_of_kind(e, list, kind);
    if extra != 0 {
        list_remove(e, list, extra, true);
    }
}

/// The ragdoll setters: with the type 0x14 extra data, a null `source` deletes it and any other is
/// handed to `fill` with its data; without one, a non-null `source` builds an `ExtraRagDollData`
/// (0x10 bytes, `00432cb0`) and its `RagDollData` (0x14 bytes, `004d9330`), stores the data in it
/// (to null plus 0xc when the first allocation failed), fills it and adds the extra data.
fn rag_doll_setter(e: &mut Engine, list: u32, source: u32, fill: u32) {
    let extra = first_of_kind(e, list, KIND_RAG_DOLL_DATA);
    if extra != 0 {
        if source == 0 {
            list_remove(e, list, extra, true);
        } else {
            let data = word_at(e, extra, EXTRA_PAYLOAD);
            e.call(fill, &args![data, source]);
        }
        return;
    }
    if source == 0 {
        return;
    }
    let holder = new_object(e, 0x10, |e, block| {
        e.call(EXTRA_RAG_DOLL_DATA_INIT, &args![block]).u32()
    });
    let data = new_object(e, 0x14, |e, block| {
        e.call(RAG_DOLL_DATA_INIT, &args![block]).u32()
    });
    e.mem.set_u32(holder.wrapping_add(EXTRA_PAYLOAD), data);
    e.call(fill, &args![data, source]);
    list_add(e, list, holder);
}

// ---- The functions --------------------------------------------------------------------------

// Typed from 0041c930
/// `ExtraDataList::SetPackageExtra` (Xbox PDB): a created package (`GetIsCreated`) whose type is not
/// 1, one that never runs (`IsNeverToRun`), or one of type 0x18 or 0x17 is ignored. Otherwise a
/// null `package` deletes the type 0x19 extra data ([`remove_package`]); any other is stored with
/// the index, the target and the three flags, or one (0x1c bytes, `00432870`, the flags
/// zero-extended) built from them and added. The package's type is read in place, once after each
/// call that could change it.
#[allow(clippy::too_many_arguments)]
pub fn set_package(
    e: &mut Engine,
    this: u32,
    package: u32,
    index: i32,
    target: u32,
    action_complete: u8,
    activated: u8,
    done_once: u8,
) {
    if package != 0 {
        let created = e.call(PACKAGE_GET_IS_CREATED, &args![package]).bool();
        if created && package_type_of(e, package) != 1 {
            return;
        }
        if e.call(PACKAGE_IS_NEVER_TO_RUN, &args![package]).bool() {
            return;
        }
        let kind = package_type_of(e, package);
        if kind == 0x18 || kind == 0x17 {
            return;
        }
    }
    let extra = first_of_kind(e, this, KIND_PACKAGE);
    if package == 0 {
        if extra != 0 {
            typed_call!(FAMILY, 0x0041_cc60, remove_package(e, this));
        }
    } else if extra == 0 {
        let created = new_object(e, 0x1c, |e, block| {
            e.call(
                EXTRA_PACKAGE_INIT,
                &args![
                    block,
                    package,
                    index as u32,
                    target,
                    u32::from(action_complete),
                    u32::from(activated),
                    u32::from(done_once)
                ],
            )
            .u32()
        });
        list_add(e, this, created);
    } else {
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), package);
        e.mem
            .set_u32(extra.wrapping_add(PACKAGE_INDEX), index as u32);
        e.mem.set_u32(extra.wrapping_add(PACKAGE_TARGET), target);
        e.mem
            .set_u8(extra.wrapping_add(PACKAGE_ACTION_COMPLETE), action_complete);
        e.mem
            .set_u8(extra.wrapping_add(PACKAGE_ACTIVATED), activated);
        e.mem
            .set_u8(extra.wrapping_add(PACKAGE_DONE_ONCE), done_once);
    }
}

// Typed from 0041cab0
/// `ExtraDataList::SetPackageExtraTarget` (Xbox PDB): the target (+0x14) of the type 0x19 extra data
/// set when there is one.
pub fn set_package_target(e: &mut Engine, this: u32, target: u32) {
    let extra = first_of_kind(e, this, KIND_PACKAGE);
    if extra != 0 {
        e.mem.set_u32(extra.wrapping_add(PACKAGE_TARGET), target);
    }
}

// Typed from 0041cae0
/// `ExtraDataList::SetPackageExtraIndex` (Xbox PDB): the index (+0x10) of the type 0x19 extra data
/// set when there is one.
pub fn set_package_index(e: &mut Engine, this: u32, index: i32) {
    let extra = first_of_kind(e, this, KIND_PACKAGE);
    if extra != 0 {
        e.mem
            .set_u32(extra.wrapping_add(PACKAGE_INDEX), index as u32);
    }
}

// Typed from 0041cb10
/// `ExtraDataList::GetPackageExtra` (Xbox PDB): the package (+0xc of the type 0x19 extra data), or 0.
pub fn package(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_PACKAGE, 0)
}

// Typed from 0041cb40
/// `ExtraDataList::GetPackageExtraIndex` (Xbox PDB): the index (+0x10 of the type 0x19 extra data),
/// or 0.
pub fn package_index(e: &mut Engine, this: u32) -> i32 {
    match first_of_kind(e, this, KIND_PACKAGE) {
        0 => 0,
        extra => word_at(e, extra, PACKAGE_INDEX) as i32,
    }
}

// Typed from 0041cba0
/// Whether the package's action is complete (the byte at +0x18 of the type 0x19 extra data), or 0.
pub fn package_action_complete(e: &mut Engine, this: u32) -> u8 {
    package_byte(e, this, PACKAGE_ACTION_COMPLETE)
}

// Typed from 0041cbd0
/// `ExtraDataList::SetPackageExtraActionComplete` (Xbox PDB): the byte at +0x18 of the type 0x19
/// extra data set when there is one.
pub fn set_package_action_complete(e: &mut Engine, this: u32, action_complete: u8) {
    let extra = first_of_kind(e, this, KIND_PACKAGE);
    if extra != 0 {
        e.mem
            .set_u8(extra.wrapping_add(PACKAGE_ACTION_COMPLETE), action_complete);
    }
}

// Typed from 0041cc00
/// Whether the package was activated (the byte at +0x19 of the type 0x19 extra data), or 0.
pub fn package_activated(e: &mut Engine, this: u32) -> u8 {
    package_byte(e, this, PACKAGE_ACTIVATED)
}

// Typed from 0041cc30
/// Whether the package was done once (the byte at +0x1a of the type 0x19 extra data), or 0.
pub fn package_done_once(e: &mut Engine, this: u32) -> u8 {
    package_byte(e, this, PACKAGE_DONE_ONCE)
}

// Typed from 0041cc60
/// `ExtraDataList::RemovePackageExtra` (Xbox PDB): the type 0x19 extra data removed and deleted, if
/// there is one.
pub fn remove_package(e: &mut Engine, this: u32) {
    remove_found(e, this, KIND_PACKAGE);
}

// Typed from 0041cc90
/// The trespass package (+0xc of the type 0x1a extra data) set: the old one deleted first (its slot
/// 0x10 with 1) when there is one; or one (0x10 bytes, `004328d0`) built from it and added, a null
/// one too.
pub fn set_trespass_package(e: &mut Engine, this: u32, package: u32) {
    let extra = first_of_kind(e, this, KIND_TRESPASS_PACKAGE);
    if extra == 0 {
        access::add_new(e, this, 0x10, EXTRA_TRESPASS_PACKAGE_INIT, Some(package));
        return;
    }
    let old = word_at(e, extra, EXTRA_PAYLOAD);
    if old != 0 {
        e.vcall(old, 0x10, &args![1u32]);
    }
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), package);
}

// Typed from 0041cd70
/// The trespass package (+0xc of the type 0x1a extra data), or 0.
pub fn trespass_package(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_TRESPASS_PACKAGE, 0)
}

// Typed from 0041cda0
/// `ExtraDataList::RemoveTrespassPackage` (Xbox PDB): with the type 0x1a extra data, its package
/// marked created (`TESPackage::SetIsCreated(1)`), the package cleared and the extra data deleted.
/// (The exe's `TESSaveLoadGame::DeleteForm` between them runs only when the save/load singleton's
/// test, `0047c850`, answers true, which it never does: in place.)
pub fn remove_trespass_package(e: &mut Engine, this: u32) {
    let extra = first_of_kind(e, this, KIND_TRESPASS_PACKAGE);
    if extra == 0 {
        return;
    }
    let package = word_at(e, extra, EXTRA_PAYLOAD);
    e.call(PACKAGE_SET_IS_CREATED, &args![package, 1u32]);
    e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), 0);
    list_remove(e, this, extra, true);
}

// Typed from 0041ce10
/// The trespass package (+0xc of the type 0x1a extra data) cleared when it is `package`.
pub fn forget_trespass_package(e: &mut Engine, this: u32, package: u32) {
    let extra = first_of_kind(e, this, KIND_TRESPASS_PACKAGE);
    if extra != 0 && word_at(e, extra, EXTRA_PAYLOAD) == package {
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), 0);
    }
}

// Typed from 0041ce50
/// `ExtraDataList::AddToPlayerCrimeList` (Xbox PDB): without the type 0x35 extra data, one (0x10
/// bytes, `00432aa0`) built from `crime` and added; with one, `crime` added at the head of its list
/// (+0xc, `005ae3d0`, given a local holding it).
pub fn add_to_player_crime_list(e: &mut Engine, this: u32, crime: u32) {
    let extra = first_of_kind(e, this, KIND_PLAYER_CRIME_LIST);
    if extra == 0 {
        access::add_new(e, this, 0x10, EXTRA_PLAYER_CRIME_LIST_INIT, Some(crime));
        return;
    }
    let crimes = word_at(e, extra, EXTRA_PAYLOAD);
    e.with_stack(4, |e, local| {
        e.mem.set_u32(local.addr(), crime);
        e.call(LIST_ADD_HEAD, &args![crimes, local]);
    });
}

// Typed from 0041cf00
/// `ExtraDataList::GetPlayerCrimeList` (Xbox PDB): the list (+0xc of the type 0x35 extra data), or 0.
pub fn player_crime_list(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_PLAYER_CRIME_LIST, 0)
}

// Typed from 0041cf30
/// `ExtraDataList::RemovePlayerCrimeListExtra` (Xbox PDB): with the type 0x35 extra data and a list
/// in it, `009eba00` run with `value` on each crime up to the first empty item, the list's nodes
/// freed (`00470470`), the list deleted (`004702f0` with 1) and cleared; then the extra data
/// deleted. Each item is read once (the node is its slot, `006815c0`, in place).
pub fn remove_player_crime_list(e: &mut Engine, this: u32, value: u32) {
    let extra = first_of_kind(e, this, KIND_PLAYER_CRIME_LIST);
    if extra == 0 {
        return;
    }
    let mut node = word_at(e, extra, EXTRA_PAYLOAD);
    if node != 0 {
        while node != 0 {
            let crime = e.mem.u32(node);
            if crime == 0 {
                break;
            }
            e.call(CRIME_REMOVE_ENTRY, &args![crime, value]);
            node = next_node(e, node);
        }
        let crimes = word_at(e, extra, EXTRA_PAYLOAD);
        e.call(SIMPLE_LIST_FREE_NODES, &args![crimes]);
        let crimes = word_at(e, extra, EXTRA_PAYLOAD);
        if crimes != 0 {
            e.call(SIMPLE_LIST_DELETE, &args![crimes, 1u32]);
        }
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), 0);
    }
    list_remove(e, this, extra, true);
}

// Typed from 0041d000
/// The type 0x2f extra data (the leveled item) removed and deleted, if there is one.
pub fn remove_leveled_item(e: &mut Engine, this: u32) {
    remove_found(e, this, KIND_LEVELED_ITEM);
}

// Typed from 0041d030
/// Whether the list holds an owner (type 0x21) and otherwise only the types 0xd, 0x1c, 0x20, 0x24,
/// 0x27, 0x2f, 0x30 and 0x4a: a walk under the extra data lock (name 0) that any other type ends
/// with false.
///
/// JIP LN NVSE's ExtraJIP framework (`SafeWrite8(0x41D0F6, 3)`) moves its keyed data's type 0x27 to
/// the arm that ends the walk with false while JIP is installed (the seam answers whether it does).
pub fn only_owner_defaults(e: &mut Engine, this: u32) -> bool {
    take_lock(e, 0);
    let mut owned = false;
    let mut node = head_of(e, this);
    while node != 0 {
        let kind = kind_of(e, node);
        if kind == JIP_KEYED_DATA_TYPE
            && e.jip_patches() & engine_core::extend::jip_patches::INSTALLED != 0
            && e.site(0x0041_d0f6, &[this]).is_some()
        {
            give_lock(e);
            return false;
        }
        match kind {
            0x0d | 0x1c | 0x20 | 0x24 | 0x27 | 0x2f | 0x30 | 0x4a => {}
            KIND_OWNERSHIP => owned = true,
            _ => {
                give_lock(e);
                return false;
            }
        }
        node = next_of(e, node);
    }
    give_lock(e);
    owned
}

// Typed from 0041d120
/// `ExtraDataList::IsExtraDefaultforContainer` (Xbox PDB): whether every extra data of the list is
/// of the types 0xd, 0x1c, 0x20, 0x21, 0x24, 0x27, 0x2f, 0x30 and 0x4a, or 0x16 (worn) with
/// `worn_allowed`: a walk under the extra data lock (name 0) that any other type ends with false.
///
/// JIP LN NVSE's ExtraJIP framework (`SafeWrite8(0x41D212, 1)` and `SafeWrite8(0x41D25A, 1)`, the
/// entries of 0x27 in the exe's two tables, worn allowed and not) makes its keyed data's type end
/// the walk with false while JIP is installed (the seam of the table the flag picks answers whether
/// it does).
pub fn is_default_for_container(e: &mut Engine, this: u32, worn_allowed: u8) -> bool {
    take_lock(e, 0);
    let mut node = head_of(e, this);
    while node != 0 {
        let kind = kind_of(e, node);
        if kind == JIP_KEYED_DATA_TYPE
            && e.jip_patches() & engine_core::extend::jip_patches::INSTALLED != 0
        {
            let asked = if worn_allowed != 0 {
                e.site(0x0041_d212, &[this])
            } else {
                e.site(0x0041_d25a, &[this])
            };
            if asked.is_some() {
                give_lock(e);
                return false;
            }
        }
        let allowed = matches!(
            kind,
            0x0d | 0x1c | 0x20 | 0x21 | 0x24 | 0x27 | 0x2f | 0x30 | 0x4a
        ) || (worn_allowed != 0 && kind == KIND_WORN);
        if !allowed {
            give_lock(e);
            return false;
        }
        node = next_of(e, node);
    }
    give_lock(e);
    true
}

// Typed from 0041d280
/// The leveled item's index (+0xc of the type 0x2f extra data) stored, or one (0x14 bytes,
/// `00432be0`) built from it and added; then its default flag (+0x10) cleared (to null plus 0x10
/// when the allocation failed, as the exe does).
pub fn set_leveled_item_index(e: &mut Engine, this: u32, index: i32) {
    let mut extra = first_of_kind(e, this, KIND_LEVELED_ITEM);
    if extra == 0 {
        extra = access::add_new(e, this, 0x14, EXTRA_LEVELED_ITEM_INIT, Some(index as u32));
    } else {
        e.mem
            .set_u32(extra.wrapping_add(EXTRA_PAYLOAD), index as u32);
    }
    e.mem.set_u8(extra.wrapping_add(0x10), 0);
}

// Typed from 0041d330
/// The leveled item's default flag (+0x10 of the type 0x2f extra data) set when there is one.
pub fn set_leveled_item_default(e: &mut Engine, this: u32, default_flag: u8) {
    let extra = first_of_kind(e, this, KIND_LEVELED_ITEM);
    if extra != 0 {
        e.mem.set_u8(extra.wrapping_add(0x10), default_flag);
    }
}

// Typed from 0041d360
/// The leveled item's index (+0xc of the type 0x2f extra data), or -1.
pub fn leveled_item_index(e: &mut Engine, this: u32) -> i32 {
    access::word_or(e, this, KIND_LEVELED_ITEM, u32::MAX) as i32
}

// Typed from 0041d390
/// `ExtraDataList::SetPersistentCell` (Xbox PDB): the cell (+0xc of the type 0xc extra data) set; a
/// null one deletes it (built by `00432c20`).
pub fn set_persistent_cell(e: &mut Engine, this: u32, cell: u32) {
    access::set_or_remove(
        e,
        this,
        KIND_PERSISTENT_CELL,
        EXTRA_PERSISTENT_CELL_INIT,
        cell,
        0,
    );
}

// Typed from 0041d460
/// The persistent cell (+0xc of the type 0xc extra data), or 0.
pub fn persistent_cell(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_PERSISTENT_CELL, 0)
}

// Typed from 0041d490
/// `ExtraDataList::SetRagDollData` (Xbox PDB): `rag_doll_setter` with
/// `RagDollData::UpdateDataFromReference` (`004d9800`) filling the data from `reference`.
pub fn set_rag_doll_data(e: &mut Engine, this: u32, reference: u32) {
    rag_doll_setter(e, this, reference, RAG_DOLL_UPDATE_FROM_REFERENCE);
}

// Typed from 0041d5b0
/// `ExtraDataList::SetRagDollDataFromCopy` (Xbox PDB): `rag_doll_setter` with `RagDollData::Copy`
/// (`004d9670`) copying `source`.
pub fn set_rag_doll_data_from_copy(e: &mut Engine, this: u32, source: u32) {
    rag_doll_setter(e, this, source, RAG_DOLL_COPY);
}

// Typed from 0041d6d0
/// `ExtraDataList::GetRagDollData` (Xbox PDB): the data (+0xc of the type 0x14 extra data), or 0.
pub fn rag_doll_data(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_RAG_DOLL_DATA, 0)
}

// Typed from 0041d700
/// A package and its flag added to the run-once packages (`004331c0` on the type 0x1b extra data;
/// one, 0x10 bytes, `00433010`, built and added when the list has none).
pub fn add_run_once_package(e: &mut Engine, this: u32, package: u32, flag: u8) {
    let mut extra = first_of_kind(e, this, KIND_RUN_ONCE_PACKAGES);
    if extra == 0 {
        extra = access::add_new(e, this, 0x10, EXTRA_RUN_ONCE_PACKAGES_INIT, None);
    }
    e.call(
        RUN_ONCE_PACKAGES_ADD,
        &args![extra, package, u32::from(flag)],
    );
}

// Typed from 0041d7b0
/// The run-once packages (the list at +0xc of the type 0x1b extra data) pruned, up to the first
/// empty item: an entry (a package and a signed byte) is kept when its byte is `flag` and it has
/// not expired; any other is removed (`00905330`, given a local holding it), deleted, and the walk
/// starts again at the list's head. An entry with a package expires when the byte at +3 of the
/// package's time stamp (+0x38) plus the stamp's word at +4 is at least 0x15. The item, the
/// package and the stamp's fields are read in place (`006815c0`, `0041d8a0`, `0086a460`,
/// `00726070`).
pub fn prune_run_once_packages(e: &mut Engine, this: u32, flag: i8) {
    let extra = first_of_kind(e, this, KIND_RUN_ONCE_PACKAGES);
    if extra == 0 {
        return;
    }
    let mut node = word_at(e, extra, EXTRA_PAYLOAD);
    while node != 0 {
        let entry = e.mem.u32(node);
        if entry == 0 {
            break;
        }
        let package = e.mem.u32(entry);
        let expired = package != 0 && {
            let stamp = package.wrapping_add(PACKAGE_TIME_STAMP);
            let day = i32::from(e.mem.u8(stamp.wrapping_add(3)) as i8);
            let hours = word_at(e, stamp, 4) as i32;
            day.wrapping_add(hours) >= 0x15
        };
        if e.mem.u8(entry.wrapping_add(4)) as i8 == flag && !expired {
            node = next_node(e, node);
            continue;
        }
        let packages = word_at(e, extra, EXTRA_PAYLOAD);
        let removed = e.with_stack(4, |e, local| {
            e.mem.set_u32(local.addr(), entry);
            e.call(SIMPLE_LIST_REMOVE_VALUE, &args![packages, local]);
            e.mem.u32(local.addr())
        });
        e.call(OPERATOR_DELETE, &args![removed]);
        node = word_at(e, extra, EXTRA_PAYLOAD);
    }
}

// Typed from 0041d8a0
/// A package's time stamp: the address of its member at +0x38.
pub fn package_time_stamp(_e: &mut Engine, this: u32) -> u32 {
    this.wrapping_add(PACKAGE_TIME_STAMP)
}

// Typed from 0041d8c0
/// `ExtraDataList::IsInRunOnceDayPackageList` (Xbox PDB): whether the run-once packages hold, before
/// the first empty item, an entry whose package is `package`.
pub fn is_in_run_once_packages(e: &mut Engine, this: u32, package: u32) -> bool {
    let extra = first_of_kind(e, this, KIND_RUN_ONCE_PACKAGES);
    if extra == 0 {
        return false;
    }
    let mut node = word_at(e, extra, EXTRA_PAYLOAD);
    while node != 0 {
        let entry = e.mem.u32(node);
        if entry == 0 {
            break;
        }
        if e.mem.u32(entry) == package {
            return true;
        }
        node = next_node(e, node);
    }
    false
}

// Typed from 0041d930
/// The type 0x1b extra data (the run-once packages) deleted.
pub fn remove_run_once_packages(e: &mut Engine, this: u32) {
    setters::remove_kind(e, this, KIND_RUN_ONCE_PACKAGES);
}

// Typed from 0041d950
/// The distant data's land normal (the point at +0xc of the type 0x13 extra data; one, 0x18 bytes,
/// `00433260`, built and added when the list has none) set from the three words at `normal`.
pub fn set_land_normal(e: &mut Engine, this: u32, normal: u32) {
    let mut extra = first_of_kind(e, this, KIND_DISTANT_DATA);
    if extra == 0 {
        extra = access::add_new(e, this, 0x18, EXTRA_DISTANT_DATA_INIT, None);
    }
    for offset in [0u32, 4, 8] {
        let word = e.mem.u32(normal.wrapping_add(offset));
        e.mem
            .set_u32(extra.wrapping_add(EXTRA_PAYLOAD + offset), word);
    }
}

// Typed from 0041da10
/// The enable state parent (+0xc of the type 0x37 extra data), or 0.
pub fn enable_state_parent(e: &mut Engine, this: u32) -> u32 {
    access::word_or(e, this, KIND_ENABLE_STATE_PARENT, 0)
}

// Typed from 0041da40
/// The enable state parent set: a null `parent` deletes the first type 0x37 extra data; any other
/// is stored in it, or in a new one (0x14 bytes, `00433300`, which takes nothing: the parent is
/// stored after it, to null plus 0xc when the allocation failed) that is then added.
pub fn set_enable_state_parent(e: &mut Engine, this: u32, parent: u32) {
    if parent == 0 {
        setters::remove_kind(e, this, KIND_ENABLE_STATE_PARENT);
        return;
    }
    let extra = first_of_kind(e, this, KIND_ENABLE_STATE_PARENT);
    if extra != 0 {
        e.mem.set_u32(extra.wrapping_add(EXTRA_PAYLOAD), parent);
        return;
    }
    let created = new_object(e, 0x14, |e, block| {
        e.call(EXTRA_ENABLE_STATE_PARENT_INIT, &args![block]).u32()
    });
    e.mem.set_u32(created.wrapping_add(EXTRA_PAYLOAD), parent);
    list_add(e, this, created);
}
