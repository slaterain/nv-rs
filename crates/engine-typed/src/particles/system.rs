//! A particle system's update: its update controller, the system's update and the modifier walk.

use super::modifiers;
use super::{NiParticleSystem, NiTimeController, FAMILY};
use crate::native::{float_precision, Real};
use crate::slot_target;
use engine_core::ptr::Ptr;
use engine_core::{args, typed_call, Engine};

/// The key times the update controller sets first (`-FLT_MAX` at `01015f5c`, and `010be6a8`) and
/// the "no time yet" the last time is compared with (`-FLT_MAX`, the double at `010241b0`).
const LO_KEY_TIME_START: u32 = 0x0101_5f5c;
const HI_KEY_TIME_START: u32 = 0x010b_e6a8;
const NO_TIME_DOUBLE: u32 = 0x0102_41b0;
/// `NiTimeController::DontDoUpdate` (`00a36250`, `thiscall`, the time).
const DONT_DO_UPDATE: u32 = 0x00a3_6250;
/// `NiPSysEmitterCtlr`'s `NiRTTI` record, an object's `GetRTTI` slot (+0x08) and a record's base
/// (+4).
const EMITTER_CTLR_RTTI: u32 = 0x0120_24e8;
const SLOT_GET_RTTI: u32 = 0x08;
const RTTI_BASE: u32 = 0x04;
/// The target's controllers (`NiObjectNET::m_spControllers`, +0x0c), an emitter controller's flags
/// byte (+0x08) and emitter (`NiPSysModifierCtlr::m_pkModifier`, +0x3c), an emitter's life span
/// (`NiPSysEmitter::m_fLifeSpan`, +0x48).
const TARGET_CONTROLLERS: u32 = 0x0c;
const CONTROLLER_FLAGS: u32 = 0x08;
const CONTROLLER_MODIFIER: u32 = 0x3c;
const EMITTER_LIFE_SPAN: u32 = 0x48;
/// The system's particle data (`NiGeometry::m_spModelData`, +0xb8).
const MODEL_DATA: u32 = 0xb8;
/// The system's vtable slots: `UpdateSystem` (+0xf0) and `Do_UpdateSystem` (+0xf4).
const SLOT_UPDATE_SYSTEM: u32 = 0xf0;
const SLOT_DO_UPDATE_SYSTEM: u32 = 0xf4;

/// The update hand-off `UpdateSystem` checks first: its switch (`012024d8`, a byte), the object at
/// `011f5b04` (its byte +0x1b0, its slot +0x8c with the word at `011ae6e0`), the thread
/// (`012024dc`, against `GetCurrentThreadId`, import `00fdf1d0`), `00c25c40` (an object or null),
/// `00c258e0` (that object, the system and the time) and the object's slot +0x94.
const HANDOFF_SWITCH: u32 = 0x0120_24d8;
const HANDOFF_MANAGER: u32 = 0x011f_5b04;
const HANDOFF_THREAD: u32 = 0x0120_24dc;
const HANDOFF_WORD: u32 = 0x011a_e6e0;
const GET_CURRENT_THREAD_ID: u32 = 0x00fd_f1d0;
const HANDOFF_OBJECT: u32 = 0x00c2_5c40;
const HANDOFF_WITH: u32 = 0x00c2_58e0;

/// `Do_UpdateSystem`'s callees: `PerformSystemReset` (`00c1b120`) and
/// `NiGeometryData::MarkAsChanged` (`00a67090`, with 7); the data's slot +0xac
/// (`ResolveAddedParticles`), a modifier's slot +0xa0 (true for the one kept for last) and its byte
/// +0x14 (`m_bActive`), and an object's slot +0x10 (true for one that takes the new-particles flag
/// `0x400000` at +0x30).
const PERFORM_SYSTEM_RESET: u32 = 0x00c1_b120;
const MARK_AS_CHANGED: u32 = 0x00a6_7090;
const SLOT_RESOLVE_ADDED: u32 = 0xac;
const SLOT_KEEP_FOR_LAST: u32 = 0xa0;
const MODIFIER_ACTIVE: u32 = 0x14;
const SLOT_TAKES_FLAG: u32 = 0x10;
const NEW_PARTICLES_FLAG: u32 = 0x0040_0000;

