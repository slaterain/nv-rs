//! The world frame's draw sequence (`00870bd0`, and its sibling `00870a00` for the frame with the
//! interface byte +0x131 set) with the set-up calls it makes first: the frame's begin (`00872ad0`),
//! the camera and values (`00872b00`), the water reflection's stages (`008727d0`), then the shadow
//! preparation and the target ([`super::shadows`], [`super::frame`]), the screen margins, the world
//! pass, water and interface passes, and the frame's wind-down.
//!
//! The exe's one- and two-load getters (the renderer and image-space globals, the root and its child,
//! the conversation target's camera port, the node flags, the settings' values, the multisample and
//! interface flags, the rendering system's address) are read in place; the passes themselves stay
//! calls. The port's step timers (`NV_STAGE_TIME`) and the world-drawn note are kept where the
//! translation has them.

use super::FAMILY;
use super::{frame, shadows};
use engine_core::ptr::Ptr;
use engine_core::stage_time::{self, Column};
use engine_core::typed::leaf;
use engine_core::{args, typed_call, Engine};

/// The renderer (`011f4748`), the image-space manager (`011f91ac`), the root `NiPointer`
/// (`011deb7c`), the world (`011dea10`) and player (`011dea3c`), the main object (`011dea0c`), the
/// interface flag (`011f91a4`), the picked reference (`011dea20`), the shadow scene node table
/// (`011f91c8`).
const RENDERER: u32 = 0x011f_4748;
const IMAGE_SPACE: u32 = 0x011f_91ac;
const ROOT: u32 = 0x011d_eb7c;
const WORLD: u32 = 0x011d_ea10;
const PLAYER: u32 = 0x011d_ea3c;
const MAIN_OBJECT: u32 = 0x011d_ea0c;
const INTERFACE_OFFSCREEN: u32 = 0x011f_91a4;
const PICKED: u32 = 0x011d_ea20;
const SCENE_NODE_TABLE: u32 = 0x011f_91c8;
/// The `Main` object's members: the world accumulator `NiPointer` (+0x88), the rendering-menu
/// byte (+0x9c).
const MAIN_WORLD_ACCUM: u32 = 0x88;
const MAIN_RENDERING_MENU: u32 = 0x9c;
/// The rendering system (`01200088`) and its stage calls.
const MT_SYSTEM: u32 = 0x0120_0088;
const MT_SET_THREAD_STAGE: u32 = 0x00ba_30f0;
const MT_ADVANCE_THREAD_STAGE: u32 = 0x00ba_3130;

/// The value of a byte setting (`00408d60` on a setting that is never null).
#[inline(always)]
fn setting_byte(e: &Engine, setting: u32) -> u8 {
    e.mem.u8(setting.wrapping_add(4))
}

/// Flag 1 of a node's flags (`00456610`) and its set (`00450f90`).
#[inline(always)]
fn flag(e: &Engine, node: u32) -> u8 {
    u8::from(e.mem.u32(node.wrapping_add(0x30)) & 1 != 0)
}
#[inline(always)]
fn set_flag(e: &mut Engine, node: u32, on: u8) {
    let at = node.wrapping_add(0x30);
    let flags = e.mem.u32(at);
    e.mem
        .set_u32(at, if on != 0 { flags | 1 } else { flags & !1 });
}

/// The root (`0045c670`) and an object's child at +0xac (`006629f0`).
#[inline(always)]
fn root(e: &Engine) -> u32 {
    e.mem.u32(ROOT)
}
#[inline(always)]
fn child(e: &Engine, object: u32) -> u32 {
    e.mem.u32(object.wrapping_add(0xac))
}

/// The world's word +0x34 (`005f36f0`).
#[inline(always)]
fn world_word(e: &Engine) -> u32 {
    let world = e.mem.u32(WORLD);
    e.mem.u32(world.wrapping_add(0x34))
}

fn both_threads(e: &mut Engine, stage: u32) {
    e.call(MT_SET_THREAD_STAGE, &args![MT_SYSTEM, 0u32, stage]);
    e.call(MT_ADVANCE_THREAD_STAGE, &args![MT_SYSTEM, 1u32, stage]);
}

