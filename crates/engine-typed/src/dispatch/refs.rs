//! The counted references, the game's recursive spin lock, lists and arrays, and the small getters
//! the AI, the inventory, the HUD's compass and the process levels call on every actor.

use super::layout::{LightLock, ListNode, RefObject};
use super::FAMILY;
use crate::anim_keys::narrow;
use crate::culling::layout::NiPoint3;
use crate::pathing;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The game's `Sleep` wrapper (`0040fca0`).
const SLEEP: u32 = 0x0040_fca0;
/// The spins the lock makes with `Sleep(0)` before it sleeps a millisecond at a time.
const SPINS_BEFORE_SLEEPING: u32 = 0x2710;
/// `NiRefObject`'s vtable and the count of live objects.
const NI_REF_OBJECT_VTABLE: u32 = 0x0101_dce4;
const NI_REF_OBJECT_COUNT: u32 = 0x011f_4400;
/// The reaction form's list (`00717e50`).
const REACTION_LIST: u32 = 0x0071_7e50;
/// A container's count of a form (`00482a90`), a reference's container (`0055d310`) and an extra
/// list's reference (`0041c8d0`).
const CONTAINER_COUNT: u32 = 0x0048_2a90;
const REFERENCE_CONTAINER: u32 = 0x0055_d310;
const EXTRA_REFERENCE: u32 = 0x0041_c8d0;
/// A reference's water height (`0057b0a0`), parent cell (`008d6f30`), the cell's water test
/// (`004518e0`) and an actor's height (`008853a0`).
const WATER_HEIGHT: u32 = 0x0057_b0a0;
const PARENT_CELL: u32 = 0x008d_6f30;
const CELL_HAS_WATER: u32 = 0x0045_18e0;
const ACTOR_HEIGHT: u32 = 0x0088_53a0;
/// The `double`s: no water (`-FLT_MAX`), the margin under the water, the eye height's share.
const NO_WATER: u32 = 0x0102_41b0;
const WATER_MARGIN: u32 = 0x0102_0998;
const HEIGHT_FACTOR: u32 = 0x0101_de30;
/// The ring of five locations and its index.
const LOCATION_RING: u32 = 0x011d_f7a0;
const LOCATION_RING_INDEX: u32 = 0x011d_f880;
/// The calendar's hour (a float), month, day (a signed byte) and year.
const GAME_HOUR: u32 = 0x011e_0358;
const GAME_MONTH: u32 = 0x011a_39c4;
const GAME_DAY: u32 = 0x011e_0360;
const GAME_YEAR: u32 = 0x011e_035c;
/// The player.
const PLAYER: u32 = 0x011d_ea3c;
/// An array's element destructor run and its buffer's free.
const DESTROY_ELEMENTS: u32 = 0x0072_ba80;
const FREE_BUFFER: u32 = 0x006a_8500;
/// The navmesh order array's vtable (`0106b1a4`) and its set-up (`006b3eb0`).
const ORDER_ARRAY_VTABLE: u32 = 0x0106_b1a4;
const ORDER_ARRAY_SET_UP: u32 = 0x006b_3eb0;

/// A list node's item: the word at the node (`006815c0` gives the node's own address).
#[inline(always)]
fn item(e: &mut Engine, node: u32) -> u32 {
    let at = typed_call!(
        leaf::FAMILY,
        0x0068_15c0,
        leaf::same_word(e, Ptr::<ListNode>::new(node))
    );
    e.mem.u32(at)
}

/// A list node's next (`00726070`).
#[inline(always)]
fn next(e: &mut Engine, node: u32) -> u32 {
    typed_call!(
        leaf::FAMILY,
        0x0072_6070,
        leaf::word_at_4(e, Ptr::<ListNode>::new(node))
    )
}

// Typed from 0040b460
/// `InterlockedIncrement` of the word at `target`, the new count.
pub fn increment(e: &mut Engine, target: Ptr) -> i32 {
    engine_core::sync_direct::interlocked_increment(e, target.addr())
}

// Typed from 004019a0
/// `InterlockedDecrement` of the word at `target`, the new count.
pub fn decrement(e: &mut Engine, target: Ptr) -> i32 {
    engine_core::sync_direct::interlocked_decrement(e, target.addr())
}

// Typed from 0040f6e0
/// `NiRefObject::IncRefCount` (`thiscall(this)`): the count (`+4`) up (`0040b460`).
pub fn add_reference(e: &mut Engine, this: Ptr<RefObject>) {
    typed_call!(FAMILY, 0x0040_b460, increment(e, this.byte_add(4)));
}

