//! `fallout shared/modelloader.cpp` (Xbox PDB source unit), part 4: its functions from `00449c00` up to
//! (not including) `ffffffff` in FalloutNV.exe 1.4.0.525, translated
//! (docs/ENGINE_CRATE.md). The unit's shared layouts and helpers are in
//! [`super::modelloader`]; anything public there may be used here.
//!
//! State of this file: the first 40 functions of the range, `00449c00` to
//! `0044b2c0`. They are the compiler-emitted glue of the loader's queues and
//! reference-counted handles: the destructor and constructors of the
//! `LockFreeMap` instances, `BSTaskManager<__int64>::CancelTask`, the
//! one-bit flag accessors of the queued files (`cFlags` bits 0 to 5), and
//! the smart pointers of `Model`, `LoadedFile` and two other counted
//! objects. The next function of the range is `0044b350`.
//!
//! Not translated: the compiler's exception-unwinding frames (none of these
//! functions has one).

#[allow(unused_imports)]
use super::modelloader::*;
#[allow(unused_imports)]
use crate::prelude::*;

/// `MemoryManager` deallocation, `__cdecl(block)` (`00401030`).
const MEMORY_FREE: u32 = 0x0040_1030;
/// `InterlockedIncrement` wrapper, `__cdecl(target) -> new value` (`0040b460`).
const INTERLOCKED_INCREMENT: u32 = 0x0040_b460;
/// `InterlockedDecrement` wrapper, `__cdecl(target) -> new value` (`004019a0`).
const INTERLOCKED_DECREMENT: u32 = 0x0040_19a0;
/// `Sleep` wrapper, `__cdecl(milliseconds)` (`0040fca0`).
const SLEEP: u32 = 0x0040_fca0;
/// `NiMemObject` deallocation, `__cdecl(block, size)` (`00aa1460`).
const NI_FREE: u32 = 0x00aa_1460;

/// The `LockFreeMapIterator` base destructor body (`00449a00`): sets the
/// base class's virtual table.
const LOCK_FREE_MAP_ITERATOR_BASE_DESTRUCT: u32 = 0x0044_9a00;
/// The three `LockFreeMap` destructor bodies the wrappers `00449c30`,
/// `00449d40` and `00449ea0` forward to.
const LOCK_FREE_MAP_DESTRUCT_00449C50: u32 = 0x0044_9c50;
const LOCK_FREE_MAP_DESTRUCT_00449D60: u32 = 0x0044_9d60;
const LOCK_FREE_MAP_DESTRUCT_00449EC0: u32 = 0x0044_9ec0;
/// The three `LockFreeMap` base constructors (`__thiscall(a, b, c)`) and the
/// virtual tables the instance constructors store after them.
const LOCK_FREE_MAP_CONSTRUCT_0044C040: u32 = 0x0044_c040;
const LOCK_FREE_MAP_CONSTRUCT_0044C270: u32 = 0x0044_c270;
const LOCK_FREE_MAP_CONSTRUCT_0044C920: u32 = 0x0044_c920;
const LOCK_FREE_MAP_VTABLE_00449F50: u32 = 0x0101_72dc;
const LOCK_FREE_MAP_VTABLE_00449FA0: u32 = 0x0101_732c;
const LOCK_FREE_MAP_VTABLE_0044A100: u32 = 0x0101_737c;
/// The routine `0044a130` hands the result of its virtual call to,
/// `__thiscall(this, value, a, b, c) -> byte` (`RET 0x10`).
const LOCK_FREE_MAP_HANDLE: u32 = 0x0044_d240;
/// The byte global `0044ada0` writes (`01202df0`).
const TASK_MANAGER_FLAG: u32 = 0x0120_2df0;
/// Tries to move a task's state (+0xC) to `6`, `__thiscall(task, state,
/// 6) -> bool` (`00449190`; its body is a compare-and-exchange on the state).
const TASK_TRY_SET_STATE: u32 = 0x0044_9190;
/// Adds a reference to a task, `__thiscall(task)` (`0092c870`: increments
/// the counter at +8).
const TASK_ADD_REFERENCE: u32 = 0x0092_c870;
/// `__thiscall(manager, task)` (`0044e050`): takes the task out of the
/// manager's table.
const TASK_MANAGER_REMOVE: u32 = 0x0044_e050;
/// `__thiscall(task)` (`0044dd60`): drops a reference of the task and
/// deletes it (virtual destructor, flag 1) when none is left.
const TASK_RELEASE: u32 = 0x0044_dd60;
/// Slot (byte offset) of the virtual function `CancelTask` calls on a task
/// with `(state, argument)`.
const TASK_CANCEL_SLOT: u32 = 0xc;
/// The state `CancelTask` moves a task to.
const TASK_STATE_CANCELLED: u32 = 6;
/// `__thiscall(element)` (`00442600`), run on every element of the array of
/// `0044adb0`.
const ARRAY_ELEMENT_ACTION: u32 = 0x0044_2600;
/// The stream's binary output, `__thiscall(stream) -> output` (`008d8b00`:
/// the dword at +0x250), and the routine that writes `count` four-byte
/// values to it, `__cdecl(output, values, count)` (`0044e0f0`).
const STREAM_OUTPUT: u32 = 0x008d_8b00;
const WRITE_BINARY: u32 = 0x0044_e0f0;
/// The release and add-reference calls of `Model`'s pointer type
/// (`__thiscall(model)`): `0040c130` and `0040f6e0`.
const MODEL_RELEASE: u32 = 0x0040_c130;
const MODEL_ADD_REFERENCE: u32 = 0x0040_f6e0;
/// The critical section (`011c3b6c`, the address is the object) that guards
/// the reference counter of a `LoadedFile`; entered with the import
/// `TryEnterCriticalSection` (slot `00fdf1b0`) and left with `0082f1f0`.
const LOADED_FILE_LOCK: u32 = 0x011c_3b6c;
const TRY_ENTER_CRITICAL_SECTION: u32 = 0x00fd_f1b0;
const LEAVE_CRITICAL_SECTION: u32 = 0x0082_f1f0;
/// `LoadedFile::~LoadedFile` (Xbox PDB, `0043bb80`).
const LOADED_FILE_DESTRUCT: u32 = 0x0043_bb80;
/// Size of a `LoadedFile` block (`0x10`), as `NiMemObject` frees it.
const LOADED_FILE_SIZE: u32 = 0x10;
/// `__thiscall(object)` (`0044b2e0`): the release of an object counted at
/// +0x20.
const RELEASE_COUNTED_AT_0X20: u32 = 0x0044_b2e0;

/// Sets (`value` non-zero) or clears `mask` in the flag byte at `flags`.
fn write_flag_bit(e: &mut Engine, value: u8, flags: Ptr, mask: u8) {
    let byte = e.mem.u8(flags.addr());
    let byte = if value != 0 {
        byte | mask
    } else {
        byte & !mask
    };
    e.mem.set_u8(flags.addr(), byte);
}

