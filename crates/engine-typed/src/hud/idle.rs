//! `HUDMainMenu::DoIdle` (`00770430`), the HUD's update on every frame of play: the compass, the
//! weapon and its condition, the bars, then (at most every 200 ms) the damage-threshold flashes, the
//! layout of the two lower bars, the fades of the subtitle and the region text, the sneak meter, the
//! enemy's health bar and its target, the quest reminders and the corner messages, and last the HUD's
//! alpha for the next frame.
//!
//! What stays by address: the sub-updates the exe calls (the weapon, the bars, the sneak meter, the
//! radiation meter, the status mode, the enemy's health, the info for the crosshair, the quest
//! reminder and objectives, the corner message's show and hide), the tiles' fades and their reaction
//! lists, the clock, the screen size settings, the VATS target list, `_ftol2`.

use super::compass::{abs, get_float, global_f64, narrow, set_float, set_int, setting};
use crate::dispatch::tile;
use crate::{crt, dispatch};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::{fistp_chop64, x87_quiet};
use engine_core::{args, typed_call, Engine};

const PLAYER: u32 = 0x011d_ea3c;
const HUD_MENU: u32 = 0x011d_96c0;
/// The HUD's alpha (`011d979c`), set at the end of each update from the HUD opacity setting.
const HUD_ALPHA: u32 = 0x011d_979c;
const HUD_OPACITY: u32 = 0x011d_9774;
/// The game's timer (`011f6394`) and its tick count (`00825c00`, `thiscall` on the timer or on the
/// HUD's own clock at +0x224).
const TIMER: u32 = 0x011f_6394;
const TIMER_TICKS: u32 = 0x0082_5c00;
const FLOAT_SETTING: u32 = 0x0040_3e20;
/// The 200 ms gate (`011d97c4`, made once behind bit 1 of `011d97c8`).
const LAST_TICK: u32 = 0x011d_97c4;
const LAST_TICK_GUARD: u32 = 0x011d_97c8;
/// The enemy-target byte the update sets (`011d96c5`) and the previous answer of `00579690` for the
/// last crosshair object (`011d97c0`).
const ENEMY_IN_VATS: u32 = 0x011d_96c5;
const LAST_OBJECT_STATE: u32 = 0x011d_97c0;
/// The interface's mode word (+8 of `011f2250`) and the actor it names in mode 4 (`011f21cc`).
const INTERFACE_MODE: u32 = 0x011f_2250;
const MODE_ACTOR: u32 = 0x011f_21cc;

/// The tile traits: `x` (0xFA1), `width` hint (0xFA2), `visible` (0xFA3), `alpha` (0xFA9), `width`
/// (0xFB0), `x` absolute (0xFB1), the text (0xFC4), the target state (0xFF4), the fade start (0x1004).
const X: i32 = 0xfa1;
const WIDTH_SET: i32 = 0xfa2;
const VISIBLE: i32 = 0xfa3;
const ALPHA: i32 = 0xfa9;
const WIDTH: i32 = 0xfb0;
const ABSOLUTE_X: i32 = 0xfb1;
const TEXT: i32 = 0xfc4;
const TARGET_STATE: i32 = 0xff4;
const FADE_START: i32 = 0x1004;

/// The doubles in `.rdata`: 0.0, 2.0, 255.0, degrees per radian, 1,000.0 (seconds to ms), the
/// lower bars' horizontal adjustment, the subtitle's timeout, the heading tolerance of a new target;
/// the floats: the fade durations.
const ZERO: u32 = 0x0101_2060;
const TWO: u32 = 0x0101_1590;
const TWO_FIFTY_FIVE: u32 = 0x0101_e568;
const DEGREES_PER_RADIAN: u32 = 0x0102_f248;
const THOUSAND: u32 = 0x0101_7b70;
const BAR_ADJUSTMENT: u32 = 0x0101_db88;
const SUBTITLE_TIMEOUT: u32 = 0x0107_3480;
const HEADING_TOLERANCE: u32 = 0x0103_57e8;
const FADE_DURATION: u32 = 0x0101_62c0;
const SNEAK_FADE_DURATION: u32 = 0x0101_6248;
const MESSAGE_FADE_DURATION: u32 = 0x0102_1f44;
/// The settings: a new target's timeout in seconds (`011cf87c`, a float), the quest reminder's
/// delay in seconds (`011d2220`, an integer).
const TARGET_TIMEOUT: u32 = 0x011c_f87c;
const REMINDER_DELAY: u32 = 0x011d_2220;
/// The subtitle's hold trait name (`01073474`, its id through `00a01860`).
const HOLD_TRAIT_NAME: u32 = 0x0107_3474;
const TRAIT_ID: u32 = 0x00a0_1860;

