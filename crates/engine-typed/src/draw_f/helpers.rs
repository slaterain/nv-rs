//! Group F's helpers, each doing what its translation does.

use crate::dispatch::{self, layout::LightLock};
use engine_core::ptr::Ptr;
use engine_core::x87::x87_quiet;
use engine_core::{args, typed_call, Engine};

// Typed from 00404010
/// `cdecl(first, second)`: the greater float, the second when the comparison is unordered; it comes back
/// through a float local, so a signalling NaN is quieted.
pub fn float_max(_e: &mut Engine, first: f32, second: f32) -> f32 {
    x87_quiet(if second < first { first } else { second })
}

// Typed from 0040ebd0
/// `cdecl(a, b)`: the smaller float, `a` when `b > a`, else `b` (also when either is NaN); quieted as
/// [`float_max`].
pub fn float_min(_e: &mut Engine, a: f32, b: f32) -> f32 {
    x87_quiet(if b > a { a } else { b })
}

// Typed from 0043b300
/// `cdecl(type descriptor, object)`: whether `object` is an instance of the class the descriptor names
/// (`NiObject::IsKindOf`, the dispatch family's); false for a null object.
pub fn is_kind_of(e: &mut Engine, type_descriptor: u32, object: Ptr) -> bool {
    if object.is_null() {
        return false;
    }
    typed_call!(
        dispatch::FAMILY,
        0x0065_32c0,
        dispatch::scene::is_kind_of(e, object, type_descriptor)
    )
}

/// The list node pool: its free list (`011c5f58`), its lock (`011c5f80`) and its refill (`0043a080`,
/// no words).
const FREE_HEAD: u32 = 0x011c_5f58;
const LOCK: u32 = 0x011c_5f80;
const REFILL: u32 = 0x0043_a080;

// Typed from 0043a010
/// The list node pool's allocation (`thiscall`, the receiver unread, no stack words): under the pool's
/// lock (the dispatch family's), the free list refilled when empty, its first node taken and its three
/// words cleared. The translation's speech diagnostic (`NV_SPEECH_CHECK`, not exe behaviour) takes no notes
/// here while the family is on.
pub fn allocate_node(e: &mut Engine, _this: Ptr) -> Ptr {
    let lock = Ptr::<LightLock>::new(LOCK);
    typed_call!(
        dispatch::FAMILY,
        0x0040_fbf0,
        dispatch::refs::lock(e, lock, 0u32)
    );
    if e.mem.u32(FREE_HEAD) == 0 {
        e.call(REFILL, &args![]);
    }
    let node = e.mem.u32(FREE_HEAD);
    let next = e.mem.u32(node);
    e.mem.set_u32(FREE_HEAD, next);
    e.mem.set_u32(node.wrapping_add(8), 0);
    e.mem.set_u32(node, 0);
    e.mem.set_u32(node.wrapping_add(4), 0);
    typed_call!(
        dispatch::FAMILY,
        0x0040_fba0,
        dispatch::refs::unlock(e, lock)
    );
    Ptr::new(node)
}
