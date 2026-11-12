//! The modifiers' `Update`s (`thiscall`, `RET 8`: the time and the particle data), each running over
//! the active particles, and the data's particle removal the age modifier calls.

// NaN-aware as the exe: a negated comparison takes its branch when the operands are unordered too,
// as the exe's flag tests after `FCOM` do.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use super::{
    BSPSysSimpleColorModifier, NiPSysAgeDeathModifier, NiPSysBombModifier,
    NiPSysBoundUpdateModifier, NiPSysData, NiPSysGrowFadeModifier, NiPSysModifier, FAMILY,
};
use crate::native::{float_precision, Real};
use crate::{culling, dispatch, slot_target};
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// A modifier's `Update` (vtable +0x8c).
const SLOT_UPDATE: u32 = 0x8c;
/// The particle data's slots: `RemoveParticle` (+0xa4) and `ResolveAddedParticles` (+0xac); a
/// spawn modifier's `SpawnParticles` (+0xa4); an object's `GetRTTI` (+0x08).
const SLOT_REMOVE_PARTICLE: u32 = 0xa4;
const SLOT_RESOLVE_ADDED: u32 = 0xac;
const SLOT_SPAWN_PARTICLES: u32 = 0xa4;
const SLOT_GET_RTTI: u32 = 0x08;
/// A particle's info: 0x1c bytes, the velocity at +0, the age +0x0c, the life span +0x10, the last
/// update +0x14, the generation +0x18 (a word).
const INFO_SIZE: u32 = 0x1c;
const INFO_AGE: u32 = 0x0c;
const INFO_LIFE_SPAN: u32 = 0x10;
const INFO_LAST_UPDATE: u32 = 0x14;
const INFO_GENERATION: u32 = 0x18;
/// `NiPoint3::ZERO` (three floats at `011f426c`).
const POINT_ZERO: u32 = 0x011f_426c;
/// An object's world transform (+0x68, 0x34 bytes).
const OBJECT_TRANSFORM: u32 = 0x68;
const TRANSFORM_SIZE: u32 = 0x34;
/// `exp` (`00ecae6c`, the double on the stack, `ST0` out).
const EXP: u32 = 0x00ec_ae6c;
/// `NiParticlesData::RemoveParticle` (`00a964c0`, `thiscall`, one word).
const PARTICLES_DATA_REMOVE_PARTICLE: u32 = 0x00a9_64c0;
/// The rotation modifier's 2π (`011aea98`) and its function-local static limit (`01202520`, guard
/// word `01202524`, set from the float at `010bdb90` the first time).
const TWO_PI: u32 = 0x011a_ea98;
const STATIC_LIMIT: u32 = 0x0120_2520;
const STATIC_GUARD: u32 = 0x0120_2524;
const STATIC_LIMIT_INIT: u32 = 0x010b_db90;
/// The grow and fade modifier's smallest size (`010c09c0`).
const MIN_SIZE: u32 = 0x010c_09c0;
/// The bound update's mesh and strip particle data (their `NiRTTI` records and updates), 0.5 (the
/// double at `01011588`) and the memory manager the skip bounds come from (`011f6238`, its
/// `Allocate` `00aa3e40` and `Deallocate` `00aa4060`, both `thiscall` with one word).
const MESH_PSYS_DATA_RTTI: u32 = 0x0120_27c0;
const STRIP_PSYS_DATA_RTTI: u32 = 0x0120_2680;
const MESH_BOUND_UPDATE: u32 = 0x00c2_dce0;
const STRIP_BOUND_UPDATE: u32 = 0x00c2_d880;
const HALF: u32 = 0x0101_1588;
const MEMORY_MANAGER: u32 = 0x011f_6238;
const ALLOCATE: u32 = 0x00aa_3e40;
const DEALLOCATE: u32 = 0x00aa_4060;
/// The simple colour modifier's 0.0 (the double at `01012060`).
const ZERO_DOUBLE: u32 = 0x0101_2060;

/// A modifier's `Update` (vtable +0x8c) with the time's bits and the data: the typed one when the
/// slot holds a modifier this family has, any other through the vtable.
pub(crate) fn update(e: &mut Engine, modifier: u32, time: u32, data: u32) {
    let t = f32::from_bits(time);
    match slot_target(e, modifier, SLOT_UPDATE) {
        0x00c2_e670 => typed_call!(
            FAMILY,
            0x00c2_e670,
            bomb(e, Ptr::new(modifier), t, Ptr::new(data))
        ),
        0x00c2_8bc0 => typed_call!(
            FAMILY,
            0x00c2_8bc0,
            position(e, Ptr::new(modifier), t, Ptr::new(data))
        ),
        0x00c2_f010 => typed_call!(
            FAMILY,
            0x00c2_f010,
            age_death(e, Ptr::new(modifier), t, Ptr::new(data))
        ),
        0x00c2_06b0 => typed_call!(
            FAMILY,
            0x00c2_06b0,
            rotation(e, Ptr::new(modifier), t, Ptr::new(data))
        ),
        0x00c2_a390 => typed_call!(
            FAMILY,
            0x00c2_a390,
            grow_fade(e, Ptr::new(modifier), t, Ptr::new(data))
        ),
        0x00c2_df20 => typed_call!(
            FAMILY,
            0x00c2_df20,
            bound_update(e, Ptr::new(modifier), t, Ptr::new(data))
        ),
        0x00c6_02e0 => typed_call!(
            FAMILY,
            0x00c6_02e0,
            simple_color(e, Ptr::new(modifier), time, Ptr::new(data))
        ),
        _ => {
            e.vcall(modifier, SLOT_UPDATE, &args![time, data]);
        }
    }
}

