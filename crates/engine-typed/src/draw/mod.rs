//! The draw family (family 18, stage 1): the main world draw's drivers, typed as the groundwork for
//! running that draw on its own thread. The world pass (`00873200`) calls the draw's two halves; each
//! goes through the accumulator's slots to the render mode's draw and finish, which draw the batch
//! renderers' pass lists by pass type range and register group, the decal pass and the alpha pass.
//! The secondary scenes (water reflections, menus, the world pass's extra scenes) reach the same
//! functions through their own draw ([`drivers::scene_draw`]), so these are typed once for every caller.
//!
//! - [`drivers`]: the halves, the slots, render mode 0's halves, the pass type range draw, the
//!   register group draw, the alpha sort, the active list clears;
//! - [`alpha`]: the alpha group draw with its distances and clears, a holder's draw, the shadow types'
//!   group-clearing draw;
//! - [`packet`] and [`own`]: the accessors. Everything the draw reads that the rest of the frame
//!   changes goes through [`packet`]; everything it writes that is its own through [`own`], one section
//!   for each of the pipeline probe's conditions that live here (1: the pass lists the next frame's
//!   registration fills again; 2: blend, clip and depth state; 4: the blocks it frees; 7: the one
//!   reference count). A later step can feed the draw from a copied packet and give a drawing thread
//!   its own state by changing those two files; nothing here copies, defers or moves anything yet.
//!
//! The drawing itself (the batch renderer's lists, passes and state setters) is the shader family's,
//! reached by `typed_call!`s that go direct while both families are on.
//!
//! Stage 1: the objects stay in guest memory where the game made them. Each function stays registered
//! at its exe address; a call the exe makes through a slot is direct when the slot holds the typed
//! function ([`crate::slot_target`]), except where the exe jumps with its caller's stack words still in
//! place (the world pass's two halves), which stay calls through the slot.
//!
//! **Floats.** Main thread only by the census, but the family keeps the shader family's exact x87
//! helpers (the alpha distances), so it is [`Threads::Both`].

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod alpha;
pub mod drivers;
pub mod own;
pub mod packet;

/// This family's bit: what each `typed_call!` of it names.
pub const FAMILY: FamilyId = FamilyId::new(18);

/// `NV_TYPED=draw`. Proved by its differential tests and the byte-identical recorded street run (on
/// against `NV_TYPED=draw:off`), so on unless `NV_TYPED` says otherwise.
pub static DRAW: Family = Family {
    name: "draw",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Both,
    stage2: false,
    funcs,
};

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        // The halves, the secondary scenes' draw and the accumulator's slots.
        entry!(0x00b6_ba20, drivers::world_draw_first(u32, Ptr<_>, u32)),
        entry!(0x00b6_b930, drivers::world_draw_second(u32, Ptr<_>, u32)),
        entry!(0x00b6_c0d0, drivers::scene_draw(u32, Ptr<_>) -> u32),
        entry!(0x00b6_6520, drivers::accumulator_draw(Ptr<_>)),
        entry!(0x00b6_6570, drivers::accumulator_finish(Ptr<_>)),
        entry!(0x00d4_7a40, packet::set_camera(Ptr<_>, u32)),
        entry!(0x00aa_0fe0, packet::finish_accumulating(Ptr<_>)),
        entry!(0x00b5_4b10, own::hold_accumulator(u32)),
        // Render mode 0's halves and what they draw with.
        entry!(0x00b6_5ae0, drivers::first_half(Ptr<_>)),
        entry!(0x00b6_5c60, drivers::second_half(Ptr<_>)),
        entry!(0x00b6_3770, drivers::draw_types(Ptr<_>, u32, u32, u32, i32)),
        entry!(0x00b6_39e0, drivers::draw_group(Ptr<_>, u32, u32)),
        entry!(0x00b6_3a50, drivers::sort_alphas(Ptr<_>)),
        entry!(0x00b6_3120, drivers::has_listed_pass(Ptr<_>, u32, u32) -> bool),
        entry!(0x00b6_5580, drivers::clear_active_lists(Ptr<_>)),
        // The alpha pass and the batch renderer's group draws.
        entry!(0x00b6_50c0, alpha::draw_alpha_group(Ptr<_>, u32)),
        entry!(0x00b9_8c80, alpha::alpha_distance(Ptr<_>, u32, u32, u32) -> f64),
        entry!(0x00b6_4f20, alpha::clear_alpha_group(Ptr<_>, u32)),
        entry!(0x00b9_a3c0, alpha::draw_and_clear_type(Ptr<_>, u32, u32, u32, u32) -> bool),
        entry!(0x00b9_a4d0, alpha::draw_holder(u32, u32)),
        entry!(0x00b9_af60, alpha::render_next_alpha(Ptr<_>, u32, u32, u32, f32)),
        // The decal pass and the fog-coloured ambient.
        entry!(0x00b6_4e30, drivers::decal_pass(Ptr<_>)),
        entry!(0x00bb_b840, own::fog_ambient()),
        // The camera handed to the scene renderer, with the camera getters it reads (game-wide).
        entry!(0x004e_9bb0, packet::hand_camera(u32, u32)),
        entry!(0x004e_9c90, packet::pass_camera_data(u32, u32, u32, u32, u32, u32, u32)),
        entry!(0x004a_0d10, packet::camera_frustum(u32) -> u32),
        entry!(0x0045_bbe0, packet::camera_port(u32) -> u32),
        entry!(0x0045_bb80, packet::world_translate(u32) -> u32),
        entry!(0x0045_bba0, packet::rotation_column_0(u32, u32) -> u32),
        entry!(0x004e_9c10, packet::rotation_column_1(u32, u32) -> u32),
        entry!(0x004e_9c50, packet::rotation_column_2(u32, u32) -> u32),
    ]
}
