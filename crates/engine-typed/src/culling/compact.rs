//! The compact culling copy (`NV_CULL_COMPACT`, OFF by default): the culling walk over a small array
//! of what it reads from each object, in the order it visits them, instead of over the game's objects
//! scattered through the heap.
//!
//! **What is copied** for each object a scene's walk can reach (an [`Item`]): its flags word, its
//! world bound's centre and radius (and the bound's address), which `OnVisible` its class has (the
//! vtable's slot 0xd4: a node, a geometry, a fade node, anything else), and for a node the range of
//! its children's items. Built from the game's bytes, depth first, so a node's subtree is one run of
//! items.
//!
//! **How it stays right** (`engine_core::mem::track`): the pages holding each copied object, its
//! bound and its children array are tracked; every write through the memory marks its page dirty
//! (after the bytes are written). Before each walk the dirty pages are taken and every item on them
//! is read again from the game's bytes; an item whose class, bound address or children changed makes
//! the whole copy be built again. The game's bytes stay the only truth.
//!
//! **The walk** makes the typed walk's decisions in the same order, with the same floats (the plane
//! test's dot product through the exact helpers): the process's modes, the fade culler's verdict,
//! the active planes and their mask kept around each object, the application-culled flag. A node's
//! children walk over items; a geometry is handed to the process (`AppendVirtual`) as the typed walk
//! hands it; a fade node makes its fade decision on the real node ([`super::fade::decide`]), walks
//! its children over items and finishes; anything else (a multi-bound node, a tree, a particle
//! system, a class with its own `OnVisible`) is walked by the real code from that object. A walk with
//! the compound frustum in use, or in a mode the copy does not cover, is the real walk.
//!
//! **Check mode** (`NV_CULL_COMPACT=check`): every walk is the real walk; before it, every item is
//! compared with the game's bytes, and the copy's walk is run beside the real one with every outgoing
//! effect logged instead of made, and the two logs compared. Any difference is printed with the
//! frame. For tests and the lead's batched check, never for play.

use super::fade;
use super::layout::{
    BSCompoundFrustum, BSCullingProcess, NiAVObject, NiCullingProcess, NiFrustumPlanes,
};
use engine_core::mem::track::Track;
use engine_core::ptr::Ptr;
use engine_core::x87::{x87_add, x87_load32, x87_m32, x87_mul, x87_store32};
use engine_core::{typed_call, Engine};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

/// The typed `OnVisible`s the copy walks itself.
const NODE_ON_VISIBLE: u32 = 0x00a5_dbe0;
const GEOMETRY_ON_VISIBLE: u32 = 0x00a7_fd90;
const FADE_ON_VISIBLE: u32 = 0x00b4_e490;
/// `m_uFlags` bits: application culled, fade verdict in and culled, always visible.
const FLAG_APP_CULLED: u32 = 0x1;
const FLAG_FADE_TESTED: u32 = 0x1000;
const FLAG_FADE_CULLED: u32 = 0x0010_0000;
const FLAG_ALWAYS_VISIBLE: u32 = 0x800;
/// The object fields copied: the vtable (+0), the world bound pointer (+0x20), the flags (+0x30);
/// a node's children array (+0xa0) and its size (+0xa6). An object's tracked span (the node's fields
/// up to its children's size; a geometry's the same prefix).
const VTABLE: u32 = 0;
const WORLD_BOUND: u32 = 0x20;
const FLAGS: u32 = 0x30;
const CHILDREN: u32 = 0xa0;
const CHILDREN_SIZE: u32 = 0xa6;
const OBJECT_SPAN: u32 = 0xa8;
/// `kCullMode` values: every object visible, none.
const MODE_ALL_VISIBLE: u32 = 1;
const MODE_NONE_VISIBLE: u32 = 2;

/// What the switch says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    On,
    Check,
}

/// The switch as read (0 not yet read, 1 Off, 2 On, 3 Check); a test sets it directly.
static MODE: AtomicU8 = AtomicU8::new(0);

/// `NV_CULL_COMPACT`: unset or `off` Off; `on`; `check`.
pub fn mode() -> Mode {
    let mut m = MODE.load(Relaxed);
    if m == 0 {
        m = match std::env::var("NV_CULL_COMPACT").as_deref() {
            Ok("on") => 2,
            Ok("check") => 3,
            _ => 1,
        };
        MODE.store(m, Relaxed);
    }
    match m {
        2 => Mode::On,
        3 => Mode::Check,
        _ => Mode::Off,
    }
}