/// The three words at `at`.
fn read3(e: &Engine, at: u32) -> [u32; 3] {
    [0, 4, 8].map(|k| e.mem.u32(at.wrapping_add(k)))
}

/// Three words to `at`.
fn write3(e: &mut Engine, at: u32, words: [u32; 3]) {
    for (k, word) in (0u32..).zip(words) {
        e.mem.set_u32(at.wrapping_add(4 * k), word);
    }
}

// Typed from 00c2e670
/// `NiPSysBombModifier::Update`: the bomb's position starts as `NiPoint3::ZERO` and its axis (a
/// local at +0x20 of the exe's 0x124-byte frame) as `m_kBombAxis`; with particles and a bomb object
/// the position is the object's in the particle system's space (the inverse of the system's world
/// transform, `004b4880`, times the object's, `0062c250`, at +0x88) and the axis that transform's
/// rotation of `m_kBombAxis` (`004b4500`), normalised (`004a0c10`). Then each particle
/// ([`bomb_particle`]).
pub fn bomb(e: &mut Engine, this: Ptr<NiPSysBombModifier>, time: f32, data: Ptr<NiPSysData>) {
    e.with_stack(0x124, |e, frame| {
        let f = frame.addr();
        let mut bomb_at = read3(e, POINT_ZERO);
        let axis_field = this.addr() + NiPSysBombModifier::m_kBombAxis.off;
        let axis = read3(e, axis_field);
        write3(e, f + 0x20, axis);
        let object = e.get(this, NiPSysBombModifier::m_pkBombObj);
        if e.get(data, NiPSysData::m_usActive) != 0 && object != 0 {
            let bytes = e
                .mem
                .bytes(object.wrapping_add(OBJECT_TRANSFORM), TRANSFORM_SIZE);
            e.mem.write(f + 0xbc, &bytes);
            let system = e.mem.u32(this.addr() + NiPSysModifier::m_pkTarget.off);
            let bytes = e
                .mem
                .bytes(system.wrapping_add(OBJECT_TRANSFORM), TRANSFORM_SIZE);
            e.mem.write(f + 0x88, &bytes);
            typed_call!(
                dispatch::FAMILY,
                0x004b_4880,
                dispatch::math::invert_non_uniform(e, Ptr::new(f + 0x88), Ptr::new(f + 0xf0))
            );
            typed_call!(
                dispatch::FAMILY,
                0x0062_c250,
                dispatch::math::compose(
                    e,
                    Ptr::new(f + 0xf0),
                    Ptr::new(f + 0x88),
                    Ptr::new(f + 0xbc)
                )
            );
            bomb_at = read3(e, f + 0xac);
            let turned = typed_call!(
                dispatch::FAMILY,
                0x004b_4500,
                dispatch::math::rotate(
                    e,
                    Ptr::new(f + 0x88),
                    Ptr::new(f + 0x58),
                    Ptr::new(axis_field)
                )
            )
            .addr();
            let turned = read3(e, turned);
            write3(e, f + 0x20, turned);
            typed_call!(
                dispatch::FAMILY,
                0x004a_0c10,
                dispatch::math::normalize(e, Ptr::new(f + 0x20))
            );
        }
        let mut i: u16 = 0;
        if i >= e.get(data, NiPSysData::m_usActive) {
            return;
        }
        let direction = f + 0x14;
        let single = float_precision();
        loop {
            if single {
                bomb_particle::<f32>(e, this, data, time, bomb_at, f, direction, u32::from(i));
            } else {
                bomb_particle::<f64>(e, this, data, time, bomb_at, f, direction, u32::from(i));
            }
            i = i.wrapping_add(1);
            if i >= e.get(data, NiPSysData::m_usActive) {
                break;
            }
        }
    });
}

