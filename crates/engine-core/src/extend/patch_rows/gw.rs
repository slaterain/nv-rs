//! Group GW's seams and wraps (step 3.9: world, sky, sound, saves, start-up). Only group GW's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GW". A row lives in the file of the group that owns the
//! translation holding the seam (a wrap: its home group's).

use super::super::Entry;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