// Translated from 00449c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LockFreeMap<TESObjectREFR*,NiPointer<QueuedHelmet>>::LockFreeMapIterator::
/// scalar deleting destructor` (Xbox PDB): runs the base destructor body
/// and, when bit 0 of `flags` is set, frees the block; returns `this`.
pub fn lock_free_map_iterator_queued_helmet_scalar_deleting_destructor(
    e: &mut Engine,
    this: Ptr,
    flags: u32,
) -> Ptr {
    e.call(LOCK_FREE_MAP_ITERATOR_BASE_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(MEMORY_FREE, &args![this]);
    }
    this
}

// Translated from 00449c30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of a `LockFreeMap` instance (its scalar deleting destructor is
/// `00449a50`): forwards to the body at `00449c50`.
pub fn fn_00449c30(e: &mut Engine, this: Ptr) {
    e.call(LOCK_FREE_MAP_DESTRUCT_00449C50, &args![this]);
}

// Translated from 00449d40 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of a `LockFreeMap` instance: forwards to the body at
/// `00449d60`.
pub fn fn_00449d40(e: &mut Engine, this: Ptr) {
    e.call(LOCK_FREE_MAP_DESTRUCT_00449D60, &args![this]);
}

// Translated from 00449ea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Destructor of a `LockFreeMap` instance: forwards to the body at
/// `00449ec0`.
pub fn fn_00449ea0(e: &mut Engine, this: Ptr) {
    e.call(LOCK_FREE_MAP_DESTRUCT_00449EC0, &args![this]);
}

// Translated from 00449f50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of a `LockFreeMap` instance: the base constructor
/// `0044c040(a, b, c)`, then the instance's virtual table (`010172dc`);
/// returns `this`.
pub fn fn_00449f50(e: &mut Engine, this: Ptr, a: u32, b: u32, c: u32) -> Ptr {
    e.call(LOCK_FREE_MAP_CONSTRUCT_0044C040, &args![this, a, b, c]);
    e.mem.set_u32(this.addr(), LOCK_FREE_MAP_VTABLE_00449F50);
    this
}

// Translated from 00449fa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of a `LockFreeMap` instance: the base constructor
/// `0044c270(a, b, c)`, then the virtual table `0101732c`; returns `this`.
pub fn fn_00449fa0(e: &mut Engine, this: Ptr, a: u32, b: u32, c: u32) -> Ptr {
    e.call(LOCK_FREE_MAP_CONSTRUCT_0044C270, &args![this, a, b, c]);
    e.mem.set_u32(this.addr(), LOCK_FREE_MAP_VTABLE_00449FA0);
    this
}

// Translated from 0044a100 (decompiled, FalloutNV.exe 1.4.0.525)
/// Constructor of a `LockFreeMap` instance: the base constructor
/// `0044c920(a, b, c)`, then the virtual table `0101737c`; returns `this`.
pub fn fn_0044a100(e: &mut Engine, this: Ptr, a: u32, b: u32, c: u32) -> Ptr {
    e.call(LOCK_FREE_MAP_CONSTRUCT_0044C920, &args![this, a, b, c]);
    e.mem.set_u32(this.addr(), LOCK_FREE_MAP_VTABLE_0044A100);
    this
}

// Translated from 0044a130 (decompiled, FalloutNV.exe 1.4.0.525)
/// Calls slot `0x24` of the object the first word of `this` points to
/// (with `a`), then `0044d240(this, result, a, b, c)`; returns the byte
/// that routine returns. The decompiler shows the arguments of the second
/// call shifted; the disassembly pushes `c`, `b`, `a`, and `a` again for
/// the virtual call, which takes one word.
pub fn fn_0044a130(e: &mut Engine, this: Ptr, a: u32, b: u32, c: u8) -> u8 {
    let owner = e.mem.u32(this.addr());
    let result = e.vcall(owner, 0x24, &args![a]).u32();
    e.call(LOCK_FREE_MAP_HANDLE, &args![this, result, a, b, c as u32])
        .u32() as u8
}

// Translated from 0044ac40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `BSTaskManager<__int64>::CancelTask` (Xbox PDB): cancels `task`
/// whatever state (+0xC) it is in, then returns.
///
/// First clears the byte at `01202df0` ([`fn_0044ada0`]). Then, repeating:
/// states 0 and 5 move the task to state 6 (`00449190`) and, when that
/// succeeds, call the task's slot `0xC` with `(state, argument)`; state 1
/// does the same after taking a reference and removing the task from the
/// manager (`0044e050`), and releases it afterwards (`0044dd60`); states 3
/// and 4 sleep (`Sleep(0)`) while the task stays in state 3 or 4; state 2
/// goes round again at once (the game's loop there tests for state 3, as
/// state 3's does); any other state returns. A failed move simply reads the
/// state again.
pub fn bs_task_manager_cancel_task(e: &mut Engine, this: Ptr, task: Ptr, argument: u32) {
    fn_0044ada0(e, 0);
    loop {
        let state = e.mem.u32(task.addr() + 0xc);
        match state {
            0 | 5 => {
                if try_set_cancelled(e, task, state) {
                    e.vcall(task.addr(), TASK_CANCEL_SLOT, &args![state, argument]);
                    return;
                }
            }
            1 => {
                if try_set_cancelled(e, task, state) {
                    e.call(TASK_ADD_REFERENCE, &args![task]);
                    e.call(TASK_MANAGER_REMOVE, &args![this, task]);
                    e.vcall(task.addr(), TASK_CANCEL_SLOT, &args![state, argument]);
                    e.call(TASK_RELEASE, &args![task]);
                    return;
                }
            }
            2 | 3 => {
                while e.mem.u32(task.addr() + 0xc) == 3 {
                    e.call(SLEEP, &args![0u32]);
                }
            }
            4 => {
                while e.mem.u32(task.addr() + 0xc) == 4 {
                    e.call(SLEEP, &args![0u32]);
                }
            }
            _ => return,
        }
    }
}

/// `00449190(task, state, 6)`: tries to move the task from `state` to the
/// cancelled state.
fn try_set_cancelled(e: &mut Engine, task: Ptr, state: u32) -> bool {
    e.call(
        TASK_TRY_SET_STATE,
        &args![task, state, TASK_STATE_CANCELLED],
    )
    .bool()
}

// Translated from 0044ada0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Stores the byte `value` in the global at `01202df0`.
pub fn fn_0044ada0(e: &mut Engine, value: u8) {
    e.set_global(TASK_MANAGER_FLAG, value);
}

// Translated from 0044adb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Runs `00442600` on each element of the pointer array at +0x50 (count at
/// +0x4C; the count is read again before every element).
pub fn fn_0044adb0(e: &mut Engine, this: Ptr) {
    let mut index = 0u32;
    while index < e.mem.u32(this.addr() + 0x4c) {
        let array = e.mem.u32(this.addr() + 0x50);
        let element = e.mem.u32(array + index * 4);
        e.call(ARRAY_ELEMENT_ACTION, &args![element]);
        index += 1;
    }
}

// Translated from 0044adf0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiStreamSaveBinary<unsigned int>` (Xbox PDB): writes the four-byte value
/// at `value` to the stream's binary output (the dword at stream +0x250,
/// `008d8b00`) as one element (`0044e0f0(output, value, 1)`).
pub fn ni_stream_save_binary_u32(e: &mut Engine, stream: Ptr, value: Ptr) {
    let output = e.call(STREAM_OUTPUT, &args![stream]).u32();
    e.call(WRITE_BINARY, &args![output, value, 1u32]);
}

