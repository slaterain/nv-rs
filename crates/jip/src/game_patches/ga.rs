//! Group GA's game patches (step 3.9: UI: HUD, messages, text, start-up INI): its handlers, wraps and slot routines. Only group
//! GA's PR edits this file (and its own tests file beside it).
//!
//! **G10** "Increase corner message max text length to 0x203 characters" (`InitGamePatches`,
//! `internal/patches_game.h` (JIP LN NVSE 57.30), a fix row: in with `NV_JIP_GAME_PATCHES`): 16
//! `SafeWrite8`s, each the second byte of a size or an offset of the HUD corner message record,
//! which grows from 0x310 to 0x410 bytes: the text buffer 0x104 -> 0x204 (the text copy's limit
//! too), the icon path `+0x104` -> `+0x204`, the sound name `+0x208` -> `+0x308`, the display time
//! `+0x30C` -> `+0x40C`. The icon and sound buffers stay 0x104 bytes (their copies' `PUSH 0x104`
//! are not written). Places: `fn_00775380` (the queueing: `0x7753B8` size, `0x7753D2` text limit,
//! `0x7753EA` time, `0x77540C`, `0x775443`, `0x775461`, `0x77547E`, `0x77549C` icon, `0x7754BC`,
//! `0x7754D0` sound, `0x77553C` icon tile, `0x775600` the urgent path's time), `DoIdle`
//! (`0x7714DA` time) and `HideMessage` (`0x7757B1`, `0x7757D3` sound, `0x77582B` icon tile). One
//! seam per function answers the text buffer's size; each function takes all its places from it.
//!
//! One layout for a record's whole life: the fixes go in once, before the first frame
//! ([`super::set_row`] is read at start-up; nothing takes `FIXES` out of the word), so every
//! record is built and read with the same answer.

use super::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;

/// G10's seams (words: the exe's text buffer size 0x104; the answer is the size): `DoIdle`'s
/// time read, the queueing `fn_00775380`, `HideMessage`'s next record.
pub const CORNER_MESSAGE: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Jip, 0x0077_14da),
    SiteSlot::new(Owner::Jip, 0x0077_53b8),
    SiteSlot::new(Owner::Jip, 0x0077_57b1),
];

/// JIP's text buffer: 0x203 letters and the end.
pub const CORNER_MESSAGE_TEXT: u32 = 0x204;

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GA",
    fix_sites: &[
        CORNER_MESSAGE[0].addr(),
        CORNER_MESSAGE[1].addr(),
        CORNER_MESSAGE[2].addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[],
    install_jip: super::nothing,
};

fn install_fixes(e: &mut Engine) {
    for site in CORNER_MESSAGE {
        e.on_site(site, corner_message_text);
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// ("Increase corner message max text length to 0x203 characters", the 16 SafeWrite8s).
/// G10: the corner message's text buffer, 0x204 bytes.
fn corner_message_text(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(CORNER_MESSAGE_TEXT)
}

#[cfg(test)]
#[path = "ga_tests.rs"]
mod tests;