/// The callees by address.
const PROCESS_OF: u32 = 0x008d_6f30;
const CLOCK_UPDATE: u32 = 0x0077_1520;
const UPDATE_COMPASS: u32 = 0x0077_9070;
const HUD_TILES_UPDATE: u32 = 0x0077_da60;
const WEAPON_UPDATE: u32 = 0x0077_21c0;
const BARS_UPDATE: u32 = 0x0077_e750;
const LATE_UPDATE: u32 = 0x0077_f700;
const SCREEN_WIDTH: u32 = 0x0071_5da0;
const SCREEN_RIGHT: u32 = 0x0071_5d40;
const COMPASS_WIDTH: u32 = 0x0071_77c0;
const SAFE_INSET: u32 = 0x0071_7820;
const REACT: u32 = 0x00a0_7530;
const FADE_FINISHED: u32 = 0x00a0_7fc0;
const START_FADE: u32 = 0x00a0_7c60;
const SET_TRAIT_TIME: u32 = 0x0071_5c60;
const UPDATE_ACTOR_VALUE: u32 = 0x0077_1540;
const XP_UPDATE: u32 = 0x0077_e4f0;
const SNEAKING: u32 = 0x0049_97b0;
const UPDATE_SNEAK: u32 = 0x0077_32d0;
const AMMO_UPDATE: u32 = 0x0077_a650;
const HOTKEYS_UPDATE: u32 = 0x0077_c4e0;
const RADIATION_UPDATE: u32 = 0x0077_4090;
const STATUS_MODE: u32 = 0x004d_1360;
const STATUS_MODE_UI: u32 = 0x0077_39f0;
const VATS_TARGETS: u32 = 0x0059_5d10;
const HEADING_BETWEEN: u32 = 0x0077_2840;
const SET_ENEMY_ACTOR: u32 = 0x0077_3190;
const IS_DEAD: u32 = 0x0047_7ba0;
const SET_INFO_FOR_REF: u32 = 0x0077_5a00;
const PERCEIVED_ENTRY: u32 = 0x0096_7090;
const DETECTS: u32 = 0x008b_c700;
const SHOWS_HEALTH: u32 = 0x0049_3bb0;
const UPDATE_ENEMY_HEALTH: u32 = 0x0077_01e0;
const OBJECT_STATE: u32 = 0x0057_9690;
const ACTIVE_QUEST: u32 = 0x005c_bb50;
const QUEST_REMINDER: u32 = 0x0077_36d0;
const GAME_STATE: u32 = 0x0045_8030;
const OBJECTIVES_DONE: u32 = 0x0077_f500;
const KILL_OBJECTIVES: u32 = 0x0077_f5e0;
const LIST_EMPTY: u32 = 0x0076_b610;
const CLEAR_SUBTITLE: u32 = 0x0077_5670;
const FREE: u32 = 0x0040_1030;
const ARRAY_REMOVE: u32 = 0x006b_f8f0;
const NEXT_SUBTITLE: u32 = 0x0077_5230;
const LIST_COUNT: u32 = 0x005a_e380;
const HIDE_MESSAGE: u32 = 0x0077_56e0;

/// JIP LN NVSE's places in the update: H21 at the HUD tiles' update (`0x7704AE`, a fix row), O14
/// at the sneak test (`0x770B0E`, `bSneakBoundingBoxFix`) and G10 at the corner message's display
/// time (`0x7714DA`, the longer message text).
const JIP_ORPHANED_TILES: u32 = 0x0077_04ae;
const JIP_SNEAK_TEST: u32 = 0x0077_0b0e;
const JIP_MESSAGE_TIME: u32 = 0x0077_14da;
/// The exe's corner message text buffer (0x104 bytes); its display time is 0x208 bytes after the
/// buffer's end (the icon and sound buffers between).
const MESSAGE_TEXT: u32 = 0x104;

