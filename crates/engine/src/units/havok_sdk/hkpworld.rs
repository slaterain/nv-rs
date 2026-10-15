//! `Havok SDK/hkpworld.cpp` (Xbox PDB source unit), subsystem `Havok SDK`: its functions in
//! FalloutNV.exe 1.4.0.525, translated (docs/ENGINE_CRATE.md).
//!
//! The unit is large (115 functions). Sessions translate it in address order; the first
//! session did the 60 queue entries `00c90e40` to `00c929e0` and the next one continues at
//! `00c92a30`.
//!
//! Layouts and helpers come first. `HkpWorld` is the PC `hkpWorld` (0x350 bytes): its field
//! names and offsets are the Xbox PDB's, and every offset used here is read by the PC code at
//! the same place. Fields inside `m_dynamicsStepInfo` (+0x1d0) that the PDB does not name
//! separately are read by offset (the `DYNAMICS_*` constants).
//!
//! Conventions of this file:
//! - `hkpWorld` methods are `__thiscall` (`this` in ECX); the uniform form passes it first.
//! - A float passed through the x87 stack (`FLD`/`FSTP`) goes through [`x87_quiet`], which
//!   quiets a signalling NaN as the hardware does.
//! - Pieces the compiler outlined from a bigger function take registers the calling
//!   convention does not have (`ESI`, `EDI`, `EAX` set by the caller). They cannot be called
//!   by address in the uniform form: they are translated as plain Rust functions with the
//!   registers as leading parameters, are not registered, and carry a `Translation
//!   incomplete` marker saying so. `00c90f40` is such a piece that also reads its caller's
//!   stack frame; it is not translated.
//! - `RDTSC` (the profiling records of the ray and linear casts) has no model in the engine:
//!   the host clock stands in for the time stamp, which nothing but the profiler reads.

#[allow(unused_imports)]
use crate::prelude::*;

// ---------------------------------------------------------------------------------------
// Callees outside this file, by address.
// ---------------------------------------------------------------------------------------

/// `TlsGetValue` import slot, and the TLS index of the profiling record stream.
const TLS_GET_VALUE: u32 = 0x00fd_f044;
const PROFILE_SLOT: u32 = 0x0126_835c;
/// `EnterCriticalSection` and `LeaveCriticalSection` import slots.
const ENTER_CRITICAL_SECTION: u32 = 0x00fd_f05c;
const LEAVE_CRITICAL_SECTION: u32 = 0x00fd_f100;
/// Profiling labels: `"TtworldCastRayCollector"`, `"TtworldCastRay"`, `"TtworldLinCast"` and
/// the end-of-section label `"Et"`.
const PROFILE_CAST_RAY_COLLECTOR: u32 = 0x010c_6490;
const PROFILE_CAST_RAY: u32 = 0x010c_64a8;
const PROFILE_LINEAR_CAST: u32 = 0x010c_64b8;
const PROFILE_END: u32 = 0x010c_41f8;
/// Increment and decrement of the reference count of an `hkReferencedObject` (`this` only);
/// the count is the signed word at +6.
const ADD_REFERENCE: u32 = 0x00c9_0510;
const REMOVE_REFERENCE: u32 = 0x00c9_05b0;
/// Hands out the thread's memory router (no arguments); `+0x10` of it is the allocator whose
/// vtable slot +4 allocates `(size)` and slot +8 frees `(block, size)`.
const MEMORY_ROUTER: u32 = 0x00c8_5750;
/// `00c90490` / `00c904e0`: the calls around the `m_isLocked` count at +0xbc (`this` only).
const LOCK_ENTER: u32 = 0x00c9_0490;
const LOCK_LEAVE: u32 = 0x00c9_04e0;
/// The `m_pendingOperations` queue (`hkpWorldOperationQueue` in the Xbox PDB): `00cf7c10` and
/// `00cf7ae0` take only the queue; `00cf7690` takes the operation record.
const QUEUE_FLUSH_PENDING: u32 = 0x00cf_7c10;
const QUEUE_FLUSH_BODY_PENDING: u32 = 0x00cf_7ae0;
const QUEUE_ENQUEUE: u32 = 0x00cf_7690;
/// `hkpSimulation` calls: `00cf86e0` (one float), `00cf86f0` (bool result).
const SIMULATION_SET_TIME: u32 = 0x00cf_86e0;
const SIMULATION_TEST: u32 = 0x00cf_86f0;
/// Entity callees (`00c9c040(entity, world, shape)` ...).
const SET_CACHED_SHAPE_DATA: u32 = 0x00c9_c040;
const SET_TIME_INFORMATION: u32 = 0x00cf_1480;
const ADD_ENTITY_SI: u32 = 0x00cb_4640;
const ADD_ENTITY_BP: u32 = 0x00cb_59f0;
const FIRE_WORLD_ENTITY_ADDED: u32 = 0x00d0_0500;
const FIRE_ENTITY_ADDED: u32 = 0x00cf_fa00;
const REMOVE_ENTITY_SI: u32 = 0x00cb_5c10;
const FIRE_WORLD_ENTITY_REMOVED: u32 = 0x00d0_05a0;
const FIRE_ENTITY_REMOVED: u32 = 0x00cf_fab0;
const REMOVE_ENTITY_BP: u32 = 0x00cb_4740;
const ENTITY_RELEASE_BOUNDING_DATA: u32 = 0x00d0_1a80;
const FIND_AGENT: u32 = 0x00cf_dd70;
const ADD_PAIRS: u32 = 0x00cf_7080;
/// `00d01d20(world)` (cdecl, one argument) and `00d03290(simulation, a, b)` (thiscall).
const WATCH_HEAP_MEMORY: u32 = 0x00d0_1d20;
const SIMULATION_FINISH_MT_STEP: u32 = 0x00d0_3290;
/// Ray and linear casters (`this` = a caller-built object): `00cbf860`, `00cfab30`, `00cfaed0`.
const WORLD_RAY_CASTER_CAST_RAY: u32 = 0x00cb_f860;
const SIMPLE_RAY_CASTER_CAST_RAY: u32 = 0x00cf_ab30;
const LINEAR_CASTER_LINEAR_CAST: u32 = 0x00cf_aed0;
/// Vtables of the objects the casts build on their stack.
const RAY_CASTER_VTABLE: u32 = 0x010c_3bd4;
const SIMPLE_RAY_CASTER_VTABLE: u32 = 0x010c_647c;
const LINEAR_CASTER_VTABLE: u32 = 0x010c_6488;
/// `float` constant stored into the linear caster object at +0x80.
const LINEAR_CASTER_DEFAULT: u32 = 0x0104_da4c;
/// Job queue calls: `00d04b90(queue, job, 1)` and `00d04380(semaphore)` (acquire).
const JOB_QUEUE_ADD_JOB: u32 = 0x00d0_4b90;
const SEMAPHORE_ACQUIRE: u32 = 0x00d0_4380;
/// `00d05de0(job)`: initializer of a job header (a bare `RET`).
const JOB_HEADER_INIT: u32 = 0x00d0_5de0;
/// `hkpPhysicsSystem` calls (`this` = the system).
const PHYSICS_SYSTEM_CONSTRUCT: u32 = 0x00d0_65e0;
const PHYSICS_SYSTEM_ADD_RIGID_BODY: u32 = 0x00d0_64a0;
const PHYSICS_SYSTEM_ADD_CONSTRAINT: u32 = 0x00d0_6540;
const PHYSICS_SYSTEM_ADD_ACTION: u32 = 0x00d0_6590;
const PHYSICS_SYSTEM_ADD_PHANTOM: u32 = 0x00d0_64f0;
/// The x87 square root helper (`_CIsqrt`): argument in ST0.
const X87_SQRT: u32 = 0x00ec_6040;
/// `00cfd450(object, world)`, `00cfca20(object, world)` and `00cfb240(registry, object,
/// kind)`: constructors and registration used by the outlined constructor piece `00c912e0`.
const MAKE_HELPER_FIRST: u32 = 0x00cf_d450;
const MAKE_HELPER_SECOND: u32 = 0x00cf_ca20;
const REGISTER_HELPER: u32 = 0x00cf_b240;
/// The `double` 0.5 that `00c90e60` multiplies with.
const HALF: u32 = 0x0101_1588;

// ---------------------------------------------------------------------------------------
// Layouts.
// ---------------------------------------------------------------------------------------

/// Offsets inside `m_dynamicsStepInfo` (+0x1d0), named only by their place in the world.
const DYNAMICS_FLOAT_1E4: u32 = 0x1e4;
const DYNAMICS_FLOAT_1E8: u32 = 0x1e8;
const DYNAMICS_FLOAT_240: u32 = 0x240;
const DYNAMICS_WORD_2F4: u32 = 0x2f4;
const DYNAMICS_WORD_2F8: u32 = 0x2f8;
const DYNAMICS_BYTE_304: u32 = 0x304;
/// Three consecutive bytes (+0x305, +0x306, +0x307).
const DYNAMICS_BYTES_305: u32 = 0x305;
const DYNAMICS_FLOAT_308: u32 = 0x308;

layout! {
    /// `hkpWorld` (Xbox PDB), 0x350 bytes. Only the fields the translations of this file use
    /// are listed; names and offsets are the PDB's, confirmed against the PC code.
    pub struct HkpWorld: 0x350 {
        /// `m_simulation` (Xbox PDB): `hkpSimulation*`.
        0x08 m_simulation: Ptr,
        /// `m_fixedIsland` (Xbox PDB): `hkpSimulationIsland*`.
        0x20 m_fixedIsland: Ptr,
        /// `m_activeSimulationIslands` (Xbox PDB): `hkArray` data pointer.
        0x28 m_activeSimulationIslands: Ptr,
        /// Size of `m_activeSimulationIslands`.
        0x2c m_activeSimulationIslandsSize: i32,
        /// `m_inactiveSimulationIslands` (Xbox PDB): `hkArray` data pointer.
        0x34 m_inactiveSimulationIslands: Ptr,
        /// Size of `m_inactiveSimulationIslands`.
        0x38 m_inactiveSimulationIslandsSize: i32,
        /// `m_memoryWatchDog` (Xbox PDB): `hkRefPtr`.
        0x50 m_memoryWatchDog: Ptr,
        /// `m_broadPhase` (Xbox PDB): `hkpBroadPhase*`.
        0x58 m_broadPhase: Ptr,
        /// `m_kdTreeManager` (Xbox PDB): `hkpTreeWorldManager*`.
        0x5c m_kdTreeManager: Ptr,
        /// `m_autoUpdateTree` (Xbox PDB).
        0x60 m_autoUpdateTree: bool,
        /// `m_broadPhaseDispatcher` (Xbox PDB): `hkpTypedBroadPhaseDispatcher*`.
        0x64 m_broadPhaseDispatcher: Ptr,
        /// `m_collisionInput` (Xbox PDB): `hkpProcessCollisionInput*`.
        0x78 m_collisionInput: Ptr,
        /// `m_collisionFilter` (Xbox PDB): `hkpCollisionFilter*`.
        0x7c m_collisionFilter: Ptr,
        /// `m_collisionDispatcher` (Xbox PDB).
        0x80 m_collisionDispatcher: Ptr,
        /// `m_convexListFilter` (Xbox PDB).
        0x84 m_convexListFilter: Ptr,
        /// `m_pendingOperations` (Xbox PDB): `hkpWorldOperationQueue*`.
        0x88 m_pendingOperations: Ptr,
        /// `m_pendingOperationsCount` (Xbox PDB).
        0x8c m_pendingOperationsCount: i32,
        /// `m_pendingBodyOperationsCount` (Xbox PDB).
        0x90 m_pendingBodyOperationsCount: i32,
        /// `m_criticalOperationsLockCount` (Xbox PDB).
        0x94 m_criticalOperationsLockCount: i32,
        /// `m_blockExecutingPendingOperations` (Xbox PDB).
        0x9c m_blockExecutingPendingOperations: bool,
        /// `m_criticalOperationsAllowed` (Xbox PDB).
        0x9d m_criticalOperationsAllowed: bool,
        /// `m_pendingOperationQueueCount` (Xbox PDB).
        0xa4 m_pendingOperationQueueCount: i32,
        /// `m_modifyConstraintCriticalSection` (Xbox PDB): `hkCriticalSection*`.
        0xb8 m_modifyConstraintCriticalSection: Ptr,
        /// `m_isLocked` (Xbox PDB).
        0xbc m_isLocked: i32,
        /// `m_lastEntityUid` (Xbox PDB).
        0x104 m_lastEntityUid: u32,
        /// `m_phantoms` (Xbox PDB): `hkArray` data pointer.
        0x110 m_phantoms: Ptr,
        /// Size of `m_phantoms`.
        0x114 m_phantomsSize: i32,
        /// `m_broadPhaseBorder` (Xbox PDB): `hkpBroadPhaseBorder*`.
        0x1bc m_broadPhaseBorder: Ptr,
        /// `m_collisionCallbackUtil` (Xbox PDB).
        0x1c0 m_collisionCallbackUtil: Ptr,
    }

    /// `hkpEntity` (Xbox PDB) as far as the world's functions read it (PC offsets).
    pub struct HkpEntity: 0x220 {
        /// `m_memSizeAndFlags` (Xbox PDB).
        0x04 m_memSizeAndFlags: u16,
        /// `m_collidable.m_shape` (Xbox PDB; `m_collidable` is at +0x10).
        0x10 m_collidableShape: Ptr,
        /// `m_collidable.m_motion` (Xbox PDB).
        0x18 m_collidableMotion: Ptr,
        /// A pointer inside the collidable's bounding-volume data (+0x54).
        0x54 m_boundingVolumeChildData: Ptr,
        /// `m_uid` (Xbox PDB).
        0xd4 m_uid: u32,
        /// Two words of the entity (+0xea, +0xec) the world clears when it adds the entity.
        0xea m_wordEa: u16,
        0xec m_wordEc: u16,
    }
}

/// `m_collidable` (Xbox PDB), +0x10 of the entity.
const ENTITY_COLLIDABLE: u32 = 0x10;
/// `m_collidable.m_broadPhaseHandle` (Xbox PDB), +0x24 of the entity.
const ENTITY_BROADPHASE_HANDLE: u32 = 0x24;
/// The entity's bounding-volume data (`m_collidable.m_boundingVolumeData`, +0x30).
const ENTITY_BOUNDING_DATA: u32 = 0x30;
/// The motion embedded in the entity (`m_motion`, +0xe0) and the swept transform inside it.
const ENTITY_MOTION: u32 = 0xe0;
const ENTITY_SWEPT_TRANSFORM: u32 = 0xf0;

// ---------------------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------------------

/// Low word of the time-stamp counter (`RDTSC`). The engine has no model of it: the host
/// clock stands in (only the profiling records take it).
fn time_stamp_low() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u32)
        .unwrap_or(0)
}

/// Appends a profiling record (`label`, time stamp) to the thread's record stream
/// (`TlsGetValue` of the index at `0126835c`) while its cursor (`+4`) is below its limit
/// (`+0xC`); a record is 12 bytes.
fn profile_record(e: &mut Engine, label: u32) {
    let slot: u32 = e.global(PROFILE_SLOT);
    let stream = e.call(TLS_GET_VALUE, &args![slot]).u32();
    let cursor = e.mem.u32(stream + 4);
    if cursor < e.mem.u32(stream + 0xc) {
        e.mem.set_u32(cursor, label);
        e.mem.set_u32(cursor + 4, time_stamp_low());
        e.mem.set_u32(stream + 4, cursor + 12);
    }
}

/// Allocates `size` bytes through the memory router's allocator (`00c85750`, vtable slot 4).
fn router_alloc(e: &mut Engine, size: u32) -> u32 {
    let router = e.call(MEMORY_ROUTER, &args![]).u32();
    let allocator = e.mem.u32(router + 0x10);
    e.vcall(allocator, 4, &args![size]).u32()
}

/// Frees a block of `size` bytes through the router's allocator (vtable slot 8).
fn router_free(e: &mut Engine, block: u32, size: u32) {
    let router = e.call(MEMORY_ROUTER, &args![]).u32();
    let allocator = e.mem.u32(router + 0x10);
    e.vcall(allocator, 8, &args![block, size]);
}

/// `FLD float` / `FSTP float` copy of the `float` at `from` to `to`.
fn copy_float(e: &mut Engine, from: u32, to: u32) {
    let value = x87_quiet(e.mem.f32(from));
    e.mem.set_f32(to, value);
}

fn copy_word(e: &mut Engine, from: u32, to: u32) {
    let value = e.mem.u32(from);
    e.mem.set_u32(to, value);
}

fn copy_byte(e: &mut Engine, from: u32, to: u32) {
    let value = e.mem.u8(from);
    e.mem.set_u8(to, value);
}

/// `MOVAPS` copy of a 16-byte vector.
fn copy_vector(e: &mut Engine, from: u32, to: u32) {
    let bytes = e.mem.bytes(from, 16);
    e.mem.write(to, &bytes);
}

/// `SHUFPS` splat: the 32-bit pattern `bits` into the four lanes at `to`.
fn splat_float(e: &mut Engine, bits: u32, to: u32) {
    for lane in 0..4 {
        e.mem.set_u32(to + 4 * lane, bits);
    }
}

/// The world has a kd-tree manager (+0x5c) and its slot +0x10 says it is in use: the test
/// every cast makes before preferring the tree.
fn kd_tree_in_use(e: &mut Engine, this: Ptr<HkpWorld>) -> bool {
    let tree = e.get(this, HkpWorld::m_kdTreeManager);
    !tree.is_null() && e.vcall(tree.addr(), 0x10, &args![]).bool()
}

/// The number of 64-element jobs `count` elements split into, as the dispatchers compute it:
/// `((count - 1 + ((count - 1) >> 31 & 0x3f)) >> 6) + 1`.
fn job_block_count(count: i32) -> i32 {
    let last = count.wrapping_sub(1);
    (last.wrapping_add((last >> 31) & 0x3f) >> 6).wrapping_add(1)
}

/// The entry `index` of the `hkArray` whose data pointer is `data`.
fn array_entry(e: &Engine, data: u32, index: i32) -> u32 {
    e.mem.u32(data.wrapping_add((index as u32).wrapping_mul(4)))
}

// ---------------------------------------------------------------------------------------
// Translations.
// ---------------------------------------------------------------------------------------

// Translated from 00c90e40 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpContactMgrFactory::calcContentStatistics` (Xbox PDB): tells the statistics collector
/// (first stack argument; its vtable slot +0x18 takes the class name `"hkpContactMgrFactory"`,
/// slot +0x1c takes nothing). The second stack argument is not read.
pub fn hkp_contact_mgr_factory_calc_content_statistics(
    e: &mut Engine,
    _this: Ptr,
    collector: Ptr,
    _unused_3: u32,
) {
    e.vcall(collector.addr(), 0x18, &args![0x010c_63a0u32]);
    e.vcall(collector.addr(), 0x1c, &args![]);
}

// Translated from 00c90e60 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (Xbox name unknown). Stores a first `float` `a` at +4, half of it (`a * 0.5`, the
/// `double` at `01011588`) at +0xC and a second `float` `b` at +8, then the ratios `b / a`
/// (+0x50), `b / (a * 0.5)` (+0x58), `a / b` (+0x54) and `(a * 0.5) / b` (+0x5C). The
/// 16-byte vectors at +0x30 and +0x40 hold `a / b` and `b / a` in all four lanes.
pub fn fn_00c90e60(e: &mut Engine, this: Ptr, a: f32, b: f32) {
    let base = this.addr();
    let a = x87_quiet(a);
    e.mem.set_f32(base + 4, a);
    let half: f64 = e.global(HALF);
    let half_a = (half * a as f64) as f32;
    e.mem.set_f32(base + 0xc, half_a);
    let b = x87_quiet(b);
    e.mem.set_f32(base + 8, b);
    e.mem.set_f32(base + 0x50, (b as f64 / a as f64) as f32);
    e.mem
        .set_f32(base + 0x58, (b as f64 / half_a as f64) as f32);
    e.mem.set_f32(base + 0x54, (a as f64 / b as f64) as f32);
    let a_over_b = e.mem.u32(base + 0x54);
    splat_float(e, a_over_b, base + 0x30);
    let b_over_a = e.mem.u32(base + 0x50);
    splat_float(e, b_over_a, base + 0x40);
    e.mem
        .set_f32(base + 0x5c, (half_a as f64 / b as f64) as f32);
}

// Translated from 00c90ed0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Called on the motion embedded in an entity (`+0xE0`) with the world's three
/// bytes at +0x305..+0x307. Compares `c & 3` with the low two bits of the byte at +9 (and
/// `c` with the whole byte): the first two arguments are complemented when the comparison
/// says "below", then shifted into the top two bits of the words at +0xA and +0xC.
pub fn fn_00c90ed0(e: &mut Engine, this: Ptr, a: u32, b: u32, c: i32) {
    let base = this.addr();
    let reference = e.mem.u8(base + 9) as u32;
    let mut first = a;
    if ((c as u32 & 3) as u8) < (reference & 3) as u8 {
        first = !first;
    }
    let mut second = b;
    if c < reference as i32 {
        second = !second;
    }
    let word_a = e.mem.u16(base + 0xa);
    e.mem
        .set_u16(base + 0xa, (word_a & 0x3fff) | (first << 14) as u16);
    let word_b = e.mem.u16(base + 0xc);
    e.mem
        .set_u16(base + 0xc, (word_b & 0x3fff) | (second << 14) as u16);
}