/// One particle of [`bomb`]: when its last update is before the time (unordered skips) and, with a
/// decay type, the decay is not below its distance to the bomb, it gets the bomb's delta-v times the
/// falloff times the time since its last update, along a direction (the local at +0x14): symmetry 0
/// from the bomb, normalised; symmetry 1 away from the axis line, normalised unless its length is
/// zero (that length is then the distance); symmetry 2 the axis, or its opposite when the offset's
/// dot product with it is negative (the dot product is the distance); another the words already
/// there. The falloff is 1, the decay less the distance over the decay (type 1) or
/// `exp(-distance / decay)` (type 2).
///
/// Native float maths in `R`, the running thread's precision ([`Real`]): each step and each float
/// store the exe's, the square roots `FSQRT` at that precision in place of the CRT's `_CIsqrt`.
#[allow(clippy::too_many_arguments)]
fn bomb_particle<R: Real>(
    e: &mut Engine,
    this: Ptr<NiPSysBombModifier>,
    data: Ptr<NiPSysData>,
    time: f32,
    bomb_at: [u32; 3],
    frame: u32,
    direction: u32,
    i: u32,
) {
    let now = R::load(time.to_bits());
    let info = e.get(data, NiPSysData::m_pkParticleInfo) + i * INFO_SIZE;
    let last = R::load(e.mem.u32(info + INFO_LAST_UPDATE));
    // `FLD time; FLD last; FCOM; TEST AH,5; JP`: on only when the last update is before the time.
    if !(last < now) {
        return;
    }
    let elapsed = (now - last).store();
    let position = e.get(data, NiPSysData::m_pkVertex) + i * 0xc;
    let mut offset = [0u32; 3];
    for (k, (slot, at)) in (0u32..).zip(offset.iter_mut().zip(bomb_at)) {
        *slot = (R::load(e.mem.u32(position + 4 * k)) - R::load(at)).store();
    }
    let [dx, dy, dz] = offset;
    let mut distance = R::load(native_length::<R>(dx, dy, dz));
    let decay_type = e.get(this, NiPSysBombModifier::m_eDecayType);
    if decay_type != 0 {
        let decay = R::load(e.get(this, NiPSysBombModifier::m_fDecay));
        // `FLD decay; FCOMP; TEST AH,1; JNZ`: skipped when the decay is below the distance (or they
        // are unordered).
        if !(decay >= distance) {
            return;
        }
    }
    let axis = read3(e, frame + 0x20);
    let a = axis.map(R::load);
    match e.get(this, NiPSysBombModifier::m_eSymmType) {
        0 => {
            let inverse = R::load((R::ONE / distance).store());
            let d = [
                (inverse * R::load(dx)).store(),
                (R::load(dy) * inverse).store(),
                (inverse * R::load(dz)).store(),
            ];
            write3(e, direction, d);
        }
        1 => {
            let (lx, ly, lz) = (R::load(dx), R::load(dy), R::load(dz));
            let first = a[1] * ly + lx * a[0];
            let along = R::load((a[2] * lz + first).store());
            let px = (a[0] * along).store();
            let py = (a[1] * along).store();
            let pz = (along * a[2]).store();
            let rx = (lx - R::load(px)).store();
            let ry = (ly - R::load(py)).store();
            let rz = (lz - R::load(pz)).store();
            write3(e, direction, [rx, ry, rz]);
            distance = R::load(native_length::<R>(rx, ry, rz));
            // `FLDZ; FLD len; FUCOM; TEST AH,0x44; JNP`: not normalised when the length is zero.
            if distance != R::ZERO {
                let inverse = R::load((R::ONE / distance).store());
                let y = e.mem.u32(direction + 4);
                let z = e.mem.u32(direction + 8);
                let d = [
                    (inverse * R::load(rx)).store(),
                    (R::load(y) * inverse).store(),
                    (inverse * R::load(z)).store(),
                ];
                write3(e, direction, d);
            }
        }
        2 => {
            let first = R::load(dx) * a[0] + a[1] * R::load(dy);
            let along = R::load((first + a[2] * R::load(dz)).store());
            if along < R::ZERO {
                let flipped = (-along).store();
                let d = [(-a[0]).store(), (-a[1]).store(), (-a[2]).store()];
                write3(e, direction, d);
                distance = R::load(flipped);
            } else {
                write3(e, direction, axis);
                distance = along;
            }
        }
        _ => {}
    }
    let decay = R::load(e.get(this, NiPSysBombModifier::m_fDecay));
    let falloff = match decay_type {
        1 => ((decay - distance) / decay).store(),
        2 => {
            let exponent = R::load((-distance / decay).store());
            let result = e.call(EXP, &args![exponent.wide()]).f64();
            R::narrow(result).store()
        }
        _ => 1.0f32.to_bits(),
    };
    let delta_v = R::load(e.get(this, NiPSysBombModifier::m_fDeltaV));
    let k = R::load((delta_v * R::load(falloff) * R::load(elapsed)).store());
    let d = read3(e, direction);
    let push = [
        (k * R::load(d[0])).store(),
        (R::load(d[1]) * k).store(),
        (k * R::load(d[2])).store(),
    ];
    for (j, value) in (0u32..).zip(push) {
        let slot = info + 4 * j;
        let sum = R::load(e.mem.u32(slot)) + R::load(value);
        e.mem.set_u32(slot, sum.store());
    }
}

/// A vector's length in native maths (`R`, [`Real`]): `(y² + x²) + z²` stored, its square root
/// (`FSQRT` at the running precision, in place of the CRT's `_CIsqrt`) stored.
fn native_length<R: Real>(x: u32, y: u32, z: u32) -> u32 {
    let (x, y, z) = (R::load(x), R::load(y), R::load(z));
    let squared = (y * y + x * x + z * z).store();
    R::load(squared).sqrt().store()
}

// Typed from 00c28bc0
/// `NiPSysPositionModifier::Update` (`this` not read): resolves the added particles (data slot
/// +0xac), then each active particle moves by its velocity times the time since its last update,
/// which becomes the time. The elapsed time and the three offsets pass through float stack slots.
///
/// Native float maths in plain `f32` on every thread: each step is one operation on two floats,
/// stored as a float, which rounds to the same float at either precision (ADR-0014); the time is
/// copied plain.
pub fn position(e: &mut Engine, _this: Ptr, time: f32, data: Ptr<NiPSysData>) {
    e.vcall(data.addr(), SLOT_RESOLVE_ADDED, &[]);
    let infos = e.get(data, NiPSysData::m_pkParticleInfo);
    let mut i: u16 = 0;
    if i >= e.get(data, NiPSysData::m_usActive) {
        return;
    }
    let float = |e: &Engine, at: u32| f32::from_bits(e.mem.u32(at));
    loop {
        let k = u32::from(i);
        let vertices = e.get(data, NiPSysData::m_pkVertex);
        let info = infos + k * INFO_SIZE;
        let at = vertices + k * 0xc;
        let elapsed = time - float(e, info + INFO_LAST_UPDATE);
        let dx = float(e, info) * elapsed;
        let dy = float(e, info + 4) * elapsed;
        let dz = elapsed * float(e, info + 8);
        let x = float(e, at) + dx;
        e.mem.set_u32(at, x.to_bits());
        let y = dy + float(e, at + 4);
        e.mem.set_u32(at + 4, y.to_bits());
        let z = float(e, at + 8) + dz;
        e.mem.set_u32(at + 8, z.to_bits());
        e.mem.set_u32(info + INFO_LAST_UPDATE, time.to_bits());
        i = i.wrapping_add(1);
        if i >= e.get(data, NiPSysData::m_usActive) {
            break;
        }
    }
}

