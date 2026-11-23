//! The fixes row "dead lights in a light list" ([`engine_typed::lights::dead`]) at every lighting
//! property's `GetRenderPasses`. The typed one of `BSShaderPPLightingProperty` (`00b67be0`) sweeps
//! the property's list itself; the other two are translations: `00bb4740` (the decal property of
//! vtable `010b9910` and the other classes built on `Lighting30ShaderProperty`) and `00c058f0` (the
//! class of vtable `010bcb60`). With the row each of these two takes the property's dead light
//! entries out first ([`engine_typed::lights::dead::sweep`]: the list marked changed, the passes'
//! key cleared), then runs the translation, which then resorts the lights and rebuilds the passes
//! without them. Without the sweep a property whose state still matched its key kept its passes,
//! and their lights, with the dead one among them, were read at the draw (the light direction set-up
//! `00b7c7b0` reads through its cleared `spLight`). Row off: nothing is registered, the exe's path.

use crate::units::bsshader::run_00bb4520::fn_00bb4740;
use crate::units::unplaced::run_00c048b0::fn_00c058f0;
use engine_core::{entry, Engine, Ptr};
use engine_typed::lights::dead;

/// The row on for `e`'s run: the run-wide flag the typed walkers read, and the sweep at the two
/// translated `GetRenderPasses`.
pub fn install(e: &mut Engine) {
    dead::set_guard(e, true);
    let (at, f) = entry!(0x00bb_4740, passes_00bb4740(Ptr, u32, u32, u32, u32, u32, u32) -> u32);
    e.register(at, f);
    let (at, f) = entry!(0x00c0_58f0, passes_00c058f0(Ptr, u32, u32, u32, u32, u32, u32) -> u32);
    e.register(at, f);
}

/// `00bb4740` with the sweep first (its first word is the geometry).
#[allow(clippy::too_many_arguments)]
fn passes_00bb4740(
    e: &mut Engine,
    this: Ptr,
    geometry: u32,
    render_mode: u32,
    unused_3: u32,
    kind: u32,
    accumulator: u32,
    unused_6: u32,
) -> u32 {
    dead::sweep(e, this.addr(), geometry);
    fn_00bb4740(
        e,
        this,
        geometry,
        render_mode,
        unused_3,
        kind,
        accumulator,
        unused_6,
    )
}

/// `00c058f0` with the sweep first (its first word is the geometry).
#[allow(clippy::too_many_arguments)]
fn passes_00c058f0(
    e: &mut Engine,
    this: Ptr,
    geometry: u32,
    render_mode: u32,
    unused_3: u32,
    kind: u32,
    accumulator: u32,
    unused_6: u32,
) -> u32 {
    dead::sweep(e, this.addr(), geometry);
    fn_00c058f0(
        e,
        this,
        geometry,
        render_mode,
        unused_3,
        kind,
        accumulator,
        unused_6,
    )
}