// Translated from 00c90f30 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpMultithreadConfig::hkpMultithreadConfig` (Xbox PDB): stores 0x46 in its first word and
/// returns `this`.
pub fn hkp_multithread_config_hkp_multithread_config(e: &mut Engine, this: Ptr) -> Ptr {
    e.mem.set_u32(this.addr(), 0x46);
    this
}

// Translated from 00c91000 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Copies the 16-byte vector at `source` to +0x10 (where the Xbox PDB layout has the
/// world's `m_gravity`).
pub fn fn_00c91000(e: &mut Engine, this: Ptr<HkpWorld>, source: Ptr) {
    copy_vector(e, source.addr(), this.addr() + 0x10);
}

// Translated from 00c91020 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::getCurrentTime` (Xbox PDB): the `float` at +0x14 of the simulation, in `ST0`.
pub fn hkp_world_get_current_time(e: &mut Engine, this: Ptr<HkpWorld>) -> f32 {
    let simulation = e.get(this, HkpWorld::m_simulation);
    x87_quiet(e.mem.f32(simulation.addr() + 0x14))
}

// Translated from 00c91030 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::getCurrentPsiTime` (Xbox PDB): the `float` at +0x18 of the simulation, in `ST0`.
pub fn hkp_world_get_current_psi_time(e: &mut Engine, this: Ptr<HkpWorld>) -> f32 {
    let simulation = e.get(this, HkpWorld::m_simulation);
    x87_quiet(e.mem.f32(simulation.addr() + 0x18))
}

// Translated from 00c91040 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Passes a `float` (through `FLD`/`FSTP`) to the simulation's `00cf86e0`.
pub fn fn_00c91040(e: &mut Engine, this: Ptr<HkpWorld>, time: f32) {
    let simulation = e.get(this, HkpWorld::m_simulation);
    e.call(SIMULATION_SET_TIME, &args![simulation, x87_quiet(time)]);
}

// Translated from 00c91060 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Tail-calls the simulation's `00cf86f0` and returns what it leaves in `EAX` (the
/// caller tests `AL`).
pub fn fn_00c91060(e: &mut Engine, this: Ptr<HkpWorld>) -> u32 {
    let simulation = e.get(this, HkpWorld::m_simulation);
    e.call(SIMULATION_TEST, &args![simulation]).u32()
}

// Translated from 00c910a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Does nothing; takes one stack argument.
pub fn fn_00c910a0(_e: &mut Engine, _this: Ptr<HkpWorld>, _unused_2: u32) {}

// Translated from 00c910b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::checkDeterminism` (Xbox PDB): does nothing in this build.
pub fn hkp_world_check_determinism(_e: &mut Engine, _this: Ptr<HkpWorld>) {}

// Translated from 00c910d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Replaces the world's `m_broadPhaseBorder` (+0x1bc): a previous border gets its
/// vtable slot +0x10 called and a reference removed (`00c905b0`), the new one (when not null)
/// gets a reference added (`00c90510`).
pub fn fn_00c910d0(e: &mut Engine, this: Ptr<HkpWorld>, border: Ptr) {
    let old = e.get(this, HkpWorld::m_broadPhaseBorder);
    if !old.is_null() {
        e.vcall(old.addr(), 0x10, &args![]);
        let current = e.get(this, HkpWorld::m_broadPhaseBorder);
        e.call(REMOVE_REFERENCE, &args![current]);
    }
    e.set(this, HkpWorld::m_broadPhaseBorder, border);
    if !border.is_null() {
        e.call(ADD_REFERENCE, &args![border]);
    }
}

// Translated from 00c91110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed getter of the world's `m_broadPhaseBorder` (+0x1bc).
pub fn fn_00c91110(e: &mut Engine, this: Ptr<HkpWorld>) -> Ptr {
    e.get(this, HkpWorld::m_broadPhaseBorder)
}

// Translated from 00c91120 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Clears `m_pendingOperationsCount` and tail-calls `00cf7c10` on the pending
/// operation queue (+0x88).
pub fn fn_00c91120(e: &mut Engine, this: Ptr<HkpWorld>) {
    e.set(this, HkpWorld::m_pendingOperationsCount, 0);
    let queue = e.get(this, HkpWorld::m_pendingOperations);
    e.call(QUEUE_FLUSH_PENDING, &args![queue]);
}

// Translated from 00c91140 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Clears `m_pendingBodyOperationsCount` and tail-calls `00cf7ae0` on the pending
/// operation queue.
pub fn fn_00c91140(e: &mut Engine, this: Ptr<HkpWorld>) {
    e.set(this, HkpWorld::m_pendingBodyOperationsCount, 0);
    let queue = e.get(this, HkpWorld::m_pendingOperations);
    e.call(QUEUE_FLUSH_BODY_PENDING, &args![queue]);
}

// Translated from 00c91160 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Hands one operation record to the pending operation queue (`00cf7690`). Every
/// caller builds the record on its stack with the operation number in its first byte.
pub fn fn_00c91160(e: &mut Engine, this: Ptr<HkpWorld>, operation: Ptr) {
    let queue = e.get(this, HkpWorld::m_pendingOperations);
    e.call(QUEUE_ENQUEUE, &args![queue, operation]);
}

// Translated from 00c91180 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Calls `00c90490` on the world, then counts `m_isLocked` up.
pub fn fn_00c91180(e: &mut Engine, this: Ptr<HkpWorld>) {
    e.call(LOCK_ENTER, &args![this]);
    let locked = e.get(this, HkpWorld::m_isLocked);
    e.set(this, HkpWorld::m_isLocked, locked.wrapping_add(1));
}

// Translated from 00c91190 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Counts `m_isLocked` down, then tail-calls `00c904e0` on the world.
pub fn fn_00c91190(e: &mut Engine, this: Ptr<HkpWorld>) {
    let locked = e.get(this, HkpWorld::m_isLocked);
    e.set(this, HkpWorld::m_isLocked, locked.wrapping_sub(1));
    e.call(LOCK_LEAVE, &args![this]);
}

// Translated from 00c911a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Does nothing.
pub fn fn_00c911a0(_e: &mut Engine, _this: Ptr<HkpWorld>) {}

// Translated from 00c911b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::unlockReadOnly` (Xbox PDB): does nothing in this build.
pub fn hkp_world_unlock_read_only(_e: &mut Engine, _this: Ptr<HkpWorld>) {}

/// Calls the import at `import_slot` (`EnterCriticalSection` or `LeaveCriticalSection`) on the
/// world's constraint-update critical section (+0xb8) when it has one; the one stack
/// argument of these functions is overwritten with the section and never read.
fn critical_section_call(e: &mut Engine, this: Ptr<HkpWorld>, import_slot: u32) {
    let section = e.get(this, HkpWorld::m_modifyConstraintCriticalSection);
    if !section.is_null() {
        e.call(import_slot, &args![section]);
    }
}

// Translated from 00c911d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::lockIslandForConstraintUpdate` (Xbox PDB): enters the world's
/// `m_modifyConstraintCriticalSection` (+0xb8) when it exists.
pub fn hkp_world_lock_island_for_constraint_update(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    _unused_2: u32,
) {
    critical_section_call(e, this, ENTER_CRITICAL_SECTION);
}

// Translated from 00c911f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed twin of `00c911d0` (same body): enters the critical section at +0xb8 when it
/// exists.
pub fn fn_00c911f0(e: &mut Engine, this: Ptr<HkpWorld>, _unused_2: u32) {
    critical_section_call(e, this, ENTER_CRITICAL_SECTION);
}

// Translated from 00c91210 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::unlockIslandForConstraintUpdate` (Xbox PDB): leaves the critical section at
/// +0xb8 when it exists.
pub fn hkp_world_unlock_island_for_constraint_update(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    _unused_2: u32,
) {
    critical_section_call(e, this, LEAVE_CRITICAL_SECTION);
}

// Translated from 00c91230 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed twin of `00c91210` (same body): leaves the critical section at +0xb8 when it
/// exists.
pub fn fn_00c91230(e: &mut Engine, this: Ptr<HkpWorld>, _unused_2: u32) {
    critical_section_call(e, this, LEAVE_CRITICAL_SECTION);
}

// Translated from 00c91250 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. When the world has a kd-tree manager (+0x5c), calls its vtable slot +0x14 with 0.
pub fn fn_00c91250(e: &mut Engine, this: Ptr<HkpWorld>) {
    let tree = e.get(this, HkpWorld::m_kdTreeManager);
    if !tree.is_null() {
        e.vcall(tree.addr(), 0x14, &args![0u32]);
    }
}

// Translated from 00c91270 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Tail-calls the kd-tree manager's vtable slot +0xc with two arguments (no null
/// check).
pub fn fn_00c91270(e: &mut Engine, this: Ptr<HkpWorld>, first: u32, second: u32) {
    let tree = e.get(this, HkpWorld::m_kdTreeManager);
    e.vcall(tree.addr(), 0xc, &args![first, second]);
}

// Translated from 00c912e0 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c912e0 - takes EAX (the world) and EDI (a registry object) as register inputs, which have no uniform-ABI form; translated as a Rust function with them as parameters and not registered.
/// Unnamed fragment the compiler outlined from the `hkpWorld` constructor (`EAX` = the
/// world, `EDI` = an object whose `00cfb240` takes `(object, kind)`). Makes three small
/// helper objects: two of 0xC bytes built by `00cfd450(world)` and `00cfca20(world)`, and one
/// of 0x14 bytes built inline (vtable `010c6420`, +8 = `010c6378`, +0x10 = 3); registers them
/// with `registry` as kinds 1, 2 and 3 and drops the creator's reference on each.
pub fn fn_00c912e0(e: &mut Engine, world: Ptr, registry: Ptr) {
    let first = router_alloc(e, 0xc);
    e.mem.set_u16(first + 4, 0xc);
    let first = e.call(MAKE_HELPER_FIRST, &args![first, world]).u32();
    let second = router_alloc(e, 0xc);
    e.mem.set_u16(second + 4, 0xc);
    let second = e.call(MAKE_HELPER_SECOND, &args![second, world]).u32();
    let third = router_alloc(e, 0x14);
    e.mem.set_u16(third + 4, 0x14);
    e.mem.set_u16(third + 6, 1);
    e.mem.set_u32(third, 0x010c_6420);
    e.mem.set_u16(third + 0xe, 1);
    e.mem.set_u32(third + 0x10, 3);
    e.mem.set_u32(third + 8, 0x010c_6378);
    e.call(REGISTER_HELPER, &args![registry, first, 1u32]);
    e.call(REGISTER_HELPER, &args![registry, second, 2u32]);
    e.call(REGISTER_HELPER, &args![registry, third, 3u32]);
    e.call(REMOVE_REFERENCE, &args![first]);
    e.call(REMOVE_REFERENCE, &args![second]);
    e.call(REMOVE_REFERENCE, &args![third]);
}

// Translated from 00c913b0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::reenableCollisionBetweenEntityPair` (Xbox PDB). While critical operations are
/// locked (+0x94 non-zero) the request is queued as operation 0x13 (the two entities at +4
/// and +8). Otherwise, when the broad phase (slot +0x4c) accepts the two broad-phase handles
/// (entity +0x24), the collision filter (+0x7c, used as its base at +8; not null-checked)
/// enables the two collidables (entity +0x10), and no agent exists yet between them
/// (`00cfdd70`), the pair is added through the broad-phase dispatcher (`00cf7080`).
pub fn hkp_world_reenable_collision_between_entity_pair(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    entity_a: Ptr,
    entity_b: Ptr,
) {
    if e.get(this, HkpWorld::m_criticalOperationsLockCount) != 0 {
        e.with_stack(0x10, |e, operation| {
            e.mem.set_u8(operation.addr(), 0x13);
            e.mem.set_u32(operation.addr() + 4, entity_a.addr());
            e.mem.set_u32(operation.addr() + 8, entity_b.addr());
            fn_00c91160(e, this, operation);
        });
        return;
    }
    let handle_a = entity_a.addr() + ENTITY_BROADPHASE_HANDLE;
    let handle_b = entity_b.addr() + ENTITY_BROADPHASE_HANDLE;
    let broad_phase = e.get(this, HkpWorld::m_broadPhase);
    if !e
        .vcall(broad_phase.addr(), 0x4c, &args![handle_a, handle_b])
        .bool()
    {
        return;
    }
    let filter = e.get(this, HkpWorld::m_collisionFilter).addr();
    let collidable_a = entity_a.addr() + ENTITY_COLLIDABLE;
    let collidable_b = entity_b.addr() + ENTITY_COLLIDABLE;
    // The filter's base at +8 answers "enabled" through a result pointer (slot +4).
    let enabled = e.with_stack(8, |e, result| {
        let base = filter.wrapping_add(8);
        let vtable = e.mem.u32(base);
        let method = e.mem.u32(vtable + 4);
        let returned = e
            .call(method, &args![base, result, collidable_a, collidable_b])
            .u32();
        e.mem.u8(returned) != 0
    });
    if !enabled {
        return;
    }
    if e.call(FIND_AGENT, &args![collidable_a, collidable_b]).u32() != 0 {
        return;
    }
    let filter_base = if filter == 0 { 0 } else { filter + 8 };
    let dispatcher = e.get(this, HkpWorld::m_broadPhaseDispatcher);
    e.with_stack(8, |e, pair| {
        e.mem.set_u32(pair.addr(), handle_a);
        e.mem.set_u32(pair.addr() + 4, handle_b);
        e.call(ADD_PAIRS, &args![dispatcher, pair, 1u32, filter_base]);
    });
}

// Translated from 00c91470 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. While critical operations are locked, queues operation 0x17 (first argument at
/// +4, the low word of the second at +8, the low byte of the third at +0xA; the rest of the
/// record is uninitialized stack in the original and zero here). Otherwise calls the
/// simulation's vtable slot +0x30 with `(first, second, world, third)`.
pub fn fn_00c91470(e: &mut Engine, this: Ptr<HkpWorld>, first: u32, second: u32, third: u32) {
    if e.get(this, HkpWorld::m_criticalOperationsLockCount) != 0 {
        e.with_stack(0x10, |e, operation| {
            e.mem.set_u8(operation.addr(), 0x17);
            e.mem.set_u32(operation.addr() + 4, first);
            e.mem.set_u16(operation.addr() + 8, second as u16);
            e.mem.set_u8(operation.addr() + 0xa, third as u8);
            fn_00c91160(e, this, operation);
        });
        return;
    }
    let simulation = e.get(this, HkpWorld::m_simulation);
    e.vcall(simulation.addr(), 0x30, &args![first, second, this, third]);
}

/// What `addEntity` and `removeEntity` do once the outermost critical section ends: unless
/// executing pending operations is blocked (+0x9c), flush the pending operations when there
/// are some and the body operations when there are some and one queue is in use.
fn leave_critical_section(e: &mut Engine, this: Ptr<HkpWorld>) {
    if e.get(this, HkpWorld::m_blockExecutingPendingOperations) {
        return;
    }
    if e.get(this, HkpWorld::m_pendingOperationsCount) != 0 {
        fn_00c91120(e, this);
    }
    if e.get(this, HkpWorld::m_pendingOperationQueueCount) == 1
        && e.get(this, HkpWorld::m_pendingBodyOperationsCount) != 0
    {
        fn_00c91140(e, this);
    }
}

// Translated from 00c914d0 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::addEntity` (Xbox PDB). While critical operations are locked the request is
/// queued (operation 1: entity at +4, second argument at +8) and 0 is returned. Otherwise:
/// caches the shape data, makes sure the collidable has its motion (entity vtable slot
/// +0x10), resets the swept transform's time information, clears two words, gives the entity
/// the next uid (`m_lastEntityUid`), adds a reference, adds it to the simulation islands
/// (`00cb4640`), updates the motion's filter bits from the world's bytes at +0x305..+0x307
/// (`00c90ed0`), then inside the critical section adds it to the broad phase and fires the
/// world and entity "added" callbacks. When the outermost critical section ends the pending
/// operations are flushed. Returns the entity.
pub fn hkp_world_add_entity(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    entity: Ptr<HkpEntity>,
    second: u32,
) -> Ptr<HkpEntity> {
    if e.get(this, HkpWorld::m_criticalOperationsLockCount) != 0 {
        e.with_stack(0x10, |e, operation| {
            e.mem.set_u8(operation.addr(), 1);
            e.mem.set_u32(operation.addr() + 4, entity.addr());
            e.mem.set_u32(operation.addr() + 8, second);
            fn_00c91160(e, this, operation);
        });
        return Ptr::NULL;
    }
    let shape = e.get(entity, HkpEntity::m_collidableShape);
    e.call(SET_CACHED_SHAPE_DATA, &args![entity, this, shape]);
    if e.get(entity, HkpEntity::m_collidableMotion).is_null() {
        let motion = e.vcall(entity.addr(), 0x10, &args![]).ptr::<()>();
        e.set(entity, HkpEntity::m_collidableMotion, motion);
    }
    e.call(
        SET_TIME_INFORMATION,
        &args![0.0f32, 0.0f32, entity.addr() + ENTITY_SWEPT_TRANSFORM],
    );
    e.set(entity, HkpEntity::m_wordEa, 0);
    e.set(entity, HkpEntity::m_wordEc, 0);
    e.set(this, HkpWorld::m_criticalOperationsAllowed, false);
    let uid = e.get(this, HkpWorld::m_lastEntityUid).wrapping_add(1);
    e.set(this, HkpWorld::m_lastEntityUid, uid);
    e.set(entity, HkpEntity::m_uid, uid);
    e.call(ADD_REFERENCE, &args![entity]);
    e.call(ADD_ENTITY_SI, &args![this, entity, second]);
    let world = this.addr();
    let byte_305 = e.mem.u8(world + DYNAMICS_BYTES_305) as u32;
    let byte_306 = e.mem.u8(world + DYNAMICS_BYTES_305 + 1) as u32;
    let byte_307 = e.mem.u8(world + DYNAMICS_BYTES_305 + 2) as i32;
    fn_00c90ed0(
        e,
        Ptr::new(entity.addr() + ENTITY_MOTION),
        byte_305,
        byte_306,
        byte_307,
    );
    let lock = e.get(this, HkpWorld::m_criticalOperationsLockCount);
    e.set(
        this,
        HkpWorld::m_criticalOperationsLockCount,
        lock.wrapping_add(1),
    );
    e.set(this, HkpWorld::m_criticalOperationsAllowed, true);
    e.call(ADD_ENTITY_BP, &args![this, entity]);
    e.call(FIRE_WORLD_ENTITY_ADDED, &args![this, entity]);
    e.call(FIRE_ENTITY_ADDED, &args![entity]);
    let lock = e
        .get(this, HkpWorld::m_criticalOperationsLockCount)
        .wrapping_sub(1);
    e.set(this, HkpWorld::m_criticalOperationsLockCount, lock);
    if lock == 0 {
        leave_critical_section(e, this);
    }
    entity
}

// Translated from 00c91620 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::removeEntity` (Xbox PDB), with the `hkBool*` result as the first stack
/// argument. While critical operations are locked the request is queued (operation 2, entity
/// at +4) and `*result` is 0. Otherwise the critical section is entered; the entity is
/// taken out of the simulation islands (`00cb5c10`), the world and entity "removed"
/// callbacks fire, the broad phase drops it (`00cb4740`); a bounding-volume pointer at +0x54
/// is released (`00d01a80` on +0x30); an entity whose `m_memSizeAndFlags` is 0 is deleted
/// through its vtable slot +0x14; its reference is removed. When the outermost critical
/// section ends the pending operations are flushed. `*result` is 1.
pub fn hkp_world_remove_entity(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    result: Ptr,
    entity: Ptr<HkpEntity>,
) {
    if e.get(this, HkpWorld::m_criticalOperationsLockCount) != 0 {
        e.with_stack(0x10, |e, operation| {
            e.mem.set_u8(operation.addr(), 2);
            e.mem.set_u32(operation.addr() + 4, entity.addr());
            fn_00c91160(e, this, operation);
        });
        e.mem.set_u8(result.addr(), 0);
        return;
    }
    e.set(this, HkpWorld::m_criticalOperationsLockCount, 1);
    e.call(REMOVE_ENTITY_SI, &args![this, entity]);
    e.call(FIRE_WORLD_ENTITY_REMOVED, &args![this, entity]);
    e.call(FIRE_ENTITY_REMOVED, &args![entity]);
    e.set(this, HkpWorld::m_criticalOperationsAllowed, false);
    e.call(REMOVE_ENTITY_BP, &args![this, entity]);
    if !e
        .get(entity, HkpEntity::m_boundingVolumeChildData)
        .is_null()
    {
        e.call(
            ENTITY_RELEASE_BOUNDING_DATA,
            &args![entity.addr() + ENTITY_BOUNDING_DATA],
        );
    }
    if e.get(entity, HkpEntity::m_memSizeAndFlags) == 0 {
        e.vcall(entity.addr(), 0x14, &args![]);
    }
    e.call(REMOVE_REFERENCE, &args![entity]);
    let lock = e
        .get(this, HkpWorld::m_criticalOperationsLockCount)
        .wrapping_sub(1);
    e.set(this, HkpWorld::m_criticalOperationsLockCount, lock);
    e.set(this, HkpWorld::m_criticalOperationsAllowed, true);
    if lock == 0 {
        leave_critical_section(e, this);
    }
    e.mem.set_u8(result.addr(), 1);
}