// Typed from 00c2f010
/// `NiPSysAgeDeathModifier::Update`: every active particle, from the last to the first, ages by the
/// time since its last update (`(time - last) + age`, stored); one whose life span is below the new
/// age is removed (the data's slot +0xa4: [`remove_particle`] directly when the slot holds it), after
/// the spawn modifier's `SpawnParticles` (its slot +0xa4: the time, `life span + time - age`, the
/// index word, the target) when spawning on death with a spawn modifier. The index word is the exe's
/// counter register: it starts at `count - 1` and gains 0xffff each time round.
///
/// Native float maths in the running thread's precision ([`Real`]).
pub fn age_death(
    e: &mut Engine,
    this: Ptr<NiPSysAgeDeathModifier>,
    time: f32,
    data: Ptr<NiPSysData>,
) {
    if float_precision() {
        age_death_in::<f32>(e, this, time, data);
    } else {
        age_death_in::<f64>(e, this, time, data);
    }
}

/// [`age_death`] in `R`.
fn age_death_in<R: Real>(
    e: &mut Engine,
    this: Ptr<NiPSysAgeDeathModifier>,
    time: f32,
    data: Ptr<NiPSysData>,
) {
    let count = e.get(data, NiPSysData::m_usActive);
    if count == 0 {
        return;
    }
    let now = R::load(time.to_bits());
    let spawns = e.get(this, NiPSysAgeDeathModifier::m_bSpawnOnDeath) != 0
        && e.get(this, NiPSysAgeDeathModifier::m_pkSpawnModifier) != 0;
    let mut counter = u32::from(count);
    let mut index = u32::from(count.wrapping_sub(1));
    loop {
        let info = e.get(data, NiPSysData::m_pkParticleInfo) + (index & 0xffff) * INFO_SIZE;
        let stored = ((now - R::load(e.mem.u32(info + INFO_LAST_UPDATE)))
            + R::load(e.mem.u32(info + INFO_AGE)))
        .store();
        let age = R::load(stored);
        e.mem.set_u32(info + INFO_AGE, stored);
        let life_span = R::load(e.mem.u32(info + INFO_LIFE_SPAN));
        if life_span < age {
            if spawns {
                let left = ((life_span + now) - age).store();
                let target = e.mem.u32(this.addr() + NiPSysModifier::m_pkTarget.off);
                let spawner = e.get(this, NiPSysAgeDeathModifier::m_pkSpawnModifier);
                e.vcall(
                    spawner,
                    SLOT_SPAWN_PARTICLES,
                    &args![now.store(), left, index, target],
                );
            }
            if slot_target(e, data.addr(), SLOT_REMOVE_PARTICLE) == 0x00c2_4cc0 {
                typed_call!(FAMILY, 0x00c2_4cc0, remove_particle(e, data, index));
            } else {
                e.vcall(data.addr(), SLOT_REMOVE_PARTICLE, &args![index]);
            }
        }
        counter = counter.wrapping_add(0xffff);
        index = index.wrapping_add(0xffff);
        if low_half(counter) == 0 {
            break;
        }
    }
}

// Typed from 00c24cc0
/// `NiPSysData::RemoveParticle` (`thiscall`, one word): `NiParticlesData::RemoveParticle`
/// (`00a964c0`) with the whole word; then, unless the index (its low half) was the last active
/// particle (taken before that call), the last particle's info (copied word by word) and rotation
/// speed (when there are rotation speeds) move into it.
pub fn remove_particle(e: &mut Engine, this: Ptr<NiPSysData>, index: u32) {
    let last = e.get(this, NiPSysData::m_usActive).wrapping_sub(1);
    e.call(PARTICLES_DATA_REMOVE_PARTICLE, &args![this, index]);
    let index = low_half(index);
    if index == last {
        return;
    }
    let info = e.get(this, NiPSysData::m_pkParticleInfo);
    let from = info.wrapping_add(u32::from(last) * INFO_SIZE);
    let to = info.wrapping_add(u32::from(index) * INFO_SIZE);
    for word in 0..7 {
        let value = e.mem.u32(from + 4 * word);
        e.mem.set_u32(to + 4 * word, value);
    }
    let speeds = e.get(this, NiPSysData::m_pfRotationSpeeds);
    if speeds != 0 {
        // The exe moves it through the x87 (`FLD`/`FSTP`); a plain copy differs only in a signalling
        // NaN's payload (ADR-0014: copies plain).
        let speed = e.mem.u32(speeds + 4 * u32::from(last));
        e.mem.set_u32(speeds + 4 * u32::from(index), speed);
    }
}