// Typed from 00401970
/// `NiRefObject::DecRefCount` (`thiscall(this)`): the count (`+4`) down (`004019a0`); at zero the
/// object deletes itself (vtable +4).
pub fn release(e: &mut Engine, this: Ptr<RefObject>) {
    if typed_call!(FAMILY, 0x0040_19a0, decrement(e, this.byte_add(4))) == 0 {
        e.vcall(this.addr(), 4, &args![]);
    }
}

// Typed from 0045cec0
/// `NiPointer::~NiPointer` (`thiscall(this)`): the held object released (`00401970`), if any.
pub fn release_held(e: &mut Engine, this: Ptr) {
    let held = e.mem.u32(this.addr());
    if held != 0 {
        typed_call!(FAMILY, 0x0040_1970, release(e, Ptr::new(held)));
    }
}

// Typed from 004968b0
/// `NiRefObject::NiRefObject` (`thiscall(this)`): its vtable, a count of 0, one more live object
/// (`0040b460`).
pub fn ref_object_new(e: &mut Engine, this: Ptr<RefObject>) -> Ptr<RefObject> {
    e.set(this, RefObject::vtable, NI_REF_OBJECT_VTABLE);
    e.set(this, RefObject::m_uiRefCount, 0);
    typed_call!(
        FAMILY,
        0x0040_b460,
        increment(e, Ptr::new(NI_REF_OBJECT_COUNT))
    );
    this
}

// Typed from 00496910
/// `NiRefObject::~NiRefObject` (`thiscall(this)`): its vtable back, one live object fewer
/// (`004019a0`).
pub fn ref_object_drop(e: &mut Engine, this: Ptr<RefObject>) {
    e.set(this, RefObject::vtable, NI_REF_OBJECT_VTABLE);
    typed_call!(
        FAMILY,
        0x0040_19a0,
        decrement(e, Ptr::new(NI_REF_OBJECT_COUNT))
    );
}

// Typed from 0040fc90
/// `GetCurrentThreadId`.
pub fn current_thread(e: &mut Engine) -> u32 {
    engine_core::sync_direct::get_current_thread_id(e)
}

// Typed from 0043b460
/// `InterlockedCompareExchange(target, new, old)`: what `target` held.
pub fn compare_exchange(e: &mut Engine, target: Ptr, old: i32, new: i32) -> i32 {
    engine_core::sync_direct::interlocked_compare_exchange(e, target.addr(), new, old)
}

// Typed from 0040fbf0
/// The recursive spin lock taken (`thiscall(this, unused)`): by its owner, one more count; otherwise
/// the owner word swapped from 0 to this thread's id (`0043b460`), `Sleep(0)` between tries and
/// `Sleep(1)` after 10,000, then a count of 1.
pub fn lock(e: &mut Engine, this: Ptr<LightLock>, _unused: u32) {
    let thread = typed_call!(FAMILY, 0x0040_fc90, current_thread(e));
    // The owner participates in the compare-exchange below. With real host
    // threads its recursive fast-path read must be atomic too.
    let owner = if e.mem.is_shared() {
        e.mem
            .atomic_u32(this.addr())
            .map(|word| word.load(std::sync::atomic::Ordering::Relaxed))
    } else {
        None
    }
    .unwrap_or_else(|| e.get(this, LightLock::owner));
    if owner == thread {
        let count = e.get(this, LightLock::count).wrapping_add(1);
        e.set(this, LightLock::count, count);
        return;
    }
    let mut attempts = 0u32;
    let new = thread as i32;
    while typed_call!(
        FAMILY,
        0x0043_b460,
        compare_exchange(e, this.cast(), 0, new)
    ) != 0
    {
        attempts = attempts.wrapping_add(1);
        let millis = u32::from(attempts > SPINS_BEFORE_SLEEPING);
        e.call(SLEEP, &args![millis]);
    }
    typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e));
    e.set(this, LightLock::count, 1);
}

// Typed from 0040fba0
/// The lock given back (`thiscall(this)`): one count fewer; at zero no owner ([`unlock_inline`]).
pub fn unlock(e: &mut Engine, this: Ptr<LightLock>) {
    typed_call!(leaf::FAMILY, 0x0040_fbe0, leaf::nothing(e));
    unlock_inline(e, this);
}

