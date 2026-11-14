//! Group GD's game patches (step 3.9: projectiles, explosions, combat AI, packages): its handlers, wraps and slot routines. Only group
//! GD's PR edits this file (and its own tests file beside it).

use super::Area;

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GD",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: super::nothing,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};
