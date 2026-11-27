//! Crash and leak fixes (`NV_CRASH_FIXES`, `[nv-rs Fixes] bCrashFixes`): one row of the `fixes`
//! preset ([`crate::fixes`]) made of parts, each a slip of FalloutNV.exe 1.4.0.525 that stops the
//! game or keeps freed memory in use. **Not 1:1 with the exe**; in the `vanilla` preset nothing
//! here runs.
//!
//! Each part is switchable alone by its own variable and its own `[nv-rs Fixes]` key
//! (`crates/host/data/game_ini.tsv`): a part's own switch wins, else it follows the family row,
//! which follows the preset. A part that is off registers nothing, so the translation runs as
//! the original. A part that is on registers its own functions at the exe addresses in
//! [`Part::addresses`] only (each listed in `crates/engine-core/data/overridable.tsv`); the
//! translations themselves are never edited.
//!
//! The parts, by the game's system: [`menus`] (the Pip-Boy inventory, containers, repair and the
//! companion wheel).
//!
//! Written clean-room from behaviour specs.

use crate::fixes::{Choice, Fix};
use engine::prelude::*;

pub mod menus;

/// One part of the family row: a fix of one slip of the game.
pub struct Part {
    /// Its name in the start-up line.
    pub name: &'static str,
    /// The environment variable that turns only this part on (`1`) or off (`0`), whatever the
    /// family row and the preset say; its `[nv-rs Fixes]` key in the table has the same switch.
    pub env: &'static str,
    /// The exe addresses it registers at.
    pub addresses: &'static [u32],
    /// One line: what a player sees fixed.
    pub summary: &'static str,
    /// Registers it, at [`Part::addresses`] only.
    pub install: fn(&mut Engine),
}

/// The parts, in the order they are installed.
pub static PARTS: &[Part] = &[
    menus::INVENTORY_USE,
    menus::QUANTITY_DROP,
    menus::INVENTORY_EQUIP,
    menus::CONTAINER_HOVER,
    menus::REPAIR_TARGET,
    menus::WHEEL_STIMPAK,
    menus::NAMELESS_ITEM,
];

/// Every part's addresses, once each (a test holds it to [`PARTS`]).
const ADDRESSES: &[u32] = &[
    menus::DO_CLICK,
    menus::DROP_CURRENT_ITEM,
    menus::SELECT_CURRENT_ITEM,
    menus::CONTAINER_DO_IDLE,
    menus::LIST_REMOVE_ALL,
    menus::LIST_DESTRUCTOR,
    menus::DO_USE_STIM,
    menus::DRINK_POTION,
    menus::LIST_FILL,
];

/// The row in [`crate::fixes::FIXES`].
pub const FIX: Fix = Fix {
    name: "crash and leak fixes",
    env: "NV_CRASH_FIXES",
    addresses: ADDRESSES,
    summary: "slips of the original that stop the game or keep freed memory in use are fixed, each switchable alone (the parts' own keys)",
    install: install_every_part,
};

/// Every part: what the row's install does when nothing switches a part alone.
fn install_every_part(e: &mut Engine) {
    for part in PARTS {
        (part.install)(e);
    }
}

/// The parts this run has on ([`Choice::is_part_on`]), installed.
pub fn install(e: &mut Engine, choice: &Choice) {
    for part in PARTS.iter().filter(|p| choice.is_part_on(p)) {
        (part.install)(e);
    }
}

/// The part a variable names.
pub fn part_by_env(env: &str) -> Option<&'static Part> {
    PARTS.iter().find(|p| p.env == env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_addresses_are_the_parts_once_each() {
        let mut parts: Vec<u32> = PARTS.iter().flat_map(|p| p.addresses).copied().collect();
        parts.sort_unstable();
        let before = parts.len();
        parts.dedup();
        assert_eq!(parts.len(), before, "two parts share an address");
        let mut row = ADDRESSES.to_vec();
        row.sort_unstable();
        assert_eq!(row, parts);
    }

    #[test]
    fn every_part_has_its_own_variable_and_a_summary() {
        let mut seen = std::collections::BTreeSet::new();
        for part in PARTS {
            assert!(part.env.starts_with("NV_CRASH_FIX_"), "{}", part.name);
            assert!(seen.insert(part.env), "{}", part.env);
            assert!(!part.summary.is_empty() && !part.addresses.is_empty());
            assert!(crate::fixes::FIXES.iter().all(|f| f.env != part.env));
        }
    }

    /// A part follows the family row, its own variable wins; a part's variable makes the run a
    /// test (vanilla), so it alone is on.
    #[test]
    fn a_part_follows_the_row_and_its_own_switch() {
        let part = &PARTS[0];
        let other = &PARTS[1];
        let fixes = Choice::from_parts(None, |_| None).unwrap();
        assert!(fixes.is_on(&FIX) && fixes.is_part_on(part) && fixes.is_part_on(other));
        let vanilla = Choice::from_parts(None, |n| (n == "NV_PRESET").then(|| "vanilla".into()));
        let vanilla = vanilla.unwrap();
        assert!(!vanilla.is_part_on(part) && !vanilla.is_part_on(other));
        let alone = Choice::from_parts(None, |n| (n == part.env).then(|| "1".into())).unwrap();
        assert!(!alone.is_on(&FIX));
        assert!(alone.is_part_on(part) && !alone.is_part_on(other));
        let without = |n: &str| match n {
            "NV_PRESET" => Some("fixes".to_string()),
            n if n == part.env => Some("0".to_string()),
            _ => None,
        };
        let without = Choice::from_parts(None, without).unwrap();
        assert!(without.is_on(&FIX));
        assert!(!without.is_part_on(part) && without.is_part_on(other));
        assert!(without
            .describe()
            .contains(&format!("without {}", part.name)));
        let bad = Choice::from_parts(None, |n| (n == part.env).then(|| "yes".into()));
        assert!(bad.is_err());
    }
}
