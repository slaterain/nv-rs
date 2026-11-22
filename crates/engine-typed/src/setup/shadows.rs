//! The frame's shadow preparation (`00871290`, from the draw sequences `00870bd0`, `00870a00` and
//! `008707c0`): the depth-pass state's start and end, the shadow casters picked (the player, then the
//! actors of the high process list in range, up to the limit) and the walk over the scene node's
//! shadow lights, each one reset, recomputed and queued or switched off.
//!
//! The exe's small getters this code calls (the render flags, the shadow scene node table, the
//! renderer globals, the settings' values, the player's and actors' words, the lights' members, the
//! node flag test and set, the rendering system's address) are read in place: their bodies are one
//! or two loads or stores, and the differential test leaves them out of the translation's calls. What
//! stays a call: the depth-pass test (`005bf7b0`, it asks the renderer), the shadow scene node's own
//! passes, the actor tests, the lock, the lights' updates and the rendering system's stages.
//!
//! **Floats**: the caster's range test and the light's weight cut-off compare a stored float with a
//! `double`. The range test's product (`largest * scale`) stays a `double` as the translation computes
//! it, so the differential test compares exactly; the exe's main-thread 24-bit `FMUL` would round it
//! to a float's precision, which can only move the test for a radius within half a float step of the
//! range (a caster's shadow at that exact distance), not anything a player sees.

use super::lists;
use super::FAMILY;
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet;
use engine_core::{args, typed_call, Engine};

/// The depth-pass test (`005bf7b0`, a bool: the renderer's level and render flag 0x10).
const DEPTH_PASSES_WANTED: u32 = 0x005b_f7b0;
/// The render flags (`011f91d8`): bit 0x10 the depth passes (`005bf770` sets or clears it).
const RENDER_FLAGS: u32 = 0x011f_91d8;
/// The byte the depth-pass state is remembered in (`011def08`) and the byte `00525420` reads.
const DEPTH_STATE: u32 = 0x011d_ef08;
const DEPTH_REQUEST: u32 = 0x011d_ea2a;
/// The shadow scene node table (`011f91c8`, `00450b80`'s entry 0).
const SCENE_NODE_TABLE: u32 = 0x011f_91c8;
/// The renderer (`011f4748`) and the texture manager (`011f91a8`).
const RENDERER: u32 = 0x011f_4748;
const TEXTURE_MANAGER: u32 = 0x011f_91a8;
/// The shadow scene node's set-up passes and the texture manager's shadow-map count.
const SCENE_PREPARE: u32 = 0x00b5_b1f0;
const SHADOW_MAPS_SET: u32 = 0x00b6_ee70;
const SHADOW_PASS_BEGIN: u32 = 0x00b4_f570;
const SCENE_DYNAMIC_SHADOWS: u32 = 0x00b5_b880;
/// The byte settings (each value at +4): the shadows' switch, the player's flag and the light's add.
const SETTING_SHADOWS: u32 = 0x011c_74d4;
const SETTING_KEEP_SAVED: u32 = 0x011c_71dc;
const SETTING_ADD_OBJECTS: u32 = 0x011c_7664;
/// The shadow-caster limit's settings (`dword`s): interior and exterior.
const SETTING_LIMIT_INTERIOR: u32 = 0x011c_72d8;
const SETTING_LIMIT_EXTERIOR: u32 = 0x011c_72e4;
/// The world (`011dea10`, its word +0x34 by `005f36f0`) and the player (`011dea3c`).
const WORLD: u32 = 0x011d_ea10;
const PLAYER: u32 = 0x011d_ea3c;
/// The rendering system (`01200088`, `004ea970`) and its stage calls.
const MT_SYSTEM: u32 = 0x0120_0088;
const MT_SET_THREAD_STAGE: u32 = 0x00ba_30f0;
const MT_ADVANCE_THREAD_STAGE: u32 = 0x00ba_3130;
/// `Main::UpdateOffscreenInterface` (`00871a50`).
const UPDATE_OFFSCREEN_INTERFACE: u32 = 0x0087_1a50;
/// The casters: the player's tests (its byte +0x64a, the vanity byte `011e07b8`, `Actor::IsRefractive`
/// `008c51c0`), its 3D (`00950bb0`), the high process list (`011e0e80`: its array at +0x88, its
/// count at +0x150) under the lock `011f1180` (`0040fbf0`, `0040fba0`), the actor's test
/// (`0087f200`), the extra key (`012031e4`), the process's node (`00a5bdd0`), the zero point
/// (`011f426c`) and the range scale (`01021928`).
const PLAYER_SEEN: u32 = 0x64a;
const VANITY_ACTIVE: u32 = 0x011e_07b8;
const IS_REFRACTIVE: u32 = 0x008c_51c0;
const PLAYER_3D: u32 = 0x0095_0bb0;
const HIGH_PROCESS_LIST: u32 = 0x011e_0e80;
const LIST_LOCK: u32 = 0x011f_1180;
const LOCK_ACQUIRE: u32 = 0x0040_fbf0;
const LOCK_RELEASE: u32 = 0x0040_fba0;
const ACTOR_CASTS: u32 = 0x0087_f200;
const EXTRA_KEY: u32 = 0x0120_31e4;
const PROCESS_NODE: u32 = 0x00a5_bdd0;
const ZERO_POINT: u32 = 0x011f_426c;
const RANGE_SCALE: u32 = 0x0102_1928;
/// Actor slots: the process (+0x1d0) and the kind (+0x214).
const SLOT_PROCESS: u32 = 0x1d0;
const SLOT_KIND: u32 = 0x214;
/// The camera root (`NiPointer` `011deb7c`), its conversation target (+0xb4) and sky (+0xac); the
/// culling camera set-up (`00a694a0`); the saved object (`011ca438`) and the water byte (`011ad7b4`).
const CAMERA_ROOT: u32 = 0x011d_eb7c;
const CAMERA_SET_UP: u32 = 0x00a6_94a0;
const SAVED_OBJECT: u32 = 0x011c_a438;
const WATER_BYTE: u32 = 0x011a_d7b4;
/// The light's calls: the fade restart (`00b9bb10`), the position (`00b9d150`), the camera
/// (`00b9e970`), the slot (`00b9dfc0`), the portal graph (`00b9fba0`), the end of the update
/// (`00ba02b0`); the update data (`0043d410`); the object lock (`00454b30`); the scene node's light
/// update (`00b5d300`); the weight cut-off (`01082cb0`).
const LIGHT_RESTART: u32 = 0x00b9_bb10;
const LIGHT_POSITION: u32 = 0x00b9_d150;
const LIGHT_CAMERA: u32 = 0x00b9_e970;
const LIGHT_SLOT: u32 = 0x00b9_dfc0;
const LIGHT_GRAPH: u32 = 0x00b9_fba0;
const LIGHT_DONE: u32 = 0x00ba_02b0;
const UPDATE_DATA: u32 = 0x0043_d410;
const OBJECT_LOCK: u32 = 0x0045_4b30;
const SCENE_LIGHT_UPDATE: u32 = 0x00b5_d300;
const WEIGHT_CUTOFF: u32 = 0x0108_2cb0;
/// The light's fade test (`004e6580`): faded when the elapsed time (+0xdc) is at least the fade time
/// (`011ad834`) and the target (+0xd8) at most 0.1 (`0101ffa0`).
const FADE_TIME: u32 = 0x011a_d834;
const FADE_LIMIT: u32 = 0x0101_ffa0;