/// `0040fba0`'s body past its empty call, as the exe inlines it where it gives the lock back in
/// place (the shadow scene node's queues' lock, `cell_refs::lighting`): one count fewer and, at
/// zero, the owner word cleared.
#[inline]
pub fn unlock_inline(e: &mut Engine, this: Ptr<LightLock>) {
    let count = e.get(this, LightLock::count).wrapping_sub(1);
    e.set(this, LightLock::count, count);
    if count == 0 {
        // 0040fbc7's x86 MOV publishes earlier protected writes. An ordinary
        // ARM store does not: pair this release with lock's acquire CAS.
        if e.mem.is_shared() {
            if let Some(owner) = e.mem.atomic_u32(this.addr()) {
                owner.store(0, std::sync::atomic::Ordering::Release);
                return;
            }
            // Preserve Mem's unaligned/fault/diagnostic path.
            std::sync::atomic::fence(std::sync::atomic::Ordering::Release);
        }
        e.set(this, LightLock::owner, 0);
    }
}

// Typed from 004534f0
/// Does nothing (`RET 8`: two stack words).
pub fn nothing_2(_e: &mut Engine, _this: u32, _value: u32) {}

// Typed from 00483710
/// Does nothing (`RET 4`).
pub fn nothing_1(_e: &mut Engine, _this: u32) {}

// Typed from 00460140
/// The address `this + 0x10` (the data handler's world space list, a holder's value).
pub fn at_10(_e: &mut Engine, this: Ptr) -> Ptr {
    this.byte_add(0x10)
}

// Typed from 008c71b0
/// The word at `this` becomes `value`; `this`.
pub fn store_word(e: &mut Engine, this: Ptr, value: u32) -> Ptr {
    e.mem.set_u32(this.addr(), value);
    this
}

// Typed from 0048c1b0
/// `TESReactionForm::GetReaction` (`thiscall(this, form)`): the reaction (+8) of the first entry of the
/// list (`00717e50`) whose form is `form`; 0 for none or a null form.
pub fn reaction(e: &mut Engine, this: Ptr, form: u32) -> u32 {
    if form == 0 {
        return 0;
    }
    let mut node = e.call(REACTION_LIST, &args![this]).u32();
    while node != 0 && item(e, node) != 0 {
        let entry = item(e, node);
        if e.mem.u32(entry) == form {
            return e.mem.u32(entry + 8);
        }
        node = next(e, node);
    }
    0
}

// Typed from 008f6650
/// `HighProcess::GetDetectionState` (`thiscall(this, actor, which)`): the first detection state of the
/// detected list (`+0x25c`, `which` 0) or the detecting list (`+0x260`) whose actor is `actor`.
pub fn detection_state(e: &mut Engine, this: Ptr, actor: u32, which: i32) -> Ptr {
    let mut found = 0;
    let mut node = e
        .mem
        .u32(this.addr() + if which == 0 { 0x25c } else { 0x260 });
    while node != 0 {
        let state = item(e, node);
        if state == 0 || found != 0 {
            break;
        }
        if e.mem.u32(state) == actor {
            found = state;
        }
        node = next(e, node);
    }
    Ptr::new(found)
}

// Typed from 00968670
/// The word at `index` of an array (`00877a30` gives the slot).
pub fn array_item(e: &mut Engine, this: Ptr, index: u32) -> u32 {
    let slot = typed_call!(
        leaf::FAMILY,
        0x0087_7a30,
        leaf::array_element(e, this, index)
    );
    e.mem.u32(slot)
}

// Typed from 004c8f30
/// `InventoryChanges::GetItemCount` (`thiscall(this, form)`): the container's count of the form
/// (`004bffb0`, `00482a90`, made positive), plus the changes' entry's count (`004bfba0`); an entry
/// that counts 0 where the container has none counts 1.
pub fn item_count(e: &mut Engine, this: Ptr, form: u32) -> i32 {
    let container = typed_call!(FAMILY, 0x004b_ffb0, container_of(e, this));
    let mut count = if container != 0 {
        e.call(CONTAINER_COUNT, &args![container, form]).i32()
    } else {
        0
    };
    if count < 0 {
        count = count.wrapping_mul(-1);
    }
    let entry = typed_call!(FAMILY, 0x004b_fba0, entry_for(e, this, form, 1, 0));
    if entry != 0 {
        let number = e.mem.i32(entry + 4);
        count = if count == 0 && number == 0 {
            1
        } else {
            e.mem.i32(entry + 4).wrapping_add(count)
        };
    }
    count
}

