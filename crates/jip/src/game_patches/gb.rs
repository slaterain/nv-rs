//! Group GB's game patches (step 3.9: UI: item lists, repair, pad dead zones): its handlers, wraps and slot routines. Only group
//! GB's PR edits this file (and its own tests file beside it).
//!
//! **D04, the pad dead zones** (`DeferredInit`, `internal/patches_game.h` (JIP LN NVSE), only when
//! `HookXInput` succeeded: the port's XInput wrap, in whenever JIP answers "installed",
//! [`crate::input`]). JIP points the operand of eleven `FCOMP double ptr [0x010718F0]` (the exe's
//! dead zone 7849.0, after `FILD axis; FSTP; CALL 00408820` (fabs)) at its own doubles
//! `s_deadZoneLSg` (nine: the caravan, lockpick, map and roulette menus' left stick, the free
//! camera's move axes) and `s_deadZoneRSg` (two: the free camera's look axes), which start at
//! 7849.0 and 8689.0 (`internal/xinput.cpp`; `SetDeadZoneLS/RS`, not built, change them). Each
//! seam answers the guest address of the double to compare with; the doubles live in the
//! built-ins' data, made as JIP installs from [`crate::input::State`]'s dead zones.
//!
//! All eleven are JIP's own (`INSTALLED`, the lead's call): JIP points them whenever its XInput hook
//! is in, whatever the game-patch row says. With JIP's 7849 the left-stick seams answer as the
//! exe; the right-stick seams change the free camera's look for every pad player with JIP
//! installed (8689 where the exe uses 7849). The free camera's four seams sit in group GP's
//! `playercharacter.rs` (GP adds them); the rows and these handlers are GB's.
//!
//! JIP's `SetDeadZoneLS/RS` (not built) write `s_deadZoneLSg`/`s_deadZoneRSg` with the new
//! integer and 19 immediates in the exe each: when they are built they must write the two doubles
//! these seams answer ([`doubles`]: `e.mem.set_f64`) as well as the input state (the dead-zone
//! atomics of [`crate::input::State`], the lock-free slot the gamepad conditions read) and the
//! immediates.
//!
//! **G87, the map menu's skip of its centring** (`InitGamePatches`: `WriteRelJump(0x79DF73,
//! RefreshMapMenuHook)`, JIP's own, `INSTALLED`). In `MapMenu::AddMapIcons` (`0079dbb0`), where
//! the map tile is centred on the player icon (traits `0xFA1`/`0xFA2`), JIP's hook tests its flag
//! `s_mapMenuSkipSetXY`: set, it clears it and jumps past both sets (`0x79E053`); clear, it does
//! the exe's work. Only JIP's item-list refresh (`RefreshItemListBox`, for `RefreshItemsList` and
//! `RemoveMe`'s G60, neither built yet) sets the flag, on the world map tab, just before it calls
//! `AddMapIcons`; [`skip_map_centring_once`] is that set.
//!
//! **G88, JIP's fade types 5 and 6** (`InitGamePatches`: `SafeWrite8(0xA081A8, 6)` and
//! `ProcessGradualSetFloatHook` in the two jump-table words after `Tile::UpdateFadeControls`'
//! table, JIP's own, `INSTALLED`). The exe passes types above 4 over; JIP's arm moves the value
//! from the start to the end over the duration and, at the end, sets the start value again: type
//! 5 starts over (forever), type 6 ends (its control is listed as finished). Only JIP's
//! `SetUIFloatGradual` (built: modes 3 and 4) makes controls of these types.
//!
//! Threads: every handler here runs where its menu code runs, on the main thread (the free
//! camera's dead-zone seams: the main thread's player update). None takes JIP's shared slot: the
//! dead zones and G87's flag are lock-free `ext_free` words made at JIP's install, G88 reads and
//! writes only the fade control and its tile in guest memory.

use super::Area;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// The exe's dead zone double (7849.0), every patched `FCOMP`'s operand.
pub const EXE_DEAD_ZONE: u32 = 0x0107_18f0;