/// The low half of a word (the `short` an argument word carries).
#[allow(clippy::cast_possible_truncation)]
fn low_half(word: u32) -> u16 {
    word as u16
}

// Typed from 00c206b0
/// `NiPSysRotationModifier::Update` (`this` not read): with rotation angles, each active particle's
/// angle advances by `(time - last update) * rotation speed`; past the function-local static limit
/// (set from the float at `010bdb90` the first time, its guard word written back after the loop) it
/// becomes 0, else while it is above 2π it loses 2π (the first test before any subtraction).
pub fn rotation(e: &mut Engine, this: Ptr, time: f32, data: Ptr<NiPSysData>) {
    if float_precision() {
        rotation_in::<f32>(e, this, time, data);
    } else {
        rotation_in::<f64>(e, this, time, data);
    }
}

/// [`rotation`] in `R`.
fn rotation_in<R: Real>(e: &mut Engine, _this: Ptr, time: f32, data: Ptr<NiPSysData>) {
    let angles = e.get(data, NiPSysData::m_pfRotationAngles);
    if angles == 0 {
        return;
    }
    let speeds = e.get(data, NiPSysData::m_pfRotationSpeeds);
    if e.get(data, NiPSysData::m_usActive) == 0 {
        return;
    }
    let now = R::load(time.to_bits());
    let two_pi = R::load(e.global::<u32>(TWO_PI));
    let mut guard = e.global::<u32>(STATIC_GUARD);
    let limit_init = e.global::<u32>(STATIC_LIMIT_INIT);
    let mut i: u16 = 0;
    loop {
        let k = u32::from(i);
        let info = e.get(data, NiPSysData::m_pkParticleInfo);
        let last_update = e.mem.u32(info + k * INFO_SIZE + INFO_LAST_UPDATE);
        let elapsed = R::load((now - R::load(last_update)).store());
        let slot = angles + 4 * k;
        let advanced =
            (elapsed * R::load(e.mem.u32(speeds + 4 * k)) + R::load(e.mem.u32(slot))).store();
        let angle = R::load(advanced);
        e.mem.set_u32(slot, advanced);
        if guard & 1 == 0 {
            guard |= 1;
            // The exe moves the float through the x87; copied plain (ADR-0014).
            e.set_global(STATIC_LIMIT, limit_init);
        }
        let limit = R::load(e.global::<u32>(STATIC_LIMIT));
        if limit < angle {
            e.mem.set_u32(slot, 0.0f32.to_bits());
        } else if two_pi < angle {
            loop {
                let value = R::load(e.mem.u32(slot));
                e.mem.set_u32(slot, (value - two_pi).store());
                let value = R::load(e.mem.u32(slot));
                if !(value > two_pi) {
                    break;
                }
            }
        }
        i = i.wrapping_add(1);
        if i >= e.get(data, NiPSysData::m_usActive) {
            break;
        }
    }
    e.set_global(STATIC_GUARD, guard);
}

// Typed from 00c2a390
/// `NiPSysGrowFadeModifier::Update` (the time not read): for each active particle the grow factor is
/// `age / grow time` for the grow generation while the grow time is above the age and not zero, else
/// 1; the fade factor likewise `(life span - age) / fade time` for the fade generation; the smaller
/// (the fade factor when they compare unordered) times `1 - base scale`, plus the base scale, at least
/// the float at `010c09c0`, becomes the particle's size.
///
/// Native float maths in the running thread's precision ([`Real`]).
pub fn grow_fade(
    e: &mut Engine,
    this: Ptr<NiPSysGrowFadeModifier>,
    time: f32,
    data: Ptr<NiPSysData>,
) {
    if float_precision() {
        grow_fade_in::<f32>(e, this, time, data);
    } else {
        grow_fade_in::<f64>(e, this, time, data);
    }
}

/// [`grow_fade`] in `R`.
fn grow_fade_in<R: Real>(
    e: &mut Engine,
    this: Ptr<NiPSysGrowFadeModifier>,
    _time: f32,
    data: Ptr<NiPSysData>,
) {
    let base = e.get(this, NiPSysGrowFadeModifier::m_fBaseScale);
    let one_minus_base = R::load((R::ONE - R::load(base)).store());
    let mut i: u16 = 0;
    if i >= e.get(data, NiPSysData::m_usActive) {
        return;
    }
    let one = 1.0f32.to_bits();
    loop {
        let k = u32::from(i);
        let info = e.get(data, NiPSysData::m_pkParticleInfo) + k * INFO_SIZE;
        let generation = e.mem.u16(info + INFO_GENERATION);
        let age = e.mem.u32(info + INFO_AGE);
        let mut grow = one;
        let grow_time = e.get(this, NiPSysGrowFadeModifier::m_fGrowTime);
        if generation == e.get(this, NiPSysGrowFadeModifier::m_usGrowGeneration)
            && R::load(grow_time) > R::load(age)
            && R::load(grow_time) != R::ZERO
        {
            grow = (R::load(age) / R::load(grow_time)).store();
        }
        let mut fade = one;
        let remaining = (R::load(e.mem.u32(info + INFO_LIFE_SPAN)) - R::load(age)).store();
        let fade_time = e.get(this, NiPSysGrowFadeModifier::m_fFadeTime);
        if generation == e.get(this, NiPSysGrowFadeModifier::m_usFadeGeneration)
            && R::load(fade_time) > R::load(remaining)
            && R::load(fade_time) != R::ZERO
        {
            fade = (R::load(remaining) / R::load(fade_time)).store();
        }
        // `FLD fade; FLD grow; FCOM; TEST AH,5; JP`: the grow factor when it is below the fade
        // factor.
        let smaller = if R::load(grow) < R::load(fade) {
            grow
        } else {
            fade
        };
        let base = R::load(e.get(this, NiPSysGrowFadeModifier::m_fBaseScale));
        let mut size = (R::load(smaller) * one_minus_base + base).store();
        let min_size = e.global::<u32>(MIN_SIZE);
        // `FLD size; FLD min; FCOM; TEST AH,0x41; JNZ`: the minimum replaces the size only when it
        // is above it.
        if R::load(min_size) > R::load(size) {
            size = min_size;
        }
        let sizes = e.get(data, NiPSysData::m_pfSizes);
        i = i.wrapping_add(1);
        e.mem.set_u32(sizes + 4 * k, size);
        if i >= e.get(data, NiPSysData::m_usActive) {
            break;
        }
    }
}

