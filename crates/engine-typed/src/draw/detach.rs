//! The drawing thread's step 1 (`measure/typed-draw/DRAW-THREAD.md`): the world draw's lists detached at the cut.
//!
//! The world pass registers every visible object into its accumulator's batch renderer, then draws (`00b6ba20`,
//! `00b6b930`). With `NV_DRAW_DETACH=1` the per-frame words of those lists (the active pass type list's head,
//! the groups it names, the register group holders' lists and counts, the alpha groups) are taken out of guest
//! memory as the draw starts: their values go into a view and the guest words are zeroed, as a fresh frame's
//! would be. The draw's list walks and clears read and write them through [`word`] and [`set_word`], which
//! answer the view while it is open. As the second half ends the view's words go back to guest memory and the
//! view closes. Serially nothing else runs between, so the frame ends with the same memory as without the
//! switch, and a list read that bypasses the view sees a zeroed word instead. A byte-identical recorded run
//! therefore shows that the list readers the route reaches go through the view. It does not show the view is
//! complete: a word left out of it stays in guest memory and reads correctly. The checks here catch what they
//! can of that: every held word must still read 0 in guest memory when the view closes and after each hand-back
//! ([`suspended`], [`suspended_for`]), or something wrote it in place.
//!
//! Each list's spare node chain stays in guest memory: the draw's clears hand this frame's nodes to it as
//! today. A drawing thread will need those handed back at its end instead (the design's "deferred return").
//!
//! Off by default; off, [`word`] and [`set_word`] are the guest reads and writes they replace, behind one relaxed
//! load. A view belongs to the run (the kernel table's address) and to the engine that opened it: another
//! engine of the same run (a real thread's) reads guest memory. With it on, the first cut and every thousandth
//! print a line (cuts, words taken, hand-backs, words found written in place).

use engine_core::Engine;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
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

/// Whether any engine has a view open: the accessors' one check while none has.
static OPEN: AtomicBool = AtomicBool::new(false);

/// An open view: the run and the engine that opened it, and the words it holds.
struct View {
    run: usize,
    engine: usize,
    words: HashMap<u32, u32>,
}

/// The open views.
static VIEWS: Mutex<Vec<View>> = Mutex::new(Vec::new());

/// What the views have done: cuts, words taken, hand-backs, words found written in place.
static CUTS: AtomicU64 = AtomicU64::new(0);
static WORDS: AtomicU64 = AtomicU64::new(0);
static HAND_BACKS: AtomicU64 = AtomicU64::new(0);
static IN_PLACE: AtomicU64 = AtomicU64::new(0);

/// The view's owner: the run (the kernel table's address, shared by every real thread's engine) and the engine.
fn key(e: &Engine) -> (usize, usize) {
    (e.kernel.id(), e as *const Engine as usize)
}

fn views() -> std::sync::MutexGuard<'static, Vec<View>> {
    VIEWS.lock().unwrap_or_else(|p| p.into_inner())
}

/// Takes this engine's view out of the list, if it has one.
fn take_view(e: &Engine) -> Option<View> {
    let (run, engine) = key(e);
    let mut views = views();
    let at = views
        .iter()
        .position(|v| v.run == run && v.engine == engine)?;
    let view = views.swap_remove(at);
    if views.is_empty() {
        OPEN.store(false, Relaxed);
    }
    Some(view)
}

/// Puts `view` back in the list.
fn put_view(view: View) {
    let mut views = views();
    views.push(view);
    OPEN.store(true, Relaxed);
}

/// The held words that no longer read 0 in guest memory: something wrote them in place. Counted, and a test
/// build stops on one.
fn check_zero(e: &Engine, words: &HashMap<u32, u32>, when: &str) {
    let mut written: Vec<u32> = words
        .keys()
        .copied()
        .filter(|addr| e.global::<u32>(*addr) != 0)
        .collect();
    if written.is_empty() {
        return;
    }
    written.sort_unstable();
    IN_PLACE.fetch_add(written.len() as u64, Relaxed);
    debug_assert!(
        false,
        "NV_DRAW_DETACH: {} held word(s) written in place {when}, the first at {:08x}",
        written.len(),
        written[0]
    );
}