// Typed from 00c271c0
/// `NiPSysUpdateCtlr::Update` (`thiscall`, the update data, whose first word is the time): the key
/// times become their start values and the flags lose bit 1 and gain bit 2; the first controller of
/// the target whose RTTI chain holds `NiPSysEmitterCtlr`'s is the emitter controller; when the last
/// time is not "no time yet" (or is NaN), there is one, its flags have neither bit 1 nor bit 2 and its
/// emitter's life span is below the time since the last time, the last time jumps to the time. Then
/// `DontDoUpdate` with the time, both key times 0, and with a target and no skip the target's
/// `UpdateSystem` (+0xf0) with the scaled time: [`update_system`] directly when the slot holds it.
pub fn update_ctlr(e: &mut Engine, this: Ptr<NiTimeController>, time: Ptr) {
    // The key times, the last time and the time words the exe moves through the x87 are copied plain
    // (ADR-0014: only a signalling NaN's payload could differ).
    let lo_start = e.global::<u32>(LO_KEY_TIME_START);
    e.set(this, NiTimeController::m_fLoKeyTime, lo_start);
    let hi_start = e.global::<u32>(HI_KEY_TIME_START);
    e.set(this, NiTimeController::m_fHiKeyTime, hi_start);
    let flags = e.get(this, NiTimeController::m_uFlags);
    e.set(this, NiTimeController::m_uFlags, (flags & 0xfffd) | 4);

    let target = e.get(this, NiTimeController::m_pkTarget);
    let mut controller = e.mem.u32(target.wrapping_add(TARGET_CONTROLLERS));
    let mut emitter_ctlr = 0u32;
    let mut emitter = 0u32;
    'search: while controller != 0 {
        let mut rtti = e.vcall(controller, SLOT_GET_RTTI, &[]).u32();
        while rtti != 0 {
            if rtti == EMITTER_CTLR_RTTI {
                emitter = e.mem.u32(controller.wrapping_add(CONTROLLER_MODIFIER));
                emitter_ctlr = controller;
                break 'search;
            }
            rtti = e.mem.u32(rtti.wrapping_add(RTTI_BASE));
        }
        controller = e
            .mem
            .u32(controller.wrapping_add(NiTimeController::m_spNext.off));
    }

    // `FCOMP double; TEST AH,0x44; JNP`: on when not equal or unordered.
    let last = e.get(this, NiTimeController::m_fLastTime);
    let no_time = f64::from_bits(e.global::<u64>(NO_TIME_DOUBLE));
    if !(f64::from(f32::from_bits(last)) == no_time)
        && emitter_ctlr != 0
        && e.mem.u8(emitter_ctlr.wrapping_add(CONTROLLER_FLAGS)) & 6 == 0
    {
        let now = e.mem.u32(time.addr());
        let last = e.get(this, NiTimeController::m_fLastTime);
        let life_span = e.mem.u32(emitter.wrapping_add(EMITTER_LIFE_SPAN));
        // `FLD life; FCOMPP; TEST AH,5; JP`: on when the life span is below the difference, which the
        // exe keeps in a register at the running thread's precision.
        let below = if float_precision() {
            life_span_below::<f32>(life_span, now, last)
        } else {
            life_span_below::<f64>(life_span, now, last)
        };
        if below {
            let now = e.mem.u32(time.addr());
            e.set(this, NiTimeController::m_fLastTime, now);
        }
    }

    let now = e.mem.u32(time.addr());
    let skip = e.call(DONT_DO_UPDATE, &args![this, now]).bool();
    e.set(this, NiTimeController::m_fHiKeyTime, 0);
    e.set(this, NiTimeController::m_fLoKeyTime, 0);
    let target = e.get(this, NiTimeController::m_pkTarget);
    if target != 0 && !skip {
        let scaled = e.get(this, NiTimeController::m_fScaledTime);
        if slot_target(e, target, SLOT_UPDATE_SYSTEM) == 0x00c1_ae40 {
            typed_call!(
                FAMILY,
                0x00c1_ae40,
                update_system(e, Ptr::new(target), f32::from_bits(scaled))
            );
        } else {
            e.vcall(target, SLOT_UPDATE_SYSTEM, &args![scaled]);
        }
    }
}

// Typed from 00c1ae40
/// `NiParticleSystem::UpdateSystem` (`thiscall`, the time): with the hand-off switch on, its object
/// there with the byte +0x1b0 set, the data's bound radius above 0.0, the current thread the hand-off's
/// and `00c25c40` answering an object, `00c258e0` hands the system over; when the manager's slot +0x8c
/// then answers true the update ends there, otherwise the object's slot +0x94 runs. Then (and in every
/// other case) `Do_UpdateSystem` (+0xf4) with the time: [`do_update_system`] directly when the slot
/// holds it.
pub fn update_system(e: &mut Engine, this: Ptr<NiParticleSystem>, time: f32) {
    let time_bits = time.to_bits();
    if e.global::<u8>(HANDOFF_SWITCH) != 0 {
        let manager = e.global::<u32>(HANDOFF_MANAGER);
        if manager != 0 && e.mem.u8(manager.wrapping_add(0x1b0)) != 0 {
            let data = e.mem.u32(this.addr().wrapping_add(MODEL_DATA));
            // FLDZ; FCOMP [data+0x1c]; TEST AH,5; JP: on only when 0.0 < the value.
            if 0.0 < e.mem.f32(data.wrapping_add(0x1c)) {
                let thread = e.call(GET_CURRENT_THREAD_ID, &args![]).u32();
                if thread == e.global::<u32>(HANDOFF_THREAD) {
                    let object = e.call(HANDOFF_OBJECT, &args![]).u32();
                    if object != 0 {
                        e.call(HANDOFF_WITH, &args![object, this, time_bits]);
                        let manager = e.global::<u32>(HANDOFF_MANAGER);
                        let word = e.global::<u32>(HANDOFF_WORD);
                        if e.vcall(manager, 0x8c, &args![object, word]).bool() {
                            return;
                        }
                        e.vcall(object, 0x94, &args![]);
                    }
                }
            }
        }
    }
    if slot_target(e, this.addr(), SLOT_DO_UPDATE_SYSTEM) == 0x00c1_b260 {
        typed_call!(
            FAMILY,
            0x00c1_b260,
            do_update_system(e, this, f32::from_bits(time_bits))
        );
    } else {
        e.vcall(this.addr(), SLOT_DO_UPDATE_SYSTEM, &args![time_bits]);
    }
}

