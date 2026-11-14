//! Group GC's seams and wraps (step 3.9: hit damage, detection, combat formulas). Only group GC's PR edits this file; rows by
//! address, owner `super::JIP`, step "3.9-GC". A row lives in the file of the group that owns the
//! translation holding the seam (a wrap: its home group's).

use super::super::Entry;

/// The group's seams.
pub(in super::super) const SITES: &[Entry] = &[];

/// The group's wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
