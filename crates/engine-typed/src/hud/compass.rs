//! The HUD main menu's compass: the heading strip, the map markers, the quest targets, the player's
//! own marker and the actors' ticks, each a tile from one of the compass's pools placed by its angle
//! to the player, faded towards the strip's ends and blinking when close.

use crate::dispatch::tile;
use crate::{crt, dispatch, pathing};
use engine_core::ptr::Ptr;
use engine_core::typed::leaf;
use engine_core::x87::x87_quiet;
use engine_core::{args, typed_call, Engine};

/// The player (`PlayerCharacter *`) and the HUD main menu (`HUDMainMenu *`).
const PLAYER: u32 = 0x011d_ea3c;
const HUD_MENU: u32 = 0x011d_96c0;
/// The HUD menu's compass width in pixels (+0x214), its half angle in degrees (+0x218) and the quest
/// target list it keeps (+0x20c).
const HUD_WIDTH: u32 = 0x214;
const HUD_HALF_ANGLE: u32 = 0x218;
const HUD_TARGETS: u32 = 0x20c;
/// The HUD's alpha (`011d979c`, a float the HUD's update sets each frame) and the byte the compass
/// sets when an actor tick shows a hostile (`011d96c4`).
const HUD_ALPHA: u32 = 0x011d_979c;
const HOSTILE_SHOWN: u32 = 0x011d_96c4;

/// The compass's user traits, made on the first call (bits 1, 2, 4 of the guard word `011d9e68`):
/// `_Distance`, `_Heading`, `_AlphaDown`.
const TRAITS_GUARD: u32 = 0x011d_9e68;
const DISTANCE_TRAIT: u32 = 0x011d_9e64;
const HEADING_TRAIT: u32 = 0x011d_9e60;
const ALPHA_DOWN_TRAIT: u32 = 0x011d_9e5c;
const DISTANCE_NAME: u32 = 0x0107_3618;
const HEADING_NAME: u32 = 0x0107_360c;
const ALPHA_DOWN_NAME: u32 = 0x0107_3500;
/// `BlinkTile`'s own `_AlphaDown` (its guard bit 1 of `011d9e58`).
const BLINK_GUARD: u32 = 0x011d_9e58;
const BLINK_ALPHA_DOWN: u32 = 0x011d_9e54;
/// `Tile::AddUserTrait(name, id)` (cdecl).
const ADD_USER_TRAIT: u32 = 0x00a0_0940;

/// The tile traits: `visible` (`0xfa3`), `alpha` (`0xfa9`), `filename` (`0xfcc`), the tick's
/// colour (`0xff4`) and the blink's next time (`0x100e`).
const VISIBLE: i32 = 0xfa3;
const ALPHA: i32 = 0xfa9;
const FILENAME: i32 = 0xfcc;
const TICK_COLOUR: i32 = 0xff4;
const BLINK_NEXT: i32 = 0x100e;

/// The textures: a landmark, a discovered landmark, a quest target, the player's marker, an actor tick.
const LANDMARK_TEXTURE: u32 = 0x0107_35a4;
const DISCOVERED_TEXTURE: u32 = 0x0107_35d0;
const OBJECTIVE_TEXTURE: u32 = 0x0107_3570;
const PLAYER_MARKER_TEXTURE: u32 = 0x0107_3534;
const TICK_TEXTURE: u32 = 0x0107_350c;

/// The game settings the compass reads (each an integer at +4 of its setting): the landmark range,
/// the quest target range, the actor tick cap, the experience of a discovered location; and the
/// blink's: its longest and shortest period, the breakpoint between them, and the timings and delays
/// at the three.
const LANDMARK_RANGE: u32 = 0x011d_12cc;
const OBJECTIVE_RANGE: u32 = 0x011d_0bd8;
const TICK_CAP: u32 = 0x011d_32d0;
const DISCOVERY_XP: u32 = 0x011d_0364;
const BLINK_LONGEST: u32 = 0x011d_1d54;
const BLINK_SHORTEST: u32 = 0x011d_30d8;
const BLINK_BREAKPOINT: u32 = 0x011d_4d44;
const BLINK_TIMING_LONGEST: u32 = 0x011d_2bd4;
const BLINK_TIMING_BREAKPOINT: u32 = 0x011d_3c94;
const BLINK_TIMING_SHORTEST: u32 = 0x011d_30f0;
const BLINK_DELAY_LONGEST: u32 = 0x011d_24f0;
const BLINK_DELAY_BREAKPOINT: u32 = 0x011d_46cc;
const BLINK_DELAY_SHORTEST: u32 = 0x011d_3924;
/// The float settings: the HUD opacity (`011d9774`), the tick range in the actor mode (`011cd7d8`) and
/// its scale otherwise (`011cdcbc`), read through `00403e20` (the setting's value address).
const HUD_OPACITY: u32 = 0x011d_9774;
const TICK_RANGE: u32 = 0x011c_d7d8;
const TICK_RANGE_SCALE: u32 = 0x011c_dcbc;
const FLOAT_SETTING: u32 = 0x0040_3e20;
/// The interface's "HUD opacity is fixed" word (+8 of `011f2250`).
const OPACITY_FIXED: u32 = 0x011f_2250;
/// The quest target switch (`011e07f0`, its byte through `00408d60`).
const TARGETS_SHOWN: u32 = 0x011e_07f0;
const SETTING_BYTE: u32 = 0x0040_8d60;

/// The game's timer (`011f6394`): its frame time (`0084d030`, the float at +0xc) and its tick count
/// (`00825c00`, the word at +0x14).
const TIMER: u32 = 0x011f_6394;
const TIMER_DELTA: u32 = 0x0084_d030;
const TIMER_TICKS: u32 = 0x0082_5c00;