/// Opens this engine's view with the guest words at `addrs` (4-byte aligned): each word's value goes into it and
/// the guest word is zeroed. A word named twice is taken once. A view this engine left open (a second half that
/// never ran) is closed first, its words back in guest memory; a test build stops on one.
pub fn open(e: &mut Engine, addrs: &[u32]) {
    if OPEN.load(Relaxed) {
        if let Some(left) = take_view(e) {
            restore(e, left);
            debug_assert!(false, "NV_DRAW_DETACH: a view was left open");
        }
    }
    let mut words = HashMap::with_capacity(addrs.len());
    for &addr in addrs {
        if let std::collections::hash_map::Entry::Vacant(slot) = words.entry(addr) {
            slot.insert(e.global::<u32>(addr));
            e.set_global(addr, 0u32);
        }
    }
    let cuts = CUTS.fetch_add(1, Relaxed) + 1;
    let taken = WORDS.fetch_add(words.len() as u64, Relaxed) + words.len() as u64;
    if cuts == 1 || cuts % 1000 == 0 {
        eprintln!(
            "NV_DRAW_DETACH: {cuts} cuts, {taken} words taken ({} this cut), {} hand-backs, {} words written in place",
            words.len(),
            HAND_BACKS.load(Relaxed),
            IN_PLACE.load(Relaxed)
        );
    }
    let (run, engine) = key(e);
    put_view(View { run, engine, words });
}

/// The view's words back in guest memory, in address order.
fn restore(e: &mut Engine, view: View) {
    let mut words: Vec<(u32, u32)> = view.words.into_iter().collect();
    words.sort_unstable();
    for (addr, value) in words {
        e.set_global(addr, value);
    }
}

/// Closes this engine's view: every word it holds must still read 0 in guest memory, then goes back there.
/// Nothing when it has none (one relaxed load while no engine has one: the switch off).
pub fn close(e: &mut Engine) {
    if !OPEN.load(Relaxed) {
        return;
    }
    let Some(view) = take_view(e) else {
        return;
    };
    check_zero(e, &view.words, "at the close");
    restore(e, view);
}

/// Whether this engine has a view open.
pub fn is_open(e: &Engine) -> bool {
    if !OPEN.load(Relaxed) {
        return false;
    }
    let (run, engine) = key(e);
    views().iter().any(|v| v.run == run && v.engine == engine)
}

fn in_view(e: &Engine, addr: u32) -> Option<u32> {
    let (run, engine) = key(e);
    views()
        .iter()
        .find(|v| v.run == run && v.engine == engine)
        .and_then(|v| v.words.get(&addr).copied())
}

fn store_in_view(e: &Engine, addr: u32, value: u32) -> bool {
    let (run, engine) = key(e);
    let mut views = views();
    match views
        .iter_mut()
        .find(|v| v.run == run && v.engine == engine)
        .and_then(|v| v.words.get_mut(&addr))
    {
        Some(word) => {
            *word = value;
            true
        }
        None => false,
    }
}

/// The word at `addr`: the view's while this engine's holds that word, else guest memory's.
#[inline(always)]
pub fn word(e: &Engine, addr: u32) -> u32 {
    if OPEN.load(Relaxed) {
        if let Some(value) = in_view(e, addr) {
            return value;
        }
    }
    e.global::<u32>(addr)
}

/// Writes the word at `addr`: into the view while this engine's holds that word, else into guest memory.
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
/// Each renderer is walked once, so holders whose renderers point back at each other end.
pub fn candidates(e: &Engine, renderer: u32, out: &mut Vec<u32>) {
    let mut seen = HashSet::new();
    candidates_of(e, renderer, out, &mut seen);
}