// Typed from 004bffb0
/// The container of the changes' reference (`+4`, `0055d310`), or 0 without one.
pub fn container_of(e: &mut Engine, this: Ptr) -> u32 {
    let owner = e.mem.u32(this.addr() + 4);
    if owner == 0 {
        return 0;
    }
    e.call(REFERENCE_CONTAINER, &args![owner]).u32()
}

// Typed from 004bfba0
/// `InventoryChanges::GetEntryForObject` (`thiscall(this, form, unused, reference)`): the change entry
/// of `form` (its `+8`) in the list (`+0`); with a reference id, only when one of the entry's extra
/// lists has a reference (`0041c8d0`) whose id (`+0xc`) is it.
pub fn entry_for(e: &mut Engine, this: Ptr, form: u32, _unused: u32, reference: u32) -> u32 {
    let mut cursor = e.mem.u32(this.addr());
    let mut searching = true;
    while cursor != 0 && searching {
        let entry = item(e, cursor);
        if entry != 0 && e.mem.u32(entry + 8) == form {
            searching = false;
        } else {
            cursor = next(e, cursor);
        }
    }
    if cursor == 0 {
        return 0;
    }
    let entry = item(e, cursor);
    if entry == 0 || reference == 0 {
        return entry;
    }
    let mut inner = e.mem.u32(entry);
    while inner != 0 {
        let extra = item(e, inner);
        if extra != 0 && e.call(EXTRA_REFERENCE, &args![extra]).u32() != 0 {
            let found = e.call(EXTRA_REFERENCE, &args![extra]).u32();
            if typed_call!(leaf::FAMILY, 0x0084_e3a0, leaf::word_at_c(e, found)) == reference {
                return entry;
            }
        }
        inner = next(e, inner);
    }
    0
}

// Typed from 008ae4c0
/// `Actor::GetLocationOnReference` (`thiscall(this)`): the position (`00436aa0`), or, when the actor
/// has the flag at `+0x14d` and a water height (`0057b0a0`) that isn't `-FLT_MAX`, a copy of it in the
/// next of five slots of the ring at `011df7a0`, its z raised by the actor's height (`008853a0`) times
/// the factor at `0101de30`, but at most to the water height minus the margin at `01020998`, in a
/// cell with water (`004518e0`). The water and the rise in `f64`, rounded where the exe stores them.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // NaN-aware as the exe: not above, including unordered.
pub fn location_on_reference(e: &mut Engine, this: Ptr) -> u32 {
    if typed_call!(FAMILY, 0x005a_2030, byte_14d(e, this)) == 0 {
        return typed_call!(FAMILY, 0x0043_6aa0, position(e, this)).addr();
    }
    let water = e.call(WATER_HEIGHT, &args![this]).f64();
    if water == e.global::<f64>(NO_WATER) {
        return typed_call!(FAMILY, 0x0043_6aa0, position(e, this)).addr();
    }
    let index = e.global::<i32>(LOCATION_RING_INDEX).wrapping_add(1);
    e.set_global(LOCATION_RING_INDEX, index);
    if index >= 5 {
        e.set_global(LOCATION_RING_INDEX, 0i32);
    }
    let position = typed_call!(FAMILY, 0x0043_6aa0, position(e, this)).addr();
    let slot = |e: &Engine| LOCATION_RING + 12 * e.global::<u32>(LOCATION_RING_INDEX);
    super::copy_words(e, position, slot(e), 3);
    let cell = e.call(PARENT_CELL, &args![this]).u32();
    if cell != 0 && e.call(CELL_HAS_WATER, &args![cell]).u32() & 0xff != 0 {
        let ceiling =
            narrow(e.call(WATER_HEIGHT, &args![this]).f64() - e.global::<f64>(WATER_MARGIN));
        let rise =
            narrow(e.call(ACTOR_HEIGHT, &args![this]).f64() * e.global::<f64>(HEIGHT_FACTOR));
        let z = Ptr::<NiPoint3>::new(slot(e));
        let top = f64::from(e.get(z, NiPoint3::z)) + f64::from(rise);
        if !(f64::from(ceiling) > top) {
            e.set(z, NiPoint3::z, ceiling);
        } else {
            let z = Ptr::<NiPoint3>::new(slot(e));
            let raised = narrow(f64::from(e.get(z, NiPoint3::z)) + f64::from(rise));
            e.set(z, NiPoint3::z, raised);
        }
    }
    slot(e)
}

