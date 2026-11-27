//! A package's flag accessors (the unit `run_0067a1b0`): the getters and setters of the bits of
//! its flags (`PackData.iPackFlags`, Xbox PDB, the word at +0x1c) and of its behaviour flags
//! (`PackData.iFOBehaviorFlags`, the half word at +0x22), the copy of all of them from another
//! package, and the package type's entry in the table of pointers at `0119bcb0`.
//!
//! Two flags are stored inverted: the getters of the bits 0x0800_0000 and 0x1000_0000 are true
//! when the bit is clear, and their setters set the bit for a zero byte. A setter takes one stack
//! word and reads its low byte.
//!
//! **Native.** Each accessor is the one line it holds in the exe. The copy (`0067a1b0`) is done
//! as what its 22 getter and setter pairs leave: the copied bits of both flag words (`0x1cfe_3000`
//! of the flags, `0x1ff` of the behaviour flags; the inverted bits are copied as they are, since
//! their getter and setter both invert) and the word at +0x48, with the pairs the exe calls
//! outside this unit done in place, each the body it holds (checked in the exe): the getter of
//! the bit 0x0020_0000 (`00441b00`), the setter of the bit 0x0080_0000 (`00671a20`, this family's)
//! and the word at +0x48's getter and setter (`006733e0`, `00673400`). The pairs read `source`
//! and write `this` in turn, so the copy of one package onto itself leaves it as it is either
//! way. The table's entry reads the package type in place (`0041ca90`: the sign-extended byte at
//! +0x20).
//!
//! No floats.

use engine_core::Engine;

/// The flags (a word), the package type (a signed byte), the behaviour flags (a half word) and
/// the word the copy moves (`pCombatStyle`).
pub(super) const PACK_FLAGS: u32 = 0x1c;
pub(super) const PACK_TYPE: u32 = 0x20;
const BEHAVIOR_FLAGS: u32 = 0x22;
const COMBAT_STYLE: u32 = 0x48;
/// The bits of the flags and of the behaviour flags `0067a1b0` copies.
const COPIED_PACK_FLAGS: u32 = 0x1cfe_3000;
const COPIED_BEHAVIOR_FLAGS: u16 = 0x01ff;
/// The table of pointers by package type.
const PACK_TYPE_TABLE: u32 = 0x0119_bcb0;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

/// Whether `bit` of the flags is set.
pub(super) fn pack_flag(e: &Engine, this: u32, bit: u32) -> bool {
    e.mem.u32(at(this, PACK_FLAGS)) & bit != 0
}

/// `bit` of the flags set (`on`) or cleared.
pub(super) fn set_pack_flag(e: &mut Engine, this: u32, bit: u32, on: bool) {
    let flags = e.mem.u32(at(this, PACK_FLAGS));
    let flags = if on { flags | bit } else { flags & !bit };
    e.mem.set_u32(at(this, PACK_FLAGS), flags);
}

fn behavior_flag(e: &Engine, this: u32, bit: u16) -> bool {
    e.mem.u16(at(this, BEHAVIOR_FLAGS)) & bit != 0
}

fn set_behavior_flag(e: &mut Engine, this: u32, bit: u16, on: bool) {
    let flags = e.mem.u16(at(this, BEHAVIOR_FLAGS));
    let flags = if on { flags | bit } else { flags & !bit };
    e.mem.set_u16(at(this, BEHAVIOR_FLAGS), flags);
}

// Typed from 0067a1b0
/// Every flag the accessors here reach copied from `source` (the copied bits of both flag words),
/// then `source`'s word at +0x48.
pub fn copy_flags_from(e: &mut Engine, this: u32, source: u32) {
    let from = e.mem.u32(at(source, PACK_FLAGS));
    let to = e.mem.u32(at(this, PACK_FLAGS));
    e.mem.set_u32(
        at(this, PACK_FLAGS),
        (to & !COPIED_PACK_FLAGS) | (from & COPIED_PACK_FLAGS),
    );
    let from = e.mem.u16(at(source, BEHAVIOR_FLAGS));
    let to = e.mem.u16(at(this, BEHAVIOR_FLAGS));
    e.mem.set_u16(
        at(this, BEHAVIOR_FLAGS),
        (to & !COPIED_BEHAVIOR_FLAGS) | (from & COPIED_BEHAVIOR_FLAGS),
    );
    let style = e.mem.u32(at(source, COMBAT_STYLE));
    e.mem.set_u32(at(this, COMBAT_STYLE), style);
}

