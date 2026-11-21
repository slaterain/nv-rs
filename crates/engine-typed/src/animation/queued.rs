//! An actor's postponed scene graph update: `Animation::UpdateQueuedScenegraph` (`00493930`), which
//! the bone pass after the frame is shown (`0096c970`) runs for every high-process actor, and
//! `Animation::UpdateSceneGraphNoController` (`00493bd0`), which the AI threads call while they
//! animate (and which postpones the update while main draws). The animation root's downward pass is
//! the scene family's, called directly; the controllers' detach and restore stay by address.

use crate::{dispatch, scene};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// `Animation`'s fields: the flag byte (+0), the animation root (`NiPointer`, +0x8), the
/// accumulation root (+0xc), the time (+0xd0).
const ANIM_ROOT: u32 = 0x08;
const ACCUM_ROOT: u32 = 0x0c;
const TIME: u32 = 0xd0;
/// The flag set (`00493880(this, value, mask)`) and the pending bit (2).
const FLAG_SET: u32 = 0x0049_3880;
const FLAG_PENDING: u32 = 2;
/// The integer setting's value address (`0043d4d0`) of the limit (`011c3ea4`, the thread count), the
/// global byte that forces the update (`011e07a8`), the flag object (`011ddf38`) and its test
/// (`0042ce10`).
const SETTING_VALUE_ADDRESS: u32 = 0x0043_d4d0;
const SETTING_SCENE_GRAPH_LIMIT: u32 = 0x011c_3ea4;
const SCENE_GRAPH_ALWAYS_UPDATE: u32 = 0x011e_07a8;
const FLAG_OBJECT_GLOBAL: u32 = 0x011d_df38;
const FLAG_OBJECT_CHECK: u32 = 0x0042_ce10;
/// The update data's constructor (`0043d410`, `thiscall(data, time, 1, 0)`), the controllers' set
/// (`00a5c000`), `NiPointer`'s init from a pointer (`00633c90`) and release (`0045cec0`).
const UPDATE_DATA_CONSTRUCT: u32 = 0x0043_d410;
const OBJECT_SET_CONTROLLERS: u32 = 0x00a5_c000;
const NI_POINTER_INIT: u32 = 0x0063_3c90;
const NI_POINTER_RELEASE: u32 = 0x0045_cec0;

fn pointer_get(e: &mut Engine, slot: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot))
}

fn controllers(e: &mut Engine, object: u32) -> u32 {
    typed_call!(
        dispatch::FAMILY,
        0x0043_b230,
        dispatch::scene::first_controller(e, Ptr::new(object))
    )
    .addr()
}

/// Whether the pending bit is set (`00493970`: `00493990` with 2).
fn pending(e: &Engine, this: u32) -> bool {
    e.mem.u8(this) & 2 != 0
}

// Typed from 00493930
/// `Animation::UpdateQueuedScenegraph` (`thiscall(animation)`): with the pending bit set, the scene
/// graph update ([`update_scene_graph_no_controller`]) and the bit cleared; whether it ran.
pub fn update_queued_scenegraph(e: &mut Engine, this: Ptr) -> bool {
    if !pending(e, this.addr()) {
        return false;
    }
    // A plain call, as the exe's (the translation's) is.
    update_scene_graph_no_controller(e, this);
    e.call(FLAG_SET, &args![this, 0u32, FLAG_PENDING]);
    true
}

// Typed from 00493bd0
/// `Animation::UpdateSceneGraphNoController` (`thiscall(animation)`): with an animation root and an
/// accumulation root, either postpones the update (the pending bit set) when the limit setting is
/// above 1, nothing forces it, it is not pending already and the flag object's test fails; or runs
/// the animation root's downward pass with the update data (the animation's time, 1, 0) and the
/// roots' controllers detached for it and put back after.
pub fn update_scene_graph_no_controller(e: &mut Engine, this: Ptr) {
    let a = this.addr();
    if pointer_get(e, a + ANIM_ROOT) == 0 {
        return;
    }
    if e.mem.u32(a + ACCUM_ROOT) == 0 {
        return;
    }
    let limit_address = e
        .call(SETTING_VALUE_ADDRESS, &args![SETTING_SCENE_GRAPH_LIMIT])
        .u32();
    let limit = e.mem.u32(limit_address) as i32;
    let forced = e.mem.u8(SCENE_GRAPH_ALWAYS_UPDATE) as i8;
    let flag_object = e.mem.u32(FLAG_OBJECT_GLOBAL);
    if limit > 1
        && forced == 0
        && !pending(e, a)
        && !e.call(FLAG_OBJECT_CHECK, &args![flag_object]).bool()
    {
        e.call(FLAG_SET, &args![this, 1u32, FLAG_PENDING]);
        return;
    }
    let time = e.mem.f32(a + TIME);
    e.with_stack(12, |e, data| {
        e.call(UPDATE_DATA_CONSTRUCT, &args![data, time, 1u32, 0u32]);
        let root = pointer_get(e, a + ANIM_ROOT);
        let root_controllers = controllers(e, root);
        e.with_stack(4, |e, saved_root| {
            e.call(NI_POINTER_INIT, &args![saved_root, root_controllers]);
            let accum_root = e.mem.u32(a + ACCUM_ROOT);
            let accum_controllers = controllers(e, accum_root);
            e.with_stack(4, |e, saved_accum| {
                e.call(NI_POINTER_INIT, &args![saved_accum, accum_controllers]);
                let root = pointer_get(e, a + ANIM_ROOT);
                e.call(OBJECT_SET_CONTROLLERS, &args![root, 0u32]);
                let accum_root = e.mem.u32(a + ACCUM_ROOT);
                e.call(OBJECT_SET_CONTROLLERS, &args![accum_root, 0u32]);
                let root = pointer_get(e, a + ANIM_ROOT);
                // The root's `UpdateDownwardPass` (+0xa4): the scene family's dispatch, directly.
                scene::walk::downward(e, root, data.addr(), 0);
                let controllers = pointer_get(e, saved_root.addr());
                let root = pointer_get(e, a + ANIM_ROOT);
                e.call(OBJECT_SET_CONTROLLERS, &args![root, controllers]);
                let controllers = pointer_get(e, saved_accum.addr());
                let accum_root = e.mem.u32(a + ACCUM_ROOT);
                e.call(OBJECT_SET_CONTROLLERS, &args![accum_root, controllers]);
                e.call(NI_POINTER_RELEASE, &args![saved_accum]);
            });
            e.call(NI_POINTER_RELEASE, &args![saved_root]);
        });
    });
}
