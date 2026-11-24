//! Owned Stewie10.05 SaveManager manual saves and deferred reload input.
use super::Entry;
use crate::extend::Owner;
const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: &[Owner::Stewie],
        step: "stewie-save-manager-manual",
    }
}
pub(in super::super) const SITES: &[Entry] = &[
    row(0x005b_6cb0),
    row(0x005c_54b7),
    row(0x0060_ca6c),
    row(0x0070_f7e2),
    row(0x0072_6f2a),
    row(0x0072_72f2),
    row(0x0075_b24a),
    row(0x0077_9529),
    row(0x0077_d6de),
    row(0x007c_bdaf),
    row(0x007c_c7c2),
    row(0x007c_fd3c),
    row(0x007d_6931),
    row(0x0085_0415),
    row(0x0085_1050),
    row(0x0086_eac2),
    row(0x0094_039b),
    row(0x0094_0c78),
    row(0x0094_2869),
    row(0x0094_7ab8),
];