// Typed from 0067a380
/// Whether the flag 0x1000 is set.
pub fn pack_flag_1000(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x1000)
}

// Typed from 0067a3a0
/// The flag 0x1000 set for a byte other than zero, else cleared.
pub fn set_pack_flag_1000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x1000, value != 0);
}

// Typed from 0067a3f0
/// Whether the flag 0x0040_0000 is set.
pub fn pack_flag_400000(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x0040_0000)
}

// Typed from 0067a410
/// The flag 0x0040_0000 set for a byte other than zero, else cleared.
pub fn set_pack_flag_400000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x0040_0000, value != 0);
}

// Typed from 0067a460
/// Whether the flag 0x0080_0000 is set (its setter is `00671a20`).
pub fn pack_flag_800000(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x0080_0000)
}

// Typed from 0067a480
/// Whether the flag 0x2000 is set.
pub fn pack_flag_2000(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x2000)
}

// Typed from 0067a4a0
/// The flag 0x2000 set for a byte other than zero, else cleared.
pub fn set_pack_flag_2000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x2000, value != 0);
}

// Typed from 0067a4f0
/// Whether the flag 0x0002_0000 is set.
pub fn pack_flag_20000(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x0002_0000)
}

// Typed from 0067a510
/// The flag 0x0002_0000 set for a byte other than zero, else cleared.
pub fn set_pack_flag_20000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x0002_0000, value != 0);
}

// Typed from 0067a560
/// Whether the flag 0x0004_0000 is set.
pub fn pack_flag_40000(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x0004_0000)
}

// Typed from 0067a580
/// The flag 0x0004_0000 set for a byte other than zero, else cleared.
pub fn set_pack_flag_40000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x0004_0000, value != 0);
}

// Typed from 0067a5d0
/// Whether the flag 0x0008_0000 is set.
pub fn pack_flag_80000(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x0008_0000)
}

// Typed from 0067a5f0
/// The flag 0x0008_0000 set for a byte other than zero, else cleared.
pub fn set_pack_flag_80000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x0008_0000, value != 0);
}

// Typed from 0067a640
/// The flag 0x0020_0000 set for a byte other than zero, else cleared (its getter is `00441b00`).
pub fn set_pack_flag_200000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x0020_0000, value != 0);
}

// Typed from 0067a690
/// Whether the flag 0x0010_0000 is set.
pub fn pack_flag_100000(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x0010_0000)
}

// Typed from 0067a6b0
/// The flag 0x0010_0000 set for a byte other than zero, else cleared.
pub fn set_pack_flag_100000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x0010_0000, value != 0);
}

// Typed from 0067a700
/// Whether the flag 0x0400_0000 is set.
pub fn pack_flag_4000000(e: &mut Engine, this: u32) -> bool {
    pack_flag(e, this, 0x0400_0000)
}

// Typed from 0067a720
/// The flag 0x0400_0000 set for a byte other than zero, else cleared.
pub fn set_pack_flag_4000000(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x0400_0000, value != 0);
}

// Typed from 0067a770
/// Whether the flag 0x0800_0000 is clear (stored inverted).
pub fn pack_flag_8000000_clear(e: &mut Engine, this: u32) -> bool {
    !pack_flag(e, this, 0x0800_0000)
}

// Typed from 0067a790
/// The flag 0x0800_0000 set for a zero byte, else cleared (stored inverted).
pub fn set_pack_flag_8000000_clear(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x0800_0000, value == 0);
}

// Typed from 0067a7e0
/// Whether the flag 0x1000_0000 is clear (stored inverted).
pub fn pack_flag_10000000_clear(e: &mut Engine, this: u32) -> bool {
    !pack_flag(e, this, 0x1000_0000)
}

// Typed from 0067a800
/// The flag 0x1000_0000 set for a zero byte, else cleared (stored inverted).
pub fn set_pack_flag_10000000_clear(e: &mut Engine, this: u32, value: u8) {
    set_pack_flag(e, this, 0x1000_0000, value == 0);
}

// Typed from 0067a850
/// Whether the behaviour flag 0x1 is set.
pub fn behavior_flag_1(e: &mut Engine, this: u32) -> bool {
    behavior_flag(e, this, 0x1)
}

