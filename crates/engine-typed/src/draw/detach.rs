//! The drawing thread's step 1 (`measure/typed-draw/DRAW-THREAD.md`): the world draw's lists detached at the cut.
//!
//! The world pass registers every visible object into its accumulator's batch renderer, then draws (`00b6ba20`,
//! `00b6b930`). With `NV_DRAW_DETACH=1` the per-frame words of those lists (the active pass type list's head,
//! the groups it names, the register group holders' lists and counts, the alpha groups) are taken out of guest
//! memory as the draw starts: their values go into a view and the guest words are zeroed, as a fresh frame's
//! would be. The draw's list walks and clears read and write them through [`word`] and [`set_word`], which
//! answer the view while it is open. As the second half ends the view's words go back to guest memory and the
//! view closes. Serially nothing else runs between, so the frame ends with the same memory as without the
//! switch; a list read that bypasses the view would see a zeroed word instead, and the recorded run would
//! differ. That is what this step proves: the draw's list state is complete in the view, so a drawing thread
//! can take it while main fills the guest lists again.
//!
//! Each list's spare node chain stays in guest memory: the draw's clears hand this frame's nodes to it as
//! today. A drawing thread will need those handed back at its end instead (the design's "deferred return").
//!
//! Off by default; off, [`word`] and [`set_word`] are the guest reads and writes they replace, behind one relaxed
//! load. The view is per run (the kernel table's address keys it), so tests' engines keep their own.

use engine_core::Engine;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};

/// `NV_DRAW_DETACH=1` (or `on`), read once.
pub fn switch_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        matches!(
            std::env::var("NV_DRAW_DETACH").as_deref(),
            Ok("1") | Ok("on")
        )
    })
}

/// Whether any run has a view open: the accessors' one check while none has.
static OPEN: AtomicBool = AtomicBool::new(false);

/// The open views, by run (the kernel table's address).
static VIEWS: Mutex<Vec<(usize, HashMap<u32, u32>)>> = Mutex::new(Vec::new());

fn run(e: &Engine) -> usize {
    e.kernel.id()
}

/// Opens this run's view with the guest words at `addrs` (4-byte aligned): each word's value goes into it and the
/// guest word is zeroed. A word named twice is taken once.
pub fn open(e: &mut Engine, addrs: &[u32]) {
    let mut words = HashMap::with_capacity(addrs.len());
    for &addr in addrs {
        if let std::collections::hash_map::Entry::Vacant(slot) = words.entry(addr) {
            slot.insert(e.global::<u32>(addr));
            e.set_global(addr, 0u32);
        }
    }
    let id = run(e);
    let mut views = VIEWS.lock().unwrap_or_else(|p| p.into_inner());
    views.retain(|(r, _)| *r != id);
    views.push((id, words));
    OPEN.store(true, Relaxed);
}

/// Closes this run's view: every word goes back to guest memory. Nothing when none is open (one relaxed load
/// when no run has one: the switch off).
pub fn close(e: &mut Engine) {
    if !OPEN.load(Relaxed) {
        return;
    }
    let id = run(e);
    let words = {
        let mut views = VIEWS.lock().unwrap_or_else(|p| p.into_inner());
        let Some(at) = views.iter().position(|(r, _)| *r == id) else {
            return;
        };
        let (_, words) = views.swap_remove(at);
        if views.is_empty() {
            OPEN.store(false, Relaxed);
        }
        words
    };
    let mut words: Vec<(u32, u32)> = words.into_iter().collect();
    words.sort_unstable();
    for (addr, value) in words {
        e.set_global(addr, value);
    }
}

/// Whether this run has a view open.
pub fn is_open(e: &Engine) -> bool {
    if !OPEN.load(Relaxed) {
        return false;
    }
    let id = run(e);
    let views = VIEWS.lock().unwrap_or_else(|p| p.into_inner());
    views.iter().any(|(r, _)| *r == id)
}