/// The doubles in `.rdata` the compass computes and compares with.
const ZERO: u32 = 0x0101_2060;
const ONE: u32 = 0x0101_2070;
const TWO: u32 = 0x0101_1590;
const THREE: u32 = 0x0102_1928;
const TWO_FIFTY_FIVE: u32 = 0x0101_e568;
const DEGREES_PER_RADIAN: u32 = 0x0102_f248;
const FULL_TURN: u32 = 0x0103_c348;
const STRIP_SHIFT: u32 = 0x0107_0c80;
const INNER_SHARE: u32 = 0x0102_17d8;
const BLINK_RANGE: u32 = 0x0102_f078;
const OBJECTIVE_OVERSHOOT: u32 = 0x0107_3568;
/// The float of the strip's width in degrees (`01073608`).
const STRIP_DEGREES: u32 = 0x0107_3608;
/// `GetCompassAngle`'s turn: below -pi add a full turn, above pi take it from a full turn; then to
/// degrees and the compass's scale.
const ANGLE_LOW: u32 = 0x0101_ff58;
const ANGLE_TURN: u32 = 0x0101_ff48;
const ANGLE_HIGH: u32 = 0x0101_ff40;
const ANGLE_SCALE: u32 = 0x0107_34f8;

/// The callees by address.
const PROCESS_OF: u32 = 0x008d_6f30;
const PROCESS_HEADING: u32 = 0x0055_5ad0;
const PLAYER_MARKERS: u32 = 0x0094_8030;
const MARKER_TRAVEL: u32 = 0x0043_8ef0;
const MARKER_VISIBLE: u32 = 0x0043_8ed0;
const MARKER_SET_VISIBLE: u32 = 0x0044_de40;
const MARKER_SET_TRAVEL: u32 = 0x0044_de80;
const MARKER_NAME: u32 = 0x0043_8eb0;
const REFERENCE_RADIUS: u32 = 0x0056_8cb0;
const SETTING_STRING: u32 = 0x0040_3df0;
const DISCOVERED_TITLE: u32 = 0x011d_2208;
const DISCOVERED_SOUND: u32 = 0x0106_f370;
const SHOW_MESSAGE: u32 = 0x0076_b960;
const EVENT: u32 = 0x005f_5950;
const STAT_INCREMENT: u32 = 0x004d_5c60;
const MARKER_HELPER: u32 = 0x0077_2aa0;
const TARGET_LIST: u32 = 0x0095_2ba0;
const TARGET_REFERENCE: u32 = 0x005c_bb70;
const FORM_TYPE: u32 = 0x0040_1170;
const CLAMP: u32 = 0x0040_ebd0;
const TILE_MENU: u32 = 0x00a0_3c90;
const MENU_SHOWN: u32 = 0x0059_bb30;
const VATS_TARGETS: u32 = 0x0059_5d10;
const ACTOR_MODE: u32 = 0x0042_5fd0;
const LIST_COUNT: u32 = 0x005a_e380;
const LIST_SORT: u32 = 0x0083_fd60;
const COMPASS_ORDER: u32 = 0x0077_b300;
const PROCESS_ACTIVE: u32 = 0x0045_0ff0;
const ENTRY_POINT: u32 = 0x005e_58f0;
const HEADING_OF: u32 = 0x004b_1550;
const LIST_ITEM: u32 = 0x0055_8dd0;
/// The pools' iterators: the landmark and tick pools' (`0057cbe0`), the targets' (`00683520`).
const POOL_NEXT: u32 = 0x0057_cbe0;
const OBJECTIVE_POOL_NEXT: u32 = 0x0068_3520;
/// JIP LN's seam at the location name (`0077951e`).
const JIP_DISCOVER_LOCATION: u32 = 0x0077_951e;
const JIP_LOCATION_DISCOVERED: u32 = 0x0077_955d;

/// The player's virtual slots: heading (+0x2bc), position (+0x1f4), sneaking (+0x22c), the
/// experience reward (+0x488); a reference's position (+0x1f4) and its "changed" mark (+0x48); an
/// actor's 3D (+0x1d0); the compass tile's strip (+0x20); the actor value owner's value (+8).
const SLOT_HEADING: u32 = 0x2bc;
const SLOT_POSITION: u32 = 0x1f4;
const SLOT_SNEAKING: u32 = 0x22c;
const SLOT_REWARD: u32 = 0x488;
const SLOT_MARK_CHANGED: u32 = 0x48;
const SLOT_3D: u32 = 0x1d0;
const SLOT_STRIP: u32 = 0x20;
const SLOT_VALUE: u32 = 0x08;

fn player(e: &Engine) -> u32 {
    e.mem.u32(PLAYER)
}

fn hud(e: &Engine) -> u32 {
    e.mem.u32(HUD_MENU)
}

pub(super) fn global_f64(e: &Engine, at: u32) -> f64 {
    e.mem.f64(at)
}

/// An integer game setting's value (`0043d4d0` and the read at the address it answers; the
/// settings named here are never null).
pub(super) fn setting(e: &Engine, at: u32) -> i32 {
    e.mem.i32(at + 4)
}

/// A float setting's value through `00403e20`.
fn float_setting(e: &mut Engine, at: u32) -> f32 {
    let value = e.call(FLOAT_SETTING, &args![at]).u32();
    e.mem.f32(value)
}

/// An integer as `FILD` holds it, for the exact comparisons.
pub(super) fn exact(value: i32) -> f64 {
    f64::from(value)
}

