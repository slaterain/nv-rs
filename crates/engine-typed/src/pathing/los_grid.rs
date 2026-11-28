//! The LOS grid map's folded `NiTMap<unsigned int, PathingLOSGrid *>` table: its constructors,
//! destructors and deleting destructor, the `BSSimpleArray` constructors and destructors of its
//! cell list, and the element-offset forwarder of the LOS grid map unit.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The vtable of `NiTMap<unsigned int, PathingLOSGrid *>` (`0106ceb8`).
const GRID_TABLE_VTABLE: u32 = 0x0106_ceb8;
/// The vtable of its base `NiTMapBase` (`0106ced8`).
const GRID_TABLE_BASE_VTABLE: u32 = 0x0106_ced8;
/// The vtable of `BSSimpleArray<char,1024>` and `BSSimpleArray<unsigned int,1024>` (`0106cf0c`).
const ARRAY_VTABLE: u32 = 0x0106_cf0c;
/// The vtable of `BSSimpleArray<PathingLOSGridCell *,1024>` (`0106cef8`).
const CELL_ARRAY_VTABLE: u32 = 0x0106_cef8;

/// Callees.
const OFFSET_TARGET: u32 = 0x006a_62f0;
const TABLE_ALLOCATE: u32 = 0x00aa_1070;
const MEMSET: u32 = 0x0040_3d30;
const REMOVE_ALL: u32 = 0x0043_8af0;
const FREE_TABLE: u32 = 0x00aa_10f0;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const ARRAY_RELEASE: u32 = 0x0084_54f0;
const CHAR_ARRAY_INIT: u32 = 0x0069_b940;

// Typed from 006e0ed0
/// Forwards `this` and one word to `006a62f0` (the word at `this+4` plus the argument); answers
/// the address it returns.
pub fn element_at(e: &mut Engine, this: Ptr, offset: u32) -> Ptr {
    e.call(OFFSET_TARGET, &args![this, offset]).ptr()
}

// Typed from 006e0f20
/// The `NiTMapBase<unsigned int, PathingLOSGrid *>` constructor (by the vtable it sets): the
/// base vtable `0106ced8` at `+0`, `buckets` at `+4`, 0 at `+0xC`, then the bucket table at `+8`:
/// `buckets * 4` bytes (wrapping) from `00aa1070`, cleared with `00403d30` (the count and the
/// table are reread after the allocation). Answers `this`.
pub fn table_base_ctor(e: &mut Engine, this: Ptr, buckets: u32) -> Ptr {
    e.mem.set_u32(this.addr(), GRID_TABLE_BASE_VTABLE);
    e.mem.set_u32(this.addr() + 4, buckets);
    e.mem.set_u32(this.addr() + 0xc, 0);
    let size = e.mem.u32(this.addr() + 4).wrapping_shl(2);
    let table = e.call(TABLE_ALLOCATE, &args![size]).u32();
    e.mem.set_u32(this.addr() + 8, table);
    let size = e.mem.u32(this.addr() + 4).wrapping_shl(2);
    let table = e.mem.u32(this.addr() + 8);
    e.call(MEMSET, &args![table, 0u32, size]);
    this
}

// Typed from 006e0ea0
/// The `NiTMap<unsigned int, PathingLOSGrid *>` constructor (by the vtable it sets): the base
/// (`006e0f20`) with `buckets`, then its own vtable `0106ceb8`. Answers `this`.
pub fn table_ctor(e: &mut Engine, this: Ptr, buckets: u32) -> Ptr {
    let _: Ptr = typed_call!(FAMILY, 0x006e_0f20, table_base_ctor(e, this, buckets));
    e.mem.set_u32(this.addr(), GRID_TABLE_VTABLE);
    this
}

// Typed from 006e0f90
/// The grid table's destructor (by the vtable it sets): its own vtable `0106ceb8`, removes every
/// item (`00438af0`), then the base destructor (`006e0ff0`).
pub fn table_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), GRID_TABLE_VTABLE);
    e.call(REMOVE_ALL, &args![this]);
    typed_call!(FAMILY, 0x006e_0ff0, table_base_dtor(e, this));
}

// Typed from 006e0ff0
/// The grid table base's destructor (by the vtable it sets): the base vtable `0106ced8`, removes
/// every item (`00438af0`), then frees the bucket table at `+8` (`00aa10f0`).
pub fn table_base_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), GRID_TABLE_BASE_VTABLE);
    e.call(REMOVE_ALL, &args![this]);
    let table = e.mem.u32(this.addr() + 8);
    e.call(FREE_TABLE, &args![table]);
}

// Typed from 006e0ef0
/// The `NiTMap<unsigned int, PathingLOSGrid *>` scalar deleting destructor (engine map): the
/// destructor (`006e0f90`), then `operator delete` when bit 0 of `flags` is set. Answers `this`.
pub fn table_scalar_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006e_0f90, table_dtor(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006e1070
/// The `BSSimpleArray<char,1024>` constructor (by the vtable it sets): its vtable `0106cf0c`,
/// then `0069b940(this, 0, 0)`. Answers `this`.
pub fn char_array_ctor(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), ARRAY_VTABLE);
    e.call(CHAR_ARRAY_INIT, &args![this, 0u32, 0u32]);
    this
}

// Typed from 006e10a0
/// The `BSSimpleArray<unsigned int,1024>` destructor (by the vtable it sets): its vtable
/// `0106cf0c`, then the array's release (`008454f0(this, 1)`).
pub fn word_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}

// Typed from 006e1050
/// The `BSSimpleArray<PathingLOSGridCell *,1024>` destructor (by the vtable it sets): the cell
/// array's vtable `0106cef8`, then the release (`008454f0(this, 1)`).
pub fn cell_array_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), CELL_ARRAY_VTABLE);
    e.call(ARRAY_RELEASE, &args![this, 1u32]);
}