fn in_view(e: &Engine, addr: u32) -> Option<u32> {
    let id = run(e);
    let views = VIEWS.lock().unwrap_or_else(|p| p.into_inner());
    views
        .iter()
        .find(|(r, _)| *r == id)
        .and_then(|(_, words)| words.get(&addr).copied())
}

fn store_in_view(e: &Engine, addr: u32, value: u32) -> bool {
    let id = run(e);
    let mut views = VIEWS.lock().unwrap_or_else(|p| p.into_inner());
    match views
        .iter_mut()
        .find(|(r, _)| *r == id)
        .and_then(|(_, words)| words.get_mut(&addr))
    {
        Some(word) => {
            *word = value;
            true
        }
        None => false,
    }
}

/// The word at `addr`: the view's while it holds that word, else guest memory's.
#[inline(always)]
pub fn word(e: &Engine, addr: u32) -> u32 {
    if OPEN.load(Relaxed) {
        if let Some(value) = in_view(e, addr) {
            return value;
        }
    }
    e.global::<u32>(addr)
}

/// Writes the word at `addr`: into the view while it holds that word, else into guest memory.
#[inline(always)]
pub fn set_word(e: &mut Engine, addr: u32, value: u32) {
    if OPEN.load(Relaxed) && store_in_view(e, addr, value) {
        return;
    }
    e.set_global(addr, value);
}

/// The 16-bit half at `addr` (2-byte aligned) through [`word`].
#[inline(always)]
pub fn half(e: &Engine, addr: u32) -> u16 {
    let w = word(e, addr & !3);
    let shift = (addr & 2) * 8;
    low16(w >> shift)
}

/// Writes the 16-bit half at `addr` through [`set_word`], keeping the other half.
#[inline(always)]
pub fn set_half(e: &mut Engine, addr: u32, value: u16) {
    let at = addr & !3;
    let shift = (addr & 2) * 8;
    let w = word(e, at);
    let w = (w & !(0xffff << shift)) | (u32::from(value) << shift);
    set_word(e, at, w);
}

/// The byte at `addr` through [`word`].
#[inline(always)]
pub fn byte(e: &Engine, addr: u32) -> u8 {
    let w = word(e, addr & !3);
    low8(w >> ((addr & 3) * 8))
}

#[allow(clippy::cast_possible_truncation)]
fn low16(w: u32) -> u16 {
    w as u16
}

#[allow(clippy::cast_possible_truncation)]
fn low8(w: u32) -> u8 {
    w as u8
}

/// The words the world draw's lists keep in renderer `renderer` and the renderers its register group holders
/// hold, with a value in them (a zero word is zero in the view and in guest memory alike): the active pass type
/// list's head (+0x2c, +0x30); for each pass type the pass map holds (its 37 buckets' chains, read in place) the
/// group's five lists' heads, tails and counts and its count; each holder's (+0x3c..+0x74) flags-and-count word
/// and list head, tail and count, then its own renderer (+8) the same way; the two alpha group lists (+0x78, +0x84:
/// head, tail, count) and the grouped alphas (+0x90, +0x94). Each list's spare chain (+0xc) stays in guest memory.
pub fn list_words(e: &Engine, renderer: u32, out: &mut Vec<u32>) {
    let mut all = Vec::new();
    candidates(e, renderer, &mut all);
    out.extend(all.into_iter().filter(|addr| e.global::<u32>(*addr) != 0));
}

