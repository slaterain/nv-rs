//! UI Organizer's patch in `Tile::FinalPostParse` (`00A03923`, made in UIOLoad): a text tile's
//! trait test rewritten as one range, `0xFB7` .. `0xFBE` or `0xFF7`, which takes the zoom
//! (`0xFB8`) in with the game's eight traits, so a text whose zoom changes is
//! rebuilt (`Tile::AddNeedsUpdate(2)`, which UIO's bytes do inline).

use crate::Uio;
use engine_core::prelude::*;

/// The traits of UIO's test: justify (`0xFB7`), zoom, font, wrap width, wrap limit, wrap lines,
/// page, is-HTML (`0xFBE`), and the line gap (`0xFF7`).
pub const FIRST: u32 = 0xfb7;
pub const LAST: u32 = 0xfbe;
pub const LINE_GAP: u32 = 0xff7;

// Translated from UIO 2.30 (426d963), ui_organizer.h 1428: SAFE_WRITE_BUF(0xA03923, ..)
// (`mov eax,[ebp+8]; cmp eax,0xFB7; jb ..; cmp eax,0xFF7; jz ..; cmp eax,0xFBE; ja ..`, then
// `test byte [tile+0x30],2; jnz; or byte [tile+0x30],2; call Tile::AddDirtyTile`, `leave; ret 0xC`
// with the tile in eax).
/// The seam at `00A03923`: `[trait]`. Answers whether the trait is in UIO's range (the
/// translation then queues the rebuild and answers the tile, as the exe's own branch does), or
/// nothing before UIOLoad (its patch is not in yet).
pub fn final_post_parse(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let trait_id = a[0];
    if !e.ext::<Uio>().loaded {
        return None;
    }
    ((FIRST..=LAST).contains(&trait_id) || trait_id == LINE_GAP).then_some(1)
}
