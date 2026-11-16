//! Group GS's game patches (step 3.9: script engine): its handlers, wraps and slot routines. Only group
//! GS's PR edits this file (and its own tests file beside it).
//!
//! **O7, `bNoFailedScriptLocks`** (`SetOptionalPatch(7)`, `internal/patches_game.h` (JIP LN NVSE); on
//! by JIP's default): `SafeWrite16(0x5E0FBE, 0x7AEB)`, `SafeWrite16(0x5E1A23, 0x0DEB)`,
//! `SafeWrite16(0x5E1F0B, 0x0DEB)`: short jumps over `MOV ECX,[EBP+8]; CALL 00500940; MOV
//! [EAX+8],0` (the jump at `0x5E0FBE` lands on the `JMP`'s target `0x5E103A`, the others right after
//! the clear), so a script whose line fails keeps its data length and runs again next time,
//! where the exe sets it to 0 and the script never runs again. JIP turns it on once and never off
//! (`!bEnable` answers false); the port's seams test the switch's bit, so a later
//! `SetOptionalPatch(7, 0)` would turn it off: not reachable while that command is not built.
//!
//! **O8, `bDoublePrecision`** (`SetOptionalPatch(8)`, on by JIP's default): `DoOperatorHook` in
//! place of `CExpression::Eval`'s operator switch ([`do_operator`]).
//!
//! **Always-on fixes** (`InitGamePatches`): G03 achievements after the console, G34
//! `PlaySound3D`'s node, G70 `GetLocked` on a terminal, G71 `IsCurrentFurnitureObj`'s null
//! process. **JIP's own** (in whenever JIP answers "installed"): G123, the "get hit" image space
//! answered by JIP's hit effects word ([`get_hit_imod`]). JIP installs it in `InitGamePatches`
//! (`WriteRelJump(0x5D2860, GetHitIMODHook)`), but it is under `INSTALLED`, not the fixes row: it
//! answers as the game does until JIP's own commands turn the hit effects off.
//!
//! **G51** (`InitGamePatches`): `AddItem` gives every added form the container items' health call
//! ([`every_item_health`]). **D07** (`DeferredInit`): one word of "JIP Companions Command & Control.esp"'s
//! main script ([`deferred_init`]). Both are fixes (the row on).
//!
//! **Threads.** Script expressions, script commands and condition functions run on the main thread
//! and on the AI threads (their packages' conditions and scripts); `GetGetHit` on any thread that
//! hits an actor. No handler here keeps state: each reads its words and the game's memory (G123
//! reads the immortal mode's lock-free word, `gp::disable_hit_effects`).

use super::Area;
use engine_core::extend::{Owner, SiteSlot, WrapSlot};
use engine_core::prelude::*;
use std::cmp::Ordering::{Equal, Greater, Less};

/// O7's seams: `ScriptRunner::Run`'s failed line, `EvaluateLine`'s set and condition errors.
pub const FAILED_SCRIPT_LOCKS: [SiteSlot; 3] = [
    SiteSlot::new(Owner::Jip, 0x005e_0fbe),
    SiteSlot::new(Owner::Jip, 0x005e_1a23),
    SiteSlot::new(Owner::Jip, 0x005e_1f0b),
];
/// O8: `CExpression::Eval`'s operator switch.
pub const DO_OPERATOR: SiteSlot = SiteSlot::new(Owner::Jip, 0x0059_3fbc);
/// G70: `GetLockedConditionFunction`'s call of `BGSTerminal::IsUnlocked`.
pub const TERMINAL_UNLOCKED: SiteSlot = SiteSlot::new(Owner::Jip, 0x0059_d0b3);
/// G71: `IsCurrentFurnitureObjConditionFunction`'s furniture read through the process.
pub const FURNITURE_PROCESS: SiteSlot = SiteSlot::new(Owner::Jip, 0x0059_e034);
/// G34: `PlaySound3D`'s object for the sound to follow.
pub const SOUND_NODE: SiteSlot = SiteSlot::new(Owner::Jip, 0x005c_240d);
/// G03: `AddAchievement`'s console-used test.
pub const ACHIEVEMENT: SiteSlot = SiteSlot::new(Owner::Jip, 0x005d_3b69);
/// G51: `AddItem`'s tests before the container items' health call (the jump at `0x5B4DBC`).
pub const EVERY_ITEM_HEALTH: SiteSlot = SiteSlot::new(Owner::Jip, 0x005b_4dbc);
/// G123: `TESImageSpaceModifier::GetGetHit` (no arguments).
pub const GET_HIT_IMOD: WrapSlot = WrapSlot::new(Owner::Jip, 0x005d_2860);

