//! The package data classes' small functions (`TESPackageData` and the classes built on it: the
//! dialogue, eat, escort, follow, patrol and use item data, in the units `run_0067ad20` to
//! `run_0067ca10`), the package event action's (`run_0067da20`) and the package location's save
//! swap (`0067eff0`, in `run_0067ec40`), each as the exe does it.
//!
//! A package data object starts with its class's vtable. The dialogue and patrol data load an
//! optional 0xc-byte object at +4 from a save; an event action holds an idle (+0), a script (+4,
//! a `Script` of 0x54 bytes the action owns), a topic (+8) and a word at +0xc.
//!
//! **Native.** The one-line bodies the exe calls are done in place, each the body it holds
//! (checked in the exe): `TESPackageData`'s constructor and destructor (`0067ad20`, `0067ad70`:
//! its vtable stored, this unit's own) and the patrol data's destructor (`0067c490`), the loaded
//! object's constructor (`0067f030`: the byte 3, then two zero words), the event action's getters
//! and setters (`00559450`/`00537e90` +0, `0044ddc0`/`00403550` +8, `0084e3a0`/`0041fd00` +0xc)
//! and the byte swaps (`00401080` and `00407a90`, over the runtime's `_byteswap_ulong` and
//! `_byteswap_ushort`). A new script is marked temporary with this family's typed call. What stays
//! a call by address: the load buffer's read, the allocator and its release, the loaded object's
//! own load, the script's constructor and copy; the script's deleting destructor is called through
//! its slot.
//!
//! The dialogue data's distance (+4) starts at 100.0 and its float at +0x14 at 0.0, the follow
//! data's float at +8 at 0.0: the exe's `FLD`/`FSTP` of its constant 100.0 and its `FLDZ`, stored
//! as the `f32` bits the exe stores.

use super::{tesform, FAMILY};
use engine_core::{args, typed_call, Engine};

const OPERATOR_NEW: u32 = 0x0040_1000;
const OPERATOR_DELETE: u32 = 0x0040_1030;
/// The load buffer's read (`00864980`, `ECX` = the buffer: destination, size).
const LOAD_READ: u32 = 0x0086_4980;
/// The loaded object's own load (`0067fbe0`, `ECX` = the object: the buffer).
const LOADED_OBJECT_LOAD: u32 = 0x0067_fbe0;
const LOADED_OBJECT_SIZE: u32 = 0xc;
/// `Script`'s constructor (`005aa0f0`), its copy from another (`005aa420`) and its size.
const SCRIPT_CONSTRUCT: u32 = 0x005a_a0f0;
const SCRIPT_COPY: u32 = 0x005a_a420;
const SCRIPT_SIZE: u32 = 0x54;
/// A form's deleting destructor slot.
const SLOT_DELETING_DESTRUCTOR: u32 = 0x10;

/// The vtables (by their RTTI).
const PACKAGE_DATA_VTABLE: u32 = 0x0106_8884;
const DIALOGUE_DATA_VTABLE: u32 = 0x0106_88ac;
const EAT_DATA_VTABLE: u32 = 0x0106_8938;
const ESCORT_DATA_VTABLE: u32 = 0x0106_8960;
const FOLLOW_DATA_VTABLE: u32 = 0x0106_8988;
const PATROL_DATA_VTABLE: u32 = 0x0106_89e4;
const USE_ITEM_DATA_VTABLE: u32 = 0x0106_8a0c;
/// The dialogue data's starting distance (100.0) and the escort data's starting word.
const DIALOGUE_DISTANCE: u32 = 0x42c8_0000;
const ESCORT_DISTANCE: u32 = 200;

fn at(this: u32, offset: u32) -> u32 {
    this.wrapping_add(offset)
}

fn swap_word(e: &mut Engine, at: u32) {
    let word = e.mem.u32(at);
    e.mem.set_u32(at, word.swap_bytes());
}

fn swap_half(e: &mut Engine, at: u32) {
    let half = e.mem.u16(at);
    e.mem.set_u16(at, half.swap_bytes());
}

// Typed from 0067ad20
/// `TESPackageData`'s constructor: its vtable. Returns `this`.
pub fn construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, PACKAGE_DATA_VTABLE);
    this
}

// Typed from 0067ad40
/// `TESPackageData`'s deleting destructor: the vtable stored back, and `this` freed when bit 0 of
/// `flags` is set. Returns `this`.
pub fn scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.mem.set_u32(this, PACKAGE_DATA_VTABLE);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 0067ad70
/// `TESPackageData`'s destructor: its vtable stored back.
pub fn destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, PACKAGE_DATA_VTABLE);
}