fn player(e: &Engine) -> u32 {
    e.mem.u32(PLAYER)
}

fn hud(e: &Engine) -> u32 {
    e.mem.u32(HUD_MENU)
}

/// The current HUD's tile at `offset` (read again each time, as the exe does).
fn hud_tile(e: &Engine, offset: u32) -> u32 {
    e.mem.u32(hud(e) + offset)
}

fn is_true(e: &mut Engine, tile: u32, trait_id: i32) -> bool {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1230,
        tile::is_true(e, Ptr::new(tile), trait_id)
    )
}

fn set_text(e: &mut Engine, tile: u32, text: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1350,
        tile::set_string(e, Ptr::new(tile), TEXT, Ptr::new(text), 1)
    );
}

fn visible(e: &mut Engine, offset: u32) -> bool {
    let tile = hud_tile(e, offset);
    is_true(e, tile, VISIBLE)
}

/// The enemy health bar's test of the action prompt (`0x770E62`..`0x770E78`): the prompt tile's
/// (+0x9C) visibility is asked; shown, it hides the bar, unless lStewieAl's Tweaks' S0022.42
/// (`HUD_HEALTH_BAR_NOT_TIED_TO_PROMPT`, a NOP over the `JNZ`) is on.
fn prompt_hides_health_bar(e: &mut Engine) -> bool {
    let shown = visible(e, 0x9c);
    shown && !e.stewie(engine_core::extend::stewie::HUD_HEALTH_BAR_NOT_TIED_TO_PROMPT)
}

fn alpha_of(e: &mut Engine, offset: u32) -> f32 {
    let tile = hud_tile(e, offset);
    get_float(e, tile, ALPHA)
}

fn fade_finished(e: &mut Engine, offset: u32) -> bool {
    let tile = hud_tile(e, offset);
    e.call(FADE_FINISHED, &args![tile, ALPHA]).u8() != 0
}

fn start_fade(e: &mut Engine, tile: u32, from: f32, to: f32, duration: f32) {
    e.call(START_FADE, &args![tile, ALPHA, from, to, duration, 0u32]);
}

fn ticks(e: &mut Engine, clock: u32) -> u32 {
    e.call(TIMER_TICKS, &args![clock]).u32()
}

/// A float setting's value through `00403e20`.
fn float_setting(e: &mut Engine, at: u32) -> f32 {
    let value = e.call(FLOAT_SETTING, &args![at]).u32();
    e.mem.f32(value)
}

/// The HUD opacity times 255 (`FLD; FMUL double; FSTP float`).
fn opacity_alpha(e: &mut Engine) -> f32 {
    let opacity = float_setting(e, HUD_OPACITY);
    narrow(f64::from(opacity) * global_f64(e, TWO_FIFTY_FIVE))
}

/// `_ftol2` of a double the exe has on the x87 stack (an `ST0` answer of a settings getter).
fn to_int(e: &mut Engine, value: f64) -> i32 {
    let low = typed_call!(crt::FAMILY, 0x00ec_62c0, crt::math::ftol2_sse(e, value));
    i32::from_ne_bytes(crt::low_word(low).to_ne_bytes())
}

/// A float in seconds as milliseconds (`FLD float; FMUL double 1000.0`: one product at the main
/// thread's 24-bit precision).
fn seconds_to_ms(e: &Engine, seconds: f32) -> f64 {
    f64::from(narrow(
        f64::from(x87_quiet(seconds)) * global_f64(e, THOUSAND),
    ))
}

