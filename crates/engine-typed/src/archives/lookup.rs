//! Which listed archive owns a file entry: the walk over the archive list (`00af6910`), an
//! archive's test of its directories (`00afdc00`) and a directory's test of its file entries
//! (`00afdbc0`). The same reads, writes and calls in the same order as the exe; the walk's calls
//! to the other two are direct.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::sync_direct;
use engine_core::{typed_call, Engine};

/// The archive list (`011f8160`): its first node; each node is `(archive, next)`.
const ARCHIVE_LIST: u32 = 0x011f_8160;
/// The archive list's critical section (`011f8170`).
const ARCHIVE_LIST_LOCK: u32 = 0x011f_8170;
/// The type mask that means any archive.
const ANY_TYPE: u32 = 0xffff;
/// An archive's `NiRefObject` part (+0x158: its vtable, +0x15c `m_uiRefCount`).
const REF_OBJECT: u32 = 0x158;
const REF_COUNT: u32 = 0x15c;
/// `NiRefObject`'s `DeleteThis` slot.
const DELETE_THIS: u32 = 4;
/// The archive's `BSArchive` header (+0x160): `iDirectories` at +0x10 of it, `pDirectories`
/// (16-byte directory entries) at +0x24.
const HEADER: u32 = 0x160;
const DIRECTORIES: u32 = 0x10;
const DIRECTORY_TABLE: u32 = 0x24;
/// `iArchiveType` (+0x180), the type bits a mask is tested against.
const ARCHIVE_TYPE: u32 = 0x180;

// Typed from 00afdbc0
/// `thiscall(directory, address)`: whether `address` lies in the directory's block of 16-byte
/// file entries (starting at its word +0xc, as many as its word +0x8), compared unsigned, the end
/// wrapping as the exe's shift and add.
pub fn directory_holds(e: &mut Engine, this: u32, address: u32) -> bool {
    let start = e.mem.u32(this.wrapping_add(0xc));
    if address < start {
        return false;
    }
    let end = (e.mem.u32(this.wrapping_add(8)) << 4).wrapping_add(start);
    address < end
}

// Typed from 00afdc00
/// `thiscall(header, address)`: whether any of the archive's directories (`iDirectories` of them
/// in `pDirectories`, both read again for each) holds `address` ([`directory_holds`]), in order.
pub fn archive_holds(e: &mut Engine, this: Ptr, address: u32) -> bool {
    let this = this.addr();
    let mut index = 0u32;
    while index < e.mem.u32(this.wrapping_add(DIRECTORIES)) {
        let directory = e
            .mem
            .u32(this.wrapping_add(DIRECTORY_TABLE))
            .wrapping_add(index.wrapping_mul(16));
        if typed_call!(FAMILY, 0x00af_dbc0, directory_holds(e, directory, address)) {
            return true;
        }
        index += 1;
    }
    false
}

/// Whether the archive's type bits meet `mask`.
fn type_matches(e: &Engine, archive: u32, mask: u32) -> bool {
    u32::from(e.mem.u16(archive.wrapping_add(ARCHIVE_TYPE))) & mask & 0xffff != 0
}

/// A reference added to `archive` (`InterlockedIncrement` of its count).
fn add_ref(e: &mut Engine, archive: u32) {
    sync_direct::interlocked_increment(e, archive.wrapping_add(REF_COUNT));
}

/// A reference released: `InterlockedDecrement` of the count, `DeleteThis` at 0.
fn release(e: &mut Engine, archive: u32) {
    if sync_direct::interlocked_decrement(e, archive.wrapping_add(REF_COUNT)) == 0 {
        e.vcall(archive.wrapping_add(REF_OBJECT), DELETE_THIS, &[]);
    }
}

/// One pass over the list: the first archive `wanted` takes that holds `entry`, with the list's
/// lock given back; `None` when none does (the lock still held). Each archive is held by a
/// reference while asked.
fn pass(e: &mut Engine, entry: u32, wanted: impl Fn(&Engine, u32) -> bool) -> Option<u32> {
    let mut node = e.mem.u32(ARCHIVE_LIST);
    while node != 0 {
        let archive = e.mem.u32(node);
        if archive != 0 {
            add_ref(e, archive);
        }
        if archive != 0
            && wanted(e, archive)
            && typed_call!(
                FAMILY,
                0x00af_dc00,
                archive_holds(e, Ptr::new(archive.wrapping_add(HEADER)), entry)
            )
        {
            sync_direct::leave_critical_section(e, ARCHIVE_LIST_LOCK);
            release(e, archive);
            return Some(archive);
        }
        node = e.mem.u32(node.wrapping_add(4));
        if archive != 0 {
            release(e, archive);
        }
    }
    None
}

// Typed from 00af6910
/// `cdecl(entry, types)`: under the archive list's critical section, the first listed archive
/// whose type bits meet `types` (or any, for 0xffff) and that holds `entry` ([`archive_holds`]);
/// then, unless `types` is 0xffff, the first of those whose type bits do not. Each archive is held
/// by a reference while asked; the one found is answered after its reference is released, as the
/// exe does. Null when none holds it: for 0xffff the exe returns then without leaving the critical
/// section, and so does this.
pub fn owner_of(e: &mut Engine, entry: Ptr, types: u32) -> u32 {
    let entry = entry.addr();
    sync_direct::enter_critical_section(e, ARCHIVE_LIST_LOCK);
    let first = pass(e, entry, |e, archive| {
        type_matches(e, archive, types) || types == ANY_TYPE
    });
    if let Some(archive) = first {
        return archive;
    }
    if types == ANY_TYPE {
        return 0;
    }
    if let Some(archive) = pass(e, entry, |e, archive| !type_matches(e, archive, types)) {
        return archive;
    }
    sync_direct::leave_critical_section(e, ARCHIVE_LIST_LOCK);
    0
}