/// Every word [`list_words`] looks at in renderer `renderer` and its holders' renderers, whatever its value. The
/// structure it walks (the group array, the pass map, the holders and their renderers) is never in the view.
pub fn candidates(e: &Engine, renderer: u32, out: &mut Vec<u32>) {
    if renderer == 0 {
        return;
    }
    let mut take = |_: &Engine, addr: u32| out.push(addr);
    take(e, renderer.wrapping_add(0x2c));
    take(e, renderer.wrapping_add(0x30));
    let groups = e.global::<u32>(renderer.wrapping_add(0x08));
    let buckets = e.global::<u32>(renderer.wrapping_add(0x14));
    let hash_size = e.global::<u32>(renderer.wrapping_add(0x10));
    if groups != 0 && buckets != 0 {
        for bucket in 0..hash_size {
            let mut item = e.global::<u32>(buckets.wrapping_add(bucket.wrapping_mul(4)));
            while item != 0 {
                let group =
                    groups.wrapping_add(e.global::<u32>(item.wrapping_add(8)).wrapping_mul(0x68));
                for list in 0..5u32 {
                    let at = group.wrapping_add(list * 0x14);
                    for offset in [4u32, 8, 0x10] {
                        take(e, at.wrapping_add(offset));
                    }
                }
                take(e, group.wrapping_add(0x64));
                item = e.global::<u32>(item);
            }
        }
    }
    for at in [0x78u32, 0x84] {
        for offset in [0u32, 4, 8] {
            take(e, renderer.wrapping_add(at + offset));
        }
    }
    take(e, renderer.wrapping_add(0x90));
    take(e, renderer.wrapping_add(0x94));
    let mut nested = Vec::new();
    for slot in (0x3cu32..=0x74).step_by(4) {
        let holder = e.global::<u32>(renderer.wrapping_add(slot));
        if holder == 0 {
            continue;
        }
        for offset in [0u32, 0x10, 0x14, 0x1c] {
            take(e, holder.wrapping_add(offset));
        }
        nested.push(e.global::<u32>(holder.wrapping_add(8)));
    }
    for inner in nested {
        if inner != renderer {
            candidates(e, inner, out);
        }
    }
}

/// Runs `f` with the view's words back in guest memory, then takes the same words again (their values then,
/// whatever `f` left), for a callee that reads or writes the lists in place: a translation called by address,
/// which the view cannot reach. Each such call is a list read the drawing thread cannot make yet (step 1's
/// list of what is still live). With no view open, `f` alone.
pub fn suspended<R>(e: &mut Engine, f: impl FnOnce(&mut Engine) -> R) -> R {
    if !OPEN.load(Relaxed) {
        return f(e);
    }
    let id = run(e);
    let words = {
        let mut views = VIEWS.lock().unwrap_or_else(|p| p.into_inner());
        let Some(at) = views.iter().position(|(r, _)| *r == id) else {
            drop(views);
            return f(e);
        };
        let (_, words) = views.swap_remove(at);
        if views.is_empty() {
            OPEN.store(false, Relaxed);
        }
        words
    };
    let mut addrs: Vec<u32> = words.keys().copied().collect();
    addrs.sort_unstable();
    for addr in &addrs {
        e.set_global(*addr, words[addr]);
    }
    let answer = f(e);
    open(e, &addrs);
    answer
}

/// [`suspended`] for the words `words` names only (those the view holds): they go back to guest memory for `f`,
/// and are taken again after it. `words` makes the list only when a view is open, so with the switch off this is
/// `f` behind one relaxed load.
pub fn suspended_for<R>(
    e: &mut Engine,
    words: impl FnOnce(&Engine) -> Vec<u32>,
    f: impl FnOnce(&mut Engine) -> R,
) -> R {
    if !OPEN.load(Relaxed) {
        return f(e);
    }
    let addrs = words(e);
    let id = run(e);
    let mut moved = Vec::new();
    {
        let mut views = VIEWS.lock().unwrap_or_else(|p| p.into_inner());
        let Some((_, words)) = views.iter_mut().find(|(r, _)| *r == id) else {
            drop(views);
            return f(e);
        };
        for addr in &addrs {
            if let Some(value) = words.remove(addr) {
                moved.push((*addr, value));
            }
        }
    }
    for (addr, value) in &moved {
        e.set_global(*addr, *value);
    }
    let answer = f(e);
    let mut views = VIEWS.lock().unwrap_or_else(|p| p.into_inner());
    if let Some((_, words)) = views.iter_mut().find(|(r, _)| *r == id) {
        for (addr, _) in moved {
            words.insert(addr, e.global::<u32>(addr));
            e.set_global(addr, 0u32);
        }
    }
    answer
}