/// `body` timed into the step `column` (off, one load).
fn timed<R>(column: Column, body: impl FnOnce() -> R) -> R {
    let _time = stage_time::scope(column);
    body()
}

// Typed from 00872ad0
/// `thiscall(main)`: when `00683a60` answers something, `00a81a80` if `00714a00` says so, then
/// `00714960(3)`.
pub fn frame_begin(e: &mut Engine, _this: Ptr) {
    if e.call(0x0068_3a60, &args![]).u32() != 0 {
        if e.call(0x0071_4a00, &args![]).bool() {
            e.call(0x00a8_1a80, &args![]);
        }
        e.call(0x0071_4960, &args![3u32]);
    }
}

/// The colour constructor (`00414430`) and the colour set on an object (`00712e60`).
const COLOR_CTOR: u32 = 0x0041_4430;
const SET_COLOR: u32 = 0x0071_2e60;
/// The update data's constructor (`0043d410`).
const UPDATE_DATA: u32 = 0x0043_d410;

// Typed from 00872b00
/// `thiscall(main)`: the per-frame camera and values; answers the world accumulator's object (see the
/// translation's account): the root's child gets the constant colour, the camera's field of view
/// (unless the state at `011f2250` +8 is 4) and the shader manager's, the rendering menu's colour,
/// the high actor culler (built on first use when its setting is on) run on the root's child, the two
/// `NiPointer` globals' values and updates, the flags and settings stored; the answer is the world's
/// object +0x68's word +0x28 when its mode is 3 or 2, else 0.
pub fn camera_setup(e: &mut Engine, this: Ptr) -> u32 {
    let root_child = child(e, root(e));
    e.call(SET_COLOR, &args![root_child, 0x011a_d840u32]);
    if typed_call!(
        leaf::FAMILY,
        0x0044_ddc0,
        leaf::word_at_8(e, 0x011f_2250u32)
    ) != 4
    {
        let player = e.mem.u32(PLAYER);
        let field_of_view = e.call(0x0071_0ab0, &args![player]).f32();
        let root = root(e);
        e.call(0x00c5_2020, &args![root, field_of_view, 0u32, 0u32, 1u32]);
    }
    let player = e.mem.u32(PLAYER);
    let field_of_view = e.call(0x0071_0ab0, &args![player]).f32();
    e.call(0x00b5_4000, &args![field_of_view]);
    if e.mem.u8(this.addr().wrapping_add(MAIN_RENDERING_MENU)) != 0 {
        e.with_stack(0x10, |e, color| {
            e.call(COLOR_CTOR, &args![color, 0.0f32, 1.0f32, 1.0f32, 0.0f32]);
            let root_child = child(e, root(e));
            e.call(SET_COLOR, &args![root_child, color]);
        });
    }
    let root_pointer = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, ROOT));
    let accumulator = child(e, root_pointer);
    if setting_byte(e, 0x011d_ed80) != 0 {
        let guard = e.global::<u32>(0x011d_efb4);
        if guard & 1 == 0 {
            e.set_global(0x011d_efb4, guard | 1);
            e.call(0x0087_2e00, &args![0x011d_ef10u32]);
            e.call(0x00ec_658f, &args![0x00fd_9380u32]);
        }
        e.call(0x008d_6f50, &args![0x011d_ef10u32, accumulator]);
    }
    for slot in [0x011d_eb34u32, 0x011d_eda4] {
        let root_pointer = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, ROOT));
        let node = e.call(0x0055_8310, &args![root_pointer]).u32();
        let value = node.wrapping_add(0x58);
        let target = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot));
        e.call(0x0044_0460, &args![target, value]);
        e.with_stack(0x10, |e, data| {
            e.call(UPDATE_DATA, &args![data, 0.0f32, 0u32, 0u32]);
            let target = typed_call!(leaf::FAMILY, 0x0055_9450, leaf::word_at_0(e, slot));
            typed_call!(
                crate::scene::FAMILY,
                0x00a5_9c60,
                crate::scene::walk::update(e, Ptr::new(target), data.addr())
            );
        });
    }
    if e.global::<u8>(0x011f_9fc0) != 0 {
        e.set_global(0x011f_9fc0, 0u8);
        e.set_global(0x011f_9fc1, 1u8);
    }
    let first = setting_byte(e, 0x011d_ecd8);
    e.set_global(0x011f_9fc3, first);
    e.set_global(0x011f_9fc4, 1u8);
    let second = setting_byte(e, 0x011d_eb60);
    e.set_global(0x011f_9fc5, second);
    e.set_global(0x011f_94a5, 0u8);

    let world = e.mem.u32(WORLD);
    let highest = typed_call!(leaf::FAMILY, 0x008d_8520, leaf::word_at_68(e, world));
    if highest == 0 || typed_call!(leaf::FAMILY, 0x0045_cd60, leaf::word_at_28(e, highest)) == 0 {
        return 0;
    }
    let world = e.mem.u32(WORLD);
    let object = typed_call!(leaf::FAMILY, 0x008d_8520, leaf::word_at_68(e, world));
    let matches = e.call(0x0047_6c90, &args![object]).u32() == 3 || {
        let world = e.mem.u32(WORLD);
        let object = typed_call!(leaf::FAMILY, 0x008d_8520, leaf::word_at_68(e, world));
        e.call(0x0047_6c90, &args![object]).u32() == 2
    };
    if !matches {
        return 0;
    }
    let world = e.mem.u32(WORLD);
    let object = typed_call!(leaf::FAMILY, 0x008d_8520, leaf::word_at_68(e, world));
    typed_call!(leaf::FAMILY, 0x0045_cd60, leaf::word_at_28(e, object))
}