/// Bound `i` of the skip bounds (re-read from `this`): centre `NiPoint3::ZERO`, radius 0.0.
fn clear_bound(e: &mut Engine, this: Ptr<NiPSysBoundUpdateModifier>, i: u32) {
    let bound = e.get(this, NiPSysBoundUpdateModifier::m_pkSkipBounds) + 0x10 * i;
    let zero = read3(e, POINT_ZERO);
    write3(e, bound, zero);
    e.mem.set_u32(bound + 0xc, 0.0f32.to_bits());
}

// Typed from 00c2d720
/// The bound update's skip (`thiscall`, one word, its low half the `short`): stored; the old skip
/// bounds given back to the memory manager and `max(skip, 1)` new ones taken (all ones when the size
/// overflows) and cleared.
pub fn set_update_skip(e: &mut Engine, this: Ptr<NiPSysBoundUpdateModifier>, skip: u32) {
    let skip = low_half(skip) as i16;
    let old = e.get(this, NiPSysBoundUpdateModifier::m_pkSkipBounds);
    e.set(this, NiPSysBoundUpdateModifier::m_sUpdateSkip, skip);
    e.call(DEALLOCATE, &args![MEMORY_MANAGER, old]);
    let count = if skip > 1 { i32::from(skip) } else { 1 };
    let count = count as u32;
    let bytes = count.saturating_mul(0x10);
    let bounds = e.call(ALLOCATE, &args![MEMORY_MANAGER, bytes]).u32();
    e.set(this, NiPSysBoundUpdateModifier::m_pkSkipBounds, bounds);
    for i in 0..count {
        clear_bound(e, this, i);
    }
}

// Typed from 00c2df20
/// `NiPSysBoundUpdateModifier::Update`: without active particles the data's bound is cleared and the
/// update count reset. Mesh and strip particle data (by their RTTI chains, data slot +8) go to
/// `00c2dce0` / `00c2d880`. Otherwise, with a skip of -1 the skip becomes `capacity / 50 + 1`
/// ([`set_update_skip`]); the step is the skip, at most `count / 50 + 1`, at least 1. From the update
/// count, every step-th particle widens a box by its radius (`size * radius`) on each axis; the box's
/// centre and half diagonal become the bound at the update count; the bounds from the step up to the
/// skip are cleared; the bound at the update count merged with the others of the first `step` that
/// have a radius (`00a7f3f0`) becomes the data's bound, whose radius falls back to the first
/// particle's `radius * size` when zero. The update count then advances, wrapping at the step.
///
/// Native float maths in the running thread's precision ([`Real`]); the root of the half diagonal is
/// `FSQRT` at that precision in place of the CRT's `_CIsqrt`.
pub fn bound_update(
    e: &mut Engine,
    this: Ptr<NiPSysBoundUpdateModifier>,
    time: f32,
    data: Ptr<NiPSysData>,
) {
    if float_precision() {
        bound_update_in::<f32>(e, this, time, data);
    } else {
        bound_update_in::<f64>(e, this, time, data);
    }
}

