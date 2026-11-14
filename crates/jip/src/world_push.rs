//! JIP LN NVSE's `PushActorNoRagdoll` (`functions_jip/jip_fn_actor.h`) with the actor helpers it
//! calls (`nvse/GameObjects.cpp`: `Actor::CanBePushed`, `Actor::AdjustPushForce`,
//! `Actor::PushActor`; `internal/utility.cpp`: `GetSinCos`, `Cos_V3`), the only user of the
//! ApplyActorVelocity hook (step 3.5b's handler: a pushed controller keeps its push while its
//! time lasts). JIP's SSE code is followed operation by operation in `f32` (each `mulss`,
//! `addss`, `subss` is one IEEE single operation); the one approximate instruction,
//! `rsqrtss`, is the processor's own (`_mm_rsqrt_ss`), as JIP runs it.

use engine_core::extend::Hook;
use engine_core::prelude::*;
use nvse::commands::Read;
use nvse::{handled, CommandArgs};

/// `GetClampedActorFloatValue` (`0066ef50`, `thiscall` on the actor's value owner `+0xA4`, the
/// value code): the actor value JIP's push force scales with (code 0xA).
const ACTOR_VALUE: u32 = 0x0066_ef50;
/// The game settings' values JIP's `AdjustPushForce` reads (`ds:0x11CEA6C`, `0x11CE664`,
/// `0x11CF9C0`, `0x11CFA20`).
const SETTING_SCALE: u32 = 0x011c_ea6c;
const SETTING_BASE: u32 = 0x011c_e664;
const SETTING_OFFSET: u32 = 0x011c_f9c0;
const SETTING_MULT: u32 = 0x011c_fa20;
/// `kPushActor` (two floats in `PushActor`), `FLT_EPSILON`, `FltPId180`, `FltPId2`, `FltPI`,
/// `FltPIx2` (`kPackedValues`, `internal/utility.cpp`), `Cos_V3`'s `kPS2dPI` and `kCosConsts`.
const PUSH_SCALE: u32 = 0x3f36_e147;
const PUSH_TIME: u32 = 0x3c2a_aaab;
const EPSILON: f32 = f32::EPSILON;
const PI_D180: u32 = 0x3c8e_fa35;
const PI_D2: u32 = 0x3fc9_0fdb;
const PI: u32 = 0x4049_0fdb;
const PI_X2: u32 = 0x40c9_0fdb;
const TWO_D_PI: u32 = 0x3f22_f983;
const COS_CONSTS: [u32; 5] = [
    0x37c2_3ab1,
    0x3ab5_9551,
    0x3d2a_a76f,
    0x3eff_ffe0,
    0x3f7f_ffff,
];