/// The value of a byte setting (`00408d60` on a setting that is never null).
#[inline(always)]
fn setting_byte(e: &Engine, setting: u32) -> u8 {
    e.mem.u8(setting.wrapping_add(4))
}

/// Flag 1 of a node's `m_uFlags` (`00456610`).
#[inline(always)]
fn flag(e: &Engine, node: u32) -> bool {
    e.mem.u32(node.wrapping_add(0x30)) & 1 != 0
}

/// Sets or clears flag 1 of a node (`00450f90`, through `0043b370`).
#[inline(always)]
fn set_flag(e: &mut Engine, node: u32, on: u8) {
    let at = node.wrapping_add(0x30);
    let flags = e.mem.u32(at);
    let flags = if on != 0 { flags | 1 } else { flags & !1 };
    e.mem.set_u32(at, flags);
}

/// Sets or clears render flag 0x10 (`005bf770`).
#[inline(always)]
fn set_depth_flag(e: &mut Engine, on: bool) {
    let flags = e.global::<u32>(RENDER_FLAGS);
    let flags = if on { flags | 0x10 } else { flags & !0x10 };
    e.set_global(RENDER_FLAGS, flags);
}

/// `MTRenderingSystem::SetThreadStage(thread, stage)`.
fn set_stage(e: &mut Engine, thread: u32, stage: u32) {
    e.call(MT_SET_THREAD_STAGE, &args![MT_SYSTEM, thread, stage]);
}