/// The words of a register group holder (its flags and count, its list's head, tail and count) and of its
/// renderer, for [`suspended_for`].
pub fn holder_words(e: &Engine, holder: u32) -> Vec<u32> {
    let mut out = vec![
        holder,
        holder.wrapping_add(0x10),
        holder.wrapping_add(0x14),
        holder.wrapping_add(0x1c),
    ];
    candidates(e, e.global::<u32>(holder.wrapping_add(8)), &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A block of `words` guest words holding 1, 2, 3, ..
    fn block(e: &mut Engine, words: u32) -> u32 {
        let at = e.mem.alloc(4 * words);
        for i in 0..words {
            e.mem.set_u32(at + 4 * i, i + 1);
        }
        at
    }

    #[test]
    fn open_takes_the_words_and_close_puts_them_back() {
        let mut e = Engine::new();
        let at = block(&mut e, 4);
        open(&mut e, &[at, at + 8, at + 8]);
        assert!(is_open(&e));
        assert_eq!(
            [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
            [0, 2, 0]
        );
        assert_eq!(
            [word(&e, at), word(&e, at + 4), word(&e, at + 8)],
            [1, 2, 3]
        );
        set_word(&mut e, at, 0x11);
        set_word(&mut e, at + 4, 0x22);
        assert_eq!([e.mem.u32(at), e.mem.u32(at + 4)], [0, 0x22]);
        set_half(&mut e, at + 10, 0xbeef);
        assert_eq!(half(&e, at + 10), 0xbeef);
        assert_eq!(half(&e, at + 8), 3);
        assert_eq!(byte(&e, at + 11), 0xbe);
        close(&mut e);
        assert!(!is_open(&e));
        assert_eq!(
            [
                e.mem.u32(at),
                e.mem.u32(at + 4),
                e.mem.u32(at + 8),
                e.mem.u32(at + 12)
            ],
            [0x11, 0x22, 0xbeef_0003, 4]
        );
        close(&mut e);
        assert_eq!(word(&e, at), 0x11);
    }

    #[test]
    fn suspended_for_hands_its_words_over_and_takes_them_again() {
        let mut e = Engine::new();
        let at = block(&mut e, 3);
        open(&mut e, &[at, at + 4]);
        let seen = suspended_for(
            &mut e,
            |_| vec![at, at + 8],
            |e| {
                let seen = (e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8));
                e.mem.set_u32(at, 7);
                e.mem.set_u32(at + 8, 9);
                seen
            },
        );
        assert_eq!(seen, (1, 0, 3));
        assert_eq!([e.mem.u32(at), e.mem.u32(at + 8)], [0, 9]);
        assert_eq!([word(&e, at), word(&e, at + 4)], [7, 2]);
        let all = suspended(&mut e, |e| (e.mem.u32(at), e.mem.u32(at + 4)));
        assert_eq!(all, (7, 2));
        assert_eq!([e.mem.u32(at), e.mem.u32(at + 4)], [0, 0]);
        close(&mut e);
        assert_eq!(
            [e.mem.u32(at), e.mem.u32(at + 4), e.mem.u32(at + 8)],
            [7, 2, 9]
        );
    }

    #[test]
    fn with_no_view_the_accessors_are_guest_memory() {
        let mut e = Engine::new();
        let at = block(&mut e, 2);
        assert!(!is_open(&e));
        set_word(&mut e, at, 5);
        assert_eq!((word(&e, at), e.mem.u32(at)), (5, 5));
        assert_eq!(suspended_for(&mut e, |_| vec![at], |e| e.mem.u32(at)), 5);
    }
}