/// `FISTP qword` under a truncating control word (`OR 0xC00`), its low word.
fn truncate_low(value: f64) -> u32 {
    let bytes = fistp_chop64(value).to_le_bytes();
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// The HUD tiles' update (`0x7704AE`): JIP's H21 seam first while JIP's fixes are in (its answer
/// not read), then the exe's call.
fn hud_tiles_update(e: &mut Engine, this: u32) {
    if e.jip_patches() & engine_core::extend::jip_patches::FIXES != 0 {
        e.site(JIP_ORPHANED_TILES, &[this]);
    }
    e.call(HUD_TILES_UPDATE, &args![this]);
}

// Typed from 00770430
/// `HUDMainMenu::DoIdle` (Xbox PDB; `thiscall`): nothing without the player or its process; the
/// compass and the per-frame sub-updates; then, at most every 200 ms by the HUD's clock, the rest.
pub fn do_idle(e: &mut Engine, this: Ptr) {
    let me = player(e);
    if me == 0 || e.call(PROCESS_OF, &args![me]).u32() == 0 {
        return;
    }
    let this = this.addr();
    e.call(CLOCK_UPDATE, &args![this + 0x224]);
    let scene = e.mem.u32(this + 0x40);
    let landmarks = e.mem.u32(this + 0x34);
    let objectives = e.mem.u32(this + 0x38);
    let ticks_pool = e.mem.u32(this + 0x3c);
    e.call(
        UPDATE_COMPASS,
        &args![scene, landmarks, objectives, ticks_pool],
    );
    hud_tiles_update(e, this);
    e.call(WEAPON_UPDATE, &args![this]);
    e.call(BARS_UPDATE, &args![this]);
    e.call(LATE_UPDATE, &args![this]);
    let guard = e.mem.u32(LAST_TICK_GUARD);
    if guard & 1 == 0 {
        e.mem.set_u32(LAST_TICK_GUARD, guard | 1);
        let now = ticks(e, this + 0x224);
        e.mem.set_u32(LAST_TICK, now);
    }
    let now = ticks(e, this + 0x224);
    if now.wrapping_sub(e.mem.u32(LAST_TICK)) < 200 {
        return;
    }
    for (timer, first, second) in [(0x24c, 0x154, 0x170), (0x250, 0x158, 0x174)] {
        let started = e.mem.u32(this + timer);
        let expired = started == u32::MAX || ticks(e, TIMER).wrapping_sub(started) > 1000;
        if expired {
            e.mem.set_u32(this + timer, u32::MAX);
            for offset in [first, second] {
                let tile = hud_tile(e, offset);
                set_int(e, tile, VISIBLE, 0);
            }
        }
    }
    let now = ticks(e, this + 0x224);
    e.mem.set_u32(LAST_TICK, now);
    geometry(e, this);
    fades(e);
    if !matches!(e.mem.u32(hud(e) + 0x1c4), 0x12 | 0x13) {
        e.call(AMMO_UPDATE, &args![this]);
    }
    e.call(HOTKEYS_UPDATE, &args![this]);
    e.call(RADIATION_UPDATE, &args![this]);
    let me = player(e);
    let status = e.call(STATUS_MODE, &args![me]).u8() != 0;
    e.call(STATUS_MODE_UI, &args![this, status]);
    target(e, this);
    quests(e, this);
    messages(e, this);
    let alpha = opacity_alpha(e);
    e.mem.set_f32(HUD_ALPHA, alpha);
}

/// The two lower bars (+0x118, +0x114) placed by the screen's width, the compass's width and the
/// safe inset; the action points and health values; the experience; the damage tiles' alpha.
fn geometry(e: &mut Engine, this: u32) {
    let width = e.call(SCREEN_WIDTH, &args![]).f64();
    let width = to_int(e, width);
    let right = e.call(SCREEN_RIGHT, &args![]).f64();
    let right = to_int(e, right);
    let compass = e.call(COMPASS_WIDTH, &args![]).f64();
    let compass = to_int(e, compass);
    let inset = e.call(SAFE_INSET, &args![]).f64();
    let inset = to_int(e, inset);
    let lower = hud_tile(e, 0x118);
    set_int(e, lower, X, compass.wrapping_mul(2).wrapping_add(10));
    let span = f64::from(width.wrapping_sub(inset.wrapping_mul(2)));
    let lower = hud_tile(e, 0x118);
    let current = get_float(e, lower, WIDTH);
    let lower = hud_tile(e, 0x118);
    set_float(e, lower, WIDTH_SET, narrow(span - f64::from(current)));
    let upper = hud_tile(e, 0x114);
    let current_x = get_float(e, upper, ABSOLUTE_X);
    // 007706c7..007706d0: FSUBR float, FADD double, FSTP float (each at 24 bits).
    let left = f64::from(right.wrapping_sub(compass.wrapping_mul(2)));
    let moved = narrow(left - f64::from(current_x));
    let x = narrow(f64::from(moved) + global_f64(e, BAR_ADJUSTMENT));
    let upper = hud_tile(e, 0x114);
    set_float(e, upper, X, x);
    let upper = hud_tile(e, 0x114);
    let current = get_float(e, upper, WIDTH);
    let upper = hud_tile(e, 0x114);
    set_float(e, upper, WIDTH_SET, narrow(span - f64::from(current)));
    for offset in [0x118, 0x114] {
        let tile = hud_tile(e, offset);
        e.call(REACT, &args![tile, 1u32]);
    }
    e.call(UPDATE_ACTOR_VALUE, &args![this, 0x10u32]);
    e.call(UPDATE_ACTOR_VALUE, &args![this, 0x0cu32]);
    e.call(XP_UPDATE, &args![this]);
    for offset in [0x180, 0x160, 0x154, 0x170, 0x174, 0x158, 0x15c] {
        let alpha = x87_quiet(e.mem.f32(HUD_ALPHA));
        let tile = hud_tile(e, offset);
        set_float(e, tile, ALPHA, alpha);
    }
}

/// The region text (+0x64) and the subtitle (+0x68) fade out once shown past the timeout; the region
/// text fades in on its hold trait when the compass shows and the subtitle is gone; the sneak meter.
fn fades(e: &mut Engine) {
    for (offset, from_setting) in [(0x64u32, false), (0x68, true)] {
        if f64::from(alpha_of(e, offset)) > global_f64(e, ZERO) && fade_finished(e, offset) {
            let now = ticks(e, TIMER);
            let tile = hud_tile(e, offset);
            let start = get_float(e, tile, FADE_START);
            if f64::from(now) - f64::from(start) > global_f64(e, SUBTITLE_TIMEOUT) {
                let alpha = if from_setting {
                    opacity_alpha(e)
                } else {
                    alpha_of(e, offset)
                };
                let duration = x87_quiet(e.mem.f32(FADE_DURATION));
                let tile = hud_tile(e, offset);
                start_fade(e, tile, alpha, 0.0, duration);
            }
        }
    }
    let hold = e.call(TRAIT_ID, &args![HOLD_TRAIT_NAME]).u32();
    let text = hud_tile(e, 0x64);
    if is_true(e, text, hold as i32) {
        let compass = hud_tile(e, 0x128);
        if f64::from(get_float(e, compass, VISIBLE)) != global_f64(e, ZERO) {
            let subtitle = hud_tile(e, 0x68);
            if f64::from(get_float(e, subtitle, ALPHA)) == global_f64(e, ZERO)
                && fade_finished(e, 0x68)
            {
                let duration = x87_quiet(e.mem.f32(FADE_DURATION));
                let alpha = opacity_alpha(e);
                let tile = hud_tile(e, 0x64);
                start_fade(e, tile, 0.0, alpha, duration);
                let now = ticks(e, TIMER);
                let tile = hud_tile(e, 0x64);
                e.call(SET_TRAIT_TIME, &args![tile, FADE_START, now]);
                let hold = e.call(TRAIT_ID, &args![HOLD_TRAIT_NAME]).u32();
                let tile = hud_tile(e, 0x64);
                set_int(e, tile, hold as i32, 0);
            }
        }
    }
    let me = player(e);
    // JIP LN NVSE's O14 at `0x770B0E` (`bSneakBoundingBoxFix`): the seam with the player answers in
    // its low byte in place of the exe's call, while that switch is on.
    let jip = if e.jip_patches() & engine_core::extend::jip_patches::SNEAK_BOUNDING_BOX_FIX != 0 {
        e.site(JIP_SNEAK_TEST, &[me])
    } else {
        None
    };
    let sneaking = match jip {
        Some(answer) => answer & 0xff != 0,
        None => e.call(SNEAKING, &args![me]).u8() != 0,
    };
    if sneaking {
        let menu = hud(e);
        e.call(UPDATE_SNEAK, &args![menu]);
    } else if f64::from(alpha_of(e, 0x88)) > global_f64(e, ZERO) && fade_finished(e, 0x88) {
        let duration = x87_quiet(e.mem.f32(SNEAK_FADE_DURATION));
        let alpha = alpha_of(e, 0x88);
        let tile = hud_tile(e, 0x88);
        start_fade(e, tile, alpha, 0.0, duration);
    }
}

/// The enemy target (+0x21C): whether it is in the VATS list; its heading against the player's; the
/// crosshair info or the enemy actor shown; its detection colour; its health bar; the last crosshair
/// object's info again when its state changed.
fn target(e: &mut Engine, this: u32) {
    e.mem.set_u8(ENEMY_IN_VATS, 0);
    if e.mem.u32(this + 0x21c) != 0 {
        let me = player(e);
        let mut entry = e.call(VATS_TARGETS, &args![me]).u32();
        while entry != 0 && !typed_call!(leaf::FAMILY, 0x0082_56d0, leaf::both_words_zero(e, entry))
        {
            let first = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, entry));
            let object = e.mem.u32(first);
            if e.mem.u32(object) == e.mem.u32(this + 0x21c)
                && e.mem.u8(object + 4) != 0
                && e.mem.u8(object + 5) == 0
            {
                e.mem.set_u8(ENEMY_IN_VATS, 1);
                break;
            }
            entry = typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, entry));
        }
    }
    let me = player(e);
    let heading = e.vcall(me, 0x2bc, &args![0u32]).f64();
    let heading = narrow(heading * global_f64(e, DEGREES_PER_RADIAN));
    let mut target_heading = 0.0f32;
    let enemy = e.mem.u32(this + 0x21c);
    if enemy != 0 {
        let to = e.vcall(enemy, 0x1f4, &args![]).u32();
        let me = player(e);
        let from = e.vcall(me, 0x1f4, &args![]).u32();
        target_heading = narrow(e.call(HEADING_BETWEEN, &args![from, to]).f64());
    }
    let now = ticks(e, this + 0x224);
    let elapsed = now.wrapping_sub(e.mem.u32(this + 0x244));
    let timeout_seconds = float_setting(e, TARGET_TIMEOUT);
    let timeout = truncate_low(seconds_to_ms(e, timeout_seconds));
    let close_enough = if elapsed <= timeout {
        true
    } else {
        let difference = narrow(f64::from(heading) - f64::from(target_heading));
        let angle = abs(e, difference);
        f64::from(angle) < global_f64(e, HEADING_TOLERANCE)
    };
    let mut show_enemy = false;
    if typed_call!(
        leaf::FAMILY,
        0x0044_ddc0,
        leaf::word_at_8(e, INTERFACE_MODE)
    ) == 4
    {
        let actor = e.mem.u32(MODE_ACTOR);
        if actor != 0 && e.vcall(actor, 0x100, &args![]).u8() != 0 {
            let actor = e.mem.u32(MODE_ACTOR);
            if e.vcall(actor, 0x22c, &args![0u32]).u8() == 0 {
                show_enemy = true;
                let actor = e.mem.u32(MODE_ACTOR);
                e.call(SET_ENEMY_ACTOR, &args![actor]);
            }
        }
    } else {
        let enemy = e.mem.u32(this + 0x21c);
        let mut show_ref = false;
        if enemy != 0 && e.vcall(enemy, 0x22c, &args![0u32]).u8() == 0 {
            let enemy = e.mem.u32(this + 0x21c);
            // `0x770E78` (lStewieAl's Tweaks' S0022.42 writes a NOP over the `JNZ`): with the row
            // on the prompt tile's (+0x9C) visibility is still asked and its answer not used.
            if e.call(IS_DEAD, &args![enemy]).u8() == 0
                && e.mem.u8(ENEMY_IN_VATS) != 0
                && !prompt_hides_health_bar(e)
                && !visible(e, 0xa8)
            {
                let tile = hud_tile(e, 0x120);
                let state = get_float(e, tile, TARGET_STATE);
                if f64::from(state) == global_f64(e, TWO) && close_enough {
                    show_ref = true;
                }
            }
        }
        if show_ref {
            show_enemy = true;
            let last = e.mem.u32(this + 0x1b8);
            e.call(SET_INFO_FOR_REF, &args![last, 0u32, 1u32]);
        } else {
            let tile = hud_tile(e, 0x120);
            let state = get_float(e, tile, TARGET_STATE);
            if f64::from(state) == global_f64(e, TWO) {
                for offset in [0x7c, 0x80, 0x84] {
                    let tile = e.mem.u32(this + offset);
                    set_int(e, tile, VISIBLE, 0);
                }
            }
        }
    }
    if show_enemy {
        let mut perceived = false;
        let enemy = e.mem.u32(this + 0x21c);
        if enemy != 0 {
            let me = player(e);
            if e.call(PERCEIVED_ENTRY, &args![me, enemy]).u32() != 0 {
                let me = player(e);
                let enemy = e.mem.u32(this + 0x21c);
                let entry = e.call(PERCEIVED_ENTRY, &args![me, enemy]).u32();
                perceived = e.mem.u8(entry + 4) != 0;
            }
            if !perceived {
                let enemy = e.mem.u32(this + 0x21c);
                let me = player(e);
                perceived = e.call(DETECTS, &args![enemy, me]).u8() != 0;
            }
        }
        for offset in [0x7c, 0x80, 0x84] {
            let tile = e.mem.u32(this + offset);
            set_int(e, tile, VISIBLE, 1);
        }
        let tile = hud_tile(e, 0x120);
        set_int(e, tile, TARGET_STATE, i32::from(perceived) + 1);
        for offset in [0x7c, 0x80, 0x84, 0x84] {
            let tile = hud_tile(e, offset);
            e.call(REACT, &args![tile, 8u32]);
        }
    }
    let enemy = e.mem.u32(this + 0x21c);
    if enemy != 0 {
        let shows = e.call(SHOWS_HEALTH, &args![enemy]).u8() != 0
            || typed_call!(
                leaf::FAMILY,
                0x0044_ddc0,
                leaf::word_at_8(e, INTERFACE_MODE)
            ) == 4;
        if shows {
            let enemy = e.mem.u32(this + 0x21c);
            e.call(UPDATE_ENEMY_HEALTH, &args![this, enemy]);
        }
    }
    let last = e.mem.u32(this + 0x1b8);
    if last != 0 {
        let previous = e.mem.u8(LAST_OBJECT_STATE);
        let current = e.call(OBJECT_STATE, &args![last]).u8();
        if previous != current {
            let last = e.mem.u32(this + 0x1b8);
            let current = e.call(OBJECT_STATE, &args![last]).u8();
            e.mem.set_u8(LAST_OBJECT_STATE, current);
            let last = e.mem.u32(this + 0x1b8);
            e.call(SET_INFO_FOR_REF, &args![last, 0u32, 1u32]);
        }
    }
}