/// The dialogue and patrol data's load of the optional object at +4: a byte read from the buffer;
/// when it is set, a new object (constructed in place, or none when the allocation fails) stored
/// at +4 and loaded from the buffer.
fn load_object(e: &mut Engine, this: u32, buffer: u32) {
    let present = e.with_stack(4, |e, flag| {
        e.mem.set_u8(flag.addr(), 0);
        e.call(LOAD_READ, &args![buffer, flag, 1u32]);
        e.mem.u8(flag.addr())
    });
    if present == 0 {
        return;
    }
    let object = e.call(OPERATOR_NEW, &args![LOADED_OBJECT_SIZE]).u32();
    if object != 0 {
        e.mem.set_u8(object, 3);
        e.mem.set_u32(at(object, 4), 0);
        e.mem.set_u32(at(object, 8), 0);
    }
    e.mem.set_u32(at(this, 4), object);
    e.call(LOADED_OBJECT_LOAD, &args![object, buffer]);
}

// Typed from 0067b050
/// `TESDialoguePackageData`'s save game load: the optional object at +4.
pub fn dialogue_load_game(e: &mut Engine, this: u32, buffer: u32) {
    load_object(e, this, buffer);
}

// Typed from 0067b100
/// `TESDialoguePackageData`'s chunk swap (its save swaps the chunk with it): the words at +0, +4,
/// +0x14 and +0xc byte-swapped, in that order.
pub fn dialogue_swap_endian(e: &mut Engine, this: u32) {
    for offset in [0, 4, 0x14, 0xc] {
        swap_word(e, at(this, offset));
    }
}

// Typed from 0067b170
/// `TESDialoguePackageData`'s constructor: the base's and its own vtable, the distance 100.0, the
/// words at +8, +0xc and +0x1c and the bytes at +0x10, +0x11 and +0x18 zero, the float at +0x14
/// 0.0. Returns `this`.
pub fn dialogue_construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, DIALOGUE_DATA_VTABLE);
    e.mem.set_u32(at(this, 4), DIALOGUE_DISTANCE);
    e.mem.set_u32(at(this, 8), 0);
    e.mem.set_u32(at(this, 0xc), 0);
    e.mem.set_u8(at(this, 0x10), 0);
    e.mem.set_u8(at(this, 0x11), 0);
    e.mem.set_u8(at(this, 0x18), 0);
    e.mem.set_u32(at(this, 0x1c), 0);
    e.mem.set_u32(at(this, 0x14), 0);
    this
}

// Typed from 0067b950
/// `TESEatPackageData`'s constructor: its vtable and the word at +4 zero. Returns `this`.
pub fn eat_construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, EAT_DATA_VTABLE);
    e.mem.set_u32(at(this, 4), 0);
    this
}

// Typed from 0067bbf0
/// `TESEscortPackageData`'s constructor: its vtable, the word at +4 zero and at +8 200. Returns
/// `this`.
pub fn escort_construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, ESCORT_DATA_VTABLE);
    e.mem.set_u32(at(this, 4), 0);
    e.mem.set_u32(at(this, 8), ESCORT_DISTANCE);
    this
}

// Typed from 0067bfd0
/// `TESFollowPackageData`'s constructor: its vtable, the word at +4 zero and the float at +8 0.0.
/// Returns `this`.
pub fn follow_construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, FOLLOW_DATA_VTABLE);
    e.mem.set_u32(at(this, 4), 0);
    e.mem.set_u32(at(this, 8), 0);
    this
}

// Typed from 0067c330
/// `TESPatrolPackageData`'s save game load: the word at +8 (4 bytes from the buffer), then the
/// optional object at +4.
pub fn patrol_load_game(e: &mut Engine, this: u32, buffer: u32) {
    e.call(LOAD_READ, &args![buffer, at(this, 8), 4u32]);
    load_object(e, this, buffer);
}

// Typed from 0067c3f0
/// `TESPatrolPackageData`'s constructor: its vtable, the byte at +4 1 and at +5 0. Returns `this`.
pub fn patrol_construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, PATROL_DATA_VTABLE);
    e.mem.set_u8(at(this, 4), 1);
    e.mem.set_u8(at(this, 5), 0);
    this
}

// Typed from 0067c420
/// `TESPatrolPackageData`'s deleting destructor: the destructor's vtables (its own, then the
/// base's), and `this` freed when bit 0 of `flags` is set. Returns `this`.
pub fn patrol_scalar_deleting_destructor(e: &mut Engine, this: u32, flags: u32) -> u32 {
    e.mem.set_u32(this, PACKAGE_DATA_VTABLE);
    if flags & 1 != 0 {
        e.call(OPERATOR_DELETE, &args![this]);
    }
    this
}