fn candidates_of(e: &Engine, renderer: u32, out: &mut Vec<u32>, seen: &mut HashSet<u32>) {
    if renderer == 0 || !seen.insert(renderer) {
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
        candidates_of(e, inner, out, seen);
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
    let Some(view) = take_view(e) else {
        return f(e);
    };
    HAND_BACKS.fetch_add(1, Relaxed);
    let mut addrs: Vec<u32> = view.words.keys().copied().collect();
    addrs.sort_unstable();
    let View { run, engine, .. } = view;
    restore(
        e,
        View {
            run,
            engine,
            words: view.words,
        },
    );
    let answer = f(e);
    let mut words = HashMap::with_capacity(addrs.len());
    for addr in addrs {
        words.insert(addr, e.global::<u32>(addr));
        e.set_global(addr, 0u32);
    }
    put_view(View { run, engine, words });
    answer
}

/// [`suspended`] for the words `words` names only (those the view holds): they go back to guest memory for `f`,
/// and are taken again after it; every other word the view holds must still read 0 in guest memory then. `words`
/// makes the list only when a view is open, so with the switch off this is `f` behind one relaxed load.
pub fn suspended_for<R>(
    e: &mut Engine,
    words: impl FnOnce(&Engine) -> Vec<u32>,
    f: impl FnOnce(&mut Engine) -> R,
) -> R {
    if !OPEN.load(Relaxed) {
        return f(e);
    }
    let addrs = words(e);
    let Some(mut view) = take_view(e) else {
        return f(e);
    };
    HAND_BACKS.fetch_add(1, Relaxed);
    let mut moved = Vec::new();
    for addr in &addrs {
        if let Some(value) = view.words.remove(addr) {
            moved.push((*addr, value));
        }
    }
    for (addr, value) in &moved {
        e.set_global(*addr, *value);
    }
    let answer = f(e);
    for (addr, _) in moved {
        view.words.insert(addr, e.global::<u32>(addr));
        e.set_global(addr, 0u32);
    }
    check_zero(e, &view.words, "by a call handed only some words");
    put_view(view);
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

    /// A batch renderer as the world accumulator's: groups, a pass map naming two of them, an active list,
    /// the alpha group lists and grouped alphas, and two holders whose renderers point back at each other.
    /// Every word the view should take holds a value. Answers the renderer and its candidate words.
    fn renderer_world(e: &mut Engine) -> (u32, Vec<u32>) {
        let renderer = e.mem.alloc(0x98);
        let groups = e.mem.alloc(4 * 0x68);
        let buckets = e.mem.alloc(4 * 37);
        e.mem.set_u32(renderer + 0x08, groups);
        e.mem.set_u32(renderer + 0x10, 37);
        e.mem.set_u32(renderer + 0x14, buckets);
        // Two map items in one bucket: pass types 0x10 and 0x11 to groups 1 and 3.
        let second = e.mem.alloc(0xc);
        e.mem.set_u32(second + 4, 0x11);
        e.mem.set_u32(second + 8, 3);
        let first = e.mem.alloc(0xc);
        e.mem.set_u32(first, second);
        e.mem.set_u32(first + 4, 0x10);
        e.mem.set_u32(first + 8, 1);
        e.mem.set_u32(buckets + 4 * 5, first);
        let inner = e.mem.alloc(0x98);
        let mut words = Vec::new();
        for r in [renderer, inner] {
            for offset in [
                0x2cu32, 0x30, 0x78, 0x7c, 0x80, 0x84, 0x88, 0x8c, 0x90, 0x94,
            ] {
                words.push(r + offset);
            }
        }
        for g in [1u32, 3] {
            for list in 0..5u32 {
                for offset in [4u32, 8, 0x10] {
                    words.push(groups + g * 0x68 + list * 0x14 + offset);
                }
            }
            words.push(groups + g * 0x68 + 0x64);
        }
        for (slot, owner) in [(0x3cu32, renderer), (0x74, inner)] {
            let holder = e.mem.alloc(0x24);
            e.mem.set_u32(owner + slot, holder);
            e.mem
                .set_u32(holder + 8, if owner == renderer { inner } else { renderer });
            for offset in [0u32, 0x10, 0x14, 0x1c] {
                words.push(holder + offset);
            }
        }
        for (i, w) in (0u32..).zip(words.iter()) {
            e.mem.set_u32(*w, 0x100 + i);
        }
        (renderer, words)
    }

    /// After the cut the guest structure reads as an empty frame: every word the view should hold reads 0 in
    /// guest memory and its own value through the view; the structure itself is untouched.
    #[test]
    fn after_the_cut_the_lists_read_as_an_empty_frame() {
        let mut e = Engine::new();
        let (renderer, expected) = renderer_world(&mut e);
        let mut found = Vec::new();
        candidates(&e, renderer, &mut found);
        let mut sorted_found = found.clone();
        sorted_found.sort_unstable();
        sorted_found.dedup();
        let mut sorted_expected = expected.clone();
        sorted_expected.sort_unstable();
        assert_eq!(sorted_found, sorted_expected);
        let mut taken = Vec::new();
        list_words(&e, renderer, &mut taken);
        open(&mut e, &taken);
        for (i, w) in (0u32..).zip(expected.iter()) {
            assert_eq!(e.mem.u32(*w), 0, "{w:08x} still in guest memory");
            assert_eq!(word(&e, *w), 0x100 + i, "{w:08x} through the view");
        }
        assert_ne!(
            e.mem.u32(renderer + 0x08),
            0,
            "the group array is not per frame"
        );
        close(&mut e);
        for (i, w) in (0u32..).zip(expected.iter()) {
            assert_eq!(e.mem.u32(*w), 0x100 + i);
        }
    }

    /// A view left open (a second half that never ran) is closed by the next open, its words back first.
    #[test]
    #[cfg_attr(debug_assertions, should_panic(expected = "a view was left open"))]
    fn a_view_left_open_is_put_back_by_the_next() {
        let mut e = Engine::new();
        let at = block(&mut e, 2);
        open(&mut e, &[at]);
        set_word(&mut e, at, 9);
        open(&mut e, &[at + 4]);
        assert_eq!(e.mem.u32(at), 9);
        close(&mut e);
        assert!(!is_open(&e));
        assert_eq!([e.mem.u32(at), e.mem.u32(at + 4)], [9, 2]);
    }

    /// A held word written in place shows at the close.
    #[test]
    #[cfg_attr(
        debug_assertions,
        should_panic(expected = "written in place at the close")
    )]
    fn a_word_written_in_place_shows_at_the_close() {
        let mut e = Engine::new();
        let at = block(&mut e, 2);
        open(&mut e, &[at, at + 4]);
        e.mem.set_u32(at + 4, 5);
        close(&mut e);
    }

    /// A word a call handed only some words wrote in place shows when the view takes them again.
    #[test]
    #[cfg_attr(
        debug_assertions,
        should_panic(expected = "by a call handed only some words")
    )]
    fn a_hand_back_that_writes_another_word_shows() {
        let mut e = Engine::new();
        let at = block(&mut e, 2);
        open(&mut e, &[at, at + 4]);
        suspended_for(&mut e, |_| vec![at], |e| e.mem.set_u32(at + 4, 6));
        close(&mut e);
    }

    /// Another engine (another kernel here, as another run) reads guest memory, not this engine's view.
    #[test]
    fn another_engine_does_not_see_the_view() {
        let mut e = Engine::new();
        let other = Engine::new();
        let at = block(&mut e, 1);
        open(&mut e, &[at]);
        assert!(!is_open(&other));
        assert_eq!(word(&e, at), 1);
        close(&mut e);
    }
}
