//! The seams and wraps of JIP LN NVSE's game patches (step 3.9: its changes to the game's own
//! code, `internal/patches_game.h` and the plain patches of `internal/hooks.h` (JIP LN NVSE)),
//! one file per pull request group (`measure/jip/PLAN-3.9-groups.md`), so that the groups add rows
//! in files of their own, side by side.
//!
//! Each file lists `SITES` and `WRAPS` rows as [`super::SITES`] and [`super::WRAPS`] do (by
//! address, each with its step), and the build checks them as it checks those: addresses in
//! order, and also that no address is listed twice anywhere (in a file, in two files, or in a file
//! and the main tables). A row here has the one owner `Jip`. A patch at an address that already has
//! a row of the main tables (a seam xNVSE also uses, say) is not a row here: JIP joins that row as
//! a second owner instead (as the result zeroing of step 3.9a does). A row lives in the file of the
//! group that owns the translation file holding the seam; a wrap in its home group's file. A
//! vtable slot JIP writes needs no row (a JIP routine is written into the guest slot).
//!
//! [`super::SiteSlot::new`] and [`super::WrapSlot::new`] find a row in the main tables and in every
//! file here.

use super::Entry;

mod dialogue;
mod ga;
mod gb;
mod gc;
mod gd;
mod ge;
mod gf;
mod gh1;
mod gh2;
mod gl;
mod gp;
mod gs;
mod gw;
mod gx;

/// Every file's seams: step 3.9a's, then the groups'.
pub(super) const SITES: &[&[Entry]] = &[
    dialogue::SITES,
    ga::SITES,
    gb::SITES,
    gc::SITES,
    gd::SITES,
    ge::SITES,
    gf::SITES,
    gh1::SITES,
    gh2::SITES,
    gl::SITES,
    gp::SITES,
    gs::SITES,
    gw::SITES,
    gx::SITES,
];

/// Every file's wraps.
pub(super) const WRAPS: &[&[Entry]] = &[
    dialogue::WRAPS,
    ga::WRAPS,
    gb::WRAPS,
    gc::WRAPS,
    gd::WRAPS,
    ge::WRAPS,
    gf::WRAPS,
    gh1::WRAPS,
    gh2::WRAPS,
    gl::WRAPS,
    gp::WRAPS,
    gs::WRAPS,
    gw::WRAPS,
    gx::WRAPS,
];

/// The owner list of a row here.
const JIP: &[super::Owner] = &[super::Owner::Jip];
