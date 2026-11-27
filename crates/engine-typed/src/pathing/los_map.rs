//! The LOS map's folded `NiTMap<unsigned int, unsigned int>`: its constructors, destructors and
//! deleting destructors, plus the byte min/max and children-array sizing helpers of the LOS map
//! unit.

use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The vtable of `NiTMap<unsigned int, unsigned int>` (`0106cf68`, by its RTTI).
const NI_T_MAP_VTABLE: u32 = 0x0106_cf68;
/// The vtable of its base `NiTMapBase` (`0106cf88`).
const NI_T_MAP_BASE_VTABLE: u32 = 0x0106_cf88;

/// Callees.
const TABLE_ALLOCATE: u32 = 0x00aa_1070;
const MEMSET: u32 = 0x0040_3d30;
const REMOVE_ALL: u32 = 0x0043_8af0;
const FREE_TABLE: u32 = 0x00aa_10f0;
const OPERATOR_DELETE: u32 = 0x0040_1030;
const CHILDREN_SET_SIZE: u32 = 0x0060_bef0;

/// An `NiNode`'s children array (+0x9C).
const CHILDREN: u32 = 0x9c;

// Typed from 006e20e0
/// The folded map's base constructor: the base vtable `0106cf88` at `+0`, the bucket count at
/// `+4`, 0 at `+0xC`, then the bucket table at `+8`: `count * 4` bytes from `00aa1070`, cleared
/// with `00403d30` (the count and the table are reread after the allocation). Answers `this`.
pub fn construct_base(e: &mut Engine, this: Ptr, buckets: u32) -> Ptr {
    e.mem.set_u32(this.addr(), NI_T_MAP_BASE_VTABLE);
    e.mem.set_u32(this.addr() + 4, buckets);
    e.mem.set_u32(this.addr() + 0xc, 0);
    let size = e.mem.u32(this.addr() + 4).wrapping_mul(4);
    let table = e.call(TABLE_ALLOCATE, &args![size]).u32();
    e.mem.set_u32(this.addr() + 8, table);
    let size = e.mem.u32(this.addr() + 4).wrapping_mul(4);
    let table = e.mem.u32(this.addr() + 8);
    e.call(MEMSET, &args![table, 0u32, size]);
    this
}

// Typed from 006e2080
/// The folded map's constructor: the base (`006e20e0`), then its own vtable `0106cf68`.
/// Answers `this`.
pub fn construct(e: &mut Engine, this: Ptr, buckets: u32) -> Ptr {
    let _: Ptr = typed_call!(FAMILY, 0x006e_20e0, construct_base(e, this, buckets));
    e.mem.set_u32(this.addr(), NI_T_MAP_VTABLE);
    this
}

// Typed from 006e20b0
/// `NiTMap<unsigned int, unsigned int>`'s scalar deleting destructor (engine map): the
/// destructor (`006e2150`), then `operator delete` when bit 0 of `flags` is set. Answers `this`.
pub fn ni_t_map_scalar_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006e_2150, ni_t_map_dtor(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006e2150
/// The `NiTMap<unsigned int, unsigned int>` destructor (by the vtable it sets): its own vtable
/// `0106cf68`, removes every item (`00438af0`), then the base destructor (`006e21b0`).
pub fn ni_t_map_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), NI_T_MAP_VTABLE);
    e.call(REMOVE_ALL, &args![this]);
    typed_call!(FAMILY, 0x006e_21b0, ni_t_map_base_dtor(e, this));
}

// Typed from 006e21b0
/// The `NiTMapBase` destructor (by the vtable it sets): the base vtable `0106cf88`, removes every
/// item (`00438af0`), then frees the bucket table at `+8` (`00aa10f0`).
pub fn ni_t_map_base_dtor(e: &mut Engine, this: Ptr) {
    e.mem.set_u32(this.addr(), NI_T_MAP_BASE_VTABLE);
    e.call(REMOVE_ALL, &args![this]);
    let table = e.mem.u32(this.addr() + 8);
    e.call(FREE_TABLE, &args![table]);
}

// Typed from 006e21e0
/// The `NiTMapBase` scalar deleting destructor (engine map): the base destructor (`006e21b0`),
/// then `operator delete` when bit 0 of `flags` is set. Answers `this`.
pub fn ni_t_map_base_scalar_deleting_dtor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    typed_call!(FAMILY, 0x006e_21b0, ni_t_map_base_dtor(e, this));
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 006e1f80
/// The smaller of two signed bytes (`b` when equal).
pub fn min_byte(_e: &mut Engine, a: i8, b: i8) -> i8 {
    if a < b {
        a
    } else {
        b
    }
}

// Typed from 006e1fb0
/// The larger of two signed bytes (`b` when equal).
pub fn max_byte(_e: &mut Engine, a: i8, b: i8) -> i8 {
    if a > b {
        a
    } else {
        b
    }
}

// Typed from 006e1fe0
/// Sizes an `NiNode`'s children array (+0x9C) for `count` (`0060bef0`).
pub fn children_set_size(e: &mut Engine, this: Ptr, count: u32) {
    e.call(CHILDREN_SET_SIZE, &args![this.byte_add(CHILDREN), count]);
}
