//! Tweaks10.05 Menu Search additional-menu hooks, source-owned native sites.
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
    row(0x00727637),
    row(0x0072d3b3),
    row(0x0072d763),
    row(0x0072da1d),
    row(0x0072dacb),
    row(0x0072dc9c),
    row(0x0072dcd3),
    row(0x0072e1bf),
    row(0x0072e208),
    row(0x0075b46f),
    row(0x0075ba66),
    row(0x0075c12b),
    row(0x0075c1d9),
    row(0x0075c3ce),
    row(0x0075c3f5),
    row(0x0075c443),
    row(0x0075c514),
    row(0x0075c584),
    row(0x0075c5a8),
    row(0x0075c9dc),
    row(0x0075ca31),
    row(0x00784db8),
    row(0x007851d3),
    row(0x0079a221),
    row(0x0079af0d),
    row(0x0079b331),
    row(0x0079b6ea),
    row(0x0079bb86),
    row(0x0079c015),
    row(0x0079d0d9),
    row(0x0079d18f),
    row(0x0079d90a),
    row(0x007cba5a),
    row(0x007d1076),
    row(0x007dc504),
    row(0x007e0051),
];