/// Nulls the first entry equal to `listener` in the `hkArray` (data pointer at `data_offset`,
/// size at `data_offset + 4`) of the world; when no entry matches (or the array is empty),
/// the word before the array is written (the original indexes with -1, and so do we).
fn null_listener(e: &mut Engine, this: Ptr<HkpWorld>, data_offset: u32, listener: u32) {
    let world = this.addr();
    let size = e.mem.i32(world + data_offset + 4);
    if size > 0 {
        let mut data = e.mem.u32(world + data_offset);
        for index in 0..size {
            if e.mem.u32(data) == listener {
                let data = e.mem.u32(world + data_offset);
                e.mem
                    .set_u32(data.wrapping_add((index as u32).wrapping_mul(4)), 0);
                return;
            }
            data = data.wrapping_add(4);
        }
    }
    let data = e.mem.u32(world + data_offset);
    e.mem.set_u32(data.wrapping_sub(4), 0);
}

// Translated from 00c91700 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Nulls the entry equal to `listener` in the array at +0x11c (`m_actionListeners`
/// in the Xbox PDB layout); see `null_listener` for the no-match case.
pub fn fn_00c91700(e: &mut Engine, this: Ptr<HkpWorld>, listener: u32) {
    null_listener(e, this, 0x11c, listener);
}

// Translated from 00c91750 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. As `00c91700` on the array at +0x140 (`m_constraintListeners` in the Xbox PDB
/// layout).
pub fn fn_00c91750(e: &mut Engine, this: Ptr<HkpWorld>, listener: u32) {
    null_listener(e, this, 0x140, listener);
}

// Translated from 00c917a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. As `00c91700` on the array at +0x128 (`m_entityListeners` in the Xbox PDB layout).
pub fn fn_00c917a0(e: &mut Engine, this: Ptr<HkpWorld>, listener: u32) {
    null_listener(e, this, 0x128, listener);
}

// Translated from 00c917f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. As `00c91700` on the array at +0x134 (`m_phantomListeners` in the Xbox PDB
/// layout).
pub fn fn_00c917f0(e: &mut Engine, this: Ptr<HkpWorld>, listener: u32) {
    null_listener(e, this, 0x134, listener);
}

// Translated from 00c91840 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. As `00c91700` on the array at +0x158 (`m_islandActivationListeners` in the Xbox
/// PDB layout).
pub fn fn_00c91840(e: &mut Engine, this: Ptr<HkpWorld>, listener: u32) {
    null_listener(e, this, 0x158, listener);
}

// Translated from 00c918e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. As `00c91700` on the array at +0x164 (`m_worldPostSimulationListeners` in the
/// Xbox PDB layout).
pub fn fn_00c918e0(e: &mut Engine, this: Ptr<HkpWorld>, listener: u32) {
    null_listener(e, this, 0x164, listener);
}

// Translated from 00c91a20 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. As `00c91700` on the array at +0x1a0 (`m_contactListeners` in the Xbox PDB
/// layout).
pub fn fn_00c91a20(e: &mut Engine, this: Ptr<HkpWorld>, listener: u32) {
    null_listener(e, this, 0x1a0, listener);
}

// Translated from 00c91a70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. As `00c91700` on the array at +0x14c (`m_worldDeletionListeners` in the Xbox PDB
/// layout).
pub fn fn_00c91a70(e: &mut Engine, this: Ptr<HkpWorld>, listener: u32) {
    null_listener(e, this, 0x14c, listener);
}

// Translated from 00c91b10 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::stepDeltaTime` (Xbox PDB). Locks the world (`00c91180`), lets the memory
/// watch-dog check (`00d01d20`), steps the simulation (vtable slot +0xc, the time through
/// `FLD`/`FSTP`); a kd-tree manager is told next (`00c91270(0, 0)` when `m_autoUpdateTree`,
/// else `00c91250`), then the world unlocks (`00c91190`). Returns the simulation's result.
pub fn hkp_world_step_delta_time(e: &mut Engine, this: Ptr<HkpWorld>, delta_time: f32) -> u32 {
    fn_00c91180(e, this);
    e.call(WATCH_HEAP_MEMORY, &args![this]);
    let simulation = e.get(this, HkpWorld::m_simulation);
    let result = e
        .vcall(simulation.addr(), 0xc, &args![x87_quiet(delta_time)])
        .u32();
    if !e.get(this, HkpWorld::m_kdTreeManager).is_null() {
        if e.get(this, HkpWorld::m_autoUpdateTree) {
            fn_00c91270(e, this, 0, 0);
        } else {
            fn_00c91250(e, this);
        }
    }
    fn_00c91190(e, this);
    result
}

// Translated from 00c91b70 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Locks the world, calls the simulation's vtable slot +0x10 with a `float` (through
/// `FLD`/`FSTP`), unlocks and returns the result.
pub fn fn_00c91b70(e: &mut Engine, this: Ptr<HkpWorld>, time: f32) -> u32 {
    fn_00c91180(e, this);
    let simulation = e.get(this, HkpWorld::m_simulation);
    let result = e
        .vcall(simulation.addr(), 0x10, &args![x87_quiet(time)])
        .u32();
    fn_00c91190(e, this);
    result
}

// Translated from 00c91ba0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Locks the world, calls the simulation's vtable slot +0x14, unlocks and returns
/// the result.
pub fn fn_00c91ba0(e: &mut Engine, this: Ptr<HkpWorld>) -> u32 {
    fn_00c91180(e, this);
    let simulation = e.get(this, HkpWorld::m_simulation);
    let result = e.vcall(simulation.addr(), 0x14, &args![]).u32();
    fn_00c91190(e, this);
    result
}

// Translated from 00c91bd0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. As `00c91ba0` with the simulation's vtable slot +0x18.
pub fn fn_00c91bd0(e: &mut Engine, this: Ptr<HkpWorld>) -> u32 {
    fn_00c91180(e, this);
    let simulation = e.get(this, HkpWorld::m_simulation);
    let result = e.vcall(simulation.addr(), 0x18, &args![]).u32();
    fn_00c91190(e, this);
    result
}

// Translated from 00c91c00 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::finishMtStep` (Xbox PDB): finishes the simulation's multithreaded step
/// (`00d03290(simulation, a, b)`); a non-zero result is returned at once. Otherwise a
/// kd-tree manager is told (`00c91270(a, b)` when `m_autoUpdateTree`, else `00c91250`) and 0
/// is returned.
pub fn hkp_world_finish_mt_step(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    first: u32,
    second: u32,
) -> u32 {
    let simulation = e.get(this, HkpWorld::m_simulation);
    let result = e
        .call(SIMULATION_FINISH_MT_STEP, &args![simulation, first, second])
        .u32();
    if result != 0 {
        return result;
    }
    if !e.get(this, HkpWorld::m_kdTreeManager).is_null() {
        if e.get(this, HkpWorld::m_autoUpdateTree) {
            fn_00c91270(e, this, first, second);
        } else {
            fn_00c91250(e, this);
        }
    }
    0
}

/// Copies the reference-counted pointer `new` into the word at `slot`: a reference is added to
/// the new object, one removed from the one replaced.
fn replace_reference(e: &mut Engine, slot: u32, new: u32) {
    if new != 0 {
        e.call(ADD_REFERENCE, &args![new]);
    }
    let replaced = e.mem.u32(slot);
    if replaced != 0 {
        e.call(REMOVE_REFERENCE, &args![replaced]);
    }
    e.mem.set_u32(slot, new);
}

// Translated from 00c91c50 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Fills the structure at `info` from the world's settings (the layout looks like
/// the world construction info; unconfirmed): vectors and words copied as they are, `float`s
/// through `FLD`/`FSTP`, the collision filter (+0x7c, into +0x58), the convex-list filter
/// (+0x84, into +0x5c) and the memory watch-dog (+0x50, into +0x6c) with a reference added
/// to each new one and removed from each one replaced, the border's two bytes (+0x30, +0x34;
/// 3 and 0 without a border), `info+0x50` = kd-tree manager present, and
/// `info+0x88` = the square root (`00ec6040`) of the step info float at +0x308 (the float at
/// `info+0x90` is written twice, the second value, from +0xd0, stays).
pub fn fn_00c91c50(e: &mut Engine, this: Ptr<HkpWorld>, info: Ptr) {
    let world = this.addr();
    let info = info.addr();
    copy_vector(e, world + 0x10, info + 0x10);
    copy_word(e, world + 0x338, info + 0x20);
    copy_vector(e, world + 0x310, info + 0x30);
    copy_vector(e, world + 0x320, info + 0x40);
    let collision_input = e.get(this, HkpWorld::m_collisionInput).addr();
    copy_float(e, collision_input + 8, info + 0x54);
    let filter = e.get(this, HkpWorld::m_collisionFilter).addr();
    replace_reference(e, info + 0x58, filter);
    let convex = e.get(this, HkpWorld::m_convexListFilter).addr();
    replace_reference(e, info + 0x5c, convex);
    let border = e.get(this, HkpWorld::m_broadPhaseBorder).addr();
    let border_kind = if border == 0 {
        3
    } else {
        e.mem.u32(border + 0x30) as u8
    };
    e.mem.set_u8(info + 0x28, border_kind);
    let border_flag = if border == 0 {
        0
    } else {
        e.mem.u8(border + 0x34)
    };
    e.mem.set_u8(info + 0x29, border_flag);
    let tree_present = !e.get(this, HkpWorld::m_kdTreeManager).is_null();
    e.mem.set_u8(info + 0x50, tree_present as u8);
    copy_byte(e, world + 0x60, info + 0x53);
    let dispatcher = e.get(this, HkpWorld::m_collisionDispatcher).addr();
    copy_float(e, dispatcher + 0x1e40, info + 0x60);
    copy_float(e, dispatcher + 0x1e44, info + 0x68);
    copy_float(e, world + 0xcc, info + 0x90);
    copy_float(e, world + 0xd0, info + 0x90);
    copy_byte(e, world + 0xd4, info + 0x98);
    let watchdog = e.get(this, HkpWorld::m_memoryWatchDog).addr();
    replace_reference(e, info + 0x6c, watchdog);
    copy_word(e, world + 0x330, info + 0x70);
    copy_word(e, world + 0x334, info + 0x64);
    copy_byte(e, world + 0x340, info + 0x74);
    copy_byte(e, collision_input + 0x6d, info + 0x75);
    copy_float(e, world + DYNAMICS_FLOAT_240, info + 0x24);
    copy_float(e, world + DYNAMICS_FLOAT_1E4, info + 0x78);
    copy_float(e, world + DYNAMICS_FLOAT_1E8, info + 0x7c);
    copy_word(e, world + DYNAMICS_WORD_2F4, info + 0x80);
    copy_word(e, world + DYNAMICS_WORD_2F8, info + 0x84);
    let argument = x87_quiet(e.mem.f32(world + DYNAMICS_FLOAT_308));
    let root = e.call(X87_SQRT, &args![argument as f64]).f32();
    e.mem.set_f32(info + 0x88, x87_quiet(root));
    copy_byte(e, world + DYNAMICS_BYTE_304, info + 0x8c);
    copy_byte(e, world + DYNAMICS_BYTES_305, info + 0xa4);
    copy_byte(e, world + DYNAMICS_BYTES_305 + 1, info + 0xa5);
    copy_byte(e, world + DYNAMICS_BYTES_305 + 2, info + 0xa6);
    // The collision input's pointer at +0x70 leads to a float and a word.
    let input_extra = e.mem.u32(collision_input + 0x70);
    copy_float(e, input_extra, info + 0x9c);
    copy_word(e, input_extra + 4, info + 0xa0);
    copy_byte(e, collision_input + 0x6c, info + 0x99);
    copy_byte(e, world + 0xd6, info + 0xa7);
    copy_float(e, world + 0xdc, info + 0xac);
    copy_word(e, world + 0xe0, info + 0xb0);
    copy_byte(e, world + 0xe4, info + 0xb4);
    copy_word(e, world + 0xe8, info + 0xb8);
    copy_word(e, world + 0xec, info + 0xbc);
    copy_float(e, world + 0xd8, info + 0xa8);
    copy_float(e, world + 0xf4, info + 0xc0);
    copy_float(e, world + 0xf8, info + 0xc4);
    copy_float(e, world + 0xfc, info + 0xc8);
    copy_float(e, world + 0x100, info + 0xcc);
    copy_byte(e, world + 0xd5, info + 0xd0);
    copy_byte(e, world + 0xf0, info + 0xd1);
    let simulation = e.get(this, HkpWorld::m_simulation).addr();
    copy_float(e, simulation + 0x24, info + 0xdc);
    copy_byte(e, world + 0xc8, info + 0xd2);
    copy_byte(e, world + 0xb0, info + 0xd8);
    copy_byte(e, world + 0xb1, info + 0xd9);
    copy_word(e, world + 0xb4, info + 0xd4);
    let callbacks = !e.get(this, HkpWorld::m_collisionCallbackUtil).is_null();
    e.mem.set_u8(info + 0xe0, callbacks as u8);
}

// Translated from 00c91f70 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::castRay` (Xbox PDB; the collector variant): brackets the work in the profiling
/// records `"TtworldCastRayCollector"` and `"Et"`. With a kd-tree manager in use (+0x5c,
/// slot +0x10) its slot +0x18 gets `(input, collision filter, collector)`; otherwise a
/// ray-caster object (vtable `010c3bd4`, words at +0x40 and +0x44 zero) runs
/// `00cbf860(broad phase, input, filter, 0, collector)`.
pub fn hkp_world_cast_ray(e: &mut Engine, this: Ptr<HkpWorld>, input: u32, collector: u32) {
    profile_record(e, PROFILE_CAST_RAY_COLLECTOR);
    let filter = e.get(this, HkpWorld::m_collisionFilter).addr();
    if kd_tree_in_use(e, this) {
        let tree = e.get(this, HkpWorld::m_kdTreeManager);
        e.vcall(tree.addr(), 0x18, &args![input, filter, collector]);
    } else {
        let broad_phase = e.get(this, HkpWorld::m_broadPhase);
        e.with_stack(0x54, |e, caster| {
            e.mem.set_u32(caster.addr(), RAY_CASTER_VTABLE);
            e.mem.set_u32(caster.addr() + 0x40, 0);
            e.mem.set_u32(caster.addr() + 0x44, 0);
            e.call(
                WORLD_RAY_CASTER_CAST_RAY,
                &args![caster, broad_phase, input, filter, 0u32, collector],
            );
        });
    }
    profile_record(e, PROFILE_END);
}

// Translated from 00c92040 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::castRay_ov2` (Xbox PDB; the plain variant): as `castRay` with the record
/// `"TtworldCastRay"`, the kd-tree manager's slot +0x1c, and a simple ray-caster object
/// (vtable `010c647c`, words at +0x30 and +0x34 zero) running `00cfab30`.
pub fn hkp_world_cast_ray_ov2(e: &mut Engine, this: Ptr<HkpWorld>, input: u32, output: u32) {
    profile_record(e, PROFILE_CAST_RAY);
    let filter = e.get(this, HkpWorld::m_collisionFilter).addr();
    if kd_tree_in_use(e, this) {
        let tree = e.get(this, HkpWorld::m_kdTreeManager);
        e.vcall(tree.addr(), 0x1c, &args![input, filter, output]);
    } else {
        let broad_phase = e.get(this, HkpWorld::m_broadPhase);
        e.with_stack(0x44, |e, caster| {
            e.mem.set_u32(caster.addr(), SIMPLE_RAY_CASTER_VTABLE);
            e.mem.set_u32(caster.addr() + 0x30, 0);
            e.mem.set_u32(caster.addr() + 0x34, 0);
            e.call(
                SIMPLE_RAY_CASTER_CAST_RAY,
                &args![caster, broad_phase, input, filter, 0u32, output],
            );
        });
    }
    profile_record(e, PROFILE_END);
}

/// Tail of the three job dispatchers: waits for the jobs. Calls the notifier object's vtable
/// slot +0xc with `(queue, 0xb)` and slot +0x10, acquires the semaphore, and frees the
/// 0x10-byte completion block.
fn finish_job_dispatch(e: &mut Engine, queue: u32, notifier: u32, semaphore: u32, block: u32) {
    e.vcall(notifier, 0xc, &args![queue, 0xbu32]);
    e.vcall(notifier, 0x10, &args![]);
    e.call(SEMAPHORE_ACQUIRE, &args![semaphore]);
    router_free(e, block, 0x10);
}

/// Where a job keeps what the batching changes.
struct JobBatching {
    /// The job (game memory) and its size in bytes.
    job: u32,
    size: u32,
    /// Offsets of the element count and of the element pointer inside the job.
    count_offset: u32,
    pointer_offset: u32,
    /// Elements per batch, and bytes the element pointer advances per batch.
    batch: i32,
    stride: u32,
}

/// Splits the job into batches and adds each to `queue` (`00d04b90(queue, copy, 1)`): the
/// copy handed to the queue carries the smaller of the remaining count and the batch size;
/// the job's element pointer advances by the stride and its count drops by one batch per
/// round, until nothing is left.
fn add_batched_jobs(e: &mut Engine, queue: u32, batching: &JobBatching) {
    let job = batching.job;
    let mut count = e.mem.i32(job + batching.count_offset);
    if count <= 0 {
        return;
    }
    e.with_stack(batching.size, |e, copy| loop {
        let bytes = e.mem.bytes(job, batching.size);
        e.mem.write(copy.addr(), &bytes);
        let batch_count = if count < batching.batch {
            count
        } else {
            batching.batch
        };
        e.mem
            .set_i32(copy.addr() + batching.count_offset, batch_count);
        let pointer = e.mem.u32(job + batching.pointer_offset);
        e.mem.set_u32(
            job + batching.pointer_offset,
            pointer.wrapping_add(batching.stride),
        );
        count = count.wrapping_sub(batching.batch);
        e.mem.set_i32(job + batching.count_offset, count);
        e.call(JOB_QUEUE_ADD_JOB, &args![queue, copy, 1u32]);
        if count <= 0 {
            break;
        }
    });
}

// Translated from 00c92110 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Dispatches `count` elements at `array` as batched jobs on `queue`: allocates a
/// 0x10-byte block whose first word is the number of 64-element jobs, builds a 0x50-byte job
/// (bytes 1, 3, 1, size 0x50, id 0xffff, the semaphore, the block, the collision input
/// `+0x78`, 0x40, the array, the count, the broad phase at +0x34; a kd-tree manager in use
/// swaps the broad phase for its words +0xc/+0x10 at +0x3c/+0x40), adds a copy per `batch`
/// elements (`batch * 0x40` bytes further each), then waits through `notifier` and
/// `semaphore`. The job's bytes the original never writes (+3, +8..+0xf, +0x44..+0x4f) are
/// stack garbage there and zero here.
#[allow(clippy::too_many_arguments)]
pub fn fn_00c92110(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    array: u32,
    count: i32,
    queue: u32,
    notifier: u32,
    semaphore: u32,
    batch: i32,
) {
    let block = router_alloc(e, 0x10);
    let collision_input = e.get(this, HkpWorld::m_collisionInput).addr();
    let broad_phase = e.get(this, HkpWorld::m_broadPhase).addr();
    e.mem.set_i32(block, job_block_count(count));
    e.with_stack(0x50, |e, job| {
        let job = job.addr();
        e.mem.set_u8(job, 1);
        e.mem.set_u8(job + 1, 3);
        e.mem.set_u8(job + 2, 1);
        e.mem.set_u16(job + 4, 0x50);
        e.mem.set_u16(job + 6, 0xffff);
        e.mem.set_u32(job + 0x10, semaphore);
        e.mem.set_u32(job + 0x14, block);
        e.mem.set_u32(job + 0x1c, collision_input);
        e.mem.set_u32(job + 0x20, 0x40);
        e.mem.set_u32(job + 0x24, array);
        e.mem.set_u32(job + 0x28, count as u32);
        e.mem.set_u32(job + 0x34, broad_phase);
        let tree = e.get(this, HkpWorld::m_kdTreeManager);
        if !tree.is_null() && e.vcall(tree.addr(), 0x10, &args![]).bool() {
            let first = e.mem.u32(tree.addr() + 0xc);
            let second = e.mem.u32(tree.addr() + 0x10);
            e.mem.set_u32(job + 0x3c, first);
            e.mem.set_u32(job + 0x40, second);
            e.mem.set_u32(job + 0x34, 0);
        }
        let batching = JobBatching {
            job,
            size: 0x50,
            count_offset: 0x28,
            pointer_offset: 0x24,
            batch,
            stride: (batch as u32).wrapping_shl(6),
        };
        add_batched_jobs(e, queue, &batching);
    });
    finish_job_dispatch(e, queue, notifier, semaphore, block);
}

