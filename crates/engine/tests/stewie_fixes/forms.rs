//! Share 1b's crash rows (forms, cells, scripts, saves and sound): each row's crashing input fed
//! to its translated function, row on and off, as in `stewie_fixes.rs`. Only share 1b edits this
//! file.

use super::{ret, LOCK};
use engine_core::extend::stewie;
use engine_core::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering::Relaxed};

/// `PlaceOcclusionPlane`'s command (`005bb9e0`) and the calls it makes that stand in here: the
/// parameters' parse, the 2D point's constructor, the occluder list's owner and list, the world's
/// animation matrix, the rotation and product, the plane's block and constructor, the list's add.
const PLACE_OCCLUSION_PLANE: u32 = 0x005b_b9e0;
const PARSE_PARAMETERS: u32 = 0x005a_ccb0;
const NI_POINT2_CONSTRUCT: u32 = 0x0045_2dc0;
const LIST_OWNER: u32 = 0x0045_0b80;
const OWNER_LIST: u32 = 0x0045_4b30;
const WORLD: u32 = 0x0045_c670;
const ANIMATION: u32 = 0x0066_29f0;
const ANIMATION_MATRIX: u32 = 0x0046_1130;
const MATRIX_FROM_Z_ANGLE: u32 = 0x004a_0c90;
const MATRIX_MULTIPLY: u32 = 0x0043_f8d0;
const NI_ALLOC: u32 = 0x00aa_13e0;
const PLANE_CONSTRUCT: u32 = 0x004a_5ff0;
const LIST_ADD: u32 = 0x00c5_ae70;
const INTERLOCKED_INCREMENT: u32 = 0x00fd_f1c0;
const INTERLOCKED_DECREMENT: u32 = 0x00fd_f1c4;
/// The plane's deleting destructor (its vtable's slot +4, which the release calls at a count of 0).
const PLANE_DELETE: u32 = 0x00ff_5000;

static PLANE: AtomicU32 = AtomicU32::new(0);
static OWNER: AtomicU32 = AtomicU32::new(0);
static LIST: AtomicU32 = AtomicU32::new(0);
static SOURCE: AtomicU32 = AtomicU32::new(0);
static ADDED: AtomicU32 = AtomicU32::new(0);
static FREED: AtomicU32 = AtomicU32::new(0);

/// The command's world: a plane object (vtable, count 0 at `+4`) the block allocation hands out,
/// the occluder list whose add is recorded; the `NiPointer` (`00633c90`, `0045cec0`) and the
/// reference count's add and release (`0040f6e0`, `00401970`) run translated.
fn occlusion_world() -> Engine {
    let mut e = Engine::new();
    for page in [0x0101_2000u32, 0x0101_b000, 0x0103_9000] {
        e.map(page, 0x1000);
    }
    e.mem.set_f32(0x0101_2054, -1.0);
    e.mem.set_f32(0x0101_b268, 50.0);
    let vtable = e.mem.alloc(0x40);
    e.mem.set_u32(vtable + 4, PLANE_DELETE);
    let plane = e.mem.alloc(0x100);
    e.mem.set_u32(plane, vtable);
    PLANE.store(plane, Relaxed);
    // The list's owner: the command reads its position at `+0x1f0` (`005bb9b0`).
    OWNER.store(e.mem.alloc(0x200), Relaxed);
    LIST.store(e.mem.alloc(0x40), Relaxed);
    SOURCE.store(e.mem.alloc(0x40), Relaxed);
    ADDED.store(0, Relaxed);
    FREED.store(0, Relaxed);
    e.register(PARSE_PARAMETERS, |e, a| {
        e.mem.set_f32(a[7], 2.0);
        e.mem.set_f32(a[8], 3.0);
        ret(1)
    });
    e.register(NI_POINT2_CONSTRUCT, |_, a| ret(a[0]));
    e.register(LIST_OWNER, |_, _| ret(OWNER.load(Relaxed)));
    e.register(OWNER_LIST, |_, _| ret(LIST.load(Relaxed)));
    e.register(WORLD, |_, _| ret(0x6100));
    e.register(ANIMATION, |_, _| ret(0x6200));
    e.register(ANIMATION_MATRIX, |_, _| ret(SOURCE.load(Relaxed)));
    e.register(MATRIX_FROM_Z_ANGLE, |_, a| ret(a[0]));
    e.register(MATRIX_MULTIPLY, |_, a| ret(a[1]));
    e.register(NI_ALLOC, |_, _| ret(PLANE.load(Relaxed)));
    e.register(PLANE_CONSTRUCT, |_, a| ret(a[0]));
    e.register(LIST_ADD, |_, a| {
        assert_eq!(a[0], LIST.load(Relaxed));
        ADDED.store(a[1], Relaxed);
        Ret::default()
    });
    // The Windows imports the count's add and release reach: `InterlockedIncrement` and
    // `InterlockedDecrement` (their new value).
    e.register(INTERLOCKED_INCREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_add(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(INTERLOCKED_DECREMENT, |e, a| {
        let n = e.mem.u32(a[0]).wrapping_sub(1);
        e.mem.set_u32(a[0], n);
        ret(n)
    });
    e.register(PLANE_DELETE, |_, a| {
        FREED.store(a[0], Relaxed);
        Ret::default()
    });
    e
}

/// S0236 `PlaceOcclusionPlaneFix`: the exe holds the new plane in a `NiPointer` while it adds it to
/// the occluder list (which takes no reference), so the pointer's release at the command's end
/// takes the count back to 0 and deletes the plane the list still holds (the later crash). With the
/// row the plane goes into the list with no pointer: its count stays 0 and nothing deletes it.
#[test]
fn s0236_place_occlusion_plane_keeps_its_plane() {
    let _one = LOCK.lock().unwrap_or_else(|p| p.into_inner());
    for on in [false, true] {
        let mut e = occlusion_world();
        e.set_stewie(stewie::OCCLUSION_PLANE_KEPT, on);
        let result = e.mem.alloc(8);
        let words = [1u32, 2, 0, 0, 5, 6, result, 8];
        assert!(e.call(PLACE_OCCLUSION_PLANE, &words).bool(), "row {on}");
        let plane = PLANE.load(Relaxed);
        assert_eq!(ADDED.load(Relaxed), plane, "row {on}: the plane is listed");
        assert_eq!(e.mem.u32(plane + 4), 0, "row {on}: its count");
        let freed = if on { 0 } else { plane };
        assert_eq!(FREED.load(Relaxed), freed, "row {on}: deleted while listed");
    }
}