/// [`bound_update`] in `R`.
fn bound_update_in<R: Real>(
    e: &mut Engine,
    this: Ptr<NiPSysBoundUpdateModifier>,
    time: f32,
    data: Ptr<NiPSysData>,
) {
    let count = e.get(data, NiPSysData::m_usActive);
    let data_bound = data.addr() + NiPSysData::m_kBound.off;
    if count == 0 {
        let zero = read3(e, POINT_ZERO);
        write3(e, data_bound, zero);
        e.mem.set_u32(data_bound + 0xc, 0.0f32.to_bits());
        e.set(this, NiPSysBoundUpdateModifier::m_usUpdateCount, 0);
        return;
    }
    for (rtti, special) in [
        (MESH_PSYS_DATA_RTTI, MESH_BOUND_UPDATE),
        (STRIP_PSYS_DATA_RTTI, STRIP_BOUND_UPDATE),
    ] {
        let mut class = e.vcall(data.addr(), SLOT_GET_RTTI, &args![]).u32();
        while class != 0 {
            if class == rtti {
                e.call(special, &args![this, time.to_bits(), data]);
                return;
            }
            class = e.mem.u32(class + 4);
        }
    }
    if e.get(this, NiPSysBoundUpdateModifier::m_sUpdateSkip) == -1 {
        let capacity = i32::from(e.get(data, NiPSysData::m_usVertices));
        set_update_skip(e, this, (capacity / 50 + 1) as u32);
    }
    let count = i32::from(count);
    let limit = count / 50 + 1;
    let mut step = i32::from(e.get(this, NiPSysBoundUpdateModifier::m_sUpdateSkip));
    if step >= limit {
        step = limit;
    }
    if step <= 1 {
        step = 1;
    }
    let vertices = e.get(data, NiPSysData::m_pkVertex);
    let sizes = e.get(data, NiPSysData::m_pfSizes);
    let radii = e.get(data, NiPSysData::m_pfRadii);
    // The first vertex starts both corners; the minima are x87 registers.
    let first = read3(e, vertices);
    let mut low = first.map(R::load);
    let mut high = first;
    let start = u32::from(e.get(this, NiPSysBoundUpdateModifier::m_usUpdateCount));
    let mut i = start as i32;
    while i < count {
        let k = i as u32;
        let vertex = vertices + 0xc * k;
        let r = R::load(
            (R::load(e.mem.u32(sizes + 4 * k)) * R::load(e.mem.u32(radii + 4 * k))).store(),
        );
        for (axis, a) in (0u32..3).zip(0usize..) {
            let v = e.mem.u32(vertex + 4 * axis);
            let below = R::load((R::load(v) - r).store());
            if !(below > low[a]) {
                low[a] = below;
            }
            let above = if axis == 2 {
                (r + R::load(v)).store()
            } else {
                (R::load(v) + r).store()
            };
            if !(R::load(above) < R::load(high[a])) {
                high[a] = above;
            }
        }
        i += step;
    }
    let sums = [0, 1, 2].map(|a| (R::load(high[a]) + low[a]).store());
    // The double 0.5 (a float exactly, so a product by it rounds as the exe's).
    let half = R::narrow(e.global::<f64>(HALF));
    let bound = e.get(this, NiPSysBoundUpdateModifier::m_pkSkipBounds) + 0x10 * start;
    let center = [
        (R::load(sums[0]) * half).store(),
        (R::load(sums[1]) * half).store(),
        (half * R::load(sums[2])).store(),
    ];
    write3(e, bound, center);
    let extent = [0, 1, 2].map(|a| (R::load(high[a]) - low[a]).store());
    let diagonal = native_length::<R>(extent[0], extent[1], extent[2]);
    let radius = (R::load(diagonal) * half).store();
    let index = u32::from(e.get(this, NiPSysBoundUpdateModifier::m_usUpdateCount));
    let bounds = e.get(this, NiPSysBoundUpdateModifier::m_pkSkipBounds);
    e.mem.set_u32(bounds + 0x10 * index + 0xc, radius);
    let mut j = step;
    while j < i32::from(e.get(this, NiPSysBoundUpdateModifier::m_sUpdateSkip)) {
        clear_bound(e, this, j as u32);
        j += 1;
    }
    e.with_stack(0x10, |e, merged| {
        let index = u32::from(e.get(this, NiPSysBoundUpdateModifier::m_usUpdateCount));
        let source = e.get(this, NiPSysBoundUpdateModifier::m_pkSkipBounds) + 0x10 * index;
        let center = read3(e, source);
        write3(e, merged.addr(), center);
        let r = e.mem.u32(source + 0xc);
        e.mem.set_u32(merged.addr() + 0xc, r);
        for other in 1..step as u32 {
            let bound = e.get(this, NiPSysBoundUpdateModifier::m_pkSkipBounds) + 0x10 * other;
            // `FLDZ; FCOM radius; TEST AH,0x44; JNP`: bounds with a zero radius are left out.
            if f32::from_bits(e.mem.u32(bound + 0xc)) != 0.0 {
                typed_call!(
                    culling::FAMILY,
                    0x00a7_f3f0,
                    culling::bound::merge(e, merged.cast(), Ptr::new(bound))
                );
            }
        }
        let radius = e.mem.u32(merged.addr() + 0xc);
        e.mem.set_u32(data_bound + 0xc, radius);
        let center = read3(e, merged.addr());
        write3(e, data_bound, center);
        if f32::from_bits(radius) == 0.0 {
            let radii = e.get(data, NiPSysData::m_pfRadii);
            let sizes = e.get(data, NiPSysData::m_pfSizes);
            let fallback = (R::load(e.mem.u32(radii)) * R::load(e.mem.u32(sizes))).store();
            e.mem.set_u32(data_bound + 0xc, fallback);
        }
    });
    let next = e
        .get(this, NiPSysBoundUpdateModifier::m_usUpdateCount)
        .wrapping_add(1);
    e.set(this, NiPSysBoundUpdateModifier::m_usUpdateCount, next);
    if i32::from(next) >= step {
        e.set(this, NiPSysBoundUpdateModifier::m_usUpdateCount, 0);
    }
}

/// `(to - from) * t + from` for component `k` of two of the simple colour modifier's fields, in `R`,
/// stored as a float.
fn lerp<R: Real>(e: &Engine, this: u32, from: u32, to: u32, t: u32, k: u32) -> u32 {
    let from = R::load(e.mem.u32(this.wrapping_add(from + 4 * k)));
    let to = R::load(e.mem.u32(this.wrapping_add(to + 4 * k)));
    ((to - from) * R::load(t) + from).store()
}