/// The left-stick seams (JIP's `SafeWrite32` addresses, the `FCOMP`s' operands):
/// `CaravanMenu::DoIdle` x and y, `LockPickMenu::RotateLockpick`, `MapMenu::DoGamepad`'s zoom,
/// `RouletteMenu::DoIdle`'s three, the free camera's move x and y (GP's file).
pub const LEFT_STICK: [SiteSlot; 9] = [
    SiteSlot::new(Owner::Jip, 0x0074_16d6),
    SiteSlot::new(Owner::Jip, 0x0074_1705),
    SiteSlot::new(Owner::Jip, 0x0079_0966),
    SiteSlot::new(Owner::Jip, 0x0079_9861),
    SiteSlot::new(Owner::Jip, 0x007b_d47d),
    SiteSlot::new(Owner::Jip, 0x007b_d4a0),
    SiteSlot::new(Owner::Jip, 0x007b_d4dd),
    SiteSlot::new(Owner::Jip, 0x0094_ab59),
    SiteSlot::new(Owner::Jip, 0x0094_ab88),
];
/// The right-stick seams: the free camera's look x and y (GP's file).
pub const RIGHT_STICK: [SiteSlot; 2] = [
    SiteSlot::new(Owner::Jip, 0x0094_a93c),
    SiteSlot::new(Owner::Jip, 0x0094_a95f),
];

const fn addrs<const N: usize>(slots: [SiteSlot; N]) -> [u32; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = slots[i].addr();
        i += 1;
    }
    out
}
const LEFT_ADDRS: [u32; 9] = addrs(LEFT_STICK);
const RIGHT_ADDRS: [u32; 2] = addrs(RIGHT_STICK);
/// All eleven, left then right.
const ALL_ADDRS: [u32; 11] = {
    let mut out = [0; 11];
    let mut i = 0;
    while i < 9 {
        out[i] = LEFT_ADDRS[i];
        i += 1;
    }
    out[9] = RIGHT_ADDRS[0];
    out[10] = RIGHT_ADDRS[1];
    out
};

/// G87's seam: `MapMenu::AddMapIcons` before the map tile is centred (JIP's jump at 0x79DF73).
pub const MAP_SKIP_CENTRING: SiteSlot = SiteSlot::new(Owner::Jip, 0x0079_df73);
/// G88's seams: `Tile::UpdateFadeControls`' jump-table words for fade types 5 and 6.
pub const FADE_TYPES: [SiteSlot; 2] = [
    SiteSlot::new(Owner::Jip, 0x00a0_8718),
    SiteSlot::new(Owner::Jip, 0x00a0_871c),
];

/// The group's JIP-own seams: D04's eleven, then G87's and G88's.
const JIP_SITES: [u32; 14] = {
    let mut out = [0; 14];
    let mut i = 0;
    while i < 11 {
        out[i] = ALL_ADDRS[i];
        i += 1;
    }
    out[11] = MAP_SKIP_CENTRING.addr();
    out[12] = FADE_TYPES[0].addr();
    out[13] = FADE_TYPES[1].addr();
    out
};

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GB",
    fix_sites: &[],
    fix_wraps: &[],
    install_fixes: super::nothing,
    jip_sites: &JIP_SITES,
    jip_wraps: &[],
    install_jip,
};

/// Where JIP's two doubles are (0: not made yet). Lock-free: the seams are asked from any game
/// thread; written once, before the threads start.
#[derive(Default)]
pub struct DeadZones {
    left: AtomicU32,
    right: AtomicU32,
}

/// JIP's `s_deadZoneLSg` and `s_deadZoneRSg`: 8 bytes each in the built-ins' data, made once with
/// the input state's dead zones (7849 and 8689 until `SetDeadZoneLS/RS` are built).
pub fn doubles(e: &mut Engine) -> (u32, u32) {
    let zones = e.ext_free::<DeadZones>();
    let left = zones.left.load(Ordering::Relaxed);
    if left != 0 {
        return (left, zones.right.load(Ordering::Relaxed));
    }
    let at = nvse::table::alloc(e, 16);
    let (ls, rs) = (crate::input::dead_zone_ls(e), crate::input::dead_zone_rs(e));
    e.mem.set_f64(at, f64::from(ls));
    e.mem.set_f64(at + 8, f64::from(rs));
    let zones = e.ext_free::<DeadZones>();
    zones.left.store(at, Ordering::Relaxed);
    zones.right.store(at + 8, Ordering::Relaxed);
    (at, at + 8)
}

