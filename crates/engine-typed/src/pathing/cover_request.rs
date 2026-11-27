//! The cover pathing request's destructor and setters (Xbox PDB names): the scalar deleting
//! destructor, the two float setters, the five flag setters and the projection-vector copy.
//!
//! The floats are only stored (`fstps`), so they are set plain; everything else is words.

use super::copy_point;
use super::layout::PathingRequestCover;
use crate::culling::layout::NiPoint3;
use engine_core::ptr::Ptr;
use engine_core::{args, Engine};

/// `006dad70`: the destructor body (the `PathingRequest` chain).
const DTOR_BODY: u32 = 0x006d_ad70;
/// `00aa1460(block, size)` (cdecl): frees a block through the memory manager.
const DEALLOCATE: u32 = 0x00aa_1460;

// Typed from 006e41e0
/// `PathingRequestCover` scalar deleting destructor (Xbox PDB): runs the destructor body
/// (`006dad70`) and frees the 0xec bytes only when `flags & 1` is set. Returns `this`.
pub fn scalar_deleting_destructor(e: &mut Engine, this: Ptr, flags: u32) -> Ptr {
    e.call(DTOR_BODY, &args![this]);
    if flags & 1 != 0 {
        e.call(DEALLOCATE, &args![this, 0xecu32]);
    }
    this
}

// Typed from 006e4320
/// Sets `fMaxCoverDistance` (+0xBC).
pub fn set_max_cover_distance(e: &mut Engine, this: Ptr<PathingRequestCover>, value: f32) {
    e.set(this, PathingRequestCover::fMaxCoverDistance, value);
}

// Typed from 006e4340
/// Sets `fMaxHeightChange` (+0xC4).
pub fn set_max_height_change(e: &mut Engine, this: Ptr<PathingRequestCover>, value: f32) {
    e.set(this, PathingRequestCover::fMaxHeightChange, value);
}

// Typed from 006e4360
/// Sets `bFindBothSides` (+0xCD).
pub fn set_find_both_sides(e: &mut Engine, this: Ptr<PathingRequestCover>, value: u8) {
    e.set(this, PathingRequestCover::bFindBothSides, value);
}

// Typed from 006e4380
/// Sets `bFindBehindTarget` (+0xCE).
pub fn set_find_behind_target(e: &mut Engine, this: Ptr<PathingRequestCover>, value: u8) {
    e.set(this, PathingRequestCover::bFindBehindTarget, value);
}

// Typed from 006e43a0
/// Sets `bSortFarthestFirst` (+0xCF).
pub fn set_sort_farthest_first(e: &mut Engine, this: Ptr<PathingRequestCover>, value: u8) {
    e.set(this, PathingRequestCover::bSortFarthestFirst, value);
}

// Typed from 006e43c0
/// Sets `bSortDistanceFromTarget` (+0xD0).
pub fn set_sort_distance_from_target(e: &mut Engine, this: Ptr<PathingRequestCover>, value: u8) {
    e.set(this, PathingRequestCover::bSortDistanceFromTarget, value);
}

// Typed from 006e43e0
/// Sets `bHideCover` (+0xD1).
pub fn set_hide_cover(e: &mut Engine, this: Ptr<PathingRequestCover>, value: u8) {
    e.set(this, PathingRequestCover::bHideCover, value);
}

// Typed from 006e4400
/// Sets `DistanceProjectionVector` (+0xD4) from the three words at `vector`.
pub fn set_projection_vector(e: &mut Engine, this: Ptr<PathingRequestCover>, vector: u32) {
    copy_point(
        e,
        Ptr::<NiPoint3>::new(vector),
        this.at(PathingRequestCover::distance_projection_vector),
    );
}