/// The interface object (`00705910`), its test (`005585e0`) and the dialog byte (`011d9514`,
/// `00705100`).
const INTERFACE_OBJECT: u32 = 0x0070_5910;
const INTERFACE_TEST: u32 = 0x0055_85e0;
const IN_DIALOG: u32 = 0x011d_9514;

/// The test `00870a00` and `00870bd0` make before drawing the world.
fn world_draw_enabled(e: &mut Engine, forced_off: bool, other: u8) -> bool {
    if forced_off || other != 0 {
        return false;
    }
    let interface = e.call(INTERFACE_OBJECT, &args![]).u32();
    if e.call(INTERFACE_TEST, &args![interface]).u32() != 0 {
        return false;
    }
    e.global::<u8>(IN_DIALOG) == 0
}

/// Both Eyefinity bytes set (`004de080`).
fn eyefinity(e: &Engine) -> bool {
    e.global::<u8>(0x011c_70ed) != 0 && e.global::<u8>(0x011c_70ee) != 0
}

/// The HUD's screen margins wanted (`00709cb0`): the HUD menu (`011d96c0`) with its byte +0x1fc.
fn margins_wanted(e: &Engine) -> u8 {
    let menu = e.global::<u32>(0x011d_96c0);
    u8::from(menu != 0 && e.mem.u8(menu.wrapping_add(0x1fc)) != 0)
}

/// The low dword of a 64-bit `FISTP` in truncating rounding mode.
fn truncate_low_dword(value: f32) -> u32 {
    let whole = engine_core::x87::fistp_chop64(f64::from(value));
    u32::try_from(whole & 0xffff_ffff).unwrap_or(0)
}