fn install_jip(e: &mut Engine) {
    doubles(e);
    e.ext_free::<MapSkipCentring>();
    for slot in LEFT_STICK {
        e.on_site(slot, left_stick);
    }
    for slot in RIGHT_STICK {
        e.on_site(slot, right_stick);
    }
    e.on_site(MAP_SKIP_CENTRING, map_skip_centring);
    for slot in FADE_TYPES {
        e.on_site(slot, gradual_set_float);
    }
}

/// JIP's `s_mapMenuSkipSetXY` (G87): lock-free, made at JIP's install.
#[derive(Default)]
pub struct MapSkipCentring(AtomicBool);

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/jip_core.cpp: RefreshItemListBox (its
// `s_mapMenuSkipSetXY = true` before `AddMapIcons` on the world map tab).
/// The next `MapMenu::AddMapIcons` leaves the map tile where it is (once). For JIP's item-list
/// refresh (`RefreshItemsList`, G60), not built yet.
pub fn skip_map_centring_once(e: &mut Engine) {
    e.ext_free::<MapSkipCentring>()
        .0
        .store(true, Ordering::Relaxed);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: RefreshMapMenuHook.
/// G87: 1 (skip the centring) when JIP's flag was set, which clears it; else 0 (the exe's path).
fn map_skip_centring(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(u32::from(
        e.ext_free::<MapSkipCentring>()
            .0
            .swap(false, Ordering::Relaxed),
    ))
}

/// `GetTickCount` through the exe's import slot (JIP calls the same system function through
/// its own import).
const TICK_COUNT_IMPORT: u32 = 0x00fd_f060;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h:
// ProcessGradualSetFloatHook, nvse/GameTiles.cpp: Tile::SetFloat.
/// G88, a fade control of type 5 or 6 (words: the control, its type). The progress is the ticks
/// since the start (a signed int, `cvtsi2ss`) over the duration, in floats; below 1 (or a NaN)
/// the value is `progress * (end - start) + start`; at 1 or above the start time becomes now
/// and the value the start value, and a type other than 5 answers 1 (listed as finished). The
/// value goes to the tile's trait as JIP's `Tile::SetFloat` does (nothing for a trait the tile
/// does not have, an equal value or a NaN). Answers 0 otherwise.
///
/// nv-rs's own: a control without a tile sets nothing (JIP would read through it).
fn gradual_set_float(e: &mut Engine, words: &[u32]) -> Option<u32> {
    let (&control, &kind) = (words.first()?, words.get(1)?);
    let now = e.call(TICK_COUNT_IMPORT, &[]).u32();
    let elapsed = now.wrapping_sub(e.mem.u32(control + 8)) as i32 as f32;
    let progress = elapsed / e.mem.f32(control + 0xc);
    let start = e.mem.f32(control);
    let (value, finished) = if progress >= 1.0 {
        e.mem.set_u32(control + 8, now);
        (start, kind != 5)
    } else {
        (progress * (e.mem.f32(control + 4) - start) + start, false)
    };
    let (id, tile) = (e.mem.u32(control + 0x10), e.mem.u32(control + 0x14));
    if tile != 0 {
        let at = crate::commands::jip_fn_ui::value_of(e, tile, id);
        if at != 0 {
            crate::commands::jip_fn_ui::value_set_float(e, at, value);
        }
    }
    Some(u32::from(finished))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit
// (SafeWrite32 of &s_deadZoneLSg at the nine left-stick operands).
/// A left-stick `FCOMP`: compares with JIP's `s_deadZoneLSg`.
fn left_stick(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(e.ext_free::<DeadZones>().left.load(Ordering::Relaxed))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit
// (SafeWrite32 of &s_deadZoneRSg at the two right-stick operands).
/// A right-stick `FCOMP`: compares with JIP's `s_deadZoneRSg`.
fn right_stick(e: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(e.ext_free::<DeadZones>().right.load(Ordering::Relaxed))
}

#[cfg(test)]
#[path = "gb_tests.rs"]
mod tests;