// Typed from 00436aa0
/// A reference's position: the address `this + 0x30`.
pub fn position(_e: &mut Engine, this: Ptr) -> Ptr {
    this.byte_add(0x30)
}

// Typed from 005a2030
/// The byte at `+0x14d`.
pub fn byte_14d(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 0x14d)
}

// Typed from 008844f0
/// Whether the actor's state (`004f8960`, read again for each test) is 2 or 1, or 6 unless
/// `exclude_six`.
pub fn in_state(e: &mut Engine, this: Ptr, exclude_six: bool) -> bool {
    let is = |e: &mut Engine, v: u32| typed_call!(FAMILY, 0x004f_8960, word_108(e, this)) == v;
    if exclude_six {
        is(e, 2) || is(e, 1)
    } else {
        is(e, 2) || is(e, 1) || is(e, 6)
    }
}

// Typed from 004f8960
/// The word at `+0x108`.
pub fn word_108(e: &mut Engine, this: Ptr) -> u32 {
    e.mem.u32(this.addr().wrapping_add(0x108))
}

// Typed from 00907650
/// `LowProcess::ComputeLastTimeProcessed` (`thiscall(this)`): the hour last processed becomes the
/// game's hour (`00693d50`) and the date `month << 9 | day | year << 13` (`007037c0`; the day byte
/// sign-extended).
pub fn compute_last_time_processed(e: &mut Engine, this: Ptr) {
    let hour = e.global::<f32>(GAME_HOUR);
    typed_call!(FAMILY, 0x0069_3d50, set_hour(e, this, hour));
    let month = e.global::<u32>(GAME_MONTH);
    let day = i32::from(e.global::<i8>(GAME_DAY)) as u32;
    let year = e.global::<u32>(GAME_YEAR);
    typed_call!(
        FAMILY,
        0x0070_37c0,
        set_date(e, this, (month << 9) | day | (year << 0xd))
    );
}

// Typed from 00693d50
/// The float at `+0x20` becomes `value`.
pub fn set_hour(e: &mut Engine, this: Ptr, value: f32) {
    e.mem.set_f32(this.addr() + 0x20, value);
}

// Typed from 007037c0
/// The word at `+0x24` becomes `value`.
pub fn set_date(e: &mut Engine, this: Ptr, value: u32) {
    e.mem.set_u32(this.addr().wrapping_add(0x24), value);
}

/// An actor's position for the compass (vtable +0x1f4), its three floats.
fn compass_position(e: &mut Engine, object: u32) -> [u32; 3] {
    let p = e.vcall(object, 0x1f4, &args![]).u32();
    [e.mem.u32(p), e.mem.u32(p + 4), e.mem.u32(p + 8)]
}

/// `|from - to|^2` as the compass sorts by it: the difference (`00439ef0`), its squared length
/// (`004a7290`) and that through the float `fabs` (`00408840`), in the translation's frames.
fn compass_distance(e: &mut Engine, from: [u32; 3], to: [u32; 3]) -> f32 {
    e.with_stack(24, |e, points| {
        for (index, word) in (0u32..).zip(from.into_iter().chain(to)) {
            e.mem.set_u32(points.addr() + 4 * index, word);
        }
        e.with_stack(12, |e, difference| {
            let a = points.cast::<NiPoint3>();
            let b = Ptr::<NiPoint3>::new(points.addr() + 12);
            let vector = typed_call!(
                pathing::FAMILY,
                0x0043_9ef0,
                pathing::point::subtract(e, a, difference.cast(), b)
            );
            let squared = typed_call!(
                pathing::FAMILY,
                0x004a_7290,
                pathing::point::length_squared(e, vector)
            );
            typed_call!(
                pathing::FAMILY,
                0x0040_8840,
                pathing::point::abs(e, squared)
            )
        })
    })
}

// Typed from 0077b300
/// The HUD's compass order of two actors (`cdecl(first, second)`, slots holding them): 1 when the
/// second is nearer the player, -1 when the first is, else 0; 0 when a slot or its actor is null.
pub fn compass_order(e: &mut Engine, first: Ptr<u32>, second: Ptr<u32>) -> i32 {
    if first.is_null()
        || e.mem.u32(first.addr()) == 0
        || second.is_null()
        || e.mem.u32(second.addr()) == 0
    {
        return 0;
    }
    let player = e.mem.u32(PLAYER);
    let player_position = compass_position(e, player);
    let first_actor = e.mem.u32(first.addr());
    let first_position = compass_position(e, first_actor);
    let first_distance = compass_distance(e, first_position, player_position);
    let second_actor = e.mem.u32(second.addr());
    let second_position = compass_position(e, second_actor);
    let second_distance = compass_distance(e, second_position, player_position);
    if second_distance < first_distance {
        1
    } else if first_distance < second_distance {
        -1
    } else {
        0
    }
}