// Typed from 00870bd0
/// `thiscall(main, target, skip_world)`: the world frame's draw sequence (see the translation's
/// account): the set-up calls, the shadow preparation, the target, the first-person passes, the
/// accumulator's owner flag forced on while the world pass runs, the screen margins, the water,
/// refraction and first-person passes, the picked reference's screen position, the interface, the
/// margins' end and the wind-down.
pub fn world_frame(e: &mut Engine, this: Ptr, target: u32, skip_world: u8) {
    let renderer = e.global::<u32>(RENDERER);
    let image_space = e.global::<u32>(IMAGE_SPACE);
    let player = e.mem.u32(PLAYER);
    let player_model = e.call(0x0095_0bb0, &args![player, 1u32]).u32();
    let flag_a = flag(e, player_model);
    let byte_flag = e.call(0x004d_c310, &args![]).u8();
    typed_call!(FAMILY, 0x0087_2ad0, frame_begin(e, this));
    let accumulator = typed_call!(FAMILY, 0x0087_2b00, camera_setup(e, this));
    typed_call!(FAMILY, 0x0087_27d0, water_stages(e, this));
    let accumulator_flag = e.call(0x0087_49b0, &args![this]).u8();
    typed_call!(FAMILY, 0x0087_1290, shadows::prepare(e, this));
    let frame = typed_call!(
        FAMILY,
        0x0087_2f50,
        frame::frame_target(e, this, renderer, target, 0, byte_flag)
    );
    e.call(0x0087_4ac0, &args![this, accumulator_flag]);
    e.call(0x0087_4b50, &args![this, frame, byte_flag, 5u32]);
    let draw_world = world_draw_enabled(e, flag_a != 0, skip_world);

    // The accumulator's owner flag is saved and forced on while the world is set up.
    let mut saved_owner_flag = 0u8;
    if draw_world && accumulator != 0 {
        let owner = e.call(0x0043_b230, &args![accumulator]).u32();
        saved_owner_flag = flag(e, owner);
        let owner = e.call(0x0043_b230, &args![accumulator]).u32();
        set_flag(e, owner, 1);
    }

    let margins_enabled = !eyefinity(e);
    let margins = margins_wanted(e);
    if margins != 0 {
        let width = e.call(0x004d_ee10, &args![renderer]).u32();
        let height = e.call(0x004d_ee70, &args![renderer]).u32();
        let aspect_setting = 0x011d_ea8cu32.wrapping_add(4);
        let aspect = e.call(0x0070_6e70, &args![]).f32();
        let scale: f64 = e.global(0x0101_1588);
        #[allow(clippy::cast_possible_truncation)] // the exe's float stores
        let factor = ((f64::from(aspect) * f64::from(e.mem.f32(aspect_setting))) * scale) as f32;
        #[allow(clippy::cast_possible_truncation)]
        let inner_width = (f64::from(width) * f64::from(factor)) as f32;
        #[allow(clippy::cast_possible_truncation)]
        let margin = (f64::from(width) - f64::from(inner_width)) as f32;
        if margins_enabled {
            e.call(
                0x00b4_f540,
                &args![
                    truncate_low_dword(inner_width),
                    0u32,
                    truncate_low_dword(margin),
                    height
                ],
            );
        }
    }

    e.call(
        0x0087_3200,
        &args![
            this,
            accumulator,
            u32::from(draw_world),
            0u32,
            accumulator_flag
        ],
    );
    // Port-added: the world is drawn (`engine_core::world_drawn`, as the translation notes it).
    engine_core::world_drawn::world_drawn(e);
    if draw_world && accumulator != 0 {
        let owner = e.call(0x0043_b230, &args![accumulator]).u32();
        set_flag(e, owner, saved_owner_flag);
    }

    let water_group = e.call(0x004e_bbc0, &args![image_space, 4u32]).u32();
    // `00871220`: the group's float +0x70 above the `double` zero.
    let water_active =
        f64::from(e.mem.f32(water_group.wrapping_add(0x70))) > e.global::<f64>(0x0101_2060);
    let bit_nine = e.call(0x00b8_b390, &args![image_space, 9u32]).bool();
    let reference_marker = bit_nine && world_word(e) == 0 && setting_byte(e, 0x011d_eadc) != 0;

    let mut water_drawn = false;
    if water_active && draw_world {
        water_drawn = true;
        timed(Column::FirstPersonNode, || {
            e.call(0x0087_46d0, &args![this, frame])
        });
        timed(Column::Targets, || {
            e.call(0x0087_4b50, &args![this, frame, byte_flag, 0u32])
        });
    }
    if e.call(0x005b_9b00, &args![]).bool() {
        let accumulator_slot = typed_call!(
            leaf::FAMILY,
            0x0055_9450,
            leaf::word_at_0(e, this.addr().wrapping_add(MAIN_WORLD_ACCUM))
        );
        let check = timed(Column::AccumulatorCheck, || {
            e.call(0x00b6_3a90, &args![accumulator_slot]).u32()
        });
        if check != 0 {
            let accumulator_slot = typed_call!(
                leaf::FAMILY,
                0x0055_9450,
                leaf::word_at_0(e, this.addr().wrapping_add(MAIN_WORLD_ACCUM))
            );
            timed(Column::RefractionNormals, || {
                e.call(0x0087_4920, &args![this, accumulator_slot, frame])
            });
        }
    }

    if draw_world {
        timed(Column::FirstPersonSetup, || {
            e.call(0x0087_4c10, &args![this])
        });
        timed(Column::Targets, || {
            e.call(0x0087_4b50, &args![this, frame, byte_flag, 0u32])
        });
        if !water_drawn {
            e.call(MT_ADVANCE_THREAD_STAGE, &args![MT_SYSTEM, 1u32, 0x16u32]);
        }
        timed(Column::DrawRest, || {
            e.call(
                0x0087_5110,
                &args![this, renderer, 0u32, accumulator, frame],
            )
        });
    } else {
        both_threads(e, 0x16);
    }

    if water_active {
        let accumulator_slot = typed_call!(
            leaf::FAMILY,
            0x0055_9450,
            leaf::word_at_0(e, this.addr().wrapping_add(MAIN_WORLD_ACCUM))
        );
        timed(Column::WaterPass, || {
            e.call(0x0087_5e40, &args![this, accumulator_slot, frame])
        });
    }

    if reference_marker && world_word(e) == 0 {
        timed(Column::TargetMarker, || screen_position_of_reference(e));
    }

    if e.global::<u8>(INTERFACE_OFFSCREEN) != 0 {
        timed(Column::Interface, || {
            e.call(
                0x0087_5fd0,
                &args![this, byte_flag, frame, renderer, target],
            )
        });
        let held = typed_call!(
            leaf::FAMILY,
            0x0055_9450,
            leaf::word_at_0(e, 0x011c_7810u32)
        );
        let held_set = held != 0 && {
            let object = typed_call!(
                leaf::FAMILY,
                0x0055_9450,
                leaf::word_at_0(e, 0x011c_7810u32)
            );
            e.call(0x0045_3470, &args![object]).u32() != 0
        };
        if (held_set || e.call(0x005b_ac20, &args![]).bool())
            && !e.call(0x0070_5a00, &args![]).bool()
        {
            timed(Column::InterfaceEnd, || {
                e.call(0x0087_6a20, &args![this, renderer])
            });
        }
    } else if target != 0 {
        timed(Column::InterfaceTarget, || {
            e.call(0x0087_6830, &args![this, byte_flag, target])
        });
    }

    let margins_time = (margins != 0).then(|| stage_time::scope(Column::Margins));
    if margins != 0 {
        if margins_enabled {
            e.call(0x00b5_40d0, &args![]);
        }
        if eyefinity(e) {
            let renderer_object = e.global::<u32>(RENDERER);
            e.vcall(renderer_object, 0xac, &args![0x011a_9bd0u32]);
            e.with_stack(0x10, |e, color| {
                let at = color.addr();
                e.call(COLOR_CTOR, &args![at, 0.0f32, 0.0f32, 0.0f32, 0.0f32]);
                e.mem.set_f32(at, 0.0);
                let second: f32 = e.global(0x0102_6994);
                e.mem.set_f32(at + 4, second);
                e.mem.set_f32(at + 8, 1.0);
                e.mem.set_f32(at + 12, 0.0);
                let renderer_object = e.global::<u32>(RENDERER);
                e.call(0x0071_48c0, &args![renderer_object, at, 1u32]);
                let first: f32 = e.global(0x0103_40a4);
                e.mem.set_f32(at, first);
                e.mem.set_f32(at + 4, 1.0);
                e.mem.set_f32(at + 8, 1.0);
                e.mem.set_f32(at + 12, 0.0);
                let renderer_object = e.global::<u32>(RENDERER);
                e.call(0x0071_48c0, &args![renderer_object, at, 1u32]);
            });
        }
    }
    drop(margins_time);
    timed(Column::WindDown, || {
        e.call(
            0x0087_6850,
            &args![this, frame, image_space, renderer, accumulator],
        )
    });
}