/// The quest reminder (forced, or on the first frame of play); the quest updates' kill; the
/// subtitles whose time has passed taken off the list; the next subtitle shown.
fn quests(e: &mut Engine, this: u32) {
    let mut remind = false;
    if e.mem.u8(this + 0x205) != 0 {
        let now = ticks(e, TIMER);
        let elapsed = now.wrapping_sub(e.mem.u32(this + 0x248));
        let delay = (setting(e, REMINDER_DELAY) as u32).wrapping_mul(1000);
        if elapsed > delay {
            let mode = e.mem.u32(hud(e) + 0x1c4);
            if mode != 0x12 && mode != 0x13 {
                remind = true;
            }
        }
    }
    if remind {
        let now = ticks(e, TIMER);
        e.mem.set_u32(this + 0x248, now);
        let me = player(e);
        let quest = e.call(ACTIVE_QUEST, &args![me]).u32();
        e.call(QUEST_REMINDER, &args![quest]);
    } else if e.mem.u32(this + 0x248) == 0 && e.call(GAME_STATE, &args![]).u32() == 2 {
        let now = ticks(e, TIMER);
        e.mem.set_u32(this + 0x248, now);
        let me = player(e);
        let quest = e.call(ACTIVE_QUEST, &args![me]).u32();
        e.call(QUEST_REMINDER, &args![quest]);
        e.mem.set_u8(this + 0x254, 0);
    }
    if e.mem.u8(this + 0x254) != 0 {
        e.call(OBJECTIVES_DONE, &args![this]);
        e.call(KILL_OBJECTIVES, &args![this]);
        e.mem.set_u8(this + 0x254, 0);
        return;
    }
    let subtitles = this + 0x198;
    if e.call(LIST_EMPTY, &args![subtitles]).u8() != 0 {
        return;
    }
    let mut index = 0u32;
    loop {
        let count = typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, subtitles));
        if index >= count {
            break;
        }
        let now = ticks(e, this + 0x224);
        let entry = typed_call!(
            leaf::FAMILY,
            0x0087_7a30,
            leaf::array_element(e, subtitles, index)
        );
        let message = e.mem.u32(entry);
        let elapsed = now.wrapping_sub(e.mem.u32(message + 0x104));
        let entry = typed_call!(
            leaf::FAMILY,
            0x0087_7a30,
            leaf::array_element(e, subtitles, index)
        );
        let message = e.mem.u32(entry);
        let duration = seconds_to_ms(e, e.mem.f32(message + 0x108));
        if duration <= f64::from(elapsed) {
            let entry = typed_call!(
                leaf::FAMILY,
                0x0087_7a30,
                leaf::array_element(e, subtitles, index)
            );
            if e.mem.u32(entry) == e.mem.u32(this + 0x1a8) {
                e.call(CLEAR_SUBTITLE, &args![this]);
            }
            let entry = typed_call!(
                leaf::FAMILY,
                0x0087_7a30,
                leaf::array_element(e, subtitles, index)
            );
            let message = e.mem.u32(entry);
            e.call(FREE, &args![message]);
            e.call(ARRAY_REMOVE, &args![subtitles, index, 1u32]);
        } else {
            index = index.wrapping_add(1);
        }
    }
    if e.mem.u32(this + 0x1a8) == 0 {
        let text = e.call(NEXT_SUBTITLE, &args![this]).u32();
        e.mem.set_u32(this + 0x1a8, text);
        if e.mem.u32(this + 0x1a8) != 0 {
            let text = e.mem.u32(this + 0x1a8);
            let tile = hud_tile(e, 0x98);
            set_text(e, tile, text);
            let tile = hud_tile(e, 0x98);
            set_int(e, tile, VISIBLE, 1);
        }
    }
}