/// For the tests: the switch as if `NV_CULL_COMPACT` said `mode`, and the copies forgotten.
#[doc(hidden)]
pub fn set_mode_for_tests(mode: Mode) {
    MODE.store(
        match mode {
            Mode::Off => 1,
            Mode::On => 2,
            Mode::Check => 3,
        },
        Relaxed,
    );
    COPIES.with(|copies| copies.borrow_mut().clear());
}

/// Which `OnVisible` an object's class has.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Node,
    Geometry,
    Fade,
    /// Any other: walked by the real code from here.
    Other(u32),
}

impl Kind {
    fn of(target: u32) -> Kind {
        match target {
            NODE_ON_VISIBLE => Kind::Node,
            GEOMETRY_ON_VISIBLE => Kind::Geometry,
            FADE_ON_VISIBLE => Kind::Fade,
            other => Kind::Other(other),
        }
    }
}

/// One object of the copy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    pub object: u32,
    pub kind: Kind,
    pub flags: u32,
    /// The world bound's address (0 for none) and, with one, its centre and radius as bits.
    pub bound: u32,
    pub center: [u32; 3],
    pub radius: u32,
    /// For a node or a fade node: the children array, its size, and the items of its children (one
    /// per slot, `u32::MAX` for an empty one) at `children_at..children_at + size`.
    pub children: u32,
    pub size: u16,
    pub children_at: u32,
}

/// The copy of one scene (keyed by its root).
pub struct Copy {
    pub root: u32,
    pub items: Vec<Item>,
    /// Item index of each child slot, laid out per node.
    pub slots: Vec<u32>,
    /// Which items read each tracked page.
    pages: HashMap<u32, Vec<u32>>,
    track: Arc<Track>,
    /// The walk count ([`WALKS`]) at this scene's last walk: a copy not walked for
    /// [`EVICT_AFTER`] walks is dropped (its scene may be gone).
    walked: u64,
}

/// The `N` bytes at `a`, or `None` where they are not mapped: the copy never faults where the real
/// walk would not read (an object freed since the copy was made is read only by a refresh).
fn bytes<const N: usize>(e: &Engine, a: u32) -> Option<[u8; N]> {
    let mut b = [0u8; N];
    e.mem.try_read(a, &mut b).ok().map(|()| b)
}

fn word(e: &Engine, a: u32) -> Option<u32> {
    bytes::<4>(e, a).map(u32::from_le_bytes)
}

/// Reads one object into an item (its children's items are filled by the builder); `None` when a
/// word it reads is not mapped (the copy is then built again, or the walk is the real one).
fn read_item(e: &Engine, object: u32) -> Option<Item> {
    let vtable = word(e, object.wrapping_add(VTABLE))?;
    let kind = Kind::of(word(e, vtable.wrapping_add(0xd4))?);
    let bound = word(e, object.wrapping_add(WORLD_BOUND))?;
    let (center, radius) = if bound != 0 {
        (
            [
                word(e, bound)?,
                word(e, bound.wrapping_add(4))?,
                word(e, bound.wrapping_add(8))?,
            ],
            word(e, bound.wrapping_add(0xc))?,
        )
    } else {
        ([0; 3], 0)
    };
    let (children, size) = match kind {
        Kind::Node | Kind::Fade => (
            word(e, object.wrapping_add(CHILDREN))?,
            bytes::<2>(e, object.wrapping_add(CHILDREN_SIZE)).map(u16::from_le_bytes)?,
        ),
        _ => (0, 0),
    };
    Some(Item {
        object,
        kind,
        flags: word(e, object.wrapping_add(FLAGS))?,
        bound,
        center,
        radius,
        children,
        size,
        children_at: 0,
    })
}

impl Copy {
    /// Builds the copy of the scene under `root` and tracks its pages; `None` when a word it reads
    /// is not mapped (the walk is then the real one).
    pub fn build(e: &Engine, root: u32, track: Arc<Track>) -> Option<Copy> {
        let mut copy = Copy {
            root,
            items: Vec::new(),
            slots: Vec::new(),
            pages: HashMap::new(),
            track,
            walked: 0,
        };
        copy.add(e, root)?;
        Some(copy)
    }

