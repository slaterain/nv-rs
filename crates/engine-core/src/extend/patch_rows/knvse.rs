//! Exact kNVSE24 4640f0f hooks.cpp call replacements. Override lookup is
//! before vanilla lookup in this source; it is not the different40 policy.
use super::super::{Entry, Owner};
const fn row(addr: u32) -> Entry {
    Entry {
        addr,
        owners: &[Owner::Knvse],
        step: "knvse24-animation",
    }
}
pub(in super::super) const SITES: &[Entry] = &[
    row(0x0048fb82),
    row(0x00493115),
    row(0x00493dc0),
    row(0x0049431b),
    row(0x0049472b),
    row(0x004948e6),
    row(0x004949d0),
    row(0x00495630),
    row(0x004956ce),
    row(0x0049575b),
    row(0x00495965),
    row(0x004959b9),
    row(0x00495a03),
    row(0x00495dc6),
    row(0x0049651b),
    row(0x004eeb4b),
    row(0x005f444f),
    row(0x008b7985),
    row(0x0097f36d),
    row(0x0097fe09),
    row(0x0097ff06),
    row(0x009d0e80),
];