/// The corner messages (+0x18C): faded in when their tiles are clear; the first one hidden once its
/// display time has passed.
fn messages(e: &mut Engine, this: u32) {
    if e.call(LIST_COUNT, &args![this + 0x18c]).u32() == 0 || !visible(e, 0x134) {
        return;
    }
    if f64::from(alpha_of(e, 0x94)) == global_f64(e, ZERO) {
        for offset in [0x94, 0x8c, 0x90] {
            let alpha = x87_quiet(e.mem.f32(HUD_ALPHA));
            let duration = x87_quiet(e.mem.f32(MESSAGE_FADE_DURATION));
            let tile = hud_tile(e, offset);
            start_fade(e, tile, 0.0, alpha, duration);
        }
    }
    if fade_finished(e, 0x94) {
        let first = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, this + 0x18c));
        let message = e.mem.u32(first);
        if e.mem.u32(this + 0x194) == 0 {
            let now = ticks(e, this + 0x224);
            e.mem.set_u32(this + 0x194, now);
        }
        let now = ticks(e, this + 0x224);
        let elapsed = now.wrapping_sub(e.mem.u32(this + 0x194));
        // JIP LN NVSE's G10 at `0x7714DA` (a fix row): the text buffer's size, so the display
        // time's place. One load while JIP's fixes are not in.
        let text = if e.jip_patches() & engine_core::extend::jip_patches::FIXES != 0 {
            e.site(JIP_MESSAGE_TIME, &[MESSAGE_TEXT])
                .unwrap_or(MESSAGE_TEXT)
        } else {
            MESSAGE_TEXT
        };
        let duration = seconds_to_ms(e, e.mem.f32(message + text + 0x208));
        if duration <= f64::from(elapsed) {
            e.call(HIDE_MESSAGE, &args![this]);
        }
    }
}