    /// Adds `object` and its subtree; answers its item.
    fn add(&mut self, e: &Engine, object: u32) -> Option<u32> {
        let index = u32::try_from(self.items.len()).unwrap_or(u32::MAX);
        let item = read_item(e, object)?;
        self.items.push(item);
        self.note_pages(index, &item);
        if matches!(item.kind, Kind::Node | Kind::Fade) {
            let at = u32::try_from(self.slots.len()).unwrap_or(u32::MAX);
            self.slots
                .extend(std::iter::repeat(u32::MAX).take(usize::from(item.size)));
            self.items[index as usize].children_at = at;
            for slot in 0..u32::from(item.size) {
                let child = word(e, item.children.wrapping_add(4 * slot))?;
                if child != 0 {
                    let child_item = self.add(e, child)?;
                    self.slots[(at + slot) as usize] = child_item;
                }
            }
        }
        Some(index)
    }

    /// Tracks the pages `item` reads and remembers which item reads each.
    fn note_pages(&mut self, index: u32, item: &Item) {
        let mut span = |addr: u32, len: u32| {
            if addr == 0 || len == 0 {
                return;
            }
            self.track.set_tracked(addr, len, true);
            let first = addr >> 12;
            let last = (u64::from(addr) + u64::from(len) - 1) >> 12;
            for page in first..=u32::try_from(last).unwrap_or(u32::MAX) {
                self.pages.entry(page).or_default().push(index);
            }
        };
        span(item.object, OBJECT_SPAN);
        span(item.bound, 0x10);
        span(item.children, u32::from(item.size) * 4);
    }

    /// The object in child slot `slot` of `item` as the copy has it (0 for an empty slot).
    fn had(&self, item: &Item, slot: u32) -> u32 {
        let at = self.slots[(item.children_at + slot) as usize];
        if at == u32::MAX {
            0
        } else {
            self.items[at as usize].object
        }
    }

    /// `item` read again from the game's bytes when its class, bound address, children array, size
    /// and child slots are still as copied (the walk's shape); `None` when not, or when a word it
    /// reads is no longer mapped.
    fn reread(&self, e: &Engine, old: &Item) -> Option<Item> {
        let new = read_item(e, old.object)?;
        if new.kind != old.kind
            || new.bound != old.bound
            || new.children != old.children
            || new.size != old.size
        {
            return None;
        }
        for slot in 0..u32::from(new.size) {
            if word(e, new.children.wrapping_add(4 * slot))? != self.had(old, slot) {
                return None;
            }
        }
        Some(Item {
            children_at: old.children_at,
            ..new
        })
    }

    /// Brings the copy up to date: the items on every dirty page (page numbers, taken once from the
    /// track for every copy) read again; answers false when an object's class, bound address or
    /// children changed, or a word it reads is no longer mapped (the copy must be built again).
    ///
    /// The items are read again in the copy's order (depth first: a parent before its children),
    /// whatever pages they sit on, so a parent whose child was detached (and maybe freed) since
    /// drops the copy before that child is read.
    pub fn refresh(&mut self, e: &Engine, dirty: &[u32]) -> bool {
        let mut indices: Vec<u32> = dirty
            .iter()
            .filter_map(|page| self.pages.get(page))
            .flatten()
            .copied()
            .collect();
        indices.sort_unstable();
        indices.dedup();
        for index in indices {
            let old = self.items[index as usize];
            match self.reread(e, &old) {
                Some(new) => self.items[index as usize] = new,
                None => return false,
            }
        }
        true
    }

    /// Every item against the game's bytes (the check mode), its children's slots too: answers the
    /// objects that differ or no longer read.
    pub fn stale(&self, e: &Engine) -> Vec<u32> {
        self.items
            .iter()
            .filter(|item| self.reread(e, item) != Some(**item))
            .map(|item| item.object)
            .collect()
    }
}

/// The plane test of `NiBound::WhichSide` on copied bits: 2 behind, 1 in front, 0 across (or a NaN).
fn which_side(e: &Engine, item: &Item, plane: u32) -> u32 {
    let n = |i: u32| e.mem.u32(plane.wrapping_add(4 * i));
    let x = x87_mul(x87_load32(n(0)), x87_m32(item.center[0]));
    let y = x87_mul(x87_load32(n(1)), x87_m32(item.center[1]));
    let sum = x87_add(x, y);
    let z = x87_mul(x87_load32(n(2)), x87_m32(item.center[2]));
    let dot = f32::from_bits(x87_store32(x87_add(sum, z)));
    let distance = dot - f32::from_bits(n(3));
    let radius = f32::from_bits(item.radius);
    if distance <= -radius {
        return 2;
    }
    if radius <= distance {
        1
    } else {
        0
    }
}