/// `cvttps2dq` of one lane: truncated, 0x80000000 for a NaN or a value out of range.
fn cvtt(x: f32) -> i32 {
    if x.is_nan() || x >= 2_147_483_648.0 || x < -2_147_483_648.0 {
        i32::MIN
    } else {
        x as i32
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: Cos_V3.
/// JIP's cosine of four lanes (the fourth is computed too): the angle's absolute value is
/// reduced to its quarter turn (by `2 / PI`, and by whole turns when any lane is past the third
/// quarter), moved into the first quarter and put through JIP's polynomial.
fn cos_v3(angles: [f32; 4]) -> [f32; 4] {
    let f = f32::from_bits;
    let mut x = angles.map(f32::abs);
    let mut q = x.map(|v| cvtt(v * f(TWO_D_PI)));
    if q.iter().any(|&v| v > 3) {
        for i in 0..4 {
            let turns = ((q[i] as u32) >> 2) as i32 as f32;
            q[i] &= 3;
            x[i] -= turns * f(PI_X2);
        }
    }
    let mut out = [0f32; 4];
    for i in 0..4 {
        let last = q[i] > 2;
        let full = if last { f(PI_X2).to_bits() } else { 0 };
        let rest = if last { 0 } else { q[i] };
        let half = rest > 0;
        let sign = if half { 0x8000_0000u32 } else { 0 };
        let shift = f32::from_bits((if half { PI } else { 0 }) | full);
        let v = x[i] - shift;
        let v2 = v * v;
        let mut r = f(COS_CONSTS[0]) * v2;
        r -= f(COS_CONSTS[1]);
        r *= v2;
        r += f(COS_CONSTS[2]);
        r *= v2;
        r -= f(COS_CONSTS[3]);
        let v2 = v2 * r;
        let c = v2 + f(COS_CONSTS[4]);
        out[i] = f32::from_bits(c.to_bits() ^ sign);
    }
    out
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), internal/utility.cpp: GetSinCos.
/// The sine and the cosine of `angle` (lanes 0 and 1: the cosines of `PI / 2 - angle` and of
/// `angle`), and the cosines of the zeros in lanes 2 and 3.
fn sin_cos(angle: f32) -> [f32; 4] {
    cos_v3([f32::from_bits(PI_D2) - angle, angle, 0.0, 0.0])
}

/// `rsqrtss`: the processor's approximate reciprocal square root, as JIP runs it.
fn rsqrt(x: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: SSE is part of the x86-64 baseline.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(x)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / x.sqrt()
    }
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::CanBePushed.
/// An actor with a high-level process (level byte `+0x28` is 0) that is not in the middle of a
/// knockdown (`+0x140` is 0), with a character controller (`+0x138`), not dead or dying (life
/// state `+0x108` not 1 or 2).
fn can_be_pushed(e: &Engine, actor: u32) -> bool {
    let process = e.mem.u32(actor + 0x68);
    if process == 0
        || e.mem.u8(process + 0x28) != 0
        || e.mem.u32(process + 0x140) != 0
        || e.mem.u32(process + 0x138) == 0
    {
        return false;
    }
    !matches!(e.mem.u32(actor + 0x108), 1 | 2)
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::AdjustPushForce.
/// The force scaled by the actor value 0xA (`(value * 10 * scale + base)`), after the setting
/// multiplier and the setting offset with the force's sign.
fn adjust_push_force(e: &mut Engine, actor: u32, force: f32) -> f32 {
    let value = e.call(ACTOR_VALUE, &args![actor + 0xa4, 0xau32]).f32();
    let mut scale = value * 10.0f32;
    scale *= e.mem.f32(SETTING_SCALE);
    scale += e.mem.f32(SETTING_BASE);
    let offset = f32::from_bits((force.to_bits() & 0x8000_0000) ^ e.mem.u32(SETTING_OFFSET));
    let mut adjusted = force * e.mem.f32(SETTING_MULT);
    adjusted += offset;
    adjusted * scale
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), nvse/GameObjects.cpp: Actor::PushActor.
/// A pushable actor's character controller is pushed: no force (its bits 0) clears the push
/// vector (`+0x500`, 16 bytes) and time (`+0x524`); else the direction (from the angle in
/// degrees, or away from the other reference, nothing when it stands within `FLT_EPSILON`) times
/// `force * k0 * |force| * k1`, the time `|force| * k1`, and JIP's push bit (`+0x417` bit 7).
fn push_actor(e: &mut Engine, actor: u32, force: f32, angle: f32, origin: u32) {
    if !can_be_pushed(e, actor) {
        return;
    }
    let controller = e.mem.u32(e.mem.u32(actor + 0x68) + 0x138);
    if force.to_bits() == 0 {
        for i in 0..4 {
            e.mem.set_u32(controller + 0x500 + 4 * i, 0);
        }
        e.mem.set_u32(controller + 0x524, 0);
        return;
    }
    let direction = if origin == 0 {
        sin_cos(angle * f32::from_bits(PI_D180))
    } else {
        let dx = e.mem.f32(actor + 0x30) - e.mem.f32(origin + 0x30);
        let dy = e.mem.f32(actor + 0x34) - e.mem.f32(origin + 0x34);
        let d = dx * dx + dy * dy;
        // `comiss` then `jb`: stops below epsilon or unordered.
        if d.is_nan() || d < EPSILON {
            return;
        }
        let r = rsqrt(d);
        [dx * r, dy * r, 0.0, 0.0]
    };
    let mut speed = force * f32::from_bits(PUSH_SCALE);
    let time = f32::from_bits(force.to_bits() & 0x7fff_ffff) * f32::from_bits(PUSH_TIME);
    speed *= time;
    let scaled = [
        direction[0] * speed,
        direction[1] * speed,
        direction[2] * 0.0,
        direction[3] * 0.0,
    ];
    for (i, v) in scaled.iter().enumerate() {
        e.mem.set_f32(controller + 0x500 + 4 * i as u32, *v);
    }
    e.mem.set_f32(controller + 0x524, time);
    let flags = e.mem.u8(controller + 0x417);
    e.mem.set_u8(controller + 0x417, flags | 0x80);
}

// Translated from JIP LN NVSE 57.30 (5a30ac4), functions_jip/jip_fn_actor.h:
// Cmd_PushActorNoRagdoll_Execute.
/// The calling actor pushed by `force` at `angle` (or away from the reference given, not
/// itself), the force adjusted unless the fourth argument is 0; the ApplyActorVelocity hook is
/// switched on.
pub(crate) fn push_actor_no_ragdoll(e: &mut Engine, w: &[u32]) -> Ret {
    let a = CommandArgs::new(w);
    let actor = a.this_obj;
    if !crate::commands::is_actor(e, actor) {
        return handled(true);
    }
    let read = Read::of(e, &a);
    if !read.ok {
        return handled(true);
    }
    let mut force = read.float(0, 0.0);
    let angle = read.float(1, 0.0);
    let mut origin = read.form(2);
    let adjust = read.int(3, 1);
    e.set_hook(Hook::ApplyActorVelocity, true);
    if origin == actor {
        origin = 0;
    }
    if adjust != 0 {
        force = adjust_push_force(e, actor, force);
    }
    push_actor(e, actor, force, angle, origin);
    handled(true)
}

#[cfg(test)]
#[path = "world_push_tests.rs"]
mod tests;