// Typed from 0086a050
/// The factor of a source's object (`009ad610`): its vtable +0x420 when its +0x100 says so, else 1.0.
pub fn object_factor(e: &mut Engine, source: u32) -> f64 {
    let object = typed_call!(FAMILY, 0x009a_d610, word_cc(e, Ptr::new(source))).addr();
    if object != 0 && e.vcall(object, 0x100, &args![]).bool() {
        return e.vcall(object, 0x420, &args![]).f64();
    }
    1.0
}

// Typed from 009ad610
/// The word at `+0xcc`.
pub fn word_cc(e: &mut Engine, this: Ptr) -> Ptr {
    Ptr::new(e.mem.u32(this.addr() + 0xcc))
}

// Typed from 008454f0
/// `BSSimpleArray::Clear` (`thiscall(this, free)`): with a buffer, its elements destroyed
/// (`0072ba80`), with `free` the buffer freed (`006a8500`) and the buffer and reserved size cleared;
/// the size 0.
pub fn array_clear(e: &mut Engine, this: Ptr, free: u8) {
    let at = this.addr();
    let buffer = e.mem.u32(at + 4);
    if buffer == 0 {
        return;
    }
    let count = e.mem.u32(at + 8);
    e.call(DESTROY_ELEMENTS, &args![this, buffer, count]);
    if free != 0 {
        e.call(FREE_BUFFER, &args![this]);
        e.mem.set_u32(at + 4, 0);
        e.mem.set_u32(at + 0xc, 0);
    }
    e.mem.set_u32(at + 8, 0);
}

// Typed from 0069aee0
/// The navmesh order array's constructor: its vtable, then its set-up with no buffer (`006b3eb0`).
pub fn order_array_new(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), ORDER_ARRAY_VTABLE);
    e.call(ORDER_ARRAY_SET_UP, &args![this, 0u32, 0u32]);
    this
}

// Typed from 0069af10
/// The navmesh info pointer array's destructor: its vtable, then cleared and freed (`008454f0`).
pub fn info_array_drop(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), ORDER_ARRAY_VTABLE);
    typed_call!(FAMILY, 0x0084_54f0, array_clear(e, this, 1));
    this
}

// Typed from 004f1540
/// `BSExtraData`'s type byte (`+4`).
pub fn extra_kind(e: &mut Engine, this: Ptr) -> u8 {
    e.mem.u8(this.addr() + 4)
}

/// The bucket and compare functions of the game's maps keyed by a word (`NiTMapBase<UInt32, ..>`):
/// the key modulo the bucket count at `+4` (`006c6a60`) and plain equality (`006b81d0`), what the
/// maps of the save's references (vtables `010800a8` and `010800c8`) hold in slots 4 and 8.
const WORD_MAP_HASH: u32 = 0x006c_6a60;
const WORD_MAP_EQUAL: u32 = 0x006b_81d0;
/// The menus' maps' own ([`super::tile`]): the menu string map's hash and compare (vtable
/// `01094d9c`) and the integer maps' (the reaction map's, vtables `01094dfc` and `01094e3c`).
const MENU_STRING_HASH: u32 = 0x00a0_d1b0;
const MENU_STRING_EQUAL: u32 = 0x00a0_d180;
const INT_MAP_HASH: u32 = 0x00a0_d260;
const INT_MAP_EQUAL: u32 = 0x00ae_8dc0;

/// The word compare the game hands [`array_find`] most (`009a3830`, `cdecl(a, b)`: whether the
/// words at `a` and `b` are equal).
const WORDS_EQUAL: u32 = 0x009a_3830;