// Translated from 00c92290 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Like `00c92110` with a 0x40-byte job (bytes 2, 2, 2, size 0x40, id 0xffff, the
/// semaphore, the block, the collision input at +0x18, the broad phase at +0x20, 0x40 at
/// +0x30, the array at +0x34, the count at +0x38, and at +0x2c the `float` at +8 of the
/// collision input through `FLD`/`FSTP`) after clearing the word at +0xc of each 0x10-byte
/// element of `array`; the job header is initialized by `00d05de0`; a kd-tree manager in use
/// puts its words +0xc/+0x10 at +0x24/+0x28 and clears +0x20. Batches advance `batch * 0x10`
/// bytes. Bytes the original leaves as stack garbage (+3, +8..+0xf, +0x3c) are zero here.
#[allow(clippy::too_many_arguments)]
pub fn fn_00c92290(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    array: u32,
    count: i32,
    queue: u32,
    notifier: u32,
    semaphore: u32,
    batch: i32,
) {
    let block = router_alloc(e, 0x10);
    let collision_input = e.get(this, HkpWorld::m_collisionInput).addr();
    let tolerance = x87_quiet(e.mem.f32(collision_input + 8));
    let broad_phase = e.get(this, HkpWorld::m_broadPhase).addr();
    for index in 0..count.max(0) as u32 {
        e.mem.set_u32(array.wrapping_add(0xc + 0x10 * index), 0);
    }
    e.mem.set_i32(block, job_block_count(count));
    e.with_stack(0x40, |e, job| {
        let job = job.addr();
        e.mem.set_u8(job, 2);
        e.mem.set_u8(job + 1, 2);
        e.mem.set_u8(job + 2, 2);
        e.mem.set_u16(job + 4, 0x40);
        e.mem.set_u16(job + 6, 0xffff);
        e.mem.set_u32(job + 0x10, semaphore);
        e.mem.set_u32(job + 0x14, block);
        e.mem.set_u32(job + 0x18, collision_input);
        e.mem.set_u32(job + 0x20, broad_phase);
        e.mem.set_f32(job + 0x2c, tolerance);
        e.mem.set_u32(job + 0x30, 0x40);
        e.mem.set_u32(job + 0x34, array);
        e.mem.set_u32(job + 0x38, count as u32);
        e.call(JOB_HEADER_INIT, &args![job]);
        let tree = e.get(this, HkpWorld::m_kdTreeManager);
        if !tree.is_null() && e.vcall(tree.addr(), 0x10, &args![]).bool() {
            let first = e.mem.u32(tree.addr() + 0xc);
            let second = e.mem.u32(tree.addr() + 0x10);
            e.mem.set_u32(job + 0x24, first);
            e.mem.set_u32(job + 0x28, second);
            e.mem.set_u32(job + 0x20, 0);
        }
        let batching = JobBatching {
            job,
            size: 0x40,
            count_offset: 0x38,
            pointer_offset: 0x34,
            batch,
            stride: (batch as u32).wrapping_shl(4),
        };
        add_batched_jobs(e, queue, &batching);
    });
    finish_job_dispatch(e, queue, notifier, semaphore, block);
}

// Translated from 00c92430 (decompiled, FalloutNV.exe 1.4.0.525)
/// `hkpWorld::linearCast` (Xbox PDB): brackets the work in the records `"TtworldLinCast"` and
/// `"Et"`. The collision filter's base (`+7c`, plus 8, or 0) and the collision input's word
/// at +0x70 are passed on. With a kd-tree manager in use its slot +0x20 gets `(a, b, filter
/// base, collision input, input+0x70, c, d)`; otherwise a linear-caster object (vtable
/// `010c6488`, +0x24 = 0, +0x80 = the `float` at `0104da4c`) runs `00cfaed0(broad phase, a,
/// b, filter base, collision input, input+0x70, 0, c, d)`.
pub fn hkp_world_linear_cast(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    first: u32,
    second: u32,
    third: u32,
    fourth: u32,
) {
    profile_record(e, PROFILE_LINEAR_CAST);
    let filter = e.get(this, HkpWorld::m_collisionFilter).addr();
    let filter_base = if filter == 0 { 0 } else { filter + 8 };
    let collision_input = e.get(this, HkpWorld::m_collisionInput).addr();
    if kd_tree_in_use(e, this) {
        let tree = e.get(this, HkpWorld::m_kdTreeManager);
        let extra = e.mem.u32(collision_input + 0x70);
        e.vcall(
            tree.addr(),
            0x20,
            &args![
                first,
                second,
                filter_base,
                collision_input,
                extra,
                third,
                fourth
            ],
        );
    } else {
        let default_value = x87_quiet(e.global::<f32>(LINEAR_CASTER_DEFAULT));
        let broad_phase = e.get(this, HkpWorld::m_broadPhase);
        let extra = e.mem.u32(collision_input + 0x70);
        e.with_stack(0x84, |e, caster| {
            e.mem.set_f32(caster.addr() + 0x80, default_value);
            e.mem.set_u32(caster.addr(), LINEAR_CASTER_VTABLE);
            e.mem.set_u32(caster.addr() + 0x24, 0);
            e.call(
                LINEAR_CASTER_LINEAR_CAST,
                &args![
                    caster,
                    broad_phase,
                    first,
                    second,
                    filter_base,
                    collision_input,
                    extra,
                    0u32,
                    third,
                    fourth
                ],
            );
        });
    }
    profile_record(e, PROFILE_END);
}

// Translated from 00c92540 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Like `00c92110` with a 0x40-byte job (bytes 4, 2, 1, size 0x40, id 0xffff, the
/// semaphore, the block, the collision input at +0x18, 0x40 at +0x20, the array at +0x24,
/// the count at +0x28, the broad phase at +0x2c); a kd-tree manager in use clears +0x2c and
/// puts its words +0xc/+0x10 at +0x34/+0x38. Batches advance `batch * 0x30` bytes. Bytes the
/// original leaves as stack garbage (+3, +8..+0xf, +0x3c) are zero here.
#[allow(clippy::too_many_arguments)]
pub fn fn_00c92540(
    e: &mut Engine,
    this: Ptr<HkpWorld>,
    array: u32,
    count: i32,
    queue: u32,
    notifier: u32,
    semaphore: u32,
    batch: i32,
) {
    let block = router_alloc(e, 0x10);
    let collision_input = e.get(this, HkpWorld::m_collisionInput).addr();
    let broad_phase = e.get(this, HkpWorld::m_broadPhase).addr();
    e.mem.set_i32(block, job_block_count(count));
    e.with_stack(0x40, |e, job| {
        let job = job.addr();
        e.mem.set_u8(job, 4);
        e.mem.set_u8(job + 1, 2);
        e.mem.set_u16(job + 4, 0x40);
        e.mem.set_u16(job + 6, 0xffff);
        e.mem.set_u32(job + 0x10, semaphore);
        e.mem.set_u32(job + 0x14, block);
        e.mem.set_u32(job + 0x18, collision_input);
        e.mem.set_u32(job + 0x20, 0x40);
        e.mem.set_u32(job + 0x24, array);
        e.mem.set_u32(job + 0x28, count as u32);
        e.mem.set_u32(job + 0x2c, broad_phase);
        let tree = e.get(this, HkpWorld::m_kdTreeManager);
        if !tree.is_null() && e.vcall(tree.addr(), 0x10, &args![]).bool() {
            let first = e.mem.u32(tree.addr() + 0xc);
            let second = e.mem.u32(tree.addr() + 0x10);
            e.mem.set_u32(job + 0x2c, 0);
            e.mem.set_u32(job + 0x34, first);
            e.mem.set_u32(job + 0x38, second);
        }
        e.mem.set_u8(job + 2, 1);
        let batching = JobBatching {
            job,
            size: 0x40,
            count_offset: 0x28,
            pointer_offset: 0x24,
            batch,
            stride: (batch as u32).wrapping_mul(0x30),
        };
        add_batched_jobs(e, queue, &batching);
    });
    finish_job_dispatch(e, queue, notifier, semaphore, block);
}

/// Adds every entity of the island's array (data at +0x48, size at +0x4c) to the physics
/// system as a rigid body.
fn add_island_bodies(e: &mut Engine, island: u32, system: u32) {
    let mut index = 0i32;
    while index < e.mem.i32(island + 0x4c) {
        let data = e.mem.u32(island + 0x48);
        let body = array_entry(e, data, index);
        e.call(PHYSICS_SYSTEM_ADD_RIGID_BODY, &args![system, body]);
        index += 1;
    }
}

// Translated from 00c926a0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (cdecl `(out, world, system)`, returns `out`). Adds the rigid bodies of the
/// world's fixed island (+0x20; the first one only when its word at +0x10 is not null) and
/// of every inactive island (+0x34) to the physics system (`00d064a0`); `*out` is 1 when
/// the fixed island had bodies or there are inactive islands.
pub fn fn_00c926a0(e: &mut Engine, out: Ptr, world: Ptr<HkpWorld>, system: Ptr) -> Ptr {
    let fixed = e.get(world, HkpWorld::m_fixedIsland).addr();
    let mut fixed_had_bodies = false;
    if fixed != 0 {
        let mut index = 0i32;
        while index < e.mem.i32(fixed + 0x4c) {
            let data = e.mem.u32(fixed + 0x48);
            let body = array_entry(e, data, index);
            let first_body = e.mem.u32(data);
            if index != 0 || e.mem.u32(first_body + 0x10) != 0 {
                e.call(PHYSICS_SYSTEM_ADD_RIGID_BODY, &args![system, body]);
            }
            index += 1;
        }
        fixed_had_bodies = e.mem.i32(fixed + 0x4c) > 0;
    }
    let mut cursor = e.get(world, HkpWorld::m_inactiveSimulationIslands).addr();
    loop {
        let data = e.get(world, HkpWorld::m_inactiveSimulationIslands).addr();
        let size = e.get(world, HkpWorld::m_inactiveSimulationIslandsSize);
        if cursor == data.wrapping_add((size as u32).wrapping_mul(4)) {
            break;
        }
        let island = e.mem.u32(cursor);
        add_island_bodies(e, island, system.addr());
        cursor = cursor.wrapping_add(4);
    }
    let any = e.get(world, HkpWorld::m_inactiveSimulationIslandsSize) > 0 || fixed_had_bodies;
    e.mem.set_u8(out.addr(), any as u8);
    out
}

// Translated from 00c92760 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (cdecl `(out, world, system)`, returns `out`). Adds the rigid bodies of every
/// active island (+0x28) of the world to the physics system; `*out` is 1 when there are
/// active islands.
pub fn fn_00c92760(e: &mut Engine, out: Ptr, world: Ptr<HkpWorld>, system: Ptr) -> Ptr {
    let mut cursor = e.get(world, HkpWorld::m_activeSimulationIslands).addr();
    loop {
        let data = e.get(world, HkpWorld::m_activeSimulationIslands).addr();
        let size = e.get(world, HkpWorld::m_activeSimulationIslandsSize);
        if cursor == data.wrapping_add((size as u32).wrapping_mul(4)) {
            break;
        }
        let island = e.mem.u32(cursor);
        add_island_bodies(e, island, system.addr());
        cursor = cursor.wrapping_add(4);
    }
    let any = e.get(world, HkpWorld::m_activeSimulationIslandsSize) > 0;
    e.mem.set_u8(out.addr(), any as u8);
    out
}

// Translated from 00c927c0 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c927c0 - takes ESI (the physics system) and EDI (the world) as register inputs, which have no uniform-ABI form; translated as a Rust function with them as parameters and not registered.
/// Unnamed fragment outlined from `00c929e0`: runs `00c926a0` and `00c92760` on the world
/// into a local flag, then stores the second flag (active islands present) in the byte at
/// +0x40 of the physics system.
pub fn fn_00c927c0(e: &mut Engine, system: Ptr, world: Ptr<HkpWorld>) {
    e.with_stack(8, |e, flag| {
        fn_00c926a0(e, flag, world, system);
        let flag = fn_00c92760(e, flag, world, system);
        let value = e.mem.u8(flag.addr());
        e.mem.set_u8(system.addr() + 0x40, value);
    });
}

// Translated from 00c927f0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (cdecl `(island, system)`). For every entity of the island (array at +0x48, size
/// +0x4c) and each of the entity's constraint entries (an array of 0x30-byte entries: data
/// at +0xac, count as a word at +0xb0): follows the entry's pointer at +0xc through the
/// chain of words at +0x14 while the entry's leading word is at least 0x17, and adds the
/// entry's first word to the physics system (`00d06540`) unless the chain ends in a leading
/// word of 0x16.
pub fn fn_00c927f0(e: &mut Engine, island: Ptr, system: Ptr) {
    let island = island.addr();
    let mut entity_index = 0i32;
    while entity_index < e.mem.i32(island + 0x4c) {
        let entities = e.mem.u32(island + 0x48);
        let entity = array_entry(e, entities, entity_index);
        let mut constraint = 0i32;
        while constraint < e.mem.u16(entity + 0xb0) as i32 {
            let data = e.mem.u32(entity + 0xac);
            let entry = data.wrapping_add(0x30 * constraint as u32);
            let mut chain = e.mem.u32(entry + 0xc);
            while e.mem.u16(chain) >= 0x17 {
                chain = e.mem.u32(chain + 0x14);
            }
            if e.mem.u16(chain) != 0x16 {
                let first = e.mem.u32(entry);
                e.call(PHYSICS_SYSTEM_ADD_CONSTRAINT, &args![system, first]);
            }
            constraint += 1;
        }
        entity_index += 1;
    }
}

// Translated from 00c92860 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c92860 - takes ESI (the world) as a register input, which has no uniform-ABI form; translated as a Rust function with it as the first parameter and not registered.
/// Unnamed fragment outlined from `00c929e0` (`ESI` = the world, the physics system on the
/// stack): runs `00c927f0` for every active island (+0x28) and then every inactive island
/// (+0x34).
pub fn fn_00c92860(e: &mut Engine, world: Ptr<HkpWorld>, system: Ptr) {
    let mut cursor = e.get(world, HkpWorld::m_activeSimulationIslands).addr();
    loop {
        let data = e.get(world, HkpWorld::m_activeSimulationIslands).addr();
        let size = e.get(world, HkpWorld::m_activeSimulationIslandsSize);
        if cursor == data.wrapping_add((size as u32).wrapping_mul(4)) {
            break;
        }
        let island = e.mem.u32(cursor);
        fn_00c927f0(e, Ptr::new(island), system);
        cursor = cursor.wrapping_add(4);
    }
    let mut cursor = e.get(world, HkpWorld::m_inactiveSimulationIslands).addr();
    loop {
        let data = e.get(world, HkpWorld::m_inactiveSimulationIslands).addr();
        let size = e.get(world, HkpWorld::m_inactiveSimulationIslandsSize);
        if cursor == data.wrapping_add((size as u32).wrapping_mul(4)) {
            break;
        }
        let island = e.mem.u32(cursor);
        fn_00c927f0(e, Ptr::new(island), system);
        cursor = cursor.wrapping_add(4);
    }
}

/// Adds every action of the island's action array (data at +0x38, size at +0x3c) to the
/// physics system (`00d06590`).
fn add_island_actions(e: &mut Engine, island: u32, system: u32) {
    let mut index = 0i32;
    while index < e.mem.i32(island + 0x3c) {
        let data = e.mem.u32(island + 0x38);
        let action = array_entry(e, data, index);
        e.call(PHYSICS_SYSTEM_ADD_ACTION, &args![system, action]);
        index += 1;
    }
}

// Translated from 00c928c0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed (cdecl `(world, system)`). Adds the actions of every active island (+0x28) and
/// then of every inactive island (+0x34) to the physics system.
pub fn fn_00c928c0(e: &mut Engine, world: Ptr<HkpWorld>, system: Ptr) {
    let mut cursor = e.get(world, HkpWorld::m_activeSimulationIslands).addr();
    loop {
        let data = e.get(world, HkpWorld::m_activeSimulationIslands).addr();
        let size = e.get(world, HkpWorld::m_activeSimulationIslandsSize);
        if cursor == data.wrapping_add((size as u32).wrapping_mul(4)) {
            break;
        }
        let island = e.mem.u32(cursor);
        add_island_actions(e, island, system.addr());
        cursor = cursor.wrapping_add(4);
    }
    let mut cursor = e.get(world, HkpWorld::m_inactiveSimulationIslands).addr();
    loop {
        let data = e.get(world, HkpWorld::m_inactiveSimulationIslands).addr();
        let size = e.get(world, HkpWorld::m_inactiveSimulationIslandsSize);
        if cursor == data.wrapping_add((size as u32).wrapping_mul(4)) {
            break;
        }
        let island = e.mem.u32(cursor);
        add_island_actions(e, island, system.addr());
        cursor = cursor.wrapping_add(4);
    }
}

// Translated from 00c92950 (decompiled, FalloutNV.exe 1.4.0.525)
// Translation incomplete: 00c92950 - takes EDI (the world) as a register input, which has no uniform-ABI form; translated as a Rust function with it as the first parameter and not registered.
/// Unnamed fragment outlined from `00c929e0` (`EDI` = the world, the physics system on the
/// stack): adds the world's phantoms (array at +0x110, size +0x114) to the physics system
/// (`00d064f0`), skipping those equal to one of the six words at +0x18..+0x2c of the broad
/// phase border (+0x1bc) when the world has one.
pub fn fn_00c92950(e: &mut Engine, world: Ptr<HkpWorld>, system: Ptr) {
    let border = fn_00c91110(e, world).addr();
    let mut index = 0i32;
    while index < e.get(world, HkpWorld::m_phantomsSize) {
        let data = e.get(world, HkpWorld::m_phantoms).addr();
        let phantom = array_entry(e, data, index);
        let skipped =
            border != 0 && (0..6).any(|slot| e.mem.u32(border + 0x18 + 4 * slot) == phantom);
        if !skipped {
            e.call(PHYSICS_SYSTEM_ADD_PHANTOM, &args![system, phantom]);
        }
        index += 1;
    }
}

// Translated from 00c929e0 (decompiled, FalloutNV.exe 1.4.0.525)
/// Unnamed. Builds a physics system holding the world's contents: allocates 0x44 bytes
/// through the memory router (word at +4 = 0x44), constructs it (`00d065e0`), then adds the
/// bodies (`00c927c0`), phantoms (`00c92950`), constraints (`00c92860`) and actions
/// (`00c928c0`) of the world. Returns the system.
pub fn fn_00c929e0(e: &mut Engine, this: Ptr<HkpWorld>) -> Ptr {
    let block = router_alloc(e, 0x44);
    e.mem.set_u16(block + 4, 0x44);
    let system = e.call(PHYSICS_SYSTEM_CONSTRUCT, &args![block]).ptr::<()>();
    fn_00c927c0(e, system, this);
    fn_00c92950(e, this, system);
    fn_00c92860(e, this, system);
    fn_00c928c0(e, this, system);
    system
}