/// The group's rows and installers.
pub const AREA: Area = Area {
    name: "GS",
    fix_sites: &[
        DO_OPERATOR.addr(),
        TERMINAL_UNLOCKED.addr(),
        FURNITURE_PROCESS.addr(),
        SOUND_NODE.addr(),
        ACHIEVEMENT.addr(),
        EVERY_ITEM_HEALTH.addr(),
        FAILED_SCRIPT_LOCKS[0].addr(),
        FAILED_SCRIPT_LOCKS[1].addr(),
        FAILED_SCRIPT_LOCKS[2].addr(),
    ],
    fix_wraps: &[],
    install_fixes,
    jip_sites: &[],
    jip_wraps: &[GET_HIT_IMOD.addr()],
    install_jip,
};

/// `TESObjectREFR::renderState` (+0x64) and its root node (+0x14).
const REFR_RENDER_STATE: u32 = 0x64;
const RENDER_STATE_ROOT: u32 = 0x14;
/// `BGSTerminal`'s flags byte (+0xA5; bit 2 "unlocked", what `005015D0` reads), a reference's
/// extra list (+0x44), `kXData_ExtraTerminalState` (0x50) and its two bytes (`lockedOut` +0xC,
/// `lockLevel` +0xD).
const TERMINAL_FLAGS: u32 = 0xa5;
const REFR_EXTRA_LIST: u32 = 0x44;
const EXTRA_TERMINAL_STATE: u32 = 0x50;
const TERMINAL_STATE_LOCKED_OUT: u32 = 0xc;
const TERMINAL_STATE_LOCK_LEVEL: u32 = 0xd;
/// `BaseExtraList::GetByType` (`00410220`).
const GET_BY_TYPE: u32 = 0x0041_0220;
/// The game's "image space on" byte that `GetGetHit` tests first, and the "get hit" image space
/// modifier's form id (JIP's `g_getHitIMOD = LookupFormByRefID(0x162)`).
const IMAGE_SPACE_ENABLED: u32 = 0x0118_abb1;
const GET_HIT_IMOD_ID: u32 = 0x162;

fn install_fixes(e: &mut Engine) {
    for slot in FAILED_SCRIPT_LOCKS {
        e.on_site(slot, keep_failed_script);
    }
    e.on_site(DO_OPERATOR, do_operator);
    e.on_site(TERMINAL_UNLOCKED, terminal_unlocked);
    e.on_site(FURNITURE_PROCESS, furniture_process);
    e.on_site(SOUND_NODE, sound_node);
    e.on_site(ACHIEVEMENT, earn_achievement);
    e.on_site(EVERY_ITEM_HEALTH, every_item_health);
}

