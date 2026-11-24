//! Dead lights in a lighting property's light list (the fixes row "dead lights in a light list",
//! `bDeadLightGuard` in Fallout.ini's `[nv-rs Fixes]`, `NV_DEAD_LIGHT_GUARD`; on in the fixes preset,
//! off in vanilla).
//!
//! **The original's fault.** A `ShadowSceneLight`'s destructor (`00b9f0c0`) takes the light out of
//! the lighting property of each geometry in its own geometry list (`00b9ef30`:
//! `BSShaderLightingProperty::RemoveLight`, `00b717a0`), then releases its `NiLight` and writes 0 to
//! `spLight` (+0xf8). A property can still hold the light after that: `RemoveLight` does nothing
//! while the property is fenced (between `FenceAllLights` and `RemoveFencedLights`), and a geometry
//! whose property word changed, or that left the light's list without `RemoveLight`, is not
//! visited. The property's list then holds a destroyed light, and the next walk that reads through
//! its `spLight` stops the game: `ResortLights` (`00b70390`) asks its luminance (`00b9dbe0` reads
//! `spLight` +0x8c: "read of unmapped memory at 0000008c" in the staged fight with JIP's projectile
//! lights, 2026-11-22), the active-light getters read its app-culled bit, the pass staging
//! (`00b70820`) its colour. JIP LN NVSE meets the same entries: its game patches put a null check
//! into one of these walkers only, `GetNumberofActiveNonShadowLights` (G01, `0xB707E0`).
//!
//! **The row.** An entry whose light is not a live `ShadowSceneLight` (another vtable than
//! `010b7eb8`, or `spLight` 0) is dead; a null entry is left to the walkers, which stop there as the
//! exe's do. [`sweep`] (at the start of `GetRenderPasses`' rebuild check and of `ResortLights`)
//! unlinks each dead entry and frees its node as `RemoveLight` does (the
//! list's `00e7db70`), marks the list changed and clears the passes' key, so the passes are rebuilt
//! without it; the first few times it prints the property, the geometry, the light and the
//! property's fence word (what is needed to find who left it). The getters, the counts and the
//! pass staging skip a light whose `spLight` is 0 ([`skips_null`]). With the row off nothing is
//! unlinked or skipped: the exe's path (the sweep only reads).
//!
//! The sweep runs at every lighting property's `GetRenderPasses`: the typed one of
//! `BSShaderPPLightingProperty` here (`00b67be0`), and the translated `00bb4740` and `00c058f0`
//! through the row's hooks (`engine::dead_lights`).
//!
//! **What it cannot see.** A dead light's block taken again by a new `ShadowSceneLight` before the
//! property's next sweep: the entry then names a live light (its vtable and `spLight` are the new
//! light's), so it is neither taken out nor printed. Nothing reads freed memory then, so nothing
//! stops; at most the object is lit by that new light (with its own place and radius) until the
//! new light's own removal or the property's list is rebuilt. A block taken by any other class is
//! caught by the vtable test.
//!
//! Run-wide: the row is kept by run (the kernel table's address, `KernelRef::id`), the same in every
//! thread's engine context.

use engine_core::{args, Engine};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::Mutex;

/// `ShadowSceneLight`'s vtable and its `spLight` (+0xf8).
pub const SHADOW_SCENE_LIGHT_VTABLE: u32 = 0x010b_7eb8;
const SP_LIGHT: u32 = 0xf8;
/// The marker light `FenceAllLights` puts into a list as its fence (`011fa310`): never dead.
const FENCE_MARKER: u32 = 0x011f_a310;
/// The property's light list (+0x60: head, tail, count), its "passes' key" (`iLastRenderPassState`,
/// +0x38), "list changed" (`bLightListChanged`, +0x74) and fence (+0x78).
const LIGHT_LIST: u32 = 0x60;
const LAST_RENDER_PASS_STATE: u32 = 0x38;
const LIGHT_LIST_CHANGED: u32 = 0x74;
const FENCE: u32 = 0x78;
/// The list's remove of an element (`00e7db70`, `thiscall(list, slot)`: the first node holding the
/// slot's word unlinked and freed, the count lowered), as `RemoveLight` calls it.
const LIST_REMOVE: u32 = 0x00e7_db70;
/// How many dead entries are printed in a run.
const PRINTED: u32 = 8;