// Typed from 00719b20
/// A word array's search (`BSSimpleArray`: buffer at `+4`, size at `+8`; `thiscall(this, argument,
/// start, matches)`): from index `start`, the first element `matches(&element, argument)` accepts
/// (`cdecl`, by address), else -1; a null buffer skips the call for every index (the size and the
/// buffer read again for each index, as the exe reads them). With the word compare (`009a3830`) the
/// two words are compared in place: `Interface::TileIsBeingDeleted` searches the interface's update
/// array this way for every tile a menu deletes (twice: its release and its destructor), so
/// closing a large menu made hundreds of thousands of calls by address here.
pub fn array_find(e: &mut Engine, this: Ptr, argument: u32, start: u32, matches: u32) -> i32 {
    let a = this.addr();
    let mut index = start;
    while index < e.mem.u32(a.wrapping_add(8)) {
        let buffer = e.mem.u32(a.wrapping_add(4));
        if buffer != 0 {
            let element = buffer.wrapping_add(index.wrapping_mul(4));
            let hit = if matches == WORDS_EQUAL {
                e.mem.u32(element) == e.mem.u32(argument)
            } else {
                e.call(matches, &args![element, argument]).bool()
            };
            if hit {
                return index as i32;
            }
        }
        index = index.wrapping_add(1);
    }
    -1
}

// Typed from 0049c680
/// A simple list's find (`thiscall(this, wanted, start)`, `RET 8`): from `start`, or from the list's
/// first node when it is 0 (`00559450`), the first node whose item equals the word at `wanted`
/// (read again for each node, as the exe reads it), else 0. Every tile a menu deletes asks it of the
/// list of hibernating tiles (`Tile::~Tile`, through `0057c730`), so a menu of many hidden tiles
/// walked that list once per tile with a call by address per node; here the walk is a loop.
pub fn list_find(e: &mut Engine, this: Ptr, wanted: Ptr<u32>, start: u32) -> u32 {
    let mut node = start;
    if node == 0 {
        node = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, this));
    }
    // The exe steps with `0057cbe0` (the node's next at `+0`, its item at `+8`), its node variable
    // on its stack; the step's two reads, here in place.
    while node != 0 {
        let current = node;
        let item = node.wrapping_add(8);
        node = e.mem.u32(node);
        if e.mem.u32(wanted.addr()) == e.mem.u32(item) {
            return current;
        }
    }
    0
}