fn install_jip(e: &mut Engine) {
    e.wrap(GET_HIT_IMOD, get_hit_imod);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: SetOptionalPatch case 7
// (the three SafeWrite16 jumps).
/// A failed line's clear of the data length: taken (the jump), so the clear is skipped.
fn keep_failed_script(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

/// `cvttsd2si`: truncated toward zero; NaN and values out of `i32`'s range give `0x80000000`
/// (the invalid exception is masked in the game, so no trap).
fn cvttsd2si(v: f64) -> i32 {
    if v.is_nan() || v >= 2_147_483_648.0 || v <= -2_147_483_649.0 {
        i32::MIN
    } else {
        v as i32
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DoOperatorHook (with its
// OPERATOR_* macros and its two jump tables).
/// O8 (`0x593FBC`; words: the operator's index (`&&` 0, `||` 1, `<=` 2, `<` 3, `>=` 4, `>` 5, `==`
/// 6, `!=` 7, `-` 8, `+` 9, `*` 10, `/` 11, `%` 12), the left operand's type word, its value's low
/// and high words, the right operand's three words, the current operand's address): the result's
/// type byte (current +0) is set to 0 (integer) first and to 1 only for a double result, written
/// with all 8 bytes at +8; an integer result writes only the low word at +8, as JIP's `mov
/// [ebp-8], eax`.
/// - The kinds: bit 0 the left's type byte, bit 1 the right's (`shl al, 1; or al, [ebp-0x28]`).
///   Both integers: integer arithmetic on the low words; otherwise both as doubles (an integer by
///   `cvtsi2sd`, a double's 8 bytes as they are), SSE double arithmetic.
/// - `&&` and `||` (whatever the kinds): a double operand is true when any bit of its 8 bytes is
///   set (so -0.0 and NaN are true), an integer when its low word is not 0; the answer is 0 or 1.
/// - Double comparisons are `comisd`'s flags: unordered (a NaN) makes `<=`, `<` and `==` true and
///   `>=`, `>`, `!=` false. Integer comparisons are signed.
/// - `/` with doubles: when the right operand's HIGH value word is 0 (read from its 8 bytes even
///   for an integer right operand) the left is kept, else `divsd`. `/` with integers: a zero
///   divisor keeps the left.
/// - `%` (both kinds) answers an integer: doubles are truncated (`cvttsd2si`) first; a zero divisor
///   keeps the (truncated) left.
///
/// Where JIP's `idiv` traps (`0x80000000 / -1` and `% -1`; the game would stop) the port wraps
/// (`0x80000000`, 0): a deliberate difference. A type byte other than 0 or 1 (never written by the
/// game) is read by its two bits where JIP's table index would run past its four entries.
fn do_operator(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (index, current) = (a[0], a[7]);
    let kinds = (a[4] as u8) << 1 | a[1] as u8;
    let (left_float, right_float) = (kinds & 1 != 0, kinds & 2 != 0);
    let (left_low, left_high, right_low, right_high) = (a[2], a[3], a[5], a[6]);
    let double = |float: bool, low: u32, high: u32| {
        if float {
            f64::from_bits(u64::from(high) << 32 | u64::from(low))
        } else {
            f64::from(low as i32)
        }
    };
    let int = |e: &mut Engine, v: i32| {
        e.mem.set_u8(current, 0);
        e.mem.set_u32(current + 8, v as u32);
        Some(1)
    };
    let float = |e: &mut Engine, v: f64| {
        e.mem.set_u8(current, 1);
        e.mem.set_f64(current + 8, v);
        Some(1)
    };
    if index <= 1 {
        let l = left_low | if left_float { left_high } else { 0 } != 0;
        let r = right_low | if right_float { right_high } else { 0 } != 0;
        return int(e, i32::from(if index == 0 { l && r } else { l || r }));
    }
    if kinds & 3 == 0 {
        let (l, r) = (left_low as i32, right_low as i32);
        return match index {
            2 => int(e, i32::from(l <= r)),
            3 => int(e, i32::from(l < r)),
            4 => int(e, i32::from(l >= r)),
            5 => int(e, i32::from(l > r)),
            6 => int(e, i32::from(l == r)),
            7 => int(e, i32::from(l != r)),
            8 => int(e, l.wrapping_sub(r)),
            9 => int(e, l.wrapping_add(r)),
            10 => int(e, l.wrapping_mul(r)),
            11 => int(e, if r == 0 { l } else { l.wrapping_div(r) }),
            12 => int(e, if r == 0 { l } else { l.wrapping_rem(r) }),
            _ => None,
        };
    }
    let l = double(left_float, left_low, left_high);
    let r = double(right_float, right_low, right_high);
    // `comisd`'s flags: `None` is unordered (a NaN), which sets ZF, PF and CF.
    let order = l.partial_cmp(&r);
    match index {
        // `setbe` (CF or ZF), `setb` (CF): true also when unordered.
        2 => int(e, i32::from(!matches!(order, Some(Greater)))),
        3 => int(e, i32::from(!matches!(order, Some(Greater | Equal)))),
        4 => int(e, i32::from(matches!(order, Some(Greater | Equal)))),
        5 => int(e, i32::from(matches!(order, Some(Greater)))),
        // `setz` is true when equal or unordered, `setnz` when neither.
        6 => int(e, i32::from(!matches!(order, Some(Less | Greater)))),
        7 => int(e, i32::from(matches!(order, Some(Less | Greater)))),
        8 => float(e, l - r),
        9 => float(e, l + r),
        10 => float(e, l * r),
        11 => float(e, if right_high == 0 { l } else { l / r }),
        12 => {
            let (l, r) = (cvttsd2si(l), cvttsd2si(r));
            int(e, if r == 0 { l } else { l.wrapping_rem(r) })
        }
        _ => None,
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite16(0x5D3B69, 0x9066)).
/// G03 (`0x5D3B69`, no words): the `jnz` over the unlock is a NOP, so the achievement is unlocked
/// even when the console-used byte is set.
fn earn_achievement(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x5C240D, "\x8B\x41\x64\x8B\x40\x14\x66\x90")).
/// G34 (`0x5C240D`: the reference): the object the sound follows is the reference's own root
/// node (`renderState->rootNode`) where the exe asks slot `0x1D0` (for the player, the
/// first-person node in first person). JIP reads it with no test; a reference with no render
/// state (JIP would stop the game) keeps the exe's answer here (`None`).
fn sound_node(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let state = e.mem.u32(a[0].wrapping_add(REFR_RENDER_STATE));
    if state == 0 {
        return None;
    }
    Some(e.mem.u32(state.wrapping_add(RENDER_STATE_ROOT)))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: TerminalGetLockedHook.
/// G70 (`0x59D0B3`: the terminal, the reference), in place of `BGSTerminal::IsUnlocked`: whether
/// the terminal answers unlocked. The terminal's own flag (+0xA5 bit 2) unless the reference has
/// an `ExtraTerminalState`: then unlocked when its `lockedOut` byte is 0x80 (the exe does not look
/// at it) or its lock level is 0xFE, the flag when the level is 0xFF, locked for any other level.
/// No reference or no extra: the flag.
fn terminal_unlocked(e: &mut Engine, a: &[u32]) -> Option<u32> {
    let (terminal, reference) = (a[0], a[1]);
    let flag = e.mem.u8(terminal.wrapping_add(TERMINAL_FLAGS)) & 2 != 0;
    if reference == 0 {
        return Some(u32::from(flag));
    }
    let extra = e
        .call(
            GET_BY_TYPE,
            &args![
                reference.wrapping_add(REFR_EXTRA_LIST),
                EXTRA_TERMINAL_STATE
            ],
        )
        .u32();
    if extra == 0 {
        return Some(u32::from(flag));
    }
    let level = e.mem.u8(extra + TERMINAL_STATE_LOCK_LEVEL);
    let unlocked = if e.mem.u8(extra + TERMINAL_STATE_LOCKED_OUT) == 0x80 || level == 0xfe {
        true
    } else if level == 0xff {
        flag
    } else {
        false
    };
    Some(u32::from(unlocked))
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SAFE_WRITE_BUF(0x59E034, "\x8B\x41\x68\x85\xC0\x74\x11...")).
/// G71 (`0x59E034`: the actor's process, `+0x68`): a null process answers no furniture (0) where
/// the exe calls the process's slot `0x4C8` through null; another process keeps the exe's call
/// (`None`: JIP makes the same call).
fn furniture_process(_: &mut Engine, a: &[u32]) -> Option<u32> {
    (a[0] == 0).then_some(0)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: GetHitIMODHook.
/// G123, `GetGetHit` replaced: 0 while the game's image space byte (`0x118ABB1`) is 0 or JIP's
/// hit effects word (`s_disableHitEffects`: `ToggleHitEffects` bit 1, immortal mode 2 bit 2) is
/// not 0; else the form `0x162` (JIP looks it up once at `DeferredInit`; here at each call, the
/// same form; no cast and no cache at `0x11C9700`, as JIP).
fn get_hit_imod(e: &mut Engine, _: &[u32]) -> Ret {
    let eax = if e.mem.u8(IMAGE_SPACE_ENABLED) == 0 || super::gp::disable_hit_effects(e) != 0 {
        0
    } else {
        crate::cosave::lookup_form(e, GET_HIT_IMOD_ID)
    };
    Ret {
        eax,
        ..Ret::default()
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: InitGamePatches
// (SafeWrite16(0x5B4DBC, 0x19EB)).
/// G51 (`0x5B4DBC`, no words): `AddItem`'s jump over its two tests (a weapon form, a weapon type
/// below 10) to the container items' health call: the call is made for every added form, a
/// leveled list's and a form list's too, and when nothing could be added. With G50 (`00482090`
/// rewritten, GH1) armour and real weapons get the 0.999 health, other items none. Main thread
/// and AI threads (scripts); no state.
fn every_item_health(_: &mut Engine, _: &[u32]) -> Option<u32> {
    Some(1)
}

/// D07: "JIP Companions Command & Control.esp"'s main script (`0xADE` in that file).
const CCC_PLUGIN: &[u8] = b"JIP Companions Command & Control.esp";
const CCC_SCRIPT: u32 = 0xade;
/// `ModInfo::modIndex` (+0x40C), `Script::data` (+0x30), the patched word's place in the data, the
/// word before and after (MSVC's multi-character constants `' 931'` and `' 552'`: the bytes "139 "
/// and "255 ").
const MOD_INDEX: u32 = 0x40c;
const SCRIPT_DATA: u32 = 0x30;
const CCC_WORD_AT: u32 = 0x29c;
const CCC_WORD_BEFORE: u32 = 0x2039_3331;
const CCC_WORD_AFTER: u32 = 0x2035_3532;
/// `TESForm::typeID` (+4) of a `Script` (`kFormType_Script`, 0x11) and `Script::info.dataLength`
/// (+0x20; JIP nvse/GameScript.h, `ScriptInfo` at +0x18, `dataLength` its +8).
const FORM_TYPE: u32 = 4;
const SCRIPT_TYPE: u8 = 0x11;
const SCRIPT_DATA_LENGTH: u32 = 0x20;

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/patches_game.h: DeferredInit (the
// "JIP Companions Command & Control.esp" lines).
/// D07, once at `DeferredInit` (the main thread, before the AI threads run scripts), with the
/// fixes: when that plugin is loaded (`DataHandler::GetModIndex` not 0xFF) and its form `0xADE`
/// holds `' 931'` at its data +0x29C, the word becomes `' 552'`. JIP reads the data with no test;
/// deliberate difference: a form that is not a script, has no data, or has data too short to hold
/// the word (JIP would stop the game or read past the data) is left alone.
pub fn deferred_init(e: &mut Engine) {
    if e.jip_patches() & engine_core::extend::jip_patches::FIXES == 0 {
        return;
    }
    let info = crate::commands::jip_fn_utility::lookup_mod_by_name(e, CCC_PLUGIN);
    if info == 0 {
        return;
    }
    let index = u32::from(e.mem.u8(info + MOD_INDEX));
    let script = crate::cosave::lookup_form(e, index << 24 | CCC_SCRIPT);
    if script == 0 {
        return;
    }
    if e.mem.u8(script + FORM_TYPE) != SCRIPT_TYPE
        || e.mem.u32(script + SCRIPT_DATA_LENGTH) < CCC_WORD_AT + 4
    {
        return;
    }
    let data = e.mem.u32(script + SCRIPT_DATA);
    if data == 0 {
        return;
    }
    if e.mem.u32(data + CCC_WORD_AT) == CCC_WORD_BEFORE {
        e.mem.set_u32(data + CCC_WORD_AT, CCC_WORD_AFTER);
    }
}

#[cfg(test)]
#[path = "gs_tests.rs"]
mod tests;