/// This unit's translated functions, by exe address.
pub fn funcs() -> Vec<(u32, AbiFn)> {
    vec![
        entry!(
            0x00c9_0e40,
            hkp_contact_mgr_factory_calc_content_statistics(Ptr, Ptr, u32)
        ),
        entry!(0x00c9_0e60, fn_00c90e60(Ptr, f32, f32)),
        entry!(0x00c9_0ed0, fn_00c90ed0(Ptr, u32, u32, i32)),
        entry!(0x00c9_0f30, hkp_multithread_config_hkp_multithread_config(Ptr) -> Ptr),
        entry!(0x00c9_1000, fn_00c91000(Ptr<HkpWorld>, Ptr)),
        entry!(
            0x00c9_1020,
            hkp_world_get_current_time(Ptr<HkpWorld>) -> f32
        ),
        entry!(
            0x00c9_1030,
            hkp_world_get_current_psi_time(Ptr<HkpWorld>) -> f32
        ),
        entry!(0x00c9_1040, fn_00c91040(Ptr<HkpWorld>, f32)),
        entry!(0x00c9_1060, fn_00c91060(Ptr<HkpWorld>) -> u32),
        entry!(0x00c9_10a0, fn_00c910a0(Ptr<HkpWorld>, u32)),
        entry!(0x00c9_10b0, hkp_world_check_determinism(Ptr<HkpWorld>)),
        entry!(0x00c9_10d0, fn_00c910d0(Ptr<HkpWorld>, Ptr)),
        entry!(0x00c9_1110, fn_00c91110(Ptr<HkpWorld>) -> Ptr),
        entry!(0x00c9_1120, fn_00c91120(Ptr<HkpWorld>)),
        entry!(0x00c9_1140, fn_00c91140(Ptr<HkpWorld>)),
        entry!(0x00c9_1160, fn_00c91160(Ptr<HkpWorld>, Ptr)),
        entry!(0x00c9_1180, fn_00c91180(Ptr<HkpWorld>)),
        entry!(0x00c9_1190, fn_00c91190(Ptr<HkpWorld>)),
        entry!(0x00c9_11a0, fn_00c911a0(Ptr<HkpWorld>)),
        entry!(0x00c9_11b0, hkp_world_unlock_read_only(Ptr<HkpWorld>)),
        entry!(
            0x00c9_11d0,
            hkp_world_lock_island_for_constraint_update(Ptr<HkpWorld>, u32)
        ),
        entry!(0x00c9_11f0, fn_00c911f0(Ptr<HkpWorld>, u32)),
        entry!(
            0x00c9_1210,
            hkp_world_unlock_island_for_constraint_update(Ptr<HkpWorld>, u32)
        ),
        entry!(0x00c9_1230, fn_00c91230(Ptr<HkpWorld>, u32)),
        entry!(0x00c9_1250, fn_00c91250(Ptr<HkpWorld>)),
        entry!(0x00c9_1270, fn_00c91270(Ptr<HkpWorld>, u32, u32)),
        entry!(
            0x00c9_13b0,
            hkp_world_reenable_collision_between_entity_pair(Ptr<HkpWorld>, Ptr, Ptr)
        ),
        entry!(0x00c9_1470, fn_00c91470(Ptr<HkpWorld>, u32, u32, u32)),
        entry!(
            0x00c9_14d0,
            hkp_world_add_entity(Ptr<HkpWorld>, Ptr<HkpEntity>, u32) -> Ptr<HkpEntity>
        ),
        entry!(
            0x00c9_1620,
            hkp_world_remove_entity(Ptr<HkpWorld>, Ptr, Ptr<HkpEntity>)
        ),
        entry!(0x00c9_1700, fn_00c91700(Ptr<HkpWorld>, u32)),
        entry!(0x00c9_1750, fn_00c91750(Ptr<HkpWorld>, u32)),
        entry!(0x00c9_17a0, fn_00c917a0(Ptr<HkpWorld>, u32)),
        entry!(0x00c9_17f0, fn_00c917f0(Ptr<HkpWorld>, u32)),
        entry!(0x00c9_1840, fn_00c91840(Ptr<HkpWorld>, u32)),
        entry!(0x00c9_18e0, fn_00c918e0(Ptr<HkpWorld>, u32)),
        entry!(0x00c9_1a20, fn_00c91a20(Ptr<HkpWorld>, u32)),
        entry!(0x00c9_1a70, fn_00c91a70(Ptr<HkpWorld>, u32)),
        entry!(
            0x00c9_1b10,
            hkp_world_step_delta_time(Ptr<HkpWorld>, f32) -> u32
        ),
        entry!(0x00c9_1b70, fn_00c91b70(Ptr<HkpWorld>, f32) -> u32),
        entry!(0x00c9_1ba0, fn_00c91ba0(Ptr<HkpWorld>) -> u32),
        entry!(0x00c9_1bd0, fn_00c91bd0(Ptr<HkpWorld>) -> u32),
        entry!(
            0x00c9_1c00,
            hkp_world_finish_mt_step(Ptr<HkpWorld>, u32, u32) -> u32
        ),
        entry!(0x00c9_1c50, fn_00c91c50(Ptr<HkpWorld>, Ptr)),
        entry!(0x00c9_1f70, hkp_world_cast_ray(Ptr<HkpWorld>, u32, u32)),
        entry!(0x00c9_2040, hkp_world_cast_ray_ov2(Ptr<HkpWorld>, u32, u32)),
        entry!(
            0x00c9_2110,
            fn_00c92110(Ptr<HkpWorld>, u32, i32, u32, u32, u32, i32)
        ),
        entry!(
            0x00c9_2290,
            fn_00c92290(Ptr<HkpWorld>, u32, i32, u32, u32, u32, i32)
        ),
        entry!(
            0x00c9_2430,
            hkp_world_linear_cast(Ptr<HkpWorld>, u32, u32, u32, u32)
        ),
        entry!(
            0x00c9_2540,
            fn_00c92540(Ptr<HkpWorld>, u32, i32, u32, u32, u32, i32)
        ),
        entry!(0x00c9_26a0, fn_00c926a0(Ptr, Ptr<HkpWorld>, Ptr) -> Ptr),
        entry!(0x00c9_2760, fn_00c92760(Ptr, Ptr<HkpWorld>, Ptr) -> Ptr),
        entry!(0x00c9_27f0, fn_00c927f0(Ptr, Ptr)),
        entry!(0x00c9_28c0, fn_00c928c0(Ptr<HkpWorld>, Ptr)),
        entry!(0x00c9_29e0, fn_00c929e0(Ptr<HkpWorld>) -> Ptr),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Addresses for test doubles that stand in for virtual functions.
    const FAKE_1: u32 = 0x0900_0001;
    const FAKE_2: u32 = 0x0900_0002;
    const FAKE_3: u32 = 0x0900_0003;
    const FAKE_4: u32 = 0x0900_0004;
    const FAKE_5: u32 = 0x0900_0005;

    type Records = Rc<RefCell<Vec<Vec<u32>>>>;
    type Operations = Rc<RefCell<Vec<(u32, Vec<u8>)>>>;

    fn logging() -> Engine {
        let mut e = Engine::new();
        e.call_log = Some(vec![]);
        e
    }

    /// The argument words of every logged call to `address`, in order.
    fn calls_to(e: &Engine, address: u32) -> Vec<Vec<u32>> {
        e.call_log
            .as_ref()
            .map(|log| {
                log.iter()
                    .filter(|(called, _)| *called == address)
                    .map(|(_, words)| words.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The addresses of the logged calls that are in `wanted`, in order.
    fn order_of(e: &Engine, wanted: &[u32]) -> Vec<u32> {
        e.call_log
            .as_ref()
            .map(|log| {
                log.iter()
                    .map(|(called, _)| *called)
                    .filter(|called| wanted.contains(called))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Doubles that do nothing and return 0.
    fn silent(e: &mut Engine, addresses: &[u32]) {
        for &address in addresses {
            e.register(address, |_, _| Ret::default());
        }
    }

    fn returning(e: &mut Engine, address: u32, value: u32) {
        e.register_double(address, move |_, _| Ret {
            eax: value,
            ..Ret::default()
        });
    }

    /// A double that stores its argument words.
    fn recording(e: &mut Engine, address: u32) -> Records {
        let records: Records = Rc::new(RefCell::new(vec![]));
        let sink = records.clone();
        e.register_double(address, move |_, words| {
            sink.borrow_mut().push(words.to_vec());
            Ret::default()
        });
        records
    }

    /// An object of `size` bytes whose vtable (also on the heap) holds `(slot, target)`.
    fn object_with_vtable(e: &mut Engine, size: u32, slots: &[(u32, u32)]) -> u32 {
        let table = e.mem.alloc(0x100);
        for &(slot, target) in slots {
            e.mem.set_u32(table + slot, target);
        }
        let object = e.mem.alloc(size);
        e.mem.set_u32(object, table);
        object
    }

    fn world(e: &mut Engine) -> Ptr<HkpWorld> {
        e.new_object()
    }

    /// A heap array of the words; one spare word sits before its data.
    fn array_of(e: &mut Engine, values: &[u32]) -> u32 {
        let block = e.mem.alloc(8 + 4 * values.len() as u32);
        for (index, value) in values.iter().enumerate() {
            e.mem.set_u32(block + 4 + 4 * index as u32, *value);
        }
        block + 4
    }

    /// The 16 bytes of an operation record as the queue functions build them.
    fn op_bytes(kind: u8, word_4: u32, word_8: u32, byte_a: u8) -> Vec<u8> {
        let mut bytes = vec![0u8; 16];
        bytes[0] = kind;
        bytes[4..8].copy_from_slice(&word_4.to_le_bytes());
        bytes[8..12].copy_from_slice(&word_8.to_le_bytes());
        bytes[10] = byte_a;
        bytes
    }

    /// Records `(queue, record bytes)` of every operation handed to the queue.
    fn operation_recorder(e: &mut Engine) -> Operations {
        let records: Operations = Rc::new(RefCell::new(vec![]));
        let sink = records.clone();
        e.register_double(QUEUE_ENQUEUE, move |e, words| {
            sink.borrow_mut()
                .push((words[0], e.mem.bytes(words[1], 16)));
            Ret::default()
        });
        records
    }

    #[test]
    fn contact_mgr_factory_names_its_class_to_the_collector() {
        let mut e = logging();
        silent(&mut e, &[FAKE_1, FAKE_2]);
        let collector = object_with_vtable(&mut e, 0x10, &[(0x18, FAKE_1), (0x1c, FAKE_2)]);
        e.call(0x00c9_0e40, &args![0u32, collector, 0u32]);
        assert_eq!(calls_to(&e, FAKE_1), vec![vec![collector, 0x010c_63a0]]);
        assert_eq!(calls_to(&e, FAKE_2), vec![vec![collector]]);
    }

    #[test]
    fn fn_00c90e60_stores_the_values_and_ratios() {
        let mut e = Engine::new();
        e.map(0x0101_1000, 0x1000);
        e.set_global(HALF, 0.5f64);
        let object = e.mem.alloc(0x60);
        e.call(0x00c9_0e60, &args![object, 4.0f32, 2.0f32]);
        assert_eq!(e.mem.f32(object + 4), 4.0);
        assert_eq!(e.mem.f32(object + 0xc), 2.0);
        assert_eq!(e.mem.f32(object + 8), 2.0);
        assert_eq!(e.mem.f32(object + 0x50), 0.5);
        assert_eq!(e.mem.f32(object + 0x58), 1.0);
        assert_eq!(e.mem.f32(object + 0x54), 2.0);
        assert_eq!(e.mem.f32(object + 0x5c), 1.0);
        for lane in 0..4 {
            assert_eq!(e.mem.f32(object + 0x30 + 4 * lane), 2.0);
            assert_eq!(e.mem.f32(object + 0x40 + 4 * lane), 0.5);
        }
        // A signalling NaN is quieted by the FLD/FSTP pair.
        let signalling = f32::from_bits(0x7f80_0001);
        e.call(0x00c9_0e60, &args![object, signalling, 2.0f32]);
        assert_eq!(e.mem.u32(object + 4), 0x7fc0_0001);
    }

    #[test]
    fn fn_00c90ed0_shifts_the_arguments_into_the_top_bits() {
        let mut e = Engine::new();
        let object = e.mem.alloc(0x20);
        // The byte at +9 is 1: `c = 3` is not below it, so nothing is complemented.
        e.mem.set_u8(object + 9, 1);
        e.mem.set_u16(object + 0xa, 0x1234);
        e.mem.set_u16(object + 0xc, 0xffff);
        e.call(0x00c9_0ed0, &args![object, 1u32, 2u32, 3i32]);
        assert_eq!(e.mem.u16(object + 0xa), 0x1234 | 0x4000);
        assert_eq!(e.mem.u16(object + 0xc), 0x3fff | 0x8000);
        // The byte is 3 and `c = 1` is below it in both comparisons: both are complemented.
        e.mem.set_u8(object + 9, 3);
        e.mem.set_u16(object + 0xa, 0);
        e.mem.set_u16(object + 0xc, 0);
        e.call(0x00c9_0ed0, &args![object, 0u32, 1u32, 1i32]);
        assert_eq!(e.mem.u16(object + 0xa), 0xc000);
        assert_eq!(e.mem.u16(object + 0xc), 0x8000);
    }

    #[test]
    fn multithread_config_constructor_sets_its_word() {
        let mut e = Engine::new();
        let object = e.mem.alloc(8);
        let returned = e.call(0x00c9_0f30, &args![object]).u32();
        assert_eq!(returned, object);
        assert_eq!(e.mem.u32(object), 0x46);
    }

    #[test]
    fn fn_00c91000_copies_a_vector() {
        let mut e = Engine::new();
        let world = world(&mut e);
        let source = e.mem.alloc(0x10);
        for lane in 0..4 {
            e.mem.set_u32(source + 4 * lane, 0x1111_1111 * (lane + 1));
        }
        e.call(0x00c9_1000, &args![world, source]);
        for lane in 0..4 {
            let copied = e.mem.u32(world.addr() + 0x10 + 4 * lane);
            assert_eq!(copied, 0x1111_1111 * (lane + 1));
        }
    }

    #[test]
    fn current_times_come_from_the_simulation() {
        let mut e = Engine::new();
        let world = world(&mut e);
        let simulation = e.mem.alloc(0x40);
        e.set(world, HkpWorld::m_simulation, Ptr::new(simulation));
        e.mem.set_f32(simulation + 0x14, 1.5);
        e.mem.set_f32(simulation + 0x18, 2.5);
        assert_eq!(e.call(0x00c9_1020, &args![world]).f32(), 1.5);
        assert_eq!(e.call(0x00c9_1030, &args![world]).f32(), 2.5);
        // FLD quiets a signalling NaN.
        e.mem.set_u32(simulation + 0x14, 0x7f80_0001);
        let quieted = e.call(0x00c9_1020, &args![world]).f32();
        assert_eq!(quieted.to_bits(), 0x7fc0_0001);
    }

    #[test]
    fn fn_00c91040_passes_the_time_to_the_simulation() {
        let mut e = logging();
        silent(&mut e, &[SIMULATION_SET_TIME]);
        let world = world(&mut e);
        e.set(world, HkpWorld::m_simulation, Ptr::new(0x1234));
        e.call(0x00c9_1040, &args![world, 0.25f32]);
        assert_eq!(
            calls_to(&e, SIMULATION_SET_TIME),
            vec![vec![0x1234, 0.25f32.to_bits()]]
        );
    }

    #[test]
    fn fn_00c91060_returns_the_simulations_answer() {
        let mut e = logging();
        returning(&mut e, SIMULATION_TEST, 0x0102);
        let world = world(&mut e);
        e.set(world, HkpWorld::m_simulation, Ptr::new(0x1234));
        assert_eq!(e.call(0x00c9_1060, &args![world]).u32(), 0x0102);
        assert_eq!(calls_to(&e, SIMULATION_TEST), vec![vec![0x1234]]);
    }

    /// A function that does nothing makes no calls of its own.
    fn assert_does_nothing(address: u32, words: &[u32]) {
        let mut e = logging();
        e.call(address, words);
        assert_eq!(e.call_log.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn fn_00c910a0_does_nothing() {
        assert_does_nothing(0x00c9_10a0, &[0x1000, 5]);
    }

    #[test]
    fn check_determinism_does_nothing() {
        assert_does_nothing(0x00c9_10b0, &[0x1000]);
    }

    #[test]
    fn fn_00c911a0_does_nothing() {
        assert_does_nothing(0x00c9_11a0, &[0x1000]);
    }

    #[test]
    fn unlock_read_only_does_nothing() {
        assert_does_nothing(0x00c9_11b0, &[0x1000]);
    }

    #[test]
    fn fn_00c910d0_swaps_the_border_and_its_references() {
        let mut e = logging();
        silent(&mut e, &[FAKE_1, ADD_REFERENCE, REMOVE_REFERENCE]);
        let world = world(&mut e);
        let old = object_with_vtable(&mut e, 0x40, &[(0x10, FAKE_1)]);
        e.set(world, HkpWorld::m_broadPhaseBorder, Ptr::new(old));
        e.call(0x00c9_10d0, &args![world, 0x5000u32]);
        assert_eq!(e.get(world, HkpWorld::m_broadPhaseBorder).addr(), 0x5000);
        assert_eq!(calls_to(&e, FAKE_1), vec![vec![old]]);
        assert_eq!(calls_to(&e, REMOVE_REFERENCE), vec![vec![old]]);
        assert_eq!(calls_to(&e, ADD_REFERENCE), vec![vec![0x5000]]);
        // Clearing the border releases the old one and adds nothing.
        let second = object_with_vtable(&mut e, 0x40, &[(0x10, FAKE_1)]);
        e.set(world, HkpWorld::m_broadPhaseBorder, Ptr::new(second));
        e.call(0x00c9_10d0, &args![world, 0u32]);
        assert_eq!(e.get(world, HkpWorld::m_broadPhaseBorder).addr(), 0);
        assert_eq!(calls_to(&e, ADD_REFERENCE).len(), 1);
        assert_eq!(
            calls_to(&e, REMOVE_REFERENCE),
            vec![vec![old], vec![second]]
        );
        // With no old border nothing is released.
        e.call(0x00c9_10d0, &args![world, 0x6000u32]);
        assert_eq!(calls_to(&e, REMOVE_REFERENCE).len(), 2);
        assert_eq!(calls_to(&e, ADD_REFERENCE).len(), 2);
    }

    #[test]
    fn fn_00c91110_returns_the_border() {
        let mut e = Engine::new();
        let world = world(&mut e);
        e.set(world, HkpWorld::m_broadPhaseBorder, Ptr::new(0x4321));
        assert_eq!(e.call(0x00c9_1110, &args![world]).u32(), 0x4321);
    }

    #[test]
    fn fn_00c91120_clears_the_count_and_flushes_the_queue() {
        let mut e = logging();
        silent(&mut e, &[QUEUE_FLUSH_PENDING]);
        let world = world(&mut e);
        e.set(world, HkpWorld::m_pendingOperations, Ptr::new(0x5000));
        e.set(world, HkpWorld::m_pendingOperationsCount, 7);
        e.call(0x00c9_1120, &args![world]);
        assert_eq!(e.get(world, HkpWorld::m_pendingOperationsCount), 0);
        assert_eq!(calls_to(&e, QUEUE_FLUSH_PENDING), vec![vec![0x5000]]);
    }

    #[test]
    fn fn_00c91140_clears_the_body_count_and_flushes_the_queue() {
        let mut e = logging();
        silent(&mut e, &[QUEUE_FLUSH_BODY_PENDING]);
        let world = world(&mut e);
        e.set(world, HkpWorld::m_pendingOperations, Ptr::new(0x5000));
        e.set(world, HkpWorld::m_pendingBodyOperationsCount, 7);
        e.call(0x00c9_1140, &args![world]);
        assert_eq!(e.get(world, HkpWorld::m_pendingBodyOperationsCount), 0);
        assert_eq!(calls_to(&e, QUEUE_FLUSH_BODY_PENDING), vec![vec![0x5000]]);
    }

    #[test]
    fn fn_00c91160_enqueues_the_operation() {
        let mut e = logging();
        silent(&mut e, &[QUEUE_ENQUEUE]);
        let world = world(&mut e);
        e.set(world, HkpWorld::m_pendingOperations, Ptr::new(0x5000));
        e.call(0x00c9_1160, &args![world, 0x6000u32]);
        assert_eq!(calls_to(&e, QUEUE_ENQUEUE), vec![vec![0x5000, 0x6000]]);
    }

    #[test]
    fn lock_and_unlock_count_the_world_and_call_the_helpers() {
        let mut e = logging();
        silent(&mut e, &[LOCK_ENTER, LOCK_LEAVE]);
        let world = world(&mut e);
        e.call(0x00c9_1180, &args![world]);
        assert_eq!(e.get(world, HkpWorld::m_isLocked), 1);
        assert_eq!(calls_to(&e, LOCK_ENTER), vec![vec![world.addr()]]);
        e.call(0x00c9_1190, &args![world]);
        assert_eq!(e.get(world, HkpWorld::m_isLocked), 0);
        assert_eq!(calls_to(&e, LOCK_LEAVE), vec![vec![world.addr()]]);
    }

    /// Runs the critical-section function at `address` with and without a section.
    fn check_critical_section(address: u32, import_slot: u32) {
        let mut e = logging();
        silent(&mut e, &[import_slot]);
        let world = world(&mut e);
        e.call(address, &args![world, 0u32]);
        assert!(calls_to(&e, import_slot).is_empty());
        e.set(
            world,
            HkpWorld::m_modifyConstraintCriticalSection,
            Ptr::new(0x7000),
        );
        e.call(address, &args![world, 0u32]);
        assert_eq!(calls_to(&e, import_slot), vec![vec![0x7000]]);
    }

    #[test]
    fn lock_island_for_constraint_update_enters_the_section() {
        check_critical_section(0x00c9_11d0, ENTER_CRITICAL_SECTION);
    }

    #[test]
    fn fn_00c911f0_enters_the_section() {
        check_critical_section(0x00c9_11f0, ENTER_CRITICAL_SECTION);
    }

    #[test]
    fn unlock_island_for_constraint_update_leaves_the_section() {
        check_critical_section(0x00c9_1210, LEAVE_CRITICAL_SECTION);
    }

    #[test]
    fn fn_00c91230_leaves_the_section() {
        check_critical_section(0x00c9_1230, LEAVE_CRITICAL_SECTION);
    }

    #[test]
    fn fn_00c91250_tells_the_tree_when_there_is_one() {
        let mut e = logging();
        silent(&mut e, &[FAKE_1]);
        let world = world(&mut e);
        e.call(0x00c9_1250, &args![world]);
        assert!(calls_to(&e, FAKE_1).is_empty());
        let tree = object_with_vtable(&mut e, 0x20, &[(0x14, FAKE_1)]);
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::new(tree));
        e.call(0x00c9_1250, &args![world]);
        assert_eq!(calls_to(&e, FAKE_1), vec![vec![tree, 0]]);
    }

    #[test]
    fn fn_00c91270_calls_the_trees_slot_with_both_arguments() {
        let mut e = logging();
        silent(&mut e, &[FAKE_1]);
        let world = world(&mut e);
        let tree = object_with_vtable(&mut e, 0x20, &[(0xc, FAKE_1)]);
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::new(tree));
        e.call(0x00c9_1270, &args![world, 5u32, 6u32]);
        assert_eq!(calls_to(&e, FAKE_1), vec![vec![tree, 5, 6]]);
    }

    #[test]
    fn fn_00c912e0_registers_three_helpers() {
        let mut e = logging();
        let blocks: Vec<u32> = (0..3).map(|_| e.mem.alloc(0x20)).collect();
        let next = Rc::new(RefCell::new(blocks.clone().into_iter()));
        e.register_double(FAKE_1, move |_, _| Ret {
            eax: next.borrow_mut().next().unwrap_or(0),
            ..Ret::default()
        });
        let allocator = object_with_vtable(&mut e, 0x20, &[(4, FAKE_1)]);
        let router = e.mem.alloc(0x20);
        e.mem.set_u32(router + 0x10, allocator);
        returning(&mut e, MEMORY_ROUTER, router);
        returning(&mut e, MAKE_HELPER_FIRST, 0xf100);
        returning(&mut e, MAKE_HELPER_SECOND, 0xf200);
        silent(&mut e, &[REGISTER_HELPER, REMOVE_REFERENCE]);
        fn_00c912e0(&mut e, Ptr::new(0x1000), Ptr::new(0x2000));
        assert_eq!(
            calls_to(&e, MAKE_HELPER_FIRST),
            vec![vec![blocks[0], 0x1000]]
        );
        assert_eq!(
            calls_to(&e, MAKE_HELPER_SECOND),
            vec![vec![blocks[1], 0x1000]]
        );
        assert_eq!(e.mem.u16(blocks[0] + 4), 0xc);
        assert_eq!(e.mem.u16(blocks[1] + 4), 0xc);
        let third = blocks[2];
        assert_eq!(e.mem.u16(third + 4), 0x14);
        assert_eq!(e.mem.u16(third + 6), 1);
        assert_eq!(e.mem.u16(third + 0xe), 1);
        assert_eq!(e.mem.u32(third), 0x010c_6420);
        assert_eq!(e.mem.u32(third + 8), 0x010c_6378);
        assert_eq!(e.mem.u32(third + 0x10), 3);
        assert_eq!(
            calls_to(&e, REGISTER_HELPER),
            vec![
                vec![0x2000, 0xf100, 1],
                vec![0x2000, 0xf200, 2],
                vec![0x2000, third, 3]
            ]
        );
        assert_eq!(
            calls_to(&e, REMOVE_REFERENCE),
            vec![vec![0xf100], vec![0xf200], vec![third]]
        );
    }

    /// Runs `reenableCollisionBetweenEntityPair` on a world whose broad phase answers
    /// `accept`, whose filter says `enabled` and whose agent search finds `agent`. Returns
    /// the pairs handed to the dispatcher as `[dispatcher, handle a, handle b, count, filter
    /// base]`, and the two entities and the filter.
    fn reenable_scenario(accept: u32, enabled: u8, agent: u32) -> (Vec<Vec<u32>>, [u32; 3]) {
        let mut e = logging();
        let pairs: Records = Rc::new(RefCell::new(vec![]));
        let sink = pairs.clone();
        e.register_double(ADD_PAIRS, move |e, words| {
            let pair = words[1];
            sink.borrow_mut().push(vec![
                words[0],
                e.mem.u32(pair),
                e.mem.u32(pair + 4),
                words[2],
                words[3],
            ]);
            Ret::default()
        });
        returning(&mut e, FIND_AGENT, agent);
        returning(&mut e, FAKE_3, accept);
        e.register_double(FAKE_2, move |e, words| {
            e.mem.set_u8(words[1], enabled);
            Ret {
                eax: words[1],
                ..Ret::default()
            }
        });
        let world = world(&mut e);
        let broad_phase = object_with_vtable(&mut e, 0x20, &[(0x4c, FAKE_3)]);
        e.set(world, HkpWorld::m_broadPhase, Ptr::new(broad_phase));
        // The filter's base is at +8: its vtable pointer sits there and slot +4 answers.
        let filter = e.mem.alloc(0x40);
        let filter_vtable = e.mem.alloc(0x20);
        e.mem.set_u32(filter + 8, filter_vtable);
        e.mem.set_u32(filter_vtable + 4, FAKE_2);
        e.set(world, HkpWorld::m_collisionFilter, Ptr::new(filter));
        e.set(world, HkpWorld::m_broadPhaseDispatcher, Ptr::new(0x8000));
        let entity_a = e.mem.alloc(0x40);
        let entity_b = e.mem.alloc(0x40);
        e.call(0x00c9_13b0, &args![world, entity_a, entity_b]);
        let recorded = pairs.borrow().clone();
        (recorded, [entity_a, entity_b, filter])
    }

    #[test]
    fn reenable_collision_adds_the_pair_when_everything_allows() {
        let (pairs, [entity_a, entity_b, filter]) = reenable_scenario(1, 1, 0);
        assert_eq!(
            pairs,
            vec![vec![
                0x8000,
                entity_a + 0x24,
                entity_b + 0x24,
                1,
                filter + 8
            ]]
        );
    }

    #[test]
    fn reenable_collision_stops_at_each_refusal() {
        assert!(reenable_scenario(0, 1, 0).0.is_empty());
        assert!(reenable_scenario(1, 0, 0).0.is_empty());
        assert!(reenable_scenario(1, 1, 0x99).0.is_empty());
    }

    #[test]
    fn reenable_collision_is_queued_while_locked() {
        let mut e = logging();
        let queued = operation_recorder(&mut e);
        let world = world(&mut e);
        e.set(world, HkpWorld::m_criticalOperationsLockCount, 1);
        e.set(world, HkpWorld::m_pendingOperations, Ptr::new(0x5000));
        e.call(0x00c9_13b0, &args![world, 0x111u32, 0x222u32]);
        assert_eq!(
            queued.borrow().clone(),
            vec![(0x5000, op_bytes(0x13, 0x111, 0x222, 0))]
        );
    }

    #[test]
    fn fn_00c91470_queues_or_calls_the_simulation() {
        let mut e = logging();
        let queued = operation_recorder(&mut e);
        silent(&mut e, &[FAKE_1]);
        let world = world(&mut e);
        let simulation = object_with_vtable(&mut e, 0x40, &[(0x30, FAKE_1)]);
        e.set(world, HkpWorld::m_simulation, Ptr::new(simulation));
        e.set(world, HkpWorld::m_pendingOperations, Ptr::new(0x5000));
        e.call(0x00c9_1470, &args![world, 1u32, 0x12345u32, 0x1ffu32]);
        assert_eq!(
            calls_to(&e, FAKE_1),
            vec![vec![simulation, 1, 0x12345, world.addr(), 0x1ff]]
        );
        assert!(queued.borrow().is_empty());
        e.set(world, HkpWorld::m_criticalOperationsLockCount, 1);
        e.call(0x00c9_1470, &args![world, 1u32, 0x12345u32, 0x1ffu32]);
        assert_eq!(
            queued.borrow().clone(),
            vec![(0x5000, op_bytes(0x17, 1, 0x2345, 0xff))]
        );
        assert_eq!(calls_to(&e, FAKE_1).len(), 1);
    }

    /// An entity whose vtable slot +0x10 makes a motion and slot +0x14 is the deleter.
    fn entity(e: &mut Engine) -> Ptr<HkpEntity> {
        Ptr::new(object_with_vtable(
            e,
            0x220,
            &[(0x10, FAKE_1), (0x14, FAKE_2)],
        ))
    }

    fn add_entity_engine() -> Engine {
        let mut e = logging();
        silent(
            &mut e,
            &[
                SET_CACHED_SHAPE_DATA,
                SET_TIME_INFORMATION,
                ADD_REFERENCE,
                ADD_ENTITY_SI,
                ADD_ENTITY_BP,
                FIRE_WORLD_ENTITY_ADDED,
                FIRE_ENTITY_ADDED,
                QUEUE_FLUSH_PENDING,
                QUEUE_FLUSH_BODY_PENDING,
                FAKE_2,
            ],
        );
        returning(&mut e, FAKE_1, 0x5555);
        e
    }

    #[test]
    fn add_entity_runs_the_whole_sequence() {
        let mut e = add_entity_engine();
        let world = world(&mut e);
        let entity = entity(&mut e);
        e.set(world, HkpWorld::m_lastEntityUid, 41);
        e.set(world, HkpWorld::m_pendingOperations, Ptr::new(0x5000));
        e.set(world, HkpWorld::m_pendingOperationsCount, 3);
        e.set(world, HkpWorld::m_pendingOperationQueueCount, 1);
        e.set(world, HkpWorld::m_pendingBodyOperationsCount, 2);
        e.mem.set_u8(world.addr() + 0x305, 3);
        e.mem.set_u8(world.addr() + 0x306, 2);
        e.mem.set_u8(world.addr() + 0x307, 1);
        e.mem.set_u8(entity.addr() + 0xe9, 2);
        e.set(entity, HkpEntity::m_collidableShape, Ptr::new(0x777));
        e.set(entity, HkpEntity::m_wordEa, 0x1111);
        e.set(entity, HkpEntity::m_wordEc, 0x2222);
        let returned = e.call(0x00c9_14d0, &args![world, entity, 9u32]).u32();
        assert_eq!(returned, entity.addr());
        assert_eq!(e.get(entity, HkpEntity::m_collidableMotion).addr(), 0x5555);
        assert_eq!(e.get(entity, HkpEntity::m_uid), 42);
        assert_eq!(e.get(world, HkpWorld::m_lastEntityUid), 42);
        assert_eq!(e.get(world, HkpWorld::m_criticalOperationsLockCount), 0);
        assert!(e.get(world, HkpWorld::m_criticalOperationsAllowed));
        // The two words were cleared, then `00c90ed0(3, 2, 1)` on the motion with the byte 2
        // at +9: both arguments are complemented, giving 0 and 0x4000.
        assert_eq!(e.get(entity, HkpEntity::m_wordEa), 0);
        assert_eq!(e.get(entity, HkpEntity::m_wordEc), 0x4000);
        assert_eq!(
            calls_to(&e, SET_CACHED_SHAPE_DATA),
            vec![vec![entity.addr(), world.addr(), 0x777]]
        );
        assert_eq!(
            calls_to(&e, SET_TIME_INFORMATION),
            vec![vec![0, 0, entity.addr() + 0xf0]]
        );
        assert_eq!(
            calls_to(&e, ADD_ENTITY_SI),
            vec![vec![world.addr(), entity.addr(), 9]]
        );
        let sequence = [
            SET_CACHED_SHAPE_DATA,
            SET_TIME_INFORMATION,
            ADD_REFERENCE,
            ADD_ENTITY_SI,
            ADD_ENTITY_BP,
            FIRE_WORLD_ENTITY_ADDED,
            FIRE_ENTITY_ADDED,
            QUEUE_FLUSH_PENDING,
            QUEUE_FLUSH_BODY_PENDING,
        ];
        assert_eq!(order_of(&e, &sequence), sequence.to_vec());
        assert_eq!(e.get(world, HkpWorld::m_pendingOperationsCount), 0);
        assert_eq!(e.get(world, HkpWorld::m_pendingBodyOperationsCount), 0);
        assert_eq!(calls_to(&e, QUEUE_FLUSH_PENDING), vec![vec![0x5000]]);
    }

    #[test]
    fn add_entity_keeps_an_existing_motion_and_respects_the_block_flag() {
        let mut e = add_entity_engine();
        let world = world(&mut e);
        let entity = entity(&mut e);
        e.set(entity, HkpEntity::m_collidableMotion, Ptr::new(0x6666));
        e.set(world, HkpWorld::m_pendingOperationsCount, 3);
        e.set(world, HkpWorld::m_blockExecutingPendingOperations, true);
        e.call(0x00c9_14d0, &args![world, entity, 9u32]);
        assert_eq!(e.get(entity, HkpEntity::m_collidableMotion).addr(), 0x6666);
        assert!(calls_to(&e, FAKE_1).is_empty());
        assert_eq!(e.get(world, HkpWorld::m_pendingOperationsCount), 3);
        assert!(calls_to(&e, QUEUE_FLUSH_PENDING).is_empty());
    }

    #[test]
    fn add_entity_is_queued_inside_a_critical_section() {
        let mut e = add_entity_engine();
        let queued = operation_recorder(&mut e);
        let world = world(&mut e);
        let entity = entity(&mut e);
        e.set(world, HkpWorld::m_criticalOperationsLockCount, 2);
        e.set(world, HkpWorld::m_pendingOperations, Ptr::new(0x5000));
        let returned = e.call(0x00c9_14d0, &args![world, entity, 9u32]).u32();
        assert_eq!(returned, 0);
        assert_eq!(
            queued.borrow().clone(),
            vec![(0x5000, op_bytes(1, entity.addr(), 9, 0))]
        );
        assert!(calls_to(&e, ADD_ENTITY_SI).is_empty());
    }

    fn remove_entity_engine() -> Engine {
        let mut e = logging();
        silent(
            &mut e,
            &[
                REMOVE_ENTITY_SI,
                FIRE_WORLD_ENTITY_REMOVED,
                FIRE_ENTITY_REMOVED,
                REMOVE_ENTITY_BP,
                ENTITY_RELEASE_BOUNDING_DATA,
                REMOVE_REFERENCE,
                QUEUE_FLUSH_PENDING,
                QUEUE_FLUSH_BODY_PENDING,
                FAKE_1,
                FAKE_2,
            ],
        );
        e
    }

    #[test]
    fn remove_entity_runs_the_whole_sequence() {
        let mut e = remove_entity_engine();
        let world = world(&mut e);
        let entity = entity(&mut e);
        let result = e.mem.alloc(8);
        e.set(world, HkpWorld::m_pendingOperationsCount, 1);
        e.set(
            entity,
            HkpEntity::m_boundingVolumeChildData,
            Ptr::new(0x9000),
        );
        e.call(0x00c9_1620, &args![world, result, entity]);
        assert_eq!(e.mem.u8(result), 1);
        assert_eq!(e.get(world, HkpWorld::m_criticalOperationsLockCount), 0);
        assert!(e.get(world, HkpWorld::m_criticalOperationsAllowed));
        let sequence = [
            REMOVE_ENTITY_SI,
            FIRE_WORLD_ENTITY_REMOVED,
            FIRE_ENTITY_REMOVED,
            REMOVE_ENTITY_BP,
            ENTITY_RELEASE_BOUNDING_DATA,
            FAKE_2,
            REMOVE_REFERENCE,
            QUEUE_FLUSH_PENDING,
        ];
        assert_eq!(order_of(&e, &sequence), sequence.to_vec());
        assert_eq!(
            calls_to(&e, ENTITY_RELEASE_BOUNDING_DATA),
            vec![vec![entity.addr() + 0x30]]
        );
        assert_eq!(calls_to(&e, FAKE_2), vec![vec![entity.addr()]]);
        assert_eq!(calls_to(&e, REMOVE_REFERENCE), vec![vec![entity.addr()]]);
    }

    #[test]
    fn remove_entity_skips_the_optional_steps() {
        let mut e = remove_entity_engine();
        let world = world(&mut e);
        let entity = entity(&mut e);
        let result = e.mem.alloc(8);
        e.set(entity, HkpEntity::m_memSizeAndFlags, 0x20);
        e.set(world, HkpWorld::m_blockExecutingPendingOperations, true);
        e.set(world, HkpWorld::m_pendingOperationsCount, 1);
        e.call(0x00c9_1620, &args![world, result, entity]);
        assert!(calls_to(&e, ENTITY_RELEASE_BOUNDING_DATA).is_empty());
        assert!(calls_to(&e, FAKE_2).is_empty());
        assert!(calls_to(&e, QUEUE_FLUSH_PENDING).is_empty());
        assert_eq!(e.mem.u8(result), 1);
    }

    #[test]
    fn remove_entity_is_queued_inside_a_critical_section() {
        let mut e = remove_entity_engine();
        let queued = operation_recorder(&mut e);
        let world = world(&mut e);
        let entity = entity(&mut e);
        let result = e.mem.alloc(8);
        e.mem.set_u8(result, 7);
        e.set(world, HkpWorld::m_criticalOperationsLockCount, 1);
        e.set(world, HkpWorld::m_pendingOperations, Ptr::new(0x5000));
        e.call(0x00c9_1620, &args![world, result, entity]);
        assert_eq!(
            queued.borrow().clone(),
            vec![(0x5000, op_bytes(2, entity.addr(), 0, 0))]
        );
        assert_eq!(e.mem.u8(result), 0);
        assert!(calls_to(&e, REMOVE_ENTITY_SI).is_empty());
    }

    /// Nulls a listener in the array at `data_offset` through the function at `address`.
    fn check_listener_removal(address: u32, data_offset: u32) {
        let mut e = Engine::new();
        let world = world(&mut e);
        let data = array_of(&mut e, &[0x10, 0x20, 0x30]);
        e.mem.set_u32(world.addr() + data_offset, data);
        e.mem.set_i32(world.addr() + data_offset + 4, 3);
        e.call(address, &args![world, 0x20u32]);
        assert_eq!(
            (e.mem.u32(data), e.mem.u32(data + 4), e.mem.u32(data + 8)),
            (0x10, 0, 0x30)
        );
        // No match: the word before the array is cleared.
        e.mem.set_u32(data - 4, 0xdead);
        e.call(address, &args![world, 0x99u32]);
        assert_eq!(e.mem.u32(data - 4), 0);
        assert_eq!(e.mem.u32(data), 0x10);
        // An empty array behaves the same way.
        e.mem.set_u32(data - 4, 0xdead);
        e.mem.set_i32(world.addr() + data_offset + 4, 0);
        e.call(address, &args![world, 0x10u32]);
        assert_eq!(e.mem.u32(data - 4), 0);
        assert_eq!(e.mem.u32(data), 0x10);
    }

    #[test]
    fn fn_00c91700_removes_a_listener() {
        check_listener_removal(0x00c9_1700, 0x11c);
    }

    #[test]
    fn fn_00c91750_removes_a_listener() {
        check_listener_removal(0x00c9_1750, 0x140);
    }

    #[test]
    fn fn_00c917a0_removes_a_listener() {
        check_listener_removal(0x00c9_17a0, 0x128);
    }

    #[test]
    fn fn_00c917f0_removes_a_listener() {
        check_listener_removal(0x00c9_17f0, 0x134);
    }

    #[test]
    fn fn_00c91840_removes_a_listener() {
        check_listener_removal(0x00c9_1840, 0x158);
    }

    #[test]
    fn fn_00c918e0_removes_a_listener() {
        check_listener_removal(0x00c9_18e0, 0x164);
    }

    #[test]
    fn fn_00c91a20_removes_a_listener() {
        check_listener_removal(0x00c9_1a20, 0x1a0);
    }

    #[test]
    fn fn_00c91a70_removes_a_listener() {
        check_listener_removal(0x00c9_1a70, 0x14c);
    }

    /// A world whose simulation's slots +0xc, +0x10, +0x14 and +0x18 all answer 0x77.
    fn stepping_world(e: &mut Engine) -> (Ptr<HkpWorld>, u32) {
        silent(e, &[LOCK_ENTER, LOCK_LEAVE, WATCH_HEAP_MEMORY]);
        let world = world(e);
        let slots = [
            (0xc, FAKE_1),
            (0x10, FAKE_1),
            (0x14, FAKE_1),
            (0x18, FAKE_1),
        ];
        let simulation = object_with_vtable(e, 0x40, &slots);
        returning(e, FAKE_1, 0x77);
        e.set(world, HkpWorld::m_simulation, Ptr::new(simulation));
        (world, simulation)
    }

    #[test]
    fn step_delta_time_steps_inside_the_lock() {
        let mut e = logging();
        let (world, simulation) = stepping_world(&mut e);
        let result = e.call(0x00c9_1b10, &args![world, 0.5f32]).u32();
        assert_eq!(result, 0x77);
        let sequence = [LOCK_ENTER, WATCH_HEAP_MEMORY, FAKE_1, LOCK_LEAVE];
        assert_eq!(order_of(&e, &sequence), sequence.to_vec());
        assert_eq!(calls_to(&e, WATCH_HEAP_MEMORY), vec![vec![world.addr()]]);
        assert_eq!(
            calls_to(&e, FAKE_1),
            vec![vec![simulation, 0.5f32.to_bits()]]
        );
        assert_eq!(e.get(world, HkpWorld::m_isLocked), 0);
    }

    #[test]
    fn step_delta_time_tells_the_tree_afterwards() {
        let mut e = logging();
        let (world, _) = stepping_world(&mut e);
        silent(&mut e, &[FAKE_2, FAKE_3]);
        let tree = object_with_vtable(&mut e, 0x20, &[(0xc, FAKE_2), (0x14, FAKE_3)]);
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::new(tree));
        e.call(0x00c9_1b10, &args![world, 0.5f32]);
        assert!(calls_to(&e, FAKE_2).is_empty());
        assert_eq!(calls_to(&e, FAKE_3), vec![vec![tree, 0]]);
        e.set(world, HkpWorld::m_autoUpdateTree, true);
        e.call(0x00c9_1b10, &args![world, 0.5f32]);
        assert_eq!(calls_to(&e, FAKE_2), vec![vec![tree, 0, 0]]);
        assert_eq!(calls_to(&e, FAKE_3).len(), 1);
    }

    #[test]
    fn fn_00c91b70_passes_the_time_to_slot_0x10() {
        let mut e = logging();
        let (world, simulation) = stepping_world(&mut e);
        let result = e.call(0x00c9_1b70, &args![world, 2.0f32]).u32();
        assert_eq!(result, 0x77);
        assert_eq!(
            calls_to(&e, FAKE_1),
            vec![vec![simulation, 2.0f32.to_bits()]]
        );
        assert_eq!(calls_to(&e, LOCK_ENTER).len(), 1);
        assert_eq!(calls_to(&e, LOCK_LEAVE).len(), 1);
    }

    #[test]
    fn fn_00c91ba0_calls_slot_0x14_locked() {
        let mut e = logging();
        let (world, simulation) = stepping_world(&mut e);
        assert_eq!(e.call(0x00c9_1ba0, &args![world]).u32(), 0x77);
        assert_eq!(calls_to(&e, FAKE_1), vec![vec![simulation]]);
        assert_eq!(calls_to(&e, LOCK_ENTER).len(), 1);
        assert_eq!(calls_to(&e, LOCK_LEAVE).len(), 1);
    }

    #[test]
    fn fn_00c91bd0_calls_slot_0x18_locked() {
        let mut e = logging();
        let (world, simulation) = stepping_world(&mut e);
        assert_eq!(e.call(0x00c9_1bd0, &args![world]).u32(), 0x77);
        assert_eq!(calls_to(&e, FAKE_1), vec![vec![simulation]]);
        assert_eq!(calls_to(&e, LOCK_ENTER).len(), 1);
        assert_eq!(calls_to(&e, LOCK_LEAVE).len(), 1);
    }

    #[test]
    fn finish_mt_step_returns_a_failure_and_otherwise_tells_the_tree() {
        let mut e = logging();
        silent(&mut e, &[FAKE_2, FAKE_3]);
        returning(&mut e, SIMULATION_FINISH_MT_STEP, 5);
        let world = world(&mut e);
        e.set(world, HkpWorld::m_simulation, Ptr::new(0x1234));
        let tree = object_with_vtable(&mut e, 0x20, &[(0xc, FAKE_2), (0x14, FAKE_3)]);
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::new(tree));
        assert_eq!(e.call(0x00c9_1c00, &args![world, 7u32, 8u32]).u32(), 5);
        assert_eq!(
            calls_to(&e, SIMULATION_FINISH_MT_STEP),
            vec![vec![0x1234, 7, 8]]
        );
        assert!(calls_to(&e, FAKE_2).is_empty() && calls_to(&e, FAKE_3).is_empty());
        returning(&mut e, SIMULATION_FINISH_MT_STEP, 0);
        assert_eq!(e.call(0x00c9_1c00, &args![world, 7u32, 8u32]).u32(), 0);
        assert_eq!(calls_to(&e, FAKE_3), vec![vec![tree, 0]]);
        e.set(world, HkpWorld::m_autoUpdateTree, true);
        assert_eq!(e.call(0x00c9_1c00, &args![world, 7u32, 8u32]).u32(), 0);
        assert_eq!(calls_to(&e, FAKE_2), vec![vec![tree, 7, 8]]);
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::NULL);
        assert_eq!(e.call(0x00c9_1c00, &args![world, 7u32, 8u32]).u32(), 0);
        assert_eq!(calls_to(&e, FAKE_2).len(), 1);
    }

    /// The plain copies of `00c91c50`: (offset in the world, offset in the info, bytes).
    const INFO_COPIES: [(u32, u32, u32); 33] = [
        (0xd0, 0x90, 4),
        (0xdc, 0xac, 4),
        (0xd8, 0xa8, 4),
        (0xf4, 0xc0, 4),
        (0xf8, 0xc4, 4),
        (0xfc, 0xc8, 4),
        (0x100, 0xcc, 4),
        (0x240, 0x24, 4),
        (0x1e4, 0x78, 4),
        (0x1e8, 0x7c, 4),
        (0x338, 0x20, 4),
        (0x330, 0x70, 4),
        (0x334, 0x64, 4),
        (0x2f4, 0x80, 4),
        (0x2f8, 0x84, 4),
        (0xe0, 0xb0, 4),
        (0xe8, 0xb8, 4),
        (0xec, 0xbc, 4),
        (0xb4, 0xd4, 4),
        (0x60, 0x53, 1),
        (0xd4, 0x98, 1),
        (0x340, 0x74, 1),
        (0x304, 0x8c, 1),
        (0x305, 0xa4, 1),
        (0x306, 0xa5, 1),
        (0x307, 0xa6, 1),
        (0xd6, 0xa7, 1),
        (0xe4, 0xb4, 1),
        (0xd5, 0xd0, 1),
        (0xf0, 0xd1, 1),
        (0xc8, 0xd2, 1),
        (0xb0, 0xd8, 1),
        (0xb1, 0xd9, 1),
    ];

    #[test]
    fn get_info_copies_the_worlds_settings() {
        let mut e = logging();
        silent(&mut e, &[ADD_REFERENCE, REMOVE_REFERENCE]);
        e.register(X87_SQRT, |_, words| Ret {
            st0: f64::take(words, &mut 0).sqrt(),
            ..Ret::default()
        });
        let world = world(&mut e);
        let w = world.addr();
        let info = e.mem.alloc(0x100);
        for (index, (from, _, size)) in INFO_COPIES.iter().enumerate() {
            let value = 0x3f00_0000 + index as u32 + 1;
            if *size == 4 {
                e.mem.set_u32(w + from, value);
            } else {
                e.mem.set_u8(w + from, value as u8);
            }
        }
        // The word at +0xcc is copied first and then overwritten by the one at +0xd0, which
        // is a signalling NaN here and comes out quieted.
        e.mem.set_u32(w + 0xcc, 0x3e99_999a);
        e.mem.set_u32(w + 0xd0, 0x7f80_0001);
        for lane in 0..4 {
            e.mem.set_u32(w + 0x10 + 4 * lane, 0xa0 + lane);
            e.mem.set_u32(w + 0x310 + 4 * lane, 0xb0 + lane);
            e.mem.set_u32(w + 0x320 + 4 * lane, 0xc0 + lane);
        }
        let collision_input = e.mem.alloc(0x80);
        let extra = e.mem.alloc(8);
        e.mem.set_f32(collision_input + 8, 1.25);
        e.mem.set_u8(collision_input + 0x6d, 0x6d);
        e.mem.set_u8(collision_input + 0x6c, 0x6c);
        e.mem.set_u32(collision_input + 0x70, extra);
        e.mem.set_f32(extra, 3.5);
        e.mem.set_u32(extra + 4, 0x1357);
        e.set(world, HkpWorld::m_collisionInput, Ptr::new(collision_input));
        let dispatcher = e.mem.alloc(0x1e50);
        e.mem.set_f32(dispatcher + 0x1e40, 0.75);
        e.mem.set_f32(dispatcher + 0x1e44, 0.125);
        e.set(world, HkpWorld::m_collisionDispatcher, Ptr::new(dispatcher));
        let simulation = e.mem.alloc(0x40);
        e.mem.set_f32(simulation + 0x24, 9.5);
        e.set(world, HkpWorld::m_simulation, Ptr::new(simulation));
        e.mem.set_f32(w + 0x308, 16.0);
        e.set(world, HkpWorld::m_collisionFilter, Ptr::new(0x1001));
        e.set(world, HkpWorld::m_convexListFilter, Ptr::new(0x1002));
        e.set(world, HkpWorld::m_memoryWatchDog, Ptr::new(0x1003));
        e.mem.set_u32(info + 0x58, 0x2001);
        e.mem.set_u32(info + 0x5c, 0);
        e.mem.set_u32(info + 0x6c, 0x2003);
        e.call(0x00c9_1c50, &args![world, info]);
        for (index, (from, to, size)) in INFO_COPIES.iter().enumerate() {
            let value = 0x3f00_0000 + index as u32 + 1;
            if *from == 0xd0 {
                assert_eq!(e.mem.u32(info + to), 0x7fc0_0001);
            } else if *size == 4 {
                assert_eq!(e.mem.u32(info + to), value, "word {from:#x}");
            } else {
                assert_eq!(e.mem.u8(info + to), value as u8, "byte {from:#x}");
            }
        }
        for lane in 0..4 {
            assert_eq!(e.mem.u32(info + 0x10 + 4 * lane), 0xa0 + lane);
            assert_eq!(e.mem.u32(info + 0x30 + 4 * lane), 0xb0 + lane);
            assert_eq!(e.mem.u32(info + 0x40 + 4 * lane), 0xc0 + lane);
        }
        assert_eq!(e.mem.f32(info + 0x54), 1.25);
        assert_eq!(e.mem.u8(info + 0x75), 0x6d);
        assert_eq!(e.mem.u8(info + 0x99), 0x6c);
        assert_eq!(e.mem.f32(info + 0x9c), 3.5);
        assert_eq!(e.mem.u32(info + 0xa0), 0x1357);
        assert_eq!(e.mem.f32(info + 0x60), 0.75);
        assert_eq!(e.mem.f32(info + 0x68), 0.125);
        assert_eq!(e.mem.f32(info + 0xdc), 9.5);
        assert_eq!(e.mem.f32(info + 0x88), 4.0);
        // No border: kind 3 and flag 0; no tree, no callback util.
        assert_eq!(e.mem.u8(info + 0x28), 3);
        assert_eq!(e.mem.u8(info + 0x29), 0);
        assert_eq!(e.mem.u8(info + 0x50), 0);
        assert_eq!(e.mem.u8(info + 0xe0), 0);
        // References: added to each new pointer, removed from each one replaced.
        assert_eq!(
            (
                e.mem.u32(info + 0x58),
                e.mem.u32(info + 0x5c),
                e.mem.u32(info + 0x6c)
            ),
            (0x1001, 0x1002, 0x1003)
        );
        assert_eq!(
            calls_to(&e, ADD_REFERENCE),
            vec![vec![0x1001], vec![0x1002], vec![0x1003]]
        );
        assert_eq!(
            calls_to(&e, REMOVE_REFERENCE),
            vec![vec![0x2001], vec![0x2003]]
        );
        // With a border, a tree and the callback util present.
        let border = e.mem.alloc(0x40);
        e.mem.set_u8(border + 0x30, 5);
        e.mem.set_u8(border + 0x34, 1);
        e.set(world, HkpWorld::m_broadPhaseBorder, Ptr::new(border));
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::new(0x3000));
        e.set(world, HkpWorld::m_collisionCallbackUtil, Ptr::new(0x3001));
        e.call(0x00c9_1c50, &args![world, info]);
        assert_eq!(e.mem.u8(info + 0x28), 5);
        assert_eq!(e.mem.u8(info + 0x29), 1);
        assert_eq!(e.mem.u8(info + 0x50), 1);
        assert_eq!(e.mem.u8(info + 0xe0), 1);
    }

    /// An engine with a profiling stream that holds `records` records; returns it and the
    /// stream block and the first record address.
    fn profiling_engine(records: u32) -> (Engine, u32, u32) {
        let mut e = logging();
        e.map(0x0126_8000, 0x1000);
        e.set_global(PROFILE_SLOT, 3u32);
        let stream = e.mem.alloc(0x20);
        let area = e.mem.alloc(12 * records);
        e.mem.set_u32(stream + 4, area);
        e.mem.set_u32(stream + 0xc, area + 12 * records);
        e.register_double(TLS_GET_VALUE, move |_, _| Ret {
            eax: stream,
            ..Ret::default()
        });
        (e, stream, area)
    }

    /// The labels of the profiling records written so far.
    fn labels(e: &Engine, stream: u32, start: u32) -> Vec<u32> {
        let cursor = e.mem.u32(stream + 4);
        (start..cursor)
            .step_by(12)
            .map(|record| e.mem.u32(record))
            .collect()
    }

    #[test]
    fn cast_ray_uses_the_tree_when_it_is_in_use() {
        let (mut e, stream, start) = profiling_engine(4);
        silent(&mut e, &[FAKE_1]);
        returning(&mut e, FAKE_2, 1);
        let world = world(&mut e);
        let tree = object_with_vtable(&mut e, 0x20, &[(0x10, FAKE_2), (0x18, FAKE_1)]);
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::new(tree));
        e.set(world, HkpWorld::m_collisionFilter, Ptr::new(0x1234));
        e.call(0x00c9_1f70, &args![world, 0x11u32, 0x22u32]);
        assert_eq!(calls_to(&e, FAKE_1), vec![vec![tree, 0x11, 0x1234, 0x22]]);
        assert!(calls_to(&e, WORLD_RAY_CASTER_CAST_RAY).is_empty());
        assert_eq!(
            labels(&e, stream, start),
            vec![PROFILE_CAST_RAY_COLLECTOR, PROFILE_END]
        );
    }

    #[test]
    fn cast_ray_builds_a_caster_without_the_tree() {
        let (mut e, stream, start) = profiling_engine(1);
        let words: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(vec![]));
        let sink = words.clone();
        e.register_double(WORLD_RAY_CASTER_CAST_RAY, move |e, a| {
            let caster = a[0];
            let seen = [
                e.mem.u32(caster),
                e.mem.u32(caster + 0x40),
                e.mem.u32(caster + 0x44),
            ];
            sink.borrow_mut().extend(seen);
            Ret::default()
        });
        let world = world(&mut e);
        e.set(world, HkpWorld::m_broadPhase, Ptr::new(0x2000));
        e.set(world, HkpWorld::m_collisionFilter, Ptr::new(0x1234));
        e.call(0x00c9_1f70, &args![world, 0x11u32, 0x22u32]);
        let log = calls_to(&e, WORLD_RAY_CASTER_CAST_RAY);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0][1..], [0x2000, 0x11, 0x1234, 0, 0x22]);
        assert_eq!(words.borrow().clone(), vec![RAY_CASTER_VTABLE, 0, 0]);
        // Only one record fits: the end record is dropped.
        assert_eq!(labels(&e, stream, start), vec![PROFILE_CAST_RAY_COLLECTOR]);
    }

    #[test]
    fn cast_ray_ov2_uses_the_tree_slot_0x1c_or_the_simple_caster() {
        let (mut e, stream, start) = profiling_engine(4);
        silent(&mut e, &[FAKE_1]);
        returning(&mut e, FAKE_2, 1);
        let world = world(&mut e);
        let tree = object_with_vtable(&mut e, 0x20, &[(0x10, FAKE_2), (0x1c, FAKE_1)]);
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::new(tree));
        e.set(world, HkpWorld::m_collisionFilter, Ptr::new(0x1234));
        e.call(0x00c9_2040, &args![world, 0x11u32, 0x22u32]);
        assert_eq!(calls_to(&e, FAKE_1), vec![vec![tree, 0x11, 0x1234, 0x22]]);
        // The tree reports it is not in use: the simple caster runs.
        returning(&mut e, FAKE_2, 0);
        let words: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(vec![]));
        let sink = words.clone();
        e.register_double(SIMPLE_RAY_CASTER_CAST_RAY, move |e, a| {
            let caster = a[0];
            let seen = [
                e.mem.u32(caster),
                e.mem.u32(caster + 0x30),
                e.mem.u32(caster + 0x34),
            ];
            sink.borrow_mut().extend(seen);
            Ret::default()
        });
        e.set(world, HkpWorld::m_broadPhase, Ptr::new(0x2000));
        e.call(0x00c9_2040, &args![world, 0x11u32, 0x22u32]);
        let log = calls_to(&e, SIMPLE_RAY_CASTER_CAST_RAY);
        assert_eq!(log[0][1..], [0x2000, 0x11, 0x1234, 0, 0x22]);
        assert_eq!(words.borrow().clone(), vec![SIMPLE_RAY_CASTER_VTABLE, 0, 0]);
        assert_eq!(
            labels(&e, stream, start),
            vec![PROFILE_CAST_RAY, PROFILE_END, PROFILE_CAST_RAY, PROFILE_END]
        );
    }

    #[test]
    fn linear_cast_uses_the_tree_or_a_linear_caster() {
        let (mut e, stream, start) = profiling_engine(8);
        silent(&mut e, &[FAKE_1]);
        returning(&mut e, FAKE_2, 1);
        let world = world(&mut e);
        let collision_input = e.mem.alloc(0x80);
        e.mem.set_u32(collision_input + 0x70, 0x4444);
        e.set(world, HkpWorld::m_collisionInput, Ptr::new(collision_input));
        let filter = e.mem.alloc(0x20);
        e.set(world, HkpWorld::m_collisionFilter, Ptr::new(filter));
        let tree = object_with_vtable(&mut e, 0x20, &[(0x10, FAKE_2), (0x20, FAKE_1)]);
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::new(tree));
        e.call(0x00c9_2430, &args![world, 1u32, 2u32, 3u32, 4u32]);
        assert_eq!(
            calls_to(&e, FAKE_1),
            vec![vec![tree, 1, 2, filter + 8, collision_input, 0x4444, 3, 4]]
        );
        // No tree: the linear caster object is built with the default float.
        e.map(0x0104_d000, 0x1000);
        e.set_global(LINEAR_CASTER_DEFAULT, f32::from_bits(0x7f80_0001));
        e.set(world, HkpWorld::m_kdTreeManager, Ptr::NULL);
        e.set(world, HkpWorld::m_broadPhase, Ptr::new(0x2000));
        let words: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(vec![]));
        let sink = words.clone();
        e.register_double(LINEAR_CASTER_LINEAR_CAST, move |e, a| {
            let caster = a[0];
            let seen = [
                e.mem.u32(caster),
                e.mem.u32(caster + 0x24),
                e.mem.u32(caster + 0x80),
            ];
            sink.borrow_mut().extend(seen);
            Ret::default()
        });
        e.call(0x00c9_2430, &args![world, 1u32, 2u32, 3u32, 4u32]);
        let log = calls_to(&e, LINEAR_CASTER_LINEAR_CAST);
        assert_eq!(
            log[0][1..],
            [0x2000, 1, 2, filter + 8, collision_input, 0x4444, 0, 3, 4]
        );
        assert_eq!(
            words.borrow().clone(),
            vec![LINEAR_CASTER_VTABLE, 0, 0x7fc0_0001]
        );
        assert_eq!(
            labels(&e, stream, start),
            vec![
                PROFILE_LINEAR_CAST,
                PROFILE_END,
                PROFILE_LINEAR_CAST,
                PROFILE_END
            ]
        );
        // Without a filter the base passed on is 0.
        e.set(world, HkpWorld::m_collisionFilter, Ptr::NULL);
        e.call(0x00c9_2430, &args![world, 1u32, 2u32, 3u32, 4u32]);
        assert_eq!(calls_to(&e, LINEAR_CASTER_LINEAR_CAST)[1][4], 0);
    }

    /// What a job dispatch test observes: the jobs handed to the queue (their bytes at the
    /// time of the call) and the blocks freed.
    struct Dispatch {
        e: Engine,
        jobs: Rc<RefCell<Vec<Vec<u8>>>>,
        frees: Records,
        world: Ptr<HkpWorld>,
        block: u32,
        notifier: u32,
    }

    fn dispatch_engine(job_size: u32) -> Dispatch {
        let mut e = logging();
        let jobs: Rc<RefCell<Vec<Vec<u8>>>> = Rc::new(RefCell::new(vec![]));
        let sink = jobs.clone();
        e.register_double(JOB_QUEUE_ADD_JOB, move |e, words| {
            sink.borrow_mut().push(e.mem.bytes(words[1], job_size));
            Ret::default()
        });
        let block = e.mem.alloc(0x10);
        returning(&mut e, FAKE_1, block);
        let frees = recording(&mut e, FAKE_2);
        let allocator = object_with_vtable(&mut e, 0x20, &[(4, FAKE_1), (8, FAKE_2)]);
        let router = e.mem.alloc(0x20);
        e.mem.set_u32(router + 0x10, allocator);
        returning(&mut e, MEMORY_ROUTER, router);
        silent(
            &mut e,
            &[FAKE_3, FAKE_4, SEMAPHORE_ACQUIRE, JOB_HEADER_INIT],
        );
        let notifier = object_with_vtable(&mut e, 0x20, &[(0xc, FAKE_3), (0x10, FAKE_4)]);
        let world = world(&mut e);
        e.set(world, HkpWorld::m_collisionInput, Ptr::new(0x3000));
        e.set(world, HkpWorld::m_broadPhase, Ptr::new(0x4000));
        Dispatch {
            e,
            jobs,
            frees,
            world,
            block,
            notifier,
        }
    }

    /// Gives the world a kd-tree manager that is in use and holds two words.
    fn use_tree(d: &mut Dispatch) {
        returning(&mut d.e, FAKE_5, 1);
        let tree = object_with_vtable(&mut d.e, 0x20, &[(0x10, FAKE_5)]);
        d.e.mem.set_u32(tree + 0xc, 0xaaaa);
        d.e.mem.set_u32(tree + 0x10, 0xbbbb);
        d.e.set(d.world, HkpWorld::m_kdTreeManager, Ptr::new(tree));
    }

    /// The word at `offset` of a recorded job.
    fn job_word(job: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            job[offset],
            job[offset + 1],
            job[offset + 2],
            job[offset + 3],
        ])
    }

    /// The jobs were added, then the notifier and the semaphore waited, then the block freed.
    fn assert_waited(d: &Dispatch, queue: u32, semaphore: u32, job_count: usize) {
        let mut expected = vec![JOB_QUEUE_ADD_JOB; job_count];
        expected.extend([FAKE_3, FAKE_4, SEMAPHORE_ACQUIRE, FAKE_2]);
        let wanted = [JOB_QUEUE_ADD_JOB, FAKE_3, FAKE_4, SEMAPHORE_ACQUIRE, FAKE_2];
        assert_eq!(order_of(&d.e, &wanted), expected);
        assert_eq!(calls_to(&d.e, FAKE_3), vec![vec![d.notifier, queue, 0xb]]);
        assert_eq!(calls_to(&d.e, FAKE_4), vec![vec![d.notifier]]);
        assert_eq!(calls_to(&d.e, SEMAPHORE_ACQUIRE), vec![vec![semaphore]]);
        let frees = d.frees.borrow().clone();
        assert_eq!(frees.len(), 1);
        assert_eq!(frees[0][1..], [d.block, 0x10]);
        for add in calls_to(&d.e, JOB_QUEUE_ADD_JOB) {
            assert_eq!(add[0], queue);
            assert_eq!(add[2], 1);
        }
    }

    #[test]
    fn fn_00c92110_splits_the_array_into_batched_jobs() {
        let mut d = dispatch_engine(0x50);
        let (queue, semaphore, array) = (0x5000, 0x6000, 0x7000);
        d.e.call(
            0x00c9_2110,
            &args![d.world, array, 130i32, queue, d.notifier, semaphore, 64i32],
        );
        let jobs = d.jobs.borrow().clone();
        assert_eq!(jobs.len(), 3);
        assert_eq!(d.e.mem.u32(d.block), 3);
        assert_eq!(jobs[0][..3], [1, 3, 1]);
        assert_eq!(u16::from_le_bytes([jobs[0][4], jobs[0][5]]), 0x50);
        assert_eq!(u16::from_le_bytes([jobs[0][6], jobs[0][7]]), 0xffff);
        assert_eq!(job_word(&jobs[0], 0x10), semaphore);
        assert_eq!(job_word(&jobs[0], 0x14), d.block);
        assert_eq!(job_word(&jobs[0], 0x1c), 0x3000);
        assert_eq!(job_word(&jobs[0], 0x20), 0x40);
        assert_eq!(job_word(&jobs[0], 0x34), 0x4000);
        let counts: Vec<u32> = jobs.iter().map(|job| job_word(job, 0x28)).collect();
        let arrays: Vec<u32> = jobs.iter().map(|job| job_word(job, 0x24)).collect();
        assert_eq!(counts, vec![64, 64, 2]);
        assert_eq!(arrays, vec![array, array + 0x1000, array + 0x2000]);
        assert_waited(&d, queue, semaphore, 3);
    }

    #[test]
    fn fn_00c92110_hands_the_trees_words_to_the_job() {
        let mut d = dispatch_engine(0x50);
        use_tree(&mut d);
        d.e.call(
            0x00c9_2110,
            &args![d.world, 0x7000u32, 10i32, 0x5000u32, d.notifier, 0x6000u32, 64i32],
        );
        let jobs = d.jobs.borrow().clone();
        assert_eq!(jobs.len(), 1);
        assert_eq!(job_word(&jobs[0], 0x34), 0);
        assert_eq!(job_word(&jobs[0], 0x3c), 0xaaaa);
        assert_eq!(job_word(&jobs[0], 0x40), 0xbbbb);
        assert_eq!(job_word(&jobs[0], 0x28), 10);
    }

    #[test]
    fn fn_00c92110_with_no_elements_adds_no_job() {
        let mut d = dispatch_engine(0x50);
        d.e.call(
            0x00c9_2110,
            &args![d.world, 0x7000u32, 0i32, 0x5000u32, d.notifier, 0x6000u32, 64i32],
        );
        assert!(d.jobs.borrow().is_empty());
        assert_eq!(d.e.mem.u32(d.block), 1);
        assert_waited(&d, 0x5000, 0x6000, 0);
    }

    #[test]
    fn fn_00c92290_clears_the_elements_and_batches_the_jobs() {
        let mut d = dispatch_engine(0x40);
        let collision_input = d.e.mem.alloc(0x20);
        d.e.mem.set_u32(collision_input + 8, 0x7f80_0001);
        d.e.set(
            d.world,
            HkpWorld::m_collisionInput,
            Ptr::new(collision_input),
        );
        let array = d.e.mem.alloc(0x10 * 5);
        for index in 0..5 {
            d.e.mem.set_u32(array + 0xc + 0x10 * index, 0xffff);
        }
        d.e.call(
            0x00c9_2290,
            &args![d.world, array, 5i32, 0x5000u32, d.notifier, 0x6000u32, 2i32],
        );
        for index in 0..5 {
            assert_eq!(d.e.mem.u32(array + 0xc + 0x10 * index), 0);
        }
        let jobs = d.jobs.borrow().clone();
        assert_eq!(jobs.len(), 3);
        assert_eq!(jobs[0][..3], [2, 2, 2]);
        assert_eq!(job_word(&jobs[0], 0x2c), 0x7fc0_0001);
        assert_eq!(job_word(&jobs[0], 0x30), 0x40);
        assert_eq!(job_word(&jobs[0], 0x20), 0x4000);
        assert_eq!(job_word(&jobs[0], 0x18), collision_input);
        let counts: Vec<u32> = jobs.iter().map(|job| job_word(job, 0x38)).collect();
        let arrays: Vec<u32> = jobs.iter().map(|job| job_word(job, 0x34)).collect();
        assert_eq!(counts, vec![2, 2, 1]);
        assert_eq!(arrays, vec![array, array + 0x20, array + 0x40]);
        assert_eq!(d.e.mem.u32(d.block), 1);
        assert_eq!(calls_to(&d.e, JOB_HEADER_INIT).len(), 1);
        assert_waited(&d, 0x5000, 0x6000, 3);
    }

    #[test]
    fn fn_00c92290_hands_the_trees_words_to_the_job() {
        let mut d = dispatch_engine(0x40);
        let collision_input = d.e.mem.alloc(0x20);
        d.e.set(
            d.world,
            HkpWorld::m_collisionInput,
            Ptr::new(collision_input),
        );
        use_tree(&mut d);
        let array = d.e.mem.alloc(0x40);
        d.e.call(
            0x00c9_2290,
            &args![d.world, array, 1i32, 0x5000u32, d.notifier, 0x6000u32, 8i32],
        );
        let jobs = d.jobs.borrow().clone();
        assert_eq!(jobs.len(), 1);
        assert_eq!(job_word(&jobs[0], 0x20), 0);
        assert_eq!(job_word(&jobs[0], 0x24), 0xaaaa);
        assert_eq!(job_word(&jobs[0], 0x28), 0xbbbb);
    }

    #[test]
    fn fn_00c92540_batches_with_a_stride_of_0x30() {
        let mut d = dispatch_engine(0x40);
        let array = 0x7000u32;
        d.e.call(
            0x00c9_2540,
            &args![d.world, array, 5i32, 0x5000u32, d.notifier, 0x6000u32, 2i32],
        );
        let jobs = d.jobs.borrow().clone();
        assert_eq!(jobs.len(), 3);
        assert_eq!(jobs[0][..3], [4, 2, 1]);
        assert_eq!(job_word(&jobs[0], 0x18), 0x3000);
        assert_eq!(job_word(&jobs[0], 0x20), 0x40);
        assert_eq!(job_word(&jobs[0], 0x2c), 0x4000);
        let counts: Vec<u32> = jobs.iter().map(|job| job_word(job, 0x28)).collect();
        let arrays: Vec<u32> = jobs.iter().map(|job| job_word(job, 0x24)).collect();
        assert_eq!(counts, vec![2, 2, 1]);
        assert_eq!(arrays, vec![array, array + 0x60, array + 0xc0]);
        assert_eq!(d.e.mem.u32(d.block), 1);
        assert_waited(&d, 0x5000, 0x6000, 3);
    }

    #[test]
    fn fn_00c92540_hands_the_trees_words_to_the_job() {
        let mut d = dispatch_engine(0x40);
        use_tree(&mut d);
        d.e.call(
            0x00c9_2540,
            &args![d.world, 0x7000u32, 1i32, 0x5000u32, d.notifier, 0x6000u32, 8i32],
        );
        let jobs = d.jobs.borrow().clone();
        assert_eq!(job_word(&jobs[0], 0x2c), 0);
        assert_eq!(job_word(&jobs[0], 0x34), 0xaaaa);
        assert_eq!(job_word(&jobs[0], 0x38), 0xbbbb);
    }

    /// An island whose entity array holds `bodies`; its action array holds `actions`.
    fn make_island(e: &mut Engine, bodies: &[u32], actions: &[u32]) -> u32 {
        let island = e.mem.alloc(0x80);
        let body_array = array_of(e, bodies);
        e.mem.set_u32(island + 0x48, body_array);
        e.mem.set_i32(island + 0x4c, bodies.len() as i32);
        let action_array = array_of(e, actions);
        e.mem.set_u32(island + 0x38, action_array);
        e.mem.set_i32(island + 0x3c, actions.len() as i32);
        island
    }

    fn set_islands(e: &mut Engine, world: Ptr<HkpWorld>, active: &[u32], inactive: &[u32]) {
        let active_array = array_of(e, active);
        e.set(
            world,
            HkpWorld::m_activeSimulationIslands,
            Ptr::new(active_array),
        );
        e.set(
            world,
            HkpWorld::m_activeSimulationIslandsSize,
            active.len() as i32,
        );
        let inactive_array = array_of(e, inactive);
        e.set(
            world,
            HkpWorld::m_inactiveSimulationIslands,
            Ptr::new(inactive_array),
        );
        e.set(
            world,
            HkpWorld::m_inactiveSimulationIslandsSize,
            inactive.len() as i32,
        );
    }

    #[test]
    fn fn_00c926a0_adds_the_fixed_and_inactive_bodies() {
        let mut e = logging();
        silent(&mut e, &[PHYSICS_SYSTEM_ADD_RIGID_BODY]);
        let world = world(&mut e);
        let first = e.mem.alloc(0x20);
        let (second, third) = (0x2222, 0x3333);
        let fixed = make_island(&mut e, &[first, second], &[]);
        let inactive = make_island(&mut e, &[third], &[]);
        e.set(world, HkpWorld::m_fixedIsland, Ptr::new(fixed));
        set_islands(&mut e, world, &[], &[inactive]);
        let out = e.mem.alloc(8);
        let returned = e.call(0x00c9_26a0, &args![out, world, 0x9000u32]).u32();
        assert_eq!(returned, out);
        assert_eq!(e.mem.u8(out), 1);
        // The first fixed body has no word at +0x10: it is skipped.
        assert_eq!(
            calls_to(&e, PHYSICS_SYSTEM_ADD_RIGID_BODY),
            vec![vec![0x9000, second], vec![0x9000, third]]
        );
        // With a word there it is added too; without any island the flag is clear.
        e.mem.set_u32(first + 0x10, 1);
        e.call(0x00c9_26a0, &args![out, world, 0x9000u32]);
        assert_eq!(calls_to(&e, PHYSICS_SYSTEM_ADD_RIGID_BODY).len(), 5);
        e.set(world, HkpWorld::m_fixedIsland, Ptr::NULL);
        set_islands(&mut e, world, &[], &[]);
        e.call(0x00c9_26a0, &args![out, world, 0x9000u32]);
        assert_eq!(e.mem.u8(out), 0);
        assert_eq!(calls_to(&e, PHYSICS_SYSTEM_ADD_RIGID_BODY).len(), 5);
    }

    #[test]
    fn fn_00c92760_adds_the_active_bodies() {
        let mut e = logging();
        silent(&mut e, &[PHYSICS_SYSTEM_ADD_RIGID_BODY]);
        let world = world(&mut e);
        let one = make_island(&mut e, &[0x11, 0x22], &[]);
        let two = make_island(&mut e, &[0x33], &[]);
        set_islands(&mut e, world, &[one, two], &[]);
        let out = e.mem.alloc(8);
        assert_eq!(
            e.call(0x00c9_2760, &args![out, world, 0x9000u32]).u32(),
            out
        );
        assert_eq!(e.mem.u8(out), 1);
        assert_eq!(
            calls_to(&e, PHYSICS_SYSTEM_ADD_RIGID_BODY),
            vec![vec![0x9000, 0x11], vec![0x9000, 0x22], vec![0x9000, 0x33]]
        );
        set_islands(&mut e, world, &[], &[]);
        e.call(0x00c9_2760, &args![out, world, 0x9000u32]);
        assert_eq!(e.mem.u8(out), 0);
    }

    #[test]
    fn fn_00c927c0_stores_the_active_flag_in_the_system() {
        let mut e = logging();
        silent(&mut e, &[PHYSICS_SYSTEM_ADD_RIGID_BODY]);
        let world = world(&mut e);
        let active = make_island(&mut e, &[0x11], &[]);
        set_islands(&mut e, world, &[active], &[]);
        let system = Ptr::new(e.mem.alloc(0x50));
        fn_00c927c0(&mut e, system, world);
        assert_eq!(e.mem.u8(system.addr() + 0x40), 1);
        set_islands(&mut e, world, &[], &[]);
        fn_00c927c0(&mut e, system, world);
        assert_eq!(e.mem.u8(system.addr() + 0x40), 0);
    }

    /// Writes a constraint entry at `entry` whose first word is `first` and whose chain holds
    /// nodes of the given kinds, linked through the word at +0x14.
    fn constraint_entry(e: &mut Engine, entry: u32, first: u32, kinds: &[u16]) {
        e.mem.set_u32(entry, first);
        let mut link = 0;
        for &kind in kinds.iter().rev() {
            let node = e.mem.alloc(0x20);
            e.mem.set_u16(node, kind);
            e.mem.set_u32(node + 0x14, link);
            link = node;
        }
        e.mem.set_u32(entry + 0xc, link);
    }

    #[test]
    fn fn_00c927f0_adds_the_constraints_that_do_not_end_in_0x16() {
        let mut e = logging();
        silent(&mut e, &[PHYSICS_SYSTEM_ADD_CONSTRAINT]);
        let entity = e.mem.alloc(0xc0);
        let entries = e.mem.alloc(0x30 * 4);
        constraint_entry(&mut e, entries, 0xa1, &[0x10]);
        constraint_entry(&mut e, entries + 0x30, 0xa2, &[0x16]);
        constraint_entry(&mut e, entries + 0x60, 0xa3, &[0x20, 0x16]);
        constraint_entry(&mut e, entries + 0x90, 0xa4, &[0x20, 0x21, 0x15]);
        e.mem.set_u32(entity + 0xac, entries);
        e.mem.set_u16(entity + 0xb0, 4);
        let island = make_island(&mut e, &[entity], &[]);
        e.call(0x00c9_27f0, &args![island, 0x9000u32]);
        assert_eq!(
            calls_to(&e, PHYSICS_SYSTEM_ADD_CONSTRAINT),
            vec![vec![0x9000, 0xa1], vec![0x9000, 0xa4]]
        );
        // An entity without constraints adds none.
        e.mem.set_u16(entity + 0xb0, 0);
        e.call(0x00c9_27f0, &args![island, 0x9000u32]);
        assert_eq!(calls_to(&e, PHYSICS_SYSTEM_ADD_CONSTRAINT).len(), 2);
    }

    /// An island holding one entity with one constraint entry whose first word is `first`.
    fn island_with_constraint(e: &mut Engine, first: u32) -> u32 {
        let entity = e.mem.alloc(0xc0);
        let entry = e.mem.alloc(0x30);
        constraint_entry(e, entry, first, &[0x10]);
        e.mem.set_u32(entity + 0xac, entry);
        e.mem.set_u16(entity + 0xb0, 1);
        make_island(e, &[entity], &[])
    }

    #[test]
    fn fn_00c92860_visits_active_then_inactive_islands() {
        let mut e = logging();
        silent(&mut e, &[PHYSICS_SYSTEM_ADD_CONSTRAINT]);
        let world = world(&mut e);
        let active = island_with_constraint(&mut e, 0xb1);
        let inactive = island_with_constraint(&mut e, 0xb2);
        set_islands(&mut e, world, &[active], &[inactive]);
        fn_00c92860(&mut e, world, Ptr::new(0x9000));
        assert_eq!(
            calls_to(&e, PHYSICS_SYSTEM_ADD_CONSTRAINT),
            vec![vec![0x9000, 0xb1], vec![0x9000, 0xb2]]
        );
    }

    #[test]
    fn fn_00c928c0_adds_the_actions_of_every_island() {
        let mut e = logging();
        silent(&mut e, &[PHYSICS_SYSTEM_ADD_ACTION]);
        let world = world(&mut e);
        let active = make_island(&mut e, &[], &[0xc1, 0xc2]);
        let inactive = make_island(&mut e, &[], &[0xc3]);
        set_islands(&mut e, world, &[active], &[inactive]);
        e.call(0x00c9_28c0, &args![world, 0x9000u32]);
        assert_eq!(
            calls_to(&e, PHYSICS_SYSTEM_ADD_ACTION),
            vec![vec![0x9000, 0xc1], vec![0x9000, 0xc2], vec![0x9000, 0xc3]]
        );
    }

    #[test]
    fn fn_00c92950_skips_the_borders_phantoms() {
        let mut e = logging();
        silent(&mut e, &[PHYSICS_SYSTEM_ADD_PHANTOM]);
        let world = world(&mut e);
        let phantoms = array_of(&mut e, &[0x11, 0x22, 0x33]);
        e.set(world, HkpWorld::m_phantoms, Ptr::new(phantoms));
        e.set(world, HkpWorld::m_phantomsSize, 3);
        fn_00c92950(&mut e, world, Ptr::new(0x9000));
        assert_eq!(calls_to(&e, PHYSICS_SYSTEM_ADD_PHANTOM).len(), 3);
        let border = e.mem.alloc(0x40);
        e.mem.set_u32(border + 0x18, 0x22);
        e.mem.set_u32(border + 0x2c, 0x33);
        e.set(world, HkpWorld::m_broadPhaseBorder, Ptr::new(border));
        fn_00c92950(&mut e, world, Ptr::new(0x9000));
        let log = calls_to(&e, PHYSICS_SYSTEM_ADD_PHANTOM);
        assert_eq!(log.len(), 4);
        assert_eq!(log[3], vec![0x9000, 0x11]);
    }

    #[test]
    fn fn_00c929e0_builds_a_system_from_the_world() {
        let mut e = logging();
        let allocated = e.mem.alloc(0x44);
        returning(&mut e, FAKE_1, allocated);
        let allocator = object_with_vtable(&mut e, 0x20, &[(4, FAKE_1)]);
        let router = e.mem.alloc(0x20);
        e.mem.set_u32(router + 0x10, allocator);
        returning(&mut e, MEMORY_ROUTER, router);
        let system = e.mem.alloc(0x50);
        returning(&mut e, PHYSICS_SYSTEM_CONSTRUCT, system);
        silent(
            &mut e,
            &[
                PHYSICS_SYSTEM_ADD_RIGID_BODY,
                PHYSICS_SYSTEM_ADD_CONSTRAINT,
                PHYSICS_SYSTEM_ADD_ACTION,
                PHYSICS_SYSTEM_ADD_PHANTOM,
            ],
        );
        let world = world(&mut e);
        let body_a = e.mem.alloc(0xc0);
        let body_b = e.mem.alloc(0xc0);
        let active = make_island(&mut e, &[body_a], &[0x21]);
        let inactive = make_island(&mut e, &[body_b], &[0x22]);
        set_islands(&mut e, world, &[active], &[inactive]);
        let phantoms = array_of(&mut e, &[0x31]);
        e.set(world, HkpWorld::m_phantoms, Ptr::new(phantoms));
        e.set(world, HkpWorld::m_phantomsSize, 1);
        let returned = e.call(0x00c9_29e0, &args![world]).u32();
        assert_eq!(returned, system);
        assert_eq!(e.mem.u16(allocated + 4), 0x44);
        assert_eq!(
            calls_to(&e, PHYSICS_SYSTEM_CONSTRUCT),
            vec![vec![allocated]]
        );
        assert_eq!(e.mem.u8(system + 0x40), 1);
        assert_eq!(
            calls_to(&e, PHYSICS_SYSTEM_ADD_RIGID_BODY),
            vec![vec![system, body_b], vec![system, body_a]]
        );
        assert_eq!(
            calls_to(&e, PHYSICS_SYSTEM_ADD_PHANTOM),
            vec![vec![system, 0x31]]
        );
        assert_eq!(
            calls_to(&e, PHYSICS_SYSTEM_ADD_ACTION),
            vec![vec![system, 0x21], vec![system, 0x22]]
        );
        let sequence = [
            PHYSICS_SYSTEM_CONSTRUCT,
            PHYSICS_SYSTEM_ADD_RIGID_BODY,
            PHYSICS_SYSTEM_ADD_PHANTOM,
            PHYSICS_SYSTEM_ADD_ACTION,
        ];
        assert_eq!(
            order_of(&e, &sequence),
            vec![
                PHYSICS_SYSTEM_CONSTRUCT,
                PHYSICS_SYSTEM_ADD_RIGID_BODY,
                PHYSICS_SYSTEM_ADD_RIGID_BODY,
                PHYSICS_SYSTEM_ADD_PHANTOM,
                PHYSICS_SYSTEM_ADD_ACTION,
                PHYSICS_SYSTEM_ADD_ACTION,
            ]
        );
    }
}
