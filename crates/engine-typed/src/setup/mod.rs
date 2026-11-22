//! The set-up family: the frame's drivers before the world is culled and drawn, which run on the main
//! thread every frame and grow with the outdoor scene (the shadow scene node's lights and casters):
//! the world frame's draw sequence `00870bd0` with its set-up calls, the shadow preparation `00871290`
//! with the shadow scene node's shadow-light list it walks, sorts and fills, and the frame's render
//! target `00872f50`.
//!
//! - [`world`], the world frame: `00870bd0`, and the set-up calls it makes first: `00872ad0`,
//!   `00872b00`, `008727d0`.
//! - [`shadows`], the shadow preparation: `00871290` (the depth-pass state, the casters, the walk over
//!   the shadow lights).
//! - [`lists`], the shadow scene node's shadow-light list: its iterator `00b5afc0` / `00b5b010`, a
//!   caster's light `00b5cbd0`, the sort around the fences `00b5cde0`, and the two shadow-map drops
//!   `00b5b060` and `00b5c3c0`.
//! - [`frame`], the target: `00872f50`.
//!
//! Stage 1: the objects stay in guest memory; each function stays registered at its exe address. The
//! translated callers reach them through that address: the frame's draw (`008706b0`) calls the world
//! frame, and the draw sequences (`00870a00`, `008707c0`, `00872940`) the set-up calls, by address
//! (they were plain Rust calls, which would have skipped the typed functions).
//! The exe's one- and two-load getters these call are read in place instead of called.
//!
//! **Floats.** Main thread only ([`Threads::Main`]): the sort's fade share is one float operation
//! stored at once (plain `f32`), the rest compares stored floats with `double`s as the exe does.

use engine_core::ptr::Ptr;
use engine_core::typed::{Family, FamilyId, Threads};
use engine_core::{entry, AbiFn};

pub mod frame;
pub mod lists;
pub mod shadows;
pub mod world;

/// This family's bit (the id the lead assigned to the set-up drivers).
pub const FAMILY: FamilyId = FamilyId::new(15);

/// `NV_TYPED=setup`. Native code, so on by its differential tests and planted faults (the
/// native-code rule of 2026-11-11) unless `NV_TYPED` says otherwise.
pub static SETUP: Family = Family {
    name: "setup",
    id: FAMILY,
    past_gate: true,
    threads: Threads::Main,
    stage2: false,
    funcs,
};

/// The family's functions, by exe address.
#[rustfmt::skip]
fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(0x0087_0bd0, world::world_frame(Ptr, u32, u8)),
        entry!(0x0087_2ad0, world::frame_begin(Ptr)),
        entry!(0x0087_2b00, world::camera_setup(Ptr) -> u32),
        entry!(0x0087_27d0, world::water_stages(Ptr)),
        entry!(0x0087_1290, shadows::prepare(Ptr)),
        entry!(0x0087_2f50, frame::frame_target(Ptr, u32, u32, u8, u8) -> u32),
        entry!(0x00b5_afc0, lists::first_shadow_light(Ptr) -> u32),
        entry!(0x00b5_b010, lists::next_shadow_light(Ptr) -> u32),
        entry!(0x00b5_cbd0, lists::shadow_light_of(Ptr, u32) -> u32),
        entry!(0x00b5_cde0, lists::sort_shadow_lights(Ptr, u32)),
        entry!(0x00b5_b060, lists::drop_lit_shadow_maps(Ptr)),
        entry!(0x00b5_c3c0, lists::drop_shadow_maps(Ptr)),
    ]
}