/// An integer as the float `FILD; FSTP float` makes of it.
#[allow(clippy::cast_precision_loss)]
fn as_float(value: i32) -> f32 {
    value as f32
}

/// A double result as the float the exe stores.
#[allow(clippy::cast_possible_truncation)]
pub(super) fn narrow(value: f64) -> f32 {
    value as f32
}

/// `004b3ab0`: `(x - low) / (high - low) * (b - a) + a`, in doubles and rounded to a float once at
/// the end. Each step rounded to a float differs by up to about 20 ulps where `x - low` cancels
/// (the review of #874's fixed headings found it); the doubles agree with the translation.
fn interpolate(a: f32, b: f32, low: f32, high: f32, x: f32) -> f32 {
    let ratio = (f64::from(x) - f64::from(low)) / (f64::from(high) - f64::from(low));
    narrow(ratio * (f64::from(b) - f64::from(a)) + f64::from(a))
}

/// A reference's position (its slot +0x1f4 answers the address of three floats).
fn position_of(e: &mut Engine, reference: u32) -> [f32; 3] {
    let at = e.vcall(reference, SLOT_POSITION, &args![]).u32();
    [e.mem.f32(at), e.mem.f32(at + 4), e.mem.f32(at + 8)]
}

/// The squared distance `|from - to|`: `00439ef0`'s differences, `004a7290`'s `(x*x + y*y) + z*z`
/// (each a float), then `std::abs` (`00408840`, by its typed call: `fabs` stays the C runtime's).
fn distance_squared(e: &mut Engine, from: [f32; 3], to: [f32; 3]) -> f32 {
    let x = from[0] - to[0];
    let y = from[1] - to[1];
    let z = from[2] - to[2];
    let squared = (x * x + y * y) + z * z;
    typed_call!(
        pathing::FAMILY,
        0x0040_8840,
        pathing::point::abs(e, squared)
    )
}

pub(super) fn abs(e: &mut Engine, value: f32) -> f32 {
    typed_call!(pathing::FAMILY, 0x0040_8840, pathing::point::abs(e, value))
}

pub(super) fn set_float(e: &mut Engine, tile: u32, trait_id: i32, value: f32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_12d0,
        tile::set_float(e, Ptr::new(tile), trait_id, value, 1)
    );
}

pub(super) fn set_int(e: &mut Engine, tile: u32, trait_id: i32, value: i32) {
    typed_call!(
        dispatch::FAMILY,
        0x0070_0320,
        tile::set_int(e, Ptr::new(tile), trait_id, value)
    );
}

pub(super) fn get_float(e: &mut Engine, tile: u32, trait_id: i32) -> f32 {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_11b0,
        tile::get_float(e, Ptr::new(tile), trait_id)
    )
}

fn set_texture(e: &mut Engine, tile: u32, texture: u32) {
    typed_call!(
        dispatch::FAMILY,
        0x00a0_1350,
        tile::set_string(e, Ptr::new(tile), FILENAME, Ptr::new(texture), 1)
    );
}

/// A user trait's id (`Tile::AddUserTrait(name, -1)`), made once behind bit `bit` of `guard`.
fn user_trait(e: &mut Engine, guard: u32, bit: u32, slot: u32, name: u32) {
    let flags = e.mem.u32(guard);
    if flags & bit == 0 {
        e.mem.set_u32(guard, flags | bit);
        let id = e.call(ADD_USER_TRAIT, &args![name, -1i32]).u32();
        e.mem.set_u32(slot, id);
    }
}

/// The trait id in `slot` (a word the compass keeps), as the setters take it.
fn trait_in(e: &Engine, slot: u32) -> i32 {
    e.mem.i32(slot)
}

/// The next tile of a pool (its iterator `next` through the cursor's address), and the cursor
/// moved on.
fn take_tile(e: &mut Engine, pool: u32, next: u32, cursor: &mut u32) -> u32 {
    e.with_stack(4, |e, slot| {
        e.mem.set_u32(slot.addr(), *cursor);
        let entry = e.call(next, &args![pool + 4, slot]).u32();
        *cursor = e.mem.u32(slot.addr());
        e.mem.u32(entry)
    })
}

/// The list node's element (`006815c0`, the node itself as the element's address).
fn element_of(e: &mut Engine, node: u32) -> u32 {
    let at = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, node));
    e.mem.u32(at)
}

fn next_node(e: &mut Engine, node: u32) -> u32 {
    typed_call!(leaf::FAMILY, 0x0072_6070, leaf::word_at_4(e, node))
}

/// The compass's x for an angle within the strip: the angle from `-half` to `half` mapped onto 0 to
/// twice the width.
fn strip_x(e: &mut Engine, angle: f32) -> f32 {
    let menu = hud(e);
    let width = e.mem.i32(menu + HUD_WIDTH).wrapping_shl(1);
    let half = e.mem.i32(menu + HUD_HALF_ANGLE);
    interpolate(
        0.0,
        as_float(width),
        as_float(half.wrapping_neg()),
        as_float(half),
        angle,
    )
}

/// Whether `|angle|` is inside the strip's half angle (`FILD half; FCOMPP`: a NaN is not).
fn within_strip(e: &mut Engine, angle: f32) -> bool {
    let magnitude = abs(e, angle);
    let half = e.mem.i32(hud(e) + HUD_HALF_ANGLE);
    f64::from(magnitude) < exact(half)
}

fn angle_to(e: &mut Engine, from: [f32; 3], to: [f32; 3]) -> f32 {
    typed_call!(
        super::FAMILY,
        0x0077_8b80,
        compass_angle(e, from[0], from[1], from[2], to[0], to[1], to[2])
    )
}

