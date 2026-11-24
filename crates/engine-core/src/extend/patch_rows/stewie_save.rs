//! Stewie 10.05 Save Manager, reconstructed from its owned DLL hook targets.
//! Timer pause and autosave queue seams; original function entry replacement.
use super::Entry;
use crate::extend::Owner;
const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: &[Owner::Stewie],
        step: "stewie-save-manager",
    }
}
pub(in super::super) const SITES: &[Entry] = &[
    row(0x0051_8e41),
    row(0x0071_d665),
    row(0x0071_d720),
    row(0x007c_b65d),
    row(0x007c_b88b),
    row(0x0085_1dae),
    row(0x0085_1dec),
];
pub(in super::super) const WRAPS: &[Entry] = &[row(0x0085_0a40)];