/// The walk over the copy from item `index` (what `NiAVObject::Cull` does for its object).
pub fn cull(e: &mut Engine, copy: &Copy, index: u32, process: u32) {
    let item = copy.items[index as usize];
    if item.flags & FLAG_APP_CULLED != 0 {
        return;
    }
    let this = Ptr::<BSCullingProcess>::new(process);
    let mode = e.get(this, BSCullingProcess::kCullMode);
    if mode == MODE_NONE_VISIBLE || mode == MODE_ALL_VISIBLE {
        if mode == MODE_ALL_VISIBLE {
            on_visible(e, copy, index, process);
        }
        return;
    }
    if item.flags & FLAG_FADE_TESTED != 0 {
        if item.flags & FLAG_FADE_CULLED == 0 {
            on_visible(e, copy, index, process);
        }
        return;
    }
    // The walk only starts with no compound frustum in use ([`walk_scene`]), so this is the
    // planes-only path of `BSCullingProcess::Process`.
    let planes = Ptr::<NiFrustumPlanes>::new(
        this.at(BSCullingProcess::base)
            .at(NiCullingProcess::m_kPlanes)
            .addr(),
    );
    let active = e.get(planes, NiFrustumPlanes::m_uiActivePlanes);
    if item.flags & FLAG_ALWAYS_VISIBLE != 0 || active == 0 {
        on_visible(e, copy, index, process);
        return;
    }
    // `DoCulling`.
    let radius = f32::from_bits(item.radius);
    if item.bound == 0 || radius == 0.0 {
        return;
    }
    let mut mask = 1u32;
    let mut index_plane = 0u32;
    while index_plane < 6 {
        if e.get(planes, NiFrustumPlanes::m_uiActivePlanes) & mask != 0 {
            let plane = planes.addr().wrapping_add(0x10 * index_plane);
            let side = which_side(e, &item, plane);
            if side == 2 {
                break;
            }
            if side == 1 {
                let now = e.get(planes, NiFrustumPlanes::m_uiActivePlanes) & !mask;
                e.set(planes, NiFrustumPlanes::m_uiActivePlanes, now);
            }
        }
        index_plane += 1;
        mask = mask.rotate_left(1);
    }
    if index_plane == 6 {
        on_visible(e, copy, index, process);
    }
    e.set(planes, NiFrustumPlanes::m_uiActivePlanes, active);
}

/// What `OnVisible` does for the item's object. In a dry walk (the check mode) nothing is made: a
/// geometry is noted, a fade node takes the decision the real walk recorded for it, anything else is
/// skipped (the real code walked it).
fn on_visible(e: &mut Engine, copy: &Copy, index: u32, process: u32) {
    let item = copy.items[index as usize];
    let dry = DRY.load(Relaxed);
    match item.kind {
        Kind::Node => children(e, copy, &item, process),
        Kind::Geometry if dry => DRY_LIST.with(|list| list.borrow_mut().push(item.object)),
        Kind::Fade if dry => {
            let recorded = RECORD
                .lock()
                .ok()
                .and_then(|r| r.as_ref().map(|r| r.fades.get(&item.object).copied()))
                .flatten();
            match recorded {
                Some(true) => children(e, copy, &item, process),
                Some(false) => {}
                // The real walk never decided this fade node: the copy reached what it did not.
                None => DRY_LIST.with(|list| list.borrow_mut().push(item.object | 1)),
            }
        }
        Kind::Other(_) if dry => {}
        Kind::Fade => {
            let decision = fade::decide(e, item.object, process);
            if decision.visible {
                children(e, copy, &item, process);
            }
            fade::finish(e, decision);
        }
        Kind::Geometry | Kind::Other(_) => {
            super::process::object_on_visible(e, Ptr::<NiAVObject>::new(item.object), process)
        }
    }
}

/// `NiNode::OnVisible` over the copy: only with a bound whose radius is not zero, each child.
fn children(e: &mut Engine, copy: &Copy, item: &Item, process: u32) {
    if item.bound == 0 || f32::from_bits(item.radius) == 0.0 {
        return;
    }
    for slot in 0..u32::from(item.size) {
        let child = copy.slots[(item.children_at + slot) as usize];
        if child != u32::MAX {
            cull(e, copy, child, process);
        }
    }
}

