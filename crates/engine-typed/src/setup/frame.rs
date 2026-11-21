//! The frame's render target (`00872f50`), what each draw sequence picks before its passes: a new
//! offscreen target when the interface draws into one, otherwise the screen's or the default group.
//!
//! The getters it calls (the interface flag `008709f0`, the texture manager `004a0ea0`, the camera
//! root's child `00524c90`) are read in place; the target's size slots, the manager's calls and the profiler scope stay calls.
//!
//! **Floats**: the size ratios are computed in `double`s as the translation computes them and handed
//! to `008731a0`, then stored to a float, so the differential test compares exactly. (The exe's
//! main-thread 24-bit x87 would round the product and the quotient to a float's precision each; the
//! ratio is a screen-size scale stored as a float, and the difference is at most a float step, which
//! no picture shows.) The world frame's margin factor (`world`) is kept the same way.

use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The interface's offscreen flag (`011f91a4`, `008709f0`) and the texture manager (`011f91a8`).
const INTERFACE_OFFSCREEN: u32 = 0x011f_91a4;
const TEXTURE_MANAGER: u32 = 0x011f_91a8;
/// The manager's target creation (`00b6e110`), the target's note (`008731d0`) and its global
/// (`011f9438`).
const CREATE_TARGET: u32 = 0x00b6_e110;
const NOTE_TARGET: u32 = 0x0087_31d0;
const CURRENT_TARGET: u32 = 0x011f_9438;
/// The renderer's width and height (`004d_ee10`, `004d_ee70`); the target's size slots (+0x94,
/// +0x98 of the object its `NiPointer` at +0x30 holds).
const RENDERER_WIDTH: u32 = 0x004d_ee10;
const RENDERER_HEIGHT: u32 = 0x004d_ee70;
/// The ratio's scale (`double`, `01082d20`), its clamp (`008731a0`), the colour's constructor
/// (`00414430`), the camera root (`011deb7c`) and the colour set (`00712e60`).
const RATIO_SCALE: u32 = 0x0108_2d20;
const RATIO_CLAMP: u32 = 0x0087_31a0;
const COLOR_CTOR: u32 = 0x0041_4430;
const CAMERA_ROOT: u32 = 0x011d_eb7c;
const SET_COLOR: u32 = 0x0071_2e60;
/// The group calls: the target's group (`00b6b260`), make a group current (`00b6b8d0`), the default
/// (`00b6b890`); the renderer's group slot (+0xc8).
const TARGET_GROUP: u32 = 0x00b6_b260;
const SET_GROUP: u32 = 0x00b6_b8d0;
const SET_DEFAULT_GROUP: u32 = 0x00b6_b890;
const SLOT_RENDERER_GROUP: u32 = 0xc8;
/// The profiler scope's constructor and destructor and the source file's name.
const PROFILE_SCOPE_CTOR: u32 = 0x0040_4eb0;
const PROFILE_SCOPE_DTOR: u32 = 0x0040_4ee0;
const SOURCE_FILE_NAME: u32 = 0x0108_29c4;

/// `body` between a profiler scope guard's constructor and destructor.
fn with_profile_scope<R>(
    e: &mut Engine,
    kind: u32,
    line: u32,
    body: impl FnOnce(&mut Engine) -> R,
) -> R {
    e.with_stack(0x10, |e, guard| {
        e.call(
            PROFILE_SCOPE_CTOR,
            &args![guard, kind, 1u32, SOURCE_FILE_NAME, line],
        );
        let result = body(e);
        e.call(PROFILE_SCOPE_DTOR, &args![guard]);
        result
    })
}

/// The target's width (`slot` 0x94) or height (0x98): the slot of the object its first `NiPointer`
/// (+0x30) holds, 0 when empty (`00872370`, `008723d0` with index 0).
fn target_size(e: &mut Engine, target: u32, slot: u32) -> u32 {
    let object = typed_call!(
        leaf::FAMILY,
        0x0055_9450,
        leaf::word_at_0(e, target.wrapping_add(0x30))
    );
    if object != 0 {
        e.vcall(object, slot, &args![]).u32()
    } else {
        0
    }
}

/// `(size * scale) / target`, clamped by `008731a0` against 1.0, as a float.
fn ratio(e: &mut Engine, size: u32, target: u32) -> f32 {
    let scale: f64 = e.global(RATIO_SCALE);
    let ratio = (f64::from(size) * scale) / f64::from(target);
    #[allow(clippy::cast_possible_truncation)] // the exe's `FSTP dword`
    {
        e.call(RATIO_CLAMP, &args![ratio, 1.0f32]).f64() as f32
    }
}

// Typed from 00872f50
/// `thiscall(main, renderer, screen, skip_new_target, use_renderer_group)`: with the interface drawing
/// offscreen and no `skip_new_target`, a new target from the texture manager (kind 4) is noted and
/// made current; when its size differs from the renderer's, the camera root's child gets the clamped
/// width and height ratios as a colour. Otherwise the screen's group (or the renderer's own) is made
/// current, or the default one without a screen. Answers the new target (0 for none).
pub fn frame_target(
    e: &mut Engine,
    _this: Ptr,
    renderer: u32,
    screen: u32,
    skip_new_target: u8,
    use_renderer_group: u8,
) -> u32 {
    let mut created = 0u32;
    if e.global::<u8>(INTERFACE_OFFSCREEN) != 0 && skip_new_target == 0 {
        with_profile_scope(e, 0x27, 0x1e6d, |e| {
            let factory = e.global::<u32>(TEXTURE_MANAGER);
            created = e
                .call(
                    CREATE_TARGET,
                    &args![factory, renderer, 4u32, 0u32, 0u32, 0u32],
                )
                .u32();
            e.call(NOTE_TARGET, &args![created]);
            e.set_global(CURRENT_TARGET, created);
            let width = e.call(RENDERER_WIDTH, &args![renderer]).u32();
            let mut changed = width != target_size(e, created, 0x94);
            if !changed {
                let height = e.call(RENDERER_HEIGHT, &args![renderer]).u32();
                changed = height != target_size(e, created, 0x98);
            }
            if changed {
                let width = e.call(RENDERER_WIDTH, &args![renderer]).u32();
                let target_width = target_size(e, created, 0x94);
                let width_ratio = ratio(e, width, target_width);
                let height = e.call(RENDERER_HEIGHT, &args![renderer]).u32();
                let target_height = target_size(e, created, 0x98);
                let height_ratio = ratio(e, height, target_height);
                e.with_stack(0x10, |e, color| {
                    e.call(
                        COLOR_CTOR,
                        &args![color, 0.0f32, width_ratio, height_ratio, 0.0f32],
                    );
                    // `00524c90`: the camera root's (`NiPointer` `011deb7c`) child at +0xac.
                    let root = e.mem.u32(CAMERA_ROOT);
                    let child = e.mem.u32(root.wrapping_add(0xac));
                    e.call(SET_COLOR, &args![child, color]);
                });
            }
            let group = e.call(TARGET_GROUP, &args![created]).u32();
            e.call(SET_GROUP, &args![7u32, group]);
        });
    } else {
        with_profile_scope(e, 0x26, 0x1e84, |e| {
            if screen != 0 {
                let group = if use_renderer_group != 0 {
                    e.vcall(renderer, SLOT_RENDERER_GROUP, &args![]).u32()
                } else {
                    e.call(TARGET_GROUP, &args![screen]).u32()
                };
                e.call(SET_GROUP, &args![7u32, group]);
            } else {
                e.call(SET_DEFAULT_GROUP, &args![7u32, 0u32]);
            }
        });
    }
    created
}