// Translated from 0044ae10 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 0 (`0x01`) of the flag byte `flags` is set.
pub fn fn_0044ae10(_e: &mut Engine, flags: u8) -> bool {
    flags & 0x01 != 0
}

// Translated from 0044ae20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`value` non-zero) or clears bit 0 (`0x01`) of the flag byte at
/// `flags`.
pub fn fn_0044ae20(e: &mut Engine, value: u8, flags: Ptr) {
    write_flag_bit(e, value, flags, 0x01);
}

// Translated from 0044ae50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 1 (`0x02`) of the flag byte `flags` is set.
pub fn fn_0044ae50(_e: &mut Engine, flags: u8) -> bool {
    flags & 0x02 != 0
}

// Translated from 0044ae60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`value` non-zero) or clears bit 1 (`0x02`) of the flag byte at
/// `flags`.
pub fn fn_0044ae60(e: &mut Engine, value: u8, flags: Ptr) {
    write_flag_bit(e, value, flags, 0x02);
}

// Translated from 0044ae90 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 2 (`0x04`) of the flag byte `flags` is set.
pub fn fn_0044ae90(_e: &mut Engine, flags: u8) -> bool {
    flags & 0x04 != 0
}

// Translated from 0044aea0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`value` non-zero) or clears bit 2 (`0x04`) of the flag byte at
/// `flags`.
pub fn fn_0044aea0(e: &mut Engine, value: u8, flags: Ptr) {
    write_flag_bit(e, value, flags, 0x04);
}

// Translated from 0044aed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `NiPointer<Model>::operator=(Model*)` (Xbox PDB name
/// `NiPointer<Model>::operator_`): when `model` differs from the pointer
/// stored at `this`, releases the old model (`0040c130`), stores the new one
/// and adds a reference to it (`0040f6e0`); returns `this`.
pub fn ni_pointer_model_assign(e: &mut Engine, this: Ptr, model: Ptr<Model>) -> Ptr {
    if e.mem.u32(this.addr()) != model.addr() {
        let old = e.mem.u32(this.addr());
        if old != 0 {
            e.call(MODEL_RELEASE, &args![old]);
        }
        e.mem.set_u32(this.addr(), model.addr());
        let new = e.mem.u32(this.addr());
        if new != 0 {
            e.call(MODEL_ADD_REFERENCE, &args![new]);
        }
    }
    this
}

// Translated from 0044af20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 3 (`0x08`) of the flag byte `flags` is set.
pub fn fn_0044af20(_e: &mut Engine, flags: u8) -> bool {
    flags & 0x08 != 0
}

// Translated from 0044af30 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`value` non-zero) or clears bit 3 (`0x08`) of the flag byte at
/// `flags`.
pub fn fn_0044af30(e: &mut Engine, value: u8, flags: Ptr) {
    write_flag_bit(e, value, flags, 0x08);
}

// Translated from 0044af60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 4 (`0x10`) of the flag byte `flags` is set.
pub fn fn_0044af60(_e: &mut Engine, flags: u8) -> bool {
    flags & 0x10 != 0
}

// Translated from 0044af70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`value` non-zero) or clears bit 4 (`0x10`) of the flag byte at
/// `flags`.
pub fn fn_0044af70(e: &mut Engine, value: u8, flags: Ptr) {
    write_flag_bit(e, value, flags, 0x10);
}

// Translated from 0044afa0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Whether bit 5 (`0x20`) of the flag byte `flags` is set.
pub fn fn_0044afa0(_e: &mut Engine, flags: u8) -> bool {
    flags & 0x20 != 0
}

// Translated from 0044afb0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Sets (`value` non-zero) or clears bit 5 (`0x20`) of the flag byte at
/// `flags`.
pub fn fn_0044afb0(e: &mut Engine, value: u8, flags: Ptr) {
    write_flag_bit(e, value, flags, 0x20);
}

// Translated from 0044afe0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer constructor for an object counted at +0xC (`QueuedKF`'s
/// constructor uses it): stores `object` at `this` and, when it is not null,
/// adds a reference ([`fn_0044b010`]); returns `this`.
pub fn fn_0044afe0(e: &mut Engine, this: Ptr, object: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), object.addr());
    if e.mem.u32(this.addr()) != 0 {
        let held = Ptr::new(e.mem.u32(this.addr()));
        fn_0044b010(e, held);
    }
    this
}

// Translated from 0044b010 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds a reference: interlocked increment of the counter at +0xC.
pub fn fn_0044b010(e: &mut Engine, this: Ptr) {
    e.call(INTERLOCKED_INCREMENT, &args![this.byte_add(0xc)]);
}

// Translated from 0044b030 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer destructor for the object counted at +0xC: when the
/// pointer at `this` is not null, drops a reference ([`fn_0044b050`]). The
/// object is not deleted here.
pub fn fn_0044b030(e: &mut Engine, this: Ptr) {
    let held = e.mem.u32(this.addr());
    if held != 0 {
        fn_0044b050(e, Ptr::new(held));
    }
}

// Translated from 0044b050 (decompiled, FalloutNV.exe 1.4.0.525)
/// Drops a reference: interlocked decrement of the counter at +0xC.
pub fn fn_0044b050(e: &mut Engine, this: Ptr) {
    e.call(INTERLOCKED_DECREMENT, &args![this.byte_add(0xc)]);
}

// Translated from 0044b070 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer assignment for the object counted at +0xC: when `object`
/// differs from the pointer stored at `this`, drops a reference of the old
/// one, stores the new one and adds a reference to it; returns `this`.
pub fn fn_0044b070(e: &mut Engine, this: Ptr, object: Ptr) -> Ptr {
    if e.mem.u32(this.addr()) != object.addr() {
        let old = e.mem.u32(this.addr());
        if old != 0 {
            fn_0044b050(e, Ptr::new(old));
        }
        e.mem.set_u32(this.addr(), object.addr());
        let new = e.mem.u32(this.addr());
        if new != 0 {
            fn_0044b010(e, Ptr::new(new));
        }
    }
    this
}

// Translated from 0044b0c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer constructor for a `LoadedFile`: stores `file` at `this`
/// and, when it is not null, adds a reference ([`fn_0044b0f0`]); returns
/// `this`.
pub fn fn_0044b0c0(e: &mut Engine, this: Ptr, file: Ptr<LoadedFile>) -> Ptr {
    e.mem.set_u32(this.addr(), file.addr());
    if e.mem.u32(this.addr()) != 0 {
        let held = Ptr::new(e.mem.u32(this.addr()));
        fn_0044b0f0(e, held);
    }
    this
}

// Translated from 0044b0f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds a reference to a `LoadedFile`: waits until the critical section
/// `011c3b6c` can be entered ([`fn_0044b130`]), increments the counter at
/// +0 with an interlocked operation and leaves the section.
pub fn fn_0044b0f0(e: &mut Engine, this: Ptr) {
    while !fn_0044b130(e, Ptr::new(LOADED_FILE_LOCK)) {}
    e.call(INTERLOCKED_INCREMENT, &args![this]);
    e.call(
        LEAVE_CRITICAL_SECTION,
        &args![Ptr::<()>::new(LOADED_FILE_LOCK)],
    );
}