thread_local! {
    /// The copies, by scene root (main thread: the scene walks).
    static COPIES: std::cell::RefCell<HashMap<u32, Copy>> = std::cell::RefCell::new(HashMap::new());
    /// A dry walk's geometries (an object with bit 0 set: a fade node the real walk never decided).
    static DRY_LIST: std::cell::RefCell<Vec<u32>> = const { std::cell::RefCell::new(Vec::new()) };
    /// Scene walks through [`walk_scene`] so far (a copy's age, [`EVICT_AFTER`]).
    static WALKS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// A scene walk of [`walk_scene`] is running.
    static IN_WALK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The check mode's dry walk is running (main thread only; the flag keeps the hot path one load).
static DRY: AtomicBool = AtomicBool::new(false);
/// The check mode records the real walk: on while it runs.
static RECORDING: AtomicBool = AtomicBool::new(false);

/// What the real walk did, for the check mode.
#[derive(Default)]
struct Record {
    appended: Vec<u32>,
    fades: HashMap<u32, bool>,
}
static RECORD: Mutex<Option<Record>> = Mutex::new(None);

/// The real walk handed `geometry` to the accumulator (`AppendVirtual`): noted while the check mode
/// records. One relaxed load otherwise.
#[inline(always)]
pub(crate) fn note_appended(geometry: u32) {
    if RECORDING.load(Relaxed) {
        if let Ok(mut r) = RECORD.lock() {
            if let Some(r) = r.as_mut() {
                r.appended.push(geometry);
            }
        }
    }
}

/// The real walk's fade decision for `node`: noted while the check mode records.
#[inline(always)]
pub(crate) fn note_fade(node: u32, visible: bool) {
    if RECORDING.load(Relaxed) {
        if let Ok(mut r) = RECORD.lock() {
            if let Some(r) = r.as_mut() {
                r.fades.insert(node, visible);
            }
        }
    }
}

/// What the check mode found over the run (for the report and the tests): walks checked, stale items
/// seen, geometries the copy found visible that the real walk did not hand over, geometries of the
/// copy the real walk handed over that the copy did not find, fade nodes it reached that the real
/// walk never decided.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheckCounts {
    pub walks: u64,
    pub stale: u64,
    pub extra: u64,
    pub missing: u64,
    pub undecided: u64,
}
static CHECK: Mutex<CheckCounts> = Mutex::new(CheckCounts {
    walks: 0,
    stale: 0,
    extra: 0,
    missing: 0,
    undecided: 0,
});

/// The check mode's totals so far.
pub fn check_counts() -> CheckCounts {
    CHECK.lock().map(|c| *c).unwrap_or_default()
}

/// The process's `Process` slot (0x44) holds `BSCullingProcess::Process` and no compound frustum
/// holds operations: the walks the copy covers.
fn covered(e: &Engine, process: u32) -> bool {
    if crate::slot_target(e, process, 0x44) != 0x00c4_ee90 {
        return false;
    }
    let frustum = e.get(
        Ptr::<BSCullingProcess>::new(process),
        BSCullingProcess::pCompoundFrustum,
    );
    frustum.is_null() || e.get(frustum, BSCompoundFrustum::iFreeOp) == 0
}

/// A copy not walked for this many scene walks is dropped: its scene may be gone (a menu's, a
/// cell's), and refreshing it would only read what no walk needs.
const EVICT_AFTER: u64 = 2_000;

/// The copy of `scene`, refreshed or built again, taken out of the map while `f` walks it;
/// `None` when it cannot be built (a word it reads is not mapped: the walk is the real one).
fn with_copy<R>(
    e: &mut Engine,
    scene: u32,
    track: Arc<Track>,
    f: impl FnOnce(&mut Engine, &Copy) -> R,
) -> Option<R> {
    let walks = WALKS.with(|w| {
        w.set(w.get() + 1);
        w.get()
    });
    let copy = COPIES.with(|copies| {
        let mut copies = copies.borrow_mut();
        // The dirty pages are taken once (taking clears them) and given to every copy: a scene not
        // walked this time must not miss a write made before its next walk.
        let mut dirty = Vec::new();
        track.take_dirty(|page| dirty.push(page >> 12));
        copies.retain(|_, copy| {
            walks - copy.walked <= EVICT_AFTER && (dirty.is_empty() || copy.refresh(e, &dirty))
        });
        copies.remove(&scene)
    });
    let mut copy = match copy {
        Some(copy) => copy,
        None => Copy::build(e, scene, track)?,
    };
    copy.walked = walks;
    let r = f(e, &copy);
    COPIES.with(|copies| copies.borrow_mut().insert(scene, copy));
    Some(r)
}