// Typed from 00853130
// Typed from 00a1e380
/// A map's lookup (`thiscall(this, key, out)`, `NiTMapBase::GetAt`): the key's bucket from the map's
/// hash (vtable `+4`), then the bucket's chain (the table at `+8`; a node is the next node, its key
/// and its value), each node's key compared with the map's compare (vtable `+8`); the value of the
/// first equal one stored through `out`, answering 1, else 0 with `out` untouched. The save's
/// references map is one of these with 37 buckets: a quicksave in Goodsprings makes about 230,000
/// lookups over about 2.8 million nodes, all on the main thread in the save's one frame, so the hash
/// and the compare are called directly when the slots hold the word map's own (above) and by address
/// otherwise; the chain is read after each compare, as in the exe.
///
/// The exe holds this body twice, instruction for instruction the same (the same frame, vtable
/// slots, table offset, node layout and `RET 8`): at `00853130` and at `00a1e380`, which the menus'
/// tile and text maps, the sound manager's maps and every thread's scrap-heap map use (the top
/// function of the start-up load in a profile of 2026-11-11). Both addresses hold this function.
pub fn map_lookup(e: &mut Engine, this: Ptr, key: u32, out: Ptr<u32>) -> u8 {
    let t = this.addr();
    let hash = match crate::slot_target(e, t, 4) {
        WORD_MAP_HASH => typed_call!(
            pathing::FAMILY,
            0x006c_6a60,
            pathing::search::hash(e, this.cast(), key)
        ),
        MENU_STRING_HASH => typed_call!(
            super::FAMILY,
            0x00a0_d1b0,
            super::tile::string_hash(e, this, Ptr::new(key))
        ),
        INT_MAP_HASH => typed_call!(
            super::FAMILY,
            0x00a0_d260,
            super::tile::int_hash(e, this, key)
        ),
        _ => e.vcall(t, 4, &args![key]).u32(),
    };
    let equal_slot = crate::slot_target(e, t, 8);
    let table = e.mem.u32(t.wrapping_add(8));
    let mut node = e.mem.u32(table.wrapping_add(hash.wrapping_mul(4)));
    while node != 0 {
        let existing = e.mem.u32(node.wrapping_add(4));
        let equal = match equal_slot {
            WORD_MAP_EQUAL => typed_call!(
                pathing::FAMILY,
                0x006b_81d0,
                pathing::search::keys_equal(e, this, key, existing)
            ),
            MENU_STRING_EQUAL => u8::from(typed_call!(
                super::FAMILY,
                0x00a0_d180,
                super::tile::string_keys_equal(e, this, Ptr::new(key), Ptr::new(existing))
            )),
            INT_MAP_EQUAL => u8::from(typed_call!(
                super::FAMILY,
                0x00ae_8dc0,
                super::tile::int_keys_equal(e, t, key, existing)
            )),
            _ => e.vcall(t, 8, &args![key, existing]).u8(),
        };
        if equal != 0 {
            let value = e.mem.u32(node.wrapping_add(8));
            e.mem.set_u32(out.addr(), value);
            return 1;
        }
        node = e.mem.u32(node);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    /// A lock (its owner word, then its count) and a word it guards, on a page of their own.
    const LOCK: u32 = 0x0300_0000;
    const GUARDED: u32 = LOCK + 0x40;

    /// An engine whose memory is shared, as with real threads.
    fn shared() -> Engine {
        let mut e = Engine::new();
        e.map(LOCK, 0x1000);
        e.mem.start_sharing();
        e
    }

    /// On shared memory a count past 1 keeps the owner; the last count clears it.
    #[test]
    fn the_last_count_clears_the_owner_on_shared_memory() {
        let mut e = shared();
        assert!(e.mem.is_shared());
        e.mem.set_u32(LOCK, 7);
        e.mem.set_u32(LOCK + 4, 2);
        unlock_inline(&mut e, Ptr::new(LOCK));
        assert_eq!((e.mem.u32(LOCK), e.mem.u32(LOCK + 4)), (7, 1));
        unlock_inline(&mut e, Ptr::new(LOCK));
        assert_eq!((e.mem.u32(LOCK), e.mem.u32(LOCK + 4)), (0, 0));
    }

    /// The release stores its count before it clears the owner word (at `0040fbc7`), seen on one
    /// thread with both words watched: whatever the timing of other threads, a release that cleared
    /// the owner first would let a waiter's count, stored between the two, be overwritten with 0.
    #[test]
    fn the_release_stores_the_count_before_it_clears_the_owner() {
        let mut e = Engine::new();
        e.map(LOCK, 0x1000);
        e.mem.set_u32(LOCK, 7);
        e.mem.set_u32(LOCK + 4, 1);
        e.mem.watch(LOCK, 8, false);
        unlock_inline(&mut e, Ptr::new(LOCK));
        let order: Vec<u32> = e.mem.take_watch_hits().iter().map(|hit| hit.addr).collect();
        assert_eq!(order, [LOCK + 4, LOCK]);
        assert_eq!((e.mem.u32(LOCK), e.mem.u32(LOCK + 4)), (0, 0));
    }

    /// Another real thread takes the lock as `0040fbf0` does (the owner word swapped from 0 to its
    /// id, spinning on it, then a count of 1) once this one gives it back, and reads what this one
    /// wrote under it. Its count of 1 stays: the release stores this thread's count before it clears
    /// the owner, so no store of this thread's lands after the waiter's. Many rounds, each waiter
    /// spinning before the release starts, so a release that cleared the owner first would let some
    /// waiter's count be overwritten with 0.
    #[test]
    fn a_thread_waiting_on_shared_memory_takes_the_lock_given_back() {
        let mut e = shared();
        for round in 0..200u32 {
            e.mem.set_u32(LOCK, 7);
            e.mem.set_u32(LOCK + 4, 1);
            let other = e.mem.view();
            let spinning = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let started = spinning.clone();
            let waiter = std::thread::spawn(move || {
                let owner = other.atomic_u32(LOCK).expect("an aligned, mapped word");
                let deadline = Instant::now() + Duration::from_secs(5);
                started.store(true, Ordering::Release);
                while owner
                    .compare_exchange(0, 9, Ordering::Acquire, Ordering::Relaxed)
                    .is_err()
                {
                    if Instant::now() > deadline {
                        return None;
                    }
                    std::hint::spin_loop();
                }
                other.set_u32(LOCK + 4, 1);
                Some(other.u32(GUARDED))
            });
            while !spinning.load(Ordering::Acquire) {
                std::hint::spin_loop();
            }
            e.mem.set_u32(GUARDED, 0x1234_5678 + round);
            unlock_inline(&mut e, Ptr::new(LOCK));
            let seen = waiter.join().expect("the waiting thread");
            assert_eq!(
                seen,
                Some(0x1234_5678 + round),
                "round {round}: the waiter took the lock"
            );
            assert_eq!(
                (e.mem.u32(LOCK), e.mem.u32(LOCK + 4)),
                (9, 1),
                "round {round}: the waiter's count stays after the release"
            );
        }
    }
}