/// The picked reference's screen position: its node's position projected through the camera root's
/// child (`00a6fdb0`) and handed to the image-space manager (`00b8b1e0`) as a two-float value.
fn screen_position_of_reference(e: &mut Engine) {
    let picked = e.mem.u32(PICKED);
    if picked == 0 || typed_call!(leaf::FAMILY, 0x0045_cd60, leaf::word_at_28(e, picked)) == 0 {
        return;
    }
    let picked = e.mem.u32(PICKED);
    let reference = typed_call!(leaf::FAMILY, 0x0045_cd60, leaf::word_at_28(e, picked));
    let node = e.vcall(reference, 4, &args![]).u32();
    let position = node.wrapping_add(0x8c);
    e.with_stack(0x20, |e, block| {
        let at = block.addr();
        for i in 0..3 {
            let word = e.mem.u32(position.wrapping_add(4 * i));
            e.mem.set_u32(at + 4 * i, word);
        }
        let (output_a, output_b, output_c) = (at + 0xc, at + 0x10, at + 0x14);
        e.mem.set_f32(output_a, 0.0);
        e.mem.set_f32(output_b, 0.0);
        e.mem.set_f32(output_c, 0.0);
        let camera_child = child(e, root(e));
        let last_argument: f32 = e.global(0x0107_18c0);
        e.call(
            0x00a6_fdb0,
            &args![
                camera_child,
                at,
                output_a,
                output_b,
                output_c,
                last_argument
            ],
        );
        let value_a = e.mem.f32(output_a);
        let value_b = e.mem.f32(output_b);
        e.with_stack(8, |e, pair| {
            let pair = e.call(0x0045_2dc0, &args![pair, value_a, value_b]).u32();
            let image_space = e.global::<u32>(IMAGE_SPACE);
            e.call(0x00b8_b1e0, &args![image_space, pair]);
        });
    });
}

