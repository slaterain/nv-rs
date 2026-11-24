//! Stewie 10.05 Menu Search: Inventory and exact startup/Pip-Boy lifetime seams.
use super::Entry;
use crate::extend::Owner;
const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: &[Owner::Stewie],
        step: "stewie-menu-search",
    }
}
pub(in super::super) const SITES: &[Entry] = &[
    row(0x0070_f6d3),
    row(0x0078_028e),
    row(0x0078_0894),
    row(0x0078_1bbd),
    row(0x0078_2f17),
    row(0x0078_319c),
    row(0x0078_3214),
    row(0x0086_b650),
];