fn blink(e: &mut Engine, tile: u32, period: f32, amplitude: f32) {
    typed_call!(
        super::FAMILY,
        0x0077_8c20,
        blink_tile(e, Ptr::new(tile), period, amplitude)
    );
}

// Typed from 0077a450
/// The compass strip's two floats (`thiscall(x, y)`, `RET 8`): stored at +0x7c and +0x80, each moved
/// through the x87 stack (a signalling NaN quieted).
pub fn set_strip(e: &mut Engine, this: Ptr, x: f32, y: f32) {
    e.mem.set_f32(this.addr() + 0x7c, x87_quiet(x));
    e.mem.set_f32(this.addr() + 0x80, x87_quiet(y));
}

// Typed from 0077a400
/// The player's own map marker (`thiscall`): the first entry of the list at +0x708 when it has any
/// (`00558dd0` with 0), else the word at +0x6f4.
pub fn player_marker(e: &mut Engine, this: Ptr) -> Ptr {
    let list = this.addr() + 0x708;
    if typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, list)) != 0 {
        let entry = e.call(LIST_ITEM, &args![list, 0u32]).u32();
        Ptr::new(e.mem.u32(entry))
    } else {
        Ptr::new(e.mem.u32(this.addr() + 0x6f4))
    }
}

// Typed from 00778b80
/// `HUDMainMenu::GetCompassAngle` (Xbox PDB; cdecl, two points by value, the answer in `ST0`): the
/// heading of `first - second` (`004b1550`, a float) less the player's heading (slot +0x2bc), brought
/// into the turn around zero (at or below the low bound a full turn added, above the high bound taken
/// from a full turn), then to degrees and the compass's scale, each step stored as a float.
pub fn compass_angle(e: &mut Engine, x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) -> f32 {
    let direction = e.with_stack(12, |e, difference| {
        let at = difference.addr();
        e.mem.set_f32(at + 8, z1 - z2);
        e.mem.set_f32(at + 4, y1 - y2);
        e.mem.set_f32(at, x1 - x2);
        narrow(e.call(HEADING_OF, &args![at]).f64())
    });
    let me = player(e);
    let heading = e.vcall(me, SLOT_HEADING, &args![0u32]).f64();
    let mut angle = narrow(f64::from(direction) - heading);
    if f64::from(angle) <= global_f64(e, ANGLE_LOW) {
        angle = narrow(f64::from(angle) + global_f64(e, ANGLE_TURN));
    } else if f64::from(angle) > global_f64(e, ANGLE_HIGH) {
        angle = narrow(global_f64(e, ANGLE_TURN) - f64::from(angle));
    }
    angle = narrow(f64::from(angle) * global_f64(e, DEGREES_PER_RADIAN));
    narrow(f64::from(angle) * global_f64(e, ANGLE_SCALE))
}

/// `BlinkTile`'s timing or delay at `period`: from the breakpoint's setting towards the shortest's
/// below the breakpoint, from the longest's towards the breakpoint's above it.
#[allow(clippy::too_many_arguments)]
fn blink_curve(
    e: &Engine,
    below: bool,
    at_longest: u32,
    at_breakpoint: u32,
    at_shortest: u32,
    longest: i32,
    shortest: i32,
    breakpoint: i32,
    period: f32,
) -> f32 {
    if below {
        let a = as_float(setting(e, at_breakpoint));
        let b = as_float(setting(e, at_shortest));
        interpolate(a, b, as_float(breakpoint), as_float(shortest), period)
    } else {
        let a = as_float(setting(e, at_longest));
        let b = as_float(setting(e, at_breakpoint));
        interpolate(a, b, as_float(longest), as_float(breakpoint), period)
    }
}

