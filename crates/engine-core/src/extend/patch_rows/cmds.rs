//! The seams JIP LN NVSE's commands need (not its game patches): places where a routine of JIP's
//! that the game calls back reads the game's own state in a way the translation cannot give it
//! (a caller's stack frame, say). Rows by address, owner `super::JIP`, step "jip-cmds"; the
//! handlers are the commands' (`jip::commands`), in with JIP's `INSTALLED`.

use super::super::Entry;
use super::JIP;

const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: JIP,
        step: "jip-cmds",
    }
}

/// The commands' seams.
pub(in super::super) const SITES: &[Entry] = &[
    // `ShowQuantityMenu`: `QuantityMenu::DoClick` 007abe40's `CALL dword ptr [0x011DA614]` (the
    // result callback). JIP's `QuantityMenuCallback` reads DoClick's button id from the caller's
    // frame (`[ebp+8]`): the seam hands it over with the callback and the count.
    row(0x007a_becc),
];

/// The commands' wraps.
pub(in super::super) const WRAPS: &[Entry] = &[];
