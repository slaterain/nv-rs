//! Group GL's game patches (step 3.9: lights and magic effects): its handlers, wraps and slot routines. Only group
//! GL's PR edits this file (and its own tests file beside it).

use super::Area;

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GL",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: super::nothing,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};
