//! kNVSE 24 (4640f0f), LoadCustomAnimation's temporary private animation map.
//! The plugin exchanges Animation::mapAnimSequenceBase while AddAnimation and
//! InitGroupSpeed run. Native translations pass that selection through this
//! scoped thread context instead: another game thread never observes a private
//! map published in the actor's Animation. All original guest map operations
//! and AddAnimation side effects still run on the selected map.
use crate::Engine;
use std::cell::RefCell;

thread_local! { static MAPS: RefCell<Vec<(u32, u32)>> = const { RefCell::new(Vec::new()) }; }

/// The native call's selected map, or the Animation's original map. An
/// unmodified path creates no heap allocation or shared extension state.
pub fn selected(animation: u32, original: u32) -> u32 {
    MAPS.with(|maps| {
        maps.borrow()
            .iter()
            .rev()
            .find(|s| s.0 == animation)
            .map_or(original, |s| s.1)
    })
}

/// kNVSE24's override-first replacement at one particular GetAt call.
/// Other map calls, including private loader bookkeeping, remain untouched.
pub fn lookup(e: &mut Engine, site: u32, animation: u32, map: u32, group: u16, out: u32) -> bool {
    let map = selected(animation, map);
    if let Some(base) = e.site(site, &[animation, map, u32::from(group)]) {
        e.mem.set_u32(out, base);
        base != 0
    } else {
        e.call(0x0049_c390, &[map, u32::from(group), out]).bool()
    }
}

/// Run AddAnimation against its private map without replacing the guest's
/// pointer. Nested calls and unwinding restore the preceding selection.
pub fn with_private_map<T>(
    e: &mut Engine,
    animation: u32,
    map: u32,
    f: impl FnOnce(&mut Engine) -> T,
) -> T {
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            MAPS.with(|maps| {
                maps.borrow_mut().pop().expect("scoped animation map");
            });
        }
    }
    MAPS.with(|maps| maps.borrow_mut().push((animation, map)));
    let _restore = Restore;
    f(e)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_map_selection_is_nested_scoped_and_thread_local() {
        let mut e = Engine::new();
        assert_eq!(selected(1, 10), 10);
        with_private_map(&mut e, 1, 20, |e| {
            assert_eq!(selected(1, 10), 20);
            with_private_map(e, 1, 40, |_| assert_eq!(selected(1, 10), 40));
            assert_eq!(selected(1, 10), 20);
            with_private_map(e, 2, 30, |_| {
                assert_eq!(selected(1, 10), 20);
                assert_eq!(selected(2, 11), 30);
                assert_eq!(std::thread::spawn(|| selected(1, 10)).join().unwrap(), 10);
            });
            assert_eq!(selected(2, 11), 11);
        });
        assert_eq!(selected(1, 10), 10);
        let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_private_map(&mut e, 1, 20, |_| panic!("test stop"));
        }));
        assert!(stopped.is_err());
        assert_eq!(selected(1, 10), 10);
    }
}