// Translated from 0044b130 (decompiled, FalloutNV.exe 1.4.0.525)
/// `TryEnterCriticalSection(section)` through the import (slot `00fdf1b0`),
/// as a bool.
pub fn fn_0044b130(e: &mut Engine, section: Ptr) -> bool {
    e.call(TRY_ENTER_CRITICAL_SECTION, &args![section]).i32() != 0
}

// Translated from 0044b160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer destructor for a `LoadedFile`: when the pointer at `this`
/// is not null, releases a reference ([`fn_0044b180`]).
pub fn fn_0044b160(e: &mut Engine, this: Ptr) {
    let held = e.mem.u32(this.addr());
    if held != 0 {
        fn_0044b180(e, Ptr::new(held));
    }
}

// Translated from 0044b180 (decompiled, FalloutNV.exe 1.4.0.525)
/// Releases a reference to a `LoadedFile`: waits for the critical section
/// `011c3b6c`, decrements the counter at +0 with an interlocked operation,
/// leaves the section, and when the counter reads 0 deletes the file with
/// its scalar deleting destructor ([`fn_0044b1f0`], flag 1).
pub fn fn_0044b180(e: &mut Engine, this: Ptr) {
    while !fn_0044b130(e, Ptr::new(LOADED_FILE_LOCK)) {}
    e.call(INTERLOCKED_DECREMENT, &args![this]);
    e.call(
        LEAVE_CRITICAL_SECTION,
        &args![Ptr::<()>::new(LOADED_FILE_LOCK)],
    );
    if e.mem.u32(this.addr()) == 0 {
        fn_0044b1f0(e, this, 1);
    }
}

// Translated from 0044b1f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `LoadedFile` scalar deleting destructor: `LoadedFile::~LoadedFile`
/// (`0043bb80`), then, when bit 0 of `flags` is set, frees the block
/// (`00aa1460`, size `0x10`); returns `this`.
pub fn fn_0044b1f0(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(LOADED_FILE_DESTRUCT, &args![this]);
    if flags & 1 != 0 {
        e.call(NI_FREE, &args![this, LOADED_FILE_SIZE]);
    }
    this
}

// Translated from 0044b220 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer assignment for a `LoadedFile`: when `file` differs from the
/// pointer stored at `this`, releases the old one ([`fn_0044b180`]), stores
/// the new one and adds a reference to it ([`fn_0044b0f0`]); returns `this`.
pub fn fn_0044b220(e: &mut Engine, this: Ptr, file: Ptr<LoadedFile>) -> Ptr {
    if e.mem.u32(this.addr()) != file.addr() {
        let old = e.mem.u32(this.addr());
        if old != 0 {
            fn_0044b180(e, Ptr::new(old));
        }
        e.mem.set_u32(this.addr(), file.addr());
        let new = e.mem.u32(this.addr());
        if new != 0 {
            fn_0044b0f0(e, Ptr::new(new));
        }
    }
    this
}

// Translated from 0044b270 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer constructor for an object counted at +0x20 (the face-gen
/// file queueing functions use it): stores `object` at `this` and, when it
/// is not null, adds a reference ([`fn_0044b2a0`]); returns `this`.
pub fn fn_0044b270(e: &mut Engine, this: Ptr, object: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), object.addr());
    if e.mem.u32(this.addr()) != 0 {
        let held = Ptr::new(e.mem.u32(this.addr()));
        fn_0044b2a0(e, held);
    }
    this
}

// Translated from 0044b2a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Adds a reference: interlocked increment of the counter at +0x20.
pub fn fn_0044b2a0(e: &mut Engine, this: Ptr) {
    e.call(INTERLOCKED_INCREMENT, &args![this.byte_add(0x20)]);
}

// Translated from 0044b2c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Smart-pointer destructor for the object counted at +0x20: when the
/// pointer at `this` is not null, calls `0044b2e0` on it (the release).
pub fn fn_0044b2c0(e: &mut Engine, this: Ptr) {
    let held = e.mem.u32(this.addr());
    if held != 0 {
        e.call(RELEASE_COUNTED_AT_0X20, &args![held]);
    }
}