/// The scene form's walk (`bs_process_scene`'s `NiAVObject::Cull` of the scene) through the copy
/// when the switch is on and the walk is one the copy covers; answers false when the caller walks
/// the real objects instead. In the check mode the real walk runs here, recorded, and the copy is
/// checked against the game's bytes and against what the real walk did.
pub fn walk_scene(e: &mut Engine, scene: u32, process: u32) -> bool {
    let mode = mode();
    if mode == Mode::Off || !covered(e, process) {
        return false;
    }
    // The track: made at start with real threads (before the memory is shared, `real_threads`), or
    // here on first use with stepped threads; none (shared without one) is the real walk.
    let Some(track) = e.mem.track().cloned().or_else(|| e.mem.enable_track()) else {
        return false;
    };
    // A scene walk inside a scene walk (a class the real code walks culling another scene) is the
    // real walk: the copies and the check mode's record belong to the outer one.
    if IN_WALK.with(|w| w.replace(true)) {
        return false;
    }
    let walked = walk_covered(e, scene, process, mode, track);
    IN_WALK.with(|w| w.set(false));
    walked
}

/// [`walk_scene`] past its tests: the copy's walk (on), or the real walk checked against it.
fn walk_covered(e: &mut Engine, scene: u32, process: u32, mode: Mode, track: Arc<Track>) -> bool {
    if mode == Mode::On {
        return with_copy(e, scene, track, |e, copy| cull(e, copy, 0, process)).is_some();
    }
    // The check mode compares what the typed `AppendVirtual` registered: a process with another
    // one is walked unchecked.
    if crate::slot_target(e, process, 0x4c) != 0x00c4_f1d0 {
        return false;
    }
    with_copy(e, scene, track, |e, copy| {
        let stale = copy.stale(e);
        if let Ok(mut r) = RECORD.lock() {
            *r = Some(Record::default());
        }
        RECORDING.store(true, Relaxed);
        typed_call!(
            super::FAMILY,
            0x00a5_9e00,
            super::visible::cull(
                e,
                Ptr::<NiAVObject>::new(scene),
                Ptr::<NiCullingProcess>::new(process)
            )
        );
        RECORDING.store(false, Relaxed);
        DRY.store(true, Relaxed);
        DRY_LIST.with(|list| list.borrow_mut().clear());
        cull(e, copy, 0, process);
        DRY.store(false, Relaxed);
        let real: std::collections::HashSet<u32> = RECORD
            .lock()
            .ok()
            .and_then(|mut r| r.take())
            .map(|r| r.appended.into_iter().collect())
            .unwrap_or_default();
        let (extra, undecided, found) = DRY_LIST.with(|list| {
            let list = list.borrow();
            let undecided: Vec<u32> = list.iter().filter(|&&o| o & 1 != 0).copied().collect();
            let extra: Vec<u32> = list
                .iter()
                .filter(|&&o| o & 1 == 0 && !real.contains(&o))
                .copied()
                .collect();
            let found: std::collections::HashSet<u32> = list.iter().copied().collect();
            (extra, undecided, found)
        });
        // The other way round: a geometry of the copy that the real walk handed over and the copy's
        // walk did not find (geometries under a class the real code walks are not in the copy).
        let missing: Vec<u32> = copy
            .items
            .iter()
            .filter(|item| {
                item.kind == Kind::Geometry
                    && real.contains(&item.object)
                    && !found.contains(&item.object)
            })
            .map(|item| item.object)
            .collect();
        if !stale.is_empty() || !extra.is_empty() || !missing.is_empty() || !undecided.is_empty() {
            eprintln!(
                "NV_CULL_COMPACT=check: scene {scene:08x}: {} stale item(s) {:08x?}, {} geometry(ies) \
                 the copy found visible and the real walk did not {:08x?}, {} the real walk found \
                 and the copy did not {:08x?}, {} fade node(s) reached that the real walk never \
                 decided",
                stale.len(),
                &stale[..stale.len().min(8)],
                extra.len(),
                &extra[..extra.len().min(8)],
                missing.len(),
                &missing[..missing.len().min(8)],
                undecided.len()
            );
        }
        if let Ok(mut c) = CHECK.lock() {
            c.walks += 1;
            c.stale += stale.len() as u64;
            c.extra += extra.len() as u64;
            c.missing += missing.len() as u64;
            c.undecided += undecided.len() as u64;
        }
    })
    .is_some()
}