/// The shadow-caster limit: the interior setting when the world's +0x34 is set, else the exterior one.
fn caster_limit(e: &Engine) -> u32 {
    let world = e.mem.u32(WORLD);
    let setting = if e.mem.u32(world.wrapping_add(0x34)) != 0 {
        SETTING_LIMIT_INTERIOR
    } else {
        SETTING_LIMIT_EXTERIOR
    };
    e.mem.u32(setting.wrapping_add(4))
}

/// The texture manager's shadow-map count call with the renderer and `count`.
fn set_shadow_maps(e: &mut Engine, count: u32) {
    let renderer = e.global::<u32>(RENDERER);
    let factory = e.global::<u32>(TEXTURE_MANAGER);
    e.call(SHADOW_MAPS_SET, &args![factory, renderer, count]);
}

// Typed from 00871290
/// The per-frame shadow preparation (`thiscall(main)`). With depth passes wanted and requested, the
/// state is remembered, render flag 0x10 cleared, the scene node prepared (`00b5b1f0`) and the
/// texture manager told no shadow maps; when the request ends, the flag comes back and the manager
/// gets the caster limit. Then, with depth passes and shadows on: the shadow pass begins, the lit
/// lights' maps are dropped, the manager gets the limit, and with no limit the scene node's maps go
/// (`00b5c3c0`), else the casters are picked and the lights walked; with either off, stages 0x11 to
/// 0x14 run on both threads. Ends with `Main::UpdateOffscreenInterface`.
pub fn prepare(e: &mut Engine, this: Ptr) {
    if e.call(DEPTH_PASSES_WANTED, &args![]).bool() && e.global::<u8>(DEPTH_REQUEST) != 0 {
        let value = e.call(DEPTH_PASSES_WANTED, &args![]).u8();
        e.set_global(DEPTH_STATE, value);
        set_depth_flag(e, false);
        let scene = e.global::<u32>(SCENE_NODE_TABLE);
        e.call(SCENE_PREPARE, &args![scene]);
        set_shadow_maps(e, 0);
    } else if e.global::<u8>(DEPTH_STATE) != 0 && e.global::<u8>(DEPTH_REQUEST) == 0 {
        e.set_global(DEPTH_STATE, 0u8);
        set_depth_flag(e, true);
        let limit = caster_limit(e);
        set_shadow_maps(e, limit);
    }

    if e.call(DEPTH_PASSES_WANTED, &args![]).bool() && setting_byte(e, SETTING_SHADOWS) != 0 {
        e.call(SHADOW_PASS_BEGIN, &args![]);
        let scene = e.global::<u32>(SCENE_NODE_TABLE);
        typed_call!(
            FAMILY,
            0x00b5_b060,
            lists::drop_lit_shadow_maps(e, Ptr::new(scene))
        );
        let limit = caster_limit(e);
        set_shadow_maps(e, limit);
        if limit == 0 {
            typed_call!(
                FAMILY,
                0x00b5_c3c0,
                lists::drop_shadow_maps(e, Ptr::new(scene))
            );
        } else {
            casters_and_lights(e, scene, i32::from_ne_bytes(limit.to_ne_bytes()));
        }
    } else {
        for stage in 0x11..0x15u32 {
            set_stage(e, 0, stage);
            e.call(MT_ADVANCE_THREAD_STAGE, &args![MT_SYSTEM, 1u32, stage]);
        }
    }
    e.call(UPDATE_OFFSCREEN_INTERFACE, &args![this]);
}