// Typed from 008727d0
/// `thiscall(main)`: with the water reflections' setting on, the reflection pass (the world's water
/// object drawn into the scene node with its flag forced) or its state reset; then the stages after
/// the water object's first word, up to 0x10, on both threads.
pub fn water_stages(e: &mut Engine, _this: Ptr) {
    if setting_byte(e, 0x011c_7adc) != 0 {
        let saved = e.global::<u32>(0x011a_9608);
        e.set_global(0x011a_9608, 1u32);
        let blocked = e.call(0x0054_68f0, &args![0xbu32]).bool();
        let first_path = !blocked && {
            let word = world_word(e);
            word == 0 || e.call(0x0045_18e0, &args![word]).bool()
        };
        if first_path {
            let scene = e.global::<u32>(SCENE_NODE_TABLE);
            let saved_flag = e.call(0x005f_9ad0, &args![scene]).u8();
            e.call(0x005f_9af0, &args![scene, 1u32]);
            let sky = child(e, root(e));
            let world = e.mem.u32(WORLD);
            let water = e.call(0x0070_ec90, &args![world]).u32();
            e.call(0x004e_1bc0, &args![water, sky, scene]);
            e.call(0x005f_9af0, &args![scene, u32::from(saved_flag)]);
        } else {
            e.set_global(0x011f_f104, 0u8);
            let world = e.mem.u32(WORLD);
            let water = e.call(0x0070_ec90, &args![world]).u32();
            e.mem.set_u32(water, 0);
            let main_object = e.mem.u32(MAIN_OBJECT);
            let reflection = e.call(0x004e_2190, &args![main_object]).u32();
            e.call(0x004e_2120, &args![reflection, 0u32]);
            e.set_global(0x011f_f8c4, 0u8);
            let world = e.mem.u32(WORLD);
            e.call(0x0070_ec90, &args![world]);
            e.set_global(0x011c_7a59, 0u8);
        }
        e.set_global(0x011a_9608, saved);
    }
    let world = e.mem.u32(WORLD);
    let water = e.call(0x0070_ec90, &args![world]).u32();
    let mut stage = e.mem.i32(water).wrapping_add(1);
    while stage < 0x11 {
        both_threads(e, u32::from_ne_bytes(stage.to_ne_bytes()));
        stage += 1;
    }
}