// Typed from 00c602e0
/// `BSPSysSimpleColorModifier::Update` (the time word not read): with a colour array, each active
/// particle is coloured by its age fraction (age / life span, stored as a float). RGB: before colour 2
/// starts (when the colour 1 to 2 span is not zero) colour 1 until colour 1 ends, then blended toward
/// colour 2; otherwise colour 2 until it ends (or always when the colour 2 to 3 span is zero), blended
/// toward colour 3 until colour 3 starts, then colour 3. Alpha: past a non-zero fade-out point blended
/// from colour 2's to colour 3's alpha over the rest of the life; before a non-zero fade-in point from
/// colour 1's to colour 2's; otherwise colour 2's.
///
/// Native float maths in the running thread's precision ([`Real`]); the colours the exe moves through
/// the x87 are copied plain (ADR-0014: only a signalling NaN's payload could differ).
pub fn simple_color(
    e: &mut Engine,
    this: Ptr<BSPSysSimpleColorModifier>,
    time: u32,
    data: Ptr<NiPSysData>,
) {
    if float_precision() {
        simple_color_in::<f32>(e, this, time, data);
    } else {
        simple_color_in::<f64>(e, this, time, data);
    }
}

/// [`simple_color`] in `R`.
fn simple_color_in<R: Real>(
    e: &mut Engine,
    this: Ptr<BSPSysSimpleColorModifier>,
    _time: u32,
    data: Ptr<NiPSysData>,
) {
    const COLOR1: u32 = BSPSysSimpleColorModifier::colors.off;
    const COLOR2: u32 = COLOR1 + 0x10;
    const COLOR3: u32 = COLOR1 + 0x20;
    const ALPHA: u32 = 0xc;
    const FADE_IN: u32 = BSPSysSimpleColorModifier::fade_in.off;
    const FADE_OUT: u32 = BSPSysSimpleColorModifier::fade_out.off;
    const COLOR1_END: u32 = BSPSysSimpleColorModifier::color1_end.off;
    const COLOR2_START: u32 = BSPSysSimpleColorModifier::color2_start.off;
    const COLOR2_END: u32 = BSPSysSimpleColorModifier::color2_end.off;
    const COLOR3_START: u32 = BSPSysSimpleColorModifier::color3_start.off;
    let this = this.addr();
    let colors = e.get(data, NiPSysData::m_pkColor);
    if colors == 0 {
        return;
    }
    let field = |e: &Engine, offset: u32| R::load(e.mem.u32(this.wrapping_add(offset)));
    // The exe compares with the double 0.0 at `01012060`; a compare is exact at either precision.
    let zero = R::narrow(e.mem.f64(ZERO_DOUBLE));
    let span1 = (field(e, COLOR2_START) - field(e, COLOR1_END)).store();
    let span2 = (field(e, COLOR3_START) - field(e, COLOR2_END)).store();
    let count = e.get(data, NiPSysData::m_usActive);
    let mut i = 0u16;
    while u32::from(i) < u32::from(count) {
        let info = e
            .get(data, NiPSysData::m_pkParticleInfo)
            .wrapping_add(u32::from(i) * INFO_SIZE);
        let age_bits = (R::load(e.mem.u32(info.wrapping_add(INFO_AGE)))
            / R::load(e.mem.u32(info.wrapping_add(INFO_LIFE_SPAN))))
        .store();
        let age = R::load(age_bits);
        let out = colors.wrapping_add(u32::from(i) * 16);
        let copy = |e: &mut Engine, color: u32| {
            for k in 0..3 {
                let value = e.mem.u32(this.wrapping_add(color + 4 * k));
                e.mem.set_u32(out.wrapping_add(4 * k), value);
            }
        };
        let in_first_part = R::load(span1) != zero && field(e, COLOR2_START) > age;
        if in_first_part {
            if field(e, COLOR1_END) > age {
                copy(e, COLOR1);
            } else {
                let t = ((age - field(e, COLOR1_END)) / R::load(span1)).store();
                for k in 0..3 {
                    let value = lerp::<R>(e, this, COLOR1, COLOR2, t, k);
                    e.mem.set_u32(out.wrapping_add(4 * k), value);
                }
            }
        } else if R::load(span2) == zero || !(field(e, COLOR2_END) < age) {
            copy(e, COLOR2);
        } else if field(e, COLOR3_START) < age {
            copy(e, COLOR3);
        } else {
            let t = ((age - field(e, COLOR2_END)) / R::load(span2)).store();
            for k in 0..3 {
                let value = lerp::<R>(e, this, COLOR2, COLOR3, t, k);
                e.mem.set_u32(out.wrapping_add(4 * k), value);
            }
        }
        let fade_out = field(e, FADE_OUT);
        let fade_in = field(e, FADE_IN);
        let alpha = if fade_out != zero && fade_out < age {
            let rest = R::ONE - field(e, FADE_OUT);
            let t = ((age - field(e, FADE_OUT)) / rest).store();
            lerp::<R>(e, this, COLOR2 + ALPHA, COLOR3 + ALPHA, t, 0)
        } else if fade_in != zero && fade_in > age {
            let t = (age / fade_in).store();
            lerp::<R>(e, this, COLOR1 + ALPHA, COLOR2 + ALPHA, t, 0)
        } else {
            e.mem.u32(this.wrapping_add(COLOR2 + ALPHA))
        };
        e.mem.set_u32(out.wrapping_add(ALPHA), alpha);
        i = i.wrapping_add(1);
    }
}