// Typed from 00778c20
/// `BlinkTile` (Xbox PDB; cdecl `(tile, period, amplitude)`): the tile's alpha rises (its
/// `_AlphaDown` 0) or falls (1) by the frame time times the timing at `period` (the period clamped to
/// the blink settings' squares) times `amplitude` over the HUD's alpha, or stays at the HUD's alpha
/// (3); past `amplitude` it is held there and turns to falling (3 is held), below zero it is set to
/// zero, rising again after the delay at `period`, and a held tile starts falling once that time has
/// passed.
pub fn blink_tile(e: &mut Engine, tile: Ptr, period: f32, amplitude: f32) {
    user_trait(e, BLINK_GUARD, 1, BLINK_ALPHA_DOWN, ALPHA_DOWN_NAME);
    let t = tile.addr();
    let longest = setting(e, BLINK_LONGEST).wrapping_mul(setting(e, BLINK_LONGEST));
    let shortest = setting(e, BLINK_SHORTEST).wrapping_mul(setting(e, BLINK_SHORTEST));
    let breakpoint = setting(e, BLINK_BREAKPOINT).wrapping_mul(setting(e, BLINK_BREAKPOINT));
    let mut period = period;
    if f64::from(period) > exact(longest) {
        period = as_float(longest);
    } else if f64::from(period) < exact(shortest) {
        period = as_float(shortest);
    }
    let below = f64::from(period) < exact(breakpoint);
    let alpha_down = trait_in(e, BLINK_ALPHA_DOWN);
    let state = get_float(e, t, alpha_down);
    let timing = blink_curve(
        e,
        below,
        BLINK_TIMING_LONGEST,
        BLINK_TIMING_BREAKPOINT,
        BLINK_TIMING_SHORTEST,
        longest,
        shortest,
        breakpoint,
        period,
    );
    let hud_alpha = e.mem.f32(HUD_ALPHA);
    let mut alpha = 0.0f32;
    let state_is = |e: &Engine, constant: u32| f64::from(state) == global_f64(e, constant);
    if state_is(e, ZERO) || state_is(e, ONE) {
        let rising = state_is(e, ZERO);
        if f64::from(hud_alpha) != global_f64(e, ZERO) {
            let current = get_float(e, t, ALPHA);
            let factor = timing * amplitude / hud_alpha;
            let delta = e.call(TIMER_DELTA, &args![TIMER]).f64();
            let step = narrow(delta) * factor;
            alpha = if rising {
                step + current
            } else {
                current - step
            };
        }
    } else if state_is(e, THREE) {
        alpha = hud_alpha;
    }
    set_float(e, t, ALPHA, alpha);
    let now_alpha = get_float(e, t, ALPHA);
    if amplitude < now_alpha {
        set_float(e, t, ALPHA, amplitude);
        set_int(e, t, alpha_down, 3);
    } else if f64::from(now_alpha) < global_f64(e, ZERO) {
        let delay = blink_curve(
            e,
            below,
            BLINK_DELAY_LONGEST,
            BLINK_DELAY_BREAKPOINT,
            BLINK_DELAY_SHORTEST,
            longest,
            shortest,
            breakpoint,
            period,
        );
        set_float(e, t, ALPHA, 0.0);
        set_int(e, t, alpha_down, 0);
        let now = e.call(TIMER_TICKS, &args![TIMER]).u32();
        set_float(e, t, BLINK_NEXT, narrow(f64::from(now) + f64::from(delay)));
    } else {
        let next = get_float(e, t, BLINK_NEXT);
        let now = e.call(TIMER_TICKS, &args![TIMER]).u32();
        if f64::from(now) > f64::from(next) && state_is(e, THREE) {
            set_int(e, t, alpha_down, 1);
        }
    }
}

/// The compass's top alpha (`0x7792F9`, `FMUL [0x101E568]`): the HUD opacity times 255. With it
/// the HUD's alpha kept to put back at [`tick_process`] (row on only).
///
/// lStewieAl's Tweaks' S0022.35 (`HUD_COMPASS_FOLLOWS_ITS_ALPHA`): with the row on, Tweaks' hook
/// keeps the HUD's alpha (`011d979c`), sets it to the compass tile's `alpha` (0xFA9, `00a011b0`)
/// and answers the opacity times that alpha; the kept value goes back at `0x779F1F`
/// ([`tick_process`]), as in the translation's `hud_main_menu_update_compass`.
fn compass_max_alpha(e: &mut Engine, compass: Ptr, opacity: f32) -> (f32, Option<f32>) {
    if e.stewie(engine_core::extend::stewie::HUD_COMPASS_FOLLOWS_ITS_ALPHA) {
        let kept = e.mem.f32(HUD_ALPHA);
        let compass_alpha = get_float(e, compass.addr(), 0xfa9);
        e.mem.set_f32(HUD_ALPHA, compass_alpha);
        (
            narrow(f64::from(opacity) * f64::from(compass_alpha)),
            Some(kept),
        )
    } else {
        (
            narrow(f64::from(opacity) * global_f64(e, TWO_FIFTY_FIVE)),
            None,
        )
    }
}

/// The player's process for the actors' ticks (`0x779F1F`, `008d6f30`).
///
/// Seam for S0022.35's second place (see [`compass_max_alpha`]): with the row on, Tweaks puts the
/// kept HUD alpha back and reads the player's word +0x40 raw in place of the call.
fn tick_process(e: &mut Engine, me: u32, kept_opacity: Option<f32>) -> u32 {
    match kept_opacity {
        Some(opacity) => {
            e.mem.set_f32(HUD_ALPHA, opacity);
            e.mem.u32(me + 0x40)
        }
        None => e.call(PROCESS_OF, &args![me]).u32(),
    }
}

/// The squared range `setting * setting` as the exe's `IMUL` makes it.
fn squared_setting(e: &Engine, at: u32) -> i32 {
    let value = setting(e, at);
    value.wrapping_mul(value)
}