/// Whether any run has the row on: the one check while none has.
static ANY: AtomicBool = AtomicBool::new(false);
/// The runs with the row on (the kernel table's address).
static RUNS: Mutex<Vec<usize>> = Mutex::new(Vec::new());
/// Dead entries printed so far.
static SHOWN: AtomicU32 = AtomicU32::new(0);

/// Turns the row on or off for `e`'s run (the host's fixes row; tests).
pub fn set_guard(e: &Engine, on: bool) {
    let id = e.kernel.id();
    let mut runs = RUNS.lock().unwrap_or_else(|p| p.into_inner());
    runs.retain(|r| *r != id);
    if on {
        runs.push(id);
    }
    ANY.store(!runs.is_empty(), Relaxed);
}

/// Whether the row is on for `e`'s run.
pub fn guard_on(e: &Engine) -> bool {
    ANY.load(Relaxed)
        && RUNS
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains(&e.kernel.id())
}

/// Whether a light whose `spLight` reads 0 is skipped by the getters, the counts and the pass
/// staging (the row on). Asked only on that path, so a live light costs no more than its read.
#[cold]
#[inline(never)]
pub fn skips_null(e: &Engine) -> bool {
    guard_on(e)
}

/// Whether a list entry's light is dead: a light (not null: the walkers stop at a null entry, as
/// the exe's do) other than the fence marker, of another vtable than `ShadowSceneLight`'s or with
/// `spLight` 0 (no other class uses that vtable: only the constructor and the destructor write it).
pub fn is_dead(e: &Engine, light: u32) -> bool {
    light != 0
        && light != FENCE_MARKER
        && (e.mem.u32(light) != SHADOW_SCENE_LIGHT_VTABLE
            || e.mem.u32(light.wrapping_add(SP_LIGHT)) == 0)
}

/// With the row on: every dead entry of `property`'s light list unlinked and its node freed (as
/// `RemoveLight` does), the list marked changed and the passes' key cleared; `geometry` (0 when not
/// known) only for the print. Answers whether anything was unlinked. With the row off in every run
/// nothing is read; with it off in this run, or no dead entry, nothing is written.
pub fn sweep(e: &mut Engine, property: u32, geometry: u32) -> bool {
    // No run has the row on: nothing read, what the exe costs.
    if !ANY.load(Relaxed) {
        return false;
    }
    let list = property.wrapping_add(LIGHT_LIST);
    let mut node = e.mem.u32(list);
    let mut any = false;
    while node != 0 {
        if is_dead(e, e.mem.u32(node.wrapping_add(8))) {
            any = true;
            break;
        }
        node = e.mem.u32(node);
    }
    if !any || !guard_on(e) {
        return false;
    }
    let mut dead = Vec::new();
    let mut node = e.mem.u32(list);
    while node != 0 {
        let light = e.mem.u32(node.wrapping_add(8));
        if is_dead(e, light) {
            dead.push(light);
        }
        node = e.mem.u32(node);
    }
    for light in dead {
        if SHOWN.fetch_add(1, Relaxed) < PRINTED {
            let (vtable, sp_light) = (e.mem.u32(light), e.mem.u32(light.wrapping_add(SP_LIGHT)));
            eprintln!(
                "nv-rs: a dead light was still in a lighting property's list, taken out (the fixes row \
                 \"dead lights in a light list\"): property {property:08x}, geometry {geometry:08x}, light \
                 {light:08x} (vtable {vtable:08x}, spLight {sp_light:08x}), the property's fence word {:08x}",
                e.mem.u32(property.wrapping_add(FENCE))
            );
        }
        e.with_stack(4, |e, slot| {
            e.mem.set_u32(slot.addr(), light);
            e.call(LIST_REMOVE, &args![list, slot]);
        });
    }
    e.mem.set_u8(property.wrapping_add(LIGHT_LIST_CHANGED), 1);
    e.mem
        .set_u32(property.wrapping_add(LAST_RENDER_PASS_STATE), 0);
    true
}
