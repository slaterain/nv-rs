//! Guard sites (docs/ENGINE_CRATE.md, "Guard sites"; docs/FIXES.md): the one way a translation is
//! edited for a fix. A guard site is one marked line at the exe instruction a community fix patches,
//! `guard_site!(e, <instruction address>, words...)`, which answers whether a fixes-preset row asks
//! the translation to take the guarded path there (a skip, a substituted value, an early end: the
//! site's own code says which, from the exe around it). Every site is a row of
//! `crates/engine-core/data/guard_sites.tsv` (a test in the host holds the two together).
//!
//! In the `vanilla` preset nothing is installed: a site reads the engine's extension table, finds no
//! guard and falls through, so the game's memory, calls and their order are the original's (no call
//! is dispatched: the trace and recording gates see nothing). With a row on, the site calls that
//! row's guard ([`install`]), which looks at the words the site passes (values the translation
//! holds there) and answers.

use crate::Engine;
use std::sync::OnceLock;

/// A row's guard: the site's instruction address and its words; true takes the guarded path.
pub type Guard = fn(&mut Engine, u32, &[u32]) -> bool;

/// How many rows may install guards in one run.
const ROWS: usize = 8;

/// The rows' guards of a run, each with the sites it answers at: an extension slot without a lock,
/// shared with every real thread's engine made from this one afterwards, filled before the game's
/// first frame and only read after.
#[derive(Default)]
pub struct GuardState {
    rows: [OnceLock<(&'static [u32], Guard)>; ROWS],
}

/// Turns a row's guard sites (`sites`) on for this run, answered by `guard`: before the game's
/// first frame. Panics when more than [`ROWS`] rows install.
pub fn install(e: &mut Engine, sites: &'static [u32], guard: Guard) {
    let state = e.ext_free::<GuardState>();
    let mut entry = Some((sites, guard));
    for row in &state.rows {
        if let Some(value) = entry.take() {
            if let Err(value) = row.set(value) {
                entry = Some(value);
            }
        }
    }
    assert!(
        entry.is_none(),
        "more than {ROWS} rows installed guard sites"
    );
}

/// What [`guard_site!`] runs: false when no row answers at `at` (the original's path).
#[inline]
pub fn site(e: &mut Engine, at: u32, words: &[u32]) -> bool {
    let Some(guard) = e.ext_get::<GuardState>().and_then(|state| {
        state
            .rows
            .iter()
            .map_while(OnceLock::get)
            .find(|(sites, _)| sites.contains(&at))
            .map(|&(_, guard)| guard)
    }) else {
        return false;
    };
    guard(e, at, words)
}

/// A guard site at the exe instruction `$at` (a literal, so a search finds every site), with the
/// words the guard looks at: true when a fixes-preset row takes the guarded path.
#[macro_export]
macro_rules! guard_site {
    ($e:expr, $at:literal $(, $word:expr)* $(,)?) => {
        $crate::guard::site($e, $at, &[$($word),*])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_site_falls_through_until_its_row_installs() {
        let mut e = Engine::new();
        assert!(!crate::guard_site!(&mut e, 0x0040_0010, 0u32));
        install(&mut e, &[0x0040_0020], |_, _, _| true);
        assert!(!crate::guard_site!(&mut e, 0x0040_0010, 0u32));
        install(&mut e, &[0x0040_0010], |_, at, w| {
            at == 0x0040_0010 && w[0] == 7
        });
        assert!(crate::guard_site!(&mut e, 0x0040_0010, 7u32));
        assert!(!crate::guard_site!(&mut e, 0x0040_0010, 8u32));
        assert!(crate::guard_site!(&mut e, 0x0040_0020));
    }
}