// Typed from 00779070
/// `HUDMainMenu::UpdateCompass` (Xbox PDB; cdecl `(compass, landmark pool, target pool, tick pool)`):
/// the heading strip from the player's and its process's heading, then each map marker in range
/// (discovering the location when the player is inside its radius), each quest target, the player's
/// own marker and the actors' ticks, each on a tile of its pool; the pools' unused tiles are hidden.
pub fn update_compass(
    e: &mut Engine,
    compass: Ptr,
    landmarks: Ptr,
    objectives: Ptr,
    actor_ticks: Ptr,
) {
    user_trait(e, TRAITS_GUARD, 1, DISTANCE_TRAIT, DISTANCE_NAME);
    user_trait(e, TRAITS_GUARD, 2, HEADING_TRAIT, HEADING_NAME);
    user_trait(e, TRAITS_GUARD, 4, ALPHA_DOWN_TRAIT, ALPHA_DOWN_NAME);

    // The heading strip.
    let me = player(e);
    let player_heading = e.vcall(me, SLOT_HEADING, &args![0u32]).f64();
    let me = player(e);
    let process = e.call(PROCESS_OF, &args![me]).u32();
    let process_heading = e.call(PROCESS_HEADING, &args![process]).f64();
    let sum = narrow(process_heading + player_heading);
    let degrees = narrow(f64::from(sum) * global_f64(e, DEGREES_PER_RADIAN));
    let magnitude = abs(e, degrees);
    // The quotient goes to `_ftol2` in `ST0` as a double (truncated to the same integer at either
    // precision).
    let turns = f64::from(magnitude) / global_f64(e, FULL_TURN);
    let turns = typed_call!(crt::FAMILY, 0x00ec_62c0, crt::math::ftol2_sse(e, turns));
    let turns = i32::from_ne_bytes(crt::low_word(turns).to_ne_bytes());
    // `IMUL EAX,EAX,0x168`, then `FILD`.
    let offset = narrow(f64::from(degrees) - exact(turns.wrapping_mul(360)));
    let strip_degrees = e.mem.f32(STRIP_DEGREES);
    let normalized = interpolate(0.0, 1.0, 0.0, strip_degrees, offset);
    let strip_offset = narrow(f64::from(normalized) - global_f64(e, STRIP_SHIFT));
    if e.vcall(compass.addr(), SLOT_STRIP, &args![]).u32() != 0 {
        let adjusted = if f64::from(strip_offset) < global_f64(e, ZERO) {
            narrow(f64::from(strip_offset) + global_f64(e, ONE))
        } else {
            strip_offset
        };
        let strip = e.vcall(compass.addr(), SLOT_STRIP, &args![]).u32();
        typed_call!(
            super::FAMILY,
            0x0077_a450,
            set_strip(e, Ptr::new(strip), adjusted, 0.0)
        );
    }

    // The map markers.
    let me = player(e);
    let mut marker = e.call(PLAYER_MARKERS, &args![me]).u32();
    let me = player(e);
    let player_position = position_of(e, me);
    let width = e.mem.i32(hud(e) + HUD_WIDTH);
    let inner_half = narrow(exact(width) * global_f64(e, INNER_SHARE));
    let inner_width = inner_half + inner_half;
    let outer_width = narrow(exact(width) * 2.0);
    let opacity = if typed_call!(leaf::FAMILY, 0x0044_ddc0, leaf::word_at_8(e, OPACITY_FIXED)) == 0
    {
        float_setting(e, HUD_OPACITY)
    } else {
        1.0
    };
    let (max_alpha, kept_opacity) = compass_max_alpha(e, compass, opacity);
    let mut landmark_tiles = typed_call!(
        leaf::FAMILY,
        0x0055_9450,
        leaf::word_at_0(e, landmarks.addr() + 4)
    );
    let me = player(e);
    let sneaking = e.vcall(me, SLOT_SNEAKING, &args![0u32]).u8() != 0;
    let landmark_range = squared_setting(e, LANDMARK_RANGE);
    // The marker helper's byte +5: written on the not-sneaking path only, kept from the previous
    // marker otherwise (the exe's stack byte before its first write).
    let mut location_changed: u8 = 0;
    while marker != 0 {
        let at = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, marker));
        if e.mem.u32(at) == 0 {
            break;
        }
        let record = element_of(e, marker);
        marker = next_node(e, marker);
        let reference = e.mem.u32(record + 4);
        if typed_call!(leaf::FAMILY, 0x0044_0da0, leaf::form_flag_800(e, reference)) {
            continue;
        }
        let reference = e.mem.u32(record + 4);
        let marker_position = position_of(e, reference);
        let distance = distance_squared(e, marker_position, player_position);
        if f64::from(distance) > exact(landmark_range) {
            continue;
        }
        let travel = e.call(MARKER_TRAVEL, &args![e.mem.u32(record)]).u8() != 0;
        let radius = e
            .call(REFERENCE_RADIUS, &args![e.mem.u32(record + 4)])
            .f32();
        let radius_squared = radius * radius;
        let mut changed = false;
        if distance < radius_squared {
            let data = e.mem.u32(record);
            if e.call(MARKER_VISIBLE, &args![data]).u8() == 0 {
                changed = true;
                e.call(MARKER_SET_VISIBLE, &args![data, 1u32]);
                let reference = e.mem.u32(record + 4);
                e.vcall(reference, SLOT_MARK_CHANGED, &args![0x8000_0000u32]);
            }
            if !travel {
                changed = true;
                let data = e.mem.u32(record);
                e.call(MARKER_SET_TRAVEL, &args![data, 1u32]);
                let reference = e.mem.u32(record + 4);
                e.vcall(reference, SLOT_MARK_CHANGED, &args![0x8000_0000u32]);
            }
        }
        if !sneaking {
            if changed {
                let data = e.mem.u32(record);
                let jip_name = if e.jip_patches() & engine_core::extend::jip_patches::INSTALLED != 0
                {
                    e.site(JIP_DISCOVER_LOCATION, &[data, record])
                } else {
                    None
                };
                let location = match jip_name {
                    Some(name) => name,
                    None => e.call(MARKER_NAME, &args![data]).u32(),
                };
                let title = e.call(SETTING_STRING, &args![DISCOVERED_TITLE]).u32();
                e.call(
                    SHOW_MESSAGE,
                    &args![title, location, 1u32, 0u32, -1i32, -1i32, DISCOVERED_SOUND],
                );
                let reference = e.mem.u32(record + 4);
                e.call(
                    EVENT,
                    &args![4u32, 1u32, reference, reference, 0u32, 0u32, 0u32],
                );
                e.call(STAT_INCREMENT, &args![1u32]);
                if e.hook_users(engine_core::extend::Hook::LocationDiscover) != 0 {
                    let reference = e.mem.u32(record + 4);
                    e.site(JIP_LOCATION_DISCOVERED, &[reference]);
                }
                let reward = setting(e, DISCOVERY_XP);
                let me = player(e);
                e.vcall(me, SLOT_REWARD, &args![reward]);
                location_changed = 1;
            } else if travel {
                location_changed = 0;
            }
            let active = !changed && travel && distance < radius_squared;
            if !active {
                e.with_stack(8, |e, state| {
                    let reference = e.mem.u32(record + 4);
                    e.mem.set_u32(state.addr(), reference);
                    e.mem.set_u8(state.addr() + 4, 0);
                    e.mem.set_u8(state.addr() + 5, location_changed);
                    e.call(MARKER_HELPER, &args![state]);
                });
            }
        }
        if landmark_tiles == 0 {
            continue;
        }
        let tile = take_tile(e, landmarks.addr(), POOL_NEXT, &mut landmark_tiles);
        let texture = if travel {
            DISCOVERED_TEXTURE
        } else {
            LANDMARK_TEXTURE
        };
        set_texture(e, tile, texture);
        if tile == 0 {
            continue;
        }
        let angle = angle_to(e, marker_position, player_position);
        if !within_strip(e, angle) {
            set_int(e, tile, VISIBLE, 0);
            continue;
        }
        let x = strip_x(e, angle);
        set_float(e, tile, trait_in(e, HEADING_TRAIT), x);
        set_float(e, tile, trait_in(e, DISTANCE_TRAIT), distance);
        set_int(e, tile, VISIBLE, 1);
        if x > inner_width {
            let fade = interpolate(1.0, 0.0, inner_width, outer_width, x);
            set_float(e, tile, ALPHA, fade * max_alpha);
        } else if f64::from(distance) < global_f64(e, BLINK_RANGE) {
            let hud_alpha = e.mem.f32(HUD_ALPHA);
            blink(e, tile, distance, hud_alpha);
        } else {
            set_float(e, tile, ALPHA, max_alpha);
        }
    }
    while landmark_tiles != 0 {
        let tile = take_tile(e, landmarks.addr(), POOL_NEXT, &mut landmark_tiles);
        set_int(e, tile, VISIBLE, 0);
    }

    // The quest targets.
    let me = player(e);
    let targets = e.call(TARGET_LIST, &args![me]).u32();
    let menu = hud(e);
    e.mem.set_u32(menu + HUD_TARGETS, targets);
    let mut objective_tiles = typed_call!(
        leaf::FAMILY,
        0x0072_6070,
        leaf::word_at_4(e, objectives.addr() + 4)
    );
    let mut target_node = e.mem.u32(hud(e) + HUD_TARGETS);
    let range = as_float(setting(e, OBJECTIVE_RANGE));
    let range_squared = range * range;
    while target_node != 0 {
        let at = typed_call!(leaf::FAMILY, 0x0068_15c0, leaf::same_word(e, target_node));
        if e.mem.u32(at) == 0 {
            break;
        }
        let record = element_of(e, target_node);
        target_node = next_node(e, target_node);
        let target = e.call(TARGET_REFERENCE, &args![record]).u32();
        let actor_marker = target != 0 && e.call(FORM_TYPE, &args![target]).u32() == 0x1c;
        if target == 0 {
            continue;
        }
        let shown = e.call(SETTING_BYTE, &args![TARGETS_SHOWN]).u32();
        if e.mem.u8(shown) == 0 {
            continue;
        }
        let position = position_of(e, target);
        let distance = distance_squared(e, position, player_position);
        if f64::from(distance) != global_f64(e, ZERO) {
            // The faded alpha goes to a stack float the exe never reads; the calls still run.
            let clamped = e.call(CLAMP, &args![distance, range_squared]).f64();
            let ratio = narrow(clamped / f64::from(range_squared));
            let half = narrow(f64::from(max_alpha) / global_f64(e, TWO));
            let _ = interpolate(half, max_alpha, ratio, 0.0, distance);
        }
        if objective_tiles == 0 {
            continue;
        }
        let tile = take_tile(
            e,
            objectives.addr(),
            OBJECTIVE_POOL_NEXT,
            &mut objective_tiles,
        );
        set_texture(e, tile, OBJECTIVE_TEXTURE);
        if tile == 0 {
            continue;
        }
        let mut angle = angle_to(e, position, player_position);
        let half = e.mem.i32(hud(e) + HUD_HALF_ANGLE);
        if f64::from(angle) > exact(half) {
            angle = as_float(half);
        } else if f64::from(angle) < exact(half.wrapping_neg()) {
            angle = as_float(half.wrapping_neg());
        }
        let mut x = strip_x(e, angle);
        let cap = narrow(f64::from(inner_width) + global_f64(e, OBJECTIVE_OVERSHOOT));
        if x > cap {
            x = cap;
        }
        set_float(e, tile, trait_in(e, HEADING_TRAIT), x);
        set_float(e, tile, trait_in(e, DISTANCE_TRAIT), distance);
        set_int(e, tile, VISIBLE, 1);
        let menu = e.call(TILE_MENU, &args![compass]).u32();
        if e.call(MENU_SHOWN, &args![menu]).u32() == 1 {
            if actor_marker {
                set_float(e, tile, ALPHA, max_alpha);
            } else {
                let hud_alpha = e.mem.f32(HUD_ALPHA);
                blink(e, tile, distance, hud_alpha);
            }
        }
    }

    // The player's own marker.
    let me = player(e);
    let own = typed_call!(super::FAMILY, 0x0077_a400, player_marker(e, Ptr::new(me)));
    if !own.is_null() && objective_tiles != 0 {
        let tile = take_tile(
            e,
            objectives.addr(),
            OBJECTIVE_POOL_NEXT,
            &mut objective_tiles,
        );
        set_texture(e, tile, PLAYER_MARKER_TEXTURE);
        if tile != 0 {
            let me = player(e);
            let own = typed_call!(super::FAMILY, 0x0077_a400, player_marker(e, Ptr::new(me)));
            let mut position = position_of(e, own.addr());
            position[2] = player_position[2];
            let distance = distance_squared(e, position, player_position);
            let angle = angle_to(e, position, player_position);
            if !within_strip(e, angle) {
                set_int(e, tile, VISIBLE, 0);
            } else {
                let x = strip_x(e, angle);
                set_float(e, tile, trait_in(e, HEADING_TRAIT), x);
                set_int(e, tile, VISIBLE, 1);
                let amplitude = if x <= inner_width {
                    e.mem.f32(HUD_ALPHA)
                } else {
                    interpolate(max_alpha, 0.0, inner_width, outer_width, x)
                };
                blink(e, tile, distance, amplitude);
            }
        }
    }
    while objective_tiles != 0 {
        let tile = take_tile(
            e,
            objectives.addr(),
            OBJECTIVE_POOL_NEXT,
            &mut objective_tiles,
        );
        set_int(e, tile, VISIBLE, 0);
    }

    // The actors' ticks.
    e.mem.set_u8(HOSTILE_SHOWN, 0);
    let mut actor_tiles = typed_call!(
        leaf::FAMILY,
        0x0055_9450,
        leaf::word_at_0(e, actor_ticks.addr() + 4)
    );
    let me = player(e);
    let mut vats = e.call(VATS_TARGETS, &args![me]).u32();
    let me = player(e);
    let process = tick_process(e, me, kept_opacity);
    let actor_mode = e.call(ACTOR_MODE, &args![process]).u8() != 0;
    let first = float_setting(e, TICK_RANGE);
    let first_squared = first * first;
    let scale = float_setting(e, TICK_RANGE_SCALE);
    let second = narrow(exact(setting(e, TICK_RANGE)) * f64::from(scale));
    let second_squared = second * second;
    let mut count = e.call(LIST_COUNT, &args![vats]).u32();
    let all = e.call(LIST_COUNT, &args![vats]).u32();
    if all > setting(e, TICK_CAP) as u32 {
        e.call(LIST_SORT, &args![vats, COMPASS_ORDER]);
        count = setting(e, TICK_CAP) as u32;
    }
    while vats != 0
        && !typed_call!(leaf::FAMILY, 0x0082_56d0, leaf::both_words_zero(e, vats))
        && count != 0
    {
        count -= 1;
        let record = element_of(e, vats);
        if record == 0 || e.mem.u8(record + 5) != 0 {
            vats = next_node(e, vats);
            continue;
        }
        let actor = e.mem.u32(record);
        if e.vcall(actor, SLOT_3D, &args![]).u32() == 0 {
            vats = next_node(e, vats);
            continue;
        }
        if e.call(PROCESS_OF, &args![actor]).u32() == 0 {
            vats = next_node(e, vats);
            continue;
        }
        let process = e.call(PROCESS_OF, &args![actor]).u32();
        if e.call(PROCESS_ACTIVE, &args![process]).u8() == 0 {
            vats = next_node(e, vats);
            continue;
        }
        let first_value = e.vcall(actor + 0xa4, SLOT_VALUE, &args![0x31u32]).u32();
        let second_value = if first_value == 0 {
            e.vcall(actor + 0xa4, SLOT_VALUE, &args![0x30u32]).u32()
        } else {
            0
        };
        if first_value != 0 || second_value != 0 {
            let entry = e.with_stack(4, |e, output| {
                e.mem.set_f32(output.addr(), 0.0);
                let me = player(e);
                e.call(ENTRY_POINT, &args![0x45u32, me, output]);
                e.mem.f32(output.addr())
            });
            if entry <= 0.0 {
                vats = next_node(e, vats);
                continue;
            }
        }
        let actor = e.mem.u32(record);
        let position = position_of(e, actor);
        let distance = distance_squared(e, position, player_position);
        let limit = if actor_mode {
            first_squared
        } else {
            second_squared
        };
        let in_range = f64::from(distance) < exact(landmark_range);
        if distance > limit || !in_range {
            vats = next_node(e, vats);
            continue;
        }
        if actor_tiles == 0 {
            continue;
        }
        let tile = take_tile(e, actor_ticks.addr(), POOL_NEXT, &mut actor_tiles);
        set_texture(e, tile, TICK_TEXTURE);
        if tile != 0 {
            let angle = angle_to(e, position, player_position);
            if !within_strip(e, angle) {
                set_int(e, tile, VISIBLE, 0);
            } else {
                let x = strip_x(e, angle);
                let shifted = narrow(f64::from(x) + global_f64(e, TWO));
                set_float(e, tile, trait_in(e, HEADING_TRAIT), shifted);
                set_int(e, tile, VISIBLE, 1);
                if e.mem.u8(record + 4) != 0 {
                    set_int(e, tile, TICK_COLOUR, 2);
                    e.mem.set_u8(HOSTILE_SHOWN, 1);
                } else {
                    set_int(e, tile, TICK_COLOUR, 1);
                }
                let alpha = if x <= inner_width {
                    max_alpha
                } else {
                    interpolate(max_alpha, 0.0, inner_width, outer_width, x)
                };
                set_float(e, tile, ALPHA, alpha);
            }
        }
        vats = next_node(e, vats);
    }
    while actor_tiles != 0 {
        let tile = take_tile(e, actor_ticks.addr(), POOL_NEXT, &mut actor_tiles);
        set_int(e, tile, VISIBLE, 0);
    }
}