/// The casters (the player's 3D unless it is hidden from shadows, then the actors in range, up to
/// `limit`; each gets its shadow light), the sort of the shadow lights, and the walk over them with
/// the camera set up for the conversation target.
fn casters_and_lights(e: &mut Engine, scene: u32, limit: i32) {
    let mut first_caster = 0u32;
    let mut chosen = 0i32;
    let player = e.mem.u32(PLAYER);
    if e.mem.u8(player.wrapping_add(PLAYER_SEEN)) != 0 || e.global::<u8>(VANITY_ACTIVE) != 0 {
        let player = e.mem.u32(PLAYER);
        if !e.call(IS_REFRACTIVE, &args![player]).bool() {
            let player = e.mem.u32(PLAYER);
            let model = e.call(PLAYER_3D, &args![player, 0u32]).u32();
            first_caster = typed_call!(
                FAMILY,
                0x00b5_cbd0,
                lists::shadow_light_of(e, Ptr::new(scene), model)
            );
            chosen += 1;
        }
    }
    e.call(LOCK_ACQUIRE, &args![LIST_LOCK, 0u32]);
    let actors = HIGH_PROCESS_LIST + 0x88;
    let count = e.mem.i32(HIGH_PROCESS_LIST + 0x150);
    let mut index = 0i32;
    while index < count && chosen < limit {
        let actor = e
            .mem
            .u32(actors.wrapping_add(u32::from_ne_bytes(index.to_ne_bytes()).wrapping_mul(4)));
        index += 1;
        if e.vcall(actor, SLOT_PROCESS, &args![]).u32() == 0 {
            continue;
        }
        let process = e.vcall(actor, SLOT_PROCESS, &args![]).u32();
        if e.mem.u32(process.wrapping_add(0x18)) == 0 {
            continue;
        }
        if !e.call(ACTOR_CASTS, &args![actor]).bool() {
            continue;
        }
        if e.vcall(actor, SLOT_KIND, &args![]).u32() == 4 {
            continue;
        }
        if e.call(IS_REFRACTIVE, &args![actor]).bool() {
            continue;
        }
        let key = e.global::<u32>(EXTRA_KEY);
        let process = e.vcall(actor, SLOT_PROCESS, &args![]).u32();
        let node = e.call(PROCESS_NODE, &args![process, key]).u32();
        let position = if node != 0 {
            node.wrapping_add(0x18)
        } else {
            ZERO_POINT
        };
        let x = e.mem.f32(position);
        let y = e.mem.f32(position.wrapping_add(4));
        let z = e.mem.f32(position.wrapping_add(8));
        // `00404010` twice: the greater float, the second on an unordered compare, quieted.
        let partial = x87_quiet(if y < x { x } else { y });
        let largest = x87_quiet(if z < partial { partial } else { z });
        let threshold = f64::from(largest) * e.global::<f64>(RANGE_SCALE);
        let process = e.vcall(actor, SLOT_PROCESS, &args![]).u32();
        let bound = typed_call!(leaf::FAMILY, 0x0043_d450, leaf::world_bound(e, process));
        let radius = f64::from(e.mem.f32(bound.wrapping_add(0xc)));
        if radius < threshold {
            let process = e.vcall(actor, SLOT_PROCESS, &args![]).u32();
            typed_call!(
                FAMILY,
                0x00b5_cbd0,
                lists::shadow_light_of(e, Ptr::new(scene), process)
            );
            chosen += 1;
        }
    }
    e.call(LOCK_RELEASE, &args![LIST_LOCK]);
    typed_call!(
        FAMILY,
        0x00b5_cde0,
        lists::sort_shadow_lights(e, Ptr::new(scene), first_caster)
    );

    let player = e.mem.u32(PLAYER);
    let player_model = e.call(PLAYER_3D, &args![player, 1u32]).u32();
    if player_model == 0 {
        return;
    }
    let root = e.mem.u32(CAMERA_ROOT);
    let target = e.mem.u32(root.wrapping_add(0xb4));
    let sky = e.mem.u32(root.wrapping_add(0xac));
    e.mem.set_u32(target.wrapping_add(0xc), sky);
    let sky = e.mem.u32(root.wrapping_add(0xac));
    e.call(CAMERA_SET_UP, &args![target, sky.wrapping_add(0xdc)]);
    let model_flag = u8::from(flag(e, player_model));
    set_flag(e, player_model, 1);
    let saved = e.global::<u32>(SAVED_OBJECT);
    e.set_global(WATER_BYTE, 0u8);
    let mut saved_flag = 0u8;
    if saved != 0 {
        saved_flag = u8::from(flag(e, saved));
        if setting_byte(e, SETTING_KEEP_SAVED) == 0 {
            set_flag(e, saved, 1);
        }
    }

    let mut light = typed_call!(
        FAMILY,
        0x00b5_afc0,
        lists::first_shadow_light(e, Ptr::new(scene))
    );
    let mut processed = 0i32;
    while light != 0 {
        shadow_light(e, light, target, limit, &mut processed);
        light = typed_call!(
            FAMILY,
            0x00b5_b010,
            lists::next_shadow_light(e, Ptr::new(scene))
        );
    }
    while processed + 0x11 < 0x15 {
        set_stage(e, 0, u32::from_ne_bytes((processed + 0x11).to_ne_bytes()));
        processed += 1;
    }
    if saved != 0 {
        set_flag(e, saved, saved_flag);
    }
    e.set_global(WATER_BYTE, 1u8);
    e.call(SCENE_DYNAMIC_SHADOWS, &args![scene, target]);
    set_flag(e, player_model, model_flag);
}