// Typed from 00c1b260
/// `NiParticleSystem::Do_UpdateSystem` (`thiscall`, the time): notes whether the data's bound radius
/// word is 0, resolves the added particles (data slot +0xac); with the reset flag set or the time
/// below the last time (ordered) `PerformSystemReset` runs and the flag is cleared; a last time equal
/// to "no time yet" becomes the time. Then each modifier of the list: one whose slot +0xa0 answers
/// true is kept as the last such one; otherwise an active one (+0x14) runs its `Update` with the time
/// and the data ([`modifiers::update`]); the kept one runs after the walk the same way when active.
/// `MarkAsChanged(7)` on the data; the last time becomes the time; when the radius word was 0 and no
/// longer is, the first of the system's parents (its +0x18, then each parent's +0x18) whose slot +0x10
/// answers nonzero gets the flag `0x400000`; the walk starts at the parent, not at the system.
pub fn do_update_system(e: &mut Engine, this: Ptr<NiParticleSystem>, time: f32) {
    let time_bits = time.to_bits();
    let data = e.mem.u32(this.addr().wrapping_add(MODEL_DATA));
    let was_empty = e.mem.u32(data.wrapping_add(0x1c)) == 0;
    e.vcall(data, SLOT_RESOLVE_ADDED, &args![]);
    // FLD m_fLastTime; FLD time; FCOMPP; TEST AH,5; JP: reset when time < last (ordered).
    if e.get(this, NiParticleSystem::m_bResetSystem) != 0
        || time < f32::from_bits(e.get(this, NiParticleSystem::m_fLastTime))
    {
        e.call(PERFORM_SYSTEM_RESET, &args![this]);
        e.set(this, NiParticleSystem::m_bResetSystem, 0);
    }
    // FCOMP double; TEST AH,0x44; JP: equal (ordered) only.
    let unset: f64 = e.global(NO_TIME_DOUBLE);
    if f64::from(f32::from_bits(e.get(this, NiParticleSystem::m_fLastTime))) == unset {
        e.set(this, NiParticleSystem::m_fLastTime, time_bits);
    }
    let list = this
        .addr()
        .wrapping_add(NiParticleSystem::m_kModifierList.off);
    if e.mem.u32(list) != 0 {
        let mut last = 0u32;
        let mut item = e.mem.u32(list);
        while item != 0 {
            let modifier = e.mem.u32(item.wrapping_add(8));
            item = e.mem.u32(item);
            if e.vcall(modifier, SLOT_KEEP_FOR_LAST, &args![]).bool() {
                last = modifier;
            } else if e.mem.u8(modifier.wrapping_add(MODIFIER_ACTIVE)) != 0 {
                modifiers::update(e, modifier, time_bits, data);
            }
        }
        if last != 0 && e.mem.u8(last.wrapping_add(MODIFIER_ACTIVE)) != 0 {
            modifiers::update(e, last, time_bits, data);
        }
    }
    e.call(MARK_AS_CHANGED, &args![data, 7u32]);
    e.set(this, NiParticleSystem::m_fLastTime, time_bits);
    if was_empty && e.mem.u32(data.wrapping_add(0x1c)) != 0 {
        let mut object = e.mem.u32(this.addr().wrapping_add(0x18));
        while object != 0 {
            if e.vcall(object, SLOT_TAKES_FLAG, &args![]).u32() != 0 {
                let flags = e.mem.u32(object.wrapping_add(0x30));
                e.mem
                    .set_u32(object.wrapping_add(0x30), flags | NEW_PARTICLES_FLAG);
                return;
            }
            object = e.mem.u32(object.wrapping_add(0x18));
        }
    }
}

/// Whether `life span < now - last` (the update controller's emitter test) in `R`: the difference
/// unrounded, as the exe keeps it in a register.
fn life_span_below<R: Real>(life_span: u32, now: u32, last: u32) -> bool {
    R::load(life_span) < R::load(now) - R::load(last)
}