/// This part's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x00449c00,
            lock_free_map_iterator_queued_helmet_scalar_deleting_destructor(Ptr, u32) -> Ptr
        ),
        entry!(0x00449c30, fn_00449c30(Ptr)),
        entry!(0x00449d40, fn_00449d40(Ptr)),
        entry!(0x00449ea0, fn_00449ea0(Ptr)),
        entry!(0x00449f50, fn_00449f50(Ptr, u32, u32, u32) -> Ptr),
        entry!(0x00449fa0, fn_00449fa0(Ptr, u32, u32, u32) -> Ptr),
        entry!(0x0044a100, fn_0044a100(Ptr, u32, u32, u32) -> Ptr),
        entry!(0x0044a130, fn_0044a130(Ptr, u32, u32, u8) -> u8),
        entry!(0x0044ac40, bs_task_manager_cancel_task(Ptr, Ptr, u32)),
        entry!(0x0044ada0, fn_0044ada0(u8)),
        entry!(0x0044adb0, fn_0044adb0(Ptr)),
        entry!(0x0044adf0, ni_stream_save_binary_u32(Ptr, Ptr)),
        entry!(0x0044ae10, fn_0044ae10(u8) -> bool),
        entry!(0x0044ae20, fn_0044ae20(u8, Ptr)),
        entry!(0x0044ae50, fn_0044ae50(u8) -> bool),
        entry!(0x0044ae60, fn_0044ae60(u8, Ptr)),
        entry!(0x0044ae90, fn_0044ae90(u8) -> bool),
        entry!(0x0044aea0, fn_0044aea0(u8, Ptr)),
        entry!(0x0044aed0, ni_pointer_model_assign(Ptr, Ptr<Model>) -> Ptr),
        entry!(0x0044af20, fn_0044af20(u8) -> bool),
        entry!(0x0044af30, fn_0044af30(u8, Ptr)),
        entry!(0x0044af60, fn_0044af60(u8) -> bool),
        entry!(0x0044af70, fn_0044af70(u8, Ptr)),
        entry!(0x0044afa0, fn_0044afa0(u8) -> bool),
        entry!(0x0044afb0, fn_0044afb0(u8, Ptr)),
        entry!(0x0044afe0, fn_0044afe0(Ptr, Ptr) -> Ptr),
        entry!(0x0044b010, fn_0044b010(Ptr)),
        entry!(0x0044b030, fn_0044b030(Ptr)),
        entry!(0x0044b050, fn_0044b050(Ptr)),
        entry!(0x0044b070, fn_0044b070(Ptr, Ptr) -> Ptr),
        entry!(0x0044b0c0, fn_0044b0c0(Ptr, Ptr<LoadedFile>) -> Ptr),
        entry!(0x0044b0f0, fn_0044b0f0(Ptr)),
        entry!(0x0044b130, fn_0044b130(Ptr) -> bool),
        entry!(0x0044b160, fn_0044b160(Ptr)),
        entry!(0x0044b180, fn_0044b180(Ptr)),
        entry!(0x0044b1f0, fn_0044b1f0(Ptr, u32) -> Ptr),
        entry!(0x0044b220, fn_0044b220(Ptr, Ptr<LoadedFile>) -> Ptr),
        entry!(0x0044b270, fn_0044b270(Ptr, Ptr) -> Ptr),
        entry!(0x0044b2a0, fn_0044b2a0(Ptr)),
        entry!(0x0044b2c0, fn_0044b2c0(Ptr)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The argument lists of the calls made to `addr` since `call_log` was
    /// switched on.
    fn calls_to(e: &Engine, addr: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .filter(|(a, _)| *a == addr)
            .map(|(_, args)| args.clone())
            .collect()
    }

    /// The addresses called, in order (the top-level call comes first).
    fn call_order(e: &Engine) -> Vec<u32> {
        e.call_log
            .as_ref()
            .unwrap()
            .iter()
            .map(|(addr, _)| *addr)
            .collect()
    }

    /// An engine with the pages of the byte global and of the lock mapped,
    /// and a zeroed block of `size` bytes.
    fn engine_with_block(size: u32) -> (Engine, Ptr) {
        let mut e = Engine::new();
        e.map(0x0120_2000, 0x1000);
        e.map(0x011c_3000, 0x1000);
        let block = Ptr::new(e.mem.alloc(size));
        (e, block)
    }

    /// Registers a double that does nothing at each address.
    fn ignore(e: &mut Engine, addrs: &[u32]) {
        for addr in addrs {
            e.register(*addr, |_, _| Ret::default());
        }
    }

    #[test]
    fn iterator_destructor_frees_only_with_flag_bit_0() {
        let (mut e, this) = engine_with_block(8);
        ignore(&mut e, &[LOCK_FREE_MAP_ITERATOR_BASE_DESTRUCT, MEMORY_FREE]);
        e.call_log = Some(vec![]);
        let back = e.call(0x0044_9c00, &args![this, 0u32]).ptr::<()>();
        assert_eq!(back, this);
        assert_eq!(
            calls_to(&e, LOCK_FREE_MAP_ITERATOR_BASE_DESTRUCT),
            vec![vec![this.addr()]]
        );
        assert!(calls_to(&e, MEMORY_FREE).is_empty());
        e.call(0x0044_9c00, &args![this, 3u32]);
        assert_eq!(calls_to(&e, MEMORY_FREE), vec![vec![this.addr()]]);
    }

    #[test]
    fn map_destructor_wrappers_forward_this() {
        let (mut e, this) = engine_with_block(8);
        let bodies = [
            LOCK_FREE_MAP_DESTRUCT_00449C50,
            LOCK_FREE_MAP_DESTRUCT_00449D60,
            LOCK_FREE_MAP_DESTRUCT_00449EC0,
        ];
        ignore(&mut e, &bodies);
        e.call_log = Some(vec![]);
        e.call(0x0044_9c30, &args![this]);
        e.call(0x0044_9d40, &args![this]);
        e.call(0x0044_9ea0, &args![this]);
        for body in bodies {
            assert_eq!(calls_to(&e, body), vec![vec![this.addr()]]);
        }
    }

    #[test]
    fn map_constructors_call_the_base_then_store_their_vtable() {
        let (mut e, this) = engine_with_block(0x40);
        ignore(
            &mut e,
            &[
                LOCK_FREE_MAP_CONSTRUCT_0044C040,
                LOCK_FREE_MAP_CONSTRUCT_0044C270,
                LOCK_FREE_MAP_CONSTRUCT_0044C920,
            ],
        );
        e.call_log = Some(vec![]);
        for (entry, base, vtable) in [
            (
                0x0044_9f50,
                LOCK_FREE_MAP_CONSTRUCT_0044C040,
                LOCK_FREE_MAP_VTABLE_00449F50,
            ),
            (
                0x0044_9fa0,
                LOCK_FREE_MAP_CONSTRUCT_0044C270,
                LOCK_FREE_MAP_VTABLE_00449FA0,
            ),
            (
                0x0044_a100,
                LOCK_FREE_MAP_CONSTRUCT_0044C920,
                LOCK_FREE_MAP_VTABLE_0044A100,
            ),
        ] {
            e.mem.set_u32(this.addr(), 0);
            let back = e.call(entry, &args![this, 1u32, 2u32, 3u32]).ptr::<()>();
            assert_eq!(back, this);
            assert_eq!(calls_to(&e, base).pop(), Some(vec![this.addr(), 1, 2, 3]));
            assert_eq!(e.mem.u32(this.addr()), vtable);
        }
    }

    #[test]
    fn virtual_call_result_is_handed_on_with_the_original_arguments() {
        let (mut e, this) = engine_with_block(8);
        let owner = Ptr::<()>::new(e.mem.alloc(8));
        e.mem.set_u32(this.addr(), owner.addr());
        e.put_vtable(0x0ff0_0000, &[0; 10]);
        e.mem.set_u32(0x0ff0_0000 + 0x24, 0x0ff1_0000);
        e.mem.set_u32(owner.addr(), 0x0ff0_0000);
        e.register(0x0ff1_0000, |_, a| (a[1] + 100).into_ret());
        e.register(LOCK_FREE_MAP_HANDLE, |_, _| 0xffff_ff01u32.into_ret());
        e.call_log = Some(vec![]);
        let result = e.call(0x0044_a130, &args![this, 7u32, 8u32, 1u8]).u32();
        // Only the low byte of the handler's result is the return value.
        assert_eq!(result & 0xff, 1);
        assert_eq!(calls_to(&e, 0x0ff1_0000), vec![vec![owner.addr(), 7]]);
        assert_eq!(
            calls_to(&e, LOCK_FREE_MAP_HANDLE),
            vec![vec![this.addr(), 107, 7, 8, 1]]
        );
    }

    #[test]
    fn byte_global_is_stored() {
        let (mut e, _) = engine_with_block(8);
        e.call(0x0044_ada0, &args![5u8]);
        assert_eq!(e.global::<u8>(TASK_MANAGER_FLAG), 5);
        e.call(0x0044_ada0, &args![0u8]);
        assert_eq!(e.global::<u8>(TASK_MANAGER_FLAG), 0);
    }

    /// A task in `state` whose cancel slot (`0xC`) is a double at
    /// `0ff10100`.
    fn task_in_state(e: &mut Engine, state: u32) -> Ptr {
        let task = Ptr::new(e.mem.alloc(0x20));
        e.put_vtable(0x0ff0_0100, &[0, 0, 0, 0x0ff1_0100]);
        e.mem.set_u32(task.addr(), 0x0ff0_0100);
        e.mem.set_u32(task.addr() + 0xc, state);
        e.register(0x0ff1_0100, |_, _| Ret::default());
        task
    }

    /// Makes `TASK_TRY_SET_STATE` answer `answer` and, like the game's
    /// compare-and-exchange, store 6 in the state when it succeeds.
    fn answer_set_state(e: &mut Engine, answer: bool) {
        e.register_double(TASK_TRY_SET_STATE, move |e, a| {
            if answer {
                e.mem.set_u32(a[0] + 0xc, 6);
            }
            answer.into_ret()
        });
    }

    #[test]
    fn cancel_in_state_0_moves_to_6_and_calls_the_slot() {
        let (mut e, manager) = engine_with_block(8);
        let task = task_in_state(&mut e, 0);
        answer_set_state(&mut e, true);
        ignore(
            &mut e,
            &[TASK_ADD_REFERENCE, TASK_MANAGER_REMOVE, TASK_RELEASE],
        );
        e.set_global(TASK_MANAGER_FLAG, 9u8);
        e.call_log = Some(vec![]);
        e.call(0x0044_ac40, &args![manager, task, 0x55u32]);
        assert_eq!(e.global::<u8>(TASK_MANAGER_FLAG), 0);
        assert_eq!(
            calls_to(&e, TASK_TRY_SET_STATE),
            vec![vec![task.addr(), 0, 6]]
        );
        assert_eq!(calls_to(&e, 0x0ff1_0100), vec![vec![task.addr(), 0, 0x55]]);
        // State 0 takes no reference and does not remove the task.
        assert!(calls_to(&e, TASK_ADD_REFERENCE).is_empty());
        assert!(calls_to(&e, TASK_RELEASE).is_empty());
    }

    #[test]
    fn cancel_in_state_1_references_removes_calls_and_releases() {
        let (mut e, manager) = engine_with_block(8);
        let task = task_in_state(&mut e, 1);
        answer_set_state(&mut e, true);
        ignore(
            &mut e,
            &[TASK_ADD_REFERENCE, TASK_MANAGER_REMOVE, TASK_RELEASE],
        );
        e.call_log = Some(vec![]);
        e.call(0x0044_ac40, &args![manager, task, 0x66u32]);
        assert_eq!(
            call_order(&e),
            vec![
                0x0044_ac40,
                TASK_TRY_SET_STATE,
                TASK_ADD_REFERENCE,
                TASK_MANAGER_REMOVE,
                0x0ff1_0100,
                TASK_RELEASE,
            ]
        );
        assert_eq!(
            calls_to(&e, TASK_MANAGER_REMOVE),
            vec![vec![manager.addr(), task.addr()]]
        );
        assert_eq!(calls_to(&e, 0x0ff1_0100), vec![vec![task.addr(), 1, 0x66]]);
        assert_eq!(calls_to(&e, TASK_RELEASE), vec![vec![task.addr()]]);
    }

    #[test]
    fn cancel_in_state_5_moves_to_6_and_calls_the_slot() {
        let (mut e, manager) = engine_with_block(8);
        let task = task_in_state(&mut e, 5);
        answer_set_state(&mut e, true);
        e.call_log = Some(vec![]);
        e.call(0x0044_ac40, &args![manager, task, 3u32]);
        assert_eq!(calls_to(&e, 0x0ff1_0100), vec![vec![task.addr(), 5, 3]]);
        assert!(calls_to(&e, TASK_ADD_REFERENCE).is_empty());
    }

    #[test]
    fn cancel_retries_when_the_state_move_fails() {
        let (mut e, manager) = engine_with_block(8);
        let task = task_in_state(&mut e, 0);
        // The first move fails and another thread has since made the task
        // state 5; the second succeeds.
        let mut attempts = 0;
        e.register_double(TASK_TRY_SET_STATE, move |e, a| {
            attempts += 1;
            if attempts == 1 {
                e.mem.set_u32(a[0] + 0xc, 5);
                false.into_ret()
            } else {
                true.into_ret()
            }
        });
        e.call_log = Some(vec![]);
        e.call(0x0044_ac40, &args![manager, task, 1u32]);
        assert_eq!(
            calls_to(&e, TASK_TRY_SET_STATE),
            vec![vec![task.addr(), 0, 6], vec![task.addr(), 5, 6]]
        );
        assert_eq!(calls_to(&e, 0x0ff1_0100), vec![vec![task.addr(), 5, 1]]);
    }

    #[test]
    fn cancel_in_states_3_and_4_sleeps_until_the_state_changes() {
        for state in [3u32, 4] {
            let (mut e, manager) = engine_with_block(8);
            let task = task_in_state(&mut e, state);
            answer_set_state(&mut e, true);
            // The third sleep ends the wait (another thread's work).
            let mut sleeps = 0;
            e.register_double(SLEEP, move |e, a| {
                assert_eq!(a, &[0]);
                sleeps += 1;
                if sleeps == 3 {
                    e.mem.set_u32(task.addr() + 0xc, 5);
                }
                Ret::default()
            });
            e.call_log = Some(vec![]);
            e.call(0x0044_ac40, &args![manager, task, 2u32]);
            assert_eq!(calls_to(&e, SLEEP).len(), 3);
            // Once the state is 5 the usual move and slot call follow.
            assert_eq!(calls_to(&e, 0x0ff1_0100), vec![vec![task.addr(), 5, 2]]);
        }
    }

    #[test]
    fn cancel_in_a_later_state_returns_at_once() {
        let (mut e, manager) = engine_with_block(8);
        let task = task_in_state(&mut e, 6);
        e.call_log = Some(vec![]);
        e.call(0x0044_ac40, &args![manager, task, 2u32]);
        assert!(calls_to(&e, TASK_TRY_SET_STATE).is_empty());
        assert!(calls_to(&e, 0x0ff1_0100).is_empty());
        // (State 2 has no test: its loop makes no call and ends only when
        // another thread changes the state.)
    }

    #[test]
    fn element_action_runs_on_every_element() {
        let (mut e, this) = engine_with_block(0x60);
        let array = e.mem.alloc(12);
        e.mem.set_u32(array, 0xa0);
        e.mem.set_u32(array + 4, 0xb0);
        e.mem.set_u32(array + 8, 0xc0);
        e.mem.set_u32(this.addr() + 0x4c, 3);
        e.mem.set_u32(this.addr() + 0x50, array);
        ignore(&mut e, &[ARRAY_ELEMENT_ACTION]);
        e.call_log = Some(vec![]);
        e.call(0x0044_adb0, &args![this]);
        assert_eq!(
            calls_to(&e, ARRAY_ELEMENT_ACTION),
            vec![vec![0xa0], vec![0xb0], vec![0xc0]]
        );
        // An empty array calls nothing.
        e.mem.set_u32(this.addr() + 0x4c, 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_adb0, &args![this]);
        assert!(calls_to(&e, ARRAY_ELEMENT_ACTION).is_empty());
    }

    #[test]
    fn save_binary_writes_one_value_to_the_streams_output() {
        let (mut e, stream) = engine_with_block(0x300);
        let value = Ptr::<()>::new(e.mem.alloc(4));
        e.register(STREAM_OUTPUT, |e, a| e.mem.u32(a[0] + 0x250).into_ret());
        ignore(&mut e, &[WRITE_BINARY]);
        e.mem.set_u32(stream.addr() + 0x250, 0x0abc);
        e.call_log = Some(vec![]);
        e.call(0x0044_adf0, &args![stream, value]);
        assert_eq!(
            calls_to(&e, WRITE_BINARY),
            vec![vec![0x0abc, value.addr(), 1]]
        );
    }

    #[test]
    fn flag_bit_tests_and_setters_use_their_own_bit() {
        let (mut e, _) = engine_with_block(8);
        let tests = [
            (0x0044_ae10u32, 0x01u8),
            (0x0044_ae50, 0x02),
            (0x0044_ae90, 0x04),
            (0x0044_af20, 0x08),
            (0x0044_af60, 0x10),
            (0x0044_afa0, 0x20),
        ];
        let setters = [
            0x0044_ae20u32,
            0x0044_ae60,
            0x0044_aea0,
            0x0044_af30,
            0x0044_af70,
            0x0044_afb0,
        ];
        let flags = Ptr::<()>::new(e.mem.alloc(4));
        for (index, (test, mask)) in tests.iter().enumerate() {
            // The test sees only its bit.
            assert!(e.call(*test, &args![*mask]).bool());
            assert!(e.call(*test, &args![0xffu8]).bool());
            assert!(!e.call(*test, &args![!*mask]).bool());
            assert!(!e.call(*test, &args![0u8]).bool());
            // The setter sets and clears only its bit, whatever non-zero
            // value it is given.
            e.mem.set_u8(flags.addr(), 0x40);
            e.call(setters[index], &args![0x80u8, flags]);
            assert_eq!(e.mem.u8(flags.addr()), 0x40 | mask);
            e.call(setters[index], &args![0u8, flags]);
            assert_eq!(e.mem.u8(flags.addr()), 0x40);
            e.mem.set_u8(flags.addr(), 0xff);
            e.call(setters[index], &args![0u8, flags]);
            assert_eq!(e.mem.u8(flags.addr()), !mask);
        }
    }

    #[test]
    fn model_pointer_assignment_releases_stores_and_references() {
        let (mut e, slot) = engine_with_block(8);
        ignore(&mut e, &[MODEL_RELEASE, MODEL_ADD_REFERENCE]);
        e.mem.set_u32(slot.addr(), 0x1000);
        e.call_log = Some(vec![]);
        let back = e
            .call(0x0044_aed0, &args![slot, Ptr::<Model>::new(0x2000)])
            .ptr::<()>();
        assert_eq!(back, slot);
        assert_eq!(e.mem.u32(slot.addr()), 0x2000);
        assert_eq!(calls_to(&e, MODEL_RELEASE), vec![vec![0x1000]]);
        assert_eq!(calls_to(&e, MODEL_ADD_REFERENCE), vec![vec![0x2000]]);

        // The same model: nothing happens.
        e.call_log = Some(vec![]);
        e.call(0x0044_aed0, &args![slot, Ptr::<Model>::new(0x2000)]);
        assert_eq!(call_order(&e).len(), 1);

        // From null to a model: no release; to null: no reference.
        e.mem.set_u32(slot.addr(), 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_aed0, &args![slot, Ptr::<Model>::new(0x3000)]);
        assert!(calls_to(&e, MODEL_RELEASE).is_empty());
        assert_eq!(calls_to(&e, MODEL_ADD_REFERENCE), vec![vec![0x3000]]);
        e.call_log = Some(vec![]);
        e.call(0x0044_aed0, &args![slot, Ptr::<Model>::NULL]);
        assert_eq!(calls_to(&e, MODEL_RELEASE), vec![vec![0x3000]]);
        assert!(calls_to(&e, MODEL_ADD_REFERENCE).is_empty());
        assert_eq!(e.mem.u32(slot.addr()), 0);
    }

    /// An engine whose interlocked operations really change the counter, so
    /// the tests see the counters move.
    fn counting_engine() -> (Engine, Ptr) {
        let (mut e, block) = engine_with_block(0x40);
        e.register(INTERLOCKED_INCREMENT, |e, a| {
            let value = e.mem.i32(a[0]).wrapping_add(1);
            e.mem.set_i32(a[0], value);
            value.into_ret()
        });
        e.register(INTERLOCKED_DECREMENT, |e, a| {
            let value = e.mem.i32(a[0]).wrapping_sub(1);
            e.mem.set_i32(a[0], value);
            value.into_ret()
        });
        (e, block)
    }

    #[test]
    fn counted_at_0xc_reference_functions() {
        let (mut e, object) = counting_engine();
        let slot = Ptr::<()>::new(e.mem.alloc(4));
        e.call(0x0044_b010, &args![object]);
        assert_eq!(e.mem.i32(object.addr() + 0xc), 1);
        e.call(0x0044_b050, &args![object]);
        assert_eq!(e.mem.i32(object.addr() + 0xc), 0);

        // Constructor: stores and references; null stores only.
        let back = e.call(0x0044_afe0, &args![slot, object]).ptr::<()>();
        assert_eq!(back, slot);
        assert_eq!(e.mem.u32(slot.addr()), object.addr());
        assert_eq!(e.mem.i32(object.addr() + 0xc), 1);
        // Destructor: drops one reference, keeps the pointer.
        e.call(0x0044_b030, &args![slot]);
        assert_eq!(e.mem.i32(object.addr() + 0xc), 0);
        assert_eq!(e.mem.u32(slot.addr()), object.addr());
        e.call(0x0044_afe0, &args![slot, Ptr::<()>::NULL]);
        assert_eq!(e.mem.u32(slot.addr()), 0);
        e.call(0x0044_b030, &args![slot]);
        assert_eq!(e.mem.i32(object.addr() + 0xc), 0);
    }

    #[test]
    fn counted_at_0xc_assignment_moves_the_reference() {
        let (mut e, first) = counting_engine();
        let second = Ptr::<()>::new(e.mem.alloc(0x40));
        let slot = Ptr::<()>::new(e.mem.alloc(4));
        e.call(0x0044_afe0, &args![slot, first]);
        assert_eq!(e.mem.i32(first.addr() + 0xc), 1);
        // Assigning the same object changes nothing.
        e.call(0x0044_b070, &args![slot, first]);
        assert_eq!(e.mem.i32(first.addr() + 0xc), 1);
        let back = e.call(0x0044_b070, &args![slot, second]).ptr::<()>();
        assert_eq!(back, slot);
        assert_eq!(e.mem.u32(slot.addr()), second.addr());
        assert_eq!(e.mem.i32(first.addr() + 0xc), 0);
        assert_eq!(e.mem.i32(second.addr() + 0xc), 1);
        // Assigning null drops the reference.
        e.call(0x0044_b070, &args![slot, Ptr::<()>::NULL]);
        assert_eq!(e.mem.u32(slot.addr()), 0);
        assert_eq!(e.mem.i32(second.addr() + 0xc), 0);
    }

    /// Adds the lock doubles: `TryEnterCriticalSection` fails `refusals`
    /// times, then succeeds; leaving does nothing.
    fn lock_doubles(e: &mut Engine, refusals: u32) {
        let mut remaining = refusals;
        e.register_double(TRY_ENTER_CRITICAL_SECTION, move |_, _| {
            if remaining > 0 {
                remaining -= 1;
                0i32.into_ret()
            } else {
                1i32.into_ret()
            }
        });
        e.register(LEAVE_CRITICAL_SECTION, |_, _| Ret::default());
    }

    #[test]
    fn try_enter_returns_whether_the_section_was_entered() {
        let (mut e, section) = engine_with_block(0x20);
        e.register(TRY_ENTER_CRITICAL_SECTION, |_, a| {
            u32::from(a[0] == LOADED_FILE_LOCK).into_ret()
        });
        assert!(e
            .call(0x0044_b130, &args![Ptr::<()>::new(LOADED_FILE_LOCK)])
            .bool());
        assert!(!e.call(0x0044_b130, &args![section]).bool());
    }

    #[test]
    fn loaded_file_reference_is_added_under_the_lock() {
        let (mut e, file) = counting_engine();
        lock_doubles(&mut e, 2);
        e.call_log = Some(vec![]);
        e.call(0x0044_b0f0, &args![file]);
        assert_eq!(e.mem.i32(file.addr()), 1);
        // Two refused attempts, then the entry that worked.
        assert_eq!(
            calls_to(&e, TRY_ENTER_CRITICAL_SECTION),
            vec![vec![LOADED_FILE_LOCK]; 3]
        );
        assert_eq!(
            calls_to(&e, LEAVE_CRITICAL_SECTION),
            vec![vec![LOADED_FILE_LOCK]]
        );
        // The counter changes after the entry and before the leave.
        let order = call_order(&e);
        assert_eq!(
            &order[order.len() - 2..],
            &[INTERLOCKED_INCREMENT, LEAVE_CRITICAL_SECTION]
        );
    }

    #[test]
    fn loaded_file_release_deletes_at_zero_only() {
        let (mut e, file) = counting_engine();
        lock_doubles(&mut e, 1);
        e.register(LOADED_FILE_DESTRUCT, |_, _| Ret::default());
        e.register(NI_FREE, |_, _| Ret::default());
        e.mem.set_i32(file.addr(), 2);
        e.call_log = Some(vec![]);
        e.call(0x0044_b180, &args![file]);
        assert_eq!(e.mem.i32(file.addr()), 1);
        assert!(calls_to(&e, LOADED_FILE_DESTRUCT).is_empty());
        assert_eq!(calls_to(&e, TRY_ENTER_CRITICAL_SECTION).len(), 2);
        // The last reference: destroyed and freed (flag 1) after the leave.
        e.call_log = Some(vec![]);
        e.call(0x0044_b180, &args![file]);
        assert_eq!(e.mem.i32(file.addr()), 0);
        let order = call_order(&e);
        assert_eq!(
            &order[order.len() - 3..],
            &[LEAVE_CRITICAL_SECTION, LOADED_FILE_DESTRUCT, NI_FREE]
        );
        assert_eq!(
            calls_to(&e, NI_FREE),
            vec![vec![file.addr(), LOADED_FILE_SIZE]]
        );
    }

    #[test]
    fn loaded_file_scalar_deleting_destructor_frees_only_with_flag_1() {
        let (mut e, file) = engine_with_block(0x10);
        e.register(LOADED_FILE_DESTRUCT, |_, _| Ret::default());
        e.register(NI_FREE, |_, _| Ret::default());
        e.call_log = Some(vec![]);
        let back = e.call(0x0044_b1f0, &args![file, 0u32]).ptr::<()>();
        assert_eq!(back, file);
        assert_eq!(calls_to(&e, LOADED_FILE_DESTRUCT), vec![vec![file.addr()]]);
        assert!(calls_to(&e, NI_FREE).is_empty());
        e.call(0x0044_b1f0, &args![file, 1u32]);
        assert_eq!(calls_to(&e, NI_FREE), vec![vec![file.addr(), 0x10]]);
    }

    #[test]
    fn loaded_file_pointer_constructor_destructor_and_assignment() {
        let (mut e, first) = counting_engine();
        let second = Ptr::<()>::new(e.mem.alloc(0x10));
        let slot = Ptr::<()>::new(e.mem.alloc(4));
        lock_doubles(&mut e, 0);
        e.register(LOADED_FILE_DESTRUCT, |_, _| Ret::default());
        e.register(NI_FREE, |_, _| Ret::default());
        let first_file = Ptr::<LoadedFile>::new(first.addr());
        let second_file = Ptr::<LoadedFile>::new(second.addr());

        // Constructor.
        let back = e.call(0x0044_b0c0, &args![slot, first_file]).ptr::<()>();
        assert_eq!(back, slot);
        assert_eq!(e.mem.u32(slot.addr()), first.addr());
        assert_eq!(e.mem.i32(first.addr()), 1);

        // Assigning the same file changes nothing; another one moves the
        // reference (and deletes the old file at zero).
        e.call(0x0044_b220, &args![slot, first_file]);
        assert_eq!(e.mem.i32(first.addr()), 1);
        e.call_log = Some(vec![]);
        e.call(0x0044_b220, &args![slot, second_file]);
        assert_eq!(e.mem.u32(slot.addr()), second.addr());
        assert_eq!(e.mem.i32(first.addr()), 0);
        assert_eq!(e.mem.i32(second.addr()), 1);
        assert_eq!(calls_to(&e, LOADED_FILE_DESTRUCT), vec![vec![first.addr()]]);

        // Destructor releases; a null pointer does nothing.
        e.call_log = Some(vec![]);
        e.call(0x0044_b160, &args![slot]);
        assert_eq!(e.mem.i32(second.addr()), 0);
        assert_eq!(
            calls_to(&e, LOADED_FILE_DESTRUCT),
            vec![vec![second.addr()]]
        );
        e.mem.set_u32(slot.addr(), 0);
        e.call_log = Some(vec![]);
        e.call(0x0044_b160, &args![slot]);
        assert_eq!(call_order(&e).len(), 1);
        // Assigning null releases the old one only.
        e.mem.set_u32(slot.addr(), second.addr());
        e.mem.set_i32(second.addr(), 1);
        e.call(0x0044_b220, &args![slot, Ptr::<LoadedFile>::NULL]);
        assert_eq!(e.mem.u32(slot.addr()), 0);
        assert_eq!(e.mem.i32(second.addr()), 0);
    }

    #[test]
    fn counted_at_0x20_constructor_reference_and_destructor() {
        let (mut e, object) = counting_engine();
        let slot = Ptr::<()>::new(e.mem.alloc(4));
        e.call(0x0044_b2a0, &args![object]);
        assert_eq!(e.mem.i32(object.addr() + 0x20), 1);
        let back = e.call(0x0044_b270, &args![slot, object]).ptr::<()>();
        assert_eq!(back, slot);
        assert_eq!(e.mem.u32(slot.addr()), object.addr());
        assert_eq!(e.mem.i32(object.addr() + 0x20), 2);
        // A null pointer is stored without a reference.
        e.call(0x0044_b270, &args![slot, Ptr::<()>::NULL]);
        assert_eq!(e.mem.u32(slot.addr()), 0);

        // The destructor calls the release only for a non-null pointer.
        ignore(&mut e, &[RELEASE_COUNTED_AT_0X20]);
        e.call_log = Some(vec![]);
        e.call(0x0044_b2c0, &args![slot]);
        assert!(calls_to(&e, RELEASE_COUNTED_AT_0X20).is_empty());
        e.mem.set_u32(slot.addr(), object.addr());
        e.call(0x0044_b2c0, &args![slot]);
        assert_eq!(
            calls_to(&e, RELEASE_COUNTED_AT_0X20),
            vec![vec![object.addr()]]
        );
    }
}