/// The light's fade test (`004e6580`, false when either float is not a number).
fn faded(e: &Engine, light: u32) -> bool {
    let time = e.global::<f32>(FADE_TIME);
    #[allow(clippy::neg_cmp_op_on_partial_ord)] // false for a NaN, as the x87 test is
    if !(time <= e.mem.f32(light.wrapping_add(0xdc))) {
        return false;
    }
    f64::from(e.mem.f32(light.wrapping_add(0xd8))) <= e.global::<f64>(FADE_LIMIT)
}

/// One light of the walk: a light with geometry (`spLight`, +0xf8) and a source (`spShadowScene`,
/// +0x128) that has not faded and whose geometry is culled is restarted (culled again when its
/// source is culled too; else its geometry shown while under the limit). Then, under the limit with
/// neither culled, it is recomputed (position from the source's, camera) and, when its count (+0x110)
/// is not 0xff and its weight (+0xd4) reaches the cut-off, slotted and queued (the scene node's light
/// update, the source's objects added when the setting says so). Any light not queued has its
/// geometry culled.
fn shadow_light(e: &mut Engine, light: u32, target: u32, limit: i32, processed: &mut i32) {
    let geometry_at = light.wrapping_add(0xf8);
    let source_at = light.wrapping_add(0x128);
    if e.mem.u32(geometry_at) == 0 || e.mem.u32(source_at) == 0 {
        return;
    }
    if !faded(e, light) {
        let geometry = e.mem.u32(geometry_at);
        if flag(e, geometry) {
            let source = e.mem.u32(source_at);
            if flag(e, source) {
                e.call(LIGHT_RESTART, &args![light, 0.0f32, 1u32]);
            } else {
                if *processed < limit {
                    let geometry = e.mem.u32(geometry_at);
                    set_flag(e, geometry, 0);
                }
                e.call(LIGHT_RESTART, &args![light, 0.0f32, 0u32]);
            }
        }
    }

    let mut queued = false;
    let usable =
        *processed < limit && !flag(e, e.mem.u32(geometry_at)) && !flag(e, e.mem.u32(source_at));
    if usable {
        let source = e.mem.u32(source_at);
        // The source's bound is copied to a local the code never reads.
        typed_call!(leaf::FAMILY, 0x0043_d450, leaf::world_bound(e, source));
        let position = e.mem.u32(source_at).wrapping_add(0x8c);
        let words = [
            e.mem.u32(position),
            e.mem.u32(position.wrapping_add(4)),
            e.mem.u32(position.wrapping_add(8)),
        ];
        e.call(
            LIGHT_POSITION,
            &args![light, 0u32, words[0], words[1], words[2]],
        );
        e.call(LIGHT_CAMERA, &args![light, target]);
        if e.mem.u16(light.wrapping_add(0x110)) != 0xff {
            let weight = f64::from(e.mem.f32(light.wrapping_add(0xd4)));
            // `FCOMP` then `TEST AH,1`: skipped when below the cut-off or unordered.
            if weight >= e.global::<f64>(WEIGHT_CUTOFF) {
                *processed += 1;
                e.with_stack(0x10, |e, data| {
                    e.call(UPDATE_DATA, &args![data, 0.0f32, 0u32, 0u32]);
                    let geometry = e.mem.u32(geometry_at);
                    typed_call!(
                        crate::scene::FAMILY,
                        0x00a5_9c60,
                        crate::scene::walk::update(e, Ptr::new(geometry), data.addr())
                    );
                });
                let slot = u32::from_ne_bytes((*processed - 1).to_ne_bytes());
                e.call(LIGHT_SLOT, &args![light, target, slot]);
                e.call(LIGHT_GRAPH, &args![light]);
                let scene = e.global::<u32>(SCENE_NODE_TABLE);
                e.call(OBJECT_LOCK, &args![scene]);
                let scene = e.global::<u32>(SCENE_NODE_TABLE);
                e.call(SCENE_LIGHT_UPDATE, &args![scene, light]);
                if setting_byte(e, SETTING_ADD_OBJECTS) != 0 {
                    let source = e.mem.u32(source_at);
                    typed_call!(
                        crate::lights::FAMILY,
                        0x00ba_0110,
                        crate::lights::walk::add_object(e, Ptr::new(light), source)
                    );
                }
                e.call(LIGHT_DONE, &args![light]);
                queued = true;
            }
        }
    }
    if !queued {
        let geometry = e.mem.u32(geometry_at);
        set_flag(e, geometry, 1);
    }
}