// Typed from 0067a870
/// The behaviour flag 0x1 set for a byte other than zero, else cleared.
pub fn set_behavior_flag_1(e: &mut Engine, this: u32, value: u8) {
    set_behavior_flag(e, this, 0x1, value != 0);
}

// Typed from 0067a8d0
/// Whether the behaviour flag 0x2 is set.
pub fn behavior_flag_2(e: &mut Engine, this: u32) -> bool {
    behavior_flag(e, this, 0x2)
}

// Typed from 0067a8f0
/// The behaviour flag 0x2 set for a byte other than zero, else cleared.
pub fn set_behavior_flag_2(e: &mut Engine, this: u32, value: u8) {
    set_behavior_flag(e, this, 0x2, value != 0);
}

// Typed from 0067a950
/// Whether the behaviour flag 0x4 is set.
pub fn behavior_flag_4(e: &mut Engine, this: u32) -> bool {
    behavior_flag(e, this, 0x4)
}

// Typed from 0067a970
/// The behaviour flag 0x4 set for a byte other than zero, else cleared.
pub fn set_behavior_flag_4(e: &mut Engine, this: u32, value: u8) {
    set_behavior_flag(e, this, 0x4, value != 0);
}

// Typed from 0067a9d0
/// Whether the behaviour flag 0x8 is set.
pub fn behavior_flag_8(e: &mut Engine, this: u32) -> bool {
    behavior_flag(e, this, 0x8)
}

// Typed from 0067a9f0
/// The behaviour flag 0x8 set for a byte other than zero, else cleared.
pub fn set_behavior_flag_8(e: &mut Engine, this: u32, value: u8) {
    set_behavior_flag(e, this, 0x8, value != 0);
}

// Typed from 0067aa50
/// Whether the behaviour flag 0x10 is set.
pub fn behavior_flag_10(e: &mut Engine, this: u32) -> bool {
    behavior_flag(e, this, 0x10)
}

// Typed from 0067aa70
/// The behaviour flag 0x10 set for a byte other than zero, else cleared.
pub fn set_behavior_flag_10(e: &mut Engine, this: u32, value: u8) {
    set_behavior_flag(e, this, 0x10, value != 0);
}

// Typed from 0067aad0
/// Whether the behaviour flag 0x20 is set.
pub fn behavior_flag_20(e: &mut Engine, this: u32) -> bool {
    behavior_flag(e, this, 0x20)
}

// Typed from 0067aaf0
/// The behaviour flag 0x20 set for a byte other than zero, else cleared.
pub fn set_behavior_flag_20(e: &mut Engine, this: u32, value: u8) {
    set_behavior_flag(e, this, 0x20, value != 0);
}

// Typed from 0067ab50
/// Whether the behaviour flag 0x40 is set.
pub fn behavior_flag_40(e: &mut Engine, this: u32) -> bool {
    behavior_flag(e, this, 0x40)
}

// Typed from 0067ab70
/// The behaviour flag 0x40 set for a byte other than zero, else cleared.
pub fn set_behavior_flag_40(e: &mut Engine, this: u32, value: u8) {
    set_behavior_flag(e, this, 0x40, value != 0);
}

// Typed from 0067abd0
/// Whether the behaviour flag 0x80 is set.
pub fn behavior_flag_80(e: &mut Engine, this: u32) -> bool {
    behavior_flag(e, this, 0x80)
}

// Typed from 0067abf0
/// The behaviour flag 0x80 set for a byte other than zero, else cleared.
pub fn set_behavior_flag_80(e: &mut Engine, this: u32, value: u8) {
    set_behavior_flag(e, this, 0x80, value != 0);
}

// Typed from 0067ac50
/// Whether the behaviour flag 0x100 is set.
pub fn behavior_flag_100(e: &mut Engine, this: u32) -> bool {
    behavior_flag(e, this, 0x100)
}

// Typed from 0067ac70
/// The behaviour flag 0x100 set for a byte other than zero, else cleared.
pub fn set_behavior_flag_100(e: &mut Engine, this: u32, value: u8) {
    set_behavior_flag(e, this, 0x100, value != 0);
}

// Typed from 0067acd0
/// The entry of the table at `0119bcb0` for the package type (the sign-extended byte at +0x20).
pub fn type_table_entry(e: &mut Engine, this: u32) -> u32 {
    let kind = e.mem.i8(at(this, PACK_TYPE)) as i32 as u32;
    e.mem
        .u32(PACK_TYPE_TABLE.wrapping_add(kind.wrapping_mul(4)))
}
