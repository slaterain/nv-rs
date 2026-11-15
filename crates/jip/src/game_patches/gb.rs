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
//! The nine left-stick seams are JIP's own (`INSTALLED`: with JIP's default 7849 they answer as
//! the exe); the two right-stick seams change the free camera for every pad player (8689 where the
//! exe uses 7849), so they are a fix (`FIXES`: the game-patch row; off in vanilla). The free
//! camera's four seams sit in group GP's `playercharacter.rs` (GP adds them); the rows and these
//! handlers are GB's.

use super::Area;
use crate::Jip;
use engine_core::extend::{Owner, SiteSlot};
use engine_core::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

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

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GB",
    fix_sites: &RIGHT_ADDRS,
    fix_wraps: &[],
    install_fixes,
    jip_sites: &LEFT_ADDRS,
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
    let (ls, rs) = {
        let input = &e.ext::<Jip>().input;
        (input.dead_zone_ls, input.dead_zone_rs)
    };
    e.mem.set_f64(at, f64::from(ls));
    e.mem.set_f64(at + 8, f64::from(rs));
    let zones = e.ext_free::<DeadZones>();
    zones.left.store(at, Ordering::Relaxed);
    zones.right.store(at + 8, Ordering::Relaxed);
    (at, at + 8)
}

fn install_jip(e: &mut Engine) {
    doubles(e);
    for slot in LEFT_STICK {
        e.on_site(slot, left_stick);
    }
}

fn install_fixes(e: &mut Engine) {
    doubles(e);
    for slot in RIGHT_STICK {
        e.on_site(slot, right_stick);
    }
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