// Typed from 0067c450
/// `TESPatrolPackageData`'s constructor from its two bytes (the stack word's low two): its vtable
/// and the bytes at +4 and +5. Returns `this`.
pub fn patrol_construct_from(e: &mut Engine, this: u32, value: u32) -> u32 {
    e.mem.set_u32(this, PATROL_DATA_VTABLE);
    let [low, high, _, _] = value.to_le_bytes();
    e.mem.set_u8(at(this, 4), low);
    e.mem.set_u8(at(this, 5), high);
    this
}

// Typed from 0067c490
/// `TESPatrolPackageData`'s destructor: its vtable, then the base's (`0067ad70`), which is the one
/// that stays.
pub fn patrol_destruct(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, PACKAGE_DATA_VTABLE);
}

// Typed from 0067c770
/// `TESUseItemPackageData`'s constructor: its vtable and the word at +4 zero. Returns `this`.
pub fn use_item_construct(e: &mut Engine, this: u32) -> u32 {
    e.mem.set_u32(this, USE_ITEM_DATA_VTABLE);
    e.mem.set_u32(at(this, 4), 0);
    this
}

// Typed from 0067ca10
/// `TESUseWeaponPackageData`'s chunk swap (its save swaps the chunk with it): the halves at +6, +8
/// and +0xa, then the words at +0xc and +0x10, byte-swapped.
pub fn use_weapon_swap_endian(e: &mut Engine, this: u32) {
    for offset in [6, 8, 0xa] {
        swap_half(e, at(this, offset));
    }
    for offset in [0xc, 0x10] {
        swap_word(e, at(this, offset));
    }
}

// Typed from 0067da20
/// A package data slot (in the use weapon data's vtable) that clears the word at +4; the stack
/// word is not read.
pub fn clear_word_4(e: &mut Engine, this: u32, _unused: u32) {
    e.mem.set_u32(at(this, 4), 0);
}

// Typed from 0067da40
/// The event action's constructor (`TESPackage`'s constructor makes three): its four words zero,
/// +0, +8, +0xc, then +4. Returns `this`.
pub fn event_action_construct(e: &mut Engine, this: u32) -> u32 {
    for offset in [0, 8, 0xc, 4] {
        e.mem.set_u32(at(this, offset), 0);
    }
    this
}

// Typed from 0067da80
/// The event action's clear: the idle and the topic zero and its script deleted (its slot, with
/// 1) when there is one; the script's word itself is left as it is.
pub fn event_action_clear(e: &mut Engine, this: u32) {
    e.mem.set_u32(this, 0);
    e.mem.set_u32(at(this, 8), 0);
    let script = e.mem.u32(at(this, 4));
    if script != 0 {
        e.vcall(script, SLOT_DELETING_DESTRUCTOR, &args![1u32]);
    }
}

// Typed from 0067dad0
/// The event action's copy from `other`: the idle, the topic and the word at +0xc; then, when
/// `other` has a script, this one's own (a new temporary script when it has none, or none when
/// the allocation fails) copied from it, and when `other` has none, this one's deleted (its slot,
/// with 1) and cleared.
pub fn event_action_copy(e: &mut Engine, this: u32, other: u32) {
    for offset in [0, 8, 0xc] {
        let word = e.mem.u32(at(other, offset));
        e.mem.set_u32(at(this, offset), word);
    }
    let theirs = e.mem.u32(at(other, 4));
    if theirs == 0 {
        let mine = e.mem.u32(at(this, 4));
        if mine != 0 {
            e.vcall(mine, SLOT_DELETING_DESTRUCTOR, &args![1u32]);
        }
        e.mem.set_u32(at(this, 4), 0);
        return;
    }
    let mut mine = e.mem.u32(at(this, 4));
    if mine == 0 {
        let block = e.call(OPERATOR_NEW, &args![SCRIPT_SIZE]).u32();
        mine = if block != 0 {
            e.call(SCRIPT_CONSTRUCT, &args![block]).u32()
        } else {
            0
        };
        e.mem.set_u32(at(this, 4), mine);
        typed_call!(FAMILY, 0x0048_4490, tesform::set_temporary(e, mine));
    }
    let theirs = e.mem.u32(at(other, 4));
    e.call(SCRIPT_COPY, &args![mine, theirs]);
}

// Typed from 0067eff0
/// `PackageLocation`'s chunk swap (its save swaps the chunk with it): the words at +4 and +8
/// byte-swapped.
pub fn location_swap_endian(e: &mut Engine, this: u32) {
    for offset in [4, 8] {
        swap_word(e, at(this, offset));
    }
}
